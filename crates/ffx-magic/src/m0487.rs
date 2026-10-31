//! `magic_0487.dll`'s own functions (besides the runtime and the shared
//! code): a part that flies along a normalized direction, the start-up
//! and the frame.

use crate::part_draw::{Camera, DrawOps, camera, draw_restart, start_part};
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The flight's start (0x10001170, an overlay table entry, no arguments;
/// an aligned frame): the part (operand `+0x32`) started with program 0
/// ([`start_part`]), two 16-byte blocks copied (`+0x3e` to `+0x55`,
/// `+0x5d` to `+0x65`), the speed (`+0x81`) 0.7 and the start point
/// (`+0x96..+0xbb`); the direction `D` (`+0x46`, `+0x4e`, `+0x74`)
/// scaled to the length `K / |D|` (`sqrt`) unless `|D|` is 0; the scale
/// (`+0x129`) 5. Returns 1.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = w(0x32);
    start_part(g, img, f, part, [0x7, 0xd, 0x12, 0x1a, 0x37], 0, &[]);
    for (src, dst) in [(0x3e, 0x55), (0x5d, 0x65)] {
        for i in 0..4 {
            let v = g.read::<u32>(w(src) + 4 * i);
            g.write::<u32>(w(dst) + 4 * i, v);
        }
    }
    let x = g.read::<f32>(w(0x46));
    let y = g.read::<f32>(w(0x4e));
    let z = g.read::<f32>(w(0x74));
    g.write::<u32>(w(0x81), 0x3f33_3333);
    for (op, v) in [
        (0x96u32, 0xc34e_7099u32),
        (0xa0, 0xc0c4_ca6f),
        (0xb1, 0xc2af_7f90),
        (0xbb, 0),
    ] {
        g.write::<u32>(w(op), v);
    }
    let sum = add(add(mul(y, y), mul(x, x)), mul(z, z));
    // _libm_sse2_sqrt_precise of the double, rounded back to f32
    #[allow(clippy::cast_possible_truncation)]
    let len = f64::from(sum).sqrt() as f32;
    // ucomiss len, 0; lahf; test ah, 0x44; jnp: skipped only when equal
    if len != 0.0 || len.is_nan() {
        let k = div(g.read::<f32>(w(0xe3)), len);
        for (src, dst) in [(0xef, 0xfb), (0x103, 0x10f), (0x117, 0x123)] {
            let v = mul(g.read::<f32>(w(src)), k);
            g.write::<f32>(w(dst), v);
        }
    }
    g.write::<u32>(w(0x129), 0x40a0_0000);
    Ret::eax(1)
}

/// 0x10001170 (314 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 314,
    hash: 0xa813_a309_31f1_64c9,
    calls: &[
        (0x37, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xc8, Callee::Import("_libm_sse2_sqrt_precise")),
    ],
    imports: &[],
};

