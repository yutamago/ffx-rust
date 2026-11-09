//! Round 24's ports (magic-r24): the shared rows and the own rows of
//! overlays below `magic_0450`.

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The light set-up of `magic_0446` (0x100053b0, 907 bytes) and
/// `magic_0543` (0x10005310, 975 bytes): internal, `ecx` = the angles
/// `A`, `edx` = the position `P`, one stack word `n`; a 16-aligned /GS
/// frame, the locals from `E` = `(ebp & !0xf) - 0x208`. The kin of
/// [`light_setup`](crate::o6_fam_pairs5::light_setup) that hands the
/// light to slot 567 as a record instead of the table rows:
/// - the record `R` = `E + 8`: `+0` 0x1c, `+4` slot 562 `()`, the u16
///   `+8` 0x8044, `+0xa` 0, `+0xc..+0x14` 0, the byte `+0x1b` = `n`, the
///   u16 `+0x18` 0x8080, `+0x1a` 0x80, `+0x1c` 0, `+0x34..+0x48` 0, the
///   u16 `+0x4c` 0x80;
/// - `h` = slot 219 `(*G_H, 0)`; the matrix `M0` (`E + 0x80`) from slot
///   235; `a` = slot 277 `(h)` (times `K1` in `magic_0446`) on the
///   diagonal, `M0[0]`, `M0[5]`, `M0[10]` times slots 15, 16, 17 `(h)`
///   (the slots' ST0 as f32, each kept at `E`);
/// - `M0` turned by slots 330, 328, 329 `(M0, M0, A.z / A.x / A.y)`, its
///   last row `P`;
/// - `magic_0543` only: slot 843 `(slot 825 (h), M0)`;
/// - slot 723 `(M1)`, 724 `(M2)`, 231 `(M3, M2, M0)` (and in `magic_0543`
///   231 `(M3, M1, M3)`): `magic_0446` has `M1, M2, M3` = `E + 0x140`,
///   `+0xc0`, `+0x100`, `magic_0543` `E + 0x100`, `+0x140`, `+0xc0`;
/// - the scaled camera ([`scaled_camera`](crate::m0052::scaled_camera),
///   `ecx` = `E + 0x1c0`); slot 230 `(E + 0x180, slot 545 ())`;
/// - `R + 0x20..` = `M3`, `E + 0x1c0`, `E + 0x180`, `M0`;
/// - slots 357 `(h, -1, 0x3c00)`, 356 `(h, -1, 0x2ee0)`, 355 `(h, -1)`,
///   354 `(h, -1)`, 567 `(*G_2, R, M0, M2, M1)`, 357 and 356 `(h, -1,
///   -1)`.
///
/// Returns the last slot 356's `eax`. The copies are told apart by the
/// byte at `+0xe5`: `magic_0543`'s `mov eax, [G]` (0xa1) where
/// `magic_0446` multiplies by `K1`; its later operands sit 8 bytes
/// earlier until its extra calls.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn light_setup_0446<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::mul;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let v0543 = img.word(f + 0xe5) as u8 == 0xa1;
    let (angles, pos, n) = (fr.ecx, fr.edx, fr.arg(g, 0));
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !0xf).wrapping_sub(0x208);
    let at = |o: u32| e.wrapping_add(o);
    let rec = at(8);
    let m0 = at(0x80);
    g.write::<u32>(rec, 0x1c);
    // call width: 0
    let h0 = g.call(slot(g, 562), angles, &[]).eax;
    g.write::<u32>(at(0xc), h0);
    for o in [0x14u32, 0x18, 0x1c] {
        g.write::<u32>(at(o), 0);
    }
    g.write::<u8>(at(0x23), n as u8);
    for o in [0x48u32, 0x4c, 0x50] {
        g.write::<u32>(at(o), 0);
    }
    g.write::<u16>(at(0x10), 0x8044);
    g.write::<u8>(at(0x12), 0);
    g.write::<u16>(at(0x20), 0x8080);
    g.write::<u8>(at(0x22), 0x80);
    for o in [0x24u32, 0x3c, 0x40, 0x44] {
        g.write::<u32>(at(o), 0);
    }
    g.write::<u16>(at(0x54), 0x80);
    let gh = g.read::<u32>(w(0x53));
    // call width: 2
    let h = g.call(slot(g, 219), angles, &[gh, 0]).eax;
    g.write::<u32>(at(4), h);
    // call width: 1
    g.call(slot(g, 235), angles, &[m0]);
    // each float slot's ST0 is kept (`fstp`) at `E`
    let float = |g: &mut G, s: u32| {
        // call width: 1
        let x = st0_f32(g.call(slot(g, s), angles, &[h]).st0);
        g.write::<f32>(e, x);
        x
    };
    let a = float(g, 277);
    let a = if v0543 {
        a
    } else {
        mul(a, g.read::<f32>(w(0xe9)))
    };
    let diag = [m0, at(0x94), at(0xa8)];
    for &d in diag.iter().rev() {
        g.write::<f32>(d, a);
    }
    for (s, d) in [15u32, 16, 17].into_iter().zip(diag) {
        let t = float(g, s);
        let x = mul(g.read::<f32>(d), t);
        g.write::<f32>(d, x);
    }
    for (i, s) in [(2u32, 330u32), (0, 328), (1, 329)] {
        let x = g.read::<u32>(angles.wrapping_add(4 * i));
        // call width: 3
        g.call(slot(g, s), angles, &[m0, m0, x]);
    }
    for i in 0..4u32 {
        let v = g.read::<u32>(pos.wrapping_add(4 * i));
        g.write::<u32>(at(0xb0 + 4 * i), v);
    }
    // `esi` = `[E + 4]`: read here in `magic_0543`, after slot 230 in
    // `magic_0446`
    let mut h = h;
    let (m1, m2, m3, cam_at, g2) = if v0543 {
        h = g.read::<u32>(at(4));
        // call width: 1
        let r = g.call(slot(g, 825), angles, &[h]).eax;
        // call width: 2
        g.call(slot(g, 843), angles, &[r, m0]);
        (at(0x100), at(0x140), at(0xc0), (0x2c1, 0x44), 0x385)
    } else {
        (at(0x140), at(0xc0), at(0x100), (0x27c, 0x2c), 0x344)
    };
    // call width: 1
    g.call(slot(g, 723), angles, &[m1]);
    // call width: 1
    g.call(slot(g, 724), angles, &[m2]);
    // call width: 3
    g.call(slot(g, 231), angles, &[m3, m2, m0]);
    if v0543 {
        // call width: 3
        g.call(slot(g, 231), angles, &[m3, m1, m3]);
    }
    let cam = at(0x1c0);
    // the call's return address below the words pushed so far
    crate::m0052::scaled_camera(
        g,
        img,
        img.rel32_target(f + cam_at.0),
        cam,
        e.wrapping_sub(cam_at.1),
    );
    // call width: 0
    let r = g.call(slot(g, 545), angles, &[]).eax;
    // call width: 2
    g.call(slot(g, 230), angles, &[at(0x180), r]);
    if !v0543 {
        h = g.read::<u32>(at(4));
    }
    for (o, v) in [(0x28u32, m3), (0x2c, cam), (0x30, at(0x180)), (0x34, m0)] {
        g.write::<u32>(at(o), v);
    }
    // call width: 3
    g.call(slot(g, 357), angles, &[h, u32::MAX, 0x3c00]);
    // call width: 3
    g.call(slot(g, 356), angles, &[h, u32::MAX, 0x2ee0]);
    // call width: 2
    g.call(slot(g, 355), angles, &[h, u32::MAX]);
    // call width: 2
    g.call(slot(g, 354), angles, &[h, u32::MAX]);
    let x = g.read::<u32>(w(g2));
    // call width: 5
    g.call(slot(g, 567), angles, &[x, rec, m0, m2, m1]);
    // call width: 3
    g.call(slot(g, 357), angles, &[h, u32::MAX, u32::MAX]);
    // call width: 3
    let eax = g.call(slot(g, 356), angles, &[h, u32::MAX, u32::MAX]).eax;
    Ret::eax(eax)
}

/// [`light_setup_0446`] in `magic_0446` (the `rel32`s: the scaled camera
/// and `__security_check_cookie`).
pub const LIGHT_SETUP_0446: Sig = Sig {
    len: 907,
    hash: 0x5b86_b1a9_2bdf_840d,
    calls: &[
        (0x27c, Callee::Sig(&crate::m0052::SCALED_CAMERA)),
        (0x383, Callee::Any),
    ],
    imports: &[],
};

/// [`light_setup_0446`] in `magic_0543`.
pub const LIGHT_SETUP_0543: Sig = Sig {
    len: 975,
    hash: 0xf3a3_bd1a_b49b_2d0d,
    calls: &[
        (0x2c1, Callee::Sig(&crate::m0052::SCALED_CAMERA)),
        (0x3c7, Callee::Any),
    ],
    imports: &[],
};

/// The light set-up a light task calls at `fr.at`: `magic_0446`'s and
/// `magic_0543`'s ([`light_setup_0446`]: `and esp, -0x10`) or the
/// 0644/0565 one ([`light_setup`](crate::o6_fam_pairs5::light_setup):
/// `and esp, -8`), told apart by the alignment byte at `+5`.
pub fn light_setup_any<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    if img.word(fr.at + 5) as u8 == 0xf0 {
        light_setup_0446(g, img, fr)
    } else {
        crate::o6_fam_pairs5::light_setup(g, img, fr)
    }
}

