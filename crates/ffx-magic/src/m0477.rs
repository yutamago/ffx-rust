//! `magic_0477.dll`'s own functions (besides the runtime and the shared
//! code): the overlay table's part functions. Each draws or starts one
//! particle part (`pppStartPart` through the wrapper, slots 712/713) at a
//! point the effect keeps in its globals; the copies differ in where the
//! point comes from and in their operand layout.

use crate::part_draw::Axis;
use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The operands of a drawing copy's start: `G_P`, `G_CAM`, the item
/// stored at `*G_P`, the tables of slots 723 and 724, `G_M`, the table of
/// slot 258 and the part.
struct Start {
    gp: u32,
    cam: u32,
    item: u32,
    t723: u32,
    t724: u32,
    m: u32,
    t258: u32,
    part: u32,
}

/// How a drawing copy ends, with the shift of its operands from the
/// first copy of that kind (0x10001070 for `Plain`, 0x10001250 for
/// `Removable`).
#[derive(Clone, Copy)]
enum End {
    /// Slot 712 with the lock 1, then the lock 0; returns 1.
    Plain(u32),
    /// Slot 712 with the lock 1 and restored; when it says nonzero, slot
    /// 713 removes the part and the lock is 0 (returns 0), else the lock
    /// is 0 (returns 1).
    Removable(u32),
}

/// Draws the part at a point: `*G_P = ITEM`, slot 723 `(*G_CAM)` and 724
/// `(*G_M)` (the camera and model matrices), the point's axes, slot 258
/// `(part)`, slot 231 `(*G_C, *G_D, *G_E)` and slot 230 `(*G_F, *G_C)`
/// (operands per [`End`]), then slot 712 `(part, 0x30)` with the game
/// flag `*G_LOCK` 1 and the end. `below`: the words the caller left
/// under the first push. Every call passes the words on the original's
/// stack.
fn draw<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    s: &Start,
    axes: &[Axis],
    end: End,
    below: &[u32],
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let gp = g.read::<u32>(w(s.gp));
    let cam = g.read::<u32>(w(s.cam));
    g.write::<u32>(gp, w(s.item));
    let mut stack = vec![cam];
    stack.extend(below);
    let t = slot_target(g, w(s.t723), 723);
    // call width: 14
    g.call(t, 0, &stack);
    let m = g.read::<u32>(w(s.m));
    stack.insert(0, m);
    let t = slot_target(g, w(s.t724), 724);
    // call width: 14
    g.call(t, 0, &stack);
    let t258 = slot_target(g, w(s.t258), 258);
    for a in axes {
        a.run(g, img, f);
    }
    let part = w(s.part);
    stack.insert(0, part);
    // call width: 14
    g.call(t258, 0, &stack);
    // the operands of slots 231 and 230 in the first copy of each kind
    let (d, o) = match end {
        End::Plain(d) => (d, [0xa9u32, 0xb4, 0xc0, 0xae, 0xc8, 0xd3, 0xcd]),
        End::Removable(d) => (d, [0x8e, 0x99, 0xa5, 0x93, 0xad, 0xb8, 0xb2]),
    };
    let w = |off: u32| img.word(f + off - d);
    for &op in &o[..3] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(o[3]), 231);
    // call width: 14
    g.call(t, 0, &stack);
    for &op in &o[4..6] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(o[6]), 230);
    // call width: 14
    g.call(t, 0, &stack);
    match end {
        End::Plain(_) => {
            let gp = g.read::<u32>(w(0xe0));
            g.write::<u32>(gp, w(0xe8));
            let lock = g.read::<u32>(w(0xed));
            g.write::<u32>(lock, 1);
            let t = slot_target(g, w(0xfd), 712);
            let mut args = vec![w(0xf2), 0x30];
            args.extend(&stack);
            // call width: 14
            g.call(t, 0, &args);
            let lock = g.read::<u32>(w(0x10a));
            g.write::<u32>(lock, 0);
            1
        }
        End::Removable(_) => {
            let lock = g.read::<u32>(w(0xc6));
            let gp = g.read::<u32>(w(0xcb));
            g.write::<u32>(lock, 1);
            g.write::<u32>(gp, w(0xd7));
            let t = slot_target(g, w(0xdc), 712);
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let mut args = vec![w(0xf1), 0x30];
            args.extend(&stack);
            // call width: 14
            let r = g.call(t, 0, &args).eax;
            let lock = g.read::<u32>(w(0xf9));
            g.write::<u32>(lock, saved);
            if r == 0 {
                g.write::<u32>(lock, 0);
                return 1;
            }
            let gp = g.read::<u32>(w(0x108));
            g.write::<u32>(gp, w(0x113));
            let t = slot_target(g, w(0x118), 713);
            g.call(t, 0, &[w(0x10d)]);
            let lock = g.read::<u32>(w(0x125));
            g.write::<u32>(lock, 0);
            0
        }
    }
}

