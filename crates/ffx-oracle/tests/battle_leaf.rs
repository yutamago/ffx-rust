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

/// A float for the x87 checks: specials (no signalling NaNs: the emulator
/// keeps them), quiet randoms, plain values.
fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(6) {
        0 => [0, 0x8000_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000, 1][rng.below(6) as usize],
        1 => battle_common::world::quiet(rng.next()),
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 37.0).to_bits(),
    }
}

/// Group C: more accessors, the loops over the actors, the equipment, the
/// results screen's exchange, cached and once-only loads.
fn helpers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x80);
    // TkCdSize, TKMallocAndReadEventMessage, op_et_eff_off_signal
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip([
            (g::TK_CD_SIZE, 2u8),
            (g::TK_READ_EVENT_MESSAGE, 1),
            (g::OP_ET_EFF_OFF_SIGNAL, 1),
        ]) {
            recording_stub(o, f, STUBS + 0x100 + 0x40 * k, rec + 0x20 * k, n, 0);
        }
    }
    // the weapon name table where there is none
    if synthetic || !readable(a, ffx_battle::menu::bridge::WEAPON_NAMES.get(b), 0x20) {
        let t = fake_records(a, b, rng, 0x40, 0x60);
        set(a, b, ffx_battle::menu::bridge::WEAPON_NAMES, t);
    }
    let area = scratch(a, b, 0x200);
    let spoils = scratch(a, b, 0x40);
    set(a, b, g::SPOIL_WEAPONS, spoils);
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    let equipment = g::EQUIPMENT.ptr(b).addr;
    for round in 0..300 {
        if w.synthetic || round % 2 == 0 {
            dress(a, b, rng, w.actors);
            for k in 0..ACTORS {
                let p = w.actor(k);
                for (at, len) in [
                    (0x3f4, 1),
                    (0x3fb, 1),
                    (0x41f, 1),
                    (0x42b, 4),
                    (0x434, 1),
                    (0x4fc, 1),
                    (0x505, 4),
                    (0xe0c, 1),
                    (0xe90, 0x10),
                ] {
                    fill(a, b, rng, p + at, len);
                }
            }
        }
        if w.synthetic {
            for k in 0..ACTORS {
                let p = w.actor(k);
                poke(a, b, p, &[0; 4]);
                poke(a, b, p + 0x13, &[0xff]);
                poke(a, b, p + 0x440, &[0]);
            }
        }
        let i = if rng.below(6) == 0 {
            rng.next()
        } else {
            rng.below(0x22)
        };
        for (name, sym, rust) in [
            (
                "MsGetChrEffectVariable",
                0x0079_4e80,
                g::ms_get_chr_effect_variable as fn(&Oracle, u32) -> u32,
            ),
            ("MsGetChrSbyte508", 0x0079_4240, g::ms_get_chr_sbyte508),
            ("MsGetChrSbyte4fc", 0x0079_4280, g::ms_get_chr_sbyte4fc),
            ("MsGetChrByte41f", 0x0079_4d70, g::ms_get_chr_byte41f),
            ("MsGetChrFly", 0x0079_4f70, g::ms_get_chr_fly),
            ("MsGetChrByte3f4", 0x0079_5a60, g::ms_get_chr_byte3f4),
        ] {
            twin(name, a, b, sym, &[i], ALL, |m| rust(m, i));
        }
        twin("MsSetChrDirLeave", a, b, 0x0079_5e70, &[i], 0, |m| {
            g::ms_set_chr_dir_leave(m, i);
            0
        });
        let out = area + 0x100;
        fill(a, b, rng, out, 0x10);
        poke(a, b, out + 4, &float(rng).to_le_bytes());
        twin("MsGetChrHeadPos", a, b, 0x0079_50b0, &[i, out], 0, |m| {
            g::ms_get_chr_head_pos(m, i, out);
            0
        });
        let v = rng.below(3);
        if round % 4 == 0 {
            twin("MsSetMonsVisible", a, b, 0x0079_6880, &[v], ALL, |m| {
                g::ms_set_mons_visible(m, &mut call_original, v)
            });
            twin("MsSetPlayersVisible", a, b, 0x0079_68b0, &[v], ALL, |m| {
                g::ms_set_players_visible(m, &mut call_original, v)
            });
            twin("MsSetAllBodyHit", a, b, 0x0079_5da0, &[v], 0, |m| {
                g::ms_set_all_body_hit(m, v);
                0
            });
            let bits =
                [rng.next(), 0x8000_0000 | rng.next(), 1 << rng.below(31)][rng.below(3) as usize];
            twin("MsClearBodyHitBits", a, b, 0x0079_3dc0, &[bits], 0, |m| {
                g::ms_clear_body_hit_bits(m, bits);
                0
            });
        }
        // the statuses and encounter types
        let status = [1i8, 2, 0x18, 0x1b, 0x1e, 0x15, 0x16, 0, -1, 3][rng.below(10) as usize];
        set(a, b, ffx_battle::state::bridge::STATUS, status);
        set(
            a,
            b,
            ffx_battle::process::bridge::ENCOUNT_TYPE2,
            [0u8, 1, 2, 3, 4, 0xff][rng.below(6) as usize],
        );
        set(a, b, g::START_READY, u8::from(rng.below(2) == 0));
        twin("MsBattleStatusMenuless", a, b, 0x007a_8c20, &[], ALL, |m| {
            g::ms_battle_status_menuless(m)
        });
        twin("MsBattleStartReady", a, b, 0x0078_1650, &[], ALL, |m| {
            g::ms_battle_start_ready(m)
        });
        // HP and MP: a character, its values and maxima
        let chr = rng.below(0x12);
        let p = ply + chr * 0x94;
        for o in [0x1c, 0x20, 0x24, 0x28] {
            poke(
                a,
                b,
                p + o,
                &[rng.range(-50, 9999), rng.next().cast_signed()][rng.below(5).min(1) as usize]
                    .to_le_bytes(),
            );
        }
        let n = [rng.range(-3000, 3000).cast_unsigned(), rng.next()][rng.below(4).min(1) as usize];
        twin("MsSaveAddHP", a, b, 0x0078_59d0, &[chr, n], 0, |m| {
            g::ms_save_add_hp(m, chr, n);
            0
        });
        twin("MsSaveAddMP", a, b, 0x0078_5ae0, &[chr, n], 0, |m| {
            g::ms_save_add_mp(m, chr, n);
            0
        });
        // a gear record: weapon or armour, its name
        let gear = area;
        fill(a, b, rng, gear, 0x20);
        poke(a, b, gear + 5, &[u8::from(rng.below(2) == 0)]);
        poke(a, b, gear + 4, &[u8::try_from(rng.below(8)).unwrap()]);
        twin("MsSetWeaponName", a, b, 0x0079_93b0, &[gear], ALL, |m| {
            g::ms_set_weapon_name(m, gear)
        });
        // a character that waits or not (its motion state), an actor
        let actor = ffx_battle::actor::bridge::ms_get_chr(b, i);
        let chr_handle: u32 = u32::from_le_bytes(b.read(actor, 4).unwrap().try_into().unwrap());
        let v = rng.next();
        if chr_handle != 0 {
            twin(
                "MsWaitMotionPick",
                a,
                b,
                0x007a_8c50,
                &[chr_handle, actor, v],
                ALL,
                |m| g::ms_wait_motion_pick(m, chr_handle, actor, v),
            );
        }
        // the equipment slots: in use or not, flags 8 and 2 at random,
        // sometimes none held
        // (none held, or every held one marked, or only the last held)
        let mode = rng.below(6);
        let none = mode == 0;
        for k in 0..200 {
            let at = equipment + 22 * k + 2;
            let used = u8::from(rng.below(3) != 0);
            let mut f = [0, 8, 0xa, 2, 0xff][rng.below(5) as usize];
            if none || (mode == 2 && k < 199) {
                f &= !8;
            }
            if mode == 1 || (mode == 2 && rng.below(2) == 0) {
                f |= 2;
            }
            poke(a, b, at, &[used, f]);
        }
        twin("MsEquipmentHasUnmarked", a, b, 0x007a_d6e0, &[], ALL, |m| {
            g::ms_equipment_has_unmarked(m)
        });
        let on = rng.below(2) * rng.next();
        twin("MsEquipmentMarkFirst", a, b, 0x007a_d710, &[on], ALL, |m| {
            g::ms_equipment_mark_first(m, on)
        });
        let (wn, frames) = (rng.below(3), rng.below(40));
        // (an actor's weapon: the original reads through a null one)
        if ffx_battle::actor::bridge::ms_get_weapon(b, i & 0xff, wn.cast_signed()) != 0 {
            twin(
                "MsWeaponSetTransparent",
                a,
                b,
                0x007a_d5b0,
                &[i & 0xff, wn, frames],
                0,
                |m| {
                    g::ms_weapon_set_transparent(m, i & 0xff, wn, frames);
                    0
                },
            );
        }
        let c = rng.below(0x12);
        for (name, sym, rust) in [
            (
                "MsGetSaveChrArmorModel",
                0x007a_bb70,
                g::ms_get_save_chr_armor_model as fn(&mut Oracle, u32) -> u32,
            ),
            (
                "MsGetSaveChrWeaponModel",
                0x007a_bba0,
                g::ms_get_save_chr_weapon_model,
            ),
        ] {
            twin(name, a, b, sym, &[c], ALL, |m| rust(m, c));
        }
        // the results screen's exchange: inventory slots
        let list: Vec<u8> = (0..0x10)
            .flat_map(|_| u16::try_from(rng.below(200)).unwrap().to_le_bytes())
            .collect();
        poke(a, b, spoils, &list);
        set(a, b, g::SPOIL_WEAPON_TOP, rng.below(6));
        set(a, b, g::SPOIL_WEAPON_CURSOR, rng.below(6));
        let slot = rng.below(8);
        set(a, b, g::SPOIL_WEAPON_SLOT, slot);
        poke(
            a,
            b,
            g::SPOIL_WEAPON_SLOTS.ptr(b).addr + 2 * slot,
            &u16::try_from([rng.below(200), 0xff][rng.below(5).min(1) as usize])
                .unwrap()
                .to_le_bytes(),
        );
        if round % 3 == 0 {
            twin("MsExchangeSaveWeapon", a, b, 0x008b_e0a0, &[], ALL, |m| {
                g::ms_exchange_save_weapon(m)
            });
        }
        // the cached size: unset (negative) or kept, the stub's size
        set(
            a,
            b,
            g::CD_SIZE_2A,
            [-1, 0x1234, i32::MIN, 0][rng.below(4) as usize],
        );
        poke(
            a,
            b,
            rec + 0xc,
            &[rng.range(-0x800, 0x80000), rng.next().cast_signed()][rng.below(3).min(1) as usize]
                .to_le_bytes(),
        );
        twin("TkCdSizeKBCached", a, b, 0x0087_41d0, &[], ALL, |m| {
            g::tk_cd_size_kb_cached(m, &mut call_original)
        });
        let v = [0, 0xff, rng.below(4), 0x100 | rng.below(4)][rng.below(4) as usize];
        poke(
            a,
            b,
            area + 0x40,
            &[0, 1, 2].map(|_| [0u8, 1, 0xff][rng.below(3) as usize]),
        );
        twin(
            "MsCountSlotsOf",
            a,
            b,
            0x007b_1e40,
            &[area + 0x40, v],
            ALL,
            |m| g::ms_count_slots_of(m, area + 0x40, v),
        );
        // effect cameras: kept or not
        for k in 0..8 {
            let v = [0, rng.next()][rng.below(2) as usize];
            poke(
                a,
                b,
                ffx_battle::minor::bridge::EFFECT_CAMERA_BUF.ptr(b).addr + 4 * k,
                &v.to_le_bytes(),
            );
        }
        let (owner, k, kind) = (
            rng.below(4),
            rng.range(-1, 10).cast_unsigned(),
            rng.below(4),
        );
        twin(
            "MsEffectCameraGet",
            a,
            b,
            0x0078_77d0,
            &[owner, k, kind],
            ALL,
            |m| g::ms_effect_camera_get(m, owner, k, kind),
        );
        let id = [
            rng.below(0x80),
            rng.below(0x1000),
            rng.next(),
            rng.next() & !0xf80,
        ][rng.below(4) as usize];
        twin("MsImportantReset", a, b, 0x0079_07a0, &[id], 0, |m| {
            g::ms_important_reset(m, id);
            0
        });
        let actor2 = area + 0x80 - 0xe90;
        for k in 0..4 {
            let e = [0, rng.next()][rng.below(2) as usize];
            poke(a, b, actor2 + 0xe90 + 4 * k, &e.to_le_bytes());
        }
        let any = rng.next();
        twin(
            "MsResetWeaponEffectAttribute",
            a,
            b,
            0x0079_ea90,
            &[any, actor2],
            ALL,
            |m| g::ms_reset_weapon_effect_attribute(m, &mut call_original, actor2),
        );
        // a cue: kind 0x1d or not, its actor
        let cue = area + 0x120 - 0x4d4;
        poke(a, b, cue + 0xc, &[[0x1d, 0x1c, 0][rng.below(3) as usize]]);
        poke(a, b, cue + 0x14, &[u8::try_from(rng.below(0x22)).unwrap()]);
        poke(a, b, cue + 0x4d4, &float(rng).to_le_bytes());
        ftwin("MsCueMoveSpeed", a, b, 0x007a_88e0, &[cue], |m| {
            g::ms_cue_move_speed(m, cue)
        });
        // the monster texts: loaded or not
        let texts = a.rebase(ffx_battle::read::bridge::MONSTER_TEXTS[0]);
        poke(
            a,
            b,
            texts,
            &[0, rng.next()][rng.below(2) as usize].to_le_bytes(),
        );
        poke(a, b, rec + 0x20 + 8, &rng.next().to_le_bytes());
        twin("readBtlMonsterName", a, b, 0x0078_44c0, &[], 0, |m| {
            g::read_btl_monster_name(m, &mut call_original);
            0
        });
        set(
            a,
            b,
            ffx_battle::parse::bridge::EFFECT_READ_CLEAR,
            [0, 1, u32::MAX][rng.below(3) as usize],
        );
        set(
            a,
            b,
            ffx_battle::parse::bridge::EFFECT_READ_CHR,
            rng.below(0x22),
        );
        twin("MsEffectReadStatInit", a, b, 0x0078_81c0, &[], 0, |m| {
            g::ms_effect_read_stat_init(m);
            0
        });
        same("helpers", a, b);
    }
}

