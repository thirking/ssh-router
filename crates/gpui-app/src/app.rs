use gpui::ClickEvent;
use gpui::{
    div, prelude::*, px, size, App, Application, Bounds, Context, FontWeight, KeyBinding, Window,
    WindowBounds, WindowOptions,
};
use gpui_component::Root;
use std::rc::Rc;

use crate::actions::{
    CloseDemoModal, CreateDefaultConfig, FocusNext, FocusPrevious, OpenDemoModal, SaveConfig,
    APP_KEY_CONTEXT,
};
use crate::app_state::{AppState, NotificationKind};
use crate::components::{
    app_button, checkbox, modal_overlay, radio, status_badge, text_field, toast_host,
    ButtonVariant, StatusKind,
};
use crate::system_status_state::SystemOperation;
use crate::theme::AppTheme;
use crate::views::config_main_window;
use crate::views::{
    cli_sync_dialog::cli_sync_dialog, quick_actions::quick_actions, status_panel::status_panel,
};

impl AppState {
    fn on_focus_next(&mut self, _: &FocusNext, window: &mut Window, _: &mut Context<Self>) {
        window.focus_next();
    }

    fn on_focus_previous(&mut self, _: &FocusPrevious, window: &mut Window, _: &mut Context<Self>) {
        window.focus_prev();
    }

    fn on_open_modal(&mut self, _: &OpenDemoModal, window: &mut Window, cx: &mut Context<Self>) {
        self.open_modal(cx);
        window.focus(&self.modal_focus);
    }

    fn on_close_modal(&mut self, _: &CloseDemoModal, window: &mut Window, cx: &mut Context<Self>) {
        self.close_modal(cx);
        window.focus(&self.primary_focus);
    }

    fn on_save_config(&mut self, _: &SaveConfig, _: &mut Window, cx: &mut Context<Self>) {
        #[cfg(windows)]
        self.save_config(cx);
        #[cfg(not(windows))]
        self.notify(NotificationKind::Info, "配置保存仅支持 Windows", cx);
    }

    fn on_create_default_config(
        &mut self,
        _: &CreateDefaultConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(windows)]
        self.create_default_config(window, cx);
        #[cfg(not(windows))]
        self.notify(NotificationKind::Info, "创建配置仅支持 Windows", cx);
    }
}

