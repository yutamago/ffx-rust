//! Loose-file mods.
//!
//! `File_Open` (`0x00607f40`, `thiscall(File*, path, mode, 3 unused)`) is the
//! only way the game opens archive files. It normalizes the path, asks the
//! mounted archives (`FileSys_FindEntry`), and if they lack it opens the path
//! as a loose file with `CreateFileW`. A `File` is two words: the OS handle
//! (`-1` when unused) and the archive entry (`0` when the file is loose); its
//! size, read and close methods use whichever is set.
//!
//! The replacement checks a mod index first: for a read open of a path that
//! `mods/<mod>/<archive path>` provides, it opens that file read-only and
//! leaves the entry empty, so the game's own loose-file code reads it. The
//! handle must be read-only because `File_Close` (`0x00607d80`) calls
//! `SetEndOfFile`, which would otherwise truncate the mod file.

use std::collections::HashMap;
use std::ffi::{CStr, c_char};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_SEQUENTIAL_SCAN, FILE_READ_DATA, FILE_SHARE_READ, OPEN_EXISTING,
};

use crate::log::log;

/// Trampoline to the original `File_Open`.
pub static FILE_OPEN_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

/// `g_FileSys` (`0x00cc9c48`): pointer to the file system object, whose
/// `+0x3c` holds the length of the root prefix `FileSys_FindEntry` skips.
const G_FILE_SYS: u32 = 0x00cc_9c48;

/// Archive path (normalized) → mod file, as a NUL-terminated wide string.
static INDEX: OnceLock<HashMap<String, Vec<u16>>> = OnceLock::new();
/// Archive path (normalized) → mod file size.
static SIZES: OnceLock<HashMap<String, u64>> = OnceLock::new();

/// Set by `FFX_HOOK_LOG_OPENS=1`: log every file the game reads.
static LOG_OPENS: AtomicBool = AtomicBool::new(false);

/// Normalizes a path the way `File_Open` does: `\` → `/`, ASCII lowercase.
fn normalize(path: &[u8]) -> String {
    path.iter()
        .map(|&b| {
            if b == b'\\' {
                '/'
            } else {
                char::from(b.to_ascii_lowercase())
            }
        })
        .collect()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Whether a mod folder is switched on: its `mod.toml` (written by the
/// desktop app; optional) does not say `enabled = false`.
fn enabled(dir: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(dir.join("mod.toml")) else {
        return true;
    };
    !text.lines().any(|l| {
        let l: String = l.chars().filter(|c| !c.is_whitespace()).collect();
        l == "enabled=false"
    })
}

/// Indexes `<game dir>/mods/<mod>/…`. Mods are applied in name order, so a
/// later mod's file replaces an earlier one's. Call once, before game code runs.
pub fn load(game_dir: &Path) {
    let root = game_dir.join("mods");
    let mut index = HashMap::new();
    let mut sizes = HashMap::new();
    let mut mods: Vec<PathBuf> = std::fs::read_dir(&root)
        .map(|r| {
            r.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir() && enabled(p))
                .collect()
        })
        .unwrap_or_default();
    mods.sort();
    for dir in &mods {
        let mut files = Vec::new();
        walk(dir, &mut files);
        for file in &files {
            let Ok(rel) = file.strip_prefix(dir) else {
                continue;
            };
            let key = normalize(rel.to_string_lossy().as_bytes());
            let wide: Vec<u16> = file.as_os_str().encode_wide().chain([0]).collect();
            sizes.insert(key.clone(), std::fs::metadata(file).map_or(0, |m| m.len()));
            if index.insert(key.clone(), wide).is_some() {
                log(&format!(
                    "  mods: {} overrides an earlier mod's {key}",
                    dir.display()
                ));
            }
        }
        log(&format!(
            "  mods: {} ({} files)",
            dir.display(),
            files.len()
        ));
    }
    log(&format!(
        "  mods: {} files indexed from {}",
        index.len(),
        root.display()
    ));
    let _ = INDEX.set(index);
    let _ = SIZES.set(sizes);
    if std::env::var_os("FFX_HOOK_LOG_OPENS").is_some_and(|v| v == "1") {
        LOG_OPENS.store(true, Ordering::Relaxed);
        log("  mods: logging every file the game opens");
    }
}

type FileOpenFn = unsafe extern "thiscall" fn(*mut usize, *const c_char, u32, u32, u32, u32) -> u32;

/// Replaces `File_Open`. Returns 0 on success, like the original.
///
/// # Safety
/// Called by the game with a valid `File` and path.
pub unsafe extern "thiscall" fn file_open(
    file: *mut usize,
    path: *const c_char,
    mode: u32,
    a3: u32,
    a4: u32,
    a5: u32,
) -> u32 {
    if !path.is_null() {
        // SAFETY: NUL-terminated, from the caller.
        crate::world::opened(unsafe { CStr::from_ptr(path) }.to_bytes());
    }
    // Only read opens (mode byte != 0) can be served from a mod.
    // SAFETY: the game passes a valid File and a NUL-terminated path or NULL.
    if mode & 0xff != 0 && !path.is_null() && unsafe { open_mod(file, path) } {
        return 0;
    }
    // SAFETY: the trampoline runs the original prologue and jumps back into
    // it, with the same arguments.
    unsafe {
        let original: FileOpenFn = std::mem::transmute(FILE_OPEN_ORIGINAL.load(Ordering::Acquire));
        original(file, path, mode, a3, a4, a5)
    }
}

unsafe fn open_mod(file: *mut usize, path: *const c_char) -> bool {
    let harness = crate::harness::ENABLED.load(Ordering::Relaxed);
    let logging = LOG_OPENS.load(Ordering::Relaxed);
    let Some(index) = INDEX.get().filter(|i| logging || harness || !i.is_empty()) else {
        return false;
    };
    // SAFETY: `g_FileSys` is a pointer-sized global in the patched image.
    let fs = unsafe { *(crate::rebase(G_FILE_SYS) as *const usize) };
    if fs == 0 {
        return false;
    }
    // SAFETY: the file system object is live once it has been created.
    let root_len = unsafe { *((fs + 0x3c) as *const u32) } as usize;
    // SAFETY: NUL-terminated, from the caller.
    let normalized = normalize(unsafe { CStr::from_ptr(path) }.to_bytes());
    let Some(key) = normalized.get(root_len..) else {
        return false;
    };
    let Some(wide) = index.get(key) else {
        crate::harness::event(format!("open {key}"));
        if logging {
            log(&format!("  open: {key}"));
        }
        return false;
    };
    // SAFETY: `wide` is NUL-terminated.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_READ_DATA,
            FILE_SHARE_READ,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_SEQUENTIAL_SCAN,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        log(&format!(
            "  mods: cannot open the mod file for {key}: {}",
            std::io::Error::last_os_error()
        ));
        return false;
    }
    // SAFETY: a File is [handle, entry]; entry 0 selects the loose-file path.
    unsafe {
        *file = handle as usize;
        *file.add(1) = 0;
    }
    log(&format!("  mods: {key}"));
    crate::harness::event(format!("mod {key}"));
    true
}

