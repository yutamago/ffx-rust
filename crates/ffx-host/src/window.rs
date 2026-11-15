//! The game-facing side of the window, message, cursor and timer imports
//! (docs/host.md, "Windows", "The platform seam"): the game's arguments
//! read from the arena, the platform ([`crate::platform::get`]) asked,
//! the results written back. Bound ahead of the other stand-ins on every
//! platform.
//!
//! The game's window procedures stay here: `RegisterClassA` keeps the
//! game's procedure (`Phyre_PFramework_PWindowWin32Base_WndProc`,
//! `0x00625130`) by class, and on a platform with real windows
//! ([`crate::platform::Platform::windowed`]) `CreateWindowExA` gives it
//! its `WM_CREATE` (a `CREATESTRUCTA` in the arena) as Windows would have
//! during the call; `DispatchMessageA/W` and `SendMessageA/W` of a message
//! for the game's window call it in the executor (emulated or ported),
//! and the messages the platform *sent* to the window are given to it at
//! the next `PeekMessage`; `WM_CLOSE` is not given to the game (its
//! handler starts the Steam launcher): it ends the run. `DefWindowProcA`
//! answers the `WM_CREATE` the host made up itself. On the headless
//! platform the game holds a stand-in window and its procedure never
//! runs.

use std::sync::Mutex;

use ffx_bridge::Mem;

use crate::exec::{Exec, Stop};
use crate::imports::ImportFn;
use crate::platform::{self, Message, Point, Rect, WindowRequest};
use crate::table::{Args, Conv, Ret};

const USER32: &str = "USER32.dll";
const WM_CREATE: u32 = 0x0001;
const WM_CLOSE: u32 = 0x0010;
const GWL_WNDPROC: i32 = -4;

/// The game's window procedures, by class name (lower case).
static CLASSES: Mutex<Vec<(String, u32)>> = Mutex::new(Vec::new());
/// The game's procedure of each window the platform made (its handle).
static WINDOWS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

#[allow(clippy::cast_possible_wrap)] // the game's int arguments
fn i(args: &Args, n: usize) -> i32 {
    a(args, n) as i32
}

#[allow(clippy::cast_sign_loss)] // a Win32 result's bits
fn ret(v: i32) -> Ret {
    Ret::eax(v as u32)
}

/// A NUL-terminated string from the arena (at most 1 KB), with its NUL.
fn c_string(e: &Exec, at: u32) -> Vec<u8> {
    let mem = e.mem();
    let mut s = Vec::new();
    if at == 0 {
        return s;
    }
    for i in 0..1024 {
        let b: u8 = mem.read(at + i);
        if b == 0 {
            break;
        }
        s.push(b);
    }
    s.push(0);
    s
}

/// The game's procedure for its window `h`.
fn game_proc(h: u32) -> Option<u32> {
    WINDOWS
        .lock()
        .ok()
        .and_then(|w| w.iter().find(|&&(x, _)| x == h).map(|&(_, p)| p))
}

/// Calls the game's window procedure.
fn call_game(e: &mut Exec, proc_: u32, m: [u32; 4]) -> u32 {
    let args = Args {
        stack: m.to_vec(),
        ..Args::default()
    };
    e.call(proc_, Conv::Stdcall, &args).map_or(0, |r| r.eax)
}

/// Gives the game's procedure the messages the platform sent since the
/// last time; `WM_CLOSE` ends the run instead.
fn deliver_sent(e: &mut Exec) {
    while let Some(m) = platform::get().take_sent() {
        if m[1] == WM_CLOSE {
            e.end(Stop::Milestone("the window was closed (WM_CLOSE)".into()));
            return;
        }
        if let Some(p) = game_proc(m[0]) {
            call_game(e, p, m);
        }
    }
}

