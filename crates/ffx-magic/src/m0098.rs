//! `magic_0098.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::m0018::st0_f32;
use crate::objcb::{PartAt, imm_stores, part_start};
use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Libm, Ret, Sig, Xmm};

/// A target's height (0x100011a0, internal; the result in `xmm0`): slot
/// 390 `(G_T)` gives the target's model id, slot 19 of its handle (slot
/// 215 `(G_T)`; the first call's word left on the stack) its float (ST0,
/// stored as `f32`), corrected for three models: `+ A` for 0x1015 and
/// 0x1127, `- B` for 0x1077. `eax` is slot 19's.
pub fn target_height<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32) -> Ret {
    let w = |off: u32| img.word(f + off);
    let s390 = slot_target(g, w(0x05), 390);
    let t = g.read::<u32>(w(0x0c));
    let id = g.call(s390, ecx, &[t]).eax;
    let s215 = slot_target(g, w(0x1c), 215);
    // call width: 2 (the word and slot 390's)
    let h = g.call(s215, ecx, &[t, t]).eax;
    let s19 = slot_target(g, w(0x30), 19);
    let o = g.call(s19, ecx, &[h]);
    let v = st0_f32(o.st0);
    let v = match id {
        0x1015 => add(v, g.read::<f32>(w(0x98))),
        0x1077 => sub(v, g.read::<f32>(w(0x82))),
        0x1127 => add(v, g.read::<f32>(w(0x6c))),
        _ => v,
    };
    Ret::xmm0(Xmm::f32(v), o.eax)
}

/// [`target_height`] (161 bytes, 1 overlay).
pub const TARGET_HEIGHT: Sig = Sig {
    len: 161,
    hash: 0xc502_d317_55e3_19ac,
    calls: &[],
    imports: &[],
};

/// Builds a view matrix (0x10001250, internal: `ecx` = the matrix, 16
/// floats): slot 325 `(&D, DIR)`, `D` with its y cleared normalised
/// (slot 232) into `F`, the axes `S = F x UP` and `U = S x F` (slot 266,
/// `UP = (0, 1, 0, 1)` at `ebp - 0x44`), both normalised; the rows `S`,
/// `U`, `F` (their `w` 0) and the position from slot 325 `(m + 0x30,
/// POS)`, `+0x3c` 1.0. Locals: `D` `ebp - 0x54`, `F` `- 0x24`, `S` `-
/// 0x14`, `U` `- 0x34`. Each call gets the words the earlier ones left
/// on the stack. Returns slot 325's result.
#[allow(clippy::similar_names)]
pub fn view_matrix<G: Game>(g: &mut G, img: &Image, f: u32, m: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp - 4;
    let (d, up, fw, s, u) = (ebp - 0x54, ebp - 0x44, ebp - 0x24, ebp - 0x14, ebp - 0x34);
    let s325 = slot_target(g, w(0x1b), 325);
    let mut stack = vec![d, w(0x15)];
    g.call(s325, m, &stack);
    g.write::<u32>(d + 4, 0);
    let s232 = slot_target(g, w(0x32), 232);
    for (o, v) in [(0u32, 0u32), (4, 0x3f80_0000), (8, 0), (0xc, 0x3f80_0000)] {
        g.write::<u32>(up + o, v);
    }
    stack.splice(0..0, [fw, d]);
    // call width: 4
    g.call(s232, m, &stack);
    let s266 = slot_target(g, w(0x6e), 266);
    stack.splice(0..0, [s, up, fw]);
    // call width: 7
    g.call(s266, m, &stack);
    let s266 = slot_target(g, w(0x87), 266);
    stack.splice(0..0, [u, fw, s]);
    // call width: 10
    g.call(s266, m, &stack);
    let s232 = slot_target(g, w(0x99), 232);
    stack.splice(0..0, [s, s]);
    // call width: 12
    g.call(s232, m, &stack);
    let s232 = slot_target(g, w(0xab), 232);
    stack.splice(0..0, [u, u]);
    // call width: 14
    g.call(s232, m, &stack);
    for (row, src) in [(0u32, s), (0x10, u), (0x20, fw)] {
        for k in 0..3u32 {
            let v = g.read::<u32>(src + 4 * k);
            g.write::<u32>(m + row + 4 * k, v);
        }
        if row == 0x20 {
            g.write::<u32>(m + 0x30, 0);
        }
    }
    let s325 = slot_target(g, w(0x111), 325);
    for o in [0xcu32, 0x1c, 0x2c, 0x34, 0x38] {
        g.write::<u32>(m + o, 0);
    }
    g.write::<u32>(m + 0x3c, 0x3f80_0000);
    stack.splice(0..0, [m + 0x30, w(0x10b)]);
    // call width: 16
    g.call(s325, m, &stack).eax
}

/// [`view_matrix`] (360 bytes, 1 overlay).
pub const VIEW_MATRIX: Sig = Sig {
    len: 360,
    hash: 0xfffb_be97_4cae_8d7b,
    // __security_check_cookie
    calls: &[(0x160, Callee::Any)],
    imports: &[],
};

/// A step (0x10002130, no arguments): `G_SCALE` = 1.0, slot 276 `(G_H,
/// 1)`. Returns 1.
pub fn show<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s276 = slot_target(g, w(0x01), 276);
    let h = g.read::<u32>(w(0x09));
    g.write::<u32>(w(0x15), w(0x19));
    g.call(s276, fr.ecx, &[h, 1]);
    Ret::eax(1)
}

/// 0x10002130 (40 bytes, 1 overlay).
pub const SHOW: Sig = Sig {
    len: 40,
    hash: 0xe40a_b4b0_8252_927f,
    calls: &[],
    imports: &[],
};

/// A step (0x10002210, no arguments): slot 276 `(G_H, 0)`, then `G_DONE`
/// = 1. Returns 1.
pub fn hide<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s276 = slot_target(g, w(0x01), 276);
    let h = g.read::<u32>(w(0x09));
    g.call(s276, fr.ecx, &[h, 0]);
    g.write::<u32>(w(0x1a), 1);
    Ret::eax(1)
}

/// 0x10002210 (40 bytes, 1 overlay).
pub const HIDE: Sig = Sig {
    len: 40,
    hash: 0x8b2e_f348_6359_2958,
    calls: &[],
    imports: &[],
};

