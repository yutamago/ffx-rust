//! `magic_0191.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// An object's set-up (0x100011b0, cdecl `(obj)`): slot 434 `(*A, F1)`
/// and `(*A, F2)` (`A` at `+0x16`/`+0x28`, the callbacks the immediates
/// `+0x10`, `+0x22`); the second one's current list word (`+0x2c + 16 *
/// [+0xac]`) gets `*C` (`+0x3c`); slots 450 `(obj, F3)` and 449 `(obj,
/// F4)` (`+0x46`, `+0x62`); the object's current list word gets
/// `(*A)[0x198]` (`+0x76`). Returns that word. Each slot call passes the
/// words the original leaves on the stack.
pub fn object_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let a = g.read::<u32>(w(0x16));
    let s = slot_target(g, w(0x04), 434);
    g.call(s, 0, &[a, w(0x10)]);
    let a2 = g.read::<u32>(w(0x28));
    let s = slot_target(g, w(0x1d), 434);
    let o = g.call(s, 0, &[a2, w(0x22), a, w(0x10)]).eax;
    let i = g.read::<u32>(o + 0xac);
    let c = g.read::<u32>(w(0x3c));
    g.write::<u32>(o.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c), c);
    let s = slot_target(g, w(0x4f), 450);
    g.call(s, 0, &[obj, w(0x46), a2, w(0x22), a, w(0x10)]);
    let s = slot_target(g, w(0x5d), 449);
    // call width: 8 (the earlier calls' words left on the stack)
    g.call(s, 0, &[obj, w(0x62), obj, w(0x46), a2, w(0x22), a, w(0x10)]);
    let i = g.read::<u32>(obj + 0xac);
    let v = g.read::<u32>(g.read::<u32>(w(0x76)) + 0x198);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c), v);
    Ret::eax(v)
}

/// [`object_setup`] (140 bytes, 1 overlay).
pub const OBJECT_SETUP: Sig = Sig {
    len: 140,
    hash: 0x45f0_2a77_2fb3_fbec,
    calls: &[],
    imports: &[],
};

/// 0x10001240 (cdecl `(obj)`): the counter `*C` (`+0x04`) goes up; below
/// the effect's count `e[0x19c]` (`e = *E`, `+0x0a`) slot 447 `(h, 0xbb)`
/// (`h = e[0x198]`) and slot 461 `(obj, C0)` (`+0x93`), returning its
/// `eax`. Else slot 447 `(h, 0xbc)`, slot 459 `(h, 2, 4)`, slot 450
/// `(obj, C1)` (`+0x58`), slot 449 `(obj, C2)` (`+0x65`) and `h` into
/// the object's record `obj + 0x2c + 16 k` (`k = obj[0xac]`); returns
/// `2 k`. Each slot call passes the words the original leaves on the
/// stack (not the saved `esi` the second path pushes first).
pub fn chain_next<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let n = g.read::<u32>(w(0x04)).wrapping_add(1);
    let e = g.read::<u32>(w(0x0a));
    g.write::<u32>(w(0x16), n);
    let h = g.read::<u32>(e + 0x198);
    let s447 = slot_target(g, w(0x1b), 447);
    if n.cast_signed() < g.read::<i32>(e + 0x19c) {
        g.call(s447, 0, &[h, 0xbb]);
        let s = slot_target(g, w(0x8e), 461);
        // call width: 4
        return Ret::eax(g.call(s, 0, &[obj, w(0x93), h, 0xbb]).eax);
    }
    g.call(s447, 0, &[h, 0xbc]);
    let s = slot_target(g, w(0x38), 459);
    // call width: 5
    g.call(s, 0, &[h, 2, 4, h, 0xbc]);
    let s = slot_target(g, w(0x4a), 450);
    // call width: 7
    g.call(s, 0, &[obj, w(0x58), h, 2, 4, h, 0xbc]);
    let s = slot_target(g, w(0x60), 449);
    // call width: 9
    g.call(s, 0, &[obj, w(0x65), obj, w(0x58), h, 2, 4, h, 0xbc]);
    let k2 = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(k2.wrapping_mul(8)).wrapping_add(0x2c), h);
    Ret::eax(k2)
}

/// [`chain_next`] (168 bytes, 1 overlay).
pub const CHAIN_NEXT: Sig = Sig {
    len: 168,
    hash: 0x7322_c8c7_b71c_2608,
    calls: &[],
    imports: &[],
};

/// 0x10001350 (cdecl `(obj)`): slot 441 `(obj, h, 0xf)` (`h` the word
/// `+0x198` of the effect `*E`, `+0x04`), slot 443 `(obj, 0x10)`; the
/// object's vectors `+0xd0` = `(0, 1, -2.5, 0)`, `+0x100` = 0 and
/// `+0x130` = `(0.55, 0.55, 0.55, 0)`; slot 440 `(*E, obj, 0, 0)` (`E`
/// again at `+0x3e`), slot 450 `(obj, C1)` (`+0xc8`) and slot 449 `(obj,
/// C2)` (`+0xdb`). Returns slot 449's `eax`. Each slot call passes the
/// words the original leaves on the stack.
pub fn chain_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let h = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x198);
    let s = slot_target(g, w(0x15), 441);
    g.call(s, 0, &[obj, h, 0xf]);
    let s = slot_target(g, w(0x23), 443);
    // call width: 5
    g.call(s, 0, &[obj, 0x10, obj, h, 0xf]);
    let e = g.read::<u32>(w(0x3e));
    for (off, v) in [
        (0xd0, 0),
        (0xd4, 0x3f80_0000),
        (0xd8, 0xc020_0000),
        (0xdc, 0),
        (0x100, 0),
        (0x104, 0),
        (0x108, 0),
        (0x10c, 0),
        (0x130, 0x3f0c_cccd),
        (0x134, 0x3f0c_cccd),
        (0x138, 0x3f0c_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    let mut stack = vec![e, obj, 0, 0, obj, 0x10, obj, h, 0xf];
    let s = slot_target(g, w(0x33), 440);
    // call width: 9
    g.call(s, 0, &stack);
    for x in [w(0xc8), obj] {
        stack.insert(0, x);
    }
    let s = slot_target(g, w(0xc3), 450);
    // call width: 11
    g.call(s, 0, &stack);
    for x in [w(0xdb), obj] {
        stack.insert(0, x);
    }
    let s = slot_target(g, w(0xd6), 449);
    // call width: 13
    Ret::eax(g.call(s, 0, &stack).eax)
}

/// [`chain_setup`] (238 bytes, 1 overlay).
pub const CHAIN_SETUP: Sig = Sig {
    len: 238,
    hash: 0x6315_ff43_a778_289a,
    calls: &[],
    imports: &[],
};
