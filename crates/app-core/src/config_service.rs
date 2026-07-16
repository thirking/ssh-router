use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ssh_router_config::{normalize_legacy_config, validate_config, Config};

use crate::error::{AppError, AppResult, ConfigError};
use crate::platform::{ElevationProvider, FileSystem};

pub const CONFIG_PATH: &str = r"C:\ProgramData\ssh\ssh-router.json";

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub trait ConfigRepository: Send + Sync {
    fn load(&self) -> AppResult<Config>;
    fn save(&self, config: &Config) -> AppResult<()>;
    fn create_default(&self) -> AppResult<Config>;
}

pub struct ConfigService<P> {
    platform: P,
    config_path: PathBuf,
    temp_directory: PathBuf,
}

impl<P> ConfigService<P> {
    pub fn new(platform: P) -> Self {
        Self::with_paths(platform, CONFIG_PATH, std::env::temp_dir())
    }

    pub fn with_paths(
        platform: P,
        config_path: impl Into<PathBuf>,
        temp_directory: impl Into<PathBuf>,
    ) -> Self {
        Self {
            platform,
            config_path: config_path.into(),
            temp_directory: temp_directory.into(),
        }
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

impl<P> ConfigRepository for ConfigService<P>
where
    P: FileSystem + ElevationProvider,
{
    fn load(&self) -> AppResult<Config> {
        if !self.platform.exists(&self.config_path)? {
            return Err(ConfigError::NotFound {
                path: self.config_path.clone(),
            }
            .into());
        }

        let contents = self.platform.read(&self.config_path)?;
        let mut config: Config =
            serde_json::from_slice(&contents).map_err(|error| ConfigError::InvalidFormat {
                message: error.to_string(),
            })?;
        normalize_legacy_config(&mut config);
        validate_config(&config).map_err(|error| ConfigError::Validation {
            message: error.to_string(),
        })?;
        Ok(config)
    }

    fn save(&self, config: &Config) -> AppResult<()> {
        validate_config(config).map_err(|error| ConfigError::Validation {
            message: error.to_string(),
        })?;
        let contents =
            serde_json::to_vec_pretty(config).map_err(|error| ConfigError::InvalidFormat {
                message: format!("序列化配置失败: {error}"),
            })?;

        match self.platform.write_atomic(&self.config_path, &contents) {
            Ok(()) => Ok(()),
            Err(AppError::Config(ConfigError::Write { source, .. }))
                if source.kind() == std::io::ErrorKind::PermissionDenied =>
            {
                self.save_elevated(&contents)
            }
            Err(error) => Err(error),
        }
    }

    fn create_default(&self) -> AppResult<Config> {
        let config = Config::default_config();
        self.save(&config)?;
        Ok(config)
    }
}

impl<P> ConfigService<P>
where
    P: FileSystem + ElevationProvider,
{
    fn save_elevated(&self, contents: &[u8]) -> AppResult<()> {
        let temporary_path = self.unique_temp_path();
        self.platform.write_atomic(&temporary_path, contents)?;

        let script = elevated_copy_script(&temporary_path, &self.config_path)?;
        let elevated_result = self.platform.run_script(&script).map(|_| ());
        let cleanup_result = self.platform.remove_file(&temporary_path);

        match (elevated_result, cleanup_result) {
            (Err(operation_error), Err(cleanup_error)) => Err(ConfigError::CleanupAfterFailure {
                path: temporary_path,
                operation_error: operation_error.to_string(),
                cleanup_error: cleanup_error.to_string(),
            }
            .into()),
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Err(error)) => Err(error),
            (Ok(()), Ok(())) => Ok(()),
        }
    }

    fn unique_temp_path(&self) -> PathBuf {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        self.temp_directory.join(format!(
            "ssh-router-config-{}-{sequence}.json",
            std::process::id()
        ))
    }
}

fn elevated_copy_script(source: &Path, destination: &Path) -> AppResult<String> {
    let destination_directory = destination
        .parent()
        .ok_or_else(|| ConfigError::Validation {
            message: format!("配置目标路径缺少父目录: {}", destination.display()),
        })?;
    let source = powershell_literal(source);
    let destination = powershell_literal(destination);
    let destination_directory = powershell_literal(destination_directory);

    Ok(format!(
        r#"$src = '{source}'
$dst = '{destination}'
$dstDir = '{destination_directory}'
if (-not (Test-Path -LiteralPath $dstDir)) {{
    New-Item -ItemType Directory -Path $dstDir -Force | Out-Null
}}
$staged = Join-Path $dstDir ('.ssh-router-' + [Guid]::NewGuid().ToString('N') + '.json.tmp')
try {{
    Copy-Item -LiteralPath $src -Destination $staged -Force
    if (Test-Path -LiteralPath $dst) {{
        [System.IO.File]::Replace($staged, $dst, $null, $true)
    }} else {{
        [System.IO.File]::Move($staged, $dst)
    }}
    & icacls $dst /reset | Out-Null
    if ($LASTEXITCODE -ne 0) {{ throw "icacls reset failed: $LASTEXITCODE" }}
    & icacls $dst /grant 'SYSTEM:(R)' 'Administrators:(F)' 'Users:(R)' | Out-Null
    if ($LASTEXITCODE -ne 0) {{ throw "icacls grant failed: $LASTEXITCODE" }}
}} finally {{
    if (Test-Path -LiteralPath $staged) {{ Remove-Item -LiteralPath $staged -Force }}
}}
"#
    ))
}

fn powershell_literal(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io;
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakePlatform {
        files: Mutex<HashMap<PathBuf, Vec<u8>>>,
        elevated_scripts: Mutex<Vec<String>>,
        deny_direct_write: bool,
        fail_elevation: bool,
        fail_cleanup: bool,
    }

    impl FileSystem for FakePlatform {
        fn exists(&self, path: &Path) -> AppResult<bool> {
            Ok(self.files.lock().expect("锁应可用").contains_key(path))
        }

        fn read(&self, path: &Path) -> AppResult<Vec<u8>> {
            self.files
                .lock()
                .expect("锁应可用")
                .get(path)
                .cloned()
                .ok_or_else(|| {
                    ConfigError::NotFound {
                        path: path.to_path_buf(),
                    }
                    .into()
                })
        }

        fn write_atomic(&self, path: &Path, contents: &[u8]) -> AppResult<()> {
            if self.deny_direct_write && path == Path::new("/protected/config.json") {
                return Err(ConfigError::Write {
                    path: path.to_path_buf(),
                    source: io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
                }
                .into());
            }
            self.files
                .lock()
                .expect("锁应可用")
                .insert(path.to_path_buf(), contents.to_vec());
            Ok(())
        }

        fn remove_file(&self, path: &Path) -> AppResult<()> {
            if self.fail_cleanup {
                return Err(ConfigError::Write {
                    path: path.to_path_buf(),
                    source: io::Error::other("cleanup failed"),
                }
                .into());
            }
            self.files.lock().expect("锁应可用").remove(path);
            Ok(())
        }
    }

    impl ElevationProvider for FakePlatform {
        fn run_script(&self, script: &str) -> AppResult<String> {
            self.elevated_scripts
                .lock()
                .expect("锁应可用")
                .push(script.to_string());
            if self.fail_elevation {
                return Err(crate::error::PermissionError::ElevationFailed {
                    message: "elevation failed".to_string(),
                }
                .into());
            }
            Ok("ok".to_string())
        }
    }

    #[test]
    fn loads_and_validates_config() {
        let platform = FakePlatform::default();
        let config = Config::default_config();
        platform.files.lock().expect("锁应可用").insert(
            PathBuf::from("config.json"),
            serde_json::to_vec(&config).expect("默认配置应可序列化"),
        );
        let repository = ConfigService::with_paths(platform, "config.json", "temp");

        assert_eq!(repository.load().expect("配置应可加载"), config);
    }

    #[test]
    fn reports_missing_config() {
        let repository = ConfigService::with_paths(FakePlatform::default(), "missing.json", "temp");

        let error = repository.load().expect_err("缺失配置应失败");

        assert!(matches!(
            error,
            AppError::Config(ConfigError::NotFound { .. })
        ));
    }

    #[test]
    fn reports_invalid_json() {
        let platform = FakePlatform::default();
        platform
            .files
            .lock()
            .expect("锁应可用")
            .insert(PathBuf::from("config.json"), b"{invalid".to_vec());
        let repository = ConfigService::with_paths(platform, "config.json", "temp");

        let error = repository.load().expect_err("无效 JSON 应失败");

        assert!(matches!(
            error,
            AppError::Config(ConfigError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn rejects_invalid_loaded_config() {
        let platform = FakePlatform::default();
        let mut config = Config::default_config();
        config.routes[1].port = config.routes[0].port;
        platform.files.lock().expect("锁应可用").insert(
            PathBuf::from("config.json"),
            serde_json::to_vec(&config).expect("测试配置应可序列化"),
        );
        let repository = ConfigService::with_paths(platform, "config.json", "temp");

        let error = repository.load().expect_err("无效配置应失败");

        assert!(matches!(
            error,
            AppError::Config(ConfigError::Validation { .. })
        ));
    }

    #[test]
    fn normalizes_loaded_legacy_config() {
        let platform = FakePlatform::default();
        let legacy_json = br#"{
            "routes": [{
                "port": 22,
                "name": "PowerShell",
                "shell": "pwsh.exe",
                "interactiveTemplate": "pwsh.exe -l",
                "commandTemplate": "pwsh.exe -File {tmpfile}",
                "tmpFileExt": ".ps1"
            }],
            "sftpCommand": "sftp-server.exe"
        }"#;
        platform
            .files
            .lock()
            .expect("锁应可用")
            .insert(PathBuf::from("config.json"), legacy_json.to_vec());
        let repository = ConfigService::with_paths(platform, "config.json", "temp");

        let loaded = repository.load().expect("旧配置应可加载");

        assert!(loaded.routes[0].default);
        assert_eq!(
            loaded.routes.iter().filter(|route| route.default).count(),
            1
        );
    }

    #[test]
    fn rejects_invalid_config_before_writing() {
        let platform = FakePlatform::default();
        let repository = ConfigService::with_paths(platform, "config.json", "temp");
        let mut config = Config::default_config();
        config.routes[1].port = config.routes[0].port;

        let error = repository.save(&config).expect_err("重复端口应失败");

        assert!(matches!(
            error,
            AppError::Config(ConfigError::Validation { .. })
        ));
    }

    #[test]
    fn saves_directly_when_destination_is_writable() {
        let platform = FakePlatform::default();
        let repository = ConfigService::with_paths(platform, "config.json", "temp");
        let config = Config::default_config();

        repository.save(&config).expect("配置应可保存");

        assert!(repository
            .platform
            .files
            .lock()
            .expect("锁应可用")
            .contains_key(Path::new("config.json")));
        assert!(repository
            .platform
            .elevated_scripts
            .lock()
            .expect("锁应可用")
            .is_empty());
    }

    #[test]
    fn falls_back_to_elevated_copy_on_permission_denied() {
        let platform = FakePlatform {
            deny_direct_write: true,
            ..Default::default()
        };
        let repository = ConfigService::with_paths(platform, "/protected/config.json", "/temp");

        repository
            .save(&Config::default_config())
            .expect("提权保存应成功");

        let scripts = repository
            .platform
            .elevated_scripts
            .lock()
            .expect("锁应可用");
        assert_eq!(scripts.len(), 1);
        assert!(scripts[0].contains("Copy-Item -LiteralPath"));
        assert!(scripts[0].contains("/protected/config.json"));
        assert!(repository
            .platform
            .files
            .lock()
            .expect("锁应可用")
            .is_empty());
    }

    #[test]
    fn cleans_temp_file_when_elevation_fails() {
        let platform = FakePlatform {
            deny_direct_write: true,
            fail_elevation: true,
            ..Default::default()
        };
        let repository = ConfigService::with_paths(platform, "/protected/config.json", "/temp");

        let error = repository
            .save(&Config::default_config())
            .expect_err("提权失败应返回错误");

        assert!(matches!(error, AppError::Permission(_)));
        assert!(repository
            .platform
            .files
            .lock()
            .expect("锁应可用")
            .is_empty());
    }

    #[test]
    fn reports_operation_and_cleanup_errors_together() {
        let platform = FakePlatform {
            deny_direct_write: true,
            fail_elevation: true,
            fail_cleanup: true,
            ..Default::default()
        };
        let repository = ConfigService::with_paths(platform, "/protected/config.json", "/temp");

        let error = repository
            .save(&Config::default_config())
            .expect_err("双重失败应返回错误");

        assert!(matches!(
            error,
            AppError::Config(ConfigError::CleanupAfterFailure { .. })
        ));
        let message = error.to_string();
        assert!(message.contains("elevation failed"));
        assert!(message.contains("cleanup failed"));
    }

    #[test]
    fn creates_and_persists_default_config() {
        let repository = ConfigService::with_paths(FakePlatform::default(), "config.json", "temp");

        let config = repository.create_default().expect("默认配置应创建成功");

        assert_eq!(config, Config::default_config());
        assert!(repository
            .platform
            .files
            .lock()
            .expect("锁应可用")
            .contains_key(Path::new("config.json")));
    }
}
