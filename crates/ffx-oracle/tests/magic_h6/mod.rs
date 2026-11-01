//! Helpers for the per-DLL parity tests of the r6 own ports and the
//! mutation fixes of `magic_0450` and up (`magic_NNNN.rs`; docs/formats/
//! magic.md, "Porting the overlays").
#![allow(dead_code)]

use crate::magic::{SCRATCH, StubMem, Twin};
use crate::magic_owna::{rec, row};

/// Where [`rec_cells`] keeps the cells it sees: per slot, 16 calls of 4
/// words.
#[must_use]
pub fn cells_row(slot: u32) -> u32 {
    SCRATCH + 0x3_8000 + 0x100 * (slot % 0x40)
}

/// [`rec`] that also keeps the words in `cells` at each call (the game
/// flag a slot runs under, the part pointer's cell), so a port that
/// skips a store before the call shows.
pub fn rec_cells(
    t: &mut Twin,
    slot: u32,
    n: usize,
    cells: &[u32],
    ret: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static,
) {
    let cells = cells.to_vec();
    rec(t, slot, n, move |m, a| {
        let k = m.r32(row(slot)).wrapping_sub(1);
        for (i, c) in (0u32..).zip(&cells) {
            let v = m.r32(*c);
            m.w32(cells_row(slot) + 0x10 * (k % 0x10) + 4 * (i % 4), v);
        }
        ret(m, a)
    });
}

/// The DLL globals at the operands `ops` of the function at `f` set to
/// distinct values (`0x00a5_0000 + op`): words a port reads and passes
/// on, so reading a neighbouring operand shows.
pub fn distinct(t: &mut Twin, f: u32, ops: &[u32]) {
    for &op in ops {
        let g = t.img.word(f + op);
        t.set_u32(g, 0x00a5_0000 + op);
    }
}

/// The cell a DLL pointer global (operand `op` of `f`) points at.
#[must_use]
pub fn cell(t: &Twin, f: u32, op: u32) -> u32 {
    t.u32(t.img.word(f + op))
}
