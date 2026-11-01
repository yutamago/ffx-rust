//! `magic_0250.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (0x10004940, cdecl `(task)`, an aligned frame): `ok` when
/// every one of the `*G_N + 1` records (operand `+0x08`; 0xb4 bytes from
/// the immediate `+0x1b`) has bit 15 of its halfword set; the motion
/// script of the record at `+0x38`
/// ([`motion_script`](crate::s4_misc::motion_script)) and the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both give 1
/// and `ok`, the task's state is 3, its record's `+0x10` the immediate
/// `+0x65`, the square registered
/// ([`register_square`](crate::s4_small::register_square)) and the grid
/// ([`register_grid`](crate::sa_misc::register_grid)). Returns the grid's
/// result, else the objects'.
pub fn grid_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; push esi; push edi
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
    let n = g.read::<u32>(w(0x8)).wrapping_add(1).cast_signed();
    let mut ok = true;
    let mut rec = w(0x1b);
    for _ in 0..n.max(0) {
        if g.read::<u16>(rec) & 0x8000 == 0 {
            ok = false;
            break;
        }
        rec = rec.wrapping_add(0xb4);
    }
    let m = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x3d), w(0x38));
    let r = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x44), esp.wrapping_sub(4));
    if r != 1 || !ok || m != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0x65));
    let sq = Frame {
        at: img.rel32_target(f + 0x6a),
        esp: esp.wrapping_sub(4),
        ..*fr
    };
    crate::s4_small::register_square(g, img, &sq);
    Ret::eax(crate::sa_misc::register_grid(
        g,
        img,
        img.rel32_target(f + 0x6f),
    ))
}

/// [`grid_step`] (121 bytes, 1 overlay).
pub const GRID_STEP: Sig = Sig {
    len: 121,
    hash: 0xbddf_fbae_39df_9e2c,
    calls: &[
        (0x3d, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x44, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x6a, Callee::Sig(&crate::s4_small::REGISTER_SQUARE)),
        (0x6f, Callee::Sig(&crate::sa_misc::REGISTER_GRID)),
    ],
    imports: &[],
};

/// A spark from an actor (0x100049c0, cdecl `(p)`, a /GS frame): nothing
/// while bit 0 of `p[0xa4]` is set (the cookie's word in `eax`); else a
/// ring entry `E` ([`ring_take`](crate::sa_misc::ring_take); none: 0)
/// gets: `+0x0` = `*G_A + 0xc` (operand `+0x4a`), `+0x4` 1, the halfwords
/// `+0x8`/`+0xa` 0xffff/0x200, `+0xc..+0x14` the point slot 217 `(p[0xa0],
/// &P)` gives (`P` at `ebp - 0x14`; `y` less `K` at `+0x85`), `+0x18`
/// 1.0, `+0x1c`/`+0x24` 0, `+0x20` -0.2, `+0x28` slot 314 `(pi)` (ST0),
/// `+0x2c` (slot 404 `(0.4)` + `A`) `M` (`+0xe6`, `+0x106`), `+0x34`
/// 0x10, `+0x38` 8, `+0x3c` -8, bytes `+0x43` 0x40 and `+0x47` 0. Returns
/// slot 404's `eax`.
pub fn spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let p = fr.arg(g, 0);
    if g.read::<u8>(p + 0xa4) & 1 != 0 {
        return Ret::eax(cookie);
    }
    let e = crate::sa_misc::ring_take(g, img, img.rel32_target(f + 0x23));
    if e == 0 {
        return Ret::eax(0);
    }
    let pt = ebp.wrapping_sub(0x14);
    let t = slot_target(g, w(0x33), 217);
    let a = g.read::<u32>(p + 0xa0);
    g.call(t, 0, &[a, pt]);
    let v = g.read::<u32>(g.read::<u32>(w(0x4a)) + 0xc);
    g.write::<u32>(e, v);
    g.write::<u32>(e + 4, 1);
    g.write::<u16>(e + 8, 0xffff);
    g.write::<u16>(e + 0xa, 0x200);
    let x = g.read::<u32>(pt);
    g.write::<u32>(e + 0xc, x);
    let y = sub(g.read::<f32>(pt + 4), k(g, 0x85));
    g.write::<f32>(e + 0x10, y);
    let z = g.read::<u32>(pt + 8);
    g.write::<u32>(e + 0x14, z);
    g.write::<u32>(e + 0x18, 0x3f80_0000);
    g.write::<u32>(e + 0x1c, 0);
    g.write::<u32>(e + 0x20, 0xbe4c_cccd);
    g.write::<u32>(e + 0x24, 0);
    // the call's word left on the stack (`ebp - 0x24`) takes each argument
    let arg = ebp.wrapping_sub(0x24);
    g.write::<u32>(arg, 0x4049_0fdb);
    let t = slot_target(g, w(0x73), 314);
    let s = g.call(t, 0, &[0x4049_0fdb]).st0;
    g.write::<f32>(e + 0x28, crate::m0018::st0_f32(s));
    g.write::<u32>(arg, 0x3ecc_cccd);
    let t = slot_target(g, w(0xc7), 404);
    let o = g.call(t, 0, &[0x3ecc_cccd]);
    let r = crate::m0018::st0_f32(o.st0);
    g.write::<f32>(ebp.wrapping_sub(0x18), r);
    g.write::<u32>(e + 0x34, 0x10);
    g.write::<u32>(e + 0x38, 8);
    g.write::<u32>(e + 0x3c, 0xffff_fff8);
    g.write::<u8>(e + 0x43, 0x40);
    g.write::<u8>(e + 0x47, 0);
    g.write::<f32>(e + 0x2c, mul(add(r, k(g, 0xe6)), k(g, 0x106)));
    Ret::eax(o.eax)
}

/// [`spark`] (295 bytes, 1 overlay; the `rel32` at `+0x11f` is
/// `__security_check_cookie`).
pub const SPARK: Sig = Sig {
    len: 295,
    hash: 0xc60b_4689_1979_b0ce,
    calls: &[
        (0x23, Callee::Sig(&crate::sa_misc::RING_TAKE)),
        (0x11f, Callee::Any),
    ],
    imports: &[],
};
