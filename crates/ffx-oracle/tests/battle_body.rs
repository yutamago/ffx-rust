//! Battle actors' models (`ffx_battle::body`), the menu lookups
//! (`ffx_battle::menu`), the camera's requests by ID and matrix copies, and
//! `MsSubPoint` against the original, as in `battle.rs`: actors with fake
//! characters on the image alone, the live ones on a snapshot in battle.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACTORS, World, fake_records, quiet, world};
use battle_common::*;
use ffx_battle::body::bridge as body;
use ffx_battle::camera::bridge as cam;
use ffx_battle::menu::bridge as menu;
use ffx_oracle::Oracle;

/// A float: small, zero, infinite or a quiet NaN.
fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(8) {
        0 => [0, 0x8000_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000][rng.below(5) as usize],
        1 => quiet(rng.next()),
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 97.0).to_bits(),
    }
}

/// Floats (no signalling NaNs) over `len` bytes at `at`.
fn floats(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, at: u32, len: u32) {
    let v: Vec<u8> = (0..len / 4)
        .flat_map(|_| float(rng).to_le_bytes())
        .collect();
    poke(a, b, at, &v);
}

/// On the synthetic image: fake characters for some actors, random actor
/// fields the functions read.
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, chars: u32) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        let c = if rng.below(4) == 0 {
            0
        } else {
            chars + 0x400 * (i % 8)
        };
        poke(a, b, p, &c.to_le_bytes());
        fill(a, b, rng, p + 0x2d, 8);
        floats(a, b, rng, p + 0x3a0, 0x10);
        floats(a, b, rng, p + 0x4dc, 4);
        for (at, len) in [
            (0x3f2, 2),
            (0x412, 0x30),
            (0x5d0, 4),
            (0x6c8, 2),
            (0xdc8, 0x10),
            (0xdf2, 1),
            (0xf5e, 1),
        ] {
            fill(a, b, rng, p + at, len);
        }
        // followers seldom, owned by an earlier actor (no cycles)
        let owner = if i == 0 || rng.below(2) == 0 {
            0xff
        } else {
            rng.below(i)
        };
        poke(
            a,
            b,
            p + 0x43f,
            &[u8::try_from(owner).unwrap(), u8::from(rng.below(6) == 0)],
        );
        poke(a, b, p + 0x3f3, &[u8::from(rng.below(2) == 0)]);
    }
    // no infinities in characters: Ch_GetRot's wrap never ends on them,
    let v: Vec<u8> = (0..0x400 * 2)
        .flat_map(|_| {
            let f = float(rng);
            // nor values too large for a step of 2π to change
            if f & 0x7f80_0000 >= 0x4b00_0000 && f & 0x7fff_ffff <= 0x7f80_0000 {
                0
            } else {
                f
            }
            .to_le_bytes()
        })
        .collect();
    poke(a, b, chars, &v);
    set(
        a,
        b,
        ffx_chara::chr::bridge::SKIP_SET_POS3,
        u8::from(rng.below(4) == 0),
    );
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, out: u32) {
    let i = if rng.below(6) == 0 {
        rng.next()
    } else {
        rng.below(0x22)
    };
    for (name, sym, rust) in [
        (
            "MsGetChrHeight",
            0x0079_50f0,
            body::ms_get_chr_height as fn(&Oracle, u32) -> f32,
        ),
        (
            "MsGetChrCamHeight",
            0x0079_4a30,
            body::ms_get_chr_cam_height,
        ),
        ("MsGetChrCamWidth", 0x0079_4a70, body::ms_get_chr_cam_width),
        ("MsGetChrGround", 0x0079_4fb0, body::ms_get_chr_ground),
    ] {
        // the scaled ones read any index; keep them to actors
        let i = if sym == 0x0079_4fb0 { i } else { i % 0x1f };
        let ra = a
            .call(a.rebase(sym), ffx_oracle::Conv::Cdecl, &[i])
            .unwrap();
        let ra = ra.st0.expect("a float");
        let rb = f64::from(rust(b, i));
        assert!(
            ra.to_bits() == rb.to_bits() || (ra.is_nan() && rb.is_nan()),
            "{name} {i:#x}: {ra} {rb}"
        );
    }
    twin("MsGetChrDir", a, b, 0x0079_4d20, &[i, out], ALL, |m| {
        body::ms_get_chr_dir(m, i, out).cast_unsigned()
    });
    floats(a, b, rng, out + 0x10, 0x10);
    twin(
        "MsSetChrPos",
        a,
        b,
        0x0079_6150,
        &[i, out + 0x10],
        ALL,
        |m| body::ms_set_chr_pos(m, i, out + 0x10).cast_unsigned(),
    );
    let how = rng.range(-2, 3);
    twin(
        "MsSetChrWeight",
        a,
        b,
        0x0079_68e0,
        &[i, how.cast_unsigned()],
        0,
        |m| {
            body::ms_set_chr_weight(m, i, how);
            0
        },
    );
    let k = i % 0x1f;
    let actor = ffx_battle::actor::bridge::ms_get_chr(b, k);
    let wv = float(rng);
    twin("msSetChrWeight", a, b, 0x0079_72c0, &[actor, wv], 0, |m| {
        body::ms_set_chr_weight2(m, actor, wv);
        0
    });
    let (part, on) = (rng.range(-1, 6), rng.range(-1, 2));
    twin(
        "msSetChrBodyHit",
        a,
        b,
        0x0079_6f20,
        &[actor, part.cast_unsigned(), on.cast_unsigned()],
        ALL,
        |m| body::ms_set_chr_body_hit(m, actor, part, on),
    );
    let v = rng.next();
    twin("MsSetChrMoveFlag", a, b, 0x0079_6110, &[i, v], ALL, |m| {
        body::ms_set_chr_move_flag(m, i, v.to_le_bytes()[0]).cast_unsigned()
    });
    twin(
        "MsSetChrMotionLevel",
        a,
        b,
        0x0079_60d0,
        &[i, v],
        ALL,
        |m| body::ms_set_chr_motion_level(m, i, v.to_le_bytes()[0]).cast_unsigned(),
    );
    let (fv, frames) = (float(rng), rng.range(-1, 40));
    twin(
        "MsSetChrSpecular",
        a,
        b,
        0x0079_6360,
        &[k, fv, frames.cast_unsigned()],
        ALL,
        |m| body::ms_set_chr_specular(m, k, fv, frames),
    );
    let mode = rng.range(0, 3);
    twin(
        "MsCheckChrMotionAll",
        a,
        b,
        0x0079_1630,
        &[mode.cast_unsigned()],
        ALL,
        |m| body::ms_check_chr_motion_all(m, mode),
    );
    let (sel, all, any) = (
        if rng.below(2) == 0 {
            rng.below(0x40)
        } else {
            (rng.range(-26, 0)).cast_unsigned()
        },
        rng.below(2),
        rng.below(2),
    );
    twin(
        "Battle_FirstSelectedActor",
        a,
        b,
        0x0079_47f0,
        &[k, sel, all, any],
        ALL,
        |m| {
            body::battle_first_selected_actor(m, k, sel, all.cast_signed(), any.cast_signed())
                .cast_unsigned()
        },
    );
    twin(
        "MsEffectResetMotionDisable",
        a,
        b,
        0x0078_8270,
        &[actor],
        ALL,
        |m| ffx_battle::minor::bridge::ms_effect_reset_motion_disable(m, actor),
    );
    // the menu
    let chr = rng.range(-1, 20);
    twin(
        "MsGetNextAP",
        a,
        b,
        0x0078_4e90,
        &[chr.cast_unsigned()],
        ALL,
        |m| menu::ms_get_next_ap(m, chr).cast_unsigned(),
    );
    twin("MsGetSaveChrName", a, b, 0x0078_4fb0, &[i], ALL, |m| {
        menu::ms_get_save_chr_name(m, i)
    });
    let s = if rng.below(2) == 0 { 0 } else { out + 0x20 };
    let id = rng.below(0x40);
    twin("MsMenuStatusNum", a, b, 0x0079_0200, &[id, s], ALL, |m| {
        menu::ms_menu_status_num(m, id.cast_signed(), s)
    });
    let (wid, wk, second) = (
        rng.next(),
        rng.below(8) | (rng.next() & !0xff),
        rng.below(2),
    );
    let icon = if rng.below(2) == 0 { 0 } else { out + 0x24 };
    twin(
        "MsWeaponName",
        a,
        b,
        0x007a_0c50,
        &[wid & 0x803f, wk, second, icon],
        ALL,
        |m| menu::ms_weapon_name(m, wid & 0x803f, wk, second, icon),
    );
    twin(
        "MsMenuGetChrSkipCommand",
        a,
        b,
        0x0078_fb80,
        &[k],
        ALL,
        |m| menu::ms_menu_get_chr_skip_command(m, k),
    );
    twin("MsGetCheckEnglish", a, b, 0x0078_4e60, &[], ALL, |m| {
        menu::ms_get_check_english(ffx_menu::tk::bridge::tk_get_lang(m)).cast_unsigned()
    });
    twin("MsGetBattleEndStatus", a, b, 0x0079_28d0, &[], ALL, |m| {
        menu::ms_get_battle_end_status(m)
    });
    // the camera by request
    let id = if rng.below(2) == 0 {
        let at = cam::SLOTS_AT.ptr(b).addr + cam::CAMERA_SIZE * rng.below(3) + 8 * rng.below(4);
        u32::from_le_bytes(b.read(at, 4).unwrap().try_into().unwrap())
    } else {
        rng.next() & 0x00ff_0f02
    };
    let id = (id & !0xff00) | (rng.below(6) << 8);
    let fv = float(rng);
    twin("MsCameraSetRoll", a, b, 0x007b_fa80, &[id, fv], 0, |m| {
        cam::ms_camera_set_roll(m, id, fv);
        0
    });
    twin("MsCameraSetScrDpt", a, b, 0x007b_fbd0, &[id, fv], 0, |m| {
        cam::ms_camera_set_scr_dpt(m, id, fv);
        0
    });
    let side = rng.range(0, 2);
    twin(
        "MsDebugGetCameraMode",
        a,
        b,
        0x007c_fb20,
        &[side.cast_unsigned()],
        ALL,
        |m| cam::ms_debug_get_camera_mode(m, side),
    );
    twin(
        "MsCopyCameraMatrix",
        a,
        b,
        0x007c_0070,
        &[out + 0x40],
        ALL,
        |m| cam::ms_copy_camera_matrix(m, out + 0x40),
    );
    twin(
        "MsCopyScreenMatrix",
        a,
        b,
        0x007b_ba50,
        &[out + 0x80],
        ALL,
        |m| cam::ms_copy_screen_matrix(m, out + 0x80),
    );
    floats(a, b, rng, out + 0xc0, 0x20);
    twin(
        "MsSubPoint",
        a,
        b,
        0x007c_40b0,
        &[out + 0xe0, out + 0xc0, out + 0xd0],
        0,
        |m| {
            ffx_battle::vector::bridge::ms_sub_point(m, out + 0xe0, out + 0xc0, out + 0xd0);
            0
        },
    );
    same("body", a, b);
}

