//! M5 finish, WP5 part 6 (wp5g): the functions the replays of the build
//! without Unicorn stopped at, and the frontier's leftovers, against the
//! original, on the image and on the snapshots (`FFX_SNAPSHOT`). Results
//! and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! The `FFX.exe` callees are recording stubs over their code.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::similar_names
)]

mod m5_main3_common;

use m5_main3_common::{G, Out, Rng, Twins};

use ffx_oracle::Conv;

/// A cdecl call through the port's oracle (the battle bridges' `Call`).
fn cdecl(m: &mut G, f: u32, args: &[u32]) -> u32 {
    m.0.call(f, Conv::Cdecl, args).unwrap().eax
}

#[test]
fn wp5g_reg_motion_read_done_matches_the_original() {
    use ffx_battle::misc::bridge::{REG_MOTION, REG_MOTION_FLAG};
    use ffx_battle::read::bridge as rd;
    Twins::each(|t| {
        let mut rng = Rng(0x0079_cc80);
        for k in 0..8 {
            t.reset();
            t.stub(rd::CHN_ADD_SYSTEM_MGRP, false, 1, 0, 0x4400 + k);
            t.g32(REG_MOTION.sym, rng.next());
            t.g8(REG_MOTION_FLAG.sym, rng.next() as u8);
            t.check(
                &format!("MsRegMotionReadDone {k}"),
                0x0079_cc80,
                Conv::Cdecl,
                &[],
                |m| Out::U32(rd::ms_reg_motion_read_done(m, &mut cdecl)),
            );
            assert_eq!(t.calls(), 1);
        }
    });
}

/// A random float word, never a signalling NaN (the emulator keeps those
/// signalling where the CPU quiets them).
fn float_word(rng: &mut Rng) -> u32 {
    let w = rng.next();
    if w & 0x7f80_0000 == 0x7f80_0000 && w & 0x0040_0000 == 0 && w & 0x003f_ffff != 0 {
        w | 0x0040_0000
    } else {
        w
    }
}

#[test]
fn wp5g_scene_read_done_and_subtitle_match_the_original() {
    use ffx_battle::misc::bridge::BTL_READ3;
    use ffx_battle::read::bridge as rd;
    Twins::each(|t| {
        let mut rng = Rng(0x0078_3dd0);
        for k in 0..24 {
            t.reset();
            t.stub(rd::MS_REGIST_SE_DATA, false, 2, 0, 0x51);
            t.stub(rd::MS_BTL_READ_SCENE_FREE, false, 0, 0, 0x52);
            t.stub(rd::MS_BTL_READ_WEAPON, false, 0, 0, 0x5300 + k);
            let n = rng.pick(&[0u32, 1, 2, 7, 0x8000_0000, u32::MAX]);
            let off = rng.pick(&[0u32, 0x40, 0x1234]);
            let block = t.words(&[n, off, rng.next(), rng.next()]);
            t.g32(BTL_READ3.sym, block);
            t.check(
                &format!("MsBtlReadSceneDone {k} ({n:#x}, {off:#x})"),
                0x0078_3dd0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(rd::ms_btl_read_scene_done(m, &mut cdecl)),
            );
            assert_eq!(t.calls(), 2);
            let lock = rng.pick(&[0, 0, 1, 0x8000_0000]);
            t.g32(rd::SUBTITLE_LOCK, lock);
            t.g32(rd::SAVE_CONFIG, rng.next());
            t.check(
                &format!("MsGetSaveConfigSubTitle {k}"),
                0x0078_5210,
                Conv::Cdecl,
                &[],
                |m| Out::U32(rd::ms_get_save_config_sub_title(m)),
            );
        }
    });
}

