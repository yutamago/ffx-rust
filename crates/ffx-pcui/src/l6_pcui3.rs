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
}
