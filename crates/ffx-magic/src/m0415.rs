//! `magic_0415.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The turns' step (0x100011a0, cdecl `(obj)`): with `m` = `C[0x198]`
/// and `n` = `C[0x19c]` (`C` = `*A`, `+0x05`), the counter `*K`
/// (`+0x0a`) goes up. Below `n`: slots 450 `(obj, F)` (`+0xa0`) and 451
/// `(obj, 40 / (n - 1))` (a signed division; `n` = 1 faults in the
/// original, 0 here). From `n` on: slots 447 `(m, 0xc2)` and 459 `(m, 2,
/// 4)`, slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0x5a`, `+0x67`), the
/// list word gets `m`. Returns slot 451's `eax`, else twice the list
/// index. Each slot call passes the words the original leaves on the
/// stack.
pub fn turns_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let c = g.read::<u32>(w(0x05));
    let m = g.read::<u32>(c + 0x198);
    let n = g.read::<i32>(c + 0x19c);
    let k = g.read::<i32>(w(0x0a)).wrapping_add(1);
    g.write::<i32>(w(0x1e), k);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    if k < n {
        let d = 40i32.checked_div(n.wrapping_sub(1)).unwrap_or(0);
        p.push_all(&[w(0xa0), obj]);
        let s = slot_target(g, w(0x92), 0x708 / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
        p.push_all(&[d.cast_unsigned(), obj]);
        let s = slot_target(g, w(0xaa), 0x70c / 4);
        // call width: 12 (Pushes caps the words at 12)
        return Ret::eax(p.call(g, s, 0).eax);
    }
    p.push_all(&[0xc2, m]);
    let s = slot_target(g, w(0x27), 0x6fc / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[4, 2, m]);
    let s = slot_target(g, w(0x3a), 0x72c / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x5a), obj]);
    let s = slot_target(g, w(0x4c), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x67), obj]);
    let s = slot_target(g, w(0x62), 0x704 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), m);
    Ret::eax(i)
}

/// [`turns_step`] (191 bytes, 1 overlay).
pub const TURNS_STEP: Sig = Sig {
    len: 191,
    hash: 0xdb06_6df6_2c2a_1e86,
    calls: &[],
    imports: &[],
};

/// An effect object on a model (0x10001b90, cdecl `(a)`): slot 444 `(1)`;
/// with `m = (*G_E)[0x1a0 + 4 a[0x2c + 16 a[0xac]]]` (`*G_E` operand
/// `+0x26`): slot 446 `(m, 0)`, slot 442 `(a, m, 6)`, slot 443 `(a,
/// 0x10)`; `a + 0xd0..0xdc` `(0, 0, -4, 0)`, `+0x130..0x138` 0.3, `+0x13c`
/// 0; slot 440 `(*G_E, a, 0, 2)` (`+0x6c`), slot 450 `(a, F1)` and slot
/// 449 `(a, F2)` (the immediates `+0xce`, `+0xe1`), each over the earlier
/// words. Returns slot 449's `eax`.
pub fn model_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let mut s = Pushes::default();
    s.push(1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x4), 444), fr.ecx);
    let a = fr.arg(g, 0);
    let i = g.read::<u32>(a + 0xac);
    let c = g.read::<u32>(a.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c));
    let m = g.read::<u32>(
        g.read::<u32>(w(0x26))
            .wrapping_add(c.wrapping_mul(4))
            .wrapping_add(0x1a0),
    );
    s.push_all(&[0, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x32), 446), fr.ecx);
    s.push_all(&[6, m, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x40), 442), fr.ecx);
    s.push_all(&[0x10, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x51), 443), fr.ecx);
    s.push_all(&[2, 0, a, g.read::<u32>(w(0x6c))]);
    for (o, v) in [(0xd0u32, 0u32), (0xd4, 0), (0xd8, 0xc080_0000), (0xdc, 0)] {
        g.write::<u32>(a + o, v);
    }
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(a + o, 0x3e99_999a);
    }
    g.write::<u32>(a + 0x13c, 0);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x61), 440), fr.ecx);
    s.push_all(&[w(0xce), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xc9), 450), fr.ecx);
    s.push_all(&[w(0xe1), a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0xdc), 449), fr.ecx).eax)
}

/// [`model_setup`] (245 bytes, 1 overlay).
pub const MODEL_SETUP: Sig = Sig {
    len: 245,
    hash: 0x1db6_cebc_5cd3_6ff9,
    calls: &[],
    imports: &[],
};
