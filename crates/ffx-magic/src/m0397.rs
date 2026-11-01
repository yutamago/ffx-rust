//! `magic_0397.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task's end (0x10004250, cdecl `(node)`): the script runs
/// ([`run`](crate::sa_script::run), `+0x0c`); while the game is paused
/// slot 561's result is returned; else the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step), `+0x25`) and when
/// neither is 0 the effect's `+0x2408` (`*E`, `+0x07`) becomes 1, slot
/// 628 `(0)` runs and the node goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back), a tail jump, `+0x54`).
/// Returns the motion step's result otherwise.
pub fn script_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x07));
    let a = crate::sa_script::run(g, img, img.rel32_target(f + 0x0c));
    let s = slot_target(g, w(0x12), 561);
    let paused = g.call(s, 0, &[]).eax;
    if paused != 0 {
        return Ret::eax(paused);
    }
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x25));
    if a == 0 || m == 0 {
        return Ret::eax(m);
    }
    g.write::<u32>(e + 0x2408, 1);
    let s = slot_target(g, w(0x32), 628);
    g.call(s, 0, &[0]);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x54),
        node,
    ))
}

/// [`script_end`] (92 bytes, 1 overlay).
pub const SCRIPT_END: Sig = Sig {
    len: 92,
    hash: 0xdb50_4c0e_96fd_55b7,
    calls: &[
        (0x0c, Callee::Sig(&crate::sa_script::RUN)),
        (0x25, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x54, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// An actor's position (0x10005800, cdecl `(obj)`): the actor `i`
/// (`+0x120`; -1 is the caster `*X`, `+0x1b`, else the table `+0x2d` at
/// the byte `B[i]`, `B` = `*P`, `+0x22`); slot 262 `(a, v, v + 4, v + 8)`
/// and slot 268 `(a, 0x10, v)` with `v` = `obj + 0xd0`, then `v + 0xc`
/// = 1.0. Returns slot 268's `eax`.
pub fn actor_position<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let i = g.read::<u32>(obj + 0x120);
    let v = obj + 0xd0;
    let a = if i == u32::MAX {
        g.read::<u32>(w(0x1b))
    } else {
        let b = g.read::<u8>(g.read::<u32>(w(0x22)).wrapping_add(i));
        g.read::<u32>(w(0x2d).wrapping_add(4 * u32::from(b)))
    };
    let s = slot_target(g, w(0x3a), 0x418 / 4);
    g.call(s, 0, &[a, v, v + 4, v + 8]);
    let s = slot_target(g, w(0x49), 0x430 / 4);
    // call width: 7 (the earlier call's words under it)
    let eax = g.call(s, 0, &[a, 0x10, v, a, v, v + 4, v + 8]).eax;
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`actor_position`] (103 bytes, 1 overlay).
pub const ACTOR_POSITION: Sig = Sig {
    len: 103,
    hash: 0x874f_7ae0_52ca_7564,
    calls: &[],
    imports: &[],
};

/// The party check (0x100064b0, cdecl `(state)`, locals `n` at `ebp - 8`
/// and a byte at `ebp - 1`): slot 288 `(9, &n)` gives the party's bytes;
/// each one but the caster's own byte `*S` (`+0x21`) is asked slot 637
/// `(b)`, and any non-zero answer clears the result; with none the
/// state's byte `+0x56` becomes 1. Returns the state then, else the last
/// value the code left in `eax` (slot 637's answer, the list with its low
/// byte the last skipped byte, or the list).
pub fn party_check<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let n_at = ebp.wrapping_sub(8);
    let s = slot_target(g, w(0x0e), 0x480 / 4);
    let list = g.call(s, 0, &[9, n_at]).eax;
    let own = g.read::<u8>(w(0x21));
    g.write::<u8>(ebp.wrapping_sub(1), own);
    let mut eax = list;
    let mut ok = true;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(n_at) {
        let b = g.read::<u8>(list.wrapping_add(i));
        eax = (eax & !0xff) | u32::from(b);
        if b != own {
            let s = slot_target(g, w(0x44), 0x9f4 / 4);
            eax = g.call(s, 0, &[u32::from(b)]).eax;
            if eax != 0 {
                ok = false;
            }
        }
        i += 1;
    }
    if !ok {
        return Ret::eax(eax);
    }
    let state = fr.arg(g, 0);
    g.write::<u8>(state + 0x56, 1);
    Ret::eax(state)
}

/// [`party_check`] (120 bytes, 1 overlay).
pub const PARTY_CHECK: Sig = Sig {
    len: 120,
    hash: 0x10d6_f17f_1e4b_7a72,
    calls: &[],
    imports: &[],
};

/// The two sprite tasks (0x10006240, no arguments, no frame): with `E` =
/// `*G` (`+0x04`, read again for the second), the sprite blocks `E +
/// 0x19c0` and `E + 0x1a60` get their colour bytes and words and each a
/// pool node ([`take`](crate::sa_pool::take), owner the callback `+0x10`
/// / `+0xb6`) holding it (`+0x14`); the nodes go last on the task list
/// (first `+0x6d`, last `+0x83`/`+0x11c`), the first block's `+0x94`
/// becomes 1 between them. Returns the second node.
pub fn sprite_tasks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x04)).wrapping_add(0x19c0);
    g.write::<u32>(a + 0x80, 0);
    g.write::<u16>(a + 0x85, 0xffff);
    g.write::<u8>(a + 0x84, 0xff);
    g.write::<u8>(a + 0x87, 0x80);
    g.write::<u32>(a + 0x88, 0);
    g.write::<u16>(a + 0x98, 0x48);
    g.write::<u32>(a + 0x90, 0x1e);
    g.write::<u32>(a + 0x8c, 0x1e);
    g.write::<u32>(a + 0x94, 0);
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0x67), w(0x10));
    let mut head = g.read::<u32>(w(0x6d));
    g.write::<u32>(first + 0x14, a);
    g.write::<u32>(first + 4, 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x98), first);
    } else {
        let last = g.read::<u32>(w(0x83));
        g.write::<u32>(first, last);
        g.write::<u32>(last + 4, first);
    }
    g.write::<u32>(first + 8, first);
    g.write::<u32>(a + 0x94, 1);
    let b = g.read::<u32>(w(0xab)).wrapping_add(0x1a60);
    g.write::<u32>(b + 0x81, 0x0080_ffff);
    g.write::<u8>(b + 0x80, 0xff);
    g.write::<u16>(b + 0x86, 0);
    g.write::<u8>(b + 0x85, 0);
    g.write::<u32>(b + 0x88, 0x1f);
    g.write::<u16>(b + 0x98, 0x48);
    g.write::<u32>(b + 0x90, 0x3c);
    g.write::<u32>(b + 0x8c, 0x3c);
    g.write::<u32>(b + 0x94, 0);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0x10d), w(0xb6));
    g.write::<u32>(second + 0x14, b);
    g.write::<u32>(second + 4, 0);
    g.write::<u32>(w(0x11c), second);
    if head == 0 {
        // only for a null first node
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x139), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first + 4, second);
    }
    g.write::<u32>(second + 8, second);
    Ret::eax(second)
}

