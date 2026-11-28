//! The platform seam (docs/host.md, "The platform seam"): everything the
//! player-facing side of `ffx-rust` asks of the operating system goes
//! through one trait, [`Platform`].
//!
//! Two sides meet here. The **game-facing side** answers the Win32
//! imports `FFX.exe` calls (`crate::window`, `crate::input`,
//! `crate::audio`, `crate::steam`, ...): it reads the game's 32-bit
//! arguments from the arena, keeps the game's own state (its window
//! procedures, its devices' handles) and writes the results back. The
//! **platform side** is this trait: plain Rust values in and out, no arena,
//! no executor, so another frontend can implement it without knowing how
//! the game is run.
//!
//! - [`windows::WindowsPlatform`] (Windows): the real window and its
//!   message loop, `DirectInput8`/`XInput`/raw input, WASAPI, D3D11,
//!   `steam_api64.dll`, the documents folder, `MessageBoxW`,
//!   `TerminateProcess`;
//! - [`native::NativePlatform`] (Linux, macOS): a winit window and its
//!   events as Win32 messages, the keyboard and mouse as `DirectInput`
//!   devices, gilrs pads as `XInput`, cpal sound, `libsteam_api`, a dialog
//!   program for message boxes; its event loop runs on the process's main
//!   thread and the game on another ([`run_main`]);
//! - [`headless::HeadlessPlatform`]: no window (a stand-in that never gets a
//!   message), no input devices, no device sound, the recording null
//!   device, no Steam. The Linux check, the tests, harness games off
//!   Windows and the null-device runs use it.
//!
//! One platform serves the process: [`install`] one before the boot (as
//! [`run_main`] does for the native platform), or [`get`] picks it on first
//! use (`FFX_HOST_PLATFORM=windows|headless`; default: Windows on Windows,
//! headless elsewhere).

use std::sync::OnceLock;

pub mod headless;
#[cfg(not(windows))]
pub mod native;
pub mod steam_api;
#[cfg(windows)]
pub mod windows;

/// A rectangle, as Win32's `RECT`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge (exclusive).
    pub right: i32,
    /// Bottom edge (exclusive).
    pub bottom: i32,
}

impl Rect {
    /// From the game's four 32-bit words.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)] // the words are the game's ints
    pub fn from_words(w: [u32; 4]) -> Self {
        Self {
            left: w[0] as i32,
            top: w[1] as i32,
            right: w[2] as i32,
            bottom: w[3] as i32,
        }
    }

    /// The game's four 32-bit words.
    #[must_use]
    #[allow(clippy::cast_sign_loss)] // the game's ints
    pub fn words(self) -> [u32; 4] {
        [self.left, self.top, self.right, self.bottom].map(|v| v as u32)
    }
}

/// A point, as Win32's `POINT`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Point {
    /// X.
    pub x: i32,
    /// Y.
    pub y: i32,
}

/// A window message as the game's 32-bit `MSG` holds it: window handle,
/// message, `wParam`, `lParam`, time, cursor position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Message {
    /// The window (the handle the game holds).
    pub window: u32,
    /// `WM_*`.
    pub message: u32,
    /// `wParam` (32 bits in the game).
    pub wparam: u32,
    /// `lParam` (32 bits in the game).
    pub lparam: u32,
    /// The message's time.
    pub time: u32,
    /// The cursor when it was posted.
    pub point: Point,
}

/// The game's `CreateWindowExA` as the platform gets it. The strings are
/// NUL-terminated bytes (the game's code page).
#[derive(Debug, Clone, Copy)]
pub struct WindowRequest<'a> {
    /// The extended style.
    pub ex_style: u32,
    /// The class name the game registered ([`Platform::register_class`]).
    pub class: &'a [u8],
    /// The title.
    pub title: &'a [u8],
    /// The style (`WS_*`).
    pub style: u32,
    /// Position and size (`CW_USEDEFAULT` passes through).
    pub x: i32,
    /// See `x`.
    pub y: i32,
    /// See `x`.
    pub width: i32,
    /// See `x`.
    pub height: i32,
}

