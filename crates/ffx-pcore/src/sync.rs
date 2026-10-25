//! The thread primitives over Win32 (`PEventFlagWin32`,
//! `PSemaphoreWin32`, `PCriticalSectionSimple`), thread initialisation and
//! the thread pool's small members.
//!
//! Layouts (from the originals' stores):
//! - `PEventFlagWin32` / `PSemaphoreWin32`: `+0` the `HANDLE`.
//! - `PCriticalSectionBase`: `+0` byte "initialised", `+4` an event flag,
//!   `+8` byte; `PCriticalSectionSimple` adds `+0xc` the waiter count,
//!   `+0x10` the lock word, `+0x14` the lock count.
//! - `PThreadInitializationBase`: `+0` entry, `+4` stack size `0x10000`,
//!   `+8` 0, `+0xc` 1, `+0x10` the name (`0x00b0e060`), `+0x14` 1.
//! - `PThreadPool`: `+0` vtable `0x00b10250`, `+8` and `+0x14` the
//!   constructor's arguments, `+4 +0xc +0x10 +0x18` zero.

/// `KERNEL32!SetEvent`'s import slot.
pub const SET_EVENT: u32 = 0x00b0_c0ec;
/// `KERNEL32!ResetEvent`'s import slot.
pub const RESET_EVENT: u32 = 0x00b0_c0f0;
/// `KERNEL32!ReleaseSemaphore`'s import slot.
pub const RELEASE_SEMAPHORE: u32 = 0x00b0_c0f4;
/// `KERNEL32!WaitForSingleObject`'s import slot.
pub const WAIT: u32 = 0x00b0_c0f8;
/// `KERNEL32!CreateEventA`'s import slot.
pub const CREATE_EVENT: u32 = 0x00b0_c0fc;
/// `KERNEL32!CreateSemaphoreA`'s import slot.
pub const CREATE_SEMAPHORE: u32 = 0x00b0_c11c;
/// `KERNEL32!InterlockedExchangeAdd`'s import slot.
pub const INTERLOCKED_ADD: u32 = 0x00b0_c128;
/// `KERNEL32!InterlockedCompareExchange`'s import slot.
pub const INTERLOCKED_CAS: u32 = 0x00b0_c12c;
/// The event flag's other release (thiscall `(flag)`, not ported).
pub const EVENT_DESTROY: u32 = 0x0043_0240;
/// The default thread name `PThreadInitializationBase` stores.
pub const THREAD_NAME: u32 = 0x00b0_e060;
/// `PThreadPool`'s vtable.
pub const THREAD_POOL_VTABLE: u32 = 0x00b1_0250;
/// The thread pool's thread count.
pub const THREAD_COUNT: u32 = 0x00c0_b950;
/// The thread pool's stack size.
pub const STACK_SIZE: u32 = 0x00c0_b954;

pub mod bridge {
    //! The originals over game memory.

    use ffx_graphic::host::{Conv, Host};

    use super::{
        CREATE_EVENT, CREATE_SEMAPHORE, EVENT_DESTROY, INTERLOCKED_ADD, INTERLOCKED_CAS,
        RELEASE_SEMAPHORE, RESET_EVENT, SET_EVENT, STACK_SIZE, THREAD_COUNT, THREAD_NAME,
        THREAD_POOL_VTABLE, WAIT,
    };
    use crate::import;

    /// `PEventFlagWin32::PEventFlagWin32(this)` (also
    /// `PSemaphoreWin32::PSemaphoreWin32`): no handle; returns `this`.
    pub fn handle_init(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        this
    }

    /// `PEventFlagWin32::initialize(this, flags)`: `CreateEventA(0,
    /// manual = !(flags & 1), initial = flags & 2, 0)` into `+0`; 0.
    pub fn event_initialize(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let initial = (flags >> 1) & 1;
        let manual = !flags & 1;
        let e = import(h, CREATE_EVENT, &[0, manual, initial, 0]);
        h.write(this, e);
        0
    }