/// `RegisterClassA(&wndclass)`.
fn register_class_a(e: &mut Exec, args: &Args) -> Ret {
    let at = a(args, 0);
    let wc: [u32; 10] = e.mem().read(at);
    let name = c_string(e, wc[9]);
    let key = String::from_utf8_lossy(&name[..name.len().saturating_sub(1)]).to_lowercase();
    if let Ok(mut c) = CLASSES.lock() {
        c.retain(|(n, _)| *n != key);
        c.push((key, wc[1]));
    }
    let p = platform::get();
    let atom = p.register_class(wc[0], &name);
    if p.windowed() {
        e.stats.messages.push(format!(
            "RegisterClassA -> {atom:#x} (game procedure {:#010x})",
            wc[1]
        ));
    }
    Ret::eax(u32::from(atom))
}

/// `CreateWindowExA(ex, class, title, style, x, y, w, h, parent, menu,
/// instance, param)`: the platform's window; on a windowed platform the
/// game's `WM_CREATE` (`--stop-at USER32.dll!CreateWindowExA` stops here
/// instead).
fn create_window_ex_a(e: &mut Exec, args: &Args) -> Ret {
    let class = c_string(e, a(args, 1));
    let title = c_string(e, a(args, 2));
    let p = platform::get();
    let request = WindowRequest {
        ex_style: a(args, 0),
        class: &class,
        title: &title,
        style: a(args, 3),
        x: i(args, 4),
        y: i(args, 5),
        width: i(args, 6),
        height: i(args, 7),
    };
    let hw = p.create_window(&request);
    if !p.windowed() {
        // a stand-in that gets no messages
        e.stats
            .milestones
            .push("first window (CreateWindowExA)".into());
        return Ret::eax(hw);
    }
    e.stats.milestones.push(format!(
        "first window (CreateWindowExA: a real one, {} x {}{})",
        request.width,
        request.height,
        if crate::harness::background() {
            ", background"
        } else {
            ""
        }
    ));
    if hw == 0 {
        return Ret::eax(0);
    }
    let key = String::from_utf8_lossy(&class[..class.len().saturating_sub(1)]).to_lowercase();
    let proc_ = CLASSES
        .lock()
        .ok()
        .and_then(|c| c.iter().find(|(n, _)| *n == key).map(|&(_, p)| p));
    if let Some(proc_) = proc_ {
        if let Ok(mut l) = WINDOWS.lock() {
            l.push((hw, proc_));
        }
        // WM_CREATE with a CREATESTRUCTA in the arena: params, instance,
        // menu, parent, cy, cx, y, x, style, name, class, ex style
        let cs = e.alloc(48);
        let words = [
            a(args, 11),
            a(args, 10),
            a(args, 9),
            a(args, 8),
            a(args, 7),
            a(args, 6),
            a(args, 5),
            a(args, 4),
            a(args, 3),
            a(args, 2),
            a(args, 1),
            a(args, 0),
        ];
        e.mem().write(cs, words);
        let r = call_game(e, proc_, [hw, WM_CREATE, 0, cs]);
        e.release(cs);
        if r == u32::MAX {
            p.destroy_window(hw);
            return Ret::eax(0);
        }
    }
    Ret::eax(hw)
}

/// `ShowWindow(hwnd, cmd)`.
fn show_window(_: &mut Exec, args: &Args) -> Ret {
    ret(platform::get().show_window(a(args, 0), i(args, 1)))
}

/// `SetWindowPos(hwnd, after, x, y, cx, cy, flags)`.
fn set_window_pos(_: &mut Exec, args: &Args) -> Ret {
    ret(platform::get().set_window_pos(
        a(args, 0),
        a(args, 1),
        i(args, 2),
        i(args, 3),
        i(args, 4),
        i(args, 5),
        a(args, 6),
    ))
}

/// `GetClientRect` / `GetWindowRect(hwnd, &rect)`.
fn rect_query(e: &mut Exec, args: &Args, client: bool) -> Ret {
    let p = platform::get();
    let (ok, r) = if client {
        p.client_rect(a(args, 0))
    } else {
        p.window_rect(a(args, 0))
    };
    if a(args, 1) != 0 {
        e.mem().write(a(args, 1), r.words());
    }
    ret(ok)
}

