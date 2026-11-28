//! [`NativePlatform`]: `ffx-rust` on Linux and macOS with a real window,
//! the keyboard, the mouse, game controllers, sound and Steam
//! (docs/host.md, "Native platform").
//!
//! - **Threads.** winit wants its event loop on the process's main thread
//!   (macOS insists; Linux follows the same rule here), and the game's
//!   main thread blocks in its own loop (`Exec::run_entry`). So
//!   [`run`] (through `platform::run_main`, from `ffx-rust`'s `main`)
//!   makes the event loop on the main thread, starts the game on a thread
//!   of its own (`ffx-rust-game`), and runs the loop until the game
//!   returns. The two meet in [`State`] (one mutex): the loop thread
//!   turns window events into the messages the game's pump peeks and into
//!   the keyboard and mouse state its `DirectInput` devices read; the game
//!   thread takes them. The game thread asks the loop thread for what
//!   needs it (making the window: [`on_loop`], which waits for the
//!   answer); the loop thread never waits for the game. winit's window
//!   methods are called from the game thread directly (winit sends them
//!   to the main thread itself where the system wants that), never while
//!   [`State`] is locked.
//! - **The window.** One window, handle [`WINDOW`]: made at the game's
//!   `CreateWindowExA` with its title and size, shown at `ShowWindow`,
//!   moved and sized by `SetWindowPos`; a borderless window covering the
//!   screen (the game's full-screen mode: a `WS_POPUP` window the size of
//!   the screen) becomes winit's borderless full screen. The client area
//!   is the window (`AdjustWindowRect` adds no frame: the system draws its
//!   frame outside), so the size the game asks for is the size it draws.
//! - **Messages** ([`messages`]): the Win32 messages the Windows platform
//!   delivers, word for word. Sent ones (`WM_SIZE`, `WM_ACTIVATE`,
//!   `WM_CLOSE`) go to [`Platform::take_sent`]; posted ones (keys,
//!   characters after `TranslateMessage`, the mouse, `WM_QUIT`) to
//!   [`Platform::peek_message`]. `DefWindowProcA` of Alt+F4 closes the
//!   window, as Windows' does.
//! - **Input** ([`dinput`], [`keys`], [`pads`]): the system keyboard and
//!   mouse as foreground `DirectInput` devices (scan codes from the
//!   physical key, raw mouse motion), game controllers through gilrs as
//!   `XInput` pads.
//! - **Sound** ([`audio`]): cpal, with the WASAPI output's queue rules.
//! - **Steam** (`super::steam_api`): `libsteam_api.so` / `.dylib` beside
//!   the executable or at `FFX_STEAM_API`; none: no Steam, as on Windows.
//! - **The documents folder**: none of its own: `player::documents`
//!   picks the Proton prefix of the game on Linux, else our own folder.
//! - **Message boxes** ([`dialog`]): stderr and a desktop dialog.
//! - **Graphics**: the null device until the wgpu back end is in place;
//!   [`Platform::surface`] hands that back end the window.
//! - **Background mode** (`FFX_HARNESS_BACKGROUND=1`): the window is never
//!   shown or activated (on Linux the loop uses X11 then, where a hidden
//!   window stays unmapped; Wayland maps every window), the pointer the
//!   game reads stays far outside it, and `input::real` keeps the game on
//!   the layer's stand-in devices.

