use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub(crate) fn files_match(bundled: &Path, deployed: &Path) -> Result<bool, String> {
    if !deployed.exists() {
        return Ok(false);
    }

    let bundled_file =
        File::open(bundled).map_err(|error| format!("读取安装包内 CLI 失败: {error}"))?;
    let deployed_file =
        File::open(deployed).map_err(|error| format!("读取已部署 CLI 失败: {error}"))?;

    let bundled_len = bundled_file
        .metadata()
        .map_err(|error| format!("读取安装包内 CLI 信息失败: {error}"))?
        .len();
    let deployed_len = deployed_file
        .metadata()
        .map_err(|error| format!("读取已部署 CLI 信息失败: {error}"))?
        .len();
    if bundled_len != deployed_len {
        return Ok(false);
    }

    let mut bundled_reader = BufReader::new(bundled_file);
    let mut deployed_reader = BufReader::new(deployed_file);
    let mut bundled_buffer = [0_u8; 64 * 1024];
    let mut deployed_buffer = [0_u8; 64 * 1024];

    loop {
        let bundled_read = bundled_reader
            .read(&mut bundled_buffer)
            .map_err(|error| format!("读取安装包内 CLI 失败: {error}"))?;
        let deployed_read = deployed_reader
            .read(&mut deployed_buffer)
            .map_err(|error| format!("读取已部署 CLI 失败: {error}"))?;

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

#[cfg(test)]
mod tests {
    use super::files_match;
    use std::fs;

    #[test]
    fn reports_matching_cli_files_as_current() {
        let temp_dir =
            std::env::temp_dir().join(format!("ssh-router-cli-sync-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let bundled = temp_dir.join("bundled.exe");
        let deployed = temp_dir.join("deployed.exe");
        fs::write(&bundled, b"same cli bytes").unwrap();
        fs::write(&deployed, b"same cli bytes").unwrap();

        assert!(files_match(&bundled, &deployed).unwrap());

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn reports_missing_deployed_cli_as_not_current() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ssh-router-cli-sync-missing-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let bundled = temp_dir.join("bundled.exe");
        let deployed = temp_dir.join("deployed.exe");
        fs::write(&bundled, b"bundled cli").unwrap();

        assert!(!files_match(&bundled, &deployed).unwrap());

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn reports_changed_cli_contents_as_not_current() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ssh-router-cli-sync-changed-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let bundled = temp_dir.join("bundled.exe");
        let deployed = temp_dir.join("deployed.exe");
        fs::write(&bundled, b"new cli bytes").unwrap();
        fs::write(&deployed, b"old cli bytes").unwrap();

        assert!(!files_match(&bundled, &deployed).unwrap());

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn reports_changed_cli_length_as_not_current() {
        let temp_dir =
            std::env::temp_dir().join(format!("ssh-router-cli-sync-length-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let bundled = temp_dir.join("bundled.exe");
        let deployed = temp_dir.join("deployed.exe");
        fs::write(&bundled, b"new cli with more bytes").unwrap();
        fs::write(&deployed, b"old cli").unwrap();

        assert!(!files_match(&bundled, &deployed).unwrap());

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn reports_missing_bundled_cli_as_an_error() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ssh-router-cli-sync-bundle-missing-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let bundled = temp_dir.join("bundled.exe");
        let deployed = temp_dir.join("deployed.exe");
        fs::write(&deployed, b"deployed cli").unwrap();

        let error = files_match(&bundled, &deployed).unwrap_err();
        assert!(error.contains("读取安装包内 CLI 失败"));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn reports_unreadable_deployed_cli_as_an_error() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ssh-router-cli-sync-unreadable-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let bundled = temp_dir.join("bundled.exe");
        let deployed = temp_dir.join("deployed.exe");
        fs::create_dir(&deployed).unwrap();
        let deployed_len = fs::metadata(&deployed).unwrap().len() as usize;
        fs::write(&bundled, vec![0_u8; deployed_len]).unwrap();

        let error = files_match(&bundled, &deployed).unwrap_err();
        assert!(error.contains("读取已部署 CLI"));

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
