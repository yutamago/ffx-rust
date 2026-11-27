//! `ffx-rust`: runs FFX without loading `FFX.exe` (docs/host.md).
//!
//! ```text
//! ffx-rust [--game-dir DIR] [--sym FILE] [--budget SECONDS] [--cover]
//!          [--lenient] [--count] [--stop-at DLL!name]... [--no-dlls]
//!          [--documents DIR] [--profile FILE] [--routes FILE] [--ports FILE]
//!          [--gfx-out DIR] [--gfx-null] [--presents N] [--replay FILE]
//!          [--audio-wav FILE] [--clock real|virtual] [--skip-opening]
//!          [--speed X]
//! ```
//!
//! `--skip-opening` ends the start-up logos on their first frame on the
//! real clock too, as the virtual clock does (`Options::skip_opening`): a
//! build without Ruffle (no `swf` feature) never gets the movie's end
//! otherwise, and a player's run on the real clock stays in the start-up
//! loop.
//!
//! The clock (docs/host.md, "The clock"): the real one, as `FFX.exe`
//! runs (the frame limiter holds 30 updates a second), for a player's run
//! (no harness, a window) and a harness game outside fast mode (a
//! `realtime` replay: no `FFX_HARNESS_FAST=1`); the virtual one (as fast as
//! the host goes) for fast-mode harness games, `--replay`, and runs on the
//! null device without the harness. `--clock` (else `FFX_HOST_CLOCK`)
//! picks one. `--speed X` (else `FFX_HOST_SPEED`) runs the real clock X
//! times as fast, 0.25 to 8, for debugging (`Options::speed`; docs/host.md
//! "Game speed").
//!
//! `FFX_HOST_GFX=null` picks the null device as `--gfx-null` does (harness
//! runs pass no arguments).
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
//! implied; the calls from Rust into Unicorn by target go beside it,
//! `x.profile.tsv` → `x.entries.tsv`; under the harness the file is written at the test's `quit`,
//! which ends the process before this program's report); `--routes` where
//! each import went (Rust, DLL, missing); `--ports` every port that ran,
//! with its calls. `--sym` defaults to `FFX_HOST_SYM`, else
//! `symbols/ffx.sym`, else `ffx.sym` beside the executable.
//!
//! The game directory defaults to `FFX_GAME_DIR`, then the directory of
//! this executable. The run's report goes to stdout. Development build:
//! functions without a port run in Unicorn from the player's `FFX.exe`.
//!
//! Started with no arguments outside the harness, it is a player's run
//! (`ffx_host::player`; docs/host.md, "A player's run"): the game folder
//! is `FFX_GAME_DIR`, else the current folder or this executable's when
//! it holds `FFX.exe`, else Steam's install of the game; the documents
//! folder the original uses (Linux: the game's Proton prefix, else a
//! folder of our own); `SteamAppId` set when Steam didn't set it; no time
//! limit. Problems show in a message box.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

