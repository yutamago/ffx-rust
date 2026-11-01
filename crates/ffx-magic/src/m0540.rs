//! `magic_0540.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{Camera, camera, start_part};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Slot 276 `(model, on)` for each model of the table from operand
/// `start` to `end` (the table of slot 276 at `t`).
fn models<G: Game>(g: &mut G, img: &Image, f: u32, (start, end, t): (u32, u32, u32), on: u32) {
    let w = |off: u32| img.word(f + off);
    let mut p = w(start);
    loop {
        let s = slot_target(g, w(t), 276);
        let m = g.read::<u32>(p);
        g.call(s, 0, &[m, on]);
        p = p.wrapping_add(4);
        if p.cast_signed() >= w(end).cast_signed() {
            break;
        }
    }
}

/// An object's start (0x100010a0, `(obj)`): program 6 started
/// ([`start_part`]), its point `(0.2, -30.4, 139, 1)`, `+0x50..` 0,
/// `+0x5c` 1, the scales 0.1756 (`+0x6c` 1.0); the effect's models shown
/// (slot 276 `(model, 1)`, [`models`]) and slot 845 `()`. Returns 1.
pub fn e10a0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x36], 0x18, &[]);
    for (off, v) in [
        (0x40u32, 0x3e4c_cccdu32),
        (0x44, 0xc1f3_3332),
        (0x48, 0x430b_0000),
        (0x4c, ONE),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x60, 0x3e33_cf70),
        (0x64, 0x3e33_cf70),
        (0x68, 0x3e33_cf70),
        (0x6c, ONE),
    ] {
        g.write::<u32>(obj + off, v);
    }
    models(g, img, f, (0x92, 0xaf, 0x97), 1);
    let t = slot_target(g, w(0xb6), 845);
    g.call(t, 0, &[]);
    Ret::eax(1)
}

/// 0x100010a0 (202 bytes, 1 overlay).
pub const E10A0: Sig = Sig {
    len: 202,
    hash: 0x7ae8_9055_6f38_13e9,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame (0x10001170, `(obj)`): slot 382 `(0, 0)`, slot 315
/// `(0)`, slot 845 `()`, slot 258 `(obj)`, slots 231 and 230 with the
/// camera globals, `*G_P = ITEM` and slot 712 `(obj, 0x30)` with the
/// game flag `*G_LOCK` 1 (restored after; the earlier words left on the
/// stack); slot 275 `(1)`. Returns 1 when slot 712 says 0, else 0.
pub fn e1170<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 382);
    let mut stack = vec![0, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x16), 315);
    stack.insert(0, 0);
    // call width: 3
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x25), 845);
    // call width: 3
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x32), 258);
    stack.insert(0, obj);
    // call width: 4
    g.call(t, 0, &stack);
    for op in [0x43, 0x4e, 0x5a] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x48), 231);
    // call width: 7
    g.call(t, 0, &stack);
    for op in [0x62, 0x6d] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x67), 230);
    // call width: 9
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x7a));
    g.write::<u32>(gp, w(0x85));
    let lock = g.read::<u32>(w(0x8a));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x97), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 11
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0xa5));
    g.write::<u32>(lock, saved);
    let t = slot_target(g, w(0xaf), 275);
    g.call(t, 0, &[1]);
    Ret::eax(u32::from(r == 0))
}

/// 0x10001170 (213 bytes, 1 overlay).
pub const E1170: Sig = Sig {
    len: 213,
    hash: 0x0bec_7190_26aa_4ebd,
    calls: &[],
    imports: &[],
};

/// An object's end (0x10001250, `(obj)`): the effect's models hidden
/// (slot 276 `(model, 0)`, [`models`]), `*G_P = ITEM`, slot 713 `(obj)`
/// removes the part and slot 846 `()`. Returns 1.
pub fn e1250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    models(g, img, f, (0x5, 0x29, 0x11), 0);
    let obj = fr.arg(g, 0);
    let gp = g.read::<u32>(w(0x30));
    g.write::<u32>(gp, w(0x39));
    let t = slot_target(g, w(0x3e), 713);
    g.call(t, 0, &[obj]);
    let t = slot_target(g, w(0x4b), 846);
    g.call(t, 0, &[]);
    Ret::eax(1)
}

/// 0x10001250 (98 bytes, 1 overlay).
pub const E1250: Sig = Sig {
    len: 98,
    hash: 0x78c4_0d9d_850e_9e1c,
    calls: &[],
    imports: &[],
};

