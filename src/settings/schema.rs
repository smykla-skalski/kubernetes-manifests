use serde_json::{Value, json};

use super::default_custom_tags;

pub fn kubernetes_workspace_configuration_schema() -> Value {
    json!({
        "type": "object",
        "description": "Workspace configuration passed to kubernetes-language-server. Use the kubernetes block for extension-owned settings and yaml for raw yaml-language-server settings that apply when Kubernetes mode owns the buffer.",
        "default": {},
        "additionalProperties": true,
        "properties": {
            "kubernetes": kubernetes_curated_settings_schema(),
            "yaml": raw_passthrough_schema("Raw yaml-language-server workspace settings applied only to Kubernetes-mode buffers. These settings override the extension defaults and the curated kubernetes block."),
            "[yaml]": raw_passthrough_schema("Internal yaml-language-server editor settings used for formatting decisions (tab size, format-on-type). To configure Zed's editor behavior, use languages.Kubernetes in Zed settings.")
        }
    })
}

pub fn kubernetes_initialization_options_schema() -> Value {
    json!({
        "type": "object",
        "description": "Raw initialization options passed through to yaml-language-server when Kubernetes mode owns the buffer.",
        "default": {},
        "additionalProperties": true
    })
}

pub fn helm_initialization_options_schema() -> Value {
    json!({
        "type": "object",
        "description": "Raw initialization options passed through to helm-language-server.",
        "default": {},
        "additionalProperties": true
    })
}

pub fn helm_workspace_configuration_schema() -> Value {
    json!({
        "type": "object",
        "description": "Workspace configuration passed through to helm-language-server.",
        "default": {},
        "additionalProperties": true,
        "properties": {
            "helm-ls": helm_ls_settings_schema()
        }
    })
}

fn raw_passthrough_schema(description: &str) -> Value {
    json!({
        "type": "object",
        "description": description,
        "default": {},
        "additionalProperties": true
    })
}

fn helm_ls_settings_schema() -> Value {
    object_schema(
        "Helm language server settings. The typed fields cover the official helm-ls configuration surface, while yamlls.config remains a raw yaml-language-server passthrough object.",
        Some(helm_ls_settings_defaults()),
        true,
        &helm_ls_settings_properties(),
    )
}

fn helm_ls_settings_defaults() -> Value {
    json!({
        "logLevel": "info",
        "valuesFiles": {
            "mainValuesFile": "values.yaml",
            "lintOverlayValuesFile": "values.lint.yaml",
            "additionalValuesFilesGlobPattern": "values*.yaml"
        },
        "helmLint": {
            "enabled": true,
            "ignoredMessages": []
        },
        "yamlls": {
            "enabled": true,
            "enabledForFilesGlob": "*.{yaml,yml}",
            "diagnosticsLimit": 50,
            "showDiagnosticsDirectly": false,
            "path": "yaml-language-server",
            "initTimeoutSeconds": 3
        }
    })
}

fn helm_ls_settings_properties() -> Value {
    json!({
        "logLevel": string_schema(
            "Log verbosity for helm-ls.",
            Some("info"),
        ),
        "valuesFiles": helm_ls_values_files_schema(),
        "helmLint": helm_ls_helm_lint_schema(),
        "yamlls": helm_ls_yamlls_schema()
    })
}

fn helm_ls_values_files_schema() -> Value {
    object_schema(
        "Values file discovery settings.",
        Some(json!({
            "mainValuesFile": "values.yaml",
            "lintOverlayValuesFile": "values.lint.yaml",
            "additionalValuesFilesGlobPattern": "values*.yaml"
        })),
        true,
        &json!({
            "mainValuesFile": string_schema(
                "Main values file used as the base for Helm values support.",
                Some("values.yaml"),
            ),
            "lintOverlayValuesFile": string_schema(
                "Overlay values file merged into the main values file for helm lint.",
                Some("values.lint.yaml"),
            ),
            "additionalValuesFilesGlobPattern": string_schema(
                "Glob pattern for additional values files exposed to completion and hover.",
                Some("values*.yaml"),
            )
        }),
    )
}

