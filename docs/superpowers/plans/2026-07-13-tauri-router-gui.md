# SSH Router 托盘 GUI + 配置化路由 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将现有 C# 单 exe SSH 路由方案重构为 Tauri v2 托盘 GUI + Rust CLI 双程序方案，通过 JSON 配置文件驱动路由。

**Architecture:** Cargo workspace 包含三个 crate：`ssh-router-config`（共享数据结构）、`ssh-router-cli`（被 sshd 调起的路由 CLI）、`src-tauri`（Tauri v2 托盘 GUI）。CLI 和 GUI 通过 JSON 配置文件解耦。CLI 完整移植现有 C# 的 CreateProcessW + Job Object + 临时文件 + SFTP 处理逻辑。

**Tech Stack:** Rust + windows crate 0.61（CLI）、Tauri v2 + React + shadcn/ui + Vite（GUI）、serde/serde_json（配置序列化）。

## Global Constraints

- 目标平台：Windows x64（`x86_64-pc-windows-msvc`）
- 配置文件路径：`C:\ProgramData\ssh\ssh-router.json`
- 日志文件路径：`C:\ProgramData\ssh\ssh-router-debug.log`
- CLI 必须用 `CreateProcessW`（不是 `Process.Start` / `std::process::Command`）启动子进程，以正确继承 stdin/stdout/stderr 句柄
- CLI 必须用 Job Object（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）确保子进程在父进程退出时被杀死
- 命令通过临时文件传递（`%TEMP%\ssh-cmd-<PID>.<ext>`），不做 shell quoting 转义
- Rust 结构体用 `#[serde(rename_all = "camelCase")]` 与 JSON camelCase 键名对应
- `default` 字段用 `#[serde(default)]`
- 现有 `SshRouter.cs` / `SshRouter.csproj` / `fix-sshd.ps1` 保留不删除
- Windows 特定 API 无法在 macOS 上运行测试，CI/构建需在 Windows 上进行

## File Structure

```
ssh-router/
├── Cargo.toml                      # workspace 定义
├── crates/
│   ├── config/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs              # Config, Route 结构 + serde + 默认配置
│   └── cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # 入口 + 流程编排
│           ├── routing.rs          # 端口匹配 + 模板替换
│           ├── win32.rs            # CreateProcessW + Job Object 封装
│           └── log.rs              # 日志写入
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   ├── src/
│   │   ├── main.rs                 # Tauri 入口 + 托盘 + 单例
│   │   └── commands.rs             # #[tauri::command] 文件 I/O
│   └── capabilities/
│       └── default.json
├── src/                            # React 前端
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── RouteTable.tsx
│   │   ├── RouteDialog.tsx
│   │   └── SftpField.tsx
│   └── lib/
│       └── api.ts
├── components.json                 # shadcn/ui 配置
├── package.json
├── tsconfig.json
├── vite.config.ts
└── index.html
```

---

### Task 1: 创建 Cargo workspace 和 config crate

**Files:**
- Create: `Cargo.toml`
- Create: `crates/config/Cargo.toml`
- Create: `crates/config/src/lib.rs`

**Interfaces:**
- Produces: `ssh-router-config` crate，导出 `Config`、`Route` 结构体，以及 `Config::default_config()` 函数返回三端口默认配置

- [ ] **Step 1: 创建 workspace 根 Cargo.toml**

```toml
[workspace]
members = ["crates/config", "crates/cli", "src-tauri"]
resolver = "2"
```

- [ ] **Step 2: 创建 config crate 的 Cargo.toml**

```toml
[package]
name = "ssh-router-config"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 3: 创建 config crate 的 lib.rs**

```rust
use serde::{Deserialize, Serialize};

/// SSH Router 配置，序列化为 ssh-router.json
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub routes: Vec<Route>,
    pub sftp_command: String,
}

/// 单条端口路由规则
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub port: u16,
    pub name: String,
    pub shell: String,
    pub interactive_template: String,
    pub command_template: String,
    pub tmp_file_ext: String,
    #[serde(default)]
    pub default: bool,
}

impl Config {
    /// 返回移植自现有 C# 硬编码的三端口默认配置
    pub fn default_config() -> Self {
        Config {
            routes: vec![
                Route {
                    port: 22,
                    name: "PowerShell".to_string(),
                    shell: "C:\\Program Files\\PowerShell\\7\\pwsh.exe".to_string(),
                    interactive_template: "\"{shell}\" -l".to_string(),
                    command_template: "\"{shell}\" -File \"{tmpfile}\"".to_string(),
                    tmp_file_ext: ".ps1".to_string(),
                    default: true,
                },
                Route {
                    port: 2222,
                    name: "Git Bash".to_string(),
                    shell: "C:\\Program Files\\Git\\usr\\bin\\bash.exe".to_string(),
                    interactive_template: "\"{shell}\" -l".to_string(),
                    command_template: "\"{shell}\" -l -c '. \"{tmpfile}\"'".to_string(),
                    tmp_file_ext: ".sh".to_string(),
                    default: false,
                },
                Route {
                    port: 2223,
                    name: "WSL Ubuntu".to_string(),
                    shell: "wsl.exe".to_string(),
                    interactive_template:
                        "wsl.exe -d Ubuntu -- bash -lc 'cd ~ && exec bash -l'"
                            .to_string(),
                    command_template:
                        "wsl.exe -d Ubuntu -- bash -c 'cd ~ && . \"{tmpfile_wsl}\"'"
                            .to_string(),
                    tmp_file_ext: ".sh".to_string(),
                    default: false,
                },
            ],
            sftp_command: "cmd.exe /c \"C:\\Windows\\System32\\OpenSSH\\sftp-server.exe\""
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_three_routes() {
        let config = Config::default_config();
        assert_eq!(config.routes.len(), 3);
    }

    #[test]
    fn test_default_config_has_exactly_one_default() {
        let config = Config::default_config();
        let defaults: Vec<_> = config.routes.iter().filter(|r| r.default).collect();
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].port, 22);
    }

    #[test]
    fn test_serde_camel_case_roundtrip() {
        let config = Config::default_config();
        let json = serde_json::to_string(&config).unwrap();
        // camelCase 键名
        assert!(json.contains("interactiveTemplate"));
        assert!(json.contains("commandTemplate"));
        assert!(json.contains("tmpFileExt"));
        assert!(json.contains("sftpCommand"));
        // 不含 snake_case
        assert!(!json.contains("interactive_template"));
        assert!(!json.contains("command_template"));

        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.routes.len(), 3);
    }

    #[test]
    fn test_default_field_serde_default() {
        // default 字段缺失时应默认为 false
        let json = r#"{
            "routes": [{
                "port": 22,
                "name": "Test",
                "shell": "pwsh.exe",
                "interactiveTemplate": "\"{shell}\" -l",
                "commandTemplate": "\"{shell}\" -File \"{tmpfile}\"",
                "tmpFileExt": ".ps1"
            }],
            "sftpCommand": "cmd.exe /c sftp-server.exe"
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.routes[0].default, false);
    }
}
```

- [ ] **Step 4: 验证 config crate 测试通过**

Run: `cargo test -p ssh-router-config`
Expected: 4 tests passed

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/config/
git commit -m "feat: add ssh-router-config crate with Config/Route structs"
```

