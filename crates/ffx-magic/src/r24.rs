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
