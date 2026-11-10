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

/// Where the walk's circle meets the line `P`..`Q` (`magic_0391`
/// 0x100016c0, 872 bytes; internal, `ecx` = the output point `O`, `edx` =
/// `P`, stack `(Q)`, `xmm3` = the radius `R`; an 8-aligned /GS frame, `B`
/// the aligned `esp` after `sub esp, 0x38`). The circle's centre `V` (`B +
/// 0x24`) is slot 292's point. With `dx = P.x - Q.x`, `dz = Q.z - P.z`,
/// `L = dz^2 + dx^2` and `c = (Q.x P.z - P.x Q.z) + (V.z dx + V.x dz)`,
/// the discriminants `D1 = (c dz)^2 - (c^2 - dx^2 R R) L` and `D2 = (c
/// dx)^2 - (c^2 - dz^2 R R) L`: either below 0, no point (returns 0).
/// Else the two roots (from `D1` in `x` when `|dx| >= |dz|` as doubles,
/// else from `D2` in `z`; the other coordinate from the line, negated),
/// moved by `V`: `O` = the first root (`y` 0, `w` 1.0), or the second when
/// the first is not on `Q`'s side of `P` in `x` or in `z` (a product below
/// 0 or NaN). Returns 1 when both discriminants are 0 (one point), else
/// 2.
pub fn circle_hit_0391<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    circle_hit(g, img, fr, fr.xmm_f32(3), HIT_0391)
}

/// [`circle_hit_0391`] in `magic_0448` (0x10004ec0, 869 bytes): the radius
/// the DLL's constant (operand `+0xd7`) instead of `xmm3`; the `x` side
/// test's product in the other operand order.
pub fn circle_hit_0448<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let r = g.read::<f32>(img.word(fr.at + 0xd7));
    circle_hit(g, img, fr, r, HIT_0448)
}

/// The operand offsets of a [`circle_hit_0391`] copy: slot 292, the sign
/// masks `(abs, neg A, neg B)`, and whether the `x` side test multiplies
/// `(Q - P) (O - P)`.
#[derive(Clone, Copy)]
struct HitOps {
    s292: u32,
    abs: [u32; 2],
    neg: [u32; 2],
    swap_x: bool,
}

const HIT_0391: HitOps = HitOps {
    s292: 0x1c,
    abs: [0x14f, 0x157],
    neg: [0x1a2, 0x235],
    swap_x: false,
};

const HIT_0448: HitOps = HitOps {
    s292: 0x1c,
    abs: [0x156, 0x15e],
    neg: [0x1a3, 0x232],
    swap_x: true,
};

