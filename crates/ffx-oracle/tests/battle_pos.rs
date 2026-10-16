//! `MsBtlGetPos` (`ffx_battle::pos`) against the original, as in
//! `battle.rs`: fake position files on the image alone and on both
//! snapshots (the battle's own file first when there is one).
//! `MsGetSummonerPos` runs as the original's in both oracles.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::*;
use ffx_battle::pos::bridge as p;
use ffx_oracle::{Conv, Oracle};

fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(6) {
        0 => quiet(rng.next()),
        1 => 0,
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 97.0).to_bits(),
    }
}

/// A position file of `groups` groups at `at` (0x800 bytes): records with
/// small counts and tables after them.
fn file(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, at: u32) {
    let mut f = vec![0u8; 0x800];
    for c in f.as_chunks_mut::<4>().0 {
        c.copy_from_slice(&float(rng).to_le_bytes());
    }
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
    // the header overlaps the first record's first two bytes
    f[0] = u8::from(flat);
    f[1] = u8::try_from(rng.below(5)).unwrap();
    poke(a, b, at, &f);
}

fn call(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, actor: u32, vec: u32) {
    let write = rng.below(2);
    let group = rng.range(-1, 6);
    let kind = rng.range(0, 16);
    let index = rng.range(-1, 5);
    let who = if rng.below(3) == 0 { 0 } else { actor };
    let v = if rng.below(5) == 0 { 0 } else { vec };
    let vals: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
    poke(a, b, vec, &vals);
    let args = [
        write,
        who,
        group.cast_unsigned(),
        kind.cast_unsigned(),
        index.cast_unsigned(),
        v,
    ];
    twin("MsBtlGetPos", a, b, 0x007a_bfe0, &args, ALL, |m| {
        p::ms_btl_get_pos(
            m,
            write,
            who,
            group,
            kind,
            index,
            v,
            |m, actor, index, k, vec| {
                let at = m.rebase(0x007a_c2b0);
                m.call(
                    at,
                    Conv::Cdecl,
                    &[actor, index.cast_unsigned(), k.cast_unsigned(), vec],
                )
                .unwrap();
            },
        )
        .cast_unsigned()
    });
    same("MsBtlGetPos", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let at = scratch(a, b, 0x800);
    let actor = scratch(a, b, 0xf90);
    let vec = scratch(a, b, 0x10);
    // actors of our own for MsGetSummonerPos (none a summon needing the
    // trigonometric imports: +4 stays 0)
    let actors = scratch(a, b, 0x1f * 0xf90);
    set(a, b, ffx_battle::actor::bridge::CHR_TOP, actors);
    for round in 0..3000 {
        if round % 4 == 0 {
            file(a, b, rng, at);
            set(a, b, p::POS_FILE, at);
            fill(a, b, rng, actor, 0xf90);
            // an actor in a defense position now and then; never a summoner
            // whose position needs the trigonometric imports
            poke(
                a,
                b,
                actor + 0x6d4,
                &[if rng.below(4) == 0 { 1 } else { 0xff }],
            );
            poke(a, b, actor + 4, &0u32.to_le_bytes());
            let v: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
            poke(a, b, actor + 0x3b0, &v);
            for o in [0x4fc, 0x4fd, 0x4fe] {
                poke(a, b, actor + o, &[u8::try_from(rng.below(4)).unwrap()]);
            }
        }
        call(a, b, rng, actor, vec);
    }
}

#[test]
fn positions_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x9999_0001));
}

#[test]
fn positions_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x1000_9999);
    let f = p::POS_FILE.get(&b);
    if readable(&mut a, f, 0x100) {
        let actor = scratch(&mut a, &mut b, 0xf90);
        poke(&mut a, &mut b, actor + 0x6d4, &[0xff]);
        let vec = scratch(&mut a, &mut b, 0x10);
        for _ in 0..500 {
            // reads only, on the real file
            let group = rng.range(0, 4);
            let kind = rng.range(2, 12);
            let index = rng.range(0, 4);
            let args = [
                0,
                actor,
                group.cast_unsigned(),
                kind.cast_unsigned(),
                index.cast_unsigned(),
                vec,
            ];
            twin(
                "MsBtlGetPos (live)",
                &mut a,
                &mut b,
                0x007a_bfe0,
                &args,
                ALL,
                |m| {
                    p::ms_btl_get_pos(m, 0, actor, group, kind, index, vec, |_, _, _, _, _| {})
                        .cast_unsigned()
                },
            );
            same("MsBtlGetPos (live)", &mut a, &mut b);
        }
    }
    check(&mut a, &mut b, &mut rng);
}
