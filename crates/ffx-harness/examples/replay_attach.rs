//! Plays a replay script on a game that is already running, with the
//! harness listening: `replay_attach <port> <script> <trace path>`. For a
//! headless `ffx-rust` on Linux (`FFX_HARNESS=<port> ffx-rust --gfx-null`;
//! docs/host.md, "Hangs"): no game slot, no window, no launch. `shot` lines
//! are skipped and `shotwhen` lines wait for their condition without the
//! shot (the null device has no back buffer). Leaves the game
//! running unless the script ends it; `FFX_REPLAY_QUIT=1` sends `quit`.
//! `FFX_REPLAY_PACE=1` plays the script a line at a time and prints, after
//! each, the wall time, the frame (updates done) and the updates per wall
//! second since the line before: the pacing of a `realtime` run.
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
        .map(shot_when_as_wait)
        .flat_map(|l| [l, "\n".to_owned()])
        .collect();
    let mut game = ffx_harness::Game::connect(port)?;
    if std::env::var_os("FFX_REPLAY_PACE").is_some() {
        pace(&mut game, &script, &trace)?;
    } else {
        let played = ffx_harness::replay::play(&mut game, &script, &trace);
        for line in played? {
            println!("{line}");
        }
    }
    if std::env::var_os("FFX_REPLAY_QUIT").is_some() {
        let _ = game.command("quit");
    }
    Ok(())
}

/// `shotwhen <range> <value> <mask> <limit> <path>` as `waitfor <range>
/// <value> <mask> <limit>`; other lines as they are.
fn shot_when_as_wait(line: &str) -> String {
    let words: Vec<&str> = line.split_whitespace().collect();
    match words.as_slice() {
        ["shotwhen", range, value, mask, limit, ..] => {
            format!("waitfor {range} {value} {mask} {limit}")
        }
        _ => line.to_owned(),
    }
}

/// `FFX_REPLAY_PACE=1`: the script a line at a time; per line the wall
/// time, the frame and the updates per wall second over the line.
fn pace(game: &mut ffx_harness::Game, script: &str, trace: &str) -> ffx_harness::Result<()> {
    let start = std::time::Instant::now();
    let frame = |g: &mut ffx_harness::Game| -> ffx_harness::Result<u32> {
        Ok(g.command("frame")?.trim().parse().unwrap_or(0))
    };
    let (mut t0, mut f0) = (0.0, frame(game)?);
    for line in script.lines() {
        let body = line.split('#').next().unwrap_or("").trim();
        if body.is_empty() || body == "realtime" {
            continue;
        }
        let replies = ffx_harness::replay::play(game, body, trace)?;
        let (t, f) = (start.elapsed().as_secs_f64(), frame(game)?);
        let rate = f64::from(f.saturating_sub(f0)) / (t - t0).max(1e-9);
        println!(
            "pace {t:9.3} s frame {f:6} {:6} updates in {:7.3} s = {rate:6.2}/s  {body}  {}",
            f.saturating_sub(f0),
            t - t0,
            replies.join(" | ")
        );
        (t0, f0) = (t, f);
    }
    Ok(())
}
