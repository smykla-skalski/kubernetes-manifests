use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use url::Url;

#[derive(Debug, Clone)]
struct CuratedKubernetesSettings {
    include_default_schemas: bool,
    inject_into_yaml_language_server: bool,
    schema_associations: Map<String, Value>,
    workspace_overrides: Value,
}

impl Default for CuratedKubernetesSettings {
    fn default() -> Self {
        Self {
            include_default_schemas: true,
            inject_into_yaml_language_server: true,
            schema_associations: Map::new(),
            workspace_overrides: Value::Object(Map::new()),
        }
    }
}

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

fn default_schemas() -> Value {
    json!({
        "kubernetes": default_schema_globs(),
        "https://json.schemastore.org/kustomization.json": [
            "kustomization.yaml",
            "kustomization.yml"
        ],
        "https://json.schemastore.org/chart.json": [
            "Chart.yaml",
            "Chart.yml"
        ]
    })
}

const fn default_custom_tags() -> Value {
    Value::Array(Vec::new())
}

pub fn default_workspace_configuration() -> Value {
    json!({
        "[yaml]": {
            "editor.tabSize": 2
        },
        "yaml": {
            "yamlVersion": "1.1",
            "schemaStore": {
                "enable": true
            },
            "validate": true,
            "hover": true,
            "hoverAnchor": true,
            "completion": true,
            "disableDefaultProperties": true,
            "maxItemsComputed": 10000,
            "customTags": default_custom_tags(),
            "style": {
                "flowMapping": "allow",
                "flowSequence": "allow"
            },
            "format": {
                "enable": true,
                "bracketSpacing": true,
                "printWidth": 120
            },
            "schemas": default_schemas()
        }
    })
}

pub fn kubernetes_workspace_configuration(
    user_settings: Option<Value>,
    worktree_root: Option<&str>,
    home_dir: Option<&str>,
) -> Value {
    let mut configuration = default_workspace_configuration();

    if let Some(user_settings) = user_settings {
        let (curated_settings, raw_workspace_settings) = split_user_settings(user_settings);
        apply_curated_workspace_settings(
            &mut configuration,
            &curated_settings,
            worktree_root,
            home_dir,
        );
        let raw_workspace_settings =
            resolve_workspace_schema_paths(raw_workspace_settings, worktree_root, home_dir);
        merge_json_value_into(raw_workspace_settings, &mut configuration);
    }

    configuration
}

pub fn yaml_server_injection_configuration(
    user_settings: Option<Value>,
    worktree_root: Option<&str>,
    home_dir: Option<&str>,
) -> Option<Value> {
    let curated_settings = user_settings
        .map(split_user_settings)
        .map(|(settings, _)| settings)
        .unwrap_or_default();

    if !curated_settings.inject_into_yaml_language_server {
        return None;
    }

    let mut schemas = if curated_settings.include_default_schemas {
        default_schema_map()
    } else {
        Map::new()
    };
    let resolved_schema_associations = resolve_schema_map_paths(
        curated_settings.schema_associations,
        worktree_root,
        home_dir,
    );
    merge_json_object_into(resolved_schema_associations, &mut schemas);

    if schemas.is_empty() {
        return None;
    }

    Some(json!({
        "yaml": {
            "schemas": Value::Object(schemas)
        }
    }))
}

#[cfg(feature = "next")]
pub fn kubernetes_workspace_configuration_schema() -> Value {
    json!({
        "type": "object",
        "description": "Workspace configuration passed to kubernetes-language-server. Use the kubernetes block for extension-owned settings and yaml for raw yaml-language-server settings that apply when Kubernetes mode owns the buffer.",
        "default": {},
        "additionalProperties": true,
        "properties": {
            "kubernetes": kubernetes_curated_settings_schema(),
            "yaml": raw_passthrough_schema("Raw yaml-language-server workspace settings applied only to Kubernetes-mode buffers. These settings override the extension defaults and the curated kubernetes block."),
            "[yaml]": raw_passthrough_schema("Editor settings applied to Kubernetes-mode YAML buffers.")
        }
    })
}

#[cfg(feature = "next")]
pub fn kubernetes_initialization_options_schema() -> Value {
    json!({
        "type": "object",
        "description": "Raw initialization options passed through to yaml-language-server when Kubernetes mode owns the buffer.",
        "default": {},
        "additionalProperties": true
    })
}

