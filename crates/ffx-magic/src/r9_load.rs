//! The load step most object-host overlays carry (a table entry, no
//! arguments; 121 copies of 118 bytes and 40 of 119 in the work list,
//! `magic_0018` 0x10001210 and `magic_0046` 0x10001210): it waits for the
//! effect's data and then runs the DLL's own start-up, the one `rel32`
//! callee. The copies differ only in their globals and in that callee,
//! so each binds by a [`Sig`] of its own naming the start-up's port, and
//! all run [`load_step`]. `magic_0446`'s copy (`s4_fam446::load_step`)
//! and `magic_0219`'s 119-byte one (`m0219::per_frame`) were ported
//! before this family.
//!
//! A copy becomes workable once its DLL's start-up is ported: add its
//! `Sig` and entry here (the [`copies!`] list) and a test in
//! `magic_r9_load.rs`.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The two shapes: slot 226's second word (`push eax`, 0, in the
/// 118-byte copies; `push 1` in the 119-byte ones, which moves every
/// later operand a byte on).
#[derive(Clone, Copy)]
pub enum Shape {
    /// 118 bytes, slot 226 `(res, 0)`.
    Zero,
    /// 119 bytes, slot 226 `(res, 1)`.
    One,
}

/// The load step: nothing more while slot 225 `(1)` is non-zero (its
/// `eax` returned); the first time (`*G_ASKED` 0, operand `+0x18`), slot
/// 226 `(*G_RES, 0 or 1)` and `G_ASKED` = 1; once slot 227 `(*G_RES)`
/// reports the data loaded (1 or more), the DLL's start-up runs
/// (`start`, the `rel32` callee) and two table words (operands
/// `+0x61`/`+0x6b`) get their next functions (the immediates `+0x65`,
/// `+0x6f`). Returns 0 (every exit is the closing `xor eax, eax`).
/// (Operands one byte later after `+0x1e` in [`Shape::One`].)
pub fn load_step<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    shape: Shape,
    start: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let f = fr.at;
    let (d, second) = match shape {
        Shape::Zero => (0, 0),
        Shape::One => (1, 1),
    };
    let w = |o: u32| img.word(f + o);
    let t = slot_target(g, w(0x1), 225);
    if g.call(t, 0, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x18)) == 0 {
        // `push eax; mov eax, [T]` here, `mov eax, [T]; push 1` there
        let t = slot_target(g, w(0x20 - d), 226);
        let res = g.read::<u32>(w(0x26 + d));
        g.call(t, 0, &[res, second]);
        g.write::<u32>(w(0x37 + d), 1);
    }
    let t = slot_target(g, w(0x40 + d), 227);
    let res = g.read::<u32>(w(0x46 + d));
    if g.call(t, 0, &[res]).eax.cast_signed() < 1 {
        return Ret::eax(0);
    }
    // the call's return address goes below the entry's
    let callee = Frame {
        at: img.rel32_target(f + 0x5b + d),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    start(g, img, &callee);
    g.write::<u32>(w(0x61 + d), w(0x65 + d));
    g.write::<u32>(w(0x6b + d), w(0x6f + d));
    Ret::eax(0)
}

