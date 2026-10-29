//! M5 stage 1, list `phyre-render-b`: three D3D11-facing methods: the
//! morph manager's 16-byte record list insert, a dynamic buffer's
//! upload through `Map`/`Unmap`, and a full-screen draw's state set-up
//! (the vertex buffers, strides and offsets in its frame).

/// `Phyre_PhyreMalloc_0042fb80(size, align)`, `Phyre_PhyreFree` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The render scope lock (thiscall on a frame word), the device holder
/// made on first use, its context word (`+0x70`), `0x00585b50(array; i)`
/// (thiscall, element `i`), `crt_memcpy_thunk` (cdecl).
pub const SCOPE_ENTER: u32 = 0x0046_3d50;
pub const SCOPE_LEAVE: u32 = 0x0046_3e90;
pub const HOLDER_GUARD: u32 = 0x00c9_628c;
pub const HOLDER: u32 = 0x00c9_4f00;
pub const HOLDER_CTOR: u32 = 0x004f_ee70;
pub const HOLDER_DTOR: u32 = 0x00af_b710;
pub const ATEXIT: u32 = 0x0094_9207;
pub const HOLDER_CONTEXT: u32 = 0x00c9_4f70;
pub const ELEMENT: u32 = 0x0058_5b50;
pub const MEMCPY: u32 = 0x0094_9256;
/// `PRendererD3D11_mapPrimitiveTopology(p)` (cdecl),
/// `PShaderPassStateD3D11_bind(state; renderer)` (thiscall).
pub const MAP_TOPOLOGY: u32 = 0x0058_ab60;
pub const PASS_STATE_BIND: u32 = 0x0058_c6c0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// COM method `off` of `obj` with `args` after it.
    fn com(h: &mut impl Host, obj: u32, off: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt + off);
        let mut words = [0u32; 8];
        words[0] = obj;
        words[1..=args.len()].copy_from_slice(args);
        h.call(f, Conv::Stdcall, &words[..=args.len()]) // call width: 7
    }

    /// `0x006ab950(list, key, a, b, c)` (stdcall): nothing when a record
    /// with `key` exists; else `{key, a, b, c}` in the first record whose
    /// key is 0, or appended to a grown buffer (`PhyreMalloc`, the old one
    /// freed when owned; 0xd when it fails); 0.
    pub fn insert(h: &mut impl Host, list: u32, rec: [u32; 4]) -> u32 {
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        let data: u32 = h.read(list + 4);
        for i in 0..n {
            if h.read::<u32>(data.wrapping_add(16 * i)) == rec[0] {
                return 0;
            }
        }
        for i in 0..n {
            let at = data.wrapping_add(16 * i);
            if h.read::<u32>(at) == 0 {
                h.write(at, rec);
                return 0;
            }
        }
        let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_add(1) << 4, 4]);
        if buf == 0 {
            return 0xd;
        }
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        let mut src: u32 = h.read(list + 4);
        for i in 0..n {
            let d = buf.wrapping_add(16 * i);
            if d != 0 {
                let v: [u32; 4] = h.read(src);
                h.write(d, v);
                src = src.wrapping_add(16);
            }
        }
        let last = buf.wrapping_add(n << 4);
        if last != 0 {
            h.write(last, rec);
        }
        let old: u32 = h.read(list + 4);
        let word: u32 = h.read(list);
        if old != buf && word & 0x8000_0000 == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(list + 4, buf);
        h.write(list, n.wrapping_add(1));
        0
    }

    /// `0x005937a0(buffer; i)`: element `i` of the `PSharray` at `+0xc`
    /// (16 bytes) with pending data (`+8`) uploaded under the render scope
    /// lock (`ebp - 0x14`): the context (`ebp - 0x10`) maps its resource
    /// (`WRITE_DISCARD` (4) for a dynamic buffer, `+0x24` == 2, else
    /// `WRITE` (2)) into the zeroed mapping at `ebp - 0x20`, the
    /// `[+0x20]` bytes at `+4` copied in, unmapped, `+8` cleared; 9 when
    /// the map fails or gives no pointer, else 0.
    pub fn upload(h: &mut impl Host, ebp: u32, this: u32, i: u32) -> u32 {
        let count = h.read::<u32>(this + 0xc) & 0x7fff_ffff;
        let data = sharray(h, this + 0xc, count);
        if h.read::<u32>(data.wrapping_add(16 * i).wrapping_add(8)) == 0 {
            return 0;
        }
        let lock = ebp - 0x14;
        this_call(h, SCOPE_ENTER, lock, &[]);
        let g = h.rebase(HOLDER_GUARD);
        let v: u32 = h.read(g);
        if v & 1 == 0 {
            h.write(g, v | 1);
            let holder = h.rebase(HOLDER);
            this_call(h, HOLDER_CTOR, holder, &[]);
            let dtor = h.rebase(HOLDER_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let ctx: u32 = h.read(h.rebase(HOLDER_CONTEXT));
        h.write(ebp - 0x10, ctx);
        let mapped = ebp - 0x20;
        h.write(mapped, [0u32; 3]);
        let kind = if h.read::<u32>(this + 0x24) == 2 {
            4
        } else {
            2
        };
        let count = h.read::<u32>(this + 0xc) & 0x7fff_ffff;
        let data = sharray(h, this + 0xc, count);
        let res: u32 = h.read(data.wrapping_add(16 * i));
        let hr = com(h, ctx, 0x38, &[res, 0, kind, 0, mapped]);
        let dst: u32 = h.read(mapped);
        if (hr as i32) < 0 || dst == 0 {
            this_call(h, SCOPE_LEAVE, lock, &[]);
            return 9;
        }
        let arr = this + 0xc;
        let e = this_call(h, ELEMENT, arr, &[i]);
        let src: u32 = h.read(e + 4);
        let size: u32 = h.read(this + 0x20);
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, size]);
        let e = this_call(h, ELEMENT, arr, &[i]);
        let res: u32 = h.read(e);
        com(h, ctx, 0x3c, &[res, 0]);
        let e = this_call(h, ELEMENT, arr, &[i]);
        h.write(e + 8, 0u32);
        this_call(h, SCOPE_LEAVE, lock, &[]);
        0
    }

    fn sharray(h: &mut impl Host, at: u32, count: u32) -> u32 {
        if count > 1 {
            h.read(at + 4)
        } else if count != 0 {
            at + 4
        } else {
            0
        }
    }

    /// `0x00592e90(quad; renderer, primitive, slot, n)`: 5 for a primitive
    /// with no topology; else the pass state bound and on the renderer's
    /// context: hull/domain shaders cleared (`+0x114`, `+0x5c`), the input
    /// layout (`+0x44`), the vertex shader (`+0x2c`), the pixel shader
    /// (`+0x24`, `+0x54` when `+0x58` is set, else `+0x50`), the
    /// constant buffer `+0x48` to both stages (`+0x1c`, `+0x40`), the
    /// sampler and resource (`+0x20`, `+0x28`), the three vertex buffers
    /// of `slot` with their strides and offsets in the frame (`+0x48`),
    /// the topology (`+0x60`), the draw of `n` (`+0x34`); 0.
    pub fn draw_setup(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        r: u32,
        primitive: u32,
        slot: u32,
        n: u32,
    ) -> u32 {
        h.write(ebp - 0x20, slot);
        let topology = h.call_sym(MAP_TOPOLOGY, Conv::Cdecl, &[primitive]);
        h.write(ebp - 0x24, topology);
        if topology == 0 {
            return 5;
        }
        this_call(h, PASS_STATE_BIND, this + 0x60, &[r]);
        let ctx = |h: &mut _| -> u32 { Mem::read(h, r + 0x70) };
        let c = ctx(h);
        com(h, c, 0x114, &[0, 0, 0]);
        let c = ctx(h);
        com(h, c, 0x5c, &[0, 0, 0]);
        let c = ctx(h);
        let layout: u32 = h.read(this + 0x44);
        com(h, c, 0x44, &[layout]);
        let c = ctx(h);
        let vs: u32 = h.read(this + 0x4c);
        com(h, c, 0x2c, &[vs, 0, 0]);
        let ps: u32 = if h.read::<u32>(this + 0x58) != 0 {
            h.read(this + 0x54)
        } else {
            h.read(this + 0x50)
        };
        let c = ctx(h);
        com(h, c, 0x24, &[ps, 0, 0]);
        for (off, at) in [(0x1c, 0x48), (0x40, 0x48), (0x20, 0x58), (0x28, 0x5c)] {
            let c = ctx(h);
            com(h, c, off, &[0, 1, this + at]);
        }
        let vb: u32 = h.read(this.wrapping_add(slot.wrapping_mul(4)));
        let (bufs, strides, offsets) = (ebp - 0x30, ebp - 0x1c, ebp - 0x10);
        h.write(bufs, [vb; 3]);
        h.write(strides, [0x28u32; 3]);
        h.write(offsets, [0u32, 0x10, 0x20]);
        let c = ctx(h);
        com(h, c, 0x48, &[0, 3, bufs, strides, offsets]);
        let c = ctx(h);
        com(h, c, 0x60, &[topology]);
        let c = ctx(h);
        com(h, c, 0x34, &[n, 0]);
        0
    }
}

