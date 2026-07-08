using System;
using System.Diagnostics;

class SshRouter
{
    static int Main(string[] args)
    {
        string sshConn = Environment.GetEnvironmentVariable("SSH_CONNECTION") ?? "";
        string port = "";
        string[] parts = sshConn.Split(' ');
        if (parts.Length >= 4) port = parts[3];

        string fileName;
        string arguments;

        if (args.Length >= 2 && args[0] == "-c")
        {
            // exec 模式（ssh user@host "command"）
            string command = args[1];
            // 用单引号包裹命令，防止 $var 被 bash 双引号展开
            string singleQuoted = "'" + command.Replace("'", "'\\''") + "'";

            if (port == "2223")
            {
                fileName = "wsl.exe";
                arguments = "-d Ubuntu -- bash -c " + singleQuoted;
            }
            else if (port == "2222")
            {
                fileName = @"C:\Program Files\Git\usr\bin\bash.exe";
                arguments = "-l -c " + singleQuoted;
            }
            else
            {
                fileName = @"C:\Program Files\PowerShell\7\pwsh.exe";
                // PowerShell 用双引号包裹
                string escaped = command.Replace("\"", "\\\"");
                arguments = "-c \"" + escaped + "\"";
            }
        }
        else
        {
            // 交互式模式（ssh user@host，不带命令）
            if (port == "2223")
            {
                fileName = "wsl.exe";
                arguments = "-d Ubuntu -- bash -l";
            }
            else if (port == "2222")
            {
                fileName = @"C:\Program Files\Git\usr\bin\bash.exe";
                arguments = "-l";
            }
            else
            {
                fileName = @"C:\Program Files\PowerShell\7\pwsh.exe";
                arguments = "-l";
            }
        }

        var psi = new ProcessStartInfo
        {
            FileName = fileName,
            Arguments = arguments,
            UseShellExecute = false
        };

        var proc = Process.Start(psi);
        proc.WaitForExit();
        return proc.ExitCode;
    }
}
