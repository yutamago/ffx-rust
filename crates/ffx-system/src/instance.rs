//! Singletons and small object accessors of the PC port's managers
//! (clusters, the asset loader, the archive stream, the magic file, asset
//! lists).

/// `AssetListReader::getAssetCount` for an asset vector `[begin, end)`:
/// the 24-byte records between them (a signed division, toward zero).
#[must_use]
pub const fn asset_count(begin: u32, end: u32) -> i32 {
    end.wrapping_sub(begin).cast_signed() / 24
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `ClusterManager::GetInstance`'s object.
    pub const CLUSTER_MANAGER: Global<u32> = Global::new(0x00cc_ca4c);
    /// `AsyncLoadManager::sGetInstance`'s object.
    pub const ASYNC_LOAD_MANAGER: Global<u32> = Global::new(0x00cc_c88c);
    /// `BigFileStream_sBigFileStreamInstacne` (sic).
    pub const BIG_FILE_STREAM: Global<u32> = Global::new(0x00cc_9c48);
    /// The magic (spell effect) file's ID, -1 when none.
    pub const MAGIC_ID: Global<u32> = Global::new(0x00c6_4ca0);
    /// The magic file's handle, -1 when none.
    pub const MAGIC_HANDLE: Global<u32> = Global::new(0x00c6_4cac);
    /// Whether the magic file is NEX data.
    pub const MAGIC_IS_NEX: Global<u32> = Global::new(0x0193_fdac);

    /// `ClusterManager_GetInstance()` (`0x0069a1a0`).
    pub fn cluster_manager(mem: &impl Mem) -> u32 {
        CLUSTER_MANAGER.get(mem)
    }

    /// `AsyncLoadManager_sGetInstance()` (`0x00691b40`).
    pub fn async_load_manager(mem: &impl Mem) -> u32 {
        ASYNC_LOAD_MANAGER.get(mem)
    }

    /// `AsyncLoadManager::isAllRequestDone` (`0x00691630`, `this` in ECX):
    /// both request counts (`+0x20`, `+0x2c`) zero.
    pub fn is_all_request_done(mem: &impl Mem, this: u32) -> bool {
        mem.read::<u32>(this.wrapping_add(0x20)) == 0
            && mem.read::<u32>(this.wrapping_add(0x2c)) == 0
    }

    /// `BigFileStream_get()` (`0x0061bdb0`).
    pub fn big_file_stream(mem: &impl Mem) -> u32 {
        BIG_FILE_STREAM.get(mem)
    }

    /// `MagicFile_GetMagicId()` (`0x009da3b0`).
    pub fn magic_id(mem: &impl Mem) -> u32 {
        MAGIC_ID.get(mem)
    }

    /// `MagicFile_GetHandle()` (`0x009da390`).
    pub fn magic_handle(mem: &impl Mem) -> u32 {
        MAGIC_HANDLE.get(mem)
    }

    /// `MagicFile_IsNexData()` (`0x009da420`).
    pub fn magic_is_nex(mem: &impl Mem) -> u32 {
        MAGIC_IS_NEX.get(mem)
    }

    /// `AssetListReader::getAssetCount` (`0x006f4170`, `this` in ECX): the
    /// asset vector at `+0x14`.
    pub fn asset_count(mem: &impl Mem, this: u32) -> i32 {
        super::asset_count(
            mem.read(this.wrapping_add(0x14)),
            mem.read(this.wrapping_add(0x18)),
        )
    }
}
