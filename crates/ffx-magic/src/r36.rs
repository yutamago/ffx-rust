//! Round 36's ports (magic-r36): the `magic_0515` group (the last shared
//! rows) and the own rows of overlays below `magic_0450`.

use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

fn put<G: Game>(g: &mut G, a: u32, x: f32) {
    g.write::<u32>(a, x.to_bits());
}

/// Copies `n` words from `from` to `to`, in order.
fn copy_words<G: Game>(g: &mut G, from: u32, to: u32, n: u32) {
    for i in 0..n {
        let x = g.read::<u32>(from + 4 * i);
        g.write::<u32>(to + 4 * i, x);
    }
}

/// Where the code of one of a camera's two point tracks
/// ([`camera_track`]) differs: the operand of the motion slot's table,
/// the operand of the 1.0 the eased move compares with (`None`: the
/// value the first track left in `xmm1`), the `rel32` fields of the four
/// key steps and whether the steady move adds in the second track's
/// operand order.
struct TrackCode {
    slot_op: u32,
    k_op: Option<u32>,
    steps: [u32; 4],
    second: bool,
}

const TRACK_EYE: TrackCode = TrackCode {
    slot_op: 0x78,
    k_op: Some(0x57),
    steps: [0x128, 0x135, 0x142, 0x14f],
    second: false,
};

const TRACK_AIM: TrackCode = TrackCode {
    slot_op: 0x196,
    k_op: None,
    steps: [0x23c, 0x249, 0x256, 0x263],
    second: true,
};

/// One step of a camera's point track: the follower `F` (its state
/// `F + 0x3c`) and the point `P`. State 1 eases the time `F + 0x2c` by
/// its step `F + 0x30`: below 1.0 slot 326 `(P, F + 0x10, F, t)` places
/// the point between the two ends, else the end `F + 0x10` (four words)
/// is the point and the state 0. State 2 moves at a speed `F` with the
/// acceleration `F + 0x10` (`F + 0x34` frames, then state 0). States 3
/// to 6 are the key and path followers
/// ([`key_step`](crate::s3_misc::key_step),
/// [`path_step`](crate::s6_misc::path_step),
/// [`turning_path_step`](crate::s6_misc::turning_path_step),
/// [`eased_turning_path_step`](crate::s6_misc::eased_turning_path_step)).
/// `sp` is the `esp` at the calls; `k_op` the operand of the 1.0 in
/// `xmm1` before the step. Returns the operand of the 1.0 in `xmm1` after.
fn camera_track<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    sp: u32,
    (tf, p): (u32, u32),
    code: &TrackCode,
    k_in: u32,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let state = g.read::<u32>(tf + 0x3c);
    match state {
        1 => {
            let t = add(f32_at(g, tf + 0x30), f32_at(g, tf + 0x2c));
            let k_op = code.k_op.unwrap_or(k_in);
            let k = f32_at(g, w(k_op));
            put(g, tf + 0x2c, t);
            if k > t {
                let target = slot_target(g, w(code.slot_op), 326);
                // call width: 4
                g.call(target, 0, &[p, tf + 0x10, tf, t.to_bits()]);
                0x157
            } else {
                copy_words(g, tf + 0x10, p, 4);
                g.write::<u32>(tf + 0x3c, 0);
                k_op
            }
        }
        2 => {
            let a = add(f32_at(g, tf + 0x14), f32_at(g, tf + 4));
            let b = add(f32_at(g, tf + 0x10), f32_at(g, tf));
            put(g, tf + 4, a);
            let c = if code.second {
                add(f32_at(g, tf + 0x18), f32_at(g, tf + 8))
            } else {
                add(f32_at(g, tf + 8), f32_at(g, tf + 0x18))
            };
            put(g, tf, b);
            put(g, tf + 8, c);
            let x = if code.second {
                add(b, f32_at(g, p))
            } else {
                add(f32_at(g, p), b)
            };
            put(g, p, x);
            let y = add(a, f32_at(g, p + 4));
            put(g, p + 4, y);
            let z = add(c, f32_at(g, p + 8));
            put(g, p + 8, z);
            let left = g.read::<i32>(tf + 0x34).wrapping_sub(1);
            g.write::<i32>(tf + 0x34, left);
            if left <= 0 {
                g.write::<u32>(tf + 0x3c, 0);
            }
            // the first track loaded the 1.0 again (`+0xe4`)
            if code.second { k_in } else { 0xe8 }
        }
        3..=6 => {
            let step = match state {
                3 => code.steps[0],
                4 => code.steps[1],
                5 => code.steps[2],
                _ => code.steps[3],
            };
            let at = img.rel32_target(f + step);
            let fr = Frame {
                at,
                esp: sp.wrapping_sub(4),
                eax: state - 1,
                ecx: tf,
                edx: p,
                xmm: [0; 4],
            };
            match state {
                3 => crate::s3_misc::key_step(g, img, &fr),
                4 => crate::s6_misc::path_step(g, img, &fr),
                5 => crate::s6_misc::turning_path_step(g, img, &fr),
                _ => crate::s6_misc::eased_turning_path_step(g, img, &fr),
            };
            0x157
        }
        _ => 0x157,
    }
}

