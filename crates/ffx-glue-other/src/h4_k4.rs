//! Continuation round 9 of list `h4-glue-other`, batch 4: the event
//! shapes' VFX render data buffers.

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
