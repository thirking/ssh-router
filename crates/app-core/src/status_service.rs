use std::path::{Path, PathBuf};

use crate::cli_service::{files_match, install_cli_script};
use crate::error::{AppError, AppResult, CliError, ServiceError};
use crate::platform::{
    ElevationProvider, FileSystem, RegistryProvider, ResourceProvider, ServiceProvider,
};
use crate::CONFIG_PATH;

pub const CLI_DEPLOY_PATH: &str = r"C:\ProgramData\ssh\ssh-router-cli.exe";
const OPENSSH_REGISTRY_KEY: &str = r"SOFTWARE\OpenSSH";
const DEFAULT_SHELL_VALUE: &str = "DefaultShell";
const SSHD_SERVICE_NAME: &str = "sshd";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemStatus {
    pub cli_deployed: bool,
    pub cli_up_to_date: bool,
    pub cli_path: String,
    pub default_shell_set: bool,
    pub default_shell: String,
    pub config_exists: bool,
    pub sshd_running: bool,
    pub sshd_detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationMessage {
    pub message: String,
}

pub trait SystemService: Send + Sync {
    fn check_status(&self) -> AppResult<SystemStatus>;
    fn install_cli(&self) -> AppResult<OperationMessage>;
    fn set_default_shell(&self) -> AppResult<OperationMessage>;
    fn restart_sshd(&self) -> AppResult<OperationMessage>;
}

pub struct DesktopSystemService<P> {
    platform: P,
    cli_deploy_path: PathBuf,
    config_path: PathBuf,
}

impl<P> DesktopSystemService<P> {
    pub fn new(platform: P) -> Self {
        Self::with_paths(platform, CLI_DEPLOY_PATH, CONFIG_PATH)
    }

    pub fn with_paths(
        platform: P,
        cli_deploy_path: impl Into<PathBuf>,
        config_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            platform,
            cli_deploy_path: cli_deploy_path.into(),
            config_path: config_path.into(),
        }
    }
}

impl<P> SystemService for DesktopSystemService<P>
where
    P: FileSystem + RegistryProvider + ServiceProvider + ElevationProvider + ResourceProvider,
{
    fn check_status(&self) -> AppResult<SystemStatus> {
        let cli_deployed = self.platform.exists(&self.cli_deploy_path)?;
        let cli_up_to_date = if cli_deployed {
            let bundled_cli = self.platform.bundled_cli_path()?;
            files_match(&bundled_cli, &self.cli_deploy_path)?
        } else {
            false
        };
        let default_shell = self
            .platform
            .read_string(OPENSSH_REGISTRY_KEY, DEFAULT_SHELL_VALUE)?
            .unwrap_or_default();
        let default_shell_set = path_strings_equal(&default_shell, &self.cli_deploy_path);
        let config_exists = self.platform.exists(&self.config_path)?;
        let service_status = match self.platform.query(SSHD_SERVICE_NAME) {
            Ok(status) => status,
            Err(AppError::Service(ServiceError::NotFound { .. })) => crate::ServiceStatus {
                running: false,
                detail: "Not installed".to_string(),
            },
            Err(error) => return Err(error),
        };

        Ok(SystemStatus {
            cli_deployed,
            cli_up_to_date,
            cli_path: self.cli_deploy_path.to_string_lossy().to_string(),
            default_shell_set,
            default_shell,
            config_exists,
            sshd_running: service_status.running,
            sshd_detail: service_status.detail,
        })
    }

    fn install_cli(&self) -> AppResult<OperationMessage> {
        let source = self.platform.bundled_cli_path()?;
        if !self.platform.exists(&source)? {
            return Err(CliError::BundledCliMissing { path: source }.into());
        }
        let script = install_cli_script(&source, &self.cli_deploy_path)?;
        match self.platform.run_script(&script) {
            Ok(_) => {}
            Err(AppError::Permission(crate::error::PermissionError::Cancelled)) => {
                return Err(crate::error::PermissionError::Cancelled.into());
            }
            Err(error) => {
                return Err(CliError::Deploy {
                    message: error.to_string(),
                }
                .into());
            }
        }
        Ok(OperationMessage {
            message: "CLI 安装或更新完成".to_string(),
        })
    }

    fn set_default_shell(&self) -> AppResult<OperationMessage> {
        self.platform.write_string_elevated(
            OPENSSH_REGISTRY_KEY,
            DEFAULT_SHELL_VALUE,
            &self.cli_deploy_path.to_string_lossy(),
        )?;
        Ok(OperationMessage {
            message: "DefaultShell 设置完成".to_string(),
        })
    }

    fn restart_sshd(&self) -> AppResult<OperationMessage> {
        self.platform.restart_elevated(SSHD_SERVICE_NAME)?;
        Ok(OperationMessage {
            message: "sshd 重启完成".to_string(),
        })
    }
}

