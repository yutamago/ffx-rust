//! Small entries the own work list repeats (round 4): the same code in
//! a few DLLs with other constants, fields and slots (the instructions'
//! operands, read from each copy's code).

use crate::o4_parts::disp8;
use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The slot a `mov eax, [eax + 4 * slot]` with its `disp32` at `at`
/// calls.
fn slot_at(img: &Image, at: u32) -> u32 {
    img.word(at) / 4
}

/// A taken pool node appended to a list (`G_HEAD`/`G_TAIL`): a first
/// node starts it (`prev` 0, head and tail the node), any other goes
/// after the tail; then its `+0x8` is itself. `empty` is whether the
/// head was 0 (read before the node's `+0x4` was cleared); `tail` the
/// tail global the append reads, `tail_set` the one it writes, `head_set`
/// and `tail_first` the two a first node writes (each a copy's own
/// operand of the same global).
pub fn append_node(
    g: &mut impl crate::Mem,
    node: u32,
    empty: bool,
    (tail, tail_set): (u32, u32),
    (head_set, tail_first): (u32, u32),
) {
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(head_set, node);
        g.write::<u32>(tail_first, node);
    } else {
        let last = g.read::<u32>(tail);
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(tail_set, node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
}

/// No arguments (`magic_0168` 0x10003400, 16 bytes): the word at `*G +
/// D` (`D` at `+0x07`) set to the immediate at `+0x0b`. Returns `*G`.
pub fn store_imm<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = g.read::<u32>(img.word(f + 0x01));
    g.write::<u32>(p.wrapping_add(img.word(f + 0x07)), img.word(f + 0x0b));
    Ret::eax(p)
}

/// No arguments (`magic_0303` 0x100014b0, 17 bytes): the slot at `+0x09`
/// called with the `push imm8` at `+0x06`. Returns its `eax`.
pub fn slot_imm<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = slot_target(g, img.word(f + 0x01), slot_at(img, f + 0x09));
    let r = g.call(s, fr.ecx, &[disp8(img, f + 0x06)]);
    Ret::eax(r.eax)
}

/// No arguments (`magic_0443` 0x10001c00, 24 bytes): the slot at `+0x09`
/// called with the `push imm8` at `+0x06`. Returns 1.
pub fn slot_imm_ret1<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = slot_target(g, img.word(f + 0x01), slot_at(img, f + 0x09));
    g.call(s, fr.ecx, &[disp8(img, f + 0x06)]);
    Ret::eax(1)
}

/// No arguments (`magic_0162` 0x100039c0, 30 bytes): the word at `*G +
/// A` (`A` at `+0x09`) copied to `*G + B` and `*G + C` (`+0x11`,
/// `+0x19`; `movss`, the bits as they are). Returns `*G`.
pub fn copy_twice<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    let v = g.read::<u32>(p.wrapping_add(w(0x09)));
    g.write::<u32>(p.wrapping_add(w(0x11)), v);
    g.write::<u32>(p.wrapping_add(w(0x19)), v);
    Ret::eax(p)
}