/// A summon's three cameras, one step each (internal, cdecl, no
/// arguments; a /GS frame, `ebp` = the entry `esp - 4`). For each camera
/// `C` (`*G_CAM`, operand `+0x14`, stride 0x190), while slot 253 says
/// the scene runs:
/// - the eye track (follower `C - 0x18`, point `C - 0x68`) and the aim
///   track (`C - 0x58`, `C - 0x78`) step ([`camera_track`]);
/// - the roll `C - 0x80` (state `C + 0x40`): 2 is an [`ease_angle`]
///   key track (keys `C + 0x28`, count `C + 0x3c`, time `C + 0x2c`, step
///   `C + 0x30`, frames `C + 0x38`; the last key at the end); 1 a blend
///   from `C + 0x2c` to `C + 0x30` over the frames left, brought within
///   a turn (`C + 0x34` at the end);
/// - the field of view `C + 0x78` (state `C + 0x60`): 1 is the time
///   `C + 0x4c` itself (then stepped by `C + 0x50`), 2 an [`ease_sample`]
///   key track (keys `C + 0x44`, count `C + 0x5c`, frames `C + 0x58`), 3
///   the same through [`quartic`](crate::s3_misc::quartic) (weights
///   `C + 0x48`, held below the count less a double);
/// - with the byte `C + 0x80` = 1 the jitter of the object `C - 0x108`
///   ([`jitter`](crate::sa_misc::jitter)).
///
/// Then with `C + 0x7c` = 1 the camera `*G_V` gets the two points (as
/// they are with the byte `C + 0x82` 0, else turned by slot 229 with the
/// object's matrix into the frame's `ebp - 0x14` and `ebp - 0x24`):
/// slot 284 `(V, 1, aim)`, `(V, 0, eye)`, 285 `(V, roll)`, 286 `(V, fov)`.
/// Returns the last camera's `eax`: slot 286's, else the jitter's, the
/// byte less 1, or slot 253's 0.
///
/// [`ease_angle`]: crate::sb_misc::ease_angle
/// [`ease_sample`]: crate::sb_misc::ease_sample
///
/// `magic_0515` 0x10001560 (1500 bytes with its two jump tables; also
/// `magic_0519`, `0611`, `0667`, `0668`).
#[allow(clippy::too_many_lines)] // the original's four tracks, in order
pub fn cameras_0515<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = fr.esp.wrapping_sub(4);
    // sub esp, 0x24; push ebx, esi, edi
    let sp = ebp.wrapping_sub(0x24 + 12);
    let (aim_l, eye_l) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let mut eax = 0;
    let mut c = w(0x14);
    for _ in 0..3 {
        eax = g.call(slot(g, 0x21, 253), 0, &[]).eax;
        if eax != 0 {
            let k = camera_track(g, img, f, sp, (c - 0x18, c - 0x68), &TRACK_EYE, 0);
            camera_track(g, img, f, sp, (c - 0x58, c - 0x78), &TRACK_AIM, k);
            let k_op = roll(g, img, f, sp, c);
            fov(g, img, f, sp, c, k_op);
            let byte = u32::from(g.read::<u8>(c + 0x80));
            eax = if byte == 1 {
                crate::sa_misc::jitter(g, img, img.rel32_target(f + 0x489), c - 0x108)
            } else {
                byte.wrapping_sub(1)
            };
        }
        if g.read::<u32>(c + 0x7c) == 1 {
            if g.read::<u8>(c + 0x82) == 0 {
                copy_words(g, c - 0x78, aim_l, 4);
                copy_words(g, c - 0x68, eye_l, 4);
            } else {
                // call width: 3
                g.call(slot(g, 0x502, 229), 0, &[aim_l, c - 0x108, c - 0x78]);
                // call width: 3
                g.call(slot(g, 0x518, 229), 0, &[eye_l, c - 0x108, c - 0x68]);
            }
            let v = g.read::<u32>(w(0x529));
            // call width: 3
            g.call(slot(g, 0x532, 284), 0, &[v, 1, aim_l]);
            // call width: 3
            g.call(slot(g, 0x546, 284), 0, &[v, 0, eye_l]);
            let r = g.read::<u32>(c - 0x80);
            // call width: 2
            g.call(slot(g, 0x55b, 285), 0, &[v, r]);
            let r = g.read::<u32>(c + 0x78);
            // call width: 2
            eax = g.call(slot(g, 0x576, 286), 0, &[v, r]).eax;
        }
        c += 0x190;
    }
    eax
}

/// [`cameras_0515`]'s roll (state `C + 0x40`, out `C - 0x80`). Returns
/// the operand of the 1.0 the field of view's curve reads (`xmm4`).
fn roll<G: Game>(g: &mut G, img: &Image, f: u32, sp: u32, c: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    match g.read::<u32>(c + 0x40) {
        2 => {
            let left = g.read::<u32>(c + 0x38).wrapping_sub(1);
            g.write::<u32>(c + 0x38, left);
            let keys = g.read::<u32>(c + 0x28);
            if left == 0 {
                let n = g.read::<u32>(c + 0x3c);
                let last = g.read::<u32>(keys.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4));
                g.write::<u32>(c - 0x80, last);
                g.write::<u32>(c + 0x40, 0);
            } else {
                let t = add(f32_at(g, c + 0x30), f32_at(g, c + 0x2c));
                let n = g.read::<u32>(c + 0x3c);
                // push 1: the stride
                g.write::<u32>(sp - 4, 1);
                put(g, c + 0x2c, t);
                let at = img.rel32_target(f + 0x295);
                let x = crate::sb_misc::ease_angle(g, img, at, sp - 8, keys, n, t);
                put(g, c - 0x80, x);
            }
            0x2a5
        }
        1 => {
            let left = g.read::<u32>(c + 0x38).wrapping_sub(1);
            g.write::<u32>(c + 0x38, left);
            if left == 0 {
                let x = g.read::<u32>(c + 0x34);
                g.write::<u32>(c - 0x80, x);
                g.write::<u32>(c + 0x40, 0);
            } else {
                let one = f32_at(g, w(0x3a3));
                let r = div(cvt(left), cvt(g.read::<u32>(c + 0x3c)));
                let to = mul(f32_at(g, c + 0x30), r);
                let v = add(mul(sub(one, r), f32_at(g, c + 0x2c)), to);
                let x = if v > f32_at(g, w(0x3dc)) {
                    sub(v, f32_at(g, w(0x3e6)))
                } else if f32_at(g, w(0x3f8)) > v {
                    add(v, f32_at(g, w(0x405)))
                } else {
                    v
                };
                put(g, c - 0x80, x);
            }
            0x3a3
        }
        _ => 0x2a5,
    }
}

