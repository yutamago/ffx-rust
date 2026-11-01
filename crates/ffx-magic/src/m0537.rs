//! `magic_0537.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{Camera, camera, start_part};
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the part (0x10001000, internal: `ecx` = the part): program 0
/// started ([`start_part`]), the position `(0, 0, 0, 1)`, `+0x50..` 0,
/// `+0x5c` 1, a random scale `r / D * M + A` with `r` slot 324's low 15
/// bits (`+0x6c` 1.0). Returns `r`.
pub fn start<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    start_part(g, img, f, obj, [0x1, 0x8, 0xd, 0x17, 0x31], 0, &[]);
    let t = slot_target(g, w(0x36), 324);
    for (off, v) in [
        (0x40u32, 0u32),
        (0x44, 0),
        (0x48, 0),
        (0x4c, 0x3f80_0000),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
    ] {
        g.write::<u32>(obj + off, v);
    }
    let r = g.call(t, 0, &[]).eax & 0x7fff;
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let s = add(mul(div(cvt(r), k(g, 0x94)), k(g, 0x9c)), k(g, 0xa4));
    for off in [0x60, 0x64, 0x68] {
        g.write::<f32>(obj + off, s);
    }
    r
}

/// [`start`] (185 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 185,
    hash: 0xb334_28f4_23e5_b098,
    calls: &[(0x31, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// Draws the part (0x10001150, internal: `ecx` = the part): slot 258
/// `(part)`, slots 231 and 230 with the camera globals, `*G_P = ITEM` and
/// slot 712 `(part, 0x30)` with the game flag `*G_LOCK` 1 (restored
/// after; the earlier words left on the stack); when 712 says nonzero,
/// slot 713 `(part)` removes it and program 0 starts again
/// ([`start_part`]). Returns 1.
pub fn draw<G: Game>(g: &mut G, img: &Image, f: u32, part: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 258);
    let mut stack = vec![part];
    g.call(t, 0, &stack);
    for op in [0x14, 0x1f, 0x2b] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x19), 231);
    // call width: 4
    g.call(t, 0, &stack);
    for op in [0x33, 0x3e] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x38), 230);
    // call width: 6
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x4b));
    g.write::<u32>(gp, w(0x53));
    let lock = g.read::<u32>(w(0x58));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x66), 712);
    stack.splice(0..0, [part, 0x30]);
    // call width: 8
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x74));
    g.write::<u32>(lock, saved);
    if r != 0 {
        let gp = g.read::<u32>(w(0x82));
        g.write::<u32>(gp, w(0x89));
        let t = slot_target(g, w(0x8e), 713);
        g.call(t, 0, &[part]);
        start_part(g, img, f, part, [0x9b, 0xa1, 0xa6, 0xae, 0xc8], 0, &[part]);
    }
    1
}

/// [`draw`] (215 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 215,
    hash: 0x45e5_c524_97d1_cb4f,
    calls: &[(0xc8, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's frame (0x10001580, an overlay table entry, no
/// arguments): 0 when slot 253 says 0 or the flag at operand `+0x16` is
/// clear. Else the matrices ([`camera`]), `*G_V = *G_W`; when slot 261
/// `(0)` says nonzero the timeline runs
/// ([`timeline`](crate::sa_misc::timeline)); when `(1)` does it stops
/// ([`timeline_stop_global`](crate::sb_misc::timeline_stop_global)) and
/// slot 214 `(*G_E)` runs, else the frame count `+0xaf` goes up. Returns
/// 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 253);
    if g.call(t, 0, &[]).eax == 0 || g.read::<u32>(w(0x16)) == 0 {
        return Ret::eax(0);
    }
    let c = Camera {
        gp: 0x1e,
        cam: 0x24,
        item: 0x2a,
        t723: 0x2f,
        t724: 0x3c,
        m: 0x42,
    };
    let mut stack = camera(g, img, f, &c, &[]);
    let v = g.read::<u32>(w(0x50));
    let x = g.read::<u32>(w(0x55));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x5c), 261);
    stack.insert(0, 0);
    // call width: 3
    if g.call(t, 0, &stack).eax != 0 {
        crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x72));
    }
    let t = slot_target(g, w(0x77), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = w(0xaf);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop_global(g, img, img.rel32_target(f + 0x8d));
    let t = slot_target(g, w(0x92), 214);
    let e = g.read::<u32>(w(0x98));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// 0x10001580 (185 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 185,
    hash: 0x444d_7da5_bc5c_257d,
    calls: &[
        (0x72, Callee::Sig(&crate::sa_misc::TIMELINE)),
        (0x8d, Callee::Sig(&crate::sb_misc::TIMELINE_STOP_GLOBAL)),
    ],
    imports: &[],
};
