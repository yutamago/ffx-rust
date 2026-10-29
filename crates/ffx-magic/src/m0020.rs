//! `magic_0020.dll`'s own functions (besides the runtime, the object
//! host and the shared code): the step table's entries and the object
//! callbacks.

use ffx_bridge::x87::cvttsd2si;

use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// A step (0x10001000, no arguments): slot 374, then slot 375 with the
/// byte at `*G_B` (operand `+0x0e`). Returns 1.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s374 = slot_target(g, w(0x01), 374);
    g.call(s374, fr.ecx, &[]);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x0e))));
    let s375 = slot_target(g, w(0x17), 375);
    g.call(s375, fr.ecx, &[b]);
    Ret::eax(1)
}

/// 0x10001000 (44 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 44,
    hash: 0xbfff_b265_78e1_73a4,
    calls: &[],
    imports: &[],
};

/// A step (0x10001030, no arguments): slot 373's result into `G_R`
/// (operand `+0x0e`); 5 sets `G_DONE` (`+0x19`) and returns 0, anything
/// else returns 1.
pub fn e1030<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s373 = slot_target(g, w(0x01), 373);
    let r = g.call(s373, fr.ecx, &[]).eax;
    g.write::<u32>(w(0x0e), r);
    if r == 5 {
        g.write::<u32>(w(0x19), 1);
        return Ret::eax(0);
    }
    Ret::eax(1)
}

/// 0x10001030 (42 bytes, 1 overlay).
pub const E1030: Sig = Sig {
    len: 42,
    hash: 0x6ba7_7801_ec46_92a9,
    calls: &[],
    imports: &[],
};

/// A step (0x10001060, no arguments): slot 372. Returns 1.
pub fn e1060<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s372 = slot_target(g, img.word(fr.at + 0x01), 372);
    g.call(s372, fr.ecx, &[]);
    Ret::eax(1)
}

/// 0x10001060 (19 bytes, 1 overlay).
pub const E1060: Sig = Sig {
    len: 19,
    hash: 0xc42c_f38a_1ef7_8e85,
    calls: &[],
    imports: &[],
};

/// A step (0x10001080, no arguments): every fifth frame (`G_FRAME % 5
/// == 0`, operand `+0x02`) the counter `G_N` (`+0x1c`) goes up; it is
/// kept in `0..=7` (a negative one becomes 0). Returns 0 once it is 7,
/// else 1.
pub fn e1080<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let frame = g.read::<u32>(w(0x02)).cast_signed();
    let mut n = g.read::<u32>(w(0x1c)).cast_signed();
    if frame % 5 == 0 {
        n = n.wrapping_add(1);
    }
    if n < 0 {
        g.write::<u32>(w(0x29), 0);
        return Ret::eax(1);
    }
    let n = if n > 7 { 7 } else { n };
    g.write::<u32>(w(0x42), n.cast_unsigned());
    Ret::eax(u32::from(n != 7))
}

/// 0x10001080 (77 bytes, 1 overlay).
pub const E1080: Sig = Sig {
    len: 77,
    hash: 0x968d_5f76_3331_c8ee,
    calls: &[],
    imports: &[],
};

/// A step (0x100010d0, no arguments): the fade `G_V` (operand `+0x13`)
/// goes up by `D` towards `MAX` (`+0x0b`) while `G_DONE` (`+0x02`) is
/// clear, else down towards 0; slot 933 gets `(c, c, c, 1.0)` with `c =
/// MAX - G_V * S`. Returns 0 when `G_V` is `END` (`+0x99`), else 1.
pub fn e10d0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let max = k(g, 0x0b);
    let v = k(g, 0x13);
    let v = if g.read::<u32>(w(0x02)) == 0 {
        let v = v + k(g, 0x1d);
        g.write::<f32>(w(0x28), v);
        if v > max {
            g.write::<f32>(w(0x52), max);
            max
        } else {
            v
        }
    } else {
        let v = v - k(g, 0x37);
        g.write::<f32>(w(0x45), v);
        if 0.0 > v {
            g.write::<f32>(w(0x52), 0.0);
            0.0
        } else {
            v
        }
    };
    let c = (max - v * k(g, 0x5a)).to_bits();
    let s933 = slot_target(g, w(0x5f), 933);
    g.call(s933, fr.ecx, &[c, c, c, ONE]);
    // ucomiss + jp: 0 only when ordered and equal
    #[allow(clippy::float_cmp)]
    let differs = k(g, 0x8f) != k(g, 0x99);
    Ret::eax(u32::from(differs))
}

/// 0x100010d0 (172 bytes, 1 overlay).
pub const E10D0: Sig = Sig {
    len: 172,
    hash: 0x5f02_56d9_a603_ce92,
    calls: &[],
    imports: &[],
};

