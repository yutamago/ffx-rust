//! `magic_0707.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Starts a part with its look (0x100010b0, internal: `ecx` = the
/// object, `xmm2`/`xmm3` two floats, cdecl `(c, t1, t2, t3, r, g, b)`):
/// `*G_P = ITEM` (`+0x07`, `+0x0e`), slot 941 `(*G_D)` (`+0x19`) gives
/// the program table, the part started with its first program (the
/// `pppStartPart` wrapper, `(0x1000, table[0], 1)`); then `+0x1e` 1 (a
/// halfword), `+0x40`/`+0x44` the two floats, `+0x48` `c`, `+0x50..+0x58`
/// the three `t` truncated, `+0x60..+0x68` the colour, `+0x4c`, `+0x6c`
/// 1.0 and `+0x5c` 1. Returns `t3` truncated.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let (a, b) = (fr.xmm[2] as u32, fr.xmm[3] as u32);
    let gp = g.read::<u32>(w(0x7));
    g.write::<u32>(gp, w(0xe));
    let d = g.read::<u32>(w(0x19));
    let t = slot_target(g, w(0x13), 941);
    let table = g.call(t, 0, &[d]).eax;
    let program = g.read::<u32>(table);
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x3f), obj, 0x1000, program, 1);
    let arg = |g: &G, i: u32| fr.arg(g, i);
    g.write::<u16>(obj + 0x1e, 1);
    g.write::<u32>(obj + 0x40, a);
    let t1 = trunc(f32::from_bits(arg(g, 1)));
    g.write::<u32>(obj + 0x50, t1);
    let t2 = trunc(f32::from_bits(arg(g, 2)));
    g.write::<u32>(obj + 0x44, b);
    let c = arg(g, 0);
    g.write::<u32>(obj + 0x48, c);
    let r = arg(g, 4);
    g.write::<u32>(obj + 0x54, t2);
    let t3 = trunc(f32::from_bits(arg(g, 3)));
    g.write::<u32>(obj + 0x60, r);
    let gr = arg(g, 5);
    g.write::<u32>(obj + 0x64, gr);
    let bl = arg(g, 6);
    g.write::<u32>(obj + 0x4c, ONE);
    g.write::<u32>(obj + 0x58, t3);
    g.write::<u32>(obj + 0x5c, 1);
    g.write::<u32>(obj + 0x68, bl);
    g.write::<u32>(obj + 0x6c, ONE);
    t3
}

/// [`start`] (189 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 189,
    hash: 0x7093_244f_3970_78b9,
    calls: &[(0x3f, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's start (0x10001210, an overlay table entry, cdecl
/// `(arg)`): `arg` into `G_ARG` (operand `+0x7`); the actor = slot 168
/// `(arg)` into two globals; unless it is 0xff, its model (slot 215) into
/// `G_M` and the shade `trunc(slot 169 (actor) * K / D)` (its ST0 stored
/// into the argument's word first, as `f32`) into two halfwords; the flag
/// at `+0x81` 0, the part started
/// ([`part_start`](crate::s4_misc::part_start)); slot 711
/// `(BLOCK, LIST, 0)` with `**G_LOCK` 1 around it, slot 710 `(ENV, BLOCK, FLAG,
/// 0x100000)`. Each call sees the words the earlier ones left. Returns 0.
pub fn effect_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(0xd), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0)
    };
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x7), arg);
    let mut p = Pushes::default();
    p.push(arg);
    let actor = call(g, &p, 168).eax;
    p.pop(1);
    g.write::<u32>(w(0x1d), actor);
    g.write::<u32>(w(0x22), actor);
    if actor != 0xff {
        p.push(actor);
        let m = call(g, &p, 215).eax;
        p.push(r(g, 0x3d));
        g.write::<u32>(w(0x42), m);
        // fstp dword [ebp + 8]: an empty x87 stack stores the indefinite NaN
        let st0 = call(g, &p, 169).st0;
        let x = st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32);
        g.write::<u32>(fr.esp + 4, x.to_bits());
        let shade = div(
            mul(x, f32::from_bits(r(g, 0x5f))),
            f32::from_bits(r(g, 0x6a)),
        );
        let v = trunc(shade) as u16;
        g.write::<u16>(w(0x74), v);
        g.write::<u16>(w(0x7a), v);
    }
    g.write::<u32>(w(0x81), 0);
    // push ebp, push esi, the return address
    let callee = Frame {
        at: img.rel32_target(f + 0x8a),
        esp: fr.esp.wrapping_sub(0xc),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s4_misc::part_start(g, img, &callee);
    let lockp = r(g, 0x8f);
    let mut p = Pushes::default();
    p.push(0);
    let saved = g.read::<u32>(lockp);
    g.write::<u32>(lockp, 1);
    p.push_all(&[w(0xa3), w(0xae)]);
    call(g, &p, 711);
    let lockp = r(g, 0xb5);
    p.push(0x10_0000);
    g.write::<u32>(lockp, saved);
    p.push_all(&[w(0xc6), w(0xd1), w(0xd6)]);
    call(g, &p, 710);
    Ret::eax(0)
}