/// `(obj)` (`magic_0276` 0x100032f0, 50 bytes): slot 237 with `(*G, K,
/// 0, V)`, `V` = `obj + 0xd0` (`G` at `+0x1f`, `K` the `push imm8` at
/// `+0x1c`), then the float at `obj + 0xdc` set to 1.0. Returns slot
/// 237's `eax`.
pub fn slot237_scale<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let s237 = slot_target(g, w(0x04), 237);
    let v = obj.wrapping_add(0xd0);
    let h = g.read::<u32>(w(0x1f));
    let r = g.call(s237, fr.ecx, &[h, disp8(img, f + 0x1c), 0, v]);
    g.write::<u32>(v.wrapping_add(0xc), 0x3f80_0000);
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0417` 0x10004920, 61 bytes): slot 719 `(P, 9, A)`
/// (`P` the operand at `+0x13`, `A` the `push imm8` at `+0x09`), slot 306
/// `(7, B, h)` with `h` slot 719's `eax` (`B` at `+0x20`; the first
/// call's words still on the stack after it), then `obj + 0x104` cleared.
/// Returns `obj`.
pub fn slot719_306<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (a, p) = (disp8(img, f + 0x09), w(0x13));
    let s719 = slot_target(g, w(0x04), 719);
    let h = g.call(s719, fr.ecx, &[p, 9, a]).eax;
    let b = disp8(img, f + 0x20);
    let s306 = slot_target(g, w(0x1b), 306);
    g.call(s306, fr.ecx, &[7, b, h, p, 9, a]);
    let obj = fr.arg(g, 0);
    g.write::<u32>(obj.wrapping_add(0x104), 0);
    Ret::eax(obj)
}

/// `(obj)` (`magic_0378` 0x10001040, 62 bytes): the slot at `+0x0c`
/// (433 or 444) with the `push imm8` at `+0x09`, `rcefTaskSetSeq(obj,
/// cb)` (slot 450, `cb` at `+0x18`) and `rcefTaskWait(obj, T)` (451, `T`
/// at `+0x2d`), each with the earlier calls' words still on the stack
/// after its own. Returns slot 451's `eax`.
pub fn slot_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = disp8(img, f + 0x09);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0c));
    g.call(s, fr.ecx, &[a]);
    let cb = w(0x18);
    let obj = fr.arg(g, 0);
    let s450 = slot_target(g, w(0x13), 450);
    g.call(s450, fr.ecx, &[obj, cb, a]);
    let t = disp8(img, f + 0x2d);
    let obj = fr.arg(g, 0);
    let s451 = slot_target(g, w(0x28), 451);
    let r = g.call(s451, fr.ecx, &[obj, t, obj, cb, a]);
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0193` 0x10001c70, 92 bytes): slot 447 `(V, K)` with
/// `V` the word at `*G + D` (`D` at `+0x0c`, `K` the `push imm8` at
/// `+0x16`), `rcefTaskSetSeq(obj, cb)` (450, `+0x2f`) and
/// `rcefTaskCallSeq(obj, cb2)` (449, `+0x3c`), each with the earlier
/// words on the stack after its own; then `V` stored in the object's
/// list `obj + 0x2c + 16 * i` (`i` the word at `obj + 0xac`). Returns
/// `2 * i` (the `eax` the original leaves).
pub fn seq_keep_447<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let k = disp8(img, f + 0x16);
    let s447 = slot_target(g, w(0x11), 447);
    g.call(s447, fr.ecx, &[v, k]);
    let obj = fr.arg(g, 0);
    let cb = w(0x2f);
    let s450 = slot_target(g, w(0x21), 450);
    g.call(s450, fr.ecx, &[obj, cb, v, k]);
    let s449 = slot_target(g, w(0x37), 449);
    g.call(s449, fr.ecx, &[obj, w(0x3c), obj, cb, v, k]);
    // add eax, eax; mov [esi + eax * 8 + 0x2c], edi
    let i = g.read::<u32>(obj.wrapping_add(0xac));
    let twice = i.wrapping_add(i);
    g.write::<u32>(
        obj.wrapping_add(twice.wrapping_mul(8)).wrapping_add(0x2c),
        v,
    );
    Ret::eax(twice)
}

/// `this` = `ecx` (`magic_0596` 0x10003900, 117 bytes): the record reset
/// (`+0x0`, `+0x1c`, `+0x24` zero, `+0x20` and `+0x28` the immediates at
/// `+0x1a` and `+0x28`, the halfword `+0x2c` zero), a pool node taken
/// ([`take`], the `rel32` at `+0x31`, the owner the operand at `+0x06`),
/// its `+0x14` the record, byte `+0x18` 1, `+0x4` cleared, appended to
/// the list ([`append_node`]: head `+0x37`/`+0x68`, tail `+0x4f`, `+0x59`,
/// `+0x6d`). Returns the node.
pub fn record_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let this = fr.ecx;
    g.write::<u32>(this, 0);
    g.write::<u32>(this.wrapping_add(0x1c), 0);
    g.write::<u32>(this.wrapping_add(0x20), w(0x1a));
    g.write::<u32>(this.wrapping_add(0x24), 0);
    g.write::<u32>(this.wrapping_add(0x28), w(0x28));
    g.write::<u16>(this.wrapping_add(0x2c), 0);
    let node = take(g, img, img.rel32_target(f + 0x31), w(0x06));
    let empty = g.read::<u32>(w(0x37)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), this);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    append_node(g, node, empty, (w(0x4f), w(0x59)), (w(0x68), w(0x6d)));
    Ret::eax(node)
}

/// No arguments (`magic_0163` 0x10002410, 135 bytes): the static record
/// `R` (operand `+0x52`) reset the same way as [`record_node`]'s (its
/// words at the operands `+0x09`, `+0x13`, `+0x1d`, `+0x27`, `+0x31`, the
/// halfword at `+0x3b`; the immediates at `+0x21` and `+0x35`), a pool
/// node taken (`rel32` at `+0x40`, owner at `+0x03`), byte `+0x18` 1,
/// `+0x14` = `R`, `+0x4` cleared, appended (head `+0x46`/`+0x7a`, tail
/// `+0x61`, `+0x6b`, `+0x7f`). Returns the node.
pub fn static_record_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0x09), 0);
    g.write::<u32>(w(0x13), 0);
    g.write::<u32>(w(0x1d), w(0x21));
    g.write::<u32>(w(0x27), 0);
    g.write::<u32>(w(0x31), w(0x35));
    g.write::<u16>(w(0x3b), 0);
    let node = take(g, img, img.rel32_target(f + 0x40), w(0x03));
    let empty = g.read::<u32>(w(0x46)) == 0;
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), w(0x52));
    g.write::<u32>(node.wrapping_add(4), 0);
    append_node(g, node, empty, (w(0x61), w(0x6b)), (w(0x7a), w(0x7f)));
    Ret::eax(node)
}

/// No arguments (`magic_0477` 0x100011c0, 133 bytes): `*G_P = ITEM`
/// (`+0x01`, `+0x07`), the part `list[d8 / 4]` (slot 941 `(*G_D + 0x10)`,
/// the entry at `+0x26`) started on the static object at `+0x2d`
/// (`pppStartPart(obj, 0x1000, program, 1)`, the wrapper's `rel32` at
/// `+0x32`), three 16-byte constants copied (`movaps`: from `+0x39`,
/// `+0x4d`, `+0x5c` to `+0x45`, `+0x55`, `+0x70`), then slot 268 `(*H,
/// 6, CB)` (`H` at `+0x69`, `CB` at `+0x61`) with the part start's words
/// still on the stack after it. Returns 1.
pub fn part_and_vectors<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    g.write::<u32>(p, w(0x07));
    let d = g.read::<u32>(g.read::<u32>(w(0x0c)).wrapping_add(0x10));
    let s941 = slot_target(g, w(0x14), 941);
    let list = g.call(s941, fr.ecx, &[d]).eax;
    let program = g.read::<u32>(list.wrapping_add(disp8(img, f + 0x26)));
    start_part(
        g,
        img,
        img.rel32_target(f + 0x32),
        w(0x2d),
        0x1000,
        program,
        1,
    );
    for (src, dst) in [(0x39, 0x45), (0x4d, 0x55), (0x5c, 0x70)] {
        for k in 0..4 {
            let v = g.read::<u32>(w(src) + 4 * k);
            g.write::<u32>(w(dst) + 4 * k, v);
        }
    }
    let h = g.read::<u32>(w(0x69));
    let s268 = slot_target(g, w(0x3e), 268);
    g.call(s268, fr.ecx, &[h, 6, w(0x61), program, 1, 0, d]);
    Ret::eax(1)
}

