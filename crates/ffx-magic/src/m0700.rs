//! `magic_0700.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start (0x10001110, an overlay table entry, cdecl
/// `(arg)`): `arg` into `G_ARG` (operand `+0x8`); the actor = slot 168
/// `(arg)`; its mode byte through slot 218 `(*G_ARG, &arg)`; slot 215 the
/// actor's and the mode's models; `G_STEP` -1; the part started
/// ([`part_start`](crate::s4_misc::part_start)); slot 711 `(BLOCK, LIST,
/// 0)` with `**G_LOCK` 1 around it, slot 710 `(ENV, BLOCK, MODE, 0x100000)`;
/// `*G_PPP = ENV`, program 3 of slot 941 `(*G_D)` started on the part
/// ([`start_part`](crate::opu_parts::start_part)); its three vectors
/// copied and its flag set; slot 268 `(model, 0x10, V)`, `V.w` 1.0; slot
/// 345 `(model)` (`ecx` the part, as the start left it), slot 350
/// `(model)`, slot 351 `(ENV, NAME, that + 0x400)` into `G_CLUT` and slot
/// 349 `(model, clut)`. Each call sees the words the earlier ones left on
/// the stack. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_with(g, img, fr, 0)
}

/// [`start`] as `magic_0703` has it: the first program (`push [eax]`, a
/// byte shorter: the code after `+0xed` one byte back).
pub fn start_0703<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_with(g, img, fr, 1)
}

/// [`start`]'s body; `shift` 1: `push [eax]` (program 0) and the code
/// after it one byte back, else `push [eax + d8]` (the `disp8` at
/// `+0xef` picks the program).
fn start_with<G: Game>(g: &mut G, img: &Image, fr: &Frame, shift: u32) -> Ret {
    let f = fr.at;
    // operands after the program's push (+0xed) move back by `shift`
    let o = |x: u32| if x > 0xed { x - shift } else { x };
    let w = |off: u32| img.word(f + o(off));
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let slot = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(0xe), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x8), arg);
    let mut p = Pushes::default();
    p.push(arg);
    let actor = slot(g, &p, 168);
    g.write::<u32>(w(0x1b), actor);
    p.push_all(&[fr.esp + 4, r(g, 0x2a)]);
    let at = slot(g, &p, 218);
    let mode = u32::from(g.read::<u8>(at));
    p.push(r(g, 0x3b));
    g.write::<u32>(w(0x40), mode);
    let m = slot(g, &p, 215);
    p.push(r(g, 0x53));
    g.write::<u32>(w(0x58), m);
    let m = slot(g, &p, 215);
    g.write::<u32>(w(0x6a), m);
    g.write::<u32>(w(0x70), 0xffff_ffff);
    // push ebp, push esi, five words, the return address
    let callee = Frame {
        at: img.rel32_target(f + 0x79),
        esp: fr.esp.wrapping_sub(0x20),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s4_misc::part_start(g, img, &callee);
    let lockp = r(g, 0x7e);
    p.push(0);
    let saved = g.read::<u32>(lockp);
    g.write::<u32>(lockp, 1);
    p.push_all(&[w(0x92), w(0x9d)]);
    slot(g, &p, 711);
    let lockp = r(g, 0xa4);
    p.push(0x10_0000);
    g.write::<u32>(lockp, saved);
    p.push_all(&[w(0xb5), w(0xc0), w(0xc5)]);
    slot(g, &p, 710);
    g.write::<u32>(r(g, 0xcc), w(0xd2));
    p.push(r(g, 0xdd));
    let table = slot(g, &p, 941);
    let entry = if shift == 0 {
        crate::o4_parts::disp8(img, f + 0xef)
    } else {
        0
    };
    let prog = g.read::<u32>(table.wrapping_add(entry));
    let part = w(0xf6);
    let start = img.rel32_target(f + o(0xfb));
    crate::opu_parts::start_part(g, img, start, part, 0x1000, prog, 1);
    for (src, dst) in [(0x102u32, 0x10cu32), (0x114, 0x12c), (0x133, 0x147)] {
        let (s, d) = (w(src), w(dst));
        for i in 0..4 {
            let v = g.read::<u32>(s + 4 * i);
            g.write::<u32>(d + 4 * i, v);
        }
    }
    g.write::<u16>(w(0x11f), 1);
    let mut p = Pushes::default();
    p.push_all(&[w(0x138), 0x10, r(g, 0x140)]);
    slot(g, &p, 268);
    p.push(r(g, 0x15a));
    g.write::<u32>(w(0x160), 0x3f80_0000);
    // `ecx` is the part still (the start's last value)
    let t = slot_target(g, w(0x154), 345);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, part);
    p.push(r(g, 0x177));
    let base = slot(g, &p, 350).wrapping_add(0x400);
    p.push_all(&[base, w(0x18f), w(0x19a)]);
    let clut = slot(g, &p, 351);
    p.push_all(&[clut, r(g, 0x1a3)]);
    g.write::<u32>(w(0x1a8), clut);
    slot(g, &p, 349);
    Ret::eax(0)
}

