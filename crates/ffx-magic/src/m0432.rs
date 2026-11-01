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
