//! M5 stage 1, round 6 (list `left6`, from `render-pcui4.left`), batch 3:
//! two Iggy `GDraw` filter-pass set-ups of `FUN_00a36590`'s filter chain:
//! the source and target rectangles as texture coordinates, a render
//! target begun through the API's callbacks, the pass drawn through the
//! target's own callback, the target ended.
//!
//! The callbacks are function pointers: the API's `+0x5c` (begin, five
//! words) and `+0x60` (end, one word) pop their arguments (the originals
//! pop their saved registers right after them), the target's `+0x10`
//! (draw, eight words) is cdecl. Floats follow the x87 at 53 bits: each
//! `fild`/`fstp dword` pair rounds an integer to f32, products stay in the
//! register until their store.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' coordinates
pub mod bridge {
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::{Conv, Host};

    /// f64 0.5 (`g_Vu0FixedRound`).
    pub const HALF: u32 = 0x00b9_2220;
    /// The two set-ups' frames (`ebp` minus the lowest local).
    pub const BOX_FRAME: u32 = 0x3c;
    pub const AXIS_FRAME: u32 = 0x4c;

    fn at(h: &impl Mem, a: u32, off: u32) -> u32 {
        h.read(a.wrapping_add(off))
    }

    /// `fild dword` then `fstp dword`: an `i32` rounded to `f32` (as
    /// `f64`, exactly the stored value).
    fn int_f32(v: u32) -> f64 {
        x87::load(x87::store(f64::from(v.cast_signed())))
    }

    /// `x` stored as `f32` (the bits).
    fn f32_bits(x: f64) -> u32 {
        x87::store(x)
    }

    /// The four corners of rectangle `r` (`i32` x0, y0, x1, y1) as texture
    /// coordinates: `(x0 / w, y0 / h, x1 / w, y1 / h)` with `1 / w` and
    /// `1 / h` rounded first (`inv`), each product rounded to f32; with
    /// `half` the corners move half a texel in (`x0 + 0.5`, `x1 - 0.5`).
    fn coords(h: &impl Mem, r: u32, inv: [f64; 2], half: Option<f64>) -> [u32; 4] {
        let mut out = [0; 4];
        for (k, o) in (0_u32..).zip(&mut out) {
            let v = int_f32(at(h, r, 4 * k));
            let v = match half {
                None => v,
                Some(d) if k < 2 => x87::add(v, d),
                Some(d) => x87::sub(v, d),
            };
            *o = f32_bits(x87::mul(v, inv[(k % 2) as usize]));
        }
        out
    }

    /// `FUN_00a37010(api, target, obj, a, b, src, dst, c)` -> `eax` as the
    /// original: the frame (`ebp - 0x3c..`) gets the source rectangle's
    /// coordinates (`ebp - 0x24`) and the destination's moved half a texel
    /// in (`ebp - 0x14`), over the target's size (`+8`, `+0xc`); the API's
    /// begin (`+0x5c(src, 0, 3, 0, c)`) failing returns `obj + 0x1c`'s word;
    /// else the target's draw (`+0x10(obj, a, b, src, &coords, aspect,
    /// &dst coords, c)`, the aspect `+4 / +0xc`) and the API's end
    /// (`+0x60(c)`), whose `eax` it returns. The frame's scratch words are
    /// left as the original leaves them.
    pub fn box_pass(h: &mut impl Host, ebp: u32, args: [u32; 8]) -> u32 {
        let [api, target, obj, a, b, src, dst, c] = args;
        let fr = |off: u32| ebp.wrapping_sub(off);
        h.write(fr(0x3c), api);
        h.write(fr(0x38), b);
        h.write(fr(0x34), src);
        let w = int_f32(at(h, target, 8));
        let inv_w = x87::load(f32_bits(x87::div(1.0, w)));
        let hh = int_f32(at(h, target, 0xc));
        let inv_h = x87::load(f32_bits(x87::div(1.0, hh)));
        let s = coords(h, src, [inv_w, inv_h], None);
        let half = f64::from_bits(h.read(h.rebase(HALF)));
        let d = coords(h, dst, [inv_w, inv_h], Some(half));
        for (k, v) in (0_u32..).zip(s.iter().chain(&d)) {
            h.write(fr(0x24 - 4 * k), *v);
        }
        // the scratch words as the original leaves them: the last integer
        // of each pair it rounded through them
        h.write(fr(0x30), f32_bits(int_f32(at(h, src, 4))));
        h.write(fr(0x2c), f32_bits(int_f32(at(h, src, 0))));
        h.write(fr(0x28), f32_bits(int_f32(at(h, dst, 0xc))));
        let begin = at(h, api, 0x5c);
        if h.call(begin, Conv::Stdcall, &[src, 0, 3, 0, c]) == 0 {
            return at(h, obj, 0x1c);
        }
        let t = int_f32(at(h, target, 4));
        let th = int_f32(at(h, target, 0xc));
        let aspect = f32_bits(x87::div(t, th));
        h.write(fr(0x28), aspect);
        let draw = at(h, target, 0x10);
        h.call(
            draw,
            Conv::Cdecl,
            &[obj, a, b, src, fr(0x24), aspect, fr(0x14), c],
        );
        let api: u32 = h.read(fr(0x3c));
        let end = at(h, api, 0x60);
        h.call(end, Conv::Stdcall, &[c])
    }

