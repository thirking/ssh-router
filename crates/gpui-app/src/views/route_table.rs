use gpui::{div, prelude::*, AnyElement, IntoElement};

use crate::{config_state::UiRoute, theme::AppTheme};

pub fn route_table(routes: &[UiRoute], theme: AppTheme) -> AnyElement {
    let header = div()
        .flex()
        .gap_3()
        .pb_2()
        .border_b_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.text_muted)
        .children([
            div().w(gpui::px(70.0)).child("端口"),
            div().w(gpui::px(140.0)).child("名称"),
            div().flex_1().child("Shell 路径"),
            div().w(gpui::px(80.0)).child("默认"),
        ]);

    let rows = routes.iter().map(|item| {
        div()
            .flex()
            .items_center()
            .gap_3()
            .py_2()
            .border_b_1()
            .border_color(theme.border.opacity(0.5))
            .text_color(theme.text)
            .child(div().w(gpui::px(70.0)).child(item.route.port.to_string()))
            .child(div().w(gpui::px(140.0)).child(item.route.name.clone()))
            .child(div().flex_1().text_sm().child(item.route.shell.clone()))
            .child(
                div()
                    .w(gpui::px(80.0))
                    .child(if item.route.default { "是" } else { "否" }),
            )
    });

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(header)
        .children(rows)
        .into_any_element()
}
