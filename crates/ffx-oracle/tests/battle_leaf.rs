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
            // Wakka's weapon change: some entries in use, flagged 8, not 2
            // (the first now and then), a gear treasure (kind 5) mostly
            let eq = g::EQUIPMENT.ptr(b).addr;
            // (not on the image alone: no gear tables there)
            if !synthetic && rng.below(3) != 0 {
                poke(a, b, r, &[5]);
            }
            if rng.below(3) == 0 {
                poke(a, b, eq + 2, &[rng.byte() | 1, 8]);
            }
            for _ in 0..8 {
                let e = eq + 0x16 * rng.below(200);
                poke(
                    a,
                    b,
                    e + 2,
                    &[
                        [0, 1, rng.byte() | 1][rng.below(3) as usize],
                        [8, 0xa, rng.byte()][rng.below(3) as usize],
                    ],
                );
            }
            if rng.below(3) == 0 {
                for k in 0..200 {
                    poke(a, b, eq + 0x16 * k + 3, &[0]);
                }
            }
            twin("MsChangeWeaponWakka", a, b, 0x007a_d640, &[t], ALL, |m| {
                g::ms_change_weapon_wakka(m, &mut call_original, t)
            });
            same("MsChangeWeaponWakka", a, b);
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
    // Mon_GetName (round 32): three kernel monster text tables (one
    // segment each: IDs 0-0x64, 0x65-0xb4, 0xb5-0x150; records of 8 bytes,
    // the first word a name's offset), its names now and then longer
    // than 0x40 bytes; and its destination with room after
    let mut mon_texts = Vec::new();
    for (first, last) in [(0u16, 0x64u16), (0x65, 0xb4), (0xb5, 0x150)] {
        let mut t = vec![0u8; 0x2000];
        t[0..2].copy_from_slice(&1u16.to_le_bytes());
        let n = usize::from(last - first) + 1;
        let strings = 8 * n;
        for (o, v) in [
            (8, first),
            (10, last),
            (12, 8),
            (14, u16::try_from(strings).unwrap()),
        ] {
            t[o..o + 2].copy_from_slice(&v.to_le_bytes());
        }
        t[16..20].copy_from_slice(&0x20u32.to_le_bytes());
        let base = 0x20 + strings;
        let mut at = 0;
        for k in 0..n {
            t[0x20 + 8 * k..0x22 + 8 * k]
                .copy_from_slice(&u16::try_from(at).unwrap().to_le_bytes());
            let len =
                [rng.below(0x3f), rng.below(8), 0x40, 0x50][rng.below(8).min(3) as usize] as usize;
            if base + at + len + 1 >= t.len() {
                break;
            }
            for c in 0..len {
                t[base + at + c] = 0x41 + u8::try_from(rng.below(26)).unwrap();
            }
            at += len + 1;
        }
        let p = scratch(a, b, 0x2000);
        poke(a, b, p, &t);
        mon_texts.push(p);
    }
    let name = scratch(a, b, 0x100);
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
        // the walk abilities: joined or not, their bits, the timer
        for k in 0..0x12 {
            let p = ply + k * 0x94;
            poke(a, b, p + 0x2c, &[rng.byte()]);
            poke(
                a,
                b,
                p + 0x4c,
                &u16::try_from(rng.below(0x1_0000)).unwrap().to_le_bytes(),
            );
            poke(a, b, p + 0x4e, &[rng.byte()]);
        }
        let secs = |rng: &mut Xorshift, d: f32| {
            [
                (f32::from(u16::try_from(rng.below(400)).unwrap()) / d).to_bits(),
                float(rng),
            ][rng.below(5).min(1) as usize]
        };
        set(a, b, g::WALK_TIMER, secs(rng, 100.0));
        set(
            a,
            b,
            g::WALK_PERIOD,
            [0x4040_0000, secs(rng, 100.0)][rng.below(4).min(1) as usize],
        );
        let dt = secs(rng, 60.0);
        twin("MsWalkAbilityProcess", a, b, 0x0078_2d60, &[dt], 0, |m| {
            g::ms_walk_ability_process(m, dt);
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
        // a monster's name from the made-up tables (the first set: no
        // read; the others now and then missing)
        // (none loaded now and then: the texts are read, the reader's
        // stub giving none)
        let unread = rng.below(6) == 0;
        for (k, &t) in mon_texts.iter().enumerate() {
            let at = a.rebase(ffx_battle::read::bridge::MONSTER_TEXTS[k]);
            let v: u32 = if (k > 0 && rng.below(5) == 0) || (k == 0 && unread) {
                0
            } else {
                t
            };
            poke(a, b, at, &v.to_le_bytes());
        }
        if unread {
            poke(a, b, rec + 0x20 + 8, &[0; 4]);
        }
        poke(a, b, name, &[0xcc; 0x100]);
        let id = [
            rng.below(0x151),
            rng.below(0x151),
            0x64,
            0x65,
            0xb4,
            0xb5,
            rng.next(),
        ][rng.below(7) as usize];
        twin("Mon_GetName", a, b, 0x0078_4260, &[name, id], ALL, |m| {
            g::mon_get_name(m, &mut call_original, name, id)
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

/// Points the function at `f` (ffx.sym address) at the code at `at`
/// (ffx.sym address) with a `jmp`.
fn redirect(o: &mut Oracle, f: u32, at: u32) {
    let entry = o.rebase(f);
    let rel = o.rebase(at).wrapping_sub(entry + 5);
    o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
        .unwrap();
}

/// `MsBtlPosPrintBin` (`0x007c9f10`, 0x57f bytes, unreachable): room for
/// group D's recording stubs.
const STUBS_D: u32 = 0x007c_9f10;

/// `FUN_007c1790` (0x44f bytes, unreachable): more stub room.
const STUBS_E: u32 = 0x007c_1790;

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
        // the last actor too, with an effect and a character
        let last = w.actor(0x1e);
        poke(a, b, last + 0x4c, &[u8::from(rng.below(3) != 0), 0, 0, 0]);
        if b.read(last, 4).unwrap() == [0; 4] {
            poke(a, b, last, &w.out.to_le_bytes());
        }
        twin("MsPauseChrEffectAll", a, b, 0x0079_eae0, &[on], ALL, |m| {
            g::ms_pause_chr_effect_all(m, &mut call_original, on)
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
        let actor = w.actor(rng.below(ACTORS));
        poke(
            a,
            b,
            actor + 0x446,
            &[[1, 4, rng.byte()][rng.below(3) as usize]],
        );
        twin(
            "MsEffectSignalAvoid",
            a,
            b,
            0x007b_7650,
            &[i, actor],
            ALL,
            |m| g::ms_effect_signal_avoid(m, i, actor),
        );
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

/// Machine code for a stub: each word of `words` emitted as `op` with the
/// address after it.
fn code_with(parts: &[(&[u8], Option<u32>)]) -> Vec<u8> {
    let mut code = Vec::new();
    for (op, imm) in parts {
        code.extend(*op);
        if let Some(v) = imm {
            code.extend(v.to_le_bytes());
        }
    }
    code
}

/// Jumps from the function at `f` (ffx.sym) to code written at `at`.
fn hook_to(o: &mut Oracle, f: u32, at: u32, code: &[u8]) {
    let at = o.rebase(at);
    o.write(at, code).unwrap();
    let entry = o.rebase(f);
    let rel = at.wrapping_sub(entry + 5);
    o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
        .unwrap();
}

/// Group E: the camera natives, their camera, the channel setters, the
/// blur check, `MsDivSqrt`.
fn natives(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use ffx_battle::camera_native::bridge as n;
    let _w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x600);
    let words_at = scratch(a, b, 0x80);
    let cursor = rec;
    // the setters and the script's lookups: recording stubs (count,
    // arguments, result), 0x40 bytes of `rec` each from 0x40
    let recorded: [(u32, u8, u32); 15] = [
        (ffx_battle::atel::bridge::ATEL_GET_ACTOR_ID, 1, 0),
        (n::ATEL_GET_EVENT_MAP_CAMERA_ID, 0, 0),
        (n::ATEL_GET_REAL_VAR_ADRS, 2, 0),
        (ffx_battle::motion::bridge::ATEL_GET_BATTLE_ID_ADRS2, 1, 0),
        (n::MS_CAMERA_MOVE_ACC, 7, 0x31),
        (n::MS_CAMERA_SHAKE, 8, 0x32),
        (n::MS_SCREEN_SHAKE, 7, 0x33),
        (n::MS_CAMERA_ADD_LINE_FILTER, 8, 0x34),
        (n::MS_CAMERA_MOVE_DIVIDE, 6, 0x35),
        (n::MS_CAMERA_SET_AIM_POLAR, 8, 0x36),
        (n::MS_CAMERA_SET_AIM_POLAR2, 9, 0x37),
        (n::MS_CAMERA_SET_ATEL_YAW, 6, 0x38),
        (n::MS_CAMERA_SET_SPLINE, 6, 0x39),
        (n::MS_CAMERA_SET_BTL, 10, 0x3a),
        (n::MS_CAMERA_REF_ADD_CIRCLE_AREA, 6, 0x3b),
    ];
    let slot = |k: u32| rec + 0x40 + 0x40 * k;
    let result = |k: u32| slot(k) + 4 + 4 * u32::from(recorded[k as usize].1);
    for o in [&mut *a, &mut *b] {
        for (k, (f, args, r)) in (0u32..).zip(recorded) {
            // (up to 10 arguments: 0x70 bytes of code each, 12 in
            // MsBtlPosPrintBin, the rest in FUN_007c1790, also unreachable)
            let at = if k < 12 {
                STUBS_D + 0x70 * k
            } else {
                STUBS_E + 0x70 * (k - 12)
            };
            recording_stub(o, f, at, slot(k), args, r);
        }
        // the pops: one words_at of words for both kinds, a cursor at `rec`
        let int = code_with(&[
            (&[0xa1], Some(cursor)),
            (&[0x83, 0x05], Some(cursor)),
            (&[0x04, 0x8b, 0x00, 0xc3], None),
        ]);
        hook_to(
            o,
            ffx_battle::motion::bridge::ATEL_POP_STACK_INTEGER,
            STUBS_E + 0x70 * 3,
            &int,
        );
        let float = code_with(&[
            (&[0xa1], Some(cursor)),
            (&[0x83, 0x05], Some(cursor)),
            (&[0x04, 0xd9, 0x00, 0xc3], None),
        ]);
        hook_to(
            o,
            n::ATEL_POP_STACK_FLOAT,
            STUBS_E + 0x70 * 3 + 0x20,
            &float,
        );
        // MsCameraSetAtelFocus: its four words and the vector it points at
        let focus_rec = rec + 0x500;
        let mut parts: Vec<(&[u8], Option<u32>)> = vec![(&[0xff, 0x05], Some(focus_rec))];
        let loads: [[u8; 4]; 4] = [
            [0x8b, 0x44, 0x24, 4],
            [0x8b, 0x44, 0x24, 8],
            [0x8b, 0x44, 0x24, 0xc],
            [0x8b, 0x44, 0x24, 0x10],
        ];
        for (k, l) in (0u32..).zip(&loads) {
            parts.push((l, None));
            parts.push((&[0xa3], Some(focus_rec + 4 + 4 * k)));
        }
        parts.push((&[0x8b, 0x4c, 0x24, 0x14], None));
        let fields: [[u8; 3]; 3] = [[0x8b, 0x41, 0], [0x8b, 0x41, 4], [0x8b, 0x41, 8]];
        for (k, l) in (0u32..).zip(&fields) {
            parts.push((l, None));
            parts.push((&[0xa3], Some(focus_rec + 0x14 + 4 * k)));
        }
        parts.push((&[0xa1], Some(focus_rec + 0x20)));
        parts.push((&[0xc3], None));
        let focus = code_with(&parts);
        assert!(focus.len() <= 0x80);
        hook_to(o, n::MS_CAMERA_SET_ATEL_FOCUS, STUBS_E + 0x70 * 4, &focus);
        // Sg_Blur
        recording_stub(
            o,
            ffx_battle::damage_end::bridge::SG_BLUR,
            STUBS_E + 0x70 * 5,
            rec + 0x540,
            1,
            0,
        );
    }
    let worker = scratch(a, b, 0x100);
    let stack = scratch(a, b, 0x10);
    let local = scratch(a, b, 0x10);
    let actor_byte = scratch(a, b, 4);
    let spline_var = scratch(a, b, 0x10);
    let point = scratch(a, b, 0x10);
    // round 31: MsAtelCameraSetBtlPc_007badb0's callees, recording stubs
    // (results at `pc_res(k, args)`) and Ch_SetHideBattle logging
    let pc_rec = scratch(a, b, 0x200);
    let pc_callees: [(u32, u8); 5] = [
        (n::MS_GET_BATTLE_SCENE, 0),
        (n::MAGIC_FILE_GET_MAGIC_ID, 0),
        (n::MAGIC_FILE_GET_HANDLE, 0),
        (n::GRAPHIC_LEVEL_GET_NAME_HASH, 0),
        (n::CH_GET_ID, 1),
    ];
    let pc_result = |k: u32| pc_rec + 0x20 * k + 4 + 4 * u32::from(pc_callees[k as usize].1);
    for o in [&mut *a, &mut *b] {
        for (k, (f, args)) in (0u32..).zip(pc_callees) {
            recording_stub(o, f, STUBS_E + 0x2a0 + 0x20 * k, pc_rec + 0x20 * k, args, 0);
        }
        logging_stub(
            o,
            n::CH_SET_HIDE_BATTLE,
            STUBS_E + 0x2a0 + 0xa0,
            pc_rec + 0xa0,
            2,
            0,
        );
    }
    for round in 0..400 {
        // the worker's camera: none yet or one
        let work = [0, rng.next(), 0x0001_0100 | rng.below(3)][rng.below(3) as usize];
        poke(a, b, worker + 0x3c, &work.to_le_bytes());
        // the script's camera table: some cameras got
        fill(
            a,
            b,
            rng,
            ffx_battle::minor::bridge::ATEL_CAMERA.ptr(b).addr,
            0x30,
        );
        for k in 0..12u32 {
            if rng.below(2) == 0 {
                poke(
                    a,
                    b,
                    ffx_battle::minor::bridge::ATEL_CAMERA.ptr(b).addr + 4 * k,
                    &[0; 4],
                );
            }
        }
        // the actor and the event map's camera: the same or not
        let actor = rng.below(4);
        poke(a, b, result(0), &actor.to_le_bytes());
        poke(
            a,
            b,
            result(1),
            &[actor, rng.below(4)][rng.below(2) as usize].to_le_bytes(),
        );
        poke(
            a,
            b,
            result(2),
            &[0, spline_var][rng.below(3).min(1) as usize].to_le_bytes(),
        );
        poke(a, b, result(3), &actor_byte.to_le_bytes());
        fill(a, b, rng, actor_byte, 1);
        // the stack's words: floats or integers, read as the pops ask
        let words: Vec<u8> = (0..0x20)
            .flat_map(|_| {
                match rng.below(4) {
                    0 => rng.range(-3, 400).cast_unsigned(),
                    1 => [0, 1, 2, 3][rng.below(4) as usize],
                    _ => float(rng),
                }
                .to_le_bytes()
            })
            .collect();
        poke(a, b, words_at, &words);
        poke(a, b, cursor, &words_at.to_le_bytes());
        // the built-in splines' counts
        for k in 0..2u32 {
            poke(
                a,
                b,
                a.rebase(n::SPLINE_COUNTS) + 4 * k,
                &rng.range(0, 6).to_le_bytes(),
            );
        }
        let (side, x) = (
            rng.below(3),
            [rng.below(4), rng.next()][rng.below(2) as usize],
        );
        let short = [rng.below(4), 0xffff_ffff, rng.next()][rng.below(3) as usize];
        let mut c = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
        let mut fc = |m: &mut Oracle, f: u32, args: &[u32]| {
            let r = m.call(f, ffx_oracle::Conv::Cdecl, args).unwrap();
            battle_common::crt::f80_to_f32(r.st0_raw.unwrap())
        };
        macro_rules! nat {
            ($name:expr, $sym:expr, $args:expr, |$m:ident, $cc:ident| $body:block) => {{
                twin($name, a, b, $sym, $args, ALL, |$m| {
                    let mut $cc = n::Calls {
                        call: &mut c,
                        fcall: &mut fc,
                    };
                    $body
                });
            }};
        }
        match round % 12 {
            0 => {
                let (kind, cm) = (rng.below(5), rng.below(4));
                nat!(
                    "MsAtelGetCamera",
                    0x007b_aa50,
                    &[worker, kind, cm],
                    |m, cc| { n::ms_atel_get_camera(m, &mut cc, worker, kind, cm) }
                );
            }
            1 => nat!(
                "MsAtelCameraMoveAcc",
                0x007b_a5b0,
                &[worker, 0, stack, side, x],
                |m, cc| { n::ms_atel_camera_move_acc(m, &mut cc, worker, stack, side, x) }
            ),
            2 => nat!(
                "MsAtelCameraShake",
                0x007b_b8c0,
                &[worker, 0, stack, side, x],
                |m, cc| { n::ms_atel_camera_shake(m, &mut cc, worker, stack, side, x) }
            ),
            3 => nat!(
                "MsAtelScreenShake",
                0x007b_b840,
                &[worker, 0, stack, x],
                |m, cc| { n::ms_atel_screen_shake(m, &mut cc, worker, stack, x) }
            ),
            4 => nat!(
                "MsAtelCameraLineFilter",
                0x007b_b2b0,
                &[worker, 0, stack, side, x],
                |m, cc| { n::ms_atel_camera_line_filter(m, &mut cc, worker, stack, side, x) }
            ),
            5 => nat!(
                "MsAtelCameraMoveDivide",
                0x007b_b520,
                &[worker, 0, stack, side],
                |m, cc| { n::ms_atel_camera_move_divide(m, &mut cc, worker, stack, side) }
            ),
            6 => nat!(
                "MsAtelCameraEyeAddArea",
                0x007b_b410,
                &[worker, 0, stack, side, short],
                |m, cc| { n::ms_atel_camera_eye_add_area(m, &mut cc, worker, stack, side, short) }
            ),
            7 => {
                fill(a, b, rng, local, 12);
                nat!(
                    "MsAtelCameraFocus",
                    0x007b_ab90,
                    &[worker, 0, stack, side],
                    |m, cc| { n::ms_atel_camera_focus(m, &mut cc, worker, stack, side, local) }
                );
                // (the original's vector is in its frame)
                let v = b.read(local, 12).unwrap();
                a.write(local, &v).unwrap();
            }
            8 => nat!(
                "MsAtelCameraYaw",
                0x007b_ac40,
                &[worker, 0, stack, side],
                |m, cc| { n::ms_atel_camera_yaw(m, &mut cc, worker, stack, side) }
            ),
            9 => {
                // the spline popped last: 0, 1 or a variable
                poke(
                    a,
                    b,
                    words_at + 12,
                    &[0u32, 1, 2, 7][rng.below(4) as usize].to_le_bytes(),
                );
                nat!(
                    "MsAtelCameraSpline",
                    0x007b_b970,
                    &[worker, 0, stack, side],
                    |m, cc| { n::ms_atel_camera_spline(m, &mut cc, worker, stack, side) }
                );
            }
            10 => nat!(
                "MsAtelCameraSetBtl",
                0x007b_ad10,
                &[worker, 0, stack, side],
                |m, cc| { n::ms_atel_camera_set_btl(m, &mut cc, worker, stack, side) }
            ),
            11 => nat!(
                "MsAtelCameraRefAddCircleArea",
                0x007b_b370,
                &[worker, 0, stack, side, x],
                |m, cc| {
                    n::ms_atel_camera_ref_add_circle_area(m, &mut cc, worker, stack, side, x)
                }
            ),
            _ => {}
        }
        same(&format!("natives {}", round % 12), a, b);
        // round 31: the direction vector native and its setter, on a camera
        // whose current request is the worker's camera or not
        poke(a, b, cursor, &words_at.to_le_bytes());
        let cam = [0x0001_0100 | rng.below(3), rng.next()][rng.below(4).min(1) as usize];
        if rng.below(4) != 0 {
            poke(a, b, worker + 0x3c, &cam.to_le_bytes());
        }
        dir_vector_camera(a, b, rng, cam & 0xff, cam, point);
        if round % 3 == 2 {
            // six scenarios a round (each fix's conditions one at a time)
            for _ in 0..6 {
                poke(a, b, cursor, &words_at.to_le_bytes());
                let (side, mode) = pc_camera(a, b, rng, [worker, words_at], &pc_result);
                nat!(
                    "MsAtelCameraSetBtlPc_007badb0",
                    0x007b_adb0,
                    &[worker, 0, stack, side, mode],
                    |m, cc| { n::ms_atel_camera_set_btl_pc(m, &mut cc, worker, stack, side, mode) }
                );
                same("MsAtelCameraSetBtlPc_007badb0", a, b);
            }
        } else if round % 3 == 0 {
            // (its EAX, a camera ID or MsVectorNormal2's, is read by no
            // caller: both natives return 0)
            twin(
                "MsAtelCameraDirVector_007bb210",
                a,
                b,
                0x007b_b210,
                &[worker, 0, stack, side],
                0,
                |m| {
                    let mut cc = n::Calls {
                        call: &mut c,
                        fcall: &mut fc,
                    };
                    n::ms_atel_camera_dir_vector(m, &mut cc, worker, stack, side, 0);
                    0
                },
            );
            // the scale's third word: the original's frame pointer
            vf2_word(a, b, 8);
        } else {
            let args = [cam, side, float(rng), float(rng), float(rng)];
            let r = a.call(a.rebase(0x007b_ce80), Conv::Cdecl, &args).unwrap();
            assert!(r.st0.is_none());
            // the stray words: the harness's esi and ebp, its return
            // address (the oracle's sentinel, taken from `a`)
            let vf2 = ffx_battle::vector::bridge::VF_M.ptr(a).addr + 0x20;
            let ret = u32::from_le_bytes(a.read(vf2 + 12, 4).unwrap().try_into().unwrap());
            ffx_battle::camera::bridge::ms_camera_set_dir_vector(
                b,
                args[0],
                args[1],
                args[2],
                args[3],
                args[4],
                [0x2222_2222, 0x4444_4444, ret],
            );
        }
        same(&format!("dir vector {}", round % 3), a, b);
    }
}

/// Camera `k`'s current request `id` (slot 0 live, the others not) or not
/// (a quarter of the time), both sides' positions and the point `+0xbc`
/// points at (`point`, a vector of floats) or null when not live.
fn dir_vector_camera(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    k: u32,
    id: u32,
    point: u32,
) {
    use ffx_battle::camera::bridge as cam;
    if k > 2 {
        return;
    }
    let slots = cam::SLOTS_AT.ptr(b).addr + k * cam::CAMERA_SIZE;
    let live = rng.below(4) != 0;
    let mut bytes = vec![0u8; 0x20];
    bytes[..4].copy_from_slice(&(if live { id } else { id ^ 0x100 }).to_le_bytes());
    bytes[4] = u8::from(rng.below(8) == 0);
    for s in 1..4 {
        bytes[8 * s + 5] = 1;
    }
    poke(a, b, slots, &bytes);
    let base = cam::CAMERAS.ptr(b).addr + k * cam::CAMERA_SIZE;
    poke(
        a,
        b,
        point,
        &(0..4)
            .flat_map(|_| float(rng).to_le_bytes())
            .collect::<Vec<u8>>(),
    );
    for side in 0..2 {
        let s = base + side * 0x61c;
        poke(
            a,
            b,
            s + 0x50,
            &(0..4)
                .flat_map(|_| float(rng).to_le_bytes())
                .collect::<Vec<u8>>(),
        );
        poke(a, b, s + 0xbc, &point.to_le_bytes());
    }
}

/// The state `MsAtelCameraSetBtlPc_007badb0`'s fixes look at, one fix's
/// whole scenario at a time (scene, magic, side, mode and the popped
/// `dist` and `x`, words 0 and 3, at the values it compares with), one of
/// its conditions broken half the time: the callees' results
/// (`res(k)`: scene, magic, magic file handle, level hash, `Ch_GetId`),
/// the command actor and its character word, the hide queue. Returns
/// `(side, mode)`. (The 0xd40000, 0x1a30004 and 0x1f60004 fixes compare
/// with products only single precision makes: never met.)
fn pc_camera(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    [worker, words_at]: [u32; 2],
    res: &dyn Fn(u32) -> u32,
) -> (u32, u32) {
    use ffx_battle::visible::bridge as vis;
    // scene, magic, side, mode, dist * 100, x * 100
    let scenarios: [(u32, u32, u32, u32, f32, f32); 12] = [
        (0x00d4_0000, 0xfb, 0, 0, -150.0, 88.99999),
        (0x01a3_0004, 0x58, 0, 0, 228.99997, 40.0),
        (0x01f6_0004, 0xc9, 0, 0, 300.0, 3.999_999),
        (0x025c_0013, 0x4c, 1, 2, 150.0, 40.0),
        (0x025c_0013, 0x137, 1, 2, 150.0, 40.0),
        (0x0159_0000, 0xa3, 1, 2, 520.0, 40.0),
        (0x0001_0000, 0x2c1, 1, 1, 3050.0, 40.0),
        (0x0001_0000, 0x2c1, 1, 1, 2860.0, 40.0),
        (0x0001_0000, 0x2c0, 1, 1, 1000.0, 40.0),
        // below both bounds: truncations of -3 and -2 (their sign matters)
        (0x0001_0000, 0x2c1, 1, 1, 2700.0, 40.0),
        (0x0001_0000, 0x2c0, 1, 1, 800.0, 40.0),
        (0x01ae_000a, 0x92, 0, 1, 1000.0, 18.5),
    ];
    let (mut scene, mut magic, mut side, mut mode, mut dist, mut x) =
        scenarios[rng.below(12) as usize];
    let mut handle = 3;
    let mut hash = 0x200f;
    let mut id = 0x10c1;
    let mut who = u8::try_from(rng.below(ACTORS)).unwrap();
    let mut chr = rng.next() | 1;
    if rng.below(2) == 0 {
        match rng.below(10) {
            0 => scene ^= 1 << rng.below(28),
            1 => magic ^= 1 << rng.below(10),
            2 => side ^= 1 + rng.below(2),
            3 => mode ^= 1 + rng.below(3),
            4 => dist = f32::from_bits(dist.to_bits() ^ (1 << rng.below(31))),
            5 => x = f32::from_bits(x.to_bits() ^ (1 << rng.below(31))),
            6 => handle = u32::MAX,
            7 => hash ^= 1,
            8 => id ^= 1 << rng.below(16),
            _ => {
                if rng.below(2) == 0 {
                    who = 0xff;
                } else {
                    chr = u32::MAX;
                }
            }
        }
    }
    for (k, v) in [scene, magic, handle, hash, id].into_iter().enumerate() {
        poke(a, b, res(u32::try_from(k).unwrap()), &v.to_le_bytes());
    }
    set(a, b, ffx_battle::actor::bridge::COMMAND_ACTOR, who);
    if who != 0xff {
        let p = ffx_battle::actor::bridge::ms_get_chr(b, u32::from(who));
        poke(a, b, p, &chr.to_le_bytes());
    }
    set(
        a,
        b,
        vis::HIDE_QUEUE_LAST,
        [0i32, 1, 3][rng.below(3) as usize],
    );
    let q = vis::HIDE_QUEUE.ptr(b).addr;
    for k in 0..4 {
        poke(a, b, q + 4 * k, &rng.next().to_le_bytes());
    }
    // the worker's camera: none yet (MsAtelGetCameraID kind 2) half the time
    if rng.below(2) == 0 {
        poke(a, b, worker + 0x3c, &[0; 4]);
    }
    poke(a, b, words_at, &dist.to_bits().to_le_bytes());
    poke(a, b, words_at + 12, &x.to_bits().to_le_bytes());
    (side, mode)
}

/// Copies `VF2`'s word at `off` from `a` to `b`.
fn vf2_word(a: &mut Oracle, b: &mut Oracle, off: u32) {
    let at = ffx_battle::vector::bridge::VF_M.ptr(a).addr + 0x20 + off;
    let w = a.read(at, 4).unwrap();
    b.write(at, &w).unwrap();
}

#[test]
fn leaf_natives_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    for o in [&mut a, &mut b] {
        battle_common::crt::patch(o);
    }
    natives(&mut a, &mut b, &mut Xorshift(0x2929_0e01), true);
}

#[test]
fn leaf_natives_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    for o in [&mut a, &mut b] {
        battle_common::crt::patch(o);
    }
    natives(&mut a, &mut b, &mut Xorshift(0x2929_0e02), false);
}

/// Group E's camera setters on random slots and channels, the blur check
/// (`Sg_Blur` a recording stub) and `MsDivSqrt`.
fn camera_setters(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_battle::camera::bridge as cam;
    let rec = scratch(a, b, 0x10);
    let local = scratch(a, b, 0x10);
    for o in [&mut *a, &mut *b] {
        recording_stub(
            o,
            ffx_battle::damage_end::bridge::SG_BLUR,
            STUBS_D,
            rec,
            1,
            0,
        );
    }
    let mut c = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
    let cameras = cam::CAMERAS.ptr(b).addr;
    for _ in 0..400 {
        // the camera's own setters on random slots and channels
        let k = rng.below(3);
        let base = cameras + k * cam::CAMERA_SIZE;
        fill(a, b, rng, base + 0xc68, 0x20);
        let id = k
            | [rng.below(4), 0x80 | rng.below(4)][rng.below(2) as usize] << 8
            | [0, rng.below(4) + 1][rng.below(2) as usize] << 16;
        for s in 0..4 {
            if rng.below(2) == 0 {
                poke(a, b, base + 0xc68 + 8 * s, &id.to_le_bytes());
                poke(
                    a,
                    b,
                    base + 0xc68 + 8 * s + 4,
                    &[u8::from(rng.below(4) == 0), u8::from(rng.below(4) == 0), 0],
                );
            }
        }
        for side in 0..2 {
            for ch in [0, 1, 2, 3, 0x80, 0x81, 0x82, 0x83] {
                let at = base + side * 0x61c + ch * 0x130 + 0xf0;
                poke(a, b, at + 4, &[u8::try_from(rng.below(7)).unwrap()]);
            }
        }
        let (side, on) = (rng.next(), rng.next());
        twin(
            "MsCameraSetChannelFlagF5",
            a,
            b,
            0x007b_f1a0,
            &[id, side, on],
            0,
            |m| {
                cam::ms_camera_set_channel_flag_f5(m, id, side, on);
                0
            },
        );
        let area = [float(rng), float(rng), float(rng)];
        let kind = [0, 1, rng.next()][rng.below(3) as usize];
        twin(
            "MsCameraRefAddCircleArea",
            a,
            b,
            0x007b_f1f0,
            &[id, side, kind, area[0], area[1], area[2]],
            0,
            |m| {
                cam::ms_camera_ref_add_circle_area(m, id, side, kind, area);
                0
            },
        );
        let weight = rng.next();
        twin(
            "MsCameraBlurIfCurrent",
            a,
            b,
            0x007b_cdd0,
            &[id, weight],
            0,
            |m| {
                cam::ms_camera_blur_if_current(m, &mut c, id, weight);
                0
            },
        );
        let (x, y) = (
            float(rng),
            [0, 0x8000_0000, float(rng)][rng.below(3) as usize],
        );
        twin("MsDivSqrt", a, b, 0x007c_13c0, &[local, x, y], 0, |m| {
            ffx_battle::misc::bridge::ms_div_sqrt(m, local, x, y);
            0
        });

        same("camera setters", a, b);
    }
}

#[test]
fn leaf_natives_camera_setters_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    for o in [&mut a, &mut b] {
        battle_common::crt::patch(o);
    }
    camera_setters(&mut a, &mut b, &mut Xorshift(0x2929_0f01));
}

#[test]
fn leaf_natives_camera_setters_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    for o in [&mut a, &mut b] {
        battle_common::crt::patch(o);
    }
    camera_setters(&mut a, &mut b, &mut Xorshift(0x2929_0f02));
}

/// Group F: character IDs, command use, the CTB list.
fn turns(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use ffx_battle::turn::bridge as t;
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x20);
    for o in [&mut *a, &mut *b] {
        recording_stub(o, t::ATEL_GET_ALBHED_RIKKU, STUBS_D, rec, 0, 0);
    }
    let chr_rec = scratch(a, b, 0x20);
    let out = scratch(a, b, 4);
    let frame = scratch(a, b, t::CTB_FRAME as usize);
    let menu = ffx_battle::misc::bridge::MENU_COMMAND.ptr(b).addr;
    for round in 0..300 {
        if w.synthetic || round % 2 == 0 {
            dress(a, b, rng, w.actors);
            for k in 0..ACTORS {
                let p = w.actor(k);
                for (at, len) in [
                    (0x613, 2),
                    (0x606, 1),
                    (0x65c, 1),
                    (0x690, 0x10),
                    (0xdc8, 1),
                    (0xdcc, 2),
                    (0xdd4, 4),
                ] {
                    fill(a, b, rng, p + at, len);
                }
                // mostly in the order, the CTB small
                for o in [0xdc8u32, 0xdd7] {
                    poke(a, b, p + o, &[u8::from(rng.below(5) != 0)]);
                }
                poke(a, b, p + 0xdcc, &[u8::from(rng.below(6) == 0)]);
                poke(a, b, p + 0x65c, &[u8::try_from(rng.below(40)).unwrap()]);
            }
        }
        if w.synthetic {
            for k in 0..ACTORS {
                poke(a, b, w.actor(k), &[0; 4]);
            }
        }
        // a character record
        let id = [
            rng.below(0x14),
            6,
            0x1000 | rng.below(0x200),
            0x2000 | rng.below(8),
            rng.next() & 0xffff,
        ][rng.below(5) as usize];
        fill(a, b, rng, chr_rec, 0x20);
        poke(
            a,
            b,
            chr_rec + 0xe,
            &u16::try_from(id & 0xffff).unwrap().to_le_bytes(),
        );
        poke(a, b, rec + 4, &rng.below(2).to_le_bytes());
        let mut c = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
        twin("MsGetChrID", a, b, 0x0079_a440, &[chr_rec], ALL, |m| {
            t::ms_get_chr_id(m, &mut c, chr_rec)
        });
        // command IDs of every kind
        let kind = [0, 2, 3, 3, 1, rng.below(16)][rng.below(6) as usize];
        let cid = kind << 12 | [rng.below(0x22), rng.below(0x100), 0xff][rng.below(3) as usize];
        let cid = if rng.below(8) == 0 { rng.next() } else { cid };
        let (chr, on) = (rng.below(0x22), rng.below(2) * rng.next());
        twin(
            "MsSetCommandUse",
            a,
            b,
            0x0079_b470,
            &[chr, cid, on],
            0,
            |m| {
                t::ms_set_command_use(m, chr, cid, on);
                0
            },
        );
        set(
            a,
            b,
            ffx_battle::save::bridge::USE_ITEM,
            [u16::try_from(cid & 0xffff).unwrap(), 0x2001][rng.below(2) as usize],
        );
        // Kimahri's limit (0x30ff): actors 0xf-0x11 able or not, one at a time
        if cid == 0x30ff {
            let pick = rng.below(4);
            for (k, i) in (0u32..).zip(0xf..0x12u32) {
                let p = ffx_battle::actor::bridge::ms_get_chr(b, i);
                let unable = pick == k;
                poke(
                    a,
                    b,
                    p + 0x5d0,
                    &(if unable { 0i32 } else { 100 }).to_le_bytes(),
                );
                poke(a, b, p + 0xdd4, &[1]);
                poke(a, b, p + 0xdcd, &[0]);
                poke(a, b, p + 0x6d8, &[0]);
            }
        }
        twin(
            "MsGetCommandUse",
            a,
            b,
            0x0079_a5b0,
            &[chr, cid],
            ALL,
            |m| t::ms_get_command_use(m, chr, cid),
        );
        if round % 2 == 0 {
            // a menu command now and then: its actor, flags, commands
            let on = [0u8, 0, 1, 2, 0x80][rng.below(5) as usize];
            set(a, b, ffx_battle::misc::bridge::MENU_COMMAND_SET, on);
            fill(a, b, rng, menu, 0x48);
            let n = u8::try_from(rng.below(4)).unwrap();
            poke(
                a,
                b,
                menu,
                &[
                    u8::try_from(rng.below(ACTORS)).unwrap(),
                    0,
                    u8::from(rng.below(2) == 0),
                    n,
                ],
            );
            for k in 0..3 {
                let ids = [0x3000 | rng.below(0x40), 0x3000 | rng.below(0x40)];
                poke(
                    a,
                    b,
                    menu + 8 + 16 * k,
                    &[
                        u16::try_from(ids[0]).unwrap().to_le_bytes(),
                        u16::try_from(ids[1]).unwrap().to_le_bytes(),
                    ]
                    .concat(),
                );
            }
            // small base counts and command ranks now and then: with the
            // tables' 255 every rank clamps to 255 (MsAdjustCTB), and the
            // menu command's rank reaching MsCalcDamageCTB goes unseen
            if rng.below(2) == 0 {
                for k in 0..ACTORS {
                    let agility = i32::from(b.read(w.actor(k) + 0x5ac, 1).unwrap()[0]);
                    let r = ffx_kernel::bridge::ms_get_rom_ctb_base(b, agility);
                    if readable(a, r, 1) {
                        poke(a, b, r, &[u8::try_from(rng.range(1, 30)).unwrap()]);
                    }
                }
                for k in 0..3 {
                    for j in 0..2 {
                        let id = u32::from(u16::from_le_bytes(
                            b.read(menu + 8 + 16 * k + 2 * j, 2)
                                .unwrap()
                                .try_into()
                                .unwrap(),
                        ));
                        let c = ffx_kernel::bridge::rom(b, ffx_kernel::bridge::COMMAND, id, 0);
                        if readable(a, c, 0x40) {
                            poke(a, b, c + 0x24, &[[0u8, 1, 2, 4, 5][rng.below(5) as usize]]);
                            poke(a, b, c + 0x39, &[u8::from(rng.below(3) != 0)]);
                        }
                    }
                }
            }
            fill(a, b, rng, frame, t::CTB_FRAME);
            // sometimes nobody in the order (the list runs out)
            if rng.below(6) == 0 {
                for k in 0..ACTORS {
                    if rng.below(8) != 0 {
                        poke(a, b, w.actor(k) + 0xdc8, &[0]);
                    }
                }
            }
            let p = [out, 0][rng.below(4).min(1) as usize];
            twin("MsGetCTBlist", a, b, 0x0079_a200, &[p], ALL, |m| {
                t::ms_get_ctb_list(m, p, frame)
            });
            // (the original's locals are in its frame)
            let f = b.read(frame, t::CTB_FRAME as usize).unwrap();
            a.write(frame, &f).unwrap();
        }
        same("turns", a, b);
    }
}

