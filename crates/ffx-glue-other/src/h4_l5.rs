//! Continuation round 10 of list `h4-glue-other`, batch 3: the escape
//! menu's update.

use ffx_bridge::Global;
use ffx_bridge::x87::{load, mul, store};
use ffx_graphic::host::{Conv, Host};

/// `PInput_GetDevice(kind, index)` (cdecl) -> the device; its vtable
/// `+0xc` is `isPressed(this; key)` (thiscall, ret 4, a bool in `al`).
pub const GET_DEVICE: u32 = 0x0062_3c90;
/// `GetClipCursor`, `ClipCursor` and `ShowCursor` through their import
/// slots (stdcall, one word).
pub const GET_CLIP_CURSOR: Global<u32> = Global::new(0x00b0_c670);
/// See [`GET_CLIP_CURSOR`].
pub const CLIP_CURSOR: Global<u32> = Global::new(0x00b0_c6c4);
/// See [`GET_CLIP_CURSOR`].
pub const SHOW_CURSOR: Global<u32> = Global::new(0x00b0_c688);
/// `graphicGetFreeCameraType()` (cdecl).
pub const FREE_CAMERA_TYPE: u32 = 0x0064_11e0;
/// `g_Settings` (`0x022fb504`): `+8` the cursor mode.
pub const SETTINGS: u32 = 0x022f_b504;
/// `FFEscMenu_unpauseAudioVideo(this)` and `FFEscMenu_pauseAudioVideo(this)`
/// (thiscall).
pub const UNPAUSE: u32 = 0x0068_ff60;
/// See [`UNPAUSE`].
pub const PAUSE: u32 = 0x0068_f940;
/// `IggyMenu_callIggyAS3External(this; a, name, b, c, d)` (thiscall, ret
/// 0x14).
pub const CALL_AS3: u32 = 0x00a2_8ba0;
/// `IggyMenu_setEnable(this; on)` (thiscall, ret 4).
pub const SET_ENABLE: u32 = 0x00a2_8ef0;
/// `SndForcePlayClick(sound)` (cdecl).
pub const PLAY_CLICK: u32 = 0x0088_60f0;
/// `Settings_Save(settings)` (`ecx`, plain ret).
pub const SETTINGS_SAVE: u32 = 0x0040_1d80;
/// `IggyMenu_setEnableCeb248_007029e0(on)` (cdecl).
pub const SAVE_BUSY: u32 = 0x0070_29e0;
/// `FFEscMenu_setEnableParaAndSpecial_006fca50(this; on)` (thiscall, ret 4).
pub const ENABLE_PARA: u32 = 0x006f_ca50;
/// `FFEscMenu_refreshParameterAndSpecialPageEnableState(this)` (thiscall).
pub const REFRESH: u32 = 0x006f_c9b0;
/// `inputIsReleaseButton(button)`, `inputIsPressButton(button)` (cdecl).
pub const IS_RELEASE: u32 = 0x0063_0d10;
/// See [`IS_RELEASE`].
pub const IS_PRESS: u32 = 0x0063_0cf0;
/// `inputGetCurrDeviceStickInfo()` (cdecl) -> the stick block's offset.
pub const STICK_INFO: u32 = 0x0063_0c90;
/// `graphicGetTimeInMicroseconds()` (cdecl) -> `edx:eax`.
pub const TIME_MICRO: u32 = 0x0064_1420;
/// `iggyDataValueSet_00a28d70(out, text)` (cdecl): writes four words.
pub const DATA_VALUE_SET: u32 = 0x00a2_8d70;
/// `IggyMenu_callFlashFunction_1(menu, name, value: 4 words)` (cdecl).
pub const CALL_FLASH: u32 = 0x0068_dd40;
/// `IggyMenuHandlers_flush_00a28ff0(this)` (thiscall).
pub const FLUSH: u32 = 0x00a2_8ff0;
/// `g_EscKeyDown` (`0x00ccc880`): the escape key's last state.
pub const ESC_DOWN: u32 = 0x00cc_c880;
/// `g_SaveMachineBusy` (`0x00ce7310`).
pub const SAVE_MACHINE_BUSY: u32 = 0x00ce_7310;
/// `g_FFEscMenuClosedTime` (`0x00ccc868`, 64 bits).
pub const CLOSED_TIME: u32 = 0x00cc_c868;
/// `g_FFEscMenu` (`0x00ccc870`).
pub const ESC_MENU: u32 = 0x00cc_c870;
/// The six menu keys (`0x00c33ed8`, 12 bytes: release flag, button,
/// command name).
pub const MENU_KEYS: u32 = 0x00c3_3ed8;
/// The four stick directions (`0x00c33f20`, 16 bytes: key, block pointer,
/// scale (f32), command name).
pub const STICK_DIRECTIONS: u32 = 0x00c3_3f20;
/// The stick threshold (`0x00b92194`, f32 0.5).
pub const STICK_THRESHOLD: u32 = 0x00b9_2194;
/// `"onEscClose"` (`0x00b46c34`), `"showEscMenu"` (`0x00b46c40`), `"keyUp"`
/// (`0x00b46c98`), `"setHelpText"` (`0x00b46ca0`).
pub const ON_ESC_CLOSE: u32 = 0x00b4_6c34;
/// See [`ON_ESC_CLOSE`].
pub const SHOW_ESC_MENU: u32 = 0x00b4_6c40;
/// See [`ON_ESC_CLOSE`].
pub const KEY_UP: u32 = 0x00b4_6c98;
/// See [`ON_ESC_CLOSE`].
pub const SET_HELP_TEXT: u32 = 0x00b4_6ca0;
/// Bytes of [`esc_update`]'s frame, from the original's `ebp - 0x18`: the
/// help text's Iggy value (four words).
pub const ESC_FRAME: u32 = 0x10;

