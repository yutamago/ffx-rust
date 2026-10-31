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
