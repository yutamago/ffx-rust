//! The camera and screen shakes (`ffx_battle::shake`) against the original,
//! as in `battle.rs`: results, every written byte and the `brnd`
//! generator's state; the CRT's cosine and sine as the shared stand-ins
//! (`battle_common::crt`).

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::*;
use ffx_battle::pos::bridge::Trig;
use ffx_battle::shake::bridge as sh;
use ffx_oracle::Oracle;

/// A float's bits: mostly ordinary, sometimes zero, tiny, huge, infinite
/// or NaN.
fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(12) {
        0 => [0, 0x8000_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000, 1][rng.below(6) as usize],
        1 => quiet(rng.next()),
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 97.0).to_bits(),
    }
}

/// The patched thunks on a double, as the shake code calls them: `x²` and
/// `2x` at 53 bits, stored as `f32`.
fn trig(t: Trig, x: f64) -> u32 {
    use ffx_bridge::x87::{add, mul, store};
    match t {
        Trig::Cos => store(mul(x, x)),
        Trig::Sin => store(add(x, x)),
        Trig::Atan => store(-x),
    }
}

/// A random shake record: every kind (and 0, 10), short counts around 0,
/// lengths of both signs, ordinary spring values.
fn record(rng: &mut Xorshift) -> Vec<u8> {
    let mut r = vec![0u8; 0x24];
    r[0] = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 10, 0xff][rng.below(13) as usize];
    r[1] = if rng.below(3) == 0 { 0 } else { rng.byte() };
    let amp = rng.next().to_le_bytes();
    r[2..4].copy_from_slice(&amp[..2]);
    let left = i16::try_from(rng.range(-3, 6)).unwrap();
    r[4..6].copy_from_slice(&left.to_le_bytes());
    let length = match rng.below(5) {
        0 => 0i16,
        1 => i16::try_from(rng.range(-40, 0)).unwrap(),
        _ => i16::try_from(rng.range(1, 60)).unwrap(),
    };
    r[6..8].copy_from_slice(&length.to_le_bytes());
    for o in [8usize, 0xc, 0x10, 0x14, 0x18, 0x1c] {
        r[o..o + 4].copy_from_slice(&float(rng).to_le_bytes());
    }
    r[0x20] = u8::try_from(rng.below(6)).unwrap();
    r[0x21] = u8::try_from(rng.below(4)).unwrap();
    r
}

/// A camera request ID, mostly for the first few cameras.
fn camera(rng: &mut Xorshift) -> u32 {
    if rng.below(8) == 0 {
        rng.word() & 0x00ff_ffff
    } else {
        rng.below(3) | (rng.below(3) << 8)
    }
}

