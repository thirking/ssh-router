mod repository;

use std::io::{self, BufRead, Write};

use serde::Deserialize;
use serde_json::{json, Value};
use ssh_router_app_core::{
    AppError, AppResult, ConfigError, ConfigRepository, PermissionError, SystemService,
    UpdateCandidate, UpdateService, UPDATE_MANIFEST_URL,
};
use ssh_router_config::{validate_config, Config};

const MAX_REQUEST_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: u64,
    method: String,
    #[serde(default)]
    params: Value,
}

struct Backend {
    repository: Box<dyn ConfigRepository>,
    system: Option<Box<dyn SystemService>>,
    config_path: String,
    updater: UpdateService,
    candidate: Option<UpdateCandidate>,
}

impl Backend {
    fn handle(&mut self, request: Request, output: &mut dyn Write) -> AppResult<Value> {
        match request.method.as_str() {
            "hello" => Ok(json!({
                "protocol": 1,
                "version": env!("CARGO_PKG_VERSION"),
                "windows": cfg!(windows),
                "configPath": self.config_path,
            })),
            "config.load" => Ok(json!(self.repository.load()?)),
            "config.defaults" => Ok(json!(Config::default_config())),
            "config.validate" | "config.save" => {
                let config: Config = serde_json::from_value(request.params).map_err(|error| {
                    ConfigError::InvalidFormat {
                        message: error.to_string(),
                    }
                })?;
                validate_config(&config).map_err(|error| ConfigError::Validation {
                    message: error.to_string(),
                })?;
                if request.method == "config.save" {
                    self.repository.save(&config)?;
                }
                Ok(json!(config))
            }
            "config.create" => match self.repository.load() {
                Err(AppError::Config(ConfigError::NotFound { .. })) => {
                    Ok(json!(self.repository.create_default()?))
                }
                Ok(_) => Err(ConfigError::Validation {
                    message: "配置已存在，请重新加载".into(),
                }
                .into()),
                Err(error) => Err(error),
            },
            "system.status" => {
                let status = self.system()?.check_status()?;
                Ok(json!({
                    "cliDeployed": status.cli_deployed,
                    "cliUpToDate": status.cli_up_to_date,
                    "cliPath": status.cli_path,
                    "defaultShellSet": status.default_shell_set,
                    "defaultShell": status.default_shell,
                    "configExists": status.config_exists,
                    "sshdRunning": status.sshd_running,
                    "sshdDetail": status.sshd_detail,
                }))
            }
            "system.installCli" => Ok(json!({"message": self.system()?.install_cli()?.message})),
            "system.setDefaultShell" => {
                Ok(json!({"message": self.system()?.set_default_shell()?.message}))
            }
            "system.restartSshd" => Ok(json!({"message": self.system()?.restart_sshd()?.message})),
            "update.check" => {
                self.system()?;
                self.candidate = self.updater.check_url(UPDATE_MANIFEST_URL)?;
                Ok(self.candidate.as_ref().map_or(
                    Value::Null,
                    |candidate| json!({"version": candidate.version, "notes": candidate.notes}),
                ))
            }
            "update.install" => {
                self.system()?;
                let candidate = self
                    .candidate
                    .as_ref()
                    .ok_or_else(|| ConfigError::Validation {
                        message: "请先检查更新".into(),
                    })?;
                let mut installer = self.updater.download_and_verify(candidate, |progress| {
                    let _ = write_json(
                        output,
                        &json!({
                            "event": "update.progress",
                            "downloadedBytes": progress.downloaded_bytes,
                            "totalBytes": progress.total_bytes,
                        }),
                    );
                })?;
                self.updater.launch_installer(&mut installer)?;
                Ok(json!({"message": "安装程序已启动"}))
            }
            _ => Err(ConfigError::Validation {
                message: format!("未知请求: {}", request.method),
            }
            .into()),
        }
    }

    fn system(&self) -> AppResult<&dyn SystemService> {
        self.system.as_deref().ok_or_else(|| {
            ssh_router_app_core::PlatformError::Unsupported {
                operation: "Windows 系统操作".into(),
            }
            .into()
        })
    }
}

fn error_value(error: &AppError) -> Value {
    let code = match error {
        AppError::Config(ConfigError::NotFound { .. }) => "not_found",
        AppError::Config(ConfigError::InvalidFormat { .. }) => "invalid_format",
        AppError::Config(ConfigError::Validation { .. }) => "validation",
        AppError::Permission(PermissionError::Cancelled) => "cancelled",
        AppError::Platform(_) => "platform",
        _ => "operation",
    };
    json!({"code": code, "message": error.to_string()})
}

fn write_json(output: &mut dyn Write, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    output.flush()
}

fn serve(mut backend: Backend, input: impl BufRead, output: &mut dyn Write) -> io::Result<()> {
    let mut input = input;
    loop {
        let mut line = String::new();
        let count = std::io::Read::take(&mut input, MAX_REQUEST_BYTES + 1).read_line(&mut line)?;
        if count == 0 {
            return Ok(());
        }
        if count as u64 > MAX_REQUEST_BYTES {
            write_json(
                output,
                &json!({"id": null, "error": {"code": "protocol", "message": "请求超过 1 MiB 限制"}}),
            )?;
            return Ok(());
        }
        match serde_json::from_str::<Request>(&line) {
            Ok(request) => {
                let id = request.id;
                let response = match backend.handle(request, output) {
                    Ok(result) => json!({"id": id, "result": result}),
                    Err(error) => json!({"id": id, "error": error_value(&error)}),
                };
                write_json(output, &response)?;
            }
            Err(error) => write_json(
                output,
                &json!({
                    "id": null, "error": {"code": "protocol", "message": error.to_string()},
                }),
            )?,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let config_path = match args.next().as_deref() {
        None => None,
        Some("--config") => Some(args.next().ok_or("--config 缺少路径")?),
        Some(_) => return Err("仅支持 --config <path>".into()),
    };
    if args.next().is_some() {
        return Err("参数过多".into());
    }
    let backend = repository::backend(config_path)?;
    serve(backend, io::stdin().lock(), &mut io::stdout().lock())?;
    Ok(())
}
