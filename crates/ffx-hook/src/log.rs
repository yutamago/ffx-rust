//! Appends lines to `ffx-hook.log` next to the running executable.

use std::io::Write;

/// Writes one line to the log; errors are ignored.
pub fn log(line: &str) {
    let path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("ffx-hook.log")));
    if let Some(path) = path
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}

/// Panics and faults go to the log (a panic in a function the game calls
/// ends the process without a word otherwise): a panic hook, and a
/// vectored exception handler that notes faults (access violations,
/// illegal instructions, stack overflows, fail-fast) with where they
/// happened, then lets the game's own handling go on.
#[cfg(windows)]
pub fn install_diagnostics() {
    use windows_sys::Win32::System::Diagnostics::Debug::AddVectoredExceptionHandler;
    std::panic::set_hook(Box::new(|info| {
        log(&format!("PANIC: {info}"));
    }));
    // SAFETY: a handler that only reads the record and logs.
    unsafe { AddVectoredExceptionHandler(1, Some(fault)) };
}

#[cfg(windows)]
unsafe extern "system" fn fault(
    info: *mut windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS,
) -> i32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEEN: AtomicU32 = AtomicU32::new(0);
    // SAFETY: the system passes a valid record.
    let (code, at) = unsafe {
        let r = &*(*info).ExceptionRecord;
        (r.ExceptionCode.cast_unsigned(), r.ExceptionAddress as usize)
    };
    let serious = matches!(
        code,
        0xc000_0005 | 0xc000_001d | 0xc000_00fd | 0xc000_0409 | 0xc000_0094 | 0xc000_0096
    );
    if serious && SEEN.fetch_add(1, Ordering::Relaxed) < 8 {
        log(&format!(
            "FAULT {code:#010x} at {at:#010x} ({})",
            module_at(at)
        ));
    }
    0 // EXCEPTION_CONTINUE_SEARCH
}

/// The module holding `at`, and the offset in it.
#[cfg(windows)]
fn module_at(at: usize) -> String {
    use windows_sys::Win32::System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleFileNameW, GetModuleHandleExW,
    };
    let mut module = std::ptr::null_mut();
    // SAFETY: looks up the module of an address, no reference taken.
    let found = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            at as *const u16,
            &raw mut module,
        )
    } != 0;
    if !found {
        return "no module".into();
    }
    let mut name = [0u16; 260];
    // SAFETY: the buffer length is passed with it.
    let n = unsafe { GetModuleFileNameW(module, name.as_mut_ptr(), name.len() as u32) } as usize;
    let path = String::from_utf16_lossy(&name[..n.min(name.len())]);
    let file = path.rsplit('\\').next().unwrap_or(&path).to_owned();
    format!("{file}+{:#x}", at - module as usize)
}
