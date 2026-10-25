//! A map model (`map/<area>/<map>/mdl/d3d11/<map>.dae.phyre`) as the
//! renderer walks it: mesh instances with their bounds, material sets and
//! segment contexts; mesh segments with their materials, render order
//! buckets, vertex blocks and indices; materials with their effect
//! variant, parameter buffer, pass remapping, textures and sampler states
//! (`docs/scene-builder.md`, "The map model").

// Graph walks name their objects (b, c, g, i, m, o, s, t).
#![allow(clippy::many_single_char_names)]

use crate::phyre::{Graph, Obj};

/// A `PMeshInstanceBounds`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min: [f32; 3],
    pub size: [f32; 3],
    pub render_order: i32,
}

/// A stream renaming of a segment context
/// (`PMeshInstanceSegmentStreamBinding`): the shader stream `name` with
/// `index` reads the segment's stream of `render_type` (`Uv1`, the
/// water's `Uv_nmlmap1`, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamBinding {
    pub render_type: String,
    pub name: String,
    pub index: u8,
    pub input_set: u8,
}

/// A mesh instance.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub name: String,
    /// Global segment indices ([`MapModel::segments`]), in mesh order.
    pub segments: Vec<usize>,
    /// The material set: element index → material ([`MapModel::materials`]).
    pub materials: Vec<Option<usize>>,
    pub bounds: Option<Bounds>,
    /// Per segment (same order as `segments`): its context's stream bindings.
    pub contexts: Vec<Vec<StreamBinding>>,
    /// `m_DObjKind`: 1 puts the bounds in the first sort class at map
    /// init (`MorphManager_registerClusterMorphs` `0x006aca10`).
    pub dobj_kind: i32,
    /// `m_dObjFlag` and `m_groupID` (what the game hides by; the hidden
    /// state itself is game state, `docs/formats/scene-maps.md`).
    pub dobj_flag: u32,
    pub group_id: i32,
}

/// A vertex stream of a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    /// `PRenderDataType`: `Vertex`, `Normal`, `ST`, `Color`, `Tangent`, ...
    pub render_type: String,
    /// `m_type`: 1 float2, 2 float3, 3 float4, 0x13 four bytes.
    pub data_type: u8,
    pub offset: usize,
}

/// A `PDataBlock`: one vertex buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub stride: usize,
    pub count: usize,
    pub streams: Vec<Stream>,
    /// File range of its vertices.
    pub bytes: std::ops::Range<usize>,
}

/// A mesh segment (`PMeshSegmentD3D11`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub material_index: u32,
    /// `m_segmentRenderOrder`: the bucket (0-3 draw, in turn).
    pub render_order: i32,
    /// `m_primitiveType` (2: triangle list).
    pub primitive: i32,
    pub blocks: Vec<Block>,
    /// `u16` indices: file range and count.
    pub indices: std::ops::Range<usize>,
    pub index_count: u32,
}

/// A texture binding of a material: the sampler parameter's offset in the
/// buffer and the texture's archive path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureRef {
    pub offset: u32,
    pub import: String,
}

/// A `PSamplerState` (`PSamplerStateBase` fields as stored).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplerState {
    pub min_filter: u8,
    pub mag_filter: u8,
    pub wrap: [u8; 3],
    pub lod_bias: f32,
    pub max_anisotropy: f32,
    pub border: [u8; 4],
    pub base_level: u32,
    pub max_level: u32,
    pub flags: u32,
}

/// A material.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// The effect variant's import id (`PS3Data/Shaders/X.fx#MD5`).
    pub effect: String,
    /// The `PParameterBuffer` object's bytes as stored (values at their
    /// definition's offset).
    pub buffer: Vec<u8>,
    pub remap_from: Option<String>,
    pub remap_to: Option<String>,
    /// Textures by sampler slot offset (`+0xc` of the 16-byte slot).
    pub textures: Vec<TextureRef>,
    /// Sampler states by slot offset (`+8` of the slot).
    pub samplers: Vec<(u32, SamplerState)>,
}