/// An object callback (0x10002660, `(obj)`): slots 230 `(G_C, VEC)`, 231
/// `(G_A, G_B, G_C)`, 230 `(G_D, G_A)` and slot 712 `(obj, 0x30)` with
/// `*G_P = ITEM` and the game flag `*G_LOCK` 1 around it (each call with
/// the words the earlier ones left). Returns 1 when 712 gave 0, else 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ecx = fr.ecx;
    let s230 = slot_target(g, w(0x04), 230);
    let c = g.read::<u32>(w(0x16));
    let mut stack = vec![c, w(0x10)];
    g.call(s230, ecx, &stack);
    let c = g.read::<u32>(w(0x1e));
    let s231 = slot_target(g, w(0x23), 231);
    let b = g.read::<u32>(w(0x29));
    let a = g.read::<u32>(w(0x35));
    stack.splice(0..0, [a, b, c]);
    // call width: 5
    g.call(s231, ecx, &stack);
    let a = g.read::<u32>(w(0x3d));
    let s230 = slot_target(g, w(0x42), 230);
    let d = g.read::<u32>(w(0x48));
    stack.splice(0..0, [d, a]);
    // call width: 7
    g.call(s230, ecx, &stack);
    let p = g.read::<u32>(w(0x55));
    let obj = fr.arg(g, 0);
    g.write::<u32>(p, w(0x60));
    let lock = g.read::<u32>(w(0x65));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s712 = slot_target(g, w(0x72), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 9
    let r = g.call(s712, lock, &stack).eax;
    let lock = g.read::<u32>(w(0x80));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// 0x10002660 (149 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 149,
    hash: 0x91e2_46be_5b1e_d450,
    calls: &[],
    imports: &[],
};

/// An object callback (0x100029d0, `(obj)`): the part `list[0]` started
/// on the object ([`part_start`]), slot 325 `(obj + 0x40, POS)` (the part
/// start's words left below), its y `+0x44` raised by `H`, the model's
/// other words. Returns 1.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let at = PartAt {
        gp: 0x04,
        item: 0x0b,
        gd: 0x10,
        table: 0x18,
        entry: 0,
        call: 0x35,
    };
    let (_, left) = part_start(g, img, f, fr.ecx, obj, at);
    let s325 = slot_target(g, w(0x43), 325);
    let mut stack = vec![obj + 0x40, w(0x3d)];
    stack.extend(left);
    // call width: 6
    g.call(s325, obj, &stack);
    let y = add(g.read::<f32>(obj + 0x44), g.read::<f32>(w(0x58)));
    imm_stores(g, img, f, obj, &[(0x50, 0x62), (0x54, 0x69), (0x58, 0x70)]);
    g.write::<f32>(obj + 0x44, y);
    imm_stores(
        g,
        img,
        f,
        obj,
        &[
            (0x5c, 0x7c),
            (0x60, 0x83),
            (0x64, 0x8a),
            (0x68, 0x91),
            (0x6c, 0x98),
        ],
    );
    Ret::eax(1)
}

/// 0x100029d0 (164 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 164,
    hash: 0xdd63_7ae1_16cf_22ad,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// A step (0x10002240, no arguments): `G_DONE` cleared; the first time
/// (`G_H` 0) the model 0x3015 loaded (slots 26 `(0x3015, 1)`, 0
/// `(0x3015)` into `G_H`) and set up (slots 387 `(h, 1)`, 385 `(G_H, 1)`,
/// 386 `(G_H, 0)`, 922 `(G_H + 0x830, 0)`; each call with the words the
/// earlier ones left); then the fade words, and one of two command
/// records (by `G_ALT`) gets the model and its handler table. Returns 1.
pub fn load<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ecx = fr.ecx;
    let mut h = g.read::<u32>(w(0x01));
    g.write::<u32>(w(0x07), 0);
    if h == 0 {
        let s26 = slot_target(g, w(0x18), 26);
        let mut stack = vec![0x3015, 1];
        g.call(s26, ecx, &stack);
        let s0 = slot_target(g, w(0x29), 0);
        stack.insert(0, 0x3015);
        // call width: 3
        let r = g.call(s0, ecx, &stack).eax;
        g.write::<u32>(w(0x37), r);
        let s387 = slot_target(g, w(0x3f), 387);
        stack.splice(0..0, [r, 1]);
        // call width: 5
        g.call(s387, ecx, &stack);
        let s385 = slot_target(g, w(0x4c), 385);
        let m = g.read::<u32>(w(0x54));
        stack.splice(0..0, [m, 1]);
        // call width: 7
        g.call(s385, ecx, &stack);
        let s386 = slot_target(g, w(0x61), 386);
        let m = g.read::<u32>(w(0x69));
        stack.splice(0..0, [m, 0]);
        // call width: 9
        g.call(s386, ecx, &stack);
        let m = g.read::<u32>(g.read::<u32>(w(0x76)) + 0x830);
        let s922 = slot_target(g, w(0x83), 922);
        stack.splice(0..0, [m, 0]);
        // call width: 11
        g.call(s922, ecx, &stack);
        h = g.read::<u32>(w(0x90));
    }
    let alt = g.read::<u32>(w(0x99));
    for (a, v) in [(0xa0u32, 0xa4u32), (0xaa, 0xae), (0xb4, 0xb8), (0xbe, 0xc2)] {
        g.write::<u32>(w(a), w(v));
    }
    for a in [0xc8u32, 0xd2, 0xdc, 0xe6] {
        g.write::<u32>(w(a), 0);
    }
    let (q0, at, q1, table) = if alt == 0 {
        (0x11c, 0x121, 0x129, (0x12f, 0x133))
    } else {
        (0xf7, 0xfc, 0x104, (0x10a, 0x10e))
    };
    g.write::<u64>(w(q0), 0);
    g.write::<u32>(w(at), h);
    g.write::<u64>(w(q1), 0);
    g.write::<u32>(w(table.0), w(table.1));
    Ret::eax(1)
}

/// 0x10002240 (317 bytes, 1 overlay).
pub const LOAD: Sig = Sig {
    len: 317,
    hash: 0xeb2d_60bb_dd3c_48ea,
    calls: &[],
    imports: &[],
};

