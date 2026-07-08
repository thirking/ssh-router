using System;
using System.Diagnostics;
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

    const int JobObjectExtendedLimitInformation = 9;
    const uint JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x2000;
    const uint INFINITE = 0xFFFFFFFF;

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

    static IntPtr CreateKillOnCloseJob()
    {
        IntPtr hJob = CreateJobObject(IntPtr.Zero, null);
        if (hJob == IntPtr.Zero) return IntPtr.Zero;
        var info = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        int size = Marshal.SizeOf(info);
        IntPtr ptr = Marshal.AllocHGlobal(size);
        try { Marshal.StructureToPtr(info, ptr, false);
              SetInformationJobObject(hJob, JobObjectExtendedLimitInformation, ptr, (uint)size); }
        finally { Marshal.FreeHGlobal(ptr); }
        return hJob;
    }

    static int Main(string[] args)
    {
        IntPtr hJob = CreateKillOnCloseJob();

        string sshConn = Environment.GetEnvironmentVariable("SSH_CONNECTION") ?? "";
        string port = "";
        string[] parts = sshConn.Split(' ');
        if (parts.Length >= 4) port = parts[3];

        string cmdLine;

        if (args.Length >= 2 && args[0] == "-c")
        {
            string command = args[1];

            if (command.Contains("sftp-server"))
            {
                cmdLine = "cmd.exe /c \"C:\\Windows\\System32\\OpenSSH\\sftp-server.exe\"";
            }
            else
            {
                string singleQuoted = "'" + command.Replace("'", "'\\''") + "'";
                if (port == "2223")
                    cmdLine = "wsl.exe -d Ubuntu -- bash -c " + singleQuoted;
                else if (port == "2222")
                    cmdLine = "\"C:\\Program Files\\Git\\usr\\bin\\bash.exe\" -l -c " + singleQuoted;
                else
                {
                    string escaped = command.Replace("\"", "\\\"");
                    cmdLine = "\"C:\\Program Files\\PowerShell\\7\\pwsh.exe\" -c \"" + escaped + "\"";
                }
            }
        }
        else
        {
            if (port == "2223")
                cmdLine = "wsl.exe -d Ubuntu -- bash -l";
            else if (port == "2222")
                cmdLine = "\"C:\\Program Files\\Git\\usr\\bin\\bash.exe\" -l";
            else
                cmdLine = "\"C:\\Program Files\\PowerShell\\7\\pwsh.exe\" -l";
        }

        // 用 CreateProcessW，bInheritHandles=true，不用 STARTF_USESTDHANDLES
        // 让子进程自动继承 stdin/stdout/stderr
        var si = new STARTUPINFO();
        si.cb = Marshal.SizeOf(si);
        // 不设 dwFlags，不设 hStdInput/Output/Error
        // CreateProcess 会自动继承父进程的句柄

        PROCESS_INFORMATION pi;
        bool ok = CreateProcess(null, cmdLine, IntPtr.Zero, IntPtr.Zero,
            true, 0, IntPtr.Zero, null, ref si, out pi);

        if (!ok) return 1;

        if (hJob != IntPtr.Zero)
            AssignProcessToJobObject(hJob, pi.hProcess);

        WaitForSingleObject(pi.hProcess, INFINITE);
        uint code;
        GetExitCodeProcess(pi.hProcess, out code);
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);

        if (hJob != IntPtr.Zero)
            CloseHandle(hJob);

        return (int)code;
    }
}
