use gpui::{div, prelude::*, AnyElement, FocusHandle, IntoElement};

use crate::theme::AppTheme;

pub fn modal_overlay(
    title: impl Into<gpui::SharedString>,
    body: impl Into<gpui::SharedString>,
    content: impl IntoElement,
    focus_handle: &FocusHandle,
    theme: AppTheme,
) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.overlay)
        .child(
            div()
                .track_focus(focus_handle)
                .w(gpui::px(440.0))
                .max_w_full()
                .flex()
                .flex_col()
                .gap_3()
                .p_5()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.surface)
                .text_color(theme.text)
                .shadow_lg()
                .child(div().text_lg().child(title.into()))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.text_muted)
                        .child(body.into()),
                )
                .child(content),
        )
        .into_any_element()
}
