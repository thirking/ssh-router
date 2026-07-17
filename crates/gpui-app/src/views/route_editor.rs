use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ClickEvent, IntoElement, Window};
use gpui_component::{
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    input::Input,
};

use crate::{
    route_editor_state::{RouteEditorInputs, RouteEditorState},
    theme::AppTheme,
};

pub type EditorAction = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub type DefaultAction = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

pub fn route_editor(
    editor: &RouteEditorState,
    inputs: &RouteEditorInputs,
    theme: AppTheme,
    on_save: EditorAction,
    on_cancel: EditorAction,
    on_default: DefaultAction,
) -> AnyElement {
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
        .child(field("端口", Input::new(&inputs.port).w_full(), theme))
        .child(field("名称", Input::new(&inputs.name).w_full(), theme))
        .child(field(
            "Shell 路径",
            Input::new(&inputs.shell).w_full(),
            theme,
        ))
        .child(field(
            "交互式模板",
            Input::new(&inputs.interactive_template).w_full(),
            theme,
        ))
        .child(field(
            "命令模板",
            Input::new(&inputs.command_template).w_full(),
            theme,
        ))
        .child(field(
            "临时文件扩展名",
            Input::new(&inputs.tmp_file_ext).w_full(),
            theme,
        ))
        .child(
            Checkbox::new("route-default")
                .label("设为默认路由")
                .checked(editor.route.default)
                .on_click(move |checked, window, cx| (on_default)(checked, window, cx)),
        )
        .children(editor.errors.iter().map(|error| {
            div()
                .text_sm()
                .text_color(theme.danger)
                .child(error.clone())
        }))
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("save-route-edit")
                        .primary()
                        .label("应用")
                        .on_click(move |event, window, cx| (on_save)(event, window, cx)),
                )
                .child(
                    Button::new("cancel-route-edit")
                        .label("取消")
                        .on_click(move |event, window, cx| (on_cancel)(event, window, cx)),
                ),
        )
        .into_any_element()
}

fn field(label: &'static str, input: impl IntoElement, theme: AppTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_sm().text_color(theme.text_muted).child(label))
        .child(input)
}
