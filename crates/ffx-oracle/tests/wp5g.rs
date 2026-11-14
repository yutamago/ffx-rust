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

/// x87 stand-ins for `graphicUiRemapX2` (`-x`) and `graphicUiRemapY2`
/// (`2y`), logging their argument.
fn remap_stubs(t: &mut Twins) {
    use ffx_menu::wp5g_menu::bridge as mn;
    let fixed = m5_main3_common::Ret::Fixed;
    t.stub_ex(
        mn::UI_REMAP_X2,
        false,
        1,
        0,
        &fixed(0x81),
        &[0xd9, 0x44, 0x24, 0x04, 0xd9, 0xe0],
    );
    t.stub_ex(
        mn::UI_REMAP_Y2,
        false,
        1,
        0,
        &fixed(0x82),
        &[0xd9, 0x44, 0x24, 0x04, 0xdc, 0xc0],
    );
}

#[test]
fn wp5g_menu_callbacks_match_the_original() {
    use ffx_menu::wp5g_menu::bridge as mn;
    Twins::each(|t| {
        let mut rng = Rng(0x008a_91d0);
        for k in 0..40u32 {
            // the battle cursors
            t.reset();
            t.stub(mn::TO_MAKE_PKT_SHAPE_CURSOR, false, 3, 0, 0xa1);
            let n = rng.pick(&[0i16, 1, 3, 5, -1]);
            t.write(t.rebase(mn::BTL_CURSOR_COUNT), &n.to_le_bytes());
            let recs = t.rebase(mn::BTL_CURSORS);
            for i in 0..6 {
                let w = rng.next();
                t.w32(recs + 8 * i, w);
                let w = rng.next();
                t.w32(recs + 8 * i + 4, w);
            }
            t.check(
                &format!("btl cursors {k}"),
                0x0089_30f0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::btl_draw_cursors(m)),
            );

            // the top window's control
            t.reset();
            t.stub(mn::GET_CMD_SLIDE, false, 0, 0, 0xa2);
            for (g, v) in [
                (
                    mn::TOP_FADE,
                    rng.pick(&[0u32, 0xaab, 0xaac, 0x1000, 0x7fff_fffe]),
                ),
                (mn::TOP_SLIDE, rng.pick(&[0u32, 0xff, 0x100, 0xf00, 0x1000])),
                (
                    mn::TOP_SLIDE_STEP,
                    rng.pick(&[0u32, 0x100, 0xffff_ff00, 0x200, 0xffff_e000]),
                ),
            ] {
                t.g32(g, v);
            }
            t.check(
                &format!("top ctrl {k}"),
                0x008a_91d0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::top_window_ctrl(m)),
            );

            // the password close, the summon test, the customize clear,
            // the module files' close
            t.reset();
            let lang = rng.pick(&[0u32, 9, 10, 11]);
            t.stub_ex(
                mn::TO_GET_FFX_LANG_JMP,
                false,
                0,
                0,
                &m5_main3_common::Ret::Seq(vec![lang, rng.pick(&[9u32, 10, 3])]),
                &[],
            );
            t.stub(mn::PASSWORD_TEXT_DISPOSE, false, 0, 0, 0xa3);
            t.check(
                &format!("password close {k}"),
                0x008c_4490,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::password_close(m)),
            );
            let w = if k % 4 == 0 { 0 } else { t.zeros(0x60) };
            if w != 0 {
                let row = rng.below(8) as i16;
                t.write(w + 0x48, &row.to_le_bytes());
            }
            t.g32(mn::SUMMON_CURSOR_WINDOW, w);
            let rows = t.rebase(mn::SUMMON_ROW_BYTES);
            for i in 0..8 {
                t.w8(rows + i, rng.pick(&[0u8, 1, 1, 2]));
            }
            t.stub(
                mn::TK_MENU_GET_CURRENT_SUMMON,
                false,
                0,
                0,
                rng.pick(&[0u32, 13, 14, 20, u32::MAX]),
            );
            t.check(
                &format!("summon test {k}"),
                0x008c_cad0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::summon_row_test(m)),
            );
            t.g32(0x0186_aa68, rng.next());
            t.check(
                &format!("kaizou clear {k}"),
                0x008d_56e0,
                Conv::Cdecl,
                &[],
                |m| {
                    mn::kaizou_clear(m);
                    Out::None
                },
            );
            t.stub(mn::READ_MODULE_FILES, false, 0, 0, 0xa4);
            t.g32(0x023c_c130, rng.next());
            t.check(
                &format!("module files close {k}"),
                0x008e_2910,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::module_files_close(m)),
            );

            // the draws
            t.reset();
            remap_stubs(t);
            for (f, n) in [
                (mn::TO_DRAW_MENU_BG, 0),
                (mn::MENU_DRAW_DIVIDER_SHAPES, 1),
                (mn::TK_MENU_DRAW_KICK, 0),
                (mn::TK_MN2_DRAW_INIT_PACKET, 0),
                (mn::MENU_STATUS_DRAW_PAGE, 9),
                (mn::TK_MN2_MAIN_STATUS_PLAYER_STATUS, 4),
                (mn::TO_MENU_DRAW_CHR_STATUS_PLATE, 3),
                (mn::TK_MN2_DRAW_STATUS_BG, 2),
                (mn::TK_MN2_DRAW_KICK_SYNC_PACKET, 0),
                (mn::TK_VU1_SYNC_PATH_THUNK, 0),
                (mn::TK_MN2_DRAW_GILL_PLATE, 5),
                (mn::TO_MKP_SHAPE_XYWH_UV, 9),
                (mn::TO_MKP_ROTATE_SHAPE_XYWH_UVC2, 12),
                (mn::MENU_ITEM_DRAW_CHR_SMALL_PLATE, 3),
                (mn::MENU_WSHOP_DRAW_COMPARE, 2),
            ] {
                t.stub(f, false, n, 0, 0xb000 + f % 0x1000);
            }
            t.stub(mn::TK_MENU_GET_CURRENT_SUMMON, false, 0, 0, rng.below(18));
            t.stub(mn::TK_MN2_GET_STATUS_BG, false, 0, 0, rng.next());
            t.stub(mn::MS_GET_GIL, false, 0, 0, rng.next());
            t.check(
                &format!("summon status draw {k}"),
                0x008c_d080,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::summon_status_draw(m)),
            );
            t.check(
                &format!("wshop window 3 draw {k}"),
                0x008d_a6b0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::wshop_window3_draw(m)),
            );
            t.g32(mn::WSHOP_CHR, rng.below(8));
            let c0 = rng.pick(&[0u32, 0, 0x1234]);
            t.g32(mn::WSHOP_COMPARE[0], c0);
            let c1 = rng.pick(&[0u32, 0x5678]);
            t.g32(mn::WSHOP_COMPARE[1], c1);
            t.check(
                &format!("wshop sub window 4 draw {k}"),
                0x008d_7f50,
                Conv::Cdecl,
                &[],
                |m| Out::U32(mn::wshop_sub_window4_draw(m)),
            );
        }
    });
}

