//! Continuation round 2 of list `h4-glue-other`, batch 5: the message
//! windows' draw (one window's text, the field's and the menu's window
//! passes) and an Iggy menu's mouse handler.
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

// ---- one message window

/// `g_MsgWindows` (eight window pointers).
pub const MSG_WINDOWS: u32 = 0x0186_76f0;
/// `g_DebugPadPrintFlag` (a byte).
pub const DEBUG_PAD_PRINT: Global<u8> = Global::new(0x0184_1d43);
/// `TODapsSetPos(x, y)` (cdecl).
pub const DAPS_SET_POS: u32 = 0x0089_de30;
/// `TODapsPrintf(fmt, ...)` (cdecl).
pub const DAPS_PRINTF: u32 = 0x0089_dde0;
/// The debug lines (`0x00b65598`, `0x00b655b4` with the window).
pub const DEBUG_LINES: [u32; 2] = [0x00b6_5598, 0x00b6_55b4];
/// `TOGetWindowStatus(i)` (cdecl).
pub const WINDOW_STATUS: u32 = 0x0090_6c20;
/// `TODrawSelectCursor(i)` (cdecl).
pub const DRAW_SELECT_CURSOR: u32 = 0x008b_5fb0;
/// `TOGetMessageWindowMode()` (cdecl).
pub const WINDOW_MODE: u32 = 0x008a_c7b0;
/// `TODrawMovieMessageWaitNamesp(window)` (cdecl).
pub const MOVIE_MESSAGE_WAIT: u32 = 0x008d_fea0;

fn window(h: &impl Host, i: u32) -> u32 {
    h.read(h.rebase(MSG_WINDOWS).wrapping_add(i.wrapping_mul(4)))
}

/// The selection cursor of window `i` when its cursor mode (`+0x1e`) is
/// 1, its status 2 and its cursor kind (`+0x1c`) `kind`. Returns what the
/// original leaves in `eax`: the window, the status, or the cursor
/// draw's result.
fn select_cursor(h: &mut impl Host, i: u32, kind: u16) -> u32 {
    let w = window(h, i);
    if h.read::<u16>(w.wrapping_add(0x1e)) != 1 {
        return w;
    }
    let status = h.call_sym(WINDOW_STATUS, Conv::Cdecl, &[i]);
    if status != 2 {
        return status;
    }
    let w = window(h, i);
    if h.read::<u16>(w.wrapping_add(0x1c)) != kind {
        return w;
    }
    h.call_sym(DRAW_SELECT_CURSOR, Conv::Cdecl, &[i])
}

/// `TODrawMessage(i)` (cdecl): window `i` (`g_MsgWindows[i]`), when open
/// (u16 `+0`): a debug window (u16 `+2` == 1) prints two debug lines when
/// the pad-print flag is set; another draws the cursor of kind 2 first,
/// then its text (the movie wait in message-window mode 1, else its draw
/// routine `+0x12c(window)`), then the cursor of kind 1. Returns what the
/// original leaves in `eax`.
pub fn draw_message(h: &mut impl Host, i: u32) -> u32 {
    let w = window(h, i);
    if h.read::<u16>(w) == 0 {
        return w;
    }
    if h.read::<u16>(w.wrapping_add(2)) == 1 {
        if DEBUG_PAD_PRINT.get(h) == 0 {
            return w;
        }
        h.call_sym(DAPS_SET_POS, Conv::Cdecl, &[0x69, 0x172]);
        let line = h.rebase(DEBUG_LINES[0]);
        h.call_sym(DAPS_PRINTF, Conv::Cdecl, &[line]);
        let line = h.rebase(DEBUG_LINES[1]);
        return h.call_sym(DAPS_PRINTF, Conv::Cdecl, &[line, i]);
    }
    select_cursor(h, i, 2);
    if h.call_sym(WINDOW_MODE, Conv::Cdecl, &[]) == 1 {
        let w = window(h, i);
        h.call_sym(MOVIE_MESSAGE_WAIT, Conv::Cdecl, &[w]);
    } else {
        let w = window(h, i);
        let f: u32 = h.read(w.wrapping_add(0x12c));
        h.call(f, Conv::Cdecl, &[w]);
    }
    select_cursor(h, i, 1)
}

// ---- the window passes

/// `g_TOMessageWindowSlots` (eight slot pointers; the slot's state
/// `+0x14`).
pub const WINDOW_SLOTS: u32 = 0x0186_5b3c;
/// `MsGetSaveMenu()` (cdecl).
pub const SAVE_MENU: u32 = 0x0078_5290;
/// The hide switches: `0x00c59558` (1: never hide) and `0x01841e0c`.
pub const HIDE_SWITCHES: [u32; 2] = [0x00c5_9558, 0x0184_1e0c];
/// `SaveDataIsBusy()` (cdecl).
pub const SAVE_BUSY: u32 = 0x0064_8040;
/// `TODrawWindow(i)` (cdecl).
pub const DRAW_WINDOW: u32 = 0x0090_6bf0;
/// `TODrawMessage`.
pub const DRAW_MESSAGE: u32 = 0x008b_5dc0;
/// `TOSortMessageWindowPriority(&order)` (cdecl).
pub const SORT_WINDOWS: u32 = 0x008b_00a0;
/// `TOMakePktScissor(x, y, w, h)` (cdecl).
pub const PKT_SCISSOR: u32 = 0x008f_dee0;
/// `g_TOPadGetCtrl` (a byte).
pub const PAD_GET_CTRL: Global<u8> = Global::new(0x025d_09c0);
/// Bytes of the passes' frames (the sort order, the cookie).
pub const PASS_FRAME: u32 = 0x14;

