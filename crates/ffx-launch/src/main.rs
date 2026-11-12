//! `ffx-launch.exe [--rust | --original] [--no-dialog] [GAME]`.
//!
//! Without `--rust` (the default, as before): starts `FFX.exe` (`GAME`,
//! default `FFX.exe` in the current folder) with bottom-up and
//! high-entropy ASLR turned off for it (a process mitigation policy set at
//! creation). Windows then places heaps, stacks and `VirtualAlloc` memory
//! at the same addresses every run, so replay traces and snapshots can be
//! compared byte for byte, pointers included. Image bases still change
//! only at boot. The environment is inherited (the harness settings come
//! through `WSLENV`). Prints the game's process ID and exits without
//! waiting for it (`tools/run-game.sh --fixed-addresses` reads it).
//!
//! With `--rust`: starts `ffx-rust.exe` from the launcher's folder on the
//! player's game folder (`GAME`, a folder or any file in it; else the
//! current folder, else the launcher's folder: [`plan::game_folder`]),
//! with the documents folder `FFX.exe` uses (`SHGetFolderPathW
//! (CSIDL_PERSONAL)`: settings and saves shared with the original), no
//! time limit, `SteamAppId=359870` when Steam didn't start it, its
//! report in `ffx-rust.log` beside it. Problems show in a message box in
//! the player's words (`--no-dialog`: on stderr only). docs/player.md.

#[cfg_attr(not(windows), allow(dead_code))]
mod plan;

#[cfg(windows)]
fn main() {
    use plan::{Command, Mode};
    let parsed = plan::parse(std::env::args_os().skip(1));
    let args = match parsed {
        Ok(Command::Launch(a)) => a,
        Ok(Command::Help) => {
            println!("{}", plan::USAGE);
            return;
        }
        Err(p) => {
            eprintln!("ffx-launch: {p}");
            std::process::exit(2);
        }
    };
    match args.mode {
        Mode::Original => original(args.game.as_deref()),
        Mode::Rust => {
            if let Err(p) = rust(&args) {
                report(&p, args.no_dialog);
                std::process::exit(1);
            }
        }
    }
}

/// `--rust`: starts `ffx-rust.exe`, prints its process ID.
#[cfg(windows)]
fn rust(args: &plan::Args) -> Result<(), plan::Problem> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let launcher_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
    let game = plan::game_folder(plan::game_arg(args), &cwd, launcher_dir.as_deref())?;
    let launch = plan::rust_launch(
        &game,
        launcher_dir.as_deref().unwrap_or(&cwd),
        documents().as_deref(),
        |k| std::env::var_os(k),
    )?;
    let log = open_log(&launch.log);
    let mut cmd = std::process::Command::new(&launch.program);
    cmd.args(&launch.args)
        .current_dir(&launch.cwd)
        .envs(launch.env.iter().map(|(k, v)| (k, v)))
        .stdin(std::process::Stdio::null())
        // the game's own window only, no console beside it
        .creation_flags(CREATE_NO_WINDOW);
    if let Some((out, err)) = log {
        cmd.stdout(out).stderr(err);
    }
    let child = cmd
        .spawn()
        .map_err(|e| plan::Problem::Start(launch.program.clone(), e.to_string()))?;
    println!("{}", child.id());
    for w in &launch.warnings {
        report(w, args.no_dialog);
    }
    Ok(())
}

/// The report file, the one before kept as `ffx-rust.previous.log`; in the
/// temporary folder when the launcher's folder is read-only.
#[cfg(windows)]
fn open_log(path: &std::path::Path) -> Option<(std::fs::File, std::fs::File)> {
    let previous = path.with_file_name(plan::PREVIOUS_LOG);
    let _ = std::fs::rename(path, previous);
    let f = std::fs::File::create(path)
        .or_else(|_| std::fs::File::create(std::env::temp_dir().join(plan::LOG)))
        .ok()?;
    let g = f.try_clone().ok()?;
    Some((f, g))
}

/// The folder `FFX.exe` asks Windows for: `SHGetFolderPathW(CSIDL_PERSONAL)`.
#[cfg(windows)]
fn documents() -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Shell::{CSIDL_PERSONAL, SHGetFolderPathW};
    let mut buf = [0u16; 260];
    // SAFETY: MAX_PATH wide characters, as the call requires.
    let hr = unsafe {
        SHGetFolderPathW(
            std::ptr::null_mut(),
            i32::try_from(CSIDL_PERSONAL).ok()?,
            std::ptr::null_mut(),
            0,
            buf.as_mut_ptr(),
        )
    };
    let len = buf.iter().position(|&c| c == 0)?;
    (hr == 0 && len > 0).then(|| std::ffi::OsString::from_wide(&buf[..len]).into())
}

/// A problem on stderr and, unless `--no-dialog`, in a message box.
#[cfg(windows)]
fn report(p: &plan::Problem, no_dialog: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_ICONWARNING, MB_OK, MessageBoxW,
    };
    eprintln!("ffx-launch: {p}");
    if no_dialog {
        return;
    }
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let text = wide(&p.to_string());
    let title = wide(p.title());
    let icon = if p.is_warning() {
        MB_ICONWARNING
    } else {
        MB_ICONERROR
    };
    // SAFETY: two NUL-terminated wide strings that outlive the call.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | icon,
        );
    }
}

/// The default: `FFX.exe` with bottom-up ASLR off.
#[cfg(windows)]
fn original(game: Option<&std::path::Path>) {
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

    let exe = game.map_or_else(|| "FFX.exe".into(), |p| p.display().to_string());
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
