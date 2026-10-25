//! `PhyreEngine`'s serialization and cluster loading, ported function by
//! function from FFX.exe (M4 port round 1, list `p1-load`): the type
//! system and class descriptors, the streams, the `PBinary` cluster
//! loader. The M3 recipe (`docs/porting.md`, "Library code").
//!
//! - [`descriptor`]: `PType`, `PClassDescriptor` and the per-class
//!   descriptors' virtual methods;
//! - [`stream`]: `PStream`, `PStreamReader`, `PStreamReaderFile`,
//!   the serialization start-up;
//! - [`cluster`]: `PCluster`, `PNamespace` and the cluster headers.
//!
//! Everything this crate does not port is called at its address through
//! [`ffx_graphic::host::Host`] (the allocator, the file layer, the
//! descriptors' other virtual methods).

// 32-bit game addresses and words, reinterpreted where the original does.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

pub mod cluster;
pub mod descriptor;
pub mod stream;

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
    /// An asset import whose cluster is not loaded (`0x0045d9b0`): 7.
    pub const MISSING_CLUSTER: u32 = 7;
}
