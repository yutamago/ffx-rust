//! M5 stage 1, list `phyre-core`, batch 5: `PThreadPoolWorkStealing`.
//!
//! The pool's layout (beyond `PThreadPool`'s): `+0x14` the job buffer's
//! capacity, `+0x18` and `+0x48` the two job buffers (by generation
//! parity), `+0x1c` the per-worker profile timelines, `+0x20`/`+0x28` and
//! `+0x30`/`+0x38` two semaphore pairs (8 bytes each: handle and flag),
//! `+0x40` the generation, `+0x44` the generation being run, `+0x4c` the
//! jobs queued, `+0x50` the jobs handed out, `+0x54` a started flag per
//! parity, `+0x58`/`+0x5c` the workers' range records per parity (12
//! bytes per worker: a word, then start and end of its slice).

/// `Phyre_PThreadPoolWorkStealing_vftable`.
pub const VTABLE: u32 = 0x00b3_13d8;
/// `Phyre_PThreadPool_PThreadPool(this; a, b)`, its destructor
/// `0x0044cc60` and body `0x0044d070` (thiscall).
pub const POOL_CTOR: u32 = 0x0044_cbe0;
pub const POOL_DTOR: u32 = 0x0044_cc60;
pub const POOL_DTOR_BODY: u32 = 0x0044_d070;
/// `eh vector constructor iterator(p, size, n, ctor, dtor)` and
/// `eh vector destructor iterator(p, size, n, dtor)` (stdcall); the
/// semaphore pair's constructor `0x0057d0f0` and destructor `0x0057d1e0`,
/// the timeline's destructor `0x0057d1d0`.
pub const VECTOR_CTOR: u32 = 0x0094_966a;
pub const VECTOR_DTOR: u32 = 0x0094_96d2;
pub const SEMAPHORE_PAIR_CTOR: u32 = 0x0057_d0f0;
pub const SEMAPHORE_PAIR_DTOR: u32 = 0x0057_d1e0;
pub const TIMELINE_ELEMENT_DTOR: u32 = 0x0057_d1d0;
/// The pool's shutdown `0x0057d7e0(this)` (thiscall).
pub const SHUTDOWN: u32 = 0x0057_d7e0;
/// `operator delete(p)`, `operator delete[](p)`, `Phyre_PhyreFree(p)`
/// (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The timeline base's destructor `0x0060ec70(this)` (thiscall).
pub const TIMELINE_DTOR: u32 = 0x0060_ec70;
/// `Phyre_PSemaphoreWin32_wait(sem)`, `Phyre_PSemaphoreWin32_signal(sem;
/// n, prev)`, `Phyre_CloseHandle_00430250(h)` (thiscall).
pub const SEMAPHORE_WAIT: u32 = 0x0043_02b0;
pub const SEMAPHORE_SIGNAL: u32 = 0x0043_0210;
pub const CLOSE: u32 = 0x0043_0250;
/// The profile timelines' collection and its
/// `removeTimeline(collection; &timeline)` (thiscall).
pub const TIMELINES: u32 = 0x00c2_e2cc;
pub const REMOVE_TIMELINE: u32 = 0x0060_ede0;

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

    fn vtable_call(h: &mut impl Host, this: u32, slot: u32) -> u32 {
        let vt: u32 = h.read(this);
        let f: u32 = h.read(vt.wrapping_add(slot));
        h.call(f, Conv::Thiscall(this), &[])
    }

    /// `Phyre_PThreadPoolWorkStealing_PThreadPoolWorkStealing(this; a, b)`
    /// -> `this`: the pool, its vtable, no timelines, the two semaphore
    /// pairs constructed, the counters zero, both started flags set.
    pub fn ctor(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        thiscall(h, POOL_CTOR, this, &[a, b]);
        let vt = h.rebase(VTABLE);
        h.write(this, vt);
        h.write(this + 0x1c, 0u32);
        let ctor = h.rebase(SEMAPHORE_PAIR_CTOR);
        let dtor = h.rebase(SEMAPHORE_PAIR_DTOR);
        h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[this + 0x20, 8, 2, ctor, dtor]);
        h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[this + 0x30, 8, 2, ctor, dtor]);
        for off in [0x40, 0x44, 0x48, 0x4c, 0x50] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x54, 0x101u16);
        h.write(this + 0x5c, 0u32);
        h.write(this + 0x58, 0u32);
        this
    }

    /// The destructor's body: the vtable, the shutdown, the semaphore
    /// pairs destroyed (`+0x30` first), `PThreadPool`'s destructor's `eax`.
    fn dtor_body(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(VTABLE);
        h.write(this, vt);
        thiscall(h, SHUTDOWN, this, &[]);
        let dtor = h.rebase(SEMAPHORE_PAIR_DTOR);
        h.call_sym(VECTOR_DTOR, Conv::Stdcall, &[this + 0x30, 8, 2, dtor]);
        h.call_sym(VECTOR_DTOR, Conv::Stdcall, &[this + 0x20, 8, 2, dtor]);
        thiscall(h, POOL_DTOR, this, &[])
    }

    /// `0x0057d1f0(this)` -> `PThreadPool`'s destructor's `eax`.
    pub fn dtor(h: &mut impl Host, this: u32) -> u32 {
        dtor_body(h, this)
    }

    /// The scalar deleting destructor `0x0057d310(this; flags)` -> `this`.
    pub fn delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        dtor_body(h, this);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `PProcessorProfileTimelineCpu`'s vector deleting destructor
    /// `0x0057d280(this; flags)` -> the block: with bit 1 the array (its
    /// count in the word before it) destroyed element by element and, with
    /// bit 0, `operator delete[]`'d; else the one timeline, `operator
    /// delete`'d with bit 0.
    pub fn timeline_vector_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        if flags & 2 != 0 {
            let block = this.wrapping_sub(4);
            let n: u32 = h.read(block);
            let dtor = h.rebase(TIMELINE_ELEMENT_DTOR);
            h.call_sym(VECTOR_DTOR, Conv::Stdcall, &[this, 4, n, dtor]);
            if flags & 1 != 0 {
                cdecl(h, OPERATOR_DELETE_ARRAY, &[block]);
            }
            return block;
        }
        thiscall(h, TIMELINE_DTOR, this, &[]);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// Starts the generation `g`'s workers once: unless its started flag
    /// (`+0x54 + parity`) is set, waits once per worker on the parity's
    /// semaphore (`+0x30 + 8 parity`), then sets the flag. `eax` as the
    /// originals leave it: the flag's address when `g` is at most two
    /// generations back, else the distance.
    fn start(h: &mut impl Host, this: u32, g: u32) -> u32 {
        let now: u32 = h.read(this + 0x40);
        let d = now.wrapping_sub(g);
        if d > 2 {
            return d;
        }
        let parity = g & 1;
        let flag = this + 0x54 + parity;
        if h.read::<u8>(flag) == 0 {
            let sem = this + 0x30 + 8 * parity;
            let mut i = 0u32;
            while i < h.read::<u32>(this + 8) {
                thiscall(h, SEMAPHORE_WAIT, sem, &[]);
                i += 1;
            }
            h.write(flag, 1u8);
        }
        flag
    }

    /// The current generation's job buffer.
    fn jobs(h: &impl Host, this: u32) -> u32 {
        if h.read::<u32>(this + 0x40) & 1 != 0 {
            h.read(this + 0x48)
        } else {
            h.read(this + 0x18)
        }
    }

    /// `0x0057d3e0(this; job)` -> `eax`: the generation two back started,
    /// the job stamped with the generation (`+8`) and queued; a full
    /// buffer is flushed (vtable `+0x18`, its `eax`), else the count.
    pub fn submit(h: &mut impl Host, this: u32, job: u32) -> u32 {
        let g: u32 = h.read(this + 0x40);
        start(h, this, g.wrapping_sub(2));
        let g: u32 = h.read(this + 0x40);
        h.write(job.wrapping_add(8), g);
        let list = jobs(h, this);
        let k: u32 = h.read(this + 0x4c);
        h.write(list.wrapping_add(4 * k), job);
        let k: u32 = h.read(this + 0x4c);
        h.write(this + 0x4c, k.wrapping_add(1));
        let k: u32 = h.read(this + 0x4c);
        if k == h.read::<u32>(this + 0x14) {
            return vtable_call(h, this, 0x18);
        }
        k
    }

    /// `0x0057d8d0(this; job)` -> 0, or 0xe when the buffer is full: as
    /// [`submit`] without the flush.
    pub fn try_submit(h: &mut impl Host, this: u32, job: u32) -> u32 {
        let g: u32 = h.read(this + 0x40);
        start(h, this, g.wrapping_sub(2));
        let g: u32 = h.read(this + 0x40);
        h.write(job.wrapping_add(8), g);
        let list = jobs(h, this);
        let k: u32 = h.read(this + 0x4c);
        if k >= h.read::<u32>(this + 0x14) {
            return 0xe;
        }
        h.write(list.wrapping_add(4 * k), job);
        let k: u32 = h.read(this + 0x4c);
        h.write(this + 0x4c, k.wrapping_add(1));
        0
    }

    /// `0x0057d970(this; job)` -> `eax`: waits for the job's generation:
    /// flushes when it is the current one, then starts it.
    pub fn wait(h: &mut impl Host, this: u32, job: u32) -> u32 {
        let g: u32 = h.read(job.wrapping_add(8));
        if g == h.read::<u32>(this + 0x40) {
            vtable_call(h, this, 0x18);
        }
        start(h, this, g)
    }

    /// The flush `0x0057d740(this)` -> 0: with jobs queued, the generation
    /// moves on and its jobs are split evenly between the workers (each
    /// record gets its slice's start and end), then the parity's run
    /// semaphore (`+0x20 + 8 parity`) is signalled once per worker. The
    /// original's `eax` is the signal's 0, or the caller's when nothing was
    /// queued.
    pub fn flush(h: &mut impl Host, this: u32) -> u32 {
        if h.read::<u32>(this + 0x4c) == 0 {
            return 0;
        }
        let g: u32 = h.read(this + 0x40);
        h.write(this + 0x44, g);
        let parity = h.read::<u32>(this + 0x44) & 1;
        let queued: u32 = h.read(this + 0x4c);
        h.write(this + 0x50, queued);
        h.write(this + 0x54 + parity, 0u8);
        let g: u32 = h.read(this + 0x40);
        let list = jobs(h, this);
        h.write(this + 0x40, g.wrapping_add(1));
        h.write(this + 0x4c, 0u32);
        let records: u32 = h.read(this + 0x58 + 4 * parity);
        let mut start = list;
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) {
            let rec = records.wrapping_add(12 * i);
            h.write(rec.wrapping_add(4), start);
            i += 1;
            let handed: u32 = h.read(this + 0x50);
            let workers: u32 = h.read(this + 8);
            let end = list.wrapping_add(4 * (handed.wrapping_mul(i) / workers));
            h.write(rec.wrapping_add(8), end);
            start = end;
        }
        let workers: u32 = h.read(this + 8);
        thiscall(h, SEMAPHORE_SIGNAL, this + 0x20 + 8 * parity, &[workers, 0])
    }

    /// The shutdown `0x0057d7e0(this)` -> `eax`: the timelines leave the
    /// profiler's collection and are destroyed (the array's first
    /// element's vector deleting destructor with 3, or `operator delete[]`
    /// for none); with range records the workers are told to stop (each
    /// thread record's `+0x14` set) and released, then `PThreadPool`'s
    /// destructor body; with records the four semaphores are closed and
    /// their flags cleared and both buffers `+0x48`, `+0x58` freed. `eax`
    /// is the last call's.
    pub fn shutdown(h: &mut impl Host, this: u32) -> u32 {
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) {
            let at = h.read::<u32>(this + 0x1c).wrapping_add(4 * i);
            let c = h.rebase(TIMELINES);
            thiscall(h, REMOVE_TIMELINE, c, &[at]);
            i += 1;
        }
        let p: u32 = h.read(this + 0x1c);
        if p != 0 {
            if h.read::<u32>(p.wrapping_sub(4)) == 0 {
                cdecl(h, OPERATOR_DELETE_ARRAY, &[p.wrapping_sub(4)]);
            } else {
                let vt: u32 = h.read(p);
                let f: u32 = h.read(vt);
                h.call(f, Conv::Thiscall(p), &[3]);
            }
        }
        h.write(this + 0x1c, 0u32);
        let has = h.read::<u32>(this + 0x58) != 0;
        if has {
            let mut i = 0u32;
            while i < h.read::<u32>(this + 8) {
                let rec = h.read::<u32>(this + 4).wrapping_add(0x18 * i);
                h.write(rec.wrapping_add(0x14), 1u8);
                i += 1;
            }
            let parity = h.read::<u32>(this + 0x40) & 1;
            let workers: u32 = h.read(this + 8);
            thiscall(h, SEMAPHORE_SIGNAL, this + 0x20 + 8 * parity, &[workers, 0]);
        }
        let eax = thiscall(h, POOL_DTOR_BODY, this, &[]);
        if !has {
            return eax;
        }
        for (sem, flag) in [(0x30, 0x34), (0x38, 0x3c), (0x20, 0x24), (0x28, 0x2c)] {
            thiscall(h, CLOSE, this + sem, &[]);
            h.write(this + flag, 0u8);
        }
        let a: u32 = h.read(this + 0x48);
        cdecl(h, PHYRE_FREE, &[a]);
        let b: u32 = h.read(this + 0x58);
        let eax = cdecl(h, PHYRE_FREE, &[b]);
        h.write(this + 0x48, 0u32);
        h.write(this + 0x58, 0u32);
        eax
    }
}
