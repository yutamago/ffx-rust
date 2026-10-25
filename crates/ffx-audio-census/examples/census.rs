//! The FMOD boundary census:
//!
//! ```text
//! census [<fmod.log>...]
//! ```
//!
//! Prints every FMOD import of `$FFX_GAME_DIR/FFX.exe` with its thunk and
//! call sites, then, for each call log (`FFX_FMOD_LOG`, written by
//! `ffx-hook`), its threads and a table of the calls per function.

use std::path::PathBuf;

use ffx_audio_census::{boundary, demangle, fmodlog::Log, summary};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logs: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if let Ok(dir) = std::env::var("FFX_GAME_DIR") {
        let exe = std::fs::read(PathBuf::from(dir).join("FFX.exe"))?;
        let imports = boundary::imports(&exe)?;
        println!("## FMOD imports of FFX.exe ({})\n", imports.len());
        println!("| import | slot | thunk | calls |\n|---|---|---|---|");
        for i in &imports {
            let thunks: Vec<String> = i.thunks.iter().map(|t| format!("{t:#010x}")).collect();
            let calls: Vec<String> = i.calls.iter().map(|c| format!("{c:#010x}")).collect();
            println!(
                "| `{}` ({}) | {:#010x} | {} | {} |",
                demangle::pretty(&i.name),
                i.dll,
                i.slot,
                thunks.join(" "),
                calls.join(" ")
            );
        }
        println!();
    }
    for path in logs {
        let log = Log::parse(&std::fs::read_to_string(&path)?)?;
        println!(
            "## {}: {} calls, {} threads, {} functions patched\n",
            path.display(),
            log.calls.len(),
            log.threads.len(),
            log.functions.len()
        );
        for (tid, start) in &log.threads {
            let n = log.calls.iter().filter(|c| c.thread == *tid).count();
            println!("- thread {tid} (start {start}): {n} calls");
        }
        println!();
        println!("{}", summary::table(&log));
    }
    Ok(())
}
