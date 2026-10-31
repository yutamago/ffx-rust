//! Families of two copies from the own work list (round 4, fifth pass):
//! rcef attach chains with stores. Operands are read from each copy's
//! code (`w(off)` = the word at `+off`).

use crate::l5_objhost::list_remove;
use crate::o4_parts::disp8;
use crate::o4_small::append_node;
use crate::objcb::{PartAt, part_start};
use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `(obj)` (`magic_0313` 0x10001340, 180 bytes; 2 copies):
/// `rcefObjAttach(obj, V, A)` (`+0x1c`, `V` the word at `*G + D`, `G` at
/// `+0x04`, `D` at `+0x10`, `A` at `+0x0d`), `rcefObjSetMode(obj, M)`
/// (`+0x2e`, `M` at `+0x28`); seven words of `obj` set (`mov dword [esi +
/// disp32], imm32` at `+0x42` .. `+0x74` and `+0x86`) and one to the
/// float at `+0x39` (`+0x82`); `rcefTaskSetSeq(obj, cb)` (`+0x92`, `cb`
/// at `+0x97`) and `rcefTaskWait(obj, T)` (`+0xa7`, `T` at `+0xa4`), each
/// with the earlier words after its own. Returns the last slot's `eax`.
pub fn attach_mode_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let obj = fr.arg(g, 0);
    let a = disp8(img, f + 0x0d);
    let v = g.read::<u32>(p.wrapping_add(w(0x10)));
    let s = slot_target(g, w(0x15), w(0x1c) / 4);
    g.call(s, fr.ecx, &[obj, v, a]);
    let s = slot_target(g, w(0x23), w(0x2e) / 4);
    g.call(s, fr.ecx, &[obj, w(0x28), obj, v, a]);
    let x = g.read::<u32>(w(0x39));
    for insn in [0x42, 0x4c, 0x56, 0x60, 0x6a, 0x74] {
        g.write::<u32>(obj.wrapping_add(w(insn + 2)), w(insn + 6));
    }
    g.write::<u32>(obj.wrapping_add(w(0x82)), x);
    g.write::<u32>(obj.wrapping_add(w(0x88)), w(0x8c));
    let s = slot_target(g, w(0x3e), w(0x92) / 4);
    g.call(s, fr.ecx, &[obj, w(0x97), obj, w(0x28), obj, v, a]);
    let t = disp8(img, f + 0xa4);
    let s = slot_target(g, w(0x9f), w(0xa7) / 4);
    let args = [obj, t, obj, w(0x97), obj, w(0x28), obj, v, a];
    Ret::eax(g.call(s, fr.ecx, &args).eax)
}

/// `(obj)` (`magic_0665` 0x10001810, 186 bytes; 2 copies): the part
/// `list[d8 / 4]` started on `obj` ([`part_start`], the entry at
/// `+0x2d`, the wrapper's `rel32` at `+0x36`); `V` = `obj + 0x40`, its
/// first word 0 and the next eleven words their immediates (`mov dword
/// [esi + d8], imm32` from `+0x53`, 7 bytes each); slot 237 (`+0xa2`) with
/// `(*H, K, 0, V)` (`H` at `+0x4f`, `K` at `+0x4c`; the part start's words
/// after them); then `obj + 0x44` = the immediate at `+0xae`. Returns 1.
pub fn part_shape<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let at = PartAt {
        gp: 0x04,
        item: 0x0b,
        gd: 0x10,
        table: 0x18,
        entry: disp8(img, f + 0x2d),
        call: 0x36,
    };
    let (_, left) = part_start(g, img, f, fr.ecx, obj, at);
    let v = obj.wrapping_add(0x40);
    g.write::<u32>(v, 0);
    let h = g.read::<u32>(w(0x4f));
    for k in 0..11 {
        let insn = f + 0x53 + 7 * k;
        g.write::<u32>(obj.wrapping_add(disp8(img, insn + 2)), img.word(insn + 3));
    }
    let s = slot_target(g, w(0x45), w(0xa2) / 4);
    let mut args = vec![h, disp8(img, f + 0x4c), 0, v];
    args.extend(left);
    // call width: 8
    g.call(s, fr.ecx, &args);
    g.write::<u32>(obj.wrapping_add(0x44), w(0xae));
    Ret::eax(1)
}

