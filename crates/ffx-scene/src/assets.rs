//! The GPU objects the archive's clusters make, by content key: shaders
//! from `*.fx.phyre`, textures from `*.dds.phyre`, one index buffer per
//! mesh segment and one vertex buffer per data block from `*.dae.phyre`
//! (`docs/formats/scene.md`, "Asset parity").

use std::collections::HashMap;

use crate::key;

/// Whether `path` is a shader cluster: `shaders/d3d11/<effect>.fx.phyre`
/// or a compiled variant `<effect>.fx#<md5>.phyre`.
#[must_use]
pub fn is_shader_cluster(path: &str) -> bool {
    path.contains("/shaders/") && (path.contains(".fx#") || path.ends_with(".fx.phyre"))
}

/// Where an object with a given key comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A shader in a shader cluster.
    Shader { cluster: String },
    /// Mip level `level` of a texture cluster, bytes as stored (rows
    /// bottom first): the game's texture starts at that level of the file
    /// (it drops the larger ones, `docs/formats/scene.md`).
    Texture {
        path: String,
        level: u32,
        /// The file's top level size and its mip count.
        width: u32,
        height: u32,
        levels: u32,
    },
    /// A model segment's data block `block` (one vertex buffer).
    Vertices {
        model: String,
        segment: usize,
        block: usize,
        /// The block's streams: (semantic, `PVertexStream.m_type`, offset).
        streams: Vec<(String, u8, usize)>,
    },
    /// A model segment's indices (one index buffer).
    Indices { model: String, segment: usize },
}

impl Source {
    /// The archive path of the cluster.
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            Self::Shader { cluster } => cluster,
            Self::Texture { path, .. } => path,
            Self::Vertices { model, .. } | Self::Indices { model, .. } => model,
        }
    }
}

/// A model segment's material and instance facts, for the census.
#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub material: Option<ffx_phyre::Material>,
    /// Names of the instances that draw the segment, and their bounds.
    pub instances: Vec<String>,
    pub bounds: Vec<Option<ffx_phyre::Bounds>>,
    /// Whether any instance has a skeleton (skinned).
    pub skinned: bool,
    pub vertices: usize,
    pub indices: usize,
}

/// The texture quality index of `TextureQuality=TQ_LOW` (`TQ_HIGH` 0,
/// `TQ_MEDIUM` 1, `TQ_LOW` 2: `Settings_SetValue` `0x00401930` stores the
/// value's index; `docs/formats/scene-maps.md`, Q1): the number of top
/// mip levels the game drops from a texture cluster, subject to
/// [`texture_start_level`]'s two exceptions.
pub const TQ_LOW_DROPPED_LEVELS: u32 = 2;

/// The mip level of a texture cluster the game's texture starts at, for
/// texture quality `q` (the levels to drop): `Phyre_PBinary_loadClusterTextures`
/// (`0x00465ab0`, from `0x00466129`). A texture with `q` or fewer levels
/// past the first keeps them all; otherwise the drop backs off while the
/// new top level (`width >> q`, `height >> q`, doubled per step back)
/// would be under 4 px wide or high.
#[must_use]
pub fn texture_start_level(t: &ffx_phyre::Texture, q: u32) -> u32 {
    start_level(t.width, t.height, t.mip_levels, q)
}

/// [`texture_start_level`] on the header's numbers.
#[must_use]
pub fn start_level(width: u32, height: u32, levels: u32, q: u32) -> u32 {
    let extra = levels.saturating_sub(1);
    if extra < q {
        return 0;
    }
    let (mut w, mut h, mut drop) = (width >> q, height >> q, q);
    while drop > 0 && (w < 4 || h < 4) {
        w *= 2;
        h *= 2;
        drop -= 1;
    }
    drop
}

/// The `ffx-drawdiff` key of the texture the game makes from a texture
/// cluster, starting at mip level `first`: its description (top size,
/// remaining levels, faces, format) and the first face's (or every
/// face's) top level.
#[must_use]
pub fn file_texture_key(t: &ffx_phyre::Texture, first: u32) -> Option<u64> {
    let faces: u32 = if t.kind == ffx_phyre::Kind::CubeMap {
        6
    } else {
        1
    };
    let format = key::dxgi_format(t.format);
    let level_dims = |l: u32| ((t.width >> l).max(1), (t.height >> l).max(1));
    let chain: usize = (0..t.mip_levels)
        .map(|l| {
            let (w, h) = level_dims(l);
            t.format.level_size(w, h)
        })
        .sum();
    let skipped: usize = (0..first)
        .map(|l| {
            let (w, h) = level_dims(l);
            t.format.level_size(w, h)
        })
        .sum();
    let (w, h) = level_dims(first);
    let len = t.format.level_size(w, h);
    let desc = [
        w,
        h,
        t.mip_levels.checked_sub(first)?,
        faces,
        format,
        1,
        0,
        0,
        0,
        0,
        0,
    ];
    let row = u32::try_from(key::row_bytes(format, w)?).ok()?;
    // Subresource `face * mips` is a face's top level (D3D11 order); the
    // key reads only those, so the other levels stay empty here. The file
    // holds each face's whole chain in turn.
    let mips = desc[2] as usize;
    let mut subresources = vec![
        ffx_gfx::Subresource {
            row_pitch: 0,
            data: Vec::new(),
        };
        faces as usize * mips
    ];
    for f in 0..faces as usize {
        let at = f * chain + skipped;
        subresources[f * mips] = ffx_gfx::Subresource {
            row_pitch: row,
            data: t.pixels().get(at..at + len)?.to_vec(),
        };
    }
    Some(key::texture(&desc, &subresources))
}

