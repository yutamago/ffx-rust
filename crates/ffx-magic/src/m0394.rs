//! `magic_0394.dll`'s own functions (besides the runtime and the object
//! host): the effect's state block `*G_STATE` (the caster's position
//! `+0xb30`, the summon's `+0xb40`, its goal `+0xb50`, the flight's
//! countdown `+0xc88`), the summon's placement and flight, its model
//! frame and the callbacks the effect stores.

use crate::l5_objhost::ONE;
use crate::ob_model::{ModelOps, RootOps, Who, root_matrix, scale_unit, st0_f32, wrap_once};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

// (the task step and the afterimage callbacks: `ob_summon`)

/// The battle character the selection's first byte names, at these
/// operand offsets.
const fn selected(sel: u32, table: u32) -> Who {
    Who::Sel { sel, at: 0, table }
}

/// The selected character's root matrix into `m` (0x100014e0, `edx` =
/// `m`; `esp` the entry stack pointer): [`root_matrix`] with the
/// selection's byte as the battle index and the local at `ebp - 0x14`
/// for its position. Returns slot 211's `eax`. (The frame's stack cookie
/// is the CRT's; nothing else reads it.)
pub fn root<G: Game>(g: &mut G, img: &Image, f: u32, m: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let sel = g.read::<u32>(w(0x11));
    let i = u32::from(g.read::<u8>(sel));
    let chr = g.read::<u32>(w(0x1e).wrapping_add(4 * i));
    let ops = RootOps {
        s210: 0x29,
        off: 0x3e,
        s217: 0x4f,
        s255: 0x61,
        s211: 0x72,
    };
    let pos = esp.wrapping_sub(4).wrapping_sub(0x14);
    root_matrix(g, img, f, &ops, (chr, m, pos), |_| i).1
}

/// [`root`] (148 bytes, 1 overlay).
pub const ROOT: Sig = Sig {
    len: 148,
    hash: 0x1c30_0775_8b53_3d3b,
    // __security_check_cookie
    calls: &[(0x8c, Callee::Any)],
    imports: &[],
};

/// The selected character's scaled unit matrix into `m` (0x10001580,
/// `edx` = `m`): [`scale_unit`] with no earlier words. Returns slot 17's
/// `eax`.
pub fn scaled<G: Game>(g: &mut G, img: &Image, f: u32, m: u32) -> u32 {
    let ops = ModelOps {
        s235: 0x05,
        c277: selected(0x17, 0x21),
        s277: 0x26,
        k: 0x3e,
        c: selected(0x51, 0x5b),
        s15: 0x60,
        s16: 0x77,
        s17: 0x93,
        s330: 0,
        s328: 0,
        s329: 0,
    };
    scale_unit(g, img, f, &ops, m, &[]).1
}

/// [`scaled`] (189 bytes, 1 overlay).
pub const SCALED: Sig = Sig {
    len: 189,
    hash: 0xd688_346b_0c23_8281,
    calls: &[],
    imports: &[],
};

