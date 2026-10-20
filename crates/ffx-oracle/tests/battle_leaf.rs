//! Round 29's actor and save accessors and the wrappers around them
//! (`ffx_battle::minor`) against the original, as in `battle.rs`: actors of
//! our own on the image alone, the live ones (or ours) on a snapshot.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACTORS, fake_records, world};
use battle_common::*;
use ffx_battle::minor::bridge as g;
use ffx_oracle::{Conv, Oracle};

/// The callees of other modules the wrappers call through the game, as
/// recording stubs in both oracles: `TOGetFFXLangJmp` (no arguments, its
/// result the language), `op_et_weapon_init` and
/// `op_et_bindeff_send_common_signal` (two each). Each gets 0x20 bytes of
/// `rec`: count, arguments, result.
const STUBBED: [(u32, u8); 3] = [
    (g::TO_GET_FFX_LANG_JMP, 0),
    (g::OP_ET_WEAPON_INIT, 2),
    (g::OP_ET_SEND_COMMON_SIGNAL, 2),
];

fn call_original(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args).unwrap().eax
}

/// Random bytes in the fields the accessors read, for every actor.
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, actors: u32) {
    for i in 0..ACTORS {
        let p = actors + i * ffx_battle::actor::ACTOR_SIZE;
        for (at, len) in [
            (0x35, 1),
            (0x415, 4),
            (0x420, 1),
            (0x432, 1),
            (0x590, 1),
            (0x5a8, 1),
            (0x5d9, 5),
            (0x616, 2),
            (0x64d, 1),
            (0x65a, 2),
            (0xdfd, 1),
        ] {
            fill(a, b, rng, p + at, len);
        }
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x80);
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(STUBBED) {
            recording_stub(o, f, STUBS + 0x100 + 0x40 * k, rec + 0x20 * k, n, 0);
        }
    }
    // the menu texts 0 and 4, the field's treasure table, a password file
    let menu_text = ffx_battle::flow::bridge::MENU_TEXT.ptr(b).addr;
    for row in [0, 4] {
        let t = fake_records(a, b, rng, 0x40, 0x10);
        poke(a, b, menu_text + 12 * row + 8, &t.to_le_bytes());
    }
    let items = fake_records(a, b, rng, 0x40, 4);
    set(a, b, ffx_battle::reward::bridge::FIELD_ITEMS, items);
    let password = scratch(a, b, 0x200);
    set(a, b, g::PASSWORD_TEXT, password);
    let cue = scratch(a, b, 0x10);
    // what MsSetSaveCommand refreshes: the command table (0x140 records
    // on our own actors), the monsters, the second menu lists
    if w.synthetic {
        let t = fake_records(a, b, rng, 0x140, 0x60);
        set(a, b, ffx_kernel::bridge::COMMAND, t);
        if !readable(a, ffx_battle::actor::bridge::MON_TOP.get(b), 0x10) {
            set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
        }
    }
    let mem2 = scratch(a, b, 0x5400);
    set(a, b, ffx_battle::misc::bridge::ITEM_MEM2, mem2);
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    // MsFldGetScene's field list (one record, map 7, its encounters at
    // offset 0 of the encounter tables)
    let fields = scratch(a, b, 14);
    poke(a, b, fields, &7u16.to_le_bytes());
    set(a, b, ffx_battle::state::bridge::FIELD_RECORDS, fields);
    set(a, b, ffx_battle::minor::bridge::FIELD_COUNT, 1);
    let encounters = scratch(a, b, 0x20);
    set(a, b, ffx_battle::misc::bridge::FIELD_TABLE2, encounters);
    for round in 0..300 {
        if w.synthetic || round % 2 == 0 {
            dress(a, b, rng, w.actors);
        }
        // our own actors: no character, not bound, no followers (the
        // visibility request recurses through those)
        if w.synthetic {
            for k in 0..ACTORS {
                let p = w.actor(k);
                poke(a, b, p, &[0; 4]);
                poke(a, b, p + 0x13, &[0xff]);
                poke(a, b, p + 0x440, &[0]);
            }
        }
        // the rows free or busy
        for row in [0u32, 4] {
            poke(a, b, menu_text + 12 * row, &[u8::from(rng.below(5) == 0)]);
        }
        let i = if rng.below(6) == 0 {
            rng.next()
        } else {
            rng.below(0x22)
        };
        let (id, second) = (rng.below(0x50) | (rng.next() & 0xf000), rng.next());
        twin(
            "MsGetRomBtlText",
            a,
            b,
            0x0078_f8f0,
            &[id, second],
            ALL,
            |m| g::ms_get_rom_btl_text(m, id, second),
        );
        twin(
            "MsMenuGetBtlEndText",
            a,
            b,
            0x0078_fb60,
            &[id, second],
            ALL,
            |m| g::ms_menu_get_btl_end_text(m, id, second),
        );
        for (name, sym, rust) in [
            (
                "MsGetChrLibraName",
                0x0078_fae0,
                g::ms_get_chr_libra_name as fn(&Oracle, u32) -> u32,
            ),
            ("MsGetRamChrStr", 0x0079_af40, g::ms_get_ram_chr_str),
            ("MsMenuAbsElement", 0x0078_f910, g::ms_menu_abs_element),
            ("MsMenuInvElement", 0x0078_fe80, g::ms_menu_inv_element),
            ("MsMenuWeakElement2", 0x0079_0240, g::ms_menu_weak_element2),
            ("MsGetChrLibraWord", 0x0078_fac0, g::ms_get_chr_libra_word),
            (
                "MsGetChrMotionLevel",
                0x0079_5640,
                g::ms_get_chr_motion_level,
            ),
            ("MsGetRamChrFemale", 0x0079_adb0, g::ms_get_ram_chr_female),
            ("MsChrMotionSpeedOn", 0x007a_aac0, g::ms_chr_motion_speed_on),
            (
                "MsEffectChrMotionSync",
                0x007b_7e80,
                g::ms_effect_chr_motion_sync,
            ),
        ] {
            twin(name, a, b, sym, &[i], ALL, |m| rust(m, i));
        }
        let actor = ffx_battle::actor::bridge::ms_get_chr(b, i);
        twin(
            "MsCheckDefenseMotion",
            a,
            b,
            0x007a_f2f0,
            &[actor],
            ALL,
            |m| g::ms_check_defense_motion(m, actor),
        );
        twin("MsMotionSpeedResetChr", a, b, 0x007a_aff0, &[i], ALL, |m| {
            g::ms_motion_speed_reset_chr(m, i)
        });
        let v = rng.next();
        twin(
            "MsSetChrVisibleMain",
            a,
            b,
            0x0079_6860,
            &[i & 0xff, v],
            ALL,
            |m| g::ms_set_chr_visible_main(m, &mut call_original, i & 0xff, v),
        );
        poke(a, b, cue + 0xc, &[u8::try_from(i & 0xff).unwrap()]);
        twin("MsClearChrDfd", a, b, 0x007b_0500, &[cue], 0, |m| {
            g::ms_clear_chr_dfd(m, cue);
            0
        });
        // the stubs' results
        let lang = [9, 10, 0, 1, rng.below(12), rng.word()][rng.below(6) as usize];
        poke(a, b, rec + 4, &lang.to_le_bytes());
        poke(a, b, rec + 0x20 + 0xc, &rng.next().to_le_bytes());
        poke(a, b, rec + 0x40 + 0xc, &rng.next().to_le_bytes());
        twin("PasswordGetTotalIndex", a, b, 0x0079_02d0, &[], ALL, |m| {
            g::password_get_total_index(m, &mut call_original)
        });
        let oef2 = rng.next();
        twin(
            "MsEffectWeaponInit",
            a,
            b,
            0x0078_8b70,
            &[oef2, i],
            ALL,
            |m| g::ms_effect_weapon_init(m, &mut call_original, oef2, i),
        );
        let bits = rng.next();
        twin(
            "MsEffectSendSignal",
            a,
            b,
            0x0079_e4f0,
            &[i, bits],
            ALL,
            |m| g::ms_effect_send_signal(m, &mut call_original, i, bits),
        );
        // the language TkGetLang reads (MsGetCheckEnglish), the flag
        set(
            a,
            b,
            ffx_menu::tk::bridge::LANG,
            [1i8, 0, 2, -1][rng.below(4) as usize],
        );
        set(
            a,
            b,
            g::BATTLE_MESSAGE_OFF,
            [0, 0, 1, 0x80][rng.below(4) as usize],
        );
        twin("MsGetBattleMessageFlag", a, b, 0x0078_4e40, &[], ALL, |m| {
            g::ms_get_battle_message_flag(m, &mut call_original)
        });
        let chr = [rng.below(0x12), rng.next()][usize::from(rng.below(5) == 0)];
        let cid = [rng.below(0x80), rng.next()][usize::from(rng.below(5) == 0)];
        let on = rng.below(3);
        if chr < 0x12 {
            twin(
                "MsSetSaveCommandWithPrefix",
                a,
                b,
                0x0087_4200,
                &[chr, cid, on],
                ALL,
                |m| g::ms_set_save_command_with_prefix(m, chr, cid, on),
            );
        }
        // treasure: gil, a key item, nothing; spoils of the battle and the
        // message's
        let t = rng.below(0x40);
        let r = items + 0x20 + 4 * t;
        poke(a, b, r, &[[0, 1, 10, 7][rng.below(4) as usize]]);
        if round % 4 == 0 {
            twin("MsFieldItemGet", a, b, 0x0079_8fd0, &[t], ALL, |m| {
                g::ms_field_item_get_battle(m, &mut call_original, t)
            });
            twin("MsFieldItemGetMes", a, b, 0x0079_8ff0, &[t], ALL, |m| {
                g::ms_field_item_get_mes(m, &mut call_original, t)
            });
        }
        // a scene: its field and formation from the encounter tables (any
        // on the image)
        // (an encounter table of our own: one field of map 7, one group)
        let n = rng.below(6);
        let mut group = vec![0u8, 1, u8::try_from(n).unwrap(), 0, 0, 0, 0];
        group.extend((0..2 * n).map(|_| rng.byte()));
        poke(a, b, encounters, &group);
        let formation = if n > 0 && rng.below(3) != 0 {
            u32::from(group[7 + 2 * rng.below(n) as usize])
        } else {
            rng.below(0x40)
        };
        let map = [7, 7, 0, rng.below(0x400)][rng.below(4) as usize];
        let scene = map << 16 | formation;
        twin("MsFldGetSceneHigh", a, b, 0x0078_2940, &[scene], ALL, |m| {
            g::ms_fld_get_scene_high(m, scene)
        });
        if round % 3 == 0 {
            let pick = |rng: &mut Xorshift| {
                [rng.below(3), rng.below(0x12), 0xff, rng.next()][rng.below(4) as usize]
            };
            let (x, y, z) = (pick(rng), pick(rng), pick(rng));
            twin(
                "MsSetSavePartyMember",
                a,
                b,
                0x0078_6950,
                &[x, y, z],
                ALL,
                |m| g::ms_set_save_party_member(m, x, y, z),
            );
        }
        let (c, on) = (rng.next(), rng.next());
        poke(a, b, ply + (c & 0xff) * 0x94 + 0x2c, &[rng.byte()]);
        twin("MsSetSavePlyBit4", a, b, 0x0078_6aa0, &[c, on], 0, |m| {
            g::ms_set_save_ply_bit4(m, c, on);
            0
        });
        let k = rng.below(0x20);
        fill(a, b, rng, password, 0x100);
        twin("PasswordText_GetEntry", a, b, 0x0079_02a0, &[k], ALL, |m| {
            g::password_text_get_entry(m, k)
        });
        same("leaf", a, b);
    }
}

#[test]
fn leaf_accessors_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x2929_0b01), true);
}

#[test]
fn leaf_accessors_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x2929_0b02), false);
}
