//! `magic_0316.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10001430, cdecl `(task)`): the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when they answer 1
/// and the effect's (`*E`, `+0x06`) two done words `+0x110`/`+0x114` are
/// 1, the task gets state 3 and its record the callback `+0x48`, three
/// words 1.0 (`+0x29`, `+0x3d`, `+0x53`), slot 790's float into `+0x65`,
/// and a tail jump into the parts' registration
/// ([`register_parts`](crate::sa_misc::register_parts)). Returns the
/// objects' answer otherwise.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x06));
    // push ebp; push esi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x0b),
        fr.esp.wrapping_sub(0xc),
    );
    if r != 1 || g.read::<u32>(fx + 0x110) != 1 || g.read::<u32>(fx + 0x114) != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(w(0x29), 0x3f80_0000);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(w(0x3d), 0x3f80_0000);
    g.write::<u32>(rec + 0x10, w(0x48));
    let s = slot_target(g, w(0x4d), 0xc58 / 4);
    g.write::<u32>(w(0x53), 0x3f80_0000);
    let out = g.call(s, 0, &[]);
    let v = out.st0.unwrap_or(0.0) as f32;
    g.write::<f32>(w(0x65), v);
    Ret::eax(crate::sa_misc::register_parts(
        g,
        img,
        img.rel32_target(f + 0x6c),
        out.eax,
    ))
}

/// [`frame`] (115 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 115,
    hash: 0xb20f_dfdc_b83c_655a,
    calls: &[
        (0x0b, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x6c, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// The sprite task (0x10001760, no arguments, no frame): the effect's
/// (`*E`, `+0x03`) sizes `+0x8c`/`+0x90` 16 and `+0x94` 0; a pool node
/// ([`take`](crate::sa_pool::take), owner the callback `+0x08`) holding
/// the effect (`+0x14`) goes last on the task list (first `+0x31`, last
/// `+0x44`); then the effect's colour `+0x80` 0x80808080, `+0x84`,
/// `+0x88`, `+0x94` 0 and the kind halfword `+0x98` 0x48. Returns the
/// node.
pub fn sprite_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x03));
    g.write::<u32>(fx + 0x90, 0x10);
    g.write::<u32>(fx + 0x8c, 0x10);
    g.write::<u32>(fx + 0x94, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x2b), w(0x08));
    let head = g.read::<u32>(w(0x31));
    g.write::<u32>(node + 0x14, fx);
    g.write::<u32>(node + 4, 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x56), node);
    } else {
        let last = g.read::<u32>(w(0x44));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(fx + 0x80, 0x8080_8080);
    g.write::<u32>(fx + 0x84, 0);
    g.write::<u32>(fx + 0x88, 0);
    g.write::<u32>(fx + 0x94, 0);
    g.write::<u16>(fx + 0x98, 0x48);
    g.write::<u32>(w(0x8f), node);
    Ret::eax(node)
}

/// [`sprite_task`] (149 bytes, 1 overlay).
pub const SPRITE_TASK: Sig = Sig {
    len: 149,
    hash: 0x9f34_7e9a_2800_3a9d,
    calls: &[(0x2b, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// The task's step (0x100013e0, no arguments): with `E` = `*G` (`+0x03`,
/// read first) the 0x30-byte block `B` (`+0x0c`, `memset`) is cleared,
/// its halfword `*H` (`+0x1d`) gets 0x80 and `*W` (`+0x2a`) the word `*V`
/// (`+0x22`); the script's answer ([`run`](crate::sa_script::run),
/// `+0x2f`) goes to `E + 0x114` and the motion's
/// ([`motion_step`](crate::s3_motion::motion_step), `+0x3a`) to `E +
/// 0x110`. Returns the motion's answer.
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x03));
    g.write_bytes(w(0x0c), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x1d));
    g.write::<u16>(w(0x1d), h | 0x80);
    let v = g.read::<u32>(w(0x22));
    g.write::<u32>(w(0x2a), v);
    let a = crate::sa_script::run(g, img, img.rel32_target(f + 0x2f));
    g.write::<u32>(e + 0x114, a);
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x3a));
    g.write::<u32>(e + 0x110, m);
    Ret::eax(m)
}

/// [`step`] (70 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 70,
    hash: 0x48b2_3753_4ce1_d3b1,
    calls: &[
        (0x11, Callee::Import("memset")),
        (0x2f, Callee::Sig(&crate::sa_script::RUN)),
        (0x3a, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};