/// A motion sequence's step (0x10003fc0, internal: `ecx` = the
/// sequence `{entries, model, index, motion}`, the entries 16 bytes each
/// `{motion, frame, callback, argument}`, a 0 motion ends the list):
/// nothing (0) at the end of the list. With no motion playing (`+0xc`
/// 0), slot 37 `(model, 1, motion)` starts the entry's and slot 18
/// `(model)` gives the playing one (into `+0xc`); -1 there skips to the
/// end of the list. Then the entry is over when slot 22's frame
/// `(model) >> 8` plus 1 reaches the entry's frame (or, with a frame of
/// 0, equals the playing motion): the index moves on and, unless the
/// list ends there, slot 30 `(model)` and the next entry's motion start
/// as above. The entry's callback (when set) runs with its argument.
/// Returns the callback's `eax`, else twice the index.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn motion_sequence<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let entries = g.read::<u32>(this);
    let entry = |g: &G| entries.wrapping_add(g.read::<u32>(this + 8).wrapping_mul(16));
    let model = |g: &G| g.read::<u32>(this + 4);
    let e = entry(g);
    let motion = g.read::<u32>(e);
    if motion == 0 {
        return 0;
    }
    if g.read::<u32>(this + 0xc) == 0 {
        let s = table(g, 0x1e, 37);
        g.call(s, 0, &[model(g), 1, motion]);
        let s = table(g, 0x30, 18);
        let m = g.call(s, 0, &[model(g), model(g), 1, motion]).eax;
        g.write::<u32>(this + 0xc, m);
        if m == u32::MAX && g.read::<u32>(entry(g)) != 0 {
            loop {
                let i = g.read::<u32>(this + 8).wrapping_add(1);
                g.write::<u32>(this + 8, i);
                if g.read::<u32>(entry(g)) == 0 {
                    break;
                }
            }
        }
    }
    let frame = g.read::<u32>(entry(g) + 4);
    let s = table(g, 0x70, 22);
    let now = (g.call(s, 0, &[model(g)]).eax.cast_signed() >> 8).wrapping_add(1);
    let over = if frame != 0 {
        g.read::<u32>(entry(g) + 4) == now.cast_unsigned()
    } else {
        g.read::<i32>(this + 0xc) == now
    };
    if over {
        let i = g.read::<u32>(this + 8).wrapping_add(1);
        g.write::<u32>(this + 8, i);
        let e = entry(g);
        if g.read::<u32>(e) == 0 {
            return i.wrapping_add(i);
        }
        let s = table(g, 0xaa, 30);
        g.call(s, 0, &[model(g)]);
        let motion = g.read::<u32>(entry(g));
        let s = table(g, 0xbf, 37);
        g.call(s, 0, &[model(g), 1, motion, model(g)]);
        let s = table(g, 0xd1, 18);
        let m = g.call(s, 0, &[model(g), model(g), 1, motion, model(g)]).eax;
        g.write::<u32>(this + 0xc, m);
    }
    let i = g.read::<u32>(this + 8);
    let e = entry(g);
    let cb = g.read::<u32>(e + 8);
    if cb == 0 {
        return i.wrapping_add(i);
    }
    let arg = g.read::<u32>(e + 0xc);
    g.call(cb, cb, &[arg]).eax
}

/// [`motion_sequence`] (252 bytes, 1 overlay).
pub const MOTION_SEQUENCE: Sig = Sig {
    len: 252,
    hash: 0x90a0_e980_f165_3339,
    calls: &[],
    imports: &[],
};

/// The frame step (0x10003340, no arguments): nothing (0) unless `*A`
/// (operand `+0x02`) is set; then `**P` = 0 (`+0x0d`), and 1 when slot
/// 253 says 0 (`+0x29`); `**Q` = the immediate `+0x40` (`+0x34`); slots
/// 723 `(*M)` and 724 `(*N, *M)` (`+0x3a`, `+0x58`); `**R = *S` (`+0x66`,
/// `+0x6b`); one of two timelines (`+0x7c`, or `+0x83` when `*F`
/// (`+0x74`) is 0) runs ([`timeline_run`](crate::sb_misc::timeline_run)).
/// When nothing is live, the other pair's timeline (`+0x91`, or `+0x9e`
/// by `*F` (`+0x97`)) stops and slot 214 `(*H)` (`+0xae`) runs; else,
/// when the state word (`+0xce`, or `+0xd7` by `*F` (`+0xc5`)) is 5, slot
/// 393 `(*V, X, 0x100)` (`+0xf5`, `+0xef`), and the counter `+0x100`
/// goes up. Returns 1. Each slot call passes the words the original
/// leaves on the stack.
pub fn frame_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x0d));
    g.write::<u32>(p, 0);
    let s = table(g, 0x18, 253);
    if g.call(s, 0, &[]).eax == 0 {
        let p = g.read::<u32>(w(0x29));
        g.write::<u32>(p, 1);
    }
    let q = g.read::<u32>(w(0x34));
    let m = g.read::<u32>(w(0x3a));
    g.write::<u32>(q, w(0x40));
    let s = table(g, 0x45, 723);
    g.call(s, 0, &[m]);
    let n = g.read::<u32>(w(0x58));
    let s = table(g, 0x52, 724);
    g.call(s, 0, &[n, m]);
    let r = g.read::<u32>(w(0x66));
    let v = g.read::<u32>(w(0x6b));
    let tl = if g.read::<u32>(w(0x74)) != 0 {
        w(0x7c)
    } else {
        w(0x83)
    };
    g.write::<u32>(r, v);
    let run = img.rel32_target(f + 0x88);
    if crate::sb_misc::timeline_run(g, img, run, tl) == 0 {
        let tl = if g.read::<u32>(w(0x97)) != 0 {
            w(0x91)
        } else {
            w(0x9e)
        };
        crate::sb_misc::timeline_stop(g, tl);
        let h = g.read::<u32>(w(0xae));
        let s = table(g, 0xa8, 214);
        g.call(s, 0, &[h]);
        return Ret::eax(1);
    }
    let state = if g.read::<u32>(w(0xc5)) != 0 {
        g.read::<u32>(w(0xce))
    } else {
        g.read::<u32>(w(0xd7))
    };
    if state == 5 {
        let x = g.read::<u32>(w(0xf5));
        let s = table(g, 0xdf, 393);
        g.call(s, 0, &[x, w(0xef), 0x100]);
    }
    let c = g.read::<u32>(w(0x100));
    g.write::<u32>(w(0x100), c.wrapping_add(1));
    Ret::eax(1)
}

