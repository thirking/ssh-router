# Tauri / React 到 GPUI 功能映射

| 旧实现 | GPUI / app-core 替代 |
| --- | --- |
| `load_config` | `ConfigService::load` + `AppState::start_config_load` |
| `save_config` | `ConfigService::save` + `AppState::save_config` |
| `create_default_config` | `ConfigService::create_default` + `AppState::create_default_config` |
| `check_status` | `DesktopSystemService::check_status` + `SystemStatusState` |
| `install_cli` | `DesktopSystemService::install_cli` + 提权适配层 |
| `set_default_shell` | `DesktopSystemService::set_default_shell` + 提权适配层 |
| `restart_sshd` | `DesktopSystemService::restart_sshd` + 提权适配层 |
| `App.tsx` | `gpui-app::AppState` 与 `app.rs` |
| `RouteTable` | `views/route_table.rs` |
| `RouteDialog` | `route_editor_state.rs` + `views/route_editor.rs` |
| `SftpField` | `views/sftp_editor.rs` |
| `StatusPanel` | `views/status_panel.rs` |
| `QuickActions` | `views/quick_actions.rs` |
| `CliSyncDialog` | `views/cli_sync_dialog.rs` |
| `UpdatePanel` / `UpdateDialog` | `views/update_panel.rs` / `views/update_dialog.rs` |
| Tauri updater/process | `app-core::UpdateService` + NSIS `/P /R /UPDATE` |
| Tauri tray / window lifecycle | Win32 `tray.rs` + `window_controller.rs` |
| Tauri single instance | Win32 `single_instance.rs` |
| React 通知与基础组件 | `components/`、`Notification`、`ModalState` |

配置格式、安装目录、卸载键、CLI 部署路径和更新公钥保持兼容。