/// The flight's step (0x100012b0, an overlay table entry, no
/// arguments): with the scale `s` (`+0x04`), the point `P` moves by `s *
/// D` in `x` and `z` (`P.x` at `+0x39`, `P.z` at `+0x29`, `D.x` `+0x14`,
/// `D.z` `+0x0c`) and goes to the part's position (`+0x74`, `y` -3,
/// `+0x54`; `w` 1.0); the scale grows by `K1`; the speed (`+0x5c`) by
/// `K2`, times `K3` the part's three scales. Then the part is drawn and
/// restarted when slot 712 says so ([`draw_restart`]). Returns 1.
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<f32>(w(0x4));
    let z = add(mul(g.read::<f32>(w(0xc)), s), g.read::<f32>(w(0x29)));
    let s2 = add(s, g.read::<f32>(w(0x31)));
    let x = add(mul(g.read::<f32>(w(0x14)), s), g.read::<f32>(w(0x39)));
    g.write::<u32>(w(0x40), 0xc040_0000);
    g.write::<f32>(w(0x4c), z);
    g.write::<f32>(w(0x54), z);
    let speed = add(g.read::<f32>(w(0x5c)), g.read::<f32>(w(0x64)));
    g.write::<f32>(w(0x6c), x);
    g.write::<f32>(w(0x74), x);
    g.write::<u32>(w(0x7a), 0x3f80_0000);
    g.write::<f32>(w(0x8b), speed);
    let scale = mul(speed, g.read::<f32>(w(0x93)));
    g.write::<f32>(w(0x9b), s2);
    for op in [0xa3, 0xab, 0xb3] {
        g.write::<f32>(w(op), scale);
    }
    let ops = DrawOps {
        t258: 0x19,
        a231: [0xc1, 0xcc, 0xd8],
        t231: 0xc6,
        a230: [0xe0, 0xeb],
        t230: 0xe5,
        lock: 0xf9,
        gp: 0xfe,
        item: 0x10a,
        t712: 0x10f,
        lock2: 0x12c,
        gp2: 0x13b,
        item2: 0x146,
        t713: 0x14b,
        start: [0x158, 0x15e, 0x163, 0x16b, 0x188],
        index: 0,
        lock3: 0x18d,
    };
    // the part's immediate (+0x83, +0x124, +0x140, +0x183: one address)
    draw_restart(g, img, f, w(0x83), &ops);
    Ret::eax(1)
}

/// 0x100012b0 (428 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 428,
    hash: 0xb0e6_1e49_695b_d0a6,
    calls: &[(0x188, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's start-up (0x10001490, an overlay table entry, `(arg)`):
/// `arg` to `+0x07`; slot 803 `(0x1000000, 0x10)` the buffer (`+0x21`,
/// and `+0x2b` half a MiB on); the name (`+0x35`) to `+0x41` and slot
/// 718, slot 222 `(name, 0, 1000, 0x2000)`, slot 719 `(name, 8, 0)` the
/// data `d` (`+0x8c`); slot 711 `(d, PARTS, 0)` with the game flag
/// `*G_LOCK` 1; slot 710 `(ITEM, d, buf, 0x80000)`; three slot 941
/// `(d[5 + k])` the program tables; two 0x110-byte timelines cleared and
/// given their defaults; slot 260 `(0)` and `(0x15)` two handles. Every
/// call passes the words on the original's stack. Returns 1.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x7), arg);
    let t = slot_target(g, w(0xc), 803);
    let mut stack = vec![0x0100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x21), buf);
    g.write::<u32>(w(0x2b), buf.wrapping_add(0x8_0000));
    let t = slot_target(g, w(0x30), 718);
    stack.insert(0, w(0x35));
    g.write::<u32>(w(0x41), w(0x45));
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x4c), 222);
    let name = g.read::<u32>(w(0x64));
    stack.splice(0..0, [name, 0, 0x3e8, 0x2000]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x6b), 719);
    let name = g.read::<u32>(w(0x7b));
    stack.splice(0..0, [name, 8, 0]);
    // call width: 17
    let d = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x83));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x8c), d);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x97), 711);
    stack.splice(0..0, [d, w(0x91), 0]);
    // call width: 17
    g.call(t, 0, &stack);
    let lock = g.read::<u32>(w(0xaa));
    let b = g.read::<u32>(w(0xb5));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xbd));
    let t = slot_target(g, w(0xc2), 710);
    stack.splice(0..0, [w(0xc7), d, b, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    // add esp, 0x44
    let mut stack = Vec::new();
    for (k, (dop, tab, dst)) in (0u32..).zip([
        (0xd4u32, 0xdfu32, 0xecu32),
        (0xf1, 0xf9, 0x106),
        (0x10b, 0x113, 0x12c),
    ]) {
        let d = g.read::<u32>(w(dop));
        stack.insert(0, g.read::<u32>(d.wrapping_add(0x14 + 4 * k)));
        let t = slot_target(g, w(tab), 941);
        // call width: 17
        let r = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    let zero = [0u8; 0x110];
    g.write_bytes(w(0x127), &zero);
    stack.splice(0..0, [w(0x127), 0, 0x110]);
    g.write::<u32>(w(0x143), w(0x147));
    for op in [0x14d, 0x157, 0x161] {
        g.write::<u32>(w(op), 1);
    }
    g.write_bytes(w(0x13d), &zero);
    stack.splice(0..0, [w(0x13d), 0, 0x110]);
    g.write::<u32>(w(0x170), w(0x174));
    g.write::<u32>(w(0x17a), 1);
    let t = slot_target(g, w(0x183), 260);
    g.write::<u32>(w(0x189), 1);
    g.write::<u32>(w(0x193), 1);
    stack.insert(0, 0);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x1a6), h);
    let t = slot_target(g, w(0x1ab), 260);
    stack.insert(0, 0x15);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x1bd), h);
    Ret::eax(1)
}

