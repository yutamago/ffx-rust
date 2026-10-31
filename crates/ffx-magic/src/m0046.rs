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

/// The start-up (0x10001370, no arguments): two pointer globals set
/// (`+0x17`, `+0x21`), slot 719 `(NAME, 8, 0)` (`+0x11`) into `+0x2f`,
/// the part's set-up ([`part_init`](crate::l5_objhost::part_init)), the
/// object `+0x44` started with the scale `K` (`+0x3c`,
/// [`start_object`]); the model `M` = `table[**B]` (`+0x62`, `B` at
/// `+0x4e`) gets slot 262 `(M, X, Y, Z)` (`+0x6c`, `+0x5b`, `+0x53`) and
/// slot 268 `(M, 0x11, X')` (`+0x7f`); a record of words set (`+0x95`
/// 1.0, `+0x9f`..`+0xbd` 0, `+0xc7`, `+0xd1` the immediates, `+0xdb`,
/// `+0xe5` 0, u16 `+0xef` 0, `+0xf5` its immediate); a task node off the
/// free list ([`pop_node`](crate::m0032::pop_node)) appended to the task
/// list (first `+0x104`/`+0x130`, last `+0x115`/`+0x122`/`+0x138`).
/// Returns 0. Each slot call passes the words the original leaves on the
/// stack.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s719 = slot_target(g, w(0x01), 0xb3c / 4);
    g.write::<u32>(w(0x17), w(0x1b));
    g.write::<u32>(w(0x21), w(0x25));
    let h = g.call(s719, fr.ecx, &[w(0x11), 8, 0]).eax;
    g.write::<u32>(w(0x2f), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x34));
    let k = g.read::<f32>(w(0x3c));
    start_object(g, img, img.rel32_target(f + 0x49), w(0x44), k);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x4e))));
    let m = g.read::<u32>(w(0x62).wrapping_add(4 * b));
    let s = slot_target(g, w(0x67), 0x418 / 4);
    let earlier = [m, w(0x6c), w(0x5b), w(0x53)];
    g.call(s, 0, &earlier);
    let s = slot_target(g, w(0x7a), 0x430 / 4);
    let mut args = vec![m, 0x11, w(0x7f)];
    args.extend_from_slice(&earlier[..3]);
    // call width: 6 (three words and the earlier call's first three)
    g.call(s, 0, &args);
    g.write::<u32>(w(0x95), 0x3f80_0000);
    for at in [0x9f, 0xa9, 0xb3, 0xbd] {
        g.write::<u32>(w(at), 0);
    }
    g.write::<u32>(w(0xc7), w(0xcb));
    g.write::<u32>(w(0xd1), w(0xd5));
    g.write::<u32>(w(0xdb), 0);
    g.write::<u32>(w(0xe5), 0);
    g.write::<u16>(w(0xef), 0);
    g.write::<u32>(w(0xf5), w(0xf9));
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0xfe));
    let first = g.read::<u32>(w(0x104));
    g.write::<u32>(node + 4, 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x130), node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(0x138), node);
    } else {
        let last = g.read::<u32>(w(0x115));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(0x122), node);
    }
    crate::Ret::eax(0)
}

/// [`start_up`] (319 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 319,
    hash: 0x20ef_39c3_853f_8f8c,
    calls: &[
        (0x34, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x49, Callee::Sig(&START_OBJECT)),
        (0xfe, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};