/// The model words an object callback sets: identity-ish transform
/// `+0x40..+0x5c` and the scale/colour `+0x60..+0x6c`.
fn model_words<G: Game>(g: &mut G, obj: u32, s0: u32) {
    for (o, v) in [
        (0x40u32, 0u32),
        (0x44, 0),
        (0x48, 0),
        (0x4c, ONE),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x60, s0),
        (0x64, 0x3e07_2325),
        (0x68, 0x3e07_2325),
        (0x6c, ONE),
    ] {
        g.write::<u32>(obj + o, v);
    }
}

/// An object callback (0x100013d0, `(obj)`): `*G_P = ITEM` (operands
/// `+0x05`, `+0x0b`), the part program `list[0]` of slot 941 `(G_D +
/// 0x10)` started on the object ([`start_part`] `(obj, 0x1000, program,
/// 1)`), then the model words. Returns what `start_part` leaves.
pub fn e13d0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let p = g.read::<u32>(w(0x04));
    g.write::<u32>(p, w(0x0b));
    let d = g.read::<u32>(g.read::<u32>(w(0x10)) + 0x10);
    let s941 = slot_target(g, w(0x18), 941);
    let list = g.call(s941, fr.ecx, &[d]).eax;
    let program = g.read::<u32>(list);
    let eax = start_part(g, img, img.rel32_target(f + 0x35), obj, 0x1000, program, 1);
    model_words(g, obj, 0x3e07_2325);
    Ret::eax(eax)
}

/// 0x100013d0 (147 bytes, 1 overlay).
pub const E13D0: Sig = Sig {
    len: 147,
    hash: 0xa91b_1c6b_7ee0_f564,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The locals of the camera callbacks: matrices `A`, `B`, `C` at `ebp -
/// 0xd4`, `- 0x94`, `- 0x54` and the vector `V` at `ebp - 0x14`.
struct Locals {
    a: u32,
    b: u32,
    c: u32,
    v: u32,
}

impl Locals {
    fn of(fr: &Frame) -> Locals {
        let ebp = fr.esp - 4;
        Locals {
            a: ebp - 0xd4,
            b: ebp - 0x94,
            c: ebp - 0x54,
            v: ebp - 0x14,
        }
    }
}

/// The camera of [`e1470`] and [`e17b0`]: slots 723 `(G_A)`, 724
/// `(G_B)`, 327 `(A, G_B)`, 230 `(C, A)`, 235 `(B)`, B's diagonal set to
/// `(b0, S, S)`, 231 `(C, C, B)`, each call with the words the earlier
/// ones left on the stack. `ops`: the operands (723's table, `G_A`,
/// 724's table, `G_B`, `G_B`, then the tables of 327, 230, 235, 231).
fn camera<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, l: &Locals, ops: [u32; 9], b0: u32) {
    let w = |off: u32| img.word(f + off);
    let (a, b, c) = (l.a, l.b, l.c);
    let s723 = slot_target(g, w(ops[0]), 723);
    let ga = g.read::<u32>(w(ops[1]));
    g.call(s723, ecx, &[ga]);
    let s724 = slot_target(g, w(ops[2]), 724);
    let gb = g.read::<u32>(w(ops[3]));
    g.call(s724, ecx, &[gb, ga]);
    let gb2 = g.read::<u32>(w(ops[4]));
    let s327 = slot_target(g, w(ops[5]), 327);
    // call width: 4 (two words and the earlier calls' two)
    g.call(s327, ecx, &[a, gb2, gb, ga]);
    let s230 = slot_target(g, w(ops[6]), 230);
    // call width: 6
    g.call(s230, ecx, &[c, a, a, gb2, gb, ga]);
    let s235 = slot_target(g, w(ops[7]), 235);
    // call width: 7
    g.call(s235, ecx, &[b, c, a, a, gb2, gb, ga]);
    let s231 = slot_target(g, w(ops[8]), 231);
    for (o, v) in [(0u32, b0), (0x14, 0x3e07_2325), (0x28, 0x3e07_2325)] {
        g.write::<u32>(b + o, v);
    }
    // call width: 10
    g.call(s231, ecx, &[c, c, b, b, c, a, a, gb2, gb, ga]);
}

/// The drift of the camera callbacks: past frame `after` of `G_T`
/// (`ops[1]`) `G_Y -= G_S`, `G_S += K` and `obj + 0x76 *= M`. Returns
/// `G_Y` (as read first, or moved). `ops`: `G_Y`, `G_T`, `G_S`, `K`, the
/// stores of `G_Y` and `G_S`, `M`.
fn drift<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, ops: [u32; 7], after: i32) -> f32 {
    let w = |off: u32| img.word(f + off);
    let mut y = g.read::<f32>(w(ops[0]));
    if g.read::<u32>(w(ops[1])).cast_signed() > after {
        let s = g.read::<f32>(w(ops[2]));
        let tilt = g.read::<u16>(obj + 0x76);
        y -= s;
        let s = s + g.read::<f32>(w(ops[3]));
        g.write::<f32>(w(ops[4]), y);
        g.write::<f32>(w(ops[5]), s);
        let m = f32::from(tilt) * g.read::<f32>(w(ops[6]));
        g.write::<u16>(obj + 0x76, cvttsd2si(f64::from(m)) as u16);
    }
    y
}

