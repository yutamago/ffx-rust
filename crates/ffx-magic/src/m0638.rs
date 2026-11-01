//! `magic_0638.dll`'s own functions (besides the runtime and the shared
//! code), and their copy in `magic_0689`.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start on its cue (0x10003070, an overlay table entry, no
/// arguments): nothing while slot 225 `(1)` answers non-zero; else, once
/// (the flag at operand `+0x18`), slot 226 `(*G_H, 0)`; then when slot
/// 227 `(*G_H)` answers 1 or more, the two nodes are started
/// ([`two_node_start`](crate::o6_fam_pairs3::two_node_start), the `rel32`
/// at `+0x5b`) and the DLL's two callback words set (operands `+0x61`,
/// `+0x6b`, to the immediates after them). Returns 0.
///
/// `magic_0638` and `magic_0689` (118 bytes each; the same code with
/// their own [`two_node_start`](crate::o6_fam_pairs3::two_node_start)).
pub fn cue_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = Pushes::default();
    p.push(1);
    let t = slot_target(g, w(0x1), 225);
    // call width: 12 (Pushes caps the words at 12)
    if p.call(g, t, 0).eax != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x18)) == 0 {
        let mut p = Pushes::default();
        p.push(0);
        p.push(g.read::<u32>(w(0x26)));
        let t = slot_target(g, w(0x20), 226);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0);
        g.write::<u32>(w(0x37), 1);
    }
    let mut p = Pushes::default();
    p.push(g.read::<u32>(w(0x46)));
    let t = slot_target(g, w(0x40), 227);
    // call width: 12 (Pushes caps the words at 12)
    if p.call(g, t, 0).eax.cast_signed() >= 1 {
        let callee = Frame {
            at: img.rel32_target(f + 0x5b),
            esp: fr.esp.wrapping_sub(4),
            eax: 0,
            ecx: 0,
            edx: 0,
            xmm: [0; 4],
        };
        crate::o6_fam_pairs3::two_node_start(g, img, &callee);
        g.write::<u32>(w(0x61), w(0x65));
        g.write::<u32>(w(0x6b), w(0x6f));
    }
    Ret::eax(0)
}

/// [`cue_start`] in `magic_0638` (0x10003070).
pub const CUE_START_0638: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(
        0x5b,
        Callee::Sig(&crate::o6_fam_pairs3::TWO_NODE_START_0638_3270),
    )],
    imports: &[],
};

/// [`cue_start`] in `magic_0689` (0x10003070).
pub const CUE_START_0689: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(
        0x5b,
        Callee::Sig(&crate::o6_fam_pairs3::TWO_NODE_START_0689_32C0),
    )],
    imports: &[],
};
