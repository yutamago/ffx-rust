//! Round 13 (magic-r13): DLL start-ups that finish their overlay (with
//! their [`r9_load`](crate::r9_load) copies), ported one per DLL.

use crate::r11_fades::stores;
use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Two objects, one at the effect and one at the DLL's own actor
/// (`magic_0318` 0x10001340, 400 bytes, a /GS frame with `V` at `ebp -
/// 0x14`): `*G_A`, `*G_B` and two words set (`+0x11` .. `+0x2f`), the
/// actors looked up ([`lookup_actors`](crate::sb_misc::lookup_actors)),
/// slot 719 `(NAME, 8, 0)` into `G_PART` (`+0x58`), the part set up; the
/// object `OBJ1` (`+0x6d`, scale `*K1` `+0x65`) starts
/// ([`start_object_at`](crate::s4_misc::start_object_at), index 0, the
/// owners the immediates `+0x78`, `+0x72`), slot 262 `(*G_H, P, P + 4, P +
/// 8)` (`+0x9f`; the immediates `+0x94`, `+0x99`, `+0x89`); `P.w` 1.0
/// (`+0xbc`) and `OBJ2` (`+0xb1`, scale `+0xa9`, index 1, no owners)
/// starts; slot 262 `(*G_H, Q, Q + 4, Q + 8)` (`+0xe7`, `+0xe2`, `+0xdd`),
/// slot 268 `(*G_H, 6, &V)`; `Q.w` 1.0 (`+0x112`), `Q.y` = `V.y` less `K2`
/// (`+0x10b`, into `+0x125`); the DLL's task starts
/// ([`task_start`](crate::s3_misc::task_start), the word `+0x11d`), the
/// actor block is filled ([`me_block`](crate::sa_misc::me_block), `+0x12f`,
/// after `+0x133`'s store); a pool node ([`take`](crate::sa_pool::take),
/// the owner `+0x146`) is appended to the task list (first `+0x151`, last
/// `+0x162`). Returns 0.
pub fn two_objects_0318<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; sub esp, 0x14; push esi
    let ebp = fr.esp.wrapping_sub(4);
    let esp = ebp.wrapping_sub(0x18);
    let v_at = ebp.wrapping_sub(0x14);
    stores(g, img, f, &[0x11, 0x1b, 0x25, 0x2f]);
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x3a));
    let s719 = slot_target(g, w(0x3f), 0xb3c / 4);
    // call width: 3
    let part = g.call(s719, 0, &[w(0x4e), 8, 0]).eax;
    g.write::<u32>(w(0x58), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x5d));
    let scale = k(g, 0x65);
    crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + 0x7f),
        w(0x6d),
        scale,
        0,
        w(0x78),
        w(0x72),
    );
    let h = g.read::<u32>(w(0x9f));
    let s262 = slot_target(g, w(0x84), 0x418 / 4);
    // call width: 4
    g.call(s262, 0, &[h, w(0x99), w(0x94), w(0x89)]);
    let scale = k(g, 0xa9);
    stores(g, img, f, &[0xbc]);
    crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0xc7), w(0xb1), scale, 1, 0, 0);
    let h = g.read::<u32>(w(0xd2));
    let s262 = slot_target(g, w(0xcc), 0x418 / 4);
    // call width: 4
    g.call(s262, 0, &[h, w(0xe7), w(0xe2), w(0xdd)]);
    let s268 = slot_target(g, w(0xf3), 0x430 / 4);
    // call width: 3
    g.call(s268, 0, &[h, 6, v_at]);
    let y = sub(g.read::<f32>(v_at.wrapping_add(4)), k(g, 0x10b));
    stores(g, img, f, &[0x112]);
    g.write::<f32>(w(0x125), y);
    // the slot 268 call's last word stays; push the task's word; call
    crate::r12h_start::task_start(
        g,
        img,
        img.rel32_target(f + 0x12a),
        esp.wrapping_sub(0xc),
        w(0x11d),
    );
    stores(g, img, f, &[0x133]);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x13e), w(0x12f));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x14b), w(0x146));
    let first = g.read::<u32>(w(0x151));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x174), node);
    } else {
        let last = g.read::<u32>(w(0x162));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x17f), node);
    Ret::eax(0)
}

/// [`two_objects_0318`] (`magic_0318` 0x10001340).
pub const TWO_OBJECTS_0318: Sig = Sig {
    len: 400,
    hash: 0x0819_144a_e0d5_e78c,
    calls: &[
        (0x3a, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x5d, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x7f, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0xc7, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x12a, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x13e, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x14b, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x188, Callee::Any),
    ],
    imports: &[],
};

/// The effect's sparks each frame (`magic_0505` 0x10001600, 135 bytes, an
/// overlay table entry, no arguments; a /GS frame with the point `V` at
/// `ebp - 0x14`): `V` = slot 237 `(*G_C, 0, 0, &V)` (`+0x25`); six spark
/// records scattered around it ([`spark_record`](crate::m0505::spark_record)
/// `(&V)`, the earlier pushes left on the stack); the sprites stepped
/// ([`sprites_step_global`](crate::s7_misc::sprites_step_global)) and drawn
/// ([`sprite_frames`](crate::s9_misc::sprite_frames), `dt` 0, the slot
/// call's first word under it); its result into `G_X` (`+0x7a`) and
/// returned.
pub fn sparks_frame_0505<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; sub esp, 0x14
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let s0 = v;
    let c = g.read::<u32>(w(0x25));
    let t = slot_target(g, w(0x15), 0x3b4 / 4);
    // call width: 4 (the words stay)
    g.call(t, 0, &[c, 0, 0, v]);
    for i in 0..4u32 {
        let x = [c, 0, 0, v][i as usize];
        g.write::<u32>(s0.wrapping_sub(0x10).wrapping_add(4 * i), x);
    }
    let mut esp = s0.wrapping_sub(0x10);
    for call in [0x30u32, 0x39, 0x42, 0x4b, 0x54, 0x5d] {
        esp = esp.wrapping_sub(4);
        g.write::<u32>(esp, v);
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp: esp.wrapping_sub(4),
            ..*fr
        };
        crate::m0505::spark_record(g, img, &callee);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x62),
        esp: esp.wrapping_sub(4),
        ..*fr
    };
    crate::s7_misc::sprites_step_global(g, img, &callee);
    // add esp, 0x24: the slot call's first word (`&V`) stays; xorps xmm3
    let callee = Frame {
        at: img.rel32_target(f + 0x6d),
        esp: s0.wrapping_sub(8),
        xmm: [0; 4],
        ..*fr
    };
    let x = crate::s9_misc::sprite_frames(g, img, &callee).eax;
    g.write::<u32>(w(0x7a), x);
    Ret::eax(x)
}

/// [`sparks_frame_0505`] (`magic_0505` 0x10001600).
pub const SPARKS_FRAME_0505: Sig = Sig {
    len: 135,
    hash: 0x850e_bce5_d2a2_6ccb,
    calls: &[
        (0x30, Callee::Sig(&crate::m0505::SPARK_RECORD)),
        (0x39, Callee::Sig(&crate::m0505::SPARK_RECORD)),
        (0x42, Callee::Sig(&crate::m0505::SPARK_RECORD)),
        (0x4b, Callee::Sig(&crate::m0505::SPARK_RECORD)),
        (0x54, Callee::Sig(&crate::m0505::SPARK_RECORD)),
        (0x5d, Callee::Sig(&crate::m0505::SPARK_RECORD)),
        (0x62, Callee::Sig(&crate::s7_misc::SPRITES_STEP_GLOBAL)),
        (0x6d, Callee::Sig(&crate::s9_misc::SPRITE_FRAMES)),
        // __security_check_cookie
        (0x7f, Callee::Any),
    ],
    imports: &[],
};

/// The effect's set-up (`magic_0505` 0x10001690, 569 bytes, a table
/// entry, cdecl `(arg)`): slot 803 `(0x1000000, 0x10)` allocates the block
/// (into `+0x19`, `+ 0x80000` into `+0x23`, `+ 0x100000` into `+0x2d`);
/// `G_ARG` (`+0x35`) = `arg`; the part `P` (`+0x3f`, stored at `+0x4b`):
/// slot 718 `(P)`, slot 222 `(P, 0, 1000, 0x2000)`, slot 719 `(P, 8, 0)`
/// gives `H` (`G_H` `+0x96`); with `**G_LOCK` 1 around it slot 711 `(H, Q,
/// 0)`; slot 710 `(R, H, block, 0x80000)`; slot 941 on `H[0x14]`,
/// `H[0x18]`, `H[0x1c]` into three lists; two 0x110-byte blocks cleared
/// with their words set after each (`+0x14b` .. `+0x1a5`); slot 260
/// `(0x14)` into `+0x1ba`; slot 719 `(P, 4, 0)` into `+0x1df`, 0x30
/// bytes cleared at `+0x1da` and two set (u16 `+0x1f3` = 0x80, `+0x1f7`);
/// the pool cleared ([`clear`](crate::m0505::clear)); four words set
/// (`+0x209` .. `+0x227`). Returns 1.
#[allow(clippy::too_many_lines)] // one step per call of the original
pub fn set_up_0505<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ecx = fr.ecx;
    let arg = fr.arg(g, 0);
    let t = w(0x04);
    let slot = |g: &G, n: u32| slot_target(g, t, n);
    // call width: 2
    let block = g.call(slot(g, 0xc8c / 4), ecx, &[w(0x12), 0x10]).eax;
    g.write::<u32>(w(0x19), block);
    let b2 = block.wrapping_add(w(0x1e));
    g.write::<u32>(w(0x23), b2);
    g.write::<u32>(w(0x2d), b2.wrapping_add(w(0x28)));
    g.write::<u32>(w(0x35), arg);
    let p = w(0x3f);
    stores(g, img, f, &[0x49]);
    // call width: 1
    g.call(slot(g, 0xb38 / 4), ecx, &[p]);
    let part = g.read::<u32>(w(0x6e));
    // call width: 4
    g.call(slot(g, 0x378 / 4), ecx, &[part, 0, w(0x66), w(0x5b)]);
    let part = g.read::<u32>(w(0x85));
    // call width: 3
    let h = g.call(slot(g, 0xb3c / 4), ecx, &[part, 8, 0]).eax;
    let lock = g.read::<u32>(w(0x8d));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x96), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(slot(g, 0xb1c / 4), lock, &[h, w(0x9b), 0]);
    let lock = g.read::<u32>(w(0xb4));
    let blk = g.read::<u32>(w(0xbf));
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0xc7));
    // call width: 4
    g.call(slot(g, 0xb18 / 4), lock, &[w(0xd1), h, blk, w(0xb9)]);
    let mut list = 0;
    for (k, (at, store)) in (0u32..).zip([(0xdeu32, 0xf6u32), (0xfb, 0x110), (0x115, 0)]) {
        let h = g.read::<u32>(w(at));
        let key = g.read::<u32>(h.wrapping_add(0x14 + 4 * k));
        // call width: 1
        list = g.call(slot(g, 0xeb4 / 4), lock, &[key]).eax;
        if store != 0 {
            g.write::<u32>(w(store), list);
        }
    }
    g.write::<u32>(w(0x136), list);
    g.write_bytes(w(0x131), &[0u8; 0x110]);
    stores(g, img, f, &[0x14b, 0x155, 0x15f, 0x169]);
    g.write_bytes(w(0x147), &[0u8; 0x110]);
    stores(g, img, f, &[0x17d, 0x187, 0x191, 0x19b, 0x1a5]);
    // call width: 1
    let x = g.call(slot(g, 0x410 / 4), lock, &[0x14]).eax;
    g.write::<u32>(w(0x1ba), x);
    let part = g.read::<u32>(w(0x1cf));
    // call width: 3
    let x = g.call(slot(g, 0xb3c / 4), lock, &[part, 4, 0]).eax;
    g.write::<u32>(w(0x1df), x);
    g.write_bytes(w(0x1da), &[0u8; 0x30]);
    g.write::<u16>(w(0x1f3), 0x80);
    stores(g, img, f, &[0x1f7]);
    crate::m0505::clear(g, img, img.rel32_target(f + 0x202));
    stores(g, img, f, &[0x209, 0x213, 0x21d, 0x227]);
    Ret::eax(1)
}

/// [`set_up_0505`] (`magic_0505` 0x10001690; the `rel32`s: `memset`
/// three times and the pool's clear).
pub const SET_UP_0505: Sig = Sig {
    len: 569,
    hash: 0xe90c_f215_7b51_92d9,
    calls: &[
        (0x13b, Callee::Import("memset")),
        (0x174, Callee::Import("memset")),
        (0x1e4, Callee::Import("memset")),
        (0x202, Callee::Sig(&crate::m0505::CLEAR)),
    ],
    imports: &[],
};

/// The effect's set-up's second half (`magic_0669` 0x10001640, 138
/// bytes, a table entry, no arguments): slot 719 `(*G_P, 4, 0)` into
/// `+0x18` and `(*G_P, 4, 1)` into `+0x3d`; 0x30 bytes cleared at `+0x38`
/// (`memset`) and two set (u16 `+0x50` = 0x80, `+0x54`'s word); the
/// records cleared ([`records_clear`](crate::s4_small::records_clear), a
/// word left under it); three words 0 (`+0x66` .. `+0x7a`). Returns 1.
pub fn set_up_tail_0669<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ecx = fr.ecx;
    let p = g.read::<u32>(w(0x11));
    let t = slot_target(g, w(0x01), 0xb3c / 4);
    // call width: 3
    let a = g.call(t, ecx, &[p, 4, 0]).eax;
    g.write::<u32>(w(0x18), a);
    let p = g.read::<u32>(w(0x2d));
    let t = slot_target(g, w(0x1d), 0xb3c / 4);
    // call width: 3
    let b = g.call(t, ecx, &[p, 4, 1]).eax;
    g.write::<u32>(w(0x3d), b);
    g.write_bytes(w(0x38), &[0u8; 0x30]);
    g.write::<u16>(w(0x50), 0x80);
    stores(g, img, f, &[0x54]);
    // add esp, 0x20: the first call's last word stays
    let callee = Frame {
        at: img.rel32_target(f + 0x5f),
        esp: fr.esp.wrapping_sub(8),
        ..*fr
    };
    crate::s4_small::records_clear(g, img, &callee);
    stores(g, img, f, &[0x66, 0x70, 0x7a]);
    Ret::eax(1)
}

/// [`set_up_tail_0669`] (`magic_0669` 0x10001640; the `rel32`s: `memset`
/// and the records' clear).
pub const SET_UP_TAIL_0669: Sig = Sig {
    len: 138,
    hash: 0x3e83_1680_9222_369b,
    calls: &[
        (0x42, Callee::Import("memset")),
        (0x5f, Callee::Sig(&crate::s4_small::RECORDS_CLEAR_500)),
    ],
    imports: &[],
};

