use ssh_router_config::Config;
use std::fs;
use std::path::Path;

/// 配置文件路径（与 Windows 上 OpenSSH 固定安装位置对齐）
const CONFIG_PATH: &str = r"C:\ProgramData\ssh\ssh-router.json";

/// 读取并反序列化配置文件
#[tauri::command]
pub fn load_config() -> Result<Config, String> {
    let content = fs::read_to_string(CONFIG_PATH).map_err(|e| format!("read config: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("parse config: {}", e))
}

/// 校验并持久化配置：必须恰好有一条默认路由
#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    // 校验恰好一条 default
    let defaults: Vec<_> = config.routes.iter().filter(|r| r.default).collect();
    if defaults.len() != 1 {
        return Err(format!(
            "必须恰好有一条默认路由，当前有 {} 条",
            defaults.len()
        ));
    }
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("serialize config: {}", e))?;
    fs::write(CONFIG_PATH, json).map_err(|e| format!("write config: {}", e))
}

/// 创建默认配置并写入磁盘（若目录不存在则创建）
#[tauri::command]
pub fn create_default_config() -> Result<Config, String> {
    let config = Config::default_config();
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("serialize config: {}", e))?;
    // 确保目录存在
    if let Some(parent) = Path::new(CONFIG_PATH).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
    }
    fs::write(CONFIG_PATH, json).map_err(|e| format!("write config: {}", e))?;
    Ok(config)
}
