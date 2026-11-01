//! `magic_0666.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{start_pair, start_part};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Starts a spark part (0x10001bd0, an overlay table entry, `(obj)`;
/// five table entries): [`start_pair`] with program 10; `+0x40`/`+0x44`
/// 0, `+0x4c`/`+0x6c` 1.0; the first part `+0x48` 2.0 and the colour
/// 0x3c4d5b68, any other -3.0000262 and 0x3b75b1c8; `+0x50..+0x58` 0,
/// `+0x5c` 1. Returns 1.
pub fn spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = 0x3c4d_5b68;
    let d = 0x3b75_b1c8;
    start_pair(
        g,
        img,
        fr,
        (0x36, 0x28),
        &[(0x40, 0), (0x44, 0), (0x4c, ONE), (0x6c, ONE)],
        &[(0x48, 0x4000_0000), (0x60, c), (0x64, c), (0x68, c)],
        &[(0x48, 0xc040_006d), (0x60, d), (0x64, d), (0x68, d)],
        &[(0x50, 0), (0x54, 0), (0x58, 0), (0x5c, 1)],
    )
}

/// [`spark`] (197 bytes, 1 overlay).
pub const SPARK: Sig = Sig {
    len: 197,
    hash: 0x649d_324e_f27b_55ac,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// Starts a ring part (0x10002370, an overlay table entry, `(obj)`; two
/// table entries): [`start_pair`] with program 3; `+0x44` -0.01, `+0x4c`
/// 1.0; the first part `+0x40`/`+0x48` 2.83 and -3.53, any other -4.27
/// and 1.67; `+0x50..+0x58` 0, `+0x5c` 1, the colour 0x3e018ca0,
/// `+0x6c` 1.0. Returns 1.
pub fn ring<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = 0x3e01_8ca0;
    start_pair(
        g,
        img,
        fr,
        (0x36, 0xc),
        &[(0x44, 0xbc23_d70a), (0x4c, ONE)],
        &[(0x40, 0x4035_1eb0), (0x48, 0xc061_eb81)],
        &[(0x40, 0xc088_a3d5), (0x48, 0x3fd5_c287)],
        &[
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, c),
            (0x64, c),
            (0x68, c),
            (0x6c, ONE),
        ],
    )
}

/// [`ring`] (183 bytes, 1 overlay).
pub const RING: Sig = Sig {
    len: 183,
    hash: 0xf951_860b_69ec_7b2a,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// Starts a flare part (0x10002430, an overlay table entry, `(obj)`;
/// two table entries): [`start_pair`] with program 11; `+0x44` -0.01,
/// `+0x4c`/`+0x6c` 1.0, the colour 0x3c01cf78, `+0x58` 0, `+0x5c` 1; the
/// first part `+0x40`/`+0x48` -17.4 and -21.14, `+0x50` 0, `+0x54`
/// 0x7054, any other -12.4, -28.04, -910 and -10920. Returns 1.
pub fn flare<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = 0x3c01_cf78;
    start_pair(
        g,
        img,
        fr,
        (0x36, 0x2c),
        &[
            (0x44, 0xbc23_d70a),
            (0x4c, ONE),
            (0x60, c),
            (0x64, c),
            (0x68, c),
            (0x6c, ONE),
            (0x58, 0),
            (0x5c, 1),
        ],
        &[
            (0x40, 0xc18b_331e),
            (0x48, 0xc1a9_1eaf),
            (0x50, 0),
            (0x54, 0x7054),
        ],
        &[
            (0x40, 0xc146_6638),
            (0x48, 0xc1e0_51c5),
            (0x50, 0xffff_fc72),
            (0x54, 0xffff_d558),
        ],
        &[],
    )
}

