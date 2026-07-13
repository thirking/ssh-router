# SSH Router 托盘 GUI + 配置化路由设计

## 背景与目标

当前 `SshRouter.cs` 是 Windows OpenSSH 的 `DefaultShell`，被 sshd 每次 SSH 连接通过 `CreateProcessW` 调起，根据 `SSH_CONNECTION` 环境变量中的端口将连接路由到不同 shell（PowerShell / Git Bash / WSL）。

**现状问题**：端口→shell 的映射硬编码在源码中，修改路由需要编辑源码 + 重新编译 + 替换 exe。

**目标**：
1. 可视化配置 SSH 端口号
2. 动态修改路由 shell，无需改源码重新编译

## 决策清单

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 程序架构 | 双程序：托盘 GUI + 路由 CLI | 路由 CLI 保持毫秒级启动；GUI 常驻不影响 SSH 连接性能 |
| 技术栈 | Tauri v2（GUI）+ Rust（CLI） | Tauri 内置系统托盘、Windows 成熟；Rust 重写 CLI 统一技术栈 |
| 前端 | React + shadcn/ui | 表格表单组件齐全，适合配置管理界面 |
| 配置粒度 | 端口 + shell 路径 + 命令模板 | 兼顾灵活性和可维护性 |
| 配置位置 | `C:\ProgramData\ssh\ssh-router.json` | 与 sshd、CLI 同目录，集中管理 |
| 配置共享 | Cargo workspace 共享 crate | 避免结构定义重复 |

## 整体架构

### 两个程序

**① `ssh-router.exe`（Tauri v2 桌面程序，常驻托盘）**
- 用户双击或开机自启运行，常驻系统托盘
- 托盘菜单：打开主界面、退出
- 主界面：表格化展示路由配置，支持增删改端口→shell 映射
- 读写 `C:\ProgramData\ssh\ssh-router.json`
- 写配置时若权限不足，提示需要管理员权限
- 不参与 SSH 路由，纯配置管理

**② `ssh-router-cli.exe`（纯 Rust CLI，被 sshd 调起）**
- 作为 sshd 的 `DefaultShell`，每次 SSH 连接被 `CreateProcessW` 调起
- 启动时读 `ssh-router.json`，根据端口匹配路由
- 移植现有 C# 的核心逻辑：`CreateProcessW` + Job Object + 临时文件传递命令 + SFTP 特殊处理
- 极轻量、毫秒级启动→执行→退出

### 数据流

```
SSH 客户端 → sshd → CreateProcessW(ssh-router-cli.exe -c "command")
                                ↓
                    ssh-router-cli.exe 读取 ssh-router.json
                                ↓
                    匹配端口 → 生成命令 → CreateProcessW 启动 shell
                                ↓
                    shell 继承 stdin/stdout（Job Object 管控）

ssh-router.exe (托盘 GUI) ←→ ssh-router.json ←→ ssh-router-cli.exe
         (读写配置)              (共享配置)        (只读配置)
```

## 配置文件数据结构

文件位置：`C:\ProgramData\ssh\ssh-router.json`

### 结构

```json
{
  "routes": [
    {
      "port": 22,
      "name": "PowerShell",
      "shell": "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
      "interactiveTemplate": "\"{shell}\" -l",
      "commandTemplate": "\"{shell}\" -File \"{tmpfile}\"",
      "tmpFileExt": ".ps1",
      "default": true
    },
    {
      "port": 2222,
      "name": "Git Bash",
      "shell": "C:\\Program Files\\Git\\usr\\bin\\bash.exe",
      "interactiveTemplate": "\"{shell}\" -l",
      "commandTemplate": "\"{shell}\" -l -c '. \"{tmpfile}\"'",
      "tmpFileExt": ".sh",
      "default": false
    },
    {
      "port": 2223,
      "name": "WSL Ubuntu",
      "shell": "wsl.exe",
      "interactiveTemplate": "wsl.exe -d Ubuntu -- bash -lc 'cd /home/thirking && exec bash -l'",
      "commandTemplate": "wsl.exe -d Ubuntu -- bash -c 'cd /home/thirking && . \"{tmpfile_wsl}\"'",
      "tmpFileExt": ".sh",
      "default": false
    }
  ],
  "sftpCommand": "cmd.exe /c \"C:\\Windows\\System32\\OpenSSH\\sftp-server.exe\""
}
```

