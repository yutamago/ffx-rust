//! Continuation round 9 of list `h4-glue-other`, batch 4: the event
//! shapes' VFX render data buffers, the shape fields' buffers.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

/// `sprintf`'s import slot.
pub const SPRINTF: Global<u32> = Global::new(0x00b0_c3dc);
/// `graphicVFXIsTextureLoaded(name, path, a, b, c, d)` (`0x00645400`,
/// cdecl): writes the loaded texture's path and four words.
pub const IS_TEXTURE_LOADED: u32 = 0x0064_5400;
/// `dbgPrintf(format, ...)` (`0x0062f500`, cdecl).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `calculateVFXSP_AlphaFunction(mode)` (`0x00712c30`, cdecl).
pub const ALPHA_FUNCTION: u32 = 0x0071_2c30;
/// `graphicVFXCreate(lo, hi, kind)` (`0x00644de0`, cdecl).
pub const VFX_CREATE: u32 = 0x0064_4de0;
/// `graphicVFXCreateDataBuffer(lo, hi, 1, &vertices, &indices, texture,
/// &one)` (`0x00645040`, cdecl).
pub const VFX_CREATE_DATA_BUFFER: u32 = 0x0064_5040;
/// `"%d_%d_%d.dds.phyre"` (`0x00b50080`), `"%s"` (`0x00b0e160`), the
/// missing-texture message (`0x00b4ff58`), the too-small message
/// (`0x00b50098`).
pub const TEXTURE_FORMAT: u32 = 0x00b5_0080;
/// See [`TEXTURE_FORMAT`].
pub const PERCENT_S: u32 = 0x00b0_e160;
/// See [`TEXTURE_FORMAT`].
pub const NO_TEXTURE: u32 = 0x00b4_ff58;
/// See [`TEXTURE_FORMAT`].
pub const TOO_SMALL: u32 = 0x00b5_0098;
/// The event's texture base (`0x023cbc5c`) and `g_AtelEventEbpAddr64`'s
/// low word (`0x013270f8`): the texture number is their difference.
pub const TEXTURE_BASE: u32 = 0x023c_bc5c;
/// See [`TEXTURE_BASE`].
pub const EVENT_EBP: u32 = 0x0132_70f8;
/// The shape buffer's capacity in shapes (`0x00c3a4d4`; 12 vertices
/// each).
pub const SHAPE_MAX: u32 = 0x00c3_a4d4;
/// Bytes of [`create_atel_shp`]'s frame, from the original's `ebp -
/// 0x268`: the texture outputs (`+0`, `+4`, `+8`), the word 1 (`+0xc`),
/// the fourth output (`+0x10`; `+0x14`, `+0x18` are the original's
/// temporaries, which no callee sees), the six
/// kinds (`+0x1c`), index counts (`+0x34`) and vertex counts (`+0x4c`),
/// the texture's path (`+0x64`, 0x100 bytes) and name (`+0x164`).
pub const ATEL_FRAME: u32 = 0x264;

/// The bucket kinds, by alpha function: 0, 2, 0x10, 0x20, 0x100, 0x200.
const ALPHAS: [u32; 6] = [0, 2, 0x10, 0x20, 0x100, 0x200];
/// The kinds `graphicVFXCreate` gets per bucket.
const KINDS: [u32; 6] = [8, 10, 0x18, 0x28, 0x108, 0x208];

