//! `magic_0511.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The fade's step (0x10002be0, an overlay table entry, no arguments):
/// with the count `n = *G_FX + 0x225c` (down by one while above 0) and
/// `t = n / D`, the four words `+0xea0..+0xeac` become
/// `(K - t) * A + t * B`. Returns the count.
pub fn fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let fx = g.read::<u32>(w(0x2));
    let mut n = g.read::<u32>(fx + 0x225c);
    if n.cast_signed() > 0 {
        n -= 1;
        g.write::<u32>(fx + 0x225c, n);
    }
    let n = g.read::<u32>(fx + 0x225c);
    let t = div(cvt(n), k(g, 0x2e));
    let u = sub(k(g, 0x26), t);
    let v = add(mul(u, k(g, 0x42)), mul(t, k(g, 0x3a)));
    for off in [0xeac, 0xea8, 0xea4, 0xea0] {
        g.write::<f32>(fx + off, v);
    }
    Ret::eax(n)
}

/// 0x10002be0 (107 bytes, 1 overlay).
pub const FADE: Sig = Sig {
    len: 107,
    hash: 0xfead_9591_f780_36b2,
    calls: &[],
    imports: &[],
};

/// Starts the effect's two timed parts (no arguments; also `magic_0589`
/// 0x10002980): the records `*G_FX + a` (white, 20 frames) and `+ b`
/// (yellow, `frames` frames, kind 0x15) get their
/// colours (bytes and halves written in the original's order, which
/// overlap), flags and counts; each gets a pool node ([`take`], owner the
/// callback at `+0x10`/`+0xb6`) holding it, the first appended to the
/// list (head `+0x6d`, tail `+0x83`) and the second linked after it (the
/// tail `+0x11c`). Returns the second node.
pub(crate) fn two_parts<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (a, b, frames): (u32, u32, u32),
) -> Ret {
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x4)).wrapping_add(a);
    g.write::<u32>(a + 0x80, 0);
    g.write::<u16>(a + 0x85, 0xffff);
    g.write::<u8>(a + 0x84, 0xff);
    g.write::<u8>(a + 0x87, 0x80);
    g.write::<u32>(a + 0x88, 0);
    g.write::<u16>(a + 0x98, 0x48);
    g.write::<u32>(a + 0x90, 0x14);
    g.write::<u32>(a + 0x8c, 0x14);
    g.write::<u32>(a + 0x94, 0);
    let n1 = take(g, img, img.rel32_target(f + 0x67), w(0x10));
    let mut head = g.read::<u32>(w(0x6d));
    g.write::<u32>(n1.wrapping_add(0x14), a);
    g.write::<u32>(n1.wrapping_add(4), 0);
    if head == 0 {
        head = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x98), n1);
    } else {
        let tail = g.read::<u32>(w(0x83));
        g.write::<u32>(n1, tail);
        g.write::<u32>(tail.wrapping_add(4), n1);
    }
    g.write::<u32>(n1.wrapping_add(8), n1);
    g.write::<u32>(a + 0x94, 1);
    let b = g.read::<u32>(w(0xab)).wrapping_add(b);
    g.write::<u32>(b + 0x81, 0x0080_ffff);
    g.write::<u8>(b + 0x80, 0xff);
    g.write::<u16>(b + 0x86, 0);
    g.write::<u8>(b + 0x85, 0);
    g.write::<u32>(b + 0x88, 0x15);
    g.write::<u16>(b + 0x98, 0x48);
    g.write::<u32>(b + 0x90, frames);
    g.write::<u32>(b + 0x8c, frames);
    g.write::<u32>(b + 0x94, 0);
    let n2 = take(g, img, img.rel32_target(f + 0x10d), w(0xb6));
    g.write::<u32>(n2.wrapping_add(0x14), b);
    g.write::<u32>(n2.wrapping_add(4), 0);
    g.write::<u32>(w(0x11c), n2);
    if head == 0 {
        // only when the first take found no node
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x139), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1.wrapping_add(4), n2);
    }
    g.write::<u32>(n2.wrapping_add(8), n2);
    Ret::eax(n2)
}

