//! Map effects' task functions the own work list repeats (round 4): the
//! same code in several DLLs with other constants (the instructions'
//! immediates, read from each copy's code). See [`crate::rcef`] for the
//! `rcef*` slots.
//!
//! - [`seq_wait_447`]: 12 copies in 10 DLLs (`magic_0191` 0x10001110 ...).
//! - The task wait of [`crate::rcef::create_task_wait`] with other times:
//!   the signatures `CREATE_TASK_WAIT_*` (7 copies in 7 DLLs), bound to
//!   that port.

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// `(obj)` (`magic_0191` 0x10001110, 76 bytes): slot 447 `(V, K)` with
/// `V` the word at `+0x198` of the game object `*G` (operand `+0x04`)
/// and `K` the copy's `push imm32` (`+0x09`), `rcefTaskSetSeq(obj, cb)`
/// (slot 450, `cb` at `+0x26`) and `rcefTaskWait(obj, T)` (slot 451, `T`
/// the `push imm8` at `+0x3b`), each with the earlier calls' words still
/// on the stack after its own. Returns slot 451's `eax`.
pub fn seq_wait_447<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(0x198));
    let k = w(0x09);
    let s447 = slot_target(g, w(0x14), 447);
    g.call(s447, fr.ecx, &[v, k]);
    let obj = fr.arg(g, 0);
    let cb = w(0x26);
    let s450 = slot_target(g, w(0x21), 450);
    g.call(s450, fr.ecx, &[obj, cb, v, k]);
    let time = disp8(img, f + 0x3b);
    let s451 = slot_target(g, w(0x36), 451);
    let r = g.call(s451, fr.ecx, &[obj, time, obj, cb, v, k]);
    Ret::eax(r.eax)
}

