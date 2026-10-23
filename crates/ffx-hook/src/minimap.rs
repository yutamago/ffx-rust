//! The field minimap (`rcbgGuide`, the guide map), for the renderer's own.
//!
//! `rcbgGuideUpdateEx` (`0x0091da80`, `cdecl(guide, show)`, from `0x00919d00`
//! each field frame) fades the minimap, reveals the fog of war, and draws
//! the map's mesh and its 2D quads. While the renderer draws its own
//! minimap (its answer's flag 512), the wrapper does only the first two
//! (the fade `0x0091d280`, the fog `0x0091f420`, the blink counter) with
//! the game's mesh and fog passes off (`0x006425f0(0)`, `0x006425d0(0)`),
//! and reads what the renderer needs: the fade, the markers through
//! `AtelGetGuideMapActorInfo` (`0x0086b740`, on this thread as the game
//! calls it), the player among them (flag 0x800), and the mesh when it
//! changes.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::log::log;

/// The renderer draws its own minimap.
pub static REPLACE: AtomicBool = AtomicBool::new(false);

struct State {
    /// The frame (presents counted) of the last update.
    frame: usize,
    data: ffx_gfx::Minimap,
    /// The mesh object read last, and a new mesh not yet sent.
    mesh_at: usize,
    pending: Option<ffx_gfx::MapMesh>,
    meshes: u32,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

const DEBUG_ENABLE: u32 = 0x00c5_e478;
const GUIDE_FLAGS: u32 = 0x00c5_e350;
const FADE: u32 = 0x0193_bf78;
const FADE_STEP: u32 = 0x0091_d280;
const FADE_OFF: u32 = 0x0091_d3a0;
const MESH_PASS: u32 = 0x0064_25f0;
const FOG_PASS: u32 = 0x0064_25d0;
const MARKER_COUNT: u32 = 0x0086_b710;
const MARKER: u32 = 0x0086_b740;
const FOG_REVEAL: u32 = 0x0091_f420;
const AFTER_DRAW: u32 = 0x008e_5cd0;

/// Replaces `rcbgGuideUpdateEx` (see the module's notes). The game's own
/// minimap runs the Rust port (`ffx_field::guide_update`) and returns its
/// `eax` (`rcbgGuideUpdate` hands it on); the renderer's returns 0.
///
/// # Safety
/// Called by the game, on its thread.
pub unsafe extern "C" fn guide_update(guide: usize, show: u32) -> u32 {
    if !REPLACE.load(Ordering::Acquire) || guide == 0 {
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        return crate::ported::rcbg_guide_update_ex(guide as u32, show);
    }
    // SAFETY: the game's own functions and globals, as the original uses
    // them, on its thread.
    unsafe { replace(guide, show) };
    0
}

unsafe fn replace(guide: usize, show: u32) {
    let read = |a: u32| unsafe { std::ptr::read_volatile(crate::rebase(a) as *const u32) };
    // SAFETY: as the caller documents.
    unsafe {
        let mesh = mesh_object(guide);
        if read(DEBUG_ENABLE) != 0 && mesh != 0 && read(GUIDE_FLAGS) & 1 != 0 {
            let step: extern "C" fn(u32) = std::mem::transmute(crate::rebase(FADE_STEP));
            step(show);
        } else if read(GUIDE_FLAGS) & 1 == 0 {
            let off: extern "C" fn() = std::mem::transmute(crate::rebase(FADE_OFF));
            off();
        }
        let pass: extern "C" fn(u32) = std::mem::transmute(crate::rebase(MESH_PASS));
        pass(0);
        let fog: extern "C" fn(u32) = std::mem::transmute(crate::rebase(FOG_PASS));
        fog(0);
        // The markers.
        let count: extern "C" fn() -> u32 = std::mem::transmute(crate::rebase(MARKER_COUNT));
        let get: extern "C" fn(u32, *mut u8) -> u16 = std::mem::transmute(crate::rebase(MARKER));
        let n = count().min(256);
        let mut markers = Vec::with_capacity(n as usize);
        let mut player = None;
        for i in 0..n {
            let mut out = [0u8; 0x40];
            get(i, out.as_mut_ptr());
            let f = |o: usize| f32::from_le_bytes(out[o..o + 4].try_into().expect("4"));
            let flags = u32::from_le_bytes(out[4..8].try_into().expect("4"));
            if flags == 0 {
                continue;
            }
            let pos = [f(8), f(0xc), f(0x10)];
            if !pos.iter().all(|v| v.is_finite()) {
                continue;
            }
            if flags & 0x800 != 0 {
                player = Some((pos, f(0x14)));
            }
            let line = (out[0x2c] != 0).then(|| [f(0x1c), f(0x20), f(0x24), f(0x28)]);
            markers.push(ffx_gfx::MapMarker {
                flags,
                color: color((flags & 0xf) as usize),
                pos,
                line,
            });
        }
        // The fog of war (four maps), with the player's position.
        if let Some((p, _)) = player
            && std::ptr::read_volatile((guide + 0x70) as *const u32) != 0
        {
            let reveal: extern "C" fn(usize, f32, f32) =
                std::mem::transmute(crate::rebase(FOG_REVEAL));
            reveal(guide, p[0], p[2]);
        }
        let blink = (guide + 0x74) as *mut u32;
        std::ptr::write_volatile(blink, std::ptr::read_volatile(blink).wrapping_add(1));
        let after: extern "C" fn() = std::mem::transmute(crate::rebase(AFTER_DRAW));
        after();
        let shown = f32::from(u8::try_from(read(FADE).min(128)).unwrap_or(0)) / 128.0;
        let Ok(mut s) = STATE.lock() else { return };
        let s = s.get_or_insert_with(|| State {
            frame: 0,
            data: ffx_gfx::Minimap::default(),
            mesh_at: 0,
            pending: None,
            meshes: 0,
        });
        if mesh != s.mesh_at {
            s.mesh_at = mesh;
            s.meshes += 1;
            s.pending = (mesh != 0).then(|| read_mesh(mesh, s.meshes)).flatten();
            if let Some(m) = &s.pending {
                log(&format!(
                    "  minimap: mesh {} ({} vertices, {} triangles)",
                    m.id,
                    m.vertices.len(),
                    m.triangles.len()
                ));
            }
        }
        s.frame = crate::gfx::stats().0 as usize;
        s.data.shown = shown;
        if let Some((p, yaw)) = player {
            s.data.player = p;
            s.data.yaw = yaw;
        }
        s.data.markers = markers;
    }
}

/// The guide's mesh object (`0x0091d460`): its own, else the one made
/// from the walkmesh.
unsafe fn mesh_object(guide: usize) -> usize {
    // SAFETY: the guide object's first words.
    unsafe {
        let own = std::ptr::read_volatile(guide as *const usize);
        if own != 0 {
            own
        } else {
            std::ptr::read_volatile((guide + 8) as *const usize)
        }
    }
}

/// The marker palette (`0x00c5e370`: 16 RGBA of floats, 128 = 1).
fn color(index: usize) -> [f32; 4] {
    const PALETTE: u32 = 0x00c5_e370;
    crate::harness::read_memory(crate::rebase(PALETTE) + index * 16, 16).map_or([1.0; 4], |b| {
        let c = |o: usize| {
            (f32::from_le_bytes(b[o..o + 4].try_into().expect("4")) / 128.0).clamp(0.0, 1.0)
        };
        [c(0), c(4), c(8), c(12)]
    })
}

/// Bytes per primitive record of each packet type (`0x00c5e790`).
const STRIDES: [usize; 8] = [0x14, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30];

/// The mesh (`rcbgModel`): the vertex→world matrix at +0xc0, the blob at
/// +0x14 (+4 packet list, +8 vertices, +0x12 vertex count; packets of type
/// 0: triangles of three vertex indices at +0xc).
fn read_mesh(mesh: usize, id: u32) -> Option<ffx_gfx::MapMesh> {
    let word = |a: usize| -> Option<u32> {
        let b = crate::harness::read_memory(a, 4).ok()?;
        Some(u32::from_le_bytes(b.try_into().ok()?))
    };
    let mut matrix = [0.0f32; 16];
    let raw = crate::harness::read_memory(mesh + 0xc0, 64).ok()?;
    for (k, v) in raw.as_chunks::<4>().0.iter().enumerate() {
        matrix[k] = f32::from_le_bytes(*v);
    }
    let blob = word(mesh + 0x14)? as usize;
    if blob == 0 {
        return None;
    }
    let head = crate::harness::read_memory(blob, 0x14).ok()?;
    let at = |o: usize| u32::from_le_bytes(head[o..o + 4].try_into().expect("4")) as usize;
    let count = usize::from(u16::from_le_bytes([head[0x12], head[0x13]]).min(0x4000));
    let vb = crate::harness::read_memory(blob + at(8), count * 6).ok()?;
    let vertices: Vec<[i16; 3]> = vb
        .as_chunks::<6>()
        .0
        .iter()
        .map(|c| [0, 2, 4].map(|o| i16::from_le_bytes([c[o], c[o + 1]])))
        .collect();
    let mut triangles = Vec::new();
    let mut packet = blob + at(4);
    for _ in 0..64 {
        let h = crate::harness::read_memory(packet, 16).ok()?;
        if u16::from_le_bytes([h[0], h[1]]) == 0xffff {
            break;
        }
        let kind = usize::from(h[1]);
        let n = usize::from(u16::from_le_bytes([h[2], h[3]]).min(0x4000));
        let stride = *STRIDES.get(kind)?;
        packet += 16;
        if kind == 0 {
            let recs = crate::harness::read_memory(packet, n * stride).ok()?;
            for r in recs.chunks_exact(stride) {
                let i = |o: usize| u16::from_le_bytes([r[o], r[o + 1]]);
                let t = [i(0xc), i(0xe), i(0x10)];
                if t.iter().all(|&v| usize::from(v) < vertices.len()) {
                    triangles.push(t);
                }
            }
        }
        packet += n * stride;
    }
    Some(ffx_gfx::MapMesh {
        id,
        matrix,
        vertices,
        triangles,
    })
}

/// The message for the renderer, before a frame: the minimap as this
/// frame's update left it (hidden when the game did not update it).
pub fn msg() -> Option<ffx_gfx::Msg> {
    if !REPLACE.load(Ordering::Acquire) {
        return None;
    }
    let mut s = STATE.lock().ok()?;
    let s = s.as_mut()?;
    let mut data = s.data.clone();
    // Updated in the last two frames, or the game shows none.
    if (crate::gfx::stats().0 as usize).saturating_sub(s.frame) > 2 {
        data.shown = 0.0;
        data.markers.clear();
    }
    data.mesh = s.pending.take();
    Some(ffx_gfx::Msg::Minimap(data))
}

/// A new renderer has no mesh: send it again.
pub fn resend() {
    if let Ok(mut s) = STATE.lock()
        && let Some(s) = s.as_mut()
    {
        s.mesh_at = 0;
    }
}
