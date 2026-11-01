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

/// `s6_misc::start_object_reset`'s shape (internal: `edx` = the object,
/// `xmm3` its scale, no frame) with the 273-byte reset with an argument
/// ([`reset_state_arg`](crate::o4_state::reset_state_arg), `ecx` = the
/// object, the argument the immediate `+0x85`): the model words, `*G_P =
/// ITEM` (`+0x01`, `+0x56`), the first part of slot 941 `(*G_D + 0x10)`
/// (`+0x5b`, `+0x63`) started ([`start_part`](crate::opu_parts::start_part),
/// 0x1000, 1), the reset, then a pool node holding the object appended to
/// the object list. Returns the object.
///
/// `magic_0182` 0x10001b70 (231 bytes; 6 copies, each with its reset).
pub fn start_object_reset_arg<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> u32 {
    let f = fr.at;
    let reset_at = img.rel32_target(f + 0x8d);
    let arg = img.word(f + 0x85);
    let reset = |g: &mut G| crate::o4_state::reset_state_arg(g, img, reset_at, fr.edx, arg);
    start_reset(
        g,
        img,
        fr,
        reset,
        (0, 0x95, [0xa0, 0xb3, 0xb9, 0xc2, 0xcd, 0xd6, 0xdf]),
    )
}

/// [`start_object_reset_arg`] with the 270-byte reset without an
/// argument ([`reset_state_bare`](crate::o4_state::reset_state_bare),
/// `ecx` = the object); the pool's `rel32` at `+0x8f` and the list's
/// operands 6 bytes earlier.
///
/// `magic_0245` 0x10002500 (225 bytes; 4 copies, each with its reset).
pub fn start_object_reset_bare<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> u32 {
    let reset_at = img.rel32_target(fr.at + 0x87);
    let reset = |g: &mut G| crate::o4_state::reset_state_bare(g, img, reset_at, fr.edx);
    start_reset(
        g,
        img,
        fr,
        reset,
        (0, 0x8f, [0x9a, 0xad, 0xb3, 0xbc, 0xc7, 0xd0, 0xd9]),
    )
}

/// [`start_object_reset_arg`] in a frame (operands 3 bytes later) with
/// the 269-byte reset ([`reset_state_edx`](crate::o4_state::reset_state_edx):
/// `ecx` = the object, `edx` = the second stack word, the argument the
/// immediate `+0x8b`); the pool's `rel32` at `+0x9b`.
///
/// `magic_0108` 0x10001c00 (239 bytes; 3 copies, each with its reset).
pub fn start_object_reset_edx<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> u32 {
    let reset_at = img.rel32_target(fr.at + 0x93);
    let x114 = fr.arg(g, 1);
    let arg = img.word(fr.at + 0x8b);
    let reset = |g: &mut G| crate::o4_state::reset_state_edx(g, img, reset_at, fr.edx, x114, arg);
    start_reset(
        g,
        img,
        fr,
        reset,
        (3, 0x9b, [0xa6, 0xb9, 0xbf, 0xc8, 0xd4, 0xdd, 0xe6]),
    )
}

/// The shapes' common body: the object start up to the reset (its
/// operands `d` bytes later), the reset, then the list append (`pool`
/// the pool's `rel32`, `list` the list's operands).
fn start_reset<G: crate::Game>(
    g: &mut G,
    img: &Image,
    fr: &crate::Frame,
    reset: impl FnOnce(&mut G),
    (d, pool, list): (u32, u32, [u32; 7]),
) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o + d);
    let obj = fr.edx;
    let scale = fr.xmm[3] as u32;
    let p = g.read::<u32>(w(0x1));
    for o in [0x48u32, 0x44, 0x40] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    for o in [0x5cu32, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + o, 0);
    }
    for o in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + o, scale);
    }
    g.write::<u32>(p, w(0x56));
    let data = g.read::<u32>(g.read::<u32>(w(0x5b)).wrapping_add(0x10));
    let t = crate::runtime::slot_target(g, w(0x63), 941);
    let table = g.call(t, 0, &[data]).eax;
    let program = g.read::<u32>(table);
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(f + 0x7d + d),
        obj,
        0x1000,
        program,
        1,
    );
    reset(g);
    crate::s4_misc::append_object(g, img, f, img.rel32_target(f + pool), obj, list);
    obj
}

/// [`start_object_reset_arg`] with its reset `Sig`.
macro_rules! reset_copies {
    ($($sig:ident, $reset:ident, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`start_object_reset_arg`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 231,
                hash: 0xa281_110d_314d_b0b4,
                calls: &[
                    (0x7d, Callee::Sig(&crate::opu_parts::START_PART)),
                    (0x8d, Callee::Sig(&crate::o4_state::$reset)),
                    (0x95, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
                ],
                imports: &[],
            };
        )*
    };
}

reset_copies! {
    START_OBJECT_RESET_0182, RESET_STATE_ARG_0182_1A50, "`magic_0182` 0x10001b70";
    START_OBJECT_RESET_0197, RESET_STATE_ARG_0197_19B0, "`magic_0197` 0x10001ad0";
    START_OBJECT_RESET_0252, RESET_STATE_ARG_0252_1EE0, "`magic_0252` 0x10002000";
    START_OBJECT_RESET_0266, RESET_STATE_ARG_0266_3900, "`magic_0266` 0x10003a20";
    START_OBJECT_RESET_0311, RESET_STATE_ARG_0311_2B40, "`magic_0311` 0x10002c60";
    START_OBJECT_RESET_0365, RESET_STATE_ARG_0365_1560, "`magic_0365` 0x10001680";
}

/// [`start_object_reset_bare`] with its reset `Sig`.
macro_rules! bare_copies {
    ($($sig:ident, $reset:ident, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`start_object_reset_bare`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 225,
                hash: 0x9340_ab38_6cfe_268d,
                calls: &[
                    (0x7d, Callee::Sig(&crate::opu_parts::START_PART)),
                    (0x87, Callee::Sig(&crate::o4_state::$reset)),
                    (0x8f, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
                ],
                imports: &[],
            };
        )*
    };
}

bare_copies! {
    START_OBJECT_BARE_0245, RESET_STATE_BARE_0245_23F0, "`magic_0245` 0x10002500";
    START_OBJECT_BARE_0298, RESET_STATE_BARE_0298_2CE0, "`magic_0298` 0x10002df0";
    START_OBJECT_BARE_0308, RESET_STATE_BARE_0308_1EC0, "`magic_0308` 0x10001fd0";
    START_OBJECT_BARE_0435, RESET_STATE_BARE_0435_2790, "`magic_0435` 0x100028a0";
}

/// [`start_object_reset_edx`] with its reset `Sig`.
macro_rules! edx_copies {
    ($($sig:ident, $reset:ident, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`start_object_reset_edx`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 239,
                hash: 0xfbf5_d324_0967_22ae,
                calls: &[
                    (0x80, Callee::Sig(&crate::opu_parts::START_PART)),
                    (0x93, Callee::Sig(&crate::o4_state::$reset)),
                    (0x9b, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
                ],
                imports: &[],
            };
        )*
    };
}

edx_copies! {
    START_OBJECT_EDX_0108, RESET_STATE_EDX_0108_1AF0, "`magic_0108` 0x10001c00";
    START_OBJECT_EDX_0221, RESET_STATE_EDX_0221_1AF0, "`magic_0221` 0x10001c00";
    START_OBJECT_EDX_0419, RESET_STATE_EDX_0419_37C0, "`magic_0419` 0x100038d0";
}
