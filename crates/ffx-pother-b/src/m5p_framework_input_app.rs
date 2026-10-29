//! `PFramework` application helpers (list `phyre-framework-input`, batch
//! 3): `PhyreExit`, the scripted frame, the viewport's render, the
//! window's host attach and mouse hook, the input devices' destructors and
//! keyboard binding, the input log's script-buffer scanners and the
//! `std::map` iterators the input tables walk.
//!
//! The script buffer (`PApplicationScriptBuffer`) keeps its cursor at
//! `+0xc` and its end at `+0x10`. `std::map` nodes are MSVC's `_Tree`
//! nodes: `+0` left, `+4` parent, `+8` right, `+0xd` the nil flag.

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

use crate::m5p_framework_input_small::float_copy;

/// `g_PhyreApplication`.
pub const G_APPLICATION: u32 = 0x00cc_9cd8;
/// The byte `PhyreExit` checks and clears (the engine is up).
pub const G_PHYRE_RUNNING: u32 = 0x00c9_01a0;
/// The engine-object registry's list head (next at `+4`).
pub const REGISTRY_HEAD: u32 = 0x00c9_01a8;
/// `Phyre_PTimerWin32_Terminate` (cdecl).
pub const TIMER_TERMINATE: u32 = 0x0043_0040;
/// `Phyre_PThreadPool_GetSingleton` (cdecl).
pub const THREAD_POOL: u32 = 0x0044_cd30;
/// `Phyre_PNamespace_clearGlobal_0043e610` (cdecl).
pub const NAMESPACE_CLEAR: u32 = 0x0043_e610;
/// A bare `ret` `PhyreExit` calls between the namespace and the lock.
pub const EXIT_NOP: u32 = 0x0046_7940;
/// `Phyre_PThreadPool_releaseLock_0045da00` (cdecl).
pub const POOL_RELEASE: u32 = 0x0045_da00;
/// `PApplicationScriptBuffer`'s newline search (thiscall; in the list,
/// `0x006277b0`).
pub const FIND_NEWLINE: u32 = 0x0062_77b0;
/// `PApplicationScript::executeNextCommand` (thiscall, no stack words).
pub const EXECUTE_NEXT: u32 = 0x0062_7480;
/// `PRendererBase::setViewport(x, y, w, h)` (thiscall, `ret 0x10`).
pub const SET_VIEWPORT: u32 = 0x005b_c060;
/// `PMotionBlur`'s target getter (`[ecx + 8]`, thiscall).
pub const BLUR_TARGET: u32 = 0x00a1_61b0;
/// `PMotionBlurBase::initializeCommonShaders(name, target)` (thiscall,
/// `ret 8`; in the list).
pub const BLUR_INIT_SHADERS: u32 = 0x00a1_8c80;
/// The shader-set name `0x00a1b7e0` passes.
pub const BLUR_SHADER_NAME: u32 = 0x0194_3da0;
/// `"PHYREMSG_SENDHWND"`.
pub const MSG_SEND_HWND: u32 = 0x00b4_0d04;
/// `g_hWnd`.
pub const G_HWND: u32 = 0x00cc_9ce8;
/// `g_WindowNcDrag` (byte).
pub const G_NC_DRAG: u32 = 0x00cc_9cd3;
/// `g_hMouseHook`.
pub const G_MOUSE_HOOK: u32 = 0x00cc_9cf0;
/// The word `0x00623ba0` copies second.
pub const G_INPUT_WORD: u32 = 0x00c3_0fa4;
/// Import slots (stdcall unless said).
pub const SET_PARENT_SLOT: u32 = 0x00b0_c69c;
pub const REGISTER_WINDOW_MESSAGE_SLOT: u32 = 0x00b0_c68c;
pub const SEND_MESSAGE_SLOT: u32 = 0x00b0_c6a4;
pub const POST_MESSAGE_SLOT: u32 = 0x00b0_c6ac;
pub const CALL_NEXT_HOOK_SLOT: u32 = 0x00b0_c664;
/// `MSVCR110!isspace` (cdecl).
pub const ISSPACE_SLOT: u32 = 0x00b0_c400;
/// `MSVCR110!sscanf_s` (cdecl, varargs).
pub const SSCANF_SLOT: u32 = 0x00b0_c418;
/// `"%f"`.
pub const FORMAT_FLOAT: u32 = 0x00b4_0274;
/// `Phyre_PTimerWin32_GetTime` (cdecl, `double` in `ST0`).
pub const GET_TIME: u32 = 0x0042_fd80;
/// `g_AppStartTime` (`f64`).
pub const G_APP_START_TIME: u32 = 0x00c9_0278;
/// The input-map lookup `0x006282c0` makes (thiscall `(id)`, `ret 4`).
pub const MAP_LOOKUP: u32 = 0x009e_ccb0;
/// The axis value of a found mapping (thiscall `(mapping)`, `ret 4`,
/// `float` in `ST0`; in the list).
pub const MAPPING_VALUE: u32 = 0x0062_9620;
/// `operator delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `std::map` subtree erase (thiscall `(node)`, `ret 4`).
pub const TREE_ERASE: u32 = 0x0062_5960;
/// `PCriticalSectionBase::release` (thiscall).
pub const CS_RELEASE: u32 = 0x0043_1950;
/// `eh vector destructor iterator(array, size, count, dtor)` (stdcall).
pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
/// The pad slot destructor `0x00620ff0` hands it.
pub const PAD_SLOT_DTOR: u32 = 0x0062_1190;
/// `PInputDevice`'s vtable.
pub const DEVICE_VTABLE: u32 = 0x00b3_fbf8;
/// `PInputDeviceTouch`'s vtable.
pub const TOUCH_VTABLE: u32 = 0x00b3_fc4c;
/// `PInputDevicePad`'s vtable.
pub const PAD_VTABLE: u32 = 0x00b3_fca4;