/// 0x10002c50 (an overlay table entry): [`two_parts`], the second 60
/// frames.
pub fn e2c50<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    two_parts(g, img, fr.at, (0x1ad0, 0x1b70, 0x3c))
}

/// 0x10002da0 (an overlay table entry): [`two_parts`], the second 30
/// frames.
pub fn e2da0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    two_parts(g, img, fr.at, (0x1ad0, 0x1b70, 0x1e))
}

/// 0x10002da0 (322 bytes, 1 overlay).
pub const E2DA0: Sig = Sig {
    len: 322,
    hash: 0x751a_b7b1_8758_d688,
    calls: &[
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10d, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// 0x10002c50 (322 bytes, 1 overlay).
pub const E2C50: Sig = Sig {
    len: 322,
    hash: 0x66b4_d8b1_9c65_5128,
    calls: &[
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10d, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// Starts the sprite effect (0x100042f0, an overlay table entry, no
/// arguments; a cookie frame): slot 219 `(b[0], 0)` (`b = *G_B`) gives a
/// model, slot 268 `(model, 14, &p)` its point into the local `p`
/// (`ebp - 0x14`); the sprite `*G_FX + 0x2030` set up with that colour
/// ([`sprite_init_colour`](crate::sb_misc::sprite_init_colour), owner
/// `S[0x28]`, `S = *G_S`) and its size and colour words; the counts
/// `+0x2260` 40 and `+0x2264` 75; a pool node ([`take`], owner `+0x8c`)
/// at the head of the list (`G` at `+0xab` 0: empty). Returns the node.
pub fn sprite_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = fr.esp.wrapping_sub(0x18);
    let s = g.read::<u32>(w(0x11));
    let fx = g.read::<u32>(w(0x18));
    let owner = g.read::<u32>(s + 0x28);
    let b = g.read::<u32>(w(0x21));
    let i = u32::from(g.read::<u8>(b));
    let t = crate::runtime::slot_target(g, w(0x2d), 219);
    let model = g.call(t, 0, &[i, 0]).eax;
    let sprite = fx.wrapping_add(0x2030);
    let t = crate::runtime::slot_target(g, w(0x47), 268);
    g.call(t, 0, &[model, 0xe, p, i, 0]);
    // the call's esp: the frame (0x14), three saves, five words, the
    // colour pointer pushed
    let esp = fr.esp.wrapping_sub(0x40);
    g.write::<u32>(esp + 4, p);
    crate::sb_misc::sprite_init_colour(g, esp, sprite, owner);
    g.write::<u32>(sprite + 0x224, 0x140);
    g.write::<u32>(sprite + 0x21c, 0x3e19_999a);
    g.write::<u32>(sprite + 0x218, 0x4000_0000);
    g.write::<u32>(sprite + 0x20c, 0x4040_4040);
    g.write::<u32>(fx + 0x2260, 0x28);
    g.write::<u32>(fx + 0x2264, 0x4b);
    let node = take(g, img, img.rel32_target(f + 0xa5), w(0x8c));
    let empty = g.read::<u32>(w(0xab)) == 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    crate::o4_more::push_head(g, node, empty, (w(0xbe), w(0xda)), (w(0xd0), w(0xda)));
    Ret::eax(node)
}

/// 0x100042f0 (234 bytes, 1 overlay).
pub const SPRITE_START: Sig = Sig {
    len: 234,
    hash: 0xed46_b3fa_b8ba_cba3,
    calls: &[
        (0x5c, Callee::Sig(&crate::sb_misc::SPRITE_INIT_COLOUR)),
        (0xa5, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0xe2, Callee::Any),
    ],
    imports: &[],
};

/// An object placed at the actor's point 0xe (0x10003df0, a callback,
/// cdecl `(obj)`): slot 219 `(*G_M as a byte, 0)` gives the model, slot
/// 268 `(model, 0xe, obj + 0xd0)` (with the first call's words under it).
/// Returns slot 268's `eax`.
pub fn place_at_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let m = u32::from(g.read::<u8>(g.read::<u32>(w(0x4))));
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[0, m]);
    let t = slot_target(g, w(0xf), 219);
    // call width: 12 (Pushes caps the words at 12)
    let model = p.call(g, t, 0).eax;
    let obj = fr.arg(g, 0);
    p.push_all(&[obj.wrapping_add(0xd0), 0xe, model]);
    let t = slot_target(g, w(0x29), 268);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, t, 0).eax)
}

