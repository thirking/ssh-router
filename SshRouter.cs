using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

class SshRouter
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr CreateJobObject(IntPtr lpJobAttributes, string lpName);

    [DllImport("kernel32.dll")]
    static extern bool SetInformationJobObject(IntPtr hJob, int infoType, IntPtr lpJobObjectInfo, uint cbJobObjectInfoLength);

    [DllImport("kernel32.dll")]
    static extern bool AssignProcessToJobObject(IntPtr hJob, IntPtr hProcess);

    [DllImport("kernel32.dll")]
    static extern bool CloseHandle(IntPtr hObject);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool CreateProcess(
        string lpApplicationName, string lpCommandLine,
        IntPtr lpProcessAttributes, IntPtr lpThreadAttributes,
        bool bInheritHandles, uint dwCreationFlags,
        IntPtr lpEnvironment, string lpCurrentDirectory,
        ref STARTUPINFO lpStartupInfo, out PROCESS_INFORMATION lpProcessInformation);

    [DllImport("kernel32.dll")]
    static extern uint WaitForSingleObject(IntPtr hHandle, uint dwMilliseconds);

    [DllImport("kernel32.dll")]
    static extern bool GetExitCodeProcess(IntPtr hProcess, out uint lpExitCode);

    [DllImport("kernel32.dll")]
    static extern uint ResumeThread(IntPtr hThread);

    const int JobObjectExtendedLimitInformation = 9;
    const uint JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x2000;
    const uint INFINITE = 0xFFFFFFFF;
    const uint CREATE_SUSPENDED = 0x00000004;
    const uint WAIT_FAILED = 0xFFFFFFFF;

    [StructLayout(LayoutKind.Sequential)]
    struct STARTUPINFO
    {
        public int cb;
        public string lpReserved, lpDesktop, lpTitle;
        public uint dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags;
        public short wShowWindow, cbReserved2;
        public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError;
    }

    [StructLayout(LayoutKind.Sequential)]
    struct PROCESS_INFORMATION
    {
        public IntPtr hProcess, hThread;
        public uint dwProcessId, dwThreadId;
    }

    [StructLayout(LayoutKind.Sequential)]
    struct JOBOBJECT_BASIC_LIMIT_INFORMATION
    {
        public long PerProcessUserTimeLimit, PerJobUserTimeLimit;
        public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize, MaximumWorkingSetSize;
        public uint ActiveProcessLimit;
        public long Affinity;
        public uint PriorityClass, SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    struct IO_COUNTERS
    {
        public ulong ReadOperationCount, WriteOperationCount, OtherOperationCount;
        public ulong ReadTransferCount, WriteTransferCount, OtherTransferCount;
    }

    [StructLayout(LayoutKind.Sequential)]
    struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION
    {
        public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation;
        public IO_COUNTERS IoInfo;
        public UIntPtr ProcessMemoryLimit, JobMemoryLimit, PeakProcessMemoryUsed, PeakJobMemoryUsed;
    }

    static string LogFile = @"C:\ProgramData\ssh\ssh-router-debug.log";

    static void Log(string msg)
    {
        try
        {
            string entry = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss.fff") + " " + msg + Environment.NewLine;
            File.AppendAllText(LogFile, entry);
        }
        catch { }
    }

    static IntPtr CreateKillOnCloseJob()
    {
        IntPtr hJob = CreateJobObject(IntPtr.Zero, null);
        if (hJob == IntPtr.Zero)
        {
            Log("WARN: CreateJobObject failed, last error: " + Marshal.GetLastWin32Error());
            return IntPtr.Zero;
        }
        var info = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        int size = Marshal.SizeOf(info);
        IntPtr ptr = Marshal.AllocHGlobal(size);
        try
        {
            Marshal.StructureToPtr(info, ptr, false);
            if (!SetInformationJobObject(hJob, JobObjectExtendedLimitInformation, ptr, (uint)size))
            {
                Log("WARN: SetInformationJobObject failed, last error: " + Marshal.GetLastWin32Error());
                CloseHandle(hJob);
                return IntPtr.Zero;
            }
        }
        finally { Marshal.FreeHGlobal(ptr); }
        return hJob;
    }

    // 启动期清理：删除之前进程残留的临时脚本（被强杀时下方 File.Delete 不会执行）
    // 文件以 PID 命名，排除当前 PID 即无跨进程竞态
    static void CleanStaleTempFiles()
    {
        try
        {
            string pid = Process.GetCurrentProcess().Id.ToString();
            string tmp = Path.GetTempPath();
            foreach (string pat in new[] { "ssh-cmd-*.ps1", "ssh-cmd-*.sh" })
            {
                foreach (string f in Directory.GetFiles(tmp, pat))
                {
                    if (Path.GetFileName(f).StartsWith("ssh-cmd-" + pid + ".")) continue;
                    try { File.Delete(f); } catch { }
                }
            }
        }
        catch { }
    }

    // Windows 路径转 WSL 路径: C:\Users\xxx → /mnt/c/Users/xxx
    static string ToWslPath(string winPath)
    {
        if (winPath.Length >= 2 && winPath[1] == ':')
        {
            char drive = char.ToLower(winPath[0]);
            string rest = winPath.Substring(2).Replace('\\', '/');
            return "/mnt/" + drive + rest;
        }
        return winPath.Replace('\\', '/');
    }

    static int Main(string[] args)
    {
        CleanStaleTempFiles();
        IntPtr hJob = CreateKillOnCloseJob();

        string sshConn = Environment.GetEnvironmentVariable("SSH_CONNECTION") ?? "";
        string port = "";
        string[] parts = sshConn.Split(new char[] { ' ' });
        if (parts.Length >= 4) port = parts[3];

        // 记录调试日志
        Log("========");
        Log("args: " + string.Join(", ", Array.ConvertAll(args, a => "\"" + a + "\"")));
        Log("port: " + port);

        string cmdLine;
        string tempFile = null;

        if (args.Length >= 2 && args[0] == "-c")
        {
            string command = args[1];
            Log("command: " + command);

            if (command.Contains("sftp-server"))
            {
                cmdLine = "cmd.exe /c \"C:\\Windows\\System32\\OpenSSH\\sftp-server.exe\"";
            }
            else
            {
                // 将命令原样写入临时文件，避免 shell quoting 破坏复杂脚本
                // (多行脚本、here-document、嵌套引号、sh -c 等)
                bool isPwsh = (port != "2222" && port != "2223");
                string ext = isPwsh ? ".ps1" : ".sh";
                try
                {
                    tempFile = Path.Combine(Path.GetTempPath(),
                        "ssh-cmd-" + Process.GetCurrentProcess().Id + ext);
                    File.WriteAllText(tempFile, command);
                }
                catch (Exception ex)
                {
                    Log("ERROR: failed to create temp file: " + ex.Message);
                    return 1;
                }

                if (port == "2223")
                {
                    string wslPath = ToWslPath(tempFile);
                    // cd 到 WSL 原生 home 目录，避免 cwd 在 /mnt/c 上导致
                    // inotify 走 9P 协议性能极差
                    cmdLine = "wsl.exe -d Ubuntu -- bash -c 'cd ~ && . \"" + wslPath + "\"'";
                }
                else if (port == "2222")
                {
                    cmdLine = "\"C:\\Program Files\\Git\\usr\\bin\\bash.exe\" -l -c '. \"" + tempFile + "\"'";
                }
                else
                {
                    cmdLine = "\"C:\\Program Files\\PowerShell\\7\\pwsh.exe\" -File \"" + tempFile + "\"";
                }
            }
        }
        else
        {
            if (port == "2223")
                cmdLine = "wsl.exe -d Ubuntu -- bash -lc 'cd ~ && exec bash -l'";
            else if (port == "2222")
                cmdLine = "\"C:\\Program Files\\Git\\usr\\bin\\bash.exe\" -l";
            else
                cmdLine = "\"C:\\Program Files\\PowerShell\\7\\pwsh.exe\" -l";
        }

        Log("cmdLine: " + cmdLine);

        // 用 CreateProcessW，bInheritHandles=true，不用 STARTF_USESTDHANDLES
        // 让子进程自动继承 stdin/stdout/stderr
        var si = new STARTUPINFO();
        si.cb = Marshal.SizeOf(si);
        // 不设 dwFlags，不设 hStdInput/Output/Error
        // CreateProcess 会自动继承父进程的句柄

        PROCESS_INFORMATION pi;
        bool ok = CreateProcess(null, cmdLine, IntPtr.Zero, IntPtr.Zero,
            true, CREATE_SUSPENDED, IntPtr.Zero, null, ref si, out pi);

        if (!ok)
        {
            Log("ERROR: CreateProcess failed, last error: " + Marshal.GetLastWin32Error());
            if (tempFile != null) try { File.Delete(tempFile); } catch { }
            if (hJob != IntPtr.Zero) CloseHandle(hJob);
            return 1;
        }

        uint code = 1;
        try
        {
            // CREATE_SUSPENDED 启动后、恢复线程前关联 Job，
            // 避免子进程派生孙进程逃逸 Job 导致孤儿进程
            if (hJob != IntPtr.Zero && !AssignProcessToJobObject(hJob, pi.hProcess))
                Log("WARN: AssignProcessToJobObject failed, last error: " + Marshal.GetLastWin32Error());
            ResumeThread(pi.hThread);

            if (WaitForSingleObject(pi.hProcess, INFINITE) == WAIT_FAILED)
                Log("WARN: WaitForSingleObject failed, last error: " + Marshal.GetLastWin32Error());

            if (!GetExitCodeProcess(pi.hProcess, out code))
            {
                Log("WARN: GetExitCodeProcess failed, last error: " + Marshal.GetLastWin32Error());
                code = 1;
            }
        }
        finally
        {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
            if (tempFile != null) try { File.Delete(tempFile); } catch { }
            if (hJob != IntPtr.Zero) CloseHandle(hJob);
        }

        Log("exit code: " + code);
        Log("========");

        return (int)code;
    }
}
