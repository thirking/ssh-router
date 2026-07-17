use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{LoadIconW, HICON, IDI_APPLICATION, WM_USER};

const TRAY_MESSAGE: u32 = WM_USER + 42;

pub struct TrayIcon {
    hwnd: HWND,
}

impl TrayIcon {
    pub fn install(hwnd: HWND, tooltip: &str) -> windows::core::Result<Self> {
        let data = notify_data(hwnd, tooltip);
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &data).as_bool() };
        if !added {
            return Err(windows::core::Error::from_win32());
        }
        Ok(Self { hwnd })
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        let data = notify_data(self.hwnd, "");
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
        }
    }
}

fn notify_data(hwnd: HWND, tooltip: &str) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: TRAY_MESSAGE,
        ..Default::default()
    };
    data.hIcon = unsafe { LoadIconW(None, IDI_APPLICATION).unwrap_or(HICON::default()) };
    for (slot, value) in data.szTip.iter_mut().zip(tooltip.encode_utf16()) {
        *slot = value;
    }
    data
}
