//! M5 stage 1, list `phyre-core`, batch 6 (`ffx_pcore::m5p_core_f`): the
//! thread pools' start-up (`PThreadPool::startThreads`,
//! `PThreadPoolWorkStealing::initialize`) and the work-stealing worker.
//!
//! Threads: start-up runs once on the main thread; `workerThread` is the
//! body of every work-stealing worker thread and runs there until the
//! pool stops it. It keeps no Rust state: its name buffer is a per-call
//! `StackFrame` (0x100 bytes, as the original's), the pool's state is the
//! game's, shared through the original semaphores and
//! `InterlockedCompareExchange`.

use super::{StackFrame, mem, ported};
use ffx_pcore::m5p_core_f::bridge as b;

ported! {
    /// `Phyre_PThreadPool_startThreads(pool; stack, priority)` -> 0, 0xd or the thread error; the initialisation and name builder in a per-call frame.
    Phyre_PThreadPool_startThreads @ 0x0044_ce70 => extern "thiscall" fn start_threads(this: u32, stack: u32, priority: u32) -> u32 {
        let mut frame = StackFrame::<9>::new();
        b::start_threads(&mut mem(), this, stack, priority, frame.at())
    }
    /// `Phyre_PThreadPoolWorkStealing_initialize(pool)`; the timeline names in a per-call frame.
    Phyre_PThreadPoolWorkStealing_initialize @ 0x0057_d4f0 => extern "thiscall" fn initialize(this: u32) -> u32 {
        let mut frame = StackFrame::<0x40>::new();
        b::initialize(&mut mem(), this, frame.at())
    }
    /// `Phyre_PThreadPoolWorkStealing_workerThread(pool; record)` -> 0; the thread's name in a per-call frame.
    Phyre_PThreadPoolWorkStealing_workerThread @ 0x0057_da20 => extern "thiscall" fn worker_thread(this: u32, rec: u32) -> u32 {
        let mut frame = StackFrame::<0x40>::new();
        b::worker_thread(&mut mem(), this, rec, frame.at())
    }
}
