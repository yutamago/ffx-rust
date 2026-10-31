//! M5 stage 1, list `map`, round 5 batch 14: a scene's animated texture
//! load (`rcbgSceneLoadTextureAnime`): the position image sets the
//! entry's rectangle, then each `<map><c>/<map><c><nnn>.tm2` frame goes
//! into the transfer (its whole image when it has the rectangle's size)
//! with its CLUT, and the animation records frame `i` (shown).
//!
//! The name, path and spills live in the original's frame and come in
//! through its `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_e::RCUI_WORK;

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x138;
/// `%s%c`, `%s_%c`, `%s/2d/tex/%s`, `host:%s/2d/tex/%s/%s%03d.tm2`,
/// `Error in %s, line %d: `, the source file's name, `File not found.`.
pub const NAME_FORMAT: u32 = 0x00b6_8c64;
pub const NAME_FORMAT_HIGH: u32 = 0x00b6_8c6c;
pub const DIR_FORMAT: u32 = 0x00b6_8c74;
pub const FRAME_FORMAT: u32 = 0x00b6_8c84;
pub const ERROR_FORMAT: u32 = 0x00b6_7878;
pub const SOURCE_FILE: u32 = 0x00b6_8be8;
pub const NOT_FOUND_TEXT: u32 = 0x00b6_8ca4;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `rcbgSceneLoadTextureAnimeRect(i, &x, &y, &w, &h)`,
/// `rcbgCountNumberedFiles_0093acb0(dir, name, digits)`,
/// `rcbgTexGetLog2Size(w, h, &lw, &lh, &tbw)`, `rcbgAnimeSetDataCnt(a, n)`,
/// `rcbgTrxNew()`, `rcbgTrxSetRegsAlloc(t, a..g, n)`, `rcbgPalAlloc(p,
/// n)`, `rcdtTm2New()`, `rcdtTm2Load(t, path)`, `get_ptr32_func(p)`,
/// `rcbgPalGetEntryData(p, n)`, `rcbgTrxGetEntryData(t, n, ..)`,
/// `rcdtTm2CopyRect(t, dst, x, y, w, h)`, `rcdtTm2Delete(t)`,
/// `rcPrint(fmt, ...)` (cdecl).
pub const ANIME_RECT: u32 = 0x0093_bbc0;
pub const COUNT_FILES: u32 = 0x0093_acb0;
pub const TEX_LOG2_SIZE: u32 = 0x0092_0050;
pub const ANIME_SET_DATA_CNT: u32 = 0x0093_08d0;
pub const TRX_NEW: u32 = 0x0092_d580;
pub const TRX_SET_REGS_ALLOC: u32 = 0x0092_d320;
pub const PAL_ALLOC: u32 = 0x0092_9950;
pub const TM2_NEW: u32 = 0x0092_3b40;
pub const TM2_LOAD: u32 = 0x0092_3800;
pub const GET_PTR32: u32 = 0x00a4_4650;
pub const PAL_GET_ENTRY_DATA: u32 = 0x0092_9990;
pub const TRX_GET_ENTRY_DATA: u32 = 0x0092_d4f0;
pub const TM2_COPY_RECT: u32 = 0x0092_3ae0;
pub const TM2_DELETE: u32 = 0x0092_3450;
pub const RC_PRINT: u32 = 0x0092_7550;

pub mod bridge {
    use super::{
        ANIME_RECT, ANIME_SET_DATA_CNT, COUNT_FILES, DIR_FORMAT, ERROR_FORMAT, FRAME_FORMAT,
        GET_PTR32, NAME_FORMAT, NAME_FORMAT_HIGH, NOT_FOUND_TEXT, PAL_ALLOC, PAL_GET_ENTRY_DATA,
        RC_PRINT, RCUI_WORK, SOURCE_FILE, SPRINTF_SLOT, TEX_LOG2_SIZE, TM2_COPY_RECT, TM2_DELETE,
        TM2_LOAD, TM2_NEW, TRX_GET_ENTRY_DATA, TRX_NEW, TRX_SET_REGS_ALLOC,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(f, Conv::Cdecl, args);
    }