/// [`store_imm`] in `magic_0168` (0x10003400, 16 bytes).
pub const STORE_IMM_0168_3400: Sig = Sig {
    len: 16,
    hash: 0xcbb9_045f_3a15_a135,
    calls: &[],
    imports: &[],
};

/// [`store_imm`] in `magic_0673` (0x10002030, 16 bytes).
pub const STORE_IMM_0673_2030: Sig = Sig {
    len: 16,
    hash: 0x2664_fec1_d0c5_299a,
    calls: &[],
    imports: &[],
};

/// [`store_imm`] in `magic_0673` (0x10002060, 16 bytes).
pub const STORE_IMM_0673_2060: Sig = Sig {
    len: 16,
    hash: 0x5b5e_debf_aa87_276c,
    calls: &[],
    imports: &[],
};

/// [`store_imm`] in `magic_0673` (0x10002070, 16 bytes).
pub const STORE_IMM_0673_2070: Sig = Sig {
    len: 16,
    hash: 0x68d4_6abf_2125_99b8,
    calls: &[],
    imports: &[],
};

/// [`slot_imm`] in `magic_0303` (0x100014b0, 17 bytes).
pub const SLOT_IMM_0303_14B0: Sig = Sig {
    len: 17,
    hash: 0x7eca_668d_30d4_b34a,
    calls: &[],
    imports: &[],
};

/// [`slot_imm`] in `magic_0473` (0x100020c0, 17 bytes).
pub const SLOT_IMM_0473_20C0: Sig = Sig {
    len: 17,
    hash: 0xb68e_3002_73dd_9bad,
    calls: &[],
    imports: &[],
};

