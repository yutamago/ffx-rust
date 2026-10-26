//! Continuation round 4 of list `h4-glue-other`, batch 6: the menu
//! message file read.

use ffx_graphic::host::{Conv, Host};

use crate::frames::FIOS_UNIFY;
use crate::h4_d1::FIOS_EXISTS;
use crate::h4_d3::GET_LANG;
use crate::h4_d4::{FIOS_CLOSE, FIOS_OPEN, FIOS_READ, FIOS_SIZE};
use crate::h4_f3::SPRINTF_IAT;
use crate::h4_f5::DBG_PRINTF;

/// `sprintf_s`'s import slot.
pub const SPRINTF_S_IAT: u32 = 0x00b0_c394;
/// The data root `"/ffx_ps2/ffx/master/new_"` (`0x00b5ea8c`).
pub const DATA_ROOT: u32 = 0x00b5_ea8c;
/// The language directory formats by `TOGetFFXLang` (0 to 10); the
/// English one (`"%suspc/"`) for 1, 6, 7, 8 and anything else.
pub const LANG_DIRS: [u32; 11] = [
    0x00b5_eaa8,
    0x00b5_eab0,
    0x00b5_eab8,
    0x00b5_eac0,
    0x00b5_eac8,
    0x00b5_ead0,
    0x00b5_eab0,
    0x00b5_eab0,
    0x00b5_eab0,
    0x00b5_ead8,
    0x00b5_eae0,
];
/// The English language directory format.
pub const LANG_DIR_US: u32 = 0x00b5_eab0;
/// `"%smenu/%s.bin"` (`0x00b5ebf0`).
pub const MENU_FILE_FORMAT: u32 = 0x00b5_ebf0;
/// `"%s"` (`0x00b0e160`).
pub const PERCENT_S: u32 = 0x00b0_e160;
/// The missing-file message (`0x00b41e3c`).
pub const NO_MESSAGE_FILE: u32 = 0x00b4_1e3c;
/// Bytes of [`read_menu_message`]'s frame, from `ebp - 0x1c4`: the
/// unified path (`+0`, 0x100 bytes), the path (`+0x100`, 0x80), the
/// language directory (`+0x180`, 0x40).
pub const MESSAGE_FRAME: u32 = 0x1c0;

/// `TKMallocAndReadMenuMessage(name, out)` (cdecl; `frame` see
/// [`MESSAGE_FRAME`]): the path `<root><lang>/menu/<name>.bin`
/// (`sprintf` twice, the language's directory by `TOGetFFXLang`),
/// unified (`fiosUnifyFilename`) and copied back (`sprintf_s(path, 0x7f,
/// "%s", unified)`). An existing file is read whole into `out` (its first
/// word cleared first; the size rounded up to 16 bytes) and 1 returned;
/// else the path is printed (`dbgPrintf`) and 0 returned.
pub fn read_menu_message(h: &mut impl Host, frame: u32, name: u32, out: u32) -> u32 {
    let unified = frame;
    let path = frame.wrapping_add(0x100);
    let lang = frame.wrapping_add(0x180);
    let l = h.call_sym(GET_LANG, Conv::Cdecl, &[]);
    let dir = LANG_DIRS.get(l as usize).copied().unwrap_or(LANG_DIR_US);
    let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
    let (dir, root) = (h.rebase(dir), h.rebase(DATA_ROOT));
    h.call(sprintf, Conv::Cdecl, &[lang, dir, root]);
    let format = h.rebase(MENU_FILE_FORMAT);
    h.call(sprintf, Conv::Cdecl, &[path, format, lang, name]);
    h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[path, unified, 0x100]);
    let sprintf_s: u32 = h.read(h.rebase(SPRINTF_S_IAT));
    let percent_s = h.rebase(PERCENT_S);
    h.call(sprintf_s, Conv::Cdecl, &[path, 0x7f, percent_s, unified]);
    if h.call_sym(FIOS_EXISTS, Conv::Cdecl, &[path]) == 0 {
        let msg = h.rebase(NO_MESSAGE_FILE);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg, path]);
        return 0;
    }
    let file = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[path, 1]);
    let size = h
        .call_sym(FIOS_SIZE, Conv::Cdecl, &[file])
        .wrapping_add(0xf)
        & !0xf;
    h.write(out, 0u32);
    h.call_sym(FIOS_READ, Conv::Cdecl, &[file, out, size]);
    h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[file]);
    1
}
