//! Frames of Direct3D calls to disk: every draw with what it had bound,
//! the constant buffer contents it saw, the shaders' bytecode, the
//! textures and buffers it read, and each render target as it was when
//! the game moved on to the next one.
//!
//! Two modes (`docs/renderer.md`, "Reference captures"): full (the
//! contents as files, for study) and hash-only (content hashes instead,
//! a few MB, for comparing runs). Either can run over several frames and
//! tags each frame (`Present` count, ctrl work 0's frame count, the
//! threads that made the calls) and samples the scene inputs
//! (`crate::world::scene_inputs`). The map's `renderWorld` calls add the
//! scene builder's inputs ([`add_inputs`], `crate::world::render_world`):
//! `SceneInputs` messages before each frame of `capture.ffxgfx`
//! (`docs/formats/scene.md`, "Scene inputs").
//!
//! All GPU read-backs here call the original methods directly: the layer's
//! own hooks take the state lock, which the caller already holds. Nothing
//! here runs while no capture is armed: every entry point returns at once
//! when `State::capture` is `None`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::c_void;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use ffx_gfx::reference::{DrawKeys, FrameTag, Ref, SceneSample, ThreadInfo, When};

use super::{Draw, State, ViewKind, device_original, real_method, state, vt};

type Ptr = *mut c_void;

/// How long a blocking `gfxcapture` waits for its frames.
const REPLY_WAIT: Duration = Duration::from_secs(600);
/// Render target read-backs per frame, at most.
const RT_DUMPS: u32 = 200;
const BIND_RENDER_TARGET: u32 = 0x20;
const BIND_DEPTH_STENCIL: u32 = 0x40;

/// What a `gfxcapture` asked for.
#[derive(Debug, Clone)]
struct Options {
    dir: PathBuf,
    /// Content hashes instead of contents.
    hash: bool,
    frames: u32,
    /// Render target 0 after each draw in this range (full mode).
    range: Option<(u32, u32)>,
    /// Start at once (the render thread is held in `Present`), not at the
    /// next `Present`.
    now: bool,
    /// Reply at once; `gfxcapture result` waits for the capture.
    detach: bool,
}

impl Options {
    /// `<dir> [hash] [frames N] [draws A B] [now] [async]`, or the old
    /// `<dir> [A B]`.
    fn parse(words: &[&str]) -> Result<Self, String> {
        let mut o = Self {
            dir: PathBuf::new(),
            hash: false,
            frames: 1,
            range: None,
            now: false,
            detach: false,
        };
        let num = |w: Option<&&str>| -> Result<u32, String> {
            w.and_then(|w| w.parse().ok())
                .ok_or_else(|| "gfxcapture: a number is missing".to_string())
        };
        let mut dir = Vec::new();
        let mut i = 0;
        let mut keywords = false;
        while i < words.len() {
            match words[i] {
                "hash" => o.hash = true,
                "now" => o.now = true,
                "async" => o.detach = true,
                "frames" => {
                    o.frames = num(words.get(i + 1))?.max(1);
                    i += 1;
                }
                "draws" => {
                    o.range = Some((num(words.get(i + 1))?, num(words.get(i + 2))?));
                    i += 2;
                }
                w => {
                    dir.push(w);
                    i += 1;
                    continue;
                }
            }
            keywords = true;
            i += 1;
        }
        // The old form: the last two words a draw range.
        if !keywords
            && let [.., a, b] = dir.as_slice()
            && let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>())
        {
            o.range = Some((a, b));
            dir.truncate(dir.len() - 2);
        }
        if dir.is_empty() {
            return Err("gfxcapture: no folder".into());
        }
        o.dir = PathBuf::from(dir.join(" "));
        Ok(o)
    }
}

pub struct Capture {
    opts: Options,
    lines: Vec<String>,
    /// Draws in the current frame.
    draws: u32,
    written: HashSet<(usize, u32)>,
    /// Views and buffers the capture used.
    views: BTreeSet<usize>,
    buffers: BTreeSet<usize>,
    rt_dumps: u32,
    draws_since_rt: u32,
    reply: Option<Sender<String>>,
    /// The current frame as scene-stream commands, the frames before it,
    /// and what they refer to.
    cmds: Vec<ffx_gfx::Cmd>,
    frames: Vec<ffx_gfx::Frame>,
    objects: BTreeSet<usize>,
    sent_versions: HashMap<usize, u32>,
    /// The frame being captured, from 0.
    frame: u32,
    /// Direct3D calls per thread in the current frame.
    calls: BTreeMap<u32, u32>,
    /// Reference records, and their address-free text (`hashes.txt`).
    refs: Vec<Ref>,
    text: Vec<String>,
    /// Render target roles by resource, and how many of each base name.
    roles: HashMap<usize, String>,
    role_counts: HashMap<String, u32>,
    /// Content hashes by (resource, version), and static index data.
    hashed: HashMap<(usize, u32), u64>,
    indices: HashMap<(usize, u32), Vec<u8>>,
    present_thread: u32,
    /// Textures the GPU wrote by copies: keyed by role, as targets.
    gpu_written: HashSet<usize>,
    /// The swap chain's back buffer (not made through the device: the
    /// layer's table can hold a stale texture at its address).
    backbuffer: usize,
    /// `State::reborn`'s length when the capture started.
    reborn_at: usize,
    /// The scene builder's inputs at each map `renderWorld` call
    /// (`crate::world::render_world`), each with its `Wire` block.
    inputs: Vec<(
        ffx_gfx::scene_inputs::SceneInputs,
        Option<ffx_gfx::scene_inputs::Wire>,
    )>,
}

static PENDING: Mutex<Option<Capture>> = Mutex::new(None);
static DRAW_RANGE: Mutex<Option<(u32, u32)>> = Mutex::new(None);
/// The reply of a detached (`async`) capture.
static RESULT: Mutex<Option<Receiver<String>>> = Mutex::new(None);
/// The swap chain of the last `Present`.
static SWAP_CHAIN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// Whether a capture is running: the `renderWorld` hook then reads the
/// scene inputs ([`add_inputs`]).
static INPUTS_WANTED: AtomicBool = AtomicBool::new(false);

/// Whether a capture wants scene inputs now (one atomic load: the
/// `renderWorld` hook asks on every call).
pub fn inputs_wanted() -> bool {
    INPUTS_WANTED.load(Ordering::Acquire)
}

/// Adds the scene inputs of a `renderWorld` call to the running capture,
/// tagged with the update that made it (the main thread renders update
/// `n` right after it; the render thread draws it later, see
/// [`end_frame`], which gives the record its capture frame).
pub fn add_inputs(
    mut inputs: ffx_gfx::scene_inputs::SceneInputs,
    wire: Option<ffx_gfx::scene_inputs::Wire>,
) {
    inputs.update = crate::harness::update_label();
    inputs.thread = thread_id();
    inputs.frame = u32::MAX;
    let mut s = state();
    if let Some(c) = s.capture.as_mut() {
        c.lines.push(format!(
            "scene_inputs update {} thread {} world {:#x} pass {} dir {} bounds {} instances {} materials {}",
            inputs.update,
            inputs.thread,
            inputs.world,
            inputs.pass,
            inputs.dir,
            inputs.bounds.len(),
            inputs.instances.len(),
            inputs.materials.len()
        ));
        c.inputs.push((inputs, wire));
    } else {
        // The capture ended some other way.
        INPUTS_WANTED.store(false, Ordering::Release);
    }
}

/// The swap chain's buffer 0 (the swap chain keeps it alive).
///
/// # Safety
/// `swap_chain` is the game's live swap chain (DXGI calls are
/// thread-safe).
unsafe fn back_buffer(swap_chain: Ptr) -> usize {
    const IID_TEXTURE2D: [u8; 16] = [
        0xf2, 0xaa, 0x15, 0x6f, 0x08, 0xd2, 0x89, 0x4e, 0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3, 0x4f,
        0x9c,
    ];
    if swap_chain.is_null() {
        return 0;
    }
    let mut back: Ptr = std::ptr::null_mut();
    // SAFETY: IDXGISwapChain::GetBuffer and IUnknown::Release.
    unsafe {
        let hr = super::com!(
            swap_chain,
            vt::GET_BUFFER,
            unsafe extern "system" fn(Ptr, u32, *const u8, *mut Ptr) -> i32,
            0,
            IID_TEXTURE2D.as_ptr(),
            &raw mut back
        );
        if hr < 0 || back.is_null() {
            return 0;
        }
        super::com!(back, vt::RELEASE, unsafe extern "system" fn(Ptr) -> u32);
    }
    back as usize
}

fn thread_id() -> u32 {
    // SAFETY: plain call.
    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }
}

fn present_count() -> u32 {
    super::FRAME.load(Ordering::Acquire)
}