/// Trampolines to the original PS2 file size functions.
pub static CD_FILE_SIZE_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
pub static CD_FILE_SIZE_PC_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

/// `Cd_FileName` (`0x00a42830`, `cdecl(number)`): the path of a PS2 file
/// of the current module, `host0:/ffx/master/…`.
const CD_FILE_NAME: u32 = 0x00a4_2830;

/// The size of the mod file standing in for PS2 file `number`, rounded up
/// to 16 bytes like `CdFileSize_PC`. The PS2 file layer sizes buffers and
/// reads from its own tables (`cdrom.mdg` sectors, `sizetbl.bin`), so
/// without this a mod file that grew would be cut at the original size.
fn mod_size(number: i32) -> Option<u32> {
    type NameFn = unsafe extern "C" fn(i32) -> *const c_char;
    let sizes = SIZES.get().filter(|s| !s.is_empty())?;
    // SAFETY: the game's own lookup, called where the game calls the size
    // functions (same module state); it returns a static string or NULL.
    let name = unsafe {
        let f: NameFn = std::mem::transmute(crate::rebase(CD_FILE_NAME));
        f(number)
    };
    if name.is_null() {
        return None;
    }
    // SAFETY: NUL-terminated.
    let path = normalize(unsafe { CStr::from_ptr(name) }.to_bytes());
    let rest = path
        .strip_prefix("host0:")
        .or_else(|| path.strip_prefix("cdrom0:"))?;
    let key = format!("ffx_ps2/{}", rest.trim_start_matches('/'));
    let size = *sizes.get(&key)?;
    let rounded = u32::try_from(size.next_multiple_of(16)).ok()?;
    log(&format!("  mods: size of {key}: {rounded}"));
    Some(rounded)
}

/// Wraps `Cd_FileSize` (`0x0076c320`, `cdecl(number)`), the sector-table
/// size: a mod file reports its own size.
///
/// # Safety
/// Called by the game.
#[must_use]
pub unsafe extern "C" fn cd_file_size(number: i32) -> u32 {
    mod_size(number).unwrap_or_else(|| {
        // SAFETY: the trampoline runs the original.
        unsafe {
            let f: unsafe extern "C" fn(i32) -> u32 =
                std::mem::transmute(CD_FILE_SIZE_ORIGINAL.load(Ordering::Acquire));
            f(number)
        }
    })
}

/// Wraps `CdFileSize_PC` (`0x00a428a0`, `cdecl(number)`), the size table
/// lookup: a mod file reports its own size.
///
/// # Safety
/// Called by the game.
#[must_use]
pub unsafe extern "C" fn cd_file_size_pc(number: i32) -> u32 {
    mod_size(number).unwrap_or_else(|| {
        // SAFETY: the trampoline runs the original.
        unsafe {
            let f: unsafe extern "C" fn(i32) -> u32 =
                std::mem::transmute(CD_FILE_SIZE_PC_ORIGINAL.load(Ordering::Acquire));
            f(number)
        }
    })
}