---

### Task 2: 创建 CLI crate 骨架与日志模块

**Files:**
- Create: `crates/cli/Cargo.toml`
- Create: `crates/cli/src/main.rs`
- Create: `crates/cli/src/log.rs`

**Interfaces:**
- Consumes: `ssh-router-config` crate 的 `Config`、`Route`
- Produces: `ssh-router-cli` crate，`log::log(msg: &str)` 函数写入 `C:\ProgramData\ssh\ssh-router-debug.log`

- [ ] **Step 1: 创建 cli crate 的 Cargo.toml**

```toml
[package]
name = "ssh-router-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "ssh-router-cli"
path = "src/main.rs"

[dependencies]
ssh-router-config = { path = "../config" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
windows = { version = "0.61", features = [
    "Win32_System_JobObjects",
    "Win32_System_Threading",
    "Win32_System_ProcessThreadsApi",
    "Win32_Foundation",
    "Win32_System_Diagnostics_Debug",
    "Win32_Storage_FileSystem",
] }
```

- [ ] **Step 2: 创建 log.rs 模块**

```rust
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const LOG_FILE: &str = r"C:\ProgramData\ssh\ssh-router-debug.log";

/// 写一条日志到 ssh-router-debug.log，格式: "yyyy-MM-dd HH:mm:ss.fff <msg>"
/// 与原 C# Log 函数行为一致，失败时静默忽略
pub fn log(msg: &str) {
    let entry = format!(
        "{} {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        msg
    );
    let _ = write_entry(&entry);
}

fn write_entry(entry: &str) -> std::io::Result<()> {
    let path = Path::new(LOG_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    file.write_all(entry.as_bytes())?;
    Ok(())
}
```

- [ ] **Step 3: 创建 main.rs 骨架**

```rust
mod log;

fn main() {
    log::log("========");
    log::log("ssh-router-cli starting");
    // TODO: Task 3-5 将填充实际逻辑
    log::log("========");
}
```

- [ ] **Step 4: 验证 CLI 能编译（macOS 交叉编译检查语法）**

Run: `cargo check -p ssh-router-cli --target x86_64-pc-windows-msvc`
Expected: 编译通过（Win32 API 调用还未引入，chrono 在所有平台可用）

注意：`log.rs` 中的路径硬编码为 Windows 路径，在 macOS 上 `cargo check` 不会报错（只是字符串），但实际运行需要 Windows。`chrono` 是跨平台的，测试可通过。

- [ ] **Step 5: Commit**

```bash
git add crates/cli/
git commit -m "feat: add ssh-router-cli crate skeleton with log module"
```

---

### Task 3: 实现 ToWslPath 和临时文件清理

**Files:**
- Create: `crates/cli/src/wsl.rs`
- Modify: `crates/cli/src/main.rs`（添加 `mod wsl;`）
- Create: `crates/cli/src/temp.rs`

**Interfaces:**
- Consumes: 无
- Produces: `wsl::to_wsl_path(win: &str) -> String`、`temp::clean_stale_temp_files(pid: u32)`

- [ ] **Step 1: 创建 wsl.rs 模块（含测试）**

```rust
/// Windows 路径转 WSL 路径: C:\Users\xxx → /mnt/c/Users/xxx
/// 移植自 C# ToWslPath
pub fn to_wsl_path(win: &str) -> String {
    if win.len() >= 2 && win.as_bytes()[1] == b':' {
        let drive = win.as_bytes()[0].to_ascii_lowercase();
        let rest = &win[2..];
        let rest = rest.replace('\\', "/");
        return format!("/mnt/{}{}", drive as char, rest);
    }
    win.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_letter_path() {
        assert_eq!(
            to_wsl_path(r"C:\Users\test"),
            "/mnt/c/Users/test"
        );
    }

    #[test]
    fn test_lowercase_drive() {
        assert_eq!(
            to_wsl_path(r"D:\foo\bar"),
            "/mnt/d/foo/bar"
        );
    }

    #[test]
    fn test_no_drive_letter() {
        assert_eq!(
            to_wsl_path(r"foo\bar\baz"),
            "foo/bar/baz"
        );
    }

    #[test]
    fn test_already_slash() {
        assert_eq!(
            to_wsl_path(r"C:/Users/test"),
            "/mnt/c/Users/test"
        );
    }
}
```

- [ ] **Step 2: 创建 temp.rs 模块**

```rust
use std::fs;
use std::path::Path;

/// 启动期清理：删除之前进程残留的临时脚本（被强杀时不会执行 finally 的 File.Delete）
/// 文件以 PID 命名，排除当前 PID 即无跨进程竞态
/// 移植自 C# CleanStaleTempFiles
pub fn clean_stale_temp_files(pid: u32) {
    let tmp = std::env::temp_dir();
    let pid_prefix = format!("ssh-cmd-{}.", pid);

    for pattern in &["ssh-cmd-*.ps1", "ssh-cmd-*.sh"] {
        if let Ok(entries) = fs::read_dir(&tmp) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("ssh-cmd-") && !name.starts_with(&pid_prefix) {
                    let path = entry.path();
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_clean_excludes_current_pid() {
        let tmp = std::env::temp_dir();
        let current_file = tmp.join("ssh-cmd-99999.sh");
        let stale_file = tmp.join("ssh-cmd-88888.sh");

        fs::write(&current_file, "test").unwrap();
        fs::write(&stale_file, "test").unwrap();

        clean_stale_temp_files(99999);

        assert!(current_file.exists(), "current PID file should survive");
        assert!(!stale_file.exists(), "stale file should be deleted");

        // cleanup
        let _ = fs::remove_file(&current_file);
    }
}
```

- [ ] **Step 3: 更新 main.rs 添加模块声明**

```rust
mod log;
mod wsl;
mod temp;

fn main() {
    temp::clean_stale_temp_files(std::process::id());
    log::log("========");
    log::log("ssh-router-cli starting");
    log::log("========");
}
```

- [ ] **Step 4: 验证 wsl 和 temp 测试通过**

Run: `cargo test -p ssh-router-cli --lib wsl`
Expected: 4 wsl tests passed

Run: `cargo test -p ssh-router-cli --lib temp`
Expected: 1 temp test passed