/// The end of the camera callbacks: slot 229 `(V, A, V)` added to C's
/// translation, slots 230 `(G_E, C)`, 231 `(G_EC, G_B, G_E)`, 230
/// `(G_DC, G_EC)` and slot 712 `(obj, 0x30)` with `*G_P = ITEM` and the
/// game flag `*G_LOCK` 1 around it (each call with the words the earlier
/// ones left); when that is nonzero, `*G_P = ITEM`, slot 713 `(obj)` and
/// 0, else 1. `ops`: 229's table; 230's, `G_E`; `G_E`, 231's, `G_B`,
/// `G_EC`; `G_EC`, 230's, `G_DC`; `G_P`, `ITEM`, `G_LOCK`, 712's,
/// `G_LOCK`; `G_P`, `ITEM`, 713's.
fn finish<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ecx: u32,
    obj: u32,
    l: &Locals,
    ops: [u32; 18],
) -> Ret {
    let w = |off: u32| img.word(f + off);
    let (a, c, v) = (l.a, l.c, l.v);
    let s229 = slot_target(g, w(ops[0]), 229);
    let mut stack = vec![v, a, v];
    g.call(s229, ecx, &stack);
    for i in 0..3u32 {
        let t = g.read::<f32>(c + 0x30 + 4 * i) + g.read::<f32>(v + 4 * i);
        g.write::<f32>(c + 0x30 + 4 * i, t);
    }
    let s230 = slot_target(g, w(ops[1]), 230);
    let ge = g.read::<u32>(w(ops[2]));
    stack.splice(0..0, [ge, c]);
    // call width: 5
    g.call(s230, ecx, &stack);
    let ge = g.read::<u32>(w(ops[3]));
    let s231 = slot_target(g, w(ops[4]), 231);
    let gb = g.read::<u32>(w(ops[5]));
    let gec = g.read::<u32>(w(ops[6]));
    stack.splice(0..0, [gec, gb, ge]);
    // call width: 8
    g.call(s231, ecx, &stack);
    let gec = g.read::<u32>(w(ops[7]));
    let s230 = slot_target(g, w(ops[8]), 230);
    let gdc = g.read::<u32>(w(ops[9]));
    stack.splice(0..0, [gdc, gec]);
    // call width: 10
    g.call(s230, ecx, &stack);
    let p = g.read::<u32>(w(ops[10]));
    g.write::<u32>(p, w(ops[11]));
    let lock = g.read::<u32>(w(ops[12]));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s712 = slot_target(g, w(ops[13]), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 12
    let r = g.call(s712, ecx, &stack).eax;
    let lock = g.read::<u32>(w(ops[14]));
    g.write::<u32>(lock, saved);
    if r == 0 {
        return Ret::eax(1);
    }
    let p = g.read::<u32>(w(ops[15]));
    g.write::<u32>(p, w(ops[16]));
    let s713 = slot_target(g, w(ops[17]), 713);
    g.call(s713, lock, &[obj]);
    Ret::eax(0)
}

/// An object callback (0x10001470, `(obj)`): the [`camera`] (B's
/// diagonal all `S`), the [`drift`] past frame 0x4b, `V = (0, 0, G_Y,
/// 0)` and the [`finish`].
pub fn e1470<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let l = Locals::of(fr);
    let ops = [0x14, 0x1b, 0x2b, 0x31, 0x3f, 0x4b, 0x63, 0x77, 0x90];
    camera(g, img, f, fr.ecx, &l, ops, 0x3e07_2325);
    let y = drift(
        g,
        img,
        f,
        obj,
        [0xb8, 0xc1, 0xcc, 0xdc, 0xe4, 0xec, 0xfb],
        0x4b,
    );
    for (o, x) in [(0u32, 0u32), (4, 0), (8, y.to_bits()), (0xc, 0)] {
        g.write::<u32>(l.v + o, x);
    }
    // (the saved esi below slot 229's words is not an argument)
    let ops = [
        0x118, 0x14d, 0x153, 0x184, 0x189, 0x18f, 0x19b, 0x1a3, 0x1a8, 0x1ae, 0x1bb, 0x1c3, 0x1c8,
        0x1d6, 0x1e4, 0x1f3, 0x1fa, 0x1ff,
    ];
    finish(g, img, f, fr.ecx, obj, &l, ops)
}

/// 0x10001470 (563 bytes, 1 overlay).
pub const E1470: Sig = Sig {
    len: 563,
    hash: 0x52d0_7ba3_f966_bd81,
    // __security_check_cookie
    calls: &[(0x217, Callee::Any), (0x22b, Callee::Any)],
    imports: &[],
};