impl Render for AppState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = AppTheme::default();
        let entity = cx.entity().downgrade();
        let on_edit = Rc::new({
            let entity = entity.clone();
            move |id: u64, _: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| state.edit_route(id, window, cx));
            }
        });
        let on_delete = Rc::new(
            move |id: u64, _: &ClickEvent, _: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| state.delete_route(id, cx));
            },
        );
        let on_add = Rc::new({
            let entity = cx.entity().downgrade();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| state.start_new_route(window, cx));
            }
        });
        let on_save_route = Rc::new({
            let entity = cx.entity().downgrade();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| state.save_route_edit(window, cx));
            }
        });
        let on_cancel_route = Rc::new({
            let entity = cx.entity().downgrade();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| state.cancel_route_edit(window, cx));
            }
        });
        let on_default = Rc::new({
            let entity = cx.entity().downgrade();
            move |checked: &bool, _: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| {
                    state.route_editor.route.default = *checked;
                    state.refresh_route_validation();
                    cx.notify();
                });
            }
        });
        let system_action = |operation: SystemOperation| {
            let entity = cx.entity().downgrade();
            Rc::new(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| {
                    state.request_system_operation(operation, window, cx)
                });
            })
        };
        let on_status_refresh = {
            let entity = cx.entity().downgrade();
            Rc::new(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| state.request_status_refresh(window, cx));
            })
        };
        let on_install_cli = system_action(SystemOperation::InstallCli);
        let on_default_shell = system_action(SystemOperation::SetDefaultShell);
        let on_restart_sshd = system_action(SystemOperation::RestartSshd);
        let on_dismiss_cli_prompt = {
            let entity = cx.entity().downgrade();
            Rc::new(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |state, cx| state.dismiss_cli_sync_prompt(cx));
            })
        };
        let on_sync_from_prompt = on_install_cli.clone();
        let content = div()
            .id("ssh-router-app")
            .key_context(APP_KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_focus_next))
            .on_action(cx.listener(Self::on_focus_previous))
            .on_action(cx.listener(Self::on_open_modal))
            .on_action(cx.listener(Self::on_close_modal))
            .on_action(cx.listener(Self::on_save_config))
            .on_action(cx.listener(Self::on_create_default_config))
            .relative()
            .size_full()
            .min_w(px(640.0))
            .min_h(px(520.0))
            .flex()
            .flex_col()
            .gap_5()
            .p_6()
            .bg(theme.background)
            .text_color(theme.text)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("SSH Router"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .child("GPUI 基础组件与状态层"),
                            ),
                    )
                    .child(status_badge("GPUI MVP", StatusKind::Success, theme)),
            )
            .child(text_field("路由名称", &self.input, theme))
            .child(status_panel(&self.system, theme, on_status_refresh))
            .child(quick_actions(
                self.system.running,
                theme,
                on_install_cli,
                on_default_shell,
                on_restart_sshd,
            ))
            .child(config_main_window(
                &self.config,
                &self.route_editor,
                &self.route_inputs,
                theme,
                on_edit,
                on_delete,
                on_add,
                on_save_route,
                on_cancel_route,
                on_default,
            ))
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(app_button(
                        "save-config",
                        if self.config.is_dirty {
                            "保存配置 *"
                        } else {
                            "保存配置"
                        },
                        ButtonVariant::Primary,
                        matches!(
                            self.config.load_state,
                            crate::config_state::ConfigLoadState::Loading
                        ),
                        &self.save_focus,
                        window,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            #[cfg(windows)]
                            this.save_config(cx);
                            #[cfg(not(windows))]
                            this.notify(NotificationKind::Info, "配置保存仅支持 Windows", cx);
                        }),
                    ))
                    .when(
                        !matches!(
                            self.config.load_state,
                            crate::config_state::ConfigLoadState::Loaded
                        ),
                        |content| {
                            content.child(app_button(
                                "create-default-config",
                                "创建默认配置",
                                ButtonVariant::Secondary,
                                false,
                                &self.create_focus,
                                window,
                                theme,
                                cx.listener(|this, _, window, cx| {
                                    #[cfg(windows)]
                                    this.create_default_config(window, cx);
                                    #[cfg(not(windows))]
                                    this.notify(
                                        NotificationKind::Info,
                                        "创建配置仅支持 Windows",
                                        cx,
                                    );
                                }),
                            ))
                        },
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(app_button(
                        "primary-action",
                        "显示成功通知",
                        ButtonVariant::Primary,
                        false,
                        &self.primary_focus,
                        window,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.notify(NotificationKind::Success, "基础组件状态正常", cx);
                        }),
                    ))
                    .child(app_button(
                        "modal-action",
                        "打开模态框",
                        ButtonVariant::Secondary,
                        false,
                        &self.secondary_focus,
                        window,
                        theme,
                        cx.listener(|this, _, window, cx| {
                            this.open_modal(cx);
                            window.focus(&this.modal_focus);
                        }),
                    ))
                    .child(app_button(
                        "error-action",
                        "显示错误通知",
                        ButtonVariant::Danger,
                        false,
                        &self.danger_focus,
                        window,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.notify(NotificationKind::Error, "错误通知样式正常", cx);
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(checkbox("sftp-choice", "启用 SFTP", true, false))
                    .child(radio("shell-choice", "设为默认路由", true, false)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(status_badge("待优化", StatusKind::Warning, theme))
                    .child(status_badge("异常示例", StatusKind::Error, theme))
                    .child(status_badge("未检查", StatusKind::Neutral, theme)),
            )
            .child(toast_host(&self.notifications, theme))
            .child(cli_sync_dialog(
                self.system.cli_sync_prompt_open,
                theme,
                on_sync_from_prompt,
                on_dismiss_cli_prompt,
            ));

        content.when_some(self.modal.clone(), |content, modal| {
            content.child(modal_overlay(
                modal.title,
                modal.body,
                app_button(
                    "close-modal",
                    "关闭",
                    ButtonVariant::Primary,
                    false,
                    &self.modal_focus,
                    window,
                    theme,
                    cx.listener(|this, _, window, cx| {
                        this.close_modal(cx);
                        window.focus(&this.primary_focus);
                    }),
                ),
                &self.modal_focus,
                theme,
            ))
        })
    }
}

pub fn run() {
    #[cfg(windows)]
    let single_instance = match ssh_router_app_core::platform::windows::single_instance::SingleInstanceGuard::acquire(
        "Global\\SSH Router",
    ) {
        Ok(ssh_router_app_core::platform::windows::single_instance::SingleInstance::Primary(
            guard,
        )) => guard,
        Ok(ssh_router_app_core::platform::windows::single_instance::SingleInstance::AlreadyRunning) => {
            let _ = ssh_router_app_core::platform::windows::single_instance::activate_existing_window(
                "SSH Router",
            );
            return;
        }
        Err(error) => {
            eprintln!("无法初始化单实例：{error:?}");
            return;
        }
    };

    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);
        cx.bind_keys([
            KeyBinding::new("tab", FocusNext, Some(APP_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", FocusPrevious, Some(APP_KEY_CONTEXT)),
            KeyBinding::new("cmd-enter", OpenDemoModal, Some(APP_KEY_CONTEXT)),
            KeyBinding::new("escape", CloseDemoModal, Some(APP_KEY_CONTEXT)),
            KeyBinding::new("cmd-s", SaveConfig, Some(APP_KEY_CONTEXT)),
            KeyBinding::new("cmd-shift-n", CreateDefaultConfig, Some(APP_KEY_CONTEXT)),
        ]);

        let bounds = Bounds::centered(None, size(px(800.0), px(680.0)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("SSH Router".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                let app = cx.new(|cx| {
                    AppState::new(
                        window,
                        cx,
                        #[cfg(windows)]
                        single_instance,
                    )
                });
                let tray_available = app.read(cx).tray_available();
                crate::window_controller::install_close_to_tray(window, cx, tray_available);
                #[cfg(windows)]
                app.update(cx, |state, cx| {
                    state.start_config_load(window, cx);
                    state.start_status_refresh(window, cx);
                });
                cx.new(|cx| Root::new(app, window, cx))
            },
        );

        if let Err(error) = result {
            eprintln!("无法创建 GPUI 窗口: {error}");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}