#[test]
fn wp5g_misc_gap_functions_match_the_original() {
    use ffx_prender::wp5g::bridge as ms;
    Twins::each(|t| {
        let mut rng = Rng(0x0087_e6f0);
        for k in 0..24u32 {
            // the event debug copy pair (memcpy and dbgPrintf as stubs)
            t.reset();
            let e = t.zeros(0x1200);
            t.stub(ms::TK_GET_SAVE_EVENT_ADDRESS, false, 0, 0, e);
            t.stub(ms::MEMCPY, false, 3, 0, 0xc1);
            t.stub(ms::DBG_PRINTF, false, 1, 0, 0xc2);
            for (addr, restore) in [(0x0087_e6f0, true), (0x0087_e720, false)] {
                t.check(
                    &format!("event debug {addr:#x} {k}"),
                    addr,
                    Conv::Cdecl,
                    &[],
                    |m| Out::U32(ms::event_debug_copy(m, restore)),
                );
            }
            t.stub(ms::SG_DISPOSE_MOTION_GROUP, false, 3, 0, 0xc3);
            t.stub(ms::CACHE_DISPOSE, false, 3, 0, 0xc4);
            t.check(
                &format!("mgrp dispose {k}"),
                0x0083_7690,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::system_mgrp_dispose(m)),
            );

            // the semaphore queue's destructor: the import and the iterator
            t.reset();
            let slot = ms::CLOSE_HANDLE_SLOT;
            let close = t.stub_code(slot, false, 1, 1, &m5_main3_common::Ret::Fixed(0xc5), &[]);
            let at = t.rebase(slot);
            t.patch(at, &close.to_le_bytes());
            t.stub(ms::EH_VECTOR_DTOR, false, 4, 4, 0xc6);
            let q: Vec<u32> = (0..0x80).map(|_| rng.next()).collect();
            let q = t.words(&q);
            t.check(
                &format!("semaphore queue dtor {k}"),
                0x006d_b6f0,
                Conv::Thiscall(q),
                &[],
                |m| Out::U32(ms::semaphore_queue_dtor(m, q)),
            );

            // the battle position lists
            t.reset();
            t.stub_ex(
                ms::MS_MALLOC,
                false,
                1,
                0,
                &m5_main3_common::Ret::Seq(vec![0x0110_0000 + k, 0x0120_0000 + k]),
                &[],
            );
            t.stub(ms::MS_STRUCT_CLEAR, false, 2, 0, 0xc7);
            let pos: Vec<u32> = (0..0x20).map(|_| rng.next() % 64).collect();
            let pos = t.words(&pos);
            t.g32(ms::BTL_POS, pos);
            t.check(
                &format!("btl pos 0x30 {k}"),
                0x007c_91a0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::btl_pos_alloc_lists(m, 0x30, 0x34, 0x4c)),
            );
            t.check(
                &format!("btl pos 0xc {k}"),
                0x007c_c180,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::btl_pos_alloc_lists(m, 0xc, 0x10, 0x28)),
            );

            // the scene's wave draws
            t.reset();
            t.stub(0x0092_0740, false, 1, 0, 0xc8);
            t.stub(0x0092_0780, false, 1, 0, 0xc9);
            let rec = t.words(&[rng.next(), rng.next(), rng.next()]);
            let list = match k % 3 {
                0 => 0,
                1 => t.words(&[0]),
                _ => t.words(&[rec]),
            };
            t.g32(ms::SCENE_WAVES, list);
            for (addr, draw) in [(0x0092_15e0, 0x0092_0740), (0x0092_1610, 0x0092_0780)] {
                t.check(&format!("{addr:#x} {k}"), addr, Conv::Cdecl, &[], |m| {
                    Out::U32(ms::scene_wave_draw(m, draw))
                });
            }
            for (addr, body) in [(0x0092_1a10, 0x0092_15e0), (0x0092_1a30, 0x0092_1610)] {
                t.check(&format!("{addr:#x} {k}"), addr, Conv::Cdecl, &[], |m| {
                    Out::U32(ms::scene_wave_draw_if_any(m, body))
                });
            }

            // the standby's end, the request list, the debug list, HDD
            t.reset();
            let w = t.words(&[rng.next()]);
            t.g32(0x00c4_9464, w);
            t.g32(0x012f_4e74, rng.next());
            t.check(
                &format!("standby done {k}"),
                0x0081_fcb0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::sqs_standby_done(m)),
            );
            t.stub(ms::USER_FREE, false, 1, 0, 0xca);
            let p = if k % 3 == 0 { 0 } else { rng.next() };
            let list = t.words(&[0, 0, p]);
            t.check(
                &format!("request list dtor {k}"),
                0x0069_0a30,
                Conv::Thiscall(list),
                &[],
                |m| Out::U32(ms::request_list_dtor(m, list)),
            );
            t.stub(ms::TK_IS_DEBUG, false, 0, 0, rng.pick(&[0u32, 1]));
            t.stub(ms::ATEL_REMOVE_PS2_DEBUG, false, 0, 0, 0xcb);
            t.stub(ms::FLUSH_CACHE, false, 1, 0, 0xcc);
            t.stub(ms::TK_FREE, false, 1, 0, 0xcd);
            let blocks = t.rebase(ms::DEBUG_BLOCKS);
            for i in 0..2 {
                let v = rng.pick(&[0u32, 0x0123_4560]);
                t.w32(blocks + 4 * i, v);
            }
            t.check(
                &format!("debug list release {k}"),
                0x0087_3d60,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::release_debug_list(m)),
            );
            t.stub(ms::HDD_MODE_REFRESH, false, 1, 0, 0xce);
            t.g32(0x0132_f570, rng.pick(&[0u32, 1, 5, u32::MAX]));
            t.check(
                &format!("hdd refresh {k}"),
                0x0088_5560,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::hdd_mode_refresh(m)),
            );

            // the battle menu's scene steps
            t.reset();
            t.stub(ms::TO_SEQ_STOP_EXEC_SCENE, false, 1, 0, 0xcf);
            t.stub(ms::TO_SEQ_START_SCENE, false, 2, 0, 0xd0);
            t.stub(ms::TO_BTL_DRAW_TARGET_CURSOR, false, 0, 0, 0xd1);
            for a in (0x008a_2a40..0x008a_2ac0).step_by(0x10) {
                t.check(&format!("scene stop {a:#x}"), a, Conv::Cdecl, &[], |m| {
                    Out::U32(ms::btl_scene_stop(m))
                });
            }
            for (a, name) in [
                (0x008a_86b0, 0x00b5_f088),
                (0x008a_86f0, 0x00b5_f088),
                (0x008a_8710, 0x00b5_f088),
                (0x008a_8730, 0x00b5_f088),
                (0x008a_8780, 0x00b5_f094),
            ] {
                t.check(&format!("scene start {a:#x}"), a, Conv::Cdecl, &[], |m| {
                    Out::U32(ms::btl_scene_start(m, name))
                });
            }
            t.check("target cursor 0", 0x008a_84c0, Conv::Cdecl, &[], |m| {
                Out::U32(ms::btl_target_cursor_0(m))
            });
        }
    });
}

