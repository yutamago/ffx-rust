//! `magic_0672.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Resets the camera (0x100016a0, internal, no arguments; `G_CAM`
/// operand `+0x08`): slots 357 `(cam, -1, 0x3c00)`, 356 `(cam, -1,
/// 0x2ee0)`, 357 and 356 `(cam, -1, -1)`, 399 `(cam, 1)` (the camera read
/// again, `+0x5e`), each seeing the earlier calls' words. Returns slot
/// 399's `eax`.
pub fn camera_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let cam = g.read::<u32>(w(0x8));
    let mut stack: Vec<u32> = Vec::new();
    for (op, slot, k) in [
        (0x1, 357, 0x3c00),
        (0x1d, 356, 0x2ee0),
        (0x32, 357, u32::MAX),
        (0x44, 356, u32::MAX),
    ] {
        let t = slot_target(g, w(op), slot);
        stack.splice(0..0, [cam, u32::MAX, k]);
        // call width: 12 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    let cam = g.read::<u32>(w(0x5e));
    let t = slot_target(g, w(0x56), 399);
    stack.splice(0..0, [cam, 1]);
    // call width: 14 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack).eax
}

/// [`camera_reset`] (111 bytes, 1 overlay).
pub const CAMERA_RESET: Sig = Sig {
    len: 111,
    hash: 0xf03e_2c3d_b987_4225,
    calls: &[],
    imports: &[],
};

/// Copies the camera's key words (0x100015f0, internal, no arguments;
/// `G_CAM` operand `+0x15`; a frame with the count at `ebp - 8`): for
/// each of the camera's slot 586 `(cam)` tracks `i` (signed), slot 584
/// `(cam, i)` gives the source keys and slot 585 `(cam, i, &n)` the
/// destination and their byte size `n`; the first word of every 16-byte
/// key is copied. Then slot 346 `(cam)` and slot 399 `(cam, 0)` (the
/// camera read again, `+0x93`; 346's word left under it). Returns slot
/// 399's `eax`.
pub fn copy_keys<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let n_at = ebp.wrapping_sub(8);
    let cam = g.read::<u32>(w(0x15));
    g.write::<u32>(ebp.wrapping_sub(4), cam);
    let t = slot_target(g, w(0x7), 586);
    let tracks = g.call(t, 0, &[cam]).eax.cast_signed();
    // ecx: the caller's, then 585's target, then the last copied word
    let mut ecx = fr.ecx;
    let mut i = 0i32;
    while i < tracks {
        let iu = i.cast_unsigned();
        let t = slot_target(g, w(0x32), 584);
        let src = g.call(t, 0, &[cam, iu]).eax;
        let t = slot_target(g, w(0x42), 585);
        let dst = g.call(t, t, &[cam, iu, n_at, cam, iu]).eax;
        ecx = t;
        let mut k = 0i32;
        while k < g.read::<i32>(n_at) {
            let ku = k.cast_unsigned();
            let v = g.read::<u32>(src.wrapping_add(ku));
            g.write::<u32>(dst.wrapping_add(ku), v);
            ecx = v;
            k += 0x10;
        }
        i += 1;
    }
    let t = slot_target(g, w(0x7d), 346);
    g.call(t, ecx, &[cam]);
    let c = g.read::<u32>(w(0x93));
    let t = slot_target(g, w(0x8b), 399);
    g.call(t, ecx, &[c, 0, cam]).eax
}

/// [`copy_keys`] (169 bytes, 1 overlay).
pub const COPY_KEYS: Sig = Sig {
    len: 169,
    hash: 0x43a6_d394_897e_9a23,
    calls: &[],
    imports: &[],
};

