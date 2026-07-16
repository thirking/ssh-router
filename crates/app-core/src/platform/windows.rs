use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows::Win32::System::Services::{
    CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_MANAGER_CONNECT,
    SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATUS,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

use crate::error::{AppResult, ConfigError, PermissionError, RegistryError, ServiceError};
use crate::platform::{
    ElevationProvider, FileSystem, RegistryProvider, ResourceProvider, ServiceProvider,
    ServiceStatus,
};

const ELEVATION_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(250);
static UNIQUE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsPlatform;

impl WindowsPlatform {
    pub fn new() -> AppResult<Self> {
        super::ensure_supported("初始化 Windows 平台服务")?;
        Ok(Self)
    }
}

impl FileSystem for WindowsPlatform {
    fn exists(&self, path: &Path) -> AppResult<bool> {
        match path.try_exists() {
            Ok(exists) => Ok(exists),
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }
            .into()),
        }
    }

    fn read(&self, path: &Path) -> AppResult<Vec<u8>> {
        fs::read(path).map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                ConfigError::NotFound {
                    path: path.to_path_buf(),
                }
                .into()
            } else {
                ConfigError::Read {
                    path: path.to_path_buf(),
                    source,
                }
                .into()
            }
        })
    }

    fn write_atomic(&self, path: &Path, contents: &[u8]) -> AppResult<()> {
        let parent = path.parent().ok_or_else(|| ConfigError::Validation {
            message: format!("目标路径缺少父目录: {}", path.display()),
        })?;
        fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })?;

        let temporary_path = unique_sibling_path(path);
        let write_result = write_new_file(&temporary_path, contents);
        if let Err(source) = write_result {
            return Err(ConfigError::Write {
                path: temporary_path,
                source,
            }
            .into());
        }

        let move_result = move_replace(&temporary_path, path);
        if let Err(source) = move_result {
            let cleanup_error = remove_file_if_exists(&temporary_path).err();
            return match cleanup_error {
                Some(cleanup_error) => Err(ConfigError::CleanupAfterFailure {
                    path: temporary_path,
                    operation_error: source.to_string(),
                    cleanup_error: cleanup_error.to_string(),
                }
                .into()),
                None => Err(ConfigError::Write {
                    path: path.to_path_buf(),
                    source,
                }
                .into()),
            };
        }
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> AppResult<()> {
        remove_file_if_exists(path).map_err(|source| {
            ConfigError::Write {
                path: path.to_path_buf(),
                source,
            }
            .into()
        })
    }
}

impl ElevationProvider for WindowsPlatform {
    fn run_script(&self, script: &str) -> AppResult<String> {
        let id = unique_id();
        let temp_directory = std::env::temp_dir();
        let script_path = temp_directory.join(format!("ssh-router-action-{id}.ps1"));
        let wrapper_path = temp_directory.join(format!("ssh-router-wrapper-{id}.ps1"));
        let result_path = temp_directory.join(format!("ssh-router-result-{id}.json"));

        write_utf8_bom(&script_path, script).map_err(|source| ConfigError::Write {
            path: script_path.clone(),
            source,
        })?;
        let wrapper = elevation_wrapper(&script_path, &result_path);
        if let Err(source) = write_utf8_bom(&wrapper_path, &wrapper) {
            return Err(combine_with_cleanup(
                ConfigError::Write {
                    path: wrapper_path.clone(),
                    source,
                }
                .into(),
                &[&script_path],
            ));
        }

        if let Err(error) = shell_execute_elevated(&wrapper_path) {
            return Err(combine_with_cleanup(error, &[&script_path, &wrapper_path]));
        }

        let wait_result = wait_for_result(&result_path);
        let result = match wait_result {
            Ok(()) => read_elevation_result(&result_path),
            Err(error) => Err(error),
        };
        let cleanup = cleanup_paths(&[&script_path, &wrapper_path, &result_path]);

        match (result, cleanup) {
            (Err(operation_error), Err(cleanup_error)) => Err(PermissionError::ElevationFailed {
                message: format!("{operation_error}; cleanup: {cleanup_error}"),
            }
            .into()),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(PermissionError::ElevationFailed {
                message: format!("操作成功但临时文件清理失败: {error}"),
            }
            .into()),
            (Ok(message), Ok(())) => Ok(message),
        }
    }
}

