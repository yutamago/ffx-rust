//! The live link to the external renderer (`ffx-render live`).
//!
//! With `FFX_RENDERER` set, the hook starts the renderer and shares two
//! rings with it (`ffx_gfx::ring`): game → renderer carries the scene
//! stream, renderer → game the finished images. Each frame the scene
//! draws (the ones into the colour and normal targets with depth, before
//! the glow, effect and menu passes) are sent with everything they use;
//! objects go once, textures and buffers again when written since. When
//! the game moves on from the scene targets, the hook waits for the
//! renderer's colour, normal and depth images and draws them over the
//! game's own in one full-screen pass, then the game carries on with its
//! effects and menus on top.
//!
//! `FFX_RENDERER=1` uses `ffx-render.exe` next to the game; another value
//! is the renderer's path.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use ffx_gfx::ring::Ring;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_TIMEOUT};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_ALL_ACCESS, MapViewOfFile, PAGE_READWRITE,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CreateProcessW, GetCurrentProcessId, PROCESS_INFORMATION, STARTUPINFOW,
    WaitForSingleObject,
};

use super::{Draw, State, capture, device_original, real_method, vt};
use crate::log::log;

type Ptr = *mut c_void;

static ON: AtomicBool = AtomicBool::new(false);
/// The overlay is showing (F10): the mouse is its own.
static OVERLAY: AtomicBool = AtomicBool::new(false);
/// The overlay takes the keyboard (a text field has focus, or the pointer
/// is over a panel): the game's keyboard reads as released.
static OVERLAY_KEYS: AtomicBool = AtomicBool::new(false);
/// The renderer wants the glowing effects drawn after the scene (it makes
/// lights of them).
static WANT_EFFECTS: AtomicBool = AtomicBool::new(false);
/// The renderer's settings turn the game's afterimage off.
static NO_AFTERIMAGE: AtomicBool = AtomicBool::new(false);
/// The renderer has the game paused (its sequence editor; `game_pause`).
static PAUSED: AtomicBool = AtomicBool::new(false);
/// The renderer's layer (the overlay's image) shows though the overlay's
/// panels are hidden: its HUD (the minimap).
static LAYER: AtomicBool = AtomicBool::new(false);
/// The renderer holds the pointer (its free camera's walk mode): kept at
/// the window's centre, hidden, its moves sent as `Input::Motion`.
static HOLD_POINTER: AtomicBool = AtomicBool::new(false);
/// The renderer's free camera (eye, look-at in game space, projection
/// distance, roll), put into the game's override camera at Present; and
/// whether the override is ours to turn off.
static FREE_CAMERA: Mutex<Option<[f32; 8]>> = Mutex::new(None);
static FREE_CAMERA_SET: AtomicBool = AtomicBool::new(false);
/// The pixel shader of the game's afterimage pass (by its bytecode hash):
/// the scene, times a colour whose alpha is the new frame's share
/// (0.8 on the S.S. Winno), blended into a history image that the next
/// draw lays over the scene.
const AFTERIMAGE_PS: u64 = 0xe928_905e_3163_6cf1;

/// Whether the draw about to happen is the afterimage's blend into its
/// history, and the renderer's settings want each frame to stand alone:
/// then it goes without blending (the history is just this frame).
pub fn afterimage_off(s: &State) -> bool {
    on() && NO_AFTERIMAGE.load(Ordering::Relaxed)
        && s.shaders
            .get(&s.bound.ps)
            .is_some_and(|sh| sh.hash == AFTERIMAGE_PS)
}
/// Window input for the overlay, sent with the next frame.
static INPUT: Mutex<Vec<ffx_gfx::Input>> = Mutex::new(Vec::new());
static WINDOW: AtomicUsize = AtomicUsize::new(0);
static WNDPROC: AtomicUsize = AtomicUsize::new(0);
/// The back buffer's size.
static SCREEN: Mutex<(u32, u32)> = Mutex::new((0, 0));

/// The game's own pause (`Pause_Enter` 0x008ab3d0, `Pause_Update`
/// 0x008ab340): its flag, set when it is entered, the fade's step, and the
/// pause screen's hide.
const PAUSE_ON: u32 = 0x0134_0828;
const PAUSE_ENTERED: u32 = 0x0134_084c;
const PAUSE_STEP: u32 = 0x0134_083c;
const PAUSE_HIDE: u32 = 0x006f_32e0;
/// 0: not ours, 1: asked for, 2: entered and its screen hidden.
static PAUSE_STATE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// The renderer's pause through the game's own: setting its flag makes
/// `Pause_Update` enter it (world update stopped, sound paused, the image
/// dimmed; the "PAUSE" text is hidden again); to end it, the fade goes
/// up as when Start is pressed, and the game resumes by itself.
fn game_pause() {
    let want = PAUSED.load(Ordering::Acquire);
    let state = PAUSE_STATE.load(Ordering::Relaxed);
    let at = |a: u32| crate::rebase(a) as *mut u32;
    // SAFETY: the game's pause globals and its function, on the thread that
    // presents (the harness calls game functions there too).
    unsafe {
        match (want, state) {
            (true, 0) => {
                std::ptr::write_volatile(at(PAUSE_ON), 1);
                PAUSE_STATE.store(1, Ordering::Relaxed);
                log("  live: game paused");
            }
            (true, 1) if std::ptr::read_volatile(at(PAUSE_ENTERED)) == 1 => {
                let hide: extern "C" fn() = std::mem::transmute(crate::rebase(PAUSE_HIDE));
                hide();
                PAUSE_STATE.store(2, Ordering::Relaxed);
            }
            (false, 1 | 2) => {
                if std::ptr::read_volatile(at(PAUSE_ON)) == 1 {
                    std::ptr::write_volatile(at(PAUSE_STEP), 4);
                }
                PAUSE_STATE.store(0, Ordering::Relaxed);
                log("  live: game resumed");
            }
            _ => {}
        }
    }
}

/// Wraps `MsScreenProcess` (`0x007bc110`, `cdecl(camera)`, which builds the view
/// matrix; its Rust port, `ffx_battle::screen`, runs in its place): while
/// the renderer has a free camera, the game's override
/// camera (`MsEffectCameraSet`) holds it right before, whatever the scene's
/// camera scripts set since; and after, the view goes to the renderer as
/// the channel's would (`Cam_Apply`, which the override's branch skips,
/// calls `graphicCameraSetFOVy` and `graphicCameraSetWorldMatrix`): the field of view the
/// renderer asks for (the game's, zoomed).
///
/// # Safety
/// Called by the game.
pub unsafe extern "C" fn cam_update(camera: i32) -> u32 {
    const VIEW: u32 = 0x0231_1440;
    const RENDER_SET_VIEW: u32 = 0x0063_9860;
    const RENDER_SET_FOV: u32 = 0x0063_95e0;
    let free = FREE_CAMERA.lock().ok().and_then(|f| *f);
    if let Some(c) = &free {
        // SAFETY: on the game's thread, before its camera update reads them.
        unsafe { put_free_camera(c) };
    }
    let out = crate::ported::ms_screen_process(camera);
    if let Some(c) = free {
        // SAFETY: the game's functions and its view matrix global, as
        // `Cam_Apply` (0x007bc980) calls them.
        unsafe {
            let set_fov: unsafe extern "C" fn(*const f32) =
                std::mem::transmute(crate::rebase(RENDER_SET_FOV));
            set_fov(&raw const c[6]);
            let set_view: unsafe extern "C" fn(usize) =
                std::mem::transmute(crate::rebase(RENDER_SET_VIEW));
            set_view(crate::rebase(VIEW));
        }
    }
    out
}

const OVERRIDE_ON: u32 = 0x0231_142c;

/// The free camera into the game's override camera globals (in its image,
/// rebased).
///
/// # Safety
/// On the game's thread, between its frames or right before its camera
/// update.
unsafe fn put_free_camera(c: &[f32; 8]) {
    const EYE: u32 = 0x0113_6fc8;
    const LOOK_AT: u32 = 0x0113_6fd8;
    const ROLL: u32 = 0x0113_6fc4;
    let at = |a: u32| crate::rebase(a) as *mut f32;
    // SAFETY: the game's camera globals.
    unsafe {
        for k in 0..3 {
            std::ptr::write_volatile(at(EYE).add(k), c[k]);
            std::ptr::write_volatile(at(LOOK_AT).add(k), c[3 + k]);
        }
        std::ptr::write_volatile(at(ROLL), c[7]);
        std::ptr::write_volatile(crate::rebase(OVERRIDE_ON) as *mut u32, 1);
    }
}

/// At Present: the free camera's start and end (the override turned off
/// once when it ends).
fn free_camera() {
    let free = FREE_CAMERA.lock().ok().and_then(|f| *f);
    match free {
        Some(c) => {
            // SAFETY: at Present, on the game's thread.
            unsafe { put_free_camera(&c) };
            if !FREE_CAMERA_SET.swap(true, Ordering::Relaxed) {
                log(&format!("  live: free camera on at {:?}", &c[..3]));
            }
        }
        None => {
            if FREE_CAMERA_SET.swap(false, Ordering::Relaxed) {
                // SAFETY: the game's flag, on its thread.
                unsafe { std::ptr::write_volatile(crate::rebase(OVERRIDE_ON) as *mut u32, 0) };
                log("  live: free camera off");
            }
        }
    }
}

/// Captures the next frame (as the harness's `gfxcapture` does) into
/// `captures/<seconds since 1970>` beside the game, on a thread of its own:
/// the capture is written by the render thread.
fn capture_now() {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let Some(dir) = std::env::current_exe().ok().and_then(|p| {
        p.parent()
            .map(|d| d.join("captures").join(secs.to_string()))
    }) else {
        return;
    };
    log(&format!("  live: capturing a frame into {}", dir.display()));
    std::thread::spawn(move || {
        let reply = crate::gfx::capture(&dir.to_string_lossy());
        log(&format!("  live: capture {}: {reply}", dir.display()));
    });
}

