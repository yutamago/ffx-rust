//! `magic_0074.dll`'s functions shared with 4 to 6 other overlays
//! (`0200`, `0247`, `0385`, `0563`, `0612`, `0659`; `0110`, `0112`,
//! `0494`, `0681`).

use crate::{Image, Mem, Sig};

/// 0x10001660 (internal; `ecx` = an index): `G_DEFAULT` for -1, else
/// byte `index` of the table at `*G_TABLE`.
pub fn pick(g: &impl Mem, img: &Image, f: u32, index: u32) -> u32 {
    if index == u32::MAX {
        return g.read::<u32>(img.word(f + 0x06));
    }
    let table = g.read::<u32>(img.word(f + 0x0c));
    u32::from(g.read::<u8>(table.wrapping_add(index)))
}

/// `pick` (21 bytes; 7 overlays).
pub const PICK: Sig = Sig {
    len: 21,
    hash: 0xd764_6eb6_ead9_725a,
    calls: &[],
    imports: &[],
};
