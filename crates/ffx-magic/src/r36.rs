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

/// `magic_0098`'s start-up (an overlay table entry, cdecl `(actor)`; a
/// cookie frame, `L(o)` = `ebp - o`). Slot 803 `(0x1000000, 0x10)`
/// gives the work buffer, cut into four 0x80000-byte parts (the operands
/// `+0x26`..`+0x44`); `*G_ACT = actor`. The bank: slots 718, 222 and 221
/// `(bank, 0, 1000, 0x2000)`, its parts 0..3 (slot 719 `(bank, 6, k)`)
/// and `T` (slot 719 `(bank, 8, 0)`); slot 711 `(T, ..., 0)` under the
/// lock, slot 710 `(at +0x180, T, buffer, 0x80000)`, three slot 941 of
/// `T`'s words; slots 27 `(0x3015, 1)` and 4 `(0x3015)`. The summon (slot
/// 168 `(actor)`), the party's first id (slot 218 `(actor, L(0x48))`'s
/// list), its model (slot 215) and matrix (slot 210 `(m, L(0x44))`), the
/// two scene points (slot 325, the first through slot 232 and scaled by
/// the constant at `+0x28c`), the distances of slots 19 and 364 (ST0).
/// Then the follow: [`height_follow`](crate::m0098::height_follow) when
/// the summon and the party's id are in the same of the two id lists
/// ([`id_list`](crate::s4_small::id_list)), else
/// [`pair_follow`](crate::m0098::pair_follow); slot 268 `(m, 0x10, P)`,
/// four 1.0 words, the camera matrix
/// ([`camera_matrix`](crate::m0098::camera_matrix) with the scales
/// `L(0x58)` = 0.001 three times), the tilt
/// ([`tilt`](crate::m0098::tilt)), fifteen slot 389 `(k, value)` and slot
/// 367 `(actor, id, L(0x48), 0, 0, 0, 0, 0)`. The flag `*L(0x48) & 1`
/// picks which of two 0x110-byte blocks is cleared (`memset`) and set.
/// Returns 1.
///
/// `magic_0098` 0x10002ba0 (1722 bytes).
#[allow(clippy::too_many_lines)] // the original's set-up, in order
pub fn start_up_0098<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, at: u32, n: u32| slot_target(g, w(at), n);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    // sub esp, 0x58; push esi
    let sp = ebp.wrapping_sub(0x5c);
    let actor = fr.arg(g, 0);
    // call width: 2
    let buf = g.call(t(g, 0x11, 803), 0, &[0x0100_0000, 0x10]).eax;
    for (k, op) in (0u32..).zip([0x26u32, 0x30, 0x3a, 0x44]) {
        g.write::<u32>(w(op), buf.wrapping_add(0x8_0000 * k));
    }
    g.write::<u32>(w(0x4c), actor);
    stores(g, img, f, &[0x60]);
    // call width: 1
    g.call(t(g, 0x51, 718), 0, &[w(0x56)]);
    for (gi, bank, n) in [(0x6du32, 0x85u32, 222u32), (0x8c, 0xa4, 221)] {
        let b = g.read::<u32>(w(bank));
        // call width: 4
        g.call(t(g, gi, n), 0, &[b, 0, 0x3e8, 0x2000]);
    }
    for (k, (gi, bank, store)) in (0u32..).zip([
        (0xabu32, 0xbbu32, 0xc2u32),
        (0xc7, 0xd7, 0xe1),
        (0xe6, 0xf6, 0xfd),
        (0x102, 0x112, 0x119),
    ]) {
        let b = g.read::<u32>(w(bank));
        // call width: 3
        let p = g.call(t(g, gi, 719), 0, &[b, 6, k]).eax;
        g.write::<u32>(w(store), p);
    }
    let b = g.read::<u32>(w(0x12e));
    // call width: 3
    let tex = g.call(t(g, 0x11e, 719), 0, &[b, 8, 0]).eax;
    let lock = g.read::<u32>(w(0x136));
    let held = g.read::<u32>(lock);
    g.write::<u32>(w(0x13f), tex);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x14a, 711), 0, &[tex, w(0x144), 0]);
    let lock = g.read::<u32>(w(0x15d));
    g.write::<u32>(lock, held);
    let (part, tex) = (g.read::<u32>(w(0x16f)), g.read::<u32>(w(0x17b)));
    // call width: 4
    g.call(t(g, 0x164, 710), 0, &[w(0x180), tex, part, 0x8_0000]);
    for (word, (tex_op, gi, store)) in (0x14u32..).step_by(4).zip([
        (0x187u32, 0x192u32, 0x19fu32),
        (0x1a4, 0x1ac, 0x1b9),
        (0x1be, 0x1c6, 0x1d3),
    ]) {
        let tex = g.read::<u32>(w(tex_op));
        let x = g.read::<u32>(tex + word);
        // call width: 1
        let p = g.call(t(g, gi, 941), 0, &[x]).eax;
        g.write::<u32>(w(store), p);
    }
    // call width: 2
    g.call(t(g, 0x1d8, 27), 0, &[0x3015, 1]);
    // call width: 1
    g.call(t(g, 0x1e9, 4), 0, &[0x3015]);
    let actor = g.read::<u32>(w(0x1fe));
    // call width: 1
    let summon = g.call(t(g, 0x1f8, 168), 0, &[actor]).eax;
    g.write::<u32>(w(0x20b), summon);
    let actor = g.read::<u32>(w(0x21a));
    // call width: 2
    let party = g.call(t(g, 0x214, 218), 0, &[actor, l(0x48)]).eax;
    let id = u32::from(g.read::<u8>(party));
    g.write::<u32>(w(0x22a), id);
    // call width: 1
    let model = g.call(t(g, 0x230, 215), 0, &[id]).eax;
    g.write::<u32>(w(0x241), model);
    // call width: 2
    g.call(t(g, 0x247, 210), 0, &[model, l(0x44)]);
    // call width: 2
    g.call(t(g, 0x258, 325), 0, &[w(0x25d), l(0x24)]);
    // call width: 2
    g.call(t(g, 0x26a, 232), 0, &[w(0x27a), w(0x26f)]);
    let k = f32_at(g, w(0x28c));
    for (from, to) in [(0x284u32, 0x29bu32), (0x2a3, 0x2af), (0x2b7, 0x2c3)] {
        let x = mul(f32_at(g, w(from)), k);
        put(g, w(to), x);
    }
    // call width: 2
    g.call(t(g, 0x2cc, 325), 0, &[w(0x2d1), l(0x14)]);
    for (m_op, gi, store, n) in [
        (0x2e4u32, 0x2deu32, 0x2efu32, 19u32),
        (0x2fa, 0x2f4, 0x308, 364),
    ] {
        let m = g.read::<u32>(w(m_op));
        // call width: 1
        let d = crate::m0018::st0_f32(g.call(t(g, gi, n), 0, &[m]).st0);
        put(g, w(store), d);
    }
    let lists = |g: &mut G, ecx: u32| {
        let fr = Frame {
            at: img.rel32_target(f + 0x316),
            esp: sp - 4,
            eax: 0,
            ecx,
            edx: 0,
            xmm: [0; 4],
        };
        crate::s4_small::id_list(g, img, &fr).eax
    };
    let summon = g.read::<u32>(w(0x30e));
    let a = lists(g, summon);
    let id = g.read::<u32>(w(0x31c));
    let b = lists(g, id);
    let same = (a == 1 && b == 1) || (a == 0 && b == 0);
    let follow = Frame {
        at: img.rel32_target(f + if same { 0x331 } else { 0x347 }),
        esp: sp - 4,
        eax: b,
        ecx: id,
        edx: 0,
        xmm: [0; 4],
    };
    if same {
        crate::m0098::height_follow(g, img, &follow);
    } else {
        crate::m0098::pair_follow(g, img, &follow);
    }
    let m = g.read::<u32>(w(0x359));
    // push P, 0x10, m: the camera matrix and the tilt see these words
    for (i, x) in (1u32..).zip([w(0x351), 0x10, m]) {
        g.write::<u32>(sp - 4 * i, x);
    }
    stores(g, img, f, &[0x35d, 0x367, 0x371, 0x37b]);
    for (o, x) in [
        (0x58u32, 0x3a83_126fu32),
        (0x54, 0x3a83_126f),
        (0x50, 0x3a83_126f),
        (0x4c, 0x3f80_0000),
    ] {
        g.write::<u32>(l(o), x);
    }
    // call width: 3
    g.call(t(g, 0x34c, 268), 0, &[m, 0x10, w(0x351)]);
    let csp = sp - 0x10;
    crate::m0098::camera_matrix(g, img, img.rel32_target(f + 0x3ad), l(0x58), csp);
    let tilt = Frame {
        at: img.rel32_target(f + 0x3b2),
        esp: csp,
        eax: 0,
        ecx: 0,
        edx: l(0x58),
        xmm: [0; 4],
    };
    crate::m0098::tilt(g, img, &tilt);
    // slot 389 `(k, value)`: the value over the earlier call's last word
    let values = [
        (0x3bau32, 0x3bfu32),
        (0x3d9, 0x3de),
        (0x3f8, 0x3fd),
        (0x417, 0x41c),
        (0x436, 0x43b),
        (0x455, 0x45a),
        (0x474, 0x479),
        (0x493, 0x498),
        (0x4b2, 0x4b7),
        (0x4d4, 0x4d9),
        (0x4f0, 0x4f5),
        (0x50f, 0x514),
        (0x52e, 0x533),
        (0x54d, 0x552),
        (0x56c, 0x571),
    ];
    let mut last = (0, 0);
    for (k, (v_op, gi)) in (0u32..).zip(values) {
        let v = g.read::<u32>(w(v_op));
        g.write::<u32>(sp - 4, v);
        g.write::<u32>(sp - 8, k);
        // call width: 2
        g.call(t(g, gi, 389), 0, &[k, v]);
        last = (k, v);
    }
    let (id, actor) = (g.read::<u32>(w(0x597)), g.read::<u32>(w(0x5a2)));
    // call width: 10 (the last slot 389 call's words still pushed)
    g.call(
        t(g, 0x59c, 367),
        0,
        &[actor, id, l(0x48), 0, 0, 0, 0, 0, last.0, last.1],
    );
    let flag = g.read::<u32>(l(0x48)) & 1;
    g.write::<u32>(w(0x5be), flag);
    if flag != 0 {
        g.write_bytes(w(0x5c7), &[0u8; 0x110]);
        stores(
            g,
            img,
            f,
            &[
                0x5d0, 0x5da, 0x5e4, 0x5ee, 0x5f8, 0x602, 0x60c, 0x616, 0x620, 0x62a, 0x634,
            ],
        );
    } else {
        g.write_bytes(w(0x641), &[0u8; 0x110]);
        stores(
            g,
            img,
            f,
            &[
                0x64a, 0x654, 0x65e, 0x668, 0x672, 0x67c, 0x686, 0x690, 0x69a,
            ],
        );
    }
    Ret::eax(1)
}

/// [`start_up_0098`] (the `rel32`s: the id lists twice, the height
/// follow twice, the pair follow, the camera matrix, the tilt, `memset`
/// twice, the cookie check).
pub const START_UP_0098: Sig = Sig {
    len: 1722,
    hash: 0xb528_1059_ccff_cebe,
    calls: &[
        (0x316, Callee::Sig(&crate::s4_small::ID_LIST)),
        (0x323, Callee::Sig(&crate::s4_small::ID_LIST)),
        (0x331, Callee::Sig(&crate::m0098::HEIGHT_FOLLOW)),
        (0x340, Callee::Sig(&crate::m0098::HEIGHT_FOLLOW)),
        (0x347, Callee::Sig(&crate::m0098::PAIR_FOLLOW)),
        (0x3ad, Callee::Sig(&crate::m0098::CAMERA_MATRIX)),
        (0x3b2, Callee::Sig(&crate::m0098::TILT)),
        (0x5cc, Callee::Import("memset")),
        (0x646, Callee::Import("memset")),
        (0x6b2, Callee::Any),
    ],
    imports: &[],
};

/// Where the code of [`start_up_0284`]'s two inlined task takes differs:
/// the operands of the free count (read, stored), the free head (read,
/// moved on, cleared), the callback, the busy list's first (tested,
/// set) and last (read, set), and the kind byte.
struct TaskTake {
    count: [u32; 2],
    head: [u32; 3],
    callback: u32,
    first: [u32; 2],
    last: [u32; 2],
    kind: u8,
}

/// Takes a node off a free ring (count `u16`, head) and appends it to the
/// busy list, as `magic_0284`'s start-up has it inlined twice (an empty
/// ring gives the node 0, whose words the original writes: it faults
/// there, and so does the port).
fn task_take<G: Game>(g: &mut G, img: &Image, f: u32, ops: &TaskTake) -> u32 {
    let w = |o: u32| img.word(f + o);
    let mut count = g.read::<u16>(w(ops.count[0]));
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(ops.head[0]));
        let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(prev + 4, next);
        let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node));
        g.write::<u32>(next, prev);
        if count > 1 {
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(w(ops.head[1]), next);
        } else {
            g.write::<u32>(w(ops.head[2]), 0);
        }
        count = count.wrapping_sub(1);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + o, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(ops.callback));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(ops.count[1]), count);
        node
    };
    let listed = g.read::<u32>(w(ops.first[0])) != 0;
    g.write::<u8>(node.wrapping_add(0x18), ops.kind);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let last = g.read::<u32>(w(ops.last[0]));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(ops.first[1]), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(ops.last[1]), node);
    node
}

/// An angle `v` brought into `-K2..K3` by a turn `K1` (`comiss` +
/// `jbe`: an unordered `v` stays), as [`start_up_0284`] has it.
fn wrap_turn(v: f32, (k3, k1, k2): (f32, f32, f32)) -> Option<f32> {
    if v > k3 {
        Some(sub(v, k1))
    } else if k2 > v {
        Some(add(v, k1))
    } else {
        None
    }
}

/// `magic_0284`'s start-up (an overlay table entry, cdecl `(actor)`; a
/// cookie frame, `L(o)` = `ebp - o`). The work buffers and the part
/// (slot 719 `(table, 8, 0)`,
/// [`part_init`](crate::l5_objhost::part_init)), the 0x80 sparks
/// cleared. The caster's facing `a1 = K - slot 333 (actor)` and `a2 =
/// a1 - K'` (each wrapped by a turn), the point `P` (slot 262, `y` lowered
/// by a constant) and the turn `L(0x94)` (slots 235, 328 `0`, 329 `a1`)
/// placing the offset `(0, 0, -13)` (slots 229, 248). Fourteen ring
/// objects ([`pool_take_140`](crate::sb_misc::pool_take_140), started by
/// [`start_object_at`](crate::s4_misc::start_object_at) with the owner
/// counting up by 3): at `P` raised by the table at operand `+0x2ba`
/// (index 0..4 cycling), facing `a1`, turned by slot 404's angles and
/// `a2` into their emitters' axes, the emitter set up
/// ([`emitter_setup`](crate::s7_misc::emitter_setup)) with the callback
/// at `+0x3d8`, listed at operand `+0x40a`. The dealt order
/// ([`deal`](crate::sb_misc::deal) into `L(0x40)`), then one smoke object
/// per target (`*G_N`): at `P` raised by the table at `rand() % 5`, the
/// target's point (slot 262 on the actor or the target's character) and
/// height (slot 268), jittered by slot 314 `(1)`, `(4)`, `(1)`, its
/// emitter set up with the callback at `+0x5bd` and listed at `+0x5d8`,
/// the order key growing by `2 + rand() % 3`. Then the effect words, the
/// summon's model (slot 215, or 413 with the flag), two tasks taken
/// ([`task_take`]) and the pool reset
/// ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)). Returns 0.
///
/// `magic_0284` 0x10001340 (2132 bytes).
#[allow(clippy::too_many_lines)] // the original's set-up, in order
pub fn start_up_0284<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, at: u32, n: u32| slot_target(g, w(at), n);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    // sub esp, 0x128; push ebx, esi, edi: the calls' return address below
    let sp = ebp.wrapping_sub(0x134 + 4);
    stores(g, img, f, &[0x27, 0x31, 0x3b]);
    // call width: 3
    let part = g.call(t(g, 0x14, 719), 0, &[w(0x23), 8, 0]).eax;
    g.write::<u32>(w(0x4b), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x50));
    stores(g, img, f, &[0x57]);
    for k in 0..0x80u32 {
        g.write::<u32>(w(0x62) + 0x140 * k, 0);
    }
    let actor = g.read::<u32>(w(0x8b));
    for (o, x) in [
        (0x34u32, 0u32),
        (0x30, 0),
        (0x2c, 0xc316_0000),
        (0x28, 0x3f80_0000),
    ] {
        g.write::<u32>(l(o), x);
    }
    g.write::<u32>(w(0xab), 0);
    stores(g, img, f, &[0xaf]);
    g.write::<u32>(l(0x120), 0x14);
    // call width: 1
    let facing = crate::m0018::st0_f32(g.call(t(g, 0x80, 333), 0, &[actor]).st0);
    put(g, l(0x118), facing);
    let turn = (f32_at(g, w(0xdf)), f32_at(g, w(0xe7)), f32_at(g, w(0xef)));
    let a1 = sub(f32_at(g, w(0xcf)), f32_at(g, l(0x118)));
    put(g, l(0x124), a1);
    let a1 = wrap_turn(a1, turn).map_or(a1, |v| {
        put(g, l(0x124), v);
        v
    });
    let a2 = sub(a1, f32_at(g, w(0x11e)));
    put(g, l(0x128), a2);
    if let Some(v) = wrap_turn(a2, turn) {
        put(g, l(0x128), v);
    }
    // call width: 1
    g.call(t(g, 0x14e, 235), 0, &[l(0x114)]);
    let actor = g.read::<u32>(w(0x16d));
    for (o, x) in [
        (0x24u32, 0u32),
        (0x20, 0),
        (0x1c, 0xc150_0000),
        (0x18, 0x3f80_0000),
    ] {
        g.write::<u32>(l(o), x);
    }
    // call width: 4
    g.call(t(g, 0x167, 262), 0, &[actor, l(0x14), l(0x10), l(0xc)]);
    let y = sub(f32_at(g, l(0x10)), f32_at(g, w(0x196)));
    g.write::<u32>(l(0x8), 0x3f80_0000);
    put(g, l(0x10), y);
    // call width: 1
    g.call(t(g, 0x1a2, 235), 0, &[l(0x94)]);
    // call width: 3
    g.call(t(g, 0x1c9, 328), 0, &[l(0x94), l(0x94), 0]);
    let a1 = g.read::<u32>(l(0x124));
    // call width: 3
    g.call(t(g, 0x1ee, 329), 0, &[l(0x94), l(0x94), a1]);
    g.write::<u32>(l(0x18), 0x3f80_0000);
    // call width: 3
    g.call(t(g, 0x20a, 229), 0, &[l(0x44), l(0x94), l(0x24)]);
    // call width: 3
    g.call(t(g, 0x227, 248), 0, &[l(0x14), l(0x14), l(0x44)]);
    g.write::<u32>(l(0x8), 0x3f80_0000);
    g.write::<u32>(l(0x118), 0);
    // the ring objects
    let mut row = 0u32;
    let mut e = w(0x244);
    loop {
        let s = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + 0x249));
        if s == 0 {
            break;
        }
        let owner = g.read::<u32>(l(0x120));
        crate::s4_misc::start_object_at(
            g,
            img,
            img.rel32_target(f + 0x26d),
            s,
            f32_at(g, w(0x25b)),
            1,
            owner,
            0,
        );
        copy_words(g, l(0x14), s + 0xd0, 4);
        let a1 = g.read::<u32>(l(0x124));
        g.write::<u32>(s + 0xc4, a1);
        let y = add(f32_at(g, w(0x2ba) + 4 * row), f32_at(g, s + 0xd4));
        g.write::<u32>(sp, 0x4248_0000);
        put(g, s + 0xd4, y);
        copy_words(g, s + 0xd0, e - 4, 4);
        // call width: 1
        let r = g.call(t(g, 0x2fc, 404), 0, &[0x4248_0000]).st0;
        put(g, l(0x11c), crate::m0018::st0_f32(r));
        let pitch = sub(f32_at(g, w(0x312)), f32_at(g, l(0x11c)));
        g.write::<u32>(sp, 0x3fc9_0fdb);
        put(g, l(0x30), pitch);
        // call width: 1
        let r = g.call(t(g, 0x31f, 404), 0, &[0x3fc9_0fdb]).st0;
        put(g, l(0x11c), crate::m0018::st0_f32(r));
        let mut yaw = add(f32_at(g, l(0x11c)), f32_at(g, l(0x128)));
        if yaw > f32_at(g, w(0x353)) {
            yaw = sub(yaw, f32_at(g, w(0x35d)));
        } else if f32_at(g, w(0x367)) > yaw {
            yaw = add(yaw, f32_at(g, w(0x374)));
        }
        // call width: 3
        g.call(t(g, 0x38d, 329), 0, &[l(0xd4), l(0x114), yaw.to_bits()]);
        // call width: 3
        g.call(t(g, 0x3ac, 229), 0, &[e + 0x23c, l(0xd4), l(0x34)]);
        let owner = g.read::<u32>(l(0x120));
        g.write::<u32>(e + 0x26c, owner);
        let n = g.read::<u32>(l(0x118));
        let emitter = e - 0x10;
        g.write::<u32>(e + 0x25c, w(0x3d8));
        g.write::<u32>(e + 0x260, 0);
        g.write::<u32>(e + 0x270, n);
        let setup = Frame {
            at: img.rel32_target(f + 0x3ed),
            esp: sp,
            eax: n,
            ecx: emitter,
            edx: 0,
            xmm: [0; 4],
        };
        crate::s7_misc::emitter_setup(g, img, &setup);
        row = if row + 1 > 4 { 0 } else { row + 1 };
        let n = g.read::<u32>(l(0x118));
        let owner = g.read::<u32>(l(0x120));
        g.write::<u32>(l(0x120), owner.wrapping_add(3));
        g.write::<u32>(w(0x40a).wrapping_add(n.wrapping_mul(4)), emitter);
        e += 0x288;
        g.write::<u32>(l(0x118), n.wrapping_add(1));
        if n.wrapping_add(1).cast_signed() >= 0xe {
            break;
        }
    }
    crate::sb_misc::deal(g, img, img.rel32_target(f + 0x428), sp, l(0x40));
    // the smoke objects, one per target
    g.write::<u32>(l(0x118), 0x17);
    g.write::<u32>(l(0x120), 0);
    if g.read::<i32>(w(0x440)) > 0 {
        let mut i = 0u32;
        let mut e = w(0x44b);
        loop {
            copy_words(g, l(0x14), e - 0x244, 4);
            let who = i32::from(g.read::<i8>(l(0x40).wrapping_add(i)));
            g.write::<i32>(l(0x124), who);
            let k = g.rand().cast_signed() % 5;
            let y = add(
                f32_at(g, w(0x4a2).wrapping_add(k.cast_unsigned().wrapping_mul(4))),
                f32_at(g, e - 0x240),
            );
            put(g, e - 0x240, y);
            let m = if who == -1 {
                g.read::<u32>(w(0x4bc))
            } else {
                let sel = g.read::<u32>(w(0x4c3));
                let c = u32::from(g.read::<u8>(sel.wrapping_add(who.cast_unsigned())));
                g.read::<u32>(w(0x4ce).wrapping_add(4 * c))
            };
            g.write::<u32>(l(0x128), m);
            // call width: 4
            g.call(t(g, 0x4e3, 262), 0, &[m, e - 4, e, e + 4]);
            let m = g.read::<u32>(l(0x128));
            // call width: 3
            g.call(t(g, 0x4f4, 268), 0, &[m, 6, l(0x54)]);
            let h = g.read::<u32>(l(0x50));
            g.write::<u32>(e, h);
            g.write::<u32>(e + 8, 0x3f80_0000);
            let jitter = |g: &mut G, op: u32, x: u32, at: u32| {
                g.write::<u32>(sp, x);
                // call width: 1
                let r = g.call(t(g, op, 314), 0, &[x]).st0;
                put(g, l(0x11c), crate::m0018::st0_f32(r));
                add(f32_at(g, at), f32_at(g, l(0x11c)))
            };
            let v = jitter(g, 0x50e, 0x3f80_0000, e - 4);
            g.write::<u32>(sp, 0x4080_0000);
            put(g, e - 4, v);
            let v = jitter(g, 0x542, 0x4080_0000, e);
            g.write::<u32>(sp, 0x3f80_0000);
            put(g, e, v);
            let v = jitter(g, 0x56c, 0x3f80_0000, e + 4);
            let key = g.read::<u32>(l(0x118));
            g.write::<u32>(e + 0x2c, key);
            let who = g.read::<u32>(l(0x124));
            let emitter = e - 0x250;
            put(g, e + 4, v);
            g.write::<u32>(e + 0x1c, 0);
            g.write::<u32>(e + 0x20, w(0x5bd));
            g.write::<u32>(e + 0x30, who);
            let setup = Frame {
                at: img.rel32_target(f + 0x5c5),
                esp: sp,
                eax: who,
                ecx: emitter,
                edx: 0,
                xmm: [0; 4],
            };
            crate::s7_misc::emitter_setup(g, img, &setup);
            let n = g.read::<u32>(l(0x120));
            e += 0x288;
            g.write::<u32>(w(0x5d8).wrapping_add(n.wrapping_mul(4)), emitter);
            let step = g.rand().cast_signed() % 3;
            let key = g
                .read::<u32>(l(0x118))
                .wrapping_add(2)
                .wrapping_add(step.cast_unsigned());
            i = n.wrapping_add(1);
            g.write::<u32>(l(0x120), i);
            g.write::<u32>(l(0x118), key);
            if i.cast_signed() >= g.read::<i32>(w(0x604)) {
                break;
            }
        }
    }
    // the effect words and the summon's model
    let flag = g.read::<u32>(w(0x612)) != 0;
    g.write::<u16>(w(0x619), 0);
    let summon = g.read::<u32>(w(0x61e));
    g.write::<u32>(w(0x623), summon);
    stores(
        g,
        img,
        f,
        &[
            0x62d, 0x637, 0x641, 0x64b, 0x655, 0x65f, 0x669, 0x673, 0x67d, 0x687,
        ],
    );
    let slot = if flag { 413 } else { 215 };
    // call width: 1
    let model = g.call(t(g, 0x629, slot), 0, &[summon]).eax;
    let flag = g.read::<u32>(w(0x6a8)) != 0;
    g.write::<u32>(w(0x6b3), model);
    stores(g, img, f, &[0x6b7, 0x6c1]);
    g.write::<u32>(w(0x6cd), 1);
    if flag {
        g.write::<u16>(w(0x6e0), 1);
    } else {
        g.write::<u16>(w(0x6d7), 0);
    }
    task_take(
        g,
        img,
        f,
        &TaskTake {
            count: [0x6e7, 0x757],
            head: [0x6fb, 0x718, 0x720],
            callback: 0x74c,
            first: [0x75d, 0x783],
            last: [0x770, 0x791],
            kind: 1,
        },
    );
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x796), w(0x788));
    task_take(
        g,
        img,
        f,
        &TaskTake {
            count: [0x79d, 0x808],
            head: [0x7ac, 0x7c9, 0x7d1],
            callback: 0x7fd,
            first: [0x80e, 0x837],
            last: [0x824, 0x845],
            kind: 2,
        },
    );
    Ret::eax(0)
}

/// [`start_up_0284`] (the `rel32`s: the part set-up, the pool take, the
/// object start, the emitter set-up twice, the deal, the pool reset, the
/// cookie check; `rand` twice).
pub const START_UP_0284: Sig = Sig {
    len: 2132,
    hash: 0x9f62_ec76_37d5_8a5c,
    calls: &[
        (0x50, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x249, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0x26d, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x3ed, Callee::Sig(&crate::s7_misc::EMITTER_SETUP)),
        (0x428, Callee::Sig(&crate::sb_misc::DEAL)),
        (0x5c5, Callee::Sig(&crate::s7_misc::EMITTER_SETUP)),
        (
            0x796,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0284_3430),
        ),
        (0x84c, Callee::Any),
    ],
    imports: &[(0x491, "rand"), (0x5de, "rand")],
};

/// `magic_0284`'s load step (0x10001210, 118 bytes): the shared step
/// ([`load_step`](crate::r9_load::load_step)) with [`start_up_0284`].
pub fn load_0284<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0284)
}

/// [`load_0284`].
pub const LOAD_0284: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0284))],
    imports: &[],
};

/// `magic_0146`'s summon task (cdecl `(task)`, a task callback; an
/// 8-aligned frame, `T` the `esp` after its three pushes; `O` = the task's
/// object `+4`). By `O + 0x18`:
/// - 0: the colour object `O + 0x30` (slot 676 `(O + 0x310, 1, 0x40)`, the
///   DLL's table, appended to the list at operand `+0x308`), three slot
///   324 words, the link node `O + 0x1bc0`
///   ([`link_node_942`](crate::s4_small::link_node_942), a speed 50) and the
///   colour node `O + 0x1c10`
///   ([`colour_node_init`](crate::sa_misc::colour_node_init) from the
///   frame's `T + 0x1c`); the state goes to 1, the count `O + 0x2c` to 1;
/// - 1: while the model's (slot 941 `(O + 0x1c40)`) flag `u16 +0x3e` bit
///   13 is set: the nodes `O + 0x5c` and `O + 0x88`
///   ([`node_append`](crate::sb_misc::node_append)) placed at the model's
///   point (slot 685) and nudged by two slot 307 randoms, the insert
///   ([`node_insert`](crate::nodes::node_insert)) and the particle node
///   `O + 0xc0` ([`particle_node`](crate::sb_nodes::particle_node)); every
///   eighth time slots 213 and 239 (the sound), else slot 388 with one of
///   four sounds (slot 324 `& 3`); the state 0x2710;
/// - 0x2710: the task's node leaves its ring.
///
/// The count `O + 0x2c` goes up on every way out. Returns the last call's
/// `eax`.
///
/// `magic_0146` 0x10006d60 (1033 bytes).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0146<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -8; sub esp, 0x2c; push ebx, esi, edi
    let t = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x2c + 12);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    g.write::<u32>(t + 0x10, o);
    let state = g.read::<u32>(o + 0x18);
    let eax = match state {
        0x2710 => {
            let node = g.read::<u32>(task);
            let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
            g.write::<u32>(prev + 8, next);
            let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
            g.write::<u32>(next + 4, prev);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            prev
        }
        1 => {
            let m = g.read::<u32>(o + 0x1c40);
            // call width: 1
            let model = g.call(slot(g, 0x55, 941), 0, &[m]).eax;
            let on = u32::from(g.read::<u16>(model + 0x3e)) & 0x2000;
            g.write::<u32>(t + 0x18, on);
            if on == 0 {
                0
            } else {
                return Ret::eax(summon_step_0146(g, img, f, t, o));
            }
        }
        0 => {
            return Ret::eax(summon_start_0146(g, img, f, t, o));
        }
        _ => state.wrapping_sub(0x2710),
    };
    let n = g.read::<u32>(o + 0x2c);
    g.write::<u32>(o + 0x2c, n.wrapping_add(1));
    Ret::eax(eax)
}

/// [`summon_task_0146`]'s state 1 (the model's flag set).
fn summon_step_0146<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, o: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let k = |g: &G, op: u32| f32_at(g, w(op));
    let m = g.read::<u32>(o + 0x1c40);
    // call width: 1
    let model = g.call(slot(g, 0x7e, 941), 0, &[m]).eax;
    let a = o + 0x5c;
    g.write::<u32>(o + 0x1c50, w(0xa5));
    let append = img.rel32_target(f + 0xaa);
    crate::sb_misc::node_append(g, img, append, a, w(0x97), model);
    let c = u32::from(g.read::<u8>(model + 0x36));
    let p = model + 0x40 + c * 0x30;
    // call width: 2
    g.call(slot(g, 0xbf, 685), 0, &[a + 0x1c, p]);
    let spread = |g: &mut G, op: u32| {
        // call width: 0
        let out = g.call(slot(g, op, 307), 0, &[]);
        put(g, t + 0x14, crate::m0018::st0_f32(out.st0));
        (f32_at(g, t + 0x14), out.eax)
    };
    let (r, _) = spread(g, 0xd0);
    let x = sub(k(g, 0xe3), mul(sub(r, k(g, 0xf5)), k(g, 0x102)));
    let x = add(x, f32_at(g, a + 0x1c));
    put(g, a + 0x1c, x);
    let (r, r307) = spread(g, 0xfa);
    let d = mul(sub(r, k(g, 0x128)), k(g, 0x147));
    let one = k(g, 0x130);
    let b = g.read::<u32>(t + 0x10) + 0x88;
    g.write::<u32>(a + 0x28, 0x3f80_0000);
    let y = add(sub(one, d), f32_at(g, a + 0x24));
    put(g, a + 0x24, y);
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x163), b, w(0x13f), r307);
    for at in [0x1cu32, 0x20, 0x24] {
        g.write::<u32>(b + at, 0x3f80_0000);
    }
    let (r, _) = spread(g, 0x168);
    let z = add(mul(sub(r, k(g, 0x197)), k(g, 0x1b1)), k(g, 0x1c9));
    let e = g.read::<u32>(t + 0x10) + 0x1c0;
    let group = t + 0x1c;
    for (i, x) in (0u32..).zip([e, w(0x1b9), w(0x1c1), a, w(0x1d5), b]) {
        g.write::<u32>(group + 4 * i, x);
    }
    put(g, b + 0x28, z);
    let insert = Frame {
        at: img.rel32_target(f + 0x1e3),
        esp: t - 4,
        eax: z.to_bits(),
        ecx: group,
        edx: w(0x13f),
        xmm: [0; 4],
    };
    crate::nodes::node_insert(g, img, &insert);
    let pn = g.read::<u32>(t + 0x10) + 0xc0;
    let x = g.read::<u32>(w(0x1ec));
    g.write::<u8>(e + 0x148, 1);
    g.write::<u32>(group, pn);
    g.write::<u32>(group + 4, w(0x205));
    g.write::<u32>(group + 8, e);
    g.write::<u32>(group + 0xc, x);
    g.write::<u32>(pn, w(0x213));
    g.write::<u32>(group + 0x10, w(0x21f));
    g.write::<u32>(group + 0x14, 2);
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x22c), group);
    let n = g.read::<u32>(w(0x231)).wrapping_add(1);
    g.write::<u32>(w(0x237), n);
    let eax = if n.cast_signed() < 8 {
        // call width: 0
        let r = g.call(slot(g, 0x28d, 324), 0, &[]).eax & 3;
        let id = g.read::<u32>(w(0x2a1) + 4 * r);
        let b = u32::from(g.read::<u8>(w(0x2a8)));
        // call width: 3
        g.call(slot(g, 0x2ae, 388), 0, &[b, id, 1]).eax
    } else {
        let b1 = u32::from(g.read::<u8>(w(0x243)));
        // call width: 2
        g.call(slot(g, 0x24b, 213), 0, &[b1, 0]);
        let b2 = u32::from(g.read::<u8>(w(0x25a)));
        let v = g.read::<u32>(w(0x268));
        // call width: 5 (the earlier call's words still pushed)
        g.call(slot(g, 0x262, 239), 0, &[v, b2, 0, b1, 0]).eax
    };
    let o = g.read::<u32>(t + 0x10);
    let c = g.read::<u32>(o + 0x2c);
    g.write::<u32>(o + 0x2c, c.wrapping_add(1));
    g.write::<u32>(o + 0x18, 0x2710);
    eax
}

