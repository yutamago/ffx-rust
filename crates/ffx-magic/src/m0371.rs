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
