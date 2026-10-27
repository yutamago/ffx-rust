//! List `h4-glue-other`, second agent (h4b), round 3 batch 4: the sphere
//! grid pad list's bar (`AbmapPadDrawListBar_00a462f0`).
//!
//! Floats follow the x87 at 53 bits ([`crate::h4_e1`]): `f64` in the
//! original's order, `f32` at each `fstp dword`; where the original
//! stores a sum and goes on from the unrounded register, so does the
//! port.

use ffx_bridge::Global;
use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::int_f32;
use crate::h4b_j::{konst, konst64};

/// `graphicDrawUIAbmapElement(record, name, kind)` (cdecl).
pub const DRAW_ABMAP_ELEMENT: u32 = 0x0063_e930;
/// `g_AbmapCjkText`: 1 for the Japanese, Korean and Chinese layouts.
pub const ABMAP_CJK: Global<u32> = Global::new(0x01a8_5f74);

/// Bytes of [`list_bar`]'s frame: the quad record (`ebp - 0x19c`, four
/// vertices of x, y, u, v and four colour words) and, `0x98` on, the name
/// buffer the callee fills (`ebp - 0x104`, 0x100 bytes).
pub const BAR_FRAME: u32 = 0x198;
/// The name buffer's offset in the frame.
const NAME: u32 = 0x98;

/// The quad record: vertex `k`'s x, y, u, v at `8k`..`8k + 3`, its
/// colour (r, g, b, a) at `8k + 4`..`8k + 7`.
struct Quad([u32; 32]);

impl Quad {
    /// Sets the x of vertices 0 and 1 (the left edge) and of 2 and 3.
    fn xs(&mut self, left: u32, right: u32) {
        self.0[0] = left;
        self.0[8] = left;
        self.0[16] = right;
        self.0[24] = right;
    }
    /// Sets the y of vertices 0 and 2 (`a`) and of 1 and 3 (`b`).
    fn ys(&mut self, a: u32, b: u32) {
        self.0[1] = a;
        self.0[17] = a;
        self.0[9] = b;
        self.0[25] = b;
    }
    /// Sets the UVs, vertex by vertex.
    fn uvs(&mut self, uv: [[u32; 2]; 4]) {
        for (k, [u, v]) in uv.into_iter().enumerate() {
            self.0[8 * k + 2] = u;
            self.0[8 * k + 3] = v;
        }
    }
    /// Sets every vertex's colour.
    fn colour(&mut self, rgba: [u32; 4]) {
        for k in 0..4 {
            self.0[8 * k + 4..8 * k + 8].copy_from_slice(&rgba);
        }
    }
    /// Writes the record into the frame.
    fn flush(&self, h: &mut impl Host, frame: u32) {
        for (k, &v) in self.0.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // 32 words
            h.write(frame.wrapping_add(4 * k as u32), v);
        }
    }
    /// Writes the record into the frame and draws it
    /// (`graphicDrawUIAbmapElement(record, name, 6)`); its `eax`.
    fn draw(&self, h: &mut impl Host, frame: u32) -> u32 {
        self.flush(h, frame);
        h.call_sym(
            DRAW_ABMAP_ELEMENT,
            Conv::Cdecl,
            &[frame, frame.wrapping_add(NAME), 6],
        )
    }
}

/// The bar's thumb in the non-CJK layout's or the CJK layout's kind:
/// `x = (p3 + add) × 0x00b92220` and that plus `0x00b92248`.
fn thumb_half(h: &mut impl Host, q: &mut Quad, frame: u32, p3: i32, add_: i32) -> u32 {
    let s = mul(
        load(int_f32(p3.wrapping_add(add_))),
        konst64(h, 0x00b9_2220),
    );
    let right = store(add(s, konst64(h, 0x00b9_2248)));
    q.xs(store(s), right);
    thumb_uvs(h, q);
    q.draw(h, frame)
}

/// The thumb's UVs: u `0x00b7b620`, `0x00b66c20`; v `0x00b7b600`,
/// `0x00b66bf0`.
fn thumb_uvs(h: &impl Host, q: &mut Quad) {
    let [g, hh, i, j] = [0x00b7_b620, 0x00b7_b600, 0x00b6_6c20, 0x00b6_6bf0].map(|c| konst(h, c));
    q.uvs([[g, hh], [i, hh], [g, j], [i, j]]);
}

