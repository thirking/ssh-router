use gpui::{div, prelude::*, AnyElement, IntoElement};
use gpui_component::{button::Button, Disableable, Sizable};

use super::status_panel::StatusAction;
use crate::{system_status_state::SystemOperation, theme::AppTheme};

pub fn quick_actions(
    running: Option<SystemOperation>,
    theme: AppTheme,
    on_install_cli: StatusAction,
    on_default_shell: StatusAction,
    on_restart_sshd: StatusAction,
) -> AnyElement {
    let busy = running.is_some();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .child(div().text_lg().child("快捷操作"))
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("install-cli")
                        .small()
                        .label(if running == Some(SystemOperation::InstallCli) {
                            "同步中…"
                        } else {
                            "同步 CLI"
                        })
                        .disabled(busy)
                        .on_click(move |event, window, cx| (on_install_cli)(event, window, cx)),
                )
                .child(
                    Button::new("set-default-shell")
                        .small()
                        .label(if running == Some(SystemOperation::SetDefaultShell) {
                            "设置中…"
                        } else {
                            "设置 DefaultShell"
                        })
                        .disabled(busy)
                        .on_click(move |event, window, cx| (on_default_shell)(event, window, cx)),
                )
                .child(
                    Button::new("restart-sshd")
                        .small()
                        .label(if running == Some(SystemOperation::RestartSshd) {
                            "重启中…"
                        } else {
                            "重启 sshd"
                        })
                        .disabled(busy)
                        .on_click(move |event, window, cx| (on_restart_sshd)(event, window, cx)),
                ),
        )
        .into_any_element()
}
