//! `magic_0045.dll`'s own functions (besides the runtime, the object
//! host and the shared code): the object callbacks and the start-up.

use crate::objcb::{PartAt, imm_stores, part_start};
use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The part start of the callbacks without a frame of their own.
const PART: PartAt = PartAt {
    gp: 0x04,
    item: 0x0b,
    gd: 0x10,
    table: 0x18,
    entry: 0,
    call: 0x36,
};

/// An object callback (0x10001250, `(obj)`): part `list[13]` started on
/// the object, its model words, the scale `+0x60..` by the object's
/// index in the block at `BASE` (operand `+0x3d`): the first of two sets
/// for index 0, the second otherwise. Returns 1.
pub fn e1250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    part_start(
        g,
        img,
        f,
        fr.ecx,
        obj,
        PartAt {
            entry: 0x34,
            ..PART
        },
    );
    let stores = [
        (0x40, 0x4c),
        (0x44, 0x53),
        (0x48, 0x5a),
        (0x4c, 0x61),
        (0x50, 0x68),
        (0x54, 0x6f),
        (0x58, 0x76),
        (0x5c, 0x7d),
        (0x6c, 0x84),
    ];
    imm_stores(g, img, f, obj, &stores);
    // test eax, 0xffffff80: the first object of the block
    if obj.wrapping_sub(img.word(f + 0x3d)) & 0xffff_ff80 == 0 {
        imm_stores(g, img, f, obj, &[(0x60, 0x92), (0x64, 0x99), (0x68, 0xa0)]);
    } else {
        imm_stores(g, img, f, obj, &[(0x60, 0xaa), (0x64, 0xb1), (0x68, 0xb8)]);
    }
    Ret::eax(1)
}

/// 0x10001250 (191 bytes, 1 overlay).
pub const E1250: Sig = Sig {
    len: 191,
    hash: 0x2fbb_30e0_5d61_0614,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object callback (0x10001470, `(obj)`): part `list[2]` started on
/// the object and its model words. Returns 1.
pub fn e1470<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    part_start(g, img, f, fr.ecx, obj, PartAt { entry: 8, ..PART });
    let stores = [
        (0x40, 0x40),
        (0x44, 0x47),
        (0x48, 0x4e),
        (0x4c, 0x55),
        (0x50, 0x5c),
        (0x54, 0x63),
        (0x58, 0x6a),
        (0x5c, 0x71),
        (0x60, 0x78),
        (0x64, 0x7f),
        (0x68, 0x86),
        (0x6c, 0x8d),
    ];
    imm_stores(g, img, f, obj, &stores);
    Ret::eax(1)
}

/// 0x10001470 (153 bytes, 1 overlay).
pub const E1470: Sig = Sig {
    len: 153,
    hash: 0x5f76_2316_84c1_5a58,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The object callbacks at 0x100015a0 and 0x100018b0 (the same code
/// with other scale words): part `list[1]` started on the object, its
/// model words, then the object placed: slot 724 `(&A)`, 327 `(&B,
/// &A)`, 230 `(VEC, &B)`, 325 `(VEC2, obj + 0x40)` (locals `A`, `B` at
/// `ebp - 0x44`, `- 0x84`; each call with the words the earlier ones
/// left on the stack). Returns 1.
fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp - 4;
    let (a, b) = (ebp - 0x44, ebp - 0x84);
    let at = PartAt {
        gp: 0x14,
        item: 0x1e,
        gd: 0x23,
        table: 0x2c,
        entry: 4,
        call: 0x47,
    };
    let (_, left) = part_start(g, img, f, fr.ecx, obj, at);
    let stores = [
        (0x44, 0x5a),
        (0x40, 0x60),
        (0x48, 0x67),
        (0x4c, 0x6e),
        (0x50, 0x75),
        (0x54, 0x7c),
        (0x58, 0x83),
        (0x5c, 0x8a),
        (0x60, 0x91),
        (0x64, 0x98),
        (0x68, 0x9f),
        (0x6c, 0xa6),
    ];
    let s724 = slot_target(g, w(0x50), 724);
    imm_stores(g, img, f, obj, &stores);
    let mut stack = vec![a];
    stack.extend(left);
    // call width: 5 (the local and the part start's four words)
    g.call(s724, obj, &stack);
    let s327 = slot_target(g, w(0xbe), 327);
    stack.splice(0..0, [b, a]);
    // call width: 7
    g.call(s327, obj, &stack);
    let s230 = slot_target(g, w(0xd2), 230);
    stack.splice(0..0, [w(0xd7), b]);
    // call width: 9
    g.call(s230, obj, &stack);
    let s325 = slot_target(g, w(0xe4), 325);
    stack.splice(0..0, [w(0xf0), obj.wrapping_add(0x40)]);
    // call width: 11
    g.call(s325, obj, &stack);
    Ret::eax(1)
}

/// An object callback (0x100015a0, `(obj)`): see [`place`].
pub fn e15a0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    place(g, img, fr)
}

