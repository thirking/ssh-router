# 桌面性能基线

性能数据用于发现迭代退化，MVP 不设置通过门槛。每次记录必须注明 Windows 版本、CPU、内存、电源模式、应用版本和安装来源。

## 启动时间

关闭所有 SSH Router 实例，固定为“最佳性能”电源模式，运行至少 10 次：

```powershell
./scripts/measure-startup.ps1 -Executable "C:\path\ssh-router.exe" -Iterations 10
```

时间范围为创建进程到 GPUI 完成首帧并写出“窗口可交互”标记。

旧版 v0.0.8 不支持该标记，使用系统输入空闲事件：

```powershell
./scripts/measure-startup.ps1 -Executable "C:\path\v0.0.8\ssh-router.exe" -ReadyMode InputIdle -Iterations 10
```

## 空闲资源

不操作窗口，预热 10 秒并观察至少 60 秒：

```powershell
./scripts/measure-idle.ps1 -Executable "C:\path\ssh-router.exe" -WarmupSeconds 10 -ObservationSeconds 60
```

也可只读采样一个已运行进程，不会终止它：

```powershell
./scripts/measure-idle.ps1 -ProcessId 1234 -WarmupSeconds 10 -ObservationSeconds 60
```

记录根进程及其后代进程汇总的 Working Set、Private Bytes 和按逻辑处理器归一化的 CPU；这使多进程旧版与单进程 GPUI 使用一致的应用级口径。可用 `-IncludeDescendants $false` 仅采样根进程。

旧版基线必须使用已发布的 v0.0.8 安装包；GPUI 基线使用对应发布候选安装包。不要使用修改过的本地 Tauri 构建链。