/// [`frame_step`] (266 bytes, 1 overlay).
pub const FRAME_STEP: Sig = Sig {
    len: 266,
    hash: 0xd1e0_bf4e_d15b_c35c,
    calls: &[
        (0x88, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xa3, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// A part's callback (0x10002730, cdecl `(part)`, a /GS frame): by the
/// part's number `(part - base) >> 7` (`base` at `+0x17`): 0 starts the
/// part with the program `table[2]` and its scale `+0x60..+0x68` 0.04
/// (`+0x6c` 1.0); 1 starts it with `table[3]` (the scale 0.056) and slot
/// 237 `(*X, 0x2a, v, part + 0x40)` gets a local vector `v` = `(0, 0,
/// 2000, 1)` (on the frame at `ebp - 0x14`; `X` at `+0xdb`), and `+0x108`
/// is set to 0x80. The program table comes from slot 941 `(*O + 0x10)`
/// (`O` at `+0x4a`), `**G` = the immediate `+0x45` first (`G` at
/// `+0x3f`), and the `pppStartPart` wrapper starts the part (`0x1000`,
/// the program, 1). In every case `+0x50..+0x58` 0 and `+0x5c` 1.
/// Returns 1. Each slot call passes the words the original leaves on
/// the stack.
pub fn part_callback<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = fr.arg(g, 0);
    // push ebp; sub esp, 0x14: the vector at [ebp - 0x14]
    let v = fr.esp.wrapping_sub(0x18);
    for (k, x) in (0u32..).zip([0, 0, 2000.0f32.to_bits(), 1.0f32.to_bits()]) {
        g.write::<u32>(v + 4 * k, x);
    }
    let n = (part.wrapping_sub(w(0x17)).cast_signed() >> 7).cast_unsigned();
    if n <= 1 {
        let o = if n == 0 { 0 } else { 0x59 };
        let p = g.read::<u32>(w(0x3f + o));
        g.write::<u32>(p, w(0x45 + o));
        let owner = g.read::<u32>(g.read::<u32>(w(0x4a + o)) + 0x10);
        let s941 = slot_target(g, w(0x52 + o), 941);
        let table = g.call(s941, 0, &[owner]).eax;
        let program = g.read::<u32>(table.wrapping_add(8 + 4 * n));
        let wrapper = img.rel32_target(f + 0x6d + o);
        crate::opu_parts::start_part(g, img, wrapper, part, 0x1000, program, 1);
        let scale = if n == 0 { 0x3d23_d70a } else { 0x3d65_6041 };
        if n == 1 {
            let x = g.read::<u32>(w(0xdb));
            for off in [0x60, 0x64, 0x68] {
                g.write::<u32>(part + off, scale);
            }
            g.write::<u32>(part + 0x6c, 0x3f80_0000);
            let s237 = slot_target(g, w(0xd3), 237);
            // call width: 8 (the earlier calls' five words left on the stack)
            g.call(s237, 0, &[x, 0x2a, v, part + 0x40, program, 1, 0, owner]);
            g.write::<u32>(w(0x108), 0x80);
        } else {
            for off in [0x60, 0x64, 0x68] {
                g.write::<u32>(part + off, scale);
            }
            g.write::<u32>(part + 0x6c, 0x3f80_0000);
        }
    }
    for off in [0x50, 0x54, 0x58] {
        g.write::<u32>(part + off, 0);
    }
    g.write::<u32>(part + 0x5c, 1);
    Ret::eax(1)
}

/// [`part_callback`] (320 bytes, 1 overlay; the `rel32` at `+0x138` is
/// `__security_check_cookie`).
pub const PART_CALLBACK: Sig = Sig {
    len: 320,
    hash: 0x283a_1c00_9b9d_9daa,
    calls: &[
        (0x6d, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xc6, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x138, Callee::Any),
    ],
    imports: &[],
};

/// The camera's height follow (0x10001590, no arguments, a /GS frame):
/// slot 210 `(E, m)` gives the camera matrix of `E` (`*` operand
/// `+0x16`) into a local `m` (`ebp - 0x84`); slot 325 copies its row 3
/// into a local `a` (`ebp - 0x44`) and its row 2 into `b` (`ebp -
/// 0x14`), slot 232 normalises `b`, and `b` is scaled by `K` (`+0x74`).
/// The target's height `h` ([`target_height`]), slot 364 `(E)` (its float
/// dropped), slot 268 `(E, 0x10, c)` into a local `c` (`ebp - 0x34`);
/// `d` (`ebp - 0x24`) = `b * s` with `s = f32(f64(h + K2) + D)` (`+0xda`,
/// the double at `+0xf2`), and slot 248 `(X, c, d)` (`X` the immediate
/// `+0xf7`). Then `n = |trunc(c.y - a.y)|`; while `f32(n)` is below `K4`
/// (`+0x14d`) the global `+0x168` becomes `a.y - K4`, else it drops by
/// `K5` (`+0x186`). Returns `n`.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn height_follow<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::mul;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let fl = |g: &G, a: u32| g.read::<f32>(a);
    // push ebp: ebp = esp - 4; the locals below it
    let ebp = fr.esp.wrapping_sub(4);
    let (m, a, b, c, d) = (
        ebp.wrapping_sub(0x84),
        ebp.wrapping_sub(0x44),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x34),
        ebp.wrapping_sub(0x24),
    );
    let e = g.read::<u32>(w(0x16));
    let mut stack = vec![e, m];
    let mut call = |g: &mut G, s: u32, words: &[u32]| {
        for &x in words.iter().rev() {
            stack.insert(0, x);
        }
        // call width: 15 (every earlier call's words stay on the stack)
        g.call(s, 0, &stack)
    };
    let s = table(g, 0x22, 210);
    call(g, s, &[]);
    let s = table(g, 0x38, 325);
    call(g, s, &[a, m + 0x30]);
    let s = table(g, 0x4d, 325);
    call(g, s, &[b, m + 0x20]);
    let s = table(g, 0x5f, 232);
    call(g, s, &[b, b]);
    let k = fl(g, w(0x74));
    for i in 0..3 {
        let x = mul(fl(g, b + 4 * i), k);
        g.write::<u32>(b + 4 * i, x.to_bits());
    }
    let r = target_height(g, img, img.rel32_target(f + 0x9e), 0);
    let h = match r.xmm0 {
        Some(Xmm::F32(x)) => f32::from_bits(x),
        _ => 0.0,
    };
    let s = table(g, 0xa3, 364);
    call(g, s, &[e]);
    let s = table(g, 0xbf, 268);
    call(g, s, &[e, 0x10, c]);
    let hk = crate::sb_sse::add(h, fl(g, w(0xda)));
    let dk =
        f64::from_bits(u64::from(img.word(w(0xf2))) | (u64::from(img.word(w(0xf2) + 4)) << 32));
    // cvtps2pd; addsd; cvtpd2ps
    #[allow(clippy::cast_possible_truncation)]
    let scale = (f64::from(hk) + dk) as f32;
    for i in 0..3 {
        let x = mul(fl(g, b + 4 * i), scale);
        g.write::<u32>(d + 4 * i, x.to_bits());
    }
    let s = table(g, 0xea, 248);
    call(g, s, &[w(0xf7), c, d]);
    let ay = fl(g, a + 4);
    let x = sub(fl(g, c + 4), ay);
    let n = ffx_bridge::x87::cvttsd2si(f64::from(x))
        .cast_signed()
        .wrapping_abs();
    let k4 = fl(g, w(0x14d));
    #[allow(clippy::cast_precision_loss)]
    if k4 > n as f32 {
        let y = sub(ay, k4);
        g.write::<u32>(w(0x168), y.to_bits());
    } else {
        let y = sub(fl(g, w(0x17e)), fl(g, w(0x186)));
        g.write::<u32>(w(0x193), y.to_bits());
    }
    Ret::eax(n.cast_unsigned())
}

