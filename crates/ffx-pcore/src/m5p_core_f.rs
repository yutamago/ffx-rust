//! M5 stage 1, list `phyre-core`, batch 6: the thread pools' start-up and
//! the work-stealing worker.
//!
//! - `PThreadPool::startThreads(stack, priority)`: the thread records
//!   (0x18 bytes: pool, scratch, thread, started flag, index and count,
//!   stop flag), the job pointers, the per-thread scratch, then one thread
//!   per record (`Phyre_PThreadWin32` `0x004300b0`, which stays original:
//!   it raises the debugger's thread-name exception in its own SEH scope);
//! - `PThreadPoolWorkStealing::initialize()`: the profile timelines, the
//!   range records, the four semaphores, the second job buffer, then
//!   `startThreads` with the configured stack size and priority;
//! - `PThreadPoolWorkStealing::workerThread(record)`: per generation
//!   parity, waits for the run semaphore, claims jobs from its own slice
//!   and then the others' with `InterlockedCompareExchange`, runs them,
//!   and signals done; until the record's stop flag.

/// `Phyre_PhyreMalloc(size)` (cdecl, unaligned), `PhyreMalloc(size,
/// align)`, `operator new[](size)` and the CRT's `memset` (cdecl).
pub const PHYRE_MALLOC_PLAIN: u32 = 0x0042_fb40;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
pub const MEMSET: u32 = 0x0094_964c;
/// `eh vector constructor iterator(p, size, n, ctor, dtor)` (stdcall).
pub const VECTOR_CTOR: u32 = 0x0094_966a;
/// The thread record's constructor and destructor, the thread entry.
pub const RECORD_CTOR: u32 = 0x0044_cc20;
pub const RECORD_DTOR: u32 = 0x0044_cc70;
pub const THREAD_ENTRY: u32 = 0x0044_ce30;
/// `Phyre_PThreadInitializationWin32(init; entry)` and
/// `Phyre_PThreadWin32` `0x004300b0(thread; &init)` (thiscall).
pub const THREAD_INIT: u32 = 0x0042_fca0;
pub const THREAD_CREATE: u32 = 0x0043_00b0;
/// `PStringBuilder`: from a text, `+= int`, destructor (thiscall); the
/// threads' name.
pub const BUILDER_NEW: u32 = 0x0044_49e0;
pub const BUILDER_APPEND_INT: u32 = 0x0044_4b90;
pub const BUILDER_DTOR: u32 = 0x0044_4ab0;
pub const THREAD_NAME: u32 = 0x00b1_0270;
/// `sprintf_s`'s import slot (cdecl) and the timeline and worker name
/// formats.
pub const SPRINTF_S: u32 = 0x00b0_c394;
pub const TIMELINE_NAME: u32 = 0x00b3_140c;
pub const WORKER_NAME: u32 = 0x00b3_141c;
/// The profile timeline's constructor and destructor (for the vector
/// constructor), its `initialize(tl; name, size)` and the collection's
/// `addTimeline(collection; &tl)` (thiscall).
pub const TIMELINE_CTOR: u32 = 0x0057_d0e0;
pub const TIMELINE_DTOR: u32 = 0x0057_d1d0;
pub const TIMELINE_INIT: u32 = 0x0060_ed70;
pub const TIMELINES: u32 = 0x00c2_e2cc;
pub const ADD_TIMELINE: u32 = 0x0060_ed40;
/// `Phyre_PSemaphoreWin32_initialize(sem; initial, max)`, `_wait(sem)`
/// and `_signal(sem; n, prev)` (thiscall).
pub const SEMAPHORE_INIT: u32 = 0x0043_0080;
pub const SEMAPHORE_WAIT: u32 = 0x0043_02b0;
pub const SEMAPHORE_SIGNAL: u32 = 0x0043_0210;
/// `Phyre_PThreadPool_startThreads(pool; stack, priority)` and the
/// configured stack size and priority it is called with.
pub const START_THREADS: u32 = 0x0044_ce70;
pub const STACK_SIZE: u32 = 0x00c0_b954;
pub const PRIORITY: u32 = 0x00c0_b958;
/// KERNEL32 `InterlockedCompareExchange`'s import slot (stdcall).
pub const COMPARE_EXCHANGE: u32 = 0x00b0_c12c;