/// Whether window slot `i` is hidden this frame: outside the save menu
/// (`*MsGetSaveMenu()` without bit 0x200), a slot in state 2 of kind 1
/// or 3 (u16 `+0x1a`) that is not a `0x4000` window (u16 `+0x10`) and
/// has no text (u16 `+0xc`), with neither hide switch set.
fn hidden(h: &mut impl Host, i: u32) -> bool {
    let menu = h.call_sym(SAVE_MENU, Conv::Cdecl, &[]);
    if h.read::<u32>(menu) & 0x200 != 0 {
        return false;
    }
    let slot: u32 = h.read(h.rebase(WINDOW_SLOTS).wrapping_add(4 * i));
    let s: u32 = h.read(slot.wrapping_add(0x14));
    if h.read::<u32>(s.wrapping_add(8)) != 2 {
        return false;
    }
    let kind: u16 = h.read(s.wrapping_add(0x1a));
    if kind != 1 && kind != 3 {
        return false;
    }
    if h.read::<u16>(s.wrapping_add(0x10)) == 0x4000 || h.read::<u16>(s.wrapping_add(0xc)) != 0 {
        return false;
    }
    let [never, other] = HIDE_SWITCHES.map(Global::<u32>::new);
    never.get(h) != 1 && other.get(h) == 0
}

/// The eight windows drawn (`TODrawWindow(i)`, `TODrawMessage(i)`) unless
/// hidden, and in the field pass not while a save is busy.
fn draw_windows(h: &mut impl Host, check_busy: bool) {
    for i in 0..8u32 {
        if hidden(h, i) {
            continue;
        }
        if check_busy && h.call_sym(SAVE_BUSY, Conv::Cdecl, &[]) != 0 {
            continue;
        }
        h.call_sym(DRAW_WINDOW, Conv::Cdecl, &[i]);
        h.call_sym(DRAW_MESSAGE, Conv::Cdecl, &[i]);
    }
}

/// `TOInitAddPkt()` (cdecl).
pub const INIT_ADD_PKT: u32 = 0x0090_4650;
/// `TOTransTexture()` (cdecl).
pub const TRANS_TEXTURE: u32 = 0x008b_1020;
/// The screen fade's alpha (`0x00c59f38`).
pub const ALPHA_BG: Global<u32> = Global::new(0x00c5_9f38);
/// `TODrawAlphaBG(alpha)` (cdecl).
pub const DRAW_ALPHA_BG: u32 = 0x008f_3a50;
/// `TODrawBtlWindow()` (cdecl).
pub const DRAW_BTL_WINDOW: u32 = 0x0089_b360;
/// `TOSetAddPkt()` (cdecl).
pub const SET_ADD_PKT: u32 = 0x008f_ae10;
/// `TODapsDraw()` (cdecl).
pub const DAPS_DRAW: u32 = 0x0089_dc10;

