//! `magic_0193.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// An object's step (0x100014d0, cdecl `(obj)`): while the counter `*N`
/// (`+0x05`) is below 20, three slot 434 `(*A, F)` calls (`A` at
/// `+0x1e`.., the callback the immediate `+0x18`..) and the counter up;
/// then slot 450 `(obj, F2)` (`+0x66`) and slot 451 `(obj, *G - 0x23)`
/// (`+0x70`). Returns the last call's `eax`. Each slot call passes the
/// words the original leaves on the stack.
pub fn spawn_three<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    if g.read::<i32>(w(0x05)) < 0x14 {
        let mut stack: Vec<u32> = Vec::new();
        let mut eax = 0;
        for (table, a, cb) in [(0x0b, 0x1e, 0x18), (0x25, 0x30, 0x2a), (0x3d, 0x48, 0x42)] {
            let av = g.read::<u32>(w(a));
            stack.splice(0..0, [av, w(cb)]);
            let s = slot_target(g, w(table), 434);
            // call width: 6 (the earlier calls' words left on the stack)
            eax = g.call(s, 0, &stack).eax;
        }
        let n = g.read::<u32>(w(0x59));
        g.write::<u32>(w(0x59), n.wrapping_add(1));
        return Ret::eax(eax);
    }
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x0b), 450);
    g.call(s, 0, &[obj, w(0x66)]);
    let v = g.read::<u32>(w(0x70)).wrapping_sub(0x23);
    let s = slot_target(g, w(0x79), 451);
    Ret::eax(g.call(s, 0, &[obj, v, obj, w(0x66)]).eax)
}

/// [`spawn_three`] (141 bytes, 1 overlay).
pub const SPAWN_THREE: Sig = Sig {
    len: 141,
    hash: 0xca21_6937_1fbc_6159,
    calls: &[],
    imports: &[],
};