    /// `PSemaphoreWin32::initialize(this, initial, max)`:
    /// `CreateSemaphoreA(0, initial, min(max, 0x7fffffff), 0)` into `+0`; 0.
    pub fn semaphore_initialize(h: &mut impl Host, this: u32, initial: u32, max: u32) -> u32 {
        let max = max.min(0x7fff_ffff);
        let s = import(h, CREATE_SEMAPHORE, &[0, initial, max, 0]);
        h.write(this, s);
        0
    }

    /// `PEventFlagWin32::reset(this)`: `ResetEvent`; 0.
    pub fn event_reset(h: &mut impl Host, this: u32) -> u32 {
        let e: u32 = h.read(this);
        import(h, RESET_EVENT, &[e]);
        0
    }

    /// `PEventFlagWin32::signal(this)`: `SetEvent`; 0.
    pub fn event_signal(h: &mut impl Host, this: u32) -> u32 {
        let e: u32 = h.read(this);
        import(h, SET_EVENT, &[e]);
        0
    }

    /// `PSemaphoreWin32::signal(this, count, previous)`:
    /// `ReleaseSemaphore(handle, count, &local)`, the previous count into
    /// `*previous` when that succeeds and `previous` is set; 0. `local` is
    /// a word of `scratch` (the original's frame slot, which holds `this`
    /// before the call).
    pub fn semaphore_signal(
        h: &mut impl Host,
        scratch: u32,
        this: u32,
        count: u32,
        previous: u32,
    ) -> u32 {
        h.write(scratch, this);
        let s: u32 = h.read(this);
        let ok = import(h, RELEASE_SEMAPHORE, &[s, count, scratch]);
        if ok != 0 && previous != 0 {
            let v: u32 = h.read(scratch);
            h.write(previous, v);
        }
        0
    }

    /// `0x00430280(this, ms)`: `WaitForSingleObject(handle, ms)`; 0.
    pub fn wait_for(h: &mut impl Host, this: u32, ms: u32) -> u32 {
        let e: u32 = h.read(this);
        import(h, WAIT, &[e, ms]);
        0
    }

    /// `PEventFlagWin32::wait(this)` / `PSemaphoreWin32::wait(this)`:
    /// `WaitForSingleObject(handle, INFINITE)`; 0.
    pub fn wait(h: &mut impl Host, this: u32) -> u32 {
        wait_for(h, this, u32::MAX)
    }

