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
