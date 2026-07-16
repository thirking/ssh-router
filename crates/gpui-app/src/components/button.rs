use gpui::{div, prelude::*, Div, ElementId, FocusHandle, Stateful, Window};

use crate::theme::AppTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
}

pub fn app_button(
    id: impl Into<ElementId>,
    label: impl Into<gpui::SharedString>,
    variant: ButtonVariant,
    disabled: bool,
    focus_handle: &FocusHandle,
    window: &Window,
    theme: AppTheme,
) -> Stateful<Div> {
    let focused = focus_handle.is_focused(window);
    let background = match variant {
        ButtonVariant::Primary => theme.primary,
        ButtonVariant::Secondary => theme.surface,
        ButtonVariant::Danger => theme.danger,
    };
    let hover = match variant {
        ButtonVariant::Primary => theme.primary_hover,
        ButtonVariant::Secondary => theme.surface_hover,
        ButtonVariant::Danger => theme.danger.opacity(0.85),
    };

    div()
        .id(id)
        .track_focus(focus_handle)
        .tab_stop(!disabled)
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(if focused { theme.focus } else { theme.border })
        .bg(background)
        .text_color(theme.text)
        .when(!disabled, |element| {
            element.cursor_pointer().hover(move |style| style.bg(hover))
        })
        .when(disabled, |element| element.opacity(0.45))
        .child(label.into())
}