#[test]
fn leaf_turns_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    turns(&mut a, &mut b, &mut Xorshift(0x2929_1001), true);
}

#[test]
fn leaf_turns_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    turns(&mut a, &mut b, &mut Xorshift(0x2929_1002), false);
}

/// Group G: the command window, the party, encounter weights, shadows,
/// transparency, equipment lists, points moved by offsets, the bound
/// camera.
fn lists(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x60);
    let extra = scratch(a, b, 0x24);
    let objs = scratch(a, b, 0x20);
    // MesSetChrTargetPosToLimitSparePosy's frame words (round 32)
    let pos_local = scratch(a, b, 0x10);
    // round 32's own stream (the later checks keep theirs)
    let mut r32 = Xorshift(0x3232_0201 ^ u32::from(synthetic));
    for o in [&mut *a, &mut *b] {
        recording_stub(o, g::MS_GET_SAVE_PLY_LEFT, STUBS_D, rec, 1, 0);
        // yiSetRotTransDObj (field)
        recording_stub(
            o,
            g::YI_SET_ROT_TRANS_DOBJ,
            STUBS_D + 0xc0,
            rec + 0x30,
            3,
            0,
        );
        // the worker's actor and the event map's camera
        recording_stub(
            o,
            ffx_battle::atel::bridge::ATEL_GET_ACTOR_ID,
            STUBS_D + 0x40,
            rec + 0x10,
            1,
            0,
        );
        recording_stub(
            o,
            ffx_battle::camera_native::bridge::ATEL_GET_EVENT_MAP_CAMERA_ID,
            STUBS_D + 0x80,
            rec + 0x20,
            0,
            0,
        );
    }
    // the command tables (0x40 records), the second menu lists
    if w.synthetic {
        let t = fake_records(a, b, rng, 0x140, 0x60);
        set(a, b, ffx_kernel::bridge::COMMAND, t);
        if !readable(a, ffx_battle::actor::bridge::MON_TOP.get(b), 0x10) {
            set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
        }
    }
    let mem2 = scratch(a, b, 0x5400);
    set(a, b, ffx_battle::misc::bridge::ITEM_MEM2, mem2);
    let local = scratch(a, b, 4);
    // an encounter table of our own (as in leaf_accessors)
    let fields = scratch(a, b, 14);
    poke(a, b, fields, &7u16.to_le_bytes());
    set(a, b, ffx_battle::state::bridge::FIELD_RECORDS, fields);
    set(a, b, ffx_battle::minor::bridge::FIELD_COUNT, 1);
    let encounters = scratch(a, b, 0x80);
    set(a, b, ffx_battle::misc::bridge::FIELD_TABLE2, encounters);
    let base = scratch(a, b, 0x400);
    let other = scratch(a, b, 0x800);
    let worker = scratch(a, b, 0x100);
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    let equipment = g::EQUIPMENT.ptr(b).addr;
    for round in 0..300 {
        if w.synthetic || round % 2 == 0 {
            dress(a, b, rng, w.actors);
            for k in 0..ACTORS {
                let p = w.actor(k);
                for (at, len) in [
                    (0x40, 4),
                    (0x423, 1),
                    (0x5bc, 2),
                    (0xc, 1),
                    (0xde5, 1),
                    (0xded, 1),
                    (0xdf0, 1),
                ] {
                    fill(a, b, rng, p + at, len);
                }
                poke(a, b, p + 0xded, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
            }
        }
        if w.synthetic {
            // no characters; followers of earlier actors only
            for k in 0..ACTORS {
                let p = w.actor(k);
                poke(a, b, p, &[0; 4]);
                poke(a, b, p + 0x13, &[0xff]);
                let owner = if k == 0 || rng.below(3) != 0 {
                    0xff
                } else {
                    rng.below(k)
                };
                poke(
                    a,
                    b,
                    p + 0x43f,
                    &[u8::try_from(owner).unwrap(), u8::from(rng.below(4) == 0)],
                );
            }
        }
        let i = rng.below(0x12);
        // the menu commands: none, items, commands
        for k in 0..8u32 {
            let id =
                [0xff, 0x3000 | rng.below(0x40), 0x2000 | rng.below(0x40)][rng.below(3) as usize];
            poke(
                a,
                b,
                mem2 + i * 0x478 + 0x28 + 2 * k,
                &u16::try_from(id).unwrap().to_le_bytes(),
            );
        }
        set(
            a,
            b,
            g::LEFT_WINDOW_RESET,
            [0, 1, 2, 0x100][rng.below(4) as usize],
        );
        // the spare actor (MsGetChr(0xff), what command 0xff would ask)
        // able to act
        let spare = ffx_battle::actor::bridge::CHR_SPARE.ptr(b).addr;
        poke(a, b, spare + 0x5d0, &100u32.to_le_bytes());
        poke(a, b, spare + 0xdd4, &[1]);
        poke(a, b, spare + 0xdcd, &[0]);
        poke(a, b, spare + 0x6d8, &[0]);
        set(
            a,
            b,
            ffx_battle::damage::bridge::DEBUG_NO_COST,
            u8::from(rng.below(4) == 0),
        );
        twin("MsCheckLeftWindow", a, b, 0x0078_f700, &[i], ALL, |m| {
            g::ms_check_left_window(m, i)
        });
        same("MsCheckLeftWindow", a, b);
        // the party: in it, out of it, full
        let party = ffx_battle::save::bridge::ms_get_save_in_party(b, 0);
        let party_bytes: Vec<u8> = (0..0x14)
            .map(|_| [0xffu8, u8::try_from(rng.below(0x12)).unwrap()][rng.below(2) as usize])
            .collect();
        poke(a, b, party, &party_bytes);
        let (chr, on) = (
            [rng.below(0x12), rng.next()][rng.below(5).min(1) as usize],
            [0, 1, rng.next()][rng.below(3) as usize],
        );
        poke(a, b, rec + 8, &rng.below(2).to_le_bytes());
        let mut c = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
        twin(
            "MsSetSavePlyJoin",
            a,
            b,
            0x0078_69b0,
            &[chr, on],
            ALL,
            |m| g::ms_set_save_ply_join(m, &mut c, chr, on, local),
        );

        let n = b.read(local, 4).unwrap();
        a.write(local, &n).unwrap();
        same("MsSetSavePlyJoin", a, b);
        if rng.below(4) == 0 {
            twin("MsDebugFullSummon", a, b, 0x0078_4b60, &[], ALL, |m| {
                g::ms_debug_full_summon(m, &mut c, local)
            });
            let n = b.read(local, 4).unwrap();
            a.write(local, &n).unwrap();
            same("MsDebugFullSummon", a, b);
        }
        // the Zanmato list: a small count (negative, full, past full)
        let list = g::ZANMATO_SEP.ptr(b).addr;
        let ids: Vec<u8> = (0..9).flat_map(|_| rng.below(6).to_le_bytes()).collect();
        poke(a, b, list + 4, &ids);
        poke(a, b, list, &rng.range(-2, 11).to_le_bytes());
        let id = [rng.below(6), rng.next()][rng.below(5).min(1) as usize];
        twin("addMonDieSe", a, b, 0x007a_8920, &[id], ALL, |m| {
            g::add_mon_die_se(m, &mut c, id)
        });
        same("addMonDieSe", a, b);
        // encounter groups: two of up to five formations
        let groups = 1 + rng.below(2);
        let mut table = vec![0u8, u8::try_from(groups).unwrap()];
        for _ in 0..groups {
            let n = rng.below(6);
            table.extend([u8::try_from(n).unwrap(), 0, 0, 0, 0]);
            table.extend((0..2 * n).map(|_| rng.byte()));
        }
        poke(a, b, encounters, &table);
        let (map, keep) = (
            [7, 0, rng.below(0x400)][rng.below(3) as usize],
            rng.below(2),
        );
        twin(
            "Enc_ResetWeights",
            a,
            b,
            0x0078_28a0,
            &[map, keep],
            0,
            |m| {
                g::enc_reset_weights(m, map, keep);
                0
            },
        );
        same("Enc_ResetWeights", a, b);
        // its wrappers (a scene of the table's group now and then)
        // (with the state they reset set: the event effect, the encounter
        // type)
        for at in [
            ffx_battle::state::bridge::EVENT_EFFECT_STAT.ptr(b).addr,
            ffx_battle::state::bridge::EVENT_EFFECT_A.ptr(b).addr,
            ffx_battle::state::bridge::EVENT_EFFECT_B.ptr(b).addr,
            g::ENCOUNT_TYPE.ptr(b).addr,
        ] {
            fill(a, b, rng, at, 1);
        }
        match rng.below(3) {
            0 => twin("Enc_ResetAll", a, b, 0x0078_0c70, &[map], 0, |m| {
                g::enc_reset_all(m, map);
                0
            }),
            1 => twin("MsBattleInitAtEvent", a, b, 0x0078_1c50, &[map], 0, |m| {
                g::ms_battle_init_at_event(m, map);
                0
            }),
            _ => {
                let f = if table[2] > 0 && rng.below(3) != 0 {
                    u32::from(table[7])
                } else {
                    rng.below(0x40)
                };
                let scene = [map << 16 | f, rng.next()][rng.below(5).min(1) as usize];
                twin(
                    "Enc_ResetSceneWeights",
                    a,
                    b,
                    0x0078_2b40,
                    &[scene],
                    0,
                    |m| {
                        g::enc_reset_scene_weights(m, scene);
                        0
                    },
                );
            }
        }
        same("encounter wrappers", a, b);
        let f = if table[2] > 0 && rng.below(4) != 0 {
            u32::from(table[7])
        } else {
            rng.below(0x40)
        };
        let scene = [map << 16 | f, rng.next()][rng.below(6).min(1) as usize];
        let weight = rng.range(-3, 20).cast_unsigned();
        twin(
            "Enc_SetGroupWeight",
            a,
            b,
            0x0078_2ba0,
            &[scene, weight],
            ALL,
            |m| g::enc_set_group_weight(m, scene, weight),
        );
        same("Enc_SetGroupWeight", a, b);
        // affection, the water, the women
        let love: Vec<u8> = (0..7)
            .flat_map(|_| {
                [rng.range(-1, 3), rng.next().cast_signed()][rng.below(6).min(1) as usize]
                    .to_le_bytes()
            })
            .collect();
        poke(a, b, ffx_battle::save::bridge::LOVE.ptr(b).addr, &love);
        for k in 0..7 {
            let p = w.actor(k);
            poke(a, b, p + 0xdcc, &[u8::from(rng.below(5) == 0)]);
            poke(
                a,
                b,
                p + 0xdce,
                &[[0, 0, 0, 1, 0xff][rng.below(5) as usize]],
            );
        }
        set(
            a,
            b,
            ffx_battle::state::bridge::UNDERWATER,
            [0, 0, 1][rng.below(3) as usize],
        );
        twin(
            "MsEffectChrGetLoveLoveMax",
            a,
            b,
            0x007b_7d00,
            &[],
            ALL,
            |m| g::ms_effect_chr_get_love_love_max(m),
        );
        // the three extra slots: some in use, some for actors' characters
        if !readable(a, g::CHR_EXTRA.get(b), 0x24) {
            set(a, b, g::CHR_EXTRA, extra);
        }
        let e = g::CHR_EXTRA.get(b);
        for k in 0..3 {
            let chr = ffx_battle::actor::bridge::ms_get_chr(b, rng.below(ACTORS));
            let c: u32 = ffx_bridge::Mem::read(b, chr);
            poke(
                a,
                b,
                e + 0xc * k,
                &[[0, 1, 1, rng.byte()][rng.below(4) as usize]],
            );
            poke(
                a,
                b,
                e + 0xc * k + 4,
                &[c, rng.next()][rng.below(3).min(1) as usize].to_le_bytes(),
            );
        }
        let j = [rng.below(ACTORS), rng.next()][rng.below(5).min(1) as usize];
        twin("MsSetChrExtraSlot", a, b, 0x0079_6bb0, &[j], ALL, |m| {
            g::ms_set_chr_extra_slot(m, j)
        });
        same("extra slots", a, b);
        // MesSetChrTargetPosToLimitSparePosy (round 32): the optional
        // position in the frame local (seeded alike), characters only
        // where they are real (a battle snapshot's; put back after)
        let j = [r32.below(ACTORS), r32.next()][r32.below(5).min(1) as usize];
        let p = ffx_battle::actor::bridge::ms_get_chr(b, j & 0xff);
        let chr: u32 = ffx_bridge::Mem::read(b, p);
        let real = !synthetic && readable(a, chr, 0x1000);
        if !real {
            poke(a, b, p, &[0; 4]);
        }
        let opt = a.read(p + 0xdf9, 1).unwrap();
        poke(
            a,
            b,
            p + 0xdf9,
            &[[0, 1, 2, 0xff, r32.byte()][r32.below(5) as usize]],
        );
        // on a real character, set entries where (i8) and (u8) of the byte
        // point (put back after)
        let byte = a.read(p + 0xdf9, 1).unwrap()[0];
        let mut entries: Vec<(u32, Vec<u8>)> = Vec::new();
        if real {
            for k in [i32::from(byte.cast_signed()), i32::from(byte)] {
                let at = chr
                    .wrapping_add(ffx_chara::attach::bridge::OPTPOS)
                    .wrapping_add((16 * k).cast_unsigned());
                if readable(a, at, 16) {
                    entries.push((at, a.read(at, 16).unwrap()));
                }
            }
        }
        for (at, _) in &entries {
            let v: Vec<u8> = (0..4).flat_map(|_| float(&mut r32).to_le_bytes()).collect();
            poke(a, b, *at, &v);
        }
        let frame = a.stack_range().end - 0x100 - 0x20;
        let seed: Vec<u8> = (0..4).flat_map(|_| float(&mut r32).to_le_bytes()).collect();
        // (the stack put back after: later originals read what is there)
        let stack = a.read(frame, 0x10).unwrap();
        a.write(frame, &seed).unwrap();
        poke(a, b, pos_local, &seed);
        twin(
            "MesSetChrTargetPosToLimitSparePosy",
            a,
            b,
            0x0079_32b0,
            &[j],
            ALL,
            |m| g::mes_set_chr_target_pos_to_limit_spare_posy(m, j, pos_local),
        );
        let v = a.read(pos_local, 0x10).unwrap();
        b.write(pos_local, &v).unwrap();
        same(&format!("MesSetChrTargetPosToLimitSparePosy {j:#x}"), a, b);
        poke(a, b, p, &chr.to_le_bytes());
        poke(a, b, p + 0xdf9, &opt);
        for (at, v) in entries.iter().rev() {
            poke(a, b, *at, v);
        }
        a.write(frame, &stack).unwrap();
        // the scene objects (yiSetRotTransDObj a recording stub: its
        // vectors are frame ones, compared by what they hold)
        twin("MsResetSceneObjects", a, b, 0x0079_e2a0, &[], 0, |m| {
            g::ms_reset_scene_objects(m, &mut c, objs);
            0
        });
        let ra = a.read(rec + 0x30, 0x10).unwrap();
        let rb = b.read(rec + 0x30, 0x10).unwrap();
        assert_eq!(ra[..8], rb[..8], "MsResetSceneObjects: count, object");
        for k in 0..2 {
            let pa = u32::from_le_bytes(ra[8 + 4 * k..12 + 4 * k].try_into().unwrap());
            let pb = u32::from_le_bytes(rb[8 + 4 * k..12 + 4 * k].try_into().unwrap());
            assert_eq!(
                a.read(pa, 0x10).unwrap(),
                b.read(pb, 0x10).unwrap(),
                "MsResetSceneObjects vector {k}"
            );
        }
        let v = b.read(rec + 0x30, 0x10).unwrap();
        a.write(rec + 0x30, &v).unwrap();
        let v = b.read(objs, 0x20).unwrap();
        a.write(objs, &v).unwrap();
        same("MsResetSceneObjects", a, b);
        fill(a, b, rng, g::MENU_PANEL.ptr(b).addr, 0x20);
        let id = [
            0x3000 | rng.below(0x60),
            0x3060 + rng.below(0xc0),
            0x3000 | rng.below(0x1000),
            rng.next(),
            0x2000 | rng.below(0x100),
        ][rng.below(5) as usize];
        twin(
            "MsGetSaveCommandMenuPanel",
            a,
            b,
            0x0078_50b0,
            &[id],
            ALL,
            |m| g::ms_get_save_command_menu_panel(m, id),
        );
        same("MsGetSaveCommandMenuPanel", a, b);
        let on = [0, 1, rng.next()][rng.below(3) as usize];
        twin(
            "MsSetSaveCommandMenuPanel",
            a,
            b,
            0x0078_5d40,
            &[id, on],
            ALL,
            |m| g::ms_set_save_command_menu_panel(m, id, on),
        );
        same("MsSetSaveCommandMenuPanel", a, b);
        // a command again: the action, the source, the reels
        set(
            a,
            b,
            ffx_battle::misc::bridge::ACTION,
            [-1, 0, rng.next().cast_signed()][rng.below(3) as usize],
        );
        let i = [rng.below(ACTORS), rng.next()][rng.below(5).min(1) as usize];
        let p = ffx_battle::actor::bridge::ms_get_chr(b, i);
        poke(
            a,
            b,
            p + 0xded,
            &[[u8::try_from(rng.below(ACTORS)).unwrap(), 0xff][rng.below(4).min(1) as usize]],
        );
        for k in 0..ACTORS {
            poke(
                a,
                b,
                w.actor(k) + 0xc,
                &[u8::try_from(rng.below(ACTORS)).unwrap()],
            );
        }
        fill(
            a,
            b,
            rng,
            ffx_battle::misc::bridge::REEL_ENTRIES.ptr(b).addr,
            0x10,
        );
        if rng.below(2) == 0 {
            let at = ffx_battle::misc::bridge::REEL_ENTRIES.ptr(b).addr + 4 * rng.below(4) + 2;
            poke(a, b, at, &[0, 0]);
        }
        // the source's command list: one of the reels' commands, mostly
        let src = b.read(p + 0xded, 1).unwrap()[0];
        let sp = ffx_battle::actor::bridge::ms_get_chr(b, u32::from(src));
        let k = b.read(sp + 0xc, 1).unwrap()[0];
        let cmd = ffx_battle::actor::bridge::ms_get_chr_command(b, u32::from(k));
        if readable(a, cmd, 0x60) && rng.below(4) != 0 {
            let n = u32::from(b.read(cmd + 2, 1).unwrap()[0]);
            let id = u16::try_from(0x3073 + rng.below(6)).unwrap();
            poke(
                a,
                b,
                cmd + 8 + 0x10 * n,
                &[id.to_le_bytes(), 0xffu16.to_le_bytes()].concat(),
            );
        }
        twin("MsActReCommand", a, b, 0x007a_cc90, &[i], ALL, |m| {
            g::ms_act_re_command(m, i, local)
        });
        let v = b.read(local, 4).unwrap();
        a.write(local, &v).unwrap();
        same("MsActReCommand", a, b);
        let (k, v) = (rng.below(4), rng.next());
        twin(
            "MsSetChrShadowMode",
            a,
            b,
            0x0079_62d0,
            &[i, k, v],
            ALL,
            |m| g::ms_set_chr_shadow_mode(m, i, k, v),
        );
        same("MsSetChrShadowMode", a, b);
        twin("MsSetChrShadow", a, b, 0x0079_62b0, &[i, v], ALL, |m| {
            g::ms_set_chr_shadow(m, i, v)
        });
        same("MsSetChrShadow", a, b);
        if round % 3 == 0 {
            let (from, to, frames, flags) = (float(rng), float(rng), rng.below(30), rng.next());
            let j = rng.below(ACTORS);
            twin(
                "MsSetChrTransparentVisible",
                a,
                b,
                0x0079_6550,
                &[j, from, to, frames, flags],
                ALL,
                |m| g::ms_set_chr_transparent_visible(m, &mut c, j, from, to, frames, flags),
            );
            same("MsSetChrTransparentVisible", a, b);
        }
        // equipment: in use or not, worn by some
        for k in 0..200u32 {
            let at = equipment + 22 * k;
            poke(
                a,
                b,
                at + 2,
                &[
                    u8::from(rng.below(3) != 0),
                    [0u8, 2, 8, 0xa][rng.below(4) as usize],
                    u8::try_from(rng.below(0x14)).unwrap(),
                    u8::try_from(rng.below(3)).unwrap(),
                ],
            );
        }
        for c2 in 0..18 {
            poke(
                a,
                b,
                ply + c2 * 0x94 + 0x2d,
                &[
                    u8::try_from(rng.below(200)).unwrap(),
                    [
                        0xffu8,
                        u8::try_from(rng.below(200)).unwrap(),
                        u8::try_from(rng.below(8)).unwrap(),
                    ][rng.below(3) as usize],
                ],
            );
        }
        // the 0x7000 slots too
        let eq7000 = ffx_battle::save::bridge::EQUIPMENT_7000.ptr(b).addr;
        for k in 0..8u32 {
            poke(a, b, eq7000 + 22 * k + 2, &[u8::from(rng.below(3) != 0)]);
        }
        let eid = [
            rng.below(200),
            0x7000 | rng.below(8),
            0xff,
            0x1000 | rng.below(200),
        ][rng.below(4) as usize];
        twin(
            "Equip_RebuildWearers",
            a,
            b,
            0x007a_bca0,
            &[eid],
            ALL,
            |m| g::equip_rebuild_wearers(m, eid),
        );
        same("Equip_RebuildWearers", a, b);
        twin("MsBuildGearList", a, b, 0x0079_ca80, &[i], 0, |m| {
            g::ms_build_gear_list(m, i);
            0
        });
        same("MsBuildGearList", a, b);
        set(
            a,
            b,
            ffx_battle::misc::bridge::ACTION,
            [-1, 0, 3, -2][rng.below(4) as usize],
        );
        let (cid, word) = (rng.next(), rng.next());
        let j = rng.below(ACTORS);
        twin(
            "MsCueAddCommand",
            a,
            b,
            0x007a_c940,
            &[j, cid, word],
            ALL,
            |m| g::ms_cue_add_command(m, j, cid, word),
        );
        same("MsCueAddCommand", a, b);
        // points: records with offsets inside the two areas
        fill(a, b, rng, base, 0x400);
        fill(a, b, rng, other, 0x800);
        let nrec = rng.below(4);
        poke(a, b, base + 1, &[u8::try_from(nrec).unwrap()]);
        for k in 0..4u32 {
            let r = base + 32 * k;
            poke(a, b, r + 4, &[[0u8, 1, 3, 5, 0xff][rng.below(5) as usize]]);
            poke(a, b, r + 0x10, &(0x100 + 0x40 * k).to_le_bytes());
        }
        for k in 0..0x30u32 {
            poke(a, b, base + 0x100 + 4 * k, &float(rng).to_le_bytes());
        }
        let n2 = rng.below(4);
        poke(a, b, other + 1, &[u8::try_from(n2).unwrap()]);
        for j in 0..3u32 {
            let e = other + 4 + 0x60 * j;
            poke(a, b, e, &[[0u8, 2, 4, 0x80][rng.below(4) as usize]]);
            poke(a, b, e + 0xc, &(0x400 + 0x100 * j).to_le_bytes());
            for c3 in 0..3 {
                poke(a, b, e + 0x2c + 4 * c3, &float(rng).to_le_bytes());
            }
        }
        let k = [rng.below(4), rng.next()][rng.below(5).min(1) as usize];
        twin(
            "MsCopyPointsOffset",
            a,
            b,
            0x007a_beb0,
            &[base, other, k],
            0,
            |m| {
                g::ms_copy_points_offset(m, base, other, k);
                0
            },
        );
        same("MsCopyPointsOffset", a, b);
        // the bound camera
        poke(
            a,
            b,
            worker + 0x3c,
            &[0, rng.next()][rng.below(2) as usize].to_le_bytes(),
        );
        let (kind, cm) = (rng.below(6), rng.below(4));
        let actor = rng.below(3);
        poke(a, b, rec + 0x18, &actor.to_le_bytes());
        poke(
            a,
            b,
            rec + 0x24,
            &[actor, rng.below(3)][rng.below(2) as usize].to_le_bytes(),
        );
        let mut fc = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
        twin(
            "MsBindCamera",
            a,
            b,
            0x007b_a9a0,
            &[worker, kind, cm],
            ALL,
            |m| {
                let mut cc = ffx_battle::camera_native::bridge::Calls {
                    call: &mut c,
                    fcall: &mut fc,
                };
                ffx_battle::camera_native::bridge::ms_bind_camera(m, &mut cc, worker, kind, cm)
            },
        );
        same("MsBindCamera", a, b);
        same(&format!("lists {round}"), a, b);
    }
}