/// An object's start (0x100012c0, `(obj)`): the first record of the
/// array at operand `+0x0a` starts program 3 (`y` -2.0, the scales
/// 0.0986), the others program 4 (`y` -35.4, 0.0796) ([`start_part`]);
/// then `+0x6c` 1.0, `+0x50..+0x5c` `(0, 0, 0, 1)`, `+0x4c` 1.0, `z`
/// 135, `x` -0.8. Returns 1.
pub fn e12c0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let first = obj.wrapping_sub(w(0xa)) & 0xffff_ff80 == 0;
    let (call, index, y, s) = if first {
        (0x44, 0xc, 0xbfff_ffe7, 0x3dc9_ecb3)
    } else {
        (0x77, 0x10, 0xc20d_993e, 0x3da3_033a)
    };
    start_part(g, img, f, obj, [0x14, 0x1a, 0x1f, 0x27, call], index, &[]);
    g.write::<u32>(obj + 0x44, y);
    for off in [0x60, 0x64, 0x68] {
        g.write::<u32>(obj + off, s);
    }
    for (off, v) in [
        (0x6cu32, ONE),
        (0x5c, 1),
        (0x58, 0),
        (0x54, 0),
        (0x50, 0),
        (0x4c, ONE),
        (0x48, 0x4307_0000),
        (0x40, 0xbf4c_ccbc),
    ] {
        g.write::<u32>(obj + off, v);
    }
    Ret::eax(1)
}

/// 0x100012c0 (214 bytes, 1 overlay).
pub const E12C0: Sig = Sig {
    len: 214,
    hash: 0xc55e_83bb_9c05_c63a,
    calls: &[
        (0x44, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x77, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// The effect's frame (0x10001a00, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else the
/// matrices ([`camera`]), `*G_V = *G_W`; the first time slot 261 `(3)`
/// says nonzero (the count at `+0x62` 0), slot 417 `(CB, 0, 0, 0)`
/// starts a task (`+0x8b`; the count up); slot 261 `(3)` and `(2)` each
/// run a timeline ([`timeline_run`](crate::sb_misc::timeline_run)) when
/// they say nonzero; when `(1)` does, slot 418 `(task)`, the three
/// timelines stop ([`timeline_stop`](crate::sb_misc::timeline_stop)) and
/// slot 214 `(*G_E)` runs; else the frame count `+0x134` goes up.
/// Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let c = Camera {
        gp: 0xd,
        cam: 0x13,
        item: 0x19,
        t723: 0x1e,
        t724: 0x2b,
        m: 0x31,
    };
    let mut stack = camera(g, img, f, &c, &[]);
    let v = g.read::<u32>(w(0x3f));
    let x = g.read::<u32>(w(0x44));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x4b), 261);
    stack.insert(0, 3);
    // call width: 3
    if g.call(t, 0, &stack).eax != 0 && g.read::<u32>(w(0x62)) == 0 {
        let t = slot_target(g, w(0x6a), 417);
        let h = g.call(t, 0, &[w(0x7b), 0, 0, 0]).eax;
        let n = w(0x86);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        g.write::<u32>(w(0x8b), h);
    }
    for (tab, k, call, this) in [(0x90u32, 3u32, 0xabu32, 0xa6u32), (0xb0, 2, 0xcb, 0xc6)] {
        let t = slot_target(g, w(tab), 261);
        if g.call(t, 0, &[k]).eax != 0 {
            crate::sb_misc::timeline_run(g, img, img.rel32_target(f + call), w(this));
        }
    }
    let t = slot_target(g, w(0xd0), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = w(0x134);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    let t = slot_target(g, w(0xe6), 418);
    let task = g.read::<u32>(w(0xec));
    g.call(t, 0, &[task]);
    for op in [0xf9, 0x103, 0x10d] {
        crate::sb_misc::timeline_stop(g, w(op));
    }
    let t = slot_target(g, w(0x117), 214);
    let e = g.read::<u32>(w(0x11d));
    g.call(t, 0, &[e, task]);
    Ret::eax(1)
}

