use gpui::{App, Window};

#[cfg(windows)]
pub fn install_close_to_tray(window: &Window, cx: &App, tray_available: bool) {
    window.on_window_should_close(cx, move |window, _| {
        if ssh_router_app_core::platform::windows::tray::take_exit_requested() {
            return true;
        }
        if !tray_available {
            return true;
        }
        let _ = hide_native_window(window);
        false
    });
}

#[cfg(not(windows))]
pub fn install_close_to_tray(_: &Window, _: &App, _: bool) {}

#[cfg(windows)]
pub fn native_hwnd(window: &Window) -> Result<windows::Win32::Foundation::HWND, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("当前窗口不是 Win32 窗口".to_string());
    };
    Ok(windows::Win32::Foundation::HWND(
        handle.hwnd.get() as *mut std::ffi::c_void
    ))
}

#[cfg(windows)]
fn hide_native_window(window: &Window) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    let hwnd = native_hwnd(window)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
    Ok(())
}
