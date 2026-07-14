# SSH Router

Windows OpenSSH 多端口智能路由器，带可视化配置界面。

通过读取 `SSH_CONNECTION` 环境变量中的服务端监听端口，将不同 SSH 端口的连接路由到不同的 shell 环境。通过托盘 GUI 可视化管理端口路由，无需改源码重新编译。

## 架构

双程序方案：

- **ssh-router-cli.exe**：被 sshd 作为 `DefaultShell` 调起的路由 CLI（Rust），每次 SSH 连接时启动，根据配置文件路由到对应 shell
- **ssh-router.exe**：Tauri v2 托盘 GUI，常驻系统托盘，可视化配置端口路由

两者通过 `C:\ProgramData\ssh\ssh-router.json` 配置文件解耦。CLI 每次被 sshd 调起时读取该 JSON，因此 GUI 中保存配置后下一次 SSH 连接立即生效，无需重启 sshd 或重新编译。

### 代码布局

| 路径 | 说明 |
|------|------|
| `crates/config/` | 共享 `Config` / `Route` 结构与默认配置（被 CLI 与 GUI 共用） |
| `crates/cli/` | `ssh-router-cli.exe`：Win32 `CreateProcessW` + Job Object 路由实现 |
| `src-tauri/` | `ssh-router.exe`：Tauri v2 后端（托盘 + 单实例 + Tauri 命令 + UAC 提权） |
| `src/` | React + shadcn/ui 前端（路由管理 + 安装状态 + 快捷操作） |
| `fix-sshd.ps1` | 一次性 sshd 修复脚本（保留） |

## 解决的问题

在 Windows OpenSSH 上，`DefaultShell` 是全局设置，只能指定一个 shell。当需要不同端口连接到不同 shell 时（如 WSL、Git Bash、PowerShell），有以下困难：

### 方案对比与踩过的坑

| 方案 | 问题 |
|------|------|
| `ForceCommand` + `DefaultShell` | ForceCommand 的值会通过 DefaultShell 执行，路径被当作 shell 命令解析，导致冲突 |
| `ForceCommand` + `SSH_ORIGINAL_COMMAND` | Windows OpenSSH 的 bug：同一 SSH 连接后续 exec 请求不更新 `SSH_ORIGINAL_COMMAND`，只有第一次 exec 的命令被传递 |
| `.cmd` 脚本作为 `DefaultShell`（用 `%*`） | cmd.exe 的 `%*` 展开时会重新解析引号，破坏包含 `'\''` 转义的复杂命令（如 zcode deploy 阶段的命令） |
| `bash.exe`（WSL 入口）作为 `DefaultShell` | bash.exe 会展开命令中的 `$var`（即使单引号内也会展开），导致 preflight 命令的变量赋值丢失 |
| `bash -c '<escaped command>'` 单引号包裹 | 对多行脚本、here-document、嵌套 `sh -c` 等复杂结构会破坏语法，导致 Remote SSH 工具报 `syntax error: unexpected end of file` |
| Git Bash `bash.exe` 作为 `DefaultShell` | `uname -s` 返回 `MSYS_NT`，被 zcode 检测为 `win32` 而非 `linux` |
| C# `Process.Start` 启动子进程 | .NET runtime 对 stdin/stdout 管道的处理导致长时间运行的进程（如 zcode server）通信卡住 |

### 最终方案

用一个原生 `.exe` 作为 `DefaultShell`，满足以下要求（Rust 重写后行为与原 C# 版一致）：

1. **绕过 cmd.exe**：sshd 直接调用 `.exe`，`argv` 由 Windows CRT 解析，原始命令完整保留
2. **不用 `SSH_ORIGINAL_COMMAND`**：避免 Windows OpenSSH 同一连接后续 exec 不更新的 bug
3. **临时文件传递命令**：将命令原样写入临时 `.sh`/`.ps1` 文件，让 shell 直接执行文件，避免 shell quoting 破坏复杂脚本（多行、here-document、嵌套引号）
4. **`CreateProcessW` 替代 `Process.Start`**：直接用 Windows API 继承 stdin/stdout/stderr 句柄，避免 .NET runtime 对管道的缓冲
5. **Job Object**：确保子进程在父进程退出时被杀死，避免僵尸进程残留
6. **SFTP 特殊处理**：SFTP 子系统也通过 DefaultShell 执行，通过 `cmd.exe /c` 启动 `sftp-server.exe` 正确继承二进制管道