/// The effect's sparks each frame (`magic_0669` 0x100016d0, 313 bytes, a
/// table entry, no arguments; a /GS frame with the point `V` at `ebp -
/// 0x14`): `V` from slot 325 `(&V, Q)` (`+0x14`); the two counts `*G_A`
/// (`+0x32`) and `*G_B` (`+0x5f`) clamped to `[K1, K2]` (`+0x2a`, `+0x3a`;
/// stored only when clamped, a NaN kept); `A` spark records
/// ([`spark_record`](crate::m0669::spark_record) `(&V)`; `A` truncated,
/// read again after each) and `B` short ones
/// ([`spark_record_short`](crate::m0669::spark_record_short)); the sprites
/// stepped ([`sprites_step_global`](crate::s7_misc::sprites_step_global));
/// the live records (first word set) of `*G_N` at `*G_R` (`+0xe0`,
/// `+0xe5`) counted into `+0x104`; the sprites drawn
/// ([`sprite_frames`](crate::s9_misc::sprite_frames), `dt` 0, the count's
/// last `ecx` under it) into `+0x10e`; slot 382 `(0, 0)`. Returns 1.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn sparks_frame_0669<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use core::cmp::Ordering;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; sub esp, 0x14
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let s0 = v;
    let t = slot_target(g, w(0x1a), 0x514 / 4);
    // call width: 2
    g.call(t, fr.ecx, &[v, w(0x14)]);
    let (lo, hi) = (k(g, 0x2a), k(g, 0x3a));
    // `comiss lo, x; jbe` then `comiss x, hi; jbe`: stored only when out
    // of range (a NaN stays)
    let clamp = |g: &mut G, at: u32| {
        let x = g.read::<f32>(at);
        if lo.partial_cmp(&x) == Some(Ordering::Greater) {
            g.write::<f32>(at, lo);
            lo
        } else if x.partial_cmp(&hi) == Some(Ordering::Greater) {
            g.write::<f32>(at, hi);
            hi
        } else {
            x
        }
    };
    let a = clamp(g, w(0x32));
    clamp(g, w(0x5f));
    // push esi: each record's call sees `&V` below it
    let esp = s0.wrapping_sub(8);
    g.write::<u32>(esp, v);
    let callee = |at: u32| Frame {
        at: img.rel32_target(f + at),
        esp: esp.wrapping_sub(4),
        ..*fr
    };
    let mut n = crate::sb_sse::cvtt(a);
    let mut i = 0i32;
    while i < n {
        g.write::<u32>(esp, v);
        crate::m0669::spark_record(g, img, &callee(0x95));
        n = crate::sb_sse::cvtt(g.read::<f32>(w(0x9d)));
        i += 1;
    }
    let mut n = crate::sb_sse::cvtt(g.read::<f32>(w(0xad)));
    let mut i = 0i32;
    while i < n {
        g.write::<u32>(esp, v);
        crate::m0669::spark_record_short(g, img, &callee(0xc5));
        n = crate::sb_sse::cvtt(g.read::<f32>(w(0xcd)));
        i += 1;
    }
    let step = Frame {
        at: img.rel32_target(f + 0xda),
        esp,
        ..*fr
    };
    crate::s7_misc::sprites_step_global(g, img, &step);
    // pop esi; the live records counted (`ecx` the count left, 0 after
    // the loop)
    let mut left = g.read::<u32>(w(0xe0));
    let mut rec = g.read::<u32>(w(0xe5));
    let mut live = 0u32;
    if left.cast_signed() > 0 {
        while left != 0 {
            if g.read::<u32>(rec) != 0 {
                live += 1;
            }
            rec = rec.wrapping_add(0x94);
            left -= 1;
        }
    }
    g.write::<u32>(s0.wrapping_sub(4), left);
    g.write::<u32>(w(0x104), live);
    let draw = Frame {
        at: img.rel32_target(f + 0x109),
        esp: s0.wrapping_sub(8),
        xmm: [0; 4],
        ..*fr
    };
    let x = crate::s9_misc::sprite_frames(g, img, &draw).eax;
    g.write::<u32>(w(0x10e), x);
    let t = slot_target(g, w(0x113), 0x5f8 / 4);
    // call width: 2
    g.call(t, fr.ecx, &[0, 0]);
    Ret::eax(1)
}

/// [`sparks_frame_0669`] (`magic_0669` 0x100016d0).
pub const SPARKS_FRAME_0669: Sig = Sig {
    len: 313,
    hash: 0x4a63_e164_7d44_6fa5,
    calls: &[
        (0x95, Callee::Sig(&crate::m0669::SPARK_RECORD)),
        (0xc5, Callee::Sig(&crate::m0669::SPARK_RECORD_SHORT)),
        (0xda, Callee::Sig(&crate::s7_misc::SPRITES_STEP_GLOBAL)),
        (0x109, Callee::Sig(&crate::s9_misc::SPRITE_FRAMES)),
        // __security_check_cookie
        (0x131, Callee::Any),
    ],
    imports: &[],
};

/// The effect's sparks each frame (`magic_0488` 0x100011e0, 188 bytes, a
/// table entry, no arguments; a /GS frame, `L` at `ebp - 0x44`, `P` at
/// `ebp - 0x14`): for each of the five keys from `T` (`+0x13`, to
/// `+0x4b`) slot 210 `(key, &L)` and ten sparks
/// ([`spark_take`](crate::m0488::spark_take), `ecx` the pool `+0x37`,
/// `(&P)`); each live record of the pool (count `+0x52`, records
/// `+0x58`, read again after each live one) gets `+0x68` = the word
/// `4 * rec[0x28]` below `K` (`+0x6c`); the sprites stepped
/// ([`sprites_step`](crate::s6_misc::sprites_step), `+0x88`) and drawn
/// ([`block_sprite_frames`](crate::s9_misc::block_sprite_frames), `+0x96`,
/// `dt` 0, the step's `ecx` under it — the port's is the pool's) into
/// `+0xa6`. Returns 1.
pub fn sparks_frame_0488<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; sub esp, 0x44; push esi; push edi
    let ebp = fr.esp.wrapping_sub(4);
    let (l, p) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x14));
    let s0 = ebp.wrapping_sub(0x4c);
    let pool = w(0x37);
    let mut key_at = w(0x13);
    loop {
        let key = g.read::<u32>(key_at);
        let t = slot_target(g, w(0x1c), 0x348 / 4);
        // call width: 2
        g.call(t, fr.ecx, &[key, l]);
        for _ in 0..10 {
            g.write::<u32>(s0.wrapping_sub(4), p);
            let callee = Frame {
                at: img.rel32_target(f + 0x3c),
                esp: s0.wrapping_sub(8),
                ecx: pool,
                ..*fr
            };
            crate::m0488::spark_take(g, img, &callee);
        }
        key_at = key_at.wrapping_add(4);
        if key_at.cast_signed() >= w(0x4b).cast_signed() {
            break;
        }
    }
    let mut n = g.read::<u32>(w(0x52));
    let mut rec = g.read::<u32>(w(0x58));
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        if g.read::<u32>(rec) != 0 {
            let back = g.read::<u32>(rec.wrapping_add(0x28)) << 2;
            let x = g.read::<u32>(w(0x6c).wrapping_sub(back));
            g.write::<u32>(rec.wrapping_add(0x68), x);
            n = g.read::<u32>(w(0x78));
        }
        i += 1;
        rec = rec.wrapping_add(0x94);
    }
    let step = Frame {
        at: img.rel32_target(f + 0x8d),
        esp: s0.wrapping_sub(4),
        ecx: w(0x88),
        ..*fr
    };
    crate::s6_misc::sprites_step(g, img, &step);
    // push ecx (what the step left in it: not read by the draw)
    g.write::<u32>(s0.wrapping_sub(4), pool);
    let draw = Frame {
        at: img.rel32_target(f + 0x9b),
        esp: s0.wrapping_sub(8),
        ecx: w(0x96),
        xmm: [0; 4],
        ..*fr
    };
    let x = crate::s9_misc::block_sprite_frames(g, img, &draw).eax;
    g.write::<u32>(w(0xa6), x);
    Ret::eax(1)
}

/// [`sparks_frame_0488`] (`magic_0488` 0x100011e0).
pub const SPARKS_FRAME_0488: Sig = Sig {
    len: 188,
    hash: 0xd81a_d290_031e_a0e5,
    calls: &[
        (0x3c, Callee::Sig(&crate::m0488::SPARK_TAKE)),
        (0x8d, Callee::Sig(&crate::s6_misc::SPRITES_STEP)),
        (0x9b, Callee::Sig(&crate::s9_misc::BLOCK_SPRITE_FRAMES_B)),
        // __security_check_cookie
        (0xb4, Callee::Any),
    ],
    imports: &[],
};

/// An object started at the caster with random spin and size
/// (`magic_0360` 0x10001930, 335 bytes, a callback, cdecl `(O)`): `a` =
/// `rand() / R * A` and `s` = `rand() / R * B + B` (`R` `+0x18`/`+0x39`,
/// `A` `+0x20`, `B` `+0x46`/`+0x66`); `O+0xbc |= 0x4000`, the place
/// `O+0xd0..` = `*G_C`'s `+0xd0..+0xd8` (`+0x3e`) with `w` 0, `+0x100`,
/// `+0x104`, `+0x10c` 0, `+0x108` = `a`, the scale `+0x130..+0x138` = `s`
/// (`+0x13c` 0), the colour `+0x170..` (1, 1, 1, 0.3); slot 458 `(*G_H,
/// O, 0)` (`+0x8b`), slot 450 `(O, F1)` and slot 449 `(O, F2)` (the
/// immediates `+0x127`, `+0x13a`). Returns slot 449's `eax`.
pub fn object_start_0360<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let r = g.rand();
    let a = mul(div(cvt(r), k(g, 0x18)), k(g, 0x20));
    let r = g.rand();
    let o = fr.arg(g, 0);
    let s = add(mul(div(cvt(r), k(g, 0x39)), k(g, 0x46)), k(g, 0x66));
    let c = g.read::<u32>(w(0x3e));
    let place: [u32; 3] =
        core::array::from_fn(|i| g.read::<u32>(c.wrapping_add(0xd0 + 4 * i as u32)));
    let flags = g.read::<u32>(o.wrapping_add(0xbc)) | 0x4000;
    g.write::<u32>(o.wrapping_add(0xbc), flags);
    for (i, x) in (0u32..).zip(place) {
        g.write::<u32>(o.wrapping_add(0xd0 + 4 * i), x);
    }
    for off in [0xdcu32, 0x100, 0x104] {
        g.write::<u32>(o.wrapping_add(off), 0);
    }
    g.write::<f32>(o.wrapping_add(0x108), a);
    g.write::<u32>(o.wrapping_add(0x10c), 0);
    for off in [0x130u32, 0x134, 0x138] {
        g.write::<f32>(o.wrapping_add(off), s);
    }
    g.write::<u32>(o.wrapping_add(0x13c), 0);
    for off in [0x170u32, 0x174, 0x178] {
        g.write::<u32>(o.wrapping_add(off), 0x3f80_0000);
    }
    g.write::<u32>(o.wrapping_add(0x17c), w(0x115));
    let h = g.read::<u32>(w(0x8b));
    let t = slot_target(g, w(0x6b), 0x728 / 4);
    // call width: 3
    g.call(t, fr.ecx, &[h, o, 0]);
    let t = slot_target(g, w(0x122), 0x708 / 4);
    // call width: 2
    g.call(t, fr.ecx, &[o, w(0x127)]);
    let t = slot_target(g, w(0x135), 0x704 / 4);
    // call width: 2
    Ret::eax(g.call(t, fr.ecx, &[o, w(0x13a)]).eax)
}

/// [`object_start_0360`] (`magic_0360` 0x10001930; `rand` through the
/// import table).
pub const OBJECT_START_0360: Sig = Sig {
    len: 335,
    hash: 0xb08e_32e5_f8ba_612b,
    calls: &[],
    imports: &[(0x07, "rand")],
};

/// The effect's frame (`magic_0477` 0x10002350, 345 bytes, a table entry,
/// no arguments): nothing (0) while slot 253 says 0 or `*G_ON` (`+0x22`)
/// is 0; else `**G_P = *G_A` and `G_X = *G_B` (`+0x2b` .. `+0x3c`); for
/// each of five timelines (`ecx` the immediates `+0x57` .. `+0xd7`) whose
/// slot 261 `(k)` says so it runs
/// ([`timeline_run`](crate::sb_misc::timeline_run)); after the fifth ran
/// and answered 0, slot 214 `(*G_SND)` (`+0x106`); otherwise, when slot
/// 261 `(6)` says so, the DLL's timeline stops
/// ([`timeline_stop_global`](crate::sb_misc::timeline_stop_global)) and
/// slot 214 follows; either way returns 1. When slot 261 `(6)` says 0:
/// the two sprite blocks from `+0x120` (0x5c apart, up to `+0x143`)
/// stepped ([`sprites_step`](crate::s6_misc::sprites_step)) and drawn
/// ([`block_sprite_frames`](crate::s9_misc::block_sprite_frames), `dt` 0)
/// into `G_X` (`+0x13d`), and the frame count `+0x14b` goes up; returns 1.
pub fn frame_0477<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; push ecx; push esi
    let s0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
    let t = slot_target(g, w(0x08), 0x3f4 / 4);
    // call width: 0
    if g.call(t, fr.ecx, &[]).eax == 0 || g.read::<u32>(w(0x22)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x2b));
    let a = g.read::<u32>(w(0x30));
    g.write::<u32>(p, a);
    let b = g.read::<u32>(w(0x37));
    g.write::<u32>(w(0x3c), b);
    let ask = |g: &mut G, table: u32, k: u32| {
        let t = slot_target(g, w(table), 0x414 / 4);
        // call width: 1
        g.call(t, fr.ecx, &[k]).eax != 0
    };
    let mut fifth = None;
    for (k, (table, tl, call)) in (1u32..).zip([
        (0x41u32, 0x57u32, 0x5cu32),
        (0x61, 0x77, 0x7c),
        (0x81, 0x97, 0x9c),
        (0xa1, 0xb7, 0xbc),
        (0xc1, 0xd7, 0xdc),
    ]) {
        if ask(g, table, k) {
            let r = crate::sb_misc::timeline_run(g, img, img.rel32_target(f + call), w(tl));
            if k == 5 {
                fifth = Some(r);
            }
        }
    }
    // the fifth answering 0 goes straight to the sound; else slot 261 (6)
    let sound = fifth == Some(0) || {
        let asked = ask(g, 0xe5, 6);
        if asked {
            crate::sb_misc::timeline_stop_global(g, img, img.rel32_target(f + 0xfb));
        }
        asked
    };
    if sound {
        let snd = g.read::<u32>(w(0x106));
        let t = slot_target(g, w(0x100), 0x358 / 4);
        // call width: 1
        g.call(t, fr.ecx, &[snd]);
        return Ret::eax(1);
    }
    let mut block = w(0x120);
    loop {
        let step = Frame {
            at: img.rel32_target(f + 0x127),
            esp: s0.wrapping_sub(4),
            ecx: block,
            ..*fr
        };
        crate::s6_misc::sprites_step(g, img, &step);
        // push ecx (what the step left in it: the draw reads no stack word)
        g.write::<u32>(s0.wrapping_sub(4), block);
        let draw = Frame {
            at: img.rel32_target(f + 0x132),
            esp: s0.wrapping_sub(8),
            ecx: block,
            xmm: [0; 4],
            ..*fr
        };
        let x = crate::s9_misc::block_sprite_frames(g, img, &draw).eax;
        g.write::<u32>(w(0x13d), x);
        block = block.wrapping_add(0x5c);
        if block.cast_signed() >= w(0x143).cast_signed() {
            break;
        }
    }
    let n = g.read::<u32>(w(0x14b)).wrapping_add(1);
    g.write::<u32>(w(0x14b), n);
    Ret::eax(1)
}

/// [`frame_0477`] (`magic_0477` 0x10002350; the `rel32`s: the five
/// timelines' run, the stop, the sprites' step and draw).
pub const FRAME_0477: Sig = Sig {
    len: 345,
    hash: 0x9950_8dff_5392_c2e5,
    calls: &[
        (0x5c, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x7c, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x9c, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xbc, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xdc, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xfb, Callee::Sig(&crate::sb_misc::TIMELINE_STOP_GLOBAL)),
        (0x127, Callee::Sig(&crate::s6_misc::SPRITES_STEP)),
        (0x132, Callee::Sig(&crate::s9_misc::BLOCK_SPRITE_FRAMES)),
    ],
    imports: &[],
};