#[cfg(test)]
mod level_tests {
    use super::start_level;

    #[test]
    fn the_games_mip_drop() {
        assert_eq!(start_level(2048, 2048, 12, 2), 2);
        assert_eq!(start_level(2048, 2048, 12, 0), 0);
        // q or fewer levels past the first: all kept
        assert_eq!(start_level(1024, 1024, 1, 2), 0);
        assert_eq!(start_level(1024, 1024, 2, 2), 0);
        assert_eq!(start_level(1024, 1024, 3, 2), 2);
        // the new top would be under 4 px: the drop backs off
        assert_eq!(start_level(8, 8, 4, 2), 1);
        assert_eq!(start_level(16, 4, 5, 2), 0);
        assert_eq!(start_level(64, 512, 10, 1), 1);
    }
}

/// Every object of the indexed clusters, by key.
#[derive(Debug, Default, Clone)]
pub struct AssetIndex {
    /// `ffx-drawdiff` keys of the objects the game makes from the clusters.
    pub by_key: HashMap<u64, Vec<Source>>,
    /// Texture levels by their texels alone ([`key::content`]).
    pub by_content: HashMap<u64, Vec<Source>>,
    /// (model path, segment) → its facts.
    pub segments: HashMap<(String, usize), SegmentInfo>,
    /// Shader key → shader effect name (`phyredefaultlitshader.fx`).
    pub shader_cluster: HashMap<u64, String>,
    /// Clusters read, and those that failed to parse (path, why).
    pub read: usize,
    pub failed: Vec<(String, String)>,
    /// Mip levels the game drops from texture clusters (its texture
    /// quality setting); [`AssetIndex::new`] takes it.
    pub dropped_levels: u32,
    /// Model path → its mesh instances: (name, bounds, segments).
    pub instances: HashMap<String, Vec<InstanceInfo>>,
}

/// A model's mesh instance, for the culling census.
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    pub name: String,
    pub bounds: Option<ffx_phyre::Bounds>,
    pub segments: Vec<usize>,
    pub skinned: bool,
    /// Its node's world matrix as the file's hierarchy gives it (each
    /// `m_localMatrix` times its parent's, column-major, f32 sums), in
    /// memory order; `None` without a node.
    pub world: Option<[f32; 16]>,
}

/// A node's world matrix: the product of the local matrices up the
/// parents (column-major: parent . local), in memory order.
#[must_use]
pub fn node_world(nodes: &[ffx_phyre::Node], node: usize) -> Option<[f32; 16]> {
    let n = nodes.get(node)?;
    let local = n.local;
    let Some(parent) = n.parent.and_then(|p| node_world(nodes, p)) else {
        return Some(local);
    };
    // Column-major: (P . L)[c][r] = sum_k P[k][r] L[c][k], element [c * 4 + r].
    Some(std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        let mut s = 0f32;
        for k in 0..4 {
            s += parent[k * 4 + r] * local[c * 4 + k];
        }
        s
    }))
}

impl AssetIndex {
    /// An empty index for a game that drops `dropped_levels` mip levels
    /// from textures ([`TQ_LOW_DROPPED_LEVELS`] on `TQ_LOW`).
    #[must_use]
    pub fn new(dropped_levels: u32) -> Self {
        Self {
            dropped_levels,
            ..Self::default()
        }
    }

    fn add(&mut self, key: u64, source: Source) {
        let list = self.by_key.entry(key).or_default();
        if !list.contains(&source) {
            list.push(source);
        }
    }

    /// Indexes every cluster of `archive` whose path `want` accepts.
    ///
    /// # Errors
    /// When the archive cannot be read.
    pub fn add_archive(
        &mut self,
        archive: &ffx_vbf::Archive,
        want: impl Fn(&str) -> bool,
    ) -> Result<(), ffx_vbf::Error> {
        for e in archive.entries() {
            if !e.path.ends_with(".phyre") || !want(&e.path) {
                continue;
            }
            let bytes = archive.read(e)?;
            self.add_cluster(&e.path, bytes);
        }
        Ok(())
    }