/// [`cameras_0515`]'s field of view (state `C + 0x60`, out `C + 0x78`);
/// `k_op` is the operand of the 1.0 in `xmm4`.
fn fov<G: Game>(g: &mut G, img: &Image, f: u32, sp: u32, c: u32, k_op: u32) {
    let w = |o: u32| img.word(f + o);
    let state = g.read::<u32>(c + 0x60);
    match state {
        1 => {
            let t = g.read::<u32>(c + 0x4c);
            g.write::<u32>(c + 0x78, t);
            let t = add(f32_at(g, c + 0x50), f32_at(g, c + 0x4c));
            put(g, c + 0x4c, t);
        }
        2 | 3 => {
            let left = g.read::<u32>(c + 0x58).wrapping_sub(1);
            g.write::<u32>(c + 0x58, left);
            let keys = g.read::<u32>(c + 0x44);
            if left == 0 {
                let n = g.read::<u32>(c + 0x5c);
                let last = g.read::<u32>(keys.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4));
                g.write::<u32>(c + 0x78, last);
                g.write::<u32>(c + 0x60, 0);
                return;
            }
            let t = add(f32_at(g, c + 0x50), f32_at(g, c + 0x4c));
            let n = g.read::<u32>(c + 0x5c);
            put(g, c + 0x4c, t);
            let (x, call) = if state == 3 {
                let one = f32_at(g, w(k_op));
                let weights = g.read::<u32>(c + 0x48);
                let x = crate::s3_misc::quartic(g, weights, t, one, f32_at(g, w(0x2db)));
                let top = cvt(n);
                let x = if x >= top {
                    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
                    let held = (f64::from(top) - g.read::<f64>(w(0x367))) as f32;
                    held
                } else {
                    x
                };
                (x, 0x375)
            } else {
                (t, 0x442)
            };
            // push 1: the stride
            g.write::<u32>(sp - 4, 1);
            let at = img.rel32_target(f + call);
            let y = crate::sb_misc::ease_sample(g, img, at, sp - 8, keys, n, x);
            put(g, c + 0x78, y);
        }
        _ => {}
    }
}

/// [`cameras_0515`] (the `rel32`s: the eye and aim tracks' four steps,
/// [`ease_angle`](crate::sb_misc::ease_angle), [`ease_sample`](crate::sb_misc::ease_sample)
/// twice, [`jitter`](crate::sa_misc::jitter), `__security_check_cookie`).
pub const CAMERAS_0515: Sig = Sig {
    len: 1500,
    hash: 0x8a9e_9423_889f_212c,
    calls: &[
        (0x128, Callee::Sig(&crate::s3_misc::KEY_STEP)),
        (0x135, Callee::Sig(&crate::s6_misc::PATH_STEP)),
        (0x142, Callee::Sig(&crate::s6_misc::TURNING_PATH_STEP)),
        (0x14f, Callee::Sig(&crate::s6_misc::EASED_TURNING_PATH_STEP)),
        (0x23c, Callee::Sig(&crate::s3_misc::KEY_STEP)),
        (0x249, Callee::Sig(&crate::s6_misc::PATH_STEP)),
        (0x256, Callee::Sig(&crate::s6_misc::TURNING_PATH_STEP)),
        (0x263, Callee::Sig(&crate::s6_misc::EASED_TURNING_PATH_STEP)),
        (0x295, Callee::Sig(&crate::sb_misc::EASE_ANGLE)),
        (0x375, Callee::Sig(&crate::sb_misc::EASE_SAMPLE)),
        (0x442, Callee::Sig(&crate::sb_misc::EASE_SAMPLE)),
        (0x489, Callee::Sig(&crate::sa_misc::JITTER)),
        (0x5a4, Callee::Any),
    ],
    imports: &[],
};

/// Where the code of [`trails_0515`]'s two draw loops differs: the point
/// local (offset in the frame `T`) and the operands of the slots' table,
/// the camera `*G_X` and the lock `*G_LOCK` (taken, put back).
struct TrailDraw {
    v: u32,
    slot270: u32,
    gx: u32,
    slot297: u32,
    slot230: u32,
    lock: [u32; 2],
    slot908: u32,
}

const TRAIL_PLAIN: TrailDraw = TrailDraw {
    v: 0x88,
    slot270: 0x1d6,
    gx: 0x1f5,
    slot297: 0x202,
    slot230: 0x248,
    lock: [0x270, 0x298],
    slot908: 0x282,
};

const TRAIL_STEPPED: TrailDraw = TrailDraw {
    v: 0x78,
    slot270: 0x388,
    gx: 0x3a4,
    slot297: 0x3b1,
    slot230: 0x3f7,
    lock: [0x41f, 0x447],
    slot908: 0x431,
};

/// One copy of a trail record `R` (`e = R + 0xa4`) at the time `x`
/// ([`trails_0515`]'s frame `T`): slot 270 `(V, x, R, R + 0x40, R +
/// 0x84 byte)` places the point `V` (`w` 1.0), the matrix `M = T + 0x98`
/// gets the scale `*(T + 0x1c)` and slot 297 `(M + 0x30, *G_X, V)` its
/// row, slot 230 `(T + 0xd8, M)`; the draw `T + 0x40` takes the colour
/// `*(T + 0x14)`, and below 0x40 copies slot 908 `(eax, edx, model,
/// T + 0x40)` (slot 772's pair) draws it under the lock. The copy count
/// `*(T + 0x18)` goes up, the record's most `*e` with it.
fn trail_copy<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, e: u32, x: f32, d: &TrailDraw) {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let v = t + d.v;
    let byte = u32::from(g.read::<u8>(e - 0x20));
    // call width: 5
    g.call(
        slot(g, d.slot270, 270),
        0,
        &[v, x.to_bits(), e - 0xa4, e - 0x64, byte],
    );
    let scale = g.read::<u32>(t + 0x1c);
    let cam = g.read::<u32>(w(d.gx));
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let m = t + 0x98;
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(m + o, scale);
    }
    // call width: 3
    g.call(slot(g, d.slot297, 297), 0, &[m + 0x30, cam, v]);
    // call width: 2
    g.call(slot(g, d.slot230, 230), 0, &[t + 0xd8, m]);
    let colour = g.read::<u32>(t + 0x14);
    let model = g.read::<u32>(e - 0x24);
    g.write::<u32>(t + 0x44, colour);
    let i = g.read::<i32>(t + 0x18);
    g.write::<u32>(t + 0x34, model);
    if i < 0x40 {
        let l = g.read::<u32>(w(d.lock[0]));
        let saved = g.read::<u32>(l);
        g.write::<u32>(l, 1);
        let (eax, edx) = (g.read::<u32>(t + 0x24), g.read::<u32>(t + 0x30));
        // call width: 4
        g.call(slot(g, d.slot908, 908), 0, &[eax, edx, model, t + 0x40]);
        let l = g.read::<u32>(w(d.lock[1]));
        g.write::<u32>(l, saved);
    }
    let i = g.read::<i32>(t + 0x18).wrapping_add(1);
    g.write::<i32>(t + 0x18, i);
    if i > g.read::<i32>(e) {
        g.write::<i32>(e, i);
    }
}

