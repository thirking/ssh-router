use gpui::{div, prelude::*, AnyElement, IntoElement};
use gpui_component::button::ButtonVariants;

use crate::{
    config_state::ConfigState,
    route_editor_state::{RouteEditorInputs, RouteEditorState},
    theme::AppTheme,
};

use super::{
    route_editor::{route_editor, DefaultAction, EditorAction},
    route_table::{route_table, RouteAction},
    sftp_editor::sftp_editor,
};

#[allow(clippy::too_many_arguments)]
pub fn config_main_window(
    config: &ConfigState,
    editor: &RouteEditorState,
    inputs: &RouteEditorInputs,
    theme: AppTheme,
    on_edit: RouteAction,
    on_delete: RouteAction,
    on_add: EditorAction,
    on_save: EditorAction,
    on_cancel: EditorAction,
    on_default: DefaultAction,
) -> AnyElement {
    let title = match &config.load_state {
        crate::config_state::ConfigLoadState::Loading => "正在加载配置".to_string(),
        crate::config_state::ConfigLoadState::NotFound => "未找到配置文件".to_string(),
        crate::config_state::ConfigLoadState::Corrupt(message)
        | crate::config_state::ConfigLoadState::Error(message) => {
            format!("配置文件损坏：{message}")
        }
        crate::config_state::ConfigLoadState::Loaded => "SSH Router 配置".to_string(),
    };

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(div().text_2xl().child(title))
        .child(
            div()
                .flex()
                .justify_between()
                .items_center()
                .child(div().text_lg().child("端口路由"))
                .child(
                    gpui_component::button::Button::new("add-route")
                        .primary()
                        .label("添加路由")
                        .on_click(move |event, window, cx| (on_add)(event, window, cx)),
                ),
        )
        .child(route_table(&config.routes, theme, on_edit, on_delete))
        .child(sftp_editor(
            &inputs.sftp_command,
            config.field_errors().get("sftp_command").cloned(),
            theme,
        ))
        .child(route_editor(
            editor, inputs, theme, on_save, on_cancel, on_default,
        ))
        .into_any_element()
}