## 端口路由

默认配置（可通过 GUI 修改，定义在 `crates/config/src/lib.rs` 的 `Config::default_config`）：

| 端口 | 目标 Shell | 用途 |
|------|-----------|------|
| 22   | PowerShell 7 | Windows 管理（默认路由） |
| 2222 | Git Bash | Codex Remote SSH |
| 2223 | WSL Ubuntu | zcode / 通用 Linux 开发 |

## 工作原理

```
SSH 客户端 (zcode / Codex / ssh)
    │
    ▼
Windows OpenSSH sshd
    │  CreateProcessW(ssh-router-cli.exe, "ssh-router-cli.exe -c \"command\"")
    │  argv 由 Windows CRT 解析，原始命令完整保留
    ▼
ssh-router-cli.exe
    │  1. 从 argv[1] 获取原始命令（不经 cmd.exe 解析）
    │  2. 从 SSH_CONNECTION 获取服务端端口
    │  3. 读取 C:\ProgramData\ssh\ssh-router.json 匹配该端口的路由
    │  4. 将命令原样写入临时 .sh/.ps1 文件（不做任何转义）
    │  5. 用 CreateProcessW(bInheritHandles=true) 启动对应 shell 执行临时文件
    │     不设 STARTF_USESTDHANDLES，让句柄自动继承
    │  6. 将子进程加入 Job Object（KILL_ON_JOB_CLOSE）
    │  7. 子进程退出后清理临时文件
    ▼
wsl.exe / bash.exe / pwsh.exe / sftp-server.exe
```

### 关键设计决策

#### 为什么用 `CreateProcessW` 而不是 `Process.Start`？

.NET 的 `Process.Start`（即使 `UseShellExecute=false`）对 stdin/stdout 管道的处理方式与直接 `CreateProcessW` 不同。对于短时间运行的命令（如 `uname -s`）没有区别，但对于**长时间运行的双向通信进程**（如 zcode server），`Process.Start` 会导致通信卡住。

直接用 `CreateProcessW` 配合 `bInheritHandles=true`，不设 `STARTF_USESTDHANDLES`，让子进程完全继承父进程的 stdin/stdout/stderr 句柄，确保二进制数据（如 MessagePack 协议、SFTP 协议）正确传递。Rust 版通过 `windows` crate 直接调用同一 Win32 API，行为与原 C# 版一致。

#### 为什么用临时文件传递命令？

之前用 `bash -c '<escaped command>'` 单引号包裹，虽然能防止 `$var` 展开，但对复杂 Shell 脚本会破坏语法：

- 多行脚本中的换行被单引号包裹后可能被截断
- here-document（`<<EOF`）的 delimiter 被转义后不再是有效 delimiter
- 嵌套 `sh -c '...'` 的引号层级冲突
- Codex Desktop / Zed / Cursor 等工具发送的复杂脚本报 `syntax error: unexpected end of file`

将命令原样写入临时文件，让 `bash script.sh` 执行，完全避免 quoting 问题。`bash script.sh` 与 `bash -c 'command'` 行为一致：脚本内容不会被预展开，`$var` 只在执行时展开。

#### 为什么需要 Job Object？

当 SSH 连接断开时，sshd 会杀死 `ssh-router-cli.exe` 进程。但子进程（wsl.exe → bash → node）不会被自动杀死，导致：
- 旧 server 进程残留，占用内存
- SQLite 数据库锁未释放
- 新连接的 server 无法获取锁，整个 UI 卡住

Job Object 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，当 `ssh-router-cli.exe` 退出时（Job Object 句柄关闭），所有子进程自动被杀死。

#### 为什么 SFTP 用 `cmd.exe /c`？

SFTP 子系统（`Subsystem sftp sftp-server.exe`）也通过 DefaultShell 执行。`ssh-router-cli.exe` 收到 `-c "sftp-server.exe"` 后，需要直接启动 Windows 的 `sftp-server.exe`。

但 `CreateProcessW` 直接启动 `sftp-server.exe` 时 stdin 管道传递有问题（进程立即退出）。通过 `cmd.exe /c sftp-server.exe` 中间层，cmd.exe 正确继承并转发 stdin/stdout 的二进制 SFTP 协议数据。默认配置中 `sftpCommand` 即为 `cmd.exe /c "C:\Windows\System32\OpenSSH\sftp-server.exe"`。

#### 为什么拆成两个 exe？

