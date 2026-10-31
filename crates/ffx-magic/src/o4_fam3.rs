//! Families of two copies from the own work list (round 4, third pass):
//! part starts with vector copies, static records on the task list,
//! store blocks and slot lists. Operands are read from each copy's code
//! (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::o4_small::append_node;
use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::sa_pool::{give_back, take};
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The slot of a `mov eax, [eax + disp32]` with the `disp32` at `at`.
fn slot_at(img: &Image, at: u32) -> u32 {
    img.word(at) / 4
}

/// No arguments (`magic_0370` 0x10001550, 107 bytes; 2 copies): `*G_P =
/// ITEM` (`+0x01`, `+0x07`), the part `list[d8 / 4]` (slot 941 of `*G_D +
/// 0x10`, `G_D` at `+0x0c`, the table at `+0x14`, the entry at `+0x26`)
/// started on the static object at `+0x2d` (`pppStartPart(obj, 0x1000,
/// program, 1)`, the wrapper's `rel32` at `+0x32`), then three 16-byte
/// constants copied (from `+0x39`, `+0x48`, `+0x57` to `+0x40`, `+0x50`,
/// `+0x61`). Returns 1.
pub fn part_vectors<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
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
    for (src, dst) in [(0x39, 0x40), (0x48, 0x50), (0x57, 0x61)] {
        for k in 0..4 {
            let v = g.read::<u32>(w(src) + 4 * k);
            g.write::<u32>(w(dst) + 4 * k, v);
        }
    }
    Ret::eax(1)
}

/// No arguments (`magic_0267` 0x100037f0, 103 bytes; 2 copies): three
/// words of a static record set (`mov dword [addr], imm32` from `+0x05`,
/// 10 bytes each), a pool node taken ([`take`], `+0x24`, the owner the
/// operand at `+0x01`), its `+0x14` the record (`+0x32`), `+0x4` 0,
/// appended to the list ([`append_node`]: head `+0x2a`/`+0x5a`, tail
/// `+0x41`, `+0x4b`, `+0x5f`). Returns the node.
pub fn static_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    for k in 0..3 {
        let insn = f + 0x05 + 10 * k;
        g.write::<u32>(img.word(insn + 2), img.word(insn + 6));
    }
    let node = take(g, img, img.rel32_target(f + 0x24), w(0x01));
    let empty = g.read::<u32>(w(0x2a)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), w(0x32));
    g.write::<u32>(node.wrapping_add(4), 0);
    append_node(g, node, empty, (w(0x41), w(0x4b)), (w(0x5a), w(0x5f)));
    Ret::eax(node)
}

/// No arguments (`magic_0307` 0x10002a30, 110 bytes; 2 copies): on `P` =
/// `*G` (`+0x01`): `x = P[A] - K` (`A` at `+0x09`, `K` at `+0x11`), eight
/// words set (`mov dword [eax + disp32], imm32` from `+0x15`, 10 bytes
/// each), then `P[B] = x` (`+0x69`). Returns `P`.
pub fn sub_stores8<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    let a = f32::from_bits(g.read::<u32>(p.wrapping_add(w(0x09))));
    let x = sub(a, f32::from_bits(g.read::<u32>(w(0x11))));
    for k in 0..8 {
        let insn = f + 0x15 + 10 * k;
        g.write::<u32>(p.wrapping_add(img.word(insn + 2)), img.word(insn + 6));
    }
    g.write::<u32>(p.wrapping_add(w(0x69)), x.to_bits());
    Ret::eax(p)
}

