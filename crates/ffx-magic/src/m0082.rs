//! `magic_0082.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_misc::{MOVE_START_2, move_start_2};
use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `comiss t, k; jb`: on once `t` is at or past `k` (a NaN never is).
fn reached(t: f32, k: f32) -> bool {
    t >= k
}

/// A task's ring node (`task[0]`) leaves its ring: the neighbours
/// linked past it, the node pointing at itself. Returns its old `+0x4`
/// as read the second time.
fn unlink_task<G: Game>(g: &mut G, task: u32) -> u32 {
    let n = g.read::<u32>(task);
    let next = g.read::<u32>(n + 4);
    g.write::<u32>(next + 8, g.read::<u32>(n + 8));
    let prev = g.read::<u32>(n + 8);
    let eax = g.read::<u32>(n + 4);
    g.write::<u32>(prev + 4, eax);
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    eax
}

/// The camera's step (0x100068e0, a task step, cdecl `(task)`; `S` =
/// `task[4]`): the clock `S + 0x568` grows by `K` (`+0x16`); by the
/// u16 mode `S + 0x56c`:
/// - 0: once the clock reaches `K0` (`+0xbd`), slot 239 `(*H, -5, 0)`
///   (`+0xd4`) and the mode goes up;
/// - 1: once it reaches `K3` (`+0x3d`), seven of the camera's words 1.0
///   (`+0x54`..`+0x90`), its move started with the times `K2` (`+0x4e`)
///   and `K3` ([`move_start_2`]) and the task leaves its ring;
/// - else nothing.
///
/// Returns 0 while waiting, `mode - 1` from mode 2 on, slot 239's `eax`,
/// or the ring node's old `+0x4` after the move started.
pub fn camera_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    let t = add(g.read::<f32>(s + 0x568), g.read::<f32>(w(0x16)));
    let mode = u32::from(g.read::<u16>(s + 0x56c));
    g.write::<u32>(s + 0x568, t.to_bits());
    match mode {
        0 => {
            if !reached(t, g.read::<f32>(w(0xbd))) {
                return Ret::eax(0);
            }
            let h = g.read::<u32>(w(0xd4));
            let s239 = slot_target(g, w(0xc4), 0x3bc / 4);
            let eax = g.call(s239, 0, &[h, (-5i32).cast_unsigned(), 0]).eax;
            let m = g.read::<u16>(s + 0x56c);
            g.write::<u16>(s + 0x56c, m.wrapping_add(1));
            Ret::eax(eax)
        }
        1 => {
            let k3 = g.read::<f32>(w(0x3d));
            if !reached(t, k3) {
                return Ret::eax(0);
            }
            let k2 = g.read::<f32>(w(0x4e));
            for at in [0x54, 0x5e, 0x68, 0x72, 0x7c, 0x86, 0x90] {
                g.write::<u32>(w(at), 0x3f80_0000);
            }
            move_start_2(g, img, img.rel32_target(f + 0x99), k2, k3);
            Ret::eax(unlink_task(g, task))
        }
        _ => Ret::eax(mode - 1),
    }
}

/// [`camera_step`] (231 bytes, 1 overlay).
pub const CAMERA_STEP: Sig = Sig {
    len: 231,
    hash: 0x3e3b_37f6_27c5_09a0,
    calls: &[(0x99, Callee::Sig(&MOVE_START_2))],
    imports: &[],
};