/// Shows or hides the overlay, as F10 does (for the test harness).
pub fn toggle_overlay() -> bool {
    let now = !OVERLAY.load(Ordering::Acquire);
    OVERLAY.store(now, Ordering::Release);
    OVERLAY_KEYS.store(false, Ordering::Release);
    push_input(ffx_gfx::Input::Toggle);
    now
}

/// Opens or closes the editor, as F11 does (for the test harness).
pub fn toggle_editor() {
    if !OVERLAY.swap(true, Ordering::AcqRel) {
        OVERLAY_KEYS.store(false, Ordering::Release);
        push_input(ffx_gfx::Input::Toggle);
    }
    push_input(ffx_gfx::Input::Key {
        vk: 0x7a,
        pressed: true,
    });
}

/// A click at back buffer pixel (x, y), for the test harness.
pub fn click(x: f32, y: f32) {
    push_input(ffx_gfx::Input::Move(x, y));
    push_input(ffx_gfx::Input::Button {
        button: 0,
        pressed: true,
    });
    push_input(ffx_gfx::Input::Button {
        button: 0,
        pressed: false,
    });
}

/// Moves the pointer in the overlay, and presses or releases the left
/// button there (for the test harness: drags take several frames).
pub fn mouse(x: f32, y: f32, button: Option<bool>) {
    push_input(ffx_gfx::Input::Move(x, y));
    if let Some(pressed) = button {
        push_input(ffx_gfx::Input::Button { button: 0, pressed });
    }
}

/// A key for the overlay (Windows virtual-key code; for the test
/// harness).
pub fn overlay_key(vk: u32, pressed: bool) {
    push_input(ffx_gfx::Input::Key { vk, pressed });
}

/// Whether the overlay is showing.
pub fn overlay_visible() -> bool {
    OVERLAY.load(Ordering::Acquire)
}

/// Whether the overlay is showing and takes the keyboard (else the keys go
/// to the game, overlay or not).
pub fn overlay_takes_keys() -> bool {
    overlay_visible() && OVERLAY_KEYS.load(Ordering::Acquire)
}

fn push_input(e: ffx_gfx::Input) {
    if let Ok(mut q) = INPUT.lock() {
        q.push(e);
    }
}

/// The game window: subclassed for the overlay's hotkey and input.
///
/// # Safety
/// `hwnd` must be the game's window, on its own thread's message loop.
pub unsafe fn set_window(hwnd: usize, width: u32, height: u32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GWL_WNDPROC, SetWindowLongW};
    if let Ok(mut s) = SCREEN.lock() {
        *s = (width, height);
    }
    if hwnd == 0 || WINDOW.swap(hwnd, Ordering::AcqRel) != 0 {
        return;
    }
    // SAFETY: replacing the window procedure of the game's window; the old
    // one is kept and called for everything the overlay does not take.
    let old = unsafe { SetWindowLongW(hwnd as _, GWL_WNDPROC, window_proc as *const () as i32) };
    WNDPROC.store(old.cast_unsigned() as usize, Ordering::Release);
    log(&format!(
        "  live: window {hwnd:#x} subclassed for the overlay (F10)"
    ));
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)] // LPARAM packs two signed 16-bit coordinates; one switch over the messages
unsafe extern "system" fn window_proc(
    hwnd: *mut c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, IDC_ARROW, LoadCursorW, SetCursor, ShowCursor,
    };
    const F10: usize = 0x79;
    const F11: usize = 0x7a;
    const F12: usize = 0x7b;
    let visible = OVERLAY.load(Ordering::Acquire);
    match msg {
        // Ctrl+F12: the next frame captured into `captures/` beside the
        // game (for a look at what the renderer got wrong there).
        0x100 | 0x104
            if wparam == F12
            // SAFETY: a plain key state query.
                && unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x11) } < 0 =>
        {
            if lparam & (1 << 30) == 0 {
                capture_now();
            }
            return 0;
        }
        // F11: the editor (and the overlay with it, if hidden).
        0x100 | 0x104 if wparam == F11 => {
            if lparam & (1 << 30) == 0 {
                if !visible {
                    OVERLAY.store(true, Ordering::Release);
                    OVERLAY_KEYS.store(false, Ordering::Release);
                    push_input(ffx_gfx::Input::Toggle);
                    // SAFETY: on the window's thread.
                    unsafe { ShowCursor(1) };
                }
                push_input(ffx_gfx::Input::Key {
                    vk: F11 as u32,
                    pressed: true,
                });
            }
            return 0;
        }
        0x101 | 0x105 if wparam == F11 => return 0,
        // WM_KEYDOWN, WM_SYSKEYDOWN with F10: show or hide.
        0x100 | 0x104 if wparam == F10 => {
            if lparam & (1 << 30) == 0 {
                let now = !visible;
                OVERLAY.store(now, Ordering::Release);
                OVERLAY_KEYS.store(false, Ordering::Release);
                push_input(ffx_gfx::Input::Toggle);
                // SAFETY: on the window's thread.
                unsafe { ShowCursor(i32::from(now)) };
            }
            return 0;
        }
        0x101 | 0x105 if wparam == F10 => return 0,
        _ => {}
    }
    if visible {
        let point = || {
            let x = f32::from(((lparam & 0xffff) as u16).cast_signed());
            let y = f32::from((((lparam >> 16) & 0xffff) as u16).cast_signed());
            client_to_screen(hwnd, x, y)
        };
        let held = HOLD_POINTER.load(Ordering::Acquire);
        let taken = match msg {
            0x200 if held => {
                // SAFETY: the game's window, on its thread.
                if let Some((dx, dy)) = unsafe { recentre(hwnd) } {
                    push_input(ffx_gfx::Input::Motion(dx, dy));
                }
                true
            }
            0x200 => {
                let (x, y) = point();
                push_input(ffx_gfx::Input::Move(x, y));
                true
            }
            0x201 | 0x202 | 0x204 | 0x205 | 0x207 | 0x208 => {
                let (x, y) = point();
                push_input(ffx_gfx::Input::Move(x, y));
                let button = match msg {
                    0x204 | 0x205 => 1,
                    0x207 | 0x208 => 2,
                    _ => 0,
                };
                push_input(ffx_gfx::Input::Button {
                    button,
                    pressed: matches!(msg, 0x201 | 0x204 | 0x207),
                });
                true
            }
            0x20a => {
                let delta = f32::from((((wparam >> 16) & 0xffff) as u16).cast_signed()) / 120.0;
                push_input(ffx_gfx::Input::Wheel(delta));
                true
            }
            // Keys reach the overlay always, the game too unless the
            // overlay takes them.
            0x100 | 0x101 => {
                push_input(ffx_gfx::Input::Key {
                    vk: wparam as u32,
                    pressed: msg == 0x100,
                });
                overlay_takes_keys()
            }
            // With Alt (Alt+F4 still closes the game).
            0x104 | 0x105 if wparam != 0x73 => {
                push_input(ffx_gfx::Input::Key {
                    vk: wparam as u32,
                    pressed: msg == 0x104,
                });
                overlay_takes_keys()
            }
            0x102 => {
                push_input(ffx_gfx::Input::Char(wparam as u32));
                overlay_takes_keys()
            }
            // WM_SETCURSOR: an arrow over the client area (none while
            // the renderer holds the pointer).
            0x20 => {
                // SAFETY: plain cursor calls on the window's thread.
                unsafe {
                    SetCursor(if held {
                        std::ptr::null_mut()
                    } else {
                        LoadCursorW(std::ptr::null_mut(), IDC_ARROW)
                    });
                };
                true
            }
            _ => false,
        };
        if taken {
            return isize::from(msg == 0x20);
        }
    }
    // SAFETY: the window's original procedure.
    unsafe {
        let old: windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC =
            std::mem::transmute(WNDPROC.load(Ordering::Acquire));
        CallWindowProcW(old, hwnd, msg, wparam, lparam)
    }
}

/// Client coordinates to back buffer pixels.
/// The held pointer: how far it is from the window's centre (in the
/// renderer's screen pixels), and back to the centre.
#[allow(clippy::cast_precision_loss)] // window sizes, far below 2^24
unsafe fn recentre(hwnd: *mut c_void) -> Option<(f32, f32)> {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, GetCursorPos, SetCursorPos};
    let mut r = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut at = POINT { x: 0, y: 0 };
    // SAFETY: the game's window; plain cursor calls.
    unsafe {
        GetClientRect(hwnd, &raw mut r);
        let mut centre = POINT {
            x: (r.right - r.left) / 2,
            y: (r.bottom - r.top) / 2,
        };
        ClientToScreen(hwnd, &raw mut centre);
        GetCursorPos(&raw mut at);
        let (dx, dy) = (at.x - centre.x, at.y - centre.y);
        if dx == 0 && dy == 0 {
            // The move that putting it back made.
            return None;
        }
        SetCursorPos(centre.x, centre.y);
        let (cw, ch) = (
            (r.right - r.left).max(1) as f32,
            (r.bottom - r.top).max(1) as f32,
        );
        let (sw, sh) = SCREEN.lock().map_or((cw, ch), |s| (s.0 as f32, s.1 as f32));
        Some((dx as f32 * sw / cw, dy as f32 * sh / ch))
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // window sizes, far below 2^24
fn client_to_screen(hwnd: *mut c_void, x: f32, y: f32) -> (f32, f32) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;
    let mut r = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: the game's window.
    unsafe { GetClientRect(hwnd, &raw mut r) };
    let (cw, ch) = (
        (r.right - r.left).max(1) as f32,
        (r.bottom - r.top).max(1) as f32,
    );
    let (sw, sh) = SCREEN.lock().map_or((cw as u32, ch as u32), |s| *s);
    (x * sw as f32 / cw, y * sh as f32 / ch)
}
static SENT_FRAMES: AtomicU32 = AtomicU32::new(0);
static IMAGES: AtomicU32 = AtomicU32::new(0);
static MISSES: AtomicU32 = AtomicU32::new(0);

/// Frames sent, images composited, frames without an image.
pub fn stats() -> (u32, u32, u32) {
    (
        SENT_FRAMES.load(Ordering::Relaxed),
        IMAGES.load(Ordering::Relaxed),
        MISSES.load(Ordering::Relaxed),
    )
}