type One = (&'static str, u32, fn(&mut Oracle, u32) -> u32);
type None0 = (&'static str, u32, u32, fn(&mut Oracle) -> u32);

/// The actor scans and getters of `ffx_battle::minor`.
fn scans(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, v: u32) {
    use ffx_battle::minor::bridge as g;
    let i = if rng.below(6) == 0 {
        rng.next()
    } else {
        rng.below(0x22)
    };
    let k = i % 0x1f;
    let ones: [One; 14] = [
        ("MsMenuCountStatus", 0x0078_fa80, |m, i| {
            g::ms_menu_count_status(m, i)
        }),
        ("MsMenuDefCountStatus", 0x0078_faa0, |m, i| {
            g::ms_menu_def_count_status(m, i)
        }),
        ("MsMenuHalfElement", 0x0078_fe40, |m, i| {
            g::ms_menu_half_element(m, i)
        }),
        ("MsMenuBitStatus", 0x0078_f950, |m, i| {
            g::ms_menu_bit_status(m, i)
        }),
        ("MsGetSummonerNum", 0x0078_8e10, |m, i| {
            g::ms_get_summoner_num(m, i)
        }),
        ("MsMenuWeakElement", 0x007a_9a20, |m, i| {
            g::ms_menu_weak_element(m, i)
        }),
        ("MsMenuCheckChrLive", 0x0078_f970, |m, i| {
            g::ms_menu_check_chr_live(m, i)
        }),
        ("MsGetSummonEnergy", 0x007b_2b00, |m, i| {
            g::ms_get_summon_energy(m, i)
        }),
        ("MsEffectGetTargetFlag", 0x0078_7c10, |m, i| {
            g::ms_effect_get_target_flag(m, i).cast_unsigned()
        }),
        ("MsSetMotionAttackWait", 0x007a_b500, |m, i| {
            g::ms_set_motion_attack_wait(m, i)
        }),
        ("MsSetMotionReturn", 0x007a_b520, |m, i| {
            g::ms_set_motion_return(m, i)
        }),
        ("MsGetStatusThreat", 0x0079_afc0, |m, i| {
            g::ms_get_status_threat(m, i)
        }),
        ("MsEffectMotionLevel", 0x0078_7ca0, |m, i| {
            g::ms_effect_motion_level(m, i).cast_unsigned()
        }),
        ("MsProvOff", 0x0078_d4e0, |m, i| {
            g::ms_prov_off(m, i);
            0
        }),
    ];
    for (name, sym, rust) in ones {
        let x = match name {
            "MsEffectMotionLevel" => {
                [2, 4, 7, 0x80, 3, 0][rng.below(6) as usize] << 8 | k | (rng.next() & 0xffff_0000)
            }
            "MsSetMotionAttackWait"
            | "MsSetMotionReturn"
            | "MsGetSummonEnergy"
            | "MsGetStatusThreat" => k,
            "MsProvOff" => {
                if rng.below(4) == 0 {
                    rng.next()
                } else {
                    rng.below(0x20)
                }
            }
            _ => i,
        };
        let mask = if name == "MsProvOff" { 0 } else { ALL };
        twin(name, a, b, sym, &[x], mask, |m| rust(m, x));
    }
    let actor = ffx_battle::actor::bridge::ms_get_chr(b, k);
    twin("MsDefensePosOff", a, b, 0x007a_f880, &[actor], ALL, |m| {
        g::ms_defense_pos_off(m, actor)
    });
    let none: [None0; 13] = [
        ("MsPushBtlPosAll", 0x007a_c680, 0, |m| {
            g::ms_push_btl_pos_all(m);
            0
        }),
        ("MsInitParamData", 0x0078_5960, 0, |m| {
            g::ms_init_param_data(m);
            0
        }),
        ("MsInitMimic", 0x0079_9ab0, 0, |m| {
            g::ms_init_mimic(m);
            0
        }),
        ("MsInitChrAbilityMap", 0x0079_8820, 0, |m| {
            g::ms_init_chr_ability_map(m);
            0
        }),
        ("MsBattleMemInit", 0x0078_1d00, 0, |m| {
            g::ms_battle_mem_init(m);
            0
        }),
        ("MsCheckChrHitMotionAll", 0x007a_f2b0, ALL, |m| {
            g::ms_check_chr_hit_motion_all(m).cast_unsigned()
        }),
        ("MsSetPartyRelifeTime", 0x0078_e0a0, 0, |m| {
            g::ms_set_party_relife_time(m);
            0
        }),
        ("MsCheckPlyBtlPosAll", 0x0079_2170, ALL, |m| {
            g::ms_check_ply_btl_pos_all(m)
        }),
        ("MsCommandStartChrStatusAll", 0x0079_3ef0, 0, |m| {
            g::ms_command_start_chr_status_all(m);
            0
        }),
        ("MsEffectCheck", 0x0078_7810, ALL, |m| g::ms_effect_check(m)),
        ("MsMenuStatusNumSize", 0x0079_0220, ALL, |m| {
            g::ms_menu_status_num_size(m).cast_unsigned()
        }),
        ("MsSaveEncountCheck", 0x0078_5b10, ALL, |m| {
            g::ms_save_encount_check(m)
        }),
        ("MsSetSaveConfigEnglish", 0x0078_5ed0, 0, |m| {
            let lang = ffx_menu::tk::bridge::tk_get_lang(m);
            g::ms_set_save_config_english(m, 1, lang);
            0
        }),
    ];
    for (name, sym, mask, rust) in none {
        // MsBattleMemInit clears the kernel tables' pointers too: last
        if name == "MsBattleMemInit" {
            continue;
        }
        let args: &[u32] = if name == "MsSetSaveConfigEnglish" {
            &[1]
        } else {
            &[]
        };
        twin(name, a, b, sym, args, mask, rust);
    }
    let actor = ffx_battle::actor::bridge::ms_get_chr(b, k);
    twin(
        "Battle_ChrMoveFlags",
        a,
        b,
        0x007b_4a70,
        &[actor],
        ALL,
        |m| body::battle_chr_move_flags(m, actor),
    );
    // MsPointLen(2D): _CIsqrt patched to fsqrt in both (see battle_vector.rs)
    floats(a, b, rng, v, 0x20);
    for (name, sym, rust) in [
        (
            "MsPointLen",
            0x007c_2ec0,
            ffx_battle::vector::bridge::ms_point_len as fn(&mut Oracle, u32, u32) -> f32,
        ),
        (
            "MsPointLen2D",
            0x007c_3030,
            ffx_battle::vector::bridge::ms_point_len_2d,
        ),
    ] {
        let ra = a
            .call(a.rebase(sym), ffx_oracle::Conv::Cdecl, &[v, v + 0x10])
            .unwrap()
            .st0
            .expect("a float");
        let rb = f64::from(rust(b, v, v + 0x10));
        assert!(
            ra.to_bits() == rb.to_bits() || (ra.is_nan() && rb.is_nan()),
            "{name}: {ra} {rb}"
        );
    }
    let ra = a
        .call(a.rebase(0x0079_8930), ffx_oracle::Conv::Cdecl, &[])
        .unwrap()
        .st0
        .expect("a float");
    let rb = g::frnd(b);
    assert_eq!(ra.to_bits(), f64::from(rb).to_bits(), "frnd");
    let id = rng.below(0x40);
    twin(
        "MsImportantName",
        a,
        b,
        0x0079_0860,
        &[id | (rng.next() & 0xf000)],
        ALL,
        |m| g::ms_important_name(m, id),
    );
}

fn globals(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for (g, len) in [
        (menu::SKIP_ALL.ptr(b).addr, 1),
        (menu::SKIP_ON.ptr(b).addr, 1),
        (menu::END_ALT.ptr(b).addr, 1),
        (menu::END_FORCED.ptr(b).addr, 1),
        (cam::DEBUG_CAMERA.ptr(b).addr, 1),
        (body::MOTION_BUSY.ptr(b).addr, 4),
        (ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr + 0x3b, 2),
    ] {
        fill(a, b, rng, g, len);
    }
    set(a, b, menu::END_ALT, u8::from(rng.below(2) == 0));
    set(
        a,
        b,
        menu::END_FORCED,
        if rng.below(4) == 0 { rng.byte() } else { 0 },
    );
    set(a, b, cam::DEBUG_CAMERA, u8::try_from(rng.below(3)).unwrap());
    set(
        a,
        b,
        ffx_menu::tk::bridge::LANG,
        rng.range(-1, 4).to_le_bytes()[0].cast_signed(),
    );
    for c in 0..18 {
        let at = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr + 0x94 * c + 0x3b;
        let n = rng.below(110);
        poke(
            a,
            b,
            at,
            &[
                u8::try_from(n / 2).unwrap(),
                u8::try_from(n - n / 2).unwrap(),
            ],
        );
    }
    let party: Vec<u8> = (0..7)
        .map(|_| {
            if rng.below(4) == 0 {
                0xff
            } else {
                u8::try_from(rng.below(0x1f)).unwrap()
            }
        })
        .collect();
    poke(a, b, menu::PARTY.ptr(b).addr, &party);
    // camera slots, flags and values of the three cameras
    for c in 0..3 {
        let base = cam::CAMERAS.ptr(b).addr + c * cam::CAMERA_SIZE;
        fill(a, b, rng, base + 0x700, 0x700);
        floats(a, b, rng, base + 0xc90, 0x80);
        fill(a, b, rng, base + 0xc68, 0x20);
        for k in 0..4 {
            // levels small, the camera's own index in byte 0
            let id = (rng.below(4) << 16) | (rng.below(2) << 20) | (rng.below(6) << 8) | c;
            poke(a, b, base + 0xc68 + 8 * k, &id.to_le_bytes());
        }
        poke(a, b, base + 0xd84, &[0, 0]);
        poke(
            a,
            b,
            base + 0xd94,
            &u16::from(rng.below(2) == 0).to_le_bytes(),
        );
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let out = scratch(a, b, 0x120);
    // _CIsqrt (an import the oracle lacks) as the CPU's fsqrt, in both
    for o in [&mut *a, &mut *b] {
        let p = o.rebase(0x0094_97b8);
        o.write(p, &[0xd9, 0xfa, 0xc3]).unwrap();
    }
    let chars = scratch(a, b, 0x400 * 8);
    // kernel tables where there are none
    for (g, stride) in [
        (menu::PLY_ROM, 0x40),
        (menu::STATUS_ROM, 0x20),
        (menu::WEAPON_NAMES, 0x60),
        (ffx_battle::minor::bridge::IMPORTANT_ROM, 0x10),
    ] {
        if synthetic || !readable(a, g.get(b), 0x20) {
            let t = fake_records(a, b, rng, 0x40, stride);
            set(a, b, g, t);
        }
    }
    if synthetic || !readable(a, ffx_battle::actor::bridge::MON_TOP.get(b), 0x10) {
        set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
    }
    for round in 0..600 {
        if w.synthetic {
            dress(a, b, rng, &w, chars);
        }
        if synthetic || round % 2 == 1 {
            globals(a, b, rng);
        }
        fill(a, b, rng, out, 0x100);
        calls(a, b, rng, out);
        scans(a, b, rng, out + 0x100);
    }
    twin("MsBattleMemInit", a, b, 0x0078_1d00, &[], 0, |m| {
        ffx_battle::minor::bridge::ms_battle_mem_init(m);
        0
    });
    same("MsBattleMemInit", a, b);
}

#[test]
fn bodies_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x0bad_cafe), true);
}

#[test]
fn bodies_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xcafe_0bad), false);
}
