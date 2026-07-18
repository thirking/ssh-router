use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, Shell_NotifyIconW, NIF_ICON,
    NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, PostMessageW,
    RegisterWindowMessageW, SetForegroundWindow, ShowWindow, TrackPopupMenu, IDI_APPLICATION,
    MF_STRING, SW_RESTORE, SW_SHOW, TPM_RETURNCMD, WM_CLOSE, WM_LBUTTONDBLCLK, WM_RBUTTONUP,
    WM_USER,
};

use crate::{AppResult, PlatformError};

const TRAY_MESSAGE: u32 = WM_USER + 42;
const TRAY_ID: usize = 1;
const OPEN_COMMAND: usize = 1001;
const EXIT_COMMAND: usize = 1002;
static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
static TASKBAR_CREATED_MESSAGE: AtomicU32 = AtomicU32::new(0);
static ACTIVATE_MESSAGE: AtomicU32 = AtomicU32::new(0);

pub struct TrayIcon {
    hwnd: HWND,
}

impl TrayIcon {
    pub fn install(hwnd: HWND, tooltip: &str) -> AppResult<Self> {
        let taskbar_created = register_message(windows::core::w!("TaskbarCreated"))?;
        TASKBAR_CREATED_MESSAGE.store(taskbar_created, Ordering::Release);
        let activate = register_message(windows::core::w!("SSHRouter.ActivateMainWindow"))?;
        ACTIVATE_MESSAGE.store(activate, Ordering::Release);
        let data = notify_data(hwnd, tooltip)?;
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &data).as_bool() };
        if !added {
            return Err(tray_error("创建通知区域图标").into());
        }
        if !unsafe { SetWindowSubclass(hwnd, Some(tray_window_proc), TRAY_ID, 0).as_bool() } {
            let error = tray_error("安装托盘窗口消息处理器");
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            }
            return Err(error.into());
        }
        Ok(Self { hwnd })
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        let data = notify_identity(self.hwnd);
        unsafe {
            let _ = RemoveWindowSubclass(self.hwnd, Some(tray_window_proc), TRAY_ID);
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
        }
    }
}

pub fn take_exit_requested() -> bool {
    EXIT_REQUESTED.swap(false, Ordering::AcqRel)
}

unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    _reference_data: usize,
) -> LRESULT {
    if message == TASKBAR_CREATED_MESSAGE.load(Ordering::Acquire) {
        match notify_data(hwnd, "SSH Router") {
            Ok(data) => {
                if !Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
                    eprintln!("{}", tray_error("资源管理器重启后恢复托盘图标"));
                }
            }
            Err(error) => eprintln!("{error}"),
        }
    }
    if message == ACTIVATE_MESSAGE.load(Ordering::Acquire) {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    if message == TRAY_MESSAGE {
        match lparam.0 as u32 {
            WM_LBUTTONDBLCLK => {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetForegroundWindow(hwnd);
            }
            WM_RBUTTONUP => show_menu(hwnd),
            _ => {}
        }
    }
    DefSubclassProc(hwnd, message, wparam, lparam)
}

unsafe fn show_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        OPEN_COMMAND,
        windows::core::w!("打开主界面"),
    );
    let _ = AppendMenuW(menu, MF_STRING, EXIT_COMMAND, windows::core::w!("退出"));
    let mut point = POINT::default();
    let _ = GetCursorPos(&mut point);
    let _ = SetForegroundWindow(hwnd);
    let command = TrackPopupMenu(menu, TPM_RETURNCMD, point.x, point.y, None, hwnd, None).0;
    match command as usize {
        OPEN_COMMAND => {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
        EXIT_COMMAND => {
            EXIT_REQUESTED.store(true, Ordering::Release);
            if PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).is_err() {
                EXIT_REQUESTED.store(false, Ordering::Release);
            }
        }
        _ => {}
    }
    let _ = DestroyMenu(menu);
}

fn register_message(name: windows::core::PCWSTR) -> AppResult<u32> {
    let message = unsafe { RegisterWindowMessageW(name) };
    if message == 0 {
        return Err(tray_error("注册托盘窗口消息").into());
    }
    Ok(message)
}

fn tray_error(operation: &str) -> PlatformError {
    PlatformError::Tray {
        message: format!("{operation}失败: {}", windows::core::Error::from_win32()),
    }
}

fn notify_identity(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID as u32,
        ..Default::default()
    }
}

fn notify_data(hwnd: HWND, tooltip: &str) -> AppResult<NOTIFYICONDATAW> {
    let mut data = NOTIFYICONDATAW {
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: TRAY_MESSAGE,
        ..notify_identity(hwnd)
    };
    data.hIcon =
        unsafe { LoadIconW(None, IDI_APPLICATION) }.map_err(|error| PlatformError::Tray {
            message: format!("加载托盘图标失败: {error}"),
        })?;
    for (slot, value) in data.szTip.iter_mut().zip(tooltip.encode_utf16()) {
        *slot = value;
    }
    Ok(data)
}