use std::collections::VecDeque;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{
    DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::PhysicalKey;
use winit::window::{Fullscreen, Window, WindowId};

use super::{DeviceCall, DeviceInfo, Message, Platform, Point, Property, Rect, WindowRequest};

pub mod audio;
pub mod dialog;
pub mod dinput;
pub mod keys;
pub mod messages;
pub mod pads;

use messages::{Button, GAME_SENT, MK_CONTROL, MK_SHIFT, WM_CLOSE, WM_QUIT};

/// The game window's handle.
pub const WINDOW: u32 = 0x0002_0010;
/// `CW_USEDEFAULT`.
const CW_USEDEFAULT: i32 = i32::MIN;
/// `WS_VISIBLE`.
const WS_VISIBLE: u32 = 0x1000_0000;
/// `WS_CAPTION`.
const WS_CAPTION: u32 = 0x00c0_0000;
/// `GWL_STYLE`.
const GWL_STYLE: i32 = -16;
/// `GWL_EXSTYLE`.
const GWL_EXSTYLE: i32 = -20;
/// `VK_F4`.
const VK_F4: u32 = 0x73;
/// `WM_SYSKEYDOWN`.
const WM_SYSKEYDOWN: u32 = messages::WM_SYSKEYDOWN;
/// Where the pointer the game reads is while it is not over the window
/// (and always in background mode): far outside, as the harness keeps it
/// (docs/harness.md, "The mouse in background games").
const OUTSIDE: Point = Point {
    x: -32108,
    y: -32131,
};

/// The native platform.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativePlatform;

/// The game's window.
#[derive(Debug, Clone)]
struct Win {
    window: Arc<Window>,
    /// The client size, physical pixels.
    size: (u32, u32),
    /// The client area's top left on the screen.
    origin: Point,
    visible: bool,
    minimized: bool,
}

/// What the loop thread and the game thread share.
#[derive(Debug)]
struct State {
    /// Sent messages for the game's procedure: window, message, w, l.
    sent: VecDeque<[u32; 4]>,
    /// Posted messages, and the text of a key-down's characters.
    posted: VecDeque<(Message, Option<String>)>,
    /// The last key-down `PeekMessage` removed, and its text, for
    /// `TranslateMessage`.
    last_key: Option<(Message, String)>,
    window: Option<Win>,
    input: dinput::Input,
    /// The primary screen.
    screen: Rect,
    /// The pointer in client coordinates (`None`: not over the window).
    cursor: Option<(i32, i32)>,
    /// Modifiers: Alt, Shift, Control.
    alt: bool,
    shift: bool,
    ctrl: bool,
    /// `MK_*` of the mouse buttons held.
    mk: u32,
    /// `ShowCursor`'s display count.
    cursor_count: i32,
    /// `SetWindowLongA` values other than the procedure, by index.
    longs: Vec<(i32, i32)>,
    classes: u16,
    start: Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sent: VecDeque::new(),
            posted: VecDeque::new(),
            last_key: None,
            window: None,
            input: dinput::Input::default(),
            screen: Rect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            cursor: None,
            alt: false,
            shift: false,
            ctrl: false,
            mk: 0,
            cursor_count: 0,
            longs: Vec::new(),
            classes: 0,
            start: Instant::now(),
        }
    }
}

impl State {
    /// Milliseconds since the start (a message's time).
    #[allow(clippy::cast_possible_truncation)] // wraps after 49 days, as GetTickCount
    fn time(&self) -> u32 {
        self.start.elapsed().as_millis() as u32
    }

    /// The pointer on the screen as the game reads it.
    fn screen_cursor(&self) -> Point {
        match (&self.window, self.cursor) {
            (Some(w), Some((x, y))) if !crate::harness::background() => Point {
                x: w.origin.x + x,
                y: w.origin.y + y,
            },
            _ => OUTSIDE,
        }
    }

    /// Queues a sent message for the game's procedure (only those the
    /// Windows platform forwards: [`GAME_SENT`]).
    fn send(&mut self, m: messages::Words) {
        if GAME_SENT.contains(&m[0]) {
            self.sent.push_back([WINDOW, m[0], m[1], m[2]]);
        }
    }

    /// Posts a message to the game's window.
    fn post(&mut self, m: messages::Words, text: Option<String>) {
        let msg = Message {
            window: WINDOW,
            message: m[0],
            wparam: m[1],
            lparam: m[2],
            time: self.time(),
            point: self.screen_cursor(),
        };
        self.posted.push_back((msg, text));
    }

    fn mk(&self) -> u32 {
        self.mk | if self.shift { MK_SHIFT } else { 0 } | if self.ctrl { MK_CONTROL } else { 0 }
    }