/// No arguments (`magic_0519` 0x10002a50, 110 bytes; 2 copies, a
/// security-cookie frame): `n` = slot 335 (`+0x23`) with `(&list)`, `list`
/// 256 words at `ebp - 0x404`; for each `list[i]`, `i < n`, slot 257
/// (`+0x50`) with `(list[i], K)` (`K` the `push imm8` at `+0x46`). Returns
/// the last call's `eax` (`n` when none ran). The cookie word the original
/// keeps at `ebp - 4` is its own (the CRT's check).
pub fn slot_list<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // ebp = esp - 4; the list at ebp - 0x404
    let list = fr.esp.wrapping_sub(0x408);
    let s = slot_target(g, w(0x1d), slot_at(img, f + 0x23));
    let n = g.call(s, fr.ecx, &[list]).eax;
    let mut eax = n;
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let v = g.read::<u32>(list.wrapping_add(4 * i));
        let s = slot_target(g, w(0x41), slot_at(img, f + 0x50));
        eax = g.call(s, fr.ecx, &[v, disp8(img, f + 0x46)]).eax;
        i += 1;
    }
    Ret::eax(eax)
}

/// `(obj)` (`magic_0194` 0x100010f0, 125 bytes; 2 copies): slot 444
/// (`+0x0c`) with `(0)`, the slot at `+0x1b` with `(K)` (`K` the `push
/// imm8` at `+0x18`), two `rcefMainCreateTask(*G, cb)` (`+0x33`/`+0x4b`,
/// `G` at `+0x2d`/`+0x45`, `cb` at `+0x27`/`+0x3f`), `rcefTaskSetSeq(obj,
/// cb3)` (`+0x60`, `+0x57`) and `rcefTaskWait(obj, T)` (`+0x72`, `T` at
/// `+0x6c`), each with the earlier words after its own. Returns the last
/// slot's `eax`.
pub fn two_tasks_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let zero = disp8(img, f + 0x09);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0c));
    g.call(s, fr.ecx, &[zero]);
    let k = disp8(img, f + 0x18);
    let s = slot_target(g, w(0x13), slot_at(img, f + 0x1b));
    g.call(s, fr.ecx, &[k, zero]);
    let a = g.read::<u32>(w(0x2d));
    let s = slot_target(g, w(0x22), slot_at(img, f + 0x33));
    g.call(s, fr.ecx, &[a, w(0x27), k, zero]);
    let b = g.read::<u32>(w(0x45));
    let s = slot_target(g, w(0x3a), slot_at(img, f + 0x4b));
    g.call(s, fr.ecx, &[b, w(0x3f), a, w(0x27), k, zero]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x52), slot_at(img, f + 0x60));
    let below = [b, w(0x3f), a, w(0x27), k, zero];
    let mut args = vec![obj, w(0x57)];
    args.extend(below);
    // call width: 8
    g.call(s, fr.ecx, &args);
    let s = slot_target(g, w(0x67), slot_at(img, f + 0x72));
    let mut args = vec![obj, disp8(img, f + 0x6c), obj, w(0x57)];
    args.extend(below);
    // call width: 10 (the earlier calls' words stay on the stack)
    Ret::eax(g.call(s, fr.ecx, &args).eax)
}

/// No arguments (`magic_0165` 0x10004750, 127 bytes; 2 copies, a
/// security-cookie frame): slot 327 (`+0x27`) with `(&m, R)` (`m` a
/// local matrix at `ebp - 0x54`, `R` at `+0x1b`); slot 229 (`+0x43`) with
/// `(&v, &m, P + D)` (`v` a local vector at `ebp - 0x14`, `P` = `*G` at
/// `+0x13`, `D` the `lea` displacement at `+0x2f`); then `X = v.x`
/// (`+0x58`), `Y` = the immediate at `+0x69` (`+0x65`), `Z = v.z`
/// (`+0x71`). Returns slot 229's `eax`.
pub fn vector_globals<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // ebp = esp - 4
    let (m, v) = (fr.esp.wrapping_sub(0x58), fr.esp.wrapping_sub(0x18));
    let p = g.read::<u32>(w(0x13));
    let s = slot_target(g, w(0x21), slot_at(img, f + 0x27));
    g.call(s, fr.ecx, &[m, w(0x1b)]);
    let s = slot_target(g, w(0x3d), slot_at(img, f + 0x43));
    let r = g
        .call(s, fr.ecx, &[v, m, p.wrapping_add(w(0x2f)), m, w(0x1b)])
        .eax;
    let x = g.read::<u32>(v);
    g.write::<u32>(w(0x58), x);
    let z = g.read::<u32>(v + 8);
    g.write::<u32>(w(0x65), w(0x69));
    g.write::<u32>(w(0x71), z);
    Ret::eax(r)
}