/// [`seq_wait_447`] in `magic_0191` (0x10001110, 76 bytes).
pub const SEQ_WAIT_447_0191_1110: Sig = Sig {
    len: 76,
    hash: 0x8ea1_7548_a0f2_ea0e,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0357` (0x100010a0, 76 bytes).
pub const SEQ_WAIT_447_0357_10A0: Sig = Sig {
    len: 76,
    hash: 0xa262_d3c7_573f_9daa,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0360` (0x10001110, 76 bytes).
pub const SEQ_WAIT_447_0360_1110: Sig = Sig {
    len: 76,
    hash: 0xa566_d3cd_b85b_6868,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0360` (0x10001210, 76 bytes).
pub const SEQ_WAIT_447_0360_1210: Sig = Sig {
    len: 76,
    hash: 0xef5d_655e_fe7a_b571,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0361` (0x10001210, 76 bytes).
pub const SEQ_WAIT_447_0361_1210: Sig = Sig {
    len: 76,
    hash: 0x6abd_bdf8_58c3_6e99,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0361` (0x10001490, 76 bytes).
pub const SEQ_WAIT_447_0361_1490: Sig = Sig {
    len: 76,
    hash: 0x52b1_ad9f_1d96_8f36,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0384` (0x100012a0, 76 bytes).
pub const SEQ_WAIT_447_0384_12A0: Sig = Sig {
    len: 76,
    hash: 0x9229_359b_8526_26ee,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0384` (0x100013b0, 76 bytes).
pub const SEQ_WAIT_447_0384_13B0: Sig = Sig {
    len: 76,
    hash: 0x312a_da46_a3b0_2c4f,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0421` (0x100012b0, 76 bytes).
pub const SEQ_WAIT_447_0421_12B0: Sig = Sig {
    len: 76,
    hash: 0xf190_03a1_63f6_cc89,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0522` (0x10001780, 76 bytes).
pub const SEQ_WAIT_447_0522_1780: Sig = Sig {
    len: 76,
    hash: 0xe8a2_6f95_7b8c_4aed,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0525` (0x10001410, 76 bytes).
pub const SEQ_WAIT_447_0525_1410: Sig = Sig {
    len: 76,
    hash: 0x81ee_32c4_dc70_d207,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_447`] in `magic_0535` (0x100012a0, 76 bytes).
pub const SEQ_WAIT_447_0535_12A0: Sig = Sig {
    len: 76,
    hash: 0xb1a7_d2a6_9d94_8d07,
    calls: &[],
    imports: &[],
};

/// [`create_task_wait`] in `magic_0191` (0x10001160, 71 bytes).
pub const CREATE_TASK_WAIT_0191_1160: Sig = Sig {
    len: 71,
    hash: 0x5b01_f46a_c283_09de,
    calls: &[],
    imports: &[],
};

/// [`create_task_wait`] in `magic_0300` (0x10001120, 71 bytes).
pub const CREATE_TASK_WAIT_0300_1120: Sig = Sig {
    len: 71,
    hash: 0x04ab_a7e3_9c85_9741,
    calls: &[],
    imports: &[],
};

/// [`create_task_wait`] in `magic_0301` (0x10001a30, 71 bytes).
pub const CREATE_TASK_WAIT_0301_1A30: Sig = Sig {
    len: 71,
    hash: 0x5fba_f0e6_9e15_7344,
    calls: &[],
    imports: &[],
};

/// [`create_task_wait`] in `magic_0378` (0x10001080, 71 bytes).
pub const CREATE_TASK_WAIT_0378_1080: Sig = Sig {
    len: 71,
    hash: 0xa5b4_1b2e_f8b1_19b8,
    calls: &[],
    imports: &[],
};

/// [`create_task_wait`] in `magic_0416` (0x100010a0, 71 bytes).
pub const CREATE_TASK_WAIT_0416_10A0: Sig = Sig {
    len: 71,
    hash: 0x4d1f_858d_99d6_b276,
    calls: &[],
    imports: &[],
};

/// [`create_task_wait`] in `magic_0521` (0x10001090, 71 bytes).
pub const CREATE_TASK_WAIT_0521_1090: Sig = Sig {
    len: 71,
    hash: 0x3dd4_1567_608d_7f97,
    calls: &[],
    imports: &[],
};

/// [`create_task_wait`] in `magic_0522` (0x100011b0, 71 bytes).
pub const CREATE_TASK_WAIT_0522_11B0: Sig = Sig {
    len: 71,
    hash: 0x8a80_fc69_edf2_23ed,
    calls: &[],
    imports: &[],
};

/// `(obj)` (`magic_0357` 0x100018e0, 198 bytes; 8 copies): the object
/// attached (slot 441 or 442, the copy's `[eax + 0x6e4]`/`0x6e8` at
/// `+0x1c`: `(obj, V, A)`, `V` the word at `*G + D` (`D` at `+0x10`,
/// `G` at `+0x04`), `A` the `push imm8` at `+0x0d`), its mode set
/// (`rcefObjSetMode(obj, M)`, slot 443, `M` at `+0x28`), eight `mov dword
/// [obj + d], imm32` (from `+0x42`, 10 bytes each), `rcefMainSetPar(P,
/// obj, 0, B)` (slot 440, `P` the game word `*G` read again, `B` at
/// `+0x38`), `rcefTaskSetSeq(obj, cb)` (450, `+0xa0`) and
/// `rcefTaskCallSeq(obj, cb2)` (449, `+0xb3`), each call with the earlier
/// ones' words still on the stack after its own. Returns slot 449's
/// `eax`.
pub fn attach_set_par<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let obj = fr.arg(g, 0);
    let a = disp8(img, f + 0x0d);
    let v = g.read::<u32>(p.wrapping_add(w(0x10)));
    // mov eax, [eax + 4 * slot]
    let attach = slot_target(g, w(0x15), w(0x1c) / 4);
    g.call(attach, fr.ecx, &[obj, v, a]);
    let mode = disp8(img, f + 0x28);
    let s443 = slot_target(g, w(0x23), 443);
    g.call(s443, fr.ecx, &[obj, mode, obj, v, a]);
    let b = disp8(img, f + 0x38);
    let p2 = g.read::<u32>(w(0x3e));
    for k in 0..8 {
        // c7 86 disp32 imm32
        let insn = f + 0x42 + 10 * k;
        g.write::<u32>(obj.wrapping_add(img.word(insn + 2)), img.word(insn + 6));
    }
    let s440 = slot_target(g, w(0x33), 440);
    g.call(s440, fr.ecx, &[p2, obj, 0, b, obj, mode, obj, v, a]);
    let (cb, cb2) = (w(0xa0), w(0xb3));
    let s450 = slot_target(g, w(0x9b), 450);
    g.call(
        s450,
        fr.ecx,
        &[obj, cb, p2, obj, 0, b, obj, mode, obj, v, a],
    );
    let s449 = slot_target(g, w(0xae), 449);
    let r = g.call(
        s449,
        fr.ecx,
        &[obj, cb2, obj, cb, p2, obj, 0, b, obj, mode, obj, v, a],
    );
    Ret::eax(r.eax)
}

/// [`attach_set_par`] in `magic_0357` (0x100018e0, 198 bytes).
pub const ATTACH_SET_PAR_0357_18E0: Sig = Sig {
    len: 198,
    hash: 0x0456_dd01_35f2_33ea,
    calls: &[],
    imports: &[],
};

/// [`attach_set_par`] in `magic_0359` (0x10001820, 198 bytes).
pub const ATTACH_SET_PAR_0359_1820: Sig = Sig {
    len: 198,
    hash: 0xb9c8_ce70_27af_7823,
    calls: &[],
    imports: &[],
};

/// [`attach_set_par`] in `magic_0361` (0x10001be0, 198 bytes).
pub const ATTACH_SET_PAR_0361_1BE0: Sig = Sig {
    len: 198,
    hash: 0xd595_d3d6_36ca_2afa,
    calls: &[],
    imports: &[],
};

/// [`attach_set_par`] in `magic_0416` (0x10001710, 198 bytes).
pub const ATTACH_SET_PAR_0416_1710: Sig = Sig {
    len: 198,
    hash: 0x7797_c0f1_bf3c_da54,
    calls: &[],
    imports: &[],
};

/// [`attach_set_par`] in `magic_0521` (0x10001200, 198 bytes).
pub const ATTACH_SET_PAR_0521_1200: Sig = Sig {
    len: 198,
    hash: 0x3c77_ac73_c0e4_26f7,
    calls: &[],
    imports: &[],
};

/// [`attach_set_par`] in `magic_0521` (0x100012d0, 198 bytes).
pub const ATTACH_SET_PAR_0521_12D0: Sig = Sig {
    len: 198,
    hash: 0x13d8_a6c5_15c4_b1fa,
    calls: &[],
    imports: &[],
};

/// [`attach_set_par`] in `magic_0522` (0x10001290, 198 bytes).
pub const ATTACH_SET_PAR_0522_1290: Sig = Sig {
    len: 198,
    hash: 0x627c_dfda_e21e_a78e,
    calls: &[],
    imports: &[],
};

/// [`attach_set_par`] in `magic_0526` (0x10001380, 198 bytes).
pub const ATTACH_SET_PAR_0526_1380: Sig = Sig {
    len: 198,
    hash: 0x4dd5_1654_5208_acc2,
    calls: &[],
    imports: &[],
};

/// [`attach_set_par`] without the mode call, with `n` stores
/// (`magic_0300` 0x10001300, 182 bytes, 8 stores; `magic_0301`
/// 0x10001d00, 222 bytes, 12 stores): the attach slot at `+0x1c`, `A` at
/// `+0x0d`, `D` at `+0x10`; `rcefMainSetPar(P, obj, 0, B)` (`B` at
/// `+0x28`, `P` at `+0x2e`, the stores from `+0x32`), then the set-seq
/// and call-seq callbacks after the stores (`+0x0e` and `+0x21` past
/// them). Returns slot 449's `eax`.
fn attach_plain<G: Game>(g: &mut G, img: &Image, fr: &Frame, n: u32) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let obj = fr.arg(g, 0);
    let a = disp8(img, f + 0x0d);
    let v = g.read::<u32>(p.wrapping_add(w(0x10)));
    // mov eax, [eax + 4 * slot]
    let attach = slot_target(g, w(0x15), w(0x1c) / 4);
    g.call(attach, fr.ecx, &[obj, v, a]);
    let b = disp8(img, f + 0x28);
    let p2 = g.read::<u32>(w(0x2e));
    for k in 0..n {
        // c7 86 disp32 imm32
        let insn = f + 0x32 + 10 * k;
        g.write::<u32>(obj.wrapping_add(img.word(insn + 2)), img.word(insn + 6));
    }
    let end = 0x32 + 10 * n;
    let s440 = slot_target(g, w(0x23), 440);
    g.call(s440, fr.ecx, &[p2, obj, 0, b, obj, v, a]);
    let (cb, cb2) = (w(end + 0x0e), w(end + 0x21));
    let s450 = slot_target(g, w(end + 0x09), 450);
    g.call(s450, fr.ecx, &[obj, cb, p2, obj, 0, b, obj, v, a]);
    let s449 = slot_target(g, w(end + 0x1c), 449);
    let r = g.call(s449, fr.ecx, &[obj, cb2, obj, cb, p2, obj, 0, b, obj, v, a]);
    Ret::eax(r.eax)
}

/// [`attach_plain`] with 8 stores (182 bytes).
pub fn attach_plain8<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    attach_plain(g, img, fr, 8)
}

/// [`attach_plain`] with 12 stores (222 bytes).
pub fn attach_plain12<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    attach_plain(g, img, fr, 12)
}

/// [`attach_plain8`] in `magic_0300` (0x10001300, 182 bytes).
pub const ATTACH_PLAIN8_0300_1300: Sig = Sig {
    len: 182,
    hash: 0x6647_5c8b_f044_4c39,
    calls: &[],
    imports: &[],
};

/// [`attach_plain8`] in `magic_0301` (0x10001de0, 182 bytes).
pub const ATTACH_PLAIN8_0301_1DE0: Sig = Sig {
    len: 182,
    hash: 0xfee1_0c08_74d9_1acb,
    calls: &[],
    imports: &[],
};

/// [`attach_plain8`] in `magic_0510` (0x100015c0, 182 bytes).
pub const ATTACH_PLAIN8_0510_15C0: Sig = Sig {
    len: 182,
    hash: 0xf983_b317_ca6d_ca24,
    calls: &[],
    imports: &[],
};

/// [`attach_plain8`] in `magic_0529` (0x10001270, 182 bytes).
pub const ATTACH_PLAIN8_0529_1270: Sig = Sig {
    len: 182,
    hash: 0x89a8_4dfc_6dd9_6584,
    calls: &[],
    imports: &[],
};

/// [`attach_plain12`] in `magic_0301` (0x10001d00, 222 bytes).
pub const ATTACH_PLAIN12_0301_1D00: Sig = Sig {
    len: 222,
    hash: 0xb9d2_8e23_d988_8529,
    calls: &[],
    imports: &[],
};

/// [`attach_plain12`] in `magic_0360` (0x10001400, 222 bytes).
pub const ATTACH_PLAIN12_0360_1400: Sig = Sig {
    len: 222,
    hash: 0x6b14_b0e9_fdbe_f558,
    calls: &[],
    imports: &[],
};

/// [`attach_plain12`] in `magic_0360` (0x100014e0, 222 bytes).
pub const ATTACH_PLAIN12_0360_14E0: Sig = Sig {
    len: 222,
    hash: 0xe8b7_2a48_79cc_74f2,
    calls: &[],
    imports: &[],
};

/// [`attach_plain12`] in `magic_0510` (0x100014e0, 222 bytes).
pub const ATTACH_PLAIN12_0510_14E0: Sig = Sig {
    len: 222,
    hash: 0xd3bb_d6bb_ac0e_d2ef,
    calls: &[],
    imports: &[],
};