/// [`summon_task_0146`]'s state 0 (the start).
fn summon_start_0146<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, o: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // call width: 3
    g.call(slot(g, 0x2d3, 676), 0, &[o + 0x310, 1, 0x40]);
    let (cn, c) = (o + 0x1c10, o + 0x30);
    g.write::<u32>(c, w(0x2f5));
    g.write::<u16>(c + 0x1a, 0xff);
    g.write::<u32>(c + 0x10, c);
    g.write::<u32>(c + 0xc, c);
    g.write::<u32>(c + 0x14, w(0x308));
    g.write::<u32>(c + 4, w(0x30f));
    let last = g.read::<u32>(w(0x314));
    g.write::<u32>(c + 8, last);
    let last = g.read::<u32>(w(0x31c));
    if last == w(0x324) {
        g.write::<u32>(w(0x32c), c);
    } else {
        g.write::<u32>(last + 4, c);
    }
    let n = g.read::<u16>(w(0x33d));
    g.write::<u16>(w(0x33d), n.wrapping_add(1));
    g.write::<u32>(w(0x343), c);
    for (op, at) in [(0x336u32, 0x1cu32), (0x358, 0x20), (0x36d, 0x24)] {
        // call width: 0
        let r = g.call(slot(g, op, 324), 0, &[]).eax & 0xffff;
        g.write::<u32>(c + at, r);
    }
    let link = o + 0x1bc0;
    // push ecx (o + 0x310), push the node's table: the link's `(x)`
    g.write::<u32>(t - 4, o + 0x310);
    g.write::<u32>(t - 8, w(0x380));
    let lf = Frame {
        at: img.rel32_target(f + 0x38e),
        esp: t - 0xc,
        eax: 0,
        ecx: link,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s4_small::link_node_942(g, img, &lf);
    g.write::<u32>(t + 0x24, link);
    g.write::<u32>(link + 0x2c, 0x4248_0000);
    g.write::<u32>(link + 0x30, 0);
    g.write::<u32>(link + 0x34, 0);
    let group = t + 0x1c;
    g.write::<u32>(group, cn);
    g.write::<u32>(group + 4, w(0x3c6));
    g.write::<u32>(group + 0xc, w(0x3ce));
    g.write::<u32>(group + 0x10, o + 0x310);
    g.write::<u32>(group + 0x14, w(0x3da));
    g.write::<u32>(group + 0x18, c);
    g.write::<u32>(cn, w(0x3e4));
    let eax = crate::sa_misc::colour_node_init(g, img, img.rel32_target(f + 0x3e9), group);
    let b = g.read::<u8>(cn + 0x34);
    g.write::<u8>(cn + 0x34, b | 0x2c);
    let b = g.read::<u8>(cn + 0x54);
    g.write::<u8>(cn + 0x54, b | 1);
    let s = g.read::<u32>(o + 0x18);
    g.write::<u32>(o + 0x18, s.wrapping_add(1));
    g.write::<u32>(o + 0x2c, 1);
    eax
}

/// [`summon_task_0146`] (the `rel32`s: the node appends, the insert, the
/// particle node, the link node, the colour node).
pub const SUMMON_TASK_0146: Sig = Sig {
    len: 1033,
    hash: 0x712c_ef07_a232_5bb9,
    calls: &[
        (0xaa, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x163, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1e3, Callee::Sig(&crate::nodes::INSERT)),
        (0x22c, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x38e, Callee::Sig(&crate::s4_small::LINK_NODE_942)),
        (0x3e9, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
    ],
    imports: &[],
};

/// Writes `words` as the stack arguments of a call from `t` (the first
/// at the lowest address) and returns the callee's `esp`.
fn push_args<G: Game>(g: &mut G, t: u32, words: &[u32]) -> u32 {
    let n = u32::try_from(words.len()).unwrap_or(0);
    let base = t.wrapping_sub(4 * n);
    for (i, x) in (0u32..).zip(words) {
        g.write::<u32>(base + 4 * i, *x);
    }
    base.wrapping_sub(4)
}

fn frame_at(at: u32, esp: u32, ecx: u32) -> Frame {
    Frame {
        at,
        esp,
        eax: 0,
        ecx,
        edx: 0,
        xmm: [0; 4],
    }
}

/// `x - g` at the operands `(to, from)` ([`scene_task_0146`]'s light
/// words: `*to = k - *from`).
fn lights<G: Game>(g: &mut G, img: &Image, f: u32, k: f32, ops: &[(u32, u32)]) {
    for &(to, from) in ops {
        let x = sub(k, f32_at(g, img.word(f + from)));
        put(g, img.word(f + to), x);
    }
}

/// `magic_0146`'s scene task (cdecl `(task)`, a task callback; an
/// 8-aligned frame, `T` the `esp` after its three pushes, `O` = the task's
/// object `+4`): a byte-table dispatch on the state `O + 0x18` (the
/// count `O + 0x2c` the frames in it), re-dispatched where a state moves
/// on at once:
/// - 0: the model nodes ([`model_nodes_pick`](crate::m0146::model_nodes_pick)),
///   two nodes, an effect node record, the colour node; the flag `O + 0x78`
///   unless slot 520 says the scene 0x161000f; on to 0xa;
/// - 0xa: the light words (a constant less each of the scene's), the
///   party's ids not in the DLL's records (slot 288 `(0x10, &n)`, slot
///   541 `(id, 0, 10)`) kept at `O + 0x34`; on to 0x14;
/// - 0x14, 0x28, 0x32: the next state, the count 0;
/// - 0x15 (after 15 frames): a node and its insert, a particle node; 0x28;
/// - 0x1e: a link node and an effect node record, the next state;
/// - 0x1f: a countdown `O + 0x28`, then a task pushed for each of the 8
///   objects (`O + 0x30`; the countdown again from slot 324), then 0x32;
/// - 0x29 (after 30 frames): a particle node, the next state;
/// - 0x2a: slot 878 `(1)` under the lock; after 5 frames the move start,
///   a node, the size by the summon's kind (slot 390), its insert, a
///   particle node and slot 295; the next state;
/// - 0x2b: at frame 10 the scene flag again; after 50 frames 0x1e;
/// - 0x33 (after 25 frames): slots 296 and 541 for each kept id;
/// - 0x34 (after 15 frames): the light words back, the flag cleared; 0x2710;
/// - 0x2710: the task's node unlinked, slot 878 `(0)` under the lock.
///
/// With the flag set slot 845 runs; the count goes up. Returns the last
/// call's `eax` (or the value the original's last step left there).
///
/// `magic_0146` 0x100071c0 (2117 bytes with its jump and byte tables).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn scene_task_0146<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -8; sub esp, 0x2c; push ebx, esi, edi
    let t = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x2c + 12);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    let mut eax;
    'dispatch: loop {
        let state = g.read::<u32>(o + 0x18);
        eax = state;
        if state > 0x2710 {
            break;
        }
        if state == 0x2710 {
            // the task's node leaves its ring, slot 878 (0) under the lock
            let node = g.read::<u32>(task);
            let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
            g.write::<u32>(prev + 8, next);
            let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
            g.write::<u32>(next + 4, prev);
            let lock = g.read::<u32>(w(0x799));
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            let held = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            // call width: 1
            g.call(slot(g, 0x7ac, 878), 0, &[0]);
            let lock = g.read::<u32>(w(0x7b9));
            g.write::<u32>(lock, held);
            eax = lock;
            break;
        }
        if state > 0x34 {
            break;
        }
        // the byte table and the jump table live in `.text`
        let entry = img.word(w(0x42) + state) & 0xff;
        eax = entry;
        let count = g.read::<i32>(o + 0x2c);
        match img.word(w(0x49) + 4 * entry).wrapping_sub(f) {
            0x4d => {
                let group = t + 0x20;
                let x = g.read::<u32>(w(0x4e));
                for (i, v) in (0u32..).zip([w(0x5a), w(0x62), x, 0]) {
                    g.write::<u32>(group + 4 * i, v);
                }
                g.write::<u8>(group + 0x10, 1);
                stores(g, img, f, &[0x77]);
                let fr1 = frame_at(img.rel32_target(f + 0x82), t - 4, group);
                let r = crate::m0146::model_nodes_pick(g, img, &fr1);
                let node = w(0x8c);
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x91), node, w(0x87), r);
                let v = g.read::<u32>(w(0xa9));
                // call width: 3
                g.call(slot(g, 0x96, 268), 0, &[v, 0x11, w(0x9b)]);
                stores(g, img, f, &[0xb2]);
                let esp = push_args(g, t, &[node, w(0xc0), node, 0]);
                let fr1 = frame_at(img.rel32_target(f + 0xcb), esp, w(0xc6));
                crate::s4_fam446::effect_node_a(g, img, &fr1);
                let v = g.read::<u32>(w(0xd1));
                g.write::<u32>(w(0xd1), v | 1);
                stores(g, img, f, &[0xdd]);
                let cg = t + 0x18;
                g.write::<u32>(cg, w(0xeb));
                g.write::<u32>(cg + 4, w(0xf3));
                stores(g, img, f, &[0xf7]);
                let fr1 = frame_at(img.rel32_target(f + 0x102), t - 4, cg);
                crate::s3_misc::colour_node(g, img, &fr1);
                // call width: 0
                let scene = g.call(slot(g, 0x107, 520), 0, &[]).eax;
                if scene != 0x0161_000f {
                    g.write::<u32>(o + 0x78, 1);
                }
                g.write::<u32>(o + 0x18, 0xa);
                scene_lights_0146(g, img, f, t, o, f32_at(g, w(0x125)));
                continue 'dispatch;
            }
            0x130 => {
                scene_lights_0146(g, img, f, t, o, f32_at(g, w(0x24)));
                continue 'dispatch;
            }
            0x542 => {}
            0x2ce => {
                if count < 0xf {
                    break;
                }
                let (node, list) = (w(0x2de), w(0x2d9));
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x2e3), node, list, eax);
                let group = t + 0x20;
                g.write::<u32>(group + 0x14, node);
                stores(g, img, f, &[0x2ef, 0x2f9, 0x303, 0x30d]);
                for (i, v) in (0u32..).zip([w(0x31b), list, w(0x327), w(0x32f), w(0x337)]) {
                    g.write::<u32>(group + 4 * i, v);
                }
                let fr1 = frame_at(img.rel32_target(f + 0x33c), t - 4, group);
                crate::nodes::node_insert(g, img, &fr1);
                let esp = push_args(g, t, &[w(0x34b), 0, 0, 1]);
                let fr1 = frame_at(img.rel32_target(f + 0x350), esp, w(0x346));
                crate::s3_nodes::particle_node_with(g, img, &fr1);
                g.write::<u32>(o + 0x18, 0x28);
                continue 'dispatch;
            }
            0x6bf => {
                let esp = push_args(g, t, &[w(0x6c0)]);
                let fr1 = frame_at(img.rel32_target(f + 0x6ca), esp, w(0x6c5));
                crate::s4_small::link_node_38_one(g, img, &fr1);
                stores(g, img, f, &[0x6d1]);
                let esp = push_args(g, t, &[0, w(0x6df), 0, 1]);
                let fr1 = frame_at(img.rel32_target(f + 0x6ea), esp, w(0x6e5));
                eax = crate::s4_fam446::effect_node_a(g, img, &fr1).eax;
                let v = g.read::<u32>(w(0x6f0));
                g.write::<u32>(w(0x6f0), v | 1);
                stores(g, img, f, &[0x6f5]);
                let s = g.read::<u32>(o + 0x18);
                g.write::<u32>(o + 0x18, s.wrapping_add(1));
                g.write::<u32>(o + 0x28, 0);
                g.write::<u32>(o + 0x30, 0);
                break;
            }
            0x39a => {
                let c = g.read::<u32>(o + 0x28);
                eax = c.wrapping_sub(1);
                g.write::<u32>(o + 0x28, eax);
                if c != 0 {
                    break;
                }
                let i = g.read::<u32>(o + 0x30);
                let obj = i.wrapping_mul(0x1c70).wrapping_add(w(0x3b6));
                let fr1 = frame_at(img.rel32_target(f + 0x3bb), t - 4, obj);
                crate::o4_tiny4::push_task(g, img, &fr1);
                let i = g.read::<i32>(o + 0x30).wrapping_add(1);
                g.write::<i32>(o + 0x30, i);
                if i < 8 {
                    // call width: 0
                    eax = (g.call(slot(g, 0x507, 324), 0, &[]).eax & 3) + 2;
                    g.write::<u32>(o + 0x28, eax);
                    break;
                }
                g.write::<u32>(o + 0x18, 0x32);
                continue 'dispatch;
            }
            0x521 => {
                if count < 0x1e {
                    break;
                }
                let esp = push_args(g, t, &[w(0x536), 0, 0, 3]);
                let fr1 = frame_at(img.rel32_target(f + 0x53b), esp, w(0x531));
                eax = crate::s3_nodes::particle_node_with(g, img, &fr1);
                let s = g.read::<u32>(o + 0x18);
                g.write::<u32>(o + 0x18, s.wrapping_add(1));
                g.write::<u32>(o + 0x2c, 0);
                break;
            }
            0x551 => {
                let lock = g.read::<u32>(w(0x552));
                let held = g.read::<u32>(lock);
                g.write::<u32>(lock, 1);
                // call width: 1
                g.call(slot(g, 0x561, 878), 0, &[1]);
                let lock = g.read::<u32>(w(0x56e));
                g.write::<u32>(lock, held);
                eax = lock;
                if g.read::<i32>(o + 0x2c) < 5 {
                    break;
                }
                let k = f32_at(g, w(0x585));
                stores(
                    g,
                    img,
                    f,
                    &[0x58e, 0x598, 0x5a2, 0x5ac, 0x5b6, 0x5c0, 0x5ca],
                );
                crate::m0146::move_start(g, w(0x58a), k);
                crate::sb_misc::node_append(
                    g,
                    img,
                    img.rel32_target(f + 0x5e4),
                    w(0x5df),
                    w(0x5da),
                    eax,
                );
                let b = u32::from(g.read::<u8>(w(0x5eb)));
                stores(g, img, f, &[0x5f5, 0x5ff, 0x609]);
                // call width: 1
                let kind = g.call(slot(g, 0x5f1, 390), 0, &[b]).eax.cast_signed();
                if (0x1088..=0x1089).contains(&kind) || kind == 0x108c {
                    stores(g, img, f, &[0x633]);
                } else {
                    stores(g, img, f, &[0x63f]);
                }
                let group = t + 0x20;
                for (i, op) in (0u32..).zip([0x651u32, 0x659, 0x661, 0x669, 0x671, 0x679]) {
                    g.write::<u32>(group + 4 * i, w(op));
                }
                let fr1 = frame_at(img.rel32_target(f + 0x67e), t - 4, group);
                crate::nodes::node_insert(g, img, &fr1);
                let esp = push_args(g, t, &[w(0x68d), 0, 0, 0]);
                let fr1 = frame_at(img.rel32_target(f + 0x692), esp, w(0x688));
                crate::s3_nodes::particle_node_with(g, img, &fr1);
                let b = u32::from(g.read::<u8>(w(0x699)));
                // call width: 2 (above them the particle node's words, two of
                // them never written: slot 295 reads only its two)
                eax = g.call(slot(g, 0x6a1, 295), 0, &[b, 0]).eax;
                let s = g.read::<u32>(o + 0x18);
                g.write::<u32>(o + 0x18, s.wrapping_add(1));
                g.write::<u32>(o + 0x2c, 0);
                break;
            }
            0x363 => {
                if count == 0xa {
                    // call width: 0
                    eax = g.call(slot(g, 0x36a, 520), 0, &[]).eax;
                    if eax == 0x0161_000f {
                        g.write::<u32>(o + 0x78, 1);
                    }
                }
                if g.read::<i32>(o + 0x2c) < 0x32 {
                    break;
                }
                g.write::<u32>(o + 0x18, 0x1e);
                continue 'dispatch;
            }
            0x718 => {
                if count < 0x19 {
                    break;
                }
                let n = u32::from(g.read::<u16>(o + 0x74));
                if n == 0 {
                    eax = 0;
                } else {
                    let mut i = 0u32;
                    let mut id = o + 0x34;
                    loop {
                        let v = u32::from(g.read::<u16>(id));
                        // call width: 2
                        g.call(slot(g, 0x73a, 296), 0, &[v, 1]);
                        let v = u32::from(g.read::<u16>(id));
                        // call width: 3
                        g.call(slot(g, 0x758, 541), 0, &[v, 0x3f80_0000, 0xf]);
                        eax = u32::from(g.read::<u16>(o + 0x74));
                        i += 1;
                        id += 2;
                        if i.cast_signed() >= eax.cast_signed() {
                            break;
                        }
                    }
                }
            }
            0x3d8 => {
                if count < 0xf {
                    break;
                }
                let k1 = f32_at(g, w(0x2c));
                let fl = (g.read::<u32>(w(0x3ee)) & !1) | 4;
                stores(g, img, f, &[0x3f8]);
                lights(g, img, f, k1, &[(0x406, 0x3e9)]);
                stores(g, img, f, &[0x415, 0x41f, 0x429, 0x433]);
                lights(g, img, f, k1, &[(0x441, 0x411)]);
                stores(g, img, f, &[0x450, 0x45a, 0x464, 0x46e]);
                lights(g, img, f, k1, &[(0x47c, 0x44c)]);
                let x = sub(k1, f32_at(g, w(0x487)));
                g.write::<u32>(w(0x48c), fl);
                stores(g, img, f, &[0x490]);
                put(g, w(0x49e), x);
                lights(g, img, f, k1, &[(0x4b1, 0x4a9), (0x4c4, 0x4bc)]);
                let x = sub(f32_at(g, w(0x4cc)), f32_at(g, w(0x4d4)));
                put(g, w(0x4dc), x);
                lights(g, img, f, k1, &[(0x4ef, 0x4e7)]);
                g.write::<u32>(o + 0x78, 0);
                g.write::<u32>(o + 0x18, 0x2710);
                continue 'dispatch;
            }
            _ => break,
        }
        // 0x542: the next state, the count 0
        let s = g.read::<u32>(o + 0x18);
        g.write::<u32>(o + 0x18, s.wrapping_add(1));
        g.write::<u32>(o + 0x2c, 0);
        break;
    }
    if g.read::<u32>(o + 0x78) != 0 {
        // call width: 0
        eax = g.call(slot(g, 0x7c9, 845), 0, &[]).eax;
    }
    let c = g.read::<u32>(o + 0x2c);
    g.write::<u32>(o + 0x2c, c.wrapping_add(1));
    Ret::eax(eax)
}

/// [`scene_task_0146`]'s state 0xa (entered from state 0 too): the light
/// words `k - *G` (`k` the 1.0 in `xmm2`), 0.5s and the step 0.05, the
/// party's ids that no DLL record holds announced (slot 541 `(id, 0,
/// 10)`) and kept at `O + 0x34` (count `u16 O + 0x74`); then 0x14.
fn scene_lights_0146<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, o: u32, k: f32) {
    use crate::r11_fades::stores;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let x = sub(k, f32_at(g, w(0x137)));
    let v = g.read::<u32>(w(0x13d));
    g.write::<u32>(w(0x13d), v & !1);
    stores(g, img, f, &[0x144, 0x14e]);
    put(g, w(0x15c), x);
    let x = sub(k, f32_at(g, w(0x167)));
    stores(g, img, f, &[0x16b, 0x175, 0x17f, 0x189]);
    put(g, w(0x197), x);
    let x = sub(k, f32_at(g, w(0x1a2)));
    stores(g, img, f, &[0x1a6, 0x1b0, 0x1ba]);
    put(g, w(0x1c8), x);
    lights(
        g,
        img,
        f,
        k,
        &[(0x1db, 0x1d3), (0x1ee, 0x1e6), (0x201, 0x1f9)],
    );
    let x = sub(f32_at(g, w(0x209)), f32_at(g, w(0x211)));
    put(g, w(0x219), x);
    lights(g, img, f, k, &[(0x22c, 0x224)]);
    g.write::<u16>(o + 0x74, 0);
    // call width: 2
    let list = g.call(slot(g, 0x23a, 288), 0, &[0x10, t + 0x14]).eax;
    g.write::<u32>(t + 0x10, 0);
    if g.read::<i32>(t + 0x14) > 0 {
        let mut i = 0u32;
        let mut p = list;
        loop {
            let n = g.read::<i32>(w(0x25e));
            let id = u32::from(g.read::<u8>(p));
            let mut j = 0i32;
            let mut held = false;
            while j < n {
                if g.read::<u32>(w(0x26c).wrapping_add(0x98 * j.cast_unsigned())) == id {
                    held = true;
                    break;
                }
                j += 1;
            }
            if !held {
                // call width: 3
                g.call(slot(g, 0x28d, 541), 0, &[id, 0, 0xa]);
                let k = g.read::<u16>(o + 0x74);
                let id = g.read::<u8>(p);
                g.write::<u16>(o + 0x34 + 2 * u32::from(k), u16::from(id));
                g.write::<u16>(o + 0x74, k.wrapping_add(1));
            }
            i += 1;
            p = p.wrapping_add(1);
            g.write::<u32>(t + 0x10, i);
            if i.cast_signed() >= g.read::<i32>(t + 0x14) {
                break;
            }
        }
    }
    g.write::<u32>(o + 0x18, 0x14);
}

/// [`scene_task_0146`] (the `rel32`s: the model nodes, the node appends,
/// the effect node records, the colour node, the inserts, the particle
/// nodes, the push task, the move start, the link node).
pub const SCENE_TASK_0146: Sig = Sig {
    len: 2117,
    hash: 0x8938_0768_5608_c38a,
    calls: &[
        (0x82, Callee::Sig(&crate::m0146::MODEL_NODES_PICK)),
        (0x91, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xcb, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x102, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x2e3, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x33c, Callee::Sig(&crate::nodes::INSERT)),
        (0x350, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x3bb, Callee::Sig(&crate::o4_tiny4::PUSH_TASK_0146_7170)),
        (0x53b, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x5d5, Callee::Sig(&crate::m0146::MOVE_START)),
        (0x5e4, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x67e, Callee::Sig(&crate::nodes::INSERT)),
        (0x692, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x6ca, Callee::Sig(&crate::s4_small::LINK_NODE_38_ONE)),
        (0x6ea, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
    ],
    imports: &[],
};

/// The operands of the ring links at the end of a 0613-style start-up
/// ([`start_up_0155`], [`start_up_0164`]): the first object's ring last
/// (read, stored), its four stores, the old last's link; the second's
/// ring last (read), its flag, its store, seven stores, the link; the
/// first's flag, three stores; and slots 557/560 (their tables'
/// operands) when the copy has them.
struct RingTail {
    last: [u32; 2],
    first: [u32; 4],
    link: u32,
    last2: u32,
    flag: u32,
    last2_to: u32,
    second: [u32; 7],
    link2: u32,
    flag2: u32,
    third: [u32; 3],
    slots: Option<(u32, u32)>,
}

fn ring_tail<G: Game>(g: &mut G, img: &Image, f: u32, r: &RingTail) {
    use crate::r23::stores_0613;
    let w = |o: u32| img.word(f + o);
    let last = g.read::<u32>(w(r.last[0]));
    g.write::<u32>(w(r.last[1]), last);
    stores_0613(g, img, f, &r.first);
    g.write::<u32>(last.wrapping_add(4), w(r.link));
    let last = g.read::<u32>(w(r.last2));
    let fl = g.read::<u32>(w(r.flag));
    g.write::<u32>(w(r.flag), fl & !1);
    g.write::<u32>(w(r.last2_to), last);
    stores_0613(g, img, f, &r.second);
    g.write::<u32>(last.wrapping_add(4), w(r.link2));
    let fl = g.read::<u32>(w(r.flag2));
    g.write::<u32>(w(r.flag2), fl & !1);
    stores_0613(g, img, f, &r.third);
    if let Some((t557, t560)) = r.slots {
        // call width: 0
        let x = g.call(slot_target(g, w(t557), 557), 0, &[]).eax & !4;
        // call width: 1
        g.call(slot_target(g, w(t560), 560), 0, &[x]);
    }
}

/// A 0613-style start-up with slot 673 `()` first (cdecl `(actor)`; an
/// 8-aligned frame): slot 673, then
/// [`start_up_head_with`](crate::r23::start_up_head_with) on the code 0xd
/// bytes on (slot 221's bank word at `+0x1fb`), then the rings
/// ([`ring_tail`]). Returns 0.
fn start_up_673<G: Game>(g: &mut G, img: &Image, fr: &Frame, tail: &RingTail) -> Ret {
    let f = fr.at;
    // call width: 0
    g.call(slot_target(g, img.word(f + 0x8), 673), 0, &[]);
    let shifted = Frame { at: f + 0xd, ..*fr };
    crate::r23::start_up_head_with(g, img, &shifted, 0x1fb - 0xd);
    ring_tail(g, img, f, tail);
    Ret::eax(0)
}

/// `magic_0155`'s start-up (cdecl `(actor)`): [`start_up_0609`](crate::r27::start_up_0609)
/// with slot 673 first and its operands 0xd bytes on ([`start_up_673`]).
///
/// `magic_0155` 0x1000bf70 (747 bytes).
pub fn start_up_0155<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_up_673(
        g,
        img,
        fr,
        &RingTail {
            last: [0x208, 0x20d],
            first: [0x211, 0x21b, 0x225, 0x22f],
            link: 0x23c,
            last2: 0x241,
            flag: 0x247,
            last2_to: 0x24d,
            second: [0x251, 0x25b, 0x265, 0x26f, 0x279, 0x283, 0x28d],
            link2: 0x29a,
            flag2: 0x2a5,
            third: [0x2aa, 0x2b4, 0x2be],
            slots: Some((0x29f, 0x2d5)),
        },
    )
}

/// [`start_up_0155`] (the `rel32`: the targets' set-up).
pub const START_UP_0155: Sig = Sig {
    len: 747,
    hash: 0xf9e9_24c3_4949_875d,
    calls: &[(0x19, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C))],
    imports: &[],
};

/// `magic_0164`'s start-up (cdecl `(actor)`): [`start_up_0613`](crate::r23::start_up_0613)
/// with slot 673 first and its operands 0xd bytes on ([`start_up_673`]).
///
/// `magic_0164` 0x1000bcf0 (717 bytes).
pub fn start_up_0164<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_up_673(
        g,
        img,
        fr,
        &RingTail {
            last: [0x208, 0x20d],
            first: [0x211, 0x21b, 0x225, 0x22f],
            link: 0x23c,
            last2: 0x241,
            flag: 0x247,
            last2_to: 0x250,
            second: [0x254, 0x25e, 0x268, 0x272, 0x27c, 0x286, 0x290],
            link2: 0x29d,
            flag2: 0x2a3,
            third: [0x2a8, 0x2b2, 0x2bc],
            slots: None,
        },
    )
}

/// [`start_up_0164`] (the `rel32`: the targets' set-up).
pub const START_UP_0164: Sig = Sig {
    len: 717,
    hash: 0x4a4e_e0aa_c680_5197,
    calls: &[(0x19, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C))],
    imports: &[],
};

/// `magic_0353`'s summon close (internal, cdecl, no arguments): the
/// effect's lights and task node
/// ([`lights_and_give_back`](crate::s6_misc::lights_and_give_back) on
/// `*G_S + 0x1180`), slots 346 `(*G_M)`, 399 `(*G_M, 0)`, 6 `(*G_M, 0)`,
/// 547 `(*G_M)`, then a tail jump to slot 856. Returns slot 856's `eax`.
///
/// `magic_0353` 0x10001a70 (108 bytes).
pub fn close_0353<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let e = g.read::<u32>(w(0x2)) + 0x1180;
    // push esi: the callee's return address below
    let lights = Frame {
        at: img.rel32_target(f + 0xe),
        esp: esp.wrapping_sub(8),
        eax: 0,
        ecx: e,
        edx: 0,
        xmm: [0; 4],
    };
    let node = g.read::<u32>(e + 0x228);
    crate::s6_misc::lights_and_give_back(g, img, &lights);
    // the `ecx` the lights leave (slot 346 reads it as `this`): the free
    // cursor they store after a give-back, else the effect (their slot
    // calls' `ecx`, as the game's slots leave it)
    let e = if node == 0 {
        e
    } else {
        g.read::<u32>(img.word(lights.at + 0x110))
    };
    let m = g.read::<u32>(w(0x19));
    // call width: 1
    g.call(slot(g, 0x13, 346), e, &[m]);
    let m = g.read::<u32>(w(0x2e));
    // call width: 2
    g.call(slot(g, 0x26, 399), e, &[m, 0]);
    let m = g.read::<u32>(w(0x41));
    // call width: 2
    g.call(slot(g, 0x3b, 6), e, &[m, 0]);
    // call width: 1
    g.call(slot(g, 0x4e, 547), e, &[m]);
    // jmp eax: slot 856 with the caller's stack
    // call width: 0
    g.call(slot(g, 0x5c, 856), e, &[]).eax
}

/// [`close_0353`] (the `rel32`: the lights).
pub const CLOSE_0353: Sig = Sig {
    len: 108,
    hash: 0xeaaa_7e9f_67ff_845e,
    calls: &[(0xe, Callee::Sig(&crate::s6_misc::LIGHTS_AND_GIVE_BACK))],
    imports: &[],
};

/// `magic_0353`'s script task (cdecl `(task)`, a task callback): the
/// DLL's script runs ([`run`](crate::sa_script::run)) and the motion
/// steps ([`motion_step`](crate::s3_motion::motion_step)); once the script
/// has ended, `*G_S + 0x13fc` = 1, the summon closes ([`close_0353`]) and
/// the task goes back to the pool (a tail jump to
/// [`give_back`](crate::sa_pool::give_back)). Returns the give-back's
/// `eax`, or the motion step's.
///
/// `magic_0353` 0x10001900 (54 bytes).
pub fn script_task_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = g.read::<u32>(img.word(f + 0x7));
    let ended = crate::sa_script::run(g, img, img.rel32_target(f + 0xc));
    let eax = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x13));
    if ended != 1 {
        return Ret::eax(eax);
    }
    g.write::<u32>(s + 0x13fc, ended);
    // push ebp, esi, edi: the call's return address below
    close_0353(
        g,
        img,
        img.rel32_target(f + 0x23),
        fr.esp.wrapping_sub(0x10),
    );
    let task = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x2e),
        task,
    ))
}

/// [`script_task_0353`] (the `rel32`s: the script, the motion step, the
/// close, the give-back).
pub const SCRIPT_TASK_0353: Sig = Sig {
    len: 54,
    hash: 0x1f6d_5c64_e9f4_4437,
    calls: &[
        (0xc, Callee::Sig(&crate::sa_script::RUN)),
        (0x13, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x23, Callee::Sig(&CLOSE_0353)),
        (0x2e, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0353`'s sprite fade task (cdecl `(task)`, a task callback; a
/// cookie frame, `L(o)` = `ebp - o`; `S = *G_S`): the alpha `80 (n - c) /
/// n` while the fade-in count `S + 0x13ec` lasts (counting down, `n` =
/// `S + 0x13f0`), then `80 c / n` of the fade-out `S + 0x13f4` (counting
/// down, `n` = `S + 0x13f8`) (`idiv`); the angle `S + 0x13e8` turns by
/// the constant at operand `+0x76` (a turn added below 0); the sprite
/// (slot 719 `(at +0xb6, 2, 12)`'s descriptor) drawn by
/// [`packed_sprite`](crate::o6_fam_pairs6::packed_sprite) on the view
/// `*G_V` with the matrix `L(0x44)` (slot 235, slot 329 by a quarter turn)
/// and slot 210's `L(0x84)`. With the fade-out over the task goes back to
/// the pool ([`give_back`](crate::sa_pool::give_back)). Returns `S`, or
/// the give-back's `eax`.
///
/// `magic_0353` 0x10001b70 (363 bytes).
pub fn fade_task_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(w(0x19));
    let c = g.read::<i32>(s + 0x13ec);
    g.write::<u32>(l(0x90), task);
    g.write::<u32>(l(0x88), s);
    let (alpha, junk) = if c > 0 {
        let n = g.read::<i32>(s + 0x13f0);
        let c = c - 1;
        g.write::<i32>(s + 0x13ec, c);
        // idiv: a zero count faults in the original
        (
            n.wrapping_sub(c)
                .wrapping_mul(80)
                .checked_div(n)
                .unwrap_or(0),
            c,
        )
    } else {
        let c = g.read::<i32>(s + 0x13f4).wrapping_sub(1);
        g.write::<i32>(s + 0x13f4, c);
        let n = g.read::<i32>(s + 0x13f8);
        (
            c.wrapping_mul(80).checked_div(n).unwrap_or(0),
            g.read::<i32>(s + 0x13ec),
        )
    };
    let mut a = sub(f32_at(g, s + 0x13e8), f32_at(g, w(0x76)));
    g.write::<i32>(l(0x8c), alpha);
    put(g, s + 0x13e8, a);
    if 0.0 > a {
        a = add(a, f32_at(g, w(0x94)));
        put(g, s + 0x13e8, a);
    }
    let v = g.read::<u32>(w(0xa7));
    // call width: 3
    let d = g.call(slot(g, 0xa1, 719), 0, &[w(0xb6), 2, 0xc]).eax;
    let (lo, hi) = (g.read::<u32>(d), g.read::<u32>(d + 4));
    // call width: 1
    g.call(slot(g, 0xc6, 235), 0, &[l(0x44)]);
    // call width: 3
    g.call(slot(g, 0xe2, 329), 0, &[l(0x44), l(0x44), 0x3fc9_0fdb]);
    let m = g.read::<u32>(w(0xfc));
    // call width: 2
    g.call(slot(g, 0xf6, 210), 0, &[m, l(0x84)]);
    let angle = g.read::<u32>(s + 0x13e8);
    // sub esp, 0x90; push ebx, esi, edi: the sprite's seven words below
    let b = ebp.wrapping_sub(0x90 + 12);
    let words = [
        junk.cast_unsigned(),
        l(0x44),
        lo,
        hi,
        angle,
        0,
        alpha.cast_unsigned(),
    ];
    for (i, x) in (0u32..).zip(words) {
        g.write::<u32>(b - 0x1c + 4 * i, x);
    }
    let sprite = Frame {
        at: img.rel32_target(f + 0x139),
        esp: b - 0x20,
        eax: l(0x44),
        ecx: v,
        edx: 0,
        xmm: [0; 4],
    };
    crate::o6_fam_pairs6::packed_sprite(g, img, &sprite);
    let s = g.read::<u32>(l(0x88));
    if g.read::<i32>(s + 0x13f4) > 0 {
        return Ret::eax(s);
    }
    let task = g.read::<u32>(l(0x90));
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x159),
        task,
    ))
}