/// [`flare`] (198 bytes, 1 overlay).
pub const FLARE: Sig = Sig {
    len: 198,
    hash: 0xccac_7fab_d9de_24cf,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's start (0x10002290, an overlay table entry, `(obj)`): the
/// array's first object (within 0x80 bytes of operand `+0x0a`) starts
/// program 8 with the scales 0.0046, any other program 9 with 0.0093
/// ([`start_part`]); then `+0x6c` 1.0, the point `+0x40..` = `(-7.08,
/// -0.01, 1.97, 1)`, `+0x50..+0x58` 0 and `+0x5c` 1. Returns 1.
pub fn start_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let first = obj.wrapping_sub(w(0xa)) & 0xffff_ff80 == 0;
    let (call, index, scale) = if first {
        (0x44, 0x20, 0x3b97_fe8f)
    } else {
        (0x70, 0x24, 0x3c17_fe8f)
    };
    start_part(g, img, f, obj, [0x14, 0x1a, 0x1f, 0x27, call], index, &[]);
    for o in [0x60u32, 0x64, 0x68] {
        g.write::<u32>(obj + o, scale);
    }
    for (o, v) in [
        (0x6cu32, 0x3f80_0000u32),
        (0x40, 0xc0e2_8f52),
        (0x44, 0xbc23_d2d8),
        (0x48, 0x3ffc_28dd),
        (0x4c, 0x3f80_0000),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
    ] {
        g.write::<u32>(obj + o, v);
    }
    Ret::eax(1)
}

/// [`start_b`] (211 bytes, 1 overlay).
pub const START_B: Sig = Sig {
    len: 211,
    hash: 0x3af2_79cc_934b_902a,
    calls: &[
        (0x44, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x70, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// The effect's frame on its cues (0x10002a90, an overlay table entry, no
/// arguments): nothing (returns 0) until the run flag (`+0x02`) is set;
/// else `*G_P = ITEM`, slot 723 `(*G_A)` and slot 724 `(*G_B)` (the first
/// call's word under it), `*G_Q = *G_V`; then for the cues 0..3 (slot
/// 261 `(k)`): cue 0 runs timeline 1; cue 1 stops timeline 1 once (its
/// flag) and runs timeline 2; cue 2 stops timeline 2 once and runs
/// timeline 3; cue 3 stops timeline 3 and ends the effect (slot 214
/// `(*G_E)`). Without cue 3 the frame counter counts up. Returns 1
/// ([`crate::sb_misc::timeline_run`], [`crate::sb_misc::timeline_stop`]).
pub fn cue_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x0d));
    let a = g.read::<u32>(w(0x13));
    g.write::<u32>(p, w(0x19));
    let t = slot_target(g, w(0x1e), 723);
    // call width: 1
    g.call(t, 0, &[a]);
    let b = g.read::<u32>(w(0x31));
    let t = slot_target(g, w(0x2b), 724);
    // call width: 2 (the first call's word under it)
    g.call(t, 0, &[b, a]);
    let q = g.read::<u32>(w(0x3f));
    let v = g.read::<u32>(w(0x44));
    g.write::<u32>(q, v);
    let cue = |g: &mut G, table: u32, k: u32| {
        let t = slot_target(g, w(table), 261);
        // call width: 3 at most (the earlier calls' words under it)
        let words = [k, b, a];
        let n = if k == 0 { 3 } else { 1 };
        g.call(t, 0, &words[..n]).eax != 0
    };
    let run = |g: &mut G, call: u32, tl: u32| {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + call), w(tl));
    };
    if cue(g, 0x4b, 0) {
        run(g, 0x66, 0x61);
    }
    // (the cue's table, its stop-once flag and immediate, the timeline
    // stopped and the stop's call, the timeline run and the run's call)
    for (table, flag, stop, run_at) in [
        (0x6bu32, 0x82u32, (0x8au32, 0x99u32), (0x9eu32, 0xa3u32)),
        (0xa8, 0xbf, (0xc7, 0xd6), (0xdb, 0xe0)),
    ] {
        let k = if table == 0x6b { 1 } else { 2 };
        if !cue(g, table, k) {
            continue;
        }
        if g.read::<u32>(w(flag)) == 0 {
            g.write::<u32>(w(flag + 0xe), w(flag + 0x12));
            crate::sb_misc::timeline_stop(g, w(stop.0));
        }
        run(g, run_at.1, run_at.0);
    }
    if cue(g, 0xe5, 3) {
        crate::sb_misc::timeline_stop(g, w(0xfb));
        let e = g.read::<u32>(w(0x10b));
        let t = slot_target(g, w(0x105), 214);
        // call width: 1
        g.call(t, 0, &[e]);
        return Ret::eax(1);
    }
    let n = g.read::<u32>(w(0x122)).wrapping_add(1);
    g.write::<u32>(w(0x122), n);
    Ret::eax(1)
}

