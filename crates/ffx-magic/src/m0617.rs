//! `magic_0617.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's frame (0x100010a0, `(obj)`; three overlay table
/// entries): slot 258 `(obj)`, slot 231 with the globals at `+0x2f`,
/// `+0x23`, `+0x18` and slot 230 with `+0x42`, `+0x37` (the earlier
/// words left on the stack); `*G_P = ITEM` (`+0x4f`, `+0x57`), slot 712
/// `(obj, 0x30)` with the game flag `*G_LOCK` (`+0x5c`) 1 and restored
/// after (`+0x78`). When 712 says nonzero, `*G_P = ITEM` again and slot
/// 713 `(obj)` removes it, and the part starts again with program 0
/// ([`start_part`]). Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 258);
    let mut stack = vec![obj];
    g.call(t, 0, &stack);
    for op in [0x18, 0x23, 0x2f] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x1d), 231);
    // call width: 4 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for op in [0x37, 0x42] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x3c), 230);
    // call width: 6 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x4f));
    g.write::<u32>(gp, w(0x57));
    let lock = g.read::<u32>(w(0x5c));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x6a), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 8 (the earlier calls' words stay on the stack)
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x78));
    g.write::<u32>(lock, saved);
    if r == 0 {
        return Ret::eax(1);
    }
    let gp = g.read::<u32>(w(0x86));
    g.write::<u32>(gp, w(0x8d));
    let t = slot_target(g, w(0x92), 713);
    g.call(t, 0, &[obj]);
    start_part(g, img, f, obj, [0x9f, 0xa5, 0xaa, 0xb2, 0xcc], 0, &[obj]);
    Ret::eax(1)
}

/// [`frame`] (220 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 220,
    hash: 0x6aa2_1ab7_cf6e_a6fc,
    calls: &[(0xcc, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
