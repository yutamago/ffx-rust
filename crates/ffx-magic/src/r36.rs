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

/// `magic_0165`'s summon glow drawn (cdecl, no arguments, reached by a
/// tail jump; a /GS frame aligned to 16, `T` = the aligned
/// `esp - 0x220`). With `O = *G_O`, the fade count `O + 0x5f18` counts
/// down (while positive); slots 357 `(V, -1, 0x3c00)`, 356
/// `(V, -1, 0x2ee0)`, 355 and 354 `(V, -1)` set the view `*G_V`'s ranges
/// (each call seeing the words the earlier ones left). The draw
/// `T + 0x10` (kind 0x5c, slot 562's handle, flags 0x8044, the colour
/// 0x80808080, the two fade bytes `(count + 20) * 64 / 20` and
/// `(20 - count) * 64 / 20`) gets the matrices: slots 723 `(T + 0x1d8)`,
/// 724 `(T + 0x198)`, 230 `(T + 0x158, O + 0x980)`, 231 twice into
/// `T + 0x118`; the colour `(0.5, 0.5, 0.5, 1)` at `T + 0x98` and an
/// identity below it; slot 237 `(V, 0x8a, 0, T + 0x88)` the bone point,
/// which becomes the first row of the sprite matrix `T + 0xd8` (`w` 20,
/// identity after); slot 567 `(*G_C, T + 0x10, ...)` draws it. Then
/// slots 357 and 356 `(V, -1, -1)` reset the ranges. Returns slot 356's
/// `eax`.
///
/// `magic_0165` 0x100025b0 (1019 bytes).
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn glow_0165<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -16; sub esp, 0x218; push esi, edi
    let t = (fr.esp.wrapping_sub(4) & !15).wrapping_sub(0x220);
    let o = g.read::<u32>(w(0x1e));
    let n = g.read::<i32>(o + 0x5f18);
    if n > 0 {
        g.write::<i32>(o + 0x5f18, n - 1);
    }
    let v1 = g.read::<u32>(w(0x3a));
    // call width: 3
    g.call(slot(g, 0x34, 357), 0, &[v1, u32::MAX, 0x3c00]);
    // call width: 6 (the earlier call's words still pushed)
    g.call(
        slot(g, 0x4f, 356),
        0,
        &[v1, u32::MAX, 0x2ee0, v1, u32::MAX, 0x3c00],
    );
    let v2 = g.read::<u32>(w(0x6a));
    let pushed = [
        v2,
        u32::MAX,
        v2,
        u32::MAX,
        v1,
        u32::MAX,
        0x2ee0,
        v1,
        u32::MAX,
        0x3c00,
    ];
    // call width: 8 (the earlier calls' words still pushed)
    g.call(slot(g, 0x64, 355), 0, &pushed[2..]);
    // call width: 10 (the earlier calls' words still pushed)
    g.call(slot(g, 0x7a, 354), 0, &pushed);
    g.write::<u32>(t + 0x10, 0x5c);
    // call width: 10 (no arguments; the earlier calls' words still pushed)
    let h = g.call(slot(g, 0x8a, 562), 0, &pushed).eax;
    let n = g.read::<i32>(o + 0x5f18);
    g.write::<u32>(t + 0x14, h);
    for at in [
        0x1cu32, 0x20, 0x24, 0x2c, 0x44, 0x48, 0x4c, 0x50, 0x54, 0x58,
    ] {
        g.write::<u32>(t + at, 0);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the byte stores
    {
        g.write::<u8>(t + 0x5c, ((n.wrapping_add(0x14) << 6) / 20) as u8);
        g.write::<u8>(t + 0x5d, ((0x14i32.wrapping_sub(n) << 6) / 20) as u8);
    }
    g.write::<u16>(t + 0x18, 0x8044);
    g.write::<u8>(t + 0x1a, 0);
    g.write::<u32>(t + 0x28, 0x8080_8080);
    // call width: 1
    g.call(slot(g, 0x11e, 723), 0, &[t + 0x1d8]);
    // call width: 1
    g.call(slot(g, 0x147, 724), 0, &[t + 0x198]);
    // call width: 2
    g.call(slot(g, 0x163, 230), 0, &[t + 0x158, o + 0x980]);
    // call width: 3
    g.call(slot(g, 0x188, 231), 0, &[t + 0x118, t + 0x198, t + 0x158]);
    // call width: 3
    g.call(slot(g, 0x1b0, 231), 0, &[t + 0x118, t + 0x1d8, t + 0x118]);
    let v = g.read::<u32>(w(0x1d2));
    // the colour, then an identity's last three rows
    let one = 0x3f80_0000;
    let half = 0x3f00_0000;
    for (i, x) in (0u32..).zip([
        half, half, half, one, 0, 0, 0, one, 0, 0, 0, one, 0, 0, 0, one,
    ]) {
        g.write::<u32>(t + 0x98 + 4 * i, x);
    }
    // call width: 4
    g.call(slot(g, 0x1c5, 237), 0, &[v, 0x8a, 0, t + 0x88]);
    let x = g.read::<u32>(t + 0x88);
    for (at, to) in [
        (0x30u32, 0x118u32),
        (0x34, 0x98),
        (0x38, 0xd8),
        (0x3c, 0x158),
    ] {
        g.write::<u32>(t + at, t + to);
    }
    g.write::<u32>(t + 0xd8, x);
    let y = g.read::<u32>(t + 0x8c);
    let cam = g.read::<u32>(w(0x2f9));
    g.write::<u32>(t + 0xdc, y);
    let z = g.read::<u32>(t + 0x90);
    g.write::<u32>(t + 0x94, one);
    g.write::<u32>(t + 0xe0, z);
    // `w` 20, then an identity's last three rows
    for (i, x) in (0u32..).zip([0x41a0_0000, 0, 0, 0, one, 0, 0, 0, one, 0, 0, 0, one]) {
        g.write::<u32>(t + 0xe4 + 4 * i, x);
    }
    // call width: 5
    g.call(
        slot(g, 0x2f3, 567),
        0,
        &[cam, t + 0x10, t + 0x158, t + 0x198, t + 0x1d8],
    );
    let v = g.read::<u32>(w(0x3c1));
    // call width: 3
    g.call(slot(g, 0x3bb, 357), 0, &[v, u32::MAX, u32::MAX]);
    // call width: 3
    Ret::eax(g.call(slot(g, 0x3d3, 356), 0, &[v, u32::MAX, u32::MAX]).eax)
}

