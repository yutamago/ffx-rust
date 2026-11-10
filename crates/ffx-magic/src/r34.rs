//! Round 34's ports (magic-r34): the shared rows and the own rows of
//! overlays below `magic_0450`.

use crate::r23::{PairLayout, TurnLayout};
use crate::r32::Shift;
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

/// The parts of a [`drop_0391`] copy that differ: the record's `+0x14`
/// (`-slot 404 (0.25)`, or `-(slot 314 (0.5) + K)`), its `+0x4c`/`+0x50`,
/// whether `+0x6b` is cleared, and the operand offsets (slot 314's after
/// the first record word, the record's `+0x24` immediate, `K1`/`K2` of
/// `+0x3c`, the sign mask, `+0x14`'s slot and constant).
struct DropOps {
    take: u32,
    s314: [u32; 6],
    s14: u32,
    k14: Option<u32>,
    neg: u32,
    imm24: u32,
    k3c: [u32; 2],
    w4c: [u32; 2],
    clear_6b: bool,
}

const DROP_0391: DropOps = DropOps {
    take: 0x79,
    s314: [0x9e, 0xc7, 0xf6, 0x13c, 0x158, 0x16f],
    s14: 0x119,
    k14: None,
    neg: 0x137,
    imm24: 0x17d,
    k3c: [0x1aa, 0x1c4],
    w4c: [0x20, 0],
    clear_6b: false,
};

const DROP_0448: DropOps = DropOps {
    take: 0x49,
    s314: [0x6e, 0x97, 0xc6, 0x10d, 0x130, 0x147],
    s14: 0xe9,
    k14: Some(0x108),
    neg: 0x11b,
    imm24: 0x155,
    k3c: [0x182, 0x19c],
    w4c: [0x18, 0x18],
    clear_6b: true,
};

/// `magic_0391`'s spark drop (0x10002bf0, 486 bytes; internal, no
/// arguments; a /GS frame, `ebp = E - 4`): the handle `H` = slot 719 `(P,
/// 9, 0)`; the caster's place (slot 262 `(*G_A, ..)`) and its height
/// (slot 12, ST0) less `K`; one record of the DLL's pool
/// ([`pool_take`](crate::sb_misc::pool_take); none: returns 0): `+0x30`
/// `H`, `+0x34` 1, the halfwords `+0x38` -1 and `+0x3a` 0x200, the place
/// `(x + rnd 5, height, z + rnd 5, 1)` (slot 314 random floats, ST0),
/// `+0x10` rnd 0.25, `+0x14` `-slot 404 (0.25)`, `+0x18` rnd 0.25,
/// `+0x1c` rnd pi, `+0x20` `(0, 0.01, 0, 1)`, `+0x3c` `(rnd 0.2 + K1) *
/// K2`, `+0x4c` 0x20, `+0x50` 0. Returns the last slot 314's `eax`.
pub fn drop_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let h = g.call(t(g, 0x11, 719), 0, &[w(0x22), 9, 0]).eax;
    let p = esp.wrapping_sub(4 + 0x14);
    let chr = g.read::<u32>(w(0x30));
    // call width: 7 (slot 719's words stay under it)
    g.call(t(g, 0x2a, 262), 0, &[chr, p, p + 4, p + 8, w(0x22), 9, 0]);
    // call width: 8
    let out = g.call(
        t(g, 0x4d, 12),
        0,
        &[chr, chr, p, p + 4, p + 8, w(0x22), 9, 0],
    );
    let y = crate::sb_sse::sub(crate::ob_model::st0_f32(out.st0), g.read::<f32>(w(0x63)));
    drop_record(g, img, f, h, p, y, &DROP_0391)
}

/// [`drop_0391`] in `magic_0448` (0x10006700, 450 bytes): `H` the word
/// `+0xc` of the block `*G_P` (read, no slot 719); no height (the
/// place's own `y`); `+0x14` = `-(rnd 0.5 + K)`, `+0x24` -0.02, `+0x4c`
/// and `+0x50` 0x18, the byte `+0x6b` cleared.
pub fn drop_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let h = g.read::<u32>(g.read::<u32>(w(0x11)) + 0xc);
    let p = esp.wrapping_sub(4 + 0x14);
    let chr = g.read::<u32>(w(0x30));
    g.call(
        crate::runtime::slot_target(g, w(0x2a), 262),
        0,
        &[chr, p, p + 4, p + 8],
    );
    let y = g.read::<f32>(p + 4);
    drop_record(g, img, f, h, p, y, &DROP_0448)
}

fn drop_record<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    h: u32,
    p: u32,
    y: f32,
    o: &DropOps,
) -> u32 {
    use crate::sb_sse::{add, mul};
    let w = |op: u32| img.word(f + op);
    let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + o.take));
    if rec == 0 {
        return 0;
    }
    let mut eax = 0;
    let mut rnd = |g: &mut G, op: u32, slot: u32, x: f32| {
        let out = g.call(
            crate::runtime::slot_target(g, w(op), slot),
            0,
            &[x.to_bits()],
        );
        eax = out.eax;
        crate::ob_model::st0_f32(out.st0)
    };
    g.write::<u32>(rec + 0x30, h);
    g.write::<u16>(rec + 0x38, 0xffff);
    g.write::<u16>(rec + 0x3a, 0x200);
    g.write::<u32>(rec + 0x34, 1);
    let r = rnd(g, o.s314[0], 314, 5.0);
    let x = add(g.read::<f32>(p), r);
    g.write::<f32>(rec, x);
    g.write::<f32>(rec + 4, y);
    let r = rnd(g, o.s314[1], 314, 5.0);
    let z = add(g.read::<f32>(p + 8), r);
    g.write::<u32>(rec + 0xc, 0x3f80_0000);
    g.write::<f32>(rec + 8, z);
    let r = rnd(g, o.s314[2], 314, 0.25);
    g.write::<f32>(rec + 0x10, r);
    let v = match o.k14 {
        None => rnd(g, o.s14, 404, 0.25),
        Some(k) => {
            let r = rnd(g, o.s14, 314, 0.5);
            add(r, g.read::<f32>(w(k)))
        }
    };
    g.write::<u32>(rec + 0x14, v.to_bits() ^ g.read::<u32>(w(o.neg)));
    let r = rnd(g, o.s314[3], 314, 0.25);
    g.write::<f32>(rec + 0x18, r);
    let r = rnd(g, o.s314[4], 314, std::f32::consts::PI);
    g.write::<f32>(rec + 0x1c, r);
    g.write::<u32>(rec + 0x20, 0);
    g.write::<u32>(rec + 0x24, img.word(f + o.imm24));
    g.write::<u32>(rec + 0x28, 0);
    g.write::<u32>(rec + 0x2c, 0x3f80_0000);
    let r = rnd(g, o.s314[5], 314, 0.2);
    g.write::<u32>(rec + 0x4c, o.w4c[0]);
    g.write::<u32>(rec + 0x50, o.w4c[1]);
    let v = mul(
        add(r, g.read::<f32>(w(o.k3c[0]))),
        g.read::<f32>(w(o.k3c[1])),
    );
    if o.clear_6b {
        g.write::<u8>(rec + 0x6b, 0);
    }
    g.write::<f32>(rec + 0x3c, v);
    eax
}

/// [`drop_0391`] (486 bytes, 1 overlay).
pub const DROP_0391_SIG: Sig = Sig {
    len: 486,
    hash: 0x20a2_1bfe_1605_5ac5,
    calls: &[
        (0x79, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x1de, Callee::Any),
    ],
    imports: &[],
};

