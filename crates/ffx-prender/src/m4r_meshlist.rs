//! `PRendererBase::renderMeshInstanceList` (`0x005b3df0`, list
//! p6-render-c; `m4-rest`, from p6's round 5 draft, checked against the
//! machine code): the culled, sorted instances `[begin, end)` of a world
//! drawn, on the job workers or here.
//!
//! `thiscall(renderer; begin, end, contexts)`, `ret 0xc`. With as many
//! render contexts (`contexts`: `+0` count, bit 31 not owned; `+4` data,
//! `0xe4` bytes each) as the thread pool has threads, the list is cut into
//! jobs:
//! - the renderer's virtual slot 2, the render thread singleton; each
//!   context gets the renderer's `+0x48` (at `+0x48`), its camera
//!   (`+0x5c`, `setCamera`) and its viewport words (`+8..+0x14` to
//!   `+8..+0x14`);
//! - a job builder in the frame (`ebp - 0x2328`, built by `0x005b4440`
//!   with the renderer, `+0x2c` and `+0x40 / 72`) takes instances through
//!   `0x005b55a0(builder; &cursor, &params)` until it holds ten
//!   (`ebp - 0x14`) or refuses one (the cursor at `ebp - 0x235c`: begin,
//!   end, the current instance, 0, byte 1; the parameters at
//!   `ebp - 0x23e0`: camera, `+0x60`, `+0xa0`, the instance and its
//!   `+0x14`);
//! - a builder holding records becomes a job: room in the two segmented
//!   rings (`+0x1c` for the builder's constant bytes `ebp - 8`, `+0x30`
//!   for its 72-byte records; when either would overtake its reader, the
//!   render semaphore `+0x44` is waited for, `+0x4c`/`+0x50` cleared and
//!   both rings move to their next segment), a job record (`0x005b3680`,
//!   vtable `0x00b36888`) given the context data, the semaphore when it
//!   waited, `+4`, the render thread's registration (`0x00614720`) and
//!   the two blocks (not-owned arrays at `+0x1c` and `+0x24`), filled by
//!   `0x005b99a0` and queued on the pool (slot 5, a kick (slot 6) after
//!   every four);
//! - the last job is waited on (slot 7). 0.
//!
//! Otherwise each instance goes through `renderMeshInstance(instance,
//! instance +0x10, 0, renderer +0x13c)`; `eax` is the last error (0 for
//! none).
//!
//! The frame: the bridge takes the original's `ebp` (`0x23e0` bytes below
//! it, the cookie at `ebp - 4` left alone) and keeps the words the
//! callees see where the original keeps them.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PThreadPool_getThreadCount` (cdecl `()`).
pub const THREAD_COUNT: u32 = 0x0044_cde0;
/// `PRenderThread`'s singleton (cdecl `()`).
pub const THREAD_INSTANCE: u32 = 0x0061_4410;
/// The thread pool singleton (cdecl `()`).
pub const THREAD_POOL: u32 = 0x0044_cd30;
/// `PRenderContext::setCamera` (thiscall `(ctx, camera)`).
pub const SET_CAMERA: u32 = 0x005b_abd0;
/// The job builder's constructor (thiscall `(builder, renderer, base,
/// records)`).
pub const BUILDER_CTOR: u32 = 0x005b_4440;
/// The builder's `add` (thiscall `(builder, &cursor, &params)`) -> 0 when
/// it took the instance.
pub const BUILDER_ADD: u32 = 0x005b_55a0;
/// The builder's `fill` (thiscall `(builder, job)`).
pub const BUILDER_FILL: u32 = 0x005b_99a0;
/// A segmented ring's `stepForAllocation` (thiscall `(ring, first, size,
/// &out)`) -> the new cursor.
pub const STEP_FOR_ALLOCATION: u32 = 0x0061_3f00;
/// `PSemaphore::wait` (thiscall `(sem)`).
pub const SEMAPHORE_WAIT: u32 = 0x0043_02b0;
/// A segmented ring's `nextSegment` (thiscall `(ring)`).
pub const NEXT_SEGMENT: u32 = 0x0061_3eb0;
/// A segmented ring's `allocate` (thiscall `(ring, size)`) -> the block.
pub const SEGMENTED_ALLOCATE: u32 = 0x0061_3e10;
/// The job record's allocator (cdecl `()`) -> the job.
pub const NEW_JOB: u32 = 0x005b_3680;
/// The job record's vtable.
pub const JOB_VTABLE: u32 = 0x00b3_6888;
/// `PRenderThread`'s registration (thiscall `(thread, link)`).
pub const THREAD_REGISTER: u32 = 0x0061_4720;
/// `Phyre_PhyreFree` (cdecl `(p)`).
pub const FREE: u32 = 0x0042_fb20;
/// `PRendererBase::renderMeshInstance` (thiscall `(renderer, instance,
/// contexts, 0, extra)`).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// The most instances a job takes.
pub const PER_JOB: u32 = 10;
/// The frame's size below `ebp`.
pub const FRAME: u32 = 0x23e0;