/// The scene inputs now, as a sample. Held samples are read until two in
/// a row agree (20 ms apart, at most 25 times): the main thread may still
/// be finishing its update when the harness asks.
fn sample(frame: u32, when: When) -> SceneSample {
    let mut fields = crate::world::scene_inputs();
    if when == When::Held {
        for _ in 0..25 {
            std::thread::sleep(Duration::from_millis(20));
            let again = crate::world::scene_inputs();
            if again == fields {
                break;
            }
            fields = again;
        }
    }
    SceneSample {
        frame,
        when,
        thread: thread_id(),
        present: present_count(),
        fields,
        opened: crate::world::take_opened(),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

impl Capture {
    fn new(opts: Options, reply: Sender<String>) -> Self {
        Self {
            opts,
            lines: Vec::new(),
            draws: 0,
            written: HashSet::new(),
            views: BTreeSet::new(),
            buffers: BTreeSet::new(),
            rt_dumps: 0,
            draws_since_rt: 0,
            reply: Some(reply),
            cmds: Vec::new(),
            frames: Vec::new(),
            objects: BTreeSet::new(),
            sent_versions: HashMap::new(),
            frame: 0,
            calls: BTreeMap::new(),
            refs: Vec::new(),
            text: vec!["frame 0".into()],
            roles: HashMap::new(),
            role_counts: HashMap::new(),
            hashed: HashMap::new(),
            indices: HashMap::new(),
            present_thread: 0,
            gpu_written: HashSet::new(),
            backbuffer: 0,
            reborn_at: 0,
            inputs: Vec::new(),
        }
    }

    fn seen(&mut self) {
        *self.calls.entry(thread_id()).or_insert(0) += 1;
    }

    /// Adds a sample to the records; held samples also go into the text,
    /// which runs compare (present samples race the main thread).
    fn add_sample(&mut self, s: SceneSample) {
        if s.when == When::Held {
            for (name, bytes) in &s.fields {
                self.text.push(format!("held {name} {}", hex(bytes)));
            }
            let mut files: Vec<&str> = s.opened.iter().map(|(_, p)| p.as_str()).collect();
            files.sort_unstable();
            files.dedup();
            for f in files {
                self.text.push(format!("opened {f}"));
            }
        }
        self.refs.push(Ref::Scene(s));
    }

    /// Where a file of this capture goes; frames after the first get a
    /// suffix in full mode.
    fn frame_name(&self, base: &str) -> String {
        if self.frame == 0 {
            base.to_owned()
        } else {
            format!("{base}_f{}", self.frame)
        }
    }
}

/// Bytes per row of texels (or of 4x4 blocks) of a DXGI format, for the
/// formats the game uses; `None` for others (the whole row pitch is hashed
/// then).
fn row_bytes(format: u32, width: u32) -> Option<u32> {
    let blocks = width.div_ceil(4);
    Some(match format {
        70..=72 | 79..=81 => blocks * 8,
        73..=78 | 82..=84 | 94..=99 => blocks * 16,
        1..=4 => width * 16,
        5..=8 => width * 12,
        9..=22 => width * 8,
        23..=47 | 85..=93 => width * 4,
        48..=59 => width * 2,
        60..=65 => width,
        _ => return None,
    })
}

/// FNV-1a of a read-back's texel rows, without the row padding.
fn hash_rows(data: &[u8], pitch: u32, rows: u32, row: Option<u32>) -> u64 {
    let used = row.unwrap_or(pitch).min(pitch) as usize;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for r in 0..rows as usize {
        let at = r * pitch as usize;
        for &b in data.get(at..at + used).unwrap_or(&[]) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

/// Captures frames into `dir` (a Windows path); returns the harness reply
/// once they are written, or at once with `async` (then `gfxcapture
/// result` waits for it).
pub fn request(args: &str) -> String {
    let words: Vec<&str> = args.split_whitespace().collect();
    if words.first() == Some(&"result") {
        let Some(rx) = RESULT.lock().ok().and_then(|mut r| r.take()) else {
            return "err no capture is running".into();
        };
        return rx
            .recv_timeout(REPLY_WAIT)
            .unwrap_or_else(|e| format!("err {e}"));
    }
    if !super::ENABLED.load(Ordering::Acquire) {
        return "err the graphics layer is off (FFX_GFX=1)".into();
    }
    let opts = match Options::parse(&words) {
        Ok(o) => o,
        Err(e) => return format!("err {e}"),
    };
    if let Ok(mut r) = DRAW_RANGE.lock() {
        *r = opts.range;
    }
    let subs: &[&str] = if opts.hash {
        &[""]
    } else {
        &["", "shaders", "tex", "buf", "rt", "cb"]
    };
    for sub in subs {
        if let Err(e) = std::fs::create_dir_all(opts.dir.join(sub)) {
            return format!("err {e}");
        }
    }
    let (tx, rx) = channel();
    let (now, detach) = (opts.now, opts.detach);
    let mut c = Capture::new(opts, tx);
    // Files opened before the capture do not belong to it.
    let _ = crate::world::take_opened();
    if now {
        // The render thread is held in Present (the caller says so): the
        // next calls are the next frame's.
        c.add_sample(sample(0, When::Held));
        // SAFETY: the swap chain of the last Present.
        c.backbuffer = unsafe { back_buffer(SWAP_CHAIN.load(Ordering::Acquire) as Ptr) };
        let mut s = state();
        c.reborn_at = s.reborn.len();
        s.capture = Some(c);
        INPUTS_WANTED.store(true, Ordering::Release);
    } else if let Ok(mut p) = PENDING.lock() {
        *p = Some(c);
    }
    if detach {
        if let Ok(mut r) = RESULT.lock() {
            *r = Some(rx);
        }
        return "ok armed".into();
    }
    rx.recv_timeout(REPLY_WAIT)
        .unwrap_or_else(|e| format!("err {e}"))
}

/// Adds a line to the capture, if one is running.
pub fn note(s: &mut State, line: &str) {
    let Some(mut c) = s.capture.take() else {
        return;
    };
    c.seen();
    c.lines.push(line.to_owned());
    // Geometry and compute shaders, dispatches and region copies go into
    // the text too.
    let key = line
        .strip_prefix("gs_set_shader ")
        .map(|p| ("gs", p))
        .or_else(|| line.strip_prefix("cs_set_shader ").map(|p| ("cs", p)));
    let hex_id = |t: &str| usize::from_str_radix(t.trim_start_matches("0x"), 16).ok();
    if let Some((stage, p)) = key {
        let h = hex_id(p)
            .and_then(|p| s.shaders.get(&p))
            .map_or(0, |sh| sh.hash);
        c.text.push(format!("{stage} {h:016x}"));
    } else if line.starts_with("dispatch ") {
        c.text.push(line.to_owned());
    } else if let Some(rest) = line
        .strip_prefix("copy_region ")
        .or_else(|| line.strip_prefix("copy_region1 "))
        && let Some(dst) = hex_id(rest.split('[').next().unwrap_or(""))
    {
        c.gpu_written.insert(dst);
        let r = role(s, &mut c, dst);
        c.text.push(format!("xr {r}"));
    }
    s.capture = Some(c);
}

fn list(slots: &[usize]) -> String {
    let mut out = String::new();
    for (i, p) in slots.iter().enumerate() {
        if *p != 0 {
            let _ = write!(out, "{i}:{p:#x},");
        }
    }
    out
}

/// A render target's (or any texture's) role: its description and the
/// order it was first seen in among those with the same description, so
/// two runs name the same target alike whatever its address.
fn role(s: &State, c: &mut Capture, view_or_resource: usize) -> String {
    if view_or_resource == 0 {
        return String::new();
    }
    let resource = s
        .views
        .get(&view_or_resource)
        .map_or(view_or_resource, |v| v.resource);
    if let Some(r) = c.roles.get(&resource) {
        return r.clone();
    }
    if resource == c.backbuffer {
        return "backbuffer".into();
    }
    let base = s.textures.get(&resource).map_or_else(
        || "untracked".to_string(),
        |t| {
            let d = &t.desc;
            format!("{}x{}f{}b{:x}", d[0], d[1], d[4], d[8])
        },
    );
    let n = c.role_counts.entry(base.clone()).or_insert(0);
    let name = format!("{base}#{n}");
    *n += 1;
    c.roles.insert(resource, name.clone());
    name
}

/// Records a texture copy.
pub fn copy(s: &mut State, dst: usize, src: usize) {
    if s.capture.is_none() || !(s.textures.contains_key(&dst) && s.textures.contains_key(&src)) {
        return;
    }
    let Some(mut c) = s.capture.take() else {
        return;
    };
    c.seen();
    c.objects.insert(dst);
    c.objects.insert(src);
    c.cmds.push(ffx_gfx::Cmd::CopyTexture {
        dst: dst as u32,
        src: src as u32,
    });
    c.gpu_written.insert(dst);
    let (d, from) = (role(s, &mut c, dst), role(s, &mut c, src));
    c.text.push(format!("x {d} {from}"));
    s.capture = Some(c);
}

/// Records a render target clear.
pub fn clear_target(s: &mut State, view: usize, color: [f32; 4]) {
    let Some(mut c) = s.capture.take() else {
        return;
    };
    c.seen();
    c.objects.insert(view);
    c.cmds.push(ffx_gfx::Cmd::ClearTarget {
        view: view as u32,
        color,
    });
    let bits: Vec<String> = color
        .iter()
        .map(|v| format!("{:08x}", v.to_bits()))
        .collect();
    let r = role(s, &mut c, view);
    c.text.push(format!("c {r} {}", bits.join(",")));
    s.capture = Some(c);
}

/// Records a depth-stencil clear.
pub fn clear_depth(s: &mut State, view: usize, flags: u32, depth: f32, stencil: u8) {
    let Some(mut c) = s.capture.take() else {
        return;
    };
    c.seen();
    c.objects.insert(view);
    c.cmds.push(ffx_gfx::Cmd::ClearDepth {
        view: view as u32,
        flags,
        depth,
        stencil,
    });
    let r = role(s, &mut c, view);
    c.text
        .push(format!("z {r} {flags} {:08x} {stencil}", depth.to_bits()));
    s.capture = Some(c);
}

/// After a draw: saves render target 0 when the draw is in the asked range.
#[allow(clippy::many_single_char_names)] // the draw range a..b, the capture, its count
pub fn after_draw(s: &mut State) {
    let Some(c) = &s.capture else { return };
    if c.opts.hash {
        return;
    }
    let n = c.draws;
    let Some((a, b)) = DRAW_RANGE.lock().ok().and_then(|r| *r) else {
        return;
    };
    if n < a || n > b {
        return;
    }
    let Some(res) = s.views.get(&s.bound.rtv[0]).map(|v| v.resource) else {
        return;
    };
    // SAFETY: on the render thread, inside a context call.
    if let Ok(t) = unsafe { read_texture(res) }
        && let Some(c) = &mut s.capture
    {
        let name = c.frame_name(&format!("rt/draw_{n:04}"));
        let _ = std::fs::write(c.opts.dir.join(format!("{name}.raw")), &t.data);
        c.lines.push(format!(
            "dump {name} {}x{} format {} pitch {} rows {}",
            t.width, t.height, t.format, t.pitch, t.rows
        ));
    }
}

/// Content hash of a texture as it is now (all subresources), cached by
/// version.
#[allow(clippy::many_single_char_names)] // state, capture, texture, hash, width
fn texture_hash(s: &State, c: &mut Capture, resource: usize) -> u64 {
    let Some(t) = s.textures.get(&resource) else {
        return 0;
    };
    if let Some(&h) = c.hashed.get(&(resource, t.version)) {
        return h;
    }
    // SAFETY: on the render thread, inside a context call.
    let h = match unsafe { read_texture_all(resource, &t.desc) } {
        Ok(subs) => {
            let (mips, mut h) = (t.desc[2].max(1), 0xcbf2_9ce4_8422_2325_u64);
            for (i, sub) in subs.iter().enumerate() {
                let mip = i as u32 % mips;
                let (w, ht) = ((t.desc[0] >> mip).max(1), (t.desc[1] >> mip).max(1));
                let rows = if block_compressed(t.desc[4]) {
                    ht.div_ceil(4)
                } else {
                    ht
                };
                let part = hash_rows(&sub.data, sub.row_pitch, rows, row_bytes(t.desc[4], w));
                h = (h ^ part).wrapping_mul(0x0100_0000_01b3);
            }
            h
        }
        Err(_) => 0,
    };
    c.hashed.insert((resource, t.version), h);
    h
}

/// A buffer's bytes: the layer's copy for dynamic ones, a read-back (kept
/// per version) for static ones.
fn buffer_bytes(s: &State, c: &mut Capture, id: usize) -> Option<Vec<u8>> {
    let b = s.buffers.get(&id)?;
    if b.desc[1] == 2
        && let Some(sh) = &b.shadow
    {
        return Some(sh.clone());
    }
    // Static buffers, and dynamic ones not written since the capture
    // started (no copy of the layer's): a read-back.
    if let Some(d) = c.indices.get(&(id, b.version)) {
        return Some(d.clone());
    }
    // SAFETY: on the render thread, inside a context call.
    let d = unsafe { read_buffer(id, b.desc[0]) }.ok()?;
    c.indices.insert((id, b.version), d.clone());
    Some(d)
}

/// Hash of `bytes[from..to]` (clamped), with its length.
fn range_hash(bytes: &[u8], from: usize, to: usize) -> u64 {
    let to = to.min(bytes.len());
    super::hash(bytes.get(from.min(to)..to).unwrap_or(&[]))
}

/// The content keys of a draw, their text line, and the ring bytes it
/// reads as updates for the scene stream (only those bytes: hash-only
/// captures carry no buffer contents otherwise).
#[allow(clippy::too_many_lines)] // one draw's keys, field by field
fn keys(s: &State, c: &mut Capture, d: &ffx_gfx::Draw) -> (DrawKeys, String, Vec<ffx_gfx::Cmd>) {
    let ring = |id: u32| {
        s.buffers
            .get(&(id as usize))
            .is_some_and(|b| b.desc[1] == 2)
    };
    let mut updates = Vec::new();
    let mut update = |id: u32, data: &[u8], from: usize, to: usize| {
        let to = to.min(data.len());
        if from < to {
            updates.push(ffx_gfx::Cmd::UpdateBuffer {
                id,
                offset: from as u32,
                data: data[from..to].to_vec(),
            });
        }
    };
    use ffx_gfx::DrawKind as K;
    let indexed = matches!(d.kind, K::Indexed | K::IndexedInstanced);
    let (ib, ifmt, ioff) = d.index;
    let isize = if ifmt == 42 { 4 } else { 2 };
    // The indices the draw reads, and the vertices they reach.
    let index_data = if indexed {
        buffer_bytes(s, c, ib as usize)
    } else {
        None
    };
    let index = index_data.as_ref().map_or(0, |data| {
        let from = ioff as usize + d.start as usize * isize;
        let to = from + d.count as usize * isize;
        if ring(ib) {
            update(ib, data, from, to);
        }
        range_hash(data, from, to)
    });
    let vertices = if indexed {
        index_data
            .as_ref()
            .and_then(|data| index_range(data, ifmt, ioff, d.start, d.count))
            .map(|(lo, hi)| {
                (
                    i64::from(lo) + i64::from(d.base),
                    i64::from(hi) + i64::from(d.base),
                )
            })
    } else {
        Some((
            i64::from(d.start),
            i64::from(d.start) + i64::from(d.count) - 1,
        ))
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
    let mut vertex = Vec::new();
    let mut vb_text = String::new();
    for &(slot, id, stride, offset) in &d.vertex {
        let dynamic = s
            .buffers
            .get(&(id as usize))
            .is_some_and(|b| b.desc[1] == 2);
        let data = buffer_bytes(s, c, id as usize);
        let h = data.as_ref().map_or(0, |data| {
            if !dynamic {
                return super::hash(data);
            }
            let (lo, hi) = if per_instance.contains(&u32::from(slot)) {
                (
                    i64::from(d.first_instance),
                    i64::from(d.first_instance) + i64::from(d.instances) - 1,
                )
            } else {
                vertices.unwrap_or((0, -1))
            };
            let (Ok(lo), Ok(hi)) = (usize::try_from(lo), usize::try_from(hi)) else {
                return 0;
            };
            if hi < lo {
                return 0;
            }
            let from = offset as usize + lo * stride as usize;
            let to = offset as usize + (hi + 1) * stride as usize;
            update(id, data, from, to);
            range_hash(data, from, to)
        });
        vertex.push((slot, h));
        let _ = write!(vb_text, "{slot}:{stride}:{h:016x},");
    }
    let mut constants = Vec::new();
    let mut cb_text = String::new();
    for (stage, list) in [(b'v', &d.vs_cb), (b'p', &d.ps_cb)] {
        for (slot, data) in list {
            let h = super::hash(data);
            constants.push((stage, *slot, h));
            let _ = write!(cb_text, "{}{slot}:{h:016x},", stage as char);
        }
    }
    let mut textures = Vec::new();
    let mut srv_text = String::new();
    for (stage, list) in [(b'v', &d.vs_srv), (b'p', &d.ps_srv)] {
        for &(slot, view) in list {
            let resource = s.views.get(&(view as usize)).map_or(0, |v| v.resource);
            let target = s
                .textures
                .get(&resource)
                .is_some_and(|t| t.desc[8] & (BIND_RENDER_TARGET | BIND_DEPTH_STENCIL) != 0)
                || c.gpu_written.contains(&resource)
                || resource == c.backbuffer;
            let (h, shown) = if target {
                let r = role(s, c, resource);
                (super::hash(r.as_bytes()), r)
            } else {
                let h = texture_hash(s, c, resource);
                (h, format!("{h:016x}"))
            };
            textures.push((stage, slot, h));
            let _ = write!(srv_text, "{}{slot}:{shown},", stage as char);
        }
    }
    let state_hash = |id: u32| {
        s.states
            .get(&(id as usize))
            .map_or(0, |(_, desc)| super::hash(desc))
    };
    let mut samp_text = String::new();
    for (stage, list) in [('v', &d.vs_samplers), ('p', &d.ps_samplers)] {
        for &(slot, id) in list {
            let _ = write!(samp_text, "{stage}{slot}:{:016x},", state_hash(id));
        }
    }
    let shader = |id: u32| s.shaders.get(&(id as usize)).map_or(0, |sh| sh.hash);
    let layout = s.layouts.get(&(d.layout as usize)).map_or(0, |l| {
        let mut t = format!("{:016x}", l.signature);
        for e in &l.elements {
            let _ = write!(
                t,
                " {}{}:{}:{}:{}:{}:{}",
                e.semantic, e.index, e.format, e.slot, e.offset, e.per_instance, e.step
            );
        }
        super::hash(t.as_bytes())
    });
    let targets: Vec<(u8, String)> = d
        .rtv
        .iter()
        .map(|&(slot, view)| (slot, role(s, c, view as usize)))
        .collect();
    let depth = role(s, c, d.dsv as usize);
    let rt_text: Vec<String> = targets.iter().map(|(i, r)| format!("{i}:{r}")).collect();
    let f = |v: &[f32]| -> String {
        v.iter()
            .map(|x| format!("{:08x}", x.to_bits()))
            .collect::<Vec<_>>()
            .join(",")
    };
    let line = format!(
        "d {} {:?} n{} i{} s{} b{} f{} t{} vs {:016x} ps {:016x} il {layout:016x} vb {vb_text} ib {ifmt}:{index:016x} cb {cb_text} srv {srv_text} samp {samp_text} blend {:016x} {} {:08x} depth {:016x} {} raster {:016x} vp {} sc {:?} rt {} ds {depth}",
        c.draws,
        d.kind,
        d.count,
        d.instances,
        d.start,
        d.base,
        d.first_instance,
        d.topology,
        shader(d.vs),
        shader(d.ps),
        state_hash(d.blend),
        f(&d.blend_factor),
        d.sample_mask,
        state_hash(d.depth),
        d.stencil_ref,
        state_hash(d.raster),
        f(&d.viewport),
        d.scissor,
        rt_text.join(","),
    );
    (
        DrawKeys {
            frame: c.frame,
            draw: c.draws,
            vertex,
            index,
            constants,
            textures,
            targets,
            depth,
        },
        line,
        updates,
    )
}

fn slots(list: &[usize]) -> Vec<(u8, u32)> {
    list.iter()
        .enumerate()
        .filter(|(_, p)| **p != 0)
        .map(|(i, p)| (i as u8, *p as u32))
        .collect()
}

/// The scene-stream form of a draw, with the constant buffer windows the
/// shaders see.
pub(super) fn stream_draw(s: &State, d: Draw) -> ffx_gfx::Draw {
    let b = &s.bound;
    let cbs = |list: &[usize], ranges: &[(u32, u32)]| -> Vec<(u8, Vec<u8>)> {
        list.iter()
            .enumerate()
            .filter(|(_, p)| **p != 0)
            .map(|(i, p)| {
                let data = s
                    .buffers
                    .get(p)
                    .and_then(|x| x.shadow.as_ref())
                    .map_or_else(Vec::new, |sh| {
                        let (first, count) = ranges[i];
                        if count == 0 {
                            sh.clone()
                        } else {
                            let from = (first as usize * 16).min(sh.len());
                            let to = (from + count as usize * 16).min(sh.len());
                            sh[from..to].to_vec()
                        }
                    });
                (i as u8, data)
            })
            .collect()
    };
    let (kind, count, instances, start, base, first) = match d {
        Draw::Indexed { count, start, base } => {
            (ffx_gfx::DrawKind::Indexed, count, 1, start, base, 0)
        }
        Draw::Plain { count, start } => (ffx_gfx::DrawKind::Plain, count, 1, start, 0, 0),
        Draw::IndexedInstanced {
            count,
            instances,
            start,
            base,
            first,
        } => (
            ffx_gfx::DrawKind::IndexedInstanced,
            count,
            instances,
            start,
            base,
            first,
        ),
        Draw::Instanced {
            count,
            instances,
            start,
            first,
        } => (
            ffx_gfx::DrawKind::Instanced,
            count,
            instances,
            start,
            0,
            first,
        ),
    };
    ffx_gfx::Draw {
        kind,
        count,
        instances,
        start,
        base,
        first_instance: first,
        topology: b.topology,
        layout: b.layout as u32,
        vs: b.vs as u32,
        ps: b.ps as u32,
        vertex: b
            .vertex
            .iter()
            .enumerate()
            .filter(|(_, v)| v.0 != 0)
            .map(|(i, v)| (i as u8, v.0 as u32, v.1, v.2))
            .collect(),
        index: (b.index.0 as u32, b.index.1, b.index.2),
        vs_cb: cbs(&b.vs_cb, &b.vs_cb_range),
        ps_cb: cbs(&b.ps_cb, &b.ps_cb_range),
        vs_srv: slots(&b.vs_srv),
        ps_srv: slots(&b.ps_srv),
        vs_samplers: slots(&b.vs_sampler),
        ps_samplers: slots(&b.ps_sampler),
        blend: b.blend as u32,
        blend_factor: b.blend_factor,
        sample_mask: b.sample_mask,
        depth: b.depth as u32,
        stencil_ref: b.stencil_ref,
        raster: b.raster as u32,
        viewport: b.viewport,
        scissor: b.scissor,
        rtv: slots(&b.rtv),
        dsv: b.dsv as u32,
    }
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

/// Records one draw.
#[allow(clippy::too_many_lines)] // one draw: its state, objects and text
pub fn draw(s: &mut State, d: Draw) {
    let Some(mut c) = s.capture.take() else {
        return;
    };
    c.seen();
    c.draws += 1;
    c.draws_since_rt += 1;
    let streamed = stream_draw(s, d);
    if c.opts.hash {
        let (k, line, updates) = keys(s, &mut c, &streamed);
        c.refs.push(Ref::Keys(k));
        c.text.push(line);
        c.cmds.extend(updates);
        c.objects.extend(draw_objects(&streamed));
        c.cmds.push(ffx_gfx::Cmd::Draw(Box::new(streamed)));
        s.capture = Some(c);
        return;
    }
    // Vertex and index rings: their contents at this draw, when they
    // changed since the last one.
    let rings = streamed
        .vertex
        .iter()
        .map(|v| v.1)
        .chain((streamed.index.0 != 0).then_some(streamed.index.0));
    for id in rings {
        if let Some(buf) = s.buffers.get(&(id as usize))
            && buf.desc[1] == 2
            && let Some(sh) = &buf.shadow
            && c.sent_versions.get(&(id as usize)) != Some(&buf.version)
        {
            c.sent_versions.insert(id as usize, buf.version);
            c.cmds.push(ffx_gfx::Cmd::UpdateBuffer {
                id,
                offset: 0,
                data: sh.clone(),
            });
        }
    }
    c.objects.extend(draw_objects(&streamed));
    c.cmds.push(ffx_gfx::Cmd::Draw(Box::new(streamed)));
    let b = &s.bound;
    let shader = |p: usize| {
        s.shaders
            .get(&p)
            .map_or_else(|| format!("{p:#x}"), |sh| format!("{:016x}", sh.hash))
    };
    let (vs, ps) = (shader(b.vs), shader(b.ps));
    let mut cbs = String::new();
    for (stage, slots, ranges) in [
        ("vs", &b.vs_cb, &b.vs_cb_range),
        ("ps", &b.ps_cb, &b.ps_cb_range),
    ] {
        for (i, &p) in slots.iter().enumerate() {
            if p == 0 {
                continue;
            }
            let ver = s.buffers.get(&p).map_or(0, |x| x.version);
            let (first, count) = ranges[i];
            let _ = write!(cbs, "{stage}{i}:{p:#x}@{ver}+{first}/{count},");
            if let Some(sh) = s.buffers.get(&p).and_then(|x| x.shadow.as_ref())
                && c.written.insert((p, ver))
            {
                let _ = std::fs::write(c.opts.dir.join(format!("cb/{p:x}_{ver}.bin")), sh);
            }
        }
    }
    for &(p, _, _) in &b.vertex {
        if p == 0 {
            continue;
        }
        if let Some(buf) = s.buffers.get(&p) {
            // Dynamic vertex data changes within the frame: keep this
            // version.
            if buf.desc[1] == 2
                && let Some(sh) = &buf.shadow
                && c.written.insert((p, buf.version))
            {
                let name = format!("buf/{p:x}_{}.bin", buf.version);
                let _ = std::fs::write(c.opts.dir.join(name), sh);
            }
        }
    }
    for &v in b.ps_srv.iter().chain(&b.vs_srv).chain(&b.rtv) {
        if v != 0 {
            c.views.insert(v);
        }
    }
    if b.dsv != 0 {
        c.views.insert(b.dsv);
    }
    for &(p, _, _) in &b.vertex {
        if p != 0 {
            c.buffers.insert(p);
        }
    }
    if b.index.0 != 0 {
        c.buffers.insert(b.index.0);
    }
    let mut vb = String::new();
    let mut vbver = String::new();
    for (i, v) in b.vertex.iter().enumerate().filter(|(_, v)| v.0 != 0) {
        let _ = write!(vb, "{i}:{:#x}/{}/{},", v.0, v.1, v.2);
        let _ = write!(vbver, "{},", s.buffers.get(&v.0).map_or(0, |b| b.version));
    }
    let line = format!(
        "draw {} {d:?} topo {} vs {vs} ps {ps} il {:#x} vb {vb} vbver {vbver} ib {:#x}/{}/{} rt {} ds {:#x} vp {:?} sc {:?} blend {:#x} {:?} depth {:#x} ref {} raster {:#x} cb {cbs} pssrv {} vssrv {} pssamp {}",
        c.draws,
        b.topology,
        b.layout,
        b.index.0,
        b.index.1,
        b.index.2,
        list(&b.rtv),
        b.dsv,
        b.viewport,
        b.scissor,
        b.blend,
        b.blend_factor,
        b.depth,
        b.stencil_ref,
        b.raster,
        list(&b.ps_srv),
        list(&b.vs_srv),
        list(&b.ps_sampler),
    );
    c.lines.push(line);
    s.capture = Some(c);
}

/// The game is about to bind other render targets: saves (or hashes) the
/// current ones if anything was drawn into them.
pub fn render_targets(s: &mut State, rtv: [usize; 8], dsv: usize) {
    let Some(mut c) = s.capture.take() else {
        return;
    };
    c.seen();
    let (old_rt, old_ds) = (s.bound.rtv[0], s.bound.dsv);
    if c.draws_since_rt > 0 && c.rt_dumps < RT_DUMPS {
        for (view, tag) in [(old_rt, "c"), (old_ds, "d")] {
            if view == 0 {
                continue;
            }
            let Some(res) = s.views.get(&view).map(|v| v.resource) else {
                continue;
            };
            // SAFETY: on the render thread, inside a context call.
            let r = unsafe { read_texture(res) };
            if c.opts.hash {
                let h = r.as_ref().map_or(0, |t| {
                    hash_rows(&t.data, t.pitch, t.rows, row_bytes(t.format, t.width))
                });
                let name = role(s, &mut c, view);
                c.text.push(format!("t {} {name} {h:016x}", c.draws));
                c.refs.push(Ref::Target {
                    frame: c.frame,
                    after_draw: c.draws,
                    view: view as u32,
                    role: name,
                    hash: h,
                });
                continue;
            }
            let name = c.frame_name(&format!("rt/{:03}_{tag}_{view:x}", c.rt_dumps));
            match r {
                Ok(t) => {
                    let _ = std::fs::write(c.opts.dir.join(format!("{name}.raw")), &t.data);
                    c.lines.push(format!(
                        "dump {name} {}x{} format {} pitch {} rows {}",
                        t.width, t.height, t.format, t.pitch, t.rows
                    ));
                }
                Err(e) => c.lines.push(format!("dump {name} failed: {e}")),
            }
        }
        c.rt_dumps += 1;
    }
    c.draws_since_rt = 0;
    c.lines
        .push(format!("om_set_render_targets {} ds {dsv:#x}", list(&rtv)));
    s.capture = Some(c);
}

struct Readback {
    width: u32,
    height: u32,
    format: u32,
    pitch: u32,
    rows: u32,
    data: Vec<u8>,
}

fn block_compressed(format: u32) -> bool {
    matches!(format, 70..=84 | 94..=99)
}

/// Mip 0, slice 0 of a 2D texture, through a staging copy.
unsafe fn read_texture(resource: usize) -> Result<Readback, String> {
    let device = super::DEVICE.load(std::sync::atomic::Ordering::Acquire) as Ptr;
    let context = super::CONTEXT.load(std::sync::atomic::Ordering::Acquire) as Ptr;
    // SAFETY: COM calls on the live device and context with their
    // documented slots; the originals bypass the layer's hooks.
    unsafe {
        let mut desc = [0u32; 11];
        super::com!(
            resource as Ptr,
            vt::RESOURCE_GET_DESC,
            unsafe extern "system" fn(Ptr, *mut u32),
            desc.as_mut_ptr()
        );
        if desc[5] != 1 {
            return Err(format!("multisampled ({})", desc[5]));
        }
        let mut sd = desc;
        sd[2] = 1;
        sd[3] = 1;
        sd[7] = 3; // staging
        sd[8] = 0;
        sd[9] = 0x2_0000; // CPU read
        sd[10] = 0;
        // Typeless depth formats copy to themselves.
        let mut staging: Ptr = std::ptr::null_mut();
        type Create = unsafe extern "system" fn(Ptr, *const u32, *const c_void, *mut Ptr) -> i32;
        let create: Create =
            std::mem::transmute(device_original(device, vt::device::CREATE_TEXTURE_2D));
        let hr = create(device, sd.as_ptr(), std::ptr::null(), &raw mut staging);
        if hr < 0 {
            return Err(format!("CreateTexture2D {hr:#x} format {}", desc[4]));
        }
        type CopyRegion =
            unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, u32, Ptr, u32, *const u32);
        let copy: CopyRegion =
            std::mem::transmute(real_method(context, vt::context::COPY_SUBRESOURCE_REGION));
        copy(
            context,
            staging,
            0,
            0,
            0,
            0,
            resource as Ptr,
            0,
            std::ptr::null(),
        );
        type Map = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, *mut usize) -> i32;
        let map: Map = std::mem::transmute(real_method(context, vt::context::MAP));
        let mut mapped = [0usize; 3];
        let hr = map(context, staging, 0, 1, 0, mapped.as_mut_ptr());
        let result = if hr < 0 {
            Err(format!("Map {hr:#x}"))
        } else {
            let rows = if block_compressed(desc[4]) {
                desc[1].div_ceil(4)
            } else {
                desc[1]
            };
            let pitch = mapped[1] as u32;
            let data = std::slice::from_raw_parts(mapped[0] as *const u8, (pitch * rows) as usize)
                .to_vec();
            type Unmap = unsafe extern "system" fn(Ptr, Ptr, u32);
            let unmap: Unmap = std::mem::transmute(real_method(context, vt::context::UNMAP));
            unmap(context, staging, 0);
            Ok(Readback {
                width: desc[0],
                height: desc[1],
                format: desc[4],
                pitch,
                rows,
                data,
            })
        };
        super::com!(staging, vt::RELEASE, unsafe extern "system" fn(Ptr) -> u32);
        result
    }
}

/// A buffer's contents, through a staging copy.
pub(super) unsafe fn read_buffer(resource: usize, width: u32) -> Result<Vec<u8>, String> {
    let device = super::DEVICE.load(std::sync::atomic::Ordering::Acquire) as Ptr;
    let context = super::CONTEXT.load(std::sync::atomic::Ordering::Acquire) as Ptr;
    // SAFETY: as in `read_texture`.
    unsafe {
        let desc = [width, 3, 0, 0x2_0000, 0, 0];
        let mut staging: Ptr = std::ptr::null_mut();
        type Create = unsafe extern "system" fn(Ptr, *const u32, *const c_void, *mut Ptr) -> i32;
        let create: Create =
            std::mem::transmute(device_original(device, vt::device::CREATE_BUFFER));
        let hr = create(device, desc.as_ptr(), std::ptr::null(), &raw mut staging);
        if hr < 0 {
            return Err(format!("CreateBuffer {hr:#x}"));
        }
        type Copy = unsafe extern "system" fn(Ptr, Ptr, Ptr);
        let copy: Copy = std::mem::transmute(real_method(context, vt::context::COPY_RESOURCE));
        copy(context, staging, resource as Ptr);
        type Map = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, *mut usize) -> i32;
        let map: Map = std::mem::transmute(real_method(context, vt::context::MAP));
        let mut mapped = [0usize; 3];
        let hr = map(context, staging, 0, 1, 0, mapped.as_mut_ptr());
        let result = if hr < 0 {
            Err(format!("Map {hr:#x}"))
        } else {
            let data = std::slice::from_raw_parts(mapped[0] as *const u8, width as usize).to_vec();
            type Unmap = unsafe extern "system" fn(Ptr, Ptr, u32);
            let unmap: Unmap = std::mem::transmute(real_method(context, vt::context::UNMAP));
            unmap(context, staging, 0);
            Ok(data)
        };
        super::com!(staging, vt::RELEASE, unsafe extern "system" fn(Ptr) -> u32);
        result
    }
}

/// The game's threads: id, start address (an `FFX.exe` address when in
/// the image) and role (`main`: created first; `render`: called
/// `Present`).
fn threads(render: u32) -> Vec<ThreadInfo> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, GetThreadTimes, OpenThread, THREAD_QUERY_INFORMATION,
    };
    type Query = unsafe extern "system" fn(isize, u32, *mut c_void, u32, *mut u32) -> i32;
    let mut out: Vec<(u64, ThreadInfo)> = Vec::new();
    // SAFETY: plain Win32 calls; every pointer is local.
    unsafe {
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr().cast());
        let Some(query) = GetProcAddress(ntdll, c"NtQueryInformationThread".as_ptr().cast()) else {
            return Vec::new();
        };
        let query: Query = std::mem::transmute(query);
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        let pid = GetCurrentProcessId();
        let mut te: THREADENTRY32 = std::mem::zeroed();
        te.dwSize = u32::try_from(size_of::<THREADENTRY32>()).unwrap_or(0);
        let base = crate::GAME_BASE.load(Ordering::Relaxed);
        let mut ok = Thread32First(snap, &raw mut te) != 0;
        while ok {
            if te.th32OwnerProcessID == pid {
                let h = OpenThread(THREAD_QUERY_INFORMATION, 0, te.th32ThreadID);
                let mut start: usize = 0;
                // ThreadQuerySetWin32StartAddress = 9
                query(
                    h as isize,
                    9,
                    (&raw mut start).cast(),
                    4,
                    std::ptr::null_mut(),
                );
                let zero = FILETIME {
                    dwLowDateTime: 0,
                    dwHighDateTime: 0,
                };
                let (mut created, mut a, mut b, mut d) = (zero, zero, zero, zero);
                GetThreadTimes(h, &raw mut created, &raw mut a, &raw mut b, &raw mut d);
                CloseHandle(h);
                let start = if start >= base && start < base + crate::SIZE_OF_IMAGE as usize {
                    start - base + crate::IMAGE_BASE as usize
                } else {
                    start
                };
                let when =
                    u64::from(created.dwLowDateTime) | (u64::from(created.dwHighDateTime) << 32);
                out.push((
                    when,
                    ThreadInfo {
                        id: te.th32ThreadID,
                        start: start as u32,
                        role: String::new(),
                    },
                ));
            }
            ok = Thread32Next(snap, &raw mut te) != 0;
        }
        CloseHandle(snap);
    }
    out.sort_by_key(|(when, t)| (*when, t.id));
    let mut list: Vec<ThreadInfo> = out.into_iter().map(|(_, t)| t).collect();
    for (i, t) in list.iter_mut().enumerate() {
        if i == 0 {
            t.role = "main".into();
        }
        if t.id == render {
            t.role = if i == 0 { "main,render" } else { "render" }.into();
        }
    }
    list
}