/// The start operands of the copies whose code starts with `mov eax,
/// G_P; push esi; push G_CAM` (the part's operand differs).
const fn start_esi(part: u32) -> Start {
    Start {
        gp: 0x1,
        cam: 0x8,
        item: 0xe,
        t723: 0x13,
        t724: 0x20,
        m: 0x26,
        t258: 0x3b,
        part,
    }
}

/// 0x10001070 (an overlay table entry, no arguments): slot 237 `(*G_A,
/// 0x66, 0, V)` fills the point `V` (operand `+0x06`), whose `y` then
/// goes down by `K` (`+0x28`); the point is copied to the part and drawn
/// ([`draw`], [`End::Plain`]). Returns 1.
pub fn e1070<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 237);
    let a = g.read::<u32>(w(0x16));
    let below = [a, 0x66, 0, w(0x6)];
    g.call(t, 0, &below);
    Axis {
        src: 0x20,
        steps: &[(false, 0x28)],
        dst: 0x3b,
    }
    .run(g, img, f);
    let s = Start {
        gp: 0x2d,
        cam: 0x33,
        item: 0x41,
        t723: 0x46,
        t724: 0x53,
        m: 0x59,
        t258: 0x6e,
        part: 0xa1,
    };
    let axes = [
        Axis::copy(0x69, 0x76),
        Axis::copy(0x7e, 0x86),
        Axis::copy(0x8e, 0x96),
    ];
    Ret::eax(draw(g, img, f, &s, &axes, End::Plain(0), &below))
}

/// 0x10001070 (285 bytes, 1 overlay).
pub const E1070: Sig = Sig {
    len: 285,
    hash: 0x5343_917c_edf7_ba0b,
    calls: &[],
    imports: &[],
};

/// 0x10001250 (an overlay table entry, no arguments): the part drawn at
/// a point `(x, y - K1 + K2, z + K3)` ([`draw`], [`End::Removable`]).
pub fn e1250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let axes = [
        Axis::copy(0x36, 0x43),
        Axis {
            src: 0x4b,
            steps: &[(false, 0x53), (true, 0x60)],
            dst: 0x68,
        },
        Axis {
            src: 0x70,
            steps: &[(true, 0x78)],
            dst: 0x80,
        },
    ];
    let r = draw(
        g,
        img,
        fr.at,
        &start_esi(0x58),
        &axes,
        End::Removable(0),
        &[],
    );
    Ret::eax(r)
}

/// 0x10001250 (321 bytes, 1 overlay).
pub const E1250: Sig = Sig {
    len: 321,
    hash: 0x8175_a33f_5888_743b,
    calls: &[],
    imports: &[],
};

/// 0x10001430 (an overlay table entry, no arguments): the part drawn at
/// a point `(x, y - K1 - K2, z + K3)` ([`draw`], [`End::Removable`]).
pub fn e1430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let axes = [
        Axis::copy(0x36, 0x43),
        Axis {
            src: 0x4b,
            steps: &[(false, 0x53), (false, 0x60)],
            dst: 0x68,
        },
        Axis {
            src: 0x70,
            steps: &[(true, 0x78)],
            dst: 0x80,
        },
    ];
    let r = draw(
        g,
        img,
        fr.at,
        &start_esi(0x58),
        &axes,
        End::Removable(0),
        &[],
    );
    Ret::eax(r)
}

/// 0x10001430 (321 bytes, 1 overlay).
pub const E1430: Sig = Sig {
    len: 321,
    hash: 0x28c1_9058_53b0_48bf,
    calls: &[],
    imports: &[],
};

