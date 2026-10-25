//! Where a captured shader or texture comes from: the archive's shader
//! and texture clusters, and the DXBC blobs `FFX.exe` carries itself (the
//! Iggy `GDraw` shaders, `docs/formats/ui-draw.md`).

use std::collections::HashMap;

use ffx_drawdiff::Key;

/// FNV-1a 64 of bytes: the texel content key (as `ffx-drawdiff` keys
/// texture slices, without the description).
#[must_use]
pub fn content(bytes: &[u8]) -> Key {
    ffx_drawdiff::key::Fnv::new().bytes(bytes).finish()
}

/// A texture level in the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureSource {
    pub path: String,
    /// The file's level the captured texture's top level equals.
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub levels: u32,
}

/// Shaders by key, textures by top-level content.
#[derive(Debug, Default)]
pub struct Index {
    /// Shader key to its origins: the effect names of the shader clusters
    /// holding that bytecode (`menudefaultshader.fx`; one vertex shader
    /// can sit in several clusters), or `FFX.exe+0x<offset>`.
    pub shaders: HashMap<Key, Vec<String>>,
    /// Content key of one level's texels (rows as stored) to the files.
    pub textures: HashMap<Key, Vec<TextureSource>>,
    /// Clusters that did not parse.
    pub failed: Vec<(String, String)>,
}

/// Whether an archive path is a shader cluster
/// (`shaders/d3d11/<effect>.fx#<md5>.phyre` and the like).
#[must_use]
pub fn is_shader_cluster(path: &str) -> bool {
    path.contains("/shaders/") && path.ends_with(".phyre") && !path.ends_with(".dds.phyre")
}

impl Index {
    /// Indexes the archive's shader clusters and the texture clusters
    /// whose path `textures` accepts.
    ///
    /// # Errors
    /// When the archive cannot be read.
    pub fn add_archive(
        &mut self,
        archive: &ffx_vbf::Archive,
        textures: impl Fn(&str) -> bool,
    ) -> Result<(), ffx_vbf::Error> {
        for e in archive.entries() {
            let shader = is_shader_cluster(&e.path);
            let texture = e.path.ends_with(".dds.phyre") && textures(&e.path);
            if !shader && !texture {
                continue;
            }
            let bytes = archive.read(e)?;
            if shader {
                self.add_shaders(&e.path, &bytes);
            } else {
                self.add_texture(&e.path, bytes);
            }
        }
        Ok(())
    }

    /// Every DXBC blob of a shader cluster, under the effect's name.
    pub fn add_shaders(&mut self, path: &str, bytes: &[u8]) {
        let file = path.rsplit('/').next().unwrap_or(path);
        let name = file.split('#').next().unwrap_or(file);
        let name = name.strip_suffix(".phyre").unwrap_or(name);
        for (_, blob) in ffx_dxbc::find_all(bytes) {
            let list = self.shaders.entry(ffx_dxbc::hash(blob)).or_default();
            if !list.iter().any(|n| n == name) {
                list.push(name.to_owned());
            }
        }
    }

    /// Every DXBC blob of a PE image, as `<label>@0x<address>` (the
    /// virtual address its file offset maps to; `<label>+0x<offset>` when
    /// no section holds it).
    pub fn add_image(&mut self, label: &str, bytes: &[u8]) {
        let sections = pe_sections(bytes);
        for (at, blob) in ffx_dxbc::find_all(bytes) {
            let name = sections
                .iter()
                .find(|s| (s.1..s.1 + s.2).contains(&at))
                .map_or_else(
                    || format!("{label}+{at:#x}"),
                    |s| format!("{label}@{:#010x}", s.0 + (at - s.1)),
                );
            self.shaders
                .entry(ffx_dxbc::hash(blob))
                .or_default()
                .push(name);
        }
    }

    /// Every level of a texture cluster's first face, by its texels.
    pub fn add_texture(&mut self, path: &str, bytes: Vec<u8>) {
        let t = match ffx_phyre::Texture::parse(bytes) {
            Ok(t) => t,
            Err(e) => {
                self.failed.push((path.to_owned(), e.to_string()));
                return;
            }
        };
        let (mut w, mut h, mut at) = (t.width, t.height, 0);
        for level in 0..t.mip_levels {
            let len = t.format.level_size(w, h);
            let Some(texels) = t.pixels().get(at..at + len) else {
                break;
            };
            let list = self.textures.entry(content(texels)).or_default();
            let s = TextureSource {
                path: path.to_owned(),
                level,
                width: t.width,
                height: t.height,
                levels: t.mip_levels,
            };
            if !list.contains(&s) {
                list.push(s);
            }
            at += len;
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
    }

    /// The origin of a shader (its first), `none` for no shader (key 0),
    /// or `?`.
    #[must_use]
    pub fn shader(&self, key: Key) -> &str {
        if key == 0 {
            return "none";
        }
        self.shaders
            .get(&key)
            .and_then(|l| l.first())
            .map_or("?", String::as_str)
    }

    /// The origin of a draw's vertex shader: the pixel shader's cluster
    /// when it holds the bytecode too, else the first.
    #[must_use]
    pub fn vertex_shader(&self, vs: Key, ps: Key) -> &str {
        let p = self.shader(ps);
        self.shaders
            .get(&vs)
            .and_then(|l| l.iter().find(|n| *n == p).or_else(|| l.first()))
            .map_or("?", String::as_str)
    }
}

/// A PE image's sections as (virtual address with the image base, file
/// offset, raw size); empty when the headers do not parse.
#[must_use]
pub fn pe_sections(b: &[u8]) -> Vec<(usize, usize, usize)> {
    let u16_at = |o: usize| {
        b.get(o..o + 2)
            .map(|s| usize::from(u16::from_le_bytes([s[0], s[1]])))
    };
    let u32_at = |o: usize| {
        b.get(o..o + 4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize)
    };
    let Some(pe) = u32_at(0x3c) else {
        return Vec::new();
    };
    let (Some(count), Some(optional), Some(base)) =
        (u16_at(pe + 6), u16_at(pe + 20), u32_at(pe + 52))
    else {
        return Vec::new();
    };
    (0..count)
        .filter_map(|i| {
            let at = pe + 24 + optional + 40 * i;
            Some((base + u32_at(at + 12)?, u32_at(at + 20)?, u32_at(at + 16)?))
        })
        .collect()
}
