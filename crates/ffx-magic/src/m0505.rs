//! `magic_0505.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{
    Axis, Camera, Plain, Transform, camera, draw_plain, start_part_eax, transform,
};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the part at operand `part` with `program` ([`start_part_eax`],
/// `call` the wrapper's `rel32`), copies three 16-byte blocks `(src,
/// dst)` and writes the four words `(dst, value)`. Returns the wrapper's
/// `eax` (the lock pointer), which the original leaves.
fn start<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (part, call, program): (u32, u32, u32),
    blocks: [(u32, u32); 3],
    words: [(u32, u32); 4],
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let (_, eax) = start_part_eax(
        g,
        img,
        f,
        w(part),
        [0x1, 0x7, 0xc, 0x14, call],
        program,
        &[],
    );
    for (src, dst) in blocks {
        for i in 0..4 {
            let v = g.read::<u32>(w(src) + 4 * i);
            g.write::<u32>(w(dst) + 4 * i, v);
        }
    }
    for (op, v) in words {
        g.write::<u32>(w(op), v);
    }
    eax
}

/// 0x10001180 (an overlay table entry, no arguments): the first part
/// started with program 0 and its point `(0, -503.5, 0, 1)` ([`start`]).
pub fn e1180<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let eax = start(
        g,
        img,
        fr.at,
        (0x2c, 0x31, 0),
        [(0x38, 0x3f), (0x47, 0x4f), (0x56, 0x60)],
        [
            (0x66, 0),
            (0x70, 0xc3fb_c000),
            (0x7a, 0),
            (0x84, 0x3f80_0000),
        ],
    );
    Ret::eax(eax)
}

