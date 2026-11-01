//! `magic_0499.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::sa_misc::part_refresh_b;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start (0x100012b0, an overlay table entry, `(arg)`):
/// `arg` to the global at operand `+0x07`; slot 803 `(0x100000, 0x10)`
/// the buffer (`+0x25`), slot 861 `(buf, NAME, 0x21940)` loads the file
/// into it, slot 259; slot 260 `(0x14)` and `(0x15)` two handles
/// (`+0x59`, `+0x8d`), each followed by five halfword flags cleared;
/// slots 718 `(NAME)` and 222 `(NAME, 0, 1000, 0x100)` (each with the
/// earlier words on the stack); three state words 0. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x7), arg);
    let t = slot_target(g, w(0xc), 803);
    let mut stack = vec![0x10_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x25), buf);
    stack.splice(0..0, [buf, w(0x2a), 0x2_1940]);
    let t = slot_target(g, w(0x30), 861);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x3d), 259);
    g.call(t, 0, &stack);
    for (tab, n, dst, flags) in [
        (0x4au32, 0x14u32, 0x59u32, [0x61u32, 0x67, 0x6d, 0x73, 0x79]),
        (0x7e, 0x15, 0x8d, [0x95, 0x9b, 0xa1, 0xa7, 0xad]),
    ] {
        let t = slot_target(g, w(tab), 260);
        stack.insert(0, n);
        // call width: 12
        let h = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), h);
        for op in flags {
            g.write::<u16>(w(op), 0);
        }
    }
    let t = slot_target(g, w(0xb2), 718);
    stack.insert(0, w(0xb7));
    // call width: 12
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xc4), 222);
    stack.splice(0..0, [w(0xdb), 0, 0x3e8, 0x100]);
    // call width: 12
    g.call(t, 0, &stack);
    for op in [0xe6, 0xf0, 0xfa] {
        g.write::<u32>(w(op), 0);
    }
    Ret::eax(0)
}

/// 0x100012b0 (262 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 262,
    hash: 0x71f3_c43e_9518_338d,
    calls: &[],
    imports: &[],
};

/// The five parts' refresh (0x100010a0, internal, `ecx` = the block; a
/// /GS frame). Part `i` lives at `block + 0x10 + 0x80 * i`; with its
/// flag `u16 +0x1e` set, parts 0-2 get their matrix rebuilt: slot 236
/// `(*block, TAB[i], &A)` (`TAB` operand `+0x3e`), `A`'s last row
/// `(0, 0, 0, 1)`, slot 243 `(&B, 0x8000)`, slot 231 `(&A, &A, &B)`, the
/// three rows of `*G_M` scaled by the part's `+0x60/+0x64/+0x68` through
/// slot 233, slot 237 `(*block, TAB[i], 0, *G_M + 0x30)`, slots 231
/// `(*G_X, *G_Y, *G_M)` and 230 `(*G_Z, *G_X)`, `**G_PART` = the DLL's
/// part block and slot 712 `(part, 0x30)` with `**G_LOCK` 1 around it;
/// slot 712 non-zero clears the flag, and the lock ends 0. Parts 3-4 get
/// slot 237 `(*block, TAB[i], 0, part + 0x40)` and the part refresh
/// ([`part_refresh_b`]). Locals: `A` `ebp - 0x44`, `B` `ebp - 0x84`.
/// Each call sees the words the earlier ones left on the stack. No caller
/// reads `eax` (the last path's value; the /GS cookie's with no flag
/// set).
pub fn parts_refresh<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let block = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let (a, b) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x84));
    let mut eax = 0;
    for i in 0..5u32 {
        let part = block + 0x10 + 0x80 * i;
        if g.read::<u16>(part + 0x1e) == 0 {
            continue;
        }
        if i >= 3 {
            let tab = g.read::<u32>(w(0x1cc) + 4 * i);
            let t = slot_target(g, w(0x1c3), 237);
            let base = g.read::<u32>(block);
            let r = g.call(t, 0, &[base, tab, 0, part + 0x40]).eax;
            eax = part_refresh_b(g, img, img.rel32_target(f + 0x1e1), part, r);
            continue;
        }
        let tab = g.read::<u32>(w(0x3e) + 4 * i);
        let t = slot_target(g, w(0x43), 236);
        let base = g.read::<u32>(block);
        g.call(t, 0, &[base, tab, a]);
        for (o, v) in [(0x38u32, 0u32), (0x34, 0), (0x30, 0), (0x3c, 0x3f80_0000)] {
            g.write::<u32>(a + o, v);
        }
        let t = slot_target(g, w(0x5e), 243);
        g.call(t, 0, &[b, 0x8000, base, tab, a]);
        let t = slot_target(g, w(0x93), 231);
        g.call(t, 0, &[a, a, b, b, 0x8000, base, tab, a]);
        // the three rows of `*G_M` scaled (slot 233 `(row, &A[k], s)`)
        let mut stack = Vec::new();
        for (k, (tm, m)) in [(0xb1u32, 0xb7u32), (0xdd, 0xd4), (0x104, 0xfb)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            let s = g.read::<u32>(part + 0x60 + 4 * k);
            let row = g.read::<u32>(w(m)) + 0x10 * k;
            let t = slot_target(g, w(tm), 233);
            stack = vec![row, a + 0x10 * k, s];
            g.call(t, 0, &stack);
        }
        let m3 = g.read::<u32>(w(0x111)) + 0x30;
        let t = slot_target(g, w(0x11a), 237);
        let tab = g.read::<u32>(w(0x123) + 4 * i);
        let base = g.read::<u32>(block);
        stack.splice(0..0, [base, tab, 0, m3]);
        // call width: 7
        g.call(t, 0, &stack);
        let m = g.read::<u32>(w(0x133));
        let t = slot_target(g, w(0x138), 231);
        let y = g.read::<u32>(w(0x13e));
        let x = g.read::<u32>(w(0x14a));
        stack.splice(0..0, [x, y, m]);
        // call width: 10
        g.call(t, 0, &stack);
        let x = g.read::<u32>(w(0x152));
        let t = slot_target(g, w(0x157), 230);
        let z = g.read::<u32>(w(0x15d));
        stack.splice(0..0, [z, x]);
        // call width: 12
        g.call(t, 0, &stack);
        let lock = g.read::<u32>(w(0x16b));
        let pp = g.read::<u32>(w(0x170));
        g.write::<u32>(lock, 1);
        g.write::<u32>(pp, w(0x17c));
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        let t = slot_target(g, w(0x189), 712);
        stack.splice(0..0, [part, 0x30]);
        // call width: 14
        let r = g.call(t, 0, &stack).eax;
        let lock = g.read::<u32>(w(0x19d));
        g.write::<u32>(lock, saved);
        if r != 0 {
            g.write::<u16>(part + 0x1e, 0);
        }
        g.write::<u32>(lock, 0);
        eax = 0;
    }
    eax
}

