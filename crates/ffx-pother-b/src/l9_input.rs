//! M5 stage 1, round 9 (`tail9`, list `phyre-framework-input`): the
//! element constructor `PInputDeviceMouse_ctor` and `PInputDevicePad_ctor`
//! hand the vector constructor iterator for their five 0x30-byte entries
//! (`0x00620540`; reach.py's node `FUN_006203b0` ran on over it, and
//! `0x006203b0` itself has no references: symbols/m5-dropped.tsv).

/// The originals over game memory.
pub mod bridge {
    use ffx_bridge::Mem;

    /// `PInputDevice_entryCtor_00620540(entry)` (`0x00620540`, thiscall)
    /// -> `entry`: an empty list head at `+0` (`+0`, `+4` = the entry),
    /// `+8`, `+0xc` = 0, `+0x10` = 1.0, `+0x14`, `+0x18`, `+0x1c` = 0.0,
    /// `+0x20` = 0, `+0x24` (u16) and `+0x26` (u8) = 0, `+0x28`, `+0x2c`
    /// = 0.
    pub fn entry_ctor(m: &mut impl Mem, entry: u32) -> u32 {
        let at = |o: u32| entry.wrapping_add(o);
        m.write(at(0), entry);
        m.write(at(0x10), 1.0_f32);
        m.write(at(4), entry);
        m.write(at(8), 0_u32);
        m.write(at(0x14), 0.0_f32);
        m.write(at(0xc), 0_u32);
        m.write(at(0x18), 0.0_f32);
        m.write(at(0x20), 0_u32);
        m.write(at(0x1c), 0.0_f32);
        m.write(at(0x24), 0_u16);
        m.write(at(0x26), 0_u8);
        m.write(at(0x28), 0_u32);
        m.write(at(0x2c), 0_u32);
        entry
    }
}
