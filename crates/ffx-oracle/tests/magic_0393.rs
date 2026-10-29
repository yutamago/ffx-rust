//! `magic_0393.dll` against its Rust port (`ffx_magic::m0393`): the
//! summon's facing, placement, camera and model updates, the task step
//! and the effect's callbacks. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

mod magic;
mod magic_l3;
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::{Ret, m0393};
use magic::{Call, Eax, SCRATCH, Twin};
use ownb::{dirty_stack, fill, floats, rec, rec_all, reset_counters};

const N: u16 = 393;

const STATE: u32 = SCRATCH + 0x8000;
const SEL: u32 = SCRATCH + 0x600;
const CHRS: u32 = SCRATCH + 0x800;
/// Cells the stubs answer from.
const CFG: u32 = SCRATCH + 0x700;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The DLL with its state block, selection and characters set, and the
/// slots answered.
fn twin() -> Option<Twin> {
    let mut t = magic_l3::twin(N)?;
    let state_g = t.img.word(t.at(0x1770) + 0x08);
    t.set_u32(state_g, STATE);
    fill(&mut t, STATE + 0x1b80, 0xe0, 0x393);
    for (i, v) in (0u32..).zip([1.5f32, -2.0, 30.0, 1.0]) {
        t.set_u32(STATE + 0x1c10 + 4 * i, v.to_bits());
    }
    let p = t.at(0x24d0);
    t.set_u32(t.img.word(p + 0x2a2), SEL);
    t.set_u8(SEL + 1, 2);
    for i in 0..4 {
        t.set_u32(t.img.word(p + 0x2ad) + 4 * i, CHRS + 0x10 * i);
    }
    // positions and turns
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            let v = f32::from((a[0] & 0x7f) as u8) + 0.5 * f32::from(i as u8);
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0
    });
    rec(&mut t, 333, 5, |_, _| 0);
    t.stub_st0(333, |m, _| f64::from(f32::from_bits(m.r32(CFG))));
    rec(&mut t, 277, 1, |_, _| 0);
    t.stub_st0(277, |_, a| 40.0 + f64::from(a[0] & 0xf));
    for (s, v) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0)] {
        rec(&mut t, s, 1, |_, _| 0);
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 0x3));
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v ^ (a[2] & 0x00ff_0000) ^ (s << 8));
            }
            s
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
            // (kept where the comparison sees it: the vector is a local)
            m.w32(SCRATCH + 0x3c0 + 4 * i, v);
        }
        0
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(7);
            m.w32(a[0] + 4 * i, v);
        }
        0x248
    });
    rec_all(&mut t, &[(210, 2), (211, 2), (217, 2), (255, 4)]);
    Some(t)
}

/// Rotations whose angle `A - rot` (operands `a`, `hi`, `lo` of `f`)
/// wraps down, stays, wraps up, lands on either limit, or is NaN.
fn rotations(t: &Twin, f: u32, a: u32, hi: u32, lo: u32) -> Vec<f32> {
    let k = |op: u32| f32::from_bits(t.u32(t.img.word(f + op)));
    let (a, hi, lo) = (k(a), k(hi), k(lo));
    let mut v = vec![0.0f32, a, a + 4.0, -2.0, f32::NAN];
    for limit in [hi, lo] {
        let guess = (a - limit).to_bits();
        if let Some(r) = (0..256u32)
            .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
            .map(f32::from_bits)
            .find(|&r| (a - r).to_bits() == limit.to_bits())
        {
            v.push(r);
        }
    }
    v
}

#[test]
fn face() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x1770);
    t.set_u32(t.img.word(f + 0x22), CHRS + 0x21);
    rec(&mut t, 627, 0, |m, _| m.r32(CFG + 4));
    rec(&mut t, 390, 1, |_, a| a[0] + 0x390);
    rec(&mut t, 719, 4, |_, a| a[2] ^ 0x719);
    rec(&mut t, 551, 7, |_, _| 0x551);
    let flags = [0x113, 0x11d, 0x127].map(|o| t.img.word(f + o));
    for (k, rot) in rotations(&t, f, 0x4f, 0x5e, 0x72).into_iter().enumerate() {
        let target = [0xffff_ffffu32, 0, 2, 0x8000_0000, 5][k % 5];
        t.set_u32(CFG, rot.to_bits());
        t.set_u32(CFG + 4, target);
        for g in flags {
            t.set_u32(g, 0x77);
        }
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0393::face(g, img, fr.at))
        });
    }
}

