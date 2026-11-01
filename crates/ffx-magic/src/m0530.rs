//! `magic_0530.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's start (0x10001140, `(obj)`): program 0 started
/// ([`start_part`]), slot 268 `(*G_H, 16, obj + 0x40)` places it (the
/// start's words left on the stack), the scales 0.23 (`+0x6c` 1.0),
/// `+0x50..` 0, `+0x5c` 1; when the flag at operand `+0x58` is set, slot
/// 393 `(*G_L, obj + 0x40, 0x100)`. Returns 1.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x19, 0x36], 0, &[]);
    let t = slot_target(g, w(0x3b), 268);
    let mut args = vec![g.read::<u32>(w(0x4d)), 0x10, obj + 0x40];
    args.extend(below);
    // call width: 7
    g.call(t, 0, &args);
    let flag = g.read::<u32>(w(0x58));
    for (off, v) in [
        (0x60u32, 0x3e6b_851fu32),
        (0x64, 0x3e6b_851f),
        (0x68, 0x3e6b_851f),
        (0x6c, 0x3f80_0000),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
    ] {
        g.write::<u32>(obj + off, v);
    }
    if flag != 0 {
        let t = slot_target(g, w(0x98), 393);
        let l = g.read::<u32>(w(0xaa));
        g.call(t, 0, &[l, obj + 0x40, 0x100]);
    }
    Ret::eax(1)
}

/// 0x10001140 (188 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 188,
    hash: 0xdae0_c328_f07a_b3e3,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame (0x10001200, `(obj)`; a /GS frame): nothing while
/// the DLL's flag (operand `+0x15`) is 0 (returns 0); else the scales
/// `+0x60..` 0.23 (`+0x6c` 1.0), the camera matrix `M` (slot 210
/// `(*G_H, &M)`) scaled by 1000 (slot 235 `(&S)` with the diagonal 1000,
/// slot 231 `(&M, &M, &S)`), slot 268 `(*G_H, 16, &V)` and slot 325
/// `(&W, &V)`, then by 0.23 the same way, slot 230 `(*G_A, &M)`; slots
/// 231 `(*G_X, *G_Y, *G_A)` and 230 `(*G_Z, *G_X)`, `**G_PART` = the
/// DLL's part block and slot 712 `(obj, 0x30)` with `**G_LOCK` 1 around
/// it. Locals: `M` `ebp - 0x94`, `W` `- 0x64`, `V` `- 0x54`, `S` `-
/// 0x44`. Each call sees the words the earlier ones left on the stack.
/// Returns 1 when slot 712 answered 0, else 0.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x15)) == 0 {
        return Ret::eax(0);
    }
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, wv, v, sm) = (
        ebp.wrapping_sub(0x94),
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x54),
        ebp.wrapping_sub(0x44),
    );
    let t = slot_target(g, w(0x3a), 210);
    let h = g.read::<u32>(w(0x40));
    for (o, x) in [
        (0x60u32, 0x3e6b_851fu32),
        (0x64, 0x3e6b_851f),
        (0x68, 0x3e6b_851f),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(obj + o, x);
    }
    let mut stack = vec![h, m];
    g.call(t, 0, &stack);
    // the scale matrix's diagonal, then `M = M * S`
    let scale = |g: &mut G, stack: &mut Vec<u32>, t235: u32, t231: u32, k: u32| {
        let t = slot_target(g, w(t235), 235);
        stack.insert(0, sm);
        g.call(t, 0, stack);
        for o in [0x28u32, 0x14, 0] {
            g.write::<u32>(sm + o, k);
        }
        let t = slot_target(g, w(t231), 231);
        stack.splice(0..0, [m, m, sm]);
        // call width: 6 or 15
        g.call(t, 0, stack);
    };
    scale(g, &mut stack, 0x6d, 0x86, 0x447a_0000);
    let t = slot_target(g, w(0xac), 268);
    let h = g.read::<u32>(w(0xb4));
    stack.splice(0..0, [h, 0x10, v]);
    // call width: 9
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xc9), 325);
    stack.splice(0..0, [wv, v]);
    // call width: 11
    g.call(t, 0, &stack);
    scale(g, &mut stack, 0xda, 0xf3, 0x3e6b_851f);
    let t = slot_target(g, w(0x11c), 230);
    let a = g.read::<u32>(w(0x122));
    stack.splice(0..0, [a, m]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x12f), 231);
    let a = g.read::<u32>(w(0x13e));
    let y = g.read::<u32>(w(0x144));
    let x = g.read::<u32>(w(0x14a));
    let mut stack = vec![x, y, a];
    g.call(t, 0, &stack);
    let x = g.read::<u32>(w(0x152));
    let t = slot_target(g, w(0x157), 230);
    let z = g.read::<u32>(w(0x15d));
    stack.splice(0..0, [z, x]);
    g.call(t, 0, &stack);
    let pp = g.read::<u32>(w(0x16a));
    g.write::<u32>(pp, w(0x172));
    let lock = g.read::<u32>(w(0x177));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x184), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 7
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x193));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// [`frame`] (437 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 437,
    hash: 0x5ab1_844f_cdc9_3a48,
    calls: &[
        // __security_check_cookie
        (0x29, Callee::Any),
        (0x1ad, Callee::Any),
    ],
    imports: &[],
};

