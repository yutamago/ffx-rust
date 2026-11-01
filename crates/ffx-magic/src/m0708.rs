//! `magic_0708.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::sb_sse::{cvt, div, mul, sub};
use crate::{Frame, Game, Image, Sig};

/// Sets the fade at time `x` of a span ending at `end` (0x10001000,
/// internal: `ecx` = `x`, `edx` = `end`): nothing from `end + 90`
/// (returns `end + 90`). The level is `x * 88 / 90` up to 90, `(end - x +
/// 90) * 88 / 90` after `end`, else 88 (signed, truncating); slot 352
/// `(level << 24, 0x44, 0)`. Returns slot 352's `eax`.
pub fn fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let x = fr.ecx.cast_signed();
    let end = fr.edx.cast_signed();
    let limit = end.wrapping_add(0x5a);
    if x >= limit {
        return limit.cast_unsigned();
    }
    let level = if x <= 0x5a {
        x.wrapping_mul(0x58) / 90
    } else if x > end {
        end.wrapping_sub(x).wrapping_add(0x5a).wrapping_mul(0x58) / 90
    } else {
        0x58
    };
    let t = slot_target(g, w(0x56), 352);
    g.call(t, 0, &[level.cast_unsigned() << 24, 0x44, 0]).eax
}

/// [`fade`] (103 bytes, 1 overlay).
pub const FADE: Sig = Sig {
    len: 103,
    hash: 0xcbb2_d056_5264_7ed1,
    calls: &[],
    imports: &[],
};

/// Sets the rising fade at step `x` of `n` (0x10001070, internal: `ecx`
/// = `x`, `edx` = `n`): nothing from `n + 1` (returns `n + 1`). The level
/// byte is `x << 7` (its low byte) up to step 1, else the low byte of
/// `((n - x + 1) << 7) / n` (signed); slot 776 `(0x44)` gives a handle
/// `h`, slot 915 `(0x3fff0000 three times, level * K, h)` (`K` at operand
/// `+0x50`). Returns slot 915's `eax`.
pub fn rise<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let x = fr.ecx.cast_signed();
    let n = fr.edx.cast_signed();
    let limit = n.wrapping_add(1);
    if x >= limit {
        return limit.cast_unsigned();
    }
    let level = if x <= 1 {
        (fr.ecx << 7) & 0xff
    } else {
        (n.wrapping_sub(x).wrapping_add(1).wrapping_shl(7) / n).cast_unsigned() & 0xff
    };
    let t = slot_target(g, w(0x25), 776);
    let h = g.call(t, 0, &[0x44]).eax;
    let v = mul(cvt(level), g.read::<f32>(w(0x50)));
    let t = slot_target(g, w(0x3f), 915);
    let c = 0x3fff_0000;
    g.call(t, 0, &[c, c, c, v.to_bits(), h]).eax
}

/// [`rise`] (124 bytes, 1 overlay).
pub const RISE: Sig = Sig {
    len: 124,
    hash: 0xc779_46fa_0309_b623,
    calls: &[],
    imports: &[],
};

/// Binds a model's parts (0x10001560, internal: `ecx` = the record,
/// `edx` = the object, cdecl `(base)`): slot 264 `(0x40)` gives a handle
/// into `obj[0x20]`, slot 231 `(handle, G_A, G_B)` (`+0x34`, `+0x2b`);
/// for each of the record's `+0x4` entries (16 bytes each from `+0x10`)
/// slot 941 `(entry[0])` gives its data `e`, `obj[0x38]` = the running
/// base, which then grows by `e[0x12] * 16` (signed), and slot 361 `(e,
/// obj, rec[0])` (941's word left under it); slot 265 `(0x40)` last.
/// Returns slot 265's `eax`.
pub fn bind_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let (rec, obj) = (fr.ecx, fr.edx);
    let n = g.read::<u16>(rec + 4);
    let t = slot_target(g, w(0x16), 264);
    let h = g.call(t, 0, &[0x40]).eax;
    g.write::<u32>(obj + 0x20, h);
    let (a, b) = (g.read::<u32>(w(0x34)), g.read::<u32>(w(0x2b)));
    let t = slot_target(g, w(0x3a), 231);
    g.call(t, 0, &[h, a, b, 0x40]);
    let mut base = fr.arg(g, 0);
    let mut p = rec + 0x10;
    for _ in 0..n {
        let x = g.read::<u32>(p);
        let t = slot_target(g, w(0x51), 941);
        let e = g.call(t, 0, &[x]).eax;
        g.write::<u32>(obj + 0x38, base);
        let k = i32::from(g.read::<i16>(e.wrapping_add(0x12))).cast_unsigned();
        base = base.wrapping_add(k << 4);
        let r0 = g.read::<u32>(rec);
        let t = slot_target(g, w(0x76), 361);
        g.call(t, 0, &[e, obj, r0, x]);
        p += 0x10;
    }
    let t = slot_target(g, w(0x8c), 265);
    g.call(t, 0, &[0x40]).eax
}

