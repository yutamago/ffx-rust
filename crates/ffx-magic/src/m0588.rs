//! `magic_0588.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::sb_sse::{mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// The current actor's word in the actor table: `TABLE[(*G_E)[1]]` (the
/// byte `+0x1` of the record at `*G_E`, the table at `table`).
fn actor<G: Game>(g: &mut G, e_at: u32, table: u32) -> u32 {
    let e = g.read::<u32>(e_at);
    let b = u32::from(g.read::<u8>(e.wrapping_add(1)));
    g.read::<u32>(table.wrapping_add(4 * b))
}

/// Starts the sound (0x10001e10, no arguments): slot 0 `(0x3016)` gives a
/// handle, slot 26 `(0x3016, 1)`; the byte at operand `+0x2a` 0x20, the
/// handle into `+0x31`; slot 10 `(handle, 0)`; `+0x44` = 2; slot 257
/// `(actor, 1)` ([`actor`]: `+0x3e`, `+0x55`). Each call sees the
/// earlier calls' words. Returns slot 257's `eax`.
pub fn sound<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x1), 0);
    let mut stack = vec![0x3016];
    let h = g.call(t, 0, &stack).eax;
    let t = slot_target(g, w(0x11), 26);
    stack.splice(0..0, [0x3016, 1]);
    // call width: 3 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    g.write::<u8>(w(0x2a), 0x20);
    g.write::<u32>(w(0x31), h);
    let t = slot_target(g, w(0x24), 10);
    stack.splice(0..0, [h, 0]);
    // call width: 5 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let e = g.read::<u32>(w(0x3e));
    g.write::<u32>(w(0x44), 2);
    let b = u32::from(g.read::<u8>(e.wrapping_add(1)));
    let a = g.read::<u32>(w(0x55).wrapping_add(4 * b));
    let t = slot_target(g, w(0x5a), 257);
    stack.splice(0..0, [a, 1]);
    // call width: 7 (the earlier calls' words stay on the stack)
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// [`sound`] (107 bytes, 1 overlay).
pub const SOUND: Sig = Sig {
    len: 107,
    hash: 0xd81b_b38f_5548_db09,
    calls: &[],
    imports: &[],
};

/// Starts the effect's part (0x10002510, no arguments; the record `R =
/// *G_R`, operand `+0x03`): `R[0xe00]` 1; a pool node ([`take`], the
/// owner the immediate at `+0x08`) with byte `+0x18` 1 and `+0x4` 0 put
/// at the head of the list (head `+0x31`/`+0x44`, the empty test `+0x1d`,
/// set `+0x5a`/`+0x5f`), `R[0xdec]` the node. Returns the node.
pub fn part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = g.read::<u32>(w(0x3));
    g.write::<u32>(r + 0xe00, 1);
    let node = take(g, img, img.rel32_target(f + 0x17), w(0x8));
    let empty = g.read::<u32>(w(0x1d)) == 0;
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 4, 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(r + 0xdec, node);
        g.write::<u32>(w(0x5a), node);
        g.write::<u32>(w(0x5f), node);
    } else {
        let head = g.read::<u32>(w(0x31));
        g.write::<u32>(node, head);
        g.write::<u32>(head.wrapping_add(4), node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(r + 0xdec, node);
        g.write::<u32>(w(0x44), node);
    }
    Ret::eax(node)
}