/// No arguments (`magic_0162` 0x100047e0, 186 bytes; 2 copies): on `P` =
/// `*G` (`+0x07`): `P[A] -= K` (`A` at `+0x10`/`+0x2f`, `K` at `+0x18`);
/// for each of three objects from `P + E` (`E` at `+0x22`, 0x130 bytes
/// apart) taken off the list `L` ([`list_remove`], `+0x48`, `L` at
/// `+0x43`): `*G_P = ITEM` (`+0x51`, `+0x58`), slot 713 (`+0x63`) frees
/// its part and its flags (`u16 +0x124`) lose 0x8000; then `P[B]` = the
/// immediate at `+0x8a` (`B` at `+0x86`), slot 371 (`+0x90`) with `(0,
/// 0x10000)` and with `(1, 0x8000)` (the first call's words after them).
/// Returns the last slot's `eax`.
pub fn clear_three<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x07));
    let a = p.wrapping_add(w(0x10));
    let x = sub(
        f32::from_bits(g.read::<u32>(a)),
        f32::from_bits(g.read::<u32>(w(0x18))),
    );
    g.write::<u32>(p.wrapping_add(w(0x2f)), x.to_bits());
    let mut obj = p.wrapping_add(w(0x22));
    for _ in 0..3 {
        if list_remove(g, img, img.rel32_target(f + 0x48), w(0x43), obj) != 0 {
            let gp = g.read::<u32>(w(0x51));
            g.write::<u32>(gp, w(0x58));
            let s = slot_target(g, w(0x5d), w(0x63) / 4);
            g.call(s, fr.ecx, &[obj]);
            let flags = g.read::<u16>(obj.wrapping_add(0x124));
            g.write::<u16>(obj.wrapping_add(0x124), flags & 0x7fff);
        }
        obj = obj.wrapping_add(0x130);
    }
    g.write::<u32>(p.wrapping_add(w(0x86)), w(0x8a));
    let s = slot_target(g, w(0x80), w(0x90) / 4);
    g.call(s, fr.ecx, &[0, 0x1_0000]);
    let s = slot_target(g, w(0x9d), w(0xa8) / 4);
    Ret::eax(g.call(s, fr.ecx, &[1, 0x8000, 0, 0x1_0000]).eax)
}

/// `(obj)` (`magic_0664` 0x10001420, 191 bytes; 2 copies): the part
/// `list[d8 / 4]` started on `obj` ([`part_start`], the entry at
/// `+0x2d`, the wrapper's `rel32` at `+0x36`), nine words of `obj` set
/// (`mov dword [esi + d8], imm32` from `+0x49`, 7 bytes each), then its
/// scale `+0x60..+0x68`: the first set (from `+0x8f`) for the first
/// object of the block at `BASE` (`+0x3d`; `(obj - BASE) & !0x7f` = 0),
/// the second (from `+0xa7`) otherwise. Returns 1.
pub fn part_scale_by_index<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let at = PartAt {
        gp: 0x04,
        item: 0x0b,
        gd: 0x10,
        table: 0x18,
        entry: disp8(img, f + 0x2d),
        call: 0x36,
    };
    part_start(g, img, f, fr.ecx, obj, at);
    let stores = |g: &mut G, from: u32, n: u32| {
        for k in 0..n {
            let insn = f + from + 7 * k;
            g.write::<u32>(obj.wrapping_add(disp8(img, insn + 2)), img.word(insn + 3));
        }
    };
    stores(g, 0x49, 9);
    // test eax, 0xffffff80: the block's first object
    if obj.wrapping_sub(img.word(f + 0x3d)) & 0xffff_ff80 == 0 {
        stores(g, 0x8f, 3);
    } else {
        stores(g, 0xa7, 3);
    }
    Ret::eax(1)
}

/// A fade record (`magic_0269` 0x10003210, 195 bytes, internal: `ecx` =
/// the record, `xmm1`/`xmm2` two values, one stack word; 2 copies): as
/// [`crate::s3_misc::fade_task`] (a value not above 0 becomes `K`, the
/// float at `+0x07`; both kept in the globals at `+0x2d`/`+0x35`; the
/// colours, the stack word at `+0x28`, `+0x34` 0), but `+0x30` and
/// `+0x2c` are the immediates at `+0x71`/`+0x78`; then a pool node
/// taken ([`take`], `+0x7d`, the owner the operand at `+0x25`) for it:
/// `+0x14` the record, byte `+0x18` 1, `+0x4` 0, appended to the list
/// ([`append_node`]: head `+0x83`/`+0xb5`, tail `+0x9b`, `+0xa5`,
/// `+0xba`). Returns the node.
pub fn fade_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = f32::from_bits(g.read::<u32>(w(0x07)));
    // comiss 0, x; jb: kept when 0 < x or unordered
    let keep = |x: f32| if 0.0 < x || x.is_nan() { x } else { k };
    let a = keep(fr.xmm_f32(1));
    let b = keep(fr.xmm_f32(2));
    let rec = fr.ecx;
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x2d), a.to_bits());
    g.write::<u32>(w(0x35), b.to_bits());
    for off in [8, 4, 0] {
        g.write::<u32>(rec + off, a.to_bits());
    }
    g.write::<u32>(rec + 0xc, 0x3f80_0000);
    for off in [0x18, 0x14, 0x10] {
        g.write::<u32>(rec + off, b.to_bits());
    }
    g.write::<u32>(rec + 0x1c, 0x3f80_0000);
    g.write::<u32>(rec + 0x28, arg);
    g.write::<u32>(rec + 0x34, 0);
    g.write::<u32>(rec + 0x30, w(0x71));
    g.write::<u32>(rec + 0x2c, w(0x78));
    let node = take(g, img, img.rel32_target(f + 0x7d), w(0x25));
    let empty = g.read::<u32>(w(0x83)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), rec);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    append_node(g, node, empty, (w(0x9b), w(0xa5)), (w(0xb5), w(0xba)));
    node
}

