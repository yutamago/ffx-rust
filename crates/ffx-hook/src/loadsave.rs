//! `loadsave <slot | windows path>`: starts the game from a save file
//! without the title menu, by running what the title's Load menu runs once
//! a save is picked (`docs/formats/save.md`), on the game thread between
//! frames:
//!
//! 1. the file goes into the save manager's buffer (`*0x00ce81f4 + 0x350`,
//!    `0x01597f70`, 0x6900 bytes) as the load machine's read step does
//!    (`0x006f0070`), and `SaveDataCheckCrc` (`0x00647d70`) checks it;
//! 2. the manager is told a load succeeded, as the machine's success step
//!    (`0x006f0020`) and `HandleSaveDataScreen` (`0x0081ff80`, states 2 to
//!    4) do: `SaveDataSaveLoadSucceed(3)` (the save's music queued at
//!    `0x00c526d0`), the Al Bhed check (`0x00647e40`),
//!    `SaveDataResetToIdle`, `AtelClearForcePlaceId`, and
//!    `TkMenuJumpToLoadedScene` (`0x008b4ec0`), which copies the save's
//!    `0x68c0` game bytes into `g_SaveData` (`0x0112ca90`);
//! 3. the title script's part (`test20.ebp`, the Load choice): before the
//!    menu `0x00aa(1)` (ctrl work 0 `+0x210` bit 0) and `AtelSetEventMode(2)`;
//!    after it `Native_MusicFadeOut(60)` (starts the queued music),
//!    `Native_FadeOut(1)`, `0x01c2`, map fade frames and type 0 and
//!    `Native_MapJump(-1, 0)`: back to the map and entrance the save holds
//!    (`TkSetEventJump`, `0x0088ea60`), which the next frame's scene step
//!    (`0x0088d3d0`) enters.
//!
//! A slot is the file `ffx_<slot, 3 digits>` in the game's save folder,
//! which `SetUpDefaultSaveFolder` (`0x006f0330`) fills in (with the harness
//! sandbox, `FFX_HARNESS_HOME`, that is the sandbox's copy). The file is
//! only read.

use crate::rebase;

const SAVE_DATA_MANAGER: u32 = 0x00ce_81f4;
const SAVE_FOLDER: u32 = 0x00ce_6ee0;
const SAVE_FOLDER_SET: u32 = 0x00ce_72e0;
const SAVE_FILE_PREFIX: u32 = 0x00c3_577c;
const SAVE_SIZE: usize = 0x6900;
const SCRIPT_CONTEXT: u32 = 0x0132_6b28;
const CTRL_WORKS: u32 = 0x0132_5ba0;

const SET_UP_DEFAULT_SAVE_FOLDER: u32 = 0x006f_0330;
const SAVE_DATA_CHECK_CRC: u32 = 0x0064_7d70;
const SAVE_DATA_SAVE_LOAD_SUCCEED: u32 = 0x0064_8540;
const SAVE_DATA_SET_DIALOG_STATE: u32 = 0x0064_8620;
const SAVE_DATA_IS_ALBHED_LOADED: u32 = 0x0064_8020;
const SAVE_DATA_MARK_ALBHED: u32 = 0x0064_7e40;
const SAVE_DATA_RESET_TO_IDLE: u32 = 0x0064_8530;
const SAVE_DATA_SET_SCREEN_STATE: u32 = 0x0064_86e0;
const ATEL_CLEAR_FORCE_PLACE_ID: u32 = 0x0086_38b0;
const TK_MENU_JUMP_TO_LOADED_SCENE: u32 = 0x008b_4ec0;
const SET_CTRL0_FLAG_210: u32 = 0x0086_f3d0;
const ATEL_SET_EVENT_MODE: u32 = 0x0087_3e40;
const NATIVE_MUSIC_FADE_OUT: u32 = 0x0085_e290;
const NATIVE_FADE_OUT: u32 = 0x00a7_7da0;
const NATIVE_01C2: u32 = 0x0085_a570;
const NATIVE_SET_MAP_FADE_FRAMES: u32 = 0x0085_96b0;
const NATIVE_SET_MAP_FADE_TYPE: u32 = 0x0085_9590;
const NATIVE_MAP_JUMP: u32 = 0x0085_8460;

fn call(address: u32, args: &[u32]) -> u32 {
    // SAFETY: the game thread, between frames (a harness job); every
    // address is a cdecl function of this build taking these arguments.
    unsafe { crate::harness::call(rebase(address), args) }
}

fn read_u32(address: usize) -> Result<u32, String> {
    let b = crate::harness::read_memory(address, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// A NUL-terminated UTF-16 string in the game's memory.
fn read_wide(address: usize) -> Result<String, String> {
    let mut units = Vec::new();
    for i in 0..1024 {
        let b = crate::harness::read_memory(address + 2 * i, 2)?;
        let u = u16::from_le_bytes([b[0], b[1]]);
        if u == 0 {
            return Ok(String::from_utf16_lossy(&units));
        }
        units.push(u);
    }
    Err(format!("no string end at {address:#x}"))
}

/// Calls a script native (`(worker, unused, stack)`) with integer
/// arguments in push order, through a stack laid out as
/// `AtelPopStackInteger` (`0x0086df00`) reads it: the count, the values
/// from `+4`, a type byte per value from `+0x54` (1 = integer).
fn native(address: u32, args: &[i32]) -> u32 {
    let mut stack = [0u32; 0x20];
    let n = args.len().min(16);
    stack[0] = n as u32;
    for (i, &a) in args.iter().take(n).enumerate() {
        stack[1 + i] = a.cast_unsigned();
    }
    let bytes: &mut [u8] = stack_bytes(&mut stack);
    for t in &mut bytes[0x54..0x54 + n] {
        *t = 1;
    }
    let ptr = stack.as_mut_ptr() as usize as u32;
    call(address, &[0, 0, ptr])
}

fn stack_bytes(words: &mut [u32; 0x20]) -> &mut [u8] {
    // SAFETY: u32s are plain bytes; the slice covers the array exactly.
    unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr().cast::<u8>(), 0x80) }
}

