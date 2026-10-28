//! `dinput8.dll` proxy. `FFX.exe` imports `DINPUT8.dll!DirectInput8Create`,
//! and the loader looks in the game folder first, so dropping this DLL next
//! to `FFX.exe` gets it loaded before any game code runs. `DllMain` then
//! installs the `ffx-hook` patches, and `DirectInput8Create` forwards to the
//! real `dinput8.dll` in the system directory.
//!
//! It also loads loose-file mods from `mods/<mod>/<archive path>` next to
//! `FFX.exe` (see `ffx_hook::mods`). Activity is logged to `ffx-hook.log`
//! next to `FFX.exe`. Set
//! `FFX_HOOK_DISABLE` to a comma-separated list of hook names to leave
//! those originals in place, or `FFX_HOOK_ONLY` to the names to keep (all
//! other replacements stay original; `@<file>` reads them from a file).

#![cfg(all(windows, target_arch = "x86"))]
// 32-bit only: `usize` is `u32`.
#![allow(clippy::cast_possible_truncation)]

use std::ffi::c_void;

use ffx_hook::log::log;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{HINSTANCE, TRUE};
use windows_sys::Win32::System::LibraryLoader::{
    DisableThreadLibraryCalls, GetModuleHandleW, GetProcAddress, LoadLibraryW,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::core::{BOOL, GUID, HRESULT};

/// `E_FAIL`, returned when the real `dinput8.dll` cannot be loaded.
const E_FAIL: HRESULT = 0x8000_4005_u32.cast_signed();

type DirectInput8CreateFn = unsafe extern "system" fn(
    HINSTANCE,
    u32,
    *const GUID,
    *mut *mut c_void,
    *mut c_void,
) -> HRESULT;

/// # Safety
/// Called by the Windows loader.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    module: HINSTANCE,
    reason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        // SAFETY: `module` is this DLL; the main module handle is its base.
        unsafe {
            DisableThreadLibraryCalls(module);
            install_hooks(GetModuleHandleW(std::ptr::null()) as usize);
        }
    }
    TRUE
}

/// # Safety
/// `base` must be the main module; called before other threads run game code.
unsafe fn install_hooks(base: usize) {
    ffx_hook::log::install_diagnostics();
    log(&format!(
        "ffx-hook {} loaded, FFX.exe at {base:#010x}",
        env!("CARGO_PKG_VERSION")
    ));
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(std::path::Path::to_path_buf))
    {
        ffx_hook::mods::load(&dir);
    }
    // SAFETY: forwarded from the caller.
    unsafe { ffx_hook::harness::install(base) };
    // Iggy call log (FFX_IGGY_LOG), after the harness: its tick-check
    // wrapper is then the import this log calls.
    // SAFETY: forwarded from the caller.
    unsafe { ffx_hook::iggy_log::install(base) };
    // After the harness: its device hook is then the one this one calls.
    // SAFETY: forwarded from the caller.
    unsafe { ffx_hook::gfx::install(base) };
    let disabled = std::env::var("FFX_HOOK_DISABLE").unwrap_or_default();
    let mut disabled: Vec<&str> = disabled
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let hooks = ffx_hook::hooks();
    // `FFX_HOOK_ONLY`: every replacement not named there stays original,
    // so a run can hold the original code plus a few ports (the M4
    // pilot's gate against the original captures); `ffx_hook::hook_only`
    // reads it.
    if let Some(keep) = ffx_hook::hook_only() {
        for h in &hooks {
            if h.original.is_none() && !keep.iter().any(|k| k == h.name) {
                disabled.push(h.name);
            }
        }
    }
    // SAFETY: forwarded from the caller.
    match unsafe { ffx_hook::install(base, &hooks, &disabled) } {
        Ok(results) => {
            // One write: the log may sit on a slow share (WSL), and
            // opening it once per hook made start-up take minutes.
            let lines: Vec<String> = results
                .iter()
                .map(|(name, outcome)| format!("  {name}: {outcome:?}"))
                .collect();
            log(&lines.join("\n"));
            // The harness's loader turn around the original lock when its
            // port was left out (docs/harness.md, "Load pacing").
            // SAFETY: forwarded from the caller.
            unsafe { ffx_hook::loader_turn::install_fallback(base, &hooks, &results) };
        }
        Err(wrong) => log(&format!("  not patching: {wrong}")),
    }
    // The Rust sound engine in place of FMOD (FFX_RUST_AUDIO=1), first: the
    // call log and the capture wrap its stubs.
    // SAFETY: forwarded from the caller.
    unsafe { ffx_hook::rust_audio::install(base) };
    // FMOD call log (FFX_FMOD_LOG): import and vtable slots only.
    // SAFETY: forwarded from the caller.
    unsafe { ffx_hook::fmod_log::install(base) };
    // FMOD's output to a WAV file (FFX_AUDIO_WAV), after the call log's
    // import patches: its wrappers call through them.
    // SAFETY: forwarded from the caller.
    unsafe { ffx_hook::audio_wav::install(base) };
    // Last, so it only arms bytes no hook or patch owns.
    // SAFETY: forwarded from the caller.
    unsafe { ffx_hook::coverage::install(base) };
}

fn real_direct_input8_create() -> Option<DirectInput8CreateFn> {
    static REAL: OnceLock<Option<usize>> = OnceLock::new();
    let addr = *REAL.get_or_init(|| {
        let mut dir = [0u16; 260];
        // SAFETY: the buffer length is passed alongside it.
        let len = unsafe { GetSystemDirectoryW(dir.as_mut_ptr(), dir.len() as u32) } as usize;
        if len == 0 || len >= dir.len() {
            return None;
        }
        let path: Vec<u16> = dir[..len]
            .iter()
            .copied()
            .chain("\\dinput8.dll\0".encode_utf16())
            .collect();
        // SAFETY: `path` is NUL-terminated UTF-16.
        let module = unsafe { LoadLibraryW(path.as_ptr()) };
        if module.is_null() {
            log("cannot load the system dinput8.dll");
            return None;
        }
        // SAFETY: valid module and NUL-terminated name.
        unsafe { GetProcAddress(module, c"DirectInput8Create".as_ptr().cast()) }.map(|f| f as usize)
    });
    // SAFETY: the address came from GetProcAddress for this exact export.
    addr.map(|a| unsafe { std::mem::transmute::<usize, DirectInput8CreateFn>(a) })
}

/// Forwards to the system `dinput8.dll`.
///
/// # Safety
/// Same contract as the real `DirectInput8Create`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DirectInput8Create(
    instance: HINSTANCE,
    version: u32,
    riid: *const GUID,
    out: *mut *mut c_void,
    outer: *mut c_void,
) -> HRESULT {
    let Some(real) = real_direct_input8_create() else {
        return E_FAIL;
    };
    // SAFETY: forwarded unchanged.
    let hr = unsafe { real(instance, version, riid, out, outer) };
    if hr >= 0 && !out.is_null() {
        // SAFETY: on success `*out` is the new IDirectInput8 object.
        unsafe { ffx_hook::harness::wrap_direct_input(*out) };
    }
    hr
}
