//! `magic_0434.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The actor's colour reset and show (0x10005d10, internal, no
/// arguments): `m0431`'s [`colours`](crate::m0431::colours) (the same
/// operands), then slot 399 `(T[*B], 1)` (`B` = `*P` read again,
/// `+0x77`; `T` `+0x86`). Returns slot 399's `eax`.
pub fn colour_show<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    crate::m0431::colours(g, img, fr.at);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x77))));
    let a = g.read::<u32>(w(0x86).wrapping_add(b * 4));
    let s = slot_target(g, w(0x8b), 0x63c / 4);
    Ret::eax(g.call(s, 0, &[a, 1]).eax)
}

/// [`colour_show`] (156 bytes, 1 overlay).
pub const COLOUR_SHOW: Sig = Sig {
    len: 156,
    hash: 0x7208_7192_9f9d_cb7b,
    calls: &[],
    imports: &[],
};

/// Starts the second object at a scale (0x10009330, a table entry, no
/// arguments; the block `S = *G_S`, operand `+0x12`): slot 282 `()` gives
/// `v` (ST0, kept as f32 at `ebp - 4`); the object `S + 0x260`
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale `K1
/// / v * K2` with `K1`, `K2` at `+0x1f`, `+0x2f`, index 2, value 0); its
/// `+0x104` the immediate `+0x4b` and byte `+0x126` 2. Returns the
/// object.
pub fn second_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let t = slot_target(g, w(0x5), 282);
    let s = g.read::<u32>(w(0x12));
    let v = crate::m0018::st0_f32(g.call(t, 0, &[]).st0);
    g.write::<f32>(ebp.wrapping_sub(4), v);
    let scale = mul(div(g.read::<f32>(w(0x1f)), v), g.read::<f32>(w(0x2f)));
    let o = s.wrapping_add(0x260);
    let r = crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x3e), o, scale, 2, 0);
    g.write::<u32>(s.wrapping_add(0x364), w(0x4b));
    g.write::<u8>(s.wrapping_add(0x386), 2);
    Ret::eax(r)
}

/// [`second_start`] (91 bytes, 1 overlay).
pub const SECOND_START: Sig = Sig {
    len: 91,
    hash: 0x9cf8_2995_7674_286b,
    calls: &[(0x3e, crate::Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};

/// Restarts the ninth object and starts the main one (0x10006b00, a
/// table entry, no arguments; the block `S = *G_S`, operand `+0x03`):
/// when `S + 0x980` leaves the list at the immediate `+0x11`
/// ([`list_remove`](crate::l5_objhost::list_remove)), `**G_P` (`+0x1f`)
/// the immediate `+0x26`, slot 713 `(it)` and bit 15 of its halfword
/// `+0x124` cleared; then `S`
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale the
/// float at `+0x4a`, index 0, value 0), `S + 0xc4` = `S + 0x1a68`, `S +
/// 0xd0..0xdc` = `S + 0xca0..`, `S + 0xd4` less `K` (`+0x92`), `S + 0xe0`
/// 2.0 and `S + 0xe8` -11.5. Returns the last word copied.
pub fn main_restart<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x3));
    let o = s + 0x980;
    if crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x16), w(0x11), o) != 0 {
        g.write::<u32>(g.read::<u32>(w(0x1f)), w(0x26));
        let t = slot_target(g, w(0x2b), 713);
        g.call(t, 0, &[o]);
        let h = g.read::<u16>(o + 0x124);
        g.write::<u16>(o + 0x124, h & 0x7fff);
    }
    let scale = g.read::<f32>(w(0x4a));
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x58), s, scale, 0, 0);
    let v = g.read::<u32>(s + 0x1a68);
    g.write::<u32>(s + 0xc4, v);
    let mut last = 0;
    for i in 0..4u32 {
        last = g.read::<u32>(s + 0xca0 + 4 * i);
        g.write::<u32>(s + 0xd0 + 4 * i, last);
    }
    let y = crate::sb_sse::sub(g.read::<f32>(s + 0xca4), g.read::<f32>(w(0x92)));
    g.write::<f32>(s + 0xd4, y);
    g.write::<u32>(s + 0xe0, 0x4000_0000);
    g.write::<u32>(s + 0xe8, 0xc138_0000);
    Ret::eax(last)
}

/// [`main_restart`] (202 bytes, 1 overlay).
pub const MAIN_RESTART: Sig = Sig {
    len: 202,
    hash: 0x0a05_7242_3c94_b6ea,
    calls: &[
        (0x16, crate::Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x58, crate::Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};