/// [`sprite_tasks`] (322 bytes, 1 overlay).
pub const SPRITE_TASKS: Sig = Sig {
    len: 322,
    hash: 0xb2d9_7460_5cac_88c2,
    calls: &[
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10d, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The camera's turn (0x10004560, no arguments): with `E` = `*G`
/// (`+0x02`) the count `E + 0x23fc`, when positive, drops; the target
/// angle `b` (`E + 0x23f8`) is taken round to the side of the start `a`
/// (`E + 0x23f4`) when they lie more than `PI` (`+0x33`) apart across 0
/// (`b - 2 PI` as `LO - (PI - b)`, `LO` `+0x2b`, or `b + PI + PI`); the
/// blend `(K - c) b + a c` with `c` = count times `S` (`K` `+0x85`, `S`
/// `+0x90`) wrapped once by `T` (`+0xa4`) into `[LO, PI]`, then `U - h`
/// (`U` `+0xc4`) wrapped the same way goes to slot 34 `(*X, angle)` (`X`
/// `+0xf3`). Returns slot 34's `eax`. Each comparison is `comiss` + `jbe`
/// (an unordered value takes no branch).
#[allow(clippy::many_single_char_names)] // the formula's names
pub fn camera_turn<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, mul, sub};
    let w = |off: u32| img.word(fr.at + off);
    let e = g.read::<u32>(w(0x02));
    let n = g.read::<i32>(e + 0x23fc);
    if n > 0 {
        g.write::<i32>(e + 0x23fc, n - 1);
    }
    let a = g.read::<f32>(e + 0x23f4);
    let mut b = g.read::<f32>(e + 0x23f8);
    let lo = g.read::<f32>(w(0x2b));
    let pi = g.read::<f32>(w(0x33));
    if 0.0 > a {
        if b > 0.0 && sub(b, a) > pi {
            b = sub(lo, sub(pi, b));
        }
    } else if 0.0 > b && sub(a, b) > pi {
        b = add(add(b, pi), pi);
    }
    let c = mul(cvt(g.read::<u32>(e + 0x23fc)), g.read::<f32>(w(0x90)));
    let k = sub(g.read::<f32>(w(0x85)), c);
    let ac = mul(a, c);
    let t = g.read::<f32>(w(0xa4));
    let wrap = |x: f32| {
        if x > pi {
            sub(x, t)
        } else if lo > x {
            add(x, t)
        } else {
            x
        }
    };
    let h = wrap(add(mul(k, b), ac));
    let angle = wrap(sub(g.read::<f32>(w(0xc4)), h));
    let s = slot_target(g, w(0xe1), 0x88 / 4);
    let x = g.read::<u32>(w(0xf3));
    Ret::eax(g.call(s, 0, &[x, angle.to_bits()]).eax)
}

