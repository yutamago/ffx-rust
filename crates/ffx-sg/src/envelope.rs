//! The character envelope (skinning): vertex lists transformed by a bone's
//! matrix (`cdc_Mul3x4`) into the model's vertex buffer, in thousandths
//! (`sg_calcenvelope*`). The vertex lists come one after another: a count,
//! then the entries.

/// The originals, over game memory.
#[allow(clippy::many_single_char_names)] // vertices, weights, indices
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_bridge::{Global, Mem};

    use crate::scratch::{SCRATCH, Scratch};

    /// 1000 (`f64`).
    pub const THOUSAND: Global<f64> = Global::new(0x00b0_dba0);

    /// `cdc_Mul3x4(dst, m, v)` on the vertex `x, y, z` (each through
    /// `fld`/`fstp`): the transformed `f32` x, y, z.
    fn transform(mem: &mut impl Mem, m: u32, v: [u32; 3]) -> [f32; 3] {
        let v = v.map(|b| x87::round_trip(f32::from_bits(b)));
        let mut s = Scratch::new(mem);
        s.write(SCRATCH + 0x20, v);
        ffx_system::math::bridge::mul3x4(&mut s, SCRATCH, m, SCRATCH + 0x20);
        s.read(SCRATCH)
    }

    fn st(x: f64) -> f32 {
        f32::from_bits(x87::store(x))
    }

    fn ld(x: f32) -> f64 {
        x87::load(x.to_bits())
    }

    /// `sg_calcenvelopeW_asm_opt(lists, out, m)` (`0x006ee270`): three
    /// lists after one another. Rigid vertices (16 bytes: `x, y, z`,
    /// index) set `out[index]` to the transformed vertex x 1000 and w 1.0;
    /// first weighted vertices (20 bytes: `x, y, z`, weight, index) set it
    /// to the transformed vertex x weight x 1000 (each product stored as
    /// `f32`), w 1.0; further weighted vertices add that to `out[index]`
    /// (w + 0.0). Returns the end of the lists (where the next would
    /// start).
    pub fn calc_envelope_w_opt(mem: &mut impl Mem, lists: u32, out: u32, m: u32) -> u32 {
        let k = THOUSAND.get(mem);
        let slot = |i: u32| out.wrapping_add(i << 4);
        let mut p = lists;
        let n: u32 = mem.read(p);
        p = p.wrapping_add(4);
        for _ in 0..n {
            let v: [u32; 3] = mem.read(p);
            let i: u32 = mem.read(p.wrapping_add(0xc));
            p = p.wrapping_add(0x10);
            let r = transform(mem, m, v);
            let o = r.map(|x| st(x87::mul(ld(x), k)));
            mem.write(slot(i), [o[0], o[1], o[2], 1.0]);
        }
        for add in [false, true] {
            let n: u32 = mem.read(p);
            p = p.wrapping_add(4);
            for _ in 0..n {
                let v: [u32; 3] = mem.read(p);
                let w = ld(x87::round_trip(mem.read(p.wrapping_add(0xc))));
                let i: u32 = mem.read(p.wrapping_add(0x10));
                p = p.wrapping_add(0x14);
                let r = transform(mem, m, v);
                let q = r.map(|x| st(x87::mul(ld(st(x87::mul(ld(x), w))), k)));
                if add {
                    let cur: [f32; 4] = mem.read(slot(i));
                    let s = [0, 1, 2].map(|j| st(x87::add(ld(cur[j]), ld(q[j]))));
                    let sw = st(x87::add(ld(cur[3]), 0.0));
                    mem.write(slot(i), [s[0], s[1], s[2], sw]);
                } else {
                    mem.write(slot(i), [q[0], q[1], q[2], 1.0]);
                }
            }
        }
        p
    }

    /// 255 (`f64`).
    pub const TWO_FIFTY_FIVE: Global<f64> = Global::new(0x00b1_70c0);

    /// A packed vertex (`i16` x, y, z in thousandths) through
    /// `cdc_Mul3x4`: each coordinate `v / 1000` stored as `f32`.
    fn transform_packed(mem: &mut impl Mem, m: u32, at: u32, k: f64) -> [f32; 3] {
        let v: [i16; 3] = mem.read(at);
        let v = v.map(|c| x87::store(x87::div(f64::from(c), k)));
        transform(mem, m, v)
    }

    /// `sg_calcenvelope_asm_opt(lists, out, m)` (`0x006eee40`): the packed
    /// form. Rigid vertices (`u16` count, 2 bytes, then 8-byte entries:
    /// `i16` x, y, z / 1000, `u16` index) set `out[index]` to the
    /// transformed vertex x 1000, w 1.0. Then groups of first weighted
    /// vertices (`u16` count, `0xffff` ending them; `u16` base; 8-byte
    /// entries: `i16` x, y, z, `u8` weight / 255, `u8` index + base x 256)
    /// set `out[index]` to transformed x weight x 1000 (each product an
    /// `f32`), w 1.0. After 2 more bytes, groups of further weighted
    /// vertices (the same form) add to `out[index]`, w 1.0. Returns the end
    /// of the lists.
    pub fn calc_envelope_opt(mem: &mut impl Mem, lists: u32, out: u32, m: u32) -> u32 {
        let k = THOUSAND.get(mem);
        let c255 = TWO_FIFTY_FIVE.get(mem);
        let slot = |i: u32| out.wrapping_add(i << 4);
        let n = u32::from(mem.read::<u16>(lists));
        let mut p = lists.wrapping_add(4);
        for _ in 0..n {
            let i = u32::from(mem.read::<u16>(p.wrapping_add(6)));
            let r = transform_packed(mem, m, p, k);
            p = p.wrapping_add(8);
            let o = r.map(|x| st(x87::mul(ld(x), k)));
            mem.write(slot(i), [o[0], o[1], o[2], 1.0]);
        }
        for add in [false, true] {
            if add {
                // the first group's count follows 2 more bytes
                p = p.wrapping_add(2);
            }
            loop {
                let n = u32::from(mem.read::<u16>(p));
                p = p.wrapping_add(2);
                if n == 0xffff {
                    break;
                }
                let base = u32::from(mem.read::<u16>(p)) << 8;
                p = p.wrapping_add(2);
                for _ in 0..n {
                    let wt = u32::from(mem.read::<u8>(p.wrapping_add(6)));
                    let i = u32::from(mem.read::<u8>(p.wrapping_add(7))).wrapping_add(base);
                    let w = ld(st(x87::div(f64::from(wt), c255)));
                    let r = transform_packed(mem, m, p, k);
                    p = p.wrapping_add(8);
                    let q = r.map(|x| st(x87::mul(ld(st(x87::mul(ld(x), w))), k)));
                    let s = if add {
                        let cur: [f32; 3] = mem.read(slot(i));
                        [0, 1, 2].map(|j| st(x87::add(ld(q[j]), ld(cur[j]))))
                    } else {
                        q
                    };
                    mem.write(slot(i), [s[0], s[1], s[2], 1.0]);
                }
            }
        }
        p
    }
}