/// The start shared by `magic_0530`'s and `magic_0531`'s effect starts
/// (their first 0x1ee bytes, the same operands): slot 803 `(0x1000000,
/// 16)` the buffer (its address and `+0x80000` to two globals, the
/// argument to a third), the name global set and slot 718 `(NAME)`,
/// slots 222 and 221 `(*G_N, 0, 1000, 0x2000)`, three handles from slot
/// 719 `(*G_N, 6, 0)`, `(*G_N, 6, 1)`, `(*G_N, 8, 0)`, slot 711 `(h3,
/// PROG, 0)` under `**G_LOCK` 1 and slot 710 `(BLOCK, *G_H3, *G_BUF,
/// 0x80000)`; the three program tables (slot 941 `(h3[0x14 + 4i])`), slot
/// 168 `(arg)` the actor, slot 215 its model, slot 218 `(arg, &arg)` its
/// mode byte (through the stack word) and slot 215 its model; slot 415
/// `(actor)` answers the actor's kind, which this returns. Each call sees
/// the words the earlier ones left on the stack, up to the two clean-ups.
pub(crate) fn start_head<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
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
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    for (tab, slot, op) in [(0x4cu32, 222u32, 0x64u32), (0x6b, 221, 0x83)] {
        let t = slot_target(g, w(tab), slot);
        let n = r(g, op);
        stack.splice(0..0, [n, 0, 0x3e8, 0x2000]);
        // call width: 11 at most
        g.call(t, 0, &stack);
    }
    for (tab, op, b, dst) in [(0x8au32, 0x9au32, 0u32, 0xa1u32), (0xa6, 0xb6, 1, 0xc0)] {
        let t = slot_target(g, w(tab), 719);
        let n = r(g, op);
        stack.splice(0..0, [n, 6, b]);
        // call width: 17 at most
        let h = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), h);
    }
    let t = slot_target(g, w(0xc5), 719);
    let n = r(g, 0xd5);
    let mut stack = vec![n, 8, 0];
    let h3 = g.call(t, 0, &stack).eax;
    let lock = r(g, 0xdd);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xe6), h3);
    let t = slot_target(g, w(0xf1), 711);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [h3, w(0xeb), 0]);
    // call width: 6
    g.call(t, 0, &stack);
    let lock = r(g, 0x104);
    let b = r(g, 0x10f);
    g.write::<u32>(lock, saved);
    let h = r(g, 0x117);
    let t = slot_target(g, w(0x11c), 710);
    stack.splice(0..0, [w(0x121), h, b, 0x8_0000]);
    // call width: 10
    g.call(t, 0, &stack);
    for (k, (hop, tab, dst)) in [
        (0x12eu32, 0x136u32, 0x143u32),
        (0x148, 0x150, 0x15d),
        (0x162, 0x16a, 0x17d),
    ]
    .into_iter()
    .enumerate()
    {
        let h = r(g, hop);
        let x = g.read::<u32>(h + 0x14 + 4 * k as u32);
        let t = slot_target(g, w(tab), 941);
        stack.insert(0, x);
        // call width: 13 at most
        let v = g.call(t, 0, &stack).eax;
        if k == 2 {
            let a = r(g, 0x178);
            stack.insert(0, a);
        }
        g.write::<u32>(w(dst), v);
    }
    let t = slot_target(g, w(0x182), 168);
    // call width: 14
    let actor = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x18f), actor);
    let t = slot_target(g, w(0x195), 215);
    stack.insert(0, actor);
    // call width: 15
    let m = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x1a2), m);
    let word = fr.esp + 4;
    let t = slot_target(g, w(0x1ab), 218);
    let a = r(g, 0x1b1);
    stack.splice(0..0, [a, word]);
    // call width: 17
    let p = g.call(t, 0, &stack).eax;
    let mode = u32::from(g.read::<u8>(p));
    g.write::<u32>(w(0x1c4), mode);
    let t = slot_target(g, w(0x1ca), 215);
    let mut stack = vec![mode];
    let m = g.call(t, 0, &stack).eax;
    let a = r(g, 0x1d8);
    g.write::<u32>(w(0x1dd), m);
    let t = slot_target(g, w(0x1e2), 415);
    stack.insert(0, a);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack).eax
}

