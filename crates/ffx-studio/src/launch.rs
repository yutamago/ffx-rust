//! Starting the game, with or without mods.

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::game::Game;

/// Whether an `FFX.exe` is running (the game refuses a second one).
#[must_use]
pub fn running() -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq FFX.exe", "/NH"])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("FFX.exe"))
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// The game's settings file, where the language is chosen.
fn settings_file() -> Option<std::path::PathBuf> {
    let docs = std::env::var_os("USERPROFILE").map(|p| Path::new(&p).join("Documents"))?;
    Some(docs.join("Square Enix/FINAL FANTASY X&X-2 HD Remaster/GameSetting.ini"))
}

/// Installs (or removes) the mod loader and starts `FFX.exe`.
///
/// # Errors
/// Fails when the game runs already, the loader is not bundled, or the
/// game cannot be started.
pub fn launch(game: &Game, with_mods: bool, hook: Option<&[u8]>, language: &str) -> Result<()> {
    if running() {
        bail!("the game is already running");
    }
    let dll = game.dir.join("DINPUT8.dll");
    if with_mods {
        let hook =
            hook.context("this build of FFX Studio does not carry the mod loader (DINPUT8.dll)")?;
        std::fs::write(&dll, hook)
            .context("installing the mod loader (DINPUT8.dll) in the game folder")?;
    } else if dll.exists() {
        std::fs::remove_file(&dll).context("removing the mod loader (DINPUT8.dll)")?;
    }
    // The game reads its language from GameSetting.ini (the Steam launcher
    // writes it); `us` is English there too.
    if let Some(ini) = settings_file() {
        let lang = match language {
            "us" | "en" => "en",
            "sp" => "es",
            other => other,
        };
        let text = std::fs::read_to_string(&ini).unwrap_or_default();
        let mut lines: Vec<String> = text
            .lines()
            .filter(|l| !l.starts_with("Language="))
            .map(str::to_owned)
            .collect();
        lines.insert(0, format!("Language={lang}"));
        if let Some(dir) = ini.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&ini, lines.join("\r\n") + "\r\n");
    }
    #[cfg(windows)]
    {
        std::process::Command::new(game.dir.join("FFX.exe"))
            .current_dir(&game.dir)
            .spawn()
            .context("starting FFX.exe")?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        bail!("the game starts from the Windows app (here: tools/run-game.sh)")
    }
}

/// Opens a folder (or a file's folder) in the file manager.
///
/// # Errors
/// Fails if the file manager cannot be started.
pub fn open_folder(path: &Path) -> Result<()> {
    let dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    #[cfg(windows)]
    let program = "explorer";
    #[cfg(not(windows))]
    let program = "xdg-open";
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .context("opening the folder")?;
    Ok(())
}
