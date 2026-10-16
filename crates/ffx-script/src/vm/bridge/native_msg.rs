//! The message and voice natives in Rust: setting a window's text,
//! position and inline values, closing it, the macro dictionary's texts,
//! and the voice waits (`0x00d8`, `0x00d9`), with `Msg_Text`.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Global, Host};

use super::native::{DEBUG_MODE, Native};
use super::run::NativeHost;
use super::{CONTEXT, atel_get_mes_win_work, atel_pop_stack_integer, atel_push_stack_integer};

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

fn push(h: &mut impl NativeHost, s: u32, v: u32) {
    atel_push_stack_integer(h, s, v);
}

/// `TOGetMessageStatus(window)` (`0x008b7ca0`): 3 once the text is shown.
const TO_GET_MESSAGE_STATUS: u32 = 0x008b_7ca0;

/// `Native_MsgOpen` (`0x00858b50`, `0x006a`): `(window, mode)`: the
/// window opens (state 1) with the choice flags `Msg_ClampChoices(window,
/// mode)` gives (`+0x1d`), its colours, style, flags, text and position
/// handed to the text box (`0x008ada30`, `TOSetWindowRGBA`, `0x008b0000`,
/// `0x008af730` with flag `0x40`, `0x008b89d0`, `0x008afa90` with flag 8
/// else `0x008af850`, `0x008afb90`, `0x008af7d0`, the choice cursor
/// `0x008b0020` with flag 2 and not `0x40`, `0x008ade10`); the window.
pub fn native_msg_open<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let mode = pop(h, s);
    let win = pop(h, s);
    let m = atel_get_mes_win_work(h, win.cast_signed());
    h.write(m + 0x14, 1_u16);
    let r = call(h, 0x0087_1210, &[m, mode]);
    let b11 = u32::from(h.read::<u8>(m + 0x11));
    let b10 = u32::from(h.read::<u8>(m + 0x10));
    #[allow(clippy::cast_possible_truncation)] // the original stores AL
    h.write(m + 0x1d, r as u8);
    let b12 = u32::from(h.read::<u8>(m + 0x12));
    h.write(m + 0x21, 1_u8);
    let b13 = u32::from(h.read::<u8>(m + 0x13));
    call(h, 0x008a_da30, &[win]);
    call(h, 0x0090_6d90, &[win, b12, b11, b10, b13]);
    let f = h.read::<u32>(m + 0x28) | h.read::<u32>(m + 0x18);
    let st = i32::from(h.read::<i8>(m + 0x20)).cast_unsigned();
    call(h, 0x008b_0000, &[win, st, f]);
    if r & 0x40 != 0 {
        let x = h.read::<u32>(m + 0x24);
        let c = u32::from(h.read::<u8>(m + 0x23));
        let b = u32::from(h.read::<u8>(m + 0x22));
        call(h, 0x008a_f730, &[win, b, c, x]);
    }
    let (t8, tc) = (h.read::<u32>(m + 8), h.read::<u32>(m + 0xc));
    call(h, 0x008b_89d0, &[win, t8, tc]);
    let sx = |h: &mut H, o: u32| i32::from(h.read::<i16>(m + o)).cast_unsigned();
    if r & 8 != 0 {
        let (a6, a4, a2, a0) = (sx(h, 6), sx(h, 4), sx(h, 2), sx(h, 0));
        let anchor = u32::from(h.read::<u8>(m + 0x1c));
        call(h, 0x008a_fa90, &[win, anchor, a0, a2, a4, a6]);
    } else {
        let (a2, a0) = (sx(h, 2), sx(h, 0));
        let anchor = u32::from(h.read::<u8>(m + 0x1c));
        call(h, 0x008a_f850, &[win, anchor, a0, a2]);
    }
    call(h, 0x008a_fb90, &[win, win]);
    call(h, 0x008a_f7d0, &[win]);
    if r & 0x40 == 0 && r & 2 != 0 {
        let sb = |h: &mut H, o: u32| i32::from(h.read::<i8>(m + o)).cast_unsigned();
        let (c21, c1f, c1e) = (sb(h, 0x21), sb(h, 0x1f), sb(h, 0x1e));
        call(h, 0x008b_0020, &[win, c1e, c1f, c21]);
    }
    call(h, 0x008a_de10, &[win]);
    win
}

/// The start both message waits share: `(window, x)`: the window clamped
/// to 0..=7 into the args word, `x` into `+4`, the window's `+0x1d` |=
/// `0x10`; `EAX` the window's address.
fn wait_start<H: NativeHost>(h: &mut H, a: u32, s: u32) -> u32 {
    let x = pop(h, s);
    let win = pop(h, s).cast_signed().clamp(0, 7).cast_unsigned();
    h.write(a, win);
    h.write(a + 4, x);
    let m = atel_get_mes_win_work(h, win.cast_signed());
    let f = h.read::<u8>(m + 0x1d);
    h.write(m + 0x1d, f | 0x10);
    m
}