/// One of the start's two state blocks: 0x110 bytes at operand `clear`
/// zeroed (`memset`), the five flags `ones` 1, the callback word
/// `cb` the immediate after it, the two counts `fives` 5 (the flags first
/// or last, as the DLL orders them).
#[allow(clippy::too_many_arguments)] // the block's operands
pub(crate) fn state_block<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    clear: u32,
    ones: [u32; 5],
    cb: u32,
    fives: [u32; 2],
    ones_first: bool,
) {
    let w = |off: u32| img.word(f + off);
    g.write_bytes(w(clear), &[0u8; 0x110]);
    let set_ones = |g: &mut G| {
        for op in ones {
            g.write::<u32>(w(op), 1);
        }
    };
    if ones_first {
        set_ones(g);
    }
    g.write::<u32>(w(cb), w(cb + 4));
    for op in fives {
        g.write::<u32>(w(op), 5);
    }
    if !ones_first {
        set_ones(g);
    }
}

/// The effect's start (0x100016c0, an overlay table entry, `(arg)`):
/// [`start_head`]; the actor's kind 0xb1 clears the DLL's choice word
/// (operand `+0x202`), 0xb9 sets it to 1 (others keep it); slot 367
/// `(*G_A, MODE, &arg, 0, 0, 0, 0, 0)` fills the stack word, whose bit 2
/// goes to the flag `+0x25f`; the choice word picks one of the two state
/// blocks ([`state_block`]). Returns 1.
pub fn start_effect<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let kind = start_head(g, img, fr);
    if kind == 0xb1 {
        g.write::<u32>(w(0x202), 0);
    } else {
        let c = if kind == 0xb9 {
            1
        } else {
            g.read::<u32>(w(0x20d))
        };
        g.write::<u32>(w(0x21b), c);
    }
    let word = fr.esp + 4;
    let mode = g.read::<u32>(w(0x22f));
    let t = slot_target(g, w(0x234), 367);
    let a = g.read::<u32>(w(0x23a));
    g.call(t, 0, &[a, mode, word, 0, 0, 0, 0, 0]);
    let v = (g.read::<u32>(word).cast_signed() >> 2).cast_unsigned() & 1;
    let choice = g.read::<u32>(w(0x254));
    g.write::<u32>(w(0x25f), v);
    if choice == 0 {
        state_block(
            g,
            img,
            f,
            0x268,
            [0x276, 0x27c, 0x282, 0x288, 0x28e],
            0x296,
            [0x2a0, 0x2aa],
            true,
        );
    } else {
        state_block(
            g,
            img,
            f,
            0x2b6,
            [0x2c4, 0x2ca, 0x2d0, 0x2d6, 0x2dc],
            0x2e4,
            [0x2ee, 0x2f8],
            true,
        );
    }
    Ret::eax(1)
}

/// [`start_effect`] (771 bytes, 1 overlay).
pub const START_EFFECT: Sig = Sig {
    len: 771,
    hash: 0xe351_ce48_0730_e13f,
    calls: &[
        (0x26d, Callee::Import("memset")),
        (0x2bb, Callee::Import("memset")),
    ],
    imports: &[],
};
