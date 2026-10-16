//! The PC port's thin wrappers over KERNEL32's threads and synchronisation
//! objects: sleeps, semaphores, events, critical sections and handle
//! closes, with the objects' handles kept in game structures.
//!
//! The Win32 calls go through [`Kernel`], which the host implements (the
//! hook through the game's import slots, the tests through the oracle's
//! stubs); each call gets the game memory first.

/// The KERNEL32 calls the wrappers make; each returns the `eax` the
/// import leaves (garbage for the `void` ones, which the wrappers pass on).
pub trait Kernel<M> {
    /// `Sleep(ms)`.
    fn sleep(&mut self, mem: &mut M, ms: u32) -> u32;
    /// `WaitForSingleObject(handle, ms)`.
    fn wait(&mut self, mem: &mut M, handle: u32, ms: u32) -> u32;
    /// `ReleaseSemaphore(handle, count, &previous)`.
    fn release_semaphore(&mut self, mem: &mut M, handle: u32, count: u32, previous: u32) -> u32;
    /// `CreateSemaphoreW(attributes, initial, maximum, name)`.
    fn create_semaphore(
        &mut self,
        mem: &mut M,
        attributes: u32,
        initial: u32,
        maximum: u32,
        name: u32,
    ) -> u32;
    /// `CloseHandle(handle)`.
    fn close_handle(&mut self, mem: &mut M, handle: u32) -> u32;
    /// `SetEvent(handle)`.
    fn set_event(&mut self, mem: &mut M, handle: u32) -> u32;
    /// `ResetEvent(handle)`.
    fn reset_event(&mut self, mem: &mut M, handle: u32) -> u32;
    /// `InitializeCriticalSection(cs)`.
    fn initialize_critical_section(&mut self, mem: &mut M, cs: u32) -> u32;
    /// `EnterCriticalSection(cs)`.
    fn enter_critical_section(&mut self, mem: &mut M, cs: u32) -> u32;
    /// `LeaveCriticalSection(cs)`.
    fn leave_critical_section(&mut self, mem: &mut M, cs: u32) -> u32;
    /// `DeleteCriticalSection(cs)`.
    fn delete_critical_section(&mut self, mem: &mut M, cs: u32) -> u32;
}

