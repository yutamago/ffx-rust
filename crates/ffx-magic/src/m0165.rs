//! `magic_0165.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The camera's look-at (no arguments, a /GS frame): slot 327 `(m, M)`
/// copies the matrix `M` (the immediate `+0x1b`) into a local `m`
/// (`ebp - 0x54`), slot 229 `(v, m, P + 0x9f0)` transforms the object's point
/// (`P` = `*` operand `+0x13`) into `v` (`ebp - 0x14`). Returns slot
/// 229's `eax`; `v` for the caller.
fn look_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> (u32, [f32; 3]) {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let p = g.read::<u32>(w(0x13));
    let s = slot_target(g, w(0x21), 327);
    g.call(s, 0, &[m, w(0x1b)]);
    let s = slot_target(g, w(0x3d), 229);
    let eax = g.call(s, 0, &[v, m, p + 0x9f0, m, w(0x1b)]).eax;
    let r = [0, 4, 8].map(|o| g.read::<f32>(v + o));
    (eax, r)
}

/// 0x10004890 (no arguments): [`look_point`], then the globals `+0x63`,
/// `+0x78`, `+0x8d` = `v.x + A`, `v.y - B`, `v.z + C` (`+0x52`, `+0x70`,
/// `+0x85`). Returns slot 229's `eax`.
pub fn look_offset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (eax, [x, y, z]) = look_point(g, img, fr);
    let fl = |g: &G, off: u32| g.read::<f32>(w(off));
    let a = add(x, fl(g, 0x52));
    g.write::<u32>(w(0x63), a.to_bits());
    let b = sub(y, fl(g, 0x70));
    g.write::<u32>(w(0x78), b.to_bits());
    let c = add(z, fl(g, 0x85));
    g.write::<u32>(w(0x8d), c.to_bits());
    Ret::eax(eax)
}

/// [`look_offset`] (154 bytes, 1 overlay; the `rel32` at `+0x92` is
/// `__security_check_cookie`).
pub const LOOK_OFFSET: Sig = Sig {
    len: 154,
    hash: 0xa1f5_6551_2c50_0e90,
    calls: &[(0x92, Callee::Any)],
    imports: &[],
};

/// 0x100047d0 (no arguments): [`look_point`], then two points from it
/// into the globals `+0x87`.. and `+0x9f`..: `(v.x - A, v.y - B, v.z +
/// C)` and `(v.x - A, v.y - B, v.z + D)` (`+0x5c`, `+0x64`, `+0x72`,
/// `+0x7a`). Returns slot 229's `eax`.
pub fn look_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (eax, [x, y, z]) = look_point(g, img, fr);
    let fl = |g: &G, off: u32| g.read::<f32>(w(off));
    let a = sub(x, fl(g, 0x5c));
    let b = sub(y, fl(g, 0x64));
    let c = add(z, fl(g, 0x72));
    let d = add(z, fl(g, 0x7a));
    for (at, v) in [
        (0x87, a),
        (0x8f, b),
        (0x97, c),
        (0x9f, a),
        (0xa7, b),
        (0xaf, d),
    ] {
        g.write::<u32>(w(at), v.to_bits());
    }
    Ret::eax(eax)
}

/// [`look_pair`] (189 bytes, 1 overlay; the `rel32` at `+0xb5` is
/// `__security_check_cookie`).
pub const LOOK_PAIR: Sig = Sig {
    len: 189,
    hash: 0x547c_fd97_c2bb_1c01,
    calls: &[(0xb5, Callee::Any)],
    imports: &[],
};

/// 0x10004ac0 (no arguments): the object `P` (`*` operand `+0x02`): its
/// colour `+0x3f0..+0x3fc` 0.26, `+0x460..+0x46c` = `+0x9f0..+0x9fc`
/// with `+0x464` less `K` (`+0x58`), `+0x478` = 140.0. Returns `P[0x9fc]`.
pub fn copy_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x02));
    for off in [0x3fc, 0x3f8, 0x3f4, 0x3f0] {
        g.write::<u32>(p + off, 0x3e85_1eb8);
    }
    g.write::<u32>(p + 0x460, g.read::<u32>(p + 0x9f0));
    g.write::<u32>(p + 0x464, g.read::<u32>(p + 0x9f4));
    let y = sub(g.read::<f32>(p + 0x464), g.read::<f32>(w(0x58)));
    g.write::<u32>(p + 0x468, g.read::<u32>(p + 0x9f8));
    let last = g.read::<u32>(p + 0x9fc);
    g.write::<u32>(p + 0x46c, last);
    g.write::<u32>(p + 0x464, y.to_bits());
    g.write::<u32>(p + 0x478, 0x430c_0000);
    Ret::eax(last)
}