#[cfg(feature = "next")]
pub fn helm_initialization_options_schema() -> Value {
    json!({
        "type": "object",
        "description": "Raw initialization options passed through to helm-language-server.",
        "default": {},
        "additionalProperties": true
    })
}

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
fn raw_passthrough_schema(description: &str) -> Value {
    json!({
        "type": "object",
        "description": description,
        "default": {},
        "additionalProperties": true
    })
}

#[cfg(feature = "next")]
fn helm_ls_settings_schema() -> Value {
    object_schema(
        "Helm language server settings. The typed fields cover the official helm-ls configuration surface, while yamlls.config remains a raw yaml-language-server passthrough object.",
        Some(helm_ls_settings_defaults()),
        true,
        &helm_ls_settings_properties(),
    )
}

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
fn kubernetes_curated_settings_schema() -> Value {
    object_schema(
        "Extension-owned settings for the Kubernetes editing experience. These cover the most important yaml-language-server knobs with typed Settings Editor support, while raw settings.yaml remains the escape hatch for niche upstream options.",
        Some(kubernetes_curated_settings_defaults()),
        false,
        &kubernetes_curated_settings_properties(),
    )
}

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
fn curated_editor_schema() -> Value {
    object_schema(
        "Editor defaults applied to Kubernetes-mode YAML buffers.",
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

#[cfg(feature = "next")]
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

fn split_user_settings(user_settings: Value) -> (CuratedKubernetesSettings, Value) {
    match user_settings {
        Value::Object(mut user_settings) => {
            let curated_settings = user_settings
                .remove("kubernetes")
                .as_ref()
                .map(parse_curated_kubernetes_settings)
                .unwrap_or_default();
            (curated_settings, Value::Object(user_settings))
        }
        user_settings => (CuratedKubernetesSettings::default(), user_settings),
    }
}

fn parse_curated_kubernetes_settings(settings: &Value) -> CuratedKubernetesSettings {
    let mut curated_settings = CuratedKubernetesSettings::default();
    let Some(settings) = settings.as_object() else {
        return curated_settings;
    };

    if let Some(include_default_schemas) = settings
        .get("includeDefaultSchemas")
        .and_then(Value::as_bool)
    {
        curated_settings.include_default_schemas = include_default_schemas;
    }

    if let Some(inject_into_yaml_language_server) = settings
        .get("injectIntoYamlLanguageServer")
        .and_then(Value::as_bool)
    {
        curated_settings.inject_into_yaml_language_server = inject_into_yaml_language_server;
    }

    if let Some(Value::Object(schema_associations)) = settings.get("schemaAssociations") {
        curated_settings
            .schema_associations
            .clone_from(schema_associations);
    }
    parse_curated_root_scalars(settings, &mut curated_settings.workspace_overrides);
    parse_curated_schema_store_settings(settings, &mut curated_settings.workspace_overrides);
    parse_curated_kubernetes_crd_store_settings(
        settings,
        &mut curated_settings.workspace_overrides,
    );
    parse_curated_suggest_settings(settings, &mut curated_settings.workspace_overrides);
    parse_curated_style_settings(settings, &mut curated_settings.workspace_overrides);
    parse_curated_format_settings(settings, &mut curated_settings.workspace_overrides);
    parse_curated_editor_settings(settings, &mut curated_settings.workspace_overrides);

    curated_settings
}

fn parse_curated_root_scalars(settings: &Map<String, Value>, workspace_overrides: &mut Value) {
    if let Some(yaml_version) = settings
        .get("yamlVersion")
        .and_then(Value::as_str)
        .filter(|yaml_version| matches!(*yaml_version, "1.1" | "1.2"))
    {
        set_workspace_override(
            workspace_overrides,
            &["yaml", "yamlVersion"],
            json!(yaml_version),
        );
    }

    for setting_key in [
        "validate",
        "hover",
        "completion",
        "disableDefaultProperties",
        "keyOrdering",
        "enableStrictSchemaValidation",
    ] {
        parse_curated_object_bool(
            workspace_overrides,
            settings,
            setting_key,
            &["yaml", setting_key],
        );
    }

    parse_curated_object_integer(
        workspace_overrides,
        settings,
        "maxItemsComputed",
        non_negative_integer,
        &["yaml", "maxItemsComputed"],
    );

    if let Some(custom_tags) = settings.get("customTags").and_then(string_array_value) {
        set_workspace_override(workspace_overrides, &["yaml", "customTags"], custom_tags);
    }
}

fn parse_curated_schema_store_settings(
    settings: &Map<String, Value>,
    workspace_overrides: &mut Value,
) {
    if let Some(Value::Object(schema_store)) = settings.get("schemaStore") {
        parse_curated_object_bool(
            workspace_overrides,
            schema_store,
            "enable",
            &["yaml", "schemaStore", "enable"],
        );
        parse_curated_object_string(
            workspace_overrides,
            schema_store,
            "url",
            &["yaml", "schemaStore", "url"],
        );
    }
}

fn parse_curated_kubernetes_crd_store_settings(
    settings: &Map<String, Value>,
    workspace_overrides: &mut Value,
) {
    if let Some(Value::Object(kubernetes_crd_store)) = settings.get("kubernetesCRDStore") {
        parse_curated_object_bool(
            workspace_overrides,
            kubernetes_crd_store,
            "enable",
            &["yaml", "kubernetesCRDStore", "enable"],
        );
        parse_curated_object_string(
            workspace_overrides,
            kubernetes_crd_store,
            "url",
            &["yaml", "kubernetesCRDStore", "url"],
        );
    }
}

fn parse_curated_suggest_settings(settings: &Map<String, Value>, workspace_overrides: &mut Value) {
    if let Some(Value::Object(suggest)) = settings.get("suggest") {
        parse_curated_object_bool(
            workspace_overrides,
            suggest,
            "parentSkeletonSelectedFirst",
            &["yaml", "suggest", "parentSkeletonSelectedFirst"],
        );
    }
}

fn parse_curated_style_settings(settings: &Map<String, Value>, workspace_overrides: &mut Value) {
    if let Some(Value::Object(style)) = settings.get("style") {
        parse_curated_object_string_enum(
            workspace_overrides,
            style,
            "flowMapping",
            &["allow", "forbid"],
            &["yaml", "style", "flowMapping"],
        );
        parse_curated_object_string_enum(
            workspace_overrides,
            style,
            "flowSequence",
            &["allow", "forbid"],
            &["yaml", "style", "flowSequence"],
        );
    }
}

fn parse_curated_format_settings(settings: &Map<String, Value>, workspace_overrides: &mut Value) {
    if let Some(Value::Object(format)) = settings.get("format") {
        parse_curated_object_bool(
            workspace_overrides,
            format,
            "enable",
            &["yaml", "format", "enable"],
        );
        parse_curated_object_bool(
            workspace_overrides,
            format,
            "singleQuote",
            &["yaml", "format", "singleQuote"],
        );
        parse_curated_object_bool(
            workspace_overrides,
            format,
            "bracketSpacing",
            &["yaml", "format", "bracketSpacing"],
        );
        parse_curated_object_integer(
            workspace_overrides,
            format,
            "printWidth",
            positive_integer,
            &["yaml", "format", "printWidth"],
        );
        parse_curated_object_string_enum(
            workspace_overrides,
            format,
            "proseWrap",
            &["preserve", "never", "always"],
            &["yaml", "format", "proseWrap"],
        );
    }
}

fn parse_curated_editor_settings(settings: &Map<String, Value>, workspace_overrides: &mut Value) {
    if let Some(Value::Object(editor)) = settings.get("editor") {
        parse_curated_object_integer(
            workspace_overrides,
            editor,
            "tabSize",
            positive_integer,
            &["[yaml]", "editor.tabSize"],
        );
        parse_curated_object_bool(
            workspace_overrides,
            editor,
            "formatOnType",
            &["[yaml]", "editor.formatOnType"],
        );
    }
}

fn apply_curated_workspace_settings(
    configuration: &mut Value,
    curated_settings: &CuratedKubernetesSettings,
    worktree_root: Option<&str>,
    home_dir: Option<&str>,
) {
    merge_json_value_into(curated_settings.workspace_overrides.clone(), configuration);

    let schemas = ensure_yaml_schemas_object(configuration);
    if !curated_settings.include_default_schemas {
        schemas.clear();
    }

    let resolved_schema_associations = resolve_schema_map_paths(
        curated_settings.schema_associations.clone(),
        worktree_root,
        home_dir,
    );
    merge_json_object_into(resolved_schema_associations, schemas);
}

fn parse_curated_object_bool(
    workspace_overrides: &mut Value,
    settings: &Map<String, Value>,
    key: &str,
    path: &[&str],
) {
    if let Some(value) = settings.get(key).and_then(Value::as_bool) {
        set_workspace_override(workspace_overrides, path, json!(value));
    }
}

fn parse_curated_object_string(
    workspace_overrides: &mut Value,
    settings: &Map<String, Value>,
    key: &str,
    path: &[&str],
) {
    if let Some(value) = settings.get(key).and_then(Value::as_str) {
        set_workspace_override(workspace_overrides, path, json!(value));
    }
}

fn parse_curated_object_string_enum(
    workspace_overrides: &mut Value,
    settings: &Map<String, Value>,
    key: &str,
    allowed_values: &[&str],
    path: &[&str],
) {
    if let Some(value) = settings
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| allowed_values.contains(value))
    {
        set_workspace_override(workspace_overrides, path, json!(value));
    }
}

fn parse_curated_object_integer(
    workspace_overrides: &mut Value,
    settings: &Map<String, Value>,
    key: &str,
    parser: fn(&Value) -> Option<i64>,
    path: &[&str],
) {
    if let Some(value) = settings.get(key).and_then(parser) {
        set_workspace_override(workspace_overrides, path, json!(value));
    }
}

fn string_array_value(value: &Value) -> Option<Value> {
    let values = value.as_array()?;
    let mut strings = Vec::with_capacity(values.len());
    for value in values {
        strings.push(Value::String(value.as_str()?.to_string()));
    }
    Some(Value::Array(strings))
}

fn positive_integer(value: &Value) -> Option<i64> {
    value.as_i64().filter(|value| *value > 0)
}

fn non_negative_integer(value: &Value) -> Option<i64> {
    value.as_i64().filter(|value| *value >= 0)
}

fn set_workspace_override(workspace_overrides: &mut Value, path: &[&str], value: Value) {
    let mut current = workspace_overrides;
    for segment in &path[..path.len().saturating_sub(1)] {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }

        let current_object = current
            .as_object_mut()
            .expect("workspace overrides should remain objects");
        current = current_object
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }

    if !current.is_object() {
        *current = Value::Object(Map::new());
    }

    if let Some(last_segment) = path.last() {
        current
            .as_object_mut()
            .expect("workspace overrides should remain objects")
            .insert((*last_segment).to_string(), value);
    }
}

