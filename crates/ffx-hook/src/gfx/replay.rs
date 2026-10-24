//! Test mode: a captured frame's scene drawn by `ffx-draw` on the game's
//! own device, in place of `PhyreEngine`'s scene draws for one frame
//! (`docs/draw.md`).
//!
//! `gfxreplay <capture.ffxgfx> [out dir]` (harness; Windows paths) loads a
//! capture taken with `gfxcapture`, then, at the game's first scene draw
//! (two render targets and a depth buffer) of the next frame:
//! - the captured scene (clears of its targets, its draws, the buffer
//!   updates and texture copies between them) goes through `ffx-draw`'s
//!   Direct3D 11 backend on the game's device and real immediate context,
//!   into the game's own scene targets; its inputs (shaders from their
//!   DXBC, layouts, states, buffers, textures) are made from the capture;
//! - the game's own scene draws, and its clears of the scene targets, are
//!   skipped for the rest of that frame;
//! - the context's state is put back as the game had it;
//! - each scene target is read back and compared byte for byte with the
//!   capture's dumps of it, and each replayed draw's state, read back from
//!   the context, with the captured draw (`ffx_draw::replay::compare_draws`;
//!   the replayed stream is also written as `replay.ffxgfx` for
//!   `ffx-drawdiff`).
//!
//! Off, it costs nothing: the proxy context's vtable holds the layer's own
//! entries. Only while a replay is pending are six of them (the four draws
//! and the two clears) pointed here, and they go back once it is done.

use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};

use ffx_draw::Backend as _;
use ffx_draw::d3d11::D3d11;
use ffx_draw::desc::ViewKind;
use ffx_draw::replay::{Capture, Replayer, Selection, compare_draws, draw_call, select_scene};
use ffx_gfx::{Cmd, Id};

use super::{proxy, state, vt};
use crate::log::log;

type Ptr = *mut c_void;

/// A render target dump the capture took (`frame.txt`): after draw
/// `after` (1-based) of the frame, of view `view`.
struct Dump {
    after: u32,
    name: String,
    view: Id,
    pitch: u32,
    rows: u32,
}

struct Pending {
    cap: Capture,
    sel: Selection,
    dumps: Vec<Dump>,
    capture_dir: PathBuf,
    out: PathBuf,
    reply: Option<Sender<String>>,
    /// Replay at the first scene draw of a frame after this one.
    armed_at: u32,
    /// The frame it was replayed in, and the game's scene views then.
    done: Option<(u32, [usize; 3])>,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// Proxy vtable slots taken over while a replay is pending, and what they
/// held.
const SLOTS: [usize; 6] = [
    vt::context::DRAW_INDEXED,
    vt::context::DRAW,
    vt::context::DRAW_INDEXED_INSTANCED,
    vt::context::DRAW_INSTANCED,
    vt::context::CLEAR_RENDER_TARGET_VIEW,
    vt::context::CLEAR_DEPTH_STENCIL_VIEW,
];
static PREVIOUS: [AtomicUsize; 6] = [const { AtomicUsize::new(0) }; 6];

fn ours() -> [usize; 6] {
    [
        draw_indexed as *const () as usize,
        draw as *const () as usize,
        draw_indexed_instanced as *const () as usize,
        draw_instanced as *const () as usize,
        clear_rtv as *const () as usize,
        clear_dsv as *const () as usize,
    ]
}

/// Points the six slots here.
fn install() {
    let table = proxy::vtable();
    for (i, (&slot, f)) in SLOTS.iter().zip(ours()).enumerate() {
        // SAFETY: our own proxy vtable (proxy::SLOTS entries, writable);
        // an aligned word store is atomic on x86.
        unsafe {
            let at = table.add(slot);
            let now = std::ptr::read_volatile(at);
            if now != f {
                PREVIOUS[i].store(now, Ordering::Release);
                std::ptr::write_volatile(at, f);
            }
        }
    }
}

/// Puts the layer's entries back.
fn uninstall() {
    let table = proxy::vtable();
    for (i, (&slot, f)) in SLOTS.iter().zip(ours()).enumerate() {
        let prev = PREVIOUS[i].load(Ordering::Acquire);
        // SAFETY: as in `install`.
        unsafe {
            let at = table.add(slot);
            if prev != 0 && std::ptr::read_volatile(at) == f {
                std::ptr::write_volatile(at, prev);
            }
        }
    }
}

fn previous(i: usize) -> usize {
    PREVIOUS[i].load(Ordering::Acquire)
}

/// `gfxreplay <capture.ffxgfx> [out dir]`: replays the capture's scene in
/// the next frame; the reply sums up the comparison.
pub fn request(args: &str) -> String {
    if !super::ENABLED.load(Ordering::Acquire) || super::DEVICE.load(Ordering::Acquire) == 0 {
        return "err the graphics layer is off (FFX_GFX=1)".into();
    }
    let words: Vec<&str> = args.split_whitespace().collect();
    let Some(&file) = words.first() else {
        return "err usage: gfxreplay <capture.ffxgfx> [out dir]".into();
    };
    let file = PathBuf::from(file);
    let capture_dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    let out = words
        .get(1)
        .map_or_else(|| capture_dir.join("replay"), PathBuf::from);
    let bytes = match std::fs::read(&file) {
        Ok(b) => b,
        Err(e) => return format!("err {}: {e}", file.display()),
    };
    let cap = match Capture::parse(&bytes) {
        Ok(c) => c,
        Err(e) => return format!("err {e}"),
    };
    let Some(sel) = select_scene(&cap.frame) else {
        return "err the capture has no scene draws".into();
    };
    let dumps = std::fs::read_to_string(capture_dir.join("frame.txt"))
        .map(|t| parse_dumps(&t))
        .unwrap_or_default();
    if let Err(e) = std::fs::create_dir_all(&out) {
        return format!("err {}: {e}", out.display());
    }
    let (tx, rx) = channel();
    {
        let Ok(mut p) = PENDING.lock() else {
            return "err replay lock".into();
        };
        if p.is_some() {
            return "err a replay is pending".into();
        }
        *p = Some(Pending {
            cap,
            sel,
            dumps,
            capture_dir,
            out,
            reply: Some(tx),
            armed_at: super::stats().0,
            done: None,
        });
    }
    install();
    match rx.recv_timeout(std::time::Duration::from_secs(120)) {
        Ok(r) => r,
        Err(_) => {
            if let Ok(mut p) = PENDING.lock() {
                *p = None;
            }
            uninstall();
            "err no scene draw within 120 s".into()
        }
    }
}

/// The `dump` lines of a capture's `frame.txt`, each with the draw it
/// followed.
fn parse_dumps(text: &str) -> Vec<Dump> {
    let mut out = Vec::new();
    let mut after = 0u32;
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.as_slice() {
            ["draw", n, ..] => after = n.parse().unwrap_or(after),
            [
                "dump",
                name,
                _size,
                "format",
                _f,
                "pitch",
                pitch,
                "rows",
                rows,
            ] => {
                // rt/<n>_<c|d>_<view hex>
                let view = name
                    .rsplit('_')
                    .next()
                    .and_then(|v| u32::from_str_radix(v, 16).ok());
                if let (Some(view), Ok(pitch), Ok(rows)) = (view, pitch.parse(), rows.parse()) {
                    out.push(Dump {
                        after,
                        name: (*name).to_owned(),
                        view,
                        pitch,
                        rows,
                    });
                }
            }
            _ => {}
        }
    }
    out
}

