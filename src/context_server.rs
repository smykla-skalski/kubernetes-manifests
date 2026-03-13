use std::fs;

use serde_json::Value as JsonValue;
use zed_extension_api::{
    self as zed, settings::ContextServerSettings, Architecture, ContextServerConfiguration,
    ContextServerId, DownloadedFileType, GithubReleaseOptions, Os, Project,
};

pub const CONTEXT_SERVER_NAME: &str = "kubernetes-context-server";
const DEFAULT_BINARY: &str = "kubernetes-mcp-server";
const GITHUB_REPO: &str = "strowk/mcp-k8s-go";

pub struct KubernetesContextServer {
    cached_binary_path: Option<String>,
}

impl KubernetesContextServer {
    pub const fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    pub fn context_server_command(
        &mut self,
        context_server_id: &ContextServerId,
        project: &Project,
    ) -> zed::Command {
        let server_settings =
            ContextServerSettings::for_project(context_server_id.as_ref(), project).ok();
        let binary_settings = server_settings.as_ref().and_then(|s| s.command.as_ref());

        let command = binary_settings
            .and_then(|s| s.path.clone())
            .or_else(|| self.downloaded_binary_path())
            .unwrap_or_else(|| DEFAULT_BINARY.to_string());

        let args = binary_settings
            .and_then(|s| s.arguments.clone())
            .unwrap_or_default();

        let mut env: Vec<(String, String)> = Vec::new();
        if let Some(overrides) = binary_settings.and_then(|s| s.env.clone()) {
            env.extend(overrides);
        }

        if let Some(settings) = server_settings.as_ref().and_then(|s| s.settings.as_ref()) {
            push_env_if_set(&mut env, settings, "kubeconfig", "KUBECONFIG");
            push_env_if_set(&mut env, settings, "context", "KUBE_CONTEXT");
            push_env_if_set(&mut env, settings, "namespace", "KUBE_NAMESPACE");
        }

        zed::Command { command, args, env }
    }

    fn downloaded_binary_path(&mut self) -> Option<String> {
        if let Some(path) = self.cached_binary_path.as_ref() {
            if fs::metadata(path).is_ok_and(|m| m.is_file()) {
                return Some(path.clone());
            }
        }

        match download_binary() {
            Ok(path) => {
                self.cached_binary_path = Some(path.clone());
                Some(path)
            }
            Err(_) => None,
        }
    }
}

fn download_binary() -> Result<String, String> {
    let (os, arch) = zed::current_platform();

    let asset_suffix = platform_asset_suffix(os, arch);
    let asset_name = format!("mcp-k8s-go_{asset_suffix}.tar.gz");

    let release = zed::latest_github_release(
        GITHUB_REPO,
        GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| format!("no asset matching {asset_name} in release"))?;

    let version_dir = format!("mcp-k8s-go-{}", release.version);
    fs::create_dir_all(&version_dir).map_err(|e| format!("failed to create directory: {e}"))?;

    let binary_path = format!("{version_dir}/mcp-k8s-go");

    zed::download_file(
        &asset.download_url,
        &version_dir,
        DownloadedFileType::GzipTar,
    )?;
    zed::make_file_executable(&binary_path)?;

    remove_outdated_versions("mcp-k8s-go-", &version_dir);

    Ok(binary_path)
}

fn platform_asset_suffix(os: Os, arch: Architecture) -> String {
    let os_str = match os {
        Os::Mac => "Darwin",
        Os::Linux => "Linux",
        Os::Windows => "Windows",
    };
    let arch_str = match arch {
        Architecture::Aarch64 => "arm64",
        Architecture::X86 | Architecture::X8664 => "x86_64",
    };
    format!("{os_str}_{arch_str}")
}

fn remove_outdated_versions(prefix: &str, current_dir: &str) {
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if name.starts_with(prefix) && name != current_dir {
                let _ = fs::remove_dir_all(name);
            }
        }
    }
}

fn push_env_if_set(
    env: &mut Vec<(String, String)>,
    settings: &JsonValue,
    key: &str,
    env_var: &str,
) {
    if let Some(value) = settings.get(key).and_then(|v| v.as_str()) {
        if !value.is_empty() {
            env.push((env_var.to_string(), value.to_string()));
        }
    }
}