#[allow(clippy::too_many_lines)]
fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    battle_common::crt::patch(a);
    battle_common::crt::patch(b);
    let buf = scratch(a, b, 0x100);
    let cams = a.rebase(0x0113_7540);
    let screens = a.rebase(0x0113_7000);
    for _ in 0..4000 {
        // one camera side's three shakes and a screen's two, freshly drawn
        let (cam, side) = (rng.below(3), rng.below(2));
        let base = cams + cam * 0xda0 + side * 0x61c + 0x5e0;
        for k in 0..3 {
            let r = record(rng);
            poke(a, b, base + 0x24 * k, &r);
        }
        let s = rng.below(3);
        for k in 0..2 {
            let r = record(rng);
            poke(a, b, screens + s * 0x1c0 + 0x16c + 0x24 * k, &r);
        }
        // the camera's request slots, for MsCameraGetID
        if rng.below(2) == 0 {
            fill(a, b, rng, cams + cam * 0xda0 + 0xc68, 0x20);
        }
        let angle = float(rng);
        for k in 0..3 {
            let rec = base + 0x24 * k;
            let out = buf + 4 * k;
            twin(
                "MsCameraShakeProcess",
                a,
                b,
                0x007c_50f0,
                &[rec, angle, out],
                0,
                |m| {
                    sh::ms_camera_shake_process(m, &trig, rec, angle, out);
                    0
                },
            );
        }
        let rec = screens + s * 0x1c0 + 0x16c + 0x24 * rng.below(2);
        twin(
            "MsCameraShakeProcess",
            a,
            b,
            0x007c_50f0,
            &[rec, angle, buf + 0x10],
            0,
            |m| {
                sh::ms_camera_shake_process(m, &trig, rec, angle, buf + 0x10);
                0
            },
        );
        same("MsCameraShakeProcess", a, b);
        let c = if rng.below(2) == 0 {
            cam | (rng.below(3) << 8)
        } else {
            camera(rng)
        };
        let sd = if rng.below(2) == 0 { side } else { rng.word() };
        twin(
            "MsCameraCheckShake",
            a,
            b,
            0x007c_4f10,
            &[c, sd],
            u32::MAX,
            |m| sh::ms_camera_check_shake(m, c, sd).cast_unsigned(),
        );
        let i = if rng.below(6) == 0 {
            rng.word()
        } else {
            rng.below(3)
        };
        twin(
            "MsScreenCheckShake",
            a,
            b,
            0x007c_5680,
            &[i],
            u32::MAX,
            |m| sh::ms_screen_check_shake(m, i).cast_unsigned(),
        );
        let start = sh::Start {
            kind: rng.byte(),
            value: float(rng),
            length: u16::try_from(rng.below(0x1_0000)).unwrap(),
            amplitude: u16::try_from(rng.below(0x1_0000)).unwrap(),
            range: rng.byte(),
        };
        let which = rng.word();
        match rng.below(4) {
            0 => {
                let args = [
                    c,
                    sd,
                    which,
                    u32::from(start.kind),
                    start.value,
                    u32::from(start.length),
                    u32::from(start.amplitude),
                    u32::from(start.range),
                ];
                twin("MsCameraShake", a, b, 0x007c_4fc0, &args, u32::MAX, |m| {
                    sh::ms_camera_shake(m, c, sd, which, start)
                });
            }
            1 => {
                twin(
                    "MsCameraResetShake",
                    a,
                    b,
                    0x007c_4f50,
                    &[c, sd],
                    u32::MAX,
                    |m| sh::ms_camera_reset_shake(m, c, sd),
                );
            }
            2 => {
                let call = [
                    i,
                    which,
                    u32::from(start.kind),
                    start.value,
                    u32::from(start.length),
                    u32::from(start.amplitude),
                    u32::from(start.range),
                ];
                twin("MsScreenShake", a, b, 0x007c_56f0, &call, 0, |m| {
                    sh::ms_screen_shake(m, i, which, start);
                    0
                });
            }
            _ => {
                let length = if rng.below(4) == 0 { 0 } else { rng.word() };
                let range = match rng.below(4) {
                    0 => 0,
                    1 => rng.word(),
                    2 => rng.range(-300, 0).cast_unsigned(),
                    _ => rng.range(1, 300).cast_unsigned(),
                };
                let (speed, pull) = (float(rng), float(rng));
                let (steps, fade) = (rng.byte(), rng.byte());
                let call = [
                    i,
                    which,
                    u32::from(start.kind),
                    u32::from(steps),
                    length,
                    speed,
                    range,
                    pull,
                    u32::from(fade),
                ];
                twin("MsScreenShake2", a, b, 0x007c_57b0, &call, 0, |m| {
                    sh::ms_screen_shake2(
                        m,
                        i,
                        which,
                        start.kind,
                        steps,
                        length,
                        speed,
                        range.cast_signed(),
                        pull,
                        fade,
                    );
                    0
                });
            }
        }
        same("the shake starts", a, b);
    }
}

#[test]
fn shakes_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x5eed_5a4e));
}

#[test]
fn shakes_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x0bad_cafe));
}