注意：`temp` 模块的测试在 macOS 上也能通过（使用 `std::env::temp_dir()`）。

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/wsl.rs crates/cli/src/temp.rs crates/cli/src/main.rs
git commit -m "feat: add to_wsl_path and stale temp file cleanup to CLI"
```

---

### Task 4: 实现路由匹配和模板替换

**Files:**
- Create: `crates/cli/src/routing.rs`
- Modify: `crates/cli/src/main.rs`（添加 `mod routing;`）

**Interfaces:**
- Consumes: `ssh-router-config` 的 `Config`、`Route`
- Produces: `routing::match_route(config: &Config, port: &str) -> Option<&Route>`、`routing::render_template(template: &str, shell: &str, tmpfile: Option<&str>, tmpfile_wsl: Option<&str>) -> String`、`routing::is_sftp_command(command: &str) -> bool`

- [ ] **Step 1: 创建 routing.rs 模块（含测试）**

```rust
use ssh_router_config::{Config, Route};

/// 判断命令是否为 SFTP 子系统调用
/// 移植自 C# command.Contains("sftp-server")
pub fn is_sftp_command(command: &str) -> bool {
    command.contains("sftp-server")
}

/// 根据端口匹配路由规则
/// 优先级: 精确匹配 > default route
/// 多条 default 时取第一条（记 WARN 由调用方处理）
/// 移植自 C# port == "22" / "2222" / "2223" 判断逻辑
pub fn match_route<'a>(config: &'a Config, port: &str) -> Option<&'a Route> {
    // 精确端口匹配
    let port_num: u16 = port.parse().unwrap_or(0);
    for route in &config.routes {
        if route.port == port_num {
            return Some(route);
        }
    }
    // 回退到 default route
    config.routes.iter().find(|r| r.default)
}