/// [`effect_start`] (228 bytes, 1 overlay).
pub const EFFECT_START: Sig = Sig {
    len: 228,
    hash: 0x6a17_1352_2d0a_e518,
    calls: &[(0x8a, Callee::Sig(&crate::s4_misc::PART_START))],
    imports: &[],
};

/// The effect's frame (0x10001310, an overlay table entry, no arguments;
/// a /GS frame): until the go flag (operand `+0x12`), slot 225 `(*G_ARG)`
/// answering non-zero waits; once, slot 226 `(*G_ARG, 0)`; until the run
/// flag (`+0x6a`), slot 227 `(*G_ARG)` below 1 waits. `**G_PAUSE` = slot
/// 253 answering 0, which returns at once. On the effect's first frame
/// (`G_T`, `+0xc4`, 0) the sound record `R` (`+0xd2`) starts unless its id
/// is 0xff (`R+0x10` 0, `R+0x8` 0xc8, `R+0xc` slot 228 `(id, 0xc8)`) and
/// slot 278 `(*G_M, 0, 0)` and `(*G_M, 3, 0)` reset the actor. The sound
/// moves on ([`sound_start`](crate::sa_misc::sound_start) with 0xc9 when
/// its time reaches `R+0xc`, 0xca at time 0x5a of 0xc9). Every 8th frame
/// from 5 below 305 a free part of the 16 (u16 `+0x1e` 0) is started
/// ([`start`]) from four draws of slot 307 (its ST0): a height `h`, an
/// angle `a` turned by slot 247 `(&M, -trunc(a))` and slot 229 `(&p, &M,
/// &p)` on `(h, 0, 0, 1)`, a second height and a size; at frame 5 slot 220
/// `(*G_S, 0x40, 0x100)`. Unless paused the time counts. Returns 0.
#[allow(clippy::too_many_lines)] // the frame as the original orders it
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let k = |g: &G, off: u32| f32::from_bits(r(g, off));
    let call = |g: &mut G, words: &[u32], slot: u32| {
        let mut p = Pushes::default();
        p.push_all(words);
        let t = slot_target(g, w(0x1a), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0)
    };
    let inc = |g: &mut G, off: u32| {
        let v = r(g, off).wrapping_add(1);
        g.write::<u32>(w(off), v);
    };
    // fstp dword: ST0 as f32 (an empty stack stores the indefinite NaN)
    let st0 = |o: crate::Out| o.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32);
    if r(g, 0x12) == 0 {
        if call(g, &[r(g, 0x20)], 225).eax != 0 {
            return Ret::eax(0);
        }
        inc(g, 0x39);
    }
    if r(g, 0x3f) == 0 {
        let a = r(g, 0x4f);
        g.write::<u32>(w(0x5b), 1);
        call(g, &[0, a], 226);
    }
    if r(g, 0x6a) == 0 {
        if call(g, &[r(g, 0x78)], 227).eax.cast_signed() < 1 {
            return Ret::eax(0);
        }
        inc(g, 0x92);
    }
    let running = call(g, &[], 253).eax != 0;
    let pause = r(g, 0xa6);
    if !running {
        g.write::<u32>(pause, 1);
        return Ret::eax(0);
    }
    let first = r(g, 0xc4) == 0;
    g.write::<u32>(pause, 0);
    if first {
        let id = r(g, 0xd2);
        if id != 0xff {
            g.write::<u32>(w(0xea), 0);
            g.write::<u32>(w(0xf4), 0xc8);
            let e = call(g, &[0xc8, id], 228).eax;
            g.write::<u32>(w(0x108), e);
        }
        let mut p = Pushes::default();
        p.push_all(&[0, 0, r(g, 0x11d)]);
        let t = slot_target(g, w(0x10d), 278);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0);
        p.push_all(&[0, 3, r(g, 0x134)]);
        let t = slot_target(g, w(0x124), 278);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0);
    }
    let next = match r(g, 0x13e) {
        0xc8 if r(g, 0x15d) == r(g, 0x163) => Some(0xc9),
        0xc9 if r(g, 0x14e) == 0x5a => Some(0xca),
        _ => None,
    };
    if let Some(v) = next {
        crate::sa_misc::sound_start(g, img, img.rel32_target(f + 0x174), w(0x16f), v);
    }
    let b = r(g, 0x17b).wrapping_sub(5);
    if b < 0x12c && b.trailing_zeros() >= 3 {
        let first_part = w(0x19e);
        let part = (0..16u32)
            .map(|i| first_part.wrapping_add(0x80 * i))
            .find(|&p| g.read::<u16>(p.wrapping_add(0x1e)) == 0);
        if let Some(part) = part {
            let ebp = fr.esp.wrapping_sub(4);
            let (m, p) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
            let a = st0(call(g, &[], 307));
            let h = add(mul(sub(a, k(g, 0x1cd)), k(g, 0x1e0)), k(g, 0x1e8));
            let b2 = st0(call(g, &[], 307));
            let angle = mul(sub(b2, k(g, 0x1ff)), k(g, 0x207));
            let turn = trunc(angle).wrapping_neg();
            let mut st = Pushes::default();
            st.push_all(&[turn, m]);
            let t = slot_target(g, w(0x21c), 247);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, 0);
            for (o, v) in [(0u32, h.to_bits()), (8, 0), (4, 0), (0xc, 0x3f80_0000)] {
                g.write::<u32>(p + o, v);
            }
            st.push_all(&[p, m, p]);
            let t = slot_target(g, w(0x23a), 229);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, 0);
            let y = g.read::<u32>(p);
            g.write::<u32>(p + 4, y);
            let c = st0(call(g, &[], 307));
            let x = mul(sub(c, k(g, 0x283)), k(g, 0x290));
            g.write::<u32>(p, x.to_bits());
            let d = st0(call(g, &[], 307));
            let size = add(mul(sub(d, k(g, 0x2ad)), k(g, 0x2c4)), k(g, 0x2cc));
            let z = g.read::<u32>(p + 8);
            let s = size.to_bits();
            // push ebx, push esi, the five words left, `sub esp, 8`
            let esp = fr.esp.wrapping_sub(0x88);
            for (i, v) in (0u32..).zip([z, 0, angle.to_bits(), 0, s, s, s]) {
                g.write::<u32>(esp + 4 + 4 * i, v);
            }
            let callee = Frame {
                at: img.rel32_target(f + 0x308),
                esp,
                eax: 0,
                ecx: part,
                edx: 0,
                xmm: [0, 0, u128::from(x.to_bits()), u128::from(y)],
            };
            start(g, img, &callee);
        }
        if b == 0 {
            call(g, &[0x100, 0x40, r(g, 0x328)], 220);
        }
    }
    if g.read::<u32>(r(g, 0x332)) == 0 {
        inc(g, 0x33e);
    }
    Ret::eax(0)
}

