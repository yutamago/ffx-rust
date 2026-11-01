//! `magic_0688.dll`'s own functions (besides the runtime and the shared
//! code; its cue start is [`crate::m0638::cue_start`]).

use crate::ob_model::st0_f32;
use crate::r12h_start::append;
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// A task node (the pool take at the `rel32` `call`, owner `CB` at
/// `cb`) of kind `kind` appended to the task list (operands `ops`, as
/// [`append`] takes them).
fn task_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (cb, call, kind): (u32, u32, u8),
    ops: [u32; 5],
) {
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), img.word(f + cb));
    g.write::<u8>(node.wrapping_add(0x18), kind);
    append(g, img, f, node, ops);
}

/// The effect's start (0x10003da0, no arguments): six globals from
/// immediates (three buffers, the record pointer, a count 20, a word 0);
/// the actors looked up ([`crate::sb_misc::lookup_actors`]); slot 719
/// `(P, 8, 0)` into `G_H`; the part set up
/// ([`crate::l5_objhost::part_init`]); the record's `+0x130` 0; the
/// script block's callback, a word 0, a halfword 0 and a byte 1; a task
/// node (kind 1, callback `CB1`) appended to the task list; the records
/// set up ([`crate::sa_misc::records_init`] on `R`); a second task node
/// (kind 2, `CB2`). Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    for o in [0x00u32, 0x0a, 0x14, 0x1e, 0x28, 0x32] {
        g.write::<u32>(w(o + 2), w(o + 6));
    }
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x3d));
    let t = slot_target(g, w(0x42), 719);
    // call width: 3
    let h = g.call(t, 0, &[w(0x51), 8, 0]).eax;
    g.write::<u32>(w(0x5b), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x60));
    let rec = g.read::<u32>(w(0x65));
    g.write::<u32>(rec.wrapping_add(0x130), 0);
    g.write::<u32>(w(0x7d), w(0x81));
    g.write::<u32>(w(0x87), w(0x8b));
    g.write::<u16>(w(0x92), w(0x96) as u16);
    g.write::<u8>(w(0x9a), w(0x9e) as u8);
    task_node(g, img, f, (0x77, 0xa0, 1), [0xa6, 0xba, 0xd9, 0xcc, 0xd9]);
    crate::sa_misc::records_init(g, img, img.rel32_target(f + 0xde), w(0xd1));
    task_node(
        g,
        img,
        f,
        (0xe3, 0xe8, 2),
        [0xee, 0x102, 0x10f, 0x11d, 0x125],
    );
    Ret::eax(0)
}

/// [`start`] (300 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 300,
    hash: 0x5333_9212_77cc_b3c8,
    calls: &[
        (0x3d, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x60, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xa0, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xde, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0xe8, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The script step (0x10003a40, a task callback, cdecl `(node)`): the
/// script block `B` (operand `+0x0f`) cleared (`memset`), its halfword
/// `+0x2` gets bit 7, its word `+0x14` = `*G_S` (`+0x25`); the script run
/// ([`crate::sa_script::run`]). When it answers 1 the record (`*G_R`,
/// `+0x06`, read first) gets `+0x130` = 1 and the node goes back to the
/// pool ([`crate::sa_pool::give_back`], a tail jump). Returns the run's
/// answer, or the give-back's.
pub fn script_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = g.read::<u32>(w(0x06));
    g.write_bytes(w(0x0f), &[0; 0x30]); // memset(B, 0, 0x30)
    let v = g.read::<u16>(w(0x20));
    g.write::<u16>(w(0x20), v | 0x80);
    let s = g.read::<u32>(w(0x25));
    g.write::<u32>(w(0x2d), s);
    let r = crate::sa_script::run(g, img, img.rel32_target(f + 0x32));
    if r != 1 {
        return Ret::eax(r);
    }
    let node = fr.arg(g, 0);
    g.write::<u32>(rec.wrapping_add(0x130), 1);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x47),
        node,
    ))
}

