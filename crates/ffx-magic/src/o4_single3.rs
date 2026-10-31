//! A third set of single functions of the own work list (round 4).
//! Operands are read from the code (`w(off)` = the word at `+off`).

use crate::runtime::slot_target;
use crate::sa_misc::place_me;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `(obj)` (`magic_0194` 0x100011c0, 100 bytes): slot 446 (`+0x18`)
/// with `(P[0x1a0], K)` (`P` = `*G`, `G` at `+0x04`, the field at `+0x0d`,
/// `K` at `+0x0a`), `rcefTaskSetSeq(obj, cb)` (`+0x28`, `+0x2d`) and
/// `rcefTaskCallSeq(obj, cb2)` (`+0x40`, `+0x3a`), each with the earlier
/// words after its own; then `V` = `(*G)[D]` (`+0x4e`, `D` at `+0x57`)
/// into the object's list `obj + 0x2c + 16 * obj[0xac]`. Returns `V`.
pub fn slot_seq_call_keep<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let a = g.read::<u32>(p.wrapping_add(w(0x0d)));
    let k = crate::o4_parts::disp8(img, f + 0x0a);
    let s = slot_target(g, w(0x12), w(0x18) / 4);
    g.call(s, fr.ecx, &[a, k]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x1f), w(0x28) / 4);
    g.call(s, fr.ecx, &[obj, w(0x2d), a, k]);
    let s = slot_target(g, w(0x35), w(0x40) / 4);
    g.call(s, fr.ecx, &[obj, w(0x3a), obj, w(0x2d), a, k]);
    let i = g.read::<u32>(obj.wrapping_add(0xac));
    let p = g.read::<u32>(w(0x4e));
    let v = g.read::<u32>(p.wrapping_add(w(0x57)));
    g.write::<u32>(
        obj.wrapping_add(i.wrapping_add(i).wrapping_mul(8))
            .wrapping_add(0x2c),
        v,
    );
    Ret::eax(v)
}

/// No arguments (`magic_0303` 0x100014d0, 97 bytes, a security-cookie
/// frame): slot 262 (`+0x29`) with `(*H, &v.x, &v.y, &v.z)` (`H` at
/// `+0x23`; `v` a local vector at `ebp - 0x14`), then `v.y -= K` (`K` at
/// `+0x38`), `v.w` = the immediate at `+0x45`, and [`place_me`] (`+0x4f`)
/// with `edx` = `&v`. Returns what it returns.
pub fn place_from_slot<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // ebp = esp - 4; v at ebp - 0x14
    let v = fr.esp.wrapping_sub(0x18);
    let h = g.read::<u32>(w(0x23));
    let s = slot_target(g, w(0x1d), w(0x29) / 4);
    g.call(s, fr.ecx, &[h, v, v + 4, v + 8]);
    let y = sub(
        f32::from_bits(g.read::<u32>(v + 4)),
        f32::from_bits(g.read::<u32>(w(0x38))),
    );
    g.write::<u32>(v + 0xc, w(0x45));
    g.write::<u32>(v + 4, y.to_bits());
    Ret::eax(place_me(g, img, img.rel32_target(f + 0x4f), v))
}

/// `this` = `ecx` (`magic_0379` 0x10006910, 105 bytes): a task node put
/// at the tail of the list `T` (anchor `L`, `+0x0f`/`+0x16`/`+0x28`; the
/// tail `T` at `+0x1b`/`+0x23`/`+0x42`, the first `F` at `+0x30`, the
/// count `C` at `+0x3c`): the halfword `+0x1a` = the immediate at
/// `+0x04`, `+0x10` and `+0xc` itself, `+0x14` and `+0x4` the anchor,
/// `+0x8` the old tail (which links to it, or `F` = it for an empty
/// list); four scales `+0x2c..+0x38` their immediates (`+0x49`, `+0x50`,
/// `+0x57`, `+0x5e`) and the vtable (`+0x64`). Returns the old tail.
pub fn task_node_tail<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = fr.ecx;
    g.write::<u16>(n + 0x1a, w(0x04) as u16);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, w(0x0f));
    g.write::<u32>(n + 4, w(0x16));
    let tail = g.read::<u32>(w(0x1b));
    g.write::<u32>(n + 8, tail);
    let tail = g.read::<u32>(w(0x23));
    if tail == w(0x28) {
        g.write::<u32>(w(0x30), n);
    } else {
        g.write::<u32>(tail + 4, n);
    }
    let c = g.read::<u16>(w(0x3c));
    g.write::<u16>(w(0x3c), c.wrapping_add(1));
    g.write::<u32>(w(0x42), n);
    for (k, imm) in (0u32..).zip([0x49, 0x50, 0x57, 0x5e]) {
        g.write::<u32>(n + 0x2c + 4 * k, w(imm));
    }
    g.write::<u32>(n, w(0x64));
    Ret::eax(tail)
}

