//! The register thunks over the handlers the shared-4 list ported: the
//! shapes of [`crate::s3_wrap`] (the same bytes in every thunk of a
//! shape; each callee its own code identity), with their own table of
//! targets so the two lists' handlers stay apart.

use crate::s3_wrap::ECX_JMP_HASH;
use crate::{Callee, Frame, Game, Image, Ret, Sig, s3_nodes, s4_summon};

/// A handler's port on the frame its thunk hands it.
type Handler<G> = fn(&mut G, &Image, &Frame) -> Ret;

/// The handlers the thunks reach: each one's code identity and its port
/// on the callee's frame (`at` the handler, `ecx` the register the thunk
/// loaded, `esp` at the return address the handler returns to).
fn targets<G: Game>() -> [(&'static Sig, Handler<G>); 6] {
    [
        (&s3_nodes::LEAVE_AND_RESET, |g, img, fr| {
            Ret::eax(s3_nodes::leave_and_reset(g, img, fr))
        }),
        (&s4_summon::ELEM_BONE, s4_summon::elem_bone::<G>),
        (
            &s4_summon::COLOUR_NODE_FADE,
            s4_summon::colour_node_fade::<G>,
        ),
        (&s4_summon::MOTION_STATE, s4_summon::motion_state::<G>),
        (&s4_summon::STEP_NODE, s4_summon::step_node::<G>),
        (&s4_summon::PARTICLE_NODE, s4_summon::particle_node::<G>),
    ]
}

/// The handler at `target`, run on `fr` (`at` set to it). A target no
/// entry of [`targets`] matches cannot be bound (the thunk's `Sig` names
/// its callee), so it returns the frame's `eax`.
fn run<G: Game>(g: &mut G, img: &Image, target: u32, fr: Frame) -> Ret {
    let fr = Frame { at: target, ..fr };
    targets::<G>()
        .iter()
        .find(|(sig, _)| img.matches(target, sig))
        .map_or(Ret::eax(fr.eax), |(_, h)| h(g, img, &fr))
}

/// The 12-byte thunk (`push ebp; mov ebp, esp; mov ecx, [ebp + 8]; pop
/// ebp; jmp X`): `ecx` = the first stack word, then a jump to the handler
/// with the caller's stack (its `eax`, nothing popped).
pub fn ecx_jmp<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    run(g, img, target, Frame { ecx, ..*fr })
}

macro_rules! thunks {
    ($($(#[$doc:meta])* $name:ident: $len:literal, $hash:ident, $off:literal => $callee:path;)*) => {
        $(
            $(#[$doc])*
            pub const $name: Sig = Sig {
                len: $len,
                hash: $hash,
                calls: &[($off, Callee::Sig(&$callee))],
                imports: &[],
            };
        )*
    };
}

thunks! {
    /// `magic_0003` 0x100019b0 (107 DLLs) over
    /// [`s3_nodes::leave_and_reset`].
    JMP_LEAVE_AND_RESET: 12, ECX_JMP_HASH, 8 => s3_nodes::LEAVE_AND_RESET;
    /// `magic_0003` 0x10002360 (107 DLLs; `magic_0087`/`0088`
    /// 0x100025a0) over [`s4_summon::elem_bone`].
    JMP_ELEM_BONE: 12, ECX_JMP_HASH, 8 => s4_summon::ELEM_BONE;
    /// `magic_0003` 0x100019c0 (107 DLLs; `magic_0087`/`0088`
    /// 0x10001c00) over [`s4_summon::colour_node_fade`].
    JMP_COLOUR_NODE_FADE: 12, ECX_JMP_HASH, 8 => s4_summon::COLOUR_NODE_FADE;
    /// `magic_0003` 0x10002a70 (103 DLLs; `magic_0087`/`0088`
    /// 0x10002cb0) over [`s4_summon::motion_state`].
    JMP_MOTION_STATE: 12, ECX_JMP_HASH, 8 => s4_summon::MOTION_STATE;
    /// `magic_0070` 0x10005950 (10 DLLs) over [`s4_summon::step_node`].
    JMP_STEP_NODE: 12, ECX_JMP_HASH, 8 => s4_summon::STEP_NODE;
    /// `magic_0070` 0x10006320 (12 DLLs) over [`s4_summon::particle_node`].
    JMP_PARTICLE_NODE: 12, ECX_JMP_HASH, 8 => s4_summon::PARTICLE_NODE;
}
