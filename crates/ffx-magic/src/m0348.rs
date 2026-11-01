//! `magic_0348.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's tasks (0x10006ca0, no arguments, a local at `ebp - 4`):
/// slot 619 `(0x5a)`; with `E` = `*G` (`+0x13`) the sprite block `E +
/// 0xcd00` gets its colour bytes and words (`+0x80..+0x98`, sizes 0x1e);
/// three pool nodes ([`take`](crate::sa_pool::take), owners the callbacks
/// `+0x29`, `+0xbc`, `+0x106`) go on the task list (first `+0x86`, last
/// `+0x9c`/`+0x11a`): the first holding the sprite block (`+0x14`), and
/// `E`'s `+0xd174..+0xd17c` (0, 0xfa, 0xfa) set between the first two.
/// Returns the third node.
pub fn tasks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x05), 0x9ac / 4);
    let fx = g.read::<u32>(w(0x13));
    // mov [ebp - 4], esi: the local holds E
    g.write::<u32>(fr.esp.wrapping_sub(8), fx);
    g.call(s, 0, &[0x5a]);
    let blk = fx.wrapping_add(0xcd00);
    g.write::<u32>(blk + 0x81, 0xff80_ffff);
    g.write::<u8>(blk + 0x80, 0xff);
    g.write::<u16>(blk + 0x85, 0xffff);
    g.write::<u8>(blk + 0x87, 0);
    g.write::<u32>(blk + 0x88, 0);
    g.write::<u16>(blk + 0x98, 0x48);
    g.write::<u32>(blk + 0x90, 0x1e);
    g.write::<u32>(blk + 0x8c, 0x1e);
    g.write::<u32>(blk + 0x94, 0);
    let take = img.rel32_target(f + 0x80);
    let first = crate::sa_pool::take(g, img, take, w(0x29));
    let mut head = g.read::<u32>(w(0x86));
    g.write::<u32>(first + 0x14, blk);
    g.write::<u32>(first + 4, 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0xb1), first);
    } else {
        let last = g.read::<u32>(w(0x9c));
        g.write::<u32>(first, last);
        g.write::<u32>(last + 4, first);
    }
    let fx = g.read::<u32>(fr.esp.wrapping_sub(8));
    g.write::<u32>(first + 8, first);
    g.write::<u32>(fx + 0xd174, 0);
    g.write::<u32>(fx + 0xd17c, 0xfa);
    g.write::<u32>(fx + 0xd178, 0xfa);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0xdf), w(0xbc));
    g.write::<u32>(second + 4, 0);
    if head == 0 {
        // only for a null first node
        head = second;
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x101), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first + 4, second);
    }
    g.write::<u32>(second + 8, second);
    let third = crate::sa_pool::take(g, img, img.rel32_target(f + 0x10e), w(0x106));
    g.write::<u32>(third + 4, 0);
    g.write::<u32>(w(0x11a), third);
    if head == 0 {
        g.write::<u32>(third, 0);
        g.write::<u32>(w(0x13a), third);
    } else {
        g.write::<u32>(third, second);
        g.write::<u32>(second + 4, third);
    }
    g.write::<u32>(third + 8, third);
    Ret::eax(third)
}

/// [`tasks`] (326 bytes, 1 overlay).
pub const TASKS: Sig = Sig {
    len: 326,
    hash: 0x5808_3bf8_239d_f79e,
    calls: &[
        (0x80, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xdf, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10e, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The lights given back and the models of slot 288 `(9, &n)` shown
/// (internal, no arguments): [`lights_and_give_back`](crate::s6_misc::lights_and_give_back)
/// on `*G_E + D` (operand `+0x06`, the displacement `+0x0e`), then slot
/// 296 `(byte, 1)` for each of the `n` model bytes (`n` a local at `ebp -
/// 4`, read again each round). Returns the last call's `eax`.
pub fn lights_models<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let e = g.read::<u32>(w(0x6)).wrapping_add(w(0xe));
    // push ecx; push esi; push edi; the call
    let callee = Frame {
        at: img.rel32_target(f + 0x13),
        esp: ebp.wrapping_sub(0x10),
        ecx: e,
        ..*fr
    };
    crate::s6_misc::lights_and_give_back(g, img, &callee);
    let n = ebp.wrapping_sub(4);
    let t = slot_target(g, w(0x1c), 288);
    let bytes = g.call(t, 0, &[9, n]).eax;
    let mut eax = bytes;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(n).cast_signed() {
        let b = u32::from(g.read::<u8>(bytes.wrapping_add(i)));
        let t = slot_target(g, w(0x45), 296);
        eax = g.call(t, 0, &[b, 1]).eax;
        i += 1;
    }
    eax
}

/// [`lights_models`] (`magic_0348` 0x10006720, 99 bytes, 1 overlay).
pub const LIGHTS_MODELS: Sig = Sig {
    len: 99,
    hash: 0x0d2a_2294_1f2e_a44f,
    calls: &[(0x13, Callee::Sig(&crate::s6_misc::LIGHTS_AND_GIVE_BACK))],
    imports: &[],
};