/// No arguments (`magic_0263` 0x10006be0, 86 bytes): the model `M` =
/// `*G` (`+0x08`): with bit 0 of the byte `M + 0x90`, slot 36 (`+0x1c`)
/// with `(M[0x54], K)` and the halfword `M + 0x90` gets 0x200; else slot
/// 228 (`+0x38`) with `(M[0x50], K)` (`K` at `+0x0d`). Both then give it
/// 0x120 (`+0x42`) and `M + 0x60` = the immediate at `+0x50`. Returns
/// 0x120.
pub fn model_flag_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(w(0x08));
    let k = crate::o4_parts::disp8(img, f + 0x0d);
    let set = |g: &mut G, bit: u32| {
        let v = g.read::<u16>(m + 0x90);
        g.write::<u16>(m + 0x90, v | bit as u16);
    };
    if g.read::<u8>(m + 0x90) & 1 != 0 {
        let a = g.read::<u32>(m + 0x54);
        let s = slot_target(g, w(0x01), w(0x1c) / 4);
        g.call(s, fr.ecx, &[a, k]);
        set(g, w(0x23));
    } else {
        let a = g.read::<u32>(m + 0x50);
        let s = slot_target(g, w(0x01), w(0x38) / 4);
        g.call(s, fr.ecx, &[a, k]);
    }
    // jmp: the first path joins the second's `or`
    set(g, w(0x42));
    g.write::<u32>(m + 0x60, w(0x50));
    Ret::eax(w(0x42))
}

/// No arguments (`magic_0263` 0x10006b30, 87 bytes): the list `L` =
/// `*G` (`+0x02`) drops its current node `L[0x14]` (if any): the node's
/// halfword `+0x18` down by one, `L[4]`/`L[8]` relinked around it
/// (into the node's `+0x10`/`+0xc` where it was an end), `L[4]` and
/// `L[8]` back to `L` and the current node cleared. Returns `L[4]` as
/// last read (`eax` as given without a current node).
pub fn drop_current<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let l = g.read::<u32>(img.word(fr.at + 0x02));
    let node = g.read::<u32>(l + 0x14);
    if node == 0 {
        return Ret::eax(fr.eax);
    }
    let c = g.read::<u16>(node + 0x18);
    g.write::<u16>(node + 0x18, c.wrapping_sub(1));
    let next = g.read::<u32>(l + 8);
    let prev = g.read::<u32>(l + 4);
    if prev == node {
        g.write::<u32>(node + 0x10, next);
    } else {
        g.write::<u32>(prev + 8, next);
    }
    let next = g.read::<u32>(l + 8);
    let prev = g.read::<u32>(l + 4);
    if next == node {
        g.write::<u32>(node + 0xc, prev);
    } else {
        g.write::<u32>(next + 4, prev);
    }
    g.write::<u32>(l + 8, l);
    g.write::<u32>(l + 4, l);
    g.write::<u32>(l + 0x14, 0);
    Ret::eax(prev)
}

