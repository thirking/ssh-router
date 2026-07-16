use gpui::{
    div, prelude::*, px, size, App, Application, Bounds, Context, FontWeight, KeyBinding, Window,
    WindowBounds, WindowOptions,
};
use gpui_component::Root;

use crate::actions::{CloseDemoModal, FocusNext, FocusPrevious, OpenDemoModal, APP_KEY_CONTEXT};
use crate::app_state::{AppState, NotificationKind};
use crate::components::{
    app_button, modal_overlay, status_badge, text_field, toast_host, ButtonVariant, StatusKind,
};
use crate::theme::AppTheme;

impl AppState {
    fn on_focus_next(&mut self, _: &FocusNext, window: &mut Window, _: &mut Context<Self>) {
        window.focus_next();
    }

    fn on_focus_previous(&mut self, _: &FocusPrevious, window: &mut Window, _: &mut Context<Self>) {
        window.focus_prev();
    }

    fn on_open_modal(&mut self, _: &OpenDemoModal, _: &mut Window, cx: &mut Context<Self>) {
        self.open_modal(cx);
    }

    fn on_close_modal(&mut self, _: &CloseDemoModal, _: &mut Window, cx: &mut Context<Self>) {
        self.close_modal(cx);
    }
}

impl Render for AppState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = AppTheme::default();
        let content = div()
            .id("ssh-router-app")
            .key_context(APP_KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_focus_next))
            .on_action(cx.listener(Self::on_focus_previous))
            .on_action(cx.listener(Self::on_open_modal))
            .on_action(cx.listener(Self::on_close_modal))
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        app_button(
                            "primary-action",
                            "显示成功通知",
                            ButtonVariant::Primary,
                            false,
                            &self.primary_focus,
                            window,
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.notify(NotificationKind::Success, "基础组件状态正常", cx);
                        })),
                    )
                    .child(
                        app_button(
                            "modal-action",
                            "打开模态框",
                            ButtonVariant::Secondary,
                            false,
                            &self.secondary_focus,
                            window,
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.open_modal(cx))),
                    )
                    .child(
                        app_button(
                            "error-action",
                            "显示错误通知",
                            ButtonVariant::Danger,
                            false,
                            &self.danger_focus,
                            window,
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.notify(NotificationKind::Error, "错误通知样式正常", cx);
                        })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(status_badge("待优化", StatusKind::Warning, theme))
                    .child(status_badge("异常示例", StatusKind::Error, theme))
                    .child(status_badge("未检查", StatusKind::Neutral, theme)),
            )
            .child(toast_host(&self.notifications, theme));

        content.when_some(self.modal.clone(), |content, modal| {
            content.child(modal_overlay(
                modal.title,
                modal.body,
                app_button(
                    "close-modal",
                    "关闭",
                    ButtonVariant::Primary,
                    false,
                    &self.primary_focus,
                    window,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                theme,
            ))
        })
    }
}

pub fn run() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.bind_keys([
            KeyBinding::new("tab", FocusNext, Some(APP_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", FocusPrevious, Some(APP_KEY_CONTEXT)),
            KeyBinding::new("cmd-enter", OpenDemoModal, Some(APP_KEY_CONTEXT)),
            KeyBinding::new("escape", CloseDemoModal, Some(APP_KEY_CONTEXT)),
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
            |window, cx| {
                let app = cx.new(|cx| AppState::new(window, cx));
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