/// The length of the string at `s`.
fn text_len(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `createVFXRenderDataBufferAtelShp(lo, hi, shapes, index)`
/// (`0x007147b0`, cdecl; `frame` see [`ATEL_FRAME`]) -> 1, or 0 when the
/// shape set has no primitives, its texture is not loaded or a bucket
/// overflows. The set is `shapes` + `i16 [shapes + 0x10 + 8 index]`:
/// `+2` primitive count (i16), `+4` offset of the primitives (8 bytes,
/// alpha mode at `+4`), `+0x10`/`+0x14` the texture's size bits (the 64-bit
/// sum `(+0x14 << 32) + sext(+0x10)`, bits 26..29 and 30..33 the log2
/// width and height). The texture is named `"<n>_<w>_<h>.dds.phyre"`
/// (the loaded path when longer). Each primitive adds 12 vertices and 6
/// indices to its alpha function's bucket; each non-empty bucket k is
/// created as VFX instance (`lo`, `hi`) + 10000 k with its data buffer.
pub fn create_atel_shp(
    h: &mut impl Host,
    frame: u32,
    lo: u32,
    hi: u32,
    shapes: u32,
    index: u32,
) -> u32 {
    let index = index & 0xffff;
    let offset = h.read::<i16>(shapes.wrapping_add(0x10 + 8 * index));
    let set = (i32::from(offset) as u32).wrapping_add(shapes);
    let vertices = frame + 0x4c;
    let indices = frame + 0x34;
    for k in 0..6 {
        h.write(vertices + 4 * k, 0u32);
    }
    for k in 0..6 {
        h.write(indices + 4 * k, 0u32);
    }
    let one = frame + 0xc;
    h.write(one, 1u32);
    let prims = h.read::<u32>(set.wrapping_add(4)).wrapping_add(set);
    if h.read::<i16>(set.wrapping_add(2)) <= 0 {
        return 0;
    }
    let size_lo: u32 = h.read(set.wrapping_add(0x10));
    let size_hi: u32 = h.read(set.wrapping_add(0x14));
    let bits = (u64::from(size_hi) << 32).wrapping_add(i64::from(size_lo as i32) as u64);
    let width = 1u32 << ((bits >> 26) as u32 & 0xf);
    let height = 1u32 << ((bits >> 30) as u32 & 0xf);
    let number = h
        .read::<u32>(h.rebase(TEXTURE_BASE))
        .wrapping_sub(h.read(h.rebase(EVENT_EBP)));
    let name = frame + 0x164;
    let path = frame + 0x64;
    let sprintf = SPRINTF.get(h);
    let format = h.rebase(TEXTURE_FORMAT);
    h.call(sprintf, Conv::Cdecl, &[name, format, number, width, height]);
    let loaded = h.call_sym(
        IS_TEXTURE_LOADED,
        Conv::Cdecl,
        &[name, path, frame + 4, frame + 8, frame, frame + 0x10],
    );
    if loaded == 0 {
        let text = h.rebase(NO_TEXTURE);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[text, name]);
        return 0;
    }
    if text_len(h, path) > text_len(h, name) {
        let percent_s = h.rebase(PERCENT_S);
        h.call(sprintf, Conv::Cdecl, &[name, percent_s, path]);
    }
    for (k, kind) in KINDS.into_iter().enumerate() {
        h.write(frame + 0x1c + 4 * k as u32, kind);
    }
    let mut i = 0i32;
    let mut mode = prims.wrapping_add(4);
    while i < i32::from(h.read::<i16>(set.wrapping_add(2))) {
        let m = u32::from(h.read::<u8>(mode));
        let alpha = h.call_sym(ALPHA_FUNCTION, Conv::Cdecl, &[m]);
        if let Some(k) = ALPHAS.iter().position(|&a| a == alpha) {
            let v = vertices + 4 * k as u32;
            let n: u32 = h.read(v);
            h.write(v, n.wrapping_add(0xc));
            let x = indices + 4 * k as u32;
            let n: u32 = h.read(x);
            h.write(x, n.wrapping_add(6));
        }
        i += 1;
        mode = mode.wrapping_add(8);
    }
    for k in 0..6u32 {
        let count = vertices + 4 * k;
        let n: i32 = h.read(count);
        if n <= 0 {
            continue;
        }
        let max = h.read::<u32>(h.rebase(SHAPE_MAX)).wrapping_mul(12) as i32;
        if n > max {
            let text = h.rebase(TOO_SMALL);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[text, k]);
            return 0;
        }
        let id = u64::from(lo).wrapping_add(u64::from(10_000 * k));
        let id_lo = id as u32;
        let id_hi = hi.wrapping_add((id >> 32) as u32);
        let kind: u32 = h.read(frame + 0x1c + 4 * k);
        h.call_sym(VFX_CREATE, Conv::Cdecl, &[id_lo, id_hi, kind]);
        h.call_sym(
            VFX_CREATE_DATA_BUFFER,
            Conv::Cdecl,
            &[id_lo, id_hi, 1, count, indices + 4 * k, name, one],
        );
    }
    1
}

// ---- createVFXRenderDataBufferShapeField