/// [`drop_0448`] (450 bytes, 1 overlay).
pub const DROP_0448_SIG: Sig = Sig {
    len: 450,
    hash: 0xa33b_ebf8_7924_0dd8,
    calls: &[
        (0x49, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x1ba, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0391`'s curve step (0x10002de0, 954 bytes; internal, `ecx` = the
/// walk state `S`; an 8-aligned /GS frame, `E1` the aligned `esp` after
/// `sub esp, 0x30` and two pushes). The count `S + 0x918` steps down
/// (when above 0); with `u = count / S + 0x91c` (as floats) and `v = K1 -
/// u` the point `C` (`E1 + 0x14`) on the curve from `P0 = S + 0x7b0`
/// towards `P1 = S + 0x7c0`, per coordinate with `d = P1 - P0`: `((v u
/// v v) K3 (d Ka) + v v v v d + (u u v v) K3 (d Kb) + (u u v u) K3 (d Kc) +
/// u u u u 0) + P0`, f32 at every step in that order. With the state `W =
/// *G_W`'s flag `W + 0xbb0` set and the battle point (slot 292, `E1 +
/// 0x24`) at least `W + 0xb88 - K` from `C` (flat), the caster stays;
/// else it goes to `C` (inlined [`place_me`](crate::sa_misc::place_me):
/// slot 255, or slots 11 and 33 / 32). Below 20 steps left a spark drops
/// ([`drop_0391`]). Returns whether the count is 0 (or below).
pub fn curve_step_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    curve_step(g, img, f, esp, s, false, drop_0391)
}

/// [`curve_step_0391`] in `magic_0448` (0x100068d0, 938 bytes): the count
/// `S + 0x914` of `S + 0x918`, the flag `W + 0xbac`, the reach the
/// constant `K` (operand `+0x2da`), the drop [`drop_0448`].
pub fn curve_step_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    curve_step(g, img, f, esp, s, true, drop_0448)
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)] // the three coordinates; `cvtdq2ps`
fn curve_step<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    s: u32,
    copy: bool,
    drop: fn(&mut G, &Image, u32, u32) -> u32,
) -> u32 {
    use crate::sb_sse::{add, div, mul, sub};
    let at = |o: u32| match (copy, o) {
        (false, _) => o,
        (true, o) if o < 0x1c4 => o,
        (true, o) if o < 0x2f0 => o - 1,
        (true, o) => o - 0x10,
    };
    let w = |o: u32| img.word(f + at(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let (count, total, flag) = if copy {
        (0x914, 0x918, 0xbac)
    } else {
        (0x918, 0x91c, 0xbb0)
    };
    let e1 = (esp.wrapping_sub(4) & !7).wrapping_sub(0x30 + 8);
    let n = g.read::<i32>(s + count);
    if n > 0 {
        g.write::<i32>(s + count, n - 1);
    }
    let u = div(
        g.read::<i32>(s + count) as f32,
        g.read::<i32>(s + total) as f32,
    );
    let v = sub(k(g, 0x4f), u);
    let (vu, vv, uu) = (mul(v, u), mul(v, v), mul(u, u));
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    // one coordinate: the operands of K3 (three uses), Ka, Kb, Kc
    let coord = |g: &G, o: u32, [k3a, k3b, k3c]: [u32; 3], [ka, kb, kc]: [u32; 3]| {
        let p0 = rf(g, s + 0x7b0 + o);
        let d = sub(rf(g, s + 0x7c0 + o), p0);
        let mut x = mul(mul(mul(vu, vv), k(g, k3a)), mul(d, k(g, ka)));
        x = add(x, mul(mul(vv, vv), d));
        x = add(x, mul(mul(mul(uu, vv), k(g, k3b)), mul(d, k(g, kb))));
        x = add(x, mul(mul(mul(uu, vu), k(g, k3c)), mul(d, k(g, kc))));
        x = add(x, mul(mul(uu, uu), 0.0));
        add(x, p0)
    };
    let cx = coord(g, 0, [0x63, 0x63, 0x63], [0xaa, 0xd0, 0xd8]);
    let cy = coord(g, 4, [0x152, 0x15a, 0x1a8], [0x165, 0x183, 0x18b]);
    let cz = coord(g, 8, [0x1f3, 0x1f3, 0x1f3], [0x229, 0x247, 0x24f]);
    let c = e1 + 0x14;
    g.write::<f32>(c, cx);
    g.write::<f32>(c + 4, cy);
    g.write::<f32>(c + 8, cz);
    let ww = g.read::<u32>(w(0x1c5));
    let mut place = true;
    if g.read::<u32>(ww + flag) == 1 {
        let vp = e1 + 0x24;
        g.call(t(g, 0x294, 292), 0, &[vp]);
        g.write::<u32>(vp + 0xc, 0x3f80_0000);
        let dx = sub(rf(g, vp), cx);
        let dz = sub(rf(g, vp + 8), cz);
        let dist = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt() as f32;
        let reach = if copy {
            g.read::<f32>(img.word(f + 0x2da))
        } else {
            sub(rf(g, ww + 0xb88), g.read::<f32>(img.word(f + 0x2e7)))
        };
        // `comiss`/`jae`: stays only when ordered and not below
        place = dist
            .partial_cmp(&reach)
            .is_none_or(std::cmp::Ordering::is_lt);
    }
    if place {
        let me = g.read::<u32>(w(0x2fe));
        if g.read::<u32>(w(0x2f6)) == 0 && me.cast_signed() < 0x20 {
            g.call(t(g, 0x30e, 255), 0, &[me, c]);
        } else {
            let obj = g.read::<u32>(w(0x326));
            let grounded = g.call(t(g, 0x320, 11), 0, &[obj]).eax != 0;
            if grounded {
                g.call(t(g, 0x33c, 32), 0, &[obj, cx.to_bits(), cz.to_bits()]);
            } else {
                let xyz = [obj, cx.to_bits(), cy.to_bits(), cz.to_bits()];
                g.call(t(g, 0x33c, 33), 0, &xyz);
            }
        }
    }
    if g.read::<i32>(s + count) < 0x14 {
        drop(g, img, img.rel32_target(f + at(0x39a)), e1.wrapping_sub(4));
    }
    u32::from(g.read::<i32>(s + count) <= 0)
}

/// [`curve_step_0391`] (954 bytes, 1 overlay).
pub const CURVE_STEP_0391: Sig = Sig {
    len: 954,
    hash: 0x89bb_31ae_2fdb_3245,
    calls: &[
        (0x2d0, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x39a, Callee::Sig(&DROP_0391_SIG)),
        // __security_check_cookie
        (0x3b2, Callee::Any),
    ],
    imports: &[],
};

/// [`curve_step_0448`] (938 bytes, 1 overlay).
pub const CURVE_STEP_0448: Sig = Sig {
    len: 938,
    hash: 0xb791_463e_bdc1_6faf,
    calls: &[
        (0x2cf, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x38a, Callee::Sig(&DROP_0448_SIG)),
        // __security_check_cookie
        (0x3a2, Callee::Any),
    ],
    imports: &[],
};

/// The flat distance `sqrt((v.z - c.z)^2 + (v.x - c.x)^2)` of two points
/// as the walk code computes it (`subps`, the lanes `z` then `x`, the
/// root of the double).
fn flat_dist<G: Game>(g: &G, v: u32, c: u32) -> f32 {
    use crate::sb_sse::{add, mul, sub};
    let dx = sub(g.read::<f32>(v), g.read::<f32>(c));
    let dz = sub(g.read::<f32>(v + 8), g.read::<f32>(c + 8));
    f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt() as f32
}

/// `magic_0448` 0x10005800's operands against `magic_0391` 0x10002050's.
const PLAN_0448: Shift = Shift {
    runs: &[
        (0x5f1, -7),
        (0x60e, -0xa),
        (0x68d, -0xe),
        (0x852, -0x15),
        (0x86b, -0x18),
        (0x89c, -0x1c),
    ],
    moved: &[],
};

/// `magic_0391`'s walk plan (0x10002050, 2500 bytes; internal, `ecx` = the
/// walk state `S`; an 8-aligned /GS frame, `E1` the aligned `esp` after
/// `sub esp, 0xe0` and two pushes). With an aim set (`S + 0x928` not -1)
/// it is the aim ([`walk_aim_0391`]). Else, with the walker `A = S +
/// 0x7a0` (copied to `E1 + 0x70`), the step `i = S + 0x910` of `n = S +
/// 0x914` and the battle character `b` the selection byte `S + 0x93c + i`
/// (`+0x93b + i` once `i` reached `n`):
/// - its place `P` (slot 262 of `TAB[(*G_SEL)[b]]`) and the yaw `a1 = K -
///   atan2` from `A` to it (wrapped); `Q = (0, a1, 0)`;
/// - the next character (when `i + 1 < n` and its byte differs) gives the
///   flag `S + 0x90c`: its yaw `a2` unwrapped towards `a1`
///   ([`unwrap_to`]), then `a2 < a1` when `|a2 - a1|` is below the double
///   `K64`, else `a2 >= a1`; at the first step with no next character,
///   the flag is `!slot 324 () & 1`;
/// - `C` = `P` moved by `(r, 0, 0)` turned by Y `a1` (slots 235, 328,
///   329, 229, 248) with `r = actor scale (b) + K` (negated when the flag
///   is set); the battle point `V` (slot 292).
///
/// When `S + 0x930` is 1 and `V` is beyond the reach `R = S + 0x908` from
/// `C`: `C` from `P` again by `(r, 0, 0)` turned by `Q` (the other sign);
/// still beyond, by `(0, 0, r)`; still beyond, the circle's crossing with
/// the line `A`..`C` ([`circle_hit_0391`]) moved by `(0, 0, 10)` turned
/// by `Q`; within `R` but beyond `R - K`, moved by `(0, 0, 10)` turned by
/// `Q`; any of these: `S + 0x750 = C`. Within the reach at the second
/// test the flag flips. Otherwise (no move): `S + 0x750` = `C` moved by
/// `(0, 0, -32)` turned by the yaw `a3` from `A` to `C`, then, when `S +
/// 0x930` is 1, the circle's crossing (beyond `R`) moved by `(0, 0, 10)`
/// turned by `(0, a3, 0)`, or just moved (beyond `R - K2`). `S + 0x760` =
/// `C`, `S + 0x720` = `(0, the caster's facing, 0, 0)`, `S + 0x730` =
/// `Q` (its `w` a frame word never written: 0, brief 2026-10-24). Returns
/// slot 333's `eax`.
pub fn walk_plan_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    walk_plan(g, img, f, esp, s, false)
}

/// [`walk_plan_0391`] in `magic_0448` (0x10005800, 2472 bytes): the state
/// words 4 bytes lower (`S + 0x924`, `+0x90c`, `+0x910`, `+0x937`,
/// `+0x908`, `+0x92c`), the reach and its inner edge the DLL's constants,
/// [`walk_aim_0448`] and [`circle_hit_0448`] (the operands at
/// [`PLAN_0448`]'s places).
pub fn walk_plan_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    walk_plan(g, img, f, esp, s, true)
}

