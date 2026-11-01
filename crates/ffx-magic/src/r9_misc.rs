//! Round 9 (magic-r9-own): small functions shared by two or three of the
//! own rows below `magic_0450` (each named for the DLLs carrying it).

use crate::{Callee, Image, Sig};

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale, the stack `(index)`): [`start_object`](crate::s4_misc) with its
/// state block reset by the 273-byte reset with an argument
/// ([`reset_state_arg`](crate::o4_state::reset_state_arg): `ecx` = the
/// object, the second stack word the immediate `+0x8c`). Returns the
/// object.
///
/// `magic_0447` 0x10003320 and `magic_0100` 0x100021a0 (240 bytes; their
/// resets differ in the count they store).
pub fn start_object_state<G: crate::Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    index: u32,
) -> u32 {
    let reset_at = img.rel32_target(f + 0x94);
    let arg = img.word(f + 0x8c);
    let reset = |g: &mut G| {
        crate::o4_state::reset_state_arg(g, img, reset_at, obj, arg);
    };
    let list = [0xa7, 0xba, 0xc0, 0xc9, 0xd5, 0xde, 0xe7];
    crate::s4_misc::start_object(g, img, f, obj, scale, index, reset, (0x84, 0x9c, list))
}

/// [`start_object_state`] in `magic_0447` (0x10003320).
pub const START_OBJECT_STATE_0447: Sig = Sig {
    len: 240,
    hash: 0xd2ce_f33d_153e_65d0,
    calls: &[
        (0x84, Callee::Sig(&crate::opu_parts::START_PART)),
        (
            0x94,
            Callee::Sig(&crate::o4_state::RESET_STATE_ARG_0447_3200),
        ),
        (0x9c, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};

/// [`start_object_state`] in `magic_0100` (0x100021a0).
pub const START_OBJECT_STATE_0100: Sig = Sig {
    len: 240,
    hash: 0xd2ce_f33d_153e_65d0,
    calls: &[
        (0x84, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x94, Callee::Sig(&crate::s6_misc::RESET_STATE_1E)),
        (0x9c, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};
