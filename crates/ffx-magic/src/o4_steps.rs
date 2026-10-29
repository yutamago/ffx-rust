//! Task steps and small object functions the own work list repeats
//! (round 4): the same code in several DLLs, the fields and globals as
//! the instructions' operands, read from each copy's code.
//!
//! - `STEP_PLAIN_*`: more copies of [`crate::ob_summon::step_plain`] (6).
//! - [`step_then_register`]: the same step ending in the part
//!   registration (5 copies).
//! - [`release_node`]: a pool node kept in a field given back (7 copies).
//! - [`sub_const`]: a float field less a constant (5 copies).
//! - [`pool_node`]: a pool node taken, a record's five words set, the
//!   node appended to a list (5 copies).

use crate::l5_objhost as host;
use crate::runtime::slot_target;
use crate::sa_misc::register_parts;
use crate::sa_pool::{give_back, take};
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (`magic_0165` 0x10001760, no arguments, 55 bytes): runs
/// the objects ([`host::run_objects`]); once none ran and the state's
/// done word (`*G_STATE + D`, `D` at `+0x13`) is 1, ends the effect
/// (slot 214 `(G_HANDLE)`) and jumps to the part registration
/// ([`register_parts`], the `rel32` at `+0x31`) with slot 214's `eax`.
/// Returns `run_objects`' result otherwise.
pub fn step_then_register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let state = g.read::<u32>(w(0x03));
    // push esi; call
    let r = host::run_objects(g, img, img.rel32_target(f + 0x08), fr.esp - 8);
    // cmp [esi + D], eax
    if r != 1 || g.read::<u32>(state.wrapping_add(w(0x13))) != 1 {
        return Ret::eax(r);
    }
    let s214 = slot_target(g, w(0x1a), 214);
    let h = g.read::<u32>(w(0x20));
    let eax = g.call(s214, 0, &[h]).eax;
    // add esp, 4; pop esi; jmp
    Ret::eax(register_parts(g, img, img.rel32_target(f + 0x31), eax))
}

/// No arguments (`magic_0162` 0x10004120, 34 bytes): the pool node in
/// the field `*G + D` (`D` at `+0x09`, again at `+0x18`), if any, given
/// back ([`give_back`], the `rel32` at `+0x12`) and the field cleared.
/// Returns what `give_back` left in `eax`, or `eax` as given without a
/// node.
pub fn release_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let base = g.read::<u32>(w(0x03));
    let node = g.read::<u32>(base.wrapping_add(w(0x09)));
    if node == 0 {
        return Ret::eax(fr.eax);
    }
    let eax = give_back(g, img, img.rel32_target(f + 0x12), node);
    g.write::<u32>(base.wrapping_add(w(0x18)), 0);
    Ret::eax(eax)
}

/// No arguments (`magic_0168` 0x100033a0, 30 bytes): the float at `*G +
/// D` (`D` at `+0x09`) less the constant at operand `+0x11`, stored at
/// `*G + D` (`+0x19`). Returns `*G`.
pub fn sub_const<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    let x = f32::from_bits(g.read::<u32>(p.wrapping_add(w(0x09))));
    let k = f32::from_bits(g.read::<u32>(w(0x11)));
    g.write::<u32>(p.wrapping_add(w(0x19)), sub(x, k).to_bits());
    Ret::eax(p)
}

