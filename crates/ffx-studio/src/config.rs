//! The app's settings, a small JSON file in the user's app data folder.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The folder with `FFX.exe`.
    pub game_dir: Option<PathBuf>,
    /// Text language for the editors (`us`, `jp`, …; see `ffx_text::Language`).
    pub language: String,
    /// The mod edits go into: a folder name under `<game>/mods/`.
    pub active_mod: Option<String>,
    /// Shows the advanced editors and raw fields.
    pub advanced: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_dir: None,
            language: "us".into(),
            active_mod: None,
            advanced: false,
        }
    }
}

impl Config {
    /// The settings at `path`, or the defaults.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Writes the settings.
    ///
    /// # Errors
    /// Fails if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}