#[allow(clippy::too_many_lines)] // the original's branches, in order
fn walk_plan<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32, copy: bool) -> u32 {
    use crate::ob_model::{st0_f32, wrap_once};
    use crate::sb_sse::{add, sub};
    const ONE: u32 = 0x3f80_0000;
    let at = |o: u32| if copy { PLAN_0448.at(o) } else { o };
    let w = |o: u32| img.word(f + at(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + at(o));
    // the state words that sit 4 bytes lower in the copy
    let st = |o: u32| s + if copy { o - 4 } else { o };
    let e1 = (esp.wrapping_sub(4) & !7).wrapping_sub(0xe0 + 8);
    if g.read::<u32>(st(0x928)) != u32::MAX {
        // ecx = S: the return address at E1 - 4
        let aim = if copy { walk_aim_0448 } else { walk_aim_0391 };
        return aim(g, img, call(0x28), e1.wrapping_sub(4), s);
    }
    let (o, c, p, q, u, a, m, wv, v) = (
        e1 + 0x10,
        e1 + 0x30,
        e1 + 0x40,
        e1 + 0x50,
        e1 + 0x20,
        e1 + 0x70,
        e1 + 0xa0,
        e1 + 0x60,
        e1 + 0x80,
    );
    let rf = |g: &G, x: u32| g.read::<f32>(x);
    let i = g.read::<i32>(st(0x910));
    let n = g.read::<i32>(st(0x914));
    let byte = |g: &G, o: u32, i: i32| u32::from(g.read::<u8>(st(o).wrapping_add_signed(i)));
    let b = if i < n {
        byte(g, 0x93c, i)
    } else {
        byte(g, 0x93b, i)
    };
    for k4 in 0..4 {
        let x = g.read::<u32>(s + 0x7a0 + 4 * k4);
        g.write::<u32>(a + 4 * k4, x);
    }
    let (ax, az) = (rf(g, a), rf(g, a + 8));
    // the battle character `TAB[(*G_SEL)[x]]` (the index is a byte: the
    // code's -1 case, `*G_SELF`, cannot come up)
    let chr = |g: &G, x: u32, sel: u32, tab: u32| {
        let k = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(x)));
        g.read::<u32>(w(tab).wrapping_add(4 * k))
    };
    let who = chr(g, b, 0xb1, 0xbc);
    g.call(t(g, 0xd1, 262), 0, &[who, p, p + 4, p + 8]);
    g.write::<u32>(p + 0xc, ONE);
    let yaw = |g: &mut G, img: &Image, x: u32, kk: u32, wr: [u32; 4]| {
        let at2 = |o: u32| if copy { PLAN_0448.at(o) } else { o };
        let wr = wr.map(at2);
        yaw_to(
            g,
            img,
            f,
            sub(rf(g, x + 8), az),
            sub(rf(g, x), ax),
            at2(kk),
            wr,
        )
    };
    let a1 = yaw(g, img, p, 0x12d, [0x144, 0x13d, 0x15a, 0x13d]);
    g.write::<f32>(o, a1);
    g.write::<u32>(q, 0);
    g.write::<f32>(q + 4, a1);
    g.write::<u32>(q + 8, 0);
    let flag = st(0x90c);
    let j = i.wrapping_add(1);
    let next = if j < n { Some(byte(g, 0x93c, j)) } else { None };
    match next {
        Some(c2) if c2 != b => {
            let who2 = chr(g, c2, 0x1b5, 0x1c0);
            let p2 = e1 + 0x90;
            g.call(t(g, 0x1de, 262), 0, &[who2, p2, p2 + 4, p2 + 8]);
            g.write::<u32>(p2 + 0xc, ONE);
            let a2 = yaw(g, img, p2, 0x243, [0x24b, 0x268, 0x253, 0x277]);
            let a2 = unwrap_to(a1, a2, k(g, 0x24b), k(g, 0x253));
            let diff = f64::from(sub(a2, a1)).abs();
            let below = if g.read::<f64>(w(0x2d3)) > diff {
                a2.partial_cmp(&a1).is_none_or(std::cmp::Ordering::is_lt)
            } else {
                a2 >= a1
            };
            g.write::<u32>(flag, u32::from(below));
        }
        _ => {
            if i == 0 {
                let r = g.call(t(g, 0x2fa, 324), 0, &[]).eax;
                g.write::<u32>(flag, u32::from(!(r as u8)) & 1);
            }
        }
    }
    let copy_p = |g: &mut G| {
        for k4 in 0..4 {
            let x = g.read::<u32>(p + 4 * k4);
            g.write::<u32>(c + 4 * k4, x);
        }
    };
    // the actor's scale plus `K` (ecx = b), negated as `neg` says
    let scale = |g: &mut G, img: &Image, call_op: u32, k_op: u32, neg_op: u32, neg: bool| {
        let fr = Frame {
            at: call(call_op),
            ecx: b,
            ..Frame::default()
        };
        let r = crate::s4_small::actor_scale_by(g, img, &fr);
        let r = r.xmm0.map_or(0.0, |x| f32::from_bits(x.reg() as u32));
        let r = add(r, k(g, k_op));
        if neg {
            f32::from_bits(r.to_bits() ^ g.read::<u32>(w(neg_op)))
        } else {
            r
        }
    };
    let put_u = |g: &mut G, words: [u32; 4]| {
        for (k4, x) in (0u32..).zip(words) {
            g.write::<u32>(u + 4 * k4, x);
        }
    };
    let turn = |g: &mut G, img: &Image, call_op: u32, out: u32| {
        // push U: the return address at E1 - 8
        g.write::<u32>(e1.wrapping_sub(4), u);
        let fr = Frame {
            at: call(call_op),
            esp: e1.wrapping_sub(8),
            ecx: q,
            edx: out,
            ..Frame::default()
        };
        crate::s4_small::turn_point(g, img, &fr);
    };
    let hit = |g: &mut G, img: &Image, call_op: u32, r: f32| {
        g.write::<u32>(e1.wrapping_sub(4), c);
        let fr = Frame {
            at: call(call_op),
            esp: e1.wrapping_sub(8),
            ecx: o,
            edx: a,
            xmm: [0, 0, 0, u128::from(r.to_bits())],
            ..Frame::default()
        };
        if copy {
            circle_hit_0448(g, img, &fr)
        } else {
            circle_hit_0391(g, img, &fr)
        };
    };
    let reach = |g: &G, op: u32| {
        if copy {
            g.read::<f32>(img.word(f + op))
        } else {
            rf(g, s + 0x908)
        }
    };
    let inner = |g: &G, op391: u32, op448: u32| {
        if copy {
            g.read::<f32>(img.word(f + op448))
        } else {
            sub(rf(g, s + 0x908), g.read::<f32>(img.word(f + op391)))
        }
    };
    copy_p(g);
    let set = g.read::<u32>(flag) != 0;
    let r = scale(g, img, 0x347, 0x356, 0x35f, set);
    put_u(g, [r.to_bits(), 0, 0, ONE]);
    g.call(t(g, 0x36c, 235), 0, &[m]);
    g.call(t(g, 0x3a7, 328), 0, &[m, m, 0]);
    g.call(t(g, 0x3cb, 329), 0, &[m, m, a1.to_bits()]);
    g.call(t(g, 0x3ed, 229), 0, &[wv, m, u]);
    g.call(t(g, 0x410, 248), 0, &[c, c, wv]);
    g.write::<u32>(c + 0xc, ONE);
    g.call(t(g, 0x428, 292), 0, &[v]);
    g.write::<u32>(v + 0xc, ONE);
    let mut moved = false;
    if g.read::<u32>(st(0x930)) == 1 && flat_dist(g, v, c) > reach(g, 0x48a) {
        copy_p(g);
        let set = g.read::<u32>(flag) != 0;
        let r = scale(g, img, 0x4c7, 0x4d6, 0x4df, !set);
        put_u(g, [r.to_bits(), 0, 0, ONE]);
        turn(g, img, 0x50f, c);
        if flat_dist(g, v, c) > reach(g, 0x549) {
            copy_p(g);
            let r = scale(g, img, 0x596, 0x59e, 0, false);
            put_u(g, [0, 0, r.to_bits(), ONE]);
            turn(g, img, 0x5be, c);
            let d = flat_dist(g, v, c);
            let rr = reach(g, 0x5f8);
            if d > rr {
                hit(g, img, 0x61a, rr);
                let (ox, oz) = (g.read::<u32>(o), g.read::<u32>(o + 8));
                g.write::<u32>(c, ox);
                g.write::<u32>(c + 8, oz);
                put_u(g, [0, 0, 0x4120_0000, ONE]);
                turn(g, img, 0x667, c);
            } else if d > inner(g, 0x691, 0x686) {
                put_u(g, [0, 0, 0x4120_0000, ONE]);
                turn(g, img, 0x6c8, c);
            }
            moved = true;
        } else {
            let x = u32::from(g.read::<u32>(flag) == 0);
            g.write::<u32>(flag, x);
        }
    }
    for k4 in 0..4 {
        let x = g.read::<u32>(c + 4 * k4);
        g.write::<u32>(s + 0x760 + 4 * k4, x);
    }
    if moved {
        for k4 in 0..4 {
            let x = g.read::<u32>(c + 4 * k4);
            g.write::<u32>(s + 0x750 + 4 * k4, x);
        }
    } else {
        let a3 = yaw(g, img, c, 0x762, [0x771, 0x77b, 0x785, 0x792]);
        g.write::<u32>(q, 0);
        g.write::<f32>(q + 4, a3);
        g.write::<u32>(q + 8, 0);
        put_u(g, [0, 0, 0xc200_0000, ONE]);
        turn(g, img, 0x7da, c);
        for k4 in 0..4 {
            let x = g.read::<u32>(c + 4 * k4);
            g.write::<u32>(s + 0x750 + 4 * k4, x);
        }
        if g.read::<u32>(st(0x930)) == 1 {
            let d = flat_dist(g, v, c);
            let rr = reach(g, 0x84b);
            let go = if d > rr {
                hit(g, img, 0x877, rr);
                let (ox, oz) = (g.read::<u32>(o), g.read::<u32>(o + 8));
                g.write::<u32>(s + 0x750, ox);
                g.write::<u32>(s + 0x758, oz);
                true
            } else {
                d > inner(g, 0x8a0, 0x887)
            };
            if go {
                put_u(g, [0, 0, 0x4120_0000, ONE]);
                turn(g, img, 0x8d9, s + 0x750);
            }
        }
    }
    g.write::<u32>(s + 0x720, 0);
    let caster = g.read::<u32>(w(0x913));
    let out = g.call(t(g, 0x903, 333), 0, &[caster]);
    let rot = sub(k(g, 0x927), st0_f32(out.st0));
    let rot = wrap_once(g, img, f, rot, [0x937, 0x941, 0x94b, 0x958].map(at));
    g.write::<f32>(s + 0x724, rot);
    g.write::<u32>(s + 0x728, 0);
    g.write::<u32>(s + 0x72c, 0);
    for k4 in 0..3 {
        let x = g.read::<u32>(q + 4 * k4);
        g.write::<u32>(s + 0x730 + 4 * k4, x);
    }
    // junk read: [esp+0x5c] (E1 + 0x5c, Q's `w`) never written; 0 (brief 2026-10-24)
    g.write::<u32>(s + 0x73c, 0);
    out.eax
}

