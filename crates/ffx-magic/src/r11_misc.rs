//! Round 11 (magic-r11-own): small own functions of the DLLs below
//! `magic_0450` (each named for the DLL carrying it).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The code byte at `at` (an 8-bit immediate).
#[must_use]
pub(crate) fn byte(img: &Image, at: u32) -> u32 {
    u32::from(img.word(at).to_le_bytes()[0])
}

/// The code byte at `at` sign-extended (`add r32, imm8`, `push imm8`).
#[must_use]
pub(crate) fn imm8(img: &Image, at: u32) -> u32 {
    i32::from(img.word(at).to_le_bytes()[0].cast_signed()).cast_unsigned()
}

/// Starts the main object (no arguments, no frame; `magic_0383`
/// 0x10001370, 39 bytes): `OBJ` (operand `+0x0c`, scale `*K` `+0x04`)
/// starts ([`start_object_first`](crate::o6_fam_object::start_object_first),
/// `rel32` `+0x11`, below `sub esp, 0x10`), then the 16 bytes at `+0x18`
/// are copied to `+0x22` (`movaps`). Returns the object start's `eax`.
pub fn main_object_0383<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let at = img.rel32_target(f + 0x11);
    let call = Frame {
        at,
        esp: fr.esp.wrapping_sub(0x14),
        edx: w(0x0c),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x04)))],
        ..*fr
    };
    let obj = crate::o6_fam_object::start_object_first(g, img, at, &call);
    let (from, to) = (w(0x18), w(0x22));
    let v: [u32; 4] = core::array::from_fn(|i| g.read::<u32>(from + 4 * i as u32));
    for (i, x) in (0u32..).zip(v) {
        g.write::<u32>(to + 4 * i, x);
    }
    Ret::eax(obj)
}

/// [`main_object_0383`] (`magic_0383` 0x10001370).
pub const MAIN_OBJECT_0383: Sig = Sig {
    len: 39,
    hash: 0x0896_2ff1_b119_e300,
    calls: &[(
        0x11,
        Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
    )],
    imports: &[],
};

/// Starts the main object at its actor (no arguments; `magic_0369`
/// 0x10001370, 213 bytes): `OBJ` (`+0x11`, scale `*K` `+0x08`) starts
/// ([`start_object_first`](crate::o6_fam_object::start_object_first),
/// `rel32` `+0x16`); slot 262 `(*A, &P.x, &P.y, &P.z)` (`A` `+0x21`, the
/// immediates `+0x36`, `+0x31`, `+0x2c`); slot 12 `(*A)`'s float less `K2`
/// (`+0x5a`) into `P.y` (`+0x4a`, `+0x77`), `P.w` (`+0x6b`) 1.0; slot 333
/// `(*A)`'s float `v` gives the yaw `K3 - v` (`+0x8a`), wrapped once
/// (above `HI` `+0x9a` less `T` `+0xa4`, below `LO` `+0xb8` plus `T`
/// `+0xc5`; `comiss` + `jbe`: a NaN stays) into `G_YAW` (`+0xac`/`+0xcd`).
/// Returns slot 333's `eax`. Each slot call passes the words the original
/// leaves on the stack.
pub fn main_object_0369<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; push ecx; push esi; sub esp, 0x10; call
    let at = img.rel32_target(f + 0x16);
    let call = Frame {
        at,
        esp: fr.esp.wrapping_sub(0x20),
        edx: w(0x11),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x08)))],
        ..*fr
    };
    crate::o6_fam_object::start_object_first(g, img, at, &call);
    let actor = g.read::<u32>(w(0x21));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x2c), w(0x31), w(0x36), actor]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1b), 0x418 / 4), 0);
    s.push(actor);
    // call width: 12 (Pushes caps the words at 12)
    let top = st0_f32(s.call(g, slot_target(g, w(0x3e), 0x30 / 4), 0).st0);
    g.write::<f32>(w(0x4a), top);
    let y = sub(g.read::<f32>(w(0x52)), k(g, 0x5a));
    let t333 = g.read::<u32>(w(0x5f));
    s.push(g.read::<u32>(w(0x65)));
    g.write::<u32>(w(0x6b), 0x3f80_0000);
    g.write::<f32>(w(0x77), y);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, g.read::<u32>(t333.wrapping_add(0x534)), 0);
    let x = sub(k(g, 0x8a), st0_f32(out.st0));
    if x > k(g, 0x9a) {
        g.write::<f32>(w(0xac), sub(x, k(g, 0xa4)));
    } else if k(g, 0xb8) > x {
        g.write::<f32>(w(0xcd), add(x, k(g, 0xc5)));
    } else {
        g.write::<f32>(w(0xcd), x);
    }
    Ret::eax(out.eax)
}

/// [`main_object_0369`] (`magic_0369` 0x10001370).
pub const MAIN_OBJECT_0369: Sig = Sig {
    len: 213,
    hash: 0x05e0_0f52_5dcc_0835,
    calls: &[(
        0x16,
        Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
    )],
    imports: &[],
};