/// A void step and its original's address.
type Step = (u32, fn(&mut G));

#[test]
fn wp5g_grid_ui_and_task_steps_match_the_original() {
    use ffx_prender::wp5g::bridge as ms;
    Twins::each(|t| {
        let mut rng = Rng(0x00a5_3450);
        let g = t.zeros(0x1_1800);
        let menu = t.zeros(0x2800);
        for k in 0..32u32 {
            t.reset();
            t.g32(ms::ABMAP, g);
            t.g32(ms::ABMAP_MENU, menu);
            for f in [
                ms::DRAW_LINKS,
                ms::DRAW_NODE_HALOS,
                ms::DRAW_CHR_RINGS,
                ms::DRAW_SPHERE_JOB_TAIL,
                ms::DRAW_NODES_EAMAP,
                ms::DRAW_NODE_SHAPES_TAIL,
                ms::DRAW_NODE_SHAPES,
                ms::STATE_HOME_ON_NODE,
                ms::MARK_EMPTY_MENUS,
            ] {
                t.stub(f, false, 0, 0, 0xe000 + f % 0x100);
            }
            for f in [
                ms::RESET_LAYERS,
                ms::DRAW_NODE_SPHERES,
                ms::DRAW_NODE_HIGHLIGHT,
                ms::HELP_MESSAGE_STEP,
                ms::RUN_PART,
            ] {
                t.stub(f, false, 1, 0, 0xe100 + f % 0x100);
            }
            let node = rng.pick(&[u32::MAX, 0xffff_fffe, 0, 7, 0x8000_0000]);
            t.w32(g + 0x116a8, node);
            t.w32(g + 0x116ac, rng.pick(&[0u32, 1, 2, 3, 4, u32::MAX]));
            t.w32(g + 0x116b0, rng.pick(&[0u32, 2, 0xffff_fffe, 5, u32::MAX]));
            t.w8(g + 0x115c3, rng.pick(&[0u8, 1, 2]));
            t.w8(g + 0x1164c, rng.pick(&[0u8, 1, 2, 3]));
            for o in [0x115a8u32, 0x115ac, 0x115b0, 0x115b4] {
                let v = rng.next();
                t.w32(g + o, v);
            }
            let progress = rng.pick(&[0.5f32, 0.999, 1.0, 1.5, f32::NAN]).to_bits();
            t.w32(g + 0x11620, progress);
            t.w32(menu + 0x16e4, rng.next());
            let panel = rng.pick(&[0u16, 1]);
            t.write(t.rebase(0x01a8_607e), &panel.to_le_bytes());
            let jobs: [Step; 4] = [
                (0x00a5_3450, |m| ms::abmap_job_links(m)),
                (0x00a5_34b0, |m| ms::abmap_job_halos(m)),
                (0x00a5_34c0, |m| ms::abmap_job_spheres(m)),
                (0x00a5_3510, |m| ms::abmap_job_shapes(m)),
            ];
            for (addr, port) in jobs {
                t.check(&format!("{addr:#x} {k}"), addr, Conv::Cdecl, &[], |m| {
                    port(m);
                    Out::None
                });
            }
            let hooks: [Step; 3] = [
                (0x00a4_8280, |m| ms::abmap_restore_state(m)),
                (0x00a4_c430, |m| ms::abmap_warp_draw(m)),
                (0x00a5_a020, |m| ms::abmap_select_homing(m)),
            ];
            for (addr, port) in hooks {
                t.check(&format!("{addr:#x} {k}"), addr, Conv::Cdecl, &[], |m| {
                    port(m);
                    Out::None
                });
            }
            t.check(
                &format!("help draw {k}"),
                0x00a5_1700,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::abmap_help_draw(m)),
            );

            // the PC UI steps: a fake context with a logging ClearDepthStencilView
            t.reset();
            let ui = t.zeros(0xc00);
            let ctx = t.zeros(0x10);
            let clear = t.stub_code(0x5d4, false, 5, 5, &m5_main3_common::Ret::Fixed(0xe2), &[]);
            let vt = t.zeros(0x100);
            t.w32(vt + ms::CLEAR_DSV_SLOT, clear);
            t.w32(ctx, vt);
            t.w32(ui + 4, ctx);
            let v = rng.next();
            t.w32(ui + 0x784, v);
            let v2 = rng.pick(&[0u32, 0x55]);
            t.w32(ui + 0x788, v2);
            let r = rng.next();
            t.w32(ui + 0xab0, r);
            let r = rng.next();
            t.w32(ui + 0xab4, r);
            t.g32(ms::PCUI, ui);
            t.stub(ms::NODE_REQUEUE_ALL, false, 1, 0, 0xe3);
            t.check(
                &format!("pcui clear depth {k}"),
                0x00a3_73c0,
                Conv::Cdecl,
                &[],
                |m| {
                    ms::pcui_clear_depth(m);
                    Out::None
                },
            );
            t.check(
                &format!("pcui requeue {k}"),
                0x00a3_b090,
                Conv::Stdcall,
                &[0x77],
                |m| Out::U32(ms::pcui_requeue_nodes(m)),
            );

            // the read task, the field scene step
            t.reset();
            t.stub(ms::CD_DECODE_DATA, false, 1, 0, 0xe4);
            t.stub(ms::HD_FILE_READ_TH_MAIN, false, 0, 0, 0xe5);
            t.g8(ms::READ_REQUEST, rng.pick(&[0u8, 1, 2, 3, 0xff]));
            t.g32(0x0231_0c2c, rng.next());
            t.check(
                &format!("read task {k}"),
                0x0076_e5f0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::read_task_step(m)),
            );
            let debug = rng.pick(&[0u32, 1]);
            t.stub(ms::FFX_IS_DEBUG, false, 0, 0, debug);
            t.stub(ms::DEBUG_DRAW_ENV, false, 0, 0, rng.pick(&[0u32, 1]));
            for f in [
                ms::YN_BATTLE_CAPTURE_NUM,
                ms::RCBG_PRE_ADD_FP_OBJECTS,
                ms::YN_SCENE_PROC,
                ms::SCENE_DEBUG_OVERLAY,
            ] {
                t.stub(f, false, 0, 0, 0xe600 + f % 0x100);
            }
            let rec = rng.pick(&[0u32, 0x1234]);
            let objs = rng.pick(&[0u32, 1]);
            let list = t.words(&[rec, 0, objs]);
            t.g32(ms::SCENE_WAVES, list);
            t.check(
                &format!("scene step {k}"),
                0x0092_2060,
                Conv::Cdecl,
                &[],
                |m| {
                    ms::yn_scene_step(m);
                    Out::None
                },
            );
        }
    });
}