/// One active trail record of [`trails_0515`] (`e = R + 0xa4`, `T` the
/// frame). Returns `None` when slot 886 refused the buffer (the
/// function ends there with `eax` 0), else the `eax` the record leaves
/// (the lock global's word).
#[allow(clippy::too_many_lines)] // the original's two loops, in order
fn trail_record<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, e: u32) -> Option<u32> {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let count = u32::from(g.read::<u8>(e - 0x1f));
    let scale = g.read::<u32>(e - 0x10);
    g.write::<u32>(t + 0x20, count);
    g.write::<u32>(t + 0x34, 0);
    let colour = g.read::<u32>(e - 0x18);
    g.write::<u32>(t + 0x14, colour);
    let colour_step = g.read::<u32>(e - 0x14);
    g.write::<u32>(t + 0x38, colour_step);
    g.write::<u32>(t + 0x1c, scale);
    let grow = g.read::<u32>(e - 0xc);
    g.write::<u32>(t + 0x28, grow);
    let grow_step = g.read::<u32>(e - 0x8);
    g.write::<u32>(t + 0x3c, grow_step);
    g.write::<u32>(t + 0x18, 0);
    // call width: 2
    let pair = g.call(slot(g, 0xa0, 772), 0, &[e - 0xa4, 0]);
    let l = g.read::<u32>(w(0xc7));
    let saved = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    g.write::<u32>(t + 0x24, pair.eax);
    g.write::<u32>(t + 0x30, pair.edx);
    // call width: 2
    let buf = g.call(slot(g, 0xd6, 773), 0, &[pair.eax, pair.edx]).eax;
    let l = g.read::<u32>(w(0xed));
    g.write::<u32>(l, saved);
    if buf != 0 {
        let model = g.read::<u32>(e - 0x24);
        let n = i32::from(g.read::<i16>(model + 2)).cast_unsigned();
        // call width: 4
        let ok = g
            .call(
                slot(g, 0x110, 886),
                0,
                &[buf, 0, n << 8, n.wrapping_mul(3) << 7],
            )
            .eax;
        if ok == 0 {
            return None;
        }
        let (eax, edx) = (g.read::<u32>(t + 0x24), g.read::<u32>(t + 0x30));
        // call width: 2
        g.call(slot(g, 0x12c, 937), 0, &[eax, edx]);
    }
    let stepped = g.read::<u8>(e - 0x1e) != 0;
    let den = i32::from(g.read::<i16>(e - 0x1a));
    let num = i32::from(g.read::<i16>(e - 0x1c));
    let mut x = div(cvt(num.cast_unsigned()), cvt(den.cast_unsigned()));
    put(g, t + 0x10, x);
    if stepped {
        // the time steps by the float `e - 4` (over the count's word)
        let step = g.read::<u32>(e - 4);
        g.write::<u32>(t + 0x20, step);
        loop {
            if g.read::<f64>(w(0x354)) >= f64::from(x) {
                trail_copy(g, img, f, t, e, x, &TRAIL_STEPPED);
                x = f32_at(g, t + 0x10);
            }
            x = sub(x, f32_at(g, t + 0x20));
            put(g, t + 0x10, x);
            if 0.0 > x {
                break;
            }
            let c = g
                .read::<u32>(t + 0x14)
                .wrapping_sub(g.read::<u32>(t + 0x38));
            g.write::<u32>(t + 0x14, c);
            if c & 0xff00_0000 == 0 {
                break;
            }
            let d = f32_at(g, t + 0x28);
            let s = add(d, f32_at(g, t + 0x1c));
            put(g, t + 0x1c, s);
            let d = add(f32_at(g, t + 0x3c), d);
            put(g, t + 0x28, d);
        }
    } else {
        let count = u32::from(g.read::<u8>(e - 0x1f));
        let step = div(
            f32_at(g, w(0x171)),
            cvt(count.wrapping_mul(den.cast_unsigned())),
        );
        let mut limit_op = 0x183;
        put(g, t + 0x2c, step);
        loop {
            if g.read::<f64>(w(limit_op)) >= f64::from(x) {
                trail_copy(g, img, f, t, e, x, &TRAIL_PLAIN);
                x = f32_at(g, t + 0x10);
                limit_op = 0x2b5;
            }
            let step = f32_at(g, t + 0x2c);
            x = sub(x, step);
            put(g, t + 0x10, x);
            if 0.0 > x {
                break;
            }
            let left = g.read::<u32>(t + 0x20).wrapping_sub(1);
            g.write::<u32>(t + 0x20, left);
            if left != 0 {
                continue;
            }
            let count = u32::from(g.read::<u8>(e - 0x1f));
            g.write::<u32>(t + 0x20, count);
            let c = g
                .read::<u32>(t + 0x14)
                .wrapping_sub(g.read::<u32>(t + 0x38));
            g.write::<u32>(t + 0x14, c);
            if c & 0xff00_0000 == 0 {
                break;
            }
            let d = f32_at(g, t + 0x28);
            if 0.0 > d {
                let s = add(d, f32_at(g, t + 0x1c));
                put(g, t + 0x1c, s);
                if 0.0 >= s {
                    break;
                }
                let d = add(f32_at(g, t + 0x3c), d);
                put(g, t + 0x28, d);
            }
        }
    }
    let model = g.read::<u32>(t + 0x34);
    let edx = g.read::<u32>(t + 0x30);
    g.write::<u32>(e - 0x24, model);
    let l = g.read::<u32>(w(0x4c2));
    let eax = g.read::<u32>(t + 0x24);
    let saved = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    // call width: 2
    g.call(slot(g, 0x4d3, 870), 0, &[eax, edx]);
    let l = g.read::<u32>(w(0x4e0));
    g.write::<u32>(l, saved);
    Some(l)
}

