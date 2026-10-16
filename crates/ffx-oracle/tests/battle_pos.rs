//! `MsBtlGetPos` (`ffx_battle::pos`) against the original, as in
//! `battle.rs`: fake position files on the image alone and on both
//! snapshots (the battle's own file first when there is one).
//! `MsGetSummonerPos` too, with `_CIcos` and `_CIsin` (imports the oracle
//! lacks) replaced at their thunks by the same exact stand-ins in both,
//! `x²` and `2x` (`fmul st, st; ret`, `fadd st, st; ret`): the angle's path
//! and the arithmetic around them are checked, the C runtime's own sine and
//! cosine being the game's in both.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::*;
use ffx_battle::pos::bridge as p;
use ffx_oracle::Oracle;

const ACTOR: u32 = 0xf90;

fn patch_trig(o: &mut Oracle) {
    let cos = o.rebase(0x0094_98c2);
    o.write(cos, &[0xd8, 0xc8, 0xc3]).unwrap();
    let sin = o.rebase(0x0094_98c8);
    o.write(sin, &[0xd8, 0xc0, 0xc3]).unwrap();
    // `_CIsqrt` (MsPointLen) as the CPU's: `fsqrt; ret`
    let sqrt = o.rebase(0x0094_97b8);
    o.write(sqrt, &[0xd9, 0xfa, 0xc3]).unwrap();
}

/// The stand-ins as the Rust gets them (`f32` bits in and out).
fn stand_in(t: p::Trig, x: u32) -> u32 {
    use ffx_bridge::x87::{add, load, mul, store};
    match t {
        p::Trig::Cos => store(mul(load(x), load(x))),
        p::Trig::Sin => store(add(load(x), load(x))),
    }
}

/// The sisters' `MsDivVector` leaves the caller's saved registers in
/// `VF_B`'s last three words: those are the harness's in `a`.
fn caller_registers(a: &mut Oracle, b: &mut Oracle) {
    let at = ffx_battle::vector::bridge::VF_B.ptr(a).addr + 4;
    let w = a.read(at, 12).unwrap();
    b.write(at, &w).unwrap();
}

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
    // at least one group: the sisters' reads must not fail
    f[1] = u8::try_from(1 + rng.below(4)).unwrap();
    poke(a, b, at, &f);
}

fn call(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, actor: u32, vec: u32) {
    let write = rng.below(2);
    let group = rng.range(-1, 6);
    let kind = rng.range(0, 16);
    let index = rng.range(-1, 5);
    // no actor for a summoner's kind faults on the sisters' +0x518
    let who = if rng.below(3) == 0 && !(12..15).contains(&kind) {
        0
    } else {
        actor
    };
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
                p::ms_get_summoner_pos(
                    m,
                    &stand_in,
                    actor,
                    index.cast_unsigned(),
                    k.cast_unsigned(),
                    vec,
                );
            },
        )
        .cast_unsigned()
    });
    caller_registers(a, b);
    same("MsBtlGetPos", a, b);
}

/// The actors `MsGetSummonerPos` reads: aeons now and then, the sisters
/// (0xf-0x11) in defense positions (a failed read would add the original's
/// stack garbage), none whose own kind is a summoner's (12-14, which could
/// recurse forever).
fn actors(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, top: u32) {
    for i in 0..0x1f {
        let at = top + i * ACTOR;
        let id = if rng.below(3) == 0 {
            0
        } else {
            0x3000 + rng.below(13)
        };
        poke(a, b, at + 4, &id.to_le_bytes());
        let v: Vec<u8> = (0..8).flat_map(|_| float(rng).to_le_bytes()).collect();
        poke(a, b, at + 0x3b0, &v[..16]);
        poke(a, b, at + 0x514, &v[16..]);
        let sister = (0xf..0x12).contains(&i);
        let place = if sister {
            // group 0, kind 3: past the file's checks to the defense position
            [0, 3, 0]
        } else {
            [
                u8::try_from(rng.below(4)).unwrap(),
                u8::try_from(rng.range(2, 12)).unwrap(),
                u8::try_from(rng.below(4)).unwrap(),
            ]
        };
        poke(a, b, at + 0x4fc, &place);
        poke(a, b, at + 0x511, &[u8::try_from(rng.below(4)).unwrap()]);
        let defense = sister || rng.below(4) == 0;
        poke(a, b, at + 0x6d4, &[if defense { 1 } else { 0xff }]);
    }
}

fn summoner(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, actor: u32, vec: u32) {
    let chr = rng.below(0x12);
    let k = rng.below(5);
    let vals: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
    poke(a, b, vec, &vals);
    let args = [actor, chr, k, vec];
    twin("MsGetSummonerPos", a, b, 0x007a_c2b0, &args, 0, |m| {
        p::ms_get_summoner_pos(m, &stand_in, actor, chr, k, vec);
        0
    });
    caller_registers(a, b);
    same("MsGetSummonerPos", a, b);
}