/// [`glow_0165`] (the `rel32`: `__security_check_cookie`).
pub const GLOW_0165: Sig = Sig {
    len: 1019,
    hash: 0x2c77_a759_a9ee_6818,
    calls: &[(0x3f3, Callee::Any)],
    imports: &[],
};

/// `magic_0165`'s spark task (cdecl `(task)`, a task callback; a /GS
/// frame aligned to 8, `T` = the aligned `esp - 0x68`). With `O = *G_O`:
/// the wait `O + 0x5f0c` counts down; once it is out, the steps `O +
/// 0x5edc` go down by 2 while above 1, and at 1 or less the task ends:
/// slot pairs ([`slot_pair_loop`](crate::o4_more::slot_pair_loop) of
/// `O + 0x5cb8`), the parts registered
/// ([`register_parts`](crate::sa_misc::register_parts)) and the task
/// given back ([`give_back`](crate::sa_pool::give_back)). Otherwise the
/// trail `O + 0x5cb8` is drawn ([`trail_draw`](crate::s4_misc::trail_draw))
/// on a matrix `M = T + 0x20` scaled 0.03 at the bone point `T + 0x10`
/// (slot 237 `(*G_V, 0x8a, 0, T + 0x10)`, `w` 1.0), and while the spark
/// count `O + 0x5f10` lasts a spark is taken
/// ([`pool_take`](crate::sb_misc::pool_take)) there: its handle from
/// `*G_X + 4`, kind 1, the point jittered by slot 314 `(1.0)` per axis, a
/// speed of three slot 314 `(0.1)`, a spin slot 314 `(pi)`, the speed
/// scaled by the constant at operand `+0x205` into `+0x20`, `w` 1.0,
/// slot 314 `(0.2)` plus the constant at `+0x25c` (scaled) into `+0x3c`,
/// the life 0x13. Returns the last slot's `eax`, the count, 0 (no
/// spark) or the give-back's.
///
/// `magic_0165` 0x10002a00 (695 bytes).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn spark_task_0165<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -8; sub esp, 0x60; push esi, edi
    let t = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x68);
    let o = g.read::<u32>(w(0x17));
    let wait = g.read::<i32>(o + 0x5f0c);
    let task = fr.arg(g, 0);
    if wait > 0 {
        g.write::<i32>(o + 0x5f0c, wait - 1);
    }
    if g.read::<i32>(o + 0x5f0c) <= 0 {
        let steps = g.read::<i32>(o + 0x5edc);
        if steps <= 1 {
            let at = img.rel32_target(f + 0x296);
            let pairs = Frame {
                at,
                esp: t - 4,
                eax: steps.cast_unsigned(),
                ecx: o + 0x5cb8,
                edx: 0,
                xmm: [0; 4],
            };
            let r = crate::o4_more::slot_pair_loop(g, img, &pairs).eax;
            crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x29b), r);
            let r = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x2a2), task);
            return Ret::eax(r);
        }
        g.write::<i32>(o + 0x5edc, steps - 2);
    }
    let (m, p) = (t + 0x20, t + 0x10);
    // call width: 1
    g.call(slot(g, 0x57, 235), 0, &[m]);
    let v = g.read::<u32>(w(0x79));
    let k = 0x3cf5_c28f;
    for at in [0x28u32, 0x14, 0] {
        g.write::<u32>(m + at, k);
    }
    // call width: 4
    g.call(slot(g, 0x6c, 237), 0, &[v, 0x8a, 0, p]);
    // push p: the trail draw's argument
    let sp = t - 0x14;
    g.write::<u32>(sp, p);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let draw = Frame {
        at: img.rel32_target(f + 0xb5),
        esp: sp - 4,
        eax: p,
        ecx: o + 0x5cb8,
        edx: m,
        xmm: [0; 4],
    };
    crate::s4_misc::trail_draw(g, img, &draw);
    let left = g.read::<i32>(o + 0x5f10);
    if left <= 0 {
        return Ret::eax(left.cast_unsigned());
    }
    g.write::<i32>(o + 0x5f10, left - 1);
    let s = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0xd2));
    if s == 0 {
        return Ret::eax(0);
    }
    let x = g.read::<u32>(w(0xe2));
    let handle = g.read::<u32>(x + 4);
    g.write::<u16>(s + 0x38, 0xffff);
    g.write::<u32>(s + 0x30, handle);
    g.write::<u32>(s + 0x34, 1);
    g.write::<u16>(s + 0x3a, 0x200);
    for i in 0..4 {
        let c = g.read::<u32>(p + 4 * i);
        g.write::<u32>(s + 4 * i, c);
    }
    // the pushed word: slot 314's argument; its ST0 into `T + 0xc`
    let arg = t - 4;
    let mut eax = 0;
    let mut spread = |g: &mut G, op: u32, x: u32| {
        g.write::<u32>(arg, x);
        // call width: 1
        let r = g.call(slot(g, op, 314), 0, &[x]);
        eax = r.eax;
        crate::m0018::st0_f32(r.st0)
    };
    for (i, op) in (0u32..).zip([0x10au32, 0x152, 0x179]) {
        let r = spread(g, op, 0x3f80_0000);
        put(g, t + 0xc, r);
        let c = add(f32_at(g, s + 4 * i), f32_at(g, t + 0xc));
        put(g, s + 4 * i, c);
    }
    for (i, op) in (0u32..).zip([0x1a1u32, 0x1bd, 0x1d4]) {
        let r = spread(g, op, 0x3dcc_cccd);
        put(g, s + 0x10 + 4 * i, r);
    }
    let r = spread(g, 0x1eb, 0x4049_0fdb);
    put(g, s + 0x1c, r);
    let k = f32_at(g, w(0x205));
    let sx = mul(f32_at(g, s + 0x10), k);
    g.write::<u32>(s + 0x2c, 0x3f80_0000);
    g.write::<u32>(arg, 0x3e4c_cccd);
    put(g, s + 0x20, sx);
    let sy = mul(f32_at(g, s + 0x14), k);
    put(g, s + 0x24, sy);
    let sz = mul(f32_at(g, s + 0x18), k);
    put(g, s + 0x28, sz);
    let r = spread(g, 0x20f, 0x3e4c_cccd);
    put(g, t + 0xc, r);
    let life = mul(
        add(f32_at(g, t + 0xc), f32_at(g, w(0x25c))),
        f32_at(g, w(0x267)),
    );
    put(g, s + 0x3c, life);
    g.write::<u32>(s + 0x4c, 0x13);
    g.write::<u32>(s + 0x50, 0);
    Ret::eax(eax)
}

