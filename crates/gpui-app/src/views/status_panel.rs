use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ClickEvent, IntoElement, Window};
use gpui_component::{button::Button, Disableable, Sizable};

use crate::{
    system_status_state::{SystemLoadState, SystemStatusState},
    theme::AppTheme,
};

pub type StatusAction = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

pub fn status_panel(
    state: &SystemStatusState,
    theme: AppTheme,
    on_refresh: StatusAction,
) -> AnyElement {
    let status = state.status.as_ref();
    let detail = match &state.load_state {
        SystemLoadState::Idle => "等待检查".to_string(),
        SystemLoadState::Loading => "正在检查系统状态…".to_string(),
        SystemLoadState::Loaded => "状态检查完成".to_string(),
        SystemLoadState::Error(error) => format!("检查失败：{error}"),
    };
    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_lg().child("系统状态"))
                .child(
                    Button::new("refresh-system-status")
                        .small()
                        .label(if state.load_state == SystemLoadState::Loading {
                            "检查中…"
                        } else {
                            "刷新"
                        })
                        .disabled(state.load_state == SystemLoadState::Loading)
                        .on_click(move |event, window, cx| (on_refresh)(event, window, cx)),
                ),
        )
        .child(div().text_sm().text_color(theme.text_muted).child(detail))
        .child(status_item(
            "CLI",
            status
                .map(|value| {
                    if value.cli_up_to_date {
                        "已部署且为最新"
                    } else if value.cli_deployed {
                        "已部署但版本不一致"
                    } else {
                        "未部署"
                    }
                })
                .unwrap_or("未检查"),
            theme,
        ))
        .child(status_item(
            "DefaultShell",
            status
                .map(|value| {
                    if value.default_shell_set {
                        "已设置"
                    } else {
                        "未设置"
                    }
                })
                .unwrap_or("未检查"),
            theme,
        ))
        .child(status_item(
            "sshd",
            status
                .map(|value| {
                    if value.sshd_running {
                        "运行中"
                    } else {
                        value.sshd_detail.as_str()
                    }
                })
                .unwrap_or("未检查"),
            theme,
        ))
        .child(status_item(
            "配置",
            status
                .map(|value| {
                    if value.config_exists {
                        "已存在"
                    } else {
                        "不存在"
                    }
                })
                .unwrap_or("未检查"),
            theme,
        ))
        .child(status_item(
            "CLI 路径",
            status
                .map(|value| value.cli_path.as_str())
                .unwrap_or("未检查"),
            theme,
        ))
        .into_any_element()
}

fn status_item(label: &'static str, value: &str, theme: AppTheme) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .child(div().text_sm().child(label))
        .child(
            div()
                .text_sm()
                .text_color(theme.text_muted)
                .child(value.to_string()),
        )
}