/// `x = s` and `s + 1` from the unrounded `s`, then drawn.
fn thumb_at(h: &mut impl Host, q: &mut Quad, frame: u32, s: f64) -> u32 {
    q.xs(store(s), store(add(s, 1.0)));
    q.draw(h, frame)
}

/// The thumb in three steps (kinds 0x40 and 0x60): at `(p3 - k) ×
/// 0x00b34f90 + base`, `(p3 - k) × 0x00b92220 + base` and `(k - p3) ×
/// 0x00b34f90 + p3`.
fn thumb_steps(h: &mut impl Host, q: &mut Quad, frame: u32, p3: i32, k: i32, base: u32) -> u32 {
    let d = load(int_f32(p3.wrapping_sub(k)));
    let s = add(mul(d, konst64(h, 0x00b3_4f90)), konst64(h, base));
    thumb_uvs(h, q);
    thumb_at(h, q, frame, s);
    let s = add(mul(d, konst64(h, 0x00b9_2220)), konst64(h, base));
    thumb_at(h, q, frame, s);
    let s = add(
        mul(load(int_f32(k.wrapping_sub(p3))), konst64(h, 0x00b3_4f90)),
        load(int_f32(p3)),
    );
    thumb_at(h, q, frame, s)
}

/// The thumb in two steps (kinds 0x48 and 0x5f): at `(p3 - k) /
/// 0x00b2c5c8 + base` and `(p3 - k) / 0x00b41d00 + base`.
fn thumb_thirds(h: &mut impl Host, q: &mut Quad, frame: u32, p3: i32, k: i32, base: u32) -> u32 {
    let d = load(int_f32(p3.wrapping_sub(k)));
    let s = add(div(d, konst64(h, 0x00b2_c5c8)), konst64(h, base));
    thumb_uvs(h, q);
    thumb_at(h, q, frame, s);
    let s = add(div(d, konst64(h, 0x00b4_1d00)), konst64(h, base));
    thumb_at(h, q, frame, s)
}

/// The track's segments, one per 0x28 rows from `y` (`y - 0x14` to `y`),
/// `n` of them.
fn segments(h: &mut impl Host, q: &mut Quad, frame: u32, y: i32, n: u32) {
    let mut y = y;
    for _ in 0..n {
        q.ys(int_f32(y.wrapping_sub(0x14)), int_f32(y));
        q.draw(h, frame);
        y = y.wrapping_add(0x28);
    }
}