/// [`spark_task_0165`] (the `rel32`s: the trail draw, the pool take,
/// `__security_check_cookie` twice, the slot pairs, the parts, the
/// give-back).
pub const SPARK_TASK_0165: Sig = Sig {
    len: 695,
    hash: 0x0fd2_3463_2487_2a6b,
    calls: &[
        (0xb5, Callee::Sig(&crate::s4_misc::TRAIL_DRAW_0009)),
        (0xd2, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x287, Callee::Any),
        (
            0x296,
            Callee::Sig(&crate::o4_more::SLOT_PAIR_LOOP_0165_5490),
        ),
        (0x29b, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (0x2a2, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x2af, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0165`'s summon set-up's objects (cdecl, no arguments): with
/// `O = *G_O`, the object `O + 0x260` leaves the list at operand `+0x12`
/// ([`list_remove`](crate::l5_objhost::list_remove); when it was there,
/// `**G_P` = the operand `+0x27`, slot 713 `(O + 0x260)` and its flag
/// bit 15 cleared); the objects `O + 0x390` (index 3, the scale at
/// operand `+0x4b`) and `O + 0x5f0` (5, `+0x6b`) start
/// ([`start_object_plain`](crate::s4_misc::start_object_plain)) at `O +
/// 0x9e0` lowered by the constant at `+0xa5`, with `O + 0x5ee8` and a
/// `-5.0`; the model object `O + 0x5f1c` (0x130 bytes from `O`) loses
/// flag bit 0; the record `O + 0x5820` (colours, sizes 0x18 and 0x48) and
/// `O + 0x5a68` (colour 0x30808080, kind 4, the steps 0.05 and 0x32, 8)
/// get tasks ([`take`](crate::sa_pool::take) with the callbacks at
/// operands `+0x15a` and `+0x233`) appended to the task list (`G_HEAD`,
/// `G_TAIL`, self-linked). Returns the second task.
///
/// `magic_0165` 0x10003080 (656 bytes).
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn objects_0165<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let o = g.read::<u32>(w(0x3));
    let first = o + 0x260;
    let was = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x17), w(0x12), first);
    if was != 0 {
        let p = g.read::<u32>(w(0x20));
        g.write::<u32>(p, w(0x27));
        // call width: 1
        g.call(slot_target(g, w(0x2c), 713), 0, &[first]);
        let fl = g.read::<u16>(first + 0x124);
        g.write::<u16>(first + 0x124, fl & 0x7fff);
    }
    for (obj, index, scale, call, low) in [
        (o + 0x390, 3u32, 0x4bu32, 0x5du32, 0xa5u32),
        (o + 0x5f0, 5, 0x6b, 0xd5, 0x11b),
    ] {
        let k = f32_at(g, w(scale));
        crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + call), obj, k, index);
        copy_words(g, o + 0x9e0, obj + 0xd0, 4);
        let y = sub(f32_at(g, obj + 0xd4), f32_at(g, w(low)));
        let x = g.read::<u32>(o + 0x5ee8);
        g.write::<u32>(obj + 0xc4, x);
        g.write::<u32>(obj + 0xe8, 0xc0a0_0000);
        put(g, obj + 0xd4, y);
    }
    let i = g.read::<u32>(o + 0x5f1c);
    let at = i.wrapping_mul(0x130).wrapping_add(o).wrapping_add(0x124);
    let fl = g.read::<u16>(at);
    g.write::<u16>(at, fl & 0xfffe);
    let r = o + 0x5820;
    g.write::<u32>(r + 0x81, 0xff80_ffff);
    g.write::<u8>(r + 0x80, 0xff);
    g.write::<u16>(r + 0x85, 0xffff);
    g.write::<u8>(r + 0x87, 0);
    g.write::<u32>(r + 0x88, 0);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, 0x18);
    g.write::<u32>(r + 0x8c, 0x18);
    g.write::<u32>(r + 0x94, 0);
    let t1 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x1b1), w(0x15a));
    let listed = g.read::<u32>(w(0x1b7)) != 0;
    g.write::<u32>(t1 + 0x14, r);
    g.write::<u32>(t1 + 4, 0);
    if listed {
        let head = g.read::<u32>(w(0x1cc));
        g.write::<u32>(t1, head);
        g.write::<u32>(head + 4, t1);
    } else {
        g.write::<u32>(t1, 0);
        g.write::<u32>(w(0x1df), t1);
    }
    let r = o + 0x5a68;
    g.write::<u32>(t1 + 8, t1);
    for (at, x) in [
        (0xf0u32, 0x3080_8080u32),
        (0x124, 4),
        (0x120, 0),
        (0x110, 0),
        (0xf8, 0),
        (0xfc, 0x3d4c_cccd),
        (0x100, 0),
        (0x108, 0x32),
        (0x104, 0x32),
        (0x10c, 8),
    ] {
        g.write::<u32>(r + at, x);
    }
    let t2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x256), w(0x233));
    let listed = g.read::<u32>(w(0x25c)) != 0;
    g.write::<u32>(t2 + 0x14, r);
    g.write::<u32>(t2 + 4, 0);
    g.write::<u32>(w(0x26c), t2);
    if listed {
        g.write::<u32>(t2, t1);
        g.write::<u32>(t1 + 4, t2);
    } else {
        g.write::<u32>(t2, 0);
        g.write::<u32>(w(0x287), t2);
    }
    g.write::<u32>(t2 + 8, t2);
    Ret::eax(t2)
}