/// [`light_fly_in`](crate::s9_misc::light_fly_in) in `magic_0446` and
/// `magic_0543` (0x10002a90: 0565's code, calling [`light_setup_0446`]
/// and the child start without the sound,
/// [`start_child_b`](crate::s4_fam446::start_child_b)).
pub const LIGHT_FLY_IN_0446: Sig = Sig {
    len: 395,
    hash: 0x3a22_0f26_8ba7_acfe,
    calls: &[
        (0xde, Callee::Sig(&LIGHT_SETUP_0446)),
        (0x12e, Callee::Sig(&crate::s4_fam446::START_CHILD_B)),
        (0x13d, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x176, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// [`LIGHT_FLY_IN_0446`] in `magic_0543`.
pub const LIGHT_FLY_IN_0543: Sig = Sig {
    len: 395,
    hash: 0x3a22_0f26_8ba7_acfe,
    calls: &[
        (0xde, Callee::Sig(&LIGHT_SETUP_0543)),
        (0x12e, Callee::Sig(&crate::s4_fam446::START_CHILD_B)),
        (0x13d, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x176, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// [`light_bounce`](crate::s9_misc::light_bounce) in `magic_0446`
/// (0x10003180: 0565's code, calling [`light_setup_0446`]).
pub const LIGHT_BOUNCE_0446: Sig = Sig {
    len: 795,
    hash: 0x10ed_6256_6b19_3781,
    calls: &[
        (0x218, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2f1, Callee::Sig(&LIGHT_SETUP_0446)),
        (0x310, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`LIGHT_BOUNCE_0446`] in `magic_0543`.
pub const LIGHT_BOUNCE_0543: Sig = Sig {
    len: 795,
    hash: 0x10ed_6256_6b19_3781,
    calls: &[
        (0x218, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2f1, Callee::Sig(&LIGHT_SETUP_0543)),
        (0x310, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`bone_node_setup`](crate::o6_fam_bone::bone_node_setup) in
/// `magic_0272` (0x100067a0, 434 bytes: `arg` and `src` from globals,
/// the operands shifted; the `rel32`s: the node insert, the bone update
/// and `__security_check_cookie`).
pub const BONE_NODE_SETUP_0272: Sig = Sig {
    len: 434,
    hash: 0xa66d_e296_d7a6_d508,
    calls: &[
        (0xec, Callee::Sig(&crate::nodes::INSERT)),
        (0xff, Callee::Sig(&crate::sa_summon::BONE)),
        (0x1aa, Callee::Any),
    ],
    imports: &[],
};

/// [`caster_place_0588`](crate::r18::caster_place_0588) in `magic_0434`
/// (0x10005860, 804 bytes: `magic_0610`'s copy with slot 946 first; the
/// `rel32`s: the others shown and `__security_check_cookie`).
pub const CASTER_PLACE_0434: Sig = Sig {
    len: 804,
    hash: 0xe8b6_f3fe_5d49_4303,
    calls: &[
        (0x2d3, Callee::Sig(&crate::sb_vm::SHOW_OTHERS)),
        (0x31c, Callee::Any),
    ],
    imports: &[],
};

/// Where [`curve_step_0435`]'s two copies keep their operands (offsets
/// in the function).
struct CurveStep {
    /// The count's store (its load is at `+0x11` in both).
    count: u32,
    /// The done flag set before the give-back, and the give-back's `rel32`.
    done: (u32, u32),
    /// `magic_0055`: slot 239 `(*G_X, -5, 0)` first (the slot table's and
    /// `G_X`'s operands).
    s239: Option<(u32, u32)>,
    /// The fade's `rel32`.
    fade: u32,
    /// The time `T`, its step `K`, the store of `T`.
    time: (u32, u32, u32),
    /// The `x` keys and their curve's `rel32`, the `y` global, the `z`
    /// keys and their curve's `rel32`.
    x: (u32, u32),
    y: u32,
    z: (u32, u32),
    /// `magic_0055`: the yaw only while `*G_F` is 0.
    flag: Option<u32>,
    /// The yaw keys and their curve's `rel32`; `K1`, `H`, `T`, `L`, `T`
    /// again (the wrap).
    yaw: (u32, u32),
    wrap: [u32; 5],
    /// The actor byte's pointer, the table it indexes (`magic_0435`:
    /// slot 34 gets `TAB[b]`; `magic_0055`: slot 170 gets `b`), the slot
    /// table's operand, the slot.
    actor: (u32, Option<u32>, u32, u32),
    /// The place's `rel32`.
    place: u32,
}

/// `magic_0435` 0x10001a60.
const STEP_0435: CurveStep = CurveStep {
    count: 0x1a,
    done: (0x22, 0x2b),
    s239: None,
    fade: 0x46,
    time: (0x4e, 0x56, 0x67),
    x: (0x5c, 0x6c),
    y: 0x79,
    z: (0x83, 0x8b),
    flag: None,
    yaw: (0x95, 0xa4),
    wrap: [0xac, 0xba, 0xc4, 0xce, 0xdb],
    actor: (0xe0, Some(0xf0), 0xf5, 34),
    place: 0x108,
};

/// `magic_0055` 0x10001ec0.
const STEP_0055: CurveStep = CurveStep {
    count: 0x1b,
    done: (0x3f, 0x48),
    s239: Some((0x22, 0x32)),
    fade: 0x64,
    time: (0x6c, 0x74, 0x85),
    x: (0x7a, 0x8a),
    y: 0x97,
    z: (0xa1, 0xa9),
    flag: Some(0xb2),
    yaw: (0xc7, 0xcf),
    wrap: [0xd7, 0xe5, 0xef, 0xf9, 0x106],
    actor: (0x10b, None, 0x11a, 170),
    place: 0x12d,
};

/// `magic_0067` 0x10004520: `magic_0055`'s without the flag.
const STEP_0067: CurveStep = CurveStep {
    count: 0x1b,
    done: (0x3f, 0x48),
    s239: Some((0x22, 0x32)),
    fade: 0x64,
    time: (0x6c, 0x74, 0x85),
    x: (0x7a, 0x8a),
    y: 0x97,
    z: (0xa1, 0xa9),
    flag: None,
    yaw: (0xb3, 0xc2),
    wrap: [0xca, 0xd8, 0xe2, 0xec, 0xf9],
    actor: (0xfe, None, 0x10d, 170),
    place: 0x120,
};

/// A summon's step along key curves (cdecl `(obj)`, a /GS frame, the
/// point `P` at `ebp - 0x14`): the count `*G_C` goes down by one. At 0
/// (in `magic_0055` slot 239 `(*G_X, -5, 0)` first) `*G_D` = 1 and `obj`
/// goes back to its pool ([`give_back`](crate::sa_pool::give_back)). Else
/// at 10 the fade starts ([`fade_541_or_38_by`](crate::s4_small::fade_541_or_38_by),
/// rate 0); the time `T += K`; `P` = (the `x` curve at `T`
/// ([`key_curve`](crate::s4_small::key_curve)), `*G_Y`, the `z` curve,
/// 1.0); the yaw `K1 - ` the angle curve
/// ([`angle_keys`](crate::s5_misc::angle_keys)) brought back once by a
/// turn when above `H` or below `L` (`comiss` + `jbe`) goes to slot 34
/// `(TAB[*actor], yaw)` (`magic_0055`: only while `*G_F` is 0, slot 170
/// `(*actor, yaw)`); the actor placed at `P`
/// ([`place`](crate::m0052::place)). Returns the give-back's or the
/// place's `eax`.
///
/// `magic_0435` 0x10001a60 (282 bytes), `magic_0055` 0x10001ec0 (320
/// bytes) and `magic_0067` 0x10004520 (307 bytes, `magic_0055`'s without
/// the flag), told apart by `push esi` (0x56) at `+0x16` in the latter two
/// and `magic_0055`'s `cmp [G_F], 0` (0x83) at `+0xb0`.
pub fn curve_step_0435<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let o = if img.word(f + 0x16) as u8 != 0x56 {
        &STEP_0435
    } else if img.word(f + 0xb0) as u8 == 0x83 {
        &STEP_0055
    } else {
        &STEP_0067
    };
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let c = g.read::<u32>(w(0x11)).wrapping_sub(1);
    g.write::<u32>(w(o.count), c);
    if c == 0 {
        if let Some((tab, x)) = o.s239 {
            let x = g.read::<u32>(w(x));
            // call width: 3
            g.call(
                slot_target(g, w(tab), 239),
                obj,
                &[x, (-5i32).cast_unsigned(), 0],
            );
        }
        g.write::<u32>(w(o.done.0), 1);
        let back = img.rel32_target(f + o.done.1);
        return Ret::eax(crate::sa_pool::give_back(g, img, back, obj));
    }
    // the callees' frame: `push ecx` below the saved registers
    let esp = ebp.wrapping_sub(0x18 + u32::from(o.s239.is_some()) * 4);
    let sub_frame = |at: u32, ecx: u32, xmm: [u128; 4]| Frame {
        at,
        esp: esp.wrapping_sub(4),
        eax: 0,
        ecx,
        edx: 0,
        xmm,
    };
    if c == 10 {
        let fade = sub_frame(img.rel32_target(f + o.fade), obj, [0; 4]);
        crate::s4_small::fade_541_or_38_by(g, img, &fade);
    }
    let t = add(k(g, o.time.0), k(g, o.time.1));
    g.write::<f32>(w(o.time.2), t);
    let curve = |g: &mut G, (keys, at): (u32, u32)| {
        let fr = sub_frame(
            img.rel32_target(f + at),
            w(keys),
            [crate::Xmm::f32(t).reg(), 0, 0, 0],
        );
        let r = crate::s4_small::key_curve(g, img, &fr);
        r.xmm0.map_or(0.0, |x| f32::from_bits(x.reg() as u32))
    };
    let p = ebp.wrapping_sub(0x14);
    let x = curve(g, o.x);
    g.write::<f32>(p, x);
    let y = g.read::<u32>(w(o.y));
    g.write::<u32>(p + 4, y);
    let z = curve(g, o.z);
    g.write::<f32>(p + 8, z);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let shown = o.flag.is_none_or(|fl| g.read::<u32>(w(fl)) == 0);
    if shown {
        let at = img.rel32_target(f + o.yaw.1);
        let (a, _) = crate::s5_misc::angle_keys(g, img, at, w(o.yaw.0), t);
        let [k1, hi, turn, lo, turn2] = o.wrap;
        let mut yaw = sub(k(g, k1), a);
        if yaw > k(g, hi) {
            yaw = sub(yaw, k(g, turn));
        } else if k(g, lo) > yaw {
            yaw = add(yaw, k(g, turn2));
        }
        let (pa, tab, slots, n) = o.actor;
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(pa))));
        let who = match tab {
            Some(tab) => g.read::<u32>(w(tab).wrapping_add(4 * b)),
            None => b,
        };
        // call width: 2
        g.call(slot_target(g, w(slots), n), obj, &[who, yaw.to_bits()]);
    }
    let place = img.rel32_target(f + o.place);
    Ret::eax(crate::m0052::place(g, img, place, p))
}

/// [`curve_step_0435`] in `magic_0435` (the `rel32`s: the give-back, the
/// cookie check (twice), the fade, the curves, the place).
pub const CURVE_STEP_0435: Sig = Sig {
    len: 282,
    hash: 0xef8a_9ed9_4071_49d6,
    calls: &[
        (0x2b, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x35, Callee::Any),
        (0x46, Callee::Sig(&crate::s4_small::FADE_541_OR_38_BY)),
        (0x6c, Callee::Sig(&crate::s4_small::KEY_CURVE_0435)),
        (0x8b, Callee::Sig(&crate::s4_small::KEY_CURVE_0435)),
        (0xa4, Callee::Sig(&crate::s5_misc::ANGLE_KEYS_0435)),
        (0x108, Callee::Sig(&crate::m0052::PLACE)),
        (0x112, Callee::Any),
    ],
    imports: &[],
};

/// [`curve_step_0435`] in `magic_0055`.
pub const CURVE_STEP_0055: Sig = Sig {
    len: 320,
    hash: 0x72e1_86fc_35ad_3bdb,
    calls: &[
        (0x48, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x53, Callee::Any),
        (0x64, Callee::Sig(&crate::s4_small::FADE_541_OR_38_BY)),
        (0x8a, Callee::Sig(&crate::s4_small::KEY_CURVE)),
        (0xa9, Callee::Sig(&crate::s4_small::KEY_CURVE)),
        (0xcf, Callee::Sig(&crate::s5_misc::ANGLE_KEYS)),
        (0x12d, Callee::Sig(&crate::m0052::PLACE)),
        (0x138, Callee::Any),
    ],
    imports: &[],
};

/// [`curve_step_0435`] in `magic_0067`.
pub const CURVE_STEP_0067: Sig = Sig {
    len: 307,
    hash: 0x7b43_6082_2bc6_58b4,
    calls: &[
        (0x48, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x53, Callee::Any),
        (0x64, Callee::Sig(&crate::s4_small::FADE_541_OR_38_BY)),
        (0x8a, Callee::Sig(&crate::s4_small::KEY_CURVE)),
        (0xa9, Callee::Sig(&crate::s4_small::KEY_CURVE)),
        (0xc2, Callee::Sig(&crate::s5_misc::ANGLE_KEYS)),
        (0x120, Callee::Sig(&crate::m0052::PLACE)),
        (0x12b, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_task`](crate::o6_fam_pairs6::trail_task) in `magic_0446`
/// (0x10002280, 633 bytes): `magic_0544`'s code with `push 0x80` and
/// [`light_setup_0446`] for the part's matrix. (`magic_0543`'s copy at
/// 0x10002250 waits on its own trail draw, 0x10004d20.)
pub const TRAIL_TASK_0446: Sig = Sig {
    len: 633,
    hash: 0xb4fb_fde7_2d07_d968,
    calls: &[
        (0xf9, Callee::Sig(&crate::s4_fam446::SPARKS_D)),
        (0x103, Callee::Sig(&crate::s4_fam446::SPARKS_C)),
        (0x10d, Callee::Sig(&crate::s4_fam446::SPARKS_A)),
        (0x120, Callee::Sig(&LIGHT_SETUP_0446)),
        (0x174, Callee::Sig(&crate::s4_fam446::START_CHILD_A)),
        (0x18c, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x1c5, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x1d2, Callee::Any),
        (
            0x261,
            Callee::Sig(&crate::o6_fam_pairs6::TRAIL_DRAW_0446_4E20),
        ),
        (0x271, Callee::Any),
    ],
    imports: &[],
};

/// [`launch_after_carry`](crate::s9_misc::launch_after_carry) in
/// `magic_0446` (0x100034a0, 1575 bytes): `magic_0544`'s code with `push
/// 0x80` and [`light_setup_0446`] for the part's matrix.
pub const LAUNCH_AFTER_CARRY_0446: Sig = Sig {
    len: 1575,
    hash: 0x1373_1b7e_3be3_3a2a,
    calls: &[
        (0xdf, Callee::Sig(&LIGHT_SETUP_0446)),
        (0xfc, Callee::Sig(&crate::s4_fam446::START_CHILD_C)),
        (0x2f8, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x30b, Callee::Import("_CIatan2")),
        (0x33a, Callee::Import("_CIatan2")),
        (0x4d6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x5ce, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x61f, Callee::Any),
    ],
    imports: &[],
};

/// [`LAUNCH_AFTER_CARRY_0446`] in `magic_0543` (0x100034a0).
pub const LAUNCH_AFTER_CARRY_0543: Sig = Sig {
    len: 1575,
    hash: 0x1373_1b7e_3be3_3a2a,
    calls: &[
        (0xdf, Callee::Sig(&LIGHT_SETUP_0543)),
        (0xfc, Callee::Sig(&crate::s4_fam446::START_CHILD_C)),
        (0x2f8, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x30b, Callee::Import("_CIatan2")),
        (0x33a, Callee::Import("_CIatan2")),
        (0x4d6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x5ce, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x61f, Callee::Any),
    ],
    imports: &[],
};

/// [`weapon_swing_drop`](crate::s5_misc::weapon_swing_drop) in
/// `magic_0446` (0x10004270, 516 bytes): `magic_0544`'s code with `push
/// 0x80` and [`light_setup_0446`] for the part's matrix.
pub const WEAPON_SWING_DROP_0446: Sig = Sig {
    len: 516,
    hash: 0xe9cb_52e0_5023_fbe6,
    calls: &[
        (0xdb, Callee::Sig(&LIGHT_SETUP_0446)),
        (0x152, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1b5, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x1ef, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x1fc, Callee::Any),
    ],
    imports: &[],
};

/// [`task_0603`](crate::r22::task_0603) in `magic_0150` (0x1000b5a0, 383
/// bytes: the step block 0x14c higher, read from the code).
pub const TASK_0150: Sig = Sig {
    len: 383,
    hash: 0xc217_7f4e_6d74_b79f,
    calls: &[
        (0xda, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xe5, Callee::Sig(&crate::s7_misc::HANDLES_REGISTERED)),
        (0x114, Callee::Any),
        (0x177, Callee::Any),
    ],
    imports: &[],
};

/// [`WEAPON_SWING_DROP_0446`] in `magic_0543` (0x10004240).
pub const WEAPON_SWING_DROP_0543: Sig = Sig {
    len: 516,
    hash: 0xe9cb_52e0_5023_fbe6,
    calls: &[
        (0xdb, Callee::Sig(&LIGHT_SETUP_0543)),
        (0x152, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1b5, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x1ef, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x1fc, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0352`'s render step (0x10005020, 1001 bytes; cdecl `(obj)`, an
/// 8-aligned /GS frame, the locals from `E` = `(ebp & !7) - 0x11c`; `B =
/// *G_B`, the slots `L` = the lock word `*G_L` taken (1) around 773, 779
/// and 870): while `B[0xbdc]` or else `B[0xbe0]` is positive it counts
/// down (else `obj` goes back to its pool,
/// [`give_back`](crate::sa_pool::give_back), and that is all); `r1` =
/// slot 772 `(0x160, 0)`, `r2` = slot 773 `(r1, obj)` (kept at `E + 4`;
/// `E + 8` = `B`, `E + 0xc` = `obj`, `E + 0x10` = `r1`):
/// - `r2` 0: `E + 4..` = `0, 4, 6`, the 10 bytes at `E + 0x14` the code's
///   (a qword and a halfword); slot 881 `(r1, obj, slot 776 (0x48))`, 882
///   `(r1, obj, 1, E + 8, E + 0xc, E + 0x14, E + 4)`; when slot 883 `(r1,
///   obj)` answers non-zero that is returned, else slot 779 `(r1, obj)`;
/// - else unless slot 886 `(r2, 0, 4, 6)` answers 0 (returned): `p` =
///   slot 887 `(r2, 0)`, the index words of slot 902 `(p)` 0x10000,
///   0x10002, 0x20003, the 12 corners of slot 898 `(p)` (+-1.0), the
///   four rows of slot 900 `(p)` (1, 1, 1, `a`) with `a = (B[0xbe0] *
///   255 / 60) * K` (integer division); slot 230 `(slot 893 (r2), *G_M)`
///   with its last column (0, 0, 0, 1); slot 890's and 891's `(r2)`
///   vectors -1e7 and +1e7 (`0xcb18967f`, `0x4b18967f`); slots 892 `(r2,
///   0)`, 884 `(r1, obj, 1)` and 870 `(r1, obj)`.
///
/// Returns `*G_L` (the lock's address) after the last slot.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn render_0352<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x6b), n);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x11c);
    let at = |o: u32| e.wrapping_add(o);
    let b = g.read::<u32>(w(0x1c));
    g.write::<u32>(at(8), b);
    let n = g.read::<i32>(b + 0xbdc);
    if n > 0 {
        g.write::<i32>(b + 0xbdc, n - 1);
    } else {
        let m = g.read::<i32>(b + 0xbe0);
        if m <= 0 {
            let back = img.rel32_target(f + 0x41);
            return Ret::eax(crate::sa_pool::give_back(g, img, back, obj));
        }
        g.write::<i32>(b + 0xbe0, m - 1);
    }
    // call width: 2
    let r1 = g.call(slot(g, 772), 0, &[0x160, 0]).eax;
    let lock_at = w(0x80);
    let locked = |g: &mut G, n: u32, args: &[u32]| {
        let lock = g.read::<u32>(lock_at);
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        // call width: 4 (the words pushed so far)
        let r = g.call(slot(g, n), 0, args).eax;
        let lock = g.read::<u32>(lock_at);
        g.write::<u32>(lock, saved);
        r
    };
    g.write::<u32>(at(0x10), r1);
    g.write::<u32>(at(0xc), obj);
    let r2 = locked(g, 773, &[r1, obj, 0x160, 0]);
    g.write::<u32>(at(4), r2);
    if r2 == 0 {
        // call width: 1
        let r3 = g.call(slot(g, 776), 0, &[0x48]).eax;
        g.write::<u32>(at(8), 4);
        g.write::<u32>(at(0xc), 6);
        g.write::<u32>(at(4), 0);
        let q = g.read::<u64>(w(0xd2));
        g.write::<u64>(at(0x14), q);
        let h = g.read::<u16>(w(0xd9));
        g.write::<u16>(at(0x1c), h);
        // call width: 3
        g.call(slot(g, 881), 0, &[r1, obj, r3]);
        let words = [r1, obj, 1, at(8), at(0xc), at(0x14), at(4)];
        // call width: 7
        g.call(slot(g, 882), 0, &words);
        // call width: 2
        let r = g.call(slot(g, 883), 0, &[r1, obj]).eax;
        if r != 0 {
            return Ret::eax(r);
        }
        locked(g, 779, &[r1, obj]);
        return Ret::eax(g.read::<u32>(w(0x3cb)));
    }
    // call width: 4
    if g.call(slot(g, 886), 0, &[r2, 0, 4, 6]).eax == 0 {
        return Ret::eax(0);
    }
    let r2 = g.read::<u32>(at(4));
    // call width: 2
    let p = g.call(slot(g, 887), 0, &[r2, 0]).eax;
    // call width: 1
    let corners = g.call(slot(g, 898), 0, &[p]).eax;
    // call width: 1
    let rows = g.call(slot(g, 900), 0, &[p]).eax;
    // call width: 1
    let index = g.call(slot(g, 902), 0, &[p]).eax;
    let blk = g.read::<u32>(at(8));
    let steps = g.read::<i32>(blk + 0xbe0).wrapping_mul(0xff) / 60;
    for (i, v) in (0u32..).zip([0x10000u32, 0x10002, 0x20003]) {
        g.write::<u32>(index + 4 * i, v);
    }
    let r2 = g.read::<u32>(at(4));
    let (one, minus) = (0x3f80_0000u32, 0xbf80_0000u32);
    let signs = [
        minus, minus, one, one, minus, one, minus, one, one, one, one, one,
    ];
    for (i, v) in (0u32..).zip(signs) {
        g.write::<u32>(corners + 4 * i, v);
    }
    let a = mul(cvt(steps.cast_unsigned()), g.read::<f32>(w(0x28b)));
    for r in 0..4u32 {
        for c in 0..3u32 {
            g.write::<u32>(rows + 0x10 * r + 4 * c, one);
        }
        g.write::<f32>(rows + 0x10 * r + 0xc, a);
    }
    // call width: 1
    let mat = g.call(slot(g, 893), 0, &[r2]).eax;
    // call width: 1
    let low = g.call(slot(g, 890), 0, &[r2]).eax;
    // call width: 1
    let high = g.call(slot(g, 891), 0, &[r2]).eax;
    let m = g.read::<u32>(w(0x332));
    // call width: 2
    g.call(slot(g, 230), 0, &[mat, m]);
    for (o, v) in [(0xcu32, 0), (0x1c, 0), (0x2c, 0), (0x3c, one)] {
        g.write::<u32>(mat + o, v);
    }
    for i in 0..3u32 {
        g.write::<u32>(low + 4 * i, 0xcb18_967f);
    }
    let r2 = g.read::<u32>(at(4));
    for i in 0..3u32 {
        g.write::<u32>(high + 4 * i, 0x4b18_967f);
    }
    // call width: 2
    g.call(slot(g, 892), 0, &[r2, 0]);
    let (obj, r1) = (g.read::<u32>(at(0xc)), g.read::<u32>(at(0x10)));
    // call width: 3
    g.call(slot(g, 884), 0, &[r1, obj, 1]);
    locked(g, 870, &[r1, obj]);
    Ret::eax(g.read::<u32>(w(0x3cb)))
}

/// [`render_0352`] (the `rel32`s: the give-back and
/// `__security_check_cookie`, twice).
pub const RENDER_0352: Sig = Sig {
    len: 1001,
    hash: 0xbcac_a9ce_de20_ef04,
    calls: &[
        (0x41, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x52, Callee::Any),
        (0x3e1, Callee::Any),
    ],
    imports: &[],
};

/// [`spawn_pair`](crate::m0588::spawn_pair) in `magic_0430` (0x100031c0,
/// 236 bytes: animated objects, the effect block's offsets 0xc0 higher).
pub const SPAWN_PAIR_0430: Sig = Sig {
    len: 236,
    hash: 0xee52_e48e_b329_be01,
    calls: &[
        (0x26, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0x46, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x8a, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0xa6, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};

/// `magic_0311`'s start-up (0x100013e0, 1208 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0311`; an 8-aligned /GS
/// frame, the locals from `E` =
/// `(ebp & !7) - 0x94`, `A = *G_A` the actor):
/// - `*G_C` = slot 719 `(D, 8, 0)` (two globals set to their immediates
///   first); the part set up ([`part_init`](crate::l5_objhost::part_init));
///   the object at the immediate started
///   ([`start_object_reset_arg`](crate::r9_misc::start_object_reset_arg),
///   scale `K1`);
/// - the point `P` (three globals) from slot 262 `(A, &P)`, `P.y` = slot
///   12 `(A)`, `P.w` 1.0; the yaw `K2 - slot 333 (A)` wrapped once by `T`
///   (above `H`, below `L`) into a global, a u16 cleared; `P.y -= K6`;
///   nine globals of a record set;
/// - `Q` (`E + 0x1c`) = slot 262 `(A, ..)`, `R` (`E + 0xc`) = slot 262
///   `(TAB[*I], ..)`, `R.y = Q.y`; the turn `K2 - atan2(R.z - Q.z, R.x -
///   Q.x)` (`_CIatan2` on x87) wrapped the same way;
/// - the matrix `M` (`E + 0x4c`) from slot 235, turned by slots 328 `(M,
///   M, 0)` and 329 `(M, M, turn)`; slot 229 `(O, M, V)` with `V = (0, 0,
///   10, 1)` (`O` at `E + 0x3c`), slot 248 `(R, R, O)`; `P.x`, `P.z` =
///   `R.x`, `R.z`; the record started at `R`
///   ([`global_node_start`](crate::o6_fam_pairs5::global_node_start),
///   scales `K7`, 1.2, 0.99);
/// - the actor block (`*G_J`, slot 413 or 215 by `*G_K`, the flags) and
///   the colour words; [`static_node`](crate::o4_fam3::static_node); two
///   fades ([`fade_task`](crate::s3_misc::fade_task)); slot 271 `(4)`;
///   a pool node for the DLL's task ([`take`](crate::sa_pool::take))
///   appended to the task list.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0311<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x94);
    let at = |o: u32| e.wrapping_add(o);
    let below = |o: u32| e.wrapping_sub(o);
    let one = 0x3f80_0000u32;
    g.write::<u32>(w(0x31), w(0x35));
    g.write::<u32>(w(0x3b), w(0x3f));
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x2b), 8, 0]).eax;
    g.write::<u32>(w(0x49), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x4e));
    let start = Frame {
        at: img.rel32_target(f + 0x63),
        esp: below(0x18),
        eax: 0,
        ecx: 0,
        edx: w(0x5e),
        xmm: [0, 0, 0, crate::Xmm::f32(k(g, 0x56)).reg()],
    };
    crate::r9_misc::start_object_reset_arg(g, img, &start);
    // the point `P` (three globals, `P.w` the fourth)
    let a = g.read::<u32>(w(0x6e));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, w(0x83), w(0x7e), w(0x79)]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[a]).st0);
    g.write::<f32>(w(0x97), y);
    let a2 = g.read::<u32>(w(0xa2));
    g.write::<u32>(w(0xa8), one);
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a2]).st0);
    g.write::<f32>(at(4), t);
    let wrap = |g: &G, x: f32, [hi, turn, lo, turn2]: [u32; 4]| {
        if x > k(g, hi) {
            sub(x, k(g, turn))
        } else if k(g, lo) > x {
            add(x, k(g, turn2))
        } else {
            x
        }
    };
    let yaw = wrap(g, sub(k(g, 0xc0), t), [0xd8, 0xce, 0xe8, 0xce]);
    g.write::<u16>(w(0xf9), 0);
    g.write::<f32>(w(0x101), yaw);
    let py = sub(g.read::<f32>(w(0x109)), k(g, 0x111));
    let a3 = g.read::<u32>(w(0x12b));
    g.write::<f32>(w(0x133), py);
    for o in [0x139u32, 0x143, 0x14d, 0x157] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x161), w(0x165));
    g.write::<u32>(w(0x16b), w(0x16f));
    for o in [0x175u32, 0x17f] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x189), w(0x18d));
    let (q, r) = (at(0x1c), at(0xc));
    // call width: 4
    g.call(slot(g, 262), 0, &[a3, q, q + 4, q + 8]);
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x19a))));
    g.write::<u32>(at(0x28), one);
    let who = g.read::<u32>(w(0x1bb).wrapping_add(4 * i));
    // call width: 4
    g.call(slot(g, 262), 0, &[who, r, r + 4, r + 8]);
    let rd = |g: &G, o: u32| g.read::<f32>(o);
    let qy = rd(g, q + 4);
    let dx = sub(rd(g, r), rd(g, q));
    g.write::<f32>(r + 4, qy);
    let dz = sub(rd(g, r + 8), rd(g, q + 8));
    g.write::<u32>(r + 0xc, one);
    // `_CIatan2(dz, dx)` on x87, the result rounded once (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let turn = wrap(g, sub(k(g, 0x228), ang), [0x237, 0x247, 0x251, 0x25e]);
    g.write::<f32>(at(4), turn);
    let (v, o, m) = (at(0x2c), at(0x3c), at(0x4c));
    // `V` = (0, 0, 10.0), its `w` 1.0 after the turns
    for (i, x) in (0u32..).zip([0, 0, 0x4120_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, turn.to_bits()]);
    g.write::<u32>(v + 0xc, one);
    // call width: 3
    g.call(slot(g, 229), 0, &[o, m, v]);
    // call width: 3
    g.call(slot(g, 248), 0, &[r, r, o]);
    let rx = g.read::<u32>(r);
    g.write::<u32>(w(0x31e), rx);
    let rz = g.read::<u32>(r + 8);
    // the record's start: `(R, 1.2, 0.99)` on the stack, `xmm3` = `K7`
    g.write::<u32>(below(0xc), 0x3f7d_70a4);
    g.write::<u32>(below(0x10), 0x3f99_999a);
    g.write::<u32>(below(0x14), r);
    g.write::<u32>(r + 0xc, one);
    g.write::<u32>(w(0x348), rz);
    let node = Frame {
        at: img.rel32_target(f + 0x34d),
        esp: below(0x18),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0, 0, 0, crate::Xmm::f32(k(g, 0x313)).reg()],
    };
    crate::o6_fam_pairs5::global_node_start(g, img, &node);
    // the actor block
    let j = g.read::<u32>(w(0x352));
    let battle = g.read::<u32>(w(0x35b)) != 0;
    g.write::<u32>(w(0x361), j);
    g.write::<u32>(w(0x36d), w(0x371));
    g.write::<u32>(w(0x377), u32::MAX);
    // call width: 1
    let h = g.call(slot(g, if battle { 413 } else { 215 }), 0, &[j]).eax;
    let battle = g.read::<u32>(w(0x396)) != 0;
    g.write::<u32>(w(0x3a1), h);
    g.write::<u32>(w(0x3a7), u32::MAX);
    g.write::<u32>(w(0x3b1), 0);
    g.write::<u32>(w(0x3bb), 1);
    if battle {
        g.write::<u16>(w(0x3ce), 1);
    } else {
        g.write::<u16>(w(0x3c5), 0);
    }
    g.write::<u32>(w(0x3d4), 0x0080_ffff);
    g.write::<u8>(w(0x3de), 0xff);
    g.write::<u16>(w(0x3e6), 0);
    g.write::<u8>(w(0x3ee), 0);
    g.write::<u32>(w(0x3f5), 0x20);
    g.write::<u16>(w(0x400), 0x48);
    let sn = Frame {
        at: img.rel32_target(f + 0x407),
        esp: below(8),
        ..Frame::default()
    };
    crate::o4_fam3::static_node(g, img, &sn);
    let x = g.read::<u32>(w(0x40c));
    g.write::<u32>(w(0x42d), x);
    // the two fades: (`rel32`, `ecx`, `edx`, `xmm1`, `xmm2`, the stack word)
    for (n, (call, ecx, edx, x1, x2, word)) in [
        (0x432u32, w(0x428), 0x1e, 0x41c, 0x414, 0u32),
        (0x453, w(0x44e), 0x10, 0x442, 0x43a, 0x36),
    ]
    .into_iter()
    .enumerate()
    {
        let sp = below(8 + 4 * n as u32);
        g.write::<u32>(sp, word);
        let fade = Frame {
            at: img.rel32_target(f + call),
            esp: sp.wrapping_sub(4),
            eax: 0,
            ecx,
            edx,
            xmm: [
                0,
                crate::Xmm::f32(k(g, x1)).reg(),
                crate::Xmm::f32(k(g, x2)).reg(),
                0,
            ],
        };
        crate::s3_misc::fade_task(g, img, &fade);
    }
    // call width: 1
    g.call(slot(g, 271), 0, &[4]);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x46f), w(0x46a));
    g.write::<u32>(node + 4, 0);
    if g.read::<u32>(w(0x475)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x497), node);
    } else {
        let tail = g.read::<u32>(w(0x485));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x4a7), node);
    Ret::eax(0)
}

