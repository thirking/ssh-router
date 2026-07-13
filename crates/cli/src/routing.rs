use ssh_router_config::{Config, Route};

/// 判断命令是否为 SFTP 子系统调用
/// 移植自 C# command.Contains("sftp-server")
pub fn is_sftp_command(command: &str) -> bool {
    command.contains("sftp-server")
}

/// 根据端口匹配路由规则
/// 优先级: 精确匹配 > default route
/// 多条 default 时取第一条（记 WARN 由调用方处理）
/// 移植自 C# port == "22" / "2222" / "2223" 判断逻辑
pub fn match_route<'a>(config: &'a Config, port: &str) -> Option<&'a Route> {
    // 精确端口匹配
    let port_num: u16 = port.parse().unwrap_or(0);
    for route in &config.routes {
        if route.port == port_num {
            return Some(route);
        }
    }
    // 回退到 default route
    config.routes.iter().find(|r| r.default)
}

/// 渲染命令模板，替换占位符
/// {shell} → route.shell
/// {tmpfile} → 临时文件 Windows 路径
/// {tmpfile_wsl} → 临时文件 WSL 路径
pub fn render_template(
    template: &str,
    shell: &str,
    tmpfile: Option<&str>,
    tmpfile_wsl: Option<&str>,
) -> String {
    let mut result = template.replace("{shell}", shell);
    if let Some(tf) = tmpfile {
        result = result.replace("{tmpfile}", tf);
    }
    if let Some(tfw) = tmpfile_wsl {
        result = result.replace("{tmpfile_wsl}", tfw);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ssh_router_config::Config;

    #[test]
    fn test_is_sftp_command() {
        assert!(is_sftp_command("C:\\Windows\\System32\\OpenSSH\\sftp-server.exe"));
        assert!(!is_sftp_command("uname -s"));
    }

    #[test]
    fn test_match_route_exact_port() {
        let config = Config::default_config();
        let route = match_route(&config, "2222").unwrap();
        assert_eq!(route.port, 2222);
        assert_eq!(route.name, "Git Bash");
    }

    #[test]
    fn test_match_route_fallback_default() {
        let config = Config::default_config();
        // 端口 9999 不在配置中，回退到 default (port 22)
        let route = match_route(&config, "9999").unwrap();
        assert_eq!(route.port, 22);
        assert_eq!(route.name, "PowerShell");
    }

    #[test]
    fn test_match_route_no_default_returns_none() {
        let mut config = Config::default_config();
        for route in &mut config.routes {
            route.default = false;
        }
        assert!(match_route(&config, "9999").is_none());
    }

    #[test]
    fn test_match_route_empty_port() {
        let config = Config::default_config();
        // 空端口字符串 parse 失败 → port_num=0 → 精确匹配失败 → 回退 default
        let route = match_route(&config, "").unwrap();
        assert_eq!(route.port, 22);
    }

    #[test]
    fn test_render_template_with_shell_only() {
        let result = render_template("\"{shell}\" -l", "pwsh.exe", None, None);
        assert_eq!(result, "\"pwsh.exe\" -l");
    }

    #[test]
    fn test_render_template_with_tmpfile() {
        let result = render_template(
            "\"{shell}\" -File \"{tmpfile}\"",
            "pwsh.exe",
            Some("C:\\tmp\\ssh-cmd-123.ps1"),
            None,
        );
        assert_eq!(result, "\"pwsh.exe\" -File \"C:\\tmp\\ssh-cmd-123.ps1\"");
    }

    #[test]
    fn test_render_template_with_tmpfile_wsl() {
        let result = render_template(
            "wsl.exe -- bash -c '. \"{tmpfile_wsl}\"'",
            "wsl.exe",
            None,
            Some("/mnt/c/tmp/ssh-cmd-123.sh"),
        );
        assert_eq!(result, "wsl.exe -- bash -c '. \"/mnt/c/tmp/ssh-cmd-123.sh\"'");
    }

    #[test]
    fn test_render_template_all_placeholders() {
        let result = render_template(
            "{shell} {tmpfile} {tmpfile_wsl}",
            "bash",
            Some("C:\\f.sh"),
            Some("/mnt/c/f.sh"),
        );
        assert_eq!(result, "bash C:\\f.sh /mnt/c/f.sh");
    }
}