/// An object callback (0x100017b0, `(obj)`): the [`camera`] (B's
/// diagonal `(S0, S, S)`), then by the object's index in the block at
/// `BASE` (operand `+0x2d`, 0x80 bytes each): 0 sets `obj + 0x76 =
/// 0x800` and `V = (0, 0, 72.0, 0)`; 1 the [`drift`] past frame 0x73 and
/// `V = (0, 0, G_Y, 0)`; any other index leaves `V` unset (the original
/// hands slot 229 whatever the stack held there; the port writes 0, the
/// junk-read rule). Then the [`finish`].
pub fn e17b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let index = obj.wrapping_sub(img.word(f + 0x2d)).cast_signed() >> 7;
    let l = Locals::of(fr);
    let ops = [0x14, 0x25, 0x37, 0x3d, 0x4b, 0x57, 0x6f, 0x83, 0x9c];
    camera(g, img, f, fr.ecx, &l, ops, 0x3e34_2edc);
    let y = match index {
        0 => {
            g.write::<u16>(obj + 0x76, 0x800);
            0x4290_0000
        }
        1 => drift(
            g,
            img,
            f,
            obj,
            [0xe9, 0xe0, 0xf3, 0x103, 0x10b, 0x113, 0x122],
            0x73,
        )
        .to_bits(),
        // never written in the original
        _ => 0,
    };
    for (o, x) in [(0u32, 0u32), (4, 0), (8, y), (0xc, 0)] {
        g.write::<u32>(l.v + o, x);
    }
    let ops = [
        0x158, 0x173, 0x179, 0x1aa, 0x1af, 0x1b5, 0x1c1, 0x1c9, 0x1ce, 0x1d4, 0x1e1, 0x1e9, 0x1ee,
        0x1fc, 0x20a, 0x218, 0x21f, 0x224,
    ];
    finish(g, img, f, fr.ecx, obj, &l, ops)
}

/// 0x100017b0 (602 bytes, 1 overlay).
pub const E17B0: Sig = Sig {
    len: 602,
    hash: 0xd4c0_a554_8aeb_af62,
    // __security_check_cookie
    calls: &[(0x23d, Callee::Any), (0x252, Callee::Any)],
    imports: &[],
};

/// An object callback (0x100016e0, `(obj)`): the object's index in the
/// block at `BASE` (operand `+0x0a`, 0x80 bytes each) picks the part
/// program: `list[1]` for 0, `list[0]` for 1 (`list` from slot 941
/// `(*G_D + 0x10)`, with `*G_P = ITEM`), started on the object
/// ([`start_part`] `(obj, 0x1000, program, 1)`); any other index starts
/// nothing. Then the model words. Returns what `start_part` leaves, or
/// the index.
pub fn e16e0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let index = obj.wrapping_sub(w(0x0a)).cast_signed() >> 7;
    let mut eax = index.cast_unsigned();
    // (G_P, ITEM, G_D, table) operands and the list entry per index
    let pick = match index {
        0 => Some((0x16u32, 0x1cu32, 0x21u32, 0x29u32, 4u32)),
        1 => Some((0x44, 0x4a, 0x4f, 0x57, 0)),
        _ => None,
    };
    if let Some((gp, item, gd, tab, entry)) = pick {
        let p = g.read::<u32>(w(gp));
        g.write::<u32>(p, w(item));
        let d = g.read::<u32>(g.read::<u32>(w(gd)) + 0x10);
        let s941 = slot_target(g, w(tab), 941);
        let list = g.call(s941, fr.ecx, &[d]).eax;
        let program = g.read::<u32>(list.wrapping_add(entry));
        eax = start_part(g, img, img.rel32_target(f + 0x71), obj, 0x1000, program, 1);
    }
    model_words(g, obj, 0x3e34_2edc);
    Ret::eax(eax)
}