/// Five sparks taken around the caster (`magic_0477` 0x10001b60, 496
/// bytes, a table entry, no arguments; a /GS frame, `P` at `ebp -
/// 0x14`): `P` from slot 268 `(*G_C, 6, &P)` (`+0x1f`); five times a
/// sprite from the pool (`+0x47`;
/// [`sprite_take`](crate::s4_misc::sprite_take), `edx` = `*G_RES`
/// `+0x42`; none: the next) gets `+0x28`, `+0x24`, `+0x2c` the
/// immediates (`+0x63`, `+0x6a`, `+0x71`), `+0x4..+0xc` = `P` moved by
/// `((r / R - H) A) B`, `+0x18` = `r / R`, `+0x58` = slot 403 `(r / R S)`
/// (ST0 as `f32`) and `+0x54` = `r / R` times the double `D` (`r` slot
/// 324's low 15 bits each; `R` `+0x92`, `H` `+0x9a`, `A` `+0xa2`, `B`
/// `+0xaa`, `S` `+0x18f`, `D` `+0x1c9`). Returns 1.
pub fn sparks_0477<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; sub esp, 0x14; push esi; push edi
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let s0 = ebp.wrapping_sub(0x1c);
    let c = g.read::<u32>(w(0x1f));
    let t = slot_target(g, w(0x17), 0x430 / 4);
    // call width: 3
    g.call(t, fr.ecx, &[c, 6, p]);
    let table = w(0x5c);
    let draw = |g: &mut G| {
        let t = slot_target(g, table, 0x510 / 4);
        let tbl = g.read::<u32>(table);
        // call width: 0
        cvt(g.call(t, tbl, &[]).eax & 0x7fff)
    };
    for _ in 0..5 {
        let take = Frame {
            at: img.rel32_target(f + 0x4c),
            esp: s0.wrapping_sub(4),
            ecx: w(0x47),
            edx: g.read::<u32>(w(0x42)),
            ..*fr
        };
        let rec = crate::s4_misc::sprite_take(g, img, &take).eax;
        if rec == 0 {
            continue;
        }
        g.write::<u32>(rec.wrapping_add(0x28), w(0x63));
        g.write::<u32>(rec.wrapping_add(0x24), w(0x6a));
        g.write::<u32>(rec.wrapping_add(0x2c), w(0x71));
        for (i, ops) in (0u32..).zip([
            [0x92u32, 0x9a, 0xa2, 0xaa],
            [0xd5, 0xdd, 0xe5, 0xed],
            [0x118, 0x120, 0x128, 0x130],
        ]) {
            let r = draw(g);
            let d = mul(
                mul(sub(div(r, k(g, ops[0])), k(g, ops[1])), k(g, ops[2])),
                k(g, ops[3]),
            );
            let v = add(d, g.read::<f32>(p.wrapping_add(4 * i)));
            g.write::<f32>(rec.wrapping_add(4 + 4 * i), v);
        }
        let r = draw(g);
        g.write::<f32>(rec.wrapping_add(0x18), div(r, k(g, 0x15b)));
        let r = draw(g);
        let x = mul(div(r, k(g, 0x181)), k(g, 0x18f));
        let t = slot_target(g, w(0x179), 0x64c / 4);
        // call width: 1
        let out = g.call(t, fr.ecx, &[x.to_bits()]);
        g.write::<f32>(rec.wrapping_add(0x58), crate::ob_model::st0_f32(out.st0));
        let r = draw(g);
        let q = div(r, k(g, 0x1be));
        let d = g.read::<f64>(w(0x1c9));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: one rounding
        let v = (f64::from(q) * d) as f32;
        g.write::<f32>(rec.wrapping_add(0x54), v);
    }
    Ret::eax(1)
}

/// [`sparks_0477`] (`magic_0477` 0x10001b60; the `rel32`s: the sprite
/// take and the cookie check).
pub const SPARKS_0477: Sig = Sig {
    len: 496,
    hash: 0x9e46_e408_77b1_b4c0,
    calls: &[
        (0x4c, Callee::Sig(&crate::s4_misc::SPRITE_TAKE)),
        // __security_check_cookie
        (0x1e8, Callee::Any),
    ],
    imports: &[],
};

/// The summon's start (`magic_0076` 0x100069d0, 431 bytes, a table entry,
/// cdecl `(actor)`): `G_PART` = the part (`+0x0b`); the targets
/// ([`targets_init_b`](crate::s4_summon::targets_init_b)); the list words
/// and the two static nodes (`+0x25` .. `+0xaf`, halfwords `+0x61` 0x1ff
/// and `+0x92` 0xff) and `G_X` (`+0xbf`) set while slot 718 `(*G_PART)`
/// is pushed, then called; slot 719 `(*G_PART, 8, 2)` gives `G_H`
/// (`+0xf3`); slot 711 `(h, PROG, 0)` with `**G_LOCK` 1 around it; slot
/// 710 `(ENV, *G_H, BUF, 0x100000)`; slots 222 and 221 `(*G_PART, 0,
/// 1000, 2)`; slot 719 `(*G_PART, 6, 1)` and slot 551 `(its eax, 0, 2)`;
/// the DLL's model start ([`model_start_0076`](crate::m0076::model_start_0076)).
/// Returns 0. Each slot call passes the words the original leaves on the
/// stack.
pub fn summon_start_0076<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let actor = fr.arg(g, 0);
    stores(g, img, f, &[0x0b]);
    // push ebp; and esp, -8; push ecx; push esi; call
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc);
    crate::s4_summon::targets_init_b(g, img, img.rel32_target(f + 0x16), esp, actor);
    let t = w(0x1b);
    let part = g.read::<u32>(w(0x21));
    stores(g, img, f, &[0x25, 0x2f, 0x39, 0x43, 0x4d, 0x57]);
    g.write::<u16>(w(0x64), 0x1ff);
    stores(g, img, f, &[0x6a, 0x74, 0x7e, 0x88]);
    g.write::<u16>(w(0x95), 0xff);
    stores(g, img, f, &[0x9b, 0xa5, 0xaf, 0xbf]);
    let mut s = Pushes::default();
    s.push(part);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, 0xb38 / 4), 0);
    s.pop(1);
    let part = g.read::<u32>(w(0xdf));
    s.push_all(&[2, 8, part]);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot_target(g, w(0xcc), 0xb3c / 4), 0).eax;
    s.pop(3);
    let lock = g.read::<u32>(w(0xe7));
    let saved = g.read::<u32>(lock);
    s.push(0);
    g.write::<u32>(w(0xf3), h);
    s.push_all(&[w(0xf8), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xfe), 0xb1c / 4), 0);
    s.pop(3);
    let lock = g.read::<u32>(w(0x111));
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0x131));
    s.push_all(&[0x0010_0000, w(0x12b), h, w(0x136)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x11b), 0xb18 / 4), 0);
    let part = g.read::<u32>(w(0x152));
    s.push_all(&[2, 1000, 0, part]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x13d), 0x378 / 4), 0);
    let part = g.read::<u32>(w(0x16e));
    s.push_all(&[2, 1000, 0, part]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x159), 0x374 / 4), 0);
    let part = g.read::<u32>(w(0x185));
    s.push_all(&[1, 6, part]);
    // call width: 12 (Pushes caps the words at 12)
    let m = s.call(g, slot_target(g, w(0x175), 0xb3c / 4), 0).eax;
    s.pop(15);
    s.push_all(&[2, 0, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x194), 0x89c / 4), 0);
    let callee = Frame {
        at: img.rel32_target(f + 0x1a4),
        esp,
        ..*fr
    };
    crate::m0076::model_start_0076(g, img, &callee);
    Ret::eax(0)
}

/// [`summon_start_0076`] (`magic_0076` 0x100069d0; the `rel32`s: the
/// targets and the model start).
pub const SUMMON_START_0076: Sig = Sig {
    len: 431,
    hash: 0xb8a6_4c48_97bb_5135,
    calls: &[
        (0x16, Callee::Sig(&crate::s4_summon::TARGETS_INIT_B)),
        (0x1a4, Callee::Sig(&crate::m0076::MODEL_START_0076)),
    ],
    imports: &[],
};

/// The head of `magic_0343`'s and `magic_0106`'s summon starts (cdecl
/// `(actor)`): the targets ([`targets_init`](crate::s4_summon::targets_init),
/// the `rel32` at `+0x08`); the list words and the two static nodes
/// (`+0x11` .. `+0x9b`, halfwords `+0x4d` 0x1ff and `+0x7e` 0xff) and
/// `G_X` (`+0xb0`) set while slot 718 `(PART)` (`+0xac`) is pushed, then
/// called; slot 719 `(PART, 8, 0)` gives `G_H` (`+0xdd`); slot 711 `(h,
/// PROG, 0)` with `**G_LOCK` 1 around it; slot 710 `(ENV, *G_H, BUF,
/// 0x100000)`; slot 222 `(PART, 0, N, 2)` (`N` 7 in `0343`, a byte; 1000
/// in `0106`, a word, which moves the operands after it on by `d` = 3)
/// and slot 221 `(PART, 0, 1000, 2)`. Returns the words left on the stack.
fn summon_head<G: Game>(g: &mut G, img: &Image, fr: &Frame, d: u32) -> crate::pushes::Pushes {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let actor = fr.arg(g, 0);
    // push ebp; push esi; call
    let esp = fr.esp.wrapping_sub(0xc);
    crate::s4_summon::targets_init(g, img, img.rel32_target(f + 0x08), esp, actor);
    let t = w(0x0d);
    stores(g, img, f, &[0x11, 0x1b, 0x25, 0x2f, 0x39, 0x43]);
    g.write::<u16>(w(0x50), 0x1ff);
    stores(g, img, f, &[0x56, 0x60, 0x6a, 0x74]);
    g.write::<u16>(w(0x81), 0xff);
    stores(g, img, f, &[0x87, 0x91, 0x9b]);
    let mut s = crate::pushes::Pushes::default();
    s.push(w(0xac));
    stores(g, img, f, &[0xb0]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, 0xb38 / 4), 0);
    s.push_all(&[0, 8, w(0xcc)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot_target(g, w(0xbd), 0xb3c / 4), 0).eax;
    let lock = g.read::<u32>(w(0xd4));
    let saved = g.read::<u32>(lock);
    s.push(0);
    g.write::<u32>(w(0xdd), h);
    s.push_all(&[w(0xe2), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xe8), 0xb1c / 4), 0);
    let lock = g.read::<u32>(w(0xfb));
    s.push(0x0010_0000);
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0x112));
    s.push_all(&[w(0x10c), h, w(0x11d)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x107), 0xb18 / 4), 0);
    // `push 7` (6a 07) or `push 0x3e8` (68 e8 03 00 00)
    let n = if d == 0 { w(0x131) & 0xff } else { w(0x131) };
    s.push_all(&[2, n, 0, w(0x135 + d)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x124), 0x378 / 4), 0);
    s.push_all(&[2, 1000, 0, w(0x150 + d)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x13c + d), 0x374 / 4), 0);
    s
}

/// The summon's start (`magic_0343` 0x10007210, 444 bytes, a table entry,
/// cdecl `(actor)`): [`summon_head`] with slot 222's 7; then the node `N`
/// (`+0x165`) appended to the list (its last `L` = `*G_LAST` `+0x157` into
/// `N + 8`, words `+0x163` .. `+0x181`, `L + 4 = N`, the list's flag bit
/// 0 cleared, `G_LAST = N`) and two words 0. Returns 0. Each slot call
/// passes the words the original leaves on the stack.
pub fn summon_start_0343<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    summon_head(g, img, fr, 0);
    let last = g.read::<u32>(w(0x157));
    g.write::<u32>(w(0x15f), last);
    stores(g, img, f, &[0x163, 0x16d, 0x177, 0x181]);
    g.write::<u32>(last.wrapping_add(4), w(0x18e));
    let flags = g.read::<u32>(w(0x194)) & !1;
    g.write::<u32>(w(0x194), flags);
    stores(g, img, f, &[0x199, 0x1a3, 0x1ad]);
    Ret::eax(0)
}

/// [`summon_start_0343`] (`magic_0343` 0x10007210; the `rel32`: the
/// targets).
pub const SUMMON_START_0343: Sig = Sig {
    len: 444,
    hash: 0xebb5_24d2_2f6a_9a20,
    calls: &[(0x08, Callee::Sig(&crate::s4_summon::TARGETS_INIT))],
    imports: &[],
};

/// The summon's start (`magic_0106` 0x10007560, 493 bytes, a table entry,
/// cdecl `(actor)`): [`summon_head`] with slot 222's 1000; slot 719
/// `(PART, 6, 3)` and slot 551 `(its eax, 0, 2)`; the node `N` (`+0x192`)
/// appended to the list as [`summon_start_0343`] does (its words
/// `+0x18c` .. `+0x1af`, the last `+0x185`, `L + 4 = N` `+0x1bc`, the flag
/// `+0x1c7`, `G_LAST` `+0x1cc`), a word 0 (`+0x1d6`) and slot 575 `()`.
/// Returns 0.
pub fn summon_start_0106<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let mut s = summon_head(g, img, fr, 3);
    s.pop(19);
    s.push_all(&[3, 6, w(0x16c)]);
    // call width: 12 (Pushes caps the words at 12)
    let m = s.call(g, slot_target(g, w(0x15a), 0xb3c / 4), 0).eax;
    s.push_all(&[2, 0, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x178), 0x89c / 4), 0);
    s.pop(6);
    let last = g.read::<u32>(w(0x185));
    stores(g, img, f, &[0x18c, 0x196, 0x1a0]);
    g.write::<u32>(w(0x1ab), last);
    stores(g, img, f, &[0x1af]);
    g.write::<u32>(last.wrapping_add(4), w(0x1bc));
    let flags = g.read::<u32>(w(0x1c7)) & !1;
    g.write::<u32>(w(0x1c7), flags);
    stores(g, img, f, &[0x1cc, 0x1d6]);
    // call width: 0 (the words under it: none left)
    s.call(g, slot_target(g, w(0x1c1), 0x8fc / 4), 0);
    Ret::eax(0)
}

/// [`summon_start_0106`] (`magic_0106` 0x10007560; the `rel32`: the
/// targets).
pub const SUMMON_START_0106: Sig = Sig {
    len: 493,
    hash: 0x2d5b_b24c_307b_392e,
    calls: &[(0x08, Callee::Sig(&crate::s4_summon::TARGETS_INIT))],
    imports: &[],
};

/// The summon's start (`magic_0513` 0x100087d0, 457 bytes, a table entry,
/// cdecl `(actor)`): the targets
/// ([`targets_init`](crate::s4_summon::targets_init)); `G_X` and the list
/// words and the two static nodes (`+0x11` .. `+0xa5`, halfwords `+0x57`
/// 0x1ff and `+0x88` 0xff); the DLL's static node under the first one
/// ([`static_child`](crate::s4_small::static_child), `edx` the immediate
/// `+0x0d`); slot 718 `(PART)` (`+0xba`); slot 719 `(PART, 8, 0)` gives
/// `G_H` (`+0xe7`); slot 711 `(h, PROG, 0)` with `**G_LOCK` 1 around it;
/// slot 710 `(ENV, *G_H, BUF, 0x100000)`; slots 222 and 221 `(PART, 0,
/// 1000, 2)`; then the node `N` (`+0x172`) appended to the list as
/// [`summon_start_0343`] does, and two words 0. Returns 0. Each slot call
/// passes the words the original leaves on the stack.
pub fn summon_start_0513<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let actor = fr.arg(g, 0);
    // push ebp; push esi; call
    let esp = fr.esp.wrapping_sub(0xc);
    let eax = crate::s4_summon::targets_init(g, img, img.rel32_target(f + 0x08), esp, actor);
    stores(g, img, f, &[0x11, 0x1b, 0x25, 0x2f, 0x39, 0x43, 0x4d]);
    g.write::<u16>(w(0x5a), 0x1ff);
    stores(g, img, f, &[0x60, 0x6a, 0x74, 0x7e]);
    g.write::<u16>(w(0x8b), 0xff);
    stores(g, img, f, &[0x91, 0x9b, 0xa5]);
    let child = Frame {
        at: img.rel32_target(f + 0xb0),
        esp,
        eax,
        edx: w(0x0d),
        ..*fr
    };
    crate::s4_small::static_child(g, img, &child);
    let mut s = Pushes::default();
    s.push(w(0xba));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xb5), 0xb38 / 4), 0);
    s.push_all(&[0, 8, w(0xd6)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot_target(g, w(0xc7), 0xb3c / 4), 0).eax;
    let lock = g.read::<u32>(w(0xde));
    let saved = g.read::<u32>(lock);
    s.push(0);
    g.write::<u32>(w(0xe7), h);
    s.push_all(&[w(0xec), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xf2), 0xb1c / 4), 0);
    let lock = g.read::<u32>(w(0x105));
    s.push(0x0010_0000);
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0x11c));
    s.push_all(&[w(0x116), h, w(0x127)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x111), 0xb18 / 4), 0);
    s.push_all(&[2, 1000, 0, w(0x142)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x12e), 0x378 / 4), 0);
    s.push_all(&[2, 1000, 0, w(0x15d)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x149), 0x374 / 4), 0);
    let last = g.read::<u32>(w(0x164));
    g.write::<u32>(w(0x16c), last);
    stores(g, img, f, &[0x170, 0x17a, 0x184, 0x18e]);
    g.write::<u32>(last.wrapping_add(4), w(0x19b));
    let flags = g.read::<u32>(w(0x1a1)) & !1;
    g.write::<u32>(w(0x1a1), flags);
    stores(g, img, f, &[0x1a6, 0x1b0, 0x1ba]);
    Ret::eax(0)
}

