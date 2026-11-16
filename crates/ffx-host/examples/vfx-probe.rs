//! The effect draws' capture (`ffx_host::vfx`), frame by frame: boots the
//! player's `FFX.exe` headless (null device, virtual clock), loads a
//! developer save, switches the capture on and prints each captured frame
//! (every draw's list place, instance key, flags, texture, counts and
//! render-space bounds and pass, the draw events decoded, the encoded
//! size, the screen distortion and the field ripple's parameters) with the
//! actors of the snapshot of the same update (field position and render
//! point `(-x, -y, z)`).
//!
//! ```text
//! FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-host \
//!   --example vfx-probe -- [devsave 7] [frames 20] [documents dir] \
//!   [--battle field,group,formation] [--auto] [--skip n] [--summary] \
//!   [--from-save] [--fields] [--drive] [--party hex]
//! ```
//!
//! `--battle` asks for a battle once the map runs and captures from its
//! start (the encounter's transition included); `--auto` switches the
//! battle's auto-battle on (the party fights without input); `--skip`
//! waits `n` more updates before capturing; `--summary` prints only frames
//! with events or draws outside the main pass, and the counts;
//! `--from-save` captures from the save's request on (the map's fade-in);
//! `--fields` prints the battle field table (`MsBtlListField`'s);
//! `--drive` plays the battle: the monsters' HP held at 99999, and at each
//! open command window (`g_BtlWindowOpen`) Yuna summons her first aeon,
//! Lulu casts her first black magic, the others attack (button presses
//! through a replay script, `ffx_host::replay`); Valefor (actor 8) gets a
//! full overdrive gauge and uses it; `--party` writes the battle party
//! (`g_SaveInParty`, 8 bytes in hex) before the battle.

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
    let (cfg, positional) = Config::parse(std::env::args().skip(1));
    let mut args = positional.into_iter();
    let save: i32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(7);
    let frames: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(20);
    let documents = args
        .next()
        .map_or_else(|| std::env::temp_dir().join("ffx-vfx-probe"), PathBuf::from);
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
    let probe = {
        let shared = Arc::clone(exec.shared());
        std::thread::spawn(move || probe(&shared, &snaps, save, frames, &cfg))
    };
    let r = exec.run_entry();
    eprintln!("game: {r:?}");
    exec.shared()
        .stop_all(Stop::Milestone("the main thread ended".into()));
    exec.join_threads(Duration::from_secs(2));
    let _ = probe.join();
}

/// The options after the positional arguments.
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)] // command-line switches
struct Config {
    battle: Option<(u16, u8, u8)>,
    auto: bool,
    skip: u32,
    summary: bool,
    from_save: bool,
    fields: bool,
    drive: bool,
    party: Option<Vec<u8>>,
}

impl Config {
    fn parse(args: impl Iterator<Item = String>) -> (Self, Vec<String>) {
        let mut c = Self::default();
        let mut rest = Vec::new();
        let mut args = args.peekable();
        while let Some(a) = args.next() {
            match a.as_str() {
                "--battle" => {
                    let v: Vec<u32> = args
                        .next()
                        .unwrap_or_default()
                        .split(',')
                        .filter_map(|x| x.parse().ok())
                        .collect();
                    if let [f, g, m] = v[..] {
                        c.battle = Some((
                            u16::try_from(f).unwrap_or(0),
                            u8::try_from(g).unwrap_or(0),
                            u8::try_from(m).unwrap_or(0),
                        ));
                    }
                }
                "--auto" => c.auto = true,
                "--summary" => c.summary = true,
                "--from-save" => c.from_save = true,
                "--fields" => c.fields = true,
                "--drive" => c.drive = true,
                "--party" => {
                    let h = args.next().unwrap_or_default();
                    c.party = (0..h.len() / 2)
                        .map(|k| u8::from_str_radix(&h[2 * k..2 * k + 2], 16).ok())
                        .collect();
                }
                "--skip" => c.skip = args.next().and_then(|x| x.parse().ok()).unwrap_or(0),
                _ => rest.push(a),
            }
        }
        (c, rest)
    }
}

/// `[0x0133d6e0]`: the battle's auto-battle flag (1 on; `forceAutoBattleOff`).
const AUTO_BATTLE: u32 = 0x0133_d6e0;

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
    let got_reply = || !matches!(reply.lock().as_deref(), Ok(None));
    wait(what, &got_reply, Duration::from_secs(900));
    eprintln!(
        "{what}: {}",
        reply
            .lock()
            .ok()
            .and_then(|r| r.clone())
            .unwrap_or_default()
    );
}