fn circle_hit<G: Game>(g: &mut G, img: &Image, fr: &Frame, r: f32, o: HitOps) -> u32 {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |op: u32| img.word(f + op);
    let (out, p) = (fr.ecx, fr.edx);
    let q = fr.arg(g, 0);
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let v = b.wrapping_add(0x24);
    g.call(crate::runtime::slot_target(g, w(o.s292), 292), 0, &[v]);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let (px, pz, qx, qz) = (rd(g, p), rd(g, p + 8), rd(g, q), rd(g, q + 8));
    let (vx, vz) = (rd(g, v), rd(g, v + 8));
    let dz = sub(qz, pz);
    let dx = sub(px, qx);
    let cross = sub(mul(qx, pz), mul(px, qz));
    let c = add(cross, add(mul(vz, dx), mul(vx, dz)));
    let (dz2, dx2) = (mul(dz, dz), mul(dx, dx));
    let l = add(dz2, dx2);
    let (cdz, cdx) = (mul(c, dz), mul(c, dx));
    let c2 = mul(c, c);
    let d1 = sub(mul(cdz, cdz), mul(sub(c2, mul(mul(dx2, r), r)), l));
    let d2 = sub(mul(cdx, cdx), mul(sub(c2, mul(mul(dz2, r), r)), l));
    if 0.0 > d1 || 0.0 > d2 {
        return 0;
    }
    let mask = |g: &G, op: u32| g.read::<u32>(w(op));
    // `andpd` with the DLL's mask (its low quadword)
    let abs = |g: &G, k: usize, x: f32| {
        let m = g.read::<u64>(w(o.abs[k]));
        f64::from_bits(f64::from(x).to_bits() & m)
    };
    let neg = |g: &G, k: usize, x: f32| f32::from_bits(x.to_bits() ^ mask(g, o.neg[k]));
    // the two roots of one coordinate, and the other from the line
    let (ax_, az_) = (abs(g, 0, dx), abs(g, 1, dz));
    // `comisd`/`jb`: below or unordered solves for `z`
    let ((x1, z1), (x2, z2)) = if ax_ < az_ || ax_.is_nan() || az_.is_nan() {
        let s = f64::from(d2).sqrt();
        let z1 = ((s - f64::from(cdx)) / f64::from(l)) as f32;
        let z2 = ((f64::from(neg(g, 1, cdx)) - s) / f64::from(l)) as f32;
        let x1 = neg(g, 1, div(add(mul(z1, dx), c), dz));
        let x2 = neg(g, 1, div(add(mul(z2, dx), c), dz));
        ((x1, z1), (x2, z2))
    } else {
        let s = f64::from(d1).sqrt();
        let x1 = ((s - f64::from(cdz)) / f64::from(l)) as f32;
        let x2 = ((f64::from(neg(g, 0, cdz)) - s) / f64::from(l)) as f32;
        let z1 = neg(g, 0, div(add(mul(x1, dz), c), dx));
        let z2 = neg(g, 0, div(add(mul(x2, dz), c), dx));
        ((x1, z1), (x2, z2))
    };
    let (ax, az) = (add(vx, x1), add(vz, z1));
    let (bx, bz) = (add(vx, x2), add(vz, z2));
    g.write::<f32>(out, ax);
    g.write::<u32>(out + 4, 0);
    g.write::<f32>(out + 8, az);
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    let side_x = if o.swap_x {
        mul(sub(qx, px), sub(ax, px))
    } else {
        mul(sub(ax, px), sub(qx, px))
    };
    // `comiss`/`jb`: below 0 or unordered takes the second point
    let first = !(side_x < 0.0 || side_x.is_nan()) && {
        let side_z = mul(sub(az, pz), sub(qz, pz));
        !(side_z < 0.0 || side_z.is_nan())
    };
    if first {
        g.write::<f32>(out, ax);
        g.write::<f32>(out + 8, az);
    } else {
        g.write::<f32>(out, bx);
        g.write::<f32>(out + 8, bz);
    }
    if d1 == 0.0 && d2 == 0.0 { 1 } else { 2 }
}

/// [`circle_hit_0391`] (872 bytes, 1 overlay).
pub const CIRCLE_HIT_0391: Sig = Sig {
    len: 872,
    hash: 0xf79f_909a_43c6_915e,
    calls: &[
        (0x16f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x202, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x337, Callee::Any),
        (0x34d, Callee::Any),
        (0x360, Callee::Any),
    ],
    imports: &[],
};

/// [`circle_hit_0448`] (869 bytes, 1 overlay).
pub const CIRCLE_HIT_0448: Sig = Sig {
    len: 869,
    hash: 0x5193_6662_a509_93dd,
    calls: &[
        (0x170, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1ff, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x334, Callee::Any),
        (0x34a, Callee::Any),
        (0x35d, Callee::Any),
    ],
    imports: &[],
};

/// `K - atan2(dz, dx)` as the overlays compute it (`_CIatan2` of the
/// doubles, rounded to f32), wrapped once (`ops`: `K`, then the wrap's).
fn yaw_to<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    dz: f32,
    dx: f32,
    k: u32,
    wrap: [u32; 4],
) -> f32 {
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let x = crate::sb_sse::sub(g.read::<f32>(img.word(f + k)), a);
    crate::ob_model::wrap_once(g, img, f, x, wrap)
}