#[test]
fn leaf_helpers_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    helpers(&mut a, &mut b, &mut Xorshift(0x2929_0c01), true);
}

#[test]
fn leaf_helpers_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    helpers(&mut a, &mut b, &mut Xorshift(0x2929_0c02), false);
}

/// `MsBtlPosPrintBin` (`0x007c9f10`, 0x57f bytes, unreachable): room for
/// group D's recording stubs.
const STUBS_D: u32 = 0x007c_9f10;

/// Group D's callees of other modules, recording stubs in both oracles:
/// (function, arguments, result).
fn d_stubs() -> [(u32, u8, u32); 13] {
    [
        (g::YI_GET_BATTLE_CENTER, 1, 0x55),
        (g::TO_DRAW_AMES_CENTERMES, 2, 0),
        (g::FFX_IS_DEBUG, 0, 0),
        (g::DBG_PRINTF, 4, 0x11),
        (g::ATEL_GET_BATTLE_ID_ADRS, 1, 0),
        (g::OP_ET_BINDEFF_DRAW_ON, 1, 0x21),
        (g::OP_ET_BINDEFF_DRAW_OFF, 1, 0x22),
        (g::OP_OVL_EFFECT_INIT, 0, 0x23),
        (g::GRAPHIC_MAGIC_ASYNC, 0, 0),
        (g::MAGIC_FILE_START, 0, 0x24),
        (g::MAGIC_FILE_OVERLAY_TABLE, 1, 0),
        (g::FREE_00687010, 1, 0),
        (g::IS_PTR32_NULL, 1, 0),
    ]
}

