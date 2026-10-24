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
    /// Names of the instances that draw the segment.
    pub instances: Vec<String>,
    /// Whether any instance has a skeleton (skinned).
    pub skinned: bool,
    pub vertices: usize,
    pub indices: usize,
}

/// Every object of the indexed clusters, by key.
#[derive(Debug, Default, Clone)]
pub struct AssetIndex {
    pub by_key: HashMap<u64, Vec<Source>>,
    /// (model path, segment) → its facts.
    pub segments: HashMap<(String, usize), SegmentInfo>,
    /// Shader key → shader effect name (`phyredefaultlitshader.fx`).
    pub shader_cluster: HashMap<u64, String>,
    /// Clusters read, and those that failed to parse (path, why).
    pub read: usize,
    pub failed: Vec<(String, String)>,
}

impl AssetIndex {
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
    pub fn build(
        archive: &ffx_vbf::Archive,
        want: impl Fn(&str) -> bool,
    ) -> Result<Self, ffx_vbf::Error> {
        let mut index = Self::default();
        for e in archive.entries() {
            if !e.path.ends_with(".phyre") || !want(&e.path) {
                continue;
            }
            let bytes = archive.read(e)?;
            index.add_cluster(&e.path, bytes);
        }
        Ok(index)
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
                    // Every level of the first face: the game may start
                    // below the top.
                    let (mut w, mut h, mut at) = (t.width, t.height, 0);
                    for level in 0..t.mip_levels {
                        let len = t.format.level_size(w, h);
                        let Some(bytes) = t.pixels().get(at..at + len) else {
                            break;
                        };
                        self.add(
                            key::texture(bytes),
                            Source::Texture {
                                path: path.to_owned(),
                                level,
                                width: t.width,
                                height: t.height,
                                levels: t.mip_levels,
                            },
                        );
                        at += len;
                        w = (w / 2).max(1);
                        h = (h / 2).max(1);
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
        for (s, seg) in m.buffers().iter().enumerate() {
            if let Some(ix) = bytes.get(seg.indices.clone()) {
                self.add(
                    key::buffer(ix),
                    Source::Indices {
                        model: path.to_owned(),
                        segment: s,
                    },
                );
            }
            for (b, block) in seg.blocks.iter().enumerate() {
                if let Some(vb) = bytes.get(block.bytes.clone()) {
                    self.add(
                        key::buffer(vb),
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
