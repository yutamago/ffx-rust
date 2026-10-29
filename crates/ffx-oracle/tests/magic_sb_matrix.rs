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
    let mut t = Twin::load(3)?;
    t.init();
    let g = t.img.word(t.at(rva) + table_op);
    t.set_u32(g, SINES);
    let mut b = Vec::with_capacity(4 * 4096);
    for i in 0..4096u32 {
        let x = match i {
            7 => -0.0f32,
            9 => 1.0e30,
            11 => -3.0,
            13 => f32::MIN_POSITIVE,
            _ => (f64::from(i) * std::f64::consts::TAU / 4096.0).sin() as f32,
        };
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

fn check(
    t: &mut Twin,
    rva: u32,
    port: fn(&mut magic::PortGame, &ffx_magic::Image, u32, u32, u32) -> u32,
) {
    for a in angle_sets() {
        for (i, x) in (0u32..).zip(a) {
            t.set_u32(ANGLES + 4 * i, x);
        }
        t.write(OUT, &[0xcd; 0x40]);
        t.check(
            t.at(rva),
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
    check(&mut t, 0x52a0, sb_matrix::rotation_52a0);
}

#[test]
fn rotation_19d0() {
    let Some(mut t) = twin(0x19d0, 0x17) else {
        return;
    };
    check(&mut t, 0x19d0, sb_matrix::rotation_19d0);
}
