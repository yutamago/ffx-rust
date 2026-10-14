//! Binary glTF (`.glb`) export of a [`Model`]:
//!
//! - the `PNode` hierarchy as nodes;
//! - one mesh per mesh instance, attached to its node, with one primitive
//!   per segment (the accessors are shared between instances);
//! - materials with their base color texture, when the caller supplies the
//!   images.
//!
//! - skins: the skeleton's hierarchy matrices are the `PNode`s of the same
//!   names, the skeleton matrices their inverse bind matrices.
//!
//! Animations are not exported yet.

use serde_json::{Value, json};

use crate::material::MaterialTexture;
use crate::model::{Model, Segment};

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
fn primitive(bin: &mut Bin, seg: &Segment) -> Option<Value> {
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
        let unit: Vec<[f32; 3]> = seg
            .normals
            .iter()
            .map(|n| {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if len > 1e-6 && len.is_finite() {
                    n.map(|v| v / len)
                } else {
                    [0.0, 0.0, 1.0]
                }
            })
            .collect();
        let n = bin.accessor(
            &floats(&unit),
            ARRAY_BUFFER,
            json!({"componentType": FLOAT, "count": count, "type": "VEC3"}),
        );
        attributes.insert("NORMAL".into(), json!(n));
    }
    if seg.uvs.len() == count {
        // Stored with v up, matching the bottom-first textures; glTF has v
        // down, matching the upright export.
        let flipped: Vec<[f32; 2]> = seg.uvs.iter().map(|&[u, v]| [u, 1.0 - v]).collect();
        let t = bin.accessor(
            &floats(&flipped),
            ARRAY_BUFFER,
            json!({"componentType": FLOAT, "count": count, "type": "VEC2"}),
        );
        attributes.insert("TEXCOORD_0".into(), json!(t));
    }
    if seg.colors.len() == count {
        let bytes: Vec<u8> = seg.colors.iter().flatten().copied().collect();
        let col = bin.accessor(
            &bytes,
            ARRAY_BUFFER,
            json!({"componentType": UNSIGNED_BYTE, "normalized": true, "count": count, "type": "VEC4"}),
        );
        attributes.insert("COLOR_0".into(), json!(col));
    }
    let indices: Vec<u8> = seg.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
    let idx = bin.accessor(
        &indices,
        ELEMENT_ARRAY_BUFFER,
        json!({"componentType": UNSIGNED_SHORT, "count": seg.indices.len(), "type": "SCALAR"}),
    );
    Some(json!({"attributes": attributes, "indices": idx, "mode": 4}))
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
    for (j, w) in seg.joints.iter().zip(&seg.weights) {
        let mut w = *w;
        for k in 0..4 {
            let bone = seg.bones.get(usize::from(j[k]));
            if bone.is_none() || !w[k].is_finite() || w[k] < 0.0 {
                w[k] = 0.0;
            }
            let joint = bone.filter(|_| w[k] > 0.0).map_or(0, |b| b.hierarchy);
            joints.extend(joint.to_le_bytes());
        }
        let sum: f32 = w.iter().sum();
        if sum > 0.0 {
            w = w.map(|x| x / sum);
        } else {
            w = [1.0, 0.0, 0.0, 0.0];
        }
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
        self.to_glb_with(false, |_| None)
    }

    /// Writes the model as binary glTF. `y_down` turns the model upright
    /// for models stored with Y down (the characters, which came from
    /// the PS2 data; the remastered maps are Y up). `image` supplies a PNG
    /// for a material's base color texture; materials without one get a
    /// plain color.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one glTF document, section by section
    pub fn to_glb_with(
        &self,
        y_down: bool,
        mut image: impl FnMut(&MaterialTexture) -> Option<Vec<u8>>,
    ) -> Vec<u8> {
        let mut bin = Bin::default();
        let primitives: Vec<Option<Value>> = self
            .segments
            .iter()
            .map(|s| primitive(&mut bin, s))
            .collect();

        let mut images = Vec::new();
        let mut materials = Vec::new();
        for m in &self.materials {
            let mut material =
                json!({"pbrMetallicRoughness": {"metallicFactor": 0.0, "roughnessFactor": 1.0}});
            if let Some(texture) = m.base_color() {
                material["name"] = json!(texture.path);
                if let Some(png) = image(texture) {
                    let view = bin.view(&png, None);
                    images.push(json!({"bufferView": view, "mimeType": "image/png"}));
                    material["pbrMetallicRoughness"]["baseColorTexture"] =
                        json!({"index": images.len() - 1});
                }
            }
            materials.push(material);
        }
        let with_material = |s: usize, material: Option<usize>| {
            let mut p = primitives.get(s)?.clone()?;
            if let Some(m) = material {
                p["material"] = json!(m);
            }
            Some(p)
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
            meshes.push(json!({"primitives": prims}));
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
            meshes.push(json!({"name": inst.name, "primitives": prims}));
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