/// The save file a `loadsave` argument names: a slot number in the game's
/// save folder, or a path.
fn resolve(spec: &str) -> Result<String, String> {
    let Ok(slot) = spec.parse::<u32>() else {
        return Ok(spec.to_owned());
    };
    if slot >= 200 {
        return Err(format!("slot {slot}: the game lists 0..199"));
    }
    call(SET_UP_DEFAULT_SAVE_FOLDER, &[]);
    if read_u32(rebase(SAVE_FOLDER_SET))? == 0 {
        return Err("the save folder could not be set up".into());
    }
    let folder = read_wide(rebase(SAVE_FOLDER))?;
    let prefix = read_wide(read_u32(rebase(SAVE_FILE_PREFIX))? as usize)?;
    Ok(format!("{folder}{prefix}{slot:03}"))
}

/// The title script's part around its Load menu (`test20.ebp`): before
/// the menu `0x00aa(1)` and `AtelSetEventMode(2)`, after it
/// `Native_MusicFadeOut(60)`, `Native_FadeOut(1)`, `0x01c2` and the map
/// fade's frames and type 0. The jump itself is the caller's (`loadsave`:
/// `Native_MapJump(-1, 0)`; `devsave`: the developers' loader).
pub(crate) fn leave_title() {
    call(SET_CTRL0_FLAG_210, &[1]);
    call(ATEL_SET_EVENT_MODE, &[2]);
    native(NATIVE_MUSIC_FADE_OUT, &[60]);
    native(NATIVE_FADE_OUT, &[1]);
    native(NATIVE_01C2, &[]);
    native(NATIVE_SET_MAP_FADE_FRAMES, &[0]);
    native(NATIVE_SET_MAP_FADE_TYPE, &[0]);
}

/// Loads the save `spec` names and starts its map; the reply names the
/// file, the map and the entrance.
///
/// # Errors
/// When the file cannot be read, has the wrong size or fails the CRC.
pub fn load(spec: &str) -> Result<String, String> {
    let path = resolve(spec)?;
    let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
    let manager = read_u32(rebase(SAVE_DATA_MANAGER))? as usize;
    if manager == 0 {
        return Err("no save data manager yet".into());
    }
    let buffer = read_u32(manager + 0x350)? as usize;
    let size = read_u32(manager + 0x354)? as usize;
    if size != SAVE_SIZE || bytes.len() != size {
        return Err(format!(
            "{path}: {} bytes, the game reads {size:#x}",
            bytes.len()
        ));
    }
    crate::harness::read_memory(buffer, size)?;
    // SAFETY: the manager's buffer, just read; the game thread is here.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, size);
    }
    // SaveDataManager_doLoad: albhed result -1, state 3 (loading).
    // SAFETY: fields of the live manager.
    unsafe {
        ((manager + 0xc) as *mut u32).write(u32::MAX);
        ((manager + 8) as *mut u32).write(3);
    }
    if call(SAVE_DATA_CHECK_CRC, &[]) != 1 {
        // The machine's CRC step clears the buffer and reports the error.
        // SAFETY: as above.
        unsafe {
            std::ptr::write_bytes(buffer as *mut u8, 0, size);
            ((manager + 8) as *mut u32).write(0);
        }
        return Err(format!("{path}: CRC mismatch"));
    }
    // The machine's success step (0x006f0020).
    call(SAVE_DATA_SAVE_LOAD_SUCCEED, &[3]);
    call(SAVE_DATA_SET_DIALOG_STATE, &[0]);
    // HandleSaveDataScreen, screen states 2, 3 and 4.
    if call(SAVE_DATA_IS_ALBHED_LOADED, &[]) == u32::MAX {
        call(SAVE_DATA_MARK_ALBHED, &[]);
    }
    call(SAVE_DATA_RESET_TO_IDLE, &[]);
    call(ATEL_CLEAR_FORCE_PLACE_ID, &[]);
    call(TK_MENU_JUMP_TO_LOADED_SCENE, &[]);
    call(SAVE_DATA_SET_SCREEN_STATE, &[0]);
    leave_title();
    // The jump needs the running event to hold control (ctrl byte 0 bit 7,
    // `AtelSetEventJump2`); the title script does, from ctrl work 0.
    let context = rebase(SCRIPT_CONTEXT) as *mut u32;
    // SAFETY: the game's current-ctrl-work pointer and ctrl work 0.
    let (old, forced) = unsafe {
        let old = context.read();
        let ctrl0 = rebase(CTRL_WORKS) as u32;
        context.write(ctrl0);
        let flags = ctrl0 as *mut u8;
        let forced = flags.read() & 0x80 == 0;
        flags.write(flags.read() | 0x80);
        (old, forced)
    };
    native(NATIVE_MAP_JUMP, &[-1, 0]);
    // SAFETY: as above.
    unsafe { context.write(old) };
    let state = crate::harness::read_memory(rebase(0x0112_ca90), 0xd)?;
    Ok(format!(
        "{path} map {} entrance {} context {old:#x}{}",
        u16::from_le_bytes([state[0], state[1]]),
        state[0xc],
        if forced { " (control taken)" } else { "" }
    ))
}