/// 0x10001670 (an overlay table entry, no arguments): the part drawn at
/// a point `(x, y - K, z)` ([`draw`], [`End::Removable`]).
pub fn e1670<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let axes = [
        Axis::copy(0x36, 0x43),
        Axis {
            src: 0x4b,
            steps: &[(false, 0x53)],
            dst: 0x60,
        },
        Axis::copy(0x68, 0x70),
    ];
    let r = draw(
        g,
        img,
        fr.at,
        &start_esi(0x58),
        &axes,
        End::Removable(0x10),
        &[],
    );
    Ret::eax(r)
}

/// 0x10001670 (305 bytes, 1 overlay).
pub const E1670: Sig = Sig {
    len: 305,
    hash: 0x5ec2_205b_35ce_4713,
    calls: &[],
    imports: &[],
};

/// 0x10001850 (an overlay table entry, no arguments): the part drawn at
/// a point copied as it is ([`draw`], [`End::Removable`]).
pub fn e1850<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let axes = [
        Axis::copy(0x36, 0x43),
        Axis::copy(0x4b, 0x53),
        Axis::copy(0x5b, 0x63),
    ];
    let r = draw(
        g,
        img,
        fr.at,
        &start_esi(0x6e),
        &axes,
        End::Removable(0x18),
        &[],
    );
    Ret::eax(r)
}

/// 0x10001850 (297 bytes, 1 overlay).
pub const E1850: Sig = Sig {
    len: 297,
    hash: 0x4099_13df_fddd_f9b6,
    calls: &[],
    imports: &[],
};

/// 0x10001de0 (an overlay table entry, no arguments): the part drawn at
/// a point copied as it is ([`draw`], [`End::Plain`]). Returns 1.
pub fn e1de0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s = Start {
        gp: 0x1,
        cam: 0x7,
        item: 0xd,
        t723: 0x12,
        t724: 0x1f,
        m: 0x25,
        t258: 0x3a,
        part: 0x6d,
    };
    let axes = [
        Axis::copy(0x35, 0x42),
        Axis::copy(0x4a, 0x52),
        Axis::copy(0x5a, 0x62),
    ];
    Ret::eax(draw(g, img, fr.at, &s, &axes, End::Plain(0x34), &[]))
}

/// 0x10001de0 (233 bytes, 1 overlay).
pub const E1DE0: Sig = Sig {
    len: 233,
    hash: 0xa575_58f4_ba13_3301,
    calls: &[],
    imports: &[],
};

/// Starts a part (the copies at 0x10001580, 0x100017b0, 0x10001980,
/// 0x10001d50): `*G_P = ITEM`, slot 941 `(o[4])` with `o = *G_O` gives
/// the program table, and the `pppStartPart` wrapper starts the part
/// (`ecx`) with `(0x1000, table[index / 4], 1)`; then three 16-byte
/// blocks `(src, dst)` operands are copied. `ops`: the operands of
/// `G_P`, `ITEM`, `G_O`, 941's table, the part, and the wrapper's `rel32`
/// (`+0x01`, `+0x07`, `+0x0c`, `+0x14`, `+0x2d`, `+0x32` in the copies
/// without a cookie). Returns the words left on the stack (the wrapper's
/// three, then 941's).
fn start_part<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ops: [u32; 6],
    index: u32,
    blocks: [(u32, u32); 3],
) -> Vec<u32> {
    let w = |off: u32| img.word(f + off);
    let [gp, item, go, t941, part, call] = ops;
    let gp = g.read::<u32>(w(gp));
    g.write::<u32>(gp, w(item));
    let o = g.read::<u32>(w(go));
    let o10 = g.read::<u32>(o.wrapping_add(0x10));
    let t = slot_target(g, w(t941), 941);
    let table = g.call(t, 0, &[o10]).eax;
    let program = g.read::<u32>(table.wrapping_add(index));
    let wrapper = img.rel32_target(f + call);
    crate::opu_parts::start_part(g, img, wrapper, w(part), 0x1000, program, 1);
    for (src, dst) in blocks {
        for i in 0..4 {
            let v = g.read::<u32>(w(src) + 4 * i);
            g.write::<u32>(w(dst) + 4 * i, v);
        }
    }
    vec![program, 1, 0, o10]
}