/// [`summon_start_0513`] (`magic_0513` 0x100087d0; the `rel32`s: the
/// targets and the static child).
pub const SUMMON_START_0513: Sig = Sig {
    len: 457,
    hash: 0xcd1d_d8a9_5219_e1e8,
    calls: &[
        (0x08, Callee::Sig(&crate::s4_summon::TARGETS_INIT)),
        (0xb0, Callee::Sig(&crate::s4_small::STATIC_CHILD)),
    ],
    imports: &[],
};

/// The effect's set-up (`magic_0429` 0x10001240, 479 bytes, a table
/// entry, cdecl `(arg)`): slot 803 `(0x1000000, 0x10)` allocates the block
/// (into `+0x19`, `+ 0x80000` into `+0x23`); `G_ARG` (`+0x2b`) = `arg`;
/// the part `P` (`+0x35`, stored at `+0x3f`): slot 718 `(P)`, slots 222
/// and 221 `(P, 0, 1000, 0x2000)`, slot 719 `(P, 6, 0)` into `+0xa1`,
/// slot 719 `(P, 8, 0)` gives `H` (`G_H` `+0xca`); with `**G_LOCK` 1
/// around it slot 711 `(H, Q, 0)`; slot 710 `(R, H, block, 0x80000)`;
/// slot 941 on `H[0x14]`, `H[0x18]`, `H[0x1c]` into three lists; 0x110
/// bytes cleared at `+0x167` (`memset`) and its words set (`+0x17f` ..
/// `+0x1a7`); slot 218 `(*G_ARG, &arg)`'s first byte into `+0x1bd` and
/// slot 215 `(that byte)` into `+0x1d3`. Returns 1.
#[allow(clippy::too_many_lines)] // one step per call of the original
pub fn set_up_0429<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let arg = fr.arg(g, 0);
    let t = w(0x04);
    let slot = |g: &G, n: u32| slot_target(g, t, n);
    let mut s = Pushes::default();
    s.push_all(&[0x10, w(0x12)]);
    // call width: 12 (Pushes caps the words at 12)
    let block = s.call(g, slot(g, 0xc8c / 4), 0).eax;
    g.write::<u32>(w(0x19), block);
    g.write::<u32>(w(0x23), block.wrapping_add(w(0x1e)));
    g.write::<u32>(w(0x2b), arg);
    s.push(w(0x35));
    stores(g, img, f, &[0x3f]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot(g, 0xb38 / 4), 0);
    for (op, n) in [(0x64u32, 0x378u32 / 4), (0x83, 0x374 / 4)] {
        let p = g.read::<u32>(w(op));
        s.push_all(&[0x2000, 1000, 0, p]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot(g, n), 0);
    }
    let p = g.read::<u32>(w(0x9a));
    s.push_all(&[0, 6, p]);
    // call width: 12 (Pushes caps the words at 12)
    let x = s.call(g, slot(g, 0xb3c / 4), 0).eax;
    g.write::<u32>(w(0xa1), x);
    let p = g.read::<u32>(w(0xb6));
    s.push_all(&[0, 8, p]);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot(g, 0xb3c / 4), 0).eax;
    let lock = g.read::<u32>(w(0xbe));
    s.pop(17);
    let saved = g.read::<u32>(lock);
    s.push(0);
    g.write::<u32>(w(0xca), h);
    s.push_all(&[w(0xcf), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot(g, 0xb1c / 4), 0);
    let lock = g.read::<u32>(w(0xe8));
    let blk = g.read::<u32>(w(0xf3));
    s.push_all(&[0x0008_0000, blk]);
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0xfb));
    s.push_all(&[h, w(0x105)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot(g, 0xb18 / 4), 0);
    for (k, (at, store)) in (0u32..).zip([(0x112u32, 0x127u32), (0x12c, 0x141), (0x146, 0x15b)]) {
        let h = g.read::<u32>(w(at));
        let key = g.read::<u32>(h.wrapping_add(0x14 + 4 * k));
        s.push(key);
        // call width: 12 (Pushes caps the words at 12)
        let list = s.call(g, slot(g, 0xeb4 / 4), 0).eax;
        g.write::<u32>(w(store), list);
    }
    let m = w(0x167);
    s.push_all(&[0x110, 0, m]);
    g.write_bytes(m, &[0u8; 0x110]);
    let arg_at = fr.esp.wrapping_add(4);
    let a = g.read::<u32>(w(0x17b));
    s.push_all(&[arg_at, a]);
    stores(g, img, f, &[0x17f, 0x189, 0x193, 0x19d, 0x1a7]);
    // call width: 12 (Pushes caps the words at 12)
    let p = s.call(g, slot(g, 0x368 / 4), 0).eax;
    let b = u32::from(g.read::<u8>(p));
    g.write::<u32>(w(0x1bd), b);
    s.push(b);
    // call width: 12 (Pushes caps the words at 12)
    let x = s.call(g, slot(g, 0x35c / 4), 0).eax;
    g.write::<u32>(w(0x1d3), x);
    Ret::eax(1)
}

/// [`set_up_0429`] (`magic_0429` 0x10001240; the `rel32`: `memset`).
pub const SET_UP_0429: Sig = Sig {
    len: 479,
    hash: 0x5011_24d0_62c5_f263,
    calls: &[(0x16c, Callee::Import("memset"))],
    imports: &[],
};

/// The effect's measures from the caster to three targets (`magic_0313`
/// 0x10001000, 493 bytes, a callback, cdecl `(O)`; a /GS frame aligned to
/// 8, `E` = `((esp - 4) & !7) - 0x50` after the saved registers, `P` at
/// `E+0x20`, `Q` at `+0x30`, `V` at `+0x10`): with `A = *G_A` (`+0x15`)
/// and its count `n = A[0x19c]`, the three picks `n / 2` (toward 0), 0
/// and `n - 1` go to `G_I[0..3]` (`+0x2e`, `+0x4a`, `+0x36`); `P` = slot
/// 456 `(A[0x198], 12, &P)`. For each pick `i`: `Q` = slot 457 `(A[0x1a0 +
/// 4 G_I[i]], 6, &Q)`, `V` = `(Q.x - P.x, 0, Q.z - P.z, 0)`, `m` = the
/// largest slot 453 `(&V)` (ST0 as `f32`; NaN or not larger: kept) from
/// 0; slot 464 `(&V)`; `s` = `acos(V.z)` (MSVCR110's), `G_S[i]` (`+0x132`) = `K2 - s`
/// when `V.x > K1` (`+0x12a`, `+0x13c`), else `s`; then `G_S[i] = (K2 -
/// G_S[i]) K3` (`+0x150` .. `+0x16b`). `G_M` (`+0x199`) = `m / K4`
/// (`+0x182`), two words 0, slot 448 `(1, 0)`, slots 450 and 449 `(O, F)`
/// (`+0x1b9`, `+0x1cc`). Returns slot 449's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn measures_0313<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x50);
    let (v, p, q) = (
        e.wrapping_add(0x10),
        e.wrapping_add(0x20),
        e.wrapping_add(0x30),
    );
    let o = fr.arg(g, 0);
    let t = w(0x44);
    let a = g.read::<u32>(w(0x15));
    let first = g.read::<u32>(a.wrapping_add(0x198));
    let n = g.read::<u32>(a.wrapping_add(0x19c)).cast_signed();
    g.write::<u32>(w(0x2e), (n / 2).cast_unsigned());
    g.write::<u32>(w(0x36), n.wrapping_sub(1).cast_unsigned());
    stores(g, img, f, &[0x48]);
    // call width: 3
    g.call(slot_target(g, t, 0x720 / 4), fr.ecx, &[first, 0xc, p]);
    let mut m = 0.0f32;
    for i in 0..3u32 {
        let pick = g.read::<u32>(w(0x72).wrapping_add(4 * i));
        let a = g.read::<u32>(w(0x77));
        let id = g.read::<u32>(a.wrapping_add(pick.wrapping_mul(4)).wrapping_add(0x1a0));
        let mut s = Pushes::default();
        s.push_all(&[q, 6, id]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x8b), 0x724 / 4), fr.ecx);
        let dx = sub(g.read::<f32>(q), g.read::<f32>(p));
        s.push(v);
        g.write::<u32>(v.wrapping_add(0xc), 0);
        g.write::<f32>(v, dx);
        let dz = sub(
            g.read::<f32>(q.wrapping_add(8)),
            g.read::<f32>(p.wrapping_add(8)),
        );
        g.write::<u32>(v.wrapping_add(4), 0);
        g.write::<f32>(v.wrapping_add(8), dz);
        // call width: 12 (Pushes caps the words at 12)
        let out = s.call(g, slot_target(g, w(0xa9), 0x714 / 4), fr.ecx);
        let d = crate::ob_model::st0_f32(out.st0);
        g.write::<f32>(e.wrapping_add(0xc), d);
        if d > m {
            m = d;
        }
        g.write::<f32>(e.wrapping_add(8), m);
        // call width: 1
        g.call(slot_target(g, w(0xfd), 0x740 / 4), fr.ecx, &[v]);
        let z = f64::from(g.read::<f32>(v.wrapping_add(8)));
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss: round to f32
        let r = g.libm(crate::Libm::Acos, z) as f32;
        let at = w(0x132).wrapping_add(4 * i);
        g.write::<f32>(at, r);
        if g.read::<f32>(v) > k(g, 0x12a) {
            g.write::<f32>(at, sub(k(g, 0x13c), r));
        }
        let x = mul(
            sub(k(g, 0x150), g.read::<f32>(w(0x158).wrapping_add(4 * i))),
            k(g, 0x163),
        );
        g.write::<f32>(w(0x16b).wrapping_add(4 * i + 4), x);
    }
    let gm = div(g.read::<f32>(e.wrapping_add(8)), k(g, 0x182));
    let mut s = Pushes::default();
    s.push_all(&[0, 1]);
    g.write::<f32>(w(0x199), gm);
    stores(g, img, f, &[0x19d, 0x1a7]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x187), 0x700 / 4), fr.ecx);
    s.push_all(&[w(0x1b9), o]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1b4), 0x708 / 4), fr.ecx);
    s.push_all(&[w(0x1cc), o]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0x1c7), 0x704 / 4), fr.ecx).eax)
}

/// [`measures_0313`] (`magic_0313` 0x10001000; the `rel32`s: the arc
/// cosine and the cookie check).
pub const MEASURES_0313: Sig = Sig {
    len: 493,
    hash: 0x69b7_08a4_2505_66cb,
    calls: &[
        (0x116, Callee::Import("_libm_sse2_acos_precise")),
        (0x1e5, Callee::Any),
    ],
    imports: &[],
};

/// The effect's set-up (`magic_0664` 0x10001f40, 522 bytes, a table
/// entry, cdecl `(arg)`): `G_ARG` (`+0x07`) = `arg`; the part `P`
/// (`+0x18`): slot 718 `(P)` after three words set (`+0x1c` .. `+0x30`,
/// `G_PART` = `P` and the block words), slot 222 `(P, 0, 1000, 0x2000)`,
/// slot 719 `(P, 8, 0)` gives `H` (`G_H` `+0x7d`); with `**G_LOCK` 1 around
/// it slot 711 `(H, Q, 0)`; slot 710 `(R, H, *G_BLOCK, 0x80000)`; slot
/// 941 on `H[0x14]`, `H[0x18]`, `H[0x1c]` into three lists; two 0x110-byte
/// blocks cleared (`memset`) with their words set before (`+0x132` ..
/// `+0x1a0`) and after (`+0x1b4` .. `+0x1d2`) the second; slot 260 `(0)`
/// into `+0x1e7` and `(1)` into `+0x1fe`. Returns 1.
pub fn set_up_0664<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x07), arg);
    let t = w(0x0c);
    let slot = |g: &G, n: u32| slot_target(g, t, n);
    let mut s = Pushes::default();
    s.push(w(0x18));
    stores(g, img, f, &[0x1c, 0x26, 0x30]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot(g, 0xb38 / 4), 0);
    let p = g.read::<u32>(w(0x55));
    s.push_all(&[0x2000, 1000, 0, p]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot(g, 0x378 / 4), 0);
    let p = g.read::<u32>(w(0x6c));
    s.push_all(&[0, 8, p]);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot(g, 0xb3c / 4), 0).eax;
    let lock = g.read::<u32>(w(0x74));
    let saved = g.read::<u32>(lock);
    s.push(0);
    g.write::<u32>(w(0x7d), h);
    s.push_all(&[w(0x82), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot(g, 0xb1c / 4), 0);
    let lock = g.read::<u32>(w(0x9b));
    let blk = g.read::<u32>(w(0xa6));
    s.push_all(&[0x0008_0000, blk]);
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0xae));
    s.push_all(&[h, w(0xb8)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot(g, 0xb18 / 4), 0);
    let mut list = 0;
    for (k, (at, store)) in (0u32..).zip([(0xc5u32, 0xdau32), (0xdf, 0xf7), (0xfc, 0x11d)]) {
        let h = g.read::<u32>(w(at));
        let key = g.read::<u32>(h.wrapping_add(0x14 + 4 * k));
        s.push(key);
        // call width: 12 (Pushes caps the words at 12)
        list = s.call(g, slot(g, 0xeb4 / 4), 0).eax;
        if k == 0 {
            s.pop(16);
        }
        if k < 2 {
            g.write::<u32>(w(store), list);
        }
    }
    s.push_all(&[0x110, 0, w(0x118)]);
    g.write::<u32>(w(0x11d), list);
    g.write_bytes(w(0x118), &[0u8; 0x110]);
    s.push_all(&[0x110, 0, w(0x12e)]);
    stores(
        g,
        img,
        f,
        &[
            0x132, 0x13c, 0x146, 0x150, 0x15a, 0x164, 0x16e, 0x178, 0x182, 0x18c, 0x196, 0x1a0,
        ],
    );
    g.write_bytes(w(0x12e), &[0u8; 0x110]);
    stores(g, img, f, &[0x1b4, 0x1be, 0x1c8, 0x1d2]);
    s.push(0);
    // call width: 12 (Pushes caps the words at 12)
    let x = s.call(g, slot(g, 0x410 / 4), 0).eax;
    g.write::<u32>(w(0x1e7), x);
    s.push(1);
    // call width: 12 (Pushes caps the words at 12)
    let x = s.call(g, slot(g, 0x410 / 4), 0).eax;
    g.write::<u32>(w(0x1fe), x);
    Ret::eax(1)
}

