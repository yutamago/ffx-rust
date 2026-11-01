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

/// Which object start [`objects_start`] calls (its `rel32` at `+0x7e`).
#[derive(Clone, Copy)]
pub enum ObjectStart {
    /// [`start_object_reset_edx`] (`magic_0108`, `0221`).
    ResetEdx,
    /// [`start_object_first`](crate::s4_misc::start_object_first)
    /// (`magic_0229`).
    First,
}

/// A start-up with one object per actor (internal, no arguments; 544
/// bytes, `magic_0229` 0x10001370 and its copies): `*G_A` = the
/// immediate `+0x21`, `*G_B` = `+0x2b`, slot 719 `(NAME, 8, 0)` into
/// `G_PART`, the part set up ([`part_init`](crate::l5_objhost::part_init));
/// then for each of the `*G_N` (`+0x4a`, signed) actors `i`: the object
/// `OBJ + i * 0x130` (`+0x75`) starts (`start`: scale `*K` `+0x64`, the
/// owner `15 i`), its point `p = obj + 0xd0` gets the actor's character
/// `CHR[*G_SEL[i]]` (`+0xa4`, `+0x99`; slot 262 `(chr, p, p + 4, p + 8)`,
/// slot 268 `(chr, 0x11, p)`, `p + 0xc` 1.0), `obj + 0x120` the actor's
/// byte (`+0xec`), and its record `REC + i * 0x34` (`+0x10b`; `+0x10..`
/// `+0x2c` 0 but `+0x20`/`+0x24` the immediates `+0x129`/`+0x130`, halfword
/// `+0x30` 0) goes into `obj + 0x110`. (The code's branches for actor
/// -1, `+0x92`/`+0xe5`, never run: `i` starts at 0.) Last a node taken
/// inline from the free ring (count `u16 +0x167`, head `+0x176`; owner the
/// immediate `+0x1cc`) is appended to the task list (first `+0x1dd`, last
/// `+0x1ed`). Returns 0. Each slot call passes the words the original
/// leaves on the stack.
#[allow(clippy::too_many_lines)]
pub fn objects_start<G: crate::Game>(
    g: &mut G,
    img: &Image,
    fr: &crate::Frame,
    start: ObjectStart,
) -> crate::Ret {
    use crate::runtime::slot_target;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s719 = slot_target(g, w(0x07), 0xb3c / 4);
    g.write::<u32>(w(0x1d), w(0x21));
    g.write::<u32>(w(0x27), w(0x2b));
    let part = g.call(s719, 0, &[w(0x17), 8, 0]).eax;
    g.write::<u32>(w(0x35), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x3a));
    // push ebp; sub esp, 8; push esi; push ebx; push edi; sub esp, 8;
    // push owner; push ecx; call
    let esp = fr.esp.wrapping_sub(0x2c);
    let mut owner = 0u32;
    let mut i = 0u32;
    if g.read::<u32>(w(0x4a)).cast_signed() > 0 {
        loop {
            let obj = w(0x75).wrapping_add(i.wrapping_mul(0x130));
            let scale = g.read::<f32>(w(0x64));
            // the pushed `ecx` is never read
            g.write::<u32>(esp + 4, 0);
            g.write::<u32>(esp + 8, owner);
            let at = img.rel32_target(f + 0x7e);
            match start {
                ObjectStart::ResetEdx => {
                    let call = crate::Frame {
                        at,
                        esp,
                        edx: obj,
                        xmm: [0, 0, 0, u128::from(scale.to_bits())],
                        ..*fr
                    };
                    start_object_reset_edx(g, img, &call);
                }
                ObjectStart::First => {
                    crate::s4_misc::start_object_first(g, img, at, obj, scale, owner);
                }
            }
            let p = obj + 0xd0;
            let sel = g.read::<u32>(w(0x99));
            let k = u32::from(g.read::<u8>(sel.wrapping_add(i)));
            let chr = g.read::<u32>(w(0xa4).wrapping_add(4 * k));
            let mut s = crate::pushes::Pushes::default();
            s.push_all(&[p + 8, p + 4, p, chr]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0xb1), 0x418 / 4), 0);
            s.push_all(&[p, 0x11, chr]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0xc3), 0x430 / 4), 0);
            g.write::<u32>(p + 0xc, 0x3f80_0000);
            let sel = g.read::<u32>(w(0xec));
            let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
            g.write::<u32>(obj + 0x120, b);
            let off = i.wrapping_mul(0x34);
            g.write::<u32>(w(0x101).wrapping_add(off), 0);
            let rec = w(0x10b).wrapping_add(off);
            for o in [0x14, 0x10, 0x1c] {
                g.write::<u32>(rec + o, 0);
            }
            g.write::<u32>(rec + 0x20, w(0x129));
            g.write::<u32>(rec + 0x24, w(0x130));
            g.write::<u32>(rec + 0x28, 0);
            g.write::<u32>(rec + 0x2c, 0);
            g.write::<u16>(rec + 0x30, 0);
            g.write::<u32>(obj + 0x110, rec);
            owner = owner.wrapping_add(0xf);
            i = i.wrapping_add(1);
            if i.cast_signed() >= g.read::<u32>(w(0x158)).cast_signed() {
                break;
            }
        }
    }
    // the task node, the pool's take inlined
    let free = g.read::<u16>(w(0x167));
    let node = if free == 0 {
        // no node left: the original goes on with node 0 (and faults)
        0
    } else {
        let n = g.read::<u32>(w(0x176));
        let (next, prev) = (g.read::<u32>(n), g.read::<u32>(n + 4));
        g.write::<u32>(next + 4, prev);
        let (prev, next) = (g.read::<u32>(n + 4), g.read::<u32>(n));
        g.write::<u32>(prev, next);
        if free > 1 {
            let head = g.read::<u32>(n + 4);
            g.write::<u32>(w(0x193), head);
        } else {
            g.write::<u32>(w(0x19b), 0);
        }
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(n + o, 0);
        }
        g.write::<u32>(n + 8, n);
        g.write::<u32>(n + 0x10, w(0x1cc));
        g.write::<u8>(n + 0x18, 0);
        g.write::<u16>(w(0x1d7), free - 1);
        n
    };
    let first = g.read::<u32>(w(0x1dd));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x20d), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x216), node);
    } else {
        let last = g.read::<u32>(w(0x1ed));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x1fb), node);
    }
    crate::Ret::eax(0)
}