/// [`objects_0165`] (the `rel32`s: the list removal, the two object
/// starts, the two task takes).
pub const OBJECTS_0165: Sig = Sig {
    len: 656,
    hash: 0x4c6a_a5d9_29b4_9069,
    calls: &[
        (0x17, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x5d, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xd5, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x1b1, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x256, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// [`texture_0098`]'s draw operands
/// ([`texture_tail`](crate::r16::texture_tail)).
const TAIL_0098: crate::r16::TexTail = crate::r16::TexTail {
    lock: [0x16b, 0x1c0],
    slot885: 0x1a8,
    xy: [0x1d8, 0x1ff],
    scale: [0x212, 0x23f],
    bytes: [
        (0x225, 0x230),
        (0x246, 0x25f),
        (0x26e, 0x282),
        (0x291, 0x2a2),
    ],
    hi: 0x2bc,
    k2: 0x2d6,
    lock879: 0x332,
    d: 0x369,
    one: 0x377,
    slot879: 0x392,
    unlock879: 0x3b1,
};

/// `magic_0098`'s screen fade texture (internal, no register arguments:
/// the `xmm0` it saves at `ebp - 0x120` is overwritten before it is
/// read; a cookie frame, `L(o)` = `ebp - o`): slot 719 `(*G_N, 2, 3)`
/// gives `T`, slot 235 `(A)` and `(B)`; the draw record's words (the
/// fade level `*G_F` times the constant at operand `+0x79`, truncated,
/// into its alpha byte; the buffers, counts and colours, two halfwords
/// and a byte) and slot 825 `(*G_X)` (`E`); then the texture's draw
/// ([`texture_tail`](crate::r16::texture_tail), as `magic_0661`'s).
/// Each slot call sees the words the earlier ones left on the stack.
/// Returns the lock pointer (0 when slot 885 does not answer).
///
/// `magic_0098` 0x10001b00 (977 bytes).
pub fn texture_0098<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, at: u32, n: u32| slot_target(g, w(at), n);
    let ebp = fr.esp.wrapping_sub(4);
    let mut st = crate::pushes::Pushes::default();
    st.push_all(&[3, 2]);
    g.write::<u32>(ebp - 0x120, fr.xmm_f32(0).to_bits());
    let fade = g.read::<u32>(w(0x2f));
    st.push(g.read::<u32>(w(0x37)));
    g.write::<u32>(ebp - 0x118, fade);
    // call width: 12 (Pushes caps the words at 12)
    let tex = st.call(g, t(g, 0x14, 719), 0).eax;
    st.push(w(0x4c));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x47, 235), 0);
    st.push(w(0x61));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x5c, 235), 0);
    let alpha = mul(f32_at(g, ebp - 0x118), f32_at(g, w(0x79)));
    let mut edx = g.read::<u32>(w(0x7f));
    st.push(g.read::<u32>(w(0x85)));
    g.write::<u32>(w(0x8f), edx);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // `mov [G], al`
    g.write::<u8>(w(0x94), crate::sb_sse::cvtt(alpha) as u8);
    let v = g.read::<u32>(w(0x99));
    g.write::<u32>(w(0x9e), v);
    let v = g.read::<u32>(w(0xa3));
    g.write::<u32>(w(0xa8), v);
    let sum = g.read::<u32>(w(0xad)).wrapping_add(g.read::<u32>(w(0xb3)));
    stores(g, img, f, &[0xb7]);
    edx = edx.wrapping_add(sum);
    stores(g, img, f, &[0xc8, 0xd2, 0xdc]);
    #[allow(clippy::cast_possible_truncation)] // the u16 and byte immediates
    {
        g.write::<u16>(w(0xe9), w(0xed) as u16);
        g.write::<u8>(w(0xf1), w(0xf5) as u8);
    }
    stores(g, img, f, &[0xf6, 0x100, 0x10a]);
    #[allow(clippy::cast_possible_truncation)] // the u16 immediates
    {
        g.write::<u16>(w(0x117), w(0x11b) as u16);
        g.write::<u16>(w(0x120), w(0x124) as u16);
    }
    g.write::<u32>(w(0x12e), edx);
    // call width: 12 (Pushes caps the words at 12)
    let e = st.call(g, t(g, 0xc4, 825), 0).eax;
    g.write::<u32>(ebp - 0x120, e);
    // `ecx` at slot 885: `wy`; at slot 879: `wy << 16`
    let v = u64::from(g.read::<u32>(tex)) | (u64::from(g.read::<u32>(tex + 4)) << 32);
    let wy = u32::from(1u16 << ((v >> 30) & 15));
    crate::r16::texture_tail(g, img, f, ebp, (tex, e), [wy, wy << 16], &TAIL_0098)
}

