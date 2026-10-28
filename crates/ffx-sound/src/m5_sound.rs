//! M5 stage 1, list `sound`: the vsync emulation thread, the rest of the
//! `i*` queue entry points, and the FMOD objects' constructors, teardown
//! and shutdown (`FfxFmod`, `FmodMusic`, `FmodVoice`, `FmodSfx`).
//!
//! Layouts (from the originals' stores):
//! - the vsync thread (`0x20` bytes, `g_VsyncThread` `0x00ccc7c0`): `+0`
//!   vtable (`Sys_Thread` `0x00b44e2c`, `VsyncThreadImpl` `0x00b44e38`),
//!   `+4` the thread `HANDLE`, `+8` its id, `+0xc` byte "finished", `+0x10`
//!   the handler, `+0x18` "running", `+0x1c` the wake semaphore.
//! - `FmodSfx`: `+4`, `+0x34`, `+0x94` `PRedBlackTree`s (each with its
//!   `PFreeList` at `+0x18`), `+0x64` a tree whose nodes own a buffer at
//!   `+0x18`, `+0xc4` an `SfxBank*`, `+0xc8` an `SfxEvent*`, `+0xcc` the
//!   mutex. A node of the `+0x64` tree holds an event count at `+0x10`
//!   and the events at `+0x18`; a node of the `+0x94` tree an `SfxBank*`
//!   at `+0x10`.
//!
//! The Win32 and CRT calls go through the game's import slots; the
//! callees in the game's own code are called by address, so a ported one
//! runs as its port.

pub use crate::fmod::{Conv, FmodHost as Host};

/// The original functions, over game memory.
pub mod bridge {
    use ffx_bridge::{Global, x87};

    use super::{Conv, Host};
    use crate::fmod::bridge::{MANAGER, push0, push1, push2, push3};

    // ---- the callees ----
    /// `operator new` (cdecl).
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    /// `operator delete` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// `user_free_thunk` (cdecl).
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `user_malloc` (cdecl).
    pub const USER_MALLOC: u32 = 0x0068_7020;
    /// `crt_memset_thunk` (cdecl, `jmp [memset]`).
    pub const MEMSET: u32 = 0x0094_964c;
    /// `Phyre_PhyreFree` (cdecl).
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// The thread-start helper (thiscall `(thread; stack, suspended,
    /// priority, name)` ret 0x10): `CreateThread` and the `SetThreadName`
    /// exception, not portable (`symbols/m5-not-portable.tsv`).
    pub const THREAD_START: u32 = 0x0067_9cb0;
    /// `Phyre_PRedBlackTree_remove` (thiscall `(tree; node)` ret 4).
    pub const TREE_REMOVE: u32 = 0x0057_5000;
    /// `Phyre_PRedBlackTree_successor` (thiscall `(tree; node)` ret 4).
    pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
    /// `Phyre_PRedBlackTree_clear` (thiscall `(tree)`).
    pub const TREE_CLEAR: u32 = 0x0057_4760;
    /// `Phyre_PRedBlackTree_orphanAllContents` (thiscall `(tree)`).
    pub const TREE_ORPHAN: u32 = 0x0057_4970;
    /// `Phyre_PFreeList_reset` (thiscall `(list)`).
    pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
    /// `Phyre_PFreeList_dtor_PFreeList` (thiscall `(list)`).
    pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
    /// `FmodEventInfo_Init` (`ecx` = the info) -> the info.
    pub const EVENT_INFO_INIT: u32 = 0x0070_a140;
    /// `SetFields_0070e290` (`ecx`) -> `ecx`: zeroes a 16-byte block.
    pub const BLOCK_INIT: u32 = 0x0070_e290;
    /// `SetFields_0070e2b0` (`ecx`): zeroes it again.
    pub const BLOCK_CLEAR: u32 = 0x0070_e2b0;
    /// `SfxEvent_stop` (thiscall `(event; immediate)` ret 4).
    pub const SFX_EVENT_STOP: u32 = 0x0070_f490;
    /// `SfxEvent_dtor_0070e850` (thiscall `(event)`).
    pub const SFX_EVENT_DTOR: u32 = 0x0070_e850;
    /// `SfxBank_dtor_SfxBank` (thiscall `(bank)`).
    pub const SFX_BANK_DTOR: u32 = 0x0070_f930;
    /// `SfxBank_stopAllEvents` (thiscall `(bank; immediate)` ret 4).
    pub const SFX_BANK_STOP_ALL: u32 = 0x0070_fcd0;
    /// `FmodSfx_stopAllSound` (thiscall `(sfx; immediate)` ret 4).
    pub const SFX_STOP_ALL_SOUND: u32 = 0x0070_da90;
    /// `FmodSfx_unloadProjects` (thiscall `(sfx; a, b)` ret 8).
    pub const SFX_UNLOAD_PROJECTS: u32 = 0x0070_dd10;
    /// `FmodSfx_shutdown` (thiscall `(sfx)`).
    pub const SFX_SHUTDOWN: u32 = 0x0070_d780;
    /// The `+0x64` tree's teardown (thiscall `(tree)`, `0x0070c3c0`).
    pub const SFX_NODE_TREE_DTOR: u32 = 0x0070_c3c0;
    /// `FmodMusic`'s teardown (thiscall, `0x007086b0`).
    pub const MUSIC_DTOR: u32 = 0x0070_86b0;
    /// `FmodVoice`'s teardown (thiscall, `0x0070a330`).
    pub const VOICE_DTOR: u32 = 0x0070_a330;
    /// `FmodSfx`'s teardown (thiscall, `0x0070c4a0`).
    pub const SFX_DTOR: u32 = 0x0070_c4a0;

    // ---- imports (`KERNEL32`, stdcall; `MSVCR110`, cdecl) ----
    pub const CLOSE_HANDLE_IAT: u32 = 0x00b0_c0dc;
    pub const RELEASE_SEMAPHORE_IAT: u32 = 0x00b0_c0f4;
    pub const WAIT_IAT: u32 = 0x00b0_c0f8;
    pub const SET_THREAD_PRIORITY_IAT: u32 = 0x00b0_c10c;
    pub const RELEASE_MUTEX_IAT: u32 = 0x00b0_c1c0;
    pub const EXIT_THREAD_IAT: u32 = 0x00b0_c1d4;
    pub const TERMINATE_THREAD_IAT: u32 = 0x00b0_c1d8;
    pub const RESUME_THREAD_IAT: u32 = 0x00b0_c1dc;
    pub const CREATE_SEMAPHORE_IAT: u32 = 0x00b0_c1e0;
    /// `MSVCR110!sprintf` (cdecl).
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;