/// 0x100016e0 (207 bytes, 1 overlay).
pub const E16E0: Sig = Sig {
    len: 207,
    hash: 0x4f81_9303_5256_2e0b,
    calls: &[(0x71, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// A step (0x10001d90, no arguments): slot 846, slot 933 `(1.0, 1.0,
/// 1.0, 1.0)`, slot 804 `(G_X)` (the four words left below). Returns 1.
pub fn e1d90<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s846 = slot_target(g, w(0x01), 846);
    g.call(s846, fr.ecx, &[]);
    let s933 = slot_target(g, w(0x0e), 933);
    g.call(s933, fr.ecx, &[ONE; 4]);
    let s804 = slot_target(g, w(0x3d), 804);
    let x = g.read::<u32>(w(0x43));
    // call width: 5 (the word and slot 933's four)
    g.call(s804, fr.ecx, &[x, ONE, ONE, ONE, ONE]);
    Ret::eax(1)
}

/// 0x10001d90 (88 bytes, 1 overlay).
pub const E1D90: Sig = Sig {
    len: 88,
    hash: 0x3474_eddb_56a0_1e11,
    calls: &[],
    imports: &[],
};

/// The start-up step (0x10001a10, `(arg)`): keeps `arg` (`G_ARG`,
/// operand `+0x07`); slot 803 `(0x1000000, 0x10)` gives a buffer cut in
/// three (`G_BUF0..2`, `+0x21`, `+0x2b`, `+0x35`); the effect `NAME`
/// (`+0x3f`, also in `G_NAME`) registered (slots 718, 221 `(NAME, 0,
/// 1000, 0x2000)`), its entries 6 and 8 (slot 719, the latter `G_D`),
/// the particle program `pppInitData(G_D, ENV, 0)` (slot 711, the game
/// flag `*G_LOCK` 1 around it) and slot 710 `(ITEM, G_D, G_BUF0,
/// 0x80000)`; three parts' lists (slot 941 of `G_D + 0x14..0x1c`); four
/// 0x110-byte state blocks cleared and given their defaults; slot 368
/// `(G_ARG, &COUNT)` (a byte list, `G_LIST`) with each entry's slot 215
/// handle in `HANDLES` (`+0x295`); slots 219 `(list[0], 0/1)`, 370
/// `(0/1)`, 367 `(G_ARG, list[0], 0, 0, &arg, 0, 0)` (which may rewrite
/// `arg` on the stack) and 369 `(list[0])`; `G_FLAG` is 1 when `arg`'s
/// or 369's bit 0 is set. Each call gets the words the earlier ones left
/// on the stack. Returns 1.
#[allow(clippy::too_many_lines)]
pub fn e1a10<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ecx = fr.ecx;
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x07), arg);
    let s803 = slot_target(g, w(0x0c), 803);
    let buf = g.call(s803, ecx, &[0x0100_0000, 0x10]).eax;
    g.write::<u32>(w(0x21), buf);
    g.write::<u32>(w(0x2b), buf.wrapping_add(0x8_0000));
    g.write::<u32>(w(0x35), buf.wrapping_add(0x10_0000));
    let name = w(0x3f);
    let s718 = slot_target(g, w(0x3a), 718);
    g.write::<u32>(w(0x4b), w(0x4f));
    let mut stack = vec![name, 0x0100_0000, 0x10];
    // call width: 3 (the name and slot 803's two words)
    g.call(s718, ecx, &stack);
    let s221 = slot_target(g, w(0x56), 221);
    let gn = g.read::<u32>(w(0x6e));
    stack.splice(0..0, [gn, 0, 0x3e8, 0x2000]);
    // call width: 7
    g.call(s221, ecx, &stack);
    let s719 = slot_target(g, w(0x75), 719);
    let gn = g.read::<u32>(w(0x85));
    stack.splice(0..0, [gn, 6, 0]);
    // call width: 10
    let r6 = g.call(s719, ecx, &stack).eax;
    g.write::<u32>(w(0x8c), r6);
    let s719 = slot_target(g, w(0x91), 719);
    let gn = g.read::<u32>(w(0xa1));
    stack.splice(0..0, [gn, 8, 0]);
    // call width: 13
    let d = g.call(s719, ecx, &stack).eax;
    let lock = g.read::<u32>(w(0xa9));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xb2), d);
    g.write::<u32>(lock, 1);
    let s711 = slot_target(g, w(0xbd), 711);
    stack.splice(0..0, [d, w(0xb7), 0]);
    // call width: 16
    g.call(s711, lock, &stack);
    let lock = g.read::<u32>(w(0xd0));
    g.write::<u32>(lock, saved);
    // add esp, 0x40
    let s710 = slot_target(g, w(0xda), 710);
    let buf0 = g.read::<u32>(w(0xe5));
    let gd = g.read::<u32>(w(0xf1));
    let mut stack = vec![w(0xf6), gd, buf0, 0x8_0000];
    g.call(s710, lock, &stack);
    for (k, (tab, dst)) in (0u32..).zip([(0x105u32, 0x112u32), (0x11f, 0x12c), (0x139, 0x152)]) {
        let gd = g.read::<u32>(w(0xfd + 0x1a * k));
        let x = g.read::<u32>(gd + 0x14 + 4 * k);
        let s941 = slot_target(g, w(tab), 941);
        stack.insert(0, x);
        // call width: 5..7
        let r = g.call(s941, lock, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    // memset(BLOCK, 0, 0x110) and each block's defaults
    let zero = [0u8; 0x110];
    g.write_bytes(w(0x14d), &zero);
    g.write::<u32>(w(0x15d), w(0x161));
    for o in [0x173u32, 0x17d, 0x187] {
        g.write::<u32>(w(o), 1);
    }
    g.write_bytes(w(0x16d), &zero);
    g.write::<u32>(w(0x1a2), w(0x1a6));
    for o in [0x1ac, 0x1b6, 0x1c0] {
        g.write::<u32>(w(o), 1);
    }
    g.write_bytes(w(0x19c), &zero);
    g.write::<u32>(w(0x1d2), w(0x1d6));
    for (o, v) in [
        (0x1e8u32, 3u32),
        (0x1f2, 3),
        (0x1fc, 1),
        (0x206, 1),
        (0x210, 1),
    ] {
        g.write::<u32>(w(o), v);
    }
    g.write_bytes(w(0x1e2), &zero);
    let s368 = slot_target(g, w(0x21e), 368);
    let count = w(0x223);
    let garg = g.read::<u32>(w(0x229));
    g.write::<u32>(w(0x22f), w(0x233));
    for o in [0x239u32, 0x243, 0x24d] {
        g.write::<u32>(w(o), 1);
    }
    g.write::<u32>(w(0x25d), 0);
    // call width: 5 (the two words and the last memset's three)
    let list = g.call(s368, lock, &[garg, count, w(0x1e2), 0, 0x110]).eax;
    g.write::<u32>(w(0x26d), list);
    let mut p = list;
    if g.read::<u32>(w(0x273)).cast_signed() > 0 {
        let mut i = 0u32;
        loop {
            let b = u32::from(g.read::<u8>(p.wrapping_add(i)));
            let s215 = slot_target(g, w(0x286), 215);
            let h = g.call(s215, lock, &[b]).eax;
            g.write::<u32>(w(0x295).wrapping_add(4 * i), h);
            p = g.read::<u32>(w(0x29a));
            i += 1;
            if i.cast_signed() >= g.read::<u32>(w(0x2a4)).cast_signed() {
                break;
            }
        }
    }
    let b0 = u32::from(g.read::<u8>(p));
    let s219 = slot_target(g, w(0x2b1), 219);
    let mut stack = vec![b0, 0];
    let r = g.call(s219, lock, &stack).eax;
    g.write::<u32>(w(0x2be), r);
    let b1 = u32::from(g.read::<u8>(g.read::<u32>(w(0x2c3))));
    let s219 = slot_target(g, w(0x2ce), 219);
    stack.splice(0..0, [b1, 1]);
    // call width: 4
    let r = g.call(s219, lock, &stack).eax;
    g.write::<u32>(w(0x2db), r);
    let s370 = slot_target(g, w(0x2e0), 370);
    stack.insert(0, 0);
    // call width: 5
    let r = g.call(s370, lock, &stack).eax;
    g.write::<u32>(w(0x2ef), r);
    let s370 = slot_target(g, w(0x2f4), 370);
    stack.insert(0, 1);
    // call width: 6
    let r = g.call(s370, lock, &stack).eax;
    g.write::<u32>(w(0x307), r);
    let b2 = u32::from(g.read::<u8>(g.read::<u32>(w(0x312))));
    let s367 = slot_target(g, w(0x31f), 367);
    let garg = g.read::<u32>(w(0x325));
    let at_arg = fr.esp + 4;
    stack.splice(0..0, [garg, b2, 0, 0, at_arg, 0, 0, 0]);
    // call width: 14
    g.call(s367, lock, &stack);
    let b3 = u32::from(g.read::<u8>(g.read::<u32>(w(0x332))));
    let arg = g.read::<u32>(at_arg);
    let s369 = slot_target(g, w(0x33e), 369);
    stack.insert(0, b3);
    // call width: 15
    let r = g.call(s369, lock, &stack).eax;
    if arg & 1 == 0 {
        g.write::<u32>(w(0x358), 0);
        if r & 1 == 0 {
            return Ret::eax(1);
        }
    }
    g.write::<u32>(w(0x366), 1);
    Ret::eax(1)
}

/// 0x10001a10 (885 bytes, 1 overlay).
pub const E1A10: Sig = Sig {
    len: 885,
    hash: 0x7a1d_2294_b6ec_ab3b,
    calls: &[
        (0x157, Callee::Import("memset")),
        (0x190, Callee::Import("memset")),
        (0x1c9, Callee::Import("memset")),
        (0x219, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The per-frame step (0x10001e00, no arguments): slot 253 0 returns 0.
/// Then the waits, each once: slot 225 `(0x2000)` (nonzero waits),
/// slot 226 `(G_ARG, 1)`, slot 227 `(G_ARG)` (below 1 waits), and the
/// start-up's `G_FLAG`. The first frame after (`G_RUN` 0) installs the
/// draw callback (slot 377 `(CB, 0, 0, 0)`, its handle kept), slot 381
/// `(1)`, shows every handle (slot 276 `(h, 1)`), slot 366 `(list[0],
/// VEC)`, the two floats of slots 365 and 364 `(HANDLES[0])` (`G_X`,
/// `G_Z`), slot 210 `(HANDLES[0], &n8)`, slot 325 `(VEC, &m)`, 232
/// `(VEC, VEC)`, `VEC *= K`, slot 288 `(0x10, &n)` (each byte through slot
/// 296 `(b, 0)`) and slot 368 `(G_ARG, &n0)` (each byte through slot
/// 296 `(b, 1)`), then `G_RUN++`. While `G_RUN` is set the distance
/// `sqrt(G_Z^2 + G_X^2) * S` goes to `G_DIST`. Returns 1. The counts
/// are locals of the aligned frame (`(ebp & !7) - 0x50 + 0`, `+ 4`).
#[allow(clippy::too_many_lines)]
pub fn e1e00<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ecx = fr.ecx;
    // and esp, -8; sub esp, 0x50
    let s = ((fr.esp - 4) & !7) - 0x50;
    let s253 = slot_target(g, w(0x15), 253);
    let r = g.call(s253, ecx, &[]).eax;
    if r == 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x3a)) == 0 {
        let s225 = slot_target(g, w(0x42), 225);
        if g.call(s225, ecx, &[0x2000]).eax != 0 {
            return Ret::eax(1);
        }
        let n = g.read::<u32>(w(0x60));
        g.write::<u32>(w(0x60), n.wrapping_add(1));
    }
    if g.read::<u32>(w(0x66)) == 0 {
        let s226 = slot_target(g, w(0x6e), 226);
        let garg = g.read::<u32>(w(0x76));
        g.write::<u32>(w(0x82), 1);
        g.call(s226, ecx, &[garg, 1]);
    }
    if g.read::<u32>(w(0x91)) == 0 {
        let s227 = slot_target(g, w(0x99), 227);
        let garg = g.read::<u32>(w(0x9f));
        if g.call(s227, ecx, &[garg]).eax.cast_signed() < 1 {
            return Ret::eax(1);
        }
        let n = g.read::<u32>(w(0xb9));
        g.write::<u32>(w(0xb9), n.wrapping_add(1));
    }
    if g.read::<u32>(w(0xbf)) == 0 {
        return Ret::eax(1);
    }
    if g.read::<u32>(w(0xcc)) == 0 {
        first_frame(g, img, f, ecx, s);
        let n = g.read::<u32>(w(0x2c0)).wrapping_add(1);
        g.write::<u32>(w(0x2c0), n);
        if n == 0 {
            return Ret::eax(1);
        }
    }
    let x = g.read::<f32>(w(0x2ca));
    let z = g.read::<f32>(w(0x2d2));
    let d = z * z + x * x;
    let d = f64::from(d).sqrt() as f32 * g.read::<f32>(w(0x2f2));
    g.write::<f32>(w(0x2fa), d);
    Ret::eax(1)
}

/// The first running frame of [`e1e00`] (from `+0xd7`; `s` the aligned
/// frame's `esp` after `sub esp, 0x50`).
fn first_frame<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, s: u32) {
    let w = |off: u32| img.word(f + off);
    let s377 = slot_target(g, w(0xd8), 377);
    let h = g.call(s377, ecx, &[w(0xe9), 0, 0, 0]).eax;
    g.write::<u32>(w(0xf0), h);
    let s381 = slot_target(g, w(0xf5), 381);
    // call width: 5 (the word and slot 377's four)
    g.call(s381, ecx, &[1, w(0xe9), 0, 0, 0]);
    if g.read::<u32>(w(0x10a)).cast_signed() > 0 {
        let mut i = 0u32;
        loop {
            let s276 = slot_target(g, w(0x111), 276);
            let h = g.read::<u32>(w(0x11a).wrapping_add(4 * i));
            g.call(s276, ecx, &[h, 1]);
            i += 1;
            if i.cast_signed() >= g.read::<u32>(w(0x12c)).cast_signed() {
                break;
            }
        }
    }
    let mut p = w(0x133);
    while p.cast_signed() < w(0x155).cast_signed() {
        let h = g.read::<u32>(p);
        if h != 0 {
            let s276 = slot_target(g, w(0x141), 276);
            g.call(s276, ecx, &[h, 1]);
        }
        p = p.wrapping_add(4);
    }
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x15c))));
    let vec = w(0x161);
    let s366 = slot_target(g, w(0x16a), 366);
    let mut stack = vec![b, vec];
    g.call(s366, ecx, &stack);
    let s365 = slot_target(g, w(0x177), 365);
    let h0 = g.read::<u32>(w(0x17d));
    stack.insert(0, h0);
    // call width: 3
    let x = crate::m0018::st0_f32(g.call(s365, ecx, &stack).st0);
    g.write::<f32>(w(0x18b), x);
    let s364 = slot_target(g, w(0x190), 364);
    let h0 = g.read::<u32>(w(0x196));
    stack.insert(0, h0);
    // call width: 4
    let z = crate::m0018::st0_f32(g.call(s364, ecx, &stack).st0);
    g.write::<f32>(w(0x1a4), z);
    let s210 = slot_target(g, w(0x1ae), 210);
    let h0 = g.read::<u32>(w(0x1b4));
    stack.splice(0..0, [h0, s + 8]);
    // call width: 6
    g.call(s210, ecx, &stack);
    let s325 = slot_target(g, w(0x1c6), 325);
    let vec2 = w(0x1cb);
    stack.splice(0..0, [vec2, s + 0x28]);
    // call width: 8
    g.call(s325, ecx, &stack);
    let s232 = slot_target(g, w(0x1d8), 232);
    stack.splice(0..0, [w(0x1e8), w(0x1dd)]);
    // call width: 10
    g.call(s232, ecx, &stack);
    let k = g.read::<f32>(w(0x1fa));
    for (ld, st) in [(0x1f2u32, 0x210u32), (0x218, 0x226), (0x22e, 0x23a)] {
        let v = g.read::<f32>(w(ld)) * k;
        g.write::<f32>(w(st), v);
    }
    let s288 = slot_target(g, w(0x208), 288);
    stack.splice(0..0, [0x10, s + 4]);
    // call width: 12
    let mut p = g.call(s288, ecx, &stack).eax;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(s + 4).cast_signed() {
        let b = u32::from(g.read::<u8>(p));
        let s296 = slot_target(g, w(0x257), 296);
        g.call(s296, ecx, &[b, 0]);
        i += 1;
        p = p.wrapping_add(1);
    }
    let s368 = slot_target(g, w(0x279), 368);
    let garg = g.read::<u32>(w(0x27f));
    let q = g.call(s368, ecx, &[garg, s]).eax;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(s).cast_signed() {
        let b = u32::from(g.read::<u8>(q.wrapping_add(i)));
        let s296 = slot_target(g, w(0x2a5), 296);
        g.call(s296, ecx, &[b, 1]);
        i += 1;
    }
}

