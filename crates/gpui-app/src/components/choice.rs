use gpui::ElementId;
use gpui_component::{checkbox::Checkbox, radio::Radio, Disableable};

pub fn checkbox(
    id: impl Into<ElementId>,
    label: impl Into<gpui_component::text::Text>,
    checked: bool,
    disabled: bool,
) -> Checkbox {
    Checkbox::new(id)
        .label(label)
        .checked(checked)
        .disabled(disabled)
}

pub fn radio(
    id: impl Into<ElementId>,
    label: impl Into<gpui_component::text::Text>,
    checked: bool,
    disabled: bool,
) -> Radio {
    Radio::new(id)
        .label(label)
        .checked(checked)
        .disabled(disabled)
}