/// Whether the game's call is to be skipped; runs the replay at the first
/// scene draw of the armed frame. Never panics (it runs inside the
/// game's draw calls).
fn on_draw(this: Ptr) -> bool {
    let Ok(mut guard) = PENDING.lock() else {
        return false;
    };
    let Some(p) = guard.as_mut() else {
        return false;
    };
    let frame = super::stats().0;
    if let Some((f, _)) = p.done
        && f != frame
    {
        // The frame is over: the game draws its own again.
        *guard = None;
        drop(guard);
        uninstall();
        return false;
    }
    if frame <= p.armed_at {
        return false;
    }
    let (rtv, dsv) = {
        let s = state();
        (s.bound.rtv, s.bound.dsv)
    };
    if rtv[0] == 0 || rtv[1] == 0 || rtv[2] != 0 || dsv == 0 {
        return false;
    }
    let live = [rtv[0], rtv[1], dsv];
    match p.done {
        Some((_, views)) => views == live,
        None => {
            let reply = replay(p, proxy::real(this), live);
            log(&format!("  gfx: replay: {reply}"));
            p.done = Some((frame, live));
            if let Some(tx) = p.reply.take() {
                let _ = tx.send(reply);
            }
            true
        }
    }
}

/// Whether a clear of the game's is to be skipped (one of the scene's
/// targets, after the replay drew them).
fn on_clear(view: usize) -> bool {
    let Ok(guard) = PENDING.lock() else {
        return false;
    };
    guard.as_ref().is_some_and(|p| {
        p.done
            .is_some_and(|(f, views)| f == super::stats().0 && views.contains(&view))
    })
}

/// Bytes per texel of the formats the scene targets use, for comparing
/// rows without their padding.
fn texel_bytes(format: u32) -> Option<u32> {
    match format {
        2 => Some(16),
        10 | 16 | 19..=21 => Some(8),
        24 | 26 | 28 | 29 | 39..=41 | 44..=47 | 87 | 91 => Some(4),
        _ => None,
    }
}