/// [`place_at_point`] (58 bytes, 1 overlay).
pub const PLACE_AT_POINT: Sig = Sig {
    len: 58,
    hash: 0x032d_ec4b_d056_ea54,
    calls: &[],
    imports: &[],
};

/// Shows the effect's three models and ends (0x100020d0, an overlay
/// table entry, no arguments): slot 384 `(T[b[i]])` for `b = *G_B`'s
/// bytes 3, 4, 5 (operands `+0x1`/`+0xc`, `+0x1e`/`+0x29`,
/// `+0x3b`/`+0x46`); slots 25 `(k, 1)` and 24 `(k, 0)` for `k` 0x301a,
/// 0x301b, 0x301c; slot 271 `(1)` (each call with the earlier words on
/// the stack). The node `*G_N` (`+0xce`), when there is one, goes back to
/// the pool ([`give_back`](crate::sa_pool::give_back)) and `G_N` = 0;
/// the 320 part handles of `*G_FX + 0x2030` registered
/// ([`register_320`](crate::s4_small::register_320)), then the parts'
/// registration ([`register_parts`](crate::sa_misc::register_parts), a
/// tail jump) with that `eax`, whose answer it returns.
pub fn models_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = crate::pushes::Pushes::default();
    for (b_op, t_op, s_op, i) in [
        (0x1, 0xc, 0x11, 3),
        (0x1e, 0x29, 0x2e, 4),
        (0x3b, 0x46, 0x4b, 5),
    ] {
        let b = g.read::<u32>(w(b_op));
        let k = u32::from(g.read::<u8>(b + i));
        p.push(g.read::<u32>(w(t_op).wrapping_add(4 * k)));
        let t = slot_target(g, w(s_op), 384);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, fr.ecx);
    }
    for (k, ops) in [
        (0x301a, [0x58, 0x69]),
        (0x301b, [0x7a, 0x8b]),
        (0x301c, [0x9c, 0xad]),
    ] {
        for (op, (slot, v)) in ops.into_iter().zip([(25, 1), (24, 0)]) {
            p.push_all(&[v, k]);
            let t = slot_target(g, w(op), slot);
            // call width: 12 (Pushes caps the words at 12)
            p.call(g, t, fr.ecx);
        }
    }
    p.push(1);
    let t = slot_target(g, w(0xbe), 271);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, fr.ecx);
    let node = g.read::<u32>(w(0xce));
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0xda), node);
        g.write::<u32>(w(0xe0), 0);
    }
    let fx = g.read::<u32>(w(0xea));
    let reg = Frame {
        at: img.rel32_target(f + 0xf5),
        esp: fr.esp.wrapping_sub(4),
        // lea ecx, [ecx + D]: 0x2030 here, 0x1fd0 in magic_0589
        ecx: fx.wrapping_add(w(0xf0)),
        ..*fr
    };
    let eax = crate::s4_small::register_320(g, img, &reg).eax;
    Ret::eax(crate::sa_misc::register_parts(
        g,
        img,
        img.rel32_target(f + 0xfa),
        eax,
    ))
}

