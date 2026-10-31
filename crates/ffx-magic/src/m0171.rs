//! `magic_0171.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Image, Sig};

/// The actor block (0x10002a00, internal: `edx` = the block, cdecl
/// `(a)`): `{a, -1, *ME, model (slot 413 in a battle, else 215), -1, 0,
/// 1, u16 battle flag}` as [`me_block`](crate::sa_misc::me_block) fills
/// its static one (`ME` at `+0x13`, the battle word `+0x1c`/`+0x57`).
/// Returns the model in a battle, else 0.
pub fn actor_block<G: crate::Game>(g: &mut G, img: &Image, f: u32, block: u32, a: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(block, a);
    g.write::<u32>(block + 4, u32::MAX);
    let me = g.read::<u32>(w(0x13));
    g.write::<u32>(block + 8, me);
    let battle = g.read::<u32>(w(0x1c)) != 0;
    let t = slot_target(g, w(0x23), if battle { 413 } else { 215 });
    let m = g.call(t, 0, &[me]).eax;
    g.write::<u32>(block + 0xc, m);
    g.write::<u32>(block + 0x10, u32::MAX);
    g.write::<u32>(block + 0x14, 0);
    g.write::<u32>(block + 0x18, 1);
    if g.read::<u32>(w(0x57)) != 0 {
        g.write::<u16>(block + 0x1c, 1);
        m
    } else {
        g.write::<u16>(block + 0x1c, 0);
        0
    }
}

/// [`actor_block`] (110 bytes, 1 overlay).
pub const ACTOR_BLOCK: Sig = Sig {
    len: 110,
    hash: 0xba9b_9b12_0a67_bbed,
    calls: &[],
    imports: &[],
};

/// The angles from `from` to `to` (0x100039a0, `ecx` = `out`, `edx` =
/// `from`, cdecl `(to)`; [`m0396::aim`](crate::m0396::aim)'s kin): the
/// pitch `atan2(dy, sqrt(dz dz + dx dx))` into `out.x`; the yaw `y = A -
/// atan2(dz, dx)` (`A` at `+0xbc`) brought back by `B` (`+0xd5`/`+0xfb`)
/// once when above `H` (`+0xcb`) or below `L` (`+0xee`) into `out.y`;
/// `out.z` = 0. The square root is `_libm_sse2_sqrt_precise`, the arcs
/// `_CIatan2`, each rounded to `f32`. (`eax` is what `_CIatan2` leaves:
/// the port gives `to`.)
pub fn aim<G: crate::Game>(g: &mut G, img: &Image, f: u32, out: u32, from: u32, to: u32) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, p: u32, k: u32| g.read::<f32>(p + 4 * k);
    let dy = sub(r(g, to, 1), r(g, from, 1));
    let dx = sub(r(g, to, 0), r(g, from, 0));
    let dz = sub(r(g, to, 2), r(g, from, 2));
    let s = add(mul(dz, dz), mul(dx, dx));
    let h = f64::from(s).sqrt();
    #[allow(clippy::cast_possible_truncation)]
    let pitch = g.atan2(f64::from(dy), h) as f32;
    g.write::<f32>(out, pitch);
    #[allow(clippy::cast_possible_truncation)]
    let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let y = sub(g.read::<f32>(w(0xbc)), yaw);
    let y = if y > g.read::<f32>(w(0xcb)) {
        sub(y, g.read::<f32>(w(0xd5)))
    } else if g.read::<f32>(w(0xee)) > y {
        add(y, g.read::<f32>(w(0xfb)))
    } else {
        y
    };
    g.write::<f32>(out + 4, y);
    g.write::<u32>(out + 8, 0);
    to
}

/// [`aim`] (272 bytes, 1 overlay).
pub const AIM: Sig = Sig {
    len: 272,
    hash: 0xfe5c_16a6_9bb7_6c27,
    calls: &[
        (0x5a, crate::Callee::Import("_libm_sse2_sqrt_precise")),
        (0x6d, crate::Callee::Import("_CIatan2")),
        (0xaa, crate::Callee::Import("_CIatan2")),
    ],
    imports: &[],
};
