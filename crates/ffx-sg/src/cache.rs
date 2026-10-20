//! The model cache's entry release (`Cache_*` callers) and a task
//! helper: glue around the game's allocators and task groups.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Host};

    /// `g_ChrTidus`: the block of the character whose model is c001/c101.
    pub const CHR_TIDUS: Global<u32> = Global::new(0x012f_bc60);
    /// `Sg_FreeA(block, 0)` (platform).
    pub const SG_FREE_A: u32 = 0x0083_f090;
    /// `user_free_thunk(block)`.
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `task_create_in_group(group, entry, stack, prio)`.
    pub const TASK_CREATE_IN_GROUP: u32 = 0x0077_a390;
    /// The kind word of entries that `Sg_FreeA` releases.
    pub const SG_BLOCK: u32 = 0x0009_03e7;

    /// `Cache_FreeEntry(e)` (`0x00840ec0`, name ours; from `Cache_Clear`,
    /// `Cache_Dispose` and the other `Cache_*`): forgets `g_ChrTidus` when
    /// it is the entry's block (`+0xc`), frees the block (`Sg_FreeA(block,
    /// 0)` for kind `0x903e7` at `+0`, else `user_free`), then sets the
    /// kind to -1 and the block to 0. Returns the free's `eax`.
    pub fn cache_free_entry(h: &mut impl Host, e: u32) -> u32 {
        let block: u32 = h.read(e.wrapping_add(0xc));
        let tidus = CHR_TIDUS.get(h);
        // and with 0 or -1: written either way
        CHR_TIDUS.set(h, if block == tidus { 0 } else { tidus });
        let kind: u32 = h.read(e);
        let block: u32 = h.read(e.wrapping_add(0xc));
        let r = if kind == SG_BLOCK {
            let f = h.rebase(SG_FREE_A);
            h.call(f, &[block, 0])
        } else {
            let f = h.rebase(USER_FREE);
            h.call(f, &[block])
        };
        h.write(e, u32::MAX);
        h.write(e.wrapping_add(0xc), 0u32);
        r
    }

    /// `task_create_group7(entry, stack, prio)` (`0x0077a060`, name ours;
    /// only in the function table at `0x00c41940`):
    /// `task_create_in_group(7, entry, stack, prio)`, its `eax` returned.
    pub fn task_create_group7(h: &mut impl Host, entry: u32, stack: u32, prio: u32) -> u32 {
        let f = h.rebase(TASK_CREATE_IN_GROUP);
        h.call(f, &[7, entry, stack, prio])
    }
}