/// [`models_end`] (254 bytes; with [`MODELS_END_0589`] family
/// P072).
pub const MODELS_END: Sig = Sig {
    len: 254,
    hash: 0x77de_39eb_c872_9950,
    calls: &[
        (0xda, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xf5, Callee::Sig(&crate::s4_small::REGISTER_320)),
        (0xfa, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`models_end`] in `magic_0589` (0x10001e90: the handles at `+0x1fd0`).
pub const MODELS_END_0589: Sig = Sig {
    len: 254,
    hash: 0xfcc2_cf1c_9005_307d,
    calls: &[
        (0xda, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xf5, Callee::Sig(&crate::s4_small::REGISTER_320)),
        (0xfa, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// The effect block `*G_FX`'s point `+0x1a60` copied into an object's
/// `+0xd0..+0xdc`.
fn copy_point<G: Game>(g: &mut G, fx: u32, obj: u32) -> u32 {
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(fx + 0x1a60 + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, last);
    }
    last
}

/// Starts the object `*G_FX + 0xe40` (0x10002b40, an overlay table
/// entry, no arguments): [`start_object_anim`](crate::s4_misc::start_object_anim)
/// (scale `*K` `+0x4`, index 0xc, 0), the point `+0x1a60` copied to its
/// `+0xd0` with `y - *K` (`+0x60`), its `+0xc4` = `+0x2258`, its callback
/// `+0x104` the immediate `+0x79`; `+0x225c` = 40. Returns `+0x2258`.
pub fn object_e40<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = g.read::<f32>(w(0x4));
    let fx = g.read::<u32>(w(0xc));
    let obj = fx.wrapping_add(0xe40);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x20), obj, scale, 0xc, 0);
    copy_point(g, fx, obj);
    let y = sub(g.read::<f32>(obj + 0xd4), g.read::<f32>(w(0x60)));
    let a = g.read::<u32>(fx + 0x2258);
    g.write::<u32>(obj + 0xc4, a);
    g.write::<u32>(obj + 0x104, w(0x79));
    g.write::<f32>(obj + 0xd4, y);
    g.write::<u32>(fx + 0x225c, 0x28);
    Ret::eax(a)
}

/// [`object_e40`] (146 bytes, 1 overlay).
pub const OBJECT_E40: Sig = Sig {
    len: 146,
    hash: 0x9305_9829_6d30_1ab1,
    calls: &[(0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};

/// Starts the object `*G_FX + 0xd10` (0x10002ef0, an overlay table
/// entry, no arguments): [`start_object_anim`](crate::s4_misc::start_object_anim)
/// (scale `*K` `+0x4`, index 0xb, 0), the point `+0x1a60` copied to its
/// `+0xd0` with `y - *K` (`+0x4e`), its `+0xc4` = `+0x2258`, which it
/// returns.
pub fn object_d10<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = g.read::<f32>(w(0x4));
    let fx = g.read::<u32>(w(0xc));
    // the displacements: the object `+0x15`, the point `+0x26`, the
    // angle `+0x71` (0x1a60 and 0x2258 here, 0x1a00 and 0x21f8 in
    // magic_0589)
    let obj = fx.wrapping_add(w(0x15));
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x20), obj, scale, 0xb, 0);
    let point = fx.wrapping_add(w(0x26));
    for i in 0..4 {
        let v = g.read::<u32>(point + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, v);
    }
    let y = sub(g.read::<f32>(obj + 0xd4), g.read::<f32>(w(0x4e)));
    g.write::<f32>(obj + 0xd4, y);
    let a = g.read::<u32>(fx.wrapping_add(w(0x71)));
    g.write::<u32>(obj + 0xc4, a);
    Ret::eax(a)
}