/// No arguments (`magic_0162` 0x10003900, 131 bytes; 2 copies): the pool
/// node in `*G + D` (`G` at `+0x03`, `D` at `+0x09`/`+0x18`), if any,
/// given back ([`give_back`], `+0x12`) and the field cleared; then slots
/// 357 and 356 (`+0x2d`, `+0x41`) each with `(M, -1, -1)` (`M` = `*H`,
/// `H` at `+0x27`) and again with `(M2, -1, -1)` (`+0x66`, `+0x75`; `M2`
/// = `TABLE[B[1]]`, `B` = `*L` at `+0x4b`, `TABLE` at `+0x5a`), each with
/// the earlier words after its own. Returns the last slot's `eax`.
pub fn release_and_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let base = g.read::<u32>(w(0x03));
    let node = g.read::<u32>(base.wrapping_add(w(0x09)));
    if node != 0 {
        give_back(g, img, img.rel32_target(f + 0x12), node);
        g.write::<u32>(base.wrapping_add(w(0x18)), 0);
    }
    let m = g.read::<u32>(w(0x27));
    let one = [m, 0xffff_ffff, 0xffff_ffff];
    let s = slot_target(g, w(0x21), slot_at(img, f + 0x2d));
    g.call(s, fr.ecx, &one);
    let s = slot_target(g, w(0x39), slot_at(img, f + 0x41));
    // call width: 6
    g.call(s, fr.ecx, &[one, one].concat());
    let b = g.read::<u32>(w(0x4b));
    let i = u32::from(g.read::<u8>(b.wrapping_add(1)));
    let m2 = g.read::<u32>(w(0x5a).wrapping_add(4 * i));
    let two = [m2, 0xffff_ffff, 0xffff_ffff];
    let s = slot_target(g, w(0x5f), slot_at(img, f + 0x66));
    // call width: 9
    g.call(s, fr.ecx, &[two, one, one].concat());
    let s = slot_target(g, w(0x6d), slot_at(img, f + 0x75));
    // call width: 12 (the earlier calls' words stay on the stack)
    Ret::eax(g.call(s, fr.ecx, &[two, two, one, one].concat()).eax)
}

/// A record set (`magic_0480` 0x10002880, 136 bytes, internal: `edx` =
/// the index, cdecl `(src)`; 2 copies): in record `R = BASE + 0x38 * i`
/// (the operands are `BASE`-relative addresses, `+0x13`..): four words
/// copied within it (`+0x13` to `+0x1a` and on, 14 bytes a pair), the
/// four words of `src` stored (`+0x4d`, `+0x57`, `+0x61`, `+0x6b`), two
/// words their immediates (`+0x72`/`+0x76`, `+0x7d`/`+0x81`). Returns
/// `src[3]`.
pub fn record_copy<G: Game>(g: &mut G, img: &Image, f: u32, i: u32, src: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let r = i.wrapping_mul(0x38);
    for k in 0..4 {
        let v = g.read::<u32>(w(0x13 + 14 * k).wrapping_add(r));
        g.write::<u32>(w(0x1a + 14 * k).wrapping_add(r), v);
    }
    let mut last = 0;
    for (k, at) in (0u32..).zip([0x4d, 0x57, 0x61, 0x6b]) {
        last = g.read::<u32>(src.wrapping_add(4 * k));
        g.write::<u32>(w(at).wrapping_add(r), last);
    }
    g.write::<u32>(w(0x72).wrapping_add(r), w(0x76));
    g.write::<u32>(w(0x7d).wrapping_add(r), w(0x81));
    last
}

/// [`part_vectors`] in `magic_0370` (0x10001550, 107 bytes).
pub const PART_VECTORS_0370_1550: Sig = Sig {
    len: 107,
    hash: 0x71b2_fd97_b302_7e66,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`part_vectors`] in `magic_0370` (0x100018c0, 107 bytes).
