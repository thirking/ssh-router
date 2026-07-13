# CLI 打包 + 一键安装 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 CLI exe 打包进 Tauri 安装包，GUI 上增加一键安装 CLI、设置 DefaultShell、重启 sshd 和安装状态检查功能。

**Architecture:** CI 先编译 CLI exe 复制到 `src-tauri/resources/`，Tauri resource 打包进安装包。GUI 新增 `elevate.rs` 模块封装 ShellExecuteW runas 按需 UAC 提权，通过结果文件轮询获取返回值。`commands.rs` 新增四个 Tauri command。前端新增 `StatusPanel` 和 `QuickActions` 组件。

**Tech Stack:** Rust + windows crate 0.61（ShellExecuteW、Registry、Services）、React + shadcn/ui、Tauri v2 resource API。

## Global Constraints

- 目标平台：Windows x64
- 提权方式：ShellExecuteW runas → PowerShell 脚本（不使用 manifest requireAdmin）
- 结果传递：`%TEMP%\ssh-router-action-result.json` 状态文件，轮询最多 30 秒
- CLI 部署路径：`C:\ProgramData\ssh\ssh-router-cli.exe`
- 注册表路径：`HKLM:\SOFTWARE\OpenSSH\DefaultShell`
- 状态检查（`check_status`）不需要提权，读取文件/注册表/服务状态
- `src-tauri/resources/` 目录由 CI 生成，加入 `.gitignore`
- 版本号：0.0.2（本次新增功能）

---

### Task 1: Tauri resource 配置 + CI 调整 + 版本号

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `.github/workflows/release.yml`
- Modify: `.gitignore`
- Modify: `src-tauri/Cargo.toml`（版本号）
- Modify: `crates/config/Cargo.toml`（版本号）
- Modify: `crates/cli/Cargo.toml`（版本号）
- Modify: `package.json`（版本号）
- Modify: `src-tauri/tauri.conf.json`（版本号）

**Interfaces:**
- Produces: `tauri.conf.json` 的 `bundle.resources` 声明，CI 生成 `src-tauri/resources/ssh-router-cli.exe`

- [ ] **Step 1: 修改 tauri.conf.json — 添加 resources 和版本号**