/// [`objects_start`] with [`start_object_reset_edx`].
pub fn objects_start_edx<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    objects_start(g, img, fr, ObjectStart::ResetEdx)
}

/// [`objects_start`] with `start_object_first`.
pub fn objects_start_first<G: crate::Game>(
    g: &mut G,
    img: &Image,
    fr: &crate::Frame,
) -> crate::Ret {
    objects_start(g, img, fr, ObjectStart::First)
}

/// [`objects_start`]'s identity with the object start `start`.
macro_rules! objects_copies {
    ($($sig:ident, $start:path, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`objects_start`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 544,
                hash: 0x5f25_e931_9bf4_9bbe,
                calls: &[
                    (0x3a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
                    (0x7e, Callee::Sig(&$start)),
                ],
                imports: &[],
            };
        )*
    };
}

objects_copies! {
    OBJECTS_START_0108, START_OBJECT_EDX_0108, "`magic_0108` 0x10001370";
    OBJECTS_START_0221, START_OBJECT_EDX_0221, "`magic_0221` 0x10001370";
    OBJECTS_START_0229, crate::s4_misc::START_OBJECT_FIRST, "`magic_0229` 0x10001370";
}

/// Which object start [`effect_start`] calls (its `rel32` at `+0x4c`).
#[derive(Clone, Copy)]
pub enum EffectObject {
    /// `o6_fam_object::start_object_first` (`magic_0166`).
    First,
    /// [`start_object_reset_bare`] (`magic_0308`).
    Bare,
}

