//! The scene builder in the game's map path (M4 round 2, m4-wire;
//! `docs/scene-builder.md`, "In the game"): with `FFX_RUST_MAP=build`
//! the map passes of the supported maps are built by `ffx_scene::build`
//! from the live inputs, and drawn through `ffx-draw` in place of
//! `PhyreEngine`'s (`super::scene`'s builder entry).
//!
//! - Main thread, at the map's `renderWorld` (update N): the pass's inputs
//!   are read as the capture snapshot reads them
//!   (`crate::world::scene_inputs_at`), the map's clusters are loaded once
//!   through the game's own file functions (the archive, or a mod's file),
//!   and the pass is built (`ffx_scene::live`, `build::build_into` onto one
//!   `Resources` per map, so the objects keep their handles).
//! - Render thread, at the end of the pass (frame N): the job makes the
//!   objects that are new (`Uploaded::extend`, once per object) and turns
//!   each built draw into a `PassDraw`. The host's objects (the scene
//!   targets, run-time textures such as the shadow maps, the dynamic
//!   vertices of morphing meshes) are the game's own draw's in the same
//!   place: the k-th built draw takes them from the k-th draw `PhyreEngine`
//!   made for the pass (recorded, not drawn). When the counts differ the
//!   game's own draws are issued instead (and counted).
//!
//! A map the builder does not support yet, or a pass it can't build,
//! returns `None`: the recorder draws it (`FFX_RUST_MAP=1`'s path).
//! Never panics (it runs inside the game's frame).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ffx_draw::backend::View;
use ffx_draw::cache::Cache;
use ffx_draw::d3d11::D3d11;
use ffx_draw::pass::{Constants, Pass, PassDraw};
use ffx_scene::build::{Clusters, MapDraw, Resources, ViewSource, build_into};
use ffx_scene::frame::{Settings, TextureQuality};
use ffx_scene::live::{Fallback, LiveInputs};
use ffx_scene::upload::{HOST, Uploaded};

use super::scene::{Builder, Job, PassInfo};
use crate::log::log;

/// The maps the builder draws, by their model's symbol (`PhyreScene
/// +0xd8`). Others go to the recorder.
const SUPPORTED: [&str; 2] = ["cdsp00", "bsil00"];

/// `PApplication_renderWorld` (`0x0062be70`) calls
/// `renderWorldCulledSorted` on `app +0x124` (the renderer, whose `+0x48`
/// names the pass) with the context `app +0x40`.
const RENDERER: usize = 0x124;

/// The game's file functions (`symbols/ffx.sym`): open (hooked for
/// mods), size, read, close, and the file system whose `+0x3c` is the
/// length of the root prefix the archive lookup skips.
const FILE_OPEN: u32 = 0x0060_7f40;
const FILE_SIZE: u32 = 0x0060_7dc0;
const FILE_READ: u32 = 0x0060_8090;
const FILE_CLOSE: u32 = 0x0060_7d80;
const G_FILE_SYS: u32 = 0x00cc_9c48;

/// A map's built state: its models, the builder's objects and their
/// backend handles (the render thread's).
struct MapState {
    clusters: Clusters,
    resources: Arc<Mutex<Resources>>,
    uploaded: Arc<Mutex<Option<Uploaded>>>,
}

/// The other pass of the last update built: its draws wait for its call.
struct Waiting {
    update: u32,
    pass: Pass,
    draws: Vec<MapDraw>,
    resources: Arc<Mutex<Resources>>,
    uploaded: Arc<Mutex<Option<Uploaded>>>,
}

/// The main thread's state.
#[derive(Default)]
struct Main {
    maps: HashMap<Vec<String>, MapState>,
    failed: Vec<Vec<String>>,
    waiting: Option<Waiting>,
}

fn make_job(
    draws: Vec<MapDraw>,
    resources: Arc<Mutex<Resources>>,
    uploaded: Arc<Mutex<Option<Uploaded>>>,
) -> Job {
    Box::new(move |b: &mut D3d11, _c: &mut Cache, host: &[PassDraw]| {
        job(b, &resources, &uploaded, &draws, host)
    })
}

static BUILT: AtomicU64 = AtomicU64::new(0);
static FELL_BACK: AtomicU64 = AtomicU64::new(0);
static MISMATCHED: AtomicU64 = AtomicU64::new(0);

/// The builder for `super::scene::set_builder`.
pub fn builder() -> Builder {
    let mut main = Main::default();
    Box::new(move |info: &PassInfo| main.pass(info))
}

/// Logs at the first and every power-of-two count.
fn counted(counter: &AtomicU64, what: &str) {
    let n = counter.fetch_add(1, Ordering::Relaxed) + 1;
    if n.is_power_of_two() {
        log(&format!("  gfx: map builder: {what} ({n})"));
    }
}