#[test]
fn leaf_lists_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    lists(&mut a, &mut b, &mut Xorshift(0x2929_1101), true);
}

#[test]
fn leaf_lists_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    lists(&mut a, &mut b, &mut Xorshift(0x2929_1102), false);
}

/// Group I: damage motions, AP, the system camera request, effect
/// motions, the move's frames left.
fn motions(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use battle_common::crt::{patch, stand_in};
    use ffx_battle::motion::bridge as mo;
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x110 * 10);
    let sig = scratch(a, b, 0x40);
    let sig_local = scratch(a, b, 8);
    let callees: [(u32, u8); 10] = [
        (g::MS_DAMAGE_SET_MOTION, 3),
        (g::MS_ATEL_REQUEST_EXE, 6),
        (g::MS_MOT_BIN_FRAME, 4),
        (mo::CH_RESET_MOTION, 1),
        (mo::CH_SET_SYS_MOTION, 3),
        (mo::CH_MOTION_SET_NEXT_HOKAN, 2),
        (mo::CH_MOTION_SET_HOKAN, 2),
        (mo::CH_GET_SEQ_FRAME, 1),
        (g::MS_GET_MOVE_TARGET_POS, 5),
        (ffx_battle::atel::bridge::ATEL_GET_ACTOR_ID, 1),
    ];
    for o in [&mut *a, &mut *b] {
        patch(o);
        for (k, (f, n)) in (0u32..).zip(callees) {
            if f == g::MS_GET_MOVE_TARGET_POS {
                continue;
            }
            if n > 4 {
                // (too long for a logging stub: the last call kept)
                recording_stub(o, f, STUBS_D + 0x60 * k, rec + 0x110 * k, n, 0);
            } else {
                logging_stub(o, f, STUBS_D + 0x60 * k, rec + 0x110 * k, n, 0);
            }
        }
        // MsGetMoveTargetPos: the target 1.5 away on x, 2.0 in ST0
        let target = rec + 0x110 * 8;
        let code = code_with(&[
            (&[0x8b, 0x44, 0x24, 0x10], None), // mov eax, [esp + 0x10] (out)
            (&[0xd9, 0x05], Some(target)),     // fld dword [target]
            (&[0xd9, 0x18], None),             // fstp dword [eax]
            (&[0xd9, 0x05], Some(target + 4)),
            (&[0xd9, 0x58, 0x04], None),
            (&[0xd9, 0x05], Some(target + 8)),
            (&[0xd9, 0x58, 0x08], None),
            (&[0xd9, 0x05], Some(target + 0xc)), // the ST0 result
            (&[0xc3], None),
        ]);
        hook_to(o, g::MS_GET_MOVE_TARGET_POS, STUBS_E, &code);
        // MsChrSePlay_0079fd40 and MsSetDamage for the effect signals
        recording_stub(o, 0x0079_fd40, STUBS_E + 0x100, sig, 2, 0);
        recording_stub(o, 0x0078_d980, STUBS_E + 0x140, sig + 0x20, 3, 0);
    }
    let local = scratch(a, b, 0x20);
    // characters to point at (the character functions are stubs; the
    // position reads come from here)
    let chars = scratch(a, b, 0x1000);
    // the player table MsGetNextAP reads, where there is none
    if synthetic || !readable(a, ffx_battle::menu::bridge::PLY_ROM.get(b), 0x20) {
        let t = fake_records(a, b, rng, 0x14, 0x40);
        set(a, b, ffx_battle::menu::bridge::PLY_ROM, t);
    }
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    for round in 0..300 {
        if w.synthetic || round % 2 == 0 {
            dress(a, b, rng, w.actors);
            for k in 0..ACTORS {
                let p = w.actor(k);
                for (at, len) in [
                    (0x20, 1),
                    (0x38, 1),
                    (0x3f0, 1),
                    (0x3fc, 2),
                    (0x404, 1),
                    (0x415, 1),
                    (0x452, 2),
                    (0x45a, 2),
                    (0x4ac, 1),
                    (0x776, 0x5b0),
                    (0xdcc, 3),
                ] {
                    fill(a, b, rng, p + at, len);
                }
                poke(a, b, p + 0x464, &float(rng).to_le_bytes());
                poke(
                    a,
                    b,
                    p + 0x415,
                    &[[0u8, 7, 3, 0x83, 0x87, 0x80][rng.below(6) as usize]],
                );
            }
        }
        if w.synthetic {
            for k in 0..ACTORS {
                let p = w.actor(k);
                poke(
                    a,
                    b,
                    p,
                    &[0, chars + 0x100 * (k % 8)][rng.below(3).min(1) as usize].to_le_bytes(),
                );
                poke(a, b, p + 0x13, &[0xff]);
                poke(a, b, p + 0x440, &[0]);
            }
        }
        for k in 0..4 {
            poke(a, b, rec + 0x110 * 8 + 4 * k, &float(rng).to_le_bytes());
        }
        for k in 0..10u32 {
            poke(a, b, rec + 0x110 * k + 4, &rng.below(40).to_le_bytes());
        }
        // Ch_GetSeqFrame: often none left
        poke(
            a,
            b,
            rec + 0x110 * 7 + 4,
            &[0u32, 1, 5][rng.below(3) as usize].to_le_bytes(),
        );
        // MsAtelRequestExe's result (a recording stub: after its 6 words)
        poke(
            a,
            b,
            rec + 0x110 + 4 + 4 * 6,
            &rng.range(-1, 3).to_le_bytes(),
        );
        let i = [rng.below(ACTORS), rng.next()][rng.below(5).min(1) as usize];
        let mut c = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
        // damage records from actor i for its command, now and then
        let i8 = i & 0xff;
        if ffx_battle::actor::bridge::ms_btl_chr_num_check(i8) != 0 {
            let cmd = ffx_battle::actor::bridge::ms_get_chr_command(b, i8);
            let cb = b.read(cmd + 2, 1).unwrap()[0];
            for j in 0..ACTORS {
                let e = ffx_battle::actor::bridge::ms_get_chr(b, j) + 0x776 + 0x2d8 * rng.below(2);
                if rng.below(2) == 0 {
                    poke(a, b, e, &[u8::try_from(i8).unwrap(), cb]);
                }
            }
        }
        let (sel, motion) = (
            [rng.below(0x22), rng.range(-24, 0).cast_unsigned()][rng.below(2) as usize],
            rng.next(),
        );
        twin(
            "MsSetDamageMotion",
            a,
            b,
            0x0078_db70,
            &[i, sel, motion],
            0,
            |m| {
                g::ms_set_damage_motion(m, &mut c, i, sel, motion);
                0
            },
        );
        same("MsSetDamageMotion", a, b);
        // the effect signals 1 and 3-8 on the same records
        let actor = w.actor(rng.below(ACTORS));
        poke(
            a,
            b,
            actor + 0x446,
            &[[1, 4, rng.byte()][rng.below(3) as usize]],
        );
        poke(a, b, actor + 0x44a, &[u8::from(rng.below(2) == 0)]);
        poke(a, b, actor + 0xdc0, &rng.next().to_le_bytes());
        poke(a, b, sig_local, &rng.next().to_le_bytes());
        // the first target's damage buffer for i's command, its result one
        // of those the signals test
        let i8 = i & 0xff;
        let first = ffx_battle::body::bridge::battle_first_selected_actor(b, i8, 0xffff_fff0, 0, 1);
        if ffx_battle::actor::bridge::ms_btl_chr_num_check(i8) != 0
            && first >= 0
            && rng.below(4) != 0
        {
            let cmd = ffx_battle::actor::bridge::ms_get_chr_command(b, i8);
            let id = b.read(cmd + 2, 1).unwrap()[0];
            let t = ffx_battle::actor::bridge::ms_get_chr(b, first.cast_unsigned());
            let (next, count) = (
                [0u8, 0, 1][rng.below(3) as usize],
                [1u8, 2][rng.below(2) as usize],
            );
            let r = [1u8, 0x11, 2, 0xff, 0][rng.below(5) as usize];
            for o in [0x774u32, 0xa4c] {
                poke(a, b, t + o, &[next, count, u8::try_from(i8).unwrap(), id]);
                for k in 0..2 {
                    poke(a, b, t + o + 0x18 + 0x2c * k, &[r]);
                }
            }
        }
        if rng.below(2) == 0 {
            twin(
                "MsEffectSignalHit",
                a,
                b,
                0x007b_75c0,
                &[i, actor],
                0,
                |m| {
                    let (fs, fd) = (m.rebase(0x0079_fd40), m.rebase(0x0078_d980));
                    g::ms_effect_signal_hit(
                        m,
                        &mut |m, i, k| {
                            call_original(m, fs, &[i, k]);
                        },
                        &mut |m, args| {
                            call_original(m, fd, &args);
                        },
                        i,
                        actor,
                        sig_local,
                    );
                    0
                },
            );
        } else {
            let signal = rng.below(11);
            twin(
                "MsEffectSignalMotion",
                a,
                b,
                0x007b_77c0,
                &[i, actor, signal],
                0,
                |m| {
                    g::ms_effect_signal_motion(m, &mut c, i, actor, signal, sig_local);
                    0
                },
            );
        }
        // (the damage result's words are the original's frame and slot)
        let v = b.read(sig_local, 8).unwrap();
        a.write(sig_local, &v).unwrap();
        same("effect signals", a, b);
        let chr = rng.below(0x12);
        let p = ply + chr * 0x94;
        poke(
            a,
            b,
            p + 0x14,
            &[rng.below(1000), 999_999_990, rng.next()][rng.below(3) as usize].to_le_bytes(),
        );
        poke(
            a,
            b,
            p + 0x18,
            &[rng.below(1000), 999_999_990, rng.next()][rng.below(3) as usize].to_le_bytes(),
        );
        poke(
            a,
            b,
            p + 0x3b,
            &[[0u8, 98, 99, 0xff, 50][rng.below(5) as usize]],
        );
        let mut n = [rng.below(2000), rng.next(), 0][rng.below(3) as usize];
        // sometimes exactly the next level's AP after the add
        if rng.below(3) == 0 {
            n = n.min(1000);
            let next = ffx_battle::menu::bridge::ms_get_next_ap(b, chr.cast_signed());
            poke(
                a,
                b,
                p + 0x18,
                &next.cast_unsigned().wrapping_sub(n).to_le_bytes(),
            );
        }
        twin("MsSaveAddAP", a, b, 0x0078_4610, &[chr, n], 0, |m| {
            g::ms_save_add_ap(m, chr, n);
            0
        });
        same("MsSaveAddAP", a, b);
        set(a, b, ffx_battle::state::bridge::VOICE_EVENT, chars + 0xffc);
        poke(a, b, chars + 0xffc, &[u8::from(rng.below(2) == 0)]);
        let k = [
            rng.below(8),
            8 + rng.below(0x10),
            rng.below(0x30),
            rng.next(),
        ][rng.below(4) as usize];
        let (flags, actor, out) = (rng.below(8), rng.next(), rng.next());
        twin(
            "MsAtelRequestSystem",
            a,
            b,
            0x0079_81e0,
            &[k, flags, actor, out],
            ALL,
            |m| g::ms_atel_request_system(m, &mut c, k, flags, actor, out),
        );
        same("MsAtelRequestSystem", a, b);
        let motion = [rng.below(0x40), 0x8000 | rng.below(0x40), rng.next()][rng.below(3) as usize];
        twin(
            "MsEffectSetMotion",
            a,
            b,
            0x0078_82c0,
            &[i, motion],
            ALL,
            |m| g::ms_effect_set_motion(m, &mut c, i, motion),
        );
        same("MsEffectSetMotion", a, b);
        twin(
            "MsEffectSetMotion_00790290",
            a,
            b,
            0x0079_0290,
            &[i, motion],
            ALL,
            |m| g::ms_effect_set_motion(m, &mut c, i, motion),
        );
        same("MsEffectSetMotion_00790290", a, b);
        let j = rng.below(ACTORS);
        // a move by speed (bit 7) reads the actor's position, which our own
        // actors (no position: MsGetChrPos writes nothing) leave as the
        // original's frame holds it: those only on the live ones
        if w.synthetic {
            let p = w.actor(j);
            let st = b.read(p + 0x415, 1).unwrap()[0];
            if st & 0x80 != 0 && st & 0x7f != 7 && st & 0x7f != 0 {
                poke(a, b, p + 0x415, &[st & 0x7f]);
            }
        }
        let mut fc = |m: &mut Oracle, f: u32, args: &[u32]| {
            let r = m.call(f, Conv::Cdecl, args).unwrap();
            battle_common::crt::f80_to_f32(r.st0_raw.unwrap())
        };
        twin("MsGetChrMoveFrameRest", a, b, 0x007a_9320, &[j], ALL, |m| {
            g::ms_get_chr_move_frame_rest(m, &stand_in, &mut fc, j, local)
        });
        let v = b.read(local, 0x20).unwrap();
        a.write(local, &v).unwrap();
        // MsPointLen leaves the target's w in VU vf2.w: the original's
        // target is its frame's, whose w nobody wrote (stack contents)
        let vf2w = a.rebase(0x00c8_f7fc);
        let w = a.read(vf2w, 4).unwrap();
        b.write(vf2w, &w).unwrap();
        same("MsGetChrMoveFrameRest", a, b);
    }
}