/// What the current-position chain reads: characters (`+0`, one of
/// `chars`), what an actor rides (`+0x43f`, `+0x441`), its index (`+0xc`),
/// the ground flags and its height. `+0x6dd` (current x and z) only for an
/// actor with a character or a ride: one without would recurse through
/// `+0xc` forever.
fn bodies(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, top: u32, chars: u32) {
    for i in 0..0x1f {
        let at = top + i * ACTOR;
        let chr = if rng.below(2) == 0 {
            0
        } else {
            chars + rng.below(3) * 0x1000
        };
        poke(a, b, at, &chr.to_le_bytes());
        let ride = match rng.below(4) {
            0 => u8::try_from(rng.below(0x1f)).unwrap(),
            1 => 0x30,
            _ => 0xff,
        };
        poke(a, b, at + 0x43f, &[ride]);
        poke(a, b, at + 0x441, &[u8::try_from(rng.below(4)).unwrap()]);
        poke(a, b, at + 0xc, &[u8::try_from(rng.below(0x24)).unwrap()]);
        for o in [0x608, 0x50d, 0xdcc, 0xde7] {
            poke(a, b, at + o, &[u8::from(rng.below(2) == 0)]);
        }
        let ok = chr != 0 || ride != 0xff;
        poke(a, b, at + 0x6dd, &[u8::from(ok && rng.below(3) == 0)]);
        for o in [0x534, 0x4f0] {
            poke(a, b, at + o, &float(rng).to_le_bytes());
        }
    }
}

/// Fake characters, 0x1000 apart, of quiet floats.
fn characters(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, chars: u32) {
    let v: Vec<u8> = (0..3 * 0x400)
        .flat_map(|_| float(rng).to_le_bytes())
        .collect();
    poke(a, b, chars, &v);
}

fn chain(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, top: u32, vec: u32) {
    let vals: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
    poke(a, b, vec, &vals);
    let i = rng.below(0x24);
    let actor = top + rng.below(0x1f) * ACTOR;
    match rng.below(5) {
        0 => twin("MsGetChrBtlPos", a, b, 0x0079_48e0, &[i, vec], ALL, |m| {
            p::ms_get_chr_btl_pos(m, &stand_in, i, vec).cast_unsigned()
        }),
        1 => twin("MsGetChrPos", a, b, 0x0079_56e0, &[i, vec], ALL, |m| {
            p::ms_get_chr_pos(m, &stand_in, i, vec).cast_unsigned()
        }),
        2 => twin("msGetChrPos", a, b, 0x0079_6ea0, &[actor, vec], 0, |m| {
            p::ms_get_chr_pos_of(m, &stand_in, actor, vec);
            0
        }),
        3 => twin(
            "msGetChrPosNoChr",
            a,
            b,
            0x0079_5ac0,
            &[actor, vec],
            0,
            |m| {
                p::ms_get_chr_pos_no_chr(m, &stand_in, actor, vec);
                0
            },
        ),
        _ => {
            // an actor whose positions are always written (a character, a
            // defense position past the file's checks): the original
            // compares stack garbage otherwise
            let i = rng.below(0x1f);
            let at = top + i * ACTOR;
            let chr: u32 = ffx_bridge::Mem::read(b, at);
            if chr == 0 {
                return;
            }
            poke(a, b, at + 0x43f, &[0xff]);
            poke(a, b, at + 0x4fc, &[0, 3, 0]);
            poke(a, b, at + 0x6d4, &[1]);
            let flat = rng.below(2);
            twin(
                "MsCheckBtlPos",
                a,
                b,
                0x007a_9230,
                &[i, actor, flat],
                ALL,
                |m| p::ms_check_btl_pos(m, &stand_in, i, actor, flat),
            );
        }
    }
    caller_registers(a, b);
    same("position chain", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let at = scratch(a, b, 0x800);
    let actor = scratch(a, b, 0xf90);
    let vec = scratch(a, b, 0x10);
    patch_trig(a);
    patch_trig(b);
    // actors of our own for MsGetSummonerPos
    let top = scratch(a, b, 0x1f * 0xf90);
    set(a, b, ffx_battle::actor::bridge::CHR_TOP, top);
    let chars = scratch(a, b, 0x3000);
    for round in 0..3000 {
        if round % 4 == 0 {
            file(a, b, rng, at);
            set(a, b, p::POS_FILE, at);
            fill(a, b, rng, actor, ACTOR);
            actors(a, b, rng, top);
            bodies(a, b, rng, top, chars);
            characters(a, b, rng, chars);
            // an actor in a defense position now and then
            poke(
                a,
                b,
                actor + 0x6d4,
                &[if rng.below(4) == 0 { 1 } else { 0xff }],
            );
            let v: Vec<u8> = (0..4).flat_map(|_| float(rng).to_le_bytes()).collect();
            poke(a, b, actor + 0x3b0, &v);
            for o in [0x4fc, 0x4fd, 0x4fe] {
                poke(a, b, actor + o, &[u8::try_from(rng.below(4)).unwrap()]);
            }
        }
        call(a, b, rng, actor, vec);
        summoner(a, b, rng, actor, vec);
        chain(a, b, rng, top, vec);
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
