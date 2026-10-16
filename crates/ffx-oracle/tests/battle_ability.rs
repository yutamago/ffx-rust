//! The usable commands and the menu's command lists (`ffx_battle::ability`)
//! against the original, as in `battle.rs`: the shared battle world with
//! random save bits, inventory, commands and lists, on the image alone and
//! on both snapshots.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::{ACTORS, World, fake_records, world};
use battle_common::*;
use ffx_battle::ability::bridge as ab;
use ffx_battle::save::bridge as sv;
use ffx_oracle::Oracle;

/// Command records of the kinds the functions tell apart.
fn commands(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, table: u32) {
    for i in 0..0x140 {
        let r = table + 0x20 + i * 0x60;
        let group = [5u8, 6, 7, 0xa, 0xf, 0x11, 1, 2][rng.below(8) as usize];
        let kind = [8u8, 0, 1, 0x48, rng.byte()][rng.below(5) as usize];
        let owner = if rng.below(2) == 0 {
            0xff
        } else {
            u8::try_from(rng.below(0x13)).unwrap()
        };
        let sub = [1u8, 2, 3, 4, 0xe, 5, group][rng.below(7) as usize];
        poke(a, b, r + 0x16, &[kind, group, sub, owner]);
    }
}

fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, mem2: u32) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        fill(a, b, rng, p + 0x664, 0x50);
        poke(a, b, p + 0x6cb, &[u8::try_from(rng.below(4)).unwrap()]);
        fill(a, b, rng, p + 0x71a, 8);
        for k in 0..4 {
            poke(a, b, p + 0x722 + k, &[u8::try_from(rng.below(9)).unwrap()]);
        }
        for o in [0x10, 0xdc8, 0xdca, 0xdcd, 0xdd9] {
            poke(a, b, p + o, &[u8::from(rng.below(2) == 0)]);
        }
        fill(a, b, rng, p + 0xe, 2);
    }
    // the lists: command IDs, free slots and others
    for k in 0..0x5070 / 2 {
        let v: u16 = match rng.below(4) {
            0 => 0xff,
            1 => u16::try_from(rng.below(0x10000)).unwrap(),
            _ => u16::try_from(0x3000 + rng.below(0x140)).unwrap(),
        };
        poke(a, b, mem2 + 2 * k, &v.to_le_bytes());
    }
    if w.synthetic {
        for (g, len) in [
            (sv::PLY_SAVE, 0x94 * 0x13),
            (sv::ITEM_COUNTS, 0x70),
            (ab::SUMMON_SAVE, 0x94 * 10),
            (sv::EQUIPMENT, 200 * 0x16),
        ] {
            let at = g.ptr(b).addr;
            fill(a, b, rng, at, len);
        }
        let at = sv::SHARED_COMMANDS.ptr(b).addr;
        fill(a, b, rng, at, 0x20);
        // items mostly absent: the 6/0x11 groups then hang on one
        let at = sv::ITEM_IDS.ptr(b).addr;
        for i in 0..0x70 {
            let v: u16 = if rng.below(40) == 0 { 0x2000 } else { 0 };
            poke(a, b, at + 2 * i, &v.to_le_bytes());
        }
    }
    let party: [u8; 7] = std::array::from_fn(|_| {
        if rng.below(3) == 0 {
            0xff
        } else {
            u8::try_from(rng.below(0x12)).unwrap()
        }
    });
    set(a, b, ab::PARTY, party);
    let extra: [u8; 17] = std::array::from_fn(|_| {
        if rng.below(3) == 0 {
            0xff
        } else {
            u8::try_from(rng.below(0x12)).unwrap()
        }
    });
    set(a, b, ab::PARTY_EXTRA, extra);
    set(
        a,
        b,
        ab::LIST_SLOT,
        i8::try_from(rng.range(-1, 0x12)).unwrap(),
    );
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, mem2: u32, out: u32) {
    let chr = rng.below(0x12);
    let id = if rng.below(4) == 0 {
        rng.next() & 0xffff
    } else {
        0x3000 + rng.below(0x140)
    };
    twin(
        "MsGetChrCommandFlag",
        a,
        b,
        0x0079_ad30,
        &[chr, id],
        ALL,
        |m| ab::ms_get_chr_command_flag(m, chr, id),
    );
    let on = rng.below(2);
    twin(
        "MsSetRamChrCommand",
        a,
        b,
        0x0079_c070,
        &[chr, id, on],
        0,
        |m| {
            ab::ms_set_chr_command_flag(m, chr, id, on);
            0
        },
    );
    same("MsSetRamChrCommand", a, b);
    let n = rng.range(-1, 0x21);
    let list = mem2 + 2 * rng.below(0x100);
    twin(
        "MsSortCommandList",
        a,
        b,
        0x007a_d970,
        &[list, out, n.cast_unsigned()],
        0,
        |m| {
            ab::ms_sort_command_list(m, list, out, n);
            0
        },
    );
    same("MsSortCommandList", a, b);
    let lists = mem2 + 0x478 * rng.below(0x12);
    twin("MsSortCommand", a, b, 0x007a_efb0, &[lists], 0, |m| {
        ab::ms_sort_command(m, lists);
        0
    });
    same("MsSortCommand", a, b);
    let kind = rng.range(-1, 0x16);
    let count = if rng.below(4) == 0 { 0 } else { out + 0x80 };
    twin(
        "MsGetBattleList",
        a,
        b,
        0x007b_6c50,
        &[kind.cast_unsigned(), count],
        ALL,
        |m| ab::ms_get_battle_list(m, kind, count),
    );
    same("MsGetBattleList", a, b);
    let actor = w.actor(chr);
    twin(
        "MsSetRamChrAbility",
        a,
        b,
        0x0079_bb50,
        &[chr, actor],
        0,
        |m| {
            ab::ms_set_ram_chr_ability(m, chr, actor);
            0
        },
    );
    same("MsSetRamChrAbility", a, b);
    let id = match rng.below(4) {
        0 => rng.next() & 0xffff,
        1 => 0x3060 + rng.below(0x100),
        _ => 0x3000 + rng.below(0x60),
    };
    twin(
        "MsSetSaveCommand",
        a,
        b,
        0x0078_5c50,
        &[chr, id, on],
        ALL,
        |m| ab::ms_set_save_command(m, chr, id, on),
    );
    same("MsSetSaveCommand", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    // 0x140 commands (the world's fake table has 0x40)
    if w.synthetic {
        let t = fake_records(a, b, rng, 0x140, 0x60);
        set(a, b, ffx_kernel::bridge::COMMAND, t);
        commands(a, b, rng, t);
        if !readable(a, ffx_battle::actor::bridge::MON_TOP.get(b), 0x10) {
            set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
        }
    }
    let mem2 = scratch(a, b, 0x5400);
    set(a, b, ffx_battle::misc::bridge::ITEM_MEM2, mem2);
    let out = scratch(a, b, 0x100);
    for round in 0..300 {
        w.shuffle(a, b, rng, round);
        dress(a, b, rng, &w, mem2);
        calls(a, b, rng, &w, mem2, out);
    }
}

#[test]
fn abilities_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xab11_0001), true);
}

#[test]
fn abilities_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xab11_0002), false);
}
