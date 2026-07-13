//! Win32 API 封装：CreateProcessW + Job Object。
//!
//! 本模块仅在 Windows 上编译。`windows` crate 是 target-specific 依赖
//! （见 `Cargo.toml` 的 `[target.'cfg(windows)'.dependencies]`），
//! 故整个模块用 `cfg(windows)` 守卫，避免在 macOS 上因缺少 `windows` crate 而编译失败。
//!
//! 设计要点（移植自 C# `SshRouter.cs`）：
//! - 用 `CreateProcessW` 启动子进程（不是 `std::process::Command`），以便精确控制
//!   句柄继承与挂起状态。
//! - 用 `CREATE_SUSPENDED` 启动，先把子进程加入 Job Object 再 `ResumeThread`，
//!   确保子进程不会在加入 Job 之前脱离父进程控制。
//! - `bInheritHandles = true` 且不设 `STARTF_USESTDHANDLES`，让子进程自动继承
//!   父进程（sshd）的 stdin/stdout/stderr，从而 SSH 会话保持连通。
//! - Job Object 设 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，父进程退出时 Job 句柄关闭，
//!   子进程随之被杀，避免遗留孤儿进程。
//!
//! 公共 API 以 `isize` 传递句柄（与 `windows::Win32::Foundation::HANDLE` 的内部指针
//! 一一对应），便于上层在不引用 `windows` crate 的代码中存储/传递句柄。

#![cfg(windows)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, WAIT_FAILED};
use windows::Win32::System::Diagnostics::Debug::{
    FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, ResumeThread, WaitForSingleObject, CREATE_SUSPENDED,
    INFINITE, PROCESS_INFORMATION, STARTUPINFOW,
};