/// 0x10001490 (457 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 457,
    hash: 0x9ad1_60bd_d859_61aa,
    calls: &[
        (0x131, Callee::Import("memset")),
        (0x16a, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The effect's frame (0x10001730, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else the
/// matrices ([`camera`]), `*G_V = *G_W`, the trail of nine points
/// shifted one place (the oldest dropped) and slot 237 `(*G_H, 0, 0, T)`
/// fills the newest `T` (`+0x1e8`); slot 261 `(k)` for `k` = 0, 1 runs
/// the two timelines ([`timeline_run`](crate::sb_misc::timeline_run))
/// when it says nonzero; when `(2)` does, both stop
/// ([`timeline_stop`](crate::sb_misc::timeline_stop)) and slot 214
/// `(*G_E)` runs, else the frame count `+0x29e` goes up. Returns 1.
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
    for (src, dst) in [
        (0x4e, 0x56),
        (0x5e, 0x66),
        (0x6e, 0x76),
        (0x7e, 0x86),
        (0x8e, 0x96),
        (0x9e, 0xa6),
        (0xae, 0xb6),
        (0xbe, 0xc6),
        (0xce, 0xd6),
        (0xde, 0xe6),
        (0xee, 0xf6),
        (0xfe, 0x106),
        (0x10e, 0x116),
        (0x11e, 0x126),
        (0x12e, 0x136),
        (0x13e, 0x146),
        (0x14e, 0x156),
        (0x15e, 0x166),
        (0x16e, 0x17b),
        (0x183, 0x18b),
        (0x193, 0x19b),
        (0x1a3, 0x1ab),
        (0x1b3, 0x1bb),
        (0x1c3, 0x1cb),
        (0x1d3, 0x1db),
        (0x1e3, 0x1f0),
        (0x1f8, 0x20a),
    ] {
        let v = g.read::<u32>(w(src));
        g.write::<u32>(w(dst), v);
    }
    let t = slot_target(g, w(0x173), 237);
    let h = g.read::<u32>(w(0x202));
    stack.splice(0..0, [h, 0, 0, w(0x1e8)]);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x217), 261);
    stack.insert(0, 0);
    // call width: 7
    if g.call(t, 0, &stack).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x232), w(0x22d));
    }
    let t = slot_target(g, w(0x237), 261);
    if g.call(t, 0, &[1]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x252), w(0x24d));
    }
    let t = slot_target(g, w(0x257), 261);
    if g.call(t, 0, &[2]).eax == 0 {
        let n = w(0x29e);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop(g, w(0x26d));
    crate::sb_misc::timeline_stop(g, w(0x277));
    let t = slot_target(g, w(0x281), 214);
    let e = g.read::<u32>(w(0x287));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// 0x10001730 (680 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 680,
    hash: 0xa793_62f5_d42a_ef14,
    calls: &[
        (0x232, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x252, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x272, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x27c, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};