fn path_strings_equal(value: &str, expected: &Path) -> bool {
    value
        .trim()
        .eq_ignore_ascii_case(&expected.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    use super::*;
    use crate::error::{ConfigError, RegistryError};
    use crate::platform::{ServiceProvider, ServiceStatus};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct FakePlatform {
        existing_paths: Mutex<HashSet<PathBuf>>,
        registry: Mutex<HashMap<(String, String), String>>,
        service_status: Mutex<Option<ServiceStatus>>,
        bundled_cli: PathBuf,
        scripts: Mutex<Vec<String>>,
        registry_writes: Mutex<Vec<(String, String, String)>>,
        service_restarts: Mutex<Vec<String>>,
        registry_read_error: bool,
    }

    impl FakePlatform {
        fn new(bundled_cli: PathBuf) -> Self {
            Self {
                existing_paths: Mutex::new(HashSet::new()),
                registry: Mutex::new(HashMap::new()),
                service_status: Mutex::new(Some(ServiceStatus {
                    running: true,
                    detail: "Running".to_string(),
                })),
                bundled_cli,
                scripts: Mutex::new(Vec::new()),
                registry_writes: Mutex::new(Vec::new()),
                service_restarts: Mutex::new(Vec::new()),
                registry_read_error: false,
            }
        }
    }

    impl FileSystem for FakePlatform {
        fn exists(&self, path: &Path) -> AppResult<bool> {
            Ok(self.existing_paths.lock().expect("锁应可用").contains(path))
        }

        fn read(&self, path: &Path) -> AppResult<Vec<u8>> {
            fs::read(path).map_err(|source| {
                ConfigError::Read {
                    path: path.to_path_buf(),
                    source,
                }
                .into()
            })
        }

        fn write_atomic(&self, _path: &Path, _contents: &[u8]) -> AppResult<()> {
            Ok(())
        }

        fn remove_file(&self, _path: &Path) -> AppResult<()> {
            Ok(())
        }
    }

    impl RegistryProvider for FakePlatform {
        fn read_string(&self, key: &str, value: &str) -> AppResult<Option<String>> {
            if self.registry_read_error {
                return Err(RegistryError::Read {
                    key: key.to_string(),
                    value: value.to_string(),
                    source: std::io::Error::other("registry unavailable"),
                }
                .into());
            }
            Ok(self
                .registry
                .lock()
                .expect("锁应可用")
                .get(&(key.to_string(), value.to_string()))
                .cloned())
        }

        fn write_string_elevated(&self, key: &str, value: &str, data: &str) -> AppResult<()> {
            self.registry_writes.lock().expect("锁应可用").push((
                key.to_string(),
                value.to_string(),
                data.to_string(),
            ));
            Ok(())
        }
    }

    impl ServiceProvider for FakePlatform {
        fn query(&self, name: &str) -> AppResult<ServiceStatus> {
            self.service_status
                .lock()
                .expect("锁应可用")
                .clone()
                .ok_or_else(|| {
                    ServiceError::NotFound {
                        name: name.to_string(),
                    }
                    .into()
                })
        }

        fn restart_elevated(&self, name: &str) -> AppResult<()> {
            self.service_restarts
                .lock()
                .expect("锁应可用")
                .push(name.to_string());
            Ok(())
        }
    }

    impl ElevationProvider for FakePlatform {
        fn run_script(&self, script: &str) -> AppResult<String> {
            self.scripts
                .lock()
                .expect("锁应可用")
                .push(script.to_string());
            Ok("OK".to_string())
        }
    }

    impl ResourceProvider for FakePlatform {
        fn bundled_cli_path(&self) -> AppResult<PathBuf> {
            Ok(self.bundled_cli.clone())
        }

        fn application_version(&self) -> AppResult<String> {
            Ok("0.0.8".to_string())
        }
    }

    fn temp_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ssh-router-status-{name}-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn reports_healthy_system_status() {
        let directory = temp_directory("healthy");
        fs::create_dir_all(&directory).expect("测试目录应可创建");
        let bundled = directory.join("bundled.exe");
        let deployed = directory.join("deployed.exe");
        let config = directory.join("config.json");
        fs::write(&bundled, b"same cli").expect("测试文件应可写入");
        fs::write(&deployed, b"same cli").expect("测试文件应可写入");
        let platform = FakePlatform::new(bundled.clone());
        platform.existing_paths.lock().expect("锁应可用").extend([
            bundled,
            deployed.clone(),
            config.clone(),
        ]);
        platform.registry.lock().expect("锁应可用").insert(
            (
                OPENSSH_REGISTRY_KEY.to_string(),
                DEFAULT_SHELL_VALUE.to_string(),
            ),
            deployed.to_string_lossy().to_string(),
        );
        let service = DesktopSystemService::with_paths(platform, &deployed, &config);

        let status = service.check_status().expect("状态检查应成功");

        assert!(status.cli_deployed);
        assert!(status.cli_up_to_date);
        assert!(status.default_shell_set);
        assert!(status.config_exists);
        assert!(status.sshd_running);
        fs::remove_dir_all(directory).expect("测试目录应可清理");
    }

    #[test]
    fn reports_missing_service_without_failing_status() {
        let platform = FakePlatform::new(PathBuf::from("bundled.exe"));
        *platform.service_status.lock().expect("锁应可用") = None;
        let service = DesktopSystemService::with_paths(platform, "deployed.exe", "config.json");

        let status = service.check_status().expect("服务缺失不应中断状态检查");

        assert!(!status.sshd_running);
        assert_eq!(status.sshd_detail, "Not installed");
    }

    #[test]
    fn invokes_install_registry_and_restart_operations() {
        let directory = temp_directory("operations");
        fs::create_dir_all(&directory).expect("测试目录应可创建");
        let bundled = directory.join("bundled.exe");
        fs::write(&bundled, b"cli").expect("测试文件应可写入");
        let platform = FakePlatform::new(bundled.clone());
        platform
            .existing_paths
            .lock()
            .expect("锁应可用")
            .insert(bundled);
        let service = DesktopSystemService::with_paths(platform, "/deployed.exe", "/config.json");

        service.install_cli().expect("CLI 安装应成功");
        service.set_default_shell().expect("注册表设置应成功");
        service.restart_sshd().expect("服务重启应成功");

        assert_eq!(service.platform.scripts.lock().expect("锁应可用").len(), 1);
        assert_eq!(
            service
                .platform
                .registry_writes
                .lock()
                .expect("锁应可用")
                .len(),
            1
        );
        assert_eq!(
            service
                .platform
                .service_restarts
                .lock()
                .expect("锁应可用")
                .as_slice(),
            ["sshd"]
        );
        fs::remove_dir_all(directory).expect("测试目录应可清理");
    }

    #[test]
    fn reports_missing_bundled_cli_during_install() {
        let platform = FakePlatform::new(PathBuf::from("missing-bundled.exe"));
        let service = DesktopSystemService::with_paths(platform, "deployed.exe", "config.json");

        let error = service.install_cli().expect_err("安装包 CLI 缺失应失败");

        assert!(matches!(
            error,
            AppError::Cli(CliError::BundledCliMissing { .. })
        ));
    }

    #[test]
    fn propagates_registry_read_failure() {
        let platform = FakePlatform {
            registry_read_error: true,
            ..FakePlatform::new(PathBuf::from("bundled.exe"))
        };
        let service = DesktopSystemService::with_paths(platform, "deployed.exe", "config.json");

        let error = service.check_status().expect_err("注册表失败应传播");

        assert!(matches!(error, AppError::Registry(_)));
    }
}