`ssh-router-cli.exe` 由 sshd 在每次连接时 `CreateProcessW` 调起，必须做到「启动快、退出即清理」；而 GUI 需要常驻托盘、读写配置文件。两者职责不同，强行合一会让每次 SSH 连接都拉起一个 GUI 进程。拆分后：

- CLI 无 UI 依赖，启动开销小
- GUI 通过单实例锁保证只有一个 `ssh-router.exe` 在托盘
- 两者只通过 JSON 文件通信，GUI 改完配置 CLI 立即可读

## 编译

需要 Rust + Node.js + Tauri CLI。**Tauri 不建议交叉编译，以下命令应在 Windows 上运行**；macOS 上只能做 CLI 的类型检查：`cargo check --target x86_64-pc-windows-msvc`。

### 前置条件

- Rust toolchain（`rustup target add x86_64-pc-windows-msvc`）
- Node.js 18+
- Tauri CLI（`cargo install tauri-cli --version "^2.0"`）
- Windows 10/11 SDK（MSVC 链接器，随 Visual Studio Build Tools 提供）

### 构建两个 exe

```bash
./build.sh
# 或指定输出目录
./build.sh D:\deploy
```

Windows CMD 下：

```cmd
build.cmd
build.cmd "D:\deploy"
```

脚本会依次执行：

1. `cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc` → 产出 `ssh-router-cli.exe`
2. `npm install && npm run build && cargo tauri build` → 产出 `ssh-router.exe`

产物汇集到 `publish/`（或指定目录）下：

- `ssh-router-cli.exe`：路由程序，CI 会自动打包进安装包（作为 Tauri resource）
- `ssh-router.exe`：托盘 GUI，安装包内含 CLI

## 安装

### 1. 下载安装包