/// [`start_up_0311`] (the `rel32`s: the part set-up, the object start,
/// `_CIatan2`, the record start, the static node, the two fades, the
/// pool take, `__security_check_cookie`).
pub const START_UP_0311: Sig = Sig {
    len: 1208,
    hash: 0x95ef_d627_353b_4bca,
    calls: &[
        (0x4e, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x63, Callee::Sig(&crate::r9_misc::START_OBJECT_RESET_0311)),
        (0x216, Callee::Import("_CIatan2")),
        (
            0x34d,
            Callee::Sig(&crate::o6_fam_pairs5::GLOBAL_NODE_START_0311_1F60),
        ),
        (0x407, Callee::Sig(&crate::o4_fam3::STATIC_NODE_0311_24C0)),
        (0x432, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x453, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x46f, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x4b0, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0149`'s start-up (0x10003450, 1278 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0149`; an 8-aligned /GS
/// frame, the locals from `E` = `(ebp & !7) - 0x3c`; `A = *G_A`, `T =
/// TAB[*I]` the target):
/// - `*G_C` = slot 719 `(D, 8, 0)` (two globals set first); the parts
///   buffer ([`parts_buffer`](crate::s4_small::parts_buffer)); object 1
///   started ([`start_object_owners`](crate::s4_misc::start_object_owners),
///   scale `K1`, `(3, 0x26, 0, 0)`);
/// - `P` = slot 262 `(A, ..)` with `P.y` from slot 268 `(A, 6, V)`, `P.w`
///   1; `Q` (`E + 0x28`) = slot 262 `(T, ..)`, `Q.y` from slot 268 `(T, 6,
///   V)`; `d = Q - P`: the pitch `atan2(d.y, sqrt(d.z^2 + d.x^2))` and the
///   yaw `K2 - atan2(d.z, d.x)` (`_CIatan2`, x87, the root in double;
///   the yaw wrapped once by `T` above `H` or below `L`) into two globals,
///   a third -8.0; the task started ([`task_start`](crate::s3_misc::task_start));
/// - object 2 `(2, 0x30, 2, CB)`, its point slot 262 `(T, ..)` with `y`
///   from slot 268, its angle `pitch + K` wrapped, the yaw, -10.0, a
///   record of nine globals; object 3 `(0, 0x30, 0, 0)` (scale `K6`), its
///   point `(T, ..)` with `y` slot 12 `(T)` less `K7`, `pi / 2`, `yaw + K`
///   wrapped, -15.0; object 4 `(1, 0x2b, 0, 0)`;
/// - two globals the pitch and the yaw; slot 326 `(O, P, X2, 0.5)`, two
///   counts, the actor block ([`me_block`](crate::sa_misc::me_block));
///   a pool node for the DLL's task ([`take`](crate::sa_pool::take))
///   appended to the task list.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0149<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x15), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x3c);
    let at = |o: u32| e.wrapping_add(o);
    let below = |o: u32| e.wrapping_sub(o);
    let one = 0x3f80_0000u32;
    let wrap = |g: &G, x: f32, [hi, turn, lo, turn2]: [u32; 4]| {
        if x > k(g, hi) {
            sub(x, k(g, turn))
        } else if k(g, lo) > x {
            add(x, k(g, turn2))
        } else {
            x
        }
    };
    g.write::<u32>(w(0x2b), w(0x2f));
    g.write::<u32>(w(0x35), w(0x39));
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x25), 8, 0]).eax;
    g.write::<u32>(w(0x43), c);
    let pb = Frame {
        at: img.rel32_target(f + 0x48),
        esp: below(0x10),
        ..Frame::default()
    };
    crate::s4_small::parts_buffer(g, img, &pb);
    let start = |g: &mut G, call: u32, obj: u32, scale: u32, words: [u32; 4]| {
        let [index, owner, a, b] = words;
        let at = img.rel32_target(f + call);
        crate::s4_misc::start_object_owners(g, img, at, w(obj), k(g, scale), index, owner, (a, b));
    };
    start(g, 0x62, 0x5d, 0x50, [3, 0x26, 0, 0]);
    let v = at(0x14);
    // a point (slot 262 into `xyz`) with its `y` from slot 268 `(who, 6, V)`
    let point = |g: &mut G, who: u32, xyz: [u32; 3]| {
        // call width: 4
        g.call(slot(g, 262), 0, &[who, xyz[0], xyz[1], xyz[2]]);
        // call width: 3
        g.call(slot(g, 268), 0, &[who, 6, v]);
        g.read::<u32>(v + 4)
    };
    let a = g.read::<u32>(w(0x6d));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, w(0x82), w(0x7d), w(0x78)]);
    // call width: 3
    g.call(slot(g, 268), 0, &[a, 6, v]);
    let y = g.read::<u32>(v + 4);
    // `T[*I]`: `I`'s and the table's operands
    let target = |g: &G, (o, tab): (u32, u32)| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(o))));
        g.read::<u32>(w(tab).wrapping_add(4 * i))
    };
    g.write::<u32>(w(0xad), y);
    g.write::<u32>(w(0xb3), one);
    let t = target(g, (0xa5, 0xc1));
    let q = at(0x28);
    // call width: 4
    g.call(slot(g, 262), 0, &[t, q, q + 4, q + 8]);
    // call width: 3
    g.call(slot(g, 268), 0, &[t, 6, v]);
    let rd = |g: &G, o: u32| g.read::<f32>(o);
    let qy = rd(g, v + 4);
    let dx = sub(rd(g, q), k(g, 0x10a));
    let dz = sub(rd(g, q + 8), k(g, 0x118));
    g.write::<f32>(q + 4, qy);
    let dy = sub(qy, k(g, 0x126));
    g.write::<u32>(q + 0xc, one);
    let r = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt();
    let pitch = g.atan2(f64::from(dy), r);
    g.write::<f64>(at(4), pitch);
    // `cvtpd2ps`: one rounding each
    #[allow(clippy::cast_possible_truncation)]
    let yaw0 = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = wrap(g, sub(k(g, 0x1b8), yaw0), [0x1cf, 0x1c8, 0x1e5, 0x1c8]);
    g.write::<f32>(e, yaw);
    #[allow(clippy::cast_possible_truncation)]
    let pitch = pitch as f32;
    g.write::<f32>(w(0x20c), pitch);
    g.write::<f32>(w(0x214), yaw);
    g.write::<u32>(w(0x21a), 0xc100_0000);
    // `push ecx; push D`: the task's argument (the word under it unread)
    g.write::<u32>(below(0xc), w(0x204));
    let ts = Frame {
        at: img.rel32_target(f + 0x223),
        esp: below(0x10),
        ..Frame::default()
    };
    crate::s3_misc::task_start(g, img, &ts);
    g.write::<u32>(w(0x241), u32::MAX);
    start(g, 0x24a, 0x23b, 0x22b, [2, 0x30, 2, w(0x230)]);
    let t = target(g, (0x24f, 0x263));
    let y2 = point(g, t, [w(0x26d), w(0x25c), w(0x254)]);
    g.write::<u32>(w(0x299), y2);
    let p2 = wrap(g, add(pitch, k(g, 0x2ae)), [0x2bf, 0x2c9, 0x2d3, 0x2e0]);
    g.write::<u32>(w(0x2b4), one);
    g.write::<f32>(w(0x2f4), p2);
    g.write::<f32>(w(0x30a), yaw);
    g.write::<u32>(w(0x310), 0xc120_0000);
    for o in [0x31au32, 0x324, 0x32e, 0x338] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x342), w(0x346));
    g.write::<u32>(w(0x34c), w(0x350));
    for o in [0x356u32, 0x360] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u16>(w(0x36a), 0);
    g.write::<u32>(w(0x370), w(0x374));
    start(g, 0x379, 0x302, 0x2e8, [0, 0x30, 0, 0]);
    let t = target(g, (0x37e, 0x392));
    // call width: 4
    g.call(slot(g, 262), 0, &[t, w(0x39c), w(0x38b), w(0x383)]);
    // call width: 1
    let y3 = st0_f32(g.call(slot(g, 12), 0, &[t]).st0);
    g.write::<f32>(w(0x3bc), y3);
    g.write::<u32>(w(0x3cd), one);
    g.write::<u32>(w(0x3d7), 0x3fc9_0fdb);
    let y3a = wrap(g, add(yaw, k(g, 0x3c4)), [0x3e2, 0x3ec, 0x3f6, 0x403]);
    g.write::<f32>(w(0x413), y3a);
    let y3b = sub(k(g, 0x41b), k(g, 0x423));
    g.write::<f32>(w(0x438), y3b);
    g.write::<u32>(w(0x43e), 0xc170_0000);
    start(g, 0x447, 0x430, 0x40b, [1, 0x2b, 0, 0]);
    g.write::<f32>(w(0x461), pitch);
    g.write::<f32>(w(0x47b), yaw);
    // the words slot 326 sees: `(O, P, X2, 0.5)` over the second zero
    // call width: 5
    g.call(
        slot(g, 326),
        0,
        &[w(0x48b), w(0x486), w(0x473), 0x3f00_0000, 0],
    );
    g.write::<u32>(w(0x498), 0x19);
    g.write::<u32>(w(0x4a2), 0x30);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x4ab), w(0x492));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x4b8), w(0x4b3));
    g.write::<u32>(node + 4, 0);
    if g.read::<u32>(w(0x4be)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x4e0), node);
    } else {
        let tail = g.read::<u32>(w(0x4ce));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x4ed), node);
    Ret::eax(0)
}