/// Byte-for-byte comparison of two images of `width`×`rows`; a line for
/// the report.
fn compare_image(
    a: &[u8],
    a_pitch: u32,
    b: &[u8],
    b_pitch: u32,
    width: u32,
    rows: u32,
    format: u32,
) -> (bool, String) {
    let row = texel_bytes(format).map_or(a_pitch.min(b_pitch), |t| t * width) as usize;
    let texel = texel_bytes(format).unwrap_or(1) as usize;
    let mut differ = 0usize;
    let mut first = None;
    let mut worst = 0u8;
    for y in 0..rows as usize {
        let (fa, fb) = (y * a_pitch as usize, y * b_pitch as usize);
        let (Some(ra), Some(rb)) = (a.get(fa..fa + row), b.get(fb..fb + row)) else {
            return (false, format!("short image (row {y})"));
        };
        if ra == rb {
            continue;
        }
        for (x, (ta, tb)) in ra.chunks(texel).zip(rb.chunks(texel)).enumerate() {
            if ta != tb {
                differ += 1;
                first.get_or_insert((x, y));
                for (u, v) in ta.iter().zip(tb) {
                    worst = worst.max(u.abs_diff(*v));
                }
            }
        }
    }
    match first {
        None => (true, "identical".into()),
        Some((x, y)) => (
            false,
            format!("{differ} texels differ, first at {x},{y}, largest byte difference {worst}"),
        ),
    }
}

/// The replay itself, on the render thread inside the game's first scene
/// draw of the frame. Returns the harness reply.
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)] // replay, read back, compare, report; p(ending), b(ackend), r(eplayer)
fn replay(p: &mut Pending, context: Ptr, live: [usize; 3]) -> String {
    let t0 = std::time::Instant::now();
    let device = super::DEVICE.load(Ordering::Acquire) as Ptr;
    // SAFETY: the game's device and its real immediate context, on the
    // render thread (inside a context call).
    let Some(mut b) = (unsafe { D3d11::borrow(device, context) }) else {
        return "err no device".into();
    };
    let cap = &p.cap;
    let sel = &p.sel;
    let targets = [sel.targets.color, sel.targets.normal, sel.targets.depth];
    let kinds = [
        ViewKind::RenderTarget,
        ViewKind::RenderTarget,
        ViewKind::DepthStencil,
    ];
    let mut r = Replayer::new();
    let mut imported = Vec::new();
    for ((&id, &view), kind) in targets.iter().zip(&live).zip(kinds) {
        // SAFETY: the game's bound scene view of that kind.
        let Some((v, t)) = (unsafe { b.import_view(view as Ptr, kind) }) else {
            return format!("err cannot use the game's view {view:#x}");
        };
        let want = cap.view_texture_desc(id);
        let have = b.texture_desc(t);
        if want.map(|d| (d.width, d.height, d.format))
            != have.map(|d| (d.width, d.height, d.format))
        {
            return format!(
                "err the game's scene target {view:#x} is {have:?}, the capture's {id:#x} {want:?}"
            );
        }
        r.import(id, v, cap.view_resource(id).map(|rid| (rid, t)));
        imported.push((id, t));
    }
    let texture_of: HashMap<Id, ffx_draw::backend::Texture> = imported.iter().copied().collect();
    // Draw numbers of the frame's draws, by command index.
    let mut numbers = HashMap::new();
    let mut n = 0u32;
    for (i, c) in cap.frame.cmds.iter().enumerate() {
        if matches!(c, Cmd::Draw(_)) {
            n += 1;
            numbers.insert(i, n);
        }
    }
    let mut report = String::new();
    let mut stream = Vec::new();
    let mut stream_diffs = 0usize;
    let mut dumps_ok = 0usize;
    let mut dumps_bad = 0usize;
    let mut per_target: Vec<String> = Vec::new();
    b.begin();
    r.run(&mut b, cap, &sel.cmds, |r, b, i, _| {
        let Some(c) = cap.frame.cmds.get(i) else {
            return;
        };
        let Cmd::Draw(d) = c else {
            stream.push(c.clone());
            return;
        };
        let seen = b.bound();
        let replayed = r.captured_draw(&seen, draw_call(d), |buf| {
            b.written(buf).map(<[u8]>::to_vec).unwrap_or_default()
        });
        let diffs = compare_draws(cap, d, &replayed);
        let number = numbers.get(&i).copied().unwrap_or(0);
        if !diffs.is_empty() {
            stream_diffs += 1;
            let _ = writeln!(report, "draw {number}: {}", diffs.join("; "));
        }
        stream.push(Cmd::Draw(Box::new(replayed)));
        // The capture's dumps after this draw, of the scene's targets.
        for dump in p.dumps.iter().filter(|x| x.after == number) {
            let Some(&tex) = texture_of.get(&dump.view) else {
                continue;
            };
            let line = match (
                b.read_texture(tex),
                std::fs::read(p.capture_dir.join(format!("{}.raw", dump.name))),
            ) {
                (Ok(rb), Ok(orig)) => {
                    let _ = std::fs::write(
                        p.out
                            .join(format!("{}.replayed.raw", dump.name.replace('/', "_"))),
                        &rb.data,
                    );
                    let (same, text) = compare_image(
                        &orig,
                        dump.pitch,
                        &rb.data,
                        rb.row_pitch,
                        rb.width,
                        dump.rows.min(rb.rows),
                        rb.format,
                    );
                    if same {
                        dumps_ok += 1;
                    } else {
                        dumps_bad += 1;
                    }
                    format!(
                        "{} (after draw {number}, {}x{} format {}): {text}",
                        dump.name, rb.width, rb.height, rb.format
                    )
                }
                (Err(e), _) => format!("{}: read back failed: {e}", dump.name),
                (_, Err(e)) => format!("{}: no dump file: {e}", dump.name),
            };
            let _ = writeln!(report, "{line}");
            per_target.push(line);
        }
    });
    // At the end of the scene: every target against the capture's copy of
    // its texture as the frame ended (right for targets nothing draws
    // into after the scene).
    for (name, id) in [
        ("color", sel.targets.color),
        ("normal", sel.targets.normal),
        ("depth", sel.targets.depth),
    ] {
        let Some(&tex) = texture_of.get(&id) else {
            continue;
        };
        let line = match (b.read_texture(tex), cap.view_texture_data(id)) {
            (Ok(rb), Some(orig)) => {
                let _ = std::fs::write(p.out.join(format!("scene_{name}.raw")), &rb.data);
                let (_, text) = compare_image(
                    &orig.data,
                    orig.row_pitch,
                    &rb.data,
                    rb.row_pitch,
                    rb.width,
                    rb.rows,
                    rb.format,
                );
                format!(
                    "{name} {id:#x} at the scene's end, against the capture's texture at the frame's end: {text}"
                )
            }
            (Err(e), _) => format!("{name}: read back failed: {e}"),
            (_, None) => format!("{name}: no texture contents in the capture"),
        };
        let _ = writeln!(report, "{line}");
        per_target.push(line);
    }
    b.end();
    let spent = t0.elapsed();
    let _ = std::fs::write(
        p.out.join("replay.ffxgfx"),
        ffx_draw::replay::write_capture(cap, stream),
    );
    let mut gaps = String::new();
    for (what, n) in &r.gaps.notes {
        let _ = writeln!(gaps, "gap: {what} (x{n})");
    }
    let counts = r.cache.counts();
    let summary = format!(
        "ok replayed {} scene draws ({} commands, {} other draws inside left out) in {:.0} ms; \
         made {} textures, {} views, {} buffers, {} shaders, {} layouts, {} states, {} constant buffers; \
         dumps {dumps_ok} identical, {dumps_bad} differ; stream: {stream_diffs} draws differ; {} gaps",
        sel.draws,
        sel.cmds.len(),
        sel.other_draws,
        spent.as_secs_f64() * 1000.0,
        r.made[0],
        r.made[1],
        r.made[2],
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        r.gaps.notes.len(),
    );
    let _ = std::fs::write(
        p.out.join("replay.txt"),
        format!("{summary}\n{gaps}{report}"),
    );
    // One line: the harness reads one reply line per command.
    format!("{summary}; {}", per_target.join("; "))
}

