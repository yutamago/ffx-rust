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
//! those originals in place.

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
    let disabled = std::env::var("FFX_HOOK_DISABLE").unwrap_or_default();
    let disabled: Vec<&str> = disabled
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    // SAFETY: forwarded from the caller.
    match unsafe { ffx_hook::install(base, &ffx_hook::hooks(), &disabled) } {
        Ok(results) => {
            for (name, outcome) in results {
                log(&format!("  {name}: {outcome:?}"));
            }
        }
        Err(wrong) => log(&format!("  not patching: {wrong}")),
    }
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
    match real_direct_input8_create() {
        // SAFETY: forwarded unchanged.
        Some(real) => unsafe { real(instance, version, riid, out, outer) },
        None => E_FAIL,
    }
}
