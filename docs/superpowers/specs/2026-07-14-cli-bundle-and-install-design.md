# CLI 打包 + 一键安装设计

## 背景

当前 `ssh-router-cli.exe` 需要用户手动下载并放到 `C:\ProgramData\ssh\`，再手动设置注册表和重启 sshd。用户希望安装包自带 CLI，GUI 上提供一键安装和快捷操作。

## 决策清单

| 决策项 | 选择 | 理由 |
|--------|------|------|
| CLI 打包方式 | Tauri resource | 安装包自带，用户无需单独下载 |
| 权限处理 | 按需 UAC 提权 | ShellExecuteW runas → PowerShell 子进程，安全且按需 |
| 提权结果传递 | 状态文件 + 轮询 | UAC 子进程独立，无法直接获取返回值 |
| 快捷功能 | 安装 CLI、设置 DefaultShell、重启 sshd、安装状态检查 | 覆盖完整安装流程 |
| 状态检查权限 | 不需要提权 | 读文件/注册表/服务状态不需要管理员权限 |

## CLI 打包与 CI 调整

### Tauri resource 声明

在 `tauri.conf.json` 的 `bundle` 中添加：

```json
"resources": ["resources/ssh-router-cli.exe"]
```

安装后 GUI 通过 `app.path().resource_dir()` 读取 CLI 路径：

```rust
app.path().resource_dir()?.join("ssh-router-cli.exe")
```

### CI 调整

在 `release.yml` 中 `tauri-action` 之前插入：

```yaml
- name: Build CLI and copy to resources
  run: |
    cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc
    New-Item -ItemType Directory -Force -Path src-tauri/resources
    Copy-Item target/x86_64-pc-windows-msvc/release/ssh-router-cli.exe src-tauri/resources/
