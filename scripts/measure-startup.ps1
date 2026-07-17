param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [ValidateRange(1, 1000)][int]$Iterations = 10,
    [ValidateRange(1, 300)][int]$TimeoutSeconds = 30,
    [ValidateSet("Marker", "InputIdle")][string]$ReadyMode = "Marker",
    [ValidateRange(1, 100)][int]$MinimumProcessCount = 1,
    [bool]$RequireMainWindow = $false,
    [string]$Output = "startup-results.csv"
)

$ErrorActionPreference = "Stop"
$previousMarker = $env:SSH_ROUTER_STARTUP_MARKER
$previousInstance = $env:SSH_ROUTER_INSTANCE_NAME
$results = @()
$process = $null
$marker = $null
function Get-ProcessTreeCount {
    param([int]$RootProcessId)
    $ids = [Collections.Generic.HashSet[int]]::new()
    $queue = [Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($RootProcessId)
    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        if (-not $ids.Add($parent)) { continue }
        Get-CimInstance Win32_Process -Filter "ParentProcessId = $parent" -ErrorAction SilentlyContinue | ForEach-Object {
            $queue.Enqueue([int]$_.ProcessId)
        }
    }
    return $ids.Count
}
try {
    for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
        $marker = Join-Path $env:TEMP "ssh-router-startup-$PID-$iteration.marker"
        Remove-Item $marker -Force -ErrorAction SilentlyContinue
        if ($ReadyMode -eq "Marker") {
            $env:SSH_ROUTER_STARTUP_MARKER = $marker
            $env:SSH_ROUTER_INSTANCE_NAME = "Local\SSHRouterPerformance-$PID-$iteration"
        } else {
            Remove-Item Env:SSH_ROUTER_STARTUP_MARKER -ErrorAction SilentlyContinue
            Remove-Item Env:SSH_ROUTER_INSTANCE_NAME -ErrorAction SilentlyContinue
        }
        $timer = [Diagnostics.Stopwatch]::StartNew()
        $process = Start-Process -FilePath $Executable -PassThru
        if ($ReadyMode -eq "InputIdle") {
            if (-not $process.WaitForInputIdle($TimeoutSeconds * 1000)) {
                throw "WaitForInputIdle timed out"
            }
        } else {
            $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
            while (-not (Test-Path $marker)) {
                $process.Refresh()
                if ($process.HasExited) { throw "Application exited before startup marker" }
                if ([DateTime]::UtcNow -gt $deadline) { throw "Startup marker timed out" }
                Start-Sleep -Milliseconds 5
            }
        }
        $timer.Stop()
        $startupMilliseconds = [Math]::Round($timer.Elapsed.TotalMilliseconds, 2)
        # 防止单实例二次启动刚进入消息循环便退出，被误记为完整冷启动。
        Start-Sleep -Milliseconds 500
        $process.Refresh()
        if ($process.HasExited) { throw "Application exited after reporting ready; another instance may already be running" }
        $processCount = Get-ProcessTreeCount $process.Id
        if ($processCount -lt $MinimumProcessCount) {
            throw "Process tree contains $processCount process(es), expected at least $MinimumProcessCount"
        }
        $mainWindowHandle = $process.MainWindowHandle.ToInt64()
        if ($RequireMainWindow -and $mainWindowHandle -eq 0) {
            throw "Application reported ready without a main window"
        }
        $results += [pscustomobject]@{
            Iteration = $iteration
            ReadyMode = $ReadyMode
            StartupMilliseconds = $startupMilliseconds
            ProcessCount = $processCount
            MainWindowHandle = $mainWindowHandle
        }
        if (-not $process.HasExited) { $process.Kill() }
        $process.WaitForExit(5000) | Out-Null
        $process = $null
        Remove-Item $marker -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 500
    }
    $results | Export-Csv $Output -NoTypeInformation -Encoding UTF8
    $values = $results.StartupMilliseconds
    Write-Host "Samples=$($values.Count) Min=$([Math]::Round(($values | Measure-Object -Minimum).Minimum, 2))ms Average=$([Math]::Round(($values | Measure-Object -Average).Average, 2))ms Max=$([Math]::Round(($values | Measure-Object -Maximum).Maximum, 2))ms"
} finally {
    if ($process -and -not $process.HasExited) { $process.Kill() }
    if ($marker) { Remove-Item $marker -Force -ErrorAction SilentlyContinue }
    if ($null -eq $previousMarker) { Remove-Item Env:SSH_ROUTER_STARTUP_MARKER -ErrorAction SilentlyContinue } else { $env:SSH_ROUTER_STARTUP_MARKER = $previousMarker }
    if ($null -eq $previousInstance) { Remove-Item Env:SSH_ROUTER_INSTANCE_NAME -ErrorAction SilentlyContinue } else { $env:SSH_ROUTER_INSTANCE_NAME = $previousInstance }
}