/// `c` moved by a turn towards `a` (the walk's unwrap; `hi`/`lo` the wrap
/// limits): with `a` below 0, `c` above 0 and `c - a` above `hi`, `lo -
/// (hi - c)`; with `a` not below 0, `c` below 0 and `a - c` above `hi`,
/// `c + hi + hi`.
fn unwrap_to(a: f32, c: f32, hi: f32, lo: f32) -> f32 {
    use crate::sb_sse::{add, sub};
    if 0.0 > a {
        if c > 0.0 && sub(c, a) > hi {
            return sub(lo, sub(hi, c));
        }
    } else if 0.0 > c && sub(a, c) > hi {
        return add(add(c, hi), hi);
    }
    c
}

/// `magic_0391`'s walk aim (0x10001a30, 1558 bytes; internal, `ecx` = the
/// walk state `S`; a /GS frame, `ebp = E - 4`, locals at `ebp - n`). The
/// segment `R` (`x0, z0, x1, z1`): with `S + 0x928` not -2, the entry
/// `S + 0x928` of the table `TA` (16 bytes each); else the entry `S +
/// 0x92c` of `TB` turned and moved by the battle character
/// `TAB[(*G_SEL)[0]]`: its facing `rot = K - slot 333` (wrapped), its
/// place `C` (slot 262), each end `(x, 0, z, 1)` turned by Y `rot`
/// (slots 235, 328, 329 into `M`, slot 229) and added to `C` (slot 248),
/// the place read again for the second. The aim point `(X, Z)` is the
/// far end when `S + 0x934` is set and bit 0 of `S + 0x910` clear, else
/// the near one. Then `S + 0x750` = `(X, S.y, Z, 1)`, `S + 0x760` = the
/// segment's middle (`K5` along it) at `S.y`, `S + 0x720` = `(0, the
/// caster's facing, 0, 0)`, `S + 0x730` = `(0, a, 0, 0)` with `a` the yaw
/// from `S + 0x7a0` to the aim point; `c` the yaw to the battle
/// character, unwrapped towards `a` ([`unwrap_to`]); `S + 0x90c` =
/// whether `c < a` (or unordered), which it returns.
/// (`S + 0x73c` is a frame word the `TA` path never writes: 0, brief
/// 2026-10-24; the `TB` path writes 0.0 there.)
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn walk_aim_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    const ONE: u32 = 0x3f80_0000;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let sel = |g: &G, sel_op: u32, tab_op: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(sel_op))));
        g.read::<u32>(w(tab_op).wrapping_add(4 * i))
    };
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let idx = g.read::<u32>(s + 0x928);
    let r = if idx == 0xffff_fffe {
        let chr = sel(g, 0x41, 0x54);
        let out = g.call(t(g, 0x59, 333), 0, &[chr]);
        let rot = sub(k(g, 0x6f), st0_f32(out.st0));
        let rot = crate::ob_model::wrap_once(g, img, f, rot, [0x87, 0x99, 0xa3, 0xb0]);
        let src = w(0x5f).wrapping_add(g.read::<u32>(s + 0x92c) << 4);
        let (c, m, u) = (l(0x24), l(0x88), l(0x14));
        // each end: (x, 0, z, 1) turned by Y rot (into W) added to C
        let end = |g: &mut G, k: u32, [s262, sel_op, tab_op]: [u32; 3], ops: [u32; 5], wv: u32| {
            let chr = sel(g, sel_op, tab_op);
            let place = [chr, c, c + 4, c + 8];
            if k == 0 {
                g.call(t(g, s262, 262), 0, &place);
            }
            let (x, z) = (g.read::<u32>(src + 8 * k), g.read::<u32>(src + 8 * k + 4));
            for (o, v) in [(0u32, x), (4, 0), (8, z), (0xc, ONE)] {
                g.write::<u32>(u + o, v);
            }
            g.write::<u32>(c + 0xc, ONE);
            // call width: 5 (slot 262's words stay under it)
            g.call(t(g, ops[0], 235), 0, &[m, chr, c, c + 4, c + 8]);
            g.call(t(g, ops[1], 328), 0, &[m, m, 0]);
            g.call(t(g, ops[2], 329), 0, &[m, m, rot.to_bits()]);
            g.call(t(g, ops[3], 229), 0, &[wv, m, u]);
            g.call(t(g, ops[4], 248), 0, &[c, c, wv]);
        };
        end(
            g,
            0,
            [0xd8, 0xbd, 0xd3],
            [0x105, 0x13e, 0x163, 0x182, 0x1a2],
            l(0x38),
        );
        let (x0, chr) = (g.read::<u32>(c), sel(g, 0x1af, 0x1d4));
        g.write::<u32>(l(0x48), x0);
        g.write::<u32>(c + 0xc, ONE);
        let z0 = g.read::<u32>(c + 8);
        g.write::<u32>(l(0x44), z0);
        g.call(t(g, 0x1d9, 262), 0, &[chr, c, c + 4, c + 8]);
        end(
            g,
            1,
            [0x1d9, 0x1af, 0x1d4],
            [0x20b, 0x244, 0x269, 0x28b, 0x2ae],
            l(0x98),
        );
        let (x1, z1) = (g.read::<u32>(c), g.read::<u32>(c + 8));
        g.write::<u32>(l(0x40), x1);
        g.write::<u32>(l(0x3c), z1);
        l(0x48)
    } else {
        // junk read: [ebp-0x8c] never written; 0 (brief 2026-10-24)
        w(0x2f).wrapping_add(idx << 4)
    };
    let far = g.read::<u32>(s + 0x934) != 0 && g.read::<u8>(s + 0x910) & 1 == 0;
    let (x, z) = if far {
        (rf(g, r + 8), rf(g, r + 0xc))
    } else {
        (rf(g, r), rf(g, r + 4))
    };
    let (sx, sy, sz) = (rf(g, s + 0x7a0), g.read::<u32>(s + 0x7a4), rf(g, s + 0x7a8));
    let a = yaw_to(
        g,
        img,
        f,
        sub(z, sz),
        sub(x, sx),
        0x3a2,
        [0x3b1, 0x3c3, 0x3cd, 0x3da],
    );
    let (r0, r1, r2, r3) = (rf(g, r), rf(g, r + 4), rf(g, r + 8), rf(g, r + 0xc));
    let mx = add(mul(sub(r2, r0), k(g, 0x40f)), r0);
    let mz = add(mul(sub(r3, r1), k(g, 0x417)), r1);
    g.write::<u32>(s + 0x764, sy);
    g.write::<u32>(s + 0x76c, ONE);
    g.write::<f32>(s + 0x760, mx);
    g.write::<f32>(s + 0x768, mz);
    g.write::<f32>(s + 0x750, x);
    g.write::<u32>(s + 0x754, sy);
    g.write::<f32>(s + 0x758, z);
    g.write::<u32>(s + 0x75c, ONE);
    g.write::<u32>(s + 0x720, 0);
    let caster = g.read::<u32>(w(0x47c));
    let out = g.call(t(g, 0x407, 333), 0, &[caster]);
    let rot2 = sub(k(g, 0x48f), st0_f32(out.st0));
    let rot2 = crate::ob_model::wrap_once(g, img, f, rot2, [0x49e, 0x4a8, 0x4b2, 0x4bf]);
    g.write::<f32>(s + 0x724, rot2);
    g.write::<u32>(s + 0x728, 0);
    g.write::<u32>(s + 0x72c, 0);
    g.write::<f32>(s + 0x734, a);
    g.write::<u32>(s + 0x730, 0);
    g.write::<u32>(s + 0x738, 0);
    g.write::<u32>(s + 0x73c, 0);
    let chr = sel(g, 0x514, 0x52a);
    let d = l(0x14);
    g.call(t(g, 0x52f, 262), 0, &[chr, d, d + 4, d + 8]);
    let c = yaw_to(
        g,
        img,
        f,
        sub(rf(g, d + 8), sz),
        sub(rf(g, d), sx),
        0x579,
        [0x581, 0x59e, 0x589, 0x5ad],
    );
    let a4 = rf(g, s + 0x734);
    let c = unwrap_to(a4, c, k(g, 0x581), k(g, 0x589));
    // `comiss`/`setb`: below or unordered
    let below = u32::from(c.partial_cmp(&a4).is_none_or(std::cmp::Ordering::is_lt));
    g.write::<u32>(s + 0x90c, below);
    below
}