/// Bytes of the game → renderer ring.
pub const TO_RENDERER: usize = 256 << 20;
/// Bytes of the renderer → game ring.
pub const TO_GAME: usize = 64 << 20;

/// Whether the live link is up.
pub fn on() -> bool {
    ON.load(Ordering::Acquire)
}

/// The scene targets: colour, normal, depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Scene {
    color: usize,
    normal: usize,
    depth: usize,
}

struct Composite {
    width: u32,
    height: u32,
    textures: [Ptr; 3],
    views: [Ptr; 3],
    vs: Ptr,
    ps: Ptr,
    depth_state: Ptr,
    blend: Ptr,
    raster: Ptr,
}

/// The renderer's shared scene images, opened on the game's device: per
/// set, the textures and their views (colour, normal, depth).
struct SharedImages {
    generation: u32,
    sets: [Option<([Ptr; 3], [Ptr; 3])>; 3],
}

/// The renderer's shared fence, opened on the game's device: its handle
/// here and the `ID3D11Fence`.
struct SharedFence {
    handle: u32,
    fence: Ptr,
}

/// Opens the renderer's fence (`ID3D11Device5::OpenSharedFence`).
unsafe fn open_fence(handle: u32) -> Option<Ptr> {
    const IID_FENCE: [u8; 16] = super::guid(
        0xaffd_e9d1,
        0x1df7,
        0x4bb7,
        [0x8a, 0x34, 0x0f, 0x46, 0x25, 0x1d, 0xab, 0x80],
    );
    let device = super::DEVICE.load(Ordering::Acquire) as Ptr;
    // SAFETY: the game's device; ID3D11Device5 from QueryInterface.
    unsafe {
        type Query = unsafe extern "system" fn(Ptr, *const u8, *mut Ptr) -> i32;
        let query: Query = std::mem::transmute(real_method(device, 0));
        let mut device5: Ptr = std::ptr::null_mut();
        if query(device, super::DEVICE_IIDS[5].as_ptr(), &raw mut device5) < 0 {
            log("  live: no ID3D11Device5 for the shared fence");
            return None;
        }
        type Open = unsafe extern "system" fn(Ptr, usize, *const u8, *mut Ptr) -> i32;
        let open: Open = std::mem::transmute(real_method(device5, 67));
        let mut fence: Ptr = std::ptr::null_mut();
        let hr = open(device5, handle as usize, IID_FENCE.as_ptr(), &raw mut fence);
        release(device5);
        if hr < 0 {
            log(&format!("  live: cannot open the shared fence ({hr:#x})"));
            return None;
        }
        Some(fence)
    }
}

/// Makes the game's GPU wait until `fence` reaches `value`
/// (`ID3D11DeviceContext4::Wait`).
unsafe fn gpu_wait(fence: Ptr, value: u64) -> bool {
    let ctx = super::CONTEXT.load(Ordering::Acquire) as Ptr;
    // SAFETY: the game's immediate context; ID3D11DeviceContext4 from
    // QueryInterface.
    unsafe {
        type Query = unsafe extern "system" fn(Ptr, *const u8, *mut Ptr) -> i32;
        let query: Query = std::mem::transmute(real_method(ctx, 0));
        let mut ctx4: Ptr = std::ptr::null_mut();
        if query(ctx, super::CONTEXT_IIDS[4].as_ptr(), &raw mut ctx4) < 0 {
            return false;
        }
        type Wait = unsafe extern "system" fn(Ptr, Ptr, u64) -> i32;
        let wait: Wait = std::mem::transmute(real_method(ctx4, 148));
        let hr = wait(ctx4, fence, value);
        release(ctx4);
        hr >= 0
    }
}

impl SharedImages {
    /// Releases everything opened.
    unsafe fn release(&mut self) {
        for (textures, views) in self.sets.iter_mut().filter_map(Option::take) {
            for p in views.into_iter().chain(textures) {
                // SAFETY: references this holds.
                unsafe { release(p) };
            }
        }
    }
}

/// `IUnknown::Release`.
unsafe fn release(object: Ptr) {
    if object.is_null() {
        return;
    }
    // SAFETY: a live COM object.
    unsafe {
        let f: unsafe extern "system" fn(Ptr) -> u32 = std::mem::transmute(real_method(object, 2));
        f(object);
    }
}

/// Opens set `set` of generation `generation` of the renderer's shared
/// images: textures and shader resource views.
unsafe fn open_shared(generation: u32, set: u32) -> Option<([Ptr; 3], [Ptr; 3])> {
    const IID_TEXTURE2D: [u8; 16] = [
        0xf2, 0xaa, 0x15, 0x6f, 0x08, 0xd2, 0x89, 0x4e, 0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3, 0x4f,
        0x9c,
    ];
    let device = super::DEVICE.load(Ordering::Acquire) as Ptr;
    let pid = std::process::id();
    // SAFETY: the game's device; ID3D11Device1 from QueryInterface.
    unsafe {
        type Query = unsafe extern "system" fn(Ptr, *const u8, *mut Ptr) -> i32;
        let query: Query = std::mem::transmute(real_method(device, 0));
        let mut device1: Ptr = std::ptr::null_mut();
        if query(device, super::DEVICE_IIDS[1].as_ptr(), &raw mut device1) < 0 {
            return None;
        }
        // ID3D11Device1::OpenSharedResourceByName.
        type Open = unsafe extern "system" fn(Ptr, *const u16, u32, *const u8, *mut Ptr) -> i32;
        let open: Open = std::mem::transmute(real_method(device1, 49));
        type CreateSrv = unsafe extern "system" fn(Ptr, Ptr, *const u32, *mut Ptr) -> i32;
        let create_srv: CreateSrv = std::mem::transmute(device_original(
            device,
            vt::device::CREATE_SHADER_RESOURCE_VIEW,
        ));
        let mut textures = [std::ptr::null_mut(); 3];
        let mut views = [std::ptr::null_mut(); 3];
        let mut ok = true;
        for plane in 0..3 {
            let name = wide(&format!("Local\\ffx-gfx-{pid}-{generation}-{set}-{plane}"));
            // DXGI_SHARED_RESOURCE_READ
            if open(
                device1,
                name.as_ptr(),
                0x8000_0000,
                IID_TEXTURE2D.as_ptr(),
                &raw mut textures[plane],
            ) < 0
                || create_srv(
                    device,
                    textures[plane],
                    std::ptr::null(),
                    &raw mut views[plane],
                ) < 0
            {
                ok = false;
                break;
            }
        }
        release(device1);
        if !ok {
            log(&format!(
                "  live: cannot open shared images {generation}/{set}"
            ));
            for p in views.into_iter().chain(textures) {
                release(p);
            }
            return None;
        }
        Some((textures, views))
    }
}

struct Link {
    to_renderer: Ring,
    to_game: Ring,
    process: HANDLE,
    /// What the renderer has: id → version.
    sent: HashMap<usize, u32>,
    /// This frame's commands for the renderer.
    cmds: Vec<ffx_gfx::Cmd>,
    clears: Vec<ffx_gfx::Cmd>,
    scene: Option<Scene>,
    /// The scene sent this frame, and the glowing effects the game has
    /// drawn over it since (for the renderer at Present).
    after: Option<Scene>,
    effects: Vec<ffx_gfx::Cmd>,
    frame: u32,
    composite: Option<Composite>,
    shared: Option<SharedImages>,
    fence: Option<SharedFence>,
    /// Frames the renderer did not answer in time.
    misses: u32,
    /// The overlay image laid over each frame at Present while it shows:
    /// the renderer sends the parts that change.
    overlay: Option<OverlayImage>,
    overlay_pass: Option<OverlayPass>,

    /// Whether the renderer answered this frame already.
    answered: bool,
    /// Skip the game's own scene draws (the renderer's replace them).
    skip: bool,
    /// Frames answered in a row, for turning skipping back on.
    streak: u32,
    /// Textures the renderer makes itself (copies): sent without data.
    made: std::collections::HashSet<usize>,
    /// Contents of the index buffers the renderer has, for finding the
    /// vertices a draw reads from a streamed vertex buffer.
    indices: HashMap<usize, Vec<u8>>,
    /// Objects (re)sent since the last report: (kind, bytes, time).
    resent: HashMap<&'static str, (u32, usize, std::time::Duration)>,
    alive: bool,
}

// SAFETY: the raw pointers are D3D objects used on the render thread only.
unsafe impl Send for Link {}

static LINK: Mutex<Option<Link>> = Mutex::new(None);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// Starts the renderer when `FFX_RENDERER` is set. Called once the device
/// exists.
pub fn start() {
    let Ok(setting) = std::env::var("FFX_RENDERER") else {
        return;
    };
    if setting.is_empty() || setting == "0" {
        return;
    }
    let exe = if setting == "1" {
        std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|d| d.join("ffx-render.exe")))
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    } else {
        setting
    };
    // SAFETY: plain Win32 calls; the mapping lives as long as the process.
    unsafe {
        let pid = GetCurrentProcessId();
        let name = format!("Local\\ffx-gfx-{pid}");
        let size = TO_RENDERER + TO_GAME;
        let mapping = CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            std::ptr::null(),
            PAGE_READWRITE,
            0,
            size as u32,
            wide(&name).as_ptr(),
        );
        if mapping.is_null() {
            log("  live: CreateFileMapping failed");
            return;
        }
        let view = MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, size);
        if view.Value.is_null() {
            log("  live: MapViewOfFile failed");
            return;
        }
        let base = view.Value.cast::<u8>();
        let to_renderer = Ring::create(base, TO_RENDERER);
        let to_game = Ring::create(base.add(TO_RENDERER), TO_GAME);
        let mut command = wide(&format!("\"{exe}\" live {name} {pid}"));
        let startup = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..std::mem::zeroed()
        };
        let mut info: PROCESS_INFORMATION = std::mem::zeroed();
        let ok = CreateProcessW(
            std::ptr::null(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_NO_WINDOW,
            std::ptr::null(),
            std::ptr::null(),
            &raw const startup,
            &raw mut info,
        );
        if ok == 0 {
            log(&format!("  live: cannot start {exe}"));
            return;
        }
        CloseHandle(info.hThread);
        log(&format!(
            "  live: renderer {exe} started (pid {})",
            info.dwProcessId
        ));
        if let Ok(mut l) = LINK.lock() {
            *l = Some(Link {
                to_renderer,
                to_game,
                process: info.hProcess,
                sent: HashMap::new(),
                cmds: Vec::new(),
                clears: Vec::new(),
                scene: None,
                after: None,
                effects: Vec::new(),
                frame: 0,
                composite: None,
                shared: None,
                fence: None,
                misses: 0,
                resent: HashMap::new(),
                indices: HashMap::new(),
                overlay: None,
                overlay_pass: None,

                answered: false,
                skip: std::env::var("FFX_RENDERER_SKIP").map_or(true, |v| v != "0"),
                streak: 0,
                made: std::collections::HashSet::new(),
                alive: true,
            });
        }
        ON.store(true, Ordering::Release);
    }
}

