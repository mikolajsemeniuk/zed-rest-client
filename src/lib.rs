use std::fs;
use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

const LSP_REPO: &str = "mikolajsemeniuk/zed-rest-client-lsp";
const BINARY_NAME: &str = "zed-rest-client-lsp";

struct RestClientExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for RestClientExtension {
    fn new() -> Self {
        RestClientExtension {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<Command> {
        let binary_path = self.language_server_binary_path(language_server_id)?;
        Ok(Command {
            command: binary_path,
            args: vec![],
            env: vec![],
        })
    }
}

impl RestClientExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            LSP_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = match (platform, arch) {
            (zed::Os::Mac, zed::Architecture::Aarch64) => {
                "zed-rest-client-lsp-aarch64-apple-darwin.tar.gz"
            }
            (zed::Os::Mac, zed::Architecture::X8664) => {
                "zed-rest-client-lsp-x86_64-apple-darwin.tar.gz"
            }
            (zed::Os::Linux, zed::Architecture::X8664) => {
                "zed-rest-client-lsp-x86_64-unknown-linux-gnu.tar.gz"
            }
            (zed::Os::Windows, zed::Architecture::X8664) => {
                "zed-rest-client-lsp-x86_64-pc-windows-msvc.zip"
            }
            (os, arch) => {
                return Err(format!("unsupported platform: {os:?} {arch:?}"));
            }
        };

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| {
                format!(
                    "asset '{asset_name}' not found in release {}",
                    release.version
                )
            })?;

        let version_dir = format!("zed-rest-client-lsp-{}", release.version);
        let binary_path_in_dir = if matches!(platform, zed::Os::Windows) {
            format!("{version_dir}/{BINARY_NAME}.exe")
        } else {
            format!("{version_dir}/{BINARY_NAME}")
        };

        if !fs::metadata(&binary_path_in_dir).map_or(false, |m| m.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let file_type = if asset_name.ends_with(".zip") {
                zed::DownloadedFileType::Zip
            } else {
                zed::DownloadedFileType::GzipTar
            };

            zed::download_file(&asset.download_url, &version_dir, file_type)
                .map_err(|e| format!("failed to download {asset_name}: {e}"))?;

            if !matches!(platform, zed::Os::Windows) {
                zed::make_file_executable(&binary_path_in_dir)?;
            }

            if let Ok(entries) = fs::read_dir(".") {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("zed-rest-client-lsp-") && name_str != version_dir {
                        let _ = fs::remove_dir_all(entry.path());
                    }
                }
            }
        }

        self.cached_binary_path = Some(binary_path_in_dir.clone());
        Ok(binary_path_in_dir)
    }
}

zed::register_extension!(RestClientExtension);