#[allow(clippy::too_many_lines)] // the argument loop and the report
fn main() -> ExitCode {
    // a player's run: no arguments, not under the harness
    let player = std::env::args_os().len() <= 1 && std::env::var_os("FFX_HARNESS").is_none();
    if player && std::env::var_os("SteamAppId").is_none_or(|v| v.is_empty()) {
        // SAFETY: the first thing the process does: no other thread exists
        // to read the environment.
        unsafe { std::env::set_var("SteamAppId", ffx_host::player::STEAM_APP_ID) };
    }
    let mut args = std::env::args().skip(1);
    let mut game: Option<PathBuf> = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    let mut sym: Option<PathBuf> = None;
    let mut budget: Option<Duration> = None;
    let mut cover = false;
    let mut lenient = false;
    let mut count = false;
    let mut stops = Vec::new();
    let mut opts = ffx_host::Options::default();
    // `FFX_HOST_GFX=null`: the null device, as `--gfx-null` (for harness
    // runs, which pass no arguments: a hang bisected to the real device)
    if std::env::var("FFX_HOST_GFX").is_ok_and(|v| v == "null") {
        opts.gfx.backend = ffx_host::gfx::Choice::Null;
    }
    let mut profile: Option<PathBuf> = None;
    let mut routes: Option<PathBuf> = None;
    let mut ports: Option<PathBuf> = None;
    let mut gfx_out: Option<PathBuf> = None;
    let mut replay: Option<PathBuf> = None;
    let mut audio_wav: Option<PathBuf> = None;
    opts.speed = std::env::var("FFX_HOST_SPEED")
        .ok()
        .and_then(|v| v.parse::<f64>().ok());
    let mut clock: Option<String> = std::env::var("FFX_HOST_CLOCK")
        .ok()
        .filter(|v| !v.is_empty());
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
            "--clock" => clock = args.next(),
            "--gfx-null" => opts.gfx.backend = ffx_host::gfx::Choice::Null,
            "--skip-opening" => opts.skip_opening = true,
            "--speed" => {
                let Some(x) = args.next().and_then(|s| s.parse::<f64>().ok()) else {
                    eprintln!("--speed X: a number, 0.25 to 8");
                    return ExitCode::FAILURE;
                };
                opts.speed = Some(x);
            }
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
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from));
    let game = if player {
        let cwd = std::env::current_dir().ok();
        ffx_host::player::find_game(
            game,
            cwd.as_deref(),
            exe_dir.as_deref(),
            &ffx_host::player::steam_roots(),
        )
    } else {
        game.or(exe_dir)
    };
    let Some(game) = game else {
        if player {
            ffx_host::platform::get().message_box(
                "ffx-rust",
                "FINAL FANTASY X/X-2 HD Remaster was not found.\n\nPut ffx-rust into the \
                 game folder (the one with FFX.exe), start it from there, or set \
                 FFX_GAME_DIR to that folder.",
                false,
            );
        } else {
            eprintln!("no game directory (--game-dir or FFX_GAME_DIR)");
        }
        return ExitCode::FAILURE;
    };
    if player && opts.documents.is_none() {
        opts.documents =
            ffx_host::player::documents(ffx_host::platform::get().documents_folder(), &game);
    }
    // the draw recording only when it will be written (it grows by every
    // frame's draws)
    opts.gfx.no_record = gfx_out.is_none()
        && std::env::var("FFX_HOST_GFX_RECORD")
            .ok()
            .is_none_or(|v| v != "1");
    // what the harness's scene-gate captures need (`gfxcapture ... hash
    // frames N now`; crate::gfx::gate)
    opts.gfx.gate = std::env::var("FFX_HOST_GFX_CAPTURE").is_ok_and(|v| v == "1");
    // the harness's sandbox (FFX_HARNESS_HOME) unless --documents says
    if opts.documents.is_none() {
        opts.documents = ffx_host::harness::home();
    }
    // a harness game runs until the test quits it
    let harness = std::env::var_os("FFX_HARNESS").is_some();
    let fast = std::env::var("FFX_HARNESS_FAST").is_ok_and(|v| v == "1");
    let null = matches!(opts.gfx.backend, ffx_host::gfx::Choice::Null);
    opts.real_clock = match clock.as_deref() {
        Some("real") => true,
        Some("virtual") => false,
        Some(other) => {
            eprintln!("--clock {other}: real or virtual");
            return ExitCode::FAILURE;
        }
        None if harness => !fast,
        None => replay.is_none() && !null,
    };
    // a harness game runs a week, a player's run a year (no time limit);
    // development runs stop after a minute unless --budget says
    let budget = budget.unwrap_or(Duration::from_secs(if harness {
        7 * 86_400
    } else if player {
        365 * 86_400
    } else {
        60
    }));
    let t = Instant::now();
    let table = ffx_host::ports::table();
    let mut exec = match ffx_host::boot_opts(&game.join("FFX.exe"), table, &opts) {
        Ok(e) => e,
        Err(e) => {
            if player {
                ffx_host::platform::get().message_box(
                    "ffx-rust",
                    &format!("The game could not start from {}:\n\n{e}", game.display()),
                    false,
                );
            }
            eprintln!("boot: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = ffx_host::harness::start(exec.shared()) {
        eprintln!("harness: {e}");
        return ExitCode::FAILURE;
    }
    // the sampler (FFX_HOST_SAMPLE; the harness's `sample reset|stop`)
    if harness {
        ffx_host::sample::start_global(exec.shared());
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
        let entries = ffx_host::report::entries_tsv(exec.shared(), &exec.all_profile(), &syms);
        if let Err(e) = std::fs::write(ffx_host::report::entries_path(&p), entries) {
            eprintln!("profile entries: {e}");
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
