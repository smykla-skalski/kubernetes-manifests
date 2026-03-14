mod context_server;
mod docs;
mod helm_language_server;
mod language_server;
mod settings;
mod templates;

use context_server::KubernetesContextServer;
use helm_language_server::HelmLanguageServer;
use language_server::{KubernetesLanguageServer, SERVER_NAME};
use settings::{
    helm_workspace_configuration_schema, kubernetes_initialization_options_schema,
    kubernetes_workspace_configuration, kubernetes_workspace_configuration_schema,
    yaml_server_injection_configuration,
};
use templates::{resource_kinds, template_for_kind};
use zed_extension_api::{
    self as zed,
    lsp::{Completion, CompletionKind, Symbol, SymbolKind},
    serde_json::Value as JsonValue,
    settings::LspSettings,
    CodeLabel, CodeLabelSpan, ContextServerConfiguration, ContextServerId, KeyValueStore,
    LanguageServerId, Project, Result, SlashCommand, SlashCommandArgumentCompletion,
    SlashCommandOutput, SlashCommandOutputSection,
};

const SLASH_COMMAND_NAME: &str = "kubernetes";
const EXPLAIN_COMMAND_NAME: &str = "kubernetes-explain";

struct KubernetesExtension {
    kubernetes_lsp: KubernetesLanguageServer,
    helm_lsp: HelmLanguageServer,
    context_server: KubernetesContextServer,
}

impl KubernetesExtension {
    fn ensure_known_server(language_server_id: &LanguageServerId) -> Result<()> {
        match language_server_id.as_ref() {
            SERVER_NAME | helm_language_server::SERVER_NAME => Ok(()),
            _ => Err(format!("Unknown language server ID {language_server_id}")),
        }
    }

    fn ensure_known_context_server(context_server_id: &ContextServerId) -> Result<()> {
        match context_server_id.as_ref() {
            context_server::CONTEXT_SERVER_NAME => Ok(()),
            _ => Err(format!("Unknown context server ID {context_server_id}")),
        }
    }

    fn run_template_command(args: &[String]) -> Result<SlashCommandOutput, String> {
        let kind = args
            .first()
            .ok_or_else(|| "Usage: /kubernetes <ResourceKind>".to_string())?;

        let template = template_for_kind(kind).ok_or_else(|| {
            let available: Vec<_> = resource_kinds().collect();
            format!(
                "Unknown resource kind: {kind}. Available: {}",
                available.join(", ")
            )
        })?;

        let text = template.to_string();
        let len = text.len();

        Ok(SlashCommandOutput {
            text,
            sections: vec![SlashCommandOutputSection {
                range: (0..len).into(),
                label: kind.clone(),
            }],
        })
    }

    fn run_explain_command(args: &[String]) -> Result<SlashCommandOutput, String> {
        let kind = args
            .first()
            .ok_or_else(|| "Usage: /kubernetes-explain <ResourceKind>".to_string())?;

        let packages = docs::suggest_packages();
        if !packages.iter().any(|p| p == kind) {
            return Err(format!(
                "Unknown resource kind: {kind}. Available: {}",
                packages.join(", ")
            ));
        }

        let text = docs::explain_resource(kind);
        let len = text.len();

        Ok(SlashCommandOutput {
            text,
            sections: vec![SlashCommandOutputSection {
                range: (0..len).into(),
                label: format!("Kubernetes: {kind}"),
            }],
        })
    }
}

