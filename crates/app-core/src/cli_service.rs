use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::error::{AppResult, CliError};

pub fn files_match(bundled: &Path, deployed: &Path) -> AppResult<bool> {
    if !deployed.try_exists().map_err(|source| CliError::Read {
        path: deployed.to_path_buf(),
        source,
    })? {
        return Ok(false);
    }

    let bundled_file = File::open(bundled).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            CliError::BundledCliMissing {
                path: bundled.to_path_buf(),
            }
        } else {
            CliError::Read {
                path: bundled.to_path_buf(),
                source,
            }
        }
    })?;
    let deployed_file = File::open(deployed).map_err(|source| CliError::Read {
        path: deployed.to_path_buf(),
        source,
    })?;

    let bundled_length = bundled_file
        .metadata()
        .map_err(|source| CliError::Read {
            path: bundled.to_path_buf(),
            source,
        })?
        .len();
    let deployed_length = deployed_file
        .metadata()
        .map_err(|source| CliError::Read {
            path: deployed.to_path_buf(),
            source,
        })?
        .len();
    if bundled_length != deployed_length {
        return Ok(false);
    }

    let mut bundled_reader = BufReader::new(bundled_file);
    let mut deployed_reader = BufReader::new(deployed_file);
    let mut bundled_buffer = [0_u8; 64 * 1024];
    let mut deployed_buffer = [0_u8; 64 * 1024];

    loop {
        let bundled_read =
            bundled_reader
                .read(&mut bundled_buffer)
                .map_err(|source| CliError::Read {
                    path: bundled.to_path_buf(),
                    source,
                })?;
        let deployed_read = deployed_reader
            .read(&mut deployed_buffer)
            .map_err(|source| CliError::Read {
                path: deployed.to_path_buf(),
                source,
            })?;
        if bundled_read != deployed_read
            || bundled_buffer[..bundled_read] != deployed_buffer[..deployed_read]
        {
            return Ok(false);
        }
        if bundled_read == 0 {
            return Ok(true);
        }
    }
}

pub(crate) fn install_cli_script(source: &Path, destination: &Path) -> AppResult<String> {
    let destination_directory = destination.parent().ok_or_else(|| CliError::Deploy {
        message: format!("CLI 目标路径缺少父目录: {}", destination.display()),
    })?;
    let source = powershell_literal(source);
    let destination = powershell_literal(destination);
    let destination_directory = powershell_literal(destination_directory);

    Ok(format!(
        r#"$src = '{source}'
$dst = '{destination}'
$dstDir = '{destination_directory}'
if (-not (Test-Path -LiteralPath $dstDir)) {{
    New-Item -ItemType Directory -Path $dstDir -Force | Out-Null
}}
$staged = Join-Path $dstDir ('.ssh-router-cli-' + [Guid]::NewGuid().ToString('N') + '.exe.tmp')
$operationError = $null
try {{
    Copy-Item -LiteralPath $src -Destination $staged -Force
    if (Test-Path -LiteralPath $dst) {{
        # NullString 保留 .NET null，避免备份路径被转换为空字符串。
        [System.IO.File]::Replace($staged, $dst, [NullString]::Value, $true)
    }} else {{
        [System.IO.File]::Move($staged, $dst)
    }}
    & icacls $dst /reset | Out-Null
    if ($LASTEXITCODE -ne 0) {{ throw "icacls reset failed: $LASTEXITCODE" }}
    & icacls $dst /grant 'SYSTEM:(RX)' 'Administrators:(F)' 'Users:(RX)' | Out-Null
    if ($LASTEXITCODE -ne 0) {{ throw "icacls grant failed: $LASTEXITCODE" }}
}} catch {{
    $operationError = $_.Exception.Message
    throw
}} finally {{
    if (Test-Path -LiteralPath $staged) {{
        try {{ Remove-Item -LiteralPath $staged -Force -ErrorAction Stop }}
        catch {{
            if ($operationError) {{ throw "$operationError; cleanup failed: $($_.Exception.Message)" }}
            throw
        }}
    }}
}}
"#
    ))
}

