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
    /// `magic_0447` 0x100049b0.
    load_0447 => LOAD_0447, Zero, crate::m0390::start_up_0447, crate::m0390::START_UP_0447;
}
