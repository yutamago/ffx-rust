//! Character model clusters (`chr/**/mdl/d3d11/*.dae.phyre`) as the
//! character builder needs them (`docs/formats/scene-chars.md`, "The
//! builder"): the map reader's view ([`MapModel`]: instances, segments,
//! vertex blocks, materials) plus, per mesh instance, what ties a
//! `PMeshInstance` of the live character world to it: its mesh's
//! skeleton matrices (`PMesh +8`) and joint parents (`+0x28`
//! `m_matrixParents`), its segments' fields and `m_skinBones`, and its
//! bounds.
//!
//! The scene inputs record holds no asset name for a character instance
//! (the character cluster `PhyreScene +0x11e4` holds every character's
//! instances, `attachChrClusterTo3DWorld`), so [`ChrIndex`] finds the
//! file instance whose mesh data the record's instance carries
//! ([`matches`]).

use std::ops::Range;

use ffx_gfx::scene_inputs::{Bounds as LiveBounds, CharInstance, Instance, SceneInputs};

use crate::map::{self, MapModel};
use crate::phyre::{Graph, Obj};

/// A `PMeshSegmentBase`'s fields the record repeats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentInfo {
    /// `+0` `m_materialIndex`, `+4` `m_matrixIndex`, `+0x18`
    /// `m_modeIndex`, `+0x1c` `m_segmentRenderOrder`.
    pub material_index: u32,
    pub matrix_index: u32,
    pub mode_index: u32,
    pub render_order: u32,
    /// `+8` `m_skinBones`: the `PSkinBoneRemap`s as their words.
    pub skin_bones: Vec<u32>,
}

/// A file mesh instance's identifying data.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceInfo {
    /// `PMesh +8` `m_skeletonMatrices` (memory order).
    pub skeleton: Vec<[f32; 16]>,
    /// `PMesh +0x28` `m_matrixParents`.
    pub parents: Vec<i32>,
    pub segments: Vec<SegmentInfo>,
    /// The instance's `PMeshInstanceBounds` (`m_min`, `m_size`).
    pub bounds: Option<map::Bounds>,
    /// Per segment, its material's `PParameterBuffer` as stored (empty
    /// for a null material; [`add_materials`]).
    pub materials: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// A `PArray`'s element count (the word at the member, top bit masked).
fn array_count(g: &Graph, o: Obj, class: &str, member: &str) -> usize {
    g.u32(o, class, member)
        .map_or(0, |w| (w & 0x7fff_ffff) as usize)
}

fn floats16(g: &Graph, at: usize) -> [f32; 16] {
    std::array::from_fn(|k| f32::from_bits(g.word_at(at + 4 * k).unwrap_or(0)))
}

fn mesh_info(g: &Graph, mesh: Obj) -> (Vec<[f32; 16]>, Vec<i32>, Vec<SegmentInfo>) {
    let c = "PMesh";
    let n = array_count(g, mesh, c, "m_skeletonMatrices");
    let skeleton = g
        .targets(mesh, c, "m_skeletonMatrices", 4)
        .into_iter()
        .take(n)
        .map(|m| floats16(g, m.at))
        .collect();
    let n = array_count(g, mesh, c, "m_matrixParents");
    let parents = g
        .array_bytes(mesh, c, "m_matrixParents", 4)
        .map(|b| {
            b.as_chunks::<4>()
                .0
                .iter()
                .take(n)
                .map(|w| i32::from_le_bytes(*w))
                .collect()
        })
        .unwrap_or_default();
    let s = "PMeshSegmentBase";
    let segments = g
        .targets(mesh, c, "m_meshSegments", 4)
        .into_iter()
        .map(|seg| {
            let n = array_count(g, seg, s, "m_skinBones");
            let skin_bones = if n == 0 {
                Vec::new()
            } else {
                g.targets(seg, s, "m_skinBones", 4)
                    .into_iter()
                    .take(n)
                    .map(|b| g.word_at(b.at).unwrap_or(0))
                    .collect()
            };
            SegmentInfo {
                material_index: g.u32(seg, s, "m_materialIndex").unwrap_or(0),
                matrix_index: g.u32(seg, s, "m_matrixIndex").unwrap_or(0),
                mode_index: g.u32(seg, s, "m_modeIndex").unwrap_or(0),
                render_order: g.u32(seg, s, "m_segmentRenderOrder").unwrap_or(0),
                skin_bones,
            }
        })
        .collect();
    (skeleton, parents, segments)
}