/// 0x100015a0 (270 bytes, 1 overlay).
pub const E15A0: Sig = Sig {
    len: 270,
    hash: 0xf89d_2547_7dec_167d,
    calls: &[
        (0x47, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x106, Callee::Any),
    ],
    imports: &[],
};

/// An object callback (0x100018b0, `(obj)`): see [`place`].
pub fn e18b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    place(g, img, fr)
}

/// 0x100018b0 (270 bytes, 1 overlay).
pub const E18B0: Sig = Sig {
    len: 270,
    hash: 0xc44f_91ab_1738_fa17,
    calls: &[
        (0x47, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x106, Callee::Any),
    ],
    imports: &[],
};

/// The end of the moving callbacks ([`e16b0`], [`e19c0`]): `*G_P = ITEM`,
/// slot 712 `(obj, 0x30)` with the game flag `*G_LOCK` 1 around it; when
/// that is nonzero, slot 713 `(obj)` and the part `list[1]` restarted
/// (with `*G_P = ITEM` again). `ops`: `G_P`, `ITEM`, `G_LOCK`, 712's
/// table, `G_LOCK`; `G_P`, `ITEM`, 713's table; `G_P`, `ITEM`, `G_D`,
/// 941's table, the `start_part` call. Returns 1.
fn done<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    mut stack: Vec<u32>,
    ops: [u32; 13],
) -> Ret {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(ops[0]));
    g.write::<u32>(p, w(ops[1]));
    let lock = g.read::<u32>(w(ops[2]));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s712 = slot_target(g, w(ops[3]), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 13 (e16b0: two words and eleven left; e19c0: seven)
    let r = g.call(s712, lock, &stack).eax;
    let lock = g.read::<u32>(w(ops[4]));
    g.write::<u32>(lock, saved);
    if r == 0 {
        return Ret::eax(1);
    }
    let p = g.read::<u32>(w(ops[5]));
    g.write::<u32>(p, w(ops[6]));
    let s713 = slot_target(g, w(ops[7]), 713);
    g.call(s713, lock, &[obj]);
    let p = g.read::<u32>(w(ops[8]));
    g.write::<u32>(p, w(ops[9]));
    let d = g.read::<u32>(g.read::<u32>(w(ops[10])).wrapping_add(0x10));
    let s941 = slot_target(g, w(ops[11]), 941);
    // call width: 2 (the part's word and slot 713's)
    let list = g.call(s941, lock, &[d, obj]).eax;
    let program = g.read::<u32>(list.wrapping_add(4));
    start_part(
        g,
        img,
        img.rel32_target(f + ops[12]),
        obj,
        0x1000,
        program,
        1,
    );
    Ret::eax(1)
}

