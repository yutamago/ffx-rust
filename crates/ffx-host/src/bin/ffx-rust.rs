//! `ffx-rust`: runs FFX without loading `FFX.exe` (docs/host.md).
//!
//! ```text
//! ffx-rust [--game-dir DIR] [--sym FILE] [--budget SECONDS] [--cover]
//!          [--lenient] [--count] [--stop-at DLL!name]... [--no-dlls]
//!          [--documents DIR] [--profile FILE] [--routes FILE]
//!          [--gfx-out DIR] [--gfx-null] [--presents N]
//! ```
//!
//! `--documents` is the folder the game sees as its documents folder
//! (`GameSetting.ini`, the saves; written to). `--profile` writes every
//! function that ran in Unicorn with its calls and time (`--cover`
//! implied); `--routes` where each import went (Rust, DLL, missing).
//!
//! The game directory defaults to `FFX_GAME_DIR`, then the directory of
//! this executable. The run's report goes to stdout. Development build:
//! functions without a port run in Unicorn from the player's `FFX.exe`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

#[allow(clippy::too_many_lines)] // the argument loop and the report
fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut game: Option<PathBuf> = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    let mut sym = PathBuf::from("symbols/ffx.sym");
    let mut budget = Duration::from_secs(60);
    let mut cover = false;
    let mut lenient = false;
    let mut count = false;
    let mut stops = Vec::new();
    let mut opts = ffx_host::Options::default();
    let mut profile: Option<PathBuf> = None;
    let mut routes: Option<PathBuf> = None;
    let mut gfx_out: Option<PathBuf> = None;
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
            "--no-dlls" => opts.no_dlls = true,
            "--documents" => opts.documents = args.next().map(PathBuf::from),
            "--profile" => {
                profile = args.next().map(PathBuf::from);
                cover = true;
            }
            "--routes" => routes = args.next().map(PathBuf::from),
            "--gfx-out" => gfx_out = args.next().map(PathBuf::from),
            "--gfx-null" => opts.gfx.backend = ffx_host::gfx::Choice::Null,
            "--presents" => {
                opts.gfx.stop_after_presents = args.next().and_then(|s| s.parse().ok());
            }
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
    let table = ffx_host::ports::table();
    let mut exec = match ffx_host::boot_opts(&game.join("FFX.exe"), table, &opts) {
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
    exec.set_lenient(lenient);
    if count && let Err(e) = exec.count_instructions() {
        eprintln!("count: {e}");
    }
    let outcome = match exec.run_entry() {
        Ok(stop) => format!("{stop}"),
        Err(e) => format!("error: {e}"),
    };
    exec.join_threads(Duration::from_secs(2));
    let syms = ffx_host::report::Symbols::read(&sym);
    println!("{}", ffx_host::report::text(&exec, &syms, &outcome));
    print!("{}", ffx_host::gfx::report(&exec));
    if let Some(dir) = gfx_out
        && let Err(e) = ffx_host::gfx::write(&exec, &dir)
    {
        eprintln!("gfx-out: {e}");
    }
    if let Some(p) = profile {
        let s = ffx_host::report::summary(&exec, &syms);
        if let Err(e) = std::fs::write(&p, ffx_host::report::profile_tsv(&s)) {
            eprintln!("profile: {e}");
        }
    }
    if let Some(p) = routes
        && let Err(e) = std::fs::write(&p, ffx_host::report::routes(&exec))
    {
        eprintln!("routes: {e}");
    }
    println!("wall: {:.3} s", t.elapsed().as_secs_f64());
    if count {
        println!(
            "instructions emulated (main thread): {}",
            exec.instructions()
        );
    }
    ExitCode::SUCCESS
}