impl Link {
    fn send(&mut self, msg: &ffx_gfx::Msg) {
        if !self.alive {
            return;
        }
        let mut bytes = Vec::new();
        msg.write(&mut bytes);
        let process = self.process;
        let mut alive = true;
        self.to_renderer.write(&bytes, || {
            // SAFETY: our process handle.
            if unsafe { WaitForSingleObject(process, 1) } != WAIT_TIMEOUT {
                alive = false;
            }
        });
        if !alive {
            log("  live: the renderer is gone");
            self.alive = false;
            // The game's own minimap again.
            crate::minimap::REPLACE.store(false, Ordering::Release);
            HOLD_POINTER.store(false, Ordering::Release);
        }
    }

    /// Sends what `id` refers to, if the renderer lacks it or it changed.
    /// Streamed buffers go once; draws carry the bytes they read.
    fn sync(&mut self, s: &State, id: usize) {
        if self.made.contains(&id) {
            if !self.sent.contains_key(&id)
                && let Some(t) = s.textures.get(&id)
            {
                self.send(&ffx_gfx::Msg::Texture {
                    id: id as u32,
                    desc: t.desc,
                    subresources: Vec::new(),
                });
                self.sent.insert(id, 0);
            }
            return;
        }
        if streamed(s, id) {
            if !self.sent.contains_key(&id)
                && let Some(b) = s.buffers.get(&id)
            {
                // The buffer itself; draws bring its bytes.
                self.send(&ffx_gfx::Msg::Buffer {
                    id: id as u32,
                    desc: b.desc,
                    data: Vec::new(),
                });
                self.sent.insert(id, 0);
            }
            return;
        }
        let version = s
            .textures
            .get(&id)
            .map(|t| t.version)
            .or_else(|| s.buffers.get(&id).map(|b| b.version))
            .unwrap_or(0);
        if self.sent.get(&id) == Some(&version) {
            return;
        }
        if let Some(v) = s.views.get(&id) {
            self.sync(s, v.resource);
            if !self.sent.contains_key(&id)
                && let Some(m) = capture::view_msg(s, id)
            {
                self.send(&m);
                self.sent.insert(id, 0);
            }
            return;
        }
        let t = std::time::Instant::now();
        if let Some(m) = capture::object_msg(s, id) {
            let (kind, bytes) = match &m {
                ffx_gfx::Msg::Texture { subresources, .. } => {
                    ("texture", subresources.iter().map(|x| x.data.len()).sum())
                }
                ffx_gfx::Msg::Buffer { data, .. } => ("buffer", data.len()),
                _ => ("other", 0),
            };
            if let ffx_gfx::Msg::Buffer { desc, data, .. } = &m
                && desc[2] & super::BIND_INDEX != 0
            {
                self.indices.insert(id, data.clone());
            }
            self.send(&m);
            let e = self.resent.entry(kind).or_default();
            e.0 += 1;
            e.1 += bytes;
            e.2 += t.elapsed();
            self.sent.insert(id, version);
        }
    }
}

/// Vertex and index buffers the game writes as it goes (dynamic ones, and
/// default ones it updates): each draw sends the part it reads.
fn streamed(s: &State, id: usize) -> bool {
    s.buffers.get(&id).is_some_and(|b| {
        b.desc[2] & (super::BIND_VERTEX | super::BIND_INDEX) != 0
            && (b.desc[1] == 2 || b.shadow.is_some())
    })
}

/// Bytes `from..to` of a streamed buffer: from the layer's copy, or for a
/// dynamic buffer from its mapped memory (read safely: only the bytes a
/// draw uses, write-combined memory is slow to read).
fn bytes(s: &State, id: usize, from: usize, to: usize) -> Option<Vec<u8>> {
    let b = s.buffers.get(&id)?;
    let to = to.min(b.desc[0] as usize);
    if from >= to {
        return None;
    }
    if let Some(sh) = &b.shadow {
        return sh.get(from..to).map(<[u8]>::to_vec);
    }
    if b.mapped_ptr == 0 {
        return None;
    }
    read_memory(b.mapped_ptr + from, to - from)
}

fn read_memory(address: usize, len: usize) -> Option<Vec<u8>> {
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut out = vec![0u8; len];
    let mut read = 0;
    // SAFETY: ReadProcessMemory validates the source range itself.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const c_void,
            out.as_mut_ptr().cast(),
            len,
            &raw mut read,
        )
    };
    (ok != 0 && read == len).then_some(out)
}

/// The smallest and largest index `count` indices from `start` use.
fn index_range(
    data: &[u8],
    format: u32,
    offset: u32,
    start: u32,
    count: u32,
) -> Option<(u32, u32)> {
    let size = if format == 42 { 4 } else { 2 };
    let from = offset as usize + start as usize * size;
    let bytes = data.get(from..from + count as usize * size)?;
    let mut lo = u32::MAX;
    let mut hi = 0;
    for c in bytes.chunks_exact(size) {
        let i = if size == 4 {
            u32::from_le_bytes([c[0], c[1], c[2], c[3]])
        } else {
            u32::from(u16::from_le_bytes([c[0], c[1]]))
        };
        lo = lo.min(i);
        hi = hi.max(i);
    }
    (lo <= hi).then_some((lo, hi))
}

/// The bytes of streamed buffers this draw reads, as updates to send
/// before it: from each binding's offset to the last byte used.
fn draw_updates(link: &Link, s: &State, d: &ffx_gfx::Draw) -> Vec<ffx_gfx::Cmd> {
    use ffx_gfx::DrawKind as K;
    let mut out = Vec::new();
    let indexed = matches!(d.kind, K::Indexed | K::IndexedInstanced);
    let (ib, ifmt, ioff) = d.index;
    let isize = if ifmt == 42 { 4 } else { 2 };
    let index_end = ioff as usize + (d.start + d.count) as usize * isize;
    // The indices: sent when streamed, and needed to find the vertices.
    let index_bytes = if indexed && streamed(s, ib as usize) {
        let b = bytes(s, ib as usize, ioff as usize, index_end);
        if let Some(b) = &b {
            out.push(ffx_gfx::Cmd::UpdateBuffer {
                id: ib,
                offset: ioff,
                data: b.clone(),
            });
        }
        b.map(|b| (b, 0u32))
    } else {
        link.indices.get(&(ib as usize)).map(|b| (b.clone(), ioff))
    };
    let vertices = if indexed {
        index_bytes
            .as_ref()
            .and_then(|(data, off)| index_range(data, ifmt, *off, d.start, d.count))
            .map(|(lo, hi)| (lo.cast_signed() + d.base, hi.cast_signed() + d.base))
    } else {
        Some((d.start.cast_signed(), (d.start + d.count).cast_signed() - 1))
    };
    let per_instance: Vec<u32> = s
        .layouts
        .get(&(d.layout as usize))
        .map(|l| {
            l.elements
                .iter()
                .filter(|e| e.per_instance)
                .map(|e| e.slot)
                .collect()
        })
        .unwrap_or_default();
    for &(slot, vb, stride, offset) in &d.vertex {
        if !streamed(s, vb as usize) {
            continue;
        }
        let last = if per_instance.contains(&u32::from(slot)) {
            (d.first_instance + d.instances.max(1)) as usize
        } else if let Some((_, hi)) = vertices {
            usize::try_from(hi.max(0)).unwrap_or(0) + 1
        } else {
            continue;
        };
        let end = offset as usize + last * stride.max(1) as usize;
        if let Some(data) = bytes(s, vb as usize, offset as usize, end) {
            out.push(ffx_gfx::Cmd::UpdateBuffer {
                id: vb,
                offset,
                data,
            });
        }
    }
    out
}

/// Whether the bound state draws something glowing in the world: blending
/// on, with the source or the destination taken whole (additive, or
/// premultiplied glow), and tested against the scene's depth (what is not
/// is drawn over everything: the map, menus).
fn glows(s: &State) -> bool {
    const ONE: u32 = 2;
    let word = |b: &[u8], i: usize| {
        b.get(i..i + 4)
            .map_or(0, |w| u32::from_le_bytes(w.try_into().expect("4")))
    };
    // No state is the default: depth tested.
    let tested = s.bound.depth == 0
        || s.states
            .get(&s.bound.depth)
            .is_some_and(|(_, d)| word(d, 0) != 0);
    tested
        && s.states
            .get(&s.bound.blend)
            .is_some_and(|(_, b)| word(b, 8) != 0 && (word(b, 12) == ONE || word(b, 16) == ONE))
}

/// A draw after the scene went to the renderer: a glowing effect over the
/// scene's colour against its depth is kept for the renderer (the game
/// still draws it).
fn effect(s: &mut State, d: Draw) {
    let b = &s.bound;
    if b.rtv[1] != 0 || b.dsv == 0 || !WANT_EFFECTS.load(Ordering::Relaxed) || !glows(s) {
        return;
    }
    let Ok(mut guard) = LINK.lock() else { return };
    let Some(link) = guard.as_mut() else { return };
    if !link.alive
        || !link
            .after
            .is_some_and(|sc| sc.color == b.rtv[0] && sc.depth == b.dsv)
    {
        return;
    }
    let streamed = capture::stream_draw(s, d);
    let reborn = std::mem::take(&mut s.reborn);
    for id in reborn {
        link.sent.remove(&id);
    }
    for id in capture::draw_objects(&streamed).collect::<Vec<_>>() {
        link.sync(s, id);
    }
    let updates = draw_updates(link, s, &streamed);
    link.effects.extend(updates);
    link.effects.push(ffx_gfx::Cmd::Draw(Box::new(streamed)));
}