/// [`fade_task_0353`] (the `rel32`s: the packed sprite, the give-back,
/// the cookie check).
pub const FADE_TASK_0353: Sig = Sig {
    len: 363,
    hash: 0x676e_2687_5961_7793,
    calls: &[
        (
            0x139,
            Callee::Sig(&crate::o6_fam_pairs6::PACKED_SPRITE_0353_13B0),
        ),
        (0x159, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x163, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0353`'s effect set-up (a table entry, cdecl, no arguments; a
/// cookie frame). With `S = *G_S` and the DLL's task pool (free head
/// `*G_FREE`, count `u16 *G_N`; busy first `*G_FIRST`, last `*G_LAST`):
/// the four records `S + 0xe60 + 0xa0 i` (colours, sizes from the
/// tables at operands `+0x63`/`+0x78`) each get a task taken from the
/// free ring (callback at operand `+0xf0`) and appended to the busy
/// list; the colour block `S + 0x1110` (1.0s and 0.5s, steps 0x1e) gets a
/// task too (callback at `+0x21a`, kind 1). Then five light records are
/// copied over (their targets `+0x10` on from their sources) and set:
/// zero steps, flags 1, the 0.65 / 0.82 / 0.68 colour and the -1.5 turn.
/// Two of their words take the frame's never-written `ebp - 0x8` and
/// `ebp - 0xc` (junk reads: 0 here, brief 2026-10-24). An empty free
/// ring makes the original write to address 0x14 / 0x18 (the port too).
/// Returns the last word copied.
///
/// `magic_0353` 0x10001ce0 (1171 bytes).
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn effect_setup_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let mut last = g.read::<u32>(w(0x11));
    let mut free = g.read::<u16>(w(0x19));
    let mut rec = g.read::<u32>(w(0x20)) + 0xe60;
    let mut head = g.read::<u32>(w(0x2f));
    g.write::<u32>(ebp - 0x20, 0);
    g.write::<u32>(ebp - 0x1c, last);
    let mut node = 0u32;
    for i in (0u32..0x10).step_by(4) {
        g.write::<u32>(rec + 0x81, 0x0080_ffff);
        g.write::<u8>(rec + 0x80, 0xff);
        g.write::<u16>(rec + 0x86, 0);
        g.write::<u8>(rec + 0x85, 0);
        let c = g.read::<u32>(w(0x63).wrapping_add(i));
        g.write::<u32>(rec + 0x88, c);
        g.write::<u16>(rec + 0x98, 0x48);
        let size = g.read::<u32>(w(0x78).wrapping_add(i));
        g.write::<u32>(rec + 0x90, size);
        g.write::<u32>(rec + 0x8c, size);
        g.write::<u32>(rec + 0x94, 0);
        node = 0;
        if free != 0 {
            let next_at = head + 4;
            g.write::<u32>(ebp - 0x18, next_at);
            let (prev, next) = (g.read::<u32>(head), g.read::<u32>(next_at));
            g.write::<u32>(prev + 4, next);
            let (p, n) = (g.read::<u32>(head), g.read::<u32>(next_at));
            node = head;
            g.write::<u32>(n, p);
            head = if free > 1 { g.read::<u32>(next_at) } else { 0 };
            g.write::<u32>(next_at, 0);
            g.write::<u32>(w(0xd2), head);
            for o in [0u32, 0xc, 0x14] {
                g.write::<u32>(node + o, 0);
            }
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 0x10, w(0xf0));
            g.write::<u8>(node + 0x18, 0);
            free = free.wrapping_sub(1);
        }
        let listed = g.read::<u32>(w(0xfd)) != 0;
        g.write::<u32>(node.wrapping_add(0x14), rec);
        g.write::<u32>(node.wrapping_add(4), 0);
        if listed {
            g.write::<u32>(node, last);
            g.write::<u32>(last.wrapping_add(4), node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0x120), node);
        }
        rec += 0xa0;
        g.write::<u32>(node.wrapping_add(8), node);
        last = node;
        g.write::<u32>(ebp - 0x1c, last);
        g.write::<u32>(ebp - 0x20, i + 4);
    }
    let s = g.read::<u32>(w(0x141));
    g.write::<u16>(w(0x148), free);
    let block = s + 0x1110;
    stores(g, img, f, &[0x152, 0x15c]);
    let one = 0x3f80_0000;
    let half = 0x3f00_0000;
    for (o, x) in [
        (8u32, one),
        (4, one),
        (0, one),
        (0xc, one),
        (0x18, half),
        (0x14, half),
        (0x10, half),
        (0x1c, one),
        (0x28, 0),
        (0x34, 0),
        (0x30, 0x1e),
        (0x2c, 0x1e),
    ] {
        g.write::<u32>(block + o, x);
    }
    let mut t = 0u32;
    if free != 0 {
        t = head;
        let (prev, next) = (g.read::<u32>(t), g.read::<u32>(t + 4));
        g.write::<u32>(prev + 4, next);
        let (next, prev) = (g.read::<u32>(t + 4), g.read::<u32>(t));
        g.write::<u32>(next, prev);
        if free > 1 {
            let next = g.read::<u32>(t + 4);
            g.write::<u32>(w(0x1db), next);
        } else {
            g.write::<u32>(w(0x1e3), 0);
        }
        free = free.wrapping_sub(1);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(t + o, 0);
        }
        g.write::<u32>(t + 8, t);
        g.write::<u32>(t + 0x10, w(0x21a));
        g.write::<u8>(t + 0x18, 0);
        g.write::<u16>(w(0x225), free);
    }
    let s = g.read::<u32>(w(0x1ed));
    let listed = g.read::<u32>(w(0x22b)) != 0;
    g.write::<u8>(t.wrapping_add(0x18), 1);
    g.write::<u32>(t.wrapping_add(0x14), block);
    g.write::<u32>(t.wrapping_add(4), 0);
    if listed {
        g.write::<u32>(t, node);
        g.write::<u32>(node.wrapping_add(4), t);
    } else {
        g.write::<u32>(t, 0);
        g.write::<u32>(w(0x24f), t);
    }
    g.write::<u32>(t.wrapping_add(8), t);
    g.write::<u32>(block + 0x10, 0x3f26_6666);
    // junk reads: [ebp-0x8] and [ebp-0xc] never written; 0 (brief 2026-10-24)
    let (j8, jc) = (0u32, 0u32);
    let light = |g: &mut G, at: u32, from: u32, x: u32| {
        copy_words(g, s + from, s + at, 4);
        for o in [0x10u32, 0x14, 0x18] {
            g.write::<u32>(s + at + o, 0);
        }
        g.write::<u32>(s + at + 0x1c, x);
        g.write::<u32>(s + at + 0x24, 1);
        g.write::<u32>(s + at + 0x20, 1);
    };
    // (the stores as the original orders them)
    copy_words(g, s + 0x1228, s + 0x1238, 4);
    for o in [0x1248u32, 0x124c, 0x1250] {
        g.write::<u32>(s + o, 0);
    }
    g.write::<u32>(s + 0x1254, j8);
    g.write::<u32>(s + 0x125c, 1);
    g.write::<u32>(s + 0x1258, 1);
    copy_words(g, s + 0x1260, s + 0x1270, 4);
    g.write::<u32>(s + 0x128c, j8);
    for o in [0x1280u32, 0x1284, 0x1288] {
        g.write::<u32>(s + o, 0);
    }
    g.write::<u32>(s + 0x1294, 1);
    g.write::<u32>(s + 0x1290, 1);
    light(g, 0x12e0, 0x12d0, j8);
    copy_words(g, s + 0x1298, s + 0x12a8, 4);
    g.write::<u32>(s + 0x12c4, j8);
    for (o, x) in [
        (0x12b8u32, one),
        (0x12bc, 0x3f51_d1d2),
        (0x12c0, 0x3f2e_aeaf),
    ] {
        g.write::<u32>(s + o, x);
    }
    g.write::<u32>(s + 0x12cc, 1);
    g.write::<u32>(s + 0x12c8, 1);
    copy_words(g, s + 0x11f0, s + 0x1200, 4);
    g.write::<u32>(s + 0x1218, jc);
    g.write::<u32>(s + 0x1210, 0xbfc0_2037);
    g.write::<u32>(s + 0x1214, 0);
    g.write::<u32>(s + 0x121c, j8);
    g.write::<u32>(w(0x46a), t);
    g.write::<u32>(s + 0x1224, 1);
    g.write::<u32>(s + 0x1220, 1);
    Ret::eax(g.read::<u32>(s + 0x11fc))
}

/// [`effect_setup_0353`] (the `rel32`: the cookie check).
pub const EFFECT_SETUP_0353: Sig = Sig {
    len: 1171,
    hash: 0xae0d_5760_5e52_67d4,
    calls: &[(0x48b, Callee::Any)],
    imports: &[],
};

/// Starts an object through `start_object_anim_2` at `call` with the
/// stack words `words` (index, -, a, b) written below `b` (the `esp`
/// before the pushes) and the scale `scale` in `xmm3`.
fn start_anim_2<G: Game>(
    g: &mut G,
    img: &Image,
    call: u32,
    b: u32,
    obj: u32,
    scale: f32,
    words: [u32; 4],
) -> u32 {
    for (i, x) in (0u32..).zip(words) {
        g.write::<u32>(b - 0x10 + 4 * i, x);
    }
    let fr = Frame {
        at: img.rel32_target(call),
        esp: b - 0x14,
        eax: 0,
        ecx: 0,
        edx: obj,
        xmm: [0, 0, 0, u128::from(scale.to_bits())],
    };
    crate::s6_misc::start_object_anim_2(g, img, &fr)
}

/// `magic_0353`'s spark task (cdecl `(task)`, a task callback): the
/// wait `task + 0x2c` counts down; at 0 it restarts at `10 + rand() % 11`
/// and a spark is taken ([`pool_take_140`](crate::sb_misc::pool_take_140)),
/// started ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2),
/// index 1) at `*G_S + 0xe50` facing `+0xe44`, with three slot 314
/// spreads (30, 10 lowered by the constant at operand `+0xcd`, 30); the
/// second goes through the task's argument word. Returns the last slot's
/// `eax`, 0 with no spark, or `eax` as it came while waiting.
///
/// `magic_0353` 0x10002200 (250 bytes).
pub fn spark_task_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let task = fr.arg(g, 0);
    let wait = g.read::<i32>(task + 0x2c).wrapping_sub(1);
    g.write::<i32>(task + 0x2c, wait);
    let s = g.read::<u32>(w(0x11));
    if wait > 0 {
        return Ret::eax(fr.eax);
    }
    let r = g.rand().cast_signed() % 11 + 10;
    g.write::<i32>(task + 0x2c, r);
    let o = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + 0x30));
    if o == 0 {
        return Ret::eax(0);
    }
    // push ebp, esi, edi
    let b = fr.esp.wrapping_sub(0xc);
    let k = f32_at(g, w(0x42));
    start_anim_2(g, img, f + 0x50, b, o, k, [1, 0, 0, 0]);
    copy_words(g, s + 0xe50, o + 0xd0, 4);
    let face = g.read::<u32>(s + 0xe44);
    g.write::<u32>(o + 0xc4, face);
    // add esp, 0xc: the first pushed word takes the spreads' argument
    let arg = b - 4;
    let spread = |g: &mut G, op: u32, x: u32| {
        g.write::<u32>(arg, x);
        // call width: 1
        let r = g.call(slot(g, op, 314), 0, &[x]);
        (crate::m0018::st0_f32(r.st0), r.eax)
    };
    let (v, _) = spread(g, 0x91, 0x41f0_0000);
    put(g, o + 0xe0, v);
    let (v, _) = spread(g, 0xae, 0x4120_0000);
    put(g, fr.esp + 4, v);
    let x = sub(f32_at(g, fr.esp + 4), f32_at(g, w(0xcd)));
    g.write::<u32>(arg, 0x41f0_0000);
    put(g, o + 0xe4, x);
    let (v, eax) = spread(g, 0xd2, 0x41f0_0000);
    put(g, o + 0xe8, v);
    Ret::eax(eax)
}

/// [`spark_task_0353`] (the `rel32`s: the pool take, the object start;
/// `rand`).
pub const SPARK_TASK_0353: Sig = Sig {
    len: 250,
    hash: 0xd381_8b16_18bc_01d5,
    calls: &[
        (0x30, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0x50, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
    ],
    imports: &[(0x1d, "rand")],
};

/// `magic_0353`'s summon object start (a table entry, no arguments): the
/// summon object `S = *G_S` started
/// ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2), index
/// 0, the animation `(500, the callback at operand +0x10)`, the scale at
/// operand `+0x4`) at `S + 0xe50` lowered by the constant at `+0x4d`,
/// facing `S + 0xe44`; the record `S + 0xe60` (colour, size 0x3c) gets a
/// task ([`take`](crate::sa_pool::take), the callback at `+0x7e`) appended
/// to the busy list. Returns the task.
///
/// `magic_0353` 0x10002460 (277 bytes).
pub fn summon_start_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0xb));
    let k = f32_at(g, w(0x4));
    // push esi
    let b = fr.esp.wrapping_sub(4);
    start_anim_2(g, img, f + 0x1f, b, s, k, [0, 0, 0x1f4, w(0x10)]);
    copy_words(g, s + 0xe50, s + 0xd0, 2);
    let y = sub(f32_at(g, s + 0xd4), f32_at(g, w(0x4d)));
    copy_words(g, s + 0xe58, s + 0xd8, 2);
    let r = s + 0xe60;
    put(g, s + 0xd4, y);
    let face = g.read::<u32>(s + 0xe44);
    g.write::<u32>(s + 0xc4, face);
    g.write::<u32>(r + 0x81, 0x0080_ffff);
    g.write::<u8>(r + 0x80, 0xff);
    g.write::<u16>(r + 0x86, 0);
    g.write::<u8>(r + 0x85, 0);
    g.write::<u32>(r + 0x88, 0);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, 0x3c);
    g.write::<u32>(r + 0x8c, 0x3c);
    g.write::<u32>(r + 0x94, 0);
    let t = crate::sa_pool::take(g, img, img.rel32_target(f + 0xd5), w(0x7e));
    let listed = g.read::<u32>(w(0xdb)) != 0;
    g.write::<u32>(t + 0x14, r);
    g.write::<u32>(t + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0xef));
        g.write::<u32>(t, last);
        g.write::<u32>(last + 4, t);
        g.write::<u32>(w(0xf9), t);
    } else {
        g.write::<u32>(t, 0);
        g.write::<u32>(w(0x108), t);
        g.write::<u32>(w(0x10d), t);
    }
    g.write::<u32>(t + 8, t);
    Ret::eax(t)
}

/// [`summon_start_0353`] (the `rel32`s: the object start, the task take).
pub const SUMMON_START_0353: Sig = Sig {
    len: 277,
    hash: 0x6407_9a68_54c6_dc67,
    calls: &[
        (0x1f, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0xd5, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0353`'s light falloff (internal: `ecx` = `n` floats out, `edx`
/// = `n` points of 16 bytes, cdecl `(n, R)`; an 8-aligned /GS frame `T`):
/// for each point, slot 234 `(T + 0x1c, R + 0x10, point)` the offset to
/// the light `R`; `len` its length (`_libm_sse2_sqrt_precise`, as a
/// double); within the radius `R + 0x20` the weight `1 - (r - len) / K`
/// while `r - len` is below `K` (operand `+0xaf`), else 0; outside it 1.
/// The argument word `n` counts down to 0. Returns `R + 0x20` (`R` for
/// no points).
fn falloff_0353<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, out: u32, pts: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; sub esp, 0x24; push ebx, esi, edi
    let t = (esp.wrapping_sub(4) & !7).wrapping_sub(0x24 + 12);
    let n_at = esp + 4;
    let rec = g.read::<u32>(esp + 8);
    if g.read::<i32>(n_at) <= 0 {
        return rec;
    }
    let (mut out, mut pt) = (out, pts);
    let r = rec + 0x20;
    loop {
        let one = f32_at(g, w(0x44));
        put(g, t + 0x10, one);
        // call width: 3
        g.call(slot_target(g, w(0x5d), 234), 0, &[t + 0x1c, r - 0x10, pt]);
        let (x, y, z) = (
            f32_at(g, t + 0x1c),
            f32_at(g, t + 0x20),
            f32_at(g, t + 0x24),
        );
        let s = add(add(mul(y, y), mul(x, x)), mul(z, z));
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
        let len = f64::from(s).sqrt() as f32;
        let radius = f32_at(g, r);
        let v = if radius > len {
            let d = sub(radius, len);
            let k = f32_at(g, w(0xaf));
            if k > d {
                let v = mul(sub(f32_at(g, w(0xc0)), div(d, k)), f32_at(g, t + 0x10));
                put(g, t + 0x10, v);
                v
            } else {
                0.0
            }
        } else {
            f32_at(g, t + 0x10)
        };
        put(g, out, v);
        out += 4;
        pt += 0x10;
        let n = g.read::<i32>(n_at).wrapping_sub(1);
        g.write::<i32>(n_at, n);
        g.write::<u32>(t + 0x14, out);
        if n == 0 {
            return r;
        }
    }
}

/// [`falloff_0353`] (the `rel32`s: `_libm_sse2_sqrt_precise`, the cookie
/// check).
pub const FALLOFF_0353: Sig = Sig {
    len: 288,
    hash: 0xfb5a_f206_09eb_2fab,
    calls: &[
        (0x96, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x118, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0353`'s lit model draw (internal, cdecl `(V, W, -, D)`; a
/// cookie frame, `L(o)` = `ebp - o`): slot 559 `(L(0x160), L(0x164),
/// L(0x15c))`, 558 `(L(0x54))`, the screen and camera matrices (723,
/// 724), slot 230 `(L(0xd4), slot 545 ())`, four slot 562 buffers (the
/// second + 0x60000 the points, the fourth + 0xf0000 the weights). For
/// each of the model `*G_M`'s `u16 +4` parts (16 bytes from `+0x10`):
/// the part's matrices (slot 941 of its two ids) combined with `V`, `W`
/// and the frame's by slot 231, slot 229 `(D + 0x10, L(0x94), D)` its
/// light point, and the falloff ([`falloff_0353`]) of its `i16 +0x12`
/// points; its groups are walked to the end. Then slot 382 `(0, 0)`.
/// Returns slot 382's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn lit_draw_0353<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let a0 = g.read::<u32>(esp + 4);
    let a1 = g.read::<u32>(esp + 8);
    let d = g.read::<u32>(esp + 0x10);
    g.write::<u32>(l(0x170), a0);
    let m = g.read::<u32>(w(0x23));
    g.write::<u32>(l(0x16c), a1);
    g.write::<u32>(l(0x168), d);
    g.write::<u32>(l(0x174), m);
    // call width: 3
    g.call(slot(g, 0x47, 559), 0, &[l(0x160), l(0x164), l(0x15c)]);
    // call width: 1
    g.call(slot(g, 0x64, 558), 0, &[l(0x54)]);
    // call width: 1
    g.call(slot(g, 0x78, 723), 0, &[l(0x154)]);
    // call width: 1
    g.call(slot(g, 0x8c, 724), 0, &[l(0x114)]);
    // call width: 0
    let r = g.call(slot(g, 0x99, 545), 0, &[]).eax;
    // call width: 2
    g.call(slot(g, 0xae, 230), 0, &[l(0xd4), r]);
    // call width: 0
    g.call(slot(g, 0xbb, 562), 0, &[]);
    // call width: 0
    let pts = g
        .call(slot(g, 0xcb, 562), 0, &[])
        .eax
        .wrapping_add(0x6_0000);
    g.write::<u32>(l(0x164), pts);
    // call width: 0
    g.call(slot(g, 0xd9, 562), 0, &[]);
    // call width: 0
    let out = g
        .call(slot(g, 0xf2, 562), 0, &[])
        .eax
        .wrapping_add(0xf_0000);
    g.write::<u32>(l(0x15c), out);
    let mut part = m + 0x10;
    g.write::<u32>(l(0x158), part);
    if g.read::<u16>(m + 4) != 0 {
        g.write::<u32>(l(0x160), d + 0x10);
        let mut i = 0u32;
        loop {
            let id = g.read::<u32>(part);
            // call width: 1
            let p = g.call(slot(g, 0x131, 941), 0, &[id]).eax;
            let groups = g.read::<u32>(p + 4).wrapping_add(p);
            let id = g.read::<u32>(part + 4);
            // call width: 1
            let mv = g.call(slot(g, 0x141, 941), 0, &[id]).eax;
            // call width: 3
            g.call(slot(g, 0x16c, 231), 0, &[l(0x94), a0, mv]);
            // call width: 3
            g.call(slot(g, 0x18b, 231), 0, &[l(0x44), l(0x114), l(0x94)]);
            // call width: 3
            g.call(slot(g, 0x1a7, 231), 0, &[l(0x44), l(0x154), l(0x44)]);
            let id = g.read::<u32>(g.read::<u32>(l(0x158)) + 4);
            // call width: 1
            let mw = g.call(slot(g, 0x1bd, 941), 0, &[id]).eax;
            // call width: 3
            g.call(slot(g, 0x1d5, 231), 0, &[l(0x44), a1, mw]);
            // call width: 3
            g.call(slot(g, 0x1f1, 231), 0, &[l(0x44), l(0xd4), l(0x44)]);
            let (dp, dd) = (g.read::<u32>(l(0x160)), g.read::<u32>(l(0x168)));
            // call width: 3
            g.call(slot(g, 0x20e, 229), 0, &[dp, l(0x94), dd]);
            let n = i32::from(g.read::<i16>(p + 0x12)).cast_unsigned();
            let (o, pt) = (g.read::<u32>(l(0x15c)), g.read::<u32>(l(0x164)));
            // add esp, 8: slot 229's first word stays; push D, n; call
            // sub esp, 0x174; three pushes; the three words; the return address
            let csp = esp.wrapping_sub(4 + 0x174 + 12 + 0xc + 4);
            g.write::<u32>(csp + 4, n);
            g.write::<u32>(csp + 8, dd);
            g.write::<u32>(csp + 0xc, dd);
            falloff_0353(g, img, img.rel32_target(f + 0x23b), csp, o, pt);
            // the part's groups walked (nothing written)
            let mut left = i32::from(g.read::<i16>(p + 0x10));
            let mut gp = groups;
            while left > 0 {
                let mut next = gp + 0x20;
                if g.read::<u8>(gp + 1) == 0x10 {
                    let k = i32::from(g.read::<i16>(gp + 2));
                    if k > 0 {
                        next = next.wrapping_add((k * 28).cast_unsigned());
                    }
                }
                left = left.wrapping_sub(i32::from(g.read::<i16>(gp + 2)));
                gp = next;
            }
            let count = u32::from(g.read::<u16>(g.read::<u32>(l(0x174)) + 4));
            part = g.read::<u32>(l(0x158)) + 0x10;
            i += 1;
            g.write::<u32>(l(0x158), part);
            if i.cast_signed() >= count.cast_signed() {
                break;
            }
        }
    }
    // call width: 2
    g.call(slot(g, 0x29c, 382), 0, &[0, 0]).eax
}

/// [`lit_draw_0353`] (the `rel32`s: the falloff, the cookie check).
pub const LIT_DRAW_0353: Sig = Sig {
    len: 704,
    hash: 0x8ff0_faf9_9782_9066,
    calls: &[(0x23b, Callee::Sig(&FALLOFF_0353)), (0x2b8, Callee::Any)],
    imports: &[],
};

/// `magic_0353`'s light flicker (internal, no arguments; a cookie frame,
/// `L(o)` = `ebp - o`; `S = *G_S`): while `S + 0x13dc` is 0, the
/// countdown `S + 0x13d4`; at 0 a new peak (slot 404 `(20)` plus the
/// constant at operand `+0x5d` into `S + 0x13d0`) and a new length `20 +
/// rand() % 21` (both counts), the flag set; the level `S + 0x13cc` is
/// `2 q p K h` (`p` the count's share, `q = 1 - p`, the end points 0).
/// With the flag set the level rises by the constant at `+0x239`, held at
/// 190 above the one at `+0x240` (the flag stays 1, else 0). Then slot
/// 857 `(*G_V, name, &x, &y, &z, &w)` for six names: the first with the
/// colour `S + 0x13bc..` and the level, the others with zeros. Returns
/// the flag.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn flicker_0353<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let v = g.read::<u32>(w(0x13));
    let s = g.read::<u32>(w(0x1a));
    let flag = if g.read::<u32>(s + 0x13dc) == 0 {
        let c = g.read::<u32>(s + 0x13d4).wrapping_sub(1);
        g.write::<u32>(s + 0x13d4, c);
        if c == 0 {
            // sub esp, 0x18; push ebx, esi, edi; push ecx: the argument
            let arg = ebp.wrapping_sub(0x18 + 12 + 4);
            g.write::<u32>(arg, 0x41a0_0000);
            // call width: 1
            let r = g.call(slot(g, 0x3d, 404), 0, &[0x41a0_0000]).st0;
            put(g, l(0x18), crate::m0018::st0_f32(r));
            let h = add(f32_at(g, l(0x18)), f32_at(g, w(0x5d)));
            put(g, s + 0x13d0, h);
            let n = g.rand().cast_signed() % 0x15 + 0x14;
            g.write::<u32>(s + 0x13dc, 1);
            g.write::<i32>(s + 0x13d8, n);
            g.write::<i32>(s + 0x13d4, n);
        }
        let p = div(
            cvt(g.read::<u32>(s + 0x13d4)),
            cvt(g.read::<u32>(s + 0x13d8)),
        );
        let q = sub(f32_at(g, w(0xb0)), p);
        let a = mul(mul(mul(q, p), f32_at(g, w(0xc7))), f32_at(g, s + 0x13d0));
        let b = mul(mul(q, q), 0.0);
        let c = mul(mul(p, p), 0.0);
        put(g, s + 0x13cc, add(add(a, b), c));
        0
    } else {
        let x = add(f32_at(g, s + 0x13cc), f32_at(g, w(0x239)));
        put(g, s + 0x13cc, x);
        if x > f32_at(g, w(0x240)) {
            g.write::<u32>(s + 0x13cc, 0x433e_0000);
            1
        } else {
            0
        }
    };
    let (px, py, pz, pw) = (l(0x14), l(0x10), l(0xc), l(0x8));
    for (i, x) in (0u32..).zip([0x13bcu32, 0x13c0, 0x13c4, 0x13cc]) {
        let v = g.read::<u32>(s + x);
        g.write::<u32>([px, py, pz, pw][i as usize], v);
    }
    // call width: 6
    g.call(slot(g, 0x124, 857), 0, &[v, w(0x141), px, py, pz, pw]);
    for (gi, name) in [
        (0x159u32, 0x180u32),
        (0x198, 0x19d),
        (0x1be, 0x1c3),
        (0x1e1, 0x1e6),
        (0x1fc, 0x210),
    ] {
        if gi == 0x159 {
            for p in [pw, pz, py, px] {
                g.write::<u32>(p, 0);
            }
        }
        // call width: 6
        g.call(slot(g, gi, 857), 0, &[v, w(name), px, py, pz, pw]);
    }
    flag
}

/// [`flicker_0353`] (the `rel32`: the cookie check; `rand`).
pub const FLICKER_0353: Sig = Sig {
    len: 609,
    hash: 0xc234_1741_18bd_445e,
    calls: &[(0x225, Callee::Any)],
    imports: &[(0x6e, "rand")],
};

/// `magic_0353`'s lit task (cdecl `(task)`, a task callback; a cookie
/// frame, `L(o)` = `ebp - o`; `S = *G_S`): the flicker ([`flicker_0353`]),
/// the matrices `L(0x84)` (slot 210 `(*G_V, ..)`) and `L(0x44)` (slot 13),
/// the scaled camera ([`scaled_camera`](crate::m0052::scaled_camera) into
/// `L(0xc4)`), the view's ranges (slots 357, 356), the lit draw
/// ([`lit_draw_0353`] `(L(0x84), L(0x44), -, S + 0x13ac)`), the ranges
/// reset; when the flicker's flag is up the task goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back)) and `S + 0x13e0` = 0.
/// Returns slot 356's `eax`, or the give-back's.
///
/// `magic_0353` 0x10002840 (259 bytes).
pub fn lit_task_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(w(0x19));
    g.write::<u32>(l(0xc8), task);
    // sub esp, 0xc8; push ebx, esi, edi: `B`
    let b = ebp.wrapping_sub(0xc8 + 12);
    let flag = flicker_0353(g, img, img.rel32_target(f + 0x26), b - 4);
    let v = g.read::<u32>(w(0x41));
    // call width: 2
    g.call(slot(g, 0x2c, 210), 0, &[v, l(0x84)]);
    let v = g.read::<u32>(w(0x52));
    // call width: 2
    g.call(slot(g, 0x4c, 13), 0, &[v, l(0x44)]);
    // the four words pushed for slots 210 and 13 stay under the camera
    for (i, x) in (1u32..).zip([l(0x84), g.read::<u32>(w(0x41)), l(0x44), v]) {
        g.write::<u32>(b - 4 * i, x);
    }
    crate::m0052::scaled_camera(g, img, img.rel32_target(f + 0x62), l(0xc4), b - 0x14);
    let v = g.read::<u32>(w(0x6d));
    // call width: 3
    g.call(slot(g, 0x67, 357), 0, &[v, u32::MAX, 0x3c00]);
    // call width: 3
    g.call(slot(g, 0x82, 356), 0, &[v, u32::MAX, 0x2ee0]);
    // add esp, 0x24: one word stays; push S + 0x13ac, ecx, L(0x44), L(0x84)
    for (i, x) in (2u32..).zip([s + 0x13ac, 0, l(0x44), l(0x84)]) {
        g.write::<u32>(b - 4 * i, x);
    }
    lit_draw_0353(g, img, img.rel32_target(f + 0xad), b - 0x18);
    // call width: 3
    g.call(slot(g, 0xb2, 357), 0, &[v, u32::MAX, u32::MAX]);
    // call width: 3
    let eax = g.call(slot(g, 0xc4, 356), 0, &[v, u32::MAX, u32::MAX]).eax;
    if flag != 1 {
        return Ret::eax(eax);
    }
    let task = g.read::<u32>(l(0xc8));
    let r = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0xe4), task);
    g.write::<u32>(s + 0x13e0, 0);
    Ret::eax(r)
}

/// [`lit_task_0353`] (the `rel32`s: the flicker, the scaled camera, the
/// lit draw, the give-back, the cookie check).
pub const LIT_TASK_0353: Sig = Sig {
    len: 259,
    hash: 0xb49b_96df_e12f_c593,
    calls: &[
        (0x26, Callee::Sig(&FLICKER_0353)),
        (0x62, Callee::Sig(&crate::m0052::SCALED_CAMERA)),
        (0xad, Callee::Sig(&LIT_DRAW_0353)),
        (0xe4, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xfb, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0353`'s lit start (a table entry, no arguments; a cookie frame):
/// `S = *G_S`: the light `S + 0x13ac` = `(0, 0, -90000, 1)`, the
/// countdown `S + 0x13d4` 1, the level and the flag 0; a task (the lit
/// task's callback at operand `+0xd4`) taken from the DLL's free ring
/// (`u16 *G_N`, `*G_FREE`; an empty ring makes the original write to
/// address 0x14) into `S + 0x13e0`, appended to the busy list; slot 855
/// `(*G_V, 1)`. Returns slot 855's `eax`.
///
/// `magic_0353` 0x10002950 (322 bytes).
pub fn lit_start_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let s = g.read::<u32>(w(0x13));
    g.write::<u64>(ebp - 0x14, 0);
    g.write::<u64>(s + 0x13ac, 0);
    let mut free = g.read::<u16>(w(0x36));
    g.write::<u32>(ebp - 0xc, 0xc7af_c800);
    g.write::<u32>(ebp - 0x8, 0x3f80_0000);
    let far = g.read::<u64>(ebp - 0xc);
    g.write::<u64>(s + 0x13b4, far);
    g.write::<u32>(s + 0x13d4, 1);
    g.write::<u32>(s + 0x13cc, 0);
    g.write::<u32>(s + 0x13dc, 0);
    let mut t = 0u32;
    if free != 0 {
        t = g.read::<u32>(w(0x7e));
        let (prev, next) = (g.read::<u32>(t), g.read::<u32>(t + 4));
        g.write::<u32>(prev + 4, next);
        let (next, prev) = (g.read::<u32>(t + 4), g.read::<u32>(t));
        g.write::<u32>(next, prev);
        if free > 1 {
            let next = g.read::<u32>(t + 4);
            g.write::<u32>(w(0x9b), next);
        } else {
            g.write::<u32>(w(0xa3), 0);
        }
        free = free.wrapping_sub(1);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(t + o, 0);
        }
        g.write::<u32>(t + 8, t);
        g.write::<u32>(t + 0x10, w(0xd4));
        g.write::<u8>(t + 0x18, 0);
        g.write::<u16>(w(0xdf), free);
    }
    let listed = g.read::<u32>(w(0xe5)) != 0;
    g.write::<u32>(s + 0x13e0, t);
    g.write::<u32>(t.wrapping_add(4), 0);
    if listed {
        let last = g.read::<u32>(w(0xfc));
        g.write::<u32>(t, last);
        g.write::<u32>(last.wrapping_add(4), t);
    } else {
        g.write::<u32>(t, 0);
        g.write::<u32>(w(0x10f), t);
    }
    g.write::<u32>(t.wrapping_add(8), t);
    let v = g.read::<u32>(w(0x125));
    g.write::<u32>(w(0x12b), t);
    // call width: 2
    Ret::eax(g.call(slot_target(g, w(0x114), 855), 0, &[v, 1]).eax)
}

/// [`lit_start_0353`] (the `rel32`: the cookie check).
pub const LIT_START_0353: Sig = Sig {
    len: 322,
    hash: 0x8215_f4ca_d7c9_f549,
    calls: &[(0x13a, Callee::Any)],
    imports: &[],
};

/// `magic_0353`'s light reset (a table entry, no arguments; `S = *G_S`):
/// the seven light records get their start colours back (four words from
/// `S + 0x1180`.. into the record, four from `S + 0x1308`.. after them) and
/// the step counts 0x1e; then the scale task of `S + 0x1148`
/// ([`scale_task_fixed`](crate::s4_small::scale_task_fixed), `xmm1`/`xmm2`
/// the constants at operands `+0x2dd`/`+0x2d5`) and its scale 0.65.
/// Returns the scale task's `eax`.
///
/// `magic_0353` 0x10002aa0 (862 bytes).
pub fn light_reset_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x3));
    for (dst, src, ext) in [
        (0x1190u32, 0x1180u32, 0x1308u32),
        (0x11c8, 0x11b8, 0x1318),
        (0x1200, 0x11f0, 0x1328),
        (0x1238, 0x1228, 0x1368),
        (0x1270, 0x1260, 0x1378),
        (0x12a8, 0x1298, 0x1388),
        (0x12e0, 0x12d0, 0x1398),
    ] {
        copy_words(g, s + src, s + dst, 4);
        copy_words(g, s + ext, s + dst + 0x10, 4);
        g.write::<u32>(s + dst + 0x24, 0x1e);
        g.write::<u32>(s + dst + 0x20, 0x1e);
    }
    let (k2, k1) = (g.read::<u32>(w(0x2d5)), g.read::<u32>(w(0x2dd)));
    // push esi, push ecx: the call's return address below
    let task = Frame {
        at: img.rel32_target(f + 0x34b),
        esp: fr.esp.wrapping_sub(0xc),
        eax: 0,
        ecx: s + 0x1148,
        edx: 0,
        xmm: [0, u128::from(k1), u128::from(k2), 0],
    };
    let r = crate::s4_small::scale_task_fixed(g, img, &task).eax;
    g.write::<u32>(s + 0x1148, 0x3f26_6666);
    Ret::eax(r)
}

/// [`light_reset_0353`] (the `rel32`: the scale task).
pub const LIGHT_RESET_0353: Sig = Sig {
    len: 862,
    hash: 0xc41d_a0ea_dbe4_1663,
    calls: &[(0x34b, Callee::Sig(&crate::s4_small::SCALE_TASK_30))],
    imports: &[],
};

/// `magic_0353`'s start-up (internal, cdecl, no arguments; called by its
/// load step): the DLL's buffers and words set, the actors looked up
/// ([`lookup_actors`](crate::sb_misc::lookup_actors)), the part (slot 719
/// `(table, 8, 0)`, [`part_setup`](crate::sb_misc::part_setup)), the
/// camera block's angles ([`angle_reset`](crate::o6_fam_pairs::angle_reset)),
/// the model's motion ([`motion_reset`](crate::m0353::motion_reset)),
/// slots 345 `(*G_M)` and 349 `(*G_M, *G_T)`, the 0x20 sparks cleared,
/// the script's words and slot 271 `(4)`; two tasks taken
/// ([`take`](crate::sa_pool::take), the script task and the light task's
/// callbacks) and appended to the busy list (kinds 1 and 2), the aim
/// records ([`aim_records`](crate::s7_misc::aim_records)) between them.
/// Returns 0.
///
/// `magic_0353` 0x10002e60 (424 bytes).
pub fn start_up_0353<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::r11_fades::stores;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    stores(g, img, f, &[0x1, 0xb, 0x15, 0x1f, 0x29, 0x33, 0x3d]);
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x48));
    // call width: 3
    let p = g.call(slot(g, 0x4d, 719), 0, &[w(0x5c), 8, 0]).eax;
    g.write::<u32>(w(0x66), p);
    // push esi; push 0 (left by the slot's add esp, 8); push the buffer
    let sp = esp.wrapping_sub(4);
    g.write::<u32>(sp - 4, 0);
    g.write::<u32>(sp - 8, w(0x6b));
    let csp = sp - 0xc;
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x70), csp);
    let at = |o: u32| Frame {
        at: img.rel32_target(f + o),
        esp: csp,
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    crate::o6_fam_pairs::angle_reset(g, img, &at(0x75));
    crate::m0353::motion_reset(g, img, &at(0x7a));
    // `ecx` is the part handle the part set-up loaded (`mov ecx, [G]` at
    // its `+0x8`; nothing after it writes `ecx`): slot 345 reads it
    let ps = img.rel32_target(f + 0x70);
    let ecx = g.read::<u32>(img.word(ps + 0xa));
    let m = g.read::<u32>(w(0x85));
    // call width: 3 (the part's two words still pushed)
    g.call(slot(g, 0x7f, 345), ecx, &[m, w(0x6b), 0]);
    let tw = g.read::<u32>(w(0x94));
    // call width: 6 (the earlier words still pushed)
    g.call(slot(g, 0x99, 349), ecx, &[m, tw, m, w(0x6b), 0]);
    stores(g, img, f, &[0xa9]);
    for i in 0..0x20u32 {
        g.write::<u32>(w(0xb4) + 0x140 * i, 0);
    }
    stores(g, img, f, &[0xd4]);
    g.write::<u32>(w(0xe0), 0);
    g.write::<u16>(w(0xe7), 0);
    g.write::<u8>(w(0xed), 1);
    g.write::<u32>(w(0xfc), 0);
    stores(g, img, f, &[0x100]);
    // call width: 1
    g.call(slot(g, 0xd0, 271), 0, &[4]);
    let t1 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x115), w(0x110));
    let listed = g.read::<u32>(w(0x11b)) != 0;
    g.write::<u8>(t1 + 0x18, 1);
    g.write::<u32>(t1 + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0x12f));
        g.write::<u32>(t1, last);
        g.write::<u32>(last + 4, t1);
    } else {
        g.write::<u32>(t1, 0);
        g.write::<u32>(w(0x141), t1);
    }
    let s = g.read::<u32>(w(0x147));
    g.write::<u32>(t1 + 8, t1);
    g.write::<u32>(w(0x155), t1);
    let aim = Frame {
        at: img.rel32_target(f + 0x15a),
        esp: sp - 4,
        eax: t1,
        ecx: s + 0x1180,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s7_misc::aim_records(g, img, &aim);
    let t2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x164), w(0x15f));
    let listed = g.read::<u32>(w(0x16a)) != 0;
    g.write::<u8>(t2 + 0x18, 2);
    g.write::<u32>(t2 + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0x17e));
        g.write::<u32>(t2, last);
        g.write::<u32>(last + 4, t2);
        g.write::<u32>(t2 + 8, t2);
        g.write::<u32>(w(0x18b), t2);
    } else {
        g.write::<u32>(t2, 0);
        g.write::<u32>(w(0x199), t2);
        g.write::<u32>(t2 + 8, t2);
        g.write::<u32>(w(0x1a1), t2);
    }
    0
}

/// [`start_up_0353`] (the `rel32`s: the actors, the part, the angles, the
/// motion, the two takes and the aim records).
pub const START_UP_0353: Sig = Sig {
    len: 424,
    hash: 0x6e9a_4e3b_d075_63c4,
    calls: &[
        (0x48, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x70, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (
            0x75,
            Callee::Sig(&crate::o6_fam_pairs::ANGLE_RESET_0353_1980),
        ),
        (0x7a, Callee::Sig(&crate::m0353::MOTION_RESET)),
        (0x115, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x15a, Callee::Sig(&crate::s7_misc::AIM_RECORDS)),
        (0x164, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

fn start_up_0353_entry<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(start_up_0353(g, img, fr.at, fr.esp))
}

/// `magic_0353`'s load step (0x100012b0, 118 bytes): the shared step
/// ([`load_step`](crate::r9_load::load_step)) with [`start_up_0353`].
pub fn load_0353<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0353_entry)
}

/// [`load_0353`].
pub const LOAD_0353: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0353))],
    imports: &[],
};

/// `magic_0351`'s glow points (internal: `ecx` = the source node `N`,
/// cdecl `(P)` four 16-byte points out; a cookie frame, `L(o)` = `ebp -
/// o`): slot 234 `(L(0x34), at operand +0x1d, N)` the axis; for each of
/// the four points at the angle `a` (0, then up by the constant at
/// `+0x19b`): slot 248 `(P_i, N, axis * a)`; the middle two also get a
/// random push (slot 266 `(L(0x24), (0, 0, 1, 0), axis)`, normalised by
/// slot 232, scaled by `(rand15 / K1 - K2) K3 K4` through slot 248) and a
/// random height step (`(rand15 / K1 - K2) K3 K5` added to `P_i.y`).
/// Returns the last slot 248's `eax`.
fn glow_points_0351<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, n: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, k: u32| slot_target(g, w(op), k);
    let k = |g: &G, op: u32| f32_at(g, w(op));
    let ebp = esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let mut p = g.read::<u32>(esp + 4);
    for (o, x) in [
        (0x48u32, 0u32),
        (0x44, 0),
        (0x40, 0x3f80_0000),
        (0x3c, 0),
        (0x38, 0x3f80_0000),
    ] {
        g.write::<u32>(l(o), x);
    }
    // call width: 3
    g.call(slot(g, 0x23, 234), 0, &[l(0x34), w(0x1d), n]);
    let mut a = 0.0f32;
    let mut eax = 0;
    for i in 0..4u32 {
        for (j, o) in (0u32..).zip([0x34u32, 0x30, 0x2c]) {
            let v = mul(f32_at(g, l(o)), a);
            put(g, l(0x14) + 4 * j, v);
        }
        // call width: 3
        eax = g.call(slot(g, 0x6e, 248), 0, &[p, n, l(0x14)]).eax;
        if i != 0 && i != 3 {
            // call width: 3
            g.call(slot(g, 0xbe, 266), 0, &[l(0x24), l(0x44), l(0x34)]);
            // call width: 2
            g.call(slot(g, 0xd0, 232), 0, &[l(0x24), l(0x24)]);
            // call width: 0
            let r = g.call(slot(g, 0xdd, 324), 0, &[]).eax & 0x7fff;
            let x = mul(
                mul(sub(div(cvt(r), k(g, 0xfc)), k(g, 0x105)), k(g, 0x114)),
                k(g, 0x11c),
            );
            for (j, o) in (0u32..).zip([0x24u32, 0x20, 0x1c]) {
                let v = mul(x, f32_at(g, l(o)));
                put(g, l(0x14) + 4 * j, v);
            }
            // call width: 3
            g.call(slot(g, 0x10a, 248), 0, &[p, p, l(0x14)]);
            // call width: 0
            let r = g.call(slot(g, 0x14d, 324), 0, &[]).eax & 0x7fff;
            let y = mul(
                mul(sub(div(cvt(r), k(g, 0x16c)), k(g, 0x174)), k(g, 0x17c)),
                k(g, 0x184),
            );
            let y = add(y, f32_at(g, p + 4));
            put(g, p + 4, y);
        }
        a = add(f32_at(g, l(0x48)), k(g, 0x19b));
        p += 0x10;
        put(g, l(0x48), a);
    }
    eax
}

/// [`glow_points_0351`] (the `rel32`: the cookie check).
pub const GLOW_POINTS_0351: Sig = Sig {
    len: 450,
    hash: 0x86d8_2945_ac16_4d1e,
    calls: &[(0x1ba, Callee::Any)],
    imports: &[],
};

/// The draw of `magic_0351`'s particle object `o` (with its slot 258 and
/// the matrices, `ecx`-free): slot 258 `(o, junk)`, slot 231 `(*A, *B,
/// *C)`, slot 230 `(*D, *E)`, `**G_P = PART`, slot 712 `(o, 0x30)` under
/// the lock. Returns slot 712's `eax`. `ops`: the operands of 258's
/// table, `C`, `B`, 231's table, `A`, `E`, 230's table, `D`, `G_P`,
/// `PART`, the lock, 712's table, the lock again.
fn particle_draw_0351<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    o: u32,
    junk: u32,
    ops: [u32; 13],
) -> u32 {
    let w = |x: u32| img.word(f + x);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // call width: 2
    g.call(slot(g, ops[0], 258), 0, &[o, junk]);
    let (c, b, a) = (
        g.read::<u32>(w(ops[1])),
        g.read::<u32>(w(ops[2])),
        g.read::<u32>(w(ops[4])),
    );
    // call width: 3
    g.call(slot(g, ops[3], 231), 0, &[a, b, c]);
    let (e, d) = (g.read::<u32>(w(ops[5])), g.read::<u32>(w(ops[7])));
    // call width: 2
    g.call(slot(g, ops[6], 230), 0, &[d, e]);
    let p = g.read::<u32>(w(ops[8]));
    g.write::<u32>(p, w(ops[9]));
    let lock = g.read::<u32>(w(ops[10]));
    let held = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    let r = g.call(slot(g, ops[11], 712), 0, &[o, 0x30]).eax;
    let lock = g.read::<u32>(w(ops[12]));
    g.write::<u32>(lock, held);
    r
}

/// `magic_0351`'s spark step (cdecl `(o)`, a particle callback): nothing
/// while `*G_OFF` is set (returns 0). The colour `o + 0x60..` (0.035,
/// 1.0); while any of the three lights (flag table at operand `+0x47`)
/// is below the constant at `+0x14` (table `+0x52`) the step ends (1).
/// Else the counter `*G_N` (slot 220 `(*G_H, 0x40, 0x100)` the first
/// time) goes up; past 40 the glow `*G_G - 12` (0..0x80) is set by slot
/// 391 `(0x60, 0x60, 0x60, glow)`. Then the object is drawn
/// ([`particle_draw_0351`], its second word the caller's `esi`: a
/// register the original saves there, 0 here). Returns 1 when slot 712
/// answered 0, else 0.
///
/// `magic_0351` 0x10001690 (343 bytes).
pub fn spark_step_0351<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    if g.read::<u32>(w(0x5)) != 0 {
        return Ret::eax(0);
    }
    let lim = f32_at(g, w(0x14));
    let o = fr.arg(g, 0);
    for x in [0x60u32, 0x64, 0x68] {
        g.write::<u32>(o + x, 0x3d0f_5c29);
    }
    g.write::<u32>(o + 0x6c, 0x3f80_0000);
    let n = g.read::<i32>(w(0x3b)).wrapping_sub(1);
    let mut i = 0i32;
    while i <= n {
        if g.read::<u32>(w(0x47).wrapping_add(4 * i.cast_unsigned())) != 0
            && lim > f32_at(g, w(0x52).wrapping_add(4 * i.cast_unsigned()))
        {
            return Ret::eax(1);
        }
        i += 1;
        if i >= 3 {
            break;
        }
    }
    let mut c = g.read::<i32>(w(0x5f));
    if c == 0 {
        let h = g.read::<u32>(w(0x7b));
        // call width: 3
        g.call(slot(g, 0x68, 220), 0, &[h, 0x40, 0x100]);
        c = g.read::<i32>(w(0x82));
    }
    c = c.wrapping_add(1);
    g.write::<i32>(w(0x8b), c);
    if c > 0x28 {
        let glow = g.read::<i32>(w(0x95)).wrapping_sub(0xc);
        // the explicit chain (docs/porting.md, "Compiler bug")
        #[allow(clippy::manual_clamp)]
        let glow = if glow < 0 {
            0
        } else if glow > 0x80 {
            0x80
        } else {
            glow
        };
        g.write::<i32>(w(0xb6), glow);
        // call width: 4
        g.call(
            slot(g, 0xbb, 391),
            0,
            &[0x60, 0x60, 0x60, glow.cast_unsigned()],
        );
    }
    // the second word: the caller's `esi`, saved there (0 here)
    let r = particle_draw_0351(
        g,
        img,
        f,
        o,
        0,
        [
            0xd1, 0xe1, 0xec, 0xe6, 0xf8, 0x100, 0x105, 0x10b, 0x118, 0x120, 0x125, 0x133, 0x141,
        ],
    );
    Ret::eax(u32::from(r == 0))
}

/// [`spark_step_0351`].
pub const SPARK_STEP_0351: Sig = Sig {
    len: 343,
    hash: 0xd9c2_80e9_43db_34a2,
    calls: &[],
    imports: &[],
};

/// `magic_0351`'s glow start (cdecl `(o)`, a particle callback): for the
/// object's index `i = (o - BASE) >> 7` within the live count `*G_N` and
/// with its flag set: slot 215 `(*G_IDS[i])` the actor, the part started
/// ([`start_part`](crate::opu_parts::start_part)) on `o` with the
/// program of slot 941, slot 268 `(actor, 0x10, o + 0x40)` its place,
/// `o + 0x50..` 0 and 1, the colour (0.089, 1.0); the light `L_i =
/// LIGHTS + 16 i` (slot 325 `(L_i, o + 0x40)`), its four glow points
/// ([`glow_points_0351`] into `PTS + 64 i`), slot 269 `(SP + 64 i, PTS,
/// 4)`, the light's level 0 and step 0.05, slot 270 `(L_i, 0, PTS, SP,
/// 4)`, the light's `w` 1.0 and slot 393 `(*G_H, L_i, 0x100)`. The
/// argument word becomes `16 i`. Returns 1.
///
/// `magic_0351` 0x10001820 (401 bytes).
pub fn glow_start_0351<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = fr.esp.wrapping_sub(4);
    let n = g.read::<i32>(w(0x7)).wrapping_sub(1);
    let o = fr.arg(g, 0);
    let i = o.wrapping_sub(w(0x14)).cast_signed() >> 7;
    g.write::<i32>(ebp - 4, i);
    if i > n {
        return Ret::eax(1);
    }
    let iu = i.cast_unsigned();
    g.write::<u32>(ebp - 8, iu.wrapping_mul(4));
    if g.read::<u32>(w(0x33).wrapping_add(iu.wrapping_mul(4))) == 0 {
        return Ret::eax(1);
    }
    let ids = g.read::<u32>(w(0x3f));
    let b = u32::from(g.read::<u8>(ids.wrapping_add(iu)));
    // call width: 1
    let actor = g.call(slot(g, 0x4a, 215), 0, &[b]).eax;
    let p = g.read::<u32>(w(0x58));
    g.write::<u32>(p, w(0x60));
    let x = g.read::<u32>(g.read::<u32>(w(0x66)) + 0x10);
    // call width: 1
    let table = g.call(slot(g, 0x6f, 941), 0, &[x]).eax;
    let program = g.read::<u32>(table + 0xc);
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x8a), o, 0x1000, program, 1);
    let q = o + 0x40;
    // call width: 3
    g.call(slot(g, 0x8f, 268), 0, &[actor, 0x10, q]);
    for (x, v) in [
        (0x50u32, 0u32),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x60, 0x3db6_45a2),
        (0x64, 0x3db6_45a2),
        (0x68, 0x3db6_45a2),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(o + x, v);
    }
    g.write::<u32>(fr.esp + 4, iu << 4);
    let light = w(0xe8).wrapping_add(iu << 4);
    // call width: 2
    g.call(slot(g, 0xed, 325), 0, &[light, q]);
    let six = iu << 6;
    let pts = w(0x105).wrapping_add(six);
    // sub esp, 8; push ebx, edi, esi; one word left over; push PTS
    let sp = ebp.wrapping_sub(8 + 12 + 4);
    // slot 268's pushed Q stays below: the glow points' never-written
    // fourth word of their offset vector
    g.write::<u32>(ebp - 44, q);
    g.write::<u32>(sp - 4, pts);
    glow_points_0351(g, img, img.rel32_target(f + 0x10d), sp - 8, light);
    let sp2 = w(0x120).wrapping_add(six);
    // call width: 3
    g.call(slot(g, 0x112, 269), 0, &[sp2, pts, 4]);
    let i4 = g.read::<u32>(ebp - 8);
    g.write::<u32>(w(0x130).wrapping_add(i4), 0);
    g.write::<u32>(w(0x13e).wrapping_add(i4), 0x3d4c_cccd);
    // call width: 5
    g.call(slot(g, 0x147, 270), 0, &[light, 0, pts, sp2, 4]);
    let i16 = g.read::<u32>(fr.esp + 4);
    g.write::<u32>(w(0x166).wrapping_add(i16), 0x3f80_0000);
    let h = g.read::<u32>(w(0x176));
    // call width: 3
    g.call(slot(g, 0x16f, 393), 0, &[h, light, 0x100]);
    Ret::eax(1)
}

