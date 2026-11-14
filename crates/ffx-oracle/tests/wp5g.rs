//! M5 finish, WP5 part 6 (wp5g): the functions the replays of the build
//! without Unicorn stopped at, and the frontier's leftovers, against the
//! original, on the image and on the snapshots (`FFX_SNAPSHOT`). Results
//! and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! The `FFX.exe` callees are recording stubs over their code.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names
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
            let mut words: Vec<u32> = (0..0x40).map(|_| rng.pick(&choice)).collect();
            if k % 4 == 0 {
                // -1 first, a 0 later: the scan's start value decides
                words[11..19].copy_from_slice(&[
                    u32::MAX,
                    0xffff_fffe,
                    0,
                    u32::MAX,
                    0,
                    0,
                    0xffff_fffe,
                    0,
                ]);
            }
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

#[test]
fn wp5g_menu_layout_and_cmd_window_match_the_original() {
    use ffx_menu::wp5g_menu::bridge as mn;
    Twins::each(|t| {
        let mut rng = Rng(0x008e_2040);
        for k in 0..48u32 {
            t.reset();
            t.stub(mn::TK_MN2_GET_COMMON_PLATE, false, 6, 0, 0x71);
            t.stub(mn::TK_MN2_SEQ_GET_POS, false, 2, 0, 0x72);
            t.stub(mn::TK_MN2_SEQ_GET_RECT, false, 2, 0, 0x73);
            t.stub(mn::TK_MN2_SEQ_GET_NUM, false, 1, 0, 0x7400 + k);
            t.check(
                &format!("summon layout {k}"),
                0x008c_eef0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::summon_init_layout(m)),
            );
            assert_eq!(t.calls(), 17);

            // the control callback
            t.reset();
            let r = rng.next();
            t.g32(mn::CMD_DONE, rng.pick(&[0, 0, 1, r]));
            let r = rng.next();
            let slide = rng.pick(&[
                0u32,
                0x100,
                0x155,
                0xeab,
                0xf00,
                0x1000,
                0x2000,
                0x8000_0000,
                r,
            ]);
            t.g32(mn::CMD_SLIDE, slide);
            t.g32(mn::CMD_BUSY, rng.next());
            for i in 0..8 {
                let d = rng.pick(&[0u16, 0, 1, 2, 0xffff]);
                let s = rng.pick(&[0u16, 0xe3, 0xf1d, 0xf1e, 0x1000, 0x7fff, 0x8000, 0xffff]);
                let at = t.rebase(mn::ROW_DELAY) + 2 * i;
                t.write(at, &d.to_le_bytes());
                let at = t.rebase(mn::ROW_SLIDE) + 2 * i;
                t.write(at, &s.to_le_bytes());
            }
            t.check(
                &format!("cmd ctrl {k} ({slide:#x})"),
                0x008e_2040,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::cmd_window_ctrl(m)),
            );
        }
    });
}

/// A float word in -64..64 (sixteenths).
fn small_float(rng: &mut Rng) -> u32 {
    #[allow(clippy::cast_precision_loss)]
    let v = (rng.below(2048) as f32 - 1024.0) / 16.0;
    v.to_bits()
}

