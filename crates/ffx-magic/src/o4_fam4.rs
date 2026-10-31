//! Families of two copies from the own work list (round 4, fourth
//! pass). Operands are read from each copy's code (`w(off)` = the word
//! at `+off`).

use crate::o4_parts::disp8;
use crate::o4_small::append_node;
use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::sb_misc::deal;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// No arguments (`magic_0307` 0x100024c0, 132 bytes; 2 copies): on `P` =
/// `*G` (`+0x08`): `P[A]` = its immediate (`+0x0e`/`+0x12`), `P[B]` = slot
/// 562 (`+0x18`, no words; `B` at `+0x20`), the halfword `P[C]` = 0x8000
/// (`+0x29`/`+0x2d`), the byte `P[D]` = 0 (`+0x31`/`+0x35`), one word its
/// immediate (`+0x4a`/`+0x4e`) and eleven words 0 (the fields at `+0x38`,
/// `+0x3e`, `+0x44`, `+0x54`, ... `+0x7e`). Returns 0.
pub fn effect_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x08));
    let at = |off: u32| p.wrapping_add(w(off));
    g.write::<u32>(at(0x0e), w(0x12));
    let s = slot_target(g, w(0x01), w(0x18) / 4);
    let r = g.call(s, fr.ecx, &[]).eax;
    g.write::<u32>(at(0x20), r);
    g.write::<u16>(at(0x29), w(0x2d) as u16);
    g.write::<u8>(at(0x31), w(0x35) as u8);
    for d in [0x38, 0x3e, 0x44] {
        g.write::<u32>(at(d), 0);
    }
    g.write::<u32>(at(0x4a), w(0x4e));
    for d in [0x54, 0x5a, 0x60, 0x66, 0x6c, 0x72, 0x78, 0x7e] {
        g.write::<u32>(at(d), 0);
    }
    Ret::eax(0)
}

/// A model's mesh flags (`magic_0391` 0x10003e40, 132 bytes, internal:
/// `ecx` = the model; 2 copies): for each of the `u16 +0xa` mesh entries
/// in the table at `+0x18` (a zero entry skipped) and each of its `i16
/// +0x6` parts (the part's `i16` offset at `+0x10 + 8 * k` from the
/// entry): a part whose kind (`+0x10`, 14 bits) is `K` (the immediate at
/// `+0x49`) and whose packet (`+0x4` from the part, relative) has the
/// byte `+0x4` = 0x48 gets 0x44 there. Returns the model.
pub fn mesh_flags<G: Game>(g: &mut G, img: &Image, f: u32, model: u32) -> u32 {
    let kind = img.word(f + 0x49);
    let table = g.read::<u32>(model + 0x18);
    // both counts are read again after each pass (a store could reach them)
    let mut j = 0u32;
    while j < u32::from(g.read::<u16>(model + 0xa)) {
        let e = g.read::<u32>(table.wrapping_add(4 * j));
        j += 1;
        if e == 0 {
            continue;
        }
        let mut k = 0i32;
        while k < i32::from(g.read::<u16>(e + 6).cast_signed()) {
            let off = i32::from(
                g.read::<u16>(e.wrapping_add(0x10 + 8 * k.cast_unsigned()))
                    .cast_signed(),
            );
            let part = e.wrapping_add(off.cast_unsigned());
            let packet = g.read::<u32>(part.wrapping_add(4)).wrapping_add(part);
            if g.read::<u32>(part.wrapping_add(0x10)) & 0x3fff == kind
                && g.read::<u8>(packet.wrapping_add(4)) == 0x48
            {
                g.write::<u8>(packet.wrapping_add(4), 0x44);
            }
            k += 1;
        }
    }
    model
}

