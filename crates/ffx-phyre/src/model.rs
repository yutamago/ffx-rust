//! Models (`*.dae.phyre`): mesh segments with their vertex streams, exported
//! as glTF.
//!
//! - Vertex data: one `PDataBlock` per stream (stride, element count, offset
//!   in the vertex buffer). The vertex buffer is the file's last
//!   `m_vertexBufferSize` bytes, the index buffer the `m_indexBufferSize`
//!   bytes before it (cluster header words 19 and 18).
//! - `PMeshSegment`: an embedded `PIndexDataBlock` (16-bit indices, a
//!   triangle list) and a material index.
//! - Stream semantics are `PRenderDataType` enum values, carried as user
//!   fixup strings (`Vertex`, `Normal`, `ST`, `SkinWeights`, …).
//!
//! Which blocks a segment uses and what each block holds comes from the
//! pointer fixups ([`crate::fixup`]):
//!
//! - `PMeshSegment.m_vertexData` points to a run of `PDataBlock`s.
//! - Each block's `m_streams` points to its `PVertexStream`s. Each stream
//!   has an offset within the stride and a data type (`m_type`: 1 float2,
//!   2 float3, 3 float4, 0x13 four bytes).
//! - Each stream's `m_renderDataType` names a `PRenderDataType` user fixup.

use crate::cluster::Cluster;
use crate::material::{Bone, Instance, Material, Node, Resolver, Skeleton};
use crate::{Error, Result};

/// One mesh segment: a triangle list with its vertex attributes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Segment {
    /// Index into [`Model::materials`].
    pub material: Option<usize>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// First texture coordinate set, as stored (OpenGL convention: v up).
    pub uvs: Vec<[f32; 2]>,
    /// Vertex colors, when present, as RGBA bytes (float colors are
    /// clamped to 0–1 and scaled).
    pub colors: Vec<[u8; 4]>,
    /// `SkinIndices`: indices into [`Segment::bones`].
    pub joints: Vec<[u8; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub indices: Vec<u16>,
    /// The bone palette the joints index.
    pub bones: Vec<Bone>,
}

/// A parsed model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model {
    pub segments: Vec<Segment>,
    pub materials: Vec<Material>,
    pub nodes: Vec<Node>,
    /// Placed meshes; when empty, every segment is shown once.
    pub instances: Vec<Instance>,
    /// One per `PMesh`, possibly empty.
    pub skeletons: Vec<Skeleton>,
}

pub(crate) fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Malformed("truncated model"))
}

fn f32s<const N: usize>(b: &[u8], at: usize, stride: usize, count: usize) -> Result<Vec<[f32; N]>> {
    (0..count)
        .map(|i| {
            let o = at + i * stride;
            let mut v = [0f32; N];
            for (k, x) in v.iter_mut().enumerate() {
                *x = f32::from_bits(u32_at(b, o + 4 * k)?);
            }
            Ok(v)
        })
        .collect()
}

fn bytes4(b: &[u8], at: usize, stride: usize, count: usize) -> Result<Vec<[u8; 4]>> {
    (0..count)
        .map(|i| {
            b.get(at + i * stride..at + i * stride + 4)
                .map(|s| s.try_into().expect("4 bytes"))
                .ok_or(Error::Malformed("vertex data outside the file"))
        })
        .collect()
}

/// A vertex stream: where its elements are and what they hold.
struct Stream<'a> {
    semantic: &'a str,
    data_type: u8,
    /// File offset of the first element.
    at: usize,
    stride: usize,
    count: usize,
}

