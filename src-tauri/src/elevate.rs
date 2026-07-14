use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// 结果文件路径
fn result_file_path() -> PathBuf {
    std::env::temp_dir().join("ssh-router-action-result.json")
}

/// 实际脚本路径
fn script_file_path() -> PathBuf {
    std::env::temp_dir().join("ssh-router-action.ps1")
}

/// Wrapper 脚本路径
fn wrapper_file_path() -> PathBuf {
    std::env::temp_dir().join("ssh-router-wrapper.ps1")
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
    let result_path = result_file_path();
    let _ = fs::remove_file(&result_path);

    // 写实际脚本（加 UTF-8 BOM，让 PowerShell 5.1 正确以 UTF-8 读取）
    let script_path = script_file_path();
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

    let wrapper_path = wrapper_file_path();
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

    // 解析 JSON: {"success": true, "message": "..."} 或 {"success": false, "message": "..."}
    // PowerShell ConvertTo-Json 输出可能带 BOM
    let result_content = result_content.trim_start_matches('\u{feff}').trim();

    // 简单解析（避免引入 serde_json 到 elevate 模块）
    let success = result_content.contains("\"success\": true")
        || result_content.contains("\"success\":true");
    let message = extract_json_value(result_content, "message")
        .unwrap_or_else(|| "未知结果".to_string());

    if success {
        Ok(message)
    } else {
        Err(message)
    }
}

/// 从 JSON 字符串中提取指定键的值（简单实现，不依赖 serde）
fn extract_json_value(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let rest = rest.trim_start();
    if rest.starts_with('"') {
        // 字符串值
        let start = 1;
        let end = rest[1..].find('"')? + 1;
        Some(rest[start..end].to_string())
    } else {
        // 非字符串值（true/false/数字）
        let end = rest.find(|c: char| c == ',' || c == '}' || c.is_whitespace())?;
        Some(rest[..end].trim().to_string())
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
    use super::*;

    #[test]
    fn test_extract_json_value_string() {
        let json = r#"{"success": true, "message": "操作成功"}"#;
        assert_eq!(extract_json_value(json, "message"), Some("操作成功".to_string()));
    }

    #[test]
    fn test_extract_json_value_boolean() {
        let json = r#"{"success": true, "message": "ok"}"#;
        assert_eq!(extract_json_value(json, "success"), Some("true".to_string()));
    }

    #[test]
    fn test_extract_json_value_missing() {
        let json = r#"{"success": true}"#;
        assert_eq!(extract_json_value(json, "message"), None);
    }

    #[test]
    fn test_extract_json_value_with_spaces() {
        let json = r#"{"success":  true,  "message":  "done"}"#;
        assert_eq!(extract_json_value(json, "message"), Some("done".to_string()));
    }
}
