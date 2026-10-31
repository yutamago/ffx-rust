//! `magic_0020.dll`'s own functions (besides the runtime, the object
//! host and the shared code): the step table's entries and the object
//! callbacks.

use ffx_bridge::x87::cvttsd2si;

use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
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
        let v = add(v, k(g, 0x1d));
        g.write::<f32>(w(0x28), v);
        if v > max {
            g.write::<f32>(w(0x52), max);
            max
        } else {
            v
        }
    } else {
        let v = sub(v, k(g, 0x37));
        g.write::<f32>(w(0x45), v);
        if 0.0 > v {
            g.write::<f32>(w(0x52), 0.0);
            0.0
        } else {
            v
        }
    };
    let c = sub(max, mul(v, k(g, 0x5a))).to_bits();
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
        y = sub(y, s);
        let s = add(s, g.read::<f32>(w(ops[3])));
        g.write::<f32>(w(ops[4]), y);
        g.write::<f32>(w(ops[5]), s);
        let m = mul(f32::from(tilt), g.read::<f32>(w(ops[6])));
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
        let t = add(g.read::<f32>(c + 0x30 + 4 * i), g.read::<f32>(v + 4 * i));
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
    // movss xmm1, [x]; movss xmm0, [z]; squares; addss xmm1, xmm0
    let d = add(mul(x, x), mul(z, z));
    let d = mul(f64::from(d).sqrt() as f32, g.read::<f32>(w(0x2f2)));
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
        let v = mul(g.read::<f32>(w(ld)), k);
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
    // x * K * W - O + H, each step its own rounding
    let pos = |g: &G, x: f32, kk: u32, o: u32| add(sub(mul(mul(x, k(g, kk)), k10), k(g, o)), k20);
    let t0 = pos(g, b, 0x1d, 0x39);
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
    let t1 = pos(g, c, 0x98, 0xcd);
    g.write::<u32>(p + 0xb4, 0);
    g.write::<u32>(p + 0xf0, 0x00ff_ffff);
    g.write::<u32>(p + 0xe4, 0);
    g.write::<u32>(p + 0xc8, ftol2(f64::from(t0)));
    let t2 = pos(g, s0, 0xfc, 0x112);
    g.write::<u32>(p + 0xcc, ftol2(f64::from(t1)));
    let t3 = pos(g, s1, 0x139, 0x151);
    g.write::<u32>(p + 0xe8, ftol2(f64::from(t2)));
    g.write::<u32>(p + 0xec, ftol2(f64::from(t3)));
    let t4 = mul(a, k(g, 0x1ad));
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
    let u = add(mul(sub(mul(add(c, s1), h), k(g, 0x28b)), h), h);
    let v = add(mul(sub(mul(add(b, s0), h), k(g, 0x2af)), h), h);
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

/// The fade and its frame (0x10001180, no arguments): the level `*L`
/// (operand `+0x13`) falls by `D` (`+0x40`) to 0 while `*C` (`+0x02`) is
/// set, else rises by `D` (`+0x26`) to `K` (`+0x0b`) once `*T` (`+0x1b`)
/// is past 5 (each step stored, the limit too). [`bars`] of the record
/// `+0xc0` with `a = L / K3` (`+0x63`) and the vector `*V` (`+0x83`,
/// `+0x7b`, `+0xab`, `+0x73`) scaled by `K4` (`+0x6b`) and moved by `K`
/// (`b`, `c` minus, `s0`, `s1` plus). In state 7 (`*S`, `+0x8e`, read
/// before [`bars`]) slot 845. Then with `*C` (`+0xe5`) clear, returns 1;
/// else slot 276 `(h, 0)` for each of the `*N` (`+0xf0`) handles at
/// `+0x10a` and each non-zero word from `+0x123` to `+0x145`, sets `*E`
/// (`+0x14d`) and returns 0.
///
/// The census counts the callback after it ([`fade_end`], 0x100012e0)
/// as part of this function; nothing jumps there from here.
pub fn fade_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::div;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fl = |g: &G, off: u32| g.read::<f32>(w(off));
    let k = fl(g, 0x0b);
    let mut x = fl(g, 0x13);
    if g.read::<u32>(w(0x02)) != 0 {
        x = sub(x, fl(g, 0x40));
        g.write::<u32>(w(0x4e), x.to_bits());
        if 0.0 > x {
            x = 0.0;
            g.write::<u32>(w(0x5b), x.to_bits());
        }
    } else if g.read::<i32>(w(0x1b)) > 5 {
        x = add(x, fl(g, 0x26));
        g.write::<u32>(w(0x31), x.to_bits());
        if x > k {
            x = k;
            g.write::<u32>(w(0x5b), x.to_bits());
        }
    }
    let a = div(x, fl(g, 0x63));
    let k4 = fl(g, 0x6b);
    let s1 = add(mul(fl(g, 0x73), k4), k);
    let c = sub(mul(fl(g, 0x7b), k4), k);
    let b = sub(mul(fl(g, 0x83), k4), k);
    let state = g.read::<u32>(w(0x8e));
    let s0 = add(mul(fl(g, 0xab), k4), k);
    bars(g, img, img.rel32_target(f + 0xca), w(0xc0), a, b, c, s0, s1);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    if state == 7 {
        let s = table(g, 0xd7, 0xd34 / 4);
        g.call(s, 0, &[]);
    }
    if g.read::<u32>(w(0xe5)) == 0 {
        return Ret::eax(1);
    }
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0xf0)) {
        let h = g.read::<u32>(w(0x10a).wrapping_add(4 * i));
        let s = table(g, 0x101, 0x450 / 4);
        g.call(s, 0, &[h, 0]);
        i += 1;
    }
    let mut p = w(0x123);
    while p < w(0x145) {
        let h = g.read::<u32>(p);
        if h != 0 {
            let s = table(g, 0x131, 0x450 / 4);
            g.call(s, 0, &[h, 0]);
        }
        p += 4;
    }
    g.write::<u32>(w(0x14d), 1);
    Ret::eax(0)
}