/// [`object_d10`] (126 bytes; with [`OBJECT_D10_0589`] family P086).
pub const OBJECT_D10: Sig = Sig {
    len: 126,
    hash: 0x4687_1e94_feff_18af,
    calls: &[(0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};

/// [`object_d10`] in `magic_0589` (0x10002ad0).
pub const OBJECT_D10_0589: Sig = Sig {
    len: 126,
    hash: 0x6ace_e007_aed8_6b48,
    calls: &[(0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};

/// Restarts the object `*G_FX + 0xab0` (0x10003150, an overlay table
/// entry, no arguments): when the object list `+0x9` held `*G_FX +
/// 0xbe0` ([`list_remove`](crate::l5_objhost::list_remove)), `**G_P`
/// (`+0x1f`) = the immediate `+0x26`, slot 713 `(obj)` and the object's
/// flag bit 0x8000 (`+0x124`) cleared. Then
/// [`start_object_anim`](crate::s4_misc::start_object_anim) (scale `*K`
/// `+0x4a`, index 9, 0), the point `+0x1a60` copied to its `+0xd0`; its
/// `+0xc4` the angle `+0x2258 + K` wrapped once
/// ([`wrap_once`](crate::ob_model::wrap_once)), `+0xe8` = -5.0, its `y`
/// `y - K + *G_Y` (`+0xd9`, `+0xf4`). Returns the copied `w`.
pub fn object_ab0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x4));
    let old = fx.wrapping_add(0xbe0);
    if crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x16), w(0x9), old) != 0 {
        g.write::<u32>(g.read::<u32>(w(0x1f)), w(0x26));
        let t = slot_target(g, w(0x2b), 713);
        g.call(t, w(0x9), &[old]);
        let v = g.read::<u16>(old + 0x124);
        g.write::<u16>(old + 0x124, v & 0x7fff);
    }
    let scale = g.read::<f32>(w(0x4a));
    let obj = fx.wrapping_add(0xab0);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x5e), obj, scale, 9, 0);
    let last = copy_point(g, fx, obj);
    let a = add(g.read::<f32>(fx + 0x2258), g.read::<f32>(w(0x9e)));
    let a = crate::ob_model::wrap_once(g, img, f, a, [0xa8, 0xb2, 0xbc, 0xc9]);
    let y = sub(g.read::<f32>(obj + 0xd4), g.read::<f32>(w(0xd9)));
    g.write::<f32>(obj + 0xc4, a);
    g.write::<u32>(obj + 0xe8, 0xc0a0_0000);
    let y = add(y, g.read::<f32>(w(0xf4)));
    g.write::<f32>(obj + 0xd4, y);
    Ret::eax(last)
}

/// [`object_ab0`] (258 bytes, 1 overlay).
pub const OBJECT_AB0: Sig = Sig {
    len: 258,
    hash: 0x2317_3990_418c_1c5c,
    calls: &[
        (0x16, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x5e, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};

/// Starts the object `*G_FX + 0x5f0` (0x10003e30, an overlay table
/// entry, no arguments): [`start_object_anim`](crate::s4_misc::start_object_anim)
/// (scale `*K` `+0x4`, index 5, 0), its `+0x6b4` = `+0x2258` and its
/// callback `+0x6f4` the immediate `+0x38`. Returns `+0x2258`.
pub fn object_5f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = g.read::<f32>(w(0x4));
    let fx = g.read::<u32>(w(0xb));
    crate::s4_misc::start_object_anim(
        g,
        img,
        img.rel32_target(f + 0x1e),
        fx.wrapping_add(0x5f0),
        scale,
        5,
        0,
    );
    let a = g.read::<u32>(fx + 0x2258);
    g.write::<u32>(fx + 0x6b4, a);
    g.write::<u32>(fx + 0x6f4, w(0x38));
    Ret::eax(a)
}

/// [`object_5f0`] (62 bytes, 1 overlay).
pub const OBJECT_5F0: Sig = Sig {
    len: 62,
    hash: 0x75bd_0fd4_5734_3ba3,
    calls: &[(0x1e, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};
