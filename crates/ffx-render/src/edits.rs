//! The editor's changes to the game's look: for all scenes and per scene
//! (a scene is a map, by the event file the game opened for it). A scene
//! with lights of its own (a rig) uses those, the others the default rig
//! (the general part's, else the built-in one); its materials and glow
//! changes go on top of the general ones.
//!
//! The working set is `edits.toml` next to the renderer; `edits/*.toml`
//! are files to share, and taking one in shows which scenes it changes.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::enhanced::lights::GlowEdit;
use crate::enhanced::materials::Material;
use crate::enhanced::rig::Rig;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneEdits {
    /// A name the user gives the scene.
    pub label: String,
    /// The settings profile to use here (empty: none of its own); for
    /// all scenes: the one for scenes without their own (empty: the
    /// profile chosen by hand).
    pub profile: String,
    /// Its lights (for all scenes: the default rig); none: the default.
    pub rig: Option<Rig>,
    pub glows: Vec<GlowEdit>,
    /// By colour texture hash (16 hex digits).
    pub materials: BTreeMap<String, Material>,
}

impl SceneEdits {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rig.is_none()
            && self.profile.is_empty()
            && self.glows.is_empty()
            && self.materials.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Edits {
    pub global: SceneEdits,
    pub scenes: BTreeMap<String, SceneEdits>,
}

/// A texture hash as a key.
#[must_use]
pub fn key(hash: u64) -> String {
    format!("{hash:016x}")
}

/// One part of an edits file: the general changes or a scene's.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    Global,
    Scene(String),
}

impl Part {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Global => "all scenes",
            Self::Scene(s) => s,
        }
    }
}

/// How a part of another file stands to ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// We have nothing there.
    New,
    Differs,
    Same,
}

/// What the renderer uses for one scene.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Effective {
    pub rig: Rig,
    pub glows: Vec<GlowEdit>,
    pub materials: std::collections::HashMap<u64, Material>,
}