    /// `rcbgSceneLoadTextureAnime(scene, i)` -> the last callee's `eax`:
    /// entry `i` (`scene +8`, 0xc0 bytes apart: `+0xc`/`+0x10` x, y, `+4`/
    /// `+8` w, h, `+0x90` the animation, `+0xb0` the palette) gets its
    /// rectangle from the position image, as many frames as
    /// `rcbgCountNumberedFiles` finds (3 digits), a transfer (regs 0x3c00,
    /// PSMT8 0x13, w rounded up to 8, h to 2, one entry per frame) and
    /// palettes; each frame loaded (missing: an error printed) sets the
    /// animation's record `i` (`+0xc`, 4 bytes each: `i`, 1), its CLUT, and
    /// its pixels (from 0, 0 when the image is the rectangle's size, else
    /// from x, y).
    pub fn load_texture_anime(h: &mut impl Host, ebp: u32, scene: u32, i: u32) -> u32 {
        let e = i.wrapping_mul(0xc0).wrapping_add(h.read(scene + 8));
        let anime = e + 0x90;
        let pal = e + 0xb0;
        let (xp, yp, wp, hp) = (e + 0xc, e + 0x10, e + 4, e + 8);
        h.write(ebp - 0x11c, anime);
        h.write(ebp - 0x128, pal);
        h.write(ebp - 0x124, hp);
        h.write(ebp - 0x120, wp);
        h.write(ebp - 0x138, yp);
        h.write(ebp - 0x130, xp);
        cdecl(h, ANIME_RECT, &[i, xp, yp, wp, hp]);
        let (letter, fmt) = if i.cast_signed() < 0x1a {
            (i.wrapping_add(0x61), NAME_FORMAT)
        } else {
            (i.wrapping_add(0x47), NAME_FORMAT_HIGH)
        };
        let name = ebp - 0x14;
        let path = ebp - 0x114;
        let fmt = h.rebase(fmt);
        let work = RCUI_WORK.get(h);
        sprintf(h, &[name, fmt, work + 0x27c, letter]);
        let fmt = h.rebase(DIR_FORMAT);
        let work = RCUI_WORK.get(h);
        sprintf(h, &[path, fmt, work + 0x174, name]);
        let n = cdecl(h, COUNT_FILES, &[path, name, 3]);
        h.write(ebp - 0x118, n);
        let hh = h.read::<u32>(hp).wrapping_add(1) & !1;
        let w = h.read::<u32>(wp).wrapping_add(7) & !7;
        let tbw_at = ebp - 0x12c;
        cdecl(h, TEX_LOG2_SIZE, &[w, hh, 0, 0, tbw_at]);
        cdecl(h, ANIME_SET_DATA_CNT, &[anime, n]);
        let trx = cdecl(h, TRX_NEW, &[]);
        h.write(anime + 0x10, trx);
        let tbw: u32 = h.read(tbw_at);
        cdecl(
            h,
            TRX_SET_REGS_ALLOC,
            &[trx, 0x3c00, tbw, 0x13, 0, 0, w, hh, n],
        );
        cdecl(h, PAL_ALLOC, &[pal, n]);
        let tm2 = cdecl(h, TM2_NEW, &[]);
        let mut eax = tm2;
        h.write(ebp - 0x134, tm2);
        let mut k = 0_u32;
        while k.cast_signed() < h.read::<i32>(ebp - 0x118) {
            let work = RCUI_WORK.get(h);
            let fmt = h.rebase(FRAME_FORMAT);
            sprintf(h, &[path, fmt, work + 0x174, name, name, k]);
            if cdecl(h, TM2_LOAD, &[tm2, path]) == 0 {
                let (fmt, file) = (h.rebase(ERROR_FORMAT), h.rebase(SOURCE_FILE));
                cdecl(h, RC_PRINT, &[fmt, file, 0x14b]);
                let text = h.rebase(NOT_FOUND_TEXT);
                eax = cdecl(h, RC_PRINT, &[text]);
            } else {
                let records: u32 = h.read(anime + 0xc);
                let at = records.wrapping_add(k.wrapping_mul(4));
                h.write(at, k.to_le_bytes()[0]);
                h.write(at + 1, 1_u8);
                let clut_at: u32 = h.read(tm2 + 0x44);
                let src = cdecl(h, GET_PTR32, &[clut_at]);
                let dst = cdecl(h, PAL_GET_ENTRY_DATA, &[pal, k]);
                for j in 0..0x100 {
                    let v: u32 = h.read(src + 4 * j);
                    h.write(dst + 4 * j, v);
                }
                let (mut x, mut y): (u32, u32) = (h.read(xp), h.read(yp));
                if h.read::<u32>(wp) == u32::from(h.read::<u16>(tm2 + 0x24))
                    && h.read::<u32>(hp) == u32::from(h.read::<u16>(tm2 + 0x26))
                {
                    (x, y) = (0, 0);
                }
                let (ww, hv): (u32, u32) = (h.read(wp), h.read(hp));
                let trx: u32 = h.read(anime + 0x10);
                let data = cdecl(h, TRX_GET_ENTRY_DATA, &[trx, k, x, y, ww, hv]);
                // the original leaves the rectangle on the stack: the copy
                // gets it
                eax = cdecl(h, TM2_COPY_RECT, &[tm2, data, x, y, ww, hv]);
            }
            k += 1;
        }
        if tm2 != 0 {
            eax = cdecl(h, TM2_DELETE, &[tm2]);
        }
        eax
    }
}
