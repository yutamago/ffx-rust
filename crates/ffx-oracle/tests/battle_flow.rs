//! The action flow (`ffx_battle::flow`) and `MsSetCameraMatrix` against the
//! original, as in `battle.rs`: the shared battle world with fake
//! characters and random actor fields, on the image alone and on both
//! snapshots. The `ffx_vfx` callees run as the original's in one oracle and
//! as `ffx_vfx`'s bridges in the other.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACTORS, World, world};
use battle_common::*;
use ffx_battle::flow::bridge as f;
use ffx_oracle::Oracle;
use ffx_vfx::et::bridge as et;

fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, chars: u32) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        let c = if rng.below(4) == 0 {
            0
        } else {
            chars + 0x400 * (i % 4)
        };
        poke(a, b, p, &c.to_le_bytes());
        for (at, len) in [
            (0x1f, 1),
            (0x3f0, 1),
            (0x400, 2),
            (0x436, 4),
            (0x508, 1),
            (0x594, 0x10),
            (0x5d0, 8),
            (0x606, 2),
            (0x608, 0xd),
            (0x616, 2),
            (0x617, 1),
            (0x640, 1),
            (0x6bc, 4),
            (0xdc8, 0x20),
            (0xe30, 0x60),
            (0xea8, 4),
        ] {
            fill(a, b, rng, p + at, len);
        }
        if rng.below(2) == 0 {
            for at in [0x594, 0x598, 0x59c, 0x5a0, 0x5d0, 0x5d4] {
                poke(a, b, p + at, &rng.range(-100, 20000).to_le_bytes());
            }
        }
    }
    // characters' words without signalling NaNs (the CPU quiets them)
    let v: Vec<u8> = (0..0x400)
        .flat_map(|_| battle_common::world::quiet(rng.next()).to_le_bytes())
        .collect();
    poke(a, b, chars, &v);
    // each character's block (Ch_SetThickness reads its default there)
    for k in 0..4 {
        poke(a, b, chars + 0x400 * k + 0x1c4, &chars.to_le_bytes());
    }
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    twin("MsStatusEffectClear", a, b, 0x0079_f110, &[i], ALL, |m| {
        f::ms_status_effect_clear(m, i, et::op_et_eff_off_signal)
    });
    twin("MsResetBindEffect", a, b, 0x0078_8e60, &[i], 0, |m| {
        f::ms_reset_bind_effect(m, i, et::op_et_bindeff_off_signal);
        0
    });
    twin("MsResetAfterImage", a, b, 0x0078_8e30, &[actor], 0, |m| {
        f::ms_reset_after_image(m, actor, et::op_et_weapon_afterimage_fade);
        0
    });
    twin(
        "MsEffectCheckMotion",
        a,
        b,
        0x0078_7840,
        &[i | (rng.next() & !0xff)],
        ALL,
        |m| f::ms_effect_check_motion(m, i).cast_unsigned(),
    );
    let set = rng.next();
    let chr = rng.below(ACTORS + 2);
    twin(
        "MsCheckStartEffect",
        a,
        b,
        0x007a_92c0,
        &[chr, set],
        ALL,
        |m| f::ms_check_start_effect(m, chr, set),
    );
    let set = match rng.below(3) {
        0 => 1 << rng.below(31),
        1 => (1 << rng.below(31)) | (1 << i),
        _ => rng.next(),
    };
    twin("MsSetHeadTarget", a, b, 0x007b_1e80, &[i, set], ALL, |m| {
        f::ms_set_head_target(m, i, set)
    });
    let (who, kind) = (rng.range(-1, 20), rng.below(6));
    twin(
        "MsSaveAddInfoData",
        a,
        b,
        0x0078_5a00,
        &[who.cast_unsigned(), kind],
        ALL,
        |m| f::ms_save_add_info_data(m, who, kind).cast_unsigned(),
    );
    let flags = if rng.below(3) == 0 {
        rng.range(-3, 0)
    } else {
        rng.range(0, 8)
    };
    let x = rng.next();
    twin(
        "MsLimitStatusProcess",
        a,
        b,
        0x0078_d270,
        &[x, actor, flags.cast_unsigned()],
        ALL,
        |m| f::ms_limit_status_process(m, actor, flags),
    );
    twin("MsMenuCheckLook", a, b, 0x0078_f9e0, &[], ALL, |m| {
        f::ms_menu_check_look(m).cast_unsigned()
    });
    let set = match rng.below(4) {
        0 => rng.next(),
        1 => 1 << rng.below(31),
        _ => (1 << rng.below(31)) | (1 << rng.below(31)),
    };
    twin("MsEffectCheckTarget", a, b, 0x0078_7900, &[set], ALL, |m| {
        f::ms_effect_check_target(m, set)
    });
    twin("MsReturnParty", a, b, 0x007a_eec0, &[], 0, |m| {
        f::ms_return_party(m);
        0
    });
    let o = w.out;
    let on = rng.below(4);
    let roll = rng.next() & 0xbfff_ffff;
    twin(
        "MsSetCameraMatrix",
        a,
        b,
        0x007c_06d0,
        &[on, o, o + 0x40, o + 0x80, o + 0x90, roll],
        ALL,
        |m| {
            ffx_battle::camera::bridge::ms_set_camera_matrix(
                m,
                on,
                o,
                o + 0x40,
                o + 0x80,
                o + 0x90,
                roll,
            )
        },
    );
    same("flow", a, b);
}