/// [`walk_aim_0391`] (1558 bytes, 1 overlay).
pub const WALK_AIM_0391: Sig = Sig {
    len: 1558,
    hash: 0x120e_5e1a_1739_b526,
    calls: &[
        (0x38c, Callee::Import("_CIatan2")),
        (0x569, Callee::Import("_CIatan2")),
        // __security_check_cookie
        (0x60e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0448`'s walk aim (0x10005230, 1481 bytes; internal, `ecx` = the
/// walk state `S`; an 8-aligned /GS frame, `E1` the aligned `esp` after
/// `sub esp, 0xbc` and three pushes). The segment's two ends `P0`, `P1`
/// (16-byte points): with `S + 0x924` not -2, its entries in the tables
/// `TA0`, `TA1`; else `S + 0x928`'s entries of `TB0`, `TB1` (their `w`
/// set to 1.0 in the table) turned by Y `rot = K - slot 333` (wrapped) of
/// the battle character `TAB[(*G_SEL)[0]]` (slots 235, 328, 329 into `M`,
/// slot 229) and added to its place (slot 262, read once per end; slot
/// 248), kept at `E1 + 0x40` and `+0x50`. The aim point is `P1` when `S +
/// 0x930` is set and bit 0 of `S + 0x90c` clear, else `P0`; then as
/// [`walk_aim_0391`]: `S + 0x750` the aim point (with its own `y`), `S +
/// 0x760` the middle (`K5` along, all three coordinates), `S + 0x720` =
/// `(0, the caster's facing, 0, 0)`, `S + 0x730` = `(0, a, 0, 0)`, the yaw
/// to the battle character unwrapped towards `a` ([`unwrap_to`]); `S +
/// 0x908` = whether it is below `a`, which it returns. (`S + 0x73c`: the
/// `TA` path's frame word is never written: 0, brief 2026-10-24.)
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn walk_aim_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    const ONE: u32 = 0x3f80_0000;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let e1 = (esp.wrapping_sub(4) & !7).wrapping_sub(0xbc + 0xc);
    let sel = |g: &G, sel_op: u32, tab_op: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(sel_op))));
        g.read::<u32>(w(tab_op).wrapping_add(4 * i))
    };
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let (p0, p1) = if g.read::<u32>(s + 0x924) == 0xffff_fffe {
        let i = g.read::<u32>(s + 0x928) << 4;
        let (u0, u1) = (w(0x55).wrapping_add(i), w(0x5b).wrapping_add(i));
        let chr = sel(g, 0x60, 0x6a);
        let out = g.call(t(g, 0x6f, 333), 0, &[chr]);
        let rot = sub(k(g, 0x83), st0_f32(out.st0));
        let rot = crate::ob_model::wrap_once(g, img, f, rot, [0x93, 0xa3, 0xad, 0xba]);
        let m = e1 + 0x80;
        let end = |g: &mut G,
                   c: u32,
                   u: u32,
                   wv: u32,
                   [sel_op, tab_op, s262]: [u32; 3],
                   ops: [u32; 5]| {
            let chr = sel(g, sel_op, tab_op);
            g.call(t(g, s262, 262), 0, &[chr, c, c + 4, c + 8]);
            g.write::<u32>(c + 0xc, ONE);
            // call width: 5 (slot 262's words stay under it)
            g.call(t(g, ops[0], 235), 0, &[m, chr, c, c + 4, c + 8]);
            g.call(t(g, ops[1], 328), 0, &[m, m, 0]);
            g.call(t(g, ops[2], 329), 0, &[m, m, rot.to_bits()]);
            g.write::<u32>(u + 0xc, ONE);
            g.call(t(g, ops[3], 229), 0, &[wv, m, u]);
            g.call(t(g, ops[4], 248), 0, &[c, c, wv]);
        };
        end(
            g,
            e1 + 0x40,
            u0,
            e1 + 0x60,
            [0xc5, 0xde, 0xe3],
            [0x101, 0x129, 0x14d, 0x168, 0x187],
        );
        end(
            g,
            e1 + 0x50,
            u1,
            e1 + 0x70,
            [0x194, 0x1b5, 0x1ba],
            [0x1cf, 0x1fa, 0x21e, 0x23c, 0x25e],
        );
        g.write::<u32>(e1 + 0x5c, ONE);
        (e1 + 0x40, e1 + 0x50)
    } else {
        // junk read: [esp+0x7c] (E1 + 0x7c) never written; 0 (brief 2026-10-24)
        let i = g.read::<u32>(s + 0x924) << 4;
        (w(0x3b).wrapping_add(i), w(0x41).wrapping_add(i))
    };
    let far = g.read::<u32>(s + 0x930) != 0 && g.read::<u8>(s + 0x90c) & 1 == 0;
    let p = if far { p1 } else { p0 };
    let (x, y, z) = (rf(g, p), g.read::<u32>(p + 4), rf(g, p + 8));
    let (sx, sz) = (rf(g, s + 0x7a0), rf(g, s + 0x7a8));
    let a = yaw_to(
        g,
        img,
        f,
        sub(z, sz),
        sub(x, sx),
        0x33a,
        [0x349, 0x359, 0x363, 0x370],
    );
    let kh = k(g, 0x39a);
    let mid = |g: &G, o: u32| add(mul(sub(rf(g, p1 + o), rf(g, p0 + o)), kh), rf(g, p0 + o));
    let (mx, my, mz) = (mid(g, 0), mid(g, 4), mid(g, 8));
    g.write::<f32>(s + 0x760, mx);
    g.write::<u32>(s + 0x76c, ONE);
    g.write::<f32>(s + 0x764, my);
    g.write::<f32>(s + 0x768, mz);
    g.write::<f32>(s + 0x750, x);
    g.write::<u32>(s + 0x754, y);
    g.write::<f32>(s + 0x758, z);
    g.write::<u32>(s + 0x75c, ONE);
    g.write::<u32>(s + 0x720, 0);
    let caster = g.read::<u32>(w(0x41f));
    let out = g.call(t(g, 0x3a5, 333), 0, &[caster]);
    let rot2 = sub(k(g, 0x433), st0_f32(out.st0));
    let rot2 = crate::ob_model::wrap_once(g, img, f, rot2, [0x443, 0x44d, 0x457, 0x464]);
    g.write::<f32>(s + 0x724, rot2);
    g.write::<u32>(s + 0x728, 0);
    g.write::<u32>(s + 0x72c, 0);
    g.write::<f32>(s + 0x734, a);
    g.write::<u32>(s + 0x730, 0);
    g.write::<u32>(s + 0x738, 0);
    g.write::<u32>(s + 0x73c, 0);
    let chr = sel(g, 0x4b5, 0x4ce);
    let d = e1 + 0x30;
    g.call(t(g, 0x4d3, 262), 0, &[chr, d, d + 4, d + 8]);
    let c = yaw_to(
        g,
        img,
        f,
        sub(rf(g, d + 8), sz),
        sub(rf(g, d), sx),
        0x527,
        [0x52f, 0x54c, 0x537, 0x55b],
    );
    let a4 = rf(g, s + 0x734);
    let c = unwrap_to(a4, c, k(g, 0x52f), k(g, 0x537));
    // `comiss`/`setb`: below or unordered
    let below = u32::from(c.partial_cmp(&a4).is_none_or(std::cmp::Ordering::is_lt));
    g.write::<u32>(s + 0x908, below);
    below
}