/// A draw went to the context: scene draws are recorded for the renderer.
pub fn draw(s: &mut State, d: Draw) -> bool {
    let b = &s.bound;
    // Scene draws: colour and normal targets with depth, nothing else.
    if b.rtv[0] == 0 || b.rtv[1] == 0 || b.rtv[2] != 0 || b.dsv == 0 {
        effect(s, d);
        return false;
    }
    let scene = Scene {
        color: b.rtv[0],
        normal: b.rtv[1],
        depth: b.dsv,
    };
    let streamed = capture::stream_draw(s, d);
    let Ok(mut guard) = LINK.lock() else {
        return false;
    };
    let Some(link) = guard.as_mut() else {
        return false;
    };
    if !link.alive {
        return false;
    }
    if link.scene.is_some_and(|sc| sc != scene) {
        // Another set of scene targets: the renderer handles one.
        return false;
    }
    link.scene = Some(scene);
    let reborn = std::mem::take(&mut s.reborn);
    for id in reborn {
        link.sent.remove(&id);
    }
    for id in capture::draw_objects(&streamed).collect::<Vec<_>>() {
        link.sync(s, id);
    }
    let updates = draw_updates(link, s, &streamed);
    link.cmds.extend(updates);
    link.cmds.push(ffx_gfx::Cmd::Draw(Box::new(streamed)));
    // The game need not draw what the renderer draws (unless it has been
    // missing frames: then the game's own image stands in).
    link.skip
}

/// A texture copy during the scene (the scene colour for water): the
/// renderer makes its own; the copy is then the renderer's, not data to
/// read back.
pub fn copy(s: &mut State, dst: usize, src: usize) {
    if !(s.textures.contains_key(&dst) && s.textures.contains_key(&src)) {
        return;
    }
    let Ok(mut guard) = LINK.lock() else { return };
    let Some(link) = guard.as_mut() else { return };
    if link.scene.is_none() || !link.alive {
        return;
    }
    link.made.insert(dst);
    link.sync(s, src);
    link.sync(s, dst);
    link.cmds.push(ffx_gfx::Cmd::CopyTexture {
        dst: dst as u32,
        src: src as u32,
    });
}

/// A clear went to the context: kept in case it is for scene targets.
pub fn clear(cmd: ffx_gfx::Cmd) {
    if let Ok(mut guard) = LINK.lock()
        && let Some(link) = guard.as_mut()
    {
        link.clears.push(cmd);
    }
}

/// The game binds other targets: if it leaves the scene, the frame goes to
/// the renderer and its image over the game's.
pub fn targets(s: &mut State, rtv: [usize; 8], dsv: usize) {
    let Ok(mut guard) = LINK.lock() else { return };
    let Some(link) = guard.as_mut() else { return };
    let Some(scene) = link.scene else { return };
    if link.cmds.is_empty()
        || (rtv[0] == scene.color && rtv[1] == scene.normal && dsv == scene.depth)
    {
        return;
    }
    finish(link, s, scene);
}

/// At Present: a scene left unfinished is finished now, and the overlay
/// (if showing) goes over the frame.
///
/// # Safety
/// On the render thread, inside Present of `swap_chain`.
pub unsafe fn present(s: &mut State, swap_chain: Ptr) {
    let Ok(mut guard) = LINK.lock() else { return };
    let Some(link) = guard.as_mut() else { return };
    if let Some(scene) = link.scene
        && !link.cmds.is_empty()
    {
        finish(link, s, scene);
    }
    if !link.effects.is_empty() && link.alive {
        let cmds = std::mem::take(&mut link.effects);
        link.send(&ffx_gfx::Msg::Effects(ffx_gfx::Frame {
            number: link.frame,
            width: 0,
            height: 0,
            cmds,
        }));
    }
    link.after = None;
    if OVERLAY.load(Ordering::Acquire) && !link.answered && link.alive {
        // No scene this frame: ask for the overlay alone.
        link.frame += 1;
        send_input(link);
        link.send(&ffx_gfx::Msg::Frame(ffx_gfx::Frame {
            number: link.frame,
            width: 0,
            height: 0,
            cmds: Vec::new(),
        }));
        let frame = link.frame;
        let _ = receive(link, frame);
    } else if !OVERLAY.load(Ordering::Acquire) {
        // Input (the toggle) still goes out.
        let pending = INPUT.lock().is_ok_and(|q| !q.is_empty());
        if pending {
            send_input(link);
        }
        if !LAYER.load(Ordering::Acquire) {
            link.overlay = None;
        }
    }
    free_camera();
    game_pause();
    if (OVERLAY.load(Ordering::Acquire) || LAYER.load(Ordering::Acquire))
        && let Some(mut image) = link.overlay.take()
    {
        // SAFETY: forwarded from the caller.
        unsafe { draw_overlay(link, s, swap_chain, &mut image) };
        link.overlay = Some(image);
    }
    link.answered = false;
    link.clears.clear();
    link.scene = None;
}

#[allow(clippy::too_many_lines)] // send, answer, composite, report
fn finish(link: &mut Link, s: &mut State, scene: Scene) {
    let t0 = std::time::Instant::now();
    link.after = Some(scene);
    link.effects.clear();
    link.frame += 1;
    let mut cmds = Vec::new();
    // The scene targets' clears first, then the draws.
    for c in link.clears.drain(..) {
        let view = match &c {
            ffx_gfx::Cmd::ClearTarget { view, .. } | ffx_gfx::Cmd::ClearDepth { view, .. } => {
                *view as usize
            }
            _ => 0,
        };
        if view == scene.color || view == scene.normal || view == scene.depth {
            cmds.push(c);
        }
    }
    for v in [scene.color, scene.normal, scene.depth] {
        link.sync(s, v);
    }
    cmds.append(&mut link.cmds);
    send_input(link);
    link.send(&crate::world::msg());
    if let Some(m) = crate::minimap::msg() {
        link.send(&m);
    }
    let (w, h) = s
        .views
        .get(&scene.color)
        .and_then(|v| s.textures.get(&v.resource))
        .map_or((0, 0), |t| (t.desc[0], t.desc[1]));
    link.send(&ffx_gfx::Msg::Frame(ffx_gfx::Frame {
        number: link.frame,
        width: w,
        height: h,
        cmds,
    }));
    SENT_FRAMES.fetch_add(1, Ordering::Relaxed);
    if !link.alive || w == 0 {
        return;
    }
    // The renderer's answer: frame, width, height, then colour, normal
    // (RGBA8) and depth (f32), rows packed.
    let frame = link.frame;
    let t_send = t0.elapsed();
    let received = receive(link, frame);
    let t_wait = t0.elapsed();
    link.answered = received.is_some();
    if received.is_some() {
        link.streak += 1;
        if link.streak >= 30
            && !link.skip
            && std::env::var("FFX_RENDERER_SKIP").map_or(true, |v| v != "0")
        {
            link.skip = true;
        }
    } else {
        link.streak = 0;
        link.skip = false;
    }
    let Some(reply) = received else {
        link.misses += 1;
        MISSES.fetch_add(1, Ordering::Relaxed);
        if link.misses % 60 == 1 {
            log(&format!(
                "  live: no image for frame {frame} ({} missed)",
                link.misses
            ));
        }
        return;
    };
    NO_AFTERIMAGE.store(reply.no_afterimage, Ordering::Relaxed);
    let data = reply.scene;
    if (reply.width, reply.height) != (w, h) {
        return;
    }
    if let Some((_, _, Some((handle, value)))) = reply.shared {
        if link.fence.as_ref().is_none_or(|f| f.handle != handle) {
            if let Some(old) = link.fence.take() {
                // SAFETY: our reference.
                unsafe { release(old.fence) };
            }
            // SAFETY: on the render thread, the game's device.
            link.fence = unsafe { open_fence(handle) }.map(|fence| SharedFence { handle, fence });
        }
        // The renderer may still be drawing the image: the GPU waits.
        // SAFETY: on the render thread, the game's context.
        let waited = link
            .fence
            .as_ref()
            .is_some_and(|f| unsafe { gpu_wait(f.fence, value) });
        if !waited {
            return;
        }
    }
    let source = match reply.shared {
        Some((generation, set, _)) => {
            if link
                .shared
                .as_ref()
                .is_none_or(|x| x.generation != generation)
            {
                if let Some(mut old) = link.shared.take() {
                    // SAFETY: on the render thread.
                    unsafe { old.release() };
                }
                link.shared = Some(SharedImages {
                    generation,
                    sets: [None, None, None],
                });
            }
            let Some(images) = link.shared.as_mut() else {
                return;
            };
            let slot = &mut images.sets[set as usize % 3];
            if slot.is_none() {
                // SAFETY: on the render thread, the game's device.
                *slot = unsafe { open_shared(generation, set) };
            }
            match slot {
                Some((_, views)) => Source::Shared(*views),
                None => return,
            }
        }
        None => Source::Pixels(&data),
    };
    IMAGES.fetch_add(1, Ordering::Relaxed);
    // SAFETY: on the render thread; the game's device and context.
    unsafe { composite(link, s, scene, w, h, &source) };
    if frame.is_multiple_of(120) {
        for (kind, (n, bytes, time)) in link.resent.drain() {
            log(&format!(
                "  live: sent {n} {kind}s, {bytes} bytes, in {time:?} (last 120 frames)"
            ));
        }
        log(&format!(
            "  live: frame {frame}: sent {t_send:?}, image at {t_wait:?}, composited at {:?}",
            t0.elapsed()
        ));
    }
}

/// Where a shared image is: generation, set, and the fence (its handle here
/// and the value that means written).
type SharedAt = (u32, u32, Option<(u32, u64)>);

