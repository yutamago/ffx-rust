//! M5 stage 1, list `main`, round 4: the sphere grid's node shapes drawn
//! through `op_shape_draw_eamap` (the ability map's highlight and all
//! nodes' shapes). Both originals align their frame to 16 bytes and build
//! a shape draw key there: `ebp` below is that aligned frame.
//!
//! The frame (`ebp - n`): `0x100` the key (`+2` the `u16` 0x14, `+4` the
//! colour, `+8`/`+0xc` zero, `+0x10` the matrix pointer, `+0x18` a zero
//! byte, `+0x1c` the light matrix pointer; its other words are never
//! written: junk reads, 0 here as in the parity tests), `0xc8` and
//! `0x88` the node's matrix and its product with the view (in either
//! order), `0x48` the
//! light matrix (rows of the emulated zero register's words, the last
//! with `16384.0` and `1.0`).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `g_Abmap` (a pointer), `g_Ee_Gpr`'s zero register (4 words), the
    /// `f32` 16384; `AbmapMakeNodeMatrix(out, node, scale)`,
    /// `FFXVu0MulMatrix(out, a, b)`, `op_shape_draw_eamap(shape, key,
    /// index, slot)` (cdecl).
    pub const ABMAP: u32 = 0x0230_5834;
    pub const ZERO_REGISTER: u32 = 0x00c8_f508;
    pub const F16384: u32 = 0x00b7_b708;
    pub const MAKE_NODE_MATRIX: u32 = 0x00a5_ad30;
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    pub const SHAPE_DRAW: u32 = 0x007f_49e0;
    pub const NODE_HIGHLIGHT_SYM: u32 = 0x00a5_1560;
    pub const NODE_SHAPES_SYM: u32 = 0x00a4_fe40;

    /// The key, the light matrix and the matrix pointers set up in the
    /// frame at `ebp` (everything both originals write before their
    /// loops); the key's matrix is the one at `ebp - product`.
    fn frame_setup(h: &mut impl Host, ebp: u32, product: u32) {
        let zero = h.rebase(ZERO_REGISTER);
        let mut w = [0u32; 4];
        for (k, x) in (0u32..).zip(w.iter_mut()) {
            *x = h.read(at(zero, 4 * k));
        }
        let f = |o: u32| ebp.wrapping_sub(o);
        h.write(f(0xf0), f(product));
        h.write(f(0xe4), f(0x48));
        for row in 0..4u32 {
            let r = f(0x48).wrapping_add(16 * row);
            for (k, x) in (0u32..).zip(w) {
                h.write(at(r, 4 * k), x);
            }
        }
        let big: u32 = h.read(h.rebase(F16384));
        h.write(f(0x10), big);
        h.write(f(0xc), 1.0f32.to_bits());
        h.write(f(0xfe), 0x14u16);
        h.write(f(0xf8), 0u32);
        h.write(f(0xf4), 0u32);
        h.write(f(0xe8), 0u8);
    }

    /// `FUN_00a51560(i)` (cdecl): node `i` of the sphere grid (when it
    /// exists: `i` up to the `i16` count `+2`) drawn as its shape (the
    /// node's matrix times the view `+0x113e0`; the shape of its type
    /// `+6` from the table `+0xf828`, the lit or the dark one by `+0x21`,
    /// colour 0x80454545 or 0x80121212). `ebp` is the original's aligned
    /// frame. Returns the draw's `eax`, or `i`.
    pub fn node_highlight(h: &mut impl Host, i: u32, ebp: u32) -> u32 {
        frame_setup(h, ebp, 0xc8);
        let f = |o: u32| ebp.wrapping_sub(o);
        let base: u32 = h.read(h.rebase(ABMAP));
        if i.cast_signed() > i32::from(h.read::<i16>(at(base, 2))) {
            return i;
        }
        let node = at(base, i.wrapping_mul(0x28).wrapping_add(0x808));
        cdecl(h, MAKE_NODE_MATRIX, &[f(0x88), node, 1.0f32.to_bits()]);
        cdecl(h, MUL_MATRIX, &[f(0xc8), at(base, 0x1_13e0), f(0x88)]);
        let kind = u32::from(h.read::<u16>(at(node, 6))).wrapping_mul(0x30);
        let (colour, shape_at) = if h.read::<u8>(at(node, 0x21)) != 0 {
            (0x8045_4545u32, 0xf828)
        } else {
            (0x8012_1212, 0xf82c)
        };
        h.write(f(0xfc), colour);
        let shape: u32 = h.read(at(base, kind.wrapping_add(shape_at)));
        let base: u32 = h.read(h.rebase(ABMAP));
        let slot: u32 = h.read(at(base, 0x1_16a4));
        cdecl(h, SHAPE_DRAW, &[shape, f(0x100), i, slot])
    }

    /// `FUN_00a4fe40()` (cdecl): every node of the sphere grid (`+2`
    /// nodes of 0x28 bytes from `+0x808`; type 0xffff skipped) drawn as
    /// its type's shape (`+0xf828 + 0x30 * type + 8`), the key's colour 0,
    /// numbered from the last down. `ebp` is the original's aligned frame.
    pub fn node_shapes(h: &mut impl Host, ebp: u32) {
        frame_setup(h, ebp, 0x88);
        let f = |o: u32| ebp.wrapping_sub(o);
        let base: u32 = h.read(h.rebase(ABMAP));
        let mut left = i32::from(h.read::<i16>(at(base, 2))).cast_unsigned();
        let table = at(base, 0xf828);
        let mut node = at(base, 0x808);
        while left != 0 {
            left = left.wrapping_sub(1);
            let kind: u16 = h.read(at(node, 6));
            if kind != 0xffff {
                cdecl(h, MAKE_NODE_MATRIX, &[f(0xc8), node, 1.0f32.to_bits()]);
                let g: u32 = h.read(h.rebase(ABMAP));
                cdecl(h, MUL_MATRIX, &[f(0x88), at(g, 0x1_13e0), f(0xc8)]);
                h.write(f(0xfc), 0u32);
                let g: u32 = h.read(h.rebase(ABMAP));
                let slot: u32 = h.read(at(g, 0x1_16a4));
                let count = i32::from(h.read::<i16>(at(g, 2))).cast_unsigned();
                let shape: u32 = h.read(at(
                    table,
                    u32::from(kind).wrapping_mul(0x30).wrapping_add(8),
                ));
                cdecl(
                    h,
                    SHAPE_DRAW,
                    &[shape, f(0x100), left.wrapping_sub(count), slot],
                );
            }
            node = at(node, 0x28);
        }
    }

    /// `AbmapMakeLinkMatrix(out, node, link, scale)`,
    /// `op1_md_draw_eiabm_sphe(obj, key, index, part)` (cdecl), the
    /// sphere model (`0x023057ec`), the sine table, the two colour tables,
    /// the constants (`f64` 0.15 and the pulse scale, `f32` the link scale,
    /// the two `f32` light values).
    pub const MAKE_LINK_MATRIX: u32 = 0x00a5_a360;
    pub const DRAW_SPHERE: u32 = 0x00a6_8140;
    pub const SPHERE_MODEL: u32 = 0x0230_57ec;
    pub const SIN_TABLE: u32 = 0x00c4_4bf0;
    pub const LIT_COLOURS: u32 = 0x00c8_659c;
    pub const DARK_COLOURS: u32 = 0x00c8_6644;
    pub const PULSE_A: u32 = 0x00b5_9870;
    pub const PULSE_B: u32 = 0x00b7_b718;
    pub const LINK_SCALE: u32 = 0x00b7_b5d4;
    pub const LIGHT_DIM: u32 = 0x00b9_2308;
    pub const LIGHT_ALT: u32 = 0x00b9_2340;
    pub const NODE_SPHERES_SYM: u32 = 0x00a5_0ed0;
    /// The sphere draw data the key points at (an image address).
    pub const SPHERE_DATA: u32 = 0x0174_0870;

    /// The colour `c` with each byte scaled by its factor (`>> 7`, the
    /// shifted sums as the original adds them: no clamping).
    fn scale_colour(c: u32, m: [u32; 4]) -> u32 {
        let part = |shift: u32, k: usize| {
            ((c.cast_signed() >> shift).cast_unsigned() & 0xff)
                .wrapping_mul(m[k])
                .cast_signed()
                >> 7
        };
        let b = |v: i32| v.cast_unsigned() & 0x1ff_ffff;
        let (b3, b2, b1, b0) = (b(part(24, 3)), b(part(16, 2)), b(part(8, 1)), b(part(0, 0)));
        let hi = (b3 << 8).wrapping_add(b2);
        ((hi << 8).wrapping_add(b1) << 8).wrapping_add(b0)
    }

    /// `FUN_00a50ed0(i)` (cdecl): node `i`'s spheres (seven parts drawn
    /// with `op1_md_draw_eiabm_sphe`, part numbers -1 down to -7) in the
    /// aligned frame at `ebp`: the key at `ebp - 0x140` (type, model
    /// data, colour, the matrix pointers), the light matrix at `ebp -
    /// 0x48` whose `+0x38` word is the light. A node type whose link word
    /// (`+0xf83c + 0x30 * type`) isn't 0x1000 draws its links: each part
    /// `j`'s link matrix (scale `0x00b7b5d4`) and, for the parts the
    /// node's mask (`base + 0x115c9 & +0x21`) has, the lit colour pulsing
    /// with the sine of the grid's clock (`0x6c - sin * 0.15 * ..`), type
    /// 0x2c and light 1.0, else the dark colour, type 0x24 and the dim
    /// light. Else every part is drawn plain (colour 0, light
    /// `0x00b92340`; the key's type word and `+0x138` are never written
    /// on that path: junk reads, 0 here as in the parity tests).
    #[allow(clippy::too_many_lines)] // the original's two loops
    pub fn node_spheres(h: &mut impl Host, i: u32, ebp: u32) {
        let f = |o: u32| ebp.wrapping_sub(o);
        let base: u32 = h.read(h.rebase(ABMAP));
        let node = at(base, i.wrapping_mul(0x28).wrapping_add(0x808));
        let clock = h.read::<u32>(at(base, 0x1_1698)) << 11;
        h.write(f(0x120), f(0xc8));
        h.write(f(0xfc), f(0x48));
        h.write(f(0x114), f(0x88));
        let zero = h.rebase(ZERO_REGISTER);
        let mut w = [0u32; 4];
        for (k, x) in (0u32..).zip(w.iter_mut()) {
            *x = h.read(at(zero, 4 * k));
        }
        for row in 0..4u32 {
            let r = f(0x48).wrapping_add(16 * row);
            for (k, x) in (0u32..).zip(w) {
                h.write(at(r, 4 * k), x);
            }
        }
        let one = 1.0f32.to_bits();
        h.write(f(0xc), one);
        for o in [0x124, 0x134, 0x130, 0x12c, 0x104, 0x108, 0x100] {
            h.write(f(o), 0u32);
        }
        h.write(f(0x136), 0u8);
        let data = h.rebase(SPHERE_DATA);
        h.write(f(0x13c), data);
        let kind = u32::from(h.read::<u16>(at(node, 6))).wrapping_mul(0x30);
        h.write(f(0x118), at(base, 0x1_1520));
        h.write(f(0x11c), at(base, 0x1_1560));
        let mut link = at(base, kind.wrapping_add(0xf83c));
        let scale_at = at(base, kind.wrapping_add(0xf838));
        if h.read::<u16>(link) == 0x1000 {
            let alt: u32 = h.read(h.rebase(LIGHT_ALT));
            for part in 1..8u32 {
                let scale = copy_f32(h.read(scale_at));
                cdecl(h, MAKE_NODE_MATRIX, &[f(0x88), node, scale]);
                let g: u32 = h.read(h.rebase(ABMAP));
                cdecl(h, MUL_MATRIX, &[f(0xc8), at(g, 0x1_13e0), f(0x88)]);
                h.write(f(0x10), copy_f32(alt));
                h.write(f(0x128), 0u32);
                let obj: u32 = h.read(h.rebase(SPHERE_MODEL));
                cdecl(h, DRAW_SPHERE, &[obj, f(0x140), i, part.wrapping_neg()]);
            }
            return;
        }
        let mut mask = h.read::<u8>(at(base, 0x1_15c9)) & h.read::<u8>(at(node, 0x21));
        let mut factors = [0u32; 4];
        if mask != 0 {
            let phase = u32::from(h.read::<u16>(at(node, 0x26))).wrapping_add(clock);
            let k = (phase.cast_signed() >> 4).cast_unsigned() & 0xfff;
            let sin = f64::from(f32::from_bits(h.read(at(h.rebase(SIN_TABLE), 4 * k))));
            let a = f64::from_bits(h.read::<u64>(h.rebase(PULSE_A)));
            let b = f64::from_bits(h.read::<u64>(h.rebase(PULSE_B)));
            let t = ffx_bridge::x87::ftol(h, sin * a * b);
            factors = [0x6cu32.wrapping_sub(t) & 0xff; 4];
        }
        h.write(f(0x138), 0x8064u16);
        let dim: u32 = h.read(h.rebase(LIGHT_DIM));
        for j in 0..7u32 {
            let light = if mask & 1 != 0 {
                let c: u32 = h.read(at(h.rebase(LIT_COLOURS), 4 * j));
                h.write(f(0x128), scale_colour(c, factors));
                h.write(f(0x125), 0x80u8);
                h.write(f(0x140), 0x2cu32);
                one
            } else {
                let c: u32 = h.read(at(h.rebase(DARK_COLOURS), 4 * j));
                h.write(f(0x128), c);
                h.write(f(0x140), 0x24u32);
                copy_f32(dim)
            };
            h.write(f(0x10), light);
            let scale = copy_f32(h.read(scale_at));
            cdecl(h, MAKE_NODE_MATRIX, &[f(0x88), node, scale]);
            let s: u32 = h.read(h.rebase(LINK_SCALE));
            cdecl(h, MAKE_LINK_MATRIX, &[f(0x88), node, link, copy_f32(s)]);
            let g: u32 = h.read(h.rebase(ABMAP));
            cdecl(h, MUL_MATRIX, &[f(0xc8), at(g, 0x1_13e0), f(0x88)]);
            let obj: u32 = h.read(h.rebase(SPHERE_MODEL));
            cdecl(h, DRAW_SPHERE, &[obj, f(0x140), i, (j + 1).wrapping_neg()]);
            link = at(link, 4);
            mask >>= 1;
        }
    }

    /// `fld dword` / `fstp dword`.
    fn copy_f32(bits: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(bits))
    }
}