/// [`script_step`] (78 bytes, 1 overlay).
pub const SCRIPT_STEP: Sig = Sig {
    len: 78,
    hash: 0xe377_7dc1_8268_23ef,
    calls: &[
        (0x14, Callee::Import("memset")),
        (0x32, Callee::Sig(&crate::sa_script::RUN)),
        (0x47, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// A spark record started (0x10003b60, no arguments; a /GS frame with
/// the vector `V` at `ebp - 0x14`): a record from the pool
/// ([`crate::sb_misc::pool_take`]; none: returns 0); slot 719 `(P, 9,
/// 0)` its `+0x30`, `+0x38` -1 and `+0x3a` 0x200 (halfwords), `+0x34` 1;
/// `V` = slot 314 `(2.0, 1.0, 1.0)` (random floats in ST0) times `K`,
/// `w` 1.0; slot 237 `(*G_M, T[rand() % 5], V, rec)`; `+0xc` 1.0, `+0x10`
/// slot 314 `(5.0)`, `+0x14` `-(slot 404 (5.0) + A)`, `+0x18` slot 314
/// `(5.0)`, `+0x1c` slot 314 `(pi)`, `+0x24` 0, `+0x20` the negated slot
/// 314 `(0.02)` times `+0x10`, `+0x2c` 1.0, `+0x28` the negated slot 314
/// `(0.02)` times `+0x18`, `+0x4c` -1,
/// `+0x50` 0, `+0x3c` `(slot 314 (0.2) + B) * C`. Each float slot gets
/// the one word the original keeps on its stack (`mov [esp], x`).
/// Returns the last slot 314's `eax`.
pub fn spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x12));
    if rec == 0 {
        return Ret::eax(0);
    }
    let t = slot_target(g, w(0x22), 719);
    // call width: 3
    let h = g.call(t, 0, &[w(0x31), 9, 0]).eax;
    g.write::<u32>(rec + 0x30, h);
    g.write::<u16>(rec + 0x38, 0xffff);
    g.write::<u16>(rec + 0x3a, 0x200);
    g.write::<u32>(rec + 0x34, 1);
    // slot 314 or 404 with the one word on the stack; ST0 as `f32`
    let float = |g: &mut G, table: u32, slot: u32, x: u32| {
        let t = slot_target(g, w(table), slot);
        // call width: 1
        let out = g.call(t, 0, &[x]);
        (st0_f32(out.st0), out.eax)
    };
    // ebp = esp - 8 (push ebp, the return address); V at ebp - 0x14
    let v = fr.esp.wrapping_sub(0x18);
    let (a, _) = float(g, 0x4b, 314, 0x4000_0000);
    g.write::<f32>(v, mul(a, k(g, 0x74)));
    let (a, _) = float(g, 0x79, 314, ONE);
    g.write::<f32>(v + 4, mul(a, k(g, 0xa2)));
    let (a, _) = float(g, 0x92, 314, ONE);
    g.write::<u32>(v + 0xc, ONE);
    g.write::<f32>(v + 8, mul(a, k(g, 0xc6)));
    let r = g.rand().cast_signed() % 5;
    let entry = g.read::<u32>(w(0xec).wrapping_add((r * 4).cast_unsigned()));
    let m = g.read::<u32>(w(0xf8));
    let t = slot_target(g, w(0xf2), 237);
    // call width: 5 (the float slots' word under them)
    g.call(t, 0, &[m, entry, v, rec, ONE]);
    g.write::<u32>(rec + 0xc, ONE);
    let (a, _) = float(g, 0x105, 314, 0x40a0_0000);
    g.write::<f32>(rec + 0x10, a);
    let sign = g.read::<u32>(w(0x158));
    let (a, _) = float(g, 0x126, 404, 0x40a0_0000);
    let a = f32::from_bits(add(a, k(g, 0x145)).to_bits() ^ sign);
    g.write::<f32>(rec + 0x14, a);
    let (a, _) = float(g, 0x14a, 314, 0x40a0_0000);
    g.write::<f32>(rec + 0x18, a);
    let (a, _) = float(g, 0x16d, 314, 0x4049_0fdb);
    g.write::<f32>(rec + 0x1c, a);
    let (a, _) = float(g, 0x184, 314, 0x3ca3_d70a);
    let sign = g.read::<u32>(w(0x1a7));
    let a = mul(
        f32::from_bits(a.to_bits() ^ sign),
        g.read::<f32>(rec + 0x10),
    );
    g.write::<u32>(rec + 0x24, 0);
    g.write::<f32>(rec + 0x20, a);
    let (a, _) = float(g, 0x198, 314, 0x3ca3_d70a);
    let sign = g.read::<u32>(w(0x1db));
    let a = mul(
        f32::from_bits(a.to_bits() ^ sign),
        g.read::<f32>(rec + 0x18),
    );
    g.write::<u32>(rec + 0x2c, ONE);
    g.write::<f32>(rec + 0x28, a);
    let (a, eax) = float(g, 0x1cc, 314, 0x3e4c_cccd);
    let a = mul(add(a, k(g, 0x20e)), k(g, 0x224));
    g.write::<u32>(rec + 0x4c, u32::MAX);
    g.write::<u32>(rec + 0x50, 0);
    g.write::<f32>(rec + 0x3c, a);
    Ret::eax(eax)
}

/// [`spark`] (572 bytes, 1 overlay).
pub const SPARK: Sig = Sig {
    len: 572,
    hash: 0xf029_058e_4326_fbb0,
    calls: &[
        (0x12, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x234, Callee::Any),
    ],
    imports: &[(0xd8, "rand")],
};