    /// One window event, as messages and input state.
    #[allow(clippy::too_many_lines)] // one arm per event
    fn event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let Some(k) = keys::key(code) else { return };
                let down = event.state == ElementState::Pressed;
                self.input.key(k.dik, down);
                if !self.input.focused() {
                    return;
                }
                let m = messages::key(k, down, event.repeat, self.alt);
                let text = event
                    .text
                    .as_ref()
                    .filter(|_| down)
                    .map(ToString::to_string);
                self.post(m, text);
            }
            WindowEvent::ModifiersChanged(m) => {
                let s = m.state();
                (self.alt, self.shift, self.ctrl) = (s.alt_key(), s.shift_key(), s.control_key());
            }
            WindowEvent::CursorMoved { position, .. } => {
                #[allow(clippy::cast_possible_truncation)] // pixels
                let (x, y) = (position.x.round() as i32, position.y.round() as i32);
                self.cursor = Some((x, y));
                let mk = self.mk();
                self.post(messages::mouse_move(x, y, mk), None);
            }
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    MouseButton::Left => Button::Left,
                    MouseButton::Right => Button::Right,
                    MouseButton::Middle => Button::Middle,
                    MouseButton::Back => Button::X1,
                    MouseButton::Forward => Button::X2,
                    MouseButton::Other(_) => return,
                };
                let down = *state == ElementState::Pressed;
                self.input.button(b.index(), down);
                if down {
                    self.mk |= b.mk();
                } else {
                    self.mk &= !b.mk();
                }
                let (x, y) = self.cursor.unwrap_or_default();
                let mk = self.mk();
                self.post(messages::button(b, down, x, y, mk), None);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                #[allow(clippy::cast_possible_truncation)] // notches and pixels
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => (y * 120.0).round() as i32,
                    MouseScrollDelta::PixelDelta(p) => p.y.round() as i32,
                };
                if d == 0 {
                    return;
                }
                self.input.wheel(d);
                let p = self.screen_cursor();
                let mk = self.mk();
                self.post(messages::wheel(d, p.x, p.y, mk), None);
            }
            WindowEvent::Resized(size) => {
                let (w, h) = (size.width, size.height);
                let kind = match &mut self.window {
                    Some(win) => {
                        win.size = (w, h);
                        win.minimized = w == 0 || h == 0 || win.window.is_minimized() == Some(true);
                        if win.minimized {
                            messages::SizeKind::Minimized
                        } else if win.window.is_maximized() {
                            messages::SizeKind::Maximized
                        } else {
                            messages::SizeKind::Restored
                        }
                    }
                    None => messages::SizeKind::Restored,
                };
                self.send(messages::size(kind, w, h));
            }
            WindowEvent::Moved(_) => {
                if let Some(win) = &mut self.window
                    && let Ok(p) = win.window.inner_position()
                {
                    win.origin = Point { x: p.x, y: p.y };
                }
            }
            WindowEvent::Focused(focused) => {
                self.input.focus(*focused);
                if !*focused {
                    (self.alt, self.shift, self.ctrl, self.mk) = (false, false, false, 0);
                }
                let minimized = self.window.as_ref().is_some_and(|w| w.minimized);
                for m in messages::activate(*focused, minimized) {
                    self.send(m);
                }
            }
            WindowEvent::CloseRequested => self.send([WM_CLOSE, 0, 0]),
            _ => {}
        }
    }
}

/// A request to the loop thread.
enum Request {
    /// Run this there.
    Run(Job),
    /// The game returned: end the loop.
    Exit,
}

/// The loop thread's proxy, once the loop exists.
static PROXY: OnceLock<Mutex<EventLoopProxy<Request>>> = OnceLock::new();
/// The shared state.
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
/// The game thread's exit code.
static EXIT: Mutex<Option<ExitCode>> = Mutex::new(None);

fn state() -> MutexGuard<'static, State> {
    let m = STATE.get_or_init(|| Mutex::new(State::default()));
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Runs `f` on the loop thread and waits for its answer (`None`: no loop,
/// or it ended).
fn on_loop<T: Send + 'static>(f: impl FnOnce(&ActiveEventLoop) -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let proxy = PROXY.get()?.lock().ok()?.clone();
    proxy
        .send_event(Request::Run(Box::new(move |el| {
            let _ = tx.send(f(el));
        })))
        .ok()?;
    rx.recv().ok()
}

/// The game window's winit window.
fn game_window() -> Option<Arc<Window>> {
    state().window.as_ref().map(|w| Arc::clone(&w.window))
}

/// Work for the loop thread.
type Job = Box<dyn FnOnce(&ActiveEventLoop) + Send>;

/// The event loop's side.
struct App {
    /// `resumed` came: windows may be made.
    ready: bool,
    /// Requests that came before it.
    waiting: Vec<Job>,
}

impl ApplicationHandler<Request> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if let Some(m) = el
            .primary_monitor()
            .or_else(|| el.available_monitors().next())
        {
            let (p, s) = (m.position(), m.size());
            #[allow(clippy::cast_possible_wrap)] // screen pixels
            let screen = Rect {
                left: p.x,
                top: p.y,
                right: p.x + s.width as i32,
                bottom: p.y + s.height as i32,
            };
            state().screen = screen;
        }
        self.ready = true;
        for f in std::mem::take(&mut self.waiting) {
            f(el);
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, request: Request) {
        match request {
            Request::Run(f) if self.ready => f(el),
            Request::Run(f) => self.waiting.push(f),
            Request::Exit => el.exit(),
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        state().event(&event);
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            #[allow(clippy::cast_possible_truncation)] // counts
            state()
                .input
                .motion(delta.0.round() as i32, delta.1.round() as i32);
        }
    }
}

/// Whether `ffx-rust`'s run should use the native platform: asked
/// (`FFX_HOST_PLATFORM=native`), or by default when nothing asks for
/// headless: not a harness game, not the null device (`--gfx-null`,
/// `FFX_HOST_GFX=null`), and a display (macOS always; Linux `DISPLAY` or
/// `WAYLAND_DISPLAY`).
#[must_use]
pub fn wanted() -> bool {
    let env = |n: &str| std::env::var(n).unwrap_or_default();
    decide(
        &env("FFX_HOST_PLATFORM"),
        std::env::var_os("FFX_HARNESS").is_some(),
        env("FFX_HOST_GFX") == "null" || std::env::args().any(|a| a == "--gfx-null"),
        cfg!(target_os = "macos")
            || !env("DISPLAY").is_empty()
            || !env("WAYLAND_DISPLAY").is_empty(),
    )
}

