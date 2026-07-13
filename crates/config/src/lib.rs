use serde::{Deserialize, Serialize};

/// SSH Router 配置，序列化为 ssh-router.json
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub routes: Vec<Route>,
    pub sftp_command: String,
}

/// 单条端口路由规则
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub port: u16,
    pub name: String,
    pub shell: String,
    pub interactive_template: String,
    pub command_template: String,
    pub tmp_file_ext: String,
    #[serde(default)]
    pub default: bool,
}

impl Config {
    /// 返回移植自现有 C# 硬编码的三端口默认配置
    pub fn default_config() -> Self {
        Config {
            routes: vec![
                Route {
                    port: 22,
                    name: "PowerShell".to_string(),
                    shell: "C:\\Program Files\\PowerShell\\7\\pwsh.exe".to_string(),
                    interactive_template: "\"{shell}\" -l".to_string(),
                    command_template: "\"{shell}\" -File \"{tmpfile}\"".to_string(),
                    tmp_file_ext: ".ps1".to_string(),
                    default: true,
                },
                Route {
                    port: 2222,
                    name: "Git Bash".to_string(),
                    shell: "C:\\Program Files\\Git\\usr\\bin\\bash.exe".to_string(),
                    interactive_template: "\"{shell}\" -l".to_string(),
                    command_template: "\"{shell}\" -l -c '. \"{tmpfile}\"'".to_string(),
                    tmp_file_ext: ".sh".to_string(),
                    default: false,
                },
                Route {
                    port: 2223,
                    name: "WSL Ubuntu".to_string(),
                    shell: "wsl.exe".to_string(),
                    interactive_template:
                        "wsl.exe -d Ubuntu -- bash -lc 'cd /home/thirking && exec bash -l'"
                            .to_string(),
                    command_template:
                        "wsl.exe -d Ubuntu -- bash -c 'cd /home/thirking && . \"{tmpfile_wsl}\"'"
                            .to_string(),
                    tmp_file_ext: ".sh".to_string(),
                    default: false,
                },
            ],
            sftp_command: "cmd.exe /c \"C:\\Windows\\System32\\OpenSSH\\sftp-server.exe\""
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_three_routes() {
        let config = Config::default_config();
        assert_eq!(config.routes.len(), 3);
    }

    #[test]
    fn test_default_config_has_exactly_one_default() {
        let config = Config::default_config();
        let defaults: Vec<_> = config.routes.iter().filter(|r| r.default).collect();
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].port, 22);
    }

    #[test]
    fn test_serde_camel_case_roundtrip() {
        let config = Config::default_config();
        let json = serde_json::to_string(&config).unwrap();
        // camelCase 键名
        assert!(json.contains("interactiveTemplate"));
        assert!(json.contains("commandTemplate"));
        assert!(json.contains("tmpFileExt"));
        assert!(json.contains("sftpCommand"));
        // 不含 snake_case
        assert!(!json.contains("interactive_template"));
        assert!(!json.contains("command_template"));

        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.routes.len(), 3);
    }

    #[test]
    fn test_default_field_serde_default() {
        // default 字段缺失时应默认为 false
        let json = r#"{
            "routes": [{
                "port": 22,
                "name": "Test",
                "shell": "pwsh.exe",
                "interactiveTemplate": "\"{shell}\" -l",
                "commandTemplate": "\"{shell}\" -File \"{tmpfile}\"",
                "tmpFileExt": ".ps1"
            }],
            "sftpCommand": "cmd.exe /c sftp-server.exe"
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(!config.routes[0].default);
    }
}