/// Ends the current frame at its `Present`: the back buffer, the tags, a
/// scene sample.
///
/// # Safety
/// On the render thread, inside Present.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // viewport sizes
unsafe fn end_frame(s: &mut State, c: &mut Capture, swap_chain: Ptr) {
    c.seen();
    c.present_thread = thread_id();
    // The back buffer as presented.
    // SAFETY: as documented on the swap chain.
    unsafe {
        const IID_TEXTURE2D: [u8; 16] = [
            0xf2, 0xaa, 0x15, 0x6f, 0x08, 0xd2, 0x89, 0x4e, 0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3,
            0x4f, 0x9c,
        ];
        let mut back: Ptr = std::ptr::null_mut();
        let hr = super::com!(
            swap_chain,
            vt::GET_BUFFER,
            unsafe extern "system" fn(Ptr, u32, *const u8, *mut Ptr) -> i32,
            0,
            IID_TEXTURE2D.as_ptr(),
            &raw mut back
        );
        if hr >= 0 && !back.is_null() {
            if let Ok(t) = read_texture(back as usize) {
                if c.opts.hash {
                    let h = hash_rows(&t.data, t.pitch, t.rows, row_bytes(t.format, t.width));
                    c.text.push(format!("p backbuffer {h:016x}"));
                    c.refs.push(Ref::Target {
                        frame: c.frame,
                        after_draw: c.draws,
                        view: 0,
                        role: "backbuffer".into(),
                        hash: h,
                    });
                } else {
                    let name = c.frame_name("rt/backbuffer");
                    let _ = std::fs::write(c.opts.dir.join(format!("{name}.raw")), &t.data);
                    c.lines.push(format!(
                        "dump {name} {:#x} {}x{} format {} pitch {} rows {}",
                        back as usize, t.width, t.height, t.format, t.pitch, t.rows
                    ));
                }
            }
            super::com!(back, vt::RELEASE, unsafe extern "system" fn(Ptr) -> u32);
        }
    }
    let event = crate::harness::read_memory(crate::rebase(crate::world::EVENT_COUNT), 4)
        .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    c.refs.push(Ref::Tag(FrameTag {
        frame: c.frame,
        present: present_count(),
        event_count: event,
        draws: c.draws,
        calls: std::mem::take(&mut c.calls).into_iter().collect(),
        present_thread: c.present_thread,
    }));
    c.add_sample(sample(c.frame, When::Present));
    // The scene inputs of the update this frame shows.
    if let Some(label) = crate::harness::present_label() {
        let mut n = 0;
        for (i, _) in &mut c.inputs {
            if i.update == label {
                i.frame = c.frame;
                n += 1;
            }
        }
        c.lines
            .push(format!("shows update {label}, {n} scene inputs"));
    }
    c.frames.push(ffx_gfx::Frame {
        number: c.frame,
        width: s.bound.viewport[2] as u32,
        height: s.bound.viewport[3] as u32,
        cmds: std::mem::take(&mut c.cmds),
    });
    c.lines.push(format!("present {}", present_count()));
}

