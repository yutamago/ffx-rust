//! `magic_0488.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Draws a part with a matrix (0x10001350, internal: `ecx` = the part,
/// `edx` = a matrix): slot 258 `(part)`, slot 231 `(*G, matrix, *G)`,
/// then slot 231 twice more with the camera globals, slot 712 `(part,
/// 0x30)` with the game flag `*G_LOCK` 1 (each with the earlier words on
/// the stack); the flag ends 0. Returns 1 when slot 712 says 0, else 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, f: u32, part: u32, matrix: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 258);
    let mut stack = vec![part];
    g.call(t, 0, &stack);
    let a = g.read::<u32>(w(0x15));
    stack.splice(0..0, [a, matrix, a]);
    let t = slot_target(g, w(0x1d), 231);
    g.call(t, 0, &stack);
    for (ops, tab) in [([0x2bu32, 0x36, 0x42], 0x30u32), ([0x4a, 0x55, 0x61], 0x4f)] {
        for op in ops {
            stack.insert(0, g.read::<u32>(w(op)));
        }
        let t = slot_target(g, w(tab), 231);
        // call width: 12
        g.call(t, 0, &stack);
    }
    let lock = g.read::<u32>(w(0x69));
    let gp = g.read::<u32>(w(0x6e));
    g.write::<u32>(lock, 1);
    g.write::<u32>(gp, w(0x7a));
    let t = slot_target(g, w(0x7f), 712);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [part, 0x30]);
    // call width: 12
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x98));
    g.write::<u32>(lock, saved);
    g.write::<u32>(lock, 0);
    u32::from(r == 0)
}

/// [`draw`] (182 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 182,
    hash: 0xb241_9610_1fd8_cd51,
    calls: &[],
    imports: &[],
};

/// The effect's start-up (0x100014e0, an overlay table entry, `(arg)`):
/// `arg` to `+0x07`; slot 803 `(0x1000000, 0x10)` the buffer, cut into
/// four parts (globals `+0x21`, `+0x2b`, `+0x35`, `+0x3f` and `+0x44`);
/// the name (`+0x4e`) to `+0x5a` and slot 718, slot 222 `(name, 0, 1000,
/// 0x2000)`, slot 719 `(name, 8, 0)` the data `d` (`+0xa5`); slot 711
/// `(d, PARTS, 0)` with the game flag `*G_LOCK` 1; slot 710 `(ITEM, d,
/// buf, 0x80000)`; three slot 941 `(d[5 + k])` the program tables; the
/// timeline block (`+0x140`) cleared and given its defaults; slot 260
/// `(0x14 + i)` five handles into the table at `+0x1b7`. Every call
/// passes the words on the original's stack. Returns 1.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x7), arg);
    let t = slot_target(g, w(0xc), 803);
    let mut stack = vec![0x0100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x21), buf);
    g.write::<u32>(w(0x2b), buf.wrapping_add(0x8_0000));
    g.write::<u32>(w(0x35), buf.wrapping_add(0x10_0000));
    g.write::<u32>(w(0x3f), buf.wrapping_add(0x18_0000));
    g.write::<u32>(w(0x44), buf.wrapping_add(0x18_0000));
    let t = slot_target(g, w(0x49), 718);
    stack.insert(0, w(0x4e));
    g.write::<u32>(w(0x5a), w(0x5e));
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x65), 222);
    let name = g.read::<u32>(w(0x7d));
    stack.splice(0..0, [name, 0, 0x3e8, 0x2000]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x84), 719);
    let name = g.read::<u32>(w(0x94));
    stack.splice(0..0, [name, 8, 0]);
    // call width: 17
    let d = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x9c));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xa5), d);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0xb0), 711);
    stack.splice(0..0, [d, w(0xaa), 0]);
    // call width: 17
    g.call(t, 0, &stack);
    let lock = g.read::<u32>(w(0xc3));
    let b = g.read::<u32>(w(0xce));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xd6));
    let t = slot_target(g, w(0xdb), 710);
    stack.splice(0..0, [w(0xe0), d, b, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    // add esp, 0x44
    let mut stack = Vec::new();
    for (k, (dop, tab, dst)) in (0u32..).zip([
        (0xedu32, 0xf8u32, 0x105u32),
        (0x10a, 0x112, 0x11f),
        (0x124, 0x12c, 0x145),
    ]) {
        let d = g.read::<u32>(w(dop));
        stack.insert(0, g.read::<u32>(d.wrapping_add(0x14 + 4 * k)));
        let t = slot_target(g, w(tab), 941);
        // call width: 17
        let r = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    g.write_bytes(w(0x140), &[0u8; 0x110]);
    // add esp, 0x18
    g.write::<u32>(w(0x153), w(0x157));
    for (op, v) in [
        (0x15du32, 5u32),
        (0x167, 5),
        (0x171, 1),
        (0x17b, 1),
        (0x185, 1),
        (0x18f, 1),
        (0x199, 1),
    ] {
        g.write::<u32>(w(op), v);
    }
    for i in 0..5u32 {
        let t = slot_target(g, w(0x1a8), 260);
        let h = g.call(t, 0, &[0x14 + i]).eax;
        g.write::<u32>(w(0x1b7) + 4 * i, h);
    }
    Ret::eax(1)
}

/// [`start_up`] (460 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 460,
    hash: 0x8018_f80d_1368_75a7,
    calls: &[(0x14a, crate::Callee::Import("memset"))],
    imports: &[],
};

/// Starts the effect's state (0x10001170, an overlay table entry, no
/// arguments): slot 719 `(*G_NAME, 4, 0)` gives the data (`+0x21`); the
/// 0x30-byte block `B` (`+0x1c`) zeroed (`memset`), its half-word `+2`
/// 0x80 and `+4` 0x80808080; the records cleared
/// ([`records_clear`](crate::s4_small::records_clear), slot 719's first
/// word under it), three globals (`+0x4c`, `+0x56`, `+0x60`) 0. Returns
/// 1.
pub fn state_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 719);
    let name = g.read::<u32>(w(0x11));
    let d = g.call(t, 0, &[name, 4, 0]).eax;
    g.write::<u32>(w(0x21), d);
    g.write_bytes(w(0x1c), &[0u8; 0x30]);
    g.write::<u16>(w(0x34), 0x80);
    g.write::<u32>(w(0x3a), 0x8080_8080);
    // the 0 slot 719 got stays on the stack under the call
    let clear = Frame {
        at: img.rel32_target(f + 0x43),
        esp: fr.esp.wrapping_sub(8),
        ..*fr
    };
    crate::s4_small::records_clear(g, img, &clear);
    for op in [0x4c, 0x56, 0x60] {
        g.write::<u32>(w(op), 0);
    }
    Ret::eax(1)
}