impl zed::Extension for KubernetesExtension {
    fn new() -> Self {
        Self {
            kubernetes_lsp: KubernetesLanguageServer::new(),
            helm_lsp: HelmLanguageServer::new(),
            context_server: KubernetesContextServer::new(),
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Self::ensure_known_server(language_server_id)?;
        match language_server_id.as_ref() {
            helm_language_server::SERVER_NAME => self
                .helm_lsp
                .language_server_command(language_server_id, worktree),
            _ => self
                .kubernetes_lsp
                .language_server_command(language_server_id, worktree),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<JsonValue>> {
        Self::ensure_known_server(language_server_id)?;

        if language_server_id.as_ref() == helm_language_server::SERVER_NAME {
            return Ok(
                LspSettings::for_worktree(language_server_id.as_ref(), worktree)
                    .ok()
                    .and_then(|settings| settings.settings),
            );
        }

        let user_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.settings);
        let worktree_root = worktree.root_path();
        let home_dir = worktree
            .shell_env()
            .into_iter()
            .find(|(key, _)| key == "HOME")
            .map(|(_, value)| value);

        Ok(Some(kubernetes_workspace_configuration(
            user_settings,
            Some(&worktree_root),
            home_dir.as_deref(),
        )))
    }

    fn language_server_initialization_options_schema(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Option<JsonValue> {
        Self::ensure_known_server(language_server_id).ok()?;

        if language_server_id.as_ref() == SERVER_NAME {
            return Some(kubernetes_initialization_options_schema());
        }

        None
    }

    fn language_server_workspace_configuration_schema(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Option<JsonValue> {
        Self::ensure_known_server(language_server_id).ok()?;

        match language_server_id.as_ref() {
            SERVER_NAME => Some(kubernetes_workspace_configuration_schema()),
            helm_language_server::SERVER_NAME => Some(helm_workspace_configuration_schema()),
            _ => None,
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<JsonValue>> {
        Self::ensure_known_server(language_server_id)?;
        Ok(
            LspSettings::for_worktree(language_server_id.as_ref(), worktree)
                .ok()
                .and_then(|settings| settings.initialization_options),
        )
    }

    fn language_server_additional_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        target_language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<JsonValue>> {
        Self::ensure_known_server(language_server_id)?;

        if language_server_id.as_ref() == SERVER_NAME
            && target_language_server_id.as_ref() == "yaml-language-server"
        {
            let user_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
                .ok()
                .and_then(|settings| settings.settings);
            let worktree_root = worktree.root_path();
            let home_dir = worktree
                .shell_env()
                .into_iter()
                .find(|(key, _)| key == "HOME")
                .map(|(_, value)| value);

            return Ok(yaml_server_injection_configuration(
                user_settings,
                Some(&worktree_root),
                home_dir.as_deref(),
            ));
        }

        Ok(None)
    }

    fn complete_slash_command_argument(
        &self,
        command: SlashCommand,
        args: Vec<String>,
    ) -> Result<Vec<SlashCommandArgumentCompletion>, String> {
        let query = args.first().map(|s| s.to_lowercase()).unwrap_or_default();

        let candidates: Vec<String> = match command.name.as_str() {
            SLASH_COMMAND_NAME => resource_kinds().map(String::from).collect(),
            EXPLAIN_COMMAND_NAME => docs::suggest_packages(),
            _ => return Ok(Vec::new()),
        };

        Ok(candidates
            .into_iter()
            .filter(|kind| kind.to_lowercase().contains(&query))
            .map(|kind| SlashCommandArgumentCompletion {
                label: kind.clone(),
                new_text: kind,
                run_command: true,
            })
            .collect())
    }

    fn run_slash_command(
        &self,
        command: SlashCommand,
        args: Vec<String>,
        _worktree: Option<&zed::Worktree>,
    ) -> Result<SlashCommandOutput, String> {
        match command.name.as_str() {
            SLASH_COMMAND_NAME => Self::run_template_command(&args),
            EXPLAIN_COMMAND_NAME => Self::run_explain_command(&args),
            _ => Err(format!("Unknown slash command: {}", command.name)),
        }
    }

    fn label_for_completion(
        &self,
        _language_server_id: &LanguageServerId,
        completion: Completion,
    ) -> Option<CodeLabel> {
        let detail = completion.detail.as_deref()?;
        let label = &completion.label;
        let code = format!("{label}: {detail}");
        let label_len = label.len();

        let detail_highlight = match completion.kind.as_ref() {
            Some(CompletionKind::Property) => "property",
            Some(CompletionKind::Field | CompletionKind::Variable) => "attribute",
            Some(CompletionKind::Keyword) => "keyword",
            _ => "comment",
        };

        Some(CodeLabel {
            spans: vec![
                CodeLabelSpan::code_range(0..label_len),
                CodeLabelSpan::literal(": ", None),
                CodeLabelSpan::literal(detail, Some(detail_highlight.to_string())),
            ],
            filter_range: (0..label_len).into(),
            code,
        })
    }

    fn label_for_symbol(
        &self,
        _language_server_id: &LanguageServerId,
        symbol: Symbol,
    ) -> Option<CodeLabel> {
        let name = &symbol.name;

        let prefix = match symbol.kind {
            SymbolKind::Class | SymbolKind::Module => Some("kind: "),
            SymbolKind::Namespace => Some("ns: "),
            _ => None,
        };

        let code = format!("{name}: ");
        let name_len = name.len();

        let mut spans = Vec::new();
        if let Some(prefix) = prefix {
            spans.push(CodeLabelSpan::literal(prefix, Some("keyword".to_string())));
        }
        spans.push(CodeLabelSpan::code_range(0..name_len));

        let filter_start = prefix.map_or(0, str::len);
        let filter_end = filter_start + name_len;

        Some(CodeLabel {
            spans,
            filter_range: (filter_start..filter_end).into(),
            code,
        })
    }

    fn suggest_docs_packages(&self, provider: String) -> Result<Vec<String>, String> {
        if !docs::is_docs_provider(&provider) {
            return Ok(Vec::new());
        }
        Ok(docs::suggest_packages())
    }

    fn index_docs(
        &self,
        provider: String,
        package: String,
        database: &KeyValueStore,
    ) -> Result<(), String> {
        if !docs::is_docs_provider(&provider) {
            return Err(format!("Unknown docs provider: {provider}"));
        }
        docs::index_package(&package, database)
    }

    fn context_server_command(
        &mut self,
        context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<zed::Command> {
        Self::ensure_known_context_server(context_server_id)?;
        Ok(self
            .context_server
            .context_server_command(context_server_id, project))
    }

    fn context_server_configuration(
        &mut self,
        context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<Option<ContextServerConfiguration>> {
        Self::ensure_known_context_server(context_server_id)?;
        Ok(Some(context_server::context_server_configuration()))
    }
}

zed::register_extension!(KubernetesExtension);

#[cfg(test)]
mod tests {
    use regex::Regex;
    use serde_json::Value as JsonValue;
    use std::{collections::HashMap, fs, path::PathBuf};
    use toml::Value as TomlValue;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read_repo_file(relative_path: &str) -> String {
        let path = repo_root().join(relative_path);
        fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
    }

    fn read_extension_manifest() -> TomlValue {
        let path = repo_root().join("extension.toml");
        let source = read_repo_file("extension.toml");
        toml::from_str(&source)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
    }

    fn read_cargo_manifest() -> TomlValue {
        let path = repo_root().join("Cargo.toml");
        let source = read_repo_file("Cargo.toml");
        toml::from_str(&source)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
    }

    fn read_language_config() -> TomlValue {
        let path = repo_root().join("languages/kubernetes/config.toml");
        let source = read_repo_file("languages/kubernetes/config.toml");
        toml::from_str(&source)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
    }

    fn kubernetes_first_line_pattern() -> Regex {
        let config = read_language_config();
        let pattern = config
            .get("first_line_pattern")
            .and_then(TomlValue::as_str)
            .expect("kubernetes config should define first_line_pattern");
        Regex::new(pattern).expect("kubernetes first_line_pattern should compile")
    }

    fn read_fixture(relative_path: &str) -> String {
        read_repo_file(relative_path)
    }

    /// Simulates Zed's content window for `first_line_pattern` detection.
    /// Zed passes only the first line of a buffer (up to 256 columns) via
    /// `content.clip_point(Point::new(0, 256), Bias::Left)`.
    fn first_line_window(content: &str) -> &str {
        let first_line_end = content.find('\n').unwrap_or(content.len());
        &content[..first_line_end.min(256)]
    }

    fn read_icon_theme() -> JsonValue {
        let path = repo_root().join("icon_themes/kubernetes.json");
        let source = read_repo_file("icon_themes/kubernetes.json");
        serde_json::from_str(&source)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
    }

    #[test]
    fn extension_manifest_uses_kubernetes_language_server_id() {
        let manifest = read_extension_manifest();
        let language_servers = manifest
            .get("language_servers")
            .and_then(TomlValue::as_table)
            .expect("extension manifest should define language_servers");

        assert!(
            language_servers.contains_key("kubernetes-language-server"),
            "extension manifest should expose kubernetes-language-server",
        );
        assert!(
            !language_servers.contains_key("kubernetes-yaml-language-server"),
            "extension manifest should not expose the old kubernetes-yaml-language-server id",
        );

        let server = language_servers
            .get("kubernetes-language-server")
            .and_then(TomlValue::as_table)
            .expect("kubernetes-language-server table should exist");
        let server_name = server
            .get("name")
            .and_then(TomlValue::as_str)
            .expect("language server should define a display name");
        assert_eq!(server_name, "Kubernetes Language Server");
    }

    #[test]
    fn first_line_pattern_matches_apiversion_on_first_line() {
        let pattern = kubernetes_first_line_pattern();

        // Zed only passes the first line (up to 256 columns) to
        // first_line_pattern via clip_point(Point::new(0, 256)).
        // Test against the same window Zed provides.
        assert!(
            pattern.is_match(first_line_window(&read_fixture(
                "fixtures/valid/plain-deployment.yaml"
            ))),
            "file starting with apiVersion should match",
        );
        assert!(
            !pattern.is_match(first_line_window(&read_fixture(
                "fixtures/valid/plain-kind-first.yaml"
            ))),
            "file starting with a comment should not match (apiVersion not on first line)",
        );
        assert!(
            !pattern.is_match(first_line_window(&read_fixture(
                "fixtures/valid/plain-multi-document.yaml"
            ))),
            "file starting with a comment should not match (apiVersion not on first line)",
        );
        assert!(
            !pattern.is_match(first_line_window(&read_fixture(
                "fixtures/invalid/plain-non-kubernetes.yaml"
            ))),
            "non-Kubernetes yaml should not match",
        );
        assert!(
            pattern.is_match(first_line_window(&read_fixture(
                "fixtures/invalid/plain-missing-kind.yaml"
            ))),
            "apiVersion on first line should match even without kind (detection is best-effort)",
        );
    }

    #[test]
    fn kubernetes_icon_theme_maps_kubernetes_suffixes() {
        let icon_theme = read_icon_theme();
        let themes = icon_theme["themes"]
            .as_array()
            .expect("kubernetes icon theme should define themes");

        assert_eq!(
            themes.len(),
            2,
            "kubernetes icon theme should define dark and light themes"
        );
        for theme in themes {
            assert_icon_theme_theme(theme);
        }
    }

    #[test]
    fn build_wasm_task_derives_artifact_name_from_cargo_package() {
        let cargo_manifest = read_cargo_manifest();
        let package_name = cargo_manifest["package"]["name"]
            .as_str()
            .expect("Cargo.toml should define package.name");
        let build_task = read_repo_file(".mise/tasks/build/wasm");

        assert!(
            build_task.contains("crate_name=\"$("),
            "build:wasm should derive the crate name from Cargo.toml",
        );
        assert!(
            build_task.contains("tr '-' '_'"),
            "build:wasm should normalize dashes to underscores for the wasm artifact name",
        );
        assert!(
            build_task.contains("artifact_path=\"target/wasm32-wasip2/debug/${crate_name}.wasm\""),
            "build:wasm should assemble the artifact path from the derived crate name",
        );
        assert!(
            !build_task.contains("kubernetes_manifests.wasm"),
            "build:wasm should not hardcode the old crate artifact name",
        );
        assert!(
            !build_task.contains(&format!(
                "target/wasm32-wasip2/debug/{}.wasm",
                package_name.replace('-', "_")
            )),
            "build:wasm should avoid hardcoding the current crate artifact name",
        );
    }

    #[test]
    fn dev_install_tasks_clean_current_and_legacy_extension_paths() {
        let install_task = read_repo_file(".mise/tasks/zed/install-dev-extension");
        let refresh_task = read_repo_file(".mise/tasks/zed/refresh-runtime");

        assert!(
            install_task.contains("installed_extension_path=\"$installed_dir/kubernetes\""),
            "zed:install-dev-extension should install to the current kubernetes id",
        );
        assert!(
            install_task.contains(
                "legacy_installed_extension_path=\"$installed_dir/kubernetes-manifests\""
            ),
            "zed:install-dev-extension should keep cleaning the legacy install id",
        );
        assert!(
            install_task.contains(
                "rm -rf \"$installed_extension_path\" \"$legacy_installed_extension_path\""
            ),
            "zed:install-dev-extension should remove both current and legacy install roots",
        );
        assert!(
            refresh_task.contains("work_dir=\"$user_data_dir/extensions/work/kubernetes\""),
            "zed:refresh-runtime should rotate the current runtime cache directory",
        );
        assert!(
            refresh_task
                .contains("find \"$legacy_work_root\" -maxdepth 1 -name 'kubernetes-manifests*' -exec rm -rf {} +"),
            "zed:refresh-runtime should keep cleaning legacy runtime caches",
        );
    }

    fn assert_icon_theme_theme(theme: &JsonValue) {
        let appearance = theme["appearance"]
            .as_str()
            .expect("icon theme should define appearance");
        assert!(
            matches!(appearance, "dark" | "light"),
            "icon theme appearance should be dark or light, got {appearance}",
        );

        let suffixes = theme["file_suffixes"]
            .as_object()
            .expect("icon theme should define file_suffixes");
        let icons = theme["file_icons"]
            .as_object()
            .expect("icon theme should define file_icons");

        for suffix in [
            "k8s.yaml",
            "k8s.yml",
            "kubernetes.yaml",
            "kubernetes.yml",
            "kustomize.yaml",
            "kustomize.yml",
        ] {
            assert_eq!(
                suffixes.get(suffix).and_then(JsonValue::as_str),
                Some("kubernetes"),
                "icon theme should map {suffix} to the kubernetes icon",
            );
        }
        for suffix in ["helm.yaml", "helm.yml"] {
            assert_eq!(
                suffixes.get(suffix).and_then(JsonValue::as_str),
                Some("helm"),
                "icon theme should map {suffix} to the helm icon",
            );
        }

        let kubernetes_icon = icons
            .get("kubernetes")
            .and_then(JsonValue::as_object)
            .expect("icon theme should define a kubernetes icon entry");
        assert_eq!(
            kubernetes_icon.get("path").and_then(JsonValue::as_str),
            Some("./icons/kubernetes.svg"),
        );

        let stems = theme["file_stems"]
            .as_object()
            .expect("icon theme should define file_stems");
        for stem in ["kustomization", "skaffold", "Tiltfile"] {
            assert_eq!(
                stems.get(stem).and_then(JsonValue::as_str),
                Some("kubernetes"),
                "icon theme should map {stem} stem to kubernetes icon",
            );
        }
        for stem in ["Chart", "values", "helmfile"] {
            assert_eq!(
                stems.get(stem).and_then(JsonValue::as_str),
                Some("helm"),
                "icon theme should map {stem} stem to helm icon",
            );
        }

        let dirs = theme["named_directory_icons"]
            .as_object()
            .expect("icon theme should define named_directory_icons");
        for dir in ["templates", "charts"] {
            assert_named_directory_icon(dirs, dir, "helm");
        }
        for dir in [
            "manifests",
            "k8s",
            "kubernetes",
            "deploy",
            "base",
            "overlays",
            "patches",
        ] {
            assert_named_directory_icon(dirs, dir, "kubernetes");
        }

        let helm_icon = icons
            .get("helm")
            .and_then(JsonValue::as_object)
            .expect("icon theme should define a helm icon entry");
        assert_eq!(
            helm_icon.get("path").and_then(JsonValue::as_str),
            Some("./icons/helm.svg"),
        );
    }

    #[test]
    fn extension_manifest_declares_context_server() {
        let manifest = read_extension_manifest();
        let context_servers = manifest
            .get("context_servers")
            .and_then(TomlValue::as_table)
            .expect("extension manifest should define context_servers");

        assert!(
            context_servers.contains_key("kubernetes-context-server"),
            "context_servers should contain kubernetes-context-server",
        );
    }

    #[test]
    fn extension_manifest_declares_kubernetes_slash_command() {
        let manifest = read_extension_manifest();
        let slash_commands = manifest
            .get("slash_commands")
            .and_then(TomlValue::as_table)
            .expect("extension manifest should define slash_commands");

        for name in ["kubernetes", "kubernetes-explain"] {
            let cmd = slash_commands
                .get(name)
                .and_then(TomlValue::as_table)
                .unwrap_or_else(|| panic!("slash_commands should contain {name}"));
            assert_eq!(
                cmd.get("requires_argument").and_then(TomlValue::as_bool),
                Some(true),
                "{name} should require an argument",
            );
        }
    }

    #[test]
    fn extension_manifest_declares_snippets() {
        let manifest = read_extension_manifest();
        let snippets = manifest
            .get("snippets")
            .and_then(TomlValue::as_str)
            .expect("extension manifest should declare snippets path");

        assert!(
            snippets.contains("kubernetes.json"),
            "snippets path should point to kubernetes.json",
        );

        let path = repo_root().join(snippets.trim_start_matches("./"));
        assert!(path.exists(), "snippets file should exist at {snippets}");
    }

    fn assert_named_directory_icon(
        dirs: &serde_json::Map<String, JsonValue>,
        dir: &str,
        icon: &str,
    ) {
        let directory_icon = dirs
            .get(dir)
            .and_then(JsonValue::as_object)
            .unwrap_or_else(|| panic!("icon theme should define {dir} as a directory icon object"));

        assert_eq!(
            directory_icon.get("collapsed").and_then(JsonValue::as_str),
            Some(icon),
            "icon theme should map collapsed {dir} directory icon to {icon}",
        );
        assert_eq!(
            directory_icon.get("expanded").and_then(JsonValue::as_str),
            Some(icon),
            "icon theme should map expanded {dir} directory icon to {icon}",
        );
    }

    #[test]
    fn snippets_produce_valid_yaml() {
        let path = repo_root().join("snippets/kubernetes.json");
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let snippets: HashMap<String, JsonValue> = serde_json::from_str(&source)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));

        let tabstop = Regex::new(r"\$\{\d+:([^}]*)\}").unwrap();
        let bare_ref = Regex::new(r"\$\d+").unwrap();

        for (name, snippet) in &snippets {
            let body = snippet["body"]
                .as_array()
                .unwrap_or_else(|| panic!("snippet {name} should have a body array"));

            let yaml_text: String = body
                .iter()
                .filter_map(|line| line.as_str())
                .map(|line| {
                    let line = tabstop.replace_all(line, "$1");
                    bare_ref.replace_all(&line, "placeholder").into_owned()
                })
                .filter(|line| line.trim() != "placeholder")
                .collect::<Vec<_>>()
                .join("\n");

            let result: Result<serde_yml::Value, _> = serde_yml::from_str(&yaml_text);
            assert!(
                result.is_ok(),
                "snippet {name} produces invalid YAML: {}",
                result.unwrap_err(),
            );
        }
    }
}