/// [`wanted`]'s rule.
#[must_use]
pub fn decide(asked: &str, harness: bool, null: bool, display: bool) -> bool {
    match asked {
        "native" => true,
        "" => !harness && !null && display,
        _ => false,
    }
}

/// Runs `game` on a thread of its own with the native platform installed
/// and the event loop on this (the main) thread; `game`'s exit code. When
/// no event loop can be made (no display) or a platform is in place
/// already, `game` runs here on the platform there is.
pub fn run<F>(game: F) -> ExitCode
where
    F: FnOnce() -> ExitCode + Send + 'static,
{
    let mut builder = EventLoop::<Request>::with_user_event();
    #[cfg(target_os = "linux")]
    if crate::harness::background() && std::env::var_os("DISPLAY").is_some() {
        use winit::platform::x11::EventLoopBuilderExtX11 as _;
        builder.with_x11();
    }
    let event_loop = match builder.build() {
        Ok(l) => l,
        Err(e) => {
            crate::log::note(&format!("native platform: no event loop ({e}); headless"));
            return game();
        }
    };
    if super::install(Box::new(NativePlatform)).is_err() {
        return game();
    }
    let _ = PROXY.set(Mutex::new(event_loop.create_proxy()));
    pads::start();
    let exit = event_loop.create_proxy();
    let spawned = std::thread::Builder::new()
        .name("ffx-rust-game".into())
        // the process's main thread's 8 MB, and room besides
        .stack_size(64 << 20)
        .spawn(move || {
            let code = game();
            if let Ok(mut e) = EXIT.lock() {
                *e = Some(code);
            }
            let _ = exit.send_event(Request::Exit);
        });
    let thread = match spawned {
        Ok(t) => t,
        Err(e) => {
            eprintln!("native platform: no game thread: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut app = App {
        ready: false,
        waiting: Vec::new(),
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        crate::log::note(&format!("native platform: the event loop ended: {e}"));
    }
    let joined = thread.join();
    match EXIT.lock().ok().and_then(|e| *e) {
        Some(code) if joined.is_ok() => code,
        _ => ExitCode::FAILURE,
    }
}

/// The game's window as the graphics back end sees it.
#[derive(Debug)]
struct WindowSurface(Arc<Window>);

impl raw_window_handle::HasWindowHandle for WindowSurface {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        self.0.window_handle()
    }
}

impl raw_window_handle::HasDisplayHandle for WindowSurface {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.0.display_handle()
    }
}

impl super::GfxWindow for WindowSurface {
    fn size(&self) -> (u32, u32) {
        let s = self.0.inner_size();
        (s.width, s.height)
    }

    /// On the event loop's thread (macOS: the main thread, where the
    /// back end's `CAMetalLayer` must be made), waiting for it.
    fn on_window_thread(&self, f: Box<dyn FnOnce() + Send>) {
        if on_loop(move |_| f()).is_none() {
            crate::log::note("native platform: no event loop for on_window_thread");
        }
    }
}

/// The window the game asked for, made on the loop thread.
fn make_window(r: &WindowRequest<'_>) -> Option<Arc<Window>> {
    let title = String::from_utf8_lossy(r.title.strip_suffix(&[0]).unwrap_or(r.title)).into_owned();
    let (w, h) = if r.width == CW_USEDEFAULT || r.width <= 0 || r.height <= 0 {
        (1280, 720)
    } else {
        (r.width.cast_unsigned(), r.height.cast_unsigned())
    };
    let background = crate::harness::background();
    let mut attrs = Window::default_attributes()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(w, h))
        .with_decorations(r.style & WS_CAPTION != 0 || r.style & 0x8000_0000 == 0)
        .with_visible(!background && r.style & WS_VISIBLE != 0)
        .with_active(!background);
    if r.x != CW_USEDEFAULT {
        attrs = attrs.with_position(PhysicalPosition::new(r.x, r.y));
    }
    on_loop(move |el| el.create_window(attrs).map(Arc::new))?
        .map_err(|e| crate::log::note(&format!("native platform: no window: {e}")))
        .ok()
}

/// Whether the rectangle is the whole screen.
fn covers(screen: Rect, x: i32, y: i32, cx: i32, cy: i32) -> bool {
    x <= screen.left && y <= screen.top && x + cx >= screen.right && y + cy >= screen.bottom
}

