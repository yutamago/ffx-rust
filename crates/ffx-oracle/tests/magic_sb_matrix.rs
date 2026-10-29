//! The node family's rotation matrices (`ffx_magic::sb_matrix`) against
//! `magic_0003.dll`. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;

use ffx_magic::{Ret, sb_matrix};
use magic::{Call, Eax, SCRATCH, Twin};

const SINES: u32 = SCRATCH + 0x1_0000;
const ANGLES: u32 = SCRATCH + 0x100;
const OUT: u32 = SCRATCH + 0x200;

/// The DLL with a sine table of 4096 floats behind its pointer global
/// (operand `table_op` of the function at `rva`): a real sine, with a few
/// entries replaced by -0.0 and odd values. No NaNs or infinities: the
/// matrices negate sines (`xorps`), so two different NaNs would meet in
/// one operation, and Unicorn (QEMU) picks between them by the x87 rule
/// (the larger significand), not the CPU's SSE rule (the destination's;
/// `ffx_magic::sb_sse`), which the port follows.
fn twin(rva: u32, table_op: u32) -> Option<Twin> {
    twin_with(rva, table_op, |i| match i {
        7 => Some(-0.0),
        9 => Some(1.0e30),
        11 => Some(-3.0),
        13 => Some(f32::MIN_POSITIVE),
        _ => None,
    })
}

/// [`twin`] with infinite sines and cosines (angles `7 << 4`, `9 << 4`,
/// `13 << 4` and their quarter turns back): the products with 0 make NaN
/// of them, and the NaN every such product and `inf - inf` give is the
/// one "indefinite" NaN, so no two different NaNs meet.
fn twin_inf(rva: u32, table_op: u32) -> Option<Twin> {
    twin_with(rva, table_op, |i| match i {
        7 | 0x40d => Some(f32::INFINITY),
        9 | 0x407 => Some(f32::NEG_INFINITY),
        11 | 0x40b => Some(1.0e30),
        13 => Some(-0.0),
        _ => None,
    })
}

fn twin_with(rva: u32, table_op: u32, special: fn(u32) -> Option<f32>) -> Option<Twin> {
    let mut t = Twin::load(3)?;
    t.init();
    let g = t.img.word(t.at(rva) + table_op);
    t.set_u32(g, SINES);
    let mut b = Vec::with_capacity(4 * 4096);
    for i in 0..4096u32 {
        let x = special(i)
            .unwrap_or_else(|| (f64::from(i) * std::f64::consts::TAU / 4096.0).sin() as f32);
        b.extend_from_slice(&x.to_bits().to_le_bytes());
    }
    t.write(SINES, &b);
    Some(t)
}

fn angle_sets() -> Vec<[u32; 3]> {
    let mut v = vec![
        [0, 0, 0],
        [0x4000, 0x8000, 0xc000],
        [0xffff_fff0, 0x10, 0x8000_0000],
        [7 << 4, 9 << 4, 11 << 4],
        [13 << 4, 0, 7 << 4],
        [
            (7u32 << 4).wrapping_sub(0x4000),
            (9u32 << 4).wrapping_sub(0x4000),
            0x1234,
        ],
    ];
    // every combination of the quarter turns and the odd entries (sines
    // and cosines of exactly 0, -0.0, 1, -1, 1e30 and the smallest
    // normal): where most terms vanish, the zero products the original
    // keeps decide the signs of the zeros left
    let special = [
        0u32,
        0x4000,
        0x8000,
        0xc000,
        7 << 4,
        (7u32 << 4).wrapping_sub(0x4000),
        9 << 4,
        13 << 4,
    ];
    for &x in &special {
        for &y in &special {
            for &z in &special {
                v.push([x, y, z]);
            }
        }
    }
    // a fixed pseudo-random spread
    let mut s = 0x1234_5678u32;
    for _ in 0..40 {
        let mut a = [0u32; 3];
        for x in &mut a {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *x = s >> 8;
        }
        v.push(a);
    }
    v
}

type Port = fn(&mut magic::PortGame, &ffx_magic::Image, u32, u32, u32) -> u32;

