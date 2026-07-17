use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, PostMessageW, RegisterWindowMessageW, SetForegroundWindow, ShowWindow,
    HWND_BROADCAST, SW_RESTORE, SW_SHOW,
};

#[derive(Debug)]
pub enum SingleInstanceError {
    CreateMutex(windows::core::Error),
}

pub enum SingleInstance {
    Primary(SingleInstanceGuard),
    AlreadyRunning,
}

pub struct SingleInstanceGuard {
    handle: HANDLE,
}

impl SingleInstanceGuard {
    pub fn acquire(name: &str) -> Result<SingleInstance, SingleInstanceError> {
        let name = wide(name);
        let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
            .map_err(SingleInstanceError::CreateMutex)?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Ok(SingleInstance::AlreadyRunning);
        }
        Ok(SingleInstance::Primary(Self { handle }))
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

pub fn activate_existing_window(title: &str) -> bool {
    let activation_message =
        unsafe { RegisterWindowMessageW(windows::core::w!("SSHRouter.ActivateMainWindow")) };
    if activation_message != 0
        && unsafe {
            PostMessageW(
                Some(HWND_BROADCAST),
                activation_message,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(0),
            )
        }
        .is_ok()
    {
        return true;
    }
    let title = wide(title);
    let Ok(hwnd) = (unsafe { FindWindowW(None, PCWSTR(title.as_ptr())) }) else {
        return false;
    };
    if hwnd == HWND::default() {
        return false;
    }
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd).as_bool()
    }
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_reports_already_running() {
        let name = format!("Local\\SSH-Router-Test-{}", std::process::id());
        let first = SingleInstanceGuard::acquire(&name).expect("首次获取应成功");
        assert!(matches!(first, SingleInstance::Primary(_)));

        let second = SingleInstanceGuard::acquire(&name).expect("第二次获取应返回状态");
        assert!(matches!(second, SingleInstance::AlreadyRunning));
    }
}