/// [`walk_plan_0391`] (2500 bytes, 1 overlay).
pub const WALK_PLAN_0391: Sig = Sig {
    len: 2500,
    hash: 0x6fcd_5a8d_9fe5_e5ed,
    calls: &[
        (0x28, Callee::Sig(&WALK_AIM_0391)),
        (0x11b, Callee::Import("_CIatan2")),
        (0x231, Callee::Import("_CIatan2")),
        (0x347, Callee::Sig(&crate::s4_small::ACTOR_SCALE_BY)),
        (0x47f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x4c7, Callee::Sig(&crate::s4_small::ACTOR_SCALE_BY)),
        (0x50f, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x53e, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x596, Callee::Sig(&crate::s4_small::ACTOR_SCALE_BY)),
        (0x5be, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x5ed, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x61a, Callee::Sig(&CIRCLE_HIT_0391)),
        (0x667, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x6c8, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x750, Callee::Import("_CIatan2")),
        (0x7da, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x84e, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x877, Callee::Sig(&CIRCLE_HIT_0391)),
        (0x8d9, Callee::Sig(&crate::s4_small::TURN_POINT)),
        // __security_check_cookie
        (0x38, Callee::Any),
        (0x9bc, Callee::Any),
    ],
    imports: &[],
};

/// [`walk_plan_0448`] (2472 bytes, 1 overlay).
pub const WALK_PLAN_0448: Sig = Sig {
    len: 2472,
    hash: 0x7f90_1719_7338_9ea9,
    calls: &[
        (0x28, Callee::Sig(&WALK_AIM_0448)),
        (0x11b, Callee::Import("_CIatan2")),
        (0x231, Callee::Import("_CIatan2")),
        (0x347, Callee::Sig(&crate::s4_small::ACTOR_SCALE_BY)),
        (0x47f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x4c7, Callee::Sig(&crate::s4_small::ACTOR_SCALE_BY)),
        (0x50f, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x53e, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x596, Callee::Sig(&crate::s4_small::ACTOR_SCALE_BY)),
        (0x5be, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x5ed, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x610, Callee::Sig(&CIRCLE_HIT_0448)),
        (0x65d, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x6ba, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x742, Callee::Import("_CIatan2")),
        (0x7cc, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x840, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x85f, Callee::Sig(&CIRCLE_HIT_0448)),
        (0x8bd, Callee::Sig(&crate::s4_small::TURN_POINT)),
        // __security_check_cookie
        (0x38, Callee::Any),
        (0x9a0, Callee::Any),
    ],
    imports: &[],
};

