//! `magic_0292.dll`'s own functions, and `magic_0293`'s kin of its draw
//! (besides the runtime, the object host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The draw both DLLs open with (operands from `f`): slot 258 `(OBJ)`
/// (the object `+0x0d`), slot 231 `(*D, *E, *C)` (`+0x2c`, `+0x20`,
/// `+0x15`), slot 230 `(*B, *D)` (`+0x3f`, `+0x34`); the game flag `*L`
/// (`+0x4d`) 1 and the pointer `*P` (`+0x52`) its block (`+0x5e`); then
/// slot 712 `(OBJ, 0x30)` with `*L` 1 around it (its value before: 1).
/// Returns slot 712's `eax` and the old flag.
fn draw<G: Game>(g: &mut G, img: &Image, f: u32, p: &mut Pushes) -> (u32, u32) {
    let w = |off: u32| img.word(f + off);
    p.push(w(0x0d));
    let s = slot_target(g, w(0x01), 0x408 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let c = g.read::<u32>(w(0x15));
    let e = g.read::<u32>(w(0x20));
    let d = g.read::<u32>(w(0x2c));
    p.push_all(&[c, e, d]);
    let s = slot_target(g, w(0x1a), 0x39c / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let d = g.read::<u32>(w(0x34));
    let b = g.read::<u32>(w(0x3f));
    p.push_all(&[d, b]);
    let s = slot_target(g, w(0x39), 0x398 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let lock = g.read::<u32>(w(0x4d));
    let ptr = g.read::<u32>(w(0x52));
    g.write::<u32>(lock, 1);
    g.write::<u32>(ptr, w(0x5e));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    p.push_all(&[0x30, w(0x78)]);
    let s = slot_target(g, w(0x63), 0xb20 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let r = p.call(g, s, 0).eax;
    (r, saved)
}

/// A part draw (`magic_0293` 0x10001070, no arguments): [`draw`]; the
/// flag gets its old value and then 0. Returns 1 when slot 712 answered
/// 0.
pub fn draw_0293<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let (r, saved) = draw(g, img, f, &mut Pushes::default());
    let lock = g.read::<u32>(img.word(f + 0x80));
    g.write::<u32>(lock, saved);
    g.write::<u32>(lock, 0);
    Ret::eax(u32::from(r == 0))
}

/// [`draw_0293`] (157 bytes, 1 overlay).
pub const DRAW_0293: Sig = Sig {
    len: 157,
    hash: 0xe5e4_ea71_fe9a_e7df,
    calls: &[],
    imports: &[],
};

/// The part start both functions here share (operands from `at`): the
/// pointer `*P` its block (`+0`/`+6`), slot 941 `((*G)[0x10])` (`G`
/// `+0xb`) gives the program list and its first program starts on the
/// object (`+0x2b`; [`start_part`](crate::opu_parts::start_part) `(obj,
/// 0x1000, P, 1)`, the `rel32` at `+0x30`).
fn start<G: Game>(g: &mut G, img: &Image, at: u32, p: &mut Pushes) {
    let w = |off: u32| img.word(at + off);
    let ptr = g.read::<u32>(w(0));
    g.write::<u32>(ptr, w(6));
    let gp = g.read::<u32>(w(0x0b));
    p.push(g.read::<u32>(gp + 0x10));
    let s = slot_target(g, w(0x13), 0xeb4 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let list = p.call(g, s, 0).eax;
    let prog = g.read::<u32>(list);
    p.push_all(&[0, 1, prog]);
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(at + 0x30),
        w(0x2b),
        0x1000,
        prog,
        1,
    );
}

/// A part draw (`magic_0292` 0x10001110, no arguments): [`draw`], the
/// flag back to its old value; when slot 712 answered non-zero the part
/// is freed (slot 713 `(OBJ)`, the pointer `*P` set before and after) and
/// started again ([`start`]); the flag ends at 0. Returns 1.
pub fn draw_restart<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (r, saved) = draw(g, img, f, &mut Pushes::default());
    let lock = g.read::<u32>(w(0x80));
    g.write::<u32>(lock, saved);
    if r == 0 {
        g.write::<u32>(lock, 0);
        return Ret::eax(1);
    }
    let ptr = g.read::<u32>(w(0x8f));
    g.write::<u32>(ptr, w(0x9a));
    let mut p = Pushes::default();
    p.push(w(0x94));
    let s = slot_target(g, w(0x9f), 0xb24 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    start(g, img, f + 0xac, &mut p);
    let lock = g.read::<u32>(w(0xe1));
    g.write::<u32>(lock, 0);
    Ret::eax(1)
}

/// [`draw_restart`] (256 bytes, 1 overlay).
pub const DRAW_RESTART: Sig = Sig {
    len: 256,
    hash: 0xa6b5_f549_4081_776b,
    calls: &[(0xdc, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The part's start-up (`magic_0292` 0x10001000, no arguments, a /GS
/// frame of 0x44 bytes): [`start`]; three 16-byte constants into the
/// colour block (`+0x54`, `+0x64`, `+0x72`); slot 405 `(0)` and slot 406
/// `(h, &M)` (`M` on the frame, `ebp - 0x44`); slot 325 `(V, &T)` (`T`
/// `ebp - 0x14`, `V` `+0x9c`); `V`'s second word plus `K` (`+0xb4`);
/// three other constants into the colour block. Returns 1. Each slot call
/// passes the words the original leaves on the stack.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let mut p = Pushes::default();
    start(g, img, f + 0x11, &mut p);
    let copy = |g: &mut G, pairs: [(u32, u32); 3]| {
        for (src, dst) in pairs {
            for k in 0..4 {
                let v = g.read::<u32>(w(src) + 4 * k);
                g.write::<u32>(w(dst) + 4 * k, v);
            }
        }
    };
    copy(g, [(0x48, 0x54), (0x5c, 0x64), (0x6b, 0x72)]);
    p.push(0);
    let s = slot_target(g, w(0x4d), 0x654 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let h = p.call(g, s, 0).eax;
    p.push_all(&[ebp.wrapping_sub(0x44), h]);
    let s = slot_target(g, w(0x86), 0x658 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[ebp.wrapping_sub(0x14), w(0x9c)]);
    let s = slot_target(g, w(0x97), 0x514 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let v = add(g.read::<f32>(w(0xac)), g.read::<f32>(w(0xb4)));
    g.write::<f32>(w(0xc9), v);
    copy(g, [(0xd0, 0xd7), (0xdf, 0xe7), (0xee, 0xf5)]);
    Ret::eax(1)
}

/// [`start_up`] (258 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 258,
    hash: 0x5dc9_9b05_7e98_3ef0,
    calls: &[
        (0x41, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0xfa, Callee::Any),
    ],
    imports: &[],
};
