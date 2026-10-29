//! `magic_0046.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::l5_objhost::pool_alloc;
use crate::m0032::reset_state;
use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Game, Image, Sig};

/// Starts an effect object (0x10001b30, internal: `edx` = the object,
/// `xmm3` its scale): the model words (scale `+0x60..+0x6c`), `*G_P =
/// ITEM`, the part `list[0]` of slot 941 `(*G_D + 0x10)` started on it
/// ([`start_part`]), its state reset with the step `STEP` (operand
/// `+0x85`, [`reset_state`]), then a pool node ([`pool_alloc`]) holding
/// it appended to the object list (`G_HEAD`/`G_TAIL`, the count up).
/// Returns the object.
pub fn start_object<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, scale: f32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    for o in [0x48u32, 0x44, 0x40] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    for o in [0x5cu32, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + o, 0);
    }
    for o in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<f32>(obj + o, scale);
    }
    g.write::<u32>(p, w(0x56));
    let d = g.read::<u32>(g.read::<u32>(w(0x5b)).wrapping_add(0x10));
    let s941 = slot_target(g, w(0x63), 941);
    let list = g.call(s941, obj, &[d]).eax;
    let program = g.read::<u32>(list);
    start_part(g, img, img.rel32_target(f + 0x7d), obj, 0x1000, program, 1);
    // push STEP; push ecx (not read); call
    reset_state(g, obj, w(0x85), 0, 0);
    let node = pool_alloc(g, img, img.rel32_target(f + 0x95));
    if node == 0 {
        return obj;
    }
    g.write::<u32>(node + 4, 0);
    let tail = g.read::<u32>(w(0xa0));
    g.write::<u32>(node, tail);
    if tail == 0 {
        g.write::<u32>(w(0xb3), node);
        g.write::<u32>(w(0xb9), node);
        g.write::<u32>(node + 8, obj);
        let n = g.read::<u32>(w(0xc2));
        g.write::<u32>(w(0xc2), n.wrapping_add(1));
    } else {
        let tail = g.read::<u32>(w(0xcd));
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(0xd6), node);
        g.write::<u32>(node + 8, obj);
        let n = g.read::<u32>(w(0xdf));
        g.write::<u32>(w(0xdf), n.wrapping_add(1));
    }
    obj
}

/// [`start_object`] (231 bytes, 1 overlay).
pub const START_OBJECT: Sig = Sig {
    len: 231,
    hash: 0xa281_110d_314d_b0b4,
    calls: &[
        (0x7d, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x8d, Callee::Sig(&crate::m0032::RESET_STATE_0)),
        (0x95, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};