/// A map model.
#[derive(Debug, Clone)]
pub struct MapModel {
    pub instances: Vec<Instance>,
    pub segments: Vec<Segment>,
    pub materials: Vec<Material>,
    /// The cluster's bytes (vertex and index data are ranges into it).
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err(what: impl Into<String>) -> Error {
    Error(what.into())
}

pub(crate) fn sampler_state(g: &Graph, o: Obj) -> SamplerState {
    let c = "PSamplerStateBase";
    let f = |m: &str| g.u32(o, c, m).unwrap_or(0);
    let b = g.field(o, c, "m_borderColor", 4).unwrap_or(&[0; 4]);
    SamplerState {
        min_filter: g.u8(o, c, "m_minFilter").unwrap_or(0),
        mag_filter: g.u8(o, c, "m_magFilter").unwrap_or(0),
        wrap: [
            g.u8(o, c, "m_wrapS").unwrap_or(0),
            g.u8(o, c, "m_wrapT").unwrap_or(0),
            g.u8(o, c, "m_wrapR").unwrap_or(0),
        ],
        lod_bias: f32::from_bits(f("m_lodBias")),
        max_anisotropy: f32::from_bits(f("m_maxAnisotropy")),
        border: [b[0], b[1], b[2], b[3]],
        base_level: f("m_baseLevel"),
        max_level: f("m_maxLevel"),
        flags: f("m_flags"),
    }
}

impl MapModel {
    /// Reads a map model cluster.
    ///
    /// # Errors
    /// When the cluster doesn't parse or lacks the classes a map has.
    #[allow(clippy::too_many_lines)] // one walk over the object graph
    pub fn parse(bytes: Vec<u8>) -> Result<Self, Error> {
        let model = ffx_phyre::Model::parse(&bytes).map_err(|e| err(format!("model: {e}")))?;
        let buffers = model.buffers();
        let g = Graph::parse(bytes).map_err(|e| err(format!("cluster: {e}")))?;
        let imports = g.imports();

        // Materials, in list order.
        let material_objs = g.objects("PMaterial");
        let mut materials = Vec::new();
        for &m in &material_objs {
            let c = "PMaterial";
            let effect = g
                .ptr(m, c, "m_effectVariant", 0)
                .and_then(|t| g.import_index(t))
                .and_then(|k| imports.get(k).cloned())
                .ok_or_else(|| err(format!("material {} has no effect import", m.index)))?;
            let mut material = Material {
                effect,
                buffer: Vec::new(),
                remap_from: g.user_text(m, c, "m_remapFrom").map(str::to_owned),
                remap_to: g.user_text(m, c, "m_remapTo").map(str::to_owned),
                textures: Vec::new(),
                samplers: Vec::new(),
            };
            if let Some(b) = g.target_obj(m, c, "m_parameterBuffer") {
                let size = g.c.lists[b.list].object_size();
                material.buffer = g.bytes[b.at..b.at + size].to_vec();
                for f in &g.pointers[b.list] {
                    if f.object() != b.index || f.member() & 0x8000_0000 == 0 {
                        continue;
                    }
                    let at = f.member() & 0x7fff_ffff;
                    if at < 8 {
                        continue;
                    }
                    // A sampler slot is 16 bytes at the definition's
                    // offset: +8 the sampler state, +0xc the texture.
                    let t = crate::phyre::Target {
                        list: f.target_list() as usize,
                        first: f.target(),
                        count: f.count(),
                        offset: f.target_offset(),
                        user: f.user_fixup(),
                    };
                    if t.user.is_some() {
                        let import = g
                            .import_index(t)
                            .and_then(|k| imports.get(k).cloned())
                            .ok_or_else(|| err("texture import"))?;
                        material.textures.push(TextureRef {
                            offset: at - 0xc,
                            import,
                        });
                    } else if g.class_of(t.list) == "PSamplerState" {
                        material
                            .samplers
                            .push((at - 8, sampler_state(&g, g.obj(t.list, t.first))));
                    }
                }
            }
            materials.push(material);
        }
        let material_index = |list: usize, obj: u32| {
            material_objs
                .iter()
                .position(|o| o.list == list && o.index == obj)
        };

        // Segments, in list order (the order `ffx_phyre::Model` reads).
        let seg_objs = g.objects("PMeshSegment");
        if seg_objs.len() != buffers.len() {
            return Err(err("segment count differs from the model reader's"));
        }
        let mut segments = Vec::new();
        for (s, b) in seg_objs.iter().zip(&buffers) {
            let c = "PMeshSegmentBase";
            let blocks = g.targets(*s, "PMeshSegmentD3D11", "m_vertexData", 4);
            if blocks.len() != b.blocks.len() {
                return Err(err("block count differs from the model reader's"));
            }
            let blocks = blocks
                .iter()
                .zip(&b.blocks)
                .map(|(o, bb)| Block {
                    stride: bb.stride,
                    count: bb.count,
                    streams: g
                        .targets(*o, "PDataBlockBase", "m_streams", 4)
                        .into_iter()
                        .map(|st| Stream {
                            render_type: g
                                .user_text(st, "PVertexStream", "m_renderDataType")
                                .unwrap_or_default()
                                .to_owned(),
                            data_type: g.u8(st, "PVertexStream", "m_type").unwrap_or(0),
                            offset: g.u32(st, "PVertexStream", "m_offset").unwrap_or(0) as usize,
                        })
                        .collect(),
                    bytes: bb.bytes.clone(),
                })
                .collect();
            let index_at = g
                .offset("PMeshSegmentD3D11", "m_indexData")
                .ok_or_else(|| err("no m_indexData"))? as usize;
            let index_count = g
                .word_at(s.at + index_at + 8)
                .ok_or_else(|| err("index count"))?;
            let index_type = g.bytes.get(s.at + index_at + 0xc).copied();
            if index_type != Some(12) {
                return Err(err(format!("index type {index_type:?} (only u16, 12)")));
            }
            segments.push(Segment {
                material_index: g.u32(*s, c, "m_materialIndex").unwrap_or(0),
                render_order: g
                    .u32(*s, c, "m_segmentRenderOrder")
                    .unwrap_or(0)
                    .cast_signed(),
                primitive: g.u32(*s, c, "m_primitiveType").unwrap_or(0).cast_signed(),
                blocks,
                indices: b.indices.clone(),
                index_count,
            });
        }

        // Bounds by instance.
        let bound_objs = g.objects("PMeshInstanceBounds");
        let bounds_of = |inst: Obj| {
            bound_objs.iter().find_map(|&b| {
                let c = "PMeshInstanceBounds";
                let t = g.ptr(b, c, "m_meshInstance", 0)?;
                (t.list == inst.list && t.first == inst.index).then(|| {
                    let v3 = |m: &str| -> [f32; 3] {
                        let at = b.at + g.offset(c, m).unwrap_or(0) as usize;
                        std::array::from_fn(|k| f32::from_bits(g.word_at(at + 4 * k).unwrap_or(0)))
                    };
                    Bounds {
                        min: v3("m_min"),
                        size: v3("m_size"),
                        render_order: g.u32(b, c, "m_render_order").unwrap_or(0).cast_signed(),
                    }
                })
            })
        };

        let mut instances = Vec::new();
        for i in g.objects("PMeshInstance") {
            let c = "PMeshInstance";
            let name = g.string(i, c, "m_name").unwrap_or_default();
            let mesh = g
                .target_obj(i, c, "m_mesh")
                .ok_or_else(|| err(format!("instance {name} has no mesh")))?;
            let segs: Vec<usize> = g
                .targets(mesh, "PMesh", "m_meshSegments", 4)
                .into_iter()
                .filter_map(|s| seg_objs.iter().position(|x| *x == s))
                .collect();
            let mut set = Vec::new();
            if let Some(t) = g.ptr(i, c, "m_materialSet", 0) {
                let holder = g.obj(t.list, t.first);
                for f in &g.pointers[t.list] {
                    if f.object() == holder.index && f.member() == 0x8000_0000 | (t.offset + 4) {
                        let k = f.count() as usize;
                        if set.len() <= k {
                            set.resize(k + 1, None);
                        }
                        set[k] = material_index(f.target_list() as usize, f.target());
                    }
                }
            }
            let contexts = g
                .targets(i, c, "m_segmentContext", 4)
                .into_iter()
                .map(|ctx| {
                    let cc = "PMeshInstanceSegmentContext";
                    let Some(off) = g.offset(cc, "m_streamBindings") else {
                        return Vec::new();
                    };
                    let count = g.word_at(ctx.at + off as usize).unwrap_or(0) & 0x7fff_ffff;
                    // A `PSharray` of pointers: one inline (+4), or more
                    // through an array of pointers. That array's
                    // pointer-array fixups are not decoded: the bindings
                    // are taken as `count` objects in a row from the
                    // first target (right on cdsp00; open item).
                    let objs: Vec<Obj> = g.pointers[ctx.list]
                        .iter()
                        .filter(|f| {
                            f.object() == ctx.index && f.member() == 0x8000_0000 | (off + 4)
                        })
                        .flat_map(|f| {
                            (f.target()..f.target() + count.max(1))
                                .map(|k| g.obj(f.target_list() as usize, k))
                        })
                        .take(count as usize)
                        .collect();
                    objs.into_iter()
                        .map(|b| {
                            let bc = "PMeshInstanceSegmentStreamBinding";
                            StreamBinding {
                                render_type: g
                                    .user_text(b, bc, "m_renderDataType")
                                    .unwrap_or_default()
                                    .to_owned(),
                                name: g.string(b, bc, "m_name").unwrap_or_default(),
                                index: g.u8(b, bc, "m_index").unwrap_or(0),
                                input_set: g.u8(b, bc, "m_inputSet").unwrap_or(0),
                            }
                        })
                        .collect()
                })
                .collect();
            instances.push(Instance {
                name,
                segments: segs,
                materials: set,
                bounds: bounds_of(i),
                contexts,
                dobj_kind: g.u32(i, c, "m_DObjKind").unwrap_or(0).cast_signed(),
                dobj_flag: g.u32(i, c, "m_dObjFlag").unwrap_or(0),
                group_id: g.u32(i, c, "m_groupID").unwrap_or(0).cast_signed(),
            });
        }
        Ok(Self {
            instances,
            segments,
            materials,
            bytes: g.bytes,
        })
    }

    /// A segment's material for an instance (through the instance's set).
    #[must_use]
    pub fn material(&self, instance: usize, segment: usize) -> Option<usize> {
        let seg = self.segments.get(segment)?;
        self.instances
            .get(instance)?
            .materials
            .get(seg.material_index as usize)
            .copied()
            .flatten()
    }
}

/// The archive path of a cluster import (`PS3Data/Shaders/X.fx#MD5` →
/// `ffx_data/gamedata/ps3data/shaders/d3d11/x.fx#md5.phyre`, textures
/// `.../tex/x.dds` → `.../tex/d3d11/x.dds.phyre`).
#[must_use]
pub fn archive_path(import: &str) -> String {
    let lower = import.to_lowercase().replace('\\', "/");
    let (dir, file) = lower.rsplit_once('/').unwrap_or(("", &lower));
    format!("ffx_data/gamedata/{dir}/d3d11/{file}.phyre")
}
