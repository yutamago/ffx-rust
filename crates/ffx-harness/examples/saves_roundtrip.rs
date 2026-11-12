//! The saves and settings round trip (M5 stage 5; `docs/harness.md`,
//! "Saves round trip"; run it through `tools/saves-roundtrip.sh`):
//!
//! ```text
//! saves_roundtrip run <out> [step...]   # steps: write-orig write-rust read-orig read-rust
//! saves_roundtrip compare <out>
//! ```
//!
//! `run` plays `tools/replays/saves-write.txt` (the writers) or
//! `saves-read.txt` (the readers) once per step, each game in its own
//! sandboxed home `<out>/<step>/home` (`Options::home`), with what it
//! started with kept as `<out>/<step>/seed` (`Options::home_seed`). The
//! writers' homes are made as every test game's (the user's saves copied,
//! or `FFX_SAVES_FROM`; the sandbox's settings); a reader's is a verbatim
//! copy of the other program's writer home (`Options::home_from`):
//! read-orig starts with write-rust's files, read-rust with write-orig's.
//! "orig" is `FFX.exe` with the original code (every replacement off, as
//! a recording); `FFX_ROUNDTRIP_FFX=hooked` keeps the Rust replacements
//! in. "rust" is `ffx-rust.exe` (`Options::rust`). The harness replies go
//! to `<out>/<step>/replies.txt`, the screenshots and trace beside them,
//! `ffx-rust.exe`'s log to `<out>/<step>/ffx-rust.log`. Steps run in the
//! order given; a reader needs its writer's home.
//!
//! `compare` (also after `run`) prints `ffx_harness::saves::compare`'s
//! report and writes it to `<out>/report.txt`. Exit code: 0 pass, 1 a
//! check failed, 2 runs missing, 3 a run failed.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ffx_harness::saves;

static STEP: AtomicUsize = AtomicUsize::new(0);

fn usage() -> ! {
    eprintln!("usage: saves_roundtrip run <out> [step...] | compare <out>");
    eprintln!("steps: {}", saves::STEPS.join(" "));
    std::process::exit(2);
}

fn run_step(out: &Path, step: &str) -> ffx_harness::Result<()> {
    let dir = out.join(step);
    std::fs::create_dir_all(&dir)?;
    let rust = step.ends_with("-rust");
    let mut options = ffx_harness::Options {
        virtual_clock: true,
        rust,
        home: ffx_harness::Home::Dir(dir.join("home")),
        home_seed: Some(dir.join("seed")),
        ..ffx_harness::Options::default()
    };
    if !rust && std::env::var("FFX_ROUNDTRIP_FFX").as_deref() != Ok("hooked") {
        options.hook_disable = Some("replacements".into());
    }
    let script = if let Some(source) = saves::source_of(step) {
        let from = out.join(source).join("home");
        if !from.is_dir() {
            return Err(ffx_harness::Error::Launch(format!(
                "{step} reads {source}'s home, and {} does not exist: run {source} first",
                from.display()
            )));
        }
        options.home_from = Some(from);
        "saves-read.txt"
    } else {
        "saves-write.txt"
    };
    let script_path = options.repo.join("tools/replays").join(script);
    let text = std::fs::read_to_string(&script_path)?;
    let trace = ffx_harness::windows_path(&dir.join(format!("{step}.trace")))?;

    // Each step's time limit runs from its game's start, as in `replay`:
    // FFX_REPLAY_TIMEOUT seconds, default 900 (FFX.exe) or 3600 (ffx-rust).
    let limit = std::env::var("FFX_REPLAY_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(if rust { 3600 } else { 900 });
    let mut game = ffx_harness::Game::launch(&options)?;
    let me = STEP.fetch_add(1, Ordering::AcqRel) + 1;
    let name = step.to_owned();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(limit));
        if STEP.load(Ordering::Acquire) == me {
            eprintln!("saves_roundtrip: {name}: {limit} s since the game started; giving up");
            std::process::exit(124);
        }
    });
    eprintln!(
        "saves_roundtrip: {step} ({script}) in slot {}, port {}, home {}",
        game.instance().unwrap_or(0),
        game.port(),
        dir.join("home").display()
    );
    let played = ffx_harness::replay::play(&mut game, &text, &trace);
    let log = game.new_log();
    let rust_log = game.game_dir().map(|d| d.join("ffx-rust.log"));
    let quit = game.quit();
    STEP.fetch_add(1, Ordering::AcqRel);
    if rust
        && let Some(log) = rust_log
        && let Err(e) = std::fs::copy(&log, dir.join("ffx-rust.log"))
    {
        eprintln!("saves_roundtrip: {step}: no ffx-rust.log ({e})");
    }
    let lines: Vec<&str> = log.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("FAULT") || line.contains("PANIC") {
            for l in &lines[i..(i + 5).min(lines.len())] {
                eprintln!("hook log: {l}");
            }
        }
    }
    let replies = played?;
    std::fs::write(dir.join("replies.txt"), replies.join("\n") + "\n")?;
    quit
}

fn compare(out: &Path) -> i32 {
    match saves::compare(out) {
        Ok(v) => {
            print!("{}", v.text);
            if let Err(e) = std::fs::write(out.join("report.txt"), &v.text) {
                eprintln!("saves_roundtrip: report.txt: {e}");
            }
            if v.failures > 0 {
                1
            } else if v.missing > 0 {
                2
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("saves_roundtrip: compare: {e}");
            3
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(mode), Some(out)) = (args.first(), args.get(1)) else {
        usage()
    };
    let out = PathBuf::from(out);
    match mode.as_str() {
        "compare" => std::process::exit(compare(&out)),
        "run" => {
            let steps: Vec<&str> = if args.len() > 2 {
                args[2..].iter().map(String::as_str).collect()
            } else {
                saves::STEPS.to_vec()
            };
            if let Some(bad) = steps.iter().find(|s| !saves::STEPS.contains(s)) {
                eprintln!("saves_roundtrip: no step {bad}");
                usage();
            }
            for step in steps {
                if let Err(e) = run_step(&out, step) {
                    eprintln!("saves_roundtrip: {step} failed: {e}");
                    std::process::exit(3);
                }
            }
            std::process::exit(compare(&out));
        }
        _ => usage(),
    }
}