/// The motion block `magic_0391`'s / `magic_0448`'s turn starts fill
/// (operands from `f + base`): `G_ANIM` = `T1` when the target yaw `c`
/// (`S + 0x734`, unwrapped towards the facing `a = S + 0x724`) is below
/// `a` (or unordered), else `T0` (`tables`: the two immediates); the
/// caster `*G_ME` kept, -1 words, slot 215 `(me)` (outside a battle) or
/// 413 `(me)` (in one) into the block, then -1, 0, 1 and the battle word
/// (u16 0 or 1). `ops`: hi, lo, hi again, then the block's operand
/// offsets in order (`G_BATTLE`, `G_ANIM`, `G_ME`, its copy, the slot
/// table, the -1 word, `G_BATTLE` again, the answer, -1, 0, 1, the u16
/// for 0 and for 1).
fn turn_block<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, tables: [u32; 2], ops: &[u32; 16]) {
    use crate::sb_sse::{add, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let a = g.read::<f32>(s + 0x724);
    let mut c = g.read::<f32>(s + 0x734);
    if 0.0 > a {
        if c > 0.0 && sub(c, a) > k(g, ops[0]) {
            c = sub(k(g, ops[1]), sub(k(g, ops[0]), c));
        }
    } else if 0.0 > c && sub(a, c) > k(g, ops[2]) {
        let hi = k(g, ops[2]);
        c = add(add(c, hi), hi);
    }
    let below = c.partial_cmp(&a).is_none_or(std::cmp::Ordering::is_lt);
    let [
        b1,
        anim,
        me_op,
        me_copy,
        slots,
        m1,
        b2,
        ans,
        m1b,
        z,
        one,
        h0,
        h1,
    ] = [
        ops[3], ops[4], ops[5], ops[6], ops[7], ops[8], ops[9], ops[10], ops[11], ops[12], ops[13],
        ops[14], ops[15],
    ];
    g.write::<u32>(w(m1), u32::MAX);
    g.write::<u32>(w(anim), if below { tables[1] } else { tables[0] });
    let me = g.read::<u32>(w(me_op));
    g.write::<u32>(w(me_copy), me);
    let slot = if g.read::<u32>(w(b1)) == 0 { 215 } else { 413 };
    let r = g
        .call(crate::runtime::slot_target(g, w(slots), slot), 0, &[me])
        .eax;
    g.write::<u32>(w(ans), r);
    g.write::<u32>(w(m1b), u32::MAX);
    g.write::<u32>(w(z), 0);
    g.write::<u32>(w(one), 1);
    if g.read::<u32>(w(b2)) == 0 {
        g.write::<u16>(w(h0), 0);
    } else {
        g.write::<u16>(w(h1), 1);
    }
}

/// The walker's start point as the turn starts end: the caster's place
/// (slot 262 `(*G_A, ..)`, operands `s262`/`caster`) into `S + 0x7b0`
/// (`w` 1.0), `S + 0x7c0` = `S + 0x7a0`, the steps `S + 0x91c` and `+0x918`
/// (`magic_0448`: `+0x918`, `+0x914`; `low`) = `steps`. Returns `S +
/// 0x7ac` (the last word moved).
fn walker_start<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    s: u32,
    [s262, caster]: [u32; 2],
    steps: u32,
    low: bool,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let p = s + 0x7b0;
    let chr = g.read::<u32>(w(caster));
    g.call(
        crate::runtime::slot_target(g, w(s262), 262),
        0,
        &[chr, p, p + 4, p + 8],
    );
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let mut last = 0;
    for k4 in 0..4 {
        last = g.read::<u32>(s + 0x7a0 + 4 * k4);
        g.write::<u32>(s + 0x7c0 + 4 * k4, last);
    }
    let d = if low { 4 } else { 0 };
    g.write::<u32>(s + 0x91c - d, steps);
    g.write::<u32>(s + 0x918 - d, steps);
    last
}

/// `magic_0391`'s turn start (0x100034c0, 361 bytes; internal, `ecx` = the
/// walk state `S`): the walk plan ([`walk_plan_0391`]), the motion block
/// ([`turn_block`], tables `0x1000a6f4` / `0x1000a230`), the walker's
/// start ([`walker_start`], 26 steps). Returns `S + 0x7ac`.
/// `magic_0448` 0x10006f90 is the same code ([`turn_start_0448`]).
pub fn turn_start_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    turn_start(g, img, f, esp, s, false)
}

/// [`turn_start_0391`] in `magic_0448` (0x10006f90).
pub fn turn_start_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    turn_start(g, img, f, esp, s, true)
}

fn turn_start<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32, copy: bool) -> u32 {
    let plan = if copy { walk_plan_0448 } else { walk_plan_0391 };
    // push esi, edi: the return address at E - 0xc
    plan(g, img, img.rel32_target(f + 0x5), esp.wrapping_sub(0xc), s);
    let tables = [img.word(f + 0x71), img.word(f + 0x76)];
    turn_block(
        g,
        img,
        f,
        s,
        tables,
        &[
            0x2a, 0x42, 0x55, 0x7f, 0x85, 0x8a, 0x8f, 0x95, 0x9b, 0xba, 0xc5, 0xcb, 0xd5, 0xdf,
            0xe9, 0xf2,
        ],
    );
    walker_start(g, img, f, s, [0x105, 0x10c], 0x1a, copy)
}

/// [`turn_start_0391`] (361 bytes).
pub const TURN_START_0391: Sig = Sig {
    len: 361,
    hash: 0x4dd5_bf48_5d90_0021,
    calls: &[(0x5, Callee::Sig(&WALK_PLAN_0391))],
    imports: &[],
};

/// [`turn_start_0448`] (361 bytes).
pub const TURN_START_0448: Sig = Sig {
    len: 361,
    hash: 0xda7c_bd22_30ed_6119,
    calls: &[(0x5, Callee::Sig(&WALK_PLAN_0448))],
    imports: &[],
};

/// `magic_0391`'s walk start with sparks (0x100031d0, 742 bytes; internal,
/// `ecx` = the walk state `S`; `ebp = E - 4`): the old flag `S + 0x90c`
/// kept, the walk plan ([`walk_plan_0391`]), the motion block
/// ([`turn_block`]: the tables by the old flag, `0x1000a218`/`0x1000a200`
/// when it was 0, else `0x1000a074`/`0x1000a05c`); then for the step
/// `i = S + 0x910 - 1` the object `O = S + 0x130 i` started
/// ([`hosted_0391`](crate::sl_host::hosted_0391), `edx` = `O`, `xmm3` =
/// `(rnd 0.2 + K1) * K2`, slot 314), the battle character
/// `TAB[(*G_SEL)[S + 0x93c + i]]` placed at `O + 0xd0` (slot 262, `w`
/// 1.0) and told 0x10 (slot 268 `(chr, 0x10, O + 0xd0)`), `O + 0xc4` =
/// the facing, `O + 0x120` = its selection byte, the record
/// `R = S + 0x7d0 + 0x34 i` set up (`+0x10..0x2c`, `+0x20` P and `+0x24`
/// the DLL's table, the u16 `+0x30` 0) and kept at `O + 0x110`; slot 306
/// `(7, 0, h)` with `h` = slot 719 `(P, 9, 1)`; the walker's start ([`walker_start`], 36 steps). Returns
/// `S + 0x7ac`. `magic_0448` 0x10006c80 is the same code
/// ([`spark_start_0448`]).
pub fn spark_start_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    spark_start(g, img, f, esp, s, false)
}

/// [`spark_start_0391`] in `magic_0448` (0x10006c80).
pub fn spark_start_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    spark_start(g, img, f, esp, s, true)
}

