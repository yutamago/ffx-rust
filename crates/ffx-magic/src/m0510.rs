//! `magic_0510.dll`'s own functions (besides the runtime and the shared
//! code): the steps of a battle sequence. Each step is a callback the
//! game calls with its sequence object `seq`; it calls the sequence
//! slots (434, 449, 450, 451, ...) and names the next step. `S = *G_S`
//! (operand `+0x04`/`+0x05`/`+0x0b`) is the battle's state block (`+0x198`
//! the actor, `+0x1a0` the target, `+0x4` a handle).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// The sequence's first step (0x10001000, `(seq)`, pushed by
/// 0x10002005): thirteen state words 0; slot 367 `(S[4], target, 0, 0,
/// &l, 0, 0, 0)` fills the local `l` (`ebp - 4`), whose bit 8 goes to
/// the flag at operand `+0xbc`; slot 523 `(0, flag)`, slot 481
/// `(target)` a handle `h` (`+0xe8`) and slot 14 `(h, &A, &B, &C)` its
/// three words; slot 448 `(1, 0)`; the flag at `+0x13d` says whether
/// slot 520 `()` answers 0x2050000; slot 450 `(seq, NEXT)` and 449
/// `(seq, END)` (the earlier words left on the stack). Returns slot
/// 449's `eax`.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let l = fr.esp.wrapping_sub(8);
    let s = g.read::<u32>(w(0x5));
    let target = g.read::<u32>(s + 0x1a0);
    for op in [
        0x20, 0x2a, 0x34, 0x3e, 0x48, 0x52, 0x5c, 0x66, 0x70, 0x7a, 0x84, 0x8e,
    ] {
        g.write::<u32>(w(op), 0);
    }
    let a = g.read::<u32>(s + 4);
    let t = slot_target(g, w(0x9b), 367);
    g.write::<u32>(w(0xa1), 0);
    let mut stack = vec![a, target, 0, 0, l, 0, 0, 0];
    g.call(t, 0, &stack);
    let flag = (g.read::<u32>(l).cast_signed() >> 8).cast_unsigned() & 1;
    g.write::<u32>(w(0xbc), flag);
    let t = slot_target(g, w(0xc1), 523);
    stack.splice(0..0, [0, flag]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xd0), 481);
    stack.insert(0, target);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0xe8), h);
    let t = slot_target(g, w(0xf3), 14);
    stack.splice(0..0, [h, w(0xed), w(0xe3), w(0xde)]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xfd), 448);
    stack.splice(0..0, [1, 0]);
    // call width: 17
    g.call(t, 0, &stack);
    // add esp, 0x44
    let t = slot_target(g, w(0x10e), 520);
    let r = g.call(t, 0, &[]).eax;
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x128), 450);
    g.write::<u32>(w(0x13d), u32::from(r == 0x0205_0000));
    let mut stack = vec![seq, w(0x136)];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x144), 449);
    stack.splice(0..0, [seq, w(0x149)]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001000 (350 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 350,
    hash: 0x7408_f7a3_4e7c_8a40,
    calls: &[],
    imports: &[],
};

/// A step (0x100011b0, `(seq)`, pushed by 0x10001178): slot 444 `(0)`,
/// slot 522 `(0, 0)`; slot 434 `(*G_S, CB)` with `CB` the callback at
/// operand `+0x47` when the flag at `+0x39` is set, else `+0x4e`; unless
/// the flag at `+0x5f` is set, slot 486 `(actor, 2, 0, 0, 20, 32, 32,
/// 15.0, 0, 0, 0)`; slot 447 `(actor, 200)`, slot 450 `(seq, NEXT)` and
/// 451 `(seq, 0x13)` (the earlier words left on the stack). Returns
/// slot 451's `eax`.
pub fn e11b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x4));
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x10), 444);
    g.call(t, 0, &[0]);
    let t = slot_target(g, w(0x1f), 522);
    g.call(t, 0, &[0, 0, 0]);
    let t = slot_target(g, w(0x30), 434);
    let cb = if g.read::<u32>(w(0x39)) != 0 {
        w(0x47)
    } else {
        w(0x4e)
    };
    let gs = g.read::<u32>(w(0x54));
    g.call(t, 0, &[gs, cb]);
    if g.read::<u32>(w(0x5f)) == 0 {
        let t = slot_target(g, w(0x67), 486);
        g.call(
            t,
            0,
            &[actor, 2, 0, 0, 0x14, 0x20, 0x20, 0x4170_0000, 0, 0, 0],
        );
    }
    let t = slot_target(g, w(0xac), 447);
    let mut stack = vec![actor, 0xc8];
    g.call(t, 0, &stack);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0xbf), 450);
    stack.splice(0..0, [seq, w(0xcd)]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xd5), 451);
    stack.splice(0..0, [seq, 0x13]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x100011b0 (234 bytes, 1 overlay).
pub const E11B0: Sig = Sig {
    len: 234,
    hash: 0x9e9b_9021_4e00_2ec8,
    calls: &[],
    imports: &[],
};

/// A step (0x100012a0, `(seq)`, pushed by 0x1000127d): slot 522 `(0,
/// 1)`, three slot 434 `(*G_S, CB)` callbacks (operands `+0x1a`,
/// `+0x32`, `+0x4a`), slot 450 `(seq, NEXT)` and 451 `(seq, 0x18)` (the
/// earlier words left on the stack). Returns slot 451's `eax`.
pub fn e12a0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x4), 522);
    let mut stack = vec![0, 1];
    g.call(t, 0, &stack);
    for (tab, cb, gs) in [
        (0x15u32, 0x1au32, 0x20u32),
        (0x2d, 0x32, 0x38),
        (0x45, 0x4a, 0x50),
    ] {
        let t = slot_target(g, w(tab), 434);
        let s = g.read::<u32>(w(gs));
        stack.splice(0..0, [s, w(cb)]);
        g.call(t, 0, &stack);
    }
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x5d), 450);
    stack.splice(0..0, [seq, w(0x62)]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x72), 451);
    stack.splice(0..0, [seq, 0x18]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x100012a0 (136 bytes, 1 overlay).
pub const E12A0: Sig = Sig {
    len: 136,
    hash: 0x13e3_5c0d_379f_2013,
    calls: &[],
    imports: &[],
};

/// A step (0x10001380, `(seq)`, pushed by 0x10001349): unless the flag
/// at operand `+0x05` is set, slot 486 `(actor, 1, 0, 0, 10, 32, 32, 0,
/// 1.0, 0.4, 0)`; slot 447 `(actor, 201)`, slot 450 `(seq, NEXT)` and
/// 449 `(seq, END)` (the earlier words left on the stack); the actor
/// goes to the sequence's entry `seq[0x2c + 16 * seq[0xac]]`. Returns
/// twice `seq[0xac]`.
pub fn e1380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let flag = g.read::<u32>(w(0x5));
    let s = g.read::<u32>(w(0xb));
    let actor = g.read::<u32>(s + 0x198);
    if flag == 0 {
        let t = slot_target(g, w(0x1a), 486);
        g.call(
            t,
            0,
            &[actor, 1, 0, 0, 0xa, 0x20, 0x20, 0, ONE, 0x3ecc_cccd, 0],
        );
    }
    let t = slot_target(g, w(0x5f), 447);
    let mut stack = vec![actor, 0xc9];
    g.call(t, 0, &stack);
    let seq = fr.arg(g, 0);
    let t = slot_target(g, w(0x72), 450);
    stack.splice(0..0, [seq, w(0x80)]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x88), 449);
    stack.splice(0..0, [seq, w(0x8d)]);
    g.call(t, 0, &stack);
    let n = g
        .read::<u32>(seq + 0xac)
        .wrapping_add(g.read::<u32>(seq + 0xac));
    g.write::<u32>(
        seq.wrapping_add(n.wrapping_mul(8)).wrapping_add(0x2c),
        actor,
    );
    Ret::eax(n)
}