/// 0x10001a00 (318 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 318,
    hash: 0xe93c_b381_8f5b_5b44,
    calls: &[
        (0xab, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xcb, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xfe, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x108, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x112, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// The effect's set-up (0x10001660, an overlay table entry, `(arg)`):
/// slot 803 `(0x1000000, 16)` the buffer (its address and `+0x80000` to
/// two globals, the argument to a third), the name global set and slot
/// 718 `(NAME)`, slot 222 `(*G_N, 0, 1000, 0x2000)`, a handle `h` from
/// slot 719 `(*G_N, 8, 0)` (kept), slot 711 `(h, PROG, 0)` under
/// `**G_LOCK` 1 and slot 710 `(BLOCK, *G_H, *G_BUF, 0x80000)`; the
/// handle's three program tables (slot 941 `(h[0x14 + 4i])`, kept); the
/// three 0x110-byte timeline blocks zeroed (`memset`) with their
/// callbacks, counts and flags (each block's fields set before the next
/// block is cleared); nine sound handles from slot 260 `(id)`. Each call
/// sees the words the earlier ones left on the stack, up to the
/// clean-ups. Returns 1.
#[allow(clippy::too_many_lines)] // the original's one long sequence
pub fn setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, op: u32| g.read::<u32>(w(op));
    let t = slot_target(g, w(0x4), 803);
    let mut stack = vec![0x100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x19), buf);
    g.write::<u32>(w(0x23), buf.wrapping_add(0x8_0000));
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x2b), arg);
    let t = slot_target(g, w(0x30), 718);
    g.write::<u32>(w(0x41), w(0x45));
    stack.insert(0, w(0x35));
    // call width: 3
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x4c), 222);
    let n = r(g, 0x64);
    stack.splice(0..0, [n, 0, 0x3e8, 0x2000]);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x6b), 719);
    let n = r(g, 0x7b);
    stack.splice(0..0, [n, 8, 0]);
    // call width: 10
    let h = g.call(t, 0, &stack).eax;
    let lock = r(g, 0x83);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x8c), h);
    let t = slot_target(g, w(0x97), 711);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [h, w(0x91), 0]);
    // call width: 13
    g.call(t, 0, &stack);
    let lock = r(g, 0xaa);
    let b = r(g, 0xb5);
    g.write::<u32>(lock, saved);
    let h2 = r(g, 0xbd);
    let t = slot_target(g, w(0xc2), 710);
    stack.splice(0..0, [w(0xc7), h2, b, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    let mut stack = Vec::new();
    for (k, (hop, tab, dst)) in [
        (0xd4u32, 0xdfu32, 0xecu32),
        (0xf1, 0xf9, 0x106),
        (0x10b, 0x113, 0x12c),
    ]
    .into_iter()
    .enumerate()
    {
        let h = r(g, hop);
        let x = g.read::<u32>(h + 0x14 + 4 * k as u32);
        let t = slot_target(g, w(tab), 941);
        stack.insert(0, x);
        // call width: 3 at most
        let v = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), v);
    }
    // the three blocks: each zeroed (its `memset` words left on the
    // stack), then its fields
    stack.splice(0..0, [w(0x127), 0, 0x110]);
    g.write_bytes(w(0x127), &[0u8; 0x110]);
    stack.splice(0..0, [w(0x13d), 0, 0x110]);
    g.write::<u32>(w(0x143), w(0x147));
    for (op, v) in [(0x14du32, 2u32), (0x157, 2), (0x161, 1), (0x16b, 1)] {
        g.write::<u32>(w(op), v);
    }
    g.write_bytes(w(0x13d), &[0u8; 0x110]);
    g.write::<u32>(w(0x17a), w(0x17e));
    stack.splice(0..0, [w(0x18a), 0, 0x110]);
    for op in [0x190u32, 0x19a, 0x1a4] {
        g.write::<u32>(w(op), 1);
    }
    g.write_bytes(w(0x18a), &[0u8; 0x110]);
    let t = slot_target(g, w(0x1b2), 260);
    g.write::<u32>(w(0x1b8), w(0x1bc));
    g.write::<u32>(w(0x1c2), 10);
    g.write::<u32>(w(0x1cc), 10);
    for op in [
        0x1d6u32, 0x1e0, 0x1ea, 0x1f4, 0x1fe, 0x208, 0x212, 0x21c, 0x226, 0x230,
    ] {
        g.write::<u32>(w(op), 1);
    }
    stack.insert(0, 3);
    // call width: 13
    let v = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x243), v);
    for (tab, id, dst) in [
        (0x248u32, 0x16u32, 0x257u32),
        (0x25c, 0x17, 0x26b),
        (0x270, 0x18, 0x27f),
    ] {
        let t = slot_target(g, w(tab), 260);
        stack.insert(0, id);
        // call width: 16 at most
        let v = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), v);
    }
    let mut stack = Vec::new();
    for (tab, id, dst) in [
        (0x284u32, 0x19u32, 0x296u32),
        (0x29b, 0x1a, 0x2aa),
        (0x2af, 0x1b, 0x2be),
        (0x2c3, 7, 0x2d2),
        (0x2d7, 0x15, 0x2e9),
    ] {
        let t = slot_target(g, w(tab), 260);
        stack.insert(0, id);
        // call width: 5 at most
        let v = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), v);
    }
    Ret::eax(1)
}