/// Whether a ring's step (`first`, the new cursor `next`, `out` what the
/// step wrote) keeps behind its reader `read`: the original's test.
#[must_use]
pub fn fits(first: u32, out: u32, next: u32, read: u32) -> bool {
    if out >= first {
        first > read || next <= read
    } else {
        first > read
    }
}

pub mod bridge {
    //! The original over game memory.

    use super::{
        BUILDER_ADD, BUILDER_CTOR, BUILDER_FILL, Conv, FREE, Host, JOB_VTABLE, NEW_JOB,
        NEXT_SEGMENT, PER_JOB, RENDER_MESH_INSTANCE, SEGMENTED_ALLOCATE, SEMAPHORE_WAIT,
        SET_CAMERA, STEP_FOR_ALLOCATION, THREAD_COUNT, THREAD_INSTANCE, THREAD_POOL,
        THREAD_REGISTER, fits,
    };

    /// The frame words, as `ebp` offsets.
    mod at {
        pub const PARAMS: u32 = 0x23e0;
        pub const PARAM_WORD_A: u32 = 0x2378;
        pub const PARAM_WORD_B: u32 = 0x2374;
        pub const THREAD: u32 = 0x2370;
        pub const DATA: u32 = 0x236c;
        pub const OUT1: u32 = 0x2368;
        pub const OUT2: u32 = 0x2364;
        pub const SIZE2: u32 = 0x2360;
        pub const CURSOR: u32 = 0x235c;
        pub const CURSOR_END: u32 = 0x2358;
        pub const CURSOR_CUR: u32 = 0x2354;
        pub const CURSOR_ZERO: u32 = 0x2350;
        pub const CURSOR_FLAG: u32 = 0x234c;
        pub const CONTEXTS: u32 = 0x2348;
        pub const QUEUED: u32 = 0x2344;
        pub const WAITED: u32 = 0x233d;
        pub const RING1: u32 = 0x233c;
        pub const SIZE1: u32 = 0x2338;
        pub const RESULT: u32 = 0x2334;
        pub const JOB: u32 = 0x2330;
        pub const RENDERER: u32 = 0x232c;
        pub const BUILDER: u32 = 0x2328;
        pub const BUILDER_A: u32 = 0x231c;
        pub const BUILDER_B: u32 = 0x31c;
        /// The builder's counters: `+0` (`ebp - 0x1c`) and `+4` its two
        /// cursors, then its instance count (`ebp - 0x14`), `ebp - 0x10`,
        /// its record count (`ebp - 0xc`) and its constant bytes
        /// (`ebp - 8`).
        pub const B_CURSOR_A: u32 = 0x1c;
        pub const B_CURSOR_B: u32 = 0x18;
        pub const B_INSTANCES: u32 = 0x14;
        pub const B_WORD: u32 = 0x10;
        pub const B_RECORDS: u32 = 0xc;
        pub const B_BYTES: u32 = 8;
    }

    /// The frame at the original's `ebp`.
    struct Frame(u32);

    impl Frame {
        fn at(&self, off: u32) -> u32 {
            self.0.wrapping_sub(off)
        }
    }

