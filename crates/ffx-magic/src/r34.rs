//! Round 34's ports (magic-r34): the shared rows and the own rows of
//! overlays below `magic_0450`.

use crate::r23::{PairLayout, TurnLayout};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0162`'s state words against `magic_0598`'s.
const PAIR_0162: PairLayout = PairLayout {
    caster: 0x9d0,
    frames: [0x950, 0x970],
    ang: 0x85c8,
    cams: [0x9e0, 0x9f0],
    summon_s17: 0x472,
};

/// `magic_0162`'s placement of the caster and the summon (0x100021d0,
/// 2074 bytes; internal, no arguments, called by [`start_up_0162`]; a
/// cookie frame): [`place_pair_0598`](crate::r23::place_pair_0598)'s
/// steps on 0162's state words ([`PAIR_0162`]: the caster's place `S +
/// 0x9d0`, the frames `S + 0x950` and `+0x970`, the angle `S + 0x85c8`,
/// the camera points `S + 0x9e0` and `+0x9f0`); then the flat mesh
/// coordinates ([`flat_coords_0586`](crate::r25::flat_coords_0586), 0162's
/// copy writing from `S + 0xa30`), `S + 0xa00..0xa30`, `+0x85c0`,
/// `+0x85c4` and `+0x85ec` cleared and four flags set. Returns the mesh
/// coordinates' `eax`.
pub fn place_pair_0162<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let (_, s) = crate::r23::place_pair_with(g, img, f, esp, &PAIR_0162);
    // the call's return address under the 0xbc-byte frame and three saves
    let eax = crate::r25::flat_coords_0586(
        g,
        img,
        img.rel32_target(f + 0x747),
        esp.wrapping_sub(4 + 0xbc + 0xc + 4),
    );
    for o in (0xa00u32..0xa30).step_by(4).chain([0x85c0, 0x85c4, 0x85ec]) {
        g.write::<u32>(s + o, 0);
    }
    for o in [0x7eau32, 0x7f4, 0x7fe, 0x808] {
        g.write::<u32>(img.word(f + o), 1);
    }
    eax
}

