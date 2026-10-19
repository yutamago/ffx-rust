//! An effect's end (`ffx_battle::effect`, round 23: `MsEffectTerminate`)
//! against the original, as in `battle.rs`: the shared battle world, the
//! actor's effect fields and the effect globals drawn at random (often 0,
//! 1 or the values the kinds look for), every kind and a few others, with
//! the actors set up or not, on the image alone and on both snapshots.
//! `MsBattleError` (kind 2 without a ready effect) is a recording stub in
//! both oracles: the original hangs there.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::{ACTORS, World, world};
use battle_common::*;
use ffx_battle::effect::bridge as e;
use ffx_oracle::{Conv, Oracle};

/// `Ms_Gui_BattleChr2Proc` (`0x007de790`), code no test runs: the
/// `MsBattleError` stub.
const GUI_STUBS: u32 = 0x007d_e790;

fn game_call(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x} {args:x?}: {e}"))
        .eax
}

fn small(rng: &mut Xorshift) -> u8 {
    [0, 0, 1, 5, 6, rng.byte()][rng.below(6) as usize]
}

fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, id: u32) {
    let p = ffx_battle::actor::bridge::ms_get_chr(b, id & 0xff);
    if readable(a, p + 0xea4, 4) {
        for o in [0xdfb, 0x3f7, 0x436, 0xe12, 0xe13, 0xe17] {
            poke(a, b, p + o, &[small(rng)]);
        }
        let v = if rng.below(2) == 0 { id } else { rng.next() };
        poke(a, b, p + 0xea4, &v.to_le_bytes());
    }
    let _ = w;
    let word = |rng: &mut Xorshift| [0, 1, 2, rng.next()][rng.below(4) as usize];
    set(a, b, ffx_battle::process::bridge::RESET_WAIT, word(rng));
    set(
        a,
        b,
        ffx_battle::chr_damage::bridge::COMMAND_HOLD2,
        word(rng),
    );
    set(
        a,
        b,
        ffx_battle::death::bridge::STONE_STARTED,
        small(rng).cast_signed(),
    );
    set(
        a,
        b,
        ffx_battle::state::bridge::EVENT_EFFECT_STAT,
        small(rng).cast_signed(),
    );
    set(
        a,
        b,
        ffx_battle::minor::bridge::EFFECT_CAMERA_MODE,
        small(rng),
    );
    let call = if rng.below(2) == 0 { id } else { rng.next() };
    set(
        a,
        b,
        ffx_battle::minor::bridge::EFFECT_CAMERA_CALL,
        call.cast_signed(),
    );
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x20);
    for o in [&mut *a, &mut *b] {
        recording_stub(
            o,
            ffx_battle::init::bridge::MS_BATTLE_ERROR,
            GUI_STUBS,
            rec,
            1,
            0,
        );
    }
    let top = ffx_battle::actor::bridge::CHR_TOP;
    let actors = top.get(b);
    let kinds = [2, 4, 7, 8, 9, 10, 0xe, 0xf, 0x80, 0, 3, 0x81, 0xff];
    for _ in 0..6000 {
        let kind = kinds[rng.below(13) as usize];
        let chr = [rng.below(ACTORS), rng.below(0x100)][rng.below(2) as usize];
        let high = if rng.below(4) == 0 {
            rng.next() << 16
        } else {
            0
        };
        let id = high | kind << 8 | chr;
        dress(a, b, rng, &w, id);
        // no actors: the kinds that read none only (the original reads
        // address 0 for the rest)
        let none = rng.below(8) == 0 && [10, 0xe, 0x80, 0, 3].contains(&kind);
        if none {
            set(a, b, top, 0);
        }
        twin("MsEffectTerminate", a, b, 0x0078_8920, &[id], 0, |m| {
            let mut call = game_call;
            e::ms_effect_terminate(m, &mut call, id);
            0
        });
        same("MsEffectTerminate", a, b);
        if none {
            set(a, b, top, actors);
        }
    }
}

#[test]
fn effect_ends_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x23e1_0001), true);
}

#[test]
fn effect_ends_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x23e1_0002), false);
}
