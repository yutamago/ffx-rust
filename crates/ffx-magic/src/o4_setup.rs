//! Callers of the summon model set-up ([`model_setup`], `ecx` = a
//! cursor word holding the record) from the own work list (round 4).
//! The cursor is a stack word where the original keeps it, and each
//! call runs on the frame the original's callee sees.

use crate::runtime::slot_target;
use crate::sa_summon::model_setup;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The set-up at `target` with `ecx` = `cursor`, its return address at
/// `esp`.
fn setup<G: Game>(g: &mut G, img: &Image, target: u32, esp: u32, cursor: u32) -> u32 {
    let fr = Frame {
        at: target,
        esp,
        ecx: cursor,
        ..Frame::default()
    };
    model_setup(g, img, &fr).eax
}

/// No arguments (`magic_0042` 0x1000b060, 61 bytes): slot 10 (`+0x11`)
/// with `(*A, 1)` (`A` at `+0x0d`), then the set-up of the records at
/// `+0x1f` and `+0x2e` (the cursor a local at `ebp - 4`; the calls at
/// `+0x24`, `+0x33`). Returns 0.
pub fn two_setups<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x0d));
    let s = slot_target(g, w(0x05), crate::o4_parts::disp8(img, f + 0x13) / 4);
    g.call(s, fr.ecx, &[a, 1]);
    // ebp = esp - 4; the cursor at ebp - 4; each call's return address
    // below it
    let cursor = fr.esp.wrapping_sub(8);
    let esp = fr.esp.wrapping_sub(12);
    g.write::<u32>(cursor, w(0x1f));
    setup(g, img, img.rel32_target(f + 0x24), esp, cursor);
    g.write::<u32>(cursor, w(0x2e));
    setup(g, img, img.rel32_target(f + 0x33), esp, cursor);
    Ret::eax(0)
}

/// `(p)` (`magic_0262` 0x10006350, 63 bytes): for each of the `B[0x4dc]`
/// records from `B + 0x1c` (0x98 bytes apart; `B` = `*p`), the set-up
/// (`+0x27`) with the cursor in the argument word itself. Returns the last
/// set-up's `eax` (`p` when none ran).
pub fn setups_of_block<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = fr.arg(g, 0);
    let b = g.read::<u32>(p);
    let cursor = fr.esp + 4;
    // push ebx, esi, edi: the call's return address 0x14 below the entry
    let esp = fr.esp.wrapping_sub(0x14);
    let mut eax = p;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(b.wrapping_add(0x4dc)).cast_signed() {
        g.write::<u32>(cursor, b.wrapping_add(0x1c + 0x98 * i));
        eax = setup(g, img, img.rel32_target(f + 0x27), esp, cursor);
        i += 1;
    }
    Ret::eax(eax)
}

/// No arguments (`magic_0262` 0x10006e50, 53 bytes): for each of the
/// `*N` records from `T` (`N` at `+0x09`/`+0x29`, `T` at `+0x11`, 0x98
/// bytes apart), the set-up (`+0x1c`) with the cursor a local at `ebp -
/// 4`. Returns the last set-up's `eax` (`eax` as given when none ran).
pub fn setups_of_table<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let cursor = fr.esp.wrapping_sub(8);
    // push ecx, esi, edi
    let esp = fr.esp.wrapping_sub(0x14);
    let mut eax = fr.eax;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(w(0x09)).cast_signed() {
        g.write::<u32>(cursor, w(0x11).wrapping_add(0x98 * i));
        eax = setup(g, img, img.rel32_target(f + 0x1c), esp, cursor);
        i += 1;
    }
    Ret::eax(eax)
}

/// [`two_setups`] in `magic_0042` (0x1000b060, 61 bytes).
pub const TWO_SETUPS_0042_B060: Sig = Sig {
    len: 61,
    hash: 0x5f0b_eaa9_9e42_20e0,
    calls: &[
        (0x24, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x33, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// [`setups_of_block`] in `magic_0262` (0x10006350, 63 bytes).
pub const SETUPS_OF_BLOCK_0262_6350: Sig = Sig {
    len: 63,
    hash: 0x3b6d_324c_ecca_d117,
    calls: &[(0x27, Callee::Sig(&crate::sa_summon::MODEL_SETUP))],
    imports: &[],
};

/// [`setups_of_table`] in `magic_0262` (0x10006e50, 53 bytes).
pub const SETUPS_OF_TABLE_0262_6E50: Sig = Sig {
    len: 53,
    hash: 0xff88_e2c2_976d_7b19,
    calls: &[(0x1c, Callee::Sig(&crate::sa_summon::MODEL_SETUP))],
    imports: &[],
};