/// No arguments (`magic_0391` 0x10002a20, 134 bytes; 2 copies): on `P` =
/// `*G` (`+0x03`): the deal into `P + E` ([`deal`], `+0x0e`, `E` the `lea`
/// displacement at `+0x09`), its result in `P[A]` (`+0x1b`), `P[B]`,
/// `P[C]`, `P[D]` 0 (`+0x21`, `+0x2b`, `+0x35`); unless `P[F]` (`+0x14`,
/// read before the stores) is -1, `P[C]` = 3; else slot 625 (`+0x54`)
/// with `(1)`. Then slot 626 (`+0x66`) with `(0)` and slot 540 (`+0x7b`)
/// with `(*H, 1)` (`H` at `+0x75`; the 626 word after them). Returns its
/// `eax`.
pub fn deal_then_slots<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x03));
    let at = |off: u32| p.wrapping_add(w(off));
    // push esi; call: the deal's return address 8 below the entry's
    let r = deal(
        g,
        img,
        img.rel32_target(f + 0x0e),
        fr.esp.wrapping_sub(8),
        at(0x09),
    );
    let none = g.read::<u32>(at(0x14)) == u32::MAX;
    g.write::<u32>(at(0x1b), r);
    for d in [0x21, 0x2b, 0x35] {
        g.write::<u32>(at(d), 0);
    }
    let table = w(0x4c);
    if none {
        let s = slot_target(g, table, w(0x54) / 4);
        g.call(s, fr.ecx, &[1]);
    } else {
        g.write::<u32>(at(0x41), w(0x45));
    }
    let s = slot_target(g, w(0x5e), w(0x66) / 4);
    g.call(s, fr.ecx, &[0]);
    let h = g.read::<u32>(w(0x75));
    let s = slot_target(g, w(0x6d), w(0x7b) / 4);
    Ret::eax(g.call(s, fr.ecx, &[h, 1, 0]).eax)
}

/// No arguments (`magic_0673` 0x10001f10, 134 bytes; 2 copies): the
/// record `R` = `*G + E` (`G` at `+0x03`, `E` at `+0x09`) reset: `+0x0`,
/// `+0x4`, `+0x8`, `+0xc`, `+0x14`, `+0x10` their immediates (`+0x16`,
/// `+0x1d`, `+0x24`, `+0x2b`, `+0x32`, `+0x39`), the halfword `+0x18` 0;
/// a pool node taken ([`take`], `+0x42`, the owner the operand at
/// `+0x10`), its `+0x14` `R`, byte `+0x18` 1, `+0x4` 0, appended to the
/// list ([`append_node`]: head `+0x48`/`+0x79`, tail `+0x60`, `+0x6a`,
/// `+0x7e`). Returns the node.
pub fn record_at_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = g.read::<u32>(w(0x03)).wrapping_add(w(0x09));
    for (d, imm) in [
        (0, 0x16),
        (4, 0x1d),
        (8, 0x24),
        (0xc, 0x2b),
        (0x14, 0x32),
        (0x10, 0x39),
    ] {
        g.write::<u32>(r.wrapping_add(d), w(imm));
    }
    g.write::<u16>(r.wrapping_add(0x18), 0);
    let node = take(g, img, img.rel32_target(f + 0x42), w(0x10));
    let empty = g.read::<u32>(w(0x48)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), r);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    append_node(g, node, empty, (w(0x60), w(0x6a)), (w(0x79), w(0x7e)));
    Ret::eax(node)
}

/// `(obj)` (`magic_0312` 0x100015b0, 142 bytes; 2 copies):
/// `rcefObjAttachChrOpt(obj, V, A)` (`+0x1c`, `V` the word at `*G + D`,
/// `G` at `+0x04`, `D` at `+0x10`, `A` the `push imm8` at `+0x0d`), four
/// words of `obj` set (`mov dword [esi + disp32], imm32` from `+0x32`, 10
/// bytes each), `rcefMainSetPar(P, obj, 0, 1)` (`+0x5c`, `P` = `*G` read
/// at `+0x2e` before the stores), `rcefTaskSetSeq(obj, cb)` (`+0x6e`,
/// `+0x68`) and `rcefTaskCallSeq(obj, cb2)` (`+0x81`, `+0x7b`), each with
/// the earlier words after its own. Returns the last slot's `eax`.
pub fn attach_par_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let obj = fr.arg(g, 0);
    let a = disp8(img, f + 0x0d);
    let v = g.read::<u32>(p.wrapping_add(w(0x10)));
    let s = slot_target(g, w(0x15), w(0x1c) / 4);
    g.call(s, fr.ecx, &[obj, v, a]);
    let p2 = g.read::<u32>(w(0x2e));
    for k in 0..4 {
        let insn = f + 0x32 + 10 * k;
        g.write::<u32>(obj.wrapping_add(img.word(insn + 2)), img.word(insn + 6));
    }
    let s = slot_target(g, w(0x23), w(0x5c) / 4);
    g.call(s, fr.ecx, &[p2, obj, 0, 1, obj, v, a]);
    let s = slot_target(g, w(0x63), w(0x6e) / 4);
    g.call(s, fr.ecx, &[obj, w(0x68), p2, obj, 0, 1, obj, v, a]);
    let s = slot_target(g, w(0x76), w(0x81) / 4);
    let args = [obj, w(0x7b), obj, w(0x68), p2, obj, 0, 1, obj, v, a];
    Ret::eax(g.call(s, fr.ecx, &args).eax)
}