/// The renderer pool's callees: `Phyre_PhyreMalloc_0042fb80(size, align)`,
/// the block init `0x00539600(blocks, n)` (cdecl), the renderer
/// constructor `Phyre_PRendering_PRendererBase_PRendererBase_005b4650(r;
/// device)` (thiscall), the renderers' release
/// `Phyre_PRendering_freeJobs_005b5330(array, n)` (cdecl) and
/// `Phyre_PRendering_PRendererForJobs_vftable`.
pub const BLOCKS_INIT: u32 = 0x0053_9600;
pub const RENDERER_CTOR: u32 = 0x005b_4650;
pub const FREE_RENDERERS: u32 = 0x005b_5330;
pub const FOR_JOBS_VTABLE: u32 = 0x00b3_63e8;
/// A `PRendererForJobs`' size.
pub const RENDERER_SIZE: u32 = 0xe4;

pub mod pool {
    //! `0x005bbc30` over game memory.

    use super::{
        BLOCKS_INIT, FOR_JOBS_VTABLE, FREE_RENDERERS, PHYRE_FREE, PHYRE_MALLOC, RENDERER_CTOR,
        RENDERER_SIZE,
    };
    use ffx_graphic::host::{Conv, Host};

    /// `0x005bbc30(pool; n, device)`: nothing when the pool already holds
    /// `n` renderers; else its 16-byte block buffer (`+0`, `+4`) sized for
    /// `n` 0xe4-byte renderers (`PhyreMalloc`, aligned 16, initialised;
    /// the old owned one freed; 0xd when it fails), each block slot
    /// constructed as a `PRendererForJobs`, the first one made becoming the
    /// pool's array (`+8`, `+0xc`, external) while it has none; 0.
    pub fn resize(h: &mut impl Host, this: u32, n: u32, device: u32) -> u32 {
        if n == h.read::<u32>(this + 8) & 0x7fff_ffff {
            return 0;
        }
        let blocks = n.wrapping_mul(RENDERER_SIZE).wrapping_add(0xf) >> 4;
        if blocks != h.read::<u32>(this) {
            let mut buf = 0;
            if blocks != 0 {
                buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[blocks << 4, 0x10]);
                if buf == 0 {
                    return 0xd;
                }
                h.call_sym(BLOCKS_INIT, Conv::Cdecl, &[buf, blocks]);
            }
            let old: u32 = h.read(this + 4);
            let word: u32 = h.read(this);
            if old != buf && word & 0x8000_0000 == 0 && old != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
            }
            h.write(this + 4, buf);
            h.write(this, blocks);
        }
        let mut p: u32 = h.read(this + 4);
        for _ in 0..n {
            let made = if p == 0 {
                0
            } else {
                h.call_sym(RENDERER_CTOR, Conv::Thiscall(p), &[device]);
                let vt = h.rebase(FOR_JOBS_VTABLE);
                h.write(p, vt);
                h.write(p + 0xe0, 0u32);
                p
            };
            if h.read::<u32>(this + 0xc) == 0 && (n as i32) >= 0 {
                let count: u32 = h.read(this + 8);
                if made != 0 && count & 0x8000_0000 == 0 {
                    let arr: u32 = h.read(this + 0xc);
                    h.call_sym(FREE_RENDERERS, Conv::Cdecl, &[arr, count & 0x7fff_ffff]);
                }
                h.write(this + 8, n | 0x8000_0000);
                h.write(this + 0xc, made);
            }
            p = p.wrapping_add(RENDERER_SIZE);
        }
        0
    }
}

