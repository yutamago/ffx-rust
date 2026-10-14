//! Writing edited segments back into their model cluster.
//!
//! The vertex and index buffers end the file, so a segment can change its
//! vertices and triangles freely: the buffers are rebuilt and the fields
//! that describe them updated. These are `PDataBlock` `m_elementCount`,
//! `m_offsetInVertexBuffer` and `m_dataSize`, the embedded
//! `PIndexDataBlock`'s `m_minimumIndex`, `m_maximumIndex`, `m_elementCount`,
//! `m_offsetInIndexBuffer` and `m_dataSize`, and header words 18 and 19.
//! Everything else (materials, skeleton, bone palettes, bounds) is kept.
//!
//! Values equal to the original keep their original bytes, so an unchanged
//! model is written back byte for byte. Tangents and binormals are kept
//! for segments whose vertex count is unchanged and recomputed from the
//! UVs otherwise.

use crate::model::{BlockLoc, Model, Segment};
use crate::{Error, Result};

const FLOAT2: u8 = 1;
const FLOAT3: u8 = 2;
const FLOAT4: u8 = 3;
const BYTE4: u8 = 0x13;

fn put_u32(b: &mut [u8], at: usize, v: usize) -> Result<()> {
    let v = u32::try_from(v).map_err(|_| Error::Unsupported("buffer over 4 GiB".into()))?;
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    Ok(())
}

fn put_f32s(b: &mut [u8], at: usize, v: &[f32]) {
    for (k, f) in v.iter().enumerate() {
        b[at + 4 * k..at + 4 * k + 4].copy_from_slice(&f.to_le_bytes());
    }
}

/// Per-vertex tangents and binormals from the triangles' UV gradients.
#[allow(clippy::many_single_char_names)] // vector maths
fn tangents(seg: &Segment) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let n = seg.positions.len();
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let unit = |v: [f32; 3], fallback: [f32; 3]| {
        let len = dot(v, v).sqrt();
        if len > 1e-12 && len.is_finite() {
            v.map(|x| x / len)
        } else {
            fallback
        }
    };
    let mut t = vec![[0f32; 3]; n];
    let mut b = vec![[0f32; 3]; n];
    let uv = |i: usize| seg.uvs.get(i).copied().unwrap_or([0.0, 0.0]);
    for tri in seg.indices.as_chunks::<3>().0 {
        let [i0, i1, i2] = [0, 1, 2].map(|k| usize::from(tri[k]));
        let (e1, e2) = (
            sub(seg.positions[i1], seg.positions[i0]),
            sub(seg.positions[i2], seg.positions[i0]),
        );
        let (u0, u1, u2) = (uv(i0), uv(i1), uv(i2));
        let (s1, t1, s2, t2) = (u1[0] - u0[0], u1[1] - u0[1], u2[0] - u0[0], u2[1] - u0[1]);
        let det = s1 * t2 - s2 * t1;
        if det.abs() < 1e-12 {
            continue;
        }
        let r = 1.0 / det;
        let sdir = [0, 1, 2].map(|k| (e1[k] * t2 - e2[k] * t1) * r);
        let tdir = [0, 1, 2].map(|k| (e2[k] * s1 - e1[k] * s2) * r);
        for i in [i0, i1, i2] {
            for k in 0..3 {
                t[i][k] += sdir[k];
                b[i][k] += tdir[k];
            }
        }
    }
    for i in 0..n {
        let normal = unit(
            seg.normals.get(i).copied().unwrap_or([0.0, 0.0, 1.0]),
            [0.0, 0.0, 1.0],
        );
        let fallback = unit(cross(normal, [0.0, 1.0, 0.0]), [1.0, 0.0, 0.0]);
        let along = dot(normal, t[i]);
        let tangent = unit(sub(t[i], normal.map(|x| x * along)), fallback);
        let mut binormal = cross(normal, tangent);
        if dot(binormal, b[i]) < 0.0 {
            binormal = binormal.map(|x| -x);
        }
        t[i] = tangent;
        b[i] = binormal;
    }
    (t, b)
}