pub fn context_server_configuration() -> ContextServerConfiguration {
    ContextServerConfiguration {
        installation_instructions: INSTALLATION_INSTRUCTIONS.to_string(),
        settings_schema: SETTINGS_SCHEMA.to_string(),
        default_settings: DEFAULT_SETTINGS.to_string(),
    }
}

const INSTALLATION_INSTRUCTIONS: &str = "\
The Kubernetes context server provides cluster state \
(namespaces, deployments, pods, services) as context for \
AI chat. It requires an MCP-compatible server binary.

The extension auto-downloads mcp-k8s-go from GitHub releases. \
To use a different binary, set the path in project settings:

```json
{
  \"context_servers\": {
    \"kubernetes-context-server\": {
      \"command\": {
        \"path\": \"/path/to/your/mcp-server\",
        \"args\": [],
        \"env\": {}
      },
      \"settings\": {
        \"kubeconfig\": \"~/.kube/config\",
        \"context\": \"\",
        \"namespace\": \"default\"
      }
    }
  }
}
```

### Settings

- **kubeconfig** - path to kubeconfig file \
  (sets KUBECONFIG env var)
- **context** - kubernetes context to use \
  (sets KUBE_CONTEXT env var)
- **namespace** - default namespace \
  (sets KUBE_NAMESPACE env var)

These env vars are passed to the MCP server process.";

const SETTINGS_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "kubeconfig": {
      "type": "string",
      "default": "",
      "description": "Path to kubeconfig file. Passed as KUBECONFIG env var to the MCP server."
    },
    "context": {
      "type": "string",
      "default": "",
      "description": "Kubernetes context to use. Passed as KUBE_CONTEXT env var to the MCP server."
    },
    "namespace": {
      "type": "string",
      "default": "default",
      "description": "Default namespace. Passed as KUBE_NAMESPACE env var to the MCP server."
    }
  },
  "additionalProperties": false
}"#;

const DEFAULT_SETTINGS: &str = r#"{
  "kubeconfig": "",
  "context": "",
  "namespace": "default"
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_instructions_mention_settings_path() {
        assert!(INSTALLATION_INSTRUCTIONS.contains("kubernetes-context-server"));
    }

    #[test]
    fn settings_schema_is_valid_json() {
        let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(SETTINGS_SCHEMA);
        assert!(parsed.is_ok(), "settings schema should be valid JSON");
    }

    #[test]
    fn settings_schema_declares_all_settings_fields() {
        let schema: serde_json::Value = serde_json::from_str(SETTINGS_SCHEMA).unwrap();
        let properties = schema["properties"].as_object().unwrap();
        assert!(properties.contains_key("kubeconfig"));
        assert!(properties.contains_key("context"));
        assert!(properties.contains_key("namespace"));
    }

    #[test]
    fn default_settings_is_valid_json() {
        let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(DEFAULT_SETTINGS);
        assert!(parsed.is_ok(), "default settings should be valid JSON");
    }

    #[test]
    fn default_settings_match_schema_fields() {
        let defaults: serde_json::Value = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
        let schema: serde_json::Value = serde_json::from_str(SETTINGS_SCHEMA).unwrap();
        let schema_keys: Vec<_> = schema["properties"].as_object().unwrap().keys().collect();
        let default_keys: Vec<_> = defaults.as_object().unwrap().keys().collect();
        assert_eq!(
            schema_keys, default_keys,
            "default settings keys should match schema properties",
        );
    }

    #[test]
    fn platform_asset_suffix_macos_arm64() {
        let suffix = platform_asset_suffix(Os::Mac, Architecture::Aarch64);
        assert_eq!(suffix, "Darwin_arm64");
    }

    #[test]
    fn platform_asset_suffix_linux_x86_64() {
        let suffix = platform_asset_suffix(Os::Linux, Architecture::X8664);
        assert_eq!(suffix, "Linux_x86_64");
    }

    #[test]
    fn platform_asset_suffix_windows_x86_64() {
        let suffix = platform_asset_suffix(Os::Windows, Architecture::X8664);
        assert_eq!(suffix, "Windows_x86_64");
    }
}