    /// `PCriticalSectionSimple::PCriticalSectionSimple(this)`; `this`.
    pub fn critical_section_simple(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u8);
        handle_init(h, this + 4);
        h.write(this + 8, 0u8);
        h.write(this + 0xc, 0u32);
        h.write(this + 0x10, 0u32);
        h.write(this + 0x14, 0u32);
        this
    }

    /// `0x004316d0(this)`: an event flag at `+0`, byte `+4` clear; `this`.
    pub fn event_flag_with_byte(h: &mut impl Host, this: u32) -> u32 {
        handle_init(h, this);
        h.write(this + 4, 0u8);
        this
    }

    /// `PThreadInitializationBase::PThreadInitializationBase(this, entry)`
    /// (and `PThreadInitializationWin32`'s, which only forwards); `this`.
    pub fn thread_initialization(h: &mut impl Host, this: u32, entry: u32) -> u32 {
        h.write(this, entry);
        h.write(this + 4, 0x1_0000u32);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 1u32);
        let name = h.rebase(THREAD_NAME);
        h.write(this + 0x10, name);
        h.write(this + 0x14, 1u32);
        this
    }

    /// `PCriticalSectionBase::initialize(this)`: once, the event flag at
    /// `+4` (auto reset, clear) and byte `+8`; byte `+0` set when that
    /// succeeded. 0 when already initialised, else the event's result.
    pub fn critical_section_initialize(h: &mut impl Host, this: u32) -> u32 {
        if h.read::<u8>(this) != 0 {
            return 0;
        }
        let r = event_initialize(h, this + 4, 1);
        h.write(this + 8, 1u8);
        if r == 0 {
            h.write(this, 1u8);
        }
        r
    }

    /// `0x004318c0(this, lock)`: counts a waiter at `+0xc`, takes the lock
    /// word (`*lock` 0 -> 1, then `lock[1] += 1`), waiting 20 ms on the
    /// event at `+4` between tries; resets the event and uncounts the
    /// waiter. Returns `InterlockedExchangeAdd`'s result (the waiter count
    /// before the decrement).
    pub fn lock_wait(h: &mut impl Host, this: u32, lock: u32) -> u32 {
        import(h, INTERLOCKED_ADD, &[this + 0xc, 1]);
        loop {
            let old = import(h, INTERLOCKED_CAS, &[lock, 1, 0]);
            if old == 0 {
                let n: u32 = h.read(lock + 4);
                h.write(lock + 4, n.wrapping_add(1));
                break;
            }
            // the wait always returns 0: the original's exit on a
            // nonzero result (skipping the count) never runs
            if wait_for(h, this + 4, 0x14) != 0 {
                break;
            }
        }
        event_reset(h, this + 4);
        import(h, INTERLOCKED_ADD, &[this + 0xc, u32::MAX])
    }

    /// `PCriticalSectionSimple::lock(this)`: [`lock_wait`] on `+0x10`.
    pub fn critical_section_lock(h: &mut impl Host, this: u32) -> u32 {
        lock_wait(h, this, this + 0x10)
    }

    /// `0x00431950(this)`: when initialised, clears the byte, releases the
    /// event flag at `+4` (`0x00430240`) and clears byte `+8`.
    pub fn critical_section_release(h: &mut impl Host, this: u32) {
        if h.read::<u8>(this) != 0 {
            h.write(this, 0u8);
            h.call_sym(EVENT_DESTROY, Conv::Thiscall(this + 4), &[]);
            h.write(this + 8, 0u8);
        }
    }

    /// `PCriticalSectionSimple::unlock(this)`: one lock fewer, the lock
    /// word back to 0 (if 1); signals the event when someone waits (0),
    /// else returns `InterlockedCompareExchange`'s result.
    pub fn critical_section_unlock(h: &mut impl Host, this: u32) -> u32 {
        let n: u32 = h.read(this + 0x14);
        h.write(this + 0x14, n.wrapping_sub(1));
        let old = import(h, INTERLOCKED_CAS, &[this + 0x10, 0, 1]);
        if h.read::<i32>(this + 0xc) > 0 {
            return event_signal(h, this + 4);
        }
        old
    }

    /// `PThreadPool::PThreadPool(this, a, b)`; `this`.
    pub fn thread_pool(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        h.write(this + 8, a);
        h.write(this + 0x14, b);
        let vt = h.rebase(THREAD_POOL_VTABLE);
        h.write(this, vt);
        h.write(this + 4, 0u32);
        h.write(this + 0xc, 0u32);
        h.write(this + 0x10, 0u32);
        h.write(this + 0x18, 0u32);
        this
    }

    /// `PThreadPool::GetThreadCount()`.
    pub fn thread_count(h: &mut impl Host) -> u32 {
        h.read(h.rebase(THREAD_COUNT))
    }

    /// `0x0044ce10(n)`: sets the thread count; returns `n`.
    pub fn set_thread_count(h: &mut impl Host, n: u32) -> u32 {
        h.write(h.rebase(THREAD_COUNT), n);
        n
    }

    /// `PThreadPool::SetStackSize(n)`; returns `n`.
    pub fn set_stack_size(h: &mut impl Host, n: u32) -> u32 {
        h.write(h.rebase(STACK_SIZE), n);
        n
    }
}
