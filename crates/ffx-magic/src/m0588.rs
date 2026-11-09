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
///
/// `magic_0430` 0x10003630 (559 bytes, `r26::ACTOR_MATRIX_0430`) is the
/// same code without the step down by `K2` and with slot 17's product
/// taken the other way round (`movss` at `+0x18d` where the others load
/// the slot table there); its last operands sit 0x18 bytes earlier (the
/// `push 0` before the actor's, 0x1a).
pub fn actor_matrix<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let v0430 = img.word(f + 0x18d) as u8 == 0xf3;
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
    let v = if v0430 {
        mul(fl(g, m + 0x28), r)
    } else {
        mul(r, fl(g, m + 0x28))
    };
    g.write::<f32>(m + 0x28, v);
    let mut stack = Vec::new();
    for (tab, slot, a) in [
        (if v0430 { 0x19bu32 } else { 0x18e }, 330u32, 0x1abu32),
        (0x1c7, 328, 0x1c2),
        (0x1e9, 329, 0x1e4),
    ] {
        let t = slot_target(g, w(tab), slot);
        let ang = g.read::<u32>(s.wrapping_add(w(a)));
        stack = vec![m, m, ang];
        g.call(t, 0, &stack);
    }
    let (ptr, d, tab, imp) = if v0430 {
        (0x1fd, 0x206, 0x20a, 0x20f)
    } else {
        let y = sub(fl(g, s.wrapping_add(w(0x200))), k(g, 0x208));
        g.write::<f32>(s.wrapping_add(w(0x212)), y);
        (0x217, 0x21e, 0x222, 0x227)
    };
    let (_, h4) = actor(g, ptr, d, tab);
    let t = slot_target(g, w(imp), 257);
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

/// Spawns two objects every 100 frames (0x10002420, an overlay table
/// entry, no arguments; family P076 with `magic_0600` 0x10007250, the
/// effect block's offsets read from the code): the count `*G_FX + C`
/// (`C` the displacement at `+0x9`) steps down; while above 0 nothing
/// more (the caller's `eax` returned). Else it is 100 again and two
/// records from the pool ([`pool_take_140`](crate::sb_misc::pool_take_140);
/// none: 0 returned) start as objects
/// ([`start_object_plain`](crate::s4_misc::start_object_plain): scale
/// `*K` `+0x38` index 1, `*K` `+0x96` index 0) at the point `*G_FX + P`
/// (`+0x4a`), their `+0xc4` the angle `*G_FX + A` (`+0x7a`), which it
/// returns.
pub fn spawn_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x3));
    let count = fx.wrapping_add(w(0x9));
    let c = g.read::<u32>(count).wrapping_sub(1);
    g.write::<u32>(count, c);
    if c.cast_signed() > 0 {
        return Ret::eax(fr.eax);
    }
    g.write::<u32>(count, 100);
    // `magic_0430`'s copy (`r24::SPAWN_PAIR_0430`) starts animated
    // objects (`push 0` at +0x41 before the index,
    // [`start_object_anim`](crate::s4_misc::start_object_anim) with 0):
    // its operands 2 bytes later per block
    let anim = img.word(f + 0x41) & 0xffff == 0x006a;
    let d = if anim { 2 } else { 0 };
    let (point, angle) = (fx.wrapping_add(w(0x4a + d)), fx.wrapping_add(w(0x7a + d)));
    let mut a = 0;
    for (take, start, k, index) in [
        (0x26, 0x44 + d, 0x38, 1),
        (0x88 + d, 0xa2 + 2 * d, 0x96 + d, 0),
    ] {
        let obj = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + take));
        if obj == 0 {
            return Ret::eax(0);
        }
        let scale = g.read::<f32>(w(k));
        let at = img.rel32_target(f + start);
        if anim {
            crate::s4_misc::start_object_anim(g, img, at, obj, scale, index, 0);
        } else {
            crate::s4_misc::start_object_plain(g, img, at, obj, scale, index);
        }
        for i in 0..4 {
            let v = g.read::<u32>(point + 4 * i);
            g.write::<u32>(obj + 0xd0 + 4 * i, v);
        }
        a = g.read::<u32>(angle);
        g.write::<u32>(obj + 0xc4, a);
    }
    Ret::eax(a)
}