/// [`texture_0098`] (the `rel32`: the cookie check).
pub const TEXTURE_0098: Sig = Sig {
    len: 977,
    hash: 0x6c33_e8ab_0e86_0ad8,
    calls: &[(0x3c9, Callee::Any)],
    imports: &[],
};

/// `magic_0098`'s screen fade step (an overlay table entry, cdecl, no
/// arguments): while slot 253 says the scene runs, the fade level
/// `*G_F` steps down by one of two constants (by the flag `*G_D`); it is
/// held within `0..K` (`comiss` + `ja`/`jbe`: an unordered level stays).
/// Slot 38 `(*G_V, level, 0)` and slot 275 `(1)`, then the texture
/// ([`texture_0098`]). Returns 0 when the level is the constant at
/// operand `+0x99` (`ucomiss`: equal and ordered), else 1.
///
/// `magic_0098` 0x10002160 (172 bytes).
pub fn fade_step_0098<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // call width: 0
    let on = g.call(slot_target(g, w(0x1), 253), 0, &[]).eax;
    let mut x = f32_at(g, w(0x11));
    if on != 0 {
        let k = if g.read::<u32>(w(0x1b)) != 0 {
            0x26
        } else {
            0x30
        };
        x = sub(x, f32_at(g, w(k)));
        put(g, w(0x38), x);
    }
    let top = f32_at(g, w(0x48));
    if 0.0 > x {
        x = 0.0;
        put(g, w(0x58), x);
    } else if x > top {
        x = top;
        put(g, w(0x58), x);
    }
    let v = g.read::<u32>(w(0x71));
    // call width: 3
    g.call(slot_target(g, w(0x5d), 38), 0, &[v, x.to_bits(), 0]);
    // call width: 4 (the earlier call's words still pushed)
    g.call(slot_target(g, w(0x78), 275), 0, &[1, v, x.to_bits(), 0]);
    let tex = Frame {
        at: img.rel32_target(f + 0x8a),
        esp: fr.esp - 4,
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    texture_0098(g, img, &tex);
    let level = f32_at(g, w(0x92));
    // `ucomiss` + `jp`: 0 only for an ordered equal
    #[allow(clippy::float_cmp)] // the original's exact compare
    let differs = level != f32_at(g, w(0x99));
    Ret::eax(u32::from(differs))
}

/// [`fade_step_0098`] (the `rel32`: [`texture_0098`]).
pub const FADE_STEP_0098: Sig = Sig {
    len: 172,
    hash: 0x72e2_fa3d_e1a6_fbaa,
    calls: &[(0x8a, Callee::Sig(&TEXTURE_0098))],
    imports: &[],
};