/// What `0x00626a00` leaves in `eax`: `al` the answer over the upper
/// bytes of the last word it loaded into `eax` (`upper`).
#[must_use]
pub const fn al_over(upper: u32, al: bool) -> u32 {
    (upper & 0xffff_ff00) | al as u32
}

pub mod bridge {
    use super::{
        BLUR_INIT_SHADERS, BLUR_SHADER_NAME, BLUR_TARGET, CALL_NEXT_HOOK_SLOT, CS_RELEASE, Conv,
        DEVICE_VTABLE, EH_VECTOR_DTOR, EXECUTE_NEXT, EXIT_NOP, FIND_NEWLINE, FORMAT_FLOAT,
        G_APP_START_TIME, G_APPLICATION, G_HWND, G_INPUT_WORD, G_MOUSE_HOOK, G_NC_DRAG,
        G_PHYRE_RUNNING, GET_TIME, Host, ISSPACE_SLOT, MAP_LOOKUP, MAPPING_VALUE, MSG_SEND_HWND,
        Mem, NAMESPACE_CLEAR, OPERATOR_DELETE, PAD_SLOT_DTOR, PAD_VTABLE, POOL_RELEASE,
        POST_MESSAGE_SLOT, REGISTER_WINDOW_MESSAGE_SLOT, REGISTRY_HEAD, SEND_MESSAGE_SLOT,
        SET_PARENT_SLOT, SET_VIEWPORT, SSCANF_SLOT, THREAD_POOL, TIMER_TERMINATE, TOUCH_VTABLE,
        TREE_ERASE, al_over, float_copy, x87,
    };

