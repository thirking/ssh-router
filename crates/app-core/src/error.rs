use std::io;
use std::path::PathBuf;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Permission(#[from] PermissionError),
    #[error(transparent)]
    Cli(#[from] CliError),
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error(transparent)]
    Update(#[from] UpdateError),
    #[error(transparent)]
    Platform(#[from] PlatformError),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("配置文件不存在: {path}")]
    NotFound { path: PathBuf },
    #[error("读取配置失败: {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("配置格式无效: {message}")]
    InvalidFormat { message: String },
    #[error("配置校验失败: {message}")]
    Validation { message: String },
    #[error("保存配置失败: {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PermissionError {
    #[error("用户取消了管理员权限操作")]
    Cancelled,
    #[error("操作需要管理员权限: {operation}")]
    Required { operation: String },
    #[error("管理员权限操作失败: {message}")]
    ElevationFailed { message: String },
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("安装包内缺少 CLI: {path}")]
    BundledCliMissing { path: PathBuf },
    #[error("读取 CLI 失败: {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("部署 CLI 失败: {message}")]
    Deploy { message: String },
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("读取注册表失败: {key}\\{value}")]
    Read {
        key: String,
        value: String,
        #[source]
        source: io::Error,
    },
    #[error("写入注册表失败: {key}\\{value}: {message}")]
    Write {
        key: String,
        value: String,
        message: String,
    },
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Windows 服务不存在: {name}")]
    NotFound { name: String },
    #[error("查询 Windows 服务失败: {name}: {message}")]
    Query { name: String, message: String },
    #[error("重启 Windows 服务失败: {name}: {message}")]
    Restart { name: String, message: String },
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("更新元数据无效: {message}")]
    InvalidManifest { message: String },
    #[error("更新签名验证失败: {message}")]
    InvalidSignature { message: String },
    #[error("更新下载失败: {message}")]
    Download { message: String },
    #[error("启动更新安装程序失败: {message}")]
    Install { message: String },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlatformError {
    #[error("当前平台不支持此操作: {operation}")]
    Unsupported { operation: String },
    #[error("平台操作失败: {message}")]
    Operation { message: String },
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn converts_specialized_error_to_app_error() {
        let error = PermissionError::Required {
            operation: "重启 sshd".to_string(),
        };

        let app_error = AppError::from(error);

        assert_eq!(app_error.to_string(), "操作需要管理员权限: 重启 sshd");
        assert!(matches!(app_error, AppError::Permission(_)));
    }

    #[test]
    fn preserves_io_error_as_source() {
        let error = ConfigError::Read {
            path: PathBuf::from("ssh-router.json"),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
        };

        let source = error.source().expect("读取错误应保留原始错误");

        assert_eq!(source.to_string(), "denied");
    }

    #[test]
    fn distinguishes_user_cancel_from_failure() {
        assert_eq!(
            PermissionError::Cancelled.to_string(),
            "用户取消了管理员权限操作"
        );
        assert_ne!(
            PermissionError::Cancelled,
            PermissionError::ElevationFailed {
                message: "拒绝访问".to_string(),
            }
        );
    }
}