### 字段说明

| 字段 | 说明 |
|------|------|
| `port` | SSH 监听端口（来自 `SSH_CONNECTION` 第 4 段） |
| `name` | 显示名（GUI 表格用） |
| `shell` | shell 可执行路径（主要供 GUI 展示） |
| `interactiveTemplate` | 无命令（交互式）时的启动命令模板 |
| `commandTemplate` | 有命令（`-c`）时的启动命令模板 |
| `tmpFileExt` | 临时文件扩展名（`.sh` / `.ps1`） |
| `default` | 是否为默认 route（未匹配端口时回退） |
| `sftpCommand` | SFTP 特殊处理的命令（全局，检测到命令含 `sftp-server` 时用） |

### 模板占位符

| 占位符 | 含义 |
|--------|------|
| `{shell}` | `shell` 字段的值 |
| `{tmpfile}` | 临时文件的 Windows 路径 |
| `{tmpfile_wsl}` | 临时文件的 WSL 路径（`/mnt/c/...` 转换后） |

### 默认配置内容

GUI 首次创建配置时，写入上述三端口路由（移植自现有 C# 硬编码）。

## 路由 CLI 内部逻辑（Rust 移植）

### 启动流程

```
1. 清理残留临时文件（CleanStaleTempFiles）
   - 遍历 %TEMP%，删除 ssh-cmd-<PID>.sh / .ps1
   - 排除当前 PID 的文件（无跨进程竞态）

2. 创建 Job Object（KILL_ON_JOB_CLOSE）
   - CreateJobObjectW → SetInformationJobObject

3. 解析环境
   - 读 SSH_CONNECTION，split 取第 4 段 = 端口
   - 读 args：args[0]=="-c" 则 args[1] 是命令

4. 读取 ssh-router.json
   - 反序列化为 Config 结构

5. 路由决策
   ├─ 命令含 "sftp-server" → 用 sftpCommand（全局）
   ├─ 有命令（-c）且非 sftp → 匹配 port → commandTemplate
   └─ 无命令（交互式）      → 匹配 port → interactiveTemplate

6. 生成临时文件（仅 commandTemplate 分支）
   - 写命令原文到 %TEMP%/ssh-cmd-<PID>.<ext>
   - 若模板含 {tmpfile_wsl}，额外做 ToWslPath 转换

7. CreateProcessW
   - CREATE_SUSPENDED + bInheritHandles=true
   - 不设 STARTF_USESTDHANDLES（自动继承父句柄）

8. AssignProcessToJobObject → ResumeThread

9. WaitForSingleObject(INFINITE) → GetExitCodeProcess

10. finally: 清理临时文件、CloseHandle
```

### Rust Win32 API 映射

| C# P/Invoke | Rust `windows` crate 等价 |
|-------------|--------------------------|
| `CreateJobObject` | `CreateJobObjectW` |
| `SetInformationJobObject` | `SetInformationJobObject` |
| `AssignProcessToJobObject` | `AssignProcessToJobObject` |
| `CreateProcess` | `CreateProcessW` |
| `ResumeThread` | `ResumeThread` |
| `WaitForSingleObject` | `WaitForSingleObject` |
| `GetExitCodeProcess` | `GetExitCodeProcess` |
| `CloseHandle` | `CloseHandle` |

结构体（`STARTUPINFOW`、`PROCESS_INFORMATION`、`JOBOBJECT_EXTENDED_LIMIT_INFORMATION` 等）在 `windows` crate 中都有对应类型，不用手写 `StructLayout`。

### ToWslPath 移植