#[test]
fn leaf_motions_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    motions(&mut a, &mut b, &mut Xorshift(0x2929_1301), true);
}

#[test]
fn leaf_motions_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    motions(&mut a, &mut b, &mut Xorshift(0x2929_1302), false);
}

/// `MsSetBtlPosReset` (`ffx_battle::chr_damage`): its battle position goes
/// through a 16-byte frame local that `MsGetChrBtlPos` does not always
/// write (no file place, a non-actor), so the original's frame (at
/// `call_top - 0x20` for one argument) starts with the bytes the port's
/// `local` holds, and the original's frame after the call is compared with
/// the port's `local`.
fn resets(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use battle_common::crt::{patch, stand_in};
    use ffx_battle::chr_damage::bridge as cd;
    let w = world(a, b, rng, synthetic);
    for o in [&mut *a, &mut *b] {
        patch(o);
    }
    let local = scratch(a, b, 0x10);
    let frame = a.stack_range().end - 0x100 - 0x20;
    // MsSetMotion, MsAtelRequestCamera, MsSetTargetDir and
    // MsAtelRequestMagicCamera as recording stubs in both: the wrappers'
    // own logic and the arguments they pass
    let rec = scratch(a, b, 0x200);
    let stubbed: [(u32, u8); 4] = [
        (0x007a_b360, 7),
        (0x0079_7bc0, 8),
        (0x0079_69f0, 2),
        (0x0079_8000, 9),
    ];
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(stubbed) {
            recording_stub(o, f, STUBS_D + 0x80 * k, rec + 0x80 * k, n, 0);
        }
        // MsGetChrInfo for the ball's offset
        recording_stub(o, 0x0079_5170, STUBS_D + 0x200, rec + 0x1c0, 4, 0);
    }
    // op_et_eff (vfx) for the weapon effect of MsEffectChrMotionIn (round
    // 32): a recording stub, its result at `et_rec + 0x14`
    let et_rec = scratch(a, b, 0x40);
    // MsSetMotion for MsEffectMagicMotion (round 32): keeps its 7
    // arguments at `motion_rec + 8` and the byte `MsGetChr(i) + 0x41b` at
    // `motion_rec`, and returns the word after that (`mov eax,[esp+4k];
    // mov [rec+8+4k],eax` for each, then `push [esp+4]; call MsGetChr;
    // add esp,4; mov al,[eax+0x41b]; mov [rec],al; mov eax,[rec+4]; ret`)
    let motion_rec = scratch(a, b, 0x24);
    for o in [&mut *a, &mut *b] {
        recording_stub(o, 0x007f_c2e0, STUBS_D + 0x280, et_rec, 4, 0);
        let at = o.rebase(STUBS_D + 0x300);
        let mut args = Vec::new();
        for k in 0..7u8 {
            args.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0xa3]);
            args.extend((motion_rec + 8 + 4 * u32::from(k)).to_le_bytes());
        }
        let n = u32::try_from(args.len()).unwrap();
        let get_chr = o.rebase(0x0079_4020).wrapping_sub(at + n + 9);
        let code = [
            &args[..],
            &[0xff, 0x74, 0x24, 0x04, 0xe8][..],
            &get_chr.to_le_bytes(),
            &[0x83, 0xc4, 0x04, 0x8a, 0x80, 0x1b, 0x04, 0x00, 0x00, 0xa2],
            &motion_rec.to_le_bytes(),
            &[0xa1],
            &(motion_rec + 4).to_le_bytes(),
            &[0xc3],
        ]
        .concat();
        o.write(at, &code).unwrap();
    }
    let pos = scratch(a, b, 0x10);
    if w.synthetic {
        // a position file (as battle_pos.rs's): floats, small counts and
        // tables inside it
        let file = scratch(a, b, 0x800);
        let mut f: Vec<u8> = (0..0x200).flat_map(|_| float(rng).to_le_bytes()).collect();
        let flat = rng.below(4) == 0;
        let stride = if flat { 0x20 } else { 0x60 };
        for g in 0..4usize {
            let r = g * stride;
            for c in [2, 3, 4, 5, 6, 9] {
                f[r + c] = u8::try_from(rng.below(4)).unwrap();
            }
            for o in [0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x2c] {
                let t = 0x400 + rng.below(0x30) * 0x10;
                f[r + o..r + o + 4].copy_from_slice(&t.to_le_bytes());
            }
        }
        f[0] = u8::from(flat);
        f[1] = u8::try_from(1 + rng.below(4)).unwrap();
        poke(a, b, file, &f);
        set(a, b, ffx_battle::pos::bridge::POS_FILE, file);
        for j in 0..ACTORS {
            let p = w.actor(j);
            // no ride, no current position (it would recurse), places in
            // the file, defense positions now and then
            poke(a, b, p + 0x43f, &[0xff]);
            poke(a, b, p + 0x6dd, &[0]);
            for o in [0x4fc, 0x4fd, 0x4fe] {
                poke(a, b, p + o, &[u8::try_from(rng.below(4)).unwrap()]);
            }
            poke(a, b, p + 0x4fd, &[u8::try_from(rng.range(2, 12)).unwrap()]);
            poke(a, b, p + 0x6d4, &[if rng.below(4) == 0 { 1 } else { 0xff }]);
            for o in [0x608, 0x50d, 0xdcc, 0xde7] {
                poke(a, b, p + o, &[u8::from(rng.below(2) == 0)]);
            }
            let v: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
            poke(a, b, p + 0x3b0, &v);
            poke(a, b, p + 0x534, &float(rng).to_le_bytes());
        }
    }
    // the actors with a character (a battle snapshot's), half the calls
    let live: Vec<u32> = (0..ACTORS)
        .filter(|&j| {
            let chr: u32 = ffx_bridge::Mem::read(b, w.actor(j));
            !w.synthetic && readable(a, chr, 0x1000)
        })
        .collect();
    for _ in 0..300 {
        let i = if !live.is_empty() && rng.below(2) == 0 {
            live[rng.below(u32::try_from(live.len()).unwrap()) as usize]
        } else {
            [rng.below(ACTORS), rng.below(0x40), rng.next()][rng.below(8).min(2) as usize]
        };
        let p = w.actor(i % ACTORS);
        poke(
            a,
            b,
            p + 0x511,
            &[[rng.byte() & 3, rng.byte()][rng.below(6).min(1) as usize]],
        );
        for k in 0..4 {
            let dir = match rng.below(6) {
                // ten turns or more (or NaN): the radian error line
                0 => battle_common::world::quiet(rng.next() | 0x7f80_0001),
                1 => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 7.0).to_bits(),
                _ => (f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 500.0).to_bits(),
            };
            poke(a, b, p + 0x514 + 4 * k, &dir.to_le_bytes());
        }
        if w.synthetic {
            // no character to place (Ch_ClothReset wants a real one)
            poke(a, b, p, &[0; 4]);
        }
        let seed: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
        a.write(frame, &seed).unwrap();
        poke(a, b, local, &seed);
        twin("MsSetBtlPosReset", a, b, 0x007a_b2a0, &[i], ALL, |m| {
            cd::ms_set_btl_pos_reset(m, &stand_in, i, local).cast_unsigned()
        });
        let v = a.read(frame, 0x10).unwrap();
        a.write(local, &v).unwrap();
        // MsDivVector (the sisters' summoner position) leaves the caller's
        // saved registers in VF_B's last three words
        let at = ffx_battle::vector::bridge::VF_B.ptr(a).addr + 4;
        let r = a.read(at, 12).unwrap();
        b.write(at, &r).unwrap();
        same(&format!("MsSetBtlPosReset {i:#x}"), a, b);
        // the same actor's other wrappers
        let v: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
        poke(a, b, pos, &v);
        twin("MsSetChrPos2D", a, b, 0x0079_61d0, &[i, pos], ALL, |m| {
            ffx_battle::minor::bridge::ms_set_chr_pos_2d(m, i, pos).cast_unsigned()
        });
        same("MsSetChrPos2D", a, b);
        let ret = |k: u32, n: u32| rec + 0x80 * k + 4 + 4 * n;
        for k in 0..4 {
            let r = rng.next();
            poke(
                a,
                b,
                ret(k, u32::from(stubbed[k as usize].1)),
                &r.to_le_bytes(),
            );
        }
        poke(a, b, p + 0xdff, &[u8::from(rng.below(3) == 0)]);
        // (the place kind put back after: a summoner's kind, 12-14, makes
        // the next position read recurse for ever)
        let kind = b.read(p + 0x4fd, 1).unwrap();
        poke(a, b, p + 0x4fd, &[rng.byte()]);
        twin("MsEffectChrMotionOut", a, b, 0x007b_7e20, &[i], ALL, |m| {
            let f = m.rebase(0x007a_b360);
            ffx_battle::minor::bridge::ms_effect_chr_motion_out(
                m,
                &mut |m, args| call_original(m, f, &args),
                i,
            )
        });
        same("MsEffectChrMotionOut", a, b);
        poke(a, b, p + 0x4fd, &kind);
        // MsEffectChrMotionIn (round 32): the same actor coming back in,
        // its escape position in the frame local (as MsSetBtlPosReset's)
        poke(a, b, p + 0x4fd, &[rng.byte()]);
        poke(a, b, p + 0xdff, &[u8::from(rng.below(3) != 0)]);
        for o in [0x50a, 0x3f0, 0x41c, 0x412, 0x5d9] {
            poke(a, b, p + o, &[rng.byte()]);
        }
        poke(a, b, p + 0xdcd, &[u8::from(rng.below(3) == 0)]);
        poke(
            a,
            b,
            et_rec + 0x14,
            &[1u32, 0, 0x8000_0000][rng.below(3) as usize].to_le_bytes(),
        );
        let seed: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
        a.write(frame, &seed).unwrap();
        poke(a, b, local, &seed);
        twin("MsEffectChrMotionIn", a, b, 0x007b_7d70, &[i], ALL, |m| {
            ffx_battle::effect_signal::bridge::ms_effect_chr_motion_in(
                m,
                &mut |m: &mut Oracle, f, args: &[u32]| call_original(m, f, args),
                i,
                local,
            )
        });
        let v = a.read(frame, 0x10).unwrap();
        a.write(local, &v).unwrap();
        let at = ffx_battle::vector::bridge::VF_B.ptr(a).addr + 4;
        let r = a.read(at, 12).unwrap();
        b.write(at, &r).unwrap();
        same(&format!("MsEffectChrMotionIn {i:#x}"), a, b);
        poke(a, b, p + 0x4fd, &kind);
        // MsEffectMagicMotion (round 32): kind 2 most of the time
        let motion = [0xff, 0x32, rng.byte()][rng.below(3) as usize];
        poke(a, b, p + 0xe01, &[motion]);
        for o in [0x405, 0x41b, 0x3f2, 0x3f3, 0xf5e, 0xdf2] {
            poke(a, b, p + o, &[rng.byte()]);
        }
        poke(a, b, p + 0x3f3, &[u8::from(rng.below(2) == 0)]);
        let kind8 = [2, 2, 2, 0, 1, 3, rng.below(0x100)][rng.below(7) as usize];
        let w8 = (i & 0xff) | kind8 << 8 | rng.next() & 0xffff_0000;
        // MsSetMotion keeps its actor's +0x41b at the call (what the
        // original sets for it) and returns a word of `motion_rec`
        poke(a, b, motion_rec, &[0; 4]);
        poke(a, b, motion_rec + 4, &rng.next().to_le_bytes());
        poke(a, b, motion_rec + 8, &[0; 0x1c]);
        for o in [&mut *a, &mut *b] {
            redirect(o, 0x007a_b360, STUBS_D + 0x300);
        }
        twin("MsEffectMagicMotion", a, b, 0x0078_7c30, &[w8], ALL, |m| {
            ffx_battle::effect_signal::bridge::ms_effect_magic_motion(
                m,
                &mut |m: &mut Oracle, f, args: &[u32]| call_original(m, f, args),
                w8,
            )
        });
        for o in [&mut *a, &mut *b] {
            redirect(o, 0x007a_b360, STUBS_D);
        }
        same(&format!("MsEffectMagicMotion {w8:#x}"), a, b);
        // a command cue running (or not), the turn word, the head
        set(
            a,
            b,
            ffx_battle::state::bridge::COMMAND_CUE_COUNT,
            [0, 1, -1][rng.below(3) as usize],
        );
        set(
            a,
            b,
            ffx_battle::damage_end::bridge::CUE_FLAG_C9DE,
            u8::from(rng.below(2) == 0),
        );
        set(
            a,
            b,
            ffx_battle::flow::bridge::NO_HEAD_TARGET,
            u8::from(rng.below(4) == 0),
        );
        poke(a, b, p + 0x437, &[u8::from(rng.below(3) != 0)]);
        let targets = [
            1 << rng.below(ACTORS),
            1 << rng.below(ACTORS),
            rng.next(),
            0,
        ][rng.below(4) as usize];
        set(
            a,
            b,
            ffx_battle::input::bridge::TURN_WORD,
            [targets, rng.next()][rng.below(3).min(1) as usize],
        );
        set(a, b, ffx_battle::actor::bridge::CHR_SET, rng.next());
        let camera = |m: &mut Oracle, args: [u32; 8]| {
            let f = m.rebase(0x0079_7bc0);
            call_original(m, f, &args)
        };
        match rng.below(3) {
            0 => twin("MsResetBtlTarget", a, b, 0x0079_2a40, &[i], 0, |m| {
                ffx_battle::minor::bridge::ms_reset_btl_target(m, &mut { camera }, i);
                0
            }),
            1 => twin("MsSetBtlTarget", a, b, 0x0079_2de0, &[i, targets], 0, |m| {
                let mut dir = |m: &mut Oracle, i: u32, t: u32| {
                    let f = m.rebase(0x0079_69f0);
                    call_original(m, f, &[i, t]);
                };
                ffx_battle::minor::bridge::ms_set_btl_target(
                    m,
                    &mut dir,
                    &mut { camera },
                    i,
                    targets,
                );
                0
            }),
            _ => twin("MsRestartMagicCamera", a, b, 0x007b_1a70, &[i], ALL, |m| {
                let f = m.rebase(0x0079_8000);
                ffx_battle::minor::bridge::ms_restart_magic_camera(
                    m,
                    &mut |m, args| call_original(m, f, &args),
                    i,
                )
            }),
        }
        same("battle targets", a, b);
        // the ball's start: its position through the same frame
        let seed: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
        a.write(frame, &seed).unwrap();
        poke(a, b, local, &seed);
        poke(a, b, rec + 0x1c0 + 0x14, &rng.next().to_le_bytes());
        twin("MsSetBallSplineInit", a, b, 0x007b_7c80, &[i], ALL, |m| {
            let f = m.rebase(0x0079_5170);
            ffx_battle::minor::bridge::ms_set_ball_spline_init(
                m,
                &stand_in,
                &mut |m, args| call_original(m, f, &args),
                i,
                local,
            )
        });
        let v = a.read(frame, 0x10).unwrap();
        a.write(local, &v).unwrap();
        same("MsSetBallSplineInit", a, b);
        // a point's side of a line on the ground
        let pts: Vec<u8> = (0..12).flat_map(|_| float(rng).to_le_bytes()).collect();
        poke(a, b, w.out, &pts);
        if rng.below(6) == 0 {
            // a line of no length
            let p = b.read(w.out, 12).unwrap();
            poke(a, b, w.out + 0x10, &p);
        }
        let flip = rng.below(2) * rng.next();
        ftwin(
            "MsLineSideDist",
            a,
            b,
            0x007b_6f10,
            &[flip, w.out, w.out + 0x10, w.out + 0x20],
            |m| {
                ffx_battle::minor::bridge::ms_line_side_dist(
                    m,
                    flip,
                    w.out,
                    w.out + 0x10,
                    w.out + 0x20,
                )
            },
        );
        same("MsLineSideDist", a, b);
    }
}