/// [`glow_start_0351`] (the `rel32`s: the part start, the glow points).
pub const GLOW_START_0351: Sig = Sig {
    len: 401,
    hash: 0x4c39_db5c_278f_c579,
    calls: &[
        (0x8a, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x10d, Callee::Sig(&GLOW_POINTS_0351)),
    ],
    imports: &[],
};

/// `magic_0351`'s glow step (cdecl `(o)`, a particle callback): for the
/// object's index `i` within the live count and with its flag set: the
/// light's level `+ step` (held at 1.0 above the constant at `+0x4b`),
/// slot 270 `(L_i, level, PTS_i, SP_i, 4)`, slot 325 `(o + 0x40, L_i)`
/// with `L_i.w` 1.0, the colour (0.089, 1.0), with `*G_DIM` set the
/// object's `u16 +0x76` scaled by the constant at `+0xe5`; the object
/// drawn ([`particle_draw_0351`], its second word the caller's `esi`: 0
/// here); when slot 712 answers, slot 713 `(o)` and the part restarted
/// ([`start_part`](crate::opu_parts::start_part)). Unless a live light's
/// level is still off 1.0 (`ucomiss`: ends 1), slot 408 `(*G_H, L_k,
/// 0x100)` with `k` where the scan stopped. Returns 0, or 1.
///
/// `magic_0351` 0x100019c0 (554 bytes).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn glow_step_0351<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let n = g.read::<i32>(w(0x4)).wrapping_sub(1);
    let o = fr.arg(g, 0);
    let i = o.wrapping_sub(w(0x11)).cast_signed() >> 7;
    if i > n {
        return Ret::eax(0);
    }
    let iu = i.cast_unsigned();
    if g.read::<u32>(w(0x24).wrapping_add(4 * iu)) == 0 {
        return Ret::eax(0);
    }
    let lv = w(0x44).wrapping_add(4 * iu);
    let x = add(
        f32_at(g, w(0x34).wrapping_add(4 * iu)),
        f32_at(g, w(0x3d).wrapping_add(4 * iu)),
    );
    put(g, lv, x);
    if x > f32_at(g, w(0x4b)) {
        g.write::<u32>(lv, 0x3f80_0000);
    }
    let level = g.read::<u32>(lv);
    let six = iu << 6;
    let light = w(0x8b).wrapping_add(iu << 4);
    // call width: 5
    g.call(
        slot(g, 0x76, 270),
        0,
        &[
            light,
            level,
            w(0x70).wrapping_add(six),
            w(0x69).wrapping_add(six),
            4,
        ],
    );
    g.write::<u32>(w(0x9e).wrapping_add(iu << 4), 0x3f80_0000);
    // call width: 2
    g.call(slot(g, 0x98, 325), 0, &[o + 0x40, light]);
    let dim = g.read::<u32>(w(0xb3)) != 0;
    for (x, v) in [
        (0x60u32, 0x3db6_45a2u32),
        (0x64, 0x3db6_45a2),
        (0x68, 0x3db6_45a2),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(o + x, v);
    }
    if dim {
        let v = mul(cvt(u32::from(g.read::<u16>(o + 0x76))), f32_at(g, w(0xe5)));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the u16 store
        g.write::<u16>(o + 0x76, crate::sb_sse::cvtt(v) as u16);
    }
    let r = particle_draw_0351(
        g,
        img,
        f,
        o,
        0,
        [
            0xf2, 0x101, 0x10c, 0x106, 0x118, 0x120, 0x125, 0x12b, 0x138, 0x140, 0x145, 0x153,
            0x161,
        ],
    );
    if r != 0 {
        let p = g.read::<u32>(w(0x170));
        g.write::<u32>(p, w(0x177));
        // call width: 1
        g.call(slot(g, 0x17c, 713), 0, &[o]);
        let p = g.read::<u32>(w(0x189));
        g.write::<u32>(p, w(0x18f));
        let x = g.read::<u32>(g.read::<u32>(w(0x194)) + 0x10);
        // call width: 1
        let table = g.call(slot(g, 0x19c, 941), 0, &[x]).eax;
        let program = g.read::<u32>(table + 0xc);
        crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x1b7), o, 0x1000, program, 1);
    }
    let n = g.read::<i32>(w(0x1c0)).wrapping_sub(1);
    let one = f32_at(g, w(0x1c8));
    let mut k = 0i32;
    while k <= n {
        let ku = k.cast_unsigned();
        if g.read::<u32>(w(0x1d7).wrapping_add(4 * ku)) != 0 {
            // `ucomiss` + `jp`: ends unless an ordered equal
            #[allow(clippy::float_cmp)] // the original's exact compare
            let off = f32_at(g, w(0x1e3).wrapping_add(4 * ku)) != one;
            if off {
                return Ret::eax(1);
            }
        }
        k += 1;
        if k >= 3 {
            break;
        }
    }
    let h = g.read::<u32>(w(0x212));
    let lk = w(0x20b).wrapping_add(k.cast_unsigned() << 4);
    // call width: 3
    g.call(slot(g, 0x1f7, 408), 0, &[h, lk, 0x100]);
    Ret::eax(0)
}