/// [`start_part`]'s operands in the copies without a cookie.
const START_OPS: [u32; 6] = [0x1, 0x7, 0xc, 0x14, 0x2d, 0x32];

/// The two `(slot 268)` copies 0x100017b0 and 0x10001980: the part
/// started ([`start_part`], program `index`), slot 268 `(*G_A, 1, V)`
/// fills the point `V` (operand `+0x61`) and its `y` moves by `K`
/// (`+0x88`; `plus`: up). Returns 1.
fn start_moved<G: Game>(g: &mut G, img: &Image, f: u32, index: u32, plus: bool) -> u32 {
    let w = |off: u32| img.word(f + off);
    let blocks = [(0x39, 0x45), (0x4d, 0x55), (0x5c, 0x70)];
    let below = start_part(g, img, f, START_OPS, index, blocks);
    let t = slot_target(g, w(0x3e), 268);
    let mut args = vec![g.read::<u32>(w(0x69)), 1, w(0x61)];
    args.extend(below);
    // call width: 7
    g.call(t, 0, &args);
    Axis {
        src: 0x80,
        steps: if plus {
            &[(true, 0x88)]
        } else {
            &[(false, 0x88)]
        },
        dst: 0x98,
    }
    .run(g, img, f);
    1
}

/// 0x100017b0 (an overlay table entry, no arguments): [`start_moved`]
/// with program 1, `y` up.
pub fn e17b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(start_moved(g, img, fr.at, 4, true))
}

/// 0x100017b0 (157 bytes, 1 overlay).
pub const E17B0: Sig = Sig {
    len: 157,
    hash: 0xf5e5_e450_7a52_1d30,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// 0x10001980 (an overlay table entry, no arguments): [`start_moved`]
/// with program 2, `y` down.
pub fn e1980<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(start_moved(g, img, fr.at, 8, false))
}

/// 0x10001980 (157 bytes, 1 overlay).
pub const E1980: Sig = Sig {
    len: 157,
    hash: 0x8df6_6998_795f_8160,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// 0x10001d50 (an overlay table entry, no arguments): the part started
/// ([`start_part`], program 3), then slot 237 `(*G_A, 0x55, 0, V)` fills
/// the point `V` (operand `+0x52`). Returns 1.
pub fn e1d50<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let blocks = [(0x39, 0x45), (0x4d, 0x5a), (0x61, 0x72)];
    let below = start_part(g, img, f, START_OPS, 0xc, blocks);
    let t = slot_target(g, w(0x3e), 237);
    let mut args = vec![g.read::<u32>(w(0x6b)), 0x55, 0, w(0x52)];
    args.extend(below);
    // call width: 8
    g.call(t, 0, &args);
    Ret::eax(1)
}

/// 0x10001d50 (135 bytes, 1 overlay).
pub const E1D50: Sig = Sig {
    len: 135,
    hash: 0xe4fe_74e0_506f_db17,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// 0x10001580 (an overlay table entry, no arguments; a cookie frame):
/// the part started ([`start_part`], program 7), slot 268 `(*G_A, 6, a)`
/// and `(*G_A, 4, b)` fill two local points (`ebp - 0x14`, `ebp -
/// 0x24`); the point `(-217.27, a.y - K, 104.63)` goes to the globals at
/// operands `+0xbb`, `+0xc7`, `+0xcd`. Returns 1.
pub fn e1580<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (a, b) = (ebp - 0x14, ebp - 0x24);
    // the start's operands sit 0x10 further on (the cookie)
    let ops = [0x11, 0x17, 0x1c, 0x24, 0x3d, 0x42];
    let blocks = [(0x49, 0x50), (0x58, 0x69), (0x70, 0x7f)];
    let below = start_part(g, img, f, ops, 0x1c, blocks);
    let ga = g.read::<u32>(w(0x78));
    let t = slot_target(g, w(0x61), 268);
    let mut args = vec![ga, 6, a];
    args.extend(below);
    // call width: 10
    g.call(t, 0, &args);
    let ga = g.read::<u32>(w(0x98));
    let t = slot_target(g, w(0x90), 268);
    let mut args2 = vec![ga, 4, b];
    args2.extend(args);
    // call width: 10
    g.call(t, 0, &args2);
    let y = sub(g.read::<f32>(a + 4), g.read::<f32>(w(0xad)));
    g.write::<u32>(w(0xbb), 0xc359_4610);
    g.write::<f32>(w(0xc7), y);
    g.write::<u32>(w(0xcd), 0x42d1_4096);
    Ret::eax(1)
}