#[test]
fn leaf_resets_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    resets(&mut a, &mut b, &mut Xorshift(0x3030_0101), true);
}

#[test]
fn leaf_resets_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    resets(&mut a, &mut b, &mut Xorshift(0x3030_0102), false);
}

/// Group J (round 32, `ffx_battle::sound`): the SEP transfers queued from
/// a sound file (`MsRegistSEdata`, `MsSEPtransRegist`), a voice line
/// (`MsSetVoice`) and a shout stream (`MsSoundStreamPlay`). The sound
/// callees are stubs: `SndGetSepNumber` logs its blocks (a result that
/// changes each call), the shout callees log theirs, `MsAtelRequestExe`
/// keeps its last call, and the transfer callees of a full queue's
/// `MsSEPtransProcess` end at once (`SndSyncWaveTrans` 0).
fn sounds(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use ffx_battle::battle_end::bridge as be;
    use ffx_battle::sound::bridge as so;
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x110 * 8);
    let callees: [(u32, u8); 8] = [
        (so::SND_GET_SEP_NUMBER, 1),
        (so::SND_IS_READ_SHOUT_VOICE_STATUS, 0),
        (so::SND_START_SHOUT_STREAM, 3),
        (so::MS_ATEL_REQUEST_EXE, 6),
        (be::SND_SEP_TRANS_SEP_ONLY, 2),
        (be::SND_STANDBY_BATTLE_WAVE, 2),
        (be::SND_SYNC_WAVE_TRANS, 1),
        (be::SND_STANDBY_SEP_WAVE, 0),
    ];
    let slot = |k: u32| rec + 0x110 * k;
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(callees) {
            if n > 4 {
                recording_stub(o, f, STUBS_D + 0x60 * k, slot(k), n, 0);
            } else {
                logging_stub(o, f, STUBS_D + 0x60 * k, slot(k), n, 0);
            }
        }
    }
    let file = scratch(a, b, 0x1000);
    // one of the values (all drawn first)
    macro_rules! pick {
        ($rng:expr, [$($v:expr),+ $(,)?] $(,)?) => {{
            let v = [$($v),+];
            v[$rng.below(u32::try_from(v.len()).unwrap()) as usize]
        }};
    }
    let mut call = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
    let queue = be::SEP_TRANS_QUEUE.ptr(b).addr;
    let count = ffx_battle::state::bridge::SEP_TRANS;
    let state = g::SEP_TRANS_STATE;
    for _ in 0..400 {
        // the queue: records of actors (the scene's would free its
        // buffer), any state and kind; a count that is full now and then
        for k in 0..16 {
            let r = queue + 12 * k;
            // (the transfer reads its data's first byte)
            poke(a, b, r, &(file + rng.below(0x1000)).to_le_bytes());
            poke(a, b, r + 4, &rng.next().to_le_bytes());
            let kind = u8::try_from(pick!(rng, [0, 1, 1, 2])).unwrap();
            let who = u8::try_from(rng.below(ACTORS)).unwrap();
            let st = u8::try_from(rng.below(3)).unwrap();
            poke(a, b, r + 8, &[kind, who, st]);
        }
        let n = {
            let r = rng.below(16);
            pick!(rng, [r, r, 0, 15, 16, 17])
        };
        set(a, b, count, n);
        // (a negative first index only while the queue is not full: a
        // full one would end a record outside the queue)
        let first = {
            let r = rng.below(16);
            if n < 16 && rng.below(8) == 0 {
                0xffff_fff3
            } else {
                pick!(rng, [r, r, 0, 15])
            }
        };
        // (word 0 of the state; the count is its word 1)
        poke(a, b, state.ptr(b).addr, &first.to_le_bytes());
        set(a, b, be::SEP_TRANS_SCENE, rng.byte());
        poke(a, b, slot(0) + 4, &rng.next().to_le_bytes());
        let actor = w.actor(rng.below(ACTORS));
        for o in [0xc, 0xf2d, 0xf2e] {
            poke(a, b, actor + o, &[rng.byte()]);
        }
        let op = rng.below(4);
        if op == 1 {
            // a file of up to 4 transfers fills no queue
            set(a, b, count, rng.below(12));
        }
        match op {
            0 => {
                let args = [
                    pick!(rng, [0xff, rng.below(ACTORS), rng.next()]),
                    file + rng.below(0x1000),
                    rng.next(),
                    pick!(rng, [0, 1, 2, rng.next()]),
                ];
                twin("MsSEPtransRegist", a, b, 0x007a_06b0, &args, ALL, |m| {
                    so::ms_sep_trans_regist(m, &mut call, args[0], args[1], args[2], args[3])
                });
                same(&format!("MsSEPtransRegist {args:x?}"), a, b);
            }
            1 => {
                // a sound file: sections of blocks (0 none) and an end
                let mut f = vec![0u8; 0x1000];
                let put = |f: &mut Vec<u8>, at: usize, v: u32| {
                    f[at..at + 4].copy_from_slice(&v.to_le_bytes());
                };
                let sections = pick!(rng, [1, 2, 3, 4, 4, 0, u32::MAX]);
                put(&mut f, 0, sections);
                let mut at = 0x20usize;
                for s in 0..4usize {
                    let none = rng.below(6) == 0;
                    put(
                        &mut f,
                        4 + 4 * s,
                        if none { 0 } else { u32::try_from(at).unwrap() },
                    );
                    let blocks = {
                        let r = rng.below(0x24);
                        pick!(rng, [r, r, r, 0, u32::MAX, 0x21, 0x88])
                    };
                    put(&mut f, at, blocks);
                    let words = usize::try_from(blocks.cast_signed().max(0))
                        .unwrap()
                        .min(0x88)
                        + 1;
                    for k in 0..words {
                        let o = if rng.below(3) == 0 {
                            0
                        } else {
                            0x800 + rng.below(0x700)
                        };
                        put(&mut f, at + 4 + 4 * k, o);
                    }
                    at += 4 * (words + 1);
                }
                poke(a, b, file, &f);
                let who = pick!(rng, [actor, actor, 0]);
                let sep = if rng.below(10) == 0 { 0 } else { file };
                // the table the scene's numbers go to, and the actor's
                let t = g::SEP_TRANS_TABLE.ptr(b).addr;
                poke(a, b, t, &vec![0xee; 0x200]);
                poke(a, b, actor + 0xeac, &[0xdd; 0x80]);
                twin("MsRegistSEdata", a, b, 0x007a_01b0, &[who, sep], ALL, |m| {
                    so::ms_regist_se_data(m, &mut call, who, sep).cast_unsigned()
                });
                same(&format!("MsRegistSEdata {who:#x}"), a, b);
            }
            2 => {
                let i = pick!(rng, [rng.below(ACTORS), rng.below(ACTORS), rng.next()]);
                let line = pick!(rng, [rng.below(0x40), u32::MAX, 0x8000_0000, 0x1_2345]);
                let r = pick!(rng, [0, 1, u32::MAX, 0x8000_0000, 0x7fff_ffff]);
                poke(a, b, slot(3) + 4 + 4 * 6, &r.to_le_bytes());
                set(
                    a,
                    b,
                    ffx_battle::state::bridge::ATEL_COUNT,
                    pick!(rng, [0i32, 3, 15, 16]),
                );
                let more = [rng.next(), rng.next(), rng.next()];
                let args = [i, line, more[0], more[1], more[2]];
                twin("MsSetVoice", a, b, 0x007a_0720, &args, ALL, |m| {
                    so::ms_set_voice(m, &mut call, i, line, more)
                });
                same(&format!("MsSetVoice {args:x?}"), a, b);
            }
            _ => {
                poke(
                    a,
                    b,
                    slot(1) + 4,
                    &pick!(rng, [0u32, 0, 0, 1]).to_le_bytes(),
                );
                set(a, b, so::DEBUG_SHOUT_TRACE, u8::from(rng.below(2) == 0));
                let speaker = pick!(
                    rng,
                    [rng.below(8), rng.below(8), u32::MAX, 0xffff_fffb, 8, 100],
                );
                let line = pick!(rng, [rng.below(8), rng.below(8), 8, u32::MAX, rng.next()]);
                twin(
                    "MsSoundStreamPlay",
                    a,
                    b,
                    0x007a_0820,
                    &[speaker, line],
                    ALL,
                    |m| {
                        so::ms_sound_stream_play(m, &mut call, speaker.cast_signed(), line)
                            .cast_unsigned()
                    },
                );
                same(&format!("MsSoundStreamPlay {speaker:#x} {line:#x}"), a, b);
            }
        }
    }
}

