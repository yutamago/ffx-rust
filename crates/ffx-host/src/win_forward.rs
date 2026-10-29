//! Windows: the window, message, cursor and timer imports forwarded to the
//! real Win32 API (docs/host.md, "Windows"). Bound ahead of the
//! platform-free stand-ins ([`crate::imports`]), so on Windows these win.
//!
//! - **The window.** `RegisterClassA` registers a real class of the same
//!   name whose window procedure is [`host_wndproc`] (a Rust function); the
//!   game's procedure (`Phyre_PFramework_PWindowWin32Base_WndProc`,
//!   `0x00625130`) is kept by class. `CreateWindowExA` makes the real
//!   window with the game's strings, styles and size, and then gives the
//!   game's procedure its `WM_CREATE` (a `CREATESTRUCTA` in the arena), as
//!   Windows would have during the call. The game sees the real `HWND` (a
//!   32-bit value on i686; stage 4 maps it through the handle table).
//! - **Messages.** `PeekMessageA/W` peek the real queue into a `MSG` in the
//!   arena; `DispatchMessageA/W` of a message for the game's window calls
//!   the game's procedure in the executor (emulated or ported), anything
//!   else goes to the real `DispatchMessage`. Messages Windows *sends* to
//!   the window (`WM_SIZE`, `WM_ACTIVATE`) reach [`host_wndproc`] on the
//!   game's main thread with no executor at hand: they are queued and given
//!   to the game's procedure at its next `PeekMessage`. `WM_CLOSE` is not
//!   given to the game (its handler starts the Steam launcher): it ends the
//!   run. `DefWindowProcA` forwards, except for the `WM_CREATE` the host
//!   made up.
//! - **Background mode** (`FFX_HARNESS_BACKGROUND=1`, the harness's,
//!   [`crate::harness::background`]): the window is made without
//!   `WS_VISIBLE`, `ShowWindow` shows it without activating it and puts it
//!   at the bottom of the window stack, `SetWindowPos` never raises or
//!   activates it, the game never moves or clips the user's mouse pointer
//!   (`SetCursorPos`, `ClipCursor` do nothing) and the pointer it reads is
//!   far outside its window, as `ffx-hook`'s harness does for `FFX.exe`.
//! - **Timers.** `timeBeginPeriod`/`timeEndPeriod` forward: without them
//!   Windows' `Sleep` keeps its 15.6 ms granularity, and the boot (whose
//!   threads poll with `Sleep(1)`) took 190 s instead of 7.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Mutex;

use ffx_bridge::Mem;
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, UpdateWindow};
use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging as wm;

use crate::exec::{Exec, Stop};
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

const USER32: &str = "USER32.dll";
const WM_CREATE: u32 = 0x0001;
const WM_SIZE: u32 = 0x0005;
const WM_ACTIVATE: u32 = 0x0006;
const WM_CLOSE: u32 = 0x0010;
const WS_VISIBLE: u32 = 0x1000_0000;
const GWL_WNDPROC: i32 = -4;

/// The game's window procedures, by class name (lower case).
static CLASSES: Mutex<Vec<(String, u32)>> = Mutex::new(Vec::new());
/// The game's procedure of each real window (`HWND` as `u32`).
static WINDOWS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

thread_local! {
    /// Messages Windows sent to a game window on this thread, for the
    /// game's procedure at its next `PeekMessage`.
    static SENT: RefCell<VecDeque<[u32; 4]>> = const { RefCell::new(VecDeque::new()) };
}

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

fn hwnd(v: u32) -> HWND {
    v as usize as HWND
}

fn hwnd_u32(h: HWND) -> u32 {
    u32::try_from(h as usize).unwrap_or(0)
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

/// The game's procedure for `hwnd`.
fn game_proc(h: u32) -> Option<u32> {
    WINDOWS
        .lock()
        .ok()
        .and_then(|w| w.iter().find(|&&(x, _)| x == h).map(|&(_, p)| p))
}

/// The real window procedure of every game window: queues what the game
/// must see, answers the rest as Windows does.
unsafe extern "system" fn host_wndproc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    match msg {
        WM_SIZE | WM_ACTIVATE | WM_CLOSE => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 32-bit words
            let m = [hwnd_u32(h), msg, w as u32, l as u32];
            SENT.with(|q| q.borrow_mut().push_back(m));
            if msg == WM_CLOSE {
                return 0;
            }
        }
        _ => {}
    }
    // SAFETY: the arguments Windows gave us.
    unsafe { wm::DefWindowProcA(h, msg, w, l) }
}