/// [`slot_imm`] in `magic_0511` (0x10002b20, 17 bytes).
pub const SLOT_IMM_0511_2B20: Sig = Sig {
    len: 17,
    hash: 0x0cf2_077c_3bef_304e,
    calls: &[],
    imports: &[],
};

/// [`slot_imm_ret1`] in `magic_0443` (0x10001c00, 24 bytes).
pub const SLOT_IMM_RET1_0443_1C00: Sig = Sig {
    len: 24,
    hash: 0x5750_251a_4b80_0cfd,
    calls: &[],
    imports: &[],
};

/// [`slot_imm_ret1`] in `magic_0540` (0x10001030, 24 bytes).
pub const SLOT_IMM_RET1_0540_1030: Sig = Sig {
    len: 24,
    hash: 0x903f_349b_edff_fdcc,
    calls: &[],
    imports: &[],
};

/// [`slot_imm_ret1`] in `magic_0540` (0x10001060, 24 bytes).
pub const SLOT_IMM_RET1_0540_1060: Sig = Sig {
    len: 24,
    hash: 0x69aa_c2c7_8a6a_4fbc,
    calls: &[],
    imports: &[],
};

/// [`copy_twice`] in `magic_0162` (0x100039c0, 30 bytes).
pub const COPY_TWICE_0162_39C0: Sig = Sig {
    len: 30,
    hash: 0x0699_6a01_f708_5082,
    calls: &[],
    imports: &[],
};

/// [`copy_twice`] in `magic_0304` (0x10003790, 30 bytes).
pub const COPY_TWICE_0304_3790: Sig = Sig {
    len: 30,
    hash: 0x84a8_4628_aafe_c923,
    calls: &[],
    imports: &[],
};

/// [`copy_twice`] in `magic_0586` (0x100035d0, 30 bytes).
pub const COPY_TWICE_0586_35D0: Sig = Sig {
    len: 30,
    hash: 0xda1d_0fa5_7513_8ee2,
    calls: &[],
    imports: &[],
};

/// [`slot237_scale`] in `magic_0276` (0x100032f0, 50 bytes).
pub const SLOT237_SCALE_0276_32F0: Sig = Sig {
    len: 50,
    hash: 0x0bd4_5330_f912_d9a0,
    calls: &[],
    imports: &[],
};

/// [`slot237_scale`] in `magic_0305` (0x10001cb0, 50 bytes).
pub const SLOT237_SCALE_0305_1CB0: Sig = Sig {
    len: 50,
    hash: 0xd16c_1003_6f7f_80c3,
    calls: &[],
    imports: &[],
};

/// [`slot237_scale`] in `magic_0417` (0x100048e0, 50 bytes).
pub const SLOT237_SCALE_0417_48E0: Sig = Sig {
    len: 50,
    hash: 0x4d83_e52e_c0da_6be4,
    calls: &[],
    imports: &[],
};

/// [`slot719_306`] in `magic_0417` (0x10004920, 61 bytes).
pub const SLOT719_306_0417_4920: Sig = Sig {
    len: 61,
    hash: 0x4e8d_1f9d_e3df_3477,
    calls: &[],
    imports: &[],
};

/// [`slot719_306`] in `magic_0419` (0x10001d10, 61 bytes).
pub const SLOT719_306_0419_1D10: Sig = Sig {
    len: 61,
    hash: 0x2c75_4de2_eeea_e5f8,
    calls: &[],
    imports: &[],
};

/// [`slot719_306`] in `magic_0432` (0x100020d0, 61 bytes).
pub const SLOT719_306_0432_20D0: Sig = Sig {
    len: 61,
    hash: 0xcf91_428e_d7fd_ef5c,
    calls: &[],
    imports: &[],
};

/// [`slot719_306`] in `magic_0572` (0x10001630, 61 bytes).
pub const SLOT719_306_0572_1630: Sig = Sig {
    len: 61,
    hash: 0x05be_d07c_16e9_d199,
    calls: &[],
    imports: &[],
};

/// [`slot_seq_wait`] in `magic_0378` (0x10001040, 62 bytes).
pub const SLOT_SEQ_WAIT_0378_1040: Sig = Sig {
    len: 62,
    hash: 0xb9e2_853f_458a_e58f,
    calls: &[],
    imports: &[],
};

/// [`slot_seq_wait`] in `magic_0521` (0x10001130, 62 bytes).
pub const SLOT_SEQ_WAIT_0521_1130: Sig = Sig {
    len: 62,
    hash: 0x0ca2_4103_c772_e8fd,
    calls: &[],
    imports: &[],
};