/// A model's fade set-up (`magic_0577` 0x10009170, 84 bytes, internal:
/// `ecx` = the model, `xmm1`/`xmm2` two floats): `+0xf8` = `b`; `y` =
/// `+0x64`; the halfword `+0x98` 0, `+0xe8` 1.0, `+0xc8` = `a`, `+0xd8`
/// = `y - a`; slot 286 (`+0x4a`) with `(model[0x130], y)`. Returns its
/// `eax`.
pub fn fade_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let m = fr.ecx;
    let (a, b) = (fr.xmm_f32(1), fr.xmm_f32(2));
    g.write::<u32>(m + 0xf8, b.to_bits());
    let y = f32::from_bits(g.read::<u32>(m + 0x64));
    let d = sub(y, a);
    g.write::<u16>(m + 0x98, 0);
    let h = g.read::<u32>(m + 0x130);
    g.write::<u32>(m + 0xe8, 0x3f80_0000);
    g.write::<u32>(m + 0xc8, a.to_bits());
    g.write::<u32>(m + 0xd8, d.to_bits());
    let s = slot_target(g, img.word(f + 0x1f), img.word(f + 0x4a) / 4);
    Ret::eax(g.call(s, fr.ecx, &[h, y.to_bits()]).eax)
}

/// A node put at the head of the list `T` (`magic_0577` 0x1000ac20, 98
/// bytes, internal: `ecx` = the node, `edx` = its object): its vtable and
/// callback (`+0x0e`, `+0x15`), `+0x14` itself, `+0x8` the old head
/// (`T` at `+0x1d`/`+0x2c`/`+0x43`, which links back to it), `+0x4` the
/// anchor (`+0x27`), the list flags (`+0x37`) masked by the `imm8` at
/// `+0x3b`; slot 228 (`+0x4c`) with `(obj[0x50], K)` (`K` at `+0x31`);
/// then `+0x20` = the object, `+0x1c` = 0. Returns the slot's `eax`.
pub fn head_node<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, obj: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(node, w(0x0e));
    g.write::<u32>(node + 0x10, w(0x15));
    g.write::<u32>(node + 0x14, node);
    let head = g.read::<u32>(w(0x1d));
    g.write::<u32>(node + 8, head);
    g.write::<u32>(node + 4, w(0x27));
    let head = g.read::<u32>(w(0x2c));
    g.write::<u32>(head + 4, node);
    let flags = g.read::<u32>(w(0x37));
    g.write::<u32>(w(0x37), flags & crate::o4_parts::disp8(img, f + 0x3b));
    g.write::<u32>(w(0x43), node);
    let a = g.read::<u32>(obj + 0x50);
    let s = slot_target(g, w(0x3d), w(0x4c) / 4);
    let r = g
        .call(s, node, &[a, crate::o4_parts::disp8(img, f + 0x31)])
        .eax;
    g.write::<u32>(node + 0x20, obj);
    g.write::<u32>(node + 0x1c, 0);
    r
}

/// `(a)` (`magic_0577` 0x1000b130, 92 bytes, `ecx` = the object, `edx` a
/// word): the object's task node `obj + 0x10` set up and put at the head
/// of the list `T` (its vtable and callback at `+0x06`/`+0x0d`, `obj +
/// 0x24` the object; `T` at `+0x1f`/`+0x2e`/`+0x41`, the anchor at
/// `+0x29`, the flags at `+0x37` masked by the `imm8` at `+0x3b`); then
/// `+0x4c` = `a`, `+0x30` = 0, `+0x48` = `edx`, `+0x28` = 0. Returns `a`.
pub fn object_task_head<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    g.write::<u32>(obj + 0x10, w(0x06));
    g.write::<u32>(obj + 0x20, w(0x0d));
    g.write::<u32>(obj + 0x24, obj);
    let n = obj + 0x10;
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    let head = g.read::<u32>(w(0x1f));
    g.write::<u32>(n + 8, head);
    g.write::<u32>(n + 4, w(0x29));
    let head = g.read::<u32>(w(0x2e));
    g.write::<u32>(head + 4, n);
    let flags = g.read::<u32>(w(0x37));
    g.write::<u32>(w(0x37), flags & crate::o4_parts::disp8(img, f + 0x3b));
    let a = fr.arg(g, 0);
    g.write::<u32>(w(0x41), n);
    g.write::<u32>(obj + 0x4c, a);
    g.write::<u32>(obj + 0x30, 0);
    g.write::<u32>(obj + 0x48, fr.edx);
    g.write::<u32>(obj + 0x28, 0);
    Ret::eax(a)
}

