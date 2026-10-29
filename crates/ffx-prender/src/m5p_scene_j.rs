//! M5 stage 1, list `scene`, batch 10: the light sort's heap sort step
//! (MSVC's `_Sort_heap` over 8-byte records with a predicate) and the
//! visibility check's occluder job set-up. Conventions as in
//! [`crate::m5p_scene_b`]; frames as in [`crate::m5p_scene_f`].
//!
//! Image addresses are rebased before use. Nothing here keeps state of
//! its own (worker threads: per-call scratch only).

use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::PHYRE_FREE;

/// `0x004d10d0(first, hole, len, &value, pred)` (cdecl): the heap's
/// adjust (`_Adjust_heap`).
pub const ADJUST_HEAP: u32 = 0x004d_10d0;
/// `Phyre_PPreprocessRingBuffer_GetSingleton` (cdecl) and its
/// `allocate(size)` (thiscall, `ret 4`).
pub const RING: u32 = 0x005c_d010;
pub const RING_ALLOCATE: u32 = 0x005c_d0f0;
/// `0x00a30260` (cdecl, six words): the occluder job's fill.
pub const OCCLUDER_FILL: u32 = 0x00a3_0260;

pub mod bridge {
    //! The ports over game memory.

    use super::{ADJUST_HEAP, Conv, Host, OCCLUDER_FILL, PHYRE_FREE, RING, RING_ALLOCATE};

    const EXTERNAL: u32 = 0x8000_0000;

    /// `0x004d2360` (cdecl `(first, last, pred)`): the heap `[first,
    /// last)` of 8-byte records sorted: while two or more are left, the
    /// last record taken out (to the frame, `ebp-8`), the top moved to its
    /// place and the taken one sifted in (`_Adjust_heap(first, 0, len - 1,
    /// &taken, pred)`). Nothing returned (the original's `eax` is the last
    /// adjust's, or its caller's for a short heap).
    pub fn sort_heap(h: &mut impl Host, first: u32, last: u32, pred: u32, frame: u32) {
        let mut end = last;
        loop {
            let bytes = end.wrapping_sub(first);
            #[allow(clippy::cast_possible_wrap)] // the original's signed compare
            if ((bytes & !7) as i32) <= 8 {
                return;
            }
            end = end.wrapping_sub(8);
            let a: u32 = h.read(end);
            let b: u32 = h.read(end + 4);
            h.write(frame, a);
            h.write(frame + 4, b);
            let t0: u32 = h.read(first);
            let t1: u32 = h.read(first + 4);
            h.write(end, t0);
            h.write(end + 4, t1);
            #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)] // sar
            let len = ((bytes.wrapping_sub(8) as i32) >> 3) as u32;
            h.call_sym(ADJUST_HEAP, Conv::Cdecl, &[first, 0, len, frame, pred]);
        }
    }

    /// `0x009f2f20` (thiscall `(world, n, context)`, `ret 0xc`): an
    /// occluder job from the preprocess ring buffer (0x10 bytes of header,
    /// then `n` 8-byte entries): its list link cleared, the block noted,
    /// its entry array (`+8`) set to the entries (external, `n` of them;
    /// an owned old buffer other than them freed: dead, as the header was
    /// just zeroed) unless `n` is negative;
    /// filled ([`OCCLUDER_FILL`] with the camera `+0x34`, else `+0`, and
    /// `+0x30`); appended to the list ending at `+0x2c`. The old tail.
    pub fn occluder_job(h: &mut impl Host, this: u32, world: u32, n: u32, context: u32) -> u32 {
        let size = n.wrapping_mul(8).wrapping_add(0x10);
        let ring = h.call_sym(RING, Conv::Cdecl, &[]);
        let block = h.call_sym(RING_ALLOCATE, Conv::Thiscall(ring), &[size]);
        let job = block.wrapping_add(0x10);
        if job != 0 {
            h.write(job + 8, 0u32);
            h.write(job + 0xc, 0u32);
        }
        let entries = job.wrapping_add(0x10);
        h.write(job.wrapping_add(4), block);
        h.write(job, 0u32);
        let array = job.wrapping_add(8);
        if n & EXTERNAL == 0 {
            let old: u32 = h.read(array + 4);
            let header: u32 = h.read(array);
            if old != entries && header & EXTERNAL == 0 && old != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
            }
            h.write(array + 4, entries);
            h.write(array, n | EXTERNAL);
        }
        let mut camera: u32 = h.read(this + 0x34);
        if camera == 0 {
            camera = h.read(this);
        }
        let extra: u32 = h.read(this + 0x30);
        h.call_sym(
            OCCLUDER_FILL,
            Conv::Cdecl,
            &[array, world, n, context, camera, extra],
        );
        let tail: u32 = h.read(this + 0x2c);
        h.write(tail, job);
        h.write(this + 0x2c, job);
        tail
    }
}
