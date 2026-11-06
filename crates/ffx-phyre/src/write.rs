//! Writing edited segments back into their model cluster.
//!
//! The vertex and index buffers end the file, so a segment can change its
//! vertices and triangles freely: the buffers are rebuilt and the fields
//! that describe them updated. These are `PDataBlock` `m_elementCount`,
//! `m_offsetInVertexBuffer` and `m_dataSize`, the embedded
//! `PIndexDataBlock`'s `m_minimumIndex`, `m_maximumIndex`, `m_elementCount`,
//! `m_offsetInIndexBuffer` and `m_dataSize`, and header words 18 and 19.
//! Materials, the skeleton and bone palettes are kept; culling boxes may
//! grow (below).
//!
//! Values equal to the original keep their original bytes, so an unchanged
//! model is written back byte for byte. Tangents and binormals are kept
//! for segments whose vertex count is unchanged and recomputed from the
//! UVs otherwise: the n-th `Tangent` and `Binormal` streams of a segment
//! from its n-th texture coordinate set.
//!
//! The culling box of an instance without a skeleton (`PMeshInstanceBounds`
//! `m_min`, `m_size`, in the instance's model space) grows to hold the
//! edited segments' vertices, so that edited geometry is not culled while
//! it is in view. It only grows, never shrinks; its centre is also the
//! instance's sort key (`docs/formats/scene.md#Sort`), so a grown box can
//! change where the instance sorts. Boxes of instances with a skeleton
//! are moved by the game and kept.

use crate::model::{BlockLoc, Layout, Model, Segment};
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

/// Per-vertex tangents and binormals from the triangles' gradients of the
/// texture coordinates `uvs`.
#[allow(clippy::many_single_char_names)] // vector maths
fn tangents(seg: &Segment, uvs: &[[f32; 2]]) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
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
    let uv = |i: usize| uvs.get(i).copied().unwrap_or([0.0, 0.0]);
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

/// Tangents and binormals computed for a segment, per texture coordinate
/// set.
type Generated = Vec<Option<(Vec<[f32; 3]>, Vec<[f32; 3]>)>>;

/// Texture coordinate set `k` of a segment: [`Segment::uvs`] for 0, else
/// the extra set, `None` when it is not given.
fn uv_set(seg: &Segment, k: usize) -> Option<&[[f32; 2]]> {
    match k {
        0 => Some(&seg.uvs),
        _ => seg
            .extra_uvs
            .get(k - 1)
            .filter(|s| !s.is_empty())
            .map(Vec::as_slice),
    }
}

/// Color set `k` of a segment, as [`uv_set`].
fn color_set(seg: &Segment, k: usize) -> Option<&[[u8; 4]]> {
    match k {
        0 => Some(&seg.colors),
        _ => seg
            .extra_colors
            .get(k - 1)
            .filter(|s| !s.is_empty())
            .map(Vec::as_slice),
    }
}

