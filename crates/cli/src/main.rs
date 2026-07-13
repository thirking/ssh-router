mod log;
mod wsl;
mod temp;
mod routing;
mod win32;

use std::env;
use std::fs;
use std::path::PathBuf;

const CONFIG_PATH: &str = r"C:\ProgramData\ssh\ssh-router.json";

#[cfg(windows)]
fn main() {
    let pid = std::process::id();
    temp::clean_stale_temp_files(pid);

    let h_job = win32::create_kill_on_close_job(&log::log);

    // 解析 SSH_CONNECTION 获取端口。
    // SSH_CONNECTION 格式: "<client_ip> <client_port> <server_ip> <server_port>"
    let ssh_conn = env::var("SSH_CONNECTION").unwrap_or_default();
    let port = ssh_conn
        .split_whitespace()
        .nth(3)
        .unwrap_or("");

    // 解析命令行参数。
    // 注意: Rust 的 env::args() 含程序名 (args[0])，而 C# 的 Main(string[] args) 不含；
    // 故判断 "-c" 用 args[1]，命令内容用 args[2]，长度阈值较 C# 多 1。
    // （C# 原逻辑: args.Length >= 2 && args[0] == "-c"）
    let args: Vec<String> = env::args().collect();
    let has_command = args.len() >= 3 && args[1] == "-c";
    let command = if has_command { Some(args[2].as_str()) } else { None };

    // 记录调试日志
    log::log("========");
    log::log(&format!("args: {:?}", args));
    log::log(&format!("port: {}", port));
    if let Some(cmd) = command {
        log::log(&format!("command: {}", cmd));
    }

    // 读取配置
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            log::log(&format!("ERROR: failed to load config: {}", e));
            win32::close_job(h_job.unwrap_or(0));
            std::process::exit(1);
        }
    };

    // 路由决策
    let cmd_line;
    let temp_file: Option<PathBuf>;

    if let Some(cmd) = command {
        if routing::is_sftp_command(cmd) {
            // SFTP 特殊处理：直接走配置中的 sftp_command
            cmd_line = config.sftp_command.clone();
            temp_file = None;
        } else {
            // 有命令：匹配端口 → commandTemplate
            let route = match routing::match_route(&config, port) {
                Some(r) => r,
                None => {
                    log::log("ERROR: no matching route and no default route");
                    win32::close_job(h_job.unwrap_or(0));
                    std::process::exit(1);
                }
            };

            // 写命令到临时文件，避免 shell quoting 破坏复杂脚本
            // (多行脚本、here-document、嵌套引号、sh -c 等)
            let ext = &route.tmp_file_ext;
            let tmp_path = env::temp_dir().join(format!("ssh-cmd-{}{}", pid, ext));
            if let Err(e) = fs::write(&tmp_path, cmd) {
                log::log(&format!("ERROR: failed to create temp file: {}", e));
                win32::close_job(h_job.unwrap_or(0));
                std::process::exit(1);
            }

            let tmp_str = tmp_path.to_string_lossy().to_string();
            let tmp_wsl = wsl::to_wsl_path(&tmp_str);

            let needs_wsl = route.command_template.contains("{tmpfile_wsl}");
            let tmpfile_wsl = if needs_wsl { Some(tmp_wsl.as_str()) } else { None };

            cmd_line = routing::render_template(
                &route.command_template,
                &route.shell,
                Some(tmp_str.as_str()),
                tmpfile_wsl,
            );
            temp_file = Some(tmp_path);
        }
    } else {
        // 无命令（交互式）：匹配端口 → interactiveTemplate
        let route = match routing::match_route(&config, port) {
            Some(r) => r,
            None => {
                log::log("ERROR: no matching route and no default route");
                win32::close_job(h_job.unwrap_or(0));
                std::process::exit(1);
            }
        };

        cmd_line = routing::render_template(
            &route.interactive_template,
            &route.shell,
            None,
            None,
        );
        temp_file = None;
    }

    log::log(&format!("cmdLine: {}", cmd_line));

    // 启动子进程并等待其退出（CreateProcessW + Job Object，详见 win32 模块）
    let exit_code = win32::launch_and_wait(&cmd_line, h_job.unwrap_or(0), &log::log);

    // 清理临时文件
    if let Some(tf) = &temp_file {
        let _ = fs::remove_file(tf);
    }

    // 关闭 Job Object（触发 KILL_ON_JOB_CLOSE，回收所有子进程）
    win32::close_job(h_job.unwrap_or(0));

    log::log(&format!("exit code: {}", exit_code));
    log::log("========");

    std::process::exit(exit_code as i32);
}

#[cfg(not(windows))]
fn main() {
    // ssh-router-cli 作为 sshd 的 DefaultShell 仅在 Windows 上运行；
    // 非 Windows 平台直接报错退出（便于在 macOS 上运行 wsl/routing/temp 单元测试）。
    eprintln!("ssh-router-cli: this program only runs on Windows");
    std::process::exit(1);
}

/// 从 CONFIG_PATH 读取并解析 JSON 配置。
/// 配置缺失或格式错误时返回带上下文的 Err 字符串。
fn load_config() -> Result<ssh_router_config::Config, String> {
    let content = fs::read_to_string(CONFIG_PATH)
        .map_err(|e| format!("read config: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("parse config: {}", e))
}
