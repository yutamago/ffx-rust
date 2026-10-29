//! `magic_0394.dll` against its Rust port (`ffx_magic::m0394`): the
//! summon's placement, root and scaled matrices, its flight, the task
//! step and the afterimage callbacks. Needs `FFX_GAME_DIR`.

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

use ffx_magic::{Ret, m0394};
use magic::{Call, Eax, SCRATCH, Twin};
use ownb::{dirty_stack, fill, floats, rec, rec_all};

const N: u16 = 394;

const STATE: u32 = SCRATCH + 0x8000;
const SEL: u32 = SCRATCH + 0x600;
const CHRS: u32 = SCRATCH + 0x800;
/// Cells the stubs answer from.
const CFG: u32 = SCRATCH + 0x700;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

fn twin() -> Option<Twin> {
    let mut t = magic_l3::twin(N)?;
    let state_g = t.img.word(t.at(0x48e0) + 0x13);
    t.set_u32(state_g, STATE);
    fill(&mut t, STATE + 0xb00, 0x1a0, 0x394);
    let p = t.at(0x14e0);
    t.set_u32(t.img.word(p + 0x11), SEL);
    t.set_u8(SEL, 2);
    for i in 0..4 {
        t.set_u32(t.img.word(p + 0x1e) + 4 * i, CHRS + 0x10 * i);
    }
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            let v = f32::from((a[0] & 0x7f) as u8) + 0.5 * f32::from(i as u8);
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0
    });
    rec(&mut t, 333, 5, |_, _| 0);
    t.stub_st0(333, |m, _| f64::from(f32::from_bits(m.r32(CFG))));
    rec(&mut t, 277, 2, |_, _| 0);
    t.stub_st0(277, |_, a| 40.0 + f64::from(a[0] & 0xf));
    for (s, n, v) in [(15u32, 3usize, 1.5f64), (16, 4, -0.75), (17, 5, 3.0)] {
        rec(&mut t, s, n.min(2), move |_, _| s);
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 0x3));
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v ^ (a[2] & 0x00ff_0000) ^ (s << 8));
            }
            s
        });
    }
    rec(&mut t, 229, 6, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
            // (kept where the comparison sees it: the vector is a local)
            m.w32(SCRATCH + 0x3c0 + 4 * i, v);
        }
        0
    });
    rec(&mut t, 248, 9, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(7);
            m.w32(a[0] + 4 * i, v);
        }
        0x248
    });
    rec_all(&mut t, &[(210, 2), (211, 2), (217, 2)]);
    rec(&mut t, 255, 2, |m, a| {
        // the position it is given (four words)
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(SCRATCH + 0x380 + 4 * i, v);
        }
        0x255
    });
    Some(t)
}

#[test]
fn root_and_scaled() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x14e0);
    let off = t.img.word(f + 0x3e);
    let m = SCRATCH + 0x4000;
    for (mm, o, i) in [
        (m, 0u32, 2u8),
        (0, 0, 2),
        (0, 1, 2),
        (0, 0, 0x20),
        (0, 0, 0x1f),
    ] {
        t.set_u32(off, o);
        t.set_u8(SEL, i);
        let c = Call {
            edx: mm,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0394::root(g, img, fr.at, fr.edx, fr.esp))
        });
    }
    t.set_u8(SEL, 1);
    let f = t.at(0x1580);
    fill(&mut t, m, 16, 3);
    let c = Call {
        edx: m,
        ..Call::default()
    };
    t.check(f, c, &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0394::scaled(g, img, fr.at, fr.edx))
    });
}

#[test]
fn place() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x48e0);
    t.set_u32(t.img.word(f + 0x2f), CHRS + 0x51);
    rec(&mut t, 251, 1, |_, _| 0);
    t.stub_st0(251, |_, a| 2.0 + f64::from(a[0]));
    rec(&mut t, 19, 1, |_, _| 0);
    t.stub_st0(19, |_, a| 0.25 + f64::from(a[0] & 0xff));
    let off = t.img.word(f + 0xde);
    let flags = [0x205, 0x20f].map(|o| t.img.word(f + o));
    let a = f32::from_bits(t.u32(t.img.word(f + 0x5c)));
    for (k, rot) in [0.0f32, a, a + 4.0, -2.0, f32::NAN].into_iter().enumerate() {
        t.set_u32(CFG, rot.to_bits());
        t.set_u32(off, u32::from(k == 2));
        t.set_u8(SEL, [2u8, 0x1f, 2, 0x20, 0][k]);
        for g in flags {
            t.set_u32(g, 0x77);
        }
        dirty_stack(&mut t, k as u32, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0394::place(g, img, fr.at, fr.esp))
        });
    }
}

#[test]
fn step_and_afterimage() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x4890);
    rec(&mut t, 561, 0, |m, _| m.r32(CFG));
    rec_all(&mut t, &[(723, 1), (724, 2), (214, 1)]);
    t.set_u32(t.img.word(f + 0x31), 0x0bfc);
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
        t.set_u32(STATE + 0xc94, done);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let f = t.at(0x4b10);
    t.set_u32(t.img.word(f + 0x20), CHRS + 0x61);
    rec(&mut t, 623, 7, |_, a| a[0] ^ 0x623);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    rec(&mut t, 624, 1, |_, a| a[0] ^ 0x624);
    t.check_entry(t.at(0x4b50), Call::default(), &[], Eax::Checked);
}

#[test]
fn flight() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x5ad0);
    rec(&mut t, 11, 1, |m, _| m.r32(CFG));
    rec_all(&mut t, &[(33, 4), (32, 3)]);
    t.set_u32(t.img.word(f + 0x298), CHRS + 0x71);
    let off = t.img.word(f + 0x1ab);
    let index = t.img.word(f + 0x270);
    floats(&mut t, STATE + 0xb40, &[1.0, -2.0, 30.0, 1.0]);
    floats(&mut t, STATE + 0xb50, &[11.0, 5.5, -30.0, 1.0]);
    let obj = SCRATCH + 0x5000;
    for (n, o, i, grav) in [
        (14u32, 0u32, 3u32, 0u32),
        (7, 0, 0x20, 0),
        (2, 1, 3, 1),
        (1, 1, 3, 0),
        (0, 0, 0xffff_ffff, 0),
        (0xffff_fff0, 1, 0, 1),
        (100, 0, 0x1f, 0),
    ] {
        t.set_u32(STATE + 0xc88, n);
        t.set_u32(off, o);
        t.set_u32(index, i);
        t.set_u32(CFG, grav);
        t.set_u8(obj + 0x56, 0x5a);
        dirty_stack(&mut t, n, &[]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}