将 `src-tauri/tauri.conf.json` 的 `version` 改为 `"0.0.2"`，`bundle` 中添加 `resources`，窗口高度改为 750：

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SSH Router",
  "version": "0.0.2",
  "identifier": "com.sshrouter.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "SSH Router",
        "width": 800,
        "height": 750,
        "resizable": true,
        "visible": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "resources": ["resources/ssh-router-cli.exe"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 2: 修改 .gitignore — 添加 resources 目录**

在 `.gitignore` 的 Tauri 部分追加：

```
# Tauri resources (CI 生成)
src-tauri/resources/
```

- [ ] **Step 3: 修改 release.yml — CI 先编译 CLI 再复制到 resources**

将 `.github/workflows/release.yml` 中"Build CLI"和"Upload CLI exe to release"两个步骤替换为：

```yaml
      - name: Build CLI and copy to resources
        run: |
          cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc
          New-Item -ItemType Directory -Force -Path src-tauri/resources
          Copy-Item target/x86_64-pc-windows-msvc/release/ssh-router-cli.exe src-tauri/resources/
```

移除最后的 `Upload CLI exe to release` 步骤（CLI 已在安装包内）。

完整的 `release.yml` steps 部分应为：

```yaml
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 22

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
            src-tauri/target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Install npm dependencies
        run: npm ci || npm install

      - name: Build CLI and copy to resources
        run: |
          cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc
          New-Item -ItemType Directory -Force -Path src-tauri/resources
          Copy-Item target/x86_64-pc-windows-msvc/release/ssh-router-cli.exe src-tauri/resources/

      - name: Build Tauri GUI (ssh-router.exe + installers)
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'SSH Router ${{ github.ref_name }}'
          releaseBody: |
            ## SSH Router ${{ github.ref_name }}

            Windows OpenSSH 多端口智能路由器 — Tauri v2 托盘 GUI + Rust CLI。

            ### 安装

            1. 下载下方的 `.msi` 或 `.exe` 安装包安装 GUI 程序
            2. 安装后运行 SSH Router，在主界面点击"安装 CLI"按钮（自动部署 CLI 到 `C:\ProgramData\ssh\`）
            3. 点击"设置 DefaultShell"按钮（自动配置 sshd 使用 CLI）
            4. 点击"重启 sshd"按钮使配置生效
            5. 在路由配置区添加/编辑端口路由，保存即可
          releaseDraft: false
          prerelease: false
```

- [ ] **Step 4: 修改版本号 — 所有 Cargo.toml 和 package.json**

将以下文件的 `version` 从 `"0.0.1"` 改为 `"0.0.2"`：
- `src-tauri/Cargo.toml`
- `crates/config/Cargo.toml`
- `crates/cli/Cargo.toml`
- `package.json`

- [ ] **Step 5: 验证编译**

Run: `cargo check -p ssh-router`
Expected: 编译通过

Run: `npm run build`
Expected: Vite 构建成功

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tauri.conf.json .gitignore .github/workflows/release.yml src-tauri/Cargo.toml crates/config/Cargo.toml crates/cli/Cargo.toml package.json
git commit -m "feat: configure Tauri resource for CLI, bump version to 0.0.2"
```

---

### Task 2: 实现 elevate.rs 模块（UAC 提权封装）

**Files:**
- Create: `src-tauri/src/elevate.rs`
- Modify: `src-tauri/Cargo.toml`（添加 windows crate 依赖）
- Modify: `src-tauri/src/lib.rs`（添加 `mod elevate;`）

**Interfaces:**
- Produces: `elevate::run_elevated(script: &str) -> Result<String, String>` — 以管理员权限执行 PowerShell 脚本，返回成功消息或错误

**注意：此 Task 只能在 Windows 上编译运行。macOS 上 `cargo check` 只能验证语法。**

- [ ] **Step 1: 修改 src-tauri/Cargo.toml 添加 windows crate**

在 `[dependencies]` 之后添加 target-specific 依赖：

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_Foundation",
    "Win32_System_Threading",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
] }
```

- [ ] **Step 2: 创建 elevate.rs 模块**

```rust
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

    // 写实际脚本
    let script_path = script_file_path();
    fs::write(&script_path, script)
        .map_err(|e| format!("write script: {}", e))?;

    // 写 wrapper 脚本：执行实际脚本，写结果 JSON
    let wrapper_script = format!(
        r#"$ErrorActionPreference = "Stop"
try {{
    & "{script}"
    $result = @{{ success = $true; message = "操作成功" }} | ConvertTo-Json
}} catch {{
    $result = @{{ success = $false; message = $_.Exception.Message }} | ConvertTo-Json
}}
$result | Out-File -FilePath "{result}" -Encoding UTF8
"#,
        script = script_path.to_string_lossy().replace('\\', "\\\\"),
        result = result_path.to_string_lossy().replace('\\', "\\\\"),
    );

    let wrapper_path = wrapper_file_path();
    fs::write(&wrapper_path, &wrapper_script)
        .map_err(|e| format!("write wrapper: {}", e))?;

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
```

- [ ] **Step 3: 修改 lib.rs 添加模块声明**

在 `src-tauri/src/lib.rs` 的 `mod commands;` 后添加：

```rust
mod elevate;
```

- [ ] **Step 4: 验证测试和编译**

Run: `cargo test -p ssh-router elevate`
Expected: 4 tests passed

Run: `cargo check -p ssh-router`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/elevate.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: add elevate.rs module for UAC elevation via ShellExecuteW"
```

---

### Task 3: 实现安装状态检查（check_status）

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/Cargo.toml`（添加 Registry 和 Services features）

**Interfaces:**
- Consumes: 无
- Produces: `commands::check_status() -> Result<Status, String>` Tauri command

**注意：此 Task 只能在 Windows 上编译运行。macOS 上 `cargo check` 只能验证语法。**

- [ ] **Step 1: 修改 Cargo.toml 添加 Registry 和 Services features**

将 `src-tauri/Cargo.toml` 的 windows features 列表扩展为：

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_Foundation",
    "Win32_System_Threading",
    "Win32_System_Registry",
    "Win32_System_Services",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
] }
```

- [ ] **Step 2: 在 commands.rs 添加 Status 结构和 check_status 命令**

在 `src-tauri/src/commands.rs` 末尾追加：

```rust
use serde::Serialize;

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
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
    };

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
            windows::core::PCWSTR(sub_key.as_ptr()),
            windows::core::PCWSTR(value_name.as_ptr()),
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
    use windows::Win32::System::Services::{
        OpenSCManagerW, OpenServiceW, QueryServiceStatus, CloseServiceHandle,
        SERVICE_STATUS, SC_MANAGER_CONNECT, SERVICE_QUERY_STATUS,
    };
    use windows::core::PCWSTR;

    let sshd_name: Vec<u16> = "sshd"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let h_scm = OpenSCManagerW(
            None,
            None,
            SC_MANAGER_CONNECT,
        );

        if h_scm.is_err() {
            return (false, "Not installed".to_string());
        }

        let h_scm = h_scm.unwrap();
        let h_service = OpenServiceW(
            h_scm,
            PCWSTR(sshd_name.as_ptr()),
            SERVICE_QUERY_STATUS,
        );

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

        // SERVICE_RUNNING = 4
        let running = status.dwCurrentState == 4;
        let status_str = if running {
            "Running".to_string()
        } else {
            "Stopped".to_string()
        };
        (running, status_str)
    }
}
```

- [ ] **Step 3: 修改 lib.rs 注册 check_status 命令**

在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中添加 `check_status`：

```rust
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::create_default_config,
            commands::check_status,
        ])
