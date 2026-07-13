$lines = Get-Content C:\ProgramData\ssh\sshd_config
$new = @()
$inserted = $false
foreach ($line in $lines) {
    $new += $line
    if ($line -match '^Port 2222$' -and -not $inserted) {
        $new += 'Port 2223'
        $inserted = $true
    }
}
Set-Content C:\ProgramData\ssh\sshd_config -Value $new -Encoding UTF8
Get-Content C:\ProgramData\ssh\sshd_config
