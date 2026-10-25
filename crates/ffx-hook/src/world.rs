//! What the game is showing, for the renderer: the scene (the event file
//! of the map, the last `.ebp` the game opened) and the camera; and, for
//! reference captures (`gfx::capture`), the scene inputs a Rust scene
//! builder reads, sampled from the game's memory (`docs/renderer.md`,
//! "Reference captures").

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

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

thread_local! {
    /// While set (the in-game builder's reads on the main thread,
    /// [`with_direct_reads`]) reads are made directly.
    static DIRECT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Regions checked in the current direct run (`VirtualQuery`'s
    /// ranges: start, end, readable), the last readable one first.
    static REGIONS: std::cell::RefCell<Vec<(usize, usize, bool)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Reads and page checks of the last [`with_direct_reads`].
    static COUNTS: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
    /// Microseconds of the last snapshot's parts: lists, bounds, the rest,
    /// the `Wire` block.
    static PHASES: std::cell::Cell<[u128; 4]> = const { std::cell::Cell::new([0; 4]) };
}

/// The last snapshot's parts' times (microseconds): lists, bounds and
/// instances, switches and depth bias, the `Wire` block.
#[must_use]
pub fn snapshot_phases() -> [u128; 4] {
    PHASES.with(std::cell::Cell::get)
}

/// The reads and page checks the last [`with_direct_reads`] made.
#[must_use]
pub fn direct_read_counts() -> (u64, u64) {
    COUNTS.with(std::cell::Cell::get)
}

/// Runs `f` with this thread's reads made directly, each page checked
/// once with `VirtualQuery` (committed, readable, not a guard page)
/// instead of a `ReadProcessMemory` per read: the snapshot takes about
/// 10 ms a pass through `ReadProcessMemory`, too slow for the game's
/// frame (`gfx::map_build`).
pub fn with_direct_reads<T>(f: impl FnOnce() -> T) -> T {
    REGIONS.with(|r| r.borrow_mut().clear());
    COUNTS.with(|c| c.set((0, 0)));
    DIRECT.with(|d| d.set(true));
    let out = f();
    DIRECT.with(|d| d.set(false));
    REGIONS.with(|r| r.borrow_mut().clear());
    out
}

/// The region `VirtualQuery` reports around `at`: its start, end and
/// whether it is committed and readable.
fn region(at: usize) -> (usize, usize, bool) {
    use windows_sys::Win32::System::Memory::{
        MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS, VirtualQuery,
    };
    // SAFETY: a zeroed out-structure of the size passed.
    let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: VirtualQuery only reads the address space's bookkeeping.
    let n = unsafe {
        VirtualQuery(
            at as *const std::ffi::c_void,
            &raw mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if n == 0 {
        let page = at & !0xfff;
        return (page, page + 0x1000, false);
    }
    let start = mbi.BaseAddress as usize;
    let ok = mbi.State == MEM_COMMIT
        && mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD) == 0
        && mbi.Protect != 0;
    (start, start.saturating_add(mbi.RegionSize), ok)
}

/// In direct mode (`None` otherwise): whether `len` bytes at `at` are
/// readable. Each region is asked about once a run (`VirtualQuery`
/// gives a whole region of like pages: a few calls cover the map's
/// data).
fn readable(at: usize, len: usize) -> Option<bool> {
    if !DIRECT.with(std::cell::Cell::get) {
        return None;
    }
    let end = at.checked_add(len.max(1))?;
    COUNTS.with(|c| c.set((c.get().0 + 1, c.get().1)));
    REGIONS.with(|r| {
        let mut r = r.borrow_mut();
        let mut from = at;
        while from < end {
            let found = r.iter().position(|&(s, e, _)| s <= from && from < e);
            let k = match found {
                Some(k) => k,
                None => {
                    COUNTS.with(|c| c.set((c.get().0, c.get().1 + 1)));
                    r.push(region(from));
                    r.len() - 1
                }
            };
            let (_, e, ok) = r[k];
            if !ok {
                return Some(false);
            }
            if k != 0 {
                r.swap(0, k);
            }
            from = e;
        }
        Some(true)
    })
}

fn bytes(at: usize, len: usize) -> Option<Vec<u8>> {
    if at == 0 {
        return None;
    }
    match readable(at, len) {
        // SAFETY: every page of the range is committed and readable (the
        // game frees no map data while its renderWorld runs).
        Some(true) => Some(unsafe { std::slice::from_raw_parts(at as *const u8, len) }.to_vec()),
        Some(false) => None,
        None => crate::harness::read_memory(at, len).ok(),
    }
}

fn word(at: usize) -> Option<usize> {
    if at == 0 {
        return None;
    }
    match readable(at, 4) {
        // SAFETY: as in `bytes`.
        Some(true) => Some(unsafe { std::ptr::read_unaligned(at as *const u32) } as usize),
        Some(false) => None,
        None => {
            let b = crate::harness::read_memory(at, 4).ok()?;
            Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        }
    }
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

// The scene builder's inputs at `renderWorld` (M4 round 2,
// `docs/formats/scene.md`, "Scene inputs").

/// The trampoline to `Phyre_PRendering_PRenderer_renderWorldCulledSorted`.
pub static RENDER_WORLD_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

/// The map's world in `PhyreScene` (`PhyreScene_fieldMapLoad`).
pub const MAP_WORLD: u32 = 0xac;
/// The `PMeshInstanceBounds` class descriptor (the cursor
/// `renderWorldCulledSorted` makes with `0x004ff6e0` and hands to
/// `startJobsForAllBounds`; `0x01a85420`, the other cursor's, is
/// `POccluderGeometryInstance`).
const BOUNDS_CLASS: u32 = 0x00ca_89b8;
/// `g_DepthBias_Count`, `g_DepthBias` (256 entries of 12 bytes).
const DEPTH_BIAS_COUNT: u32 = 0x0194_2998;
const DEPTH_BIAS: u32 = 0x0194_299c;
/// The screen size `PSceneWideParameters_getBuffer` reads.
const SCREEN_SIZE: u32 = 0x00c9_5100;
/// Limits on what is walked (a damaged list must not hang the frame).
const LISTS_MAX: usize = 4096;
const OBJECTS_MAX: u32 = 1 << 16;
const SEGMENTS_MAX: u32 = 4096;
const PARAMETERS_MAX: u32 = 1 << 16;

/// Wraps `Phyre_PRendering_PRenderer_renderWorldCulledSorted`
/// (`0x009f3c30`, `thiscall(renderer, world, camera, context, dir)`, the
/// body of `PApplication_renderWorld`): while a capture runs, the map
/// world's inputs are read first (before the renderer culls). The game
/// calls it on the main thread, after the update: `renderWorld` records
/// `PhyreEngine`'s commands, which the render thread turns into Direct3D
/// calls a frame later.
///
/// # Safety
/// Called by the game.
pub unsafe extern "thiscall" fn render_world(
    renderer: *mut std::ffi::c_void,
    world: u32,
    camera: u32,
    context: u32,
    dir: i32,
) -> u32 {
    if crate::gfx::capture::inputs_wanted() && world_offset(world as usize) == MAP_WORLD {
        if let Some((s, wire)) =
            scene_inputs_at(renderer as usize, world as usize, camera as usize, dir)
        {
            crate::gfx::capture::add_inputs(s, Some(wire));
        }
        if watch::wanted() && !watch::armed() {
            watch::arm(&watch_targets(world as usize));
        }
    }
    // SAFETY: the trampoline runs the original with the same arguments.
    unsafe {
        let original: unsafe extern "thiscall" fn(
            *mut std::ffi::c_void,
            u32,
            u32,
            u32,
            i32,
        ) -> u32 = std::mem::transmute(RENDER_WORLD_ORIGINAL.load(Ordering::Acquire));
        original(renderer, world, camera, context, dir)
    }
}

/// The world's offset in the scene (`0xac`, `0x7c`, `0xbc`, ...), or 0.
fn world_offset(world: usize) -> u32 {
    let scene = word(crate::rebase(G_PHYRE_SCENE)).unwrap_or(0);
    match world.checked_sub(scene) {
        Some(off @ (0x7c | 0x8c | 0xac | 0xbc)) if scene != 0 => off as u32,
        _ => 0,
    }
}

fn u32_at(at: usize) -> Option<u32> {
    word(at).map(|w| w as u32)
}

fn floats<const N: usize>(at: usize) -> Option<[f32; N]> {
    let b = bytes(at, 4 * N)?;
    let mut out = [0.0f32; N];
    for (o, c) in out.iter_mut().zip(b.as_chunks::<4>().0) {
        *o = f32::from_le_bytes(*c);
    }
    Some(out)
}

/// A NUL-terminated string (at most 255 bytes), read up to 16 bytes at a
/// time without crossing a 16-byte boundary (so never into a next page).
fn cstr(at: usize) -> Option<String> {
    let mut out = Vec::new();
    let mut p = at;
    while out.len() < 255 {
        let b = bytes(p, 16 - (p & 15))?;
        if let Some(end) = b.iter().position(|&c| c == 0) {
            out.extend_from_slice(&b[..end]);
            break;
        }
        out.extend_from_slice(&b);
        p += b.len();
    }
    Some(String::from_utf8_lossy(&out).into_owned())
}

/// A pass name: a pointer to a static that holds the name's pointer
/// (`renderer +0x48`, a material's remapping); empty for 0.
fn pass_name(v: usize) -> String {
    (v != 0)
        .then(|| word(v).and_then(cstr))
        .flatten()
        .unwrap_or_default()
}

/// One `PInstanceList` (`Phyre_PInstanceList_PInstanceList` `0x00576270`,
/// `0x005763b0`): `+8` the free slots (ascending, each linking the next),
/// `+0x28` the cluster, `+0x2c` the class (`+0x18` its name, `+0x1c` the
/// object size), `+0x34` the objects, `+0x3c` their count with free
/// slots.
struct List {
    cluster: usize,
    class: String,
    base: usize,
    count: u32,
    stride: u32,
    free: usize,
}

impl List {
    fn at(list: usize, names: &mut std::collections::HashMap<usize, String>) -> Option<Self> {
        let class = word(list + 0x2c)?;
        let name = names
            .entry(class)
            .or_insert_with(|| word(class + 0x18).and_then(cstr).unwrap_or_default())
            .clone();
        Some(Self {
            cluster: word(list + 0x28)?,
            class: name,
            base: word(list + 0x34)?,
            count: u32_at(list + 0x3c)?.min(OBJECTS_MAX),
            stride: u32_at(class + 0x1c)?,
            free: word(list + 8)?,
        })
    }

    /// The live objects with their slots, free slots skipped as the
    /// renderer's cursor does (`0x009f3930`, `0x009f3dc0`).
    fn objects(&self) -> Vec<(usize, u32)> {
        let mut out = Vec::new();
        let mut free = self.free;
        for k in 0..self.count {
            let obj = self.base + (k * self.stride) as usize;
            if obj == free {
                free = word(free).unwrap_or(0);
                continue;
            }
            out.push((obj, k));
        }
        out
    }

    fn slot_of(&self, p: usize) -> Option<u32> {
        let off = p.checked_sub(self.base)?;
        let stride = self.stride as usize;
        (stride != 0 && off % stride == 0 && off / stride < self.count as usize)
            .then_some((off / stride) as u32)
    }
}

/// The world's clusters (`PWorld +0`: a ring of the clusters' `+0x48`
/// links, newest first, `Phyre_PWorld_addCluster` `0x00443250`).
fn world_clusters(world: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut node = word(world).unwrap_or(world);
    while node != world && node >= 0x48 && out.len() < LISTS_MAX {
        out.push(node - 0x48);
        node = word(node).unwrap_or(world);
    }
    out
}

/// A cluster's instance lists (`+0x1c`: a ring through each list's `+0`).
fn cluster_lists(cluster: usize) -> Vec<usize> {
    let head = cluster + 0x1c;
    let mut out = Vec::new();
    let mut node = word(head).unwrap_or(head);
    while node != head && node != 0 && out.len() < LISTS_MAX {
        out.push(node);
        node = word(node).unwrap_or(head);
    }
    out
}

/// The instance lists of a class and the classes below it, in the order
/// the renderer walks them (`0x009f23f0` with `0x00443ef0`): the world's
/// class table (`+0xc`, `+8` entries of `0x14` bytes, by the class's id
/// `+0x80`) chains each class's lists (through their `+0x20` links) into
/// the next entry; the walk goes on into the next entries while their
/// depth (`+0x10`) is greater than the class's.
fn world_lists_of(world: usize, class: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let (Some(id), Some(n), Some(table)) =
        (u32_at(class + 0x80), u32_at(world + 8), word(world + 0xc))
    else {
        return out;
    };
    if id >= n & 0x7fff_ffff || table == 0 {
        return out;
    }
    let entry = table + id as usize * 0x14;
    let depth = u32_at(entry + 0x10).unwrap_or(0).cast_signed();
    let mut end = entry + 0x14;
    let next = |node: usize| match word(node) {
        Some(n) if n != node => n,
        _ => 0,
    };
    let mut node = next(entry);
    while out.len() < LISTS_MAX {
        while node != 0 && node == end {
            node = if depth < u32_at(end + 0x10).unwrap_or(0).cast_signed() {
                next(node)
            } else {
                0
            };
            end += 0x14;
        }
        if node < 0x20 {
            break;
        }
        out.push(node - 0x20);
        node = next(node);
    }
    out
}

/// What [`scene_inputs_at`] resolves pointers with: the instance lists of
/// the world's clusters and the tables built so far.
struct Resolver {
    clusters: Vec<usize>,
    lists: Vec<List>,
    model: usize,
    scene: usize,
    cluster_ix: std::collections::HashMap<usize, u32>,
    instance_ix: std::collections::HashMap<usize, u32>,
    material_ix: std::collections::HashMap<usize, u32>,
}

impl Resolver {
    /// The index of cluster `c` in `s.clusters`, added on first use.
    fn cluster(&mut self, s: &mut ffx_gfx::scene_inputs::SceneInputs, c: usize) -> u32 {
        if let Some(&i) = self.cluster_ix.get(&c) {
            return i;
        }
        let name = if c == self.model && self.scene != 0 {
            word(self.scene + 0xd8).and_then(cstr).unwrap_or_default()
        } else {
            String::new()
        };
        s.clusters.push(ffx_gfx::scene_inputs::Cluster {
            index: self
                .clusters
                .iter()
                .position(|&x| x == c)
                .map_or(u32::MAX, |p| p as u32),
            name,
            lists: self
                .lists
                .iter()
                .filter(|l| l.cluster == c)
                .map(|l| (l.class.clone(), l.count))
                .collect(),
        });
        let i = (s.clusters.len() - 1) as u32;
        self.cluster_ix.insert(c, i);
        i
    }

    /// The (cluster, slot) of an object of class `class` (the name's last
    /// part).
    fn object(&self, p: usize, class: &str) -> Option<(usize, u32)> {
        self.lists
            .iter()
            .filter(|l| l.class.rsplit("::").next() == Some(class))
            .find_map(|l| Some((l.cluster, l.slot_of(p)?)))
    }

    /// The index of material `mat` in `s.materials`, added on first use.
    fn material(&mut self, s: &mut ffx_gfx::scene_inputs::SceneInputs, mat: usize) -> u32 {
        if let Some(&i) = self.material_ix.get(&mat) {
            return i;
        }
        let buf = word(mat + 4).unwrap_or(0);
        let size = if buf == 0 {
            0
        } else {
            u32_at(buf).unwrap_or(0)
        };
        s.materials.push(ffx_gfx::scene_inputs::Material {
            remap_from: pass_name(word(mat + 8).unwrap_or(0)),
            remap_to: pass_name(word(mat + 0xc).unwrap_or(0)),
            parameters: bytes(buf, size.min(PARAMETERS_MAX) as usize).unwrap_or_default(),
        });
        let i = (s.materials.len() - 1) as u32;
        self.material_ix.insert(mat, i);
        i
    }

    /// The index of `PMeshInstance` `inst` in `s.instances`, read on first
    /// use: `+0` the mesh (count, `+4` its `0x6c`-byte segments), `+4` the
    /// world matrix, `+0x10` the material set (count; one inline at `+4`,
    /// several through the pointer at `+4`), as `renderMeshInstance`
    /// (`0x005b9a90`) reads them.
    fn instance(
        &mut self,
        s: &mut ffx_gfx::scene_inputs::SceneInputs,
        inst: usize,
        bounds_world: usize,
    ) -> u32 {
        use ffx_gfx::scene_inputs as si;
        if let Some(&i) = self.instance_ix.get(&inst) {
            return i;
        }
        let (cluster, slot) = match self.object(inst, "PMeshInstance") {
            Some((c, slot)) => (self.cluster(s, c), slot),
            None => (u32::MAX, u32::MAX),
        };
        let world_ptr = word(inst + 4).unwrap_or(0);
        let mut out = si::Instance {
            cluster,
            slot,
            world: (world_ptr != 0).then(|| floats::<12>(world_ptr)).flatten(),
            world_shared: world_ptr != 0 && world_ptr == bounds_world,
            ..si::Instance::default()
        };
        if let Some(f) = bytes(inst + 0x48, 0x14) {
            let w = |o: usize| u32::from_le_bytes([f[o], f[o + 1], f[o + 2], f[o + 3]]);
            out.group_id = w(0);
            out.dobj_kind = w(4);
            out.dobj_flag = w(0xc);
            out.obj_id = w(0x10);
        }
        let mesh = word(inst).unwrap_or(0);
        if mesh != 0 {
            let n = u32_at(mesh).unwrap_or(0) & 0x7fff_ffff;
            let segs = word(mesh + 4).unwrap_or(0);
            for k in 0..n.min(SEGMENTS_MAX) as usize {
                let Some(b) = bytes(segs + 0x6c * k, 0x20) else {
                    break;
                };
                let w = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
                out.segments.push(si::Segment {
                    material: w(0),
                    matrix: w(4),
                    hidden: b[0x14],
                    id: w(0x18),
                    render_order: w(0x1c),
                });
            }
        }
        let set = word(inst + 0x10).unwrap_or(0);
        if set != 0 {
            let n = u32_at(set).unwrap_or(0) & 0x7fff_ffff;
            let base = match n {
                0 => 0,
                1 => set + 4,
                _ => word(set + 4).unwrap_or(0),
            };
            for k in 0..n.min(SEGMENTS_MAX) as usize {
                let mat = if base == 0 {
                    0
                } else {
                    word(base + 4 * k).unwrap_or(0)
                };
                let m = (mat != 0).then(|| self.material(s, mat));
                out.materials.push(m);
            }
        }
        s.instances.push(out);
        let i = (s.instances.len() - 1) as u32;
        self.instance_ix.insert(inst, i);
        i
    }
}

/// The scene inputs of one `renderWorld(world, camera, dir)` call on
/// `renderer` (`docs/formats/scene.md`, "Scene inputs"), on any thread:
/// plain memory reads (never a fault). The caller fills in `frame`,
/// `update` and `thread`.
#[must_use]
pub fn scene_inputs_at(
    renderer: usize,
    world: usize,
    camera: usize,
    dir: i32,
) -> Option<(
    ffx_gfx::scene_inputs::SceneInputs,
    ffx_gfx::scene_inputs::Wire,
)> {
    use ffx_gfx::scene_inputs as si;
    let app = word(crate::rebase(G_PHYRE_APPLICATION)).unwrap_or(0);
    let scene = word(crate::rebase(G_PHYRE_SCENE)).unwrap_or(0);
    let screen = crate::rebase(SCREEN_SIZE);
    let mut s = si::SceneInputs {
        world: world_offset(world),
        pass: pass_name(word(renderer + 0x48).unwrap_or(0)),
        dir,
        camera: bytes(camera, si::CAMERA_LEN)?,
        app: bytes(app + si::APP_FROM, si::APP_LEN).unwrap_or_default(),
        screen: [u32_at(screen).unwrap_or(0), u32_at(screen + 4).unwrap_or(0)],
        ..si::SceneInputs::default()
    };
    let t0 = std::time::Instant::now();
    let clusters = world_clusters(world);
    let mut names = std::collections::HashMap::new();
    let lists = clusters
        .iter()
        .flat_map(|&c| cluster_lists(c))
        .filter_map(|l| List::at(l, &mut names))
        .collect();
    let mut r = Resolver {
        clusters,
        lists,
        model: if scene == 0 {
            0
        } else {
            word(scene + 0x10d4).unwrap_or(0)
        },
        scene,
        cluster_ix: std::collections::HashMap::new(),
        instance_ix: std::collections::HashMap::new(),
        material_ix: std::collections::HashMap::new(),
    };
    let t1 = std::time::Instant::now();
    // The bounds, in the renderer's order.
    let mut bounds_at = std::collections::HashMap::new();
    let walk = world_lists_of(world, crate::rebase(BOUNDS_CLASS));
    for (list_no, &l) in walk.iter().enumerate() {
        let Some(list) = List::at(l, &mut names) else {
            continue;
        };
        for (obj, slot) in list.objects() {
            let Some(raw) = bytes(obj, 0x24) else {
                continue;
            };
            let w = |o: usize| u32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]);
            let f = |o: usize| f32::from_bits(w(o));
            let world_ptr = w(0xc) as usize;
            let inst = w(0x1c) as usize;
            let cluster = r.cluster(&mut s, list.cluster);
            let instance = (inst != 0).then(|| r.instance(&mut s, inst, world_ptr));
            bounds_at.insert(obj, s.bounds.len() as u32);
            s.bounds.push(si::Bounds {
                cluster,
                slot,
                list: list_no as u32,
                min: [f(0), f(4), f(8)],
                size: [f(0x10), f(0x14), f(0x18)],
                world: (world_ptr != 0).then(|| floats::<12>(world_ptr)).flatten(),
                instance,
                render_order: w(0x20).cast_signed(),
            });
        }
    }
    let t2 = std::time::Instant::now();
    if scene != 0 {
        scene_switches(&mut s, scene, &mut r);
    }
    let n = u32_at(crate::rebase(DEPTH_BIAS_COUNT)).unwrap_or(0);
    s.depth_bias_count = n;
    for k in 0..n.min(256) as usize {
        let at = crate::rebase(DEPTH_BIAS) + 12 * k;
        let (Some(key), Some([scale, bias])) = (word(at), floats::<2>(at + 4)) else {
            break;
        };
        s.depth_bias.push(si::DepthBias {
            bounds: bounds_at.get(&key).copied(),
            scale,
            bias,
        });
    }
    let t3 = std::time::Instant::now();
    let wire = wire_at(camera, &r);
    let t4 = std::time::Instant::now();
    PHASES.with(|p| {
        p.set([
            (t1 - t0).as_micros(),
            (t2 - t1).as_micros(),
            (t3 - t2).as_micros(),
            (t4 - t3).as_micros(),
        ]);
    });
    Some((s, wire))
}

/// `Phyre_PRenderInterface` (`0x00c94f00`) `+0x1340`: the texture
/// quality (`Phyre_PRenderInterface_setTextureQuality` `0x0058e000`).
const TEXTURE_QUALITY: u32 = 0x00c9_6240;
/// The anisotropy setting `PSamplerStateD3D11` (`0x00595bf0`) reads.
const ANISOTROPY: u32 = 0x00c2_5370;
/// The asset reference tree's header (`0x0045d420`): nil node `+0`, root
/// `+0xc`; nodes are `PAssetReference +4` (left `+0`, right `+4`, parent
/// and colour `+8`), the reference's id `+0x18`, asset `+0x1c`
/// (`Phyre_PAssetReference_GetAssetReferenceByAsset` `0x0045cb50`,
/// `Phyre_PRedBlackTree_successor` `0x00575160`).
const ASSET_TREE: u32 = 0x00c9_424c;
/// Nodes walked at most.
const ASSETS_MAX: usize = 1 << 17;
/// Asset ids by object, for the model cluster they were found under.
type AssetCache = (usize, std::collections::HashMap<usize, String>);
static ASSET_CACHE: Mutex<Option<AssetCache>> = Mutex::new(None);

/// A `PString`'s text (the pointer's low bit is a flag).
fn pstring(p: usize) -> Option<String> {
    let p = p & !1;
    (p != 0).then(|| cstr(p)).flatten()
}

/// The asset ids of `wanted` objects, from the asset reference tree.
fn asset_ids(
    wanted: &std::collections::HashSet<usize>,
) -> std::collections::HashMap<usize, String> {
    let mut out = std::collections::HashMap::new();
    let nil = crate::rebase(ASSET_TREE);
    let mut stack = vec![word(nil + 0xc).unwrap_or(nil)];
    let mut seen = 0;
    while let Some(node) = stack.pop() {
        if node == nil || node == 0 || seen >= ASSETS_MAX || out.len() == wanted.len() {
            continue;
        }
        seen += 1;
        let Some(b) = bytes(node, 0x1c) else {
            continue;
        };
        let w = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as usize;
        stack.push(w(0));
        stack.push(w(4));
        if wanted.contains(&w(0x18))
            && let Some(id) = pstring(w(0x14))
        {
            out.insert(w(0x18), id);
        }
    }
    out
}

/// The [`ffx_gfx::scene_inputs::Wire`] block of a `renderWorld` call:
/// the camera's world matrix, the texture settings, the clusters' files and the materials' textures.
fn wire_at(camera: usize, r: &Resolver) -> ffx_gfx::scene_inputs::Wire {
    use ffx_gfx::scene_inputs as si;
    let world_ptr = word(camera + 0xb0).unwrap_or(0);
    let mut wire = si::Wire {
        camera_world: (world_ptr != 0).then(|| floats::<12>(world_ptr)).flatten(),
        texture_quality: u32_at(crate::rebase(TEXTURE_QUALITY)).unwrap_or(u32::MAX),
        anisotropy: u32_at(crate::rebase(ANISOTROPY)).unwrap_or(0),
        ..si::Wire::default()
    };
    // Clusters: their first asset reference's id, up to the `#`.
    let mut clusters: Vec<(usize, u32)> = r.cluster_ix.iter().map(|(&c, &i)| (c, i)).collect();
    clusters.sort_by_key(|c| c.1);
    for (c, _) in clusters {
        let id = r
            .lists
            .iter()
            .filter(|l| l.cluster == c && l.class.rsplit("::").next() == Some("PAssetReference"))
            .find_map(|l| {
                let (obj, _) = *l.objects().first()?;
                pstring(word(obj + 0x18)?)
            })
            .unwrap_or_default();
        wire.cluster_assets
            .push(id.split('#').next().unwrap_or_default().to_owned());
    }
    // Materials' texture slots, then their textures' ids.
    let mut materials: Vec<(usize, u32)> = r.material_ix.iter().map(|(&m, &i)| (m, i)).collect();
    materials.sort_by_key(|m| m.1);
    let mut slots = Vec::new();
    for &(m, _) in &materials {
        let variant = word(m).unwrap_or(0);
        let buf = word(m + 4).unwrap_or(0);
        let n = if variant == 0 {
            0
        } else {
            u32_at(variant + 0x20).unwrap_or(0) & 0x7fff_ffff
        };
        let defs = word(variant + 0x24).unwrap_or(0);
        let mut list = Vec::new();
        for k in 0..n.min(PARAMETERS_MAX) as usize {
            let Some(e) = bytes(defs + 16 * k, 16) else {
                break;
            };
            if !matches!(e[2], 0x42 | 0x44) {
                continue;
            }
            let offset = u32::from(u16::from_le_bytes([e[8], e[9]]));
            let texture = if buf == 0 {
                0
            } else {
                word(buf + offset as usize + 0xc).unwrap_or(0)
            };
            list.push((offset, e[2], texture));
        }
        slots.push(list);
    }
    let wanted: std::collections::HashSet<usize> = slots
        .iter()
        .flatten()
        .map(|t| t.2)
        .filter(|&t| t != 0)
        .collect();
    // Ids found before (by the model cluster: a new map frees the old
    // textures); the tree is walked only for pointers not seen yet.
    let ids = {
        let mut cache = ASSET_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cache = cache.get_or_insert_with(|| (r.model, std::collections::HashMap::new()));
        if cache.0 != r.model {
            *cache = (r.model, std::collections::HashMap::new());
        }
        let new: std::collections::HashSet<usize> = wanted
            .iter()
            .copied()
            .filter(|t| !cache.1.contains_key(t))
            .collect();
        if !new.is_empty() {
            let found = asset_ids(&new);
            for t in new {
                cache
                    .1
                    .insert(t, found.get(&t).cloned().unwrap_or_default());
            }
        }
        cache.1.clone()
    };
    for list in slots {
        wire.material_textures.push(
            list.into_iter()
                .map(|(offset, kind, texture)| si::MaterialTexture {
                    offset,
                    kind,
                    set: texture != 0,
                    asset: ids.get(&texture).cloned().unwrap_or_default(),
                })
                .collect(),
        );
    }
    wire
}

/// The pass a renderer draws (`+0x48`: `OpaqueNoMSAA`, `postOpaque`, ...).
#[must_use]
pub fn renderer_pass(renderer: usize) -> String {
    pass_name(word(renderer + 0x48).unwrap_or(0))
}

/// The map model's symbol (`PhyreScene +0xd8`, e.g. `cdsp00`), cheaply.
#[must_use]
pub fn map_symbol() -> Option<String> {
    let scene = word(crate::rebase(G_PHYRE_SCENE))?;
    (scene != 0).then(|| word(scene + 0xd8).and_then(cstr))?
}

/// The scene's switches that hide map parts (group mask, 2D layers) and
/// the shadow solution and alpha-fast flag, into `s`.
fn scene_switches(s: &mut ffx_gfx::scene_inputs::SceneInputs, scene: usize, r: &mut Resolver) {
    use ffx_gfx::scene_inputs as si;
    s.group_mask = u32_at(scene + 0xfb14).unwrap_or(0);
    s.shadow_solution = u32_at(scene + 0xf9cc).unwrap_or(0);
    s.alpha_fast = bytes(scene + 0xd4, 1).map_or(0, |b| b[0]);
    s.layer_group = u32_at(scene + 0x11d8).unwrap_or(0);
    let present = word(scene + 0x1128).unwrap_or(0) != 0;
    let count = u32_at(scene + 0x1138).unwrap_or(0).min(256) as usize;
    let table = word(scene + 0x1140).unwrap_or(0);
    if present && table != 0 && count > 0 {
        for e in bytes(table, 8 * count)
            .unwrap_or_default()
            .as_chunks::<8>()
            .0
        {
            let mesh = u32::from_le_bytes([e[4], e[5], e[6], e[7]]) as usize;
            let at = r.object(mesh, "PMeshInstance");
            let mesh_ix = at.map(|(c, slot)| (r.cluster(s, c), slot));
            s.layers.push(si::Layer {
                shown: e[0],
                group: e[1].cast_signed(),
                id: i16::from_le_bytes([e[2], e[3]]),
                has_mesh: mesh != 0,
                mesh: mesh_ix,
            });
        }
    }
}

/// What a world-matrix watch ([`watch`]) watches: up to four map
/// instances' world matrices (slot in the `PMeshInstanceBounds` list,
/// word, address): the first three whose matrix is not the common
/// 10 x unit one, at their first word that differs from it, and one
/// common instance at word 1.
fn watch_targets(world: usize) -> Vec<(u32, u32, usize)> {
    const COMMON: [f32; 12] = [
        0.0, 10.0, 0.0, 10.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let mut names = std::collections::HashMap::new();
    let mut moved = Vec::new();
    let mut common = None;
    for l in world_lists_of(world, crate::rebase(BOUNDS_CLASS)) {
        let Some(list) = List::at(l, &mut names) else {
            continue;
        };
        for (obj, slot) in list.objects() {
            let Some(m) = word(obj + 0xc).filter(|&m| m != 0) else {
                continue;
            };
            let Some(v) = floats::<12>(m) else {
                continue;
            };
            match (0..12).find(|&k| v[k].to_bits() != COMMON[k].to_bits()) {
                Some(k) => moved.push((slot, k as u32, m + 4 * k)),
                None => common = common.or(Some((slot, 1, m + 4))),
            }
        }
    }
    moved.truncate(3);
    moved.extend(common);
    moved
}

/// A write watch on map instances' world matrices (M4 round 2, Q4 in
/// `docs/formats/scene.md`): with `watch-worlds` in `FFX_GFX`, the first
/// map `renderWorld` of a capture puts the x86 debug registers of every
/// thread on four words of world matrices ([`watch_targets`]); each write
/// traps (a single-step exception after the writing instruction) and is
/// counted by the instruction's address. The capture's end takes the
/// counts into `frame.txt` and clears the registers.
pub mod watch {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::{Mutex, OnceLock};

    use windows_sys::Win32::System::Diagnostics::Debug::{
        AddVectoredExceptionHandler, CONTEXT, EXCEPTION_POINTERS, GetThreadContext,
        SetThreadContext,
    };

    const CONTEXT_DEBUG_REGISTERS: u32 = 0x0001_0010;
    const HITS_MAX: usize = 4096;

    static ARMED: AtomicBool = AtomicBool::new(false);
    static HANDLER: OnceLock<()> = OnceLock::new();
    /// What is watched: (slot, word, address) per debug register.
    static TARGETS: Mutex<Vec<(u32, u32, usize)>> = Mutex::new(Vec::new());
    /// Hits: the instruction after the write, the debug register, the
    /// update and the thread, four words each.
    static HITS: [AtomicU32; HITS_MAX * 4] = [const { AtomicU32::new(0) }; HITS_MAX * 4];
    static COUNT: AtomicU32 = AtomicU32::new(0);

    /// Whether `FFX_GFX` asks for the watch.
    pub fn wanted() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("FFX_GFX").is_ok_and(|v| v.contains("watch-worlds")))
    }

    pub fn armed() -> bool {
        ARMED.load(Ordering::Acquire)
    }

    unsafe extern "system" fn on_step(info: *mut EXCEPTION_POINTERS) -> i32 {
        // SAFETY: the system passes a valid record and context.
        unsafe {
            if (*(*info).ExceptionRecord).ExceptionCode.cast_unsigned() != 0x8000_0004 {
                return 0;
            }
            let ctx = &mut *(*info).ContextRecord;
            let hit = ctx.Dr6 & 0xf;
            if hit == 0 {
                return 0;
            }
            ctx.Dr6 = 0;
            let n = COUNT.fetch_add(1, Ordering::AcqRel) as usize;
            if n < HITS_MAX {
                HITS[4 * n].store(ctx.Eip, Ordering::Relaxed);
                HITS[4 * n + 1].store(hit.trailing_zeros(), Ordering::Relaxed);
                HITS[4 * n + 2].store(crate::harness::update_label(), Ordering::Relaxed);
                HITS[4 * n + 3].store(
                    windows_sys::Win32::System::Threading::GetCurrentThreadId(),
                    Ordering::Release,
                );
            }
        }
        -1 // EXCEPTION_CONTINUE_EXECUTION
    }

    /// Sets (or with `None`, clears) the debug registers of every other
    /// thread of the process, each suspended meanwhile.
    fn set_all(addrs: Option<&[usize]>) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        };
        use windows_sys::Win32::System::Threading::{
            GetCurrentProcessId, GetCurrentThreadId, OpenThread, ResumeThread, SuspendThread,
            THREAD_GET_CONTEXT, THREAD_SET_CONTEXT, THREAD_SUSPEND_RESUME,
        };
        let mut dr7 = 0u32;
        let mut regs = [0u32; 4];
        for (k, &a) in addrs.unwrap_or(&[]).iter().take(4).enumerate() {
            regs[k] = a as u32;
            // Local enable, break on writes (01), 4 bytes (11).
            dr7 |= 1 << (2 * k) | 0b1101 << (16 + 4 * k);
        }
        // SAFETY: plain Win32 calls on this process's threads; each is
        // suspended while its context is changed.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            let (pid, me) = (GetCurrentProcessId(), GetCurrentThreadId());
            let mut te: THREADENTRY32 = std::mem::zeroed();
            te.dwSize = u32::try_from(size_of::<THREADENTRY32>()).unwrap_or(0);
            let mut ok = Thread32First(snap, &raw mut te) != 0;
            while ok {
                if te.th32OwnerProcessID == pid && te.th32ThreadID != me {
                    let h = OpenThread(
                        THREAD_GET_CONTEXT | THREAD_SET_CONTEXT | THREAD_SUSPEND_RESUME,
                        0,
                        te.th32ThreadID,
                    );
                    if !h.is_null() {
                        if SuspendThread(h) != u32::MAX {
                            let mut ctx: CONTEXT = std::mem::zeroed();
                            ctx.ContextFlags = CONTEXT_DEBUG_REGISTERS;
                            if GetThreadContext(h, &raw mut ctx) != 0 {
                                ctx.Dr0 = regs[0];
                                ctx.Dr1 = regs[1];
                                ctx.Dr2 = regs[2];
                                ctx.Dr3 = regs[3];
                                ctx.Dr6 = 0;
                                ctx.Dr7 = dr7;
                                SetThreadContext(h, &raw const ctx);
                            }
                            ResumeThread(h);
                        }
                        CloseHandle(h);
                    }
                }
                ok = Thread32Next(snap, &raw mut te) != 0;
            }
            CloseHandle(snap);
        }
    }

    /// Watches `targets` (at most four) from every thread. The registers
    /// are set from a helper thread, so the caller's own thread is
    /// watched too.
    pub fn arm(targets: &[(u32, u32, usize)]) {
        if targets.is_empty() || ARMED.swap(true, Ordering::AcqRel) {
            return;
        }
        HANDLER.get_or_init(|| {
            // SAFETY: registers a handler that only reads and writes the
            // exception's own context and static atomics.
            unsafe {
                AddVectoredExceptionHandler(1, Some(on_step));
            }
        });
        COUNT.store(0, Ordering::Release);
        if let Ok(mut t) = TARGETS.lock() {
            *t = targets.iter().take(4).copied().collect();
        }
        let addrs: Vec<usize> = targets.iter().take(4).map(|t| t.2).collect();
        let _ = std::thread::spawn(move || set_all(Some(&addrs))).join();
        crate::log::log(&format!("  world watch: armed {targets:x?}"));
    }

    /// Clears the registers and gives the hits as lines: per watched word
    /// and writing instruction (`FFX.exe` addresses: the instruction after
    /// the write), the updates and threads seen.
    pub fn report() -> Vec<String> {
        // (register, instruction) -> (count, first update, last update,
        // threads).
        type Seen = (u32, u32, u32, Vec<u32>);
        if !ARMED.swap(false, Ordering::AcqRel) {
            return Vec::new();
        }
        let _ = std::thread::spawn(|| set_all(None)).join();
        let targets = TARGETS.lock().map(|t| t.clone()).unwrap_or_default();
        let base = crate::GAME_BASE.load(Ordering::Acquire) as u32;
        let image = |eip: u32| {
            eip.checked_sub(base)
                .filter(|&o| o < crate::SIZE_OF_IMAGE)
                .map_or(eip, |o| o + crate::IMAGE_BASE)
        };
        let n = (COUNT.load(Ordering::Acquire) as usize).min(HITS_MAX);
        let mut by: std::collections::BTreeMap<(u32, u32), Seen> =
            std::collections::BTreeMap::new();
        for k in 0..n {
            let eip = image(HITS[4 * k].load(Ordering::Relaxed));
            let reg = HITS[4 * k + 1].load(Ordering::Relaxed);
            let update = HITS[4 * k + 2].load(Ordering::Relaxed);
            let thread = HITS[4 * k + 3].load(Ordering::Acquire);
            let e = by
                .entry((reg, eip))
                .or_insert((0, update, update, Vec::new()));
            e.0 += 1;
            e.1 = e.1.min(update);
            e.2 = e.2.max(update);
            if !e.3.contains(&thread) {
                e.3.push(thread);
            }
        }
        let mut out: Vec<String> = targets
            .iter()
            .enumerate()
            .map(|(r, (slot, word, _))| format!("watch dr{r} bounds slot {slot} word {word}"))
            .collect();
        for ((reg, eip), (count, from, to, threads)) in by {
            out.push(format!(
                "watch dr{reg} write before {eip:#010x}: {count} times, updates {from}..{to}, threads {threads:?}"
            ));
        }
        out
    }
}
