//! `devsave <n> [lockstep]`: loads developer save `n` (1..110), the way
//! the developers' own loader does, on the game thread between frames
//! (`docs/harness.md`, "Developer saves").
//!
//! The game ships 110 saves the developers used to jump into the story:
//! `ffx_data/gamedata/ps3data/savesforviewer/1` .. `110` in
//! `FFX_Data.vbf`, 0x64f8 bytes each (a PS2-sized save: the first 0x64f8
//! bytes of the PC's 0x6900). Their loader, `TkDebugLoadViewerSave`
//! (`0x008b55e0`, cdecl `(n)`, reached only from the PC tweak panel's
//! "Game Section" entry), skips `n` when it equals the last one loaded
//! (`0x00c5a038`), else has `debugLoadForViewer` (`0x00649040`, cdecl
//! `(n, buffer, size) -> 0 | -1`) read the file into `g_SaveFileBuffer`
//! (`0x01597f70`, 0x6900 bytes), copies the buffer's game block (`+0x40`,
//! 0x68c0 bytes) over `g_SaveData` (`MsGetSaveEventAddress`), notes `n`
//! and jumps with `AtelSetEventJump` (`0x0086fec0`) to the map and
//! entrance at block `+0xba` and `+0xb8` (save 0x97: map 0xf6, scenario
//! 0x7c15). `debugLoadForViewer` opens the file with the CRT's `fopen`
//! (`<stream prefix>/FFX_Data/GameData/PS3Data/savesforviewer/<n>`), which
//! cannot see into the archive, so on PC it always fails and nothing is
//! loaded (facts from the Debug Revival mod's research, `field.md` and
//! `battle.md`; checked in the code here).
//!
//! With the harness on, `debugLoadForViewer` is replaced ([`install`]) by
//! [`load_for_viewer`], which reads the same file through the game's own
//! file layer (`fiosUnifyFilename`, `fiosIsFileExist`, `fiosOpen`,
//! `fiosRead`, `fiosClose`), which finds it in the archive, and keeps the
//! original's results: 0 and the file's bytes at the buffer's start (the
//! rest of the buffer as it was, as `fread` of a shorter file leaves it),
//! -1 when there is no such file. `devsave` then calls the original
//! `TkDebugLoadViewerSave`. Nothing is written to any save file.
//!
//! `AtelSetEventJump` acts only when the current ctrl work holds the
//! control (`*g_ScriptContext` bit 0x80) or the debug flag is on; the
//! command takes the control as `loadsave` does (ctrl work 0, bit 0x80,
//! the context put back after the call). On the title (map `0x17`) it first
//! runs the title script's part around its Load menu, as `loadsave` does
//! ([`crate::loadsave`]), so the title's menu and music leave as for a
//! load.

use crate::rebase;

const SCRIPT_CONTEXT: u32 = 0x0132_6b28;
const CTRL_WORKS: u32 = 0x0132_5ba0;
const SAVE_DATA: u32 = 0x0112_ca90;
/// `TkDebugLoadViewerSave`'s last loaded save.
const VIEWER_SAVE_LAST: u32 = 0x00c5_a038;
const TITLE_MAP: u16 = 0x17;

const DEBUG_LOAD_FOR_VIEWER: u32 = 0x0064_9040;
const TK_DEBUG_LOAD_VIEWER_SAVE: u32 = 0x008b_55e0;
const FIOS_UNIFY_FILENAME: u32 = 0x0067_9820;
const FIOS_IS_FILE_EXIST: u32 = 0x0067_96f0;
const FIOS_OPEN: u32 = 0x0067_9730;
const FIOS_READ: u32 = 0x0067_9800;
const FIOS_CLOSE: u32 = 0x0067_9510;

/// The developers' folder, as `debugLoadForViewer` names it (`0x00b4…`).
const FOLDER: &str = "/FFX_Data/GameData/PS3Data/savesforviewer/";
/// The highest developer save the archive holds.
pub const LAST: i32 = 110;

fn call(address: u32, args: &[u32]) -> u32 {
    // SAFETY: the game thread (a harness job, or the game's own call of
    // the replaced function); every address is a cdecl function of this
    // build taking these arguments.
    unsafe { crate::harness::call(rebase(address), args) }
}

