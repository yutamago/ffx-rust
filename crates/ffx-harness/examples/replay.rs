//! Plays a replay script (`crate::replay`) and writes its trace:
//! `replay <script> <trace windows path> [original]`. With `original` the
//! Rust replacements are not installed, so the trace is the original
//! code's. `$OUT` in the script is the trace's folder (screenshots). Compare
//! two traces with `replay_diff`. Faults and panics the hook logged during
//! the run are printed to stderr (`hook log: ...`). A `settings NAME` line
//! in the script (or `FFX_SETTINGS=NAME`) starts the game with
//! `tools/sandbox/NAME.ini` merged over the sandbox's settings.
fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let script = std::fs::read_to_string(
        args.next()
            .expect("usage: replay <script> <trace> [original]"),
    )?;
    let trace = args.next().expect("trace path");
    let mut options = ffx_harness::Options {
        virtual_clock: true,
        ..ffx_harness::Options::default()
    };
    if ffx_harness::replay::realtime(&script) {
        options.fast = false;
        options.virtual_clock = false;
    }
    if let Some(name) = ffx_harness::replay::settings(&script) {
        options.settings = Some(name);
    }
    if args.next().as_deref() == Some("original") {
        options.hook_disable = Some("replacements".into());
    }
    let mut game = ffx_harness::Game::launch(&options)?;
    // The time limit runs from here, with the game slot ours: a limit around
    // the whole command (run-all.sh's `timeout`) also counted the wait for a
    // slot, and with many agents queued, replays were killed before they
    // started (2026-10-29). FFX_REPLAY_TIMEOUT seconds, default 900; ends
    // the run with 124 as `timeout` does.
    let limit = std::env::var("FFX_REPLAY_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(900);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(limit));
        eprintln!("replay: {limit} s since the game started; giving up");
        std::process::exit(124);
    });
    eprintln!(
        "replay: slot {}, port {}",
        game.instance().unwrap_or(0),
        game.port()
    );
    let played = ffx_harness::replay::play(&mut game, &script, &trace);
    let log = game.new_log();
    let quit = game.quit();
    // FAULT/PANIC lines and a few after each.
    let lines: Vec<&str> = log.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("FAULT") || line.contains("PANIC") {
            for l in &lines[i..(i + 5).min(lines.len())] {
                eprintln!("hook log: {l}");
            }
        }
    }
    for line in played? {
        println!("{line}");
    }
    quit
}