/// [`frame`] (850 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 850,
    hash: 0xc7d2_6ce5_1f76_0c92,
    calls: &[
        // __security_check_cookie
        (0xba, Callee::Any),
        (0x174, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x308, Callee::Sig(&START)),
        (0x34a, Callee::Any),
    ],
    imports: &[],
};

/// The effect's draw (0x10001670, an overlay table entry, no arguments; a
/// /GS frame): nothing until the run flag (operand `+0x12`) is set, or
/// while the time `G_T` (`+0x1f`) is negative. `*G_PPP = ENV`, `*G_ENV =
/// BLOCK`, slots 723 `(*G_X)` and 724 `(*G_Y)`, slot 236 `(*G_M, 6, &M)`
/// (the actor's bone matrix into the local at `ebp - 0x54`); each of the
/// 16 parts with its flag (u16 `+0x1e`) set has its position `+0x40`
/// moved by `M` (slot 229 `(pos, &M, &p)`, the local `p` at `ebp - 0x14`
/// holding the old position first), refreshed
/// ([`part_refresh_b`](crate::sa_misc::part_refresh_b)), and the old
/// position put back. When the sound reached 0xca and its time `R+0xc`:
/// slot 278 `(*G_M, 0, 1)` and `(*G_M, 3, 0)`, slot 213 `(R, 0)`, slot 214
/// `(*G_ARG)`. Unless paused the time counts. Returns 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let tbl = 0x49;
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(tbl), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0)
    };
    if r(g, 0x12) == 0 || r(g, 0x1f).cast_signed() < 0 {
        return Ret::eax(0);
    }
    g.write::<u32>(r(g, 0x2b), w(0x32));
    let mut p = Pushes::default();
    p.push(r(g, 0x3e));
    g.write::<u32>(r(g, 0x37), w(0x44));
    call(g, &p, 723);
    p.push(r(g, 0x5c));
    call(g, &p, 724);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, at) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    p.push_all(&[m, 6, r(g, 0x75)]);
    call(g, &p, 236);
    let refresh = img.rel32_target(f + 0xc7);
    let first = w(0x8a);
    for i in 0..16u32 {
        let pos = first.wrapping_add(0x80 * i);
        if g.read::<u16>(pos.wrapping_sub(0x22)) == 0 {
            continue;
        }
        let old: [u32; 4] = [0u32, 4, 8, 12].map(|o| g.read::<u32>(pos + o));
        for (o, v) in (0u32..).zip(old) {
            g.write::<u32>(at + 4 * o, v);
        }
        let mut p = Pushes::default();
        p.push_all(&[at, m, pos]);
        call(g, &p, 229);
        crate::sa_misc::part_refresh_b(g, img, refresh, pos.wrapping_sub(0x40), 0);
        for o in 0..4 {
            let v = g.read::<u32>(at + 4 * o);
            g.write::<u32>(pos + 4 * o, v);
        }
    }
    if r(g, 0xe6) == 0xca && r(g, 0xf3) == r(g, 0xf9) {
        let mut p = Pushes::default();
        p.push_all(&[1, 0, r(g, 0x110)]);
        call(g, &p, 278);
        p.push_all(&[0, 3, r(g, 0x127)]);
        call(g, &p, 278);
        p.push_all(&[0, r(g, 0x136)]);
        call(g, &p, 213);
        p.push(r(g, 0x149));
        call(g, &p, 214);
    }
    if g.read::<u32>(r(g, 0x159)) == 0 {
        let v = r(g, 0x164).wrapping_add(1);
        g.write::<u32>(w(0x164), v);
    }
    Ret::eax(0)
}

/// [`draw`] (376 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 376,
    hash: 0xcb35_b1ac_c256_260e,
    calls: &[
        (0xc7, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        // __security_check_cookie
        (0x170, Callee::Any),
    ],
    imports: &[],
};
