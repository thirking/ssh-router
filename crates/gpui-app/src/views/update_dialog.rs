#![allow(dead_code)]

use gpui::{div, prelude::*, AnyElement, IntoElement};
use gpui_component::button::{Button, ButtonVariants};

use super::update_panel::UpdateAction;
use crate::{
    theme::AppTheme,
    update_state::{UpdatePhase, UpdateState},
};

pub fn update_dialog(
    state: &UpdateState,
    theme: AppTheme,
    on_install: UpdateAction,
    on_dismiss: UpdateAction,
) -> AnyElement {
    if !state.dialog_open {
        return div().into_any_element();
    }
    let candidate = state.candidate.as_ref();
    div()
        .absolute()
        .top_4()
        .right_4()
        .w(gpui::px(400.0))
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.primary)
        .bg(theme.surface)
        .shadow_md()
        .child(
            div().text_lg().child(
                candidate
                    .map(|value| format!("发现新版本 {}", value.version))
                    .unwrap_or_else(|| "更新".to_string()),
            ),
        )
        .child(
            div().text_sm().text_color(theme.text_muted).child(
                candidate
                    .and_then(|value| value.notes.clone())
                    .unwrap_or_default(),
            ),
        )
        .child(
            div().text_sm().child(match state.phase {
                UpdatePhase::Downloading => format!("已下载 {} 字节", state.downloaded_bytes),
                UpdatePhase::Installing => "正在启动安装程序…".to_string(),
                UpdatePhase::Error => state
                    .error
                    .clone()
                    .unwrap_or_else(|| "更新失败".to_string()),
                _ => String::new(),
            }),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .mt_3()
                .child(
                    Button::new("install-update")
                        .primary()
                        .label(if state.phase == UpdatePhase::Downloading {
                            "下载中…"
                        } else {
                            "安装更新"
                        })
                        .on_click(move |event, window, cx| (on_install)(event, window, cx)),
                )
                .child(
                    Button::new("dismiss-update")
                        .label("稍后处理")
                        .on_click(move |event, window, cx| (on_dismiss)(event, window, cx)),
                ),
        )
        .into_any_element()
}
