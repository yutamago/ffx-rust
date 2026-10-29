//! `magic_0054.dll`'s own functions.

use crate::runtime::slot_target;
use crate::{Game, Image, Sig};

/// 0x100019e0 (internal, cdecl, no arguments): with the game flag
/// `*G_LOCK` set to 1, `pppInitData(G_DATA, program, 0)` (slot 711, the
/// program in `.rdata`); then `pppInitEnv(G_ENV, G_DATA, G_X, 0x60000)`
/// (slot 710; the three words of the first call stay on the stack after
/// it) and the four words after `G_ENV`'s block are cleared. Returns
/// what `pppInitEnv` returned.
pub fn init_ppp<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let lock = g.read::<u32>(w(0x01));
    let data = g.read::<u32>(w(0x07));
    let saved = g.read::<u32>(lock);
    let x = g.read::<u32>(w(0x11));
    g.write::<u32>(lock, 1);
    let slot711 = slot_target(g, w(0x1c), 711);
    let program = w(0x29);
    g.call(slot711, 0, &[data, program, 0]);
    let lock = g.read::<u32>(w(0x31));
    g.write::<u32>(lock, saved);
    let slot710 = slot_target(g, w(0x3d), 710);
    let data2 = g.read::<u32>(w(0x44));
    let r = g
        .call(slot710, 0, &[w(0x4f), data2, x, 0x60000, data, program, 0])
        .eax;
    for off in [0x5a, 0x65, 0x6f, 0x79] {
        g.write::<u32>(w(off), 0);
    }
    r
}

/// `init_ppp` (131 bytes).
pub const INIT_PPP: Sig = Sig {
    len: 131,
    hash: 0x4c58_738f_66fe_aab3,
    calls: &[],
    imports: &[],
};