/// [`walk_aim_0448`] (1481 bytes, 1 overlay).
pub const WALK_AIM_0448: Sig = Sig {
    len: 1481,
    hash: 0xd737_c527_28bf_bfcb,
    calls: &[
        (0x328, Callee::Import("_CIatan2")),
        (0x515, Callee::Import("_CIatan2")),
        // __security_check_cookie
        (0x5c1, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0391`'s walk step (0x10002ab0, 305 bytes; internal, `ecx` = the
/// walk state `S`; an 8-aligned /GS frame, `E1` the aligned `esp` after
/// `sub esp, 0x28` and two pushes). With `S + 0x91c` 0 or the count `S +
/// 0x918` not above 0, returns 1. Else the caster's place `P` (slot 262
/// `(*G_A, ..)` into `E1 + 0xc`, `w` 1.0) moved by `(S + 0x780, S +
/// 0x788)` in `x` and `z`; the state `W = *G_W`: when `W + 0xbb0` is 1
/// and the battle point `V` (slot 292, `E1 + 0x1c`) is at least
/// `W + 0xb88 - K` away from `P` (flat), nothing is placed; else the caster goes to
/// `P` ([`place_me`](crate::sa_misc::place_me)). The count steps down;
/// returns whether it reached 0 (or below).
pub fn walk_step_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    if g.read::<u32>(s + 0x91c) == 0 || g.read::<i32>(s + 0x918) <= 0 {
        return 1;
    }
    let e1 = (esp.wrapping_sub(4) & !7).wrapping_sub(0x28 + 8);
    let p = e1 + 0xc;
    let chr = g.read::<u32>(w(0x56));
    g.call(t(g, 0x50, 262), 0, &[chr, p, p + 4, p + 8]);
    let ww = g.read::<u32>(w(0x72));
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let x = add(g.read::<f32>(s + 0x780), g.read::<f32>(p));
    g.write::<f32>(p, x);
    let z = add(g.read::<f32>(s + 0x788), g.read::<f32>(p + 8));
    g.write::<f32>(p + 8, z);
    let mut place = true;
    if g.read::<u32>(ww + 0xbb0) == 1 {
        let v = e1 + 0x1c;
        g.call(t(g, 0xaa, 292), 0, &[v]);
        g.write::<u32>(v + 0xc, 0x3f80_0000);
        let dx = sub(g.read::<f32>(v), x);
        let dz = sub(g.read::<f32>(v + 8), z);
        let d = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt() as f32;
        let lim = sub(g.read::<f32>(ww + 0xb88), g.read::<f32>(w(0xfd)));
        // `comiss`/`jae`: not placed only when ordered and not below
        place = d.partial_cmp(&lim).is_none_or(std::cmp::Ordering::is_lt);
    }
    if place {
        crate::sa_misc::place_me(g, img, img.rel32_target(f + 0x10b), p);
    }
    let n = g.read::<i32>(s + 0x918).wrapping_sub(1);
    g.write::<i32>(s + 0x918, n);
    u32::from(n <= 0)
}

/// [`walk_step_0391`] (305 bytes, 1 overlay).
pub const WALK_STEP_0391: Sig = Sig {
    len: 305,
    hash: 0xbe0b_9251_7d52_5314,
    calls: &[
        (0xe6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x10b, Callee::Sig(&crate::sa_misc::PLACE_ME)),
        // __security_check_cookie
        (0x2f, Callee::Any),
        (0x129, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0448`'s walk step (0x100064d0, 554 bytes; internal, `ecx` = the
/// walk state `S`; an 8-aligned /GS frame, `E1` the aligned `esp` after
/// `sub esp, 0x54` and one push). With `S + 0x918` 0 or the count `S +
/// 0x914` not above 0, returns 1. Else the caster's place `P` (slot 262
/// `(*G_A, ..)` into `E1 + 0x34`, `w` 1.0); towards the point `S + 0x7a0`
/// the pitch `atan2(dy, sqrt(dz^2 + dx^2))` (the root kept as a double)
/// and the yaw `K - atan2(dz, dx)` (wrapped); `P` moved by the point `(0,
/// 0, -3.5)` turned by them ([`turn_point`](crate::s4_small::turn_point),
/// the angles at `E1 + 0x10`). The state `W = *G_W`: when `W + 0xbac` is
/// 1 and the battle point `V` (slot 292, `E1 + 0x20`) is at least `K2`
/// away from `P` (flat), nothing is placed; else the caster goes to `P`
/// ([`place_me`](crate::sa_misc::place_me)). The count steps down;
/// returns whether it reached 0 (or below).
pub fn walk_step_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    if g.read::<u32>(s + 0x918) == 0 || g.read::<i32>(s + 0x914) <= 0 {
        return 1;
    }
    let e1 = (esp.wrapping_sub(4) & !7).wrapping_sub(0x54 + 4);
    let p = e1 + 0x34;
    let chr = g.read::<u32>(w(0x54));
    g.call(t(g, 0x4e, 262), 0, &[chr, p, p + 4, p + 8]);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let dy = sub(rf(g, s + 0x7a4), rf(g, p + 4));
    let dx = sub(rf(g, s + 0x7a0), rf(g, p));
    let dz = sub(rf(g, s + 0x7a8), rf(g, p + 8));
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let h = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt();
    let pitch = g.atan2(f64::from(dy), h) as f32;
    let yaw = yaw_to(g, img, f, dz, dx, 0x128, [0x137, 0x141, 0x14b, 0x158]);
    let a = e1 + 0x10;
    g.write::<f32>(a, pitch);
    g.write::<f32>(a + 4, yaw);
    g.write::<u32>(a + 8, 0);
    let u = e1 + 0x44;
    for (i, v) in (0u32..).zip([0, 0, 0xc060_0000, 0x3f80_0000]) {
        g.write::<u32>(u + 4 * i, v);
    }
    // push U: the return address at E1 - 8
    g.write::<u32>(e1.wrapping_sub(4), u);
    let turn = Frame {
        at: img.rel32_target(f + 0x198),
        esp: e1.wrapping_sub(8),
        ecx: a,
        edx: p,
        ..Frame::default()
    };
    crate::s4_small::turn_point(g, img, &turn);
    let ww = g.read::<u32>(w(0x19d));
    let mut place = true;
    if g.read::<u32>(ww + 0xbac) == 1 {
        let v = e1 + 0x20;
        g.call(t(g, 0x1b3, 292), 0, &[v]);
        g.write::<u32>(v + 0xc, 0x3f80_0000);
        let dx = sub(rf(g, v), rf(g, p));
        let dz = sub(rf(g, v + 8), rf(g, p + 8));
        let d = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt() as f32;
        // `comiss`/`jae`: not placed only when ordered and not below
        place = d
            .partial_cmp(&rf(g, w(0x1fa)))
            .is_none_or(std::cmp::Ordering::is_lt);
    }
    if place {
        crate::sa_misc::place_me(g, img, img.rel32_target(f + 0x205), p);
    }
    let n = g.read::<i32>(s + 0x914).wrapping_sub(1);
    g.write::<i32>(s + 0x914, n);
    u32::from(n <= 0)
}

/// [`walk_step_0448`] (554 bytes, 1 overlay).
pub const WALK_STEP_0448: Sig = Sig {
    len: 554,
    hash: 0xe6df_71e7_b311_f67c,
    calls: &[
        (0xc4, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xd7, Callee::Import("_CIatan2")),
        (0x116, Callee::Import("_CIatan2")),
        (0x198, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x1ef, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x205, Callee::Sig(&crate::sa_misc::PLACE_ME)),
        // __security_check_cookie
        (0x2d, Callee::Any),
        (0x222, Callee::Any),
    ],
    imports: &[],
};
