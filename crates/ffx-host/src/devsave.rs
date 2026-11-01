//! The harness's `devsave <n> [lockstep]` in `ffx-rust`: loads developer
//! save `n` (1..110) the way the developers' own loader does, at the
//! clock's frame boundary, as the hook's `crate::devsave` does inside
//! `FFX.exe` (docs/harness.md, "Developer saves").
//!
//! The saves are `ffx_data/gamedata/ps3data/savesforviewer/1` .. `110` in
//! `FFX_Data.vbf`. Their loader, `TkDebugLoadViewerSave` (`0x008b55e0`,
//! cdecl `(n)`), has `debugLoadForViewer` (`0x00649040`, cdecl
//! `(n, buffer, size) -> 0 | -1`) read the file with the CRT's `fopen`,
//! which cannot see into the archive. With the harness on, [`register`]
//! puts [`load_for_viewer`] in the table under that address: it reads the
//! same name through the game's own file layer (`fiosUnifyFilename`,
//! `fiosIsFileExist`, `fiosOpen(path, 1)`, `fiosRead`, `fiosClose`), with
//! the original's results (0 and the file's bytes at the buffer's start,
//! the rest of the buffer as it was; -1 when there is no such file).
//! Without the harness it runs the original. [`load`] then calls the
//! original `TkDebugLoadViewerSave` (Unicorn, or its port when one exists),
//! with the control taken and, on the title, the title script's part around
//! its Load menu run first, as `loadsave` does.

use ffx_bridge::Mem;

use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

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

/// The developers' folder, as `debugLoadForViewer` names it.
const FOLDER: &str = "/FFX_Data/GameData/PS3Data/savesforviewer/";
/// The highest developer save the archive holds.
pub const LAST: i32 = 110;
/// `fiosUnifyFilename`'s output size, as the hook passes it.
const PATH_SIZE: u32 = 0x100;

/// Puts the archive reader under `debugLoadForViewer` (`0x00649040`).
pub fn register(t: &mut Table) {
    // push ebp; mov ebp, esp; sub esp, 0x40c
    t.wrap(
        DEBUG_LOAD_FOR_VIEWER,
        Entry {
            name: "debugLoadForViewer",
            conv: Conv::Cdecl,
            words: 3,
            port: Port::Exec(load_for_viewer),
        },
        &[0x55, 0x8b, 0xec, 0x81, 0xec, 0x0c, 0x04, 0x00, 0x00],
    );
}

/// `debugLoadForViewer(n, buffer, size)`: with the harness, 0 with the
/// file's bytes (at most `size`) at `buffer`, -1 when the archive has no
/// save `n`; without it, the original.
fn load_for_viewer(e: &mut Exec, args: &Args) -> Ret {
    if !crate::harness::enabled() {
        return e
            .call_original(DEBUG_LOAD_FOR_VIEWER, Conv::Cdecl, args)
            .unwrap_or(Ret::eax(u32::MAX));
    }
    let a = |i| args.stack.get(i).copied().unwrap_or(0);
    let (n, buffer, size) = (a(0).cast_signed(), a(1), a(2));
    Ret::eax(read(e, n, buffer, size).map_or(u32::MAX, |_| 0))
}

/// Reads developer save `n` into `buffer` through the game's file layer;
/// the byte count.
fn read(e: &mut Exec, n: i32, buffer: u32, size: u32) -> Option<u32> {
    let mut name = format!("{FOLDER}{n}").into_bytes();
    name.push(0);
    #[allow(clippy::cast_possible_truncation)] // a short name
    let block = e.alloc(name.len() as u32 + PATH_SIZE);
    if block == 0 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)] // as above
    let path = block + name.len() as u32;
    {
        let mut mem = e.mem();
        mem.write_bytes(block, &name);
        mem.write_bytes(path, &[0; PATH_SIZE as usize]);
    }
    let r = read_named(e, block, path, buffer, size);
    e.release(block);
    r
}