/// [`set_up_0664`] (`magic_0664` 0x10001f40; the `rel32`s: `memset`
/// twice).
pub const SET_UP_0664: Sig = Sig {
    len: 522,
    hash: 0xf56c_43b2_97da_282b,
    calls: &[
        (0x122, Callee::Import("memset")),
        (0x1ab, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The summon's start (`magic_0090` 0x10006c80, 542 bytes, a table entry,
/// cdecl `(actor)`): the targets
/// ([`targets_init_b`](crate::s4_summon::targets_init_b)); the flag byte
/// `+0x11` copied to `+0x1d` and the halfword count `+0x18` up by one; the
/// list words, the static nodes and the node graph's words (`+0x26` ..
/// `+0x11d`, halfwords `+0x62` 0x1ff, `+0x7f` and `+0xa6` 0xff) and `G_X`
/// (`+0x132`) set; slot 718 `(PART)` (`+0x12e`); slot 719 `(PART, 8, 0)`
/// gives `G_H` (`+0x165`); slot 711 `(h, PROG, 0)` with `**G_LOCK` 1
/// around it; slot 710 `(ENV, *G_H, BUF, 0x100000)`; slots 222 and 221
/// `(PART, 0, 1000, 2)`; slot 719 `(PART, 6, 1)` and slot 551 `(its eax,
/// 0, 2)`; then the DLL's camera start ([`start_0090`](crate::m0089::start_0090)).
/// Returns 0. Each slot call passes the words the original leaves on the
/// stack.
pub fn summon_start_0090<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let actor = fr.arg(g, 0);
    // push ebp; and esp, -8; push ecx; push esi; call
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc);
    crate::s4_summon::targets_init_b(g, img, img.rel32_target(f + 0x0c), esp, actor);
    let flag = g.read::<u8>(w(0x11));
    let n = g.read::<u16>(w(0x18)).wrapping_add(1);
    g.write::<u16>(w(0x18), n);
    g.write::<u8>(w(0x1d), flag);
    let t = w(0x22);
    stores(g, img, f, &[0x26, 0x30, 0x3a, 0x44, 0x4e, 0x58]);
    g.write::<u16>(w(0x65), 0x1ff);
    stores(g, img, f, &[0x6b, 0x75]);
    g.write::<u16>(w(0x82), 0xff);
    stores(g, img, f, &[0x88, 0x92, 0x9c]);
    g.write::<u16>(w(0xa9), 0xff);
    stores(
        g,
        img,
        f,
        &[
            0xaf, 0xb9, 0xc3, 0xcd, 0xd7, 0xe1, 0xeb, 0xf5, 0xff, 0x109, 0x113, 0x11d,
        ],
    );
    let mut s = Pushes::default();
    s.push(w(0x12e));
    stores(g, img, f, &[0x132]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, 0xb38 / 4), 0);
    s.pop(1);
    s.push_all(&[0, 8, w(0x151)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot_target(g, w(0x13f), 0xb3c / 4), 0).eax;
    let lock = g.read::<u32>(w(0x159));
    s.pop(3);
    let saved = g.read::<u32>(lock);
    s.push(0);
    g.write::<u32>(w(0x165), h);
    s.push_all(&[w(0x16a), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x170), 0xb1c / 4), 0);
    let lock = g.read::<u32>(w(0x183));
    s.pop(3);
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0x1a3));
    s.push_all(&[0x0010_0000, w(0x19d), h, w(0x1a8)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x18d), 0xb18 / 4), 0);
    s.push_all(&[2, 1000, 0, w(0x1c3)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1af), 0x378 / 4), 0);
    s.push_all(&[2, 1000, 0, w(0x1de)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1ca), 0x374 / 4), 0);
    s.push_all(&[1, 6, w(0x1ee)]);
    // call width: 12 (Pushes caps the words at 12)
    let m = s.call(g, slot_target(g, w(0x1e9), 0xb3c / 4), 0).eax;
    s.pop(15);
    s.push_all(&[2, 0, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x203), 0x89c / 4), 0);
    let callee = Frame {
        at: img.rel32_target(f + 0x213),
        esp,
        ..*fr
    };
    crate::m0089::start_0090(g, img, &callee);
    Ret::eax(0)
}

/// [`summon_start_0090`] (`magic_0090` 0x10006c80; the `rel32`s: the
/// targets and the camera start).
pub const SUMMON_START_0090: Sig = Sig {
    len: 542,
    hash: 0xc30a_8f85_b662_9e40,
    calls: &[
        (0x0c, Callee::Sig(&crate::s4_summon::TARGETS_INIT_B)),
        (0x213, Callee::Sig(&crate::m0089::START_0090)),
    ],
    imports: &[],
};

/// A pool node appended to the DLL's task list (first `first`, last
/// `last`; operands of the inlined append): `+4` 0, the byte `+0x18` =
/// `kind`, linked after the last (or the first when empty; `empty` the
/// operands storing the first and the last then, `last_w` the last's
/// store otherwise).
pub(crate) fn append_kind<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    kind: u8,
    ops: [u32; 5],
) {
    let w = |o: u32| img.word(f + o);
    let [first, last, last_w, first_e, last_e] = ops;
    let head = g.read::<u32>(w(first));
    g.write::<u8>(node.wrapping_add(0x18), kind);
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(first_e), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(last_e), node);
    } else {
        let tail = g.read::<u32>(w(last));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(last_w), node);
    }
}

/// The effect's start-up (`magic_0394` 0x10005df0, 290 bytes, run by its
/// load step; no arguments): five words set (`+0x14` .. `+0x3c`) and
/// slot 719 `(PART, 8, 0)` (`+0x10`) into `+0x4c`; the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the caster placed
/// ([`place`](crate::m0394::place)); two words, a halfword and a byte
/// set (`+0x5f` .. `+0x7c`); slot 271 `(4)`; a pool node
/// ([`take`](crate::sa_pool::take), owner `+0x91`, kind 1) appended to
/// the task list; the records set up
/// ([`records_init`](crate::sa_misc::records_init), `+0xc7`); a second
/// node (owner `+0xd9`, kind 2) appended. Returns 0.
pub fn start_0394<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    stores(g, img, f, &[0x14, 0x1e, 0x28, 0x32, 0x3c]);
    let t = slot_target(g, w(0x01), 0xb3c / 4);
    // call width: 3
    let h = g.call(t, fr.ecx, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x4c), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x51));
    // add esp, 4: two of slot 719's words stay
    crate::m0394::place(g, img, img.rel32_target(f + 0x56), fr.esp.wrapping_sub(0xc));
    stores(g, img, f, &[0x5f, 0x69]);
    g.write::<u16>(w(0x76), 0);
    g.write::<u8>(w(0x7e), 1);
    let t = slot_target(g, w(0x5b), 0x43c / 4);
    // call width: 3 (two words of slot 719's under it)
    g.call(t, fr.ecx, &[4, 8, 0]);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x96), w(0x91));
    append_kind(g, img, f, node, 1, [0x9c, 0xb0, 0xcf, 0xc2, 0xcf]);
    crate::sa_misc::records_init(g, img, img.rel32_target(f + 0xd4), w(0xc7));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xde), w(0xd9));
    append_kind(g, img, f, node, 2, [0xe4, 0xf8, 0x105, 0x113, 0x11b]);
    Ret::eax(0)
}

/// [`start_0394`] (`magic_0394` 0x10005df0; the `rel32`s: the part's
/// set-up, the place, the pool's take twice and the records).
pub const START_0394: Sig = Sig {
    len: 290,
    hash: 0x9623_109a_6d0b_97ea,
    calls: &[
        (0x51, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x56, Callee::Sig(&crate::m0394::PLACE)),
        (0x96, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xd4, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0xde, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The aim start-up (run by the load step; no arguments; `magic_0348`
/// 0x10007490, 297 bytes, and `magic_0350` 0x10006b80, 317 bytes, which
/// sets a fifth word at `+0x3c` (its operands after it `d` = 0xa bytes
/// later) and resets a pool after the aim records): four or five words
/// set (`+0x14` ..) and slot 719 `(PART, 8, 0)` (`+0x10`) into `+0x42`;
/// the particle set-up ([`part_setup`](crate::sb_misc::part_setup), its
/// buffer `+0x47`); the sound list and the angle
/// ([`list_angle_reset`](crate::o6_fam_pairs2::list_angle_reset)); the
/// model's light ranges ([`model_ranges`](crate::s4_small::model_ranges));
/// two words, a halfword and a byte set (`+0x5f` .. `+0x7c`); slot 271
/// `(4)`; a pool node ([`take`](crate::sa_pool::take), owner `+0x91`,
/// kind 1) appended to the task list; the aim records
/// ([`aim_records`](crate::s7_misc::aim_records), `ecx` = `*G_B` (`+0xc8`)
/// plus the displacement `+0xd1`); `magic_0350`: the pool at `+0xea`
/// reset ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)); a
/// second node (owner `+0xe0`, kind 2) appended. Returns 0.
fn start_aim<G: Game>(g: &mut G, img: &Image, fr: &Frame, d: u32) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    stores(g, img, f, &[0x14, 0x1e, 0x28, 0x32]);
    if d != 0 {
        stores(g, img, f, &[0x3c]);
    }
    let t = slot_target(g, w(0x01), 0xb3c / 4);
    // call width: 3
    let h = g.call(t, fr.ecx, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x42 + d), h);
    // add esp, 8: slot 719's last word stays; push BUF; call
    let esp = fr.esp.wrapping_sub(0xc);
    let buf = w(0x47 + d);
    g.write::<u32>(esp.wrapping_add(4), buf);
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x4c + d), esp);
    let callee = |at: u32| Frame {
        at: img.rel32_target(f + at + d),
        esp,
        ..*fr
    };
    crate::o6_fam_pairs2::list_angle_reset(g, img, &callee(0x51));
    crate::s4_small::model_ranges(g, img, &callee(0x56));
    stores(g, img, f, &[0x5f + d, 0x69 + d]);
    g.write::<u16>(w(0x76 + d), 0);
    g.write::<u8>(w(0x7e + d), 1);
    let t = slot_target(g, w(0x5b + d), 0x43c / 4);
    // call width: 3 (the buffer and slot 719's last word under it)
    g.call(t, fr.ecx, &[4, buf, 0]);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x96 + d), w(0x91 + d));
    let head = g.read::<u32>(w(0x9c + d));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xc2 + d), node);
    } else {
        let tail = g.read::<u32>(w(0xb0 + d));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    let block = g.read::<u32>(w(0xc8 + d)).wrapping_add(w(0xd1 + d));
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0xd6 + d), node);
    let aim = Frame {
        at: img.rel32_target(f + 0xdb + d),
        esp: fr.esp.wrapping_sub(4),
        ecx: block,
        ..*fr
    };
    crate::s7_misc::aim_records(g, img, &aim);
    // `magic_0350`: mov ecx, POOL; call pool_reset_take (0x14 bytes)
    let e = if d == 0 {
        0
    } else {
        let at = img.rel32_target(f + 0xef);
        crate::o6_fam_pool::pool_reset_take(g, img, at, w(0xea));
        d + 0xa
    };
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xe5 + e), w(0xe0 + e));
    append_kind(
        g,
        img,
        f,
        node,
        2,
        [0xeb + e, 0xff + e, 0x10c + e, 0x11a + e, 0x122 + e],
    );
    Ret::eax(0)
}

/// [`start_aim`] in `magic_0348` (0x10007490).
pub fn start_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_aim(g, img, fr, 0)
}

/// [`start_aim`] in `magic_0350` (0x10006b80).
pub fn start_0350<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_aim(g, img, fr, 0xa)
}

/// [`start_0348`] (`magic_0348` 0x10007490; the `rel32`s: the particle
/// set-up, the list and angle, the light ranges, the pool's take twice
/// and the aim records).
pub const START_0348: Sig = Sig {
    len: 297,
    hash: 0x081d_e510_8243_16d4,
    calls: &[
        (0x4c, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (
            0x51,
            Callee::Sig(&crate::o6_fam_pairs2::LIST_ANGLE_RESET_0348_6600),
        ),
        (0x56, Callee::Sig(&crate::s4_small::MODEL_RANGES)),
        (0x96, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xdb, Callee::Sig(&crate::s7_misc::AIM_RECORDS)),
        (0xe5, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// [`start_0350`] (`magic_0350` 0x10006b80; the `rel32`s as
/// [`START_0348`]'s, and the pool's reset).
pub const START_0350: Sig = Sig {
    len: 317,
    hash: 0xed8d_0c9f_6970_a464,
    calls: &[
        (0x56, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (
            0x5b,
            Callee::Sig(&crate::o6_fam_pairs2::LIST_ANGLE_RESET_0350_5A50),
        ),
        (0x60, Callee::Sig(&crate::s4_small::MODEL_RANGES)),
        (0xa0, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xe5, Callee::Sig(&crate::s7_misc::AIM_RECORDS)),
        (
            0xef,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0350_2CF0),
        ),
        (0xf9, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The effect's start-up (`magic_0473` 0x10002100, 377 bytes, run by its
/// load step; no arguments): four words set (`+0x14` .. `+0x32`) and slot
/// 719 `(PART, 8, 0)` (`+0x10`) into `+0x42`; the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); with `B = *G_B`
/// (`+0x4c`): `B+0x5fc`, `+0x600`, `+0x5ac`, `+0x5d4`, `+0x610` 0,
/// `+0x604` 200, `+0x60c` and `+0x608` 60; four words, a halfword and a
/// byte set (`+0xa7` .. `+0xce`); slot 271 `(4)`; a pool node
/// ([`take`](crate::sa_pool::take), owner `+0xe1`, kind 1) appended to the
/// task list; the aim records ([`aim_records`](crate::s7_misc::aim_records),
/// `ecx` = `*G_B` (`+0x118`) plus the displacement `+0x121`); a second
/// node (owner `+0x130`, kind 2) appended. Returns 0.
pub fn start_0473<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    stores(g, img, f, &[0x14, 0x1e, 0x28, 0x32]);
    let t = slot_target(g, w(0x01), 0xb3c / 4);
    // call width: 3
    let h = g.call(t, fr.ecx, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x42), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x47));
    let b = g.read::<u32>(w(0x4c));
    for (o, v) in [
        (0x5fcu32, 0u32),
        (0x600, 0),
        (0x604, 0xc8),
        (0x60c, 0x3c),
        (0x608, 0x3c),
        (0x5ac, 0),
        (0x5d4, 0),
        (0x610, 0),
    ] {
        g.write::<u32>(b.wrapping_add(o), v);
    }
    stores(g, img, f, &[0xa7, 0xb1, 0xbb]);
    g.write::<u16>(w(0xc8), 0);
    g.write::<u8>(w(0xd0), 1);
    let t = slot_target(g, w(0xa3), 0x43c / 4);
    // call width: 3 (two of slot 719's words under it)
    g.call(t, fr.ecx, &[4, 8, 0]);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xe6), w(0xe1));
    let head = g.read::<u32>(w(0xec));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x112), node);
    } else {
        let tail = g.read::<u32>(w(0x100));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    let block = g.read::<u32>(w(0x118)).wrapping_add(w(0x121));
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x126), node);
    let aim = Frame {
        at: img.rel32_target(f + 0x12b),
        esp: fr.esp.wrapping_sub(4),
        ecx: block,
        ..*fr
    };
    crate::s7_misc::aim_records(g, img, &aim);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x135), w(0x130));
    append_kind(g, img, f, node, 2, [0x13b, 0x14f, 0x15c, 0x16a, 0x172]);
    Ret::eax(0)
}