/// The camera's step with the parts (0x100069d0, a task step, cdecl
/// `(task)`, an aligned frame; `S` = `task[4]`): the clock and modes of
/// [`camera_step`] (operands `+0x1d`, `+0x45`, `+0x56`, `+0x5c`..
/// `+0x98`); mode 0, once the clock reaches `K0` (`+0xc9`): the node
/// `S + 0x24c` appended to the list `L` (`+0xda`,
/// [`node_append`](crate::sb_misc::node_append)), slot 268
/// `(*M, 0x11, S + 0x268)` (`+0xfb`) with `S + 0x274` = 1.0 and
/// `S + 0x26c` = slot 12's float `(*M)` less `C` (`+0x12a`); the node
/// `S + 0x2a4` appended to `L`, its scale `+0x1c..+0x28` 1.0, `node
/// insert` of `S + 0x41c` with both nodes, the particle node `S + 0x120`
/// (vtable `+0x1b1`, record `[.., P, S + 0x41c, *A, Q, 0]`), slot 239
/// `(*H, -5, 0)` and the mode goes up. Returns as [`camera_step`] does.
pub fn camera_parts_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    let t = add(g.read::<f32>(s + 0x568), g.read::<f32>(w(0x1d)));
    let mode = u32::from(g.read::<u16>(s + 0x56c));
    g.write::<u32>(s + 0x568, t.to_bits());
    match mode {
        0 => {}
        1 => {
            let k3 = g.read::<f32>(w(0x45));
            if !reached(t, k3) {
                return Ret::eax(0);
            }
            let k2 = g.read::<f32>(w(0x56));
            for at in [0x5c, 0x66, 0x70, 0x7a, 0x84, 0x8e, 0x98] {
                g.write::<u32>(w(at), 0x3f80_0000);
            }
            move_start_2(g, img, img.rel32_target(f + 0xa1), k2, k3);
            return Ret::eax(unlink_task(g, task));
        }
        _ => return Ret::eax(mode - 1),
    }
    if !reached(t, g.read::<f32>(w(0xc9))) {
        return Ret::eax(0);
    }
    let list = w(0xda);
    let n1 = s + 0x24c;
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xe1), n1, list, 0);
    let v = s + 0x268;
    let m = g.read::<u32>(w(0xfb));
    let s268 = slot_target(g, w(0xe6), 0x430 / 4);
    g.call(s268, 0, &[m, 0x11, v]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let m = g.read::<u32>(w(0x112));
    let s12 = slot_target(g, w(0x102), 0x30 / 4);
    let y = g.call(s12, 0, &[m]).st0.unwrap_or(0.0) as f32;
    g.write::<u32>(v + 4, y.to_bits());
    let y = sub(g.read::<f32>(s + 0x26c), g.read::<f32>(w(0x12a)));
    g.write::<u32>(s + 0x26c, y.to_bits());
    let list = w(0x138);
    let n2 = s + 0x2a4;
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x145), n2, list, 0);
    // push ebp; and esp, -8; sub esp, 0x1c; three pushes: the record at
    // [esp + 0x10] = the aligned frame - 0x18
    let rec = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let group = s + 0x41c;
    for off in [0x2c0, 0x2c4, 0x2c8, 0x2cc] {
        g.write::<u32>(s + off, 0x3f80_0000);
    }
    for (k, v) in (0u32..).zip([group, list, w(0x18b), n1, w(0x197), n2]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    let at = img.rel32_target(f + 0x19c);
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at,
            ecx: rec,
            ..Frame::default()
        },
    );
    let a = g.read::<u32>(w(0x1a1));
    let part = s + 0x120;
    g.write::<u32>(rec, part);
    g.write::<u32>(part, w(0x1b1));
    for (k, v) in (1u32..).zip([w(0x1bd), group, a, w(0x1cd), 0]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x1da), rec);
    let h = g.read::<u32>(w(0x1ef));
    let s239 = slot_target(g, w(0x1df), 0x3bc / 4);
    let eax = g.call(s239, 0, &[h, (-5i32).cast_unsigned(), 0]).eax;
    let m = g.read::<u16>(s + 0x56c);
    g.write::<u16>(s + 0x56c, m.wrapping_add(1));
    Ret::eax(eax)
}

/// [`camera_parts_step`] (518 bytes, 1 overlay).
pub const CAMERA_PARTS_STEP: Sig = Sig {
    len: 518,
    hash: 0x70cb_28fb_e9fc_851f,
    calls: &[
        (0xa1, Callee::Sig(&MOVE_START_2)),
        (0xe1, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x145, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x19c, Callee::Sig(&crate::nodes::INSERT)),
        (0x1da, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};
