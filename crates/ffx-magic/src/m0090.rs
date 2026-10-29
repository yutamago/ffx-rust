//! `magic_0090.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// A task step (stored at 0x100068aa, `(task)`; the object at `task +
/// 4`): its clock `+0x2c4` advances by `D`; the stage `+0x2c8` (u16)
/// moves on once the clock passes the stage's time: stage 0 sets bit 0 of
/// `G_FLAGS` and clears `G_WAIT`; stage 1 calls slot 239 `(G_H, -5, 0)`;
/// stage 2 (which stays) sets the fade block (`1 - x` of eight values,
/// the scales 1.0, `G_MODE` bit 0 cleared and bit 2 set) and takes the
/// task's node out of its list. Returns `eax` as the original leaves it.
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let obj = g.read::<u32>(task + 4);
    let clock = add(g.read::<f32>(obj + 0x2c4), g.read::<f32>(w(0x16)));
    let stage = u32::from(g.read::<u16>(obj + 0x2c8));
    g.write::<f32>(obj + 0x2c4, clock);
    // comiss + jb: below (or unordered) waits
    let reached = |g: &G, op: u32| clock >= g.read::<f32>(w(op));
    let next = |g: &mut G| {
        let s = g.read::<u16>(obj + 0x2c8).wrapping_add(1);
        g.write::<u16>(obj + 0x2c8, s);
    };
    match stage {
        0 => {
            if reached(g, 0x1ab) {
                let v = g.read::<u32>(w(0x1b3)) | 1;
                g.write::<u32>(w(0x1b3), v);
                g.write::<u32>(w(0x1ba), 0);
                next(g);
            }
            Ret::eax(0)
        }
        1 => {
            if !reached(g, 0x17e) {
                return Ret::eax(0);
            }
            let s239 = slot_target(g, w(0x185), 239);
            let h = g.read::<u32>(w(0x195));
            let r = g.call(s239, fr.ecx, &[h, 0xffff_fffb, 0]).eax;
            next(g);
            Ret::eax(r)
        }
        2 => {
            if !reached(g, 0x43) {
                return Ret::eax(0);
            }
            fade(g, img, f);
            let node = g.read::<u32>(task);
            let prev = g.read::<u32>(node + 4);
            let next = g.read::<u32>(node + 8);
            g.write::<u32>(prev + 8, next);
            let next = g.read::<u32>(node + 8);
            let prev = g.read::<u32>(node + 4);
            g.write::<u32>(next + 4, prev);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            Ret::eax(prev)
        }
        _ => Ret::eax(stage.wrapping_sub(2)),
    }
}

/// Stage 2 of [`step`] (from `+0x4d`): every operand pair is a global
/// set from `X - global` (`X` the constant at `+0x51`) or a fixed word.
fn fade<G: Game>(g: &mut G, img: &Image, f: u32) {
    let w = |off: u32| img.word(f + off);
    let x = g.read::<f32>(w(0x51));
    let less = |g: &mut G, dst: u32, src: u32| {
        let v = sub(x, g.read::<f32>(w(src)));
        g.write::<f32>(w(dst), v);
    };
    less(g, 0x82, 0x5c);
    g.write::<u32>(w(0x61), 0);
    let mode = g.read::<u32>(w(0x66));
    g.write::<u32>(w(0x6c), ONE);
    g.write::<u32>(w(0x76), ONE);
    less(g, 0xbd, 0x8d);
    for o in [0x93u32, 0x9d, 0xa7, 0xb1] {
        g.write::<u32>(w(o), ONE);
    }
    less(g, 0xf1, 0xc8);
    g.write::<u32>(w(0xce), ONE);
    g.write::<u32>(w(0xd8), 0x3d4c_cccd);
    g.write::<u32>(w(0xe2), ONE);
    less(g, 0x10c, 0xfc);
    g.write::<u32>(w(0x104), (mode & !1) | 4);
    less(g, 0x11f, 0x117);
    less(g, 0x13a, 0x12a);
    less(g, 0x152, 0x132);
    let v = sub(g.read::<f32>(w(0x142)), g.read::<f32>(w(0x14a)));
    g.write::<f32>(w(0x15a), v);
}

/// 0x100066c0 (460 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 460,
    hash: 0xd67c_d139_27f1_813e,
    calls: &[],
    imports: &[],
};
