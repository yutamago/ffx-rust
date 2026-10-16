//! The message and voice natives in Rust: setting a window's text,
//! position and inline values, closing it, the macro dictionary's texts,
//! and the voice waits (`0x00d8`, `0x00d9`), with `Msg_Text`.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Global, Host};

use super::native::{DEBUG_MODE, Native};
use super::run::NativeHost;
use super::{CONTEXT, atel_get_mes_win_work, atel_pop_stack_integer};

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

fn call(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, args)
}

/// The `printf` import (`0x00b0c380`) the debug lines go through.
pub const PRINTF_IAT: Global<u32> = Global::new(0x00b0_c380);

fn printf(h: &mut impl Host, fmt: u32, args: &[u32]) {
    let p = PRINTF_IAT.get(h);
    let mut v = vec![h.rebase(fmt)];
    v.extend_from_slice(args);
    h.call(p, &v);
}

/// `g_MsgLastIndex` (`0x01325b70`): the last message index looked up.
pub const MSG_LAST_INDEX: Global<u32> = Global::new(0x0132_5b70);
/// `g_MsgNone` (`0x01325b74`): what a lookup without a table returns.
pub const MSG_NONE: Global<u8> = Global::new(0x0132_5b74);
/// `g_EventEbpMessages` (`0x023cbd60`): the `.ebp`'s own message table.
pub const EBP_MESSAGES: Global<u32> = Global::new(0x023c_bd60);
/// `g_AtelLastSetMsg` (`0x01325ac0`): the message `Native_MsgSetText` last
/// put in a window.
pub const LAST_SET_MSG: Global<u32> = Global::new(0x0132_5ac0);
/// `g_CurrentMapNo` (`0x012fbc40`).
pub const CURRENT_MAP_NO: Global<u32> = Global::new(0x012f_bc40);

/// `Msg_Text(i)` (`0x0086bf30`): `0x008af6a0(i)`, records `i`, then entry
/// `i`'s text in the event's message table (`context + 0x34`; a zero
/// offset falls back to the `.ebp`'s table), or [`MSG_NONE`] without one.
pub fn msg_text(h: &mut impl Host, i: u32) -> u32 {
    call(h, 0x008a_f6a0, &[i]);
    MSG_LAST_INDEX.set(h, i);
    let ctx = CONTEXT.get(h);
    let mut t = h.read::<u32>(ctx + 0x34);
    if t == 0 {
        return MSG_NONE.ptr(h).addr;
    }
    let mut off = h.read::<u16>(t.wrapping_add(i.wrapping_mul(8)));
    if off == 0 {
        t = EBP_MESSAGES.get(h);
        off = h.read::<u16>(t.wrapping_add(i.wrapping_mul(8)));
    }
    t.wrapping_add(u32::from(off))
}

/// `Native_MsgSetText` (`0x00857870`, `0x0064`): `(window, msg)`: the
/// window's text (`+8`, `+0xc`) and flags (`+0x16`, `Msg_Flags`); 0.
pub fn native_msg_set_text<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let m = pop(h, s);
    let win = pop(h, s);
    LAST_SET_MSG.set(h, m);
    let o = atel_get_mes_win_work(h, win.cast_signed());
    let t = msg_text(h, m);
    h.write(o + 8, t);
    h.write(o + 0xc, t);
    let x = call(h, 0x0086_bbc0, &[]);
    let f = call(h, 0x0086_bf10, &[m, x]);
    #[allow(clippy::cast_possible_truncation)] // the original stores AX
    h.write(o + 0x16, f as u16);
    0
}

/// `Msg_CloseWindow` (`0x008640f0`, the menu module's).
pub const MSG_CLOSE_WINDOW: u32 = 0x0086_40f0;

/// `Native_MsgClose` (`0x00859060`, `0x006b`): `(window)`:
/// `Msg_CloseWindow(window)`; the window.
pub fn native_msg_close<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, MSG_CLOSE_WINDOW, &[v]);
    v
}

/// `Native_MsgSetPos` (`0x008580c0`, `0x0065`): `(window, x, y, anchor)`:
/// `+0` x, `+2` y, `+0x1c` the anchor byte; three windows of three maps
/// are moved (map 0x17 (0x1e6, 0x104) to (0x1c7, 0xc7); map 0xab
/// (0x100, 0x14f) to y 0x140 when `0x008ac2f0()` is 2 or 3; map 0xd4
/// (0x16, 0xdc) to y 0x82 when it is 4); 0.
pub fn native_msg_set_pos<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let anchor = pop(h, s);
    let mut y = pop(h, s);
    let mut x = pop(h, s);
    let win = pop(h, s);
    let map = CURRENT_MAP_NO.get(h);
    if map == 0x17 {
        if x == 0x1e6 && y == 0x104 {
            y = 0xc7;
            x = 0x1c7;
        }
    } else {
        if map == 0xab && x == 0x100 && y == 0x14f {
            let v = call(h, 0x008a_c2f0, &[]);
            if v == 2 || call(h, 0x008a_c2f0, &[]) == 3 {
                y = 0x140;
            }
        }
        if CURRENT_MAP_NO.get(h) == 0xd4 && x == 0x16 && y == 0xdc && call(h, 0x008a_c2f0, &[]) == 4
        {
            y = x + 0x6c;
        }
    }
    let o = atel_get_mes_win_work(h, win.cast_signed());
    #[allow(clippy::cast_possible_truncation)] // the original stores SI, BX, CL
    {
        h.write(o, x as u16);
        h.write(o + 2, y as u16);
        h.write(o + 0x1c, anchor as u8);
    }
    0
}

