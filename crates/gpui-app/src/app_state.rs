use gpui::{AppContext, Context, Entity, FocusHandle, SharedString, Subscription, Window};
use gpui_component::input::{InputEvent, InputState};

use crate::{
    config_state::ConfigState,
    route_editor_state::{RouteEditorInputs, RouteEditorState},
    system_status_state::{SystemOperation, SystemStatusState},
    update_state::{UpdateCheckMode, UpdateInstallDecision, UpdateState},
};

const UPDATE_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationKind {
    Success,
    Warning,
    Error,
    Info,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: u64,
    pub kind: NotificationKind,
    pub message: SharedString,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalState {
    pub title: SharedString,
    pub body: SharedString,
}

pub struct AppState {
    pub config: ConfigState,
    pub route_editor: RouteEditorState,
    pub route_inputs: RouteEditorInputs,
    pub system: SystemStatusState,
    pub update: UpdateState,
    pub input: Entity<InputState>,
    pub modal: Option<ModalState>,
    pub notifications: Vec<Notification>,
    pub focus_handle: FocusHandle,
    pub primary_focus: FocusHandle,
    pub secondary_focus: FocusHandle,
    pub danger_focus: FocusHandle,
    pub save_focus: FocusHandle,
    pub create_focus: FocusHandle,
    pub modal_focus: FocusHandle,
    _route_subscriptions: Vec<Subscription>,
    syncing_inputs: bool,
    next_notification_id: u64,
    #[cfg(windows)]
    _tray_icon: Option<ssh_router_app_core::platform::windows::tray::TrayIcon>,
    #[cfg(windows)]
    _single_instance_guard:
        ssh_router_app_core::platform::windows::single_instance::SingleInstanceGuard,
}

impl AppState {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        #[cfg(windows)] single_instance_guard: ssh_router_app_core::platform::windows::single_instance::SingleInstanceGuard,
    ) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入路由名称以验证中文 IME")
                .default_value("PowerShell")
        });
        let route_inputs = RouteEditorInputs::new(window, cx);
        #[cfg(windows)]
        let tray_icon = crate::window_controller::native_hwnd(window)
            .ok()
            .and_then(|hwnd| {
                ssh_router_app_core::platform::windows::tray::TrayIcon::install(hwnd, "SSH Router")
                    .ok()
            });
        let subscriptions = vec![
            cx.subscribe(&route_inputs.port, |this, input, event: &InputEvent, cx| {
                if !this.syncing_inputs && matches!(event, InputEvent::Change) {
                    this.route_editor.route.port = input.read(cx).value().to_string();
                    this.refresh_route_validation();
                    cx.notify();
                }
            }),
            cx.subscribe(&route_inputs.name, |this, input, event: &InputEvent, cx| {
                if !this.syncing_inputs && matches!(event, InputEvent::Change) {
                    this.route_editor.route.name = input.read(cx).value().to_string();
                    this.refresh_route_validation();
                    cx.notify();
                }
            }),
            cx.subscribe(
                &route_inputs.shell,
                |this, input, event: &InputEvent, cx| {
                    if !this.syncing_inputs && matches!(event, InputEvent::Change) {
                        this.route_editor.route.shell = input.read(cx).value().to_string();
                        this.refresh_route_validation();
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(
                &route_inputs.interactive_template,
                |this, input, event: &InputEvent, cx| {
                    if !this.syncing_inputs && matches!(event, InputEvent::Change) {
                        this.route_editor.route.interactive_template =
                            input.read(cx).value().to_string();
                        this.refresh_route_validation();
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(
                &route_inputs.command_template,
                |this, input, event: &InputEvent, cx| {
                    if !this.syncing_inputs && matches!(event, InputEvent::Change) {
                        this.route_editor.route.command_template =
                            input.read(cx).value().to_string();
                        this.refresh_route_validation();
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(
                &route_inputs.tmp_file_ext,
                |this, input, event: &InputEvent, cx| {
                    if !this.syncing_inputs && matches!(event, InputEvent::Change) {
                        this.route_editor.route.tmp_file_ext = input.read(cx).value().to_string();
                        this.refresh_route_validation();
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(
                &route_inputs.sftp_command,
                |this, input, event: &InputEvent, cx| {
                    if !this.syncing_inputs && matches!(event, InputEvent::Change) {
                        this.config
                            .set_sftp_command(input.read(cx).value().to_string());
                        cx.notify();
                    }
                },
            ),
        ];
        let update = UpdateState {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            ..UpdateState::default()
        };
        Self {
            config: ConfigState::default(),
            route_editor: RouteEditorState::default(),
            route_inputs,
            system: SystemStatusState::default(),
            update,
            input,
            modal: None,
            notifications: vec![Notification {
                id: 1,
                kind: NotificationKind::Info,
                message: "GPUI 基础状态已初始化".into(),
            }],
            focus_handle: cx.focus_handle(),
            primary_focus: cx.focus_handle().tab_index(10).tab_stop(true),
            secondary_focus: cx.focus_handle().tab_index(11).tab_stop(true),
            danger_focus: cx.focus_handle().tab_index(12).tab_stop(true),
            save_focus: cx.focus_handle().tab_index(13).tab_stop(true),
            create_focus: cx.focus_handle().tab_index(14).tab_stop(true),
            modal_focus: cx.focus_handle().tab_index(20).tab_stop(true),
            _route_subscriptions: subscriptions,
            syncing_inputs: false,
            next_notification_id: 2,
            #[cfg(windows)]
            _tray_icon: tray_icon,
            #[cfg(windows)]
            _single_instance_guard: single_instance_guard,
        }
    }

    pub fn tray_available(&self) -> bool {
        #[cfg(windows)]
        return self._tray_icon.is_some();
        #[cfg(not(windows))]
        false
    }

    fn notify_error(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.notify(NotificationKind::Error, message, cx);
    }

    pub(crate) fn refresh_route_validation(&mut self) {
        let Ok(route) = self.route_editor.validate() else {
            return;
        };
        if let Err(error) = self
            .config
            .validate_route_candidate(self.route_editor.editing_id, route)
        {
            self.route_editor.errors.push(error);
        }
    }

    #[cfg(windows)]
    pub fn start_status_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use ssh_router_app_core::platform::windows::WindowsPlatform;
        use ssh_router_app_core::{DesktopSystemService, SystemService};

        if !self.system.begin_refresh() {
            return;
        }
        cx.notify();
        let task = cx.background_spawn(async move {
            let platform = WindowsPlatform::new().map_err(|error| error.to_string())?;
            let service = DesktopSystemService::new(platform);
            service.check_status().map_err(|error| error.to_string())
        });
        cx.spawn_in(window, async move |entity, cx| {
            let result = task.await;
            let _ = entity.update_in(cx, |state, _, cx| {
                state.system.apply_refresh(result);
                if state.system.maybe_prompt_cli_sync() {
                    state.notify(
                        NotificationKind::Warning,
                        "CLI 版本与当前应用不一致，请同步",
                        cx,
                    );
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn request_status_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(windows)]
        self.start_status_refresh(window, cx);
        #[cfg(not(windows))]
        self.notify(NotificationKind::Info, "系统状态检查仅支持 Windows", cx);
    }

    #[cfg(windows)]
    pub fn start_system_operation(
        &mut self,
        operation: SystemOperation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use ssh_router_app_core::platform::windows::WindowsPlatform;
        use ssh_router_app_core::{DesktopSystemService, SystemService};

        if operation == SystemOperation::Refresh {
            self.start_status_refresh(window, cx);
            return;
        }
        if !self.system.begin_operation(operation) {
            return;
        }
        if operation == SystemOperation::InstallCli {
            self.system.close_cli_prompt();
        }
        cx.notify();
        let task = cx.background_spawn(async move {
            let platform = WindowsPlatform::new()?;
            let service = DesktopSystemService::new(platform);
            match operation {
                SystemOperation::InstallCli => service.install_cli(),
                SystemOperation::SetDefaultShell => service.set_default_shell(),
                SystemOperation::RestartSshd => service.restart_sshd(),
                SystemOperation::Refresh => Err(ssh_router_app_core::PlatformError::Operation {
                    message: "刷新应使用独立状态任务".to_string(),
                }
                .into()),
            }
        });
        cx.spawn_in(window, async move |entity, cx| {
            let result = task.await;
            let _ = entity.update_in(cx, |state, window, cx| {
                let cancelled = matches!(
                    &result,
                    Err(ssh_router_app_core::AppError::Permission(
                        ssh_router_app_core::PermissionError::Cancelled
                    ))
                );
                let success = state
                    .system
                    .apply_operation(result.map_err(|error| error.to_string()));
                if success {
                    state.notify(NotificationKind::Success, "系统操作已完成", cx);
                    state.start_status_refresh(window, cx);
                } else {
                    let message = state.system.error.clone().unwrap_or_default();
                    if cancelled {
                        state.notify(NotificationKind::Info, "已取消管理员权限操作", cx);
                    } else {
                        state.notify_error(format!("系统操作失败：{message}"), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn request_system_operation(
        &mut self,
        operation: SystemOperation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(windows)]
        self.start_system_operation(operation, window, cx);
        #[cfg(not(windows))]
        self.notify(NotificationKind::Info, "系统快捷操作仅支持 Windows", cx);
    }

    pub fn dismiss_cli_sync_prompt(&mut self, cx: &mut Context<Self>) {
        self.system.close_cli_prompt();
        cx.notify();
    }

    pub fn start_update_schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.request_update_check(UpdateCheckMode::Automatic, window, cx);
        cx.spawn_in(window, async move |entity, cx| loop {
            cx.background_executor().timer(UPDATE_CHECK_INTERVAL).await;
            if entity
                .update_in(cx, |state, window, cx| {
                    state.request_update_check(UpdateCheckMode::Automatic, window, cx);
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    pub fn request_update_check(
        &mut self,
        mode: UpdateCheckMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.update.candidate.is_some() {
            if mode == UpdateCheckMode::Manual {
                self.update.show_available();
                cx.notify();
            }
            return;
        }
        if !self.update.begin_check() {
            return;
        }
        cx.notify();
        let task = cx.background_spawn(async move {
            ssh_router_app_core::UpdateService::new(
                env!("CARGO_PKG_VERSION"),
                ssh_router_app_core::WINDOWS_X86_64_PLATFORM,
            )
            .check_url(ssh_router_app_core::UPDATE_MANIFEST_URL)
            .map_err(|error| error.to_string())
        });
        cx.spawn_in(window, async move |entity, cx| {
            let result = task.await;
            let outcome = match &result {
                Ok(Some(_)) => Ok(true),
                Ok(None) => Ok(false),
                Err(message) => Err(message.clone()),
            };
            let _ = entity.update_in(cx, |state, _, cx| {
                state.update.apply_check(mode, result);
                match (mode, outcome) {
                    (_, Ok(true)) => {
                        state.notify(NotificationKind::Info, "发现可用更新", cx);
                    }
                    (UpdateCheckMode::Manual, Ok(false)) => {
                        state.notify(NotificationKind::Info, "当前已是最新版本", cx);
                    }
                    (UpdateCheckMode::Manual, Err(message)) => {
                        state.notify_error(format!("检查更新失败：{message}"), cx);
                    }
                    (UpdateCheckMode::Automatic, Ok(false)) => {}
                    (UpdateCheckMode::Automatic, Err(_)) => {}
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn request_update_install(
        &mut self,
        confirmed_dirty_config: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let decision = if confirmed_dirty_config {
            if self.update.confirm_download() {
                UpdateInstallDecision::Started
            } else {
                UpdateInstallDecision::Busy
            }
        } else {
            self.update.request_download(self.config.is_dirty)
        };
        match decision {
            UpdateInstallDecision::Started => self.start_update_download(window, cx),
            UpdateInstallDecision::NeedsConfirmation => {
                self.notify(
                    NotificationKind::Warning,
                    "配置尚未保存，请确认后再安装更新",
                    cx,
                );
            }
            UpdateInstallDecision::Busy => {}
            UpdateInstallDecision::Unavailable => {
                self.notify(NotificationKind::Info, "当前没有可安装的更新", cx);
            }
        }
        cx.notify();
    }

    fn start_update_download(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(candidate) = self.update.candidate.clone() else {
            self.update.fail("更新候选已失效");
            cx.notify();
            return;
        };
        let service = ssh_router_app_core::UpdateService::new(
            env!("CARGO_PKG_VERSION"),
            ssh_router_app_core::WINDOWS_X86_64_PLATFORM,
        );
        let download_service = service.clone();
        let (sender, receiver) = async_channel::bounded(8);
        let task = cx.background_spawn(async move {
            download_service.download_and_verify(&candidate, |progress| {
                let _ = sender.send_blocking(progress);
            })
        });
        cx.spawn_in(window, async move |entity, cx| {
            while let Ok(progress) = receiver.recv().await {
                let _ = entity.update_in(cx, |state, _, cx| {
                    state
                        .update
                        .progress(progress.downloaded_bytes, progress.total_bytes);
                    cx.notify();
                });
            }
            let mut installer = match task.await {
                Ok(installer) => installer,
                Err(error) => {
                    let _ = entity.update_in(cx, |state, _, cx| {
                        state.update.fail(error.to_string());
                        state.notify_error(format!("更新下载失败：{error}"), cx);
                    });
                    return;
                }
            };
            if entity
                .update_in(cx, |state, _, cx| {
                    state.update.finished();
                    cx.notify();
                })
                .is_err()
            {
                return;
            }
            let launch_task =
                cx.background_spawn(async move { service.launch_installer(&mut installer) });
            match launch_task.await {
                Ok(()) => {
                    let _ = cx.update(|_, cx| cx.quit());
                }
                Err(error) => {
                    let _ = entity.update_in(cx, |state, _, cx| {
                        state.update.fail(error.to_string());
                        state.notify_error(format!("启动安装程序失败：{error}"), cx);
                    });
                }
            }
        })
        .detach();
    }

    pub fn dismiss_update(&mut self, cx: &mut Context<Self>) {
        self.update.dismiss();
        cx.notify();
    }

    #[cfg(windows)]
    pub fn save_config(&mut self, cx: &mut Context<Self>) {
        use ssh_router_app_core::platform::windows::WindowsPlatform;
        use ssh_router_app_core::{ConfigRepository, ConfigService};

        if let Err(error) = self.config.validate() {
            self.notify_error(format!("保存失败：{error:?}"), cx);
            return;
        }
        let config = self.config.to_config();
        let saved_config = config.clone();
        let task = cx.background_spawn(async move {
            let platform = WindowsPlatform::new().map_err(|error| error.to_string())?;
            let repository = ConfigService::new(platform);
            repository.save(&config).map_err(|error| error.to_string())
        });
        cx.spawn(async move |entity, cx| {
            let result = task.await;
            let _ = entity.update(cx, |state, cx| {
                match state.config.apply_save_result(result, saved_config) {
                    Ok(()) => state.notify(NotificationKind::Success, "配置已保存", cx),
                    Err(error) => state.notify_error(format!("保存失败：{error:?}"), cx),
                }
            });
        })
        .detach();
    }

    #[cfg(windows)]
    pub fn create_default_config(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use ssh_router_app_core::platform::windows::WindowsPlatform;
        use ssh_router_app_core::{ConfigRepository, ConfigService};

        let task = cx.background_spawn(async move {
            let platform = WindowsPlatform::new().map_err(|error| error.to_string())?;
            let repository = ConfigService::new(platform);
            repository
                .create_default()
                .map_err(|error| error.to_string())
        });
        cx.spawn_in(window, async move |entity, cx| {
            let result = task.await;
            let _ = entity.update_in(cx, |state, window, cx| match result {
                Ok(config) => {
                    state.config.apply_loaded(config);
                    state.sync_sftp_input(window, cx);
                    state.notify(NotificationKind::Success, "已创建默认配置", cx);
                }
                Err(error) => state.notify_error(format!("创建默认配置失败：{error}"), cx),
            });
        })
        .detach();
    }

    #[cfg(windows)]
    pub fn start_config_load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use ssh_router_app_core::platform::windows::WindowsPlatform;
        use ssh_router_app_core::{ConfigRepository, ConfigService};

        self.config.load_state = crate::config_state::ConfigLoadState::Loading;
        let repository = match WindowsPlatform::new() {
            Ok(platform) => ConfigService::new(platform),
            Err(error) => {
                self.config.apply_load_result(Err(error));
                cx.notify();
                return;
            }
        };
        let task = cx.background_spawn(async move { repository.load() });
        cx.spawn_in(window, async move |entity, cx| {
            let result = task.await;
            let _ = entity.update_in(cx, |state, window, cx| {
                state.config.apply_load_result(result);
                state.sync_sftp_input(window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub fn notify(
        &mut self,
        kind: NotificationKind,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.notifications.push(Notification {
            id: self.next_notification_id,
            kind,
            message: message.into(),
        });
        self.next_notification_id += 1;
        if self.notifications.len() > 3 {
            self.notifications.remove(0);
        }
        cx.notify();
    }

    pub fn open_modal(&mut self, cx: &mut Context<Self>) {
        self.modal = Some(ModalState {
            title: "基础组件验证".into(),
            body: "模态层、键盘动作与焦点样式已经接入。".into(),
        });
        self.notify(NotificationKind::Warning, "模态框已打开，可按 Esc 关闭", cx);
    }

    pub fn close_modal(&mut self, cx: &mut Context<Self>) {
        self.modal = None;
        cx.notify();
    }

    pub fn edit_route(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self.config.routes.iter().find(|item| item.id == id) {
            self.route_editor = RouteEditorState::new(Some((id, item.route.clone())));
            self.sync_route_inputs(window, cx);
            cx.notify();
        }
    }

    pub fn start_new_route(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.route_editor = RouteEditorState::default();
        self.sync_route_inputs(window, cx);
        cx.notify();
    }

    pub fn cancel_route_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.route_editor.cancel();
        self.route_editor = RouteEditorState::default();
        self.sync_route_inputs(window, cx);
        cx.notify();
    }

    pub fn save_route_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.route_editor.validate() {
            Ok(route) => {
                if let Err(error) = self
                    .config
                    .validate_route_candidate(self.route_editor.editing_id, route.clone())
                {
                    self.route_editor.errors.push(error.clone());
                    self.notify_error(error, cx);
                    return;
                }
                if let Some(id) = self.route_editor.editing_id {
                    self.config.update_route(id, route);
                } else {
                    self.config.add_route(route);
                }
                self.route_editor = RouteEditorState::default();
                self.sync_route_inputs(window, cx);
                self.notify(NotificationKind::Success, "路由草稿已更新", cx);
            }
            Err(errors) => self.notify_error(errors.join("；"), cx),
        }
    }

    pub fn delete_route(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.config.delete_route(id) {
            self.notify(NotificationKind::Info, "路由已从草稿中删除", cx);
        }
    }

    fn sync_route_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.syncing_inputs = true;
        self.route_inputs
            .set_route(&self.route_editor.route, window, cx);
        self.syncing_inputs = false;
    }

    fn sync_sftp_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.syncing_inputs = true;
        self.route_inputs
            .set_sftp_command(self.config.sftp_command.clone(), window, cx);
        self.syncing_inputs = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_actions_are_distinct() {
        assert_ne!(
            std::any::TypeId::of::<crate::actions::FocusNext>(),
            std::any::TypeId::of::<crate::actions::FocusPrevious>()
        );
    }

    #[test]
    fn modal_state_keeps_user_facing_copy() {
        let modal = ModalState {
            title: "标题".into(),
            body: "正文".into(),
        };

        assert_eq!(modal.title, SharedString::from("标题"));
        assert_eq!(modal.body, SharedString::from("正文"));
    }
}