```

移除原来单独上传 CLI exe 的步骤（CLI 已在安装包内）。

`src-tauri/resources/` 加入 `.gitignore`（CI 生成，不提交）。

## 按需 UAC 提权机制

### 提权方式：ShellExecuteW runas → PowerShell 脚本

GUI 以普通用户运行。执行提权操作时：

1. GUI 后端生成一段 PowerShell 脚本（含具体操作命令）
2. 用 `ShellExecuteW` 以 `runas` 动词启动 `powershell.exe`，把脚本通过 `-File` 传入
3. 用户看到 UAC 弹窗，同意后 PowerShell 以管理员权限执行
4. PowerShell 执行完毕后退出

### 返回值传递

`ShellExecuteW runas` 启动的进程是独立的，GUI 无法直接等待或获取退出码。解决方案：写状态文件。

PowerShell wrapper 脚本执行实际操作后，把结果写入 `%TEMP%\ssh-router-action-result.json`：

```json
{ "success": true, "message": "CLI 已安装到 C:\\ProgramData\\ssh\\" }
```

GUI 启动 UAC 进程后，轮询等待该文件出现（最多 30 秒），读取结果后删除文件。超时则提示"操作可能仍在进行中，请检查状态"。

### 三条提权命令

**安装 CLI：**
```powershell
$src = "<resource_dir>\ssh-router-cli.exe"
$dst = "C:\ProgramData\ssh\ssh-router-cli.exe"
Copy-Item $src $dst -Force
```

**设置 DefaultShell：**
```powershell
Set-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name "DefaultShell" -Value "C:\ProgramData\ssh\ssh-router-cli.exe"
```

**重启 sshd：**
```powershell
Restart-Service sshd -Force
```

### Rust 封装：elevate.rs

```rust
/// 以管理员权限执行 PowerShell 脚本
/// 1. 写实际脚本到 %TEMP%\ssh-router-action.ps1
/// 2. 写 wrapper 脚本到 %TEMP%\ssh-router-wrapper.ps1（执行实际脚本 + 写结果 JSON）
/// 3. ShellExecuteW(runas) 启动 powershell.exe -ExecutionPolicy Bypass -File wrapper.ps1
/// 4. 轮询结果文件（最多 30 秒）
/// 5. 读取结果，删除临时文件，返回
pub fn run_elevated(script: &str) -> Result<String, String> { ... }
```

## 安装状态检查（无需提权）

### 检查项

| 检查项 | 检查方式 | 不需要提权 |
|--------|---------|:-----------:|
| CLI 已部署 | `C:\ProgramData\ssh\ssh-router-cli.exe` 文件存在 | ✓ |
| DefaultShell 已设置 | 读注册表 `HKLM:\SOFTWARE\OpenSSH\DefaultShell` | ✓ |
| 配置文件存在 | `C:\ProgramData\ssh\ssh-router.json` 文件存在 | ✓ |
| sshd 服务状态 | 查询 `sshd` 服务状态 | ✓ |

### Status 结构

```rust
pub struct Status {
    pub cli_deployed: bool,
    pub cli_path: String,
    pub default_shell_set: bool,
    pub default_shell_value: String,
    pub config_exists: bool,
    pub sshd_running: bool,
    pub sshd_status: String,  // "Running" / "Stopped" / "Not installed"
}
```

### 实现

- CLI 检查：`Path::exists()`
- 注册表读取：`windows` crate `RegGetValueW`（读 `HKLM` 不需要提权）
- 配置文件检查：`Path::exists()`
- sshd 状态：`OpenSCManager` → `OpenServiceW("sshd")` → `QueryServiceStatus`（失败则 "Not installed"）

## GUI 界面布局

在现有路由配置界面上方增加"安装状态"面板和"快捷操作"按钮区。

### 新增组件

| 组件 | 职责 |
|------|------|
| `StatusPanel` | 显示四项检查结果（绿勾/红叉 + 详情）+ 刷新按钮 |
| `QuickActions` | 三个操作按钮 + 执行中 loading 状态 |

### 交互流程

每个快捷操作按钮的执行流程：

1. 用户点击按钮
2. 按钮变为 loading 状态（禁用 + spinner）
3. 调用后端 Tauri command（触发 UAC 提权）
4. 后端等待 PowerShell 子进程完成（轮询结果文件，最多 30 秒）
5. 成功 → toast 成功提示 + 自动刷新状态面板
6. 失败/超时 → toast 错误提示
7. 按钮恢复可用

### 状态面板自动刷新时机

- 窗口显示时（从托盘双击打开）
- 快捷操作完成后
- 手动点击"刷新状态"按钮

## 新增 Tauri commands

```rust
#[tauri::command]
fn check_status() -> Result<Status, String> { ... }

#[tauri::command]
fn install_cli(app: AppHandle) -> Result<String, String> { ... }

#[tauri::command]
fn set_default_shell() -> Result<String, String> { ... }

#[tauri::command]
fn restart_sshd() -> Result<String, String> { ... }
```

## 前端 API

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

export async function checkStatus(): Promise<Status>
export async function installCli(): Promise<string>
export async function setDefaultShell(): Promise<string>
export async function restartSshd(): Promise<string>
```

## 文件变更清单

| 文件 | 变更 |
|------|------|
| `src-tauri/tauri.conf.json` | 添加 `resources`，窗口高度 600→750 |
| `src-tauri/Cargo.toml` | 添加 `windows` crate（Registry、Services、Shell） |
| `src-tauri/src/commands.rs` | 新增 `check_status`、`install_cli`、`set_default_shell`、`restart_sshd` |
| `src-tauri/src/elevate.rs` | 新建：`run_elevated` 封装 |
| `src-tauri/src/lib.rs` | 注册新 commands，添加 `mod elevate` |
| `src/lib/api.ts` | 新增 `Status` 接口 + 四个函数 |
| `src/components/StatusPanel.tsx` | 新建：状态面板 |
| `src/components/QuickActions.tsx` | 新建：快捷操作按钮 |
| `src/App.tsx` | 集成状态面板和快捷操作区 |
| `.github/workflows/release.yml` | CI 先编译 CLI 复制到 resources；移除单独上传 CLI |
| `.gitignore` | 添加 `src-tauri/resources/` |

## Cargo.toml 新增依赖

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_System_Registry",
    "Win32_System_Services",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Foundation",
    "Win32_System_Threading",
] }
```
