//! `magic_0447.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Frame, Game, Image, Ret, Sig};

/// The camera's copy (0x10005430, no arguments): with `E` = `*G`
/// (`+0x02`) the quads `E + 0x600` and `E + 0x5f0` go to `E + 0x610` and
/// `E + 0x620`; `E + 0x7cc` and `E + 0x7c8` become 30. Returns `E +
/// 0x5fc`'s word (the last one copied).
pub fn camera_copy<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let e = g.read::<u32>(img.word(fr.at + 0x02));
    let mut v = 0;
    for (src, dst) in [(0x600, 0x610), (0x5f0, 0x620)] {
        for k in 0..4 {
            v = g.read::<u32>(e + src + 4 * k);
            g.write::<u32>(e + dst + 4 * k, v);
        }
    }
    g.write::<u32>(e + 0x7cc, 0x1e);
    g.write::<u32>(e + 0x7c8, 0x1e);
    Ret::eax(v)
}

/// [`camera_copy`] (123 bytes, 1 overlay).
pub const CAMERA_COPY: Sig = Sig {
    len: 123,
    hash: 0xd094_08de_3142_fae2,
    calls: &[],
    imports: &[],
};

/// Table entry (0x10005290, no arguments): object 1 (`G_STATE + 0x130`)
/// when `G_STATE + 0x7d4` is set, else object 3, starts
/// ([`start_object_state`](crate::r9_misc::start_object_state): scale
/// `*K_S` `+0x35`, the index); the character `CHR[*G_SEL's byte]`
/// (`+0x62`, `+0x52`) gives the object's point `p` = `obj + 0xd0` (slot
/// 262 `(CHR, p, p + 4, p + 8)`, slot 268 `(CHR, 0x10, p)`, `p + 0xc`
/// 1.0); `obj + 0xc4` = `G_STATE + 0x7c4`. The record `G_STATE + 0x790`
/// (`+0x10..+0x1c`, `+0x28`, `+0x2c`, halfword `+0x30` 0; `+0x20` the
/// immediate `+0xd8`; `+0x24` the callback `+0x1f` or `+0x2d`) goes into
/// `obj + 0x110`; the colour block `G_STATE + 0x630` is set (`+0x80`
/// 0x80808080, `+0x84` 0x808080, `+0x88`/`+0x94` 0, `+0x8c`/`+0x90`
/// 0x14, halfword `+0x98` 0x48) and handed to a task node
/// ([`take`](crate::sa_pool::take), owner `+0x103`) appended to the task
/// list (first `+0x13f`, last `+0x152`/`+0x189`); then the fade task
/// `G_STATE + 0x708` ([`fade_task`](crate::s3_misc::fade_task): `edx`
/// 0x1e, `xmm1`/`xmm2` the floats `+0x174`/`+0x16c`, the stack word
/// 0x23). Returns the fade task's node. Each slot call passes the words
/// the original leaves on the stack.
pub fn object_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let state = g.read::<u32>(w(0x07));
    let (index, callback) = if g.read::<u32>(state + 0x7d4) == 0 {
        (3u32, w(0x2d))
    } else {
        (1, w(0x1f))
    };
    let obj = state.wrapping_add(index * 0x130);
    let scale = g.read::<f32>(w(0x35));
    // push ebx/esi/edi; sub esp, 0xc; push ecx; call
    let esp = fr.esp.wrapping_sub(0x2c);
    g.write::<u32>(esp + 4, index);
    crate::r9_misc::start_object_state(g, img, img.rel32_target(f + 0x4d), obj, scale, index);
    let mut s = crate::pushes::Pushes::default();
    s.push(index);
    let sel = u32::from(g.read::<u8>(g.read::<u32>(w(0x52))));
    let p = obj + 0xd0;
    let chr = g.read::<u32>(w(0x62).wrapping_add(4 * sel));
    s.push_all(&[p + 8, p + 4, p, chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, crate::runtime::slot_target(g, w(0x6f), 0x418 / 4), 0);
    s.push_all(&[p, 0x10, chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, crate::runtime::slot_target(g, w(0x7e), 0x430 / 4), 0);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let v = g.read::<u32>(state + 0x7c4);
    g.write::<u32>(obj + 0xc4, v);
    let rec = state + 0x790;
    let colour = state + 0x630;
    for o in [0x18, 0x14, 0x10, 0x1c] {
        g.write::<u32>(rec + o, 0);
    }
    g.write::<u32>(rec + 0x24, callback);
    g.write::<u32>(rec + 0x20, w(0xd8));
    g.write::<u32>(rec + 0x28, 0);
    g.write::<u32>(rec + 0x2c, 0);
    g.write::<u16>(rec + 0x30, 0);
    g.write::<u32>(obj + 0x110, rec);
    g.write::<u32>(colour + 0x88, 0);
    g.write::<u32>(colour + 0x94, 0);
    g.write::<u32>(colour + 0x80, 0x8080_8080);
    g.write::<u32>(colour + 0x84, 0x0080_8080);
    g.write::<u16>(colour + 0x98, 0x48);
    g.write::<u32>(colour + 0x90, 0x14);
    g.write::<u32>(colour + 0x8c, 0x14);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x139), w(0x103));
    let head = g.read::<u32>(w(0x13f));
    g.write::<u32>(node + 0x14, colour);
    g.write::<u32>(node + 4, 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x164), node);
    } else {
        let last = g.read::<u32>(w(0x152));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x189), node);
    // the fade task's call: 0x23 pushed below the saved registers
    let fade = Frame {
        at: img.rel32_target(f + 0x18e),
        esp: fr.esp.wrapping_sub(0x20),
        ecx: state + 0x708,
        edx: 0x1e,
        xmm: [
            0,
            u128::from(g.read::<u32>(w(0x174))),
            u128::from(g.read::<u32>(w(0x16c))),
            0,
        ],
        ..*fr
    };
    g.write::<u32>(fade.esp + 4, 0x23);
    Ret::eax(crate::s3_misc::fade_task(g, img, &fade))
}