impl Stream<'_> {
    /// Reads the stream into the segment's attribute for its semantic.
    fn decode(&self, bytes: &[u8], seg: &mut Segment) -> Result<()> {
        let (at, stride, count) = (self.at, self.stride, self.count);
        match (
            self.semantic.trim_start_matches("Skinnable"),
            self.data_type,
        ) {
            ("Vertex", FLOAT3) => seg.positions = f32s(bytes, at, stride, count)?,
            ("Vertex", FLOAT4) => {
                seg.positions = f32s::<4>(bytes, at, stride, count)?
                    .into_iter()
                    .map(|[x, y, z, _]| [x, y, z])
                    .collect();
            }
            ("Normal", FLOAT3) => seg.normals = f32s(bytes, at, stride, count)?,
            ("ST", FLOAT2) => seg.uvs = f32s(bytes, at, stride, count)?,
            ("Color", BYTE4) => seg.colors = bytes4(bytes, at, stride, count)?,
            ("Color", FLOAT4) => {
                seg.colors = f32s::<4>(bytes, at, stride, count)?
                    .into_iter()
                    .map(float_color)
                    .collect();
            }
            ("SkinIndices", BYTE4) => seg.joints = bytes4(bytes, at, stride, count)?,
            ("SkinWeights", FLOAT4) => seg.weights = f32s(bytes, at, stride, count)?,
            ("Vertex" | "Normal" | "ST" | "Color" | "SkinIndices" | "SkinWeights", _) => {
                return Err(Error::Unsupported(format!(
                    "{} stream of type {:#x}",
                    self.semantic, self.data_type
                )));
            }
            // Tangents, binormals, further texture coordinates.
            _ => {}
        }
        Ok(())
    }
}

const FLOAT2: u8 = 1;
const FLOAT3: u8 = 2;
const FLOAT4: u8 = 3;
const BYTE4: u8 = 0x13;

fn pointer(offset: usize) -> u32 {
    0x8000_0000 | u32::try_from(offset).unwrap_or(u32::MAX)
}

fn float_color(c: [f32; 4]) -> [u8; 4] {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to 0–255
    c.map(|f| (f.clamp(0.0, 1.0) * 255.0).round() as u8)
}

impl Model {
    /// Parses a model cluster.
    ///
    /// # Errors
    /// Fails on clusters without meshes and on stream layouts not handled
    /// yet.
    #[allow(clippy::too_many_lines)] // one pass over the cluster's tables
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let c = Cluster::parse(bytes)?;
        let need = |class: &str, member: &str| {
            c.member_offset(class, member)
                .map(|o| o as usize)
                .ok_or_else(|| Error::Unsupported(format!("no {class}.{member}")))
        };
        let list_index = |class: &str| {
            c.lists
                .iter()
                .position(|l| c.classes[l.class].name == class)
                .ok_or_else(|| Error::Unsupported(format!("no {class} objects")))
        };
        let (segments_at, blocks_at, streams_at) = (
            list_index("PMeshSegment")?,
            list_index("PDataBlock")?,
            list_index("PVertexStream")?,
        );
        let (o_stride, o_count, o_vb, o_streams) = (
            need("PDataBlock", "m_stride")?,
            need("PDataBlock", "m_elementCount")?,
            need("PDataBlock", "m_offsetInVertexBuffer")?,
            need("PDataBlock", "m_streams")?,
        );
        let (o_stream_offset, o_stream_type) = (
            need("PVertexStream", "m_offset")?,
            need("PVertexStream", "m_type")?,
        );
        let (o_prim, o_index, o_vertex_data) = (
            need("PMeshSegment", "m_primitiveType")?,
            need("PMeshSegment", "m_indexData")?,
            need("PMeshSegment", "m_vertexData")?,
        );
        let (o_index_count, o_index_type, o_index_offset) = (
            need("PIndexDataBlock", "m_elementCount")?,
            need("PIndexDataBlock", "m_type")?,
            need("PIndexDataBlock", "m_offsetInIndexBuffer")?,
        );

        let vertex_buffer = bytes
            .len()
            .checked_sub(c.header.vertex_buffer_size as usize)
            .ok_or(Error::Malformed("vertex buffer larger than the file"))?;
        let index_buffer = vertex_buffer
            .checked_sub(c.header.index_buffer_size as usize)
            .ok_or(Error::Malformed("index buffer larger than the file"))?;