/// `Native_MsgWaitEx` (`0x0085abd0`, `0x0084`, start).
pub fn native_msg_wait_ex<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    wait_start(h, a, s)
}

/// `Native_MsgWait` (`0x0085b7c0`, `0x007c`, start): pushes 0 for `x`,
/// then as [`native_msg_wait_ex`].
pub fn native_msg_wait<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    push(h, s, 0);
    wait_start(h, a, s)
}

/// `Native_MsgWaitEx_Poll` (`0x0085ae90`, also `Native_MsgWait_Poll`
/// `0x0085b980`): 1 once the window is closed (0) or in state 4; open (2)
/// with its text shown and the current ctrl work's byte 0 bit 2 set, the
/// window is closed and the bit cleared; else 0.
pub fn native_msg_wait_ex_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let win = h.read::<u32>(a);
    let m = atel_get_mes_win_work(h, win.cast_signed());
    match h.read::<u16>(m + 0x14) {
        0 | 4 => 1,
        2 => {
            if call(h, TO_GET_MESSAGE_STATUS, &[win]) == 3 {
                let c = call(h, 0x0086_aff0, &[]);
                let b = h.read::<u8>(c);
                if b & 4 != 0 {
                    call(h, MSG_CLOSE_WINDOW, &[win]);
                    let b = h.read::<u8>(c);
                    h.write(c, b & 0xfb);
                }
            }
            0
        }
        _ => 0,
    }
}

/// `Native_MsgWaitEx_End` (`0x0085b6d0`, int; also `Native_MsgWait_End`
/// `0x0085ba80`): pushes the saved window, pops it again and closes it; 0.
pub fn native_msg_wait_ex_end<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = h.read::<u32>(a);
    push(h, s, v);
    let v = pop(h, s);
    call(h, MSG_CLOSE_WINDOW, &[v]);
    0
}

/// `Native_MsgSelect` (`0x00859710`, `0x007d`, start): `(window[, a[,
/// b]])`: the window (clamped) into the args word, up to two more words
/// from the stack (as many as it holds) into `+8`, their count in `+6`;
/// the window's `+0x1d` |= `0x20`; `EAX` its address.
pub fn native_msg_select<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let win = pop(h, s).cast_signed().clamp(0, 7);
    h.write(a + 6, 0_u16);
    h.write(a, win.cast_unsigned());
    let d = h.read::<i32>(s);
    if d != 0 {
        let n = d.min(2);
        for i in 0..n.max(0).cast_unsigned() {
            let v = pop(h, s);
            h.write(a + 8 + i * 4, v);
            let c = h.read::<u16>(a + 6);
            h.write(a + 6, c.wrapping_add(1));
        }
    }
    let m = atel_get_mes_win_work(h, win);
    let f = h.read::<u8>(m + 0x1d);
    h.write(m + 0x1d, f | 0x20);
    m
}

/// The chosen option: the first byte of the text box's choice list
/// (`0x008b7fd0`) when it has any (`0x008b7ff0`), else -1, into `+4`.
fn store_choice<H: NativeHost>(h: &mut H, a: u32, win: u32) {
    let p = call(h, 0x008b_7fd0, &[win]);
    let n = call(h, 0x008b_7ff0, &[win]);
    let v = if n.cast_signed() > 0 {
        u16::from(h.read::<u8>(p))
    } else {
        0xffff
    };
    h.write(a + 4, v);
}

/// `Native_MsgSelect_Poll` (`0x00859c50`, also `Native_MsgShowWait_Poll`
/// `0x008603d0`): closed (0) or state 4: the choice stored, 1; open (2)
/// with the text shown and the box's input done (`0x008b7fb0` 2): the
/// choice stored and the window closed, 0; else 0.
pub fn native_msg_select_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let win = h.read::<u32>(a);
    let m = atel_get_mes_win_work(h, win.cast_signed());
    match h.read::<u16>(m + 0x14) {
        0 | 4 => {
            store_choice(h, a, win);
            1
        }
        2 => {
            if call(h, TO_GET_MESSAGE_STATUS, &[win]) == 3 && call(h, 0x008b_7fb0, &[win]) == 2 {
                store_choice(h, a, win);
                call(h, MSG_CLOSE_WINDOW, &[win]);
            }
            0
        }
        _ => 0,
    }
}

