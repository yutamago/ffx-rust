//! The shadow-map cascades' caster rectangles (M4 coverage 2, list
//! `c2-new`): what `0x006c9e50` (the cascade pass `PhyreScene_RenderShadow`
//! runs for shadow solutions 2-4) calls with `Shadow` on. Only the
//! `settings-shadow` replay reaches them.
//!
//! The `ShadowManager` (`g_ShadowManager`, 0x4068 bytes) keeps an 8 x 8
//! grid of caster cells: a list per cell at `+0xde8` (`{count | flag bit
//! 31, nodes}`), two "use it" words per cell at `+0xfe8` and `+0x10e8`
//! (1: in), one byte per cell and cascade at `+0x2b28` (cascade `i` at
//! `+0x40 i`). The cascade's view-projection matrices are at `+0x210 +
//! 0x40 i`. Each cascade's caster rectangles (shadow-map texels, 16 bytes:
//! `x0, y0, x1, y1`, up to 64) are at `+0x2fa8 + 0x400 i`, their count at
//! `+0x37a8 + 4 i`.
//!
//! The x87 code is followed as the originals run it: products and sums at
//! 53 bits, rounded to `f32` where they store, `fld`/`fstp` copies through
//! [`x87::round_trip`], comparisons with their unordered outcome.

use ffx_bridge::x87;

/// `PMatrix4_FromMatrix3x4(this; m)`, `PMatrix4_Multiply(out, a, b)`
/// (cdecl), `PMatrix4_Copy(this; m)` (two copies), `PObject_InitDefaults
/// (this)`, `PMatrix4_MulThis(this; out, b)`,
/// `PBounds_forward(this; min, max, matrix)` and `Vec3Min`/`Vec3Max(out, a,
/// b)` (cdecl).
pub const FROM_3X4: u32 = 0x0045_1080;
pub const MULTIPLY: u32 = 0x004c_a360;
pub const COPY_4C9430: u32 = 0x004c_9430;
pub const COPY_453340: u32 = 0x0045_3340;
pub const INIT_DEFAULTS: u32 = 0x0045_30a0;
pub const MUL_THIS: u32 = 0x004c_9da0;
pub const BOUNDS_FORWARD: u32 = 0x0050_08c0;
pub const VEC3_MIN: u32 = 0x004f_8580;
pub const VEC3_MAX: u32 = 0x004f_84f0;
/// `3.0` (`f64`), the margin around a caster's rectangle.
pub const THREE: u32 = 0x00b2_c5c8;
/// The ports' own callees: [`bridge::rect_union`], [`bridge::align_rects`],
/// [`bridge::copy_records`], [`bridge::append_record`].
pub const RECT_UNION: u32 = 0x006d_2b50;
pub const ALIGN_RECTS: u32 = 0x006d_1800;
pub const COPY_RECORDS: u32 = 0x006c_7190;
pub const APPEND_RECORD: u32 = 0x006d_2760;
/// `Phyre_PhyreMalloc(size, align)` and `Phyre_PhyreFree(p)` (cdecl).
pub const MALLOC: u32 = 0x0042_fb80;
pub const FREE: u32 = 0x0042_fb20;
/// `GetASpareGradMeshInstance(this; &extra)` and the two gradient set-ups
/// `0x00678e60` / `0x006790c0` (`this` the mesh's material; scale, kind,
/// a, b).
pub const GET_SPARE_GRAD_MESH: u32 = 0x006c_c980;
pub const GRAD_SETUP_A: u32 = 0x0067_8e60;
pub const GRAD_SETUP_B: u32 = 0x0067_90c0;

/// The caster rectangles of cascade `i`: the first word of rectangle `k`.
#[must_use]
pub const fn rect(this: u32, i: u32, k: u32) -> u32 {
    this.wrapping_add(0x2fa8)
        .wrapping_add(i.wrapping_mul(0x400))
        .wrapping_add(k.wrapping_mul(0x10))
}

/// The rectangle count of cascade `i`.
#[must_use]
pub const fn count_at(this: u32, i: u32) -> u32 {
    this.wrapping_add(0x37a8).wrapping_add(i.wrapping_mul(4))
}

/// An x87 comparison of `a` with `b` (`fcom`): `None` when unordered.
fn cmp(a: f64, b: f64) -> Option<std::cmp::Ordering> {
    a.partial_cmp(&b)
}

