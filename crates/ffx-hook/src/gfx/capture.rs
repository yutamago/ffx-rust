//! One frame's Direct3D calls to disk, for study: every draw with what it
//! had bound, the constant buffer contents it saw, the shaders' bytecode,
//! the textures and buffers it read, and each render target as it was
//! when the game moved on to the next one.
//!
//! All GPU read-backs here call the original methods directly: the layer's
//! own hooks take the state lock, which the caller already holds.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::c_void;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};

use super::{Draw, State, ViewKind, device_original, real_method, state, vt};

type Ptr = *mut c_void;

pub struct Capture {
    dir: PathBuf,
    lines: Vec<String>,
    draws: u32,
    written: HashSet<(usize, u32)>,
    /// Views and buffers the frame used.
    views: BTreeSet<usize>,
    buffers: BTreeSet<usize>,
    rt_dumps: u32,
    draws_since_rt: u32,
    reply: Option<Sender<String>>,
    /// The frame as scene-stream commands, and what they refer to.
    cmds: Vec<ffx_gfx::Cmd>,
    objects: BTreeSet<usize>,
    sent_versions: std::collections::HashMap<usize, u32>,
}

static PENDING: Mutex<Option<(PathBuf, Sender<String>)>> = Mutex::new(None);
static DRAW_RANGE: Mutex<Option<(u32, u32)>> = Mutex::new(None);

/// After a draw: saves render target 0 when the draw is in the asked range.
pub fn after_draw(s: &mut State) {
    let Some(c) = &s.capture else { return };
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
        let name = format!("rt/draw_{n:04}");
        let _ = std::fs::write(c.dir.join(format!("{name}.raw")), &t.data);
        c.lines.push(format!(
            "dump {name} {}x{} format {} pitch {} rows {}",
            t.width, t.height, t.format, t.pitch, t.rows
        ));
    }
}

/// Captures the next whole frame into `dir` (a Windows path); returns the
/// harness reply once it is written.
pub fn request(args: &str) -> String {
    if !super::ENABLED.load(std::sync::atomic::Ordering::Acquire) {
        return "err the graphics layer is off (FFX_GFX=1)".into();
    }
    // `<dir> [first last]`: also save render target 0 after each draw in
    // first..=last.
    let words: Vec<&str> = args.split_whitespace().collect();
    let range = match words.as_slice() {
        [.., a, b] if a.parse::<u32>().is_ok() && b.parse::<u32>().is_ok() => {
            Some((a.parse().unwrap_or(0), b.parse().unwrap_or(0)))
        }
        _ => None,
    };
    let dir_words = if range.is_some() {
        &words[..words.len() - 2]
    } else {
        &words[..]
    };
    if let Ok(mut r) = DRAW_RANGE.lock() {
        *r = range;
    }
    let dir = PathBuf::from(dir_words.join(" "));
    for sub in ["", "shaders", "tex", "buf", "rt", "cb"] {
        if let Err(e) = std::fs::create_dir_all(dir.join(sub)) {
            return format!("err {e}");
        }
    }
    let (tx, rx) = channel();
    if let Ok(mut p) = PENDING.lock() {
        *p = Some((dir, tx));
    }
    rx.recv().unwrap_or_else(|_| "err no reply".into())
}