/// [`height_follow`] (416 bytes, 1 overlay; the `rel32`s at `+0x172` and
/// `+0x198` are `__security_check_cookie`).
pub const HEIGHT_FOLLOW: Sig = Sig {
    len: 416,
    hash: 0x9453_21cc_02c9_8be8,
    calls: &[
        (0x9e, Callee::Sig(&TARGET_HEIGHT)),
        (0x172, Callee::Any),
        (0x198, Callee::Any),
    ],
    imports: &[],
};

/// A part's draw (0x10002870, cdecl `(part)`, a /GS frame): by the
/// part's number `(part - base) >> 7` (`base` at `+0x18`): 0 calls slot
/// 237 `(*X, 0x2a, v, part + 0x40)` with the local vector `v` = `(0, 0,
/// 2000, 1)` (`ebp - 0x14`); 1 fades the alpha `*C` (`+0x62`): at 0x80
/// slot 393 `(*D, X, 0x100)` (`+0x84`, `+0x7e`) first, then `*C - 12`
/// held to `0..=0x80` is stored and slot 391 `(0x60, 0x60, 0x60, alpha)`
/// runs. Then slot 258 `(part)`, slot 231 `(*A, *B, *C2)` (`+0xe8`,
/// `+0xdc`, `+0xd1`), slot 230 `(*F, *G)` (`+0xfb`, `+0xf0`), `**H` the
/// immediate `+0x110`, and slot 712 `(part, 0x30)` draws it with the
/// game flag `*L` (`+0x115`) 1 around the call. Returns 1 when slot 712
/// says 0, else 0.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn part_draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let part = fr.arg(g, 0);
    let v = fr.esp.wrapping_sub(0x18);
    for (k, x) in (0u32..).zip([0, 0, 2000.0f32.to_bits(), 1.0f32.to_bits()]) {
        g.write::<u32>(v + 4 * k, x);
    }
    let n = (part.wrapping_sub(w(0x18)).cast_signed() >> 7).cast_unsigned();
    if n == 0 {
        let x = g.read::<u32>(w(0x50));
        let s = table(g, 0x48, 237);
        g.call(s, 0, &[x, 0x2a, v, part + 0x40]);
    } else if n == 1 {
        let mut a = g.read::<u32>(w(0x62));
        if a == 0x80 {
            let d = g.read::<u32>(w(0x84));
            let s = table(g, 0x6e, 393);
            g.call(s, 0, &[d, w(0x7e), 0x100]);
            a = g.read::<u32>(w(0x8b));
        }
        let a = a
            .cast_signed()
            .wrapping_sub(12)
            .clamp(0, 0x80)
            .cast_unsigned();
        g.write::<u32>(w(0xa7), a);
        let s = table(g, 0xac, 391);
        g.call(s, 0, &[0x60, 0x60, 0x60, a]);
    }
    let s = table(g, 0xc2, 258);
    g.call(s, 0, &[part]);
    let (x, y, z) = (
        g.read::<u32>(w(0xe8)),
        g.read::<u32>(w(0xdc)),
        g.read::<u32>(w(0xd1)),
    );
    let s = table(g, 0xd6, 231);
    g.call(s, 0, &[x, y, z, part]);
    let (p, q) = (g.read::<u32>(w(0xfb)), g.read::<u32>(w(0xf0)));
    let s = table(g, 0xf5, 230);
    g.call(s, 0, &[p, q, x, y, z, part]);
    let h = g.read::<u32>(w(0x108));
    g.write::<u32>(h, w(0x110));
    let lock = g.read::<u32>(w(0x115));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s = table(g, 0x123, 712);
    // call width: 8 (the earlier calls' words left on the stack)
    let r = g.call(s, 0, &[part, 0x30, p, q, x, y, z, part]).eax;
    let lock = g.read::<u32>(w(0x131));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// [`part_draw`] (339 bytes, 1 overlay; the `rel32` at `+0x14b` is
/// `__security_check_cookie`).
pub const PART_DRAW: Sig = Sig {
    len: 339,
    hash: 0x9c57_a640_2f75_bffa,
    calls: &[(0x14b, Callee::Any)],
    imports: &[],
};

/// The camera's tilt (0x100013c0, no arguments, an aligned /GS frame):
/// the view matrix `M` on the frame ([`view_matrix`], `esp + 0x30`);
/// four points `(0, 0, -K, 1)`, `(0, 0, K, 1)`, `(-K, 0, 0, 1)`, `(K, 0,
/// 0, 1)` (`K` about 10.4, the code's immediates; one local vector `v` at
/// `esp`, rewritten for each)
/// transformed by `M` through slot 229 into the globals `+0x52`, `+0x8e`,
/// `+0xca`, `+0x106`; slot 232 normalises the global `+0x111` into `a`
/// (`esp + 0x20`) and `M`'s row 2 into `b` (`esp + 0x10`); slot 394
/// `(a, b)` gives `x` (ST0), and the global `+0x1c1` becomes `(K2 -
/// atan(x / sqrt(K1 - x²))) * K3 / K4` (`+0x172`, `+0x19b`, `+0x1b1`,
/// `+0x1b9`; `x` as a double, the square root and arctangent the game's
/// `MSVCR110`). Returns the `eax` slot 394 leaves (the libm calls in
/// between leave it).
///
/// Each slot call passes the words the original leaves on the stack.
pub fn tilt<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fl = |g: &G, a: u32| g.read::<f32>(a);
    // push ebp; and esp, -8; sub esp, 0x78
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x78);
    let (m, v, a, b) = (e + 0x30, e, e + 0x20, e + 0x10);
    view_matrix(g, img, img.rel32_target(f + 0x19), m, e.wrapping_sub(4));
    let mut stack: Vec<u32> = Vec::new();
    // each vector's four `mov dword [esp + n], imm32` (8 bytes each, the
    // immediate at +4) from these offsets
    for (stores, (table, out)) in [0x2b, 0x67, 0xa3, 0xdf].into_iter().zip([
        (0x27, 0x52),
        (0x63, 0x8e),
        (0x9f, 0xca),
        (0xdb, 0x106),
    ]) {
        for i in 0..4 {
            g.write::<u32>(v + 4 * i, w(stores + 8 * i + 4));
        }
        let s = slot_target(g, w(table), 229);
        for x in [v, m, w(out)] {
            stack.insert(0, x);
        }
        // call width: 12 (the earlier calls' words left on the stack)
        g.call(s, 0, &stack);
    }
    let s = slot_target(g, w(0x117), 232);
    for x in [w(0x111), a] {
        stack.insert(0, x);
    }
    // call width: 14
    g.call(s, 0, &stack);
    let s = slot_target(g, w(0x131), 232);
    for x in [m + 0x20, b] {
        stack.insert(0, x);
    }
    // call width: 16
    g.call(s, 0, &stack);
    let s = slot_target(g, w(0x14b), 394);
    let r = g.call(s, 0, &[a, b]);
    #[allow(clippy::cast_possible_truncation)]
    let x = r.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32);
    // the double x on the frame (the arctangent's numerator)
    let xd = f64::from(x);
    g.write::<u32>(e, xd.to_bits() as u32);
    g.write::<u32>(e + 4, (xd.to_bits() >> 32) as u32);
    let d = sub(fl(g, w(0x172)), mul(x, x));
    let root = f64::from(d).sqrt();
    let ang = g.libm(Libm::Atan, xd / root);
    #[allow(clippy::cast_possible_truncation)]
    let ang = ang as f32;
    let y = div(
        mul(sub(fl(g, w(0x19b)), ang), fl(g, w(0x1b1))),
        fl(g, w(0x1b9)),
    );
    g.write::<u32>(w(0x1c1), y.to_bits());
    Ret::eax(r.eax)
}