    fn import(h: &impl Mem, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// `Phyre::PhyreExit` (`0x00430d60`, cdecl): when the engine is up,
    /// every registry entry's virtual `+8`, the timer, the thread pool's
    /// virtual `+0xc`, the global namespace, the pool lock; the flag
    /// cleared. `Some(eax)` (the lock release's), `None` when the engine
    /// was down (the caller's `eax`).
    pub fn phyre_exit(h: &mut impl Host) -> Option<u32> {
        let running = h.rebase(G_PHYRE_RUNNING);
        let up: u8 = h.read(running);
        if up == 0 {
            return None;
        }
        let mut e: u32 = h.read(h.rebase(REGISTRY_HEAD));
        while e != 0 {
            h.call_virtual(e, 2, &[]);
            e = h.read(e + 4);
        }
        h.call_sym(TIMER_TERMINATE, Conv::Cdecl, &[]);
        let pool = h.call_sym(THREAD_POOL, Conv::Cdecl, &[]);
        h.call_virtual(pool, 3, &[]);
        h.call_sym(NAMESPACE_CLEAR, Conv::Cdecl, &[]);
        h.call_sym(EXIT_NOP, Conv::Cdecl, &[]);
        let eax = h.call_sym(POOL_RELEASE, Conv::Cdecl, &[]);
        h.write(running, 0u8);
        Some(eax)
    }

    /// `PApplication::scriptedFrame` (`0x0062c6f0`): 9 while paused
    /// (`+0x39`); on the first frame (`+0x34` 0) the virtual `+0x24`, whose
    /// non-zero answer is returned; else the frame count grows and the
    /// script (`+0x2c`) runs its next command (tail jump), its `eax`.
    pub fn scripted_frame(h: &mut impl Host, this: u32) -> u32 {
        let paused: u8 = h.read(this + 0x39);
        if paused != 0 {
            return 9;
        }
        let frames: u32 = h.read(this + 0x34);
        if frames == 0 {
            let r = h.call_virtual(this, 9, &[]);
            if r != 0 {
                return r;
            }
        }
        let frames: u32 = h.read(this + 0x34);
        h.write(this + 0x34, frames.wrapping_add(1));
        let script: u32 = h.read(this + 0x2c);
        h.call_sym(EXECUTE_NEXT, Conv::Thiscall(script), &[])
    }

    /// `PApplicationViewport::render` (`0x0062be00`): 0x15 without an
    /// application; else the renderer `+0xc`'s `setViewport(x +0x14,
    /// app height +0x270 - y +0x18 - h +0x20, w +0x1c, h)`, its `eax`.
    pub fn viewport_render(h: &mut impl Host, this: u32) -> u32 {
        let app: u32 = h.read(h.rebase(G_APPLICATION));
        if app == 0 {
            return 0x15;
        }
        let height: u32 = h.read(this + 0x20);
        let y: u32 = h.read(this + 0x18);
        let app_height: u32 = h.read(app + 0x270);
        let x: u32 = h.read(this + 0x14);
        let w: u32 = h.read(this + 0x1c);
        let renderer: u32 = h.read(this + 0xc);
        let top = app_height.wrapping_sub(y).wrapping_sub(height);
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[x, top, w, height])
    }

    /// `PMotionBlur` slot 2 (`0x00a1b7e0`): with a target (`+8` and its
    /// first word set) the common shaders for it, whose non-zero answer is
    /// returned; else (or on 0) `+0xc` = `+8` and 0.
    pub fn motion_blur_init(h: &mut impl Host, this: u32) -> u32 {
        let t: u32 = h.read(this + 8);
        if t != 0 && h.read::<u32>(t) != 0 {
            let target = h.call_sym(BLUR_TARGET, Conv::Thiscall(this), &[]);
            let name = h.rebase(BLUR_SHADER_NAME);
            let r = h.call_sym(BLUR_INIT_SHADERS, Conv::Thiscall(this), &[name, target]);
            if r != 0 {
                return r;
            }
        }
        let t: u32 = h.read(this + 8);
        h.write(this + 0xc, t);
        0
    }

    /// `PWindowWin32::AttachToHost(host, hwnd)` (`0x00623d50`, cdecl):
    /// `SetParent(hwnd, host)`, then `SendMessageA(host,
    /// RegisterWindowMessageA("PHYREMSG_SENDHWND"), hwnd, 0)`; its `eax`.
    /// `None` without a host (the caller's `eax`).
    pub fn attach_to_host(h: &mut impl Host, host: u32, hwnd: u32) -> Option<u32> {
        if host == 0 {
            return None;
        }
        let f = import(h, SET_PARENT_SLOT);
        h.call(f, Conv::Stdcall, &[hwnd, host]);
        let name = h.rebase(MSG_SEND_HWND);
        let f = import(h, REGISTER_WINDOW_MESSAGE_SLOT);
        let msg = h.call(f, Conv::Stdcall, &[name]);
        let f = import(h, SEND_MESSAGE_SLOT);
        Some(h.call(f, Conv::Stdcall, &[host, msg, hwnd, 0]))
    }

