//! Windows smoke test for the hook installer, run against the real
//! `FFX.exe` without starting the game.
//!
//! It maps `FFX.exe` as an image without running it or resolving its imports
//! (`DONT_RESOLVE_DLL_REFERENCES`), installs the hooks, then calls the
//! *original* addresses. Those now jump into Rust, which shows that the
//! build check, rebasing, prologue check, patching and calling convention
//! all work on real Windows.
//!
//! Usage: `hook-smoke.exe <path\to\FFX.exe>`

// A linear checklist; 32-bit only, so `usize` casts to `u32` are exact.
#![allow(clippy::too_many_lines, clippy::cast_possible_truncation)]

#[cfg(all(windows, target_arch = "x86"))]
fn main() -> std::process::ExitCode {
    windows::main()
}

#[cfg(not(all(windows, target_arch = "x86")))]
fn main() {
    eprintln!("hook-smoke only runs on 32-bit Windows");
}

#[cfg(all(windows, target_arch = "x86"))]
mod windows {
    use std::os::windows::ffi::OsStrExt;
    use std::process::ExitCode;

    use std::sync::atomic::{AtomicUsize, Ordering};

    use ffx_hook::{Hook, IMAGE_BASE, Outcome};
    use windows_sys::Win32::System::LibraryLoader::{
        DONT_RESOLVE_DLL_REFERENCES, GetModuleHandleW, LoadLibraryExW,
    };

    type LowerFn = unsafe extern "C" fn(*mut u8) -> *mut u8;
    type UncompressFn = unsafe extern "C" fn(*mut u8, *mut u32, *const u8, u32) -> i32;
    type SlashesFn = unsafe extern "C" fn(*mut u8);

    /// Trampoline to `Path_Slashes` (`0x00607c20`) and how often the wrapper ran.
    static SLASHES_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
    static SLASHES_CALLS: AtomicUsize = AtomicUsize::new(0);

    /// Wraps the original: counts the call, then runs it through the trampoline.
    unsafe extern "C" fn slashes_wrapper(s: *mut u8) {
        SLASHES_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the trampoline replays the stolen prologue and jumps back.
        unsafe {
            std::mem::transmute::<usize, SlashesFn>(SLASHES_ORIGINAL.load(Ordering::SeqCst))(s);
        }
    }

    // zlib.compress(b"hello hello hello hello", 9)
    const HELLO: &[u8] = &[
        0x78, 0xda, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x27, 0x01, 0x68, 0x03, 0x08,
        0xb1,
    ];

    fn check(ok: bool, what: &str, failures: &mut u32) {
        println!("[{}] {what}", if ok { "ok" } else { "FAIL" });
        if !ok {
            *failures += 1;
        }
    }

    pub fn main() -> ExitCode {
        let Some(path) = std::env::args_os().nth(1) else {
            eprintln!("usage: hook-smoke <path to FFX.exe>");
            return ExitCode::FAILURE;
        };
        let wide: Vec<u16> = path.encode_wide().chain([0]).collect();
        // SAFETY: NUL-terminated path; the image is mapped but never run.
        let module = unsafe {
            LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                DONT_RESOLVE_DLL_REFERENCES,
            )
        };
        if module.is_null() {
            eprintln!(
                "cannot map {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            );
            return ExitCode::FAILURE;
        }
        let base = module as usize;
        println!("FFX.exe mapped at {base:#010x}");
        let mut failures = 0;

        // SAFETY: our own executable is a mapped PE image.
        let own = unsafe { ffx_hook::check_build(GetModuleHandleW(std::ptr::null()) as usize) };
        check(own.is_err(), "another module is rejected", &mut failures);

        // SAFETY: `base` is FFX.exe and nothing executes it concurrently.
        let results = match unsafe { ffx_hook::install(base, &ffx_hook::hooks(), &[]) } {
            Ok(results) => results,
            Err(wrong) => {
                eprintln!("{wrong}");
                return ExitCode::FAILURE;
            }
        };
        for (name, outcome) in &results {
            check(
                *outcome == Outcome::Installed,
                &format!("{name} installed ({outcome:?})"),
                &mut failures,
            );
        }
        // A second install finds the jmp instead of the prologue and leaves it alone.
        // SAFETY: as above.
        let again = unsafe { ffx_hook::install(base, &ffx_hook::hooks(), &[]) }.unwrap_or_default();
        check(
            again
                .iter()
                .all(|(_, o)| matches!(o, Outcome::PrologueMismatch(found) if found.first() == Some(&0xe9))),
            "re-install is refused",
            &mut failures,
        );

        let at = |address: u32| base + (address - IMAGE_BASE) as usize;
        // SAFETY: the sites now jump to replacements with these signatures.
        let (lower, uncompress) = unsafe {
            (
                std::mem::transmute::<usize, LowerFn>(at(0x0060_7d50)),
                std::mem::transmute::<usize, UncompressFn>(at(0x0040_5e20)),
            )
        };

        let mut text = *b"/FFX_Data/GameData/PS3Data\0";
        // SAFETY: NUL-terminated writable buffer.
        let end = unsafe { lower(text.as_mut_ptr()) };
        check(
            &text == b"/ffx_data/gamedata/ps3data\0",
            "Path_ToLowerAscii lowercases",
            &mut failures,
        );
        check(
            end as usize == text.as_ptr() as usize + 26,
            "Path_ToLowerAscii returns the NUL",
            &mut failures,
        );

        let mut out = [0u8; 64];
        let mut len = out.len() as u32;
        // SAFETY: valid buffers with matching lengths.
        let code = unsafe {
            uncompress(
                out.as_mut_ptr(),
                &raw mut len,
                HELLO.as_ptr(),
                HELLO.len() as u32,
            )
        };
        check(
            code == 0 && &out[..len as usize] == b"hello hello hello hello",
            "zlib_uncompress inflates",
            &mut failures,
        );
        // A trampoline hook: the wrapper must run, and the original body
        // (backslashes to slashes) must still do its work.
        let wrapper = Hook {
            name: "Path_Slashes",
            address: 0x0060_7c20,
            prologue: &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08],
            replacement: slashes_wrapper as *const () as usize,
            original: Some(&SLASHES_ORIGINAL),
        };
        // SAFETY: as above.
        let installed = unsafe { ffx_hook::install(base, &[wrapper], &[]) }.unwrap_or_default();
        check(
            installed.first().map(|r| &r.1) == Some(&Outcome::Installed),
            "trampoline hook installed",
            &mut failures,
        );
        let mut path = *b"a\\b\\c\0";
        // SAFETY: the site jumps to the wrapper, which takes a writable string.
        unsafe { std::mem::transmute::<usize, SlashesFn>(at(0x0060_7c20))(path.as_mut_ptr()) };
        check(
            &path == b"a/b/c\0" && SLASHES_CALLS.load(Ordering::SeqCst) == 1,
            "trampoline runs the wrapper and the original",
            &mut failures,
        );

        let mut small = [0u8; 4];
        let mut len = 4;
        // SAFETY: as above.
        let code = unsafe {
            uncompress(
                small.as_mut_ptr(),
                &raw mut len,
                HELLO.as_ptr(),
                HELLO.len() as u32,
            )
        };
        check(
            code == -5 && len == 4,
            "zlib_uncompress reports Z_BUF_ERROR",
            &mut failures,
        );

        if failures == 0 {
            println!("all checks passed");
            ExitCode::SUCCESS
        } else {
            println!("{failures} check(s) failed");
            ExitCode::FAILURE
        }
    }
}