/// The command queue's callees: `Phyre_PRendering_PRenderRingBuffer_stepForAllocation`
/// `(ring; cursor, size, &wrapped)` and
/// `Phyre_PRendering_PRenderRingBufferSegmented_allocate(ring; size)`.
pub const RING_STEP: u32 = 0x0061_3f00;
pub const RING_ALLOCATE: u32 = 0x0061_3e10;

pub mod queue {
    //! `0x005bb2e0` over game memory.

    use super::{RING_ALLOCATE, RING_STEP};
    use ffx_graphic::host::{Conv, Host};

    /// Whether the ring at `ring` (cursor `+8`, end `+0xc`) is full for
    /// `size` more bytes, the step writing its wrapped cursor to `out`:
    /// the cursor not past the end and either a wrap or the new cursor
    /// past the end.
    fn full(h: &mut impl Host, ring: u32, size: u32, out: u32) -> bool {
        let cur: u32 = h.read(ring + 8);
        let end: u32 = h.read(ring + 0xc);
        let r = h.call_sym(RING_STEP, Conv::Thiscall(ring), &[cur, size, out]);
        let o: u32 = h.read(out);
        cur <= end && (o < cur || r > end)
    }

    /// `0x005bb2e0(renderer; arg)` (thiscall, `ret 4`): when the data ring
    /// (`+0x1c`, step 0, wrapped cursor at `ebp - 4`) or the command ring
    /// (`+0x30`, step 0x48, `ebp - 8`) is full, virtual slot 2 (the
    /// flush); then a 0x48-byte command from the command ring (0xd when
    /// there is none) becomes the first one (`+0x4c`, when unset) and the
    /// last end (`+0x50`), with type 0x20 and `arg` at `+8`; 0.
    pub fn push(h: &mut impl Host, ebp: u32, this: u32, arg: u32) -> u32 {
        if full(h, this + 0x1c, 0, ebp - 4) || full(h, this + 0x30, 0x48, ebp - 8) {
            h.call_virtual(this, 2, &[]);
        }
        let cmd = h.call_sym(RING_ALLOCATE, Conv::Thiscall(this + 0x30), &[0x48]);
        if cmd == 0 {
            return 0xd;
        }
        if h.read::<u32>(this + 0x4c) == 0 {
            h.write(this + 0x4c, cmd);
        }
        h.write(this + 0x50, cmd.wrapping_add(0x48));
        h.write(cmd + 8, arg);
        h.write(cmd, 0x20u32);
        0
    }
}

