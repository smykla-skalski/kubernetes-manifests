mod language_server;
mod settings;

use language_server::{KubernetesLanguageServer, SERVER_NAME};
use settings::merged_workspace_configuration;
use zed_extension_api::{
    self as zed, lsp::Completion, settings::LspSettings, CodeLabel, CodeLabelSpan,
    LanguageServerId, Result,
};

struct KubernetesExtension {
    kubernetes_language_server: KubernetesLanguageServer,
}

impl KubernetesExtension {
    fn ensure_known_server(language_server_id: &LanguageServerId) -> Result<()> {
        match language_server_id.as_ref() {
            SERVER_NAME => Ok(()),
            _ => Err(format!("Unknown language server ID {language_server_id}")),
        }
    }
}

impl zed::Extension for KubernetesExtension {
    fn new() -> Self {
        Self {
            kubernetes_language_server: KubernetesLanguageServer::new(),
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Self::ensure_known_server(language_server_id)?;
        self.kubernetes_language_server
            .language_server_command(language_server_id, worktree)
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Self::ensure_known_server(language_server_id)?;
        let user_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.settings);
        let worktree_root = worktree.root_path();

        Ok(Some(merged_workspace_configuration(
            user_settings,
            Some(&worktree_root),
        )))
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Self::ensure_known_server(language_server_id)?;
        Ok(
            LspSettings::for_worktree(language_server_id.as_ref(), worktree)
                .ok()
                .and_then(|settings| settings.initialization_options),
        )
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

        Some(CodeLabel {
            spans: vec![
                CodeLabelSpan::code_range(0..label_len),
                CodeLabelSpan::literal(": ", None),
                CodeLabelSpan::literal(detail, Some("comment".to_string())),
            ],
            filter_range: (0..label_len).into(),
            code,
        })
    }

    fn label_for_symbol(
        &self,
        _language_server_id: &LanguageServerId,
        symbol: zed::lsp::Symbol,
    ) -> Option<CodeLabel> {
        let name = &symbol.name;
        let code = format!("{name}: ");
        let name_len = name.len();

        Some(CodeLabel {
            spans: vec![CodeLabelSpan::code_range(0..name_len)],
            filter_range: (0..name_len).into(),
            code,
        })
    }
}

zed::register_extension!(KubernetesExtension);

#[cfg(test)]
mod tests {
    use regex::Regex;
    use serde_json::Value as JsonValue;
    use std::{fs, path::PathBuf};
    use toml::Value as TomlValue;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read_extension_manifest() -> TomlValue {
        let path = repo_root().join("extension.toml");
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        source
            .parse::<TomlValue>()
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
    }

    fn read_language_config() -> TomlValue {
        let path = repo_root().join("languages/kubernetes/config.toml");
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        source
            .parse::<TomlValue>()
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
        let path = repo_root().join(relative_path);
        fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
    }

    fn read_icon_theme() -> JsonValue {
        let path = repo_root().join("icon_themes/kubernetes.json");
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
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
    fn first_line_pattern_detects_kubernetes_headers_in_plain_yaml() {
        let pattern = kubernetes_first_line_pattern();

        assert!(
            pattern.is_match(&read_fixture("fixtures/valid/plain-deployment.yaml")),
            "plain deployment yaml should auto-detect as Kubernetes",
        );
        assert!(
            pattern.is_match(&read_fixture("fixtures/valid/plain-kind-first.yaml")),
            "kind/apiVersion in reverse order should auto-detect as Kubernetes",
        );
        assert!(
            pattern.is_match(&read_fixture("fixtures/valid/plain-multi-document.yaml")),
            "plain multi-document yaml should auto-detect as Kubernetes",
        );
        assert!(
            !pattern.is_match(&read_fixture("fixtures/invalid/plain-non-kubernetes.yaml")),
            "plain non-Kubernetes yaml should not auto-detect as Kubernetes",
        );
        assert!(
            !pattern.is_match(&read_fixture("fixtures/invalid/plain-missing-kind.yaml")),
            "plain yaml missing kind should not auto-detect as Kubernetes",
        );
    }

    #[test]
    fn kubernetes_icon_theme_maps_kubernetes_suffixes() {
        let icon_theme = read_icon_theme();

        let theme = icon_theme["themes"]
            .as_array()
            .and_then(|themes| themes.first())
            .expect("kubernetes icon theme should define at least one theme");
        let suffixes = theme["file_suffixes"]
            .as_object()
            .expect("icon theme should define file_suffixes");
        let icons = theme["file_icons"]
            .as_object()
            .expect("icon theme should define file_icons");

        for suffix in ["k8s.yaml", "k8s.yml", "kubernetes.yaml", "kubernetes.yml"] {
            assert_eq!(
                suffixes.get(suffix).and_then(JsonValue::as_str),
                Some("kubernetes"),
                "icon theme should map {suffix} to the kubernetes icon",
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
        assert_eq!(
            stems.get("kustomization").and_then(JsonValue::as_str),
            Some("kubernetes"),
        );
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
            assert_eq!(
                dirs.get(dir).and_then(JsonValue::as_str),
                Some("helm"),
                "icon theme should map {dir} directory to helm icon",
            );
        }
        for dir in ["manifests", "k8s", "kubernetes", "deploy"] {
            assert_eq!(
                dirs.get(dir).and_then(JsonValue::as_str),
                Some("kubernetes"),
                "icon theme should map {dir} directory to kubernetes icon",
            );
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
}