/// At each Present: ends a captured frame (and the capture after its last
/// frame), or starts a requested capture.
///
/// # Safety
/// On the render thread, inside Present.
pub unsafe fn on_present(swap_chain: Ptr) {
    let mut s = state();
    if let Some(mut c) = s.capture.take() {
        // SAFETY: forwarded.
        unsafe { end_frame(&mut s, &mut c, swap_chain) };
        if c.frame + 1 < c.opts.frames {
            c.frame += 1;
            c.draws = 0;
            c.rt_dumps = 0;
            c.draws_since_rt = 0;
            c.text.push(format!("frame {}", c.frame));
            c.lines.push(format!("frame {}", c.frame));
            s.capture = Some(c);
            return;
        }
        INPUTS_WANTED.store(false, Ordering::Release);
        c.lines.extend(crate::world::watch::report());
        let reply = if c.opts.hash {
            finish_hashed(&s, c)
        } else {
            finish(&s, c)
        };
        // The copies of dynamic vertex and index buffers were for the
        // capture: kept, they would stand for the buffers from now on
        // (the live link reads a copy before the mapped memory) and
        // freeze what they draw.
        for b in s.buffers.values_mut() {
            if b.desc[1] == 2 && b.desc[2] & (super::BIND_VERTEX | super::BIND_INDEX) != 0 {
                b.shadow = None;
            }
        }
        drop(s);
        let (msg, tx) = reply;
        if let Some(tx) = tx {
            let _ = tx.send(msg);
        }
        return;
    }
    SWAP_CHAIN.store(swap_chain as usize, Ordering::Release);
    if let Some(mut c) = PENDING.lock().ok().and_then(|mut p| p.take()) {
        c.add_sample(sample(0, When::Present));
        // SAFETY: inside its Present.
        c.backbuffer = unsafe { back_buffer(swap_chain) };
        c.reborn_at = s.reborn.len();
        c.lines
            .push(format!("start at present {}", present_count()));
        s.capture = Some(c);
        INPUTS_WANTED.store(true, Ordering::Release);
    }
}