/// A file through the game's file functions, as the game reads it: the
/// hooked open (a mod's file first, then the archive), its size, the
/// bytes, close. `None` when it is not there.
fn read_game_file(archive_path: &str) -> Option<Vec<u8>> {
    type Open = unsafe extern "thiscall" fn(*mut usize, *const u8, u32, u32, u32, u32) -> u32;
    type Size = unsafe extern "thiscall" fn(*mut usize) -> u32;
    type Read = unsafe extern "thiscall" fn(*mut usize, *mut u8, u32) -> u32;
    type Close = unsafe extern "thiscall" fn(*mut usize) -> u32;
    let fs = crate::harness::read_memory(crate::rebase(G_FILE_SYS), 4).ok()?;
    let fs = u32::from_le_bytes([fs[0], fs[1], fs[2], fs[3]]) as usize;
    if fs == 0 {
        return None;
    }
    let root = crate::harness::read_memory(fs + 0x3c, 4).ok()?;
    let root = u32::from_le_bytes([root[0], root[1], root[2], root[3]]) as usize;
    // A relative prefix of the root's length ("./" pairs, a leading "."
    // when odd): the archive lookup and the mod index skip it.
    let mut path = String::new();
    if root % 2 == 1 {
        path.push('.');
    }
    path.push_str(&"./".repeat(root / 2));
    path.push_str(archive_path);
    path.push('\0');
    let mut file = [usize::MAX, 0usize];
    // SAFETY: the game's thiscall file functions on a File of two words
    // (handle, archive entry), as the game's callers pass them (open
    // with mode 1 and `0, 0, 1`); the buffer is as long as the size given.
    unsafe {
        let open: Open = std::mem::transmute(crate::rebase(FILE_OPEN));
        let size: Size = std::mem::transmute(crate::rebase(FILE_SIZE));
        let read: Read = std::mem::transmute(crate::rebase(FILE_READ));
        let close: Close = std::mem::transmute(crate::rebase(FILE_CLOSE));
        open(file.as_mut_ptr(), path.as_ptr(), 1, 0, 0, 1);
        if file[0] == usize::MAX && file[1] == 0 {
            return None;
        }
        let n = size(file.as_mut_ptr()) as usize;
        let mut out = vec![0u8; n];
        let mut at = 0;
        while at < n {
            let got = read(
                file.as_mut_ptr(),
                out.as_mut_ptr().add(at),
                u32::try_from(n - at).unwrap_or(u32::MAX),
            ) as usize;
            if got == 0 {
                break;
            }
            at += got;
        }
        close(file.as_mut_ptr());
        (at == n).then_some(out)
    }
}

