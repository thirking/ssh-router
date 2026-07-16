use gpui::{div, prelude::*, AnyElement, Entity, IntoElement};
use gpui_component::input::{Input, InputState};

use crate::theme::AppTheme;

pub fn text_field(
    label: impl Into<gpui::SharedString>,
    input: &Entity<InputState>,
    theme: AppTheme,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(theme.text_muted)
                .child(label.into()),
        )
        .child(Input::new(input).w_full())
        .into_any_element()
}