impl Platform for NativePlatform {
    fn name(&self) -> &'static str {
        "native"
    }

    fn windowed(&self) -> bool {
        true
    }

    fn set_icon_source(&self, _: &std::path::Path) {}

    fn register_class(&self, _: u32, _: &[u8]) -> u16 {
        let mut s = state();
        s.classes += 1;
        0xc000 + s.classes
    }

    fn create_window(&self, r: &WindowRequest<'_>) -> u32 {
        if state().window.is_some() {
            // one window: the game makes one
            return 0;
        }
        let Some(window) = make_window(r) else {
            return 0;
        };
        let size = window.inner_size();
        let origin = window
            .inner_position()
            .map(|p| Point { x: p.x, y: p.y })
            .unwrap_or_default();
        let visible = window.is_visible().unwrap_or(false);
        let mut s = state();
        s.longs = vec![
            (GWL_STYLE, r.style.cast_signed()),
            (GWL_EXSTYLE, r.ex_style.cast_signed()),
        ];
        s.window = Some(Win {
            window,
            size: (size.width, size.height),
            origin,
            visible,
            minimized: false,
        });
        WINDOW
    }

    fn show_window(&self, window: u32, cmd: i32) -> i32 {
        let Some(w) = (window == WINDOW).then(game_window).flatten() else {
            return 0;
        };
        let was = state().window.as_ref().is_some_and(|w| w.visible);
        if crate::harness::background() {
            return i32::from(was);
        }
        match cmd {
            0 => w.set_visible(false),
            2 | 6 | 7 => {
                w.set_visible(true);
                w.set_minimized(true);
            }
            3 => {
                w.set_visible(true);
                w.set_maximized(true);
            }
            _ => {
                w.set_visible(true);
                w.set_minimized(false);
            }
        }
        if let Some(win) = state().window.as_mut() {
            win.visible = cmd != 0;
        }
        i32::from(was)
    }

    fn set_window_pos(
        &self,
        window: u32,
        _: u32,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32 {
        const NOSIZE: u32 = 0x1;
        const NOMOVE: u32 = 0x2;
        const SHOWWINDOW: u32 = 0x40;
        const HIDEWINDOW: u32 = 0x80;
        let Some(w) = (window == WINDOW).then(game_window).flatten() else {
            return 0;
        };
        let (screen, style) = {
            let s = state();
            (
                s.screen,
                s.longs
                    .iter()
                    .find(|l| l.0 == GWL_STYLE)
                    .map_or(0, |l| l.1.cast_unsigned()),
            )
        };
        let background = crate::harness::background();
        let full = flags & (NOSIZE | NOMOVE) == 0
            && style & WS_CAPTION == 0
            && covers(screen, x, y, cx, cy);
        if full && !background {
            w.set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            if w.fullscreen().is_some() {
                w.set_fullscreen(None);
            }
            w.set_decorations(style & WS_CAPTION != 0);
            if flags & NOSIZE == 0 && cx > 0 && cy > 0 {
                let _ =
                    w.request_inner_size(PhysicalSize::new(cx.cast_unsigned(), cy.cast_unsigned()));
            }
            if flags & NOMOVE == 0 {
                w.set_outer_position(PhysicalPosition::new(x, y));
            }
        }
        if !background {
            if flags & SHOWWINDOW != 0 {
                w.set_visible(true);
            } else if flags & HIDEWINDOW != 0 {
                w.set_visible(false);
            }
        }
        1
    }

    fn client_rect(&self, window: u32) -> (i32, Rect) {
        let s = state();
        match (&s.window, window == WINDOW) {
            #[allow(clippy::cast_possible_wrap)] // pixels
            (Some(w), true) => (
                1,
                Rect {
                    left: 0,
                    top: 0,
                    right: w.size.0 as i32,
                    bottom: w.size.1 as i32,
                },
            ),
            _ => (0, Rect::default()),
        }
    }

    fn window_rect(&self, window: u32) -> (i32, Rect) {
        let s = state();
        match (&s.window, window == WINDOW) {
            #[allow(clippy::cast_possible_wrap)] // pixels
            (Some(w), true) => (
                1,
                Rect {
                    left: w.origin.x,
                    top: w.origin.y,
                    right: w.origin.x + w.size.0 as i32,
                    bottom: w.origin.y + w.size.1 as i32,
                },
            ),
            _ => (0, Rect::default()),
        }
    }

    fn adjust_window_rect(&self, _: &mut Rect, _: u32, _: i32) -> i32 {
        // no frame of ours: the system draws it outside the client area
        1
    }

    fn client_to_screen(&self, window: u32, point: &mut Point) -> i32 {
        let s = state();
        match (&s.window, window == WINDOW) {
            (Some(w), true) => {
                point.x += w.origin.x;
                point.y += w.origin.y;
                1
            }
            _ => 0,
        }
    }

    fn take_sent(&self) -> Option<[u32; 4]> {
        state().sent.pop_front()
    }

    fn peek_message(&self, window: u32, min: u32, max: u32, remove: u32) -> Option<Message> {
        let mut s = state();
        let at = s.posted.iter().position(|(m, _)| {
            m.message == WM_QUIT
                || ((window == 0 || m.window == window)
                    && ((min == 0 && max == 0) || (min..=max).contains(&m.message)))
        })?;
        let m = s.posted[at].0;
        if remove & 1 != 0 {
            let (m, text) = s.posted.remove(at)?;
            s.last_key = text.map(|t| (m, t));
        }
        Some(m)
    }

    fn translate_message(&self, message: &Message) -> i32 {
        let mut s = state();
        let key = matches!(message.message, 0x100..=0x109);
        if let Some((m, text)) = s.last_key.take()
            && m == *message
        {
            for c in messages::chars([m.message, m.wparam, m.lparam], &text)
                .into_iter()
                .rev()
            {
                let msg = Message {
                    message: c[0],
                    wparam: c[1],
                    ..m
                };
                s.posted.push_front((msg, None));
            }
        }
        i32::from(key)
    }

    fn dispatch_message(&self, _: &Message) -> u32 {
        0
    }

    fn def_window_proc(&self, window: u32, message: u32, wparam: u32, lparam: u32) -> u32 {
        // Alt+F4: SC_CLOSE, then WM_CLOSE, as Windows' default procedure
        if window == WINDOW && message == WM_SYSKEYDOWN && wparam == VK_F4 && lparam & 1 << 29 != 0
        {
            state().send([WM_CLOSE, 0, 0]);
        }
        0
    }

    fn set_window_long(&self, _: u32, index: i32, value: i32) -> i32 {
        let decorations = {
            let mut s = state();
            let old = s.longs.iter().find(|l| l.0 == index).map_or(0, |l| l.1);
            s.longs.retain(|l| l.0 != index);
            s.longs.push((index, value));
            (index == GWL_STYLE).then_some((old, value.cast_unsigned() & WS_CAPTION != 0))
        };
        if let Some((old, d)) = decorations {
            if let Some(w) = game_window() {
                w.set_decorations(d);
            }
            return old;
        }
        0
    }

    fn window_long(&self, _: u32, index: i32) -> i32 {
        state()
            .longs
            .iter()
            .find(|l| l.0 == index)
            .map_or(0, |l| l.1)
    }

    fn set_window_text(&self, window: u32, text: &[u8]) -> i32 {
        let Some(w) = (window == WINDOW).then(game_window).flatten() else {
            return 0;
        };
        w.set_title(&String::from_utf8_lossy(
            text.strip_suffix(&[0]).unwrap_or(text),
        ));
        1
    }

    fn update_window(&self, _: u32) -> i32 {
        1
    }

    fn destroy_window(&self, window: u32) -> i32 {
        if window != WINDOW {
            return 0;
        }
        // the window goes with its last reference, on the loop thread
        let w = state().window.take();
        if let Some(w) = w {
            let _ = on_loop(move |_| drop(w));
        }
        1
    }

    fn set_parent(&self, _: u32, _: u32) -> u32 {
        0
    }

    fn post_message(&self, window: u32, message: u32, wparam: u32, lparam: u32) -> i32 {
        let mut s = state();
        let msg = Message {
            window,
            message,
            wparam,
            lparam,
            time: s.time(),
            point: s.screen_cursor(),
        };
        s.posted.push_back((msg, None));
        1
    }

    fn post_quit_message(&self, code: i32) {
        let mut s = state();
        let msg = Message {
            window: 0,
            message: WM_QUIT,
            wparam: code.cast_unsigned(),
            lparam: 0,
            time: s.time(),
            point: s.screen_cursor(),
        };
        s.posted.push_back((msg, None));
    }

    fn system_metric(&self, index: i32) -> i32 {
        let r = state().screen;
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        match index {
            0 | 16 | 78 => w, // SM_CXSCREEN, SM_CXFULLSCREEN, SM_CXVIRTUALSCREEN
            1 | 17 | 79 => h,
            80 => 1, // SM_CMONITORS
            _ => 0,
        }
    }

    fn cursor_pos(&self) -> Point {
        state().screen_cursor()
    }

    fn set_cursor_pos(&self, _: i32, _: i32) -> i32 {
        // as on Windows: the game never moves the player's pointer
        1
    }

    fn clip_cursor(&self, _: Option<Rect>) -> i32 {
        1
    }

    fn clip_rect(&self) -> Rect {
        state().screen
    }

    fn show_cursor(&self, show: i32) -> i32 {
        let count = {
            let mut s = state();
            s.cursor_count += if show != 0 { 1 } else { -1 };
            s.cursor_count
        };
        if let Some(w) = game_window() {
            w.set_cursor_visible(count >= 0);
        }
        count
    }

    fn timer_period(&self, _: bool, _: u32) -> u32 {
        // Linux's and macOS's sleeps are fine-grained already
        0
    }

    fn graphics(&self) -> Box<dyn crate::gfx::Backend> {
        // the wgpu back end on the game's window (offscreen before one
        // exists; the null device in a build without `gfx-wgpu`)
        crate::gfx::wgpu_backend(self.gfx_window())
    }

    fn gfx_window(&self) -> Option<Arc<dyn super::GfxWindow>> {
        game_window().map(|w| Arc::new(WindowSurface(w)) as Arc<dyn super::GfxWindow>)
    }

    fn input_devices(&self) -> bool {
        true
    }

    fn input_open(&self, handle: u32, guid: u128) -> Result<String, i32> {
        state().input.open(handle, guid).map(str::to_owned)
    }

    fn input_has(&self, handle: u32) -> bool {
        state().input.has(handle)
    }

    fn input_release(&self, handle: u32) {
        state().input.release(handle);
    }

    fn input_status(&self, guid: u128) -> Option<i32> {
        // DI_OK for the system devices; DI_NOTATTACHED (1) otherwise
        Some(i32::from(dinput::Kind::of(guid).is_none()))
    }

    fn input_controllers(&self, _: u32) -> Vec<Vec<u8>> {
        pads::pads()
            .iter()
            .map(|p| pads::instance(p, &pads::name(p.id)))
            .collect()
    }

    fn input_objects(&self, handle: u32, _: u32) -> Option<Vec<Vec<u8>>> {
        state().input.has(handle).then(Vec::new)
    }

    fn input_set_format(&self, handle: u32, _: &crate::input::Format) -> Option<i32> {
        state().input.has(handle).then_some(dinput::DI_OK)
    }

    fn input_set_cooperative_level(&self, handle: u32, _: u32, _: u32) -> Option<i32> {
        state().input.has(handle).then_some(dinput::DI_OK)
    }

    fn input_property(&self, handle: u32, _: &Property, _: &mut [u8], set: bool) -> Option<i32> {
        state().input.has(handle).then_some(if set {
            dinput::DI_OK
        } else {
            dinput::DIERR_UNSUPPORTED
        })
    }

    fn input_call(&self, handle: u32, call: DeviceCall) -> Option<i32> {
        let mut s = state();
        match call {
            DeviceCall::Acquire => s.input.acquire(handle),
            DeviceCall::Unacquire => s.input.unacquire(handle),
            // the system devices need no polling: DI_NOEFFECT
            DeviceCall::Poll => s.input.has(handle).then_some(dinput::DI_NOEFFECT),
        }
    }

    fn input_state(&self, handle: u32, size: usize) -> Option<Result<Vec<u8>, i32>> {
        state().input.state(handle, size)
    }

    fn input_data(&self, handle: u32, _: u32, _: u32) -> Option<Result<(Vec<[u32; 5]>, i32), i32>> {
        // the game never calls GetDeviceData: no buffered items
        state()
            .input
            .has(handle)
            .then(|| Ok((Vec::new(), dinput::DI_OK)))
    }

    fn input_info(&self, handle: u32, what: DeviceInfo, data: &mut [u8]) -> Option<i32> {
        state().input.info(handle, what, data)
    }

    fn xinput_state(&self, user: u32) -> Result<[u8; 16], u32> {
        pads::pads()
            .iter()
            .find(|p| p.slot == user)
            .map(pads::xinput_state)
            .ok_or(crate::input::NOT_CONNECTED)
    }

    fn xinput_set_state(&self, user: u32, v: [u8; 4]) -> u32 {
        if !pads::pads().iter().any(|p| p.slot == user) {
            return crate::input::NOT_CONNECTED;
        }
        pads::rumble(
            user,
            u16::from_le_bytes([v[0], v[1]]),
            u16::from_le_bytes([v[2], v[3]]),
        );
        0
    }

    fn xinput_ids(&self) -> Vec<(u32, u32)> {
        pads::pads().iter().map(|p| p.ids).collect()
    }

    fn audio_start(&self) {
        audio::start();
    }

    fn audio_running(&self) -> bool {
        audio::on()
    }

    fn audio_queued(&self) -> usize {
        audio::queued()
    }

    fn audio_underruns(&self) -> u64 {
        audio::underruns()
    }

    fn audio_push(&self, block: &[f32]) {
        audio::push(block);
    }

    fn steam(&self) -> Result<Box<dyn crate::steam::Backend>, String> {
        super::steam_api::load()
    }

    fn documents_folder(&self) -> Option<std::path::PathBuf> {
        // `player::documents` decides: the game's Proton prefix on Linux,
        // else our own folder
        None
    }

    fn message_box(&self, title: &str, text: &str, warning: bool) {
        dialog::show(title, text, warning);
    }

    fn end_process(&self, code: u32) -> ! {
        std::process::exit(code.cast_signed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    #[test]
    fn the_default_is_native_only_for_a_players_desktop() {
        assert!(decide("", false, false, true));
        assert!(!decide("", true, false, true), "a harness game: headless");
        assert!(!decide("", false, true, true), "the null device: headless");
        assert!(!decide("", false, false, false), "no display: headless");
        assert!(decide("native", true, true, false));
        assert!(!decide("headless", false, false, true));
    }

    fn key_event(s: &mut State, code: KeyCode, down: bool, text: Option<&str>) {
        let k = keys::key(code).unwrap();
        s.input.key(k.dik, down);
        let m = messages::key(k, down, false, s.alt);
        s.post(m, text.filter(|_| down).map(ToString::to_string));
    }

    #[test]
    fn keys_reach_the_pump_and_translate_into_characters() {
        let mut s = State::default();
        s.input.focus(true);
        key_event(&mut s, KeyCode::KeyA, true, Some("a"));
        key_event(&mut s, KeyCode::KeyA, false, None);
        // the pump: peek (remove), translate, the character comes next
        STATE.get_or_init(|| Mutex::new(State::default()));
        *state() = s;
        let p = NativePlatform;
        let down = p.peek_message(0, 0, 0, 1).unwrap();
        assert_eq!(
            (down.window, down.message, down.wparam, down.lparam),
            (WINDOW, 0x100, 0x41, 0x001e_0001)
        );
        assert_eq!(p.translate_message(&down), 1);
        let ch = p.peek_message(0, 0, 0, 1).unwrap();
        assert_eq!(
            (ch.message, ch.wparam, ch.lparam),
            (0x102, 0x61, 0x001e_0001)
        );
        // a filter that skips the key-up, then PM_NOREMOVE
        assert!(p.peek_message(0, 0x200, 0x20e, 1).is_none());
        let up = p.peek_message(WINDOW, 0x100, 0x108, 0).unwrap();
        assert_eq!(up.message, 0x101);
        assert_eq!(p.peek_message(0, 0, 0, 1).unwrap().message, 0x101);
        assert!(p.peek_message(0, 0, 0, 1).is_none());
        // WM_QUIT passes every filter
        p.post_quit_message(3);
        let q = p.peek_message(WINDOW, 0x100, 0x101, 1).unwrap();
        assert_eq!((q.message, q.wparam), (WM_QUIT, 3));
        // Alt+F4 through DefWindowProc closes the window
        let alt_f4 = messages::key(keys::key(KeyCode::F4).unwrap(), true, false, true);
        p.def_window_proc(WINDOW, alt_f4[0], alt_f4[1], alt_f4[2]);
        assert_eq!(p.take_sent(), Some([WINDOW, WM_CLOSE, 0, 0]));
        // the keyboard device saw the key go down and up
        assert_eq!(state().input.state(1, 256), None);
    }

    #[test]
    fn window_events_become_the_games_messages() {
        let mut s = State::default();
        s.event(&WindowEvent::Focused(true));
        assert_eq!(
            s.sent.pop_front(),
            Some([WINDOW, messages::WM_ACTIVATE, 1, 0])
        );
        assert!(
            s.sent.is_empty(),
            "WM_ACTIVATEAPP and WM_SETFOCUS stay out, as on Windows"
        );
        s.event(&WindowEvent::Resized(PhysicalSize::new(1280, 720)));
        assert_eq!(
            s.sent.pop_front(),
            Some([WINDOW, messages::WM_SIZE, 0, 0x02d0_0500])
        );
        s.event(&WindowEvent::CloseRequested);
        assert_eq!(s.sent.pop_front(), Some([WINDOW, WM_CLOSE, 0, 0]));
        // no window yet: the pointer is far outside
        s.event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: winit::dpi::PhysicalPosition::new(10.0, 20.0),
        });
        let (m, _) = s.posted.pop_front().unwrap();
        assert_eq!(
            (m.message, m.lparam, m.point),
            (messages::WM_MOUSEMOVE, 0x0014_000a, OUTSIDE)
        );
        s.event(&WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Left,
        });
        let (m, _) = s.posted.pop_front().unwrap();
        assert_eq!((m.message, m.wparam, m.lparam), (0x201, 1, 0x0014_000a));
        s.event(&WindowEvent::Focused(false));
        assert_eq!(
            s.sent.pop_front(),
            Some([WINDOW, messages::WM_ACTIVATE, 0, 0])
        );
        assert_eq!(s.mk, 0);
    }

    #[test]
    fn full_screen_is_a_rectangle_covering_the_screen() {
        let screen = Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert!(covers(screen, 0, 0, 1920, 1080));
        assert!(!covers(screen, 0, 0, 1280, 720));
        assert!(!covers(screen, 10, 0, 1920, 1080));
    }
}
