//! The battle menu's texts (`ffx_battle::menu`, round 23: `MsMenuGetText`,
//! `MsMenuGetHelp`, `MsGetMenuChrInfo`, which ask the 3-byte
//! `MsGetSaveConfigEnglish` through its Rust bridge) against the original,
//! as in `battle.rs`: the shared battle world, the 17 menu text rows (the
//! snapshot's first, then fake blocks, busy flags and missing blocks), the
//! battle file's text table, the actors' text lines and the kana bit, on
//! the image alone and on both snapshots.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::{World, fake_records, world};
use battle_common::*;
use ffx_battle::menu::bridge as m;
use ffx_oracle::Oracle;

const ROWS: u32 = 17;

fn row(b: &Oracle, i: u32) -> u32 {
    let base = ffx_battle::flow::bridge::MENU_TEXT.ptr(b).addr;
    base.wrapping_add(i.wrapping_mul(m::MENU_TEXT_ROW))
}

/// Fake blocks for every row (missing now and then), busy flags mostly
/// clear; the three rows on either side too (other globals, which the
/// originals read as rows for the texts past the last).
fn dress_rows(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, blocks: &[u32]) {
    for i in (0..ROWS + 3).chain([u32::MAX - 2, u32::MAX - 1, u32::MAX]) {
        let busy = if rng.below(5) == 0 { rng.byte() | 1 } else { 0 };
        let block = if rng.below(6) == 0 {
            0
        } else {
            blocks[rng.below(u32::try_from(blocks.len()).unwrap()) as usize]
        };
        poke(a, b, row(b, i), &[busy]);
        poke(a, b, row(b, i) + 8, &block.to_le_bytes());
    }
}

/// The equipment the name kinds (5, 7, 0xb) read, of names the fake
/// weapon table has.
fn dress_equipment(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let mut slots = vec![(ffx_battle::save::bridge::EQUIPMENT.ptr(b).addr, 200)];
    for g in [
        ffx_battle::save::bridge::SPOILS_EQUIPMENT,
        ffx_battle::save::bridge::EQUIPMENT_7000,
    ] {
        slots.push((g.ptr(b).addr, 8));
    }
    for (at, n) in slots {
        fill(a, b, rng, at, n * 0x16);
        for i in 0..n {
            let id = u16::try_from(rng.below(0x40)).unwrap();
            poke(a, b, at + i * 0x16, &id.to_le_bytes());
        }
    }
}

fn id(rng: &mut Xorshift) -> u32 {
    match rng.below(4) {
        0 => rng.next(),
        1 => rng.below(0x1000),
        _ => rng.below(0x48),
    }
}

fn texts(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let i = match rng.below(6) {
        0 => rng.range(-3, 0x14).cast_unsigned(),
        _ => rng.below(ROWS),
    };
    let id = id(rng);
    let second = [0, 1, rng.next()][rng.below(3) as usize];
    twin(
        "MsMenuGetText",
        a,
        b,
        0x0078_fcf0,
        &[i, id, second],
        ALL,
        |o| m::ms_menu_get_text(o, i, id, second),
    );
    twin(
        "MsMenuGetHelp",
        a,
        b,
        0x0078_fbb0,
        &[i, id, second],
        ALL,
        |o| m::ms_menu_get_help(o, i, id, second),
    );
}

fn chr_info(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, file_text: u32) {
    set(a, b, ffx_battle::save::bridge::MENU, rng.next());
    let table = if rng.below(4) == 0 { 0 } else { file_text };
    set(a, b, m::FILE_TEXT, table);
    let top = ffx_battle::actor::bridge::CHR_TOP;
    let actors = w.actors;
    set(a, b, top, if rng.below(8) == 0 { 0 } else { actors });
    let chr = match rng.below(5) {
        0 => {
            let kind = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa, 0xb, 0xc, 0xf][rng.below(13) as usize];
            kind << 12 | rng.below(0x48)
        }
        1 => rng.next(),
        2 => rng.below(0x100),
        _ => rng.below(ffx_battle::actor::ACTORS),
    };
    // the actor's (or the spare record's) two text lines
    let p = if chr < 0x1000 {
        ffx_battle::actor::bridge::ms_get_chr(b, chr)
    } else {
        w.actor(0)
    };
    let line = |rng: &mut Xorshift| if rng.below(3) == 0 { 0xff } else { rng.byte() };
    let lines = [line(rng), line(rng)];
    // (a weapon record's +0xf74 is past the scratch weapons: left alone)
    if readable(a, p + 0xf74, 2) {
        poke(a, b, p + 0xf74, &lines);
    }
    let k = [0, 1, 2, 3, rng.next()][rng.below(5) as usize];
    twin("MsGetMenuChrInfo", a, b, 0x0078_f7c0, &[chr, k], ALL, |o| {
        m::ms_get_menu_chr_info(o, chr, k)
    });
    same("MsGetMenuChrInfo", a, b);
    set(a, b, top, actors);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    // the snapshot's rows as they are, blocks it can't read cleared
    for i in 0..ROWS {
        let block: u32 = ffx_bridge::Mem::read(&*b, row(b, i) + 8);
        if block != 0 && !readable(a, block, 0x40) {
            poke(a, b, row(b, i) + 8, &[0; 4]);
        }
    }
    for (g, stride) in [
        (m::PLY_ROM, 0x40),
        (m::WEAPON_NAMES, 0x60),
        (ffx_battle::minor::bridge::IMPORTANT_ROM, 0x10),
        (ffx_kernel::bridge::A_ABILITY, 0x40),
    ] {
        if w.synthetic || !readable(a, g.get(b), 0x20) {
            let t = fake_records(a, b, rng, 0x40, stride);
            set(a, b, g, t);
        }
    }
    let real = m::FILE_TEXT.get(b);
    let file_text = if !w.synthetic && readable(a, real, 0x800) {
        real
    } else {
        let t = scratch(a, b, 0x800);
        fill(a, b, rng, t, 0x800);
        t
    };
    let blocks: Vec<u32> = (0..3)
        .map(|_| fake_records(a, b, rng, 0x40, 0x10))
        .collect();
    if w.synthetic {
        dress_equipment(a, b, rng);
    }
    for round in 0..3000 {
        // the snapshot's own rows first
        if w.synthetic || round >= 300 {
            if round % 4 == 0 {
                dress_rows(a, b, rng, &blocks);
            }
            w.shuffle(a, b, rng, round);
        }
        texts(a, b, rng);
        chr_info(a, b, rng, &w, file_text);
    }
    same("menu texts", a, b);
}

#[test]
fn menu_texts_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x23a1_0001), true);
}

#[test]
fn menu_texts_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x23a1_0002), false);
}