/// The renderer's answer to a frame.
struct Reply {
    width: u32,
    height: u32,
    /// Colour and normal (RGBA8) then depth (f32), rows packed; empty
    /// when the image is in shared textures.
    scene: Vec<u8>,
    /// The shared textures' generation and set, and the fence (its handle
    /// here and the value that means written).
    shared: Option<SharedAt>,
    /// The renderer's settings turn the game's afterimage off.
    no_afterimage: bool,
}

/// Where the renderer's image is.
enum Source<'a> {
    Pixels(&'a [u8]),
    /// Shader resource views of shared colour, normal and depth.
    Shared([Ptr; 3]),
}

/// Sends the window input gathered since the last frame.
fn send_input(link: &mut Link) {
    let events = INPUT
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default();
    if events.is_empty() && !OVERLAY.load(Ordering::Acquire) {
        return;
    }
    let (width, height) = SCREEN.lock().map_or((0, 0), |s| *s);
    link.send(&ffx_gfx::Msg::Input {
        width,
        height,
        events,
    });
}

/// Reads answers until the one for `frame` (older ones are dropped).
#[allow(clippy::too_many_lines)] // the answer, part by part
fn receive(link: &mut Link, frame: u32) -> Option<Reply> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1500);
    let process = link.process;
    let mut wait = || {
        if std::time::Instant::now() > deadline {
            return false;
        }
        // SAFETY: our process handle.
        unsafe {
            WaitForSingleObject(process, 0) == WAIT_TIMEOUT && {
                std::thread::yield_now();
                true
            }
        }
    };
    loop {
        let mut head = [0u8; 16];
        if !link.to_game.read(&mut head, &mut wait) {
            return None;
        }
        let word = |i: usize| u32::from_le_bytes(head[4 * i..4 * i + 4].try_into().expect("4"));
        let (number, w, h, flags) = (word(0), word(1), word(2), word(3));
        let mut shared = None;
        let mut scene = Vec::new();
        if flags & 2 != 0 {
            let mut at = [0u8; 8];
            if !link.to_game.read(&mut at, &mut wait) {
                return None;
            }
            let mut fence = None;
            if flags & 4 != 0 {
                let mut f = [0u8; 12];
                if !link.to_game.read(&mut f, &mut wait) {
                    return None;
                }
                fence = Some((
                    u32::from_le_bytes(f[0..4].try_into().expect("4")),
                    u64::from_le_bytes(f[4..12].try_into().expect("8")),
                ));
            }
            shared = Some((
                u32::from_le_bytes(at[0..4].try_into().expect("4")),
                u32::from_le_bytes(at[4..8].try_into().expect("4")),
                fence,
            ));
        } else {
            scene = vec![0u8; (w * h * 12) as usize];
            if !link.to_game.read(&mut scene, &mut wait) {
                return None;
            }
        }
        if flags & 1 != 0 {
            // The screen's size, then the part that changed (x, y, width,
            // height) and its pixels: kept from every answer, also those
            // for frames no longer waited for.
            let mut at = [0u8; 24];
            if !link.to_game.read(&mut at, &mut wait) {
                return None;
            }
            let v = |i: usize| u32::from_le_bytes(at[4 * i..4 * i + 4].try_into().expect("4"));
            let screen = (v(0), v(1));
            let area = (v(2), v(3), v(4), v(5));
            let mut pixels = vec![0u8; (area.2 * area.3 * 4) as usize];
            if !link.to_game.read(&mut pixels, &mut wait) {
                return None;
            }
            let image = link
                .overlay
                .get_or_insert_with(|| OverlayImage::new(screen));
            if image.size != screen {
                *image = OverlayImage::new(screen);
            }
            image.put(area, &pixels);
        }
        let mut free = None;
        if flags & 64 != 0 {
            let mut at = [0u8; 32];
            if !link.to_game.read(&mut at, &mut wait) {
                return None;
            }
            let v: Vec<f32> = at
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            free = Some([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]]);
        }
        if let Ok(mut f) = FREE_CAMERA.lock() {
            *f = free;
        }
        if number == frame {
            OVERLAY_KEYS.store(flags & 16 != 0, Ordering::Release);
            PAUSED.store(flags & 128 != 0, Ordering::Release);
            LAYER.store(flags & 256 != 0, Ordering::Release);
            HOLD_POINTER.store(flags & 1024 != 0, Ordering::Release);
            crate::minimap::REPLACE.store(flags & 512 != 0, Ordering::Release);
            WANT_EFFECTS.store(flags & 32 != 0, Ordering::Relaxed);
            return Some(Reply {
                width: w,
                height: h,
                scene,
                shared,
                no_afterimage: flags & 8 != 0,
            });
        }
    }
}

/// The overlay as the renderer last sent it, and what changed since the
/// game's texture had it.
struct OverlayImage {
    size: (u32, u32),
    pixels: Vec<u8>,
    /// x0, y0, x1, y1.
    dirty: Option<(u32, u32, u32, u32)>,
}

impl OverlayImage {
    fn new(size: (u32, u32)) -> Self {
        Self {
            size,
            pixels: vec![0; (size.0 * size.1 * 4) as usize],
            dirty: Some((0, 0, size.0, size.1)),
        }
    }

    /// `pixels` at `area` (x, y, width, height).
    fn put(&mut self, area: (u32, u32, u32, u32), pixels: &[u8]) {
        let (left, top, width, height) = area;
        let (right, bottom) = (left + width, top + height);
        if width == 0 || height == 0 || right > self.size.0 || bottom > self.size.1 {
            return;
        }
        let row = (width * 4) as usize;
        for r in 0..height as usize {
            let at = ((top as usize + r) * self.size.0 as usize + left as usize) * 4;
            self.pixels[at..at + row].copy_from_slice(&pixels[r * row..(r + 1) * row]);
        }
        self.dirty = Some(match self.dirty {
            Some(d) => (d.0.min(left), d.1.min(top), d.2.max(right), d.3.max(bottom)),
            None => (left, top, right, bottom),
        });
    }
}

struct OverlayPass {
    size: (u32, u32),
    texture: Ptr,
    view: Ptr,
    ps: Ptr,
    blend: Ptr,
    /// The back buffer it last drew on, and a view of it.
    target: (usize, Ptr),
}

const OVERLAY_HLSL: &str = r"
Texture2D O : register(t0);
float4 vs(uint id : SV_VertexID) : SV_Position {
    float2 uv = float2((id << 1) & 2, id & 2);
    return float4(uv * float2(2, -2) + float2(-1, 1), 0, 1);
}
float4 ps(float4 p : SV_Position) : SV_Target0 {
    return O.Load(int3(p.xy, 0));
}
";