    /// The cursor's next instance: `begin` moves on; with one left its
    /// word, the pair after it cleared and the byte set (the original's
    /// `ebx`), else the one kept.
    fn advance(h: &mut impl Host, f: &Frame, begin: &mut u32, end: u32) -> u32 {
        *begin = begin.wrapping_add(4);
        h.write(f.at(at::CURSOR), *begin);
        if end > *begin {
            let cur: u32 = h.read(*begin);
            h.write(f.at(at::CURSOR_CUR), cur);
            h.write(f.at(at::CURSOR_ZERO), 0_u32);
            h.write(f.at(at::CURSOR_FLAG), 1_u8);
            cur
        } else {
            h.read(f.at(at::CURSOR_CUR))
        }
    }

    /// A not-owned array (count word at `at`, data after it) given `block`
    /// of `size` bytes, unless the size has bit 31 set; the old owned,
    /// non-null, different storage is freed.
    fn give(h: &mut impl Host, at: u32, block: u32, size: u32) {
        if size & 0x8000_0000 != 0 {
            return;
        }
        let old: u32 = h.read(at + 4);
        let head: u32 = h.read(at);
        if old != block && head >> 31 == 0 && old != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(at + 4, block);
        h.write(at, size | 0x8000_0000);
    }

    /// The single-thread path: `renderMeshInstance` on each instance;
    /// the last error.
    fn render_here(h: &mut impl Host, f: &Frame, r: u32, begin: u32, end: u32, first: u32) -> u32 {
        let mut last = 0_u32;
        let mut p = begin;
        while end > p {
            let inst: u32 = if p == begin { first } else { h.read(p) };
            let contexts: u32 = h.read(inst.wrapping_add(0x10));
            let extra: u32 = h.read(r.wrapping_add(0x13c));
            let e = h.call_sym(
                RENDER_MESH_INSTANCE,
                Conv::Thiscall(r),
                &[inst, contexts, 0, extra],
            );
            if e != 0 {
                last = e;
                h.write(f.at(at::RESULT), e);
            }
            p = p.wrapping_add(4);
        }
        last
    }

    /// Each context gets the pass, the camera and the viewport.
    fn set_up_contexts(h: &mut impl Host, f: &Frame, r: u32, data: u32, count: u32) {
        let pass: u32 = h.read(r.wrapping_add(0x48));
        h.write(f.at(at::RESULT), pass);
        let camera: u32 = h.read(r.wrapping_add(0x5c));
        h.write(f.at(at::RING1), camera);
        h.write(f.at(at::DATA), data);
        for k in 0..count {
            let ctx = data.wrapping_add(0xe4_u32.wrapping_mul(k));
            h.write(ctx.wrapping_add(0x48), pass);
            if camera != 0 {
                h.call_sym(SET_CAMERA, Conv::Thiscall(ctx), &[camera]);
            }
            for i in 0..4 {
                let w: u32 = h.read(r.wrapping_add(8 + 4 * i));
                h.write(ctx.wrapping_add(8 + 4 * i), w);
            }
        }
        if count != 0 {
            h.write(f.at(at::SIZE1), 0_u32);
        }
    }