/// 0x100010a0 (515 bytes, 1 overlay).
pub const PARTS_REFRESH: Sig = Sig {
    len: 515,
    hash: 0x8058_21a4_de42_a5b5,
    calls: &[
        (0x1e1, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        // __security_check_cookie
        (0x1fb, Callee::Any),
    ],
    imports: &[],
};

/// The per-frame step (0x10001400, an overlay table entry, no
/// arguments). Three one-time steps on `*G_H`: slot 225 (non-zero: not
/// ready, return), slot 226 `(*G_H, 0)` once, slot 227 (below 1: not
/// ready); then once slot 711 `(PROG, NAME, 0)` and slot 710 `(BLOCK,
/// PROG, *G_BUF, 0x80000)` with `**G_LOCK` 1 around the first. Slot 261
/// `(0)` non-zero counts the frame; nothing more while `**G_STOP` is set.
/// Frame 1 starts two rows of three parts (scale 0.16, program 0),
/// frame 30 two rows of two (scale 0.05, program 1), frame 100 ends with
/// slot 214 `(*G_H)`. Returns 0.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let h = |g: &mut G, op: u32| g.read::<u32>(w(op));
    if h(g, 0x2) == 0 {
        let t = slot_target(g, w(0xa), 225);
        let a = h(g, 0x10);
        if g.call(t, 0, &[a]).eax != 0 {
            return Ret::eax(0);
        }
        let n = h(g, 0x29);
        g.write::<u32>(w(0x29), n.wrapping_add(1));
    }
    if h(g, 0x2f) == 0 {
        let t = slot_target(g, w(0x37), 226);
        let a = h(g, 0x3f);
        g.write::<u32>(w(0x4b), 1);
        g.call(t, 0, &[a, 0]);
    }
    if h(g, 0x5a) == 0 {
        let t = slot_target(g, w(0x62), 227);
        let a = h(g, 0x68);
        if g.call(t, 0, &[a]).eax.cast_signed() < 1 {
            return Ret::eax(0);
        }
        let n = h(g, 0x82);
        g.write::<u32>(w(0x82), n.wrapping_add(1));
    }
    if h(g, 0x88) == 0 {
        let lock = h(g, 0x91);
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        let t = slot_target(g, w(0xa0), 711);
        g.write::<u32>(w(0xb6), 1);
        let first = [w(0xb0), w(0xa5), 0];
        g.call(t, 0, &first);
        let lock = h(g, 0xc1);
        let buf = h(g, 0xcc);
        g.write::<u32>(lock, saved);
        let t = slot_target(g, w(0xd3), 710);
        let mut args = vec![w(0xe3), w(0xd8), buf, 0x8_0000];
        args.extend(first);
        // call width: 7
        g.call(t, 0, &args);
    }
    let t = slot_target(g, w(0xed), 261);
    let r = g.call(t, 0, &[0]).eax;
    let stop = h(g, 0xfd);
    let n = if r != 0 {
        if g.read::<u32>(stop) != 0 {
            return Ret::eax(0);
        }
        let n = h(g, 0x112).wrapping_add(1);
        g.write::<u32>(w(0x118), n);
        n
    } else {
        h(g, 0x11f)
    };
    if g.read::<u32>(stop) != 0 {
        return Ret::eax(0);
    }
    match n {
        1 => rows(g, img, f, w(0x138), &ROWS_1, 0, 0x3e23_d70a),
        0x1e => rows(g, img, f, w(0x34c), &ROWS_30, 4, 0x3d4c_cccd),
        0x64 => {
            let t = slot_target(g, w(0x4ab), 214);
            let a = h(g, 0x4b1);
            g.call(t, 0, &[a]);
        }
        _ => {}
    }
    Ret::eax(0)
}

