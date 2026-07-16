use gpui::{div, prelude::*, AnyElement, IntoElement};

use crate::app_state::{Notification, NotificationKind};
use crate::theme::AppTheme;

pub fn toast_host(notifications: &[Notification], theme: AppTheme) -> AnyElement {
    div()
        .absolute()
        .top_4()
        .right_4()
        .w(gpui::px(360.0))
        .flex()
        .flex_col()
        .gap_2()
        .children(notifications.iter().map(|notification| {
            let accent = match notification.kind {
                NotificationKind::Success => theme.success,
                NotificationKind::Warning => theme.warning,
                NotificationKind::Error => theme.danger,
                NotificationKind::Info => theme.primary,
            };
            div()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(accent)
                .bg(theme.surface)
                .text_color(theme.text)
                .shadow_md()
                .child(notification.message.clone())
        }))
        .into_any_element()
}
