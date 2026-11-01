//! `magic_0184.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The parts' release (0x10001900, no arguments): for each of four
/// objects (the immediates `+0x06`, `+0x23`, `+0x40`, `+0x5d`), `**Q` the
/// immediate after it (`Q` at `+0x01`..) and slot 713 `(obj)`, each call
/// seeing the earlier ones' words. Returns 0.
pub fn release_four<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut stack: Vec<u32> = Vec::new();
    for (q, obj, imm, table) in [
        (0x01, 0x06, 0x0c, 0x11),
        (0x1e, 0x23, 0x29, 0x2e),
        (0x3b, 0x40, 0x46, 0x4b),
        (0x58, 0x5d, 0x63, 0x68),
    ] {
        let qp = g.read::<u32>(w(q));
        stack.insert(0, w(obj));
        g.write::<u32>(qp, w(imm));
        let s = slot_target(g, w(table), 713);
        // call width: 4 (the earlier calls' words left on the stack)
        g.call(s, 0, &stack);
    }
    Ret::eax(0)
}

/// [`release_four`] (122 bytes, 1 overlay).
pub const RELEASE_FOUR: Sig = Sig {
    len: 122,
    hash: 0x00ec_4b97_734f_8a8a,
    calls: &[],
    imports: &[],
};

/// The particle set-up (0x10001390, no arguments): for the effect `NAME`
/// (the immediate at `+0x06`, again in each call) slot 718 `(NAME)`,
/// slots 222 and 221 `(NAME, 0, 1000, 0x100)` and slot 719 `(NAME, 6,
/// i)` for i = 0..3 into four globals (`+0x65`, `+0x80`, `+0x9e`,
/// `+0xbc`). Returns the last one. Each slot call passes the words the
/// original leaves on the stack.
pub fn part_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = crate::pushes::Pushes::default();
    p.push(w(0x06));
    let s = slot_target(g, w(0x01), 0xb38 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0x100, 1000, 0, w(0x2a)]);
    let s = slot_target(g, w(0x13), 0x378 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0x100, 1000, 0, w(0x48)]);
    let s = slot_target(g, w(0x31), 0x374 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let mut last = 0;
    for (i, table, name, dst) in [
        (0, 0x4f, 0x5e, 0x65),
        (1, 0x6a, 0x79, 0x80),
        (2, 0x85, 0x94, 0x9e),
        (3, 0xa3, 0xb2, 0xbc),
    ] {
        p.push_all(&[i, 6, w(name)]);
        let s = slot_target(g, w(table), 0xb3c / 4);
        // call width: 12 (Pushes caps the words at 12)
        last = p.call(g, s, 0).eax;
        if i == 2 {
            // add esp, 0x48: everything pushed so far
            p.pop(18);
        }
        g.write::<u32>(w(dst), last);
    }
    Ret::eax(last)
}

/// [`part_setup`] (193 bytes, 1 overlay).
pub const PART_SETUP: Sig = Sig {
    len: 193,
    hash: 0x8e45_ba1b_2eb6_e0e2,
    calls: &[],
    imports: &[],
};
