# SSH Router

Windows OpenSSH 多端口智能路由器。

通过读取 `SSH_CONNECTION` 环境变量中的服务端监听端口，将不同 SSH 端口的连接路由到不同的 shell 环境。

## 解决的问题

在 Windows OpenSSH 上，`DefaultShell` 是全局设置，只能指定一个 shell。当需要不同端口连接到不同 shell 时（如 WSL、Git Bash、PowerShell），有以下困难：

### 方案对比与踩过的坑

| 方案 | 问题 |
|------|------|
| `ForceCommand` + `DefaultShell` | ForceCommand 的值会通过 DefaultShell 执行，路径被当作 shell 命令解析，导致冲突 |
| `ForceCommand` + `SSH_ORIGINAL_COMMAND` | Windows OpenSSH 的 bug：同一 SSH 连接后续 exec 请求不更新 `SSH_ORIGINAL_COMMAND`，只有第一次 exec 的命令被传递 |
| `.cmd` 脚本作为 `DefaultShell`（用 `%*`） | cmd.exe 的 `%*` 展开时会重新解析引号，破坏包含 `'\''` 转义的复杂命令（如 zcode deploy 阶段的命令） |
| `bash.exe`（WSL 入口）作为 `DefaultShell` | bash.exe 会展开命令中的 `$var`（即使单引号内也会展开），导致 preflight 命令的变量赋值丢失 |
| Git Bash `bash.exe` 作为 `DefaultShell` | `uname -s` 返回 `MSYS_NT`，被 zcode 检测为 `win32` 而非 `linux` |
| C# `Process.Start` 启动子进程 | .NET runtime 对 stdin/stdout 管道的处理导致长时间运行的进程（如 zcode server）通信卡住 |

### 最终方案

用一个 C# 程序作为 `DefaultShell`，满足以下要求：

1. **绕过 cmd.exe**：sshd 直接调用 `.exe`，`argv` 由 Windows CRT 解析，原始命令完整保留
2. **不用 `SSH_ORIGINAL_COMMAND`**：避免 Windows OpenSSH 同一连接后续 exec 不更新的 bug
3. **单引号包裹命令**：传给 `bash -c` 的命令用单引号包裹，防止 `$var` 被提前展开
4. **`CreateProcessW` 替代 `Process.Start`**：直接用 Windows API 继承 stdin/stdout/stderr 句柄，避免 .NET runtime 对管道的缓冲
5. **Job Object**：确保子进程在父进程退出时被杀死，避免僵尸进程残留
6. **SFTP 特殊处理**：SFTP 子系统也通过 DefaultShell 执行，通过 `cmd.exe /c` 启动 `sftp-server.exe` 正确继承二进制管道

## 端口路由

| 端口 | 目标 Shell | 用途 |
|------|-----------|------|
| 22   | PowerShell 7 | Windows 管理 |
| 2222 | Git Bash | Codex Remote SSH |
| 2223 | WSL Ubuntu | zcode / 通用 Linux 开发 |

## 工作原理

```
SSH 客户端 (zcode / Codex / ssh)
    │
    ▼
Windows OpenSSH sshd
    │  CreateProcessW(SshRouter.exe, "SshRouter.exe -c \"command\"")
    │  argv 由 Windows CRT 解析，原始命令完整保留
    ▼
SshRouter.exe
    │  1. 从 argv[1] 获取原始命令（不经 cmd.exe 解析）
    │  2. 从 SSH_CONNECTION 获取服务端端口
    │  3. 用单引号包裹命令，防止 $var 被 bash 双引号展开
    │  4. 用 CreateProcessW(bInheritHandles=true) 启动对应 shell
    │     不设 STARTF_USESTDHANDLES，让句柄自动继承
    │  5. 将子进程加入 Job Object（KILL_ON_JOB_CLOSE）
    ▼
wsl.exe / bash.exe / pwsh.exe / sftp-server.exe
```

### 关键设计决策

#### 为什么用 `CreateProcessW` 而不是 `Process.Start`？

.NET 的 `Process.Start`（即使 `UseShellExecute=false`）对 stdin/stdout 管道的处理方式与直接 `CreateProcessW` 不同。对于短时间运行的命令（如 `uname -s`）没有区别，但对于**长时间运行的双向通信进程**（如 zcode server），`Process.Start` 会导致通信卡住。