/// No arguments (`magic_0511` 0x10002f70, 142 bytes; 2 copies, a
/// security-cookie frame): on `P` = `*G` (`+0x12`): `P[0xf44]` = 0
/// (`+0x1f`/`+0x23`), four words copied within `P` (from the fields at
/// `+0x29`, `+0x35`, `+0x41`, `+0x4d` to `+0x2f`, `+0x3b`, `+0x47`,
/// `+0x53`); slot 332 (`+0x69`) with `(&r)`, `r` a local rectangle at
/// `ebp - 0x14` (its four words the immediates at `+0x19`, `+0x63`,
/// `+0x70`, `+0x77`). Returns the slot's `eax`.
pub fn copy_then_rect<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x12));
    // ebp = esp - 4; r at ebp - 0x14
    let r = fr.esp.wrapping_sub(0x18);
    g.write::<u32>(r, w(0x19));
    g.write::<u32>(p.wrapping_add(w(0x1f)), w(0x23));
    for (src, dst) in [(0x29, 0x2f), (0x35, 0x3b), (0x41, 0x47), (0x4d, 0x53)] {
        let v = g.read::<u32>(p.wrapping_add(w(src)));
        g.write::<u32>(p.wrapping_add(w(dst)), v);
    }
    for (k, imm) in (1u32..).zip([0x63, 0x70, 0x77]) {
        g.write::<u32>(r + 4 * k, w(imm));
    }
    let s = slot_target(g, w(0x5c), w(0x69) / 4);
    Ret::eax(g.call(s, fr.ecx, &[r]).eax)
}

/// No arguments (`magic_0267` 0x10002f80, 156 bytes; 2 copies): a packet
/// built in the DLL's buffer: three 8-byte zeros (`movlpd`, at `+0x0d`,
/// `+0x15`, `+0x27`), the header word at `+0x1b` (`+0x1f`), then words
/// from the immediates and from `P` = `*G` (`+0x02`; its fields the
/// `disp8` at `+0x41`, `+0x49`, `+0x65`, `+0x6d`) in the order the
/// original stores them; then slot 382 (`+0x8e`) with `(0, 0)`. Returns
/// its `eax`.
pub fn packet<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x02));
    for at in [0x0d, 0x15] {
        g.write::<u64>(w(at), 0);
    }
    g.write::<u32>(w(0x1b), w(0x1f));
    g.write::<u64>(w(0x27), 0);
    g.write::<u32>(w(0x2d), w(0x31));
    g.write::<u32>(w(0x37), w(0x3b));
    let field = |g: &G, at: u32| g.read::<u32>(p.wrapping_add(disp8(img, f + at)));
    let v = field(g, 0x41);
    g.write::<u32>(w(0x43), v);
    let v = field(g, 0x49);
    g.write::<u32>(w(0x4b), v);
    g.write::<u32>(w(0x51), w(0x55));
    g.write::<u32>(w(0x5b), w(0x5f));
    let v = field(g, 0x65);
    g.write::<u32>(w(0x67), v);
    let v = field(g, 0x6d);
    g.write::<u32>(w(0x6f), v);
    g.write::<u32>(w(0x7a), w(0x7e));
    g.write::<u32>(w(0x84), w(0x88));
    let s = slot_target(g, w(0x74), w(0x8e) / 4);
    Ret::eax(g.call(s, fr.ecx, &[0, 0]).eax)
}