/// A summon's model trails drawn (internal, cdecl, no arguments; a /GS
/// frame aligned to 16, `T` = the aligned `esp - 0x120`). The matrix
/// `M = T + 0x98` is made (slot 235) and the draw `T + 0x40` set up (the
/// u16 4 at `+2`, the buffer `T + 0xd8` at `+0x10` and `+0x1c`). Each
/// record `R` of 0xa8 bytes (from operand `+0x5b` less 0xa4 up to the
/// one at operand `+0x4f1`) with a model `R + 0x80`: slot 772 `(R, 0)`
/// gives a pair whose buffer slot 773 makes under the lock (slot 886
/// sizes it by the model's count `u16 +2`, then slot 937; a refused
/// size ends the function). From the time `R + 0x88 / R + 0x8a` down,
/// copies ([`trail_copy`]) at most the double at operand `+0x183`:
/// - plain (`R + 0x86` 0): by `1 / (count R + 0x85 * R + 0x8a)`, and
///   after each `count` copies the colour `R + 0x8c` loses `R + 0x90`
///   and a shrinking scale (`R + 0x94` by `R + 0x98`, that by
///   `R + 0x9c`) shrinks;
/// - stepped: by the float `R + 0xa0`, the colour and scale changed
///   after every copy;
///
/// until the time is below 0 or the colour's top byte 0 (or the scale
/// 0). Then the record keeps its model only if it drew, and slot 870
/// `(pair)` under the lock ends it. Returns the lock global's word of
/// the last record, slot 235's `eax` with none, or 0 when slot 886
/// refused.
///
/// `magic_0515` 0x10002fa0 (1295 bytes; also `magic_0611`, `0667`,
/// `0668`).
pub fn trails_0515<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -16; sub esp, 0x118: `S`; push esi, edi: `T`
    let s = (fr.esp.wrapping_sub(4) & !15).wrapping_sub(0x118);
    let t = s.wrapping_sub(8);
    g.write::<u32>(s + 0x54, s + 0xd0);
    g.write::<u32>(s + 0x48, s + 0xd0);
    g.write::<u16>(s + 0x3a, 4);
    g.write::<u32>(s + 0x40, 0);
    g.write::<u8>(s + 0x50, 0);
    // call width: 1
    let mut eax = g.call(slot_target(g, w(0x3e), 235), 0, &[t + 0x98]).eax;
    let mut e = w(0x5b);
    while e.cast_signed() < w(0x4f1).cast_signed() {
        if g.read::<u32>(e - 0x24) != 0 {
            match trail_record(g, img, f, t, e) {
                Some(x) => eax = x,
                None => return 0,
            }
        }
        e += 0xa8;
    }
    eax
}

/// [`trails_0515`] (the `rel32`: `__security_check_cookie`).
pub const TRAILS_0515: Sig = Sig {
    len: 1295,
    hash: 0xf22d_8e3d_ba2c_c530,
    calls: &[(0x507, Callee::Any)],
    imports: &[],
};

/// A summon's spark streaks drawn (internal, cdecl, no arguments; a /GS
/// frame aligned to 16 below a saved `ebx`, `A` its `ebp`). The matrix
/// `M = A - 0x58` (slot 235) and slot 298 `(A - 0x98)`; the draw
/// `D = A - 0x110` (u16 4 at `+2`, the buffer `A - 0xd8` at `+0x10`,
/// `A - 0x98` at `+0x1c`). Each of the 0x200 records `R` of 0x3c bytes
/// (from operand `+0xa6` less 0x18) with a model `R + 0x20`: the first
/// one calls slot 315 `(0)`; `R + 0x24` copies between the points `R`
/// and `R + 0x10` from the time `R + 0x28 / R + 0x2c` by `1 / count`
/// (wrapped by the double at operand `+0x36f` above 1), each: the point
/// `V = A - 0x14` (`w` 1.0), `M` scaled by `R + 0x34` with slot 297 `(M +
/// 0x30, *G_X, V)`, slot 230 `(A - 0xd8, M)`, the buffer's `z` and `w`
/// (the copy's plus the constant at operand `+0x1fb`) at `A - 0x60`; slot
/// 772 `(R, i)` gives a pair; slot 773 makes its buffer under the lock,
/// then slot 875 `(pair, model, D)` and 870 `(pair)`; without one slot
/// 776 `(the model's kind byte)` and 874 `(pair, model, D, kind)`. The
/// copy count `i` goes up, the record's most `R + 0x38` with it.
/// Returns 0 (the record count's last value).
///
/// `magic_0515` 0x10003520 (950 bytes; also `magic_0611`, `0667`,
/// `0668`).
pub fn streaks_0515<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebx; sub esp, 8; and esp, -16; add esp, 4; push ebp: `A`
    let a = fr.esp.wrapping_sub(0xc) & !15;
    g.write::<u32>(a - 0x100, a - 0xd8);
    g.write::<u32>(a - 0xf4, a - 0x98);
    g.write::<u16>(a - 0x10e, 4);
    g.write::<u32>(a - 0x118, 0x200);
    g.write::<u32>(a - 0x128, 1);
    g.write::<u64>(a - 0x108, 0);
    g.write::<u8>(a - 0xf8, 0);
    // call width: 1
    g.call(slot(g, 0x51, 235), 0, &[a - 0x58]);
    // call width: 1
    g.call(slot(g, 0x8e, 298), 0, &[a - 0x98]);
    let mut k_op = 0x9e;
    let mut e = w(0xa6);
    for left in (0..0x200u32).rev() {
        if g.read::<u32>(e + 8) != 0 {
            k_op = streak_record(g, img, f, a, e, k_op);
        }
        e += 0x3c;
        g.write::<u32>(a - 0x118, left);
    }
    0
}