/// The fade-out task (0x10001b80, `(task)`, a task step): the countdown
/// `n = S[0x5b8]` (`S = *G_S`) down by one while above 0; slot 348
/// `(*G_B, *G_H, (80 - n) * 256 / 80, (n - 80) * 64 / 80, (n - 80) * 40 /
/// 80, 0)` (signed divisions); once the countdown is out the task goes
/// back to its pool (a tail jump into
/// [`give_back`](crate::sa_pool::give_back)). Returns slot 348's `eax` or
/// the give-back's.
pub fn fade_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x6));
    let c = g.read::<i32>(s + 0x5b8);
    if c > 0 {
        g.write::<i32>(s + 0x5b8, c - 1);
    }
    let n = g.read::<i32>(s + 0x5b8);
    let d = n.wrapping_sub(0x50);
    let a = d.wrapping_mul(40) / 80;
    let b = d.wrapping_shl(6) / 80;
    let e = 0x50i32.wrapping_sub(n).wrapping_shl(8) / 80;
    let h = g.read::<u32>(w(0x73));
    let t = slot_target(g, w(0x78), 348);
    let bg = g.read::<u32>(w(0x7e));
    let r = g
        .call(
            t,
            0,
            &[
                bg,
                h,
                e.cast_unsigned(),
                b.cast_unsigned(),
                a.cast_unsigned(),
                0,
            ],
        )
        .eax;
    if g.read::<i32>(s + 0x5b8) > 0 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x9e),
        task,
    ))
}

/// [`fade_task`] (164 bytes, 1 overlay).
pub const FADE_TASK: Sig = Sig {
    len: 164,
    hash: 0x0982_6d64_dfc8_364e,
    calls: &[(0x9e, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// Starts the fade (0x10001c30, no arguments): the colour block `E =
/// S + 0x280` (`S = *G_S`) set (`+0x80` 0xff, `+0x81` 0xff80ffff, `+0x85`
/// 0xffff, `+0x87` 0, `+0x88` 0, `+0x8c`/`+0x90` 30, `+0x94` 0, `+0x98`
/// 0x48); a pool node ([`take`](crate::sa_pool::take)) with `+0x14` =
/// `E` and `+0x4` 0 put at the head of the DLL's list
/// ([`push_head`](crate::o4_more::push_head)); slot 587 `(*G_B, 0)`.
/// Returns slot 587's `eax`.
pub fn fade_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x3)) + 0x280;
    g.write::<u32>(e + 0x81, 0xff80_ffff);
    g.write::<u8>(e + 0x80, 0xff);
    g.write::<u16>(e + 0x85, 0xffff);
    g.write::<u8>(e + 0x87, 0);
    g.write::<u32>(e + 0x88, 0);
    g.write::<u16>(e + 0x98, 0x48);
    g.write::<u32>(e + 0x90, 0x1e);
    g.write::<u32>(e + 0x8c, 0x1e);
    g.write::<u32>(e + 0x94, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x65), w(0xe));
    let empty = g.read::<u32>(w(0x6b)) == 0;
    g.write::<u32>(node + 0x14, e);
    g.write::<u32>(node + 4, 0);
    crate::o4_more::push_head(g, node, empty, (w(0x7f), w(0x99)), (w(0x91), w(0x99)));
    let t = slot_target(g, w(0x9e), 587);
    let b = g.read::<u32>(w(0xa6));
    Ret::eax(g.call(t, 0, &[b, 0]).eax)
}

