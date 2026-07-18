# GPUI MVP 人工测试清单

## 测试包

- 文件：`publish/SSH-Router_0.0.8_x64-setup.exe`
- 大小：`6,661,277` bytes
- SHA-256：`0D856C214C061F3160341C448501165F1309C282C8379E01DDBA27E44AC27BC9`
- 用途：本地最终候选，仅用于人工安装与功能验证；未生成正式 Release 的 `.sig` 和 `latest.json`。

建议在可销毁的 Windows 11 VM 或全新交互用户中测试。三项快捷操作会修改 `C:\ProgramData\ssh`、OpenSSH 注册表和 `sshd` 服务，不要在生产 SSH 服务器上直接执行。

## 测试前记录

```powershell
Get-FileHash .\SSH-Router_0.0.8_x64-setup.exe -Algorithm SHA256
Get-ComputerInfo | Select-Object WindowsProductName, WindowsVersion, OsBuildNumber
Get-Service sshd -ErrorAction SilentlyContinue
Get-ItemProperty 'HKLM:\SOFTWARE\OpenSSH' -ErrorAction SilentlyContinue
Get-FileHash 'C:\ProgramData\ssh\ssh-router.json' -Algorithm SHA256 -ErrorAction SilentlyContinue
```

如环境已有配置，先备份：

```powershell
Copy-Item 'C:\ProgramData\ssh\ssh-router.json' "$env:TEMP\ssh-router.json.backup" -ErrorAction SilentlyContinue
reg export 'HKLM\SOFTWARE\OpenSSH' "$env:TEMP\openssh.reg" /y
```

## 1. 全新安装与首次启动

- [ ] 在全新用户中运行安装包，安装过程无错误。
- [ ] 存在 `%LOCALAPPDATA%\SSH Router\ssh-router.exe`。
- [ ] 存在 `%LOCALAPPDATA%\SSH Router\resources\ssh-router-cli.exe`。
- [ ] 开始菜单存在 `SSH Router` 快捷方式。
- [ ] `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router` 信息正确。
- [ ] 首次启动不弹主窗口，通知区域出现图标。
- [ ] 双击托盘图标可显示并聚焦主窗口。

## 2. 默认配置与兼容配置

仅在可销毁环境中删除或移动现有配置。

- [ ] `C:\ProgramData\ssh\ssh-router.json` 不存在时，界面明确提示配置缺失。
- [ ] 点击“创建默认配置”，接受 UAC 后文件创建成功，界面加载默认路由。
- [ ] 将一个旧版兼容 `ssh-router.json` 放回原路径，重新加载后内容显示正确。
- [ ] 编辑一个路由并保存，重新启动 GUI 后修改仍存在。
- [ ] 保存后候选 CLI 能读取配置并成功执行：

```powershell
$env:SSH_CONNECTION = '127.0.0.1 12345 127.0.0.1 22'
& "$env:LOCALAPPDATA\SSH Router\resources\ssh-router-cli.exe" -c 'exit 0'
$LASTEXITCODE # 预期 0
```

## 3. 三项提权操作

- [ ] “安装 CLI”接受 UAC 后，`C:\ProgramData\ssh\ssh-router-cli.exe` 存在且状态变为已安装/最新。
- [ ] “设置 DefaultShell”接受 UAC 后，`HKLM\SOFTWARE\OpenSSH\DefaultShell` 指向部署的 CLI。
- [ ] “重启 sshd”接受 UAC 后服务恢复为 `Running`，现有测试会话未异常中断。
- [ ] 至少取消一次 UAC，界面明确显示已取消，按钮不会永久停留在忙碌状态。
- [ ] 操作失败时错误信息可理解，点击重试后状态可恢复。

## 4. 窗口、托盘与输入

- [ ] 关闭主窗口只隐藏到托盘，进程仍存在。
- [ ] 再次启动程序只激活现有窗口，没有第二个常驻进程。
- [ ] 托盘菜单“退出”真正结束进程。
- [ ] 启动程序后重启 Explorer，托盘图标能够恢复：

```powershell
Stop-Process -Name explorer
Start-Process explorer.exe
```

- [ ] 使用中文 IME 输入“中文路由测试”，完成组合、选词和提交；保存后文本不丢失、不重复。
- [ ] Tab、Shift+Tab、Enter、Space 和 Escape 操作正常，焦点样式清晰。
- [ ] 将窗口缩放到约 `640x520`，所有操作仍可通过滚动访问，文字和控件不重叠。

## 5. 覆盖升级

需要先在可销毁 VM 安装已发布的旧 v0.0.8。

- [ ] 旧版中创建一份配置并记录哈希。
- [ ] 不卸载旧版，直接运行本候选安装包。
- [ ] 安装后运行的是 GPUI 版本，配置内容和 `C:\ProgramData\ssh\ssh-router-cli.exe` 均保留。
- [ ] 开始菜单、安装目录和卸载信息没有重复项或残留旧入口。

## 6. 更新

- [ ] 网络可访问生产 `latest.json` 时，点击“检查更新”；无新版本应提示“当前已是最新版本”。
- [ ] 断网后手动检查，界面显示网络错误且可重试。
- [ ] 自动更新替换与重启需要一个高于 `0.0.8`、带有效 minisign 签名的测试 Release；确认下载、验签、父进程退出、安装替换和重启全部完成。
- [ ] 无效签名安装包必须被拒绝，不能启动安装程序。

## 7. 卸载

- [ ] 从系统卸载 SSH Router，GUI、资源目录、开始菜单快捷方式和卸载注册表项均删除。
- [ ] `C:\ProgramData\ssh\ssh-router.json` 和其他 SSH 数据仍保留。
- [ ] 若程序仍在运行，卸载器应阻止删除并提示先从托盘退出。

## 反馈内容

请为每个失败项提供：步骤编号、实际结果、期望结果、截图或录屏、Windows Build、安装包 SHA-256。涉及配置、注册表或服务时，再附测试前后的值；不要发送私钥、密码或敏感路由内容。
