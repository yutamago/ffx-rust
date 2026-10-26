//! Measures how many frames Valefor's magic file takes to load with every
//! frame slowed (`ffx_harness::pacing`): `load_pacing [original] [ms]`.
//! With `original` the Rust replacements are not installed;
//! `FFX_HOOK_ONLY` / `FFX_HOOK_DISABLE` pick replacements as for any
//! launch. Prints `frames <n> loader <n> request <frame>`.
fn main() -> ffx_harness::Result<()> {
    let mut options = ffx_harness::Options {
        virtual_clock: true,
        ..ffx_harness::Options::default()
    };
    let mut pacing = ffx_harness::pacing::Options::default();
    for arg in std::env::args().skip(1) {
        if arg == "original" {
            options.hook_disable = Some("replacements".into());
        } else if arg == "stacks" {
            pacing.stacks = true;
        } else if let Ok(ms) = arg.parse() {
            pacing.slow = std::time::Duration::from_millis(ms);
        }
    }
    let mut game = ffx_harness::Game::launch(&options)?;
    let measured = ffx_harness::pacing::measure(&mut game, &options.repo, &pacing);
    let quit = game.quit();
    let m = measured?;
    for line in &m.log {
        eprintln!("{line}");
    }
    println!(
        "frames {} loader {} request {}",
        m.frames, m.loader_busy, m.request
    );
    quit
}
