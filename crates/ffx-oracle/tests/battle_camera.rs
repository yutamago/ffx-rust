//! The battle camera's request setters and resets (`ffx_battle::camera`,
//! round 25) against the original, as in `battle.rs`: results and every
//! written byte; the CRT's trigonometry as the shared stand-ins
//! (`battle_common::crt`).

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::*;
use ffx_battle::camera::bridge as cam;
use ffx_oracle::Oracle;

/// A float's bits: mostly ordinary, sometimes zero or a quiet NaN.
fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(10) {
        0 => [0, 0x8000_0000, 0x7f80_0000, 0x7fc0_0000][rng.below(4) as usize],
        1 => quiet(rng.next()),
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 97.0).to_bits(),
    }
}

/// `n` words of [`float`] bits.
fn floats(rng: &mut Xorshift, n: u32) -> Vec<u8> {
    (0..n).flat_map(|_| float(rng).to_le_bytes()).collect()
}

/// A request ID for camera `c`: often one its slots hold, else made up.
fn request(a: &mut Oracle, rng: &mut Xorshift, slots: u32, c: u32) -> u32 {
    if rng.below(3) == 0 {
        c | (rng.below(6) << 8) | (rng.below(3) << 16)
    } else {
        let at = slots + 8 * rng.below(4);
        let w = a.read(at, 4).unwrap();
        u32::from_le_bytes(w.try_into().unwrap())
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use battle_common::crt::stand_in;
    battle_common::crt::patch(a);
    battle_common::crt::patch(b);
    let buf = scratch(a, b, 0x200);
    let list = scratch(a, b, 0x100);
    let cams = a.rebase(0x0113_7540);
    for _ in 0..3000 {
        // camera c: quiet floats throughout, then request slots whose IDs
        // name it, the channels' filter counts small
        let c = rng.below(3);
        let base = cams + c * 0xda0;
        let words = floats(rng, 0xda0 / 4);
        poke(a, b, base, &words);
        let slots = base + 0xc68;
        for k in 0..4u32 {
            let id = c | (rng.below(4) << 8) | (rng.below(3) << 16);
            let mut s = id.to_le_bytes().to_vec();
            s.extend([
                u8::from(rng.below(5) == 0),
                u8::from(rng.below(5) == 0),
                u8::try_from(rng.below(3)).unwrap(),
                0,
            ]);
            poke(a, b, slots + 8 * k, &s);
        }
        for side in 0..2 {
            for ch in 0..4 {
                let n = u8::try_from(rng.below(7)).unwrap();
                poke(a, b, base + side * 0x61c + 0xf0 + ch * 0x130 + 4, &[n]);
            }
        }
        let id = request(a, rng, slots, c);
        let side = if rng.below(4) == 0 {
            rng.word()
        } else {
            rng.below(2)
        };
        let v = [float(rng), float(rng), float(rng), float(rng)];
        match rng.below(8) {
            0 => twin(
                "MsCameraResetFilter",
                a,
                b,
                0x007b_ed10,
                &[id, side],
                0,
                |m| {
                    cam::ms_camera_reset_filter(m, id, side);
                    0
                },
            ),
            1 => {
                let x = rng.word();
                twin(
                    "MsCameraMoveStat",
                    a,
                    b,
                    0x007b_df90,
                    &[id, side, x],
                    0,
                    |m| {
                        cam::ms_camera_move_stat(m, id, side, x.to_le_bytes()[0]);
                        0
                    },
                );
            }
            2 => twin(
                "MsCameraMoveDivide",
                a,
                b,
                0x007b_dde0,
                &[id, side, v[0], v[1], v[2], v[3]],
                0,
                |m| {
                    cam::ms_camera_move_divide(m, id, side, v);
                    0
                },
            ),
            3 => {
                let kind = if rng.below(2) == 0 { 3 } else { rng.word() };
                let value = rng.word();
                twin(
                    "MsCameraSetSplineFilter",
                    a,
                    b,
                    0x007b_fe70,
                    &[id, side, value, v[0], kind],
                    0,
                    |m| {
                        cam::ms_camera_set_spline_filter(m, id, side, value, v[0], kind);
                        0
                    },
                );
            }
            4 => twin(
                "MsCameraResetDirVector",
                a,
                b,
                0x007b_ec90,
                &[id, side],
                0,
                |m| {
                    cam::ms_camera_reset_dir_vector(m, id, side);
                    0
                },
            ),
            5 => {
                let x = rng.word();
                twin(
                    "MsCameraSetFocus",
                    a,
                    b,
                    0x007b_f280,
                    &[id, side, x],
                    0,
                    |m| {
                        cam::ms_camera_set_focus(m, id, side, x);
                        0
                    },
                );
            }
            6 => {
                // the reset takes the side as it comes; keep it to 0, 1, 2
                let side = rng.below(3);
                twin(
                    "MsCameraReset",
                    a,
                    b,
                    0x007b_eae0,
                    &[id, side],
                    u32::MAX,
                    |m| cam::ms_camera_reset(m, id, side),
                );
            }
            _ => match rng.below(3) {
                0 => {
                    // the yaw of the camera in use, through a local
                    set(a, b, cam::CAMERA, base + rng.below(2) * 0x61c);
                    ftwin("MsCameraGetYaw", a, b, 0x007b_cf20, &[], |m| {
                        cam::ms_camera_get_yaw(m, &stand_in, buf).to_bits()
                    });
                    poke(a, b, buf, &[0; 4]);
                }
                1 => start_pos(a, b, rng, buf, base),
                2 if rng.below(2) == 0 => {
                    for _ in 0..4 {
                        aims(a, b, rng, buf);
                    }
                }
                _ => process_helpers(a, b, rng, buf, list),
            },
        }
        same("the cameras", a, b);
    }
}

/// `MsCameraSetStartPos` on a made-up side at `buf + 0x10` (its base
/// vector at `buf + 0x100`), the move at `buf + 0x180`.
fn start_pos(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32, base: u32) {
    use battle_common::crt::stand_in;
    let side = buf + 0x10;
    let words = floats(rng, 0x24);
    poke(a, b, side, &words);
    let vector = floats(rng, 4);
    poke(a, b, buf + 0x100, &vector);
    let origin = if rng.below(3) == 0 { 0 } else { buf + 0x100 };
    poke(a, b, side + 0x8c, &origin.to_le_bytes());
    // r0 is 0 in the game; the original's extra stack words only matter
    // to a null base when it is not
    set(a, b, cam::EE_R0, [0; 4]);
    let mv = buf + 0x180;
    let kind = if rng.below(2) == 0 { 1 } else { rng.byte() };
    poke(a, b, mv, &[kind, 0, 0, 0, 0, 0, 0, rng.byte()]);
    let camera = if rng.below(3) == 0 { 0 } else { base };
    twin(
        "MsCameraSetStartPos",
        a,
        b,
        0x007b_ff10,
        &[camera, side, mv],
        u32::MAX,
        |m| cam::ms_camera_set_start_pos(m, &stand_in, camera, side, mv),
    );
    // VU register 4's yzw: the original's saved esi, ebp and return address
    let lanes = a.rebase(0x00c8_f814);
    let w = a.read(lanes, 12).unwrap();
    b.write(lanes, &w).unwrap();
}

/// The three aims of `MsCameraProcess` (round 26) on a made-up side at
/// `buf + 0x10` (its base vector at `buf + 0x100`, or none) and move at
/// `buf + 0x180`; the port's two scratch words (the original's stack) at
/// `buf + 0x1f0`, cleared after.
fn aims(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32) {
    use battle_common::crt::stand_in;
    let side = buf + 0x10;
    poke(a, b, side, &floats(rng, 0x24));
    poke(a, b, buf + 0x100, &floats(rng, 4));
    let origin = if rng.below(3) == 0 { 0 } else { buf + 0x100 };
    poke(a, b, side + 0x8c, &origin.to_le_bytes());
    let mv = buf + 0x180;
    poke(a, b, mv, &floats(rng, 0x18));
    for o in [0x54, 0x58] {
        if rng.below(3) == 0 {
            let z = [0u32, 0x8000_0000][rng.below(2) as usize];
            poke(a, b, mv + o, &z.to_le_bytes());
        }
    }
    let scratch = buf + 0x1f0;
    let k = rng.below(3);
    let (name, sym) = [
        ("MsCameraAimPolar_007c0910", 0x007c_0910),
        ("MsCameraAimPolar2_007c09b0", 0x007c_09b0),
        ("MsCameraAimPolar3_007c0a30", 0x007c_0a30),
    ][k as usize];
    twin(name, a, b, sym, &[side, mv], 0, |m| {
        match k {
            0 => cam::ms_camera_aim_polar(m, &stand_in, side, mv, scratch),
            1 => cam::ms_camera_aim_polar2(m, &stand_in, side, mv, scratch),
            _ => cam::ms_camera_aim_polar3(m, &stand_in, side, mv, scratch),
        }
        0
    });
    poke(a, b, scratch, &[0; 8]);
}

/// `MsCameraFocusWeights_007c0ae0` on a made-up side (`buf + 0x10`), move
/// (`buf + 0x180`) and list of up to four records (`list`), and
/// `MsScaleAboutPoint_007c4d40` on three vectors from `buf`.
fn process_helpers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32, list: u32) {
    let side = buf + 0x10;
    let words = floats(rng, 0x24);
    poke(a, b, side, &words);
    let mv = buf + 0x180;
    let words = floats(rng, 0x14);
    poke(a, b, mv, &words);
    let count = rng.range(-1, 5);
    poke(a, b, mv + 3, &[rng.byte() & 0xf]);
    poke(a, b, mv + 8, &list.to_le_bytes());
    let words = floats(rng, 0x40);
    poke(a, b, list, &words);
    poke(a, b, list, &count.to_le_bytes());
    // sometimes a record on the side's position
    if rng.below(4) == 0 {
        let p = a.read(side + 0x30, 12).unwrap();
        poke(a, b, list + 0x10, &p);
    }
    twin(
        "MsCameraFocusWeights_007c0ae0",
        a,
        b,
        0x007c_0ae0,
        &[side, mv],
        0,
        |m| {
            cam::ms_camera_focus_weights(m, side, mv);
            0
        },
    );
    let words = floats(rng, 12);
    poke(a, b, buf + 0x100, &words);
    let r0 = cam::EE_R0;
    let origin = buf + 0x110;
    set(
        a,
        b,
        r0,
        [if rng.below(3) == 0 { origin } else { 0 }, 0, 0, 0],
    );
    let regs = a.rebase(0x00c8_f7b0);
    let words = floats(rng, 4);
    poke(a, b, regs, &words);
    twin(
        "MsScaleAboutPoint_007c4d40",
        a,
        b,
        0x007c_4d40,
        &[buf + 0x100, origin, buf + 0x120],
        u32::MAX,
        |m| ffx_battle::vector::bridge::ms_scale_about_point(m, buf + 0x100, origin, buf + 0x120),
    );
    set(a, b, r0, [0; 4]);
}

#[test]
fn camera_setters_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xca3e_0025));
}

#[test]
fn camera_setters_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x25ca_3e00));
}