/// [`fade_start`] (182 bytes, 1 overlay).
pub const FADE_START: Sig = Sig {
    len: 182,
    hash: 0xfa52_48bd_5b17_2637,
    calls: &[(0x65, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// The effect's start (0x10001d50, no arguments): seven globals from
/// immediates; the actors looked up ([`crate::sb_misc::lookup_actors`]);
/// slot 719 `(P, 8, 0)` into `G_H`; the part set up
/// ([`crate::sb_misc::part_setup`] `(BUF)`); the camera's angle and
/// keys reset ([`crate::o6_fam_pairs::angle_reset`],
/// [`camera_reset`]); slot 345 `(*G_O)`, slot 349 `(*G_O, *G_C)`; the
/// record table's pointer and the 32 records' first words 0; the script
/// block's callback, a word, a halfword, a byte and two counters; slot
/// 271 `(4)`; two task nodes ([`crate::sa_pool::take`], kinds 1 and 2,
/// callbacks `CB1`, `CB2`): the first linked after the list's last node
/// without moving `G_LAST` (or the list's first), the second after it.
/// Each call sees the words the earlier ones left. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    for o in [0x01u32, 0x0b, 0x15, 0x1f, 0x29, 0x33, 0x3d] {
        g.write::<u32>(w(o + 2), w(o + 6));
    }
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x48));
    let t = slot_target(g, w(0x4d), 719);
    // call width: 3
    let h = g.call(t, 0, &[w(0x5c), 8, 0]).eax;
    g.write::<u32>(w(0x66), h);
    // push esi; slot 719's 0 left; push BUF; the return address
    let esp = fr.esp.wrapping_sub(0x10);
    g.write::<u32>(esp + 8, 0);
    g.write::<u32>(esp + 4, w(0x6b));
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x70), esp);
    let callee = |at: u32| Frame {
        at: img.rel32_target(f + at),
        esp,
        ..Frame::default()
    };
    crate::o6_fam_pairs::angle_reset(g, img, &callee(0x75));
    camera_reset(g, img, &callee(0x7a));
    let o = g.read::<u32>(w(0x85));
    let mut stack = vec![o, w(0x6b), 0];
    let t = slot_target(g, w(0x7f), 345);
    // `ecx`: `G_H` (slot 719's answer), which the part set-up loaded
    // and nothing after changed
    // call width: 3
    g.call(t, h, &stack);
    let c = g.read::<u32>(w(0x94));
    stack.splice(0..0, [o, c]);
    let t = slot_target(g, w(0x99), 349);
    // call width: 5
    g.call(t, 0, &stack);
    g.write::<u32>(w(0xab), w(0xaf));
    let recs = w(0xb4);
    for i in 0..0x20u32 {
        g.write::<u32>(recs.wrapping_add(0x140 * i), 0);
    }
    g.write::<u32>(w(0xd7), w(0xdb));
    g.write::<u32>(w(0xe1), 0);
    g.write::<u16>(w(0xe8), 0);
    g.write::<u8>(w(0xee), w(0xf2) as u8);
    g.write::<u32>(w(0xfd), 0);
    g.write::<u32>(w(0x103), w(0x107));
    let t = slot_target(g, w(0xd0), 271);
    // call width: 1
    g.call(t, 0, &[4]);
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0x116), w(0x111));
    let mut head = g.read::<u32>(w(0x11c));
    g.write::<u8>(first.wrapping_add(0x18), 1);
    g.write::<u32>(first.wrapping_add(4), 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x148), first);
    } else {
        let last = g.read::<u32>(w(0x133));
        g.write::<u32>(first, last);
        g.write::<u32>(last.wrapping_add(4), first);
    }
    g.write::<u32>(first.wrapping_add(8), first);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0x155), w(0x14d));
    g.write::<u8>(second.wrapping_add(0x18), 2);
    g.write::<u32>(second.wrapping_add(4), 0);
    // the head is only 0 here when the first take gave 0 (which faults
    // before in the original); ported as written
    if head == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x181), second);
        g.write::<u32>(second.wrapping_add(8), second);
        g.write::<u32>(w(0x189), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first.wrapping_add(4), second);
        g.write::<u32>(second.wrapping_add(8), second);
        g.write::<u32>(w(0x172), second);
    }
    Ret::eax(0)
}

/// [`start`] (401 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 401,
    hash: 0x8981_30db_d352_60a8,
    calls: &[
        (0x48, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x70, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (
            0x75,
            Callee::Sig(&crate::o6_fam_pairs::ANGLE_RESET_0672_1500),
        ),
        (0x7a, Callee::Sig(&CAMERA_RESET)),
        (0x116, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x155, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The effect's task (0x100014c0, `(task)`, a task callback): the
/// objects ([`crate::l5_objhost::run_objects`]); when they say 1 and the
/// block `*G_B`'s `+0x5c0` is 1, its keys copied ([`copy_keys`]) and
/// slot 214 `(*G_A)`. Returns slot 214's `eax`, or the objects' answer.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let b = g.read::<u32>(w(0x03));
    // push esi; the return address
    let esp = fr.esp.wrapping_sub(8);
    let r = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x08), esp);
    if r != 1 || g.read::<u32>(b.wrapping_add(0x5c0)) != 1 {
        return Ret::eax(r);
    }
    // `ecx`: the security cookie run_objects' check left there (its
    // global, the operand at `+0x0a` of run_objects)
    let run = img.rel32_target(f + 0x08);
    let callee = Frame {
        at: img.rel32_target(f + 0x1a),
        esp,
        ecx: g.read::<u32>(img.word(run + 0x0a)),
        ..Frame::default()
    };
    copy_keys(g, img, &callee);
    let a = g.read::<u32>(w(0x25));
    let t = slot_target(g, w(0x1f), 214);
    // call width: 1
    Ret::eax(g.call(t, 0, &[a]).eax)
}