/// Replaces `debugLoadForViewer` (`0x00649040`) while the harness is on;
/// installed whatever `--hook-disable` says, so the original code runs
/// under the same loader.
///
/// # Safety
/// The game's start-up, before the main loop.
pub unsafe fn install(module: usize) {
    let hooks = [crate::Hook {
        name: "debugLoadForViewer",
        address: DEBUG_LOAD_FOR_VIEWER,
        // push ebp; mov ebp, esp; sub esp, 0x40c
        prologue: &[0x55, 0x8b, 0xec, 0x81, 0xec, 0x0c, 0x04, 0x00, 0x00],
        replacement: load_for_viewer as *const () as usize,
        original: None,
        steal: 0,
    }];
    // SAFETY: forwarded from the caller.
    let r = unsafe { crate::install(module, &hooks, &[]) };
    crate::log::log(&format!(
        "  harness: developer saves read from the archive {r:?}"
    ));
}

/// `debugLoadForViewer(n, buffer, size)`: 0 with the file's bytes (at
/// most `size`) at `buffer`, -1 when the archive has no save `n`.
extern "C" fn load_for_viewer(n: i32, buffer: u32, size: u32) -> i32 {
    read(n, buffer, size).map_or(-1, |_| 0)
}

/// Reads developer save `n` into `buffer` through the game's file layer;
/// the byte count.
fn read(n: i32, buffer: u32, size: u32) -> Option<u32> {
    let mut name = format!("{FOLDER}{n}").into_bytes();
    name.push(0);
    let mut path = [0u8; 0x100];
    let name_ptr = name.as_ptr() as usize as u32;
    let path_ptr = path.as_mut_ptr() as usize as u32;
    call(FIOS_UNIFY_FILENAME, &[name_ptr, path_ptr, 0x100]);
    // fiosIsFileExist answers in al
    if call(FIOS_IS_FILE_EXIST, &[path_ptr]).to_le_bytes()[0] == 0 {
        return None;
    }
    // mode 1: read (every reader of the game passes 1)
    let file = call(FIOS_OPEN, &[path_ptr, 1]);
    if file == 0 {
        return None;
    }
    let got = call(FIOS_READ, &[file, buffer, size]);
    call(FIOS_CLOSE, &[file]);
    Some(got)
}

fn read_u16(address: u32) -> Result<u16, String> {
    let b = crate::harness::read_memory(rebase(address), 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

/// Loads developer save `n` and starts its map: the reply names the map,
/// the entrance, the scenario (story progress) and the place name index.
///
/// # Errors
/// When `n` is out of range, the archive has no such save, or no event
/// runs yet.
pub fn load(n: i32) -> Result<String, String> {
    if !(1..=LAST).contains(&n) {
        return Err(format!("developer save {n}: the archive holds 1..{LAST}"));
    }
    let context = rebase(SCRIPT_CONTEXT) as *mut u32;
    // SAFETY: the game's current-ctrl-work pointer.
    if unsafe { context.read() } == 0 {
        return Err("no event runs yet".into());
    }
    let title = read_u16(SAVE_DATA)? == TITLE_MAP;
    if title {
        crate::loadsave::leave_title();
    }
    // the loader skips a repeat of the last save (the mod does the same)
    // SAFETY: a word of the game's data.
    unsafe { (rebase(VIEWER_SAVE_LAST) as *mut u32).write(u32::MAX) };
    // SAFETY: as above; ctrl work 0.
    let (old, forced) = unsafe {
        let old = context.read();
        let ctrl0 = rebase(CTRL_WORKS) as u32;
        context.write(ctrl0);
        let flags = ctrl0 as *mut u8;
        let forced = flags.read() & 0x80 == 0;
        flags.write(flags.read() | 0x80);
        (old, forced)
    };
    call(TK_DEBUG_LOAD_VIEWER_SAVE, &[n.cast_unsigned()]);
    // SAFETY: as above.
    unsafe { context.write(old) };
    let last = crate::harness::read_memory(rebase(VIEWER_SAVE_LAST), 4)?;
    if u32::from_le_bytes([last[0], last[1], last[2], last[3]]) != n.cast_unsigned() {
        return Err(format!("developer save {n} could not be read"));
    }
    let state = crate::harness::read_memory(rebase(SAVE_DATA), 0xbee)?;
    let word = |o: usize| u16::from_le_bytes([state[o], state[o + 1]]);
    Ok(format!(
        "developer save {n} map {} entrance {} scenario {} place {} context {old:#x}{}{}",
        word(0),
        state[0xc],
        word(0xbec),
        word(0xe),
        if title { " (from the title)" } else { "" },
        if forced { " (control taken)" } else { "" }
    ))
}
