//! M5 finish, WP5 part 5 (wp5f): the last `FFX.exe` frontier: dlmalloc's
//! heap growth and release (`ffx_sg::m5main4_lib`). Parity tests:
//! `crates/ffx-oracle/tests/m5_main4.rs` (`dlmalloc_*`).
//!
//! Threads: the heaps are the game's `mspace`s (the cloth heap on the
//! cloth workers, the graphics heap on the main thread). No shim keeps
//! state; the region queries' buffers are `StackFrame`s (7 words).

use super::{StackFrame, mem, ported};
use ffx_sg::m5main4_lib::bridge as lib;

ported! {
    /// `init_top(m, p, psize)`.
    dlmalloc_init_top_006ea140 @ 0x006e_a140 => extern "C" fn init_top(m: u32, p: u32, psize: u32) {
        lib::init_top(&mut mem(), m, p, psize);
    }
    /// `segment_holding(m, p)` -> the segment record or 0.
    dlmalloc_segment_holding_006eb1a0 @ 0x006e_b1a0 => extern "C" fn segment_holding(m: u32, p: u32) -> u32 {
        lib::segment_holding(&mut mem(), m, p)
    }
    /// `mmap_alloc(m, nb)` -> the chunk's memory or 0.
    dlmalloc_mmap_alloc_006eaa30 @ 0x006e_aa30 => extern "C" fn mmap_alloc(m: u32, nb: u32) -> u32 {
        lib::mmap_alloc(&mut mem(), m, nb)
    }
    /// `prepend_alloc(m, newbase, oldbase, nb)` -> the chunk's memory.
    dlmalloc_prepend_alloc_006eab10 @ 0x006e_ab10 => extern "C" fn prepend_alloc(m: u32, newbase: u32, oldbase: u32, nb: u32) -> u32 {
        lib::prepend_alloc(&mut mem(), m, newbase, oldbase, nb)
    }
    /// `add_segment(m, tbase, tsize, mmapped)`.
    dlmalloc_add_segment_006e9ae0 @ 0x006e_9ae0 => extern "C" fn add_segment(m: u32, tbase: u32, tsize: u32, mmapped: u32) {
        lib::add_segment(&mut mem(), m, tbase, tsize, mmapped);
    }
    /// `sys_alloc(m, nb)` -> the chunk's memory or 0.
    dlmalloc_sys_alloc_006eb1d0 @ 0x006e_b1d0 => extern "C" fn sys_alloc(m: u32, nb: u32) -> u32 {
        lib::sys_alloc(&mut mem(), m, nb)
    }
    /// `sys_trim(m, pad)` -> 1 when memory went back; the region query in
    /// a frame.
    dlmalloc_sys_trim_006eb3e0 @ 0x006e_b3e0 => extern "C" fn sys_trim(m: u32, pad: u32) -> u32 {
        let mut f = StackFrame::<7>::new();
        lib::sys_trim(&mut mem(), m, pad, f.at())
    }
    /// `release_unused_segments(m)` -> the bytes released; the region
    /// query in a frame.
    dlmalloc_release_unused_segments_006eae70 @ 0x006e_ae70 => extern "C" fn release_unused_segments(m: u32) -> u32 {
        let mut f = StackFrame::<7>::new();
        lib::release_unused_segments(&mut mem(), m, f.at())
    }
    /// `tmalloc_large(m, nb)` -> the chunk's memory or 0.
    dlmalloc_tmalloc_large_006eb540 @ 0x006e_b540 => extern "C" fn tmalloc_large(m: u32, nb: u32) -> u32 {
        lib::tmalloc_large(&mut mem(), m, nb)
    }
}
