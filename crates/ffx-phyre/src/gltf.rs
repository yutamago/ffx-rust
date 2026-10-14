//! Binary glTF (`.glb`) export of a [`Model`]:
//!
//! - the `PNode` hierarchy as nodes;
//! - one mesh per mesh instance, attached to its node, with one primitive
//!   per segment (the accessors are shared between instances);
//! - materials with their base color texture, when the caller supplies the
//!   images.
//!
//! Skeletons and animations are not exported yet.

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
    fn accessor(&mut self, bytes: &[u8], target: u32, accessor: Value) -> usize {
        let view = self.view(bytes, Some(target));
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

/// The accessors of one segment, as a primitive without a material.
fn primitive(bin: &mut Bin, seg: &Segment) -> Value {
    let count = seg.positions.len();
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
        let n = bin.accessor(
            &floats(&seg.normals),
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
    json!({"attributes": attributes, "indices": idx, "mode": 4})
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
        let primitives: Vec<Value> = self
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
            let mut p = primitives[s].clone();
            if let Some(m) = material {
                p["material"] = json!(m);
            }
            p
        };

        // Node 0 turns Y-down models upright with a half turn about X (a
        // rotation, so the winding is kept).
        let mut nodes = vec![json!({"name": "root"})];
        if y_down {
            nodes[0]["rotation"] = json!([1.0, 0.0, 0.0, 0.0]);
        }
        for n in &self.nodes {
            let mut node = json!({"name": n.name});
            if n.local != IDENTITY {
                node["matrix"] = json!(n.local);
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
        if self.instances.is_empty() {
            let prims: Vec<Value> = self
                .segments
                .iter()
                .enumerate()
                .map(|(i, s)| with_material(i, s.material))
                .collect();
            meshes.push(json!({"primitives": prims}));
            nodes.push(json!({"name": "mesh", "mesh": 0}));
            let last = nodes.len() - 1;
            add_child(&mut nodes, 0, last);
        }
        for inst in &self.instances {
            let prims: Vec<Value> = inst
                .segments
                .iter()
                .filter(|&&(s, _)| s < primitives.len())
                .map(|&(s, m)| with_material(s, m))
                .collect();
            if prims.is_empty() {
                continue;
            }
            meshes.push(json!({"name": inst.name, "primitives": prims}));
            nodes.push(json!({"name": inst.name, "mesh": meshes.len() - 1}));
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
            "scenes": [{"nodes": [0]}],
            "nodes": nodes,
            "meshes": meshes,
            "buffers": [{"byteLength": bin.data.len()}],
            "bufferViews": bin.views,
            "accessors": bin.accessors,
        });
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
