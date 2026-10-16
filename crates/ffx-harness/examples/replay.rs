//! Plays a replay script (`crate::replay`) and writes its trace:
//! `replay <script> <trace windows path> [original]`. With `original` the
//! Rust replacements are not installed, so the trace is the original
//! code's. Compare two traces with `replay_diff`.
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
    for line in ffx_harness::replay::play(&mut game, &script, &trace)? {
        println!("{line}");
    }
    game.quit()
}
