//! The game slot from the shell ([`ffx_harness::slot`]).
//!
//!   game-slot status             who has the game, who waits
//!   game-slot run -- <command>   queues, runs the command in the slot
//!
//! `run` sets `FFX_GAME_SLOT_HELD=1` for the command, so the games it
//! launches (one or several, as the replays do) use this slot instead of
//! queueing again.
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("status") => {
            print!("{}", ffx_harness::slot::status());
            std::process::ExitCode::SUCCESS
        }
        Some("run") => {
            let rest = args.iter().skip(1).skip_while(|a| *a == "--");
            let mut rest = rest.cloned();
            let Some(program) = rest.next() else {
                eprintln!("game-slot run -- <command> [args]");
                return std::process::ExitCode::from(2);
            };
            let _slot = match ffx_harness::slot::Slot::acquire() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("game slot: {e}");
                    return std::process::ExitCode::from(2);
                }
            };
            match std::process::Command::new(&program)
                .args(rest)
                .env("FFX_GAME_SLOT_HELD", "1")
                .status()
            {
                Ok(s) => {
                    std::process::ExitCode::from(u8::try_from(s.code().unwrap_or(1)).unwrap_or(1))
                }
                Err(e) => {
                    eprintln!("{program}: {e}");
                    std::process::ExitCode::from(127)
                }
            }
        }
        _ => {
            eprintln!("game-slot status | game-slot run -- <command> [args]");
            std::process::ExitCode::from(2)
        }
    }
}
