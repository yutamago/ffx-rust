//! Continuation round 10 of list `h4-glue-other`, batch 3: the movie
//! player manager's video index map.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::h4_k4::{DBG_PRINTF, MEMSET};

/// `LocalizationManager_GetInstance()` (cdecl): the language word.
pub const LOCALIZATION: u32 = 0x006d_b070;
/// `fiosUnifyFilename(path, out, size)` (cdecl).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// `fiosOpen(path, mode)` (cdecl) -> the file, 0 when missing.
pub const FIOS_OPEN: u32 = 0x0067_9730;
/// `fiosRead(file, buffer, size)` (cdecl) -> bytes read.
pub const FIOS_READ: u32 = 0x0067_9800;
/// `fiosClose(file)` (cdecl).
pub const FIOS_CLOSE: u32 = 0x0067_9510;
/// `Phyre_PSerialization_PStreamFile_GetStreamPrefix()` (cdecl).
pub const STREAM_PREFIX: u32 = 0x0060_7d30;
/// `report_rangecheckfailure()` (`0x00949b02`; does not return).
pub const RANGE_CHECK: u32 = 0x0094_9b02;
/// `VideoIndexMap_buyNode_006d5ca0(map; &(id, path))` (thiscall, ret 4)
/// -> a new node holding the id and a `std::string` of the path.
pub const BUY_NODE: u32 = 0x006d_5ca0;
/// `VideoIndexMap_insertNode_006d5ff0(map; out, 0, &value, node)`
/// (thiscall, ret 0x10).
pub const INSERT_NODE: u32 = 0x006d_5ff0;
/// The CRT imports: `sprintf_s`, `strchr`, `strncpy`, `strstr`, `atoi`.
pub const SPRINTF_S: Global<u32> = Global::new(0x00b0_c394);
/// See [`SPRINTF_S`].
pub const STRCHR: Global<u32> = Global::new(0x00b0_c3c0);
/// See [`SPRINTF_S`].
pub const STRNCPY: Global<u32> = Global::new(0x00b0_c368);
/// See [`SPRINTF_S`].
pub const STRSTR: Global<u32> = Global::new(0x00b0_c378);
/// See [`SPRINTF_S`].
pub const ATOI: Global<u32> = Global::new(0x00b0_c40c);
/// The video lists by language (`0x00b49bc8` JP, `0x00b49c00` US,
/// `0x00b49c38` Asia), the unknown-language message (`0x00b49c70`), the
/// missing-list message (`0x00b49c90`).
pub const LIST_JP: u32 = 0x00b4_9bc8;
/// See [`LIST_JP`].
pub const LIST_US: u32 = 0x00b4_9c00;
/// See [`LIST_JP`].
pub const LIST_ASIA: u32 = 0x00b4_9c38;
/// See [`LIST_JP`].
pub const UNKNOWN_LANGUAGE: u32 = 0x00b4_9c70;
/// See [`LIST_JP`].
pub const NO_LIST: u32 = 0x00b4_9c90;
/// `"%s%s"` (`0x00b40eb0`), `":"` (`0x00b485cc`), `","` (`0x00b8f884`),
/// `"%s%s%s%s%s"` (`0x00b49cec`) and the path parts `"/FFX_Data/GameData/"`
/// (`0x00b49cd8`), `"PS3Data"` (`0x00b49cc8`), `"/Video/"` (`0x00b49cd0`).
pub const APPEND_FORMAT: u32 = 0x00b4_0eb0;
/// See [`APPEND_FORMAT`].
pub const COLON: u32 = 0x00b4_85cc;
/// See [`APPEND_FORMAT`].
pub const COMMA: u32 = 0x00b8_f884;
/// See [`APPEND_FORMAT`].
pub const PATH_FORMAT: u32 = 0x00b4_9cec;
/// See [`APPEND_FORMAT`].
pub const GAME_DATA: u32 = 0x00b4_9cd8;
/// See [`APPEND_FORMAT`].
pub const PS3_DATA: u32 = 0x00b4_9cc8;
/// See [`APPEND_FORMAT`].
pub const VIDEO_DIR: u32 = 0x00b4_9cd0;
/// Bytes of [`create_video_index_map`]'s frame, from the original's
/// `ebp - 0x3364`: the insert's result (`+0`, 8 bytes), the (id, path)
/// pair (`+8`), the list's text (`+0x18`, 0x2800), a read chunk
/// (`+0x2818`, 0x400), the video's path (`+0x2c18`, 0x200), the line
/// (`+0x2e18`, 0x400), the list's file name (`+0x3218`, 0x100), the
/// video's name (`+0x3318`, 0x40) and the id's text (`+0x3358`, 8).
pub const VIDEO_FRAME: u32 = 0x3360;