/// No arguments (`magic_0473` 0x10002010, 165 bytes; 2 copies): the
/// record `R` = `*G + E` (`G` at `+0x03`, `E` at `+0x09`) set with stores
/// of three widths (each `(disp32, immediate, size)` read from the code),
/// a pool node taken ([`take`], `+0x65`, the owner the operand at
/// `+0x0e`), its `+0x14` `R`, `+0x4` 0, appended to the list
/// ([`append_node`]: head `+0x6b`/`+0x98`, tail `+0x7f`, `+0x89`,
/// `+0x9d`). Returns the node.
pub fn mixed_record_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = g.read::<u32>(w(0x03)).wrapping_add(w(0x09));
    // (disp32 offset, immediate offset, bytes)
    let stores: [(u32, u32, u8); 9] = [
        (0x14, 0x18, 4),
        (0x1e, 0x22, 1),
        (0x26, 0x2a, 2),
        (0x2e, 0x32, 1),
        (0x35, 0x39, 4),
        (0x40, 0x44, 2),
        (0x48, 0x4c, 4),
        (0x52, 0x56, 4),
        (0x5c, 0x60, 4),
    ];
    for (d, imm, size) in stores {
        let at = r.wrapping_add(w(d));
        match size {
            1 => g.write::<u8>(at, w(imm) as u8),
            2 => g.write::<u16>(at, w(imm) as u16),
            _ => g.write::<u32>(at, w(imm)),
        }
    }
    let node = take(g, img, img.rel32_target(f + 0x65), w(0x0e));
    let empty = g.read::<u32>(w(0x6b)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), r);
    g.write::<u32>(node.wrapping_add(4), 0);
    append_node(g, node, empty, (w(0x7f), w(0x89)), (w(0x98), w(0x9d)));
    Ret::eax(node)
}

/// `(obj)` (`magic_0313` 0x10001240, 167 bytes; 2 copies): slot 447
/// (`+0x1c`) with `(V, K)` (`V` the word at `*G + D`, `G` at `+0x04`, `D`
/// at `+0x0c`, `K` the `push imm32` at `+0x16`), three
/// `rcefMainCreateTask(*G, cb)` (`+0x35`, `+0x4d`, `+0x65`; `G` at
/// `+0x2f`, `+0x47`, `+0x5f`; `cb` at `+0x29`, `+0x41`, `+0x59`),
/// `rcefTaskSetSeq(obj, cb4)` (`+0x75`, `+0x7a`) and
/// `rcefTaskCallSeq(obj, cb5)` (`+0x8d`, `+0x87`), each with the earlier
/// words after its own; then `V` in the object's list `obj + 0x2c + 16 *
/// i` (`i` the word at `obj + 0xac`). Returns `2 * i`.
pub fn three_tasks_keep<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let mut stack = vec![v, w(0x16)];
    let s = slot_target(g, w(0x11), w(0x1c) / 4);
    g.call(s, fr.ecx, &stack);
    for (gl, cb, table, disp) in [
        (0x2f, 0x29, 0x24, 0x35),
        (0x47, 0x41, 0x3c, 0x4d),
        (0x5f, 0x59, 0x54, 0x65),
    ] {
        let a = g.read::<u32>(w(gl));
        stack.splice(0..0, [a, w(cb)]);
        let s = slot_target(g, w(table), w(disp) / 4);
        // call width: 8 at most (the earlier calls' words stay)
        g.call(s, fr.ecx, &stack);
    }
    let obj = fr.arg(g, 0);
    for (cb, table, disp) in [(0x7a, 0x6c, 0x75), (0x87, 0x82, 0x8d)] {
        stack.splice(0..0, [obj, w(cb)]);
        let s = slot_target(g, w(table), w(disp) / 4);
        // call width: 12 at most
        g.call(s, fr.ecx, &stack);
    }
    // add eax, eax; mov [esi + eax * 8 + 0x2c], edi
    let i = g.read::<u32>(obj.wrapping_add(0xac));
    let twice = i.wrapping_add(i);
    g.write::<u32>(
        obj.wrapping_add(twice.wrapping_mul(8)).wrapping_add(0x2c),
        v,
    );
    Ret::eax(twice)
}