/// [`glow_step_0351`] (the `rel32`s: the part start).
pub const GLOW_STEP_0351: Sig = Sig {
    len: 554,
    hash: 0x7ece_beb6_7fc2_38df,
    calls: &[(0x1b7, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// `magic_0351`'s scene start (a table entry, no arguments): the
/// effect's handle `*G_H` made once (slot 0 `(0x3015)`, slots 26
/// `(0x3015, 1)`, 387 `(H, 1)`, 385 `(H, 1)`, 386 `(H, 0)`); `*G_X` 0; the
/// colour (0.75, 1.0) and four words 0; slot 215 `(0x14)` into
/// `*G_ACT`; the record at operand `+0xe9` (its handle `H`, the callback
/// at `+0x105`) and slot 325 `(at +0x115, at +0x110)`; two of its words
/// lowered by the constant at `+0x127`. Returns 1.
///
/// `magic_0351` 0x10001e60 (340 bytes).
pub fn scene_start_0351<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let made = g.read::<u32>(w(0x2)) != 0;
    stores(g, img, f, &[0x7]);
    if !made {
        // call width: 1
        let h = g.call(slot(g, 0x14, 0), 0, &[0x3015]).eax;
        g.write::<u32>(w(0x22), h);
        // call width: 2
        g.call(slot(g, 0x27, 26), 0, &[0x3015, 1]);
        let h = g.read::<u32>(w(0x40));
        // call width: 2
        g.call(slot(g, 0x38, 387), 0, &[h, 1]);
        let h = g.read::<u32>(w(0x55));
        // call width: 2
        g.call(slot(g, 0x4d, 385), 0, &[h, 1]);
        let h = g.read::<u32>(w(0x6a));
        // call width: 2
        g.call(slot(g, 0x62, 386), 0, &[h, 0]);
    }
    stores(g, img, f, &[0x7e, 0x88, 0x92, 0x9c, 0xa6, 0xb0, 0xba, 0xc4]);
    // call width: 1
    let a = g.call(slot(g, 0x7a, 215), 0, &[0x14]).eax;
    g.write::<u32>(w(0xd9), a);
    let h = g.read::<u32>(w(0xde));
    g.write::<u64>(w(0xe9), 0);
    g.write::<u32>(w(0xee), h);
    g.write::<u64>(w(0xfb), 0);
    stores(g, img, f, &[0xff]);
    // call width: 2
    g.call(slot(g, 0xf3, 325), 0, &[w(0x115), w(0x110)]);
    let x = sub(f32_at(g, w(0x11f)), f32_at(g, w(0x127)));
    put(g, w(0x137), x);
    let x = sub(f32_at(g, w(0x13f)), f32_at(g, w(0x147)));
    put(g, w(0x14f), x);
    Ret::eax(1)
}

/// [`scene_start_0351`].
pub const SCENE_START_0351: Sig = Sig {
    len: 340,
    hash: 0x75a9_08d6_49b2_fa7c,
    calls: &[],
    imports: &[],
};

/// `magic_0351`'s camera draw (a table entry, no arguments; a cookie
/// frame, `L(o)` = `ebp - o`; `C` = the camera matrix at operand `+0x47`):
/// the glow colour (0.75, 0.75, 0.75, 1) stored, slot 235 clearing `C`,
/// `L(0x84)` and `L(0x44)`; slot 329 `(L(0x84), L(0x84), turn)` (the
/// turn from operand `+0x76`), slot 231 `(C, L(0x84), C)`, slot 395 `(C,
/// C, P)`; `L(0x44)` scaled by the colour times the constant at `+0xd5`
/// and slot 231 `(C, C, L(0x44))`; slot 211 `(*G_H, C)`, the motion
/// script step ([`motion_script_step`](crate::s6_misc::motion_script_step)).
/// Returns 1 when the word at operand `+0x14e` is 0, else 0.
///
/// `magic_0351` 0x10001fc0 (352 bytes).
pub fn camera_0351<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    crate::r11_fades::stores(g, img, f, &[0x18, 0x22, 0x2c, 0x36]);
    // call width: 1
    g.call(slot(g, 0x14, 235), 0, &[w(0x47)]);
    // call width: 1
    g.call(slot(g, 0x55, 235), 0, &[l(0x84)]);
    // call width: 1
    g.call(slot(g, 0x66, 235), 0, &[l(0x44)]);
    let turn = g.read::<u32>(w(0x76));
    // call width: 3
    g.call(slot(g, 0x8b, 329), 0, &[l(0x84), l(0x84), turn]);
    // call width: 3
    g.call(slot(g, 0xa4, 231), 0, &[w(0xa9), l(0x84), w(0x98)]);
    // call width: 3
    g.call(slot(g, 0xb6, 395), 0, &[w(0xcb), w(0xc6), w(0xbb)]);
    let k = f32_at(g, w(0xd5));
    for (dst, src) in [(0x44u32, 0xddu32), (0x30, 0xf7), (0x1c, 0x112)] {
        let v = mul(f32_at(g, w(src)), k);
        put(g, l(dst), v);
    }
    // call width: 3
    g.call(slot(g, 0xea, 231), 0, &[w(0x105), w(0x100), l(0x44)]);
    let h = g.read::<u32>(w(0x133));
    // call width: 2
    g.call(slot(g, 0x128, 211), 0, &[h, w(0x12d)]);
    let step = Frame {
        at: img.rel32_target(f + 0x143),
        esp: l(0x88),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s6_misc::motion_script_step(g, img, &step);
    Ret::eax(u32::from(g.read::<u32>(w(0x14e)) == 0))
}

/// [`camera_0351`] (the `rel32`s: the motion script step, the cookie
/// check).
pub const CAMERA_0351: Sig = Sig {
    len: 352,
    hash: 0xd1e2_aa69_114c_f971,
    calls: &[
        (0x143, Callee::Sig(&crate::s6_misc::MOTION_SCRIPT_STEP)),
        (0x158, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0351`'s glow draw (a table entry, cdecl `(o)`; a cookie frame,
/// `L(o)` = `ebp - o`): slot 325 `(L(0x14), P)` with `L(0x10)` raised by
/// the constant at operand `+0x3a`, slot 235 clearing `L(0x94)` and
/// `L(0x54)`, `L(0x54)` scaled by 0.0534, slot 395 `(L(0x94), L(0x94),
/// L(0x14))`; slot 231 `(*G_A, L(0x94), L(0x54))` and `(*G_C, *G_B,
/// *G_A)`, slot 230 `(*G_D, *G_C)`; the draw state set, the lock word
/// held at 1 for slot 712 `(o, 0x30)` and put back. Returns 1 when the
/// slot answers 0, else 0.
///
/// `magic_0351` 0x10002310 (312 bytes).
pub fn glow_draw_0351<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let o = fr.arg(g, 0);
    // call width: 2
    g.call(slot(g, 0x22, 325), 0, &[l(0x14), w(0x1c)]);
    let v = add(f32_at(g, w(0x32)), f32_at(g, w(0x3a)));
    // call width: 1
    g.call(slot(g, 0x46, 235), 0, &[l(0x94)]);
    put(g, l(0x10), v);
    // call width: 1
    g.call(slot(g, 0x5c, 235), 0, &[l(0x54)]);
    for x in [0x2cu32, 0x40, 0x54] {
        g.write::<u32>(l(x), 0x3d5a_e148);
    }
    // call width: 3
    g.call(slot(g, 0x75, 395), 0, &[l(0x94), l(0x94), l(0x14)]);
    let a = g.read::<u32>(w(0xa8));
    // call width: 3
    g.call(slot(g, 0xa2, 231), 0, &[a, l(0x94), l(0x54)]);
    let (a, b, c) = (
        g.read::<u32>(w(0xb6)),
        g.read::<u32>(w(0xc1)),
        g.read::<u32>(w(0xcd)),
    );
    // call width: 3
    g.call(slot(g, 0xbb, 231), 0, &[c, b, a]);
    let (c, d) = (g.read::<u32>(w(0xd5)), g.read::<u32>(w(0xe0)));
    // call width: 2
    g.call(slot(g, 0xda, 230), 0, &[d, c]);
    let p = g.read::<u32>(w(0xed));
    g.write::<u32>(p, w(0xf5));
    let lock = g.read::<u32>(w(0xfa));
    let old = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    let r = g.call(slot(g, 0x108, 712), 0, &[o, 0x30]).eax;
    let lock = g.read::<u32>(w(0x116));
    g.write::<u32>(lock, old);
    Ret::eax(u32::from(r == 0))
}

/// [`glow_draw_0351`] (the `rel32`: the cookie check).
pub const GLOW_DRAW_0351: Sig = Sig {
    len: 312,
    hash: 0x0530_3879_65fe_682f,
    calls: &[(0x130, Callee::Any)],
    imports: &[],
};

/// `magic_0351`'s fade (a table entry, no arguments; a cookie frame,
/// `L(o)` = `ebp - o`; `F` = the level at operand `+0x3a`, `T` = the top
/// at `+0x1b`): with the count at `+0x12` above 0, or the flag at `+0x23`
/// set and the step at `+0x2c` above 0xdc, `F` rises by the constant at
/// `+0x71`, else it falls by the one at `+0x42`; below the floor it is
/// the floor, above `T` it is `T` (a rise past `T` stores 1.0 and returns
/// 0, a rise to `T` exactly returns 0). Then the colour (`F` four times,
/// slot 353 `(L(0x14))`), the grey words 0x80 and `(T - F) K + K`
/// truncated into `L(0x24)`; slot 801 with the level over the word at
/// `+0x14e` six times (the level, or the constant at `+0x13e` when not
/// above 0), slot 785 `(level * *G_S)`, the level stored, slot 397
/// `(L(0x24))`. Returns 1.
///
/// `magic_0351` 0x100024b0 (474 bytes).
#[allow(clippy::float_cmp)] // ucomiss: exactly the top
pub fn fade_0351<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let l = |o: u32| fr.esp.wrapping_sub(4).wrapping_sub(o);
    let k1 = f32_at(g, w(0x1b));
    let rise = g.read::<i32>(w(0x12)) > 0
        || (g.read::<u32>(w(0x23)) != 0 && g.read::<i32>(w(0x2c)) > 0xdc);
    let (x, low) = if rise {
        let x = add(f32_at(g, w(0x69)), f32_at(g, w(0x71)));
        put(g, w(0x84), x);
        (x, f32_at(g, w(0x79)))
    } else {
        let x = sub(f32_at(g, w(0x3a)), f32_at(g, w(0x42)));
        put(g, w(0x55), x);
        (x, f32_at(g, w(0x4a)))
    };
    let x = if low > x {
        put(g, w(0xc9), low);
        low
    } else if x > k1 {
        if rise {
            g.write::<u32>(w(0x91), 0x3f80_0000);
            return Ret::eax(0);
        }
        put(g, w(0xc9), k1);
        k1
    } else if rise && x == k1 {
        return Ret::eax(0);
    } else {
        x
    };
    for i in 0..4u32 {
        put(g, l(0x14) + 4 * i, x);
    }
    let v = add(mul(sub(k1, x), f32_at(g, w(0xe9))), f32_at(g, w(0xff)));
    for x in [0x20u32, 0x1c, 0x18] {
        g.write::<u32>(l(x), 0x80);
    }
    g.write::<i32>(l(0x24), crate::sb_sse::cvtt(v));
    // call width: 1
    g.call(slot(g, 0x116, 353), 0, &[l(0x14)]);
    let mut level = f32_at(g, w(0x126));
    if 0.0 >= level {
        level = f32_at(g, w(0x13e));
    }
    let s = div(level, f32_at(g, w(0x14e))).to_bits();
    // call width: 6
    g.call(slot(g, 0x153, 801), 0, &[s; 6]);
    let v = mul(f32_at(g, w(0x18e)), level);
    put(g, w(0x1a8), level);
    // call width: 1
    g.call(slot(g, 0x193, 785), 0, &[v.to_bits()]);
    // call width: 1
    g.call(slot(g, 0x1b8, 397), 0, &[l(0x24)]);
    Ret::eax(1)
}

/// [`fade_0351`] (the `rel32`s: the cookie checks).
pub const FADE_0351: Sig = Sig {
    len: 474,
    hash: 0xe917_ea1c_1d39_0856,
    calls: &[
        (0xa1, Callee::Any),
        (0xba, Callee::Any),
        (0x1d2, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0351`'s start-up (a table entry, cdecl `(a)`): slot 803
/// `(0x1000000, 0x10)` split into six 0x80000-byte buffers, `a` kept;
/// the handle set (slot 718), slots 222 and 221 `(h, 0, 1000, 0x2000)`,
/// four parts from slot 719 `(h, 6, 0..2)` and `(h, 8, 0)`; the lock held
/// at 1 for slot 711 `(part, table, 0)` and put back, slot 710 `(state,
/// part, buffer, 0x80000)`, three matrices from slot 941 (the part's words
/// `+0x14`.., `+0x1c`), slots 4 `(0x3015)` and 27 `(0x3015, 1)`; slot 407
/// `(a, &count, 0)` answers the ids, each id's flag (bit 0 of slot 367
/// `(a, id, &word, 0, 0, 0, 0, 0)`'s word, the argument word) kept, and
/// the all-clear flag set when none is; the light block cleared
/// (`memset`) and set, its eight flags 1, the light at (0, -30, -54) and
/// slot 389 for its three words. Returns 1.
///
/// `magic_0351` 0x100027d0 (903 bytes).
pub fn start_up_0351<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let arg_at = fr.esp + 4;
    // call width: 2
    let base = g.call(slot(g, 0x4, 803), 0, &[0x0100_0000, 0x10]).eax;
    for (i, op) in (0u32..).zip([0x19u32, 0x23, 0x2d, 0x37, 0x41, 0x4b]) {
        g.write::<u32>(w(op), base.wrapping_add(0x8_0000 * i));
    }
    let a = g.read::<u32>(arg_at);
    g.write::<u32>(w(0x53), a);
    stores(g, img, f, &[0x67]);
    // call width: 1
    g.call(slot(g, 0x58, 718), 0, &[w(0x5d)]);
    let h = g.read::<u32>(w(0x8c));
    // call width: 4
    g.call(slot(g, 0x74, 222), 0, &[h, 0, 0x3e8, 0x2000]);
    let h = g.read::<u32>(w(0xab));
    // call width: 4
    g.call(slot(g, 0x93, 221), 0, &[h, 0, 0x3e8, 0x2000]);
    for (op, h_op, n, k, out) in [
        (0xb2u32, 0xc2u32, 6u32, 0u32, 0xc9u32),
        (0xce, 0xde, 6, 1, 0xe8),
        (0xed, 0xfd, 6, 2, 0x104),
        (0x109, 0x119, 8, 0, 0x12a),
    ] {
        let h = g.read::<u32>(w(h_op));
        // call width: 3
        let p = g.call(slot(g, op, 719), 0, &[h, n, k]).eax;
        g.write::<u32>(w(out), p);
    }
    let part = g.read::<u32>(w(0x12a));
    let lock = g.read::<u32>(w(0x121));
    let old = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(slot(g, 0x135, 711), 0, &[part, w(0x12f), 0]);
    let lock = g.read::<u32>(w(0x148));
    g.write::<u32>(lock, old);
    let (b, p) = (g.read::<u32>(w(0x153)), g.read::<u32>(w(0x15b)));
    // call width: 4
    g.call(slot(g, 0x160, 710), 0, &[w(0x165), p, b, 0x8_0000]);
    for (op, p_op, x, out) in [
        (0x17au32, 0x172u32, 0x14u32, 0x187u32),
        (0x194, 0x18c, 0x18, 0x1a1),
        (0x1ae, 0x1a6, 0x1c, 0x1bb),
    ] {
        let id = g.read::<u32>(g.read::<u32>(w(p_op)) + x);
        // call width: 1
        let m = g.call(slot(g, op, 941), 0, &[id]).eax;
        g.write::<u32>(w(out), m);
    }
    let v = g.read::<u32>(w(0x1c0));
    g.write::<u32>(w(0x1c5), v);
    // call width: 1
    g.call(slot(g, 0x1ca, 4), 0, &[0x3015]);
    // call width: 2
    g.call(slot(g, 0x1dc, 27), 0, &[0x3015, 1]);
    let a = g.read::<u32>(w(0x200));
    // call width: 3
    let mut ids = g.call(slot(g, 0x1ed, 407), 0, &[a, w(0x1fa), 0]).eax;
    let mut n = g.read::<i32>(w(0x208));
    g.write::<u32>(w(0x215), ids);
    g.write::<u32>(w(0x21d), 0);
    g.write::<u32>(w(0x21d) + 4, 0);
    stores(g, img, f, &[0x221]);
    let mut i = 0i32;
    while i < n {
        let id = u32::from(g.read::<u8>(ids.wrapping_add(i.cast_unsigned())));
        let a = g.read::<u32>(w(0x24a));
        // call width: 8
        g.call(slot(g, 0x244, 367), 0, &[a, id, arg_at, 0, 0, 0, 0, 0]);
        let v = g.read::<u32>(arg_at);
        n = g.read::<i32>(w(0x25b));
        g.write::<u32>(w(0x265).wrapping_add(4 * i.cast_unsigned()), v & 1);
        i += 1;
        if i >= n {
            break;
        }
        ids = g.read::<u32>(w(0x272));
    }
    let mut clear = 1u32;
    for k in 0..n.max(0).cast_unsigned() {
        if g.read::<u32>(w(0x295).wrapping_add(4 * k)) != 0 {
            clear = 0;
        }
    }
    g.write::<u32>(w(if n > 0 { 0x2a2 } else { 0x280 }), clear);
    stores(g, img, f, &[0x2b2]);
    g.write_bytes(w(0x2ae), &[0u8; 0x110]);
    stores(g, img, f, &[0x2c4, 0x2ce, 0x2d8]);
    let mut p = w(0x2e3);
    loop {
        g.write::<u32>(p, 1);
        p = p.wrapping_add(8);
        if p.cast_signed() >= w(0x2fa).cast_signed() {
            break;
        }
    }
    stores(g, img, f, &[0x306, 0x310, 0x31a, 0x324]);
    // call width: 2
    g.call(slot(g, 0x301, 389), 0, &[0, 0]);
    let y = g.read::<u32>(w(0x343));
    // call width: 2
    g.call(slot(g, 0x348, 389), 0, &[1, y]);
    let z = g.read::<u32>(w(0x362));
    // call width: 2
    g.call(slot(g, 0x367, 389), 0, &[2, z]);
    Ret::eax(1)
}

/// [`start_up_0351`] (the `rel32`: `memset`).
pub const START_UP_0351: Sig = Sig {
    len: 903,
    hash: 0x5ce2_e29c_54c0_fc97,
    calls: &[(0x2bd, Callee::Import("memset"))],
    imports: &[],
};

/// `magic_0317`'s scene nodes (internal to [`summon_task_0317`], case 12):
/// for two sets `i` (`esi` = 0x2c i), three nodes appended to the DLL's
/// list (`A`, `B`, `C`: an empty child list, pass byte 0xff, the list's
/// last child linked, the count up, the list's last the node, the DLL's
/// vtables), `A`'s light `(-5, -50, 1)` and height `50 - 100 i`, `B`'s
/// colour (1, 1, -1, 0.15), `C`'s scroll `(0, 0x4000, 0)`; a group on the
/// frame (`L(0x1c)`..) put into the model list ([`node_insert`](crate::nodes::node_insert)),
/// the model node's byte `+0x148` 1, a particle node ([`particle_node`](crate::sb_nodes::particle_node))
/// and an effect node ([`effect_node`](crate::sb_nodes::effect_node)) with
/// its vtable; the record at `L(0x10)` flagged and cleared.
#[allow(clippy::too_many_lines)] // the original's stores, in order
fn scene_nodes_0317<G: Game>(g: &mut G, img: &Image, f: u32, esp0: u32) {
    let w = |o: u32| img.word(f + o);
    let l = |o: u32| esp0.wrapping_add(o);
    // append NODE to the DLL's list (operands: the list's last, the list)
    let append = |g: &mut G, node: u32, ops: [u32; 4]| {
        let last = g.read::<u32>(w(ops[0]));
        if last == w(ops[1]) {
            g.write::<u32>(w(ops[2]), node);
        } else {
            g.write::<u32>(last + 4, node);
        }
        let n = g.read::<u16>(w(ops[3])).wrapping_add(1);
        g.write::<u16>(w(ops[3]), n);
    };
    let (mut rec, mut model, mut part) = (w(0xf2), w(0xe5), w(0xea));
    let mut i = 0u32;
    let mut esi = 0u32;
    while esi < 0x58 {
        g.write::<u32>(l(0x14), i);
        g.write::<u32>(l(0x18), model);
        g.write::<u32>(l(0x10), rec);
        let a = w(0x102).wrapping_add(esi);
        g.write::<u16>(w(0x109).wrapping_add(esi), 0xff);
        g.write::<u32>(w(0x111).wrapping_add(esi), a);
        g.write::<u32>(w(0x117).wrapping_add(esi), a);
        g.write::<u32>(w(0x11d).wrapping_add(esi), w(0x121));
        g.write::<u32>(w(0x127).wrapping_add(esi), w(0x12b));
        let last = g.read::<u32>(w(0x130));
        g.write::<u32>(w(0x136).wrapping_add(esi), last);
        append(g, a, [0x13b, 0x140, 0x148, 0x154]);
        let h = sub(f32_at(g, w(0x15c)), mul(cvt(i), f32_at(g, w(0x171))));
        g.write::<u32>(w(0x169), a);
        g.write::<u32>(a, w(0x177));
        g.write::<u32>(w(0x17d).wrapping_add(esi), 0xc0a0_0000);
        g.write::<u32>(w(0x18b).wrapping_add(esi), 0xc248_0000);
        g.write::<u32>(w(0x195).wrapping_add(esi), 0x3f80_0000);
        let b = w(0x19f).wrapping_add(esi);
        put(g, w(0x1a7).wrapping_add(esi), h);
        g.write::<u16>(w(0x1ae).wrapping_add(esi), 0xff);
        g.write::<u32>(w(0x1b6).wrapping_add(esi), b);
        g.write::<u32>(w(0x1bc).wrapping_add(esi), b);
        g.write::<u32>(w(0x1c2).wrapping_add(esi), w(0x1c6));
        g.write::<u32>(w(0x1cc).wrapping_add(esi), w(0x1d0));
        let last = g.read::<u32>(w(0x1d5));
        g.write::<u32>(w(0x1db).wrapping_add(esi), last);
        append(g, b, [0x1e0, 0x1e5, 0x1ed, 0x1f9]);
        g.write::<u32>(w(0x1ff), b);
        g.write::<u32>(b, w(0x205));
        for (op, v) in [
            (0x20bu32, 0x3f80_0000u32),
            (0x215, 0x3f80_0000),
            (0x21f, 0xbf80_0000),
            (0x229, 0x3e19_999a),
        ] {
            g.write::<u32>(w(op).wrapping_add(esi), v);
        }
        let c = w(0x233).wrapping_add(esi);
        g.write::<u16>(w(0x23a).wrapping_add(esi), 0xff);
        g.write::<u32>(c, w(0x242));
        g.write::<u32>(w(0x248).wrapping_add(esi), c);
        g.write::<u32>(w(0x24e).wrapping_add(esi), c);
        g.write::<u32>(w(0x254).wrapping_add(esi), w(0x258));
        g.write::<u32>(w(0x25e).wrapping_add(esi), w(0x262));
        let last = g.read::<u32>(w(0x267));
        g.write::<u32>(w(0x26d).wrapping_add(esi), last);
        append(g, c, [0x272, 0x277, 0x27f, 0x28f]);
        g.write::<u32>(l(0x28), a);
        g.write::<u32>(w(0x29d), c);
        g.write::<u32>(w(0x2a3).wrapping_add(esi), 0);
        g.write::<u32>(w(0x2ad).wrapping_add(esi), 0x4000);
        g.write::<u32>(w(0x2b7).wrapping_add(esi), 0);
        for (o, v) in [
            (0x1cu32, model),
            (0x20, w(0x2c7)),
            (0x24, w(0x2cf)),
            (0x2c, c),
            (0x30, b),
        ] {
            g.write::<u32>(l(o), v);
        }
        let call = |o: u32| Frame {
            at: img.rel32_target(f + o),
            esp: esp0.wrapping_sub(4),
            eax: 0,
            ecx: l(0x1c),
            edx: 0,
            xmm: [0; 4],
        };
        crate::nodes::node_insert(g, img, &call(0x2dc));
        let mdl = g.read::<u32>(l(0x18));
        let e = g.read::<u32>(w(0x2e5));
        g.write::<u8>(mdl + 0x148, 1);
        for (o, v) in [
            (0x1cu32, part),
            (0x20, w(0x2fc)),
            (0x24, mdl),
            (0x28, e),
            (0x2c, w(0x30c)),
            (0x30, 1),
        ] {
            g.write::<u32>(l(o), v);
        }
        g.write::<u32>(part, w(0x31a));
        crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x31f), l(0x1c));
        let r = g.read::<u32>(l(0x10)).wrapping_sub(0x2c);
        for (o, v) in [
            (0x1cu32, r),
            (0x20, w(0x336)),
            (0x24, mdl),
            (0x28, w(0x342)),
            (0x2c, w(0x34a)),
            (0x30, 2),
        ] {
            g.write::<u32>(l(o), v);
        }
        crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x357), l(0x1c));
        let rr = g.read::<u32>(l(0x10));
        let node = g.read::<u32>(l(0x1c));
        i = g.read::<u32>(l(0x14)).wrapping_add(1);
        g.write::<u32>(l(0x14), i);
        g.write::<u32>(node, w(0x369));
        let fl = g.read::<u32>(rr) | 1;
        g.write::<u32>(rr, fl);
        g.write::<u32>(rr + 4, 0);
        rec = rr + 0x38;
        esi += 0x2c;
        model = mdl.wrapping_add(0x14c);
        part = part.wrapping_add(0x100);
        g.write::<u32>(l(0x10), rec);
        g.write::<u32>(l(0x18), model);
    }
}

/// `magic_0317`'s light tail (cases 14 and 0): the light block's levels
/// `1 - x` (`H` = 1.0, case 14) or `0.5 - x` and `1 - x` (`H` = 0.5, case
/// 0) and its constants (operands as `ops`).
fn light_levels_0317<G: Game>(g: &mut G, img: &Image, f: u32, ops: &[(u32, u32, u32)]) {
    let w = |o: u32| img.word(f + o);
    for &(dst, a, b) in ops {
        let x = if a < 0x1000 {
            f32_at(g, w(a))
        } else {
            f32::from_bits(a)
        };
        put(g, w(dst), sub(x, f32_at(g, w(b))));
    }
}

/// `magic_0317`'s summon task (a callback, cdecl `(t)`; an aligned
/// cookie frame, `L(o)` = the frame's `esp + o`; `R = *(t + 4)`, its
/// state `+0x18` and count `+0x2c`):
/// - 0: the scene set up (the model nodes, the summon node, the colour
///   node, two nodes appended, the model's position and facing, the
///   light node inserted; the light levels at 0.5), state 10;
/// - 10: the flag slots, a node appended, slot 262 for the light
///   position, an effect node; the count restarts;
/// - 11 (count 190 on), 13 (count 10 on): an effect node; the words
///   `+0x28`/`+0x2c` cleared;
/// - 12 (count 200 on): the scene nodes ([`scene_nodes_0317`]) and the
///   per-light flags set;
/// - 14: each light whose flag is set and whose aim (slot 394 on the
///   model's position from slot 262 and slot 12) is at or below -0.9
///   gets its nodes (heading, append, insert, particle node, slot 239)
///   and its flag cleared; with every flag clear (or count 30 on) the
///   light levels at 1.0 and the next state;
/// - 15: slot 20 or 411 (by the flag) until it answers, then 10000;
/// - 10000: the summon node leaves, its model set up, the task unlinked.
///
/// Every way out counts `+0x2c` up. Returns the last value in `eax`.
///
/// `magic_0317` 0x10006bc0 (3108 bytes).
#[allow(clippy::too_many_lines)] // the original's cases, in order
pub fn summon_task_0317<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -8; sub esp, 0x74; push ebx, esi, edi
    let esp0 = (fr.esp.wrapping_sub(4) & !7)
        .wrapping_sub(0x74)
        .wrapping_sub(12);
    let l = |o: u32| esp0.wrapping_add(o);
    let t = fr.arg(g, 0);
    let r = g.read::<u32>(t + 4);
    g.write::<u32>(l(0x3c), r);
    let at = |o: u32, esp: u32, ecx: u32, edx: u32| Frame {
        at: img.rel32_target(f + o),
        esp,
        eax: 0,
        ecx,
        edx,
        xmm: [0; 4],
    };
    let count_up = |g: &mut G| {
        let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
        g.write::<u32>(r + 0x2c, c);
    };
    let step_up = |g: &mut G| {
        let s = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, s);
        let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
        g.write::<u32>(r + 0x2c, c);
    };
    let count = |g: &G| g.read::<i32>(r + 0x2c);
    let mut state = g.read::<u32>(r + 0x18);
    loop {
        if state <= 0xa {
            if state == 0xa {
                return Ret::eax(state_a_0317(g, img, f, esp0, r));
            }
            if state != 0 {
                count_up(g);
                return Ret::eax(state);
            }
            return Ret::eax(state_0_0317(g, img, f, esp0, r));
        }
        if state > 0x2710 {
            count_up(g);
            return Ret::eax(state);
        }
        if state == 0x2710 {
            let node = w(0x8ef);
            crate::sa_misc::leave(g, node, state);
            g.write::<u32>(l(0x34), node);
            crate::sa_summon::model_setup(g, img, &at(0x901, esp0.wrapping_sub(4), l(0x34), 0));
            let n = g.read::<u32>(t);
            let next = g.read::<u32>(n + 4);
            let prev = g.read::<u32>(n + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(n + 8);
            let next = g.read::<u32>(n + 4);
            g.write::<u32>(prev + 4, next);
            g.write::<u32>(n + 8, n);
            g.write::<u32>(n + 4, n);
            count_up(g);
            return Ret::eax(next);
        }
        let k = state - 0xb;
        match k {
            // 11
            0 => {
                if count(g) < 0xbe {
                    count_up(g);
                    return Ret::eax(0);
                }
                let sp = esp0.wrapping_sub(0x10);
                for (i, v) in (0u32..).zip([w(0xaa), w(0xa5), fr.ecx, 1]) {
                    g.write::<u32>(sp + 4 * i, v);
                }
                let e = crate::s3_nodes::effect_node_with(g, img, &at(0xb4, sp - 4, w(0xaf), 0));
                let v = g.read::<u32>(w(0xba)) | 1;
                g.write::<u32>(w(0xba), v);
                crate::r11_fades::stores(g, img, f, &[0xbf]);
                step_up(g);
                return Ret::eax(e);
            }
            // 12
            1 => {
                if count(g) < 0xc8 {
                    count_up(g);
                    return Ret::eax(1);
                }
                scene_nodes_0317(g, img, f, esp0);
                let v = g.read::<u32>(w(0x3a0)).wrapping_add(0x2000);
                g.write::<u32>(w(0x3a0), v);
                let v = g.read::<u32>(w(0x3aa)).wrapping_sub(0x2000);
                g.write::<u32>(w(0x3aa), v);
                let mut i = 0i32;
                if g.read::<i32>(w(0x3ba)) > 0 {
                    loop {
                        g.write::<u32>(r + 0x30 + 4 * i.cast_unsigned(), 1);
                        i += 1;
                        if i >= g.read::<i32>(w(0x3cf)) {
                            break;
                        }
                    }
                }
                let hi = g.read::<u32>(w(0x3d6));
                crate::r11_fades::stores(g, img, f, &[0x3da]);
                g.write::<u16>(w(0x3e7), 0xff);
                crate::r11_fades::stores(g, img, f, &[0x3ed, 0x3f7, 0x401, 0x40b]);
                g.write::<u32>(w(0x416), hi);
                if hi == w(0x41b) {
                    crate::r11_fades::stores(g, img, f, &[0x421]);
                } else {
                    g.write::<u32>(hi + 4, w(0x430));
                }
                let ax = g.read::<u16>(w(0x436)).wrapping_add(1);
                crate::r11_fades::stores(g, img, f, &[0x441, 0x44b, 0x455, 0x45f]);
                g.write::<u16>(w(0x46c), 0xff);
                crate::r11_fades::stores(g, img, f, &[0x472, 0x47c, 0x486, 0x490, 0x49a]);
                if w(0x43b) == w(0x4a6) {
                    crate::r11_fades::stores(g, img, f, &[0x4ac]);
                } else {
                    crate::r11_fades::stores(g, img, f, &[0x4b8]);
                }
                let ax = ax.wrapping_add(1);
                crate::r11_fades::stores(g, img, f, &[0x4c4]);
                g.write::<u16>(w(0x4d0), ax);
                crate::r11_fades::stores(g, img, f, &[0x4d4, 0x4de, 0x4e8, 0x4f2]);
                g.write::<u8>(w(0x4fe), 1);
                g.write::<u32>(r + 0x2c, 0);
                step_up(g);
                return Ret::eax((hi & 0xffff_0000) | u32::from(ax));
            }
            // 13
            2 => {
                if count(g) >= 0xa {
                    g.write::<u32>(r + 0x28, 0);
                    g.write::<u32>(r + 0x2c, 0);
                    step_up(g);
                } else {
                    count_up(g);
                }
                return Ret::eax(2);
            }
            // 14
            3 => return Ret::eax(lights_0317(g, img, f, esp0, r)),
            // 15
            4 => {
                let (n, a) = if g.read::<u8>(w(0x56)) & 1 == 0 {
                    (411, g.read::<u32>(w(0x6f)))
                } else {
                    (20, g.read::<u32>(w(0x64)))
                };
                // call width: 1
                let v = g.call(slot(g, 0x5c, n), 0, &[a]).eax;
                if v == 0 {
                    count_up(g);
                    return Ret::eax(0);
                }
                g.write::<u32>(r + 0x18, 0x2710);
                state = 0x2710;
            }
            _ => {
                count_up(g);
                return Ret::eax(k);
            }
        }
    }
}

/// [`summon_task_0317`]'s case 14 (see there).
fn lights_0317<G: Game>(g: &mut G, img: &Image, f: u32, esp0: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let l = |o: u32| esp0.wrapping_add(o);
    g.write::<u32>(l(0x18), 0);
    g.write::<u32>(l(0x40), 0);
    let ran = g.read::<i32>(w(0x52e)) > 0;
    if ran {
        let (mut any, mut n);
        g.write::<u32>(l(0x10), w(0x558));
        g.write::<u32>(l(0x14), w(0x560));
        let (mut edi, mut esi, mut flag) = (w(0x565), 0u32, r + 0x30);
        loop {
            let m = g.read::<u32>(edi);
            for (i, v) in (1u32..).zip([l(0x64), l(0x60), l(0x5c), m]) {
                g.write::<u32>(esp0.wrapping_sub(4 * i), v);
            }
            // call width: 4
            g.call(slot(g, 0x580, 262), 0, &[m, l(0x5c), l(0x60), l(0x64)]);
            g.write::<u32>(esp0.wrapping_sub(0x14), m);
            // call width: 1
            let h = crate::m0018::st0_f32(g.call(slot(g, 0x58f, 12), 0, &[m]).st0);
            put(g, l(0x60), h);
            // call width: 3
            g.call(slot(g, 0x5ad, 234), 0, &[l(0x5c), l(0x5c), w(0x5a6)]);
            // call width: 2
            g.call(slot(g, 0x5c3, 232), 0, &[l(0x5c), l(0x5c)]);
            // call width: 2
            g.call(slot(g, 0x5da, 232), 0, &[l(0x6c), w(0x5d4)]);
            if g.read::<u32>(flag) != 0 {
                // call width: 2
                let s = g.call(slot(g, 0x5fd, 394), 0, &[l(0x5c), l(0x6c)]);
                let v = crate::m0018::st0_f32(s.st0);
                put(g, l(0x34), v);
                let k = f32_at(g, w(0x611));
                if !(k < v || v.is_nan() || k.is_nan()) {
                    lit_light_0317(g, img, f, esp0, s.eax, edi, esi);
                    g.write::<u32>(flag, 0);
                }
            }
            any = g.read::<u32>(l(0x18)) | g.read::<u32>(flag);
            n = g.read::<u32>(l(0x40)).wrapping_add(1);
            let v = g.read::<u32>(l(0x14)).wrapping_add(0x100);
            g.write::<u32>(l(0x14), v);
            let v = g.read::<u32>(l(0x10)).wrapping_add(0x14c);
            g.write::<u32>(l(0x10), v);
            flag += 4;
            esi += 0x2c;
            edi = edi.wrapping_add(0x98);
            g.write::<u32>(l(0x18), any);
            g.write::<u32>(l(0x40), n);
            if n.cast_signed() >= g.read::<i32>(w(0x79f)) {
                break;
            }
        }
        if any != 0 && g.read::<i32>(r + 0x2c) < 0x1e {
            let v = g.read::<u32>(w(0x8d1)).wrapping_sub(0x1000);
            g.write::<u32>(w(0x8d1), v);
            let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
            g.write::<u32>(r + 0x2c, c);
            return n;
        }
    }
    let one = if ran { 0x7bf } else { 0x537 };
    let one = f32_at(g, w(one)).to_bits();
    light_levels_0317(g, img, f, &[(0x7e7, one, 0x7ca)]);
    let fl = (g.read::<u32>(w(0x7cf)) & !1) | 4;
    crate::r11_fades::stores(g, img, f, &[0x7d9]);
    light_levels_0317(g, img, f, &[(0x822, one, 0x7f2)]);
    crate::r11_fades::stores(g, img, f, &[0x7f6, 0x800, 0x80a, 0x814]);
    light_levels_0317(g, img, f, &[(0x858, one, 0x82d)]);
    crate::r11_fades::stores(g, img, f, &[0x831, 0x83b, 0x845]);
    g.write::<u32>(w(0x850), fl);
    light_levels_0317(g, img, f, &[(0x875, one, 0x863)]);
    crate::r11_fades::stores(g, img, f, &[0x867]);
    light_levels_0317(
        g,
        img,
        f,
        &[
            (0x890, one, 0x880),
            (0x8a8, one, 0x888),
            (0x8b0, 0x898, 0x8a0),
        ],
    );
    light_levels_0317(g, img, f, &[(0x8c8, 0x8b8, 0x8c0)]);
    let s = g.read::<u32>(r + 0x18).wrapping_add(1);
    g.write::<u32>(r + 0x18, s);
    let v = g.read::<u32>(w(0x8d1)).wrapping_sub(0x1000);
    g.write::<u32>(w(0x8d1), v);
    let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
    g.write::<u32>(r + 0x2c, c);
    fl
}

/// One light of [`lights_0317`] lit (`edi` the light's record, `esi` =
/// 0x2c times its index; `eax` slot 394's).
fn lit_light_0317<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp0: u32,
    eax: u32,
    edi: u32,
    esi: u32,
) {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let l = |o: u32| esp0.wrapping_add(o);
    let at = |o: u32, ecx: u32| Frame {
        at: img.rel32_target(f + o),
        esp: esp0.wrapping_sub(4),
        eax: 0,
        ecx,
        edx: 0,
        xmm: [0; 4],
    };
    let list = w(0x62a);
    crate::sa_misc::node_append(
        g,
        img,
        img.rel32_target(f + 0x62f),
        w(0x625).wrapping_add(esi),
        list,
        eax,
    );
    g.write::<u32>(l(0x1c), edi.wrapping_sub(0x54));
    g.write::<u32>(l(0x20), w(0x63c).wrapping_add(esi));
    g.write::<u32>(l(0x24), w(0x64c));
    let h = crate::s4_small::heading_to(g, img, &at(0x651, l(0x1c))).eax;
    let p = w(0x657).wrapping_add(esi);
    let v = g.read::<u32>(p).wrapping_add(0xffff_e000);
    g.write::<u32>(p, v);
    let e = crate::sb_misc::node_append(
        g,
        img,
        img.rel32_target(f + 0x66b),
        w(0x661).wrapping_add(esi),
        w(0x666),
        h,
    );
    let mut eax = e;
    if g.read::<u8>(edi + 0x3c) & 1 == 0 {
        let m = g.read::<u32>(edi.wrapping_sub(4));
        // call width: 2
        eax = g
            .call(slot(g, 0x67d, 366), 0, &[m, w(0x677).wrapping_add(esi)])
            .eax;
    }
    let d = w(0x691).wrapping_add(esi);
    let dl = w(0x696);
    g.write::<u32>(w(0x69c).wrapping_add(esi), 0x3f80_0000);
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x6a5), d, dl, eax);
    let model = g.read::<u32>(l(0x10));
    for (op, v) in [
        (0x6cbu32, 0x3f80_0000u32),
        (0x6d5, 0x3f80_0000),
        (0x6df, 0x3f80_0000),
        (0x6e9, 0x3dcc_cccd),
    ] {
        g.write::<u32>(w(op).wrapping_add(esi), v);
    }
    for (o, v) in [
        (0x44u32, model),
        (0x48, dl),
        (0x4c, w(0x6f9)),
        (0x50, w(0x6b3).wrapping_add(esi)),
        (0x54, w(0x6bd).wrapping_add(esi)),
        (0x58, d),
    ] {
        g.write::<u32>(l(o), v);
    }
    crate::nodes::node_insert(g, img, &at(0x702, l(0x44)));
    let model = g.read::<u32>(l(0x10));
    let part = g.read::<u32>(l(0x14));
    g.write::<u8>(model + 0x148, 1);
    let e = g.read::<u32>(w(0x71a));
    g.write::<u32>(part, w(0x724));
    for (o, v) in [
        (0x44u32, part),
        (0x48, w(0x730)),
        (0x4c, model),
        (0x50, e),
        (0x54, w(0x73c)),
        (0x58, 0),
    ] {
        g.write::<u32>(l(o), v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x749), l(0x44));
    let (h, m) = (g.read::<u32>(w(0x75f)), g.read::<u32>(edi.wrapping_sub(4)));
    // call width: 3
    g.call(slot(g, 0x74e, 239), 0, &[h, m, 0]);
}

/// [`summon_task_0317`]'s case 0 (see there).
fn state_0_0317<G: Game>(g: &mut G, img: &Image, f: u32, esp0: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let l = |o: u32| esp0.wrapping_add(o);
    let at = |o: u32, ecx: u32, edx: u32| Frame {
        at: img.rel32_target(f + o),
        esp: esp0.wrapping_sub(4),
        eax: 0,
        ecx,
        edx,
        xmm: [0; 4],
    };
    let a = g.read::<u32>(w(0x943));
    g.write::<u32>(l(0x1c), w(0x94f));
    g.write::<u32>(l(0x20), w(0x957));
    g.write::<u32>(l(0x24), a);
    g.write::<u32>(l(0x28), 0);
    g.write::<u8>(l(0x2c), 1);
    crate::r11_fades::stores(g, img, f, &[0x96c]);
    crate::s3_summon::model_nodes(g, img, &at(0x977, l(0x1c), 0));
    let a = g.read::<u32>(w(0x97c));
    g.write::<u32>(l(0x1c), w(0x988));
    g.write::<u32>(l(0x20), w(0x990));
    g.write::<u32>(l(0x24), a);
    g.write::<u8>(l(0x28), 0);
    crate::r11_fades::stores(g, img, f, &[0x99d]);
    crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0x9a8), l(0x1c));
    g.write::<u32>(l(0x34), w(0x9b4));
    g.write::<u32>(l(0x38), w(0x9bc));
    crate::r11_fades::stores(g, img, f, &[0x9c0]);
    let e = crate::s3_misc::colour_node(g, img, &at(0x9cb, l(0x34), 0));
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x9da), w(0x9d5), w(0x9d0), e);
    let e = crate::sa_misc::model_position(g, img, img.rel32_target(f + 0x9e9), w(0x9e4), w(0x9df));
    let v = sub(f32_at(g, w(0x9f1)), f32_at(g, w(0x9f9)));
    crate::r11_fades::stores(g, img, f, &[0xa07]);
    put(g, w(0xa15), v);
    crate::sa_misc::node_append(g, img, img.rel32_target(f + 0xa1a), w(0xa03), w(0x9fe), e);
    crate::sa_misc::facing_int(
        g,
        img,
        img.rel32_target(f + 0xa29),
        w(0xa24),
        w(0xa1f),
        esp0.wrapping_sub(4),
    );
    let sp = esp0.wrapping_sub(0x10);
    for (i, op) in (0u32..).zip([0xa3du32, 0xa38, 0xa33, 0xa2e]) {
        g.write::<u32>(sp + 4 * i, w(op));
    }
    let mut fr = at(0xa4c, w(0xa47), w(0xa42));
    fr.esp = sp - 4;
    let eax = crate::sa_misc::insert_with(g, img, &fr);
    let half = f32_at(g, w(0xa54)).to_bits();
    light_levels_0317(g, img, f, &[(0xa85, half, 0xa5f)]);
    let v = g.read::<u32>(w(0xa68)) & !1;
    g.write::<u32>(w(0xa68), v);
    crate::r11_fades::stores(g, img, f, &[0xa6d, 0xa77]);
    light_levels_0317(g, img, f, &[(0xac0, half, 0xa90)]);
    crate::r11_fades::stores(g, img, f, &[0xa94, 0xa9e, 0xaa8, 0xab2]);
    light_levels_0317(g, img, f, &[(0xafb, half, 0xacb)]);
    crate::r11_fades::stores(g, img, f, &[0xacf, 0xad9, 0xae3, 0xaed]);
    light_levels_0317(g, img, f, &[(0xb0e, half, 0xb06)]);
    let one = f32_at(g, w(0xb31)).to_bits();
    light_levels_0317(
        g,
        img,
        f,
        &[
            (0xb29, half, 0xb19),
            (0xb39, half, 0xb21),
            (0xb54, one, 0xb44),
        ],
    );
    light_levels_0317(g, img, f, &[(0xb5c, one, 0xb4c)]);
    g.write::<u32>(r + 0x18, 0xa);
    let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
    g.write::<u32>(r + 0x2c, c);
    eax
}