/// [`tilt`] (462 bytes, 1 overlay; the `rel32` at `+0x1c6` is
/// `__security_check_cookie`).
pub const TILT: Sig = Sig {
    len: 462,
    hash: 0xcc53_47a3_2a4b_62a4,
    calls: &[
        (0x19, Callee::Sig(&VIEW_MATRIX)),
        (0x181, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x193, Callee::Import("_libm_sse2_atan_precise")),
        (0x1c6, Callee::Any),
    ],
    imports: &[],
};

/// The camera's follow between two actors (0x10001730, no arguments, a
/// /GS frame): slot 210 `(E, m)` gives `E`'s camera matrix (`*` operand
/// `+0x16`) into a local `m` (`ebp - 0xa4`), slot 325 copies its row 3
/// into `a` (`ebp - 0x64`); slot 217 gives the positions `p` (`*P`,
/// `+0x4f`, into `ebp - 0x54`) and `q` (`*Q`, `+0x66`, into `ebp -
/// 0x44`); slot 234 `(b, q, p)` and slot 232 `(b, b)` make the direction
/// `b` (`ebp - 0x14`). With `h` = `*H + target_height` (`+0xa6`,
/// [`target_height`]), slot 364 `(E)` (its float dropped) and slot 268
/// `(E, 0x10, c)` (`ebp - 0x34`): `d` (`ebp - 0x24`) = `b * s`, `s =
/// f32(f64(h + K2) + D)` (`+0xe6`, the double at `+0xfc`), and slot 248
/// `(X, c, d)` (`+0x106`). Then `n = |trunc(c.y - a.y)|`; below `K4`
/// (`+0x152`) the global `+0x177` becomes `a.y - K4`, else it drops by
/// `K5` (`+0x195`). Returns `n`.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn pair_follow<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::mul;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let fl = |g: &G, a: u32| g.read::<f32>(a);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, a, p, q, b, c, d) = (
        ebp.wrapping_sub(0xa4),
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x54),
        ebp.wrapping_sub(0x44),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x34),
        ebp.wrapping_sub(0x24),
    );
    let e = g.read::<u32>(w(0x16));
    let mut stack = vec![e, m];
    let mut call = |g: &mut G, s: u32, words: &[u32]| {
        for &x in words.iter().rev() {
            stack.insert(0, x);
        }
        // call width: 17 (every earlier call's words stay on the stack)
        g.call(s, 0, &stack)
    };
    let s = table(g, 0x22, 210);
    call(g, s, &[]);
    let s = table(g, 0x38, 325);
    call(g, s, &[a, m + 0x30]);
    let pp = g.read::<u32>(w(0x4f));
    let s = table(g, 0x49, 217);
    call(g, s, &[pp, p]);
    let qq = g.read::<u32>(w(0x66));
    let s = table(g, 0x60, 217);
    call(g, s, &[qq, q]);
    let s = table(g, 0x7f, 234);
    call(g, s, &[b, q, p]);
    let s = table(g, 0x91, 232);
    call(g, s, &[b, b]);
    let r = target_height(g, img, img.rel32_target(f + 0x9e), 0);
    let th = match r.xmm0 {
        Some(Xmm::F32(x)) => f32::from_bits(x),
        _ => 0.0,
    };
    let h = add(fl(g, w(0xa6)), th);
    let s = table(g, 0xab, 364);
    call(g, s, &[e]);
    let s = table(g, 0xcb, 268);
    call(g, s, &[e, 0x10, c]);
    let hk = add(h, fl(g, w(0xe6)));
    let dk =
        f64::from_bits(u64::from(img.word(w(0xfc))) | (u64::from(img.word(w(0xfc) + 4)) << 32));
    // cvtps2pd; addsd; cvtpd2ps
    #[allow(clippy::cast_possible_truncation)]
    let scale = (f64::from(hk) + dk) as f32;
    for i in 0..3 {
        let x = mul(fl(g, b + 4 * i), scale);
        g.write::<u32>(d + 4 * i, x.to_bits());
    }
    let s = table(g, 0x101, 248);
    g.call(s, 0, &[w(0x106), c, d]);
    let ay = fl(g, a + 4);
    let x = sub(fl(g, c + 4), ay);
    let n = ffx_bridge::x87::cvttsd2si(f64::from(x))
        .cast_signed()
        .wrapping_abs();
    let k4 = fl(g, w(0x152));
    #[allow(clippy::cast_precision_loss)]
    if k4 > n as f32 {
        g.write::<u32>(w(0x177), sub(ay, k4).to_bits());
    } else {
        let y = sub(fl(g, w(0x18d)), fl(g, w(0x195)));
        g.write::<u32>(w(0x1a2), y.to_bits());
    }
    Ret::eax(n.cast_unsigned())
}

/// [`pair_follow`] (431 bytes, 1 overlay; the `rel32`s at `+0x181` and
/// `+0x1a7` are `__security_check_cookie`).
pub const PAIR_FOLLOW: Sig = Sig {
    len: 431,
    hash: 0xea9a_f24e_888e_6fe8,
    calls: &[
        (0x9e, Callee::Sig(&TARGET_HEIGHT)),
        (0x181, Callee::Any),
        (0x1a7, Callee::Any),
    ],
    imports: &[],
};