/// 0x10001180 (141 bytes, 1 overlay).
pub const E1180: Sig = Sig {
    len: 141,
    hash: 0x60c8_47ee_9c4c_d96c,
    calls: &[(0x31, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// 0x10001380 (an overlay table entry, no arguments): the second part
/// started with program 1 and its point `(0, -505, 0, 1)` ([`start`]).
pub fn e1380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let eax = start(
        g,
        img,
        fr.at,
        (0x2d, 0x32, 4),
        [(0x39, 0x40), (0x48, 0x50), (0x57, 0x61)],
        [
            (0x67, 0),
            (0x71, 0xc3fc_8000),
            (0x7b, 0),
            (0x85, 0x3f80_0000),
        ],
    );
    Ret::eax(eax)
}

/// 0x10001380 (142 bytes, 1 overlay).
pub const E1380: Sig = Sig {
    len: 142,
    hash: 0x7909_3900_1297_dded,
    calls: &[(0x32, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// 0x10001210 (an overlay table entry, no arguments): three slot 941
/// `(d[5 + k])` fill the program tables (`+0x16`, `+0x30`, `+0x50`;
/// `d = *G_D`), then the first part is drawn at its point ([`camera`],
/// [`transform`], [`draw_plain`]; the earlier words left on the stack).
/// Returns 1.
pub fn e1210<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut below = Vec::new();
    for (k, (dop, tab, dst)) in (0u32..).zip([
        (0x1u32, 0x9u32, 0x16u32),
        (0x1b, 0x23, 0x30),
        (0x35, 0x3d, 0x50),
    ]) {
        let d = g.read::<u32>(w(dop));
        below.insert(0, g.read::<u32>(d.wrapping_add(0x14 + 4 * k)));
        let t = slot_target(g, w(tab), 941);
        // call width: 3
        let r = g.call(t, 0, &below).eax;
        g.write::<u32>(w(dst), r);
    }
    let c = Camera {
        gp: 0x55,
        cam: 0x4b,
        item: 0x5b,
        t723: 0x60,
        t724: 0x6d,
        m: 0x73,
    };
    let mut stack = camera(g, img, f, &c, &below);
    let t = Transform {
        t258: 0x88,
        part: 0xbb,
        a231: [0xc3, 0xce, 0xda],
        t231: 0xc8,
        a230: [0xe2, 0xed],
        t230: 0xe7,
    };
    let axes = [
        Axis::copy(0x83, 0x90),
        Axis::copy(0x98, 0xa0),
        Axis::copy(0xa8, 0xb0),
    ];
    transform(g, img, f, &t, &axes, &mut stack);
    let p = Plain {
        gp: 0xfa,
        item: 0x102,
        lock: 0x107,
        part: 0x10c,
        t712: 0x117,
        lock2: 0x124,
    };
    draw_plain(g, img, f, &p, &stack);
    Ret::eax(1)
}

/// 0x10001210 (311 bytes, 1 overlay).
pub const E1210: Sig = Sig {
    len: 311,
    hash: 0xd513_9782_16d4_6b2f,
    calls: &[],
    imports: &[],
};

/// Registers the parts' models (0x100018d0, an overlay table entry, no
/// arguments; an aligned frame): for each of the `*G_N` records (0x94
/// bytes from `*G_R + 0x30`), slot 941 `(record[0])` maps it, slot 772
/// `(model, i)` turns it into a 64-bit handle (`edx:eax`) and slot 779
/// registers it, with the game flag `*G_LOCK` 1 around the call; then
/// slot 804 frees the buffer `*G_B`. Returns 1.
pub fn register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut rec = g.read::<u32>(w(0xa));
    let mut i = 0u32;
    if g.read::<i32>(w(0x14)) > 0 {
        rec = rec.wrapping_add(0x30);
        loop {
            let t = slot_target(g, w(0x21), 941);
            let x = g.read::<u32>(rec);
            let model = g.call(t, 0, &[x]).eax;
            let t = slot_target(g, w(0x35), 772);
            let h = g.call(t, 0, &[model, i]);
            let lock = g.read::<u32>(w(0x43));
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let t = slot_target(g, w(0x4c), 779);
            g.call(t, 0, &[h.eax, h.edx, model, i]);
            let lock = g.read::<u32>(w(0x5f));
            i += 1;
            g.write::<u32>(lock, saved);
            rec = rec.wrapping_add(0x94);
            if i.cast_signed() >= g.read::<i32>(w(0x71)) {
                break;
            }
        }
    }
    let t = slot_target(g, w(0x78), 804);
    let b = g.read::<u32>(w(0x7e));
    g.call(t, 0, &[b]);
    Ret::eax(1)
}

/// 0x100018d0 (153 bytes, 1 overlay).
pub const REGISTER: Sig = Sig {
    len: 153,
    hash: 0xc8a3_7198_2e58_fa3a,
    calls: &[],
    imports: &[],
};

/// Clears the effect's object pool (0x10002af0, internal, no
/// arguments): the 0x5c-byte header (operand `+0x0b`) cleared, then 100
/// objects (count `+0x1e`, first `+0x28` = the array at `+0x10`, three
/// header words 0), each 0x94 bytes cleared. Returns the last object
/// (`memset`'s answer).
pub fn clear<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write_bytes(w(0xb), &[0u8; 0x5c]);
    let first = w(0x10);
    g.write::<u32>(w(0x1e), 0x64);
    g.write::<u32>(w(0x28), first);
    for op in [0x2e, 0x38, 0x42] {
        g.write::<u32>(w(op), 0);
    }
    let mut obj = first;
    let mut last = obj;
    for _ in 0..0x64 {
        g.write_bytes(obj, &[0u8; 0x94]);
        last = obj;
        g.write::<u32>(obj, 0);
        obj = obj.wrapping_add(0x94);
    }
    last
}

/// [`clear`] (117 bytes, 1 overlay).
pub const CLEAR: Sig = Sig {
    len: 117,
    hash: 0x0727_3432_74d0_aa28,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x59, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The effect's frame (0x10001a20, an overlay table entry, no
/// arguments): 0 when slot 253 says 0 or the flag at operand `+0x16` is
/// clear. Else `*G_V = *G_W`, `*G_X = *G_Y`; slot 261 `(0)` and `(1)`
/// each run a timeline ([`timeline_run`](crate::sb_misc::timeline_run))
/// when they say nonzero; when `(2)` does both stop
/// ([`timeline_stop`](crate::sb_misc::timeline_stop)), the name
/// (`+0xa4`) goes to `+0xb0` and slot 718, slot 222 `(name, 0, 1000,
/// 0x2000)`, slot 378 `(0x2000)` and slot 214 `(*G_E)` (the earlier
/// words left on the stack); else the frame count `+0x109` goes up.
/// Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 253);
    if g.call(t, 0, &[]).eax == 0 || g.read::<u32>(w(0x16)) == 0 {
        return Ret::eax(0);
    }
    let v = g.read::<u32>(w(0x1f));
    let x = g.read::<u32>(w(0x24));
    g.write::<u32>(v, x);
    let y = g.read::<u32>(w(0x2b));
    g.write::<u32>(w(0x30), y);
    for (tab, k, call, this) in [(0x35u32, 0u32, 0x50u32, 0x4bu32), (0x55, 1, 0x70, 0x6b)] {
        let t = slot_target(g, w(tab), 261);
        if g.call(t, 0, &[k]).eax != 0 {
            crate::sb_misc::timeline_run(g, img, img.rel32_target(f + call), w(this));
        }
    }
    let t = slot_target(g, w(0x75), 261);
    if g.call(t, 0, &[2]).eax == 0 {
        let n = w(0x109);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop(g, w(0x8b));
    crate::sb_misc::timeline_stop(g, w(0x95));
    let t = slot_target(g, w(0x9f), 718);
    let mut stack = vec![w(0xa4)];
    g.write::<u32>(w(0xb0), w(0xb4));
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xbb), 222);
    let name = g.read::<u32>(w(0xd3));
    stack.splice(0..0, [name, 0, 0x3e8, 0x2000]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xda), 378);
    stack.insert(0, 0x2000);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xec), 214);
    stack.insert(0, g.read::<u32>(w(0xf2)));
    // call width: 7
    g.call(t, 0, &stack);
    Ret::eax(1)
}

