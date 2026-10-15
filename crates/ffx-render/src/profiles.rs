//! Named sets of render settings: `profiles/<name>.toml` next to the
//! renderer, the active one's name in `profiles/active`. The active
//! settings also stay in `ffx-render.toml` (what the renderer reads at
//! start and watches for edits); "Save" writes both.
//!
//! A scene can ask for a profile (the editor's `profile`): entering it
//! switches to that one, and scenes that ask for none go back to the
//! profile chosen by hand (`profiles/chosen`).

use std::path::{Path, PathBuf};

use crate::enhanced::Settings;

/// What the clipboard text of a shared profile starts with.
const SHARE_HEADER: &str = "# FFX renderer profile: ";

pub struct Profiles {
    dir: PathBuf,
    /// `ffx-render.toml`.
    file: PathBuf,
    stamp: Option<std::time::SystemTime>,
    /// The profiles' names, sorted.
    pub names: Vec<String>,
    pub active: String,
    /// The active profile as saved: what a setting resets to.
    pub saved: Settings,
    /// The profile chosen by hand: where scenes without their own go.
    pub chosen: String,
    /// The scene and profile last followed (to switch once per change).
    followed: Option<(String, String)>,
    /// A choice by hand stands until the scene or what it asks changes.
    hold: bool,
}

/// A profile name made safe as a file name.
fn clean(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control() && !"/\\:*?\"<>|".contains(*c))
        .collect();
    let s = s.trim_matches('.').trim().to_owned();
    if s.is_empty() { "Profile".into() } else { s }
}

