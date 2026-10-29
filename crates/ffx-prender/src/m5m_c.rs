//! M5 stage 1, list `map`, batch 3: the debug GUI's TIM2 picture loader
//! (`rcdtTm2`) and its opaque-area bounds, the material-file dispatch, the
//! map ID file load, the rect/circle/line packets of the debug drawing
//! (`rcPacket*`), and wrappers around the guide-map and stage-name tools.
//!
//! None of it ran in a coverage run. Callees stay calls to the originals,
//! cdecl as their `ret` says. Float work follows the x87 at 53 bits
//! (`ffx_bridge::x87`: NaN choice, stores to f32).

/// `get_ptr32_func(p)` (cdecl): the 32-bit pointer of a stored one.
pub const GET_PTR32: u32 = 0x00a4_4650;
/// `set_ptr32(p)` (cdecl): the stored form of a pointer.
pub const SET_PTR32: u32 = 0x00a4_4700;
pub const CHUNK_NEW: u32 = 0x0092_b900;
pub const CHUNK_LOAD_FROM_FILE: u32 = 0x0092_b880;
pub const CHUNK_DELETE: u32 = 0x0092_b7d0;
pub const RC_PRINT: u32 = 0x0092_7550;
pub const TM2_FINI: u32 = 0x0092_3470;
pub const PARSE_NEW: u32 = 0x0092_bfd0;
pub const PARSE_SET: u32 = 0x0092_c0a0;
pub const PARSE_GET_BIN: u32 = 0x0092_ba90;
pub const PARSE_SET_POS: u32 = 0x0092_c0d0;
pub const TM2_GET_PSM: u32 = 0x0092_3280;
pub const RC_ALLOC: u32 = 0x0092_58a0;
pub const TM2_CLUT_ALPHA_TO_PS2: u32 = 0x0092_3240;
pub const RC_FREE_THUNK: u32 = 0x0092_ba50;
/// `rcdtTm2Load_009232a0(pic, path, ps2_alpha)`.
pub const TM2_LOAD: u32 = 0x0092_32a0;

/// `strrchr` and `sprintf` (import slots), `_stricmp` (the thunk).
pub const STRRCHR: u32 = 0x00b0_c3a4;
pub const SPRINTF: u32 = 0x00b0_c3dc;
pub const STRICMP: u32 = 0x0094_9664;
/// `"mat"`, `"ma2"`.
pub const EXT_MAT: u32 = 0x00b6_79c4;
pub const EXT_MA2: u32 = 0x00b6_79c8;
pub const LOAD_MAT_RSD: u32 = 0x0092_3bc0;
pub const LOAD_MAT_RSD2: u32 = 0x0092_3ed0;
/// `g_RcuiWork`; `"host:%s/bin/ffxmap.id"`.
pub const RCUI_WORK: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0193_c0f0);
pub const ID_PATH_FORMAT: u32 = 0x00b6_7a70;
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_LSEEK: u32 = 0x0062_f8e0;
pub const SCE_READ: u32 = 0x0062_fc00;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const ID_FIXUP: u32 = 0x0084_5000;
pub const GET_PLAYER: u32 = 0x0082_d860;
pub const EN_MAP_ID: u32 = 0x0082_aa70;

/// `rcVectorToFixed_00927ac0(out, in)` (cdecl).
pub const TO_FIXED: u32 = 0x0092_7ac0;
/// The rect's half-size scale and the circle's y scale (doubles), the
/// sine table (4096 f32).
pub const HALF_SCALE: u32 = 0x00b9_2220;
pub const CIRCLE_SCALE: u32 = 0x00b6_7698;
pub const SIN_TABLE: u32 = 0x00c4_4bf0;

/// `x` as `fld dword` leaves it: a signalling NaN gets its quiet bit.
#[must_use]
pub fn widen(x: u32) -> f64 {
    ffx_bridge::x87::load(x)
}

pub const FUN_0092C600: u32 = 0x0092_c600;
pub const STAGE_NAME2: u32 = 0x0093_7960;
pub const JUMP_MAP_NO: u32 = 0x0086_c1f0;
pub const FP_SAVE_SUB: u32 = 0x0093_8350;
pub const GUIDE_MAP_CONV: u32 = 0x0093_8590;
pub const RENDER_STATUS: u32 = 0x0090_8f10;
pub const FP_LOAD_TEXT_SUB: u32 = 0x0093_a800;

pub const PARSE_SET_MODE: u32 = 0x0092_c0c0;
pub const PARSE_ADVANCE_LINE: u32 = 0x0092_b9b0;
pub const PARSE_GET_INT: u32 = 0x0092_bc50;
pub const PARSE_NEXT_TOKEN: u32 = 0x0092_ba30;
/// `"..\program\src\rcData\rcdtRsd2.c"`, the not-found message, the
/// assertion's text and format.
pub const RSD_FILE: u32 = 0x00b6_793c;
pub const MAT_NOT_FOUND: u32 = 0x00b6_79cc;
pub const MA2_NOT_FOUND: u32 = 0x00b6_7a3c;
pub const PLY_NOT_FOUND: u32 = 0x00b6_799c;
pub const RSD_NOT_FOUND: u32 = 0x00b6_7978;
/// `"[]="`; the keywords `PLY`, `MAT`, `GRP`, `NTEX`, `TEX`, `VGR`, then
/// the table's end.
pub const RSD_SEPARATORS: u32 = 0x00b6_7998;
pub const RSD_KEYWORDS: [u32; 7] = [
    0x00b6_7960,
    0x00b6_7964,
    0x00b6_7968,
    0x00b6_796c,
    0x00b5_9940,
    0x00b6_7974,
    0,
];
/// `_strlwr` (import slot).
pub const STRLWR: u32 = 0x00b0_c538;
pub const PARSE_ADD_SEPARATORS: u32 = 0x0092_c0f0;
pub const PARSE_IS_END: u32 = 0x0092_bfb0;
pub const PARSE_GET_KEYWORD: u32 = 0x0092_be40;
pub const PARSE_GET_TEXT: u32 = 0x0092_bdd0;
pub const PARSE_TOKEN_LEN_TEXT: u32 = 0x0092_be10;
pub const LOAD_PLY: u32 = 0x0092_4090;
pub const LOAD_MAT: u32 = 0x0092_3b60;
/// `rcdtParseGetFloat(parse)` (cdecl) -> `ST0`.
pub const PARSE_GET_FLOAT: u32 = 0x0092_bbb0;
pub const MEMSET: u32 = 0x0094_964c;
pub const ASSERT_TEXT: u32 = 0x00b6_79f8;
pub const ASSERT_FMT: u32 = 0x00b6_7a14;

