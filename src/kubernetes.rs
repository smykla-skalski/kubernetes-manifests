mod language_server;
mod settings;

use language_server::{KubernetesYamlLanguageServer, SERVER_NAME};
use settings::merged_workspace_configuration;
use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

struct KubernetesExtension {
    kubernetes_yaml_language_server: KubernetesYamlLanguageServer,
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
            kubernetes_yaml_language_server: KubernetesYamlLanguageServer::new(),
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Self::ensure_known_server(language_server_id)?;
        self.kubernetes_yaml_language_server
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

        Ok(Some(merged_workspace_configuration(user_settings)))
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
}

zed::register_extension!(KubernetesExtension);