fn ensure_yaml_schemas_object(configuration: &mut Value) -> &mut Map<String, Value> {
    let configuration = configuration
        .as_object_mut()
        .expect("workspace configuration should be an object");
    let yaml = configuration
        .entry("yaml".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("yaml settings should be an object");
    yaml.entry("schemas".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("yaml.schemas should be an object")
}

fn resolve_workspace_schema_paths(
    mut settings: Value,
    worktree_root: Option<&str>,
    home_dir: Option<&str>,
) -> Value {
    let yaml_schemas = settings
        .as_object_mut()
        .and_then(|settings| settings.get_mut("yaml"))
        .and_then(Value::as_object_mut)
        .and_then(|yaml| yaml.remove("schemas"));

    let Some(Value::Object(schemas)) = yaml_schemas else {
        return settings;
    };

    let resolved = resolve_schema_map_paths(schemas, worktree_root, home_dir);

    if let Some(yaml) = settings
        .as_object_mut()
        .and_then(|settings| settings.get_mut("yaml"))
        .and_then(Value::as_object_mut)
    {
        yaml.insert("schemas".to_string(), Value::Object(resolved));
    }

    settings
}

fn resolve_schema_map_paths(
    schemas: Map<String, Value>,
    worktree_root: Option<&str>,
    home_dir: Option<&str>,
) -> Map<String, Value> {
    schemas
        .into_iter()
        .map(|(schema_path, globs)| {
            (
                resolve_schema_path(&schema_path, worktree_root, home_dir),
                globs,
            )
        })
        .collect()
}

fn resolve_schema_path(
    schema_path: &str,
    worktree_root: Option<&str>,
    home_dir: Option<&str>,
) -> String {
    if let Some((home_dir, rest)) = home_dir.zip(schema_path.strip_prefix("~/")) {
        return local_schema_path_to_uri(PathBuf::from(home_dir).join(rest), schema_path);
    }

    if let Some(worktree_root) = worktree_root.filter(|_| schema_path.starts_with('.')) {
        return local_schema_path_to_uri(
            PathBuf::from(worktree_root)
                .join(schema_path.strip_prefix("./").unwrap_or(schema_path)),
            schema_path,
        );
    }

    if Path::new(schema_path).is_absolute() {
        return local_schema_path_to_uri(PathBuf::from(schema_path), schema_path);
    }

    schema_path.to_string()
}

fn local_schema_path_to_uri(path: PathBuf, fallback: &str) -> String {
    Url::from_file_path(path).map_or_else(|()| fallback.to_string(), |url| url.to_string())
}

fn default_schema_map() -> Map<String, Value> {
    default_schemas().as_object().cloned().unwrap_or_default()
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

    fn file_url(path: &str) -> String {
        Url::from_file_path(path)
            .expect("test path should convert to file URL")
            .to_string()
    }

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
    fn default_config_enables_schema_store_and_validation() {
        let configuration = default_workspace_configuration();

        assert_eq!(configuration["yaml"]["schemaStore"]["enable"], true);
        assert_eq!(configuration["yaml"]["validate"], true);
        assert_eq!(configuration["yaml"]["hover"], true);
        assert_eq!(configuration["yaml"]["completion"], true);
    }

    #[test]
    fn default_config_uses_yaml_1_1_for_kubernetes_compat() {
        let configuration = default_workspace_configuration();

        assert_eq!(configuration["yaml"]["yamlVersion"], "1.1");
    }

    #[test]
    fn default_config_sets_kubernetes_friendly_defaults() {
        let configuration = default_workspace_configuration();

        assert_eq!(configuration["yaml"]["hoverAnchor"], true);
        assert_eq!(configuration["yaml"]["disableDefaultProperties"], true);
        assert_eq!(configuration["yaml"]["maxItemsComputed"], 10000);
        assert_eq!(configuration["yaml"]["style"]["flowMapping"], "allow");
        assert_eq!(configuration["yaml"]["style"]["flowSequence"], "allow");
        assert_eq!(configuration["yaml"]["format"]["bracketSpacing"], true);
        assert_eq!(configuration["yaml"]["format"]["printWidth"], 120);
    }

    #[test]
    fn default_config_has_empty_custom_tags() {
        let configuration = default_workspace_configuration();
        let tags = configuration["yaml"]["customTags"]
            .as_array()
            .expect("customTags should be an array");

        assert!(tags.is_empty(), "customTags should default to empty");
    }

    #[test]
    fn yaml_injection_defaults_to_extension_owned_schema_associations() {
        let configuration =
            yaml_server_injection_configuration(None, Some("/home/user/project"), None)
                .expect("yaml injection should be enabled by default");
        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");

        assert!(
            schemas.contains_key("kubernetes"),
            "additional config should inject kubernetes schema",
        );
        assert!(
            schemas.contains_key("https://json.schemastore.org/kustomization.json"),
            "additional config should inject kustomization schema",
        );
        assert!(
            schemas.contains_key("https://json.schemastore.org/chart.json"),
            "additional config should inject chart schema",
        );
        assert!(
            !configuration["yaml"]
                .as_object()
                .unwrap()
                .contains_key("completion"),
            "additional config should not override yaml server settings",
        );
    }

    #[test]
    fn default_config_contains_kustomization_and_chart_schemas() {
        let configuration = default_workspace_configuration();
        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");

        assert!(
            schemas.contains_key("https://json.schemastore.org/kustomization.json"),
            "default config should include kustomization schema",
        );
        assert!(
            schemas.contains_key("https://json.schemastore.org/chart.json"),
            "default config should include chart schema",
        );
    }

    #[test]
    fn raw_workspace_settings_preserve_defaults_and_add_nested_keys() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "yaml": {
                    "validate": true,
                    "hover": true
                }
            })),
            None,
            None,
        );

        assert_eq!(configuration["yaml"]["format"]["enable"], true);
        assert_eq!(configuration["yaml"]["validate"], true);
        assert_eq!(configuration["yaml"]["hover"], true);
        assert_eq!(configuration["[yaml]"]["editor.tabSize"], 2);
    }

    #[test]
    fn raw_yaml_settings_can_override_default_schema_globs() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "kubernetes": ["*.tmpl.yaml"]
                    }
                }
            })),
            None,
            None,
        );

        assert_eq!(
            configuration["yaml"]["schemas"]["kubernetes"],
            json!(["*.tmpl.yaml"]),
        );
    }

    #[test]
    fn curated_schema_associations_merge_into_workspace_configuration() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "kubernetes": {
                    "schemaAssociations": {
                        "./schemas/custom.json": ["crds/*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        );
        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");

        assert!(
            schemas.contains_key(&file_url("/home/user/project/schemas/custom.json")),
            "curated relative schema paths should resolve against worktree root",
        );
        assert!(
            !configuration
                .as_object()
                .expect("workspace config should be an object")
                .contains_key("kubernetes"),
            "extension-owned settings should not be forwarded to yaml-language-server",
        );
    }

    #[test]
    fn curated_schema_associations_override_default_schema_associations() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "kubernetes": {
                    "schemaAssociations": {
                        "kubernetes": ["*.manual.yaml"]
                    }
                }
            })),
            None,
            None,
        );

        assert_eq!(
            configuration["yaml"]["schemas"]["kubernetes"],
            json!(["*.manual.yaml"]),
        );
    }

    #[test]
    fn curated_settings_can_disable_default_schema_associations() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "kubernetes": {
                    "includeDefaultSchemas": false
                }
            })),
            None,
            None,
        );
        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");

        assert!(
            schemas.is_empty(),
            "default schema associations should be removed when explicitly disabled",
        );
    }

    #[test]
    fn curated_settings_can_override_core_yaml_language_server_defaults() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "kubernetes": {
                    "yamlVersion": "1.2",
                    "validate": false,
                    "hover": false,
                    "completion": false,
                    "disableDefaultProperties": false,
                    "maxItemsComputed": 42,
                    "customTags": [],
                    "keyOrdering": true,
                    "enableStrictSchemaValidation": true
                }
            })),
            None,
            None,
        );

        assert_eq!(configuration["yaml"]["yamlVersion"], "1.2");
        assert_eq!(configuration["yaml"]["validate"], false);
        assert_eq!(configuration["yaml"]["hover"], false);
        assert_eq!(configuration["yaml"]["completion"], false);
        assert_eq!(configuration["yaml"]["disableDefaultProperties"], false);
        assert_eq!(configuration["yaml"]["maxItemsComputed"], 42);
        assert_eq!(configuration["yaml"]["customTags"], json!([]));
        assert_eq!(configuration["yaml"]["keyOrdering"], true);
        assert_eq!(configuration["yaml"]["enableStrictSchemaValidation"], true,);
    }

    #[test]
    fn curated_settings_can_override_nested_yaml_language_server_defaults() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "kubernetes": {
                    "schemaStore": {
                        "enable": false,
                        "url": "https://example.com/schema-store.json"
                    },
                    "kubernetesCRDStore": {
                        "enable": true,
                        "url": "https://example.com/crd-store"
                    },
                    "suggest": {
                        "parentSkeletonSelectedFirst": true
                    },
                    "style": {
                        "flowMapping": "forbid",
                        "flowSequence": "forbid"
                    },
                    "format": {
                        "enable": false,
                        "singleQuote": true,
                        "bracketSpacing": false,
                        "printWidth": 88,
                        "proseWrap": "never"
                    },
                    "editor": {
                        "tabSize": 4,
                        "formatOnType": true
                    }
                }
            })),
            None,
            None,
        );

        assert_eq!(configuration["yaml"]["schemaStore"]["enable"], false);
        assert_eq!(
            configuration["yaml"]["schemaStore"]["url"],
            "https://example.com/schema-store.json",
        );
        assert_eq!(configuration["yaml"]["kubernetesCRDStore"]["enable"], true);
        assert_eq!(
            configuration["yaml"]["kubernetesCRDStore"]["url"],
            "https://example.com/crd-store",
        );
        assert_eq!(
            configuration["yaml"]["suggest"]["parentSkeletonSelectedFirst"],
            true,
        );
        assert_eq!(configuration["yaml"]["style"]["flowMapping"], "forbid");
        assert_eq!(configuration["yaml"]["style"]["flowSequence"], "forbid");
        assert_eq!(configuration["yaml"]["format"]["enable"], false);
        assert_eq!(configuration["yaml"]["format"]["singleQuote"], true);
        assert_eq!(configuration["yaml"]["format"]["bracketSpacing"], false);
        assert_eq!(configuration["yaml"]["format"]["printWidth"], 88);
        assert_eq!(configuration["yaml"]["format"]["proseWrap"], "never");
        assert_eq!(configuration["[yaml]"]["editor.tabSize"], 4);
        assert_eq!(configuration["[yaml]"]["editor.formatOnType"], true);
    }

    #[test]
    fn raw_yaml_settings_override_curated_settings() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "kubernetes": {
                    "validate": false,
                    "editor": {
                        "tabSize": 4
                    },
                    "schemaAssociations": {
                        "./schemas/custom.json": ["crds/*.yaml"]
                    }
                },
                "[yaml]": {
                    "editor.tabSize": 8
                },
                "yaml": {
                    "validate": true,
                    "schemas": {
                        "./schemas/custom.json": ["overrides/*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        );

        let custom_schema_url = file_url("/home/user/project/schemas/custom.json");
        assert_eq!(
            configuration["yaml"]["schemas"][custom_schema_url.as_str()],
            json!(["overrides/*.yaml"]),
        );
        assert_eq!(configuration["yaml"]["validate"], true);
        assert_eq!(configuration["[yaml]"]["editor.tabSize"], 8);
    }

    #[test]
    fn raw_schema_paths_resolve_against_worktree_root() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "./schemas/custom.json": ["*.yaml"],
                        "https://example.com/schema.json": ["*.k8s.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        );

        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");
        assert!(
            schemas.contains_key(&file_url("/home/user/project/schemas/custom.json")),
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
    fn schema_paths_resolve_parent_directory_references() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "../shared/schema.json": ["*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        );

        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");
        assert!(
            schemas.contains_key(&file_url("/home/user/project/../shared/schema.json")),
            "parent-relative path should resolve against worktree root",
        );
        assert!(
            !schemas.contains_key("../shared/schema.json"),
            "original relative path should be replaced",
        );
    }

    #[test]
    fn schema_path_resolution_preserves_non_relative_paths() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "kubernetes": ["*.yaml"],
                        "/absolute/path/schema.json": ["*.yml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        );

        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");
        assert!(schemas.contains_key("kubernetes"));
        assert!(schemas.contains_key(&file_url("/absolute/path/schema.json")));
    }

    #[test]
    fn tilde_schema_paths_resolve_with_home_dir() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "kubernetes": {
                    "schemaAssociations": {
                        "~/schemas/custom.json": ["*.yaml"]
                    }
                },
                "yaml": {
                    "schemas": {
                        "~/schemas/raw.json": ["*.yml"]
                    }
                }
            })),
            Some("/home/user/project"),
            Some("/home/user"),
        );

        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");
        assert!(
            schemas.contains_key(&file_url("/home/user/schemas/custom.json")),
            "~/path should resolve to $HOME/path for curated schema associations",
        );
        assert!(
            schemas.contains_key(&file_url("/home/user/schemas/raw.json")),
            "~/path should resolve to $HOME/path for raw yaml.schemas",
        );
    }

    #[test]
    fn tilde_schema_paths_pass_through_without_home_dir() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "~/schemas/custom.json": ["*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        );

        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");
        assert!(
            schemas.contains_key("~/schemas/custom.json"),
            "~/path should pass through when HOME is not set",
        );
    }

    #[test]
    fn tilde_other_user_path_is_not_expanded() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "yaml": {
                    "schemas": {
                        "~other/schemas/custom.json": ["*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            Some("/home/user"),
        );

        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");
        assert!(
            schemas.contains_key("~other/schemas/custom.json"),
            "~other/path should not be expanded",
        );
    }

    #[test]
    fn yaml_injection_only_mirrors_extension_owned_schema_associations() {
        let configuration = yaml_server_injection_configuration(
            Some(json!({
                "kubernetes": {
                    "schemaAssociations": {
                        "./schemas/custom.json": ["crds/*.yaml"]
                    }
                },
                "yaml": {
                    "completion": false,
                    "schemas": {
                        "./schemas/raw.json": ["raw/*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        )
        .expect("yaml injection should be enabled");
        let yaml = configuration["yaml"]
            .as_object()
            .expect("yaml injection should be an object");
        let schemas = yaml
            .get("schemas")
            .and_then(Value::as_object)
            .expect("yaml.schemas should be an object");

        assert!(
            yaml.get("completion").is_none(),
            "raw yaml-language-server settings should not leak into built-in YAML injection",
        );
        assert!(
            schemas.contains_key(&file_url("/home/user/project/schemas/custom.json")),
            "curated schema associations should be mirrored into built-in YAML",
        );
        assert!(
            !schemas.contains_key(&file_url("/home/user/project/schemas/raw.json")),
            "raw yaml.schemas should not be mirrored into built-in YAML",
        );
    }

    #[test]
    fn yaml_injection_can_be_disabled() {
        let configuration = yaml_server_injection_configuration(
            Some(json!({
                "kubernetes": {
                    "injectIntoYamlLanguageServer": false
                }
            })),
            Some("/home/user/project"),
            None,
        );

        assert!(
            configuration.is_none(),
            "yaml injection should be disabled when explicitly requested",
        );
    }

    #[test]
    fn yaml_injection_respects_disabled_defaults() {
        let configuration = yaml_server_injection_configuration(
            Some(json!({
                "kubernetes": {
                    "includeDefaultSchemas": false,
                    "schemaAssociations": {
                        "./schemas/custom.json": ["crds/*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        )
        .expect("yaml injection should still include curated schemas");
        let schemas = configuration["yaml"]["schemas"]
            .as_object()
            .expect("yaml.schemas should be an object");

        assert!(
            !schemas.contains_key("kubernetes"),
            "default schema associations should be removed from YAML injection",
        );
        assert!(
            schemas.contains_key(&file_url("/home/user/project/schemas/custom.json")),
            "curated schema associations should still be mirrored",
        );
    }

    #[cfg(feature = "next")]
    #[test]
    fn workspace_schema_exposes_curated_kubernetes_defaults() {
        let schema = kubernetes_workspace_configuration_schema();

        assert_eq!(schema["type"], "object");
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["includeDefaultSchemas"]["default"],
            true,
        );
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["injectIntoYamlLanguageServer"]
                ["default"],
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
            schema["properties"]["kubernetes"]["properties"]["editor"]["properties"]["tabSize"]
                ["default"],
            2,
        );
        assert_eq!(
            schema["properties"]["kubernetes"]["properties"]["yamlVersion"]["enum"],
            json!(["1.1", "1.2"]),
        );
    }

    #[cfg(feature = "next")]
    #[test]
    fn initialization_and_helm_schemas_are_typed_objects() {
        let initialization_schema = kubernetes_initialization_options_schema();
        let helm_schema = helm_workspace_configuration_schema();

        assert_eq!(initialization_schema["type"], "object");
        assert_eq!(initialization_schema["additionalProperties"], true);
        assert_eq!(helm_schema["type"], "object");
        assert_eq!(helm_schema["properties"]["helm-ls"]["type"], "object");
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["properties"]["yamlls"]["properties"]["enabled"]
                ["default"],
            true,
        );
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["properties"]["valuesFiles"]["properties"]
                ["mainValuesFile"]["default"],
            "values.yaml",
        );
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["properties"]["yamlls"]["properties"]["path"]
                ["default"],
            "yaml-language-server",
        );
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["properties"]["yamlls"]["properties"]["config"]
                ["type"],
            "object",
        );
    }

    #[cfg(feature = "next")]
    #[test]
    fn helm_initialization_options_schema_is_a_permissive_object() {
        let schema = helm_initialization_options_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], true);
        assert_eq!(schema["default"], json!({}));
    }
}