直接用 `CreateProcessW` 配合 `bInheritHandles=true`，不设 `STARTF_USESTDHANDLES`，让子进程完全继承父进程的 stdin/stdout/stderr 句柄，确保二进制数据（如 MessagePack 协议、SFTP 协议）正确传递。

#### 为什么用单引号包裹命令？

bash 的 `bash -c "command"` 中，双引号内的 `$var` 会被 bash 提前展开。对于包含变量赋值的命令（如 zcode 的 preflight：`download=; if ...; then download=curl; fi; printf ... $download`），`$download` 会被提前展开为空。

用单引号包裹 `bash -c 'command'`，单引号阻止 `$var` 展开，命令在 WSL bash 内部正确执行。

#### 为什么需要 Job Object？

当 SSH 连接断开时，sshd 会杀死 SshRouter.exe 进程。但子进程（wsl.exe → bash → node）不会被自动杀死，导致：
- 旧 server 进程残留，占用内存
- SQLite 数据库锁未释放
- 新连接的 server 无法获取锁，整个 UI 卡住

Job Object 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，当 SshRouter.exe 退出时（Job Object 句柄关闭），所有子进程自动被杀死。

#### 为什么 SFTP 用 `cmd.exe /c`？

SFTP 子系统（`Subsystem sftp sftp-server.exe`）也通过 DefaultShell 执行。SshRouter.exe 收到 `-c "sftp-server.exe"` 后，需要直接启动 Windows 的 `sftp-server.exe`。

但 `CreateProcessW` 直接启动 `sftp-server.exe` 时 stdin 管道传递有问题（进程立即退出）。通过 `cmd.exe /c sftp-server.exe` 中间层，cmd.exe 正确继承并转发 stdin/stdout 的二进制 SFTP 协议数据。

## 编译

需要 .NET Framework 4.0+（Windows 自带）。

```powershell
& "C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe" /out:"C:\ProgramData\ssh\SshRouter.exe" "SshRouter.cs"
```

## 安装

### 1. 编译 SshRouter.exe

```powershell
& "C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe" /out:"C:\ProgramData\ssh\SshRouter.exe" "C:\ProgramData\ssh\SshRouter.cs"
```

### 2. 设置 DefaultShell

```powershell
Set-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name "DefaultShell" -Value "C:\ProgramData\ssh\SshRouter.exe"
```

### 3. 配置 sshd_config

确保 `sshd_config` 中**没有** `ForceCommand`（会与本程序冲突）：

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

### 4. 重启 sshd

```powershell
Restart-Service sshd -Force
```

## 自定义路由

修改 `SshRouter.cs` 中的端口判断和目标 shell 路径：

```csharp
if (port == "你的端口")
{
    cmdLine = "你的shell.exe 参数 " + singleQuoted;
}
```

重新编译后替换 `SshRouter.exe` 即可，无需重启 sshd。

## 环境要求

- Windows 10/11
- OpenSSH Server（sshd）
- WSL 2（端口 2223 路由需要）
- Git for Windows（端口 2222 路由需要）
- PowerShell 7（端口 22 路由需要）
- .NET Framework 4.0+（编译需要，Windows 自带）

## 已知限制

- 端口号硬编码在源码中，修改需要重新编译
- WSL 发行版名称硬编码为 `Ubuntu`，如需修改请编辑源码
- 不支持 `ForceCommand`（如果 sshd_config 中设置了 ForceCommand，会与本程序冲突）
- SFTP 通过 `cmd.exe /c` 执行，读取的是 Windows 文件系统（`C:\` 路径），不是 WSL 文件系统（`/home/` 路径）

## 排错

### zcode 连接卡住 / 转圈

1. 检查是否有僵尸进程：`pkill -9 -f zcode`（在 WSL 中执行）
2. 清理 SQLite WAL 锁文件：`rm -f ~/.zcode/v2/tasks-index.sqlite-*`
3. 完全退出 zcode（Cmd+Q），重新打开
4. 确认 SshRouter.exe 使用的是 `CreateProcessW` 版本（不是 `Process.Start`）

### detect 返回错误的 arch

- `arch: "linux"` → SSH_ORIGINAL_COMMAND bug，确认没有使用 ForceCommand
- `arch: "x64"` → 正确

### SFTP 卡住

- 确认 SshRouter.exe 中 SFTP 分支使用 `cmd.exe /c`
- 确认 `sftp-server.exe` 路径正确：`C:\Windows\System32\OpenSSH\sftp-server.exe`
