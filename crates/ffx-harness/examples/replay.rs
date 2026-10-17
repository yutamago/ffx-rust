//! Plays a replay script (`crate::replay`) and writes its trace:
//! `replay <script> <trace windows path> [original]`. With `original` the
//! Rust replacements are not installed, so the trace is the original
//! code's. `$OUT` in the script is the trace's folder (screenshots). Compare
//! two traces with `replay_diff`. Faults and panics the hook logged during
//! the run are printed to stderr (`hook log: ...`).
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
    if args.next().as_deref() == Some("original") {
        options.hook_disable = Some("replacements".into());
    }
    let mut game = ffx_harness::Game::launch(&options)?;
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
