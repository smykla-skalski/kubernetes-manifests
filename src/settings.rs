use std::path::PathBuf;

use serde_json::{json, Map, Value};

#[derive(Debug, Clone)]
struct CuratedKubernetesSettings {
    include_default_schemas: bool,
    inject_into_yaml_language_server: bool,
    schema_associations: Map<String, Value>,
}

impl Default for CuratedKubernetesSettings {
    fn default() -> Self {
        Self {
            include_default_schemas: true,
            inject_into_yaml_language_server: true,
            schema_associations: Map::new(),
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

fn default_custom_tags() -> Value {
    json!([
        "!Ref",
        "!Sub",
        "!Sub sequence",
        "!GetAtt",
        "!GetAtt sequence",
        "!Fn::Sub",
        "!Fn::Sub sequence",
        "!FindInMap sequence",
        "!Join sequence",
        "!Select sequence",
        "!Split sequence",
        "!If sequence",
        "!Not sequence",
        "!Equals sequence",
        "!And sequence",
        "!Or sequence",
        "!Condition",
        "!Base64",
        "!Cidr sequence",
        "!ImportValue",
        "!Transform mapping",
    ])
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

pub fn kubernetes_workspace_configuration_schema() -> Value {
    json!({
        "type": "object",
        "description": "Workspace configuration passed to kubernetes-language-server. Use the kubernetes block for extension-owned settings and yaml for raw yaml-language-server settings that apply when Kubernetes mode owns the buffer.",
        "default": {},
        "additionalProperties": true,
        "properties": {
            "kubernetes": {
                "type": "object",
                "description": "Extension-owned settings that control Kubernetes schema defaults and how this extension injects schema associations into built-in YAML buffers.",
                "default": {
                    "includeDefaultSchemas": true,
                    "injectIntoYamlLanguageServer": true,
                    "schemaAssociations": {}
                },
                "additionalProperties": false,
                "properties": {
                    "includeDefaultSchemas": {
                        "type": "boolean",
                        "default": true,
                        "description": "Keep the extension's default Kubernetes, Kustomize, and Helm chart schema associations inside Kubernetes-mode buffers."
                    },
                    "injectIntoYamlLanguageServer": {
                        "type": "boolean",
                        "default": true,
                        "description": "Mirror the extension-owned schema associations into built-in yaml-language-server for plain YAML buffers that stay in YAML mode."
                    },
                    "schemaAssociations": {
                        "type": "object",
                        "default": {},
                        "description": "Additional schema-to-glob associations merged into yaml.schemas. Relative schema paths are resolved against the worktree root and ~/ paths are resolved against HOME.",
                        "additionalProperties": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        }
                    }
                }
            },
            "yaml": raw_passthrough_schema("Raw yaml-language-server workspace settings applied only to Kubernetes-mode buffers. These settings override the extension defaults and the curated kubernetes block."),
            "[yaml]": raw_passthrough_schema("Editor settings applied to Kubernetes-mode YAML buffers.")
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

pub fn helm_workspace_configuration_schema() -> Value {
    json!({
        "type": "object",
        "description": "Workspace configuration passed through to helm-language-server.",
        "default": {},
        "additionalProperties": true,
        "properties": {
            "helm-ls": raw_passthrough_schema("Raw helm-language-server settings. Configure helm-ls exactly as upstream documents it.")
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

    curated_settings
}

fn apply_curated_workspace_settings(
    configuration: &mut Value,
    curated_settings: &CuratedKubernetesSettings,
    worktree_root: Option<&str>,
    home_dir: Option<&str>,
) {
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
        return PathBuf::from(home_dir)
            .join(rest)
            .to_string_lossy()
            .into_owned();
    }

    if let Some(worktree_root) = worktree_root.filter(|_| schema_path.starts_with('.')) {
        return PathBuf::from(worktree_root)
            .join(schema_path.strip_prefix("./").unwrap_or(schema_path))
            .to_string_lossy()
            .into_owned();
    }

    schema_path.to_string()
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
    fn default_config_includes_custom_tags() {
        let configuration = default_workspace_configuration();
        let tags = configuration["yaml"]["customTags"]
            .as_array()
            .expect("customTags should be an array");

        assert!(!tags.is_empty(), "customTags should not be empty");
        assert!(
            tags.contains(&json!("!Ref")),
            "customTags should include !Ref",
        );
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
            schemas.contains_key("/home/user/project/schemas/custom.json"),
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
    fn raw_yaml_settings_override_curated_settings() {
        let configuration = kubernetes_workspace_configuration(
            Some(json!({
                "kubernetes": {
                    "schemaAssociations": {
                        "./schemas/custom.json": ["crds/*.yaml"]
                    }
                },
                "yaml": {
                    "schemas": {
                        "./schemas/custom.json": ["overrides/*.yaml"]
                    }
                }
            })),
            Some("/home/user/project"),
            None,
        );

        assert_eq!(
            configuration["yaml"]["schemas"]["/home/user/project/schemas/custom.json"],
            json!(["overrides/*.yaml"]),
        );
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
            schemas.contains_key("/home/user/project/../shared/schema.json"),
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
        assert!(schemas.contains_key("/absolute/path/schema.json"));
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
            schemas.contains_key("/home/user/schemas/custom.json"),
            "~/path should resolve to $HOME/path for curated schema associations",
        );
        assert!(
            schemas.contains_key("/home/user/schemas/raw.json"),
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
            schemas.contains_key("/home/user/project/schemas/custom.json"),
            "curated schema associations should be mirrored into built-in YAML",
        );
        assert!(
            !schemas.contains_key("/home/user/project/schemas/raw.json"),
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
            schemas.contains_key("/home/user/project/schemas/custom.json"),
            "curated schema associations should still be mirrored",
        );
    }

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
    }

    #[test]
    fn initialization_and_helm_schemas_are_permissive_objects() {
        let initialization_schema = kubernetes_initialization_options_schema();
        let helm_schema = helm_workspace_configuration_schema();

        assert_eq!(initialization_schema["type"], "object");
        assert_eq!(initialization_schema["additionalProperties"], true);
        assert_eq!(helm_schema["type"], "object");
        assert_eq!(helm_schema["properties"]["helm-ls"]["type"], "object");
        assert_eq!(
            helm_schema["properties"]["helm-ls"]["additionalProperties"],
            true,
        );
    }
}