fn helm_ls_helm_lint_schema() -> Value {
    object_schema(
        "Helm lint diagnostics settings.",
        Some(json!({
            "enabled": true,
            "ignoredMessages": []
        })),
        true,
        &json!({
            "enabled": boolean_schema(
                "Enable diagnostics gathered from helm lint.",
                Some(true),
            ),
            "ignoredMessages": string_array_schema(
                "Helm lint messages to ignore.",
                Some(json!([])),
            )
        }),
    )
}

fn helm_ls_yamlls_schema() -> Value {
    object_schema(
        "yaml-language-server integration inside helm-ls.",
        Some(json!({
            "enabled": true,
            "enabledForFilesGlob": "*.{yaml,yml}",
            "diagnosticsLimit": 50,
            "showDiagnosticsDirectly": false,
            "path": "yaml-language-server",
            "initTimeoutSeconds": 3
        })),
        true,
        &json!({
            "enabled": boolean_schema(
                "Enable yaml-language-server integration inside helm-ls.",
                Some(true),
            ),
            "enabledForFilesGlob": string_schema(
                "Glob that controls which files get yaml-language-server integration.",
                Some("*.{yaml,yml}"),
            ),
            "diagnosticsLimit": integer_schema(
                "Maximum number of yaml-language-server diagnostics shown per file. Set to 0 to disable diagnostics while keeping other yamlls features.",
                Some(50),
                Some(0),
            ),
            "showDiagnosticsDirectly": boolean_schema(
                "Show yaml-language-server diagnostics while typing.",
                Some(false),
            ),
            "path": string_or_string_array_schema(
                "Path to yaml-language-server. Can be a command string or an argv array.",
                Some(json!("yaml-language-server")),
            ),
            "initTimeoutSeconds": integer_schema(
                "Initialization timeout for the embedded yaml-language-server.",
                Some(3),
                Some(1),
            ),
            "config": raw_passthrough_schema(
                "Raw yaml-language-server configuration passed through by helm-ls.",
            )
        }),
    )
}

fn kubernetes_curated_settings_schema() -> Value {
    object_schema(
        "Extension-owned settings for the Kubernetes editing experience. These cover the most important yaml-language-server knobs with typed Settings Editor support, while raw settings.yaml remains the escape hatch for niche upstream options.",
        Some(kubernetes_curated_settings_defaults()),
        false,
        &kubernetes_curated_settings_properties(),
    )
}

fn kubernetes_curated_settings_defaults() -> Value {
    json!({
        "includeDefaultSchemas": true,
        "injectIntoYamlLanguageServer": true,
        "schemaAssociations": {},
        "yamlVersion": "1.1",
        "validate": true,
        "hover": true,
        "completion": true,
        "disableDefaultProperties": true,
        "maxItemsComputed": 10000,
        "customTags": default_custom_tags(),
        "schemaStore": {
            "enable": true
        },
        "style": {
            "flowMapping": "allow",
            "flowSequence": "allow"
        },
        "format": {
            "enable": true,
            "bracketSpacing": true,
            "printWidth": 120
        },
        "editor": {
            "tabSize": 2
        }
    })
}

fn kubernetes_curated_settings_properties() -> Value {
    json!({
        "includeDefaultSchemas": boolean_schema(
            "Keep the extension's default Kubernetes, Kustomize, and Helm chart schema associations inside Kubernetes-mode buffers.",
            Some(true),
        ),
        "injectIntoYamlLanguageServer": boolean_schema(
            "Mirror the extension-owned schema associations into built-in yaml-language-server for plain YAML buffers that stay in YAML mode.",
            Some(true),
        ),
        "schemaAssociations": schema_associations_schema(),
        "yamlVersion": string_enum_schema(
            "YAML version used for Kubernetes-mode buffers.",
            Some("1.1"),
            &["1.1", "1.2"],
        ),
        "validate": boolean_schema(
            "Enable diagnostics and schema validation in Kubernetes-mode buffers.",
            Some(true),
        ),
        "hover": boolean_schema(
            "Enable hover information in Kubernetes-mode buffers.",
            Some(true),
        ),
        "completion": boolean_schema(
            "Enable completion in Kubernetes-mode buffers.",
            Some(true),
        ),
        "disableDefaultProperties": boolean_schema(
            "Hide inferred placeholder properties from completion items.",
            Some(true),
        ),
        "maxItemsComputed": integer_schema(
            "Maximum number of outline, folding, or breadcrumb items computed by yaml-language-server.",
            Some(10_000),
            Some(0),
        ),
        "customTags": string_array_schema(
            "Custom YAML tags understood by yaml-language-server.",
            Some(default_custom_tags()),
        ),
        "keyOrdering": boolean_schema(
            "Enable key ordering validation.",
            None,
        ),
        "enableStrictSchemaValidation": boolean_schema(
            "Promote schema validation warnings to errors when a property does not match the associated schema.",
            None,
        ),
        "schemaStore": curated_schema_store_schema(),
        "kubernetesCRDStore": curated_kubernetes_crd_store_schema(),
        "suggest": curated_suggest_schema(),
        "style": curated_style_schema(),
        "format": curated_format_schema(),
        "editor": curated_editor_schema()
    })
}

