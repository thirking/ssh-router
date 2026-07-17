use gpui::{div, prelude::*, AnyElement, IntoElement};
use gpui_component::{
    button::{Button, ButtonVariants},
    Disableable,
};

use super::update_panel::UpdateAction;
use crate::{
    theme::AppTheme,
    update_state::{UpdatePhase, UpdateState},
};

pub fn update_dialog(
    state: &UpdateState,
    theme: AppTheme,
    on_install: UpdateAction,
    on_confirm_install: UpdateAction,
    on_dismiss: UpdateAction,
) -> AnyElement {
    if !state.dialog_open {
        return div().into_any_element();
    }
    let candidate = state.candidate.as_ref();
    let primary_action = if state.confirm_dirty_install {
        on_confirm_install
    } else {
        on_install
    };
    let progress = match (state.downloaded_bytes, state.total_bytes) {
        (downloaded, Some(total)) if total > 0 => {
            format!("已下载 {:.0}%", downloaded as f64 / total as f64 * 100.0)
        }
        (downloaded, _) if downloaded > 0 => format!("已下载 {downloaded} 字节"),
        _ => String::new(),
    };
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
                UpdatePhase::Downloading => progress,
                UpdatePhase::Installing => "正在启动安装程序…".to_string(),
                UpdatePhase::Error => state
                    .error
                    .clone()
                    .unwrap_or_else(|| "更新失败".to_string()),
                _ => String::new(),
            }),
        )
        .when(state.confirm_dirty_install, |dialog| {
            dialog.child(
                div()
                    .mt_3()
                    .p_3()
                    .rounded_md()
                    .bg(theme.warning.opacity(0.12))
                    .text_sm()
                    .child("配置尚未保存。继续安装将退出应用，请确认已处理草稿。"),
            )
        })
        .child(
            div()
                .flex()
                .gap_2()
                .mt_3()
                .child(
                    Button::new("install-update")
                        .primary()
                        .label(if state.confirm_dirty_install {
                            "仍然安装"
                        } else if state.phase == UpdatePhase::Downloading {
                            "下载中…"
                        } else if state.phase == UpdatePhase::Installing {
                            "安装中…"
                        } else {
                            "安装更新"
                        })
                        .disabled(matches!(
                            state.phase,
                            UpdatePhase::Downloading | UpdatePhase::Installing
                        ))
                        .on_click(move |event, window, cx| (primary_action)(event, window, cx)),
                )
                .child(
                    Button::new("dismiss-update")
                        .label("稍后处理")
                        .on_click(move |event, window, cx| (on_dismiss)(event, window, cx)),
                ),
        )
        .into_any_element()
}