/// [`summon_task_0317`]'s case 10 (see there).
fn state_a_0317<G: Game>(g: &mut G, img: &Image, f: u32, esp0: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let e = crate::o4_fam2::flag_slots(g, img, img.rel32_target(f + 0xb82), w(0xb7d));
    let node = w(0xb8c);
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xb91), node, w(0xb87), e);
    let h = g.read::<u32>(w(0xbb1));
    // call width: 4
    g.call(slot(g, 0xb96, 262), 0, &[h, w(0xbab), w(0xba6), w(0xb9b)]);
    crate::r11_fades::stores(g, img, f, &[0xbba]);
    let sp = esp0.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([w(0xbcd), w(0xbc8), node, 0]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let fr = Frame {
        at: img.rel32_target(f + 0xbd7),
        esp: sp - 4,
        eax: 0,
        ecx: w(0xbd2),
        edx: 0,
        xmm: [0; 4],
    };
    let e = crate::s3_nodes::effect_node_with(g, img, &fr);
    let v = g.read::<u32>(w(0xbdd)) | 1;
    g.write::<u32>(w(0xbdd), v);
    crate::r11_fades::stores(g, img, f, &[0xbe2]);
    g.write::<u32>(r + 0x2c, 0);
    let s = g.read::<u32>(r + 0x18).wrapping_add(1);
    g.write::<u32>(r + 0x18, s);
    let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
    g.write::<u32>(r + 0x2c, c);
    e
}

/// [`summon_task_0317`] (the `rel32`s: the nodes, the cookie checks).
pub const SUMMON_TASK_0317: Sig = Sig {
    len: 3108,
    hash: 0xa8ab_71d9_31cc_80f4,
    calls: &[
        (0xb4, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x2dc, Callee::Sig(&crate::nodes::INSERT)),
        (0x31f, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x357, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x62f, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x651, Callee::Sig(&crate::s4_small::HEADING_TO)),
        (0x66b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6a5, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x702, Callee::Sig(&crate::nodes::INSERT)),
        (0x749, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x8e6, Callee::Any),
        (0x8f4, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x901, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x92c, Callee::Any),
        (0x977, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x9a8, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x9cb, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x9da, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x9e9, Callee::Sig(&crate::sa_misc::MODEL_POSITION)),
        (0xa1a, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xa29, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0xa4c, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0xb74, Callee::Any),
        (0xb82, Callee::Sig(&crate::o4_fam2::FLAG_SLOTS_0317_6A40)),
        (0xb91, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xbd7, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xc06, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0295`'s summon task (a callback, cdecl `(t)`; `R = *(t + 4)`,
/// its state `+0x18`, timer `+0x20`): the timers `+0x1c`/`+0x20` step by
/// the constant at operand `+0x17`, `+0x28` counts up; then by the state
/// (a byte table at operand `+0x55` into a jump table at `+0x5c`, states
/// up to 0x3e; each case runs on into the next):
/// - 0: the scale node, the flag byte copied, slot 215 `(0x14)` kept,
///   the node under its parent, state 10;
/// - 10: slot 228 `(0x14, 0xb0)`, slot 701 (the eleven words with 70.0
///   and 1.0) and slot 537, state 0x14;
/// - 0x14: slot 411 `(0x14)` answering, slot 228 `(0x14, 0xb1)`, state
///   0x1e and the timer 0;
/// - 0x1e (the timer at 1): the nodes appended and inserted, a particle
///   node and two effect nodes; state 0x28;
/// - 0x28: slot 411 answering, slot 228 `(0x14, 0xb2)`, a node and slot
///   325; state 0x32;
/// - 0x29, 0x32 (the timer at 0.4): a node inserted, a particle node,
///   slot 239 `(*G, -5, 0)`; the state one up, the timer 0;
/// - 0x33 (the timer at 1): slot 228 `(0x14, 0xb3)`, state 0x3c;
/// - 0x3c: slot 411 answering, slot 228 `(0x14, 0xb4)`, slot 701 (with
///   0.7); 0x3d: slot 411, slot 228 `(0x14, 0xb5)`; 0x3e: slot 411, state
///   10000;
/// - 10000: slot 213 `(0x14, 0)` and the task unlinked.
///
/// Returns the last value in `eax` (the state, the table's byte, a
/// slot's answer or the task's next link).
///
/// `magic_0295` 0x100075d0 (1279 bytes).
#[allow(clippy::too_many_lines)] // the original's cases, in order
pub fn summon_task_0295<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let t = fr.arg(g, 0);
    let r = g.read::<u32>(t + 4);
    // push ebp; and esp, -8; push esi, edi
    let esp0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
    let step = f32_at(g, w(0x17));
    let v = add(f32_at(g, r + 0x1c), step);
    let state = g.read::<u32>(r + 0x18);
    let c = g.read::<u32>(r + 0x28).wrapping_add(1);
    g.write::<u32>(r + 0x28, c);
    put(g, r + 0x1c, v);
    let v = add(f32_at(g, r + 0x20), f32_at(g, w(0x2f)));
    put(g, r + 0x20, v);
    let at = |o: u32, ecx: u32, edx: u32, esp: u32| Frame {
        at: img.rel32_target(f + o),
        esp,
        eax: 0,
        ecx,
        edx,
        xmm: [0; 4],
    };
    // the four words of a node call (`push`es and `sub esp, 8`)
    let four = |g: &mut G, words: [u32; 4]| {
        let sp = esp0.wrapping_sub(0x10);
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(sp + 4 * i, v);
        }
        sp - 4
    };
    // `push A; sub esp, 8; push B`: the words between are not read
    let ends = |g: &mut G, a: u32, d: u32| {
        let sp = esp0.wrapping_sub(0x10);
        g.write::<u32>(sp, a);
        g.write::<u32>(sp + 12, d);
        sp - 4
    };
    let answer = |g: &mut G, op: u32| g.call(slot(g, op, 411), 0, &[0x14]).eax;
    let mut eax;
    let mut pc;
    if state > 0x2710 {
        return Ret::eax(state);
    }
    if state == 0x2710 {
        pc = 0x45d;
        eax = state;
    } else if state > 0x3e {
        return Ret::eax(state);
    } else {
        // the tables are in the code
        eax = img.word(w(0x55).wrapping_add(state)) & 0xff;
        pc = img.word(w(0x5c).wrapping_add(4 * eax)).wrapping_sub(f);
    }
    loop {
        match pc {
            0x60 => {
                crate::l3_common::scale_node(g, img, img.rel32_target(f + 0x66), w(0x61));
                let b = g.read::<u8>(w(0x6b));
                let v = g.read::<u8>(w(0x71)) | 1;
                g.write::<u8>(w(0x71), v);
                g.write::<u8>(w(0x77), b);
                // call width: 1
                let a = g.call(slot(g, 0x7c, 215), 0, &[0x14]).eax;
                g.write::<u32>(w(0x91), a);
                crate::m0295::node_under(g, img, &at(0x96, w(0x8c), 0, esp0.wrapping_sub(0xc)));
                g.write::<u32>(r + 0x18, 0xa);
                pc = 0xa4;
            }
            0xa4 | 0x394 => {
                let (code, k, b_op, n, half, op) = if pc == 0xa4 {
                    (
                        0xb0u32,
                        0xbu32,
                        0xbbu32,
                        0x1eu32,
                        (0x428c_0000u32, 0x3f80_0000u32),
                        0xf0u32,
                    )
                } else {
                    if answer(g, 0x395) == 0 {
                        return Ret::eax(0);
                    }
                    (0xb4, 1, 0x3c5, 0x1c, (0, 0x3f33_3333), 0x3fa)
                };
                // call width: 2
                g.call(
                    slot(g, if pc == 0xa4 { 0xa5 } else { 0x3af }, 228),
                    0,
                    &[0x14, code],
                );
                let b = u32::from(g.read::<u8>(w(b_op)));
                // call width: 11
                g.call(
                    slot(g, op, 701),
                    0,
                    &[0x14, k, b, 0, n, 0x40, 0x10, half.0, half.1, 0, 0],
                );
                if pc == 0xa4 {
                    let b = u32::from(g.read::<u8>(w(0x103)));
                    // call width: 6
                    g.call(slot(g, 0x110, 537), 0, &[0x14, 2, 0, b, 5, 0xf]);
                    g.write::<u32>(r + 0x18, 0x14);
                    pc = 0x12c;
                } else {
                    let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                    g.write::<u32>(r + 0x18, s);
                    pc = 0x410;
                }
            }
            0x12c => {
                if answer(g, 0x12d) == 0 {
                    return Ret::eax(0);
                }
                // call width: 2
                eax = g.call(slot(g, 0x147, 228), 0, &[0x14, 0xb1]).eax;
                g.write::<u32>(r + 0x18, 0x1e);
                g.write::<u32>(r + 0x20, 0);
                pc = 0x16b;
            }
            0x16b | 0x364 => {
                let k = f32_at(g, w(if pc == 0x16b { 0x16f } else { 0x368 }));
                if k > f32_at(g, r + 0x20) {
                    return Ret::eax(eax);
                }
                if pc == 0x364 {
                    // call width: 2
                    g.call(slot(g, 0x377, 228), 0, &[0x14, 0xb3]);
                    g.write::<u32>(r + 0x18, 0x3c);
                    pc = 0x394;
                    continue;
                }
                let list = w(0x17e);
                crate::sa_misc::node_append(
                    g,
                    img,
                    img.rel32_target(f + 0x188),
                    w(0x183),
                    list,
                    eax,
                );
                let node = w(0x18d);
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x192), node, list, eax);
                let sp = four(g, [w(0x1a2), w(0x19d), w(0x198), node]);
                crate::r11_fades::stores(g, img, f, &[0x1ab, 0x1b5, 0x1bf, 0x1c9]);
                crate::sa_misc::insert_with(g, img, &at(0x1d4, w(0x1a7), list, sp));
                g.write::<u8>(w(0x1e7), 1);
                let sp = ends(g, w(0x1ed), 1);
                crate::s3_nodes::particle_node_with(g, img, &at(0x1f2, w(0x1dc), list, sp));
                let sp = ends(g, w(0x204), 0);
                crate::s4_fam446::effect_node_b(g, img, &at(0x209, w(0x1fa), list, sp));
                let v = g.read::<u32>(w(0x20f)) | 1;
                g.write::<u32>(w(0x20f), v);
                crate::r11_fades::stores(g, img, f, &[0x221]);
                let sp = ends(g, w(0x22c), 1);
                crate::s4_fam446::effect_node_b(g, img, &at(0x231, w(0x218), list, sp));
                let v = g.read::<u32>(w(0x23a)) | 1;
                g.write::<u32>(w(0x23a), v);
                crate::r11_fades::stores(g, img, f, &[0x23f]);
                g.write::<u32>(r + 0x20, 0);
                g.write::<u32>(r + 0x18, 0x28);
                pc = 0x257;
            }
            0x257 => {
                if answer(g, 0x258) == 0 {
                    return Ret::eax(0);
                }
                // call width: 2
                g.call(slot(g, 0x272, 228), 0, &[0x14, 0xb2]);
                crate::sb_misc::node_append(
                    g,
                    img,
                    img.rel32_target(f + 0x290),
                    w(0x28b),
                    w(0x286),
                    0,
                );
                // call width: 2
                eax = g.call(slot(g, 0x295, 325), 0, &[w(0x2a5), w(0x29a)]).eax;
                crate::r11_fades::stores(g, img, f, &[0x2ab]);
                g.write::<u32>(r + 0x18, 0x32);
                pc = 0x2bf;
            }
            0x2bf => {
                if f32_at(g, w(0x2c3)) > f32_at(g, r + 0x20) {
                    return Ret::eax(eax);
                }
                let (node, list) = (w(0x2d7), w(0x2d2));
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x2dc), node, list, eax);
                let sp = four(g, [w(0x2ec), w(0x2e7), w(0x2e2), node]);
                crate::r11_fades::stores(g, img, f, &[0x2f5, 0x2ff, 0x309, 0x313]);
                crate::sa_misc::insert_with(g, img, &at(0x31e, w(0x2f1), list, sp));
                g.write::<u8>(w(0x331), 1);
                let sp = ends(g, w(0x337), 0);
                crate::s3_nodes::particle_node_with(g, img, &at(0x33c, w(0x326), list, sp));
                let h = g.read::<u32>(w(0x351));
                // call width: 3
                eax = g.call(slot(g, 0x341, 239), 0, &[h, 0xffff_fffb, 0]).eax;
                let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, s);
                g.write::<u32>(r + 0x20, 0);
                pc = 0x364;
            }
            0x410 => {
                if answer(g, 0x411) == 0 {
                    return Ret::eax(0);
                }
                // call width: 2
                g.call(slot(g, 0x427, 228), 0, &[0x14, 0xb5]);
                let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, s);
                pc = 0x440;
            }
            0x440 => {
                if answer(g, 0x441) == 0 {
                    return Ret::eax(0);
                }
                g.write::<u32>(r + 0x18, 0x2710);
                pc = 0x45d;
            }
            0x45d => {
                // call width: 2
                g.call(slot(g, 0x45e, 213), 0, &[0x14, 0]);
                let n = g.read::<u32>(t);
                let next = g.read::<u32>(n + 4);
                let prev = g.read::<u32>(n + 8);
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(n + 8);
                let next = g.read::<u32>(n + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(n + 8, n);
                g.write::<u32>(n + 4, n);
                return Ret::eax(next);
            }
            _ => return Ret::eax(eax),
        }
    }
}

/// [`summon_task_0295`] (the `rel32`s: the nodes).
pub const SUMMON_TASK_0295: Sig = Sig {
    len: 1279,
    hash: 0xa686_61b5_0723_4512,
    calls: &[
        (0x66, Callee::Sig(&crate::l3_common::SCALE_NODE)),
        (0x96, Callee::Sig(&crate::m0295::NODE_UNDER)),
        (0x188, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x192, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1d4, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x1f2, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x209, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
        (0x231, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
        (0x290, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x2dc, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x31e, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x33c, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
    ],
    imports: &[],
};

/// `magic_0295`'s sparks (a node method, cdecl `(group)`; an aligned
/// cookie frame, `L(o)` = the frame's `esp + o`): the pass walk
/// ([`walk`](crate::nodes::walk)); with the flag set, the owner `N`'s two
/// children (`+0x48`, `+0x4c`) visited through their method `+0x1c` with
/// a group on the frame (`[child, mode at +5, N]`). While the clock
/// `N + 0x58` is below the constant at operand `+0xb3`: a reservation of
/// 2..5 sparks (slot 324) in the two rings ([`ring_reserve`](crate::sa_misc::ring_reserve)),
/// the place from slots 229 and 234, each new spark's colour, size, glow
/// and speed (slots 307 and 324; slot 325 for its start); past it, with
/// no sparks out (`N + 0x5c`) the node leaves
/// ([`leave_and_register`](crate::m0295::leave_and_register)). Then the
/// sparks of both rings that are alive (byte `+0x2b`) are drawn (slots
/// 229 and 230), faded in, streamed ([`stream_block`](crate::s7_misc::stream_block))
/// and moved, `N + 0x5c` counting them; the clock steps on. Returns the
/// last value in `eax`.
///
/// `magic_0295` 0x10001900 (1400 bytes).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn sparks_0295<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -16; sub esp, 0x108; push esi, edi
    let esp0 = (fr.esp.wrapping_sub(4) & !0xf)
        .wrapping_sub(0x108)
        .wrapping_sub(8);
    let l = |o: u32| esp0.wrapping_add(o);
    let grp = fr.arg(g, 0);
    let owner = g.read::<u32>(grp);
    g.write::<u32>(l(0x10), owner);
    // eax: the cookie word, `^ esp`, under the walk's mode byte
    let cookie = g.read::<u32>(w(0xd)) ^ esp0.wrapping_add(8);
    let (eax, walked) = crate::nodes::walk(g, grp, owner, cookie);
    if walked.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(grp);
    let mode = g.read::<u8>(grp + 5);
    // the children's groups: [child, mode, N] at L(0x18) and L(0x24)
    let c1 = g.read::<u32>(n + 0x48);
    g.write::<u32>(l(0x18), c1);
    g.write::<u8>(l(0x1d), mode);
    g.write::<u32>(l(0x20), n);
    let m = g.read::<u32>(g.read::<u32>(c1) + 0x1c);
    g.call(m, l(0x18), &[l(0x18)]);
    let c2 = g.read::<u32>(n + 0x4c);
    g.write::<u32>(l(0x24), c2);
    g.write::<u8>(l(0x29), mode);
    let m = g.read::<u32>(g.read::<u32>(c2) + 0x1c);
    g.call(m, l(0x24), &[l(0x24)]);
    let k = f32_at(g, w(0xb3));
    let spawn = k > f32_at(g, n + 0x58);
    if spawn {
        g.write::<u32>(l(0x2c), n);
        // call width: 0
        let r = g.call(slot(g, 0xc5, 324), 0, &[]).eax;
        let k = ((r.cast_signed() >> 2) & 3) + 2;
        g.write::<u16>(l(0x30), k.cast_unsigned() as u16);
        crate::sa_misc::ring_reserve(g, l(0x2c));
        let c = g.read::<u32>(n + 0x4c);
        // call width: 3
        g.call(slot(g, 0x102, 229), 0, &[l(0x78), c + 0x9c, n + 0x38]);
        let c = g.read::<u32>(n + 0x4c);
        // call width: 3
        g.call(slot(g, 0x121, 234), 0, &[l(0x78), l(0x78), c + 0xcc]);
        for ring in 0..2u32 {
            g.write::<u32>(l(0x10), ring);
            let count = u32::from(g.read::<u16>(l(0x32) + 2 * ring));
            let mut e = g.read::<u32>(l(0x38) + 4 * ring);
            g.write::<u32>(l(0x14), count);
            if count.cast_signed() <= 0 {
                continue;
            }
            let mut left = count;
            loop {
                g.write::<u32>(e + 0x10, 0);
                let rnd = |g: &mut G, op: u32| {
                    // call width: 0
                    crate::m0018::st0_f32(g.call(slot(g, op, 307), 0, &[]).st0)
                };
                let x = sub(rnd(g, 0x15b), f32_at(g, w(0x175)));
                put(g, l(0xc), x);
                // call width: 0
                let r = g.call(slot(g, 0x17a, 324), 0, &[]).eax;
                g.write::<u16>(e + 0x12, r as u16);
                let even = u32::from(r.trailing_zeros() >= 2);
                g.write::<u32>(e + 0x24, even);
                if even != 0 {
                    let v = mul(
                        add(mul(x, f32_at(g, w(0x1ab))), f32_at(g, w(0x1ba))),
                        f32_at(g, w(0x1c2)),
                    );
                    g.write::<u32>(e + 0x28, 0x8060_6060);
                    put(g, e + 0x20, v);
                } else {
                    g.write::<u32>(e + 0x20, 0x3f00_0000);
                    g.write::<u32>(e + 0x28, 0x8080_8000);
                }
                let s = add(mul(x, f32_at(g, w(0x1e5))), f32_at(g, w(0x1f9)));
                g.write::<u32>(e + 0x2c, 0);
                put(g, l(0xc), s);
                for (i, (op, k0, k1, k2, k3, out)) in [
                    (0x1ea, 0x219, 0x232, 0x23a, 0x246, 0x14u32),
                    (0x22a, 0x265, 0x27e, 0x286, 0x292, 0x18),
                    (0x276, 0x2b1, 0x2c8, 0x2d0, 0x2dc, 0x1c),
                ]
                .into_iter()
                .enumerate()
                {
                    let y = rnd(g, op);
                    let i = u32::try_from(i).unwrap_or(0);
                    let v0 = mul(f32_at(g, l(0x78) + 4 * i), s);
                    let v = mul(
                        mul(
                            add(
                                mul(sub(y, f32_at(g, w(k0))), f32_at(g, w(k1))),
                                f32_at(g, w(k2)),
                            ),
                            v0,
                        ),
                        f32_at(g, w(k3)),
                    );
                    put(g, e + out, v);
                }
                let c = g.read::<u32>(n + 0x48);
                // call width: 2
                g.call(slot(g, 0x2ed, 325), 0, &[e, c + 0x1c]);
                e += 0x30;
                left -= 1;
                g.write::<u32>(l(0x14), left);
                if left == 0 {
                    break;
                }
            }
        }
    } else if g.read::<u32>(n + 0x5c) == 0 {
        g.write::<u32>(l(0x18), n);
        let r = crate::m0295::leave_and_register(g, img, img.rel32_target(f + 0x560), l(0x18));
        return Ret::eax(r);
    }
    // the draw: both rings' live sparks
    g.write::<u32>(l(0x5c), l(0xc8));
    g.write::<u32>(l(0x50), l(0xc8));
    g.write::<u16>(l(0x42), 0x80);
    let saved = g.read::<u32>(w(0x336));
    g.write::<u32>(l(0x54), saved);
    for o in [
        0x8cu32, 0x90, 0x94, 0x98, 0xa0, 0xa4, 0xa8, 0xac, 0xb4, 0x48,
    ] {
        g.write::<u32>(l(o), 0);
    }
    g.write::<u8>(l(0x58), 0);
    g.write::<u32>(n + 0x5c, 0);
    let mut counts = n + 0x2c;
    let mut rings = n + 0x24;
    for left in (1..=2u32).rev() {
        g.write::<u32>(l(0x14), counts);
        g.write::<u32>(l(0x10), rings);
        g.write::<u32>(l(0xc), left);
        let mut e = g.read::<u32>(rings);
        let mut j = 0u32;
        while j < u32::from(g.read::<u16>(counts)) {
            if g.read::<u8>(e + 0x2b) != 0 {
                // call width: 3
                g.call(slot(g, 0x3fc, 229), 0, &[l(0xb8), w(0x3f6), e]);
                let v = mul(f32_at(g, e + 0x20), f32_at(g, e + 0x2c));
                for o in [0xb0u32, 0x9c, 0x88] {
                    put(g, l(o), v);
                }
                // call width: 2
                g.call(slot(g, 0x426, 230), 0, &[l(0xc8), l(0x88)]);
                let a = f32_at(g, e + 0x2c);
                let one = f32_at(g, w(0x456));
                if one > a {
                    let a = add(a, f32_at(g, w(0x466)));
                    put(g, e + 0x2c, a);
                    if a > one {
                        g.write::<u32>(e + 0x2c, 0x3f80_0000);
                    }
                }
                let colour = g.read::<u32>(e + 0x28);
                g.write::<u32>(l(0x44), colour);
                let which = g.read::<u32>(e + 0x24);
                // sub esp, 8; push L(0x40): the stream's argument
                g.write::<u32>(esp0.wrapping_sub(0xc), l(0x40));
                let table = g.read::<u32>(n + 0x50 + 4 * which);
                let sb = Frame {
                    at: img.rel32_target(f + 0x499),
                    esp: esp0.wrapping_sub(0x10),
                    eax: n,
                    ecx: e + 0x10,
                    edx: table,
                    xmm: [0; 4],
                };
                crate::s7_misc::stream_block(g, img, &sb);
                let x = add(f32_at(g, e + 0x14), f32_at(g, e));
                let dy = f32_at(g, e + 0x18);
                let b = g.read::<u8>(e + 0x2b).wrapping_add(0xfe);
                g.write::<u8>(e + 0x2b, b);
                put(g, e, x);
                let y = add(dy, f32_at(g, e + 4));
                let dy = add(dy, f32_at(g, w(0x4c7)));
                put(g, e + 4, y);
                let z = add(f32_at(g, e + 0x1c), f32_at(g, e + 8));
                put(g, e + 0x18, dy);
                put(g, e + 8, z);
                let c = g.read::<u32>(n + 0x5c).wrapping_add(1);
                g.write::<u32>(n + 0x5c, c);
            }
            j += 1;
            e += 0x30;
        }
        rings += 4;
        counts += 2;
    }
    let v = g.read::<u32>(l(0x54));
    g.write::<u32>(w(0x51f), v);
    let c = add(f32_at(g, n + 0x58), f32_at(g, w(0x530)));
    put(g, n + 0x58, c);
    Ret::eax(n)
}

/// [`sparks_0295`] (the `rel32`s: the reservation, the stream, the cookie
/// checks, the leave).
pub const SPARKS_0295: Sig = Sig {
    len: 1400,
    hash: 0xb5bb_3770_0371_02f7,
    calls: &[
        (0xe8, Callee::Sig(&crate::sa_misc::RING_RESERVE)),
        (0x499, Callee::Sig(&crate::s7_misc::STREAM_BLOCK)),
        (0x545, Callee::Any),
        (0x560, Callee::Sig(&crate::m0295::LEAVE_AND_REGISTER)),
        (0x570, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0263`'s summon task (a callback, cdecl `(t)`; `R = *(t + 4)`,
/// its state `+0x18` and count `+0x2c`, which every way out counts up;
/// a byte table at operand `+0x32` into a jump table at `+0x39`):
/// - 0: state 1, the count restarted; 1: at 30, state 10;
/// - 10 (count 45): the static node, slot 268, a node appended and the
///   light inserted, slot 702 and slot 211 on the model (its flags
///   `+0x90` with 0x10, `+0x4c` the light); the next state;
/// - 11: the level falls; at 15, state 0x14;
/// - 0x14: slot 325, the four words, the static link (`xmm2` from operand
///   `+0x1b5`), three words copied and three random spreads (slot 324),
///   the model's flag step; the next state;
/// - 0x15: the level rises to 1.0; at 10 the next state; 0x16: at 25,
///   state 0x1e;
/// - 0x1e: a node, the centre of the objects, slot 325 and the four
///   words; state 0x32;
/// - 0x32 (count 6): a node, the light inserted, a particle node, the
///   model's slots; 0x33 (count 5): slot 239 `(*G, -5, 0)`; 0x34: the
///   scale shrinks, at 30 state 10000;
/// - 10000: the current node dropped, the model set up, the task
///   unlinked.
///
/// Returns the last value in `eax`.
///
/// `magic_0263` 0x10007030 (1277 bytes).
#[allow(clippy::too_many_lines)] // the original's cases, in order
pub fn summon_task_0263<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let t = fr.arg(g, 0);
    let r = g.read::<u32>(t + 4);
    // push ebp; and esp, -8; sub esp, 0x10; push esi, edi
    let esp0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let l = |o: u32| esp0.wrapping_add(o);
    let at = |o: u32, ecx: u32, edx: u32, eax: u32, esp: u32| Frame {
        at: img.rel32_target(f + o),
        esp,
        eax,
        ecx,
        edx,
        xmm: [0; 4],
    };
    let count = |g: &G| g.read::<i32>(r + 0x2c);
    let count_up = |g: &mut G| {
        let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
        g.write::<u32>(r + 0x2c, c);
    };
    // the state one up (or `s`), the count restarted at 1
    let next = |g: &mut G, s: Option<u32>| {
        let s = s.unwrap_or_else(|| g.read::<u32>(r + 0x18).wrapping_add(1));
        g.write::<u32>(r + 0x18, s);
        g.write::<u32>(r + 0x2c, 1);
    };
    let state = g.read::<u32>(r + 0x18);
    if state == 0x2710 {
        crate::o4_single3::drop_current(g, img, &at(0x45e, state, 0, fr.eax, l(0) - 4));
        let m = g.read::<u32>(w(0x463));
        g.write::<u32>(l(0x8), m);
        crate::sa_summon::model_setup(g, img, &at(0x470, l(0x8), 0, 0, l(0) - 4));
        let n = g.read::<u32>(t);
        let nx = g.read::<u32>(n + 4);
        let pv = g.read::<u32>(n + 8);
        g.write::<u32>(nx + 8, pv);
        let pv = g.read::<u32>(n + 8);
        let nx = g.read::<u32>(n + 4);
        g.write::<u32>(pv + 4, nx);
        g.write::<u32>(n + 8, n);
        g.write::<u32>(n + 4, n);
        count_up(g);
        return Ret::eax(nx);
    }
    if state > 0x34 {
        count_up(g);
        return Ret::eax(fr.eax);
    }
    // the tables are in the code
    let k = img.word(w(0x32).wrapping_add(state)) & 0xff;
    let pc = img.word(w(0x39).wrapping_add(4 * k)).wrapping_sub(f);
    // below the count's mark: counted, the table's byte in eax
    let wait = |g: &mut G, mark: i32| {
        if count(g) < mark {
            count_up(g);
            true
        } else {
            false
        }
    };
    match pc {
        0x3d => {
            g.write::<u32>(r + 0x2c, 1);
            g.write::<u32>(r + 0x18, state + 1);
            Ret::eax(state + 1)
        }
        0x53 => {
            if !wait(g, 0x1e) {
                g.write::<u32>(r + 0x18, 0xa);
                count_up(g);
            }
            Ret::eax(k)
        }
        0x6d => {
            if wait(g, 0x2d) {
                return Ret::eax(k);
            }
            g.write::<u32>(l(0) - 4, state);
            crate::m0263::static_node(g, img, &at(0x79, state, 0, k, l(0) - 8));
            let h = g.read::<u32>(w(0x94));
            // call width: 3
            g.call(slot(g, 0x7e, 268), 0, &[h, 0xf, w(0x8c)]);
            let (node, list) = (w(0xa3), w(0x9e));
            stores(g, img, f, &[0xa7]);
            crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xb2), node, list, 0);
            let sp = l(0) - 0x10;
            for (i, v) in (0u32..).zip([w(0xc2), w(0xbd), w(0xb8), node]) {
                g.write::<u32>(sp + 4 * i, v);
            }
            stores(g, img, f, &[0xcb, 0xd5, 0xdf, 0xe9]);
            crate::sa_misc::insert_with(g, img, &at(0xf4, w(0xc7), list, 0, sp - 4));
            let m = g.read::<u32>(w(0xf9));
            g.write::<u8>(w(0x101), 1);
            let o = g.read::<u32>(m + 0x50);
            // call width: 3
            g.call(slot(g, 0x10c, 702), 0, &[o, 0xffff_ffff, 0xf]);
            let m = g.read::<u32>(w(0x11a));
            let model = g.read::<u32>(m + 0x54);
            // call width: 2
            let e = g.call(slot(g, 0x11f, 211), 0, &[model, w(0x12d)]).eax;
            let fl = g.read::<u16>(m + 0x90) | 0x10;
            g.write::<u16>(m + 0x90, fl);
            g.write::<u32>(m + 0x4c, w(0x141));
            next(g, None);
            Ret::eax(e)
        }
        0x15b => {
            let v = sub(f32_at(g, w(0x15f)), f32_at(g, w(0x167)));
            put(g, w(0x16f), v);
            if !wait(g, 0xf) {
                g.write::<u32>(r + 0x18, 0x14);
                count_up(g);
            }
            Ret::eax(k)
        }
        0x18d => {
            // call width: 2
            let e = g.call(slot(g, 0x18e, 325), 0, &[w(0x19e), w(0x193)]).eax;
            crate::o4_tiny::set_four(g, img, &at(0x1ad, state, w(0x1a8), e, l(0) - 4));
            let k2 = g.read::<u32>(w(0x1b5));
            stores(g, img, f, &[0x1b9, 0x1c3, 0x1cd]);
            let mut sl = at(0x1d8, state, w(0x1a8), e, l(0) - 4);
            sl.xmm[2] = u128::from(k2);
            crate::s4_small::static_link_xmm(g, img, &sl);
            for (src, dst) in [(0x1dd, 0x1e2), (0x1e7, 0x1ec), (0x1f1, 0x1f6)] {
                let v = g.read::<u32>(w(src));
                g.write::<u32>(w(dst), v);
            }
            for (op, out, sh, form) in [
                (0x1fb, 0x21b, 0x12, 0u8),
                (0x215, 0x233, 0x11, 1),
                (0x238, 0x253, 0x10, 0),
            ] {
                // call width: 0
                let v = (g.call(slot(g, op, 324), 0, &[]).eax & 2) << sh;
                let x = if form == 0 {
                    (1u32 << sh).wrapping_sub(v)
                } else {
                    v.wrapping_sub(0x2_0000)
                };
                g.write::<u32>(w(out), x);
            }
            stores(g, img, f, &[0x257]);
            let e = crate::o4_single3::model_flag_step(g, img, &at(0x262, 0, 0, 0, l(0) - 4)).eax;
            next(g, None);
            Ret::eax(e)
        }
        0x279 => {
            let x = add(f32_at(g, w(0x27d)), f32_at(g, w(0x285)));
            let top = f32_at(g, w(0x28c));
            put(g, w(0x294), x);
            if !(x < top || x.is_nan() || top.is_nan()) {
                g.write::<u32>(w(0x29c), 0x3f80_0000);
            }
            if !wait(g, 0xa) {
                next(g, None);
            }
            Ret::eax(k)
        }
        0x2c1 => {
            if !wait(g, 0x19) {
                next(g, Some(0x1e));
            }
            Ret::eax(k)
        }
        0x2e2 => {
            let list = w(0x2e3);
            crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x2ed), w(0x2e8), list, k);
            for (o, op) in [(0xcu32, 0x2f9u32), (0x10, 0x301), (0x14, 0x309)] {
                g.write::<u32>(l(o), w(op));
            }
            crate::r32::centre_0263(g, img, &at(0x30e, l(0xc), list, 0, l(0) - 4));
            // call width: 2
            let e = g.call(slot(g, 0x313, 325), 0, &[w(0x323), w(0x318)]).eax;
            crate::o4_tiny::set_four(g, img, &at(0x332, l(0xc), w(0x32d), e, l(0) - 4));
            stores(g, img, f, &[0x336, 0x340, 0x34a]);
            next(g, Some(0x32));
            Ret::eax(e)
        }
        0x36b => {
            if wait(g, 6) {
                return Ret::eax(k);
            }
            let (node, list) = (w(0x37b), w(0x376));
            crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x380), node, list, k);
            let sp = l(0) - 0x10;
            for (i, v) in (0u32..).zip([w(0x390), w(0x38b), w(0x386), node]) {
                g.write::<u32>(sp + 4 * i, v);
            }
            stores(g, img, f, &[0x399, 0x3a3, 0x3ad, 0x3b7]);
            crate::sa_misc::insert_with(g, img, &at(0x3c2, w(0x395), list, 0, sp - 4));
            g.write::<u32>(sp, w(0x3d4));
            g.write::<u32>(sp + 12, 2);
            let e =
                crate::s3_nodes::particle_node_with(g, img, &at(0x3d9, w(0x3ca), list, 0, sp - 4));
            let e = crate::o4_single::model_slots(g, img, &at(0x3e1, 0, 0, e, l(0) - 4)).eax;
            next(g, None);
            Ret::eax(e)
        }
        0x3f8 => {
            if wait(g, 5) {
                return Ret::eax(k);
            }
            let h = g.read::<u32>(w(0x413));
            // call width: 3
            let e = g.call(slot(g, 0x403, 239), 0, &[h, 0xffff_fffb, 0]).eax;
            next(g, None);
            Ret::eax(e)
        }
        0x42f => {
            let v = mul(f32_at(g, w(0x433)), f32_at(g, w(0x43b)));
            put(g, w(0x443), v);
            if !wait(g, 0x1e) {
                g.write::<u32>(r + 0x18, 0x2710);
                count_up(g);
            }
            Ret::eax(k)
        }
        _ => {
            count_up(g);
            Ret::eax(k)
        }
    }
}