/// `(a)` (`magic_0577` 0x10008d70, 105 bytes, `ecx` = the node): as
/// [`task_node_tail`] (the first vtable at `+0x05`, the halfword `+0x1a`
/// at `+0x0d`, the anchor at `+0x18`/`+0x1f`/`+0x31`, the tail at
/// `+0x24`/`+0x2c`/`+0x4e`, the first at `+0x39`, the count at `+0x45`),
/// then `+0x54` = `a`, the second vtable (`+0x5c`) and the halfword
/// `+0x90` = the immediate at `+0x56`. Returns that immediate.
pub fn model_node_tail<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = fr.ecx;
    g.write::<u32>(n, w(0x05));
    g.write::<u16>(n + 0x1a, w(0x0d) as u16);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, w(0x18));
    g.write::<u32>(n + 4, w(0x1f));
    let tail = g.read::<u32>(w(0x24));
    g.write::<u32>(n + 8, tail);
    let tail = g.read::<u32>(w(0x2c));
    if tail == w(0x31) {
        g.write::<u32>(w(0x39), n);
    } else {
        g.write::<u32>(tail + 4, n);
    }
    let c = g.read::<u16>(w(0x45));
    g.write::<u16>(w(0x45), c.wrapping_add(1));
    let a = fr.arg(g, 0);
    g.write::<u32>(w(0x4e), n);
    g.write::<u32>(n + 0x54, a);
    g.write::<u32>(n, w(0x5c));
    g.write::<u16>(n + 0x90, w(0x56) as u16);
    Ret::eax(w(0x56))
}

/// [`slot_seq_call_keep`] in `magic_0194` (0x100011c0, 100 bytes).
pub const SLOT_SEQ_CALL_KEEP_0194_11C0: Sig = Sig {
    len: 100,
    hash: 0x78ac_6fab_a5d1_148d,
    calls: &[],
    imports: &[],
};

/// [`place_from_slot`] in `magic_0303` (0x100014d0, 97 bytes).
pub const PLACE_FROM_SLOT_0303_14D0: Sig = Sig {
    len: 97,
    hash: 0x9c1e_cc1b_a665_c2bc,
    calls: &[
        (0x4f, Callee::Sig(&crate::sa_misc::PLACE_ME)),
        (0x59, Callee::Any),
    ],
    imports: &[],
};

/// [`task_node_tail`] in `magic_0379` (0x10006910, 105 bytes).
pub const TASK_NODE_TAIL_0379_6910: Sig = Sig {
    len: 105,
    hash: 0xd45e_ac06_3b45_e448,
    calls: &[],
    imports: &[],
};

/// [`model_flag_step`] in `magic_0263` (0x10006be0, 86 bytes).
pub const MODEL_FLAG_STEP_0263_6BE0: Sig = Sig {
    len: 86,
    hash: 0xe011_275d_f736_512c,
    calls: &[],
    imports: &[],
};

/// [`drop_current`] in `magic_0263` (0x10006b30, 87 bytes).
pub const DROP_CURRENT_0263_6B30: Sig = Sig {
    len: 87,
    hash: 0x44a1_f9f0_11fc_7217,
    calls: &[],
    imports: &[],
};

/// [`fade_setup`] in `magic_0577` (0x10009170, 84 bytes).
pub const FADE_SETUP_0577_9170: Sig = Sig {
    len: 84,
    hash: 0x30b2_8de6_8381_1148,
    calls: &[],
    imports: &[],
};

/// [`head_node`] in `magic_0577` (0x1000ac20, 98 bytes).
pub const HEAD_NODE_0577_AC20: Sig = Sig {
    len: 98,
    hash: 0xef29_1908_971a_37a8,
    calls: &[],
    imports: &[],
};

/// [`object_task_head`] in `magic_0577` (0x1000b130, 92 bytes).
pub const OBJECT_TASK_HEAD_0577_B130: Sig = Sig {
    len: 92,
    hash: 0xcaeb_f8d1_6234_72d4,
    calls: &[],
    imports: &[],
};

/// [`model_node_tail`] in `magic_0577` (0x10008d70, 105 bytes).
pub const MODEL_NODE_TAIL_0577_8D70: Sig = Sig {
    len: 105,
    hash: 0x03d2_8787_34b5_25d8,
    calls: &[],
    imports: &[],
};
