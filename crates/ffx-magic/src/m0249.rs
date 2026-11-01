//! `magic_0249.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (0x10001a20, cdecl `(task)`, a /GS frame): the motion
/// step ([`motion_step`](crate::s3_motion::motion_step)) and the lights'
/// pulse ([`pulse_lights`](crate::o6_fam_pairs4::pulse_lights)); slot 210
/// `(*G_E, &A)` and slot 13 `(*G_E, &B)` (`*G_E` operand `+0x3c`/`+0x4d`;
/// `A`, `B` locals at `ebp - 0x84`, `- 0x44`; the first call's words left
/// under the second's); the scaled camera
/// ([`scaled_camera`](crate::m0052::scaled_camera), `ecx` = `ebp -
/// 0xc4`) and the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)). When all three
/// give 1: slot 6 `(E, 0)`, slot 547 `(E)`, slots 357 and 356 `(*G_E, -1,
/// -1)` (each over the earlier words), the count or slot 257
/// ([`count_or_257`](crate::s4_small::count_or_257)), the task's state 3
/// and its record's `+0x10` the immediate `+0xd6`, then slot 856 `()`.
/// Returns slot 856's `eax`, else the objects' result.
pub fn camera_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    // sub esp, 0xc4; push ebx/esi/edi; the call
    let below = ebp.wrapping_sub(0xd4);
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x1a));
    let pulse = Frame {
        at: img.rel32_target(f + 0x21),
        esp: below,
        ..*fr
    };
    let p = crate::o6_fam_pairs4::pulse_lights(g, img, &pulse).eax;
    let e = g.read::<u32>(w(0x3c));
    let (a, b) = (ebp.wrapping_sub(0x84), ebp.wrapping_sub(0x44));
    let t = slot_target(g, w(0x27), 210);
    g.call(t, 0, &[e, a]);
    let e2 = g.read::<u32>(w(0x4d));
    let t = slot_target(g, w(0x47), 13);
    g.call(t, 0, &[e2, b, e, a]);
    crate::m0052::scaled_camera(
        g,
        img,
        img.rel32_target(f + 0x60),
        ebp.wrapping_sub(0xc4),
        below,
    );
    let r = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x65), below);
    if r != 1 || m != 1 || p != 1 {
        return Ret::eax(r);
    }
    let e = g.read::<u32>(w(0x7d));
    let t = slot_target(g, w(0x77), 6);
    g.call(t, 0, &[e, 0]);
    let t = slot_target(g, w(0x8a), 547);
    g.call(t, 0, &[e, e, 0]);
    let e2 = g.read::<u32>(w(0x9e));
    let m1 = u32::MAX;
    let t = slot_target(g, w(0x98), 357);
    g.call(t, 0, &[e2, m1, m1, e, e, 0]);
    let t = slot_target(g, w(0xb0), 356);
    g.call(t, 0, &[e2, m1, m1, e2, m1, m1, e, e, 0]);
    let count = Frame {
        at: img.rel32_target(f + 0xc5),
        esp: below,
        ..*fr
    };
    crate::s4_small::count_or_257(g, img, &count);
    let task = fr.arg(g, 0);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(task.wrapping_add(0x14), 3);
    g.write::<u32>(rec.wrapping_add(0x10), w(0xd6));
    let t = slot_target(g, w(0xdb), 856);
    Ret::eax(g.call(t, 0, &[]).eax)
}

/// [`camera_step`] (248 bytes, 1 overlay; the `rel32` at `+0xf0` is
/// `__security_check_cookie`).
pub const CAMERA_STEP: Sig = Sig {
    len: 248,
    hash: 0x5da8_7de9_2fdc_b7a7,
    calls: &[
        (0x1a, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (
            0x21,
            Callee::Sig(&crate::o6_fam_pairs4::PULSE_LIGHTS_0249_1F30),
        ),
        (0x60, Callee::Sig(&crate::m0052::SCALED_CAMERA)),
        (0x65, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0xc5, Callee::Sig(&crate::s4_small::COUNT_OR_257)),
        (0xf0, Callee::Any),
    ],
    imports: &[],
};