/// 0x10001580 (227 bytes, 1 overlay).
pub const E1580: Sig = Sig {
    len: 227,
    hash: 0xee00_cc48_aff9_16b0,
    calls: &[
        (0x42, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0xdb, Callee::Any),
    ],
    imports: &[],
};

/// The effect's start-up (0x10001f10, an overlay table entry, `(arg)`):
/// slot 803 `(0x1000000, 0x10)` gives the work buffer, cut into three
/// 512 KiB parts (globals at operands `+0x1b`, `+0x25`, `+0x2f`); `arg`
/// to `+0x37`; the file name (`+0x41`) to `+0x4d` and slot 718, slot 222
/// `(name, 0, 1000, 0x2000)`, slot 719 `(name, 8, 0)` gives the data `d`
/// (`+0x98`); slot 711 `(d, PARTS, 0)` with the game flag `*G_LOCK` 1;
/// slot 710 `(ITEM, d, buf, 0x80000)`; three slot 941 `(d[5 + k])` fill
/// the program tables. Five 0x110-byte part blocks are cleared and get
/// their defaults (a callback, flags), slot 260 `(0)` and `(0x14)` two
/// handles, slot 719 `(name, 4, 0)` a second data `e`; a 0x30-byte block
/// cleared (`+0x2` 0x80, `+0x4` 0x80808080); then two 0x5c-byte records
/// (from operand `+0x2e1` up to `+0x362`) each get 250 cleared entries of
/// 0x94 bytes from the buffer `+0x2cc` on (the stack argument follows the
/// cursor, as the original keeps it there). Every call passes the words
/// on the original's stack. Returns 1.
#[allow(clippy::too_many_lines)]
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s803 = slot_target(g, w(0x4), 803);
    let mut stack = vec![0x0100_0000, 0x10];
    let buf = g.call(s803, 0, &stack).eax;
    g.write::<u32>(w(0x1b), buf);
    g.write::<u32>(w(0x25), buf.wrapping_add(0x8_0000));
    g.write::<u32>(w(0x2f), buf.wrapping_add(0x10_0000));
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x37), arg);
    let s718 = slot_target(g, w(0x3c), 718);
    stack.insert(0, w(0x41));
    g.write::<u32>(w(0x4d), w(0x51));
    // call width: 17
    g.call(s718, 0, &stack);
    let s222 = slot_target(g, w(0x58), 222);
    let name = g.read::<u32>(w(0x70));
    stack.splice(0..0, [name, 0, 0x3e8, 0x2000]);
    // call width: 17
    g.call(s222, 0, &stack);
    let s719 = slot_target(g, w(0x77), 719);
    let name = g.read::<u32>(w(0x87));
    stack.splice(0..0, [name, 8, 0]);
    // call width: 17
    let d = g.call(s719, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x8f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x98), d);
    g.write::<u32>(lock, 1);
    let s711 = slot_target(g, w(0xa3), 711);
    stack.splice(0..0, [d, w(0x9d), 0]);
    // call width: 17
    g.call(s711, 0, &stack);
    let lock = g.read::<u32>(w(0xb6));
    let b = g.read::<u32>(w(0xc1));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xc9));
    let s710 = slot_target(g, w(0xce), 710);
    stack.splice(0..0, [w(0xd3), d, b, 0x8_0000]);
    // call width: 17
    g.call(s710, 0, &stack);
    // add esp, 0x44
    let mut stack = Vec::new();
    for (k, (dop, tab, dst)) in (0u32..).zip([
        (0xe0u32, 0xebu32, 0xf8u32),
        (0xfd, 0x105, 0x112),
        (0x117, 0x11f, 0x138),
    ]) {
        let d = g.read::<u32>(w(dop));
        let x = g.read::<u32>(d.wrapping_add(0x14 + 4 * k));
        stack.insert(0, x);
        let s941 = slot_target(g, w(tab), 941);
        // call width: 17
        let r = g.call(s941, 0, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    // memset(BLOCK, 0, 0x110) then the block's defaults (written between
    // the calls; each lies in a block cleared before), the memsets' words
    // left on the stack
    let zero = [0u8; 0x110];
    let blocks: [(u32, &[(u32, u32)]); 5] = [
        (0x133, &[]),
        (0x171, &[(0x152, 1), (0x15c, 1), (0x166, 1)]),
        (0x182, &[(0x192, 2), (0x19c, 2), (0x1a6, 1), (0x1b0, 1)]),
        (0x1c5, &[(0x1d5, 2), (0x1df, 2), (0x1e9, 1), (0x1f3, 1)]),
        (0x208, &[(0x218, 2), (0x222, 2), (0x22c, 1), (0x236, 1)]),
    ];
    // each block's callback word, set before the next clear
    let callbacks = [
        (0x148u32, 0x14cu32),
        (0x188, 0x18c),
        (0x1cb, 0x1cf),
        (0x20e, 0x212),
    ];
    for (k, (dst, defaults)) in blocks.into_iter().enumerate() {
        if k > 0 {
            let (at, v) = callbacks[k - 1];
            g.write::<u32>(w(at), w(v));
        }
        for &(at, v) in defaults {
            g.write::<u32>(w(at), v);
        }
        g.write_bytes(w(dst), &zero);
        stack.splice(0..0, [w(dst), 0, 0x110]);
    }
    // add esp, 0x48
    g.write::<u32>(w(0x24d), w(0x251));
    for o in [0x257, 0x261, 0x26b] {
        g.write::<u32>(w(o), 1);
    }
    let s260 = slot_target(g, w(0x244), 260);
    let r = g.call(s260, 0, &[0]).eax;
    g.write::<u32>(w(0x27e), r);
    let s260 = slot_target(g, w(0x283), 260);
    let r = g.call(s260, 0, &[0x14, 0]).eax;
    g.write::<u32>(w(0x292), r);
    let s719 = slot_target(g, w(0x297), 719);
    let name = g.read::<u32>(w(0x2a7));
    let e = g.call(s719, 0, &[name, 4, 0, 0x14, 0]).eax;
    g.write::<u32>(w(0x2b7), e);
    g.write_bytes(w(0x2b2), &[0u8; 0x30]);
    g.write::<u16>(w(0x2c7), 0x80);
    g.write::<u32>(w(0x2d5), 0x8080_8080);
    let mut p = w(0x2cc);
    let arg_at = fr.esp.wrapping_add(4);
    g.write::<u32>(arg_at, p);
    let mut rec = w(0x2e1);
    let entry = [0u8; 0x94];
    loop {
        let r = rec.wrapping_sub(8);
        g.write_bytes(r, &[0u8; 0x5c]);
        g.write::<u32>(r, 0xfa);
        g.write::<u32>(rec.wrapping_sub(4), p);
        for o in [0u32, 8, 0x10] {
            g.write::<u32>(rec.wrapping_add(o), 0);
        }
        let mut e = p;
        for _ in 0..0xfa {
            g.write_bytes(e, &entry);
            g.write::<u32>(e, 0);
            e = e.wrapping_add(0x94);
        }
        g.write::<u32>(rec, 1);
        g.write::<u32>(rec.wrapping_add(4), 0x3dcc_cccd);
        g.write::<u32>(rec.wrapping_add(8), 0);
        g.write::<u32>(rec.wrapping_add(0x10), 0);
        p = p.wrapping_add(0x9088);
        g.write::<u32>(arg_at, p);
        rec = rec.wrapping_add(0x5c);
        if rec.cast_signed() >= w(0x362).cast_signed() {
            break;
        }
    }
    for o in [0x36f, 0x375, 0x37b] {
        g.write::<u32>(w(o), 0);
    }
    Ret::eax(1)
}

/// [`start_up`] (902 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 902,
    hash: 0x02dc_12a3_5072_c554,
    calls: &[
        (0x13d, Callee::Import("memset")),
        (0x176, Callee::Import("memset")),
        (0x1b9, Callee::Import("memset")),
        (0x1fc, Callee::Import("memset")),
        (0x23f, Callee::Import("memset")),
        (0x2bc, Callee::Import("memset")),
        (0x2f0, Callee::Import("memset")),
        (0x329, Callee::Import("memset")),
    ],
    imports: &[],
};
