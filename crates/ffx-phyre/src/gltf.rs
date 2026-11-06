//! Binary glTF (`.glb`) export of a [`Model`]:
//!
//! - the `PNode` hierarchy as nodes;
//! - one mesh per mesh instance, attached to its node, with one primitive
//!   per segment (the accessors are shared between instances);
//! - materials with their base color texture, when the caller supplies the
//!   images.
//!
//! - skins: the skeleton's hierarchy matrices are the `PNode`s of the same
//!   names, the skeleton matrices their inverse bind matrices;
//! - animations the caller supplies, as sampled node transforms (the
//!   remaster plays the PS2 motions, which are not in the cluster).

use serde_json::{Value, json};

use crate::material::{Bone, Material, MaterialTexture};
use crate::model::{Model, Segment};

/// One node's sampled transforms in an [`Animation`]; empty paths are
/// not animated.
#[derive(Debug, Clone, Default)]
pub struct Track {
    /// Name of the `PNode` it animates.
    pub node: String,
    pub translation: Vec<[f32; 3]>,
    /// Quaternions `[x, y, z, w]`.
    pub rotation: Vec<[f32; 4]>,
    pub scale: Vec<[f32; 3]>,
}

/// An animation: tracks sampled at a fixed rate from time 0.
#[derive(Debug, Clone, Default)]
pub struct Animation {
    pub name: String,
    pub fps: f32,
    pub tracks: Vec<Track>,
}

/// Bytes being collected into the binary chunk, with their buffer views.
#[derive(Default)]
struct Bin {
    data: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Bin {
    fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while !self.data.len().is_multiple_of(4) {
            self.data.push(0);
        }
        let mut view =
            json!({"buffer": 0, "byteOffset": self.data.len(), "byteLength": bytes.len()});
        if let Some(target) = target {
            view["target"] = json!(target);
        }
        self.views.push(view);
        self.data.extend(bytes);
        self.views.len() - 1
    }

    /// Adds an accessor over freshly appended bytes; returns its index.
    fn accessor(&mut self, bytes: &[u8], target: impl Into<Option<u32>>, accessor: Value) -> usize {
        let view = self.view(bytes, target.into());
        let mut accessor = accessor;
        accessor["bufferView"] = json!(view);
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }
}

const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
const FLOAT: u32 = 5126;
const UNSIGNED_BYTE: u32 = 5121;
const UNSIGNED_SHORT: u32 = 5123;
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

fn floats<const N: usize>(v: &[[f32; N]]) -> Vec<u8> {
    v.iter().flatten().flat_map(|f| f.to_le_bytes()).collect()
}

