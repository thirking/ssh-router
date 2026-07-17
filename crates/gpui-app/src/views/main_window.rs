use gpui::{div, prelude::*, AnyElement, IntoElement};

use crate::{config_state::ConfigState, route_editor_state::RouteEditorState, theme::AppTheme};

use super::{route_editor::route_editor, route_table::route_table, sftp_editor::sftp_editor};

pub fn config_main_window(
    config: &ConfigState,
    editor: &RouteEditorState,
    theme: AppTheme,
) -> AnyElement {
    let title = match &config.load_state {
        crate::config_state::ConfigLoadState::Loading => "正在加载配置".to_string(),
        crate::config_state::ConfigLoadState::NotFound => "未找到配置文件".to_string(),
        crate::config_state::ConfigLoadState::Corrupt(message) => {
            format!("配置文件损坏：{message}")
        }
        crate::config_state::ConfigLoadState::Loaded => "SSH Router 配置".to_string(),
    };

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(div().text_2xl().child(title))
        .child(route_table(&config.routes, theme))
        .child(sftp_editor(config.sftp_command.clone(), theme))
        .child(route_editor(editor, theme))
        .into_any_element()
}