/// Fills one data block for a segment. `sets` gives each of the block's
/// streams its place among the segment's streams of the same semantic
/// (in block order): the attribute set it is written from.
#[allow(clippy::float_cmp, clippy::too_many_lines)] // unchanged values are bit for bit the parsed ones; one match over the semantics
fn block_bytes(
    block: &BlockLoc,
    sets: &[usize],
    seg: &Segment,
    orig: Option<(&Segment, &[u8])>,
    generated: &mut Generated,
) -> Result<Vec<u8>> {
    let n = seg.positions.len();
    let mut out = match orig {
        Some((_, bytes)) => bytes.to_vec(),
        None => vec![0u8; block.stride * n],
    };
    for (s, &set) in block.streams.iter().zip(sets) {
        let name = s.semantic.trim_start_matches("Skinnable");
        // A further set the segment does not give keeps its stored values,
        // or takes the first set's when the vertex count changed.
        let (uvs, old_uvs) = match (uv_set(seg, set), orig) {
            (Some(u), _) => (u, orig.and_then(|(o, _)| uv_set(o, set))),
            (None, Some(_)) if name == "ST" => continue,
            (None, _) => (&seg.uvs[..], orig.map(|(o, _)| &o.uvs[..])),
        };
        let (colors, old_colors) = match (color_set(seg, set), orig) {
            (Some(c), _) => (c, orig.and_then(|(o, _)| color_set(o, set))),
            (None, Some(_)) if name == "Color" => continue,
            (None, _) => (&seg.colors[..], orig.map(|(o, _)| &o.colors[..])),
        };
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
                    missing("texture coordinates", uvs.len())?;
                    if old_uvs.is_some_and(|o| o.get(v) == Some(&uvs[v])) {
                        continue;
                    }
                    put_f32s(&mut out, at, &uvs[v]);
                }
                ("Color", BYTE4 | FLOAT4) => {
                    missing("colors", colors.len())?;
                    if old_colors.is_some_and(|o| o.get(v) == Some(&colors[v])) {
                        continue;
                    }
                    let c = colors[v];
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
                    if generated.len() <= set {
                        generated.resize(set + 1, None);
                    }
                    let uvs = uv_set(seg, set).unwrap_or(&seg.uvs);
                    let (t, b) = generated[set].get_or_insert_with(|| tangents(seg, uvs));
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

/// Each stream's set, per block: its place among its segment's streams of
/// the same semantic, in block order.
fn stream_sets(layout: &Layout) -> Vec<Vec<usize>> {
    let mut sets: Vec<Vec<usize>> = layout
        .blocks
        .iter()
        .map(|b| vec![0; b.streams.len()])
        .collect();
    for loc in &layout.segments {
        let mut seen: Vec<(&str, usize)> = Vec::new();
        for b in loc.blocks.clone() {
            for (k, s) in layout.blocks[b].streams.iter().enumerate() {
                let name = s.semantic.trim_start_matches("Skinnable");
                let n = match seen.iter_mut().find(|(x, _)| *x == name) {
                    Some((_, n)) => {
                        *n += 1;
                        *n
                    }
                    None => {
                        seen.push((name, 0));
                        0
                    }
                };
                sets[b][k] = n;
            }
        }
    }
    sets
}

impl Model {
    /// Grows the culling box of each unskinned instance with edited
    /// positions to hold them (`out` is the cluster being written).
    #[allow(clippy::float_cmp)] // an unchanged box keeps its bytes
    fn grow_bounds(&self, base: &Model, out: &mut [u8]) {
        for (i, inst) in self.instances.iter().enumerate() {
            let Some(Some([at_min, at_size])) = self.layout.bounds.get(i).copied() else {
                continue;
            };
            let Some(old) = base.instances.get(i).and_then(|b| b.bounds) else {
                continue;
            };
            let segments: Vec<&Segment> = inst
                .segments
                .iter()
                .filter_map(|&(s, _)| self.segments.get(s).map(|seg| (s, seg)))
                .filter(|(s, seg)| {
                    base.segments
                        .get(*s)
                        .is_none_or(|b| b.positions != seg.positions)
                })
                .map(|(_, seg)| seg)
                .collect();
            if segments.is_empty()
                || inst.skeleton.is_some()
                || segments.iter().any(|seg| !seg.joints.is_empty())
            {
                continue;
            }
            let mut lo = old.min;
            let mut hi = [0, 1, 2].map(|k| old.min[k] + old.size[k]);
            for p in segments.iter().flat_map(|seg| &seg.positions) {
                for k in 0..3 {
                    if p[k].is_finite() {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
            }
            if lo != old.min || hi != [0, 1, 2].map(|k| old.min[k] + old.size[k]) {
                put_f32s(out, at_min, &lo);
                put_f32s(out, at_size, &[0, 1, 2].map(|k| hi[k] - lo[k]));
            }
        }
    }

    /// Writes the model's segments into `original`, the cluster it was
    /// parsed from: vertices and triangles may change in number. Materials,
    /// bone palettes and the scene are not written, apart from the culling
    /// boxes of instances without a skeleton, which grow to hold edited
    /// positions.
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
        let sets = stream_sets(layout);
        let mut generated: Vec<Generated> = vec![Vec::new(); self.segments.len()];
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
                    block_bytes(block, &sets[b], seg, orig, &mut generated[s])
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

        self.grow_bounds(&base, &mut out);

        put_u32(&mut out, 0x48, index_buffer.len())?;
        put_u32(&mut out, 0x4c, vertex_buffer.len())?;
        out.extend(index_buffer);
        out.extend(vertex_buffer);
        Ok(out)
    }
}
