//! Which shader family each of the game's shaders belongs to
//! (`phyredefaultlitshader`, `phyrechrlitshader`, …), from the shader
//! clusters in the game's archives, keyed by the bytecode hash the scene
//! stream uses. Cached in a small file, since scanning the clusters takes
//! a few seconds.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// What the enhanced renderer does with a draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Field and battle scenery: textured, vertex-coloured (baked light).
    Scenery,
    /// Characters, monsters, objects: skinned or rigid, lit by the map's
    /// character lights.
    Character,
    Water,
    /// Scenery with scrolling texture coordinates.
    UvAnimation,
    Ice,
    /// Painted backdrops: drawn as they are.
    Prerendered,
    /// The game's blob and cone shadows.
    FakeShadow,
    /// Anything else: drawn with the game's own shader.
    Other,
}

impl Family {
    #[must_use]
    pub fn from_cluster(name: &str) -> Self {
        let name = name
            .split('#')
            .next()
            .unwrap_or(name)
            .trim_end_matches(".phyre");
        match name {
            "phyredefaultlitshader.fx" => Self::Scenery,
            "phyrechrlitshader.fx" => Self::Character,
            "phyrewatershader.fx" => Self::Water,
            "phyreuvanimationshader.fx" => Self::UvAnimation,
            "phyreiceshader.fx" => Self::Ice,
            "phyreprerenderedshader.fx" => Self::Prerendered,
            "chry0shadow.fx" | "chreasyshadow.fx" | "abmapshadercone.fx" => Self::FakeShadow,
            _ => Self::Other,
        }
    }
}

/// Bytecode hash → family.
#[derive(Debug, Default, Clone)]
pub struct Families {
    map: HashMap<u64, Family>,
}

impl Families {
    #[must_use]
    pub fn get(&self, hash: u64) -> Family {
        self.map.get(&hash).copied().unwrap_or(Family::Other)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Loads the cache next to `cache`, or scans the game's archive in
    /// `game_dir` and writes the cache. Empty when neither works.
    #[must_use]
    pub fn load(game_dir: Option<&Path>, cache: &Path) -> Self {
        if let Ok(text) = std::fs::read_to_string(cache) {
            let map: HashMap<u64, Family> = text
                .lines()
                .filter_map(|l| {
                    let (h, name) = l.split_once('\t')?;
                    Some((u64::from_str_radix(h, 16).ok()?, Family::from_cluster(name)))
                })
                .collect();
            if !map.is_empty() {
                return Self { map };
            }
        }
        let Some(dir) = game_dir else {
            return Self::default();
        };
        let mut lines = String::new();
        let mut map = HashMap::new();
        for vbf in ["FFX_Data.vbf", "FFX2_Data.vbf"] {
            let Ok(archive) = ffx_vbf::Archive::open(dir.join("data").join(vbf)) else {
                continue;
            };
            for e in archive.entries() {
                if !e.path.contains("/shaders/d3d11/") || !e.path.ends_with(".phyre") {
                    continue;
                }
                let Ok(bytes) = archive.read(e) else { continue };
                let name = e.path.rsplit('/').next().unwrap_or(&e.path);
                for (_, blob) in ffx_dxbc::find_all(&bytes) {
                    let h = ffx_dxbc::hash(blob);
                    if map.insert(h, Family::from_cluster(name)).is_none() {
                        let _ = writeln!(lines, "{h:016x}\t{name}");
                    }
                }
            }
        }
        if !map.is_empty() {
            let _ = std::fs::write(cache, lines);
        }
        Self { map }
    }
}

/// Where the game is: `FFX_GAME_DIR`, else the folder of the executable
/// when it has the game's archives.
#[must_use]
pub fn game_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from)
        && d.join("data").is_dir()
    {
        return Some(d);
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .filter(|d| d.join("data/FFX_Data.vbf").is_file())
}