/// `INFINITE`.
pub const INFINITE: u32 = u32::MAX;

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{INFINITE, Kernel};

    /// `SleepThreadInMillisecond(ms)` (`0x006f2d20`): `Sleep(ms)`; returns 0.
    pub fn sleep_ms<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, ms: u32) -> u32 {
        k.sleep(mem, ms);
        0
    }

    /// `SleepThread()` (`0x006f2d10`): `Sleep(10)`; returns 0.
    pub fn sleep_thread<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>) -> u32 {
        k.sleep(mem, 10);
        0
    }

    /// `0x006f2a40(ms)`: `Sleep(ms)`, its `eax` passed on.
    pub fn sleep<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, ms: u32) -> u32 {
        k.sleep(mem, ms)
    }

    /// `SignalSema(&sema)` (`0x006f2ce0`): one count released on the
    /// semaphore whose handle `**sema` holds; 1, or 0 for null.
    pub fn signal_sema<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, sema: u32) -> u32 {
        if sema == 0 {
            return 0;
        }
        let h: u32 = mem.read(mem.read::<u32>(sema));
        k.release_semaphore(mem, h, 1, 0);
        1
    }

    /// `0x006f2d80(&sema)` and `0x006f2da0(&sema, ms)`: a wait on `**sema`
    /// (forever, or `ms`); 1, or 0 for null.
    pub fn wait_sema<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, sema: u32, ms: u32) -> u32 {
        if sema == 0 {
            return 0;
        }
        let h: u32 = mem.read(mem.read::<u32>(sema));
        k.wait(mem, h, ms);
        1
    }

    /// `0x00679ab0(n)` (`thiscall`, `ret 4`): `*this` = a new semaphore of
    /// `n` (initial and maximum); returns `this`.
    pub fn semaphore_ctor<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32, n: u32) -> u32 {
        let h = k.create_semaphore(mem, 0, n, n, 0);
        mem.write(this, h);
        this
    }

    /// `0x0061b500(request)` (`thiscall` on a queue, `ret 4`): a request's
    /// `+0x58` cleared and the queue's semaphore (`*this`) released once;
    /// what that returns, or 0 for no request.
    pub fn queue_post<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32, request: u32) -> u32 {
        if request == 0 {
            return 0;
        }
        mem.write(request.wrapping_add(0x58), 0u32);
        let h: u32 = mem.read(this);
        k.release_semaphore(mem, h, 1, 0)
    }

    /// `0x0061ba60(request)` (`ret 4`): a wait without end on the request's
    /// event (`+0x4c`); what that returns.
    pub fn request_wait<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, request: u32) -> u32 {
        let h: u32 = mem.read(request.wrapping_add(0x4c));
        k.wait(mem, h, INFINITE)
    }

    /// `0x00633a80(mutex)` (`thiscall`, `ret 4`): a lock guard: `*this` =
    /// `mutex`, a wait without end on `*mutex`; returns `this`.
    pub fn mutex_guard<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32, mutex: u32) -> u32 {
        mem.write(this, mutex);
        let h: u32 = mem.read(mutex);
        k.wait(mem, h, INFINITE);
        this
    }

    /// `0x006db7a0()` (`thiscall`): under the semaphore at `+0x2c` (waited
    /// for, then released once), `+0x20`, `+0x24`, `+0x28` zeroed; what the
    /// release returns.
    pub fn reset_counters<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32) -> u32 {
        let h: u32 = mem.read(this.wrapping_add(0x2c));
        k.wait(mem, h, INFINITE);
        for off in [0x28, 0x24, 0x20] {
            mem.write(this.wrapping_add(off), 0u32);
        }
        let h: u32 = mem.read(this.wrapping_add(0x2c));
        k.release_semaphore(mem, h, 1, 0)
    }

    /// `0x00703a90()` (`thiscall`): with a handle at `+4`, a wait without
    /// end on it, then it is closed and `+4`, `+8` zeroed; what the close
    /// returns, or 0 with no handle.
    pub fn join_close<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32) -> u32 {
        let h: u32 = mem.read(this.wrapping_add(4));
        if h == 0 {
            return 0;
        }
        k.wait(mem, h, INFINITE);
        let h: u32 = mem.read(this.wrapping_add(4));
        let r = k.close_handle(mem, h);
        mem.write(this.wrapping_add(4), 0u32);
        mem.write(this.wrapping_add(8), 0u32);
        r
    }

    /// `CloseHandle(*(this + at))` (`0x00634830` with `at` 0, `0x006db6e0`
    /// with 0x2c, `thiscall`); what it returns.
    pub fn close_member<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32, at: u32) -> u32 {
        let h: u32 = mem.read(this.wrapping_add(at));
        k.close_handle(mem, h)
    }

    /// `0x006f2bf0(&event)` / `0x006f2be0(&event)`: `SetEvent` / `ResetEvent`
    /// on `*event`; what it returns.
    pub fn event_set<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, event: u32, on: bool) -> u32 {
        let h: u32 = mem.read(event);
        if on {
            k.set_event(mem, h)
        } else {
            k.reset_event(mem, h)
        }
    }

    /// `0x006fb500()` (`thiscall`): the critical section at `this`
    /// initialized; returns `this`.
    pub fn cs_init<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32) -> u32 {
        k.initialize_critical_section(mem, this);
        this
    }

    /// `0x006fb510(cs)` (`thiscall`, `ret 4`): a critical-section guard:
    /// `*this` = `cs`, entered; returns `this`.
    pub fn cs_guard<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32, cs: u32) -> u32 {
        mem.write(this, cs);
        k.enter_critical_section(mem, cs);
        this
    }

    /// `0x006fb540()` (`thiscall`): the guard's critical section (`*this`)
    /// left; the import's `eax`.
    pub fn cs_guard_drop<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32) -> u32 {
        let cs: u32 = mem.read(this);
        k.leave_critical_section(mem, cs)
    }

    /// `0x006fb530()` (`thiscall`): the critical section at `this`
    /// deleted; the import's `eax`.
    pub fn cs_delete<M: Mem>(mem: &mut M, k: &mut impl Kernel<M>, this: u32) -> u32 {
        k.delete_critical_section(mem, this)
    }
}
