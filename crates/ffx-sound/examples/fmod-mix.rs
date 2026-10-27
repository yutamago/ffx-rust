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
use ffx_sound::replay::{Engine, Files, parse};

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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [log, out] = args.as_slice() else {
        eprintln!("usage: fmod-mix <log> <out.wav>");
        std::process::exit(2);
    };
    let dir = std::env::var_os("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive =
        ffx_vbf::Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).expect("FFX_Data.vbf");
    let text = std::fs::read(log).expect("log");
    let entries = parse(&String::from_utf8_lossy(&text));
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