impl RegistryProvider for WindowsPlatform {
    fn read_string(&self, key: &str, value: &str) -> AppResult<Option<String>> {
        let key_wide = to_wide(std::ffi::OsStr::new(key));
        let value_wide = to_wide(std::ffi::OsStr::new(value));
        let mut buffer = vec![0_u16; 1024];
        let mut byte_length = (buffer.len() * std::mem::size_of::<u16>()) as u32;
        let result = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(key_wide.as_ptr()),
                PCWSTR(value_wide.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut byte_length),
            )
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        result.ok().map_err(|_| RegistryError::Read {
            key: key.to_string(),
            value: value.to_string(),
            source: io::Error::from_raw_os_error(result.0 as i32),
        })?;

        let char_length = (byte_length as usize / std::mem::size_of::<u16>()).min(buffer.len());
        let string = String::from_utf16_lossy(&buffer[..char_length])
            .trim_end_matches('\0')
            .to_string();
        Ok(Some(string))
    }

    fn write_string_elevated(&self, key: &str, value: &str, data: &str) -> AppResult<()> {
        let key = powershell_string(key);
        let value = powershell_string(value);
        let data = powershell_string(data);
        let script =
            format!("Set-ItemProperty -Path 'HKLM:\\{key}' -Name '{value}' -Value '{data}'\n");
        self.run_script(&script).map(|_| ())
    }
}

impl ServiceProvider for WindowsPlatform {
    fn query(&self, name: &str) -> AppResult<ServiceStatus> {
        let service_name = to_wide(std::ffi::OsStr::new(name));
        unsafe {
            let manager = OpenSCManagerW(None, None, SC_MANAGER_CONNECT).map_err(|error| {
                ServiceError::Query {
                    name: name.to_string(),
                    message: error.to_string(),
                }
            })?;
            let service_result =
                OpenServiceW(manager, PCWSTR(service_name.as_ptr()), SERVICE_QUERY_STATUS);
            let manager_close = CloseServiceHandle(manager);
            if let Err(error) = manager_close {
                return Err(ServiceError::Query {
                    name: name.to_string(),
                    message: format!("关闭服务管理器失败: {error}"),
                }
                .into());
            }
            let service = match service_result {
                Ok(service) => service,
                Err(_) => {
                    return Err(ServiceError::NotFound {
                        name: name.to_string(),
                    }
                    .into());
                }
            };

            let mut status = SERVICE_STATUS::default();
            let query_result = QueryServiceStatus(service, &mut status);
            let close_result = CloseServiceHandle(service);
            query_result.map_err(|error| ServiceError::Query {
                name: name.to_string(),
                message: error.to_string(),
            })?;
            close_result.map_err(|error| ServiceError::Query {
                name: name.to_string(),
                message: format!("关闭服务句柄失败: {error}"),
            })?;

            let running = status.dwCurrentState == SERVICE_RUNNING;
            Ok(ServiceStatus {
                running,
                detail: if running { "Running" } else { "Stopped" }.to_string(),
            })
        }
    }

    fn restart_elevated(&self, name: &str) -> AppResult<()> {
        let name = powershell_string(name);
        self.run_script(&format!("Restart-Service -Name '{name}' -Force\n"))
            .map(|_| ())
    }
}

impl ResourceProvider for WindowsPlatform {
    fn bundled_cli_path(&self) -> AppResult<PathBuf> {
        let executable =
            std::env::current_exe().map_err(|error| crate::error::PlatformError::Operation {
                message: format!("获取应用路径失败: {error}"),
            })?;
        let directory =
            executable
                .parent()
                .ok_or_else(|| crate::error::PlatformError::Operation {
                    message: format!("应用路径缺少父目录: {}", executable.display()),
                })?;
        Ok(directory.join("resources").join("ssh-router-cli.exe"))
    }

    fn application_version(&self) -> AppResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }
}

