param([Parameter(Mandatory=$true)][string]$Executable,[string]$ResultFile)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class StartupWindow {
 public delegate bool EnumProc(IntPtr h, IntPtr d);
 [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
 [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr d);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder t, int n);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll",EntryPoint="PostMessageW",SetLastError=true)] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
 [DllImport("user32.dll",EntryPoint="SendMessageW")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
 [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref Point p);
 [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
 public static IntPtr Find(int pid) {
  IntPtr found=IntPtr.Zero;
  EnumWindows((h,d)=>{uint owner;GetWindowThreadProcessId(h,out owner);if(owner!=(uint)pid)return true;
   var title=new StringBuilder(128);GetWindowText(h,title,128);if(title.ToString()!="SSH Router")return true;
   found=h;return false;},IntPtr.Zero);
  return found;
 }
 public static int Hit(IntPtr h, double x, double y) {
  double scale=GetDpiForWindow(h)/96.0;
  var p=new Point {X=(int)(x*scale),Y=(int)(y*scale)};ClientToScreen(h,ref p);
  return SendMessage(h,0x84,IntPtr.Zero,new IntPtr((p.Y<<16)|(p.X&0xffff))).ToInt32();
 }
}
'@
function Await([scriptblock]$Condition, [string]$Message) {
 $deadline=[DateTime]::UtcNow.AddSeconds(15)
 do { if (& $Condition) { return }; Start-Sleep -Milliseconds 100 } while([DateTime]::UtcNow -lt $deadline)
 throw $Message
}
[void][StartupWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$temporary=Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
New-Item -ItemType Directory $temporary|Out-Null
$names=@('SSH_ROUTER_SHOW','SSH_ROUTER_STARTUP_MARKER','SSH_ROUTER_INSTANCE_NAME','SSH_ROUTER_CONFIG','SSH_ROUTER_WIDTH','SSH_ROUTER_HEIGHT','GPUIX_BACKGROUND')
$previous=@{}
foreach($name in $names){$previous[$name]=[Environment]::GetEnvironmentVariable($name);[Environment]::SetEnvironmentVariable($name,$null)}
$env:SSH_ROUTER_INSTANCE_NAME="Local\SSHRouterStartup-$PID"
$env:SSH_ROUTER_CONFIG=Join-Path $temporary 'config.json'
$result=@{Success=$false;Error=$null}
$primary=$null
try {
 $primary=Start-Process -FilePath $Executable -PassThru
 Await { $primary.Refresh();if($primary.HasExited){throw 'GUI exited during startup'};$script:window=[StartupWindow]::Find($primary.Id);$script:window -ne [IntPtr]::Zero } 'Main window not created'
 $result.ProcessId=$primary.Id
 Await { $script:window=[StartupWindow]::Find($primary.Id);$script:window -ne [IntPtr]::Zero -and [StartupWindow]::Hit($script:window,100,16) -eq 2 } 'Titlebar did not initialize'
 Await { -not [StartupWindow]::IsWindowVisible($script:window) } 'Default startup is not hidden'
 [void][StartupWindow]::SendMessage($script:window,1066,[IntPtr]::Zero,[IntPtr]0x0203)
 Await { [StartupWindow]::IsWindowVisible($script:window) } 'Tray did not restore window'
 Await { [StartupWindow]::Hit($script:window,100,16) -eq 2 } 'Titlebar did not render after hidden startup'
 $rect=New-Object StartupWindow+Rect
 [void][StartupWindow]::GetClientRect($script:window,[ref]$rect)
 $scale=[StartupWindow]::GetDpiForWindow($script:window)/96.0
 if([Math]::Abs($rect.Right/$scale-620) -gt 1){throw "Unexpected client width: $($rect.Right/$scale)"}
 if([StartupWindow]::Hit($script:window,600,16) -ne 1){throw 'Close button belongs to drag region'}
 [void][StartupWindow]::PostMessage($script:window,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Await { -not [StartupWindow]::IsWindowVisible($script:window) } 'Close did not hide window'
 $primary.Refresh();if($primary.HasExited){throw 'Close exited the GUI'}
 $result.Success=$true
 Write-Output 'WINDOWS_HIDDEN_STARTUP_OK'
} catch {
 $result.Error=$_.Exception.Message
 throw
} finally {
 if($primary){
  Get-CimInstance Win32_Process -Filter "ParentProcessId = $($primary.Id)"|Where-Object Name -eq 'ssh-router-backend.exe'|ForEach-Object {Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue}
  $primary.Refresh()
  if(-not $primary.HasExited){Stop-Process -Id $primary.Id -Force;$primary.WaitForExit()}
  $primary.Dispose()
 }
 foreach($name in $names){[Environment]::SetEnvironmentVariable($name,$previous[$name])}
 for($attempt=0;$attempt -lt 20 -and (Test-Path $temporary);$attempt++){
  Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
  if(Test-Path $temporary){Start-Sleep -Milliseconds 100}
 }
 if($ResultFile){$result|ConvertTo-Json|Set-Content -LiteralPath $ResultFile -Encoding UTF8}
}