/// The per-frame tags and samples as text (`frames.txt`).
fn frames_text(refs: &[Ref]) -> String {
    let mut out = String::new();
    for r in refs {
        match r {
            Ref::Tag(t) => {
                let calls: Vec<String> =
                    t.calls.iter().map(|(id, n)| format!("{id}:{n}")).collect();
                let _ = writeln!(
                    out,
                    "tag frame {} present {} event {} draws {} present_thread {} calls {}",
                    t.frame,
                    t.present,
                    t.event_count,
                    t.draws,
                    t.present_thread,
                    calls.join(",")
                );
            }
            Ref::Scene(sm) => {
                let when = match sm.when {
                    When::Held => "held",
                    When::Present => "present",
                };
                let _ = writeln!(
                    out,
                    "sample frame {} {when} thread {} present {}",
                    sm.frame, sm.thread, sm.present
                );
                for (name, bytes) in &sm.fields {
                    let _ = writeln!(out, "  {name} {}", hex(bytes));
                }
                for (at, path) in &sm.opened {
                    let _ = writeln!(out, "  opened {at} {path}");
                }
            }
            Ref::Threads(list) => {
                for t in list {
                    let _ = writeln!(out, "thread {} start {:08x} {}", t.id, t.start, t.role);
                }
            }
            Ref::Reborn { frame, id } => {
                let _ = writeln!(out, "reborn {id:#x} by frame {frame}");
            }
            Ref::Keys(_) | Ref::Target { .. } => {}
        }
    }
    out
}

