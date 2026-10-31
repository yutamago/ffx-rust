//! M5 stage 1, list `map`, round 5 batch 13: an animated texture's
//! position image (`rcbgSceneLoadTextureAnimeRect`,
//! `host:<dir>/2d/tex/<map><c>/<map><c>pos.tm2`): its key-colour bounds,
//! which must sit on even x and width.
//!
//! The name and path live in the original's frame and come in through its
//! `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_e::RCUI_WORK;

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x11c;
/// `%s%c`, `%s_%c`, `host:%s/2d/tex/%s/%spos.tm2`, `Error in %s, line
/// %d: `, the source file's name, `Anime pos file not found.`, `Anime
/// size,pos error!! w:%d x:%d`.
pub const NAME_FORMAT: u32 = 0x00b6_8c64;
pub const NAME_FORMAT_HIGH: u32 = 0x00b6_8c6c;
pub const PATH_FORMAT: u32 = 0x00b6_8cb4;
pub const ERROR_FORMAT: u32 = 0x00b6_7878;
pub const SOURCE_FILE: u32 = 0x00b6_8be8;
pub const NOT_FOUND_TEXT: u32 = 0x00b6_8cd0;
pub const ODD_FORMAT: u32 = 0x00b6_8cec;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `rcdtTm2New()`, `rcdtTm2Load(t, path)` -> 0 when not found,
/// `rcdtTm2KeyBounds_00923660(t, &x, &y, &w, &h)`, `rcdtTm2Delete(t)`,
/// `rcPrint(fmt, ...)` (cdecl).
pub const TM2_NEW: u32 = 0x0092_3b40;
pub const TM2_LOAD: u32 = 0x0092_3800;
pub const TM2_KEY_BOUNDS: u32 = 0x0092_3660;
pub const TM2_DELETE: u32 = 0x0092_3450;
pub const RC_PRINT: u32 = 0x0092_7550;

pub mod bridge {
    use super::{
        ERROR_FORMAT, NAME_FORMAT, NAME_FORMAT_HIGH, NOT_FOUND_TEXT, ODD_FORMAT, PATH_FORMAT,
        RC_PRINT, RCUI_WORK, SOURCE_FILE, SPRINTF_SLOT, TM2_DELETE, TM2_KEY_BOUNDS, TM2_LOAD,
        TM2_NEW,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `rcbgSceneLoadTextureAnimeRect(i, x, y, w, h)` -> the last callee's
    /// `eax`: the position image of entry `i` (letter `'a' + i`, or `'_'`
    /// and `'a' + i - 26` from 26, after the rcui work's map `+0x27c`)
    /// loaded (missing: deleted, an error printed); its key bounds into
    /// `x`, `y`, `w`, `h`, an error printed when `x` or `w` is odd, then
    /// the image deleted.
    #[allow(clippy::many_single_char_names)]
    pub fn load_anime_rect(
        h: &mut impl Host,
        ebp: u32,
        i: u32,
        x: u32,
        y: u32,
        w: u32,
        hh: u32,
    ) -> u32 {
        h.write(ebp - 0x118, y);
        h.write(ebp - 0x11c, hh);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        let (letter, fmt) = if i.cast_signed() < 0x1a {
            (i.wrapping_add(0x61), NAME_FORMAT)
        } else {
            (i.wrapping_add(0x47), NAME_FORMAT_HIGH)
        };
        let name = ebp - 0x14;
        let fmt = h.rebase(fmt);
        let work = RCUI_WORK.get(h);
        h.call(sprintf, Conv::Cdecl, &[name, fmt, work + 0x27c, letter]);
        let path = ebp - 0x114;
        let fmt = h.rebase(PATH_FORMAT);
        let work = RCUI_WORK.get(h);
        h.call(sprintf, Conv::Cdecl, &[path, fmt, work + 0x174, name, name]);
        let tm2 = cdecl(h, TM2_NEW, &[]);
        let mut eax = cdecl(h, TM2_LOAD, &[tm2, path]);
        if eax == 0 {
            if tm2 != 0 {
                cdecl(h, TM2_DELETE, &[tm2]);
            }
            let (fmt, file) = (h.rebase(ERROR_FORMAT), h.rebase(SOURCE_FILE));
            cdecl(h, RC_PRINT, &[fmt, file, 0x180]);
            let text = h.rebase(NOT_FOUND_TEXT);
            return cdecl(h, RC_PRINT, &[text]);
        }
        let (y, hh): (u32, u32) = (h.read(ebp - 0x118), h.read(ebp - 0x11c));
        eax = cdecl(h, TM2_KEY_BOUNDS, &[tm2, x, y, w, hh]);
        if h.read::<u8>(x) & 1 != 0 || h.read::<u8>(w) & 1 != 0 {
            let (fmt, file) = (h.rebase(ERROR_FORMAT), h.rebase(SOURCE_FILE));
            cdecl(h, RC_PRINT, &[fmt, file, 0x189]);
            let (wv, xv): (u32, u32) = (h.read(w), h.read(x));
            let fmt = h.rebase(ODD_FORMAT);
            eax = cdecl(h, RC_PRINT, &[fmt, wv, xv]);
        }
        if tm2 != 0 {
            eax = cdecl(h, TM2_DELETE, &[tm2]);
        }
        eax
    }
}
