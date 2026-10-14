//! The game folder: its archive, its language data and its mods.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ffx_text::{Charset, Language};
use ffx_vbf::Archive;

/// An opened game.
pub struct Game {
    pub dir: PathBuf,
    pub archive: Archive,
    pub language: Language,
    pub charset: Charset,
}

impl Game {
    /// Opens the game in `dir` (the folder with `FFX.exe`).
    ///
    /// # Errors
    /// Fails if the folder has no `data/FFX_Data.vbf` or the language's
    /// character table is missing.
    pub fn open(dir: &Path, language: &str) -> Result<Self> {
        let vbf = dir.join("data/FFX_Data.vbf");
        if !vbf.is_file() {
            bail!(
                "{} is not the game folder: it has no data\\FFX_Data.vbf (choose the folder with FFX.exe)",
                dir.display()
            );
        }
        let archive = Archive::open(&vbf).with_context(|| format!("opening {}", vbf.display()))?;
        let language = Language::from_code(language).unwrap_or(Language::English);
        let table = read_archive(&archive, &language.charset_path())?;
        let charset = Charset::from_table(&table, language).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self {
            dir: dir.to_path_buf(),
            archive,
            language,
            charset,
        })
    }

    #[must_use]
    pub fn mods_dir(&self) -> PathBuf {
        self.dir.join("mods")
    }

    /// A file as the game would see it with the `overlay` mod: the mod's
    /// copy if it has one, else the archive's.
    ///
    /// # Errors
    /// Fails if neither has the file.
    pub fn read(&self, overlay: Option<&Path>, path: &str) -> Result<Vec<u8>> {
        if let Some(dir) = overlay {
            let file = dir.join(path);
            if file.is_file() {
                return Ok(std::fs::read(&file)?);
            }
        }
        read_archive(&self.archive, path)
    }

    /// The archive's own copy of a file.
    ///
    /// # Errors
    /// Fails if the archive has no such file.
    pub fn original(&self, path: &str) -> Result<Vec<u8>> {
        read_archive(&self.archive, path)
    }
}

fn read_archive(archive: &Archive, path: &str) -> Result<Vec<u8>> {
    let entry = archive
        .find(path)
        .with_context(|| format!("the game has no file {path}"))?;
    Ok(archive.read(entry)?)
}

/// Writes `bytes` as `path` into the mod folder `dir`.
///
/// # Errors
/// Fails on paths that leave the folder, or when writing fails.
pub fn write_mod_file(dir: &Path, path: &str, bytes: &[u8]) -> Result<PathBuf> {
    let file = dir.join(safe_relative(path)?);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&file, bytes)?;
    Ok(file)
}

/// An archive path as a relative path, refusing `..` and absolute parts.
///
/// # Errors
/// Fails on paths that would leave the folder they are joined to.
pub fn safe_relative(path: &str) -> Result<PathBuf> {
    let rel = PathBuf::from(path);
    if rel
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail!("{path} is not a plain relative path");
    }
    Ok(rel)
}

/// The game folder of a Steam install, if one is found.
#[must_use]
pub fn find_install() -> Option<PathBuf> {
    const GAME: &str = "FINAL FANTASY FFX&FFX-2 HD Remaster";
    let mut libraries: Vec<PathBuf> = Vec::new();
    for steam in [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
        r"D:\Steam",
        r"D:\SteamLibrary",
        r"E:\SteamLibrary",
    ] {
        let steam = PathBuf::from(steam);
        libraries.push(steam.clone());
        // Other libraries are listed in libraryfolders.vdf as "path" values.
        if let Ok(vdf) = std::fs::read_to_string(steam.join("steamapps/libraryfolders.vdf")) {
            for line in vdf.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("\"path\"") {
                    let p = rest.trim().trim_matches('"').replace(r"\\", r"\");
                    libraries.push(PathBuf::from(p));
                }
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        libraries.push(PathBuf::from(home).join(".steam/steam"));
    }
    if let Some(dir) = std::env::var_os("FFX_GAME_DIR") {
        return Some(PathBuf::from(dir)).filter(|d| d.join("data/FFX_Data.vbf").is_file());
    }
    libraries
        .into_iter()
        .map(|l| l.join("steamapps/common").join(GAME))
        .find(|d| d.join("data/FFX_Data.vbf").is_file())
}