/// An object callback (0x100016b0, `(obj)`): the camera turned (slot 236
/// `(G_C, 0x86, &M)`, its three rows normalised by slot 232, M through
/// slot 230 `(VEC, &M)`), slot 229 `(&P, &M, POS)` and 325 `(VEC2, &P)`;
/// a diagonal matrix of the object's `+0x60` from slot 235 `(&S)`, slot
/// 231 `(&T, VEC, &S)`, then slots 230 `(G_E, &T)`, 231 `(G_EC, G_B,
/// G_E)`, 230 `(G_DC, G_EC)` and the [`done`]. Locals: `M` `ebp - 0x44`,
/// `P` `- 0x54`, `S` `- 0x94`, `T` `- 0xd4`. Each call gets the words the
/// earlier ones left on the stack.
pub fn e16b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp - 4;
    let (m, p, s, t) = (ebp - 0x44, ebp - 0x54, ebp - 0x94, ebp - 0xd4);
    let obj = fr.arg(g, 0);
    let c = g.read::<u32>(w(0x28));
    let s236 = slot_target(g, w(0x1d), 236);
    g.call(s236, fr.ecx, &[c, 0x86, m]);
    let s232 = slot_target(g, w(0x3a), 232);
    // call width: 5
    g.call(s232, fr.ecx, &[m, m, c, 0x86, m]);
    let s232 = slot_target(g, w(0x4c), 232);
    // call width: 7
    g.call(s232, fr.ecx, &[m + 0x10, m + 0x10, m, m, c, 0x86, m]);
    let s232 = slot_target(g, w(0x5e), 232);
    let mut stack = vec![m + 0x20, m + 0x20, m + 0x10, m + 0x10, m, m, c, 0x86, m];
    // call width: 9
    g.call(s232, fr.ecx, &stack);
    let s230 = slot_target(g, w(0x6f), 230);
    stack.splice(0..0, [w(0x74), m]);
    // call width: 11
    g.call(s230, fr.ecx, &stack);
    let s229 = slot_target(g, w(0x8e), 229);
    stack.splice(0..0, [p, m, w(0x81)]);
    // call width: 14
    g.call(s229, fr.ecx, &stack);
    let s325 = slot_target(g, w(0x9f), 325);
    stack.splice(0..0, [w(0xa4), p]);
    // call width: 16
    g.call(s325, fr.ecx, &stack);
    // add esp, 0x40
    let k = g.read::<f32>(obj + 0x60);
    g.write::<f32>(ebp - 0xd8, k);
    let s235 = slot_target(g, w(0xc0), 235);
    g.call(s235, fr.ecx, &[s]);
    let k = g.read::<f32>(ebp - 0xd8);
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(s + o, k);
    }
    let s231 = slot_target(g, w(0xf0), 231);
    let mut stack = vec![t, w(0xea), s, s];
    g.call(s231, fr.ecx, &stack);
    let ge = g.read::<u32>(w(0x11c));
    let s230 = slot_target(g, w(0x116), 230);
    stack.splice(0..0, [ge, t]);
    // call width: 6
    g.call(s230, fr.ecx, &stack);
    let ge = g.read::<u32>(w(0x12a));
    let gb = g.read::<u32>(w(0x135));
    let s231 = slot_target(g, w(0x12f), 231);
    let gec = g.read::<u32>(w(0x141));
    stack.splice(0..0, [gec, gb, ge]);
    // call width: 9
    g.call(s231, fr.ecx, &stack);
    let gec = g.read::<u32>(w(0x149));
    let gdc = g.read::<u32>(w(0x154));
    let s230 = slot_target(g, w(0x14e), 230);
    stack.splice(0..0, [gdc, gec]);
    // call width: 11
    g.call(s230, fr.ecx, &stack);
    let ops = [
        0x161, 0x169, 0x16e, 0x17c, 0x18a, 0x198, 0x19f, 0x1a4, 0x1b1, 0x1b7, 0x1bc, 0x1c4, 0x1df,
    ];
    // call width: 13 (slot 712: two words and these eleven)
    done(g, img, f, obj, stack, ops)
}

/// 0x100016b0 (507 bytes, 1 overlay).
pub const E16B0: Sig = Sig {
    len: 507,
    hash: 0xc235_f263_75cf_dc5d,
    calls: &[
        (0x1df, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x1f3, Callee::Any),
    ],
    imports: &[],
};

