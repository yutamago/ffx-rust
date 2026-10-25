//! Shader families: which effect file (`shaders/d3d11/<effect>.fx#<md5>.phyre`
//! in `FFX_Data.vbf`) each captured shader comes from, by `ffx-drawdiff`'s
//! shader key (the bytecode's FNV-1a 64).

use std::collections::HashMap;
use std::path::Path;

use ffx_drawdiff::key::{self, Key};

/// Shader key -> effect file name (`phyredefaultlitshader.fx`), and the
/// effect shaders' table from `vfxShader_fx.txt`.
#[derive(Debug, Clone, Default)]
pub struct Families {
    pub by_key: HashMap<Key, String>,
    /// VSP flags -> effect file name, lower case (`vfxadditiveshader.fx`).
    pub vsp: HashMap<u32, String>,
}

/// The effect shader table: `ffx_data/gamedata/ps3data/shaders/vfxShader_fx.txt`,
/// read by `VfxShaderManager_loadVfxShader` (`0x006c0fc0`); each line
/// `Name.fx.phyre:VSP_A|VSP_B` gives the shader the `VfxShaderManager`
/// hands a geometry whose flags (`+0x3c`) are the OR of the named bits
/// (`VfxShaderManager_loadShader` `0x006c0a70`). The bits' values are in
/// the file's own comment block (`VSP_NAME = 0x...`).
#[must_use]
pub fn parse_vsp_table(text: &str) -> HashMap<u32, String> {
    let mut bits: HashMap<String, u32> = HashMap::new();
    for line in text.lines() {
        let l = line.trim_start_matches(['/', ' ', '\t']);
        if let Some((name, rest)) = l.split_once('=')
            && let Some(name) = name.trim().strip_prefix("VSP_")
            && let Some(hex) = rest.trim().strip_prefix("0x")
        {
            let digits: String = hex.chars().take_while(char::is_ascii_hexdigit).collect();
            if let Ok(x) = u32::from_str_radix(&digits, 16) {
                bits.insert(name.trim().to_owned(), x);
            }
        }
    }
    let mut out = HashMap::new();
    for line in text.lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let Some((file, props)) = line.trim().split_once(':') else {
            continue;
        };
        let mut flags = 0;
        let mut ok = true;
        for p in props.split('|') {
            match p.trim().strip_prefix("VSP_").and_then(|n| bits.get(n)) {
                Some(b) => flags |= b,
                None => ok = false,
            }
        }
        if ok {
            let name = file.trim().to_lowercase();
            let name = name.strip_suffix(".phyre").unwrap_or(&name).to_owned();
            out.entry(flags).or_insert(name);
        }
    }
    out
}

/// Whether `path` is a shader cluster.
#[must_use]
pub fn is_shader_cluster(path: &str) -> bool {
    path.contains("/shaders/") && (path.contains(".fx#") || path.ends_with(".fx.phyre"))
}

impl Families {
    /// Indexes every shader cluster of `<game>/data/FFX_Data.vbf`.
    ///
    /// # Errors
    /// When the archive cannot be read.
    pub fn from_game(game: &Path) -> Result<Self, ffx_vbf::Error> {
        let archive = ffx_vbf::Archive::open(game.join("data/FFX_Data.vbf"))?;
        let mut f = Self::default();
        for e in archive.entries() {
            if e.path.ends_with("shaders/vfxshader_fx.txt") {
                let bytes = archive.read(e)?;
                f.vsp = parse_vsp_table(&String::from_utf8_lossy(&bytes));
                continue;
            }
            if !e.path.ends_with(".phyre") || !is_shader_cluster(&e.path) {
                continue;
            }
            let bytes = archive.read(e)?;
            f.add_cluster(&e.path, &bytes);
        }
        Ok(f)
    }

    /// Indexes one shader cluster's DXBC blobs under its effect name.
    pub fn add_cluster(&mut self, path: &str, bytes: &[u8]) {
        let file = path.rsplit('/').next().unwrap_or(path);
        let name = file.split('#').next().unwrap_or(file);
        let name = name.strip_suffix(".phyre").unwrap_or(name).to_owned();
        for (_, blob) in ffx_dxbc::find_all(bytes) {
            self.by_key
                .entry(key::shader(blob))
                .or_insert_with(|| name.clone());
        }
    }

    /// The family of a shader key, or `?`.
    #[must_use]
    pub fn name(&self, k: Key) -> &str {
        self.by_key.get(&k).map_or("?", String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vsp_table_ors_the_named_bits() {
        let text = "// \tVSP_ZMAT\t\t= 0x00000001,\n\
                    // \tVSP_ALPHA_MODULATE = 0x00000000, // src*srcAlpha\n\
                    // \tVSP_ALPHA_ADDITIVE = 0x00000002,\n\
                    // \tVSP_ENABLE_TEXTURE = 0x00000008,\n\
                    \n\
                    VfxShapeShader.fx.phyre:VSP_ZMAT|VSP_ALPHA_MODULATE|VSP_ENABLE_TEXTURE\n\
                    VfxAdditiveShader.fx.phyre:VSP_ALPHA_ADDITIVE|VSP_ENABLE_TEXTURE\n\
                    VfxBad.fx.phyre:VSP_UNKNOWN\n";
        let t = parse_vsp_table(text);
        assert_eq!(t.get(&9).map(String::as_str), Some("vfxshapeshader.fx"));
        assert_eq!(
            t.get(&0xa).map(String::as_str),
            Some("vfxadditiveshader.fx")
        );
        assert_eq!(t.len(), 2, "a line with an unknown bit is left out");
    }
}