unsafe extern "system" fn draw_indexed(this: Ptr, count: u32, start: u32, base: i32) {
    if on_draw(this) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, i32);
    // SAFETY: the layer's entry this slot held, same arguments.
    unsafe {
        let f: F = std::mem::transmute(previous(0));
        f(this, count, start, base);
    }
}

unsafe extern "system" fn draw(this: Ptr, count: u32, start: u32) {
    if on_draw(this) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(1));
        f(this, count, start);
    }
}

unsafe extern "system" fn draw_indexed_instanced(
    this: Ptr,
    count: u32,
    instances: u32,
    start: u32,
    base: i32,
    first: u32,
) {
    if on_draw(this) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, u32, i32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(2));
        f(this, count, instances, start, base, first);
    }
}

unsafe extern "system" fn draw_instanced(
    this: Ptr,
    count: u32,
    instances: u32,
    start: u32,
    first: u32,
) {
    if on_draw(this) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(3));
        f(this, count, instances, start, first);
    }
}

unsafe extern "system" fn clear_rtv(this: Ptr, view: Ptr, color: *const f32) {
    if on_clear(view as usize) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, *const f32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(4));
        f(this, view, color);
    }
}

unsafe extern "system" fn clear_dsv(this: Ptr, view: Ptr, flags: u32, depth: f32, stencil: u8) {
    if on_clear(view as usize) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, f32, u8);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(5));
        f(this, view, flags, depth, stencil);
    }
}
