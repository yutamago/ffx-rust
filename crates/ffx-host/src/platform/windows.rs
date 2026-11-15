//! [`WindowsPlatform`]: the real Win32 window and its messages, the cursor
//! and the timers (docs/host.md, "Windows", "The platform seam").
//!
//! - **The window.** [`Platform::register_class`] registers a real class
//!   of the game's name whose procedure is [`host_wndproc`] (a Rust
//!   function), with `FFX.exe`'s own icon; [`Platform::create_window`]
//!   makes the real window with the game's strings, styles and size. The
//!   game holds the real `HWND` (a 32-bit value: window handles fit in 32
//!   bits on 64-bit Windows too).
//! - **Messages.** What Windows *sends* to the window (`WM_SIZE`,
//!   `WM_ACTIVATE`, `WM_CLOSE`) reaches [`host_wndproc`] on the game's
//!   main thread with no executor at hand: it is queued for
//!   [`Platform::take_sent`]. `PeekMessage` peeks the real queue.
//! - **Background mode** (`FFX_HARNESS_BACKGROUND=1`, the harness's,
//!   [`crate::harness::background`]): the window is made without
//!   `WS_VISIBLE`, `ShowWindow` shows it without activating it and puts it
//!   at the bottom of the window stack, `SetWindowPos` never raises or
//!   activates it, and the pointer the game reads is far outside its
//!   window, as `ffx-hook`'s harness does for `FFX.exe`. The game never
//!   moves or clips the user's mouse pointer (`SetCursorPos`, `ClipCursor`
//!   do nothing; outside the harness too, for now).
//! - **Graphics.** The real D3D11 device ([`crate::d3d11`]), its swap chain
//!   on the real window.
//! - **Timers.** `timeBeginPeriod`/`timeEndPeriod` forward, so `Sleep`
//!   has the 1 ms granularity the game asks for, not Windows' default
//!   15.6 ms.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, UpdateWindow};
use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows_sys::Win32::System::LibraryLoader::{
    EnumResourceNamesW, GetModuleHandleW, LOAD_LIBRARY_AS_DATAFILE, LOAD_LIBRARY_AS_IMAGE_RESOURCE,
    LoadLibraryExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging as wm;

use super::{Message, Platform, Point, Rect, WindowRequest};

const WM_SIZE: u32 = 0x0005;
const WM_ACTIVATE: u32 = 0x0006;
const WM_CLOSE: u32 = 0x0010;
const WS_VISIBLE: u32 = 0x1000_0000;

/// The Windows platform.
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsPlatform;

/// The player's `FFX.exe`, whose icon the real window class shows
/// ([`Platform::set_icon_source`]).
static ICON_SOURCE: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

thread_local! {
    /// Messages Windows sent to a game window on this thread, for the
    /// game's procedure at its next `PeekMessage`.
    static SENT: RefCell<VecDeque<[u32; 4]>> = const { RefCell::new(VecDeque::new()) };
}

/// The real `HWND` of the game's window handle.
pub(crate) fn hwnd(v: u32) -> HWND {
    v as usize as HWND
}

fn hwnd_u32(h: HWND) -> u32 {
    u32::try_from(h as usize).unwrap_or(0)
}

fn rect(r: Rect) -> RECT {
    RECT {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}

fn from_rect(r: RECT) -> Rect {
    Rect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}

/// The first icon group of [`ICON_SOURCE`] at `size` pixels (0: the
/// system's default size), or null when there is none. The module stays
/// loaded as a data file (icons are shared from it).
fn game_icon(size: i32) -> wm::HICON {
    use std::os::windows::ffi::OsStrExt;
    unsafe extern "system" fn first(
        _: windows_sys::Win32::Foundation::HMODULE,
        _: windows_sys::core::PCWSTR,
        name: windows_sys::core::PCWSTR,
        found: isize,
    ) -> windows_sys::core::BOOL {
        // SAFETY: `found` is the `usize` the caller passed (below).
        unsafe { *(found as *mut usize) = name as usize };
        0
    }
    let Some(path) = ICON_SOURCE.lock().ok().and_then(|s| s.clone()) else {
        return std::ptr::null_mut();
    };
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: plain calls; `wide` and `name` outlive them.
    unsafe {
        let module = LoadLibraryExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE,
        );
        if module.is_null() {
            return std::ptr::null_mut();
        }
        let mut name = 0usize;
        EnumResourceNamesW(
            module,
            wm::RT_GROUP_ICON,
            Some(first),
            (&raw mut name) as isize,
        );
        if name == 0 {
            return std::ptr::null_mut();
        }
        wm::LoadImageW(
            module,
            name as windows_sys::core::PCWSTR,
            wm::IMAGE_ICON,
            size,
            size,
            if size == 0 { wm::LR_DEFAULTSIZE } else { 0 },
        )
        .cast()
    }
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

/// The real structure of a game message.
fn real_msg(m: &Message) -> wm::MSG {
    // SAFETY: a zeroed plain structure.
    let mut r: wm::MSG = unsafe { std::mem::zeroed() };
    r.hwnd = hwnd(m.window);
    r.message = m.message;
    r.wParam = m.wparam as usize;
    #[allow(clippy::cast_possible_wrap)]
    {
        r.lParam = m.lparam as i32 as isize;
    }
    r.pt.x = m.point.x;
    r.pt.y = m.point.y;
    r.time = m.time;
    r
}

#[allow(clippy::cast_possible_wrap)] // lParam: the game's 32-bit word, sign-extended
fn lparam(l: u32) -> LPARAM {
    l as i32 as isize
}

impl Platform for WindowsPlatform {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn windowed(&self) -> bool {
        true
    }

    fn set_icon_source(&self, exe: &std::path::Path) {
        if let Ok(mut s) = ICON_SOURCE.lock() {
            *s = Some(exe.to_path_buf());
        }
    }

    fn register_class(&self, style: u32, name: &[u8]) -> u16 {
        // SAFETY: plain calls with locals that outlive them.
        unsafe {
            let real = wm::WNDCLASSA {
                style,
                lpfnWndProc: Some(host_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(std::ptr::null()),
                hIcon: game_icon(0),
                hCursor: wm::LoadCursorW(std::ptr::null_mut(), wm::IDC_ARROW),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: name.as_ptr(),
            };
            wm::RegisterClassA(&raw const real)
        }
    }

    fn create_window(&self, r: &WindowRequest<'_>) -> u32 {
        let mut style = r.style;
        if crate::harness::background() {
            style &= !WS_VISIBLE;
        }
        // SAFETY: the strings outlive the call; no parent, menu or param on
        // the real side (the game's param reaches its procedure in the
        // game-facing side's WM_CREATE).
        let real = unsafe {
            wm::CreateWindowExA(
                r.ex_style,
                r.class.as_ptr(),
                r.title.as_ptr(),
                style,
                r.x,
                r.y,
                r.width,
                r.height,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            )
        };
        hwnd_u32(real)
    }

    fn show_window(&self, window: u32, cmd: i32) -> i32 {
        let h = hwnd(window);
        let mut cmd = cmd;
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
        r
    }

    fn set_window_pos(
        &self,
        window: u32,
        after: u32,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32 {
        let mut after = hwnd(after);
        let mut flags = flags;
        if crate::harness::background() {
            after = wm::HWND_BOTTOM;
            flags |= wm::SWP_NOACTIVATE;
            flags &= !wm::SWP_NOZORDER;
        }
        // SAFETY: the game's window.
        unsafe { wm::SetWindowPos(hwnd(window), after, x, y, cx, cy, flags) }
    }

    fn client_rect(&self, window: u32) -> (i32, Rect) {
        let mut r = rect(Rect::default());
        // SAFETY: a local out-structure.
        let ok = unsafe { wm::GetClientRect(hwnd(window), &raw mut r) };
        (ok, from_rect(r))
    }

    fn window_rect(&self, window: u32) -> (i32, Rect) {
        let mut r = rect(Rect::default());
        // SAFETY: a local out-structure.
        let ok = unsafe { wm::GetWindowRect(hwnd(window), &raw mut r) };
        (ok, from_rect(r))
    }

    fn adjust_window_rect(&self, r: &mut Rect, style: u32, menu: i32) -> i32 {
        let mut real = rect(*r);
        // SAFETY: a local.
        let ok = unsafe { wm::AdjustWindowRect(&raw mut real, style, menu) };
        *r = from_rect(real);
        ok
    }

    fn client_to_screen(&self, window: u32, point: &mut Point) -> i32 {
        let mut p = POINT {
            x: point.x,
            y: point.y,
        };
        // SAFETY: a local.
        let ok = unsafe { ClientToScreen(hwnd(window), &raw mut p) };
        *point = Point { x: p.x, y: p.y };
        ok
    }

    fn take_sent(&self) -> Option<[u32; 4]> {
        SENT.with(|q| q.borrow_mut().pop_front())
    }

    fn peek_message(&self, window: u32, min: u32, max: u32, remove: u32) -> Option<Message> {
        // SAFETY: a zeroed plain structure.
        let mut m: wm::MSG = unsafe { std::mem::zeroed() };
        // SAFETY: a local out-structure; the filter is the game's.
        let got = unsafe { wm::PeekMessageW(&raw mut m, hwnd(window), min, max, remove) };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 32-bit words
        (got != 0).then(|| Message {
            window: hwnd_u32(m.hwnd),
            message: m.message,
            wparam: m.wParam as u32,
            lparam: m.lParam as u32,
            time: m.time,
            point: Point {
                x: m.pt.x,
                y: m.pt.y,
            },
        })
    }

    fn translate_message(&self, message: &Message) -> i32 {
        let m = real_msg(message);
        // SAFETY: a local copy of the game's message.
        unsafe { wm::TranslateMessage(&raw const m) }
    }

    fn dispatch_message(&self, message: &Message) -> u32 {
        let m = real_msg(message);
        // SAFETY: a local copy of a message for a window that isn't the game's.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let r = unsafe { wm::DispatchMessageW(&raw const m) } as u32;
        r
    }

    fn def_window_proc(&self, window: u32, message: u32, wparam: u32, l: u32) -> u32 {
        // SAFETY: the game's window and a message with a plain `lParam`
        // (posted, or one `host_wndproc` queued).
        let r = unsafe { wm::DefWindowProcA(hwnd(window), message, wparam as usize, lparam(l)) };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let r = r as u32;
        r
    }

    fn set_window_long(&self, window: u32, index: i32, value: i32) -> i32 {
        // SAFETY: the game's window.
        unsafe { wm::SetWindowLongA(hwnd(window), index, value) }
    }

    fn window_long(&self, window: u32, index: i32) -> i32 {
        // SAFETY: the game's window.
        unsafe { wm::GetWindowLongA(hwnd(window), index) }
    }

    fn set_window_text(&self, window: u32, text: &[u8]) -> i32 {
        // SAFETY: a NUL-terminated string.
        unsafe { wm::SetWindowTextA(hwnd(window), text.as_ptr()) }
    }

    fn update_window(&self, window: u32) -> i32 {
        // SAFETY: the game's window.
        unsafe { UpdateWindow(hwnd(window)) }
    }

    fn destroy_window(&self, window: u32) -> i32 {
        // SAFETY: the game's window.
        unsafe { wm::DestroyWindow(hwnd(window)) }
    }

    fn set_parent(&self, child: u32, parent: u32) -> u32 {
        // SAFETY: the game's window and the parent it was handed.
        hwnd_u32(unsafe { wm::SetParent(hwnd(child), hwnd(parent)) })
    }

    fn post_message(&self, window: u32, message: u32, wparam: u32, l: u32) -> i32 {
        // SAFETY: a message with plain words for the game's window.
        unsafe { wm::PostMessageA(hwnd(window), message, wparam as usize, lparam(l)) }
    }

    fn post_quit_message(&self, code: i32) {
        // SAFETY: a plain call.
        unsafe { wm::PostQuitMessage(code) }
    }

    fn system_metric(&self, index: i32) -> i32 {
        // SAFETY: a plain query.
        unsafe { wm::GetSystemMetrics(index) }
    }

    fn cursor_pos(&self) -> Point {
        let mut p = POINT {
            x: -32000,
            y: -32000,
        };
        if !crate::harness::background() {
            // SAFETY: a local.
            unsafe { wm::GetCursorPos(&raw mut p) };
        }
        Point { x: p.x, y: p.y }
    }

    fn set_cursor_pos(&self, _: i32, _: i32) -> i32 {
        1
    }

    fn clip_cursor(&self, _: Option<Rect>) -> i32 {
        1
    }

    fn clip_rect(&self) -> Rect {
        let mut r = rect(Rect::default());
        // SAFETY: a local.
        unsafe { wm::GetClipCursor(&raw mut r) };
        from_rect(r)
    }

    fn show_cursor(&self, show: i32) -> i32 {
        // SAFETY: a plain call.
        unsafe { wm::ShowCursor(show) }
    }

    fn timer_period(&self, begin: bool, ms: u32) -> u32 {
        // SAFETY: plain calls.
        unsafe {
            if begin {
                timeBeginPeriod(ms)
            } else {
                timeEndPeriod(ms)
            }
        }
    }

    fn graphics(&self) -> Box<dyn crate::gfx::Backend> {
        Box::new(crate::d3d11::D3d11::default())
    }
}
