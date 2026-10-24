//! Captures the pinned field-walk frame (`docs/drawdiff.md`, "Reference
//! captures"): save slot 1 (cdsp0000), on the original code
//! (`--hook-disable replacements`), seeded, fast mode, virtual clock,
//! lockstep, in background mode through the game-slot queue, with ctrl
//! work 0's event count (`0x01325d94`) held at field-walk-1's value.
//!
//! `field_capture <windows dir> [rust]`: writes `capture.ffxgfx` and the
//! rest of `gfxcapture`'s output into that folder; prints the event count
//! before the capture. With `rust` the Rust replacements stay in.
//!
//! `gfxcapture` waits for two Presents, which lockstep holds back, so the
//! capture is asked for together with `lockstep off` in one write: the
//! harness reads both lines at once and sets the capture up while the
//! game thread is still asleep in lockstep's 1 ms hold.
use std::io::Write as _;

fn main() -> ffx_harness::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: field_capture <windows dir> [rust]");
    let rust = std::env::args().nth(2).as_deref() == Some("rust");
    let options = ffx_harness::Options {
        gfx: Some("1".into()),
        hook_disable: (!rust).then(|| "replacements".into()),
        virtual_clock: true,
        ..ffx_harness::Options::default()
    };
    let mut game = ffx_harness::Game::launch(&options)?;
    eprintln!("field_capture: slot {}", game.instance().unwrap_or(0));
    let opened = ffx_harness::screens::load_save_direct(&mut game, "1")?;
    if !opened.contains("cdsp0000") {
        return Err(ffx_harness::Error::Harness(format!(
            "{opened} started, not cdsp0000"
        )));
    }
    game.command("lockstep on")?;
    // As tools/replays/field-walk.txt: the count in 64..127, then 37
    // frames (field-walk-1 is taken at 101).
    game.command("waitfor 0x01325d94:4 40000000 c0ffffff 1500")?;
    game.wait(37)?;
    let count = game.command("read 0x01325d94 4")?;
    println!("event count {count}");
    // The folders gfxcapture makes, made first so it sets up at once.
    let started = std::time::Instant::now();
    let reply = game.command(&format!("lockstep off\ngfxcapture {dir}"))?;
    println!("lockstep off: {reply}");
    // The next reply line is gfxcapture's; `ping`'s pong stays queued and
    // is read by the next command.
    let draws = game.command("ping")?;
    println!(
        "gfxcapture: {draws} ({:.1} s)",
        started.elapsed().as_secs_f64()
    );
    let _ = std::io::stdout().flush();
    let log = game.new_log();
    for line in log
        .lines()
        .filter(|l| l.contains("FAULT") || l.contains("PANIC"))
    {
        eprintln!("hook log: {line}");
    }
    game.quit()
}
