//! `magic_0300.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// An effect object's set-up (0x10001600, cdecl `(a)`): slot 442 `(a,
/// (*G_E)[0x1a0], 0)`, slot 443 `(a, 0x10)`; `a + 0xd0..0xdc` `(0, -0.1,
/// 0, 0)`, `+0x130..0x138` 0.06, `+0x13c` 0; slot 440 `(*G_E, a, 0, 1)`,
/// slot 450 `(a, F1)` and slot 449 `(a, F2)` (`*G_E` operand `+0x04`/`+0x3e`,
/// `F1`, `F2` the immediates `+0xa0`, `+0xb3`; each over the earlier
/// words). Returns slot 449's `eax`.
pub fn object_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let a = fr.arg(g, 0);
    let mut s = Pushes::default();
    let x = g.read::<u32>(g.read::<u32>(w(0x4)) + 0x1a0);
    s.push_all(&[0, x, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x15), 442), fr.ecx);
    s.push_all(&[0x10, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x23), 443), fr.ecx);
    s.push_all(&[1, 0, a, g.read::<u32>(w(0x3e))]);
    for (o, v) in [(0xd0u32, 0u32), (0xd4, 0xbdcc_cccd), (0xd8, 0), (0xdc, 0)] {
        g.write::<u32>(a + o, v);
    }
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(a + o, 0x3d75_c28f);
    }
    g.write::<u32>(a + 0x13c, 0);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x33), 440), fr.ecx);
    s.push_all(&[w(0xa0), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x9b), 450), fr.ecx);
    s.push_all(&[w(0xb3), a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0xae), 449), fr.ecx).eax)
}

/// [`object_setup`] (198 bytes, 1 overlay).
pub const OBJECT_SETUP: Sig = Sig {
    len: 198,
    hash: 0x7f71_6826_610d_5f12,
    calls: &[],
    imports: &[],
};

/// An effect object at a bone (0x100014f0, cdecl `(a)`, a /GS frame):
/// slot 457 `((*G_E)[0x1a0], 6, &P)` (`P` a local at `ebp - 0x14`, `*G_E`
/// operand `+0x19`) gives a point; `a + 0xd0..0xdc` = `(P.x, P.y - K,
/// P.z, 0)` (`K` at `+0x3b`; `P.y` takes the difference too), `+0xe0..`
/// `(0, 15, 0, 0)`, `+0x130..0x138` 0.045, `+0x13c` 0; slot 440 `(*G_E, a,
/// 0, 0)`, slot 450 `(a, F1)` and slot 449 `(a, F2)` (the immediates
/// `+0xde`, `+0xf1`), each over the earlier words. Returns slot 449's
/// `eax`.
pub fn bone_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let a = fr.arg(g, 0);
    let p = ebp.wrapping_sub(0x14);
    let mut s = Pushes::default();
    let x = g.read::<u32>(g.read::<u32>(w(0x19)) + 0x1a0);
    s.push_all(&[p, 6, x]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x26), 457), fr.ecx);
    let y = crate::sb_sse::sub(g.read::<f32>(p + 4), g.read::<f32>(w(0x3b)));
    s.push_all(&[0, 0, a, g.read::<u32>(w(0x5d))]);
    let px = g.read::<u32>(p);
    g.write::<u32>(a + 0xd0, px);
    let pz = g.read::<u32>(p + 8);
    g.write::<f32>(p + 4, y);
    g.write::<f32>(a + 0xd4, y);
    g.write::<u32>(a + 0xd8, pz);
    for (o, v) in [
        (0xdcu32, 0u32),
        (0xe0, 0),
        (0xe4, 0x4170_0000),
        (0xe8, 0),
        (0xec, 0),
    ] {
        g.write::<u32>(a + o, v);
    }
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(a + o, 0x3d38_51ec);
    }
    g.write::<u32>(a + 0x13c, 0);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x45), 440), fr.ecx);
    s.push_all(&[w(0xde), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xd9), 450), fr.ecx);
    s.push_all(&[w(0xf1), a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0xec), 449), fr.ecx).eax)
}

/// [`bone_setup`] (272 bytes, 1 overlay; the `rel32` at `+0x108` is
/// `__security_check_cookie`).
pub const BONE_SETUP: Sig = Sig {
    len: 272,
    hash: 0xa25f_77e4_2c48_dc3d,
    calls: &[(0x108, crate::Callee::Any)],
    imports: &[],
};

/// An effect object at an offset from a bone (0x100013c0, cdecl `(a)`, a
/// /GS frame): slot 474 `((*G_E)[0x198], 0x25, &O, &P)` with `O = (0,
/// -23, 0, 0)` (locals `O` at `ebp - 0x14`, `P` at `ebp - 0x24`; `*G_E`
/// operand `+0x1d`) gives the point `P`; `a + 0xd0..0xdc` = `(P.x, P.y,
/// P.z, 0)`, `+0xe0..` `(0, -15, 0, 0)`, `+0x130..0x138` 0.045, `+0x13c`
/// 0; slot 440 `(*G_E, a, 0, 0)`, slot 450 `(a, F1)` and slot 449 `(a,
/// F2)` (the immediates `+0xf1`, `+0x104`), each over the earlier words.
/// Returns slot 449's `eax`.
pub fn offset_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let a = fr.arg(g, 0);
    let (o, p) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let mut s = Pushes::default();
    let x = g.read::<u32>(g.read::<u32>(w(0x1d)) + 0x198);
    s.push_all(&[p, o, 0x25, x]);
    for (i, v) in (0u32..).zip([0u32, 0xc1b8_0000, 0, 0]) {
        g.write::<u32>(o + 4 * i, v);
    }
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x2a), 474), fr.ecx);
    s.push_all(&[0, 0, a, g.read::<u32>(w(0x83))]);
    for k in 0..3u32 {
        let v = g.read::<u32>(p + 4 * k);
        g.write::<u32>(a + 0xd0 + 4 * k, v);
    }
    for (off, v) in [
        (0xdcu32, 0u32),
        (0xe0, 0),
        (0xe4, 0xc170_0000),
        (0xe8, 0),
        (0xec, 0),
    ] {
        g.write::<u32>(a + off, v);
    }
    for off in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(a + off, 0x3d38_51ec);
    }
    g.write::<u32>(a + 0x13c, 0);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x58), 440), fr.ecx);
    s.push_all(&[w(0xf1), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xec), 450), fr.ecx);
    s.push_all(&[w(0x104), a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0xff), 449), fr.ecx).eax)
}

/// [`offset_setup`] (291 bytes, 1 overlay; the `rel32` at `+0x11b` is
/// `__security_check_cookie`).
pub const OFFSET_SETUP: Sig = Sig {
    len: 291,
    hash: 0x1733_ce8a_4ead_7ba6,
    calls: &[(0x11b, crate::Callee::Any)],
    imports: &[],
};