/// [`summon_task_0263`] (the `rel32`s: the nodes, the model calls).
pub const SUMMON_TASK_0263: Sig = Sig {
    len: 1277,
    hash: 0xa12f_237b_bfcf_36a1,
    calls: &[
        (0x79, Callee::Sig(&crate::m0263::STATIC_NODE)),
        (0xb2, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xf4, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x1ad, Callee::Sig(&crate::o4_tiny::SET_FOUR_0263_6F40)),
        (0x1d8, Callee::Sig(&crate::s4_small::STATIC_LINK_XMM)),
        (
            0x262,
            Callee::Sig(&crate::o4_single3::MODEL_FLAG_STEP_0263_6BE0),
        ),
        (0x2ed, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x30e, Callee::Sig(&crate::r32::CENTRE_0263)),
        (0x332, Callee::Sig(&crate::o4_tiny::SET_FOUR_0263_6F40)),
        (0x380, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x3c2, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x3d9, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x3e1, Callee::Sig(&crate::o4_single::MODEL_SLOTS_0263_6B90)),
        (
            0x45e,
            Callee::Sig(&crate::o4_single3::DROP_CURRENT_0263_6B30),
        ),
        (0x470, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// `magic_0263`'s summon start (a callback, cdecl `(t)`; an aligned
/// cookie frame, `L(o)` = the frame's `esp + o`; `R = *(t + 4)`, its
/// state `+0x18` and count `+0x2c`, which every way out counts up):
/// - 0: the summon's model node, the big node, a node appended and set
///   up, slot 215's actor into a node, the static child, the linked node
///   with the flag byte, a node with the scale `K / slot 277`, the model
///   start; then on as 10;
/// - 10: slot 228 `(*G_A, 0xba)`, the centre of the objects, the height
///   from a distance and the model placed (`xmm2` from operand `+0x246`);
///   on as 11 with the count restarted;
/// - 11 (count 5): a node, the group inserted, a particle node, two
///   effect nodes flagged; 12 (count 25): state 0x3c;
/// - 0x3c (count 20): a node, the group inserted, a particle node; 0x3d
///   (count 45): a node, the light inserted, a particle node, the node
///   leaving; 0x3e (count 12): state 0x3f; 0x3f: slot 411 `(*G_A)`
///   answering, slot 213 `(*G_A, 0)`, state 10000;
/// - 10000: the task unlinked.
///
/// Returns the last value in `eax`.
///
/// `magic_0263` 0x10007630 (1557 bytes).
#[allow(clippy::too_many_lines)] // the original's cases, in order
pub fn summon_start_0263<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let t = fr.arg(g, 0);
    let r = g.read::<u32>(t + 4);
    // push ebp; and esp, -8; sub esp, 0x2c; push esi
    let esp0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let l = |o: u32| esp0.wrapping_add(o);
    let at = |o: u32, ecx: u32, edx: u32, eax: u32, esp: u32| Frame {
        at: img.rel32_target(f + o),
        esp,
        eax,
        ecx,
        edx,
        xmm: [0; 4],
    };
    let call_esp = l(0).wrapping_sub(4);
    let count = |g: &G| g.read::<i32>(r + 0x2c);
    let count_up = |g: &mut G| {
        let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
        g.write::<u32>(r + 0x2c, c);
    };
    let next = |g: &mut G, s: Option<u32>| {
        let s = s.unwrap_or_else(|| g.read::<u32>(r + 0x18).wrapping_add(1));
        g.write::<u32>(r + 0x18, s);
        g.write::<u32>(r + 0x2c, 1);
    };
    let group = |g: &mut G, words: [u32; 6]| {
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(l(0x14) + 4 * i, v);
        }
    };
    let state = g.read::<u32>(r + 0x18);
    let mut pc = if state == 0x3c {
        0x3ad
    } else if state > 0x3c {
        match state {
            0x3d => 0x4c5,
            0x3e => 0x4a4,
            0x3f => 0x572,
            0x2710 => 0x5c5,
            _ => {
                count_up(g);
                return Ret::eax(state);
            }
        }
    } else if state > 0xc {
        count_up(g);
        return Ret::eax(state);
    } else {
        // the tables are in the code
        let k = img.word(w(0x39).wrapping_add(state)) & 0xff;
        img.word(w(0x40).wrapping_add(4 * k)).wrapping_sub(f)
    };
    let k = if state <= 0xc {
        img.word(w(0x39).wrapping_add(state)) & 0xff
    } else {
        state
    };
    let mut eax = k;
    loop {
        match pc {
            0x44 => {
                let a = g.read::<u32>(w(0x45));
                g.write::<u32>(l(0x14), w(0x51));
                g.write::<u32>(l(0x18), w(0x59));
                g.write::<u32>(l(0x1c), a);
                g.write::<u8>(l(0x20), 0);
                stores(g, img, f, &[0x66]);
                crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0x71), l(0x14));
                for (o, op) in [
                    (0x14u32, 0x7du32),
                    (0x18, 0x85),
                    (0x1c, 0x8d),
                    (0x20, 0x95),
                    (0x24, 0x9d),
                ] {
                    g.write::<u32>(l(o), w(op));
                }
                crate::m0070::node_init_big_table(g, img, img.rel32_target(f + 0xa2), l(0x14));
                let a = g.read::<u32>(w(0xa7));
                let (list, node) = (w(0xac), w(0xb1));
                g.write::<u32>(w(0xb6), a);
                stores(g, img, f, &[0xba]);
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xc5), node, list, a);
                // push ecx; push ecx: the argument and a word under it
                g.write::<u32>(l(0) - 4, node);
                g.write::<u32>(l(0) - 8, node);
                stores(g, img, f, &[0xd0, 0xda, 0xe4, 0xee]);
                crate::m0263::node_setup(g, img, &at(0xf9, w(0xcc), list, a, l(0) - 0xc));
                let h = g.read::<u32>(w(0x104));
                g.write::<u32>(l(0x14), w(0x112));
                g.write::<u32>(l(0x18), list);
                // call width: 1
                let actor = g.call(slot(g, 0xfe, 215), 0, &[h]).eax;
                g.write::<u32>(l(0x1c), actor);
                g.write::<u32>(l(0x20), 0xc);
                crate::m0042::node_init(g, img, img.rel32_target(f + 0x130), l(0x14));
                crate::sa_misc::static_child(g, img, img.rel32_target(f + 0x13a), w(0x135));
                let b = g.read::<u8>(w(0x13f));
                g.write::<u32>(l(0) - 4, w(0x135));
                g.write::<u8>(w(0x14a), b);
                crate::s4_small::link_node_38(g, img, &at(0x14f, w(0x145), list, 0, l(0) - 8));
                let (list2, node2) = (w(0x157), w(0x15c));
                stores(g, img, f, &[0x160, 0x16a, 0x174]);
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x17f), node2, list2, 0);
                let x = g.read::<u32>(w(0x185));
                stores(g, img, f, &[0x189, 0x193]);
                let s = crate::o4_tiny::slot_float(g, img, &at(0x19e, x, list2, 0, call_esp));
                let s = match s.xmm0 {
                    Some(crate::Xmm::F32(b)) => f32::from_bits(b),
                    _ => f32::NAN,
                };
                let v = div(f32_at(g, w(0x1a6)), s);
                g.write::<u32>(l(0) - 4, x);
                g.write::<u32>(l(0) - 8, w(0x1b0));
                put(g, w(0x1bd), v);
                stores(g, img, f, &[0x1c1]);
                crate::m0263::node_setup(g, img, &at(0x1cc, w(0x1b5), list2, 0, l(0) - 0xc));
                crate::m0263::model_start(g, img, &at(0x1d4, 0, 0, 0, call_esp));
                g.write::<u32>(r + 0x18, 0xa);
                pc = 0x1df;
            }
            0x1df => {
                let h = g.read::<u32>(w(0x1eb));
                // call width: 2
                g.call(slot(g, 0x1e0, 228), 0, &[h, 0xba]);
                g.write::<u32>(l(0x4), w(0x206));
                g.write::<u32>(l(0x8), l(0x14));
                g.write::<u32>(l(0xc), w(0x212));
                crate::r32::centre_0263(g, img, &at(0x217, l(0x4), 0, 0, call_esp));
                g.write::<u32>(l(0x4), w(0x227));
                g.write::<u32>(l(0x10), 0xffff_f8e4);
                g.write::<u32>(l(0xc), l(0x14));
                crate::r32::height_0263(g, img, &at(0x238, l(0x4), 0, 0, call_esp));
                let y = g.read::<u32>(l(0x8));
                let obj = g.read::<u32>(l(0x4));
                g.write::<u32>(l(0x20), 0);
                g.write::<u32>(l(0x1c), 0);
                g.write::<u32>(l(0x14), 0);
                g.write::<u32>(l(0x18), y);
                let mut mp = at(0x271, obj, l(0x14), 0, call_esp);
                mp.xmm[2] = u128::from(g.read::<u32>(w(0x246)));
                eax = crate::s4_small::model_place(g, img, &mp).eax;
                let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, s);
                g.write::<u32>(r + 0x2c, 0);
                pc = 0x27f;
            }
            0x27f => {
                if count(g) < 5 {
                    count_up(g);
                    return Ret::eax(eax);
                }
                let (list, node) = (w(0x28a), w(0x28f));
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x294), node, list, eax);
                stores(g, img, f, &[0x2a0, 0x2aa, 0x2b4, 0x2be]);
                group(g, [w(0x2cc), list, w(0x2d8), w(0x2e0), w(0x2e8), node]);
                crate::nodes::node_insert(g, img, &at(0x2ed, l(0x14), list, 0, call_esp));
                let sp = l(0) - 0x10;
                g.write::<u32>(sp, w(0x2fc));
                g.write::<u32>(sp + 12, 3);
                g.write::<u8>(w(0x302), 1);
                crate::s3_nodes::particle_node_with(g, img, &at(0x308, w(0x2f7), list, 0, sp - 4));
                for (i, v) in (0u32..).zip([w(0x318), w(0x313), w(0x2f7), 0]) {
                    g.write::<u32>(sp + 4 * i, v);
                }
                crate::s3_nodes::effect_node_with(g, img, &at(0x322, w(0x31d), list, 0, sp - 4));
                let v = g.read::<u32>(w(0x328)) | 1;
                g.write::<u32>(w(0x328), v);
                stores(g, img, f, &[0x330]);
                for (i, v) in (0u32..).zip([w(0x343), w(0x33e), w(0x31d), 1]) {
                    g.write::<u32>(sp + 4 * i, v);
                }
                let e = crate::s3_nodes::effect_node_with(
                    g,
                    img,
                    &at(0x34d, w(0x348), list, 0, sp - 4),
                );
                let v = g.read::<u32>(w(0x353)) | 1;
                g.write::<u32>(w(0x353), v);
                stores(g, img, f, &[0x358]);
                next(g, None);
                return Ret::eax(e);
            }
            0x382 => {
                if count(g) < 0x19 {
                    count_up(g);
                } else {
                    next(g, Some(0x3c));
                }
                return Ret::eax(eax);
            }
            0x3ad => {
                if count(g) < 0x14 {
                    count_up(g);
                    return Ret::eax(eax);
                }
                let (list, node) = (w(0x3b8), w(0x3bd));
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x3c2), node, list, eax);
                stores(g, img, f, &[0x3ce, 0x3d8, 0x3e2, 0x3ec]);
                group(g, [w(0x3fa), list, w(0x406), w(0x40e), w(0x416), node]);
                crate::nodes::node_insert(g, img, &at(0x41b, l(0x14), list, 0, call_esp));
                let e = g.read::<u32>(w(0x420));
                g.write::<u8>(w(0x42a), 1);
                group(g, [w(0x433), w(0x43b), w(0x443), e, w(0x44f), 0]);
                stores(g, img, f, &[0x45b]);
                let e =
                    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x466), l(0x14));
                next(g, None);
                return Ret::eax(e);
            }
            0x4a4 => {
                if count(g) >= 0xc {
                    g.write::<u32>(r + 0x18, 0x3f);
                    count_up(g);
                    return Ret::eax(0x3f);
                }
                count_up(g);
                return Ret::eax(state);
            }
            0x4c5 => {
                if count(g) < 0x2d {
                    count_up(g);
                    return Ret::eax(state);
                }
                let (list, node) = (w(0x4d0), w(0x4d5));
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x4da), node, list, state);
                let sp = l(0) - 0x10;
                for (i, v) in (0u32..).zip([w(0x4ea), w(0x4e5), w(0x4e0), node]) {
                    g.write::<u32>(sp + 4 * i, v);
                }
                stores(g, img, f, &[0x4f3, 0x4fd, 0x507, 0x511]);
                crate::sa_misc::insert_with(g, img, &at(0x51c, w(0x4ef), list, 0, sp - 4));
                g.write::<u32>(sp, w(0x535));
                g.write::<u32>(sp + 12, 1);
                g.write::<u8>(w(0x52f), 1);
                let e = crate::s3_nodes::particle_node_with(
                    g,
                    img,
                    &at(0x53a, w(0x524), list, 0, sp - 4),
                );
                stores(g, img, f, &[0x546]);
                let e = crate::sa_misc::leave(g, w(0x542), e);
                next(g, None);
                return Ret::eax(e);
            }
            0x572 => {
                let h = g.read::<u32>(w(0x579));
                // call width: 1
                if g.call(slot(g, 0x573, 411), 0, &[h]).eax == 0 {
                    count_up(g);
                    return Ret::eax(0);
                }
                let h = g.read::<u32>(w(0x595));
                // call width: 2
                let e = g.call(slot(g, 0x58d, 213), 0, &[h, 0]).eax;
                count_up(g);
                g.write::<u32>(r + 0x18, 0x2710);
                return Ret::eax(e);
            }
            0x5c5 => {
                let n = g.read::<u32>(t);
                let nx = g.read::<u32>(n + 4);
                let pv = g.read::<u32>(n + 8);
                g.write::<u32>(nx + 8, pv);
                let pv = g.read::<u32>(n + 8);
                let nx = g.read::<u32>(n + 4);
                g.write::<u32>(pv + 4, nx);
                g.write::<u32>(n + 8, n);
                g.write::<u32>(n + 4, n);
                count_up(g);
                return Ret::eax(nx);
            }
            _ => {
                count_up(g);
                return Ret::eax(eax);
            }
        }
    }
}

/// [`summon_start_0263`] (the `rel32`s: the nodes, the model calls, the
/// cookie checks).
pub const SUMMON_START_0263: Sig = Sig {
    len: 1557,
    hash: 0x5cdd_70a1_f76d_b5b8,
    calls: &[
        (0x71, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xa2, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0xc5, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xf9, Callee::Sig(&crate::m0263::NODE_SETUP)),
        (0x130, Callee::Sig(&crate::m0042::NODE_INIT)),
        (0x13a, Callee::Sig(&crate::sa_misc::STATIC_CHILD)),
        (0x14f, Callee::Sig(&crate::s4_small::LINK_NODE_38)),
        (0x17f, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x19e, Callee::Sig(&crate::o4_tiny::SLOT_FLOAT_0263_8980)),
        (0x1cc, Callee::Sig(&crate::m0263::NODE_SETUP)),
        (0x1d4, Callee::Sig(&crate::m0263::MODEL_START)),
        (0x217, Callee::Sig(&crate::r32::CENTRE_0263)),
        (0x238, Callee::Sig(&crate::r32::HEIGHT_0263)),
        (0x271, Callee::Sig(&crate::s4_small::MODEL_PLACE)),
        (0x294, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x2ed, Callee::Sig(&crate::nodes::INSERT)),
        (0x308, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x322, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x34d, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x37a, Callee::Any),
        (0x3a5, Callee::Any),
        (0x3c2, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x41b, Callee::Sig(&crate::nodes::INSERT)),
        (0x466, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x47f, Callee::Any),
        (0x4bd, Callee::Any),
        (0x4da, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x51c, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x53a, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x551, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x56a, Callee::Any),
        (0x5b6, Callee::Any),
        (0x5ea, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0153`'s turning sparks (a node method, cdecl `(group)`; an
/// aligned cookie frame, `L(o)` = the frame's `esp + o`): as
/// [`sparks_0295`] with sparks of 0x34 bytes ([`ring_take`](crate::m0070::ring_take)
/// for the reservation; the odd ones keep slot 324's halfword as their
/// angle `+0x30`, the size scaled twice), each drawn spark's matrix turned
/// by its angle (the sine table `*G_T`, operand `+0x432`: rows 0 and 1
/// mixed by `s`/`c` and put back through slot 325), the fade-in capped
/// at 0.7, the life byte down by 4 and the stream's argument the sprite
/// record itself; with no sparks out past the clock the node releases its
/// effects ([`release_effects`](crate::m0153::release_effects)). Returns
/// the saved draw word (the global at operand `+0x356`) put back.
///
/// `magic_0153` 0x10001980 (1782 bytes).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn sparks_0153<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -16; sub esp, 0x118; push esi, edi
    let esp0 = (fr.esp.wrapping_sub(4) & !0xf)
        .wrapping_sub(0x118)
        .wrapping_sub(8);
    let l = |o: u32| esp0.wrapping_add(o);
    let grp = fr.arg(g, 0);
    let owner = g.read::<u32>(grp);
    g.write::<u32>(l(0x14), owner);
    // eax: the cookie word, `^ esp`, under the walk's mode byte
    let cookie = g.read::<u32>(w(0xd)) ^ esp0.wrapping_add(8);
    let (eax, walked) = crate::nodes::walk(g, grp, owner, cookie);
    if walked.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(grp);
    let mode = g.read::<u8>(grp + 5);
    // the children's groups: [child, mode] at L(0x14) and L(0x24)
    let c1 = g.read::<u32>(n + 0x48);
    g.write::<u32>(l(0x14), c1);
    g.write::<u8>(l(0x19), mode);
    let m = g.read::<u32>(g.read::<u32>(c1) + 0x1c);
    g.call(m, l(0x14), &[l(0x14)]);
    let c2 = g.read::<u32>(n + 0x4c);
    g.write::<u32>(l(0x24), c2);
    g.write::<u8>(l(0x29), mode);
    let m = g.read::<u32>(g.read::<u32>(c2) + 0x1c);
    g.call(m, l(0x24), &[l(0x24)]);
    let k = f32_at(g, w(0xaf));
    if k > f32_at(g, n + 0x58) {
        g.write::<u32>(l(0x2c), n);
        // call width: 0
        let r = g.call(slot(g, 0xc1, 324), 0, &[]).eax;
        let k = ((r.cast_signed() >> 2) & 3) + 2;
        g.write::<u16>(l(0x30), k.cast_unsigned() as u16);
        crate::m0070::ring_take(g, l(0x2c));
        let c = g.read::<u32>(n + 0x4c);
        // call width: 3
        g.call(slot(g, 0xfe, 229), 0, &[l(0x78), c + 0x9c, n + 0x38]);
        let c = g.read::<u32>(n + 0x4c);
        // call width: 3
        g.call(slot(g, 0x11d, 234), 0, &[l(0x78), l(0x78), c + 0xcc]);
        for ring in 0..2u32 {
            g.write::<u32>(l(0x20), ring);
            let mut e = g.read::<u32>(l(0x38) + 4 * ring);
            let count = u32::from(g.read::<u16>(l(0x32) + 2 * ring));
            g.write::<u32>(l(0x1c), count);
            if count == 0 {
                continue;
            }
            let mut left = count;
            loop {
                g.write::<u32>(e + 0x10, 0);
                let rnd = |g: &mut G, op: u32| {
                    // call width: 0
                    crate::m0018::st0_f32(g.call(slot(g, op, 307), 0, &[]).st0)
                };
                let x = sub(rnd(g, 0x156), f32_at(g, w(0x170)));
                // call width: 0
                let r = g.call(slot(g, 0x175, 324), 0, &[]).eax;
                let even = u32::from(r.trailing_zeros() >= 2);
                g.write::<u16>(e + 0x12, r as u16);
                g.write::<u32>(e + 0x24, even);
                if even != 0 {
                    let v = mul(
                        add(mul(x, f32_at(g, w(0x1a6))), f32_at(g, w(0x1bc))),
                        f32_at(g, w(0x1c4)),
                    );
                    g.write::<u32>(e + 0x28, 0x8060_6060);
                    g.write::<u32>(e + 0x30, 0);
                    put(g, e + 0x20, v);
                } else {
                    g.write::<u32>(e + 0x20, 0x3e4c_cccd);
                    g.write::<u32>(e + 0x28, 0x8080_8000);
                    g.write::<u32>(e + 0x30, r & 0xffff);
                }
                let s = add(mul(x, f32_at(g, w(0x1ed))), f32_at(g, w(0x201)));
                g.write::<u32>(e + 0x2c, 0);
                for (i, (op, k0, k1, k2, k3, k4, out)) in [
                    (0x1f2, 0x221, 0x23a, 0x242, 0x24e, 0x256, 0x14u32),
                    (0x232, 0x275, 0x28e, 0x296, 0x2a2, 0x2aa, 0x18),
                    (0x286, 0x2c9, 0x2e0, 0x2e8, 0x2f4, 0x2fc, 0x1c),
                ]
                .into_iter()
                .enumerate()
                {
                    let y = rnd(g, op);
                    let i = u32::try_from(i).unwrap_or(0);
                    let v0 = mul(f32_at(g, l(0x78) + 4 * i), s);
                    let v = add(
                        mul(sub(y, f32_at(g, w(k0))), f32_at(g, w(k1))),
                        f32_at(g, w(k2)),
                    );
                    let v = mul(mul(mul(v, v0), f32_at(g, w(k3))), f32_at(g, w(k4)));
                    put(g, e + out, v);
                }
                let c = g.read::<u32>(n + 0x48);
                // call width: 2
                g.call(slot(g, 0x30d, 325), 0, &[e, c + 0x1c]);
                e += 0x34;
                left -= 1;
                g.write::<u32>(l(0x1c), left);
                if left == 0 {
                    break;
                }
            }
        }
    } else if g.read::<u32>(n + 0x5c) == 0 {
        g.write::<u32>(l(0x14), n);
        let r = crate::m0153::release_effects(g, img, img.rel32_target(f + 0x6de), l(0x14));
        return Ret::eax(r);
    }
    // the draw: both rings' live sparks
    g.write::<u32>(l(0x5c), l(0xd8));
    g.write::<u32>(l(0x50), l(0xd8));
    g.write::<u16>(l(0x42), 0x80);
    let saved = g.read::<u32>(w(0x356));
    g.write::<u32>(l(0x54), saved);
    for o in [
        0x8cu32, 0x90, 0x94, 0x98, 0xa0, 0xa4, 0xa8, 0xac, 0xb4, 0x48,
    ] {
        g.write::<u32>(l(o), 0);
    }
    g.write::<u8>(l(0x58), 0);
    g.write::<u32>(n + 0x5c, 0);
    let mut counts = n + 0x2c;
    let mut rings = n + 0x24;
    for left in (1..=2u32).rev() {
        g.write::<u32>(l(0x10), counts);
        g.write::<u32>(l(0x1c), rings);
        g.write::<u32>(l(0x14), left);
        let mut e = g.read::<u32>(rings);
        let mut j = 0u32;
        while j < u32::from(g.read::<u16>(counts)) {
            g.write::<u32>(l(0x20), j);
            if g.read::<u8>(e + 0x2b) != 0 {
                // call width: 3
                g.call(slot(g, 0x41a, 229), 0, &[l(0xb8), w(0x414), e]);
                let v = mul(f32_at(g, e + 0x20), f32_at(g, e + 0x2c));
                let table = g.read::<u32>(w(0x432));
                for o in [0xb0u32, 0x9c, 0x88] {
                    put(g, l(o), v);
                }
                let a = g.read::<u32>(e + 0x30);
                let s = f32_at(g, table + 4 * ((a >> 4) & 0xfff));
                let c = f32_at(g, table + 4 * ((a.wrapping_add(0x4000) >> 4) & 0xfff));
                let at = |g: &G, o: u32| f32_at(g, l(o));
                // rows 0 and 1 turned
                let r0 = [
                    add(mul(c, v), mul(s, at(g, 0x98))),
                    add(mul(s, v), mul(c, at(g, 0x8c))),
                    add(mul(s, at(g, 0xa0)), mul(c, at(g, 0x90))),
                    add(mul(s, at(g, 0xa4)), mul(c, at(g, 0x94))),
                ];
                let r1 = [
                    sub(mul(c, at(g, 0x98)), mul(s, v)),
                    sub(mul(c, v), mul(s, at(g, 0x8c))),
                    sub(mul(c, at(g, 0xa0)), mul(s, at(g, 0x90))),
                    sub(mul(c, at(g, 0xa4)), mul(s, at(g, 0x94))),
                ];
                for (i, (x, y)) in (0u32..).zip(r0.into_iter().zip(r1)) {
                    put(g, l(0xc8) + 4 * i, x);
                    put(g, l(0x2c) + 4 * i, y);
                }
                // call width: 2
                g.call(slot(g, 0x4d4, 325), 0, &[l(0x88), l(0xc8)]);
                // call width: 2
                g.call(slot(g, 0x5a6, 325), 0, &[l(0x98), l(0x2c)]);
                // call width: 2
                g.call(slot(g, 0x5c3, 230), 0, &[l(0xd8), l(0x88)]);
                let a = f32_at(g, e + 0x2c);
                if f32_at(g, w(0x5d8)) > a {
                    let a = add(a, f32_at(g, w(0x5e8)));
                    put(g, e + 0x2c, a);
                    if a > f32_at(g, w(0x5ef)) {
                        g.write::<u32>(e + 0x2c, 0x3f33_3333);
                    }
                }
                let colour = g.read::<u32>(e + 0x28);
                g.write::<u32>(l(0x44), colour);
                let which = g.read::<u32>(e + 0x24);
                let table = g.read::<u32>(n + 0x50 + 4 * which);
                // sub esp, 8; push L(0x40): the stream's argument
                g.write::<u32>(esp0.wrapping_sub(0xc), l(0x40));
                let sb = Frame {
                    at: img.rel32_target(f + 0x61b),
                    esp: esp0.wrapping_sub(0x10),
                    eax: l(0x40),
                    ecx: e + 0x10,
                    edx: table,
                    xmm: [0; 4],
                };
                crate::s7_misc::stream_block(g, img, &sb);
                let x = add(f32_at(g, e + 0x14), f32_at(g, e));
                let dy = f32_at(g, e + 0x18);
                let b = g.read::<u8>(e + 0x2b).wrapping_add(0xfc);
                g.write::<u8>(e + 0x2b, b);
                put(g, e, x);
                let y = add(f32_at(g, e + 4), dy);
                let dy = add(dy, f32_at(g, w(0x64a)));
                put(g, e + 4, y);
                let z = add(f32_at(g, e + 0x1c), f32_at(g, e + 8));
                put(g, e + 0x18, dy);
                put(g, e + 8, z);
                let c = g.read::<u32>(n + 0x5c).wrapping_add(1);
                g.write::<u32>(n + 0x5c, c);
            }
            j += 1;
            e += 0x34;
        }
        rings += 4;
        counts += 2;
    }
    let c = add(f32_at(g, n + 0x58), f32_at(g, w(0x6a5)));
    let v = g.read::<u32>(l(0x54));
    g.write::<u32>(w(0x6ae), v);
    put(g, n + 0x58, c);
    Ret::eax(v)
}

/// [`sparks_0153`] (the `rel32`s: the reservation, the stream, the cookie
/// checks, the release).
pub const SPARKS_0153: Sig = Sig {
    len: 1782,
    hash: 0x42de_41e9_8484_839e,
    calls: &[
        (0xe4, Callee::Sig(&crate::m0070::RING_TAKE)),
        (0x61b, Callee::Sig(&crate::s7_misc::STREAM_BLOCK)),
        (0x6c3, Callee::Any),
        (0x6de, Callee::Sig(&crate::m0153::RELEASE_EFFECTS)),
        (0x6ee, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0153`'s light stores (operand offsets `(dst, a, b)`): `*dst =
/// *a - *b`, or `k - *b` for the `(dst, b)` pairs with a constant.
fn levels_0153<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ops: &[(u32, u32, u32)],
    k: f32,
    ks: &[(u32, u32)],
) {
    let w = |o: u32| img.word(f + o);
    for &(dst, b) in ks {
        put(g, w(dst), sub(k, f32_at(g, w(b))));
    }
    for &(dst, a, b) in ops {
        put(g, w(dst), sub(f32_at(g, w(a)), f32_at(g, w(b))));
    }
}

/// `magic_0153`'s static node at operand `n` appended to the DLL's list
/// (the immediate stores at `stores`, the pass halfword at `pass`; the
/// list's last read at `last`, kept at `prev`, compared with `head`, the
/// first written by `first` or the old last's `+4` = the node through
/// `link`; the count `count` up).
#[allow(clippy::too_many_arguments)]
fn static_append_0153<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    stores: &[u32],
    pass: u32,
    (last, prev, head, first, link, count): (u32, u32, u32, u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let l = g.read::<u32>(w(last));
    crate::r11_fades::stores(g, img, f, &stores[..1]);
    g.write::<u16>(w(pass), 0xff);
    crate::r11_fades::stores(g, img, f, &stores[1..]);
    g.write::<u32>(w(prev), l);
    if l == w(head) {
        crate::r11_fades::stores(g, img, f, &[first]);
    } else {
        g.write::<u32>(l + 4, w(link));
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
}

/// `magic_0153`'s summon task (a callback, cdecl `(t)`; `R = *(t + 4)`,
/// its state `+0x18`, timers `+0x1c`/`+0x20` stepped by the constant at
/// operand `+0x1a`, `+0x28` counted up; the locals from `L = esp + 8`):
/// - 0: slot 719 `(table, 5, 0)` into slot 306 `(7, 0, p)`, the scale
///   node with the flag byte, slot 215 `(*G_A)` kept, the timed node, the
///   ambient node, the colour node and the owner node with their words,
///   the light block's start; on as 10;
/// - 10: slot 228 `(*G_A, 0xb8)`, the static node appended, a record
///   node, a node appended and the group inserted, an effect node; the
///   timer restarts, on as 0x14;
/// - 0x14 (the timer at the mark `+0x47c`): a particle node, on as 0x1e;
/// - 0x1e (the mark `+0x4d9`): the big node, the owner's handle kept, the
///   particle node, two parts (slot 719 `(table, 9, 0..1)`), the light
///   block's next words, an effect node; on as 0x28;
/// - 0x28 (the mark `+0x6a5`): the second static node, two nodes
///   appended, slot 268 on slot 215's actor, the group inserted, a
///   particle node, slot 306, an effect node, slot 239 `(*G_A, -5, 0)`,
///   the light levels; on as 0x29;
/// - 0x29: slot 411 `(*G_A)` answering, slot 213 `(*G_A, 0)`; then as
///   10000: three nodes leave (the second reset), the ambient node drops
///   and the task is unlinked.
///
/// Returns the last value in `eax`.
///
/// `magic_0153` 0x10007bf0 (2839 bytes).
#[allow(clippy::too_many_lines)] // the original's cases, in order
pub fn summon_task_0153<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let t = fr.arg(g, 0);
    let r = g.read::<u32>(t + 4);
    // push ebp; and esp, -8; sub esp, 0x18; push esi, edi
    let esp0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let l = |o: u32| esp0.wrapping_add(o);
    let at = |o: u32, ecx: u32, esp: u32| Frame {
        at: img.rel32_target(f + o),
        esp,
        eax: 0,
        ecx,
        edx: 0,
        xmm: [0; 4],
    };
    let rec = |g: &mut G, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(l(8) + 4 * i, v);
        }
    };
    let v = add(f32_at(g, r + 0x1c), f32_at(g, w(0x1a)));
    let state = g.read::<u32>(r + 0x18);
    let c = g.read::<u32>(r + 0x28).wrapping_add(1);
    g.write::<u32>(r + 0x28, c);
    put(g, r + 0x1c, v);
    let v = add(f32_at(g, r + 0x20), f32_at(g, w(0x32)));
    put(g, r + 0x20, v);
    let mut pc = match state {
        0 => 0x66,
        0xa => 0x2d5,
        0x14 => 0x478,
        0x1e => 0x4d5,
        0x28 => 0x6a1,
        0x29 => 0xa95,
        0x2710 => 0xac7,
        s if s > 0x1e => return Ret::eax(s.wrapping_sub(0x2710)),
        s => return Ret::eax(s),
    };
    // what the dispatch's `sub`s leave in eax
    let mut eax = if state > 0x1e { 0 } else { state };
    let one = |g: &G, op: u32| f32_at(g, w(op)) > f32_at(g, r + 0x20);
    loop {
        match pc {
            0x66 => {
                // call width: 3
                let p = g.call(slot(g, 0x67, 719), 0, &[w(0x76), 5, 0]).eax;
                // call width: 3
                g.call(slot(g, 0x81, 306), 0, &[7, 0, p]);
                crate::l3_common::scale_node(g, img, img.rel32_target(f + 0x9a), w(0x95));
                let b = g.read::<u8>(w(0x9f));
                let fl = g.read::<u8>(w(0xa5)) | 1;
                g.write::<u8>(w(0xa5), fl);
                let h = g.read::<u32>(w(0xac));
                g.write::<u32>(l(0) - 4, h);
                g.write::<u8>(w(0xb1), b);
                // call width: 1
                let a = g.call(slot(g, 0xb6, 215), 0, &[h]).eax;
                g.write::<u32>(w(0xc9), a);
                crate::m0153::timed_node(g, img, img.rel32_target(f + 0xce), w(0xc4));
                rec(g, &[w(0xda), w(0xe2)]);
                crate::m0153::ambient_node(g, img, img.rel32_target(f + 0xe7), l(8));
                stores(
                    g,
                    img,
                    f,
                    &[
                        0xef, 0xf9, 0x103, 0x10d, 0x117, 0x121, 0x12b, 0x135, 0x13f, 0x149,
                    ],
                );
                rec(g, &[w(0x157), w(0x15f)]);
                stores(g, img, f, &[0x163]);
                crate::s3_misc::colour_node(g, img, &at(0x16e, l(8), l(0) - 0xc));
                let v = g.read::<u32>(w(0x174)) | 1;
                g.write::<u32>(w(0x174), v);
                g.write::<u32>(l(0) - 8, w(0x182));
                stores(
                    g,
                    img,
                    f,
                    &[
                        0x186, 0x190, 0x19a, 0x1a4, 0x1ae, 0x1b8, 0x1c2, 0x1cc, 0x1d6, 0x1e0,
                    ],
                );
                crate::m0153::owner_node(g, img, img.rel32_target(f + 0x1eb), w(0x17d), w(0x182));
                let k = f32_at(g, w(0x1f3));
                g.write::<u8>(w(0x1fc), 0);
                stores(
                    g,
                    img,
                    f,
                    &[
                        0x201, 0x20b, 0x215, 0x21f, 0x229, 0x233, 0x23d, 0x247, 0x251, 0x25b,
                        0x265, 0x26f, 0x279, 0x283, 0x28d, 0x297, 0x2a8, 0x2b2, 0x2bc,
                    ],
                );
                put(g, w(0x2ca), sub(k, 0.0));
                g.write::<u32>(r + 0x18, 0xa);
                pc = 0x2d5;
            }
            0x2d5 => {
                let h = g.read::<u32>(w(0x2e1));
                // call width: 2
                g.call(slot(g, 0x2d6, 228), 0, &[h, 0xb8]);
                static_append_0153(
                    g,
                    img,
                    f,
                    &[0x2f5, 0x308, 0x312, 0x31c, 0x326],
                    0x302,
                    (0x2ee, 0x331, 0x336, 0x33c, 0x34b, 0x352),
                );
                stores(g, img, f, &[0x35a]);
                rec(g, &[w(0x368), w(0x370), w(0x378), w(0x380)]);
                crate::l3_common::record_node(g, img, img.rel32_target(f + 0x385), l(8));
                let (list, node) = (w(0x38a), w(0x38f));
                stores(g, img, f, &[0x393, 0x39d, 0x3a7]);
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x3b2), node, list, 0);
                g.write::<u32>(l(0x1c), node);
                stores(g, img, f, &[0x3be, 0x3c8, 0x3d2, 0x3dc]);
                rec(g, &[w(0x3ea), list, w(0x3f6), w(0x3fe), w(0x406)]);
                crate::nodes::node_insert(g, img, &at(0x40b, l(8), l(0) - 4));
                g.write::<u8>(w(0x415), 1);
                rec(g, &[w(0x41e), w(0x426), w(0x42e), w(0x436), w(0x43e), 0]);
                crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x44b), l(8));
                eax = g.read::<u32>(l(8));
                g.write::<u32>(eax, w(0x455));
                let v = g.read::<u32>(w(0x45b)) | 1;
                g.write::<u32>(w(0x45b), v);
                stores(g, img, f, &[0x460]);
                g.write::<u32>(r + 0x18, 0x14);
                g.write::<u32>(r + 0x20, 0);
                pc = 0x478;
            }
            0x478 => {
                if one(g, 0x47c) {
                    return Ret::eax(eax);
                }
                let e = g.read::<u32>(w(0x48b));
                rec(g, &[w(0x497), w(0x49f), w(0x4a7), e, w(0x4b3), 0]);
                stores(g, img, f, &[0x4bf]);
                eax = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x4ca), l(8));
                g.write::<u32>(r + 0x18, 0x1e);
                pc = 0x4d5;
            }
            0x4d5 => {
                if one(g, 0x4d9) {
                    return Ret::eax(eax);
                }
                rec(g, &[w(0x4ef), w(0x4f7), w(0x4ff), w(0x507), w(0x50f)]);
                crate::m0070::node_init_big_table(g, img, img.rel32_target(f + 0x514), l(8));
                let a = g.read::<u32>(w(0x519));
                g.write::<u32>(w(0x51e), a);
                stores(g, img, f, &[0x528]);
                rec(g, &[w(0x536), w(0x53e), 0, 0, w(0x552), w(0x55a)]);
                stores(g, img, f, &[0x55e]);
                crate::m0153::particle_node(g, img, img.rel32_target(f + 0x569), l(8));
                // call width: 3
                let p = g.call(slot(g, 0x56e, 719), 0, &[w(0x57d), 9, 0]).eax;
                g.write::<u32>(w(0x587), p);
                // call width: 3
                let p = g.call(slot(g, 0x58c, 719), 0, &[w(0x59b), 9, 1]).eax;
                let v = g.read::<u32>(w(0x5a3)) | 1;
                g.write::<u32>(w(0x5a3), v);
                g.write::<u32>(w(0x5b0), p);
                stores(
                    g,
                    img,
                    f,
                    &[
                        0x5b4, 0x5be, 0x5c8, 0x5d2, 0x5dc, 0x5e6, 0x5f0, 0x5fa, 0x604, 0x60e,
                        0x618, 0x622, 0x62c, 0x636, 0x640,
                    ],
                );
                rec(g, &[w(0x64e), w(0x656), w(0x65e), w(0x666), w(0x66e), 2]);
                crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x67b), l(8));
                eax = g.read::<u32>(l(8));
                g.write::<u32>(eax, w(0x685));
                let v = g.read::<u32>(w(0x68b)) | 1;
                g.write::<u32>(w(0x68b), v);
                stores(g, img, f, &[0x690]);
                g.write::<u32>(r + 0x18, 0x28);
                pc = 0x6a1;
            }
            0x6a1 => {
                if one(g, 0x6a5) {
                    return Ret::eax(eax);
                }
                static_append_0153(
                    g,
                    img,
                    f,
                    &[0x6b8, 0x6cb, 0x6d5, 0x6df, 0x6e9],
                    0x6c5,
                    (0x6b4, 0x6f4, 0x6f9, 0x6ff, 0x72b, 0x737),
                );
                let a = g.read::<u32>(w(0x730)).wrapping_add(0x8000);
                g.write::<u32>(w(0x741), a);
                let a = g.read::<u32>(w(0x746));
                g.write::<u32>(w(0x74b), a);
                let a = g.read::<u32>(w(0x750));
                let list = w(0x755);
                stores(g, img, f, &[0x75e]);
                g.write::<u32>(w(0x769), a);
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x76e), w(0x75a), list, a);
                stores(g, img, f, &[0x777, 0x781, 0x78b, 0x795]);
                crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x7a0), w(0x773), list, a);
                let b = u32::from(g.read::<u8>(w(0x7a7)));
                // call width: 1
                let actor = g.call(slot(g, 0x7ad, 215), 0, &[b]).eax;
                // call width: 3
                g.call(slot(g, 0x7c5, 268), 0, &[actor, 0x11, w(0x7bd)]);
                stores(g, img, f, &[0x7d8]);
                rec(
                    g,
                    &[w(0x7e6), w(0x7ee), w(0x7f6), w(0x7fe), w(0x806), w(0x80e)],
                );
                crate::nodes::node_insert(g, img, &at(0x813, l(8), l(0) - 4));
                let e = g.read::<u32>(w(0x818));
                g.write::<u8>(w(0x822), 1);
                rec(g, &[w(0x82b), w(0x833), w(0x83b), e, w(0x847), 1]);
                stores(g, img, f, &[0x853]);
                crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x85e), l(8));
                // call width: 3
                let p = g.call(slot(g, 0x863, 719), 0, &[w(0x872), 5, 0]).eax;
                // call width: 3
                g.call(slot(g, 0x87d, 306), 0, &[7, 1, p]);
                rec(g, &[w(0x898), w(0x8a0), w(0x8a8), w(0x8b0), w(0x8b8), 1]);
                crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x8c5), l(8));
                let node = g.read::<u32>(l(8));
                g.write::<u32>(node, w(0x8d1));
                let v = g.read::<u32>(w(0x8d7)) | 1;
                g.write::<u32>(w(0x8d7), v);
                let h = g.read::<u32>(w(0x8e5));
                stores(g, img, f, &[0x8e9]);
                // call width: 3
                g.call(slot(g, 0x8dd, 239), 0, &[h, 0xffff_fffb, 0]);
                let k = f32_at(g, w(0x8ff));
                levels_0153(g, img, f, &[], k, &[(0x920, 0x90a)]);
                let fl = (g.read::<u32>(w(0x90f)) & !1) | 4;
                levels_0153(g, img, f, &[(0x960, 0x928, 0x930)], k, &[]);
                stores(g, img, f, &[0x934, 0x93e, 0x948, 0x952]);
                levels_0153(g, img, f, &[], k, &[(0x994, 0x96b), (0x9ac, 0x973)]);
                stores(g, img, f, &[0x977]);
                g.write::<u32>(w(0x982), fl);
                stores(g, img, f, &[0x986]);
                levels_0153(g, img, f, &[(0x9d0, 0x99c, 0x9a4)], k, &[]);
                stores(g, img, f, &[0x9b8, 0x9c2]);
                levels_0153(g, img, f, &[(0xa10, 0x9d8, 0x9e0)], k, &[]);
                stores(g, img, f, &[0x9e4, 0x9ee, 0x9f8, 0xa02]);
                let k2 = f32_at(g, w(0x9b4));
                levels_0153(
                    g,
                    img,
                    f,
                    &[(0xa28, 0xa18, 0xa20), (0xa40, 0xa30, 0xa38)],
                    k2,
                    &[],
                );
                levels_0153(
                    g,
                    img,
                    f,
                    &[(0xa8e, 0xa76, 0xa7e)],
                    k2,
                    &[(0xa53, 0xa4b), (0xa6e, 0xa5e), (0xa86, 0xa66)],
                );
                let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, s);
                pc = 0xa95;
            }
            0xa95 => {
                let h = g.read::<u32>(w(0xa9c));
                // call width: 1
                if g.call(slot(g, 0xa96, 411), 0, &[h]).eax == 0 {
                    return Ret::eax(0);
                }
                let h = g.read::<u32>(w(0xab8));
                // call width: 2
                eax = g.call(slot(g, 0xab0, 213), 0, &[h, 0]).eax;
                pc = 0xac7;
            }
            _ => {
                let e = crate::sa_misc::leave(g, w(0xac8), eax);
                let node = w(0xad2);
                crate::sa_misc::leave(g, node, e);
                g.write::<u32>(l(8), node);
                let e = crate::s3_nodes::leave_and_reset(g, img, &at(0xae4, l(8), l(0) - 4));
                let last = w(0xae9);
                crate::sa_misc::leave(g, last, e);
                crate::m0153::node_drop(g, img, img.rel32_target(f + 0xaf3), last);
                let n = g.read::<u32>(t);
                let nx = g.read::<u32>(n + 4);
                let pv = g.read::<u32>(n + 8);
                g.write::<u32>(nx + 8, pv);
                let pv = g.read::<u32>(n + 8);
                let nx = g.read::<u32>(n + 4);
                g.write::<u32>(pv + 4, nx);
                g.write::<u32>(n + 8, n);
                g.write::<u32>(n + 4, n);
                return Ret::eax(nx);
            }
        }
    }
}