fn more(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    twin(
        "MsSetChrImageDamageAttack",
        a,
        b,
        0x0078_d790,
        &[actor],
        ALL,
        |m| f::ms_set_chr_image_damage_attack(m, actor).cast_unsigned(),
    );
    poke(a, b, actor + 0x50c, &[u8::try_from(rng.below(3)).unwrap()]);
    twin("MsSetChrHeight", a, b, 0x007a_b300, &[i], 0, |m| {
        f::ms_set_chr_height(m, i);
        0
    });
    let out: [u32; 4] = std::array::from_fn(|k| {
        if rng.below(4) == 0 {
            0
        } else {
            w.out + 0xc0 + 4 * u32::try_from(k).unwrap()
        }
    });
    twin("MsGetScreenData", a, b, 0x007b_bd00, &out, ALL, |m| {
        f::ms_get_screen_data(m, out)
    });
    let (k, v) = (rng.below(11), battle_common::world::quiet(rng.next()));
    twin(
        "MsSetChrMotionInfo",
        a,
        b,
        0x007b_4a90,
        &[i, k, v],
        0,
        |m| {
            f::ms_set_chr_motion_info(m, i, k, v);
            0
        },
    );
    let v = rng.next();
    twin("MsChrSEfoot", a, b, 0x0079_fd20, &[i, v], 0, |m| {
        f::ms_chr_se_foot(m, i, v.to_le_bytes()[0]);
        0
    });
    twin("MsEffectLimitSync", a, b, 0x007a_fde0, &[i], ALL, |m| {
        f::ms_effect_limit_sync(m, i).cast_unsigned()
    });
    let n = rng.range(-300, 300);
    twin(
        "MsAddYoujinbouCom",
        a,
        b,
        0x007b_2950,
        &[n.cast_unsigned()],
        ALL,
        |m| f::ms_add_youjinbou_com(m, n).cast_unsigned(),
    );
    let c = rng.below(0x22);
    twin("MsMagicEffectSet", a, b, 0x0079_e890, &[c, actor], 0, |m| {
        f::ms_magic_effect_set(m, c, actor);
        0
    });
    twin(
        "MsMenuCheckChrLook",
        a,
        b,
        0x0078_f990,
        &[c | (rng.next() & !0xff)],
        ALL,
        |m| f::ms_menu_check_chr_look(m, c),
    );
    let on = if rng.below(2) == 0 { 0 } else { rng.next() };
    twin("MsSetChrMoveFlagAll", a, b, 0x0078_8e90, &[on], 0, |m| {
        f::ms_set_chr_move_flag_all(m, on);
        0
    });
    for (name, sym, rust) in [
        (
            "MsBtlPosInit",
            0x007c_93d0,
            f::ms_btl_pos_init as fn(&mut Oracle),
        ),
        ("MsAtelRegistInit", 0x0079_7ae0, f::ms_atel_regist_init),
        ("MsExchangeParty", 0x007a_de10, f::ms_exchange_party),
        ("MsSEP3Dinit", 0x007a_0910, f::ms_sep3d_init),
        ("MsMenuInitText", 0x0078_fe60, f::ms_menu_init_text),
    ] {
        twin(name, a, b, sym, &[], 0, |m| {
            rust(m);
            0
        });
    }
    same("more", a, b);
}