fn read_named(e: &mut Exec, name: u32, path: u32, buffer: u32, size: u32) -> Option<u32> {
    e.call_cdecl(FIOS_UNIFY_FILENAME, &[name, path, PATH_SIZE])
        .ok()?;
    // fiosIsFileExist answers in al
    if e.call_cdecl(FIOS_IS_FILE_EXIST, &[path])
        .ok()?
        .to_le_bytes()[0]
        == 0
    {
        return None;
    }
    // mode 1: read (every reader of the game passes 1)
    let file = e.call_cdecl(FIOS_OPEN, &[path, 1]).ok()?;
    if file == 0 {
        return None;
    }
    let got = e.call_cdecl(FIOS_READ, &[file, buffer, size]).ok();
    let _ = e.call_cdecl(FIOS_CLOSE, &[file]);
    got
}

/// Loads developer save `n` and starts its map: the reply names the map,
/// the entrance, the scenario (story progress) and the place name index,
/// as the hook's.
///
/// # Errors
/// When `n` is out of range, the archive has no such save, no event runs
/// yet, or a call fails.
pub fn load(e: &mut Exec, n: i32) -> Result<String, String> {
    if !(1..=LAST).contains(&n) {
        return Err(format!("developer save {n}: the archive holds 1..{LAST}"));
    }
    if e.mem().read::<u32>(SCRIPT_CONTEXT) == 0 {
        return Err("no event runs yet".into());
    }
    let title = e.mem().read::<u16>(SAVE_DATA) == TITLE_MAP;
    if title {
        crate::harness::leave_title(e)?;
    }
    // the loader skips a repeat of the last save (the mod does the same);
    // it acts only when the current ctrl work holds the control: take it
    // as loadsave does (ctrl work 0, bit 0x80, the context put back)
    let (old, forced) = {
        let mut mem = e.mem();
        mem.write(VIEWER_SAVE_LAST, u32::MAX);
        let old: u32 = mem.read(SCRIPT_CONTEXT);
        mem.write(SCRIPT_CONTEXT, CTRL_WORKS);
        let flags: u8 = mem.read(CTRL_WORKS);
        mem.write(CTRL_WORKS, flags | 0x80);
        (old, flags & 0x80 == 0)
    };
    let r = e.call_cdecl(TK_DEBUG_LOAD_VIEWER_SAVE, &[n.cast_unsigned()]);
    e.mem().write(SCRIPT_CONTEXT, old);
    r.map_err(|err| err.to_string())?;
    let mem = e.mem();
    if mem.read::<u32>(VIEWER_SAVE_LAST) != n.cast_unsigned() {
        return Err(format!("developer save {n} could not be read"));
    }
    Ok(format!(
        "developer save {n} map {} entrance {} scenario {} place {} context {old:#x}{}{}",
        mem.read::<u16>(SAVE_DATA),
        mem.read::<u8>(SAVE_DATA + 0xc),
        mem.read::<u16>(SAVE_DATA + 0xbec),
        mem.read::<u16>(SAVE_DATA + 0xe),
        if title { " (from the title)" } else { "" },
        if forced { " (control taken)" } else { "" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reader_wraps_debug_load_for_viewer() {
        let mut t = Table::new();
        register(&mut t);
        let entry = t.get(DEBUG_LOAD_FOR_VIEWER).expect("an entry");
        assert_eq!(entry.name, "debugLoadForViewer");
        assert_eq!((entry.conv, entry.words), (Conv::Cdecl, 3));
        // the original stays callable (call_original) without the harness
        assert!(t.wrapped().any(|(a, _)| a == DEBUG_LOAD_FOR_VIEWER));
    }

    #[test]
    fn ffx_rust_has_it_in_its_table() {
        let t = crate::ports::table();
        assert_eq!(
            t.get(DEBUG_LOAD_FOR_VIEWER).map(|e| e.name),
            Some("debugLoadForViewer")
        );
    }
}