/// [`start_up_0149`] (the `rel32`s: the parts buffer, the four object
/// starts, the root and `_CIatan2` (twice), the task start, the actor
/// block, the pool take, `__security_check_cookie`).
pub const START_UP_0149: Sig = Sig {
    len: 1278,
    hash: 0x7ee2_abe9_4f1c_7c97,
    calls: &[
        (0x48, Callee::Sig(&crate::s4_small::PARTS_BUFFER_80000)),
        (0x62, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x164, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x177, Callee::Import("_CIatan2")),
        (0x1a6, Callee::Import("_CIatan2")),
        (0x223, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x24a, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x379, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x447, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x4ab, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x4b8, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x4f6, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0435`'s key curves (0x10001660, 1013 bytes; internal, no
/// arguments, called by its start-up; an 8-aligned /GS frame, the locals
/// from `E` = `(ebp & !7) - 0x9c`; `T = TAB[*I]` the target):
/// - `P0` (`E + 0x24`) = slot 262 `(T, ..)`, `P0.w` 1; the matrix `M`
///   (`E + 0x54`) from slot 235, turned by slot 329 `(M, M, yaw)` with
///   `yaw = K1 - slot 333 (T)` wrapped once by `K2` (above `K3`, below
///   `K4`; `comiss` + `jbe`);
/// - key 0 (`KEY`, the immediate `+0x185` less 8) = `(P0.x, P0.y, P0.z,
///   yaw')`, `yaw'` slot 333 again, wrapped the same way;
/// - keys 1..3: `M` times the three vectors at `V` (the immediate `+0x180`,
///   slot 229 into `O` at `E + 0x34`) plus `P0`, `w` the turn `K1 -
///   atan2(z - z', x - x')` from the key before (`_CIatan2`, x87, one
///   rounding), wrapped;
/// - the actor block (`*G_B` 0 or not: slot 215 or 413 `(*G_X)`, -1, 0,
///   1, the u16 2, the record `T`), the count `G_C` 0x28, the time 0;
/// - a node off the free ring for the step (`curve_step_0435`, the
///   immediate `+0x39c`) appended to the busy list
///   ([`take_busy_node`](crate::o6_fam_pairs5::take_busy_node), no
///   record).
///
/// Returns the node take's `eax` (the old last node, or 0xffff).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn curve_keys_0435<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x3c), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x9c);
    let at = |o: u32| e.wrapping_add(o);
    let one = 0x3f80_0000u32;
    let target = |g: &G, tab: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x1b))));
        g.read::<u32>(w(tab).wrapping_add(4 * i))
    };
    let wrap = |g: &G, x: f32, [hi, turn, lo, turn2]: [u32; 4]| {
        if x > k(g, hi) {
            sub(x, k(g, turn))
        } else if k(g, lo) > x {
            add(x, k(g, turn2))
        } else {
            x
        }
    };
    let (p0, m, o) = (at(0x24), at(0x54), at(0x34));
    let t = target(g, 0x37);
    // call width: 4
    g.call(slot(g, 262), 0, &[t, p0, p0 + 4, p0 + 8]);
    g.write::<u32>(p0 + 0xc, one);
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    let t = target(g, 0x70);
    // call width: 1
    let a = st0_f32(g.call(slot(g, 333), 0, &[t]).st0);
    g.write::<f32>(at(4), a);
    let yaw = wrap(g, sub(k(g, 0x89), a), [0xa1, 0x97, 0xb1, 0x97]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, yaw.to_bits()]);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let key = w(0x185).wrapping_sub(8);
    let (px, pz) = (rd(g, p0), rd(g, p0 + 8));
    g.write::<f32>(w(0xec), px);
    g.write::<f32>(at(4), px);
    let py = rd(g, p0 + 4);
    g.write::<f32>(w(0x100), py);
    let pw = rd(g, p0 + 0xc);
    g.write::<f32>(w(0x10e), pz);
    g.write::<f32>(w(0x116), pw);
    g.write::<f32>(at(0x10), pz);
    let t = target(g, 0x123);
    // call width: 1
    let a2 = st0_f32(g.call(slot(g, 333), 0, &[t]).st0);
    g.write::<f32>(at(0x14), a2);
    let yaw2 = wrap(g, sub(k(g, 0x13e), a2), [0x152, 0x15c, 0x166, 0x173]);
    g.write::<f32>(w(0x17b), yaw2);
    // keys 1..3 from the vectors at `V`
    let (mut prev_x, mut prev_z) = (px, pz);
    for i in 1..4u32 {
        let v = w(0x180).wrapping_add(0x10 * (i - 1));
        let kk = key.wrapping_add(0x10 * i);
        // call width: 3
        g.call(slot(g, 229), 0, &[o, m, v]);
        for j in 0..3u32 {
            let c = add(rd(g, o + 4 * j), rd(g, p0 + 4 * j));
            g.write::<f32>(kk + 4 * j, c);
        }
        let w1 = g.read::<u32>(p0 + 0xc);
        g.write::<u32>(kk + 0xc, w1);
        let (x, z) = (rd(g, kk), rd(g, kk + 8));
        // `_CIatan2(z - z', x - x')`, its result rounded once (`cvtpd2ps`)
        #[allow(clippy::cast_possible_truncation)]
        let ang = g.atan2(f64::from(sub(z, prev_z)), f64::from(sub(x, prev_x))) as f32;
        let turn = wrap(g, sub(k(g, 0x22d), ang), [0x23c, 0x246, 0x250, 0x25d]);
        g.write::<f32>(kk + 0xc, turn);
        prev_x = rd(g, kk);
        prev_z = rd(g, kk + 8);
    }
    // the actor block
    let battle = g.read::<u32>(w(0x2a0)) != 0;
    let x = g.read::<u32>(w(0x2a6));
    g.write::<u32>(w(0x2ab), x);
    g.write::<u32>(w(0x2b7), 0x28);
    g.write::<u32>(w(0x2c1), 0);
    g.write::<u32>(w(0x2cb), w(0x2cf));
    g.write::<u32>(w(0x2d5), u32::MAX);
    // call width: 1
    let h = g.call(slot(g, if battle { 413 } else { 215 }), 0, &[x]).eax;
    let free = g.read::<u16>(w(0x2f2));
    g.write::<u32>(w(0x2f7), h);
    g.write::<u16>(w(0x302), 2);
    g.write::<u32>(w(0x30d), u32::MAX);
    g.write::<u32>(w(0x317), 0);
    g.write::<u32>(w(0x321), 1);
    let t = target(g, 0x332);
    g.write::<u32>(w(0x337), t);
    let ops = crate::o6_fam_pairs5::NodeOps {
        a: 0,
        t217: (0, 0),
        t234: (0, 0),
        t233: [(0, 0); 3],
        free: 0x2f2,
        k_a4: 0,
        k_9c: 0,
        head: 0x346,
        callback: 0x39c,
        free_w: 0x3a7,
        last_cmp: 0x3ad,
        last: 0x3c0,
        last_w: 0x3d3,
        first_w: 0x3e5,
    };
    // no record: the node's `+0x14` stays the 0 the take wrote
    let eax = crate::o6_fam_pairs5::take_busy_node(g, img, f, free, &ops, 0, true);
    Ret::eax(eax)
}

