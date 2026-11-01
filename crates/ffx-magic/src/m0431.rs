//! `magic_0431.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Whether both sounds are ready (0x10001620, no arguments): for the ids
/// 0x3019 and 0x301f in turn slot 5 `(id)` must answer 0 and slot 28
/// `(id, 1)` must answer 0. Returns 1 when all four do, else 0.
pub fn sounds_ready<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    for (t5, t28, id) in [(0x01, 0x17, 0x3019), (0x35, 0x4b, 0x301f)] {
        let s = slot_target(g, w(t5), 0x14 / 4);
        if g.call(s, 0, &[id]).eax != 0 {
            return Ret::eax(0);
        }
        let s = slot_target(g, w(t28), 0x70 / 4);
        if g.call(s, 0, &[id, 1]).eax != 0 {
            return Ret::eax(0);
        }
    }
    Ret::eax(1)
}

/// [`sounds_ready`] (108 bytes, 1 overlay).
pub const SOUNDS_READY: Sig = Sig {
    len: 108,
    hash: 0x07bf_afc4_11c6_1002,
    calls: &[],
    imports: &[],
};

/// The actor's colour reset (0x10001a10, no arguments): the actor `a` =
/// `T[*B]` (`B` = `*P`, `+0x01`; `T` `+0x11`): slot 357 `(a, -1,
/// 0x3c00)`, slot 356 `(a, -1, 0x2ee0)`, slot 362 `(*X, a, 1, 0)` (`X`
/// `+0x4c`), slot 357 `(a, -1, -1)` and slot 356 `(a, -1, -1)`. Returns
/// the last slot's `eax`. Each slot call passes the words the original
/// leaves on the stack.
pub fn colour_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(colours(g, img, fr.at))
}

/// [`colour_reset`]'s body at `f` (also `m0434`'s colour show, same
/// operands). Returns the last slot's `eax`.
pub(crate) fn colours<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x01))));
    let a = g.read::<u32>(w(0x11).wrapping_add(b * 4));
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[0x3c00, u32::MAX, a]);
    let s = slot_target(g, w(0x16), 0x594 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0x2ee0, u32::MAX, a]);
    let s = slot_target(g, w(0x26), 0x590 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0, 1, a, g.read::<u32>(w(0x4c))]);
    let s = slot_target(g, w(0x3b), 0x5a8 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[u32::MAX, u32::MAX, a]);
    let s = slot_target(g, w(0x53), 0x594 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[u32::MAX, u32::MAX, a]);
    let s = slot_target(g, w(0x65), 0x590 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0).eax
}

/// [`colour_reset`] (123 bytes, 1 overlay).
pub const COLOUR_RESET: Sig = Sig {
    len: 123,
    hash: 0xc807_db32_941a_c1f4,
    calls: &[],
    imports: &[],
};

/// The camera's start (0x10001760, internal, no arguments): the camera's
/// angle as `m0449`'s ([`camera_angle`](crate::m0449::camera_angle), `C`
/// = `E + 0x820`, into `E + 0xd00`); `E + 0xc64`, `E + 0xc68` and `E +
/// 0xd10` become 0, the words `+0xac` and `+0xb6` 1. Returns slot 333's
/// `eax`.
pub fn camera_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (e, eax) = crate::m0449::camera_angle(g, img, fr.at, 0x820, 0xd00);
    g.write::<u32>(e + 0xc64, 0);
    g.write::<u32>(e + 0xc68, 0);
    g.write::<u32>(e + 0xd10, 0);
    g.write::<u32>(w(0xac), 1);
    g.write::<u32>(w(0xb6), 1);
    Ret::eax(eax)
}

/// [`camera_start`] (195 bytes, 1 overlay).
pub const CAMERA_START: Sig = Sig {
    len: 195,
    hash: 0xa914_3b26_1b95_c219,
    calls: &[],
    imports: &[],
};

/// The actors' show and the sounds (0x10001830, internal, no arguments):
/// slot 384 `(T[B[k]])` for the actors `k` = 1..4 (`B` = `*P`, read
/// again for each, `+0x01`/`+0x1e`/`+0x3b`/`+0x58`; `T` `+0x0c`/`+0x29`/
/// `+0x46`/`+0x63`), then for the ids 0x3019 and 0x301f slot 25 `(id, 1)`
/// and slot 24 `(id, 0)`; a tail jump into the parts' registration
/// ([`register_parts`](crate::sa_misc::register_parts), `+0xbc`) with
/// slot 24's `eax`. Returns what that returns. Each slot call passes the
/// words the original leaves on the stack.
pub fn show_and_sounds<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut st = crate::pushes::Pushes::default();
    let actors = [
        (0x01, 0x0c, 0x11),
        (0x1e, 0x29, 0x2e),
        (0x3b, 0x46, 0x4b),
        (0x58, 0x63, 0x68),
    ];
    for (nth, (ptr_op, table_op, slot_op)) in (1u32..).zip(actors) {
        let actor = u32::from(g.read::<u8>(g.read::<u32>(w(ptr_op)) + nth));
        st.push(g.read::<u32>(w(table_op).wrapping_add(actor * 4)));
        let target = slot_target(g, w(slot_op), 0x600 / 4);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, target, 0);
    }
    let mut eax = 0;
    for (t25, t24, id) in [(0x75, 0x86, 0x3019), (0x97, 0xa8, 0x301f)] {
        st.push_all(&[1, id]);
        let target = slot_target(g, w(t25), 0x64 / 4);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, target, 0);
        st.push_all(&[0, id]);
        let target = slot_target(g, w(t24), 0x60 / 4);
        // call width: 12 (Pushes caps the words at 12)
        eax = st.call(g, target, 0).eax;
    }
    Ret::eax(crate::sa_misc::register_parts(
        g,
        img,
        img.rel32_target(f + 0xbc),
        eax,
    ))
}

/// [`show_and_sounds`] (192 bytes, 1 overlay).
pub const SHOW_AND_SOUNDS: Sig = Sig {
    len: 192,
    hash: 0xf0da_38ac_ff79_d2d1,
    calls: &[(0xbc, crate::Callee::Sig(&crate::sa_misc::REGISTER_PARTS))],
    imports: &[],
};
