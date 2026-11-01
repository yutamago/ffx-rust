//! `magic_0610.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Sig};

/// Checks the targets (0x1000a610, internal, no arguments): slots 4
/// `(0x3018)` and 27 `(0x3018, 1)` (the first call's word under the
/// second's); `*G_P` (operand `+0x2c`) the immediate `+0x30`. A target
/// (of `*G_N`, `+0x3b`, signed; the list `*G_L` `+0x54`, the objects at
/// `+0x5f`; target -1 is `G_ME` `+0x48` with `+0x4d`) whose object's kind
/// halfword is one of the seven words at `+0x6a`, or which slot 580
/// `(id)` rejects (non-zero), stops the scan with the result 1. Then
/// `*G_OK` (`+0xa9`) = no such target and slot 523 `(0, result)`.
/// Returns slot 523's `eax`.
pub fn check_targets<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x1), 4);
    g.call(t, 0, &[0x3018]);
    let t = slot_target(g, w(0x13), 27);
    g.call(t, 0, &[0x3018, 1, 0x3018]);
    g.write::<u32>(w(0x2c), w(0x30));
    let mut found = 0u32;
    let mut i = 0u32;
    'scan: while i.cast_signed() < g.read::<i32>(w(if i == 0 { 0x3b } else { 0x93 })) {
        let (id, o) = if i == u32::MAX {
            (g.read::<u32>(w(0x48)), g.read::<u32>(w(0x4d)))
        } else {
            let list = g.read::<u32>(w(0x54));
            let id = u32::from(g.read::<u8>(list.wrapping_add(i)));
            (id, g.read::<u32>(w(0x5f).wrapping_add(4 * id)))
        };
        let kind = u32::from(g.read::<u16>(o));
        for k in 0..7 {
            if kind == g.read::<u32>(w(0x6a) + 4 * k) {
                found = 1;
                break 'scan;
            }
        }
        let t = slot_target(g, w(0x79), 580);
        if g.call(t, 0, &[id]).eax != 0 {
            found = 1;
            break;
        }
        i = i.wrapping_add(1);
    }
    g.write::<u32>(w(0xa9), 1 - found);
    let t = slot_target(g, w(0xa3), 523);
    g.call(t, 0, &[0, found]).eax
}

/// [`check_targets`] (191 bytes, 1 overlay).
pub const CHECK_TARGETS: Sig = Sig {
    len: 191,
    hash: 0xcec2_9e39_c7bb_6202,
    calls: &[],
    imports: &[],
};
