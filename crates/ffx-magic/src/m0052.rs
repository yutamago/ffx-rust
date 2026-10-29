//! `magic_0052.dll`'s functions shared with `0055`, `0064`, `0067`,
//! `0266`, `0353`, `0435`: internal helpers around the effect's model
//! (`TABLE[*G_INDEX]`, a byte index into a table of model handles).
//!
//! Frames: a port that passes the address of a local to the game keeps
//! the local where the original's frame has it (below the entry `esp`),
//! so the words the game sees are the original's; the hosts leave that
//! space to the port (the 32-bit hook runs ports 64 KiB below the
//! caller's stack, docs/formats/magic.md "Porting the overlays").

use crate::runtime::slot_target;
use crate::{Game, Image, Sig};

/// The model handle `TABLE[byte *G_INDEX]` (operands of a `mov reg,
/// [G_INDEX]` and a `[eax*4 + TABLE]`).
fn model(g: &impl crate::Mem, img: &Image, f: u32, index_op: u32, table_op: u32) -> u32 {
    let i = u32::from(g.read::<u8>(g.read::<u32>(img.word(f + index_op))));
    g.read::<u32>(img.word(f + table_op) + 4 * i)
}

/// 0x10001580 (internal, no arguments): slot 357 (model, -1, -1) and
/// slot 356 (model, -1, -1; the first call's words stay on the stack).
/// Returns slot 356's `eax`.
pub fn reset_model<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let m = model(g, img, f, 0x01, 0x0e);
    let all = u32::MAX;
    let s357 = slot_target(g, w(0x13), 357);
    g.call(s357, 0, &[m, all, all]);
    let s356 = slot_target(g, w(0x23), 356);
    g.call(s356, 0, &[m, all, all, m, all, all]).eax
}

/// `reset_model` (57 bytes; `magic_0052`, `0064`).
pub const RESET_MODEL: Sig = Sig {
    len: 57,
    hash: 0xa138_7f1b_1433_be05,
    calls: &[],
    imports: &[],
};

/// 0x10001cd0 (internal; `edx` = a position, 3 floats): with `G_MODE`
/// 0 and a model index below 0x20, slot 255 (index, pos); else, the
/// model's handle `h`: slot 11 (h) says whether it is a model; if not,
/// slot 33 (h, x, y, z), else slot 32 (h, x, z). The floats are passed
/// as their bits. Returns the last slot's `eax`.
pub fn place<G: Game>(g: &mut G, img: &Image, f: u32, pos: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mode = g.read::<u32>(w(0x02));
    let index = g.read::<u32>(w(0x09));
    if mode == 0 {
        let i = u32::from(g.read::<u8>(index));
        if i < 0x20 {
            let s255 = slot_target(g, w(0x1d), 255);
            return g.call(s255, 0, &[i, pos]).eax;
        }
    }
    let i = u32::from(g.read::<u8>(index));
    let h = g.read::<u32>(w(0x35) + 4 * i);
    let s11 = slot_target(g, w(0x3a), 11);
    let r = g.call(s11, 0, &[h]).eax;
    let z = g.read::<u32>(pos + 8);
    if r == 0 {
        let s33 = slot_target(g, w(0x4f), 33);
        let y = g.read::<u32>(pos + 4);
        let x = g.read::<u32>(pos);
        g.call(s33, 0, &[h, x, y, z]).eax
    } else {
        let s32 = slot_target(g, w(0x4f), 32);
        let x = g.read::<u32>(pos);
        g.call(s32, 0, &[h, x, z]).eax
    }
}

/// `place` (162 bytes; 5 overlays).
pub const PLACE: Sig = Sig {
    len: 162,
    hash: 0x3ae0_4741_274e_a00f,
    calls: &[],
    imports: &[],
};

/// 0x10001f00 (internal; `ecx` = an argument passed on, `esp` the
/// frame's entry stack pointer): the four rows of the camera matrix
/// (slot 544 (i, &row), 16 bytes each, into the frame's locals at
/// `ebp - 0x44`, `- 0x34`, `- 0x24`, `- 0x14`), the first three floats of
/// each scaled by `K` (SSE), then slot 553 (ecx, &rows...; the earlier
/// calls' words stay on the stack). Returns slot 553's `eax`. (The
/// frame's stack cookie is the CRT's; nothing else reads it.)
pub fn scaled_camera<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp.wrapping_sub(4);
    let rows = [0x44u32, 0x34, 0x24, 0x14].map(|o| ebp.wrapping_sub(o));
    let mut pushed: Vec<u32> = Vec::new();
    for (i, (row, t)) in (0u32..).zip(rows.iter().zip([0x16, 0x2b, 0x3e, 0x51])) {
        let s544 = slot_target(g, w(t), 544);
        let mut args = vec![i, *row];
        args.extend(pushed.iter().rev());
        // call width: 8 (i, the row and three earlier calls' two words)
        g.call(s544, 0, &args);
        pushed.extend([*row, i]);
    }
    let k = f32::from_bits(g.read::<u32>(w(0x63)));
    for row in rows {
        for j in 0..3 {
            let v = f32::from_bits(g.read::<u32>(row + 4 * j)) * k;
            g.write::<u32>(row + 4 * j, v.to_bits());
        }
    }
    let s553 = slot_target(g, w(0xb9), 553);
    let mut args = vec![ecx, rows[0], rows[1], rows[2], rows[3]];
    args.extend(pushed.iter().rev());
    // call width: 13 (five words and the four earlier calls' eight)
    g.call(s553, 0, &args).eax
}

/// `scaled_camera` (319 bytes; 4 overlays).
pub const SCALED_CAMERA: Sig = Sig {
    len: 319,
    hash: 0x6e46_556c_b5a6_e524,
    calls: &[(0x137, crate::Callee::Any)],
    imports: &[],
};