/// An object callback (0x100019c0, `(obj)`): [`e16b0`] with the rotation
/// first: after the camera's normalisation and slot 230 `(VEC, &M)`,
/// slot 235 `(&R)`, slot 328 `(&R, &R, ANGLE)` and slot 231 `(VEC, &M,
/// &R)`; then slots 229 `(&P, &M, POS)`, 325 `(VEC2, &P)`, the diagonal
/// matrix (`&S`, slot 231 `(&T, VEC, &S)`), 230 `(G_E, &T)`, and after
/// the stack is dropped 231 `(G_EC, G_B, G_E)`, 230 `(G_DC, G_EC)` and
/// the [`done`]. Locals: `M` `ebp - 0x44`, `P` `- 0x54`, `S` `- 0x94`,
/// `R` `- 0xd4`, `T` `- 0x114`.
pub fn e19c0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp - 4;
    let (m, p, s, r, t) = (ebp - 0x44, ebp - 0x54, ebp - 0x94, ebp - 0xd4, ebp - 0x114);
    let obj = fr.arg(g, 0);
    let c = g.read::<u32>(w(0x28));
    let s236 = slot_target(g, w(0x1d), 236);
    g.call(s236, fr.ecx, &[c, 0x86, m]);
    let s232 = slot_target(g, w(0x3a), 232);
    // call width: 5
    g.call(s232, fr.ecx, &[m, m, c, 0x86, m]);
    let s232 = slot_target(g, w(0x4c), 232);
    // call width: 7
    g.call(s232, fr.ecx, &[m + 0x10, m + 0x10, m, m, c, 0x86, m]);
    let s232 = slot_target(g, w(0x5e), 232);
    let mut stack = vec![m + 0x20, m + 0x20, m + 0x10, m + 0x10, m, m, c, 0x86, m];
    // call width: 9
    g.call(s232, fr.ecx, &stack);
    let s230 = slot_target(g, w(0x6f), 230);
    stack.splice(0..0, [w(0x74), m]);
    // call width: 11
    g.call(s230, fr.ecx, &stack);
    let s235 = slot_target(g, w(0x88), 235);
    stack.insert(0, r);
    // call width: 12
    g.call(s235, fr.ecx, &stack);
    // add esp, 0x2c leaves the first word, overwritten with the angle
    let s328 = slot_target(g, w(0xa7), 328);
    let angle = w(0xa0);
    let mut stack = vec![r, r, angle];
    g.call(s328, fr.ecx, &stack);
    let s231 = slot_target(g, w(0xbf), 231);
    stack.splice(0..0, [w(0xc4), m, r]);
    // call width: 6
    g.call(s231, fr.ecx, &stack);
    let s229 = slot_target(g, w(0xde), 229);
    stack.splice(0..0, [p, m, w(0xd1)]);
    // call width: 9
    g.call(s229, fr.ecx, &stack);
    let s325 = slot_target(g, w(0xef), 325);
    stack.splice(0..0, [w(0xf4), p]);
    // call width: 11
    g.call(s325, fr.ecx, &stack);
    let k = g.read::<f32>(obj + 0x60);
    g.write::<f32>(ebp - 0x118, k);
    let s235 = slot_target(g, w(0x10d), 235);
    stack.insert(0, s);
    // call width: 12
    g.call(s235, fr.ecx, &stack);
    let k = g.read::<f32>(ebp - 0x118);
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(s + o, k);
    }
    let s231 = slot_target(g, w(0x14f), 231);
    stack.splice(0..0, [t, w(0x149), s]);
    // call width: 15
    g.call(s231, fr.ecx, &stack);
    let ge = g.read::<u32>(w(0x169));
    let s230 = slot_target(g, w(0x163), 230);
    stack.splice(0..0, [ge, t]);
    // call width: 17
    g.call(s230, fr.ecx, &stack);
    // add esp, 0x44
    let s231 = slot_target(g, w(0x176), 231);
    let ge = g.read::<u32>(w(0x185));
    let gb = g.read::<u32>(w(0x18b));
    let gec = g.read::<u32>(w(0x191));
    let mut stack = vec![gec, gb, ge];
    g.call(s231, fr.ecx, &stack);
    let gec = g.read::<u32>(w(0x199));
    let gdc = g.read::<u32>(w(0x1a4));
    let s230 = slot_target(g, w(0x19e), 230);
    stack.splice(0..0, [gdc, gec]);
    // call width: 5
    g.call(s230, fr.ecx, &stack);
    let ops = [
        0x1b1, 0x1b9, 0x1be, 0x1cc, 0x1da, 0x1e8, 0x1ef, 0x1f4, 0x201, 0x207, 0x20c, 0x214, 0x22f,
    ];
    // call width: 7 (slot 712: two words and these five)
    done(g, img, f, obj, stack, ops)
}

/// 0x100019c0 (587 bytes, 1 overlay).
pub const E19C0: Sig = Sig {
    len: 587,
    hash: 0x5d62_0d08_cd6e_7fb0,
    calls: &[
        (0x22f, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x243, Callee::Any),
    ],
    imports: &[],
};

