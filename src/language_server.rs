use std::{env, fs, path::Path};

use zed_extension_api::{
    self as zed, LanguageServerId, Result,
    settings::{CommandSettings, LspSettings},
};

use crate::util;

pub const SERVER_NAME: &str = "kubernetes-language-server";
pub const BINARY_NAME: &str = "yaml-language-server";
const PACKAGE_NAME: &str = "yaml-language-server";
const PACKAGE_VERSION: &str = "1.21.0";
const SERVER_PATH: &str = "node_modules/yaml-language-server/bin/yaml-language-server";
const SETTINGS_HANDLER_PATHS: &[&str] = &[
    "node_modules/yaml-language-server/out/server/src/languageserver/handlers/settingsHandlers.js",
    "node_modules/yaml-language-server/lib/esm/languageserver/handlers/settingsHandlers.js",
];
const INVALID_YAML_EDITOR_SCOPE_URI: &str = "{ section: '[yaml]', scopeUri: 'null' },";
const PATCHED_YAML_EDITOR_SCOPE_URI: &str = "{ section: '[yaml]' },";

#[derive(Default)]
pub struct KubernetesLanguageServer {
    cached_server_script_path: Option<String>,
}

impl KubernetesLanguageServer {
    pub fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_settings = LspSettings::for_worktree(SERVER_NAME, worktree)
            .ok()
            .and_then(|settings| settings.binary);
        let args = server_arguments(binary_settings.as_ref());
        let env = util::merged_env(worktree.shell_env(), binary_settings.as_ref());

        if let Some(path) = binary_settings
            .as_ref()
            .and_then(|settings| settings.path.clone())
        {
            return Ok(binary_command(path, args, env));
        }

        if let Some(path) = worktree.which(BINARY_NAME) {
            return Ok(binary_command(path, args, env));
        }

        let server_script_path = self.bundled_server_script_path(language_server_id)?;
        Ok(managed_node_command(
            zed::node_binary_path()?,
            server_script_path,
            args,
            env,
        ))
    }

    fn bundled_server_script_path(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        if let Some(path) = self
            .cached_server_script_path
            .as_ref()
            .filter(|path| file_exists(path))
        {
            patch_managed_yaml_language_server()?;
            return Ok(path.clone());
        }

        let absolute_server_path = extension_file_path(SERVER_PATH)?;

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let installed_version = zed::npm_package_installed_version(PACKAGE_NAME)?;
        let server_exists = file_exists(&absolute_server_path);

        if !server_exists || installed_version.as_deref() != Some(PACKAGE_VERSION) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            match zed::npm_install_package(PACKAGE_NAME, PACKAGE_VERSION) {
                Ok(()) => {}
                Err(error) if file_exists(&absolute_server_path) => {
                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::Failed(format!(
                            "update failed, using cached binary: {error}"
                        )),
                    );
                }
                Err(error) => return Err(error),
            }

            if !file_exists(&absolute_server_path) {
                return Err(format!(
                    "installed package '{PACKAGE_NAME}' did not contain expected path '{SERVER_PATH}'",
                ));
            }
        }

        patch_managed_yaml_language_server()?;

        self.cached_server_script_path = Some(absolute_server_path.clone());
        Ok(absolute_server_path)
    }
}

fn extension_file_path(relative_path: &str) -> Result<String> {
    let current_dir = env::current_dir()
        .map_err(|error| format!("failed to resolve extension directory: {error}"))?;
    Ok(current_dir
        .join(relative_path)
        .to_string_lossy()
        .into_owned())
}

fn file_exists(path: impl AsRef<Path>) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn patch_managed_yaml_language_server() -> Result<()> {
    for relative_path in SETTINGS_HANDLER_PATHS {
        let absolute_path = extension_file_path(relative_path)?;
        if !file_exists(&absolute_path) {
            continue;
        }

        patch_yaml_language_server_settings_handler(&absolute_path).map_err(|error| {
            format!(
                "failed to patch managed yaml-language-server settings handler '{relative_path}': {error}"
            )
        })?;
    }

    Ok(())
}