/// [`place_pair_0162`] (2,074 bytes, 1 overlay).
pub const PLACE_PAIR_0162: Sig = Sig {
    len: 2074,
    hash: 0x7880_0540_e317_f437,
    calls: &[
        (0x747, Callee::Sig(&crate::r30::FLAT_COORDS_0162)),
        // __security_check_cookie
        (0x812, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162`'s turn layout against `magic_0598`'s: the turns at `S +
/// 0x950`, the angle `S + 0x85c8`, the camera point `S + 0x960`, the copy
/// `S + 0x9b0`, the second model `S + 0x910` of the third selected
/// character.
const TURN_LAYOUT_0162: TurnLayout = TurnLayout {
    turns: 0x950,
    ang: 0x85c8,
    cam: 0x960,
    copy: 0x9b0,
    m2: 0x910,
    sel2: 2,
};

/// `magic_0162`'s turn (0x10004b60, 961 bytes; a table entry, no
/// arguments; a cookie frame): [`turn_0598`](crate::r23::turn_0598)'s
/// steps on [`TURN_LAYOUT_0162`], closing with a scale task
/// ([`scale_task`](crate::s4_misc::scale_task) of `S + 0x8460`, the stack
/// word 0x55, `xmm2`/`xmm1` the DLL's constants) instead of the fade
/// task. Returns the task's node.
pub fn turn_0162<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (s, esp) = crate::r23::turn_with(g, img, fr, &TURN_LAYOUT_0162);
    let w = |o: u32| img.word(fr.at + o);
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(w(0x394)));
    xmm[1] = u128::from(g.read::<u32>(w(0x39c)));
    let task = Frame {
        at: img.rel32_target(fr.at + 0x3a9),
        esp,
        ecx: s + 0x8460,
        xmm,
        ..*fr
    };
    crate::s4_misc::scale_task(g, img, &task)
}

/// [`turn_0162`] (961 bytes, 1 overlay).
pub const TURN_0162: Sig = Sig {
    len: 961,
    hash: 0x33cf_3116_7fd6_5858,
    calls: &[
        (0x3a9, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        // __security_check_cookie
        (0x3b9, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162`'s per-frame task (0x10002030, 341 bytes; cdecl `(node)`):
/// [`task_0600`](crate::r23::task_0600)'s steps with four points copied
/// (`S + 0x960`, `+0x980`, `+0x9a0`, `+0x9c0` to `S + 0x880`, `+0x8c0`,
/// `+0x900`, `+0x940`) and four motion scripts before the camera rigs'
/// step (a tail jump); once the first script is done, `S + 0x85ec` = 1
/// and the node goes back to the pool (a tail jump, no effect end).
/// Returns the tail jump's answer.
pub fn task_0162<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x7));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), h | 0x80);
    let v = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x33), v);
    let first = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x38), w(0x2e));
    crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x44), w(0x3d));
    if first != 0 {
        g.write::<u32>(s + 0x85ec, 1);
        let node = fr.arg(g, 0);
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x151),
            node,
        ));
    }
    for (from, to) in [
        (0x960u32, 0x880u32),
        (0x980, 0x8c0),
        (0x9a0, 0x900),
        (0x9c0, 0x940),
    ] {
        for i in 0..4 {
            let x = g.read::<u32>(s + from + 4 * i);
            g.write::<u32>(s + to + 4 * i, x);
        }
    }
    for (op, call) in [
        (0x10bu32, 0x116u32),
        (0x11b, 0x120),
        (0x125, 0x12a),
        (0x12f, 0x134),
    ] {
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + call), w(op));
    }
    let rigs = Frame {
        at: img.rel32_target(f + 0x13c),
        ..*fr
    };
    crate::s8_misc::camera_rigs_step(g, img, &rigs)
}

/// [`task_0162`] (341 bytes, 1 overlay).
pub const TASK_0162: Sig = Sig {
    len: 341,
    hash: 0x31a6_ea04_dd06_abe1,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x116, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x120, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x12a, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x134, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x13c, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x151, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0162`'s start-up (0x10004f70, 346 bytes; no arguments, called
/// by its load step): [`start_up_0586`](crate::r27::start_up_0586)'s code
/// byte for byte, calling [`place_pair_0162`] for the caster frames and
/// 0162's turned start.
pub fn start_up_0162<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r27::start_up_with_0586(g, img, fr, place_pair_0162)
}

/// [`start_up_0162`] (346 bytes, 1 overlay).
pub const START_UP_0162: Sig = Sig {
    len: 346,
    hash: 0x31cb_33ff_cf4e_729c,
    calls: &[
        (0x60, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x65, Callee::Sig(&PLACE_PAIR_0162)),
        (0x6a, Callee::Sig(&crate::o4_effect::START_TURNED_0162_29F0)),
        (0xce, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10c, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x116, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0162`'s load step (0x100013a0, the 118-byte shape) running
/// [`start_up_0162`].
pub fn load_0162<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0162)
}

/// [`load_0162`].
pub const LOAD_0162: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0162))],
    imports: &[],
};

/// The index of `x` in the -1-ended word table at `table`, or -1.
fn find_word<G: Game>(g: &G, table: u32, x: u32) -> u32 {
    let mut i = 0u32;
    loop {
        let v = g.read::<u32>(table.wrapping_add(4 * i));
        if v == u32::MAX {
            return u32::MAX;
        }
        if v == x {
            return i;
        }
        i = i.wrapping_add(1);
    }
}

/// The caster's place and facing as the `magic_0391` / `magic_0448` walk
/// starts read them (`S = *G_S`, `P = S + 0x260`): slot 262 `(*G_A, P,
/// P + 4, P + 8)`, `P.w` 1.0, the facing `K - slot 333 (*G_A)` wrapped once
/// kept at `S + ang` (operand offsets: `S`, the first character, slot 262,
/// the second character, slot 333, `K`; then the wrap's). Returns `S`.
fn caster_facing<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    [s_op, a1, s262, a2, s333, k]: [u32; 6],
    wrap: [u32; 4],
    ang: u32,
) -> u32 {
    use crate::ob_model::{st0_f32, wrap_once};
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(s_op));
    let p = s.wrapping_add(0x260);
    let chr = g.read::<u32>(w(a1));
    let stack = [chr, p, p + 4, p + 8];
    g.call(crate::runtime::slot_target(g, w(s262), 262), 0, &stack);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let chr2 = g.read::<u32>(w(a2));
    // call width: 5 (slot 262's words stay under it)
    let out = g.call(
        crate::runtime::slot_target(g, w(s333), 333),
        0,
        &[chr2, chr, p, p + 4, p + 8],
    );
    let x = crate::sb_sse::sub(g.read::<f32>(w(k)), st0_f32(out.st0));
    let x = wrap_once(g, img, f, x, wrap);
    g.write::<f32>(s.wrapping_add(ang), x);
    s
}

/// `magic_0391`'s start of the battle walk (0x100013c0, 609 bytes;
/// internal, no arguments, called by its start-up; an 8-aligned /GS
/// frame, `B` the aligned `esp` after `sub esp, 0x1c`): the caster's place
/// and facing ([`caster_facing`], kept at `S + 0xce0`); slot 520's answer
/// `a` looked up in the table `T1` (index into `S + 0xba8`); when not
/// found and `a` is not 0x2050000, the reach `R` = `K4` if `a` is in `T2`
/// (else `K3`), the battle character `slot 390 ((*G_SEL)[0])` looked up in
/// `T3` (found: `S + 0xba8` = -2, `S + 0xbac` = its index) and `S + 0xbb0`
/// = 1, else `R = K3` and `S + 0xbb0` = 0. `S + 0xb88` = `R`; slot 292
/// writes the point `V` (`B + 8`), and when its flat distance to the
/// caster `sqrt((V.z - P.z)^2 + (V.x - P.x)^2)` is above `R`, `S + 0xbb0`
/// is cleared; `S + 0xbb4` cleared; slot 542 for each actor
/// ([`each_542`](crate::s4_small::each_542), `ecx` = -1); `S + 0xce4` =
/// -1, `+0xce8` 0, `*G_F` 1. Returns the last slot 542's `eax` (slot 292's
/// when there are none).
pub fn walk_start_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let b = (esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    let s = caster_facing(
        g,
        img,
        f,
        [0x18, 0x33, 0x2c, 0x4d, 0x40, 0x61],
        [0x71, 0x7b, 0x85, 0x92],
        0xce0,
    );
    let a = g.call(t(g, 0x97, 520), 0, &[]).eax;
    let idx = find_word(g, w(0xb9), a);
    g.write::<u32>(s + 0xba8, idx);
    let mut reach = k(g, 0xd9);
    if idx == u32::MAX && a != 0x0205_0000 {
        if find_word(g, w(0x104), a) != u32::MAX {
            reach = k(g, 0x125);
        }
        let c = u32::from(g.read::<u8>(g.read::<u32>(w(0x130))));
        let r = g.call(t(g, 0x139, 390), 0, &[c]).eax;
        let j = find_word(g, w(0x151), r);
        if j != u32::MAX {
            g.write::<u32>(s + 0xba8, 0xffff_fffe);
            g.write::<u32>(s + 0xbac, j);
        }
        g.write::<u32>(s + 0xbb0, 1);
    } else {
        g.write::<u32>(s + 0xbb0, 0);
    }
    g.write::<f32>(s + 0xb88, reach);
    // the point above the three saved registers: B + 8
    let v = b.wrapping_add(8);
    let eax = g.call(t(g, 0x1b9, 292), 0, &[v]).eax;
    let dz = sub(g.read::<f32>(v + 8), g.read::<f32>(s + 0x268));
    let dx = sub(g.read::<f32>(v), g.read::<f32>(s + 0x260));
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let sq = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise` of the double: exact
    let d = f64::from(sq).sqrt() as f32;
    if d > g.read::<f32>(s + 0xb88) {
        g.write::<u32>(s + 0xbb0, 0);
    }
    g.write::<u32>(s + 0xbb4, 0);
    let each = Frame {
        at: img.rel32_target(f + 0x22d),
        esp: b.wrapping_sub(0xc + 4),
        eax,
        ecx: u32::MAX,
        ..Frame::default()
    };
    let eax = crate::s4_small::each_542(g, img, &each).eax;
    g.write::<u32>(s + 0xce4, u32::MAX);
    g.write::<u32>(s + 0xce8, 0);
    g.write::<u32>(w(0x250), 1);
    eax
}

/// [`walk_start_0391`] (609 bytes, 1 overlay).
pub const WALK_START_0391: Sig = Sig {
    len: 609,
    hash: 0x1ed8_ad25_4ebc_9787,
    calls: &[
        (0x204, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x22d, Callee::Sig(&crate::s4_small::EACH_542)),
        // __security_check_cookie
        (0x259, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0448`'s start of the battle walk (0x10004cc0, 359 bytes;
/// internal, no arguments, called by its start-up; `ebp = E - 4`, no
/// cookie): the caster's place and facing ([`caster_facing`], kept at
/// `S + 0xcd0`); slot 520's answer looked up in `T1` (index into `S +
/// 0xba4`); when not found, the battle character `slot 390
/// ((*G_SEL)[0])` looked up in `T3` (found: `S + 0xba4` = -2, `S + 0xba8`
/// = its index). `S + 0xbac`, `+0xbb0` cleared, slot 542 for each actor
/// ([`each_542`](crate::s4_small::each_542), `ecx` = -1), `S + 0xcd4` =
/// -1, `+0xcd8` 0, `*G_F` 1. Returns the last slot 542's `eax` (the last
/// slot answer when there are none).
pub fn walk_start_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let s = caster_facing(
        g,
        img,
        f,
        [0x8, 0x22, 0x1b, 0x3c, 0x2f, 0x4f],
        [0x5e, 0x68, 0x72, 0x7f],
        0xcd0,
    );
    let a = g.call(t(g, 0x84, 520), 0, &[]).eax;
    let idx = find_word(g, w(0xa5), a);
    g.write::<u32>(s + 0xba4, idx);
    let mut eax = a;
    if idx == u32::MAX {
        let c = u32::from(g.read::<u8>(g.read::<u32>(w(0xd1))));
        eax = g.call(t(g, 0xda, 390), 0, &[c]).eax;
        let j = find_word(g, w(0xf2), eax);
        if j != u32::MAX {
            g.write::<u32>(s + 0xba4, 0xffff_fffe);
            g.write::<u32>(s + 0xba8, j);
        }
    }
    g.write::<u32>(s + 0xbac, 0);
    g.write::<u32>(s + 0xbb0, 0);
    // push ebp, ecx, esi, edi: the return address at E - 0x14
    let each = Frame {
        at: img.rel32_target(f + 0x13f),
        esp: esp.wrapping_sub(0x14),
        eax,
        ecx: u32::MAX,
        ..Frame::default()
    };
    let eax = crate::s4_small::each_542(g, img, &each).eax;
    g.write::<u32>(s + 0xcd4, u32::MAX);
    g.write::<u32>(s + 0xcd8, 0);
    g.write::<u32>(w(0x15a), 1);
    eax
}

/// [`walk_start_0448`] (359 bytes, 1 overlay).
pub const WALK_START_0448: Sig = Sig {
    len: 359,
    hash: 0x87fb_af29_89bb_3dee,
    calls: &[(0x13f, Callee::Sig(&crate::s4_small::EACH_542))],
    imports: &[],
};

/// `magic_0391`'s start-up (0x10003d10, 296 bytes; no arguments, called by
/// its load step; no frame): the store run, slot 719 `(P, 8, 0)` into
/// `G_H`; the part set up ([`part_setup`](crate::sb_misc::part_setup)
/// `(BUF)`, slot 719's `0` under it); the mesh flags of the model `*G_H`
/// ([`mesh_flags`](crate::o4_fam4::mesh_flags)); the walk start
/// ([`walk_start_0391`]); the script blocks' words; slot 271 `(4)`; two
/// task nodes (kinds 1 and 2, [`take`](crate::sa_pool::take)) put last on
/// the task list, the records block set up after the first
/// ([`records_init`](crate::sa_misc::records_init)). Returns 0.
/// (`magic_0586`'s start-up shape, [`start_up_0586`](crate::r27::start_up_0586),
/// with fewer stores.)
pub fn start_up_0391<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_up_walk(g, img, fr, true, walk_start_0391)
}

/// [`start_up_0391`] in `magic_0448` (0x100074e0, 285 bytes): no mesh
/// flags (the operands after `+0x50` 11 bytes earlier), the walk start
/// [`walk_start_0448`].
pub fn start_up_0448<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_up_walk(g, img, fr, false, walk_start_0448)
}

fn start_up_walk<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    mesh: bool,
    walk: fn(&mut G, &Image, u32, u32) -> u32,
) -> Ret {
    let f = fr.at;
    let at = |o: u32| if mesh || o < 0x50 { o } else { o - 11 };
    let w = |o: u32| img.word(f + at(o));
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + at(o));
    let s719 = t(g, 0x1, 719);
    crate::r25::abs_stores(g, img, f, 0x14, 0x3c);
    // call width: 3
    let h = g.call(s719, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x42), h);
    // slot 719's last word stays; push BUF: the return address at `E - 0xc`
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 8, 0);
    g.write::<u32>(esp + 4, w(0x47));
    crate::sb_misc::part_setup(g, img, call(0x4c), esp);
    if mesh {
        let model = g.read::<u32>(w(0x52));
        crate::o4_fam4::mesh_flags(g, img, call(0x57), model);
    }
    walk(g, img, call(0x5c), esp);
    let s271 = t(g, 0x61, 271);
    crate::r25::abs_stores(g, img, f, at(0x65), at(0x89));
    g.call(s271, 0, &[4]);
    let append = |g: &mut G, n: u32, kind: u8, ops: [u32; 5]| {
        g.write::<u8>(n + 0x18, kind);
        g.write::<u32>(n + 4, 0);
        if g.read::<u32>(w(ops[0])) == 0 {
            g.write::<u32>(n, 0);
            g.write::<u32>(w(ops[2]), n);
            g.write::<u32>(n + 8, n);
            g.write::<u32>(w(ops[4]), n);
        } else {
            let last = g.read::<u32>(w(ops[1]));
            g.write::<u32>(n, last);
            g.write::<u32>(last + 4, n);
            g.write::<u32>(n + 8, n);
            g.write::<u32>(w(ops[3]), n);
        }
    };
    let n1 = crate::sa_pool::take(g, img, call(0x9c), w(0x97));
    append(g, n1, 1, [0xa2, 0xb6, 0xc8, 0xd5, 0xd5]);
    crate::sa_misc::records_init(g, img, call(0xda), w(0xcd));
    let n2 = crate::sa_pool::take(g, img, call(0xe4), w(0xdf));
    append(g, n2, 2, [0xea, 0xfe, 0x119, 0x10b, 0x121]);
    Ret::eax(0)
}

/// [`start_up_0391`] (296 bytes, 1 overlay).
pub const START_UP_0391: Sig = Sig {
    len: 296,
    hash: 0x90cf_2b3e_61f8_138e,
    calls: &[
        (0x4c, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x57, Callee::Sig(&crate::o4_fam4::MESH_FLAGS_0391_3E40)),
        (0x5c, Callee::Sig(&WALK_START_0391)),
        (0x9c, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xda, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0xe4, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// [`start_up_0448`] (285 bytes, 1 overlay).
pub const START_UP_0448: Sig = Sig {
    len: 285,
    hash: 0x8196_c7a9_f758_e6d8,
    calls: &[
        (0x4c, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x51, Callee::Sig(&WALK_START_0448)),
        (0x91, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xcf, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0xd9, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0391`'s load step (0x10001210, the 118-byte shape) running
/// [`start_up_0391`].
pub fn load_0391<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0391)
}

/// [`load_0391`].
pub const LOAD_0391: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0391))],
    imports: &[],
};

/// `magic_0448`'s load step (0x10004b10, the 118-byte shape) running
/// [`start_up_0448`].
pub fn load_0448<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0448)
}

/// [`load_0448`].
pub const LOAD_0448: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0448))],
    imports: &[],
};
