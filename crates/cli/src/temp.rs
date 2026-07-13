use std::fs;
use std::path::Path;

/// 启动期清理：删除之前进程残留的临时脚本（被强杀时不会执行 finally 的 File.Delete）
/// 文件以 PID 命名，排除当前 PID 即无跨进程竞态
/// 移植自 C# CleanStaleTempFiles
pub fn clean_stale_temp_files(pid: u32) {
    let tmp = std::env::temp_dir();
    let pid_prefix = format!("ssh-cmd-{}.", pid);

    for pattern in &["ssh-cmd-*.ps1", "ssh-cmd-*.sh"] {
        if let Ok(entries) = fs::read_dir(&tmp) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("ssh-cmd-") && !name.starts_with(&pid_prefix) {
                    let path = entry.path();
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_clean_excludes_current_pid() {
        let tmp = std::env::temp_dir();
        let current_file = tmp.join("ssh-cmd-99999.sh");
        let stale_file = tmp.join("ssh-cmd-88888.sh");

        fs::write(&current_file, "test").unwrap();
        fs::write(&stale_file, "test").unwrap();

        clean_stale_temp_files(99999);

        assert!(current_file.exists(), "current PID file should survive");
        assert!(!stale_file.exists(), "stale file should be deleted");

        // cleanup
        let _ = fs::remove_file(&current_file);
    }
}