```rust
// C:\Users\xxx → /mnt/c/Users/xxx
fn to_wsl_path(win: &str) -> String { ... }
```

逻辑同 C#：检测 `X:` 盘符，转 `/mnt/x`，反斜杠转正斜杠。

### 错误处理与日志

- 保留 `C:\ProgramData\ssh\ssh-router-debug.log` 日志机制
- 每个 Win32 调用失败时 `Log("WARN: ... last error: " + GetLastError())`
- JSON 读取失败 / 端口未匹配且无 default → 记日志、退出码 1

### CLI 依赖

```toml
[dependencies]
ssh-router-config = { path = "../config" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
windows = { version = "0.61", features = [
    "Win32_System_JobObjects",
    "Win32_System_Threading",
    "Win32_Foundation",
    "Win32_System_Diagnostics_Debug",
] }
```

## 模板占位符与路由匹配细节

### 模板占位符替换规则

CLI 读取 `commandTemplate` / `interactiveTemplate` 后做字符串替换：

| 占位符 | 替换为 | 何时替换 |
|--------|--------|---------|
| `{shell}` | `route.shell` 的值 | 两个模板都替换 |
| `{tmpfile}` | 临时文件 Windows 路径 | 仅 `commandTemplate` |
| `{tmpfile_wsl}` | 临时文件 WSL 路径 | 仅 `commandTemplate`，按需 |

替换顺序：`{shell}` → `{tmpfile}` → `{tmpfile_wsl}`。简单字符串替换，不做引号转义。

### 临时文件生成时机

只有 `commandTemplate` 分支（有命令、非 SFTP）才生成临时文件：

1. 写命令原文到 `%TEMP%\ssh-cmd-<PID>.<tmpFileExt>`
2. 模板含 `{tmpfile_wsl}` → 额外调用 `to_wsl_path` 生成 WSL 路径

### 路由匹配优先级

```
1. 命令含 "sftp-server" → 用全局 sftpCommand（不走路由表）
2. 端口精确匹配 route → 用该 route
3. 端口未匹配 → default route（default: true 的那条）
4. 无 default route → 记日志，退出码 1
```

### default route 约束

- `routes` 中 `default: true` 的至多一条
- 多条 `default: true` → CLI 取第一条，记 WARN 日志
- 零条 `default: true` → 未匹配端口的连接失败退出
- GUI 保存时校验：保证恰好一条 default，否则保存失败并提示

### JSON 读取容错

- 文件不存在 → CLI 记日志退出码 1；GUI 提示"配置文件不存在，是否创建默认配置"
- JSON 解析失败 → CLI 记日志退出码 1；GUI 提示"配置文件损坏"
- 字段缺失 → `default` 用 `#[serde(default)]`，其余字段缺失则解析失败（严格模式）

## 托盘 GUI（Tauri v2）

### 程序形态

- 常驻系统托盘，开机可选自启
- 启动时不弹主窗口，只在托盘显示图标
- 托盘菜单：打开主界面 / 退出

### 主界面功能

1. **表格展示**：列出所有 route（端口、名称、shell 路径、是否默认）
2. **添加/编辑/删除**：弹窗编辑 route
3. **SFTP 命令**：单独输入框（全局配置）
4. **保存**：写入 `ssh-router.json`
5. **默认标记**：单选，只有一个 route 能是 default

### 编辑表单字段

| 字段 | 说明 |
|------|------|
| 端口 | 数字输入 |
| 名称 | 文本（显示用） |
| Shell 路径 | 文本 + 文件选择按钮 |
| 交互式模板 | 文本（`interactiveTemplate`） |
| 命令模板 | 文本（`commandTemplate`） |
| 临时文件扩展名 | 文本（`.sh` / `.ps1`） |
| 默认 | 复选框（单选语义） |

### 权限处理

- 启动时检测写权限：尝试写测试文件，失败则提示
- 保存失败时引导：捕获写入异常，弹窗提示"需要管理员权限运行"
- 不自动提权：避免 UAC 弹窗打断使用，让用户自行决定以管理员身份重启

