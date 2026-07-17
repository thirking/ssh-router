#![allow(dead_code)]

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ClickEvent, IntoElement, Window};
use gpui_component::{button::Button, Disableable, Sizable};

use crate::{
    theme::AppTheme,
    update_state::{UpdatePhase, UpdateState},
};

pub type UpdateAction = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

pub fn update_panel(state: &UpdateState, theme: AppTheme, on_check: UpdateAction) -> AnyElement {
    let label = match state.phase {
        UpdatePhase::Idle => "检查更新",
        UpdatePhase::Checking => "检查中…",
        UpdatePhase::Available => "发现新版本",
        UpdatePhase::Downloading => "下载中…",
        UpdatePhase::Installing => "安装中…",
        UpdatePhase::Error => "重试更新检查",
    };
    div()
        .flex()
        .items_center()
        .justify_between()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .child(div().text_sm().child(if state.current_version.is_empty() {
            "更新：未检查"
        } else {
            "更新检查"
        }))
        .child(
            Button::new("check-updates")
                .small()
                .label(label)
                .disabled(matches!(
                    state.phase,
                    UpdatePhase::Checking | UpdatePhase::Downloading | UpdatePhase::Installing
                ))
                .on_click(move |event, window, cx| (on_check)(event, window, cx)),
        )
        .into_any_element()
}