/// A window the wgpu back end draws into (`crate::gfx::wgpu`), handed out by
/// a platform with a real window ([`Platform::gfx_window`]): its
/// `raw-window-handle` handles and its client size.
pub trait GfxWindow:
    raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle + Send + Sync
{
    /// The client area in pixels (the surface's size; 0 while minimised).
    fn size(&self) -> (u32, u32);
    /// Runs `f` on the thread that owns the window and returns when it
    /// ran: the back end makes its surface there (Metal needs the main
    /// thread). Default: on the caller's thread.
    fn on_window_thread(&self, f: Box<dyn FnOnce() + Send>) {
        f();
    }
}

/// A `DirectInput` property (`Set/GetProperty`): `MAKEDIPROP(n)` or a GUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
    /// `MAKEDIPROP(n)` (a number where the API takes a GUID pointer).
    Id(u32),
    /// A property GUID.
    Guid(u128),
}

/// The argument-less device calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceCall {
    /// `Acquire`.
    Acquire,
    /// `Unacquire`.
    Unacquire,
    /// `Poll`.
    Poll,
}

/// What a device describes about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceInfo {
    /// `GetCapabilities` (`DIDEVCAPS`).
    Capabilities,
    /// `GetDeviceInfo` (`DIDEVICEINSTANCEA`).
    Instance,
}

/// The process's platform.
///
/// **Threads.** The implementation is shared by every game thread
/// (`Send + Sync`); each method says who calls it. The window methods run
/// on the game's main thread (the one that created the window and pumps
/// its messages).
///
/// **Handles.** Window handles are the 32-bit words the game holds; the
/// platform hands them out ([`Platform::create_window`]) and gets them
/// back in later calls.
pub trait Platform: Send + Sync {
    /// Its name, for the notes and the report.
    fn name(&self) -> &'static str;

    // ---- the window (the game's main thread) ----------------------------