fn get_client_rect(e: &mut Exec, args: &Args) -> Ret {
    rect_query(e, args, true)
}

fn get_window_rect(e: &mut Exec, args: &Args) -> Ret {
    rect_query(e, args, false)
}

/// `AdjustWindowRect(&rect, style, menu)`.
fn adjust_window_rect(e: &mut Exec, args: &Args) -> Ret {
    let at = a(args, 0);
    let before = Rect::from_words(e.mem().read(at));
    let mut r = before;
    let ok = platform::get().adjust_window_rect(&mut r, a(args, 1), i(args, 2));
    if r != before {
        e.mem().write(at, r.words());
    }
    ret(ok)
}

/// `ClientToScreen(hwnd, &point)`.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)] // the game's ints
fn client_to_screen(e: &mut Exec, args: &Args) -> Ret {
    let at = a(args, 1);
    let w: [u32; 2] = e.mem().read(at);
    let before = Point {
        x: w[0] as i32,
        y: w[1] as i32,
    };
    let mut pt = before;
    let ok = platform::get().client_to_screen(a(args, 0), &mut pt);
    if pt != before {
        e.mem().write(at, [pt.x as u32, pt.y as u32]);
    }
    ret(ok)
}

/// `PeekMessageA/W(&msg, hwnd, min, max, remove)`: first the messages
/// the platform sent (to the game's procedure), then its queue.
fn peek_message(e: &mut Exec, args: &Args) -> Ret {
    deliver_sent(e);
    if e.stopped().is_some() {
        return Ret::eax(0);
    }
    let got = platform::get().peek_message(a(args, 1), a(args, 2), a(args, 3), a(args, 4));
    // a message sent while peeking
    deliver_sent(e);
    if let Some(m) = got
        && a(args, 0) != 0
    {
        #[allow(clippy::cast_sign_loss)] // the game's ints
        let words = [
            m.window,
            m.message,
            m.wparam,
            m.lparam,
            m.time,
            m.point.x as u32,
            m.point.y as u32,
        ];
        e.mem().write(a(args, 0), words);
    }
    Ret::eax(u32::from(got.is_some()))
}

/// The `MSG` at `at` in the arena.
#[allow(clippy::cast_possible_wrap)] // the game's ints
fn read_msg(e: &Exec, at: u32) -> Message {
    let w: [u32; 7] = e.mem().read(at);
    Message {
        window: w[0],
        message: w[1],
        wparam: w[2],
        lparam: w[3],
        time: w[4],
        point: Point {
            x: w[5] as i32,
            y: w[6] as i32,
        },
    }
}

/// `TranslateMessage(&msg)`.
fn translate_message(e: &mut Exec, args: &Args) -> Ret {
    let m = read_msg(e, a(args, 0));
    ret(platform::get().translate_message(&m))
}

/// `DispatchMessageA/W(&msg)`: the game's procedure for its window, the
/// platform for others.
fn dispatch_message(e: &mut Exec, args: &Args) -> Ret {
    let w: [u32; 4] = e.mem().read(a(args, 0));
    if let Some(p) = game_proc(w[0]) {
        return Ret::eax(call_game(e, p, w));
    }
    let m = read_msg(e, a(args, 0));
    Ret::eax(platform::get().dispatch_message(&m))
}

/// `DefWindowProcA(hwnd, msg, w, l)`: the platform's, except for the
/// `WM_CREATE` the host made up (its `lParam` is an arena address).
fn def_window_proc_a(_: &mut Exec, args: &Args) -> Ret {
    let msg = a(args, 1);
    if msg == WM_CREATE {
        return Ret::eax(0);
    }
    Ret::eax(platform::get().def_window_proc(a(args, 0), msg, a(args, 2), a(args, 3)))
}