/// Writes the frame log, the objects the frames used, their contents (or
/// hashes) and the reference records.
#[allow(clippy::too_many_lines)] // the report, the snapshot, the files
fn finish(s: &State, mut c: Capture) -> (String, Option<Sender<String>>) {
    let started = std::time::Instant::now();
    let dir = c.opts.dir.clone();
    c.refs.push(Ref::Threads(threads(c.present_thread)));
    for &id in s.reborn.get(c.reborn_at..).unwrap_or(&[]) {
        c.refs.push(Ref::Reborn {
            frame: c.frame,
            id: id as u32,
        });
    }
    let mut objects = String::new();
    let mut resources = BTreeSet::new();
    for &v in &c.views {
        if let Some(view) = s.views.get(&v) {
            let _ = writeln!(
                objects,
                "view {v:#x} {:?} resource {:#x} desc {:?}",
                view.kind, view.resource, view.desc
            );
            if view.kind == ViewKind::ShaderResource {
                resources.insert(view.resource);
            }
        }
    }
    for &r in &resources {
        if let Some(t) = s.textures.get(&r) {
            let _ = writeln!(objects, "texture {r:#x} desc {:?}", t.desc);
            // SAFETY: on the render thread.
            match unsafe { read_texture(r) } {
                Ok(t) => {
                    let _ = std::fs::write(dir.join(format!("tex/{r:x}.raw")), &t.data);
                    let _ = writeln!(
                        objects,
                        "texdata {r:#x} {}x{} format {} pitch {} rows {}",
                        t.width, t.height, t.format, t.pitch, t.rows
                    );
                }
                Err(e) => {
                    let _ = writeln!(objects, "texdata {r:#x} failed {e}");
                }
            }
        }
    }
    let views: Vec<usize> = c.views.iter().copied().collect();
    for v in views {
        if let Some(view) = s.views.get(&v)
            && view.kind != ViewKind::ShaderResource
            && let Some(t) = s.textures.get(&view.resource)
        {
            let _ = writeln!(objects, "texture {:#x} desc {:?}", view.resource, t.desc);
        }
    }
    for &b in &c.buffers {
        if let Some(buf) = s.buffers.get(&b) {
            let _ = writeln!(
                objects,
                "buffer {b:#x} desc {:?} version {}",
                buf.desc, buf.version
            );
            // Static vertex and index data: read it back once.
            if buf.desc[1] != 2 {
                // SAFETY: on the render thread.
                if let Ok(data) = unsafe { read_buffer(b, buf.desc[0]) } {
                    let _ = std::fs::write(dir.join(format!("buf/{b:x}.bin")), data);
                }
            }
        }
    }
    let mut layouts: BTreeMap<usize, String> = BTreeMap::new();
    for (p, l) in &s.layouts {
        let mut line = format!("layout {p:#x} sig {:016x}", l.signature);
        for e in &l.elements {
            let _ = write!(
                line,
                " {}{}:f{}:s{}:o{}{}",
                e.semantic,
                e.index,
                e.format,
                e.slot,
                e.offset,
                if e.per_instance { ":inst" } else { "" }
            );
        }
        layouts.insert(*p, line);
    }
    for l in layouts.values() {
        let _ = writeln!(objects, "{l}");
    }
    let mut states: Vec<_> = s.states.iter().collect();
    states.sort_by_key(|(p, _)| **p);
    for (p, (kind, bytes)) in states {
        let _ = writeln!(objects, "state {p:#x} {kind} {}", hex(bytes));
    }
    let mut shaders: Vec<_> = s.shaders.iter().collect();
    shaders.sort_by_key(|(p, _)| **p);
    for (p, sh) in shaders {
        let _ = writeln!(
            objects,
            "shader {p:#x} {} {:016x} {}",
            sh.kind,
            sh.hash,
            sh.bytecode.len()
        );
        let path = dir.join(format!("shaders/{:016x}.{}s", sh.hash, sh.kind));
        if !path.exists() {
            let _ = std::fs::write(path, &sh.bytecode[..]);
        }
    }
    let _ = std::fs::write(dir.join("objects.txt"), objects);
    let draws: u32 = c.frames.iter().map(frame_draws).sum();
    let frames = c.frames.len();
    let stream = snapshot(s, &mut c);
    let _ = std::fs::write(dir.join("capture.ffxgfx"), stream);
    let mut text = c.lines.join("\n");
    text.push('\n');
    let _ = std::fs::write(dir.join("frame.txt"), text);
    write_refs(&c);
    (
        format!(
            "ok {draws} draws, {frames} frames, written in {} ms",
            started.elapsed().as_millis()
        ),
        c.reply.take(),
    )
}

