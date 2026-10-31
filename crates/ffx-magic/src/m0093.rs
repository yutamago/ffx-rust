//! `magic_0093.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::m0032::reset_state;
use crate::{Mem, Sig};

/// 0x10001b80 (internal; `ecx` = the object, two stack words of which
/// only the second is read, cdecl): [`reset_state`] with `+0x114` 0 and
/// the count `+0x11c` 0x12. Returns 0x8000 (the flag word it stores).
pub fn reset_state_12(g: &mut impl Mem, this: u32, arg: u32) -> u32 {
    reset_state(g, this, arg, 0, 0x12);
    0x8000
}

/// [`reset_state_12`] (273 bytes, 1 overlay).
pub const RESET_STATE_12: Sig = Sig {
    len: 273,
    hash: 0x4dac_897b_4810_f785,
    calls: &[],
    imports: &[],
};

/// Sets up a part object (0x10001ca0, internal: `edx` = the object, the
/// float `xmm3` its four scales): `+0x40..+0x48` and `+0x50..+0x5c` 0,
/// `+0x4c` 1.0, `+0x60..+0x6c` the scale; the word the pointer global
/// (operand `+0x01`) points at gets the immediate at `+0x56`; slot 941
/// `(*O + 0x10)` (`O` at `+0x5b`) gives the program table and the
/// `pppStartPart` wrapper starts the object (`ecx`) with `(0x1000,
/// table[0], 1)`; the state reset with the step `+0x85`
/// ([`reset_state_12`]); then a pool node
/// ([`pool_alloc`](crate::l5_objhost::pool_alloc)), when one is free,
/// holds the object (`+0x8`) at the tail of the list (tail `+0xa0`, head
/// `+0xb3`, count `+0xc2`; `+0x4` 0, `+0x0` the old tail). Returns the
/// object.
pub fn part_object<G: crate::Game>(
    g: &mut G,
    img: &crate::Image,
    f: u32,
    obj: u32,
    scale: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    for off in [0x48, 0x44, 0x40] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    for off in [0x5c, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + off, 0);
    }
    for off in [0x6c, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + off, scale);
    }
    g.write::<u32>(p, w(0x56));
    let o = g.read::<u32>(w(0x5b));
    let s941 = crate::runtime::slot_target(g, w(0x63), 941);
    let table = g.call(s941, 0, &[g.read::<u32>(o + 0x10)]).eax;
    let program = g.read::<u32>(table);
    let wrapper = img.rel32_target(f + 0x7d);
    crate::opu_parts::start_part(g, img, wrapper, obj, 0x1000, program, 1);
    reset_state_12(g, obj, w(0x85));
    let node = crate::l5_objhost::pool_alloc(g, img, img.rel32_target(f + 0x95));
    if node == 0 {
        return obj;
    }
    let tail = g.read::<u32>(w(0xa0));
    g.write::<u32>(node + 4, 0);
    let count = if tail == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xb3), node);
        g.write::<u32>(w(0xb9), node);
        w(0xc2)
    } else {
        g.write::<u32>(node, tail);
        let tail = g.read::<u32>(w(0xcd));
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(0xd6), node);
        w(0xdf)
    };
    g.write::<u32>(node + 8, obj);
    let n = g.read::<u32>(count);
    g.write::<u32>(count, n.wrapping_add(1));
    obj
}

/// [`part_object`] (231 bytes, 1 overlay).
pub const PART_OBJECT: Sig = Sig {
    len: 231,
    hash: 0xa281_110d_314d_b0b4,
    calls: &[
        (0x7d, crate::Callee::Sig(&crate::opu_parts::START_PART)),
        (0x8d, crate::Callee::Sig(&RESET_STATE_12)),
        (0x95, crate::Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};