pub const PART_VECTORS_0370_18C0: Sig = Sig {
    len: 107,
    hash: 0x9fba_bd0d_25ac_08ea,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`static_node`] in `magic_0267` (0x100037f0, 103 bytes).
pub const STATIC_NODE_0267_37F0: Sig = Sig {
    len: 103,
    hash: 0x01ee_63fc_4b93_c819,
    calls: &[(0x24, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`static_node`] in `magic_0311` (0x100024c0, 103 bytes).
pub const STATIC_NODE_0311_24C0: Sig = Sig {
    len: 103,
    hash: 0x43d0_73d3_2f01_5c75,
    calls: &[(0x24, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`sub_stores8`] in `magic_0307` (0x10002a30, 110 bytes).
pub const SUB_STORES8_0307_2A30: Sig = Sig {
    len: 110,
    hash: 0xbf8d_2ee4_8d62_7c24,
    calls: &[],
    imports: &[],
};

/// [`sub_stores8`] in `magic_0563` (0x10002530, 110 bytes).
pub const SUB_STORES8_0563_2530: Sig = Sig {
    len: 110,
    hash: 0xde29_dfca_7813_4624,
    calls: &[],
    imports: &[],
};

/// [`slot_list`] in `magic_0519` (0x10002a50, 110 bytes).
pub const SLOT_LIST_0519_2A50: Sig = Sig {
    len: 110,
    hash: 0x8b4f_42b9_297c_0267,
    calls: &[(0x66, Callee::Any)],
    imports: &[],
};

/// [`slot_list`] in `magic_0519` (0x10002ac0, 110 bytes).
pub const SLOT_LIST_0519_2AC0: Sig = Sig {
    len: 110,
    hash: 0xf219_baa3_d6ea_ca72,
    calls: &[(0x66, Callee::Any)],
    imports: &[],
};

/// [`two_tasks_seq_wait`] in `magic_0194` (0x100010f0, 125 bytes).
pub const TWO_TASKS_SEQ_WAIT_0194_10F0: Sig = Sig {
    len: 125,
    hash: 0xe4d7_fc9f_22a2_15d1,
    calls: &[],
    imports: &[],
};

/// [`two_tasks_seq_wait`] in `magic_0300` (0x100010a0, 125 bytes).
pub const TWO_TASKS_SEQ_WAIT_0300_10A0: Sig = Sig {
    len: 125,
    hash: 0x1bc8_9f14_2e32_e27a,
    calls: &[],
    imports: &[],
};

/// [`vector_globals`] in `magic_0165` (0x10004750, 127 bytes).
pub const VECTOR_GLOBALS_0165_4750: Sig = Sig {
    len: 127,
    hash: 0x544a_8546_5063_6dd4,
    calls: &[(0x77, Callee::Any)],
    imports: &[],
};

/// [`vector_globals`] in `magic_0563` (0x10005670, 127 bytes).
pub const VECTOR_GLOBALS_0563_5670: Sig = Sig {
    len: 127,
    hash: 0xb3fa_c159_a9c3_d96d,
    calls: &[(0x77, Callee::Any)],
    imports: &[],
};

/// [`release_and_reset`] in `magic_0162` (0x10003900, 131 bytes).
pub const RELEASE_AND_RESET_0162_3900: Sig = Sig {
    len: 131,
    hash: 0xdd80_7bcd_9d0f_2333,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`release_and_reset`] in `magic_0586` (0x10003510, 131 bytes).
pub const RELEASE_AND_RESET_0586_3510: Sig = Sig {
    len: 131,
    hash: 0xd08a_f22d_8e51_1ab3,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`record_copy`] in `magic_0480` (0x10002880, 136 bytes).
pub const RECORD_COPY_0480_2880: Sig = Sig {
    len: 136,
    hash: 0x37c0_c919_e542_32aa,
    calls: &[],
    imports: &[],
};

/// [`record_copy`] in `magic_0480` (0x10002910, 136 bytes).
pub const RECORD_COPY_0480_2910: Sig = Sig {
    len: 136,
    hash: 0xa46e_6256_48a1_bec2,
    calls: &[],
    imports: &[],
};