        // Array pointers of each list: object → (first target, count).
        let fixups = c.pointer_fixups(bytes)?;
        let arrays = c.array_fixups(bytes)?;
        let scene = Resolver {
            c: &c,
            bytes,
            pointers: &fixups,
            arrays: &arrays,
        }
        .scene()?;
        let targets = |list: usize, member: usize, target_list: usize| {
            let mut map = vec![None; c.lists[list].count as usize];
            for f in &fixups[list] {
                if f.member() == pointer(member + 4)
                    && f.target_list() as usize == target_list
                    && let Some(slot) = map.get_mut(f.object() as usize)
                {
                    *slot = Some((f.target() as usize, f.count() as usize));
                }
            }
            map
        };
        let segment_blocks = targets(segments_at, o_vertex_data, blocks_at);
        let block_streams = targets(blocks_at, o_streams, streams_at);
        let mut semantics = vec![None; c.lists[streams_at].count as usize];
        for f in &fixups[streams_at] {
            let text = f
                .user_fixup()
                .and_then(|u| c.user_fixups.get(u as usize))
                .map(|u| u.text.as_str());
            if let (Some(slot), Some(text)) = (semantics.get_mut(f.object() as usize), text) {
                *slot = Some(text);
            }
        }

        let (blocks, streams, segments_list) = (
            &c.lists[blocks_at],
            &c.lists[streams_at],
            &c.lists[segments_at],
        );
        let stream = |block: usize, k: usize| -> Result<Stream> {
            let b =
                blocks.object(u32::try_from(block).map_err(|_| Error::Malformed("block index"))?);
            let s = streams.object(u32::try_from(k).map_err(|_| Error::Malformed("stream index"))?);
            if block >= blocks.count as usize || k >= streams.count as usize {
                return Err(Error::Malformed("vertex stream pointer outside its list"));
            }
            Ok(Stream {
                semantic: semantics[k].ok_or(Error::Malformed("vertex stream without a type"))?,
                data_type: *bytes
                    .get(s + o_stream_type)
                    .ok_or(Error::Malformed("truncated model"))?,
                at: vertex_buffer
                    + u32_at(bytes, b + o_vb)? as usize
                    + u32_at(bytes, s + o_stream_offset)? as usize,
                stride: u32_at(bytes, b + o_stride)? as usize,
                count: u32_at(bytes, b + o_count)? as usize,
            })
        };

        let mut segments = Vec::new();
        for i in 0..segments_list.count {
            let o = segments_list.object(i);
            if u32_at(bytes, o + o_prim)? != 2 {
                return Err(Error::Unsupported(
                    "primitive type other than triangle lists".into(),
                ));
            }
            let ix = o + o_index;
            if bytes.get(ix + o_index_type) != Some(&12) {
                return Err(Error::Unsupported("index type other than 16-bit".into()));
            }
            let index_count = u32_at(bytes, ix + o_index_count)? as usize;
            let at = index_buffer + u32_at(bytes, ix + o_index_offset)? as usize;
            let raw = bytes
                .get(at..at + 2 * index_count)
                .ok_or(Error::Malformed("indices outside the file"))?;
            let indices: Vec<u16> = raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&b| u16::from_le_bytes(b))
                .collect();
            let mut seg = Segment {
                material: scene.segment_materials.get(i as usize).copied().flatten(),
                bones: scene
                    .segment_bones
                    .get(i as usize)
                    .cloned()
                    .unwrap_or_default(),
                indices,
                ..Segment::default()
            };
            let (first, n) = segment_blocks[i as usize]
                .ok_or(Error::Malformed("mesh segment without vertex data"))?;
            let mut vertices = None;
            for block in first..first + n {
                let (k0, kn) = block_streams
                    .get(block)
                    .copied()
                    .flatten()
                    .ok_or(Error::Malformed("data block without streams"))?;
                for k in k0..k0 + kn {
                    let s = stream(block, k)?;
                    if *vertices.get_or_insert(s.count) != s.count {
                        return Err(Error::Malformed(
                            "vertex streams of one segment differ in length",
                        ));
                    }
                    s.decode(bytes, &mut seg)?;
                }
            }
            let vertices = vertices.unwrap_or(0);
            if seg.positions.is_empty() {
                return Err(Error::Unsupported("a segment without positions".into()));
            }
            if !index_count.is_multiple_of(3)
                || seg.indices.iter().any(|&x| usize::from(x) >= vertices)
            {
                return Err(Error::Malformed(
                    "indices do not form triangles over the segment's vertices",
                ));
            }
            segments.push(seg);
        }
        Ok(Self {
            segments,
            materials: scene.materials,
            nodes: scene.nodes,
            instances: scene.instances,
            skeletons: scene.skeletons,
        })
    }
}