/// `(obj)` (`magic_0421` 0x100014d0, 198 bytes; 2 copies):
/// `rcefObjAttach(obj, V, A)` (`+0x1c`, `V` the word at `*G + D`, `G` at
/// `+0x04`, `D` at `+0x10`, `A` at `+0x0d`), `rcefObjSetMode(obj, M)`
/// (`+0x2e`, `M` the `push imm32` at `+0x28`), eight words of `obj` set
/// (from `+0x45`, 10 bytes each), `rcefMainSetPar(P, obj, X, Y)` (`+0x97`,
/// `P` = `*G` read at `+0x41` before the stores, `X`/`Y` at
/// `+0x3d`/`+0x3b`), `rcefTaskSetSeq(obj, cb)` (`+0xa9`, `+0xa3`) and
/// `rcefTaskWait(obj, T)` (`+0xb9`, `T` at `+0xb6`), each with the earlier
/// words after its own. Returns the last slot's `eax`.
pub fn attach_mode32_par_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let obj = fr.arg(g, 0);
    let v = g.read::<u32>(p.wrapping_add(w(0x10)));
    let mut stack = vec![obj, v, disp8(img, f + 0x0d)];
    let s = slot_target(g, w(0x15), w(0x1c) / 4);
    g.call(s, fr.ecx, &stack);
    stack.splice(0..0, [obj, w(0x28)]);
    let s = slot_target(g, w(0x23), w(0x2e) / 4);
    // call width: 5
    g.call(s, fr.ecx, &stack);
    let p2 = g.read::<u32>(w(0x41));
    for k in 0..8 {
        let insn = f + 0x45 + 10 * k;
        g.write::<u32>(obj.wrapping_add(img.word(insn + 2)), img.word(insn + 6));
    }
    stack.splice(0..0, [p2, obj, disp8(img, f + 0x3d), disp8(img, f + 0x3b)]);
    let s = slot_target(g, w(0x36), w(0x97) / 4);
    // call width: 9
    g.call(s, fr.ecx, &stack);
    stack.splice(0..0, [obj, w(0xa3)]);
    let s = slot_target(g, w(0x9e), w(0xa9) / 4);
    // call width: 11
    g.call(s, fr.ecx, &stack);
    stack.splice(0..0, [obj, disp8(img, f + 0xb6)]);
    let s = slot_target(g, w(0xb1), w(0xb9) / 4);
    // call width: 13
    Ret::eax(g.call(s, fr.ecx, &stack).eax)
}

/// `(obj)` (`magic_0661` 0x100018d0, 184 bytes; 3 copies): the part
/// `list[d8 / 4]` started on `obj` ([`part_start`], the entry at
/// `+0x2d`, the wrapper's `rel32` at `+0x36`); `i` = the first of 14
/// pairs at `T` (`+0x40`; pair `k` is the words `T + 8k - 4`, `T + 8k`)
/// that is `(1, 0)`, or -1; slot 325 (`+0x6f`) with `(obj + 0x40, BASE +
/// 16 * i)` (`BASE` at `+0x5f`); then eight words of `obj` set (`mov
/// dword [esi + d8], imm32` from `+0x78`, 7 bytes each). Returns 1.
pub fn part_free_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let at = PartAt {
        gp: 0x04,
        item: 0x0b,
        gd: 0x10,
        table: 0x18,
        entry: disp8(img, f + 0x2d),
        call: 0x36,
    };
    part_start(g, img, f, fr.ecx, obj, at);
    let t = w(0x40);
    let i = (0..14u32)
        .find(|&k| g.read::<u32>(t + 8 * k - 4) == 1 && g.read::<u32>(t + 8 * k) == 0)
        .unwrap_or(u32::MAX);
    let addr = i.wrapping_shl(4).wrapping_add(w(0x5f));
    let s = slot_target(g, w(0x69), w(0x6f) / 4);
    g.call(s, fr.ecx, &[obj.wrapping_add(0x40), addr]);
    for k in 0..8 {
        let insn = f + 0x78 + 7 * k;
        g.write::<u32>(obj.wrapping_add(disp8(img, insn + 2)), img.word(insn + 3));
    }
    Ret::eax(1)
}