/// `memset` through its import thunk (`0x0094964c`, cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `get_ptr32_func(p)` (`0x00a44650`, cdecl): the shape data.
pub const GET_PTR32: u32 = 0x00a4_4650;
/// `Ch_GetChrRenderInsatanceFromOEF2Content(shape)` (`0x00826af0`,
/// cdecl): the monster whose effect texture a shape uses; -1 none.
pub const CHR_FROM_CONTENT: u32 = 0x0082_6af0;
/// `graphicGetMonsterEtTextureName(chr, texture)` (`0x00641330`, cdecl).
pub const MONSTER_TEXTURE_NAME: u32 = 0x0064_1330;
/// `graphicVFXIsTextureWithBpLoaded(texture, bpp, 0, 0, w, h, name)`
/// (`0x00645450`, cdecl): writes the texture's name.
pub const IS_TEXTURE_WITH_BP_LOADED: u32 = 0x0064_5450;
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (`0x006453d0`, cdecl).
pub const GET_RENDER_DATA: u32 = 0x0064_53d0;
/// The texture-not-loaded message (`0x00b4ff04`).
pub const SHAPE_NO_TEXTURE: u32 = 0x00b4_ff04;
/// `g_VfxYiqMode` (`0x00c3a504`; -1 off) and `g_CurrentMapNo`
/// (`0x012fbc40`): YIQ shapes on map 0x188.
pub const YIQ_MODE: u32 = 0x00c3_a504;
/// See [`YIQ_MODE`].
pub const CURRENT_MAP: u32 = 0x012f_bc40;
/// Bytes of [`create_shape_field`]'s frame, from the original's `ebp -
/// 0x6f20` (its cookie excluded): seven 100-word arrays at `+0x2c` (the
/// data buffers' flags, first word 1), `+0x1bc` (texture numbers),
/// `+0x34c` (bit depths), `+0x4dc` (heights), `+0x66c` (widths),
/// `+0x7fc` (vertex counts), `+0x98c` (index counts), then 100 names of
/// 0x100 bytes at `+0xb1c`. The words below `+0x2c` are the original's
/// temporaries, which no callee sees; the last name's last word is where
/// the original's `__chkstk` call left `ecx`.
pub const SHAPE_FRAME: u32 = 0x6f1c;
/// Entries per array.
const SLOTS: u32 = 100;

/// A shape's primitives (`+2`, i16) times `scale`.
fn primitives(h: &impl Host, sub: u32, scale: u32) -> i32 {
    i32::from(h.read::<i16>(sub.wrapping_add(2))).wrapping_mul(scale as i32)
}

