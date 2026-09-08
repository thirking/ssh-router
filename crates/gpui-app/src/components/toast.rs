use std::time::Duration;

use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, ElementId, IntoElement};

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
                .relative()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(accent)
                .bg(theme.surface)
                .text_color(theme.text)
                .shadow_md()
                .child(notification.message.clone())
                .with_animation(
                    ElementId::NamedInteger("toast-enter".into(), notification.id),
                    Animation::new(Duration::from_millis(180)).with_easing(gpui::ease_out_quint()),
                    |toast, delta| toast.right(gpui::px(16.0 * (1.0 - delta))).opacity(delta),
                )
        }))
        .into_any_element()
}
