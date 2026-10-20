//! The battle's small bookkeeping (`ffx_battle::misc`) against the
//! original, as in `battle.rs`: random globals, actors and arguments, on
//! the image alone and on both snapshots.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::*;
use ffx_battle::misc::bridge as g;
use ffx_oracle::Oracle;

/// Functions of no arguments: name, `ffx.sym` address, result mask, the
/// Rust.
type NoArgs = (&'static str, u32, u32, fn(&mut Oracle) -> u32);

const NO_ARGS: [NoArgs; 20] = [
    ("MsComBufInit", 0x0079_d290, 0, |m| {
        g::ms_com_buf_init(m);
        0
    }),
    ("MsEtEffectReadInit", 0x0079_e730, 0, |m| {
        g::ms_et_effect_read_init(m);
        0
    }),
    ("MsNumberInit", 0x0079_f9f0, 0, |m| {
        g::ms_number_init(m);
        0
    }),
    ("MsActionInit", 0x007a_ce90, 0, |m| {
        g::ms_action_init(m);
        0
    }),
    ("MsEffectCameraReset", 0x007b_bb00, 0, |m| {
        g::ms_effect_camera_reset(m);
        0
    }),
    ("MsBattleGetFontAddress", 0x0078_1680, ALL, |m| {
        g::ms_battle_get_font_address(m)
    }),
    ("MsRegMotionInit", 0x0079_ccd0, 0, |m| {
        g::ms_reg_motion_init(m);
        0
    }),
    ("MsBtlStartAP", 0x0079_8e90, 0, |m| {
        g::ms_btl_start_ap(m);
        0
    }),
    ("MsBtlInitItemMem", 0x0079_a000, 0, |m| {
        g::ms_btl_init_item_mem(m);
        0
    }),
    ("MsBtlInitItemMem_007a0cd0", 0x007a_0cd0, 0, |m| {
        g::ms_btl_init_item_mem2(m);
        0
    }),
    ("MsMessageCueInit", 0x007a_2730, 0, |m| {
        g::ms_message_cue_init(m);
        0
    }),
    ("MsSplineInit", 0x007b_aaa0, 0, |m| {
        g::ms_spline_init(m);
        0
    }),
    ("MsBattleSpecialCameraFree", 0x0079_dd90, 0, |m| {
        g::ms_battle_special_camera_free(m);
        0
    }),
    ("MsBtlReadStatusComplete", 0x007a_d550, 0, |m| {
        g::ms_btl_read_status_complete(m);
        0
    }),
    ("MsScreenFocusReset", 0x007b_bd80, 0, |m| {
        g::ms_screen_focus_reset(m);
        0
    }),
    ("MsBtlListInit", 0x0079_d250, 0, |m| {
        g::ms_btl_list_init(m);
        0
    }),
    ("MsBtlReadInit", 0x0078_3000, 0, |m| {
        g::ms_btl_read_init(m);
        0
    }),
    ("MsMoveTargetPos", 0x007a_b160, ALL, |m| {
        g::ms_move_target_pos(m)
    }),
    ("FinishReadPhyreBattleMap", 0x0078_2fd0, 0, |m| {
        g::finish_read_phyre_battle_map(m);
        0
    }),
    ("MsDebugCameraSleepInit", 0x0079_2200, 0, |m| {
        g::ms_debug_camera_sleep_init(m);
        0
    }),
];

/// Functions of one actor (or pointer).
type OnActor = (&'static str, u32, u32, fn(&mut Oracle, u32) -> u32);

const ON_ACTOR: [OnActor; 7] = [
    ("MsResetDirMove", 0x007a_b170, 0, |m, p| {
        g::ms_reset_dir_move(m, p);
        0
    }),
    ("MsPaymentProcess", 0x007a_fa60, ALL, |m, p| {
        g::ms_payment_process(m, p)
    }),
    ("MsMotionSpeedSetNormal", 0x007a_a260, 0, |m, p| {
        g::ms_motion_speed_set_normal(m, p);
        0
    }),
    ("msMoveReset", 0x007a_a280, 0, |m, p| {
        g::ms_move_reset(m, p);
        0
    }),
    ("MsSetMenuCommand", 0x0079_2e50, 0, |m, p| {
        g::ms_set_menu_command(m, p);
        0
    }),
    ("MsSummonLimitSave", 0x007b_06e0, 0, |m, p| {
        g::ms_summon_limit_save(m, p);
        0
    }),
    ("MsSummonLimitLoad", 0x007b_06b0, 0, |m, p| {
        g::ms_summon_limit_load(m, p);
        0
    }),
];

fn randomize(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    area: u32,
    screen: u32,
    list: u32,
) {
    for (gl, len) in [
        (g::COM_BUF.ptr(b).addr, 4),
        (g::START_AP.ptr(b).addr, 0x90),
        (g::SPECIAL_CAMERA.ptr(b).addr, 3),
        (g::READ_CUE.ptr(b).addr, 12 * 0x100),
        (g::READ_CUE_INDEX.ptr(b).addr, 2),
        (g::SUMMON_LIMIT.ptr(b).addr, 1),
        (ffx_battle::state::bridge::STATUS.ptr(b).addr, 1),
    ] {
        fill(a, b, rng, gl, len);
    }
    fill(a, b, rng, area, 0x1000);
    set(a, b, g::SCREEN, screen);
    fill(a, b, rng, list, 0x10);
    poke(a, b, list + 4, &rng.below(0x1000).to_le_bytes());
    poke(a, b, list + 8, &rng.word().to_le_bytes());
    set(a, b, g::FIELD_LIST, list);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let area = scratch(a, b, 0x1000);
    let screen = scratch(a, b, 0x40);
    let list = scratch(a, b, 0x10);
    for _ in 0..500 {
        randomize(a, b, rng, area, screen, list);
        for (name, sym, mask, rust) in NO_ARGS {
            twin(name, a, b, sym, &[], mask, rust);
        }
        for (name, sym, mask, rust) in ON_ACTOR {
            let p = area + rng.below(0x400);
            twin(name, a, b, sym, &[p], mask, |m| rust(m, p));
        }
        let x = rng.next();
        twin("MsSaveUcID", a, b, 0x007d_42e0, &[x], 0, |m| {
            g::ms_save_uc_id(
                m,
                x.to_le_bytes()[..2]
                    .try_into()
                    .map_or(0, u16::from_le_bytes),
            );
            0
        });
        let (w, h) = (rng.next(), rng.next());
        twin("MsScreenInit", a, b, 0x007c_0540, &[w, h], 0, |m| {
            g::ms_screen_init(m, w, h);
            0
        });
        let p = if rng.below(3) == 0 { 0 } else { area + 0x800 };
        twin(
            "MsEffectGetBufferAddress",
            a,
            b,
            0x007b_7eb0,
            &[p],
            ALL,
            |m| g::ms_effect_get_buffer_address(m, p),
        );
        let id = rng.next();
        let id = if rng.below(2) == 0 {
            (id & !0xff00) | 0x200
        } else {
            id
        };
        twin(
            "MsEffectGetLastMagicNum2",
            a,
            b,
            0x0078_7bf0,
            &[id],
            ALL,
            |m| g::ms_effect_get_last_magic_num2(m, id),
        );
        let c = rng.next();
        twin(
            "MsBattleSpecialCameraPause",
            a,
            b,
            0x0079_ddb0,
            &[c],
            0,
            |m| {
                g::ms_battle_special_camera_pause(m, c.to_le_bytes()[0]);
                0
            },
        );
        let (fx, fy) = (quiet(rng.next()), quiet(rng.next()));
        twin("MsScreenOffset", a, b, 0x007b_c0c0, &[fx, fy], 0, |m| {
            g::ms_screen_offset(m, fx, fy);
            0
        });
        twin(
            "MsScreenResetOffset",
            a,
            b,
            0x007b_c720,
            &[fy, fx],
            0,
            |m| {
                g::ms_screen_reset_offset(m, fy, fx);
                0
            },
        );
        let i = rng.range(-1, 5).cast_unsigned();
        twin("MsMoveSplineSet", a, b, 0x007a_b140, &[i], 0, |m| {
            g::ms_move_spline_set(m, i);
            0
        });
        let i = rng.range(-1, 18).cast_unsigned();
        twin(
            "MsEffectSetCameraBufFloat",
            a,
            b,
            0x007b_7f50,
            &[i, fx],
            0,
            |m| {
                g::ms_effect_set_camera_buf_float(m, i, fx);
                0
            },
        );
        same("misc", a, b);
    }
}

#[test]
fn misc_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x1029_3847));
}