/// `createVFXRenderDataBufferShapeField(lo, hi, shapes, scale, flags)`
/// (`0x00714d40`, cdecl; `frame` see [`SHAPE_FRAME`]) -> 1, or 0 when a
/// texture is not loaded, its monster is missing (-1) or the render data
/// is. The shapes (`get_ptr32_func([shapes])`, `+6` count, `+0x10` i16
/// offsets 8 apart) are grouped by texture (number: low 14 bits of the
/// 64-bit `(+0x14 << 32) + sext(+0x10)`, bit depth bits 20..25, log2 width
/// 26..29, log2 height 30..33); a new texture (numbers from 0x3f00 name a
/// monster's texture) must be loaded; each group keeps the largest
/// vertex (12 per primitive `+2` times `scale`) and index (6) counts;
/// then the instance (`flags` | 8, | 0x80000 for YIQ on map 0x188) is
/// created with one data buffer per group (indices times 3) and every
/// render entry's `+0x24` set to 2.
#[allow(clippy::too_many_lines)] // the grouping, then the creation
pub fn create_shape_field(
    h: &mut impl Host,
    frame: u32,
    lo: u32,
    hi: u32,
    shapes: u32,
    scale: u32,
    flags: u32,
) -> u32 {
    let array = |off: u32| frame + off;
    let (flag_words, numbers, depths, heights, widths, vertices, indices, names) = (
        array(0x2c),
        array(0x1bc),
        array(0x34c),
        array(0x4dc),
        array(0x66c),
        array(0x7fc),
        array(0x98c),
        array(0xb1c),
    );
    for base in [vertices, indices, numbers, depths, widths, heights] {
        h.write(base, 0u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[base + 4, 0, 4 * (SLOTS - 1)]);
    }
    h.write(flag_words, 1u32);
    h.call_sym(MEMSET, Conv::Cdecl, &[flag_words + 4, 0, 4 * (SLOTS - 1)]);
    let pointer: u32 = h.read(shapes);
    let shape = h.call_sym(GET_PTR32, Conv::Cdecl, &[pointer]);
    let mut groups = 0u32;
    let mut i = 0i32;
    while i < i32::from(h.read::<i16>(shape.wrapping_add(6))) {
        let entry = shape.wrapping_add(0x10 + 8 * i as u32);
        let sub = (i32::from(h.read::<i16>(entry)) as u32).wrapping_add(shape);
        let size_lo: u32 = h.read(sub.wrapping_add(0x10));
        let size_hi: u32 = h.read(sub.wrapping_add(0x14));
        let bits = (u64::from(size_hi) << 32).wrapping_add(i64::from(size_lo as i32) as u64);
        let depth = (bits >> 20) as u32 & 0x3f;
        let width = 1u32 << ((bits >> 26) as u32 & 0xf);
        let height = 1u32 << ((bits >> 30) as u32 & 0xf);
        let number = bits as u32 & 0x3fff;
        let word = |base: u32, c: u32| base + 4 * c;
        let found = (0..groups).find(|&c| {
            h.read::<u32>(word(numbers, c)) == number
                && h.read::<u32>(word(depths, c)) == depth
                && h.read::<u32>(word(widths, c)) == width
                && h.read::<u32>(word(heights, c)) == height
        });
        if let Some(c) = found {
            let v = primitives(h, sub, scale).wrapping_mul(12);
            if h.read::<i32>(word(vertices, c)) < v {
                h.write(word(vertices, c), v);
            }
            let x = primitives(h, sub, scale).wrapping_mul(6);
            if h.read::<i32>(word(indices, c)) < x {
                h.write(word(indices, c), x);
            }
        } else {
            let name = names + 0x100 * groups;
            let mut copied = false;
            if number >= 0x3f00 {
                let chr = h.call_sym(CHR_FROM_CONTENT, Conv::Cdecl, &[shape]);
                if chr == u32::MAX {
                    return 0;
                }
                if chr != 0 {
                    let text = h.call_sym(MONSTER_TEXTURE_NAME, Conv::Cdecl, &[chr, number]);
                    if text != 0 {
                        // the inlined strcpy
                        let mut k = 0u32;
                        loop {
                            let b: u8 = h.read(text.wrapping_add(k));
                            h.write(name.wrapping_add(k), b);
                            if b == 0 {
                                break;
                            }
                            k = k.wrapping_add(1);
                        }
                        copied = true;
                    }
                }
            }
            if !copied {
                let loaded = h.call_sym(
                    IS_TEXTURE_WITH_BP_LOADED,
                    Conv::Cdecl,
                    &[number, depth, 0, 0, width, height, name],
                );
                if loaded == 0 {
                    let text = h.rebase(SHAPE_NO_TEXTURE);
                    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[text, number]);
                    return 0;
                }
            }
            let g = groups;
            h.write(word(widths, g), width);
            h.write(word(heights, g), height);
            h.write(word(numbers, g), number);
            let v = primitives(h, sub, scale).wrapping_mul(12);
            h.write(word(vertices, g), v);
            h.write(word(depths, g), depth);
            let x = primitives(h, sub, scale).wrapping_mul(6);
            h.write(word(indices, g), x);
            groups += 1;
        }
        i += 1;
    }
    let mut flags = flags | 8;
    if h.read::<u32>(h.rebase(YIQ_MODE)) != u32::MAX
        && h.read::<u32>(h.rebase(CURRENT_MAP)) == 0x188
    {
        flags |= 0x8_0000;
    }
    for c in 0..groups {
        let at = indices + 4 * c;
        let n: u32 = h.read(at);
        h.write(at, n.wrapping_mul(3));
    }
    h.call_sym(VFX_CREATE, Conv::Cdecl, &[lo, hi, flags]);
    h.call_sym(
        VFX_CREATE_DATA_BUFFER,
        Conv::Cdecl,
        &[lo, hi, groups, vertices, indices, names, flag_words],
    );
    let data = h.call_sym(GET_RENDER_DATA, Conv::Cdecl, &[lo, hi]);
    if data == 0 {
        return 0;
    }
    let mut k = 0i32;
    while k < h.read::<i32>(data.wrapping_add(0x90)) {
        let entries: u32 = h.read(data.wrapping_add(0x94));
        h.write(entries.wrapping_add(0x6c * k as u32 + 0x24), 2u32);
        k += 1;
    }
    1
}