/// `fild` of an integer, then `fstp dword` / `fld dword`: the integer as
/// the nearest `f32`.
#[allow(clippy::cast_precision_loss)] // the store rounds to f32
fn int_f32(v: u32) -> f64 {
    f64::from(v.cast_signed() as f32)
}

pub mod bridge {
    //! The originals over game memory.

    use std::cmp::Ordering::{Equal, Greater, Less};

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn r32(h: &impl Host, at: u32) -> u32 {
        h.read(at)
    }

    /// The `f64` 3.0 at [`THREE`].
    fn three(h: &impl Host) -> f64 {
        let at = h.rebase(THREE);
        let lo = u64::from(r32(h, at));
        let hi = u64::from(r32(h, at + 4));
        f64::from_bits(hi << 32 | lo)
    }

    /// A float word as `fld dword` loads it.
    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `FUN_006c7190(dst, src, n)` (cdecl): `n` 16-byte records from `src`
    /// to `dst`, one after the other; a null `dst` skips the copy but not
    /// the step (`src` stays). Returns nothing the caller reads.
    pub fn copy_records(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let (mut d, mut s) = (dst, src);
        for _ in 0..n {
            if d != 0 {
                for w in 0..4 {
                    let v = r32(h, s + 4 * w);
                    h.write(d + 4 * w, v);
                }
                s = s.wrapping_add(0x10);
            }
            d = d.wrapping_add(0x10);
        }
    }