/// The length of the string at `s`.
fn text_len(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `PhyFMVPlayerManager_createVideoIndexMap(this; english)`
/// (`0x006d82f0`, thiscall, ret 4; `frame` see [`VIDEO_FRAME`]): reads the
/// language's `FFX_VideoList.txt` (all of it, 0x3ff bytes per read,
/// appended with `sprintf_s`), then each line `"<id>:<a>,<b>,<c>"` maps the
/// id (`atoi`) to `"<prefix>/FFX_Data/GameData/PS3Data/Video/<name>"` in the
/// map at `+0x6e8`, the name being `<b>`, or `<c>` with `english` set. A
/// line without `':'` or two commas after it ends the map. Lengths past
/// the buffers (0x400 per line or read, 8 for the id, 0x40 for the name)
/// stop the game (`report_rangecheckfailure`). An unknown language leaves
/// the file name as the frame holds it (zeros in the port; the original's
/// stack junk, brief 2026-10-24).
#[allow(clippy::too_many_lines)] // the read, then the lines
pub fn create_video_index_map(h: &mut impl Host, frame: u32, this: u32, english: u32) {
    let out = frame;
    let pair = frame + 8;
    let text = frame + 0x18;
    let chunk = frame + 0x2818;
    let path = frame + 0x2c18;
    let line = frame + 0x2e18;
    let list = frame + 0x3218;
    let name = frame + 0x3318;
    let id = frame + 0x3358;
    let language: u32 = {
        let at = h.call_sym(LOCALIZATION, Conv::Cdecl, &[]);
        h.read(at)
    };
    let source = match language {
        0 => Some(LIST_JP),
        1 | 2 => Some(LIST_US),
        3 => Some(LIST_ASIA),
        _ => None,
    };
    if let Some(s) = source {
        let s = h.rebase(s);
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[s, list, 0x100]);
    } else {
        let message = h.rebase(UNKNOWN_LANGUAGE);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, message]);
    }
    let file = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[list, 1]);
    if file == 0 {
        let message = h.rebase(NO_LIST);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[message]);
        return;
    }
    h.write(text, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[text + 1, 0, 0x27ff]);
    let sprintf_s = SPRINTF_S.get(h);
    loop {
        let n = h.call_sym(FIOS_READ, Conv::Cdecl, &[file, chunk, 0x3ff]);
        if n as i32 <= 0 {
            break;
        }
        if n >= 0x400 {
            h.call_sym(RANGE_CHECK, Conv::Cdecl, &[]);
            return;
        }
        h.write(chunk + n, 0u8);
        let format = h.rebase(APPEND_FORMAT);
        h.call(sprintf_s, Conv::Cdecl, &[text, 0x2800, format, text, chunk]);
    }
    h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[file]);
    let (strchr, strncpy, strstr) = (STRCHR.get(h), STRNCPY.get(h), STRSTR.get(h));
    let (colon, comma) = (h.rebase(COLON), h.rebase(COMMA));
    let mut cursor = text;
    let mut last = false;
    loop {
        let newline = h.call(strchr, Conv::Cdecl, &[cursor, 10]);
        let len = if newline == 0 {
            let n = text_len(h, cursor);
            h.call(strncpy, Conv::Cdecl, &[line, cursor, n]);
            last = true;
            text_len(h, cursor)
        } else {
            let n = newline.wrapping_sub(cursor);
            h.call(strncpy, Conv::Cdecl, &[line, cursor, n]);
            cursor = newline.wrapping_add(1);
            n
        };
        if len >= 0x400 {
            h.call_sym(RANGE_CHECK, Conv::Cdecl, &[]);
            return;
        }
        h.write(line + len, 0u8);
        let at = h.call(strstr, Conv::Cdecl, &[line, colon]);
        if at == 0 {
            return;
        }
        let n = at.wrapping_sub(line);
        h.call(strncpy, Conv::Cdecl, &[id, line, n]);
        if n >= 8 {
            h.call_sym(RANGE_CHECK, Conv::Cdecl, &[]);
            return;
        }
        h.write(id + n, 0u8);
        // the original tests each found address plus one against 0
        let after = at.wrapping_add(1);
        if after == 0 {
            return;
        }
        let first = h.call(strstr, Conv::Cdecl, &[after, comma]).wrapping_add(1);
        if first == 0 {
            return;
        }
        let second = h.call(strstr, Conv::Cdecl, &[first, comma]);
        if second == 0 {
            return;
        }
        let n = if english & 0xff != 0 {
            let start = second.wrapping_add(1);
            let n = text_len(h, start);
            h.call(strncpy, Conv::Cdecl, &[name, start, n]);
            text_len(h, start)
        } else {
            let n = second.wrapping_sub(first);
            h.call(strncpy, Conv::Cdecl, &[name, first, n]);
            n
        };
        if n >= 0x40 {
            h.call_sym(RANGE_CHECK, Conv::Cdecl, &[]);
            return;
        }
        h.write(name + n, 0u8);
        let prefix = h.call_sym(STREAM_PREFIX, Conv::Cdecl, &[]);
        let parts = [GAME_DATA, PS3_DATA, VIDEO_DIR, PATH_FORMAT].map(|s| h.rebase(s));
        h.call(
            sprintf_s,
            Conv::Cdecl,
            &[
                path, 0x200, parts[3], prefix, parts[0], parts[1], parts[2], name,
            ],
        );
        let atoi = ATOI.get(h);
        let number = h.call(atoi, Conv::Cdecl, &[id]);
        h.write(pair, number);
        h.write(pair + 4, path);
        let map = this.wrapping_add(0x6e8);
        let node = h.call_sym(BUY_NODE, Conv::Thiscall(map), &[pair]);
        h.call_sym(
            INSERT_NODE,
            Conv::Thiscall(map),
            &[out, 0, node.wrapping_add(0x10), node],
        );
        if last {
            return;
        }
    }
}
