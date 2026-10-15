//! What the game is showing, for the renderer: the scene (the event file
//! of the map, the last `.ebp` the game opened) and the camera.

use std::sync::Mutex;

/// `g_Camera`: the active camera channel (`symbols/ffx.sym`).
const G_CAMERA: u32 = 0x0231_1430;
/// Offsets in a camera channel: the look-at point and the eye.
const LOOK_AT: usize = 0x50;
const EYE: usize = 0x61c + 0x50;

static SCENE: Mutex<String> = Mutex::new(String::new());

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
}

fn read3(at: usize) -> Option<[f32; 3]> {
    // SAFETY: inside the camera channel table of the game's image (a
    // static block), read as plain floats.
    let v = unsafe { [0, 4, 8].map(|o| std::ptr::read_unaligned((at + o) as *const f32)) };
    v.iter()
        .all(|x| x.is_finite() && x.abs() < 1e7)
        .then_some(v)
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