/// [`part_start`] (101 bytes, 1 overlay).
pub const PART_START: Sig = Sig {
    len: 101,
    hash: 0x6c00_f50b_c24b_9da4,
    calls: &[(0x17, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// Sets the actor's colours (0x10002800, no arguments): for the current
/// actor ([`actor`]: `+0x01`, `+0x12`) slots 357 `(a, -1, 0x3540)`, 356
/// `(a, -1, 0x3bd0)`, 355 `(a, -1)` and 354 `(a, -1)`, each seeing the
/// earlier calls' words. Returns slot 354's `eax`.
pub fn colours<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let a = actor(g, w(0x1), w(0x12));
    let mut stack: Vec<u32> = Vec::new();
    let mut eax = 0;
    for (op, slot, k) in [
        (0x17, 357, Some(0x3540)),
        (0x27, 356, Some(0x3bd0)),
        (0x3c, 355, None),
        (0x4c, 354, None),
    ] {
        if let Some(k) = k {
            stack.insert(0, k);
        }
        stack.splice(0..0, [a, u32::MAX]);
        let t = slot_target(g, w(op), slot);
        // call width: 10 at most (the earlier calls' words stay on the stack)
        eax = g.call(t, 0, &stack).eax;
    }
    Ret::eax(eax)
}

/// [`colours`] (96 bytes, 1 overlay).
pub const COLOURS: Sig = Sig {
    len: 96,
    hash: 0x1b62_3950_7bdc_7d08,
    calls: &[],
    imports: &[],
};

/// Places the effect's light (0x10002860, an overlay table entry, no
/// arguments; a frame with a stack cookie; the record `R = *G_R`,
/// operand `+0x11`): the matrix `m` at `ebp - 0x64` reset (slot 235)
/// and turned by X 0 (slot 328 `(m, m, 0)`) and Y `R[0xdf4]` (slot 329);
/// the vector (-20, 0, -10, 1) at `ebp - 0x14` through it into `ebp -
/// 0x24` (slot 229); slot 248 `(P, P, that)` with `P = R + 0xbd4`, then
/// `P[0xc]` 1.0. Each call sees the words the earlier ones left. Returns
/// slot 248's `eax`.
pub fn light<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v, o) = (
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x24),
    );
    let r = g.read::<u32>(w(0x11));
    let y = g.read::<u32>(r + 0xdf4);
    let p = r + 0xbd4;
    g.write::<u32>(ebp.wrapping_sub(0x68), y);
    for (i, b) in (0u32..).zip([0xc1a0_0000, 0, 0xc120_0000, ONE]) {
        g.write::<u32>(v + 4 * i, b);
    }
    let t = slot_target(g, w(0x29), 235);
    g.call(t, 0, &[m]);
    let t = slot_target(g, w(0x63), 328);
    g.call(t, 0, &[m, m, 0]);
    let t = slot_target(g, w(0x82), 329);
    g.call(t, 0, &[m, m, y]);
    g.write::<u32>(v + 0xc, ONE);
    let t = slot_target(g, w(0x9b), 229);
    let mut stack = vec![o, m, v, m, m, y];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xb3), 248);
    stack.splice(0..0, [p, p, o]);
    // call width: 9 (the earlier calls' words stay on the stack)
    let eax = g.call(t, 0, &stack).eax;
    g.write::<u32>(p + 0xc, ONE);
    Ret::eax(eax)
}

/// [`light`] (218 bytes, 1 overlay).
pub const LIGHT: Sig = Sig {
    len: 218,
    hash: 0xa678_fa68_60fe_3d30,
    // __security_check_cookie
    calls: &[(0xd2, Callee::Any)],
    imports: &[],
};

/// A displacement byte or word of the original's own instructions (the
/// two DLLs of [`actor_matrix`] differ only in these).
fn disp8(img: &Image, at: u32) -> u32 {
    img.word(at) & 0xff
}