/// The render thread's stop callees: `Phyre_PRendering_PRenderThread_sync`,
/// the Win32 wrappers `Phyre_PSemaphoreWin32_wait` (`0x004302b0`),
/// `Phyre_PEventFlagWin32_signal` (`0x00430200`), the thread's join
/// (`0x004301b0(thread; &exit_code)`, `WaitForSingleObject` then
/// `GetExitCodeThread`), the thread's no-op `0x00430260` and the handle
/// closes `0x00430240` (events) and `0x00430250` (semaphores), all
/// thiscall.
pub const THREAD_SYNC: u32 = 0x0061_5ea0;
pub const SEMAPHORE_WAIT: u32 = 0x0043_02b0;
pub const EVENT_SIGNAL: u32 = 0x0043_0200;
pub const THREAD_JOIN: u32 = 0x0043_01b0;
pub const THREAD_NOP: u32 = 0x0043_0260;
pub const EVENT_CLOSE: u32 = 0x0043_0240;
pub const SEMAPHORE_CLOSE: u32 = 0x0043_0250;

pub mod thread {
    //! `0x00615f40` over game memory.

    use super::{
        EVENT_CLOSE, EVENT_SIGNAL, SEMAPHORE_CLOSE, SEMAPHORE_WAIT, THREAD_JOIN, THREAD_NOP,
        THREAD_SYNC,
    };
    use ffx_graphic::host::{Conv, Host};

    /// `0x00615f40(thread)` (thiscall, called by
    /// `Phyre_PFramework_PApplication_onExit`, which reads no `eax`): for
    /// a render thread not stopped (`+0xc` != 2) a sync; then, when it
    /// runs (`+0xc` == 1): stopped (2), the semaphore `+0x15c` waited, the
    /// next of the 16 command slots (20 bytes at `+0x10`, index `+0x154`
    /// wrapping) cleared (`+0x10`, `+0x14`, `+0x20`) and its event
    /// (`+0x18`) signalled, the thread joined (its exit code through the
    /// frame's `ebp - 4`, which the prologue's `push ecx` set to `thread`),
    /// every slot's event closed and its flag (`+0x1c`) cleared, both
    /// semaphores (`+0x15c`, `+0x164`) closed and their flags cleared, the
    /// thread's own flag `+4` too.
    pub fn stop(h: &mut impl Host, ebp: u32, this: u32) {
        h.write(ebp - 4, this);
        if h.read::<u32>(this + 0xc) == 2 {
            return;
        }
        h.call_sym(THREAD_SYNC, Conv::Thiscall(this), &[]);
        if h.read::<u32>(this + 0xc) != 1 {
            return;
        }
        h.write(this + 0xc, 2u32);
        h.call_sym(SEMAPHORE_WAIT, Conv::Thiscall(this + 0x15c), &[]);
        let i: u32 = h.read(this + 0x154);
        let e = this.wrapping_add(i.wrapping_mul(20));
        let next = i.wrapping_add(1);
        h.write(e + 0x10, [0u32; 2]);
        h.write(e + 0x20, 0u32);
        h.write(this + 0x154, if next == 0x10 { 0 } else { next });
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(e + 0x18), &[]);
        h.call_sym(THREAD_JOIN, Conv::Thiscall(this), &[ebp - 4]);
        h.call_sym(THREAD_NOP, Conv::Thiscall(this), &[]);
        h.write(this + 4, 0u8);
        for k in 0..0x10 {
            let s = this + 0x18 + 0x14 * k;
            h.call_sym(EVENT_CLOSE, Conv::Thiscall(s), &[]);
            h.write(s + 4, 0u8);
        }
        h.call_sym(SEMAPHORE_CLOSE, Conv::Thiscall(this + 0x15c), &[]);
        h.write(this + 0x160, 0u8);
        h.call_sym(SEMAPHORE_CLOSE, Conv::Thiscall(this + 0x164), &[]);
        h.write(this + 0x168, 0u8);
    }
}

