use ssh_router_config::Config;
use std::fs;
use std::path::Path;

/// 配置文件路径（与 Windows 上 OpenSSH 固定安装位置对齐）
const CONFIG_PATH: &str = r"C:\ProgramData\ssh\ssh-router.json";

/// 读取并反序列化配置文件
#[tauri::command]
pub fn load_config() -> Result<Config, String> {
    let content = fs::read_to_string(CONFIG_PATH).map_err(|e| format!("read config: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("parse config: {}", e))
}

/// 校验并持久化配置：必须恰好有一条默认路由
#[tauri::command]
pub async fn save_config(config: Config) -> Result<(), String> {
    // 校验恰好有一条 default
    let defaults: Vec<_> = config.routes.iter().filter(|r| r.default).collect();
    if defaults.len() != 1 {
        return Err(format!(
            "必须恰好有一条默认路由，当前有 {} 条",
            defaults.len()
        ));
    }
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("serialize config: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        write_config_elevated(&json)?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = json;
        Err("Save config is only available on Windows".to_string())
    }
}

/// 创建默认配置并写入磁盘（若目录不存在则创建）
#[tauri::command]
pub async fn create_default_config() -> Result<Config, String> {
    let config = Config::default_config();
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("serialize config: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        write_config_elevated(&json)?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = json;
        return Err("Create default config is only available on Windows".to_string());
    }

    Ok(config)
}

/// 通过 UAC 提权将配置 JSON 写入 C:\ProgramData\ssh\ssh-router.json
///
/// 先写到临时文件，再用提权的 PowerShell 复制到目标路径，
/// 避免在 PowerShell 脚本中嵌入大段 JSON（转义问题）。
#[cfg(target_os = "windows")]
fn write_config_elevated(json: &str) -> Result<(), String> {
    use tauri::async_runtime::spawn_blocking;

    // 写 JSON 到临时文件
    let tmp = std::env::temp_dir().join("ssh-router-config-tmp.json");
    fs::write(&tmp, json).map_err(|e| format!("write temp config: {}", e))?;
    let tmp_path = tmp.to_string_lossy();

    let script = format!(
        r#"$src = "{src}"
$dst = "C:\ProgramData\ssh\ssh-router.json"
if (-not (Test-Path "C:\ProgramData\ssh")) {{
    New-Item -ItemType Directory -Path "C:\ProgramData\ssh" -Force
}}
Copy-Item $src $dst -Force
Remove-Item $src -Force
"#,
        src = tmp_path,
    );

    spawn_blocking(move || crate::elevate::run_elevated(&script))
        .await
        .map_err(|e| format!("spawn_blocking join: {}", e))?
        .map(|_| ())
}

use serde::Serialize;
use tauri::async_runtime::spawn_blocking;
use tauri::{AppHandle, Manager};

/// 安装状态检查结果
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub cli_deployed: bool,
    pub cli_path: String,
    pub default_shell_set: bool,
    pub default_shell_value: String,
    pub config_exists: bool,
    pub sshd_running: bool,
    pub sshd_status: String,
}

const CLI_DEPLOY_PATH: &str = r"C:\ProgramData\ssh\ssh-router-cli.exe";

/// 检查安装状态（不需要管理员权限）
#[tauri::command]
pub fn check_status() -> Result<Status, String> {
    #[cfg(target_os = "windows")]
    {
        // CLI 部署检查
        let cli_deployed = Path::new(CLI_DEPLOY_PATH).exists();

        // 注册表读取 DefaultShell
        let (default_shell_value, default_shell_set) = read_default_shell();

        // 配置文件检查
        let config_exists = Path::new(CONFIG_PATH).exists();

        // sshd 服务状态
        let (sshd_running, sshd_status) = check_sshd_service();

        Ok(Status {
            cli_deployed,
            cli_path: CLI_DEPLOY_PATH.to_string(),
            default_shell_set,
            default_shell_value,
            config_exists,
            sshd_running,
            sshd_status,
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Status check is only available on Windows".to_string())
    }
}

/// 读取注册表 HKLM\SOFTWARE\OpenSSH\DefaultShell
#[cfg(target_os = "windows")]
fn read_default_shell() -> (String, bool) {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RegGetValueW, RRF_RT_REG_SZ};

    let sub_key: Vec<u16> = "SOFTWARE\\OpenSSH"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = "DefaultShell"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut buf = [0u16; 1024];
    let mut buf_len = (buf.len() * 2) as u32;

    let result = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(sub_key.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut buf_len),
        )
    };

    if result.is_err() {
        return (String::new(), false);
    }

    let len = (buf_len as usize) / 2;
    let value = String::from_utf16_lossy(&buf[..len]);
    let value = value.trim_end_matches('\0').to_string();
    let is_set = value.eq_ignore_ascii_case(CLI_DEPLOY_PATH);
    (value, is_set)
}

