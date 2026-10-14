//! `PhyreEngine` cluster files (`*.phyre`) of the FFX/X-2 HD Remaster.
//!
//! Every asset of the remaster (textures, models, shaders) is a `PhyreEngine`
//! "cluster": a header, a packed namespace describing the classes it uses,
//! instance data, fixup tables and, for textures, the pixel data. This crate
//! currently understands textures (`*.dds.phyre`) well enough to convert
//! them to DDS and back. The layout below was recovered from the files
//! themselves and checked against all 44,364 shipped textures; see
//! `docs/formats/phyre.md`.

pub mod cluster;
mod dds;
mod decode;
pub mod edit;
pub mod fixup;
mod flip;
mod gltf;
mod import;
mod material;
mod model;
mod texture;
mod write;

pub use dds::{Dds, DdsError};
pub use decode::png;
pub use gltf::{Animation, Track};
pub use import::import_into_cluster;
pub use material::{Bone, Instance, Material, MaterialTexture, Node, Parameter, Skeleton};
pub use model::{Model, Segment};
pub use texture::{Format, Kind, Texture};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a little-endian Phyre cluster (magic {0:02x?})")]
    NotPhyre([u8; 4]),
    #[error("malformed cluster: {0}")]
    Malformed(&'static str),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("DDS: {0}")]
    Dds(#[from] DdsError),
    #[error("the DDS does not match the texture: {0}")]
    Mismatch(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The fixed cluster header: 21 little-endian `u32`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub namespace_size: u32,
    pub platform: [u8; 4],
    pub instance_list_count: u32,
    pub array_fixup_size: u32,
    pub array_fixup_count: u32,
    pub pointer_fixup_size: u32,
    pub pointer_fixup_count: u32,
    pub pointer_array_fixup_size: u32,
    pub user_fixup_count: u32,
    pub user_fixup_data_size: u32,
    pub total_data_size: u32,
    /// Word 15: header class counts (`u32` each) after the user fixups.
    pub header_class_count: u32,
    /// Word 16: header class fixups (16 bytes each) after those.
    pub header_class_fixup_count: u32,
    /// Word 18: size of the index buffer at the end of the file (models).
    pub index_buffer_size: u32,
    /// Word 19: size of the vertex buffer at the end of the file (models).
    pub vertex_buffer_size: u32,
    /// Word 20: for 2D textures, the size of the top mip level.
    pub max_texture_buffer_size: u32,
}

/// Size of the cluster header.
pub const HEADER_LEN: usize = 0x54;
/// `"RYHP"` read as a little-endian `u32`: the magic of PC clusters.
pub const MAGIC: u32 = 0x5048_5952;

impl Header {
    /// Parses the header.
    ///
    /// # Errors
    /// Fails on a wrong magic or a truncated header.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let words: Vec<u32> = bytes
            .get(..HEADER_LEN)
            .ok_or(Error::Malformed("truncated header"))?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&c| u32::from_le_bytes(c))
            .collect();
        if words[0] != MAGIC {
            return Err(Error::NotPhyre(bytes[..4].try_into().expect("4 bytes")));
        }
        if words[1] as usize != HEADER_LEN {
            return Err(Error::Malformed("unexpected header size"));
        }
        Ok(Self {
            namespace_size: words[2],
            platform: words[3].to_le_bytes(),
            instance_list_count: words[4],
            array_fixup_size: words[5],
            array_fixup_count: words[6],
            pointer_fixup_size: words[7],
            pointer_fixup_count: words[8],
            pointer_array_fixup_size: words[9],
            user_fixup_count: words[12],
            user_fixup_data_size: words[13],
            total_data_size: words[14],
            header_class_count: words[15],
            header_class_fixup_count: words[16],
            index_buffer_size: words[18],
            vertex_buffer_size: words[19],
            max_texture_buffer_size: words[20],
        })
    }

    /// Offset where the instance data starts.
    #[must_use]
    pub fn instance_data(&self) -> usize {
        HEADER_LEN + self.namespace_size as usize
    }

    /// Offset where the instance data ends.
    #[must_use]
    pub fn instance_data_end(&self) -> usize {
        self.instance_data() + self.total_data_size as usize
    }

    /// Offset of the user fixup data (strings such as the class name).
    #[must_use]
    pub fn user_fixup_data(&self) -> usize {
        self.instance_data_end() + 0x24 * self.instance_list_count as usize
    }

    /// Offset after all tables: where trailing buffers (pixel data) start.
    #[must_use]
    pub fn buffers(&self) -> usize {
        self.user_fixup_data()
            + self.user_fixup_data_size as usize
            + 12 * self.user_fixup_count as usize
            + self.array_fixup_size as usize
            + self.pointer_fixup_size as usize
            + self.pointer_array_fixup_size as usize
    }
}