/// The thread-pool job's callees: `Phyre_PRendering_PRendererBase_initRings_005b8e80`,
/// the instance-list render `0x005b9ee0(renderer; list, &cursor, arg)` and
/// `Phyre_PRendering_PDeferredKick_canKick`, all thiscall.
pub const INIT_RINGS: u32 = 0x005b_8e80;
pub const RENDER_LIST: u32 = 0x005b_9ee0;
pub const CAN_KICK: u32 = 0x0061_47e0;

pub mod job {
    //! `0x005b6820` over game memory.

    use super::{CAN_KICK, INIT_RINGS, RENDER_LIST, RENDERER_SIZE};
    use ffx_graphic::host::{Conv, Host};

    /// `PRendererThreadPoolJob` virtual slot 1, `0x005b6820(job; a, b,
    /// index)` (thiscall, `ret 0xc`): renderer `index` of the array `+0x10`
    /// (0xe4 bytes each, stored over the `index` argument's slot `slot`) takes
    /// the context `+0x38` (`+4`) and its rings (`initRings(+0x28, +0x20,
    /// +0x24 & 0x7fffffff, +0x1c & 0x7fffffff, 1)`, `+0x44` = 0 before,
    /// then `+0xe0` = the job's kick `+0x18`, `+0x44` = `+0x34`); each of
    /// the `+0x2c & 0x7fffffff` 12-byte lists at `+0x30` rendered (with
    /// the cursor `ebp - 0x14` = `+0x14` + 0x10, the frame's other words
    /// `+0x30`, `+0x34`, the job and the count left, decremented each
    /// time, and `+0x3c`); virtual slot 2 when the kick can kick, then
    /// slot 1; the words `+4` of `+0xc` (when set) and `+0x14` cleared;
    /// `eax` = `+0x14`.
    pub fn run(h: &mut impl Host, ebp: u32, slot: u32, this: u32, index: u32) -> u32 {
        let lists: u32 = h.read(this + 0x30);
        let mut count = h.read::<u32>(this + 0x2c) & 0x7fff_ffff;
        let r = index
            .wrapping_mul(RENDERER_SIZE)
            .wrapping_add(h.read(this + 0x10));
        h.write(slot, r);
        let out: u32 = h.read(this + 0x14);
        let first: u32 = h.read(this + 0x34);
        h.write(
            ebp - 0x14,
            [out.wrapping_add(0x10), lists, first, this, count],
        );
        let ctx: u32 = h.read(this + 0x38);
        h.write(r + 4, ctx);
        h.write(r + 0x44, 0u32);
        let rings: [u32; 4] = h.read(this + 0x1c);
        h.call_sym(
            INIT_RINGS,
            Conv::Thiscall(r),
            &[
                rings[3],
                rings[1],
                rings[2] & 0x7fff_ffff,
                rings[0] & 0x7fff_ffff,
                1,
            ],
        );
        let kick = this + 0x18;
        h.write(r + 0xe0, kick);
        h.write(r + 0x44, first);
        let mut list = lists;
        while count != 0 {
            let arg: u32 = h.read(this + 0x3c);
            h.call_sym(RENDER_LIST, Conv::Thiscall(r), &[list, ebp - 0x14, arg]);
            list = list.wrapping_add(0xc);
            count = h.read::<u32>(ebp - 4).wrapping_sub(1);
            h.write(ebp - 4, count);
        }
        if h.call_sym(CAN_KICK, Conv::Thiscall(kick), &[]) & 0xff != 0 {
            h.call_virtual(r, 2, &[]);
        }
        h.call_virtual(r, 1, &[]);
        let a: u32 = h.read(this + 0xc);
        if a != 0 {
            h.write(a + 4, 0u32);
        }
        let b: u32 = h.read(this + 0x14);
        h.write(b + 4, 0u32);
        b
    }
}
