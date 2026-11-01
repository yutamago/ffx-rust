//! `magic_0304.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's rise (0x10001cd0, cdecl `(obj)`): from frame 0x54
/// (`+0x118`) on, slot 12 `(T[b])`'s float is dropped (`b` the byte
/// `**P`, `P` `+0x11`, `T` `+0x22`), the effect's speed `+0x1720` (`E`
/// = `*G`, `+0x1b`) grows by `K` (`+0x3e`) and the object's height
/// `+0xd4` by the speed. Returns slot 12's `eax` (the caller's before).
pub fn rise<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    if g.read::<i32>(obj + 0x118) < 0x54 {
        return Ret::eax(fr.eax);
    }
    let b = g.read::<u8>(g.read::<u32>(w(0x11)));
    let fx = g.read::<u32>(w(0x1b));
    let x = g.read::<u32>(w(0x22).wrapping_add(4 * u32::from(b)));
    let s = slot_target(g, w(0x27), 0x30 / 4);
    let eax = g.call(s, 0, &[x]).eax;
    let speed = add(g.read::<f32>(fx + 0x1720), g.read::<f32>(w(0x3e)));
    g.write::<f32>(fx + 0x1720, speed);
    let h = add(g.read::<f32>(obj + 0xd4), speed);
    g.write::<f32>(obj + 0xd4, h);
    Ret::eax(eax)
}

/// [`rise`] (101 bytes, 1 overlay).
pub const RISE: Sig = Sig {
    len: 101,
    hash: 0xf2d3_7558_0eff_428f,
    calls: &[],
    imports: &[],
};

/// A battle-mode drift (0x10002150, no arguments): slot 520 `()`; for
/// the mode 0x018b0046 the effect's (`*G`, `+0x15`) `+0x334` grows by `B`
/// (`+0x33`) and `+0x204` by `A` (`+0x2b`), else both by 0. Returns slot
/// 520's `eax`.
pub fn drift<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = slot_target(g, w(0x05), 0x820 / 4);
    let fx = g.read::<u32>(w(0x15));
    let mode = g.call(s, 0, &[]).eax;
    let (a, b) = if mode == 0x018b_0046 {
        (g.read::<f32>(w(0x2b)), g.read::<f32>(w(0x33)))
    } else {
        (0.0, 0.0)
    };
    let v = add(g.read::<f32>(fx + 0x334), b);
    g.write::<f32>(fx + 0x334, v);
    let v = add(g.read::<f32>(fx + 0x204), a);
    g.write::<f32>(fx + 0x204, v);
    Ret::eax(mode)
}

/// [`drift`] (111 bytes, 1 overlay).
pub const DRIFT: Sig = Sig {
    len: 111,
    hash: 0x0cb6_c3f2_3148_1a6b,
    calls: &[],
    imports: &[],
};

/// The eased height (0x100021c0, no arguments, no frame): with `E` =
/// `*G` (`+0x03`), the ease of the key table `+0x17` (5 keys, stride 1;
/// [`ease_sample`](crate::sb_misc::ease_sample)) at the time `+0x1724`,
/// which then moves on by `K` (`+0x27`); with `h` = `+0x149c`: `+0x464` =
/// `h + A + e`, `+0x6c4` = `h + A' + e` (`A`, `A'` `+0x45`, `+0x61`),
/// `+0x146c` and `+0x140c` = `h + e`, `+0x148c` = `h + B + e` (`B`
/// `+0x80`). Returns 0 (the ease's scratch `eax` in the original, which
/// no caller reads).
pub fn eased_height<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let fx = g.read::<u32>(w(0x03));
    let t = g.read::<f32>(fx + 0x1724);
    // push esi; push 1 (the stride): the call's return address below
    let stride = fr.esp.wrapping_sub(8);
    g.write::<u32>(stride, 1);
    let e = crate::sb_misc::ease_sample(
        g,
        img,
        img.rel32_target(fr.at + 0x1f),
        fr.esp.wrapping_sub(0xc),
        w(0x17),
        5,
        t,
    );
    let t = add(t, g.read::<f32>(w(0x27)));
    g.write::<f32>(fx + 0x1724, t);
    let h = |g: &G| g.read::<f32>(fx + 0x149c);
    let v = add(add(h(g), g.read::<f32>(w(0x45))), e);
    g.write::<f32>(fx + 0x464, v);
    let v = add(add(h(g), g.read::<f32>(w(0x61))), e);
    g.write::<f32>(fx + 0x6c4, v);
    let x = h(g);
    let hb = add(add(x, g.read::<f32>(w(0x80))), e);
    let he = add(x, e);
    g.write::<f32>(fx + 0x146c, he);
    g.write::<f32>(fx + 0x148c, hb);
    g.write::<f32>(fx + 0x140c, he);
    Ret::eax(0)
}