fn read(path: &Path) -> Option<Settings> {
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

fn write(path: &Path, s: &Settings) -> Result<(), String> {
    let text = toml::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

impl Profiles {
    /// The profiles beside `file` (`ffx-render.toml`), whose settings are
    /// `current`. With no profiles yet, `current` becomes "Default".
    #[must_use]
    pub fn new(file: &Path, current: &Settings) -> Self {
        let dir = file.with_file_name("profiles");
        let _ = std::fs::create_dir_all(&dir);
        let mut p = Self {
            dir,
            file: file.to_path_buf(),
            stamp: None,
            names: Vec::new(),
            active: String::new(),
            saved: current.clone(),
            chosen: String::new(),
            followed: None,
            hold: false,
        };
        p.scan();
        let active = std::fs::read_to_string(p.dir.join("active"))
            .map(|s| s.trim().to_owned())
            .unwrap_or_default();
        if p.names.is_empty() {
            let _ = write(&p.path("Default"), current);
            p.scan();
            if p.names.is_empty() {
                // An unwritable folder: a profile in memory only.
                p.names.push("Default".into());
            }
        }
        p.active = if p.names.contains(&active) {
            active
        } else {
            p.names[0].clone()
        };
        p.stamp = p.file_stamp();
        let chosen = std::fs::read_to_string(p.dir.join("chosen"))
            .map(|s| s.trim().to_owned())
            .unwrap_or_default();
        p.chosen = if p.names.contains(&chosen) {
            chosen
        } else {
            p.active.clone()
        };
        p
    }

    /// The active profile is now the user's choice (switched, made,
    /// renamed by hand).
    pub fn chose(&mut self) {
        self.chosen.clone_from(&self.active);
        let _ = std::fs::write(self.dir.join("chosen"), &self.chosen);
        self.followed = None;
        self.hold = true;
    }

    /// Follows the scene: switches to the profile it asks for (`want`),
    /// or back to the chosen one, once per change of either. Unsaved
    /// changes are kept (no switch). What happened, if anything.
    pub fn follow(&mut self, scene: &str, want: Option<&str>, s: &mut Settings) -> Option<String> {
        let target = want
            .filter(|w| self.names.iter().any(|n| n == w))
            .unwrap_or(&self.chosen)
            .to_owned();
        let key = (scene.to_owned(), target.clone());
        if self.followed.as_ref() == Some(&key) {
            return None;
        }
        self.followed = Some(key);
        if std::mem::take(&mut self.hold) || target == self.active || target.is_empty() {
            return None;
        }
        if *s != self.saved {
            return Some(format!(
                "this scene uses \"{target}\"; unsaved changes: stayed on \"{}\"",
                self.active
            ));
        }
        Some(match self.switch(&target, s) {
            Ok(()) => format!("scene profile \"{target}\""),
            Err(e) => format!("failed: {e}"),
        })
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.toml"))
    }

    fn scan(&mut self) {
        self.names = std::fs::read_dir(&self.dir)
            .map(|d| {
                d.filter_map(|e| {
                    let p = e.ok()?.path();
                    (p.extension()? == "toml")
                        .then(|| p.file_stem()?.to_str().map(str::to_owned))?
                })
                .collect()
            })
            .unwrap_or_default();
        self.names.sort_by_key(|n| n.to_lowercase());
    }

    fn file_stamp(&self) -> Option<std::time::SystemTime> {
        std::fs::metadata(&self.file)
            .and_then(|m| m.modified())
            .ok()
    }

    /// A name not taken yet, from `want`.
    fn free_name(&self, want: &str) -> String {
        let base = clean(want);
        if !self.names.iter().any(|n| n.eq_ignore_ascii_case(&base)) {
            return base;
        }
        (2..=self.names.len() + 2)
            .map(|i| format!("{base} {i}"))
            .find(|n| !self.names.iter().any(|m| m.eq_ignore_ascii_case(n)))
            .expect("one of names + 1 numbers is free")
    }

    fn remember_active(&mut self) {
        let _ = std::fs::write(self.dir.join("active"), &self.active);
    }

    /// Edits to `ffx-render.toml` from outside apply at once (the file
    /// replaces both the settings and what they reset to).
    pub fn poll(&mut self, settings: &mut Settings) -> bool {
        let stamp = self.file_stamp();
        if stamp == self.stamp {
            return false;
        }
        self.stamp = stamp;
        match std::fs::read_to_string(&self.file).map(|t| toml::from_str::<Settings>(&t)) {
            Ok(Ok(s)) => {
                *settings = s.clone();
                self.saved = s;
                true
            }
            _ => false,
        }
    }

    /// Writes `s` as the active profile.
    ///
    /// # Errors
    /// When a file cannot be written.
    pub fn save(&mut self, s: &Settings) -> Result<(), String> {
        write(&self.path(&self.active), s)?;
        write(&self.file, s)?;
        self.stamp = self.file_stamp();
        self.saved = s.clone();
        Ok(())
    }

    /// Makes `name` active: its settings replace `s` (unsaved edits go).
    ///
    /// # Errors
    /// When the profile cannot be read.
    pub fn switch(&mut self, name: &str, s: &mut Settings) -> Result<(), String> {
        let loaded = read(&self.path(name)).ok_or_else(|| format!("cannot read {name}"))?;
        name.clone_into(&mut self.active);
        self.remember_active();
        *s = loaded.clone();
        self.saved = loaded;
        write(&self.file, s)?;
        self.stamp = self.file_stamp();
        Ok(())
    }

    /// A new profile with settings `s`, made active.
    ///
    /// # Errors
    /// When it cannot be written.
    pub fn create(&mut self, want: &str, s: &Settings) -> Result<String, String> {
        let name = self.free_name(want);
        write(&self.path(&name), s)?;
        self.scan();
        self.active.clone_from(&name);
        self.remember_active();
        self.saved = s.clone();
        write(&self.file, s)?;
        self.stamp = self.file_stamp();
        Ok(name)
    }

    /// Renames the active profile.
    ///
    /// # Errors
    /// When the file cannot be renamed.
    pub fn rename(&mut self, want: &str) -> Result<String, String> {
        if clean(want) == self.active {
            return Ok(self.active.clone());
        }
        let name = self.free_name(want);
        std::fs::rename(self.path(&self.active), self.path(&name)).map_err(|e| e.to_string())?;
        self.active.clone_from(&name);
        self.remember_active();
        self.scan();
        Ok(name)
    }

    /// Deletes the active profile (not the last one); the first other one
    /// becomes active.
    ///
    /// # Errors
    /// When it is the only one or the files fail.
    pub fn delete(&mut self, s: &mut Settings) -> Result<(), String> {
        if self.names.len() < 2 {
            return Err("the last profile stays".into());
        }
        std::fs::remove_file(self.path(&self.active)).map_err(|e| e.to_string())?;
        self.scan();
        let next = self.names[0].clone();
        self.switch(&next, s)
    }

    /// The text that shares the active profile's saved settings.
    #[must_use]
    pub fn share_text(&self) -> String {
        format!(
            "{SHARE_HEADER}{}\n{}",
            self.active,
            toml::to_string_pretty(&self.saved).unwrap_or_default()
        )
    }

    /// A new profile from shared text, made active.
    ///
    /// # Errors
    /// When the text is not a profile.
    pub fn import(&mut self, text: &str, s: &mut Settings) -> Result<String, String> {
        let name = text
            .lines()
            .next()
            .and_then(|l| l.trim().strip_prefix(SHARE_HEADER.trim_end()))
            .map_or("Imported", str::trim)
            .to_owned();
        let loaded: Settings = toml::from_str(text).map_err(|e| format!("not a profile: {e}"))?;
        *s = loaded.clone();
        self.create(&name, &loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_round_trip() {
        let dir = std::env::temp_dir().join(format!("ffx-profiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("ffx-render.toml");
        let mut s = Settings {
            exposure: 2.0,
            ..Settings::default()
        };
        let mut p = Profiles::new(&file, &s);
        assert_eq!(p.names, ["Default"]);
        assert_eq!(p.active, "Default");
        s.exposure = 3.0;
        p.create("Bright/er", &s).unwrap();
        assert_eq!(p.active, "Brighter");
        p.rename("Night").unwrap();
        assert_eq!(p.names, ["Default", "Night"]);
        let shared = p.share_text();
        p.switch("Default", &mut s).unwrap();
        assert!((s.exposure - 2.0).abs() < 1e-6);
        let name = p.import(&shared, &mut s).unwrap();
        assert_eq!(name, "Night 2");
        assert!((s.exposure - 3.0).abs() < 1e-6);
        p.delete(&mut s).unwrap();
        assert_eq!(p.names, ["Default", "Night"]);
        // Scenes that ask for a profile.
        p.chose();
        assert!(p.follow("bsil0000", None, &mut s).is_none());
        assert!(p.follow("cdsp0000", Some("Night"), &mut s).is_some());
        assert_eq!(p.active, "Night");
        assert!((s.exposure - 3.0).abs() < 1e-6);
        assert!(p.follow("cdsp0000", Some("Night"), &mut s).is_none());
        assert!(p.follow("bsil0000", Some("Gone"), &mut s).is_some());
        assert_eq!(p.active, "Default");
        s.exposure = 5.0;
        assert!(p.follow("cdsp0000", Some("Night"), &mut s).is_some());
        assert_eq!(p.active, "Default");
        let again = Profiles::new(&file, &Settings::default());
        assert_eq!(again.active, "Default");
        assert_eq!(again.chosen, "Default");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
