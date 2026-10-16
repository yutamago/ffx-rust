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