/// [`curve_keys_0435`] (the `rel32`s: `_CIatan2` and
/// `__security_check_cookie`; the loop's `jmp` stays inside).
pub const CURVE_KEYS_0435: Sig = Sig {
    len: 1013,
    hash: 0x2a57_c705_24fa_efba,
    calls: &[(0x21b, Callee::Import("_CIatan2")), (0x3ed, Callee::Any)],
    imports: &[],
};

/// `magic_0435`'s start-up (0x10001340, 550 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0435`; an `ebp` frame,
/// `[ebp - 4]` the yaw's slot; `X = *G_X` the caster, `T = *G_T`):
/// - `*G_H` = slot 219 `(X, 0)` (two globals set first), the byte flag
///   0x1e, a word 1; `*G_C` = slot 719 `(D, 8, 0)`; the part set-up
///   ([`part_init`](crate::l5_objhost::part_init)); the object started
///   ([`start_object_reset_bare`](crate::r9_misc::start_object_reset_bare),
///   scale `K`);
/// - the point `P` = slot 262 `(T, ..)`, `P.y -= K'`, `P.w` 1; the yaw
///   `K1 - slot 333 (T)` wrapped once (above `H`, below `L`) into a
///   global; the key curves ([`curve_keys_0435`]);
/// - two words, slot 220 `(slot 719 (D, 6, 0), 0x40, 0x100)`; the actor
///   block (slot 413 or 215 `(X)` by `*G_B`, -1, 0, 1, the u16 flag);
/// - a pool node for the DLL's task ([`take`](crate::sa_pool::take))
///   appended to the task list.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0435<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x5), n);
    let ebp = fr.esp.wrapping_sub(4);
    let x = g.read::<u32>(w(0xd));
    g.write::<u32>(w(0x19), w(0x1d));
    g.write::<u32>(w(0x23), w(0x27));
    // call width: 2
    let h = g.call(slot(g, 219), 0, &[x, 0]).eax;
    g.write::<u32>(w(0x2e), h);
    g.write::<u8>(w(0x3b), 0x1e);
    g.write::<u32>(w(0x42), 1);
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x53), 8, 0]).eax;
    g.write::<u32>(w(0x5d), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x62));
    let start = Frame {
        at: img.rel32_target(f + 0x77),
        esp: ebp.wrapping_sub(0x18),
        eax: 0,
        ecx: 0,
        edx: w(0x72),
        xmm: [0, 0, 0, crate::Xmm::f32(k(g, 0x6a)).reg()],
    };
    crate::r9_misc::start_object_reset_bare(g, img, &start);
    let t = g.read::<u32>(w(0x97));
    // call width: 4
    g.call(slot(g, 262), 0, &[t, w(0x91), w(0x8c), w(0x81)]);
    let py = sub(k(g, 0xa1), k(g, 0xa9));
    let t = g.read::<u32>(w(0xb4));
    g.write::<u32>(w(0xba), 0x3f80_0000);
    g.write::<f32>(w(0xc6), py);
    // call width: 1
    let a = st0_f32(g.call(slot(g, 333), 0, &[t]).st0);
    g.write::<f32>(ebp.wrapping_sub(4), a);
    let yaw = sub(k(g, 0xd9), a);
    let yaw = if yaw > k(g, 0xe8) {
        sub(yaw, k(g, 0xf2))
    } else if k(g, 0xfc) > yaw {
        add(yaw, k(g, 0x109))
    } else {
        yaw
    };
    g.write::<f32>(w(0x111), yaw);
    let keys = Frame {
        at: img.rel32_target(f + 0x116),
        esp: ebp.wrapping_sub(8),
        ..Frame::default()
    };
    curve_keys_0435(g, img, &keys);
    g.write::<u32>(w(0x130), 0x32);
    g.write::<u32>(w(0x13a), 0);
    // call width: 3
    let r = g.call(slot(g, 719), 0, &[w(0x12a), 6, 0]).eax;
    // call width: 3
    g.call(slot(g, 220), 0, &[r, 0x40, 0x100]);
    // the actor block
    let x = g.read::<u32>(w(0x15a));
    let battle = g.read::<u32>(w(0x163)) != 0;
    g.write::<u32>(w(0x169), x);
    g.write::<u32>(w(0x175), w(0x179));
    g.write::<u32>(w(0x17f), u32::MAX);
    // call width: 1
    let m = g.call(slot(g, if battle { 413 } else { 215 }), 0, &[x]).eax;
    let battle = g.read::<u32>(w(0x19e)) != 0;
    g.write::<u32>(w(0x1a9), m);
    g.write::<u32>(w(0x1af), u32::MAX);
    g.write::<u32>(w(0x1b9), 0);
    g.write::<u32>(w(0x1c3), 1);
    if battle {
        g.write::<u16>(w(0x1d6), 1);
    } else {
        g.write::<u16>(w(0x1cd), 0);
    }
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x1e0), w(0x1db));
    g.write::<u32>(node + 4, 0);
    if g.read::<u32>(w(0x1e6)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x214), node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(0x21c), node);
    } else {
        let tail = g.read::<u32>(w(0x1f6));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(0x203), node);
    }
    Ret::eax(0)
}