/// 0x10001380 (173 bytes, 1 overlay).
pub const E1380: Sig = Sig {
    len: 173,
    hash: 0x47a8_49b9_b2c5_abf5,
    calls: &[],
    imports: &[],
};

/// A step (0x10001680, `(seq)`, pushed by 0x100012ea): slot 442 `(seq,
/// actor, 0)`; the sequence's point `+0xd0` `(0, -12, -30, 0)` and
/// scales `+0x130` `(0.1, 0.1, 0.1, 0)`; slot 450 `(seq, NEXT)` and 451
/// `(seq, 0x17)` (the earlier words left on the stack). Returns slot
/// 451's `eax`.
pub fn e1680<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x4));
    let seq = fr.arg(g, 0);
    let actor = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x15), 442);
    let mut stack = vec![seq, actor, 0];
    g.call(t, 0, &stack);
    for (off, v) in [
        (0xd0u32, 0u32),
        (0xd4, 0xc140_0000),
        (0xd8, 0xc1f0_0000),
        (0xdc, 0),
        (0x130, 0x3dcc_cccd),
        (0x134, 0x3dcc_cccd),
        (0x138, 0x3dcc_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(seq + off, v);
    }
    let t = slot_target(g, w(0x23), 450);
    stack.splice(0..0, [seq, w(0x7e)]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x86), 451);
    stack.splice(0..0, [seq, 0x17]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001680 (155 bytes, 1 overlay).
pub const E1680: Sig = Sig {
    len: 155,
    hash: 0x9a76_0f39_9a34_bf91,
    calls: &[],
    imports: &[],
};