fn third(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    let chance = rng.range(-10, 300);
    for c in 0..18 {
        let at = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr + 0x94 * c + 0x4a;
        poke(a, b, at, &[u8::from(rng.below(8) == 0) << 2]);
    }
    twin(
        "MsGetFirstAttack",
        a,
        b,
        0x0078_d090,
        &[chance.cast_unsigned()],
        ALL,
        |m| f::ms_get_first_attack(m, chance),
    );
    for i in 0..ACTORS {
        let d = if rng.below(3) == 0 {
            0xff
        } else {
            u8::try_from(rng.below(ACTORS)).unwrap()
        };
        poke(a, b, w.actor(i) + 0x6d4, &[d]);
    }
    twin("MsDefenseProcess", a, b, 0x007a_f8a0, &[], ALL, |m| {
        f::ms_defense_process(m)
    });
    let chr = if rng.below(3) == 0 {
        rng.range(-1, 3)
    } else {
        0
    };
    twin(
        "MsLimitTidusLearn",
        a,
        b,
        0x007b_0cd0,
        &[chr.cast_unsigned()],
        ALL,
        |m| f::ms_limit_tidus_learn(m, chr),
    );
    // camera requests: the slots' own IDs often, so that some are freed
    let at = ffx_battle::minor::bridge::ATEL_CAMERA.ptr(b).addr;
    for k in 0..12 {
        let id = if rng.below(2) == 0 {
            let slot = ffx_battle::camera::bridge::SLOTS_AT.ptr(b).addr
                + ffx_battle::camera::bridge::CAMERA_SIZE * rng.below(3)
                + 8 * rng.below(4);
            u32::from_le_bytes(b.read(slot, 4).unwrap().try_into().unwrap())
        } else {
            rng.below(4)
        };
        poke(a, b, at + 4 * k, &id.to_le_bytes());
    }
    twin("MsBattleCameraFree", a, b, 0x007b_a950, &[], 0, |m| {
        f::ms_battle_camera_free(m);
        0
    });
    // the table again (MsBattleCameraFree cleared half of it)
    for k in 0..12 {
        let id = rng.below(4) << 16 | rng.below(3);
        poke(a, b, at + 4 * k, &id.to_le_bytes());
    }
    let worker = w.out + 0x40;
    let slot = ffx_script::vm::bridge::atel_get_camera_work_adrs(worker);
    let id = u32::from_le_bytes(
        b.read(at + 4 * rng.below(12), 4)
            .unwrap()
            .try_into()
            .unwrap(),
    );
    poke(
        a,
        b,
        slot,
        &if rng.below(4) == 0 { 0 } else { id }.to_le_bytes(),
    );
    twin("MsBindOffCamera", a, b, 0x007b_a9c0, &[worker], 0, |m| {
        f::ms_bind_off_camera(m, worker, |_, w| {
            ffx_script::vm::bridge::atel_get_camera_work_adrs(w)
        });
        0
    });
    let actor = w.actor(rng.below(ACTORS));
    twin(
        "MsCheckStatusBeforeActionDamage",
        a,
        b,
        0x007a_f770,
        &[actor],
        ALL,
        |m| f::ms_check_status_before_action_damage(m, actor),
    );
    same("third", a, b);
}

fn fourth(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    let cmd = w.command;
    fill(a, b, rng, cmd, 0x60);
    for k in 0..4 {
        let ids: Vec<u8> = (0..2)
            .flat_map(|_| {
                let id: u16 = match rng.below(4) {
                    0 => 0x3118,
                    1 => u16::try_from(0x3000 + rng.below(0x40)).unwrap(),
                    _ => rng.next().to_le_bytes()[..2]
                        .try_into()
                        .map_or(0, u16::from_le_bytes),
                };
                id.to_le_bytes()
            })
            .collect();
        poke(a, b, cmd + 8 + 0x10 * k, &ids);
    }
    let (x, k) = (rng.next(), rng.below(4));
    twin(
        "MsLimitCommandProcess",
        a,
        b,
        0x007b_03e0,
        &[x, actor, cmd, k],
        ALL,
        |m| f::ms_limit_command_process(m, actor, cmd, k),
    );
    let chr = rng.below(ACTORS + 2);
    twin(
        "MsSetLimitParty",
        a,
        b,
        0x007b_1aa0,
        &[chr, actor],
        0,
        |m| {
            f::ms_set_limit_party(m, chr, actor);
            0
        },
    );
    let (al, fr) = (battle_common::world::quiet(rng.next()), rng.range(-1, 40));
    twin(
        "MsSetChrTransparent",
        a,
        b,
        0x0079_6480,
        &[i | (rng.next() & !0xff), al, fr.cast_unsigned()],
        ALL,
        |m| f::ms_set_chr_transparent(m, i, al, fr),
    );
    let j = rng.below(0x22);
    twin("MsEffectSetMotionFlag", a, b, 0x0078_83f0, &[j], 0, |m| {
        f::ms_effect_set_motion_flag(m, j);
        0
    });
    let n = if rng.below(4) == 0 { 0 } else { w.out + 0xf0 };
    twin(
        "MsGetTargetRandomList2",
        a,
        b,
        0x007b_7280,
        &[j, n],
        ALL,
        |m| f::ms_get_target_random_list2(m, j, n),
    );
    twin("MsAtelCamFree", a, b, 0x0079_74e0, &[], 0, |m| {
        f::ms_atel_cam_free(m, ffx_script::vm::bridge::atel_remove_ctrl_work);
        0
    });
    // magic effects: the underwater flag 0 or 1 (other values index the stack)
    set(
        a,
        b,
        ffx_battle::state::bridge::UNDERWATER,
        i8::try_from(rng.below(2)).unwrap(),
    );
    poke(a, b, cmd + 0x19, &[u8::try_from(rng.below(0x14)).unwrap()]);
    poke(
        a,
        b,
        cmd + 0x58,
        &[[0x24, 0x34, 0x44, 0x54, 0x25, 0][rng.below(6) as usize]],
    );
    let id = 0x3000 + rng.below(0x10);
    let who = rng.range(-1, 20);
    twin(
        "MsGetMagicEffNum",
        a,
        b,
        0x0079_d2f0,
        &[who.cast_unsigned(), id, cmd],
        ALL,
        |m| f::ms_get_magic_eff_num(m, who, id, cmd, |m| ffx_menu::tk::bridge::tk_get_lang(m)),
    );
    same("fourth", a, b);
}

