//! PS2 library calls and other functions the PC port reduced to constants,
//! and the pointer helpers of the PS2 code (`set_ptr32` and friends, the
//! identity on 32-bit pointers).

/// `sceCdDiskReady(mode)` (`0x0062f550`): always ready (`SCECdComplete`).
pub const CD_DISK_READY: u32 = 2;
/// `sceCdRead(...)` (`0x00a447c0`): always accepted.
pub const CD_READ: u32 = 1;
/// `sceSifSetDma(...)` (`0x00a44790`): always a valid transfer ID.
pub const SIF_SET_DMA: u32 = 1;
/// `scePadGetState(port, slot)` (`0x0062fb30`): always stable
/// (`scePadStateStable`).
pub const PAD_GET_STATE: u32 = 6;
/// `ModelBuffer_AllocateBuffer(...)` (`0x0072ac20`).
pub const MODEL_BUFFER_ALLOCATE: u32 = 0xffff;
/// `TOGetLineFeedHeight()` (`0x008b7c60`): a text line is 16 pixels.
pub const LINE_FEED_HEIGHT: u32 = 0x10;
/// What `sceMcSync(mode, &cmd, &result)` (`0x0062f9f0`) returns: -1, no
/// memory-card operation running.
pub const MC_SYNC: i32 = -1;

/// `set_ptr32(p)` (`0x00a44700`) and `get_ptr32_func(p)` (`0x00a44650`).
#[must_use]
pub const fn ptr32(p: u32) -> u32 {
    p
}

/// `is_ptr32_null(p)` (`0x00a446a0`).
#[must_use]
pub const fn is_ptr32_null(p: u32) -> u32 {
    if p == 0 { 1 } else { 0 }
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// `sceMcSync(mode, cmd, result)` (`0x0062f9f0`): clears `*cmd` and
    /// `*result`, returns -1.
    pub fn mc_sync(mem: &mut impl Mem, cmd: u32, result: u32) -> i32 {
        mem.write(cmd, 0u32);
        mem.write(result, 0u32);
        super::MC_SYNC
    }
}
