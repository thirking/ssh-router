use gpui::{AppContext, Context, Entity, FocusHandle, SharedString, Window};
use gpui_component::input::InputState;

use crate::{config_state::ConfigState, route_editor_state::RouteEditorState};

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
    pub input: Entity<InputState>,
    pub modal: Option<ModalState>,
    pub notifications: Vec<Notification>,
    pub focus_handle: FocusHandle,
    pub primary_focus: FocusHandle,
    pub secondary_focus: FocusHandle,
    pub danger_focus: FocusHandle,
    pub modal_focus: FocusHandle,
    next_notification_id: u64,
}

impl AppState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入路由名称以验证中文 IME")
                .default_value("PowerShell")
        });
        Self {
            config: ConfigState::from_loaded(ssh_router_config::Config::default_config()),
            route_editor: RouteEditorState::default(),
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
            modal_focus: cx.focus_handle().tab_index(20).tab_stop(true),
            next_notification_id: 2,
        }
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