fn frame_draws(f: &ffx_gfx::Frame) -> u32 {
    f.cmds
        .iter()
        .filter(|c| matches!(c, ffx_gfx::Cmd::Draw(_)))
        .count() as u32
}

/// `capture.ffxref`, `frames.txt` and `hashes.txt`.
fn write_refs(c: &Capture) {
    let dir = &c.opts.dir;
    let mut out = ffx_gfx::reference::MAGIC.to_vec();
    for r in &c.refs {
        r.write(&mut out);
    }
    let _ = std::fs::write(dir.join("capture.ffxref"), out);
    let _ = std::fs::write(dir.join("frames.txt"), frames_text(&c.refs));
    if c.opts.hash {
        let mut text = c.text.join("\n");
        text.push('\n');
        let _ = std::fs::write(dir.join("hashes.txt"), text);
    }
}

/// Hash-only `finish`: the objects without their contents (which
/// `hashes.txt` and the draw keys stand for), shaders included (the
/// constants' reflection needs them).
fn finish_hashed(s: &State, mut c: Capture) -> (String, Option<Sender<String>>) {
    let started = std::time::Instant::now();
    c.refs.push(Ref::Threads(threads(c.present_thread)));
    for &id in s.reborn.get(c.reborn_at..).unwrap_or(&[]) {
        c.refs.push(Ref::Reborn {
            frame: c.frame,
            id: id as u32,
        });
    }
    let draws: u32 = c.frames.iter().map(frame_draws).sum();
    let frames = c.frames.len();
    let stream = snapshot(s, &mut c);
    let _ = std::fs::write(c.opts.dir.join("capture.ffxgfx"), stream);
    let mut text = c.lines.join("\n");
    text.push('\n');
    let _ = std::fs::write(c.opts.dir.join("frame.txt"), text);
    write_refs(&c);
    (
        format!(
            "ok {draws} draws, {frames} frames, written in {} ms",
            started.elapsed().as_millis()
        ),
        c.reply.take(),
    )
}

