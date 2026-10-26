//! The sphere grid's render data (`0x00681c10`, 0xd0 bytes), one per
//! part `GraphicAbmap`'s constructor makes.
//!
//! Layout: a matrix (`+0..0x40`, set to the identity), the segment count
//! (`+0x90`) and the segments (`+0x94`, 0x6c bytes each), the bounds
//! (`+0x98..0xac`), two words (`+0xb8 = 2`, `+0xbc = 0`). A segment: a
//! word (`+0`), the vertex and index counts (`+4`, `+8`), buffers
//! (`+0xc`, `+0x10`, `+0x14` positions, `+0x18` texture coordinates,
//! `+0x1c` indices), the texture name (`+0x20`, 0x100 bytes), `+0x24 =
//! 1`, and six words cleared (`+0x2c..0x34`, `+0x48..0x50`).

use ffx_graphic::host::{Conv, Host};

use crate::p4_mesh::OPERATOR_NEW;

/// `operator_new_006304c0` (cdecl `(size)`).
pub const OPERATOR_NEW2: u32 = 0x0063_04c0;
/// `crt_memset_thunk` (cdecl `(p, value, n)`).
pub const MEMSET: u32 = 0x0094_964c;
/// `memcpy` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// `TOGetFFXLang` (cdecl `()`): the game's language.
pub const GET_LANG: u32 = 0x008a_c2f0;
/// `RemapButtonIcon_sGetInstance` (cdecl `()`).
pub const REMAP_INSTANCE: u32 = 0x006e_7710;
/// `AddrOfGlobal00c35030_006e6650` (`ecx` the remap instance, unread).
pub const REMAP_GLOBAL: u32 = 0x006e_6650;
/// `RemapButtonIcon_getPadIconPath` (thiscall `(remap)`).
pub const REMAP_PAD_ICON_PATH: u32 = 0x006e_6660;
/// `fiosUnifyFilename` (cdecl `(name, out, size)`).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
/// `"NoTexture"` (`.rdata`, nine bytes copied without the NUL).
pub const NO_TEXTURE: u32 = 0x00b4_1b50;

/// The texture-name pointers (`.data` words holding `.rdata` string
/// addresses) a kind's name comes from: (owner 0, owner 0 in Japanese,
/// owner 2, owner 2 in Japanese).
#[derive(Clone, Copy)]
struct Names(u32, u32, u32, u32);

/// Kinds 1, 2, 5 and 6 (shared), 9, 10.
const NAMES1: Names = Names(0x00c3_3984, 0x00c3_39ec, 0x00c3_39c0, 0x00c3_3a28);
const NAMES2: Names = Names(0x00c3_39a4, 0x00c3_3a0c, 0x00c3_39e0, 0x00c3_3a48);
const NAMES5: Names = Names(0x00c3_3980, 0x00c3_39e8, 0x00c3_39bc, 0x00c3_3a24);
const NAMES9: Names = Names(0x00c3_39a8, 0x00c3_3a10, 0x00c3_39e4, 0x00c3_3a4c);
const NAMES10: Names = Names(0x00c3_398c, 0x00c3_39f4, 0x00c3_39c8, 0x00c3_3a30);
/// Kind 3's second segment.
const NAMES3: Names = Names(0x00c3_3990, 0x00c3_39f8, 0x00c3_39cc, 0x00c3_3a34);

/// Kind 3's first name by language (2, 3, 5, 0xb; else the default).
fn names3_first(lang: u32) -> u32 {
    match lang {
        2 => 0x00c3_39b8,
        3 => 0x00c3_39b0,
        5 => 0x00c3_39b4,
        0xb => 0x00c3_3a14,
        _ => 0x00c3_39ac,
    }
}

/// Kinds 0xb and 0xc's name pointers by language: segment 2's, 3's, 0's
/// and 1's (the jump tables `0x00684344`/`0x0068436c`).
fn names4(lang: u32) -> [u32; 4] {
    match lang {
        1..=5 | 0x12 => [0x00c5_95e8, 0x00c5_95ec, 0x00c5_9628, 0x00c5_962c],
        9 => [0x00c5_9768, 0x00c5_976c, 0x00c5_97a8, 0x00c5_97ac],
        10 => [0x00c5_9668, 0x00c5_966c, 0x00c5_96e8, 0x00c5_96ec],
        0xb => [0x00c5_96a8, 0x00c5_96ac, 0x00c5_9728, 0x00c5_972c],
        _ => [0x00c5_9568, 0x00c5_956c, 0x00c5_95a8, 0x00c5_95ac],
    }
}

/// Kind 6's second and third segments' names.
const NAMES6: [u32; 2] = [0x00c5_9d5c, 0x00c5_9d64];