    /// `0x00623ba0` (cdecl `(out)`): `out[0]` = the application's `+4`
    /// (0 without one), `out[1]` = the word at `0x00c30fa4`; `out`.
    pub fn app_pair(m: &mut impl Mem, out: u32) -> u32 {
        let app: u32 = m.read(m.rebase(G_APPLICATION));
        let word: u32 = m.read(m.rebase(G_INPUT_WORD));
        m.write(out, if app == 0 { 0 } else { app + 4 });
        m.write(out + 4, word);
        out
    }

    /// `0x00627810` (thiscall): the first byte from the cursor below the
    /// end that `isspace` (sign-extended) accepts, or 0.
    pub fn find_space(h: &mut impl Host, this: u32) -> u32 {
        let f = import(h, ISSPACE_SLOT);
        let mut p: u32 = h.read(this + 0xc);
        while p < h.read::<u32>(this + 0x10) {
            let c: i8 = h.read(p);
            if h.call(f, Conv::Cdecl, &[c as u32]) != 0 {
                return p;
            }
            p += 1;
        }
        0
    }

    /// `0x0062cf90` (thiscall): skips blanks (`isspace`) and `#` comment
    /// lines from the cursor; the cursor and the answer are the next
    /// command's first byte, or the end and 0.
    pub fn skip_blank(h: &mut impl Host, this: u32) -> u32 {
        let f = import(h, ISSPACE_SLOT);
        loop {
            let mut p: u32 = h.read(this + 0xc);
            let mut found = false;
            while p < h.read::<u32>(this + 0x10) {
                let c: i8 = h.read(p);
                if h.call(f, Conv::Cdecl, &[c as u32]) == 0 {
                    found = true;
                    break;
                }
                p += 1;
            }
            let end: u32 = h.read(this + 0x10);
            if !found {
                h.write(this + 0xc, end);
                return 0;
            }
            h.write(this + 0xc, p);
            if h.read::<u8>(p) != b'#' {
                return p;
            }
            let nl = h.call_sym(FIND_NEWLINE, Conv::Thiscall(this), &[]);
            if nl == 0 {
                let end: u32 = h.read(this + 0x10);
                h.write(this + 0xc, end);
                return 0;
            }
            h.write(this + 0xc, nl);
        }
    }

    /// `0x006282c0` (thiscall `(id)`, `ret 4`): the input's (`+0x2b8`)
    /// map lookup of `id` and that mapping's value, through an `f32`; 0
    /// without a map or a mapping.
    pub fn axis_value(h: &mut impl Host, this: u32, id: u32) -> f32 {
        let input: u32 = h.read(this + 0x2b8);
        let map: u32 = h.read(input);
        if map == 0 {
            return 0.0;
        }
        let mapping = h.call_sym(MAP_LOOKUP, Conv::Thiscall(map), &[id]);
        if mapping == 0 {
            return 0.0;
        }
        let v = h.call_sym_st0(MAPPING_VALUE, Conv::Thiscall(input), &[mapping]);
        f32::from_bits(float_copy(v.to_bits()))
    }

    /// `PApplication::setMousePosition(x, y)` (`0x0062bea0`): the input's
    /// (`+0x2b8`) `+0xa8`/`+0xac` = x, y and the deltas `+0xb0`/`+0xb4` 0.
    pub fn set_mouse_position(m: &mut impl Mem, this: u32, x: u32, y: u32) {
        let input: u32 = m.read(this + 0x2b8);
        m.write(input + 0xa8, x);
        let input: u32 = m.read(this + 0x2b8);
        m.write(input + 0xac, y);
        let input: u32 = m.read(this + 0x2b8);
        m.write(input + 0xb0, 0u32);
        let input: u32 = m.read(this + 0x2b8);
        m.write(input + 0xb4, 0u32);
    }