/// `callIggyAS3External(this; 0, name, 0, 0, 1)`.
fn as3(h: &mut impl Host, this: u32, name: u32) {
    h.call_sym(CALL_AS3, Conv::Thiscall(this), &[0, name, 0, 0, 1]);
}

/// The escape menu's busy-save pair: `IggyMenu_setEnableCeb248_007029e0(on)`
/// and `setEnableParaAndSpecial(this; on)` while a save loads.
fn save_busy(h: &mut impl Host, this: u32, on: u32) {
    if h.read::<u32>(h.rebase(SAVE_MACHINE_BUSY)) != 0 {
        h.call_sym(SAVE_BUSY, Conv::Cdecl, &[on]);
        h.call_sym(ENABLE_PARA, Conv::Thiscall(this), &[on]);
    }
}

/// The click once per update (`+0x117`).
fn click(h: &mut impl Host, this: u32, sound: u32) {
    let at = this.wrapping_add(0x117);
    if h.read::<u8>(at) == 0 {
        h.call_sym(PLAY_CLICK, Conv::Cdecl, &[sound]);
        h.write(at, 1u8);
    }
}

/// A stick direction's value: its block's float at `stick` times its
/// scale, rounded to f32 (`fld`, `fmul dword`, `fstp dword`).
fn stick_value(h: &impl Host, direction: u32, stick: u32) -> u32 {
    let block: u32 = h.read(direction + 4);
    let x: u32 = h.read(block.wrapping_add(stick));
    let scale: u32 = h.read(direction + 8);
    store(mul(load(x), load(scale)))
}