#[allow(clippy::too_many_lines)] // the original's steps, in order
fn spark_start<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32, copy: bool) -> u32 {
    use crate::sb_sse::{add, mul};
    let plan = if copy { walk_plan_0448 } else { walk_plan_0391 };
    let hosted = if copy {
        crate::sl_host::hosted_0448
    } else {
        crate::sl_host::hosted_0391
    };
    // the state words 4 bytes lower in `magic_0448`
    let st = |o: u32| s + if copy { o - 4 } else { o };
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let old = g.read::<u32>(st(0x90c));
    // push ebp; sub esp, 8; push esi, edi: the return address at E - 0x18
    plan(
        g,
        img,
        img.rel32_target(f + 0x11),
        esp.wrapping_sub(0x18),
        s,
    );
    let tables = if old == 0 {
        [img.word(f + 0x7e), img.word(f + 0x83)]
    } else {
        [img.word(f + 0x8a), img.word(f + 0x8f)]
    };
    turn_block(
        g,
        img,
        f,
        s,
        tables,
        &[
            0x36, 0x4e, 0x61, 0xa5, 0xab, 0xb0, 0xb5, 0xbb, 0x98, 0xd6, 0xe1, 0xe7, 0xf1, 0xfb,
            0x105, 0x10e,
        ],
    );
    let out = g.call(t(g, 0x113, 314), 0, &[0x3e4c_cccd]);
    let r = crate::ob_model::st0_f32(out.st0);
    let i = g.read::<u32>(st(0x910)).wrapping_sub(1);
    let scale = mul(add(r, g.read::<f32>(w(0x13a))), g.read::<f32>(w(0x14b)));
    let o = s.wrapping_add(i.wrapping_mul(0x130));
    // push ebx; push (0.2); sub esp, 0xc: the return address at E - 0x2c
    let at = esp.wrapping_sub(0x2c);
    g.write::<u32>(at + 0x10, 0x3e4c_cccd);
    let mut xmm = [0u128; 4];
    xmm[3] = u128::from(scale.to_bits());
    let fr = Frame {
        at: img.rel32_target(f + 0x15a),
        esp: at,
        edx: o,
        xmm,
        ..Frame::default()
    };
    hosted(g, img, &fr);
    let p2 = o.wrapping_add(0xd0);
    let byte = |g: &G| u32::from(g.read::<u8>(st(0x93c).wrapping_add(i)));
    let sel = |g: &G, sel_op: u32, x: u32| {
        u32::from(g.read::<u8>(g.read::<u32>(w(sel_op)).wrapping_add(x)))
    };
    // (a byte: the code's -1 cases, `*G_SELF` and `*G_ME`, cannot come up)
    let c = byte(g);
    let chr = g.read::<u32>(w(0x18b).wrapping_add(4 * sel(g, 0x180, c)));
    g.call(t(g, 0x198, 262), 0, &[chr, p2, p2 + 4, p2 + 8]);
    // call width: 7 (slot 262's words stay under it)
    g.call(
        t(g, 0x1aa, 268),
        0,
        &[chr, 0x10, p2, chr, p2, p2 + 4, p2 + 8],
    );
    g.write::<u32>(p2 + 0xc, 0x3f80_0000);
    let face = g.read::<u32>(s + 0x724);
    g.write::<u32>(o + 0xc4, face);
    let d = byte(g);
    let x = sel(g, 0x1eb, d);
    g.write::<u32>(o + 0x120, x);
    let rec = s.wrapping_add(0x7d0).wrapping_add(i.wrapping_mul(0x34));
    for (off, v) in [
        (0x18u32, 0),
        (0x14, 0),
        (0x10, 0),
        (0x1c, 0),
        (0x20, img.word(f + 0x225)),
        (0x24, img.word(f + 0x22c)),
        (0x28, 0),
        (0x2c, 0),
    ] {
        g.write::<u32>(rec + off, v);
    }
    g.write::<u16>(rec + 0x30, 0);
    g.write::<u32>(o + 0x110, rec);
    let h = g.call(t(g, 0x249, 719), 0, &[w(0x258), 9, 1]).eax;
    // call width: 6 (slot 719's words stay under it)
    g.call(t(g, 0x260, 306), 0, &[7, 0, h, w(0x258), 9, 1]);
    walker_start(g, img, f, s, [0x27f, 0x286], 0x24, copy)
}

/// [`spark_start_0391`] (742 bytes).
pub const SPARK_START_0391: Sig = Sig {
    len: 742,
    hash: 0x13dc_2f0c_b489_9484,
    calls: &[
        (0x11, Callee::Sig(&WALK_PLAN_0391)),
        (0x15a, Callee::Sig(&crate::sl_host::HOSTED_0391)),
    ],
    imports: &[],
};

/// [`spark_start_0448`] (742 bytes).
pub const SPARK_START_0448: Sig = Sig {
    len: 742,
    hash: 0xb24f_4743_f94e_8b04,
    calls: &[
        (0x11, Callee::Sig(&WALK_PLAN_0448)),
        (0x15a, Callee::Sig(&crate::sl_host::HOSTED_0448)),
    ],
    imports: &[],
};

/// The motion block of the walk resets (as in [`turn_block`], the motion
/// table `anim` given): operands `[G_BATTLE, G_ME, its copy, G_ANIM,
/// -1 word, slot table, G_BATTLE again, the answer, -1, 0, 1, u16 for 0,
/// u16 for 1]`.
fn motion_block<G: Game>(g: &mut G, img: &Image, f: u32, anim: u32, ops: &[u32; 13]) {
    let w = |o: u32| img.word(f + o);
    let me = g.read::<u32>(w(ops[1]));
    g.write::<u32>(w(ops[2]), me);
    g.write::<u32>(w(ops[3]), anim);
    g.write::<u32>(w(ops[4]), u32::MAX);
    let slot = if g.read::<u32>(w(ops[0])) == 0 {
        215
    } else {
        413
    };
    let r = g
        .call(crate::runtime::slot_target(g, w(ops[5]), slot), 0, &[me])
        .eax;
    g.write::<u32>(w(ops[7]), r);
    g.write::<u32>(w(ops[8]), u32::MAX);
    g.write::<u32>(w(ops[9]), 0);
    g.write::<u32>(w(ops[10]), 1);
    if g.read::<u32>(w(ops[6])) == 0 {
        g.write::<u16>(w(ops[11]), 0);
    } else {
        g.write::<u16>(w(ops[12]), 1);
    }
}

/// The walk's first-step set-up shared by the resets (`i = S + i_at` 0):
/// the walker at the caster (slot 262 into `S + 0x7a0`, `w` 1.0), the walk
/// plan, `S + 0x740` = `S + 0x730`, `S + count_at` = 6, `S + on_at` = 1,
/// the motion block with the table the immediate `+anim_imm`.
#[allow(clippy::too_many_arguments)]
fn reset_head<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    s: u32,
    plan: (u32, u32),
    [s262, caster, anim_imm]: [u32; 3],
    [count_at, on_at]: [u32; 2],
    ops: &[u32; 13],
    walk: fn(&mut G, &Image, u32, u32, u32) -> u32,
) {
    let w = |o: u32| img.word(f + o);
    let p = s + 0x7a0;
    let chr = g.read::<u32>(w(caster));
    g.call(
        crate::runtime::slot_target(g, w(s262), 262),
        0,
        &[chr, p, p + 4, p + 8],
    );
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    walk(g, img, img.rel32_target(f + plan.0), plan.1, s);
    for k4 in 0..4 {
        let x = g.read::<u32>(s + 0x730 + 4 * k4);
        g.write::<u32>(s + 0x740 + 4 * k4, x);
    }
    g.write::<u32>(s + count_at, 6);
    g.write::<u32>(s + on_at, 1);
    motion_block(g, img, f, img.word(f + anim_imm), ops);
}

/// The battle character's place `D` (slot 262 `(*G_A, ..)` at `d`, `w`
/// 1.0) and the step count from the flat (`dy` too when `full`) distance
/// to `S + 0x750`: `trunc(dist / K)` at most 300, plus 1, into `S +
/// steps` and `S + steps - 4`; then `S + 0x770` = `S + 0x760`, `S + 0x7b0`
/// = `(D, 1)`, `S + 0x7c0` = `S + 0x770`, `S + 0x7a0` = `S + 0x750`.
/// Returns `S + 0x75c` (the last word moved).
#[allow(clippy::too_many_arguments)]
fn reset_tail<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    s: u32,
    d: u32,
    [s262, caster, k]: [u32; 3],
    steps: u32,
    full: bool,
) -> u32 {
    use crate::sb_sse::{add, div, mul, sub};
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let chr = g.read::<u32>(w(caster));
    g.call(
        crate::runtime::slot_target(g, w(s262), 262),
        0,
        &[chr, d, d + 4, d + 8],
    );
    let dx = sub(rf(g, d), rf(g, s + 0x750));
    let dz = sub(rf(g, d + 8), rf(g, s + 0x758));
    let sq = if full {
        let dy = sub(rf(g, d + 4), rf(g, s + 0x754));
        add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz))
    } else {
        g.write::<u32>(d + 0xc, 0x3f80_0000);
        add(mul(dz, dz), mul(dx, dx))
    };
    let dist = f64::from(sq).sqrt() as f32;
    // `cvttss2si` (the integer indefinite 0x80000000 for NaN or out of range)
    let q = div(dist, rf(g, w(k)));
    let n = if q.is_nan() || q >= 2_147_483_648.0 || q < -2_147_483_648.0 {
        i32::MIN
    } else {
        q as i32
    };
    let n = n.min(300) + 1;
    g.write::<i32>(s + steps, n);
    g.write::<i32>(s + steps - 4, n);
    for k4 in 0..4 {
        let x = g.read::<u32>(s + 0x760 + 4 * k4);
        g.write::<u32>(s + 0x770 + 4 * k4, x);
    }
    for k4 in 0..3 {
        let x = g.read::<u32>(d + 4 * k4);
        g.write::<u32>(s + 0x7b0 + 4 * k4, x);
    }
    g.write::<u32>(s + 0x7bc, 0x3f80_0000);
    for k4 in 0..4 {
        let x = g.read::<u32>(s + 0x770 + 4 * k4);
        g.write::<u32>(s + 0x7c0 + 4 * k4, x);
    }
    let mut last = 0;
    for k4 in 0..4 {
        last = g.read::<u32>(s + 0x750 + 4 * k4);
        g.write::<u32>(s + 0x7a0 + 4 * k4, last);
    }
    last
}