/// 查询 sshd 服务状态
#[cfg(target_os = "windows")]
fn check_sshd_service() -> (bool, String) {
    use windows::core::PCWSTR;
    use windows::Win32::System::Services::{
        CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_MANAGER_CONNECT,
        SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATUS,
    };

    let sshd_name: Vec<u16> = "sshd"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let h_scm = OpenSCManagerW(None, None, SC_MANAGER_CONNECT);

        if h_scm.is_err() {
            return (false, "Not installed".to_string());
        }

        let h_scm = h_scm.unwrap();
        let h_service =
            OpenServiceW(h_scm, PCWSTR(sshd_name.as_ptr()), SERVICE_QUERY_STATUS);

        let _ = CloseServiceHandle(h_scm);

        if h_service.is_err() {
            return (false, "Not installed".to_string());
        }

        let h_service = h_service.unwrap();
        let mut status = SERVICE_STATUS::default();
        let result = QueryServiceStatus(h_service, &mut status);
        let _ = CloseServiceHandle(h_service);

        if result.is_err() {
            return (false, "Unknown".to_string());
        }

        let running = status.dwCurrentState == SERVICE_RUNNING;
        let status_str = if running {
            "Running".to_string()
        } else {
            "Stopped".to_string()
        };
        (running, status_str)
    }
}

/// 安装 CLI：从 Tauri resource 释放到 C:\ProgramData\ssh\
///
/// 注：`run_elevated` 内部轮询等待结果（最多 30 秒），为避免阻塞 Tauri
/// 主线程导致 UI 卡死，此处使用 `spawn_blocking` 在专用阻塞线程上执行。
#[tauri::command]
pub async fn install_cli(app: AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // 获取 resource 目录中的 CLI exe 路径（需要在主线程外执行 IO 前准备好路径）
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|e| format!("get resource dir: {}", e))?;
        let src = resource_dir.join("ssh-router-cli.exe");
        let src_path = src.to_string_lossy();

        let script = format!(
            r#"$src = "{src}"
$dst = "C:\ProgramData\ssh\ssh-router-cli.exe"
if (-not (Test-Path "C:\ProgramData\ssh")) {{
    New-Item -ItemType Directory -Path "C:\ProgramData\ssh" -Force
}}
Copy-Item $src $dst -Force
"#,
            src = src_path,
        );

        spawn_blocking(move || crate::elevate::run_elevated(&script))
            .await
            .map_err(|e| format!("spawn_blocking join: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("Install CLI is only available on Windows".to_string())
    }
}

/// 设置 DefaultShell 注册表
#[tauri::command]
pub async fn set_default_shell() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let script = r#"Set-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name "DefaultShell" -Value "C:\ProgramData\ssh\ssh-router-cli.exe"
"#;
        spawn_blocking(move || crate::elevate::run_elevated(script))
            .await
            .map_err(|e| format!("spawn_blocking join: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Set DefaultShell is only available on Windows".to_string())
    }
}

/// 重启 sshd 服务
#[tauri::command]
pub async fn restart_sshd() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let script = r#"Restart-Service sshd -Force
"#;
        spawn_blocking(move || crate::elevate::run_elevated(script))
            .await
            .map_err(|e| format!("spawn_blocking join: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Restart sshd is only available on Windows".to_string())
    }
}
