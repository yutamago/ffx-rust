//! Shader families: the effect file each DXBC blob of the archive's shader
//! clusters (`shaders/d3d11/<effect>.fx[#<md5>].phyre`) comes from, by
//! `ffx-drawdiff`'s shader key.

use std::collections::{BTreeSet, HashMap};

/// Whether `path` is a shader cluster.
#[must_use]
pub fn is_shader_cluster(path: &str) -> bool {
    path.contains("/shaders/") && (path.contains(".fx#") || path.ends_with(".fx.phyre"))
}

/// The effect name of a shader cluster path: `copybuffer.fx` for
/// `.../shaders/d3d11/copybuffer.fx.phyre` or `copybuffer.fx#<md5>.phyre`.
#[must_use]
pub fn effect_name(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let base = file.split('#').next().unwrap_or(file);
    base.strip_suffix(".phyre").unwrap_or(base).to_owned()
}

/// Shader keys to the effects that hold that bytecode (a vertex shader is
/// often shared by several effects).
#[derive(Debug, Clone, Default)]
pub struct ShaderNames {
    by_key: HashMap<u64, BTreeSet<String>>,
}

impl ShaderNames {
    /// Indexes every shader cluster of the archive.
    ///
    /// # Errors
    /// When the archive cannot be read.
    pub fn from_archive(archive: &ffx_vbf::Archive) -> Result<Self, ffx_vbf::Error> {
        let mut names = Self::default();
        for e in archive.entries() {
            if is_shader_cluster(&e.path) {
                names.add_cluster(&e.path, &archive.read(e)?);
            }
        }
        Ok(names)
    }

    /// Indexes one shader cluster's blobs.
    pub fn add_cluster(&mut self, path: &str, bytes: &[u8]) {
        let name = effect_name(path);
        for (_, blob) in ffx_dxbc::find_all(bytes) {
            self.by_key
                .entry(ffx_drawdiff::key::shader(blob))
                .or_default()
                .insert(name.clone());
        }
    }

    /// The effects holding a shader, `?` when none does.
    #[must_use]
    pub fn family(&self, key: u64) -> String {
        self.by_key.get(&key).map_or_else(
            || "?".to_owned(),
            |s| s.iter().cloned().collect::<Vec<_>>().join("|"),
        )
    }

    /// Whether some effect holding `key` has a name starting with `prefix`.
    #[must_use]
    pub fn is(&self, key: u64, prefix: &str) -> bool {
        self.by_key
            .get(&key)
            .is_some_and(|s| s.iter().any(|n| n.starts_with(prefix)))
    }

    /// Number of distinct shaders indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_names_drop_the_variant_hash() {
        assert_eq!(
            effect_name("ffx_data/gamedata/ps3data/shaders/d3d11/copybuffer.fx.phyre"),
            "copybuffer.fx"
        );
        assert_eq!(
            effect_name("x/shaders/d3d11/phyreglow.fx#0123abcd.phyre"),
            "phyreglow.fx"
        );
        assert!(is_shader_cluster("a/shaders/d3d11/x.fx#00.phyre"));
        assert!(!is_shader_cluster("a/map/x.dae.phyre"));
    }
}