/// Draws the overlay (premultiplied RGBA8) over the back buffer.
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::cast_precision_loss
)] // one pass set up, drawn and undone
unsafe fn draw_overlay(link: &mut Link, s: &State, swap_chain: Ptr, image: &mut OverlayImage) {
    let (w, h) = image.size;
    const IID_TEXTURE2D: [u8; 16] = [
        0xf2, 0xaa, 0x15, 0x6f, 0x08, 0xd2, 0x89, 0x4e, 0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3, 0x4f,
        0x9c,
    ];
    let device = super::DEVICE.load(Ordering::Acquire) as Ptr;
    let ctx = super::CONTEXT.load(Ordering::Acquire) as Ptr;
    // SAFETY: the game's device, context and swap chain, original methods.
    unsafe {
        if link.overlay_pass.as_ref().is_none_or(|o| o.size != (w, h)) {
            image.dirty = Some((0, 0, w, h));
            let Some(vs) = link.composite.as_ref().map(|c| c.vs).or_else(|| {
                let code = compile_source(OVERLAY_HLSL, "vs", "vs_4_0")?;
                type CreateShader =
                    unsafe extern "system" fn(Ptr, *const u8, usize, Ptr, *mut Ptr) -> i32;
                let f: CreateShader =
                    std::mem::transmute(device_original(device, vt::device::CREATE_VERTEX_SHADER));
                let mut vs: Ptr = std::ptr::null_mut();
                (f(
                    device,
                    code.as_ptr(),
                    code.len(),
                    std::ptr::null_mut(),
                    &raw mut vs,
                ) >= 0)
                    .then_some(vs)
            }) else {
                return;
            };
            let _ = vs;
            let Some(ps_code) = compile_source(OVERLAY_HLSL, "ps", "ps_4_0") else {
                return;
            };
            type CreateShader =
                unsafe extern "system" fn(Ptr, *const u8, usize, Ptr, *mut Ptr) -> i32;
            let create_ps: CreateShader =
                std::mem::transmute(device_original(device, vt::device::CREATE_PIXEL_SHADER));
            let mut ps: Ptr = std::ptr::null_mut();
            if create_ps(
                device,
                ps_code.as_ptr(),
                ps_code.len(),
                std::ptr::null_mut(),
                &raw mut ps,
            ) < 0
            {
                return;
            }
            type CreateTex =
                unsafe extern "system" fn(Ptr, *const u32, *const c_void, *mut Ptr) -> i32;
            type CreateSrv = unsafe extern "system" fn(Ptr, Ptr, *const u32, *mut Ptr) -> i32;
            let create_tex: CreateTex =
                std::mem::transmute(device_original(device, vt::device::CREATE_TEXTURE_2D));
            let create_srv: CreateSrv = std::mem::transmute(device_original(
                device,
                vt::device::CREATE_SHADER_RESOURCE_VIEW,
            ));
            let desc = [w, h, 1, 1, 28, 1, 0, 0, 0x8, 0, 0];
            let mut texture: Ptr = std::ptr::null_mut();
            let mut view: Ptr = std::ptr::null_mut();
            if create_tex(device, desc.as_ptr(), std::ptr::null(), &raw mut texture) < 0
                || create_srv(device, texture, std::ptr::null(), &raw mut view) < 0
            {
                return;
            }
            let put =
                |d: &mut [u8], at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
            let mut blend_desc = [0u8; 8 + 8 * 32];
            for i in 0..8 {
                let at = 8 + 32 * i;
                put(&mut blend_desc, at, u32::from(i == 0)); // blend on target 0
                put(&mut blend_desc, at + 4, 2); // one
                put(&mut blend_desc, at + 8, 6); // inverse source alpha
                put(&mut blend_desc, at + 12, 1);
                put(&mut blend_desc, at + 16, 2);
                put(&mut blend_desc, at + 20, 6);
                put(&mut blend_desc, at + 24, 1);
                blend_desc[at + 28] = 0xf;
            }
            type CreateState = unsafe extern "system" fn(Ptr, *const u8, *mut Ptr) -> i32;
            let create_blend: CreateState =
                std::mem::transmute(device_original(device, vt::device::CREATE_BLEND_STATE));
            let mut blend: Ptr = std::ptr::null_mut();
            create_blend(device, blend_desc.as_ptr(), &raw mut blend);
            link.overlay_pass = Some(OverlayPass {
                size: (w, h),
                texture,
                view,
                ps,
                blend,
                target: (0, std::ptr::null_mut()),
            });
        }
        if link.composite.is_none() {
            let (sw, sh) = SCREEN.lock().map_or((w, h), |x| *x);
            link.composite = make_composite(sw, sh);
        }
        let (Some(o), Some(c)) = (link.overlay_pass.as_mut(), link.composite.as_ref()) else {
            return;
        };
        // The back buffer and a view of it.
        let mut back: Ptr = std::ptr::null_mut();
        if super::com!(
            swap_chain,
            vt::GET_BUFFER,
            unsafe extern "system" fn(Ptr, u32, *const u8, *mut Ptr) -> i32,
            0,
            IID_TEXTURE2D.as_ptr(),
            &raw mut back
        ) < 0
        {
            return;
        }
        if o.target.0 != back as usize {
            type CreateRtv = unsafe extern "system" fn(Ptr, Ptr, *const u32, *mut Ptr) -> i32;
            let create_rtv: CreateRtv = std::mem::transmute(device_original(
                device,
                vt::device::CREATE_RENDER_TARGET_VIEW,
            ));
            let mut rtv: Ptr = std::ptr::null_mut();
            create_rtv(device, back, std::ptr::null(), &raw mut rtv);
            o.target = (back as usize, rtv);
        }
        super::com!(back, vt::RELEASE, unsafe extern "system" fn(Ptr) -> u32);
        macro_rules! call {
            ($slot:expr, $ty:ty $(, $arg:expr)*) => {{
                let f: $ty = std::mem::transmute(real_method(ctx, $slot));
                f(ctx $(, $arg)*)
            }};
        }
        use vt::context as k;
        if let Some((x0, y0, x1, y1)) = image.dirty.take() {
            // D3D11_BOX: left, top, front, right, bottom, back.
            let area = [x0, y0, 0, x1, y1, 1];
            call!(
                k::UPDATE_SUBRESOURCE,
                unsafe extern "system" fn(Ptr, Ptr, u32, *const u32, *const u8, u32, u32),
                o.texture,
                0,
                area.as_ptr(),
                image.pixels[((y0 * w + x0) * 4) as usize..].as_ptr(),
                w * 4,
                0
            );
        }
        let rtvs = [o.target.1 as usize];
        call!(
            k::OM_SET_RENDER_TARGETS,
            unsafe extern "system" fn(Ptr, u32, *const usize, Ptr),
            1,
            rtvs.as_ptr(),
            std::ptr::null_mut()
        );
        let vp = [0.0f32, 0.0, w as f32, h as f32, 0.0, 1.0];
        call!(
            k::RS_SET_VIEWPORTS,
            unsafe extern "system" fn(Ptr, u32, *const f32),
            1,
            vp.as_ptr()
        );
        call!(
            k::RS_SET_STATE,
            unsafe extern "system" fn(Ptr, Ptr),
            c.raster
        );
        call!(
            k::OM_SET_BLEND_STATE,
            unsafe extern "system" fn(Ptr, Ptr, *const f32, u32),
            o.blend,
            std::ptr::null(),
            u32::MAX
        );
        call!(
            k::OM_SET_DEPTH_STENCIL_STATE,
            unsafe extern "system" fn(Ptr, Ptr, u32),
            std::ptr::null_mut(),
            0
        );
        call!(
            k::IA_SET_INPUT_LAYOUT,
            unsafe extern "system" fn(Ptr, Ptr),
            std::ptr::null_mut()
        );
        call!(
            k::IA_SET_PRIMITIVE_TOPOLOGY,
            unsafe extern "system" fn(Ptr, u32),
            4
        );
        call!(
            k::VS_SET_SHADER,
            unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32),
            c.vs,
            std::ptr::null(),
            0
        );
        call!(
            k::PS_SET_SHADER,
            unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32),
            o.ps,
            std::ptr::null(),
            0
        );
        let views = [o.view];
        call!(
            k::PS_SET_SHADER_RESOURCES,
            unsafe extern "system" fn(Ptr, u32, u32, *const Ptr),
            0,
            1,
            views.as_ptr()
        );
        call!(k::DRAW, unsafe extern "system" fn(Ptr, u32, u32), 3, 0);
        restore(ctx, s);
    }
}

/// Puts the game's tracked bindings back on the real context.
unsafe fn restore(ctx: Ptr, s: &State) {
    macro_rules! call {
        ($slot:expr, $ty:ty $(, $arg:expr)*) => {{
            let f: $ty = std::mem::transmute(real_method(ctx, $slot));
            f(ctx $(, $arg)*)
        }};
    }
    use vt::context as k;
    let b = &s.bound;
    // SAFETY: the real context; values the game set before.
    unsafe {
        call!(
            k::RS_SET_VIEWPORTS,
            unsafe extern "system" fn(Ptr, u32, *const f32),
            1,
            b.viewport.as_ptr()
        );
        call!(
            k::RS_SET_STATE,
            unsafe extern "system" fn(Ptr, Ptr),
            b.raster as Ptr
        );
        call!(
            k::OM_SET_BLEND_STATE,
            unsafe extern "system" fn(Ptr, Ptr, *const f32, u32),
            b.blend as Ptr,
            b.blend_factor.as_ptr(),
            b.sample_mask
        );
        call!(
            k::OM_SET_DEPTH_STENCIL_STATE,
            unsafe extern "system" fn(Ptr, Ptr, u32),
            b.depth as Ptr,
            b.stencil_ref
        );
        call!(
            k::IA_SET_INPUT_LAYOUT,
            unsafe extern "system" fn(Ptr, Ptr),
            b.layout as Ptr
        );
        call!(
            k::IA_SET_PRIMITIVE_TOPOLOGY,
            unsafe extern "system" fn(Ptr, u32),
            b.topology
        );
        call!(
            k::VS_SET_SHADER,
            unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32),
            b.vs as Ptr,
            std::ptr::null(),
            0
        );
        call!(
            k::PS_SET_SHADER,
            unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32),
            b.ps as Ptr,
            std::ptr::null(),
            0
        );
        // The targets first: Direct3D 11 refuses a shader view of a
        // texture bound as a target, so the game's views go back after.
        call!(
            k::OM_SET_RENDER_TARGETS,
            unsafe extern "system" fn(Ptr, u32, *const usize, Ptr),
            8,
            b.rtv.as_ptr(),
            b.dsv as Ptr
        );
        call!(
            k::PS_SET_SHADER_RESOURCES,
            unsafe extern "system" fn(Ptr, u32, u32, *const usize),
            0,
            3,
            b.ps_srv.as_ptr()
        );
    }
}

const COMPOSITE_HLSL: &str = r"
Texture2D<float4> C : register(t0);
Texture2D<float4> N : register(t1);
Texture2D<float> Z : register(t2);
struct Out { float4 c : SV_Target0; float4 n : SV_Target1; float d : SV_Depth; };
float4 vs(uint id : SV_VertexID) : SV_Position {
    float2 uv = float2((id << 1) & 2, id & 2);
    return float4(uv * float2(2, -2) + float2(-1, 1), 0, 1);
}
Out ps(float4 p : SV_Position) {
    int3 i = int3(p.xy, 0);
    Out o;
    o.c = C.Load(i);
    o.n = N.Load(i);
    o.d = Z.Load(i);
    return o;
}
";

/// Compiles the composite HLSL with the system's `d3dcompiler_47.dll`.
unsafe fn compile(entry: &str, target: &str) -> Option<Vec<u8>> {
    // SAFETY: forwarded.
    unsafe { compile_source(COMPOSITE_HLSL, entry, target) }
}

