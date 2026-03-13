use std::path::PathBuf;

use serde_json::{json, Map, Value};

fn default_schema_globs() -> Value {
    json!([
        "/*.k8s.yaml",
        "/*.k8s.yml",
        "/*.kubernetes.yaml",
        "/*.kubernetes.yml",
        "/**/*.k8s.yaml",
        "/**/*.k8s.yml",
        "/**/*.kubernetes.yaml",
        "/**/*.kubernetes.yml"
    ])
}

pub(crate) fn default_workspace_configuration() -> Value {
    json!({
        "[yaml]": {
            "editor.tabSize": 2
        },
        "yaml": {
            "format": {
                "enable": true
            },
            "schemas": {
                "kubernetes": default_schema_globs()
            }
        }
    })
}

pub(crate) fn merged_workspace_configuration(
    user_settings: Option<Value>,
    worktree_root: Option<&str>,
) -> Value {
    let mut configuration = default_workspace_configuration();

    if let Some(user_settings) = user_settings {
        let user_settings = match worktree_root {
            Some(root) => resolve_schema_paths(user_settings, root),
            None => user_settings,
        };
        merge_json_value_into(user_settings, &mut configuration);
    }

    configuration
}

fn resolve_schema_paths(mut settings: Value, worktree_root: &str) -> Value {
    let yaml_schemas = settings
        .as_object_mut()
        .and_then(|obj| obj.get_mut("yaml"))
        .and_then(Value::as_object_mut)
        .and_then(|yaml| yaml.remove("schemas"));

    let Some(Value::Object(schemas)) = yaml_schemas else {
        return settings;
    };

    let resolved: Map<String, Value> = schemas
        .into_iter()
        .map(|(url, globs)| {
            if url.starts_with('.') {
                let relative = url.strip_prefix("./").unwrap_or(&url);
                let resolved = PathBuf::from(worktree_root)
                    .join(relative)
                    .to_string_lossy()
                    .into_owned();
                (resolved, globs)
            } else {
                (url, globs)
            }
        })
        .collect();

    if let Some(yaml) = settings
        .as_object_mut()
        .and_then(|obj| obj.get_mut("yaml"))
        .and_then(Value::as_object_mut)
    {
        yaml.insert("schemas".to_string(), Value::Object(resolved));
    }

    settings
}

fn merge_json_value_into(source: Value, destination: &mut Value) {
    match (source, destination) {
        (Value::Object(source_object), Value::Object(destination_object)) => {
            merge_json_object_into(source_object, destination_object);
        }
        (source, destination) => *destination = source,
    }
}

fn merge_json_object_into(
    source_object: Map<String, Value>,
    destination_object: &mut Map<String, Value>,
) {
    for (key, source_value) in source_object {
        match destination_object.get_mut(&key) {
            Some(destination_value) => merge_json_value_into(source_value, destination_value),
            None => {
                destination_object.insert(key, source_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_contains_kubernetes_schema_and_formatting() {
        let configuration = default_workspace_configuration();

        assert_eq!(configuration["[yaml]"]["editor.tabSize"], 2);
        assert_eq!(configuration["yaml"]["format"]["enable"], true);
        assert_eq!(
            configuration["yaml"]["schemas"]["kubernetes"],
            default_schema_globs(),
        );
    }

    #[test]
    fn recursive_merge_preserves_defaults_and_adds_nested_user_keys() {
        let configuration = merged_workspace_configuration(
            Some(json!({
                "yaml": {
                    "validate": true,
                    "hover": true
                }
            })),
            None,
        );

        assert_eq!(configuration["yaml"]["format"]["enable"], true);
        assert_eq!(configuration["yaml"]["validate"], true);
        assert_eq!(configuration["yaml"]["hover"], true);
        assert_eq!(configuration["[yaml]"]["editor.tabSize"], 2);
    }

    #[test]
    fn user_settings_can_override_default_kubernetes_schema_globs() {
        let configuration = merged_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "kubernetes": ["*.tmpl.yaml"]
                    }
                }
            })),
            None,
        );

        assert_eq!(
            configuration["yaml"]["schemas"]["kubernetes"],
            json!(["*.tmpl.yaml"]),
        );
    }

    #[test]
    fn relative_schema_paths_resolve_against_worktree_root() {
        let configuration = merged_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "./schemas/custom.json": ["*.yaml"],
                        "https://example.com/schema.json": ["*.k8s.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
        );

        let schemas = configuration["yaml"]["schemas"].as_object().unwrap();
        assert!(
            schemas.contains_key("/home/user/project/schemas/custom.json"),
            "relative path should resolve against worktree root",
        );
        assert!(
            schemas.contains_key("https://example.com/schema.json"),
            "absolute URL should pass through unchanged",
        );
        assert!(
            !schemas.contains_key("./schemas/custom.json"),
            "original relative path should be replaced",
        );
    }

    #[test]
    fn relative_schema_paths_resolve_parent_directory_references() {
        let configuration = merged_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "../shared/schema.json": ["*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
        );

        let schemas = configuration["yaml"]["schemas"].as_object().unwrap();
        assert!(
            schemas.contains_key("/home/user/project/../shared/schema.json"),
            "parent-relative path should resolve against worktree root",
        );
        assert!(
            !schemas.contains_key("../shared/schema.json"),
            "original relative path should be replaced",
        );
    }

    #[test]
    fn schema_path_resolution_preserves_non_relative_urls() {
        let configuration = merged_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "kubernetes": ["*.yaml"],
                        "/absolute/path/schema.json": ["*.yml"]
                    }
                }
            })),
            Some("/home/user/project"),
        );

        let schemas = configuration["yaml"]["schemas"].as_object().unwrap();
        assert!(schemas.contains_key("kubernetes"));
        assert!(schemas.contains_key("/absolute/path/schema.json"));
    }
}