    /// Whether the platform shows a real window that gets messages. When
    /// it does, the game-facing side gives the game's window procedure its
    /// `WM_CREATE` and every message the platform sends or posts; when it
    /// doesn't (headless), the game holds a stand-in window and its
    /// procedure never runs.
    fn windowed(&self) -> bool;
    /// The player's `FFX.exe` (at boot), for the window's icon (its first
    /// icon group; nothing of the game is built in).
    fn set_icon_source(&self, exe: &std::path::Path);
    /// `RegisterClassA` of the game's class `name` (NUL-terminated) with
    /// `style`: an atom, 0 when refused. The game's procedure stays on the
    /// game-facing side.
    fn register_class(&self, style: u32, name: &[u8]) -> u16;
    /// `CreateWindowExA`: the window's handle, 0 when it failed.
    fn create_window(&self, request: &WindowRequest<'_>) -> u32;
    /// `ShowWindow(window, cmd)`: whether it was visible before.
    fn show_window(&self, window: u32, cmd: i32) -> i32;
    /// `SetWindowPos`: nonzero on success.
    #[allow(clippy::too_many_arguments)] // the call's own
    fn set_window_pos(
        &self,
        window: u32,
        after: u32,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    /// `GetClientRect`: the result and the rectangle.
    fn client_rect(&self, window: u32) -> (i32, Rect);
    /// `GetWindowRect`: the result and the rectangle.
    fn window_rect(&self, window: u32) -> (i32, Rect);
    /// `AdjustWindowRect(rect, style, menu)`, in place.
    fn adjust_window_rect(&self, rect: &mut Rect, style: u32, menu: i32) -> i32;
    /// `ClientToScreen(window, point)`, in place.
    fn client_to_screen(&self, window: u32, point: &mut Point) -> i32;
    /// The next message the system *sent* to the game's window since the
    /// last call on this thread (`WM_SIZE`, `WM_ACTIVATE`, `WM_CLOSE`):
    /// window, message, `wParam`, `lParam`. The game-facing side gives
    /// them to the game's procedure before and after each
    /// [`Platform::peek_message`]; `WM_CLOSE` ends the run.
    fn take_sent(&self) -> Option<[u32; 4]>;
    /// `PeekMessage(window, min, max, remove)`: the next posted message,
    /// if any.
    fn peek_message(&self, window: u32, min: u32, max: u32, remove: u32) -> Option<Message>;
    /// `TranslateMessage`.
    fn translate_message(&self, message: &Message) -> i32;
    /// `DispatchMessage` of a message for a window that is not the game's
    /// (the game's own go to its procedure on the game-facing side).
    fn dispatch_message(&self, message: &Message) -> u32;
    /// `DefWindowProcA` (never the made-up `WM_CREATE`).
    fn def_window_proc(&self, window: u32, message: u32, wparam: u32, lparam: u32) -> u32;
    /// `SetWindowLongA` of anything but the window procedure (which the
    /// game-facing side keeps): the previous value.
    fn set_window_long(&self, window: u32, index: i32, value: i32) -> i32;
    /// `GetWindowLongA` of anything but the window procedure.
    fn window_long(&self, window: u32, index: i32) -> i32;
    /// `SetWindowTextA` (NUL-terminated).
    fn set_window_text(&self, window: u32, text: &[u8]) -> i32;
    /// `UpdateWindow`.
    fn update_window(&self, window: u32) -> i32;
    /// `DestroyWindow`.
    fn destroy_window(&self, window: u32) -> i32;
    /// `SetParent(child, parent)` (the embedded mode's host window): the
    /// previous parent.
    fn set_parent(&self, child: u32, parent: u32) -> u32;
    /// `PostMessageA`.
    fn post_message(&self, window: u32, message: u32, wparam: u32, lparam: u32) -> i32;
    /// `PostQuitMessage(code)`.
    fn post_quit_message(&self, code: i32);
    /// `GetSystemMetrics(index)`.
    fn system_metric(&self, index: i32) -> i32;
    /// `GetCursorPos`, in screen coordinates.
    fn cursor_pos(&self) -> Point;
    /// `SetCursorPos(x, y)`.
    fn set_cursor_pos(&self, x: i32, y: i32) -> i32;
    /// `ClipCursor(rect)` (`None`: free).
    fn clip_cursor(&self, rect: Option<Rect>) -> i32;
    /// `GetClipCursor`.
    fn clip_rect(&self) -> Rect;
    /// `ShowCursor(show)`: the display count after it.
    fn show_cursor(&self, show: i32) -> i32;

    // ---- timers (any thread) --------------------------------------------

    /// `timeBeginPeriod(ms)` (`begin`) / `timeEndPeriod(ms)`: the game's
    /// threads poll with `Sleep(1)`, so a platform whose sleeps are coarse
    /// makes them fine here.
    fn timer_period(&self, begin: bool, ms: u32) -> u32;

    // ---- graphics (the game's render thread, through `crate::gfx`) ------

    /// The back end behind the game's D3D11 calls when the run asks for
    /// the default ([`crate::gfx::Choice::Default`]): made at the game's
    /// `D3D11CreateDevice(AndSwapChain)`. Every call reaches it after the
    /// layer recorded it (`crate::gfx`: the draw-stream gate compares the
    /// recording, so it doesn't depend on the back end); `Present` shows a
    /// frame.
    fn graphics(&self) -> Box<dyn crate::gfx::Backend>;
    /// The window the wgpu back end draws into (`FFX_HOST_GFX=wgpu`, or a
    /// platform's own `graphics` through `crate::gfx::wgpu_backend`);
    /// `None`: offscreen.
    fn gfx_window(&self) -> Option<std::sync::Arc<dyn GfxWindow>> {
        None
    }

    // ---- input (the game's input threads, inside the layer's calls) -----
    //
    // The game's `IDirectInput8A` and its devices stay the layer's COM
    // objects (`crate::gfx`); each device the game creates may get one of
    // the platform's behind it, by the layer's handle. A device the
    // platform has none for is the layer's stand-in (the keyboard reads
    // only the harness's keys, the mouse nothing). The keyboard's state is
    // DirectInput's 256 scan-code bytes (0x80: down), the mouse's the
    // game's data format (`DIMOUSESTATE2`), a pad's `XINPUT_STATE`.
    // `crate::input` decides whether the devices are used at all
    // (`input_devices`, `FFX_HOST_INPUT`, not in a background harness
    // game) and adds the harness's keys.

    /// Whether the platform has the player's devices.
    fn input_devices(&self) -> bool;
    /// `CreateDevice(guid)` for the layer's device `handle`: the device's
    /// name, or the `HRESULT`.
    ///
    /// # Errors
    /// The `HRESULT` when there is no such device.
    fn input_open(&self, handle: u32, guid: u128) -> Result<String, i32>;
    /// Whether `handle` has a device of the platform's.
    fn input_has(&self, handle: u32) -> bool;
    /// The game released `handle`.
    fn input_release(&self, handle: u32);
    /// `GetDeviceStatus(guid)` (`None`: no devices).
    fn input_status(&self, guid: u128) -> Option<i32>;
    /// The attached game controllers' `DIDEVICEINSTANCEA`s (0x244 bytes
    /// each) for `EnumDevices(DI8DEVCLASS_GAMECTRL, flags)`.
    fn input_controllers(&self, flags: u32) -> Vec<Vec<u8>>;
    /// `EnumObjects(flags)`: each `DIDEVICEOBJECTINSTANCEA` (0x13c bytes;
    /// `None`: no device).
    fn input_objects(&self, handle: u32, flags: u32) -> Option<Vec<Vec<u8>>>;
    /// `SetDataFormat` (the game's format, read from the arena).
    fn input_set_format(&self, handle: u32, format: &crate::input::Format) -> Option<i32>;
    /// `SetCooperativeLevel(window, flags)` (the platform's window handle).
    fn input_set_cooperative_level(&self, handle: u32, window: u32, flags: u32) -> Option<i32>;
    /// `SetProperty`/`GetProperty` (`set`) on the property's bytes.
    fn input_property(
        &self,
        handle: u32,
        prop: &Property,
        data: &mut [u8],
        set: bool,
    ) -> Option<i32>;
    /// `Acquire`, `Unacquire`, `Poll`.
    fn input_call(&self, handle: u32, call: DeviceCall) -> Option<i32>;
    /// `GetDeviceState(size)`: the state, or the `HRESULT`.
    fn input_state(&self, handle: u32, size: usize) -> Option<Result<Vec<u8>, i32>>;
    /// `GetDeviceData(flags, max)`: buffered items (offset, data, time
    /// stamp, sequence, app data) and the `HRESULT`, or the failure.
    #[allow(clippy::type_complexity)] // items and the HRESULT
    fn input_data(
        &self,
        handle: u32,
        flags: u32,
        max: u32,
    ) -> Option<Result<(Vec<[u32; 5]>, i32), i32>>;
    /// `GetCapabilities` / `GetDeviceInfo` into the game's struct.
    fn input_info(&self, handle: u32, what: DeviceInfo, data: &mut [u8]) -> Option<i32>;
    /// `XInputGetState(user)`: `XINPUT_STATE`, or the error
    /// (`ERROR_DEVICE_NOT_CONNECTED`).
    ///
    /// # Errors
    /// No pad there.
    fn xinput_state(&self, user: u32) -> Result<[u8; 16], u32>;
    /// `XInputSetState(user, vibration)`.
    fn xinput_set_state(&self, user: u32, vibration: [u8; 4]) -> u32;
    /// The `XInput` pads' vendor and product ids (the game's
    /// `Input_IsXInputDevice` asks).
    fn xinput_ids(&self) -> Vec<(u32, u32)>;

    // ---- sound (the mixer's threads, `crate::audio`) ---------------------
    //
    // The mix is the host's (`ffx_sound`'s mixer, FMOD's 48,000 Hz
    // interleaved stereo f32, 1,600 frames a block); the platform only
    // plays it. A player's run on the real clock mixes on a feeder thread
    // that keeps `audio_queued` above the device's prefill; otherwise each
    // `EventSystem::update` pushes one block. Neither waits for the device.

    /// Starts the device output (once, on the first FMOD call that needs
    /// the mixer). It may fail or be switched off: [`Platform::audio_running`]
    /// says.
    fn audio_start(&self);
    /// Whether the device output runs (the feeder follows its queue then,
    /// else the wall clock).
    fn audio_running(&self) -> bool;
    /// Frames queued for the device and not played yet.
    fn audio_queued(&self) -> usize;
    /// Underruns so far (the device wanted frames that weren't there).
    fn audio_underruns(&self) -> u64;
    /// Frames the feeder keeps queued for the device (default
    /// [`crate::audio::FEED_TARGET`], 6,400: the WASAPI output's prefill and
    /// a block); a platform whose output buffers little itself may ask
    /// for less (lower latency) and must then start its output at that.
    fn audio_target(&self) -> usize {
        crate::audio::FEED_TARGET
    }
    /// One mixed block (interleaved stereo) for the device; never waits.
    fn audio_push(&self, block: &[f32]);

    // ---- Steam (the game's main thread, `crate::steam`) ------------------

    /// The Steam library behind the game's `steam_api.dll` imports and
    /// interfaces, loaded at the game's `SteamAPI_Init`, or why there is
    /// none (Steam is then absent, as `FFX.exe` without Steam: the game
    /// never asks for an interface). `crate::steam` keeps the game's
    /// side: the interface handles, the callback objects' order, the calls
    /// never forwarded (`SteamAPI_RestartAppIfNecessary`, `ResetAllStats`).
    ///
    /// # Errors
    /// Why there is no Steam (a note).
    fn steam(&self) -> Result<Box<dyn crate::steam::Backend>, String>;

    // ---- the player's folders and dialogs (any thread) -------------------

    /// The documents folder `FFX.exe` uses (`GameSetting.ini` and the saves
    /// under `Square Enix\FINAL FANTASY X&X-2 HD Remaster`), so the
    /// player's settings and saves are shared with the original; `None`:
    /// none. `ffx-launch` asks Windows the same and passes it as
    /// `--documents`; a frontend that starts the game itself asks here.
    fn documents_folder(&self) -> Option<std::path::PathBuf>;
    /// A problem shown to the player, in the player's words (the program's
    /// own; the game's `MessageBoxW` stays a note, as it is in a run of
    /// `FFX.exe` nobody watches). Returns when the player closed it.
    fn message_box(&self, title: &str, text: &str, warning: bool);

    // ---- the process (any thread) ----------------------------------------

    /// Ends the process at once with `code`, without the game's own
    /// shutdown (the harness's `quit`: the game's shutdown from the
    /// harness thread can deadlock).
    fn end_process(&self, code: u32) -> !;
}

static PLATFORM: OnceLock<Box<dyn Platform>> = OnceLock::new();

/// Installs the process's platform (before the boot: the first
/// [`get`] fixes it). `Err` gives `platform` back when one is in place
/// already.
///
/// # Errors
/// When a platform was installed or chosen before.
pub fn install(platform: Box<dyn Platform>) -> Result<(), Box<dyn Platform>> {
    PLATFORM.set(platform)
}

/// The process's platform: the one [`install`]ed, else [`choose`]'s.
#[must_use]
pub fn get() -> &'static dyn Platform {
    PLATFORM.get_or_init(choose).as_ref()
}

/// The platform `FFX_HOST_PLATFORM` names (`windows`, `headless`), else
/// the default: Windows on Windows, headless elsewhere. The native
/// platform is never chosen here: it needs the main thread for its event
/// loop, so only [`run_main`] installs it.
#[must_use]
pub fn choose() -> Box<dyn Platform> {
    let asked = std::env::var("FFX_HOST_PLATFORM").unwrap_or_default();
    match asked.as_str() {
        "headless" => return Box::new(headless::HeadlessPlatform),
        "" | "windows" => {}
        "native" => crate::log::note(
            "FFX_HOST_PLATFORM=native: only ffx-rust's own start (platform::run_main) makes it; the default",
        ),
        other => crate::log::note(&format!(
            "FFX_HOST_PLATFORM={other}: not a platform (windows, native, headless); the default"
        )),
    }
    #[cfg(windows)]
    {
        Box::new(windows::WindowsPlatform)
    }
    #[cfg(not(windows))]
    {
        if asked == "windows" {
            crate::log::note("FFX_HOST_PLATFORM=windows: Windows only; headless");
        }
        Box::new(headless::HeadlessPlatform)
    }
}

/// Runs a program's game (`game`, which boots and runs it) on the
/// platform its system gives a player: off Windows the native platform
/// when [`native::wanted`] says so (a desktop, not a harness game, not the
/// null device; `FFX_HOST_PLATFORM=native|headless` decides otherwise),
/// with its event loop on this thread, which must be the process's main
/// thread, and `game` on a thread of its own; anywhere else `game` here,
/// on the platform [`get`] picks. `game`'s exit code.
pub fn run_main<F>(game: F) -> std::process::ExitCode
where
    F: FnOnce() -> std::process::ExitCode + Send + 'static,
{
    #[cfg(not(windows))]
    if native::wanted() {
        return native::run(game);
    }
    game()
}