/// One active record of [`streaks_0515`] (`e = R + 0x18`); `k_op` is
/// the operand of the 1.0 in `xmm2`. Returns the one after.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn streak_record<G: Game>(g: &mut G, img: &Image, f: u32, a: u32, e: u32, k_op: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let first = g.read::<u32>(a - 0x128) != 0;
    let mut n = g.read::<u32>(e + 0xc);
    g.write::<u32>(a - 0x11c, 0);
    let mut k_op = k_op;
    if first {
        g.write::<u32>(a - 0x128, 0);
        // call width: 1
        g.call(slot(g, 0xd1, 315), 0, &[0]);
        k_op = 0xed;
    }
    let mut t = div(cvt(g.read::<u32>(e + 0x10)), cvt(g.read::<u32>(e + 0x14)));
    let dt = div(f32_at(g, w(k_op)), cvt(n));
    let colour = g.read::<u32>(e + 0x18);
    g.write::<u32>(a - 0x10c, colour);
    put(g, a - 0x120, t);
    put(g, a - 0x134, dt);
    if n == 0 {
        return k_op;
    }
    let (p, q) = (e - 0x18, e - 8);
    let (v, m, d) = (a - 0x14, a - 0x58, a - 0x110);
    loop {
        n = n.wrapping_sub(1);
        g.write::<u32>(a - 0x130, n);
        let u = sub(f32_at(g, w(k_op)), t);
        let x = add(mul(f32_at(g, q), u), mul(f32_at(g, p), t));
        let cam = g.read::<u32>(w(0x16f));
        put(g, v, x);
        let y = add(mul(f32_at(g, p + 4), t), mul(f32_at(g, q + 4), u));
        g.write::<u32>(v + 0xc, 0x3f80_0000);
        put(g, v + 4, y);
        let z = add(mul(f32_at(g, p + 8), t), mul(f32_at(g, q + 8), u));
        let scale = g.read::<u32>(e + 0x1c);
        for o in [0x28u32, 0x14, 0] {
            g.write::<u32>(m + o, scale);
        }
        put(g, v + 8, z);
        // call width: 3
        g.call(slot(g, 0x18a, 297), 0, &[m + 0x30, cam, v]);
        // call width: 2
        g.call(slot(g, 0x1e3, 230), 0, &[a - 0xd8, m]);
        let bz = add(f32_at(g, a - 0xa0), f32_at(g, w(0x1fb)));
        let i = g.read::<u32>(a - 0x11c);
        g.write::<u32>(a - 0xf4, 0);
        put(g, a - 0x60, bz);
        let bw = g.read::<u32>(a - 0x9c);
        g.write::<u32>(a - 0x5c, bw);
        // call width: 2
        let pair = g.call(slot(g, 0x200, 772), 0, &[p, i]);
        let l = g.read::<u32>(w(0x231));
        let saved = g.read::<u32>(l);
        g.write::<u32>(l, 1);
        g.write::<u32>(a - 0x114, pair.edx);
        g.write::<u32>(a - 0x12c, pair.eax);
        g.write::<u32>(a - 0x124, saved);
        // call width: 2
        let buf = g.call(slot(g, 0x243, 773), 0, &[pair.eax, pair.edx]).eax;
        let l = g.read::<u32>(w(0x265));
        let saved = g.read::<u32>(a - 0x124);
        g.write::<u32>(l, saved);
        if buf != 0 {
            let model = g.read::<u32>(e + 8);
            let edx = g.read::<u32>(a - 0x114);
            g.write::<u32>(l, 1);
            // call width: 4
            g.call(slot(g, 0x284, 875), 0, &[pair.eax, edx, model, d]);
            let l = g.read::<u32>(w(0x29e));
            let edx = g.read::<u32>(a - 0x114);
            g.write::<u32>(l, 1);
            // call width: 2
            g.call(slot(g, 0x2af, 870), 0, &[pair.eax, edx]);
            let l = g.read::<u32>(w(0x2bd));
            let saved = g.read::<u32>(a - 0x124);
            g.write::<u32>(l, saved);
        } else {
            let model = g.read::<u32>(e + 8);
            let parts = g.read::<u32>(model + 4);
            let kind = u32::from(g.read::<u8>(parts.wrapping_add(model).wrapping_add(4)));
            // call width: 1
            let r = g.call(slot(g, 0x2db, 776), 0, &[kind]).eax;
            let model = g.read::<u32>(e + 8);
            let l = g.read::<u32>(w(0x2ec));
            let saved = g.read::<u32>(l);
            let (edx, eax) = (g.read::<u32>(a - 0x114), g.read::<u32>(a - 0x12c));
            g.write::<u32>(l, 1);
            // call width: 5
            g.call(slot(g, 0x2fe, 874), 0, &[eax, edx, model, d, r]);
            let l = g.read::<u32>(w(0x31e));
            g.write::<u32>(l, saved);
        }
        let i = g.read::<i32>(a - 0x11c).wrapping_add(1);
        g.write::<i32>(a - 0x11c, i);
        if i > g.read::<i32>(e + 0x20) {
            g.write::<i32>(e + 0x20, i);
        }
        t = add(f32_at(g, a - 0x134), f32_at(g, a - 0x120));
        k_op = 0x350;
        put(g, a - 0x120, t);
        if t > f32_at(g, w(k_op)) {
            #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
            let wrapped = (f64::from(t) - g.read::<f64>(w(0x36f))) as f32;
            t = wrapped;
            put(g, a - 0x120, t);
        }
        n = g.read::<u32>(a - 0x130);
        if n == 0 {
            return k_op;
        }
    }
}

/// [`streaks_0515`] (the `rel32`: `__security_check_cookie`).
pub const STREAKS_0515: Sig = Sig {
    len: 950,
    hash: 0xee3d_b689_9a52_935e,
    calls: &[(0x3ab, Callee::Any)],
    imports: &[],
};

/// A summon's draw callback (cdecl, a task callback; `esp` aligned to
/// 8): `**G_A = *G_B`, the part refresh of the object at operand `+0x14`
/// ([`part_refresh_b`](crate::sa_misc::part_refresh_b)); unless the
/// pause word `**G_P` is set (`*G_P` non-zero) the trails
/// ([`trails_0515`]) and the streaks ([`streaks_0515`]) are drawn.
/// Returns the streaks' 0, or `*G_P`.
///
/// `magic_0515` 0x100041f0 (57 bytes; also `magic_0611`, `0667`,
/// `0668`).
pub fn draw_0515<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let to = g.read::<u32>(w(0x8));
    let x = g.read::<u32>(w(0xd));
    g.write::<u32>(to, x);
    crate::sa_misc::part_refresh_b(g, img, img.rel32_target(f + 0x19), w(0x14), x);
    let pause = g.read::<u32>(w(0x1e));
    if pause != 0 && g.read::<u32>(pause) != 0 {
        return Ret::eax(pause);
    }
    // push ebp; and esp, -8; the calls' return address below
    let sp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(4);
    let at = |o: u32| Frame {
        at: img.rel32_target(f + o),
        esp: sp,
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    trails_0515(g, img, &at(0x2c));
    Ret::eax(streaks_0515(g, img, &at(0x31)))
}

/// [`draw_0515`].
pub const DRAW_0515: Sig = Sig {
    len: 57,
    hash: 0x6c46_4263_7911_014d,
    calls: &[
        (0x19, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x2c, Callee::Sig(&TRAILS_0515)),
        (0x31, Callee::Sig(&STREAKS_0515)),
    ],
    imports: &[],
};