/// [`start`] (449 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 449,
    hash: 0x276d_c8d1_d017_804b,
    calls: &[
        (0x79, Callee::Sig(&crate::s4_misc::PART_START)),
        (0xfb, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// The effect's frame on its cue (0x10001320, an overlay table entry, no
/// arguments): until the go flag (`+0x2`) is set, slot 225 `(*G_ARG)`
/// answering non-zero waits; once, slot 226 `(*G_ARG, 1)`; until the run
/// flag (`+0x5a`), slot 227 `(*G_ARG)` below 1 waits. While the step
/// `G_STEP` is negative: slot 347 `(*G_ARG)` with bit 0 clear waits; else
/// slot 220 `(*G_M, 0x40, 0x100)`, slot 239 `(*G_ARG, -5, 0)` and the
/// step 0. Then slot 349 `(model, clut)`, and for the steps 1..=15 the
/// fade slot 348 `(model, clut, 0, -q, q, 0)` with `q = 64 n / 15`, for
/// 16..=45 with `q = 64 - 64 (n - 15) / 30`. Returns 0.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let call = |g: &mut G, words: &[u32], slot: u32| {
        let mut p = Pushes::default();
        p.push_all(words);
        let t = slot_target(g, w(0xa), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    let inc = |g: &mut G, off: u32| {
        let v = r(g, off).wrapping_add(1);
        g.write::<u32>(w(off), v);
    };
    if r(g, 0x2) == 0 {
        if call(g, &[r(g, 0x10)], 225) != 0 {
            return Ret::eax(0);
        }
        inc(g, 0x29);
    }
    if r(g, 0x2f) == 0 {
        let a = r(g, 0x3f);
        g.write::<u32>(w(0x4b), 1);
        call(g, &[1, a], 226);
    }
    if r(g, 0x5a) == 0 {
        if call(g, &[r(g, 0x68)], 227).cast_signed() < 1 {
            return Ret::eax(0);
        }
        inc(g, 0x82);
    }
    if r(g, 0x88).cast_signed() < 0 {
        if call(g, &[r(g, 0x96)], 347) & 1 == 0 {
            return Ret::eax(0);
        }
        let mut p = Pushes::default();
        p.push_all(&[0x100, 0x40, r(g, 0xc1)]);
        let t = slot_target(g, w(0xc8), 220);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0);
        p.push_all(&[0, 0xffff_fffb, r(g, 0xd8)]);
        let t = slot_target(g, w(0xc8), 239);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0);
        g.write::<u32>(w(0xe3), 0);
    }
    call(g, &[r(g, 0xed), r(g, 0xf8)], 349);
    let n = r(g, 0x106).cast_signed();
    let n = if (1..=15).contains(&n) {
        let q = n * 64 / 15;
        let words = [0, q.cast_unsigned(), q.wrapping_neg().cast_unsigned(), 0];
        call(
            g,
            &[
                words[0],
                words[1],
                words[2],
                words[3],
                r(g, 0x13a),
                r(g, 0x146),
            ],
            348,
        );
        r(g, 0x14e).cast_signed()
    } else {
        n
    };
    let m = n.wrapping_sub(0xf);
    if (1..=30).contains(&m) {
        let c = 0x40 - m * 64 / 30;
        let words = [0, c.cast_unsigned(), c.wrapping_neg().cast_unsigned(), 0];
        call(
            g,
            &[
                words[0],
                words[1],
                words[2],
                words[3],
                r(g, 0x193),
                r(g, 0x199),
            ],
            348,
        );
    }
    Ret::eax(0)
}

/// [`frame`] (422 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 422,
    hash: 0x6291_a50b_dc98_b1d9,
    calls: &[],
    imports: &[],
};

/// [`start`] in `magic_0701` (0x10001110; program 2).
pub const START_0701: Sig = Sig {
    len: 449,
    hash: 0xfdaf_4541_1550_055f,
    calls: &[
        (0x79, Callee::Sig(&crate::s4_misc::PART_START)),
        (0xfb, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// [`start`] in `magic_0702` (0x10001110; program 1).
pub const START_0702: Sig = Sig {
    len: 449,
    hash: 0xffe5_57a5_44ca_cb73,
    calls: &[
        (0x79, Callee::Sig(&crate::s4_misc::PART_START)),
        (0xfb, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// [`start_0703`] (0x10001110, 448 bytes, 1 overlay).
pub const START_0703: Sig = Sig {
    len: 448,
    hash: 0x542d_06c4_a50e_844d,
    calls: &[
        (0x79, Callee::Sig(&crate::s4_misc::PART_START)),
        (0xfa, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};
