//! `ffx-rust`: runs FFX without loading `FFX.exe` (docs/host.md).
//!
//! ```text
//! ffx-rust [--game-dir DIR] [--sym FILE] [--budget SECONDS] [--cover]
//!          [--lenient] [--count] [--stop-at DLL!name]...
//! ```
//!
//! The game directory defaults to `FFX_GAME_DIR`, then the directory of
//! this executable. The run's report goes to stdout. Development build:
//! functions without a port run in Unicorn from the player's `FFX.exe`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut game: Option<PathBuf> = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    let mut sym = PathBuf::from("symbols/ffx.sym");
    let mut budget = Duration::from_secs(60);
    let mut cover = false;
    let mut lenient = false;
    let mut count = false;
    let mut stops = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--game-dir" => game = args.next().map(PathBuf::from),
            "--sym" => sym = args.next().map_or(sym, PathBuf::from),
            "--budget" => {
                budget = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .map_or(budget, Duration::from_secs_f64);
            }
            "--cover" => cover = true,
            "--lenient" => lenient = true,
            "--count" => count = true,
            "--stop-at" => stops.extend(args.next()),
            other => {
                eprintln!("unknown argument {other}");
                return ExitCode::FAILURE;
            }
        }
    }
    let game = game.or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from))
    });
    let Some(game) = game else {
        eprintln!("no game directory (--game-dir or FFX_GAME_DIR)");
        return ExitCode::FAILURE;
    };
    let t = Instant::now();
    let mut exec = match ffx_host::boot(&game.join("FFX.exe")) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("boot: {e}");
            return ExitCode::FAILURE;
        }
    };
    if cover && let Err(e) = exec.cover() {
        eprintln!("cover: {e}");
    }
    for s in &stops {
        exec.stop_at(s);
    }
    exec.set_budget(budget);
    exec.lenient = lenient;
    if count && let Err(e) = exec.count_instructions() {
        eprintln!("count: {e}");
    }
    let outcome = match exec.run_entry() {
        Ok(stop) => format!("{stop:?}"),
        Err(e) => format!("error: {e}"),
    };
    let syms = ffx_host::report::Symbols::read(&sym);
    println!("{}", ffx_host::report::text(&exec, &syms, &outcome));
    println!("wall: {:.3} s", t.elapsed().as_secs_f64());
    if count {
        println!("instructions emulated: {}", exec.instructions());
    }
    ExitCode::SUCCESS
}