    /// `a / n` as `idiv` (0 where the original faults: `n` 0, overflow).
    fn idiv(a: u32, n: u32) -> u32 {
        a.cast_signed()
            .checked_div(n.cast_signed())
            .unwrap_or(0)
            .cast_unsigned()
    }

    /// `((q - 0.5) * n + 0.5) * inv`, rounded to f32 (the downsampled edge
    /// back in source texels, as texture coordinates).
    fn scaled(q: f64, half: f64, n: f64, inv: f64) -> u32 {
        f32_bits(x87::mul(
            x87::add(x87::mul(x87::sub(q, half), n), half),
            inv,
        ))
    }

    /// `FUN_00a37170(api, sub, obj, a, b, src, vertical, n, w, h, dst, c)`
    /// -> `eax` as the original: a one-axis downsample pass. The source
    /// rectangle's integers go into the frame (`ebp - 0x34..`) with the
    /// filtered axis divided by `n` (its far edge rounded up,
    /// `(e - 1) / n + 1`; x unless `vertical`), and its texture coordinates over `w` x
    /// `h` (`ebp - 0x14..`; the divided axis scaled back by `n` around
    /// half a texel). The API's begin (`+0x5c(&rect, 0, 3, 0, c)`)
    /// failing returns `obj + 0x1c`'s word; else the destination's
    /// coordinates half a texel in (`ebp - 0x24..`), `sub`'s draw
    /// (`+0x10(obj, a, b, &rect, &src coords, aspect, &dst coords, c)`, the
    /// aspect `sub[1] / sub[3]`) and the API's end (`+0x60(c)`), whose `eax`
    /// it returns. The frame's scratch words are left as the original
    /// leaves them.
    #[allow(clippy::too_many_lines)]
    pub fn axis_pass(h: &mut impl Host, ebp: u32, args: [u32; 12]) -> u32 {
        let [api, sub, obj, a, b, src, vertical, n, w, hh, dst, c] = args;
        let fr = |off: u32| ebp.wrapping_sub(off);
        h.write(fr(0x40), sub);
        h.write(fr(0x4c), b);
        let inv_w = f32_bits(x87::div(1.0, int_f32(w)));
        h.write(fr(0x48), inv_w);
        let inv_h = f32_bits(x87::div(1.0, int_f32(hh)));
        h.write(fr(0x44), inv_h);
        let (iw, ih) = (x87::load(inv_w), x87::load(inv_h));
        let half = f64::from_bits(h.read(h.rebase(HALF)));
        let r = [0_u32, 4, 8, 0xc].map(|o| at(h, src, o));
        let nf = int_f32(n);
        let rect = if vertical == 0 {
            [
                idiv(r[0], n),
                r[1],
                idiv(r[2].wrapping_sub(1), n).wrapping_add(1),
                r[3],
            ]
        } else {
            [
                r[0],
                idiv(r[1], n),
                r[2],
                idiv(r[3].wrapping_sub(1), n).wrapping_add(1),
            ]
        };
        for (k, v) in (0_u32..).zip(rect) {
            h.write(fr(0x34 - 4 * k), v);
        }
        h.write(fr(0x3c), f32_bits(nf));
        let f = rect.map(int_f32);
        let coords = if vertical == 0 {
            [
                scaled(f[0], half, nf, iw),
                f32_bits(x87::mul(f[1], ih)),
                scaled(f[2], half, nf, iw),
                f32_bits(x87::mul(ih, f[3])),
            ]
        } else {
            [
                f32_bits(x87::mul(f[0], iw)),
                scaled(f[1], half, nf, ih),
                f32_bits(x87::mul(iw, f[2])),
                scaled(f[3], half, nf, ih),
            ]
        };
        for (k, v) in (0_u32..).zip(coords) {
            h.write(fr(0x14 - 4 * k), v);
        }
        h.write(fr(0x38), f32_bits(f[3]));
        let begin = at(h, api, 0x5c);
        if h.call(begin, Conv::Stdcall, &[fr(0x34), 0, 3, 0, c]) == 0 {
            return at(h, obj, 0x1c);
        }
        let d = [0_u32, 4, 8, 0xc].map(|o| int_f32(at(h, dst, o)));
        let dc = [
            f32_bits(x87::mul(x87::add(d[0], half), iw)),
            f32_bits(x87::mul(x87::add(d[1], half), ih)),
            f32_bits(x87::mul(x87::sub(d[2], half), iw)),
            f32_bits(x87::mul(x87::sub(d[3], half), ih)),
        ];
        for (k, v) in (0_u32..).zip(dc) {
            h.write(fr(0x24 - 4 * k), v);
        }
        h.write(fr(0x38), f32_bits(d[3]));
        let t = int_f32(at(h, sub, 4));
        let u = int_f32(at(h, sub, 0xc));
        let aspect = f32_bits(x87::div(t, u));
        h.write(fr(0x40), aspect);
        let draw = at(h, sub, 0x10);
        h.call(
            draw,
            Conv::Cdecl,
            &[obj, a, b, fr(0x34), fr(0x14), aspect, fr(0x24), c],
        );
        let end = at(h, api, 0x60);
        h.call(end, Conv::Stdcall, &[c])
    }