/// `FFEscMenu_update(this)` (`0x0068ffb0`, `ecx`, plain ret; `frame` see
/// [`ESC_FRAME`]) -> `IggyMenuHandlers_flush`'s `eax` (`None` when the menu
/// is not set up, `+0xa0`, and the original returns at once). With the
/// escape key watched (`+0xe0`), a fresh press of key 0x40 closes an open
/// menu (`+0x70`: cursor clip and visibility restored, audio and video
/// resumed, `onEscClose`, settings saved) or schedules opening a closed
/// one (`+0xe4` 0: the next update pauses and shows it). While open: the
/// six menu keys send `keyUp` and their command on press (release for
/// flagged keys), or `keyUp` on release; the stick's strongest direction
/// over 0.5 sends its command once (`+0x12c`), and the cursor follows
/// `+0x94`. The open and closed times are measured in microseconds
/// (`+0xf0`, `g_FFEscMenuClosedTime`), and a help-text countdown
/// (`+0x234`) sets the help text when it reaches 0.
#[allow(clippy::too_many_lines)] // the original's steps in order
pub fn esc_update(h: &mut impl Host, frame: u32, this: u32) -> Option<u32> {
    let at = |off: u32| this.wrapping_add(off);
    if h.read::<u8>(at(0xa0)) == 0 {
        return None;
    }
    let me = Conv::Thiscall(this);
    let show_cursor = SHOW_CURSOR.get(h);
    let settings = h.rebase(SETTINGS);
    if h.read::<u8>(at(0xe0)) != 0 {
        let device = h.call_sym(GET_DEVICE, Conv::Cdecl, &[1, 0]);
        let vtable: u32 = h.read(device);
        let pressed_fn: u32 = h.read(vtable.wrapping_add(0xc));
        let pressed = h.call(pressed_fn, Conv::Thiscall(device), &[0x40]) as u8;
        let down = h.rebase(ESC_DOWN);
        let fresh = pressed != 0 && h.read::<u8>(down) == 0;
        h.write(down, pressed);
        if fresh {
            if h.read::<u8>(at(0x70)) == 0 {
                let get_clip = GET_CLIP_CURSOR.get(h);
                if h.call(get_clip, Conv::Stdcall, &[at(0x11c)]) != 0 {
                    let clip = CLIP_CURSOR.get(h);
                    h.call(clip, Conv::Stdcall, &[0]);
                    h.write(at(0x118), 1u8);
                }
                if h.read::<u8>(at(0x130)) != 0 {
                    h.write(at(0x130), 0u8);
                    h.call(show_cursor, Conv::Stdcall, &[0]);
                }
                h.write(at(0xe4), 0u32);
            } else {
                if h.read::<u8>(at(0x118)) != 0
                    && (h.call_sym(FREE_CAMERA_TYPE, Conv::Cdecl, &[]) != 0 || {
                        let s: u32 = h.read(settings);
                        h.read::<u32>(s.wrapping_add(8)) == 1
                    })
                {
                    let clip = CLIP_CURSOR.get(h);
                    h.call(clip, Conv::Stdcall, &[at(0x11c)]);
                }
                if h.read::<u8>(at(0x130)) != 1 {
                    h.write(at(0x130), 1u8);
                    h.call(show_cursor, Conv::Stdcall, &[1]);
                }
                h.call_sym(UNPAUSE, me, &[]);
                h.write(at(0xe4), u32::MAX);
                let name = h.rebase(ON_ESC_CLOSE);
                as3(h, this, name);
                h.call_sym(SET_ENABLE, me, &[0]);
                click(h, this, 4);
                let s: u32 = h.read(settings);
                h.call_sym(SETTINGS_SAVE, Conv::Thiscall(s), &[]);
                save_busy(h, this, 1);
            }
        }
    }
    let wait: i32 = h.read(at(0xe4));
    if wait >= 0 {
        let wait = wait.wrapping_add(1);
        h.write(at(0xe4), wait);
        if wait > 1 {
            h.call_sym(PAUSE, me, &[]);
            h.write(at(0xe4), u32::MAX);
            h.call_sym(REFRESH, me, &[]);
            let name = h.rebase(SHOW_ESC_MENU);
            as3(h, this, name);
            h.call_sym(SET_ENABLE, me, &[1]);
            click(h, this, 1);
            save_busy(h, this, 0);
        }
    }
    if h.read::<u8>(at(0x70)) != 0 {
        let key_up = h.rebase(KEY_UP);
        let keys = h.rebase(MENU_KEYS);
        for k in 0..6u32 {
            let key = keys + 12 * k;
            let button: u32 = h.read(key + 4);
            let mut hit = false;
            if h.read::<u8>(key) != 0 {
                hit = h.call_sym(IS_RELEASE, Conv::Cdecl, &[button]) != 0;
            }
            if !hit && h.read::<u8>(key) == 0 {
                let button: u32 = h.read(key + 4);
                hit = h.call_sym(IS_PRESS, Conv::Cdecl, &[button]) != 0;
            }
            if hit {
                as3(h, this, key_up);
                let command: u32 = h.read(key + 8);
                as3(h, this, command);
                break;
            }
            let button: u32 = h.read(key + 4);
            if h.call_sym(IS_RELEASE, Conv::Cdecl, &[button]) != 0 {
                as3(h, this, key_up);
            }
        }
        let stick = h.call_sym(STICK_INFO, Conv::Cdecl, &[]);
        let directions = h.rebase(STICK_DIRECTIONS);
        let mut best = 0u32;
        let mut which = u32::MAX;
        for d in 0..4u32 {
            let v = stick_value(h, directions + 16 * d, stick);
            // taken when strictly greater and ordered
            if load(v) > load(best) {
                best = v;
                which = d;
            }
        }
        let threshold = load(h.read(h.rebase(STICK_THRESHOLD)));
        let current: u32 = h.read(at(0x12c));
        if load(best) > threshold {
            let direction = directions.wrapping_add(which.wrapping_mul(16));
            let key: u32 = h.read(direction);
            if current != key {
                as3(h, this, key_up);
                let command: u32 = h.read(direction + 0xc);
                as3(h, this, command);
                let key: u32 = h.read(direction);
                h.write(at(0x12c), key);
            }
        } else if current != 0 {
            as3(h, this, key_up);
            h.write(at(0x12c), 0u32);
        }
        let want = u8::from(h.read::<u8>(at(0x94)) == 0);
        if h.read::<u8>(at(0x130)) != want {
            h.write(at(0x130), want);
            h.call(show_cursor, Conv::Stdcall, &[u32::from(want)]);
        }
    }
    h.write(at(0x117), 0u8);
    let open = h.read::<u8>(at(0x70)) != 0;
    if open {
        if h.read::<u8>(at(0xe8)) == 0 {
            let t = h.call_sym_u64(TIME_MICRO, Conv::Cdecl, &[]);
            h.write(at(0xf0), t);
        }
    } else if h.read::<u8>(at(0xe8)) != 0 {
        let t = h.call_sym_u64(TIME_MICRO, Conv::Cdecl, &[]);
        let since: u64 = h.read(at(0xf0));
        let closed = h.rebase(CLOSED_TIME);
        h.write(closed, t.wrapping_sub(since));
    }
    let state: u8 = h.read(at(0x70));
    h.write(at(0xe8), state);
    let countdown: i32 = h.read(at(0x234));
    if countdown >= 0 {
        let countdown = countdown.wrapping_sub(1);
        h.write(at(0x234), countdown);
        if countdown == 0 {
            h.call_sym(DATA_VALUE_SET, Conv::Cdecl, &[frame, at(0x131)]);
            let menu: u32 = h.read(h.rebase(ESC_MENU));
            let name = h.rebase(SET_HELP_TEXT);
            let v: [u32; 4] = [0, 4, 8, 12].map(|k| h.read(frame + k));
            h.call_sym(
                CALL_FLASH,
                Conv::Cdecl,
                &[menu, name, v[0], v[1], v[2], v[3]],
            );
        }
    }
    Some(h.call_sym(FLUSH, me, &[]))
}