/// All subresources of a 2D texture, through a staging copy.
unsafe fn read_texture_all(
    resource: usize,
    desc: &[u32; 11],
) -> Result<Vec<ffx_gfx::Subresource>, String> {
    let device = super::DEVICE.load(std::sync::atomic::Ordering::Acquire) as Ptr;
    let context = super::CONTEXT.load(std::sync::atomic::Ordering::Acquire) as Ptr;
    if desc[5] != 1 {
        return Err("multisampled".into());
    }
    // SAFETY: as in `read_texture`.
    unsafe {
        let mut sd = *desc;
        sd[7] = 3;
        sd[8] = 0;
        sd[9] = 0x2_0000;
        sd[10] &= 0x4; // keep only TEXTURECUBE
        let mut staging: Ptr = std::ptr::null_mut();
        type Create = unsafe extern "system" fn(Ptr, *const u32, *const c_void, *mut Ptr) -> i32;
        let create: Create =
            std::mem::transmute(device_original(device, vt::device::CREATE_TEXTURE_2D));
        let hr = create(device, sd.as_ptr(), std::ptr::null(), &raw mut staging);
        if hr < 0 {
            return Err(format!("CreateTexture2D {hr:#x}"));
        }
        type Copy = unsafe extern "system" fn(Ptr, Ptr, Ptr);
        let copy: Copy = std::mem::transmute(real_method(context, vt::context::COPY_RESOURCE));
        copy(context, staging, resource as Ptr);
        type Map = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, *mut usize) -> i32;
        type Unmap = unsafe extern "system" fn(Ptr, Ptr, u32);
        let map: Map = std::mem::transmute(real_method(context, vt::context::MAP));
        let unmap: Unmap = std::mem::transmute(real_method(context, vt::context::UNMAP));
        let (mips, slices) = (desc[2].max(1), desc[3].max(1));
        let mut out = Vec::new();
        let mut result = Ok(());
        'all: for slice in 0..slices {
            for mip in 0..mips {
                let sub = mip + slice * mips;
                let mut mapped = [0usize; 3];
                let hr = map(context, staging, sub, 1, 0, mapped.as_mut_ptr());
                if hr < 0 {
                    result = Err(format!("Map {hr:#x}"));
                    break 'all;
                }
                let h = (desc[1] >> mip).max(1);
                let rows = if block_compressed(desc[4]) {
                    h.div_ceil(4)
                } else {
                    h
                };
                let pitch = mapped[1] as u32;
                let data =
                    std::slice::from_raw_parts(mapped[0] as *const u8, (pitch * rows) as usize)
                        .to_vec();
                unmap(context, staging, sub);
                out.push(ffx_gfx::Subresource {
                    row_pitch: pitch,
                    data,
                });
            }
        }
        super::com!(staging, vt::RELEASE, unsafe extern "system" fn(Ptr) -> u32);
        result.map(|()| out)
    }
}

/// The scene-stream message describing object `id` (contents read back
/// from the GPU for textures and static buffers), or `None` when the
/// layer does not know it (or it is a view: see [`view_msg`]).
pub(super) fn object_msg(s: &State, id: usize) -> Option<ffx_gfx::Msg> {
    use ffx_gfx::Msg;
    if let Some(sh) = s.shaders.get(&id) {
        return matches!(sh.kind, 'v' | 'p').then(|| Msg::Shader {
            id: id as u32,
            kind: sh.kind as u8,
            bytecode: sh.bytecode.to_vec(),
        });
    }
    if let Some(l) = s.layouts.get(&id) {
        return Some(Msg::Layout {
            id: id as u32,
            elements: l
                .elements
                .iter()
                .map(|e| ffx_gfx::Element {
                    semantic: e.semantic.clone(),
                    index: e.index,
                    format: e.format,
                    slot: e.slot,
                    offset: e.offset,
                    per_instance: e.per_instance,
                    step: e.step,
                })
                .collect(),
        });
    }
    if let Some((kind, desc)) = s.states.get(&id) {
        let kind = match kind {
            'b' => ffx_gfx::StateKind::Blend,
            'd' => ffx_gfx::StateKind::Depth,
            'r' => ffx_gfx::StateKind::Raster,
            _ => ffx_gfx::StateKind::Sampler,
        };
        return Some(Msg::State {
            id: id as u32,
            kind,
            desc: desc.clone(),
        });
    }
    if let Some(t) = s.textures.get(&id) {
        // SAFETY: on the render thread.
        let subresources = unsafe { read_texture_all(id, &t.desc) }.unwrap_or_default();
        return Some(Msg::Texture {
            id: id as u32,
            desc: t.desc,
            subresources,
        });
    }
    if let Some(b) = s.buffers.get(&id) {
        let data = if b.desc[1] == 2 {
            b.shadow.clone().unwrap_or_default()
        } else {
            // SAFETY: on the render thread.
            unsafe { read_buffer(id, b.desc[0]) }.unwrap_or_default()
        };
        return Some(Msg::Buffer {
            id: id as u32,
            desc: b.desc,
            data,
        });
    }
    None
}

/// The message for view `id`.
pub(super) fn view_msg(s: &State, id: usize) -> Option<ffx_gfx::Msg> {
    let v = s.views.get(&id)?;
    let kind = match v.kind {
        ViewKind::ShaderResource => ffx_gfx::ViewKind::ShaderResource,
        ViewKind::RenderTarget => ffx_gfx::ViewKind::RenderTarget,
        ViewKind::DepthStencil => ffx_gfx::ViewKind::DepthStencil,
    };
    Some(ffx_gfx::Msg::View {
        id: id as u32,
        kind,
        resource: v.resource as u32,
        desc: v.desc,
    })
}

/// The ids a draw refers to (views included, not their resources).
pub(super) fn draw_objects(d: &ffx_gfx::Draw) -> impl Iterator<Item = usize> + '_ {
    [
        d.layout, d.vs, d.ps, d.index.0, d.blend, d.depth, d.raster, d.dsv,
    ]
    .into_iter()
    .chain(d.vertex.iter().map(|v| v.1))
    .chain(
        d.vs_srv
            .iter()
            .chain(&d.ps_srv)
            .chain(&d.vs_samplers)
            .chain(&d.ps_samplers)
            .chain(&d.rtv)
            .map(|v| v.1),
    )
    .filter(|&id| id != 0)
    .map(|id| id as usize)
}

/// A light message for object `id` (hash-only captures): a texture's one
/// subresource holds its 8-byte content hash (row pitch 0; 0 for render
/// and depth targets, whose contents change in the frame), a static
/// vertex buffer's data its hash, a static index buffer its data, a ring
/// nothing (each draw's `UpdateBuffer` brings the bytes it reads).
fn light_msg(s: &State, c: &mut Capture, id: usize) -> Option<ffx_gfx::Msg> {
    if let Some(t) = s.textures.get(&id) {
        let target = t.desc[8] & (BIND_RENDER_TARGET | BIND_DEPTH_STENCIL) != 0;
        let h = if target { 0 } else { texture_hash(s, c, id) };
        return Some(ffx_gfx::Msg::Texture {
            id: id as u32,
            desc: t.desc,
            subresources: vec![ffx_gfx::Subresource {
                row_pitch: 0,
                data: h.to_le_bytes().to_vec(),
            }],
        });
    }
    if let Some(b) = s.buffers.get(&id) {
        let data = if b.desc[1] == 2 {
            Vec::new()
        } else if b.desc[2] & super::BIND_INDEX != 0 {
            buffer_bytes(s, c, id).unwrap_or_default()
        } else {
            buffer_bytes(s, c, id)
                .map(|d| super::hash(&d).to_le_bytes().to_vec())
                .unwrap_or_default()
        };
        return Some(ffx_gfx::Msg::Buffer {
            id: id as u32,
            desc: b.desc,
            data,
        });
    }
    object_msg(s, id)
}

/// The capture as a scene stream: every object the frames used (textures
/// and buffers read back now, or without contents in hash mode), then the
/// frames.
fn snapshot(s: &State, c: &mut Capture) -> Vec<u8> {
    let mut out = ffx_gfx::CAPTURE_MAGIC.to_vec();
    // Views pull in their resources.
    let mut objects = c.objects.clone();
    for id in &c.objects {
        if let Some(v) = s.views.get(id) {
            objects.insert(v.resource);
        }
    }
    let mut msgs: Vec<ffx_gfx::Msg> = Vec::new();
    for &id in &objects {
        let m = if c.opts.hash {
            light_msg(s, c, id)
        } else {
            object_msg(s, id)
        };
        msgs.extend(m);
    }
    // Views after their resources.
    msgs.extend(objects.iter().filter_map(|&id| view_msg(s, id)));
    for m in &msgs {
        m.write(&mut out);
    }
    crate::world::msg().write(&mut out);
    // Each frame's scene inputs before the frame (inputs of updates no
    // captured frame shows are left out).
    let mut inputs = std::mem::take(&mut c.inputs);
    for f in std::mem::take(&mut c.frames) {
        for (i, wire) in inputs.extract_if(.., |(i, _)| i.frame == f.number) {
            ffx_gfx::scene_inputs::write_message(&i, wire.as_ref(), &mut out);
        }
        ffx_gfx::Msg::Frame(f).write(&mut out);
    }
    out
}