/// Machine code for a recording stub's `extra`: copies `n` words from
/// the pointer argument `arg` into the log entry from `+at`, then (when
/// `matrix` is set) the 16 words behind the word at `+matrix` of that
/// pointer into the next 64-byte slot of `dump` (counter `cnt`).
fn copy_extra(arg: u8, n: u8, at: u8, matrix: Option<(u8, u32, u32)>) -> Vec<u8> {
    copy_extra_n(arg, n, at, matrix.map(|(o, c, d)| (vec![o], c, d)))
}

/// [`copy_extra`] with several matrices (one dump slot each, in order).
fn copy_extra_n(arg: u8, n: u8, at: u8, matrices: Option<(Vec<u8>, u32, u32)>) -> Vec<u8> {
    let mut x = vec![0x56, 0x8b, 0x74, 0x24, 8 + 4 * arg];
    for i in 0..n {
        x.extend([0x8b, 0x4e, 4 * i, 0x89, 0x48, at + 4 * i]);
    }
    if let Some((offs, cnt, dump)) = matrices {
        for off in offs {
            x.extend([0x8b, 0x74, 0x24, 8 + 4 * arg]);
            x.extend([0x8b, 0x76, off, 0x8b, 0x0d]);
            x.extend(cnt.to_le_bytes());
            x.extend([0xc1, 0xe1, 0x06, 0x81, 0xc1]);
            x.extend(dump.to_le_bytes());
            x.extend([
                0x33, 0xd2, 0x8b, 0x04, 0x96, 0x89, 0x04, 0x91, 0x42, 0x83, 0xfa, 0x10, 0x72, 0xf4,
                0xff, 0x05,
            ]);
            x.extend(cnt.to_le_bytes());
        }
    }
    x.push(0x5e);
    x
}

