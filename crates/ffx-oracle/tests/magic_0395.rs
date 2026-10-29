//! `magic_0395.dll` against its Rust port (`ffx_magic::m0395` and the
//! summon entries of `ffx_magic::ob_summon`): the facing, the
//! vibration, the end's resets, the task step and the afterimage.
//! Needs `FFX_GAME_DIR`.

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

use ffx_magic::{Ret, m0395};
use magic::{Call, Eax, SCRATCH, Twin};
use ownb::{fill, rec, rec_all};

const N: u16 = 395;

const STATE: u32 = SCRATCH + 0x8000;
const CHRS: u32 = SCRATCH + 0x800;
const CFG: u32 = SCRATCH + 0x700;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

fn twin() -> Option<Twin> {
    let mut t = magic_l3::twin(N)?;
    let state_g = t.img.word(t.at(0x4200) + 0x08);
    t.set_u32(state_g, STATE);
    fill(&mut t, STATE + 0x1080, 0x200, 0x395);
    t.set_u32(t.img.word(t.at(0x4200) + 0x22), CHRS + 0x11);
    Some(t)
}

#[test]
fn face() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x4200);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            let v = f32::from((a[0] & 0x7f) as u8) + 0.5 * f32::from(i as u8);
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0
    });
    rec(&mut t, 333, 5, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, |m, _| f64::from(f32::from_bits(m.r32(CFG))));
    let flags = [0xa2, 0xac].map(|o| t.img.word(f + o));
    let k = |t: &Twin, op: u32| f32::from_bits(t.u32(t.img.word(f + op)));
    let (a, hi) = (k(&t, 0x4f), k(&t, 0x5e));
    let mut rots = vec![0.0f32, a, a + 4.0, -2.0, 5.0, f32::NAN];
    let guess = (a - hi).to_bits();
    if let Some(r) = (0..256u32)
        .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
        .map(f32::from_bits)
        .find(|&r| (a - r).to_bits() == hi.to_bits())
    {
        rots.push(r);
    }
    for rot in rots {
        t.set_u32(CFG, rot.to_bits());
        for g in flags {
            t.set_u32(g, 0x77);
        }
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(ffx_magic::ob_summon::face(g, img, fr.at))
        });
    }
}

#[test]
fn vibrate_and_resets() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x4110);
    rec(&mut t, 719, 3, |_, a| a[0] ^ 0x719);
    rec(&mut t, 306, 6, |_, a| a[1] + 0x306);
    let flag = t.img.word(f + 0x02);
    for v in [0u32, 1, 0x8000_0000] {
        t.set_u32(flag, v);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let f = t.at(0x42c0);
    rec_all(&mut t, &[(628, 1), (933, 4)]);
    rec(&mut t, 785, 1, |_, a| a[0] ^ 0x785);
    let scale = t.img.word(f + 0x46);
    let one = t.img.word(f + 0x55);
    t.set_u32(scale, 0x4040_0000);
    t.set_u32(one, 0x77);
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0395::resets(g, img, fr.at))
    });
}

#[test]
fn step_and_afterimage() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x41b0);
    rec(&mut t, 561, 0, |m, _| m.r32(CFG));
    rec_all(&mut t, &[(723, 1), (724, 2), (235, 3), (214, 1)]);
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
        t.set_u32(STATE + 0x1530, done);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    rec(&mut t, 623, 7, |_, a| a[0] ^ 0x623);
    t.check_entry(t.at(0x4340), Call::default(), &[], Eax::Checked);
    rec(&mut t, 624, 1, |_, a| a[0] ^ 0x624);
    t.check_entry(t.at(0x4920), Call::default(), &[], Eax::Checked);
}