/// Adds a line to the capture, if one is running.
pub fn note(s: &mut State, line: &str) {
    if let Some(c) = &mut s.capture {
        c.lines.push(line.to_owned());
    }
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

/// Records a render target clear.
pub fn clear_target(s: &mut State, view: usize, color: [f32; 4]) {
    if let Some(c) = &mut s.capture {
        c.objects.insert(view);
        c.cmds.push(ffx_gfx::Cmd::ClearTarget {
            view: view as u32,
            color,
        });
    }
}

/// Records a depth-stencil clear.
pub fn clear_depth(s: &mut State, view: usize, flags: u32, depth: f32, stencil: u8) {
    if let Some(c) = &mut s.capture {
        c.objects.insert(view);
        c.cmds.push(ffx_gfx::Cmd::ClearDepth {
            view: view as u32,
            flags,
            depth,
            stencil,
        });
    }
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

/// Records one draw.
pub fn draw(s: &mut State, d: Draw) {
    let streamed = stream_draw(s, d);
    let mut updates = Vec::new();
    for &(_, id, _, _) in &streamed.vertex {
        if let Some(buf) = s.buffers.get(&(id as usize))
            && buf.desc[1] == 2
            && let Some(sh) = &buf.shadow
            && s.capture
                .as_ref()
                .is_some_and(|c| c.sent_versions.get(&(id as usize)) != Some(&buf.version))
        {
            updates.push((id as usize, buf.version, sh.clone()));
        }
    }
    if let Some(c) = &mut s.capture {
        for (id, version, data) in updates {
            c.sent_versions.insert(id, version);
            c.cmds.push(ffx_gfx::Cmd::UpdateBuffer {
                id: id as u32,
                offset: 0,
                data,
            });
        }
        c.objects.extend(draw_objects(&streamed));
        c.cmds.push(ffx_gfx::Cmd::Draw(Box::new(streamed)));
    }
    let b = s.bound.clone();
    let shader = |p: usize| {
        s.shaders
            .get(&p)
            .map_or_else(|| format!("{p:#x}"), |sh| format!("{:016x}", sh.hash))
    };
    let (vs, ps) = (shader(b.vs), shader(b.ps));
    let mut cbs = String::new();
    let mut blobs = Vec::new();
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
            if let Some(sh) = s.buffers.get(&p).and_then(|x| x.shadow.clone()) {
                blobs.push((format!("cb/{p:x}_{ver}.bin"), (p, ver), sh));
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
                && let Some(sh) = buf.shadow.clone()
            {
                blobs.push((
                    format!("buf/{p:x}_{}.bin", buf.version),
                    (p, buf.version),
                    sh,
                ));
            }
        }
    }
    let Some(c) = &mut s.capture else { return };
    c.draws += 1;
    c.draws_since_rt += 1;
    for (name, key, bytes) in blobs {
        if c.written.insert(key) {
            let _ = std::fs::write(c.dir.join(name), bytes);
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
        let _ = write!(vbver, "{},", s_version(&s.buffers, v.0));
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
}

fn s_version(buffers: &std::collections::HashMap<usize, super::Buffer>, p: usize) -> u32 {
    buffers.get(&p).map_or(0, |b| b.version)
}

/// The game is about to bind other render targets: saves the current ones
/// if anything was drawn into them.
pub fn render_targets(s: &mut State, rtv: [usize; 8], dsv: usize) {
    let (old_rt, old_ds) = (s.bound.rtv[0], s.bound.dsv);
    let dump = s
        .capture
        .as_ref()
        .is_some_and(|c| c.draws_since_rt > 0 && c.rt_dumps < 200);
    if dump {
        for (view, tag) in [(old_rt, "c"), (old_ds, "d")] {
            if view == 0 {
                continue;
            }
            let Some(res) = s.views.get(&view).map(|v| v.resource) else {
                continue;
            };
            let n = s.capture.as_ref().map_or(0, |c| c.rt_dumps);
            let name = format!("rt/{n:03}_{tag}_{view:x}");
            // SAFETY: on the render thread, inside a context call.
            let r = unsafe { read_texture(res) };
            if let Some(c) = &mut s.capture {
                match r {
                    Ok(t) => {
                        let _ = std::fs::write(c.dir.join(format!("{name}.raw")), &t.data);
                        c.lines.push(format!(
                            "dump {name} {}x{} format {} pitch {} rows {}",
                            t.width, t.height, t.format, t.pitch, t.rows
                        ));
                    }
                    Err(e) => c.lines.push(format!("dump {name} failed: {e}")),
                }
            }
        }
        if let Some(c) = &mut s.capture {
            c.rt_dumps += 1;
        }
    }
    if let Some(c) = &mut s.capture {
        c.draws_since_rt = 0;
        c.lines
            .push(format!("om_set_render_targets {} ds {dsv:#x}", list(&rtv)));
    }
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
unsafe fn read_buffer(resource: usize, width: u32) -> Result<Vec<u8>, String> {
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

/// At each Present: finishes a running capture, or starts a requested one.
///
/// # Safety
/// On the render thread, inside Present.
pub unsafe fn on_present(swap_chain: Ptr) {
    let mut s = state();
    if s.capture.is_some() {
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
                if let (Ok(t), Some(c)) = (read_texture(back as usize), &mut s.capture) {
                    let _ = std::fs::write(c.dir.join("rt/backbuffer.raw"), &t.data);
                    c.lines.push(format!(
                        "dump rt/backbuffer {:#x} {}x{} format {} pitch {} rows {}",
                        back as usize, t.width, t.height, t.format, t.pitch, t.rows
                    ));
                }
                super::com!(back, vt::RELEASE, unsafe extern "system" fn(Ptr) -> u32);
            }
        }
        if let Some(c) = s.capture.take() {
            let reply = finish(&s, c);
            drop(s);
            let (msg, tx) = reply;
            if let Some(tx) = tx {
                let _ = tx.send(msg);
            }
            return;
        }
    }
    if let Some((dir, tx)) = PENDING.lock().ok().and_then(|mut p| p.take()) {
        s.capture = Some(Capture {
            dir,
            lines: Vec::new(),
            draws: 0,
            written: HashSet::new(),
            views: BTreeSet::new(),
            buffers: BTreeSet::new(),
            rt_dumps: 0,
            draws_since_rt: 0,
            reply: Some(tx),
            cmds: Vec::new(),
            objects: BTreeSet::new(),
            sent_versions: std::collections::HashMap::new(),
        });
    }
}

/// Writes the frame log, the objects it used, and their contents.
fn finish(s: &State, mut c: Capture) -> (String, Option<Sender<String>>) {
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
                    let _ = std::fs::write(c.dir.join(format!("tex/{r:x}.raw")), &t.data);
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
                    let _ = std::fs::write(c.dir.join(format!("buf/{b:x}.bin")), data);
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
        let mut hex = String::new();
        for b in bytes {
            let _ = write!(hex, "{b:02x}");
        }
        let _ = writeln!(objects, "state {p:#x} {kind} {hex}");
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
        let path = c.dir.join(format!("shaders/{:016x}.{}s", sh.hash, sh.kind));
        if !path.exists() {
            let _ = std::fs::write(path, &sh.bytecode[..]);
        }
    }
    let _ = std::fs::write(c.dir.join("objects.txt"), objects);
    let stream = snapshot(s, &mut c);
    let _ = std::fs::write(c.dir.join("capture.ffxgfx"), stream);
    let mut text = c.lines.join("\n");
    text.push('\n');
    let _ = std::fs::write(c.dir.join("frame.txt"), text);
    (format!("ok {} draws", c.draws), c.reply.take())
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

/// The capture as a scene stream: every object the frame used (textures
/// and buffers read back now), then the frame.
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

fn snapshot(s: &State, c: &mut Capture) -> Vec<u8> {
    let mut out = ffx_gfx::CAPTURE_MAGIC.to_vec();
    // Views pull in their resources.
    let mut objects = c.objects.clone();
    for id in &c.objects {
        if let Some(v) = s.views.get(id) {
            objects.insert(v.resource);
        }
    }
    let mut msgs: Vec<ffx_gfx::Msg> = objects.iter().filter_map(|&id| object_msg(s, id)).collect();
    // Views after their resources.
    msgs.extend(objects.iter().filter_map(|&id| view_msg(s, id)));
    for m in &msgs {
        m.write(&mut out);
    }
    ffx_gfx::Msg::Frame(ffx_gfx::Frame {
        number: 0,
        width: s.bound.viewport[2] as u32,
        height: s.bound.viewport[3] as u32,
        cmds: std::mem::take(&mut c.cmds),
    })
    .write(&mut out);
    out
}
