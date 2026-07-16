use gpui::{
    actions, div, prelude::*, px, rgb, size, App, Application, Bounds, Context, Div, ElementId,
    FocusHandle, FontWeight, KeyBinding, SharedString, Stateful, Window, WindowBounds,
    WindowOptions,
};

actions!(ssh_router, [FocusNext, FocusPrevious]);

struct SpikeView {
    click_count: usize,
    focus_handle: FocusHandle,
    primary_button_focus: FocusHandle,
    secondary_button_focus: FocusHandle,
    status: SharedString,
}

impl SpikeView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let primary_button_focus = cx.focus_handle().tab_index(1).tab_stop(true);
        let secondary_button_focus = cx.focus_handle().tab_index(2).tab_stop(true);

        window.focus(&primary_button_focus);

        Self {
            click_count: 0,
            focus_handle,
            primary_button_focus,
            secondary_button_focus,
            status: "等待交互验证".into(),
        }
    }

    fn focus_next(&mut self, _: &FocusNext, window: &mut Window, _: &mut Context<Self>) {
        window.focus_next();
    }

    fn focus_previous(&mut self, _: &FocusPrevious, window: &mut Window, _: &mut Context<Self>) {
        window.focus_prev();
    }

    fn activate_primary(&mut self, cx: &mut Context<Self>) {
        self.click_count += 1;
        self.status = format!("主按钮已触发 {} 次", self.click_count).into();
        cx.notify();
    }

    fn activate_secondary(&mut self, cx: &mut Context<Self>) {
        self.status = "辅助按钮已触发，鼠标与焦点事件正常".into();
        cx.notify();
    }

    fn render_button(
        &self,
        id: impl Into<ElementId>,
        label: &'static str,
        focus_handle: &FocusHandle,
        window: &Window,
    ) -> Stateful<Div> {
        let focused = focus_handle.is_focused(window);

        div()
            .id(id)
            .track_focus(focus_handle)
            .tab_stop(true)
            .px_4()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(if focused {
                rgb(0x60a5fa)
            } else {
                rgb(0x475569)
            })
            .bg(if focused {
                rgb(0x1d4ed8)
            } else {
                rgb(0x1e293b)
            })
            .text_color(rgb(0xf8fafc))
            .cursor_pointer()
            .child(label)
    }
}

impl Render for SpikeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("gpui-spike")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::focus_next))
            .on_action(cx.listener(Self::focus_previous))
            .size_full()
            .min_w(px(640.0))
            .min_h(px(480.0))
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(rgb(0x0f172a))
            .text_color(rgb(0xe2e8f0))
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("SSH Router · GPUI Windows 验证"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x94a3b8))
                    .child("验证中文渲染、窗口缩放、鼠标点击和 Tab/Shift-Tab 焦点。"),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        self.render_button(
                            "primary-button",
                            "主操作",
                            &self.primary_button_focus,
                            window,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.activate_primary(cx))),
                    )
                    .child(
                        self.render_button(
                            "secondary-button",
                            "辅助操作",
                            &self.secondary_button_focus,
                            window,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.activate_secondary(cx))),
                    ),
            )
            .child(
                div()
                    .mt_2()
                    .p_4()
                    .rounded_md()
                    .bg(rgb(0x1e293b))
                    .child(self.status.clone()),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("tab", FocusNext, None),
            KeyBinding::new("shift-tab", FocusPrevious, None),
        ]);

        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        let window_result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("SSH Router GPUI Spike".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| SpikeView::new(window, cx)),
        );

        if let Err(error) = window_result {
            eprintln!("无法创建 GPUI 窗口: {error}");
            cx.quit();
            return;
        }

        cx.activate(true);
    });
}
