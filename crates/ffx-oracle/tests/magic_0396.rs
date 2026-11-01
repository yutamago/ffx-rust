//! `magic_0396.dll` against its Rust port (`ffx_magic::m0396` and
//! `ffx_magic::ob_summon`'s facing): the aim angles, the start, the
//! parts' refresh, the cues, the fade-out and the weapon frame. Needs
//! `FFX_GAME_DIR`.

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

use ffx_magic::{Ret, m0171, m0396};
use magic::{Call, Eax, SCRATCH, Twin};
use ownb::{dirty_stack, fill, floats, rec, rec_all, rec_seq, reset_counters};

const N: u16 = 396;

const STATE: u32 = SCRATCH + 0x8000;
const CHRS: u32 = SCRATCH + 0x800;
const CFG: u32 = SCRATCH + 0x700;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

fn twin() -> Option<Twin> {
    let mut t = magic_l3::twin(N)?;
    let f = t.at(0x4bc0);
    t.set_u32(t.img.word(f + 0x08), STATE);
    fill(&mut t, STATE + 0xb80, 0x800, 0x396);
    t.set_u32(t.img.word(f + 0x22), CHRS + 0x11);
    Some(t)
}

#[test]
fn aim() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x3910);
    let (out, from, to) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    for (a, b) in [
        ([0.0f32, 0.0, 0.0], [1.0f32, 1.0, 1.0]),
        ([1.0, 2.0, 3.0], [-4.0, 0.5, 8.0]),
        ([0.0, 0.0, 0.0], [0.0, 5.0, 0.0]),
        ([2.0, 0.0, 0.0], [-3.0, 0.0, 0.0]),
        ([0.0, 0.0, 2.0], [0.0, 0.0, -3.0]),
        ([0.0, 0.0, 0.0], [0.0, -1.0, 1e-3]),
        ([1.0, 1.0, 1.0], [1.0, 1.0, 1.0]),
    ] {
        floats(&mut t, from, &a);
        floats(&mut t, to, &b);
        fill(&mut t, out, 4, 7);
        let c = Call {
            ecx: out,
            edx: from,
            ..Call::default()
        };
        assert_eq!(ffx_magic::covered(&t.img, f), Some("m0171 aim"));
        t.check(f, c, &[to], Eax::Junk, |g, img, fr| {
            let to = fr.arg(g, 0);
            m0171::aim(g, img, fr.at, fr.ecx, fr.edx, to);
            Ret::default()
        });
    }
}

#[test]
fn face() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x4bc0);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            let v = f32::from((a[0] & 0x7f) as u8) + 0.5 * f32::from(i as u8);
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0
    });
    rec(&mut t, 333, 5, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, |m, _| f64::from(f32::from_bits(m.r32(CFG))));
    for rot in [0.0f32, 4.7, 9.0, -2.0, 5.0, f32::NAN] {
        t.set_u32(CFG, rot.to_bits());
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(ffx_magic::ob_summon::face(g, img, fr.at))
        });
    }
}

/// The object block's slots (`l5_objhost::obj_block_init`): `n` entries.
fn block_stubs(t: &mut Twin, n: u32) {
    const BLOCK: u32 = SCRATCH + 0x1000;
    const BYTES: u32 = SCRATCH + 0x1800;
    t.set_u32(BLOCK + 4, n);
    for i in 0..8 {
        t.set_u8(BYTES + i, (3 * i + 1) as u8);
    }
    rec(t, 168, 1, |_, _| BLOCK);
    rec(t, 218, 2, |_, _| BYTES);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    rec(t, 291, 5, |_, a| a[1] + 0x10);
    rec(t, 367, 8, |_, a| a[2] ^ 0x367);
}

#[test]
fn start() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x4570);
    rec_all(&mut t, &[(556, 1), (555, 2), (331, 3), (790, 0), (720, 2)]);
    rec(&mut t, 718, 3, |_, a| a[0] ^ 0x718);
    rec(&mut t, 222, 7, |_, _| 0x222);
    rec(&mut t, 221, 11, |_, _| 0x221);
    rec(&mut t, 719, 14, |_, a| a[0] + 0x40);
    rec(&mut t, 551, 17, |_, _| 0x551);
    t.stub_st0(790, |_, _| 1.25);
    let payloads = t.img.word(f + 0x81);
    for n in [0u32, 2] {
        block_stubs(&mut t, n);
        t.write(payloads, &vec![0xa5; 12 * 0x3ff]);
        for op in [0x57, 0x61, 0x6b, 0xa3, 0xab, 0x13b] {
            let g = t.img.word(f + op);
            t.set_u32(g, 0x0101_0101);
        }
        t.check_entry(f, Call::default(), &[0x00ab_c000], Eax::Checked);
    }
}

#[test]
fn refresh() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x4c80);
    rec_all(&mut t, &[(628, 1)]);
    rec(&mut t, 772, 2, |_, a| a[0] ^ (a[1] << 8));
    let lock = t.u32(t.img.word(f + 0x51));
    rec(&mut t, 779, 4, move |m, a| {
        // runs with the lock set (kept where the comparison sees it)
        let k = m.r32(SCRATCH + 0x3f0);
        m.w32(SCRATCH + 0x3f0, k + 1);
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x3f4 + 4 * (k % 3), l);
        l ^ a[0]
    });
    let parts = t.img.word(f + 0x1c);
    for hs in [[0u32, 0, 0, 0], [0x11, 0, 0x33, 0], [0, 0x22, 0, 0x44]] {
        for (i, h) in (0u32..).zip(hs) {
            t.set_u32(parts + 4 * i, h);
        }
        t.set_u32(lock, 0x77);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0396::refresh(g, img, fr.at))
        });
    }
}

