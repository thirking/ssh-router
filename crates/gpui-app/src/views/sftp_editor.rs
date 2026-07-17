use gpui::{div, prelude::*, AnyElement, Entity, IntoElement};
use gpui_component::input::{Input, InputState};

use crate::theme::AppTheme;

pub fn sftp_editor(input: &Entity<InputState>, theme: AppTheme) -> AnyElement {
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
        .child(Input::new(input).w_full())
        .into_any_element()
}
