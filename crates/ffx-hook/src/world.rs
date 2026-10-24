//! What the game is showing, for the renderer: the scene (the event file
//! of the map, the last `.ebp` the game opened) and the camera; and, for
//! reference captures (`gfx::capture`), the scene inputs a Rust scene
//! builder reads, sampled from the game's memory (`docs/renderer.md`,
//! "Reference captures").

use std::sync::Mutex;
use std::sync::atomic::Ordering;

/// `g_Camera`: the active camera channel (`symbols/ffx.sym`).
const G_CAMERA: u32 = 0x0231_1430;
/// Offsets in a camera channel: the look-at point and the eye.
const LOOK_AT: usize = 0x50;
const EYE: usize = 0x61c + 0x50;

static SCENE: Mutex<String> = Mutex::new(String::new());
/// Files opened since the last [`take_opened`], with the `Present` count
/// at the open; kept only while the graphics layer is on, at most
/// [`OPENED_MAX`].
static OPENED: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());
const OPENED_MAX: usize = 4096;

/// Notes a file the game opens: an event file names the scene.
pub fn opened(path: &[u8]) {
    let name = path
        .rsplit(|&c| c == b'/' || c == b'\\')
        .next()
        .unwrap_or(path);
    if let Some(stem) = name.strip_suffix(b".ebp")
        && let Ok(stem) = std::str::from_utf8(stem)
        && let Ok(mut s) = SCENE.lock()
        && *s != stem
    {
        crate::log::log(&format!("  world: scene {stem}"));
        stem.clone_into(&mut s);
    }
    if crate::gfx::ENABLED.load(Ordering::Relaxed)
        && let Ok(mut list) = OPENED.lock()
    {
        if list.len() >= OPENED_MAX {
            list.remove(0);
        }
        list.push((
            crate::gfx::stats().0,
            String::from_utf8_lossy(path).into_owned(),
        ));
    }
}

/// The files opened since the last call (the openFile hook,
/// `Phyre_PSerialization_PStreamFileWin32_openFile` `0x00607f40`).
pub fn take_opened() -> Vec<(u32, String)> {
    OPENED
        .lock()
        .map(|mut l| std::mem::take(&mut *l))
        .unwrap_or_default()
}

/// A point of the camera channel, in the renderer's world space: the
/// game's field space with x and y negated, as its view matrix has it (on
/// the S.S. Winno deck the channel's eye is (-73.02, -26.47, -6.02) and
/// the view matrix's (73.02, 27.12, -6.02)).
fn read3(at: usize) -> Option<[f32; 3]> {
    // SAFETY: inside the camera channel table of the game's image (a
    // static block), read as plain floats.
    let v = unsafe { [0, 4, 8].map(|o| std::ptr::read_unaligned((at + o) as *const f32)) };
    v.iter()
        .all(|x| x.is_finite() && x.abs() < 1e7)
        .then_some([-v[0], -v[1], v[2]])
}

/// The message for the renderer.
pub fn msg() -> ffx_gfx::Msg {
    // SAFETY: a pointer-sized global of the game's image.
    let camera = unsafe { std::ptr::read_volatile(crate::rebase(G_CAMERA) as *const usize) };
    let camera = (camera != 0)
        .then(|| Some((read3(camera + EYE)?, read3(camera + LOOK_AT)?)))
        .flatten();
    ffx_gfx::Msg::World {
        scene: SCENE.lock().map(|s| s.clone()).unwrap_or_default(),
        camera,
    }
}

/// `g_CameraMatrix`: the game's 4x4 view matrix `Cam_Apply` passes to
/// `graphicCameraSetWorldMatrix`.
const G_CAMERA_MATRIX: u32 = 0x0231_1440;
/// `g_PhyreScene`, `g_PhyreApplication`, `g_PostProcessManager`: pointers.
const G_PHYRE_SCENE: u32 = 0x00cc_b9d8;
const G_PHYRE_APPLICATION: u32 = 0x00cc_9cd8;
const G_POST_PROCESS_MANAGER: u32 = 0x00cc_c8a0;
/// Ctrl work 0's frame count (`g_AtelCtrlWorks` +0x1f4).
pub const EVENT_COUNT: u32 = 0x0132_5d94;
/// At most this many 2D-layer entries are sampled.
const LAYERS_MAX: u32 = 256;

fn bytes(at: usize, len: usize) -> Option<Vec<u8>> {
    (at != 0).then(|| crate::harness::read_memory(at, len).ok())?
}

fn word(at: usize) -> Option<usize> {
    let b = bytes(at, 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

/// The scene inputs, by name, as raw bytes (absent when unreadable). Plain
/// memory reads (`ReadProcessMemory`, never a fault), on any thread.
/// Pointers are left out (they differ between runs); the 2D-layer
/// entries' mesh pointer becomes 1 or 0.
#[must_use]
pub fn scene_inputs() -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut put = |name: &str, v: Option<Vec<u8>>| {
        if let Some(v) = v {
            out.push((name.to_owned(), v));
        }
    };
    put("event_count", bytes(crate::rebase(EVENT_COUNT), 4));
    put("g_CameraMatrix", bytes(crate::rebase(G_CAMERA_MATRIX), 64));
    let channel = word(crate::rebase(G_CAMERA)).unwrap_or(0);
    put("g_Camera+0x50", bytes(channel + LOOK_AT, 12));
    put("g_Camera+0x66c", bytes(channel + EYE, 12));
    let scene = word(crate::rebase(G_PHYRE_SCENE)).unwrap_or(0);
    if scene != 0 {
        // The camera's world matrix (graphicCameraSetWorldMatrix) and the
        // screen shift getViewProjectionMatrix adds.
        put("scene+0xfa04", bytes(scene + 0xfa04, 48));
        put("scene+0xfa34", bytes(scene + 0xfa34, 8));
        let camera = word(scene + 0xdc).unwrap_or(0);
        put("camera+0xbc", bytes(camera + 0xbc, 4));
        put("camera+0x150", bytes(camera + 0x150, 8));
        // 2D layers (PhyreScene_show2DLayer): present flag, count, group,
        // and the entries.
        // (a pointer: only whether it is set)
        let layers = word(scene + 0x1128).map(|p| u32::from(p != 0).to_le_bytes().to_vec());
        put("scene+0x1128!=0", layers);
        let count = word(scene + 0x1138).unwrap_or(0) as u32;
        put("scene+0x1138", Some(count.to_le_bytes().to_vec()));
        put("scene+0x11d8", bytes(scene + 0x11d8, 4));
        let table = word(scene + 0x1140).unwrap_or(0);
        if table != 0 && count > 0 {
            let n = count.min(LAYERS_MAX) as usize;
            if let Some(raw) = bytes(table, n * 8) {
                let entries: Vec<u8> = raw
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .flat_map(|e| {
                        let mesh = u32::from(e[4..8].iter().any(|&b| b != 0));
                        [e[0], e[1], e[2], e[3]]
                            .into_iter()
                            .chain(mesh.to_le_bytes())
                    })
                    .collect();
                put("scene+0x1140[]", Some(entries));
            }
        }
    }
    let app = word(crate::rebase(G_PHYRE_APPLICATION)).unwrap_or(0);
    // Ambient, fog colour, fog near/far/limit, background colour.
    put("app+0x50", bytes(app + 0x50, 0x40));
    let post = word(crate::rebase(G_POST_PROCESS_MANAGER)).unwrap_or(0);
    // Fade colour (4 floats), alpha mode, lock byte, last reason.
    put("fade+0x1238c", bytes(post + 0x1_238c, 0x1c));
    out
}