/// The start-up (0x10001c10, `(arg)`): slot 803 `(0x80000, 0x10)` gives a
/// buffer (`G_BUF`, operand `+0x19`; `+0x80000` in `+0x23`); `arg` kept
/// (`+0x2b`); the effect `NAME` (`+0x35`, also in `G_NAME`) registered
/// (slots 718, 222 `(NAME, 0, 1000, 0x2000)`), its entry 8 (slot 719,
/// `G_D`), `pppInitData(G_D, ENV, 0)` (slot 711, the game flag `*G_LOCK`
/// 1 around it), slot 710 `(ITEM, G_D, G_BUF2, 0x80000)`, three parts'
/// lists (slot 941 of `G_D + 0x14..0x1c`), four 0x110-byte state blocks
/// cleared and given their defaults, and slot 260 `(1)` / `(0)` into two
/// globals. Each call gets the words the earlier ones left on the stack.
/// Returns 1.
#[allow(clippy::too_many_lines)]
pub fn e1c10<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ecx = fr.ecx;
    let s803 = slot_target(g, w(0x4), 803);
    let buf = g.call(s803, ecx, &[0x80000, 0x10]).eax;
    g.write::<u32>(w(0x19), buf);
    g.write::<u32>(w(0x23), buf.wrapping_add(0x80000));
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x2b), arg);
    let s718 = slot_target(g, w(0x30), 718);
    g.write::<u32>(w(0x41), w(0x45));
    let mut stack = vec![w(0x35), 0x80000, 0x10];
    // call width: 3
    g.call(s718, ecx, &stack);
    let s222 = slot_target(g, w(0x4c), 222);
    let gn = g.read::<u32>(w(0x64));
    stack.splice(0..0, [gn, 0, 0x3e8, 0x2000]);
    // call width: 7
    g.call(s222, ecx, &stack);
    let s719 = slot_target(g, w(0x6b), 719);
    let gn = g.read::<u32>(w(0x7b));
    stack.splice(0..0, [gn, 8, 0]);
    // call width: 10
    let d = g.call(s719, ecx, &stack).eax;
    let lock = g.read::<u32>(w(0x83));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x8c), d);
    g.write::<u32>(lock, 1);
    let s711 = slot_target(g, w(0x97), 711);
    stack.splice(0..0, [d, w(0x91), 0]);
    // call width: 13
    g.call(s711, lock, &stack);
    let lock = g.read::<u32>(w(0xaa));
    let buf2 = g.read::<u32>(w(0xb5));
    g.write::<u32>(lock, saved);
    let gd = g.read::<u32>(w(0xbd));
    let s710 = slot_target(g, w(0xc2), 710);
    stack.splice(0..0, [w(0xc7), gd, buf2, 0x80000]);
    // call width: 17
    g.call(s710, lock, &stack);
    // add esp, 0x44
    let mut stack = Vec::new();
    for (k, (gd_op, tab, dst)) in (0u32..).zip([
        (0xd4u32, 0xdfu32, 0xecu32),
        (0xf1, 0xf9, 0x106),
        (0x10b, 0x113, 0x12c),
    ]) {
        let gd = g.read::<u32>(w(gd_op));
        let x = g.read::<u32>(gd.wrapping_add(0x14 + 4 * k));
        let s941 = slot_target(g, w(tab), 941);
        stack.insert(0, x);
        // call width: 1..3
        let r = g.call(s941, lock, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    // memset(BLOCK, 0, 0x110) and each block's defaults, the memsets'
    // words left on the stack
    let zero = [0u8; 0x110];
    g.write_bytes(w(0x127), &zero);
    stack.splice(0..0, [w(0x127), 0, 0x110]);
    g.write::<u32>(w(0x143), w(0x147));
    for o in [0x14d, 0x157, 0x161] {
        g.write::<u32>(w(o), 1);
    }
    g.write_bytes(w(0x13d), &zero);
    stack.splice(0..0, [w(0x13d), 0, 0x110]);
    g.write::<u32>(w(0x170), w(0x174));
    for o in [0x17a, 0x190, 0x19a] {
        g.write::<u32>(w(o), 1);
    }
    g.write_bytes(w(0x18a), &zero);
    stack.splice(0..0, [w(0x18a), 0, 0x110]);
    g.write::<u32>(w(0x1b5), w(0x1b9));
    for o in [0x1bf, 0x1c9, 0x1d3] {
        g.write::<u32>(w(o), 1);
    }
    g.write_bytes(w(0x1af), &zero);
    stack.splice(0..0, [w(0x1af), 0, 0x110]);
    let s260 = slot_target(g, w(0x1e1), 260);
    g.write::<u32>(w(0x1e7), w(0x1eb));
    for (o, v) in [(0x1f1u32, 2u32), (0x1fb, 2), (0x205, 1), (0x20f, 1)] {
        g.write::<u32>(w(o), v);
    }
    stack.insert(0, 1);
    // call width: 16
    let r = g.call(s260, lock, &stack).eax;
    g.write::<u32>(w(0x222), r);
    let s260 = slot_target(g, w(0x227), 260);
    let r = g.call(s260, lock, &[0]).eax;
    g.write::<u32>(w(0x23c), r);
    Ret::eax(1)
}

/// 0x10001c10 (584 bytes, 1 overlay).
pub const E1C10: Sig = Sig {
    len: 584,
    hash: 0xa2b6_42da_c20e_900f,
    calls: &[
        (0x131, Callee::Import("memset")),
        (0x16a, Callee::Import("memset")),
        (0x1a3, Callee::Import("memset")),
        (0x1dc, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The closing frame (0x10001f30, no arguments): nothing (0) unless `*A`
/// (operand `+0x02`) is set; then `**P` = the immediate `+0x19`
/// (`P` at `+0x0d`), slots 723 `(*M)` and 724 `(*N, *M)` (`+0x13`,
/// `+0x31`), `**Q = *R` (`+0x3f`, `+0x44`); slot 261 asked for flags 0,
/// 4, 1, 2 and 3 in turn: 0 runs the timelines `+0x61` and `+0x6b`
/// ([`timeline_run`](crate::sb_misc::timeline_run)), 4 runs `+0x8b`, 1
/// stops `+0xab` ([`timeline_stop`](crate::sb_misc::timeline_stop)), 2
/// runs `+0xcb`; with 3 clear the counter `*K` (`+0x11c`) counts up, else
/// the timelines `+0xeb` and `+0xf5` stop and slot 214 `(*H)` (`+0x105`)
/// runs. Returns 1. Each slot call passes the words the original leaves
/// on the stack.
pub fn close_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_misc::{timeline_run, timeline_stop};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x0d));
    let m = g.read::<u32>(w(0x13));
    g.write::<u32>(p, w(0x19));
    let s = table(g, 0x1e, 0xb4c / 4);
    g.call(s, 0, &[m]);
    let n = g.read::<u32>(w(0x31));
    let s = table(g, 0x2b, 0xb50 / 4);
    g.call(s, 0, &[n, m]);
    let q = g.read::<u32>(w(0x3f));
    g.write::<u32>(q, g.read::<u32>(w(0x44)));
    let ask = |g: &mut G, at: u32, flag: u32, earlier: &[u32]| {
        let s = table(g, at, 0x414 / 4);
        let mut args = vec![flag];
        args.extend_from_slice(earlier);
        // call width: 3 (the flag and at most the two earlier words)
        g.call(s, 0, &args).eax != 0
    };
    let run = |g: &mut G, call: u32, this: u32| {
        timeline_run(g, img, img.rel32_target(f + call), w(this));
    };
    if ask(g, 0x4b, 0, &[n, m]) {
        run(g, 0x66, 0x61);
        run(g, 0x70, 0x6b);
    }
    if ask(g, 0x75, 4, &[]) {
        run(g, 0x90, 0x8b);
    }
    if ask(g, 0x95, 1, &[]) {
        timeline_stop(g, w(0xab));
    }
    if ask(g, 0xb5, 2, &[]) {
        run(g, 0xd0, 0xcb);
    }
    if !ask(g, 0xd5, 3, &[]) {
        let k = g.read::<u32>(w(0x11c));
        g.write::<u32>(w(0x11c), k.wrapping_add(1));
        return Ret::eax(1);
    }
    timeline_stop(g, w(0xeb));
    timeline_stop(g, w(0xf5));
    let h = g.read::<u32>(w(0x105));
    let s = table(g, 0xff, 0x358 / 4);
    g.call(s, 0, &[h]);
    Ret::eax(1)
}

/// [`close_frame`] (294 bytes, 1 overlay).
pub const CLOSE_FRAME: Sig = Sig {
    len: 294,
    hash: 0x482b_0d61_68b3_62a3,
    calls: &[
        (0x66, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x70, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x90, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xb0, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xd0, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xf0, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xfa, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};
