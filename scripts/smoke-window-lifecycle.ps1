param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,

    [Parameter(Mandatory = $true)]
    [string]$Output
)

$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

public static class SshRouterWindowProbe
{
    public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lparam);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);

    [DllImport("user32.dll")]
    public static extern bool EnumThreadWindows(uint threadId, EnumWindowsProc callback, IntPtr lparam);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr FindWindow(string className, string windowName);

    [DllImport("oleacc.dll")]
    private static extern int AccessibleObjectFromWindow(
        IntPtr hwnd,
        uint objectId,
        ref Guid interfaceId,
        [MarshalAs(UnmanagedType.Interface)] out object accessible
    );

    public static IntPtr FindWindow(int processId)
    {
        foreach (ProcessThread thread in Process.GetProcessById(processId).Threads)
        {
            IntPtr found = IntPtr.Zero;
            EnumThreadWindows((uint)thread.Id, (hwnd, lparam) =>
            {
                uint owner;
                GetWindowThreadProcessId(hwnd, out owner);
                if (owner != (uint)processId)
                {
                    return true;
                }

                found = hwnd;
                return false;
            }, IntPtr.Zero);
            if (found != IntPtr.Zero)
            {
                return found;
            }
        }

        return IntPtr.Zero;
    }

    public static object GetAccessibleClient(IntPtr hwnd)
    {
        const uint ObjectIdClient = 0xFFFFFFFC;
        Guid accessibleInterface = new Guid("618736e0-3c3d-11cf-810c-00aa00389b71");
        object accessible;
        int result = AccessibleObjectFromWindow(
            hwnd,
            ObjectIdClient,
            ref accessibleInterface,
            out accessible
        );
        Marshal.ThrowExceptionForHR(result);
        return accessible;
    }
}
"@

function Wait-Until {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock]$Condition,
        [int]$TimeoutMilliseconds = 20000
    )

    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        if (& $Condition) {
            return $true
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)

    return $false
}

$primary = $null
$secondary = $null
$result = [ordered]@{
    Executable = $Executable
    SessionId = (Get-Process -Id $PID).SessionId
    WindowCreatedMilliseconds = $null
    InitialVisible = $null
    TrayRestoreMilliseconds = $null
    RestoredFromTray = $false
    CloseToTrayMilliseconds = $null
    HiddenAfterClose = $false
    SecondaryActivationMilliseconds = $null
    SecondaryExited = $false
    SecondaryExitCode = $null
    VisibleAfterSecondary = $false
    ExitedFromTrayMenu = $false
    Error = $null
}

try {
    Remove-Item Env:SSH_ROUTER_STARTUP_MARKER -ErrorAction SilentlyContinue
    Remove-Item Env:SSH_ROUTER_INSTANCE_NAME -ErrorAction SilentlyContinue

    $timer = [Diagnostics.Stopwatch]::StartNew()
    $primary = Start-Process -FilePath $Executable -PassThru
    $window = [IntPtr]::Zero
    $windowReady = Wait-Until {
        if ($primary.HasExited) {
            return $false
        }
        $script:window = [SshRouterWindowProbe]::FindWindow($primary.Id)
        return $script:window -ne [IntPtr]::Zero
    }
    if (-not $windowReady) {
        throw "Main window handle was not created before timeout"
    }
    $timer.Stop()
    $result.WindowCreatedMilliseconds = [Math]::Round($timer.Elapsed.TotalMilliseconds, 2)

    $result.InitialVisible = [SshRouterWindowProbe]::IsWindowVisible($window)
    if ($result.InitialVisible) {
        throw "Production window was visible before tray activation"
    }

    $timer.Restart()
    if (-not [SshRouterWindowProbe]::PostMessage($window, 1066, [IntPtr]::Zero, [IntPtr]0x0203)) {
        throw "Failed to post the tray double-click message"
    }
    $result.RestoredFromTray = Wait-Until {
        [SshRouterWindowProbe]::IsWindowVisible($window)
    }
    if (-not $result.RestoredFromTray) {
        throw "Window did not restore from the tray"
    }
    $timer.Stop()
    $result.TrayRestoreMilliseconds = [Math]::Round($timer.Elapsed.TotalMilliseconds, 2)

    $timer.Restart()
    if (-not [SshRouterWindowProbe]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw "Failed to post the window close message"
    }
    $result.HiddenAfterClose = Wait-Until {
        -not [SshRouterWindowProbe]::IsWindowVisible($window)
    }
    if (-not $result.HiddenAfterClose) {
        throw "Window did not hide after close"
    }
    $timer.Stop()
    $result.CloseToTrayMilliseconds = [Math]::Round($timer.Elapsed.TotalMilliseconds, 2)

    $timer.Restart()
    $secondary = Start-Process -FilePath $Executable -PassThru
    $result.SecondaryExited = Wait-Until { $secondary.HasExited }
    if (-not $result.SecondaryExited) {
        throw "Secondary instance did not exit"
    }
    $result.SecondaryExitCode = $secondary.ExitCode
    if ($result.SecondaryExitCode -ne 0) {
        throw "Secondary instance exited with code $($result.SecondaryExitCode)"
    }
    $result.VisibleAfterSecondary = Wait-Until {
        [SshRouterWindowProbe]::IsWindowVisible($window)
    }
    if (-not $result.VisibleAfterSecondary) {
        throw "Secondary instance did not activate the primary window"
    }
    $timer.Stop()
    $result.SecondaryActivationMilliseconds = [Math]::Round($timer.Elapsed.TotalMilliseconds, 2)

    if (-not [SshRouterWindowProbe]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw "Failed to post the window close message after secondary activation"
    }
    if (-not (Wait-Until { -not [SshRouterWindowProbe]::IsWindowVisible($window) })) {
        throw "Window did not hide after secondary activation"
    }

    if (-not [SshRouterWindowProbe]::PostMessage($window, 1066, [IntPtr]::Zero, [IntPtr]0x0205)) {
        throw "Failed to post the tray context-menu message"
    }
    $menu = [IntPtr]::Zero
    if (-not (Wait-Until {
        $script:menu = [SshRouterWindowProbe]::FindWindow("#32768", $null)
        return $script:menu -ne [IntPtr]::Zero
    })) {
        throw "Tray context menu window was not created"
    }
    $accessible = [SshRouterWindowProbe]::GetAccessibleClient($menu)
    $itemCount = [int]$accessible.accChildCount
    if ($itemCount -lt 1) {
        throw "Tray context menu did not expose any accessible items"
    }
    $accessible.accDoDefaultAction($itemCount)
    $result.ExitedFromTrayMenu = Wait-Until { $primary.HasExited }
    if (-not $result.ExitedFromTrayMenu) {
        throw "Primary instance did not exit from the tray menu"
    }
}
catch {
    $result.Error = $_.Exception.Message
}
finally {
    if ($null -ne $secondary -and -not $secondary.HasExited) {
        Stop-Process -Id $secondary.Id -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne $primary -and -not $primary.HasExited) {
        Stop-Process -Id $primary.Id -Force -ErrorAction SilentlyContinue
    }
    $result | ConvertTo-Json | Set-Content -Path $Output -Encoding UTF8
}

if ($null -ne $result.Error) {
    exit 1
}