#[test]
fn wp5g_native_targets_match_the_original() {
    use ffx_script::wp5g::bridge as sc;
    Twins::each(|t| {
        let mut rng = Rng(0x0086_1480);
        for k in 0..40u32 {
            t.reset();
            t.stub(sc::SND_STOP_STREAM, false, 0, 0, 0x61);
            t.check(
                &format!("0x007b2c00 {k}"),
                0x007b_2c00,
                Conv::Cdecl,
                &[],
                |m| Out::U32(sc::stop_stream_0(m, &mut cdecl)),
            );

            // the loader's DONE steps
            t.stub(sc::TRANSLATE_BG_SEPS, false, 3, 0, 0x62);
            t.stub(sc::CLEAR_OTAKARA, false, 0, 0, 0x63);
            t.stub(sc::GUIDE_MAP_SET_DISP_PROG, false, 1, 0, 0x64);
            t.g8(sc::LOADER_FLAGS, rng.next() as u8);
            t.g32(sc::CUR_WORK, rng.next());
            for g in sc::DONE_WORDS {
                t.g32(g, rng.next());
            }
            let work = t.rebase(sc::MAP_WORK);
            for i in 0..8 {
                let w = float_word(&mut rng);
                t.w32(work + 0x218 + 4 * i, w);
            }
            t.check(&format!("DONE {k}"), 0x0086_1480, Conv::Cdecl, &[], |m| {
                Out::U32(sc::loader_done(m, &mut cdecl))
            });
            for g in sc::DONE_WORDS {
                t.g32(g, rng.next());
            }
            t.check(&format!("DONE2 {k}"), 0x0086_1510, Conv::Cdecl, &[], |m| {
                Out::U32(sc::loader_done2(m, &mut cdecl))
            });

            // the save event's words
            let choice = [
                0u32,
                1,
                2,
                5,
                u32::MAX,
                0xffff_fffe,
                0x8000_0000,
                0x7fff_ffff,
            ];
            let words: Vec<u32> = (0..0x40).map(|_| rng.pick(&choice)).collect();
            let e = t.words(&words);
            t.stub(sc::TK_SAVE_EVENT, false, 0, 0, e);
            t.check(
                &format!("0x0086bcc0 {k}"),
                0x0086_bcc0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(sc::save_event_max_index(m, &mut cdecl)),
            );
            let state = rng.below(7);
            t.w32(e + 0xe0, state);
            let movie = rng.next();
            t.w32(e + 0xdc, movie);
            for g in sc::MOVIE_ARGS {
                let v = rng.next();
                t.g32(g, v);
            }
            t.stub(sc::MOVIE_START, false, 5, 0, 0x65);
            t.check(
                &format!("0x00874060 {k}"),
                0x0087_4060,
                Conv::Cdecl,
                &[],
                |m| Out::U32(sc::texture_video_play(m, &mut cdecl)),
            );
            t.g8(sc::SND_BYTE, rng.next() as u8);
            t.check(
                &format!("0x00886430 {k}"),
                0x0088_6430,
                Conv::Cdecl,
                &[],
                |m| Out::U32(sc::snd_byte(m)),
            );
            let b = t.zeros(0x600);
            t.w8(b + 0x5ec, rng.next() as u8);
            t.stub(sc::MS_SAVE_EVENT, false, 0, 0, b);
            t.check(
                &format!("0x008cc140 {k}"),
                0x008c_c140,
                Conv::Cdecl,
                &[],
                |m| Out::U32(sc::save_event_bit_5ec(m, &mut cdecl)),
            );

            // the scene's waves: no list, a list without a record, a record
            t.stub(sc::WAVE_DC_STOP, false, 1, 0, 0x66);
            t.stub(sc::WAVE_C0_STOP, false, 1, 0, 0x67);
            let rec = t.words(&[rng.next(), rng.next(), rng.next()]);
            let list = match k % 3 {
                0 => 0,
                1 => t.words(&[0]),
                _ => t.words(&[rec]),
            };
            t.g32(sc::SCENE_WAVES, list);
            for (addr, stop) in [
                (0x0091_a8d0, sc::WAVE_DC_STOP),
                (0x0091_a900, sc::WAVE_C0_STOP),
            ] {
                t.check(&format!("{addr:#x} {k}"), addr, Conv::Cdecl, &[], |m| {
                    Out::U32(sc::scene_wave_stop(m, &mut cdecl, stop))
                });
            }
        }
    });
}