    /// Indexes the loose-file mods of a game folder (`<game>/mods/<mod>/<archive
    /// path>`, `ffx-hook`'s `mods.rs`), with paths `mods/<mod>/<archive path>`.
    /// The harness links the game folder's mods into every instance, so a
    /// capture's objects can come from them.
    ///
    /// # Errors
    /// When a folder cannot be read.
    pub fn add_mods(&mut self, mods: &std::path::Path) -> std::io::Result<()> {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
            for e in std::fs::read_dir(dir)? {
                let p = e?.path();
                if p.is_dir() {
                    walk(&p, out)?;
                } else if p.extension().is_some_and(|x| x == "phyre") {
                    out.push(p);
                }
            }
            Ok(())
        }
        let mut files = Vec::new();
        if mods.is_dir() {
            walk(mods, &mut files)?;
        }
        files.sort();
        for f in files {
            let rel = f.strip_prefix(mods).unwrap_or(&f).to_string_lossy();
            let path = format!("mods/{}", rel.replace('\\', "/").to_lowercase());
            self.add_cluster(&path, std::fs::read(&f)?);
        }
        Ok(())
    }

    /// Indexes one cluster file.
    pub fn add_cluster(&mut self, path: &str, bytes: Vec<u8>) {
        self.read += 1;
        if is_shader_cluster(path) {
            let file = path.rsplit('/').next().unwrap_or(path);
            let name = file.split('#').next().unwrap_or(file).to_owned();
            for (_, blob) in ffx_dxbc::find_all(&bytes) {
                let k = key::shader(blob);
                self.shader_cluster.entry(k).or_insert_with(|| name.clone());
                self.add(
                    k,
                    Source::Shader {
                        cluster: path.to_owned(),
                    },
                );
            }
        } else if path.ends_with(".dds.phyre") {
            match ffx_phyre::Texture::parse(bytes) {
                Ok(t) => {
                    let source = |level| Source::Texture {
                        path: path.to_owned(),
                        level,
                        width: t.width,
                        height: t.height,
                        levels: t.mip_levels,
                    };
                    // Every level of the first face by its texels alone:
                    // which level a captured texture starts at.
                    let (mut w, mut h, mut at) = (t.width, t.height, 0);
                    for level in 0..t.mip_levels {
                        let len = t.format.level_size(w, h);
                        let Some(bytes) = t.pixels().get(at..at + len) else {
                            break;
                        };
                        let list = self.by_content.entry(key::content(bytes)).or_default();
                        let s = source(level);
                        if !list.contains(&s) {
                            list.push(s);
                        }
                        at += len;
                        w = (w / 2).max(1);
                        h = (h / 2).max(1);
                    }
                    // The texture the game makes from the file.
                    let first = texture_start_level(&t, self.dropped_levels);
                    if let Some(k) = file_texture_key(&t, first) {
                        self.add(k, source(first));
                    }
                }
                Err(e) => self.failed.push((path.to_owned(), e.to_string())),
            }
        } else if path.ends_with(".dae.phyre") {
            match ffx_phyre::Model::parse(&bytes) {
                Ok(m) => self.add_model(path, &bytes, &m),
                Err(e) => self.failed.push((path.to_owned(), e.to_string())),
            }
        }
    }

    fn add_model(&mut self, path: &str, bytes: &[u8], m: &ffx_phyre::Model) {
        self.instances.insert(
            path.to_owned(),
            m.instances
                .iter()
                .map(|i| InstanceInfo {
                    name: i.name.clone(),
                    bounds: i.bounds,
                    segments: i.segments.iter().map(|x| x.0).collect(),
                    skinned: i.skeleton.is_some(),
                    world: i.node.and_then(|n| node_world(&m.nodes, n)),
                })
                .collect(),
        );
        for (s, seg) in m.buffers().iter().enumerate() {
            if let Some(ix) = bytes.get(seg.indices.clone()) {
                self.add(
                    key::static_buffer(ix, true),
                    Source::Indices {
                        model: path.to_owned(),
                        segment: s,
                    },
                );
            }
            for (b, block) in seg.blocks.iter().enumerate() {
                if let Some(vb) = bytes.get(block.bytes.clone()) {
                    self.add(
                        key::static_buffer(vb, false),
                        Source::Vertices {
                            model: path.to_owned(),
                            segment: s,
                            block: b,
                            streams: block.streams.clone(),
                        },
                    );
                }
            }
            let instances: Vec<&ffx_phyre::Instance> = m
                .instances
                .iter()
                .filter(|i| i.segments.iter().any(|x| x.0 == s))
                .collect();
            let segment = &m.segments[s];
            self.segments.insert(
                (path.to_owned(), s),
                SegmentInfo {
                    material: segment.material.and_then(|i| m.materials.get(i).cloned()),
                    instances: instances.iter().map(|i| i.name.clone()).collect(),
                    bounds: instances.iter().map(|i| i.bounds).collect(),
                    skinned: instances.iter().any(|i| i.skeleton.is_some())
                        || !segment.joints.is_empty(),
                    vertices: segment.positions.len(),
                    indices: segment.indices.len(),
                },
            );
        }
    }

    /// The sources of a key.
    #[must_use]
    pub fn get(&self, key: u64) -> &[Source] {
        self.by_key.get(&key).map_or(&[], Vec::as_slice)
    }
}
