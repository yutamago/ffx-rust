//! The program's own problems shown to the player (docs/host.md, "Native
//! platform"): always on stderr, and in a desktop dialog where the system
//! has one: on macOS `osascript`'s `display alert` (always there), on Linux
//! `zenity` (GNOME and most desktops; `SteamOS` has it), else `kdialog`
//! (KDE). Nothing is linked: the dialog is a separate program, so no GTK or
//! D-Bus library is a build dependency, and a system without either tool
//! still has stderr.

use std::process::Command;

/// Shows `text` under `title`; returns when the player closed it.
pub fn show(title: &str, text: &str, warning: bool) {
    eprintln!("{title}: {text}");
    if std::env::var_os("FFX_HARNESS").is_some() {
        // never a window in a harness game (the user's screen)
        return;
    }
    for mut c in commands(title, text, warning) {
        if c.status().is_ok_and(|s| s.code().is_some()) {
            return;
        }
    }
}

/// The dialog programs to try, in order.
fn commands(title: &str, text: &str, warning: bool) -> Vec<Command> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        let mut c = Command::new("osascript");
        c.args([
            "-e",
            "on run argv",
            "-e",
            &format!(
                "display alert (item 1 of argv) message (item 2 of argv) as {}",
                if warning { "warning" } else { "critical" }
            ),
            "-e",
            "end run",
            title,
            text,
        ]);
        out.push(c);
    } else {
        let mut z = Command::new("zenity");
        z.args([
            if warning { "--warning" } else { "--error" },
            "--no-markup",
            "--title",
            title,
            "--text",
            text,
        ]);
        out.push(z);
        let mut k = Command::new("kdialog");
        k.args([
            "--title",
            title,
            if warning { "--sorry" } else { "--error" },
            text,
        ]);
        out.push(k);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_text_is_an_argument_never_a_script() {
        let cmds = super::commands("t\"", "a \" b", true);
        assert!(!cmds.is_empty());
        for c in &cmds {
            let args: Vec<_> = c.get_args().collect();
            assert!(args.iter().any(|a| *a == "a \" b"), "{args:?}");
        }
    }
}