/// Places the summon (0x100048e0, no arguments; `esp` the entry stack
/// pointer): the caster's position (slot 262 `(*G_CASTER, &+0xb30..)`,
/// `w` = 1) and facing `A - Ch_GetRot` wrapped once into `+0xc84`; the
/// selected character's position into `+0xb40` (`w` = 1), moved by `(0,
/// 0, d + 10)` turned by X 0 and Y the facing (slots 235, 328, 329 into
/// the matrix at `ebp - 0x64`, slot 229 into `ebp - 0x24`, slot 248),
/// where `d` is the battle character's width (slot 251, battle on and
/// index below 32) or its thickness times its Z scale (slots 19, 17);
/// `+0xc8c` = -1, `+0xc94` = 0 and two flags. Returns slot 248's `eax`.
/// (The frame's stack cookie is the CRT's; nothing else reads it.)
pub fn place<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let s = g.read::<u32>(w(0x13));
    let p = s + 0xb30;
    let caster = g.read::<u32>(w(0x2f));
    let s262 = slot_target(g, w(0x28), 262);
    let mut stack = vec![caster, p, p + 4, p + 8];
    g.call(s262, 0, &stack);
    g.write::<u32>(p + 0xc, ONE);
    let s333 = slot_target(g, w(0x3c), 333);
    stack.insert(0, g.read::<u32>(w(0x49)));
    // call width: 9 at most (the earlier calls' words stay on the stack)
    let rot = st0_f32(g.call(s333, 0, &stack).st0);
    let x = g.read::<f32>(w(0x5c)) - rot;
    let x = wrap_once(g, img, f, x, [0x6b, 0x75, 0x7f, 0x8c]);
    g.write::<f32>(s + 0xc84, x);
    let sel = g.read::<u32>(w(0x99));
    let q = s + 0xb40;
    let k = u32::from(g.read::<u8>(sel));
    let chr = g.read::<u32>(w(0xa9).wrapping_add(4 * k));
    let s262 = slot_target(g, w(0xb6), 262);
    g.call(s262, 0, &[chr, q, q + 4, q + 8]);
    g.write::<u32>(q + 0xc, ONE);
    let ang = g.read::<u32>(s + 0xc84);
    let sel = g.read::<u32>(w(0xd5));
    let v = l(0x14);
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    let i = u32::from(g.read::<u8>(sel));
    let d = if g.read::<u32>(w(0xde)) == 0 && i < 0x20 {
        let s251 = slot_target(g, w(0x102), 251);
        st0_f32(g.call(s251, 0, &[i]).st0)
    } else {
        let i = u32::from(g.read::<u8>(sel));
        let c = g.read::<u32>(w(0x121).wrapping_add(4 * i));
        let s19 = slot_target(g, w(0x126), 19);
        let th = st0_f32(g.call(s19, 0, &[c]).st0);
        let s17 = slot_target(g, w(0x134), 17);
        let sz = st0_f32(g.call(s17, 0, &[c, c]).st0);
        sz * th
    };
    let d = d + g.read::<f32>(w(0x152));
    let (mat, out) = (l(0x64), l(0x24));
    g.write::<u32>(l(0x8), ONE);
    g.write::<f32>(l(0xc), d);
    let s235 = slot_target(g, w(0x15b), 235);
    g.call(s235, 0, &[mat]);
    let s328 = slot_target(g, w(0x180), 328);
    g.call(s328, 0, &[mat, mat, 0]);
    let s329 = slot_target(g, w(0x19f), 329);
    g.call(s329, 0, &[mat, mat, ang]);
    g.write::<u32>(l(0x8), ONE);
    let s229 = slot_target(g, w(0x1b8), 229);
    let mut stack = vec![out, mat, v, mat, mat, ang];
    g.call(s229, 0, &stack);
    let s248 = slot_target(g, w(0x1d0), 248);
    stack.splice(0..0, [q, q, out]);
    // call width: 9 (the earlier calls' words stay on the stack)
    let r = g.call(s248, 0, &stack).eax;
    g.write::<u32>(q + 0xc, ONE);
    g.write::<u32>(s + 0xc8c, 0xffff_ffff);
    g.write::<u32>(s + 0xc94, 0);
    g.write::<u32>(w(0x205), 1);
    g.write::<u32>(w(0x20f), 0xffff_ffff);
    r
}

/// [`place`] (545 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 545,
    hash: 0xfffa_3d77_d03b_633b,
    // __security_check_cookie
    calls: &[(0x219, Callee::Any)],
    imports: &[],
};

/// One axis of the flight (`a` the start, `b` the goal; `k` the
/// constants 4, 1.05, 4, 0.5, 4, 0.1 as the axis reads them): with `d =
/// b - a`, the sum of `(ut uu 4)(d 1.05)`, `uu uu d`, `(tt uu 4)(d 0.5)`,
/// `(tt ut 4)(d 0.1)`, `tt tt 0` and `a`, rounded to `f32` at every step
/// in the original's order.
#[allow(clippy::similar_names)]
fn flight_axis(a: f32, b: f32, t: f32, u: f32, k: [f32; 6]) -> f32 {
    let [k4a, k105, k4b, k05, k4c, k01] = k;
    let (ut, uu, tt) = (u * t, u * u, t * t);
    let d = b - a;
    let mut p = ut * uu * k4a * (d * k105);
    p += uu * uu * d;
    p += tt * uu * k4b * (d * k05);
    p += tt * ut * k4c * (d * k01);
    p += tt * tt * 0.0;
    p + a
}