/// No arguments (`magic_0350` 0x10006280, 121 bytes): the record `*G`
/// (operand `+0x01`) gets five words (`mov dword [eax + d], imm32` from
/// `+0x0a`, 10 bytes each); a node taken from the pool ([`take`], the
/// `rel32` at `+0x3d`, the owner the operand at `+0x06`), its `+0x4`
/// cleared, appended to the list `G_HEAD`/`G_TAIL` (a first node starts
/// it: `prev` 0, head and tail the node; else after the tail), its `+0x8`
/// itself. Returns the node.
pub fn pool_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = g.read::<u32>(w(0x01));
    for k in 0..5 {
        // c7 80 disp32 imm32
        let insn = f + 0x0a + 10 * k;
        g.write::<u32>(rec.wrapping_add(img.word(insn + 2)), img.word(insn + 6));
    }
    let node = take(g, img, img.rel32_target(f + 0x3d), w(0x06));
    let empty = g.read::<u32>(w(0x43)) == 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x6c), node);
        g.write::<u32>(w(0x71), node);
    } else {
        let tail = g.read::<u32>(w(0x53));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(w(0x5d), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`step_plain`] in `magic_0162` (0x10002190, 49 bytes).
pub const STEP_PLAIN_0162_2190: Sig = Sig {
    len: 49,
    hash: 0x6141_f131_6f52_6b3f,
    calls: &[(0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0348` (0x100065c0, 49 bytes).
pub const STEP_PLAIN_0348_65C0: Sig = Sig {
    len: 49,
    hash: 0x1fde_4fff_bfa9_09bf,
    calls: &[(0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0350` (0x10005a10, 49 bytes).
pub const STEP_PLAIN_0350_5A10: Sig = Sig {
    len: 49,
    hash: 0xd53c_a9fe_8b7b_5156,
    calls: &[(0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0352` (0x10004c90, 49 bytes).
pub const STEP_PLAIN_0352_4C90: Sig = Sig {
    len: 49,
    hash: 0x57a3_05b1_1aae_a0a5,
    calls: &[(0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0353` (0x10001940, 49 bytes).
pub const STEP_PLAIN_0353_1940: Sig = Sig {
    len: 49,
    hash: 0xf5d1_fcd5_7f86_e821,
    calls: &[(0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0392` (0x10001460, 49 bytes).
pub const STEP_PLAIN_0392_1460: Sig = Sig {
    len: 49,
    hash: 0x711d_7a72_04b6_f465,
    calls: &[(0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_then_register`] in `magic_0165` (0x10001760, 55 bytes).
pub const STEP_THEN_REGISTER_0165_1760: Sig = Sig {
    len: 55,
    hash: 0xe3bb_ed66_b85a_5564,
    calls: &[
        (0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x31, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`step_then_register`] in `magic_0307` (0x100015a0, 55 bytes).
pub const STEP_THEN_REGISTER_0307_15A0: Sig = Sig {
    len: 55,
    hash: 0xe814_5e35_b022_189d,
    calls: &[
        (0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x31, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`step_then_register`] in `magic_0605` (0x10006ef0, 55 bytes).
pub const STEP_THEN_REGISTER_0605_6EF0: Sig = Sig {
    len: 55,
    hash: 0xe576_e56f_ae4b_1d65,
    calls: &[
        (0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x31, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`step_then_register`] in `magic_0612` (0x10007390, 55 bytes).
pub const STEP_THEN_REGISTER_0612_7390: Sig = Sig {
    len: 55,
    hash: 0x2197_ffc7_a576_74a0,
    calls: &[
        (0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x31, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`step_then_register`] in `magic_0688` (0x10003a90, 55 bytes).
pub const STEP_THEN_REGISTER_0688_3A90: Sig = Sig {
    len: 55,
    hash: 0xaa7d_4177_c4e2_b08e,
    calls: &[
        (0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x31, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`release_node`] in `magic_0162` (0x10004120, 34 bytes).
pub const RELEASE_NODE_0162_4120: Sig = Sig {
    len: 34,
    hash: 0x1ca1_20b3_0d4f_4a31,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`release_node`] in `magic_0165` (0x10003050, 34 bytes).
pub const RELEASE_NODE_0165_3050: Sig = Sig {
    len: 34,
    hash: 0x9345_61ff_729b_9273,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`release_node`] in `magic_0430` (0x10003190, 34 bytes).
pub const RELEASE_NODE_0430_3190: Sig = Sig {
    len: 34,
    hash: 0x4700_8c12_2923_82ab,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`release_node`] in `magic_0431` (0x10004360, 34 bytes).
pub const RELEASE_NODE_0431_4360: Sig = Sig {
    len: 34,
    hash: 0x3ef7_f8bb_afcf_cf7b,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`release_node`] in `magic_0586` (0x10003d30, 34 bytes).
pub const RELEASE_NODE_0586_3D30: Sig = Sig {
    len: 34,
    hash: 0x4186_5764_9e85_9671,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`release_node`] in `magic_0587` (0x10003af0, 34 bytes).
pub const RELEASE_NODE_0587_3AF0: Sig = Sig {
    len: 34,
    hash: 0x6fa8_1180_0bea_987b,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`release_node`] in `magic_0588` (0x10002580, 34 bytes).
pub const RELEASE_NODE_0588_2580: Sig = Sig {
    len: 34,
    hash: 0x420d_1857_5f9c_4681,
    calls: &[(0x12, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// [`sub_const`] in `magic_0168` (0x100033a0, 30 bytes).
pub const SUB_CONST_0168_33A0: Sig = Sig {
    len: 30,
    hash: 0x0512_f124_54aa_11d1,
    calls: &[],
    imports: &[],
};

/// [`sub_const`] in `magic_0430` (0x10003860, 30 bytes).
pub const SUB_CONST_0430_3860: Sig = Sig {
    len: 30,
    hash: 0x4266_3225_aa15_08b5,
    calls: &[],
    imports: &[],
};

/// [`sub_const`] in `magic_0568` (0x10002820, 30 bytes).
pub const SUB_CONST_0568_2820: Sig = Sig {
    len: 30,
    hash: 0x5ecc_e62b_11e5_ad45,
    calls: &[],
    imports: &[],
};

/// [`sub_const`] in `magic_0568` (0x10002dc0, 30 bytes).
pub const SUB_CONST_0568_2DC0: Sig = Sig {
    len: 30,
    hash: 0xa51a_8e92_0512_8775,
    calls: &[],
    imports: &[],
};

/// [`sub_const`] in `magic_0587` (0x10002ec0, 30 bytes).
pub const SUB_CONST_0587_2EC0: Sig = Sig {
    len: 30,
    hash: 0x08c5_316c_8b3e_0a11,
    calls: &[],
    imports: &[],
};

/// [`pool_node`] in `magic_0350` (0x10006280, 121 bytes).
pub const POOL_NODE_0350_6280: Sig = Sig {
    len: 121,
    hash: 0x9211_d10c_a834_e4b8,
    calls: &[(0x3d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`pool_node`] in `magic_0353` (0x10002180, 121 bytes).
pub const POOL_NODE_0353_2180: Sig = Sig {
    len: 121,
    hash: 0xb21d_f0d3_8caf_e1d5,
    calls: &[(0x3d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`pool_node`] in `magic_0434` (0x10007a50, 121 bytes).
pub const POOL_NODE_0434_7A50: Sig = Sig {
    len: 121,
    hash: 0x62b3_6628_da84_1564,
    calls: &[(0x3d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`pool_node`] in `magic_0610` (0x1000b980, 121 bytes).
pub const POOL_NODE_0610_B980: Sig = Sig {
    len: 121,
    hash: 0x4b6a_86d3_2089_64cc,
    calls: &[(0x3d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`pool_node`] in `magic_0672` (0x10001860, 121 bytes).
pub const POOL_NODE_0672_1860: Sig = Sig {
    len: 121,
    hash: 0x49af_951a_6dad_67ef,
    calls: &[(0x3d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};
