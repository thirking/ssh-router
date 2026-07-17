param(
    [string]$Executable = "",
    [int]$ProcessId = 0,
    [ValidateRange(0, 3600)][int]$WarmupSeconds = 10,
    [ValidateRange(1, 86400)][int]$ObservationSeconds = 60,
    [ValidateRange(1, 3600)][int]$IntervalSeconds = 1,
    [bool]$IncludeDescendants = $true,
    [string]$Output = "idle-results.csv"
)

$ErrorActionPreference = "Stop"
if (($ObservationSeconds % $IntervalSeconds) -ne 0) {
    throw "ObservationSeconds must be divisible by IntervalSeconds"
}
$ownsProcess = $ProcessId -eq 0
$process = $null
$marker = $null
$previousMarker = $env:SSH_ROUTER_STARTUP_MARKER
$previousInstance = $env:SSH_ROUTER_INSTANCE_NAME
function Get-ProcessTree {
    param([Diagnostics.Process]$RootProcess)
    if (-not $IncludeDescendants) { return @($RootProcess) }
    $ids = [Collections.Generic.List[int]]::new()
    $queue = [Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($RootProcess.Id)
    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        $ids.Add($parent)
        Get-CimInstance Win32_Process -Filter "ParentProcessId = $parent" -ErrorAction SilentlyContinue | ForEach-Object {
            $queue.Enqueue([int]$_.ProcessId)
        }
    }
    return @(Get-Process -Id $ids.ToArray() -ErrorAction SilentlyContinue)
}
try {
    if ($ownsProcess) {
        if (-not $Executable) { throw "Executable or ProcessId is required" }
        $marker = Join-Path $env:TEMP "ssh-router-idle-$PID.marker"
        Remove-Item $marker -Force -ErrorAction SilentlyContinue
        $env:SSH_ROUTER_STARTUP_MARKER = $marker
        $env:SSH_ROUTER_INSTANCE_NAME = "Local\SSHRouterPerformance-$PID"
        $process = Start-Process -FilePath $Executable -PassThru
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        while (-not (Test-Path $marker)) {
            $process.Refresh()
            if ($process.HasExited) { throw "Application exited before startup marker" }
            if ([DateTime]::UtcNow -gt $deadline) { throw "Startup marker timed out" }
            Start-Sleep -Milliseconds 20
        }
    } else {
        $process = Get-Process -Id $ProcessId -ErrorAction Stop
    }
    Start-Sleep -Seconds $WarmupSeconds
    $samples = @()
    $previousCpuById = @{}
    @(Get-ProcessTree $process) | ForEach-Object {
        $_.Refresh()
        $previousCpuById[$_.Id] = $_.TotalProcessorTime.TotalMilliseconds
    }
    $previousTime = [DateTime]::UtcNow
    for ($elapsed = 0; $elapsed -lt $ObservationSeconds; $elapsed += $IntervalSeconds) {
        Start-Sleep -Seconds $IntervalSeconds
        # 子进程可能在观察期间启动或重启，因此每轮重新发现整棵进程树。
        $processes = @(Get-ProcessTree $process)
        $nextCpuById = @{}
        $cpuDelta = 0.0
        $processes | ForEach-Object {
            $_.Refresh()
            $currentCpu = $_.TotalProcessorTime.TotalMilliseconds
            if ($previousCpuById.ContainsKey($_.Id)) {
                $cpuDelta += [Math]::Max(0, $currentCpu - $previousCpuById[$_.Id])
            } else {
                # 新发现的进程在当前采样区间内创建，其累计 CPU 均属于本区间。
                $cpuDelta += $currentCpu
            }
            $nextCpuById[$_.Id] = $currentCpu
        }
        $now = [DateTime]::UtcNow
        $wall = ($now - $previousTime).TotalMilliseconds
        $samples += [pscustomobject]@{
            TimestampUtc = $now.ToString("o")
            ProcessCount = $processes.Count
            WorkingSetMiB = [Math]::Round((($processes.WorkingSet64 | Measure-Object -Sum).Sum) / 1MB, 2)
            PrivateBytesMiB = [Math]::Round((($processes.PrivateMemorySize64 | Measure-Object -Sum).Sum) / 1MB, 2)
            CpuPercent = [Math]::Round(($cpuDelta / $wall / [Environment]::ProcessorCount) * 100, 3)
        }
        $previousCpuById = $nextCpuById
        $previousTime = $now
    }
    $samples | Export-Csv $Output -NoTypeInformation -Encoding UTF8
    Write-Host "Samples=$($samples.Count) AverageWorkingSet=$([Math]::Round(($samples.WorkingSetMiB | Measure-Object -Average).Average, 2))MiB AveragePrivateBytes=$([Math]::Round(($samples.PrivateBytesMiB | Measure-Object -Average).Average, 2))MiB AverageCPU=$([Math]::Round(($samples.CpuPercent | Measure-Object -Average).Average, 3))%"
} finally {
    if ($ownsProcess -and $process -and -not $process.HasExited) { $process.Kill() }
    if ($marker) { Remove-Item $marker -Force -ErrorAction SilentlyContinue }
    if ($null -eq $previousMarker) { Remove-Item Env:SSH_ROUTER_STARTUP_MARKER -ErrorAction SilentlyContinue } else { $env:SSH_ROUTER_STARTUP_MARKER = $previousMarker }
    if ($null -eq $previousInstance) { Remove-Item Env:SSH_ROUTER_INSTANCE_NAME -ErrorAction SilentlyContinue } else { $env:SSH_ROUTER_INSTANCE_NAME = $previousInstance }
}
