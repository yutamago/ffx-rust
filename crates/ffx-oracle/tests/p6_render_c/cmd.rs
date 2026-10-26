//! Continuation round 5: `PRenderThread::handleNonRenderInstanceCommand`
//! (`ffx_prender::p6_cmd`) against its original.

#![allow(clippy::cast_precision_loss)] // small test values

use ffx_oracle::Conv;
use ffx_prender::p6_cmd as cm;

use super::{G, Out, Rng, Twins, object};

/// The callees and the words their `ret n` pops (read off the machine
/// code, not the port's table).
const POPS: &[(u32, u32)] = &[
    (0x0061_5de0, 4),
    (0x0059_aaf0, 4),
    (0x0061_5bc0, 4),
    (0x0059_9cc0, 1),
    (0x0059_9d80, 2),
    (0x0059_9eb0, 1),
    (0x0059_9ec0, 1),
    (0x0061_5c50, 2),
    (0x0061_5d00, 1),
    (0x0058_ec50, 1),
    (0x0059_bc40, 3),
    (0x0059_bc30, 3),
    (0x0058_e510, 3),
    (0x0059_8d00, 3),
    (0x0058_ebb0, 2),
    (0x0061_47b0, 1),
    (0x0058_e3d0, 1),
    (0x0059_3010, 0),
    (0x0059_3760, 0),
    (0x0043_0210, 2),
    (0x0059_3790, 0),
    (0x0059_98f0, 3),
    (0x0059_9950, 5),
    (0x0061_5b50, 4),
    (0x0059_97a0, 3),
    (0x0059_97b0, 2),
    (0x0059_9810, 2),
    (0x0059_a120, 2),
    (0x0059_a1c0, 1),
    (0x0059_a070, 1),
    (0x0059_a8d0, 1),
    (0x0061_5da0, 3),
    (0x0059_9ed0, 4),
    (0x0059_a8b0, 1),
    (0x0059_a2b0, 1),
    (0x0059_a8c0, 1),
    (0x0059_bac0, 15),
    (0x0061_5cd0, 3),
    (0x0059_7ff0, 1),
    (0x0059_7f20, 1),
    (0x0059_7f10, 0),
    (0x0061_c870, 0),
];

/// A stub of a callee of `pops` words over 6: copies its tag, `ecx` and
/// all its words to `buf` (one call per case), returns `ret`.
fn wide_stub(t: &mut Twins, buf: u32, pops: u32, ret: u32) -> u32 {
    let mut c = vec![0xc7, 0x05];
    c.extend(buf.to_le_bytes());
    c.extend(0x5157_1de0_u32.to_le_bytes()); // mov dword [buf], tag
    c.extend([0x89, 0x0d]);
    c.extend((buf + 4).to_le_bytes()); // mov [buf+4], ecx
    for i in 0..pops {
        c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x15]);
        c.extend((buf + 8 + 4 * i).to_le_bytes()); // mov [buf+8+4i], edx
    }
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.push(0xc2);
    c.extend(((pops * 4) as u16).to_le_bytes());
    t.code(&c)
}

/// A float word for a payload: ordinary values, zeros, infinities, quiet
/// NaNs and denormals (no signalling NaNs: the emulator keeps them).
fn float_word(rng: &mut Rng) -> u32 {
    match rng.below(8) {
        0 => 0,
        1 => 0x8000_0000,
        2 => 0x7f80_0000,
        3 => 0xffc0_0000 | (rng.next() & 0x3f_ffff),
        4 => rng.next() & 0x807f_ffff,
        _ => {
            let v = (rng.next() & 0xffff) as f32 / 37.0 - 800.0;
            v.to_bits()
        }
    }
}

#[test]
fn handle_non_render_instance_command_matches_the_original() {
    // every callee of the table is in POPS with the words the port passes
    for (i, k) in cm::KINDS.iter().enumerate() {
        let pops = |f: u32| POPS.iter().find(|p| p.0 == f).map(|p| p.1);
        match *k {
            cm::Kind::Call(f, a) | cm::Kind::State(f, a, _) => {
                assert_eq!(pops(f), Some(a.len() as u32), "kind {}: {f:#x}", i + 4);
            }
            cm::Kind::Pick(f, g) => {
                assert_eq!(pops(f), Some(3));
                assert_eq!(pops(g), Some(3));
            }
            _ => {}
        }
    }
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0c60);
        let wide = t.zeros(0x48);
        for &(f, pops) in POPS {
            let ret = 0x00c9_0000 | (f & 0xffff);
            if pops > 6 {
                let s = wide_stub(&mut t, wide, pops, ret);
                t.jump(f, s);
            } else {
                t.stub(f, true, pops, pops, ret);
            }
        }
        // callees whose eax low byte is 0 (the flush skipped on bit 0)
        let mut n = 0;
        for kind in 0..48u32 {
            for case in 0..6u32 {
                let thread = object(&mut t, &mut rng, 0x60, false);
                let renderer = object(&mut t, &mut rng, 4, false);
                let dirty = object(&mut t, &mut rng, 4, false);
                let cmd = t.zeros(0x48);
                t.w32(cmd, kind);
                for k in (8..0x44).step_by(4) {
                    let w = if case % 2 == 0 {
                        float_word(&mut rng)
                    } else {
                        rng.next()
                    };
                    t.w32(cmd + k, w);
                }
                // kind 14's pick
                if case < 2 {
                    t.w32(cmd + 0x14, 0);
                }
                // the dirty mask: all clear, all set, random
                let mask = match case {
                    0 => 0,
                    1 => u32::MAX,
                    _ => rng.next(),
                };
                t.w32(dirty + 8, mask);
                // kinds 24 and 25's flush test takes the callee's eax:
                // its low byte 0 or not
                if case == 3 {
                    t.stub(0x0059_98f0, true, 3, 3, 0x1234_5600);
                    t.stub(0x0059_9950, true, 5, 5, 0x1234_5600);
                }
                let log = t.log;
                t.w32(log, 0);
                t.check(
                    &format!("{name} kind {kind} case {case}"),
                    0x0061_4c60,
                    Conv::Thiscall(thread),
                    &[renderer, cmd, dirty],
                    |g: &mut G| Out::U32(cm::bridge::handle(g, thread, renderer, cmd, dirty)),
                );
                if case == 3 {
                    t.unstub(0x0059_98f0);
                    t.unstub(0x0059_9950);
                    t.stub(0x0059_98f0, true, 3, 3, 0x00c9_98f0);
                    t.stub(0x0059_9950, true, 5, 5, 0x00c9_9950);
                }
                n += t.log_entries();
            }
        }
        assert!(n > 200, "{name}: {n} calls");
    }
}
