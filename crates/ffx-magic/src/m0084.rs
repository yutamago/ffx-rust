//! `magic_0084.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::l5_objhost::pool_alloc;
use crate::m0032::reset_state;
use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Game, Image, Sig};

/// Starts an effect object (0x100026c0, internal: `edx` = the object,
/// `xmm3` its scale, the stack argument the part's index): as
/// [`crate::m0046::start_object`] with the part `list[index]` and the
/// state's count 0x14: the model words, `*G_P = ITEM`, the part started
/// ([`start_part`]), the state reset with the step `STEP` (operand
/// `+0x8c`), a pool node holding it appended to the object list. Returns
/// the object.
pub fn start_object<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    index: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
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
    g.write::<u32>(p, w(0x59));
    let d = g.read::<u32>(g.read::<u32>(w(0x5e)).wrapping_add(0x10));
    let s941 = slot_target(g, w(0x66), 941);
    let list = g.call(s941, obj, &[d]).eax;
    let program = g.read::<u32>(list.wrapping_add(4 * index));
    start_part(g, img, img.rel32_target(f + 0x84), obj, 0x1000, program, 1);
    // push STEP; push ecx (not read); call
    reset_state(g, obj, w(0x8c), 0, 0x14);
    let node = pool_alloc(g, img, img.rel32_target(f + 0x9c));
    if node == 0 {
        return obj;
    }
    g.write::<u32>(node + 4, 0);
    let tail = g.read::<u32>(w(0xa7));
    g.write::<u32>(node, tail);
    if tail == 0 {
        g.write::<u32>(w(0xba), node);
        g.write::<u32>(w(0xc0), node);
        g.write::<u32>(node + 8, obj);
        let n = g.read::<u32>(w(0xc9));
        g.write::<u32>(w(0xc9), n.wrapping_add(1));
    } else {
        let tail = g.read::<u32>(w(0xd5));
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(0xde), node);
        g.write::<u32>(node + 8, obj);
        let n = g.read::<u32>(w(0xe7));
        g.write::<u32>(w(0xe7), n.wrapping_add(1));
    }
    obj
}

/// [`start_object`] (240 bytes, 1 overlay).
pub const START_OBJECT: Sig = Sig {
    len: 240,
    hash: 0xd2ce_f33d_153e_65d0,
    calls: &[
        (0x84, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x94, Callee::Sig(&crate::m0032::RESET_STATE)),
        (0x9c, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};