/// Calls the game's window procedure.
fn call_game(e: &mut Exec, proc_: u32, m: [u32; 4]) -> u32 {
    let args = Args {
        stack: m.to_vec(),
        ..Args::default()
    };
    e.call(proc_, Conv::Stdcall, &args).map_or(0, |r| r.eax)
}

/// Gives the game's procedure the messages Windows sent since the last
/// time; `WM_CLOSE` ends the run instead.
fn deliver_sent(e: &mut Exec) {
    while let Some(m) = SENT.with(|q| q.borrow_mut().pop_front()) {
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
    // SAFETY: plain calls with locals that outlive them.
    let atom = unsafe {
        let real = wm::WNDCLASSA {
            style: wc[0],
            lpfnWndProc: Some(host_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: wm::LoadCursorW(std::ptr::null_mut(), wm::IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: name.as_ptr(),
        };
        wm::RegisterClassA(&raw const real)
    };
    e.stats.messages.push(format!(
        "RegisterClassA -> {atom:#x} (game procedure {:#010x})",
        wc[1]
    ));
    Ret::eax(u32::from(atom))
}

/// `CreateWindowExA(ex, class, title, style, x, y, w, h, parent, menu,
/// instance, param)`.
fn create_window_ex_a(e: &mut Exec, args: &Args) -> Ret {
    let class = c_string(e, a(args, 1));
    let title = c_string(e, a(args, 2));
    let mut style = a(args, 3);
    if crate::harness::background() {
        style &= !WS_VISIBLE;
    }
    let key = String::from_utf8_lossy(&class[..class.len().saturating_sub(1)]).to_lowercase();
    let proc_ = CLASSES
        .lock()
        .ok()
        .and_then(|c| c.iter().find(|(n, _)| *n == key).map(|&(_, p)| p));
    #[allow(clippy::cast_possible_wrap)] // CW_USEDEFAULT and friends
    let [left, top, width, height] =
        [a(args, 4), a(args, 5), a(args, 6), a(args, 7)].map(|v| v as i32);
    // SAFETY: the strings are locals; no parent, menu or param on the
    // real side (the game's param reaches its procedure below).
    let real = unsafe {
        wm::CreateWindowExA(
            a(args, 0),
            class.as_ptr(),
            title.as_ptr(),
            style,
            left,
            top,
            width,
            height,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        )
    };
    let hw = hwnd_u32(real);
    e.stats.milestones.push(format!(
        "first window (CreateWindowExA: a real one, {width} x {height}{})",
        if crate::harness::background() {
            ", background"
        } else {
            ""
        }
    ));
    if hw == 0 {
        return Ret::eax(0);
    }
    if let Some(p) = proc_ {
        if let Ok(mut l) = WINDOWS.lock() {
            l.push((hw, p));
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
        let r = call_game(e, p, [hw, WM_CREATE, 0, cs]);
        e.release(cs);
        if r == u32::MAX {
            // SAFETY: our own window.
            unsafe { wm::DestroyWindow(real) };
            return Ret::eax(0);
        }
    }
    Ret::eax(hw)
}

/// `ShowWindow(hwnd, cmd)`; background mode: never activated, at the
/// bottom of the stack.
fn show_window(_: &mut Exec, args: &Args) -> Ret {
    let h = hwnd(a(args, 0));
    #[allow(clippy::cast_possible_wrap)]
    let mut cmd = a(args, 1) as i32;
    let background = crate::harness::background() && cmd != 0;
    if background {
        cmd = if matches!(cmd, 2 | 6 | 7) {
            wm::SW_SHOWMINNOACTIVE
        } else {
            wm::SW_SHOWNOACTIVATE
        };
    }
    // SAFETY: the game's window.
    let r = unsafe { wm::ShowWindow(h, cmd) };
    if background {
        // SAFETY: as above; only the stacking order changes.
        unsafe {
            wm::SetWindowPos(
                h,
                wm::HWND_BOTTOM,
                0,
                0,
                0,
                0,
                wm::SWP_NOMOVE | wm::SWP_NOSIZE | wm::SWP_NOACTIVATE,
            );
        }
    }
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(r as u32)
}

/// `SetWindowPos(hwnd, after, x, y, cx, cy, flags)`; background mode: at
/// the bottom, not activated.
fn set_window_pos(_: &mut Exec, args: &Args) -> Ret {
    let mut after = hwnd(a(args, 1));
    let mut flags = a(args, 6);
    if crate::harness::background() {
        after = wm::HWND_BOTTOM;
        flags |= wm::SWP_NOACTIVATE;
        flags &= !wm::SWP_NOZORDER;
    }
    #[allow(clippy::cast_possible_wrap)]
    let [x, y, cx, cy] = [a(args, 2), a(args, 3), a(args, 4), a(args, 5)].map(|v| v as i32);
    // SAFETY: the game's window.
    let r = unsafe { wm::SetWindowPos(hwnd(a(args, 0)), after, x, y, cx, cy, flags) };
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(r as u32)
}

/// `GetClientRect` / `GetWindowRect(hwnd, &rect)`.
fn rect_query(e: &mut Exec, args: &Args, client: bool) -> Ret {
    let mut r = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: a local out-structure.
    let ok = unsafe {
        if client {
            wm::GetClientRect(hwnd(a(args, 0)), &raw mut r)
        } else {
            wm::GetWindowRect(hwnd(a(args, 0)), &raw mut r)
        }
    };
    if a(args, 1) != 0 {
        #[allow(clippy::cast_sign_loss)]
        e.mem().write(
            a(args, 1),
            [r.left, r.top, r.right, r.bottom].map(|v| v as u32),
        );
    }
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(ok as u32)
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
    let w: [u32; 4] = e.mem().read(at);
    #[allow(clippy::cast_possible_wrap)]
    let mut r = RECT {
        left: w[0] as i32,
        top: w[1] as i32,
        right: w[2] as i32,
        bottom: w[3] as i32,
    };
    #[allow(clippy::cast_possible_wrap)]
    // SAFETY: a local.
    let ok = unsafe { wm::AdjustWindowRect(&raw mut r, a(args, 1), a(args, 2) as i32) };
    #[allow(clippy::cast_sign_loss)]
    e.mem()
        .write(at, [r.left, r.top, r.right, r.bottom].map(|v| v as u32));
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(ok as u32)
}

/// `ClientToScreen(hwnd, &point)`.
fn client_to_screen(e: &mut Exec, args: &Args) -> Ret {
    let at = a(args, 1);
    let w: [u32; 2] = e.mem().read(at);
    #[allow(clippy::cast_possible_wrap)]
    let mut p = POINT {
        x: w[0] as i32,
        y: w[1] as i32,
    };
    // SAFETY: a local.
    let ok = unsafe { ClientToScreen(hwnd(a(args, 0)), &raw mut p) };
    #[allow(clippy::cast_sign_loss)]
    e.mem().write(at, [p.x as u32, p.y as u32]);
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(ok as u32)
}

/// `PeekMessageA/W(&msg, hwnd, min, max, remove)`: first the messages
/// Windows sent (to the game's procedure), then the real queue.
fn peek_message(e: &mut Exec, args: &Args) -> Ret {
    deliver_sent(e);
    if e.stopped().is_some() {
        return Ret::eax(0);
    }
    // SAFETY: a zeroed plain structure.
    let mut m: wm::MSG = unsafe { std::mem::zeroed() };
    // SAFETY: a local out-structure; the filter is the game's.
    let got = unsafe {
        wm::PeekMessageW(
            &raw mut m,
            hwnd(a(args, 1)),
            a(args, 2),
            a(args, 3),
            a(args, 4),
        )
    };
    // a message sent while peeking
    deliver_sent(e);
    if got != 0 && a(args, 0) != 0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 32-bit words
        let words = [
            hwnd_u32(m.hwnd),
            m.message,
            m.wParam as u32,
            m.lParam as u32,
            m.time,
            m.pt.x as u32,
            m.pt.y as u32,
        ];
        e.mem().write(a(args, 0), words);
    }
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(got as u32)
}

/// The `MSG` at `at` in the arena, as the real structure.
fn read_msg(e: &Exec, at: u32) -> wm::MSG {
    let w: [u32; 7] = e.mem().read(at);
    // SAFETY: a zeroed plain structure.
    let mut m: wm::MSG = unsafe { std::mem::zeroed() };
    m.hwnd = hwnd(w[0]);
    m.message = w[1];
    m.wParam = w[2] as usize;
    #[allow(clippy::cast_possible_wrap)]
    {
        m.lParam = w[3] as i32 as isize;
        m.pt.x = w[5] as i32;
        m.pt.y = w[6] as i32;
    }
    m.time = w[4];
    m
}

/// `TranslateMessage(&msg)`.
fn translate_message(e: &mut Exec, args: &Args) -> Ret {
    let m = read_msg(e, a(args, 0));
    // SAFETY: a local copy of the game's message.
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(unsafe { wm::TranslateMessage(&raw const m) } as u32)
}

/// `DispatchMessageA/W(&msg)`: the game's procedure for its window.
fn dispatch_message(e: &mut Exec, args: &Args) -> Ret {
    let w: [u32; 4] = e.mem().read(a(args, 0));
    if let Some(p) = game_proc(w[0]) {
        return Ret::eax(call_game(e, p, w));
    }
    let m = read_msg(e, a(args, 0));
    // SAFETY: a local copy of a message for a window that isn't the game's.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ret::eax(unsafe { wm::DispatchMessageW(&raw const m) } as u32)
}

/// `DefWindowProcA(hwnd, msg, w, l)`: the real one, except for the
/// `WM_CREATE` the host made up (its `lParam` is an arena address).
fn def_window_proc_a(_: &mut Exec, args: &Args) -> Ret {
    let msg = a(args, 1);
    if msg == WM_CREATE {
        return Ret::eax(0);
    }
    #[allow(clippy::cast_possible_wrap)]
    // SAFETY: the game's window and a message with a plain `lParam`
    // (posted, or one `host_wndproc` queued).
    let r = unsafe {
        wm::DefWindowProcA(
            hwnd(a(args, 0)),
            msg,
            a(args, 2) as usize,
            a(args, 3) as i32 as isize,
        )
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ret::eax(r as u32)
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

/// `SetWindowLongA(hwnd, index, value)`: the real one, except the window
/// procedure (`GWL_WNDPROC`), which the host keeps.
fn set_window_long_a(_: &mut Exec, args: &Args) -> Ret {
    #[allow(clippy::cast_possible_wrap)]
    let index = a(args, 1) as i32;
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
    #[allow(clippy::cast_possible_wrap)]
    // SAFETY: the game's window.
    let r = unsafe { wm::SetWindowLongA(hwnd(h), index, a(args, 2) as i32) };
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(r as u32)
}

/// `GetWindowLongA(hwnd, index)`.
fn get_window_long_a(_: &mut Exec, args: &Args) -> Ret {
    #[allow(clippy::cast_possible_wrap)]
    let index = a(args, 1) as i32;
    if index == GWL_WNDPROC {
        return Ret::eax(game_proc(a(args, 0)).unwrap_or(0));
    }
    // SAFETY: the game's window.
    let r = unsafe { wm::GetWindowLongA(hwnd(a(args, 0)), index) };
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(r as u32)
}

/// `SetWindowTextA(hwnd, text)`.
fn set_window_text_a(e: &mut Exec, args: &Args) -> Ret {
    let text = c_string(e, a(args, 1));
    // SAFETY: a local NUL-terminated string.
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(unsafe { wm::SetWindowTextA(hwnd(a(args, 0)), text.as_ptr()) } as u32)
}

/// `GetSystemMetrics(index)`.
fn get_system_metrics(_: &mut Exec, args: &Args) -> Ret {
    #[allow(clippy::cast_possible_wrap)]
    // SAFETY: a plain query.
    let r = unsafe { wm::GetSystemMetrics(a(args, 0) as i32) };
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(r as u32)
}

/// `GetCursorPos(&point)`: background mode: far outside every window
/// (`ffx-hook`'s harness does the same for `FFX.exe`).
fn get_cursor_pos(e: &mut Exec, args: &Args) -> Ret {
    let mut p = POINT {
        x: -32000,
        y: -32000,
    };
    if !crate::harness::background() {
        // SAFETY: a local.
        unsafe { wm::GetCursorPos(&raw mut p) };
    }
    #[allow(clippy::cast_sign_loss)]
    e.mem().write(a(args, 0), [p.x as u32, p.y as u32]);
    Ret::eax(1)
}

/// `SetCursorPos(x, y)` / `ClipCursor(&rect)`: the game never moves or
/// clips the user's pointer (in background mode as the hook does; outside
/// the harness too, for now: a development host).
fn cursor_noop(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(1)
}

/// `GetClipCursor(&rect)`: the whole screen.
fn get_clip_cursor(e: &mut Exec, args: &Args) -> Ret {
    let mut r = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: a local.
    unsafe { wm::GetClipCursor(&raw mut r) };
    #[allow(clippy::cast_sign_loss)]
    e.mem().write(
        a(args, 0),
        [r.left, r.top, r.right, r.bottom].map(|v| v as u32),
    );
    Ret::eax(1)
}

/// `UpdateWindow(hwnd)`.
fn update_window(_: &mut Exec, args: &Args) -> Ret {
    // SAFETY: the game's window.
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(unsafe { UpdateWindow(hwnd(a(args, 0))) } as u32)
}

/// `DestroyWindow(hwnd)`.
fn destroy_window(_: &mut Exec, args: &Args) -> Ret {
    let h = a(args, 0);
    if let Ok(mut l) = WINDOWS.lock() {
        l.retain(|e| e.0 != h);
    }
    // SAFETY: the game's window.
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(unsafe { wm::DestroyWindow(hwnd(h)) } as u32)
}

/// `PostQuitMessage(code)`.
fn post_quit_message(_: &mut Exec, args: &Args) -> Ret {
    #[allow(clippy::cast_possible_wrap)]
    // SAFETY: a plain call.
    unsafe {
        wm::PostQuitMessage(a(args, 0) as i32);
    }
    Ret::VOID
}

/// `PostMessageA(hwnd, msg, w, l)`.
fn post_message_a(_: &mut Exec, args: &Args) -> Ret {
    #[allow(clippy::cast_possible_wrap)]
    // SAFETY: a message with plain words for the game's window.
    let r = unsafe {
        wm::PostMessageA(
            hwnd(a(args, 0)),
            a(args, 1),
            a(args, 2) as usize,
            a(args, 3) as i32 as isize,
        )
    };
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(r as u32)
}

/// `ShowCursor(show)`.
fn show_cursor(_: &mut Exec, args: &Args) -> Ret {
    #[allow(clippy::cast_possible_wrap)]
    // SAFETY: a plain call.
    let r = unsafe { wm::ShowCursor(a(args, 0) as i32) };
    #[allow(clippy::cast_sign_loss)]
    Ret::eax(r as u32)
}

/// `timeBeginPeriod(ms)` / `timeEndPeriod(ms)`.
fn time_begin_period(_: &mut Exec, args: &Args) -> Ret {
    // SAFETY: a plain call.
    Ret::eax(unsafe { timeBeginPeriod(a(args, 0)) })
}

fn time_end_period(_: &mut Exec, args: &Args) -> Ret {
    // SAFETY: a plain call.
    Ret::eax(unsafe { timeEndPeriod(a(args, 0)) })
}

/// Every forwarded import: DLL, name, convention, stack words.
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
    (USER32, "SetCursorPos", Conv::Stdcall, 2, cursor_noop),
    (USER32, "ClipCursor", Conv::Stdcall, 1, cursor_noop),
    (USER32, "GetClipCursor", Conv::Stdcall, 1, get_clip_cursor),
    (USER32, "UpdateWindow", Conv::Stdcall, 1, update_window),
    (USER32, "DestroyWindow", Conv::Stdcall, 1, destroy_window),
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
