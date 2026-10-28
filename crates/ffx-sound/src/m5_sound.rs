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
    use ffx_bridge::Global;

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
}
