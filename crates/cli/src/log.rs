use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const LOG_FILE: &str = r"C:\ProgramData\ssh\ssh-router-debug.log";

/// 写一条日志到 ssh-router-debug.log，格式: "yyyy-MM-dd HH:mm:ss.fff <msg>"
/// 与原 C# Log 函数行为一致，失败时静默忽略
pub fn log(msg: &str) {
    let entry = format!(
        "{} {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        msg
    );
    let _ = write_entry(&entry);
}

fn write_entry(entry: &str) -> std::io::Result<()> {
    let path = Path::new(LOG_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    file.write_all(entry.as_bytes())?;
    Ok(())
}
