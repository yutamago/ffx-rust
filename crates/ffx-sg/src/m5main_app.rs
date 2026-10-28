//! M5 `main`: `FFXApplication`'s own methods (the constructor, the
//! deleting destructor, `afterGraphicSync`, the global-time step), the
//! launcher reset, a `PhyreMalloc` copy helper, the `PhyreFree` thunk and
//! the save screen's repeat-key poll.
//!
//! The constructor's C++ exception frame (`fs:[0]`) is left out: nothing
//! on its path throws into it, and no callee sees it.

/// `FFXApplication`'s vtable.
pub const VTABLE: u32 = 0x00b0_d998;
/// `g_hWnd`.
pub const HWND: u32 = 0x00cc_9ce8;
/// The word that makes the destructor close the window (`== 1`).
pub const CLOSE_ON_DESTROY: u32 = 0x00c8_fae4;
/// `g_PauseOn`.
pub const PAUSE_ON: u32 = 0x0134_0828;
/// The global time's wrap point (`f32`, `.rdata`).
pub const TIME_WRAP: u32 = 0x00b0_db50;
/// The byte that stops the constructor's `PUtility` registrations.
pub const UTILITY_STOP: u32 = 0x00c0_a09c;
/// The `PUtility` objects the constructor registers (slot 1 of each),
/// in order.
pub const UTILITIES: [u32; 7] = [
    0x00c0_a37c,
    0x00c0_a33c,
    0x00c0_a2d0,
    0x00c0_a3c0,
    0x00c0_a3fc,
    0x00c0_a440,
    0x00c9_0288,
];

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers and points: h, x, y, z, m, v
pub mod bridge {
    use super::{CLOSE_ON_DESTROY, HWND, PAUSE_ON, TIME_WRAP, UTILITIES, UTILITY_STOP, VTABLE};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre::PFramework::PApplication::PApplication` (thiscall).
    pub const APPLICATION_CTOR: u32 = 0x0061_f810;
    /// `PApplication::setWindowIconID` (thiscall, one word).
    pub const SET_WINDOW_ICON_ID: u32 = 0x0062_cf50;
    /// `PApplication::GetApplication` (cdecl).
    pub const GET_APPLICATION: u32 = 0x0062_3b90;
    /// `PApplication`'s destructor (thiscall).
    pub const APPLICATION_DTOR: u32 = 0x0062_0bc0;
    /// `Phyre::PhyreFree` (cdecl).
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `Phyre::PhyreMalloc(size, align)` (cdecl).
    pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
    /// `memcpy`'s import thunk (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    /// `USER32!SendMessageW`'s import slot (stdcall, four words).
    pub const SEND_MESSAGE_IAT: u32 = 0x00b0_c67c;
    /// `WM_CLOSE`.
    pub const WM_CLOSE: u32 = 0x10;
    /// `graphicDynGeoMemManagerSwapBuffer(keep)` (cdecl).
    pub const DYN_GEO_SWAP: u32 = 0x0063_ff70;
    /// `graphicIsEscMenuActive()` (cdecl).
    pub const IS_ESC_MENU_ACTIVE: u32 = 0x0063_da80;
    /// `FreeCamera_CreateInstance()` (cdecl).
    pub const FREE_CAMERA: u32 = 0x0042_f260;
    /// `inputGetCurrDeviceStickInfo()` (cdecl): the left stick, `x`
    /// then `y` (`f32`).
    pub const STICK_INFO: u32 = 0x0063_0c90;
    /// `inputIsHoldButton(mask)` and `inputIsPressButton(mask)` (cdecl).
    pub const IS_HOLD: u32 = 0x0063_0cd0;
    pub const IS_PRESS: u32 = 0x0063_0cf0;
    /// The repeat poll's stick pointer and its eight counters.
    pub const REPEAT_STICK: u32 = 0x00ce_81a4;
    pub const REPEAT_COUNTERS: u32 = 0x00ce_8184;
    /// The stick thresholds the poll compares with (`f32`, `.rdata`):
    /// `-0.3` and `0.3`.
    pub const STICK_NEG: u32 = 0x00b4_9998;
    pub const STICK_POS: u32 = 0x00b3_eb44;

    fn rd8(h: &impl Host, at: u32) -> u8 {
        h.read(at)
    }

    /// `FFXApplication::FFXApplication(this)` (`0x0042ef40`, thiscall):
    /// the base class, then `+0x3b0` = 0.0, the vtable, `+0x3a0` = 0,
    /// `+0x3ac` = 1 (a word), `+0x288` = 1; the window icon 0x65; the
    /// application's 1280 x 720 (`+0x26c`, `+0x270`) and `+0x28c` = 2;
    /// then slot 1 of each [`UTILITIES`] object while
    /// [`UTILITY_STOP`] stays 0. Returns `this`.
    pub fn ctor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(APPLICATION_CTOR, Conv::Thiscall(this), &[]);
        h.write(this + 0x3b0, 0u32);
        h.write(this, h.rebase(VTABLE));
        h.write(this + 0x3a0, 0u8);
        h.write(this + 0x3ac, 1u16);
        h.write(this + 0x288, 1u8);
        h.call_sym(SET_WINDOW_ICON_ID, Conv::Thiscall(this), &[0x65]);
        for (field, v) in [(0x26c, 0x500u32), (0x270, 0x2d0), (0x28c, 2)] {
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            h.write(app + field, v);
        }
        for obj in UTILITIES {
            if rd8(h, h.rebase(UTILITY_STOP)) != 0 {
                break;
            }
            let obj = h.rebase(obj);
            h.call_virtual(obj, 1, &[]);
        }
        this
    }

    /// `SendMessageW(g_hWnd, WM_CLOSE, 0, 0)`; its `eax`.
    fn close_window(h: &mut impl Host) -> u32 {
        let f: u32 = h.read(h.rebase(SEND_MESSAGE_IAT));
        let hwnd: u32 = h.read(h.rebase(HWND));
        h.call(f, Conv::Stdcall, &[hwnd, WM_CLOSE, 0, 0])
    }

    /// `FFXApplication`'s scalar deleting destructor `(this; flags)`
    /// (`0x0042f150`, thiscall, `ret 4`): the vtable back, the window
    /// closed when [`CLOSE_ON_DESTROY`] is 1, the base destructor, and
    /// `PhyreFree(this)` when `flags & 1`. Returns `this`.
    pub fn deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        h.write(this, h.rebase(VTABLE));
        if h.read::<u32>(h.rebase(CLOSE_ON_DESTROY)) == 1 {
            close_window(h);
        }
        h.call_sym(APPLICATION_DTOR, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x0042f140`: `jmp Phyre::PhyreFree` (the thunk C++ unwinding
    /// code calls); `PhyreFree`'s `eax`.
    pub fn phyre_free_thunk(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p])
    }

    /// `0x0042f220(src, size)` (cdecl): 0 for a null `src`; else a copy
    /// of `size` bytes in `PhyreMalloc(size, 2)` (0 when that fails).
    pub fn mem_dup(h: &mut impl Host, src: u32, size: u32) -> u32 {
        if src == 0 {
            return 0;
        }
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 2]);
        if p != 0 {
            h.call_sym(MEMCPY, Conv::Cdecl, &[p, src, size]);
        }
        p
    }

    /// `ResetToLauncher()` (`0x0042f4d0`, cdecl): `SendMessageW(g_hWnd,
    /// WM_CLOSE, 0, 0)`; its result.
    pub fn reset_to_launcher(h: &mut impl Host) -> u32 {
        close_window(h)
    }

    /// `FFXApplication::afterGraphicSync(this)` (`0x0042f4f0`, thiscall,
    /// vtable slot 10): while `+0x3ad` is 0, the dynamic geometry's
    /// buffers swap (`keep` 1) and `+0x3ad` becomes whether the Esc menu
    /// is up. Returns `eax` as the original leaves it on that path (the
    /// menu test's upper bytes and the flag); on the other path the
    /// original leaves the caller's `eax`, which its one caller
    /// (`0x00627df6`, `xor eax, eax` after it) drops, and this returns 0.
    pub fn after_graphic_sync(h: &mut impl Host, this: u32) -> u32 {
        if rd8(h, this + 0x3ad) != 0 {
            return 0;
        }
        h.call_sym(DYN_GEO_SWAP, Conv::Cdecl, &[1]);
        let r = h.call_sym(IS_ESC_MENU_ACTIVE, Conv::Cdecl, &[]);
        let flag = u8::from(r != 0);
        h.write(this + 0x3ad, flag);
        (r & 0xffff_ff00) | u32::from(flag)
    }

    /// `FFXApplication::updateGlobalTime(this)` (`0x0042fa00`, thiscall,
    /// from `animate`): with `+0x3ac` set, no pause, and the free camera
    /// neither active (`+0x338`, `+0x33c`) nor in mode 2, `+0x3b0` +=
    /// the frame step `+0x24`; from [`TIME_WRAP`] on it is 0 again; then
    /// `+0x120` = `+0x3b0`.
    pub fn update_global_time(h: &mut impl Host, this: u32) {
        if rd8(h, this + 0x3ac) != 0 && h.read::<u32>(h.rebase(PAUSE_ON)) == 0 {
            let cam = h.call_sym(FREE_CAMERA, Conv::Cdecl, &[]);
            if rd8(h, cam + 0x338) == 0 && rd8(h, cam + 0x33c) == 0 && h.read::<u32>(cam) != 2 {
                let t = x87::add(
                    x87::load(h.read(this + 0x24)),
                    x87::load(h.read(this + 0x3b0)),
                );
                h.write(this + 0x3b0, x87::store(t));
            }
        }
        let wrap = x87::load(h.read(h.rebase(TIME_WRAP)));
        let t = x87::load(h.read(this + 0x3b0));
        // fcomp, test ah 0x41, jp: zero unless wrap > t or unordered
        if !(wrap > t || wrap.is_nan() || t.is_nan()) {
            h.write(this + 0x3b0, 0u32);
        }
        let t: u32 = h.read(this + 0x3b0);
        h.write(this + 0x120, x87::store(x87::load(t)));
    }

    /// A stick test: the threshold's address, the stick word's offset,
    /// and whether the stick must be below it.
    type StickTest = Option<(u32, u32, bool)>;

    /// A counter that repeats: 1 on the first frame, then every fifth
    /// frame past the tenth.
    const fn fires(n: i32) -> bool {
        n == 1 || (n > 10 && n % 5 == 0)
    }

    /// `0x006ef320()` (cdecl; name ours, `SaveData_PollRepeatInput`):
    /// the save screen's held-direction counters, up (button 0x1000 or
    /// stick `y < -0.3`), down (0x4000, `y > 0.3`), left (0x8000,
    /// `x < -0.3`), right (0x2000, `x > 0.3`), buttons 1 and 2 held, and
    /// 0x20 and 0x40 pressed: each counts the frames it holds, 0 when
    /// not. Returns 1..8 for the first counter that [`fires`], else 0.
    pub fn poll_repeat_input(h: &mut impl Host) -> u32 {
        let stick = h.call_sym(STICK_INFO, Conv::Cdecl, &[]);
        let stick_at = h.rebase(REPEAT_STICK);
        h.write(stick_at, stick);
        let neg = h.rebase(STICK_NEG);
        let pos = h.rebase(STICK_POS);
        let base = h.rebase(REPEAT_COUNTERS);
        // (mask, press, threshold, the stick word's offset, below)
        let tests: [(u32, bool, StickTest); 8] = [
            (0x1000, false, Some((neg, 4, true))),
            (0x4000, false, Some((pos, 4, false))),
            (0x8000, false, Some((neg, 0, true))),
            (0x2000, false, Some((pos, 0, false))),
            (1, false, None),
            (2, false, None),
            (0x20, true, None),
            (0x40, true, None),
        ];
        for (i, (mask, press, stick_test)) in (0u32..).zip(tests) {
            let f = if press { IS_PRESS } else { IS_HOLD };
            let held = h.call_sym(f, Conv::Cdecl, &[mask]) != 0;
            let on = held
                || stick_test.is_some_and(|(c, off, below)| {
                    let s: u32 = h.read(stick_at);
                    let v = x87::load(h.read(s + off));
                    let c = x87::load(h.read(c));
                    // fld c; fcomp v: ordered c > v (below) or c < v
                    if below { c > v } else { c < v }
                });
            let at = base + 4 * i;
            let n: u32 = h.read(at);
            h.write(at, if on { n.wrapping_add(1) } else { 0 });
        }
        for i in 0..8u32 {
            if fires(h.read::<u32>(base + 4 * i).cast_signed()) {
                return i + 1;
            }
        }
        0
    }
}