    /// `FUN_006d2760(this; storage, &record)` (`ret 8`): the `PArray` of
    /// 16-byte records `this` (`{count | bit 31: storage not owned, data}`)
    /// moves into `storage` with `record` after its elements; the old data
    /// is freed when owned and not `storage`. 0xd for a null storage, else
    /// 0.
    pub fn append_record(h: &mut impl Host, this: u32, storage: u32, record: u32) -> u32 {
        if storage == 0 {
            return 0xd;
        }
        let n = r32(h, this) & 0x7fff_ffff;
        let data = r32(h, this + 4);
        h.call_sym(COPY_RECORDS, Conv::Cdecl, &[storage, data, n]);
        let end = (n << 4).wrapping_add(storage);
        if end != 0 {
            for w in 0..4 {
                let v = r32(h, record + 4 * w);
                h.write(end + 4 * w, v);
            }
        }
        let old = r32(h, this + 4);
        let word = r32(h, this);
        if old != storage && word >> 31 == 0 && old != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, storage);
        h.write(this, n + 1);
        0
    }

    /// `FUN_006d17d0(this; &record)` (`ret 4`): [`append_record`] into a
    /// new block of `count + 1` records (`Phyre_PhyreMalloc`, align 4);
    /// returns its `eax`.
    pub fn push_record(h: &mut impl Host, this: u32, record: u32) -> u32 {
        let n = r32(h, this) & 0x7fff_ffff;
        let size = n.wrapping_add(1) << 4;
        let storage = h.call_sym(MALLOC, Conv::Cdecl, &[size, 4]);
        h.call_sym(APPEND_RECORD, Conv::Thiscall(this), &[storage, record])
    }

    /// `FUN_006d2b50(a0, a1, a2, a3, b0, b1, b2, b3, &o0, &o1, &o2, &o3)`
    /// (`ret 0x30`; `ecx` unread): two rectangles given as `x0, x1, y0, y1`.
    /// Writes their intersection (`max x0, min x1, max y0, min y1`); when
    /// it is not empty (read back through the pointers), writes their
    /// bounding box instead and returns `al` 1, else `al` 0. The rest of
    /// `eax` is what the original leaves there.
    #[allow(clippy::too_many_arguments)] // the original's twelve words
    pub fn rect_union(h: &mut impl Host, a: [u32; 4], b: [u32; 4], out: [u32; 4]) -> u32 {
        let i = |v: u32| v.cast_signed();
        let [a0, a1, a2, a3] = a;
        let [b0, b1, b2, b3] = b;
        let [o0, o1, o2, o3] = out;
        h.write(o0, if i(b0) > i(a0) { b0 } else { a0 });
        h.write(o2, if i(b2) > i(a2) { b2 } else { a2 });
        h.write(o1, if i(b1) < i(a1) { b1 } else { a1 });
        let y1 = if i(b3) < i(a3) { b3 } else { a3 };
        h.write(o3, y1);
        let x0 = r32(h, o0);
        if i(x0) > i(r32(h, o1)) {
            return x0 & !0xff;
        }
        if i(r32(h, o2)) > i(y1) {
            return y1 & !0xff;
        }
        h.write(o0, if i(b0) < i(a0) { b0 } else { a0 });
        h.write(o2, if i(b2) < i(a2) { b2 } else { a2 });
        h.write(o1, if i(b1) > i(a1) { b1 } else { a1 });
        h.write(o3, if i(b3) > i(a3) { b3 } else { a3 });
        (o3 & !0xff) | 1
    }

    /// `FUN_006d1800(this; i)` (`ret 4`): cascade `i`'s rectangles to
    /// 32-texel blocks: `x0`, `y0` toward zero, `x1`, `y1` to the next
    /// multiple above their quotient toward zero when not one already.
    pub fn align_rects(h: &mut impl Host, this: u32, i: u32) {
        let down = |v: u32| {
            let v = v.cast_signed();
            (v.wrapping_add((v >> 31) & 0x1f) & !0x1f).cast_unsigned()
        };
        let up = |v: u32| {
            let v = v.cast_signed();
            let q = v.wrapping_add((v >> 31) & 0x1f) >> 5;
            let r = v.wrapping_rem(32);
            (q.wrapping_add(i32::from(r != 0)) << 5).cast_unsigned()
        };
        let count = count_at(this, i);
        let mut k = 0u32;
        while k.cast_signed() < r32(h, count).cast_signed() {
            let e = rect(this, i, k);
            for (w, round) in [(0, down as fn(u32) -> u32), (4, down), (8, up), (12, up)] {
                let v = r32(h, e + w);
                h.write(e + w, round(v));
            }
            k += 1;
        }
    }

    /// The x0/y0/x1/y1 of one caster box: `min * scale - 3` and `max *
    /// scale + 3` toward zero (`ftol2_sse`).
    fn texels(h: &impl Host, min: f64, max: f64, scale: f64, three: f64) -> (u32, u32) {
        let lo = x87::ftol(h, x87::sub(x87::mul(min, scale), three));
        let hi = x87::ftol(h, x87::add(x87::mul(max, scale), three));
        (lo, hi)
    }

    /// Clamps every rectangle of cascade `i` to `0..=lim` (x) and
    /// `0..=lim_y` (y), signed.
    fn clamp_rects(h: &mut impl Host, this: u32, i: u32, lim_x: u32, lim_y: u32) {
        let clamp = |v: u32, lim: u32| {
            let v = if v.cast_signed() < lim.cast_signed() {
                v
            } else {
                lim
            };
            if v.cast_signed() < 0 { 0 } else { v }
        };
        let count = count_at(this, i);
        let mut k = 0u32;
        loop {
            let e = rect(this, i, k);
            for (w, lim) in [(0, lim_x), (4, lim_y), (8, lim_x), (12, lim_y)] {
                let v = r32(h, e + w);
                h.write(e + w, clamp(v, lim));
            }
            k += 1;
            if k.cast_signed() >= r32(h, count).cast_signed() {
                break;
            }
        }
    }

    /// Merges overlapping rectangles of cascade `i` until none overlap:
    /// each pair through `FUN_006d2b50` (outputs in the frame at
    /// `outs`), the first rectangle becomes the box (grown by `grow` on
    /// every side), the second leaves the list, and the scan starts over.
    fn merge_rects(h: &mut impl Host, this: u32, i: u32, outs: [u32; 4], grow: u32) {
        let count = count_at(this, i);
        'restart: loop {
            let n = r32(h, count);
            let mut a = 0u32;
            while (a + 1).cast_signed() < n.cast_signed() {
                let ea = rect(this, i, a);
                let [a0, a1, a2, a3] = [0, 4, 8, 12].map(|w| r32(h, ea + w));
                let mut b = a + 1;
                while b.cast_signed() < n.cast_signed() {
                    let eb = rect(this, i, b);
                    let [b0, b1, b2, b3] = [0, 4, 8, 12].map(|w| r32(h, eb + w));
                    let mut args = [a0, a2, a1, a3, b0, b2, b1, b3, 0, 0, 0, 0];
                    args[8..].copy_from_slice(&outs);
                    let al = h.call_sym(RECT_UNION, Conv::Thiscall(this), &args);
                    if al & 0xff != 0 {
                        let [x0, x1, y0, y1] = outs.map(|o| r32(h, o));
                        h.write(ea, x0.wrapping_sub(grow));
                        h.write(ea + 8, x1.wrapping_add(grow));
                        h.write(ea + 4, y0.wrapping_sub(grow));
                        h.write(ea + 12, y1.wrapping_add(grow));
                        let mut k = b;
                        while k.cast_signed() < r32(h, count).cast_signed() {
                            let e = rect(this, i, k);
                            for w in [0, 4, 8, 12] {
                                let v = r32(h, e + 0x10 + w);
                                h.write(e + w, v);
                            }
                            k += 1;
                        }
                        let c = r32(h, count);
                        h.write(count, c.wrapping_sub(1));
                        continue 'restart;
                    }
                    b += 1;
                }
                a += 1;
            }
            return;
        }
    }

    /// The map's size in texels: the scales toward zero (`ftol2_sse`).
    fn limits(h: &impl Host, sx: u32, sy: u32) -> (u32, u32) {
        (x87::ftol(h, x87::load(sx)), x87::ftol(h, x87::load(sy)))
    }

    /// Whether each cascade pair's id lists hold the same ids
    /// ([`sync_cascades`]).
    fn same_ids(h: &impl Host, this: u32) -> bool {
        let mask = 0x7fff_ffff;
        for pair in 0..2u32 {
            let a = this + 0x290 + 8 * pair;
            let b = a + 0x10;
            if (r32(h, a) ^ r32(h, b)) & mask != 0 {
                return false;
            }
        }
        for pair in 0..2u32 {
            let a = this + 0x290 + 8 * pair;
            let b = a + 0x10;
            let m = r32(h, a) & mask;
            let n = r32(h, b) & mask;
            for k in 0..n {
                if m == 0 {
                    return false;
                }
                let id = r32(h, r32(h, b + 4).wrapping_add(4 * k));
                let ids = r32(h, a + 4);
                if !(0..m).any(|j| r32(h, ids.wrapping_add(4 * j)) == id) {
                    return false;
                }
            }
        }
        true
    }

    /// A caster list's node count (its first word without the flag bit).
    fn list_len(h: &impl Host, list: u32) -> u32 {
        r32(h, list) & 0x7fff_ffff
    }

    /// The node matrix and bounds of a caster list's entry `k`: the
    /// node's object (`+0x1c`) and its 3x4 matrix (`+0xc`).
    fn node_object(h: &impl Host, list: u32, k: u32) -> u32 {
        let nodes = r32(h, list + 4);
        let node = r32(h, nodes.wrapping_add(4 * k));
        r32(h, node + 0x1c)
    }

    /// `FUN_006c88a0(this; i, scale_x, scale_y)` (`ret 0xc`, frame at
    /// `ebp`): cascade `i`'s caster rectangles. Every cell in use whose
    /// cascade byte is set and whose list has nodes gives one rectangle:
    /// the union of its nodes' boxes (`PBounds_forward` through the
    /// cascade matrix times the node's), scaled to texels with a margin of
    /// 3. Then they are clamped to the map, merged where they overlap
    /// (grown by 3 each time), clamped again, aligned to 32 texels
    /// (`FUN_006d1800`) and merged again.
    pub fn build_rects(h: &mut impl Host, ebp: u32, this: u32, i: u32, sx: u32, sy: u32) {
        let (min_a, max_a) = (ebp - 0x4c, ebp - 0x3c);
        let (min_b, max_b) = (ebp - 0x5c, ebp - 0x6c);
        let (node_a, node_b) = (ebp - 0xbc, ebp - 0xfc);
        let product = ebp - 0x14c;
        let count = count_at(this, i);
        let three = three(h);
        h.write(count, 0u32);
        let cascade = this.wrapping_add(i.wrapping_mul(0x40)).wrapping_add(0x210);
        for cell in 0..64u32 {
            let list = r32(h, this + 0xde8 + 4 * cell);
            if list == 0
                || r32(h, this + 0xfe8 + 4 * cell) != 1
                || r32(h, this + 0x10e8 + 4 * cell) != 1
                || h.read::<u8>(this + 0x2b28 + cell + i.wrapping_mul(0x40)) == 0
                || list_len(h, list) == 0
            {
                continue;
            }
            let object = node_object(h, list, 0);
            let m = r32(h, object + 0xc);
            h.call_sym(FROM_3X4, Conv::Thiscall(node_a), &[m]);
            let p = h.call_sym(MULTIPLY, Conv::Cdecl, &[product, cascade, node_a]);
            h.call_sym(COPY_4C9430, Conv::Thiscall(node_b), &[p]);
            let object = node_object(h, list, 0);
            h.call_sym(
                BOUNDS_FORWARD,
                Conv::Thiscall(object),
                &[min_a, max_a, node_b],
            );
            let mut k = 1u32;
            while k < list_len(h, list) {
                let object = node_object(h, list, k);
                let m = r32(h, object + 0xc);
                h.call_sym(FROM_3X4, Conv::Thiscall(node_b), &[m]);
                let p = h.call_sym(MULTIPLY, Conv::Cdecl, &[product, cascade, node_b]);
                h.call_sym(COPY_4C9430, Conv::Thiscall(node_a), &[p]);
                let object = node_object(h, list, k);
                h.call_sym(
                    BOUNDS_FORWARD,
                    Conv::Thiscall(object),
                    &[min_b, max_b, node_a],
                );
                let lo = h.call_sym(VEC3_MIN, Conv::Cdecl, &[ebp - 0x7c, min_a, min_b]);
                for w in [0, 4, 8] {
                    let v = x87::store(f(h, lo + w));
                    h.write(min_a + w, v);
                }
                let hi = h.call_sym(VEC3_MAX, Conv::Cdecl, &[ebp - 0x10c, max_a, max_b]);
                for w in [0, 4, 8] {
                    let v = x87::store(f(h, hi + w));
                    h.write(max_a + w, v);
                }
                k += 1;
            }
            let (fx, fy) = (x87::load(sx), x87::load(sy));
            let (x0, x1) = texels(h, f(h, min_a), f(h, max_a), fx, three);
            let (y0, y1) = texels(h, f(h, min_a + 4), f(h, max_a + 4), fy, three);
            let n = r32(h, count);
            let e = rect(this, i, n);
            h.write(e, x0);
            h.write(e + 4, y0);
            h.write(e + 8, x1);
            h.write(e + 12, y1);
            h.write(count, n.wrapping_add(1));
        }
        if r32(h, count).cast_signed() > 0 {
            let (lx, ly) = limits(h, sx, sy);
            clamp_rects(h, this, i, lx, ly);
        }
        merge_rects(
            h,
            this,
            i,
            [ebp - 0x28, ebp - 0x1c, ebp - 0x18, ebp - 0x2c],
            3,
        );
        if r32(h, count).cast_signed() > 0 {
            let (lx, ly) = limits(h, sx, sy);
            clamp_rects(h, this, i, lx, ly);
        }
        h.call_sym(ALIGN_RECTS, Conv::Thiscall(this), &[i]);
        merge_rects(h, this, i, [ebp - 0x24, ebp - 0x10, ebp - 4, ebp - 8], 0);
    }

    /// `FUN_006c8f50(this; view matrix by value, flag, ratio, scale_x,
    /// scale_y)` (`ret 0x50`; `args` the address of its 20 stack words,
    /// frame at `ebp`) -> `al`: whether a caster of cascade 0 or 1 (its
    /// box through `view` times the node matrix, `PMatrix4_MulThis`) lands
    /// across the map's edge: its texel rectangle (as
    /// [`build_rects`]'s) reaches below `ratio * scale_y` from above `scale_y`
    /// in y, or with `flag` crosses 0 or `scale_x` in x or 0 in y.
    #[allow(clippy::too_many_lines)] // one loop, as the original
    pub fn caster_crosses_edge(h: &mut impl Host, ebp: u32, this: u32, args: u32) -> u8 {
        let view = args;
        let flag: u8 = h.read(args + 0x40);
        let (ratio, sx, sy) = (args + 0x44, args + 0x48, args + 0x4c);
        let (min_a, max_a) = (ebp - 0x3c, ebp - 0x4c);
        let (min_b, max_b) = (ebp - 0x6c, ebp - 0x7c);
        let (node_a, node_b) = (ebp - 0xfc, ebp - 0xbc);
        let (product_a, product_b) = (ebp - 0x13c, ebp - 0x17c);
        let three = three(h);
        for cell in 0..64u32 {
            let (row, col) = (cell / 8, cell % 8);
            let list = r32(h, this + 0xde8 + 4 * cell);
            if list == 0
                || r32(h, this + 0xfe8 + 4 * cell) != 1
                || r32(h, this + 0x10e8 + 4 * cell) != 1
            {
                continue;
            }
            let byte = this + 0x2b68 + row * 8 + col;
            if h.read::<u8>(byte - 0x40) == 0 && h.read::<u8>(byte) == 0 {
                continue;
            }
            if list_len(h, list) == 0 {
                continue;
            }
            let object = node_object(h, list, 0);
            let m = r32(h, object + 0xc);
            h.call_sym(FROM_3X4, Conv::Thiscall(node_a), &[m]);
            let object = node_object(h, list, 0);
            let p = h.call_sym(MUL_THIS, Conv::Thiscall(view), &[product_a, node_a]);
            h.call_sym(BOUNDS_FORWARD, Conv::Thiscall(object), &[min_a, max_a, p]);
            let mut k = 1u32;
            while k < list_len(h, list) {
                let object = node_object(h, list, k);
                let m = r32(h, object + 0xc);
                h.call_sym(FROM_3X4, Conv::Thiscall(node_b), &[m]);
                let object = node_object(h, list, k);
                let p = h.call_sym(MUL_THIS, Conv::Thiscall(view), &[product_b, node_b]);
                h.call_sym(BOUNDS_FORWARD, Conv::Thiscall(object), &[min_b, max_b, p]);
                // the new value when it is not above (min) / below (max) the
                // kept one, or unordered
                for w in [8, 4, 0] {
                    let (kept, new) = (f(h, min_a + w), f(h, min_b + w));
                    let v = if matches!(cmp(new, kept), Some(Greater)) {
                        kept
                    } else {
                        new
                    };
                    h.write(min_a + w, x87::store(v));
                }
                for w in [8, 4, 0] {
                    let (kept, new) = (f(h, max_a + w), f(h, max_b + w));
                    let v = if matches!(cmp(new, kept), Some(Less)) {
                        kept
                    } else {
                        new
                    };
                    h.write(max_a + w, x87::store(v));
                }
                k += 1;
            }
            let (fx, fy) = (f(h, sx), f(h, sy));
            let (x0, x1) = texels(h, f(h, min_a), f(h, max_a), fx, three);
            let (y0, y1) = texels(h, f(h, min_a + 4), f(h, max_a + 4), fy, three);
            let edge = x87::mul(f(h, ratio), fy);
            // fc * fy below y1 and y0 not above fy
            if matches!(cmp(edge, int_f32(y1)), Some(Less))
                && matches!(cmp(int_f32(y0), fy), Some(Less | Equal))
            {
                return 1;
            }
            if flag == 0 {
                continue;
            }
            if x0.cast_signed() < 0 && x1.cast_signed() >= 0 {
                return 1;
            }
            if matches!(cmp(int_f32(x1), fx), Some(Greater))
                && matches!(cmp(int_f32(x0), fx), Some(Less | Equal))
            {
                return 1;
            }
            if y0.cast_signed() < 0 && y1.cast_signed() >= 0 {
                return 1;
            }
        }
        0
    }

    /// `FUN_006c9780(this)`: when the two caster-id lists of each cascade
    /// pair hold the same ids (`+0x290`/`+0x2a0`, `+0x298`/`+0x2a8`: equal
    /// counts, every id of the second found in the first), the saved 128
    /// words at `+0x2b0` go back to `+0x90`; else `+0x90` is saved there.
    /// Copies through the x87 (`fld`/`fstp`).
    pub fn sync_cascades(h: &mut impl Host, this: u32) {
        let (from, to) = if same_ids(h, this) {
            (this + 0x2b0, this + 0x90)
        } else {
            (this + 0x90, this + 0x2b0)
        };
        for w in 0..128u32 {
            let v: f32 = h.read(from + 4 * w);
            h.write(to + 4 * w, x87::round_trip(v));
        }
    }

    /// `FUN_006c7cc0(this; m3x4, mat, a, b)` (`ret 0x10`, frame at `ebp`)
    /// -> `this`: a cascade's view record: `PObject_InitDefaults`, the 3x4
    /// matrix copied (x87) to `+0`, `mat` to `+0x30`, `mat` times the 3x4
    /// (as 4x4) to `+0x70`, `a`, `b` to `+0xb4`, `+0xb8` and 0 to `+0xb0`.
    pub fn view_record_init(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        m: u32,
        mat: u32,
        a: f32,
        b: f32,
    ) -> u32 {
        h.call_sym(INIT_DEFAULTS, Conv::Thiscall(this), &[]);
        for w in 0..12u32 {
            let v: f32 = h.read(m + 4 * w);
            h.write(this + 4 * w, x87::round_trip(v));
        }
        h.call_sym(COPY_453340, Conv::Thiscall(this + 0x30), &[mat]);
        h.call_sym(FROM_3X4, Conv::Thiscall(ebp - 0x40), &[m]);
        let p = h.call_sym(
            MULTIPLY,
            Conv::Cdecl,
            &[ebp - 0x80, this + 0x30, ebp - 0x40],
        );
        h.call_sym(COPY_453340, Conv::Thiscall(this + 0x70), &[p]);
        h.write(this + 0xb4, x87::round_trip(a));
        h.write(this + 0xb8, x87::round_trip(b));
        h.write(this + 0xb0, 0u32);
        this
    }

    /// The gradient values of one rectangle, as the two originals below
    /// compute them: `kind` 1-3 (0: none, `a`/`b` kept), `a`, `b` as
    /// `f32` bits.
    struct Grad {
        kind: u32,
        a: u32,
        b: u32,
    }

    /// `v / d` rounded to `f32`, then to `0.0..=1.0` (`x >= 1` or NaN: 1;
    /// below 0: 0).
    fn unit(v: f64, d: f64) -> u32 {
        let t = x87::store(x87::div(v, d));
        let t = if matches!(cmp(x87::load(t), 1.0), Some(Less)) {
            t
        } else {
            1f32.to_bits()
        };
        if matches!(cmp(x87::load(t), 0.0), Some(Less)) {
            0
        } else {
            t
        }
    }

    /// `FUN_006d12f0(this; i, meshes, extras, scale, unused, depth)` (`ret
    /// 0x18`): one gradient mesh per rectangle of cascade `i` reaching
    /// `scale * depth` in `y1` (`meshes[k]`, a spare one
    /// (`GetASpareGradMeshInstance(&extras[k])`) when it holds none or the
    /// default `+0x8b8`), set up by `0x00678e60(extras[k]; scale, kind, a,
    /// b)` with `y0`, `y1` over `depth` (kind 1: `y0` below the edge,
    /// clamped to 0..1; 2: `y0` from it and `y1` up to `depth`; 3: both
    /// within `depth`'s bounds), its `+0x10` the manager's `+0xde4` or its
    /// own first word + 0x30. A rectangle below gets the default mesh.
    /// Kind 0 passes the previous `a`, `b` (first: `meshes` and `i`, the
    /// original's argument slots).
    #[allow(clippy::too_many_arguments)] // the original's six words
    pub fn grad_meshes_a(
        h: &mut impl Host,
        this: u32,
        i: u32,
        meshes: u32,
        extras: u32,
        scale: u32,
        _unused: u32,
        depth: u32,
    ) {
        let count = count_at(this, i);
        let d = x87::load(depth);
        let s = x87::load(scale);
        let edge = x87::mul(s, d);
        let (mut a, mut b) = (meshes, i);
        let mut k = 0u32;
        while k.cast_signed() < r32(h, count).cast_signed() {
            let e = rect(this, i, k);
            let slot = meshes.wrapping_add(4 * k);
            let v = int_f32(r32(h, e + 12));
            if matches!(cmp(v, edge), Some(Less)) {
                let default = r32(h, this + 0x8b8);
                h.write(slot, default);
                k += 1;
                continue;
            }
            let u = int_f32(r32(h, e + 4));
            let g = grad_a(u, v, edge, d);
            if g.kind != 0 {
                (a, b) = (g.a, g.b);
            }
            set_mesh(
                h,
                this,
                slot,
                extras.wrapping_add(4 * k),
                &[scale, g.kind, a, b],
                GRAD_SETUP_A,
            );
            k += 1;
        }
    }

    /// The kinds of [`grad_meshes_a`] (`y0` = `u`, `y1` = `v`, `v` not
    /// below `edge`); kind 0 leaves `a`, `b` to the caller.
    fn grad_a(u: f64, v: f64, edge: f64, d: f64) -> Grad {
        let none = Grad {
            kind: 0,
            a: 0,
            b: 0,
        };
        if matches!(cmp(u, edge), Some(Less)) && !matches!(cmp(v, edge), Some(Less) | None) {
            return Grad {
                kind: 1,
                a: unit(u, d),
                b: unit(v, d),
            };
        }
        if !matches!(cmp(u, edge), Some(Less) | None) {
            if !matches!(cmp(v, d), Some(Greater) | None) {
                return Grad {
                    kind: 2,
                    a: x87::store(x87::div(u, d)),
                    b: x87::store(x87::div(v, d)),
                };
            }
            if matches!(cmp(u, d), Some(Less)) && matches!(cmp(v, d), Some(Greater)) {
                return Grad {
                    kind: 3,
                    a: x87::store(x87::div(u, d)),
                    b: x87::store(x87::div(v, d)),
                };
            }
        }
        none
    }

    /// The mesh of one rectangle: `slot`'s, or a spare one when it holds
    /// none or the default; `setup(extras[k]; scale, kind, a, b)` with
    /// the floats through the x87; `+0x10` and `slot` set.
    fn set_mesh(h: &mut impl Host, this: u32, slot: u32, extra: u32, words: &[u32; 4], setup: u32) {
        let mut mesh = r32(h, slot);
        if mesh == 0 || mesh == r32(h, this + 0x8b8) {
            mesh = h.call_sym(GET_SPARE_GRAD_MESH, Conv::Thiscall(this), &[extra]);
        }
        let [scale, kind, a, b] = *words;
        let rt = |bits: u32| x87::store(x87::load(bits));
        let material = r32(h, extra);
        h.call_sym(
            setup,
            Conv::Thiscall(material),
            &[rt(scale), kind, rt(a), rt(b)],
        );
        let mut v = r32(h, this + 0xde4);
        if v == 0 {
            v = r32(h, mesh).wrapping_add(0x30);
        }
        h.write(mesh + 0x10, v);
        h.write(slot, mesh);
    }

    /// `FUN_006d1530(this; i, meshes, extras, scale, unused, depth)` (`ret
    /// 0x18`): [`grad_meshes_a`] for the other gradient
    /// (`0x006790c0`): rectangles whose `y0` is above `scale * depth` keep
    /// their mesh (a null one gets the default); kind 1: `y0` below the
    /// edge and `y1` from it (clamped); 2: `y0` at the edge (or unordered)
    /// or `y1` below it, `y0` not negative and `y1` within; 3: `y0`
    /// negative, `y1` positive and within. Kind 0 passes the previous `a`
    /// (first: 0, a never-written frame word) and `b` (first: `meshes`).
    #[allow(clippy::too_many_arguments)] // the original's six words
    pub fn grad_meshes_b(
        h: &mut impl Host,
        this: u32,
        i: u32,
        meshes: u32,
        extras: u32,
        scale: u32,
        _unused: u32,
        depth: u32,
    ) {
        let count = count_at(this, i);
        let d = x87::load(depth);
        let edge = x87::mul(x87::load(scale), d);
        // junk read: [ebp-0x8] never written before a kind-0 rectangle; 0
        // (brief 2026-10-24)
        let (mut a, mut b) = (0u32, meshes);
        let mut k = 0u32;
        while k.cast_signed() < r32(h, count).cast_signed() {
            let e = rect(this, i, k);
            let slot = meshes.wrapping_add(4 * k);
            let y0 = r32(h, e + 4);
            let u = int_f32(y0);
            if matches!(cmp(u, edge), Some(Greater)) {
                if r32(h, slot) == 0 {
                    let default = r32(h, this + 0x8b8);
                    h.write(slot, default);
                }
                k += 1;
                continue;
            }
            let y1 = r32(h, e + 12);
            let g = grad_b(y0, y1, edge, d);
            if g.kind != 0 {
                (a, b) = (g.a, g.b);
            }
            set_mesh(
                h,
                this,
                slot,
                extras.wrapping_add(4 * k),
                &[scale, g.kind, a, b],
                GRAD_SETUP_B,
            );
            k += 1;
        }
    }

    /// The kinds of [`grad_meshes_b`] (`y0` not above `edge`).
    fn grad_b(y0: u32, y1: u32, edge: f64, d: f64) -> Grad {
        let (u, v) = (int_f32(y0), int_f32(y1));
        let none = Grad {
            kind: 0,
            a: 0,
            b: 0,
        };
        if matches!(cmp(u, edge), Some(Less)) && !matches!(cmp(v, edge), Some(Less) | None) {
            return Grad {
                kind: 1,
                a: unit(u, d),
                b: unit(v, d),
            };
        }
        if y0.cast_signed() >= 0 && matches!(cmp(v, edge), Some(Less | Equal)) {
            return Grad {
                kind: 2,
                a: x87::store(x87::div(u, d)),
                b: x87::store(x87::div(v, d)),
            };
        }
        if matches!(cmp(v, edge), Some(Less | Equal))
            && y0.cast_signed() < 0
            && y1.cast_signed() > 0
        {
            return Grad {
                kind: 3,
                a: x87::store(x87::div(u, d)),
                b: x87::store(x87::div(v, d)),
            };
        }
        none
    }
}