/// `AbmapPadDrawListBar_00a462f0(kind, top, pos, bottom, at)` (cdecl):
/// the sphere grid pad list's bar, quads of `graphicDrawUIAbmapElement`
/// (kind 6) from one record:
///
/// - the caps above `top` and below `bottom` (`top - 2 ∓ 0x00b7b680`), at
///   `kind` and `pos` across, or for 5 or more rows (`n = (bottom - top)
///   / 20`) the layout's fixed edges;
/// - with `at` 0 the track: one piece for fewer than 3 rows, else the odd
///   and the even segments; with `at` 1..3 and more than 0x3c high the
///   segments from row `at` on, odd and even ones with their own UVs;
/// - the thumb, by kind and layout ([`ABMAP_CJK`]): kinds 0xa0 (CJK) and
///   0x88 (else) at `(pos + kind) / 2`; 0x60 / 0x40 in three steps, 0x5f /
///   0x48 in two; other kinds none.
///
/// Returns the last draw's `eax`, or the kind when no thumb is drawn.
/// `frame` is the original's `ebp - 0x19c`
/// ([`BAR_FRAME`] bytes).
#[allow(clippy::too_many_lines)]
pub fn list_bar(
    h: &mut impl Host,
    frame: u32,
    kind: i32,
    top: i32,
    pos: i32,
    bottom: i32,
    at: u32,
) -> u32 {
    let height = bottom.wrapping_sub(top);
    let n = height / 20;
    let cjk = ABMAP_CJK.get(h) != 0;
    let (left, right) = if n <= 4 {
        (int_f32(kind), int_f32(pos))
    } else if cjk && kind == 0xa0 {
        (konst(h, 0x00b5_f330), konst(h, 0x00b6_56dc))
    } else if !cjk && kind == 0x88 {
        (konst(h, 0x00b7_b6d0), konst(h, 0x00b7_b6ec))
    } else {
        (konst(h, 0x00b5_83d8), konst(h, 0x00b6_6dc8))
    };
    // two words between the record and the name buffer, zeroed
    h.write(frame.wrapping_add(0x90), 0u32);
    h.write(frame.wrapping_add(0x94), 0u32);
    let mut q = Quad([0; 32]);
    q.xs(left, right);
    // the top cap
    let ya = int_f32(top.wrapping_sub(2));
    let cap = konst64(h, 0x00b7_b680);
    q.ys(store(sub(load(ya), cap)), ya);
    q.colour([0x80; 4]);
    let [a, b, c, d] = [0x00b5_1b9c, 0x00b7_b5f4, 0x00b4_936c, 0x00b5_ed88].map(|k| konst(h, k));
    q.uvs([[a, b], [a, c], [d, b], [d, c]]);
    q.draw(h, frame);
    // the bottom cap
    let yb = int_f32(bottom.wrapping_sub(2));
    q.ys(store(add(load(yb), cap)), yb);
    q.draw(h, frame);
    let [ua, va, vb, ud, ve, vf] = [
        0x00b7_b5f0,
        0x00b7_b5dc,
        0x00b7_b5e0,
        0x00b7_b614,
        0x00b6_5610,
        0x00b7_b5f8,
    ]
    .map(|k| konst(h, k));
    if at == 0 {
        if n >= 3 {
            q.uvs([[ua, va], [ua, vb], [ud, va], [ud, vb]]);
            // `(n - 1) >> 1` + 1 and `(n - 2) >> 1` + 1, unsigned
            let odd = (n.wrapping_sub(1).cast_unsigned() >> 1).wrapping_add(1);
            segments(h, &mut q, frame, top.wrapping_add(0x12), odd);
            q.uvs([[ua, ve], [ua, vf], [ud, ve], [ud, vf]]);
            let even = (n.wrapping_sub(2).cast_unsigned() >> 1).wrapping_add(1);
            segments(h, &mut q, frame, top.wrapping_add(0x26), even);
        } else {
            q.ys(ya, yb);
            q.uvs([[ua, va], [ua, vb], [ud, va], [ud, vb]]);
            q.draw(h, frame);
        }
    } else if at.cast_signed() < 4 && height > 0x3c {
        let mut k = at.cast_signed();
        let mut y = top.wrapping_add(k.wrapping_mul(0x14)).wrapping_add(0x12);
        while k < 4 {
            q.ys(int_f32(y.wrapping_sub(0x14)), int_f32(y));
            // `and eax, 0x80000001` and the sign fix-up: k odd
            if k % 2 == 0 {
                q.uvs([[ua, va], [ua, vb], [ud, va], [ud, vb]]);
            } else {
                q.uvs([[ua, ve], [ua, vf], [ud, ve], [ud, vf]]);
            }
            q.draw(h, frame);
            k += 1;
            y = y.wrapping_add(0x14);
        }
    }
    // the thumb: the full height, the frame colour; other kinds leave
    // the colour in the record and the kind in `eax`
    q.colour([0xb8, 0xb7, 0xef, 0x80]);
    let thumb = if cjk {
        match kind {
            0xa0 | 0x60 | 0x5f => Some(kind),
            _ => None,
        }
    } else {
        match kind {
            0x88 | 0x40 | 0x48 => Some(kind),
            _ => None,
        }
    };
    let Some(kind) = thumb else {
        q.flush(h, frame);
        return kind.cast_unsigned();
    };
    q.ys(ya, yb);
    match kind {
        0xa0 | 0x88 => thumb_half(h, &mut q, frame, pos, kind),
        0x60 => thumb_steps(h, &mut q, frame, pos, 0x60, 0x00b5_ee40),
        0x40 => thumb_steps(h, &mut q, frame, pos, 0x40, 0x00b4_1308),
        0x5f => thumb_thirds(h, &mut q, frame, pos, 0x5f, 0x00b7_b6b0),
        _ => thumb_thirds(h, &mut q, frame, pos, 0x48, 0x00b6_5658),
    }
}