/// [`spawn_pair`] (232 bytes; family P076).
pub const SPAWN_PAIR: Sig = Sig {
    len: 232,
    hash: 0x7f56_a91a_bcad_2937,
    calls: &[
        (0x26, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0x44, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x88, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0xa2, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
    ],
    imports: &[],
};

/// [`spawn_pair`] in `magic_0600` (0x10007250).
pub const SPAWN_PAIR_0600: Sig = Sig {
    len: 232,
    hash: 0x96e1_f6be_cb3e_e69b,
    calls: SPAWN_PAIR.calls,
    imports: &[],
};

/// Starts the effect block's two objects (0x10002230, an overlay table
/// entry, no arguments): `*G_FX + 0x130` (scale `*K` `+0x4`, index 1)
/// and `*G_FX` itself (scale `*K` `+0x2c`, index 0), each
/// ([`start_object_plain`](crate::s4_misc::start_object_plain)) at the
/// point `+0xc04` lowered by `K` (`+0x66`, `+0xc6`), its `+0xc4` the
/// angle `+0xdf4`, which it returns.
pub fn objects_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0xc));
    let mut a = 0;
    for (o, k, call, index, low) in [
        (fx.wrapping_add(0x130), 0x4, 0x1e, 1, 0x66),
        (fx, 0x2c, 0x86, 0, 0xc6),
    ] {
        let scale = g.read::<f32>(w(k));
        crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + call), o, scale, index);
        for i in 0..4 {
            let v = g.read::<u32>(fx + 0xc04 + 4 * i);
            g.write::<u32>(o + 0xd0 + 4 * i, v);
        }
        let y = sub(g.read::<f32>(o + 0xd4), g.read::<f32>(w(low)));
        a = g.read::<u32>(fx + 0xdf4);
        g.write::<u32>(o + 0xc4, a);
        g.write::<f32>(o + 0xd4, y);
    }
    Ret::eax(a)
}

/// [`objects_start`] (228 bytes, 1 overlay).
pub const OBJECTS_START: Sig = Sig {
    len: 228,
    hash: 0xd7c5_851d_5cad_37fd,
    calls: &[
        (0x1e, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x86, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
    ],
    imports: &[],
};

/// Restarts the effect's light (0x10002320, an overlay table entry, no
/// arguments; a cookie frame): the scale task of `*G_FX + 0xd7c`
/// ([`scale_task_fixed`](crate::s4_small::scale_task_fixed): `xmm1` 0,
/// `xmm2` = `*K` `+0x14`, the entry's `ecx` pushed under it); `+0xd88` 0
/// and `+0xd98` 1.0. When the object list `+0x3b` held `*G_FX + 0x980`
/// ([`list_remove`](crate::l5_objhost::list_remove)): `**G_P` (`+0x5d`)
/// = the immediate `+0x64`, slot 713 `(obj)` and the object's flag bit
/// 0x8000 cleared. Then `+0x204` and `+0xd4` = `+0xc08`, and slot 332
/// gets the local copy (`ebp - 0x14`) of the four floats at operands
/// `+0xa0`, `+0xad`, `+0xba`, `+0xca`. Returns slot 332's `eax`.
pub fn light_restart<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x1c));
    // push ebp; sub esp, 0x14; push esi, edi, ecx; the return address
    let ebp = fr.esp.wrapping_sub(4);
    let esp = ebp.wrapping_sub(0x14 + 0x10);
    g.write::<u32>(esp + 4, fr.ecx);
    let mut xmm = fr.xmm;
    xmm[1] = 0;
    xmm[2] = u128::from(g.read::<u32>(w(0x14)));
    let task = Frame {
        at: img.rel32_target(f + 0x2b),
        esp,
        ecx: fx.wrapping_add(0xd7c),
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_fixed(g, img, &task);
    g.write::<u32>(fx + 0xd88, 0);
    g.write::<u32>(fx + 0xd98, 0x3f80_0000);
    let obj = fx.wrapping_add(0x980);
    if crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x54), w(0x3b), obj) != 0 {
        g.write::<u32>(g.read::<u32>(w(0x5d)), w(0x64));
        let t = slot_target(g, w(0x69), 713);
        g.call(t, 0, &[obj]);
        let v = g.read::<u16>(obj + 0x124);
        g.write::<u16>(obj + 0x124, v & 0x7fff);
    }
    let y = g.read::<u32>(fx + 0xc08);
    g.write::<u32>(fx + 0x204, y);
    g.write::<u32>(fx + 0xd4, y);
    let local = ebp.wrapping_sub(0x14);
    for (i, op) in [0xa0u32, 0xad, 0xba, 0xca].into_iter().enumerate() {
        let v = g.read::<u32>(w(op));
        g.write::<u32>(local + 4 * i as u32, v);
    }
    let t = slot_target(g, w(0xd0), 332);
    Ret::eax(g.call(t, 0, &[local]).eax)
}

/// [`light_restart`] (244 bytes, 1 overlay).
pub const LIGHT_RESTART: Sig = Sig {
    len: 244,
    hash: 0x9e96_9a47_f602_237d,
    calls: &[
        (0x2b, Callee::Sig(&crate::s4_small::SCALE_TASK_1)),
        (0x54, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        // __security_check_cookie
        (0xec, Callee::Any),
    ],
    imports: &[],
};