```

- [ ] **Step 4: 验证编译**

Run: `cargo check -p ssh-router`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: add check_status command for installation status"
```

---

### Task 4: 实现安装 CLI、设置 DefaultShell、重启 sshd 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`（注册新命令）

**Interfaces:**
- Consumes: `elevate::run_elevated(script: &str) -> Result<String, String>`（Task 2）
- Produces: `commands::install_cli(app: AppHandle) -> Result<String, String>`、`commands::set_default_shell() -> Result<String, String>`、`commands::restart_sshd() -> Result<String, String>`

- [ ] **Step 1: 在 commands.rs 添加三个提权命令**

在 `src-tauri/src/commands.rs` 末尾追加：

```rust
use tauri::AppHandle;
use tauri::Manager;

/// 安装 CLI：从 Tauri resource 释放到 C:\ProgramData\ssh\
#[tauri::command]
pub fn install_cli(app: AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // 获取 resource 目录中的 CLI exe 路径
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|e| format!("get resource dir: {}", e))?;
        let src = resource_dir.join("ssh-router-cli.exe");
        let src_path = src.to_string_lossy().replace('\\', "\\\\");

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

        crate::elevate::run_elevated(&script)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("Install CLI is only available on Windows".to_string())
    }
}

/// 设置 DefaultShell 注册表
#[tauri::command]
pub fn set_default_shell() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let script = r#"Set-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name "DefaultShell" -Value "C:\ProgramData\ssh\ssh-router-cli.exe"
"#;
        crate::elevate::run_elevated(script)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Set DefaultShell is only available on Windows".to_string())
    }
}

/// 重启 sshd 服务
#[tauri::command]
pub fn restart_sshd() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let script = r#"Restart-Service sshd -Force
"#;
        crate::elevate::run_elevated(script)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Restart sshd is only available on Windows".to_string())
    }
}
```

- [ ] **Step 2: 修改 lib.rs 注册三个新命令**

将 `src-tauri/src/lib.rs` 的 `invoke_handler` 更新为：

```rust
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::create_default_config,
            commands::check_status,
            commands::install_cli,
            commands::set_default_shell,
            commands::restart_sshd,
        ])
```

- [ ] **Step 3: 验证编译**

Run: `cargo check -p ssh-router`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add install_cli, set_default_shell, restart_sshd commands"
```

---

### Task 5: 前端 — 状态面板 + 快捷操作 + api.ts

**Files:**
- Modify: `src/lib/api.ts`
- Create: `src/components/StatusPanel.tsx`
- Create: `src/components/QuickActions.tsx`
- Modify: `src/App.tsx`

**Interfaces:**
- Consumes: Tauri commands `check_status`、`install_cli`、`set_default_shell`、`restart_sshd`（Task 3-4）

- [ ] **Step 1: 修改 api.ts — 添加 Status 接口和四个函数**

在 `src/lib/api.ts` 末尾追加：

```typescript
export interface Status {
  cliDeployed: boolean
  cliPath: string
  defaultShellSet: boolean
  defaultShellValue: string
  configExists: boolean
  sshdRunning: boolean
  sshdStatus: string
}