#[test]
fn cues() {
    let Some(mut t) = twin() else { return };
    rec(&mut t, 719, 3, |_, a| a[0] ^ a[2] ^ 0x719);
    rec(&mut t, 306, 6, |_, a| a[1] + 0x306);
    rec(&mut t, 220, 6, |_, a| a[0] + 0x220);
    let f = t.at(0x4d90);
    let alt = t.img.word(f + 0x02);
    for v in [0u32, 1, 0x8000_0000] {
        t.set_u32(alt, v);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
        t.check_entry(t.at(0x4e20), Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn fades() {
    let Some(mut t) = twin() else { return };
    let obj = SCRATCH + 0x5000;
    fill(&mut t, obj + 0x60, 0x50, 9);
    t.check_entry(t.at(0x5150), Call::default(), &[obj], Eax::Checked);
    for n in [30u32, 2, 1, 0, 0x8000_0000, 0x7fff_ffff] {
        t.set_u32(obj + 0x120, n);
        t.set_u32(obj + 0x104, 0x1234);
        t.check_entry(t.at(0x5100), Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn weapon_frame() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x4f90);
    t.set_u32(t.img.word(f + 0x27), CHRS + 0x21);
    rec(&mut t, 219, 2, |_, a| a[0] ^ 0x219);
    rec(&mut t, 236, 5, |m, a| {
        for i in 0..16 {
            m.w32(a[2] + 4 * i, (1.5f32 + f32::from(i as u8)).to_bits());
        }
        0
    });
    rec(&mut t, 277, 6, |_, _| 0);
    rec_seq(&mut t, 235, &[7], |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0
    });
    rec(&mut t, 330, 3, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ a[2]);
        }
        0
    });
    rec(&mut t, 231, 6, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(3);
            m.w32(a[0] + 4 * i, v);
        }
        0
    });
    rec(&mut t, 268, 9, |_, a| a[2] ^ 0x268);
    let obj = SCRATCH + 0x5000;
    for d in [2.0f64, 0.5, -1.0, 0.0] {
        t.stub_st0(277, move |_, _| d);
        reset_counters(&mut t);
        dirty_stack(&mut t, d.to_bits() as u32, &[]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

// Mutation testing (M5 magic round 3, tools/mutants.py, a mutant per
// code line plus a deletion per store or call):
// - ffx_magic::m0396: 107 mutants, 104 killed. Equivalent: aim's `hi`
//   operand and `y > hi` as `>=` (`hi` only orders the two stores to
//   `out + 4` and `out + 8`; the memory after is the same);
//   weapon_frame's `stack.insert(1, wpn)` (`stack[0]` is `wpn` already).
// - ffx_magic::m0395 (magic_0395): 20 mutants, all killed.

/// 0x10004af0: paused, the script or the motion still running, and the
/// end (the parts refreshed with one handle, then the node alone given
/// back to the pool).
#[test]
fn script_end() {
    let Some(mut t) = twin() else { return };
    let at = t.at(0x4af0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let paused = SCRATCH + 0x5f0;
    rec(&mut t, 561, 0, move |m, _| m.r32(paused));
    rec_all(&mut t, &[(628, 1)]);
    rec(&mut t, 772, 2, |_, a| a[0] ^ (a[1] << 8));
    rec(&mut t, 779, 4, |_, a| a[0] ^ 0x779);
    t.set_u32(w(&t, 0x06), STATE);
    let rf = t.img.rel32_target(at + 0x63);
    let parts = t.img.word(rf + 0x1c);
    let lock = t.u32(t.img.word(rf + 0x51));
    let run = t.img.rel32_target(at + 0x33);
    let ms = t.img.rel32_target(at + 0x4c);
    let give = t.img.rel32_target(at + 0x6e);
    let s = t.img.word(run + 0x1c);
    let node = SCRATCH + 0x2000;
    for (p, mode, done) in [(1u32, 0u8, true), (0, 2, true), (0, 0, false), (0, 0, true)] {
        t.set_u32(paused, p);
        for (i, h) in (0u32..).zip([0u32, 0x22, 0, 0]) {
            t.set_u32(parts + 4 * i, h);
        }
        t.set_u32(lock, 0x77);
        t.set_u32(w(&t, 0x26), 0x0026_0026);
        t.set_u16(w(&t, 0x21), 0x1234);
        // the script ended (mode 0, returns 1) or waiting for ever (2)
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u16(s + 0x54, 0xffff);
        t.set_u32(s + 0x24, 0);
        t.set_u32(s + 0x28, 0);
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(STATE + 0x2458, 0x2458);
        // the node alone (no siblings, nothing chained) for the pool
        t.write(node, &[0; 0x1c]);
        t.set_u32(node + 8, node);
        for op in [0x48, 0x55, 0x64, 0x74] {
            t.set_u32(t.img.word(give + op), SCRATCH + 0x3000 + op);
        }
        t.set_u32(SCRATCH + 0x3074, SCRATCH + 0x3074);
        t.set_u32(SCRATCH + 0x3078, SCRATCH + 0x3074);
        t.set_u16(t.img.word(give + 0x8e), 7);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}