/// A start-up with one object (internal, no arguments; 426 bytes,
/// `magic_0166` 0x10001340 and `magic_0308`'s copy): `*G_A`/`*G_B` set
/// (`+0x1a`/`+0x1e`, `+0x24`/`+0x28`), slot 719 `(NAME, 8, 0)` into
/// `G_PART` (`+0x32`), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the object `OBJ`
/// (`+0x47`, scale `*K` `+0x3f`) starts; slot 262 `(*G_H, P, P + 4, P +
/// 8)` (the immediates `+0x66`, `+0x61`, `+0x56`) gives a point whose y
/// drops by `K2` (`+0x7e`), its `w` 1.0; slot 333 `(*G_H)`'s float `v`
/// makes the yaw `K3 - v` wrapped once (`+0xae`, `+0xbd`, `+0xd1`,
/// `+0xc7`/`+0xde`) into `G_YAW` (`+0xed`); the record `REC` (`+0xf3` ..
/// `+0x139` 0, but `+0x11b`/`+0x125` the immediates, halfword `+0x143`
/// 0) and two words (`+0x149`, `+0x153`: the immediates after them;
/// 0x1e in `0166`, 0x2d in `0308`); the actor block
/// ([`me_block`](crate::sa_misc::me_block), `(the immediate +0xe5)`);
/// a free-list node ([`pop_node`](crate::m0032::pop_node)) appended to
/// the task list (first `+0x16a`, last `+0x17a`). Returns 0. Each slot
/// call passes the words the original leaves on the stack (the object
/// start's two junk words left out).
#[allow(clippy::too_many_lines)]
pub fn effect_start<G: crate::Game>(
    g: &mut G,
    img: &Image,
    fr: &crate::Frame,
    object: EffectObject,
) -> crate::Ret {
    use crate::runtime::slot_target;
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s719 = slot_target(g, w(0x05), 0xb3c / 4);
    g.write::<u32>(w(0x1a), w(0x1e));
    g.write::<u32>(w(0x24), w(0x28));
    let part = g.call(s719, 0, &[w(0x14), 8, 0]).eax;
    g.write::<u32>(w(0x32), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x37));
    // push ebp; push ecx; three pushes, one popped; sub esp, 8; call
    let call = crate::Frame {
        at: img.rel32_target(f + 0x4c),
        esp: fr.esp.wrapping_sub(0x1c),
        edx: w(0x47),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x3f)))],
        ..*fr
    };
    match object {
        EffectObject::First => {
            crate::o6_fam_object::start_object_first(g, img, call.at, &call);
        }
        EffectObject::Bare => {
            start_object_reset_bare(g, img, &call);
        }
    }
    let mut s = crate::pushes::Pushes::default();

    let h = g.read::<u32>(w(0x6c));
    s.push_all(&[w(0x56), w(0x61), w(0x66), h]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x51), 0x418 / 4), 0);
    let y = sub(g.read::<f32>(w(0x76)), k(g, 0x7e));
    s.push(g.read::<u32>(w(0x89)));
    g.write::<u32>(w(0x8f), 0x3f80_0000);
    g.write::<f32>(w(0x9b), y);
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x83), 0x534 / 4), 0).st0);
    let x = sub(k(g, 0xae), v);
    // comiss + jbe twice: an unordered value stays
    let yaw = if x > k(g, 0xbd) {
        sub(x, k(g, 0xc7))
    } else if k(g, 0xd1) > x {
        add(x, k(g, 0xde))
    } else {
        x
    };
    g.write::<f32>(w(0xed), yaw);
    for o in [0xf3, 0xfd, 0x107, 0x111] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x11b), w(0x11f));
    g.write::<u32>(w(0x125), w(0x129));
    g.write::<u32>(w(0x12f), 0);
    g.write::<u32>(w(0x139), 0);
    g.write::<u16>(w(0x143), 0);
    g.write::<u32>(w(0x149), w(0x14d));
    g.write::<u32>(w(0x153), w(0x157));
    // push IMM; call: the block's word
    let esp = fr.esp.wrapping_sub(0x10);
    g.write::<u32>(esp + 4, w(0xe5));
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x15c), w(0xe5));
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x164));
    let first = g.read::<u32>(w(0x16a));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x198), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x1a0), node);
    } else {
        let last = g.read::<u32>(w(0x17a));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x187), node);
    }
    crate::Ret::eax(0)
}

/// [`effect_start`] with `start_object_first`.
pub fn effect_start_first<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    effect_start(g, img, fr, EffectObject::First)
}

/// [`effect_start`] with [`start_object_reset_bare`].
pub fn effect_start_bare<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    effect_start(g, img, fr, EffectObject::Bare)
}

/// [`effect_start`] in `magic_0166` (0x10001340).
pub const EFFECT_START_0166: Sig = Sig {
    len: 426,
    hash: 0x720f_6d6a_22ca_3d74,
    calls: &[
        (0x37, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (
            0x4c,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0166_1FD0),
        ),
        (0x15c, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x164, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// [`effect_start`] in `magic_0308` (0x10001340).
pub const EFFECT_START_0308: Sig = Sig {
    len: 426,
    hash: 0x4df5_845f_ccca_95f9,
    calls: &[
        (0x37, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4c, Callee::Sig(&START_OBJECT_BARE_0308)),
        (0x15c, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x164, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};