/// [`object_fade`] (412 bytes, 1 overlay).
pub const OBJECT_FADE: Sig = Sig {
    len: 412,
    hash: 0xf89b_dc65_f365_f840,
    calls: &[
        (
            0x4d,
            crate::Callee::Sig(&crate::r9_misc::START_OBJECT_STATE_0447),
        ),
        (0x139, crate::Callee::Sig(&crate::sa_pool::TAKE)),
        (0x18e, crate::Callee::Sig(&crate::s3_misc::FADE_TASK)),
    ],
    imports: &[],
};

/// Table entry (0x10004cb0, no arguments; a /GS frame): `magic_0390`'s
/// [`e1750`](crate::m0390::e1750) opening on the point `B` = `G_STATE +
/// 0x600`: slot 262 `(CHR, B, B + 4, B + 8)` (`CHR[*G_SEL's byte]`,
/// `+0x22`, `+0x11`; `B + 0xc` 1.0), slot 333 `(*G_H2)`'s float `v` and
/// the yaw `K - v` wrapped once (operands `+0x65`, `+0x74`, `+0x8d`,
/// `+0x83`/`+0x9a`); the height `h` from slot 251 `(sel)` (`*G_FLAG`
/// `+0xa5` 0 and `sel < 0x20`) or slot 17 `(C)` times slot 19 `(C)`
/// (`C` = `CHR[sel]`, `+0xea`), plus `K5` (`+0x11c`); the local matrix
/// `M` (slot 235, slot 328 `(M, M, 0)`, slot 329 `(M, M, yaw)`) takes `V
/// = (0, 0, h, 1)` to `O` (slot 229) and slot 248 `(B, B, O)` moves `B`
/// (`B + 0xc` 1.0 again). Then `G_STATE + 0x5f0..0x60c` is copied to
/// `+0x610..0x62c`, `+0x7cc` and `+0x7c8` become 0x1e, slot 623 `(*G_H2,
/// 0, 0, 0, 0, 0, 0)` goes into `+0x7d0`, three floats 1.0 (`+0x24d`,
/// `+0x257`, `+0x261`) and slot 790's float into `+0x26d`, and the fade
/// task `G_STATE + 0x6d0` starts
/// ([`fade_task`](crate::s3_misc::fade_task): `edx` 0x14, `xmm1`/`xmm2`
/// the floats `+0x28a`/`+0x282`, the stack word 0). Returns the fade
/// task's node. Each slot call passes the words the original leaves on
/// the stack (the branch's saved `esi` left out: no slot reads that
/// deep).
#[allow(clippy::too_many_lines)]
pub fn camera_turn<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::runtime::slot_target;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let (m, o, v) = (ebp - 0x64, ebp - 0x24, ebp - 0x14);
    let sel_p = g.read::<u32>(w(0x11));
    let state = g.read::<u32>(w(0x1b));
    let chr = g.read::<u32>(w(0x22).wrapping_add(4 * u32::from(g.read::<u8>(sel_p))));
    let b = state + 0x600;
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[b + 8, b + 4, b, chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x36), 0x418 / 4), 0);
    g.write::<u32>(b + 0xc, 0x3f80_0000);
    s.push(g.read::<u32>(w(0x52)));
    // call width: 12 (Pushes caps the words at 12)
    let a = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x45), 0x534 / 4), 0).st0);
    s.pop(5);
    let x = sub(k(g, 0x65), a);
    let yaw = if x > k(g, 0x74) {
        sub(x, k(g, 0x83))
    } else if k(g, 0x8d) > x {
        add(x, k(g, 0x9a))
    } else {
        x
    };
    let flag = g.read::<u32>(w(0xa5));
    let sel_p = g.read::<u32>(w(0xac));
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    let sel = u32::from(g.read::<u8>(sel_p));
    let h = if flag == 0 && sel < 0x20 {
        s.push(sel);
        // call width: 12 (Pushes caps the words at 12)
        let r = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0xca), 0x3ec / 4), 0).st0);
        s.pop(1);
        r
    } else {
        let c = g.read::<u32>(w(0xea).wrapping_add(4 * sel));
        s.push(c);
        // call width: 12 (Pushes caps the words at 12)
        let a = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0xef), 0x4c / 4), 0).st0);
        s.push(c);
        // call width: 12 (Pushes caps the words at 12)
        let b = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0xfa), 0x44 / 4), 0).st0);
        s.pop(2);
        mul(b, a)
    };
    let h = add(h, k(g, 0x11c));
    s.push(m);
    g.write::<f32>(v + 8, h);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x125), 0x3ac / 4), 0);
    s.set_top(0);
    s.push_all(&[m, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x143), 0x520 / 4), 0);
    s.pop(2);
    s.set_top(yaw.to_bits());
    s.push_all(&[m, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x162), 0x524 / 4), 0);
    s.push_all(&[v, m, o]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x17b), 0x394 / 4), 0);
    s.push_all(&[o, b, b]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x193), 0x3e0 / 4), 0);
    g.write::<u32>(b + 0xc, 0x3f80_0000);
    for i in 0..8u32 {
        let x = g.read::<u32>(state + 0x5f0 + 4 * i);
        g.write::<u32>(state + 0x610 + 4 * i, x);
    }
    g.write::<u32>(state + 0x7cc, 0x1e);
    g.write::<u32>(state + 0x7c8, 0x1e);
    let h2 = g.read::<u32>(w(0x231));
    s.push_all(&[0, 0, 0, 0, 0, 0, h2]);
    // call width: 12 (Pushes caps the words at 12)
    let r = s.call(g, slot_target(g, w(0x20b), 0x9bc / 4), 0).eax;
    g.write::<u32>(state + 0x7d0, r);
    for off in [0x24d, 0x257, 0x261] {
        g.write::<u32>(w(off), 0x3f80_0000);
    }
    let t = slot_target(g, w(0x23e), 0xc58 / 4);
    let x = crate::m0018::st0_f32(g.call(t, 0, &[]).st0);
    g.write::<f32>(w(0x26d), x);
    // push ebp; sub esp, 0x70; push ebx/edi; push 0; call
    let fade = Frame {
        at: img.rel32_target(f + 0x28f),
        esp: fr.esp.wrapping_sub(0x84),
        ecx: state + 0x6d0,
        edx: 0x14,
        xmm: [
            0,
            u128::from(g.read::<u32>(w(0x28a))),
            u128::from(g.read::<u32>(w(0x282))),
            0,
        ],
        ..*fr
    };
    g.write::<u32>(fade.esp + 4, 0);
    Ret::eax(crate::s3_misc::fade_task(g, img, &fade))
}

