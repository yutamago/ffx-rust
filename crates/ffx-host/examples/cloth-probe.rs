//! The cloth capture (`ffx_host::cloth`), frame by frame: boots the
//! player's `FFX.exe` headless (null device, virtual clock), loads a
//! developer save, switches the capture on and prints per captured frame
//! each character's cloth (actor slot, model, segments with their model,
//! piece, vertex counts, simulated vertices, render-space bounds and the
//! distance of their centre to the actor's render point) and how far the
//! vertices moved since the previous frame, in render space and relative
//! to the actor; then the counts and the encoded size.
//!
//! ```text
//! FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-host \
//!   --example cloth-probe -- [devsave 7] [frames 20] [documents dir] \
//!   [--skip n] [--party hex] [--battle field,group,formation]
//! FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-host \
//!   --example cloth-probe -- 12 6 --party 01000405ff020306 --battle 34,0,0 --skip 300
//! ```
//!
//! `--skip` waits `n` more updates before capturing; `--party` writes the
//! battle party (`g_SaveInParty`, 8 bytes in hex) and `--battle` asks for
//! a battle once the map runs (the second line: Tidus, Yuna and Wakka, Lulu
//! the reserve, against the first formation of battle field 34). The probe ends
//! with exit code 1 when no frame had a simulated vertex, none moved
//! relative to its actor, or a frame did not decode to itself.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ffx_host::cloth::{Character, Frame, Segment};
use ffx_host::exec::{Shared, Stop};

type Snaps = Mutex<BTreeMap<u32, Arc<ffx_state::Snapshot>>>;

fn main() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set");
        std::process::exit(2);
    };
    let mut cfg = Config::default();
    let mut positional = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--skip" => cfg.skip = args.next().and_then(|x| x.parse().ok()).unwrap_or(0),
            "--party" => {
                let h = args.next().unwrap_or_default();
                cfg.party = (0..h.len() / 2)
                    .map(|k| u8::from_str_radix(&h[2 * k..2 * k + 2], 16).ok())
                    .collect();
            }
            "--battle" => {
                let v: Vec<u32> = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|x| x.parse().ok())
                    .collect();
                if let [f, g, m] = v[..] {
                    cfg.battle = Some((
                        u16::try_from(f).unwrap_or(0),
                        u8::try_from(g).unwrap_or(0),
                        u8::try_from(m).unwrap_or(0),
                    ));
                }
            }
            _ => positional.push(a),
        }
    }
    let mut positional = positional.into_iter();
    let save: i32 = positional.next().and_then(|a| a.parse().ok()).unwrap_or(7);
    let frames: u32 = positional.next().and_then(|a| a.parse().ok()).unwrap_or(20);
    let documents = positional.next().map_or_else(
        || std::env::temp_dir().join("ffx-cloth-probe"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&documents).expect("documents folder");
    let opts = ffx_host::Options {
        documents: Some(documents),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        skip_opening: true,
        ..ffx_host::Options::default()
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).expect("boot");
    let snaps: Arc<Snaps> = Arc::default();
    {
        let snaps = Arc::clone(&snaps);
        exec.shared().observers.add(Arc::new(move |s| {
            if let Ok(mut m) = snaps.lock() {
                m.insert(s.update, Arc::clone(s));
                while m.len() > 256 {
                    m.pop_first();
                }
            }
        }));
    }
    let ok = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let probe = {
        let shared = Arc::clone(exec.shared());
        let ok = Arc::clone(&ok);
        std::thread::spawn(move || {
            let good = probe(&shared, &snaps, save, frames, &cfg);
            ok.store(good, std::sync::atomic::Ordering::Release);
        })
    };
    let r = exec.run_entry();
    eprintln!("game: {r:?}");
    exec.shared()
        .stop_all(Stop::Milestone("the main thread ended".into()));
    exec.join_threads(Duration::from_secs(2));
    let _ = probe.join();
    if !ok.load(std::sync::atomic::Ordering::Acquire) {
        std::process::exit(1);
    }
}

/// The options after the positional arguments.
#[derive(Debug, Default)]
struct Config {
    skip: u32,
    party: Option<Vec<u8>>,
    battle: Option<(u16, u8, u8)>,
}

/// `g_SaveInParty`: the battle party (`MsGetSaveInParty`).
const SAVE_IN_PARTY: u32 = 0x0113_07e8;