/// Tangents and binormals computed for a segment.
type Generated = Option<(Vec<[f32; 3]>, Vec<[f32; 3]>)>;

/// Fills one data block for a segment.
#[allow(clippy::float_cmp)] // unchanged values are bit for bit the parsed ones
fn block_bytes(
    block: &BlockLoc,
    seg: &Segment,
    orig: Option<(&Segment, &[u8])>,
    generated: &mut Generated,
) -> Result<Vec<u8>> {
    let n = seg.positions.len();
    let mut out = match orig {
        Some((_, bytes)) => bytes.to_vec(),
        None => vec![0u8; block.stride * n],
    };
    for s in &block.streams {
        let name = s.semantic.trim_start_matches("Skinnable");
        let missing = |what: &str, len: usize| {
            if len == n {
                Ok(())
            } else {
                Err(Error::Unsupported(format!(
                    "{what}: {len} values for {n} vertices"
                )))
            }
        };
        for v in 0..n {
            let at = v * block.stride + s.offset;
            match (name, s.data_type) {
                ("Vertex", FLOAT3 | FLOAT4) => {
                    missing("positions", seg.positions.len())?;
                    if orig.is_some_and(|(o, _)| o.positions[v] == seg.positions[v]) {
                        continue;
                    }
                    put_f32s(&mut out, at, &seg.positions[v]);
                    if s.data_type == FLOAT4 && orig.is_none() {
                        put_f32s(&mut out, at + 12, &[1.0]);
                    }
                }
                ("Normal", FLOAT3) => {
                    missing("normals", seg.normals.len())?;
                    if orig.is_some_and(|(o, _)| o.normals[v] == seg.normals[v]) {
                        continue;
                    }
                    put_f32s(&mut out, at, &seg.normals[v]);
                }
                ("ST", FLOAT2) => {
                    missing("texture coordinates", seg.uvs.len())?;
                    if orig.is_some_and(|(o, _)| o.uvs[v] == seg.uvs[v]) {
                        continue;
                    }
                    put_f32s(&mut out, at, &seg.uvs[v]);
                }
                ("Color", BYTE4 | FLOAT4) => {
                    missing("colors", seg.colors.len())?;
                    if orig.is_some_and(|(o, _)| o.colors[v] == seg.colors[v]) {
                        continue;
                    }
                    let c = seg.colors[v];
                    if s.data_type == BYTE4 {
                        out[at..at + 4].copy_from_slice(&c);
                    } else {
                        put_f32s(&mut out, at, &c.map(|x| f32::from(x) / 255.0));
                    }
                }
                ("SkinIndices", BYTE4) => {
                    missing("joints", seg.joints.len())?;
                    out[at..at + 4].copy_from_slice(&seg.joints[v]);
                }
                ("SkinWeights", FLOAT4) => {
                    missing("weights", seg.weights.len())?;
                    if orig.is_some_and(|(o, _)| o.weights[v] == seg.weights[v]) {
                        continue;
                    }
                    put_f32s(&mut out, at, &seg.weights[v]);
                }
                ("Tangent" | "Binormal", FLOAT3) => {
                    if orig.is_some() {
                        continue;
                    }
                    let (t, b) = generated.get_or_insert_with(|| tangents(seg));
                    let value = if name == "Tangent" { t[v] } else { b[v] };
                    put_f32s(&mut out, at, &value);
                }
                _ => {
                    return Err(Error::Unsupported(format!(
                        "writing a {} stream of type {:#x}",
                        s.semantic, s.data_type
                    )));
                }
            }
        }
    }
    Ok(out)
}