/// [`copy_point`] (129 bytes, 1 overlay).
pub const COPY_POINT: Sig = Sig {
    len: 129,
    hash: 0x394b_a29f_54de_c669,
    calls: &[],
    imports: &[],
};

/// The sounds' fade (0x100035a0, cdecl `(task)`): the task's countdown
/// `+0x30` drops while positive; then for each of the `*N` (`+0x1e`)
/// sounds: the volume `P[0x5eec + 4 i] * count / 60` (`P` = `*` operand
/// `+0x09`; 32-bit products, the division truncating) for the byte `b`
/// of the list `*L` (`+0x56`): with `*M` (`+0x4f`) set, slot 249
/// `(table[b], volume)` with a negative volume as 0x100 (`table` at
/// `+0x9b`); else slot 530 `(b, volume)` below 0x20, slot 249
/// `(table[b], volume)` from 0x20 on. Returns the last slot's `eax`, or
/// the task with no sounds.
///
/// The original also tests the sound index against -1 (taking `*X`,
/// operands `+0x5d` and `+0x8e`, instead of the list's byte); the index
/// runs from 0 up to `*N`, so those paths never run.
pub fn sound_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let task = fr.arg(g, 0);
    let p = g.read::<u32>(w(0x09));
    let mut count = g.read::<i32>(task + 0x30);
    if count > 0 {
        count -= 1;
        g.write::<i32>(task + 0x30, count);
    }
    let mut eax = task;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x1e)) {
        let v = g
            .read::<i32>(p.wrapping_add(0x5eec + 4 * i))
            .wrapping_mul(count)
            / 60;
        let list = g.read::<u32>(w(0x56));
        let b = u32::from(g.read::<u8>(list.wrapping_add(i)));
        // Some: slot 249 with this volume; None: slot 530
        let volume = if g.read::<u32>(w(0x4f)) != 0 {
            Some(if v < 0 { 0x100 } else { v })
        } else if b < 0x20 {
            None
        } else {
            Some(v)
        };
        eax = match volume {
            Some(v) => {
                let a = g.read::<u32>(w(0x9b).wrapping_add(4 * b));
                let s = slot_target(g, w(0xa2), 249);
                g.call(s, 0, &[a, v.cast_unsigned()]).eax
            }
            None => {
                let s = slot_target(g, w(0x72), 530);
                g.call(s, 0, &[b, v.cast_unsigned()]).eax
            }
        };
        i += 1;
    }
    Ret::eax(eax)
}

/// [`sound_fade`] (195 bytes, 1 overlay).
pub const SOUND_FADE: Sig = Sig {
    len: 195,
    hash: 0xe39b_b349_4606_6338,
    calls: &[],
    imports: &[],
};

/// A pool node (`sa_pool::take`, owner the immediate at `owner`) put at
/// the tail of the task list (head `*H`, tail `*T`): `+0x4` 0, `+0x0` the
/// old tail (or 0), `+0x8` itself. `h_r`/`t_r`/`t_w`/`h_w`/`t_w0` are the
/// operand offsets of the head read, the tail read, the tail store, the
/// empty list's head store and its tail store. Returns the node.
fn task_tail<G: Game>(g: &mut G, img: &Image, f: u32, take: u32, owner: u32, ops: [u32; 5]) -> u32 {
    let w = |off: u32| img.word(f + off);
    let [h_r, t_r, t_w, h_w, t_w0] = ops;
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + take), w(owner));
    g.write::<u32>(node + 4, 0);
    if g.read::<u32>(w(h_r)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(h_w), node);
        g.write::<u32>(w(t_w0), node);
    } else {
        let tail = g.read::<u32>(w(t_r));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(t_w), node);
    }
    g.write::<u32>(node + 8, node);
    node
}

