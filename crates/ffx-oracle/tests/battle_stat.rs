//! `MsGetChrStatInfo` (`ffx_battle::stat`) against the original, as in
//! `battle.rs`: every property over random actor sets, on the image alone
//! (actors of random bytes, fake characters and info records) and on both
//! snapshots (their actors when a battle is live). `_CIcos`, `_CIsin` and
//! `_CIsqrt` (for property 0xf4's `MsCheckBtlPos`) are the same exact
//! stand-ins in both oracles, as in `battle_pos.rs`.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::crt::{patch, stand_in};
use battle_common::world::{ACTOR, ACTORS, World, quiet, world};
use battle_common::*;
use ffx_battle::stat::bridge as s;
use ffx_oracle::Oracle;

fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(4) {
        0 => quiet(rng.next()),
        1 => rng.below(0x8000_0000) | 0x4000_0000,
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 37.0).to_bits(),
    }
}

/// Fake characters, info records and a position file for the synthetic
/// actors.
struct Fakes {
    chars: u32,
    infos: u32,
    file: u32,
}

fn fakes(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> Fakes {
    let chars = scratch(a, b, 0x3000);
    let v: Vec<u8> = (0..0xc00)
        .flat_map(|_| quiet(rng.next()).to_le_bytes())
        .collect();
    poke(a, b, chars, &v);
    let infos = scratch(a, b, 0x480);
    fill(a, b, rng, infos, 0x480);
    // one group, not flat: kind 3 reaches the defense positions
    let file = scratch(a, b, 0x100);
    poke(a, b, file, &[0, 1]);
    set(a, b, ffx_battle::pos::bridge::POS_FILE, file);
    Fakes { chars, infos, file }
}

/// Random actors: every byte, then the pointers (characters, info) and
/// the floats; `+0x6dd` only with a character (it would recurse without).
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, f: &Fakes) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        fill(a, b, rng, p, ACTOR);
        let chr = if rng.below(3) == 0 {
            0
        } else {
            f.chars + rng.below(3) * 0x1000
        };
        poke(a, b, p, &chr.to_le_bytes());
        let info = if rng.below(4) == 0 {
            0
        } else {
            f.infos + rng.below(4) * 0x120
        };
        poke(a, b, p + 0xf88, &info.to_le_bytes());
        // the floats, the defense position (0x3b0) among them, without
        // signalling NaNs
        for o in [
            0x3a4, 0x3a8, 0x3b0, 0x3b4, 0x3b8, 0x3bc, 0x4e8, 0x4ec, 0x4f0, 0x534,
        ] {
            poke(a, b, p + o, &float(rng).to_le_bytes());
        }
        poke(a, b, p + 0x6dd, &[u8::from(chr != 0 && rng.below(3) == 0)]);
        // healthy now and then: the conditions on many clear bytes
        if rng.below(2) == 0 {
            for o in [
                0xdce, 0x608, 0x609, 0xdcc, 0x617, 0x60d, 0x60b, 0x60c, 0x613,
            ] {
                poke(a, b, p + o, &[0]);
            }
            poke(
                a,
                b,
                p + 0x606,
                &(u16::try_from(rng.below(0x1_0000)).unwrap() & 0xf0ff).to_le_bytes(),
            );
            poke(
                a,
                b,
                p + 0x616,
                &(u16::try_from(rng.below(0x1_0000)).unwrap() & 0xfbff).to_le_bytes(),
            );
            poke(a, b, p + 0xdc8, &[1]);
        }
        // HP and MP near their maxima now and then
        if rng.below(2) == 0 {
            for at in [0x594, 0x598, 0x5d0, 0x5d4] {
                poke(a, b, p + at, &rng.range(-5, 20).to_le_bytes());
            }
        }
    }
    for g in [s::STAT_INVERT_1B, ffx_battle::damage::bridge::DEBUG_NO_COST] {
        let v = u8::from(rng.below(2) == 0);
        set(a, b, g, v);
    }
    set(
        a,
        b,
        ffx_battle::damage::bridge::GIL_AMOUNT,
        rng.word().cast_signed(),
    );
    set(a, b, ffx_bridge::x87::SSE2_AVAILABLE, rng.below(2));
    let _ = f.file;
}

/// Property 0xf4 compares positions the original leaves on its stack when
/// unwritten: every actor gets a character, a defense position past the
/// file's checks (`+0x4fc` group 0 kind 3) and no ride.
fn positions(a: &mut Oracle, b: &mut Oracle, w: &World, f: &Fakes) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        poke(a, b, p, &f.chars.to_le_bytes());
        poke(a, b, p + 0x43f, &[0xff]);
        poke(a, b, p + 0x4fc, &[0, 3, 0]);
        poke(a, b, p + 0x6d4, &[1]);
    }
}

fn call(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, kind: u32) {
    let chr = rng.below(0x24);
    let selector = match rng.below(8) {
        0 => (-21i32).cast_unsigned(),
        1 => (-5i32).cast_unsigned(),
        2 => (-15i32).cast_unsigned(),
        3 => (-14i32).cast_unsigned(),
        4 => (-13i32).cast_unsigned(),
        _ => rng.below(0x24),
    };
    let out = if rng.below(4) == 0 { 0 } else { w.out };
    let args = [chr, kind, selector, out];
    twin("MsGetChrStatInfo", a, b, 0x007b_2dc0, &args, ALL, |m| {
        s::ms_get_chr_stat_info(m, &stand_in, chr, kind, selector, out)
    });
    same("MsGetChrStatInfo", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    patch(a);
    patch(b);
    let w = world(a, b, rng, synthetic);
    let f = fakes(a, b, rng);
    for round in 0..400 {
        if w.synthetic {
            dress(a, b, rng, &w, &f);
        }
        for kind in 0..0x15c {
            if kind == 0xf4 && !w.synthetic {
                continue;
            }
            if kind == 0xf4 {
                positions(a, b, &w, &f);
            }
            call(a, b, rng, &w, kind);
        }
        let _ = round;
    }
}

#[test]
fn stat_info_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x5747_0001), true);
}

#[test]
fn stat_info_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x5747_0002), false);
}
