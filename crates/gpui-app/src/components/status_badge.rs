use gpui::{div, prelude::*, AnyElement, IntoElement};

use crate::theme::AppTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Success,
    Warning,
    Error,
    Neutral,
}

pub fn status_badge(
    label: impl Into<gpui::SharedString>,
    kind: StatusKind,
    theme: AppTheme,
) -> AnyElement {
    let color = match kind {
        StatusKind::Success => theme.success,
        StatusKind::Warning => theme.warning,
        StatusKind::Error => theme.danger,
        StatusKind::Neutral => theme.text_muted,
    };

    div()
        .px_2()
        .py_1()
        .rounded_full()
        .bg(color.opacity(0.16))
        .text_color(color)
        .text_sm()
        .child(label.into())
        .into_any_element()
}
