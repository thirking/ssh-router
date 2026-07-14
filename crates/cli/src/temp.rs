use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

const STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const CREATE_ATTEMPTS: usize = 10;

/// 单个远程命令脚本的所有权守卫。
pub struct TempScript {
    path: PathBuf,
}

impl TempScript {
    pub fn create(ext: &str, contents: &str) -> io::Result<Self> {
        Self::create_in(&std::env::temp_dir(), ext, contents)
    }

    fn create_in(dir: &Path, ext: &str, contents: &str) -> io::Result<Self> {
        for _ in 0..CREATE_ATTEMPTS {
            let path = dir.join(format!("ssh-cmd-{}{}", Uuid::new_v4(), ext));
            match Self::create_at(path, contents) {
                Ok(script) => return Ok(script),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "failed to allocate a unique temp script name",
        ))
    }

    fn create_at(path: PathBuf, contents: &str) -> io::Result<Self> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
        if let Err(error) = file.write_all(contents.as_bytes()) {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempScript {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// 启动期清理被异常终止进程遗留超过 24 小时的临时脚本。
pub fn clean_stale_temp_files() {
    let tmp = std::env::temp_dir();
    clean_stale_temp_files_in(&tmp, SystemTime::now(), STALE_AFTER);
}

fn clean_stale_temp_files_in(tmp: &Path, now: SystemTime, stale_after: Duration) {
    let Ok(entries) = fs::read_dir(tmp) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_temp_script =
            name.starts_with("ssh-cmd-") && (name.ends_with(".ps1") || name.ends_with(".sh"));
        if !is_temp_script {
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        let is_stale = metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > stale_after);
        if is_stale {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    #[test]
    fn clean_removes_only_expired_temp_scripts() {
        let tmp = std::env::temp_dir().join(format!(
            "ssh-router-clean-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp).unwrap();

        let expired_file = tmp.join("ssh-cmd-expired.sh");
        let unrelated_file = tmp.join("other-file.sh");

        fs::write(&expired_file, "expired").unwrap();
        fs::write(&unrelated_file, "unrelated").unwrap();

        let modified = fs::metadata(&expired_file).unwrap().modified().unwrap();
        let now = modified + Duration::from_secs(25 * 60 * 60);

        clean_stale_temp_files_in(&tmp, now, Duration::from_secs(24 * 60 * 60));

        assert!(!expired_file.exists(), "expired script should be deleted");
        assert!(unrelated_file.exists(), "unrelated file should survive");

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn clean_preserves_temp_scripts_younger_than_threshold() {
        let tmp = std::env::temp_dir().join(format!(
            "ssh-router-fresh-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp).unwrap();
        let fresh_file = tmp.join("ssh-cmd-fresh.ps1");
        fs::write(&fresh_file, "fresh").unwrap();

        let modified = fs::metadata(&fresh_file).unwrap().modified().unwrap();
        let now = modified + Duration::from_secs(23 * 60 * 60);

        clean_stale_temp_files_in(&tmp, now, Duration::from_secs(24 * 60 * 60));

        assert!(fresh_file.exists(), "fresh script should survive");
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn temp_script_deletes_its_own_file_on_drop() {
        let tmp = std::env::temp_dir().join(format!(
            "ssh-router-script-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp).unwrap();
        let unrelated_file = tmp.join("keep.sh");
        fs::write(&unrelated_file, "keep").unwrap();

        let script_path = {
            let script = TempScript::create_in(&tmp, ".sh", "echo test").unwrap();
            let path = script.path().to_path_buf();
            assert_eq!(fs::read_to_string(&path).unwrap(), "echo test");
            assert!(path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("ssh-cmd-"));
            assert_eq!(path.extension().unwrap(), "sh");
            path
        };

        assert!(!script_path.exists(), "owned script should be deleted");
        assert!(unrelated_file.exists(), "unrelated file should survive");

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn temp_scripts_use_unique_guid_names() {
        let tmp = std::env::temp_dir().join(format!(
            "ssh-router-guid-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp).unwrap();

        let first = TempScript::create_in(&tmp, ".sh", "first").unwrap();
        let second = TempScript::create_in(&tmp, ".sh", "second").unwrap();
        assert_ne!(first.path(), second.path());

        for script in [&first, &second] {
            let name = script.path().file_name().unwrap().to_string_lossy();
            let guid = name
                .strip_prefix("ssh-cmd-")
                .unwrap()
                .strip_suffix(".sh")
                .unwrap();
            Uuid::parse_str(guid).expect("temp script name should contain a GUID");
        }

        drop(first);
        drop(second);
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn temp_script_creation_never_overwrites_an_existing_file() {
        let tmp = std::env::temp_dir().join(format!(
            "ssh-router-create-new-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&tmp).unwrap();
        let existing = tmp.join("ssh-cmd-existing.sh");
        fs::write(&existing, "original").unwrap();

        let error = match TempScript::create_at(existing.clone(), "replacement") {
            Ok(_) => panic!("existing file must not be overwritten"),
            Err(error) => error,
        };

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&existing).unwrap(), "original");
        fs::remove_dir_all(&tmp).unwrap();
    }
}
