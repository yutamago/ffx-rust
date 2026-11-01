//! `magic_0508.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's callback (0x10001b40, `(obj)`, pushed by 0x100017bd; a
/// cookie frame): with the actor `a = T[b[0]]` (`b = *G_B`, operands
/// `+0x11` and `+0x1d`), slot 262 `(a, &v.x, &v.y, &v.z)` and slot 268
/// `(a, 0x10, &v)` fill the local point `v` (`ebp - 0x14`), whose `y`
/// goes to the object's `+0xd4`; `+0x104` 0. Returns slot 268's `eax`.
pub fn e1b40<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let b = g.read::<u32>(w(0x11));
    let i = u32::from(g.read::<u8>(b));
    let a = g.read::<u32>(w(0x1d).wrapping_add(4 * i));
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x31), 262);
    let mut stack = vec![a, v, v + 4, v + 8];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x43), 268);
    stack.splice(0..0, [a, 0x10, v]);
    let r = g.call(t, 0, &stack).eax;
    let y = g.read::<u32>(v + 4);
    g.write::<u32>(obj + 0xd4, y);
    g.write::<u32>(obj + 0x104, 0);
    Ret::eax(r)
}

/// 0x10001b40 (124 bytes, 1 overlay).
pub const E1B40: Sig = Sig {
    len: 124,
    hash: 0x061c_dd0e_27bf_e7ce,
    calls: &[
        // __security_check_cookie
        (0x74, Callee::Any),
    ],
    imports: &[],
};

/// The effect's per-frame task callback (0x10001a20, cdecl `(node)`; a
/// cookie frame). The count `G_A` (operand `+0x11`), when above 0,
/// steps down; reaching 0 it starts the flight: slot 531 `(*G_CH, L)`
/// fills the local `L` (`ebp - 0x14`) and the flight task
/// ([`flight_task`](crate::s5_misc::flight_task): `ecx` the object
/// `+0x51`, `xmm3` = `*K` `+0x41`, the stack `(L, 1.2, 1.0, 0x13)`)
/// starts, the flag `+0x6f` 1. The count `G_B` (`+0x78`) likewise; at 0
/// slot 624 `(*G_X)` (`+0x8f`). Then the motion script steps
/// ([`motion_script`](crate::s4_misc::motion_script), `+0x9f`) and the
/// objects run ([`run_objects`](crate::l5_objhost::run_objects)); when
/// both say 1, `G_B` is 0 and the flag word `+0xc3` has bit 0x8000:
/// unless the half-word `+0xd0` is set slot 213 `(*G_M, 0)`, the flag
/// `+0xf1` 1, the node's `+0x14` = 3 and its task's `+0x10` the
/// callback (the immediate `+0x106`), returning the task. Else the
/// objects' answer.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let node = fr.arg(g, 0);
    let a = g.read::<u32>(w(0x11)).cast_signed();
    if a > 0 {
        g.write::<u32>(w(0x20), (a - 1).cast_unsigned());
        if a == 1 {
            // ebp = esp - 4; L = ebp - 0x14
            let local = fr.esp.wrapping_sub(0x18);
            let t = slot_target(g, w(0x2b), 531);
            let ch = g.read::<u32>(w(0x31));
            g.call(t, 0, &[ch, local]);
            // the flight's frame: (L, 1.2, 1.0, 0x13) under the return
            let esp = fr.esp.wrapping_sub(0x34);
            for (i, v) in (1u32..).zip([local, 0x3f99_999a, 0x3f80_0000, 0x13]) {
                g.write::<u32>(esp + 4 * i, v);
            }
            let mut xmm = fr.xmm;
            xmm[3] = u128::from(g.read::<u32>(w(0x41)));
            let flight = Frame {
                at: img.rel32_target(f + 0x66),
                esp,
                ecx: w(0x51),
                xmm,
                ..*fr
            };
            crate::s5_misc::flight_task(g, img, &flight);
            g.write::<u32>(w(0x6f), 1);
        }
    }
    let b = g.read::<u32>(w(0x78)).cast_signed();
    if b > 0 {
        g.write::<u32>(w(0x82), (b - 1).cast_unsigned());
        if b == 1 {
            let t = slot_target(g, w(0x89), 624);
            let x = g.read::<u32>(w(0x8f));
            g.call(t, 0, &[x]);
        }
    }
    let m = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0xa4), w(0x9f));
    // push ebp, esi, edi, the return address; 0x14 bytes of locals
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0xab),
        fr.esp.wrapping_sub(0x24),
    );
    if r != 1 || m != 1 || g.read::<u32>(w(0xba)) != 0 || g.read::<u32>(w(0xc3)) & 0x8000 == 0 {
        return Ret::eax(r);
    }
    if g.read::<u16>(w(0xd0)) == 0 {
        let t = slot_target(g, w(0xd8), 213);
        let model = g.read::<u32>(w(0xe0));
        g.call(t, 0, &[model, 0]);
    }
    g.write::<u32>(w(0xf1), 1);
    let task = g.read::<u32>(node.wrapping_add(8));
    g.write::<u32>(node.wrapping_add(0x14), 3);
    g.write::<u32>(task.wrapping_add(0x10), w(0x106));
    Ret::eax(task)
}

/// [`task`] (282 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 282,
    hash: 0x66fd_532a_d61c_c017,
    calls: &[
        (0x66, Callee::Sig(&crate::s5_misc::FLIGHT_TASK)),
        (0xa4, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xab, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        // __security_check_cookie
        (0x112, Callee::Any),
    ],
    imports: &[],
};