#[test]
fn step() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x16f0);
    rec(&mut t, 561, 0, |m, _| m.r32(CFG));
    rec_all(&mut t, &[(723, 1), (724, 2), (631, 4), (214, 5)]);
    rec(&mut t, 219, 2, |_, a| a[0] ^ 0x219);
    rec(&mut t, 257, 4, |_, _| 0x257);
    t.set_u32(t.img.word(f + 0x33), CHRS + 0x31);
    t.set_u32(t.img.word(f + 0x63), 0x0bfc);
    let ro = t.img.rel32_target(f + 0x19);
    let list = t.img.word(ro + 0x7c);
    let (node, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(node + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    for (pause, busy, done) in [
        (1u32, false, 1u32),
        (0, false, 1),
        (0, false, 0),
        (0, false, 2),
        (0, true, 1),
    ] {
        t.set_u32(CFG, pause);
        t.set_u32(list + 8, if busy { node } else { 0 });
        t.set_u32(STATE + 0x1f00, done);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn models() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x19f0);
    // Ch_Allocate: 0 for model 7
    rec(
        &mut t,
        0,
        1,
        |_, a| if a[0] == 7 { 0 } else { a[0] + 0x1000 },
    );
    rec(&mut t, 26, 3, |_, _| 0x26);
    rec(&mut t, 10, 5, |_, a| a[0] ^ 0x10);
    rec(&mut t, 215, 1, |_, a| a[0] + 0x215);
    rec_all(&mut t, &[(278, 4), (632, 3)]);
    let n = t.img.word(f + 0x4f);
    for (model, target) in [(0xffff_ffffu32, 6u32), (3, 6), (3, 5), (7, 6)] {
        t.set_u32(STATE + 0x1eec, model);
        t.set_u32(STATE + 0x1ee8, target);
        t.set_u32(n, 0x40);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn small_callbacks() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x2370);
    t.set_u32(t.img.word(f + 0x1e), CHRS + 0x41);
    rec(&mut t, 268, 3, |_, a| a[2] ^ 0x268);
    let obj = SCRATCH + 0x5000;
    fill(&mut t, obj + 0xc0, 0x50, 1);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    let f = t.at(0x2460);
    rec(&mut t, 633, 2, |m, a| m.r32(STATE + 0x1ba4) ^ a[0]);
    t.check_entry(f, Call::default(), &[0x11, 0x22], Eax::Checked);
    let f = t.at(0x24b0);
    for m in [0xffff_ffffu32, 0, 0xffff_fffe] {
        t.set_u32(STATE + 0x1eec, m);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn place() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x24d0);
    // the facing -rot: facing + K = K - rot hits each case
    for rot in rotations(&t, f, 0x30, 0x51, 0x7f) {
        t.set_u32(STATE + 0x1ee0, (-rot).to_bits());
        reset_counters(&mut t);
        dirty_stack(&mut t, rot.to_bits(), &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn update() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x2960);
    for (k, n) in [0u32, 1, 2, 46, 0xffff_fffd, 0x7fff_ffff]
        .into_iter()
        .enumerate()
    {
        t.set_u32(STATE + 0x1ef4, n);
        let a = [0.0f32, 2.5, -3.0, 3.1, -4.2, f32::NAN][k];
        t.set_u32(STATE + 0x1bf0, a.to_bits());
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    // the turn landing exactly on either wrap limit
    let d = f32::from_bits(t.u32(t.img.word(f + 0xc0)));
    for op in [0xc7u32, 0xdb] {
        let limit = f32::from_bits(t.u32(t.img.word(f + op)));
        let guess = (limit - d).to_bits();
        if let Some(r) = (0..256u32)
            .flat_map(|e| [guess.wrapping_add(e), guess.wrapping_sub(e)])
            .map(f32::from_bits)
            .find(|&r| (r + d).to_bits() == limit.to_bits())
        {
            t.set_u32(STATE + 0x1bf0, r.to_bits());
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

#[test]
fn cue() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x2c90);
    rec(&mut t, 939, 0, |m, _| m.r32(CFG));
    rec(&mut t, 719, 3, |_, a| a[2] ^ 0x719);
    rec(&mut t, 220, 6, |_, a| a[0] + 0x220);
    for r in [0u32, 1, 2, 3, 4, 0xffff_ffff] {
        t.set_u32(CFG, r);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn frame() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x2d00);
    rec_all(&mut t, &[(723, 1), (724, 2)]);
    rec(&mut t, 327, 4, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x327);
        }
        0
    });
    rec(&mut t, 282, 0, |m, _| m.r32(CFG + 4));
    t.stub_st0(282, |m, _| f64::from(f32::from_bits(m.r32(CFG))));
    let obj = SCRATCH + 0x5000;
    for (n, k, d) in [
        (0xffff_fffbu32, 0u32, 2.0f32),
        (0, 1, 0.5),
        (8, 5, 3.0),
        (9, 5, 0.0),
        (27, 5, -1.0),
        (28, 0, 7.0),
        (28, 1, 7.0),
        (100, 6, 2.0),
        (0x7fff_ffff, 0x8000_0001, 1.0),
        (0x8000_0000, 2, 1.0),
    ] {
        fill(&mut t, obj + 0x60, 0x50, n);
        t.set_u32(obj + 0x118, n);
        t.set_u32(obj + 0x120, k);
        t.set_u32(CFG, d.to_bits());
        t.set_u32(CFG + 4, 0x282 ^ n);
        dirty_stack(&mut t, n, &[]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn eye() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x3d90);
    let pi = f32::from_bits(t.u32(t.img.word(f + 0x5a)));
    for a in [0.0f32, 0.5, -0.5, -4.0, 1.0, f32::NAN, -pi] {
        t.set_u32(STATE + 0x1b90, a.to_bits());
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    floats(&mut t, STATE + 0x1ba0, &[1.0, 2.0, 3.0, 4.0]);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}
