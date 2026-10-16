//! `ffx-launch.exe [FFX.exe]`: starts the game in the current folder with
//! bottom-up and high-entropy ASLR turned off for it (a process mitigation
//! policy set at creation). Windows then places heaps, stacks and
//! `VirtualAlloc` memory at the same addresses every run, so replay traces
//! and snapshots can be compared byte for byte, pointers included. Image
//! bases still change only at boot. The environment is inherited (the
//! harness settings come through `WSLENV`). Prints the game's process ID and
//! exits without waiting for it.

#[cfg(windows)]
fn main() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, EXTENDED_STARTUPINFO_PRESENT, InitializeProcThreadAttributeList,
        LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY, PROCESS_INFORMATION,
        STARTUPINFOEXW, UpdateProcThreadAttribute,
    };

    // PROCESS_CREATION_MITIGATION_POLICY_BOTTOM_UP_ASLR_ALWAYS_OFF and
    // ..._HIGH_ENTROPY_ASLR_ALWAYS_OFF (winnt.h).
    const BOTTOM_UP_OFF: u64 = 2 << 16;
    const HIGH_ENTROPY_OFF: u64 = 2 << 20;

    let exe = std::env::args().nth(1).unwrap_or_else(|| "FFX.exe".into());
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let mut cmdline = wide(std::ffi::OsStr::new(&format!("\"{exe}\"")));
    let app = wide(std::ffi::OsStr::new(&exe));

    let policy: u64 = BOTTOM_UP_OFF | HIGH_ENTROPY_OFF;
    let mut size = 0usize;
    // SAFETY: the documented two-call pattern: size first, then the list.
    unsafe {
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &raw mut size);
    }
    let mut list = vec![0u8; size];
    let attrs: LPPROC_THREAD_ATTRIBUTE_LIST = list.as_mut_ptr().cast();
    // SAFETY: `list` has the size asked for; `policy` outlives the call that
    // creates the process.
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
        eprintln!("ffx-launch: cannot set the mitigation policy");
        std::process::exit(1);
    }
    // SAFETY: plain structures.
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = u32::try_from(size_of::<STARTUPINFOEXW>()).expect("size");
    si.lpAttributeList = attrs;
    // SAFETY: as above.
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: every pointer is valid for the call.
    let created = unsafe {
        CreateProcessW(
            app.as_ptr(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT,
            std::ptr::null(),
            std::ptr::null(),
            (&raw const si).cast(),
            &raw mut pi,
        )
    };
    if created == 0 {
        eprintln!(
            "ffx-launch: cannot start {exe}: {}",
            std::io::Error::last_os_error()
        );
        std::process::exit(1);
    }
    println!("{}", pi.dwProcessId);
}

#[cfg(not(windows))]
fn main() {
    eprintln!("ffx-launch runs on Windows");
    std::process::exit(1);
}