/// The identifying data of every `PMeshInstance` of a cluster, in the
/// order [`MapModel::parse`] reads them.
///
/// # Errors
/// When the cluster doesn't parse.
pub fn instance_infos(bytes: Vec<u8>) -> Result<Vec<InstanceInfo>, Error> {
    let g = Graph::parse(bytes).map_err(|e| Error(format!("cluster: {e}")))?;
    let bounds = g.objects("PMeshInstanceBounds");
    let mut out = Vec::new();
    for i in g.objects("PMeshInstance") {
        let c = "PMeshInstance";
        let Some(mesh) = g.target_obj(i, c, "m_mesh") else {
            return Err(Error("an instance without a mesh".into()));
        };
        let (skeleton, parents, segments) = mesh_info(&g, mesh);
        let b = bounds.iter().find_map(|&b| {
            let bc = "PMeshInstanceBounds";
            let t = g.ptr(b, bc, "m_meshInstance", 0)?;
            (t.list == i.list && t.first == i.index).then(|| {
                let v3 = |m: &str| -> [f32; 3] {
                    let at = b.at + g.offset(bc, m).unwrap_or(0) as usize;
                    std::array::from_fn(|k| f32::from_bits(g.word_at(at + 4 * k).unwrap_or(0)))
                };
                map::Bounds {
                    min: v3("m_min"),
                    size: v3("m_size"),
                    render_order: g.u32(b, bc, "m_render_order").unwrap_or(0).cast_signed(),
                }
            })
        });
        out.push(InstanceInfo {
            skeleton,
            parents,
            segments,
            bounds: b,
            materials: Vec::new(),
        });
    }
    Ok(out)
}

fn bits(m: &[f32]) -> impl Iterator<Item = u32> + '_ {
    m.iter().map(|x| x.to_bits())
}

/// Whether a record instance is an instance of this file instance's
/// mesh: the same segments (material index, matrix index, mode index,
/// render order and skin bone remaps, as `PMeshSegment` holds them), the
/// same joint parents and skeleton matrices, bit for bit; an instance
/// without a skeleton (a weapon's part) also needs the file's bounds
/// (the game copies them; a skinned instance's bounds move with it).
#[must_use]
pub fn matches(
    info: &InstanceInfo,
    i: &Instance,
    c: &CharInstance,
    bounds: Option<&LiveBounds>,
) -> bool {
    if info.segments.len() != i.segments.len()
        || info.parents != c.joint_parent
        || info.skeleton.len() != c.skeleton.len()
    {
        return false;
    }
    let segments = info
        .segments
        .iter()
        .zip(&i.segments)
        .enumerate()
        .all(|(k, (f, r))| {
            let bones = c.skin_bones.get(k).map_or(&[][..], Vec::as_slice);
            f.material_index == r.material
                && f.matrix_index == r.matrix
                && f.mode_index == r.id
                && f.render_order == r.render_order
                && f.skin_bones.len() == bones.len()
                && f.skin_bones
                    .iter()
                    .zip(bones)
                    .all(|(&w, b)| w == u32::from(b[0]) | u32::from(b[1]) << 16)
        });
    if !segments {
        return false;
    }
    if !info
        .skeleton
        .iter()
        .zip(&c.skeleton)
        .all(|(a, b)| bits(a).eq(bits(b)))
    {
        return false;
    }
    if info.skeleton.is_empty() {
        return match (&info.bounds, bounds) {
            (Some(f), Some(b)) => bits(&f.min).eq(bits(&b.min)) && bits(&f.size).eq(bits(&b.size)),
            _ => false,
        };
    }
    true
}

/// Fills each instance's segment material buffers from the map reader's
/// view of the same cluster (instances in the same order).
pub fn add_materials(infos: &mut [InstanceInfo], map: &MapModel) {
    for (k, info) in infos.iter_mut().enumerate() {
        info.materials = map.instances.get(k).map_or_else(Vec::new, |inst| {
            inst.segments
                .iter()
                .map(|&s| {
                    map.material(k, s)
                        .and_then(|m| map.materials.get(m))
                        .map_or_else(Vec::new, |m| m.buffer.clone())
                })
                .collect()
        });
    }
}

/// How alike a record instance's live materials are to a file instance's:
/// per segment, the 4-byte words its live `PParameterBuffer` and the file
/// material's buffer have equal. Telling apart meshes the record can't
/// (models sharing a rig, Besaid's NPCs): the game writes only some of a
/// material's values at run time.
#[must_use]
pub fn material_score(info: &InstanceInfo, s: &SceneInputs, i: &Instance) -> usize {
    info.materials
        .iter()
        .zip(&i.segments)
        .map(|(file, seg)| {
            let live = i
                .materials
                .get(seg.material as usize)
                .copied()
                .flatten()
                .and_then(|m| s.materials.get(m as usize))
                .map_or(&[][..], |m| m.parameters.as_slice());
            file.as_chunks::<4>()
                .0
                .iter()
                .zip(live.as_chunks::<4>().0)
                .filter(|(a, b)| a == b)
                .count()
        })
        .sum()
}