/// Group D: setters and checks on the actors, the message and effect
/// glue, the equipment list's free slot.
fn setters(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use battle_common::crt::patch;
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x200);
    let stubs = d_stubs();
    for o in [&mut *a, &mut *b] {
        patch(o);
        for (k, (f, n, r)) in (0u32..).zip(stubs) {
            recording_stub(o, f, STUBS_D + 0x40 * k, rec + 0x20 * k, n, r);
        }
        // op_ovl_effect_init keeps the event effect table it sees
        // (`mov eax, [table]; mov [rec + 0x1fc], eax; ret`)
        let table = g::EVENT_EFFECT_TABLE.ptr(o).addr;
        let at = o.rebase(STUBS_D + 0x40 * 13);
        let mut code = vec![0xa1];
        code.extend(table.to_le_bytes());
        code.push(0xa3);
        code.extend((rec + 0x1fc).to_le_bytes());
        code.push(0xc3);
        o.write(at, &code).unwrap();
        let entry = o.rebase(g::OP_OVL_EFFECT_INIT);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
            .unwrap();
    }
    // the stub results: the word after the arguments
    let result = |k: u32| rec + 0x20 * k + 4 + 4 * u32::from(stubs[k as usize].1);
    let area = scratch(a, b, 0x400);
    let messages = scratch(a, b, 0x100);
    let extra = scratch(a, b, 0x24);
    set(a, b, g::CHR_EXTRA, extra);
    let password = scratch(a, b, 0x10);
    let rank_tables = scratch(a, b, 0x80);
    let equipment = g::EQUIPMENT.ptr(b).addr;
    for round in 0..300 {
        if w.synthetic || round % 2 == 0 {
            dress(a, b, rng, w.actors);
            for k in 0..ACTORS {
                let p = w.actor(k);
                for (at, len) in [
                    (0x34, 1),
                    (0x3f0, 1),
                    (0x400, 5),
                    (0x41e, 1),
                    (0x425, 1),
                    (0x434, 1),
                    (0x446, 1),
                    (0x4c, 4),
                    (0x4fc, 1),
                    (0x509, 1),
                    (0x50d, 1),
                    (0x510, 1),
                    (0x5d0, 4),
                    (0x608, 1),
                    (0x6d8, 1),
                    (0x776, 0x5b0),
                    (0xdc0, 4),
                    (0xdcc, 3),
                    (0xdd4, 1),
                    (0xde7, 1),
                    (0xf5c, 2),
                ] {
                    fill(a, b, rng, p + at, len);
                }
                poke(a, b, p + 0x534, &float(rng).to_le_bytes());
                poke(a, b, p + 0x53c, &float(rng).to_le_bytes());
                let reel =
                    [0x3074u16, 0x3075, 0x3076, 0x3077, 0x3073, 0x3078][rng.below(6) as usize];
                poke(a, b, p + 0xf5c, &reel.to_le_bytes());
            }
        }
        if w.synthetic {
            for k in 0..ACTORS {
                let p = w.actor(k);
                poke(a, b, p, &[0; 4]);
                poke(a, b, p + 0x13, &[0xff]);
                poke(a, b, p + 0x440, &[0]);
            }
        }
        let i = if rng.below(6) == 0 {
            rng.next()
        } else {
            rng.below(0x22)
        };
        // a file of tables, or none
        let file = area;
        fill(a, b, rng, file, 0x20);
        set(a, b, g::TABLE_FILE, [0, file][rng.below(4).min(1) as usize]);
        set(a, b, g::TABLE_FILE2, file + 4);
        twin("SetFileTables", a, b, 0x0077_6f00, &[], 0, |m| {
            g::set_file_tables(m);
            0
        });
        // the command buffer: room or full
        set(
            a,
            b,
            ffx_battle::misc::bridge::COM_BUF,
            rng.range(-2, 10).cast_unsigned(),
        );
        let (x, y, z) = (rng.next(), rng.next(), rng.next());
        twin("MsComBufAdd", a, b, 0x0079_d2a0, &[x, y, z], ALL, |m| {
            g::ms_com_buf_add(m, x, y, z)
        });
        let bits =
            [rng.next(), 0x8000_0000 | rng.next(), 1 << rng.below(31)][rng.below(3) as usize];
        twin(
            "MsSetChrScriptTargets",
            a,
            b,
            0x0079_6430,
            &[i, bits],
            ALL,
            |m| g::ms_set_chr_script_targets(m, i, bits),
        );
        set(a, b, g::MAP_CENTER_OWN, u8::from(rng.below(2) == 0));
        fill(
            a,
            b,
            rng,
            ffx_battle::minor::bridge::BATTLE_MEM.ptr(b).addr,
            0x10,
        );
        let out = area + 0x40;
        twin("MsGetMapCenter", a, b, 0x0078_2ae0, &[out], ALL, |m| {
            g::ms_get_map_center(m, &mut call_original, out)
        });
        twin("MsAnyChrMotion", a, b, 0x007a_8be0, &[bits], ALL, |m| {
            g::ms_any_chr_motion(m, bits)
        });
        let speed = [rng.below(0x200), 0xffff_ffff, rng.next()][rng.below(3) as usize];
        twin(
            "MsEffectSetSpeed",
            a,
            b,
            0x0078_8430,
            &[i, speed],
            ALL,
            |m| g::ms_effect_set_speed(m, i, speed),
        );
        // the message table: some messages, the windows on or off, the
        // English check (TkGetLang) and the message flag
        fill(a, b, rng, messages, 0x100);
        set(
            a,
            b,
            g::BATTLE_MESSAGES,
            [0, messages][rng.below(4).min(1) as usize],
        );
        set(a, b, g::MESSAGE_WINDOWS_ON, rng.below(2));
        set(
            a,
            b,
            ffx_menu::tk::bridge::LANG,
            [1i8, 0, 2][rng.below(3) as usize],
        );
        set(a, b, g::BATTLE_MESSAGE_OFF, u8::from(rng.below(4) == 0));
        let msg = rng.below(0x10);
        twin("MsShowBattleMessage", a, b, 0x0079_d090, &[msg], ALL, |m| {
            g::ms_show_battle_message(m, &mut call_original, msg)
        });
        let (chr, id) = (
            rng.below(8),
            [0x3000 | rng.below(0x1000), rng.next()][rng.below(3).min(1) as usize],
        );
        twin(
            "MsAbilityMapSetCommand",
            a,
            b,
            0x0079_8840,
            &[chr, id],
            0,
            |m| {
                g::ms_ability_map_set_command(m, chr, id);
                0
            },
        );
        let mode = rng.next();
        twin(
            "MsSetChrGravMode",
            a,
            b,
            0x0079_6070,
            &[i, mode],
            ALL,
            |m| g::ms_set_chr_grav_mode(m, i, mode),
        );
        twin("MsEffectCheckMotion2", a, b, 0x0078_78a0, &[i], ALL, |m| {
            g::ms_effect_check_motion2(m, i)
        });
        let (c, off) = (
            [rng.below(8), rng.next()][rng.below(4).min(1) as usize],
            rng.next(),
        );
        twin(
            "MsChangeWeaponInvisible",
            a,
            b,
            0x007a_d5e0,
            &[c, off],
            0,
            |m| {
                g::ms_change_weapon_invisible(m, c, off);
                0
            },
        );
        ftwin("MsGetChrLiftHeight", a, b, 0x0079_4990, &[i], |m| {
            g::ms_get_chr_lift_height(m, i)
        });
        set(
            a,
            b,
            g::RAND_FIXED,
            [0u16, 5, 0x1234][rng.below(3) as usize],
        );
        let (rid, rn) = (
            [0, 5, rng.below(0x10)][rng.below(3) as usize],
            rng.range(-2, 0x120).cast_unsigned(),
        );
        twin("MsRandCheck", a, b, 0x007a_8ad0, &[rid, rn], ALL, |m| {
            g::ms_rand_check(m, &mut call_original, rid, rn)
        });
        poke(a, b, result(2), &u32::from(rng.below(2) == 0).to_le_bytes());
        twin("MsBattleDraw", a, b, 0x0078_0d00, &[], ALL, |m| {
            g::ms_battle_draw(m, &mut call_original)
        });
        set(a, b, ffx_battle::parse::bridge::EFFECT_READ_CHR, rng.next());
        twin("MsEffectReadError", a, b, 0x0078_81b0, &[], ALL, |m| {
            g::ms_effect_read_error(m)
        });
        if round % 4 == 0 {
            let v = rng.below(3);
            twin("MsSetAllVisible", a, b, 0x0079_6680, &[v], ALL, |m| {
                g::ms_set_all_visible(m, &mut call_original, v)
            });
        }
        twin("MsChrUnableToAct", a, b, 0x0079_a080, &[i], ALL, |m| {
            g::ms_chr_unable_to_act(m, i)
        });
        // a handler's battle work byte: AtelGetBattleIdAdrs gives the area
        poke(a, b, result(4), &(area + 0x80).to_le_bytes());
        let (kind, owner, n, v) = (rng.below(6), rng.below(4), rng.below(4), rng.next());
        twin(
            "MsAtelSetHandlerByte",
            a,
            b,
            0x0079_87b0,
            &[kind, owner, n, v],
            ALL,
            |m| g::ms_atel_set_handler_byte(m, &mut call_original, kind, owner, n, v),
        );
        twin("MsChrGroundFlag", a, b, 0x0079_4db0, &[i], ALL, |m| {
            g::ms_chr_ground_flag(m, i)
        });
        let on = rng.below(2) * rng.next();
        twin("MsPauseChrEffect", a, b, 0x0079_ea40, &[i, on], ALL, |m| {
            g::ms_pause_chr_effect(m, &mut call_original, i, on)
        });
        poke(a, b, result(8), &rng.below(2).to_le_bytes());
        poke(
            a,
            b,
            result(10),
            &[0, rng.next()][rng.below(2) as usize].to_le_bytes(),
        );
        set(
            a,
            b,
            ffx_battle::state::bridge::EVENT_EFFECT_STAT,
            [0i8, 1, 3][rng.below(3) as usize],
        );
        twin(
            "MsEventEffectReadComplete",
            a,
            b,
            0x0078_8d10,
            &[],
            ALL,
            |m| g::ms_event_effect_read_complete(m, &mut call_original),
        );
        // the extra records: on or off, an actor's character or another
        let actor = ffx_battle::actor::bridge::ms_get_chr(b, i);
        let chr: u32 = u32::from_le_bytes(b.read(actor, 4).unwrap().try_into().unwrap());
        for k in 0..3 {
            let e = extra + 12 * k;
            poke(a, b, e, &[u8::from(rng.below(2) == 0)]);
            poke(
                a,
                b,
                e + 4,
                &[chr, rng.next()][rng.below(2) as usize].to_le_bytes(),
            );
        }
        twin("MsChrInExtra", a, b, 0x0079_6d10, &[i], ALL, |m| {
            g::ms_chr_in_extra(m, i)
        });
        // CTB: the counters near their ends
        let j = rng.below(ACTORS);
        fill(a, b, rng, rank_tables, 0x80);
        for at in [rank_tables + 0x40 + j, rank_tables + 0x60 + j] {
            poke(
                a,
                b,
                at,
                &[[0u8, 1, 0xfd, 0xfe, 0xff, 3][rng.below(6) as usize]],
            );
        }
        let rank = rng.range(-1, 6).cast_unsigned();
        twin(
            "MsAddNextCtb",
            a,
            b,
            0x0079_a020,
            &[j, rank, rank_tables, rank_tables + 0x40, rank_tables + 0x60],
            0,
            |m| {
                g::ms_add_next_ctb(
                    m,
                    j,
                    rank,
                    rank_tables,
                    rank_tables + 0x40,
                    rank_tables + 0x60,
                );
                0
            },
        );
        let sel = [rng.below(0x20), rng.range(-24, 0).cast_unsigned()][rng.below(2) as usize];
        twin("MsResetAvoidFlag", a, b, 0x0078_d620, &[i, sel], ALL, |m| {
            g::ms_reset_avoid_flag(m, i, sel)
        });
        // the damage limit records name actor i and its command's byte +2
        let i8 = i & 0xff;
        if ffx_battle::actor::bridge::ms_btl_chr_num_check(i8) != 0 {
            let cmd = ffx_battle::actor::bridge::ms_get_chr_command(b, i8);
            let cb = b.read(cmd + 2, 1).unwrap()[0];
            let tgt = ffx_battle::actor::bridge::ms_get_chr(b, j);
            for k in 0..2 {
                let e = tgt + 0x776 + 0x2d8 * k;
                if rng.below(2) == 0 {
                    poke(
                        a,
                        b,
                        e,
                        &[
                            u8::try_from(i8).unwrap(),
                            [cb, cb ^ 1][rng.below(3).min(1) as usize],
                        ],
                    );
                }
            }
        }
        twin("MsGetDamageLimit", a, b, 0x0079_a860, &[i, j], ALL, |m| {
            g::ms_get_damage_limit(m, i, j)
        });
        let v = rng.next();
        twin("MsSetChrByte446", a, b, 0x007b_75a0, &[i, v], 0, |m| {
            g::ms_set_chr_byte446(m, i, v);
            0
        });
        twin("MsSetChrSpeedBytes", a, b, 0x0079_5d40, &[i, v], ALL, |m| {
            g::ms_set_chr_speed_bytes(m, i, v)
        });
        let f = float(rng);
        twin(
            "MsSetChrFloat53c",
            a,
            b,
            0x0079_5ff0,
            &[i, v, f],
            ALL,
            |m| g::ms_set_chr_float53c(m, i, v, f),
        );
        // (finite: Ch_SetRot prints non-finite angles with dbgPrintf, which
        // its port leaves out)
        let f = if f & 0x7f80_0000 == 0x7f80_0000 {
            0x3fc9_0fdb
        } else {
            f
        };
        twin("MsSetChrRot", a, b, 0x0079_5dd0, &[i, f], ALL, |m| {
            g::ms_set_chr_rot(m, i, f)
        });
        set(a, b, g::PASSWORD_TEXT_ON, rng.below(2));
        set(
            a,
            b,
            g::PASSWORD_TEXT,
            [0, password][rng.below(4).min(1) as usize],
        );
        twin("PasswordTextDispose", a, b, 0x0079_02f0, &[], 0, |m| {
            g::password_text_dispose(m, &mut call_original);
            0
        });
        twin("MsGetChrReelKind", a, b, 0x007b_19f0, &[i], ALL, |m| {
            g::ms_get_chr_reel_kind(m, i)
        });
        // the equipment list: full or with free slots
        let (full, last) = (rng.below(4) == 0, rng.below(4) == 0);
        for k in 0..200 {
            let used = full || (last && k < 199) || (!last && rng.below(4) != 0);
            poke(a, b, equipment + 22 * k + 2, &[u8::from(used)]);
        }
        fill(a, b, rng, area + 0x100, 0x18);
        if round % 2 == 0 {
            twin(
                "MsAddEquipment",
                a,
                b,
                0x007a_b910,
                &[area + 0x100],
                ALL,
                |m| g::ms_add_equipment(m, area + 0x100),
            );
        }
        same("setters", a, b);
    }
}

#[test]
fn leaf_setters_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    setters(&mut a, &mut b, &mut Xorshift(0x2929_0d01), true);
}

#[test]
fn leaf_setters_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    setters(&mut a, &mut b, &mut Xorshift(0x2929_0d02), false);
}