#[test]
fn leaf_sounds_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    sounds(&mut a, &mut b, &mut Xorshift(0x3232_0101), true);
}

#[test]
fn leaf_sounds_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    sounds(&mut a, &mut b, &mut Xorshift(0x3232_0102), false);
}

/// The battle's reads of round 33 (`ffx_battle::read`): the language
/// data's reread (`MsBattleReadDataNB`, `MsBattleReadData`), the monster
/// and weapon files' attaches and the weapon reads with their
/// completions. Every callee of other modules and the CD object's methods
/// are logging or recording stubs (the CD's read `+0` keeps the request
/// block's `+0xc` and `+0x10` at `seen`, since the original's block is in
/// its frame and the port's at `request`); the sound files are registered
/// for real (`MsRegistSEdata`, its own callees stubs).
fn reads(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use ffx_battle::battle_end::bridge as be;
    use ffx_battle::read::bridge as rd;
    use ffx_battle::sound::bridge as so;
    macro_rules! pick {
        ($rng:expr, [$($v:expr),+ $(,)?] $(,)?) => {{
            let v = [$($v),+];
            v[$rng.below(u32::try_from(v.len()).unwrap()) as usize]
        }};
    }
    let w = world(a, b, rng, synthetic);
    if w.synthetic {
        set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
    }
    // (function, arguments): logging stubs, results at `slot(k) + 4`
    let logged: [(u32, u8); 18] = [
        (ffx_battle::getmem::bridge::GET_CD, 0),
        (g::OP_ET_WEAPON_INIT, 2),
        (rd::MS_BTL_READ_MONSTER, 0),
        (rd::MS_GET_CHECK_ENGLISH, 0),
        (rd::MS_BATTLE_EXE, 4),
        (rd::MS_BTL_READ_SET_SCENE, 0),
        (rd::KERNEL_LOAD_TABLES, 1),
        (rd::MS_MENU_READ_START_TEXT, 1),
        (rd::MS_IMPORTANT_READ, 0),
        (rd::MS_WEAPON_NAME_READ, 0),
        (rd::EI_ABM_SET_LANG, 0),
        (so::SND_GET_SEP_NUMBER, 1),
        (so::SND_IS_READ_SHOUT_VOICE_STATUS, 0),
        (so::SND_START_SHOUT_STREAM, 3),
        (be::SND_SEP_TRANS_SEP_ONLY, 2),
        (be::SND_STANDBY_BATTLE_WAVE, 2),
        (be::SND_SYNC_WAVE_TRANS, 1),
        (be::SND_STANDBY_SEP_WAVE, 0),
    ];
    let rec = scratch(a, b, 0x110 * 24 + 0x40);
    let slot = |k: u32| rec + 0x110 * k;
    // the CD object's methods +0 (3 words logged), +8, +0x10, +0x14
    let cd_slot = |k: u32| slot(18 + k);
    let malloc_rec = slot(22);
    let seen = slot(23);
    let obj = scratch(a, b, 0x20);
    let stub_at = |k: u32| {
        if k < 21 {
            STUBS_D + 0x40 * k
        } else {
            STUBS_E + 0x40 * (k - 21)
        }
    };
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(logged) {
            logging_stub(o, f, stub_at(k), slot(k), n, 0);
        }
        recording_stub(o, rd::SG_MALLOC_A, stub_at(18), malloc_rec, 5, 0);
        for (k, (m, n)) in (0u32..).zip([(0u32, 3u8), (8, 1), (0x10, 1), (0x14, 1)]) {
            logging_stub(o, 0, stub_at(19 + k), cd_slot(k), n, 0);
            let f = o.rebase(stub_at(19 + k));
            o.write(obj + m, &f.to_le_bytes()).unwrap();
        }
        // the read: the request's +0xc and +0x10 to `seen`, then the log
        let at = o.rebase(stub_at(23));
        let log = o.rebase(stub_at(19));
        let mut code = vec![0x8b, 0x44, 0x24, 0x10, 0x85, 0xc0, 0x74, 0x12];
        code.extend([0x8b, 0x48, 0x0c, 0x89, 0x0d]);
        code.extend(seen.to_le_bytes());
        code.extend([0x8b, 0x48, 0x10, 0x89, 0x0d]);
        code.extend((seen + 4).to_le_bytes());
        code.push(0xe9);
        code.extend(
            log.wrapping_sub(at + u32::try_from(code.len()).unwrap() + 4)
                .to_le_bytes(),
        );
        o.write(at, &code).unwrap();
        o.write(obj, &at.to_le_bytes()).unwrap();
    }
    poke(a, b, slot(0) + 4, &obj.to_le_bytes());
    let request = scratch(a, b, rd::WEAPON_REQUEST as usize);
    let done = scratch(a, b, 0x20);
    // files: a count, then section offsets into the 0x400 bytes; a sound
    // file at +0x200 (one section of one block) the sections may point at
    let files: Vec<u32> = (0..4).map(|_| scratch(a, b, 0x400)).collect();
    let make_file = |a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, f: u32| {
        let mut bytes = vec![0u8; 0x400];
        for x in &mut bytes[0x40..0x200] {
            *x = rng.byte();
        }
        let n = pick!(rng, [0u32, 1, 2, 3, 6, 7, 8, rng.below(10), u32::MAX]);
        bytes[..4].copy_from_slice(&n.to_le_bytes());
        for k in 1..8usize {
            // (sections 1 and 6 are the weapons' and monsters' sounds)
            let o = if k == 1 || k == 6 {
                pick!(rng, [0u32, 0x200, 0x200])
            } else {
                pick!(rng, [0u32, 0x200, 0x40 + rng.below(0x1c0)])
            };
            bytes[4 * k..4 * k + 4].copy_from_slice(&o.to_le_bytes());
        }
        for (k, v) in [1u32, 0x10, 0, 0, 1, 0x40, 0x80].into_iter().enumerate() {
            let at = 0x200 + 4 * k;
            bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        poke(a, b, f, &bytes);
    };
    let actor_index =
        |rng: &mut Xorshift| pick!(rng, [rng.below(0x12), rng.below(0x12), 6, 7, 0x11]);
    for round in 0..300 {
        for &f in &files {
            make_file(a, b, rng, f);
        }
        // (a read leaves its block, a made-up address, in +0x48)
        for k in 0..ACTORS {
            let f = pick!(rng, [0, files[0], files[1]]);
            poke(a, b, w.actor(k) + 0x48, &f.to_le_bytes());
        }
        // the sound queue empty, its state at the start
        set(a, b, ffx_battle::state::bridge::SEP_TRANS, 0);
        poke(a, b, g::SEP_TRANS_STATE.ptr(b).addr, &[0; 4]);
        for k in 0..18u32 {
            poke(
                a,
                b,
                slot(k) + 4,
                &pick!(rng, [0u32, 0, 1, u32::MAX, rng.next()]).to_le_bytes(),
            );
        }
        poke(a, b, slot(0) + 4, &obj.to_le_bytes());
        for k in 0..4 {
            poke(a, b, cd_slot(k) + 4, &rng.next().to_le_bytes());
        }
        let block = rng.next() | 0x10;
        poke(a, b, malloc_rec + 4 + 4 * 5, &block.to_le_bytes());
        poke(a, b, seen, &[0; 8]);
        poke(a, b, request, &[0; rd::WEAPON_REQUEST as usize]);
        set(a, b, ffx_battle::misc::bridge::BTL_READ, rng.word());
        let op = round % 7;
        // an actor with one of the files or none, shown or not
        let i = actor_index(rng);
        let actor_i = i;
        let actor = ffx_battle::actor::bridge::ms_get_chr(b, i);
        let file = files[rng.below(4) as usize];
        for at in [0x11u32, 0xdc8] {
            poke(a, b, actor + at, &[pick!(rng, [0u8, 1, rng.byte()])]);
        }
        // (the attaches read the file: one there for them)
        let p48 = if op == 0 || op == 5 {
            file
        } else {
            pick!(rng, [0, file, file])
        };
        poke(a, b, actor + 0x48, &p48.to_le_bytes());
        poke(
            a,
            b,
            actor + 0x4c,
            &pick!(rng, [0u32, 0, rng.next()]).to_le_bytes(),
        );
        for o in [0xc, 0xf2d, 0xf2e] {
            poke(a, b, actor + o, &[rng.byte()]);
        }
        match op {
            0 => {
                let i = pick!(rng, [i, i, rng.below(ACTORS), ACTORS + rng.below(ACTORS)]);
                if i != actor_i {
                    let p = ffx_battle::actor::bridge::ms_get_chr(b, i);
                    poke(a, b, p + 0x11, &[rng.byte()]);
                    poke(a, b, p + 0x48, &file.to_le_bytes());
                    poke(
                        a,
                        b,
                        p + 0x4c,
                        &pick!(rng, [0u32, rng.next()]).to_le_bytes(),
                    );
                }
                let mut call = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
                twin(
                    "MsAttachWeaponData_00784000",
                    a,
                    b,
                    0x0078_4000,
                    &[i, rng.word()],
                    ALL,
                    |m| rd::ms_attach_weapon_data(m, &mut call, i),
                );
                same(&format!("MsAttachWeaponData_00784000 {i:#x}"), a, b);
            }
            1 | 2 => {
                let k = pick!(rng, [rng.below(8), rng.below(8), 0x107]);
                let m = ffx_battle::actor::bridge::ms_get_mon(b, k);
                poke(a, b, m + 0x48, &file.to_le_bytes());
                poke(a, b, m + 0x11, &[pick!(rng, [0u8, 1, rng.byte()])]);
                for o in [0xc, 0xf2d, 0xf2e] {
                    poke(a, b, m + o, &[rng.byte()]);
                }
                let mut call = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
                if op == 1 {
                    twin("Mon_AttachData", a, b, 0x0078_3b00, &[k], ALL, |m| {
                        rd::mon_attach_data(m, &mut call, k)
                    });
                } else {
                    poke(a, b, done + 0xc, &k.to_le_bytes());
                    twin(
                        "MsBtlReadMonsterDone_00783990",
                        a,
                        b,
                        0x0078_3990,
                        &[done],
                        ALL,
                        |m| rd::ms_btl_read_monster_done(m, &mut call, done),
                    );
                }
                same(&format!("Mon_AttachData {op} {k:#x}"), a, b);
            }
            3..=5 => {
                // the weapon's save slot: any model
                let id = pick!(
                    rng,
                    [
                        rng.below(0xff),
                        rng.below(8) | 0x7000,
                        rng.below(8) | 0xb000
                    ]
                );
                poke(a, b, actor + 0x592, &[id.to_le_bytes()[0]]);
                let s = ffx_battle::save::bridge::ms_get_save_weapon(
                    b,
                    u32::from(id.to_le_bytes()[0]),
                    0,
                );
                poke(
                    a,
                    b,
                    s + 0xc,
                    &u16::try_from(rng.below(0x1_0000)).unwrap().to_le_bytes(),
                );
                let mut call = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
                match op {
                    3 => {
                        let i = pick!(rng, [i, i, 0xffff_fff9, 0x8000_0000]);
                        // (an attach is of MsGetChr(i)'s file)
                        let p = ffx_battle::actor::bridge::ms_get_chr(b, i);
                        if p != actor {
                            poke(a, b, p + 0x48, &file.to_le_bytes());
                        }
                        let then = rng.word();
                        twin(
                            "MsCDreadWeapon",
                            a,
                            b,
                            0x0078_4120,
                            &[i, actor, then],
                            ALL,
                            |m| rd::ms_cd_read_weapon(m, &mut call, [i, actor, then], request),
                        );
                    }
                    4 => {
                        // every actor's weapon flag and file at random
                        for k in 0..0x12 {
                            let p = w.actor(k);
                            poke(a, b, p + 0xdc8, &[pick!(rng, [0u8, 0, 0, 1])]);
                            poke(a, b, p + 0x48, &pick!(rng, [0, 0, file]).to_le_bytes());
                            poke(a, b, p + 0x592, &[rng.below(0xff).to_le_bytes()[0]]);
                        }
                        let next = ffx_battle::misc::bridge::BTL_READ2.ptr(b).addr;
                        poke(
                            a,
                            b,
                            next,
                            &[pick!(
                                rng,
                                [
                                    0u8,
                                    rng.below(0x13).to_le_bytes()[0],
                                    0x12,
                                    0xfe,
                                    rng.byte()
                                ]
                            )],
                        );
                        twin("MsBtlReadWeapon", a, b, 0x0078_3f60, &[], ALL, |m| {
                            rd::ms_btl_read_weapon(m, &mut call, request)
                        });
                    }
                    _ => {
                        for k in 0..0x12 {
                            let p = w.actor(k);
                            poke(a, b, p + 0xdc8, &[pick!(rng, [0u8, 0, 1])]);
                            poke(a, b, p + 0x592, &[rng.below(0xff).to_le_bytes()[0]]);
                        }
                        let then = pick!(rng, [0u32, 0, 1, 2, 2, 3, rng.next()]);
                        poke(a, b, done + 0xc, &i.to_le_bytes());
                        poke(a, b, done + 0x10, &then.to_le_bytes());
                        set(
                            a,
                            b,
                            ffx_battle::misc::bridge::BTL_READ2,
                            u16::from(rng.byte()),
                        );
                        set(
                            a,
                            b,
                            ffx_battle::state::bridge::READ_CUE_LEVEL,
                            pick!(rng, [0u8, 1, 0x80]),
                        );
                        set(
                            a,
                            b,
                            ffx_battle::misc::bridge::READ_CUE_INDEX,
                            u8::try_from(rng.below(32)).unwrap(),
                        );
                        let mask = if then == 2 { 0 } else { ALL };
                        twin(
                            "MsCDreadWeaponDone_00783fc0",
                            a,
                            b,
                            0x0078_3fc0,
                            &[done],
                            mask,
                            |m| rd::ms_cd_read_weapon_done(m, &mut call, done, request),
                        );
                    }
                }
                // the request is the port's own block (the original's frame)
                let r = b.read(request, rd::WEAPON_REQUEST as usize).unwrap();
                a.write(request, &r).unwrap();
                same(&format!("weapon read {op} {i:#x}"), a, b);
            }
            _ => {
                set(
                    a,
                    b,
                    ffx_battle::init::bridge::CONFIG_ENGLISH,
                    pick!(rng, [0u32, 1, 1, rng.next()]),
                );
                set(
                    a,
                    b,
                    rd::READ_DATA_STEP,
                    pick!(rng, [rng.below(8), rng.below(8), 7, 8, u32::MAX]),
                );
                set(a, b, rd::READ_DATA_PART, rng.word());
                poke(
                    a,
                    b,
                    slot(3) + 4,
                    &pick!(rng, [0u32, u32::MAX, u32::MAX, 1]).to_le_bytes(),
                );
                // the two script records the shift moves
                let r = ffx_battle::getmem::bridge::SCRIPT_RECORD1.ptr(b).addr;
                let bytes: Vec<u8> = (0..0x40).map(|_| rng.byte()).collect();
                poke(a, b, r - 0x20, &bytes);
                let mut call = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
                if rng.below(3) == 0 {
                    // to the end: the loads finish at once
                    poke(a, b, slot(6) + 4, &[0; 4]);
                    poke(a, b, slot(7) + 4, &[0; 4]);
                    twin("MsBattleReadData", a, b, 0x0078_2050, &[], ALL, |m| {
                        rd::ms_battle_read_data(m, &mut call)
                    });
                } else {
                    // a run of steps, the loads' parts not the last at
                    // times (part words left over)
                    for _ in 0..8 {
                        for k in [6u32, 7] {
                            let r = pick!(rng, [0u32, 0, 1, rng.next()]);
                            poke(a, b, slot(k) + 4, &r.to_le_bytes());
                        }
                        let more = pick!(rng, [0u32, 1, 1, 1, 5]);
                        twin("MsBattleReadDataNB", a, b, 0x0078_2090, &[more], ALL, |m| {
                            rd::ms_battle_read_data_nb(m, &mut call, more)
                        });
                        same(&format!("MsBattleReadDataNB {more}"), a, b);
                    }
                }
                same("MsBattleReadData", a, b);
            }
        }
    }
}