/// The fade's frame (0x10001f10, no arguments, a /GS frame): while the
/// clock (`+0x1b`, or `+0x27` when `*A` (`+0x12`) is 0) is past its mark
/// (0xaf, or 0x50) the level `*L` (`+0x30`) rises by `D` (`+0x42`), else
/// it falls by `D2` (`+0x97`); each new level is stored and held to
/// `K1..K2` (`+0x38`, `+0x57`/`+0xac`). Rising past `K2` sets it to 1.0
/// and returns 0, reaching `K2` exactly returns 0. Otherwise: slot 353
/// `(v)` with `v` = the level four times (`ebp - 0x14`), `y` = the level
/// (re-read) raised to `K1`, slot 801 `(z, z, z, z, z, z)` with `z = y /
/// *Z` (`+0x12d`), `*Z = y`, slot 785 `(S * y)` (`S` at `+0x16d`) and
/// slot 397 `(n, S * y)` with `n` four ints 0x80 (`ebp - 0x24`). Returns
/// 1.
pub fn fade_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let fl = |g: &G, at: u32| g.read::<f32>(w(at));
    let past = if g.read::<u32>(w(0x12)) != 0 {
        g.read::<i32>(w(0x1b)) > 0xaf
    } else {
        g.read::<i32>(w(0x27)) > 0x50
    };
    let k1 = fl(g, 0x38);
    let l = fl(g, 0x30);
    let x = if past {
        let x = add(l, fl(g, 0x42));
        g.write::<u32>(w(0x4d), x.to_bits());
        if k1 > x {
            g.write::<u32>(w(0xbc), k1.to_bits());
            k1
        } else {
            let k2 = fl(g, 0x57);
            if x > k2 {
                g.write::<u32>(w(0x62), w(0x66));
                return Ret::eax(0);
            }
            if x.partial_cmp(&k2) == Some(std::cmp::Ordering::Equal) {
                return Ret::eax(0);
            }
            x
        }
    } else {
        let x = sub(l, fl(g, 0x97));
        g.write::<u32>(w(0xa2), x.to_bits());
        let k2 = fl(g, 0xac);
        if k1 > x {
            g.write::<u32>(w(0xbc), k1.to_bits());
            k1
        } else if x > k2 {
            g.write::<u32>(w(0xbc), k2.to_bits());
            k2
        } else {
            x
        }
    };
    // push ebp; sub esp, 0x28: the locals below ebp
    let ebp = fr.esp.wrapping_sub(4);
    let (v, n, yl) = (ebp - 0x14, ebp - 0x24, ebp - 0x28);
    for i in 0..4 {
        g.write::<u32>(v + 4 * i, x.to_bits());
        g.write::<u32>(n + 4 * i, 0x80);
    }
    let s = table(g, 0xc5, 353);
    g.call(s, 0, &[v]);
    let l = fl(g, 0x105);
    let k1 = fl(g, 0x10d);
    // comiss k1, l; jb: the level unless K1 is not below it
    let y = if k1 < l || l.is_nan() { l } else { k1 };
    g.write::<u32>(yl, y.to_bits());
    let z = div(y, fl(g, 0x12d));
    let s = table(g, 0x132, 801);
    g.call(s, 0, &[z.to_bits(); 6]);
    let sy = mul(fl(g, 0x16d), y);
    g.write::<u32>(w(0x187), y.to_bits());
    let s = table(g, 0x172, 785);
    g.call(s, 0, &[sy.to_bits()]);
    let s = table(g, 0x197, 397);
    g.call(s, 0, &[n, sy.to_bits()]);
    Ret::eax(1)
}

/// [`fade_frame`] (441 bytes, 1 overlay; the `rel32`s at `+0x72`,
/// `+0x8b` and `+0x1b1` are `__security_check_cookie`).
pub const FADE_FRAME: Sig = Sig {
    len: 441,
    hash: 0x4aa6_d027_3e45_94f2,
    calls: &[
        (0x72, Callee::Any),
        (0x8b, Callee::Any),
        (0x1b1, Callee::Any),
    ],
    imports: &[],
};

/// The camera matrix (0x100018e0, internal: `edx` = three scales, a /GS
/// frame): slot 235 resets the global matrix `A` (`+0x20`) and two local
/// ones `M` (`ebp - 0x84`) and `S` (`ebp - 0xd4`); slot 234 `(t, P, Q)`
/// (`+0x5c`, `+0x51`) and slot 232 make the forward axis `F` (`ebp -
/// 0x34`); with `up` = `(0, 1, 0, 1)` (`ebp - 0x44`), slot 266 gives
/// `R = up x F` (`ebp - 0x14`) and `V = F x R` (`ebp - 0x24`), both
/// normalised (slot 232). `M`'s rows become `R`, `V`, `F` (their `w` 0)
/// and `(0, 0, 0, 1)`; slot 231 `(A, M, A)`, slot 395 `(A, A, P)`, `S`
/// gets the scales on its diagonal and slot 231 `(A, A, S)`. Returns the
/// last call's `eax`.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn camera_matrix<G: Game>(g: &mut G, img: &Image, f: u32, scales: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let ebp = esp.wrapping_sub(4);
    let (m, s, t, up, fw, r, v) = (
        ebp - 0x84,
        ebp - 0xd4,
        ebp - 0x94,
        ebp - 0x44,
        ebp - 0x34,
        ebp - 0x14,
        ebp - 0x24,
    );
    let a = w(0x20);
    let mut stack: Vec<u32> = Vec::new();
    let mut call = |g: &mut G, sl: u32, words: &[u32]| {
        for &x in words.iter().rev() {
            stack.insert(0, x);
        }
        // call width: 16 (the earlier calls' words stay on the stack)
        g.call(sl, 0, &stack).eax
    };
    let sl = table(g, 0x14, 235);
    call(g, sl, &[a]);
    let sl = table(g, 0x30, 235);
    call(g, sl, &[m]);
    let sl = table(g, 0x44, 235);
    call(g, sl, &[s]);
    for (i, x) in (0u32..).zip([0, 0x3f80_0000, 0, 0x3f80_0000]) {
        g.write::<u32>(up + 4 * i, x);
    }
    let sl = table(g, 0x62, 234);
    call(g, sl, &[t, w(0x5c), w(0x51)]);
    let sl = table(g, 0x96, 232);
    call(g, sl, &[fw, t]);
    let sl = table(g, 0xaf, 266);
    call(g, sl, &[r, up, fw]);
    let sl = table(g, 0xc8, 266);
    call(g, sl, &[v, fw, r]);
    let sl = table(g, 0xda, 232);
    call(g, sl, &[r, r]);
    let sl = table(g, 0xef, 232);
    g.call(sl, 0, &[v, v]);
    for (row, src) in [
        (0u32, Some(r)),
        (0x10, Some(v)),
        (0x20, Some(fw)),
        (0x30, None),
    ] {
        for i in 0..3 {
            let x = src.map_or(0, |p| g.read::<u32>(p + 4 * i));
            g.write::<u32>(m + row + 4 * i, x);
        }
        let last = if src.is_none() { 0x3f80_0000 } else { 0 };
        g.write::<u32>(m + row + 12, last);
    }
    let sl = table(g, 0x164, 231);
    g.call(sl, 0, &[w(0x1a1), m, w(0x158), v, v]);
    let sl = table(g, 0x1a8, 395);
    g.call(
        sl,
        0,
        &[w(0x1bd), w(0x1b8), w(0x1ad), w(0x1a1), m, w(0x158), v, v],
    );
    for (k, i) in (0u32..).zip([0u32, 5, 10]) {
        let x = g.read::<u32>(scales + 4 * k);
        g.write::<u32>(s + 4 * i, x);
    }
    let sl = table(g, 0x1dc, 231);
    // call width: 11 (the two earlier calls' words left on the stack)
    g.call(
        sl,
        0,
        &[
            w(0x201),
            w(0x1fc),
            s,
            w(0x1bd),
            w(0x1b8),
            w(0x1ad),
            w(0x1a1),
            m,
            w(0x158),
            v,
            v,
        ],
    )
    .eax
}