/// The grid's pulsing node shapes against the original, the port's frame
/// at the original's aligned `ebp`; the node matrices run as the
/// originals, the eamap draw is a stub that copies the request and its
/// matrix.
#[test]
fn wp5g_grid_node_shapes_pulse_matches_the_original() {
    use ffx_prender::wp5g::bridge as ms;
    Twins::each(|t| {
        let mut rng = Rng(0x00a4_ffd0);
        let g = t.zeros(0x1_1800);
        for k in 0..32u32 {
            t.reset();
            t.g32(ms::ABMAP, g);
            let (cnt, dump) = (t.zeros(4), t.zeros(0x400));
            let x = copy_extra(1, 8, 0x18, Some((0x10, cnt, dump)));
            t.stub_ex(
                ms::SHAPE_DRAW_EAMAP,
                false,
                4,
                0,
                &m5_main3_common::Ret::Fixed(0xf1),
                &x,
            );
            let count = rng.pick(&[0i16, 1, 2, 4]);
            t.write(g + 2, &count.to_le_bytes());
            for i in 0..4u32 {
                let n = g + 0x808 + 0x28 * i;
                for w in 0..10 {
                    let v = small_float(&mut rng);
                    t.w32(n + 4 * w, v);
                }
                let e = rng.pick(&[0u16, 3, 0xffff, 9]);
                t.write(n + 6, &e.to_le_bytes());
                t.w8(n + 0x22, rng.pick(&[0u8, 2, 3, 0xfd]));
            }
            for e in 0..10u32 {
                let v = rng.next();
                t.w32(g + 0xf828 + 48 * e + 8, v);
            }
            for i in 0..16 {
                let v = small_float(&mut rng);
                t.w32(g + 0x113e0 + 4 * i, v);
            }
            t.w8(g + 0x115bc, rng.below(5) as u8);
            let v = rng.next();
            t.w32(g + 0x1169c, v);
            t.w32(g + 0x116a0, rng.pick(&[0u32, 5, 15, 16, 100]));
            let v = rng.next();
            t.w32(g + 0x116a4, v);
            let entry = t.frame_ebp(0) + 4;
            let ebp = (entry - 12) & !15;
            let lo = ebp - ms::NODE_SHAPES_FRAME;
            t.write(lo, &vec![0; ms::NODE_SHAPES_FRAME as usize]);
            t.b.set_call_top(Some(lo - 0x100));
            t.check(
                &format!("node shapes pulse {k} ({count})"),
                0x00a4_ffd0,
                Conv::Cdecl,
                &[],
                |m| Out::U32(ms::abmap_draw_node_shapes_pulse(m, ebp)),
            );
            t.b.set_call_top(None);
        }
    });
}