### 配置文件操作

通过 Tauri 的 Rust 后端（`#[tauri::command]`）执行文件 I/O：

```rust
#[tauri::command]
fn load_config() -> Result<Config, String> { ... }

#[tauri::command]
fn save_config(config: Config) -> Result<(), String> { ... }
```

前端（TypeScript）只调 `invoke("load_config")` / `invoke("save_config", { config })`，不直接碰文件系统。

### 托盘实现

Tauri v2 内置 `TrayIconBuilder`：

```rust
let tray = TrayIconBuilder::with_id("main")
    .icon(app.default_window_icon().unwrap().clone())
    .menu(&menu)
    .on_event(|tray, event| { /* 处理点击 */ })
    .build(app)?;
```

- 双击托盘图标 → 显示主窗口
- 右键 → 菜单（打开 / 退出）

### 前端技术栈

- React + shadcn/ui（基于 Radix UI 的组件库）
- 构建工具：Vite（Tauri v2 官方推荐的 React 模板就是 Vite）
- 状态管理：React 内置（useState/useEffect），界面简单不需要全局状态库
- 样式：shadcn/ui 自带 Tailwind CSS

### 用到的 shadcn 组件

| 组件 | 用途 |
|------|------|
| `Table` | 路由列表展示 |
| `Dialog` | 添加/编辑 route 表单弹窗 |
| `Input` | 端口、名称、模板等文本输入 |
| `Checkbox` | 默认 route 标记 |
| `Button` | 添加/删除/保存 |
| `Label` | 表单字段标签 |
| `Toast`（sonner） | 保存成功/失败提示 |

### 单例运行

- 用命名互斥锁（`windows` crate `CreateMutexW`）检测已有实例
- 若已运行，激活已有窗口后退出

## Cargo Workspace 组织

### Workspace 结构

```
ssh-router/                     # Cargo workspace 根
├── Cargo.toml                  # [workspace] 定义
├── crates/
│   ├── config/                 # 共享 crate: ssh-router-config
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs          # Config, Route 结构 + serde
│   └── cli/                    # 路由 CLI: ssh-router-cli
│       ├── Cargo.toml          # 依赖 config + windows crate
│       └── src/
│           └── main.rs         # Win32 API 路由逻辑
├── src-tauri/                  # 托盘 GUI (Tauri v2)
│   ├── Cargo.toml              # 依赖 config + tauri
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs             # Tauri 入口 + 托盘
│   │   └── commands.rs         # #[tauri::command] 文件 I/O
│   └── icons/
├── src/                        # React 前端
│   ├── App.tsx
│   ├── components/
│   │   ├── RouteTable.tsx
│   │   ├── RouteDialog.tsx
│   │   └── SftpField.tsx
│   ├── lib/
│   │   └── api.ts              # invoke 封装
│   └── main.tsx
├── components.json             # shadcn/ui 配置
├── package.json
└── vite.config.ts
```

### 根 Cargo.toml

```toml
[workspace]
members = ["crates/config", "crates/cli", "src-tauri"]
resolver = "2"
```

### 共享 crate：`ssh-router-config`

```toml
# crates/config/Cargo.toml
[package]
name = "ssh-router-config"
version = "0.1.0"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

```rust
// crates/config/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub routes: Vec<Route>,
    pub sftp_command: String,
}

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
```

`#[serde(rename_all = "camelCase")]` 确保 Rust snake_case 字段名与 JSON camelCase 键名对应（`interactive_template` ↔ `interactiveTemplate`）。

### 依赖关系

```
ssh-router-config  ←── ssh-router-cli（路由 CLI）
                  ←── src-tauri     （托盘 GUI）
```

- `ssh-router-cli` 依赖 `config` + `windows` crate
- `src-tauri` 依赖 `config` + `tauri`
- 两者互不依赖，通过 `config` crate 共享数据契约

### 构建产物

