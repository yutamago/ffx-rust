//! `ffx-rust`: runs FFX without loading `FFX.exe` (docs/host.md).
//!
//! ```text
//! ffx-rust [--game-dir DIR] [--sym FILE] [--budget SECONDS] [--cover]
//!          [--lenient] [--count] [--stop-at DLL!name]... [--no-dlls]
//!          [--documents DIR] [--profile FILE] [--routes FILE] [--ports FILE]
//!          [--gfx-out DIR] [--gfx-null] [--presents N] [--replay FILE]
//!          [--audio-wav FILE]
//! ```
//!
//! Under the test harness (`FFX_HARNESS=<port>`, docs/harness.md "Two
//! processes") it serves the harness protocol, takes its Documents folder
//! from `FFX_HARNESS_HOME` and runs until the test quits it.
//! `--audio-wav` (else `FFX_AUDIO_WAV`) writes the mix to a WAV file and
//! the harness's `audiomark`s to `<file>.marks` beside it.
//!
//! `--documents` is the folder the game sees as its documents folder
//! (`GameSetting.ini`, the saves; written to). `--profile` writes every
//! function that ran in Unicorn with its calls and time (`--cover`
//! implied; under the harness the file is written at the test's `quit`,
//! which ends the process before this program's report); `--routes` where
//! each import went (Rust, DLL, missing); `--ports` every port that ran,
//! with its calls. `--sym` defaults to `FFX_HOST_SYM`, else
//! `symbols/ffx.sym`, else `ffx.sym` beside the executable.
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
    let mut sym: Option<PathBuf> = None;
    let mut budget: Option<Duration> = None;
    let mut cover = false;
    let mut lenient = false;
    let mut count = false;
    let mut stops = Vec::new();
    let mut opts = ffx_host::Options::default();
    let mut profile: Option<PathBuf> = None;
    let mut routes: Option<PathBuf> = None;
    let mut ports: Option<PathBuf> = None;
    let mut gfx_out: Option<PathBuf> = None;
    let mut replay: Option<PathBuf> = None;
    let mut audio_wav: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--game-dir" => game = args.next().map(PathBuf::from),
            "--sym" => sym = args.next().map(PathBuf::from).or(sym),
            "--budget" => {
                budget = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .map(Duration::from_secs_f64);
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
            "--ports" => ports = args.next().map(PathBuf::from),
            "--gfx-out" => gfx_out = args.next().map(PathBuf::from),
            "--replay" => replay = args.next().map(PathBuf::from),
            "--audio-wav" => audio_wav = args.next().map(PathBuf::from),
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
    let sym = sym
        .or_else(ffx_host::report::Symbols::locate)
        .unwrap_or_else(|| PathBuf::from("symbols/ffx.sym"));
    let game = game.or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from))
    });
    let Some(game) = game else {
        eprintln!("no game directory (--game-dir or FFX_GAME_DIR)");
        return ExitCode::FAILURE;
    };
    // the draw recording only when it will be written (it grows by every
    // frame's draws)
    opts.gfx.no_record = gfx_out.is_none()
        && std::env::var("FFX_HOST_GFX_RECORD")
            .ok()
            .is_none_or(|v| v != "1");
    // the harness's sandbox (FFX_HARNESS_HOME) unless --documents says
    if opts.documents.is_none() {
        opts.documents = ffx_host::harness::home();
    }
    // a harness game runs until the test quits it
    let harness = std::env::var_os("FFX_HARNESS").is_some();
    let budget = budget.unwrap_or(Duration::from_secs(if harness { 7 * 86_400 } else { 60 }));
    let t = Instant::now();
    let table = ffx_host::ports::table();
    let mut exec = match ffx_host::boot_opts(&game.join("FFX.exe"), table, &opts) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("boot: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = ffx_host::harness::start(exec.shared()) {
        eprintln!("harness: {e}");
        return ExitCode::FAILURE;
    }
    if harness && let Some(p) = &profile {
        // the test's `quit` ends the process: the profile is written there
        let abs = |p: &PathBuf| std::path::absolute(p).unwrap_or_else(|_| p.clone());
        ffx_host::harness::profile_at_quit(abs(p), abs(&sym));
    }
    if cover && let Err(e) = exec.cover() {
        eprintln!("cover: {e}");
    }
    for s in &stops {
        exec.stop_at(s);
    }
    exec.set_budget(budget);
    // the audio capture: `--audio-wav`, or the harness's `FFX_AUDIO_WAV`
    // (a Windows path, as `tools/run-game.sh` passes it), with its marks
    // file beside it (docs/harness.md, "Audio capture")
    let audio_wav = audio_wav.or_else(|| {
        std::env::var_os("FFX_AUDIO_WAV")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    });
    if let Some(w) = &audio_wav
        && let Err(e) = exec.shared().audio.capture(w)
    {
        eprintln!("audio capture {}: {e}", w.display());
    }
    if let Some(p) = &replay {
        match std::fs::read_to_string(p) {
            Ok(text) => {
                let mut script = ffx_host::replay::Script::new(&text);
                script.stop_when_done = true;
                if let Ok(mut s) = exec.shared().clock.script.lock() {
                    *s = Some(script);
                }
            }
            Err(e) => {
                eprintln!("replay {}: {e}", p.display());
                return ExitCode::FAILURE;
            }
        }
    }
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
    if let Some(p) = ports
        && let Err(e) = std::fs::write(&p, ffx_host::report::ports_tsv(&exec))
    {
        eprintln!("ports: {e}");
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