    /// Fills the builder from the cursor until it holds [`PER_JOB`]
    /// instances, refuses one, or the list ends. Returns the cursor
    /// (`p`, `stop`) and the current instance.
    fn fill_builder(
        h: &mut impl Host,
        f: &Frame,
        r: u32,
        mut p: u32,
        mut stop: u32,
        mut cur: u32,
    ) -> (u32, u32, u32) {
        let builder = f.at(at::BUILDER);
        while stop > p {
            let camera: u32 = h.read(r.wrapping_add(0x5c));
            let params = [
                camera,
                r.wrapping_add(0x60),
                r.wrapping_add(0xa0),
                0,
                cur,
                0,
                0,
                0,
                0,
                0,
            ];
            for (i, &w) in params.iter().enumerate() {
                h.write(f.at(at::PARAMS) + 4 * i as u32, w);
            }
            h.write(f.at(at::PARAM_WORD_A), 0_u32);
            h.write(f.at(at::PARAM_WORD_B), 0_u32);
            let kind: u32 = h.read(cur.wrapping_add(0x14));
            h.write(f.at(at::PARAMS) + 0x18, kind);
            let refused = h.call_sym(
                BUILDER_ADD,
                Conv::Thiscall(builder),
                &[f.at(at::CURSOR), f.at(at::PARAMS)],
            );
            p = h.read(f.at(at::CURSOR));
            stop = h.read(f.at(at::CURSOR_END));
            if refused == 0 {
                cur = advance(h, f, &mut p, stop);
                if h.read::<u32>(f.at(at::B_INSTANCES)) >= PER_JOB {
                    break;
                }
            } else {
                // a refused first instance is skipped
                cur = if h.read::<u32>(f.at(at::B_INSTANCES)) == 0 {
                    advance(h, f, &mut p, stop)
                } else {
                    h.read(f.at(at::CURSOR_CUR))
                };
                break;
            }
        }
        (p, stop, cur)
    }

    /// Room for `size` bytes in `ring` at its write cursor (`+8`) without
    /// passing its reader (`+0xc`).
    fn ring_fits(h: &mut impl Host, f: &Frame, ring: u32, size: u32, out: u32) -> bool {
        let read: u32 = h.read(ring + 0xc);
        h.write(f.at(at::JOB), read);
        let first: u32 = h.read(ring + 8);
        h.write(f.at(at::RESULT), first);
        let next = h.call_sym(
            STEP_FOR_ALLOCATION,
            Conv::Thiscall(ring),
            &[first, size, f.at(out)],
        );
        let wrote: u32 = h.read(f.at(out));
        fits(first, wrote, next, read)
    }

    /// The builder's records as a queued job; returns the job.
    fn queue_job(
        h: &mut impl Host,
        f: &Frame,
        r: u32,
        records: u32,
        thread: u32,
        data: u32,
        pool: u32,
    ) -> u32 {
        let ring1 = r.wrapping_add(0x1c);
        let ring2 = r.wrapping_add(0x30);
        h.write(f.at(at::RING1), ring1);
        let size1 = records.wrapping_mul(72);
        h.write(f.at(at::SIZE1), size1);
        let size2: u32 = h.read(f.at(at::B_BYTES));
        h.write(f.at(at::SIZE2), size2);
        let ok = ring_fits(h, f, ring1, size2, at::OUT1) && ring_fits(h, f, ring2, size1, at::OUT2);
        let waited = !ok;
        h.write(f.at(at::WAITED), u8::from(waited));
        if waited {
            let sem: u32 = h.read(r.wrapping_add(0x44));
            if sem != 0 {
                h.call_sym(SEMAPHORE_WAIT, Conv::Thiscall(sem), &[]);
            }
            h.write(r.wrapping_add(0x4c), 0_u32);
            h.write(r.wrapping_add(0x50), 0_u32);
            h.call_sym(NEXT_SEGMENT, Conv::Thiscall(ring1), &[]);
            h.call_sym(NEXT_SEGMENT, Conv::Thiscall(ring2), &[]);
        }
        let block1 = h.call_sym(SEGMENTED_ALLOCATE, Conv::Thiscall(ring2), &[size1]);
        h.write(f.at(at::RESULT), block1);
        let block2 = h.call_sym(SEGMENTED_ALLOCATE, Conv::Thiscall(ring1), &[size2]);
        h.write(f.at(at::RING1), block2);
        let job = h.call_sym(NEW_JOB, Conv::Cdecl, &[]);
        h.write(job.wrapping_add(0x10), data);
        h.write(f.at(at::JOB), job);
        let vt = h.rebase(JOB_VTABLE);
        h.write(job.wrapping_add(4), vt);
        let sem = if waited {
            h.read(r.wrapping_add(0x44))
        } else {
            0_u32
        };
        h.write(job.wrapping_add(0x34), sem);
        let word: u32 = h.read(r.wrapping_add(4));
        h.write(job.wrapping_add(0x38), word);
        h.call_sym(
            THREAD_REGISTER,
            Conv::Thiscall(thread),
            &[job.wrapping_add(0x18)],
        );
        give(h, job.wrapping_add(0x1c), block1, size1);
        give(h, job.wrapping_add(0x24), block2, size2);
        let extra: u32 = h.read(r.wrapping_add(0x13c));
        h.write(job.wrapping_add(0x3c), extra);
        h.call_sym(BUILDER_FILL, Conv::Thiscall(f.at(at::BUILDER)), &[job]);
        h.call_virtual(pool, 5, &[job]);
        job
    }

