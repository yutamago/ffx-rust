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