/// [`eased_height`] (166 bytes, 1 overlay).
pub const EASED_HEIGHT: Sig = Sig {
    len: 166,
    hash: 0x31fe_c1d4_22a7_1c3c,
    calls: &[(0x1f, Callee::Sig(&crate::sb_misc::EASE_SAMPLE))],
    imports: &[],
};

/// Places the block's point (0x10002e20, a table entry, no arguments; a
/// /GS frame; the block `S = *G_S`, operand `+0x13`): slot 235 `(&M)`,
/// slot 329 `(&M, &M, S[0x145c])` and slot 229 `(&P, &M, &P)` with `P =
/// (-10, 0, 0, 1)` (`M`, `P` locals at `ebp - 0x54`, `- 0x14`; each call
/// over the earlier words); `S + 0x1408` = `S[0x1468] + P.x`, `S +
/// 0x146c` and `+0x140c` = `S[0x149c] - K` (`+0xa5`). Returns slot 229's
/// `eax`.
pub fn place_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let s = g.read::<u32>(w(0x13));
    let (m, p) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let mut st = crate::pushes::Pushes::default();
    st.push(m);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x1c), 235), fr.ecx);
    st.set_top(g.read::<u32>(s + 0x145c));
    st.push_all(&[m, m]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x3b), 329), fr.ecx);
    st.push_all(&[p, m, p]);
    for (i, v) in (0u32..).zip([0xc120_0000u32, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(p + 4 * i, v);
    }
    // call width: 12 (Pushes caps the words at 12)
    let eax = st.call(g, slot_target(g, w(0x54), 229), fr.ecx).eax;
    let x = add(g.read::<f32>(s + 0x1468), g.read::<f32>(p));
    g.write::<f32>(s + 0x1408, x);
    let y = crate::sb_sse::sub(g.read::<f32>(s + 0x149c), g.read::<f32>(w(0xa5)));
    g.write::<f32>(s + 0x146c, y);
    g.write::<f32>(s + 0x140c, y);
    Ret::eax(eax)
}

/// [`place_point`] (195 bytes, 1 overlay; the `rel32` at `+0xbb` is
/// `__security_check_cookie`).
pub const PLACE_POINT: Sig = Sig {
    len: 195,
    hash: 0x3339_cb4d_c6b4_f88e,
    calls: &[(0xbb, Callee::Any)],
    imports: &[],
};

/// Starts the block's host object (0x10002f30, a table entry, no
/// arguments; the block `S = *G_S`, operand `+0x0b`): the object `O = S +
/// 0x720` ([`start_host_object`](crate::s4_small::start_host_object), `ecx`
/// the immediate `+0x20`, scale the float at `+0x04`, index 1, motion 0);
/// `O + 0xd0..0xdc` = `S + 0x1488..`, `O + 0xc4` = `S + 0x171c`, and `O +
/// 0xd4` plus the float at `+0x65`. Returns the word copied to `+0xc4`.
pub fn host_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0xb));
    let o = s + 0x720;
    // push esi; push edi; sub esp, 8; push 0; push 1; the call
    let esp = fr.esp.wrapping_sub(0x1c);
    g.write::<u32>(esp + 4, 1);
    g.write::<u32>(esp + 8, 0);
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(g.read::<u32>(w(0x4)));
    let start = Frame {
        at: img.rel32_target(f + 0x25),
        esp,
        ecx: w(0x20),
        edx: o,
        xmm,
        ..*fr
    };
    crate::s4_small::start_host_object(g, img, &start);
    for k in 0..4u32 {
        let v = g.read::<u32>(s + 0x1488 + 4 * k);
        g.write::<u32>(o + 0xd0 + 4 * k, v);
    }
    let y = add(g.read::<f32>(o + 0xd4), g.read::<f32>(w(0x65)));
    let v = g.read::<u32>(s + 0x171c);
    g.write::<u32>(o + 0xc4, v);
    g.write::<f32>(o + 0xd4, y);
    Ret::eax(v)
}

/// [`host_start`] (131 bytes, 1 overlay).
pub const HOST_START: Sig = Sig {
    len: 131,
    hash: 0x7211_817f_1689_202f,
    calls: &[(0x25, Callee::Sig(&crate::s4_small::START_HOST_OBJECT))],
    imports: &[],
};
