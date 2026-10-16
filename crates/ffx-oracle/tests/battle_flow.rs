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
    ] {
        fill(a, b, rng, g, len);
    }
    set(a, b, f::MENU_LOOK, u8::from(rng.below(4) == 0));
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