/// [`fade_frame`] (352 bytes, 1 overlay).
pub const FADE_FRAME: Sig = Sig {
    len: 352,
    hash: 0x5501_c87f_87c8_11b0,
    calls: &[(0xca, Callee::Sig(&BARS))],
    imports: &[],
};

/// The fade's end (0x100012e0, a callback with no arguments, pushed by
/// [`e1e00`]): `**P = *Q` (`+0x02`, `+0x07`), slots 382 `(0, 0)` and 371
/// `(0, 0)`, `(1, 0)`, the timeline `+0x44` runs
/// ([`timeline_run`](crate::sb_misc::timeline_run)); when the level
/// (`+0x51`) is exactly `K` (`+0x58`), slots 315 `(0)`, 222 `(*H, 0,
/// 1000, 0x2000)` (`+0x8e`), 378, 380 and 379 `(0x2000)` and the
/// timelines `+0xce`, `+0xd8` and `+0xe2` (a tail jump) run. Returns the
/// last timeline's answer. Each slot call passes the words the original
/// leaves on the stack.
pub fn fade_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let q = g.read::<u32>(w(0x07));
    let p = g.read::<u32>(w(0x02));
    g.write::<u32>(p, q);
    let s = table(g, 0x0e, 0x5f8 / 4);
    g.call(s, 0, &[0, 0]);
    let s = table(g, 0x1f, 0x5cc / 4);
    g.call(s, 0, &[0, 0, 0, 0]);
    let s = table(g, 0x30, 0x5cc / 4);
    g.call(s, 0, &[1, 0, 0, 0, 0, 0]);
    let run = img.rel32_target(f + 0x49);
    let r = crate::sb_misc::timeline_run(g, img, run, w(0x44));
    // ucomiss; lahf; test ah, 0x44; jp: on unless equal (NaN too), with
    // the flags in `ah` (bit 1 always set; CF below, PF and ZF unordered)
    let (x, k) = (g.read::<f32>(w(0x51)), g.read::<f32>(w(0x58)));
    let ah: u32 = match x.partial_cmp(&k) {
        Some(std::cmp::Ordering::Equal) => 0x42,
        Some(std::cmp::Ordering::Less) => 0x03,
        Some(std::cmp::Ordering::Greater) => 0x02,
        None => 0x47,
    };
    if ah != 0x42 {
        return Ret::eax((r & !0xff00) | (ah << 8));
    }
    let s = table(g, 0x67, 0x4ec / 4);
    g.call(s, 0, &[0]);
    let h = g.read::<u32>(w(0x8e));
    let s = table(g, 0x76, 0x378 / 4);
    g.call(s, 0, &[h, 0, 0x3e8, 0x2000, 0]);
    for (k, (at, slot)) in (0usize..).zip([(0x95, 0x5e8), (0xa7, 0x5f0), (0xb9, 0x5ec)]) {
        let s = table(g, at, slot / 4);
        let mut args = vec![0x2000; k + 1];
        args.extend_from_slice(&[h, 0, 0x3e8, 0x2000, 0]);
        args.truncate(6);
        // call width: 6 (truncated above)
        g.call(s, 0, &args);
    }
    for at in [0xce, 0xd8] {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + at + 5), w(at));
    }
    // jmp: the last timeline's answer is the caller's
    let r = crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0xe7), w(0xe2));
    Ret::eax(r)
}