从 [GitHub Releases](https://github.com/thirking/ssh-router/releases) 下载 `setup.exe` 安装包，安装 SSH Router GUI。

安装包自带 `ssh-router-cli.exe`（打包为 Tauri resource），无需单独下载。

> 从 `v0.0.8` 起统一使用 NSIS `setup.exe`。`v0.0.7` 及更早版本需要手动安装 `v0.0.8` 一次；如果旧版通过 MSI 安装，建议先在 Windows“已安装的应用”中卸载旧 GUI。卸载 GUI 不会删除 `C:\ProgramData\ssh\` 中的路由配置和已部署 CLI。

### 2. 一键安装（GUI 内）

运行 SSH Router（从开始菜单或桌面快捷方式启动），主界面上方有**安装状态**面板和**快捷操作**按钮区：

1. 点击 **「安装 CLI」** — 从安装包释放 CLI 到 `C:\ProgramData\ssh\ssh-router-cli.exe`（会弹出 UAC 提权）
2. 点击 **「设置 DefaultShell」** — 自动设置注册表 `HKLM:\SOFTWARE\OpenSSH\DefaultShell` 指向 CLI（UAC 提权）
3. 点击 **「重启 sshd」** — 重启 SSH 服务使配置生效（UAC 提权）

安装状态面板会实时显示四项检查结果（CLI 部署、DefaultShell 设置、配置文件、sshd 服务状态），绿勾表示已完成。

> 也可以手动执行这些操作（见下方手动安装），但 GUI 一键安装更便捷。

### 3. 配置 sshd_config

确保 `sshd_config` 中**没有** `ForceCommand`（会与本程序冲突），并监听所需端口：

```text
Port 22
Port 2222
Port 2223

HostKey __PROGRAMDATA__/ssh/ssh_host_rsa_key
HostKey __PROGRAMDATA__/ssh/ssh_host_ecdsa_key
HostKey __PROGRAMDATA__/ssh/ssh_host_ed25519_key

AuthorizedKeysFile .ssh/authorized_keys

Subsystem sftp sftp-server.exe

AllowTcpForwarding yes
AllowStreamLocalForwarding yes
GatewayPorts yes

Match Group administrators
    AuthorizedKeysFile __PROGRAMDATA__/ssh/administrators_authorized_keys
```

### 4. 配置路由

SSH Router 常驻系统托盘（单实例，第二次启动会激活已有窗口）。双击托盘图标打开主界面：

- 若 `C:\ProgramData\ssh\ssh-router.json` 不存在，会提示创建默认三端口配置
- 在主界面中添加/编辑/删除端口路由、设置默认路由、配置 SFTP 命令

### 5. 手动安装（可选）

如果不使用 GUI 一键安装，也可以手动执行：

```powershell
# 部署 CLI（从安装包 resource 目录或 Release 下载）
Copy-Item ssh-router-cli.exe "C:\ProgramData\ssh\ssh-router-cli.exe" -Force

# 设置 DefaultShell
Set-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name "DefaultShell" -Value "C:\ProgramData\ssh\ssh-router-cli.exe"

# 重启 sshd
Restart-Service sshd -Force
```

## 自定义路由

运行 `ssh-router.exe`，在主界面中：

- 添加/编辑/删除端口路由（端口号、名称、目标 shell 路径、交互式模板、命令模板、临时文件扩展名）
- 勾选某条路由的「默认」标记，作为未匹配端口时的回退
- 配置 SFTP 命令（默认 `cmd.exe /c "C:\Windows\System32\OpenSSH\sftp-server.exe"`）

模板中可使用占位符：`{shell}`、`{tmpfile}`、`{tmpfile_wsl}`（详见 `crates/config/src/lib.rs`）。保存后写入 `ssh-router.json`，**立即生效，下次 SSH 连接即使用新配置，无需重启 sshd 或重新编译**。

## 自动更新

- 应用启动时检查一次稳定版更新，常驻托盘期间每 24 小时再次检查
- 检测到新版本后显示版本号和更新说明；确认后下载、验证签名、安装并重启
- 主界面的“软件更新”区域可随时手动检查
- GUI 更新后如果已部署 CLI 与安装包内版本不一致，会提示通过 UAC 同步；取消后可使用“安装/更新 CLI”重试
- 自动检查网络失败不会影响托盘、路由配置或 SSH 服务

更新包由 Tauri 签名机制验证，更新源固定为本项目的 GitHub Releases 稳定版；草稿和预发布版本不会进入自动更新通道。

## 环境要求

- Windows 10/11
- OpenSSH Server（sshd）
- WSL 2（端口 2223 路由需要）
- Git for Windows（端口 2222 路由需要）
- PowerShell 7（端口 22 路由需要）

## 已知限制

- WSL 发行版名称和 home 路径写在命令模板字符串中（非独立字段），改发行版需编辑模板
- 不支持 `ForceCommand`（如果 sshd_config 中设置了 ForceCommand，会与本程序冲突）
- SFTP 通过 `cmd.exe /c` 执行，读取的是 Windows 文件系统（`C:\` 路径），不是 WSL 文件系统（`/home/` 路径）
- Tauri GUI 需在 Windows 上原生构建，不支持交叉编译；CLI 可在 macOS 上 `cargo check` 但无法产出 Windows exe

## 排错

### 查看调试日志

连接端口、匹配路由、错误和退出码记录在 `C:\ProgramData\ssh\ssh-router-debug.log`。
日志默认不记录远程命令、完整参数或最终命令行，格式如下：

```text
2026-07-08 12:00:00.000 ========
2026-07-08 12:00:00.000 port: 2222
2026-07-08 12:00:00.000 route: Git Bash
2026-07-08 12:00:00.000 exit code: 0
2026-07-08 12:00:00.000 ========
```

如果 Remote SSH 工具报错，先查看日志确认端口、匹配路由和系统错误。日志由 `crates/cli/src/log.rs` 写入。

### zcode 连接卡住 / 转圈

1. 检查是否有僵尸进程：`pkill -9 -f zcode`（在 WSL 中执行）
2. 清理 SQLite WAL 锁文件：`rm -f ~/.zcode/v2/tasks-index.sqlite-*`
3. 完全退出 zcode（Cmd+Q），重新打开
4. 确认 `ssh-router-cli.exe` 使用的是 `CreateProcessW` 版本（不是 `Process.Start`，且不是旧 C# 版）

### detect 返回错误的 arch

- `arch: "linux"` → SSH_ORIGINAL_COMMAND bug，确认没有使用 ForceCommand
- `arch: "x64"` → 正确

### SFTP 卡住

- 确认配置中 `sftpCommand` 使用 `cmd.exe /c`（默认值即可）
- 确认 `sftp-server.exe` 路径正确：`C:\Windows\System32\OpenSSH\sftp-server.exe`

### GUI 不显示 / 托盘无图标

- SSH Router 以普通用户运行即可，安装 CLI / 设置 DefaultShell / 重启 sshd 时会按需弹出 UAC 提权
- 单实例插件会激活已有窗口，若托盘有图标说明已在运行
- 双击托盘图标可打开主界面
