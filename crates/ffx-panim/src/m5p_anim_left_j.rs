//! List `phyre-anim-left`, batch 10 (M5 stage 1): `PAnimatableComponent`'s
//! evaluation order (`0x0053cb90`): its `0x94`-byte records become the
//! nodes of a network optimiser (in the caller's frame, `frame`), linked
//! by their sources' inputs; the optimiser's order is written back into
//! the records, the deepest level sized into the component's level array.

#![allow(clippy::many_single_char_names)]

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The optimiser's constructor and destructor (thiscall).
pub const OPT_CTOR: u32 = 0x005c_d3f0;
/// See [`OPT_CTOR`].
pub const OPT_DTOR: u32 = 0x005c_d490;
/// The optimiser's node table, node link and run (`0x005cd9e0`,
/// `0x005cd920`, `0x005cd7a0`; in this list).
pub const OPT_MAKE_NODES: u32 = 0x005c_d9e0;
/// See [`OPT_MAKE_NODES`].
pub const OPT_LINK: u32 = 0x005c_d920;
/// See [`OPT_MAKE_NODES`].
pub const OPT_RUN: u32 = 0x005c_d7a0;
/// The optimiser's queries (thiscall): node count `()`, level `(i, 0)`,
/// node of place `(i)`, input count `(i)`, input `(i, k)`.
pub const OPT_COUNT: u32 = 0x005c_d790;
/// See [`OPT_COUNT`].
pub const OPT_LEVEL: u32 = 0x005c_d700;
/// See [`OPT_COUNT`].
pub const OPT_NODE: u32 = 0x005c_d750;
/// See [`OPT_COUNT`].
pub const OPT_INPUTS: u32 = 0x005c_d6b0;
/// See [`OPT_COUNT`].
pub const OPT_INPUT: u32 = 0x005c_d660;
/// The word every node is set up with.
pub const NODE_TAG: u32 = 0x00b2_ab88;
/// How many words of the caller's frame the optimiser takes (`ebp -
/// 0x30`).
pub const FRAME_WORDS: u32 = 2;

pub mod bridge {
    use super::{
        Conv, Host, Mem, NODE_TAG, NOT_OWNED, OPT_COUNT, OPT_CTOR, OPT_DTOR, OPT_INPUT, OPT_INPUTS,
        OPT_LEVEL, OPT_LINK, OPT_MAKE_NODES, OPT_NODE, OPT_RUN, PHYRE_FREE, PHYRE_MALLOC,
    };

    fn free(h: &mut impl Host, p: u32) {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
    }

    /// The record `i` (`0x94` bytes from `[this + 8]`).
    fn record(m: &impl Mem, this: u32, i: u32) -> u32 {
        let base: u32 = m.read(this.wrapping_add(8));
        base.wrapping_add(i.wrapping_mul(0x94))
    }

    /// `0x0053cb90` (`thiscall`): see the module. 0, or 0xd when the level
    /// array cannot be allocated.
    #[allow(clippy::too_many_lines)]
    pub fn order_records(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let opt = frame;
        h.call_sym(OPT_CTOR, Conv::Thiscall(opt), &[]);
        let n = h.read::<u32>(this.wrapping_add(4)) & !NOT_OWNED;
        h.call_sym(OPT_MAKE_NODES, Conv::Thiscall(opt), &[n]);
        let tag = h.rebase(NODE_TAG);
        for i in 0..n {
            let src: u32 = h.read(record(h, this, i));
            let m = h.call_virtual(src, 1, &[]);
            let pairs = if m == 0 {
                0
            } else {
                h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[m << 3, 4])
            };
            let held = if pairs == 0 { 0 } else { m };
            let mut k = 0u32;
            for j in 0..m {
                let input = h.call_virtual(src, 2, &[j]);
                if input == 0 {
                    continue;
                }
                let base: u32 = h.read(this.wrapping_add(8));
                if let Some(d) = (0..n)
                    .find(|&d| h.read::<u32>(base.wrapping_add(d.wrapping_mul(0x94))) == input)
                {
                    k += 1;
                    h.write(pairs.wrapping_add(8 * k - 8), d);
                    h.write(pairs.wrapping_add(8 * k - 4), 0u32);
                }
            }
            h.call_sym(OPT_LINK, Conv::Thiscall(opt), &[i, tag, k, pairs, 1, 0]);
            if held & NOT_OWNED == 0 && pairs != 0 {
                free(h, pairs);
            }
        }
        h.call_sym(OPT_RUN, Conv::Thiscall(opt), &[0]);
        let nb = h.call_sym(OPT_COUNT, Conv::Thiscall(opt), &[]);
        let mut levels = 0u32;
        for s in 0..nb {
            let v = h
                .call_sym(OPT_LEVEL, Conv::Thiscall(opt), &[s, 0])
                .wrapping_add(1);
            if levels <= v {
                levels = v;
            }
        }
        // the level array at +0x2c/+0x30
        let raw: u32 = h.read(this.wrapping_add(0x2c));
        if levels != raw {
            let keep = levels.min(raw & !NOT_OWNED);
            let old: u32 = h.read(this.wrapping_add(0x30));
            let mut p = 0;
            if levels != 0 {
                p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[levels.wrapping_mul(4), 4]);
                if p == 0 {
                    h.call_sym(OPT_DTOR, Conv::Thiscall(opt), &[]);
                    return 0xd;
                }
                let (mut d, mut s) = (p, old);
                for _ in 0..keep {
                    if d != 0 {
                        let w: u32 = h.read(s);
                        h.write(d, w);
                        s = s.wrapping_add(4);
                    }
                    d = d.wrapping_add(4);
                }
            }
            let cur: u32 = h.read(this.wrapping_add(0x30));
            let raw: u32 = h.read(this.wrapping_add(0x2c));
            if cur != p && raw & NOT_OWNED == 0 && cur != 0 {
                free(h, cur);
            }
            h.write(this.wrapping_add(0x30), p);
            h.write(this.wrapping_add(0x2c), levels);
        }
        let heads: Vec<u32> = (0..n).map(|i| h.read(record(h, this, i))).collect();
        let mut marks: Vec<u32> = (0..nb).collect();
        let last = h.call_sym(OPT_LEVEL, Conv::Thiscall(opt), &[nb.wrapping_sub(1), 0]);
        for m in &mut marks {
            if *m == 0 {
                *m = last;
            } else if *m == last {
                *m = 0;
            }
        }
        // out-of-range answers read the original's frame; here they read 0
        let pick = |v: &[u32], i: u32| v.get(i as usize).copied().unwrap_or(0);
        for b in 0..nb {
            let rec = record(h, this, nb.wrapping_sub(1).wrapping_sub(b));
            let o = h.call_sym(OPT_NODE, Conv::Thiscall(opt), &[b]);
            h.write(rec, pick(&heads, o));
            let v = h.call_sym(OPT_LEVEL, Conv::Thiscall(opt), &[b, 0]);
            h.write(rec.wrapping_add(0x88), pick(&marks, v));
            let cnt = h.call_sym(OPT_INPUTS, Conv::Thiscall(opt), &[b]);
            h.write(rec.wrapping_add(4), cnt);
            let mut q = 0u32;
            while q < h.read::<u32>(rec.wrapping_add(4)) {
                let w = h.call_sym(OPT_INPUT, Conv::Thiscall(opt), &[b, q]);
                h.write(rec.wrapping_add(8 + 4 * q), pick(&marks, w));
                q += 1;
            }
        }
        h.call_sym(OPT_DTOR, Conv::Thiscall(opt), &[]);
        0
    }
}
