use std::{env, fs};

use zed_extension_api::{self as zed, Os, settings::CommandSettings};

/// Removes version directories that don't match the current version.
pub fn remove_outdated_versions(prefix: &str, current_dir: &str) -> Result<(), String> {
    let entries =
        fs::read_dir(".").map_err(|e| format!("failed to read extension directory: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with(prefix)
            && name != current_dir
            && entry.metadata().is_ok_and(|m| m.is_dir())
        {
            let _ = fs::remove_dir_all(name);
        }
    }
    Ok(())
}

pub fn merged_env(
    mut base_env: Vec<(String, String)>,
    binary_settings: Option<&CommandSettings>,
) -> Vec<(String, String)> {
    if let Some(overrides) = binary_settings.and_then(|settings| settings.env.clone()) {
        base_env.extend(overrides);
    }
    base_env
}

pub fn expand_tilde(value: &str) -> String {
    let (os, _) = zed::current_platform();
    let home_var = match os {
        Os::Windows => "USERPROFILE",
        _ => "HOME",
    };
    expand_tilde_with_home(value, env::var(home_var).ok().as_deref())
}

pub fn expand_tilde_with_home(value: &str, home_dir: Option<&str>) -> String {
    match (value.strip_prefix("~/"), home_dir) {
        (Some(rest), Some(home)) => format!("{home}/{rest}"),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn command_settings(
        path: Option<&str>,
        arguments: Option<Vec<&str>>,
        env: Option<Vec<(&str, &str)>>,
    ) -> CommandSettings {
        CommandSettings {
            path: path.map(ToOwned::to_owned),
            arguments: arguments.map(|arguments| {
                arguments
                    .into_iter()
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            }),
            env: env.map(|env| {
                env.into_iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect::<HashMap<_, _>>()
            }),
        }
    }

    #[test]
    fn merged_env_appends_user_env_overrides() {
        let env = merged_env(
            vec![("PATH".to_string(), "/usr/bin".to_string())],
            Some(&command_settings(
                None,
                None,
                Some(vec![("KUBECONFIG", "/tmp/kubeconfig")]),
            )),
        );

        assert_eq!(
            env,
            vec![
                ("PATH".to_string(), "/usr/bin".to_string()),
                ("KUBECONFIG".to_string(), "/tmp/kubeconfig".to_string()),
            ],
        );
    }

    #[test]
    fn expand_tilde_with_home_expands_tilde_prefix() {
        assert_eq!(
            expand_tilde_with_home("~/.kube/config", Some("/home/user")),
            "/home/user/.kube/config",
        );
    }

    #[test]
    fn expand_tilde_with_home_preserves_absolute_paths() {
        assert_eq!(
            expand_tilde_with_home("/etc/config", Some("/home/user")),
            "/etc/config",
        );
    }

    #[test]
    fn expand_tilde_with_home_preserves_tilde_without_home() {
        assert_eq!(
            expand_tilde_with_home("~/.kube/config", None),
            "~/.kube/config",
        );
    }

    #[test]
    fn expand_tilde_with_home_does_not_expand_other_user_tilde() {
        assert_eq!(
            expand_tilde_with_home("~other/.kube/config", Some("/home/user")),
            "~other/.kube/config",
        );
    }
}