    // ---- data ----
    /// `g_VsyncThread`: the vsync emulation thread object.
    pub const VSYNC_THREAD: Global<u32> = Global::new(0x00cc_c7c0);
    /// `Sys_Thread`'s vtable.
    pub const SYS_THREAD_VTABLE: u32 = 0x00b4_4e2c;
    /// `VsyncThreadImpl`'s vtable.
    pub const VSYNC_THREAD_VTABLE: u32 = 0x00b4_4e38;
    /// `"VsyncEmulationThread"`, the thread's name format.
    pub const VSYNC_THREAD_NAME: u32 = 0x00b4_4e40;

    /// A stdcall import through its slot.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// A cdecl import (the CRT) through its slot.
    fn crt(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), args)
    }

    // ---- the vsync emulation thread ----

    /// `createVsyncEmuThread(handler)` (0x00679e60) -> `ResumeThread`'s
    /// `eax`: a `VsyncThreadImpl` (`0x20` bytes) with its wake semaphore,
    /// started suspended under the name `"VsyncEmulationThread"` (the
    /// original's frame buffer at `ebp - 0x68`, `frame` its `ebp`), then
    /// raised to priority 1 and resumed.
    pub fn create_vsync_emu_thread(h: &mut impl Host, frame: u32, handler: u32) -> u32 {
        let t = cdecl(h, OPERATOR_NEW, &[0x20]);
        if t != 0 {
            h.write(t + 4, 0u32);
            h.write(t + 8, 0u32);
            h.write(t + 0xc, 0u8);
            let vt = h.rebase(VSYNC_THREAD_VTABLE);
            h.write(t, vt);
            let sem = import(h, CREATE_SEMAPHORE_IAT, &[0, 1, 1, 0]);
            h.write(t + 0x1c, sem);
        }
        VSYNC_THREAD.set(h, t);
        h.write(t.wrapping_add(0x10), handler);
        let name = frame - 0x68;
        let fmt = h.rebase(VSYNC_THREAD_NAME);
        crt(h, SPRINTF_IAT, &[name, fmt]);
        let t = VSYNC_THREAD.get(h);
        h.write(t.wrapping_add(0x18), 1u32);
        let t = VSYNC_THREAD.get(h);
        this_call(h, THREAD_START, t, &[0x8000, 0, 0, name]);
        let t = VSYNC_THREAD.get(h);
        let thread: u32 = h.read(t.wrapping_add(4));
        import(h, SET_THREAD_PRIORITY_IAT, &[thread, 1]);
        let t = VSYNC_THREAD.get(h);
        let thread: u32 = h.read(t.wrapping_add(4));
        import(h, RESUME_THREAD_IAT, &[thread])
    }

    /// `endVsyncEmuThread()` (0x00679f20): stops the loop (`+0x18`), waits
    /// for the thread and closes it, deletes the object (vtable slot 0,
    /// flags 1) and clears `g_VsyncThread`. Returns the `eax` the
    /// original leaves: 0 without a thread, else the last call's.
    pub fn end_vsync_emu_thread(h: &mut impl Host) -> u32 {
        let t = VSYNC_THREAD.get(h);
        if t == 0 {
            return 0;
        }
        h.write(t + 0x18, 0u32);
        let mut t = VSYNC_THREAD.get(h);
        let thread: u32 = h.read(t.wrapping_add(4));
        let mut eax = thread;
        if thread != 0 {
            import(h, WAIT_IAT, &[thread, 0xffff_ffff]);
            let thread: u32 = h.read(t.wrapping_add(4));
            eax = import(h, CLOSE_HANDLE_IAT, &[thread]);
            h.write(t.wrapping_add(4), 0u32);
            h.write(t.wrapping_add(8), 0u32);
            t = VSYNC_THREAD.get(h);
        }
        if t != 0 {
            eax = h.call_virtual(t, 0, &[1]);
        }
        VSYNC_THREAD.set(h, 0);
        eax
    }

    /// `VsyncEmu_Wake()` (0x00679f80): `ReleaseSemaphore(+0x1c, 1, 0)`.
    pub fn vsync_emu_wake(h: &mut impl Host) -> u32 {
        let t = VSYNC_THREAD.get(h);
        let sem: u32 = h.read(t.wrapping_add(0x1c));
        import(h, RELEASE_SEMAPHORE_IAT, &[sem, 1, 0])
    }

    /// The thread's handle released: `TerminateThread(+4, 0)`,
    /// `CloseHandle(+4)`, `+4` and `+8` cleared (when `+4` is set).
    fn kill_thread(h: &mut impl Host, this: u32) {
        let thread: u32 = h.read(this + 4);
        if thread != 0 {
            import(h, TERMINATE_THREAD_IAT, &[thread, 0]);
            let thread: u32 = h.read(this + 4);
            import(h, CLOSE_HANDLE_IAT, &[thread]);
            h.write(this + 4, 0u32);
            h.write(this + 8, 0u32);
        }
    }

    /// `Sys_Thread`'s deleting destructor `(this; flags)` (0x00679bb0)
    /// -> `this`.
    pub fn sys_thread_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(SYS_THREAD_VTABLE);
        h.write(this, vt);
        kill_thread(h, this);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `VsyncThreadImpl`'s deleting destructor `(this; flags)`
    /// (0x00679c00) -> `this`: the semaphore closed first, then as
    /// [`sys_thread_deleting_dtor`].
    pub fn vsync_thread_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let sem: u32 = h.read(this + 0x1c);
        import(h, CLOSE_HANDLE_IAT, &[sem]);
        sys_thread_deleting_dtor(h, this, flags)
    }

    /// `VsyncThreadImpl_run(this)` (0x00679c60, vtable `+4`) -> 0: the
    /// handler (`+0x10`, no arguments) then a 16 ms wait on the
    /// semaphore, while `+0x18` is set; then the thread released.
    pub fn vsync_thread_run(h: &mut impl Host, this: u32) -> u32 {
        while h.read::<u32>(this + 0x18) != 0 {
            let handler: u32 = h.read(this + 0x10);
            h.call(handler, Conv::Cdecl, &[]);
            let sem: u32 = h.read(this + 0x1c);
            import(h, WAIT_IAT, &[sem, 0x10]);
        }
        kill_thread(h, this);
        0
    }

    /// `Thread_Start_00679db0(thread)` (0x00679db0, the threads' start
    /// address): the object's `run` (vtable slot 1), `+0xc` = 1, then
    /// `ExitThread(run's result)`, which does not return in the game.
    /// Returns `ExitThread`'s `eax` for hosts where it does.
    pub fn thread_start(h: &mut impl Host, thread: u32) -> u32 {
        let code = h.call_virtual(thread, 1, &[]);
        h.write(thread + 0xc, 1u8);
        import(h, EXIT_THREAD_IAT, &[code])
    }

    /// `code_0044cd10(p)` (0x0044cd10): `jmp Phyre_PhyreFree`.
    pub fn phyre_free_thunk(h: &mut impl Host, p: u32) -> u32 {
        cdecl(h, PHYRE_FREE, &[p])
    }

    // ---- the `i*` entry points (FMOD commands, `docs/formats/sound.md`) ----

    /// `iSfxStopNonLoopSound()` (0x0067a4c0): command `0x39`.
    pub fn i_sfx_stop_non_loop_sound(h: &mut impl Host) -> u32 {
        let m = MANAGER.get(h);
        push0(h, m, 0x39)
    }

    /// `iShoutStart(a, b, volume)` (0x0067a4e0) -> 0: command 6.
    pub fn i_shout_start(h: &mut impl Host, a: u32, b: u32, volume: u32) -> u32 {
        let m = MANAGER.get(h);
        push3(h, m, 6, a, b, volume);
        0
    }

    /// The one-argument `i*` setters: `iSoundSetMasterVolume` (0x42),
    /// `iSoundSetMusicVolume` (0x43), `iSoundSetVoiceVolume` (0x44),
    /// `iSoundSetSfxVolume` (0x45), `iVoicePanDirect` (0x0c).
    pub fn i_set1(h: &mut impl Host, cmd: u32, a: u32) -> u32 {
        let m = MANAGER.get(h);
        push1(h, m, cmd, a)
    }

    /// The two-argument ones: `iSoundSetMute` (0x00), `iVoicePanTranslate`
    /// (0x0b).
    pub fn i_set2(h: &mut impl Host, cmd: u32, a: u32, b: u32) -> u32 {
        let m = MANAGER.get(h);
        push2(h, m, cmd, a, b)
    }

    // ---- the FMOD objects ----

    /// `FmodVoice`'s constructor (0x0070a1a0, `ecx`) -> `this`: the
    /// `0x90` bytes zeroed but `+8` = -1 and `+0x88` = 1.0, an
    /// `FMOD_EVENT_INFO` at `+0x68` (`0x34` bytes), two zeroed `0x54`-byte
    /// blocks at `+0x18` and `+0xc`, a 16-byte block at `+0x84`.
    pub fn voice_ctor(h: &mut impl Host, this: u32) -> u32 {
        for off in [0x34, 0x38, 0x3c, 0, 0x50] {
            h.write(this + off, 0u32);
        }
        h.write(this + 4, 0u8);
        h.write(this + 8, 0xffff_ffffu32);
        h.write(this + 0x10, 0u32);
        h.write(this + 0x14, 0u32);
        h.write(this + 0x88, 1.0f32.to_bits());
        for off in [0x1c, 0x20, 0x24, 0x28] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x2c, 0u8);
        h.write(this + 0x30, 0u32);
        for off in (0x40..=0x4c).step_by(4).chain((0x54..=0x64).step_by(4)) {
            h.write(this + off, 0u32);
        }
        for off in (0x6c..=0x80).step_by(4) {
            h.write(this + off, 0u32);
        }
        let info = cdecl(h, OPERATOR_NEW, &[0x34]);
        let info = if info == 0 {
            0
        } else {
            this_call(h, EVENT_INFO_INIT, info, &[])
        };
        h.write(this + 0x68, info);
        for off in [0x18, 0xc] {
            let p = cdecl(h, USER_MALLOC, &[0x54]);
            h.write(this + off, p);
            cdecl(h, MEMSET, &[p, 0, 0x54]);
        }
        let block = cdecl(h, OPERATOR_NEW, &[0x10]);
        let block = if block == 0 {
            0
        } else {
            this_call(h, BLOCK_INIT, block, &[])
        };
        h.write(this + 0x84, block);
        this
    }

    /// `HandleHolder_close_006f9f60(this)` (0x006f9f60):
    /// `CloseHandle(*this)`.
    pub fn handle_close(h: &mut impl Host, this: u32) -> u32 {
        let handle: u32 = h.read(this);
        import(h, CLOSE_HANDLE_IAT, &[handle])
    }

    /// `user_free(this + off)` and the word cleared, when it is set.
    fn free_field(h: &mut impl Host, this: u32, off: u32) {
        let p: u32 = h.read(this + off);
        if p != 0 {
            cdecl(h, USER_FREE, &[p]);
            h.write(this + off, 0u32);
        }
    }

    /// `FmodMusic`'s teardown (0x007086b0, `ecx`): the buffers at `+0x14`
    /// and `+0x10` freed, the header words cleared.
    pub fn music_dtor(h: &mut impl Host, this: u32) {
        free_field(h, this, 0x14);
        free_field(h, this, 0x10);
        for off in [4, 0xc, 0, 8, 0x18, 0x1c, 0x20, 0x24] {
            h.write(this + off, 0u32);
        }
    }

    /// `FmodVoice`'s teardown (0x0070a330, `ecx`): the buffers at `+0x24`,
    /// `+0x28`, `+0x18`, `+0xc` freed, the event info deleted, the fields
    /// cleared, the `+0x84` block cleared and deleted.
    pub fn voice_dtor(h: &mut impl Host, this: u32) {
        for off in [0x24, 0x28, 0x18, 0xc] {
            free_field(h, this, off);
        }
        let info: u32 = h.read(this + 0x68);
        cdecl(h, OPERATOR_DELETE, &[info]);
        let block: u32 = h.read(this + 0x84);
        for off in [0x34, 0x38, 0x3c, 0x68, 0x50, 0x1c, 0x10, 0x14, 0x20] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x2c, 0u8);
        h.write(this + 0x30, 0u32);
        for off in (0x40..=0x4c).step_by(4).chain((0x54..=0x64).step_by(4)) {
            h.write(this + off, 0u32);
        }
        for off in (0x6c..=0x80).step_by(4) {
            h.write(this + off, 0u32);
        }
        if block != 0 {
            this_call(h, BLOCK_CLEAR, block, &[]);
            cdecl(h, OPERATOR_DELETE, &[block]);
        }
        h.write(this + 0x84, 0u32);
    }

    /// Every node of `tree` removed (from `+0xc` until it is the tree or
    /// 0), each node's `+0x18` buffer freed.
    fn drain_nodes(h: &mut impl Host, tree: u32) {
        loop {
            let node: u32 = h.read(tree + 0xc);
            if node == tree || node == 0 {
                break;
            }
            this_call(h, TREE_REMOVE, tree, &[node]);
            let buf: u32 = h.read(node + 0x18);
            if buf != 0 {
                cdecl(h, USER_FREE, &[buf]);
            }
        }
    }

    /// The `+0x64` tree's teardown (0x0070c3c0, `ecx` = the tree): the
    /// nodes drained, the free list reset and destroyed, the tree cleared
    /// twice (its own destructor and its base's).
    pub fn sfx_node_tree_dtor(h: &mut impl Host, tree: u32) {
        drain_nodes(h, tree);
        this_call(h, FREE_LIST_RESET, tree + 0x18, &[]);
        this_call(h, FREE_LIST_DTOR, tree + 0x18, &[]);
        this_call(h, TREE_CLEAR, tree, &[]);
        this_call(h, TREE_CLEAR, tree, &[]);
    }

    /// The tree's first node as the originals find it: from `+0xc` along
    /// the `+0` links until the next is the tree (`+0xc` itself when it is
    /// the tree or 0).
    fn leftmost(h: &mut impl Host, tree: u32) -> u32 {
        let mut node: u32 = h.read(tree + 0xc);
        if node == tree || node == 0 {
            return node;
        }
        loop {
            let next: u32 = h.read(node);
            if next == tree {
                return node;
            }
            node = next;
        }
    }

    /// `FmodSfx_stopAllSound(this; immediate)` (0x0070da90) ->
    /// `ReleaseMutex`'s `eax`: under the mutex (`+0xcc`), every event of
    /// every node of the `+0x64` tree stopped, destroyed and deleted; the
    /// nodes drained and the free list reset.
    pub fn sfx_stop_all_sound(h: &mut impl Host, this: u32, immediate: u32) -> u32 {
        let mutex: u32 = h.read(this + 0xcc);
        import(h, WAIT_IAT, &[mutex, 0xffff_ffff]);
        let tree = this + 0x64;
        let mut node = leftmost(h, tree);
        if node != tree {
            let mut next = this_call(h, TREE_SUCCESSOR, tree, &[node]);
            loop {
                let mut i = 0u32;
                while i < h.read::<u32>(node + 0x10) {
                    let events: u32 = h.read(node + 0x18);
                    let event: u32 = h.read(events + 4 * i);
                    if event != 0 {
                        this_call(h, SFX_EVENT_STOP, event, &[immediate]);
                        let events: u32 = h.read(node + 0x18);
                        let event: u32 = h.read(events + 4 * i);
                        if event != 0 {
                            this_call(h, SFX_EVENT_DTOR, event, &[]);
                            cdecl(h, OPERATOR_DELETE, &[event]);
                        }
                    }
                    i += 1;
                }
                node = next;
                if next != tree {
                    next = this_call(h, TREE_SUCCESSOR, tree, &[next]);
                }
                if node == tree {
                    break;
                }
            }
        }
        drain_nodes(h, tree);
        this_call(h, FREE_LIST_RESET, tree + 0x18, &[]);
        let mutex: u32 = h.read(this + 0xcc);
        import(h, RELEASE_MUTEX_IAT, &[mutex])
    }

    /// `SfxBank_stopAllEvents(this; immediate)` (0x0070fcd0): the
    /// project's (`*this`) `stopAllEvents(immediate)`, vtable `+0x24`
    /// (FMOD, stdcall with `this` on the stack), when there is one.
    /// Returns the `eax` the original leaves (the project pointer when
    /// there is none).
    pub fn sfx_bank_stop_all_events(h: &mut impl Host, this: u32, immediate: u32) -> u32 {
        let project: u32 = h.read(this);
        if project == 0 {
            return 0;
        }
        let vt: u32 = h.read(project);
        let f: u32 = h.read(vt + 0x24);
        h.call(f, Conv::Stdcall, &[project, immediate])
    }

    /// `FmodSfx_shutdown(this)` (0x0070d780): every sound stopped
    /// (`stopAllSound(1)`), the projects unloaded (`unloadProjects(1, 1)`),
    /// every bank of the `+0x94` tree destroyed and deleted, the tree
    /// orphaned and its free list reset; then the `+0xc8` event stopped,
    /// destroyed, deleted and cleared, the `+0xc4` bank's events stopped
    /// and the bank destroyed, deleted and cleared.
    pub fn sfx_shutdown(h: &mut impl Host, this: u32) {
        this_call(h, SFX_STOP_ALL_SOUND, this, &[1]);
        this_call(h, SFX_UNLOAD_PROJECTS, this, &[1, 1]);
        let tree = this + 0x94;
        let mut node = leftmost(h, tree);
        if node != tree {
            let mut next = this_call(h, TREE_SUCCESSOR, tree, &[node]);
            loop {
                let bank: u32 = h.read(node + 0x10);
                if bank != 0 {
                    this_call(h, SFX_BANK_DTOR, bank, &[]);
                    cdecl(h, OPERATOR_DELETE, &[bank]);
                }
                node = next;
                if next != tree {
                    next = this_call(h, TREE_SUCCESSOR, tree, &[next]);
                }
                if node == tree {
                    break;
                }
            }
        }
        this_call(h, TREE_ORPHAN, tree, &[]);
        this_call(h, FREE_LIST_RESET, tree + 0x18, &[]);
        let event: u32 = h.read(this + 0xc8);
        if event != 0 {
            this_call(h, SFX_EVENT_STOP, event, &[1]);
            let event: u32 = h.read(this + 0xc8);
            if event != 0 {
                this_call(h, SFX_EVENT_DTOR, event, &[]);
                cdecl(h, OPERATOR_DELETE, &[event]);
            }
            h.write(this + 0xc8, 0u32);
        }
        let bank: u32 = h.read(this + 0xc4);
        if bank != 0 {
            this_call(h, SFX_BANK_STOP_ALL, bank, &[1]);
            let bank: u32 = h.read(this + 0xc4);
            if bank != 0 {
                this_call(h, SFX_BANK_DTOR, bank, &[]);
                cdecl(h, OPERATOR_DELETE, &[bank]);
            }
            h.write(this + 0xc4, 0u32);
        }
    }

    /// A tree member's destruction: orphaned, its free list reset and
    /// destroyed, cleared twice.
    fn tree_member_dtor(h: &mut impl Host, tree: u32) {
        this_call(h, TREE_ORPHAN, tree, &[]);
        this_call(h, FREE_LIST_RESET, tree + 0x18, &[]);
        this_call(h, FREE_LIST_DTOR, tree + 0x18, &[]);
        this_call(h, TREE_CLEAR, tree, &[]);
        this_call(h, TREE_CLEAR, tree, &[]);
    }

    /// `FmodSfx`'s teardown (0x0070c4a0, `ecx`): `shutdown`, the mutex
    /// closed, then the members destroyed in reverse order (`+0x94`, the
    /// `+0x64` node tree, `+0x34`, `+4`).
    pub fn sfx_dtor(h: &mut impl Host, this: u32) {
        this_call(h, SFX_SHUTDOWN, this, &[]);
        let mutex: u32 = h.read(this + 0xcc);
        import(h, CLOSE_HANDLE_IAT, &[mutex]);
        tree_member_dtor(h, this + 0x94);
        this_call(h, SFX_NODE_TREE_DTOR, this + 0x64, &[]);
        tree_member_dtor(h, this + 0x34);
        tree_member_dtor(h, this + 4);
    }

    /// `FfxFmod`'s teardown (0x00706590, `ecx`): `+0`..`+8` cleared, then
    /// the music (`+0xc`), voice (`+0x10`) and sfx (`+0x14`) objects
    /// destroyed, deleted and cleared.
    pub fn ffx_fmod_dtor(h: &mut impl Host, this: u32) {
        let music: u32 = h.read(this + 0xc);
        for off in [0, 4, 8] {
            h.write(this + off, 0u32);
        }
        delete_member(h, music, MUSIC_DTOR);
        let voice: u32 = h.read(this + 0x10);
        h.write(this + 0xc, 0u32);
        delete_member(h, voice, VOICE_DTOR);
        let sfx: u32 = h.read(this + 0x14);
        h.write(this + 0x10, 0u32);
        delete_member(h, sfx, SFX_DTOR);
        h.write(this + 0x14, 0u32);
    }

    /// `obj`'s teardown (`dtor`, `ecx`) and `operator delete`, when set.
    fn delete_member(h: &mut impl Host, obj: u32, dtor: u32) {
        if obj != 0 {
            this_call(h, dtor, obj, &[]);
            cdecl(h, OPERATOR_DELETE, &[obj]);
        }
    }

    // ---- batch 2: the FmodManager, the FMOD objects' shutdown ----

    /// `FmodManager`'s vtable.
    pub const FMOD_MANAGER_VTABLE: u32 = 0x00b4_c2e4;
    /// `"FmodManager"`, the thread's name.
    pub const FMOD_THREAD_NAME: u32 = 0x00b4_c2e8;
    /// `FmodManager_threadMain`, the thread's body.
    pub const FMOD_THREAD_MAIN: u32 = 0x006f_a1b0;
    /// `1000.0` (f64): microseconds to milliseconds.
    pub const MSEC_DIVISOR: u32 = 0x00b0_dba0;
    /// The FMOD thread's tick (f64, milliseconds).
    pub const TICK_PERIOD: u32 = 0x00b4_c368;
    /// `127.0` (f64): the game's volume scale.
    pub const VOLUME_SCALE: u32 = 0x00b9_22e8;
    /// `graphicGetTimeInMicroseconds()` -> `edx:eax`.
    pub const GET_TIME_US: u32 = 0x0064_1420;
    /// `FfxFmod`'s constructor (`ecx`, `0x00706490`).
    pub const FFX_FMOD_CTOR: u32 = 0x0070_6490;
    /// `FfxFmod`'s teardown (`ecx`).
    pub const FFX_FMOD_DTOR: u32 = 0x0070_6590;
    /// `FfxFmod_soundLoop` (`ecx`).
    pub const FFX_FMOD_SOUND_LOOP: u32 = 0x0070_70d0;
    /// The FMOD thread's queue drain (`ecx` = the manager, `0x006fa7e0`).
    pub const MANAGER_DRAIN: u32 = 0x006f_a7e0;
    /// `FmodManager`'s constructor (`ecx`, `0x006f9de0`).
    pub const FMOD_MANAGER_CTOR: u32 = 0x006f_9de0;
    /// `FmodManager`'s teardown (`ecx`, `0x006f9f70`).
    pub const FMOD_MANAGER_DTOR: u32 = 0x006f_9f70;
    /// `CreateThread_ps2(params, name)` (cdecl) -> the thread.
    pub const CREATE_THREAD_PS2: u32 = 0x006f_2e10;
    /// `StartThread_ps2(thread, arg)` (cdecl).
    pub const START_THREAD_PS2: u32 = 0x006f_2d30;
    /// `0x006f3050(thread, priority)` (cdecl): `SetThreadPriority`.
    pub const SET_PRIORITY_PS2: u32 = 0x006f_3050;
    /// `SleepThread()` (cdecl).
    pub const SLEEP_THREAD: u32 = 0x006f_2d10;
    /// `SleepThreadInMillisecond(ms)` (cdecl).
    pub const SLEEP_MS: u32 = 0x006f_2d20;
    /// `ERRCHECK(result, file, line)` (cdecl).
    pub const ERRCHECK: u32 = 0x0070_79d0;
    /// `FmodMusic_shutdown` (`ecx`).
    pub const MUSIC_SHUTDOWN: u32 = 0x0070_9640;
    /// `FmodVoice_shutdown` (`ecx`).
    pub const VOICE_SHUTDOWN: u32 = 0x0070_b480;
    /// The voice's stream release (`ecx`, `0x0070e450`).
    pub const VOICE_STREAM_RELEASE: u32 = 0x0070_e450;
    /// `FmodMusic_volumeDirect` (thiscall `(music; id, volume, frames)`
    /// ret 0xc).
    pub const MUSIC_VOLUME_DIRECT: u32 = 0x0070_9dd0;
    /// `user_mem_size(p)` (cdecl).
    pub const USER_MEM_SIZE: u32 = 0x0068_7050;
    // FMOD's exports (stdcall, `this` first), through their thunks
    /// `FMOD::Sound::release`.
    pub const SOUND_RELEASE: u32 = 0x0094_8cfa;
    /// `FMOD::EventSystem::unload`.
    pub const EVENT_SYSTEM_UNLOAD: u32 = 0x0094_8c8e;
    /// `FMOD::EventSystem::release`.
    pub const EVENT_SYSTEM_RELEASE: u32 = 0x0094_8c7c;
    /// `FMOD_Memory_GetStats(&current, &max, blocking)`.
    pub const MEMORY_GET_STATS: u32 = 0x0094_8cdc;
    /// `FMOD::Event::setPaused(event, paused)`.
    pub const EVENT_SET_PAUSED: u32 = 0x0094_8cbe;
    /// `FMOD::Event::setVolume(event, volume)`.
    pub const EVENT_SET_VOLUME: u32 = 0x0094_8cb2;
    /// The source-file names `ERRCHECK` reports.
    pub const MUSIC_FILE: u32 = 0x00b4_f4a0;
    pub const VOICE_FILE: u32 = 0x00b4_f8c0;
    pub const SFX_EVENT_FILE: u32 = 0x00b4_fd34;
    pub const FFX_FMOD_FILE: u32 = 0x00b4_d268;
    /// `FMOD_ErrorString`'s jump table (96 entries) and its default case.
    pub const ERROR_STRING_TABLE: u32 = 0x0070_7850;
    pub const ERROR_STRING_DEFAULT: u32 = 0x0070_7846;
    // imports
    pub const CREATE_EVENT_IAT: u32 = 0x00b0_c078;
    pub const WAIT_EX_IAT: u32 = 0x00b0_c080;
    pub const RESET_EVENT_IAT: u32 = 0x00b0_c0f0;
    pub const CREATE_MUTEX_IAT: u32 = 0x00b0_c198;
    /// `MSVCR110!memcpy_s` (cdecl).
    pub const MEMCPY_S_IAT: u32 = 0x00b0_c434;

    fn f64_at(h: &mut impl Host, sym: u32) -> f64 {
        let at = h.rebase(sym);
        f64::from_bits(h.read::<u64>(at))
    }

    /// `graphicGetTimeInMicroseconds() / 1000.0`: the `u64` as a double
    /// (the originals' two `fild qword`, which round once as the `u64`
    /// conversion does), divided at 53 bits.
    fn now_msec(h: &mut impl Host) -> f64 {
        let us = h.call_sym_u64(GET_TIME_US, Conv::Cdecl, &[]);
        #[allow(clippy::cast_precision_loss)] // the original's rounding
        let us = us as f64;
        let k = f64_at(h, MSEC_DIVISOR);
        x87::div(us, k)
    }

    fn errcheck(h: &mut impl Host, result: u32, file: u32, line: u32) -> u32 {
        let file = h.rebase(file);
        cdecl(h, ERRCHECK, &[result, file, line])
    }

    /// An FMOD object's method at vtable byte `off` (stdcall, `this` on
    /// the stack, no other words), then `ERRCHECK`.
    fn fmod_method(h: &mut impl Host, obj: u32, off: u32, file: u32, line: u32) {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt + off);
        let r = h.call(f, Conv::Stdcall, &[obj]);
        errcheck(h, r, file, line);
    }

    fn sleep_ms(h: &mut impl Host, ms: u32) {
        cdecl(h, SLEEP_MS, &[ms]);
    }

    /// `FmodManager`'s constructor (0x006f9de0, `ecx`) -> `this`: two
    /// mutexes (`+0x10`, `+0x14`), two manual-reset events (`+0x18`,
    /// `+0x1c`), the start time in ms (f64 `+0x20`), `+0x44` byte 1,
    /// `+0x4c` u16 1, `g_FmodManager`, the `FfxFmod` (`+8`), commands
    /// `0x2a` and `0x2b` queued, and the FMOD thread (`+0xc`, priority
    /// 300) started from a parameter block in the original's frame
    /// (`ebp - 0x48`: `+4` the body, `+0xc` the stack size `0x10000`,
    /// `+0x20` 0; `frame` the original's `ebp`).
    pub fn fmod_manager_ctor(h: &mut impl Host, frame: u32, this: u32) -> u32 {
        let vt = h.rebase(FMOD_MANAGER_VTABLE);
        h.write(this, vt);
        h.write(this + 8, 0u32);
        let m = import(h, CREATE_MUTEX_IAT, &[0, 0, 0]);
        h.write(this + 0x10, m);
        let m = import(h, CREATE_MUTEX_IAT, &[0, 0, 0]);
        h.write(this + 0x14, m);
        let e = import(h, CREATE_EVENT_IAT, &[0, 1, 0, 0]);
        h.write(this + 0x18, e);
        let e = import(h, CREATE_EVENT_IAT, &[0, 1, 0, 0]);
        h.write(this + 0x1c, e);
        let start = now_msec(h);
        h.write(this + 0x28, 0u32);
        h.write(this + 0x2c, 0u32);
        h.write(this + 0x44, 1u8);
        h.write(this + 0x48, 0u32);
        h.write(this + 0x4c, 1u16);
        h.write(this + 0x20, start.to_bits());
        crate::fmod::bridge::MANAGER.set(h, this);
        let p = cdecl(h, OPERATOR_NEW, &[0x28]);
        let ffx = if p == 0 {
            0
        } else {
            this_call(h, FFX_FMOD_CTOR, p, &[])
        };
        h.write(this + 8, ffx);
        crate::fmod::bridge::push_command(h, this, 0x2a, [0; 4]);
        crate::fmod::bridge::push_command(h, this, 0x2b, [0; 4]);
        let params = frame - 0x48;
        let body = h.rebase(FMOD_THREAD_MAIN);
        h.write(params + 4, body);
        h.write(params + 0xc, 0x1_0000u32);
        h.write(params + 0x20, 0u32);
        let name = h.rebase(FMOD_THREAD_NAME);
        let thread = cdecl(h, CREATE_THREAD_PS2, &[params, name]);
        h.write(this + 0xc, thread);
        if thread != 0 {
            cdecl(h, START_THREAD_PS2, &[thread, 0]);
            let thread: u32 = h.read(this + 0xc);
            cdecl(h, SET_PRIORITY_PS2, &[thread, 300]);
        }
        this
    }

    /// `FmodManager`'s teardown (0x006f9f70, `ecx`) -> the last
    /// `CloseHandle`'s `eax`: `+0x4c` cleared (the thread's "run"), then
    /// `SleepThread` until the thread sets `+0x4d`; the `FfxFmod` torn
    /// down and deleted, the events and mutexes closed.
    pub fn fmod_manager_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(FMOD_MANAGER_VTABLE);
        h.write(this, vt);
        h.write(this + 0x4c, 0u8);
        while h.read::<u8>(this + 0x4d) == 0 {
            cdecl(h, SLEEP_THREAD, &[]);
        }
        let ffx: u32 = h.read(this + 8);
        delete_member(h, ffx, FFX_FMOD_DTOR);
        h.write(this + 8, 0u32);
        let mut r = 0;
        for off in [0x1c, 0x18, 0x14, 0x10] {
            let handle: u32 = h.read(this + off);
            r = import(h, CLOSE_HANDLE_IAT, &[handle]);
        }
        r
    }

    /// `FmodManager`'s deleting destructor `(this; flags)` (0x006fa040)
    /// -> `this`.
    pub fn fmod_manager_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        this_call(h, FMOD_MANAGER_DTOR, this, &[]);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `FmodManager_create_006fa0b0()` (0x006fa0b0, `iFmodInitialize`'s
    /// callee): a new manager (`0x50` bytes) into `g_FmodManager` unless
    /// there is one. (The original's `eax` when there is one is its
    /// cookie; no caller reads it.)
    pub fn fmod_manager_create(h: &mut impl Host) {
        if crate::fmod::bridge::MANAGER.get(h) != 0 {
            return;
        }
        let p = cdecl(h, OPERATOR_NEW, &[0x50]);
        let m = if p == 0 {
            0
        } else {
            this_call(h, FMOD_MANAGER_CTOR, p, &[])
        };
        crate::fmod::bridge::MANAGER.set(h, m);
    }

    /// `FmodManager_tick(this)` (0x006fa700) -> the queue drain's `eax`:
    /// once a tick period (f64 `0x00b4c368`) has passed since `+0x20`,
    /// `FfxFmod_soundLoop` and `+0x20` advanced by whole periods;
    /// otherwise a wait on the `+0x18` event for the rest of the period
    /// (at least 1 ms, `WaitForSingleObjectEx(.., TRUE)` again while it
    /// fails), then `ResetEvent`. The queue is drained either way.
    pub fn fmod_manager_tick(h: &mut impl Host, this: u32) -> u32 {
        let start = f64::from_bits(h.read::<u64>(this + 0x20));
        let now = x87::sub(now_msec(h), start);
        let period = f64_at(h, TICK_PERIOD);
        if period > now || period.is_nan() || now.is_nan() {
            let wait = x87::ftol(h, x87::sub(period, now)).cast_signed();
            let wait = if wait > 0 { wait.cast_unsigned() } else { 1 };
            loop {
                let event: u32 = h.read(this + 0x18);
                if import(h, WAIT_EX_IAT, &[event, wait, 1]) != 0xffff_ffff {
                    break;
                }
            }
            let event: u32 = h.read(this + 0x18);
            import(h, RESET_EVENT_IAT, &[event]);
        } else {
            let ffx: u32 = h.read(this + 8);
            this_call(h, FFX_FMOD_SOUND_LOOP, ffx, &[]);
            let n = x87::ftol(h, x87::div(now, period)).cast_signed();
            let start = f64::from_bits(h.read::<u64>(this + 0x20));
            let next = x87::add(x87::mul(f64::from(n), period), start);
            h.write(this + 0x20, next.to_bits());
        }
        this_call(h, MANAGER_DRAIN, this, &[])
    }

    /// `FmodManager_Destroy()` (0x006fa190): the manager's deleting
    /// destructor (vtable slot 0, flags 1), `SleepThread`, the global
    /// cleared.
    pub fn fmod_manager_destroy(h: &mut impl Host) {
        let m = crate::fmod::bridge::MANAGER.get(h);
        if m != 0 {
            h.call_virtual(m, 0, &[1]);
            cdecl(h, SLEEP_THREAD, &[]);
            crate::fmod::bridge::MANAGER.set(h, 0);
        }
    }

    /// `FmodManager_getSystemTimeInMsec()` (0x006fa210): the clock in ms
    /// (f64 in `ST0`).
    pub fn get_system_time_in_msec(h: &mut impl Host) -> f64 {
        now_msec(h)
    }

    /// `FmodMusic_shutdown(this)` (0x00709640): the two event groups
    /// (`+8`, `+0xc`) each stopped (vtable `+0x28`), 100 ms later
    /// released (`+0`) and cleared, every result through `ERRCHECK`.
    pub fn music_shutdown(h: &mut impl Host, this: u32) {
        for (off, lines) in [(8, [0xbb, 0xc2]), (0xc, [0xc8, 0xcf])] {
            let g: u32 = h.read(this + off);
            if g != 0 {
                fmod_method(h, g, 0x28, MUSIC_FILE, lines[0]);
            }
            sleep_ms(h, 100);
            let g: u32 = h.read(this + off);
            if g != 0 {
                fmod_method(h, g, 0, MUSIC_FILE, lines[1]);
                h.write(this + off, 0u32);
            }
        }
    }

    /// The voice's stream release (0x0070e450, `ecx`) -> 0:
    /// `FMOD::Sound::release(*this)` through `ERRCHECK`, when set.
    pub fn voice_stream_release(h: &mut impl Host, this: u32) -> u32 {
        let sound: u32 = h.read(this);
        if sound != 0 {
            let r = h.call_sym(SOUND_RELEASE, Conv::Stdcall, &[sound]);
            errcheck(h, r, SFX_EVENT_FILE, 0x85);
        }
        0
    }

    /// `FmodVoice_shutdown(this)` (0x0070b480): the stream released
    /// (`+0x84`), the 21 voice events (`+0x18`) stopped, then released and
    /// cleared, the `+0x20` event likewise, 100 ms between the steps.
    pub fn voice_shutdown(h: &mut impl Host, this: u32) {
        let stream: u32 = h.read(this + 0x84);
        this_call(h, VOICE_STREAM_RELEASE, stream, &[]);
        sleep_ms(h, 100);
        for i in 0..21 {
            let arr: u32 = h.read(this + 0x18);
            let e: u32 = h.read(arr + 4 * i);
            if e != 0 {
                fmod_method(h, e, 0x28, VOICE_FILE, 0x1b6);
            }
        }
        sleep_ms(h, 100);
        for i in 0..21 {
            let arr: u32 = h.read(this + 0x18);
            let e: u32 = h.read(arr + 4 * i);
            if e != 0 {
                fmod_method(h, e, 0, VOICE_FILE, 0x1be);
                let arr: u32 = h.read(this + 0x18);
                h.write(arr + 4 * i, 0u32);
            }
        }
        sleep_ms(h, 100);
        let e: u32 = h.read(this + 0x20);
        if e != 0 {
            fmod_method(h, e, 0x28, VOICE_FILE, 0x1c6);
        }
        sleep_ms(h, 100);
        let e: u32 = h.read(this + 0x20);
        if e != 0 {
            fmod_method(h, e, 0, VOICE_FILE, 0x1cc);
            h.write(this + 0x20, 0u32);
        }
    }

    /// `FfxFmod_shutDown(this)` (0x00706ff0) -> `user_free`'s `eax` (0
    /// without the `+0` buffer): the voice, music and sfx objects shut
    /// down, the event system unloaded and released (through
    /// `ERRCHECK`), 100 ms after each step; the `+0` buffer freed.
    pub fn ffx_fmod_shutdown(h: &mut impl Host, this: u32) -> u32 {
        for (off, f) in [
            (0x10, VOICE_SHUTDOWN),
            (0xc, MUSIC_SHUTDOWN),
            (0x14, SFX_SHUTDOWN),
        ] {
            let obj: u32 = h.read(this + off);
            this_call(h, f, obj, &[]);
            sleep_ms(h, 100);
        }
        for (f, line) in [(EVENT_SYSTEM_UNLOAD, 0xc9), (EVENT_SYSTEM_RELEASE, 0xcd)] {
            let es: u32 = h.read(this + 4);
            let r = h.call_sym(f, Conv::Stdcall, &[es]);
            errcheck(h, r, FFX_FMOD_FILE, line);
            sleep_ms(h, 100);
        }
        let p: u32 = h.read(this);
        if p == 0 { 0 } else { cdecl(h, USER_FREE, &[p]) }
    }

    /// `FmodMemory_getCurrent_007067f0()` (0x007067f0): FMOD's current
    /// memory use (`FMOD_Memory_GetStats(&cur, &max, 0)` with both in the
    /// original's frame, `frame` its `ebp`), 0 when the call fails.
    pub fn fmod_memory_current(h: &mut impl Host, frame: u32) -> u32 {
        let (cur, max) = (frame - 4, frame - 8);
        h.write(cur, 0u32);
        h.write(max, 0u32);
        let r = h.call_sym(MEMORY_GET_STATS, Conv::Stdcall, &[cur, max, 0]);
        if r == 0 { h.read(cur) } else { 0 }
    }

    /// `FmodMemory_realloc(ptr, size, type, source)` (0x00706730, FMOD's
    /// realloc callback, stdcall) -> the new block: 16-aligned inside a
    /// `user_malloc(size + 16)`, its offset in the byte before it; the old
    /// block's bytes copied (`memcpy_s`, at most its usable size) and the
    /// old allocation freed.
    pub fn fmod_memory_realloc(h: &mut impl Host, ptr: u32, size: u32) -> u32 {
        let mut block = 0;
        if size != 0 {
            let raw = cdecl(h, USER_MALLOC, &[size.wrapping_add(0x10)]);
            block = raw.wrapping_sub(raw & 0xf).wrapping_add(0x10);
            #[allow(clippy::cast_possible_truncation)] // the low bytes
            let off = (block as u8).wrapping_sub(raw as u8);
            h.write(block.wrapping_sub(1), off);
        }
        if ptr != 0 {
            let off = u32::from(h.read::<u8>(ptr - 1));
            let base = ptr.wrapping_sub(off);
            if block != 0 {
                let usable = cdecl(h, USER_MEM_SIZE, &[base]).wrapping_sub(off);
                let n = if usable > size { size } else { usable };
                let memcpy_s: u32 = h.read(h.rebase(MEMCPY_S_IAT));
                h.call(memcpy_s, Conv::Cdecl, &[block, size, ptr, n]);
            }
            cdecl(h, USER_FREE, &[base]);
        }
        block
    }

    /// `FMOD_ErrorString(result)` (0x00707590): the message for an FMOD
    /// result, as the original's jump table picks it (each case a `mov
    /// eax, string`; read from the image), "Unknown error" past 0x5f.
    pub fn fmod_error_string(h: &mut impl Host, result: u32) -> u32 {
        let case = if result > 0x5f {
            h.rebase(ERROR_STRING_DEFAULT)
        } else {
            let table = h.rebase(ERROR_STRING_TABLE);
            h.read(table + 4 * result)
        };
        h.read(case + 1)
    }

    /// `FmodMusic_pauseAll(this; paused)` (0x00709000): every one of the
    /// 182 music slots (`+0x10`, `0x3c` bytes each) with an event whose
    /// `+8` is not 1 gets `Event::setPaused(paused != 0)` through
    /// `ERRCHECK`.
    pub fn music_pause_all(h: &mut impl Host, this: u32, paused: u32) {
        for i in 0..MUSIC_SLOTS {
            let slot = music_slot(h, this, i);
            let event: u32 = h.read(slot);
            if event != 0 && h.read::<u32>(slot + 8) != 1 {
                let r = h.call_sym(
                    EVENT_SET_PAUSED,
                    Conv::Stdcall,
                    &[event, u32::from(paused != 0)],
                );
                errcheck(h, r, MUSIC_FILE, 0x3b7);
            }
        }
    }

    /// The music slots (`FmodMusic +0x10`, `0x3c` bytes each).
    pub const MUSIC_SLOTS: u32 = 182;

    fn music_slot(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let arr: u32 = h.read(this + 0x10);
        arr.wrapping_add(0x3c * i)
    }

    /// A slot's volume `v / 127.0` stored at `+0x38` (f32) and sent to
    /// FMOD as `+0x38 * music volume (+0x30)` (`Event::setVolume`) through
    /// `ERRCHECK`.
    fn music_send_volume(h: &mut impl Host, this: u32, i: u32, v: f64, line: u32) {
        let scale = f64_at(h, VOLUME_SCALE);
        let vol = x87::store(x87::div(v, scale));
        let slot = music_slot(h, this, i);
        h.write(slot + 0x38, vol);
        let slot = music_slot(h, this, i);
        let master: u32 = h.read(this + 0x30);
        let arg = x87::store(x87::mul(x87::load(h.read(slot + 0x38)), x87::load(master)));
        let event: u32 = h.read(slot);
        let r = h.call_sym(EVENT_SET_VOLUME, Conv::Stdcall, &[event, arg]);
        errcheck(h, r, MUSIC_FILE, line);
    }

    /// `FmodMusic_setMute(this; mute)` (0x00709520): `+0x24` = mute. Muted
    /// (1): each slot with an event that plays (`+0x20` = 1) loses a fade
    /// (`+0xc` 1 -> 0, `+0x10` 0.0, `+0x14` 0) and gets volume 0; one that
    /// does not is marked (`+0xc` 1, `+0x10` 0.0, `+0x14` 0). Otherwise
    /// each slot's own volume (`+0x34`) goes back through
    /// `FmodMusic_volumeDirect(i, volume, 0)`.
    pub fn music_set_mute(h: &mut impl Host, this: u32, mute: u32) {
        h.write(this + 0x24, mute);
        if mute != 1 {
            for i in 0..MUSIC_SLOTS {
                let slot = music_slot(h, this, i);
                // `fld dword`, then `fstp dword` into the argument word
                let v = x87::store(x87::load(h.read(slot + 0x34)));
                this_call(h, MUSIC_VOLUME_DIRECT, this, &[i, v, 0]);
            }
            return;
        }
        for i in 0..MUSIC_SLOTS {
            let slot = music_slot(h, this, i);
            if h.read::<u32>(slot) == 0 {
                continue;
            }
            if h.read::<u32>(slot + 0x20) == 1 {
                if h.read::<u32>(slot + 0xc) == 1 {
                    h.write(slot + 0xc, 0u32);
                    let slot = music_slot(h, this, i);
                    h.write(slot + 0x10, 0u32);
                    let slot = music_slot(h, this, i);
                    h.write(slot + 0x14, 0u32);
                }
                music_send_volume(h, this, i, 0.0, 0x33a);
            } else {
                h.write(slot + 0xc, 1u32);
                let slot = music_slot(h, this, i);
                h.write(slot + 0x10, 0u32);
                let slot = music_slot(h, this, i);
                h.write(slot + 0x14, 0u32);
            }
        }
    }

    /// `FmodMusic_volumeReset(this; id, volume, keep)` (0x00709f20,
    /// ret 0xc) -> 0: for a playing slot (`id` <= 0xb5, `+0xc`, `+0`
    /// set, `+0x20` = 1): the volume (f32) kept at `+0x34` when `keep` is
    /// 1, sent (0 while muted, `+0x24` = 1), the fade cleared (`+0xc` 0,
    /// `+0x10` 0.0, `+0x14` 0).
    pub fn music_volume_reset(
        h: &mut impl Host,
        this: u32,
        id: u32,
        volume: u32,
        keep: u32,
    ) -> u32 {
        if id > 0xb5 {
            return 0;
        }
        let slot = music_slot(h, this, id);
        if h.read::<u32>(slot + 0xc) == 0
            || h.read::<u32>(slot) == 0
            || h.read::<u32>(slot + 0x20) != 1
        {
            return 0;
        }
        let v = x87::load(volume);
        if keep == 1 {
            h.write(slot + 0x34, x87::store(v));
        }
        // muted: 0.0, through an f32 store; else the argument as loaded
        let v = if h.read::<u32>(this + 0x24) == 1 {
            x87::load(x87::store(0.0))
        } else {
            v
        };
        music_send_volume(h, this, id, v, 0x31c);
        let slot = music_slot(h, this, id);
        h.write(slot + 0xc, 0u32);
        let slot = music_slot(h, this, id);
        h.write(slot + 0x10, 0u32);
        let slot = music_slot(h, this, id);
        h.write(slot + 0x14, 0u32);
        0
    }
}