/// [`start_up_0435`] (the `rel32`s: the part set-up, the object start,
/// the key curves, the pool take).
pub const START_UP_0435: Sig = Sig {
    len: 550,
    hash: 0x349f_1709_4fd6_18af,
    calls: &[
        (0x62, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x77, Callee::Sig(&crate::r9_misc::START_OBJECT_BARE_0435)),
        (0x116, Callee::Sig(&CURVE_KEYS_0435)),
        (0x1e0, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0342`'s summon visitor (0x10001990, 1251 bytes; internal:
/// `ecx` = the cursor `C` = `{node, flag +4, pass +5}`; a 16-aligned /GS
/// frame, the locals from `E` = `(ebp & !0xf) - 0x128`; `n0` the cursor's
/// node). Unless `n0`'s pass byte `+0x1a` is already `C`'s (the flag 0,
/// nothing more), each child on `n0`'s ring (`+0x10`, linked by `+8`) is
/// visited (`C`'s node the child, its method `+0x1c` `(C)`, `ecx` `n0`),
/// then `C` back on `n0` with its pass byte and the flag 1. Then:
/// - the cursor `C2` = `{n0[0x48], pass}` (`E + 0x1c`; its flag byte
///   never written: 0, the junk-read rule) visits the child `n0[0x48]`
///   (method `+0x1c`, `ecx` = `C2`);
/// - while `n0[0x54]` is below `K` and its count `n0[0x5c]` runs out: a
///   new count `(r & 3) + 1` and a span `(r >> 2 & 3) + 2` (`r` slot
///   324), the span taken from `n0`'s ring
///   ([`ring_take`](crate::m0070::ring_take), the request `{n0, span}` at
///   `E + 0x34`); slot 237 `(child[0x54], 0, 0, B)` (`B` = `E + 0xd0`),
///   the matrix `M` (`E + 0x90`) from slot 235; each record of the one or
///   two pieces (u16 counts at `E + 0x3a`, addresses at `E + 0x40`) set
///   up from slots 307 (ST0) and 324 (a kind, a colour, a size), turned
///   by two slot 324 angles ([`rotation_19d0`](crate::sb_matrix::rotation_19d0)),
///   its point slot 229 `(rec, M, (20, 0, 0, 1))` plus `B` (slot 248),
///   `w` 1.0;
/// - past `K` with no records left the node leaves its parent
///   ([`leave`](crate::sa_misc::leave)) and that is all;
/// - the draw: a record `{u16 0x80, colour, 0, M2, *G_Y, 0, M2}` at `E +
///   0x48` (`M2` = `E + 0xe0`); for both pieces (`n0 + 0x24` addresses,
///   `n0 + 0x2c` u16 counts) each live record (byte `+0x34`) gets slot 229
///   `(E + 0xc0, G_M, rec)`, the scaled diagonal `rec[0x20] * rec[0x2c]`
///   and slot 231 `(M2, G_M2, E + 0x90)`; its fade `+0x2c` grows by `K'`
///   while below `K''` (capped at 0.7 past `K'''`), and the motion step
///   ([`motion_sequence_step`](crate::s6_misc::motion_sequence_step),
///   `ecx` = `rec + 0x10`, `edx` = `n0[0x4c + 4 * rec[0x24]]`, the stack
///   `(E + 0x48)`) answers its live byte; `n0[0x58]` counts them;
/// - `n0[0x54] += K4`, `*G_Y` = the record's word.
///
/// Returns `*G_Y`'s new value (the draw), the leave's `eax`, or the
/// cookie word with the pass byte (the early way).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn visit_0342<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x102), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !0xf).wrapping_sub(0x128);
    let at = |o: u32| e.wrapping_add(o);
    let one = 0x3f80_0000u32;
    let c = fr.ecx;
    let n0 = g.read::<u32>(c);
    let pass = g.read::<u8>(c + 5);
    let method = |g: &G, obj: u32| g.read::<u32>(g.read::<u32>(obj).wrapping_add(0x1c));
    if g.read::<u8>(n0 + 0x1a) == pass {
        g.write::<u8>(c + 4, 0);
    } else {
        let mut child = g.read::<u32>(n0 + 0x10);
        while child != n0 {
            g.write::<u32>(c, child);
            let next = g.read::<u32>(child + 8);
            let m = method(g, child);
            // call width: 1
            g.call(m, n0, &[c]);
            child = next;
        }
        let pass = g.read::<u8>(c + 5);
        g.write::<u32>(c, n0);
        g.write::<u8>(n0 + 0x1a, pass);
        g.write::<u8>(c + 4, 1);
    }
    if g.read::<u8>(c + 4) & 1 == 0 {
        // the cookie word `xor`ed with the frame, its low byte the pass
        let cookie = g.read::<u32>(w(0xd)) ^ e;
        let b = g.read::<u8>(n0 + 0x1a);
        return Ret::eax((cookie & !0xff) | u32::from(b));
    }
    let n0 = g.read::<u32>(c);
    g.write::<u32>(at(0x10), n0);
    let child = g.read::<u32>(n0 + 0x48);
    let c2 = at(0x1c);
    g.write::<u32>(c2, child);
    // junk read: [C2 + 4] never written; 0 (brief 2026-10-24)
    g.write::<u8>(c2 + 4, 0);
    g.write::<u8>(c2 + 5, g.read::<u8>(c + 5));
    let m = method(g, child);
    // call width: 1
    let mut eax = g.call(m, c2, &[c2]).eax;
    let n0 = g.read::<u32>(at(0x10));
    let mut draw = true;
    if k(g, 0x98) > g.read::<f32>(n0 + 0x54) {
        let count = g.read::<u32>(n0 + 0x5c);
        g.write::<u32>(n0 + 0x5c, count.wrapping_sub(1));
        if count == 0 {
            // call width: 0
            let r = g.call(slot(g, 324), 0, &[]).eax;
            let n0 = g.read::<u32>(at(0x10));
            g.write::<u32>(n0 + 0x5c, (r & 3) + 1);
            let req = at(0x34);
            g.write::<u32>(req, n0);
            // (`sar` then `and 3`: the same two bits as a logical shift)
            g.write::<u16>(req + 4, ((r >> 2) & 3) as u16 + 2);
            crate::m0070::ring_take(g, req);
            let ch = g.read::<u32>(n0 + 0x48);
            let (b, mm) = (at(0xd0), at(0x90));
            let x = g.read::<u32>(ch + 0x54);
            // call width: 4
            g.call(slot(g, 237), 0, &[x, 0, 0, b]);
            // call width: 1
            g.call(slot(g, 235), 0, &[mm]);
            let v = at(0x80);
            g.write::<u32>(v + 8, 0);
            g.write::<u32>(v + 4, 0);
            g.write::<u32>(v + 0xc, one);
            for piece in 0..2u32 {
                let n = g.read::<u16>(at(0x3a + 2 * piece));
                g.write::<u32>(at(0x14), piece);
                let mut rec = g.read::<u32>(at(0x40 + 4 * piece));
                g.write::<u32>(at(0xc), u32::from(n));
                for _ in 0..n {
                    g.write::<u32>(rec + 0x10, 0);
                    // call width: 0
                    let s = st0_f32(g.call(slot(g, 307), 0, &[]).st0);
                    g.write::<f32>(at(0xc), s);
                    let s = sub(s, k(g, 0x190));
                    // call width: 0
                    let r = g.call(slot(g, 324), 0, &[]).eax;
                    g.write::<f32>(at(0x18), s);
                    let even = r.trailing_zeros() >= 2;
                    g.write::<u16>(rec + 0x12, r as u16);
                    g.write::<u32>(rec + 0x24, u32::from(even));
                    let size = if even {
                        g.write::<u32>(rec + 0x28, 0x8060_8080);
                        g.write::<u32>(rec + 0x30, 0);
                        add(mul(s, k(g, 0x1c6)), k(g, 0x1dc))
                    } else {
                        let x = add(mul(s, k(g, 0x1e6)), k(g, 0x1f1));
                        g.write::<u32>(rec + 0x28, 0x8080_8060);
                        g.write::<u32>(rec + 0x30, r & 0xffff);
                        x
                    };
                    let size = mul(size, k(g, 0x203));
                    g.write::<u8>(rec + 0x34, 1);
                    let size = mul(size, k(g, 0x214));
                    g.write::<u32>(rec + 0x2c, 0);
                    g.write::<f32>(rec + 0x20, add(size, k(g, 0x223)));
                    // call width: 0
                    let a = g.call(slot(g, 324), 0, &[]).eax;
                    g.write::<u32>(at(0x24), a);
                    // call width: 0
                    let b2 = g.call(slot(g, 324), 0, &[]).eax;
                    g.write::<u32>(at(0x28), b2);
                    // junk read: [E + 0x2c] (the third angle) never written;
                    // 0 (brief 2026-10-24)
                    g.write::<u32>(at(0x2c), 0);
                    let rot = img.rel32_target(f + 0x255);
                    crate::sb_matrix::rotation_19d0(g, img, rot, mm, at(0x24));
                    g.write::<u32>(v, 0x41a0_0000);
                    // call width: 3
                    g.call(slot(g, 229), 0, &[rec, mm, v]);
                    // call width: 3
                    g.call(slot(g, 248), 0, &[rec, b, rec]);
                    g.write::<u32>(rec + 0xc, one);
                    rec = rec.wrapping_add(0x40);
                }
            }
        }
    } else if g.read::<u32>(n0 + 0x58) == 0 {
        eax = crate::sa_misc::leave(g, n0, eax);
        draw = false;
    }
    if !draw {
        return Ret::eax(eax);
    }
    // the draw record at `E + 0x48`
    let n0 = g.read::<u32>(at(0x10));
    let m2 = at(0xe0);
    g.write::<u32>(at(0x64), m2);
    g.write::<u32>(at(0x58), m2);
    g.write::<u16>(at(0x4a), 0x80);
    let gy = g.read::<u32>(w(0x2e0));
    g.write::<u32>(at(0x5c), gy);
    for o in [
        0x94u32, 0x98, 0x9c, 0xa0, 0xa8, 0xac, 0xb0, 0xb4, 0xbc, 0x50,
    ] {
        g.write::<u32>(at(o), 0);
    }
    g.write::<u8>(at(0x60), 0);
    g.write::<u32>(n0 + 0x58, 0);
    let mut counts = n0.wrapping_add(0x2c);
    let mut addrs = n0.wrapping_add(0x24);
    for _ in 0..2 {
        g.write::<u32>(at(0x18), counts);
        g.write::<u32>(at(0x14), addrs);
        let mut rec = g.read::<u32>(addrs);
        let mut i = 0u32;
        while i < u32::from(g.read::<u16>(counts)) {
            if g.read::<u8>(rec + 0x34) != 0 {
                // call width: 3
                g.call(slot(g, 229), 0, &[at(0xc0), w(0x3a3), rec]);
                let d = mul(g.read::<f32>(rec + 0x20), g.read::<f32>(rec + 0x2c));
                for o in [0xb8u32, 0xa4, 0x90] {
                    g.write::<f32>(at(o), d);
                }
                // call width: 3
                g.call(slot(g, 231), 0, &[m2, w(0x3d2), at(0x90)]);
                let fade = g.read::<f32>(rec + 0x2c);
                if k(g, 0x408) > fade {
                    let fade = add(fade, k(g, 0x418));
                    g.write::<f32>(rec + 0x2c, fade);
                    if fade > k(g, 0x41f) {
                        g.write::<u32>(rec + 0x2c, 0x3f33_3333);
                    }
                }
                let colour = g.read::<u32>(rec + 0x28);
                g.write::<u32>(at(0x4c), colour);
                let n0 = g.read::<u32>(at(0x10));
                let kind = g.read::<u32>(rec + 0x24);
                let seq = g.read::<u32>(n0.wrapping_add(0x4c).wrapping_add(kind.wrapping_mul(4)));
                // `sub esp, 8; push E + 0x48`: the record on the stack
                let sp = e.wrapping_sub(0x14);
                g.write::<u32>(sp, at(0x48));
                let step = Frame {
                    at: img.rel32_target(f + 0x44f),
                    esp: sp.wrapping_sub(4),
                    eax: 0,
                    ecx: rec.wrapping_add(0x10),
                    edx: seq,
                    xmm: [0; 4],
                };
                let live = crate::s6_misc::motion_sequence_step(g, img, &step);
                g.write::<u8>(rec + 0x34, live as u8);
                let n0 = g.read::<u32>(at(0x10));
                let c = g.read::<u32>(n0 + 0x58).wrapping_add(1);
                g.write::<u32>(n0 + 0x58, c);
            }
            i += 1;
            rec = rec.wrapping_add(0x40);
        }
        addrs = addrs.wrapping_add(4);
        counts = counts.wrapping_add(2);
    }
    let n0 = g.read::<u32>(at(0x10));
    let t = add(g.read::<f32>(n0 + 0x54), k(g, 0x498));
    let y = g.read::<u32>(at(0x5c));
    g.write::<u32>(w(0x4a1), y);
    g.write::<f32>(n0 + 0x54, t);
    Ret::eax(y)
}

/// [`visit_0342`] (the `rel32`s: the ring take, the rotation, the motion
/// step, the node leave and `__security_check_cookie`, twice).
pub const VISIT_0342: Sig = Sig {
    len: 1251,
    hash: 0xfb94_24aa_9fea_b05b,
    calls: &[
        (0xeb, Callee::Sig(&crate::m0070::RING_TAKE)),
        (0x255, Callee::Sig(&crate::sb_matrix::ROTATION_19D0)),
        (0x44f, Callee::Sig(&crate::s6_misc::MOTION_SEQUENCE_STEP)),
        (0x4b6, Callee::Any),
        (0x4cb, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x4db, Callee::Any),
    ],
    imports: &[],
};

/// The thunk of [`visit_0342`] (`magic_0342` 0x10001e80: `ecx` = the
/// stack word, a jump with the caller's stack).
pub fn visit_0342_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::o6_fam_pairs3::thunk_to(g, img, fr, visit_0342::<G>)
}

/// [`visit_0342_thunk`].
pub const VISIT_0342_THUNK: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(0x8, Callee::Sig(&VISIT_0342))],
    imports: &[],
};

