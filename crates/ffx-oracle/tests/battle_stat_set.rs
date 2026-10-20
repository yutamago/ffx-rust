//! `MsSetChrStatInfo` (`ffx_battle::stat_set`) against the original, as in
//! `battle.rs`: every property in turn with random values and selectors,
//! on actors of our own (the image) and the live ones (a battle snapshot),
//! the battle's other functions it calls as logging stubs (their counts
//! and arguments compared) and the map names it looks for.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACTORS, quiet, world};
use battle_common::*;
use ffx_battle::stat_set::bridge as s;
use ffx_oracle::{Conv, Oracle};

/// `MsBtlPosPrintBin` and `FUN_007c1790`, unreachable: room for the stubs.
const STUBS_A: u32 = 0x007c_9f10;
const STUBS_B: u32 = 0x007c_1790;

/// The callees as logging stubs: (function, arguments).
const CALLEES: [(u32, u8); 27] = [
    (s::MS_ALIVE_PROCESS, 3),
    (s::MS_SET_CHR_IMAGE_DAMAGE_ATTACK, 1),
    (s::MS_SET_CHR_VISIBLE_MAIN, 2),
    (s::CH_EFFECT_SET_EFFECT_LEVEL, 2),
    (s::BATTLE_FIRST_SELECTED_ACTOR, 4),
    (s::MS_SET_CHR_BODY_HIT, 3),
    (s::MS_BLOW_PROCESS, 2),
    (s::MS_SET_CHR_VISIBLE_APPEAR, 2),
    (s::MS_SET_CHR_VISIBLE_CAMERA, 2),
    (s::MS_SET_CHR_VISIBLE, 2),
    (s::MS_SET_CHR_SHADOW_MODE, 3),
    (s::MS_MOVE_RESET, 1),
    (s::MS_MOTION_SPEED_SET_NORMAL, 1),
    (s::CH_SET_THICKNESS_BTL_CAM, 2),
    (s::CH_SET_CAM_HEIGHT, 2),
    (s::CH_SET_FIX_HEIGHT, 2),
    (s::MS_ADD_YOUJINBOU_COM, 1),
    (s::MS_MOTION_SPEED_RESET, 1),
    (s::MS_SET_SAVE_PLY_JOIN, 2),
    (s::MS_SET_ETERNAL_RELIFE, 1),
    (s::MS_STATUS_EFFECT_CHECK, 1),
    (s::MS_SET_CHR_WEAK, 2),
    (s::MS_CHECK_AUTO_STATUS, 2),
    (s::MS_DAMAGE_CHECK_DEATH, 4),
    (s::MS_RESET_AUTO_STATUS, 1),
    (s::MS_SET_RAM_CHR_PARAM, 1),
    (s::MS_SET_AUTO_STATUS, 1),
];

fn call_original(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args).unwrap().eax
}

/// A float for the actor's scale fields: plain, zero or special (no
/// signalling NaNs: the emulator keeps them).
fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(6) {
        0 => [0, 0x8000_0000, 0x7f80_0000, 0x7fc0_0000, 0x3f80_0000][rng.below(5) as usize],
        1 => quiet(rng.next()),
        _ => (f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 37.0).to_bits(),
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    // each stub's log: count, result, 16 calls of its arguments
    let rec = scratch(a, b, 0x110 * CALLEES.len());
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(CALLEES) {
            let at = if k < 21 {
                STUBS_A + 0x40 * k
            } else {
                STUBS_B + 0x40 * (k - 21)
            };
            logging_stub(o, f, at, rec + 0x110 * k, n, 0);
        }
    }
    let chars = scratch(a, b, 0x100);
    let infos = scratch(a, b, 0x200);
    let name = a.rebase(ffx_battle::read::bridge::FILE_NAME);
    let kinds = 0x160u32;
    let mut hits = 0;
    for round in 0..18 * kinds {
        let kind = round % kinds;
        let pass = round / kinds % 6;
        // the actors: random fields, a character or none, an info record
        // or none (on the image; the snapshot's own otherwise)
        for i in 0..ACTORS {
            let p = w.actor(i);
            if w.synthetic || rng.below(3) == 0 {
                fill(a, b, rng, p + 0x10, 0xf70);
                for o in [0x3a4, 0x3a8] {
                    poke(a, b, p + o, &float(rng).to_le_bytes());
                }
            }
            if w.synthetic {
                let ch = [0, chars + 0x10 * (i % 8)][rng.below(3).min(1) as usize];
                poke(a, b, p, &ch.to_le_bytes());
                let info = [0, infos + 0x40 * (i % 8)][rng.below(3).min(1) as usize];
                poke(a, b, p + 0xf88, &info.to_le_bytes());
            }
        }
        fill(a, b, rng, infos, 0x200);
        // motion speeds 0x14b leaves alone: 7, 0x10, held (bit 7)
        for i in 0..ACTORS {
            if rng.below(2) == 0 {
                poke(
                    a,
                    b,
                    w.actor(i) + 0x415,
                    &[[7u8, 0x10, 0x87, 3][rng.below(4) as usize]],
                );
            }
        }
        // the stubs' results: the first selected actor's
        for k in 0..u32::try_from(CALLEES.len()).unwrap() {
            poke(a, b, rec + 0x110 * k + 4, &rng.below(0x22).to_le_bytes());
        }
        // the battle file: one of the two maps or another
        let file = [&b"sins07_07\0"[..], b"znkd09_00\0", b"xyz00_00\0"][rng.below(3) as usize];
        poke(a, b, name, file);
        set(
            a,
            b,
            ffx_battle::parse::bridge::PARSED_COMMAND,
            [0x60ee, rng.next()][rng.below(2) as usize],
        );
        set(a, b, s::ZNKD_HP, rng.below(2));
        let chr = [0x14, rng.below(0x22)][rng.below(2) as usize];
        let selector = [rng.below(0x22), rng.range(-24, 0).cast_unsigned()][rng.below(2) as usize];
        // each property once with each edge value, then at random
        let v = match pass {
            0 => 0,
            1 => 0xffff_ffff,
            2 => 0x0100_0001,
            3 => [1, 4, 100, 0x100][rng.below(4) as usize],
            4 => rng.range(-300, 300).cast_unsigned(),
            _ => [rng.next(), rng.below(0x200)][rng.below(2) as usize],
        };
        let kind = if rng.below(8) == 0 {
            [0x57, 0, 0x88][rng.below(3) as usize]
        } else {
            kind
        };
        let picked = ffx_battle::actor::bridge::ms_get_chr_bit(b, chr, selector, 0, 1);
        hits += u32::from(picked != 0);
        twin(
            "MsSetChrStatInfo",
            a,
            b,
            0x007b_4b70,
            &[chr, kind, selector, v],
            ALL,
            |m| s::ms_set_chr_stat_info(m, &mut call_original, chr, kind, selector, v),
        );
        same(&format!("MsSetChrStatInfo {kind:#x} {v:#x}"), a, b);
    }
    eprintln!("rounds with actors: {hits} of {}", 18 * kinds);
    assert!(hits > kinds, "too few rounds pick actors");
}

#[test]
fn stat_sets_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x2929_1201), true);
}

#[test]
fn stat_sets_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x2929_1202), false);
}
