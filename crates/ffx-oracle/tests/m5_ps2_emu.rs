//! M5 stage 1, list `ps2-emu` (`ffx_ps2::m5_ps2`) against the original:
//! on the image alone and on the snapshots (`FFX_SNAPSHOT`). Results and
//! every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! The thunks' targets, the descriptors' constructors, `atexit` and
//! `getDefaultInstance`, and the Ogg page step are recording stubs in
//! both oracles alike; the descriptors' guards and the Ogg state are
//! synthetic, so both branches of each run on every oracle.

#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

#[path = "m5_small/twins.rs"]
mod twins;

use ffx_ps2::m5_ps2::bridge as b;
use twins::{Conv, Out, Twins};

#[test]
fn ps2_thunks_match_the_original() {
    Twins::each(|t| {
        let thunks: [(u32, u32, u32); 7] = [
            (0x006e_d5c0, b::CDC_MUL4X4, 3),
            (0x006e_d450, b::CDC_ORTHONORMAL_INVERSE, 2),
            (0x006e_d990, b::CDC_TRANSPOSE, 2),
            (0x009d_ba10, b::FFX_VU0_MUL_MATRIX, 3),
            (0x009d_b180, b::MEMCPY, 3),
            (0x009d_be80, b::ALIGNED_FREE_THUNK, 1),
            (0x009d_be90, b::USER_ALIGNED_MALLOC, 2),
        ];
        for (sym, target, n) in thunks {
            t.reset();
            t.stub(target, false, n, 0, sym ^ 0x77);
            let args: Vec<u32> = (0..n).map(|i| 0x1000 * (i + 1) + 3).collect();
            t.check(&format!("thunk {sym:#x}"), sym, Conv::Cdecl, &args, |h| {
                Out::U32(b::jmp_thunk(h, target, &args))
            });
        }
        for v in [0, 0x1234_5678] {
            t.reset();
            t.g32(b::WORD_012FBC40.sym, v);
            t.check("code_009db170", 0x009d_b170, Conv::Cdecl, &[], |h| {
                Out::U32(b::get_word_012fbc40(h))
            });
        }
    });
}

#[test]
fn input_descriptors_match_the_original() {
    Twins::each(|t| {
        for (sym, d) in [
            (0x009d_c580u32, b::INPUT_ACTION_ARRAY),
            (0x009d_c610, b::INPUT_SOURCE_ARRAY),
        ] {
            for guard in [0u32, 1, 2, 0xffff_fffe, 0xffff_ffff] {
                t.reset();
                t.g32(d.guard, guard);
                let obj = t.rebase(d.object);
                t.w32(obj + 0x90, 0xffff_ffff);
                t.stub(d.ctor, true, 0, 0, obj);
                t.stub(b::ATEXIT, false, 1, 0, 0);
                t.stub(b::GET_DEFAULT_INSTANCE, true, 0, 0, 0x0def_0000);
                t.check(
                    &format!("descriptor {sym:#x} {guard:#x}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |h| Out::U32(b::descriptor(h, d)),
                );
            }
        }
    });
}

#[test]
fn ogg_page_end_matches_the_original() {
    Twins::each(|t| {
        let values: [u64; 6] = [
            0,
            1,
            0x1_0000_0000,
            0x8000_0000_0000_0000,
            u64::MAX,
            0xffff_ffff,
        ];
        for (i, &pos) in values.iter().enumerate() {
            for (j, &left) in values.iter().enumerate() {
                for step in [0u32, 5] {
                    t.reset();
                    let mut w = vec![0u32; 0x14];
                    w[1] = 0x4141;
                    w[0x28 / 4] = 0x00aa_0000 | (((i + j) as u32 & 1) << 23);
                    w[0x40 / 4] = pos as u32;
                    w[0x44 / 4] = (pos >> 32) as u32;
                    w[0x48 / 4] = left as u32;
                    w[0x4c / 4] = (left >> 32) as u32;
                    let this = t.words(&w);
                    t.stub(b::OGG_PAGE_STEP, true, 1, 1, step);
                    t.check(
                        &format!("0x0042d370 {i} {j} {step}"),
                        0x0042_d370,
                        Conv::Thiscall(this),
                        &[],
                        |h| Out::U32(b::ogg_page_end(h, this)),
                    );
                }
            }
        }
    });
}