    /// `CamDebug_MatVec_00942260(dst, m, v)`; f64 1.25, 1/1024, 480 and
    /// 0.5 (`g_Vu0FixedRound`).
    pub const MAT_VEC: u32 = 0x0094_2260;
    pub const SCALE_X: u32 = 0x00b4_99c0;
    pub const PER_1024: u32 = 0x00b5_8b50;
    pub const HEIGHT_480: u32 = 0x00b5_5138;
    /// `0x00940f30`'s frame.
    pub const PICK_FRAME: u32 = 0x50;

    /// `CamDebug_ScreenPick_00940f30(px, py, cam, point, offset)` ->
    /// `CamDebug_MatVec_00942260`'s `eax`: the debug camera's mouse pick.
    /// The mouse (`px - 0x100`, `py - 0xd0`) scaled to the view (`x 1.25 /
    /// 1024`, `/ 480 x 0.5`); the point and the point plus the offset
    /// through `cam` (`ebp - 0x24`, `- 0x14`); the ray's parameter at the
    /// view plane (the near point's dot with the difference over the
    /// difference's along the mouse ray); the hit less `cam`'s translation
    /// (`+0x30..`) back through its rotation (transposed rows `+0`,
    /// `+0x10`, `+0x20`) into `point`. Each value the original stores is
    /// rounded to f32 there; the frame is left as it leaves it.
    pub fn screen_pick(h: &mut impl Host, ebp: u32, args: [u32; 5]) -> u32 {
        let [px, py, cam, point, offset] = args;
        let fr = |off: u32| ebp.wrapping_sub(off);
        let k = |h: &mut _, sym: u32| -> f64 { f64::from_bits(Mem::read(h, Mem::rebase(h, sym))) };
        let f = |h: &mut _, a: u32, off: u32| -> f64 { x87::load(at(h, a, off)) };
        let r32 = |x: f64| x87::load(f32_bits(x));
        let mx = int_f32(px.wrapping_sub(0x100));
        let sx = r32(x87::mul(x87::mul(mx, k(h, SCALE_X)), k(h, PER_1024)));
        let my = int_f32(py.wrapping_sub(0xd0));
        let half = k(h, HALF);
        let sy = r32(x87::mul(x87::div(my, k(h, HEIGHT_480)), half));
        let far = [
            f32_bits(x87::add(f(h, point, 0), f(h, offset, 0))),
            f32_bits(x87::add(f(h, offset, 4), f(h, point, 4))),
            f32_bits(x87::add(f(h, offset, 8), f(h, point, 8))),
            1.0_f32.to_bits(),
        ];
        for (j, v) in (0_u32..).zip(far) {
            h.write(fr(0x50 - 4 * j), v);
        }
        h.call_sym(MAT_VEC, Conv::Cdecl, &[fr(0x24), cam, point]);
        let r = h.call_sym(MAT_VEC, Conv::Cdecl, &[fr(0x14), cam, fr(0x50)]);
        let a = [0_u32, 4, 8].map(|o| f(h, fr(0x14), o));
        let b = [0_u32, 4, 8].map(|o| f(h, fr(0x24), o));
        let d: [f64; 3] = std::array::from_fn(|j| r32(x87::sub(a[j], b[j])));
        for (j, v) in (0_u32..).zip(d) {
            h.write(fr(0x40 - 4 * j), f32_bits(v));
        }
        let dot = x87::add(
            x87::add(x87::mul(b[0], d[0]), x87::mul(b[1], d[1])),
            x87::mul(b[2], d[2]),
        );
        let along = x87::add(d[2], x87::add(x87::mul(d[0], sx), x87::mul(d[1], sy)));
        let t = r32(x87::div(r32(dot), r32(along)));
        let r0 = r32(x87::sub(r32(x87::mul(sx, t)), f(h, cam, 0x30)));
        let r1 = r32(x87::sub(r32(x87::mul(sy, t)), f(h, cam, 0x34)));
        let r2 = r32(x87::sub(t, f(h, cam, 0x38)));
        let c = |h: &mut _, off: u32| f(h, cam, off);
        let oy = f32_bits(x87::add(
            x87::add(x87::mul(c(h, 0x10), r0), x87::mul(c(h, 0x14), r1)),
            x87::mul(c(h, 0x18), r2),
        ));
        let oz = f32_bits(x87::add(
            x87::add(x87::mul(c(h, 0x24), r1), x87::mul(c(h, 0x20), r0)),
            x87::mul(c(h, 0x28), r2),
        ));
        let ox = f32_bits(x87::add(
            x87::add(x87::mul(c(h, 0), r0), x87::mul(c(h, 4), r1)),
            x87::mul(r2, c(h, 8)),
        ));
        h.write(fr(0x30), f32_bits(r1));
        h.write(fr(0x2c), oz);
        h.write(fr(0x28), oy);
        h.write(point, ox);
        h.write(point.wrapping_add(4), oy);
        h.write(point.wrapping_add(8), oz);
        r
    }