/// One entry and one [`Sig`] per copy: `name => SIG, shape, start-up
/// port, its Sig`.
macro_rules! copies {
    ($($(#[$doc:meta])* $f:ident => $sig:ident, $shape:ident, $start:path, $ssig:path;)*) => {
        $(
            $(#[$doc])*
            pub fn $f<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
                load_step(g, img, fr, Shape::$shape, $start)
            }

            #[doc = concat!("[`", stringify!($f), "`].")]
            pub const $sig: Sig = Sig {
                len: match Shape::$shape {
                    Shape::Zero => 118,
                    Shape::One => 119,
                },
                hash: match Shape::$shape {
                    Shape::Zero => 0xf93b_1968_cd8e_f184,
                    Shape::One => 0x2da6_1889_6875_2369,
                },
                calls: &[(
                    match Shape::$shape {
                        Shape::Zero => 0x5b,
                        Shape::One => 0x5c,
                    },
                    Callee::Sig(&$ssig),
                )],
                imports: &[],
            };
        )*
    };
}

copies! {
    /// `magic_0018` 0x10001210.
    load_0018 => LOAD_0018, Zero, crate::m0018::start, crate::m0018::START_0018;
    /// `magic_0019` 0x10003720.
    load_0019 => LOAD_0019, Zero, crate::m0018::start, crate::m0018::START_0019;
    /// `magic_0046` 0x10001210.
    load_0046 => LOAD_0046, One, crate::m0046::start_up, crate::m0046::START_UP;
    /// `magic_0064` 0x100026d0.
    load_0064 => LOAD_0064, Zero, crate::m0052::start_up_0064, crate::m0052::START_UP_0064;
    /// `magic_0171` 0x10001210.
    load_0171 => LOAD_0171, Zero, crate::m0171::start_up, crate::m0171::START_UP;
    /// `magic_0367` 0x10001210.
    load_0367 => LOAD_0367, Zero, crate::m0367::start_up, crate::m0367::START_UP;
    /// `magic_0369` 0x10001210.
    load_0369 => LOAD_0369, Zero, crate::m0367::start_up_0369, crate::m0367::START_UP_0369;
    /// `magic_0383` 0x10001210.
    load_0383 => LOAD_0383, Zero, crate::m0381::start_up_0383, crate::m0381::START_UP_0383;
    /// `magic_0390` 0x100012b0.
    load_0390 => LOAD_0390, Zero, crate::m0390::start_up, crate::m0390::START_UP;
    /// `magic_0395` 0x10003df0.
    load_0395 => LOAD_0395, Zero, crate::m0395::start_up, crate::m0395::START_UP;
    /// `magic_0108` 0x10001210.
    load_0108 => LOAD_0108, One, crate::r9_misc::objects_start_edx, crate::r9_misc::OBJECTS_START_0108;
    /// `magic_0221` 0x10001210.
    load_0221 => LOAD_0221, One, crate::r9_misc::objects_start_edx, crate::r9_misc::OBJECTS_START_0221;
    /// `magic_0229` 0x10001210.
    load_0229 => LOAD_0229, One, crate::r9_misc::objects_start_first, crate::r9_misc::OBJECTS_START_0229;
    /// `magic_0166` 0x10001210.
    load_0166 => LOAD_0166, Zero, crate::r9_misc::effect_start_first, crate::r9_misc::EFFECT_START_0166;
    /// `magic_0308` 0x10001210.
    load_0308 => LOAD_0308, Zero, crate::r9_misc::effect_start_bare, crate::r9_misc::EFFECT_START_0308;
    /// `magic_0447` 0x100049b0.
    load_0447 => LOAD_0447, Zero, crate::m0390::start_up_0447, crate::m0390::START_UP_0447;
    /// `magic_0110` 0x100012b0.
    load_0110 => LOAD_0110, One, crate::r11_objects::three_nodes, crate::r11_objects::THREE_NODES_0110;
    /// `magic_0112` 0x100012b0.
    load_0112 => LOAD_0112, One, crate::r11_objects::three_nodes, crate::r11_objects::THREE_NODES_0112;
    /// `magic_0121` 0x10002d60.
    load_0121 => LOAD_0121, One, crate::r11_objects::three_nodes, crate::r11_objects::THREE_NODES_0121;
    /// `magic_0116` 0x10001300.
    load_0116 => LOAD_0116, One, crate::r11_objects::two_per_actor, crate::r11_objects::TWO_PER_ACTOR_0116;
    /// `magic_0117` 0x10002da0.
    load_0117 => LOAD_0117, One, crate::r11_objects::two_per_actor, crate::r11_objects::TWO_PER_ACTOR_0117;
    /// `magic_0120` 0x100012b0.
    load_0120 => LOAD_0120, One, crate::r11_objects::three_nodes_height, crate::r11_objects::THREE_NODES_HEIGHT_0120;
    /// `magic_0126` 0x10001300.
    load_0126 => LOAD_0126, One, crate::r11_objects::three_nodes_kind, crate::r11_objects::THREE_NODES_KIND_0126;
    /// `magic_0127` 0x10002bd0.
    load_0127 => LOAD_0127, One, crate::r11_objects::three_nodes_scaled, crate::r11_objects::THREE_NODES_SCALED_0127;
    /// `magic_0105` 0x10002fd0.
    load_0105 => LOAD_0105, One, crate::r11_fades::object_two_fades, crate::r11_fades::OBJECT_TWO_FADES_0105;
    /// `magic_0101` 0x10002e70.
    load_0101 => LOAD_0101, One, crate::r11_fades::object_colour_two_fades, crate::r11_fades::OBJECT_COLOUR_TWO_FADES_0101;
    /// `magic_0093` 0x10003120.
    load_0093 => LOAD_0093, One, crate::r11_fades::part_object_two_fades, crate::r11_fades::PART_OBJECT_TWO_FADES_0093;
    /// `magic_0085` 0x100030a0.
    load_0085 => LOAD_0085, One, crate::r11_fades::yaw_two_fades, crate::r11_fades::YAW_TWO_FADES_0085;
    /// `magic_0103` 0x10003220.
    load_0103 => LOAD_0103, One, crate::r11_fades::mean_two_fades, crate::r11_fades::MEAN_TWO_FADES_0103;
    /// `magic_0252` 0x10001210.
    load_0252 => LOAD_0252, Zero, crate::r11_effect::effect_start_flat, crate::r11_effect::EFFECT_START_FLAT_0252;
    /// `magic_0365` 0x10002ab0.
    load_0365 => LOAD_0365, One, crate::r11_effect::effect_start_height, crate::r11_effect::EFFECT_START_HEIGHT_0365;
    /// `magic_0197` 0x10001210.
    load_0197 => LOAD_0197, Zero, crate::r11_effect::effect_start_lookup, crate::r11_effect::EFFECT_START_LOOKUP_0197;
    /// `magic_0065` 0x10002e90.
    load_0065 => LOAD_0065, Zero, crate::r11_misc::colour_start_0065, crate::r11_misc::COLOUR_START_0065;
    /// `magic_0054` 0x10001210.
    load_0054 => LOAD_0054, One, crate::r11_misc::two_anim_objects_0054, crate::r11_misc::TWO_ANIM_OBJECTS_0054;
    /// `magic_0231` 0x10001210.
    load_0231 => LOAD_0231, One, crate::r11_effect::effect_start_kind, crate::r11_effect::EFFECT_START_KIND_0231;
    /// `magic_0233` 0x10001210.
    load_0233 => LOAD_0233, One, crate::r11_objects::sized_actors_0233, crate::r11_objects::SIZED_ACTORS_0233;
    /// `magic_0100` 0x100012b0.
    load_0100 => LOAD_0100, One, crate::r11_fades::state_two_fades, crate::r11_fades::STATE_TWO_FADES_0100;
    /// `magic_0084` 0x10001300.
    load_0084 => LOAD_0084, One, crate::r11_fades::state_yaw_two_fades, crate::r11_fades::STATE_YAW_TWO_FADES_0084;
    /// `magic_0102` 0x10001300.
    load_0102 => LOAD_0102, One, crate::r11_fades::kinds_two_fades, crate::r11_fades::KINDS_TWO_FADES_0102;
}
