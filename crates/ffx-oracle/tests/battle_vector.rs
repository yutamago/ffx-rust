//! The battle vector and matrix helpers (`ffx_battle::vector`) against the
//! original, as in `battle.rs`: float results compare bit for bit, NaNs
//! included (quiet ones: `docs/porting.md`), and so do the emulated VU0
//! registers they leave behind.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::*;
use ffx_battle::vector::bridge as v;
use ffx_oracle::{Conv, Oracle};

/// A float's bits: often ordinary, sometimes zero, tiny, huge, infinite
/// or NaN.
fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(10) {
        0 => [
            0,
            0x8000_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_0000,
            0xffc0_0000,
            1,
            0x8000_0001,
        ][rng.below(8) as usize],
        1 => quiet(rng.next()),
        2 => 0x7fc0_0000 | rng.below(0x40_0000) | (rng.below(2) << 31),
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 97.0).to_bits(),
    }
}

/// An `f32` in an 80-bit x87 register (as `fld m32` left it), as bits.
fn f80_to_f32(r: [u8; 10]) -> u32 {
    let mant = u64::from_le_bytes(r[..8].try_into().unwrap());
    let se = u16::from_le_bytes([r[8], r[9]]);
    let sign = u32::from(se >> 15) << 31;
    let exp = i32::from(se & 0x7fff);
    if exp == 0x7fff {
        let frac = u32::try_from((mant >> 40) & 0x7f_ffff).unwrap();
        return sign | 0x7f80_0000 | frac;
    }
    if mant == 0 {
        return sign;
    }
    // exact: it came from an f32 (24 significant bits)
    #[allow(clippy::cast_precision_loss)]
    let x = (mant as f64) * 2f64.powi(exp - 16383 - 63);
    #[allow(clippy::cast_possible_truncation)]
    let f = x as f32;
    sign | f.to_bits()
}