/// The candidate character models: per cluster path, its instances'
/// identifying data.
#[derive(Debug, Clone, Default)]
pub struct ChrIndex {
    pub models: Vec<(String, Vec<InstanceInfo>)>,
}

impl ChrIndex {
    /// Reads the identifying data of each cluster of `paths` through
    /// `read`; clusters that don't parse are left out.
    pub fn new<'a>(
        read: impl Fn(&str) -> Option<Vec<u8>>,
        paths: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let models = paths
            .into_iter()
            .filter_map(|p| {
                let bytes = read(p)?;
                let mut infos = instance_infos(bytes.clone()).ok()?;
                let map = MapModel::parse(bytes).ok()?;
                add_materials(&mut infos, &map);
                Some((p.to_owned(), infos))
            })
            .collect();
        Self { models }
    }

    /// The (model, file instance) pairs a record instance matches, best
    /// first ([`material_score`]; equal scores in index order).
    #[must_use]
    pub fn candidates(&self, s: &SceneInputs, index: usize) -> Vec<(usize, usize)> {
        let (Some(i), Some(c)) = (
            s.instances.get(index),
            s.chars.as_ref().and_then(|w| w.instances.get(index)),
        ) else {
            return Vec::new();
        };
        let bounds = s
            .bounds
            .iter()
            .find(|b| b.instance == u32::try_from(index).ok());
        let mut out = Vec::new();
        for (m, (_, infos)) in self.models.iter().enumerate() {
            for (k, info) in infos.iter().enumerate() {
                if matches(info, i, c, bounds) {
                    out.push((material_score(info, s, i), m, k));
                }
            }
        }
        out.sort_by_key(|x| std::cmp::Reverse(x.0));
        out.into_iter().map(|(_, m, k)| (m, k)).collect()
    }

    /// The model paths the record's character instances need: per
    /// instance the best matching model (several clusters can hold the
    /// same mesh).
    #[must_use]
    pub fn models_for(&self, s: &SceneInputs) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for k in 0..s.instances.len() {
            if let Some(&(m, _)) = self.candidates(s, k).first() {
                let p = &self.models[m].0;
                if !out.contains(p) {
                    out.push(p.clone());
                }
            }
        }
        out
    }
}

/// A per-shader parameter (`PShader +0xc` `m_parameterDefinitionsForPasses`):
/// the node context's light values, which location group 4 reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderParam {
    pub name: String,
    /// `m_parameterType`: `0xc2` a light's `m_colorIntensity`, `0xcb` its
    /// `m_direction`, `0xcf`-`0xd2` `CombinedLightAttenuationCoeff0/1`,
    /// `CombinedLightSpotCoeff0/1`.
    pub kind: u8,
    /// `m_bufferLoc`: offset (bits 0-15), size (bits 16-28), the light
    /// (bits 29-31).
    pub offset: u32,
    pub size: u32,
    pub light: u32,
}

/// An effect's node contexts (`PEffect`): which shader of a render pass
/// a node draws with (`0x004ef440`: a binary search of the node's packed
/// switches in `m_contextVariantSwitches`, compared by length, then word
/// by word, `0x004eea10`; the found index is the shader's), and per
/// render pass and shader its own parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectContexts {
    /// `+0x2c` `m_contextVariantSwitches`: each `PNodeContext`'s
    /// `m_packedSwitches`, in order.
    pub keys: Vec<Vec<u32>>,
    /// `+0x38` `m_maxLightCount`.
    pub max_lights: u32,
    /// Per render pass (the variant's order), per shader:
    /// `m_parameterBufferSize` and the parameters.
    pub shaders: Vec<Vec<(u32, Vec<ShaderParam>)>>,
}