/// [`start_0473`] (`magic_0473` 0x10002100; the `rel32`s: the part's
/// set-up, the pool's take twice and the aim records).
pub const START_0473: Sig = Sig {
    len: 377,
    hash: 0xac33_8b7e_41b8_ef63,
    calls: &[
        (0x47, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xe6, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x12b, Callee::Sig(&crate::s7_misc::AIM_RECORDS)),
        (0x135, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The effect's start-up (`magic_0325` 0x100013e0, 298 bytes, run by its
/// load step; no arguments): four words set (`+0x00` .. `+0x1e`); the
/// actors looked up ([`lookup_actors`](crate::sb_misc::lookup_actors));
/// slot 719 `(PART, 8, 0)` (`+0x3d`) into `+0x47`; the part's buffer
/// ([`parts_buffer`](crate::s4_small::parts_buffer)); two fade records
/// ([`fade_record`](crate::o4_fam5::fade_record): `ecx` `+0x63`, `xmm2`
/// / `xmm1` the floats `+0x54`/`+0x5c`, the word 0; then `+0x82`, the
/// floats the other way round (`+0x70`/`+0x78`), the word 0x12c, after
/// three of its words set (`+0x86` .. `+0x9a`)); three more words
/// (`+0xae` .. `+0xc2`); slot 271 `(4)`; a pool node
/// ([`take`](crate::sa_pool::take), owner `+0xda`) with `+0x14` = the
/// immediate `+0xed` and kind 2 appended to the task list. Returns 0.
pub fn start_0325<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| u128::from(g.read::<u32>(w(o)));
    stores(g, img, f, &[0x00, 0x0a, 0x14, 0x1e]);
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x29));
    let t = slot_target(g, w(0x2e), 0xb3c / 4);
    // call width: 3
    let h = g.call(t, fr.ecx, &[w(0x3d), 8, 0]).eax;
    g.write::<u32>(w(0x47), h);
    // add esp, 4: two of slot 719's words stay
    let base = fr.esp.wrapping_sub(8);
    let buf = Frame {
        at: img.rel32_target(f + 0x4c),
        esp: base.wrapping_sub(4),
        ..*fr
    };
    crate::s4_small::parts_buffer(g, img, &buf);
    let fade = |g: &mut G, (x2, x1): (u32, u32), rec: u32, word: u32, call: u32, depth: u32| {
        let esp = base.wrapping_sub(4 * depth);
        g.write::<u32>(esp, word);
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp: esp.wrapping_sub(4),
            ecx: w(rec),
            xmm: [0, k(g, x1), k(g, x2), 0],
            ..*fr
        };
        crate::o4_fam5::fade_record(g, img, &callee);
    };
    fade(g, (0x54, 0x5c), 0x63, 0, 0x68, 1);
    stores(g, img, f, &[0x86, 0x90, 0x9a]);
    fade(g, (0x70, 0x78), 0x82, w(0x7d), 0xa5, 2);
    stores(g, img, f, &[0xae, 0xb8, 0xc2]);
    let t = slot_target(g, w(0xaa), 0x43c / 4);
    // call width: 5 (the fades' words and slot 719's two under it)
    g.call(t, fr.ecx, &[4, w(0x7d), 0, 8, 0]);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xdf), w(0xda));
    let head = g.read::<u32>(w(0xe5));
    g.write::<u32>(node.wrapping_add(0x14), w(0xed));
    g.write::<u8>(node.wrapping_add(0x18), 2);
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x11b), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x123), node);
    } else {
        let tail = g.read::<u32>(w(0x100));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x10d), node);
    }
    Ret::eax(0)
}

/// [`start_0325`] (`magic_0325` 0x100013e0; the `rel32`s: the lookup,
/// the part's buffer, the two fades and the pool's take).
pub const START_0325: Sig = Sig {
    len: 298,
    hash: 0x2e87_251c_dcd9_9e88,
    calls: &[
        (0x29, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x4c, Callee::Sig(&crate::s4_small::PARTS_BUFFER_80000)),
        (0x68, Callee::Sig(&crate::o4_fam5::FADE_RECORD_0325_2250)),
        (0xa5, Callee::Sig(&crate::o4_fam5::FADE_RECORD_0325_2250)),
        (0xdf, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The effect's start-up (`magic_0253` 0x100019c0, 498 bytes, run by its
/// load step; no arguments; a /GS frame, `V` at `ebp - 0x14`): two words
/// set (`+0x24`, `+0x2e`) and slot 719 `(PART, 8, 0)` (`+0x20`) into
/// `+0x3e`; the part set up ([`part_init`](crate::l5_objhost::part_init));
/// the global node at `V` = `(-194, 0, 60, 1)` with the scale `*K`
/// (`+0x4b`) and the immediates 1.3 and 0.99
/// ([`global_node_start`](crate::o6_fam_pairs5::global_node_start)); `C` =
/// `*G_C` (`+0x84`) into `+0x93`, two words set, slot 413 `(C)` (`*G_B`
/// `+0x8d` set) or 215 into `+0xd3`; three words set and the halfword
/// `+0xf7` 1 or 0 by `*G_B` again (`+0xc8`); slot 525 `(*G_D)` (`+0x10b`);
/// the camera records ([`camera_records_start`](crate::s5_misc::camera_records_start));
/// the record `*G_I` (`+0x120`, 0x148 bytes from `+0x131`) word 0 cleared,
/// ten words, a halfword and a byte set (`+0x139` .. `+0x1a6`); a pool
/// node ([`take`](crate::sa_pool::take), owner `+0x12b`) appended to the
/// task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_0253<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; sub esp, 0x14
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let s0 = ebp.wrapping_sub(0x14);
    stores(g, img, f, &[0x24, 0x2e]);
    let t = slot_target(g, w(0x11), 0xb3c / 4);
    // call width: 3
    let h = g.call(t, fr.ecx, &[w(0x20), 8, 0]).eax;
    g.write::<u32>(w(0x3e), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x43));
    // add esp, 4; mov [esp], s2; push ecx; mov [esp], s1; push &V
    for (i, x) in (0u32..).zip([0xc342_0000u32, 0, 0x4270_0000, 0x3f80_0000]) {
        g.write::<u32>(v.wrapping_add(4 * i), x);
    }
    let esp = s0.wrapping_sub(0x10);
    for (i, x) in (0u32..).zip([v, w(0x5d), w(0x52), 0]) {
        g.write::<u32>(esp.wrapping_add(4 * i), x);
    }
    let node = Frame {
        at: img.rel32_target(f + 0x7f),
        esp: esp.wrapping_sub(4),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x4b)))],
        ..*fr
    };
    crate::o6_fam_pairs5::global_node_start(g, img, &node);
    let c = g.read::<u32>(w(0x84));
    let battle = g.read::<u32>(w(0x8d)) != 0;
    g.write::<u32>(w(0x93), c);
    stores(g, img, f, &[0x9d, 0xa7]);
    let n = if battle { 0x674 / 4 } else { 0x35c / 4 };
    let t = slot_target(g, w(0x99), n);
    // call width: 1
    let x = g.call(t, fr.ecx, &[c]).eax;
    let battle = g.read::<u32>(w(0xc8)) != 0;
    g.write::<u32>(w(0xd3), x);
    stores(g, img, f, &[0xd7, 0xe1]);
    g.write::<u32>(w(0xed), 1);
    g.write::<u16>(w(0xf7), u16::from(battle));
    let d = g.read::<u32>(w(0x10b));
    let t = slot_target(g, w(0x105), 0x834 / 4);
    // call width: 1
    g.call(t, fr.ecx, &[d]);
    let cam = Frame {
        at: img.rel32_target(f + 0x11b),
        esp: s0.wrapping_sub(4),
        ..*fr
    };
    crate::s5_misc::camera_records_start(g, img, &cam);
    let i = g.read::<u32>(w(0x120));
    g.write::<u32>(i.wrapping_mul(0x148).wrapping_add(w(0x131)), 0);
    stores(
        g,
        img,
        f,
        &[
            0x139, 0x143, 0x14d, 0x157, 0x161, 0x16b, 0x175, 0x17f, 0x189, 0x193,
        ],
    );
    g.write::<u16>(w(0x1a0), 0);
    g.write::<u8>(w(0x1a8), 1);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x1ae), w(0x12b));
    let head = g.read::<u32>(w(0x1b4));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x1d6), node);
    } else {
        let tail = g.read::<u32>(w(0x1c4));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x1e1), node);
    Ret::eax(0)
}

/// [`start_0253`] (`magic_0253` 0x100019c0; the `rel32`s: the part's
/// set-up, the global node, the camera records, the pool's take and the
/// cookie check).
pub const START_0253: Sig = Sig {
    len: 498,
    hash: 0xf793_9821_744c_3820,
    calls: &[
        (0x43, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (
            0x7f,
            Callee::Sig(&crate::o6_fam_pairs5::GLOBAL_NODE_START_0253_4370),
        ),
        (0x11b, Callee::Sig(&crate::s5_misc::CAMERA_RECORDS_START)),
        (0x1ae, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x1ea, Callee::Any),
    ],
    imports: &[],
};

/// The effect's start-up (`magic_0288` 0x10001340, 504 bytes, run by its
/// load step; no arguments): two words set (`+0x14`, `+0x1e`) and slot
/// 719 `(PART, 8, 0)` (`+0x10`) into `+0x2e`; the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); a halfword and ten
/// words cleared or set (`+0x37` .. `+0x99`); three character records:
/// the actor byte `*G_SEL` (`+0x45`) into `+0xb1`, then `*G_C` (`+0x143`)
/// into `+0x148`, each with the character slot 413 (in a battle: `*G_B`
/// `+0xab`/`+0x101`) or 215 into `+0x109`/`+0x185`, words -1, 0 and 1 and
/// a halfword 1 in a battle else 0 (`+0x113` .. `+0x13e`, `+0x189` ..
/// `+0x1b2`; the second record's battle flag `+0x17f`); five more words
/// for the first (`+0xbb` .. `+0xe3`) and two for the second (`+0x152`,
/// `+0x15c`); a free-list node ([`pop_node`](crate::m0032::pop_node))
/// appended to the task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's records, in order
pub fn start_0288<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = w(0x01);
    stores(g, img, f, &[0x14, 0x1e]);
    // call width: 3
    let h = g
        .call(slot_target(g, t, 0xb3c / 4), fr.ecx, &[w(0x10), 8, 0])
        .eax;
    g.write::<u32>(w(0x2e), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x33));
    g.write::<u16>(w(0x3b), 0);
    g.write::<u32>(w(0x40), 0);
    let sel = g.read::<u32>(w(0x45));
    stores(
        g,
        img,
        f,
        &[0x49, 0x53, 0x5d, 0x67, 0x71, 0x7b, 0x85, 0x8f, 0x99],
    );
    let b = u32::from(g.read::<u8>(sel));
    let battle = g.read::<u32>(w(0xab)) != 0;
    g.write::<u32>(w(0xb1), b);
    stores(g, img, f, &[0xbb, 0xc5, 0xcf, 0xd9, 0xe3]);
    let chr = |g: &mut G, battle: bool, x: u32| {
        let n = if battle { 0x674 / 4 } else { 0x35c / 4 };
        // call width: 1
        g.call(slot_target(g, t, n), fr.ecx, &[x]).eax
    };
    let c = chr(g, battle, b);
    let battle = g.read::<u32>(w(0x101)) != 0;
    g.write::<u32>(w(0x109), c);
    stores(g, img, f, &[0x113, 0x11d]);
    g.write::<u32>(w(0x129), 1);
    g.write::<u16>(w(0x135), u16::from(battle));
    let x = g.read::<u32>(w(0x143));
    g.write::<u32>(w(0x148), x);
    stores(g, img, f, &[0x152, 0x15c]);
    let c = chr(g, battle, x);
    let battle = g.read::<u32>(w(0x17f)) != 0;
    g.write::<u32>(w(0x185), c);
    stores(g, img, f, &[0x189, 0x193]);
    g.write::<u32>(w(0x19f), 1);
    g.write::<u16>(w(0x1a9), u16::from(battle));
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x1b7));
    let head = g.read::<u32>(w(0x1bd));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x1e9), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x1f1), node);
    } else {
        let tail = g.read::<u32>(w(0x1ce));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x1db), node);
    }
    Ret::eax(0)
}

/// [`start_0288`] (`magic_0288` 0x10001340; the `rel32`s: the part's
/// set-up and the free list's pop).
pub const START_0288: Sig = Sig {
    len: 504,
    hash: 0x67e0_ca37_2608_fdce,
    calls: &[
        (0x33, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x1b7, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// The effect's start-up (`magic_0244` 0x10001340, 533 bytes, run by its
/// load step; no arguments): two parts, each set up as the effect's are
/// (words set, slot 719 `(P, 8, k)` into `G_H`, slot 711 `(h, Q, 0)` with
/// `**G_LOCK` 1 around it, slot 710 `(R, h, *G_BUF, 0x80000)`; the first
/// with `k` 0 and its operands from `+0x09`, the second with `k` 1 from
/// `+0xdf`), each followed by its object's start
/// ([`object_start`](crate::m0244::object_start), `xmm3` the floats
/// `+0x97`/`+0x163`, the animations 0 and 0x8c, after four words set
/// each); four more words; a free-list node
/// ([`pop_node`](crate::m0032::pop_node)) appended to the task list.
/// Returns 0.
#[allow(clippy::too_many_lines)] // the two parts, step by step
pub fn start_0244<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = w(0x01);
    let slot = |g: &G, n: u32| slot_target(g, t, n);
    // push esi; push edi
    let base = fr.esp.wrapping_sub(8);
    // a part: its operands (the part, the lock, the buffer, `G_H`, the
    // program, the lock again, `G_H` read, the environment)
    let part = |g: &mut G, k: u32, stores_at: &[u32], ops: [u32; 8]| {
        let [p, lock, buf, h_at, q, lock_back, h_word, r] = ops;
        stores(g, img, f, stores_at);
        // call width: 3
        let h = g.call(slot(g, 0xb3c / 4), 0, &[w(p), 8, k]).eax;
        let lock = g.read::<u32>(w(lock));
        let buf = g.read::<u32>(w(buf));
        g.write::<u32>(w(h_at), h);
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        // call width: 3
        g.call(slot(g, 0xb1c / 4), 0, &[h, w(q), 0]);
        let lock = g.read::<u32>(w(lock_back));
        g.write::<u32>(lock, saved);
        let h = g.read::<u32>(w(h_word));
        // call width: 4
        g.call(slot(g, 0xb18 / 4), 0, &[w(r), h, buf, 0x0008_0000]);
    };
    // the object's start at the `rel32` `call`: `ecx` the context, `edx`
    // the object, `xmm3` the float, the stack (junk, animation)
    let start = |g: &mut G, call: u32, ctx: u32, obj: u32, k: u32, anim: u32| {
        let esp = base.wrapping_sub(0x14);
        g.write::<u32>(esp.wrapping_add(4), 0);
        g.write::<u32>(esp.wrapping_add(8), anim);
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: w(ctx),
            edx: w(obj),
            xmm: [0, 0, 0, u128::from(g.read::<u32>(w(k)))],
            ..*fr
        };
        crate::m0244::object_start(g, img, &callee);
    };
    part(
        g,
        0,
        &[0x09, 0x13, 0x1d, 0x27],
        [0x3a, 0x42, 0x48, 0x4f, 0x56, 0x6f, 0x82, 0x8d],
    );
    stores(g, img, f, &[0xab, 0xb5, 0xbf, 0xc9]);
    start(g, 0xd4, 0xa7, 0x9f, 0x97, 0);
    part(
        g,
        1,
        &[0xdf, 0xe9, 0xf3],
        [0x106, 0x10e, 0x114, 0x11b, 0x122, 0x13b, 0x14e, 0x159],
    );
    stores(g, img, f, &[0x16a, 0x179, 0x183, 0x18d]);
    start(g, 0x1a3, 0x19e, 0x199, 0x163, w(0x175));
    stores(g, img, f, &[0x1aa, 0x1b4, 0x1be, 0x1c8]);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x1d3));
    let head = g.read::<u32>(w(0x1d9));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x206), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x20e), node);
    } else {
        let tail = g.read::<u32>(w(0x1eb));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x1f8), node);
    }
    Ret::eax(0)
}