    /// `g_GDrawState`; `gdraw_lockDynamicBuffer_00a3cf40(buffer, n)`;
    /// `IggyGDrawSendWarning` (import slot, cdecl, varargs) and its texts.
    pub const GDRAW: u32 = 0x01a8_58b4;
    pub const LOCK_DYNAMIC: u32 = 0x00a3_cf40;
    pub const SEND_WARNING_SLOT: u32 = 0x00b0_c874;
    pub const MAP_FAILED_TEXTS: [u32; 4] = [0x00b7_a4ec, 0x00b7_a50c, 0x00b6_868c, 0x00b7_a408];
    /// `0x00a36090`'s frame.
    pub const QUAD_FRAME: u32 = 0x14;

    /// `g_GDrawState`'s value.
    fn gstate(h: &impl Mem) -> u32 {
        h.read(h.rebase(GDRAW))
    }

    /// A context method at byte offset `off` (stdcall, `ctx` first).
    fn com(h: &mut impl Host, ctx: u32, off: u32, args: &[u32]) -> u32 {
        let f = at(h, at(h, ctx, 0), off);
        let mut words = vec![ctx];
        words.extend_from_slice(args);
        h.call(f, Conv::Stdcall, &words)
    }

    /// `FUN_00a36090(src, stats, x0, y0, x1, y1)` -> `eax` as the original:
    /// a textured quad: four vertices (position, the source's u/v corners)
    /// into the dynamic vertex buffer (`gdraw_lockDynamicBuffer_00a3cf40`;
    /// none: 0), the buffer unmapped; the vertex constants mapped (a
    /// failure warns: out of memory or the error code), set to the identity
    /// scale and `g_GDrawState + 0x71c`'s 12 words, unmapped, then the
    /// constant buffer, shader, layout, vertex buffer (stride 16 at `ebp -
    /// 8`, its offset at `- 4`), a 4-vertex strip drawn, the topology put
    /// back, and the stats counted (`+0` flag 1, `+2` draws, `+0x10`
    /// indices 6, `+0x14` vertices 4); `eax` the stats then, the `Map`
    /// result without data.
    pub fn quad(h: &mut impl Host, ebp: u32, args: [u32; 6]) -> u32 {
        let [src, stats, x0, y0, x1, y1] = args;
        let fr = |off: u32| ebp.wrapping_sub(off);
        let s = gstate(h);
        let ctx = at(h, s, 4);
        let v = h.call_sym(LOCK_DYNAMIC, Conv::Cdecl, &[s.wrapping_add(0x998), 0x40]);
        if v == 0 {
            return 0;
        }
        let c = |w: u32| x87::store(x87::load(w));
        let uv = [0_u32, 4, 8, 0xc].map(|o| c(at(h, src, o)));
        let (x0, y0, x1, y1) = (c(x0), c(y0), c(x1), c(y1));
        let verts = [
            x0, y0, uv[0], uv[1], x1, y0, uv[2], uv[1], x0, y1, uv[0], uv[3], x1, y1, uv[2], uv[3],
        ];
        for (k, w) in (0_u32..).zip(verts) {
            h.write(v.wrapping_add(4 * k), w);
        }
        let s = gstate(h);
        let buffer = at(h, s, 0x998);
        com(h, at(h, s, 4), 0x3c, &[buffer, 0]);
        h.write(fr(4), at(h, s, 0x9a0));
        let s2 = gstate(h);
        h.write(fr(8), 0x10_u32);
        let hr = com(
            h,
            at(h, s2, 4),
            0x38,
            &[at(h, s2, 0x984), 0, 4, 0, fr(0x14)],
        );
        if hr.cast_signed() < 0 {
            let f = at(h, h.rebase(SEND_WARNING_SLOT), 0);
            let [oom, code, a, b] = MAP_FAILED_TEXTS.map(|t| h.rebase(t));
            return if hr == 0x8007_000e {
                h.call(f, Conv::Cdecl, &[0, oom, a, b])
            } else {
                h.call(f, Conv::Cdecl, &[0, code, a, b, hr])
            };
        }
        let p = at(h, fr(0x14), 0);
        if p == 0 {
            return hr;
        }
        let one = 1.0_f32.to_bits();
        for (k, w) in (0_u32..).zip([one, 0, 0, 0, 0, one, 0, 0]) {
            h.write(p.wrapping_add(4 * k), w);
        }
        let s = gstate(h);
        for k in 0..12_u32 {
            let w = at(h, s, 0x71c + 4 * k);
            h.write(p.wrapping_add(0x50 + 4 * k), w);
        }
        com(h, at(h, s, 4), 0x3c, &[at(h, s, 0x984), 0]);
        let cb = gstate(h).wrapping_add(0x984);
        com(h, ctx, 0x1c, &[0, 1, cb]);
        let shader = at(h, gstate(h), 0x284);
        com(h, ctx, 0x2c, &[shader, 0, 0]);
        let layout = at(h, gstate(h), 0x268);
        com(h, ctx, 0x44, &[layout]);
        let vb = gstate(h).wrapping_add(0x998);
        com(h, ctx, 0x48, &[0, 1, vb, fr(8), fr(4)]);
        com(h, ctx, 0x60, &[5]);
        com(h, ctx, 0x34, &[4, 0]);
        com(h, ctx, 0x60, &[4]);
        let flags: u16 = h.read(stats);
        h.write(stats, flags | 1);
        let n: u16 = h.read(stats.wrapping_add(2));
        h.write(stats.wrapping_add(2), n.wrapping_add(1));
        let i = at(h, stats, 0x10);
        h.write(stats.wrapping_add(0x10), i.wrapping_add(6));
        let w = at(h, stats, 0x14);
        h.write(stats.wrapping_add(0x14), w.wrapping_add(4));
        stats
    }
}