/// [`fade_end`] (236 bytes, 1 overlay).
pub const FADE_END: Sig = Sig {
    len: 236,
    hash: 0x220d_7785_7432_2923,
    calls: &[
        (0x49, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xd3, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xdd, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xe7, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
    ],
    imports: &[],
};

/// The closing frame (0x10002120, no arguments): nothing (0) unless `*A`
/// (operand `+0x06`) is set; then `**P` = the immediate `+0x20`
/// (`P` at `+0x14`), slots 723 `(*M)` and 724 `(*N, *M)` (`+0x1a`,
/// `+0x38`), `**Q = *R` (`+0x46`, `+0x4b`), and by `*F` (`+0x54`, read
/// before that store):
/// - clear: slots 239 `(*H, -5, 0)` and 214 `(*H)` (`+0x6e`, `+0x7b`);
/// - set, `*E` (`+0x95`) clear: slot 220 `(*J, 0x40, 0x100)` (`+0x1cd`)
///   while the counter `*K` (`+0x1b1`) is 0, which counts up;
/// - set, `*E` set: the four timelines (`+0xa3`, `+0xad`, `+0xb7`,
///   `+0xc1`) stopped ([`timeline_stop`](crate::sb_misc::timeline_stop)),
///   slots 376 `(*S)` (`+0xd1`) and 381 `(0)`, slot 276 `(h, 0)` for the
///   `*N` (`+0xf3`) handles at `+0x10a` and the non-zero words from
///   `+0x123` to `+0x145`; slot 288 `(0x10, &n)` gives a byte list
///   (`n` a local holding the caller's `ecx` until the slot writes it:
///   `push ecx`), slot 296 `(byte, 1)` for each of its `n` bytes, and
///   slot 214 `(*H)` (`+0x196`).
///
/// Returns 1. Each slot call passes the words the original leaves on
/// the stack.
pub fn close_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    if g.read::<u32>(w(0x06)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x14));
    let m = g.read::<u32>(w(0x1a));
    g.write::<u32>(p, w(0x20));
    let s = table(g, 0x25, 0xb4c / 4);
    g.call(s, 0, &[m]);
    let n = g.read::<u32>(w(0x38));
    let s = table(g, 0x32, 0xb50 / 4);
    g.call(s, 0, &[n, m]);
    let q = g.read::<u32>(w(0x46));
    let r = g.read::<u32>(w(0x4b));
    let flag = g.read::<u32>(w(0x54));
    g.write::<u32>(q, r);
    if flag == 0 {
        let h = g.read::<u32>(w(0x6e));
        let s = table(g, 0x5e, 0x3bc / 4);
        g.call(s, 0, &[h, (-5i32).cast_unsigned(), 0]);
        let h2 = g.read::<u32>(w(0x7b));
        let s = table(g, 0x75, 0x358 / 4);
        g.call(s, 0, &[h2, h, (-5i32).cast_unsigned(), 0]);
        return Ret::eax(1);
    }
    if g.read::<u32>(w(0x95)) == 0 {
        let mut k = g.read::<u32>(w(0x1b1));
        if k == 0 {
            let j = g.read::<u32>(w(0x1cd));
            let s = table(g, 0x1ba, 0x370 / 4);
            g.call(s, 0, &[j, 0x40, 0x100]);
            k = g.read::<u32>(w(0x1d4));
        }
        g.write::<u32>(w(0x1dd), k.wrapping_add(1));
        return Ret::eax(1);
    }
    for at in [0xa3, 0xad, 0xb7, 0xc1] {
        crate::sb_misc::timeline_stop(g, w(at));
    }
    let sv = g.read::<u32>(w(0xd1));
    let s = table(g, 0xcb, 0x5e0 / 4);
    g.call(s, 0, &[sv]);
    let s = table(g, 0xde, 0x5f4 / 4);
    g.call(s, 0, &[0, sv]);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0xf3)) {
        let h = g.read::<u32>(w(0x10a).wrapping_add(4 * i));
        let s = table(g, 0x101, 0x450 / 4);
        g.call(s, 0, &[h, 0]);
        i += 1;
    }
    let mut p = w(0x123);
    while p < w(0x145) {
        let h = g.read::<u32>(p);
        if h != 0 {
            let s = table(g, 0x131, 0x450 / 4);
            g.call(s, 0, &[h, 0]);
        }
        p += 4;
    }
    // push ecx: the local at [ebp - 4]
    let local = fr.esp.wrapping_sub(8);
    g.write::<u32>(local, fr.ecx);
    let s = table(g, 0x150, 0x480 / 4);
    let mut list = g.call(s, 0, &[0x10, local]).eax;
    let mut k = 0i32;
    while k < g.read::<i32>(local) {
        let b = u32::from(g.read::<u8>(list));
        let s = table(g, 0x174, 0x4a0 / 4);
        g.call(s, 0, &[b, 1]);
        k += 1;
        list = list.wrapping_add(1);
    }
    let h = g.read::<u32>(w(0x196));
    let s = table(g, 0x190, 0x358 / 4);
    g.call(s, 0, &[h]);
    Ret::eax(1)
}

/// [`close_frame`] (490 bytes, 1 overlay).
pub const CLOSE_FRAME: Sig = Sig {
    len: 490,
    hash: 0xbe90_1951_d3fd_5958,
    calls: &[
        (0xa8, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xb2, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xbc, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xc6, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};