/// [`slot_seq_wait`] in `magic_0525` (0x10001150, 62 bytes).
pub const SLOT_SEQ_WAIT_0525_1150: Sig = Sig {
    len: 62,
    hash: 0xa400_ba53_7dc9_652d,
    calls: &[],
    imports: &[],
};

/// [`seq_keep_447`] in `magic_0193` (0x10001c70, 92 bytes).
pub const SEQ_KEEP_447_0193_1C70: Sig = Sig {
    len: 92,
    hash: 0x1b2a_4b7d_ccc9_d224,
    calls: &[],
    imports: &[],
};

/// [`seq_keep_447`] in `magic_0300` (0x10001280, 92 bytes).
pub const SEQ_KEEP_447_0300_1280: Sig = Sig {
    len: 92,
    hash: 0x12d9_eb92_ac7c_db34,
    calls: &[],
    imports: &[],
};

/// [`seq_keep_447`] in `magic_0525` (0x100015a0, 92 bytes).
pub const SEQ_KEEP_447_0525_15A0: Sig = Sig {
    len: 92,
    hash: 0xa092_ca7d_5e49_31c2,
    calls: &[],
    imports: &[],
};

/// [`seq_keep_447`] in `magic_0529` (0x100015c0, 92 bytes).
pub const SEQ_KEEP_447_0529_15C0: Sig = Sig {
    len: 92,
    hash: 0xb414_1766_662b_c614,
    calls: &[],
    imports: &[],
};

/// [`record_node`] in `magic_0596` (0x10003900, 117 bytes).
pub const RECORD_NODE_0596_3900: Sig = Sig {
    len: 117,
    hash: 0x8b71_1a0b_f20d_19b9,
    calls: &[(0x31, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`record_node`] in `magic_0348` (0x100029a0, 117 bytes).
pub const RECORD_NODE_0348_29A0: Sig = Sig {
    len: 117,
    hash: 0x2076_6ff6_3d7f_7207,
    calls: &[(0x31, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`record_node`] in `magic_0353` (0x10004a00, 117 bytes).
pub const RECORD_NODE_0353_4A00: Sig = Sig {
    len: 117,
    hash: 0x5cd4_eea8_1c4b_3a67,
    calls: &[(0x31, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`record_node`] in `magic_0612` (0x10003e60, 117 bytes).
pub const RECORD_NODE_0612_3E60: Sig = Sig {
    len: 117,
    hash: 0x80af_dc96_b770_b6f8,
    calls: &[(0x31, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`static_record_node`] in `magic_0163` (0x10002410, 135 bytes).
pub const STATIC_RECORD_NODE_0163_2410: Sig = Sig {
    len: 135,
    hash: 0x7582_26c8_1354_84ea,
    calls: &[(0x40, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`static_record_node`] in `magic_0253` (0x100048d0, 135 bytes).
pub const STATIC_RECORD_NODE_0253_48D0: Sig = Sig {
    len: 135,
    hash: 0xe853_ed64_8882_ea4d,
    calls: &[(0x40, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`static_record_node`] in `magic_0298` (0x10002850, 135 bytes).
pub const STATIC_RECORD_NODE_0298_2850: Sig = Sig {
    len: 135,
    hash: 0xb4ce_465c_9db2_a6b5,
    calls: &[(0x40, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`static_record_node`] in `magic_0508` (0x10002b80, 135 bytes).
pub const STATIC_RECORD_NODE_0508_2B80: Sig = Sig {
    len: 135,
    hash: 0x0182_de01_a552_11b4,
    calls: &[(0x40, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`part_and_vectors`] in `magic_0477` (0x100011c0, 133 bytes).
pub const PART_AND_VECTORS_0477_11C0: Sig = Sig {
    len: 133,
    hash: 0x1f0f_3592_088e_b2ed,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`part_and_vectors`] in `magic_0477` (0x100013a0, 133 bytes).
pub const PART_AND_VECTORS_0477_13A0: Sig = Sig {
    len: 133,
    hash: 0xc732_0182_87ec_75c5,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`part_and_vectors`] in `magic_0486` (0x100011c0, 133 bytes).
pub const PART_AND_VECTORS_0486_11C0: Sig = Sig {
    len: 133,
    hash: 0x38d4_7638_21a3_6f01,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`part_and_vectors`] in `magic_0486` (0x10001360, 133 bytes).
pub const PART_AND_VECTORS_0486_1360: Sig = Sig {
    len: 133,
    hash: 0xec72_c61b_a05f_1b3d,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
