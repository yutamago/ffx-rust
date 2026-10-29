//! Effect start-ups the own work list repeats (round 4): the same code in
//! several DLLs, the fields it reads as the instructions' displacements
//! (read from each copy's code).

use crate::runtime::slot_target;
use crate::sb_misc::effect_reset;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// No arguments (`magic_0596` 0x10007060, 142 bytes; 11 copies in 11
/// DLLs): the effect's camera and records reset ([`effect_reset`]), the
/// started flag (the byte at operand `+0x13`) set to 1, then the
/// effect's matrix `M`: `FFXVu0UnitMatrix(M)` (slot 235), and
/// `FFXVu0RotMatrixY(M, M, a)` (slot 329, `a` the float at `+d` of the
/// caster `*G_CASTER`, `d` the displacement at `+0x29`; it takes the
/// place of the unit call's word on the stack); the caster's position
/// (the four floats at the displacements `+0x4d`, `+0x5d`, `+0x6d`,
/// `+0x80`) copied into the effect's vector (operands `+0x55`, `+0x65`,
/// `+0x78`, `+0x88`). Returns slot 329's `eax`.
pub fn start_turned<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let caster = g.read::<u32>(w(0x03));
    effect_reset(g, img, img.rel32_target(f + 0x08));
    g.write::<u8>(w(0x13), 1);
    let s235 = slot_target(g, w(0x0d), 235);
    g.call(s235, fr.ecx, &[w(0x1f)]);
    let angle = g.read::<u32>(caster.wrapping_add(w(0x29)));
    let s329 = slot_target(g, w(0x2e), 329);
    let r = g.call(s329, fr.ecx, &[w(0x43), w(0x3e), angle]);
    // movss: copies of the bits
    for (src, dst) in [(0x4d, 0x55), (0x5d, 0x65), (0x6d, 0x78), (0x80, 0x88)] {
        let v = g.read::<u32>(caster.wrapping_add(w(src)));
        g.write::<u32>(w(dst), v);
    }
    Ret::eax(r.eax)
}

/// [`start_turned`] in `magic_0596` (0x10007060, 142 bytes).
pub const START_TURNED_0596_7060: Sig = Sig {
    len: 142,
    hash: 0x5245_53ce_c5d3_5da0,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0162` (0x100029f0, 142 bytes).
pub const START_TURNED_0162_29F0: Sig = Sig {
    len: 142,
    hash: 0x26f1_e2f8_6a30_9df1,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0304` (0x10001c10, 142 bytes).
pub const START_TURNED_0304_1C10: Sig = Sig {
    len: 142,
    hash: 0x31d7_8e16_6cae_ee09,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0307` (0x10002430, 142 bytes).
pub const START_TURNED_0307_2430: Sig = Sig {
    len: 142,
    hash: 0x3ca0_a8c5_1701_b09a,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0568` (0x10001860, 142 bytes).
pub const START_TURNED_0568_1860: Sig = Sig {
    len: 142,
    hash: 0xe0ee_629b_8f18_7748,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0586` (0x10002750, 142 bytes).
pub const START_TURNED_0586_2750: Sig = Sig {
    len: 142,
    hash: 0xf0a1_3871_ae99_5891,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0587` (0x10001890, 142 bytes).
pub const START_TURNED_0587_1890: Sig = Sig {
    len: 142,
    hash: 0x5313_02b0_3f69_f266,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0588` (0x10001b70, 142 bytes).
pub const START_TURNED_0588_1B70: Sig = Sig {
    len: 142,
    hash: 0xd458_8694_0bd0_52f9,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0589` (0x10001e00, 142 bytes).
pub const START_TURNED_0589_1E00: Sig = Sig {
    len: 142,
    hash: 0xe8cb_75e4_e27c_d055,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0598` (0x10007470, 142 bytes).
pub const START_TURNED_0598_7470: Sig = Sig {
    len: 142,
    hash: 0x6bad_8c55_1298_ff19,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};

/// [`start_turned`] in `magic_0605` (0x10007db0, 142 bytes).
pub const START_TURNED_0605_7DB0: Sig = Sig {
    len: 142,
    hash: 0x8d35_ce7e_4e1f_9082,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};