/// [`camera_turn`] (678 bytes, 1 overlay).
pub const CAMERA_TURN: Sig = Sig {
    len: 678,
    hash: 0xb412_7218_d57d_aab5,
    calls: &[
        (0x28f, crate::Callee::Sig(&crate::s3_misc::FADE_TASK)),
        // __security_check_cookie
        (0x29e, crate::Callee::Any),
    ],
    imports: &[],
};

/// One coordinate of [`camera_curve`]'s cubic: `a` = `ut uu C1 (d K20) +
/// uu uu d + tt uu C2 (d K18) + tt ut C3 (d K10) + tt tt 0`, plus `p1`
/// (`d` = `p2 - p1`; each product and sum one `f32` rounding in the
/// original's order). `c` and `k` are the operands' offsets.
fn cubic<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    (u, t): (f32, f32),
    (p1, p2): (f32, f32),
    c: [u32; 3],
    k: [u32; 3],
) -> f32 {
    use crate::sb_sse::{add, mul, sub};
    let r = |o: u32| g.read::<f32>(img.word(f + o));
    let (uu, ut, tt) = (mul(u, u), mul(u, t), mul(t, t));
    let d = sub(p2, p1);
    let mut a = mul(mul(mul(ut, uu), r(c[0])), mul(d, r(k[0])));
    a = add(a, mul(mul(uu, uu), d));
    a = add(a, mul(mul(mul(tt, uu), r(c[1])), mul(d, r(k[1]))));
    a = add(a, mul(mul(mul(tt, ut), r(c[2])), mul(d, r(k[2]))));
    a = add(a, mul(mul(tt, tt), 0.0));
    add(a, p1)
}