/// 0x10001e00 (788 bytes, 1 overlay).
pub const E1E00: Sig = Sig {
    len: 788,
    hash: 0x2aec_296d_cfbf_d407,
    calls: &[
        // __security_check_cookie
        (0x30, Callee::Any),
        (0x2e6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x30c, Callee::Any),
    ],
    imports: &[],
};

/// Sets up the bars' sprite record (0x10002b00, internal: `ecx` = the
/// record, floats `a`, `b`, `c` in `xmm1`..`xmm3` and `s0`, `s1` on the
/// stack): screen positions from the values (`x * K * W - O + H`, each
/// truncated the x87 way into `+0xc8`, `+0xcc`, `+0xe8`, `+0xec`; `a * A`
/// into `+0xc4`), the fixed words, then slots 821 `(0)`, 822 `(1.0, 1.0,
/// 1.0, a)`, 823 `(1.0, 1.0)` and 824 with the centre of `(b + s0, c +
/// s1)` scaled (`(v * h - E) * h + h`).
#[allow(clippy::too_many_arguments)]
pub fn bars<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    p: u32,
    a: f32,
    b: f32,
    c: f32,
    s0: f32,
    s1: f32,
) {
    use ffx_bridge::x87::ftol2;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let (k10, k20) = (k(g, 0x0a), k(g, 0x12));
    let t0 = b * k(g, 0x1d) * k10 - k(g, 0x39) + k20;
    for (o, v) in [
        (0xa8u32, 0u32),
        (0xac, 0),
        (0xb0, ONE),
        (0xd8, ONE),
        (0xdc, 0x3f50_0000),
        (0xe0, ONE),
        (0xd0, 0x00ff_ffff),
    ] {
        g.write::<u32>(p + o, v);
    }
    let t1 = c * k(g, 0x98) * k10 - k(g, 0xcd) + k20;
    g.write::<u32>(p + 0xb4, 0);
    g.write::<u32>(p + 0xf0, 0x00ff_ffff);
    g.write::<u32>(p + 0xe4, 0);
    g.write::<u32>(p + 0xc8, ftol2(f64::from(t0)));
    let t2 = s0 * k(g, 0xfc) * k10 - k(g, 0x112) + k20;
    g.write::<u32>(p + 0xcc, ftol2(f64::from(t1)));
    let t3 = s1 * k(g, 0x139) * k10 - k(g, 0x151) + k20;
    g.write::<u32>(p + 0xe8, ftol2(f64::from(t2)));
    g.write::<u32>(p + 0xec, ftol2(f64::from(t3)));
    let t4 = a * k(g, 0x1ad);
    for o in [0xb8u32, 0xbc, 0xc0] {
        g.write::<u32>(p + o, 0x80);
    }
    g.write::<u32>(p + 0xc4, ftol2(f64::from(t4)));
    let s821 = slot_target(g, w(0x1f5), 821);
    g.call(s821, p, &[0]);
    let s822 = slot_target(g, w(0x20a), 822);
    g.call(s822, p, &[ONE, ONE, ONE, a.to_bits()]);
    let s823 = slot_target(g, w(0x236), 823);
    g.call(s823, p, &[ONE, ONE]);
    let h = k(g, 0x262);
    let u = ((c + s1) * h - k(g, 0x28b)) * h + h;
    let v = ((b + s0) * h - k(g, 0x2af)) * h + h;
    let s824 = slot_target(g, w(0x276), 824);
    g.call(s824, p, &[v.to_bits(), u.to_bits(), 0, 0]);
}

/// [`bars`] (713 bytes, 1 overlay).
pub const BARS: Sig = Sig {
    len: 713,
    hash: 0xf740_a821_6694_156f,
    calls: &[],
    imports: &[],
};