/// `SendMessageA/W(hwnd, msg, w, l)`: the game's procedure for its own
/// window; nothing for others.
fn send_message(e: &mut Exec, args: &Args) -> Ret {
    let m = [a(args, 0), a(args, 1), a(args, 2), a(args, 3)];
    match game_proc(m[0]) {
        Some(p) => Ret::eax(call_game(e, p, m)),
        None => Ret::eax(0),
    }
}

/// `SetWindowLongA(hwnd, index, value)`: the platform's, except the window
/// procedure (`GWL_WNDPROC`), which this side keeps.
fn set_window_long_a(_: &mut Exec, args: &Args) -> Ret {
    let index = i(args, 1);
    let h = a(args, 0);
    if index == GWL_WNDPROC {
        let old = game_proc(h).unwrap_or(0);
        if let Ok(mut l) = WINDOWS.lock() {
            for e in l.iter_mut().filter(|e| e.0 == h) {
                e.1 = a(args, 2);
            }
        }
        return Ret::eax(old);
    }
    ret(platform::get().set_window_long(h, index, i(args, 2)))
}

/// `GetWindowLongA(hwnd, index)`.
fn get_window_long_a(_: &mut Exec, args: &Args) -> Ret {
    let index = i(args, 1);
    if index == GWL_WNDPROC {
        return Ret::eax(game_proc(a(args, 0)).unwrap_or(0));
    }
    ret(platform::get().window_long(a(args, 0), index))
}

/// `SetWindowTextA(hwnd, text)`.
fn set_window_text_a(e: &mut Exec, args: &Args) -> Ret {
    let text = c_string(e, a(args, 1));
    ret(platform::get().set_window_text(a(args, 0), &text))
}

/// `GetSystemMetrics(index)`.
fn get_system_metrics(_: &mut Exec, args: &Args) -> Ret {
    ret(platform::get().system_metric(i(args, 0)))
}

/// `GetCursorPos(&point)`.
#[allow(clippy::cast_sign_loss)] // the game's ints
fn get_cursor_pos(e: &mut Exec, args: &Args) -> Ret {
    let p = platform::get().cursor_pos();
    e.mem().write(a(args, 0), [p.x as u32, p.y as u32]);
    Ret::eax(1)
}

/// `SetCursorPos(x, y)`.
fn set_cursor_pos(_: &mut Exec, args: &Args) -> Ret {
    ret(platform::get().set_cursor_pos(i(args, 0), i(args, 1)))
}

/// `ClipCursor(&rect)` (null: free).
fn clip_cursor(e: &mut Exec, args: &Args) -> Ret {
    let at = a(args, 0);
    let r = (at != 0).then(|| Rect::from_words(e.mem().read(at)));
    ret(platform::get().clip_cursor(r))
}

/// `GetClipCursor(&rect)`.
fn get_clip_cursor(e: &mut Exec, args: &Args) -> Ret {
    let r = platform::get().clip_rect();
    e.mem().write(a(args, 0), r.words());
    Ret::eax(1)
}

/// `UpdateWindow(hwnd)`.
fn update_window(_: &mut Exec, args: &Args) -> Ret {
    ret(platform::get().update_window(a(args, 0)))
}

/// `DestroyWindow(hwnd)`.
fn destroy_window(_: &mut Exec, args: &Args) -> Ret {
    let h = a(args, 0);
    if let Ok(mut l) = WINDOWS.lock() {
        l.retain(|e| e.0 != h);
    }
    ret(platform::get().destroy_window(h))
}

/// `SetParent(child, parent)`: the embedded mode's host window
/// (`PWindowWin32_AttachToHost`); the previous parent.
fn set_parent(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(platform::get().set_parent(a(args, 0), a(args, 1)))
}

/// `PostQuitMessage(code)`.
fn post_quit_message(_: &mut Exec, args: &Args) -> Ret {
    platform::get().post_quit_message(i(args, 0));
    Ret::VOID
}

/// `PostMessageA(hwnd, msg, w, l)`.
fn post_message_a(_: &mut Exec, args: &Args) -> Ret {
    ret(platform::get().post_message(a(args, 0), a(args, 1), a(args, 2), a(args, 3)))
}