/// `(obj)` (`magic_0194` 0x10001620, 173 bytes; 2 copies): slot 444
/// (`+0x0b`) with `(K)` (`+0x10`); `rcefObjAttachChrOpt(obj, V, A)`
/// (`+0x2b`, `V` the word at `*G + D`, `G` at `+0x14`, `D` at `+0x1f`, `A`
/// at `+0x1c`); `rcefObjSetMode(obj, M)` (`+0x3a`, `M` at `+0x37`); four
/// words of `obj` set (from `+0x51`, 10 bytes each); `rcefMainSetPar(P,
/// obj, 0, 0)` (`+0x7b`, `P` = `*G` read at `+0x4d` before the stores);
/// `rcefTaskSetSeq(obj, cb)` (`+0x8d`, `+0x87`) and
/// `rcefTaskCallSeq(obj, cb2)` (`+0xa0`, `+0x9a`), each with the earlier
/// words after its own. Returns the last slot's `eax`.
pub fn slot_attach_mode_par<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut stack = vec![disp8(img, f + 0x10)];
    let s = slot_target(g, w(0x04), w(0x0b) / 4);
    g.call(s, fr.ecx, &stack);
    let p = g.read::<u32>(w(0x14));
    let obj = fr.arg(g, 0);
    let v = g.read::<u32>(p.wrapping_add(w(0x1f)));
    stack.splice(0..0, [obj, v, disp8(img, f + 0x1c)]);
    let s = slot_target(g, w(0x24), w(0x2b) / 4);
    // call width: 4
    g.call(s, fr.ecx, &stack);
    stack.splice(0..0, [obj, disp8(img, f + 0x37)]);
    let s = slot_target(g, w(0x32), w(0x3a) / 4);
    // call width: 6
    g.call(s, fr.ecx, &stack);
    let p2 = g.read::<u32>(w(0x4d));
    for k in 0..4 {
        let insn = f + 0x51 + 10 * k;
        g.write::<u32>(obj.wrapping_add(img.word(insn + 2)), img.word(insn + 6));
    }
    stack.splice(0..0, [p2, obj, 0, 0]);
    let s = slot_target(g, w(0x42), w(0x7b) / 4);
    // call width: 10
    g.call(s, fr.ecx, &stack);
    stack.splice(0..0, [obj, w(0x87)]);
    let s = slot_target(g, w(0x82), w(0x8d) / 4);
    // call width: 12
    g.call(s, fr.ecx, &stack);
    stack.splice(0..0, [obj, w(0x9a)]);
    let s = slot_target(g, w(0x95), w(0xa0) / 4);
    // call width: 14
    Ret::eax(g.call(s, fr.ecx, &stack).eax)
}

/// [`effect_record`] in `magic_0307` (0x100024c0, 132 bytes).
pub const EFFECT_RECORD_0307_24C0: Sig = Sig {
    len: 132,
    hash: 0xf395_29cd_afa2_70aa,
    calls: &[],
    imports: &[],
};

/// [`effect_record`] in `magic_0605` (0x10007e40, 132 bytes).
pub const EFFECT_RECORD_0605_7E40: Sig = Sig {
    len: 132,
    hash: 0xe25f_446e_a8c2_a5e3,
    calls: &[],
    imports: &[],
};

/// [`mesh_flags`] in `magic_0391` (0x10003e40, 132 bytes).
pub const MESH_FLAGS_0391_3E40: Sig = Sig {
    len: 132,
    hash: 0x25d0_de71_e376_807c,
    calls: &[],
    imports: &[],
};

/// [`mesh_flags`] in `magic_0392` (0x10004070, 132 bytes).
pub const MESH_FLAGS_0392_4070: Sig = Sig {
    len: 132,
    hash: 0x48a7_f6c8_21a8_4741,
    calls: &[],
    imports: &[],
};

/// [`deal_then_slots`] in `magic_0391` (0x10002a20, 134 bytes).
pub const DEAL_THEN_SLOTS_0391_2A20: Sig = Sig {
    len: 134,
    hash: 0x899d_1349_681e_b336,
    calls: &[(0xe, Callee::Sig(&crate::sb_misc::DEAL))],
    imports: &[],
};

/// [`deal_then_slots`] in `magic_0448` (0x10006440, 134 bytes).
pub const DEAL_THEN_SLOTS_0448_6440: Sig = Sig {
    len: 134,
    hash: 0xce2f_d0e4_108a_0be2,
    calls: &[(0xe, Callee::Sig(&crate::sb_misc::DEAL))],
    imports: &[],
};