    /// `renderMeshInstanceList(renderer; begin, end, contexts)` with the
    /// original's frame at `ebp`: see the module.
    pub fn render_list(
        h: &mut impl Host,
        r: u32,
        begin: u32,
        end: u32,
        contexts: u32,
        ebp: u32,
    ) -> u32 {
        let f = Frame(ebp);
        h.write(f.at(at::RENDERER), r);
        h.write(f.at(at::CONTEXTS), contexts);
        h.write(f.at(at::RESULT), 0_u32);
        h.write(f.at(at::CURSOR), begin);
        h.write(f.at(at::CURSOR_END), end);
        h.write(f.at(at::QUEUED), 0_u32);
        h.write(f.at(at::CURSOR_CUR), 0_u32);
        h.write(f.at(at::CURSOR_ZERO), 0_u32);
        h.write(f.at(at::CURSOR_FLAG), 1_u8);
        let mut cur = 0_u32;
        if end > begin {
            cur = h.read(begin);
            h.write(f.at(at::QUEUED), cur);
            h.write(f.at(at::CURSOR_CUR), cur);
            h.write(f.at(at::CURSOR_ZERO), 0_u32);
            h.write(f.at(at::CURSOR_FLAG), 1_u8);
        }
        let threads = h.call_sym(THREAD_COUNT, Conv::Cdecl, &[]);
        let count = h.read::<u32>(contexts) & 0x7fff_ffff;
        h.write(f.at(at::SIZE1), count);
        if threads == 0 || count != threads {
            return render_here(h, &f, r, begin, end, cur);
        }
        h.call_virtual(r, 2, &[]);
        let thread = h.call_sym(THREAD_INSTANCE, Conv::Cdecl, &[]);
        h.write(f.at(at::THREAD), thread);
        let data: u32 = h.read(contexts.wrapping_add(4));
        set_up_contexts(h, &f, r, data, count);
        let records: u32 = h.read(r.wrapping_add(0x40));
        let base: u32 = h.read(r.wrapping_add(0x2c));
        h.call_sym(
            BUILDER_CTOR,
            Conv::Thiscall(f.at(at::BUILDER)),
            &[r, base, records / 72],
        );
        let pool = h.call_sym(THREAD_POOL, Conv::Cdecl, &[]);
        h.write(f.at(at::CONTEXTS), pool);
        let mut job = 0_u32;
        let mut queued = 0_u32;
        h.write(f.at(at::JOB), 0_u32);
        h.write(f.at(at::QUEUED), 0_u32);
        let (mut p, mut stop) = (begin, end);
        while stop > p {
            h.write(f.at(at::B_CURSOR_A), f.at(at::BUILDER_A));
            h.write(f.at(at::B_CURSOR_B), f.at(at::BUILDER_B));
            for off in [at::B_INSTANCES, at::B_WORD, at::B_RECORDS, at::B_BYTES] {
                h.write(f.at(off), 0_u32);
            }
            (p, stop, cur) = fill_builder(h, &f, r, p, stop, cur);
            let records: u32 = h.read(f.at(at::B_RECORDS));
            if records == 0 {
                continue;
            }
            job = queue_job(h, &f, r, records, thread, data, pool);
            queued += 1;
            h.write(f.at(at::QUEUED), queued);
            if queued == 4 {
                queued = 0;
                h.write(f.at(at::QUEUED), 0_u32);
                h.call_virtual(pool, 6, &[]);
            }
        }
        if job != 0 {
            h.call_virtual(pool, 7, &[job]);
        }
        0
    }
}
