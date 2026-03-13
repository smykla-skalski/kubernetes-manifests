use serde_json::Value as JsonValue;
use zed_extension_api::{
    self as zed, settings::ContextServerSettings, ContextServerConfiguration, ContextServerId,
    Project,
};

pub const CONTEXT_SERVER_NAME: &str = "kubernetes-context-server";
const DEFAULT_BINARY: &str = "kubernetes-mcp-server";

pub fn context_server_command(
    context_server_id: &ContextServerId,
    project: &Project,
) -> zed::Command {
    let server_settings =
        ContextServerSettings::for_project(context_server_id.as_ref(), project).ok();
    let binary_settings = server_settings.as_ref().and_then(|s| s.command.as_ref());

    let command = binary_settings
        .and_then(|s| s.path.clone())
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

### Install

```sh
npm install -g kubernetes-mcp-server
```

### Configure

The extension runs `kubernetes-mcp-server` by default. \
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
    fn installation_instructions_mention_npm_install() {
        assert!(INSTALLATION_INSTRUCTIONS.contains("npm install -g kubernetes-mcp-server"));
    }

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
}