/// [`state_start`] (110 bytes, 1 overlay).
pub const STATE_START: Sig = Sig {
    len: 110,
    hash: 0x7aa8_2354_4a2f_1205,
    calls: &[
        (0x26, crate::Callee::Import("memset")),
        (
            0x43,
            crate::Callee::Sig(&crate::s4_small::RECORDS_CLEAR_1000),
        ),
    ],
    imports: &[],
};

/// Draws a part at its actor (0x10001410, cdecl `(part)`; a cookie
/// frame): the part's index `(part - BASE) >> 7` (`BASE` operand
/// `+0x21`) picks the actor in the table at `+0x2b`; slot 210 `(actor,
/// M)` fills the local matrix `M` (`ebp - 0x84`), slot 235 the local `S`
/// (`ebp - 0x44`), whose diagonal becomes 100.0; slot 231 `(M, M, S)`
/// scales `M`, and the part is drawn with it ([`draw`]), whose answer it
/// returns.
pub fn draw_at_actor<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = fr.arg(g, 0);
    let m = fr.esp.wrapping_sub(0x88);
    let s = fr.esp.wrapping_sub(0x48);
    let i = part.wrapping_sub(w(0x21)).cast_signed() >> 7;
    let actor = g.read::<u32>(w(0x2b).wrapping_add(i.cast_unsigned().wrapping_mul(4)));
    let t = slot_target(g, w(0x30), 210);
    g.call(t, 0, &[actor, m]);
    let t = slot_target(g, w(0x41), 235);
    g.call(t, 0, &[s, actor, m]);
    for o in [0x28, 0x14, 0] {
        g.write::<u32>(s + o, 0x42c8_0000);
    }
    let t = slot_target(g, w(0x5a), 231);
    g.call(t, 0, &[m, m, s, s, actor, m]);
    Ret::eax(draw(g, img, img.rel32_target(f + 0x87), part, m))
}

/// [`draw_at_actor`] (154 bytes, 1 overlay).
pub const DRAW_AT_ACTOR: Sig = Sig {
    len: 154,
    hash: 0xdf9f_a4c3_5ed9_04d4,
    calls: &[
        (0x87, crate::Callee::Sig(&DRAW)),
        // __security_check_cookie
        (0x92, crate::Callee::Any),
    ],
    imports: &[],
};