/// 0x10002cc0 (no arguments, a /GS frame): slot 237 `(*X, 0x8a, 0, c)`
/// fills a local colour `c` (`ebp - 0x14`, its `w` then 1.0); the sprite
/// `P + 0x5cb8` (`P` = `*` operand `+0x18`) set up with it
/// ([`sprite_init_colour`](crate::sb_misc::sprite_init_colour), the owner
/// `(*A)[8]`, `A` at `+0x11`) and its words `+0x20c`, `+0x218`, `+0x21c`,
/// `+0x224`; `P + 0x5f0c` 500, `+0x5f10` 0x220, and a task node for the
/// callback `+0x85` ([`task_tail`]). Returns the node.
pub fn sprite_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let c = ebp.wrapping_sub(0x14);
    let owner = g.read::<u32>(g.read::<u32>(w(0x11)) + 8);
    let p = g.read::<u32>(w(0x18));
    let x = g.read::<u32>(w(0x39));
    let s = slot_target(g, w(0x26), 237);
    g.call(s, 0, &[x, 0x8a, 0, c]);
    let sprite = p + 0x5cb8;
    g.write::<u32>(c + 0xc, 0x3f80_0000);
    // three pushes, slot 237's four words, the colour's address, the call
    g.write::<u32>(ebp.wrapping_sub(0x34), c);
    crate::sb_misc::sprite_init_colour(g, ebp.wrapping_sub(0x38), sprite, owner);
    for (off, v) in [
        (0x224, 0xa0),
        (0x21c, 0x3e19_999a),
        (0x218, 0),
        (0x20c, 0x4040_4040),
    ] {
        g.write::<u32>(sprite + off, v);
    }
    g.write::<u32>(p + 0x5f0c, 0x1f4);
    g.write::<u32>(p + 0x5f10, 0x220);
    let node = task_tail(g, img, f, 0x9e, 0x85, [0xa4, 0xb7, 0xd3, 0xc9, 0xd3]);
    Ret::eax(node)
}

/// [`sprite_task`] (227 bytes, 1 overlay; the `rel32` at `+0xdb` is
/// `__security_check_cookie`).
pub const SPRITE_TASK: Sig = Sig {
    len: 227,
    hash: 0x36dd_819d_aaa1_3923,
    calls: &[
        (0x55, Callee::Sig(&crate::sb_misc::SPRITE_INIT_COLOUR)),
        (0x9e, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xdb, Callee::Any),
    ],
    imports: &[],
};

/// 0x10004b50 (no arguments): as [`copy_point`] (the colour 0.13, the
/// point `+0x9e0..+0x9ec`, `+0x478` -5.0), then the sprite `P + 0x5b90`'s
/// words (`+0xf0`..`+0x124`) and a task node for the callback `+0xce`
/// holding the sprite at `+0x14` ([`task_tail`]). Returns the node.
pub fn sprite_point_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x02));
    for off in [0x3fc, 0x3f8, 0x3f4, 0x3f0] {
        g.write::<u32>(p + off, 0x3e05_1eb8);
    }
    g.write::<u32>(p + 0x460, g.read::<u32>(p + 0x9e0));
    g.write::<u32>(p + 0x464, g.read::<u32>(p + 0x9e4));
    let y = sub(g.read::<f32>(p + 0x464), g.read::<f32>(w(0x59)));
    g.write::<u32>(p + 0x468, g.read::<u32>(p + 0x9e8));
    g.write::<u32>(p + 0x46c, g.read::<u32>(p + 0x9ec));
    g.write::<u32>(p + 0x464, y.to_bits());
    g.write::<u32>(p + 0x478, 0xc0a0_0000);
    let s = p + 0x5b90;
    for (off, v) in [
        (0xf0, 0x3080_8080),
        (0x124, 4),
        (0x120, 0),
        (0x110, 0),
        (0xf8, 0x3cf5_c28f),
        (0xfc, 0x3cf5_c28f),
        (0x100, 0x3cf5_c28f),
        (0x108, 0x75),
        (0x104, 0x75),
        (0x10c, 8),
    ] {
        g.write::<u32>(s + off, v);
    }
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xf1), w(0xce));
    let empty = g.read::<u32>(w(0xf7)) == 0;
    g.write::<u32>(node + 0x14, s);
    g.write::<u32>(node + 4, 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x124), node);
        g.write::<u32>(w(0x129), node);
    } else {
        let tail = g.read::<u32>(w(0x10b));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(0x115), node);
    }
    g.write::<u32>(node + 8, node);
    Ret::eax(node)
}

