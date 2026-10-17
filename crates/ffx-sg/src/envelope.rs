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
}
