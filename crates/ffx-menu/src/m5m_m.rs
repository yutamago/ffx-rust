//! M5 stage 1, the menu, battle and script leftovers (round 4), batch 3:
//! the script list's platform pieces (the PS2 thread, semaphore and mutex
//! wrappers over Win32, the IOP mouse RPCs, the async loader's job queue),
//! the event debugger's host-file layer (`AtelPC*`: `sce*` file calls on
//! `host0:` paths) and its setup and teardown, and the small script and
//! battle helpers the static call graph reaches.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// A Win32 import (stdcall) through its slot.
    fn win32(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// A CRT import (cdecl) through its slot.
    fn crt(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    // ---- Win32 and CRT import slots ----

    pub const RESUME_THREAD: u32 = 0x00b0_c1dc;
    pub const TERMINATE_THREAD: u32 = 0x00b0_c1d8;
    pub const CLOSE_HANDLE: u32 = 0x00b0_c0dc;
    pub const CREATE_SEMAPHORE_W: u32 = 0x00b0_c1e0;
    pub const SET_THREAD_PRIORITY: u32 = 0x00b0_c10c;
    pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
    pub const RELEASE_SEMAPHORE: u32 = 0x00b0_c0f4;
    pub const CREATE_MUTEX_W: u32 = 0x00b0_c198;
    pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;
    pub const STRSTR: u32 = 0x00b0_c378;
    pub const STRRCHR: u32 = 0x00b0_c3a4;
    pub const SPRINTF: u32 = 0x00b0_c3dc;

    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const EH_VECTOR_ITERATOR: u32 = 0x0094_96d2;
    pub const SYS_THREAD_CREATE: u32 = 0x0067_9cb0;

    // ---- the IOP mouse RPCs ----

    pub const FFX_SIF_CHECK_STAT_RPC: u32 = 0x00a4_4760;
    pub const FFX_SIF_CALL_RPC: u32 = 0x00a4_4750;
    /// The mouse's RPC client (`0x01325a40`) and its 0x20-byte buffer.
    pub const MOUSE_RPC: u32 = 0x0132_5a40;
    pub const MOUSE_BUF: u32 = 0x0132_5a80;

    /// `FFXUsbMouseSync(mode; slots)` (`0x00855bf0`): mode 0 waits while
    /// `FFXSifCheckStatRpc(&rpc)` is busy, returns 0; mode 1 tail-jumps
    /// to it (the client's address over the caller's argument word);
    /// other modes return 0.
    pub fn usb_mouse_sync(h: &mut impl Host, mode: u32, slots: u32) -> u32 {
        let rpc = h.rebase(MOUSE_RPC);
        match mode {
            0 => {
                while cdecl(h, FFX_SIF_CHECK_STAT_RPC, &[rpc]) != 0 {}
                0
            }
            1 => {
                h.write(slots, rpc);
                cdecl(h, FFX_SIF_CHECK_STAT_RPC, &[rpc])
            }
            _ => 0,
        }
    }

    /// `FFXUsbMouse_IopMemGet(dst)` (`0x00855c40`): RPC 5 into `dst`.
    pub fn usb_mouse_iop_mem_get(h: &mut impl Host, dst: u32) -> u32 {
        let rpc = h.rebase(MOUSE_RPC);
        let buf = h.rebase(MOUSE_BUF);
        cdecl(
            h,
            FFX_SIF_CALL_RPC,
            &[rpc, 5, 0, buf, 0x20, dst, 0x20, 0, 0],
        )
    }

    /// `FFXUsbMouseRead2(arg, dst)` (`0x00855bc0`): the buffer's first word
    /// = `arg`, RPC 4 into `dst`.
    pub fn usb_mouse_read2(h: &mut impl Host, arg: u32, dst: u32) -> u32 {
        let rpc = h.rebase(MOUSE_RPC);
        let buf = h.rebase(MOUSE_BUF);
        h.write(buf, arg);
        cdecl(
            h,
            FFX_SIF_CALL_RPC,
            &[rpc, 4, 0, buf, 0x20, dst, 0x20, 0, 0],
        )
    }

    // ---- threads, semaphores, mutexes ----

    /// `AsyncJobQueue_dtor_006db730(this)` (name ours; thiscall): the
    /// object's eight `0x1e8`-byte queues (`+0x50`, destructor
    /// `0x006db6f0`) and four `0x14`-byte records (destructor
    /// `0x00703990`) destroyed with `eh_vector_destructor_iterator`
    /// (its SEH frame left out: nothing here throws). Returns the
    /// second call's `eax`.
    pub fn job_queue_dtor(h: &mut impl Host, this: u32) -> u32 {
        let d = h.rebase(0x006d_b6f0);
        h.call_sym(
            EH_VECTOR_ITERATOR,
            Conv::Stdcall,
            &[at(this, 0x50), 0x1e8, 1, d],
        );
        let d = h.rebase(0x0070_3990);
        h.call_sym(EH_VECTOR_ITERATOR, Conv::Stdcall, &[this, 0x14, 4, d])
    }

    /// `StartThread_ps2(thread, arg)` (`0x006f2d30`): the argument at the
    /// thread object's `+0x14`, `ResumeThread`; 0.
    pub fn start_thread(h: &mut impl Host, t: u32, arg: u32) -> u32 {
        let o: u32 = h.read(t);
        h.write(at(o, 0x14), arg);
        let o: u32 = h.read(t);
        let handle: u32 = h.read(at(o, 4));
        win32(h, RESUME_THREAD, &[handle]);
        0
    }

    /// `CreateThread_ps2(params, name)` (`0x006f2e10`): a 4-byte handle
    /// holding a new 0x18-byte thread object (vtable `0x00b4bbcc`, all
    /// else 0; null when that allocation fails), its entry `+0x10` from
    /// `params +4`, created suspended by `SysThread_create(obj; params
    /// +0xc, 0, 0, name or "ps2thread")`. Returns the handle (a failed
    /// handle allocation reads through null, as the original does).
    pub fn create_thread(h: &mut impl Host, params: u32, name: u32) -> u32 {
        let handle = cdecl(h, OPERATOR_NEW, &[4]);
        if handle != 0 {
            let o = cdecl(h, OPERATOR_NEW, &[0x18]);
            if o == 0 {
                h.write(handle, 0u32);
            } else {
                h.write(at(o, 4), [0u32, 0]);
                h.write(at(o, 0xc), 0u8);
                let vt = h.rebase(0x00b4_bbcc);
                h.write(o, vt);
                h.write(at(o, 0x10), [0u32, 0]);
                h.write(handle, o);
            }
        }
        let o: u32 = h.read(handle);
        let entry: u32 = h.read(at(params, 4));
        h.write(at(o, 0x10), entry);
        let name = if name == 0 {
            h.rebase(0x00b4_bbf0)
        } else {
            name
        };
        let o: u32 = h.read(handle);
        let arg: u32 = h.read(at(params, 0xc));
        h.call_sym(SYS_THREAD_CREATE, Conv::Thiscall(o), &[arg, 0, 0, name]);
        handle
    }

    /// `DeleteThread_ps2_006f2f00(handle)` (name ours): a running thread
    /// terminated and its handle closed (`+4`, `+8` cleared), the object
    /// destroyed through its vtable (`(*vt[0])(obj; 1)`), the handle
    /// freed; 0.
    pub fn delete_thread(h: &mut impl Host, handle: u32) -> u32 {
        let o: u32 = h.read(handle);
        let th: u32 = h.read(at(o, 4));
        if th != 0 {
            win32(h, TERMINATE_THREAD, &[th, 0]);
            let th: u32 = h.read(at(o, 4));
            win32(h, CLOSE_HANDLE, &[th]);
            h.write(at(o, 4), [0u32, 0]);
        }
        let o: u32 = h.read(handle);
        if o != 0 {
            let vt: u32 = h.read(o);
            let f: u32 = h.read(vt);
            h.call(f, Conv::Thiscall(o), &[1]);
        }
        cdecl(h, OPERATOR_DELETE, &[handle]);
        0
    }

    /// `CreateSema_ps2_006f2b10(count)` (name ours): a 4-byte handle to a
    /// 4-byte object holding `CreateSemaphoreW(0, count, count, 0)` (null
    /// when that allocation fails); null when the handle's fails.
    pub fn create_sema(h: &mut impl Host, count: u32) -> u32 {
        let p = cdecl(h, OPERATOR_NEW, &[4]);
        if p == 0 {
            return 0;
        }
        let q = cdecl(h, OPERATOR_NEW, &[4]);
        if q == 0 {
            h.write(p, 0u32);
            return p;
        }
        let s = win32(h, CREATE_SEMAPHORE_W, &[0, count, count, 0]);
        h.write(q, s);
        h.write(p, q);
        p
    }

    /// `ChangeThreadPriority_ps2_006f3050(handle, prio)` (name ours):
    /// `SetThreadPriority` of the thread; its `eax`.
    pub fn set_thread_priority(h: &mut impl Host, handle: u32, prio: u32) -> u32 {
        let o: u32 = h.read(handle);
        let th: u32 = h.read(at(o, 4));
        win32(h, SET_THREAD_PRIORITY, &[th, prio])
    }

    /// `(i + 7) mod 8` as the original's signed remainder.
    fn prev8(i: u32) -> u32 {
        (i.wrapping_add(7) as i32).wrapping_rem(8) as u32
    }

    /// `AsyncJobQueue_push_006db800(this; job, q)` (name ours; thiscall,
    /// pops 8): queue `q` (`this + q * 0x1e8`): its lock `+0x1fc` taken,
    /// the job stored one slot back (`+0x1f4` = `(+0x1f4 + 7) % 8`, the
    /// slots from `+0x1d0`), the count `+0x1f8` raised, the lock released
    /// and the queue's event semaphore `+0x234` signalled. Returns that
    /// `ReleaseSemaphore`'s `eax`.
    pub fn job_push(h: &mut impl Host, this: u32, job: u32, q: u32) -> u32 {
        let o = this.wrapping_add(q.wrapping_mul(0x1e8));
        let lock: u32 = h.read(at(o, 0x1fc));
        win32(h, WAIT_FOR_SINGLE_OBJECT, &[lock, 0xffff_ffff]);
        let i = prev8(h.read(at(o, 0x1f4)));
        h.write(at(o, 0x1f4), i);
        h.write(at(o, 0x1d0 + i.wrapping_mul(4)), job);
        let n: u32 = h.read(at(o, 0x1f8));
        h.write(at(o, 0x1f8), n.wrapping_add(1));
        let lock: u32 = h.read(at(o, 0x1fc));
        win32(h, RELEASE_SEMAPHORE, &[lock, 1, 0]);
        let ev: u32 = h.read(at(o, 0x234));
        win32(h, RELEASE_SEMAPHORE, &[ev, 1, 0])
    }

    /// `AsyncJobQueue_pop_006db8a0(this; q)` (name ours; thiscall, pops
    /// 4): waits for queue `q`'s event (`+0x230`) and lock (`+0x22c`),
    /// takes the job one slot back from `+0x220` (count `+0x228` lowered),
    /// releases the lock. Returns the job.
    pub fn job_pop(h: &mut impl Host, this: u32, q: u32) -> u32 {
        let o = this.wrapping_add(q.wrapping_mul(0x1e8));
        let ev: u32 = h.read(at(o, 0x230));
        win32(h, WAIT_FOR_SINGLE_OBJECT, &[ev, 0xffff_ffff]);
        let b = at(o, 0x200);
        let lock: u32 = h.read(at(b, 0x2c));
        win32(h, WAIT_FOR_SINGLE_OBJECT, &[lock, 0xffff_ffff]);
        let i = prev8(h.read(at(b, 0x20)));
        let n: u32 = h.read(at(b, 0x28));
        h.write(at(b, 0x28), n.wrapping_sub(1));
        h.write(at(b, 0x20), i);
        let job: u32 = h.read(at(b, i.wrapping_mul(4)));
        let lock: u32 = h.read(at(b, 0x2c));
        win32(h, RELEASE_SEMAPHORE, &[lock, 1, 0]);
        job
    }

    /// `Ps2Thread_dtor_006f29a0(this; flags)` (name ours; thiscall, pops
    /// 4; the thread object's vtable slot 0): vtable `0x00b44e2c`, a
    /// running thread terminated and closed (`+4`, `+8` cleared), freed
    /// with flag 1. Returns `this`.
    pub fn thread_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(0x00b4_4e2c);
        let th: u32 = h.read(at(this, 4));
        h.write(this, vt);
        if th != 0 {
            win32(h, TERMINATE_THREAD, &[th, 0]);
            let th: u32 = h.read(at(this, 4));
            win32(h, CLOSE_HANDLE, &[th]);
            h.write(at(this, 4), [0u32, 0]);
        }
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `PollSema_ps2_006f2d50(handle)` (name ours): null returns 0; else
    /// `WaitForSingleObject(sema, 1)` until it stops failing (-1); 1 when
    /// it was signalled (0).
    pub fn poll_sema(h: &mut impl Host, handle: u32) -> u32 {
        if handle == 0 {
            return 0;
        }
        let o: u32 = h.read(handle);
        loop {
            let s: u32 = h.read(o);
            let r = win32(h, WAIT_FOR_SINGLE_OBJECT, &[s, 1]);
            if r != 0xffff_ffff {
                return u32::from(r == 0);
            }
        }
    }

    /// `Mutex_Create()` (`0x006f2de0`): a 4-byte object holding
    /// `CreateMutexW(0, 0, 0)`, or null.
    pub fn mutex_create(h: &mut impl Host) -> u32 {
        let p = cdecl(h, OPERATOR_NEW, &[4]);
        if p == 0 {
            return 0;
        }
        let m = win32(h, CREATE_MUTEX_W, &[0, 0, 0]);
        h.write(p, m);
        p
    }

    /// `Mutex_Lock(m)` (`0x006f2ed0`): `WaitForSingleObject(*m, INFINITE)`.
    pub fn mutex_lock(h: &mut impl Host, m: u32) -> u32 {
        let x: u32 = h.read(m);
        win32(h, WAIT_FOR_SINGLE_OBJECT, &[x, 0xffff_ffff])
    }

    /// `Mutex_Unlock(m)` (`0x006f2f50`): `ReleaseMutex(*m)`.
    pub fn mutex_unlock(h: &mut impl Host, m: u32) -> u32 {
        let x: u32 = h.read(m);
        win32(h, RELEASE_MUTEX, &[x])
    }

    /// `SignalSema_copy_006f3000(handle)`: null returns 0; else
    /// `ReleaseSemaphore(sema, 1, 0)`, 1.
    pub fn signal_sema(h: &mut impl Host, handle: u32) -> u32 {
        if handle == 0 {
            return 0;
        }
        let o: u32 = h.read(handle);
        let s: u32 = h.read(o);
        win32(h, RELEASE_SEMAPHORE, &[s, 1, 0]);
        1
    }

    pub const IGGY_MENU_CTOR: u32 = 0x00a2_88f0;
    pub const IGGY_MENU_SET_ENABLE: u32 = 0x00a2_8ef0;
    /// The overlay menu singleton (`0x00ce81fc`).
    pub const OVERLAY_MENU: Global<u32> = Global::new(0x00ce_81fc);

    /// `OverlayMenu_create_006f3130(path)` (name ours): once, a 0xa0-byte
    /// `IggyMenu` (`IggyMenu_ctor(path, 0.0, 0.0, 1.0, 1.0, 1)`, vtable
    /// `0x00b4bc04`, disabled) into `0x00ce81fc` (null when the allocation
    /// fails). The SEH frame (unwinding the allocation) is left out.
    pub fn overlay_menu_create(h: &mut impl Host, path: u32) {
        if OVERLAY_MENU.get(h) != 0 {
            return;
        }
        let p = cdecl(h, OPERATOR_NEW, &[0xa0]);
        if p != 0 {
            let one = 0x3f80_0000;
            h.call_sym(
                IGGY_MENU_CTOR,
                Conv::Thiscall(p),
                &[path, 0, 0, one, one, 1],
            );
            let vt = h.rebase(0x00b4_bc04);
            h.write(p, vt);
            h.call_sym(IGGY_MENU_SET_ENABLE, Conv::Thiscall(p), &[0]);
        }
        OVERLAY_MENU.set(h, p);
    }

    // ---- the debug host files (sce* on host0:) ----

    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_WRITE: u32 = 0x0062_fdc0;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const ATEL_STRCPY: u32 = 0x0087_1980;
    pub const ATEL_STRCAT: u32 = 0x0087_1950;
    pub const TK_SYNC_V: u32 = 0x0088_7f00;
    /// The host prefix `"host0:"`-style string (`0x00b59c78`).
    pub const HOST_PREFIX: u32 = 0x00b5_9c78;

    /// `printf2_dump_0076e890(buf, size)` (name ours): the buffer written
    /// to the dump file (`0x00b521ac`): created (`0x602`) the first time
    /// (`0x01128aae` clear) else appended (`0x402`, seek to the end);
    /// failing, the message. Returns the last call's `eax`.
    pub fn dump_write(h: &mut impl Host, buf: u32, size: u32) -> u32 {
        let first = h.read::<u8>(h.rebase(0x0112_8aae)) == 0;
        let name = h.rebase(0x00b5_21ac);
        let fd = cdecl(h, SCE_OPEN, &[name, if first { 0x602 } else { 0x402 }]);
        if (fd as i32) < 0 {
            let m = h.rebase(0x00b5_21c0);
            return cdecl(h, DBG_PRINTF, &[m]);
        }
        cdecl(h, SCE_LSEEK, &[fd, 0, if first { 0 } else { 2 }]);
        cdecl(h, SCE_WRITE, &[fd, buf, size]);
        cdecl(h, SCE_CLOSE, &[fd])
    }

    /// The host path `prefix + name` at `buf` (`AtelStrcpy`, `AtelStrcat`)
    /// and a cache flush.
    fn host_path(h: &mut impl Host, buf: u32, name: u32) {
        let p = h.rebase(HOST_PREFIX);
        cdecl(h, ATEL_STRCPY, &[buf, p]);
        cdecl(h, ATEL_STRCAT, &[buf, name]);
        cdecl(h, FLUSH_CACHE, &[0]);
    }

    /// `AtelPCClose(fd; slots)` (`0x00879c30`): a set `fd` closed, then
    /// `FlushCache` with the caller's argument word cleared (the
    /// original tail-jumps through it); 0 for none.
    pub fn pc_close(h: &mut impl Host, fd: u32, slots: u32) -> u32 {
        if fd == 0 {
            return 0;
        }
        cdecl(h, SCE_CLOSE, &[fd]);
        h.write(slots, 0u32);
        cdecl(h, FLUSH_CACHE, &[0])
    }

    /// `AtelPCCreate(name)` (`0x00879c60`; the path at `buf`, the
    /// original's `ebp - 0x104`): `sceOpen(path, 0x203)` up to twice,
    /// `TkSyncV(1)` after each; returns the descriptor.
    pub fn pc_create(h: &mut impl Host, buf: u32, name: u32) -> u32 {
        host_path(h, buf, name);
        let mut tries = 2;
        loop {
            let fd = cdecl(h, SCE_OPEN, &[buf, 0x203]);
            tries -= 1;
            cdecl(h, TK_SYNC_V, &[1]);
            if fd != 0 || tries <= 0 {
                return fd;
            }
        }
    }

    /// `AtelPCOpen(name)` (`0x00879cf0`; path at `buf`): `sceOpen(path,
    /// 1)` up to twice.
    pub fn pc_open(h: &mut impl Host, buf: u32, name: u32) -> u32 {
        host_path(h, buf, name);
        let mut tries = 2;
        loop {
            let fd = cdecl(h, SCE_OPEN, &[buf, 1]);
            tries -= 1;
            if fd != 0 || tries <= 0 {
                return fd;
            }
        }
    }

    /// `AtelPCOpen2(name)` (`0x00879d60`; path at `buf`): `sceOpen(path,
    /// 3)`, else `sceOpen(path, 0x203)`.
    pub fn pc_open2(h: &mut impl Host, buf: u32, name: u32) -> u32 {
        host_path(h, buf, name);
        let fd = cdecl(h, SCE_OPEN, &[buf, 3]);
        if fd != 0 {
            return fd;
        }
        cdecl(h, SCE_OPEN, &[buf, 0x203])
    }

    /// `AtelPCRead(fd, buf, n)` (`0x00879de0`): `sceRead`, a flush; the
    /// read's result.
    pub fn pc_read(h: &mut impl Host, fd: u32, buf: u32, n: u32) -> u32 {
        let r = cdecl(h, SCE_READ, &[fd, buf, n]);
        cdecl(h, FLUSH_CACHE, &[0]);
        r
    }

    /// `AtelPCSize(fd)` (`0x00879e10`): the file's size (seek to the end
    /// and back).
    pub fn pc_size(h: &mut impl Host, fd: u32) -> u32 {
        let cur = cdecl(h, SCE_LSEEK, &[fd, 0, 1]);
        let end = cdecl(h, SCE_LSEEK, &[fd, 0, 2]);
        cdecl(h, SCE_LSEEK, &[fd, cur, 0]);
        end
    }

    /// `AtelPCWrite(fd, buf, n)` / `AtelPClseek(fd, off, whence)`
    /// (`0x00879e50` / `0x00879e60`): tail jumps to `sceWrite` /
    /// `sceLseek`.
    pub fn pc_forward(h: &mut impl Host, target: u32, a: u32, b: u32, c: u32) -> u32 {
        cdecl(h, target, &[a, b, c])
    }

    pub const ATEL_PC_OPEN: u32 = 0x0087_9cf0;
    pub const ATEL_PC_OPEN2: u32 = 0x0087_9d60;
    pub const ATEL_PC_CLOSE: u32 = 0x0087_9c30;
    pub const ATEL_PC_WRITE: u32 = 0x0087_9e50;
    pub const ATEL_PC_LSEEK: u32 = 0x0087_9e60;

    /// `AtelOutputEncountStart()` (`0x00885a80`; the line at `buf`, the
    /// original's `ebp - 0x404`): the encounter log (`0x00c545ec`) opened
    /// (`AtelPCOpen2`), and when open, a header line `sprintf`ed and
    /// appended, the file closed. Returns the close's `eax` or the
    /// descriptor.
    pub fn output_encount_start(h: &mut impl Host, buf: u32) -> u32 {
        let name = h.rebase(0x00c5_45ec);
        let fd = cdecl(h, ATEL_PC_OPEN2, &[name]);
        if (fd as i32) < 0 {
            return fd;
        }
        cdecl(h, ATEL_PC_LSEEK, &[fd, 0, 2]);
        let fmt = h.rebase(0x00b5_d924);
        crt(h, SPRINTF, &[buf, fmt]);
        let mut n = 0u32;
        while h.read::<u8>(at(buf, n)) != 0 {
            n += 1;
        }
        cdecl(h, ATEL_PC_WRITE, &[fd, buf, n]);
        cdecl(h, ATEL_PC_CLOSE, &[fd])
    }

    /// The event names' file entry buffer (`0x01328958`).
    pub const EVENT_NAME: u32 = 0x0132_8958;

    /// `AtelGetEventName(i)` (`0x00879760`; its 8-byte index entry at
    /// `entry`, the original's `ebp - 0x2004`): entry `i` of the name
    /// index file (`0x00c533b8`) (offset, length), the name read to
    /// `0x01328958`, the file closed when open; returns the name.
    pub fn get_event_name(h: &mut impl Host, entry: u32, i: u32) -> u32 {
        let f = h.rebase(0x00c5_33b8);
        let fd = cdecl(h, ATEL_PC_OPEN, &[f]);
        cdecl(h, SCE_LSEEK, &[fd, 0, 0]);
        cdecl(h, SCE_LSEEK, &[fd, i.wrapping_mul(8), 0]);
        cdecl(h, SCE_READ, &[fd, entry, 8]);
        cdecl(h, FLUSH_CACHE, &[0]);
        let [off, len]: [u32; 2] = h.read(entry);
        cdecl(h, SCE_LSEEK, &[fd, off, 0]);
        let name = h.rebase(EVENT_NAME);
        cdecl(h, SCE_READ, &[fd, name, len]);
        cdecl(h, FLUSH_CACHE, &[0]);
        if fd != 0 {
            cdecl(h, SCE_CLOSE, &[fd]);
            cdecl(h, FLUSH_CACHE, &[0]);
        }
        let m = h.rebase(0x00b5_c558);
        cdecl(h, DBG_PRINTF, &[m, i, name]);
        name
    }

    pub const ATEL_GET_EVENT_NAME: u32 = 0x0087_9760;
    pub const ATEL_EVENT_GET_FILE_PATH2: u32 = 0x0087_95b0;
    pub const TK_MEM_LOCK: u32 = 0x0088_e680;
    pub const TK_MEM_UNLOCK: u32 = 0x0088_e6b0;
    /// The debug data buffers (7 slots of 4 MB, `0x01328598`), the source
    /// buffers (`0x01328698`, `0x013285b8`) and their sizes
    /// (`0x01328778`).
    pub const DEBUG_DATA: u32 = 0x0132_8598;
    pub const SOURCE_BUFS: u32 = 0x0132_8698;
    pub const SOURCE_BUFS2: u32 = 0x0132_85b8;
    pub const SOURCE_SIZES: u32 = 0x0132_8778;

    /// The file's size with the cursor kept (`AtelPCSize` inline).
    fn size_of(h: &mut impl Host, fd: u32) -> u32 {
        let cur = cdecl(h, SCE_LSEEK, &[fd, 0, 1]);
        let end = cdecl(h, SCE_LSEEK, &[fd, 0, 2]);
        cdecl(h, SCE_LSEEK, &[fd, cur, 0]);
        end
    }

    /// Reads the open file `fd` (at most `max` bytes) into `buf` between
    /// `TkMemUnlock(buf)` (before or after the size is taken) and
    /// `TkMemLock(buf)`, closing it; returns the size read and the lock's
    /// `eax`.
    fn read_locked(
        h: &mut impl Host,
        fd: u32,
        buf: u32,
        max: i32,
        unlock_first: bool,
    ) -> (u32, u32) {
        if unlock_first {
            cdecl(h, TK_MEM_UNLOCK, &[buf]);
        }
        let n = size_of(h, fd) as i32;
        if !unlock_first {
            cdecl(h, TK_MEM_UNLOCK, &[buf]);
        }
        let n = if n < max { n } else { max } as u32;
        cdecl(h, SCE_READ, &[fd, buf, n]);
        cdecl(h, FLUSH_CACHE, &[0]);
        cdecl(h, SCE_CLOSE, &[fd]);
        cdecl(h, FLUSH_CACHE, &[0]);
        (n, cdecl(h, TK_MEM_LOCK, &[buf]))
    }

    /// `AtelLoadDebugData(slot, event)` (`0x008799b0`): the event's debug
    /// file (`AtelEventGetFilePath2(1, dir, 0, name, ext)`) read into
    /// debug slot `slot` (at most 0x3fffff bytes); 1, or 0 when it does
    /// not open.
    pub fn load_debug_data(h: &mut impl Host, slot: u32, event: u32) -> u32 {
        let name = cdecl(h, ATEL_GET_EVENT_NAME, &[event]);
        let dir = h.rebase(0x00b5_9148);
        let ext = h.rebase(0x00b5_c588);
        let path = cdecl(h, ATEL_EVENT_GET_FILE_PATH2, &[1, dir, 0, name, ext]);
        let fd = cdecl(h, ATEL_PC_OPEN, &[path]);
        if fd == 0 {
            return 0;
        }
        let buf: u32 = h.read(h.rebase(DEBUG_DATA).wrapping_add(slot.wrapping_mul(4)));
        read_locked(h, fd, buf, 0x3f_ffff, true);
        1
    }

    /// `Atel_LoadSourceFile_00879b60(a, _, b, name)`: the source file
    /// read into source buffer `b + a * 8` (at most 0xfffff bytes for
    /// `a` 0, else 0x1ffff), its size kept (0 when it does not open).
    /// Returns `eax` as the original leaves it (the lock's, or the
    /// index).
    pub fn load_source_file(h: &mut impl Host, a: u32, b: u32, name: u32) -> u32 {
        let fd = cdecl(h, ATEL_PC_OPEN, &[name]);
        let idx = b.wrapping_add(a.wrapping_mul(8));
        let sizes = h.rebase(SOURCE_SIZES).wrapping_add(idx.wrapping_mul(4));
        if fd == 0 {
            h.write(sizes, 0u32);
            return idx;
        }
        let max = if a == 0 { 0xf_ffff } else { 0x1_ffff };
        let buf: u32 = h.read(h.rebase(SOURCE_BUFS).wrapping_add(idx.wrapping_mul(4)));
        let (n, eax) = read_locked(h, fd, buf, max, false);
        h.write(sizes, n);
        eax
    }

    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE_THUNK: u32 = 0x0068_7010;
    pub const TK_IS_DEBUG: u32 = 0x0088_7d10;

    /// `AtelRemovePS2Debug()` (`0x00879e70`): every debug data buffer (7)
    /// and source buffer pair (56) freed (`user_free_thunk`) when set.
    /// Returns `eax` as the original leaves it.
    pub fn remove_ps2_debug(h: &mut impl Host) -> u32 {
        let data = h.rebase(DEBUG_DATA);
        let a = h.rebase(SOURCE_BUFS);
        let b = h.rebase(SOURCE_BUFS2);
        let mut eax = 0;
        let mut k = 0u32;
        for slot in 0..7u32 {
            let p: u32 = h.read(at(data, slot * 4));
            eax = p;
            if p != 0 {
                eax = cdecl(h, USER_FREE_THUNK, &[p]);
            }
            for _ in 0..8 {
                for base in [a, b] {
                    let p: u32 = h.read(at(base, k * 4));
                    eax = p;
                    if p != 0 {
                        eax = cdecl(h, USER_FREE_THUNK, &[p]);
                    }
                }
                k += 1;
            }
        }
        eax
    }

    /// `AtelInitPS2Debug()` (`0x008798f0`): debug slots 0 and 6 get 4 MB
    /// each, slot 0's eight source pairs 1 MB, slot 6's 128 KB (each
    /// locked with `TkMemLock(first)`, the call's other words being the
    /// sizes); then tail-jumps to `AtelInitDebugger_0087f0d0`.
    pub fn init_ps2_debug(h: &mut impl Host) -> u32 {
        let a = h.rebase(SOURCE_BUFS);
        let b = h.rebase(SOURCE_BUFS2);
        for slot in [0u32, 6] {
            let p = cdecl(h, USER_MALLOC, &[0x40_0000]);
            h.write(h.rebase(DEBUG_DATA).wrapping_add(slot * 4), p);
            cdecl(h, TK_MEM_LOCK, &[p, 0x40_0000]);
            let size = if slot == 0 { 0x10_0000 } else { 0x2_0000 };
            for i in 0..8u32 {
                let k = i + slot * 8;
                let x = cdecl(h, USER_MALLOC, &[size]);
                h.write(at(a, k * 4), x);
                let y = cdecl(h, USER_MALLOC, &[size]);
                h.write(at(b, k * 4), y);
                let x: u32 = h.read(at(a, k * 4));
                cdecl(h, TK_MEM_LOCK, &[x, size, size]);
            }
        }
        h.call_sym(INIT_DEBUGGER, Conv::Cdecl, &[])
    }

    pub const INIT_DEBUGGER: u32 = 0x0087_f0d0;
    pub const CLEAR_DEBUGGER: u32 = 0x0087_f1c0;
    pub const SET_GLOBALS_0086E970: u32 = 0x0086_e970;
    pub const TK_SET_START_MAP_INDEX: u32 = 0x0088_ebf0;

    /// `AtelClearDebugger_0087f1c0()` (name ours): the debugger's five
    /// 0xaa4-byte windows from `0x01328af0` cleared: each one's header
    /// words and its 42 0x40-byte rows (their words, the pair at
    /// `+0x50`/`+0x54` set to -1). Returns `eax` as the original leaves it
    /// (0xa80).
    pub fn clear_debugger(h: &mut impl Host) -> u32 {
        let first = h.rebase(0x0132_8afc);
        let end = h.rebase(0x0132_c030);
        let mut c = first;
        while c < end {
            h.write(c.wrapping_sub(0xc), 0u32);
            h.write(c.wrapping_sub(4), 0u32);
            h.write(c, [0u32; 6]);
            let mut e = 0u32;
            while e < 0xa80 {
                for o in [
                    0x18u32, 0x20, 0x3c, 0x38, 0x48, 0x1c, 0x28, 0x2c, 0x34, 0x4c,
                ] {
                    h.write(at(c, e + o), 0u32);
                }
                h.write(at(c, e + 0x50), [0xffff_ffffu32, 0xffff_ffff]);
                e += 0x40;
            }
            c = c.wrapping_add(0xaa4);
        }
        0xa80
    }

    /// `AtelInitDebugger_0087f0d0()` (name ours): the debugger's state
    /// reset: its table cleared, the callbacks set
    /// (`SetGlobals_0086e970(0x0087e720, 0x0087e6f0)`), a 4x4 identity
    /// matrix at `0x01328ab0`, the font colours (0x80) and flags, then
    /// `TkSetStartMapIndex(0x17, 0)` (its `eax`).
    pub fn init_debugger(h: &mut impl Host) -> u32 {
        h.write(h.rebase(0x0132_8a78), 0u8);
        cdecl(h, CLEAR_DEBUGGER, &[]);
        let a = h.rebase(0x0087_e720);
        let b = h.rebase(0x0087_e6f0);
        cdecl(h, SET_GLOBALS_0086E970, &[a, b]);
        let m = h.rebase(0x0132_8ab0);
        for i in 0..16u32 {
            let v = if i % 5 == 0 { 0x3f80_0000u32 } else { 0 };
            h.write(at(m, i * 4), v);
        }
        for (sym, v) in [
            (0x0132_8aa8u32, 0u32),
            (0x0132_8a7c, 0),
            (0x0132_8a80, 0x88),
            (0x0132_8a88, 0),
            (0x0132_8a8c, 0x80),
            (0x0132_8a90, 0x80),
            (0x0132_8a98, 0x80),
            (0x0132_8a94, 0x80),
        ] {
            h.write(h.rebase(sym), v);
        }
        h.write(h.rebase(0x0132_8a7a), 1u8);
        h.write(h.rebase(0x0132_8a79), 0u8);
        h.write(h.rebase(0x0132_8a9c), 1u32);
        h.write(h.rebase(0x0132_8a78), 0u8);
        h.write(h.rebase(0x0132_8a70), 0u32);
        h.write(h.rebase(0x0132_8aa4), 0u32);
        cdecl(h, TK_SET_START_MAP_INDEX, &[0x17, 0])
    }

    /// `AtelInitTotalDebug_00873ba0()` (name ours): six words cleared,
    /// then in debug mode tail-jumps to `AtelInitPS2Debug` (else returns
    /// `TkIsDebug`'s 0).
    pub fn init_total_debug(h: &mut impl Host) -> u32 {
        for sym in [
            0x023c_bd78u32,
            0x023c_bd70,
            0x023c_bd68,
            0x023c_bd7c,
            0x023c_bd74,
            0x023c_bd6c,
        ] {
            h.write(h.rebase(sym), 0u32);
        }
        let d = cdecl(h, TK_IS_DEBUG, &[]);
        if d == 0 {
            return 0;
        }
        h.call_sym(ATEL_INIT_PS2_DEBUG, Conv::Cdecl, &[])
    }

    pub const ATEL_INIT_PS2_DEBUG: u32 = 0x0087_98f0;

    // ---- script helpers ----

    pub const ATEL_GET_BASIC_WORK: u32 = 0x0086_a830;
    pub const ATEL_UC_OFF: u32 = 0x0087_19e0;
    pub const ATEL_UC_ON: u32 = 0x0087_1ab0;
    pub const EVENT_SET_PLAYER: u32 = 0x0086_f6a0;
    pub const TK_GET_SAVE_EVENT_ADDRESS: u32 = 0x0088_c830;
    pub const ATEL_REGIST_CALLBACK_QUE2: u32 = 0x0087_51b0;
    /// `g_ScriptContext`, the current control work.
    pub const SCRIPT_CONTEXT: Global<u32> = Global::new(0x0132_6b28);

    /// The control work's control-flag refresh after a change: bit 1 of
    /// its first byte becomes `bit` again after `AtelUcOff` and, unless
    /// paused (`+1` bit 2, `+3` bits 2 and 7), `AtelUcOn` when `on`.
    /// Returns the last call's `eax`.
    fn uc_refresh(h: &mut impl Host, bit: u8, on: bool) -> u32 {
        let mut eax = cdecl(h, ATEL_UC_OFF, &[]);
        let mut c = SCRIPT_CONTEXT.get(h);
        if h.read::<u8>(at(c, 1)) & 4 == 0 && h.read::<u8>(at(c, 3)) & 0x84 == 0 && on {
            eax = cdecl(h, ATEL_UC_ON, &[]);
            c = SCRIPT_CONTEXT.get(h);
        }
        let b: u8 = h.read(c);
        h.write(c, b ^ ((bit << 1) ^ b) & 2);
        eax
    }

    /// `AtelSetCtrlChar(id)` (`0x0086f580`): makes worker `id` the
    /// controlled character (a negative `id` restores the one saved at
    /// `+0xe`, keeping flag 0x40 for the save): the old player's
    /// controlled flag (`+0x36` bit 3) dropped and the control flags
    /// refreshed, then the new one set (`Event_SetPlayer`, flag 0x20) or
    /// none (0x20 cleared), the flags refreshed again. Returns the last
    /// call's `eax`.
    pub fn set_ctrl_char(h: &mut impl Host, id: u32) -> u32 {
        let mut c = SCRIPT_CONTEXT.get(h);
        let mut id = id;
        let b: u8 = h.read(c);
        if (id as i32) >= 0 {
            if b & 0x40 == 0 {
                let p: u16 = h.read(at(c, 0xa));
                h.write(at(c, 0xe), p);
                h.write(c, b | 0x40);
            }
        } else if b & 0x40 != 0 {
            id = u32::from(h.read::<u16>(at(c, 0xe)));
            h.write(c, b & 0xbf);
        } else {
            id = 0xffff_ffff;
        }
        let p = u32::from(h.read::<u16>(at(c, 0xa)));
        if p != 0xffff {
            let w = cdecl(h, ATEL_GET_BASIC_WORK, &[p]);
            let f: u16 = h.read(at(w, 0x36));
            h.write(at(w, 0x36), f & 0xfff7);
            let b: u8 = h.read(c);
            let bit = (b >> 1) & 1;
            let on = b & 0x22 != 0;
            uc_refresh(h, bit, on);
            c = SCRIPT_CONTEXT.get(h);
        }
        if (id as i32) >= 0 && id != 0xffff {
            cdecl(h, EVENT_SET_PLAYER, &[id]);
            let w = cdecl(h, ATEL_GET_BASIC_WORK, &[id]);
            let f: u16 = h.read(at(w, 0x36));
            h.write(at(w, 0x36), f | 8);
            let b: u8 = h.read(c);
            h.write(c, b | 0x20);
        } else {
            let b: u8 = h.read(c);
            h.write(c, b & 0xdf);
        }
        let b: u8 = h.read(c);
        uc_refresh(h, (b >> 1) & 1, b & 0x22 != 0)
    }

    /// `SaveEvent_SetStep_00873db0()` (name ours): the save event's step
    /// `+0xe0` = 6 and `+0xd8` = -1; the second lookup's `eax`.
    pub fn save_event_step6(h: &mut impl Host) -> u32 {
        let e = cdecl(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
        h.write(at(e, 0xe0), 6u32);
        let e = cdecl(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
        h.write(at(e, 0xd8), 0xffff_ffffu32);
        e
    }

    /// The save event's step `+0xe0` set to `to` when it is 2..=5
    /// (`0x00873c40` 4, `0x00873d30` 1). Returns `eax` as left.
    pub fn save_event_step(h: &mut impl Host, to: u32) -> u32 {
        let e = cdecl(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
        let s = h.read::<u32>(at(e, 0xe0)).wrapping_sub(2);
        if s > 3 {
            return s;
        }
        let e = cdecl(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
        h.write(at(e, 0xe0), to);
        e
    }

    /// `Atel_GetWorkerTableWord_0086c250(worker, i)` (name ours): word `i`
    /// of the worker's table (`*(*(w) + 0x20) + w[1]`).
    pub fn worker_table_word(h: &mut impl Host, worker: u32, i: u32) -> u32 {
        let w = cdecl(h, ATEL_GET_BASIC_WORK, &[worker]);
        let head: u32 = h.read(w);
        let t: u32 = h.read(at(head, 0x20));
        let base = t.wrapping_add(h.read(at(w, 4)));
        h.read(at(base, i.wrapping_mul(4)))
    }

    /// `AtelRegistDebugCallbacks_00861440(keep)` (name ours): registers
    /// `0x00861480` and, unless `keep`, `0x00861510`
    /// (`AtelRegistCallbackQue2(0, f, name)`). Returns the last call's
    /// `eax`.
    pub fn regist_debug_callbacks(h: &mut impl Host, keep: u32) -> u32 {
        let f = h.rebase(0x0086_1480);
        let n = h.rebase(0x00b5_be1c);
        let r = cdecl(h, ATEL_REGIST_CALLBACK_QUE2, &[0, f, n]);
        if keep != 0 {
            return r;
        }
        let f = h.rebase(0x0086_1510);
        let n = h.rebase(0x00b5_be24);
        cdecl(h, ATEL_REGIST_CALLBACK_QUE2, &[0, f, n])
    }

    // ---- battle helpers ----

    pub const TK_FREE: u32 = 0x0088_d520;
    pub const TK_MALLOC_AND_READ_EVENT_MESSAGE: u32 = 0x0088_cac0;
    pub const TK_MALLOC_AND_READ_EVENT_FTC: u32 = 0x0088_c860;
    pub const BTL_FILE_NAME: u32 = 0x0231_12e0;
    pub const BTL_TEXT: Global<u32> = Global::new(0x0231_12c4);
    pub const BTL_FONT: Global<u32> = Global::new(0x0231_12c0);
    pub const BTL_NOT_SYSTEM: Global<u32> = Global::new(0x0231_12bc);

    /// `MsBtlReadAsyncTask_00784500()`: the battle file's name cut to its
    /// last directory part after the `"btl/"`-style marker (`strstr`,
    /// `+0xc`, the last `'/'` ended), moved to the name's start; the
    /// battle's text and font reloaded (`TKMallocAndReadEventMessage`,
    /// `...FTC`), the old ones freed unless the system file's. Returns
    /// the last value set (0 when the text is missing).
    pub fn btl_read_async(h: &mut impl Host) -> u32 {
        let name = h.rebase(BTL_FILE_NAME);
        let mark = h.rebase(0x00b5_48e0);
        let p = crt(h, STRSTR, &[name, mark]).wrapping_add(0xc);
        let q = crt(h, STRRCHR, &[p, u32::from(b'/')]);
        h.write(q, 0u8);
        let mut i = 0u32;
        loop {
            let b: u8 = h.read(at(p, i));
            h.write(at(name, i), b);
            i += 1;
            if b == 0 {
                break;
            }
        }
        let old = BTL_TEXT.get(h);
        if old != 0 {
            if BTL_NOT_SYSTEM.get(h) != 0 {
                cdecl(h, TK_FREE, &[old]);
            }
            BTL_TEXT.set(h, 0);
        }
        let t = cdecl(h, TK_MALLOC_AND_READ_EVENT_MESSAGE, &[name]);
        BTL_TEXT.set(h, t);
        if t == 0 {
            return 0;
        }
        let old = BTL_FONT.get(h);
        if old != 0 {
            if BTL_NOT_SYSTEM.get(h) != 0 {
                cdecl(h, TK_FREE, &[old]);
            }
            BTL_FONT.set(h, 0);
        }
        let f = cdecl(h, TK_MALLOC_AND_READ_EVENT_FTC, &[name]);
        BTL_FONT.set(h, f);
        f
    }

    pub const CHN_SYSTEM_MGRP_LOADED: u32 = 0x0083_7710;
    pub const CHN_SYSTEM_MGRP_RESET_TAIL: u32 = 0x0083_7690;

    /// `ChnSystemMgrpReset_0079cca0()` (name ours; `Native_70a4`'s body):
    /// `g_BtlRegMotionFlag` cleared; with `g_BtlRegMotion` set and the
    /// motion group loaded (`ChN_SystemMgrpLoaded`), `g_BtlRegMotion`
    /// cleared and `0x00837690` (a tail jump). The caller ignores `eax`.
    pub fn chn_system_mgrp_reset(h: &mut impl Host) {
        let reg = h.rebase(0x0113_606c);
        h.write(h.rebase(0x0112_c9cf), 0u8);
        if Mem::read::<u32>(h, reg) == 0 {
            return;
        }
        if cdecl(h, CHN_SYSTEM_MGRP_LOADED, &[]) == 0 {
            return;
        }
        h.write(reg, 0u32);
        cdecl(h, CHN_SYSTEM_MGRP_RESET_TAIL, &[]);
    }

    // ---- one-liners ----

    /// The `.rdata` address a table entry returns: `0x007829a0` returns
    /// `0x00b546a0`; `0x00860770`, `0x00878130`, `0x00879420` return
    /// `0x00b5b9fc` (an empty string).
    pub fn rdata_address(h: &impl Host, sym: u32) -> u32 {
        h.rebase(sym)
    }

    /// `MsBtlReadFileDone_00782fe0()`: `g_BtlStatus` = 7 (`eax` kept).
    pub fn btl_read_file_done(h: &mut impl Host) {
        h.write(h.rebase(0x0112_a8e0), 7u8);
    }

    /// `AtelGetGlobal01326b58_0086b060()` (name ours): the word at
    /// `0x01326b58`.
    pub fn global_01326b58(h: &impl Host) -> u32 {
        h.read(h.rebase(0x0132_6b58))
    }

    /// `AtelSetGlobal01326b81_008708d0()` (name ours): byte `0x01326b81` =
    /// 1 (`eax` kept).
    pub fn set_global_01326b81(h: &mut impl Host) {
        h.write(h.rebase(0x0132_6b81), 1u8);
    }
}
