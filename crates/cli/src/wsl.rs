/// Windows 路径转 WSL 路径: C:\Users\xxx → /mnt/c/Users/xxx
/// 移植自 C# ToWslPath
pub fn to_wsl_path(win: &str) -> String {
    if win.len() >= 2 && win.as_bytes()[1] == b':' {
        let drive = win.as_bytes()[0].to_ascii_lowercase();
        let rest = &win[2..];
        let rest = rest.replace('\\', "/");
        return format!("/mnt/{}{}", drive as char, rest);
    }
    win.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_letter_path() {
        assert_eq!(
            to_wsl_path(r"C:\Users\test"),
            "/mnt/c/Users/test"
        );
    }

    #[test]
    fn test_lowercase_drive() {
        assert_eq!(
            to_wsl_path(r"D:\foo\bar"),
            "/mnt/d/foo/bar"
        );
    }

    #[test]
    fn test_no_drive_letter() {
        assert_eq!(
            to_wsl_path(r"foo\bar\baz"),
            "foo/bar/baz"
        );
    }

    #[test]
    fn test_already_slash() {
        assert_eq!(
            to_wsl_path(r"C:/Users/test"),
            "/mnt/c/Users/test"
        );
    }
}