fn powershell_literal(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ssh-router-app-core-{name}-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn reports_matching_files() {
        let directory = temp_directory("matching");
        fs::create_dir_all(&directory).expect("测试目录应可创建");
        let bundled = directory.join("bundled.exe");
        let deployed = directory.join("deployed.exe");
        fs::write(&bundled, b"same cli").expect("测试文件应可写入");
        fs::write(&deployed, b"same cli").expect("测试文件应可写入");

        assert!(files_match(&bundled, &deployed).expect("文件应可比较"));

        fs::remove_dir_all(directory).expect("测试目录应可清理");
    }

    #[test]
    fn reports_changed_files() {
        let directory = temp_directory("changed");
        fs::create_dir_all(&directory).expect("测试目录应可创建");
        let bundled = directory.join("bundled.exe");
        let deployed = directory.join("deployed.exe");
        fs::write(&bundled, b"new cli").expect("测试文件应可写入");
        fs::write(&deployed, b"old cli").expect("测试文件应可写入");

        assert!(!files_match(&bundled, &deployed).expect("文件应可比较"));

        fs::remove_dir_all(directory).expect("测试目录应可清理");
    }

    #[test]
    fn reports_changed_file_length() {
        let directory = temp_directory("changed-length");
        fs::create_dir_all(&directory).expect("测试目录应可创建");
        let bundled = directory.join("bundled.exe");
        let deployed = directory.join("deployed.exe");
        fs::write(&bundled, b"new cli with more bytes").expect("测试文件应可写入");
        fs::write(&deployed, b"old cli").expect("测试文件应可写入");

        assert!(!files_match(&bundled, &deployed).expect("文件应可比较"));

        fs::remove_dir_all(directory).expect("测试目录应可清理");
    }

    #[test]
    fn reports_missing_deployed_file_as_outdated() {
        let directory = temp_directory("missing-deployed");
        fs::create_dir_all(&directory).expect("测试目录应可创建");
        let bundled = directory.join("bundled.exe");
        fs::write(&bundled, b"new cli").expect("测试文件应可写入");

        assert!(
            !files_match(&bundled, &directory.join("missing.exe")).expect("缺失部署文件应返回过期")
        );

        fs::remove_dir_all(directory).expect("测试目录应可清理");
    }

    #[test]
    fn reports_missing_bundled_file_as_error() {
        let directory = temp_directory("missing-bundled");
        fs::create_dir_all(&directory).expect("测试目录应可创建");
        let deployed = directory.join("deployed.exe");
        fs::write(&deployed, b"old cli").expect("测试文件应可写入");

        let error = files_match(&directory.join("missing.exe"), &deployed)
            .expect_err("安装包 CLI 缺失应失败");

        assert!(matches!(
            error,
            crate::error::AppError::Cli(CliError::BundledCliMissing { .. })
        ));
        fs::remove_dir_all(directory).expect("测试目录应可清理");
    }

    #[test]
    fn reports_unreadable_deployed_file_as_error() {
        let directory = temp_directory("unreadable-deployed");
        fs::create_dir_all(&directory).expect("测试目录应可创建");
        let bundled = directory.join("bundled.exe");
        let deployed = directory.join("deployed.exe");
        fs::create_dir(&deployed).expect("测试目录应可创建");
        let deployed_length = fs::metadata(&deployed).expect("测试目录信息应可读取").len() as usize;
        fs::write(&bundled, vec![0_u8; deployed_length]).expect("测试文件应可写入");

        let error = files_match(&bundled, &deployed).expect_err("目录不能作为 CLI 读取");

        assert!(matches!(
            error,
            crate::error::AppError::Cli(CliError::Read { .. })
        ));
        fs::remove_dir_all(directory).expect("测试目录应可清理");
    }

    #[test]
    fn install_script_uses_atomic_replace_and_checks_acl() {
        let script = install_cli_script(
            Path::new(r"C:\Program Files\SSH Router\resources\ssh-router-cli.exe"),
            Path::new(r"C:\ProgramData\ssh\ssh-router-cli.exe"),
        )
        .expect("脚本应生成");

        assert!(script.contains("[System.IO.File]::Replace"));
        assert!(script.contains("$LASTEXITCODE"));
        assert!(!script.contains("SilentlyContinue"));
    }

    #[cfg(windows)]
    #[test]
    fn install_script_replaces_existing_cli_under_windows_powershell() {
        let directory = tempfile::tempdir().expect("测试目录应可创建");
        let bundled_directory = directory.path().join("bundled 包's");
        let deployed_directory = directory.path().join("deployed 包's");
        fs::create_dir_all(&bundled_directory).unwrap();
        fs::create_dir_all(&deployed_directory).unwrap();
        let source = bundled_directory.join("ssh-router-cli.exe");
        let destination = deployed_directory.join("ssh-router-cli.exe");
        fs::write(&source, b"new cli").unwrap();
        fs::write(&destination, b"old cli").unwrap();
        let script = install_cli_script(&source, &destination).unwrap();
        let wrapper = format!(
            "$ErrorActionPreference = 'Stop'\n[Console]::OutputEncoding = [Text.Encoding]::UTF8\ntry {{\n{script}\n}} catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit 1 }}\n"
        );
        let path = directory.path().join("install.ps1");
        fs::write(&path, format!("\u{feff}{wrapper}")).unwrap();
        let output = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(path)
            .output()
            .expect("Windows PowerShell 应可启动");
        assert!(
            output.status.success(),
            "CLI 替换失败: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&destination).unwrap(), b"new cli");
        assert_eq!(fs::read_dir(&deployed_directory).unwrap().count(), 1);
    }
}