/// 0x10001a20 (275 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 275,
    hash: 0x5fc0_dff4_e695_b376,
    calls: &[
        (0x50, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x70, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x90, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x9a, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// A spark record taken and scattered around a point (0x10001000, cdecl
/// `(point)`): a record ([`crate::s4_small::record_take`], the resource
/// `*G_RES`; none: returns 0) gets `+0x28`, `+0x24` 20 and `+0x2c` 2;
/// `+0x4..+0xc` the point's `x, y, z` each moved by `((r / R - H) * A) *
/// B` (`r` slot 324's low 15 bits), `+0x58` slot 403 `(r / R * S)` (ST0
/// as `f32`; the word on the stack) and `+0x54` `r / R * T * U`. Returns
/// the last draw's low 15 bits.
pub fn spark_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let res = g.read::<u32>(w(0x06));
    // push ebp; push ecx; push edi; the return address
    let callee = Frame {
        at: img.rel32_target(f + 0x0c),
        esp: fr.esp.wrapping_sub(0x10),
        edx: res,
        ..Frame::default()
    };
    let rec = crate::s4_small::record_take(g, img, &callee).eax;
    if rec == 0 {
        return Ret::eax(0);
    }
    g.write::<u32>(rec + 0x28, w(0x23));
    g.write::<u32>(rec + 0x24, w(0x2a));
    g.write::<u32>(rec + 0x2c, w(0x31));
    let draw = |g: &mut G, table: u32| {
        let t = slot_target(g, w(table), 324);
        // call width: 0
        g.call(t, 0, &[]).eax & 0x7fff
    };
    let point = fr.arg(g, 0);
    for (i, (table, ops)) in (0u32..).zip([
        (0x1cu32, [0x51u32, 0x5e, 0x66, 0x6e]),
        (0x56, [0x98, 0xa0, 0xa8, 0xb0]),
        (0x90, [0xdb, 0xe3, 0xeb, 0xf3]),
    ]) {
        let r = draw(g, table);
        let d = div(cvt(r), k(g, ops[0]));
        let d = mul(mul(sub(d, k(g, ops[1])), k(g, ops[2])), k(g, ops[3]));
        let v = add(d, g.read::<f32>(point + 4 * i));
        g.write::<f32>(rec + 4 + 4 * i, v);
    }
    let r = draw(g, 0xd3);
    let x = mul(div(cvt(r), k(g, 0x11e)), k(g, 0x12c));
    let t = slot_target(g, w(0x116), 403);
    // call width: 1
    let out = g.call(t, 0, &[x.to_bits()]);
    g.write::<f32>(rec + 0x58, crate::ob_model::st0_f32(out.st0));
    let r = draw(g, 0x13c);
    let v = mul(mul(div(cvt(r), k(g, 0x15c)), k(g, 0x164)), k(g, 0x16c));
    g.write::<f32>(rec + 0x54, v);
    Ret::eax(r)
}

/// [`spark_record`] (378 bytes, 1 overlay).
pub const SPARK_RECORD: Sig = Sig {
    len: 378,
    hash: 0x6e3a_aa5a_8da5_78cd,
    calls: &[(0x0c, Callee::Sig(&crate::s4_small::RECORD_TAKE))],
    imports: &[],
};

/// Starts the effect's state (0x10001580, an overlay table entry, no
/// arguments): slot 719 `(*G_NAME, 4, 0)` gives the data (`+0x21`); the
/// 0x30-byte block `B` (`+0x1c`) zeroed (`memset`), its half-word `+2`
/// 0x80 and `+4` 0x80808080; the objects cleared ([`clear`]); `+0x4c`
/// 1, `+0x56` 0.1, `+0x60` and `+0x6a` 0. Returns the clear's `eax`.
pub fn state_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 719);
    let name = g.read::<u32>(w(0x11));
    let d = g.call(t, 0, &[name, 4, 0]).eax;
    g.write::<u32>(w(0x21), d);
    g.write_bytes(w(0x1c), &[0u8; 0x30]);
    g.write::<u16>(w(0x34), 0x80);
    g.write::<u32>(w(0x3a), 0x8080_8080);
    let eax = clear(g, img, img.rel32_target(f + 0x43));
    g.write::<u32>(w(0x4c), 1);
    g.write::<u32>(w(0x56), 0x3dcc_cccd);
    g.write::<u32>(w(0x60), 0);
    g.write::<u32>(w(0x6a), 0);
    Ret::eax(eax)
}

/// [`state_start`] (115 bytes, 1 overlay).
pub const STATE_START: Sig = Sig {
    len: 115,
    hash: 0x1417_2782_0fbb_cea1,
    calls: &[
        (0x26, Callee::Import("memset")),
        (0x43, Callee::Sig(&CLEAR)),
    ],
    imports: &[],
};