/// The grid's lit node spheres against the original, the port's frame
/// at the original's aligned `ebp`; the node matrices run as the
/// originals, the sphere draw is a stub that copies the request and its
/// matrix.
#[test]
fn wp5g_grid_node_spheres_lit_matches_the_original() {
    use ffx_prender::wp5g::bridge as ms;
    Twins::each(|t| {
        let mut rng = Rng(0x00a5_0b50);
        let g = t.zeros(0x1_1800);
        for k in 0..32u32 {
            t.reset();
            t.g32(ms::ABMAP, g);
            let (cnt, dump) = (t.zeros(4), t.zeros(0x4000));
            let x = copy_extra_n(1, 10, 0x18, Some((vec![0x20, 0x44], cnt, dump)));
            t.stub_ex(
                ms::DRAW_EIABM_SPHERE,
                false,
                4,
                0,
                &m5_main3_common::Ret::Fixed(0xf2),
                &x,
            );
            let v = rng.next();
            t.g32(0x0230_57ec, v);
            t.w8(g + 0x115c9, rng.pick(&[0u8, 0x7f, 0x15, 0xff]));
            let count = rng.pick(&[0i16, 1, 2, 4]);
            t.write(g + 2, &count.to_le_bytes());
            for i in 0..4u32 {
                let n = g + 0x808 + 0x28 * i;
                for w in 0..10 {
                    let v = small_float(&mut rng);
                    t.w32(n + 4 * w, v);
                }
                let e = rng.pick(&[0u16, 3, 0xffff, 9]);
                t.write(n + 6, &e.to_le_bytes());
                t.w8(n + 0x21, rng.pick(&[0u8, 0x7f, 0x42, 0xff]));
            }
            for e in 0..10u32 {
                for w in 0..12 {
                    let v = small_float(&mut rng);
                    t.w32(g + 0xf828 + 48 * e + 4 * w, v);
                }
                let mark = rng.pick(&[0u16, 0x1000]);
                t.write(g + 0xf828 + 48 * e + 0x14, &mark.to_le_bytes());
            }
            for i in 0..16 {
                let v = small_float(&mut rng);
                t.w32(g + 0x113e0 + 4 * i, v);
            }
            t.w8(g + 0x115bc, rng.below(5) as u8);
            let v = rng.next();
            t.w32(g + 0x11698, v);

            let v = rng.next();
            t.w32(g + 0x116a4, v);
            let entry = t.frame_ebp(0) + 4;
            let ebp = (entry - 12) & !15;
            let lo = ebp - ms::NODE_SPHERES_FRAME;
            t.write(lo, &vec![0; ms::NODE_SPHERES_FRAME as usize]);
            t.b.set_call_top(Some(lo - 0x100));
            t.check(
                &format!("node spheres lit {k} ({count})"),
                0x00a5_0b50,
                Conv::Cdecl,
                &[],
                |m| {
                    ms::abmap_draw_node_spheres_lit(m, ebp);
                    Out::None
                },
            );
            t.b.set_call_top(None);
        }
    });
}

