//! `magic_0432.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The sound's start (0x10005240, internal, cdecl `(id)`): with the word `*L` (`L`
/// = `*P`, `+0x04`) at 1 slot 711 `(*H, I, 0)` (`H` `+0x0a`, `I` the
/// immediate `+0x25`) runs and `*L` is put back (`P` read again,
/// `+0x2d`); slot 710 `(J, *H, id, 0xe0000)` (`J` the immediate `+0x47`,
/// `H` read again `+0x3d`); the words `+0x58`, `+0x62`, `+0x6c` and
/// `+0x76` become 0. Returns slot 710's `eax`. Each slot call passes the
/// words the original leaves on the stack.
pub fn sound_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let l = g.read::<u32>(w(0x04));
    let h = g.read::<u32>(w(0x0a));
    let old = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[0, w(0x25), h]);
    let s = slot_target(g, w(0x18), 0xb1c / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let l = g.read::<u32>(w(0x2d));
    p.push_all(&[0xe0000, fr.arg(g, 0)]);
    g.write::<u32>(l, old);
    p.push_all(&[g.read::<u32>(w(0x3d)), w(0x47)]);
    let s = slot_target(g, w(0x42), 0xb18 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    for off in [0x58, 0x62, 0x6c, 0x76] {
        g.write::<u32>(w(off), 0);
    }
    Ret::eax(eax)
}

/// [`sound_start`] (129 bytes, 1 overlay).
pub const SOUND_START: Sig = Sig {
    len: 129,
    hash: 0x0bd6_c59a_9ec3_5895,
    calls: &[],
    imports: &[],
};

/// Starts the block's object (0x10001da0, a table entry, no arguments;
/// `S = *G_S`, operand `+0x0b`): `S`
/// ([`start_object_at`](crate::s4_misc::start_object_at), scale the float
/// at `+0x04`, index 0, owners 0, 0); `S + 0xc4` = `S + 0x32c0`, `S +
/// 0x104` the immediate `+0x32`. Returns the word copied.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x4));
    let s = g.read::<u32>(w(0xb));
    crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x19), s, scale, 0, 0, 0);
    let v = g.read::<u32>(s + 0x32c0);
    g.write::<u32>(s + 0xc4, v);
    g.write::<u32>(s + 0x104, w(0x32));
    Ret::eax(v)
}

/// [`object_start`] (56 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 56,
    hash: 0xcd23_72cf_49dd_17dc,
    calls: &[(0x19, crate::Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};
/// A point far below (0x10001d30, cdecl `(a)`, a /GS frame): slot 237
/// `(*G_M, 0xe2, &V, a + 0xd0)` with the local `V = (-30, 0, 0, 1)`
/// (`ebp - 0x14`; `*G_M` operand `+0x2b`), then `a + 0xdc` 1.0. Returns
/// slot 237's `eax`.
pub fn point_below<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let out = fr.arg(g, 0).wrapping_add(0xd0);
    let v = ebp.wrapping_sub(0x14);
    for (i, x) in (0u32..).zip([0xc1f0_0000u32, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    let t = slot_target(g, w(0x20), 237);
    let m = g.read::<u32>(w(0x2b));
    let eax = g.call(t, 0, &[m, 0xe2, v, out]).eax;
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`point_below`] (108 bytes, 1 overlay; the `rel32` at `+0x64` is
/// `__security_check_cookie`).
pub const POINT_BELOW: Sig = Sig {
    len: 108,
    hash: 0xc1e3_4c09_81f2_78c7,
    calls: &[(0x64, crate::Callee::Any)],
    imports: &[],
};
