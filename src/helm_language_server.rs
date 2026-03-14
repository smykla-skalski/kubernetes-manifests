use std::fs;

use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, GithubReleaseOptions, LanguageServerId, Os,
    Result,
    settings::{CommandSettings, LspSettings},
};

use crate::util;

pub const SERVER_NAME: &str = "helm-language-server";
const BINARY_NAME: &str = "helm_ls";
const GITHUB_REPO: &str = "mrjosh/helm-ls";

#[derive(Default)]
pub struct HelmLanguageServer {
    cached_binary_path: Option<String>,
}

impl HelmLanguageServer {
    pub fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
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
            return Ok(zed::Command {
                command: path,
                args,
                env,
            });
        }

        if let Some(path) = worktree.which(BINARY_NAME) {
            return Ok(zed::Command {
                command: path,
                args,
                env,
            });
        }

        let binary_path = self.download_binary()?;
        Ok(zed::Command {
            command: binary_path,
            args,
            env,
        })
    }

    fn download_binary(&mut self) -> Result<String> {
        if let Some(path) = self.cached_binary_path.as_ref()
            && fs::metadata(path).is_ok_and(|m| m.is_file())
        {
            return Ok(path.clone());
        }

        let (os, arch) = zed::current_platform();

        let platform_binary = match os {
            Os::Windows => format!("{BINARY_NAME}.exe"),
            _ => BINARY_NAME.to_string(),
        };
        if let Some(path) = util::find_installed_binary("helm-ls-", &platform_binary) {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        let asset_name = platform_asset_name(os, arch);

        let release = zed::latest_github_release(
            GITHUB_REPO,
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )
        .map_err(|e| format!("failed to fetch latest helm-ls release: {e}"))?;

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no asset matching {asset_name} in release"))?;

        let version_dir = format!("helm-ls-{}", release.version);
        fs::create_dir_all(&version_dir).map_err(|e| format!("failed to create directory: {e}"))?;

        let binary_path = platform_binary_path(&version_dir, os);

        zed::download_file(
            &asset.download_url,
            &binary_path,
            DownloadedFileType::Uncompressed,
        )
        .map_err(|e| format!("failed to download helm-ls binary: {e}"))?;
        zed::make_file_executable(&binary_path)
            .map_err(|e| format!("failed to make helm-ls binary executable: {e}"))?;

        util::remove_outdated_versions("helm-ls-", &version_dir).ok();

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

fn platform_binary_path(version_dir: &str, os: Os) -> String {
    let ext = match os {
        Os::Windows => ".exe",
        _ => "",
    };
    format!("{version_dir}/{BINARY_NAME}{ext}")
}

fn platform_asset_name(os: Os, arch: Architecture) -> String {
    let os_str = match os {
        Os::Mac => "darwin",
        Os::Linux => "linux",
        Os::Windows => "windows",
    };
    let arch_str = match arch {
        Architecture::Aarch64 => "arm64",
        Architecture::X86 | Architecture::X8664 => "amd64",
    };
    let ext = match os {
        Os::Windows => ".exe",
        _ => "",
    };
    format!("helm_ls_{os_str}_{arch_str}{ext}")
}

fn default_server_arguments() -> Vec<String> {
    vec!["serve".to_string()]
}

fn server_arguments(binary_settings: Option<&CommandSettings>) -> Vec<String> {
    binary_settings
        .and_then(|settings| settings.arguments.clone())
        .unwrap_or_else(default_server_arguments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_asset_name_macos_arm64() {
        let name = platform_asset_name(Os::Mac, Architecture::Aarch64);
        assert_eq!(name, "helm_ls_darwin_arm64");
    }

    #[test]
    fn platform_asset_name_linux_x86_64() {
        let name = platform_asset_name(Os::Linux, Architecture::X8664);
        assert_eq!(name, "helm_ls_linux_amd64");
    }

    #[test]
    fn platform_asset_name_windows_x86_64() {
        let name = platform_asset_name(Os::Windows, Architecture::X8664);
        assert_eq!(name, "helm_ls_windows_amd64.exe");
    }

    #[test]
    fn platform_binary_path_appends_exe_on_windows() {
        let path = platform_binary_path("helm-ls-v1.0.0", Os::Windows);
        assert_eq!(path, "helm-ls-v1.0.0/helm_ls.exe");
    }

    #[test]
    fn platform_binary_path_omits_exe_on_non_windows() {
        let path = platform_binary_path("helm-ls-v1.0.0", Os::Mac);
        assert_eq!(path, "helm-ls-v1.0.0/helm_ls");
    }

    #[test]
    fn default_args_use_serve_subcommand() {
        let args = default_server_arguments();
        assert_eq!(args, vec!["serve"]);
    }
}
