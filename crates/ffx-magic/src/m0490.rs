//! `magic_0490.dll`'s own functions (besides the runtime and the shared
//! code): the start-up (its frame is `s4_misc::frame_step`).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start-up (0x10001130, an overlay table entry, `(arg)`):
/// slot 803 `(0x1000000, 0x10)` the buffer (`+0x19`, and `+0x23` half a
/// MiB on), `arg` to `+0x2b`; the name (`+0x35`) to `+0x41` and slot
/// 718, slot 222 `(name, 0, 1000, 0x2000)`, slot 719 `(name, 8, 0)` the
/// data `d` (`+0x8c`); slot 711 `(d, PARTS, 0)` with the game flag
/// `*G_LOCK` 1; slot 710 `(ITEM, d, buf, 0x80000)`; three slot 941
/// `(d[5 + k])` the program tables; the timeline (`+0x127`) cleared and
/// given its defaults; slot 260 `(4)` a handle (`+0x170`). Every call
/// passes the words on the original's stack. Returns 1.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x4), 803);
    let mut stack = vec![0x0100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x19), buf);
    g.write::<u32>(w(0x23), buf.wrapping_add(0x8_0000));
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x2b), arg);
    let t = slot_target(g, w(0x30), 718);
    stack.insert(0, w(0x35));
    g.write::<u32>(w(0x41), w(0x45));
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x4c), 222);
    let name = g.read::<u32>(w(0x64));
    stack.splice(0..0, [name, 0, 0x3e8, 0x2000]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x6b), 719);
    let name = g.read::<u32>(w(0x7b));
    stack.splice(0..0, [name, 8, 0]);
    // call width: 17
    let d = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x83));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x8c), d);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x97), 711);
    stack.splice(0..0, [d, w(0x91), 0]);
    // call width: 17
    g.call(t, 0, &stack);
    let lock = g.read::<u32>(w(0xaa));
    let b = g.read::<u32>(w(0xb5));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xbd));
    let t = slot_target(g, w(0xc2), 710);
    stack.splice(0..0, [w(0xc7), d, b, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    // add esp, 0x44
    let mut stack = Vec::new();
    for (k, (dop, tab, dst)) in (0u32..).zip([
        (0xd4u32, 0xdfu32, 0xecu32),
        (0xf1, 0xf9, 0x106),
        (0x10b, 0x113, 0x12c),
    ]) {
        let d = g.read::<u32>(w(dop));
        stack.insert(0, g.read::<u32>(d.wrapping_add(0x14 + 4 * k)));
        let t = slot_target(g, w(tab), 941);
        // call width: 17
        let r = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    g.write_bytes(w(0x127), &[0u8; 0x110]);
    stack.splice(0..0, [w(0x127), 0, 0x110]);
    let t = slot_target(g, w(0x136), 260);
    g.write::<u32>(w(0x13c), w(0x140));
    for op in [0x146, 0x150, 0x15a] {
        g.write::<u32>(w(op), 1);
    }
    stack.insert(0, 4);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x170), h);
    Ret::eax(1)
}

/// 0x10001130 (380 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 380,
    hash: 0xc41e_f992_f950_b6ab,
    calls: &[(0x131, Callee::Import("memset"))],
    imports: &[],
};