    /// `std::map::clear` (`0x00626b90`, thiscall): every node from the
    /// root down its left spine, each one's right subtree erased first
    /// and the node deleted; the head's links back to itself, the size 0.
    /// `eax`: the head.
    pub fn tree_clear(h: &mut impl Host, this: u32) -> u32 {
        let head: u32 = h.read(this);
        let mut node: u32 = h.read(head + 4);
        while h.read::<u8>(node + 0xd) == 0 {
            let right: u32 = h.read(node + 8);
            h.call_sym(TREE_ERASE, Conv::Thiscall(this), &[right]);
            let left: u32 = h.read(node);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
            node = left;
        }
        let head: u32 = h.read(this);
        h.write(head + 4, head);
        h.write(head, head);
        h.write(head + 8, head);
        h.write(this + 4, 0u32);
        head
    }

    fn is_nil(m: &impl Mem, node: u32) -> bool {
        m.read::<u8>(node + 0xd) != 0
    }

    /// `std::map::iterator::operator++` (`0x006214b0`, thiscall on the
    /// iterator); the iterator.
    pub fn tree_next(m: &mut impl Mem, it: u32) -> u32 {
        let node: u32 = m.read(it);
        if is_nil(m, node) {
            return it;
        }
        let right: u32 = m.read(node + 8);
        if !is_nil(m, right) {
            let mut cur = right;
            loop {
                let left: u32 = m.read(cur);
                if is_nil(m, left) {
                    break;
                }
                cur = left;
            }
            m.write(it, cur);
            return it;
        }
        let mut p: u32 = m.read(node + 4);
        while !is_nil(m, p) {
            let at: u32 = m.read(it);
            if at != m.read::<u32>(p + 8) {
                break;
            }
            m.write(it, p);
            p = m.read(p + 4);
        }
        m.write(it, p);
        it
    }

    /// `std::map::iterator::operator--` (`0x00621580`): from the end (the
    /// nil head) to the largest node (`+8`); the iterator.
    pub fn tree_prev(m: &mut impl Mem, it: u32) -> u32 {
        let node: u32 = m.read(it);
        if is_nil(m, node) {
            let last: u32 = m.read(node + 8);
            m.write(it, last);
            return it;
        }
        let left: u32 = m.read(node);
        if !is_nil(m, left) {
            let mut cur = left;
            loop {
                let right: u32 = m.read(cur + 8);
                if is_nil(m, right) {
                    break;
                }
                cur = right;
            }
            m.write(it, cur);
            return it;
        }
        let mut p: u32 = m.read(node + 4);
        while !is_nil(m, p) {
            let at: u32 = m.read(it);
            if at != m.read::<u32>(p) {
                break;
            }
            m.write(it, p);
            p = m.read(p + 4);
        }
        let at: u32 = m.read(it);
        if !is_nil(m, at) {
            m.write(it, p);
        }
        it
    }

    /// `PWindowWin32::MouseHookProc(code, wparam, lparam)` (`0x006250e0`,
    /// stdcall): a left-button release during a non-client drag posts
    /// `WM_NCLBUTTONUP` (0xa2) to the window; then `CallNextHookEx`, its
    /// `eax`.
    pub fn mouse_hook(h: &mut impl Host, code: u32, wparam: u32, lparam: u32) -> u32 {
        let drag: u8 = h.read(h.rebase(G_NC_DRAG));
        if code == 0 && wparam == 0x202 && drag != 0 {
            let wnd: u32 = h.read(h.rebase(G_HWND));
            let f = import(h, POST_MESSAGE_SLOT);
            h.call(f, Conv::Stdcall, &[wnd, 0xa2, 0, 0]);
        }
        let hook: u32 = h.read(h.rebase(G_MOUSE_HOOK));
        let f = import(h, CALL_NEXT_HOOK_SLOT);
        h.call(f, Conv::Stdcall, &[hook, code, wparam, lparam])
    }

