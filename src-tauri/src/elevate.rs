use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// 结果文件路径（每次调用唯一，避免连续操作读到上一次的结果）
fn result_file_path(id: u64) -> PathBuf {
    std::env::temp_dir().join(format!("ssh-router-result-{}.json", id))
}

/// 实际脚本路径
fn script_file_path(id: u64) -> PathBuf {
    std::env::temp_dir().join(format!("ssh-router-action-{}.ps1", id))
}

/// Wrapper 脚本路径
fn wrapper_file_path(id: u64) -> PathBuf {
    std::env::temp_dir().join(format!("ssh-router-wrapper-{}.ps1", id))
}

/// 生成唯一 ID（用时间戳 + 随机数）
fn unique_id() -> u64 {
    use std::time::SystemTime;
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    ts ^ (std::process::id() as u64)
}

/// 以管理员权限执行 PowerShell 脚本
///
/// 1. 写实际脚本到临时文件
/// 2. 写 wrapper 脚本（执行实际脚本 + 写结果 JSON）
/// 3. ShellExecuteW(runas) 启动 powershell.exe
/// 4. 轮询结果文件（最多 30 秒）
/// 5. 读取结果，删除临时文件，返回
#[cfg(target_os = "windows")]
pub fn run_elevated(script: &str) -> Result<String, String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    // 清理之前的结果文件
    let id = unique_id();
    let result_path = result_file_path(id);

    // 写实际脚本（加 UTF-8 BOM，让 PowerShell 5.1 正确以 UTF-8 读取）
    let script_path = script_file_path(id);
    let mut script_bytes = Vec::with_capacity(script.len() + 3);
    script_bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    script_bytes.extend_from_slice(script.as_bytes());
    fs::write(&script_path, &script_bytes).map_err(|e| format!("write script: {}", e))?;

    // 写 wrapper 脚本：执行实际脚本，写结果 JSON
    // 用英文避免 PowerShell 5.1 ANSI 编码读取中文乱码
    let wrapper_script = format!(
        r#"$ErrorActionPreference = "Stop"
try {{
    & "{script}"
    $result = @{{ success = $true; message = "OK" }} | ConvertTo-Json
}} catch {{
    $result = @{{ success = $false; message = $_.Exception.Message }} | ConvertTo-Json
}}
$result | Out-File -FilePath "{result}" -Encoding UTF8
"#,
        script = script_path.to_string_lossy(),
        result = result_path.to_string_lossy(),
    );

    let wrapper_path = wrapper_file_path(id);
    // 加 UTF-8 BOM，让 PowerShell 5.1 正确以 UTF-8 读取脚本
    let mut wrapper_bytes = Vec::with_capacity(wrapper_script.len() + 3);
    wrapper_bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    wrapper_bytes.extend_from_slice(wrapper_script.as_bytes());
    fs::write(&wrapper_path, &wrapper_bytes).map_err(|e| format!("write wrapper: {}", e))?;

    // ShellExecuteW runas 启动 PowerShell
    let powershell = to_wide("powershell.exe");
    let params = to_wide(&format!(
        "-ExecutionPolicy Bypass -NoProfile -WindowStyle Hidden -File \"{}\"",
        wrapper_path.to_string_lossy()
    ));
    let verb = to_wide("runas");

    let h_inst = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(powershell.as_ptr()),
            PCWSTR(params.as_ptr()),
            None,
            SW_HIDE,
        )
    };

    // ShellExecuteW 返回值 > 32 表示成功
    if h_inst.0 as usize <= 32 {
        // 清理临时文件
        let _ = fs::remove_file(&script_path);
        let _ = fs::remove_file(&wrapper_path);
        return Err(format!(
            "ShellExecuteW failed, error code: {}",
            h_inst.0 as usize
        ));
    }

    // 轮询等待结果文件（最多 30 秒）
    let start = Instant::now();
    let timeout = Duration::from_secs(30);

    loop {
        if result_path.exists() {
            break;
        }
        if start.elapsed() >= timeout {
            // 清理临时文件
            let _ = fs::remove_file(&script_path);
            let _ = fs::remove_file(&wrapper_path);
            return Err("操作超时（30秒），请检查状态".to_string());
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    // 读取结果
    let result_content = fs::read_to_string(&result_path)
        .map_err(|e| format!("read result: {}", e))?;

    // 清理临时文件
    let _ = fs::remove_file(&script_path);
    let _ = fs::remove_file(&wrapper_path);
    let _ = fs::remove_file(&result_path);

    // 去掉 BOM 并解析 JSON
    let result_content = result_content.trim_start_matches('\u{feff}').trim();

    // 用 serde_json 解析，避免手写解析器在多行/空格差异上出错
    let json: serde_json::Value = serde_json::from_str(result_content)
        .map_err(|e| format!("parse result JSON: {} | raw: {}", e, result_content))?;

    let success = json.get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let message = json.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("未知结果")
        .to_string();

    if success {
        Ok(message)
    } else {
        Err(message)
    }
}

/// &str 转 UTF-16 null-terminated Vec<u16>
#[cfg(target_os = "windows")]
fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// 非 Windows 平台的 stub
#[cfg(not(target_os = "windows"))]
pub fn run_elevated(_script: &str) -> Result<String, String> {
    Err("UAC elevation is only available on Windows".to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // elevate.rs 的核心逻辑依赖 Windows API，无法在 macOS 上测试。
        // JSON 解析现在用 serde_json，不需要手写解析器测试。
    }
}
