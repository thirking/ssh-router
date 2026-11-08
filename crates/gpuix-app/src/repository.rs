#[cfg(not(windows))]
use std::path::PathBuf;

#[cfg(windows)]
use ssh_router_app_core::CONFIG_PATH;
use ssh_router_app_core::{ConfigService, UpdateService, WINDOWS_X86_64_PLATFORM};

pub(super) fn backend(
    config_path: Option<String>,
) -> Result<super::Backend, Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let config_path = config_path.unwrap_or_else(|| CONFIG_PATH.into());
    #[cfg(not(windows))]
    let config_path = config_path.unwrap_or_else(|| {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".config/ssh-router/ssh-router.json")
            .to_string_lossy()
            .into_owned()
    });

    #[cfg(windows)]
    let platform = ssh_router_app_core::platform::windows::WindowsPlatform::new()?;
    #[cfg(not(windows))]
    let platform = LocalPlatform;

    #[cfg(windows)]
    let system = Some(
        Box::new(ssh_router_app_core::DesktopSystemService::new(platform))
            as Box<dyn ssh_router_app_core::SystemService>,
    );
    #[cfg(not(windows))]
    let system = None;

    Ok(super::Backend {
        repository: Box::new(ConfigService::with_paths(
            platform,
            &config_path,
            std::env::temp_dir(),
        )),
        system,
        config_path,
        updater: UpdateService::new(env!("CARGO_PKG_VERSION"), WINDOWS_X86_64_PLATFORM),
        candidate: None,
    })
}

#[cfg(not(windows))]
struct LocalPlatform;

#[cfg(not(windows))]
impl ssh_router_app_core::FileSystem for LocalPlatform {
    fn exists(&self, path: &std::path::Path) -> ssh_router_app_core::AppResult<bool> {
        path.try_exists().map_err(|source| {
            ssh_router_app_core::ConfigError::Read {
                path: path.into(),
                source,
            }
            .into()
        })
    }

    fn read(&self, path: &std::path::Path) -> ssh_router_app_core::AppResult<Vec<u8>> {
        std::fs::read(path).map_err(|source| {
            ssh_router_app_core::ConfigError::Read {
                path: path.into(),
                source,
            }
            .into()
        })
    }

    fn write_atomic(
        &self,
        path: &std::path::Path,
        contents: &[u8],
    ) -> ssh_router_app_core::AppResult<()> {
        use std::io::Write;
        let write = || -> std::io::Result<()> {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."));
            std::fs::create_dir_all(parent)?;
            let mut staged = tempfile::NamedTempFile::new_in(parent)?;
            staged.write_all(contents)?;
            staged.as_file().sync_all()?;
            staged.persist(path).map_err(|error| error.error)?;
            Ok(())
        };
        write().map_err(|source| {
            ssh_router_app_core::ConfigError::Write {
                path: path.into(),
                source,
            }
            .into()
        })
    }

    fn remove_file(&self, path: &std::path::Path) -> ssh_router_app_core::AppResult<()> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(ssh_router_app_core::ConfigError::Write {
                path: path.into(),
                source,
            }
            .into()),
        }
    }
}

#[cfg(not(windows))]
impl ssh_router_app_core::ElevationProvider for LocalPlatform {
    fn run_script(&self, _: &str) -> ssh_router_app_core::AppResult<String> {
        Err(ssh_router_app_core::PlatformError::Unsupported {
            operation: "Windows 提权".into(),
        }
        .into())
    }
}
