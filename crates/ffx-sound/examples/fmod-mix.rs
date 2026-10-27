//! Mixes an FMOD call log offline and writes FMOD's WAV format (48 kHz
//! stereo float), to compare with the capture the log came with
//! (`tools/wav-compare.py`; `docs/formats/audio-platform.md`, "The offline
//! engine").
//!
//! ```bash
//! FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-sound \
//!   --example fmod-mix -- <log> <out.wav>
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use ffx_sound::mixer::{BLOCK, RATE};
use ffx_sound::replay::{Call, Engine, Entry, Files, parse};

struct Vbf {
    archive: ffx_vbf::Archive,
    cache: HashMap<String, Option<Arc<Vec<u8>>>>,
}

impl Files for Vbf {
    fn read(&mut self, path: &str) -> Option<Arc<Vec<u8>>> {
        if let Some(b) = self.cache.get(path) {
            return b.clone();
        }
        let full = format!("ffx_data/gamedata/ps3data/sound_pc/{path}");
        let b = self
            .archive
            .find(&full)
            .and_then(|e| self.archive.read(e).ok())
            .map(Arc::new);
        if b.is_none() {
            eprintln!("missing: {full}");
        }
        self.cache.insert(path.to_owned(), b.clone());
        b
    }
}

fn wav(frames: &[f32]) -> Vec<u8> {
    let data = u32::try_from(frames.len() * 4).unwrap_or(u32::MAX);
    let mut w = Vec::with_capacity(44 + frames.len() * 4);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&3u16.to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&RATE.to_le_bytes());
    w.extend_from_slice(&(RATE * 8).to_le_bytes());
    w.extend_from_slice(&8u16.to_le_bytes());
    w.extend_from_slice(&32u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data.to_le_bytes());
    for s in frames {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}

/// An `fmod-probe` script as log entries (`--script <dir below Sound_PC/>`):
/// the same calls the probe makes, on made-up handles.
fn script(text: &str, dir: &str) -> Vec<Entry> {
    let mut handles: HashMap<String, u32> = HashMap::new();
    let mut next = 0x100u32;
    let mut out = Vec::new();
    let mut h = |name: &str, handles: &mut HashMap<String, u32>| {
        *handles.entry(name.to_owned()).or_insert_with(|| {
            next += 0x100;
            next
        })
    };
    let f32w = |s: &str| s.parse::<f32>().map_or(0, f32::to_bits);
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.is_empty() || w[0].starts_with('#') {
            continue;
        }
        let mut c = Call::default();
        match w[0] {
            "load" => {
                c.function = "EventSystem::load".into();
                c.strings = vec![(1, format!("Sound_PC/{dir}/{}", w[2]))];
                c.outs = vec![(3, h(w[1], &mut handles))];
            }
            "group" => {
                c.function = "EventProjectI::getGroup".into();
                c.args = vec![h(w[2], &mut handles)];
                c.strings = vec![(1, w[3].to_owned())];
                c.outs = vec![(3, h(w[1], &mut handles))];
            }
            "event" => {
                c.function = "EventGroupI::getEventByIndex".into();
                c.args = vec![h(w[2], &mut handles), w[3].parse().unwrap_or(0)];
                c.outs = vec![(3, h(w[1], &mut handles))];
            }
            "start" => {
                c.function = "Event::start".into();
                c.args = vec![h(w[1], &mut handles)];
            }
            "stop" | "pause" => {
                c.function = if w[0] == "stop" {
                    "Event::stop"
                } else {
                    "Event::setPaused"
                }
                .into();
                c.args = vec![h(w[1], &mut handles), w[2].parse().unwrap_or(0)];
            }
            "vol" => {
                c.function = "Event::setVolume".into();
                c.args = vec![h(w[1], &mut handles), f32w(w[2])];
            }
            "update" => {
                let n: u64 = w.get(1).and_then(|n| n.parse().ok()).unwrap_or(1);
                for _ in 0..n {
                    out.push(Entry::Update { n: 0, clock: 0 });
                }
                continue;
            }
            _ => continue, // the pan has no effect (audio-platform.md)
        }
        out.push(Entry::Call(c));
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (log, out, dir) = match args.as_slice() {
        [log, out] => (log, out, None),
        [flag, dir, log, out] if flag == "--script" => (log, out, Some(dir.clone())),
        _ => {
            eprintln!("usage: fmod-mix [--script <dir below Sound_PC>] <log> <out.wav>");
            std::process::exit(2);
        }
    };
    let dir_env = std::env::var_os("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(PathBuf::from(dir_env).join("data/FFX_Data.vbf"))
        .expect("FFX_Data.vbf");
    let text = std::fs::read(log).expect("log");
    let text = String::from_utf8_lossy(&text);
    let entries = match &dir {
        Some(d) => script(&text, d),
        None => parse(&text),
    };
    let mut engine = Engine::new(Vbf {
        archive,
        cache: HashMap::new(),
    });
    engine.run(&entries);
    std::fs::write(out, wav(&engine.out)).expect("write");
    eprintln!(
        "{} updates, {} frames ({BLOCK} a block)",
        engine.updates,
        engine.out.len() / 2
    );
    eprintln!(
        "channel index checks: {} match, {} differ",
        engine.channel_checks.0, engine.channel_checks.1
    );
    let mut ignored: Vec<_> = engine.ignored.iter().collect();
    ignored.sort();
    for (f, n) in ignored {
        eprintln!("  not followed: {f} x{n}");
    }
}