/// Table entry (0x10004f60, no arguments; a /GS frame): the countdown
/// `G_STATE + 0x7c8` goes down by one while positive; `t = n / N` (`N`
/// `+0x7cc`, both ints), `u = 1 - t` (operand `+0x4d`) and a point on a
/// cubic from `G_STATE + 0x610` to `+0x620` per coordinate ([`cubic`]).
/// With `*G_FLAG` (`+0x1a7`) clear and `*G_INDEX` (`+0x277`) below 0x20
/// the point (a local) goes to slot 255 `(index, &p)`; otherwise slot 11
/// `(*G_H)` picks slot 33 `(h, x, y, z)` (0) or slot 32 `(h, x, z)`.
/// Returns the last call's result.
pub fn camera_curve<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::runtime::slot_target;
    use crate::sb_sse::{cvt, div, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let state = g.read::<u32>(w(0x12));
    let n = g.read::<u32>(state + 0x7c8).cast_signed();
    if n > 0 {
        g.write::<u32>(state + 0x7c8, (n - 1).cast_unsigned());
    }
    let t = div(
        cvt(g.read::<u32>(state + 0x7c8)),
        cvt(g.read::<u32>(state + 0x7cc)),
    );
    let u = sub(g.read::<f32>(w(0x4d)), t);
    let p = |g: &G, o: u32| {
        (
            g.read::<f32>(state + 0x610 + o),
            g.read::<f32>(state + 0x620 + o),
        )
    };
    let x = cubic(
        g,
        img,
        f,
        (u, t),
        p(g, 0),
        [0x61, 0x61, 0x61],
        [0xa4, 0xca, 0xd2],
    );
    let y = cubic(
        g,
        img,
        f,
        (u, t),
        p(g, 4),
        [0x148, 0x150, 0x19d],
        [0x15b, 0x179, 0x181],
    );
    let z = cubic(
        g,
        img,
        f,
        (u, t),
        p(g, 8),
        [0x1e0, 0x1e0, 0x1e0],
        [0x214, 0x232, 0x23a],
    );
    let at = fr.esp.wrapping_sub(0x18);
    g.write::<f32>(at, x);
    g.write::<f32>(at + 4, y);
    g.write::<f32>(at + 8, z);
    let index = g.read::<u32>(w(0x277));
    if g.read::<u32>(w(0x1a7)) == 0 && index.cast_signed() < 0x20 {
        let s = slot_target(g, w(0x286), 0x3fc / 4);
        return Ret::eax(g.call(s, 0, &[index, at]).eax);
    }
    let h = g.read::<u32>(w(0x2ab));
    let (x, y, z) = (x.to_bits(), y.to_bits(), z.to_bits());
    if g.call(slot_target(g, w(0x2a4), 0x2c / 4), 0, &[h]).eax == 0 {
        let s = slot_target(g, w(0x2c0), 0x84 / 4);
        Ret::eax(g.call(s, 0, &[h, x, y, z]).eax)
    } else {
        let s = slot_target(g, w(0x2c0), 0x80 / 4);
        Ret::eax(g.call(s, 0, &[h, x, z]).eax)
    }
}

/// [`camera_curve`] (813 bytes, 1 overlay).
pub const CAMERA_CURVE: Sig = Sig {
    len: 813,
    hash: 0xbba0_3596_ffc7_ded8,
    // __security_check_cookie
    calls: &[
        (0x29b, crate::Callee::Any),
        (0x2f7, crate::Callee::Any),
        (0x325, crate::Callee::Any),
    ],
    imports: &[],
};