/// `"Error in %s, line %d: "`, `"..\\program\\src\\rcData\\rcdtTm2.c"`,
/// `"File not found"`, `"Image Type Error"`.
pub const ERROR_IN: u32 = 0x00b6_7878;
pub const TM2_FILE: u32 = 0x00b6_78f8;
pub const FILE_NOT_FOUND: u32 = 0x00b6_7918;
pub const IMAGE_TYPE_ERROR: u32 = 0x00b6_7928;

/// Which pixels of an 8-bit picture count, by their palette entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opaque {
    /// Alpha (the top byte) non-zero.
    Alpha,
    /// Colour (the low three bytes) zero: the black key.
    Black,
}

/// The bounds of the counted pixels of a `w` x `h` 8-bit picture
/// (`pixels`, row by row) under `counted` (256 flags): `(min x, min y,
/// max x, max y)`, starting from `(w - 1, h - 1, 0, 0)`.
#[must_use]
pub fn bounds(pixels: &[u8], w: i32, h: i32, counted: &[bool; 256]) -> (i32, i32, i32, i32) {
    let (mut x0, mut y0, mut x1, mut y1) = (w - 1, h - 1, 0, 0);
    let mut i = 0;
    for y in 0..h {
        for x in 0..w {
            if counted[usize::from(pixels[i])] {
                if x0 > x {
                    x0 = x;
                }
                if x1 < x {
                    x1 = x;
                }
                if y0 > y {
                    y0 = y;
                }
                if y1 < y {
                    y1 = y;
                }
            }
            i += 1;
        }
    }
    (x0, y0, x1, y1)
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    /// `rcdtTm2OpaqueBounds_009234d0(pic, &x, &y, &w, &h)` /
    /// `rcdtTm2KeyBounds_00923660(...)`: the bounds of the picture's
    /// counted pixels (palette `get_ptr32_func(+0x44)`, pixels
    /// `get_ptr32_func(+0x40)`, size u16 `+0x24` x `+0x26`). The opaque
    /// form widens to an even x and an even width; the key form doesn't.
    /// -> `&h` (the last pointer, left in `eax`).
    pub fn tm2_bounds(h: &mut impl Host, kind: Opaque, pic: u32, out: [u32; 4]) -> u32 {
        let clut_p = word(h, pic + 0x44);
        let clut = cdecl(h, GET_PTR32, &[clut_p]);
        let mut counted = [false; 256];
        for (i, c) in counted.iter_mut().enumerate() {
            let v = word(h, clut.wrapping_add(4 * i as u32));
            *c = match kind {
                Opaque::Alpha => v & 0xff00_0000 != 0,
                Opaque::Black => v.trailing_zeros() >= 24,
            };
        }
        let w = i32::from(h.read::<u16>(pic + 0x24));
        let hh = i32::from(h.read::<u16>(pic + 0x26));
        let img_p = word(h, pic + 0x40);
        let img = cdecl(h, GET_PTR32, &[img_p]);
        let n = (w * hh).max(0).cast_unsigned();
        let mut pixels = vec![0_u8; n as usize];
        h.read_bytes(img, &mut pixels);
        let (mut x0, y0, x1, y1) = bounds(&pixels, w, hh, &counted);
        let mut width = x1 - x0 + 1;
        let height = y1 - y0 + 1;
        if kind == Opaque::Alpha {
            if x0 & 1 != 0 {
                x0 -= 1;
                width += 1;
            }
            if width & 1 != 0 {
                width += 1;
            }
        }
        h.write(out[0], x0);
        h.write(out[1], y0);
        h.write(out[2], width);
        h.write(out[3], height);
        out[3]
    }

    fn error(h: &mut impl Host, line: u32, what: u32) {
        let (e, f, w) = (h.rebase(ERROR_IN), h.rebase(TM2_FILE), h.rebase(what));
        cdecl(h, RC_PRINT, &[e, f, line]);
        cdecl(h, RC_PRINT, &[w]);
    }

    /// `rcdtTm2Load_009232a0(pic, path, ps2_alpha)` -> 1 when loaded: the
    /// file into a new chunk (not found: the chunk deleted, the error
    /// printed, 0); the picture emptied (`rcdtTm2Fini`), its 0x10-byte
    /// file header and 0x30-byte picture header read through a new parse;
    /// `+0x48` = the PSM. Image types 4 (4-bit, `w * h / 2` bytes) and 5
    /// (8-bit) only (else the error, 0, and the chunk and parse kept).
    /// The image (after the header, `+0x1c`'s u16 + 0x10) and the palette
    /// (`+0x18` further, `+0x14` bytes) into `rcAlloc`ed buffers stored
    /// through `set_ptr32` in `+0x40`/`+0x44`; with `ps2_alpha` the palette
    /// converted; the parse freed and the chunk deleted.
    pub fn tm2_load(h: &mut impl Host, pic: u32, path: u32, ps2_alpha: u32) -> u32 {
        let chunk = cdecl(h, CHUNK_NEW, &[]);
        if cdecl(h, CHUNK_LOAD_FROM_FILE, &[chunk, path]) == 0 {
            if chunk != 0 {
                cdecl(h, CHUNK_DELETE, &[chunk]);
            }
            error(h, 0x4d, FILE_NOT_FOUND);
            return 0;
        }
        cdecl(h, TM2_FINI, &[pic]);
        let parse = cdecl(h, PARSE_NEW, &[]);
        cdecl(h, PARSE_SET, &[parse, chunk]);
        cdecl(h, PARSE_GET_BIN, &[parse, pic, 0x10]);
        let head = pic + 0x10;
        cdecl(h, PARSE_GET_BIN, &[parse, head, 0x30]);
        let psm = cdecl(h, TM2_GET_PSM, &[head]);
        h.write(pic + 0x48, psm);
        let kind: u8 = h.read(head + 0x13);
        let w = u32::from(h.read::<u16>(head + 0x14));
        let hh = u32::from(h.read::<u16>(head + 0x16));
        let size = match kind {
            4 => (hh.wrapping_mul(w).cast_signed() / 2).cast_unsigned(),
            5 => hh.wrapping_mul(w),
            _ => {
                error(h, 0x66, IMAGE_TYPE_ERROR);
                return 0;
            }
        };
        let header = u32::from(h.read::<u16>(head + 0xc)) + 0x10;
        cdecl(h, PARSE_SET_POS, &[parse, header]);
        let file = h.rebase(TM2_FILE);
        let image = cdecl(h, RC_ALLOC, &[size, file, 0x6c]);
        let stored = cdecl(h, SET_PTR32, &[image]);
        h.write(pic + 0x40, stored);
        let p = cdecl(h, GET_PTR32, &[stored, size]);
        cdecl(h, PARSE_GET_BIN, &[parse, p, size]);
        let at = word(h, head + 8).wrapping_add(header);
        cdecl(h, PARSE_SET_POS, &[parse, at]);
        let clut_size = word(h, pic + 0x14);
        let clut = cdecl(h, RC_ALLOC, &[clut_size, file, 0x71]);
        let stored = cdecl(h, SET_PTR32, &[clut]);
        h.write(pic + 0x44, stored);
        let clut_size = word(h, pic + 0x14);
        let p = cdecl(h, GET_PTR32, &[stored, clut_size]);
        cdecl(h, PARSE_GET_BIN, &[parse, p, clut_size]);
        if ps2_alpha != 0 {
            let stored = word(h, pic + 0x44);
            let p = cdecl(h, GET_PTR32, &[stored]);
            cdecl(h, TM2_CLUT_ALPHA_TO_PS2, &[p]);
        }
        if parse != 0 {
            cdecl(h, RC_FREE_THUNK, &[parse]);
        }
        if chunk != 0 {
            cdecl(h, CHUNK_DELETE, &[chunk]);
        }
        1
    }

    /// `rcdtRsdLoad_LoadMat_00923b60(model, path)`: by the extension after
    /// the last `'.'` (`strrchr`; none -> 0): `"mat"` ->
    /// `rcdtRsdLoad_LoadMatRsd`, `"ma2"` -> `rcdtRsdLoad_LoadMatRsd2`
    /// (`_stricmp` through `0x00949664`) -> its `eax`; else the last
    /// compare's.
    pub fn load_mat(h: &mut impl Host, model: u32, path: u32) -> u32 {
        let f = word(h, h.rebase(STRRCHR));
        let dot = h.call(f, Conv::Cdecl, &[path, u32::from(b'.')]);
        if dot == 0 {
            return 0;
        }
        let ext = dot.wrapping_add(1);
        let mat = h.rebase(EXT_MAT);
        let r = cdecl(h, STRICMP, &[ext, mat]);
        if r == 0 {
            return cdecl(h, LOAD_MAT_RSD, &[model, path]);
        }
        let ma2 = h.rebase(EXT_MA2);
        let r = cdecl(h, STRICMP, &[ext, ma2]);
        if r != 0 {
            return r;
        }
        cdecl(h, LOAD_MAT_RSD2, &[model, path])
    }

    /// `rcbgIDLoad()` (frame: the path at `ebp - 0x104`) -> the ID file's
    /// contents or 0: `sprintf(path, "host:%s/bin/ffxmap.id",
    /// g_RcuiWork + 0x174)`, `sceOpen(path, 1)`; sized by two `sceLseek`s,
    /// read into `user_malloc(size)`, closed; `Sg_IdFixupAndSetThunk(mem)`,
    /// `Ch_EnMapID(Sg_GetPlayer(), 1)`.
    pub fn id_load(h: &mut impl Host, ebp: u32) -> u32 {
        let path = ebp - 0x104;
        let dir = RCUI_WORK.get(h).wrapping_add(0x174);
        let fmt = h.rebase(ID_PATH_FORMAT);
        let f = word(h, h.rebase(SPRINTF));
        h.call(f, Conv::Cdecl, &[path, fmt, dir]);
        let file = cdecl(h, SCE_OPEN, &[path, 1]);
        if file == 0 {
            return 0;
        }
        let size = cdecl(h, SCE_LSEEK, &[file, 0, 2]);
        cdecl(h, SCE_LSEEK, &[file, 0, 0]);
        let mem = cdecl(h, USER_MALLOC, &[size]);
        cdecl(h, SCE_READ, &[file, mem, size]);
        cdecl(h, SCE_CLOSE, &[file]);
        cdecl(h, ID_FIXUP, &[mem]);
        // the 1 pushed before `Sg_GetPlayer` (which reads no words) is
        // `Ch_EnMapID`'s second word: the original pops both together
        let player = cdecl(h, GET_PLAYER, &[]);
        cdecl(h, EN_MAP_ID, &[player, 1]);
        mem
    }

    fn ftol(h: &mut impl Host, at: u32) -> u32 {
        let x = widen(word(h, at));
        ffx_bridge::x87::ftol(h, x)
    }

    /// The packet's colour: four floats truncated (`ftol2_sse`) into the
    /// first four words at the packet's write position (`pkt + 4`).
    fn colour(h: &mut impl Host, pkt: u32, rgba: u32) -> u32 {
        let at = word(h, pkt + 4);
        for k in 0..4 {
            let v = ftol(h, rgba.wrapping_add(4 * k));
            h.write(at.wrapping_add(4 * k), v);
        }
        at
    }

    /// `rcVectorToFixed_00927ac0(out = ebp - 0x20, in = ebp - 0x10)`.
    fn to_fixed(h: &mut impl Host, ebp: u32) {
        cdecl(h, TO_FIXED, &[ebp - 0x20, ebp - 0x10]);
    }

    fn put_f32(h: &mut impl Host, at: u32, x: f64) {
        h.write(at, x87::store(x));
    }

    /// `rcPacketRect_00927630(pkt, x, y, w, h, rgba)` (frame: the rect at
    /// `ebp - 0x10`, its fixed form at `ebp - 0x20`): the colour, then the
    /// rect `(x - w k, y - h k, x + w k, y + h k)` (`k` the double at
    /// `0x00b92220`; f64, rounded to f32 at each store) converted, its four
    /// corners as (x, y, 0, 0) quads; the position moves by 0x50 -> it.
    pub fn packet_rect(h: &mut impl Host, ebp: u32, pkt: u32, a: [u32; 5]) -> u32 {
        let [x, y, w, hh, rgba] = a;
        let at = colour(h, pkt, rgba);
        let k: f64 = h.read(h.rebase(HALF_SCALE));
        let (x, y, w, hh) = (widen(x), widen(y), widen(w), widen(hh));
        let (dx, dy) = (x87::mul(w, k), x87::mul(hh, k));
        put_f32(h, ebp - 0x10, x87::sub(x, dx));
        put_f32(h, ebp - 0xc, x87::sub(y, dy));
        put_f32(h, ebp - 8, x87::add(dx, x));
        put_f32(h, ebp - 4, x87::add(dy, y));
        to_fixed(h, ebp);
        let f: [u32; 4] = std::array::from_fn(|i| word(h, ebp - 0x20 + 4 * i as u32));
        for (q, (px, py)) in [(f[0], f[1]), (f[2], f[1]), (f[0], f[3]), (f[2], f[3])]
            .into_iter()
            .enumerate()
        {
            let o = at.wrapping_add(0x10 + 0x10 * q as u32);
            h.write(o, px);
            h.write(o.wrapping_add(4), py);
            h.write(o.wrapping_add(8), 0_u32);
            h.write(o.wrapping_add(0xc), 0_u32);
        }
        let end = at.wrapping_add(0x50);
        h.write(pkt + 4, end);
        end
    }

    /// `rcPacketCircle_00927720(pkt, x, y, r, rgba)` (frame as
    /// `rcPacketRect_00927630`): the colour, then 17 points around the
    /// circle (`i * 0x1000`, i = 0..=16, as angles into the sine table at
    /// `0x00c44bf0`, cosine 0x4000 further): `(x + sin r, y + k r cos)`
    /// (`k` the double at `0x00b67698`), each converted into an (x, y, 0,
    /// 0) quad. -> the last conversion's y word. The words at `ebp - 8`
    /// and `ebp - 4` the conversion reads are never written: 0 here (junk
    /// read, brief 2026-10-24); only its x and y outputs are used.
    pub fn packet_circle(h: &mut impl Host, ebp: u32, pkt: u32, a: [u32; 4]) -> u32 {
        let [x, y, r, rgba] = a;
        let mut at = colour(h, pkt, rgba).wrapping_add(0x10);
        let k: f64 = h.read(h.rebase(CIRCLE_SCALE));
        let (x, y, r) = (widen(x), widen(y), widen(r));
        let table = h.rebase(SIN_TABLE);
        let mut last = 0;
        // junk read: [ebp-0x8], [ebp-0x4] never written; 0 (brief 2026-10-24)
        h.write(ebp - 8, 0_u32);
        h.write(ebp - 4, 0_u32);
        let mut angle = 0_i32;
        while angle <= 0x1_0000 {
            let s = (angle >> 4) & 0xfff;
            let c = ((angle + 0x4000) >> 4) & 0xfff;
            let sin = widen(word(h, table + 4 * s.cast_unsigned()));
            let cos = widen(word(h, table + 4 * c.cast_unsigned()));
            put_f32(h, ebp - 0x10, x87::add(x, x87::mul(sin, r)));
            put_f32(h, ebp - 0xc, x87::add(y, x87::mul(x87::mul(r, cos), k)));
            to_fixed(h, ebp);
            let fx = word(h, ebp - 0x20);
            last = word(h, ebp - 0x1c);
            h.write(at, fx);
            h.write(at.wrapping_add(4), last);
            h.write(at.wrapping_add(8), 0_u32);
            h.write(at.wrapping_add(0xc), 0_u32);
            angle += 0x1000;
            at = at.wrapping_add(0x10);
        }
        h.write(pkt + 4, at);
        last
    }

    /// `rcPacketLine_00927810(pkt, x0, y0, x1, y1, rgba)` (frame as
    /// `rcPacketRect_00927630`): the colour, then each end converted into
    /// an (x, y, 0, 0) quad; the position moves by 0x30 -> it. The
    /// conversion's `ebp - 8`/`- 4` inputs are never written: 0 here (junk
    /// read, brief 2026-10-24); only its x and y outputs are used.
    pub fn packet_line(h: &mut impl Host, ebp: u32, pkt: u32, a: [u32; 5]) -> u32 {
        let [x0, y0, x1, y1, rgba] = a;
        let at = colour(h, pkt, rgba);
        // junk read: [ebp-0x8], [ebp-0x4] never written; 0 (brief 2026-10-24)
        h.write(ebp - 8, 0_u32);
        h.write(ebp - 4, 0_u32);
        for (q, (px, py)) in [(x0, y0), (x1, y1)].into_iter().enumerate() {
            let (px, py) = (widen(px), widen(py));
            put_f32(h, ebp - 0x10, px);
            put_f32(h, ebp - 0xc, py);
            to_fixed(h, ebp);
            let o = at.wrapping_add(0x10 + 0x10 * q as u32);
            let fx = word(h, ebp - 0x20);
            let fy = word(h, ebp - 0x1c);
            h.write(o, fx);
            h.write(o.wrapping_add(4), fy);
            h.write(o.wrapping_add(8), 0_u32);
            h.write(o.wrapping_add(0xc), 0_u32);
        }
        let end = at.wrapping_add(0x30);
        h.write(pkt + 4, end);
        end
    }

    /// `Wrap_0092c600_0092c5e0(a, b)` -> `FUN_0092c600(a, b, 1)`.
    pub fn wrap_0092c600(h: &mut impl Host, a: u32, b: u32) -> u32 {
        cdecl(h, FUN_0092C600, &[a, b, 1])
    }

    /// `rcuiGetStageName(a, b)` -> `rcuiGetStageName2(a, b, 0)`.
    pub fn stage_name(h: &mut impl Host, a: u32, b: u32) -> u32 {
        cdecl(h, STAGE_NAME2, &[a, b, 0])
    }

    /// `rcbgFpSaveCurrent_00938340()` -> `rcbgFpSaveSub(the event's jump
    /// map)`.
    pub fn fp_save_current(h: &mut impl Host) -> u32 {
        let map = cdecl(h, JUMP_MAP_NO, &[]);
        cdecl(h, FP_SAVE_SUB, &[map])
    }

    /// `rcbgGuideMapConvCurrent_00938570()`: with a jump map,
    /// `FUN_00938590(map)` -> its `eax`; else 0.
    pub fn guide_map_conv_current(h: &mut impl Host) -> u32 {
        let map = cdecl(h, JUMP_MAP_NO, &[]);
        if map == 0 {
            return 0;
        }
        cdecl(h, GUIDE_MAP_CONV, &[map])
    }

    /// `rcbgFpLoadTextCurrent_0093a7e0()` ->
    /// `rcbgFpLoadTextSub(the jump map, render status bit 12)`.
    pub fn fp_load_text_current(h: &mut impl Host) -> u32 {
        let bit = (cdecl(h, RENDER_STATUS, &[]) >> 12) & 1;
        let map = cdecl(h, JUMP_MAP_NO, &[]);
        cdecl(h, FP_LOAD_TEXT_SUB, &[map, bit])
    }

    fn get_int(h: &mut impl Host, parse: u32) -> u32 {
        cdecl(h, PARSE_GET_INT, &[parse])
    }

    /// A polygon's colour: three ints into the bytes at `at`, alpha 0x80.
    fn rgb(h: &mut impl Host, parse: u32, at: u32) {
        for k in 0..3 {
            let v = get_int(h, parse);
            h.write(at + k, v.to_le_bytes()[0]);
        }
        h.write(at + 3, 0x80_u8);
    }

    /// The texture: flag 2 set, the texture id (`+0x24`) and four UV
    /// pairs (`+0x28`).
    fn uv(h: &mut impl Host, parse: u32, poly: u32, fixed_colour: bool) {
        let f: u32 = h.read(poly);
        h.write(poly, f | 2);
        let id = get_int(h, parse);
        h.write(poly + 0x24, id);
        for k in 0..4 {
            let u = get_int(h, parse);
            h.write(poly + 0x28 + 8 * k, u);
            let v = get_int(h, parse);
            h.write(poly + 0x2c + 8 * k, v);
            if fixed_colour {
                h.write(poly + 0x48 + 4 * k, 0x8080_8080_u32);
            }
        }
    }

    /// `rcdtRsdLoad_LoadMatRsd(model, path)` (frame: the type character at
    /// `ebp - 1`): the `.mat` file into a chunk (none: deleted, "File not
    /// found." printed -> rcPrint's `eax`); a text parse skips the `@`/`#`
    /// lines, reads the polygon count (an assertion message when it isn't
    /// `+0x10`), then per 0x58-byte polygon of `+0x14`: three ints
    /// skipped, the type character, then `C` a flat colour (copied to the
    /// four vertices), `D` texture + flat colour, `G` four colours, `H`
    /// texture + four colours, `T` texture with grey 0x80 colours; others
    /// nothing. The parse freed, the chunk deleted -> its `eax` (0 without
    /// a chunk).
    pub fn load_mat_rsd(h: &mut impl Host, ebp: u32, model: u32, path: u32) -> u32 {
        let chunk = cdecl(h, CHUNK_NEW, &[]);
        if cdecl(h, CHUNK_LOAD_FROM_FILE, &[chunk, path]) == 0 {
            if chunk != 0 {
                cdecl(h, CHUNK_DELETE, &[chunk]);
            }
            let (e, f) = (h.rebase(ERROR_IN), h.rebase(RSD_FILE));
            cdecl(h, RC_PRINT, &[e, f, 0x138]);
            let m = h.rebase(MAT_NOT_FOUND);
            return cdecl(h, RC_PRINT, &[m]);
        }
        let parse = cdecl(h, PARSE_NEW, &[]);
        cdecl(h, PARSE_SET, &[parse, chunk]);
        cdecl(h, PARSE_SET_MODE, &[parse, 1]);
        loop {
            let cur = word(h, parse + 4);
            let c: u8 = h.read(cur);
            if c != b'@' && c != b'#' {
                break;
            }
            cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
        }
        let count = get_int(h, parse);
        if count != word(h, model + 0x10) {
            let (a, f, fmt) = (
                h.rebase(ASSERT_TEXT),
                h.rebase(RSD_FILE),
                h.rebase(ASSERT_FMT),
            );
            cdecl(h, RC_PRINT, &[fmt, a, f, 0x147]);
        }
        let mut poly = word(h, model + 0x14);
        let mut left = count.cast_signed();
        while left > 0 {
            for _ in 0..3 {
                get_int(h, parse);
            }
            let kind = ebp - 1;
            cdecl(h, PARSE_GET_BIN, &[parse, kind, 1]);
            cdecl(h, PARSE_NEXT_TOKEN, &[parse]);
            let c: i8 = h.read(kind);
            match c {
                0x43 => rgb4(h, parse, poly, true),
                0x44 => {
                    uv(h, parse, poly, false);
                    rgb4(h, parse, poly, true);
                }
                0x47 => rgb4(h, parse, poly, false),
                0x48 => {
                    uv(h, parse, poly, false);
                    rgb4(h, parse, poly, false);
                }
                0x54 => uv(h, parse, poly, true),
                _ => {}
            }
            poly = poly.wrapping_add(0x58);
            left -= 1;
        }
        cdecl(h, RC_FREE_THUNK, &[parse]);
        if chunk == 0 {
            return 0;
        }
        cdecl(h, CHUNK_DELETE, &[chunk])
    }

    /// `rcdtRsdLoad_LoadMatRsd2(model, path)` (frame as
    /// `rcdtRsdLoad_LoadMatRsd`): the `.ma2` form: per polygon eight ints
    /// skipped and the type character; `H`: the texture id (`+0x24`) and
    /// four UV pairs (`+0x28`), anything else id -1; then four RGBA colours
    /// (four ints each) at `+0x48`. Errors and the end as the `.mat` form
    /// (its own lines 0x1bf and 0x1cd).
    pub fn load_mat_rsd2(h: &mut impl Host, ebp: u32, model: u32, path: u32) -> u32 {
        let chunk = cdecl(h, CHUNK_NEW, &[]);
        if cdecl(h, CHUNK_LOAD_FROM_FILE, &[chunk, path]) == 0 {
            if chunk != 0 {
                cdecl(h, CHUNK_DELETE, &[chunk]);
            }
            let (e, f) = (h.rebase(ERROR_IN), h.rebase(RSD_FILE));
            cdecl(h, RC_PRINT, &[e, f, 0x1bf]);
            let m = h.rebase(MA2_NOT_FOUND);
            return cdecl(h, RC_PRINT, &[m]);
        }
        let parse = cdecl(h, PARSE_NEW, &[]);
        cdecl(h, PARSE_SET, &[parse, chunk]);
        cdecl(h, PARSE_SET_MODE, &[parse, 1]);
        loop {
            let cur = word(h, parse + 4);
            let c: u8 = h.read(cur);
            if c != b'@' && c != b'#' {
                break;
            }
            cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
        }
        let count = get_int(h, parse);
        if count != word(h, model + 0x10) {
            let (a, f, fmt) = (
                h.rebase(ASSERT_TEXT),
                h.rebase(RSD_FILE),
                h.rebase(ASSERT_FMT),
            );
            cdecl(h, RC_PRINT, &[fmt, a, f, 0x1cd]);
        }
        let mut poly = word(h, model + 0x14);
        let mut left = count.cast_signed();
        while left > 0 {
            for _ in 0..8 {
                get_int(h, parse);
            }
            let kind = ebp - 1;
            cdecl(h, PARSE_GET_BIN, &[parse, kind, 1]);
            cdecl(h, PARSE_NEXT_TOKEN, &[parse]);
            let c: u8 = h.read(kind);
            if c == b'H' {
                let id = get_int(h, parse);
                h.write(poly.wrapping_add(0x24), id);
                for k in 0..4 {
                    let u = get_int(h, parse);
                    h.write(poly.wrapping_add(0x28 + 8 * k), u);
                    let v = get_int(h, parse);
                    h.write(poly.wrapping_add(0x2c + 8 * k), v);
                }
            } else {
                h.write(poly.wrapping_add(0x24), 0xffff_ffff_u32);
            }
            for k in 0..16 {
                let v = get_int(h, parse);
                h.write(poly.wrapping_add(0x48 + k), v.to_le_bytes()[0]);
            }
            poly = poly.wrapping_add(0x58);
            left -= 1;
        }
        cdecl(h, RC_FREE_THUNK, &[parse]);
        if chunk == 0 {
            return 0;
        }
        cdecl(h, CHUNK_DELETE, &[chunk])
    }

    /// `count` 16-byte vectors (three `rcdtParseGetFloat`s stored as f32,
    /// then 1.0) into `rcAlloc(count << 4, rcdtRsd2.c, line)`.
    fn vectors(h: &mut impl Host, parse: u32, count: u32, line: u32) -> u32 {
        let file = h.rebase(RSD_FILE);
        let block = cdecl(h, RC_ALLOC, &[count << 4, file, line]);
        let mut at = block;
        let mut i = 0_i32;
        while i < count.cast_signed() {
            for k in 0..3 {
                let x = h.call_sym_st0(PARSE_GET_FLOAT, Conv::Cdecl, &[parse]);
                h.write(at.wrapping_add(4 * k), x.to_bits());
            }
            h.write(at.wrapping_add(0xc), 1.0_f32.to_bits());
            at = at.wrapping_add(0x10);
            i += 1;
        }
        block
    }

    /// `rcdtRsdLoad_LoadPly(model, path)`: the `.ply` file (not found:
    /// as `rcdtRsdLoad_LoadMatRsd`, line 0xbd); after the `@`/`#` lines
    /// the vertex, normal and polygon counts (`+0`, `+8`, `+0x10`), each
    /// block on its own line: vertices and normals as (x, y, z, 1.0)
    /// floats (`+4`, `+0xc`), polygons 0x58 bytes each (`+0x14`, cleared):
    /// a flag int (non-zero: bit 0), four vertex and four normal indices
    /// (u16). The parse freed, the chunk deleted -> its `eax` (0 without).
    pub fn load_ply(h: &mut impl Host, model: u32, path: u32) -> u32 {
        let chunk = cdecl(h, CHUNK_NEW, &[]);
        if cdecl(h, CHUNK_LOAD_FROM_FILE, &[chunk, path]) == 0 {
            if chunk != 0 {
                cdecl(h, CHUNK_DELETE, &[chunk]);
            }
            let (e, f) = (h.rebase(ERROR_IN), h.rebase(RSD_FILE));
            cdecl(h, RC_PRINT, &[e, f, 0xbd]);
            let m = h.rebase(PLY_NOT_FOUND);
            return cdecl(h, RC_PRINT, &[m]);
        }
        let parse = cdecl(h, PARSE_NEW, &[]);
        cdecl(h, PARSE_SET, &[parse, chunk]);
        cdecl(h, PARSE_SET_MODE, &[parse, 1]);
        loop {
            let cur = word(h, parse + 4);
            let c: u8 = h.read(cur);
            if c != b'@' && c != b'#' {
                break;
            }
            cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
        }
        for off in [0_u32, 8, 0x10] {
            let n = get_int(h, parse);
            h.write(model + off, n);
        }
        cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
        let n = word(h, model);
        let v = vectors(h, parse, n, 0xd1);
        h.write(model + 4, v);
        cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
        let n = word(h, model + 8);
        let v = vectors(h, parse, n, 0xdf);
        h.write(model + 0xc, v);
        cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
        let n = word(h, model + 0x10).wrapping_mul(0x58);
        let file = h.rebase(RSD_FILE);
        let polys = cdecl(h, RC_ALLOC, &[n, file, 0xec]);
        h.write(model + 0x14, polys);
        let n = word(h, model + 0x10).wrapping_mul(0x58);
        cdecl(h, MEMSET, &[polys, 0, n]);
        let mut poly = word(h, model + 0x14);
        let mut i = 0_i32;
        while i < word(h, model + 0x10).cast_signed() {
            if get_int(h, parse) != 0 {
                let f: u32 = h.read(poly);
                h.write(poly, f | 1);
            }
            for k in 0..8 {
                let x = get_int(h, parse) & 0xffff;
                h.write(poly.wrapping_add(4 + 4 * k), x);
            }
            poly = poly.wrapping_add(0x58);
            i += 1;
        }
        cdecl(h, RC_FREE_THUNK, &[parse]);
        if chunk == 0 {
            return 0;
        }
        cdecl(h, CHUNK_DELETE, &[chunk])
    }

    fn c_len(h: &impl Mem, s: u32) -> u32 {
        let mut n = 0_u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n += 1;
        }
        n
    }

    /// `dst = src` then `dst += tail` (with its NUL), as the original's
    /// inline copies.
    fn join(h: &mut impl Host, dst: u32, src: u32, tail: u32) {
        let n = c_len(h, src);
        for i in 0..=n {
            let b: u8 = h.read(src.wrapping_add(i));
            h.write(dst.wrapping_add(i), b);
        }
        let m = c_len(h, tail);
        for i in 0..=m {
            let b: u8 = h.read(tail.wrapping_add(i));
            h.write(dst.wrapping_add(n + i), b);
        }
    }

    /// `rcdtRsdLoad_009243c0(model, path)` (frame: the keyword table at
    /// `ebp - 0x328`, the directory at `- 0x304`, a path at `- 0x204`, the
    /// token at `- 0x104`) -> 1, or 0 when the `.rsd` file isn't found
    /// (chunk deleted, line 0x69, "rcdtRsdLoad : File not found."). The
    /// directory is `path` up to its last `'/'`. A text parse with `[]=`
    /// as separators reads until its end: `@`/`#` lines skipped; `PLY`
    /// and `MAT` name a file (lowercased, after the directory) for
    /// `rcdtRsdLoad_LoadPly` / `rcdtRsdLoad_LoadMat_00923b60`; `GRP` reads
    /// and lowercases a name, `VGR` reads one; `NTEX` n (> 0) allocates
    /// `n` texture-name pointers (`+0x1c`), `TEX` i a name buffer of its
    /// token's length + 1 at slot i and reads the name into it.
    pub fn rsd_load(h: &mut impl Host, ebp: u32, model: u32, path: u32) -> u32 {
        let table = ebp - 0x328;
        for (i, k) in RSD_KEYWORDS.iter().enumerate() {
            let v = if *k == 0 { 0 } else { h.rebase(*k) };
            h.write(table + 4 * i as u32, v);
        }
        let dir = ebp - 0x304;
        let n = c_len(h, path);
        for i in 0..=n {
            let b: u8 = h.read(path.wrapping_add(i));
            h.write(dir + i, b);
        }
        let f = word(h, h.rebase(STRRCHR));
        let slash = h.call(f, Conv::Cdecl, &[dir, u32::from(b'/')]);
        if slash != 0 {
            h.write(slash.wrapping_add(1), 0_u8);
        }
        let chunk = cdecl(h, CHUNK_NEW, &[]);
        if cdecl(h, CHUNK_LOAD_FROM_FILE, &[chunk, path]) == 0 {
            if chunk != 0 {
                cdecl(h, CHUNK_DELETE, &[chunk]);
            }
            let (e, f) = (h.rebase(ERROR_IN), h.rebase(RSD_FILE));
            cdecl(h, RC_PRINT, &[e, f, 0x69]);
            let m = h.rebase(RSD_NOT_FOUND);
            cdecl(h, RC_PRINT, &[m]);
            return 0;
        }
        let parse = cdecl(h, PARSE_NEW, &[]);
        cdecl(h, PARSE_SET, &[parse, chunk]);
        cdecl(h, PARSE_SET_MODE, &[parse, 1]);
        let seps = h.rebase(RSD_SEPARATORS);
        cdecl(h, PARSE_ADD_SEPARATORS, &[parse, seps]);
        let tok = ebp - 0x104;
        let file_path = ebp - 0x204;
        let lower = word(h, h.rebase(STRLWR));
        if cdecl(h, PARSE_IS_END, &[parse]) == 0 {
            loop {
                let cur = word(h, parse + 4);
                let c: u8 = h.read(cur);
                if c == b'@' || c == b'#' {
                    cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
                } else {
                    match cdecl(h, PARSE_GET_KEYWORD, &[parse, table]) {
                        k @ (0 | 1) => {
                            cdecl(h, PARSE_GET_TEXT, &[parse, tok]);
                            h.call(lower, Conv::Cdecl, &[tok]);
                            join(h, file_path, dir, tok);
                            let load = if k == 0 { LOAD_PLY } else { LOAD_MAT };
                            cdecl(h, load, &[model, file_path]);
                        }
                        2 => {
                            cdecl(h, PARSE_GET_TEXT, &[parse, tok]);
                            h.call(lower, Conv::Cdecl, &[tok]);
                        }
                        3 => {
                            let n = get_int(h, parse);
                            if n.cast_signed() > 0 {
                                let file = h.rebase(RSD_FILE);
                                let p = cdecl(h, RC_ALLOC, &[n << 2, file, 0x94]);
                                h.write(model + 0x1c, p);
                            }
                        }
                        4 => {
                            let i = get_int(h, parse);
                            let len = cdecl(h, PARSE_TOKEN_LEN_TEXT, &[parse]);
                            let file = h.rebase(RSD_FILE);
                            let p = cdecl(h, RC_ALLOC, &[len.wrapping_add(1), file, 0x9b]);
                            let names = word(h, model + 0x1c);
                            h.write(names.wrapping_add(i.wrapping_mul(4)), p);
                            let names = word(h, model + 0x1c);
                            let p = word(h, names.wrapping_add(i.wrapping_mul(4)));
                            cdecl(h, PARSE_GET_TEXT, &[parse, p]);
                        }
                        5 => {
                            cdecl(h, PARSE_GET_TEXT, &[parse, tok]);
                        }
                        _ => {}
                    }
                }
                if cdecl(h, PARSE_IS_END, &[parse]) != 0 {
                    break;
                }
            }
        }
        if parse != 0 {
            cdecl(h, RC_FREE_THUNK, &[parse]);
        }
        if chunk != 0 {
            cdecl(h, CHUNK_DELETE, &[chunk]);
        }
        1
    }

    /// The four vertex colours at `+0x48`: one read and copied (`flat`) or
    /// four read.
    fn rgb4(h: &mut impl Host, parse: u32, poly: u32, flat: bool) {
        if flat {
            rgb(h, parse, poly + 0x48);
            let c: u32 = h.read(poly + 0x48);
            for k in 1..4 {
                h.write(poly + 0x48 + 4 * k, c);
            }
        } else {
            for k in 0..4 {
                rgb(h, parse, poly + 0x48 + 4 * k);
            }
        }
    }

    /// `rcdtTm2Load(pic, path)` -> `rcdtTm2Load_009232a0(pic, path, 1)`.
    pub fn tm2_load_ps2(h: &mut impl Host, pic: u32, path: u32) -> u32 {
        cdecl(h, TM2_LOAD, &[pic, path, 1])
    }
}