/// Compiles HLSL with the system's `d3dcompiler_47.dll`.
unsafe fn compile_source(source: &str, entry: &str, target: &str) -> Option<Vec<u8>> {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    type Compile = unsafe extern "system" fn(
        *const u8,
        usize,
        *const u8,
        *const c_void,
        *const c_void,
        *const u8,
        *const u8,
        u32,
        u32,
        *mut Ptr,
        *mut Ptr,
    ) -> i32;
    // SAFETY: the documented D3DCompile signature; ID3DBlob is IUnknown
    // plus GetBufferPointer (3) and GetBufferSize (4).
    unsafe {
        let lib = LoadLibraryA(c"d3dcompiler_47.dll".as_ptr().cast());
        let f: Compile = std::mem::transmute(GetProcAddress(lib, c"D3DCompile".as_ptr().cast())?);
        let entry = format!("{entry}\0");
        let target = format!("{target}\0");
        let mut code: Ptr = std::ptr::null_mut();
        let mut errors: Ptr = std::ptr::null_mut();
        let hr = f(
            source.as_ptr(),
            source.len(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            entry.as_ptr(),
            target.as_ptr(),
            0,
            0,
            &raw mut code,
            &raw mut errors,
        );
        if hr < 0 || code.is_null() {
            log(&format!("  live: D3DCompile {entry} failed {hr:#x}"));
            return None;
        }
        let ptr: unsafe extern "system" fn(Ptr) -> *const u8 =
            std::mem::transmute(*(*(code as *const *const usize)).add(3));
        let len: unsafe extern "system" fn(Ptr) -> usize =
            std::mem::transmute(*(*(code as *const *const usize)).add(4));
        let bytes = std::slice::from_raw_parts(ptr(code), len(code)).to_vec();
        super::com!(code, vt::RELEASE, unsafe extern "system" fn(Ptr) -> u32);
        Some(bytes)
    }
}

/// The composite pass's objects, for a `width` × `height` frame.
unsafe fn make_composite(width: u32, height: u32) -> Option<Composite> {
    let device = super::DEVICE.load(Ordering::Acquire) as Ptr;
    // SAFETY: the game's device, original methods.
    unsafe {
        let vs_code = compile("vs", "vs_4_0")?;
        let ps_code = compile("ps", "ps_4_0")?;
        type CreateShader = unsafe extern "system" fn(Ptr, *const u8, usize, Ptr, *mut Ptr) -> i32;
        let new_vertex_shader: CreateShader =
            std::mem::transmute(device_original(device, vt::device::CREATE_VERTEX_SHADER));
        let new_pixel_shader: CreateShader =
            std::mem::transmute(device_original(device, vt::device::CREATE_PIXEL_SHADER));
        let mut vs: Ptr = std::ptr::null_mut();
        let mut ps: Ptr = std::ptr::null_mut();
        if new_vertex_shader(
            device,
            vs_code.as_ptr(),
            vs_code.len(),
            std::ptr::null_mut(),
            &raw mut vs,
        ) < 0
            || new_pixel_shader(
                device,
                ps_code.as_ptr(),
                ps_code.len(),
                std::ptr::null_mut(),
                &raw mut ps,
            ) < 0
        {
            return None;
        }
        type CreateTex = unsafe extern "system" fn(Ptr, *const u32, *const c_void, *mut Ptr) -> i32;
        type CreateSrv = unsafe extern "system" fn(Ptr, Ptr, *const u32, *mut Ptr) -> i32;
        let create_tex: CreateTex =
            std::mem::transmute(device_original(device, vt::device::CREATE_TEXTURE_2D));
        let create_srv: CreateSrv = std::mem::transmute(device_original(
            device,
            vt::device::CREATE_SHADER_RESOURCE_VIEW,
        ));
        let mut textures = [std::ptr::null_mut(); 3];
        let mut views = [std::ptr::null_mut(); 3];
        for (i, format) in [28u32, 28, 41].into_iter().enumerate() {
            // Default usage, shader resource.
            let desc = [width, height, 1, 1, format, 1, 0, 0, 0x8, 0, 0];
            if create_tex(
                device,
                desc.as_ptr(),
                std::ptr::null(),
                &raw mut textures[i],
            ) < 0
                || create_srv(device, textures[i], std::ptr::null(), &raw mut views[i]) < 0
            {
                return None;
            }
        }
        type CreateState = unsafe extern "system" fn(Ptr, *const u8, *mut Ptr) -> i32;
        let mut depth_desc = [0u8; 52];
        let put = |d: &mut [u8], at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut depth_desc, 0, 1); // depth on
        put(&mut depth_desc, 4, 1); // write all
        put(&mut depth_desc, 8, 8); // always
        let mut depth_state: Ptr = std::ptr::null_mut();
        let create_depth: CreateState = std::mem::transmute(device_original(
            device,
            vt::device::CREATE_DEPTH_STENCIL_STATE,
        ));
        create_depth(device, depth_desc.as_ptr(), &raw mut depth_state);
        let mut blend_desc = [0u8; 8 + 8 * 32];
        for i in 0..8 {
            let at = 8 + 32 * i;
            put(&mut blend_desc, at + 4, 2); // one
            put(&mut blend_desc, at + 8, 1); // zero
            put(&mut blend_desc, at + 12, 1); // add
            put(&mut blend_desc, at + 16, 2);
            put(&mut blend_desc, at + 20, 1);
            put(&mut blend_desc, at + 24, 1);
            blend_desc[at + 28] = 0xf;
        }
        let mut blend: Ptr = std::ptr::null_mut();
        let create_blend: CreateState =
            std::mem::transmute(device_original(device, vt::device::CREATE_BLEND_STATE));
        create_blend(device, blend_desc.as_ptr(), &raw mut blend);
        let mut raster_desc = [0u8; 40];
        put(&mut raster_desc, 0, 3); // solid
        put(&mut raster_desc, 4, 1); // no culling
        put(&mut raster_desc, 24, 1); // depth clip
        let mut raster: Ptr = std::ptr::null_mut();
        let create_raster: CreateState =
            std::mem::transmute(device_original(device, vt::device::CREATE_RASTERIZER_STATE));
        create_raster(device, raster_desc.as_ptr(), &raw mut raster);
        Some(Composite {
            width,
            height,
            textures,
            views,
            vs,
            ps,
            depth_state,
            blend,
            raster,
        })
    }
}

/// Draws the renderer's images into the scene targets, then puts the
/// game's bindings back.
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::cast_precision_loss
)] // one pass set up, drawn and undone
unsafe fn composite(link: &mut Link, s: &State, scene: Scene, w: u32, h: u32, source: &Source) {
    let ctx = super::CONTEXT.load(Ordering::Acquire) as Ptr;
    if link
        .composite
        .as_ref()
        .is_none_or(|c| (c.width, c.height) != (w, h))
    {
        // SAFETY: forwarded.
        link.composite = unsafe { make_composite(w, h) };
    }
    let Some(c) = &link.composite else { return };
    let plane = (w * h * 4) as usize;
    macro_rules! call {
        ($slot:expr, $ty:ty $(, $arg:expr)*) => {{
            let f: $ty = std::mem::transmute(real_method(ctx, $slot));
            f(ctx $(, $arg)*)
        }};
    }
    use vt::context as k;
    // SAFETY: the real immediate context on the render thread; every
    // binding changed here is restored from the tracked state after.
    unsafe {
        let views = match source {
            Source::Pixels(data) => {
                for i in 0..3 {
                    call!(
                        k::UPDATE_SUBRESOURCE,
                        unsafe extern "system" fn(Ptr, Ptr, u32, *const u32, *const u8, u32, u32),
                        c.textures[i],
                        0,
                        std::ptr::null(),
                        data[i * plane..].as_ptr(),
                        w * 4,
                        0
                    );
                }
                c.views
            }
            Source::Shared(views) => *views,
        };
        let rtvs = [scene.color, scene.normal];
        call!(
            k::OM_SET_RENDER_TARGETS,
            unsafe extern "system" fn(Ptr, u32, *const usize, Ptr),
            2,
            rtvs.as_ptr(),
            scene.depth as Ptr
        );
        let vp = [0.0f32, 0.0, w as f32, h as f32, 0.0, 1.0];
        call!(
            k::RS_SET_VIEWPORTS,
            unsafe extern "system" fn(Ptr, u32, *const f32),
            1,
            vp.as_ptr()
        );
        call!(
            k::RS_SET_STATE,
            unsafe extern "system" fn(Ptr, Ptr),
            c.raster
        );
        call!(
            k::OM_SET_BLEND_STATE,
            unsafe extern "system" fn(Ptr, Ptr, *const f32, u32),
            c.blend,
            std::ptr::null(),
            u32::MAX
        );
        call!(
            k::OM_SET_DEPTH_STENCIL_STATE,
            unsafe extern "system" fn(Ptr, Ptr, u32),
            c.depth_state,
            0
        );
        call!(
            k::IA_SET_INPUT_LAYOUT,
            unsafe extern "system" fn(Ptr, Ptr),
            std::ptr::null_mut()
        );
        call!(
            k::IA_SET_PRIMITIVE_TOPOLOGY,
            unsafe extern "system" fn(Ptr, u32),
            4
        );
        call!(
            k::VS_SET_SHADER,
            unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32),
            c.vs,
            std::ptr::null(),
            0
        );
        call!(
            k::PS_SET_SHADER,
            unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32),
            c.ps,
            std::ptr::null(),
            0
        );
        call!(
            k::PS_SET_SHADER_RESOURCES,
            unsafe extern "system" fn(Ptr, u32, u32, *const Ptr),
            0,
            3,
            views.as_ptr()
        );
        call!(k::DRAW, unsafe extern "system" fn(Ptr, u32, u32), 3, 0);
        // Put the game's bindings back.
        let b = &s.bound;
        call!(
            k::RS_SET_VIEWPORTS,
            unsafe extern "system" fn(Ptr, u32, *const f32),
            1,
            b.viewport.as_ptr()
        );
        call!(
            k::RS_SET_STATE,
            unsafe extern "system" fn(Ptr, Ptr),
            b.raster as Ptr
        );
        call!(
            k::OM_SET_BLEND_STATE,
            unsafe extern "system" fn(Ptr, Ptr, *const f32, u32),
            b.blend as Ptr,
            b.blend_factor.as_ptr(),
            b.sample_mask
        );
        call!(
            k::OM_SET_DEPTH_STENCIL_STATE,
            unsafe extern "system" fn(Ptr, Ptr, u32),
            b.depth as Ptr,
            b.stencil_ref
        );
        call!(
            k::IA_SET_INPUT_LAYOUT,
            unsafe extern "system" fn(Ptr, Ptr),
            b.layout as Ptr
        );
        call!(
            k::IA_SET_PRIMITIVE_TOPOLOGY,
            unsafe extern "system" fn(Ptr, u32),
            b.topology
        );
        call!(
            k::VS_SET_SHADER,
            unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32),
            b.vs as Ptr,
            std::ptr::null(),
            0
        );
        call!(
            k::PS_SET_SHADER,
            unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32),
            b.ps as Ptr,
            std::ptr::null(),
            0
        );
        // The targets first: Direct3D 11 refuses a shader view of a
        // texture bound as a target, so the game's views go back after.
        call!(
            k::OM_SET_RENDER_TARGETS,
            unsafe extern "system" fn(Ptr, u32, *const usize, Ptr),
            8,
            b.rtv.as_ptr(),
            b.dsv as Ptr
        );
        call!(
            k::PS_SET_SHADER_RESOURCES,
            unsafe extern "system" fn(Ptr, u32, u32, *const usize),
            0,
            3,
            b.ps_srv.as_ptr()
        );
    }
}
