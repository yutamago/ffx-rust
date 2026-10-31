//! `magic_0707.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::{Callee, Frame, Game, Image, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Starts a part with its look (0x100010b0, internal: `ecx` = the
/// object, `xmm2`/`xmm3` two floats, cdecl `(c, t1, t2, t3, r, g, b)`):
/// `*G_P = ITEM` (`+0x07`, `+0x0e`), slot 941 `(*G_D)` (`+0x19`) gives
/// the program table, the part started with its first program (the
/// `pppStartPart` wrapper, `(0x1000, table[0], 1)`); then `+0x1e` 1 (a
/// halfword), `+0x40`/`+0x44` the two floats, `+0x48` `c`, `+0x50..+0x58`
/// the three `t` truncated, `+0x60..+0x68` the colour, `+0x4c`, `+0x6c`
/// 1.0 and `+0x5c` 1. Returns `t3` truncated.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let (a, b) = (fr.xmm[2] as u32, fr.xmm[3] as u32);
    let gp = g.read::<u32>(w(0x7));
    g.write::<u32>(gp, w(0xe));
    let d = g.read::<u32>(w(0x19));
    let t = slot_target(g, w(0x13), 941);
    let table = g.call(t, 0, &[d]).eax;
    let program = g.read::<u32>(table);
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x3f), obj, 0x1000, program, 1);
    let arg = |g: &G, i: u32| fr.arg(g, i);
    g.write::<u16>(obj + 0x1e, 1);
    g.write::<u32>(obj + 0x40, a);
    let t1 = trunc(f32::from_bits(arg(g, 1)));
    g.write::<u32>(obj + 0x50, t1);
    let t2 = trunc(f32::from_bits(arg(g, 2)));
    g.write::<u32>(obj + 0x44, b);
    let c = arg(g, 0);
    g.write::<u32>(obj + 0x48, c);
    let r = arg(g, 4);
    g.write::<u32>(obj + 0x54, t2);
    let t3 = trunc(f32::from_bits(arg(g, 3)));
    g.write::<u32>(obj + 0x60, r);
    let gr = arg(g, 5);
    g.write::<u32>(obj + 0x64, gr);
    let bl = arg(g, 6);
    g.write::<u32>(obj + 0x4c, ONE);
    g.write::<u32>(obj + 0x58, t3);
    g.write::<u32>(obj + 0x5c, 1);
    g.write::<u32>(obj + 0x68, bl);
    g.write::<u32>(obj + 0x6c, ONE);
    t3
}

/// [`start`] (189 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 189,
    hash: 0x7093_244f_3970_78b9,
    calls: &[(0x3f, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