/// A summon's start-up (cdecl `(actor)`): `*G_ACT = actor`, the scene
/// flags from six calls of slot 280 (the battle scene against
/// 0x2350000, 0x2360000, 0x2370000, 0x2470000, 0x24a0000, 0x18b0046);
/// the summon `S` (slot 168 `(actor)`, the first target record's word)
/// and its two neighbours `S + 1`, `S + 2` (or `S - 1`, `S - 2` when slot
/// 293 `(S)` says so). The three records (0xcc bytes from operand
/// `+0x118` less 0x44) are measured
/// ([`target_measure_modes`](crate::s5_misc::target_measure_modes)) and
/// their points `+0x3c`/`+0x44` averaged into the centre `C` (`y` from
/// slot 289 `(S)`, `w` 1.0; the first record's own with the first
/// scene flag); slot 325 `(at operand +0x1df, C)`. The party (slot 218
/// `(actor, &count)`, one record each from operand `+0x20d`), the enemy
/// leader (slot 294 `(S)`, 0xff when slot 293 says so) and every other
/// target of slot 288 `(0x10, &actor)` not in a record yet (into the
/// records at operand `+0x319`, counted at operand `+0x320`) are
/// measured too. Then the four texture banks (slots 718 and 222 / 221),
/// the eleven parts of the last bank (slot 719 `(bank, 6, k)`) and the
/// fifth one of `(bank, 5, 0)`, a row of u16 counters cleared (one
/// `0x7fff`), slot 291 `(actor, the summon, 0, 0, 0)`, five times the
/// party count, another row cleared, the camera blocks
/// ([`camera_blocks_reset`](crate::s6_misc::camera_blocks_reset)), the
/// first camera block on, the 0x100 sparks, the trail records and the
/// streak records cleared, and the draw task (slot 312 `(the draw
/// callback, 0, 0, 0)`, into operand `+0x788`). Returns 0.
///
/// `magic_0515` 0x10004230 (1938 bytes; also `magic_0611`).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0515<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let actor = fr.arg(g, 0);
    g.write::<u32>(w(0x8), actor);
    g.write::<u32>(w(0x13), 0);
    for (op, scene, flag) in [
        (0xd, 0x0235_0000u32, 0x3a),
        (0x2b, 0x0236_0000, 0x57),
        (0x48, 0x0237_0000, 0x74),
        (0x65, 0x0247_0000, 0x91),
        (0x82, 0x024a_0000, 0xae),
        (0x9f, 0x018b_0046, 0xd1),
    ] {
        // call width: 0
        let r = g.call(slot(g, op, 280), 0, &[]).eax;
        g.write::<u32>(w(flag), u32::from(r == scene));
    }
    let actor = g.read::<u32>(w(0xb6));
    // call width: 1
    let summon = g.call(slot(g, 0xc2, 168), 0, &[actor]).eax;
    g.write::<u32>(w(0xd8), summon);
    // call width: 1
    let flip = g.call(slot(g, 0xde, 293), 0, &[summon]).eax;
    let summon = g.read::<u32>(w(0xec));
    let second = if flip == 0 {
        g.write::<u32>(w(0xfb), summon.wrapping_add(1));
        summon.wrapping_add(2)
    } else {
        g.write::<u32>(w(0x108), summon.wrapping_sub(1));
        summon.wrapping_sub(2)
    };
    g.write::<u32>(w(0x112), second);
    // push ebp, ecx, ebx, esi, edi: the callees' return address below
    let sp = fr.esp.wrapping_sub(0x18);
    let measure = |g: &mut G, rec: u32| {
        let fr = Frame {
            at: img.rel32_target(f + 0x124),
            esp: sp,
            eax: 0,
            ecx: rec,
            edx: 0,
            xmm: [0; 4],
        };
        crate::s5_misc::target_measure_modes(g, img, &fr);
    };
    // the three records' points summed, then averaged
    let mut rec = w(0x118);
    let (x, z) = loop {
        measure(g, rec - 0x44);
        let x = f32_at(g, rec - 8);
        let z = add(f32_at(g, w(0x131)), f32_at(g, rec));
        let x = add(x, f32_at(g, w(0x13d)));
        rec += 0xcc;
        put(g, w(0x14b), z);
        put(g, w(0x153), x);
        if rec.cast_signed() >= w(0x159).cast_signed() {
            break (x, z);
        }
    };
    let x = div(x, f32_at(g, w(0x163)));
    let z = div(z, f32_at(g, w(0x16b)));
    let summon = g.read::<u32>(w(0x176));
    put(g, w(0x17e), x);
    put(g, w(0x186), z);
    // call width: 1
    let y = crate::m0018::st0_f32(g.call(slot(g, 0x170, 289), 0, &[summon]).st0);
    put(g, w(0x19e), y);
    g.write::<u32>(w(0x1a4), 0x3f80_0000);
    if g.read::<u32>(w(0x197)) != 0 {
        let x = g.read::<u32>(w(0x1b2));
        g.write::<u32>(w(0x1ba), x);
        let z = g.read::<u32>(w(0x1c2));
        g.write::<u32>(w(0x1ca), z);
    }
    // call width: 2
    g.call(slot(g, 0x1cf, 325), 0, &[w(0x1df), w(0x1d4)]);
    let actor = g.read::<u32>(w(0x1f1));
    // call width: 2
    let party = g.call(slot(g, 0x1e6, 218), 0, &[actor, w(0x1eb)]).eax;
    if g.read::<i32>(w(0x206)) > 0 {
        let (mut i, mut rec) = (0u32, w(0x20d));
        loop {
            let t = u32::from(g.read::<u8>(party.wrapping_add(i)));
            g.write::<u32>(rec, t);
            measure(g, rec);
            i += 1;
            rec += 0xcc;
            if i.cast_signed() >= g.read::<i32>(w(0x227)) {
                break;
            }
        }
    }
    let summon = g.read::<u32>(w(0x234));
    // call width: 1
    let flip = g.call(slot(g, 0x22e, 293), 0, &[summon]).eax;
    if flip == 0 {
        let summon = g.read::<u32>(w(0x24e));
        // call width: 1
        let leader = g.call(slot(g, 0x248, 294), 0, &[summon]).eax;
        g.write::<u32>(w(0x25e), leader);
    } else {
        g.write::<u32>(w(0x266), 0xff);
    }
    measure(g, w(0x26f));
    // slot 288 writes the target count over the actor's word
    let n_at = fr.esp + 4;
    // call width: 2
    let list = g.call(slot(g, 0x27d, 288), 0, &[0x10, n_at]).eax;
    let list_at = fr.esp - 8;
    g.write::<u32>(list_at, list);
    g.write::<u32>(w(0x297), 0);
    if g.read::<i32>(n_at) > 0 {
        let mut more = 0u32;
        let mut party = g.read::<u32>(w(0x2a6));
        let mut i = 0u32;
        loop {
            let list = g.read::<u32>(list_at);
            let t = u32::from(g.read::<u8>(list.wrapping_add(i)));
            let known = (0..3u32).any(|j| g.read::<u32>(w(0x2ba) + 0xcc * j) == t)
                || (0..party).any(|j| g.read::<u32>(w(0x2d2).wrapping_add(0xcc * j)) == t)
                || g.read::<u32>(w(0x2f5)) == t;
            if !known {
                let rec = more.wrapping_mul(0xcc).wrapping_add(w(0x319));
                g.write::<u32>(w(0x320), more + 1);
                g.write::<u32>(rec, t);
                measure(g, rec);
                party = g.read::<u32>(w(0x32d));
                more = g.read::<u32>(w(0x333));
            }
            i += 1;
            if i.cast_signed() >= g.read::<i32>(n_at) {
                break;
            }
        }
    }
    // the texture banks: (the table's operand, the bank's, a store)
    for (gi, bank, store) in [
        (0x345u32, 0x34a, None),
        (0x372, 0x377, None),
        (0x39f, 0x3a4, Some(0x3b0)),
        (0x3bb, 0x3c0, Some(0x3cc)),
    ] {
        if let Some(op) = store {
            g.write::<u32>(w(op), w(op + 4));
        }
        // call width: 1
        g.call(slot(g, gi, 718), 0, &[w(bank)]);
        if gi == 0x345 || gi == 0x372 {
            let (gi, bank) = if gi == 0x345 {
                (0x357, 0x36b)
            } else {
                (0x384, 0x398)
            };
            // call width: 4
            g.call(slot(g, gi, 222), 0, &[w(bank), 0, 0xa, 0x100]);
        }
    }
    // the last bank's parts: (the table's operand, the bank's, the store's)
    let last_bank: [(u32, u32, u32); 11] = [
        (0x3d7, 0x3e7, 0x3ee),
        (0x3f3, 0x403, 0x40d),
        (0x412, 0x422, 0x429),
        (0x42e, 0x43e, 0x445),
        (0x44a, 0x45a, 0x461),
        (0x466, 0x476, 0x47d),
        (0x482, 0x492, 0x499),
        (0x49e, 0x4ae, 0x4b8),
        (0x4bd, 0x4cd, 0x4d4),
        (0x4d9, 0x4e9, 0x4f0),
        (0x4f5, 0x505, 0x511),
    ];
    for (k, (gi, bank, store)) in (0u32..).zip(last_bank) {
        let b = g.read::<u32>(w(bank));
        // call width: 3
        let p = g.call(slot(g, gi, 719), 0, &[b, 6, k]).eax;
        g.write::<u32>(w(store), p);
    }
    let b = g.read::<u32>(w(0x526));
    // call width: 4
    g.call(slot(g, 0x516, 221), 0, &[b, 0, 0xa, 0x100]);
    let b = g.read::<u32>(w(0x53d));
    // call width: 3
    let p = g.call(slot(g, 0x52d, 719), 0, &[b, 5, 0]).eax;
    g.write::<u32>(w(0x544), p);
    for op in (0x54fu32..=0x60f).step_by(6) {
        g.write::<u16>(w(op), 0);
    }
    g.write::<u16>(w(0x61a), 0x7fff);
    let first = g.read::<u32>(w(0x631));
    let actor = g.read::<u32>(w(0x637));
    // call width: 5
    g.call(slot(g, 0x61f, 291), 0, &[actor, first, 0, 0, 0]);
    let party = g.read::<u32>(w(0x63e));
    #[allow(clippy::cast_possible_truncation)] // the u16 store
    g.write::<u16>(w(0x64a), party.wrapping_mul(5) as u16);
    for op in (0x652u32..=0x6be).step_by(6) {
        g.write::<u16>(w(op), 0);
    }
    g.write::<u32>(w(0x6c3), 0);
    g.write::<u32>(w(0x6c8), 0);
    let cams = Frame {
        at: img.rel32_target(f + 0x6cd),
        esp: sp,
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s6_misc::camera_blocks_reset(g, img, &cams);
    let i = g.read::<u32>(w(0x6d2));
    g.write::<u32>(i.wrapping_mul(0x190).wrapping_add(w(0x6df)), 0);
    g.write::<u32>(w(0x6ea), 1);
    g.write::<u32>(w(0x6f4), 0);
    for k in 0..0x100u32 {
        g.write::<u32>(w(0x702) + 0x4c * k, 0);
    }
    let mut a = w(0x71d);
    loop {
        g.write::<u32>(a - 0x24, 0);
        g.write::<u32>(a, 0);
        a += 0xa8;
        if a.cast_signed() >= w(0x734).cast_signed() {
            break;
        }
    }
    for k in 0..0x200u32 {
        let a = w(0x740) + 0x3c * k;
        g.write::<u32>(a - 0x18, 0);
        g.write::<u32>(a, 0);
    }
    g.write::<u32>(w(0x778), 0);
    g.write::<u32>(w(0x77e), 0);
    // call width: 4
    let task = g.call(slot(g, 0x764, 312), 0, &[w(0x772), 0, 0, 0]).eax;
    g.write::<u32>(w(0x788), task);
    Ret::eax(0)
}

/// [`start_up_0515`] (the `rel32`s: the target measure five times, the
/// camera blocks reset).
pub const START_UP_0515: Sig = Sig {
    len: 1938,
    hash: 0x5165_1b34_9e4c_1c1b,
    calls: &[
        (0x124, Callee::Sig(&crate::s5_misc::TARGET_MEASURE_MODES)),
        (0x21a, Callee::Sig(&crate::s5_misc::TARGET_MEASURE_MODES)),
        (0x274, Callee::Sig(&crate::s5_misc::TARGET_MEASURE_MODES)),
        (0x327, Callee::Sig(&crate::s5_misc::TARGET_MEASURE_MODES)),
        (0x6cd, Callee::Sig(&crate::s6_misc::CAMERA_BLOCKS_RESET)),
    ],
    imports: &[],
};