| 命令 | 产物 |
|------|------|
| `cargo build -p ssh-router-cli --release` | `ssh-router-cli.exe`（路由程序） |
| `cargo tauri build` | `ssh-router.exe`（托盘 GUI，含前端打包） |

## 构建、部署与迁移

### 构建流程

#### 路由 CLI

```bash
cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc
```

产物：`target/x86_64-pc-windows-msvc/release/ssh-router-cli.exe`

#### 托盘 GUI（含前端）

```bash
# 前端
npm install
npm run build          # Vite 构建 React → dist/

# Tauri 打包（自动包含前端）
cargo tauri build
```

产物：`src-tauri/target/release/ssh-router.exe`（含前端静态资源）

### 构建脚本

更新 `build.sh` / `build.cmd` 为 Rust 构建命令：

```bash
# build.sh
#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${1:-$SCRIPT_DIR/publish}"

# CLI
cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc
# GUI
(cd "$SCRIPT_DIR" && npm install && npm run build && cargo tauri build)

# 汇集产物
mkdir -p "$OUT_DIR"
cp "$SCRIPT_DIR/target/x86_64-pc-windows-msvc/release/ssh-router-cli.exe" "$OUT_DIR/"
cp "$SCRIPT_DIR/src-tauri/target/release/ssh-router.exe" "$OUT_DIR/"

echo "Build succeeded: $OUT_DIR/"
```

### 交叉编译说明

当前项目从 macOS 交叉编译到 Windows。Rust + Tauri 的交叉编译比 C# 复杂：

- **CLI**：macOS 交叉编译到 `x86_64-pc-windows-msvc` 需要 Windows 工具链（rustup target add + lld 链接器）。实际操作中建议在 Windows 上原生构建，或用 GitHub Actions Windows runner。
- **Tauri GUI**：Tauri 官方明确不建议交叉编译，推荐原生 Windows 构建。

**建议**：CI 用 GitHub Actions Windows runner 构建两个 exe，或直接在 Windows 机器上构建。

### 部署步骤

1. 将 `ssh-router-cli.exe` 和 `ssh-router.exe` 复制到 `C:\ProgramData\ssh\`
2. 设置 sshd DefaultShell 指向 CLI：
   ```powershell
   Set-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name "DefaultShell" -Value "C:\ProgramData\ssh\ssh-router-cli.exe"
   ```
3. 首次运行 `ssh-router.exe`（需管理员权限），创建默认 `ssh-router.json`
4. 可选：将 `ssh-router.exe` 加入开机启动项
5. sshd_config 不变（`Port 22 / 2222 / 2223` 保留）

### 从现有 C# 方案迁移

| 现有 | 迁移后 |
|------|--------|
| `SshRouter.exe`（C# 单 exe） | `ssh-router-cli.exe`（Rust） |
| 无 GUI，改路由要改源码 | `ssh-router.exe`（Tauri 托盘 GUI） |
| 端口硬编码 | `ssh-router.json` 配置驱动 |
| `SshRouter.cs` / `.csproj` | 保留作历史参考，不删除 |
| `build.cmd` / `build.sh` | 更新为 Rust 构建命令 |
| `fix-sshd.ps1` | 保留不变 |
| `README.md` | 更新文档 |

### README 更新要点

- 新增"托盘 GUI"使用说明
- 构建步骤改为 Rust + npm
- "自定义路由"章节改为"用 GUI 配置，无需改源码"
- "已知限制"中删除"端口号硬编码"（已解决）

### .gitignore 更新

新增：
```
node_modules/
dist/
src-tauri/target/
```

## 已知限制（迁移后）

- WSL 发行版名称和 home 路径写在命令模板中（非独立字段），修改需编辑模板
- 不支持 `ForceCommand`（sshd_config 中设置会与本程序冲突）
- SFTP 通过 `cmd.exe /c` 执行，读取 Windows 文件系统
- Tauri GUI 不建议交叉编译，需在 Windows 上原生构建