/// `ShowCursor(show)`.
fn show_cursor(_: &mut Exec, args: &Args) -> Ret {
    ret(platform::get().show_cursor(i(args, 0)))
}

/// `timeBeginPeriod(ms)` / `timeEndPeriod(ms)`.
fn time_begin_period(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(platform::get().timer_period(true, a(args, 0)))
}

fn time_end_period(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(platform::get().timer_period(false, a(args, 0)))
}

/// Every import of this side: DLL, name, convention, stack words.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (USER32, "RegisterClassA", Conv::Stdcall, 1, register_class_a),
    (
        USER32,
        "CreateWindowExA",
        Conv::Stdcall,
        12,
        create_window_ex_a,
    ),
    (USER32, "ShowWindow", Conv::Stdcall, 2, show_window),
    (USER32, "SetWindowPos", Conv::Stdcall, 7, set_window_pos),
    (USER32, "GetClientRect", Conv::Stdcall, 2, get_client_rect),
    (USER32, "GetWindowRect", Conv::Stdcall, 2, get_window_rect),
    (
        USER32,
        "AdjustWindowRect",
        Conv::Stdcall,
        3,
        adjust_window_rect,
    ),
    (USER32, "ClientToScreen", Conv::Stdcall, 2, client_to_screen),
    (USER32, "PeekMessageA", Conv::Stdcall, 5, peek_message),
    (USER32, "PeekMessageW", Conv::Stdcall, 5, peek_message),
    (
        USER32,
        "TranslateMessage",
        Conv::Stdcall,
        1,
        translate_message,
    ),
    (
        USER32,
        "DispatchMessageA",
        Conv::Stdcall,
        1,
        dispatch_message,
    ),
    (
        USER32,
        "DispatchMessageW",
        Conv::Stdcall,
        1,
        dispatch_message,
    ),
    (
        USER32,
        "DefWindowProcA",
        Conv::Stdcall,
        4,
        def_window_proc_a,
    ),
    (USER32, "SendMessageA", Conv::Stdcall, 4, send_message),
    (USER32, "SendMessageW", Conv::Stdcall, 4, send_message),
    (
        USER32,
        "SetWindowLongA",
        Conv::Stdcall,
        3,
        set_window_long_a,
    ),
    (
        USER32,
        "GetWindowLongA",
        Conv::Stdcall,
        2,
        get_window_long_a,
    ),
    (
        USER32,
        "SetWindowTextA",
        Conv::Stdcall,
        2,
        set_window_text_a,
    ),
    (
        USER32,
        "GetSystemMetrics",
        Conv::Stdcall,
        1,
        get_system_metrics,
    ),
    (USER32, "GetCursorPos", Conv::Stdcall, 1, get_cursor_pos),
    (USER32, "SetCursorPos", Conv::Stdcall, 2, set_cursor_pos),
    (USER32, "ClipCursor", Conv::Stdcall, 1, clip_cursor),
    (USER32, "GetClipCursor", Conv::Stdcall, 1, get_clip_cursor),
    (USER32, "UpdateWindow", Conv::Stdcall, 1, update_window),
    (USER32, "DestroyWindow", Conv::Stdcall, 1, destroy_window),
    (USER32, "SetParent", Conv::Stdcall, 2, set_parent),
    (
        USER32,
        "PostQuitMessage",
        Conv::Stdcall,
        1,
        post_quit_message,
    ),
    (USER32, "PostMessageA", Conv::Stdcall, 4, post_message_a),
    (USER32, "ShowCursor", Conv::Stdcall, 1, show_cursor),
    (
        "WINMM.dll",
        "timeBeginPeriod",
        Conv::Stdcall,
        1,
        time_begin_period,
    ),
    (
        "WINMM.dll",
        "timeEndPeriod",
        Conv::Stdcall,
        1,
        time_end_period,
    ),
];