/// [`attach_mode_wait`] in `magic_0313` (0x10001340, 180 bytes).
pub const ATTACH_MODE_WAIT_0313_1340: Sig = Sig {
    len: 180,
    hash: 0xa4e0_2757_2ea6_13d2,
    calls: &[],
    imports: &[],
};

/// [`attach_mode_wait`] in `magic_0313` (0x10001630, 180 bytes).
pub const ATTACH_MODE_WAIT_0313_1630: Sig = Sig {
    len: 180,
    hash: 0x19b6_2503_f044_4019,
    calls: &[],
    imports: &[],
};

/// [`part_shape`] in `magic_0665` (0x10001810, 186 bytes).
pub const PART_SHAPE_0665_1810: Sig = Sig {
    len: 186,
    hash: 0xcdeb_9835_7505_0c62,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`part_shape`] in `magic_0666` (0x100017a0, 186 bytes).
pub const PART_SHAPE_0666_17A0: Sig = Sig {
    len: 186,
    hash: 0xf7a4_2ae7_6059_0fc1,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`clear_three`] in `magic_0162` (0x100047e0, 186 bytes).
pub const CLEAR_THREE_0162_47E0: Sig = Sig {
    len: 186,
    hash: 0x546b_8197_37ea_8c48,
    calls: &[(0x48, Callee::Sig(&crate::l5_objhost::LIST_REMOVE))],
    imports: &[],
};

/// [`clear_three`] in `magic_0598` (0x10007e30, 186 bytes).
pub const CLEAR_THREE_0598_7E30: Sig = Sig {
    len: 186,
    hash: 0xd1b6_3505_3e48_04f0,
    calls: &[(0x48, Callee::Sig(&crate::l5_objhost::LIST_REMOVE))],
    imports: &[],
};

/// [`part_scale_by_index`] in `magic_0664` (0x10001420, 191 bytes).
pub const PART_SCALE_BY_INDEX_0664_1420: Sig = Sig {
    len: 191,
    hash: 0xb179_b57d_1ba0_a94f,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`part_scale_by_index`] in `magic_0665` (0x10001610, 191 bytes).
pub const PART_SCALE_BY_INDEX_0665_1610: Sig = Sig {
    len: 191,
    hash: 0x7e0c_68a4_4508_98c6,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`fade_record`] in `magic_0269` (0x10003210, 195 bytes).
pub const FADE_RECORD_0269_3210: Sig = Sig {
    len: 195,
    hash: 0x6118_719f_51bb_6782,
    calls: &[(0x7d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`fade_record`] in `magic_0325` (0x10002250, 195 bytes).
pub const FADE_RECORD_0325_2250: Sig = Sig {
    len: 195,
    hash: 0x4564_8e0f_e622_fce2,
    calls: &[(0x7d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`attach_mode32_par_wait`] in `magic_0421` (0x100014d0, 198 bytes).
pub const ATTACH_MODE32_PAR_WAIT_0421_14D0: Sig = Sig {
    len: 198,
    hash: 0xbbf4_4d40_6823_f698,
    calls: &[],
    imports: &[],
};

/// [`attach_mode32_par_wait`] in `magic_0535` (0x100014b0, 198 bytes).
pub const ATTACH_MODE32_PAR_WAIT_0535_14B0: Sig = Sig {
    len: 198,
    hash: 0x4e6d_46a2_1c46_61d1,
    calls: &[],
    imports: &[],
};

/// [`part_free_pair`] in `magic_0661` (0x100018d0, 184 bytes).
pub const PART_FREE_PAIR_0661_18D0: Sig = Sig {
    len: 184,
    hash: 0x9cd8_17f3_fce1_51bd,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`part_free_pair`] in `magic_0661` (0x10001990, 184 bytes).
pub const PART_FREE_PAIR_0661_1990: Sig = Sig {
    len: 184,
    hash: 0xe6d5_5598_cfa3_85e9,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`part_free_pair`] in `magic_0661` (0x10001b60, 184 bytes).
pub const PART_FREE_PAIR_0661_1B60: Sig = Sig {
    len: 184,
    hash: 0x4764_6569_cf50_2601,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