/// [`camera_turn`] (253 bytes, 1 overlay).
pub const CAMERA_TURN: Sig = Sig {
    len: 253,
    hash: 0x384c_63e7_37de_96fd,
    calls: &[],
    imports: &[],
};

/// The party's mean position (0x10001120, internal, `ecx` = the output
/// quad; a /GS frame of 0x24 bytes): slot 288 `(0x13, &n)` gives the
/// actors' byte list `L` and their count `n` (`ebp - 0x1c`, read again
/// after each actor); each actor's slot 215 `(L[i])` handle goes to slot
/// 262 `(h, &x, &y, &z)` (on the frame, `ebp - 0x14`/`- 0x10`/`- 0xc`)
/// and the sums gain them; the output is the sums over `n` and 1.0 (the
/// divisions run for `n` 0 too). Returns the last slot's `eax`. Slot 262
/// passes the words the original leaves on the stack.
#[allow(clippy::many_single_char_names)] // the registers' roles
pub fn party_mean<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div};
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let out = fr.ecx;
    let count = ebp.wrapping_sub(0x1c);
    let (px, py, pz) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
    );
    let s = slot_target(g, w(0x18), 0x480 / 4);
    let mut eax = g.call(s, 0, &[0x13, count]).eax;
    let list = eax;
    let (mut sx, mut sy, mut sz) = (0.0f32, 0.0f32, 0.0f32);
    let mut i = 0i32;
    while i < g.read::<i32>(count) {
        let actor = u32::from(g.read::<u8>(list.wrapping_add(i.cast_unsigned())));
        let mut p = crate::pushes::Pushes::default();
        p.push(actor);
        let s = slot_target(g, w(0x56), 0x35c / 4);
        // call width: 12 (Pushes caps the words at 12)
        let h = p.call(g, s, 0).eax;
        p.push_all(&[pz, py, px, h]);
        let s = slot_target(g, w(0x70), 0x418 / 4);
        // call width: 12 (Pushes caps the words at 12)
        eax = p.call(g, s, 0).eax;
        sx = add(g.read::<f32>(px), sx);
        sy = add(g.read::<f32>(py), sy);
        sz = add(g.read::<f32>(pz), sz);
        i += 1;
    }
    let n = cvt(g.read::<u32>(count));
    g.write::<f32>(out, div(sx, n));
    g.write::<f32>(out + 4, div(sy, n));
    g.write::<f32>(out + 8, div(sz, n));
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`party_mean`] (246 bytes, 1 overlay).
pub const PARTY_MEAN: Sig = Sig {
    len: 246,
    hash: 0x811e_ea19_eccd_4571,
    calls: &[
        // __security_check_cookie
        (0xee, Callee::Any),
    ],
    imports: &[],
};