/// `magic_0391`'s walk reset (0x10003630, 864 bytes; internal, `ecx` = the
/// walk state `S`; an 8-aligned /GS frame, `E1` the aligned `esp` after
/// `sub esp, 0x80` and two pushes). At the first step (`S + 0x910` 0): the
/// walker at the caster, the plan ([`walk_plan_0391`]), `S + 0x740` = `S +
/// 0x730`, `S + 0x920` = 6, `S + 0x934` = 1, the motion block. Always: the
/// caster's place `D` (`E1 + 0x10`) and the step count from its flat
/// distance to `S + 0x750` (into `S + 0x91c`, `+0x918`), the points moved
/// on ([`reset_tail`]); `S + 0x780` = `(0, 0, 0)` plus the point `(0, 0,
/// -3.5)` turned by Y `S + 0x734` (slots 235, 328, 329 into `E1 + 0x40`,
/// 229 into `E1 + 0x30`, 248), `w` 1.0. Returns slot 248's `eax`.
pub fn walk_reset_0391<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let e1 = (esp.wrapping_sub(4) & !7).wrapping_sub(0x80 + 8);
    if g.read::<u32>(s + 0x910) == 0 {
        reset_head(
            g,
            img,
            f,
            s,
            (0x57, e1.wrapping_sub(4)),
            [0x37, 0x3e, 0xbc],
            [0x920, 0x934],
            &[
                0xa1, 0xa7, 0xac, 0xb8, 0xc2, 0xb2, 0xe1, 0xec, 0xf2, 0xfc, 0x106, 0x110, 0x119,
            ],
            walk_plan_0391,
        );
    }
    let d = e1 + 0x10;
    reset_tail(g, img, f, s, d, [0x12d, 0x133, 0x184], 0x91c, false);
    let yaw = g.read::<u32>(s + 0x734);
    g.write::<u32>(e1 + 0xc, yaw);
    let o = s + 0x780;
    let (m, u, wv) = (e1 + 0x40, e1 + 0x20, e1 + 0x30);
    for k4 in 0..4 {
        g.write::<u32>(o + 4 * k4, 0);
    }
    for (k4, x) in (0u32..).zip([0, 0, 0xc060_0000, 0x3f80_0000]) {
        g.write::<u32>(u + 4 * k4, x);
    }
    // call width: 5 (slot 262's words stay under it)
    g.call(
        t(g, 0x276, 235),
        0,
        &[m, g.read::<u32>(w(0x133)), d, d + 4, d + 8],
    );
    g.call(t(g, 0x2dd, 328), 0, &[m, m, 0]);
    g.call(t(g, 0x2fe, 329), 0, &[m, m, yaw]);
    g.call(t(g, 0x31a, 229), 0, &[wv, m, u]);
    let eax = g.call(t(g, 0x334, 248), 0, &[o, o, wv]).eax;
    g.write::<u32>(o + 0xc, 0x3f80_0000);
    eax
}