/// The flight's per-frame step (0x10005ad0, cdecl `(obj)`; `esp` the
/// entry stack pointer): the countdown `+0xc88` drops (while positive),
/// `t = count / 14`, `u = 1 - t`; the summon's position is the curve
/// from `+0xb40` to `+0xb50` ([`flight_axis`]), set on the battle
/// character (slot 255 `(index, &pos)` through the locals at `ebp -
/// 0x14`, battle on and index below 32) or the summon (slot 33 `(chr, x,
/// y, z)`, or slot 32 `(chr, x, z)` when slot 11 says it keeps to the
/// ground); at the end `obj + 0x56` = 1. Returns the position slot's
/// `eax`. (The frame's stack cookie is the CRT's.)
pub fn flight<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let kf = |g: &G, op: u32| g.read::<f32>(w(op));
    let ebp = fr.esp.wrapping_sub(4);
    let obj = fr.arg(g, 0);
    let s = g.read::<u32>(w(0x17));
    let n = g.read::<u32>(s + 0xc88).cast_signed();
    if n > 0 {
        g.write::<u32>(s + 0xc88, (n - 1).cast_unsigned());
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps rounds the same
    let t = g.read::<u32>(s + 0xc88).cast_signed() as f32 / kf(g, 0x43);
    let u = kf(g, 0x3b) - t;
    let at = |g: &G, o: u32| g.read::<f32>(s + o);
    let x = flight_axis(
        at(g, 0xb40),
        at(g, 0xb50),
        t,
        u,
        [0x5f, 0x94, 0x5f, 0xc3, 0x5f, 0xcb].map(|o| kf(g, o)),
    );
    let y = flight_axis(
        at(g, 0xb44),
        at(g, 0xb54),
        t,
        u,
        [0x141, 0x154, 0x149, 0x172, 0x196, 0x17a].map(|o| kf(g, o)),
    );
    let z = flight_axis(
        at(g, 0xb48),
        at(g, 0xb58),
        t,
        u,
        [0x1d9, 0x20d, 0x1d9, 0x22b, 0x1d9, 0x233].map(|o| kf(g, o)),
    );
    let pos = ebp - 0x14;
    g.write::<f32>(pos, x);
    g.write::<f32>(pos + 4, y);
    g.write::<f32>(pos + 8, z);
    let i = g.read::<u32>(w(0x270));
    let r = if g.read::<u32>(w(0x1ab)) == 0 && i.cast_signed() < 0x20 {
        let s255 = slot_target(g, w(0x27f), 255);
        g.call(s255, 0, &[i, pos]).eax
    } else {
        let s11 = slot_target(g, w(0x291), 11);
        let c = g.read::<u32>(w(0x298));
        let grav = g.call(s11, 0, &[c]).eax;
        let table = w(0x2ad);
        if grav == 0 {
            let s33 = slot_target(g, table, 33);
            g.call(s33, 0, &[c, x.to_bits(), y.to_bits(), z.to_bits()])
                .eax
        } else {
            let s32 = slot_target(g, table, 32);
            g.call(s32, 0, &[c, x.to_bits(), z.to_bits()]).eax
        }
    };
    if g.read::<u32>(s + 0xc88).cast_signed() <= 0 {
        g.write::<u8>(obj + 0x56, 1);
    }
    Ret::eax(r)
}

/// [`flight`] (796 bytes, 1 overlay).
pub const FLIGHT: Sig = Sig {
    len: 796,
    hash: 0xde0b_227d_806b_56dd,
    // __security_check_cookie
    calls: &[(0x314, Callee::Any)],
    imports: &[],
};

/// A task's end (0x10004800, cdecl `(node)`): with `E` = `*G` (`+0x07`,
/// read first) the 0x30-byte block `B` (`+0x10`, `memset`) is cleared,
/// its halfword `*H` (`+0x21`) gets 0x80 and `*W` (`+0x2e`) the word `*V`
/// (`+0x26`); the script runs ([`run`](crate::sa_script::run), `+0x33`);
/// while the game is paused (slot 561, called through `ecx`) its result
/// is returned; else the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step), `+0x4c`) and when
/// neither is 0 `E + 0xc94` becomes 1, slot 628 `(0)` runs, its `eax`
/// goes to [`register_parts`](crate::sa_misc::register_parts) (`+0x75`)
/// and the node goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back), a tail jump, `+0x80`).
/// Returns the motion step's result otherwise.
pub fn script_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x07));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), h | 0x80);
    let v = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x2e), v);
    let a = crate::sa_script::run(g, img, img.rel32_target(f + 0x33));
    let s = slot_target(g, w(0x39), 0x8c4 / 4);
    let paused = g.call(s, s, &[]).eax;
    if paused != 0 {
        return Ret::eax(paused);
    }
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x4c));
    if a == 0 || m == 0 {
        return Ret::eax(m);
    }
    let s = slot_target(g, w(0x59), 0x9d0 / 4);
    g.write::<u32>(e + 0xc94, 1);
    let r = g.call(s, 0, &[0]).eax;
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x75), r);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x80),
        node,
    ))
}

/// [`script_end`] (136 bytes, 1 overlay).
pub const SCRIPT_END: Sig = Sig {
    len: 136,
    hash: 0x711b_7f78_b669_05c3,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x33, Callee::Sig(&crate::sa_script::RUN)),
        (0x4c, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x75, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (0x80, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
