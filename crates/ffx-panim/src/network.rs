//! `PAnimationNetworkInstance` and its parts.

pub mod bridge {
    use ffx_bridge::Mem;

    /// The size of a `PAnimationDataSourceListEntry`.
    pub const LIST_ENTRY_SIZE: u32 = 0x94;

    /// `PAnimationDataSourceListEntry::PAnimationDataSourceListEntry()`
    /// (`0x00534360`, `thiscall`): every word zero. Returns `this`.
    pub fn list_entry_ctor(m: &mut impl Mem, this: u32) -> u32 {
        for off in (0..LIST_ENTRY_SIZE).step_by(4) {
            m.write(this + off, 0u32);
        }
        this
    }
}