fn check(t: &mut Twin, rva: u32, thunk: u32, port: Port, sets: &[[u32; 3]]) {
    for &a in sets {
        for (i, x) in (0u32..).zip(a) {
            t.set_u32(ANGLES + 4 * i, x);
        }
        t.write(OUT, &[0xcd; 0x40]);
        t.check_thunked(
            t.at(rva),
            t.at(thunk),
            true,
            Call {
                eax: 1,
                ecx: OUT,
                edx: ANGLES,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(port(g, img, fr.at, fr.ecx, fr.edx)),
        );
    }
}

#[test]
fn rotation_52a0() {
    let Some(mut t) = twin(0x52a0, 0x1c) else {
        return;
    };
    check(
        &mut t,
        0x52a0,
        0x5b90,
        sb_matrix::rotation_52a0,
        &angle_sets(),
    );
}

#[test]
fn rotation_19d0() {
    let Some(mut t) = twin(0x19d0, 0x17) else {
        return;
    };
    check(
        &mut t,
        0x19d0,
        0x5b50,
        sb_matrix::rotation_19d0,
        &angle_sets(),
    );
}

/// The shared-3 list's three (`ffx_magic::s3_matrix`), through their
/// thunks too.
#[test]
fn rotation_47c0() {
    let Some(mut t) = twin(0x47c0, 0x1f) else {
        return;
    };
    check(
        &mut t,
        0x47c0,
        0x5b80,
        ffx_magic::s3_matrix::rotation_47c0,
        &angle_sets(),
    );
}

#[test]
fn rotation_4d40() {
    let Some(mut t) = twin(0x4d40, 0x0c) else {
        return;
    };
    check(
        &mut t,
        0x4d40,
        0x5b60,
        ffx_magic::s3_matrix::rotation_4d40,
        &angle_sets(),
    );
}

#[test]
fn rotation_4250() {
    let Some(mut t) = twin(0x4250, 0x0c) else {
        return;
    };
    check(
        &mut t,
        0x4250,
        0x5b70,
        ffx_magic::s3_matrix::rotation_4250,
        &angle_sets(),
    );
}

/// The angle words of [`twin_inf`]'s special entries in every place, and
/// the usual spread.
fn inf_angle_sets() -> Vec<[u32; 3]> {
    let special = [
        7u32 << 4,
        9 << 4,
        13 << 4,
        (7u32 << 4).wrapping_sub(0x4000),
        (11u32 << 4).wrapping_sub(0x4000),
        0x1230,
    ];
    let mut sets = angle_sets();
    for a in special {
        for b in special {
            for c in special {
                sets.push([a, b, c]);
            }
        }
    }
    sets
}

fn check_inf(rva: u32, table_op: u32, thunk: u32, port: Port) {
    let Some(mut t) = twin_inf(rva, table_op) else {
        return;
    };
    check(&mut t, rva, thunk, port, &inf_angle_sets());
}

#[test]
fn rotation_52a0_on_infinities() {
    check_inf(0x52a0, 0x1c, 0x5b90, sb_matrix::rotation_52a0);
}

#[test]
fn rotation_19d0_on_infinities() {
    check_inf(0x19d0, 0x17, 0x5b50, sb_matrix::rotation_19d0);
}

#[test]
fn rotation_47c0_on_infinities() {
    check_inf(0x47c0, 0x1f, 0x5b80, ffx_magic::s3_matrix::rotation_47c0);
}

#[test]
fn rotation_4d40_on_infinities() {
    check_inf(0x4d40, 0x0c, 0x5b60, ffx_magic::s3_matrix::rotation_4d40);
}

#[test]
fn rotation_4250_on_infinities() {
    check_inf(0x4250, 0x0c, 0x5b70, ffx_magic::s3_matrix::rotation_4250);
}

// Mutation testing (M5 magic round 3, tools/mutants.py, a mutant per
// code line plus a deletion per store or call):
// - ffx_magic::sb_matrix: 358 mutants, 251 killed. The 107 left (52 in
//   rotation_52a0, 55 in rotation_19d0) are judged equivalent on a
//   native check: every one of them, built as a copy of the port with
//   the change, gave bit-identical matrices and `eax` to the port on
//   5,002,240 angle triples (the special combinations above, every
//   quarter-turn neighbourhood, 5,000,000 random) with a real sine table
//   and with this file's odd one; the same check flags killed mutants at
//   once. The ones read by hand change terms that only feed the
//   original's zero products (an operand that is the constant 0.0), a
//   zero whose sign no output keeps while the sines are finite.