/// [`summon_task_0153`] (the `rel32`s: the nodes).
pub const SUMMON_TASK_0153: Sig = Sig {
    len: 2839,
    hash: 0x373a_e05f_1287_273d,
    calls: &[
        (0x9a, Callee::Sig(&crate::l3_common::SCALE_NODE)),
        (0xce, Callee::Sig(&crate::m0153::TIMED_NODE)),
        (0xe7, Callee::Sig(&crate::m0153::AMBIENT_NODE)),
        (0x16e, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x1eb, Callee::Sig(&crate::m0153::OWNER_NODE)),
        (0x385, Callee::Sig(&crate::l3_common::RECORD_NODE)),
        (0x3b2, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x40b, Callee::Sig(&crate::nodes::INSERT)),
        (0x44b, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x4ca, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x514, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0x569, Callee::Sig(&crate::m0153::PARTICLE_NODE)),
        (0x67b, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x76e, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x7a0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x813, Callee::Sig(&crate::nodes::INSERT)),
        (0x85e, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x8c5, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0xacd, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xad7, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xae4, Callee::Sig(&crate::s3_nodes::LEAVE_AND_RESET)),
        (0xaee, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xaf3, Callee::Sig(&crate::m0153::NODE_DROP)),
    ],
    imports: &[],
};

/// `magic_0389`'s drifting sparks (internal, `ecx` = the group; an
/// aligned cookie frame, `L(o)` = the frame's `esp + o`; `esp` the
/// return address slot): the pass walk
/// ([`walk`](crate::nodes::walk)); with the flag set, the owner `N`'s
/// child `+0x48` visited through its method `+0x1c` (a group `[child,
/// mode]` at `L(0x24)`). With `N + 0x5c` clear the countdown `N + 0x58`
/// steps; at 0 it restarts at `1 + (r & 3)` (slot 324) and `2..5` sparks
/// are taken from the rings ([`ring_take`](crate::m0070::ring_take)), the
/// model's place from slot 237 and slot 235 clearing `L(0x98)`: each
/// spark (0x40 bytes) gets its colour, size and drop, a turn from two
/// more draws ([`rotation_19d0`](crate::sb_matrix::rotation_19d0) into
/// `L(0x98)`), its place (slot 229 with (10, 0, 0, 1)) and slot 248. With
/// `N + 0x5c` set and no sparks out (`N + 0x54`) the node leaves. Then
/// the live sparks (byte `+0x34`) are drawn: slots 229, 235 and 330 (the
/// spark's angle), faded in to 0.7, stepped through their motion
/// sequence ([`motion_sequence_step`](crate::s6_misc::motion_sequence_step),
/// whose answer keeps them alive) and dropped. Returns the saved draw
/// word (the global at operand `+0x2fb`) put back.
///
/// `magic_0389` 0x10001a10 (1336 bytes).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn sparks_0389<G: Game>(g: &mut G, img: &Image, f: u32, grp: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -16; sub esp, 0x128; push esi, edi
    let esp0 = (esp.wrapping_sub(4) & !0xf)
        .wrapping_sub(0x128)
        .wrapping_sub(8);
    let l = |o: u32| esp0.wrapping_add(o);
    let owner = g.read::<u32>(grp);
    g.write::<u32>(l(0x14), owner);
    // eax: the cookie word, `^ esp`, under the walk's mode byte
    let cookie = g.read::<u32>(w(0xd)) ^ esp0.wrapping_add(8);
    let (eax, walked) = crate::nodes::walk(g, grp, owner, cookie);
    if walked.is_none() {
        return eax;
    }
    let n = g.read::<u32>(grp);
    g.write::<u32>(l(0x20), n);
    let c1 = g.read::<u32>(n + 0x48);
    g.write::<u32>(l(0x24), c1);
    let mode = g.read::<u8>(grp + 5);
    g.write::<u8>(l(0x29), mode);
    let m = g.read::<u32>(g.read::<u32>(c1) + 0x1c);
    let e = g.call(m, l(0x24), &[l(0x24)]).eax;
    let spawn = if g.read::<u32>(n + 0x5c) != 0 {
        if g.read::<u32>(n + 0x54) == 0 {
            return crate::sa_misc::leave(g, n, e);
        }
        false
    } else {
        let c = g.read::<u32>(n + 0x58);
        g.write::<u32>(n + 0x58, c.wrapping_sub(1));
        c == 0
    };
    if spawn {
        // call width: 0
        let r = g.call(slot(g, 0xb3, 324), 0, &[]).eax;
        g.write::<u32>(n + 0x58, (r & 3) + 1);
        let k = ((r.cast_signed() >> 2) & 3) + 2;
        g.write::<u32>(l(0x3c), n);
        g.write::<u16>(l(0x40), k.cast_unsigned() as u16);
        crate::m0070::ring_take(g, l(0x3c));
        let model = g.read::<u32>(g.read::<u32>(n + 0x48) + 0x54);
        // call width: 4
        g.call(slot(g, 0xfa, 237), 0, &[model, 0x32, 0, l(0xd8)]);
        // call width: 1
        g.call(slot(g, 0x10f, 235), 0, &[l(0x98)]);
        g.write::<u32>(l(0x90), 0);
        g.write::<u32>(l(0x8c), 0);
        g.write::<u32>(l(0x94), 0x3f80_0000);
        g.write::<u32>(l(0x2c), 0);
        for ring in 0..2u32 {
            g.write::<u32>(l(0x1c), ring);
            let count = u32::from(g.read::<u16>(l(0x42) + 2 * ring));
            let mut e = g.read::<u32>(l(0x48) + 4 * ring);
            for _ in 0..count {
                g.write::<u32>(e + 0x10, 0);
                let rnd = |g: &mut G, op: u32| {
                    // call width: 0
                    crate::m0018::st0_f32(g.call(slot(g, op, 307), 0, &[]).st0)
                };
                let x = sub(rnd(g, 0x167), f32_at(g, w(0x181)));
                // call width: 0
                let r = g.call(slot(g, 0x186, 324), 0, &[]).eax;
                let even = u32::from(r.trailing_zeros() >= 2);
                g.write::<u16>(e + 0x12, r as u16);
                g.write::<u32>(e + 0x28, 0x8080_8080);
                g.write::<u32>(e + 0x24, even);
                let v = if even != 0 {
                    g.write::<u32>(e + 0x30, 0);
                    add(mul(x, f32_at(g, w(0x1be))), f32_at(g, w(0x1cd)))
                } else {
                    let v = add(mul(x, f32_at(g, w(0x1d7))), f32_at(g, w(0x1e2)));
                    g.write::<u32>(e + 0x30, r & 0xffff);
                    v
                };
                let v = mul(mul(v, f32_at(g, w(0x1ed))), f32_at(g, w(0x1fe)));
                g.write::<u8>(e + 0x34, 1);
                g.write::<u32>(e + 0x2c, 0);
                put(g, e + 0x20, add(v, f32_at(g, w(0x20d))));
                let y = mul(sub(rnd(g, 0x1f2), f32_at(g, w(0x22c))), f32_at(g, w(0x239)));
                put(g, e + 0x18, y);
                // call width: 0
                let a1 = g.call(slot(g, 0x231, 324), 0, &[]).eax;
                g.write::<u32>(l(0x30), a1);
                // call width: 0
                let a2 = g.call(slot(g, 0x24f, 324), 0, &[]).eax & 0x7fff;
                g.write::<u32>(l(0x34), a2);
                crate::sb_matrix::rotation_19d0(
                    g,
                    img,
                    img.rel32_target(f + 0x270),
                    l(0x98),
                    l(0x2c),
                );
                g.write::<u32>(l(0x88), 0x4120_0000);
                // call width: 3
                g.call(slot(g, 0x285, 229), 0, &[e, l(0x98), l(0x88)]);
                // call width: 3
                g.call(slot(g, 0x2aa, 248), 0, &[e, l(0xd8), e]);
                g.write::<u32>(e + 0xc, 0x3f80_0000);
                e += 0x40;
            }
        }
    }
    // the draw: both rings' live sparks
    g.write::<u32>(l(0x6c), l(0xe8));
    g.write::<u32>(l(0x60), l(0xe8));
    g.write::<u16>(l(0x52), 0x80);
    let saved = g.read::<u32>(w(0x2fb));
    g.write::<u32>(l(0x64), saved);
    for o in [
        0x9cu32, 0xa0, 0xa4, 0xa8, 0xb0, 0xb4, 0xb8, 0xbc, 0xc4, 0x58,
    ] {
        g.write::<u32>(l(o), 0);
    }
    g.write::<u8>(l(0x68), 0);
    g.write::<u32>(n + 0x54, 0);
    let mut counts = n + 0x2c;
    let mut rings = n + 0x24;
    for left in (1..=2u32).rev() {
        g.write::<u32>(l(0x14), counts);
        g.write::<u32>(l(0x1c), rings);
        g.write::<u32>(l(0x18), left);
        let mut e = g.read::<u32>(rings);
        let mut j = 0u32;
        while j < u32::from(g.read::<u16>(counts)) {
            if g.read::<u8>(e + 0x34) != 0 {
                // call width: 3
                g.call(slot(g, 0x3bc, 229), 0, &[l(0xc8), w(0x3b6), e]);
                let v = mul(f32_at(g, e + 0x20), f32_at(g, e + 0x2c));
                for o in [0xc0u32, 0xac, 0x98] {
                    put(g, l(o), v);
                }
                // call width: 1
                g.call(slot(g, 0x3de, 235), 0, &[l(0xe8)]);
                #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
                let angle = f64::from(g.read::<u32>(e + 0x30)) as f32;
                // call width: 3
                g.call(slot(g, 0x436, 330), 0, &[l(0xe8), l(0x98), angle.to_bits()]);
                let a = f32_at(g, e + 0x2c);
                if f32_at(g, w(0x44b)) > a {
                    let a = add(a, f32_at(g, w(0x45b)));
                    put(g, e + 0x2c, a);
                    if a > f32_at(g, w(0x462)) {
                        g.write::<u32>(e + 0x2c, 0x3f33_3333);
                    }
                }
                let colour = g.read::<u32>(e + 0x28);
                g.write::<u32>(l(0x54), colour);
                let which = g.read::<u32>(e + 0x24);
                let table = g.read::<u32>(n + 0x4c + 4 * which);
                // sub esp, 8; push L(0x50): the step's argument
                g.write::<u32>(esp0.wrapping_sub(0xc), l(0x50));
                let ms = Frame {
                    at: img.rel32_target(f + 0x492),
                    esp: esp0.wrapping_sub(0x10),
                    eax: n,
                    ecx: e + 0x10,
                    edx: table,
                    xmm: [0; 4],
                };
                let alive = crate::s6_misc::motion_sequence_step(g, img, &ms);
                g.write::<u8>(e + 0x34, alive as u8);
                let dy = f32_at(g, e + 0x18);
                let y = add(dy, f32_at(g, e + 4));
                let dy = add(dy, f32_at(g, w(0x4b2)));
                put(g, e + 4, y);
                put(g, e + 0x18, dy);
                let c = g.read::<u32>(n + 0x54).wrapping_add(1);
                g.write::<u32>(n + 0x54, c);
            }
            j += 1;
            e += 0x40;
        }
        rings += 4;
        counts += 2;
    }
    let v = g.read::<u32>(l(0x64));
    g.write::<u32>(w(0x4fb), v);
    v
}

/// [`sparks_0389`] (the `rel32`s: the reservation, the turn, the motion
/// step, the cookie checks, the leave).
pub const SPARKS_0389: Sig = Sig {
    len: 1336,
    hash: 0xc551_7a3c_506e_05f3,
    calls: &[
        (0xe3, Callee::Sig(&crate::m0070::RING_TAKE)),
        (0x270, Callee::Sig(&crate::sb_matrix::ROTATION_19D0)),
        (0x492, Callee::Sig(&crate::s6_misc::MOTION_SEQUENCE_STEP)),
        (0x50b, Callee::Any),
        (0x520, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x530, Callee::Any),
    ],
    imports: &[],
};

/// The 12-byte thunk of [`sparks_0389`] (cdecl `(group)`: `mov ecx,
/// [ebp + 8]` and a tail jump).
///
/// `magic_0389` 0x10001f50.
pub fn sparks_thunk_0389<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let grp = fr.arg(g, 0);
    Ret::eax(sparks_0389(
        g,
        img,
        img.rel32_target(fr.at + 8),
        grp,
        fr.esp,
    ))
}

/// [`sparks_thunk_0389`].
pub const SPARKS_THUNK_0389: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(0x8, Callee::Sig(&SPARKS_0389))],
    imports: &[],
};

/// `magic_0389`'s light levels (operand offsets): `*dst = k - *b` for
/// `ks`, `*dst = *a - *b` for `ops`.
fn levels_0389<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    k: f32,
    ks: &[(u32, u32)],
    ops: &[(u32, u32, u32)],
) {
    let w = |o: u32| img.word(f + o);
    for &(dst, b) in ks {
        put(g, w(dst), sub(k, f32_at(g, w(b))));
    }
    for &(dst, a, b) in ops {
        put(g, w(dst), sub(f32_at(g, w(a)), f32_at(g, w(b))));
    }
}

/// `magic_0389`'s summon task (a callback, cdecl `(t)`; `R = *(t + 4)`,
/// its state `+0x18` and count `+0x28`, which every way out counts up;
/// the locals from `L = esp + 0x10`):
/// - 0: the colour node and the summon's model node; on as 10;
/// - 10: the light levels at 0.5, the model's mode switch; state 11;
/// - 11 (count 30): three static nodes appended (the light, its place
///   from slot 262 and slot 12's height less the constant at `+0x10d`,
///   the scale and a third), the owner words and a particle node; the
///   count restarts, on as 0x14;
/// - 0x14 (count 150): the static strip, two parts (slot 719 `(table, 9,
///   0)`); the next state; 0x15 (count 90): the light levels at 1.0, the
///   flag word set; the next state; 0x16: the model's id, at a model
///   state 10000;
/// - 10000: the model set up, the task unlinked.
///
/// Returns the last value in `eax`.
///
/// `magic_0389` 0x10006f10 (1608 bytes).
#[allow(clippy::too_many_lines)] // the original's cases, in order
pub fn summon_task_0389<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let t = fr.arg(g, 0);
    let r = g.read::<u32>(t + 4);
    // push ebp; and esp, -8; sub esp, 0x34; push ebx, esi, edi
    let esp0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x34 + 12);
    let l = |o: u32| esp0.wrapping_add(o);
    let at = |o: u32, ecx: u32| Frame {
        at: img.rel32_target(f + o),
        esp: esp0.wrapping_sub(4),
        eax: 0,
        ecx,
        edx: 0,
        xmm: [0; 4],
    };
    let count_up = |g: &mut G| {
        let c = g.read::<u32>(r + 0x28).wrapping_add(1);
        g.write::<u32>(r + 0x28, c);
    };
    let count = |g: &G| g.read::<i32>(r + 0x28);
    let mut state = g.read::<u32>(r + 0x18);
    loop {
        match state {
            0 => {
                g.write::<u32>(l(0x10), w(0x2b5));
                g.write::<u32>(l(0x14), w(0x2bd));
                stores(g, img, f, &[0x2c1]);
                crate::s3_misc::colour_node(g, img, &at(0x2cc, l(0x10)));
                g.write::<u32>(l(0x18), w(0x2d8));
                g.write::<u32>(l(0x1c), w(0x2e0));
                g.write::<u32>(l(0x20), 0x14);
                g.write::<u8>(l(0x24), 1);
                stores(g, img, f, &[0x2f1]);
                crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0x2fc), l(0x18));
                g.write::<u32>(r + 0x18, 0xa);
                state = 0xa;
            }
            0xa => {
                let k = f32_at(g, w(0x36f));
                levels_0389(g, img, f, k, &[(0x3a7, 0x37a)], &[]);
                let v = g.read::<u32>(w(0x380)) & !1;
                g.write::<u32>(w(0x380), v);
                stores(g, img, f, &[0x385, 0x38f, 0x399]);
                levels_0389(g, img, f, k, &[(0x3e2, 0x3b2)], &[]);
                stores(g, img, f, &[0x3b6, 0x3c0, 0x3ca, 0x3d4]);
                levels_0389(g, img, f, k, &[(0x409, 0x3ed)], &[]);
                stores(g, img, f, &[0x3f1, 0x3fb]);
                levels_0389(
                    g,
                    img,
                    f,
                    k,
                    &[
                        (0x41c, 0x414),
                        (0x42f, 0x427),
                        (0x44a, 0x43a),
                        (0x462, 0x442),
                    ],
                    &[(0x46a, 0x452, 0x45a)],
                );
                let e = crate::m0389::mode_switch(g, img, &at(0x46f, 0)).eax;
                let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, s);
                g.write::<u32>(r + 0x28, 1);
                return Ret::eax(e);
            }
            0xb => {
                if count(g) < 0x1e {
                    count_up(g);
                    return Ret::eax(0);
                }
                // the light node appended to the DLL's list
                let last = g.read::<u32>(w(0x5a));
                g.write::<u16>(w(0x61), 0xff);
                stores(g, img, f, &[0x67, 0x71, 0x7b, 0x85]);
                g.write::<u32>(w(0x90), last);
                if last == w(0x95) {
                    stores(g, img, f, &[0x9b]);
                } else {
                    g.write::<u32>(last + 4, w(0xaa));
                }
                let n = g.read::<u16>(w(0xb6)).wrapping_add(1);
                g.write::<u16>(w(0xb6), n);
                let h = g.read::<u32>(w(0xcb));
                stores(g, img, f, &[0xcf, 0xd9]);
                // call width: 4
                g.call(slot(g, 0xaf, 262), 0, &[h, w(0xc5), w(0xc0), w(0xbb)]);
                let h = g.read::<u32>(w(0xf2));
                // call width: 1
                let y = crate::m0018::st0_f32(g.call(slot(g, 0xec, 12), 0, &[h]).st0);
                put(g, w(0xfd), y);
                let y = sub(f32_at(g, w(0x105)), f32_at(g, w(0x10d)));
                let last = g.read::<u32>(w(0x112));
                stores(g, img, f, &[0x119]);
                g.write::<u16>(w(0x126), 0xff);
                put(g, w(0x130), y);
                stores(g, img, f, &[0x134, 0x13e, 0x148, 0x152]);
                g.write::<u32>(w(0x15d), last);
                if last == w(0x162) {
                    stores(g, img, f, &[0x168]);
                } else {
                    g.write::<u32>(last + 4, w(0x177));
                }
                let ax = g.read::<u16>(w(0x17d)).wrapping_add(1);
                stores(g, img, f, &[0x183, 0x18d, 0x197, 0x1a1, 0x1ab, 0x1b5]);
                g.write::<u16>(w(0x1c2), 0xff);
                stores(g, img, f, &[0x1c8, 0x1d2, 0x1dc, 0x1e6, 0x1f0]);
                if w(0x27) == w(0x1fc) {
                    stores(g, img, f, &[0x202]);
                } else {
                    stores(g, img, f, &[0x20e]);
                }
                let ax = ax.wrapping_add(1);
                g.write::<u16>(w(0x21c), ax);
                let e = g.read::<u32>(w(0x221));
                stores(g, img, f, &[0x229, 0x233, 0x23d, 0x247, 0x251]);
                g.write::<u8>(w(0x25d), 0);
                for (i, v) in (0u32..).zip([w(0x266), w(0x26e), w(0x276), e, w(0x282), 0]) {
                    g.write::<u32>(l(0x28) + 4 * i, v);
                }
                stores(g, img, f, &[0x28e]);
                crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x299), l(0x28));
                g.write::<u32>(r + 0x28, 0);
                g.write::<u32>(r + 0x18, 0x14);
                state = 0x14;
            }
            0x14 => {
                if count(g) < 0x96 {
                    count_up(g);
                    return Ret::eax(0x14);
                }
                // sub esp, 0x10
                let mut ss = at(0x498, 0);
                ss.esp = esp0.wrapping_sub(0x14);
                crate::m0389::static_strip(g, img, &ss);
                // call width: 3
                let p = g.call(slot(g, 0x49d, 719), 0, &[w(0x4ac), 9, 0]).eax;
                g.write::<u32>(w(0x4b6), p);
                // call width: 3
                let p = g.call(slot(g, 0x4bb, 719), 0, &[w(0x4ca), 9, 0]).eax;
                g.write::<u32>(w(0x4d1), p);
                let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, s);
                g.write::<u32>(r + 0x28, 1);
                return Ret::eax(p);
            }
            0x15 => {
                if count(g) < 0x5a {
                    count_up(g);
                    return Ret::eax(0);
                }
                let one = f32_at(g, w(0x317));
                levels_0389(g, img, f, one, &[(0x547, 0x52a)], &[]);
                let fl = (g.read::<u32>(w(0x52f)) & !1) | 4;
                stores(g, img, f, &[0x539]);
                levels_0389(g, img, f, one, &[(0x582, 0x552)], &[]);
                stores(g, img, f, &[0x556, 0x560, 0x56a, 0x574]);
                levels_0389(g, img, f, one, &[(0x5bd, 0x58d)], &[]);
                stores(g, img, f, &[0x591, 0x59b, 0x5a5, 0x5af]);
                levels_0389(g, img, f, one, &[(0x5e9, 0x5c8)], &[]);
                g.write::<u32>(w(0x5cd), fl);
                stores(g, img, f, &[0x5d1, 0x5db]);
                levels_0389(
                    g,
                    img,
                    f,
                    one,
                    &[(0x5fc, 0x5f4), (0x617, 0x607), (0x62f, 0x60f)],
                    &[(0x637, 0x61f, 0x627)],
                );
                let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, s);
                count_up(g);
                return Ret::eax(fl);
            }
            0x16 => {
                g.write::<u32>(l(0x10), w(0x4f4));
                let e = crate::sa_misc::model_id(g, img, img.rel32_target(f + 0x4f9), l(0x10));
                if g.read::<u32>(l(0x14)) != 0 {
                    g.write::<u32>(r + 0x18, 0x2710);
                }
                count_up(g);
                return Ret::eax(e);
            }
            0x2710 => {
                g.write::<u32>(l(0x10), w(0x33e));
                crate::sa_summon::model_setup(g, img, &at(0x343, l(0x10)));
                let n = g.read::<u32>(t);
                let nx = g.read::<u32>(n + 4);
                let pv = g.read::<u32>(n + 8);
                g.write::<u32>(nx + 8, pv);
                let pv = g.read::<u32>(n + 8);
                let nx = g.read::<u32>(n + 4);
                g.write::<u32>(pv + 4, nx);
                g.write::<u32>(n + 8, n);
                g.write::<u32>(n + 4, n);
                count_up(g);
                return Ret::eax(nx);
            }
            s => {
                count_up(g);
                return Ret::eax(if s > 0x14 {
                    s.wrapping_sub(0x2710)
                } else {
                    s.wrapping_sub(0xb)
                });
            }
        }
    }
}

/// [`summon_task_0389`] (the `rel32`s: the nodes, the model calls).
pub const SUMMON_TASK_0389: Sig = Sig {
    len: 1608,
    hash: 0xed5a_7d91_a597_37d1,
    calls: &[
        (0x299, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x2cc, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x2fc, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x343, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x46f, Callee::Sig(&crate::m0389::MODE_SWITCH)),
        (0x498, Callee::Sig(&crate::m0389::STATIC_STRIP)),
        (0x4f9, Callee::Sig(&crate::sa_misc::MODEL_ID)),
    ],
    imports: &[],
};
