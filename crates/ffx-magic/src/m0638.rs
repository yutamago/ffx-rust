//! `magic_0638.dll`'s own functions (besides the runtime and the shared
//! code), and their copy in `magic_0689`.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start on its cue (0x10003070, an overlay table entry, no
/// arguments): nothing while slot 225 `(1)` answers non-zero; else, once
/// (the flag at operand `+0x18`), slot 226 `(*G_H, 0)`; then when slot
/// 227 `(*G_H)` answers 1 or more, the effect's start (the `rel32` at
/// `+0x5b`, the port bound there) and the DLL's two callback words set
/// (operands `+0x61`, `+0x6b`, to the immediates after them). Returns 0.
///
/// `magic_0638` and `magic_0689` (118 bytes each; the same code with
/// their own [`two_node_start`](crate::o6_fam_pairs3::two_node_start)),
/// `magic_0469` and `magic_0551` (their own
/// [`objects_start`](crate::r12h_start::objects_start)).
pub fn cue_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    cue(g, img, fr, 0, bound)
}

/// [`cue_start`] with slot 226's second word 1 (`push 1` where
/// [`cue_start`] has `push eax`, one byte longer: 119 bytes): `magic_0470`,
/// `0554`, `0571`, `0572` and `0590`.
pub fn cue_start_1<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    cue(g, img, fr, 1, bound)
}

/// [`cue_start`] in `magic_0479` (0x10003a40): its start is
/// [`crate::m0479::start`].
pub fn cue_start_0479<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    cue(g, img, fr, 0, |g, img, callee| {
        crate::m0479::start(g, img, callee.at);
    })
}

/// [`cue_start`] in `magic_0671` (0x10001210): its start is
/// [`crate::m0671::start`].
pub fn cue_start_0671<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    cue(g, img, fr, 0, |g, img, callee| {
        crate::m0671::start(g, img, callee);
    })
}

/// The entry port bound at `callee.at` run on `callee` (code without a
/// port goes to the host).
fn bound<G: Game>(g: &mut G, img: &Image, callee: &Frame) {
    match crate::bind(img, callee.at).and_then(crate::entry::<G>) {
        Some(port) => {
            port(g, img, callee);
        }
        None => {
            // call width: 0
            g.call(callee.at, 0, &[]);
        }
    }
}

/// [`cue_start`]'s body; the operands after `+0x1e` sit `d` bytes on
/// (`d` 1: `push 1`, also slot 226's second word); `start` runs the
/// effect's start (its frame at the `rel32` target).
fn cue<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    d: u32,
    start: impl FnOnce(&mut G, &Image, &Frame),
) -> Ret {
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
        p.push(d);
        p.push(g.read::<u32>(w(0x26 + d)));
        let t = slot_target(g, w(0x20 - d), 226);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0);
        g.write::<u32>(w(0x37 + d), 1);
    }
    let mut p = Pushes::default();
    p.push(g.read::<u32>(w(0x46 + d)));
    let t = slot_target(g, w(0x40 + d), 227);
    // call width: 12 (Pushes caps the words at 12)
    if p.call(g, t, 0).eax.cast_signed() >= 1 {
        let callee = Frame {
            at: img.rel32_target(f + 0x5b + d),
            esp: fr.esp.wrapping_sub(4),
            ..Frame::default()
        };
        start(g, img, &callee);
        g.write::<u32>(w(0x61 + d), w(0x65 + d));
        g.write::<u32>(w(0x6b + d), w(0x6f + d));
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

/// [`cue_start`] in `magic_0469` (0x10001210).
pub const CUE_START_0469: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::r12h_start::OBJECTS_START_0469))],
    imports: &[],
};

/// [`cue_start`] in `magic_0551` (0x10001210).
pub const CUE_START_0551: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::r12h_start::OBJECTS_START_0551))],
    imports: &[],
};

/// [`cue_start_1`] in `magic_0470` (0x10001210).
pub const CUE_START_0470: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&crate::r12h_start::OBJECTS_START_B_0470))],
    imports: &[],
};

/// [`cue_start_1`] in `magic_0554` (0x10001210).
pub const CUE_START_0554: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&crate::r12h_start::OBJECTS_START_B_0554))],
    imports: &[],
};

/// [`cue_start_1`] in `magic_0590` (0x10001210).
pub const CUE_START_0590: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&crate::r12h_start::OBJECTS_START_B_0590))],
    imports: &[],
};

/// [`cue_start_0479`] (0x10003a40).
pub const CUE_START_0479: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0479::START))],
    imports: &[],
};

/// [`cue_start_0671`] (0x10001210).
pub const CUE_START_0671: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0671::START))],
    imports: &[],
};

/// [`cue_start`] in `magic_0472` (0x100030a0).
pub const CUE_START_0472: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0472::START))],
    imports: &[],
};

/// [`cue_start`] in `magic_0475` (0x100012b0).
pub const CUE_START_0475: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0475::START))],
    imports: &[],
};

/// [`cue_start`] in `magic_0688` (0x10003940).
pub const CUE_START_0688: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0688::START))],
    imports: &[],
};

/// [`cue_start`] in `magic_0672` (0x10001210).
pub const CUE_START_0672: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0672::START))],
    imports: &[],
};

/// [`cue_start`] in `magic_0596` (0x10006800).
pub const CUE_START_0596: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0596::START2))],
    imports: &[],
};

/// [`cue_start`] in `magic_0476` (0x100012b0).
pub const CUE_START_0476: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0476::START))],
    imports: &[],
};

/// [`cue_start`] in `magic_0673` (0x10001200).
pub const CUE_START_0673: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::m0673::EFFECT_START))],
    imports: &[],
};