/// One segment's vertex and index counts.
#[derive(Clone, Copy)]
struct Counts(u32, u32);

pub mod bridge {
    //! The originals over game memory.

    use super::{
        Conv, Counts, GET_LANG, Host, MEMCPY, MEMSET, NAMES1, NAMES2, NAMES3, NAMES5, NAMES6,
        NAMES9, NAMES10, NO_TEXTURE, Names, OPERATOR_NEW, OPERATOR_NEW2, REMAP_GLOBAL,
        REMAP_INSTANCE, REMAP_PAD_ICON_PATH, UNIFY_FILENAME, names3_first, names4,
    };

    /// The bytes of the original's frame callees see: the file name
    /// buffer at `ebp - 0x104` (0x100 bytes; the security cookie above).
    pub const FRAME: u32 = 0x104;

    fn new2(h: &mut impl Host, size: u32) -> u32 {
        h.call_sym(OPERATOR_NEW2, Conv::Cdecl, &[size])
    }

    fn memset(h: &mut impl Host, seg: u32, field: u32, size: u32) {
        let p: u32 = h.read(seg.wrapping_add(field));
        h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, size]);
    }

    /// `strlen` as the original's inline loop counts it.
    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `memcpy(the buffer at segs + field, s, strlen(s))` for the string
    /// the `.data` word `ptr` points at (no NUL: the buffer was zeroed).
    fn name(h: &mut impl Host, segs: u32, field: u32, ptr: u32) {
        let s: u32 = h.read(h.rebase(ptr));
        let n = strlen(h, s);
        let dst: u32 = h.read(segs.wrapping_add(field));
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, s, n]);
    }

    /// The owner-chosen name at `segs + field`: owner 0 or 2 copies its
    /// name and, in Japanese (language 0xb), the Japanese one over it;
    /// any other owner copies none (and never asks the language).
    fn owner_name(h: &mut impl Host, abmap: u32, segs: u32, field: u32, names: Names) {
        let owner: u32 = h.read(abmap);
        let (first, japanese) = match owner {
            0 => (names.0, names.1),
            2 => (names.2, names.3),
            _ => return,
        };
        name(h, segs, field, first);
        if h.call_sym(GET_LANG, Conv::Cdecl, &[]) == 0xb {
            name(h, segs, field, japanese);
        }
    }

    /// A segment's buffers in the original's allocation order: the name
    /// (0x100), `+0x14` (`v * 16`), `+0x1c` (`i * 2`), `+0x10` (`v * 12`;
    /// kind 3 clears the word instead), `+0x18` (`v * 8`), `+0xc` (`v *
    /// 12`); the counts and `+0x24 = 1`.
    fn buffers(h: &mut impl Host, seg: u32, c: Counts, with_10: bool) {
        let Counts(v, i) = c;
        h.write(seg.wrapping_add(4), v);
        h.write(seg.wrapping_add(8), i);
        let p = new2(h, 0x100);
        h.write(seg.wrapping_add(0x20), p);
        let p = new2(h, v * 16);
        h.write(seg.wrapping_add(0x14), p);
        let p = new2(h, i * 2);
        h.write(seg.wrapping_add(0x1c), p);
        if with_10 {
            let p = new2(h, v * 12);
            h.write(seg.wrapping_add(0x10), p);
        } else {
            h.write(seg.wrapping_add(0x10), 0u32);
        }
        let p = new2(h, v * 8);
        h.write(seg.wrapping_add(0x18), p);
        let p = new2(h, v * 12);
        h.write(seg.wrapping_add(0xc), p);
        h.write(seg.wrapping_add(0x24), 1u32);
    }

    /// The four clears after the name (`+0x14`, `+0x1c`, `+0x10`,
    /// `+0x18`).
    fn clear4(h: &mut impl Host, seg: u32, c: Counts) {
        let Counts(v, i) = c;
        memset(h, seg, 0x14, v * 16);
        memset(h, seg, 0x1c, i * 2);
        memset(h, seg, 0x10, v * 12);
        memset(h, seg, 0x18, v * 8);
    }

    /// All six clears: the name, [`clear4`], `+0xc`.
    fn clear6(h: &mut impl Host, seg: u32, c: Counts) {
        memset(h, seg, 0x20, 0x100);
        clear4(h, seg, c);
        memset(h, seg, 0xc, c.0 * 12);
    }

    /// The word `+0` and the six words the original clears last.
    fn finish(h: &mut impl Host, seg: u32, first: u32) {
        h.write(seg, first);
        for off in [0x2c, 0x48, 0x30, 0x4c, 0x34, 0x50] {
            h.write(seg.wrapping_add(off), 0u32);
        }
    }

    /// `"NoTexture"` over the cleared name.
    fn no_texture(h: &mut impl Host, seg: u32) {
        let dst: u32 = h.read(seg.wrapping_add(0x20));
        let src = h.rebase(NO_TEXTURE);
        for k in 0..9 {
            let b: u8 = h.read(src + k);
            h.write(dst.wrapping_add(k), b);
        }
    }

    /// The segments: `size` bytes from `operator_new` (or
    /// `operator_new_006304c0` when `array`) into `obj + 0x94`, the count
    /// `n` into `+0x90`.
    fn segments(h: &mut impl Host, obj: u32, size: u32, n: u32, array: bool) -> u32 {
        let segs = if array {
            new2(h, size)
        } else {
            h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[size])
        };
        h.write(obj.wrapping_add(0x94), segs);
        h.write(obj.wrapping_add(0x90), n);
        segs
    }

    /// One segment named by the owner (kinds 1, 2, 9, 10): its buffers,
    /// the name cleared and set, the other clears, the word `+0` 1.
    fn owner_kind(h: &mut impl Host, abmap: u32, obj: u32, c: Counts, names: Names) {
        let seg = segments(h, obj, 0x6c, 1, false);
        buffers(h, seg, c, true);
        memset(h, seg, 0x20, 0x100);
        owner_name(h, abmap, seg, 0x20, names);
        clear4(h, seg, c);
        memset(h, seg, 0xc, c.0 * 12);
        finish(h, seg, 1);
    }

    /// One `"NoTexture"` segment (kinds 4, 7, 8), its word `+0` 0.
    fn no_texture_kind(h: &mut impl Host, obj: u32, c: Counts) {
        let seg = segments(h, obj, 0x6c, 1, false);
        buffers(h, seg, c, true);
        memset(h, seg, 0x20, 0x100);
        no_texture(h, seg);
        clear4(h, seg, c);
        memset(h, seg, 0xc, c.0 * 12);
        finish(h, seg, 0);
    }

    /// Four segments of the same counts named by the language (kinds 0xb
    /// and 0xc).
    fn four_kind(h: &mut impl Host, obj: u32, c: Counts) {
        let segs = segments(h, obj, 0x1b0, 4, true);
        for k in 0..4 {
            let seg = segs + 0x6c * k;
            buffers(h, seg, c, true);
            clear6(h, seg, c);
            finish(h, seg, 1);
        }
        let lang = h.call_sym(GET_LANG, Conv::Cdecl, &[]);
        for (ptr, field) in names4(lang).into_iter().zip([0xf8, 0x164, 0x20, 0x8c]) {
            name(h, segs, field, ptr);
        }
    }

    /// The button remap's path (`getter` on its instance) copied into
    /// `dst`'s buffer: its length first, then the path again (two
    /// instance and getter calls, as the original makes).
    fn remap_path(h: &mut impl Host, getter: u32, buffer: u32) {
        let remap = h.call_sym(REMAP_INSTANCE, Conv::Cdecl, &[]);
        let s = h.call_sym(getter, Conv::Thiscall(remap), &[]);
        let n = strlen(h, s);
        let remap = h.call_sym(REMAP_INSTANCE, Conv::Cdecl, &[]);
        let s = h.call_sym(getter, Conv::Thiscall(remap), &[]);
        let dst: u32 = h.read(buffer);
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, s, n]);
    }

    /// `GraphicAbmap::createRenderData(abmap, kind)` (`0x00681c10`,
    /// thiscall, `ret 4`) -> the new render data (0xd0 bytes from
    /// `operator_new`, never checked): `+0xb8 = 2`, `+0xbc = 0`; per kind
    /// 0..0xd its segments with their buffers cleared and their texture
    /// names (by the owner `[abmap]`, the language, `"NoTexture"`, or the
    /// button remap's paths), any other kind none; then the identity
    /// matrix, the bounds -1000 (`+0x98..0xa0`) and 2000 (`+0xa4..0xac`),
    /// and each segment's name put through `fiosUnifyFilename` into the
    /// frame's buffer (`ebp - 0x104`, the port's `frame`) and copied back
    /// without its NUL.
    #[allow(clippy::too_many_lines)]
    pub fn create_render_data(h: &mut impl Host, frame: u32, abmap: u32, kind: u32) -> u32 {
        let obj = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xd0]);
        h.write(obj.wrapping_add(0xb8), 2u32);
        h.write(obj.wrapping_add(0xbc), 0u32);
        match kind {
            0 => {
                let c = Counts(4, 6);
                let seg = segments(h, obj, 0x6c, 1, false);
                buffers(h, seg, c, true);
                memset(h, seg, 0x20, 0x100);
                no_texture(h, seg);
                memset(h, seg, 0x14, 0x40);
                // the index and texture-coordinate buffers cleared inline
                let p: u32 = h.read(seg.wrapping_add(0x1c));
                for k in 0..3 {
                    h.write(p.wrapping_add(4 * k), 0u32);
                }
                memset(h, seg, 0x10, 0x30);
                let p: u32 = h.read(seg.wrapping_add(0x18));
                for k in 0..8 {
                    h.write(p.wrapping_add(4 * k), 0u32);
                }
                memset(h, seg, 0xc, 0x30);
                finish(h, seg, 0);
            }
            1 => owner_kind(h, abmap, obj, Counts(0x5e8, 0x8dc), NAMES1),
            2 => owner_kind(h, abmap, obj, Counts(600, 900), NAMES2),
            3 => {
                let c = Counts(0xd74, 0x142e);
                let segs = segments(h, obj, 0xd8, 2, true);
                for k in 0..2 {
                    let seg = segs + 0x6c * k;
                    buffers(h, seg, c, false);
                    memset(h, seg, 0x20, 0x100);
                    memset(h, seg, 0x14, c.0 * 16);
                    memset(h, seg, 0x1c, c.1 * 2);
                    memset(h, seg, 0x18, c.0 * 8);
                    memset(h, seg, 0xc, c.0 * 12);
                    finish(h, seg, 1);
                }
                let lang = h.call_sym(GET_LANG, Conv::Cdecl, &[]);
                name(h, segs, 0x20, names3_first(lang));
                owner_name(h, abmap, segs, 0x8c, NAMES3);
            }
            4 => no_texture_kind(h, obj, Counts(0x2070, 0x30a8)),
            5 => {
                let c = Counts(0x38, 0x54);
                let seg = segments(h, obj, 0x6c, 1, false);
                buffers(h, seg, c, true);
                clear6(h, seg, c);
                owner_name(h, abmap, seg, 0x20, NAMES5);
                finish(h, seg, 1);
            }
            6 => {
                let segs = segments(h, obj, 0x1b0, 4, true);
                let c = Counts(0x78, 0xb4);
                buffers(h, segs, c, true);
                clear6(h, segs, c);
                finish(h, segs, 1);
                owner_name(h, abmap, segs, 0x20, NAMES5);
                let rest = [Counts(0xa0, 0xf0), Counts(0x14, 0x1e), Counts(0x14, 0x1e)];
                for (k, c) in (1u32..).zip(rest) {
                    let seg = segs + 0x6c * k;
                    buffers(h, seg, c, true);
                    clear6(h, seg, c);
                    finish(h, seg, 1);
                    if let Some(&ptr) = NAMES6.get(k as usize - 1) {
                        name(h, segs, 0x6c * k + 0x20, ptr);
                    }
                }
                remap_path(h, REMAP_GLOBAL, segs + 0x164);
            }
            7 => no_texture_kind(h, obj, Counts(0x5e10, 0x8d18)),
            8 => no_texture_kind(h, obj, Counts(0x3400, 0x4e00)),
            9 => owner_kind(h, abmap, obj, Counts(0x2f08, 0x468c), NAMES9),
            10 => owner_kind(h, abmap, obj, Counts(0xaa0, 0xff0), NAMES10),
            0xb => four_kind(h, obj, Counts(0xa0, 0xf0)),
            0xc => four_kind(h, obj, Counts(0x4b0, 0x708)),
            0xd => {
                let c = Counts(0x14, 0x1e);
                let seg = segments(h, obj, 0x6c, 1, false);
                buffers(h, seg, c, true);
                clear6(h, seg, c);
                finish(h, seg, 1);
                remap_path(h, REMAP_PAD_ICON_PATH, seg + 0x20);
            }
            _ => {}
        }
        // the identity (`fld1`/`fldz`), then the bounds (two `.rdata`
        // floats stored as they are)
        for k in 0..16u32 {
            let w = if k % 5 == 0 { 0x3f80_0000u32 } else { 0 };
            h.write(obj.wrapping_add(4 * k), w);
        }
        for k in 0..3 {
            h.write(obj.wrapping_add(0x98 + 4 * k), 0xc47a_0000u32);
            h.write(obj.wrapping_add(0xa4 + 4 * k), 0x44fa_0000u32);
        }
        let mut k = 0i32;
        while k < h.read::<i32>(obj.wrapping_add(0x90)) {
            let at = (k as u32).wrapping_mul(0x6c).wrapping_add(0x20);
            let segs: u32 = h.read(obj.wrapping_add(0x94));
            let name: u32 = h.read(segs.wrapping_add(at));
            h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[name, frame, 0x100]);
            let n = strlen(h, frame);
            let segs: u32 = h.read(obj.wrapping_add(0x94));
            let dst: u32 = h.read(segs.wrapping_add(at));
            h.call_sym(MEMCPY, Conv::Cdecl, &[dst, frame, n]);
            k += 1;
        }
        obj
    }
}