/// The actor matrix frame of `magic_0588` 0x100025b0 and `magic_0600`
/// 0x100073e0 (internal, no arguments; a /GS frame; the same code but
/// for the actor index's offset and the summon block's field offsets,
/// read from the instructions): the actor `b = (*G_A)[d]`, its handle
/// `h = H[b]` (`H` operand `+0x25`), the matrix `M = S + D` (`S = *G_S`);
/// with `M` 0 (never, unless `S` is `-D`), slots 217 and 255 `(b, &L)`
/// while the DLL's flag is 0 and `b` below 32 (`L` at `ebp - 0x14`);
/// slot 210 `(h, M)` otherwise; slot 211 `(h, M)`; the 8 words of the
/// block's keys copied; slot 235 `(M)`; the scale (slot 277's ST0 times
/// `K`) on the diagonal, times slots 15, 16, 17 (ST0) of the handle;
/// slots 330, 328, 329 `(M, M, angle)` turn it by three of the copied
/// floats; one of them down by `K2`; slot 257 `(h, 0)`. Each call sees the
/// words the earlier ones left on the stack. Returns slot 257's `eax`.
pub fn actor_matrix<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let fl = |g: &G, a: u32| g.read::<f32>(a);
    let st = |out: crate::Out| st0_f32(out.st0);
    let actor = |g: &G, ptr: u32, d: u32, tab: u32| {
        let a = g.read::<u32>(w(ptr));
        let b = u32::from(g.read::<u8>(a + disp8(img, f + d)));
        (b, g.read::<u32>(w(tab) + 4 * b))
    };
    let s = g.read::<u32>(w(0x18));
    let (b, h) = actor(g, 0x11, 0x20, 0x25);
    let m = s.wrapping_add(w(0x2b));
    if m == 0 {
        if g.read::<u32>(w(0x4c)) == 0 && b.cast_signed() < 0x20 {
            let l = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
            let t = slot_target(g, w(0x5d), 217);
            let mut stack = vec![b, l];
            g.call(t, 0, &stack);
            let t = slot_target(g, w(0x6f), 255);
            stack.splice(0..0, [b, l]);
            g.call(t, 0, &stack);
        }
    } else {
        let t = slot_target(g, w(0x39), 210);
        g.call(t, 0, &[h, m]);
    }
    let t = slot_target(g, w(0x85), 211);
    let mut stack = vec![h, m];
    g.call(t, 0, &stack);
    let (src, dst) = (w(0x93), w(0x99));
    for i in 0..8u32 {
        let v = g.read::<u32>(s.wrapping_add(src) + 4 * i);
        g.write::<u32>(s.wrapping_add(dst) + 4 * i, v);
    }
    let t = slot_target(g, w(0xf2), 235);
    stack.insert(0, m);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    let (_, h2) = actor(g, 0x100, 0x107, 0x10b);
    let t = slot_target(g, w(0x110), 277);
    stack.insert(0, h2);
    // call width: 18 at most (the earlier calls' words)
    let x = mul(st(g.call(t, 0, &stack)), k(g, 0x128));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + o, x);
    }
    let (_, h3) = actor(g, 0x13b, 0x142, 0x146);
    let t = slot_target(g, w(0x14b), 15);
    stack.insert(0, h3);
    // call width: 5
    let r = st(g.call(t, 0, &stack));
    let v = mul(r, fl(g, m));
    let t = slot_target(g, w(0x162), 16);
    g.write::<f32>(m, v);
    stack.insert(0, h3);
    // call width: 6
    let r = st(g.call(t, 0, &stack));
    let v = mul(fl(g, m + 0x14), r);
    let t = slot_target(g, w(0x17e), 17);
    g.write::<f32>(m + 0x14, v);
    stack.insert(0, h3);
    // call width: 7
    let r = st(g.call(t, 0, &stack));
    let v = mul(r, fl(g, m + 0x28));
    g.write::<f32>(m + 0x28, v);
    let mut stack = Vec::new();
    for (tab, slot, a) in [
        (0x18eu32, 330u32, 0x1abu32),
        (0x1c7, 328, 0x1c2),
        (0x1e9, 329, 0x1e4),
    ] {
        let t = slot_target(g, w(tab), slot);
        let ang = g.read::<u32>(s.wrapping_add(w(a)));
        stack = vec![m, m, ang];
        g.call(t, 0, &stack);
    }
    let y = sub(fl(g, s.wrapping_add(w(0x200))), k(g, 0x208));
    g.write::<f32>(s.wrapping_add(w(0x212)), y);
    let (_, h4) = actor(g, 0x217, 0x21e, 0x222);
    let t = slot_target(g, w(0x227), 257);
    stack.splice(0..0, [h4, 0]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// [`actor_matrix`] in `magic_0588` (583 bytes, 1 overlay).
pub const ACTOR_MATRIX_0588: Sig = Sig {
    len: 583,
    hash: 0xe5bf_6eb6_805f_73aa,
    calls: &[
        // __security_check_cookie
        (0x23f, Callee::Any),
    ],
    imports: &[],
};

/// [`actor_matrix`] in `magic_0600` (583 bytes, 1 overlay).
pub const ACTOR_MATRIX_0600: Sig = Sig {
    len: 583,
    hash: 0x384c_7fd7_d25c_d220,
    calls: &[
        // __security_check_cookie
        (0x23f, Callee::Any),
    ],
    imports: &[],
};
