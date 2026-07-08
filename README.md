# SSH Router

Windows OpenSSH 多端口智能路由器。

通过读取 `SSH_CONNECTION` 环境变量中的服务端监听端口，将不同 SSH 端口的连接路由到不同的 shell 环境。

## 解决的问题

Windows OpenSSH 的 `DefaultShell` 是全局设置，只能指定一个 shell。当需要不同端口连接到不同 shell 时（如 WSL、Git Bash、PowerShell），无法通过 `sshd_config` 的 `ForceCommand` 实现，因为 `ForceCommand` 的值会通过 `DefaultShell` 执行，导致路径被当作 shell 命令解析。

此外，`.cmd` 脚本作为 `DefaultShell` 时，`%*` 会破坏包含复杂引号嵌套的命令（如 `'\''` 转义），导致 VSCode Remote SSH、Codex Remote SSH、zcode 等工具的 deploy 阶段失败。

本程序作为 `DefaultShell`，直接接收 sshd 传递的 `argv`（绕过 cmd.exe 的引号解析），根据端口路由到对应的 shell。

## 端口路由

| 端口 | 目标 Shell | 用途 |
|------|-----------|------|
| 22   | PowerShell 7 | Windows 管理 |
| 2222 | Git Bash | Codex Remote SSH |
| 2223 | WSL Ubuntu | zcode / 通用 Linux 开发 |

## 工作原理

```
SSH 客户端
    │
    ▼
Windows OpenSSH sshd
    │  调用 CreateProcessW(SshRouter.exe, "SshRouter.exe -c \"command\"")
    ▼
SshRouter.exe
    │  1. 从 argv[1] 获取原始命令（不经 cmd.exe 解析）
    │  2. 从 SSH_CONNECTION 获取服务端端口
    │  3. 用单引号包裹命令，防止 $var 被 bash 双引号展开
    │  4. 根据端口启动对应 shell
    ▼
wsl.exe / bash.exe / pwsh.exe
```

### 关键设计

- **绕过 cmd.exe**：sshd 直接调用 `.exe`，`argv` 由 Windows CRT 解析，原始命令完整保留
- **单引号包裹**：传给 `bash -c` 的命令用单引号包裹，防止 `$var` 在双引号中被提前展开（这是 `bash.exe` WSL 入口程序的已知问题）
- **stdin/stdout 继承**：`UseShellExecute = false` 让子进程直接继承父进程的标准流

## 编译

需要 .NET Framework 4.0+（Windows 自带）。

```powershell
& "C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe" /out:"C:\ProgramData\ssh\SshRouter.exe" "SshRouter.cs"
```

## 安装

### 1. 编译

```powershell
& "C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe" /out:"C:\ProgramData\ssh\SshRouter.exe" "C:\ProgramData\ssh\SshRouter.cs"
```

### 2. 设置 DefaultShell

```powershell
Set-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name "DefaultShell" -Value "C:\ProgramData\ssh\SshRouter.exe"
```

### 3. sshd_config

确保 `sshd_config` 中**没有** `ForceCommand`（ForceCommand 的值会通过 DefaultShell 执行，导致冲突）：

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
    fileName = "你的shell.exe";
    arguments = "你的参数 " + singleQuoted;
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
