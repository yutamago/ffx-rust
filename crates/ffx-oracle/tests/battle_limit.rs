//! Overdrive hits handed out (`ffx_battle::limit`, round 24:
//! `MsMenuLimitSet`, `MsMenuLimitSet_007b0470`) against the original, as in
//! `battle.rs`: the shared battle world (actors shuffled), each actor's two
//! results blocks often the user's (its index and command number, a
//! drain block now and then), the user's command of one of the overdrive
//! kinds (its `+0x2b` and `+0x1c` drawn), the command cue entry and the
//! minigame's count, on the image alone and on both snapshots.
//! `MsCalcDamage`, `MsCalcDrainResult`, `MsGetTargetRandomList` and
//! `brnd` run as the originals and the ports; `TOGetFFXLang` and the
//! achievements return fixed values, `MsAtelRequestMagicCamera` is a
//! recording stub. The ports' scratch (the original's stack locals) is
//! cleared after each call.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACHIEVEMENT, ACTORS, TO_GET_FFX_LANG, World, world};
use battle_common::*;
use ffx_battle::limit::bridge as l;
use ffx_oracle::{Conv, Oracle};

/// `Ms_Gui_BattleChr2Proc` (`0x007de790`), code no test runs: the stub.
const GUI_STUBS: u32 = 0x007d_e790;
const KINDS: [u8; 11] = [
    0x20, 0x22, 0x25, 0x30, 0x32, 0x40, 0x42, 0x50, 0x52, 0, 0x21,
];

fn game_call(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x} {args:x?}: {e}"))
        .eax
}

/// The user's command (kind, hit count, flag), its cue entry, every
/// actor's results blocks.
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, chr: u32) -> u32 {
    let user = w.actor(chr);
    let id = 0x3000 + rng.below(0x40);
    poke(
        a,
        b,
        user + 0xf5c,
        &u16::try_from(id).unwrap().to_le_bytes(),
    );
    let (command, _) = ffx_battle::command::bridge::get_command(b, u32::MAX, 0, id);
    let kind = KINDS[rng.below(11) as usize];
    poke(a, b, command + 0x58, &[kind]);
    poke(a, b, command + 0x2b, &[[0, 1, 3, 8][rng.below(4) as usize]]);
    let flags: u32 = ffx_bridge::Mem::read(&*b, command + 0x1c);
    let flags = if rng.below(2) == 0 {
        flags | 0x8000
    } else {
        flags & !0x8000
    };
    poke(a, b, command + 0x1c, &flags.to_le_bytes());
    // the fixed commands 0x30ec and 0x310d (their +0x2b)
    for fixed in [0x30ec, 0x310d] {
        let (c, _) = ffx_battle::command::bridge::get_command(b, u32::MAX, 0, fixed);
        if c != 0 && c != command {
            // small: a negative difference adds hits (results stay below 16)
            poke(a, b, c + 0x2b, &[[0, 1, 2][rng.below(3) as usize]]);
        }
    }
    // its own cue entry (+0x72c)
    poke(a, b, user + 0xde5, &[0xff]);
    let number = rng.below(4);
    let entry = user + 0x72c;
    poke(
        a,
        b,
        entry + 2,
        &[
            u8::try_from(number).unwrap(),
            u8::try_from(rng.below(5)).unwrap(),
        ],
    );
    // a party of distinct actors (MsGetTargetRandomList lists the party's
    // first; a repeated one would count its hits again)
    let party: [u8; 7] = [0, 1, 2, 0xff, 0xff, 0xff, 0xff];
    let mut party = party;
    party.swap(0, rng.below(3) as usize);
    poke(a, b, ffx_battle::menu::bridge::PARTY.ptr(b).addr, &party);
    let mut drain = rng.below(3) == 0;
    // at most 6 blocks of the user's (the random target list holds at
    // most 0x40 hits; past that the original hangs in MsBattleError)
    let mut mine_left = 6;
    for i in 0..ACTORS {
        let p = w.actor(i);
        // DmgCalc_Bribe divides by the target's max HP (a fault on 0)
        for at in [0x594, 0x598] {
            let v: u32 = ffx_bridge::Mem::read(&*b, p + at);
            if v == 0 {
                poke(a, b, p + at, &1u32.to_le_bytes());
            }
        }
        for k in 0..2 {
            let s = p + l::RESULTS + k * l::RESULTS_SIZE;
            fill(a, b, rng, s, 0x18);
            let mine = mine_left > 0 && rng.below(3) != 0;
            if mine {
                mine_left -= 1;
            }
            let who = if mine {
                chr
            } else {
                (chr + 1 + rng.below(ACTORS - 1)) % ACTORS
            };
            let num = if mine || rng.below(2) == 0 {
                number
            } else {
                rng.below(4)
            };
            let done = u8::try_from(rng.below(4)).unwrap();
            let wanted = done + u8::try_from(rng.below(6)).unwrap();
            let bits = if drain && rng.below(8) == 0 {
                drain = false;
                4
            } else {
                rng.byte() & !4
            };
            poke(
                a,
                b,
                s,
                &[
                    done,
                    wanted,
                    u8::try_from(who).unwrap(),
                    u8::try_from(num).unwrap(),
                    bits,
                ],
            );
            poke(a, b, s + 0x16, &[[0, 1][rng.below(2) as usize]]);
        }
    }
    user
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x300);
    for o in [&mut *a, &mut *b] {
        let at = o.rebase(ACHIEVEMENT);
        o.stub_return(at, 0).unwrap();
        let at = o.rebase(TO_GET_FFX_LANG);
        o.stub_return(at, 1).unwrap();
        recording_stub(o, l::MS_ATEL_REQUEST_MAGIC_CAMERA, GUI_STUBS, rec, 9, 0);
    }
    let frame = scratch(a, b, (l::LIMIT_SCRATCH_WORDS * 4) as usize);
    for round in 0..1500 {
        w.shuffle(a, b, rng, round);
        let chr = [0, 2, rng.below(8), rng.below(ACTORS)][rng.below(4) as usize];
        let user = dress(a, b, rng, &w, chr);
        let count = [0, 1, 2, 5, -1, rng.range(-3, 12)][rng.below(6) as usize];
        if rng.below(4) == 0 {
            // the minigame's window: done or not, its two floats
            poke(a, b, user + 0xd24, &[[0, 1][rng.below(2) as usize]]);
            let x = (f32::from(i16::try_from(rng.range(-50, 50)).unwrap()) / 3.0).to_bits();
            let y = rng.next() & 0x7f7f_ffff;
            let c = count.cast_unsigned();
            let high = rng.next() & 0xffff_ff00;
            twin(
                "MsMenuLimitSet_007b0470",
                a,
                b,
                0x007b_0470,
                &[high | chr, c, x, y],
                ALL,
                |m| {
                    let mut call = game_call;
                    l::ms_menu_limit_start(m, &mut call, high | chr, c, x, y, frame)
                },
            );
        } else {
            twin(
                "MsMenuLimitSet",
                a,
                b,
                0x007a_fe00,
                &[chr, user, count.cast_unsigned()],
                ALL,
                |m| {
                    let mut call = game_call;
                    l::ms_menu_limit_set(m, &mut call, chr, user, count, frame)
                },
            );
        }
        b.write(frame, &[0; (l::LIMIT_SCRATCH_WORDS * 4) as usize])
            .unwrap();
        same("MsMenuLimitSet", a, b);
    }
}

#[test]
fn limits_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x24a1_0001), true);
}

#[test]
fn limits_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x24a1_0002), false);
}
