//! Plays a replay script on a game that is already running, with the
//! harness listening: `replay_attach <port> <script> <trace path>`. For a
//! headless `ffx-rust` on Linux (`FFX_HARNESS=<port> ffx-rust --gfx-null`;
//! docs/host.md, "Hangs"): no game slot, no window, no launch. `shot` lines
//! are skipped (the null device has no back buffer). Leaves the game
//! running unless the script ends it; `FFX_REPLAY_QUIT=1` sends `quit`.
fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let port: u16 = args
        .next()
        .and_then(|p| p.parse().ok())
        .expect("usage: replay_attach <port> <script> <trace>");
    let script = std::fs::read_to_string(args.next().expect("script"))?;
    let trace = args.next().expect("trace path");
    let script: String = script
        .lines()
        .filter(|l| !l.trim_start().starts_with("shot "))
        .flat_map(|l| [l, "\n"])
        .collect();
    let mut game = ffx_harness::Game::connect(port)?;
    let played = ffx_harness::replay::play(&mut game, &script, &trace);
    for line in played? {
        println!("{line}");
    }
    if std::env::var_os("FFX_REPLAY_QUIT").is_some() {
        let _ = game.command("quit");
    }
    Ok(())
}