impl Main {
    /// One map pass. Both passes of an update are built at its first map
    /// call (the inputs are the same at both: camera, worlds, materials);
    /// the other's draws wait for its call.
    #[allow(clippy::too_many_lines)] // one pass: inputs, models, build
    fn pass(&mut self, info: &PassInfo) -> Option<Job> {
        // Other maps leave at once (the snapshot below is not free).
        let symbol = crate::world::map_symbol()?;
        if !SUPPORTED.contains(&symbol.as_str()) {
            return None;
        }
        let name = crate::world::renderer_pass(info.app + RENDERER);
        if !name.starts_with("Opaque") && !name.starts_with("Transparent") {
            return None;
        }
        let update = crate::harness::update_label();
        if let Some(w) = self.waiting.take()
            && w.update == update
            && w.pass == info.pass
        {
            return Some(make_job(w.draws, w.resources, w.uploaded));
        }
        let t = std::time::Instant::now();
        let (s, wire) = crate::world::with_direct_reads(|| {
            crate::world::scene_inputs_at(info.app + RENDERER, info.world, info.camera, info.dir)
        })?;
        let read_us = t.elapsed().as_micros();
        let counts = crate::world::direct_read_counts();
        let phases = crate::world::snapshot_phases();
        let supported = s
            .clusters
            .iter()
            .any(|c| SUPPORTED.contains(&c.name.as_str()));
        if !supported {
            return None;
        }
        let records = [(s, Some(wire))];
        let models = ffx_scene::gate::model_paths(&records);
        if models.is_empty() || self.failed.contains(&models) {
            return None;
        }
        if !self.maps.contains_key(&models) {
            let refs: Vec<&str> = models.iter().map(String::as_str).collect();
            let t = std::time::Instant::now();
            match Clusters::load_models(read_game_file, &refs) {
                Ok(clusters) => {
                    log(&format!(
                        "  gfx: map builder: {} loaded in {:.2} s",
                        models.join(", "),
                        t.elapsed().as_secs_f64()
                    ));
                    self.maps.insert(
                        models.clone(),
                        MapState {
                            clusters,
                            resources: Arc::new(Mutex::new(Resources::default())),
                            uploaded: Arc::new(Mutex::new(None)),
                        },
                    );
                }
                Err(e) => {
                    log(&format!(
                        "  gfx: map builder: {}: {e}; the recorder draws it",
                        models.join(", ")
                    ));
                    self.failed.push(models);
                    return None;
                }
            }
        }
        let map = self.maps.get_mut(&models)?;
        let settings = Settings {
            texture_quality: TextureQuality::Low,
            anisotropy: 1.0,
        };
        let live = match LiveInputs::new(&map.clusters, &records, settings, &Fallback::default()) {
            Ok(l) => l,
            Err(e) => {
                counted(&FELL_BACK, &format!("no inputs: {e}"));
                return None;
            }
        };
        if let Err(e) = map
            .clusters
            .add_textures(read_game_file, &live.texture_imports())
        {
            counted(&FELL_BACK, &format!("textures: {e}"));
            return None;
        }
        let built = {
            let mut r = map.resources.lock().ok()?;
            let had = std::mem::take(&mut *r);
            match build_into(&map.clusters, &live, had) {
                Ok(b) => b,
                Err(e) => {
                    // The objects are gone with the failed build: start
                    // over (the backend's earlier objects stay unused).
                    if let Ok(mut u) = map.uploaded.lock() {
                        *u = None;
                    }
                    counted(&FELL_BACK, &format!("build: {e}"));
                    return None;
                }
            }
        };
        let of = |p: Pass| match p {
            Pass::Opaque => ffx_scene::build::Pass::Opaque,
            Pass::Transparent => ffx_scene::build::Pass::Transparent,
        };
        let other = match info.pass {
            Pass::Opaque => Pass::Transparent,
            Pass::Transparent => Pass::Opaque,
        };
        let draws: Vec<MapDraw> = built.pass(of(info.pass)).cloned().collect();
        let later: Vec<MapDraw> = built.pass(of(other)).cloned().collect();
        if let Ok(mut r) = map.resources.lock() {
            *r = built.resources;
        }
        self.waiting = Some(Waiting {
            update,
            pass: other,
            draws: later,
            resources: Arc::clone(&map.resources),
            uploaded: Arc::clone(&map.uploaded),
        });
        let n = BUILT.fetch_add(1, Ordering::Relaxed) + 1;
        if n.is_power_of_two() {
            log(&format!(
                "  gfx: map builder: {n} passes built ({symbol}: inputs {read_us} us ({} reads, {} pages, parts {phases:?} us), all {} us, {} draws)",
                counts.0,
                counts.1,
                t.elapsed().as_micros(),
                draws.len()
            ));
        }
        Some(make_job(
            draws,
            Arc::clone(&map.resources),
            Arc::clone(&map.uploaded),
        ))
    }
}

/// The render-thread half: new objects made, the built draws on the
/// backend's handles with the host's objects from the game's draw in the
/// same place.
fn job(
    b: &mut D3d11,
    resources: &Mutex<Resources>,
    uploaded: &Mutex<Option<Uploaded>>,
    draws: &[MapDraw],
    host: &[PassDraw],
) -> Vec<PassDraw> {
    if draws.len() != host.len() {
        counted(
            &MISMATCHED,
            &format!(
                "{} built draws for {} of the game's: the game's are drawn",
                draws.len(),
                host.len()
            ),
        );
        return host.to_vec();
    }
    let (Ok(r), Ok(mut up)) = (resources.lock(), uploaded.lock()) else {
        return host.to_vec();
    };
    let up = up.get_or_insert_with(Uploaded::default);
    if let Err(e) = up.extend(b, &r, |_| Some(View(HOST))) {
        counted(&FELL_BACK, &format!("upload: {e}"));
        return host.to_vec();
    }
    let external = |v: Option<View>| {
        v.and_then(|v| r.views.get(v.0 as usize))
            .is_some_and(|x| matches!(x.source, ViewSource::External(_)))
    };
    draws
        .iter()
        .zip(host)
        .map(|(d, h)| {
            let mut state = up.state(&d.state);
            state.targets = h.state.targets;
            state.depth_target = h.state.depth_target;
            for (out, built, game) in [
                (&mut state.vs, &d.state.vs, &h.state.vs),
                (&mut state.ps, &d.state.ps, &h.state.ps),
            ] {
                for (k, v) in built.srvs.iter().enumerate() {
                    if external(*v)
                        && let (Some(o), Some(g)) = (out.srvs.get_mut(k), game.srvs.get(k))
                    {
                        *o = *g;
                    }
                }
            }
            for (k, v) in d.state.vertex.iter().enumerate() {
                if v.is_some_and(|v| r.external_buffers.contains_key(&v.buffer.0))
                    && let (Some(o), Some(g)) = (state.vertex.get_mut(k), h.state.vertex.get(k))
                {
                    *o = *g;
                }
            }
            PassDraw {
                state,
                call: d.call,
                constants: d
                    .constants
                    .iter()
                    .map(|(stage, slot, data)| Constants {
                        stage: *stage,
                        slot: *slot,
                        data: data.clone(),
                    })
                    .collect(),
                writes: Vec::new(),
            }
        })
        .collect()
}
