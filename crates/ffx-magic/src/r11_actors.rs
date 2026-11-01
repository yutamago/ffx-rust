//! Round 11 (magic-r11-own): an object set up between two actors
//! (`magic_0356`, `magic_0357`).

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::sb_sse::{cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// How the last `rand` picks the callback: `r % 6 < 3` in `magic_0356`,
/// `r % 2 != 0` in `magic_0357` (the code after it a byte later).
#[derive(Clone, Copy)]
enum Pick {
    Sixths,
    Halves,
}

/// An object's set-up between two actors (cdecl `(obj)`, a /GS frame;
/// `magic_0356` 0x10001260, 750 bytes; `magic_0357` 0x10001320, 751):
/// the actors `A = *(G + 0x198)`, `B = *(G + 0x1a0)` (`G` = `*G_P`,
/// operand `+0x1d`); two `rand`s as fractions `u = r / K0`, `v = r / K0`
/// (`+0x40`, `+0x63`); slot 463 `(A, 0, &MA)` and `(B, 0, &MB)` (two
/// matrices on the frame at `ebp - 0xa4`, `ebp - 0x64`), slots 488 and 489
/// `(A)`, `(B)` (floats `a1, b1, a2, b2`). The points `PA = (a2 u K1 -
/// a2, -(a1 v), a2 K2, 1)` and `PB = (b2 u K1 - b2, -(b1 v), -b2, 1)`
/// (the sign flips are `xorps` with `*S`, `+0xf0`; `K1` `+0x107`/`+0x164`,
/// `K2` `+0x121`) go through slot 229 `(&P, &M, &P)` with their actor's
/// matrix; then `obj + 0xbc` |= 0x2000, `obj + 0xd0..` = `PA` (w 0),
/// `obj + 0xe0..` = `(PB - PA) K3` (`+0x1ee`; w 0), `obj + 0x130..0x138` 0.01
/// and `+0x13c` 0; slot 490 `(*G_P, obj, 1)`; byte `obj + 0x3d8` = 0x42;
/// a third `rand` picks slot 450 `(obj, CB1)` and slot 451 `(obj, 15)`,
/// or `(obj, CB2)` and `(obj, 30)` (`+0x2b8`, `+0x2c4`). Returns slot
/// 451's `eax`.
#[allow(clippy::too_many_lines)]
fn between_actors<G: Game>(g: &mut G, img: &Image, fr: &Frame, pick: Pick) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // the frame: ebp = the entry's esp - 4
    let ebp = fr.esp.wrapping_sub(4);
    let (ma, mb, pa, pb) = (ebp - 0xa4, ebp - 0x64, ebp - 0x24, ebp - 0x14);
    let obj = fr.arg(g, 0);
    let gp = g.read::<u32>(w(0x1d));
    let a = g.read::<u32>(gp.wrapping_add(0x198));
    let b = g.read::<u32>(gp.wrapping_add(0x1a0));
    let u = div(cvt(g.rand()), k(g, 0x40));
    let v = div(cvt(g.rand()), k(g, 0x63));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[ma, 0, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x69), 0x73c / 4), 0);
    s.push_all(&[mb, 0, b]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x85), 0x73c / 4), 0);
    let float = |g: &mut G, s: &mut crate::pushes::Pushes, table: u32, slot: u32, actor: u32| {
        s.push(actor);
        // call width: 12 (Pushes caps the words at 12)
        st0_f32(s.call(g, slot_target(g, w(table), slot), 0).st0)
    };
    let a1 = float(g, &mut s, 0x95, 0x7a0 / 4, a);
    let b1 = float(g, &mut s, 0xa9, 0x7a0 / 4, b);
    let a2 = float(g, &mut s, 0xbd, 0x7a4 / 4, a);
    let b2 = float(g, &mut s, 0xd1, 0x7a4 / 4, b);
    let sign = g.read::<u32>(w(0xf0));
    let flip = |x: f32| f32::from_bits(x.to_bits() ^ sign);
    s.push(pa);
    let near_x = sub(mul(mul(a2, u), k(g, 0x107)), a2);
    g.write::<u32>(ebp - 0x18, 0x3f80_0000);
    g.write::<u32>(ebp - 8, 0x3f80_0000);
    let near_z = mul(a2, k(g, 0x121));
    g.write::<f32>(pa, near_x);
    let near_y = flip(mul(a1, v));
    g.write::<f32>(pa + 8, near_z);
    g.write::<f32>(pa + 4, near_y);
    let far_x = sub(mul(mul(b2, u), k(g, 0x164)), b2);
    g.write::<f32>(pb + 8, flip(b2));
    g.write::<f32>(pb, far_x);
    g.write::<f32>(pb + 4, flip(mul(b1, v)));
    s.push_all(&[ma, pa]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x197), 0x394 / 4), 0);
    s.push_all(&[pb, mb, pb]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1b0), 0x394 / 4), 0);
    let from = [pa, pa + 4, pa + 8].map(|at| g.read::<f32>(at));
    let to = [pb, pb + 4, pb + 8].map(|at| g.read::<f32>(at));
    let flags = g.read::<u32>(obj + 0xbc);
    g.write::<u32>(obj + 0xbc, flags | 0x2000);
    let k3 = k(g, 0x1ee);
    let step: [f32; 3] = core::array::from_fn(|i| mul(sub(to[i], from[i]), k3));
    g.write::<f32>(obj + 0xd0, from[0]);
    let gv = g.read::<u32>(w(0x21f));
    g.write::<f32>(obj + 0xd4, from[1]);
    g.write::<f32>(obj + 0xd8, from[2]);
    g.write::<u32>(obj + 0xdc, 0);
    for (off, v) in [0xe0u32, 0xe4, 0xe8].into_iter().zip(step) {
        g.write::<f32>(obj + off, v);
    }
    g.write::<u32>(obj + 0xec, 0);
    for off in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(obj + off, 0x3c23_d70a);
    }
    g.write::<u32>(obj + 0x13c, 0);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[1, obj, gv]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1f3), 0x7a8 / 4), 0);
    s.pop(3);
    g.write::<u8>(obj + 0x3d8, 0x42);
    let r = g.rand().cast_signed();
    // the table's operand 4 bytes later in magic_0357, the others 1
    let (first, table, e) = match pick {
        Pick::Sixths => (r % 6 < 3, 0x2a8, 0),
        Pick::Halves => (r % 2 != 0, 0x2ac, 1),
    };
    let t450 = slot_target(g, w(table), 0x708 / 4);
    let (cb, n) = if first {
        (w(0x2b8 + e), 0xf)
    } else {
        (w(0x2c4 + e), 0x1e)
    };
    s.push_all(&[cb, obj]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t450, 0);
    s.push_all(&[n, obj]);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot_target(g, w(0x2ce + e), 0x70c / 4), 0);
    // __security_check_cookie leaves `eax`
    Ret::eax(out.eax)
}

/// [`between_actors`] in `magic_0356`.
pub fn between_actors_0356<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    between_actors(g, img, fr, Pick::Sixths)
}

/// [`between_actors`] in `magic_0357`.
pub fn between_actors_0357<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    between_actors(g, img, fr, Pick::Halves)
}

/// [`between_actors_0356`] (0x10001260).
pub const BETWEEN_ACTORS_0356: Sig = Sig {
    len: 750,
    hash: 0x96d9_3409_50bb_b157,
    calls: &[
        // __security_check_cookie
        (0x2e6, Callee::Any),
    ],
    imports: &[(0x31, "rand"), (0x4e, "rand"), (0x29b, "rand")],
};

/// [`between_actors_0357`] (`magic_0357` 0x10001320).
pub const BETWEEN_ACTORS_0357: Sig = Sig {
    len: 751,
    hash: 0xc8e1_c64a_ffd6_6073,
    calls: &[
        // __security_check_cookie
        (0x2e7, Callee::Any),
    ],
    imports: &[(0x31, "rand"), (0x4e, "rand"), (0x29b, "rand")],
};