/// `magic_0419`'s start-up (0x100013e0, 1618 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0419`; an 8-aligned /GS
/// frame, the locals from `E` = `(ebp & !7) - 0x90`; `A = *G_A`, `T =
/// TAB[*I]`):
/// - three globals, `*G_C` = slot 719 `(D, 8, 0)`, the part set-up
///   ([`part_init`](crate::l5_objhost::part_init));
/// - `P` (`E + 0x28`) = slot 262 `(A, ..)`, `Q` (`E + 8`) = slot 262 `(T,
///   ..)` with `Q.y = P.y`; the yaw `K1 - atan2(Q.z - P.z, Q.x - P.x)`
///   (`_CIatan2`, x87) wrapped once; the height `h` = slot 251 `(*I)`
///   outside a battle with `*I` below 0x20, else slot 17 `(T)` times slot
///   19 `(T)` (ST0 each);
/// - the matrix `M` (`E + 0x48`) from slot 235 turned by slots 328 `(M,
///   M, 0)` and 329 `(M, M, yaw)`; `Q += M (0, 0, h, 1)` (slots 229, 248);
///   `n = trunc(|P - Q| / K6)` (the root in double);
/// - the object started ([`start_object_reset_edx`](crate::r9_misc::start_object_reset_edx),
///   `(-, n + 3)`, scale `K5`); the point `P2` = slot 262 `(A, ..)` with
///   `y` slot 12 `(A)` less `K8` and the yaw `K1 - slot 333 (A)` wrapped,
///   `P2.x`, `P2.z` = `Q.x`, `Q.z`; a record of globals; the camera path
///   task ([`camera_path_task`](crate::s7_misc::camera_path_task), `(Q,
///   0.7, 1.0, n)`, scale `K7`);
/// - the actor block (slot 413 or 215 `(*G_X)` by `*G_B`, -1, 0, 1, the
///   u16 flag), slot 256 `(A, 2.0, 0.3, 0.3, n)`, the colour words, the
///   counted task ([`counted_task`](crate::s4_small::counted_task)), two
///   fades ([`fade_task`](crate::s3_misc::fade_task): 9 or 16 steps by
///   `n < 7`), slot 271 `(4)`;
/// - three task nodes ([`take`](crate::sa_pool::take), the second
///   after the first, the third flagged 2) and the pool reset
///   ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)).
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0419<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x90);
    let at = |o: u32| e.wrapping_add(o);
    let s = e.wrapping_sub(8);
    let one = 0x3f80_0000u32;
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let wrap = |g: &G, x: f32, [hi, turn, lo, turn2]: [u32; 4]| {
        if x > k(g, hi) {
            sub(x, k(g, turn))
        } else if k(g, lo) > x {
            add(x, k(g, turn2))
        } else {
            x
        }
    };
    for (dst, src) in [(0x32u32, 0x36u32), (0x3c, 0x40), (0x46, 0x4a)] {
        g.write::<u32>(w(dst), w(src));
    }
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x2c), 8, 0]).eax;
    g.write::<u32>(w(0x54), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x59));
    let (p, q) = (at(0x28), at(8));
    let a = g.read::<u32>(w(0x73));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, p, p + 4, p + 8]);
    let idx = |g: &G| u32::from(g.read::<u8>(g.read::<u32>(w(0x80))));
    g.write::<u32>(p + 0xc, one);
    let t = g.read::<u32>(w(0xa1).wrapping_add(4 * idx(g)));
    // call width: 4
    g.call(slot(g, 262), 0, &[t, q, q + 4, q + 8]);
    let py = g.read::<u32>(p + 4);
    g.write::<u32>(q + 4, py);
    let dz = sub(rd(g, q + 8), rd(g, p + 8));
    g.write::<u32>(q + 0xc, one);
    let dx = sub(rd(g, q), rd(g, p));
    // `_CIatan2(dz, dx)`, one rounding (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = wrap(g, sub(k(g, 0x10e), ang), [0x125, 0x11e, 0x13b, 0x11e]);
    g.write::<f32>(e, yaw);
    g.write::<u32>(at(0x38), 0);
    g.write::<u32>(at(0x3c), 0);
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x157))));
    let h = if g.read::<u32>(w(0x150)) == 0 && i < 0x20 {
        // call width: 1
        let x = st0_f32(g.call(slot(g, 251), 0, &[i]).st0);
        g.write::<f32>(at(4), x);
        x
    } else {
        let t = g.read::<u32>(w(0x198).wrapping_add(4 * i));
        // call width: 1
        let a19 = st0_f32(g.call(slot(g, 19), 0, &[t]).st0);
        g.write::<f32>(at(0x18), a19);
        // call width: 1
        let a17 = st0_f32(g.call(slot(g, 17), 0, &[t]).st0);
        g.write::<f32>(at(4), a17);
        mul(a17, a19)
    };
    let m = at(0x48);
    g.write::<f32>(at(0x40), h);
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, g.read::<u32>(e)]);
    let (o, v) = (at(0x18), at(0x38));
    g.write::<u32>(at(0x44), one);
    // call width: 3
    g.call(slot(g, 229), 0, &[o, m, v]);
    // call width: 3
    g.call(slot(g, 248), 0, &[q, q, o]);
    g.write::<u32>(q + 0xc, one);
    let d = [0u32, 4, 8].map(|j| sub(rd(g, p + j), rd(g, q + j)));
    let sum = add(add(mul(d[1], d[1]), mul(d[0], d[0])), mul(d[2], d[2]));
    #[allow(clippy::cast_possible_truncation)] // `cvtsd2ss`
    let dist = f64::from(sum).sqrt() as f32;
    let n = crate::s4_summon::trunc(div(dist, k(g, 0x2ab)));
    // the object start's stack `(*G_I, n + 3)` (the first word unread)
    g.write::<u32>(s.wrapping_sub(0xc), n.wrapping_add(3));
    g.write::<u32>(s.wrapping_sub(0x10), 0);
    let start = Frame {
        at: img.rel32_target(f + 0x2c1),
        esp: s.wrapping_sub(0x14),
        eax: 0,
        ecx: 0,
        edx: w(0x2b7),
        xmm: [0, 0, 0, crate::Xmm::f32(k(g, 0x2a3)).reg()],
    };
    crate::r9_misc::start_object_reset_edx(g, img, &start);
    let a = g.read::<u32>(w(0x2cc));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, w(0x2e1), w(0x2dc), w(0x2d7)]);
    // call width: 1
    let y2 = st0_f32(g.call(slot(g, 12), 0, &[a]).st0);
    g.write::<f32>(w(0x2f5), y2);
    let a = g.read::<u32>(w(0x2fb));
    g.write::<u32>(w(0x301), one);
    // call width: 1
    let t2 = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    g.write::<f32>(at(0x18), t2);
    let yaw2 = wrap(g, sub(k(g, 0x31e), t2), [0x32e, 0x338, 0x342, 0x34f]);
    g.write::<f32>(w(0x35f), yaw2);
    let y2 = sub(k(g, 0x367), k(g, 0x36f));
    g.write::<u16>(w(0x37b), 0);
    g.write::<f32>(w(0x383), y2);
    let qx = g.read::<u32>(q);
    g.write::<u32>(w(0x391), qx);
    let qz = g.read::<u32>(q + 8);
    g.write::<u32>(w(0x3b1), w(0x3b5));
    for o in [0x3bbu32, 0x3c5, 0x3cf, 0x3d9] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x3e3), w(0x3e7));
    g.write::<u32>(w(0x3ed), w(0x3f1));
    g.write::<u32>(w(0x3f7), 0);
    g.write::<u32>(w(0x401), 0);
    g.write::<u32>(w(0x40b), w(0x40f));
    g.write::<u32>(w(0x417), qz);
    // the path task's `(Q, 0.7, 1.0, n)`, `xmm3` = `K7`
    for (j, x) in (0u32..).zip([q, 0x3f33_3333, one, n]) {
        g.write::<u32>(s.wrapping_sub(0x10) + 4 * j, x);
    }
    let path = Frame {
        at: img.rel32_target(f + 0x41c),
        esp: s.wrapping_sub(0x14),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0, 0, 0, crate::Xmm::f32(k(g, 0x357)).reg()],
    };
    crate::s7_misc::camera_path_task(g, img, &path);
    // the actor block
    let x = g.read::<u32>(w(0x421));
    let battle = g.read::<u32>(w(0x42a)) != 0;
    g.write::<u32>(w(0x430), x);
    g.write::<u32>(w(0x43c), w(0x440));
    g.write::<u32>(w(0x446), u32::MAX);
    // call width: 1
    let hm = g.call(slot(g, if battle { 413 } else { 215 }), 0, &[x]).eax;
    let battle = g.read::<u32>(w(0x465)) != 0;
    g.write::<u32>(w(0x470), hm);
    g.write::<u32>(w(0x476), u32::MAX);
    g.write::<u32>(w(0x480), 0);
    g.write::<u32>(w(0x48a), 1);
    if battle {
        g.write::<u16>(w(0x49d), 1);
    } else {
        g.write::<u16>(w(0x494), 0);
    }
    g.write::<u16>(w(0x4ad), 1);
    g.write::<u32>(w(0x4c3), 9);
    let a = g.read::<u32>(w(0x4da));
    // call width: 5
    g.call(
        slot(g, 256),
        0,
        &[a, 0x4000_0000, 0x3e99_999a, 0x3e99_999a, n],
    );
    g.write::<u32>(w(0x4e5), 0x0080_ffff);
    g.write::<u8>(w(0x4ef), 0xff);
    g.write::<u16>(w(0x4f7), 0);
    g.write::<u8>(w(0x4ff), 0);
    g.write::<u32>(w(0x505), n.wrapping_add(5));
    g.write::<u16>(w(0x50c), 0x48);
    let ct = Frame {
        at: img.rel32_target(f + 0x513),
        esp: s.wrapping_sub(0x18),
        ..Frame::default()
    };
    crate::s4_small::counted_task(g, img, &ct);
    let c5 = g.read::<u32>(w(0x518));
    g.write::<u32>(w(0x52d), c5);
    let steps = if n.cast_signed() < 7 { 9 } else { 0x10 };
    // the two fades: (`rel32`, `ecx`, `edx`, `xmm1`, `xmm2`, the stack word)
    for (j, (call, ecx, edx, x1, x2, word)) in [
        (0x54bu32, w(0x546), steps, 0x528, 0x520, 0u32),
        (
            0x570,
            w(0x56b),
            0x10,
            0x55b,
            0x553,
            n.wrapping_sub(1).wrapping_add(steps),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let sp = s.wrapping_sub(0x18 + 4 * j as u32);
        g.write::<u32>(sp, word);
        let fade = Frame {
            at: img.rel32_target(f + call),
            esp: sp.wrapping_sub(4),
            eax: 0,
            ecx,
            edx,
            xmm: [
                0,
                crate::Xmm::f32(k(g, x1)).reg(),
                crate::Xmm::f32(k(g, x2)).reg(),
                0,
            ],
        };
        crate::s3_misc::fade_task(g, img, &fade);
    }
    // call width: 1
    g.call(slot(g, 271), 0, &[4]);
    // the task nodes
    let n1 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x58c), w(0x587));
    let mut first = g.read::<u32>(w(0x592));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 4, 0);
    if first == 0 {
        first = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x5be), n1);
    } else {
        let last = g.read::<u32>(w(0x5a9));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x5cb), w(0x5c3));
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        // (not reached: `first` is the first node or `n1`)
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x5e8), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x5f5), n2);
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x5fa), w(0x5ed));
    let n3 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x604), w(0x5ff));
    let empty = g.read::<u32>(w(0x60a)) == 0;
    g.write::<u8>(n3 + 0x18, 2);
    g.write::<u32>(n3 + 4, 0);
    if empty {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x630), n3);
    } else {
        let last = g.read::<u32>(w(0x61e));
        g.write::<u32>(n3, last);
        g.write::<u32>(last + 4, n3);
    }
    g.write::<u32>(n3 + 8, n3);
    g.write::<u32>(w(0x641), n3);
    Ret::eax(0)
}

/// [`start_up_0419`] (the `rel32`s: the part set-up, `_CIatan2`, the
/// root, the object start, the path task, the counted task, two fades,
/// three pool takes, the pool reset, `__security_check_cookie`).
pub const START_UP_0419: Sig = Sig {
    len: 1618,
    hash: 0x4a3a_5816_cae1_3b2a,
    calls: &[
        (0x59, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xfc, Callee::Import("_CIatan2")),
        (0x297, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2c1, Callee::Sig(&crate::r9_misc::START_OBJECT_EDX_0419)),
        (0x41c, Callee::Sig(&crate::s7_misc::CAMERA_PATH_TASK)),
        (0x513, Callee::Sig(&crate::s4_small::COUNTED_TASK_20)),
        (0x54b, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x570, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x58c, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x5cb, Callee::Sig(&crate::sa_pool::TAKE)),
        (
            0x5fa,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0205_2A60),
        ),
        (0x604, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x64a, Callee::Any),
    ],
    imports: &[],
};
