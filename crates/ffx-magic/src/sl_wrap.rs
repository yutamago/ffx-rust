//! Register thunks over per-DLL handlers (agent magic-small-lo): the
//! 12-byte `ecx` thunk of [`crate::s3_wrap`] (the same bytes in every
//! copy; each callee its own code identity), with its own table of
//! targets. These are the parameter family `P073-956bfb0a` (41 thunks
//! of 12 bytes, `tools/overlay-pfamilies.py`): a member binds once its
//! target's port is here.

use crate::s3_wrap::ECX_JMP_HASH;
use crate::{Callee, Frame, Game, Image, Ret, Sig, m0146, m0164, m0186, m0201, m0203};

/// A handler's port on the frame its thunk hands it.
type Handler<G> = fn(&mut G, &Image, &Frame) -> Ret;

/// The handlers the thunks reach: each one's code identity and its port
/// on the callee's frame (`at` the handler, `ecx` the loaded word, `esp`
/// at the return address the handler returns to).
fn targets<G: Game>() -> [(&'static Sig, Handler<G>); 13] {
    [
        (&m0146::SUB_NODE_VISIT, m0146::sub_node_visit::<G>),
        (&m0146::SUB_NODE_VISIT_0173, m0146::sub_node_visit::<G>),
        (&m0164::MEAN_VISIT, m0164::mean_visit::<G>),
        (&m0164::MEAN_VISIT_0613, m0164::mean_visit::<G>),
        (&m0186::INT_MOTION, m0186::int_motion::<G>),
        (&m0201::MATRIX_VISIT, m0201::matrix_visit::<G>),
        (&m0203::MODEL_VISIT, m0203::model_visit::<G>),
        (&crate::s7_misc::LIT_COLOUR_NODE, |g, img, fr| {
            Ret::eax(crate::s7_misc::lit_colour_node(g, img, fr))
        }),
        (&crate::m0573::REFRESH, crate::m0573::refresh::<G>),
        (&crate::m0445::FADE_VISIT, crate::m0445::fade_visit::<G>),
        (
            &crate::o6_fam_pairs5::SPRITE_VISIT_0585_3110,
            crate::o6_fam_pairs5::sprite_visit::<G>,
        ),
        (
            &crate::r28::STRIP_TASK_0150,
            crate::r28::strip_task_0150::<G>,
        ),
        (
            &crate::r28::STRIP_FRAME_0150,
            crate::r28::strip_frame_0150::<G>,
        ),
    ]
}

/// The 12-byte thunk (`push ebp; mov ebp, esp; mov ecx, [ebp + 8]; pop
/// ebp; jmp X`): `ecx` = the first stack word, then a jump to the handler
/// with the caller's stack (its `eax`, nothing popped). A target no entry
/// of [`targets`] matches cannot be bound (the thunk's `Sig` names its
/// callee), so it returns the frame's `eax`.
pub fn ecx_jmp<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    let fr = Frame {
        at: target,
        ecx,
        ..*fr
    };
    targets::<G>()
        .iter()
        .find(|(sig, _)| img.matches(target, sig))
        .map_or(Ret::eax(fr.eax), |(_, h)| h(g, img, &fr))
}

macro_rules! thunks {
    ($($(#[$doc:meta])* $name:ident => $callee:path;)*) => {
        $(
            $(#[$doc])*
            pub const $name: Sig = Sig {
                len: 12,
                hash: ECX_JMP_HASH,
                calls: &[(8, Callee::Sig(&$callee))],
                imports: &[],
            };
        )*
    };
}

thunks! {
    /// `magic_0146` 0x10006380 over [`m0146::sub_node_visit`].
    JMP_SUB_NODE_VISIT => m0146::SUB_NODE_VISIT;
    /// `magic_0173` 0x100061f0 over [`m0146::sub_node_visit`] (agent
    /// magic-r26).
    JMP_SUB_NODE_VISIT_0173 => m0146::SUB_NODE_VISIT_0173;
    /// `magic_0164` 0x100049a0 over [`m0164::mean_visit`].
    JMP_MEAN_VISIT => m0164::MEAN_VISIT;
    /// `magic_0186` 0x100057f0 over [`m0186::int_motion`].
    JMP_INT_MOTION => m0186::INT_MOTION;
    /// `magic_0201` 0x100043a0 over [`m0201::matrix_visit`].
    JMP_MATRIX_VISIT => m0201::MATRIX_VISIT;
    /// `magic_0203` 0x10002f20 over [`m0203::model_visit`].
    JMP_MODEL_VISIT => m0203::MODEL_VISIT;
    /// `magic_0573` 0x10002df0 over [`crate::m0573::refresh`].
    JMP_REFRESH_0573 => crate::m0573::REFRESH;
    /// `magic_0445` 0x100035b0 over [`crate::m0445::fade_visit`].
    JMP_FADE_VISIT => crate::m0445::FADE_VISIT;
    /// `magic_0585` 0x10003410 over [`crate::o6_fam_pairs5::sprite_visit`].
    JMP_SPRITE_VISIT_0585 => crate::o6_fam_pairs5::SPRITE_VISIT_0585_3110;
    /// `magic_0150` 0x10007bf0 over [`crate::r28::strip_task_0150`] (agent
    /// magic-r28).
    JMP_STRIP_TASK_0150 => crate::r28::STRIP_TASK_0150;
    /// `magic_0150` 0x10006e70 over [`crate::r28::strip_frame_0150`] (agent
    /// magic-r28).
    JMP_STRIP_FRAME_0150 => crate::r28::STRIP_FRAME_0150;
}