/// `Native_MsgSelect_Result` (`0x0085a6f0`, int; also
/// `Native_MsgShowWait_Result` `0x00860760`): closes the window (pushed and
/// popped again), pushes the saved words back (the last first); the
/// choice (`+4`, sign-extended).
pub fn native_msg_select_result<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = h.read::<u32>(a);
    push(h, s, v);
    let v = pop(h, s);
    call(h, MSG_CLOSE_WINDOW, &[v]);
    let n = h.read::<u16>(a + 6);
    let r = i32::from(h.read::<i16>(a + 4)).cast_unsigned();
    for i in (1..=u32::from(n)).rev() {
        let v = h.read::<u32>(a + 4 + i * 4);
        push(h, s, v);
    }
    r
}

/// `Msg_ShowSetup(worker, args, stack, mode)` (`0x008607a0`): `(window,
/// message, choice a, choice b, x, y, anchor)`: re-pushes them for
/// `Native_MsgSetPos`, sets the style (`+0x20`) to 0, the text as
/// `Native_MsgSetText` does, the flags (`+0x18`) to 0, the choice cursor
/// bytes (`+0x1e` b... `+0x1f` a), opens the window with `mode | 2` and
/// starts [`native_msg_select`] on it; its `EAX`.
pub fn msg_show_setup<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32, mode: u32) -> u32 {
    let p1 = pop(h, s);
    let p2 = pop(h, s);
    let p3 = pop(h, s);
    let p4 = pop(h, s);
    let p5 = pop(h, s);
    let p6 = pop(h, s);
    let win = pop(h, s);
    push(h, s, win);
    push(h, s, p3);
    push(h, s, p2);
    push(h, s, p1);
    native_msg_set_pos(h, w, a, s);
    push(h, s, win);
    push(h, s, 0);
    let style = pop(h, s);
    let v = pop(h, s);
    let m = atel_get_mes_win_work(h, v.cast_signed());
    #[allow(clippy::cast_possible_truncation)] // the original stores BL
    h.write(m + 0x20, style as u8);
    push(h, s, win);
    push(h, s, p6);
    native_msg_set_text(h, w, a, s);
    push(h, s, win);
    push(h, s, 0);
    let fl = pop(h, s);
    let v = pop(h, s);
    let m = atel_get_mes_win_work(h, v.cast_signed());
    h.write(m + 0x18, fl);
    push(h, s, win);
    push(h, s, p5);
    push(h, s, p4);
    let c4 = pop(h, s);
    let c5 = pop(h, s);
    let v = pop(h, s);
    let m = atel_get_mes_win_work(h, v.cast_signed());
    #[allow(clippy::cast_possible_truncation)] // the original stores BL, CL
    {
        h.write(m + 0x1e, c5 as u8);
        h.write(m + 0x1f, c4 as u8);
    }
    push(h, s, win);
    push(h, s, mode | 2);
    native_msg_open(h, w, a, s);
    push(h, s, win);
    native_msg_select(h, w, a, s)
}

/// `Native_MsgShowWait` (`0x008602a0`, `0x013b`, start):
/// [`msg_show_setup`] with mode 0.
pub fn native_msg_show_wait<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    msg_show_setup(h, w, a, s, 0)
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
        0x0085_8b50 => native_msg_open,
        0x0085_9060 => native_msg_close,
        0x0085_9710 => native_msg_select,
        0x0085_9c50 | 0x0086_03d0 => native_msg_select_poll,
        0x0085_a6f0 | 0x0086_0760 => native_msg_select_result,
        0x0085_abd0 => native_msg_wait_ex,
        0x0085_ae90 | 0x0085_b980 => native_msg_wait_ex_poll,
        0x0085_b6d0 | 0x0085_ba80 => native_msg_wait_ex_end,
        0x0085_b7c0 => native_msg_wait,
        0x0085_e1d0 => native_voice_is_playing,
        0x0085_e4a0 => native_sound_wait_poll,
        0x0086_02a0 => native_msg_show_wait,
        _ => return super::native_move::find(sym),
    }))
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 21] = [
    (0x0085_5c70, false),
    (0x0085_6a00, false),
    (0x0085_7870, false),
    (0x0085_78c0, false),
    (0x0085_80c0, false),
    (0x0085_8b50, false),
    (0x0085_9060, false),
    (0x0085_9710, false),
    (0x0085_9c50, false),
    (0x0086_03d0, false),
    (0x0085_a6f0, false),
    (0x0086_0760, false),
    (0x0085_abd0, false),
    (0x0085_ae90, false),
    (0x0085_b980, false),
    (0x0085_b6d0, false),
    (0x0085_ba80, false),
    (0x0085_b7c0, false),
    (0x0085_e1d0, false),
    (0x0085_e4a0, false),
    (0x0086_02a0, false),
];