/// [`task`] (54 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 54,
    hash: 0xbc93_11d4_9dfe_9895,
    calls: &[
        (0x08, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x1a, Callee::Sig(&COPY_KEYS)),
    ],
    imports: &[],
};

/// A sprite of the packed descriptor (0x10001710, a task callback, cdecl
/// `(node)`; a /GS frame of 0x90 bytes, `ebp` = `esp - 4`): with the
/// block `B = *G_B`, its fade `n` is `(s - c) * 80 / s` while the
/// count-up `c = B[0x5a8]` is above 0 (it counts down; `s = B[0x5ac]`),
/// else `B[0x5b0] - 1` (stored) `* 80 / B[0x5b4]`; the angle `B[0x5a4]
/// += DA`, less `T` above `T`; slot 719 `(P, 2, 7)` gives the descriptor
/// `(lo, hi)`; slot 235 `(M)` (`ebp - 0x44`), slot 210 `(*G_O, M2)`
/// (`ebp - 0x84`); the sprite drawn
/// ([`crate::o6_fam_pairs6::packed_sprite`], `ecx` = `*G_O`, `(-, M, lo,
/// hi, 0, angle, n)`); once `B[0x5b0]` is not above 0 the node goes back
/// ([`crate::sa_pool::give_back`]). Returns the give-back's `eax`, or
/// `B`.
pub fn packed_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let node = fr.arg(g, 0);
    let b = g.read::<u32>(w(0x19));
    let c = g.read::<i32>(b + 0x5a8);
    let n = if c > 0 {
        let s = g.read::<i32>(b + 0x5ac);
        let c = c - 1;
        g.write::<i32>(b + 0x5a8, c);
        s.wrapping_sub(c)
            .wrapping_mul(80)
            .checked_div(s)
            .unwrap_or(0)
    } else {
        let left = g.read::<i32>(b + 0x5b0).wrapping_sub(1);
        g.write::<i32>(b + 0x5b0, left);
        left.wrapping_mul(80)
            .checked_div(g.read::<i32>(b + 0x5b4))
            .unwrap_or(0)
    };
    let mut angle = add(g.read::<f32>(b + 0x5a4), g.read::<f32>(w(0x76)));
    let turn = g.read::<f32>(w(0x7e));
    g.write::<f32>(b + 0x5a4, angle);
    if angle > turn {
        angle = sub(angle, turn);
        g.write::<f32>(b + 0x5a4, angle);
    }
    let o = g.read::<u32>(w(0xa8));
    let t = slot_target(g, w(0xa2), 719);
    // call width: 3
    let d = g.call(t, 0, &[w(0xb7), 2, 7]).eax;
    let (lo, hi) = (g.read::<u32>(d), g.read::<u32>(d + 4));
    let ebp = fr.esp.wrapping_sub(4);
    let (m, m2) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x84));
    let t = slot_target(g, w(0xc7), 235);
    // call width: 4 (slot 719's words under it)
    g.call(t, 0, &[m, w(0xb7), 2, 7]);
    let a = g.read::<u32>(w(0xe1));
    let t = slot_target(g, w(0xdb), 210);
    // call width: 6 (the earlier calls' words under it)
    g.call(t, 0, &[a, m2, m, w(0xb7), 2, 7]);
    let angle = g.read::<u32>(b + 0x5a4);
    // the frame, ebx/esi/edi, n, 8 bytes and four pushes (the first a
    // register, which the callee doesn't read), the return address
    let esp = ebp.wrapping_sub(0xbc);
    for (i, v) in (1u32..).zip([0, m, lo, hi, 0, angle, n.cast_unsigned()]) {
        g.write::<u32>(esp + 4 * i, v);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x11e),
        esp,
        ecx: o,
        ..Frame::default()
    };
    crate::o6_fam_pairs6::packed_sprite(g, img, &callee);
    if g.read::<i32>(b + 0x5b0) > 0 {
        return Ret::eax(b);
    }
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x13e),
        node,
    ))
}