fn wait(what: &str, until: &dyn Fn() -> bool, limit: Duration) {
    let t = Instant::now();
    while !until() {
        assert!(t.elapsed() < limit, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The save, the map's first frames, then what `cfg` asks for before the
/// capture: the field table, the party, the battle, the skip.
fn set_up(shared: &Shared, save: i32, cfg: &Config) {
    wait(
        "the title",
        &|| shared.clock.frame() >= 30,
        Duration::from_secs(600),
    );
    if cfg.from_save {
        shared.vfx.set_enabled(true);
    }
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
        Duration::from_secs(900),
    );
    if cfg.fields {
        print_fields(shared);
    }
    if let Some(party) = &cfg.party {
        shared.arena().write_bytes(SAVE_IN_PARTY, party);
    }
    if let Some((field, group, formation)) = cfg.battle {
        // capture from the request on: the encounter's transition
        shared.vfx.set_enabled(true);
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
    if cfg.skip > 0 {
        let start = shared.clock.frame() + cfg.skip;
        // the battle goes on meanwhile
        wait(
            "the skip",
            &|| {
                play(shared, cfg);
                shared.clock.frame() >= start
            },
            Duration::from_secs(1800),
        );
    }
}

fn probe(shared: &Shared, snaps: &Snaps, save: i32, frames: u32, cfg: &Config) {
    set_up(shared, save, cfg);
    shared.vfx.set_enabled(true);
    let mut got: Vec<Arc<ffx_host::vfx::Frame>> = Vec::new();
    let mut seen = 0u32;
    let t = Instant::now();
    while got.len() < frames as usize && t.elapsed() < Duration::from_secs(3600) {
        std::thread::sleep(Duration::from_millis(1));
        play(shared, cfg);
        let Some(f) = shared.vfx.latest() else {
            continue;
        };
        if got.last().is_some_and(|g| g.update == f.update) {
            continue;
        }
        if got.is_empty() {
            print_ripple(shared);
        }
        if cfg.summary {
            // keep memory down: only the frames worth printing in full
            let worth =
                !f.events.is_empty() || f.draws.iter().any(|d| d.pass != ffx_host::vfx::Pass::Main);
            count(&f);
            if worth {
                print_frame(shared.arena(), &f, None);
            }
            // only the update is kept, to tell the next frame
            got.clear();
            got.push(Arc::new(ffx_host::vfx::Frame {
                update: f.update,
                ..Default::default()
            }));
            seen += 1;
            if seen >= frames {
                break;
            }
            continue;
        }
        got.push(f);
    }
    if cfg.summary {
        print_counts();
        shared.vfx.set_enabled(false);
        shared.stop_all(Stop::Milestone("probe done".into()));
        return;
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
        print_frame(shared.arena(), f, snap.as_deref());
    }
    shared.stop_all(Stop::Milestone("probe done".into()));
}

thread_local! {
    static COUNTS: std::cell::RefCell<BTreeMap<String, u64>> = std::cell::RefCell::default();
}

/// Counts a frame into the summary: frames, draws per pass, events per
/// kind, frames with each.
fn count(f: &ffx_host::vfx::Frame) {
    COUNTS.with(|c| {
        let mut c = c.borrow_mut();
        *c.entry("frames".into()).or_default() += 1;
        let mut passes = std::collections::BTreeSet::new();
        for d in &f.draws {
            *c.entry(format!("draws {:?}", d.pass)).or_default() += 1;
            *c.entry(format!("draws flags {:#x} {}", d.flags, blend_text(d)))
                .or_default() += 1;
            passes.insert(d.pass);
        }
        for p in passes {
            *c.entry(format!("frames with {p:?} draws")).or_default() += 1;
        }
        let mut kinds = std::collections::BTreeSet::new();
        for e in &f.events {
            *c.entry(format!("events kind {}", e.kind)).or_default() += 1;
            kinds.insert(e.kind);
        }
        for k in kinds {
            *c.entry(format!("frames with event kind {k}")).or_default() += 1;
        }
        *c.entry("unresolved".into()).or_default() += u64::from(f.unresolved);
    });
}

/// A draw's technique and each shader pass's blend: colour `(src, dest,
/// op)`, alpha when it differs (D3D11's names), the write mask when not
/// all.
fn blend_text(d: &ffx_host::vfx::Draw) -> String {
    use std::fmt::Write as _;
    let Some(t) = d.technique else {
        return "blend ?".into();
    };
    let mut s = format!("{t:?}");
    for b in &d.blends {
        if b.enabled {
            let _ = write!(s, " ({:?}, {:?}, {:?})", b.src, b.dest, b.op);
            if (b.src_alpha, b.dest_alpha, b.op_alpha) != (b.src, b.dest, b.op) {
                let _ = write!(
                    s,
                    " alpha ({:?}, {:?}, {:?})",
                    b.src_alpha, b.dest_alpha, b.op_alpha
                );
            }
        } else {
            s += " (blend off)";
        }
        if b.write_mask != 15 {
            let _ = write!(s, " mask {}", b.write_mask);
        }
    }
    s
}

fn print_counts() {
    COUNTS.with(|c| {
        println!("summary:");
        for (k, v) in c.borrow().iter() {
            println!("  {k}: {v}");
        }
    });
}

/// The character id (`Ch_GetId`: the record's first `u16`) of a character
/// event's instance (`+0x18`), read now.
fn chr_id(arena: &ffx_host::arena::Arena, e: &ffx_host::vfx::Event) -> Option<u16> {
    if !(3..=6).contains(&e.kind) {
        return None;
    }
    let mut b = [0u8; 4];
    let at = e.words[0].checked_add(0x18)?;
    if !arena.is_mapped(at, 4) {
        return None;
    }
    arena.read_bytes(at, &mut b);
    let ch = u32::from_le_bytes(b);
    if ch == 0 || !arena.is_mapped(ch, 2) {
        return None;
    }
    let mut id = [0u8; 2];
    arena.read_bytes(ch, &mut id);
    Some(u16::from_le_bytes(id))
}

fn print_frame(
    arena: &ffx_host::arena::Arena,
    f: &ffx_host::vfx::Frame,
    snap: Option<&ffx_state::Snapshot>,
) {
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
        println!(
            "  event kind {} at {} words {:#x} {:#x} actor {}{} {:?}",
            e.kind,
            e.at,
            e.words[0],
            e.words[1],
            e.actor,
            chr_id(arena, e).map_or_else(String::new, |id| format!(" (id {id:#06x})")),
            e.info()
        );
    }
    print_distortion(f.distortion.as_ref());
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
            "  {:?} [{:4}] key ({:#x}, {:#x}){} flags {:#x} seg {} {} v {} i {} tex {} min {:.1?} max {:.1?}",
            d.pass,
            d.place,
            d.key & 0xffff_ffff,
            d.key >> 32,
            if d.pooled { " pooled" } else { "" },
            d.flags,
            d.segment,
            blend_text(d),
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

/// What `--auto` and `--drive` do at each poll.
fn play(shared: &Shared, cfg: &Config) {
    if cfg.auto {
        shared.arena().write_bytes(AUTO_BATTLE, &1u32.to_le_bytes());
    }
    if cfg.drive {
        drive(shared);
    }
}

/// `g_SaveInParty`: the battle party (`MsGetSaveInParty`).
const SAVE_IN_PARTY: u32 = 0x0113_07e8;

/// `g_BtlWindowOpen`: the bit of the actor whose command window is open.
const WINDOW_OPEN: u32 = 0x023c_c08c;
/// `g_BtlChrTop`'s pointer; actors of 0xf90 bytes, HP `+0x5d0`, max HP
/// `+0x594`, in battle `+0xdc8`.
const BTL_CHR_TOP: u32 = 0x0113_34d4;

/// One step of `--drive`: the monsters' HP held, and the next command
/// pressed when a window is open and no script runs.
fn drive(shared: &Shared) {
    let arena = shared.arena();
    let word = |at: u32| {
        let mut b = [0u8; 4];
        if arena.is_mapped(at, 4) {
            arena.read_bytes(at, &mut b);
        }
        u32::from_le_bytes(b)
    };
    let top = word(BTL_CHR_TOP);
    if top != 0 {
        for k in 0x14..0x1c {
            let a = top + 0xf90 * k;
            if arena.is_mapped(a + 0xdc8, 1) && word(a + 0xdc8) & 0xff != 0 {
                arena.write_bytes(a + 0x594, &99_999u32.to_le_bytes());
                arena.write_bytes(a + 0x5d0, &99_999u32.to_le_bytes());
            }
        }
    }
    // Valefor's overdrive gauge (+0x5bc) full (+0x5bd the max)
    if top != 0 {
        let v = top + 0xf90 * 8;
        if arena.is_mapped(v + 0xdc8, 1) && word(v + 0xdc8) & 0xff != 0 {
            let max = (word(v + 0x5bc) >> 8) & 0xff;
            arena.write_bytes(v + 0x5bc, &[u8::try_from(max).unwrap_or(0)]);
        }
    }
    let busy = shared
        .clock
        .script
        .lock()
        .is_ok_and(|s| s.as_ref().is_some_and(|s| !s.done()));
    let open = word(WINDOW_OPEN);
    if busy || open == 0 {
        return;
    }
    // Yuna (actor 1): Summon (below Attack), the first aeon; Lulu (5): Black
    // Magic, the first spell, its default target; Valefor (8): its
    // overdrive; others: Attack
    let presses = match open.trailing_zeros() {
        1 | 5 => "press down 4 30\npress circle 4 30\npress circle 4 30\npress circle 4 30\n",
        // an aeon: Left opens its overdrive, the first one, its target
        8 => "press left 4 30\npress circle 4 30\npress circle 4 30\npress circle 4 30\n",
        _ => "press circle 4 30\npress circle 4 30\n",
    };
    eprintln!(
        "drive: frame {} window {open:#x}: {}",
        shared.clock.frame(),
        presses.replace('\n', "; ")
    );
    if let Ok(mut s) = shared.clock.script.lock() {
        *s = Some(ffx_host::replay::Script::new(&format!(
            "{presses}wait 60\n"
        )));
    }
}

/// The battle field table (`*0x0112a9c8`, count `0x0112a9d0`; 14-byte
/// entries: map, block offset, first formation, 8 name bytes) and the
/// encounter blocks' group count (`*0x0112a9cc + offset + 1`).
fn print_fields(shared: &Shared) {
    let arena = shared.arena();
    let word = |at: u32| {
        let mut b = [0u8; 4];
        arena.read_bytes(at, &mut b);
        u32::from_le_bytes(b)
    };
    let (table, blocks, n) = (word(0x0112_a9c8), word(0x0112_a9cc), word(0x0112_a9d0));
    if table == 0 || !arena.is_mapped(table, 14 * n.min(0x400) as usize) {
        println!("fields: no table");
        return;
    }
    for i in 0..n.min(0x400) {
        let mut e = [0u8; 14];
        arena.read_bytes(table + 14 * i, &mut e);
        let off = u32::from(u16::from_le_bytes([e[2], e[3]]));
        let mut g = [0u8; 2];
        if blocks != 0 && arena.is_mapped(blocks + off, 2) {
            arena.read_bytes(blocks + off, &mut g);
        }
        let name: String = e[6..14]
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| char::from(c))
            .collect();
        println!(
            "field {i}: map {} first {} name {name} formations {} groups {}",
            i16::from_le_bytes([e[0], e[1]]),
            i16::from_le_bytes([e[4], e[5]]),
            g[0],
            g[1]
        );
    }
}

/// The screen distortion: its flags and alpha, how far the samples move
/// from their vertices (screen fractions), the colour range.
fn print_distortion(d: Option<&ffx_host::vfx::Distortion>) {
    let Some(d) = d else {
        println!("  distortion off");
        return;
    };
    let (mut shift, mut sum) = (0.0f32, 0.0f32);
    for (p, t) in d.positions.iter().zip(&d.uvs) {
        let s = (t[0] - p[0]).hypot(t[1] - p[1]);
        shift = shift.max(s);
        sum += s;
    }
    let (mut lo, mut hi) = ([f32::MAX; 4], [f32::MIN; 4]);
    for c in &d.colours {
        for k in 0..4 {
            lo[k] = lo[k].min(c[k]);
            hi[k] = hi[k].max(c[k]);
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let mean = sum / d.positions.len().max(1) as f32;
    println!(
        "  distortion on{}{} blur {} (copy alpha {}) v {} i {} sample shift max {shift:.4} mean {mean:.4} colours {lo:.3?}..{hi:.3?} centre uv {:.4?}",
        if d.skipped { " skipped" } else { "" },
        if d.reset { " reset" } else { "" },
        d.blur,
        d.copy_alpha(),
        d.positions.len(),
        d.indices.len(),
        d.uvs.get(40),
    );
}

/// The field ripple's parameters (`yiScreff_draw`, `g_YiRender +0xbe078..`,
/// docs/formats/postfx.md "Distortion"): flags, depth, phases, step,
/// speed, amplitude, blur level, tint, frame count.
fn print_ripple(shared: &Shared) {
    let arena = shared.arena();
    let at = 0x0187_6e00 + 0xbe078;
    if !arena.is_mapped(at, 0x34) {
        return;
    }
    let mut raw = [0u8; 0x34];
    arena.read_bytes(at, &mut raw);
    let word = |o: usize| u32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]);
    let float = |o: usize| f32::from_bits(word(o));
    let mut status = [0u8; 4];
    arena.read_bytes(0x018d_ed08, &mut status);
    println!(
        "ripple: status bit 24 {} flags {:#x} depth {} phases {} {} step {} speed {} amplitude {} (+0xbe09c {}) blur level {} tint {:02x?} frames {}",
        u32::from_le_bytes(status) >> 24 & 1,
        word(0),
        float(4),
        float(8),
        float(0xc),
        float(0x18),
        float(0x1c),
        float(0x20),
        float(0x24),
        word(0x28),
        &raw[0x2c..0x2f],
        word(0x30),
    );
}
