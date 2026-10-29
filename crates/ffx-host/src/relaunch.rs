//! i686 Windows: `ffx-rust.exe` runs the game in a child process of itself
//! whose `FFX.exe` range was reserved before it ran (docs/host.md, "The
//! arena").
//!
//! The arena is the process's own address space there, and `FFX.exe`
//! wants `0x00400000..0x0277d000`. `ffx-rust.exe` links at `0x70000000`,
//! but Windows places the main thread's stack (and the first heaps) at the
//! lowest free addresses when the process is created, before any of our
//! code runs: on the first launch the stack's 1 MB reservation sat at
//! `0x00400000`. So when [`crate::arena::reserve_image_range`] fails, the
//! program starts itself again suspended, with bottom-up ASLR off (as
//! `ffx-launch` does for the game), reserves the range in the child with
//! `VirtualAllocEx` while nothing in it has run, and resumes it with
//! `FFX_HOST_IMAGE_RESERVED=1`, so the child adopts the reservation. A child
//! whose range is taken anyway is ended and another started (8 tries). The
//! parent waits for the child and exits with its code; the child is in a
//! job that ends it when the parent goes, so ending the parent's process
//! ends the game.

use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Environment::GetCommandLineW;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows_sys::Win32::System::Memory::{MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx};
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, CreateProcessW, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE,
    InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY, PROCESS_INFORMATION, ResumeThread, STARTUPINFOEXW,
    TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
};

use crate::arena::IMAGE_RANGE;

/// `PROCESS_CREATION_MITIGATION_POLICY_BOTTOM_UP_ASLR_ALWAYS_OFF` and
/// `..._HIGH_ENTROPY_ASLR_ALWAYS_OFF` (winnt.h).
const POLICY: u64 = (2 << 16) | (2 << 20);
const TRIES: u32 = 8;

/// Whether this process is the child (its parent reserved the range).
#[must_use]
pub fn is_child() -> bool {
    std::env::var("FFX_HOST_IMAGE_RESERVED").is_ok_and(|v| v == "1")
}

/// Runs this program again as a child with `FFX.exe`'s range reserved,
/// waits for it and returns its exit code.
///
/// # Errors
/// When no child could be started with the range free.
pub fn run_child() -> Result<u32, String> {
    // SAFETY: set before any thread of ours starts (the child inherits it).
    unsafe { std::env::set_var("FFX_HOST_IMAGE_RESERVED", "1") };
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let app: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
    let job = kill_on_close_job()?;
    for attempt in 1..=TRIES {
        let pi = spawn_suspended(&app)?;
        let (start, end) = IMAGE_RANGE;
        // SAFETY: a reservation in the suspended child's address space.
        let p = unsafe {
            VirtualAllocEx(
                pi.hProcess,
                start as usize as *const _,
                (end - start) as usize,
                MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if p as usize != start as usize {
            eprintln!("ffx-rust: child {attempt}: FFX.exe's range taken; another try");
            // SAFETY: our own suspended child.
            unsafe {
                TerminateProcess(pi.hProcess, 1);
                CloseHandle(pi.hThread);
                CloseHandle(pi.hProcess);
            }
            continue;
        }
        // SAFETY: handles we own.
        unsafe {
            AssignProcessToJobObject(job, pi.hProcess);
            ResumeThread(pi.hThread);
            CloseHandle(pi.hThread);
        }
        eprintln!("ffx-rust: game process {}", pi.dwProcessId);
        let mut code = 1u32;
        // SAFETY: the child's handle, ours until closed below.
        unsafe {
            if WaitForSingleObject(pi.hProcess, INFINITE) == WAIT_OBJECT_0 {
                GetExitCodeProcess(pi.hProcess, &raw mut code);
            }
            CloseHandle(pi.hProcess);
        }
        return Ok(code);
    }
    Err(format!(
        "FFX.exe's range was taken in {TRIES} child processes"
    ))
}

fn kill_on_close_job() -> Result<HANDLE, String> {
    // SAFETY: an unnamed job; the limit structure is a local.
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            (&raw const info).cast(),
            u32::try_from(std::mem::size_of_val(&info)).unwrap_or(0),
        );
        Ok(job)
    }
}

fn spawn_suspended(app: &[u16]) -> Result<PROCESS_INFORMATION, String> {
    let policy = POLICY;
    let mut size = 0usize;
    // SAFETY: the documented two-call pattern: size first, then the list.
    unsafe {
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &raw mut size);
    }
    let mut list = vec![0u8; size];
    let attrs: LPPROC_THREAD_ATTRIBUTE_LIST = list.as_mut_ptr().cast();
    // SAFETY: `list` has the size asked for; `policy` outlives the call
    // that creates the process.
    let ok = unsafe {
        InitializeProcThreadAttributeList(attrs, 1, 0, &raw mut size) != 0
            && UpdateProcThreadAttribute(
                attrs,
                0,
                PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY as usize,
                (&raw const policy).cast(),
                size_of::<u64>(),
                std::ptr::null_mut(),
                std::ptr::null(),
            ) != 0
    };
    if !ok {
        return Err("cannot set the mitigation policy".into());
    }
    // SAFETY: the process's own command line, copied (CreateProcessW may
    // write to it).
    let mut cmdline: Vec<u16> = unsafe {
        let p = GetCommandLineW();
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        std::slice::from_raw_parts(p, n + 1).to_vec()
    };
    // SAFETY: plain structures.
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = u32::try_from(size_of::<STARTUPINFOEXW>()).unwrap_or(0);
    si.lpAttributeList = attrs;
    // SAFETY: as above.
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: every pointer is valid for the call; handles are inherited
    // (the standard streams).
    let created = unsafe {
        CreateProcessW(
            app.as_ptr(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED,
            std::ptr::null(),
            std::ptr::null(),
            (&raw const si).cast(),
            &raw mut pi,
        )
    };
    if created == 0 {
        return Err(format!(
            "cannot start the game process: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(pi)
}
