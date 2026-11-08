use std::cell::RefCell;
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

use napi_derive::napi;
use ssh_router_app_core::platform::windows::{
    single_instance::{activate_existing_window, SingleInstance, SingleInstanceGuard},
    tray::{take_exit_requested, TrayIcon},
};
use windows::core::{w, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsZoomed, PostMessageW,
    RegisterWindowMessageW, SendMessageTimeoutW, SetWindowsHookExW, ShowWindow,
    UnhookWindowsHookEx, CWPSTRUCT, HTCAPTION, HTCLIENT, SC_MAXIMIZE, SC_RESTORE, SMTO_ABORTIFHUNG,
    SW_HIDE, WH_CALLWNDPROC, WM_CLOSE, WM_NCDESTROY, WM_NCHITTEST, WM_SYSCOMMAND,
};

const WINDOW_SUBCLASS: usize = 2;
static ATTACH_MESSAGE: AtomicU32 = AtomicU32::new(0);
static ATTACH_RESULT: AtomicI32 = AtomicI32::new(0);
static WINDOW: AtomicUsize = AtomicUsize::new(0);
static TITLEBAR: Mutex<Option<TitlebarRegion>> = Mutex::new(None);

#[napi(object)]
pub struct TitlebarRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[napi]
pub fn set_titlebar_region(region: Option<TitlebarRegion>) {
    *TITLEBAR.lock().unwrap() = region;
}

#[napi]
pub fn is_window_maximized() -> bool {
    let handle = WINDOW.load(Ordering::Acquire);
    handle != 0 && unsafe { IsZoomed(HWND(handle as _)).as_bool() }
}

#[napi]
pub fn toggle_maximize_window() -> napi::Result<()> {
    post_window_message(
        WM_SYSCOMMAND,
        if is_window_maximized() {
            SC_RESTORE
        } else {
            SC_MAXIMIZE
        } as usize,
    )
}

#[napi]
pub fn close_window() -> napi::Result<()> {
    let handle = WINDOW.load(Ordering::Acquire);
    if handle == 0 {
        return Err(napi::Error::from_reason("窗口尚未就绪"));
    }
    unsafe {
        let _ = ShowWindow(HWND(handle as _), SW_HIDE);
    }
    Ok(())
}

fn post_window_message(message: u32, command: usize) -> napi::Result<()> {
    let handle = WINDOW.load(Ordering::Acquire);
    if handle == 0 {
        return Err(napi::Error::from_reason("窗口尚未就绪"));
    }
    unsafe { PostMessageW(Some(HWND(handle as _)), message, WPARAM(command), LPARAM(0)) }
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

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

/// 在 GPUI 的窗口线程安装托盘、标题栏命中和关闭处理器。
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
                if !SetWindowSubclass(message.hwnd, Some(window_subclass), WINDOW_SUBCLASS, 0)
                    .as_bool()
                {
                    return false;
                }
                *slot.borrow_mut() = Some(tray);
                WINDOW.store(message.hwnd.0 as usize, Ordering::Release);
                true
            });
            ATTACH_RESULT.store(if installed { 1 } else { -1 }, Ordering::Release);
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn window_subclass(
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
        WINDOW.store(0, Ordering::Release);
        let _ = RemoveWindowSubclass(hwnd, Some(window_subclass), WINDOW_SUBCLASS);
        TRAY.with(|slot| {
            slot.borrow_mut().take();
        });
    }
    let result = DefSubclassProc(hwnd, message, wparam, lparam);
    // 保留 GPUI 的边缘缩放命中，仅将客户区内的标题区域交给系统拖动。
    if message == WM_NCHITTEST && result.0 == HTCLIENT as isize {
        let mut point = POINT {
            x: lparam.0 as i16 as i32,
            y: (lparam.0 >> 16) as i16 as i32,
        };
        if ScreenToClient(hwnd, &mut point).as_bool() {
            let scale = GetDpiForWindow(hwnd) as f64 / 96.0;
            if scale > 0.0 {
                if let Some(region) = TITLEBAR.lock().unwrap().as_ref() {
                    let x = point.x as f64 / scale;
                    let y = point.y as f64 / scale;
                    if x >= region.x
                        && x < region.x + region.width
                        && y >= region.y
                        && y < region.y + region.height
                    {
                        return LRESULT(HTCAPTION as isize);
                    }
                }
            }
        }
    }
    result
}
