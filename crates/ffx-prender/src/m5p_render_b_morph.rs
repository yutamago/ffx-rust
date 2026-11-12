//! M5 stage 1, list `phyre-render-b`: the dynamic geometry's morph
//! blends, one generic port for the nine copies (`StaticInit_Morph<K><N>`
//! register them): `(weights, n, streams, out)` (cdecl) blends `K`
//! weighted source streams of `n` vertices of `N` floats into `*out`.
//! The weights that are not 0 (a NaN counts) are gathered with their
//! streams into locals (`ebp - wl`, `ebp - sl`), which the 2-, 3- and
//! 4-way blends get by address; one weight scales its stream (or
//! copies it for 1.0); none zeroes the output.

use ffx_bridge::x87;

/// `crt_memcpy_thunk`, the CRT's `memset`, and the 2-, 3-, 4-way blends
/// `(&weights, count, &streams, out)` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
pub const MEMSET: u32 = 0x0094_964c;
pub const BLEND2: u32 = 0x005c_b870;
pub const BLEND3: u32 = 0x005c_b8d0;
pub const BLEND4: u32 = 0x005c_b940;

/// One copy: `K` weights, `N` floats a vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Morph {
    pub k: u32,
    pub n: u32,
}

/// The copies `(address, shape)`.
pub const MORPHS: [(u32, Morph); 9] = [
    (0x005c_9f50, Morph { k: 2, n: 2 }),
    // not in the dump (`symbols/sizes.tsv`); M5 finish wp5b
    (0x005c_a0d0, Morph { k: 2, n: 3 }),
    (0x005c_a250, Morph { k: 2, n: 4 }),
    (0x005c_a3e0, Morph { k: 3, n: 2 }),
    (0x005c_a5b0, Morph { k: 3, n: 3 }),
    (0x005c_a780, Morph { k: 3, n: 4 }),
    (0x005c_a950, Morph { k: 4, n: 2 }),
    (0x005c_ab40, Morph { k: 4, n: 3 }),
    (0x005c_ad30, Morph { k: 4, n: 4 }),
];

/// The [`MORPHS`] shape of `f`.
#[must_use]
pub const fn morph_of(f: u32) -> Morph {
    let mut i = 0;
    while i < MORPHS.len() {
        if MORPHS[i].0 == f {
            return MORPHS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// Where a copy keeps its gathered weights and streams below `ebp`.
#[must_use]
pub const fn locals(k: u32) -> (u32, u32) {
    match k {
        2 => (0x10, 0x8),
        3 => (0x10, 0x1c),
        _ => (0x14, 0x24),
    }
}

pub mod bridge {
    //! The original over game memory; `ebp` is where the original's frame
    //! pointer is.

    use super::{BLEND2, BLEND3, BLEND4, MEMCPY, MEMSET, Morph, locals, x87};
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// One morph blend `(weights, n, streams, out)` of shape `m`; the
    /// `eax` the original leaves (the table's callers may read it): the
    /// callee's, or what the scale loop leaves (`dst - src`, or the
    /// destination's end when the unrolled loop covers it), 0 for no
    /// vertices (the original's is the stack cookie's).
    #[allow(clippy::float_cmp)] // the original's `fucomp` with 1.0
    pub fn morph(
        h: &mut impl Host,
        ebp: u32,
        m: Morph,
        weights: u32,
        n: u32,
        streams: u32,
        out: u32,
    ) -> u32 {
        if n == 0 {
            return 0;
        }
        let (wl, sl) = locals(m.k);
        let (wl, sl) = (ebp - wl, ebp - sl);
        let mut count = 0u32;
        for j in 0..m.k {
            let w: u32 = h.read(weights + 4 * j);
            if x87::load(w) == 0.0 {
                continue;
            }
            let s: u32 = h.read(streams + 4 * j);
            h.write(sl + 4 * count, s);
            let q = x87::store(x87::load(w));
            h.write(wl + 4 * count, q);
            count += 1;
        }
        let total = n.wrapping_mul(m.n);
        let dst: u32 = h.read(out);
        match count {
            0 => cdecl(h, MEMSET, &[dst, 0, total.wrapping_mul(4)]),
            1 => {
                let w = x87::load(h.read(wl));
                let src: u32 = h.read(sl);
                if w == 1.0 {
                    return cdecl(h, MEMCPY, &[dst, src, total.wrapping_mul(4)]);
                }
                for i in 0..total {
                    let v = x87::load(h.read(src.wrapping_add(4 * i)));
                    h.write(dst.wrapping_add(4 * i), x87::store(x87::mul(v, w)));
                }
                // the unrolled loop runs total / 4 times (none below 4)
                let done = if total >= 4 { total & !3 } else { 0 };
                if done < total {
                    dst.wrapping_sub(src)
                } else {
                    dst.wrapping_add(4 * done)
                }
            }
            c => {
                let f = match c {
                    2 => BLEND2,
                    3 => BLEND3,
                    _ => BLEND4,
                };
                cdecl(h, f, &[wl, total, sl, out])
            }
        }
    }
}
