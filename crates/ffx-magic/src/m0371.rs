//! `magic_0371.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect block's reset (0x10002cb0, internal, no arguments): the
/// 0x110-byte block `B` (`+0x08`, `memset`) cleared, `*P` (`+0x16`) = the
/// immediate `+0x1a`, the words `+0x20`/`+0x2a` 10 and ten words
/// (`+0x34` to `+0x8e`, ten bytes apart in the code) 1. Returns `B`
/// (`memset`'s result).
pub fn block_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    g.write_bytes(w(0x08), &[0u8; 0x110]);
    g.write::<u32>(w(0x16), w(0x1a));
    g.write::<u32>(w(0x20), 0xa);
    g.write::<u32>(w(0x2a), 0xa);
    for k in 0..10 {
        g.write::<u32>(w(0x34 + 0xa * k), 1);
    }
    Ret::eax(w(0x08))
}

/// [`block_reset`] (151 bytes, 1 overlay).
pub const BLOCK_RESET: Sig = Sig {
    len: 151,
    hash: 0x4269_67e8_416e_cab4,
    calls: &[(0x0d, Callee::Import("memset"))],
    imports: &[],
};

/// The start-up's records (0x10001440, a table entry, no arguments):
/// slot 719 `(*G_D, 4, 0)` (operand `+0x11`) into `+0x1b`; the records
/// cleared ([`records_clear`](crate::s4_small::records_clear), one word
/// of the call left on the stack); the words `+0x29` 1, `+0x33` 0.025,
/// `+0x3d`/`+0x47` 0. Returns 1.
pub fn records_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = crate::runtime::slot_target(g, w(0x1), 719);
    let d = g.read::<u32>(w(0x11));
    let a = g.call(t, 0, &[d, 4, 0]).eax;
    g.write::<u32>(w(0x1b), a);
    let clear = Frame {
        at: img.rel32_target(f + 0x20),
        esp: fr.esp.wrapping_sub(8),
        ..*fr
    };
    crate::s4_small::records_clear(g, img, &clear);
    for (o, v) in [(0x29u32, 1u32), (0x33, 0x3ccc_cccd), (0x3d, 0), (0x47, 0)] {
        g.write::<u32>(w(o), v);
    }
    Ret::eax(1)
}

/// [`records_start`] (85 bytes, 1 overlay).
pub const RECORDS_START: Sig = Sig {
    len: 85,
    hash: 0x670b_aebf_a21e_8048,
    calls: &[(0x20, Callee::Sig(&crate::s4_small::RECORDS_CLEAR_1000))],
    imports: &[],
};