#[test]
fn misc_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x5647_3829));
}

/// A float for the leaves' x87 copies: specials (no signalling NaNs: the
/// emulator keeps them), quiet randoms, plain values.
fn leaf_float(rng: &mut Xorshift) -> u32 {
    match rng.below(6) {
        0 => [
            0,
            0x8000_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_0000,
            1,
            0x7fc1_2345,
        ][rng.below(7) as usize],
        1 => quiet(rng.next()),
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 37.0).to_bits(),
    }
}

/// Round 29's leaves (`ffx_battle::misc`, `ffx_battle::camera`) and the
/// two copies of vfx ports, on random globals and arguments.
fn leaves(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_battle::camera::bridge as cam;
    use ffx_oracle::Conv;
    let area = scratch(a, b, 0x400);
    let rec = scratch(a, b, 0x40);
    // TOGetFFXLang's thunk and sceWrite as recording stubs: the language
    // and the write's result are the test's
    for o in [&mut *a, &mut *b] {
        recording_stub(o, 0x008a_c2f0, STUBS + 0x100, rec, 0, 0);
        recording_stub(o, 0x0062_fdc0, STUBS + 0x140, rec + 0x10, 3, 0);
    }
    let menu_text = ffx_battle::flow::bridge::MENU_TEXT.ptr(b).addr;
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    let cameras = cam::CAMERAS.ptr(b).addr;
    for round in 0..400 {
        for (gl, len) in [
            (g::GLOBAL_00C44248.ptr(b).addr, 4),
            (g::SCREEN_ASPECT_Z.ptr(b).addr, 4),
            (g::REEL_ENTRIES.ptr(b).addr, 16),
            (g::EFFECT_CAMERA_WORDS.ptr(b).addr, 0x48),
            (ffx_battle::action::bridge::SET_COMMAND_FLAG.ptr(b).addr, 1),
            (ffx_battle::process::bridge::ENCOUNT_FADE.ptr(b).addr, 1),
            (ffx_battle::process::bridge::RESET_WAIT2.ptr(b).addr, 1),
            (menu_text, 12 * 18),
            (ply, 0x94 * 19),
            (cam::SCREENS.ptr(b).addr, cam::SCREEN_SIZE * 3 + 4),
        ] {
            fill(a, b, rng, gl, len);
        }
        for k in 0..16 {
            let f = leaf_float(rng);
            poke(
                a,
                b,
                g::EFFECT_CAMERA_FLOATS.ptr(b).addr + 4 * k,
                &f.to_le_bytes(),
            );
        }
        fill(a, b, rng, area, 0x400);
        let w = rng.word();
        twin("GetGlobal00c44248", a, b, 0x007c_6db0, &[], ALL, |m| {
            g::get_global_00c44248(m)
        });
        twin("MsMenuSetWindowSystem", a, b, 0x0079_01f0, &[w], 0, |m| {
            g::ms_menu_set_window_system(m, w);
            0
        });
        let w = rng.word();
        twin("MsBattleEncountFlag", a, b, 0x0078_0fe0, &[w], 0, |m| {
            g::ms_battle_encount_flag(m, w);
            0
        });
        let w = rng.next();
        twin("MsEffectWaitFlag", a, b, 0x0078_8b20, &[w], 0, |m| {
            g::ms_effect_wait_flag(m, w);
            0
        });
        let f = leaf_float(rng);
        twin("setScreenDataAspectZ", a, b, 0x007c_0e60, &[f], 0, |m| {
            g::set_screen_data_aspect_z(m, f);
            0
        });
        // the camera in use: one of the three, its roll a test float
        let n = rng.below(3);
        let camera = cameras + n * cam::CAMERA_SIZE;
        set(a, b, cam::CAMERA, camera);
        poke(a, b, camera + 0xc90, &leaf_float(rng).to_le_bytes());
        ftwin("MsCameraGetRollNow", a, b, 0x007b_d500, &[], |m| {
            g::ms_camera_get_roll_now(m)
        });
        let n = [rng.range(-3, 10).cast_unsigned(), rng.word()][rng.below(2) as usize];
        twin("Clamp0To6", a, b, 0x007a_86e0, &[n], ALL, |_| {
            g::clamp0_to6(n)
        });
        if round % 8 == 0 {
            twin("MsDebugSetAllSP", a, b, 0x0078_4ae0, &[], ALL, |m| {
                g::ms_debug_set_all_sp(m)
            });
        }
        let i = [rng.range(-2, 6).cast_unsigned(), rng.word()][rng.below(2) as usize];
        twin("MsGetReelEntry", a, b, 0x007b_19d0, &[i], ALL, |m| {
            g::ms_get_reel_entry(m, i)
        });
        // the vectors: in the area, the output sometimes the input
        for k in 0..8 {
            poke(a, b, area + 4 * k, &leaf_float(rng).to_le_bytes());
            poke(
                a,
                b,
                ffx_battle::vector::bridge::VF_ACC.ptr(b).addr + 4 * (k & 3),
                &leaf_float(rng).to_le_bytes(),
            );
        }
        let (v, out) = (area, [area + 0x20, area][rng.below(2) as usize]);
        let s: [u32; 4] = std::array::from_fn(|_| leaf_float(rng));
        twin(
            "Vec4Scale4",
            a,
            b,
            0x007c_11c0,
            &[out, v, s[0], s[1], s[2], s[3]],
            ALL,
            |m| g::vec4_scale4(m, out, v, s),
        );
        let (u1, u2) = (rng.next(), rng.next());
        twin(
            "Vu0MaddaScalar4",
            a,
            b,
            0x007c_1000,
            &[out, v, u1, u2, s[0]],
            ALL,
            |m| g::vu0_madda_scalar4(m, out, v, s[0]),
        );
        let i = [rng.range(-2, 18).cast_unsigned(), rng.word()][rng.below(2) as usize];
        twin("MsEffectGetCameraBuf", a, b, 0x007b_7ed0, &[i], ALL, |m| {
            g::ms_effect_get_camera_buf(m, i)
        });
        ftwin("MsEffectGetCameraBufFloat", a, b, 0x007b_7ef0, &[i], |m| {
            g::ms_effect_get_camera_buf_float(m, i)
        });
        let v = rng.next();
        twin("MsEffectSetCameraBuf", a, b, 0x007b_7f30, &[i, v], 0, |m| {
            g::ms_effect_set_camera_buf(m, i, v);
            0
        });
        let k = [rng.range(-1, 0x13).cast_unsigned(), rng.word()][rng.below(2) as usize];
        poke(a, b, area + 0x100 + 0xc, &k.to_le_bytes());
        twin(
            "MsMenuReadTextDone",
            a,
            b,
            0x0078_fed0,
            &[area + 0x100],
            0,
            |m| {
                g::ms_menu_read_text_done(m, area + 0x100);
                0
            },
        );
        // the language: Japanese (0) often, English, any
        let lang = [0, 1, rng.below(12), rng.word()][rng.below(4) as usize];
        poke(a, b, rec + 4, &lang.to_le_bytes());
        let get_lang = |m: &mut Oracle| {
            let f = m.rebase(0x008a_c2f0);
            m.call(f, Conv::Cdecl, &[]).unwrap().eax
        };
        twin("isNeedShowJapanLogo", a, b, 0x0078_7390, &[], ALL, |m| {
            g::is_need_show_japan_logo(get_lang(m))
        });
        let was = [lang, lang ^ 0x100, lang & 0xff | 0x80, rng.next()][rng.below(4) as usize];
        twin("isNeedRenamePlayer", a, b, 0x0078_7370, &[was], ALL, |m| {
            let lang = get_lang(m);
            g::is_need_rename_player(was, lang)
        });
        // a name: letters of both cases, high bytes, empty
        let len = rng.below(24);
        let name: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'A' + u8::try_from(rng.below(26)).unwrap(),
                1 => 0x40 + u8::try_from(rng.below(0x40)).unwrap(),
                _ => u8::try_from(1 + rng.below(255)).unwrap(),
            })
            .chain([0])
            .collect();
        poke(a, b, area + 0x200, &name);
        twin("NameHash", a, b, 0x007c_f000, &[area + 0x200], ALL, |m| {
            g::name_hash(m, area + 0x200)
        });
        let (screen, c) = (
            [rng.range(-1, 4).cast_unsigned(), rng.word()][rng.below(2) as usize],
            rng.next(),
        );
        twin(
            "Cam_SetScreenCameraNo",
            a,
            b,
            0x007b_c870,
            &[screen, c],
            0,
            |m| {
                cam::cam_set_screen_camera_no(m, screen, c);
                0
            },
        );
        // a camera side's point: one of the three cameras, the outputs
        // in the area or null
        let (n, side) = (rng.below(3), rng.next());
        let pos = cameras + n * cam::CAMERA_SIZE + (side & 1) * 0x61c + 0x40;
        for k in 0..3 {
            poke(a, b, pos + 4 * k, &leaf_float(rng).to_le_bytes());
        }
        let outs = [0x300u32, 0x304, 0x308].map(|o| if rng.below(4) == 0 { 0 } else { area + o });
        let cam_arg = n | rng.next() << 8;
        twin(
            "Cam_GetRefPos",
            a,
            b,
            0x007b_d4b0,
            &[cam_arg, side, outs[0], outs[1], outs[2]],
            ALL,
            |m| cam::cam_get_ref_pos(m, cam_arg, side, outs),
        );
        // the vfx copies, each against its own original
        let (dst, src) = (area + 0x340, [area, area + 0x340][rng.below(2) as usize]);
        // (no signalling NaNs in the source: the emulator keeps them)
        for k in 0..4 {
            poke(a, b, dst + 4 * k, &leaf_float(rng).to_le_bytes());
        }
        twin(
            "pppVec4Copy_007c1110",
            a,
            b,
            0x007c_1110,
            &[dst, src],
            0,
            |m| {
                ffx_vfx::op::bridge::ppp_vec4_copy(m, dst, src);
                0
            },
        );
        // op_file_puts: the buffer full, partly full, over-full
        let (file, data, text) = (area + 0x380, area + 0x240, area + 0x3a0);
        let cap = [1u32, 3, 8, 0x40][rng.below(4) as usize];
        let pos = rng.below(cap + 1) + [0, 0, 1][rng.below(3) as usize];
        for (o, v) in [(0, data), (8, rng.next()), (0xc, pos), (0x10, cap)] {
            poke(a, b, file + o, &u32::to_le_bytes(v));
        }
        let len = [0u32, 1, 2, 7, 0x1f][rng.below(5) as usize];
        let s: Vec<u8> = (0..len)
            .map(|_| u8::try_from(1 + rng.below(255)).unwrap())
            .chain([0])
            .collect();
        poke(a, b, text, &s);
        poke(a, b, rec + 0x20, &rng.next().to_le_bytes());
        let mask = if len == 0 { 0 } else { ALL };
        twin(
            "op_file_puts_007cf860",
            a,
            b,
            0x007c_f860,
            &[file, text],
            mask,
            |m| {
                ffx_vfx::op::bridge::op_file_puts(m, file, text, &mut |m, f, args| {
                    m.call(f, Conv::Cdecl, args).unwrap().eax
                })
                .unwrap_or(0)
            },
        );
        same("leaves", a, b);
    }
}

#[test]
fn leaves_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    leaves(&mut a, &mut b, &mut Xorshift(0x2929_0001));
}

#[test]
fn leaves_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    leaves(&mut a, &mut b, &mut Xorshift(0x2929_0002));
}