/// 渲染命令模板，替换占位符
/// {shell} → route.shell
/// {tmpfile} → 临时文件 Windows 路径
/// {tmpfile_wsl} → 临时文件 WSL 路径
pub fn render_template(
    template: &str,
    shell: &str,
    tmpfile: Option<&str>,
    tmpfile_wsl: Option<&str>,
) -> String {
    let mut result = template.replace("{shell}", shell);
    if let Some(tf) = tmpfile {
        result = result.replace("{tmpfile}", tf);
    }
    if let Some(tfw) = tmpfile_wsl {
        result = result.replace("{tmpfile_wsl}", tfw);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ssh_router_config::Config;

    #[test]
    fn test_is_sftp_command() {
        assert!(is_sftp_command("C:\\Windows\\System32\\OpenSSH\\sftp-server.exe"));
        assert!(!is_sftp_command("uname -s"));
    }

    #[test]
    fn test_match_route_exact_port() {
        let config = Config::default_config();
        let route = match_route(&config, "2222").unwrap();
        assert_eq!(route.port, 2222);
        assert_eq!(route.name, "Git Bash");
    }

    #[test]
    fn test_match_route_fallback_default() {
        let config = Config::default_config();
        // 端口 9999 不在配置中，回退到 default (port 22)
        let route = match_route(&config, "9999").unwrap();
        assert_eq!(route.port, 22);
        assert_eq!(route.name, "PowerShell");
    }

    #[test]
    fn test_match_route_no_default_returns_none() {
        let mut config = Config::default_config();
        for route in &mut config.routes {
            route.default = false;
        }
        assert!(match_route(&config, "9999").is_none());
    }

    #[test]
    fn test_match_route_empty_port() {
        let config = Config::default_config();
        // 空端口字符串 parse 失败 → port_num=0 → 精确匹配失败 → 回退 default
        let route = match_route(&config, "").unwrap();
        assert_eq!(route.port, 22);
    }

    #[test]
    fn test_render_template_with_shell_only() {
        let result = render_template("\"{shell}\" -l", "pwsh.exe", None, None);
        assert_eq!(result, "\"pwsh.exe\" -l");
    }

    #[test]
    fn test_render_template_with_tmpfile() {
        let result = render_template(
            "\"{shell}\" -File \"{tmpfile}\"",
            "pwsh.exe",
            Some("C:\\tmp\\ssh-cmd-123.ps1"),
            None,
        );
        assert_eq!(result, "\"pwsh.exe\" -File \"C:\\tmp\\ssh-cmd-123.ps1\"");
    }

    #[test]
    fn test_render_template_with_tmpfile_wsl() {
        let result = render_template(
            "wsl.exe -- bash -c '. \"{tmpfile_wsl}\"'",
            "wsl.exe",
            None,
            Some("/mnt/c/tmp/ssh-cmd-123.sh"),
        );
        assert_eq!(result, "wsl.exe -- bash -c '. \"/mnt/c/tmp/ssh-cmd-123.sh\"'");
    }

    #[test]
    fn test_render_template_all_placeholders() {
        let result = render_template(
            "{shell} {tmpfile} {tmpfile_wsl}",
            "bash",
            Some("C:\\f.sh"),
            Some("/mnt/c/f.sh"),
        );
        assert_eq!(result, "bash C:\\f.sh /mnt/c/f.sh");
    }
}
```

- [ ] **Step 2: 更新 main.rs 添加模块声明**

在 main.rs 顶部添加：
```rust
mod routing;
```

- [ ] **Step 3: 验证 routing 测试通过**

Run: `cargo test -p ssh-router-cli --lib routing`
Expected: 8 routing tests passed

- [ ] **Step 4: Commit**

```bash
git add crates/cli/src/routing.rs crates/cli/src/main.rs
git commit -m "feat: add route matching and template rendering to CLI"
```

---

### Task 5: 实现 Win32 API 封装（CreateProcessW + Job Object）

**Files:**
- Create: `crates/cli/src/win32.rs`
- Modify: `crates/cli/src/main.rs`（添加 `mod win32;`）

**Interfaces:**
- Consumes: 无（直接调用 windows crate）
- Produces: `win32::create_kill_on_close_job() -> Option<isize>`、`win32::launch_and_wait(cmd_line: &str, job: isize, log: &dyn Fn(&str)) -> u32`

**注意：此 Task 只能在 Windows 上编译和测试。macOS 上 `cargo check --target x86_64-pc-windows-msvc` 只能做语法检查，无法运行。**

- [ ] **Step 1: 创建 win32.rs 模块**

```rust
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows::Win32::System::Diagnostics::Debug::FormatMessageW;
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::ProcessThreadsApi::GetExitCodeProcess;
use windows::Win32::System::Threading::{
    CreateProcessW, ResumeThread,
    WaitForSingleObject, CREATE_SUSPENDED, INFINITE, PROCESS_INFORMATION, STARTUPINFOW,
    WAIT_FAILED,
};

/// 将 &str 转为 UTF-16 null-terminated Vec<u16>
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// 获取 Win32 错误消息文本
fn last_error_message() -> String {
    unsafe {
        let err = GetLastError();
        let mut buf = [0u16; 512];
        let len = FormatMessageW(
            windows::Win32::System::Diagnostics::Debug::FORMAT_MESSAGE_FROM_SYSTEM
                | windows::Win32::System::Diagnostics::Debug::FORMAT_MESSAGE_IGNORE_INSERTS,
            None,
            err.0,
            0,
            Some(&mut buf),
            ptr::null(),
        );
        if len == 0 {
            return format!("error {}", err.0);
        }
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

/// 创建 KILL_ON_JOB_CLOSE 的 Job Object
/// 移植自 C# CreateKillOnCloseJob
pub fn create_kill_on_close_job(log: &dyn Fn(&str)) -> Option<isize> {
    unsafe {
        let h_job = CreateJobObjectW(None, None);
        let h_job_ptr = h_job.0 as isize;
        if h_job_ptr == 0 {
            log(&format!(
                "WARN: CreateJobObject failed, last error: {}",
                last_error_message()
            ));
            return None;
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let size = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>();
        let result = SetInformationJobObject(
            h_job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            size as u32,
        );

        if result.is_err() {
            log(&format!(
                "WARN: SetInformationJobObject failed, last error: {}",
                last_error_message()
            ));
            let _ = CloseHandle(h_job);
            return None;
        }

        Some(h_job_ptr)
    }
}

/// 用 CreateProcessW 启动子进程并等待其退出
/// - CREATE_SUSPENDED 启动，关联 Job Object 后再 ResumeThread
/// - bInheritHandles=true，不设 STARTF_USESTDHANDLES，让子进程自动继承 stdin/stdout/stderr
/// 移植自 C# Main 中的进程启动逻辑
pub fn launch_and_wait(cmd_line: &str, job: isize, log: &dyn Fn(&str)) -> u32 {
    let cmd_wide = to_wide(cmd_line);

    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let ok = unsafe {
        CreateProcessW(
            None,
            windows::core::PCWSTR(cmd_wide.as_ptr()),
            None,
            None,
            true, // bInheritHandles
            CREATE_SUSPENDED,
            None,
            None,
            &si,
            &mut pi,
        )
    };

    if ok.is_err() {
        log(&format!(
            "ERROR: CreateProcess failed, last error: {}",
            last_error_message()
        ));
        return 1;
    }

    let mut exit_code: u32 = 1;
    let h_job = HANDLE(job as *const _);
    unsafe {
        // CREATE_SUSPENDED 启动后、恢复线程前关联 Job
        if AssignProcessToJobObject(h_job, pi.hProcess).is_err() {
            log(&format!(
                "WARN: AssignProcessToJobObject failed, last error: {}",
                last_error_message()
            ));
        }
        let _ = ResumeThread(pi.hThread);

        if WaitForSingleObject(pi.hProcess, INFINITE) == WAIT_FAILED {
            log(&format!(
                "WARN: WaitForSingleObject failed, last error: {}",
                last_error_message()
            ));
        }

        if GetExitCodeProcess(pi.hProcess, &mut exit_code).is_err() {
            log(&format!(
                "WARN: GetExitCodeProcess failed, last error: {}",
                last_error_message()
            ));
            exit_code = 1;
        }

        let _ = CloseHandle(pi.hProcess);
        let _ = CloseHandle(pi.hThread);
    }

    exit_code
}

/// 关闭 Job Object 句柄（在 finally 块中调用）
pub fn close_job(job: isize) {
    if job != 0 {
        unsafe {
            let _ = CloseHandle(HANDLE(job as *const _));
        }
    }
}
```

- [ ] **Step 2: 更新 main.rs 添加模块声明**

在 main.rs 顶部添加：
```rust
mod win32;
```

- [ ] **Step 3: 验证 Windows 交叉编译检查**

Run: `cargo check -p ssh-router-cli --target x86_64-pc-windows-msvc`
Expected: 编译通过（可能有 unused warning，因为 win32 模块还未被 main 调用）

注意：如果 `windows` crate 的 API 签名与上述代码不完全匹配（版本差异），需要查阅 `windows` crate 0.61 文档调整。关键 API：`CreateJobObjectW`、`SetInformationJobObject`、`CreateProcessW`、`AssignProcessToJobObject`、`ResumeThread`、`WaitForSingleObject`、`GetExitCodeProcess`、`CloseHandle`。

- [ ] **Step 4: Commit**

```bash
git add crates/cli/src/win32.rs crates/cli/src/main.rs
git commit -m "feat: add Win32 CreateProcessW + Job Object wrappers to CLI"
```

---

### Task 6: 完成 CLI main 函数（整合所有模块）

**Files:**
- Modify: `crates/cli/src/main.rs`（完整实现）

**Interfaces:**
- Consumes: `log`、`wsl`、`temp`、`routing`、`win32` 模块，`ssh-router-config` crate
- Produces: 完整的 `ssh-router-cli.exe`，可被 sshd 作为 DefaultShell 调用

- [ ] **Step 1: 完整实现 main.rs**

```rust
mod log;
mod wsl;
mod temp;
mod routing;
mod win32;

use std::env;
use std::fs;
use std::path::PathBuf;

const CONFIG_PATH: &str = r"C:\ProgramData\ssh\ssh-router.json";

fn main() {
    let pid = std::process::id();
    temp::clean_stale_temp_files(pid);

    let h_job = win32::create_kill_on_close_job(&log::log);

    // 解析 SSH_CONNECTION 获取端口
    let ssh_conn = env::var("SSH_CONNECTION").unwrap_or_default();
    let port = ssh_conn
        .split_whitespace()
        .nth(3)
        .unwrap_or("");

    // 解析命令行参数
    let args: Vec<String> = env::args().collect();
    let has_command = args.len() >= 2 && args[0] == "-c";
    let command = if has_command { Some(&args[1]) } else { None };

    // 记录调试日志
    log::log("========");
    log::log(&format!("args: {:?}", args));
    log::log(&format!("port: {}", port));
    if let Some(cmd) = &command {
        log::log(&format!("command: {}", cmd));
    }

    // 读取配置
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            log::log(&format!("ERROR: failed to load config: {}", e));
            win32::close_job(h_job.unwrap_or(0));
            std::process::exit(1);
        }
    };

    // 路由决策
    let cmd_line;
    let temp_file: Option<PathBuf>;

    if let Some(cmd) = command {
        if routing::is_sftp_command(cmd) {
            // SFTP 特殊处理
            cmd_line = config.sftp_command.clone();
            temp_file = None;
        } else {
            // 有命令：匹配端口 → commandTemplate
            let route = match routing::match_route(&config, port) {
                Some(r) => r,
                None => {
                    log::log("ERROR: no matching route and no default route");
                    win32::close_job(h_job.unwrap_or(0));
                    std::process::exit(1);
                }
            };

            // 写命令到临时文件
            let ext = &route.tmp_file_ext;
            let tmp_path = env::temp_dir().join(format!("ssh-cmd-{}{}", pid, ext));
            if let Err(e) = fs::write(&tmp_path, cmd) {
                log::log(&format!("ERROR: failed to create temp file: {}", e));
                win32::close_job(h_job.unwrap_or(0));
                std::process::exit(1);
            }

            let tmp_str = tmp_path.to_string_lossy().to_string();
            let tmp_wsl = wsl::to_wsl_path(&tmp_str);

            let needs_wsl = route.command_template.contains("{tmpfile_wsl}");
            let tmpfile_wsl = if needs_wsl { Some(tmp_wsl.as_str()) } else { None };

            cmd_line = routing::render_template(
                &route.command_template,
                &route.shell,
                Some(tmp_str.as_str()),
                tmpfile_wsl,
            );
            temp_file = Some(tmp_path);
        }
    } else {
        // 无命令（交互式）：匹配端口 → interactiveTemplate
        let route = match routing::match_route(&config, port) {
            Some(r) => r,
            None => {
                log::log("ERROR: no matching route and no default route");
                win32::close_job(h_job.unwrap_or(0));
                std::process::exit(1);
            }
        };

        cmd_line = routing::render_template(
            &route.interactive_template,
            &route.shell,
            None,
            None,
        );
        temp_file = None;
    }

    log::log(&format!("cmdLine: {}", cmd_line));

    // 启动子进程并等待
    let exit_code = win32::launch_and_wait(&cmd_line, h_job.unwrap_or(0), &log::log);

    // 清理临时文件
    if let Some(tf) = &temp_file {
        let _ = fs::remove_file(tf);
    }

    // 关闭 Job Object
    win32::close_job(h_job.unwrap_or(0));

    log::log(&format!("exit code: {}", exit_code));
    log::log("========");

    std::process::exit(exit_code as i32);
}

fn load_config() -> Result<ssh_router_config::Config, String> {
    let content = fs::read_to_string(CONFIG_PATH)
        .map_err(|e| format!("read config: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("parse config: {}", e))
}
```

- [ ] **Step 2: 验证交叉编译**

Run: `cargo check -p ssh-router-cli --target x86_64-pc-windows-msvc`
Expected: 编译通过

- [ ] **Step 3: 验证在 macOS 上非 Windows 部分测试仍通过**

Run: `cargo test -p ssh-router-cli --lib`
Expected: wsl (4) + routing (8) + temp (1) = 13 tests passed

- [ ] **Step 4: Commit**

```bash
git add crates/cli/src/main.rs
git commit -m "feat: complete CLI main function integrating all modules"
```

---

### Task 7: 初始化 Tauri v2 项目（前端 + 后端骨架）

**Files:**
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/App.tsx`（占位）
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`（占位）
- Create: `src-tauri/capabilities/default.json`
- Modify: `.gitignore`（添加 `node_modules/`、`dist/`、`src-tauri/target/`）

**注意：此 Task 需要在有网络连接的环境运行 `npm install`。**

- [ ] **Step 1: 初始化前端项目**

Run:
```bash
cd /Users/thinkinghuang/Source/ssh-router
npm create vite@latest . -- --template react-ts
```

如果提示目录非空，选择忽略已有文件。这会生成 `package.json`、`vite.config.ts`、`tsconfig.json`、`index.html`、`src/main.tsx`、`src/App.tsx`。

- [ ] **Step 2: 安装 Tauri v2 和前端依赖**

Run:
```bash
npm install
npm install -D @tauri-apps/cli
npm install @tauri-apps/api
```

- [ ] **Step 3: 添加 Tauri npm scripts 到 package.json**

修改 `package.json` 的 `scripts` 部分：

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  }
}
```

- [ ] **Step 4: 创建 src-tauri/Cargo.toml**

```toml
[package]
name = "ssh-router"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
ssh-router-config = { path = "../crates/config" }
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tauri-plugin-single-instance = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[features]
custom-protocol = ["tauri/custom-protocol"]
```

- [ ] **Step 5: 创建 src-tauri/build.rs**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 6: 创建 src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SSH Router",
  "version": "0.1.0",
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
        "height": 600,
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

- [ ] **Step 7: 创建 src-tauri/capabilities/default.json**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open"
  ]
}
```

- [ ] **Step 8: 创建 src-tauri/src/main.rs（占位）**

```rust
// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ssh_router_lib::run()
}
```

- [ ] **Step 9: 创建 lib.rs（Tauri 应用入口）**

修改 `src-tauri/Cargo.toml`，添加 `lib` target：

```toml
[lib]
name = "ssh_router_lib"
crate-type = ["staticlib", "cdylib", "rlib"]
```

创建 `src-tauri/src/lib.rs`：

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 10: 生成 Tauri 图标**

Run:
```bash
npm run tauri -- icon
```

这会生成默认图标到 `src-tauri/icons/`。如果没有图标源文件，用 Tauri 默认图标即可。

- [ ] **Step 11: 更新 .gitignore**

在现有 `.gitignore` 末尾追加：

```
# Node.js
node_modules/
dist/

# Tauri
src-tauri/target/
src-tauri/gen/
```

- [ ] **Step 12: 验证前端能构建**

Run: `npm run build`
Expected: Vite 构建成功，生成 `dist/` 目录

- [ ] **Step 13: 验证 Tauri Rust 后端能编译**

Run: `cargo check -p ssh-router`
Expected: 编译通过

注意：完整 Tauri 构建（`cargo tauri build`）只能在 Windows 上完成，macOS 上 `cargo check` 检查 Rust 代码语法即可。

- [ ] **Step 14: Commit**

```bash
git add package.json vite.config.ts tsconfig.json index.html src/ src-tauri/ .gitignore
git commit -m "feat: initialize Tauri v2 project with React frontend"
```

---

### Task 8: 添加 shadcn/ui 和 UI 组件

**Files:**
- Create: `components.json`
- Create: `src/lib/utils.ts`
- Create: `src/components/ui/*`（shadcn 生成）
- Create: `src/components/RouteTable.tsx`
- Create: `src/components/RouteDialog.tsx`
- Create: `src/components/SftpField.tsx`
- Create: `src/lib/api.ts`
- Modify: `src/App.tsx`
- Modify: `src/index.css`

- [ ] **Step 1: 安装 Tailwind CSS 和 shadcn/ui 依赖**

Run:
```bash
npm install -D tailwindcss @tailwindcss/vite
npm install class-variance-authority clsx tailwind-merge lucide-react
```

- [ ] **Step 2: 配置 Tailwind CSS**

修改 `vite.config.ts`：

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
```

修改 `tsconfig.json`，在 `compilerOptions` 中添加：

```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
```

修改 `src/index.css`（替换全部内容）：

```css
@import "tailwindcss";
```

- [ ] **Step 3: 创建 components.json（shadcn 配置）**

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "src/index.css",
    "baseColor": "neutral",
    "cssVariables": true
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "lib": "@/lib",
    "hooks": "@/hooks"
  }
}
```

- [ ] **Step 4: 创建 src/lib/utils.ts**

```typescript
import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

- [ ] **Step 5: 安装 shadcn 组件**

Run:
```bash
npx shadcn@latest init
npx shadcn@latest add table dialog input checkbox button label sonner
```

这会生成 `src/components/ui/` 下的组件文件。

- [ ] **Step 6: 创建 src/lib/api.ts（Tauri invoke 封装）**

```typescript
import { invoke } from "@tauri-apps/api/core"

export interface Route {
  port: number
  name: string
  shell: string
  interactiveTemplate: string
  commandTemplate: string
  tmpFileExt: string
  default: boolean
}

export interface Config {
  routes: Route[]
  sftpCommand: string
}

export async function loadConfig(): Promise<Config> {
  return invoke<Config>("load_config")
}

export async function saveConfig(config: Config): Promise<void> {
  await invoke("save_config", { config })
}

export async function createDefaultConfig(): Promise<Config> {
  return invoke<Config>("create_default_config")
}
```

- [ ] **Step 7: 创建 src/components/RouteTable.tsx**

```tsx
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import type { Route } from "@/lib/api"

interface RouteTableProps {
  routes: Route[]
  onEdit: (index: number) => void
  onDelete: (index: number) => void
}

export function RouteTable({ routes, onEdit, onDelete }: RouteTableProps) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead className="w-20">端口</TableHead>
          <TableHead className="w-32">名称</TableHead>
          <TableHead>Shell 路径</TableHead>
          <TableHead className="w-20">默认</TableHead>
          <TableHead className="w-32">操作</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {routes.map((route, index) => (
          <TableRow key={index}>
            <TableCell>{route.port}</TableCell>
            <TableCell>{route.name}</TableCell>
            <TableCell className="font-mono text-sm">{route.shell}</TableCell>
            <TableCell>
              <Checkbox checked={route.default} disabled />
            </TableCell>
            <TableCell>
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={() => onEdit(index)}>
                  编辑
                </Button>
                <Button variant="outline" size="sm" onClick={() => onDelete(index)}>
                  删除
                </Button>
              </div>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
```

- [ ] **Step 8: 创建 src/components/RouteDialog.tsx**

```tsx
import { useState, useEffect } from "react"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Checkbox } from "@/components/ui/checkbox"
import { Button } from "@/components/ui/button"
import type { Route } from "@/lib/api"

interface RouteDialogProps {
  open: boolean
  route: Route | null
  onSave: (route: Route) => void
  onClose: () => void
}

const emptyRoute: Route = {
  port: 0,
  name: "",
  shell: "",
  interactiveTemplate: "",
  commandTemplate: "",
  tmpFileExt: ".sh",
  default: false,
}

export function RouteDialog({ open, route, onSave, onClose }: RouteDialogProps) {
  const [form, setForm] = useState<Route>(emptyRoute)

  useEffect(() => {
    setForm(route ?? emptyRoute)
  }, [route, open])

  const handleChange = (field: keyof Route, value: string | number | boolean) => {
    setForm(prev => ({ ...prev, [field]: value }))
  }

  const handleSave = () => {
    onSave(form)
    onClose()
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{route ? "编辑路由" : "添加路由"}</DialogTitle>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label htmlFor="port">端口</Label>
              <Input
                id="port"
                type="number"
                value={form.port || ""}
                onChange={e => handleChange("port", parseInt(e.target.value) || 0)}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="name">名称</Label>
              <Input
                id="name"
                value={form.name}
                onChange={e => handleChange("name", e.target.value)}
              />
            </div>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="shell">Shell 路径</Label>
            <Input
              id="shell"
              value={form.shell}
              onChange={e => handleChange("shell", e.target.value)}
              placeholder="C:\Program Files\PowerShell\7\pwsh.exe"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="interactiveTemplate">交互式模板</Label>
            <Input
              id="interactiveTemplate"
              value={form.interactiveTemplate}
              onChange={e => handleChange("interactiveTemplate", e.target.value)}
              placeholder="&quot;{shell}&quot; -l"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="commandTemplate">命令模板</Label>
            <Input
              id="commandTemplate"
              value={form.commandTemplate}
              onChange={e => handleChange("commandTemplate", e.target.value)}
              placeholder="&quot;{shell}&quot; -File &quot;{tmpfile}&quot;"
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label htmlFor="tmpFileExt">临时文件扩展名</Label>
              <Input
                id="tmpFileExt"
                value={form.tmpFileExt}
                onChange={e => handleChange("tmpFileExt", e.target.value)}
                placeholder=".sh"
              />
            </div>
            <div className="grid gap-2 items-end">
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="default"
                  checked={form.default}
                  onCheckedChange={checked => handleChange("default", checked === true)}
                />
                <Label htmlFor="default">设为默认路由</Label>
              </div>
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>取消</Button>
          <Button onClick={handleSave}>保存</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 9: 创建 src/components/SftpField.tsx**

```tsx
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

interface SftpFieldProps {
  value: string
  onChange: (value: string) => void
}

export function SftpField({ value, onChange }: SftpFieldProps) {
  return (
    <div className="grid gap-2">
      <Label htmlFor="sftpCommand">SFTP 命令</Label>
      <Input
        id="sftpCommand"
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder="cmd.exe /c sftp-server.exe"
      />
    </div>
  )
}
```

- [ ] **Step 10: 创建 src/App.tsx（主界面）**

```tsx
import { useState, useEffect } from "react"
import { Toaster } from "@/components/ui/sonner"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { RouteTable } from "@/components/RouteTable"
import { RouteDialog } from "@/components/RouteDialog"
import { SftpField } from "@/components/SftpField"
import { loadConfig, saveConfig, createDefaultConfig, type Config, type Route } from "@/lib/api"

function App() {
  const [config, setConfig] = useState<Config | null>(null)
  const [sftpCommand, setSftpCommand] = useState("")
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editingIndex, setEditingIndex] = useState<number | null>(null)

  useEffect(() => {
    loadConfig()
      .then(cfg => {
        setConfig(cfg)
        setSftpCommand(cfg.sftpCommand)
      })
      .catch(err => {
        toast.error("加载配置失败", { description: String(err) })
      })
  }, [])

  const routes = config?.routes ?? []

  const handleAdd = () => {
    setEditingIndex(null)
    setDialogOpen(true)
  }

  const handleEdit = (index: number) => {
    setEditingIndex(index)
    setDialogOpen(true)
  }

  const handleDelete = (index: number) => {
    if (!config) return
    const newRoutes = routes.filter((_, i) => i !== index)
    setConfig({ ...config, routes: newRoutes })
  }

  const handleSaveRoute = (route: Route) => {
    if (!config) return
    const newRoutes = [...routes]
    // 如果设为默认，取消其他默认
    let finalRoutes = newRoutes
    if (route.default) {
      finalRoutes = newRoutes.map(r => ({ ...r, default: false }))
    }
    if (editingIndex !== null) {
      finalRoutes[editingIndex] = route
    } else {
      finalRoutes.push(route)
    }
    setConfig({ ...config, routes: finalRoutes })
  }

  const handleSave = () => {
    if (!config) return
    const finalConfig = { ...config, sftpCommand }
    // 校验恰好一条 default
    const defaults = finalConfig.routes.filter(r => r.default)
    if (defaults.length === 0) {
      toast.error("保存失败", { description: "必须有一条默认路由" })
      return
    }
    if (defaults.length > 1) {
      toast.error("保存失败", { description: "只能有一条默认路由" })
      return
    }
    saveConfig(finalConfig)
      .then(() => toast.success("配置已保存"))
      .catch(err => toast.error("保存失败", { description: String(err) }))
  }

  const handleCreateDefault = () => {
    createDefaultConfig()
      .then(cfg => {
        setConfig(cfg)
        setSftpCommand(cfg.sftpCommand)
        toast.success("已创建默认配置")
      })
      .catch(err => toast.error("创建默认配置失败", { description: String(err) }))
  }

  if (!config) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center">
          <p className="mb-4 text-muted-foreground">配置文件不存在或损坏</p>
          <Button onClick={handleCreateDefault}>创建默认配置</Button>
        </div>
      </div>
    )
  }

  return (
    <div className="container mx-auto p-6">
      <Toaster />
      <h1 className="text-2xl font-bold mb-6">SSH Router 配置</h1>

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
}

export default App
```

- [ ] **Step 11: 验证前端构建**

Run: `npm run build`
Expected: TypeScript + Vite 构建成功

- [ ] **Step 12: Commit**

```bash
git add components.json src/ vite.config.ts tsconfig.json
git commit -m "feat: add React + shadcn/ui frontend with route management UI"
```

---

### Task 9: 实现 Tauri 后端 commands（文件 I/O + 托盘 + 单例）

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/Cargo.toml`（添加 windows crate 依赖用于单例互斥锁）
- Modify: `src-tauri/tauri.conf.json`（托盘配置）

**Interfaces:**
- Consumes: `ssh-router-config` crate
- Produces: Tauri 应用，提供 `load_config`、`save_config`、`create_default_config` commands，系统托盘，单例运行

- [ ] **Step 1: 创建 src-tauri/src/commands.rs**

```rust
use ssh_router_config::Config;
use std::fs;
use std::path::Path;

const CONFIG_PATH: &str = r"C:\ProgramData\ssh\ssh-router.json";

#[tauri::command]
pub fn load_config() -> Result<Config, String> {
    let content = fs::read_to_string(CONFIG_PATH)
        .map_err(|e| format!("read config: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("parse config: {}", e))
}

#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    // 校验恰好一条 default
    let defaults: Vec<_> = config.routes.iter().filter(|r| r.default).collect();
    if defaults.len() != 1 {
        return Err(format!(
            "必须恰好有一条默认路由，当前有 {} 条",
            defaults.len()
        ));
    }
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("serialize config: {}", e))?;
    fs::write(CONFIG_PATH, json)
        .map_err(|e| format!("write config: {}", e))
}

#[tauri::command]
pub fn create_default_config() -> Result<Config, String> {
    let config = Config::default_config();
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("serialize config: {}", e))?;
    // 确保目录存在
    if let Some(parent) = Path::new(CONFIG_PATH).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create dir: {}", e))?;
    }
    fs::write(CONFIG_PATH, json)
        .map_err(|e| format!("write config: {}", e))?;
    Ok(config)
}
```

- [ ] **Step 2: 实现 lib.rs（托盘 + 单例 + commands 注册）**

```rust
mod commands;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 已有实例运行时，激活已有窗口
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::create_default_config,
        ])
        .setup(|app| {
            // 创建托盘菜单
            let open_item = MenuItem::with_id(app, "open", "打开主界面", true, None::<&tauri::Image>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&tauri::Image>)?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            // 创建托盘图标
            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("SSH Router")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "open" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // 点击关闭按钮时隐藏窗口而不是退出
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: 更新 src-tauri/Cargo.toml 添加 single-instance 插件**

在 `[dependencies]` 中添加：

```toml
tauri-plugin-single-instance = "2"
```

注意：不再需要 windows crate 依赖（单例逻辑由插件处理）。

- [ ] **Step 4: 验证 Tauri 后端编译**

Run: `cargo check -p ssh-router`
Expected: 编译通过

注意：`MenuItem::with_id` 和 `TrayIconBuilder` 的 API 签名可能因 Tauri 版本略有差异，需查阅 Tauri v2 文档调整。最后一个参数是 `accelerator`，用 `None` 表示不设快捷键。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands.rs src-tauri/Cargo.toml
git commit -m "feat: add Tauri commands, system tray, and single-instance support"
```

---

### Task 10: 更新构建脚本和文档

**Files:**
- Modify: `build.sh`
- Modify: `build.cmd`
- Modify: `README.md`
- Modify: `.gitignore`（补充）

**Interfaces:**
- Produces: 更新后的构建脚本和文档

- [ ] **Step 1: 更新 build.sh**

```bash
#!/bin/bash
# Build script for SSH Router (Tauri v2 + Rust CLI)
#
# 双程序构建:
# - ssh-router-cli.exe: 被 sshd 调起的路由 CLI (Rust + windows crate)
# - ssh-router.exe:     托盘 GUI (Tauri v2 + React + shadcn/ui)
#
# 注意: Tauri 不建议交叉编译, 此脚本应在 Windows 上运行
# macOS 上只能构建 CLI (cargo check --target x86_64-pc-windows-msvc)
#
# 用法 (Windows):
#   ./build.sh              编译到 ./publish/
#   ./build.sh /path/to/dir 编译到指定目录

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${1:-$SCRIPT_DIR/publish}"

echo "Building ssh-router-cli.exe..."
cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc

echo "Building ssh-router.exe (Tauri GUI)..."
npm install
npm run build
cargo tauri build

# 汇集产物
mkdir -p "$OUT_DIR"
cp "$SCRIPT_DIR/target/x86_64-pc-windows-msvc/release/ssh-router-cli.exe" "$OUT_DIR/"
cp "$SCRIPT_DIR/src-tauri/target/release/ssh-router.exe" "$OUT_DIR/"

echo "Build succeeded:"
echo "  $OUT_DIR/ssh-router-cli.exe"
echo "  $OUT_DIR/ssh-router.exe"
```

- [ ] **Step 2: 更新 build.cmd**

```cmd
@echo off
REM Build script for SSH Router (Tauri v2 + Rust CLI)
REM
REM 双程序构建:
REM - ssh-router-cli.exe: 被 sshd 调起的路由 CLI
REM - ssh-router.exe:     托盘 GUI (Tauri v2 + React)
REM
REM 用法:
REM   build.cmd                 编译到 .\publish\
REM   build.cmd "D:\deploy"     编译到指定目录

set OUT_DIR=%~1
if "%OUT_DIR%"=="" set OUT_DIR=%~dp0publish

echo Building ssh-router-cli.exe...
cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc
if %ERRORLEVEL% neq 0 (
    echo CLI build failed
    exit /b %ERRORLEVEL%
)

echo Building ssh-router.exe (Tauri GUI)...
call npm install
call npm run build
cargo tauri build
if %ERRORLEVEL% neq 0 (
    echo Tauri build failed
    exit /b %ERRORLEVEL%
)

REM 汇集产物
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"
copy "%~dp0target\x86_64-pc-windows-msvc\release\ssh-router-cli.exe" "%OUT_DIR%\"
copy "%~dp0src-tauri\target\release\ssh-router.exe" "%OUT_DIR%\"

echo Build succeeded:
echo   %OUT_DIR%\ssh-router-cli.exe
echo   %OUT_DIR%\ssh-router.exe
```

- [ ] **Step 3: 更新 README.md**

将 README.md 的内容替换为更新后的版本（保留原有的方案对比踩坑历史，更新构建、安装、自定义路由章节）。关键变更：

- 标题保持 "SSH Router"
- 新增"架构"章节说明双程序方案
- "编译"章节改为 Rust + npm
- "安装"章节增加 GUI 程序安装步骤
- "自定义路由"章节改为"用 GUI 配置，无需改源码"
- "已知限制"中删除"端口号硬编码"

```markdown
# SSH Router

Windows OpenSSH 多端口智能路由器，带可视化配置界面。

通过读取 `SSH_CONNECTION` 环境变量中的服务端监听端口，将不同 SSH 端口的连接路由到不同的 shell 环境。通过托盘 GUI 可视化管理端口路由，无需改源码重新编译。

## 架构

双程序方案：

- **ssh-router-cli.exe**：被 sshd 作为 `DefaultShell` 调起的路由 CLI（Rust），每次 SSH 连接时启动，根据配置文件路由到对应 shell
- **ssh-router.exe**：Tauri v2 托盘 GUI，常驻系统托盘，可视化配置端口路由

两者通过 `C:\ProgramData\ssh\ssh-router.json` 配置文件解耦。

（保留原有的"解决的问题"和"方案对比与踩过的坑"章节）

## 端口路由

默认配置（可通过 GUI 修改）：

| 端口 | 目标 Shell | 用途 |
|------|-----------|------|
| 22   | PowerShell 7 | Windows 管理（默认） |
| 2222 | Git Bash | Codex Remote SSH |
| 2223 | WSL Ubuntu | zcode / 通用 Linux 开发 |

## 编译

需要 Rust + Node.js + Tauri CLI。

### 前置条件

- Rust toolchain（`rustup target add x86_64-pc-windows-msvc`）
- Node.js 18+
- Tauri CLI（`cargo install tauri-cli --version "^2.0"`）

### 构建两个 exe

```bash
./build.sh
```

产物在 `publish/` 下：
- `ssh-router-cli.exe`：路由程序
- `ssh-router.exe`：托盘 GUI

## 安装

### 1. 部署 exe

将两个 exe 复制到 `C:\ProgramData\ssh\`。

### 2. 设置 DefaultShell

```powershell
Set-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name "DefaultShell" -Value "C:\ProgramData\ssh\ssh-router-cli.exe"
```

### 3. 配置 sshd_config

（保留原有 sshd_config 示例）

### 4. 首次运行 GUI

以管理员身份运行 `ssh-router.exe`，会自动创建默认配置文件 `ssh-router.json`。

### 5. 重启 sshd

```powershell
Restart-Service sshd -Force
```

## 自定义路由

运行 `ssh-router.exe`，在主界面中：
- 添加/编辑/删除端口路由
- 设置默认路由（未匹配端口时回退）
- 配置 SFTP 命令

保存后立即生效，下次 SSH 连接即使用新配置，无需重启 sshd 或重新编译。

## 已知限制

- WSL 发行版名称和 home 路径写在命令模板中（非独立字段）
- 不支持 ForceCommand
- SFTP 通过 cmd.exe /c 执行，读取 Windows 文件系统
- Tauri GUI 需在 Windows 上原生构建，不支持交叉编译

## 排错

（保留原有排错章节，更新日志路径说明）
```

- [ ] **Step 4: Commit**

```bash
git add build.sh build.cmd README.md
git commit -m "docs: update build scripts and README for Tauri v2 dual-program architecture"
```

---

### Task 11: 整体验证和交叉编译检查

**Files:**
- 无新文件，仅验证

- [ ] **Step 1: 验证所有 Rust 测试通过**

Run: `cargo test --workspace`
Expected: config crate (4 tests) + cli crate (13 tests) = 17 tests passed

- [ ] **Step 2: 验证 CLI 交叉编译**

Run: `cargo check -p ssh-router-cli --target x86_64-pc-windows-msvc`
Expected: 编译通过

- [ ] **Step 3: 验证 Tauri 后端编译**

Run: `cargo check -p ssh-router`
Expected: 编译通过

- [ ] **Step 4: 验证前端构建**

Run: `npm run build`
Expected: Vite 构建成功

- [ ] **Step 5: 提交最终状态（如有未提交的变更）**

```bash
git status
# 如有变更:
git add -A
git commit -m "chore: final verification of workspace build"
```

- [ ] **Step 6: 说明 Windows 构建限制**

在 macOS 上无法完成的步骤（需要在 Windows 上执行）：

1. `cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc`（实际生成 exe，不只是 check）
2. `cargo tauri build`（生成 GUI exe，含前端打包）
3. 实际 SSH 连接测试（需要 sshd + ssh-router-cli.exe 部署）
4. GUI 功能测试（需要 Windows 桌面环境）

建议：在 Windows 机器上或 GitHub Actions Windows runner 上完成最终构建和测试。
