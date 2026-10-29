//! M5 stage 1, list `phyre-core`, batch 5 (`ffx_pcore::m5p_core_e`):
//! `PThreadPoolWorkStealing`'s constructor, destructors, job submission,
//! flush, wait and shutdown, and the profile timeline's vector deleting
//! destructor.
//!
//! Threads: submission, flush and wait run on the threads that queue
//! jobs (main thread, render); the workers only read the records. The
//! pool's own locking is the originals' semaphores, called as before. No
//! shim keeps state.

use super::{mem, ported};
use ffx_pcore::m5p_core_e::bridge as b;

ported! {
    /// `Phyre_PThreadPoolWorkStealing_PThreadPoolWorkStealing(this; a, b)` -> `this`.
    Phyre_PThreadPoolWorkStealing_PThreadPoolWorkStealing @ 0x0057_d100 => extern "thiscall" fn ctor(this: u32, a: u32, v: u32) -> u32 {
        b::ctor(&mut mem(), this, a, v)
    }
    /// `Phyre_PThreadPoolWorkStealing_dtor_PThreadPoolWorkStealing(this)`.
    Phyre_PThreadPoolWorkStealing_dtor_PThreadPoolWorkStealing @ 0x0057_d1f0 => extern "thiscall" fn dtor(this: u32) -> u32 {
        b::dtor(&mut mem(), this)
    }
    /// `Phyre_PThreadPoolWorkStealing_scalarDeletingDtor(this; flags)` -> `this`.
    Phyre_PThreadPoolWorkStealing_scalarDeletingDtor @ 0x0057_d310 => extern "thiscall" fn delete(this: u32, flags: u32) -> u32 {
        b::delete(&mut mem(), this, flags)
    }
    /// `Phyre_PProfiling_PProcessorProfileTimelineCpu_vectorDeletingDtor(this; flags)` -> the block.
    Phyre_PProfiling_PProcessorProfileTimelineCpu_vectorDeletingDtor @ 0x0057_d280 => extern "thiscall" fn timeline_vector_delete(this: u32, flags: u32) -> u32 {
        b::timeline_vector_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PThreadPoolWorkStealing_submit_0057d3e0(this; job)`.
    Phyre_PThreadPoolWorkStealing_submit_0057d3e0 @ 0x0057_d3e0 => extern "thiscall" fn submit(this: u32, job: u32) -> u32 {
        b::submit(&mut mem(), this, job)
    }
    /// `Phyre_PThreadPoolWorkStealing_trySubmit_0057d8d0(this; job)` -> 0 or 0xe.
    Phyre_PThreadPoolWorkStealing_trySubmit_0057d8d0 @ 0x0057_d8d0 => extern "thiscall" fn try_submit(this: u32, job: u32) -> u32 {
        b::try_submit(&mut mem(), this, job)
    }
    /// `Phyre_PThreadPoolWorkStealing_wait_0057d970(this; job)`.
    Phyre_PThreadPoolWorkStealing_wait_0057d970 @ 0x0057_d970 => extern "thiscall" fn wait(this: u32, job: u32) -> u32 {
        b::wait(&mut mem(), this, job)
    }
    /// `Phyre_PThreadPoolWorkStealing_flush_0057d740(this)` -> 0.
    Phyre_PThreadPoolWorkStealing_flush_0057d740 @ 0x0057_d740 => extern "thiscall" fn flush(this: u32) -> u32 {
        b::flush(&mut mem(), this)
    }
    /// `Phyre_PThreadPoolWorkStealing_shutdown_0057d7e0(this)`.
    Phyre_PThreadPoolWorkStealing_shutdown_0057d7e0 @ 0x0057_d7e0 => extern "thiscall" fn shutdown(this: u32) -> u32 {
        b::shutdown(&mut mem(), this)
    }
}