fn wait(what: &str, until: &dyn Fn() -> bool, limit: Duration) {
    let t = Instant::now();
    while !until() {
        assert!(t.elapsed() < limit, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn ask(shared: &Shared, what: &str, request: ffx_host::requests::Request) {
    let reply: Arc<Mutex<Option<String>>> = Arc::default();
    {
        let reply = Arc::clone(&reply);
        shared.requests.push(
            request,
            Box::new(move |r| {
                if let Ok(mut x) = reply.lock() {
                    *x = Some(r.unwrap_or_else(|e| format!("err {e}")));
                }
            }),
        );
    }
    wait(
        what,
        &|| !matches!(reply.lock().as_deref(), Ok(None)),
        Duration::from_secs(900),
    );
    eprintln!(
        "{what}: {}",
        reply
            .lock()
            .ok()
            .and_then(|r| r.clone())
            .unwrap_or_default()
    );
}

/// Captures `frames` frames and prints them; true when a frame had
/// simulated vertices and some moved between frames.
fn probe(shared: &Shared, snaps: &Snaps, save: i32, frames: u32, cfg: &Config) -> bool {
    wait(
        "the title",
        &|| shared.clock.frame() >= 30,
        Duration::from_secs(600),
    );
    ask(
        shared,
        &format!("devsave {save}"),
        ffx_host::requests::Request::DevSave(save),
    );
    // the map's load and its first frames
    let start = shared.clock.frame() + 120;
    wait(
        "the map",
        &|| shared.clock.frame() >= start,
        Duration::from_secs(1800),
    );
    if let Some(party) = &cfg.party {
        shared.arena().write_bytes(SAVE_IN_PARTY, party);
    }
    if let Some((field, group, formation)) = cfg.battle {
        ask(
            shared,
            &format!("battle {field},{group},{formation}"),
            ffx_host::requests::Request::Battle {
                field,
                group,
                formation,
            },
        );
    }
    let start = shared.clock.frame() + cfg.skip;
    wait(
        "the skip",
        &|| shared.clock.frame() >= start,
        Duration::from_secs(1800),
    );
    shared.cloth.set_enabled(true);
    let mut got: Vec<Arc<Frame>> = Vec::new();
    let t = Instant::now();
    while got.len() < frames as usize && t.elapsed() < Duration::from_secs(1800) {
        std::thread::sleep(Duration::from_millis(1));
        let Some(f) = shared.cloth.latest() else {
            continue;
        };
        if got.last().is_some_and(|g| g.update == f.update) {
            continue;
        }
        got.push(f);
    }
    // the snapshot of update n comes at the boundary after the frame
    let last = got.last().map_or(0, |f| f.update);
    wait(
        "the last snapshot",
        &|| shared.clock.frame() > last,
        Duration::from_secs(60),
    );
    shared.cloth.set_enabled(false);
    let mut totals = Totals::default();
    let mut prev: Option<(&Frame, Option<Arc<ffx_state::Snapshot>>)> = None;
    for f in &got {
        let snap = snaps.lock().ok().and_then(|m| m.get(&f.update).cloned());
        print_frame(f, snap.as_deref(), prev.as_ref(), &mut totals);
        prev = Some((f, snap));
    }
    eprintln!(
        "summary: {} frames, {} with cloth; {} character frames, {} segment frames; {} vertices, {} simulated (weight > 0); moved: {} of {} simulated vertices between frames (max {:.4} render units), {} relative to the actor; decoded equal: {}/{}; largest blob {} bytes",
        got.len(),
        totals.frames_with_cloth,
        totals.characters,
        totals.segments,
        totals.vertices,
        totals.simulated,
        totals.moved,
        totals.compared,
        totals.max_move,
        totals.moved_relative,
        totals.decoded,
        got.len(),
        totals.largest_blob,
    );
    shared.stop_all(Stop::Milestone("probe done".into()));
    totals.simulated > 0 && totals.moved_relative > 0 && totals.decoded == got.len()
}

#[derive(Default)]
struct Totals {
    frames_with_cloth: usize,
    characters: usize,
    segments: usize,
    vertices: usize,
    simulated: usize,
    compared: usize,
    moved: usize,
    moved_relative: usize,
    max_move: f32,
    decoded: usize,
    largest_blob: usize,
}

fn render_point(a: &ffx_state::Actor) -> [f32; 3] {
    [-a.position[0], -a.position[1], a.position[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn len(a: [f32; 3]) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// `local` placed by `ClothWorld` read the other way round (column
/// vectors), for comparison with `cloth::to_render`.
fn to_render_columns(w: &[f32; 16], v: [f32; 4]) -> [f32; 3] {
    std::array::from_fn(|k| {
        w[4 * k] * v[0] + w[4 * k + 1] * v[1] + w[4 * k + 2] * v[2] + w[4 * k + 3]
    })
}

fn print_frame(
    f: &Frame,
    snap: Option<&ffx_state::Snapshot>,
    prev: Option<&(&Frame, Option<Arc<ffx_state::Snapshot>>)>,
    t: &mut Totals,
) {
    let blob = f.encode();
    t.largest_blob = t.largest_blob.max(blob.len());
    if Frame::decode(&blob).as_ref() == Some(f) {
        t.decoded += 1;
    }
    println!(
        "update {}: {} characters, {} bytes",
        f.update,
        f.characters.len(),
        blob.len()
    );
    if !f.characters.is_empty() {
        t.frames_with_cloth += 1;
    }
    for c in &f.characters {
        t.characters += 1;
        let actor = snap.and_then(|s| {
            s.actors
                .iter()
                .find(|a| i32::from(a.slot) == i32::from(c.actor))
        });
        let at = actor.map(render_point);
        println!(
            "  actor {} id {:#06x} model {} table slot {} at {:?} drawn {}: {} segments",
            c.actor,
            c.id,
            actor.map_or("?", |a| a.model.as_str()),
            c.table_slot,
            at.map(|p| p.map(|x| (x * 100.0).round() / 100.0)),
            actor.is_some_and(|a| a.drawn),
            c.segments.len()
        );
        let before = prev.and_then(|(pf, ps)| {
            let pc = pf.characters.iter().find(|x| x.actor == c.actor)?;
            let pa = ps
                .as_ref()
                .and_then(|s| {
                    s.actors
                        .iter()
                        .find(|a| i32::from(a.slot) == i32::from(c.actor))
                })
                .map(render_point);
            Some((pc, pa))
        });
        for s in &c.segments {
            print_segment(s, at, before, t);
        }
    }
}

/// One segment's line: counts, bounds, the distance of the simulated
/// vertices' centre to the actor (both matrix readings) and the movement
/// against `before` (the previous frame's character and actor point).
fn print_segment(
    seg: &Segment,
    at: Option<[f32; 3]>,
    before: Option<(&Character, Option<[f32; 3]>)>,
    totals: &mut Totals,
) {
    totals.segments += 1;
    totals.vertices += seg.local.len();
    let sim: Vec<usize> = (0..seg.local.len())
        .filter(|&k| seg.weights[k] > 0.0)
        .collect();
    totals.simulated += sim.len();
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    let mut centre = [0.0f32; 3];
    let mut centre_cols = [0.0f32; 3];
    let count = f32::from(u16::try_from(sim.len()).unwrap_or(u16::MAX));
    for &k in &sim {
        let p = seg.positions[k];
        let q = to_render_columns(&seg.world, seg.local[k]);
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
            centre[i] += p[i] / count;
            centre_cols[i] += q[i] / count;
        }
    }
    let (dist, dist_cols) = at.map_or((f32::NAN, f32::NAN), |a| {
        (len(sub(centre, a)), len(sub(centre_cols, a)))
    });
    let moved = before
        .and_then(|(pc, pa)| {
            let prev = pc
                .segments
                .iter()
                .find(|x| (x.instance, x.segment) == (seg.instance, seg.segment))
                .filter(|x| x.positions.len() == seg.positions.len())?;
            let shift = match (at, pa) {
                (Some(a), Some(b)) => sub(a, b),
                _ => [0.0; 3],
            };
            Some(movement(seg, prev, &sim, shift, totals))
        })
        .unwrap_or_default();
    let round = |v: [f32; 3]| v.map(|x| (x * 100.0).round() / 100.0);
    println!(
        "    inst {} seg {} model {} piece {}: {} vertices (segment {}), {} simulated, bounds {:?}..{:?}, centre {dist:.2} from the actor (columns: {dist_cols:.2}){moved}",
        seg.instance,
        seg.segment,
        seg.model,
        seg.piece,
        seg.local.len(),
        seg.segment_vertices,
        sim.len(),
        round(lo),
        round(hi),
    );
}

/// How the simulated vertices `sim` of `seg` moved since `prev`, in render
/// space and less the actor's own move `shift`, into `totals`.
fn movement(
    seg: &Segment,
    prev: &Segment,
    sim: &[usize],
    shift: [f32; 3],
    totals: &mut Totals,
) -> String {
    let (mut max, mut max_rel) = (0.0f32, 0.0f32);
    let (mut moved, mut moved_rel) = (0, 0);
    for &k in sim {
        if prev.weights[k] == 0.0 {
            continue;
        }
        totals.compared += 1;
        let step = sub(seg.positions[k], prev.positions[k]);
        let (abs, rel) = (len(step), len(sub(step, shift)));
        max = max.max(abs);
        max_rel = max_rel.max(rel);
        moved += usize::from(abs > 1e-5);
        moved_rel += usize::from(rel > 1e-4);
    }
    totals.moved += moved;
    totals.moved_relative += moved_rel;
    totals.max_move = totals.max_move.max(max);
    format!("; moved {moved} (max {max:.4}), relative to the actor {moved_rel} (max {max_rel:.4})")
}