fn patch_yaml_language_server_settings_handler(path: &str) -> Result<()> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("failed to read '{path}': {error}"))?;
    let Some(patched_contents) = patched_yaml_editor_scope_uri(&contents) else {
        return Ok(());
    };

    fs::write(path, patched_contents).map_err(|error| format!("failed to write '{path}': {error}"))
}

fn patched_yaml_editor_scope_uri(contents: &str) -> Option<String> {
    let patched = contents.replace(INVALID_YAML_EDITOR_SCOPE_URI, PATCHED_YAML_EDITOR_SCOPE_URI);
    if patched == contents {
        None
    } else {
        Some(patched)
    }
}

fn default_server_arguments() -> Vec<String> {
    vec!["--stdio".to_string()]
}

fn server_arguments(binary_settings: Option<&CommandSettings>) -> Vec<String> {
    binary_settings
        .and_then(|settings| settings.arguments.clone())
        .unwrap_or_else(default_server_arguments)
}

const fn binary_command(
    command: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
) -> zed::Command {
    zed::Command { command, args, env }
}

fn managed_node_command(
    node_binary: String,
    server_script_path: String,
    server_args: Vec<String>,
    env: Vec<(String, String)>,
) -> zed::Command {
    let mut args = Vec::with_capacity(server_args.len() + 1);
    args.push(server_script_path);
    args.extend(server_args);

    zed::Command {
        command: node_binary,
        args,
        env,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::tests::command_settings;

    #[test]
    fn binary_command_uses_default_stdio_arguments_when_none_supplied() {
        let command = binary_command(BINARY_NAME.to_string(), server_arguments(None), Vec::new());

        assert_eq!(command.command, BINARY_NAME);
        assert_eq!(command.args, vec!["--stdio"]);
        assert!(command.env.is_empty());
    }

    #[test]
    fn binary_command_respects_user_provided_args_and_env() {
        let settings = command_settings(
            Some("/opt/bin/yaml-language-server"),
            Some(vec!["--socket=6000"]),
            Some(vec![("YAML_SCHEMA_STORE_ENABLE", "false")]),
        );
        let command = binary_command(
            settings.path.clone().expect("path should exist for test"),
            server_arguments(Some(&settings)),
            util::merged_env(Vec::new(), Some(&settings)),
        );

        assert_eq!(command.command, "/opt/bin/yaml-language-server");
        assert_eq!(command.args, vec!["--socket=6000"]);
        assert_eq!(
            command.env,
            vec![("YAML_SCHEMA_STORE_ENABLE".to_string(), "false".to_string(),)],
        );
    }

    #[test]
    fn managed_node_command_places_script_path_before_server_arguments() {
        let command = managed_node_command(
            "/opt/node/bin/node".to_string(),
            "/ext/node_modules/yaml-language-server/bin/yaml-language-server".to_string(),
            vec!["--stdio".to_string(), "--verbose".to_string()],
            Vec::new(),
        );

        assert_eq!(command.command, "/opt/node/bin/node");
        assert_eq!(
            command.args,
            vec![
                "/ext/node_modules/yaml-language-server/bin/yaml-language-server",
                "--stdio",
                "--verbose",
            ],
        );
    }

    #[test]
    fn patch_removes_invalid_yaml_editor_scope_uri() {
        let contents = "before\n            { section: '[yaml]', scopeUri: 'null' },\nafter\n";
        let patched = patched_yaml_editor_scope_uri(contents)
            .expect("patch should update the invalid scopeUri entry");

        assert!(
            patched.contains("{ section: '[yaml]' },"),
            "patch should keep the [yaml] config request while removing the invalid scopeUri",
        );
        assert!(
            !patched.contains("scopeUri: 'null'"),
            "patch should remove the invalid literal null scopeUri",
        );
    }

    #[test]
    fn patch_is_noop_when_invalid_scope_uri_is_already_absent() {
        let contents = "before\n            { section: '[yaml]' },\nafter\n";

        assert!(
            patched_yaml_editor_scope_uri(contents).is_none(),
            "patch should not rewrite files that already avoid the invalid scopeUri",
        );
    }
}
