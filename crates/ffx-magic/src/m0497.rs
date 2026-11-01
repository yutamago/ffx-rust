//! `magic_0497.dll`'s own functions (besides the runtime and the shared
//! code): three lights and the start-up.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Slot 401 `(i, P, 20.0, 0xff, 0xff, 0xff, i)` for the lights `i` =
/// 0..2 at the point `P` (operands of `x`, `y`, `z`), each call's
/// table read through the operand `t`.
fn lights<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, [x, y, z]: [u32; 3]) {
    let w = |off: u32| img.word(f + off);
    for i in 0..3 {
        let p = [x, y, z].map(|op| g.read::<u32>(w(op)));
        let s = slot_target(g, w(t), 401);
        g.call(
            s,
            0,
            &[i, p[0], p[1], p[2], 0x41a0_0000, 0xff, 0xff, 0xff, i],
        );
    }
}

/// 0x10001000 (an overlay table entry, no arguments): the point
/// (-7.7, -86.9, 57.1, 1) into the globals at operands `+0x08..+0x26`,
/// slot 400 `(3)`, the three lights ([`lights`]), slot 398 `(*G_H, 7)`
/// and slot 399 `(*G_H, 1)` (398's words left on the stack). Returns 1.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 400);
    for (op, v) in [
        (0x8u32, 0xc0f6_6556u32),
        (0x12, 0xc2ad_ccc7),
        (0x1c, 0x4264_6672),
        (0x26, 0x3f80_0000),
    ] {
        g.write::<u32>(w(op), v);
    }
    g.call(t, 0, &[3]);
    lights(g, img, f, 0x49, [0x86, 0x78, 0x44]);
    let t = slot_target(g, w(0x9c), 398);
    let h = g.read::<u32>(w(0xa4));
    g.call(t, 0, &[h, 7]);
    let t = slot_target(g, w(0xb1), 399);
    let h2 = g.read::<u32>(w(0xb9));
    g.call(t, 0, &[h2, 1, h, 7]);
    Ret::eax(1)
}

/// 0x10001000 (207 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 207,
    hash: 0xf43f_102f_f8b6_dec1,
    calls: &[],
    imports: &[],
};

/// 0x100010d0 (an overlay table entry, no arguments): the three lights
/// ([`lights`]). Returns 1.
pub fn e10d0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    lights(g, img, fr.at, 0x19, [0x56, 0x48, 0x14]);
    Ret::eax(1)
}

/// 0x100010d0 (114 bytes, 1 overlay).
pub const E10D0: Sig = Sig {
    len: 114,
    hash: 0x567e_4e81_1bc3_26f5,
    calls: &[],
    imports: &[],
};

/// The effect's start-up (0x100012f0, an overlay table entry, `(arg)`):
/// slot 803 `(0x1000000, 0x10)` the buffer (`+0x19`, and `+0x23` half a
/// MiB on), `arg` to `+0x2b`; the name (`+0x35`) to `+0x41` and slot
/// 718, slot 222 `(name, 0, 1000, 0x2000)`, slot 719 `(name, 8, 0)` the
/// data `d` (`+0x8c`); slot 711 `(d, PARTS, 0)` with the game flag
/// `*G_LOCK` 1; slot 710 `(ITEM, d, buf, 0x80000)`; three slot 941
/// `(d[5 + k])` the program tables; the timeline (`+0x127`) cleared and
/// given its defaults; slot 260 `(7)` a handle (`+0x17a`). Every call
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
    for (op, v) in [(0x146u32, 2u32), (0x150, 2), (0x15a, 1), (0x164, 1)] {
        g.write::<u32>(w(op), v);
    }
    stack.insert(0, 7);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x17a), h);
    Ret::eax(1)
}

/// 0x100012f0 (390 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 390,
    hash: 0x2236_ee87_dad0_4c33,
    calls: &[(0x131, Callee::Import("memset"))],
    imports: &[],
};