/// One part of a [`rows`] row: the operands of `G_P` (read before the
/// part, or after the previous part's start), `ITEM` (stored after the
/// previous part, or before the first), the slot table, 941's word and
/// the wrapper's `rel32`.
struct RowPart {
    p: u32,
    item: u32,
    slots: u32,
    prog: u32,
    call: u32,
}

const ROWS_1: [RowPart; 3] = [
    RowPart {
        p: 0x151,
        item: 0x157,
        slots: 0x15c,
        prog: 0x162,
        call: 0x17d,
    },
    RowPart {
        p: 0x182,
        item: 0x1db,
        slots: 0x1e0,
        prog: 0x1e6,
        call: 0x201,
    },
    RowPart {
        p: 0x206,
        item: 0x284,
        slots: 0x289,
        prog: 0x28f,
        call: 0x2ad,
    },
];

const ROWS_30: [RowPart; 2] = [
    RowPart {
        p: 0x361,
        item: 0x367,
        slots: 0x36c,
        prog: 0x372,
        call: 0x38e,
    },
    RowPart {
        p: 0x393,
        item: 0x3ec,
        slots: 0x3f1,
        prog: 0x3f7,
        call: 0x413,
    },
];

/// Two rows (`0x290` bytes apart from `first`) of parts `0x80` apart:
/// part `j`'s node `first - 0x40 + 0x80 * j` started with the program
/// table's word at `index` (slot 941 `(*G_PROG)`, `*G_P = ITEM` before),
/// its `u16 +0x1e` 1 and `+0x40..+0x6c` = `(0, 0, 0, 1, 0, 0, 0, 1, s,
/// s, s, 1.0)`. The pushes of a row stay on the stack until its end.
fn rows<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    first: u32,
    parts: &[RowPart],
    index: u32,
    s: u32,
) {
    let w = |off: u32| img.word(f + off);
    for row in 0..2u32 {
        let base = first + 0x290 * row;
        let mut stack: Vec<u32> = Vec::new();
        let mut p = g.read::<u32>(w(parts[0].p));
        g.write::<u32>(p, w(parts[0].item));
        for (j, part) in (0u32..).zip(parts) {
            if j > 0 {
                g.write::<u32>(p, w(part.item));
            }
            let t = slot_target(g, w(part.slots), 941);
            stack.insert(0, g.read::<u32>(w(part.prog)));
            // call width: 9 (941's word and the earlier parts' four each)
            let table = g.call(t, 0, &stack).eax;
            let prog = g.read::<u32>(table + index);
            stack.splice(0..0, [prog, 1, 0]);
            let node = base - 0x40 + 0x80 * j;
            start_part(
                g,
                img,
                img.rel32_target(f + part.call),
                node,
                0x1000,
                prog,
                1,
            );
            if let Some(next) = parts.get(j as usize + 1) {
                p = g.read::<u32>(w(next.p));
            }
            let at = base + 0x80 * j;
            g.write::<u16>(at - 0x22, 1);
            for (o, v) in [
                (0u32, 0u32),
                (4, 0),
                (8, 0),
                (0xc, 0x3f80_0000),
                (0x10, 0),
                (0x14, 0),
                (0x18, 0),
                (0x1c, 1),
                (0x20, s),
                (0x24, s),
                (0x28, s),
                (0x2c, 0x3f80_0000),
            ] {
                g.write::<u32>(at + o, v);
            }
        }
    }
}

/// 0x10001400 (1222 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 1222,
    hash: 0x8523_3697_932f_479f,
    calls: &[
        (0x17d, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x201, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x2ad, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x38e, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x413, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};
