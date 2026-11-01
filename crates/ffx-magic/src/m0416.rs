//! `magic_0416.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The copies' step (0x10001570, cdecl `(obj)`): below 20 copies (`*C`,
/// `+0x05`) slot 434 `(*A, F)` three times and the count goes up; from
/// 20 on slot 434 `(*A, F2)` (`+0x60`), slot 450 `(obj, F3)` (`+0x72`)
/// and slot 451 `(obj, *T - 0x23)` (`T` `+0x82`). Returns the last
/// slot's `eax`. Each slot call passes the words the original leaves on
/// the stack.
pub fn copies_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = crate::pushes::Pushes::default();
    let s434 = slot_target(g, w(0x0b), 0x6c8 / 4);
    if g.read::<i32>(w(0x05)) < 0x14 {
        let mut eax = 0;
        for (cb, list, table) in [(0x18, 0x1e, 0), (0x2a, 0x30, 0x25), (0x42, 0x48, 0x3d)] {
            p.push_all(&[w(cb), g.read::<u32>(w(list))]);
            let s = if table == 0 {
                s434
            } else {
                slot_target(g, w(table), 0x6c8 / 4)
            };
            // call width: 12 (Pushes caps the words at 12)
            eax = p.call(g, s, 0).eax;
        }
        let c = g.read::<u32>(w(0x59));
        g.write::<u32>(w(0x59), c.wrapping_add(1));
        return Ret::eax(eax);
    }
    p.push_all(&[w(0x60), g.read::<u32>(w(0x66))]);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s434, 0);
    let obj = fr.arg(g, 0);
    p.push_all(&[w(0x72), obj]);
    let s = slot_target(g, w(0x6d), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let t = g.read::<u32>(w(0x82)).wrapping_sub(0x23);
    p.push_all(&[t, obj]);
    let s = slot_target(g, w(0x8b), 0x70c / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`copies_step`] (159 bytes, 1 overlay).
pub const COPIES_STEP: Sig = Sig {
    len: 159,
    hash: 0xc3e0_af60_2989_5f29,
    calls: &[],
    imports: &[],
};