/// [`cue_frame`] (300 bytes, 1 overlay).
pub const CUE_FRAME: Sig = Sig {
    len: 300,
    hash: 0x3f4c_1ff1_bf6e_5e3f,
    calls: &[
        (0x66, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x99, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xa3, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xd6, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xe0, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x100, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// An object drawn on the level look matrix (0x10001330, an overlay table
/// entry, `(obj)`; a /GS frame with the matrix `M` at `ebp - 0x44`): the
/// matrix ([`crate::s6_misc::level_look_matrix`], height `K`, scale `obj +
/// 0x60`) from `*G_S`, or `*G_T` for an object not within the 0x80 bytes
/// from `BASE`; slot 230 `(*G_A, M)`, slot 231 `(*G_C, *G_B, *G_A)` and
/// slot 230 `(*G_D, *G_C)` (each seeing the earlier words, down to the
/// saved register under slot 231's); `*G_P = ITEM`; slot 712 `(obj,
/// 0x30)` with the game flag `*G_LOCK` 1 around it; when that is nonzero,
/// `*G_P = ITEM`, slot 713 `(obj)`, `*G_P = ITEM` again and part 13 of
/// slot 941 `((*G_Q)[0x10])` started on `obj`
/// ([`crate::opu_parts::start_part`], `0x1000`, `(entry, 1)`). Returns 1.
pub fn look_draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, op: u32| g.read::<u32>(w(op));
    let obj = fr.arg(g, 0);
    let first = obj.wrapping_sub(w(0x2a)) & 0xffff_ff80 == 0;
    let src = if first { r(g, 0x1a) } else { r(g, 0x3a) };
    let ebp = fr.esp.wrapping_sub(4);
    let m = ebp.wrapping_sub(0x44);
    let height = g.read::<u32>(w(0x14));
    let scale = g.read::<u32>(obj.wrapping_add(0x60));
    // edi pushed under the frame; the return address
    let callee = Frame {
        at: img.rel32_target(f + 0x3f),
        esp: m.wrapping_sub(8),
        ecx: m,
        edx: src,
        xmm: [0, 0, u128::from(height), u128::from(scale)],
        ..Frame::default()
    };
    crate::s6_misc::level_look_matrix(g, img, &callee);
    let a = r(g, 0x4e);
    let t = slot_target(g, w(0x48), 230);
    // call width: 2
    g.call(t, 0, &[a, m]);
    // push esi (a saved register) under the next calls' words: they get
    // the words above it
    let words = [r(g, 0x77), r(g, 0x71), r(g, 0x6b)];
    let t = slot_target(g, w(0x5b), 231);
    // call width: 3
    g.call(t, 0, &words);
    let mut stack = words.to_vec();
    stack.splice(0..0, [r(g, 0x8a), r(g, 0x7f)]);
    let t = slot_target(g, w(0x84), 230);
    // call width: 5
    g.call(t, 0, &stack);
    let p = r(g, 0x97);
    g.write::<u32>(p, w(0x9f));
    let lock = r(g, 0xa4);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [obj, 0x30]);
    let t = slot_target(g, w(0xb2), 712);
    // call width: 7
    let done = g.call(t, 0, &stack).eax;
    let lock = r(g, 0xc0);
    g.write::<u32>(lock, saved);
    if done == 0 {
        return Ret::eax(1);
    }
    let p = r(g, 0xcf);
    g.write::<u32>(p, w(0xd6));
    let t = slot_target(g, w(0xdb), 713);
    // call width: 1
    g.call(t, lock, &[obj]);
    let p = r(g, 0xe8);
    g.write::<u32>(p, w(0xee));
    let q = r(g, 0xf3);
    let d = g.read::<u32>(q.wrapping_add(0x10));
    let t = slot_target(g, w(0xfb), 941);
    // call width: 2 (slot 713's word under it)
    let list = g.call(t, lock, &[d, obj]).eax;
    let entry = g.read::<u32>(list.wrapping_add(0x34));
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x116), obj, 0x1000, entry, 1);
    Ret::eax(1)
}

/// [`look_draw`] (305 bytes, 1 overlay).
pub const LOOK_DRAW: Sig = Sig {
    len: 305,
    hash: 0x09da_09f6_de05_5395,
    calls: &[
        (0x3f, Callee::Sig(&crate::s6_misc::LEVEL_LOOK_MATRIX)),
        (0x116, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x129, Callee::Any),
    ],
    imports: &[],
};
