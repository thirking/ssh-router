use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// SSH Router 配置，序列化为 ssh-router.json
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub routes: Vec<Route>,
    pub sftp_command: String,
}

/// 单条端口路由规则
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ConfigValidationError {
    #[error("至少需要一条路由")]
    EmptyRoutes,
    #[error("必须恰好有一条默认路由，当前有 {count} 条")]
    DefaultRouteCount { count: usize },
    #[error("端口必须在 1 到 65535 之间: {port}")]
    InvalidPort { port: u16 },
    #[error("端口重复: {port}")]
    DuplicatePort { port: u16 },
    #[error("第 {route_index} 条路由的 {field} 不能为空")]
    EmptyRouteField {
        route_index: usize,
        field: &'static str,
    },
    #[error("临时文件扩展名必须以点开头且不能包含路径分隔符: {value}")]
    InvalidTempFileExtension { value: String },
    #[error("SFTP 命令不能为空")]
    EmptySftpCommand,
}

pub fn validate_config(config: &Config) -> Result<(), ConfigValidationError> {
    if config.routes.is_empty() {
        return Err(ConfigValidationError::EmptyRoutes);
    }

    let default_count = config.routes.iter().filter(|route| route.default).count();
    if default_count != 1 {
        return Err(ConfigValidationError::DefaultRouteCount {
            count: default_count,
        });
    }

    let mut ports = HashSet::with_capacity(config.routes.len());
    for (index, route) in config.routes.iter().enumerate() {
        if route.port == 0 {
            return Err(ConfigValidationError::InvalidPort { port: route.port });
        }
        if !ports.insert(route.port) {
            return Err(ConfigValidationError::DuplicatePort { port: route.port });
        }

        validate_required_field(index, "名称", &route.name)?;
        validate_required_field(index, "Shell 路径", &route.shell)?;
        validate_required_field(index, "交互式模板", &route.interactive_template)?;
        validate_required_field(index, "命令模板", &route.command_template)?;
        validate_required_field(index, "临时文件扩展名", &route.tmp_file_ext)?;

        let extension = route.tmp_file_ext.trim();
        if !extension.starts_with('.') || extension.len() == 1 || extension.contains(['/', '\\']) {
            return Err(ConfigValidationError::InvalidTempFileExtension {
                value: route.tmp_file_ext.clone(),
            });
        }
    }

    if config.sftp_command.trim().is_empty() {
        return Err(ConfigValidationError::EmptySftpCommand);
    }

    Ok(())
}

fn validate_required_field(
    route_index: usize,
    field: &'static str,
    value: &str,
) -> Result<(), ConfigValidationError> {
    if value.trim().is_empty() {
        return Err(ConfigValidationError::EmptyRouteField {
            route_index: route_index + 1,
            field,
        });
    }
    Ok(())
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
                    interactive_template: "wsl.exe -d Ubuntu -- bash -lc 'cd ~ && exec bash -l'"
                        .to_string(),
                    command_template: "wsl.exe -d Ubuntu -- bash -c 'cd ~ && . \"{tmpfile_wsl}\"'"
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

    #[test]
    fn validates_default_config() {
        assert_eq!(validate_config(&Config::default_config()), Ok(()));
    }

    #[test]
    fn rejects_empty_routes() {
        let mut config = Config::default_config();
        config.routes.clear();

        assert_eq!(
            validate_config(&config),
            Err(ConfigValidationError::EmptyRoutes)
        );
    }

    #[test]
    fn rejects_duplicate_ports() {
        let mut config = Config::default_config();
        config.routes[1].port = config.routes[0].port;

        assert_eq!(
            validate_config(&config),
            Err(ConfigValidationError::DuplicatePort { port: 22 })
        );
    }

    #[test]
    fn rejects_missing_default_route() {
        let mut config = Config::default_config();
        for route in &mut config.routes {
            route.default = false;
        }

        assert_eq!(
            validate_config(&config),
            Err(ConfigValidationError::DefaultRouteCount { count: 0 })
        );
    }

    #[test]
    fn rejects_multiple_default_routes() {
        let mut config = Config::default_config();
        config.routes[1].default = true;

        assert_eq!(
            validate_config(&config),
            Err(ConfigValidationError::DefaultRouteCount { count: 2 })
        );
    }

    #[test]
    fn rejects_zero_port() {
        let mut config = Config::default_config();
        config.routes[0].port = 0;

        assert_eq!(
            validate_config(&config),
            Err(ConfigValidationError::InvalidPort { port: 0 })
        );
    }

    #[test]
    fn rejects_blank_route_field() {
        let mut config = Config::default_config();
        config.routes[0].shell = "  ".to_string();

        assert_eq!(
            validate_config(&config),
            Err(ConfigValidationError::EmptyRouteField {
                route_index: 1,
                field: "Shell 路径",
            })
        );
    }

    #[test]
    fn rejects_invalid_temp_file_extension() {
        let mut config = Config::default_config();
        config.routes[0].tmp_file_ext = "scripts/test.ps1".to_string();

        assert_eq!(
            validate_config(&config),
            Err(ConfigValidationError::InvalidTempFileExtension {
                value: "scripts/test.ps1".to_string(),
            })
        );
    }

    #[test]
    fn rejects_blank_sftp_command() {
        let mut config = Config::default_config();
        config.sftp_command.clear();

        assert_eq!(
            validate_config(&config),
            Err(ConfigValidationError::EmptySftpCommand)
        );
    }
}