/// [`sprite_point_task`] (305 bytes, 1 overlay).
pub const SPRITE_POINT_TASK: Sig = Sig {
    len: 305,
    hash: 0x4a97_9127_dace_e6db,
    calls: &[(0xf1, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// 0x10004930 (no arguments, a /GS frame): [`look_point`]; the camera's
/// point `G` (`+0x62`, `+0x77`, `+0x8c`) = `v + (A, -B, C)` (`+0x52`,
/// `+0x6f`, `+0x84`); while slot 520 says 0x1590000, `G.z` = `v.z + D`
/// (`+0xac`). The target `T` (`+0xd1`, `+0xee`, `+0xe6`) = `(v.x - 0,
/// v.y - E, v.z - 0)` (`+0xde`); when slot 520 says 0x1590000 again, `G`
/// moves towards `T` by `1 / K` (`(T - G) / K + G`, `K` at `+0x115`).
/// Returns slot 520's last `eax`.
pub fn look_follow<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::div;
    let w = |off: u32| img.word(fr.at + off);
    let fl = |g: &G, off: u32| g.read::<f32>(w(off));
    let (_, [x, y, z]) = look_point(g, img, fr);
    let a = add(x, fl(g, 0x52));
    g.write::<u32>(w(0x62), a.to_bits());
    let b = sub(y, fl(g, 0x6f));
    g.write::<u32>(w(0x77), b.to_bits());
    let c = add(z, fl(g, 0x84));
    g.write::<u32>(w(0x8c), c.to_bits());
    let s = slot_target(g, w(0x57), 520);
    if g.call(s, 0, &[]).eax == 0x0159_0000 {
        let c = add(z, fl(g, 0xac));
        g.write::<u32>(w(0xb4), c.to_bits());
    }
    let tx = sub(x, 0.0);
    let tz = sub(z, 0.0);
    g.write::<u32>(w(0xd1), tx.to_bits());
    let ty = sub(y, fl(g, 0xde));
    g.write::<u32>(w(0xe6), tz.to_bits());
    g.write::<u32>(w(0xee), ty.to_bits());
    let s = slot_target(g, w(0xbe), 520);
    let eax = g.call(s, 0, &[]).eax;
    if eax == 0x0159_0000 {
        let k = fl(g, 0x115);
        for (t, gr, gw, gr2) in [
            (0x105, 0x10d, 0x129, 0x121),
            (0x131, 0x139, 0x14d, 0x145),
            (0x155, 0x15d, 0x171, 0x169),
        ] {
            let d = div(sub(fl(g, t), fl(g, gr)), k);
            let v = add(d, fl(g, gr2));
            g.write::<u32>(w(gw), v.to_bits());
        }
    }
    Ret::eax(eax)
}

/// [`look_follow`] (387 bytes, 1 overlay; the `rel32` at `+0x17b` is
/// `__security_check_cookie`).
pub const LOOK_FOLLOW: Sig = Sig {
    len: 387,
    hash: 0x3601_0df9_172b_e2f8,
    calls: &[(0x17b, Callee::Any)],
    imports: &[],
};

/// 0x10003750 (no arguments): three objects of `P` (`*` operand `+0x05`;
/// `P + 0x130 * P[0x5f1c]`, `P + 0x390`, `P + 0x4c0`) each taken out of
/// the object list `L` (`+0x0a`, [`list_remove`](crate::l5_objhost::list_remove));
/// for each one removed, `**Q` the immediate `+0x34` (`Q` at `+0x2d`),
/// slot 713 `(obj)` and its flags `+0x124` lose bit 15. Then the sprite
/// `P + 0x5a68`'s words (`+0xf0`..`+0x124`) and a task node for the
/// callback `+0xc8` holding the sprite at `+0x14` (the task list's head
/// `+0x137`, tail `+0x14d`). Returns the node.
pub fn objects_off_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x05));
    let first = g
        .read::<u32>(p + 0x5f1c)
        .wrapping_mul(0x130)
        .wrapping_add(p);
    for (obj, list, call, q, imm, table) in [
        (first, 0x0a, 0x1f, 0x2d, 0x34, 0x39),
        (p + 0x390, 0x58, 0x5d, 0x66, 0x6d, 0x72),
        (p + 0x4c0, 0x91, 0x96, 0x9f, 0xa6, 0xab),
    ] {
        let at = img.rel32_target(f + call);
        if crate::l5_objhost::list_remove(g, img, at, w(list), obj) != 0 {
            let qp = g.read::<u32>(w(q));
            g.write::<u32>(qp, w(imm));
            let s = slot_target(g, w(table), 713);
            g.call(s, 0, &[obj]);
            let fl = g.read::<u16>(obj + 0x124);
            g.write::<u16>(obj + 0x124, fl & 0x7fff);
        }
    }
    let s = p + 0x5a68;
    for (off, v) in [
        (0xf0, 0x3080_8080),
        (0x124, 4),
        (0x120, 0),
        (0x110, 0),
        (0xf8, 0x3d23_d70a),
        (0xfc, 0x3ca3_d70a),
        (0x100, 0),
        (0x108, 0x14),
        (0x104, 0x14),
        (0x10c, 8),
    ] {
        g.write::<u32>(s + off, v);
    }
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x131), w(0xc8));
    let empty = g.read::<u32>(w(0x137)) == 0;
    g.write::<u32>(node + 0x14, s);
    g.write::<u32>(node + 4, 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x166), node);
        g.write::<u32>(w(0x16b), node);
    } else {
        let tail = g.read::<u32>(w(0x14d));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(0x157), node);
    }
    g.write::<u32>(node + 8, node);
    Ret::eax(node)
}