/// [`camera_matrix`] (537 bytes, 1 overlay; the `rel32` at `+0x211` is
/// `__security_check_cookie`).
pub const CAMERA_MATRIX: Sig = Sig {
    len: 537,
    hash: 0xeba7_0b0c_1f2b_cdc5,
    calls: &[(0x211, Callee::Any)],
    imports: &[],
};

/// A part's start (0x100023d0, cdecl `(part)`, a /GS frame): `**G` the
/// immediate `+0x1e`, the part started with `table[1]` of slot 941
/// `(*O + 0x10)` (the `pppStartPart` wrapper, `0x1000`, 1), slot 388
/// `(*X, 5, 1)` with the part's transform words set (`+0x40..+0x5c`, the
/// scale 0.04); slot 237 `(*Y, 0x2e, 0, q)` and slot 236 `(*Y, 0x25, m)`
/// into locals `q` (`ebp - 0x24`) and `m` (`ebp - 0x64`); `m` scaled by
/// 1000 (slot 235 resets a local `s`, `ebp - 0xa4`, its diagonal set,
/// slot 231 `(m, m, s)`), `p` = `(1.5, 0, 0, 0)` (`ebp - 0x14`) through
/// slot 229 `(p, m, p)` and slot 248 `(q, p, q)`; `m` scaled by 0.048 the
/// same way, three slot 325 copies (`r` at `ebp - 0xb4`, `u` at `ebp -
/// 0x44`), `m`'s translation cleared, slot 395 `(Z, m, q)` (`+0x23b`) and
/// slot 393 `(h, X, 0x100)` with `h` = `*H1` (`+0x265`), or `*H2`
/// (`+0x26d`) when `*F` (`+0x24b`) is 0. Returns 1.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn part_begin<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let part = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (q, m, s, p, r, u) = (
        ebp - 0x24,
        ebp - 0x64,
        ebp - 0xa4,
        ebp - 0x14,
        ebp - 0xb4,
        ebp - 0x44,
    );
    let gp = g.read::<u32>(w(0x14));
    g.write::<u32>(gp, w(0x1e));
    let owner = g.read::<u32>(g.read::<u32>(w(0x23)) + 0x10);
    let sl = table(g, 0x2b, 941);
    let tbl = g.call(sl, 0, &[owner]).eax;
    let program = g.read::<u32>(tbl.wrapping_add(4));
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x46), part, 0x1000, program, 1);
    let mut stack = vec![program, 1, 0, owner];
    let mut call = |g: &mut G, sl: u32, words: &[u32]| {
        for &x in words.iter().rev() {
            stack.insert(0, x);
        }
        // call width: 18 (the earlier calls' words stay on the stack)
        g.call(sl, 0, &stack)
    };
    let x = g.read::<u32>(w(0x55));
    for (off, v) in [
        (0x40, 0),
        (0x44, 0),
        (0x48, 0),
        (0x4c, 0x3f80_0000),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x60, 0x3d23_d70a),
        (0x64, 0x3d23_d70a),
        (0x68, 0x3d23_d70a),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(part + off, v);
    }
    let sl = table(g, 0x4b, 388);
    call(g, sl, &[x, 5, 1]);
    let y = g.read::<u32>(w(0xca));
    for (i, v) in (0u32..).zip([0x3fc0_0000, 0, 0, 0]) {
        g.write::<u32>(p + 4 * i, v);
    }
    let sl = table(g, 0xba, 237);
    call(g, sl, &[y, 0x2e, 0, q]);
    let y = g.read::<u32>(w(0xf9));
    let sl = table(g, 0xf1, 236);
    call(g, sl, &[y, 0x25, m]);
    let sl = table(g, 0x10d, 235);
    call(g, sl, &[s]);
    for i in [10u32, 5, 0] {
        g.write::<u32>(s + 4 * i, 0x447a_0000);
    }
    let sl = table(g, 0x126, 231);
    call(g, sl, &[m, m, s]);
    let mut stack: Vec<u32> = Vec::new();
    let mut call = |g: &mut G, sl: u32, words: &[u32]| {
        for &x in words.iter().rev() {
            stack.insert(0, x);
        }
        // call width: 16 (the earlier calls' words stay on the stack)
        g.call(sl, 0, &stack)
    };
    let sl = table(g, 0x15d, 229);
    call(g, sl, &[p, m, p]);
    let sl = table(g, 0x176, 248);
    call(g, sl, &[q, p, q]);
    let sl = table(g, 0x18a, 235);
    call(g, sl, &[s]);
    for i in [10u32, 5, 0] {
        g.write::<u32>(s + 4 * i, 0x3d44_9ba6);
    }
    let sl = table(g, 0x1a3, 231);
    call(g, sl, &[m, m, s]);
    let sl = table(g, 0x1d6, 325);
    call(g, sl, &[r, m]);
    let sl = table(g, 0x1eb, 325);
    call(g, sl, &[m, u]);
    let sl = table(g, 0x203, 325);
    call(g, sl, &[u, r]);
    for off in [0x38, 0x34, 0x30] {
        g.write::<u32>(m + off, 0);
    }
    let sl = table(g, 0x21b, 395);
    g.call(sl, 0, &[w(0x23b), m, q]);
    let h = if g.read::<u32>(w(0x24b)) == 0 {
        g.read::<u32>(w(0x26d))
    } else {
        g.read::<u32>(w(0x265))
    };
    let sl = table(g, 0x242, 393);
    g.call(sl, 0, &[h, w(0x25d), 0x100]);
    Ret::eax(1)
}

/// [`part_begin`] (649 bytes, 1 overlay; the `rel32` at `+0x281` is
/// `__security_check_cookie`).
pub const PART_BEGIN: Sig = Sig {
    len: 649,
    hash: 0xc734_672d_a3da_f577,
    calls: &[
        (0x46, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x281, Callee::Any),
    ],
    imports: &[],
};
