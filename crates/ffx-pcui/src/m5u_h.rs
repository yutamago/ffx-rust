//! M5 stage 1, list `render-pcui` (round 3), batch 3: the pc-ui side's
//! small functions: jump thunks, the page manager's lock and helper
//! thread, a pointer-array test, a list walk, the movie player's memory
//! callbacks, and the Iggy player and `GDraw` wrappers.

/// The originals over game memory.
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    pub const WINDOW_CONTROL_SET_TEXT: u32 = 0x0084_a710;
    /// `KERNEL32!LeaveCriticalSection`, `KERNEL32!CreateThread`.
    pub const LEAVE_CRITICAL_SECTION_IAT: u32 = 0x00b0_c158;
    pub const CREATE_THREAD_IAT: u32 = 0x00b0_c104;
    /// `g_PageManager` (a pointer): its lock at `+0x1c0c`, its helper
    /// thread's handle at `+0x1c70`.
    pub const PAGE_MANAGER: u32 = 0x00c6_13b0;
    /// The thread entry `0x00948c30` and the page manager's thread body
    /// `0x00947e30`.
    pub const THREAD_ENTRY: u32 = 0x0094_8c30;
    pub const PAGE_THREAD: u32 = 0x0094_7e30;
    pub const CREATE_THREAD: u32 = 0x0094_8c10;
    /// `memcpy`'s thunk (`jmp [MSVCR110!memcpy]`), `MSVCR110!memmove`'s
    /// slot, `crt_memset_thunk`.
    pub const MEMCPY_THUNK: u32 = 0x0094_9256;
    pub const MEMMOVE_IAT: u32 = 0x00b0_c34c;
    pub const MEMSET_THUNK: u32 = 0x0094_964c;
    /// `iggy_w32!IggyPlayerGotoFrameRS@12`, `IggyPlayerDestroy@4`.
    pub const IGGY_GOTO_FRAME_IAT: u32 = 0x00b0_c7f4;
    pub const IGGY_DESTROY_IAT: u32 = 0x00b0_c814;
    pub const IGGY_REPLAY: u32 = 0x00a3_db30;
    /// The `GDraw` globals: the context (`+4` the device object) and the
    /// block whose address `0x00a3d7c0` returns.
    pub const GDRAW: u32 = 0x01a8_58b4;
    pub const GDRAW_BLOCK: u32 = 0x01a8_58cc;
    pub const GDRAW_CALLBACK: u32 = 0x00a3_b480;
    pub const GDRAW_RELEASE: u32 = 0x00a3_d310;
    pub const GDRAW_TEARDOWN: u32 = 0x00a3_d410;

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    /// A stdcall import through its slot `iat` -> its `eax`.
    fn import(h: &mut impl Host, iat: u32, args: &[u32]) -> u32 {
        let f = word(h, h.rebase(iat));
        h.call(f, Conv::Stdcall, args)
    }

    /// A `jmp target` thunk: `target` with the thunk's own arguments
    /// (`conv` as the target takes them) -> its `eax`.
    pub fn forward(h: &mut impl Host, target: u32, conv: Conv, args: &[u32]) -> u32 {
        h.call_sym(target, conv, args)
    }

    /// `LeaveCriticalSection_ecx_009453c0(cs)` (`ecx`): `LeaveCriticalSection(cs)`.
    pub fn leave(h: &mut impl Host, cs: u32) -> u32 {
        import(h, LEAVE_CRITICAL_SECTION_IAT, &[cs])
    }

    /// `PageManager_unlock_00947ea0()`: `LeaveCriticalSection` on the page
    /// manager's lock (`+0x1c0c`).
    pub fn page_unlock(h: &mut impl Host) -> u32 {
        let pm = word(h, h.rebase(PAGE_MANAGER));
        import(h, LEAVE_CRITICAL_SECTION_IAT, &[pm.wrapping_add(0x1c0c)])
    }

    /// `PtrArray_isSet_00947da0(this; i)` (`ret 4`): `[[this + 4] + 4 i]`
    /// set.
    pub fn is_set(h: &impl Mem, this: u32, i: u32) -> u32 {
        let data = word(h, this + 4);
        u32::from(word(h, data.wrapping_add(i.wrapping_mul(4))) != 0)
    }

    /// `Thread_create_00948c10(arg)`: `CreateThread(0, 0, 0x00948c30, arg,
    /// 0, 0)` -> the handle.
    pub fn create_thread(h: &mut impl Host, arg: u32) -> u32 {
        let entry = h.rebase(THREAD_ENTRY);
        import(h, CREATE_THREAD_IAT, &[0, 0, entry, arg, 0, 0])
    }

    /// `PageManager_startThread_00947630()`: the page manager's helper
    /// thread (`0x00947e30` through `Thread_create_00948c10`), its handle
    /// at `+0x1c70` (the manager read before the call).
    pub fn page_start_thread(h: &mut impl Host) -> u32 {
        let pm = word(h, h.rebase(PAGE_MANAGER));
        let body = h.rebase(PAGE_THREAD);
        let t = h.call_sym(CREATE_THREAD, Conv::Cdecl, &[body]);
        h.write(pm.wrapping_add(0x1c70), t);
        t
    }

    /// `List_walkToEnd_00948a80(this)`: follows `+0x14`, then each node's
    /// `+0xc`, to the end -> 0 (the last link read).
    pub fn walk_to_end(h: &impl Mem, this: u32) -> u32 {
        let mut node = word(h, this + 0x14);
        while node != 0 {
            node = word(h, node.wrapping_add(0xc));
        }
        0
    }

    /// `Thread_entry_00948c30(f)` (stdcall): `f()` -> its `eax`.
    pub fn thread_entry(h: &mut impl Host, f: u32) -> u32 {
        h.call(f, Conv::Cdecl, &[])
    }

    /// `PVideoPlaybackWin32_memcpy/memmove/memset` (`0x00a26fb0`..):
    /// the CRT function with the same three words -> its `eax`.
    pub fn crt3(h: &mut impl Host, which: u32, dst: u32, src: u32, len: u32) -> u32 {
        if which == MEMMOVE_IAT {
            let func = word(h, h.rebase(MEMMOVE_IAT));
            return h.call(func, Conv::Cdecl, &[dst, src, len]);
        }
        h.call_sym(which, Conv::Cdecl, &[dst, src, len])
    }

    /// `IggyPlayer_replay_00a3db30(this)`: `IggyPlayerGotoFrameRS([this],
    /// 0, 0)`.
    pub fn iggy_replay(h: &mut impl Host, this: u32) -> u32 {
        let player = word(h, this);
        import(h, IGGY_GOTO_FRAME_IAT, &[player, 0, 0])
    }

    /// `IggyMenu_replay(menu)`: `IggyPlayer_replay_00a3db30` on the menu's
    /// player holder (`+0x6c`).
    pub fn iggy_menu_replay(h: &mut impl Host, menu: u32) -> u32 {
        let holder = word(h, menu + 0x6c);
        h.call_sym(IGGY_REPLAY, Conv::Thiscall(holder), &[])
    }

    /// `GDraw_releaseHandle_00a362c0(handle)`: the `GDraw` device's method
    /// `+0x3c` (stdcall, `this` first) with `[handle]` and 0 -> `[handle +
    /// 8]`.
    pub fn gdraw_release_handle(h: &mut impl Host, handle: u32) -> u32 {
        let g = word(h, h.rebase(GDRAW));
        let dev = word(h, g.wrapping_add(4));
        let vt = word(h, dev);
        let f = word(h, vt.wrapping_add(0x3c));
        let res = word(h, handle);
        h.call(f, Conv::Stdcall, &[dev, res, 0]);
        word(h, handle.wrapping_add(8))
    }

    /// `GDraw_callbackIf_00a398e0(a, key, x)` (stdcall): when `[a + 0x14]`
    /// is `key`, `0x00a3b480(a, x)` (its `eax`); else `[a + 0x14]`.
    pub fn callback_if(h: &mut impl Host, a: u32, key: u32, x: u32) -> u32 {
        let k = word(h, a.wrapping_add(0x14));
        if k != key {
            return k;
        }
        h.call_sym(GDRAW_CALLBACK, Conv::Cdecl, &[a, x])
    }

    /// `GDraw_releaseThis_00a3d3c0(this)`: `0x00a3d310(this)` -> `this`.
    pub fn release_this(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(GDRAW_RELEASE, Conv::Thiscall(this), &[]);
        this
    }

    /// `Callback_invoke_00a3d6d0(p)`: the callback pair at `[p + 4]`
    /// (object, function) called as `function(object; p)` (callee pops).
    pub fn invoke(h: &mut impl Host, p: u32) -> u32 {
        let pair = word(h, p.wrapping_add(4));
        let obj = word(h, pair);
        let f = word(h, pair.wrapping_add(4));
        h.call(f, Conv::Thiscall(obj), &[p])
    }

    /// `IggyPlayerHolder_destroy_00a3d780(this; x)` (`ret 4`, `x` unread):
    /// a player at `+0` is destroyed (`IggyPlayerDestroy`) and cleared ->
    /// its `eax`, or 0 without one.
    pub fn iggy_destroy(h: &mut impl Host, this: u32) -> u32 {
        let p = word(h, this);
        if p == 0 {
            return 0;
        }
        let r = import(h, IGGY_DESTROY_IAT, &[p]);
        h.write(this, 0_u32);
        r
    }

    /// `AddrOfGlobal01a858cc_00a3d7c0()` -> the `GDraw` block's address.
    pub fn gdraw_block(h: &impl Mem) -> u32 {
        h.rebase(GDRAW_BLOCK)
    }
}