impl Model {
    /// Writes the model's segments into `original`, the cluster it was
    /// parsed from: vertices and triangles may change in number. Materials,
    /// bone palettes and the scene are not written.
    ///
    /// # Errors
    /// Fails when `original` is not the model's cluster, an attribute has
    /// the wrong number of values, or a segment has over 65,536 vertices.
    pub fn write(&self, original: &[u8]) -> Result<Vec<u8>> {
        let base = Model::parse(original)?;
        let layout = &base.layout;
        if layout != &self.layout || base.segments.len() != self.segments.len() {
            return Err(Error::Mismatch(
                "not the cluster this model came from".into(),
            ));
        }
        let mut out = original[..layout.index_buffer].to_vec();
        let mut vertex_buffer = Vec::new();
        let mut index_buffer = Vec::new();

        // Index buffer, segments in their original order.
        let mut segment_order: Vec<usize> = (0..self.segments.len()).collect();
        segment_order.sort_by_key(|&s| layout.segments[s].index_offset);
        let [o_min, o_max, o_count, o_offset, o_size] = layout.index_fields;
        for &s in &segment_order {
            let (seg, loc) = (&self.segments[s], &layout.segments[s]);
            if seg.positions.len() > 0x1_0000 {
                return Err(Error::Unsupported(format!(
                    "segment {s}: {} vertices (16-bit indices)",
                    seg.positions.len()
                )));
            }
            if !seg.indices.len().is_multiple_of(3)
                || seg
                    .indices
                    .iter()
                    .any(|&i| usize::from(i) >= seg.positions.len())
            {
                return Err(Error::Unsupported(format!(
                    "segment {s}: indices do not form triangles over its vertices"
                )));
            }
            // Each segment's indices start 4-byte aligned.
            index_buffer.resize(index_buffer.len().next_multiple_of(4), 0);
            let ix = loc.index_block;
            put_u32(&mut out, ix + o_offset, index_buffer.len())?;
            put_u32(&mut out, ix + o_count, seg.indices.len())?;
            put_u32(&mut out, ix + o_size, 2 * seg.indices.len())?;
            if seg.indices != base.segments[s].indices {
                let (min, max) = seg
                    .indices
                    .iter()
                    .fold((u16::MAX, 0), |(lo, hi), &i| (lo.min(i), hi.max(i)));
                put_u32(&mut out, ix + o_min, usize::from(min.min(max)))?;
                put_u32(&mut out, ix + o_max, usize::from(max))?;
            }
            for i in &seg.indices {
                index_buffer.extend(i.to_le_bytes());
            }
        }

        // Vertex buffer, blocks in their original order.
        let mut owner = vec![None; layout.blocks.len()];
        for (s, loc) in layout.segments.iter().enumerate() {
            for b in loc.blocks.clone() {
                owner[b] = Some(s);
            }
        }
        let mut block_order: Vec<usize> = (0..layout.blocks.len()).collect();
        block_order.sort_by_key(|&b| layout.blocks[b].offset);
        let mut generated: Vec<Generated> = vec![None; self.segments.len()];
        let [b_count, b_offset, b_size] = layout.block_fields;
        for &b in &block_order {
            let block = &layout.blocks[b];
            let at = layout.vertex_buffer + block.offset;
            let original_bytes = &original[at..at + block.stride * block.count];
            let bytes = match owner[b] {
                Some(s) => {
                    let (seg, old) = (&self.segments[s], &base.segments[s]);
                    let orig =
                        (seg.positions.len() == block.count).then_some((old, original_bytes));
                    block_bytes(block, seg, orig, &mut generated[s])
                        .map_err(|e| Error::Unsupported(format!("segment {s}: {e}")))?
                }
                None => original_bytes.to_vec(),
            };
            let o = block.object;
            put_u32(&mut out, o + b_offset, vertex_buffer.len())?;
            put_u32(&mut out, o + b_count, bytes.len() / block.stride.max(1))?;
            put_u32(&mut out, o + b_size, bytes.len())?;
            vertex_buffer.extend(bytes);
        }

        put_u32(&mut out, 0x48, index_buffer.len())?;
        put_u32(&mut out, 0x4c, vertex_buffer.len())?;
        out.extend(index_buffer);
        out.extend(vertex_buffer);
        Ok(out)
    }
}