/// [`packed_frame`] (336 bytes, 1 overlay).
pub const PACKED_FRAME: Sig = Sig {
    len: 336,
    hash: 0x3c07_d896_fc01_a8ed,
    calls: &[
        (
            0x11e,
            Callee::Sig(&crate::o6_fam_pairs6::PACKED_SPRITE_0672_1310),
        ),
        (0x13e, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        // __security_check_cookie
        (0x148, Callee::Any),
    ],
    imports: &[],
};

/// A spark object spawned now and then (0x100018e0 and 0x10001a30, task
/// callbacks, cdecl `(rec)`): the record's countdown `+0x2c` drops; at 0
/// it restarts at `rand() % 6 + 5` and an object from the pool
/// ([`crate::sb_misc::pool_take_140`]; none: returns 0) is started
/// ([`crate::s4_misc::start_object_plain`], part `K` (the `push imm8` at
/// `+0x6a`), the scale `(slot 314 (0.2) + A) * B`); its `+0xd0..+0xdc`
/// and `+0xc4` from the block `*G_B` (`+0x270..`, `+0x264`); `+0xe0`
/// slot 314 `(30)` `+ C`, `+0xe4` slot 314 `(X)` `- D`, `+0xe8` slot 314
/// `(30)` `+ C` (each float slot gets the one word on the stack; ST0
/// goes through the record argument's word as `f32`). Returns the last
/// slot 314's `eax`, the record's (none spawned: the caller's `eax`).
pub fn spawn<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let rec = fr.arg(g, 0);
    let left = g.read::<i32>(rec + 0x2c).wrapping_sub(1);
    g.write::<i32>(rec + 0x2c, left);
    let blk = g.read::<u32>(w(0x11));
    if left > 0 {
        return Ret::eax(fr.eax);
    }
    let r = g.rand().cast_signed() % 6 + 5;
    g.write::<i32>(rec + 0x2c, r);
    let obj = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + 0x30));
    if obj == 0 {
        return Ret::eax(0);
    }
    let arg = fr.esp + 4;
    let float = |g: &mut G, table: u32, x: u32| {
        let t = slot_target(g, w(table), 314);
        // call width: 1
        let out = g.call(t, 0, &[x]);
        let v = crate::ob_model::st0_f32(out.st0);
        g.write::<f32>(arg, v);
        (v, out.eax)
    };
    let (a, _) = float(g, 0x41, w(0x48));
    let scale = mul(add(a, k(g, 0x60)), k(g, 0x6f));
    let index = w(0x6a) & 0xff;
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x74), obj, scale, index);
    for (src, dst) in [
        (0x270u32, 0xd0u32),
        (0x274, 0xd4),
        (0x278, 0xd8),
        (0x27c, 0xdc),
        (0x264, 0xc4),
    ] {
        let v = g.read::<u32>(blk.wrapping_add(src));
        g.write::<u32>(obj.wrapping_add(dst), v);
    }
    let (a, _) = float(g, 0xb5, w(0xc5));
    g.write::<f32>(obj + 0xe0, add(a, k(g, 0xd7)));
    let (a, _) = float(g, 0xdc, w(0xe3));
    g.write::<f32>(obj + 0xe4, sub(a, k(g, 0x103)));
    let (a, eax) = float(g, 0x108, w(0x10f));
    g.write::<f32>(obj + 0xe8, add(a, k(g, 0x12f)));
    Ret::eax(eax)
}

/// [`spawn`] in `magic_0672` (0x100018e0, part 1).
pub const SPAWN_18E0: Sig = Sig {
    len: 322,
    hash: 0x5c71_0a3a_25c4_e748,
    calls: &[
        (0x30, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0x74, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
    ],
    imports: &[(0x1d, "rand")],
};

/// [`spawn`] in `magic_0672` (0x10001a30, part 0).
pub const SPAWN_1A30: Sig = Sig {
    len: 322,
    hash: 0x3be2_5e45_813c_cd25,
    calls: &[
        (0x30, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0x74, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
    ],
    imports: &[(0x1d, "rand")],
};
