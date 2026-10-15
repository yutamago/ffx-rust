//! The editor's changes to the game's look: for all scenes and per scene
//! (a scene is a map, by the event file the game opened for it). A
//! scene's changes go on top of the general ones: its lights add to
//! theirs, its sun and materials replace theirs where it sets them.
//!
//! The working set is `edits.toml` next to the renderer; `edits/*.toml`
//! are files to share, and taking one in shows which scenes it changes.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::enhanced::lights::{GlowEdit, Light, Sun};
use crate::enhanced::materials::Material;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneEdits {
    /// A name the user gives the scene.
    pub label: String,
    pub sun: Sun,
    pub lights: Vec<Light>,
    pub glows: Vec<GlowEdit>,
    /// By colour texture hash (16 hex digits).
    pub materials: BTreeMap<String, Material>,
}

impl SceneEdits {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sun.is_empty()
            && self.lights.is_empty()
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
    pub lights: Vec<Light>,
    pub sun: Sun,
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

    /// Writes the changes to `path` (scenes without any are left out).
    ///
    /// # Errors
    /// When the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let mut out = self.clone();
        out.scenes
            .retain(|_, s| !s.is_empty() || !s.label.is_empty());
        let text = toml::to_string_pretty(&out).map_err(|e| e.to_string())?;
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

    /// What the renderer uses in `scene`.
    #[must_use]
    pub fn effective(&self, scene: &str) -> Effective {
        let local = self.scenes.get(scene);
        let mut lights = self.global.lights.clone();
        let mut glows: Vec<GlowEdit> = Vec::new();
        let mut materials = std::collections::HashMap::new();
        let mut sun = self.global.sun.clone();
        for (k, m) in &self.global.materials {
            if let Ok(h) = u64::from_str_radix(k, 16) {
                materials.insert(h, m.clone());
            }
        }
        glows.extend(self.global.glows.iter().cloned());
        if let Some(l) = local {
            lights.extend(l.lights.iter().cloned());
            // A scene's glow edits come first: they win where both match.
            glows.splice(0..0, l.glows.iter().cloned());
            for (k, m) in &l.materials {
                if let Ok(h) = u64::from_str_radix(k, 16) {
                    materials.insert(h, m.clone());
                }
            }
            if l.sun.direction.is_some() {
                sun.direction = l.sun.direction;
            }
            if l.sun.color.is_some() {
                sun.color = l.sun.color;
            }
            if l.sun.scale.is_some() {
                sun.scale = l.sun.scale;
            }
        }
        Effective {
            lights,
            sun,
            glows,
            materials,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_go_on_top_and_files_compare() {
        let mut ours = Edits::default();
        ours.global.lights.push(Light::default());
        ours.global.sun.scale = Some(2.0);
        ours.global.materials.insert(key(7), Material::default());
        let deck = ours.scene_mut("cdsp0000");
        deck.lights.push(Light {
            name: "lamp".into(),
            ..Light::default()
        });
        deck.sun.scale = Some(0.5);
        let e = ours.effective("cdsp0000");
        assert_eq!(e.lights.len(), 2);
        assert_eq!(e.sun.scale, Some(0.5));
        assert!(e.materials.contains_key(&7));
        assert_eq!(ours.effective("other").lights.len(), 1);

        let mut theirs = ours.clone();
        theirs.scene_mut("cdsp0000").sun.scale = Some(1.5);
        theirs.scene_mut("bika00").lights.push(Light::default());
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
        assert_eq!(ours.scenes["cdsp0000"].sun.scale, Some(0.5));

        let dir = std::env::temp_dir().join(format!("ffx-edits-{}", std::process::id()));
        let path = dir.join("edits.toml");
        ours.save(&path).unwrap();
        let (back, err) = Edits::load(&path);
        assert!(err.is_none());
        assert_eq!(back, ours);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
