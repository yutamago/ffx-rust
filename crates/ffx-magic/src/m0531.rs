//! `magic_0531.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::start_part;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's start (0x10001140, `(obj)`): program 0 started
/// ([`start_part`]), slot 268 `(*G_H, 16, obj + 0x40)` places it and
/// slot 268 `(*G_H, 16, P)` fills the light's point `P` (operand
/// `+0x58`; the earlier words left on the stack); the scales 0.23
/// (`+0x6c` 1.0), `+0x50..` 0, `+0x5c` 1; when the flag at `+0xa9` is
/// set, slot 393 `(*G_L, P, 0x100)`. Returns 1.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x35], 0, &[]);
    let t = slot_target(g, w(0x3e), 268);
    let mut stack = vec![g.read::<u32>(w(0x46)), 0x10, obj + 0x40];
    stack.extend(below);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x53), 268);
    let h = g.read::<u32>(w(0x60));
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
    stack.splice(0..0, [h, 0x10, w(0x58)]);
    // call width: 10
    g.call(t, 0, &stack);
    if g.read::<u32>(w(0xa9)) != 0 {
        let t = slot_target(g, w(0xb2), 393);
        let l = g.read::<u32>(w(0xc8));
        g.call(t, 0, &[l, w(0xc2), 0x100]);
    }
    Ret::eax(1)
}

/// 0x10001140 (216 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 216,
    hash: 0x315c_d72c_9bbb_81f4,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame (0x10001220, `(obj)`; a /GS frame): nothing while
/// the DLL's flag (operand `+0x15`) is 0 (returns 0); else the scales
/// `+0x60..` 0.23 (`+0x6c` 1.0), the camera matrix `M` (slot 210
/// `(*G_H, &M)`) scaled by 1000 (slot 235 `(&S)` with the diagonal 1000,
/// slot 231 `(&M, &M, &S)`), slot 325 `(&V, POINT)`, then by 0.23 the
/// same way, slot 230 `(*G_A, &M)`; slots 231 `(*G_X, *G_Y, *G_A)` and
/// 230 `(*G_Z, *G_X)`, `**G_PART` = the DLL's part block and slot 712
/// `(obj, 0x30)` with `**G_LOCK` 1 around it. Locals: `M` `ebp - 0x84`,
/// `V` `- 0x54`, `S` `- 0x44`. Each call sees the words the earlier ones
/// left on the stack. Returns 1 when slot 712 answered 0, else 0.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x15)) == 0 {
        return Ret::eax(0);
    }
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v, sm) = (
        ebp.wrapping_sub(0x84),
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
        // call width: 12 at most
        g.call(t, 0, stack);
    };
    scale(g, &mut stack, 0x6d, 0x86, 0x447a_0000);
    let t = slot_target(g, w(0xb1), 325);
    stack.splice(0..0, [v, w(0xab)]);
    // call width: 8
    g.call(t, 0, &stack);
    scale(g, &mut stack, 0xc2, 0xdb, 0x3e6b_851f);
    let t = slot_target(g, w(0x104), 230);
    let a = g.read::<u32>(w(0x10a));
    stack.splice(0..0, [a, m]);
    // call width: 14
    g.call(t, 0, &stack);
    let a = g.read::<u32>(w(0x118));
    let t = slot_target(g, w(0x11d), 231);
    let y = g.read::<u32>(w(0x123));
    let x = g.read::<u32>(w(0x12f));
    stack.splice(0..0, [x, y, a]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x136), 230);
    let x = g.read::<u32>(w(0x145));
    let z = g.read::<u32>(w(0x14b));
    let mut stack = vec![z, x];
    g.call(t, 0, &stack);
    let pp = g.read::<u32>(w(0x152));
    g.write::<u32>(pp, w(0x15a));
    let lock = g.read::<u32>(w(0x15f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x16d), 712);
    stack.splice(0..0, [obj, 0x30]);
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x17b));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// [`frame`] (413 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 413,
    hash: 0x3584_c824_e4e7_eda1,
    calls: &[
        // __security_check_cookie
        (0x29, Callee::Any),
        (0x195, Callee::Any),
    ],
    imports: &[],
};

/// The effect's start (0x100016c0, an overlay table entry, `(arg)`):
/// `magic_0530`'s head ([`start_head`](crate::m0530::start_head), the
/// same bytes); the actor's kind 0xb1 clears the DLL's choice word
/// (operand `+0x1fb`) and takes the second state block, 0xb9 sets it and
/// takes the first, others take the first when the word is set, else the
/// second ([`state_block`](crate::m0530::state_block)); then slot 367
/// `(*G_A, MODE, &arg, 0, 0, 0, 0, 0)` fills the stack word, whose bit 0
/// goes to the flag `+0x2b2`. Returns 1.
pub fn start_effect<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let kind = crate::m0530::start_head(g, img, fr);
    let first = match kind {
        0xb1 => {
            g.write::<u32>(w(0x1fb), 0);
            false
        }
        0xb9 => {
            g.write::<u32>(w(0x215), 1);
            true
        }
        _ => g.read::<u32>(w(0x2bf)) != 0,
    };
    if first {
        crate::m0530::state_block(
            g,
            img,
            f,
            0x225,
            [0x24e, 0x258, 0x262, 0x26c, 0x276],
            0x230,
            [0x23a, 0x244],
            false,
        );
    } else {
        crate::m0530::state_block(
            g,
            img,
            f,
            0x2d2,
            [0x2fb, 0x305, 0x30f, 0x319, 0x323],
            0x2dd,
            [0x2e7, 0x2f1],
            false,
        );
    }
    let word = fr.esp + 4;
    let mode = g.read::<u32>(w(0x291));
    let t = slot_target(g, w(0x296), 367);
    let a = g.read::<u32>(w(0x29c));
    g.call(t, 0, &[a, mode, word, 0, 0, 0, 0, 0]);
    let v = g.read::<u32>(word) & 1;
    g.write::<u32>(w(0x2b2), v);
    Ret::eax(1)
}

/// [`start_effect`] (816 bytes, 1 overlay).
pub const START_EFFECT: Sig = Sig {
    len: 816,
    hash: 0x1d64_7c70_7ad7_18e4,
    calls: &[
        (0x22a, Callee::Import("memset")),
        (0x2d7, Callee::Import("memset")),
    ],
    imports: &[],
};