fn unique_id() -> u64 {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    timestamp ^ (std::process::id() as u64) ^ UNIQUE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

fn unique_sibling_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "config".into());
    path.with_file_name(format!(".{file_name}.{}.tmp", unique_id()))
}

fn write_new_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

fn write_utf8_bom(path: &Path, contents: &str) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(contents.len() + 3);
    bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
    bytes.extend_from_slice(contents.as_bytes());
    write_new_file(path, &bytes)
}

fn move_replace(source: &Path, destination: &Path) -> io::Result<()> {
    let source = to_wide(source.as_os_str());
    let destination = to_wide(destination.as_os_str());
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(io::Error::other)
}

fn shell_execute_elevated(wrapper_path: &Path) -> AppResult<()> {
    let verb = to_wide(std::ffi::OsStr::new("runas"));
    let executable = to_wide(std::ffi::OsStr::new("powershell.exe"));
    let parameters = to_wide(std::ffi::OsStr::new(&format!(
        "-ExecutionPolicy Bypass -NoProfile -WindowStyle Hidden -File \"{}\"",
        wrapper_path.display()
    )));
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(executable.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            None,
            SW_HIDE,
        )
    };
    let code = result.0 as usize;
    if code > 32 {
        return Ok(());
    }
    if code == 5 {
        return Err(PermissionError::Cancelled.into());
    }
    Err(PermissionError::ElevationFailed {
        message: format!("ShellExecuteW 返回错误码 {code}"),
    }
    .into())
}

fn wait_for_result(result_path: &Path) -> AppResult<()> {
    let start = Instant::now();
    loop {
        match result_path.try_exists() {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(error) => {
                return Err(PermissionError::ElevationFailed {
                    message: format!("检查提权结果失败: {error}"),
                }
                .into());
            }
        }
        if start.elapsed() >= ELEVATION_TIMEOUT {
            return Err(PermissionError::ElevationFailed {
                message: "操作超时（30 秒）".to_string(),
            }
            .into());
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn read_elevation_result(result_path: &Path) -> AppResult<String> {
    let contents =
        fs::read_to_string(result_path).map_err(|error| PermissionError::ElevationFailed {
            message: format!("读取提权结果失败: {error}"),
        })?;
    let contents = contents.trim_start_matches('\u{feff}').trim();
    let json: serde_json::Value =
        serde_json::from_str(contents).map_err(|error| PermissionError::ElevationFailed {
            message: format!("解析提权结果失败: {error}"),
        })?;
    let success = json
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| PermissionError::ElevationFailed {
            message: "提权结果缺少 success".to_string(),
        })?;
    let message = json
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("未知结果")
        .to_string();

    if success {
        Ok(message)
    } else {
        Err(PermissionError::ElevationFailed { message }.into())
    }
}

fn elevation_wrapper(script_path: &Path, result_path: &Path) -> String {
    format!(
        r#"$ErrorActionPreference = 'Stop'
try {{
    & '{}'
    $result = @{{ success = $true; message = 'OK' }} | ConvertTo-Json -Compress
}} catch {{
    $result = @{{ success = $false; message = $_.Exception.Message }} | ConvertTo-Json -Compress
}}
$result | Out-File -LiteralPath '{}' -Encoding UTF8
"#,
        powershell_literal(script_path),
        powershell_literal(result_path),
    )
}

fn powershell_literal(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn powershell_string(value: &str) -> String {
    value.replace('\'', "''")
}

fn to_wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn cleanup_paths(paths: &[&Path]) -> Result<(), String> {
    let errors: Vec<String> = paths
        .iter()
        .filter_map(|path| {
            remove_file_if_exists(path)
                .err()
                .map(|error| format!("{}: {error}", path.display()))
        })
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn combine_with_cleanup(error: crate::error::AppError, paths: &[&Path]) -> crate::error::AppError {
    match cleanup_paths(paths) {
        Ok(()) => error,
        Err(cleanup_error) => PermissionError::ElevationFailed {
            message: format!("{error}; cleanup: {cleanup_error}"),
        }
        .into(),
    }
}