/// `Native_MsgSetValue` (`0x00855c70`, `0x009e`): `(window, slot, type,
/// value)`: `TOMacroSetType(window, slot, type)`,
/// `TOMacroSetValue(window, slot, value)`; the window.
pub fn native_msg_set_value<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let t = pop(h, s);
    let n = pop(h, s);
    let win = pop(h, s);
    call(h, 0x008b_8960, &[win, n, t]);
    call(h, 0x008b_8980, &[win, n, v]);
    win
}

/// `Native_MacroDicText` (`0x00856a00`, `0x013f`): `(dict, entry)`: in
/// `MacroDic_Section(dict, 0x0086bbc0())`, the u16 offset at index
/// `entry * 2 + (0x0086bad0() != 0)`, added to the section.
pub fn native_macro_dic_text<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let e = pop(h, s);
    let d = pop(h, s);
    let x = call(h, 0x0086_bbc0, &[]);
    let t = call(h, 0x008b_70a0, &[d, x]);
    let mut i = e.wrapping_add(e);
    if call(h, 0x0086_bad0, &[]) != 0 {
        i = i.wrapping_add(1);
    }
    t.wrapping_add(u32::from(h.read::<u16>(t.wrapping_add(i.wrapping_mul(2)))))
}

/// `Native_BitIndex` (`0x008578c0`, `0x0142`): `(mask, n)`: the index of
/// the `n`th set bit of `mask` (from bit 0), 0 if there are fewer.
pub fn native_bit_index<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    let mask = pop(h, s);
    let mut k = 0;
    for b in 0..32 {
        if mask & (1 << b) != 0 {
            if k == n {
                return b;
            }
            k += 1;
        }
    }
    0
}

/// `SndGetStreamState` (`0x008864b0`).
pub const SND_GET_STREAM_STATE: u32 = 0x0088_64b0;

/// The countdowns and debug counters of the two voice waits:
/// `(debug counter, flag, countdown)`.
struct VoiceWait {
    count: Global<u32>,
    flag: Global<u32>,
    left: Global<i32>,
}

/// `Native_VoiceIsPlaying`'s.
const VOICE_PLAYING: VoiceWait = VoiceWait {
    count: Global::new(0x0132_5acc),
    flag: Global::new(0x0132_5ae4),
    left: Global::new(0x0132_5ae0),
};
/// `Native_SoundWait_Poll`'s.
const SOUND_WAIT: VoiceWait = VoiceWait {
    count: Global::new(0x0132_5afc),
    flag: Global::new(0x0132_5aec),
    left: Global::new(0x0132_5ae8),
};

/// The debug prologue both waits share: the first call prints `fmt`,
/// every call in debug mode counts.
fn voice_debug(h: &mut impl Host, v: &VoiceWait, fmt: u32) {
    if DEBUG_MODE.get(h) != 1 {
        return;
    }
    if v.count.get(h) == 0 {
        printf(h, fmt, &[]);
        if DEBUG_MODE.get(h) != 1 {
            return;
        }
    }
    v.count.set(h, v.count.get(h).wrapping_add(1));
}

/// Whether the voice still plays: with the flag set, the countdown (1 and
/// down by one while positive, else 0 and both cleared); else whether
/// `SndGetStreamState()` is nonzero.
fn voice_playing(h: &mut impl Host, v: &VoiceWait) -> bool {
    if v.flag.get(h) == 1 {
        let c = v.left.get(h);
        if c > 0 {
            v.left.set(h, c - 1);
            true
        } else {
            v.left.set(h, 0);
            v.flag.set(h, 0);
            false
        }
    } else {
        call(h, SND_GET_STREAM_STATE, &[]) != 0
    }
}

/// `Native_VoiceIsPlaying` (`0x0085e1d0`, `0x00d8`): 1 while the voice
/// plays, else 0; in debug mode counts calls and prints the count when done.
pub fn native_voice_is_playing<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    voice_debug(h, &VOICE_PLAYING, 0x00b5_bb00);
    let r = u32::from(voice_playing(h, &VOICE_PLAYING));
    if DEBUG_MODE.get(h) == 1 && r == 0 {
        let n = VOICE_PLAYING.count.get(h);
        printf(h, 0x00b5_bb14, &[n]);
        VOICE_PLAYING.count.set(h, r);
    }
    r
}

/// `Native_SoundWait_Poll` (`0x0085e4a0`, `0x00d9`): 1 once the voice has
/// stopped, else 0; the same debug counting.
pub fn native_sound_wait_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    voice_debug(h, &SOUND_WAIT, 0x00b5_bb34);
    let r = u32::from(!voice_playing(h, &SOUND_WAIT));
    if DEBUG_MODE.get(h) == 1 && r == 1 {
        let n = SOUND_WAIT.count.get(h);
        printf(h, 0x00b5_bb48, &[n]);
        SOUND_WAIT.count.set(h, 0);
    }
    r
}

/// The Rust version of a message or voice handler, if there is one.
#[must_use]
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    Some(Native::Int(match sym {
        0x0085_5c70 => native_msg_set_value,
        0x0085_6a00 => native_macro_dic_text,
        0x0085_7870 => native_msg_set_text,
        0x0085_78c0 => native_bit_index,
        0x0085_80c0 => native_msg_set_pos,
        0x0085_9060 => native_msg_close,
        0x0085_e1d0 => native_voice_is_playing,
        0x0085_e4a0 => native_sound_wait_poll,
        _ => return None,
    }))
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 8] = [
    (0x0085_5c70, false),
    (0x0085_6a00, false),
    (0x0085_7870, false),
    (0x0085_78c0, false),
    (0x0085_80c0, false),
    (0x0085_9060, false),
    (0x0085_e1d0, false),
    (0x0085_e4a0, false),
];