/// [`record_at_node`] in `magic_0673` (0x10001f10, 134 bytes).
pub const RECORD_AT_NODE_0673_1F10: Sig = Sig {
    len: 134,
    hash: 0x434e_3084_9274_5646,
    calls: &[(0x42, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`record_at_node`] in `magic_0673` (0x10001fa0, 134 bytes).
pub const RECORD_AT_NODE_0673_1FA0: Sig = Sig {
    len: 134,
    hash: 0xf00c_b327_b54b_2b20,
    calls: &[(0x42, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`attach_par_seq`] in `magic_0312` (0x100015b0, 142 bytes).
pub const ATTACH_PAR_SEQ_0312_15B0: Sig = Sig {
    len: 142,
    hash: 0xedfa_390d_3209_4fc6,
    calls: &[],
    imports: &[],
};

/// [`attach_par_seq`] in `magic_0378` (0x10001250, 142 bytes).
pub const ATTACH_PAR_SEQ_0378_1250: Sig = Sig {
    len: 142,
    hash: 0xf76f_ea7f_742c_2a24,
    calls: &[],
    imports: &[],
};

/// [`copy_then_rect`] in `magic_0511` (0x10002f70, 142 bytes).
pub const COPY_THEN_RECT_0511_2F70: Sig = Sig {
    len: 142,
    hash: 0xf39c_da38_ef58_648b,
    calls: &[(0x86, Callee::Any)],
    imports: &[],
};

/// [`copy_then_rect`] in `magic_0589` (0x10002b50, 142 bytes).
pub const COPY_THEN_RECT_0589_2B50: Sig = Sig {
    len: 142,
    hash: 0x375b_ac82_f65d_7e4b,
    calls: &[(0x86, Callee::Any)],
    imports: &[],
};

/// [`packet`] in `magic_0267` (0x10002f80, 156 bytes).
pub const PACKET_0267_2F80: Sig = Sig {
    len: 156,
    hash: 0xf007_e711_55fd_e674,
    calls: &[],
    imports: &[],
};

/// [`packet`] in `magic_0431` (0x10001430, 156 bytes).
pub const PACKET_0431_1430: Sig = Sig {
    len: 156,
    hash: 0x9ed5_4df4_8457_b014,
    calls: &[],
    imports: &[],
};

/// [`mixed_record_node`] in `magic_0473` (0x10002010, 165 bytes).
pub const MIXED_RECORD_NODE_0473_2010: Sig = Sig {
    len: 165,
    hash: 0x7788_63fe_fa69_9eb5,
    calls: &[(0x65, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`mixed_record_node`] in `magic_0673` (0x10001850, 165 bytes).
pub const MIXED_RECORD_NODE_0673_1850: Sig = Sig {
    len: 165,
    hash: 0xbc2e_b238_c365_378b,
    calls: &[(0x65, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`three_tasks_keep`] in `magic_0313` (0x10001240, 167 bytes).
pub const THREE_TASKS_KEEP_0313_1240: Sig = Sig {
    len: 167,
    hash: 0x7d35_0082_8479_a7f5,
    calls: &[],
    imports: &[],
};

/// [`three_tasks_keep`] in `magic_0361` (0x10001330, 167 bytes).
pub const THREE_TASKS_KEEP_0361_1330: Sig = Sig {
    len: 167,
    hash: 0xbb22_39ef_8a79_e8e8,
    calls: &[],
    imports: &[],
};

/// [`slot_attach_mode_par`] in `magic_0194` (0x10001620, 173 bytes).
pub const SLOT_ATTACH_MODE_PAR_0194_1620: Sig = Sig {
    len: 173,
    hash: 0x5c4e_2ea7_6a91_fd24,
    calls: &[],
    imports: &[],
};

/// [`slot_attach_mode_par`] in `magic_0359` (0x10001420, 173 bytes).
pub const SLOT_ATTACH_MODE_PAR_0359_1420: Sig = Sig {
    len: 173,
    hash: 0x935d_e84a_7bb3_abab,
    calls: &[],
    imports: &[],
};