    /// `0x00625720` (cdecl `(app, text)`): `sscanf_s(text, "%f", &v)` into
    /// the frame word `local`; `+0x3c` = v; when `v <= 0` (ordered) the
    /// clock rebases `+0x28` so the elapsed time is kept:
    /// `((now - start) - t28 - t20) + t28`, stored as `f32`. 0.
    pub fn set_frame_time(h: &mut impl Host, app: u32, text: u32, local: u32) -> u32 {
        let f = import(h, SSCANF_SLOT);
        let format = h.rebase(FORMAT_FLOAT);
        h.call(f, Conv::Cdecl, &[text, format, local]);
        // junk read: [ebp-0x4] is never written when sscanf_s matches
        // nothing; the shim's frame word is 0 then (brief 2026-10-24)
        let bits: u32 = h.read(local);
        h.write(app + 0x3c, float_copy(bits));
        let v = x87::load(bits);
        // fcompp of 0 against v: C0 (0 < v) or unordered skip the rebase;
        // a NaN compares false here too
        if v <= 0.0 {
            let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
            let start: f64 = f64::from_bits(h.read(h.rebase(G_APP_START_TIME)));
            let t28 = x87::load(h.read(app + 0x28));
            let t20 = x87::load(h.read(app + 0x20));
            let r = x87::add(x87::sub(x87::sub(x87::sub(now, start), t28), t20), t28);
            h.write(app + 0x28, x87::store(r));
        }
        0
    }

    /// `0x00626a00` (thiscall `(key, device)`, `ret 8`): takes a pressed
    /// key: for `device < 7` with a state block (`input +0x2b8`, `+0x80 +
    /// 4 device`) and `key - 0x1c < 0x100`, the byte `+0x18 + key - 0x1c`
    /// is read and cleared; `al` whether it was set (see [`al_over`]).
    pub fn take_key(m: &mut impl Mem, this: u32, key: u32, device: u32) -> u32 {
        let input: u32 = m.read(this + 0x2b8);
        if device >= 7 {
            return al_over(input, false);
        }
        let block: u32 = m.read(input.wrapping_add(device * 4).wrapping_add(0x80));
        if block == 0 {
            return al_over(input, false);
        }
        let k = key.wrapping_sub(0x1c);
        if k >= 0x100 {
            return al_over(key, false);
        }
        let at = block.wrapping_add(k).wrapping_add(0x18);
        let set = m.read::<u8>(at) != 0;
        if set {
            m.write(at, 0u8);
        }
        al_over(key, set)
    }

    /// `PInputDeviceTouch`'s destructor (`0x00621130`): its vtable, the
    /// critical section at `+0x17c` released, `PInputDevice`'s vtable; the
    /// release's `eax`.
    pub fn touch_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(TOUCH_VTABLE);
        h.write(this, vt);
        let eax = h.call_sym(CS_RELEASE, Conv::Thiscall(this + 0x17c), &[]);
        let vt = h.rebase(DEVICE_VTABLE);
        h.write(this, vt);
        eax
    }

    /// `PInputDevicePad`'s destructor (`0x00620ff0`): its vtable, the four
    /// 0x30-byte pad slots at `+0x6c` destroyed (`eh vector destructor
    /// iterator`), `PInputDevice`'s vtable; the iterator's `eax`.
    pub fn pad_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(PAD_VTABLE);
        h.write(this, vt);
        let dtor = h.rebase(PAD_SLOT_DTOR);
        let eax = h.call_sym(EH_VECTOR_DTOR, Conv::Stdcall, &[this + 0x6c, 0x30, 4, dtor]);
        let vt = h.rebase(DEVICE_VTABLE);
        h.write(this, vt);
        eax
    }

    /// `PInputDeviceKeyboard::bind(mapping, id)` (`0x00625ca0`, `ret 8`):
    /// 5 for ids below 0x1c, 0x12 for a mapping already bound (`+8`); else
    /// the mapping reads the key byte `this + 0x1fc + id` (`+0x28`), keeps
    /// its device and id, and joins the device's mapping list (tail
    /// insert at `this + 0xc`); 0.
    pub fn keyboard_bind(m: &mut impl Mem, this: u32, mapping: u32, id: u32) -> u32 {
        if (id as i32) < 0x1c {
            return 5;
        }
        if m.read::<u32>(mapping + 8) != 0 {
            return 0x12;
        }
        m.write(mapping + 8, this);
        m.write(mapping + 0x28, this.wrapping_add(0x1fc).wrapping_add(id));
        m.write(mapping + 0xc, id);
        m.write(mapping + 0x2c, 0u32);
        let list = this + 0xc;
        let last: u32 = m.read(list + 4);
        m.write(mapping + 4, last);
        m.write(mapping, list);
        m.write(last, mapping);
        m.write(list + 4, mapping);
        0
    }
}
