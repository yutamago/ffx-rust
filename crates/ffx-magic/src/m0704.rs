//! `magic_0704.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{cvt, mul};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Sets the fade for step `i` (0x10001000, internal: `ecx` = `i`):
/// nothing from step 9 (returns the caller's `eax`). The level byte is
/// `i << 7` (its low byte) up to step 1, else `((9 - i) << 7) / 8`; slot
/// 776 `(0x44)` gives a handle `h`, slot 915 `(0x3fff0000 three times,
/// level * K, h)` (`K` at operand `+0x52`). Returns slot 915's `eax`.
pub fn fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let i = fr.ecx.cast_signed();
    if i >= 9 {
        return fr.eax;
    }
    let level = if i <= 1 {
        (fr.ecx << 7) & 0xff
    } else {
        // cdq; and edx, 7; sar 3: a signed divide by 8
        (((9 - i) << 7) / 8).cast_unsigned() & 0xff
    };
    let t = slot_target(g, w(0x27), 776);
    let h = g.call(t, 0, &[0x44]).eax;
    let x = mul(cvt(level), g.read::<f32>(w(0x52)));
    let t = slot_target(g, w(0x41), 915);
    let c = 0x3fff_0000;
    g.call(t, 0, &[c, c, c, x.to_bits(), h]).eax
}

/// [`fade`] (125 bytes, 1 overlay).
pub const FADE: Sig = Sig {
    len: 125,
    hash: 0x78fb_4e91_aca8_bc7d,
    calls: &[],
    imports: &[],
};

/// Loads the effect data (0x10001080, internal, no arguments; the data
/// at the immediate `+0x06`): slot 718 `(data)`, slots 222 and 221
/// `(data, 0, 1000, 0x100)`, three slot 719 `(data, 6, k)` into the
/// words at `+0x65`, `+0x80` and `+0x9e`, each call seeing the earlier
/// calls' words. Returns the last entry.
pub fn load<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let data = w(0x6);
    let t = slot_target(g, w(0x1), 718);
    let mut stack = vec![data];
    g.call(t, 0, &stack);
    for (op, slot) in [(0x13, 222), (0x31, 221)] {
        let t = slot_target(g, w(op), slot);
        stack.splice(0..0, [data, 0, 0x3e8, 0x100]);
        // call width: 9 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    let mut eax = 0;
    for (op, k, out) in [(0x4f, 0, 0x65), (0x6a, 1, 0x80), (0x85, 2, 0x9e)] {
        let t = slot_target(g, w(op), 719);
        stack.splice(0..0, [data, 6, k]);
        // call width: 18 at most (the earlier calls' words stay on the stack)
        eax = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(out), eax);
    }
    eax
}

/// [`load`] (163 bytes, 1 overlay).
pub const LOAD: Sig = Sig {
    len: 163,
    hash: 0x8d00_92ba_efe4_6f03,
    calls: &[],
    imports: &[],
};

/// One of the start's three actors (`id`, operand `+id`, already stored):
/// unless 0xff, its model (slot 215 `(id)`) into `+model`, and its shade
/// `trunc(slot 169 (*id) * K / D)` (ST0 stored into the argument's word
/// as `f32` first; `K`, `D` at `+k`, `+d`) into the halfwords at `+s1`,
/// `+s2`. Returns whether it ran.
fn actor_shade<G: Game>(g: &mut G, img: &Image, fr: &Frame, id: u32, ops: [u32; 6]) -> bool {
    use crate::sb_sse::div;
    let w = |off: u32| img.word(fr.at + off);
    let [tbl, model, again, k, d, s1] = ops;
    if id == 0xff {
        return false;
    }
    let mut p = Pushes::default();
    p.push(id);
    let t = slot_target(g, w(tbl), 215);
    // call width: 12 (Pushes caps the words at 12)
    let m = p.call(g, t, 0).eax;
    p.push(g.read::<u32>(w(again)));
    g.write::<u32>(w(model), m);
    let t = slot_target(g, w(tbl), 169);
    // fstp dword [ebp + 8]: an empty x87 stack stores the indefinite NaN
    // call width: 12 (Pushes caps the words at 12)
    let x = p
        .call(g, t, 0)
        .st0
        .map_or(f32::from_bits(0xffc0_0000), |v| v as f32);
    g.write::<u32>(fr.esp + 4, x.to_bits());
    let kx = mul(x, f32::from_bits(g.read::<u32>(w(k))));
    let v = crate::s4_summon::trunc(div(kx, f32::from_bits(g.read::<u32>(w(d))))) as u16;
    g.write::<u16>(w(s1), v);
    g.write::<u16>(w(s1 + 6), v);
    true
}