fn curated_schema_store_schema() -> Value {
    object_schema(
        "Schema Store settings for Kubernetes-mode buffers.",
        Some(json!({
            "enable": true
        })),
        false,
        &json!({
            "enable": boolean_schema(
                "Enable Schema Store lookups.",
                Some(true),
            ),
            "url": string_schema(
                "Override the Schema Store catalog URL.",
                None,
            )
        }),
    )
}

fn curated_kubernetes_crd_store_schema() -> Value {
    object_schema(
        "Kubernetes CRD store integration for fetching remote CRD schemas.",
        None,
        false,
        &json!({
            "enable": boolean_schema(
                "Enable remote Kubernetes CRD schema lookups.",
                None,
            ),
            "url": string_schema(
                "Override the Kubernetes CRD store base URL.",
                None,
            )
        }),
    )
}

fn curated_suggest_schema() -> Value {
    object_schema(
        "Completion suggestion behavior.",
        None,
        false,
        &json!({
            "parentSkeletonSelectedFirst": boolean_schema(
                "Prefer selecting the parent skeleton completion before leaf properties.",
                None,
            )
        }),
    )
}

fn curated_style_schema() -> Value {
    object_schema(
        "Formatting style controls for flow collections.",
        Some(json!({
            "flowMapping": "allow",
            "flowSequence": "allow"
        })),
        false,
        &json!({
            "flowMapping": string_enum_schema(
                "Allow or forbid flow-style mappings such as {a: 1}.",
                Some("allow"),
                &["allow", "forbid"],
            ),
            "flowSequence": string_enum_schema(
                "Allow or forbid flow-style sequences such as [a, b].",
                Some("allow"),
                &["allow", "forbid"],
            )
        }),
    )
}

fn curated_format_schema() -> Value {
    object_schema(
        "Formatting controls for Kubernetes-mode buffers.",
        Some(json!({
            "enable": true,
            "bracketSpacing": true,
            "printWidth": 120
        })),
        false,
        &json!({
            "enable": boolean_schema(
                "Enable formatting.",
                Some(true),
            ),
            "singleQuote": boolean_schema(
                "Prefer single quotes when formatting.",
                None,
            ),
            "bracketSpacing": boolean_schema(
                "Insert spaces inside flow collection brackets.",
                Some(true),
            ),
            "printWidth": integer_schema(
                "Preferred line width used during formatting.",
                Some(120),
                Some(1),
            ),
            "proseWrap": string_enum_schema(
                "How prose content should wrap when formatting.",
                None,
                &["preserve", "never", "always"],
            )
        }),
    )
}

fn curated_editor_schema() -> Value {
    object_schema(
        "Internal yaml-language-server editor defaults used for formatting decisions. To configure Zed's editor behavior, use languages.Kubernetes in Zed settings.",
        Some(json!({
            "tabSize": 2
        })),
        false,
        &json!({
            "tabSize": integer_schema(
                "Indent size for Kubernetes-mode buffers.",
                Some(2),
                Some(1),
            ),
            "formatOnType": boolean_schema(
                "Enable format-on-type for Kubernetes-mode buffers.",
                None,
            )
        }),
    )
}

fn boolean_schema(description: &str, default: Option<bool>) -> Value {
    let mut schema = json!({
        "type": "boolean",
        "description": description,
    });
    if let Some(default) = default {
        schema["default"] = json!(default);
    }
    schema
}

fn integer_schema(description: &str, default: Option<i64>, minimum: Option<i64>) -> Value {
    let mut schema = json!({
        "type": "integer",
        "description": description,
    });
    if let Some(default) = default {
        schema["default"] = json!(default);
    }
    if let Some(minimum) = minimum {
        schema["minimum"] = json!(minimum);
    }
    schema
}

