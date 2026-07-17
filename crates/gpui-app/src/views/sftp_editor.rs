use gpui::{div, prelude::*, AnyElement, IntoElement, SharedString};

use crate::theme::AppTheme;

pub fn sftp_editor(command: impl Into<SharedString>, theme: AppTheme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_sm()
                .text_color(theme.text_muted)
                .child("SFTP 命令"),
        )
        .child(
            div()
                .p_3()
                .border_1()
                .border_color(theme.border)
                .bg(theme.surface)
                .text_color(theme.text)
                .child(command.into()),
        )
        .into_any_element()
}