/// [`bind_parts`] (164 bytes, 1 overlay).
pub const BIND_PARTS: Sig = Sig {
    len: 164,
    hash: 0x3827_6214_7c20_83f5,
    calls: &[],
    imports: &[],
};

/// Shakes the screen at time `t` (0x100012d0, internal: `ecx`/`edx` two
/// words `a`/`b`, cdecl `(t)`): nothing after 150 (returns the caller's
/// `eax`). Else slot 348 `(a, b, 0, -q, q, 0)` with `q = t * 255 / 150`
/// and the byte at operand `+0x60` = `0x80 - t * 128 / 150` (signed,
/// truncating). Returns the low half of the second product's `imul` with
/// that byte in its low 8 bits.
pub fn shake<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let t = fr.arg(g, 0).cast_signed();
    if t > 0x96 {
        return fr.eax;
    }
    let q = t.wrapping_mul(0xff) / 150;
    let s = slot_target(g, w(0x38), 348);
    let q = q.cast_unsigned();
    g.call(s, 0, &[fr.ecx, fr.edx, 0, q.wrapping_neg(), q, 0]);
    let x = t.wrapping_shl(7);
    let r = (x / 150).cast_unsigned();
    let byte = 0x80u32.wrapping_sub(r) & 0xff;
    #[allow(clippy::cast_possible_truncation)]
    g.write::<u8>(w(0x60), byte as u8);
    // mov eax, 0x1b4e81b5; imul edi: eax keeps the product's low half
    let low = x.cast_unsigned().wrapping_mul(0x1b4e_81b5);
    low & !0xff | byte
}

/// [`shake`] (105 bytes, 1 overlay).
pub const SHAKE: Sig = Sig {
    len: 105,
    hash: 0x98f9_556f_798f_186b,
    calls: &[],
    imports: &[],
};

/// Places the effect on its actor (0x10001180, internal, no arguments;
/// the actor `*G_A`, operand `+0x0b`): slot 215 `(actor)` gives its
/// model into `+0x18`, slot 210 `(model, M)` (the matrix at the
/// immediate `+0x1d`); slot 169 `(actor)` (ST0) times `K1` over `K2`
/// (`+0x4e`, `+0x61`), truncated, into the halfwords at `+0x6b` and
/// `+0x71`; slot 217 `(actor, P)` (`+0x53`), slot 325 `(A, P)` and `(B,
/// P)` (`+0x93`, `+0xaa`); `*G_H` (`+0xd7`) drops by slot 216 `(actor)`
/// (ST0) times `K3` (`+0xcf`); slot 251 `(actor)`'s ST0 into `+0xfc` and
/// the byte at `+0x105` 0x80. Every call sees the earlier calls' words.
/// Returns slot 251's `eax`.
pub fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let st = |o: crate::Out| crate::ob_model::st0_f32(o.st0);
    let actor = |g: &G, op: u32| g.read::<u32>(w(op));
    let mut stack = vec![actor(g, 0xb)];
    let t = slot_target(g, w(0x5), 215);
    let m = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x18), m);
    stack.splice(0..0, [m, w(0x1d)]);
    let t = slot_target(g, w(0x23), 210);
    g.call(t, 0, &stack);
    stack.insert(0, actor(g, 0x36));
    let t = slot_target(g, w(0x30), 169);
    // call width: 4 (the earlier calls' words stay on the stack)
    let s = st(g.call(t, 0, &stack));
    let x = div(mul(s, g.read::<f32>(w(0x4e))), g.read::<f32>(w(0x61)));
    #[allow(clippy::cast_possible_truncation)]
    let h = trunc(x) as u16;
    g.write::<u16>(w(0x6b), h);
    g.write::<u16>(w(0x71), h);
    stack.splice(0..0, [actor(g, 0x59), w(0x53)]);
    let t = slot_target(g, w(0x76), 217);
    // call width: 6 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for (tab, a, p) in [(0x83, 0x93, 0x88), (0x9a, 0xaa, 0x9f)] {
        stack.splice(0..0, [w(a), w(p)]);
        let t = slot_target(g, w(tab), 325);
        // call width: 10 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    stack.insert(0, actor(g, 0xb7));
    let t = slot_target(g, w(0xb1), 216);
    // call width: 11 (the earlier calls' words stay on the stack)
    let s = st(g.call(t, 0, &stack));
    let y = sub(g.read::<f32>(w(0xd7)), mul(s, g.read::<f32>(w(0xcf))));
    g.write::<f32>(w(0xee), y);
    stack.insert(0, actor(g, 0xe2));
    let t = slot_target(g, w(0xdc), 251);
    // call width: 12 (the earlier calls' words stay on the stack)
    let o = g.call(t, 0, &stack);
    g.write::<f32>(w(0xfc), st(o));
    g.write::<u8>(w(0x105), 0x80);
    o.eax
}