/// `TODrawMessageWindow()` (cdecl; `frame` its sort order at `ebp -
/// 0x14`): the packet begun, textures sent, windows sorted, the fade
/// (`0x00c59f38`, when positive) and the battle window drawn under a
/// 512 x 416 scissor, the eight windows ([`draw_windows`], skipped while a
/// save is busy), the scissor again, the pad-control byte set, the packet
/// and the debug text sent. Returns `TODapsDraw`'s `eax`.
pub fn draw_message_window(h: &mut impl Host, frame: u32) -> u32 {
    h.call_sym(INIT_ADD_PKT, Conv::Cdecl, &[]);
    h.call_sym(TRANS_TEXTURE, Conv::Cdecl, &[]);
    h.call_sym(SORT_WINDOWS, Conv::Cdecl, &[frame]);
    let alpha = ALPHA_BG.get(h);
    if alpha.cast_signed() > 0 {
        h.call_sym(DRAW_ALPHA_BG, Conv::Cdecl, &[alpha]);
    }
    h.call_sym(PKT_SCISSOR, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    h.call_sym(DRAW_BTL_WINDOW, Conv::Cdecl, &[]);
    draw_windows(h, true);
    h.call_sym(PKT_SCISSOR, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    PAD_GET_CTRL.set(h, 1);
    h.call_sym(SET_ADD_PKT, Conv::Cdecl, &[]);
    h.call_sym(DAPS_DRAW, Conv::Cdecl, &[])
}

/// `TOSetPktBuffAdrs(a, b, c)` (cdecl).
pub const SET_PKT_BUFF: u32 = 0x0090_47c0;
/// `TOOpenPktbuff()` (cdecl).
pub const OPEN_PKT_BUFF: u32 = 0x0090_4600;
/// `TOSetRegFont1612()` (cdecl).
pub const SET_REG_FONT: u32 = 0x0090_48e0;
/// `TOClosePktbuff()` (cdecl).
pub const CLOSE_PKT_BUFF: u32 = 0x008f_3930;
/// `TODrawKick()` (cdecl).
pub const DRAW_KICK: u32 = 0x008f_5bc0;

/// `TODrawMessageWindowForMenu()` (cdecl; `frame` its sort order): the
/// menu's packet buffer (`TOSetPktBuffAdrs(0x1f00000, 0x100000,
/// 0xf0000)`) opened, windows sorted, a 512 x 416 scissor, the 16 x 12
/// font, the eight windows ([`draw_windows`]), the scissor again, the
/// buffer closed and kicked, the pad-control byte set. Returns
/// `TODrawKick`'s `eax`.
pub fn draw_message_window_for_menu(h: &mut impl Host, frame: u32) -> u32 {
    h.call_sym(
        SET_PKT_BUFF,
        Conv::Cdecl,
        &[0x1f0_0000, 0x10_0000, 0xf_0000],
    );
    h.call_sym(OPEN_PKT_BUFF, Conv::Cdecl, &[]);
    h.call_sym(SORT_WINDOWS, Conv::Cdecl, &[frame]);
    h.call_sym(PKT_SCISSOR, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    h.call_sym(SET_REG_FONT, Conv::Cdecl, &[]);
    draw_windows(h, false);
    h.call_sym(PKT_SCISSOR, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    h.call_sym(CLOSE_PKT_BUFF, Conv::Cdecl, &[]);
    let eax = h.call_sym(DRAW_KICK, Conv::Cdecl, &[]);
    PAD_GET_CTRL.set(h, 1);
    eax
}

// ---- an Iggy menu's mouse

/// `_IggyMakeEventNone@4` (import slot `0x00b0c840`).
pub const MAKE_EVENT_NONE: Global<u32> = Global::new(0x00b0_c840);
/// `_IggyMakeEventMouseMove@12` (import slot `0x00b0c844`).
pub const MAKE_EVENT_MOUSE_MOVE: Global<u32> = Global::new(0x00b0_c844);
/// `_IggyPlayerDispatchEventRS@12` (import slot `0x00b0c85c`).
pub const DISPATCH_EVENT: Global<u32> = Global::new(0x00b0_c85c);
/// Bytes of [`menu_mouse_event`]'s frame (the Iggy event, the cookie).
pub const MOUSE_FRAME: u32 = 0x1c;

/// `IggyMenu_handleMouseEvent_00a28c10(menu; event, result)` (thiscall,
/// ret 8; `frame` its Iggy event at `ebp - 0x1c`): for an active menu
/// (byte `+0x70`), a mouse move (`*event == 7`) is made a move event
/// relative to the menu's origin (`+0x84`, `+0x88`; event x `+8`, y
/// `+0xc`), dispatched to the player (`*(+0x6c)`), and the hover byte
/// `+0x94` set to whether the point lies strictly inside the menu's
/// rectangle (size `+0x8c`, `+0x90`; signed); other events are
/// dispatched as they are. Returns the hover flag or the dispatch's
/// `eax` (0 when inactive, where the original leaves its cookie word).
pub fn menu_mouse_event(h: &mut impl Host, frame: u32, this: u32, event: u32, result: u32) -> u32 {
    if h.read::<u8>(this + 0x70) == 0 {
        return 0;
    }
    if h.read::<u32>(event) != 7 {
        let player: u32 = h.read(h.read::<u32>(this + 0x6c));
        let f = DISPATCH_EVENT.get(h);
        return h.call(f, Conv::Stdcall, &[player, event, result]);
    }
    let f = MAKE_EVENT_NONE.get(h);
    h.call(f, Conv::Stdcall, &[frame]);
    let x: u32 = h.read(event.wrapping_add(8));
    let y: u32 = h.read(event.wrapping_add(0xc));
    let left: u32 = h.read(this + 0x84);
    let top: u32 = h.read(this + 0x88);
    let f = MAKE_EVENT_MOUSE_MOVE.get(h);
    h.call(
        f,
        Conv::Stdcall,
        &[frame, x.wrapping_sub(left), y.wrapping_sub(top)],
    );
    let player: u32 = h.read(h.read::<u32>(this + 0x6c));
    let f = DISPATCH_EVENT.get(h);
    h.call(f, Conv::Stdcall, &[player, frame, result]);
    let x = h.read::<u32>(event.wrapping_add(8)).cast_signed();
    let left = h.read::<u32>(this + 0x84).cast_signed();
    let width = h.read::<u32>(this + 0x8c).cast_signed();
    let y = h.read::<u32>(event.wrapping_add(0xc)).cast_signed();
    let top = h.read::<u32>(this + 0x88).cast_signed();
    let height = h.read::<u32>(this + 0x90).cast_signed();
    let inside =
        x > left && x < left.wrapping_add(width) && y > top && y < top.wrapping_add(height);
    h.write(this + 0x94, u8::from(inside));
    u32::from(inside)
}
