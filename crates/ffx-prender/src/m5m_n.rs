//! M5 stage 1, list `map`, round 5 batch 10: a scene's plain texture load
//! (`rcbgSceneLoadTextureNormal`): `host:<dir>/2d/tex/<map><c>.tm2`
//! loaded, its opaque bounds cut into a transfer with one animation frame,
//! and its CLUT copied into the entry's palette.
//!
//! The path and the outputs live in the original's frame and come in
//! through its `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_e::RCUI_WORK;

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x118;
/// `host:%s/2d/tex/%s%c.tm2` (entries `a`..`z`) and
/// `host:%s/2d/tex/%s_%c.tm2` (from 26: `a`.. again after `_`).
pub const PATH_FORMAT: u32 = 0x00b6_8c30;
pub const PATH_FORMAT_HIGH: u32 = 0x00b6_8c48;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `rcdtTm2New()`, `rcdtTm2Load(t, path)`, `rcdtTm2OpaqueBounds_009234d0(t,
/// &x, &y, &w, &h)`, `rcbgTexGetLog2Size(w, h, &lw, &lh, &tbw)`,
/// `rcbgAnimeSetDataCnt(a, n)`, `rcbgTrxNew()`, `rcbgTrxSetRegsAlloc(t, a..g,
/// n)`, `rcbgTrxGetEntryData(t, n, ..)`, `rcdtTm2CopyRect(t, dst, x, y, w,
/// h)`, `rcbgPalAlloc(p, n)`, `get_ptr32_func(p)`, `rcbgPalGetEntryData(p,
/// n)`, `rcdtTm2Delete(t)` (cdecl).
pub const TM2_NEW: u32 = 0x0092_3b40;
pub const TM2_LOAD: u32 = 0x0092_3800;
pub const TM2_OPAQUE_BOUNDS: u32 = 0x0092_34d0;
pub const TEX_LOG2_SIZE: u32 = 0x0092_0050;
pub const ANIME_SET_DATA_CNT: u32 = 0x0093_08d0;
pub const TRX_NEW: u32 = 0x0092_d580;
pub const TRX_SET_REGS_ALLOC: u32 = 0x0092_d320;
pub const TRX_GET_ENTRY_DATA: u32 = 0x0092_d4f0;
pub const TM2_COPY_RECT: u32 = 0x0092_3ae0;
pub const PAL_ALLOC: u32 = 0x0092_9950;
pub const GET_PTR32: u32 = 0x00a4_4650;
pub const PAL_GET_ENTRY_DATA: u32 = 0x0092_9990;
pub const TM2_DELETE: u32 = 0x0092_3450;

pub mod bridge {
    use super::{
        ANIME_SET_DATA_CNT, GET_PTR32, PAL_ALLOC, PAL_GET_ENTRY_DATA, PATH_FORMAT,
        PATH_FORMAT_HIGH, RCUI_WORK, SPRINTF_SLOT, TEX_LOG2_SIZE, TM2_COPY_RECT, TM2_DELETE,
        TM2_LOAD, TM2_NEW, TM2_OPAQUE_BOUNDS, TRX_GET_ENTRY_DATA, TRX_NEW, TRX_SET_REGS_ALLOC,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `rcbgSceneLoadTextureNormal(scene, i)` -> `rcdtTm2Delete`'s `eax`:
    /// entry `i` (`scene +8`, 0xc0 bytes apart) gets the texture named by
    /// the rcui work's directory (`+0x174`), map (`+0x27c`) and letter
    /// (`'a' + i`, or `'_'` and `'a' + i - 26` from 26); its opaque bounds
    /// (`+0xc` x, `+0x10` y, `+4` w, `+8` h); a transfer (`+0x90` the
    /// animation, one frame, `+0xa0` the transfer: regs 0x3c00, PSMT8 0x13,
    /// w rounded up to 8, h to 2) holding the rectangle; the palette
    /// (`+0xb0`) the image's 256 CLUT words.
    pub fn load_texture_normal(h: &mut impl Host, ebp: u32, scene: u32, i: u32) -> u32 {
        let entry = i.wrapping_mul(0xc0).wrapping_add(h.read(scene + 8));
        h.write(ebp - 0x10c, entry);
        let anime = entry + 0x90;
        h.write(ebp - 0x108, anime);
        let pal = entry + 0xb0;
        h.write(ebp - 0x114, pal);
        let (letter, fmt) = if i.cast_signed() < 0x1a {
            (i.wrapping_add(0x61), PATH_FORMAT)
        } else {
            (i.wrapping_add(0x47), PATH_FORMAT_HIGH)
        };
        let work = RCUI_WORK.get(h);
        let path = ebp - 0x104;
        let fmt = h.rebase(fmt);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(
            sprintf,
            Conv::Cdecl,
            &[path, fmt, work + 0x174, work + 0x27c, letter],
        );
        let tm2 = cdecl(h, TM2_NEW, &[]);
        h.write(ebp - 0x118, tm2);
        cdecl(h, TM2_LOAD, &[tm2, path]);
        cdecl(
            h,
            TM2_OPAQUE_BOUNDS,
            &[tm2, entry + 0xc, entry + 0x10, entry + 4, entry + 8],
        );
        let w = h.read::<u32>(entry + 4).wrapping_add(7) & !7;
        let hh = h.read::<u32>(entry + 8).wrapping_add(1) & !1;
        let log2 = ebp - 0x110;
        cdecl(h, TEX_LOG2_SIZE, &[w, hh, 0, 0, log2]);
        cdecl(h, ANIME_SET_DATA_CNT, &[anime, 1]);
        let trx = cdecl(h, TRX_NEW, &[]);
        h.write(anime + 0x10, trx);
        let tbw: u32 = h.read(log2);
        cdecl(
            h,
            TRX_SET_REGS_ALLOC,
            &[trx, 0x3c00, tbw, 0x13, 0, 0, w, hh, 1],
        );
        let rect: [u32; 4] = [
            h.read(entry + 0xc),
            h.read(entry + 0x10),
            h.read(entry + 4),
            h.read(entry + 8),
        ];
        let trx: u32 = h.read(anime + 0x10);
        let data = cdecl(
            h,
            TRX_GET_ENTRY_DATA,
            &[trx, 0, rect[0], rect[1], rect[2], rect[3]],
        );
        // the original leaves the rectangle on the stack: the copy gets it
        cdecl(
            h,
            TM2_COPY_RECT,
            &[tm2, data, rect[0], rect[1], rect[2], rect[3]],
        );
        cdecl(h, PAL_ALLOC, &[pal, 1]);
        let clut_at: u32 = h.read(tm2 + 0x44);
        let src = cdecl(h, GET_PTR32, &[clut_at]);
        let dst = cdecl(h, PAL_GET_ENTRY_DATA, &[pal, 0]);
        for k in 0..0x100 {
            let v: u32 = h.read(src + 4 * k);
            h.write(dst + 4 * k, v);
        }
        cdecl(h, TM2_DELETE, &[tm2])
    }
}
