use std::path::{Path, PathBuf};

use crate::error::AppResult;

#[cfg(windows)]
pub mod windows;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceStatus {
    pub running: bool,
    pub detail: String,
}

pub trait FileSystem: Send + Sync {
    fn exists(&self, path: &Path) -> bool;
    fn read(&self, path: &Path) -> AppResult<Vec<u8>>;
    fn write_atomic(&self, path: &Path, contents: &[u8]) -> AppResult<()>;
}

pub trait RegistryProvider: Send + Sync {
    fn read_string(&self, key: &str, value: &str) -> AppResult<Option<String>>;
    fn write_string_elevated(&self, key: &str, value: &str, data: &str) -> AppResult<()>;
}

pub trait ServiceProvider: Send + Sync {
    fn query(&self, name: &str) -> AppResult<ServiceStatus>;
    fn restart_elevated(&self, name: &str) -> AppResult<()>;
}

pub trait ElevationProvider: Send + Sync {
    fn run_script(&self, script: &str) -> AppResult<String>;
}

pub trait ResourceProvider: Send + Sync {
    fn bundled_cli_path(&self) -> AppResult<PathBuf>;
    fn application_version(&self) -> AppResult<String>;
}

pub trait PlatformServices:
    FileSystem + RegistryProvider + ServiceProvider + ElevationProvider + ResourceProvider
{
}

impl<T> PlatformServices for T where
    T: FileSystem + RegistryProvider + ServiceProvider + ElevationProvider + ResourceProvider
{
}

#[cfg(not(windows))]
pub fn ensure_supported(operation: impl Into<String>) -> AppResult<()> {
    Err(crate::error::PlatformError::Unsupported {
        operation: operation.into(),
    }
    .into())
}

#[cfg(windows)]
pub fn ensure_supported(_operation: impl Into<String>) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(not(windows))]
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn reports_unsupported_platform() {
        let error = ensure_supported("读取 Windows 注册表").expect_err("非 Windows 应拒绝操作");

        assert_eq!(
            error.to_string(),
            "当前平台不支持此操作: 读取 Windows 注册表"
        );
    }
}