#[test]
fn wp5g_grid_states_match_the_original() {
    use ffx_prender::wp5g::bridge as ms;
    Twins::each(|t| {
        let mut rng = Rng(0x00a4_5010);
        let g = t.zeros(0x1_1800);
        let menu = t.zeros(0x2800);
        let panel = t.zeros(0x40);
        for k in 0..512u32 {
            t.reset();
            t.g32(ms::ABMAP, g);
            t.g32(ms::ABMAP_MENU, menu);
            for (f, n) in [
                (ms::FREE_CURSOR_STEP, 1),
                (ms::TRY_CHANGE_PLAYER, 0),
                (ms::PAD_START_SLIDE_IN, 1),
                (ms::PAD_START_SLIDE_OUT, 1),
                (ms::SND_SEP_PLAY_SIMPLE, 1),
                (ms::SELECT_NODE, 2),
                (ms::MARK_LOCKED_ENTRIES, 1),
                (ms::INIT_MOVING_TO_TARGET, 2),
                (ms::ALL_LINKS_LIMIT_FLAG, 1),
                (ms::ENTER_COMMAND_MENU, 0),
                (ms::COPY_DEREF_A596D0, 1),
                (ms::COPY_DEREF_A59680, 1),
                (ms::RESET_HELP, 0),
                (ms::TRY_USE_ITEM, 3),
                (ms::MASK_NODE_FLAGS, 1),
                (ms::ENTER_ITEM_MENU, 0),
            ] {
                t.stub(f, false, n, 0, 0xf300 + f % 0x100);
            }
            t.stub(ms::OPEN_PANEL, false, 2, 0, panel);
            // the use-item menu's refill sets the new count
            let count = rng.pick(&[0u16, 1, 2, 3, 8, 9]);
            let mut x = vec![0xa1];
            x.extend(t.rebase(ms::ABMAP_MENU).to_le_bytes());
            x.extend([0x66, 0xc7, 0x80, 0x1e, 0x1a, 0x00, 0x00]);
            x.extend(count.to_le_bytes());
            t.stub_ex(
                ms::FILL_USE_ITEM_MENU,
                false,
                2,
                0,
                &m5_main3_common::Ret::Fixed(0xf4),
                &x,
            );
            let paging = k % 8 == 3;
            let using = k % 8 == 2;
            t.w8(
                g + 0x115cd,
                if paging { 0 } else { rng.pick(&[0u8, 0, 0, 1]) },
            );
            t.w32(
                g + 0x115b0,
                if paging {
                    0
                } else {
                    rng.pick(&[0u32, 0, 0, 5])
                },
            );
            let pad = if paging {
                0x10
            } else if using {
                0x20
            } else {
                rng.pick(&[0u16, 0x20, 0x40, 0x10, 0x30, 0x60])
            };
            t.write(g + 0x1166e, &pad.to_le_bytes());
            t.w8(g + 0x115c4, rng.pick(&[0u8, 1, 2]));
            t.w8(g + 0x115c8, rng.below(7) as u8);
            t.w8(g + 0x115bc, rng.below(7) as u8);
            for i in 0..20u32 {
                let v = rng.below(0x400) as u16;
                t.write(g + 0x82c + 40 * i, &v.to_le_bytes());
            }
            for i in 0..7u32 {
                let v = rng.next() as u16;
                t.write(g + 0x110cc + 80 * i, &v.to_le_bytes());
            }
            // (after the player nodes: the seventh's record ends at +0x112fc)
            let cur = rng.below(20) as u16;
            t.write(g + 0x112fc, &cur.to_le_bytes());
            let links = rng.pick(&[0i16, 1, 5]);
            t.write(g + 4, &links.to_le_bytes());
            for i in 0..5u32 {
                t.w8(g + 0xa816 + 0x14 * i, rng.next() as u8);
            }
            for o in [0x13b4u32, 0x374, 0x9f4, 0x6b4, 0xd34, 0x1074, 0x1a34] {
                let busy = rng.pick(&[0u32, 0, 0, 0, 0, 0, 0, 0, 1]);
                t.w32(
                    menu + o,
                    if (paging && k % 16 == 3) || using {
                        0
                    } else {
                        busy
                    },
                );
            }
            if paging && k % 32 == 11 {
                // only the last panel still moving
                for o in [0x13b4u32, 0x374, 0x9f4, 0x6b4, 0xd34] {
                    t.w32(menu + o, 0);
                }
                t.w32(menu + 0x1074, 1);
            }
            for o in [0x1a18u32, 0x1a1a, 0x1a16] {
                let v = rng.pick(&[0i16, 1, 2, 3, 8, 9, -1]);
                t.write(menu + o, &v.to_le_bytes());
            }
            // the item rows (`+0x1a00 + 12 * (row + 6)`, past the list's
            // own words), the panels' "seen" bytes as junk
            for i in 5..21u32 {
                let v = rng.next();
                t.w32(menu + 0x1a00 + 12 * i, v);
            }
            for slot in 0..8u32 {
                t.w8(menu + 0x340 * slot + 0x23, 0xff);
            }
            let v = rng.next();
            t.w32(panel + 0x24, v);
            let v = rng.next();
            t.w32(menu + 0x23e4, v);
            let states: [Step; 4] = [
                (0x00a4_4ef0, |m| ms::abmap_choose_move_target(m)),
                (0x00a4_8230, |m| ms::abmap_moving_to_target(m)),
                (0x00a4_52d0, |m| ms::abmap_item_target(m)),
                (0x00a4_5010, |m| ms::abmap_help(m)),
            ];
            let (addr, port) = states[(k % 4) as usize];
            t.check(
                &format!("{addr:#x} {k} (pad {pad:#x})"),
                addr,
                Conv::Cdecl,
                &[],
                |m| {
                    port(m);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn wp5g_screen_fade_reset_matches_the_original() {
    use ffx_prender::wp5g::bridge as ms;
    Twins::each(|t| {
        let mut rng = Rng(0x0063_7c50);
        let m = t.zeros(0x1_2400);
        for k in 0..8u32 {
            t.reset();
            t.stub(ms::POST_PROCESS_GET_INSTANCE, false, 0, 0, m);
            t.w8(m + 0x123a0, rng.pick(&[0u8, 1]));
            for i in 0..4 {
                let v = rng.next();
                t.w32(m + 0x1238c + 4 * i, v);
            }
            t.check(
                &format!("screen fade reset {k}"),
                0x0063_7c50,
                Conv::Cdecl,
                &[],
                |g| Out::U32(ms::reset_screen_fade(g)),
            );
        }
    });
}
