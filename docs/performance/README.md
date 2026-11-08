# 桌面性能基线

性能数据用于发现迭代退化，MVP 不设置通过门槛。每次记录必须注明 Windows 版本、CPU、内存、电源模式、应用版本和安装来源。

## 启动时间

关闭所有 SSH Router 实例，固定为“最佳性能”电源模式，运行至少 10 次：

```powershell
./scripts/measure-startup.ps1 -Executable "C:\path\ssh-router.exe" -Iterations 10
```

时间范围为创建进程到 gpuix 完成首次布局并写出启动标记。配置加载时间应另行记录。

旧版 v0.0.8 不支持该标记，使用系统输入空闲事件：

```powershell
./scripts/measure-startup.ps1 -Executable "C:\path\v0.0.8\ssh-router.exe" -ReadyMode InputIdle -Iterations 10 -MinimumProcessCount 2 -RequireMainWindow $true
```

旧版采样前必须确认同一 Windows 会话中没有任何 SSH Router 实例。CSV 中每轮必须有非零 `MainWindowHandle`，且 `ProcessCount` 至少为 2，以排除单实例二次启动退出并证明 WebView 进程树已创建。

## 空闲资源

不操作窗口，预热 10 秒并观察至少 60 秒：

```powershell
./scripts/measure-idle.ps1 -Executable "C:\path\ssh-router.exe" -WarmupSeconds 10 -ObservationSeconds 60
```

也可只读采样一个已运行进程，不会终止它：

```powershell
./scripts/measure-idle.ps1 -ProcessId 1234 -WarmupSeconds 10 -ObservationSeconds 60
```

记录 GUI 及其业务后端进程汇总的 Working Set、Private Bytes 和按逻辑处理器归一化的 CPU。可用 `-IncludeDescendants $false` 仅采样根进程。

gpuix 版本的 Windows 性能基线尚未采样。每份基线必须对应明确的安装包哈希，不能将其他前端版本的测量结果用作 gpuix 数据。