/// 将 &str 转为 UTF-16 null-terminated `Vec<u16>`。
/// 移植自 C# 中 `Marshal.StringToHGlobalUni` 的用途。
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// 获取最近一次 Win32 错误的可读文本。失败时回退到 `error <code>`。
/// 移植自 C# 中通过 `Marshal.GetLastWin32Error` + `FormatMessage` 的逻辑。
fn last_error_message() -> String {
    unsafe {
        let err = GetLastError();
        let mut buf = [0u16; 512];
        // FormatMessageW 的 lpbuffer 是 PWSTR（可写指针），nsize 是缓冲区容量（含结尾 0）。
        let len = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            None,
            err.0,
            0,
            PWSTR::from_raw(buf.as_mut_ptr()),
            buf.len() as u32,
            None,
        );
        if len == 0 {
            return format!("error {}", err.0);
        }
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

/// 创建带 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 的 Job Object。
///
/// 返回 Job 句柄的 `isize` 表示；调用方应在不再需要时调用 [`close_job`] 释放
/// （通常放在 `finally` 块中）。失败返回 `None`，并已记录 WARN 日志。
///
/// 移植自 C# `CreateKillOnCloseJob`。
pub fn create_kill_on_close_job(log: &dyn Fn(&str)) -> Option<isize> {
    unsafe {
        // windows 0.61: CreateJobObjectW 返回 Result<HANDLE>（内部用 is_invalid 判空）。
        let h_job = match CreateJobObjectW(None, PCWSTR::null()) {
            Ok(h) => h,
            Err(_) => {
                log(&format!(
                    "WARN: CreateJobObject failed, last error: {}",
                    last_error_message()
                ));
                return None;
            }
        };
        let h_job_ptr = h_job.0 as isize;
        if h_job_ptr == 0 {
            log(&format!(
                "WARN: CreateJobObject returned null handle, last error: {}",
                last_error_message()
            ));
            return None;
        }

        // 设置扩展限制信息；只需 KILL_ON_JOB_CLOSE 一项。
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let size = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>();
        if let Err(_) = SetInformationJobObject(
            h_job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            size as u32,
        ) {
            log(&format!(
                "WARN: SetInformationJobObject failed, last error: {}",
                last_error_message()
            ));
            let _ = CloseHandle(h_job);
            return None;
        }

        Some(h_job_ptr)
    }
}

/// 用 `CreateProcessW` 启动子进程并等待其退出。
///
/// 流程（对应 C# `Main` 中的进程启动逻辑）：
/// 1. `CREATE_SUSPENDED` 创建子进程，`bInheritHandles = true`。
/// 2. 在恢复线程前调用 `AssignProcessToJobObject`，把子进程挂到 Job 上。
/// 3. `ResumeThread` 让子进程开始执行。
/// 4. `WaitForSingleObject` 阻塞等待子进程退出。
/// 5. `GetExitCodeProcess` 读取退出码并返回。
/// 6. 关闭 process/thread 句柄。
///
/// 不设 `STARTF_USESTDHANDLES`，子进程自动继承父进程的 stdin/stdout/stderr，
/// 从而 SSH 会话的输入输出直通子进程 shell。
///
/// `cmd_line` 为完整命令行（含程序路径与参数），由调用方负责转义。
/// `job` 为 [`create_kill_on_close_job`] 返回的句柄；若为 0 则跳过 Job 关联
/// （仅作为容错，正常路径下不应为 0）。
/// 返回子进程退出码；若 `CreateProcessW` 失败则返回 1。
pub fn launch_and_wait(cmd_line: &str, job: isize, log: &dyn Fn(&str)) -> u32 {
    // 命令行缓冲必须可变且生命周期覆盖到 CreateProcessW 返回（仅调用期间被读）。
    let mut cmd_wide = to_wide(cmd_line);

    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    // 刻意不设 si.dwFlags 的 STARTF_USESTDHANDLES，以让子进程继承父进程标准句柄。

    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    // windows 0.61: CreateProcessW 返回 Result<()>。
    //   lpapplicationname:  Param<PCWSTR>  -> PCWSTR::null() 表示由命令行解析。
    //   lpcommandline:      Option<PWSTR>  -> 需要可写指针，故传 Some(PWSTR::from_raw(..))。
    //   lpcurrentdirectory: Param<PCWSTR>  -> null 表示继承父进程 cwd。
    let created = unsafe {
        CreateProcessW(
            PCWSTR::null(),
            Some(PWSTR::from_raw(cmd_wide.as_mut_ptr())),
            None,
            None,
            true, // bInheritHandles：继承父进程句柄（含 stdin/stdout/stderr）
            CREATE_SUSPENDED,
            None,
            PCWSTR::null(),
            &si,
            &mut pi,
        )
    };

    if let Err(_) = created {
        log(&format!(
            "ERROR: CreateProcess failed, last error: {}",
            last_error_message()
        ));
        return 1;
    }

    let mut exit_code: u32 = 1;
    unsafe {
        // CREATE_SUSPENDED 启动后、ResumeThread 前关联 Job，确保子进程被 Job 管控。
        if job != 0 {
            let h_job = HANDLE(job as *mut _);
            if let Err(_) = AssignProcessToJobObject(h_job, pi.hProcess) {
                log(&format!(
                    "WARN: AssignProcessToJobObject failed, last error: {}",
                    last_error_message()
                ));
            }
        }

        let _ = ResumeThread(pi.hThread);

        // windows 0.61: WaitForSingleObject 返回 WAIT_EVENT（newtype），与 WAIT_FAILED 比较。
        if WaitForSingleObject(pi.hProcess, INFINITE) == WAIT_FAILED {
            log(&format!(
                "WARN: WaitForSingleObject failed, last error: {}",
                last_error_message()
            ));
        }

        // windows 0.61: GetExitCodeProcess(hprocess, *mut u32) -> Result<()>。
        if let Err(_) = GetExitCodeProcess(pi.hProcess, &mut exit_code) {
            log(&format!(
                "WARN: GetExitCodeProcess failed, last error: {}",
                last_error_message()
            ));
            exit_code = 1;
        }

        let _ = CloseHandle(pi.hProcess);
        let _ = CloseHandle(pi.hThread);
    }

    exit_code
}

/// 关闭 Job Object 句柄。通常在父进程退出的 `finally` 阶段调用，
/// 触发 `KILL_ON_JOB_CLOSE` 以回收所有子进程。`job` 为 0 时无操作。
pub fn close_job(job: isize) {
    if job != 0 {
        unsafe {
            let _ = CloseHandle(HANDLE(job as *mut _));
        }
    }
}
