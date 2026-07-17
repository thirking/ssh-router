use gpui::{div, prelude::*, AnyElement, IntoElement};

use crate::{route_editor_state::RouteEditorState, theme::AppTheme};

pub fn route_editor(editor: &RouteEditorState, theme: AppTheme) -> AnyElement {
    let title = if editor.editing_id.is_some() {
        "编辑路由"
    } else {
        "添加路由"
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .child(div().text_lg().child(title))
        .child(div().text_sm().text_color(theme.text_muted).child(format!(
            "端口：{} · {}",
            editor.route.port, editor.route.name
        )))
        .children(editor.errors.iter().map(|error| {
            div()
                .text_sm()
                .text_color(theme.danger)
                .child(error.clone())
        }))
        .into_any_element()
}