/// The effect's start (0x100011d0, an overlay table entry, cdecl
/// `(arg)`): `arg` into `G_ARG` (operand `+0x8`); the actor = slot 168
/// `(arg)`; its mode byte through slot 218 `(*G_ARG, &b)` (the local at
/// `ebp - 4`); slot 215 the actor's and the mode's models; then the
/// actor, the next and the one after ([`actor_shade`]) with their models
/// and shades; the flag at `+0x1a2` 0, the data loaded ([`load`]); slot
/// 711 `(BLOCK, LIST, 0)` with `**G_LOCK` 1 around it, slot 710 `(ENV,
/// BLOCK, FLAG, 0x100000)` and slot 278 `(model, 0, 1)`. Each call sees
/// the words the earlier ones left. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(0xe), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x8), arg);
    let mut p = Pushes::default();
    p.push(arg);
    let actor = call(g, &p, 168);
    g.write::<u32>(w(0x1b), actor);
    p.push_all(&[fr.esp.wrapping_sub(8), r(g, 0x2a)]);
    let at = call(g, &p, 218);
    let mode = u32::from(g.read::<u8>(at));
    p.push(r(g, 0x3b));
    g.write::<u32>(w(0x40), mode);
    let m = call(g, &p, 215);
    p.push(r(g, 0x53));
    g.write::<u32>(w(0x58), m);
    let m = call(g, &p, 215);
    g.write::<u32>(w(0x6a), m);
    let mut c = r(g, 0x6f);
    g.write::<u32>(w(0x77), c);
    if actor_shade(g, img, fr, c, [0x84, 0x97, 0x92, 0xb4, 0xbf, 0xc9]) {
        c = r(g, 0xd4);
    }
    let c1 = c.wrapping_add(1);
    g.write::<u32>(w(0xdd), c1);
    if actor_shade(g, img, fr, c1, [0xea, 0xfe, 0xf9, 0x11b, 0x126, 0x130]) {
        c = r(g, 0x13b);
    }
    let c2 = c.wrapping_add(2);
    g.write::<u32>(w(0x143), c2);
    actor_shade(g, img, fr, c2, [0x150, 0x163, 0x15e, 0x180, 0x18b, 0x195]);
    g.write::<u32>(w(0x1a2), 0);
    // push ebp, push ecx, push esi, the return address
    let callee = Frame {
        at: img.rel32_target(f + 0x1ab),
        esp: fr.esp.wrapping_sub(0x10),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    load(g, img, &callee);
    let lockp = r(g, 0x1b0);
    let mut p = Pushes::default();
    p.push(0);
    let saved = g.read::<u32>(lockp);
    g.write::<u32>(lockp, 1);
    p.push_all(&[w(0x1c4), w(0x1cf)]);
    call(g, &p, 711);
    let lockp = r(g, 0x1d6);
    p.push(0x10_0000);
    g.write::<u32>(lockp, saved);
    p.push_all(&[w(0x1e7), w(0x1f2), w(0x1f7)]);
    call(g, &p, 710);
    p.push_all(&[1, 0, r(g, 0x20e)]);
    call(g, &p, 278);
    Ret::eax(0)
}

/// [`start`] (542 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 542,
    hash: 0x30ba_6ccb_5a88_08ae,
    calls: &[(0x1ab, Callee::Sig(&LOAD))],
    imports: &[],
};

/// The effect's draw (0x100018e0, an overlay table entry, no arguments):
/// nothing until the run flag (operand `+0x2`) is set or while the time
/// `G_T` (`+0xf`) is negative. `*G_PPP = PART`, `*G_ENV = BLOCK`, slots 723
/// `(*G_X)` and 724 `(*G_Y)`. While the light part's flag (u16 `+0x5c`) is
/// set, its time `t = part[+0x8] >> 12` fades the light in (slot 352
/// `((64 t / 30) << 24, 0x44, 0)` below 30), holds it (`0x40000000`
/// below 80) and fades it out after ([`fade`] with `t - 80` when positive). The
/// two parts are refreshed
/// ([`part_refresh_b`](crate::sa_misc::part_refresh_b)); when the sound
/// reached 0xb2 and its time, slot 213 `(id, 0)` stops the three sounds and
/// slot 214 `(*G_ARG)` follows. The time counts. Returns 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(0x37), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    if r(g, 0x2) == 0 || r(g, 0xf).cast_signed() < 0 {
        return Ret::eax(0);
    }
    let mut p = Pushes::default();
    p.push(r(g, 0x21));
    g.write::<u32>(r(g, 0x1b), w(0x27));
    g.write::<u32>(r(g, 0x2c), w(0x32));
    call(g, &p, 723);
    p.push(r(g, 0x4a));
    call(g, &p, 724);
    if g.read::<u16>(w(0x5c)) != 0 {
        let t = (r(g, 0x65).cast_signed() >> 12).cast_unsigned();
        let level = if t < 0x1e {
            Some(((t << 6) / 30) << 24)
        } else if t < 0x50 {
            Some(0x4000_0000)
        } else {
            None
        };
        if let Some(level) = level {
            let mut p = Pushes::default();
            p.push_all(&[0, 0x44, level]);
            call(g, &p, 352);
        } else {
            let n = t.wrapping_sub(0x50);
            if n.cast_signed() > 0 {
                let callee = Frame {
                    at: img.rel32_target(f + 0xb0),
                    esp: fr.esp.wrapping_sub(4),
                    eax: 0,
                    ecx: n,
                    edx: 0,
                    xmm: [0; 4],
                };
                fade(g, img, &callee);
            }
        }
    }
    let refresh = img.rel32_target(f + 0xba);
    crate::sa_misc::part_refresh_b(g, img, refresh, w(0xb5), 0);
    let refresh = img.rel32_target(f + 0xc4);
    crate::sa_misc::part_refresh_b(g, img, refresh, w(0xbf), 0);
    if r(g, 0xca) == 0xb2 && r(g, 0xd5) == r(g, 0xdb) {
        let first = w(0xe4);
        for i in 0..3u32 {
            let id = g.read::<u32>(first.wrapping_add(0x18 * i));
            let mut p = Pushes::default();
            p.push_all(&[0, id]);
            call(g, &p, 213);
        }
        let mut p = Pushes::default();
        p.push(r(g, 0x111));
        call(g, &p, 214);
    }
    let v = r(g, 0x124).wrapping_add(1);
    g.write::<u32>(w(0x124), v);
    Ret::eax(0)
}

/// [`draw`] (299 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 299,
    hash: 0x4509_9d9f_7f6b_b0db,
    calls: &[
        (0xb0, Callee::Sig(&FADE)),
        (0xba, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xc4, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
    ],
    imports: &[],
};