/// [`walk_reset_0391`] (864 bytes, 1 overlay).
pub const WALK_RESET_0391: Sig = Sig {
    len: 864,
    hash: 0x687d_8c1a_401a_408e,
    calls: &[
        (0x57, Callee::Sig(&WALK_PLAN_0391)),
        (0x173, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x358, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0448`'s walk reset (0x100061b0, 641 bytes; internal, `ecx` = the
/// walk state `S`; an 8-aligned /GS frame, `E1` the aligned `esp` after
/// `sub esp, 0x18` and two pushes): [`walk_reset_0391`]'s first-step
/// set-up on 0448's words (`S + 0x90c` 0; [`walk_plan_0448`]; `S + 0x91c`
/// = 6, `S + 0x930` = 1), the step count from the full distance (`y`
/// too; into `S + 0x918`, `+0x914`) and the points moved on, no turned
/// point. Returns `S + 0x75c`.
pub fn walk_reset_0448<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, s: u32) -> u32 {
    let e1 = (esp.wrapping_sub(4) & !7).wrapping_sub(0x18 + 8);
    if g.read::<u32>(s + 0x90c) == 0 {
        reset_head(
            g,
            img,
            f,
            s,
            (0x54, e1.wrapping_sub(4)),
            [0x34, 0x3b, 0xb9],
            [0x91c, 0x930],
            &[
                0x9e, 0xa4, 0xa9, 0xb5, 0xbf, 0xaf, 0xde, 0xe9, 0xef, 0xf9, 0x103, 0x10d, 0x116,
            ],
            walk_plan_0448,
        );
    }
    reset_tail(g, img, f, s, e1 + 0xc, [0x12a, 0x130, 0x192], 0x918, true)
}

/// [`walk_reset_0448`] (641 bytes, 1 overlay).
pub const WALK_RESET_0448: Sig = Sig {
    len: 641,
    hash: 0x10f3_c99b_e3c0_4c91,
    calls: &[
        (0x54, Callee::Sig(&WALK_PLAN_0448)),
        (0x181, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x279, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0448` 0x10007100's operands against `magic_0391` 0x10003990's.
const TASK_0448: Shift = Shift {
    runs: &[(0x91, 0x16), (0x163, 0x19), (0x1ff, 0x2f)],
    moved: &[],
};

/// `magic_0391`'s walk task (0x10003990, 892 bytes; cdecl `(node)`, a task
/// callback; an 8-aligned /GS frame, `E1` the aligned `esp` after `sub
/// esp, 0x28` and two pushes). The walk state `S = *G_S + 0x280` runs the
/// state `S + 0x924` (a jump table; above 4 none):
/// - 0: the walk reset ([`walk_reset_0391`]), the step up, state 1;
/// - 1: a walk step ([`walk_step_0391`]); when the caster (slot 262) is
///   within `K` of `S + 0x770` (flat) or the step says done, the spark
///   start ([`spark_start_0391`]) and state 2;
/// - 2: a curve step ([`curve_step_0391`]): done, the next state 0 while
///   steps are left, else slot 213 when idle
///   ([`call_213_idle`](crate::s4_small::call_213_idle), `edx` 0) and the
///   node's byte `+0x56` = 1; not done, at the step count 23 the handle
///   call ([`handle_393`](crate::s4_small::handle_393), `edx` 0, the
///   caster's place, 0x100), `S + 0x740` = `S + 0x730`, `S + 0x920` = 10;
/// - 3: the walk reset, state 4;
/// - 4: a walk step; near `S + 0x750` or done, the turn start
///   ([`turn_start_0391`]) and state 2.
///
/// Then while the count `S + 0x920` is above 0 it steps down and the
/// caster turns (slot 34 `(*G_A, K - v)`, wrapped) by `v = (1 - t) c + t
/// a` (`t = count / K2`, `a` the facing `S + 0x724`, `c` = `S + 0x744`
/// unwrapped towards it; wrapped). Returns slot 34's `eax`, or the count.
pub fn walk_task_0391<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    walk_task(g, img, fr, false)
}

/// [`walk_task_0391`] in `magic_0448` (0x10007100, 936 bytes): the state
/// words 4 bytes lower, the distances in all three axes, `edx` 2 for the
/// handle call, the 0448 callees; the operands at [`TASK_0448`]'s places.
pub fn walk_task_0448<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    walk_task(g, img, fr, true)
}

#[allow(clippy::too_many_lines)] // the five states, in order
fn walk_task<G: Game>(g: &mut G, img: &Image, fr: &Frame, copy: bool) -> Ret {
    use crate::ob_model::wrap_once;
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let at = |o: u32| if copy { TASK_0448.at(o) } else { o };
    let w = |o: u32| img.word(f + at(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| crate::runtime::slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + at(o));
    let s = g.read::<u32>(w(0x1f)).wrapping_add(0x280);
    let st = |o: u32| s + if copy { o - 4 } else { o };
    let e1 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28 + 8);
    // the internal callees' return address, under nothing pushed
    let esp = e1.wrapping_sub(4);
    let node = fr.arg(g, 0);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    // the caster's distance to `S + to` (`full`: in all three axes)
    let near = |g: &mut G, [s262, caster]: [u32; 2], to: u32, k_op: u32| {
        let p = e1 + 0xc;
        let chr = g.read::<u32>(w(caster));
        g.call(t(g, s262, 262), 0, &[chr, p, p + 4, p + 8]);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        let dz = sub(rf(g, p + 8), rf(g, s + to + 8));
        let dx = sub(rf(g, p), rf(g, s + to));
        let sq = if copy {
            let dy = sub(rf(g, p + 4), rf(g, s + to + 4));
            add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz))
        } else {
            add(mul(dz, dz), mul(dx, dx))
        };
        let d = f64::from(sq).sqrt() as f32;
        k(g, k_op) > d
    };
    let walk_step = if copy { walk_step_0448 } else { walk_step_0391 };
    let walk_reset = if copy {
        walk_reset_0448
    } else {
        walk_reset_0391
    };
    let mut eased = false;
    match g.read::<u32>(st(0x924)) {
        0 => {
            walk_reset(g, img, call(0x46), esp, s);
            let i = g.read::<u32>(st(0x910));
            g.write::<u32>(st(0x910), i.wrapping_add(1));
            g.write::<u32>(st(0x924), 1);
            eased = true;
        }
        1 => {
            let done = walk_step(g, img, call(0x62), esp, s);
            if near(g, [0x68, 0x7f], 0x770, 0xd1) || done == 1 {
                let spark = if copy {
                    spark_start_0448
                } else {
                    spark_start_0391
                };
                spark(g, img, call(0xe6), esp, s);
                g.write::<u32>(st(0x924), 2);
            }
            eased = true;
        }
        2 => {
            let curve = if copy {
                curve_step_0448
            } else {
                curve_step_0391
            };
            if curve(g, img, call(0xf2), esp, s) == 1 {
                if g.read::<i32>(st(0x910)) < g.read::<i32>(st(0x914)) {
                    g.write::<u32>(st(0x924), 0);
                } else {
                    let idle = Frame {
                        at: call(0x11b),
                        esp,
                        eax: 1,
                        edx: 0,
                        ..Frame::default()
                    };
                    crate::s4_small::call_213_idle(g, img, &idle);
                    g.write::<u8>(node + 0x56, 1);
                }
            } else if g.read::<u32>(st(0x918)) == 0x17 {
                let v = e1 + 0x1c;
                let chr = g.read::<u32>(w(0x14b));
                g.call(t(g, 0x145, 262), 0, &[chr, v, v + 4, v + 8]);
                g.write::<u32>(v + 0xc, 0x3f80_0000);
                // push V, 0x100 over slot 262's words: the return address at E1 - 0x1c
                let hs = e1.wrapping_sub(0x1c);
                for (i, x) in (1u32..).zip([v, 0x100, chr, v, v + 4, v + 8]) {
                    g.write::<u32>(hs + 4 * i, x);
                }
                let h = Frame {
                    at: call(0x171),
                    esp: hs,
                    ecx: w(0x164),
                    edx: if copy { 2 } else { 0 },
                    ..Frame::default()
                };
                crate::s4_small::handle_393(g, img, &h);
                for k4 in 0..4 {
                    let x = g.read::<u32>(s + 0x730 + 4 * k4);
                    g.write::<u32>(s + 0x740 + 4 * k4, x);
                }
                g.write::<u32>(st(0x920), 10);
            }
            eased = true;
        }
        3 => {
            walk_reset(g, img, call(0x1ba), esp, s);
            g.write::<u32>(st(0x924), 4);
            eased = true;
        }
        4 => {
            let done = walk_step(g, img, call(0x1d0), esp, s);
            if near(g, [0x1d6, 0x1ed], 0x750, 0x23f) || done == 1 {
                let turn = if copy {
                    turn_start_0448
                } else {
                    turn_start_0391
                };
                turn(g, img, call(0x250), esp, s);
                g.write::<u32>(st(0x924), 2);
            }
            eased = true;
        }
        _ => {}
    }
    // the easing's `1` (the prologue's operand, or the one reloaded after
    // a state)
    let one = k(g, if eased { 0x262 } else { 0x18 });
    let n = g.read::<i32>(st(0x920));
    if n <= 0 {
        return Ret::eax(n.cast_unsigned());
    }
    let n = n - 1;
    g.write::<i32>(st(0x920), n);
    let a = rf(g, s + 0x724);
    let (hi, lo) = (k(g, 0x280), k(g, 0x278));
    let c = unwrap_to(a, rf(g, s + 0x744), hi, lo);
    #[allow(clippy::cast_precision_loss)] // `cvtdq2ps`
    let tt = div(n as f32, k(g, 0x2e8));
    let v = add(mul(sub(one, tt), c), mul(tt, a));
    let v = wrap_once(g, img, f, v, [0x280, 0x2fc, 0x278, 0x2fc].map(at));
    let x = sub(k(g, 0x31c), v);
    let x = wrap_once(g, img, f, x, [0x280, 0x2fc, 0x278, 0x2fc].map(at));
    let caster = g.read::<u32>(w(0x34b));
    Ret::eax(g.call(t(g, 0x339, 34), 0, &[caster, x.to_bits()]).eax)
}

/// [`walk_task_0391`] (892 bytes, 1 overlay).
pub const WALK_TASK_0391: Sig = Sig {
    len: 892,
    hash: 0x37ca_e72a_3b41_41c6,
    calls: &[
        (0x46, Callee::Sig(&WALK_RESET_0391)),
        (0x62, Callee::Sig(&WALK_STEP_0391)),
        (0xc2, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xe6, Callee::Sig(&SPARK_START_0391)),
        (0xf2, Callee::Sig(&CURVE_STEP_0391)),
        (0x11b, Callee::Sig(&crate::s4_small::CALL_213_IDLE)),
        (0x171, Callee::Sig(&crate::s4_small::HANDLE_393)),
        (0x1ba, Callee::Sig(&WALK_RESET_0391)),
        (0x1d0, Callee::Sig(&WALK_STEP_0391)),
        (0x230, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x250, Callee::Sig(&TURN_START_0391)),
        // __security_check_cookie
        (0x35d, Callee::Any),
    ],
    imports: &[],
};

/// [`walk_task_0448`] (936 bytes, 1 overlay).
pub const WALK_TASK_0448: Sig = Sig {
    len: 936,
    hash: 0xa2fa_0949_88cd_3be9,
    calls: &[
        (0x46, Callee::Sig(&WALK_RESET_0448)),
        (0x62, Callee::Sig(&WALK_STEP_0448)),
        (0xd8, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xfc, Callee::Sig(&SPARK_START_0448)),
        (0x108, Callee::Sig(&CURVE_STEP_0448)),
        (0x131, Callee::Sig(&crate::s4_small::CALL_213_IDLE)),
        (0x18a, Callee::Sig(&crate::s4_small::HANDLE_393)),
        (0x1d3, Callee::Sig(&WALK_RESET_0448)),
        (0x1e9, Callee::Sig(&WALK_STEP_0448)),
        (0x25f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x27f, Callee::Sig(&TURN_START_0448)),
        // __security_check_cookie
        (0x38c, Callee::Any),
    ],
    imports: &[],
};