/// `_CIsqrt`'s thunk (`0x009497b8`, an import the oracle lacks) as the
/// CPU's square root: `fsqrt; ret`.
fn patch_sqrt(a: &mut Oracle, b: &mut Oracle) {
    for o in [a, b] {
        let at = o.rebase(0x0094_97b8);
        o.write(at, &[0xd9, 0xfa, 0xc3]).unwrap();
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    patch_sqrt(a, b);
    battle_common::crt::patch(a);
    battle_common::crt::patch(b);
    let buf = scratch(a, b, 0x100);
    for _ in 0..3000 {
        let vals: Vec<u8> = (0..0x40).flat_map(|_| float(rng).to_le_bytes()).collect();
        poke(a, b, buf, &vals);
        let args = [float(rng), float(rng), float(rng), float(rng)];
        let call = [buf, args[0], args[1], args[2], args[3]];
        twin("MsMulVector", a, b, 0x007c_2e10, &call, 0, |m| {
            v::ms_mul_vector(m, buf, args);
            0
        });
        let call = [buf + 0x10, args[0], args[1], args[2], args[3]];
        twin("MsDivVector", a, b, 0x007c_2520, &call, 0, |m| {
            v::ms_div_vector(m, buf + 0x10, args);
            0
        });
        twin(
            "MsAddPoint",
            a,
            b,
            0x007c_1480,
            &[buf + 0x20, buf + 0x30, buf + 0x40],
            0,
            |m| {
                v::ms_add_point(m, buf + 0x20, buf + 0x30, buf + 0x40);
                0
            },
        );
        let p = if rng.below(4) == 0 { 0 } else { buf + 0x50 };
        twin("MsSetNormalUpVector", a, b, 0x007c_0ab0, &[p], 0, |m| {
            v::ms_set_normal_up_vector(m, p);
            0
        });
        twin(
            "MsApplyMatrix",
            a,
            b,
            0x007c_1550,
            &[buf + 0x60, buf + 0x80, buf + 0xc0],
            0,
            |m| {
                v::ms_apply_matrix(m, buf + 0x60, buf + 0x80, buf + 0xc0);
                0
            },
        );
        // angles: mostly within a few turns
        let ang = |rng: &mut Xorshift| {
            if rng.below(4) == 0 {
                float(rng)
            } else {
                (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 1000.0).to_bits()
            }
        };
        let (x, y) = (ang(rng), ang(rng));
        let ra = a.call(a.rebase(0x007c_04c0), Conv::Cdecl, &[x, y]).unwrap();
        let rb = v::ms_near_rag(b, x, y).to_bits();
        assert_eq!(
            f80_to_f32(ra.st0_raw.expect("a float")),
            rb,
            "MsNearRag {x:#x} {y:#x}"
        );
        twin(
            "MsMulMatrix",
            a,
            b,
            0x007c_2770,
            &[buf + 0x60, buf + 0x80, buf + 0xc0],
            0,
            |m| {
                v::ms_mul_matrix(m, buf + 0x60, buf + 0x80, buf + 0xc0);
                0
            },
        );
        let st = [float(rng), float(rng), float(rng), float(rng), float(rng)];
        let call = [
            buf + 0x20,
            buf + 0x30,
            buf + 0x40,
            st[0],
            st[1],
            st[2],
            st[3],
            st[4],
        ];
        twin("MsMiddlePoint", a, b, 0x007c_25e0, &call, 0, |m| {
            v::ms_middle_point(m, buf + 0x20, buf + 0x30, buf + 0x40, st);
            0
        });
        if rng.below(4) == 0 {
            poke(a, b, buf + 0x30, &[0; 16]);
        }
        for (name, sym, rust) in [
            (
                "MsVectorLen",
                0x007c_4280,
                v::ms_vector_len as fn(&mut Oracle, u32) -> f32,
            ),
            ("MsVectorLen2D", 0x007c_43a0, v::ms_vector_len_2d),
        ] {
            let ra = a.call(a.rebase(sym), Conv::Cdecl, &[buf + 0x30]).unwrap();
            let rb = rust(b, buf + 0x30).to_bits();
            assert_eq!(f80_to_f32(ra.st0_raw.expect("a float")), rb, "{name}");
        }
        twin(
            "MsVectorNormal",
            a,
            b,
            0x007c_44a0,
            &[buf + 0x40, buf + 0x30],
            0,
            |m| {
                v::ms_vector_normal(m, buf + 0x40, buf + 0x30);
                0
            },
        );
        same("vectors", a, b);
        polar(a, b, rng, buf);
    }
}

/// The polar conversions and `MsVectorNormal2`, with the CRT's trigonometry
/// as the shared stand-ins (`battle_common::crt`).
fn polar(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32) {
    use battle_common::crt::stand_in;
    let vals: Vec<u8> = (0..0x40).flat_map(|_| float(rng).to_le_bytes()).collect();
    poke(a, b, buf, &vals);
    // sometimes a zero difference or zero coordinates
    if rng.below(4) == 0 {
        let p = a.read(buf, 16).unwrap();
        poke(a, b, buf + 0x10, &p);
    }
    if rng.below(4) == 0 {
        poke(a, b, buf + 4 * rng.below(3), &[0; 4]);
    }
    let pick = |rng: &mut Xorshift, k: u32| if rng.below(5) == 0 { 0 } else { buf + 0x40 + k };
    let (yaw, pitch, len) = (pick(rng, 0x20), pick(rng, 0x24), pick(rng, 0x28));
    let base = if rng.below(3) == 0 { 0 } else { buf + 0x10 };
    let args = [buf, base, yaw, pitch, len];
    twin("MsVectorRect2Polar", a, b, 0x007c_4990, &args, 0, |m| {
        v::ms_vector_rect2_polar(m, &stand_in, buf, base, yaw, pitch, len);
        0
    });
    same("MsVectorRect2Polar", a, b);
    let r = [float(rng), float(rng), float(rng), float(rng)];
    let (y, p) = (float(rng), float(rng));
    // the original compares base with r0's first word, not with 0
    let r0 = ffx_battle::camera::bridge::EE_R0;
    let saved = r0.get(b);
    if rng.below(4) == 0 {
        set(a, b, r0, [base, saved[1], saved[2], saved[3]]);
    }
    let args = [buf + 0x40, base, y, p, r[0], r[1], r[2], r[3]];
    twin("MsVectorPolar2Rect", a, b, 0x007c_47c0, &args, 0, |m| {
        v::ms_vector_polar2_rect(m, &stand_in, buf + 0x40, base, y, p, r);
        0
    });
    same("MsVectorPolar2Rect", a, b);
    set(a, b, r0, saved);
    let args = [buf + 0x50, buf, buf + 0x10, r[0], r[1], r[2], r[3]];
    twin("MsVectorNormal2", a, b, 0x007c_45e0, &args, 0, |m| {
        v::ms_vector_normal2(m, buf + 0x50, buf, buf + 0x10, r);
        0
    });
    same("MsVectorNormal2", a, b);
    // the rotations, sometimes in place; the rotation rows' other lanes
    // are the original's uninitialized locals (copied)
    for (name, sym, axis, free) in [
        (
            "MsRotMatrixX",
            0x007c_3170,
            v::Axis::X,
            [0x810u32, 0x81c, 0x820, 0x82c],
        ),
        (
            "MsRotMatrixY",
            0x007c_3480,
            v::Axis::Y,
            [0x814, 0x81c, 0x824, 0x82c],
        ),
        (
            "MsRotMatrixZ",
            0x007c_3790,
            v::Axis::Z,
            [0x818, 0x81c, 0x828, 0x82c],
        ),
    ] {
        let vals: Vec<u8> = (0..0x10).flat_map(|_| float(rng).to_le_bytes()).collect();
        poke(a, b, buf + 0x80, &vals);
        let out = if rng.below(2) == 0 {
            buf + 0x80
        } else {
            buf + 0xc0
        };
        let angle = float(rng);
        let args = [out, buf + 0x80, angle];
        twin(name, a, b, sym, &args, ALL, |m| {
            v::ms_rot_matrix(m, &stand_in, axis, out, buf + 0x80, angle)
        });
        for o in free {
            let at = 0x00c8_f000 + o;
            let at = a.rebase(at);
            let w = a.read(at, 4).unwrap();
            b.write(at, &w).unwrap();
        }
        same(name, a, b);
    }
}

#[test]
fn vectors_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x5a5a_a5a5));
    // the other float-to-integer conversion
    set(&mut a, &mut b, ffx_bridge::x87::SSE2_AVAILABLE, 1);
    check(&mut a, &mut b, &mut Xorshift(0xa5a5_5a5a));
}

#[test]
fn vectors_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x0f0f_f0f0));
}
