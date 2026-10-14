//! Binary glTF (`.glb`) export of a [`Model`]: one mesh, one primitive per
//! segment. Skeletons and materials are not exported yet.

use serde_json::{Value, json};

use crate::model::Model;

/// Bytes being collected into the binary chunk, with their buffer views.
#[derive(Default)]
struct Bin {
    data: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Bin {
    /// Adds an accessor over freshly appended bytes; returns its index.
    fn accessor(&mut self, bytes: &[u8], target: u32, accessor: Value) -> usize {
        while !self.data.len().is_multiple_of(4) {
            self.data.push(0);
        }
        self.views.push(json!({
            "buffer": 0,
            "byteOffset": self.data.len(),
            "byteLength": bytes.len(),
            "target": target,
        }));
        self.data.extend(bytes);
        let mut accessor = accessor;
        accessor["bufferView"] = json!(self.views.len() - 1);
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }
}

const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
const FLOAT: u32 = 5126;
const UNSIGNED_BYTE: u32 = 5121;
const UNSIGNED_SHORT: u32 = 5123;

fn floats<const N: usize>(v: &[[f32; N]]) -> Vec<u8> {
    v.iter().flatten().flat_map(|f| f.to_le_bytes()).collect()
}

impl Model {
    /// Writes the model as binary glTF. Texture coordinates are flipped to
    /// glTF's convention (v down), matching the upright DDS export.
    #[must_use]
    pub fn to_glb(&self) -> Vec<u8> {
        let mut bin = Bin::default();
        let mut primitives = Vec::new();
        for seg in &self.segments {
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
            primitives.push(json!({"attributes": attributes, "indices": idx, "mode": 4}));
        }
        while !bin.data.len().is_multiple_of(4) {
            bin.data.push(0);
        }
        let doc = json!({
            "asset": {"version": "2.0", "generator": "ffx-decomp ffx-phyre"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            // The game's Y axis points down: turn the model upright with a
            // half turn about X (a rotation, so the winding is kept).
            "nodes": [{"mesh": 0, "rotation": [1.0, 0.0, 0.0, 0.0]}],
            "meshes": [{"primitives": primitives}],
            "buffers": [{"byteLength": bin.data.len()}],
            "bufferViews": bin.views,
            "accessors": bin.accessors,
        });
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
