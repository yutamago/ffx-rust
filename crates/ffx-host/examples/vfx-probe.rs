//! The effect draws' capture (`ffx_host::vfx`), frame by frame: boots the
//! player's `FFX.exe` headless (null device, virtual clock), loads a
//! developer save, switches the capture on and prints each captured frame
//! (every draw's list place, instance key, flags, texture, counts and
//! render-space bounds, the encoded size) with the actors of the snapshot
//! of the same update (field position and render point `(-x, -y, z)`).
//!
//! ```text
//! FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-host \
//!   --example vfx-probe -- [devsave 7] [frames 20] [documents dir]
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ffx_host::exec::{Shared, Stop};

type Snaps = Mutex<BTreeMap<u32, Arc<ffx_state::Snapshot>>>;

fn main() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set");
        std::process::exit(2);
    };
    let mut args = std::env::args().skip(1);
    let save: i32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(7);
    let frames: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(20);
    let documents = args
        .next()
        .map_or_else(|| std::env::temp_dir().join("ffx-vfx-probe"), PathBuf::from);
    std::fs::create_dir_all(&documents).expect("documents folder");
    let opts = ffx_host::Options {
        documents: Some(documents),
        gfx: ffx_host::gfx::Config {
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
    let probe = {
        let shared = Arc::clone(exec.shared());
        std::thread::spawn(move || probe(&shared, &snaps, save, frames))
    };
    let r = exec.run_entry();
    eprintln!("game: {r:?}");
    exec.shared()
        .stop_all(Stop::Milestone("the main thread ended".into()));
    exec.join_threads(Duration::from_secs(2));
    let _ = probe.join();
}

fn wait(what: &str, until: &dyn Fn() -> bool, limit: Duration) {
    let t = Instant::now();
    while !until() {
        assert!(t.elapsed() < limit, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn probe(shared: &Shared, snaps: &Snaps, save: i32, frames: u32) {
    wait(
        "the title",
        &|| shared.clock.frame() >= 30,
        Duration::from_secs(600),
    );
    let reply: Arc<Mutex<Option<String>>> = Arc::default();
    {
        let reply = Arc::clone(&reply);
        shared.requests.push(
            ffx_host::requests::Request::DevSave(save),
            Box::new(move |r| {
                if let Ok(mut x) = reply.lock() {
                    *x = Some(r.unwrap_or_else(|e| format!("err {e}")));
                }
            }),
        );
    }
    let got_reply = || !matches!(reply.lock().as_deref(), Ok(None));
    wait("the save", &got_reply, Duration::from_secs(600));
    eprintln!(
        "devsave {save}: {}",
        reply
            .lock()
            .ok()
            .and_then(|r| r.clone())
            .unwrap_or_default()
    );
    // the map's load and its first frames
    let start = shared.clock.frame() + 120;
    wait(
        "the map",
        &|| shared.clock.frame() >= start,
        Duration::from_secs(900),
    );
    shared.vfx.set_enabled(true);
    let mut got: Vec<Arc<ffx_host::vfx::Frame>> = Vec::new();
    let t = Instant::now();
    while got.len() < frames as usize && t.elapsed() < Duration::from_secs(600) {
        std::thread::sleep(Duration::from_millis(1));
        let Some(f) = shared.vfx.latest() else {
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
    shared.vfx.set_enabled(false);
    for f in &got {
        let snap = snaps.lock().ok().and_then(|m| m.get(&f.update).cloned());
        print_frame(f, snap.as_deref());
    }
    shared.stop_all(Stop::Milestone("probe done".into()));
}

fn print_frame(f: &ffx_host::vfx::Frame, snap: Option<&ffx_state::Snapshot>) {
    let bytes = f.encode();
    let same = ffx_host::vfx::Frame::decode(&bytes).as_ref() == Some(f);
    println!(
        "frame update {} draws {} events {} unresolved {} encoded {} bytes (decodes {})",
        f.update,
        f.draws.len(),
        f.events.len(),
        f.unresolved,
        bytes.len(),
        if same { "the same" } else { "DIFFERENT" },
    );
    for e in &f.events {
        println!("  event kind {} at {} words {:#x?}", e.kind, e.at, e.words);
    }
    for d in &f.draws {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in &d.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let tex = d.texture.rsplit('/').next().unwrap_or("");
        println!(
            "  [{:4}] key ({:#x}, {:#x}){} flags {:#x} seg {} v {} i {} tex {} min {:.1?} max {:.1?}",
            d.place,
            d.key & 0xffff_ffff,
            d.key >> 32,
            if d.pooled { " pooled" } else { "" },
            d.flags,
            d.segment,
            d.positions.len(),
            d.indices.len(),
            tex,
            lo,
            hi,
        );
    }
    let mut dirs: Vec<&str> = f
        .draws
        .iter()
        .map(|d| d.texture.rsplit_once('/').map_or("", |(a, _)| a))
        .collect();
    dirs.sort_unstable();
    dirs.dedup();
    println!("  texture folders: {dirs:?}");
    match snap {
        Some(s) => {
            println!(
                "  snapshot {}: map {} level {:?}",
                s.update, s.world.map, s.world.level
            );
            for a in &s.actors {
                let p = a.position;
                println!(
                    "    actor slot {} {:?} parent {} field ({:.1}, {:.1}, {:.1}) render ({:.1}, {:.1}, {:.1})",
                    a.slot, a.model, a.parent, p[0], p[1], p[2], -p[0], -p[1], p[2]
                );
            }
        }
        None => println!("  no snapshot of update {}", f.update),
    }
}