/// [`start_0244`] (`magic_0244` 0x10001340; the `rel32`s: the objects'
/// starts and the free list's pop).
pub const START_0244: Sig = Sig {
    len: 533,
    hash: 0x677c_88af_8939_070d,
    calls: &[
        (0xd4, Callee::Sig(&crate::m0244::OBJECT_START)),
        (0x1a3, Callee::Sig(&crate::m0244::OBJECT_START)),
        (0x1d3, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// The sprites' colours, step and draw (`magic_0371` 0x100014a0, 118
/// bytes, a table entry, no arguments): each live record (first word set)
/// of the pool (count `+0x01`, records `+0x07`, the count read again
/// after each live one) gets `+0x68` = the float `4 * rec[0x28]` bytes
/// below `T` (`+0x2c`) times `K` (`+0x16`); the pool's sprites stepped
/// ([`sprites_step`](crate::s6_misc::sprites_step), `+0x50`) and drawn
/// ([`block_sprite_frames`](crate::s9_misc::block_sprite_frames), `+0x5e`,
/// `dt` 0) into `+0x6b`. Returns 1.
pub fn sprites_frame_0371<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let mut n = g.read::<u32>(w(0x01));
    let mut rec = g.read::<u32>(w(0x07));
    let k = g.read::<f32>(w(0x16));
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        if g.read::<u32>(rec) != 0 {
            let back = g.read::<u32>(rec.wrapping_add(0x28)) << 2;
            let x = g.read::<f32>(w(0x2c).wrapping_sub(back));
            g.write::<f32>(rec.wrapping_add(0x68), crate::sb_sse::mul(x, k));
            n = g.read::<u32>(w(0x40));
        }
        i += 1;
        rec = rec.wrapping_add(0x94);
    }
    // push esi; call
    let esp = fr.esp.wrapping_sub(8);
    let step = Frame {
        at: img.rel32_target(f + 0x55),
        esp,
        ecx: w(0x50),
        ..*fr
    };
    crate::s6_misc::sprites_step(g, img, &step);
    // push ecx (what the step left in it: the draw reads no stack word)
    g.write::<u32>(esp, w(0x50));
    let draw = Frame {
        at: img.rel32_target(f + 0x63),
        esp: esp.wrapping_sub(4),
        ecx: w(0x5e),
        xmm: [0; 4],
        ..*fr
    };
    let x = crate::s9_misc::block_sprite_frames(g, img, &draw).eax;
    g.write::<u32>(w(0x6b), x);
    Ret::eax(1)
}

/// [`sprites_frame_0371`] (`magic_0371` 0x100014a0; the `rel32`s: the
/// sprites' step and draw).
pub const SPRITES_FRAME_0371: Sig = Sig {
    len: 118,
    hash: 0x6373_9d0a_da32_1b82,
    calls: &[
        (0x55, Callee::Sig(&crate::s6_misc::SPRITES_STEP)),
        (0x63, Callee::Sig(&crate::s9_misc::BLOCK_SPRITE_FRAMES_B)),
    ],
    imports: &[],
};

/// The DLL's object `B = *G_B + D` set up and its task listed (`magic_0673`
/// 0x10001eb0, 88 bytes, a table entry, no arguments; `G_B` `+0x02`, the
/// displacement `D` `+0x08`): `B+0xf0` a colour, u16 `+0x124` 0, `+0x114`
/// 0x100, `+0x118` 0xd0, `+0x110` and `+0xf8` 0, `+0xfc` and `+0x100`
/// 0.06; then a tail jump to its task ([`object_task`](crate::r13_misc::object_task),
/// `ecx` = `B`), whose node it returns.
pub fn object_listed_0673<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let b = g.read::<u32>(w(0x02)).wrapping_add(w(0x08));
    for (o, v) in [
        (0xf0u32, 0x3080_8080u32),
        (0x114, 0x100),
        (0x118, 0xd0),
        (0x110, 0),
        (0xf8, 0),
        (0xfc, 0x3d75_c28f),
        (0x100, 0x3d75_c28f),
    ] {
        if o == 0x114 {
            g.write::<u16>(b.wrapping_add(0x124), 0);
        }
        g.write::<u32>(b.wrapping_add(o), v);
    }
    let task = Frame {
        at: img.rel32_target(f + 0x54),
        ecx: b,
        ..*fr
    };
    crate::r13_misc::object_task(g, img, &task)
}

/// [`object_listed_0673`] (`magic_0673` 0x10001eb0; the `rel32`: the
/// object's task).
pub const OBJECT_LISTED_0673: Sig = Sig {
    len: 88,
    hash: 0xaf82_6131_18e0_b5b8,
    calls: &[(0x54, Callee::Sig(&crate::r13_misc::OBJECT_TASK_0673))],
    imports: &[],
};

/// The DLL's rows 3 to 9 started (`magic_0673` 0x10001950, 425 bytes, a
/// table entry, no arguments; `ebp` = `esp - 4`, a float at `ebp - 4`):
/// each object `O` of row `k` ([`start_row`](crate::m0673::start_row),
/// `ecx` = `k`) gets `+0x104` = the callback (`+0x1a` ..), `+0x120` 0x90;
/// rows 7, 8 and 9 also the scale `+0x60..+0x6c` = `(s + K1) K2` (`s` slot
/// 404 `(0.5)`'s ST0 as `f32`; `K1` `+0xab`, `K2` `+0xcc`), each set after
/// the next row's start for 3..6 and after the slot for 7..9. Returns slot
/// 404's last `eax`.
pub fn rows_0673<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; push ecx; push esi: each start's frame
    let esp = fr.esp.wrapping_sub(0xc);
    let row = |g: &mut G, call: u32, i: u32, esp: u32| {
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp: esp.wrapping_sub(4),
            ecx: i,
            ..*fr
        };
        crate::m0673::start_row(g, img, &callee)
    };
    let mark = |g: &mut G, o: u32, cb: u32| {
        g.write::<u32>(o.wrapping_add(0x104), w(cb));
        g.write::<u32>(o.wrapping_add(0x120), 0x90);
    };
    let mut o = row(g, 0x0b, 3, esp);
    for (call, i, cb) in [
        (0x29u32, 4u32, 0x1au32),
        (0x47, 5, 0x38),
        (0x65, 6, 0x56),
        (0x83, 7, 0x74),
    ] {
        mark(g, o, cb);
        o = row(g, call, i, esp);
    }
    // push ecx: slot 404's word (0.5) for the three scales
    let esp = esp.wrapping_sub(4);
    g.write::<u32>(esp, 0x3f00_0000);
    let mut eax = 0;
    for (next, cb, k1, k2) in [
        (Some((0xe5u32, 8u32)), 0xbau32, 0xabu32, 0xccu32),
        (Some((0x146, 9)), 0x11b, 0x10c, 0x12d),
        (None, 0x17a, 0x16d, 0x18c),
    ] {
        let t = slot_target(g, w(0x8a), 0x650 / 4);
        // call width: 1
        let out = g.call(t, fr.ecx, &[0x3f00_0000]);
        eax = out.eax;
        let s = crate::ob_model::st0_f32(out.st0);
        let x = mul(add(s, k(g, k1)), k(g, k2));
        mark(g, o, cb);
        for off in [0x6cu32, 0x68, 0x64, 0x60] {
            g.write::<f32>(o.wrapping_add(off), x);
        }
        if let Some((call, i)) = next {
            o = row(g, call, i, esp);
        }
    }
    Ret::eax(eax)
}

/// [`rows_0673`] (`magic_0673` 0x10001950; the `rel32`s: the rows'
/// start seven times).
pub const ROWS_0673: Sig = Sig {
    len: 425,
    hash: 0x329f_936f_398a_1cf2,
    calls: &[
        (0x0b, Callee::Sig(&crate::m0673::START_ROW)),
        (0x29, Callee::Sig(&crate::m0673::START_ROW)),
        (0x47, Callee::Sig(&crate::m0673::START_ROW)),
        (0x65, Callee::Sig(&crate::m0673::START_ROW)),
        (0x83, Callee::Sig(&crate::m0673::START_ROW)),
        (0xe5, Callee::Sig(&crate::m0673::START_ROW)),
        (0x146, Callee::Sig(&crate::m0673::START_ROW)),
    ],
    imports: &[],
};

/// An object sent from the caster to its target (`magic_0415`
/// 0x10001390, 620 bytes, a callback, cdecl `(O)`; a /GS frame, `P` at
/// `ebp - 0x24`, `Q` at `ebp - 0x34`, `V` at `ebp - 0x14`): with `C =
/// *G_C` (`+0x12`) and `i = O[0xac]`, the target's character `T =
/// C[0x1a0 + 4 O[0x2c + 16 i]]`; `P` = slot 456 `(C[0x198], 11, &P)`; `Q`
/// = slot 457 `(T, 0 or 6, &Q)` (0 when slot 520 says 0x2380000);
/// `V = Q - P`; the steps `n = trunc(d / K2)` with `d` slot 453 `(&V)`
/// (ST0 as `f32`, times `K1` in the first case; `K1` `+0xcb`, `K2`
/// `+0x12f`); `O+0xd0` = `P` (`w` 0), its step `O+0xe0` = `V K3 / n` (`K3`
/// `+0x137`, `w` 0), the scale `+0x130..` 0.17 (`w` 0), `O[16 (i + 3)]` =
/// `n`; slots 444 `(0)`, 522 `(0, 0)`, 440 `(*G_C, O, 0, 1)` and 450 `(O,
/// F)` (`+0x24c`). Returns slot 450's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn send_0415<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = w(0x33);
    let slot = |g: &G, n: u32| slot_target(g, t, n);
    let ebp = fr.esp.wrapping_sub(4);
    let (p, q, v) = (
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x34),
        ebp.wrapping_sub(0x14),
    );
    let c = g.read::<u32>(w(0x12));
    let o = fr.arg(g, 0);
    let i = g.read::<u32>(o.wrapping_add(0xac));
    let pick = g.read::<u32>(o.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c));
    let target = g.read::<u32>(c.wrapping_add(pick.wrapping_mul(4)).wrapping_add(0x1a0));
    let a = g.read::<u32>(c.wrapping_add(0x198));
    // call width: 3
    g.call(slot(g, 0x720 / 4), fr.ecx, &[a, 0xb, p]);
    // call width: 0
    let battle = g.call(slot(g, 0x820 / 4), fr.ecx, &[]).eax == 0x0238_0000;
    // call width: 3
    g.call(
        slot(g, 0x724 / 4),
        fr.ecx,
        &[target, if battle { 0 } else { 6 }, q],
    );
    for j in 0..4u32 {
        let d = sub(
            g.read::<f32>(q.wrapping_add(4 * j)),
            g.read::<f32>(p.wrapping_add(4 * j)),
        );
        g.write::<f32>(v.wrapping_add(4 * j), d);
    }
    // call width: 1
    let out = g.call(slot(g, 0x714 / 4), fr.ecx, &[v]);
    let d = crate::ob_model::st0_f32(out.st0);
    let x = if battle { mul(d, k(g, 0xcb)) } else { d };
    let n = crate::sb_sse::cvtt(div(x, k(g, 0x12f)));
    let s = div(k(g, 0x137), cvt(n.cast_unsigned()));
    let step: [f32; 4] =
        core::array::from_fn(|j| mul(g.read::<f32>(v.wrapping_add(4 * j as u32)), s));
    g.write::<f32>(v.wrapping_add(0xc), step[3]);
    for j in 0..3u32 {
        let x = g.read::<u32>(p.wrapping_add(4 * j));
        g.write::<u32>(o.wrapping_add(0xd0 + 4 * j), x);
    }
    g.write::<u32>(o.wrapping_add(0xdc), 0);
    for j in 0..3u32 {
        g.write::<f32>(o.wrapping_add(0xe0 + 4 * j), step[j as usize]);
    }
    g.write::<u32>(o.wrapping_add(0xec), 0);
    for off in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(o.wrapping_add(off), 0x3e2e_147b);
    }
    g.write::<u32>(o.wrapping_add(0x13c), 0);
    let i = g.read::<u32>(o.wrapping_add(0xac)).wrapping_add(3);
    for j in 0..3u32 {
        g.write::<f32>(v.wrapping_add(4 * j), step[j as usize]);
    }
    g.write::<u32>(o.wrapping_add(i.wrapping_mul(16)), n.cast_unsigned());
    // call width: 1
    g.call(slot(g, 0x6f0 / 4), fr.ecx, &[0]);
    // call width: 2
    g.call(slot(g, 0x828 / 4), fr.ecx, &[0, 0]);
    let c = g.read::<u32>(w(0x240));
    // call width: 4
    g.call(slot(g, 0x6e0 / 4), fr.ecx, &[c, o, 0, 1]);
    // call width: 2
    Ret::eax(g.call(slot(g, 0x708 / 4), fr.ecx, &[o, w(0x24c)]).eax)
}

/// [`send_0415`] (`magic_0415` 0x10001390; the `rel32`: the cookie
/// check).
pub const SEND_0415: Sig = Sig {
    len: 620,
    hash: 0xe6f0_afb9_89a2_2b2c,
    calls: &[(0x264, Callee::Any)],
    imports: &[],
};

/// The effect's targets measured (`magic_0421` 0x10001000, 597 bytes, a
/// callback, cdecl `(O)`; a /GS frame, `h` at `ebp - 0x30`, the largest
/// `m` at `ebp - 0x2c`, `P` at `ebp - 0x24`, `Q` at `ebp - 0x14`): slot 519
/// `(11, 0, 0 or 6, 1)` (0 when slot 520 says 0x2380000); `G_N` (`+0x52`)
/// = `C[0x19c]` (`C = *G_C`, `+0x44`), `G_M` (`+0x6c`) 0; `h` = slot 515's
/// ST0; slot 474 `(C[0x198], 11, &Q, &P)` (`Q` zeroed first). For each
/// target `i` below `*G_N`: `x` = slot 518 `(i)` plus `K` (`+0xee`), and
/// when above `G_M` it becomes `G_M`, `G_X` (`+0x114`) = `trunc(x / K2)`
/// (`+0x10a`) and `G_Y` (`+0x127`) = slot 516 `(i)`; `y` = slot 514 `(slot
/// 517 (i), h)` (all ST0 as `f32`) raises `m`; slot 513 `(C[0x1a0 + 4i], 0,
/// R + 16 i)` (`R` `+0xd0`). Then with `s` = `m`, or `K3` (`+0x1ae`) when
/// that is larger (always without targets, `+0x1bf`): `+0x1d2` = `h`,
/// `+0x1eb` = `h - s`, `+0x1f3` = `h + s`, three words set, the block
/// `+0x1e3` cleared (0x40 bytes, `memset`); slots 450 `(O, F1)` and 449
/// `(O, F2)` (`+0x229`, `+0x236`). Returns slot 449's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn targets_0421<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, div, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = w(0x17);
    let slot = |g: &G, n: u32| slot_target(g, t, n);
    let ebp = fr.esp.wrapping_sub(4);
    let (p, q) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let o = fr.arg(g, 0);
    // call width: 0
    let battle = g.call(slot(g, 0x820 / 4), fr.ecx, &[]).eax == 0x0238_0000;
    // call width: 4
    g.call(
        slot(g, 0x81c / 4),
        fr.ecx,
        &[0xb, 0, if battle { 0 } else { 6 }, 1],
    );
    let c = g.read::<u32>(w(0x44));
    let n = g.read::<u32>(c.wrapping_add(0x19c));
    g.write::<u32>(w(0x52), n);
    stores(g, img, f, &[0x6a]);
    // call width: 0
    let h = st0_f32(g.call(slot(g, 0x80c / 4), fr.ecx, &[]).st0);
    let c = g.read::<u32>(w(0x82));
    for j in 0..4u32 {
        g.write::<u32>(q.wrapping_add(4 * j), 0);
    }
    let a = g.read::<u32>(c.wrapping_add(0x198));
    // call width: 4
    g.call(slot(g, 0x768 / 4), fr.ecx, &[a, 0xb, q, p]);
    let mut m = 0.0f32;
    let mut s = k(g, 0x1bf);
    if g.read::<u32>(w(0xbe)).cast_signed() > 0 {
        let mut i = 0u32;
        loop {
            // call width: 1
            let x = add(
                st0_f32(g.call(slot(g, 0x818 / 4), fr.ecx, &[i]).st0),
                k(g, 0xee),
            );
            if x > g.read::<f32>(w(0xf8)) {
                g.write::<f32>(w(0x102), x);
                g.write::<u32>(
                    w(0x114),
                    crate::sb_sse::cvtt(div(x, k(g, 0x10a))).cast_unsigned(),
                );
                // call width: 1
                let y = st0_f32(g.call(slot(g, 0x810 / 4), fr.ecx, &[i]).st0);
                g.write::<f32>(w(0x127), y);
            }
            // call width: 1
            let a = st0_f32(g.call(slot(g, 0x814 / 4), fr.ecx, &[i]).st0);
            // call width: 2
            let y = st0_f32(
                g.call(slot(g, 0x808 / 4), fr.ecx, &[a.to_bits(), h.to_bits()])
                    .st0,
            );
            if y > m {
                m = y;
            }
            let c = g.read::<u32>(w(0x178));
            let id = g.read::<u32>(c.wrapping_add(0x1a0 + 4 * i));
            // call width: 3
            g.call(
                slot(g, 0x804 / 4),
                fr.ecx,
                &[id, 0, w(0xd0).wrapping_add(16 * i)],
            );
            i += 1;
            if i.cast_signed() >= g.read::<u32>(w(0x19b)).cast_signed() {
                break;
            }
        }
        // `comiss K3, m; jbe`: `K3` only when it is larger (a NaN keeps `m`)
        let k3 = k(g, 0x1ae);
        s = if k3 > m { k3 } else { m };
    }
    g.write::<f32>(w(0x1d2), h);
    g.write::<f32>(w(0x1eb), sub(h, s));
    g.write::<f32>(w(0x1f3), add(h, s));
    stores(g, img, f, &[0x1f7, 0x201, 0x20b]);
    g.write_bytes(w(0x1e3), &[0u8; 0x40]);
    // call width: 2
    g.call(slot(g, 0x708 / 4), fr.ecx, &[o, w(0x229)]);
    // call width: 2
    Ret::eax(g.call(slot(g, 0x704 / 4), fr.ecx, &[o, w(0x236)]).eax)
}

