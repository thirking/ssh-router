use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ClickEvent, IntoElement, Window};
use gpui_component::{button::Button, Sizable};

use crate::{config_state::UiRoute, theme::AppTheme};

pub type RouteAction = Rc<dyn Fn(u64, &ClickEvent, &mut Window, &mut App)>;

pub fn route_table(
    routes: &[UiRoute],
    theme: AppTheme,
    on_edit: RouteAction,
    on_delete: RouteAction,
) -> AnyElement {
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
        let edit = on_edit.clone();
        let delete = on_delete.clone();
        let id = item.id;
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
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        Button::new(("edit-route", id))
                            .small()
                            .label("编辑")
                            .on_click(move |event, window, cx| (edit)(id, event, window, cx)),
                    )
                    .child(
                        Button::new(("delete-route", id))
                            .small()
                            .label("删除")
                            .on_click(move |event, window, cx| (delete)(id, event, window, cx)),
                    ),
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