/// `n * size + 4`, saturated at `u32::MAX` (the array new's count header).
#[must_use]
pub const fn array_bytes(n: u32, size: u32) -> u32 {
    match n.checked_mul(size) {
        Some(b) => match b.checked_add(4) {
            Some(s) => s,
            None => u32::MAX,
        },
        None => u32::MAX,
    }
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn thiscall(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, buf: u32, fmt: u32, v: u32) {
        let f: u32 = h.read(h.rebase(SPRINTF_S));
        let fmt = h.rebase(fmt);
        h.call(f, Conv::Cdecl, &[buf, 0x100, fmt, v]);
    }

    /// Zeroes the `[at]` words at `[base]` (the count re-read each step).
    fn zero_words(h: &mut impl Host, base: u32, count: u32) {
        let mut i = 0u32;
        while i < h.read::<u32>(count) {
            let p: u32 = h.read(base);
            h.write(p.wrapping_add(4 * i), 0u32);
            i += 1;
        }
    }

    /// `Phyre_PThreadPool_startThreads(pool; stack, priority)` -> 0, 0xd
    /// when an allocation fails, or the first thread creation's error.
    /// The frame at `frame` holds the thread initialisation (6 words, the
    /// original's `ebp - 0x3c`) and the name builder (3 words).
    pub fn start_threads(
        h: &mut impl Host,
        this: u32,
        stack: u32,
        priority: u32,
        frame: u32,
    ) -> u32 {
        let n: u32 = h.read(this + 8);
        if n != 0 {
            let block = cdecl(h, PHYRE_MALLOC_PLAIN, &[array_bytes(n, 0x18)]);
            let records = if block == 0 {
                0
            } else {
                h.write(block, n);
                let ctor = h.rebase(RECORD_CTOR);
                let dtor = h.rebase(RECORD_DTOR);
                h.call_sym(
                    VECTOR_CTOR,
                    Conv::Stdcall,
                    &[block + 4, 0x18, n, ctor, dtor],
                );
                block + 4
            };
            h.write(this + 4, records);
            if records == 0 {
                return 0xd;
            }
        }
        let jobs: u32 = h.read(this + 0x14);
        if jobs != 0 {
            let p = cdecl(h, PHYRE_MALLOC, &[jobs << 2, 4]);
            h.write(this + 0x18, p);
            if p == 0 {
                return 0xd;
            }
            zero_words(h, this + 0x18, this + 0x14);
        }
        if stack != 0 {
            let workers: u32 = h.read(this + 8);
            let p = cdecl(h, PHYRE_MALLOC, &[workers.wrapping_mul(stack), 0x80]);
            h.write(this + 0xc, p);
            if p == 0 {
                return 0xd;
            }
        }
        h.write(this + 0x10, stack);
        let init = frame;
        let builder = frame + 0x18;
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) {
            let rec = h.read::<u32>(this + 4).wrapping_add(0x18 * i);
            h.write(rec, this);
            let scratch = h
                .read::<u32>(this + 0xc)
                .wrapping_add(stack.wrapping_mul(i));
            h.write(rec.wrapping_add(4), scratch);
            h.write(rec.wrapping_add(0x10), i as u16);
            let count: u16 = h.read(this + 8);
            h.write(rec.wrapping_add(0x12), count);
            let entry = h.rebase(THREAD_ENTRY);
            thiscall(h, THREAD_INIT, init, &[entry]);
            h.write(init + 0xc, 2u32);
            h.write(init + 4, priority);
            let name = h.rebase(THREAD_NAME);
            thiscall(h, BUILDER_NEW, builder, &[name]);
            thiscall(h, BUILDER_APPEND_INT, builder, &[i]);
            let len: u32 = h.read(builder + 8);
            let text = if len == 0 { 0 } else { h.read(builder + 4) };
            h.write(init + 0x10, text);
            h.write(init + 8, rec);
            h.write(init + 0x14, 1u32);
            let r = thiscall(h, THREAD_CREATE, rec.wrapping_add(8), &[init]);
            h.write(rec.wrapping_add(0xc), 1u8);
            thiscall(h, BUILDER_DTOR, builder, &[]);
            if r != 0 {
                return r;
            }
            i += 1;
        }
        0
    }

    /// `Phyre_PThreadPoolWorkStealing_initialize(pool)` -> 0, 7 without
    /// workers or job room, 0xd when an allocation fails, a timeline's
    /// error, or `startThreads`'s result. The timelines' name goes through
    /// the 0x100-byte frame buffer at `name`.
    pub fn initialize(h: &mut impl Host, this: u32, name: u32) -> u32 {
        let n: u32 = h.read(this + 8);
        if n == 0 || h.read::<u32>(this + 0x14) == 0 {
            return 7;
        }
        let block = cdecl(h, OPERATOR_NEW_ARRAY, &[array_bytes(n, 4)]);
        let tls = if block == 0 {
            0
        } else {
            h.write(block, n);
            let ctor = h.rebase(TIMELINE_CTOR);
            let dtor = h.rebase(TIMELINE_DTOR);
            h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[block + 4, 4, n, ctor, dtor]);
            block + 4
        };
        h.write(this + 0x1c, tls);
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) {
            sprintf(h, name, TIMELINE_NAME, i);
            let tl = h.read::<u32>(this + 0x1c).wrapping_add(4 * i);
            let r = thiscall(h, TIMELINE_INIT, tl, &[name, 0x2000]);
            if r != 0 {
                return r;
            }
            let tl = h.read::<u32>(this + 0x1c).wrapping_add(4 * i);
            let c = h.rebase(TIMELINES);
            let r = thiscall(h, ADD_TIMELINE, c, &[tl]);
            if r != 0 {
                return r;
            }
            i += 1;
        }
        let workers: u32 = h.read(this + 8);
        let records = cdecl(h, PHYRE_MALLOC, &[workers.wrapping_mul(0x18), 4]);
        h.write(this + 0x58, records);
        if records == 0 {
            return 0xd;
        }
        let workers: u32 = h.read(this + 8);
        h.write(this + 0x5c, records.wrapping_add(workers.wrapping_mul(12)));
        let workers: u32 = h.read(this + 8);
        cdecl(h, MEMSET, &[records, 0, workers.wrapping_mul(0x18)]);
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) {
            for at in [0x58, 0x5c] {
                let tl = h.read::<u32>(this + 0x1c).wrapping_add(4 * i);
                let r: u32 = h.read(this + at);
                h.write(r.wrapping_add(12 * i), tl);
            }
            i += 1;
        }
        for (sem, flag) in [(0x30, 0x34), (0x38, 0x3c), (0x20, 0x24), (0x28, 0x2c)] {
            let workers: u32 = h.read(this + 8);
            thiscall(h, SEMAPHORE_INIT, this + sem, &[0, workers]);
            h.write(this + flag, 1u8);
        }
        let jobs: u32 = h.read(this + 0x14);
        let p = cdecl(h, PHYRE_MALLOC, &[jobs << 2, 4]);
        h.write(this + 0x48, p);
        if p == 0 {
            return 0xd;
        }
        zero_words(h, this + 0x48, this + 0x14);
        h.write(this + 0x54, 0x101u16);
        let stack: u32 = h.read(h.rebase(STACK_SIZE));
        let priority: u32 = h.read(h.rebase(PRIORITY));
        thiscall(h, START_THREADS, this, &[stack, priority])
    }

    /// `Phyre_PThreadPoolWorkStealing_workerThread(pool; record)` -> 0:
    /// names itself (`sprintf_s` into the 0x100-byte frame buffer at
    /// `name`, never read), then until the record's stop flag (`+0x14`):
    /// for each parity, waits for its run semaphore (`+0x20 + 8 parity`),
    /// claims the jobs of each worker's slice in turn from its own
    /// (`InterlockedCompareExchange` on the slice's start), runs each
    /// (vtable `+4`, thiscall `(job; scratch, profiler, index)`; a null job
    /// ends that slice), and signals the parity's done semaphore once.
    pub fn worker_thread(h: &mut impl Host, this: u32, rec: u32, name: u32) -> u32 {
        let index = u32::from(h.read::<u16>(rec + 0x10));
        let scratch: u32 = h.read(rec + 4);
        let workers = u32::from(h.read::<u16>(rec + 0x12));
        let profiler: u32 = h.read(this + 0x10);
        sprintf(h, name, WORKER_NAME, index);
        let cas: u32 = h.read(h.rebase(COMPARE_EXCHANGE));
        loop {
            for parity in 0..2u32 {
                thiscall(h, SEMAPHORE_WAIT, this + 0x20 + 8 * parity, &[]);
                if h.read::<u8>(rec + 0x14) != 0 {
                    return 0;
                }
                let records: u32 = h.read(this + 0x58 + 4 * parity);
                for k in 0..workers {
                    let slot = (k.wrapping_add(index)) % workers;
                    let r = records.wrapping_add(12 * slot);
                    let start = r.wrapping_add(4);
                    'slice: loop {
                        let end: u32 = h.read(r.wrapping_add(8));
                        let mut cur: u32 = h.read(start);
                        loop {
                            if cur >= end {
                                break 'slice;
                            }
                            let prev =
                                h.call(cas, Conv::Stdcall, &[start, cur.wrapping_add(4), cur]);
                            if prev == cur {
                                break;
                            }
                            cur = h.read(start);
                        }
                        let job: u32 = h.read(cur);
                        if job == 0 {
                            break 'slice;
                        }
                        let vt: u32 = h.read(job);
                        let f: u32 = h.read(vt.wrapping_add(4));
                        h.call(f, Conv::Thiscall(job), &[scratch, profiler, index]);
                    }
                }
                thiscall(h, SEMAPHORE_SIGNAL, this + 0x30 + 8 * parity, &[1, 0]);
            }
            if h.read::<u8>(rec + 0x14) != 0 {
                return 0;
            }
        }
    }
}