/// [`targets_0421`] (`magic_0421` 0x10001000; the `rel32`s: `memset` and
/// the cookie check).
pub const TARGETS_0421: Sig = Sig {
    len: 597,
    hash: 0x339e_fa90_9b64_05cd,
    calls: &[(0x216, Callee::Import("memset")), (0x24d, Callee::Any)],
    imports: &[],
};

/// The effect's start-up (`magic_0352` 0x10005700, 345 bytes, run by its
/// load step; no arguments): four words set (`+0x1c` .. `+0x3a`) and slot
/// 719 `(PART, 8, 0)` (`+0x18`) into `+0x4a`; the particle set-up
/// ([`part_setup`](crate::sb_misc::part_setup), its buffer `+0x4f`); the
/// camera ([`camera_start`](crate::m0352::camera_start)); slot 620 `(R[k],
/// 0x64a21a00, 6)` for the 51 records' words `R[k]` = `*G_R + 0x7c + 0x20 k`
/// (`+0x71`); two words, a halfword and a byte set (`+0xa0` .. `+0xbd`);
/// slot 271 `(4)`; two pool nodes ([`take`](crate::sa_pool::take), owners
/// `+0xd2`, `+0x10e`, kinds 1 and 2) linked after the list's first or
/// last, the second made the last. Returns 0.
pub fn start_0352<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; push esi; push edi
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
    stores(g, img, f, &[0x1c, 0x26, 0x30, 0x3a]);
    let t = slot_target(g, w(0x07), 0xb3c / 4);
    // call width: 3
    let h = g.call(t, fr.ecx, &[w(0x18), 8, 0]).eax;
    g.write::<u32>(w(0x4a), h);
    // add esp, 8: slot 719's last word stays; push BUF; call
    let esp = base.wrapping_sub(0xc);
    g.write::<u32>(esp.wrapping_add(4), w(0x4f));
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x54), esp);
    let cam = Frame {
        at: img.rel32_target(f + 0x5c),
        esp: base.wrapping_sub(4),
        ..*fr
    };
    crate::m0352::camera_start(g, img, &cam);
    let mut k = 0x60u32;
    while k <= 0x6a0 {
        let r = g.read::<u32>(w(0x71));
        let x = g.read::<u32>(r.wrapping_add(k).wrapping_add(0x1c));
        let t = slot_target(g, w(0x81), 0x9b0 / 4);
        // call width: 3
        g.call(t, fr.ecx, &[x, 0x64a2_1a00, 6]);
        k += 0x20;
    }
    stores(g, img, f, &[0xa0, 0xaa]);
    g.write::<u16>(w(0xb7), 0);
    g.write::<u8>(w(0xbf), 1);
    let t = slot_target(g, w(0x9c), 0x43c / 4);
    // call width: 1
    g.call(t, fr.ecx, &[4]);
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0xd7), w(0xd2));
    let mut head = g.read::<u32>(w(0xdd));
    g.write::<u8>(first.wrapping_add(0x18), 1);
    g.write::<u32>(first.wrapping_add(4), 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x109), first);
    } else {
        let tail = g.read::<u32>(w(0xf4));
        g.write::<u32>(first, tail);
        g.write::<u32>(tail.wrapping_add(4), first);
    }
    g.write::<u32>(first.wrapping_add(8), first);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0x116), w(0x10e));
    g.write::<u8>(second.wrapping_add(0x18), 2);
    g.write::<u32>(second.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x146), second);
        g.write::<u32>(second.wrapping_add(8), second);
        g.write::<u32>(w(0x14e), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first.wrapping_add(4), second);
        g.write::<u32>(second.wrapping_add(8), second);
        g.write::<u32>(w(0x132), second);
    }
    Ret::eax(0)
}

/// [`start_0352`] (`magic_0352` 0x10005700; the `rel32`s: the particle
/// set-up, the camera and the pool's take twice).
pub const START_0352: Sig = Sig {
    len: 345,
    hash: 0x6602_6290_4144_1bdc,
    calls: &[
        (0x54, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x5c, Callee::Sig(&crate::m0352::CAMERA_START)),
        (0xd7, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x116, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The operand offsets of [`actors_start`]'s copies from `+0x142` on.
struct ActorsStart {
    /// `O+0xd4`'s constant, `O+0xe8`'s immediate.
    lower: u32,
    e8: u32,
    /// The kind for the actor -1 (a global) and the selection list.
    kind_none: u32,
    kind_sel: u32,
    /// `magic_0572`: slot 293 `(*G_K)` (its table) and, when it answers
    /// 0, `O+0x104` = the immediate.
    check: Option<(u32, u32)>,
    /// The record's `+0x18` and base, its two callbacks, the count.
    rec18: u32,
    rec: u32,
    cb1: u32,
    cb2: u32,
    count: u32,
    /// The pool: count, free head, free stores (next, 0), callback,
    /// count store; the task list: head, tail, tail store, head store,
    /// tail store (empty).
    pool: (u32, u32, u32, u32, u32, u32),
    list: (u32, u32, u32, u32, u32),
}

/// `magic_0571` 0x10001340.
const ACTORS_START_0571: ActorsStart = ActorsStart {
    lower: 0x142,
    e8: 0x14c,
    kind_none: 0x15e,
    kind_sel: 0x165,
    check: None,
    rec18: 0x17a,
    rec: 0x184,
    cb1: 0x1a2,
    cb2: 0x1a9,
    count: 0x1d1,
    pool: (0x1df, 0x1ee, 0x20b, 0x213, 0x244, 0x24f),
    list: (0x255, 0x266, 0x274, 0x286, 0x28f),
};

/// `magic_0572` 0x10001340.
const ACTORS_START_0572: ActorsStart = ActorsStart {
    lower: 0x148,
    e8: 0x152,
    kind_none: 0x130,
    kind_sel: 0x168,
    check: Some((0x177, 0x191)),
    rec18: 0x19c,
    rec: 0x1a6,
    cb1: 0x1c4,
    cb2: 0x1cb,
    count: 0x1f3,
    pool: (0x201, 0x210, 0x22d, 0x235, 0x266, 0x271),
    list: (0x277, 0x288, 0x296, 0x2a8, 0x2b1),
};

/// The effect's actors' objects started (run by the load step; no
/// arguments; a frame with two locals): `*G_1 = I1`, `*G_2 = I2` (`+0x1e`,
/// `+0x28`), slot 719 `(PART, 8, 0)` (`+0x18`) into `+0x36`; the part set
/// up ([`part_init`](crate::l5_objhost::part_init)). For each of the `*G_N`
/// (`+0x4b`, then the layout's count) actors `i`: the object `O = T +
/// 0x130 i` (`T` the immediate `+0x75`) started
/// ([`start_object_first`](crate::s4_misc::start_object_first), scale
/// `+0x64`, owner `0xf i`); the actor `a` (`*G_SELF` for -1, else
/// `TAB[SEL[i]]`); slot 262 `(a, O+0xd0, +0xd4, +0xd8)`, `O+0xdc` 1.0;
/// slot 333 `(a)` gives the yaw `K - v` (`+0xfa`) wrapped once (above
/// `+0x109` less `+0x113`, below `+0x11d` plus `+0x12a`) into `O+0xc4`;
/// `O+0xd4` less a constant, `O+0xe8` an immediate (-25.0; -6.0 in
/// `magic_0572`), `O+0x120` = `SEL[i]`; in `magic_0572` slot 293 `(*G_K)`
/// answering 0 sets `O+0x104`; the record `R = Q + 0x34 i` cleared with
/// two callbacks, `O+0x110 = R`. Then a node of the pool's ring
/// ([`pool_pop`](crate::r13_misc::pool_pop); with the pool empty the
/// original writes through 0, as here) is appended to the task list.
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn actors_start<G: Game>(g: &mut G, img: &Image, fr: &Frame, l: &ActorsStart) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; sub esp, 8; push esi, edi
    g.write::<u32>(w(0x1e), w(0x22));
    g.write::<u32>(w(0x28), w(0x2c));
    // call width: 3
    let h = g
        .call(slot_target(g, w(0x7), 719), fr.ecx, &[w(0x18), 8, 0])
        .eax;
    g.write::<u32>(w(0x36), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x3b));
    let actor = |g: &G, i: u32, none: u32, sel: u32, tab: u32| {
        if i == u32::MAX {
            g.read::<u32>(w(none))
        } else {
            let n = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(i)));
            g.read::<u32>(n.wrapping_mul(4).wrapping_add(w(tab)))
        }
    };
    let mut owner = 0u32;
    let mut i = 0u32;
    if g.read::<i32>(w(0x4b)) > 0 {
        loop {
            let o = i.wrapping_mul(0x130).wrapping_add(w(0x75));
            let scale = k(g, 0x64);
            let start = img.rel32_target(f + 0x7e);
            crate::s4_misc::start_object_first(g, img, start, o, scale, owner);
            let b = o + 0xd0;
            let a = actor(g, i, 0x92, 0x99, 0xa4);
            // call width: 4
            g.call(slot_target(g, w(0xb1), 262), fr.ecx, &[a, b, b + 4, b + 8]);
            g.write::<u32>(b + 0xc, 0x3f80_0000);
            let a = actor(g, i, 0xcf, 0xd6, 0xe1);
            // call width: 1
            let t = g.call(slot_target(g, w(0xe7), 333), fr.ecx, &[a]);
            let t = crate::m0018::st0_f32(t.st0);
            let mut y = sub(k(g, 0xfa), t);
            if y > k(g, 0x109) {
                y = sub(y, k(g, 0x113));
            } else if k(g, 0x11d) > y {
                y = add(y, k(g, 0x12a));
            }
            g.write::<f32>(o + 0xc4, y);
            let none = g.read::<u32>(w(l.kind_none));
            let v = sub(g.read::<f32>(o + 0xd4), k(g, l.lower));
            g.write::<u32>(o + 0xe8, w(l.e8));
            g.write::<f32>(o + 0xd4, v);
            let kind = if i == u32::MAX {
                none
            } else {
                u32::from(g.read::<u8>(g.read::<u32>(w(l.kind_sel)).wrapping_add(i)))
            };
            g.write::<u32>(o + 0x120, kind);
            if let Some((tab, cb)) = l.check {
                // call width: 1
                if g.call(slot_target(g, w(tab), 293), fr.ecx, &[none]).eax == 0 {
                    g.write::<u32>(o + 0x104, w(cb));
                }
            }
            let r = i.wrapping_mul(0x34);
            g.write::<u32>(r.wrapping_add(w(l.rec18)), 0);
            let r = r.wrapping_add(w(l.rec));
            for (at, v) in [
                (0x14u32, 0u32),
                (0x10, 0),
                (0x1c, 0),
                (0x20, w(l.cb1)),
                (0x24, w(l.cb2)),
                (0x28, 0),
                (0x2c, 0),
            ] {
                g.write::<u32>(r.wrapping_add(at), v);
            }
            g.write::<u16>(r.wrapping_add(0x30), 0);
            g.write::<u32>(o + 0x110, r);
            owner = owner.wrapping_add(0xf);
            i = i.wrapping_add(1);
            if i.cast_signed() >= g.read::<i32>(w(l.count)) {
                break;
            }
        }
    }
    let (n, free, next, zero, cb, n_out) = l.pool;
    let node = if g.read::<u16>(w(n)) == 0 {
        0
    } else {
        let node = g.read::<u32>(w(free));
        crate::r13_misc::pool_pop(g, img, f, node, (n, next, zero, cb, n_out));
        node
    };
    let (head, tail, tail_w, head_w, tail_w2) = l.list;
    let listed = g.read::<u32>(w(head)) != 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let last = g.read::<u32>(w(tail));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(tail_w), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(head_w), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(tail_w2), node);
    }
    Ret::eax(0)
}

/// [`actors_start`] in `magic_0571` (0x10001340, 665 bytes).
pub fn start_0571<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    actors_start(g, img, fr, &ACTORS_START_0571)
}

/// [`actors_start`] in `magic_0572` (0x10001340, 699 bytes; slot 293's
/// check).
pub fn start_0572<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    actors_start(g, img, fr, &ACTORS_START_0572)
}

/// [`start_0571`] (the `rel32`s: the part's set-up and the object start).
pub const START_0571: Sig = Sig {
    len: 665,
    hash: 0xc18e_330e_6045_d50e,
    calls: &[
        (0x3b, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x7e, Callee::Sig(&crate::s4_misc::START_OBJECT_FIRST)),
    ],
    imports: &[],
};

/// [`start_0572`].
pub const START_0572: Sig = Sig {
    len: 699,
    hash: 0xf217_5473_c075_4760,
    ..START_0571
};

/// `magic_0571`'s load step (0x10001210, 119 bytes): the summon's load
/// wait, then [`start_0571`]
/// ([`load_then_start`](crate::s7_misc::load_then_start)).
pub fn load_0571<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s7_misc::load_then_start(g, img, fr, start_0571)
}

/// `magic_0572`'s load step (0x10001210), then [`start_0572`].
pub fn load_0572<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s7_misc::load_then_start(g, img, fr, start_0572)
}

/// [`load_0571`] (the `rel32`: [`start_0571`]).
pub const LOAD_0571: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&START_0571))],
    imports: &[],
};

/// [`load_0572`] (the `rel32`: [`start_0572`]).
pub const LOAD_0572: Sig = Sig {
    calls: &[(0x5c, Callee::Sig(&START_0572))],
    ..LOAD_0571
};