/// [`place`] (270 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 270,
    hash: 0xf81e_ddb4_3f2a_24f9,
    calls: &[],
    imports: &[],
};

/// Picks the effect's actor (0x10001780, cdecl `(n)`): `G_N` (operand
/// `+0x7`) = `n`, slot 168 `(n)` into `G_A` and `G_B` (`+0x1d`,
/// `+0x22`); unless that is 0xff the actor is placed ([`place`]); `+0x35`
/// and `+0x3f` 0, the part's handles
/// ([`part_handles`](crate::s4_small::part_handles)); then with the game
/// flag `**G_LOCK` (`+0x4d`) 1 slot 711 `(T, NAME, 0)` (`+0x6c`, `+0x61`)
/// and, the flag restored, slot 710 `(D, T, H, 0x200000)` (`+0x94`,
/// `+0x8f`, `+0x84`). Every call sees the earlier calls' words. Returns
/// 0.
pub fn pick_actor<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = fr.arg(g, 0);
    g.write::<u32>(w(0x7), n);
    let t = slot_target(g, w(0xd), 168);
    let a = g.call(t, 0, &[n]).eax;
    g.write::<u32>(w(0x1d), a);
    g.write::<u32>(w(0x22), a);
    // push ebp, the return address (the slot's word popped)
    let esp = fr.esp.wrapping_sub(8);
    if a != 0xff {
        let pl = Frame {
            at: img.rel32_target(f + 0x2e),
            esp,
            eax: a,
            ..*fr
        };
        place(g, img, &pl);
    }
    g.write::<u32>(w(0x35), 0);
    g.write::<u32>(w(0x3f), 0);
    // push esi too
    let ph = Frame {
        at: img.rel32_target(f + 0x48),
        esp: esp.wrapping_sub(4),
        ..*fr
    };
    crate::s4_small::part_handles(g, img, &ph);
    let lock = g.read::<u32>(w(0x4d));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x5c), 711);
    g.call(t, 0, &[w(0x6c), w(0x61), 0]);
    let lock = g.read::<u32>(w(0x73));
    g.write::<u32>(lock, saved);
    let t = slot_target(g, w(0x7f), 710);
    g.call(
        t,
        0,
        &[w(0x94), w(0x8f), w(0x84), 0x20_0000, w(0x6c), w(0x61), 0],
    );
    crate::Ret::eax(0)
}

/// [`pick_actor`] (162 bytes, 1 overlay).
pub const PICK_ACTOR: Sig = Sig {
    len: 162,
    hash: 0x4b02_c1c6_101d_9a2b,
    calls: &[
        (0x2e, crate::Callee::Sig(&PLACE)),
        (0x48, crate::Callee::Sig(&crate::s4_small::PART_HANDLES)),
    ],
    imports: &[],
};
