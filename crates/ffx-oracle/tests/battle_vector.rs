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
