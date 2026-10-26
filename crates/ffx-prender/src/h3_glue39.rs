//! The render-side glue, thirty-ninth batch (M4 port round 9, list
//! `h3-glue-render`): the scene's VFX render arrays merged at the end of
//! a frame. The original's 0x8014-byte frame (bucket heads and tails, a
//! copy of the draw list) holds nothing a callee sees: the port keeps it
//! in vectors, and its `memset`/`memcpy` calls are Rust copies.

use ffx_bridge::Global;

/// `PhyreScene_GetVFXSortMode(this)` (0, 1 or 2), the CRT's `qsort`
/// (cdecl, its import slot), the depth comparator
/// `VFXGeometry_CompareDepth` and the depth scale (`double`) of the
/// bucket sort.
pub const SORT_MODE: u32 = 0x0067_21b0;
pub const QSORT: u32 = 0x00b0_c3ac;
pub const COMPARE_DEPTH: u32 = 0x0065_9140;
pub const DEPTH_SCALE: Global<u64> = Global::new(0x00b4_3d20);

/// The scene's fields: the draw list (`+0x38cc`, count `+0xf8cc`), the
/// geometry list (`+0x68cc`, count `+0xf8d0`), the bucket sort's pairs
/// (`+0x98cc`), the segments' ends of both lists (`+0xf8d4`, `+0xf914`,
/// count `+0xf954`) and the "sort" byte `+0x10d2c`.
pub const DRAWS: u32 = 0x38cc;
pub const GEOMETRIES: u32 = 0x68cc;
pub const PAIRS: u32 = 0x98cc;
pub const DRAW_COUNT: u32 = 0xf8cc;
pub const GEOMETRY_COUNT: u32 = 0xf8d0;
pub const DRAW_ENDS: u32 = 0xf8d4;
pub const GEOMETRY_ENDS: u32 = 0xf914;
pub const SEGMENTS: u32 = 0xf954;
pub const SORT: u32 = 0x1_0d2c;
/// The draw list's room (words), the original's copy buffer.
pub const ROOM: u32 = 0xc00;
/// The bucket sort's buckets.
pub const BUCKETS: u32 = 0x1000;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The bucket sort of geometries `start..end` (mode 1): each one's
    /// bucket is its depth (`+0xdc`, 0 for none) times the scale, toward
    /// zero (`_ftol2_sse`), clamped to 0..0xfff; the pairs (geometry,
    /// next) chain each bucket in order; the geometries are written back
    /// bucket by bucket.
    fn bucket_sort(h: &mut impl Host, this: u32, start: u32, end: u32) {
        let mut heads = vec![0u32; BUCKETS as usize];
        let mut tails = vec![0u32; BUCKETS as usize];
        let scale = f64::from_bits(DEPTH_SCALE.get(h));
        let mut src = this + GEOMETRIES + 4 * start;
        let mut pair = this + PAIRS;
        for _ in 0..end.wrapping_sub(start) {
            let geometry: u32 = h.read(src);
            let depth = if geometry == 0 {
                0.0
            } else {
                x87::mul(x87::load(h.read(geometry + 0xdc)), scale)
            };
            let bucket = (x87::ftol(h, depth) as i32).clamp(0, BUCKETS as i32 - 1) as usize;
            h.write(pair, geometry);
            h.write(pair + 4, 0u32);
            if heads[bucket] == 0 {
                heads[bucket] = pair;
            } else {
                h.write(tails[bucket] + 4, pair);
            }
            tails[bucket] = pair;
            src += 4;
            pair += 8;
        }
        let mut at = start;
        for &head in &heads {
            let mut p = head;
            while p != 0 {
                let geometry: u32 = h.read(p);
                h.write(this + GEOMETRIES + 4 * at, geometry);
                p = h.read(p + 4);
                at = at.wrapping_add(1);
            }
        }
    }

    /// `PhyreScene_MergeVFXRenderArrays(this)` (thiscall, no stack
    /// words; its eax is not read): the current draw and geometry counts
    /// close a segment; with the "sort" byte set (then cleared), each
    /// segment's geometries are sorted by depth, by `qsort` (mode 2) or
    /// the bucket sort (mode 1). Then the draw list is rebuilt from a
    /// copy of itself: per segment its draws, then its geometries' draws
    /// (`+0xc`, 0 for none), up to the list's room (a segment whose draws
    /// would pass it ends the merge; geometries are skipped once the old
    /// count reached it). The new count is stored; the segments and the
    /// geometry count cleared.
    #[allow(clippy::cast_possible_wrap)] // signed counts, as compared
    pub fn merge_vfx_render_arrays(h: &mut impl Host, this: u32) {
        let mode = h.call_sym(SORT_MODE, Conv::Thiscall(this), &[]);
        let n: u32 = h.read(this + SEGMENTS);
        let draws: u32 = h.read(this + DRAW_COUNT);
        h.write(this + DRAW_ENDS + 4 * n, draws);
        let n: u32 = h.read(this + SEGMENTS);
        let geometries: u32 = h.read(this + GEOMETRY_COUNT);
        h.write(this + GEOMETRY_ENDS + 4 * n, geometries);
        h.write(this + SEGMENTS, n.wrapping_add(1));
        if h.read::<u8>(this + SORT) != 0 {
            h.write(this + SORT, 0u8);
            let mut c = 0u32;
            while (c as i32) < (h.read::<u32>(this + SEGMENTS) as i32) {
                let start = if c == 0 {
                    0
                } else {
                    h.read(this + GEOMETRY_ENDS + 4 * c - 4)
                };
                let end: u32 = h.read(this + GEOMETRY_ENDS + 4 * c);
                if (start as i32) < (end as i32) {
                    if mode == 2 {
                        let qsort: u32 = h.read(h.rebase(QSORT));
                        let compare = h.rebase(COMPARE_DEPTH);
                        let base = this + GEOMETRIES + 4 * start;
                        h.call(qsort, Conv::Cdecl, &[base, end - start, 4, compare]);
                    } else if mode == 1 {
                        bucket_sort(h, this, start, end);
                    }
                }
                c += 1;
            }
        }
        // the copy of the draw list (the original's frame buffer at
        // ebp - 0x3014)
        let count: u32 = h.read(this + DRAW_COUNT);
        let mut copy = vec![0u32; ROOM as usize];
        for (k, w) in copy.iter_mut().enumerate().take(count.min(ROOM) as usize) {
            *w = h.read(this + DRAWS + 4 * k as u32);
        }
        let mut out = 0u32;
        let mut c = 0u32;
        while (c as i32) < (h.read::<u32>(this + SEGMENTS) as i32) {
            let (a, b) = if c == 0 {
                (0, 0)
            } else {
                (
                    h.read(this + DRAW_ENDS + 4 * c - 4),
                    h.read(this + GEOMETRY_ENDS + 4 * c - 4),
                )
            };
            let a_end: u32 = h.read(this + DRAW_ENDS + 4 * c);
            let b_end: u32 = h.read(this + GEOMETRY_ENDS + 4 * c);
            if (a as i32) < (a_end as i32) {
                let len = a_end - a;
                let old: u32 = h.read(this + DRAW_COUNT);
                if (len.wrapping_add(old) as i32) >= ROOM as i32 {
                    break;
                }
                for k in 0..len {
                    // junk read: [ebp-0x3014 + 4 * index] never written
                    // past the old count; 0 (brief 2026-10-24)
                    let w = copy.get((a + k) as usize).copied().unwrap_or(0);
                    h.write(this + DRAWS + 4 * (out + k), w);
                }
                out += len;
            }
            let mut b = b;
            while (b as i32) < (b_end as i32) {
                if (h.read::<u32>(this + DRAW_COUNT) as i32) >= ROOM as i32 {
                    break;
                }
                let geometry: u32 = h.read(this + GEOMETRIES + 4 * b);
                let draw = if geometry == 0 {
                    0
                } else {
                    h.read(geometry + 0xc)
                };
                h.write(this + DRAWS + 4 * out, draw);
                b += 1;
                out += 1;
            }
            c += 1;
        }
        h.write(this + DRAW_COUNT, out);
        h.write(this + SEGMENTS, 0u32);
        h.write(this + GEOMETRY_COUNT, 0u32);
    }
}
