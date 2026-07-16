use gpui::actions;

actions!(
    ssh_router,
    [FocusNext, FocusPrevious, OpenDemoModal, CloseDemoModal]
);

pub const APP_KEY_CONTEXT: &str = "SshRouter";