/// The broken-screen encounter's draw steps against the original: the
/// port's frame at the original's (aligned) `ebp`, so the callees see the
/// same addresses. The renderer's calls and the lights query are recording
/// stubs that also copy what they read through their pointers (the vertex
/// values, the light values, the block matrices, the lights request) into
/// memory the diff compares; the matrix operations run as the originals.
#[test]
fn wp5g_encounter_draw_steps_match_the_original() {
    use ffx_vfx::wp5g_encount::bridge as en;
    Twins::each(|t| {
        let mut rng = Rng(0x007f_75e0);
        for k in 0..32u32 {
            t.reset();
            let lit = k % 2 == 0;
            // the unit, a, b
            let unit_words: Vec<u32> = (0..0x40).map(|_| small_float(&mut rng)).collect();
            let unit = t.words(&unit_words);
            let oef_words: Vec<u32> = (0..0x40).map(|_| rng.next()).collect();
            let a = t.words(&oef_words);
            let b = t.zeros(0x40);
            t.w8(b + 0x1e, rng.below(4) as u8);
            t.stub(en::OPUS_NO_GET, false, 3, 0, unit);
            t.stub(en::OEF2_A_GET, false, 3, 0, 0x91);
            // the lights query: copies the request, writes its answers
            let req_copy = t.zeros(0x50);
            let mut x = vec![0x8b, 0x54, 0x24, 0x04];
            for i in 0..20u32 {
                x.extend([0x8b, 0x4a, (4 * i) as u8, 0x89, 0x0d]);
                x.extend((req_copy + 4 * i).to_le_bytes());
            }
            x.extend([0xc6, 0x42, 0x4d, rng.next() as u8]);
            x.extend([0x8b, 0x4a, 0x28]);
            for o in [0x30u8, 0x34, 0x38] {
                x.extend([0xc7, 0x41, o]);
                x.extend(small_float(&mut rng).to_le_bytes());
            }
            x.extend([0x8b, 0x4a, 0x24]);
            for o in [0u8, 4, 8] {
                x.extend([0xc7, 0x41, o]);
                x.extend(small_float(&mut rng).to_le_bytes());
            }
            t.stub_ex(
                en::MDC_LIGHTS_GET,
                false,
                5,
                0,
                &m5_main3_common::Ret::Fixed(0x92),
                &x,
            );
            // the light parameters: the four values behind args 3..6
            let mut x = Vec::new();
            for i in 0..4u8 {
                x.extend([
                    0x8b,
                    0x4c,
                    0x24,
                    0x10 + 4 * i,
                    0x8b,
                    0x09,
                    0x89,
                    0x48,
                    0x24 + 4 * i,
                ]);
            }
            t.stub_ex(
                en::LIGHT_PARAMETERS,
                false,
                7,
                0,
                &m5_main3_common::Ret::Fixed(0x93),
                &x,
            );
            // a polygon's vertex: x, y, z behind args 3..5
            let mut x = Vec::new();
            for i in 0..3u8 {
                x.extend([
                    0x8b,
                    0x4c,
                    0x24,
                    0x10 + 4 * i,
                    0x8b,
                    0x09,
                    0x89,
                    0x48,
                    0x34 + 4 * i,
                ]);
            }
            t.stub_ex(
                en::POLYGON_DATA,
                false,
                11,
                0,
                &m5_main3_common::Ret::Fixed(0x94),
                &x,
            );
            // a block's matrix (arg 2) into the next of the matrix dumps
            let (cnt, dump) = (t.zeros(4), t.zeros(0x400));
            let mut x = vec![0x56, 0x8b, 0x74, 0x24, 0x10, 0x8b, 0x0d];
            x.extend(cnt.to_le_bytes());
            x.extend([0xc1, 0xe1, 0x06, 0x81, 0xc1]);
            x.extend(dump.to_le_bytes());
            x.extend([
                0x33, 0xd2, 0x8b, 0x04, 0x96, 0x89, 0x04, 0x91, 0x42, 0x83, 0xfa, 0x10, 0x72, 0xf4,
                0xff, 0x05,
            ]);
            x.extend(cnt.to_le_bytes());
            x.push(0x5e);
            t.stub_ex(
                en::POLYGON_W_MATRIX,
                false,
                3,
                0,
                &m5_main3_common::Ret::Fixed(0x95),
                &x,
            );
            // the blocks
            let count = rng.pick(&[0u32, 1, 2, 3, 4]);
            let enc = t.zeros(0x110 + 0x70 * 4);
            t.w32(enc + 0x10, a);
            t.w32(enc + 0x14, b);
            t.write(enc + 6, &(count as u16).to_le_bytes());
            for i in 0..16 {
                let v = small_float(&mut rng);
                t.w32(enc + 0x60 + 4 * i, v);
            }
            for i in 0..count {
                let blk = enc + 0x110 + 0x70 * i;
                t.w8(blk, rng.pick(&[0u8, 1, 1, 0x80]));
                t.write(blk + 2, &(rng.below(0x1_0000) as u16).to_le_bytes());
                let nv = rng.pick(&[0i16, 1, 3, 4, -1]);
                let rec = t.zeros(0x20);
                t.write(rec + 0x12, &nv.to_le_bytes());
                t.w32(blk + 4, rec);
                let verts: Vec<u32> = (0..16).map(|_| small_float(&mut rng)).collect();
                let v = t.words(&verts);
                t.w32(blk + 8, v);
                let normals: Vec<u32> = (0..16).map(|_| rng.next()).collect();
                let n = t.words(&normals);
                t.w32(blk + 0xc, n);
                for o in [0x10u32, 0x14, 0x18, 0x50, 0x54, 0x58] {
                    let v = small_float(&mut rng);
                    t.w32(blk + o, v);
                }
            }
            let cmd = t.zeros(0x20);
            t.write(cmd + 4, &(rng.below(8) as u16).to_le_bytes());
            t.write(cmd + 6, &rng.pick(&[2i16, 8, 0x10]).to_le_bytes());
            // the original's ebp: `push ebx`, then esp - 8 aligned to 16
            let entry = t.frame_ebp(2) + 4;
            let ebp = (entry - 12) & !15;
            let lo = ebp - en::FRAME;
            t.write(lo, &vec![0; en::FRAME as usize]);
            t.b.set_call_top(Some(lo - 0x100));
            let step = if lit { en::STEP_LIT } else { en::STEP_FOG };
            t.check(
                &format!("encount4 draw {k} (lit {lit}, {count} blocks)"),
                step,
                Conv::Cdecl,
                &[enc, cmd],
                |m| Out::U32(en::encount4_draw(m, enc, cmd, ebp, lit, &mut cdecl)),
            );
            t.b.set_call_top(None);
        }
    });
}