export async function checkStatus(): Promise<Status> {
  return invoke<Status>("check_status")
}

export async function installCli(): Promise<string> {
  return invoke<string>("install_cli")
}

export async function setDefaultShell(): Promise<string> {
  return invoke<string>("set_default_shell")
}

export async function restartSshd(): Promise<string> {
  return invoke<string>("restart_sshd")
}
```

- [ ] **Step 2: 创建 StatusPanel.tsx**

```tsx
import { CheckCircle2, XCircle, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/button"
import type { Status } from "@/lib/api"

interface StatusPanelProps {
  status: Status | null
  loading: boolean
  onRefresh: () => void
}

function StatusItem({ ok, label, detail }: { ok: boolean; label: string; detail?: string }) {
  return (
    <div className="flex items-center gap-2">
      {ok ? (
        <CheckCircle2 className="h-4 w-4 text-green-500" />
      ) : (
        <XCircle className="h-4 w-4 text-red-500" />
      )}
      <span className="text-sm">{label}</span>
      {detail && <span className="text-xs text-muted-foreground truncate max-w-64">{detail}</span>}
    </div>
  )
}

export function StatusPanel({ status, loading, onRefresh }: StatusPanelProps) {
  return (
    <div className="rounded-lg border p-4 mb-4">
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-lg font-semibold">安装状态</h2>
        <Button variant="outline" size="sm" onClick={onRefresh} disabled={loading}>
          <RefreshCw className={`h-4 w-4 mr-1 ${loading ? "animate-spin" : ""}`} />
          刷新
        </Button>
      </div>
      {status ? (
        <div className="grid grid-cols-2 gap-2">
          <StatusItem
            ok={status.cliDeployed}
            label="CLI 已部署"
            detail={status.cliDeployed ? status.cliPath : undefined}
          />
          <StatusItem
            ok={status.defaultShellSet}
            label="DefaultShell 已设置"
            detail={status.defaultShellSet ? status.defaultShellValue : undefined}
          />
          <StatusItem
            ok={status.configExists}
            label="配置文件存在"
          />
          <StatusItem
            ok={status.sshdRunning}
            label="sshd 服务"
            detail={status.sshdStatus}
          />
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">
          {loading ? "检查中..." : "点击刷新检查状态"}
        </p>
      )}
    </div>
  )
}
```

- [ ] **Step 3: 创建 QuickActions.tsx**

```tsx
import { useState } from "react"
import { Button } from "@/components/ui/button"
import { toast } from "sonner"
import { installCli, setDefaultShell, restartSshd, checkStatus, type Status } from "@/lib/api"

interface QuickActionsProps {
  onStatusRefresh: () => void
}

export function QuickActions({ onStatusRefresh }: QuickActionsProps) {
  const [loadingAction, setLoadingAction] = useState<string | null>(null)

  const runAction = async (name: string, fn: () => Promise<string>) => {
    setLoadingAction(name)
    try {
      const msg = await fn()
      toast.success(name + "成功", { description: msg })
      onStatusRefresh()
    } catch (err) {
      toast.error(name + "失败", { description: String(err) })
    } finally {
      setLoadingAction(null)
    }
  }

  return (
    <div className="rounded-lg border p-4 mb-4">
      <h2 className="text-lg font-semibold mb-2">快捷操作</h2>
      <div className="flex gap-2 flex-wrap">
        <Button
          variant="outline"
          onClick={() => runAction("安装 CLI", installCli)}
          disabled={loadingAction !== null}
        >
          {loadingAction === "安装 CLI" ? "安装中..." : "安装 CLI"}
        </Button>
        <Button
          variant="outline"
          onClick={() => runAction("设置 DefaultShell", setDefaultShell)}
          disabled={loadingAction !== null}
        >
          {loadingAction === "设置 DefaultShell" ? "设置中..." : "设置 DefaultShell"}
        </Button>
        <Button
          variant="outline"
          onClick={() => runAction("重启 sshd", restartSshd)}
          disabled={loadingAction !== null}
        >
          {loadingAction === "重启 sshd" ? "重启中..." : "重启 sshd"}
        </Button>
      </div>
    </div>
  )
}
```

- [ ] **Step 4: 修改 App.tsx — 集成状态面板和快捷操作**

在 `src/App.tsx` 中添加状态管理和新组件。修改 import 和函数体：

在 import 部分添加：

```tsx
import { StatusPanel } from "@/components/StatusPanel"
import { QuickActions } from "@/components/QuickActions"
import { checkStatus, type Status } from "@/lib/api"
```

在 `App` 函数中，`loadError` state 后添加：

```tsx
  const [status, setStatus] = useState<Status | null>(null)
  const [statusLoading, setStatusLoading] = useState(false)
```

添加刷新状态的函数（在 `useEffect` 之后）：

```tsx
  const refreshStatus = () => {
    setStatusLoading(true)
    checkStatus()
      .then(s => setStatus(s))
      .catch(err => toast.error("状态检查失败", { description: String(err) }))
      .finally(() => setStatusLoading(false))
  }
```

修改 `useEffect`，在加载配置后也刷新状态：

```tsx
  useEffect(() => {
    loadConfig()
      .then(cfg => {
        setConfig(cfg)
        setSftpCommand(cfg.sftpCommand)
      })
      .catch(err => {
        const msg = String(err)
        setLoadError(msg)
        toast.error("加载配置失败", { description: msg })
      })
    refreshStatus()
  }, [])
```

在主界面 return 中，`<h1>` 之后、端口路由之前添加：

```tsx
      <StatusPanel status={status} loading={statusLoading} onRefresh={refreshStatus} />
      <QuickActions onStatusRefresh={refreshStatus} />
```

完整的 `return` 部分（当 config 存在时）应为：

```tsx
  return (
    <div className="container mx-auto p-6">
      <Toaster />
      <h1 className="text-2xl font-bold mb-6">SSH Router 配置</h1>

      <StatusPanel status={status} loading={statusLoading} onRefresh={refreshStatus} />
      <QuickActions onStatusRefresh={refreshStatus} />

      <div className="mb-4">
        <h2 className="text-lg font-semibold mb-2">端口路由</h2>
        <RouteTable routes={routes} onEdit={handleEdit} onDelete={handleDelete} />
        <Button className="mt-2" onClick={handleAdd}>添加路由</Button>
      </div>

      <div className="mb-6">
        <SftpField value={sftpCommand} onChange={setSftpCommand} />
      </div>

      <Button onClick={handleSave}>保存配置</Button>

      <RouteDialog
        open={dialogOpen}
        route={editingIndex !== null ? routes[editingIndex] : null}
        onSave={handleSaveRoute}
        onClose={() => setDialogOpen(false)}
      />
    </div>
  )
```

- [ ] **Step 5: 安装 lucide-react 图标**

Run: `npm install lucide-react`
Expected: 已安装（package.json 中已有），确认即可

- [ ] **Step 6: 验证前端构建**

Run: `npm run build`
Expected: TypeScript + Vite 构建成功

- [ ] **Step 7: 验证 Tauri 后端编译**

Run: `cargo check -p ssh-router`
Expected: 编译通过

- [ ] **Step 8: Commit**

```bash
git add src/lib/api.ts src/components/StatusPanel.tsx src/components/QuickActions.tsx src/App.tsx
git commit -m "feat: add StatusPanel and QuickActions to GUI"
```

---

### Task 6: 整体验证和推送

**Files:**
- 无新文件，仅验证

- [ ] **Step 1: 验证所有 Rust 测试通过**

Run: `cargo test --workspace`
Expected: 所有测试通过

- [ ] **Step 2: 验证 CLI 交叉编译**

Run: `cargo check -p ssh-router-cli --target x86_64-pc-windows-msvc`
Expected: 编译通过

- [ ] **Step 3: 验证 Tauri 后端编译**

Run: `cargo check -p ssh-router`
Expected: 编译通过

- [ ] **Step 4: 验证前端构建**

Run: `npm run build`
Expected: Vite 构建成功

- [ ] **Step 5: 推送并打 tag**

```bash
git push
git tag v0.0.2
git push origin v0.0.2
```

- [ ] **Step 6: 监控 CI/CD**

Run: `gh run watch --exit-status`
Expected: Release workflow 全部通过

- [ ] **Step 7: 验证 Release 产物**

Run: `gh release view v0.0.2`
Expected: Release 包含 MSI 安装包、NSIS 安装程序（CLI 已打包在安装包内，不再单独上传）
