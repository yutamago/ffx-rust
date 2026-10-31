//! Model flag steps and a node constructor of the own work list (round
//! 4): register-argument variants of the flag step (bit 0 of the byte
//! `+0x90` picks slot 36 on `+0x54` or slot 228 on `+0x50`; the halfword
//! `+0x90` then gets 0x200 and/or 0x120). Operands are read from the code.

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The flags `+0x90` of the model `m` or'ed with `bits`.
fn set(g: &mut impl crate::Mem, m: u32, bits: u32) {
    let v = g.read::<u16>(m + 0x90);
    g.write::<u16>(m + 0x90, v | bits as u16);
}

/// The step's slot: 36 (the `disp32` at `on`) on `+0x54` with bit 0,
/// else 228 (`off`) on `+0x50`, each with `(word, k)`. Returns whether the
/// bit was set.
fn step<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    m: u32,
    k: u32,
    (table, on, off): (u32, u32, u32),
) -> bool {
    let w = |o: u32| img.word(f + o);
    let bit = g.read::<u8>(m + 0x90) & 1 != 0;
    let (field, disp) = if bit { (0x54, on) } else { (0x50, off) };
    let a = g.read::<u32>(m + field);
    let s = slot_target(g, w(table), w(disp) / 4);
    g.call(s, m, &[a, k]);
    bit
}

/// `(a)` (`magic_0203` 0x100073c0, 84 bytes, internal: `ecx` = the
/// model, `edx` = `K`, one stack word): the step with `K` (the slots at
/// `+0x1a`/`+0x36`), 0x200 with the bit (`+0x21`), `+0x60` = `a`, then
/// 0x120 (`+0x46`). Returns 0x120.
pub fn flag_step_arg<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let m = fr.ecx;
    if step(g, img, f, m, fr.edx, (0x04, 0x1a, 0x36)) {
        set(g, m, w(0x21));
    }
    let a = fr.arg(g, 0);
    g.write::<u32>(m + 0x60, a);
    set(g, m, w(0x46));
    Ret::eax(w(0x46))
}

/// A flag step (`magic_0262` 0x10006af0, 97 bytes, internal: `ecx` = the
/// model, `edx` = `K`): the step with `K` (`+0x1a`/`+0x46`), 0x200 with
/// the bit (`+0x21`), 0x120 (`+0x2d`/`+0x53`), `+0x60` = `K`. Returns
/// 0x120.
pub fn flag_step_edx<G: Game>(g: &mut G, img: &Image, f: u32, m: u32, k: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    if step(g, img, f, m, k, (0x01, 0x1a, 0x46)) {
        set(g, m, w(0x21));
        set(g, m, w(0x2d));
        g.write::<u32>(m + 0x60, k);
    } else {
        g.write::<u32>(m + 0x60, k);
        set(g, m, w(0x53));
    }
    w(0x2d)
}

/// A node put at the head of the list `T` (`magic_0203` 0x10007940, 97
/// bytes, internal: `ecx` = the node, `edx` = its object): its vtable and
/// callback (`+0x10`, `+0x17`), `+0x14` itself, `+0x8` the old head (`T`
/// at `+0x1f`/`+0x2e`/`+0x43`, which links back to it), `+0x4` the anchor
/// (`+0x29`), the list flags (`+0x37`) masked by the `imm8` at `+0x3b`,
/// `+0x18` the object; slot 228 (`+0x4f`) with `(obj[0x50], K)` (`K` the
/// `push imm32` at `+0x04`); then `+0x1c` = 0. Returns the slot's `eax`.
pub fn object_head_node<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, obj: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(node, w(0x10));
    g.write::<u32>(node + 0x10, w(0x17));
    g.write::<u32>(node + 0x14, node);
    let head = g.read::<u32>(w(0x1f));
    g.write::<u32>(node + 8, head);
    g.write::<u32>(node + 4, w(0x29));
    let head = g.read::<u32>(w(0x2e));
    g.write::<u32>(head + 4, node);
    let flags = g.read::<u32>(w(0x37));
    g.write::<u32>(w(0x37), flags & crate::o4_parts::disp8(img, f + 0x3b));
    g.write::<u32>(w(0x43), node);
    g.write::<u32>(node + 0x18, obj);
    let a = g.read::<u32>(obj + 0x50);
    let s = slot_target(g, w(0x3d), w(0x4f) / 4);
    let r = g.call(s, node, &[a, w(0x04)]).eax;
    g.write::<u32>(node + 0x1c, 0);
    r
}

/// [`flag_step_arg`] in `magic_0203` (0x100073c0, 84 bytes).
pub const FLAG_STEP_ARG_0203_73C0: Sig = Sig {
    len: 84,
    hash: 0x3a18_2bf8_f6ef_6aef,
    calls: &[],
    imports: &[],
};

/// [`flag_step_edx`] in `magic_0262` (0x10006af0, 97 bytes).
pub const FLAG_STEP_EDX_0262_6AF0: Sig = Sig {
    len: 97,
    hash: 0x392d_17d8_d872_4adc,
    calls: &[],
    imports: &[],
};

/// [`object_head_node`] in `magic_0203` (0x10007940, 97 bytes).
pub const OBJECT_HEAD_NODE_0203_7940: Sig = Sig {
    len: 97,
    hash: 0x3916_e156_bff4_6ff2,
    calls: &[],
    imports: &[],
};