impl EffectContexts {
    /// Reads them from an effect variant cluster.
    ///
    /// # Errors
    /// When the cluster doesn't parse.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, Error> {
        let g = Graph::parse(bytes).map_err(|e| Error(format!("effect: {e}")))?;
        let mut out = Self::default();
        if let Some(&e) = g.objects("PEffect").first() {
            let c = "PEffect";
            out.max_lights = g.u32(e, c, "m_maxLightCount").unwrap_or(0);
            let n = array_count(&g, e, c, "m_contextVariantSwitches");
            let objs = if n == 0 {
                Vec::new()
            } else {
                g.targets(e, c, "m_contextVariantSwitches", 4)
            };
            for o in objs.into_iter().take(n) {
                let k = "PNodeContext";
                let count = array_count(&g, o, k, "m_packedSwitches");
                let key = match count {
                    0 => Vec::new(),
                    1 => vec![g.word_at(o.at + 4).unwrap_or(0)],
                    _ => g
                        .array_bytes(o, k, "m_packedSwitches", 4)
                        .map(|b| {
                            b.as_chunks::<4>()
                                .0
                                .iter()
                                .take(count)
                                .map(|w| u32::from_le_bytes(*w))
                                .collect()
                        })
                        .unwrap_or_default(),
                };
                out.keys.push(key);
            }
        }
        if let Some(&v) = g.objects("PEffectVariant").first() {
            for rp in g.targets(v, "PEffectVariant", "m_sceneRenderPasses", 4) {
                let mut shaders = Vec::new();
                for sh in g.targets(rp, "PSceneRenderPass", "m_shaders", 4) {
                    let s = "PShader";
                    let n = array_count(&g, sh, s, "m_parameterDefinitionsForPasses");
                    let defs = if n == 0 {
                        Vec::new()
                    } else {
                        g.targets(sh, s, "m_parameterDefinitionsForPasses", 4)
                    };
                    let params = defs
                        .into_iter()
                        .take(n)
                        .map(|d| {
                            let t = "PShaderParameterDefinition";
                            let loc = g.u32(d, t, "m_bufferLoc").unwrap_or(0);
                            ShaderParam {
                                name: g.string(d, t, "m_name").unwrap_or_default(),
                                kind: g.u8(d, t, "m_parameterType").unwrap_or(0),
                                offset: loc & 0xffff,
                                size: (loc >> 16) & 0x1fff,
                                light: loc >> 29,
                            }
                        })
                        .collect();
                    let size = g.u32(sh, s, "m_parameterBufferSize").unwrap_or(0);
                    shaders.push((size, params));
                }
                out.shaders.push(shaders);
            }
        }
        Ok(out)
    }

    /// The shader (context) index for a node lit by `lights` lights: the
    /// context whose packed switches count that many (the low four bits
    /// of the first word: `[0]`, `[0x11]`, `[0x4012]`, `[0x01004013, 0]`
    /// for 0-3 directional lights on `phyrechrlitshader`), at most
    /// `m_maxLightCount`; the first when none does.
    #[must_use]
    pub fn context(&self, lights: u32) -> usize {
        let n = lights.min(self.max_lights);
        self.keys
            .iter()
            .position(|k| k.first().map_or(0, |w| w & 0xf) == n)
            .unwrap_or(0)
    }
}

/// Appends `m` to `into` (segment, material and byte ranges moved on);
/// returns the instance range `m`'s instances got.
pub fn append(into: &mut MapModel, m: MapModel) -> Range<usize> {
    let n = m.instances.len();
    let first = into.append(m);
    first..first + n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> InstanceInfo {
        InstanceInfo {
            skeleton: vec![[1.0; 16]],
            parents: vec![-1, 0],
            segments: vec![SegmentInfo {
                material_index: 0,
                matrix_index: u32::MAX,
                mode_index: 1,
                render_order: 0,
                skin_bones: vec![0x0002_0001],
            }],
            bounds: None,
            materials: Vec::new(),
        }
    }

    #[test]
    fn records_match_their_mesh() {
        let i = Instance {
            segments: vec![ffx_gfx::scene_inputs::Segment {
                material: 0,
                matrix: u32::MAX,
                id: 1,
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut c = CharInstance {
            joint_parent: vec![-1, 0],
            skeleton: vec![[1.0; 16]],
            skin_bones: vec![vec![[1, 2]]],
            ..Default::default()
        };
        assert!(matches(&info(), &i, &c, None));
        c.skin_bones[0][0] = [2, 1];
        assert!(!matches(&info(), &i, &c, None));
        c.skin_bones[0][0] = [1, 2];
        c.skeleton[0][3] = 0.5;
        assert!(!matches(&info(), &i, &c, None));
    }

    #[test]
    fn unskinned_instances_match_by_bounds() {
        let mut f = info();
        f.skeleton.clear();
        f.parents.clear();
        f.segments[0].skin_bones.clear();
        f.bounds = Some(map::Bounds {
            min: [-1.0; 3],
            size: [2.0; 3],
            render_order: 0,
        });
        let i = Instance {
            segments: vec![ffx_gfx::scene_inputs::Segment {
                material: 0,
                matrix: u32::MAX,
                id: 1,
                ..Default::default()
            }],
            ..Default::default()
        };
        let c = CharInstance {
            skin_bones: vec![Vec::new()],
            ..Default::default()
        };
        let mut b = LiveBounds {
            min: [-1.0; 3],
            size: [2.0; 3],
            ..Default::default()
        };
        assert!(matches(&f, &i, &c, Some(&b)));
        b.size[1] = 3.0;
        assert!(!matches(&f, &i, &c, Some(&b)));
        assert!(!matches(&f, &i, &c, None));
    }
}
