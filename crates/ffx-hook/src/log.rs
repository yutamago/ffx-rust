//! Appends lines to `ffx-hook.log` next to the running executable.

use std::io::Write;

/// Writes one line to the log; errors are ignored.
pub fn log(line: &str) {
    let path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("ffx-hook.log")));
    if let Some(path) = path
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}