#[test]
fn leaf_reads_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    reads(&mut a, &mut b, &mut Xorshift(0x3333_0101), true);
}

#[test]
fn leaf_reads_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    reads(&mut a, &mut b, &mut Xorshift(0x3333_0102), false);
}

/// Round 33's cursors and overdrive bars (`ffx_battle::cursor`):
/// `getSparedLmtPos`, `MsGetChrLimitBarFlag`, `MsCalcCursorPos`,
/// `MsBattleCursorCalc`. Stubs in both oracles: the screen size
/// (`graphicFontGetScreenWH` writes the two words at `rec`), the
/// projection (its results from a ring of 64 entries at `rec + 0x10`:
/// the two words and the `f32` it returns; it logs the point at the
/// entry's `+0x400`), `graphicUiRemapY2` (its argument times the
/// `f32` at `rec + 8`), `MagicFile_GetMagicId` and `MsNumberDrawProcess`
/// (logging). The positions (`MsGetChrPos`, `Ch_OptposGet`, ...) are the
/// originals' and the ports'; the originals' frames are their own, the
/// ports' at `local` (copied after).
// screen coordinates through f32, as the game's
#[allow(clippy::cast_precision_loss)]
fn cursors(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use battle_common::crt::{patch, stand_in};
    use ffx_battle::cursor::bridge as cu;
    // the character the others ride (hidden, never picked)
    const RIDE: u32 = ACTORS - 1;
    macro_rules! pick {
        ($rng:expr, [$($v:expr),+ $(,)?] $(,)?) => {{
            let v = [$($v),+];
            v[$rng.below(u32::try_from(v.len()).unwrap()) as usize]
        }};
    }
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x900);
    let table = rec + 0x10;
    let logs = scratch(a, b, 0x110 * 2);
    let local = scratch(a, b, 0x80);
    let out_words = scratch(a, b, 8);
    for o in [&mut *a, &mut *b] {
        patch(o);
        // the screen size: [arg0] = [rec], [arg1] = [rec + 4]
        let mut code = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x0d];
        code.extend(rec.to_le_bytes());
        code.extend([0x89, 0x08, 0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
        code.extend((rec + 4).to_le_bytes());
        code.extend([0x89, 0x08, 0xc3]);
        let at = o.rebase(STUBS_D);
        o.write(at, &code).unwrap();
        redirect(o, cu::FONT_GET_SCREEN_WH, STUBS_D);
        // the projection: entry n & 63 of the table (n at rec + 0xc)
        let mut code = vec![0x8b, 0x0d];
        code.extend((rec + 0xc).to_le_bytes());
        code.extend([0xff, 0x05]);
        code.extend((rec + 0xc).to_le_bytes());
        code.extend([0x83, 0xe1, 0x3f, 0xc1, 0xe1, 0x04, 0x81, 0xc1]);
        code.extend(table.to_le_bytes());
        code.extend([
            0x8b, 0x54, 0x24, 0x04, 0x8b, 0x01, 0x89, 0x02, 0x8b, 0x41, 0x04, 0x89, 0x42, 0x04,
        ]);
        code.extend([0x8b, 0x54, 0x24, 0x08]);
        for k in 0..4u8 {
            // mov eax,[edx+4k]; mov [ecx+0x400+4k],eax
            code.extend([0x8b, 0x42, 4 * k, 0x89, 0x81]);
            code.extend((0x400 + 4 * u32::from(k)).to_le_bytes());
        }
        code.extend([0xd9, 0x41, 0x08, 0xc3]);
        let at = o.rebase(STUBS_D + 0x40);
        o.write(at, &code).unwrap();
        redirect(o, cu::PROJECT_POINT, STUBS_D + 0x40);
        // graphicUiRemapY2: fld [esp+4]; fmul dword [rec + 8]; ret
        let mut code = vec![0xd9, 0x44, 0x24, 0x04, 0xd8, 0x0d];
        code.extend((rec + 8).to_le_bytes());
        code.push(0xc3);
        let at = o.rebase(STUBS_D + 0xc0);
        o.write(at, &code).unwrap();
        redirect(o, cu::UI_REMAP_Y2, STUBS_D + 0xc0);
        logging_stub(o, cu::MAGIC_FILE_GET_MAGIC_ID, STUBS_D + 0x100, logs, 0, 0);
        logging_stub(
            o,
            cu::MS_NUMBER_DRAW_PROCESS,
            STUBS_D + 0x140,
            logs + 0x110,
            0,
            0,
        );
    }
    // the spare heights: three slots of 0xc bytes
    if w.synthetic || !readable(a, ffx_battle::minor::bridge::CHR_EXTRA.get(b), 0x24) {
        let e = scratch(a, b, 0x24);
        set(a, b, ffx_battle::minor::bridge::CHR_EXTRA, e);
    }
    let extra = ffx_battle::minor::bridge::CHR_EXTRA.get(b);
    let camera = {
        let sel = u32::from(ffx_battle::camera::bridge::SELECTED.get(b));
        ffx_battle::camera::bridge::SLOTS_AT.ptr(b).addr
            + (sel & 0xff) * ffx_battle::camera::bridge::CAMERA_SIZE
    };
    let fake = scratch(a, b, 0x1000);
    let ride = w.actor(RIDE);
    poke(a, b, ride, &fake.to_le_bytes());
    poke(a, b, ride + 0x10, &[0]);
    poke(a, b, ride + 0x43f, &[0xff]);
    let mut call = |m: &mut Oracle, f: u32, args: &[u32]| call_original(m, f, args);
    let mut fc = |m: &mut Oracle, f: u32, args: &[u32]| {
        let r = m.call(f, Conv::Cdecl, args).unwrap();
        battle_common::crt::f80_to_f32(r.st0_raw.unwrap())
    };
    for round in 0..200 {
        let (sw, sh) = pick!(
            rng,
            [
                (512u32, 416u32),
                (512, 416),
                (1920, 1080),
                (640, 480),
                (1280, 720),
                (0, 0),
                (0x100_0001, 0x100_0003),
            ]
        );
        poke(a, b, rec, &[sw.to_le_bytes(), sh.to_le_bytes()].concat());
        let factor = pick!(rng, [0.5f32, 0.5, 0.25, 0.45, 1.0]);
        poke(a, b, rec + 8, &factor.to_le_bytes());
        poke(a, b, rec + 0xc, &[0; 4]);
        // the table: points on and off the 512 x 416 screen at this size
        let sx = sw as f32 / 512.0;
        let sy = sh as f32 / 416.0;
        for k in 0..64 {
            let x = pick!(
                rng,
                [rng.range(-60, 600), rng.range(0, 513), 0x200, 0x201, 256]
            );
            let y = pick!(
                rng,
                [
                    rng.range(-60, 480),
                    rng.range(5, 420),
                    10,
                    9,
                    0x196,
                    0x197,
                    rng.range(70, 90),
                    rng.range(180, 400),
                    // the limits at 0.5 (380, 80) and their spare moves
                    390,
                    380,
                    370,
                    70,
                    80,
                    90,
                    -5,
                ]
            );
            #[allow(clippy::cast_possible_truncation)]
            let ox = ((x - 256) as f32 * sx).round() as i32;
            #[allow(clippy::cast_possible_truncation)]
            let oy = ((y - 208) as f32 * sy).round() as i32;
            // (now and then a projection no f32 holds exactly)
            let (ox, oy) = if rng.below(16) == 0 {
                (0x100_0001 + 2 * rng.range(0, 100), oy)
            } else {
                (ox, oy)
            };
            let w = pick!(rng, [1.0f32, 0.5, 2.0, 0.0, -1.0, f32::NAN, 1e-3]);
            let e = table + 16 * k;
            poke(
                a,
                b,
                e,
                &[ox.to_le_bytes(), oy.to_le_bytes(), w.to_le_bytes(), [0; 4]].concat(),
            );
        }
        for k in 0..3 {
            let h = pick!(rng, [0.0f32, 12.5, -30.0, rng.range(-100, 100) as f32]);
            poke(a, b, extra + 12 * k + 8, &h.to_le_bytes());
        }
        fill(a, b, rng, camera, 0x20);
        poke(
            a,
            b,
            logs + 4,
            &pick!(rng, [0u32, 0x130, 0x254, 0x58]).to_le_bytes(),
        );
        poke(a, b, local, &[0; 0x80]);
        let i = pick!(
            rng,
            [
                rng.below(ACTORS),
                rng.below(0x12),
                rng.below(ACTORS) | 0x100
            ]
        );
        let actor = ffx_battle::actor::bridge::ms_get_chr(b, i);
        for at in [0x15u32, 0x718, 0x719] {
            poke(a, b, actor + at, &[pick!(rng, [1u8, 1, 1, 0, rng.byte()])]);
        }
        // (an actor without a character on no other one's would leave the
        // position as the original's frame holds it: every one rides
        // RIDE's character, its optional positions)
        for k in 0..RIDE {
            poke(a, b, w.actor(k) + 0x43f, &[u8::try_from(RIDE).unwrap()]);
            poke(
                a,
                b,
                w.actor(k) + 0x441,
                &[u8::try_from(rng.below(0x20)).unwrap()],
            );
        }
        for k in 0..0x20 {
            let v = [
                rng.range(-200, 200) as f32,
                rng.range(-50, 50) as f32,
                rng.range(-200, 200) as f32,
                pick!(rng, [1.0f32, 0.0, 2.0]),
            ];
            let bytes: Vec<u8> = v.iter().flat_map(|f| f.to_le_bytes()).collect();
            poke(
                a,
                b,
                fake + ffx_chara::attach::bridge::OPTPOS + 16 * k,
                &bytes,
            );
        }
        let f40 = pick!(rng, [1.0f32, 0.0, -0.0, 3.5, -2.0, f32::NAN]);
        poke(a, b, actor + 0x40, &f40.to_le_bytes());
        let lift = pick!(rng, [0.0f32, 10.0, -5.5, f32::NAN]);
        poke(a, b, actor + 0x4ec, &lift.to_le_bytes());
        match round % 4 {
            0 => {
                for _ in 0..12 {
                    let i = pick!(
                        rng,
                        [rng.below(RIDE), rng.below(8), rng.below(RIDE) | 0x100]
                    );
                    poke(a, b, rec + 0xc, &rng.below(64).to_le_bytes());
                    let (x, y) = (out_words, out_words + 4);
                    twin("getSparedLmtPos", a, b, 0x0079_6c30, &[i, x, y], ALL, |m| {
                        cu::get_spared_lmt_pos(m, &stand_in, [&mut call, &mut fc], [i, x, y], local)
                    });
                    let keep = b.read(local, 0x80).unwrap();
                    a.write(local, &keep).unwrap();
                    same(&format!("getSparedLmtPos {i:#x}"), a, b);
                }
            }
            1 | 2 => {
                // its exits mostly open, the camera mostly at level 0
                for _ in 0..12 {
                    let i = pick!(
                        rng,
                        [rng.below(RIDE), rng.below(0x12), rng.below(RIDE) | 0x100]
                    );
                    let actor = ffx_battle::actor::bridge::ms_get_chr(b, i);
                    for at in [0x15u32, 0x718, 0x719] {
                        poke(
                            a,
                            b,
                            actor + at,
                            &[pick!(rng, [1u8, 1, 1, 1, 1, 1, 0, rng.byte()])],
                        );
                    }
                    let f40 = pick!(rng, [1.0f32, 1.0, 3.5, 0.0, -0.0, -2.0, f32::NAN]);
                    poke(a, b, actor + 0x40, &f40.to_le_bytes());
                    let magic = pick!(rng, [0u32, 0, 0, 0, 0x130, 0x254, 0x58]);
                    poke(a, b, logs + 4, &magic.to_le_bytes());
                    // the camera's level: slot 0's (byte 2 of its ID), or
                    // random slots
                    if rng.below(4) == 0 {
                        fill(a, b, rng, camera, 0x20);
                    } else {
                        let mut slots = [0u8; 0x20];
                        slots[2] = u8::try_from(pick!(rng, [0u32, 0, 5, 6, 3])).unwrap();
                        poke(a, b, camera, &slots);
                    }
                    // the projections it will use: on the screen first, then
                    // at the limits (380 and 80 at 0.5) and the spare moves
                    let c = rng.below(64);
                    poke(a, b, rec + 0xc, &c.to_le_bytes());
                    for j in 0..5 {
                        if rng.below(3) == 0 {
                            continue;
                        }
                        let x = rng.range(0, 0x201);
                        let y = if j == 0 {
                            pick!(
                                rng,
                                [rng.range(380, 407), rng.range(10, 81), 390, 85, 0x196]
                            )
                        } else {
                            pick!(rng, [390, 380, 370, 70, 80, 90, rng.range(0, 480)])
                        };
                        #[allow(clippy::cast_possible_truncation)]
                        let ox = ((x - 256) as f32 * sx).round() as i32;
                        #[allow(clippy::cast_possible_truncation)]
                        let oy = ((y - 208) as f32 * sy).round() as i32;
                        let e = table + 16 * ((c + j) & 63);
                        poke(
                            a,
                            b,
                            e,
                            &[ox.to_le_bytes(), oy.to_le_bytes(), 1.0f32.to_le_bytes()].concat(),
                        );
                    }
                    twin("MsGetChrLimitBarFlag", a, b, 0x0079_5250, &[i], ALL, |m| {
                        cu::ms_get_chr_limit_bar_flag(m, &stand_in, &mut call, &mut fc, i, local)
                    });
                    let keep = b.read(local, 0x80).unwrap();
                    a.write(local, &keep).unwrap();
                    same(&format!("MsGetChrLimitBarFlag {i:#x}"), a, b);
                }
            }
            _ => {
                // every actor shown or not, with and without optional points
                // (the ridden one shown at times too: its own optional
                // positions, its character's zeros)
                for k in 0..ACTORS {
                    let p = w.actor(k);
                    poke(a, b, p + 0x10, &[pick!(rng, [0u8, 1, 1])]);
                    poke(a, b, p + 0x442, &[pick!(rng, [0xffu8, 0, 1, 3])]);
                    poke(a, b, p + 0xdf9, &[pick!(rng, [0xffu8, 0, 2])]);
                    let lift = pick!(rng, [0.0f32, 4.0, -2.5, f32::NAN]);
                    poke(a, b, p + 0x4e8, &lift.to_le_bytes());
                }
                if rng.below(2) == 0 {
                    set(
                        a,
                        b,
                        ffx_battle::state::bridge::STATUS,
                        pick!(rng, [0i8, 1, 0x13]),
                    );
                    twin("MsBattleCursorCalc", a, b, 0x0078_0cc0, &[], 0, |m| {
                        cu::ms_battle_cursor_calc(m, &stand_in, &mut call, &mut fc, local);
                        0
                    });
                } else {
                    twin("MsCalcCursorPos", a, b, 0x0079_f380, &[], 0, |m| {
                        cu::ms_calc_cursor_pos(m, &stand_in, &mut call, &mut fc, local);
                        0
                    });
                }
            }
        }
        // the port's frame (the originals' are on their stacks)
        let keep = b.read(local, 0x80).unwrap();
        a.write(local, &keep).unwrap();
        same(&format!("cursors {round} {i:#x}"), a, b);
    }
}

#[test]
fn leaf_cursors_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    cursors(&mut a, &mut b, &mut Xorshift(0x3333_0201), true);
}

#[test]
fn leaf_cursors_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    cursors(&mut a, &mut b, &mut Xorshift(0x3333_0202), false);
}