/// [`objects_off_task`] (371 bytes, 1 overlay).
pub const OBJECTS_OFF_TASK: Sig = Sig {
    len: 371,
    hash: 0x728f_5f64_79e4_eacd,
    calls: &[
        (0x1f, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x5d, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x96, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x131, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// 0x100044e0 (no arguments): three task nodes off the pool
/// ([`take`](crate::sa_pool::take), owners the immediates `+0x0a`,
/// `+0xa5`, `+0x111`) chained onto the task list: the first holds the
/// sprite `P + 0x58c0` (its words `+0x80`..`+0x98` set; `P` = `*` operand
/// `+0x05`) after the list's old tail `*T` (`+0x82`), the second (flag
/// byte 1) the block `P + 0x597c` (`+0x00`..`+0x18`) after the first, the
/// third goes into `P + 0x5ee0` after the second (`P + 0x5f14` = 12
/// first) and becomes the tail (`+0x139`). Each node whose list was empty
/// (`*H` 0, operands `+0x6d`, `+0xdd`, `+0x126`) becomes the head
/// (`+0x95`, `+0x103`, `+0x154`) instead of linking back. Returns the
/// third node.
pub fn three_tasks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sa_pool::take;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x05));
    let s = p + 0x58c0;
    g.write::<u32>(s + 0x81, 0xff80_ffff);
    g.write::<u8>(s + 0x80, 0xff);
    g.write::<u16>(s + 0x85, 0xffff);
    g.write::<u8>(s + 0x87, 0);
    g.write::<u32>(s + 0x88, 0);
    g.write::<u16>(s + 0x98, 0x48);
    g.write::<u32>(s + 0x90, 0x18);
    g.write::<u32>(s + 0x8c, 0x18);
    g.write::<u32>(s + 0x94, 0);
    // each node after `prev` (`None`: the list's tail global), or the head
    let link = |g: &mut G, node: u32, prev: Option<u32>, head_r: u32, head_w: u32| {
        g.write::<u32>(node + 4, 0);
        if g.read::<u32>(w(head_r)) == 0 {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(head_w), node);
        } else {
            let prev = prev.unwrap_or_else(|| g.read::<u32>(w(0x82)));
            g.write::<u32>(node, prev);
            g.write::<u32>(prev + 4, node);
        }
        g.write::<u32>(node + 8, node);
    };
    let n1 = take(g, img, img.rel32_target(f + 0x67), w(0x0a));
    g.write::<u32>(n1 + 0x14, s);
    link(g, n1, None, 0x6d, 0x95);
    let b = p + 0x597c;
    for (off, v) in [
        (0u32, 0u32),
        (4, 0x50),
        (8, 0x14),
        (0xc, 0),
        (0x14, 0x5a),
        (0x10, 0x5a),
    ] {
        g.write::<u32>(b + off, v);
    }
    g.write::<u16>(b + 0x18, 0);
    let n2 = take(g, img, img.rel32_target(f + 0xd7), w(0xa5));
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, b);
    link(g, n2, Some(n1), 0xdd, 0x103);
    let p = g.read::<u32>(w(0x109));
    g.write::<u32>(p + 0x5f14, 0xc);
    let n3 = take(g, img, img.rel32_target(f + 0x120), w(0x111));
    let empty = g.read::<u32>(w(0x126)) == 0;
    g.write::<u32>(p + 0x5ee0, n3);
    g.write::<u32>(n3 + 4, 0);
    g.write::<u32>(w(0x139), n3);
    if empty {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x154), n3);
    } else {
        g.write::<u32>(n3, n2);
        g.write::<u32>(n2 + 4, n3);
    }
    g.write::<u32>(n3 + 8, n3);
    Ret::eax(n3)
}

/// [`three_tasks`] (349 bytes, 1 overlay).
pub const THREE_TASKS: Sig = Sig {
    len: 349,
    hash: 0xe15a_1465_97e0_0e65,
    calls: &[
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xd7, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x120, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