fn string_schema(description: &str, default: Option<&str>) -> Value {
    let mut schema = json!({
        "type": "string",
        "description": description,
    });
    if let Some(default) = default {
        schema["default"] = json!(default);
    }
    schema
}

fn string_enum_schema(description: &str, default: Option<&str>, values: &[&str]) -> Value {
    let mut schema = string_schema(description, default);
    schema["enum"] = Value::Array(
        values
            .iter()
            .map(|value| Value::String((*value).to_string()))
            .collect(),
    );
    schema
}

fn string_array_schema(description: &str, default: Option<Value>) -> Value {
    let mut schema = json!({
        "type": "array",
        "description": description,
        "items": {
            "type": "string"
        }
    });
    if let Some(default) = default {
        schema["default"] = default;
    }
    schema
}

fn string_or_string_array_schema(description: &str, default: Option<Value>) -> Value {
    let mut schema = json!({
        "description": description,
        "anyOf": [
            {
                "type": "string"
            },
            {
                "type": "array",
                "items": {
                    "type": "string"
                }
            }
        ]
    });
    if let Some(default) = default {
        schema["default"] = default;
    }
    schema
}

fn object_schema(
    description: &str,
    default: Option<Value>,
    additional_properties: bool,
    properties: &Value,
) -> Value {
    let mut schema = json!({
        "type": "object",
        "description": description,
        "additionalProperties": additional_properties,
        "properties": properties.clone(),
    });
    if let Some(default) = default {
        schema["default"] = default;
    }
    schema
}

fn schema_associations_schema() -> Value {
    json!({
        "type": "object",
        "default": {},
        "description": "Additional schema-to-glob associations merged into yaml.schemas. Relative schema paths are resolved against the worktree root and ~/ paths are resolved against HOME.",
        "additionalProperties": {
            "type": "array",
            "items": {
                "type": "string"
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_schema_exposes_curated_kubernetes_defaults() {
        let schema = kubernetes_workspace_configuration_schema();

        assert_eq!(schema["type"], "object");
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["includeDefaultSchemas"]["default"],
            true,
        );
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["injectIntoYamlLanguageServer"]["default"],
            true,
        );
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["schemaAssociations"]["type"],
            "object",
        );
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["schemaStore"]["properties"]["enable"]
                ["default"],
            true,
        );
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["format"]["properties"]["printWidth"]
                ["default"],
            120,
        );
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["editor"]["properties"]["tabSize"]["default"],
            2,
        );
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["yamlVersion"]["enum"],
            json!(["1.1", "1.2"]),
        );
    }

    #[test]
    fn workspace_schema_yaml_block_description_clarifies_scope() {
        let schema = kubernetes_workspace_configuration_schema();
        let desc = schema["properties"]["[yaml]"]["description"]
            .as_str()
            .expect("[yaml] should have a description");
        assert!(
            desc.contains("yaml-language-server"),
            "description should mention yaml-language-server",
        );
        assert!(
            desc.contains("languages.Kubernetes"),
            "description should point to languages.Kubernetes",
        );
    }

    #[test]
    fn initialization_and_helm_schemas_are_typed_objects() {
        let initialization_schema = kubernetes_initialization_options_schema();
        let helm_schema = helm_workspace_configuration_schema();

        assert_eq!(initialization_schema["type"], "object");
        assert_eq!(initialization_schema["additionalProperties"], true);
        assert_eq!(helm_schema["type"], "object");
        assert_eq!(helm_schema["properties"]["helm-ls"]["type"], "object");
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["properties"]["yamlls"]["properties"]["enabled"]["default"],
            true,
        );
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["properties"]["valuesFiles"]["properties"]["mainValuesFile"]
                ["default"],
            "values.yaml",
        );
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["properties"]["yamlls"]["properties"]["path"]["default"],
            "yaml-language-server",
        );
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["properties"]["yamlls"]["properties"]["config"]["type"],
            "object",
        );
    }

    #[test]
    fn helm_initialization_options_schema_is_a_permissive_object() {
        let schema = helm_initialization_options_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], true);
        assert_eq!(schema["default"], json!({}));
    }
}