impl Edits {
    /// `path`, or nothing when it is missing or broken (the reason).
    #[must_use]
    pub fn load(path: &Path) -> (Self, Option<String>) {
        match std::fs::read_to_string(path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(e) => (e, None),
                Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            },
            Err(_) => (Self::default(), None),
        }
    }

    /// Without the scenes that have no changes.
    #[must_use]
    pub fn trimmed(&self) -> Self {
        let mut out = self.clone();
        out.scenes
            .retain(|_, s| !s.is_empty() || !s.label.is_empty());
        out
    }

    /// Writes the changes to `path` (scenes without any are left out).
    ///
    /// # Errors
    /// When the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string_pretty(&self.trimmed()).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    /// The changes of `part`.
    #[must_use]
    pub fn part(&self, part: &Part) -> Option<&SceneEdits> {
        match part {
            Part::Global => Some(&self.global),
            Part::Scene(s) => self.scenes.get(s),
        }
    }

    /// The scene's changes, made when missing.
    pub fn scene_mut(&mut self, scene: &str) -> &mut SceneEdits {
        self.scenes.entry(scene.to_owned()).or_default()
    }

    /// Every part of `other` that has changes, and how it stands to ours.
    #[must_use]
    pub fn compare(&self, other: &Self) -> Vec<(Part, Status)> {
        let mut out = Vec::new();
        let mut look = |part: Part, theirs: &SceneEdits| {
            if theirs.is_empty() {
                return;
            }
            let status = match self.part(&part) {
                Some(ours) if ours == theirs => Status::Same,
                Some(ours) if !ours.is_empty() => Status::Differs,
                _ => Status::New,
            };
            out.push((part, status));
        };
        look(Part::Global, &other.global);
        for (k, s) in &other.scenes {
            look(Part::Scene(k.clone()), s);
        }
        out
    }

    /// Takes `parts` from `other` in place of ours.
    pub fn take(&mut self, other: &Self, parts: &[Part]) {
        for p in parts {
            let Some(theirs) = other.part(p) else {
                continue;
            };
            match p {
                Part::Global => self.global = theirs.clone(),
                Part::Scene(s) => {
                    self.scenes.insert(s.clone(), theirs.clone());
                }
            }
        }
    }

    /// The profile `scene` asks for (its own, else the one for all
    /// scenes).
    #[must_use]
    pub fn profile_for(&self, scene: &str) -> Option<&str> {
        self.scenes
            .get(scene)
            .map(|s| s.profile.as_str())
            .filter(|p| !p.is_empty())
            .or_else(|| Some(self.global.profile.as_str()).filter(|p| !p.is_empty()))
    }

    /// What the renderer uses in `scene`.
    #[must_use]
    pub fn effective(&self, scene: &str) -> Effective {
        let local = self.scenes.get(scene);
        let rig = self.rig_for(scene).clone();
        let mut glows: Vec<GlowEdit> = Vec::new();
        let mut materials = std::collections::HashMap::new();
        for (k, m) in &self.global.materials {
            if let Ok(h) = u64::from_str_radix(k, 16) {
                materials.insert(h, m.clone());
            }
        }
        glows.extend(self.global.glows.iter().cloned());
        if let Some(l) = local {
            // A scene's glow edits come first: they win where both match.
            glows.splice(0..0, l.glows.iter().cloned());
            for (k, m) in &l.materials {
                if let Ok(h) = u64::from_str_radix(k, 16) {
                    materials.insert(h, m.clone());
                }
            }
        }
        Effective {
            rig,
            glows,
            materials,
        }
    }

    /// The default rig: the general part's, else the built-in one.
    #[must_use]
    pub fn default_rig(&self) -> &Rig {
        static BUILT_IN: std::sync::OnceLock<Rig> = std::sync::OnceLock::new();
        self.global
            .rig
            .as_ref()
            .unwrap_or_else(|| BUILT_IN.get_or_init(Rig::default))
    }

    /// The rig `scene` uses: its own, else the default.
    #[must_use]
    pub fn rig_for(&self, scene: &str) -> &Rig {
        self.scenes
            .get(scene)
            .and_then(|s| s.rig.as_ref())
            .unwrap_or_else(|| self.default_rig())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_go_on_top_and_files_compare() {
        use crate::enhanced::rig::{LightType, SceneLight};
        let mut ours = Edits::default();
        ours.global.rig = Some(Rig {
            exposure: 11.0,
            ..Rig::default()
        });
        ours.global.materials.insert(key(7), Material::default());
        let mut own = Rig::default();
        own.lights.push(SceneLight::new(LightType::Point));
        ours.scene_mut("cdsp0000").rig = Some(own);
        let e = ours.effective("cdsp0000");
        assert_eq!(e.rig.lights.len(), 5);
        assert!(e.materials.contains_key(&7));
        assert!((ours.effective("other").rig.exposure - 11.0).abs() < 1e-6);
        assert_eq!(ours.profile_for("cdsp0000"), None);
        ours.global.profile = "Day".into();
        ours.scene_mut("cdsp0000").profile = "Night".into();
        assert_eq!(ours.profile_for("cdsp0000"), Some("Night"));
        assert_eq!(ours.profile_for("bsil0000"), Some("Day"));

        let mut theirs = ours.clone();
        theirs.scene_mut("cdsp0000").rig = Some(Rig::default());
        theirs.scene_mut("bika00").rig = Some(Rig::default());
        let c = theirs.compare(&ours);
        assert!(c.is_empty() || c.iter().all(|(_, s)| *s != Status::New));
        let c = ours.compare(&theirs);
        assert_eq!(
            c,
            vec![
                (Part::Global, Status::Same),
                (Part::Scene("bika00".into()), Status::New),
                (Part::Scene("cdsp0000".into()), Status::Differs),
            ]
        );
        ours.take(&theirs, &[Part::Scene("bika00".into())]);
        assert!(ours.scenes.contains_key("bika00"));
        assert_eq!(
            ours.scenes["cdsp0000"].rig.as_ref().map(|r| r.lights.len()),
            Some(5)
        );

        let dir = std::env::temp_dir().join(format!("ffx-edits-{}", std::process::id()));
        let path = dir.join("edits.toml");
        ours.save(&path).unwrap();
        let (back, err) = Edits::load(&path);
        assert!(err.is_none());
        assert_eq!(back, ours);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