/// The accessors of one segment, as a primitive without a material; none
/// for empty segments.
fn primitive(bin: &mut Bin, seg: &Segment, with_tangents: bool) -> Option<Value> {
    let count = seg.positions.len();
    if count == 0 || seg.indices.is_empty() {
        return None;
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &seg.positions {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    let mut attributes = serde_json::Map::new();
    let position = bin.accessor(
        &floats(&seg.positions),
        ARRAY_BUFFER,
        json!({"componentType": FLOAT, "count": count, "type": "VEC3", "min": min, "max": max}),
    );
    attributes.insert("POSITION".into(), json!(position));
    if seg.normals.len() == count {
        // glTF wants unit normals; a few stored ones are zero or unnormalized.
        let unit: Vec<[f32; 3]> = seg.normals.iter().map(|&n| export_normal(n)).collect();
        let n = bin.accessor(
            &floats(&unit),
            ARRAY_BUFFER,
            json!({"componentType": FLOAT, "count": count, "type": "VEC3"}),
        );
        attributes.insert("NORMAL".into(), json!(n));
    }
    if with_tangents && seg.tangents.len() == count && seg.normals.len() == count {
        // glTF tangents: unit xyz, w the bitangent's side (cross(n, t) · w
        // points along +v; the export flips v, hence the minus).
        let tangents: Vec<[f32; 4]> = (0..count)
            .map(|i| {
                let (n, t) = (export_normal(seg.normals[i]), seg.tangents[i]);
                let t = {
                    let d = n[0] * t[0] + n[1] * t[1] + n[2] * t[2];
                    let t = [t[0] - n[0] * d, t[1] - n[1] * d, t[2] - n[2] * d];
                    let len = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
                    if len > 1e-6 && len.is_finite() {
                        t.map(|v| v / len)
                    } else if n[0].abs() < 0.9 {
                        export_normal([0.0, -n[2], n[1]])
                    } else {
                        export_normal([-n[2], 0.0, n[0]])
                    }
                };
                let b = seg.binormals.get(i).copied().unwrap_or([0.0; 3]);
                let c = [
                    n[1] * t[2] - n[2] * t[1],
                    n[2] * t[0] - n[0] * t[2],
                    n[0] * t[1] - n[1] * t[0],
                ];
                let side = c[0] * b[0] + c[1] * b[1] + c[2] * b[2];
                [t[0], t[1], t[2], if side < 0.0 { 1.0 } else { -1.0 }]
            })
            .collect();
        let acc = bin.accessor(
            &floats(&tangents),
            ARRAY_BUFFER,
            json!({"componentType": FLOAT, "count": count, "type": "VEC4"}),
        );
        attributes.insert("TANGENT".into(), json!(acc));
    }
    // Every set as TEXCOORD_n and COLOR_n: glTF numbers them without gaps,
    // so a set of another length ends the run.
    let uv_sets = std::iter::once(&seg.uvs).chain(&seg.extra_uvs);
    for (k, set) in uv_sets.take_while(|set| set.len() == count).enumerate() {
        // Stored with v up, matching the bottom-first textures; glTF has v
        // down, matching the upright export.
        let flipped: Vec<[f32; 2]> = set.iter().map(|&uv| export_uv(uv)).collect();
        let t = bin.accessor(
            &floats(&flipped),
            ARRAY_BUFFER,
            json!({"componentType": FLOAT, "count": count, "type": "VEC2"}),
        );
        attributes.insert(format!("TEXCOORD_{k}"), json!(t));
    }
    let color_sets = std::iter::once(&seg.colors).chain(&seg.extra_colors);
    for (k, set) in color_sets.take_while(|set| set.len() == count).enumerate() {
        let bytes: Vec<u8> = set.iter().flatten().copied().collect();
        let col = bin.accessor(
            &bytes,
            ARRAY_BUFFER,
            json!({"componentType": UNSIGNED_BYTE, "normalized": true, "count": count, "type": "VEC4"}),
        );
        attributes.insert(format!("COLOR_{k}"), json!(col));
    }
    let indices: Vec<u8> = seg.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
    // The stream set of each TEXCOORD_n and COLOR_n, when any is not the
    // plain 0 (`m_streamSet`; the attributes are numbered by block order).
    let mut extras = serde_json::Map::new();
    if seg.uv_sets.iter().any(|&s| s != 0) {
        extras.insert("ffx_uv_sets".into(), json!(seg.uv_sets));
    }
    if seg.color_sets.iter().any(|&s| s != 0) {
        extras.insert("ffx_color_sets".into(), json!(seg.color_sets));
    }
    let idx = bin.accessor(
        &indices,
        ELEMENT_ARRAY_BUFFER,
        json!({"componentType": UNSIGNED_SHORT, "count": seg.indices.len(), "type": "SCALAR"}),
    );
    let mut p = json!({"attributes": attributes, "indices": idx, "mode": 4});
    if !extras.is_empty() {
        p["extras"] = Value::Object(extras);
    }
    Some(p)
}

/// A normal as exported: unit length (a few stored ones are zero or
/// unnormalized).
pub(crate) fn export_normal(n: [f32; 3]) -> [f32; 3] {
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len > 1e-6 && len.is_finite() {
        n.map(|v| v / len)
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Texture coordinates as exported: stored with v up, matching the
/// bottom-first textures; glTF has v down, matching the upright export.
pub(crate) fn export_uv([u, v]: [f32; 2]) -> [f32; 2] {
    [u, 1.0 - v]
}

/// One vertex's skinning as exported: palette indices mapped to skeleton
/// hierarchy indices (0 where the weight is 0), weights normalized.
pub(crate) fn export_skin(j: [u8; 4], w: [f32; 4], bones: &[Bone]) -> ([u16; 4], [f32; 4]) {
    let mut w = w;
    let mut joints = [0u16; 4];
    for k in 0..4 {
        let bone = bones.get(usize::from(j[k]));
        if bone.is_none() || !w[k].is_finite() || w[k] < 0.0 {
            w[k] = 0.0;
        }
        joints[k] = bone.filter(|_| w[k] > 0.0).map_or(0, |b| b.hierarchy);
    }
    let sum: f32 = w.iter().sum();
    let w = if sum > 0.0 {
        w.map(|x| x / sum)
    } else {
        [1.0, 0.0, 0.0, 0.0]
    };
    (joints, w)
}

/// `JOINTS_0` (palette indices mapped to skeleton hierarchy indices) and
/// `WEIGHTS_0` (normalized) of a skinned segment.
fn skin_attributes(bin: &mut Bin, seg: &Segment) -> Option<(usize, usize)> {
    let count = seg.positions.len();
    if seg.bones.is_empty() || seg.joints.len() != count || seg.weights.len() != count {
        return None;
    }
    let mut joints = Vec::with_capacity(count * 8);
    let mut weights = Vec::with_capacity(count * 16);
    for (&j, &w) in seg.joints.iter().zip(&seg.weights) {
        let (j, w) = export_skin(j, w, &seg.bones);
        joints.extend(j.iter().flat_map(|x| x.to_le_bytes()));
        weights.extend(w.iter().flat_map(|f| f.to_le_bytes()));
    }
    let j = bin.accessor(
        &joints,
        ARRAY_BUFFER,
        json!({"componentType": UNSIGNED_SHORT, "count": count, "type": "VEC4"}),
    );
    let w = bin.accessor(
        &weights,
        ARRAY_BUFFER,
        json!({"componentType": FLOAT, "count": count, "type": "VEC4"}),
    );
    Some((j, w))
}

/// A node's transform as glTF TRS properties, or its matrix when it has
/// shear. Zero-scale nodes (hidden parts) are only valid as TRS.
#[allow(clippy::many_single_char_names, clippy::float_cmp)] // matrix columns x, y, z; an affine bottom row is exactly 0 0 0 1
fn transform(m: &[f32; 16]) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    if *m == IDENTITY {
        return out;
    }
    let col = |c: usize| [m[4 * c], m[4 * c + 1], m[4 * c + 2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (x, y, z) = (col(0), col(1), col(2));
    let mut scale = [dot(x, x).sqrt(), dot(y, y).sqrt(), dot(z, z).sqrt()];
    let cross = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    if dot(cross, z) < 0.0 {
        scale[0] = -scale[0];
    }
    let bottom = [m[3], m[7], m[11], m[15]];
    let mut rotation = [0.0, 0.0, 0.0, 1.0];
    if scale.iter().all(|s| s.abs() > 1e-6) {
        let unit = |c: [f32; 3], s: f32| c.map(|v| v / s);
        let (x, y, z) = (unit(x, scale[0]), unit(y, scale[1]), unit(z, scale[2]));
        let tolerance = 1e-3;
        if dot(x, y).abs() > tolerance
            || dot(x, z).abs() > tolerance
            || dot(y, z).abs() > tolerance
            || bottom != [0.0, 0.0, 0.0, 1.0]
        {
            out.insert("matrix".into(), json!(m));
            return out;
        }
        // Rotation matrix (columns x, y, z) to a quaternion.
        let (m00, m11, m22) = (x[0], y[1], z[2]);
        let trace = m00 + m11 + m22;
        let q = if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            [
                (y[2] - z[1]) / s,
                (z[0] - x[2]) / s,
                (x[1] - y[0]) / s,
                s / 4.0,
            ]
        } else if m00 > m11 && m00 > m22 {
            let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
            [
                s / 4.0,
                (y[0] + x[1]) / s,
                (z[0] + x[2]) / s,
                (y[2] - z[1]) / s,
            ]
        } else if m11 > m22 {
            let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
            [
                (y[0] + x[1]) / s,
                s / 4.0,
                (z[1] + y[2]) / s,
                (z[0] - x[2]) / s,
            ]
        } else {
            let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
            [
                (z[0] + x[2]) / s,
                (z[1] + y[2]) / s,
                s / 4.0,
                (x[1] - y[0]) / s,
            ]
        };
        let len = q.iter().map(|v| v * v).sum::<f32>().sqrt();
        rotation = q.map(|v| v / len);
    } else if bottom != [0.0, 0.0, 0.0, 1.0] {
        out.insert("matrix".into(), json!(m));
        return out;
    }
    out.insert("translation".into(), json!([m[12], m[13], m[14]]));
    out.insert("rotation".into(), json!(rotation));
    out.insert("scale".into(), json!(scale));
    out
}

/// The keys, or only the first when they never change.
#[allow(clippy::float_cmp)] // the same samples, bit for bit
fn held<const N: usize>(keys: &[[f32; N]]) -> &[[f32; N]] {
    if keys.windows(2).all(|w| w[0] == w[1]) {
        &keys[..1]
    } else {
        keys
    }
}

/// A glTF material from a Phyre one:
///
/// - `TextureSampler` → base color texture, `MaterialColour` → base color
///   factor;
/// - `AlphaThreshold` (when in 0–1) → alpha mask cutoff;
/// - `NormalMapSampler` → normal texture;
/// - `EmissionMapSampler` → emissive texture, `MaterialEmissiveness` →
///   emissive factor.
///
/// All parameter values are kept in `extras.ffx_parameters`.
fn material(
    bin: &mut Bin,
    m: &Material,
    texture: &mut impl FnMut(&mut Bin, &MaterialTexture) -> Option<usize>,
) -> Value {
    let mut out = json!({"pbrMetallicRoughness": {"metallicFactor": 0.0, "roughnessFactor": 1.0}});
    if let Some(t) = m.base_color() {
        out["name"] = json!(t.path);
        if let Some(i) = texture(bin, t) {
            out["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index": i});
        }
    }
    if let Some(c) = m.parameter("MaterialColour").filter(|c| c.len() == 4) {
        let c: Vec<f32> = c.iter().map(|v| v.clamp(0.0, 1.0)).collect();
        if c != [1.0; 4] {
            out["pbrMetallicRoughness"]["baseColorFactor"] = json!(c);
        }
    }
    if let Some(&[t]) = m.parameter("AlphaThreshold")
        && t > 0.0
        && t <= 1.0
    {
        out["alphaMode"] = json!("MASK");
        out["alphaCutoff"] = json!(t);
    }
    if let Some(i) = m.texture("NormalMapSampler").and_then(|t| texture(bin, t)) {
        out["normalTexture"] = json!({"index": i});
    }
    let emissive = m
        .parameter("MaterialEmissiveness")
        .filter(|e| e.len() >= 3)
        .map(|e| [e[0], e[1], e[2]].map(|v| v.clamp(0.0, 1.0)));
    if let Some(i) = m
        .texture("EmissionMapSampler")
        .and_then(|t| texture(bin, t))
    {
        out["emissiveTexture"] = json!({"index": i});
        out["emissiveFactor"] = json!(
            emissive
                .filter(|e| e.iter().any(|&v| v > 0.0))
                .unwrap_or([1.0; 3])
        );
    } else if let Some(e) = emissive.filter(|e| e.iter().any(|&v| v > 0.0)) {
        out["emissiveFactor"] = json!(e);
    }
    let parameters: serde_json::Map<String, Value> = m
        .parameters
        .iter()
        .map(|p| (p.name.clone(), json!(p.values)))
        .collect();
    let samplers: serde_json::Map<String, Value> = m
        .textures
        .iter()
        .map(|t| (t.sampler.clone(), json!(t.path)))
        .collect();
    out["extras"] = json!({"ffx_parameters": parameters, "ffx_textures": samplers});
    out
}

/// An animation's samplers and channels over the glTF `nodes`.
#[allow(clippy::cast_precision_loss)] // frame numbers
fn animation(bin: &mut Bin, a: &Animation, nodes: &[Value]) -> Option<Value> {
    let frames = a
        .tracks
        .iter()
        .map(|t| t.translation.len().max(t.rotation.len()).max(t.scale.len()))
        .max()
        .filter(|&n| n > 0 && a.fps > 0.0)?;
    let times: Vec<[f32; 1]> = (0..frames).map(|f| [f as f32 / a.fps]).collect();
    let mut time_accessors = std::collections::HashMap::new();
    let mut time_accessor = |bin: &mut Bin, n: usize| {
        *time_accessors.entry(n).or_insert_with(|| {
            bin.accessor(
                &floats(&times[..n]),
                None,
                json!({"componentType": FLOAT, "count": n, "type": "SCALAR",
                       "min": [0.0], "max": [times[n - 1][0]]}),
            )
        })
    };
    let mut samplers = Vec::new();
    let mut channels = Vec::new();
    for t in &a.tracks {
        let Some(node) = nodes
            .iter()
            .position(|n| n["name"] == t.node.as_str() && n.get("matrix").is_none())
        else {
            continue;
        };
        let mut add = |bin: &mut Bin, path: &str, bytes: Vec<u8>, n: usize, kind: &str| {
            let input = time_accessor(bin, n);
            let output = bin.accessor(
                &bytes,
                None,
                json!({"componentType": FLOAT, "count": n, "type": kind}),
            );
            samplers.push(json!({"input": input, "output": output, "interpolation": "LINEAR"}));
            channels.push(
                json!({"sampler": samplers.len() - 1, "target": {"node": node, "path": path}}),
            );
        };
        if !t.translation.is_empty() {
            let v = held(&t.translation);
            add(bin, "translation", floats(v), v.len(), "VEC3");
        }
        if !t.rotation.is_empty() {
            // Unit quaternions on the same hemisphere as their predecessor,
            // so that interpolation takes the short way.
            let mut prev = [0.0, 0.0, 0.0, 1.0];
            let rotation: Vec<[f32; 4]> = t
                .rotation
                .iter()
                .map(|q| {
                    let len = q.iter().map(|v| v * v).sum::<f32>().sqrt();
                    let mut q = if len > 1e-6 { q.map(|v| v / len) } else { prev };
                    if q.iter().zip(&prev).map(|(a, b)| a * b).sum::<f32>() < 0.0 {
                        q = q.map(|v| -v);
                    }
                    prev = q;
                    q
                })
                .collect();
            let v = held(&rotation);
            add(bin, "rotation", floats(v), v.len(), "VEC4");
        }
        if !t.scale.is_empty() {
            let v = held(&t.scale);
            add(bin, "scale", floats(v), v.len(), "VEC3");
        }
    }
    (!channels.is_empty())
        .then(|| json!({"name": a.name, "samplers": samplers, "channels": channels}))
}

fn add_child(nodes: &mut [Value], parent: usize, child: usize) {
    if nodes[parent].get("children").is_none() {
        nodes[parent]["children"] = json!([]);
    }
    nodes[parent]["children"]
        .as_array_mut()
        .expect("array")
        .push(json!(child));
}

impl Model {
    /// Writes the model as binary glTF, as stored and without textures.
    #[must_use]
    pub fn to_glb(&self) -> Vec<u8> {
        self.to_glb_with(false, |_| None, &[])
    }

    /// Writes the model as binary glTF. `y_down` turns the model upright
    /// for models stored with Y down (the characters, which came from
    /// the PS2 data; the remastered maps are Y up). `image` supplies a PNG
    /// for a material's base color texture; materials without one get a
    /// plain color. Tracks of `animations` for nodes the model lacks (or
    /// that keep a sheared matrix) are left out.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one glTF document, section by section
    pub fn to_glb_with(
        &self,
        y_down: bool,
        mut image: impl FnMut(&MaterialTexture) -> Option<Vec<u8>>,
        animations: &[Animation],
    ) -> Vec<u8> {
        let mut bin = Bin::default();
        // Tangents for segments drawn with a normal map.
        let normal_mapped = |m: Option<usize>| {
            m.and_then(|m| self.materials.get(m))
                .is_some_and(|m| m.texture("NormalMapSampler").is_some())
        };
        let primitives: Vec<Option<Value>> = self
            .segments
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let tangents = normal_mapped(s.material)
                    || self.instances.iter().any(|inst| {
                        inst.segments
                            .iter()
                            .any(|&(x, m)| x == i && normal_mapped(m))
                    });
                primitive(&mut bin, s, tangents)
            })
            .collect();

        let mut images = Vec::new();
        let mut materials = Vec::new();
        // Images by texture path, each embedded once.
        let mut image_index: std::collections::HashMap<String, Option<usize>> =
            std::collections::HashMap::new();
        let mut texture_of = |bin: &mut Bin, t: &MaterialTexture| -> Option<usize> {
            *image_index.entry(t.path.clone()).or_insert_with(|| {
                let png = image(t)?;
                let view = bin.view(&png, None);
                images.push(json!({"bufferView": view, "mimeType": "image/png"}));
                Some(images.len() - 1)
            })
        };
        for m in &self.materials {
            materials.push(material(&mut bin, m, &mut texture_of));
        }
        // Each primitive names its segment, for `Model::import_glb`.
        let with_material = |s: usize, material: Option<usize>| {
            let mut p = primitives.get(s)?.clone()?;
            if let Some(m) = material {
                p["material"] = json!(m);
            }
            p["extras"]["ffx_segment"] = json!(s);
            Some(p)
        };
        let segment_list = |prims: &[Value]| -> Value {
            json!({"ffx_segments": prims.iter().map(|p| p["extras"]["ffx_segment"].clone()).collect::<Vec<_>>()})
        };

        // Node 0 turns Y-down models upright with a half turn about X (a
        // rotation, so the winding is kept).
        let mut nodes = vec![json!({"name": "root"})];
        if y_down {
            nodes[0]["rotation"] = json!([1.0, 0.0, 0.0, 0.0]);
        }
        for n in &self.nodes {
            let mut node = json!({"name": n.name});
            for (k, v) in transform(&n.local) {
                node[k] = v;
            }
            nodes.push(node);
        }
        for (i, n) in self.nodes.iter().enumerate() {
            let parent = n
                .parent
                .filter(|&p| p < self.nodes.len() && p != i)
                .map_or(0, |p| p + 1);
            add_child(&mut nodes, parent, i + 1);
        }

        let mut meshes = Vec::new();
        if self.instances.is_empty() && primitives.iter().any(Option::is_some) {
            let prims: Vec<Value> = self
                .segments
                .iter()
                .enumerate()
                .filter_map(|(i, s)| with_material(i, s.material))
                .collect();
            meshes.push(json!({"extras": segment_list(&prims), "primitives": prims}));
            nodes.push(json!({"name": "mesh", "mesh": 0}));
            let last = nodes.len() - 1;
            add_child(&mut nodes, 0, last);
        }
        let mut skins = Vec::new();
        let mut scene_roots = vec![0];
        let mut skin_accessors: Vec<Option<Option<(usize, usize)>>> =
            vec![None; self.segments.len()];
        for inst in &self.instances {
            // The skeleton's joints as glTF nodes, found by name.
            let joints: Option<Vec<usize>> = inst
                .skeleton
                .and_then(|k| self.skeletons.get(k))
                .and_then(|sk| {
                    sk.names
                        .iter()
                        .map(|name| {
                            self.nodes
                                .iter()
                                .position(|n| &n.name == name)
                                .map(|n| n + 1)
                        })
                        .collect()
                });
            let mut skinned = false;
            let mut prims = Vec::new();
            for &(s, m) in inst.segments.iter().filter(|&&(s, _)| s < primitives.len()) {
                let Some(mut p) = with_material(s, m) else {
                    continue;
                };
                if joints.is_some()
                    && let Some((j, w)) = *skin_accessors[s]
                        .get_or_insert_with(|| skin_attributes(&mut bin, &self.segments[s]))
                {
                    p["attributes"]["JOINTS_0"] = json!(j);
                    p["attributes"]["WEIGHTS_0"] = json!(w);
                    skinned = true;
                }
                prims.push(p);
            }
            if prims.is_empty() {
                continue;
            }
            meshes.push(
                json!({"name": inst.name, "extras": segment_list(&prims), "primitives": prims}),
            );
            nodes.push(json!({"name": inst.name, "mesh": meshes.len() - 1}));
            if let (true, Some(joints), Some(sk)) = (
                skinned,
                joints,
                inst.skeleton.and_then(|k| self.skeletons.get(k)),
            ) {
                // Inverse bind matrices per joint, from the bone palettes;
                // joints no vertex uses keep the identity.
                let mut ibm = vec![IDENTITY; joints.len()];
                for &(s, _) in &inst.segments {
                    for b in self.segments.get(s).map_or(&[][..], |seg| &seg.bones) {
                        if let (Some(slot), Some(m)) = (
                            ibm.get_mut(usize::from(b.hierarchy)),
                            sk.inverse_bind.get(usize::from(b.skeleton)),
                        ) {
                            *slot = *m;
                        }
                    }
                }
                let bytes: Vec<u8> = ibm.iter().flatten().flat_map(|f| f.to_le_bytes()).collect();
                let acc = bin.accessor(
                    &bytes,
                    None,
                    json!({"componentType": FLOAT, "count": ibm.len(), "type": "MAT4"}),
                );
                skins.push(json!({"joints": joints, "inverseBindMatrices": acc}));
                let last = nodes.len() - 1;
                nodes[last]["skin"] = json!(skins.len() - 1);
                // Skinned meshes ignore their node's transform: a scene root.
                scene_roots.push(last);
                continue;
            }
            let parent = inst
                .node
                .filter(|&n| n < self.nodes.len())
                .map_or(0, |n| n + 1);
            let last = nodes.len() - 1;
            add_child(&mut nodes, parent, last);
        }

        let gltf_animations: Vec<Value> = animations
            .iter()
            .filter_map(|a| animation(&mut bin, a, &nodes))
            .collect();

        while !bin.data.len().is_multiple_of(4) {
            bin.data.push(0);
        }
        let mut doc = json!({
            "asset": {"version": "2.0", "generator": "ffx-decomp ffx-phyre"},
            "scene": 0,
            "scenes": [{"nodes": scene_roots}],
            "nodes": nodes,
            "meshes": meshes,
            "buffers": [{"byteLength": bin.data.len()}],
            "bufferViews": bin.views,
            "accessors": bin.accessors,
        });
        if !skins.is_empty() {
            doc["skins"] = json!(skins);
        }
        if !gltf_animations.is_empty() {
            doc["animations"] = json!(gltf_animations);
        }
        if !materials.is_empty() {
            doc["materials"] = json!(materials);
        }
        if !images.is_empty() {
            let textures: Vec<Value> = (0..images.len())
                .map(|i| json!({"source": i, "sampler": 0}))
                .collect();
            doc["images"] = json!(images);
            doc["samplers"] = json!([{"wrapS": 10497, "wrapT": 10497}]);
            doc["textures"] = json!(textures);
        }
        let mut text = serde_json::to_vec(&doc).expect("JSON");
        while !text.len().is_multiple_of(4) {
            text.push(b' ');
        }
        let total = 12 + 8 + text.len() + 8 + bin.data.len();
        let mut out = Vec::with_capacity(total);
        out.extend(b"glTF");
        out.extend(2u32.to_le_bytes());
        out.extend(u32::try_from(total).expect("glb under 4 GiB").to_le_bytes());
        out.extend(u32::try_from(text.len()).expect("small").to_le_bytes());
        out.extend(b"JSON");
        out.extend(&text);
        out.extend(
            u32::try_from(bin.data.len())
                .expect("glb under 4 GiB")
                .to_le_bytes(),
        );
        out.extend(b"BIN\0");
        out.extend(&bin.data);
        out
    }
}
