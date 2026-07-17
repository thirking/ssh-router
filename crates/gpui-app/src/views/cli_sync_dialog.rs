use gpui::{div, prelude::*, AnyElement, IntoElement};
use gpui_component::button::{Button, ButtonVariants};

use super::status_panel::StatusAction;
use crate::theme::AppTheme;

pub fn cli_sync_dialog(
    open: bool,
    theme: AppTheme,
    on_sync: StatusAction,
    on_dismiss: StatusAction,
) -> AnyElement {
    if !open {
        return div().into_any_element();
    }
    div()
        .absolute()
        .top_4()
        .left_4()
        .right_4()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.warning)
        .bg(theme.surface)
        .shadow_md()
        .child(div().text_lg().child("CLI 版本需要同步"))
        .child(
            div()
                .text_sm()
                .text_color(theme.text_muted)
                .child("检测到已部署 CLI 与当前版本不一致。"),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .mt_3()
                .child(
                    Button::new("sync-cli-prompt")
                        .primary()
                        .label("立即同步")
                        .on_click(move |event, window, cx| (on_sync)(event, window, cx)),
                )
                .child(
                    Button::new("dismiss-cli-prompt")
                        .label("稍后处理")
                        .on_click(move |event, window, cx| (on_dismiss)(event, window, cx)),
                ),
        )
        .into_any_element()
}
