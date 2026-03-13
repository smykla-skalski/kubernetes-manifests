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

pub(crate) fn merged_workspace_configuration(user_settings: Option<Value>) -> Value {
    let mut configuration = default_workspace_configuration();

    if let Some(user_settings) = user_settings {
        merge_json_value_into(user_settings, &mut configuration);
    }

    configuration
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
        let configuration = merged_workspace_configuration(Some(json!({
            "yaml": {
                "validate": true,
                "hover": true
            }
        })));

        assert_eq!(configuration["yaml"]["format"]["enable"], true);
        assert_eq!(configuration["yaml"]["validate"], true);
        assert_eq!(configuration["yaml"]["hover"], true);
        assert_eq!(configuration["[yaml]"]["editor.tabSize"], 2);
    }

    #[test]
    fn user_settings_can_override_default_kubernetes_schema_globs() {
        let configuration = merged_workspace_configuration(Some(json!({
            "yaml": {
                "schemas": {
                    "kubernetes": ["*.tmpl.yaml"]
                }
            }
        })));

        assert_eq!(
            configuration["yaml"]["schemas"]["kubernetes"],
            json!(["*.tmpl.yaml"]),
        );
    }
}
