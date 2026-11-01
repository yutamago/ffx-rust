//! `magic_0562.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's per-frame task callback (0x10001a00, cdecl `(node)`; a
/// cookie frame): the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step)), the lights pulse
/// ([`pulse_lights`](crate::o6_fam_pairs4::pulse_lights)); slot 210
/// `(*G_A, M)` and slot 13 `(*G_A, N)` fill the local matrices `M`
/// (`ebp - 0x84`) and `N` (`ebp - 0x44`); the camera is scaled
/// ([`scaled_camera`](crate::m0052::scaled_camera), `ecx` = `ebp -
/// 0xc4`) and the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)). When they are done
/// `**G_F` (`+0x73`) = 1, and when the motion and the lights are done
/// too: slots 6 `(m, 0)`, 547 `(m)` on `*G_M` (`+0x95`), 357 and 356
/// `(m', -1, -1)` on `*G_M` again (`+0xb6`), 238 `(*G_S, 0)` (`+0xe2`),
/// the node's `+0x14` = 3 and its task's `+0x10` the callback (`+0xfe`),
/// the grid drawn ([`draw_grid`](crate::sb_misc::draw_grid)) and slot
/// 856 `()`, whose `eax` it returns. Else the objects' answer, or `G_F`.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let node = fr.arg(g, 0);
    // push ebp; sub esp, 0xc4; push ebx, esi, edi; the return address
    let ebp = fr.esp.wrapping_sub(4);
    let esp = ebp.wrapping_sub(0xd4);
    let m_done = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x1a));
    let pulse = Frame {
        at: img.rel32_target(f + 0x21),
        esp,
        ..*fr
    };
    let l_done = crate::o6_fam_pairs4::pulse_lights(g, img, &pulse).eax;
    let (mtx, n) = (ebp.wrapping_sub(0x84), ebp.wrapping_sub(0x44));
    let t = slot_target(g, w(0x27), 210);
    let a = g.read::<u32>(w(0x3c));
    g.call(t, t, &[a, mtx]);
    let t = slot_target(g, w(0x47), 13);
    let a2 = g.read::<u32>(w(0x4d));
    g.call(t, 0, &[a2, n, a, mtx]);
    crate::m0052::scaled_camera(
        g,
        img,
        img.rel32_target(f + 0x60),
        ebp.wrapping_sub(0xc4),
        esp,
    );
    let r = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x65), esp);
    if r != 1 {
        return Ret::eax(r);
    }
    let flag = g.read::<u32>(w(0x73));
    g.write::<u32>(flag, 1);
    if m_done != 1 || l_done != 1 {
        return Ret::eax(flag);
    }
    let m = g.read::<u32>(w(0x95));
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[0, m]);
    let t = slot_target(g, w(0x8f), 6);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    p.push(m);
    let t = slot_target(g, w(0xa2), 547);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    let m2 = g.read::<u32>(w(0xb6));
    for (op, slot) in [(0xb0, 357), (0xc8, 356)] {
        p.push_all(&[u32::MAX, u32::MAX, m2]);
        let t = slot_target(g, w(op), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0);
    }
    let s = g.read::<u32>(w(0xe2));
    p.push_all(&[0, s]);
    let t = slot_target(g, w(0xda), 238);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    g.write::<u32>(node.wrapping_add(0x14), 3);
    let task = g.read::<u32>(node.wrapping_add(8));
    g.write::<u32>(task.wrapping_add(0x10), w(0xfe));
    crate::sb_misc::draw_grid(g, img, img.rel32_target(f + 0x103));
    let t = slot_target(g, w(0x108), 856);
    Ret::eax(g.call(t, 0, &[]).eax)
}

/// [`task`] (293 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 293,
    hash: 0x05b9_53f6_c0d9_0049,
    calls: &[
        (0x1a, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (
            0x21,
            Callee::Sig(&crate::o6_fam_pairs4::PULSE_LIGHTS_0249_1F30),
        ),
        (0x60, Callee::Sig(&crate::m0052::SCALED_CAMERA)),
        (0x65, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x103, Callee::Sig(&crate::sb_misc::DRAW_GRID)),
        // __security_check_cookie
        (0x11d, Callee::Any),
    ],
    imports: &[],
};
