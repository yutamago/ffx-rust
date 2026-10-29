//! Shapes the effects' own object callbacks repeat (ported per DLL in
//! `mNNNN.rs`; each function keeps its own signature, these are only the
//! steps they share). Operands are offsets into the calling function, as
//! everywhere (`img.word(f + off)`).

use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::{Game, Image};

/// Where a part start's operands are in its function: `*G_P = ITEM`
/// (`gp`, `item`), `list = slot 941 (*G_D + 0x10)` (`gd`, `table`),
/// `start_part(obj, 0x1000, list[entry], 1)` (its `rel32` at `call`).
#[derive(Debug, Clone, Copy)]
pub struct PartAt {
    pub gp: u32,
    pub item: u32,
    pub gd: u32,
    pub table: u32,
    pub entry: u32,
    pub call: u32,
}

/// The part start of an object callback (see [`PartAt`]). Returns what
/// `start_part` leaves in `eax` and the words left on the stack after it
/// (`[program, 1, 0, d]`, top first).
pub fn part_start<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ecx: u32,
    obj: u32,
    at: PartAt,
) -> (u32, [u32; 4]) {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(at.gp));
    g.write::<u32>(p, w(at.item));
    let d = g.read::<u32>(g.read::<u32>(w(at.gd)).wrapping_add(0x10));
    let s941 = slot_target(g, w(at.table), 941);
    let list = g.call(s941, ecx, &[d]).eax;
    let program = g.read::<u32>(list.wrapping_add(at.entry));
    let eax = start_part(
        g,
        img,
        img.rel32_target(f + at.call),
        obj,
        0x1000,
        program,
        1,
    );
    (eax, [program, 1, 0, d])
}

/// `mov dword [base + field], imm32` for each `(field, operand)`, the
/// immediate read from the function's code.
pub fn imm_stores<G: Game>(g: &mut G, img: &Image, f: u32, base: u32, stores: &[(u32, u32)]) {
    for &(field, op) in stores {
        g.write::<u32>(base.wrapping_add(field), img.word(f + op));
    }
}
