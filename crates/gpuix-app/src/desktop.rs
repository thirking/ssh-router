use std::cell::RefCell;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use napi_derive::napi;
use ssh_router_app_core::platform::windows::{
    single_instance::{activate_existing_window, SingleInstance, SingleInstanceGuard},
    tray::{take_exit_requested, TrayIcon},
};
use windows::core::{w, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EnumWindows, GetWindowTextW, GetWindowThreadProcessId, RegisterWindowMessageW,
    SendMessageTimeoutW, SetWindowsHookExW, ShowWindow, UnhookWindowsHookEx, CWPSTRUCT,
    SMTO_ABORTIFHUNG, SW_HIDE, WH_CALLWNDPROC, WM_CLOSE, WM_NCDESTROY,
};

const CLOSE_SUBCLASS: usize = 2;
static ATTACH_MESSAGE: AtomicU32 = AtomicU32::new(0);
static ATTACH_RESULT: AtomicI32 = AtomicI32::new(0);

thread_local! {
    static INSTANCE: RefCell<Option<SingleInstanceGuard>> = const { RefCell::new(None) };
    static TRAY: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
}

#[napi]
pub fn acquire_instance(name: Option<String>) -> napi::Result<bool> {
    match SingleInstanceGuard::acquire(name.as_deref().unwrap_or("Global\\SSH Router")) {
        Ok(SingleInstance::Primary(guard)) => {
            INSTANCE.with(|slot| *slot.borrow_mut() = Some(guard));
            Ok(true)
        }
        Ok(SingleInstance::AlreadyRunning) => {
            activate_existing_window("SSH Router");
            Ok(false)
        }
        Err(error) => Err(napi::Error::from_reason(format!(
            "单实例初始化失败: {error:?}"
        ))),
    }
}

/// 在 GPUI 的窗口线程安装托盘和关闭处理器。
#[napi]
pub fn attach_window() -> napi::Result<bool> {
    unsafe {
        let mut hwnd = HWND::default();
        let _ = EnumWindows(Some(find_window), LPARAM(&mut hwnd as *mut HWND as isize));
        if hwnd == HWND::default() {
            return Ok(false);
        }
        let mut process_id = 0;
        let thread_id = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if process_id != GetCurrentProcessId() || thread_id == 0 {
            return Ok(false);
        }
        let message = RegisterWindowMessageW(w!("SSHRouter.GpuixAttach"));
        if message == 0 {
            return Err(napi::Error::from_reason("注册窗口消息失败"));
        }
        ATTACH_MESSAGE.store(message, Ordering::Release);
        ATTACH_RESULT.store(0, Ordering::Release);
        let hook = SetWindowsHookExW(WH_CALLWNDPROC, Some(attach_hook), None, thread_id)
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        let result = SendMessageTimeoutW(
            hwnd,
            message,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            2000,
            None,
        );
        let _ = UnhookWindowsHookEx(hook);
        if result.0 == 0 || ATTACH_RESULT.load(Ordering::Acquire) != 1 {
            return Err(napi::Error::from_reason("安装托盘失败，主窗口保持可见"));
        }
        Ok(true)
    }
}

unsafe extern "system" fn find_window(hwnd: HWND, data: LPARAM) -> BOOL {
    let mut process_id = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    if process_id == GetCurrentProcessId() {
        let mut title = [0_u16; 128];
        let count = GetWindowTextW(hwnd, &mut title);
        if String::from_utf16_lossy(&title[..count as usize]) == "SSH Router" {
            *(data.0 as *mut HWND) = hwnd;
            return BOOL(0);
        }
    }
    BOOL(1)
}

unsafe extern "system" fn attach_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let message = &*(lparam.0 as *const CWPSTRUCT);
        if message.message == ATTACH_MESSAGE.load(Ordering::Acquire) {
            let installed = TRAY.with(|slot| {
                if slot.borrow().is_some() {
                    return true;
                }
                let tray = match TrayIcon::install(message.hwnd, "SSH Router") {
                    Ok(tray) => tray,
                    Err(error) => {
                        eprintln!("{error}");
                        return false;
                    }
                };
                if !SetWindowSubclass(message.hwnd, Some(close_to_tray), CLOSE_SUBCLASS, 0)
                    .as_bool()
                {
                    return false;
                }
                *slot.borrow_mut() = Some(tray);
                true
            });
            ATTACH_RESULT.store(if installed { 1 } else { -1 }, Ordering::Release);
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn close_to_tray(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if message == WM_CLOSE && !take_exit_requested() {
        let _ = ShowWindow(hwnd, SW_HIDE);
        return LRESULT(0);
    }
    if message == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(close_to_tray), CLOSE_SUBCLASS);
        TRAY.with(|slot| {
            slot.borrow_mut().take();
        });
    }
    DefSubclassProc(hwnd, message, wparam, lparam)
}