/// The camera point's step (0x10004660, cdecl `(obj)`, a /GS frame of
/// 0x78 bytes): below frame 0xc0 (`+0x118`) slot 237 `(*X, 1, 0, &P)`
/// (`X` `+0x35`, `P` on the frame at `ebp - 0x24`) puts `P.x` and `P.z`
/// into `+0xd0` and `+0xd8`; after it the matrix `M` (`ebp - 0x74`)
/// from slot 235 `(&M)` turned by slot 328 `(&M, &M, 0)` and slot 329
/// `(&M, &M, a)` (`a` = `E + 0x23f8`, `E` = `*G`, `+0x68`) carries `V` =
/// `(0, 0, -1.5, 1.0)` (`ebp - 0x14`) through slot 229 `(&R, &M, &V)`
/// into `R` (`ebp - 0x34`), slot 248 `(o, o, &R)` (`o` = `obj + 0xd0`)
/// and `o + 0xc` = 1.0. Returns the last slot's `eax`. The slot calls
/// pass the words the original leaves on the stack.
pub fn point_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let obj = fr.arg(g, 0);
    if g.read::<i32>(obj + 0x118) < 0xc0 {
        let at = ebp.wrapping_sub(0x24);
        let s = slot_target(g, w(0x25), 0x3b4 / 4);
        let x = g.read::<u32>(w(0x35));
        let eax = g.call(s, 0, &[x, 1, 0, at]).eax;
        let px = g.read::<u32>(at);
        g.write::<u32>(obj + 0xd0, px);
        let pz = g.read::<u32>(at + 8);
        g.write::<u32>(obj + 0xd8, pz);
        return Ret::eax(eax);
    }
    let e = g.read::<u32>(w(0x68));
    let v = ebp.wrapping_sub(0x14);
    let m = ebp.wrapping_sub(0x74);
    let r = ebp.wrapping_sub(0x34);
    g.write::<u32>(v, 0);
    let a = g.read::<u32>(e + 0x23f8);
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 8, 0xbfc0_0000);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let o = obj + 0xd0;
    let mut p = crate::pushes::Pushes::default();
    p.push(m);
    let s = slot_target(g, w(0x80), 0x3ac / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.set_top(0);
    p.push_all(&[m, m]);
    let s = slot_target(g, w(0xb9), 0x520 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.pop(2);
    p.set_top(a);
    p.push_all(&[m, m]);
    let s = slot_target(g, w(0xd8), 0x524 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[v, m, r]);
    let s = slot_target(g, w(0xf1), 0x394 / 4);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[r, o, o]);
    let s = slot_target(g, w(0x109), 0x3e0 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    g.write::<u32>(o + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`point_step`] (304 bytes, 1 overlay).
pub const POINT_STEP: Sig = Sig {
    len: 304,
    hash: 0xf826_791f_ba76_c074,
    calls: &[
        // __security_check_cookie
        (0x5f, Callee::Any),
        (0x128, Callee::Any),
    ],
    imports: &[],
};