/// [`setup`] (757 bytes, 1 overlay).
pub const SETUP: Sig = Sig {
    len: 757,
    hash: 0x8b9f_d001_fa8e_9410,
    calls: &[
        (0x131, Callee::Import("memset")),
        (0x174, Callee::Import("memset")),
        (0x1ad, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The part's program list: `*G_P = ITEM` (operands `op`, `imm`), slot
/// 941 `((*G_D)[0x10])` (`G_D` at `d`, the table global at `t`).
fn programs<G: Game>(g: &mut G, img: &Image, f: u32, (op, imm, d, t): (u32, u32, u32, u32)) -> u32 {
    let w = |off: u32| img.word(f + off);
    let gp = g.read::<u32>(w(op));
    g.write::<u32>(gp, w(imm));
    let arg = g.read::<u32>(g.read::<u32>(w(d)).wrapping_add(0x10));
    let s941 = slot_target(g, w(t), 941);
    // call width: 1
    g.call(s941, 0, &[arg]).eax
}

/// The 32-bit stores `mov dword [esi + d8], imm32` (7 bytes each) at the
/// offsets `at`, on `obj`.
fn obj_stores<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, at: &[u32]) {
    for &o in at {
        let d = crate::o4_parts::disp8(img, f + o + 2);
        g.write::<u32>(obj.wrapping_add(d), img.word(f + o + 3));
    }
}

/// One of four parts started (0x10001460, cdecl `(part)`): the part's
/// index `(part - P) >> 7` (`P` at `+0x2a`; arithmetic) picks the
/// program (1 for part 3, else 0) and, through the jump table (`+0x66`),
/// its position `+0x40..+0x48` and scale `+0x60..+0x68`; then `+0x6c`
/// and `+0x4c` 1.0, `+0x5c` 1, `+0x50..+0x58` 0. An index above 3 sets
/// none of them. The program is started on the part
/// ([`crate::opu_parts::start_part`], `0x1000`, `(program, 1)`).
/// Returns 1.
pub fn part_start_of4<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let part = fr.arg(g, 0);
    let table = programs(g, img, f, (0x04, 0x0e, 0x13, 0x1c));
    let index = part.wrapping_sub(img.word(f + 0x2a)).cast_signed() >> 7;
    let (prog, call) = if index == 3 {
        (g.read::<u32>(table.wrapping_add(4)), 0x5c)
    } else {
        (g.read::<u32>(table), 0x46)
    };
    crate::opu_parts::start_part(g, img, img.rel32_target(f + call), part, 0x1000, prog, 1);
    if index.cast_unsigned() > 3 {
        return Ret::eax(1);
    }
    // the jump table's entry for the index: one of the four cases
    let case = img
        .word(img.word(f + 0x66).wrapping_add(4 * index.cast_unsigned()))
        .wrapping_sub(f);
    let first = match case {
        0x6a => 0x6a,
        0x99 => 0x99,
        0xc5 => 0xc5,
        _ => 0xf1,
    };
    let at: Vec<u32> = (0..6).map(|i| first + 7 * i).collect();
    obj_stores(g, img, f, part, &at);
    obj_stores(g, img, f, part, &[0x11b, 0x122, 0x129, 0x130, 0x137, 0x13e]);
    Ret::eax(1)
}

/// [`part_start_of4`] (352 bytes with its jump table, 1 overlay).
pub const PART_START_OF4: Sig = Sig {
    len: 352,
    hash: 0x0fa5_3cfe_401e_fbb5,
    calls: &[
        (0x46, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x5c, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// The fifth part started (0x100015c0, cdecl `(part)`): program 5 of
/// the list (`*G_P = ITEM`, slot 941) started on it
/// ([`crate::opu_parts::start_part`]); its position, `+0x4c` 1.0,
/// `+0x50..+0x58` 0, `+0x5c` 1, its scale and `+0x6c` 1.0 from
/// immediates. Returns 1.
pub fn part_start_5<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = programs(g, img, f, (0x04, 0x0b, 0x10, 0x18));
    let part = fr.arg(g, 0);
    let prog = g.read::<u32>(table.wrapping_add(0x14));
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x36), part, 0x1000, prog, 1);
    let at: Vec<u32> = (0..12).map(|i| 0x3d + 7 * i).collect();
    obj_stores(g, img, f, part, &at);
    Ret::eax(1)
}

/// [`part_start_5`] (153 bytes, 1 overlay).
pub const PART_START_5: Sig = Sig {
    len: 153,
    hash: 0x69da_6324_dd25_46b6,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
