//! `PhyreEngine`'s serialization and cluster loading, ported function by
//! function from FFX.exe (M4 port round 1, list `p1-load`): the type
//! system and class descriptors, the streams, the `PBinary` cluster
//! loader. The M3 recipe (`docs/porting.md`, "Library code").
//!
//! - [`descriptor`]: `PType`, `PClassDescriptor` and the per-class
//!   descriptors' virtual methods;
//! - [`stream`]: `PStream`, `PStreamReader`, `PStreamReaderFile`,
//!   the serialization start-up;
//! - [`cluster`]: `PCluster`, `PNamespace` and the cluster headers;
//! - [`types`]: the `GetType<T>` statics of the primitive and enum types;
//! - [`p1_misc`]: data members, namespace lookups, name hashes (batch 10);
//! - [`p1_types`]: descriptor constructors, wrapper types (batch 11);
//! - [`p1_loader`]: instance lists, namespace mapping, asset registry, the asset file load (batches 11-12);
//! - [`p1_bind`]: the classes' static registration as step tables (batches 12-13);
//! - [`p1_reader`]: the cluster reader's instance-list headers (batch 13);
//! - [`p1_tree`]: `PRedBlackTree::postRemoveFixup` (round 3);
//! - [`p1_d3d`]: the D3D11 shaders, input layout, constant buffer, pass states, cube maps, data blocks (round 3);
//! - [`p1_post`]: the post-processing constructors (round 3);
//! - [`p1_declare`]: two `PArray<T>` descriptor declarations (round 3);
//! - [`p1_buffers`]: the class descriptors' default and validation buffers (round 3);
//! - [`p1_fixup`]: `PCluster::fixupInstances` (round 3);
//! - [`p1_lists`]: the cluster reader's instance-list data (round 3);
//! - [`p1_fixups`]: the cluster reader's user fixups (round 3);
//! - [`p1_dynbind`]: `PClassDescriptorDynamic::Bind` (round 4);
//! - [`p1_mapping`]: the namespace mapping's build and class checks (round 4);
//! - [`p1_pointers`]: the cluster reader's array and pointer-array fixups (round 4);
//! - [`p1_body`]: the cluster reader's body, its steps after the header (round 4);
//! - [`p1_header`]: the cluster reader's header classes (round 4);
//! - [`p1_cluster`]: `PBinary::loadCluster` (round 4);
//! - [`p1_textures`]: `PBinary::loadClusterTextures` (round 5).
//! - [`m5p_tmpl_abstract_ab`]: the `PClassDescriptorAbstract<T>` slots 7, 12, 13, 14 (M5 lists tmpl-abstract-a, -b).
//! - [`m5p_tmpl_abstract_cd`]: the same slots for M5 lists tmpl-abstract-c, -d (the ab ports, one more for slot 4).
//!
//! Everything this crate does not port is called at its address through
//! [`ffx_graphic::host::Host`] (the allocator, the file layer, the
//! descriptors' other virtual methods).

// 32-bit game addresses and words, reinterpreted where the original does.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

pub mod cluster;
pub mod descriptor;
pub mod m5_static_init_ab;
pub mod m5_static_init_b;
pub mod m5_static_init_c;
pub mod m5_static_init_d;
pub mod m5p_tmpl_abstract_ab;
pub mod m5p_tmpl_abstract_cd;
pub mod p1_bind;
pub mod p1_body;
pub mod p1_buffers;
pub mod p1_cluster;
pub mod p1_d3d;
pub mod p1_declare;
pub mod p1_dynbind;
pub mod p1_fixup;
pub mod p1_fixups;
pub mod p1_header;
pub mod p1_lists;
pub mod p1_loader;
pub mod p1_mapping;
pub mod p1_misc;
pub mod p1_pointers;
pub mod p1_post;
pub mod p1_reader;
pub mod p1_textures;
pub mod p1_tree;
pub mod p1_types;
pub mod stream;
pub mod types;

/// `PhyreEngine`'s error codes, as its functions return them in `eax`.
pub mod error {
    /// No handler for the request (a missing fixup function, no
    /// dynamic-array resizer): 0x15.
    pub const UNSUPPORTED: u32 = 0x15;
    /// Already done (`InitializeAssetTree` a second time): 0x16.
    pub const ALREADY: u32 = 0x16;
    /// A name lookup that found nothing (`0x00507b50`): 0x13.
    pub const NOT_FOUND: u32 = 0x13;
    /// A read or write that moved fewer bytes than asked: 9.
    pub const SHORT_IO: u32 = 9;
    /// An enum entry without a name (`0x00507ad0`): 0x11.
    pub const NO_NAME: u32 = 0x11;
    /// An enum entry whose name is registered already (`0x00507ad0`):
    /// 0x12.
    pub const DUPLICATE: u32 = 0x12;
    /// An allocation that failed (`0x0044a010`): 0xd.
    pub const NO_BLOCK: u32 = 0xd;
    /// An asset import whose cluster is not loaded (`0x0045d9b0`): 7.
    pub const MISSING_CLUSTER: u32 = 7;
}