fn globals(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for (g, len) in [
        (f::EFFECT_WAIT.ptr(b).addr, 2),
        (f::NO_HEAD_TARGET.ptr(b).addr, 1),
        (f::PARTY_SAVED.ptr(b).addr, 7),
        (f::PARTY_EXTRA.ptr(b).addr, 0x22),
        (f::RECORDS_OFF.ptr(b).addr, 1),
        (ffx_battle::minor::bridge::DIR_FREE.ptr(b).addr, 1),
        (ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr + 0x50, 0x10),
        (f::YOJIMBO_COMMANDS.ptr(b).addr, 4),
        (f::GROUND.ptr(b).addr, 4),
        (ffx_battle::minor::bridge::ATEL_MANAGERS.ptr(b).addr, 0x1900),
        (f::MENU_TEXT.ptr(b).addr, 0xcc),
        (f::BTL_POS.ptr(b).addr, 0x50),
        (f::TIDUS_LIMIT_USES.ptr(b).addr, 4),
        (f::RANDOM_TARGETS.ptr(b).addr, 1),
        (ffx_battle::minor::bridge::ATEL_CAM_ON.ptr(b).addr, 1),
        (ffx_battle::save::bridge::MENU.ptr(b).addr, 4),
        (
            ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr + 0xa60,
            0x2c * 18,
        ),
        (f::EFFECT_SUBST.ptr(b).addr, 6 * 8),
        (f::LIMIT_LEARNT.ptr(b).addr, 1),
        (ffx_battle::camera::bridge::SLOTS_AT.ptr(b).addr, 0xda0 * 3),
    ] {
        fill(a, b, rng, g, len);
    }
    set(a, b, f::MENU_LOOK, u8::from(rng.below(4) == 0));
    set(a, b, f::EFFECT_SUBST_COUNT, rng.range(-1, 9));
    set(
        a,
        b,
        ffx_menu::tk::bridge::LANG,
        rng.range(-1, 7).to_le_bytes()[0].cast_signed(),
    );
    let subst: Vec<u8> = (0..8)
        .flat_map(|_| {
            let id = u16::try_from(0x3000 + rng.below(0x10)).unwrap();
            [
                id.to_le_bytes(),
                rng.next().to_le_bytes()[..2].try_into().unwrap(),
                rng.next().to_le_bytes()[..2].try_into().unwrap(),
            ]
            .concat()
        })
        .collect();
    poke(a, b, f::EFFECT_SUBST.ptr(b).addr, &subst);
    set(
        a,
        b,
        ffx_battle::misc::bridge::SCREEN,
        ffx_battle::camera::bridge::SCREENS.ptr(b).addr,
    );
    let floats: Vec<u8> = (0..0x10)
        .flat_map(|_| {
            (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 7.0).to_le_bytes()
        })
        .collect();
    poke(
        a,
        b,
        ffx_battle::camera::bridge::SCREENS.ptr(b).addr + 0x40,
        &floats,
    );
    let g = (f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 7.0).to_bits();
    set(a, b, f::GROUND, g);
    set(a, b, f::RECORDS_OFF, u8::try_from(rng.below(3)).unwrap());
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let chars = scratch(a, b, 0x1000);
    for round in 0..800 {
        if w.synthetic {
            dress(a, b, rng, &w, chars);
        }
        if synthetic || round % 2 == 1 {
            globals(a, b, rng);
        }
        fill(a, b, rng, w.out, 0x100);
        calls(a, b, rng, &w);
        more(a, b, rng, &w);
        third(a, b, rng, &w);
        fourth(a, b, rng, &w);
    }
    // MsBtlChrInit clears every actor: last
    if w.synthetic {
        let extra = w.out + 0x80;
        set(a, b, f::CHR_EXTRA, extra);
        twin("MsBtlChrInit", a, b, 0x0079_35b0, &[], 0, |m| {
            f::ms_btl_chr_init(m);
            0
        });
        same("MsBtlChrInit", a, b);
    }
}

#[test]
fn flow_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x2468_ace0), true);
}

#[test]
fn flow_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x0eca_8642), false);
}
