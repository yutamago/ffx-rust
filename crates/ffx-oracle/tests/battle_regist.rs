//! The actors' character registration (`ffx_battle::regist`, battle round
//! 35: `MsBtlChrRegist`, `MsBtlReadPlayerModelProcess`,
//! `MsBtlReadMonsterProcess`, `MsBtlReadPlayerProcess`,
//! `MsBtlWeaponRegist`) against the original on the battle snapshot's own
//! actors and characters, each round from a fresh copy of it (a new
//! character takes heap the snapshot has little of). Script motions,
//! actions, status effects, weapon attachment, the vfx calls, the render
//! instances and `dbgPrintf` are logging stand-ins in both oracles; the
//! rest runs as the originals (the battle and chara ones the ports call as
//! their Rust). The C runtime's trigonometry is the shared stand-ins. On
//! the image alone only the paths without a character run.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use std::path::PathBuf;

use battle_common::crt::{patch, stand_in};
use battle_common::*;
use ffx_battle::regist::bridge as rg;
use ffx_oracle::{Oracle, Pages};

fn call(m: &mut Oracle, f: u32, a: &[u32]) -> u32 {
    m.call_cdecl(f, a).unwrap()
}

/// The stand-ins: (function, arguments logged).
const STANDINS: [(u32, u8); 15] = [
    (0x0062_f500, 1), // dbgPrintf (callers pass it more or fewer words)
    (0x0063_d370, 4), // graphicCharacterRegistToInstantiate
    (0x0063_a030, 2), // graphicCharacterInstanceDestroy
    (0x0063_a160, 1), // graphicCharacterInstanceDestroyClothVertexBuffers
    (0x0090_ae50, 4), // yiAssert
    (0x007f_c290, 0), // opus_change_chrhd_ffx_to_battle
    (0x007f_b290, 1), // op_et_bindeff
    (0x007f_b360, 1), // op_et_bindeff2
    (0x007a_ceb0, 6), // MsActionRequest
    (0x007a_b360, 7), // MsSetMotion
    (0x0079_eff0, 1), // MsStatusEffectCheck
    (0x0079_3330, 2), // MsAttachWeapon
    (0x00a4_4b30, 0), // JobScheduleWait_Ch_Cloth_PerCharacter (threads)
    (0x0068_7010, 1), // user_free_thunk (its heap keeps caller state)
    (0x0084_0e60, 1), // Sg_CacheDisposeChr (leaves stack-dependent bytes)
];

/// The stand-ins in both oracles: each logs its count and arguments in a
/// ring at its record (`ffx_oracle` code memory, the same address in
/// both) and returns 0.
fn standins(a: &mut Oracle, b: &mut Oracle) {
    for (f, n) in STANDINS {
        let rec = scratch(a, b, 8 + 0x40 * usize::from(n.max(1)));
        let mut code = vec![0x8b, 0x0d];
        code.extend(rec.to_le_bytes());
        code.extend([0xff, 0x05]);
        code.extend(rec.to_le_bytes());
        code.extend([0x83, 0xe1, 0x0f, 0x6b, 0xc9, 4 * n, 0x81, 0xc1]);
        code.extend((rec + 8).to_le_bytes());
        for k in 0..n {
            code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
        }
        code.extend([0x31, 0xc0, 0xc3]); // xor eax, eax; ret
        let at = a.alloc_code(&code).unwrap();
        assert_eq!(b.alloc_code(&code).unwrap(), at);
        for o in [&mut *a, &mut *b] {
            let entry = o.rebase(f);
            let rel = at.wrapping_sub(entry + 5);
            o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
                .unwrap();
        }
    }
}

/// The snapshot's pages, read once.
fn pages() -> Option<(PathBuf, std::sync::Arc<Pages>)> {
    let exe = exe()?;
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    Some((exe, Pages::read(PathBuf::from(snap)).unwrap()))
}

/// A fresh pair from the snapshot, stand-ins and trigonometry in.
fn fresh(exe: &PathBuf, pages: &std::sync::Arc<Pages>) -> (Oracle, Oracle) {
    let mut a = Oracle::from_snapshot(exe, pages).unwrap();
    let mut b = Oracle::from_snapshot(exe, pages).unwrap();
    patch(&mut a);
    patch(&mut b);
    standins(&mut a, &mut b);
    (a, b)
}

fn word(o: &Oracle, at: u32) -> u32 {
    ffx_bridge::Mem::read(o, at)
}

fn byte(o: &Oracle, at: u32) -> u8 {
    ffx_bridge::Mem::read(o, at)
}

/// The actors with a live character (`+0`, active).
fn live_actors(b: &Oracle) -> Vec<u32> {
    // no battle on this snapshot: no actors
    if ffx_battle::actor::bridge::CHR_TOP.get(b) == 0 {
        return Vec::new();
    }
    (0..ffx_battle::actor::ACTORS)
        .filter(|&i| {
            let actor = ffx_battle::actor::bridge::ms_get_chr(b, i);
            let chr = word(b, actor);
            chr != 0 && chr != 0xcdcd_cdcd && byte(b, chr + 2) != 0
        })
        .collect()
}

/// Random flags among the bytes the steps test.
fn flags(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, actor: u32) {
    for o in [0x506, 0x44c, 0x44d, 0xdce, 0x1e, 0x24, 0xdd7] {
        let v = u8::from(rng.below(3) == 0);
        poke(a, b, actor + o, &[v]);
    }
    poke(a, b, actor + 0x43d, &[rng.byte() & 0xf]);
    let kind = [0x14, 0x15, 0x16, 0x17, 0x18, 0, rng.byte()][rng.below(7) as usize];
    poke(a, b, actor + 0x5c2, &[kind]);
    if rng.below(3) == 0 {
        poke(a, b, actor + 0x4c, &[0; 4]);
    }
    let started = u8::from(rng.below(4) == 0);
    set(a, b, ffx_battle::read::bridge::MAP_STARTED, started);
    scramble(a, b, rng, actor);
}

/// A plain float (no NaN): the snapshot's fields hold the values the steps
/// store, so the stores are checked against random ones.
fn small_float(rng: &mut Xorshift) -> [u8; 4] {
    let v = f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 37.0;
    v.to_bits().to_le_bytes()
}

/// Random values in the record's (actor's or weapon actor's) fields the
/// steps write or read as data, and in its character's. Not the binding
/// links `+0x12`..`+0x15`: random ones make the visibility walk recurse.
fn scramble(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, actor: u32) {
    use ffx_chara::layout as l;
    for o in [
        0x40, 0x3a0, 0x3a4, 0x3a8, 0x3ac, 0x4dc, 0x4e4, 0x4e8, 0x514, 0x518, 0x51c,
    ] {
        let v = small_float(rng);
        poke(a, b, actor + o, &v);
    }
    for o in [
        0x1f, 0x38, 0x3f0, 0x41c, 0x423, 0x433, 0x44f, 0x4b5, 0x508, 0x5c3, 0xdcd, 0xdfc,
    ] {
        let v = rng.byte();
        poke(a, b, actor + o, &[v]);
    }
    poke(a, b, actor + 0x511, &[u8::try_from(rng.below(3)).unwrap()]);
    // the callees' fields: the packet request, the effect motion, the
    // height mode and its floats, the motion table and its index
    for (o, v) in [
        (0x3a, u8::from(rng.below(3) == 0)),
        (0x3f3, u8::from(rng.below(3) == 0)),
        (0x50c, u8::try_from(rng.below(3)).unwrap()),
        (0x4bb, u8::try_from(rng.below(8)).unwrap()),
        (0xf5e, rng.byte()),
        (0xdf2, rng.byte()),
    ] {
        poke(a, b, actor + o, &[v]);
    }
    for o in [0x534, 0x538] {
        let v = small_float(rng);
        poke(a, b, actor + o, &v);
    }
    poke(a, b, actor + 0x3f2, &rng.next().to_le_bytes()[..2]);
    poke(a, b, actor + 0x4f8, &rng.next().to_le_bytes());
    poke(
        a,
        b,
        actor + 0x400,
        &u16::try_from(rng.below(0x400)).unwrap().to_le_bytes(),
    );
    poke(a, b, actor + 0x6fc, &rng.next().to_le_bytes());
    poke(a, b, actor + 0xe2c, &rng.below(4).to_le_bytes());
    let slots: Vec<u8> = (0..3).flat_map(|_| rng.below(50).to_le_bytes()).collect();
    poke(a, b, rg::MONSTER_SLOTS.ptr(b).addr, &slots);
    let chr = word(b, actor);
    if chr == 0 || chr == 0xcdcd_cdcd || a.read(chr, 0x880).is_err() {
        return;
    }
    for o in [
        l::SCALE,
        l::SCALE + 4,
        l::SCALE + 8,
        l::SP,
        l::DIR,
        l::BIND_SCALE,
        l::THICKNESS,
        l::WEIGHT,
    ] {
        let v = small_float(rng);
        poke(a, b, chr + o, &v);
    }
    for k in 0..6 {
        let v = small_float(rng);
        poke(a, b, chr + l::FADE_ALPHA + 4 * k, &v);
    }
    poke(a, b, chr + l::MOTION_SPEED, &rng.next().to_le_bytes());
    poke(a, b, chr + l::BATTLE_CHR_NUM, &rng.next().to_le_bytes());
    poke(a, b, chr + l::HIDE, &[rng.byte()]);
    poke(a, b, chr + l::EFFECT_LEVEL, &rng.below(4).to_le_bytes());
}

#[test]
fn regist_matches_the_original_on_the_image() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    patch(&mut a);
    patch(&mut b);
    standins(&mut a, &mut b);
    let mut rng = Xorshift(0x0035_39b0);
    let w = battle_common::world::world(&mut a, &mut b, &mut rng, true);
    let mut c = call;
    let v = b.stack_range().start + 0x400;
    // monsters without a model or not shown: 0xffff back
    for (k, i) in [0x14u32, 0x15, 0x1d].into_iter().enumerate() {
        let actor = w.actor(i);
        if k == 2 {
            poke(&mut a, &mut b, actor + 0x24, &[0]);
        } else {
            poke(&mut a, &mut b, actor + 0xe, &0xffffu16.to_le_bytes());
        }
        twin(
            "MsBtlReadMonsterProcess",
            &mut a,
            &mut b,
            0x0078_39b0,
            &[i],
            ALL,
            |m| rg::ms_btl_read_monster_process(m, &stand_in, &mut c, i, v),
        );
        same("MsBtlReadMonsterProcess", &mut a, &mut b);
    }
}

#[test]
fn regist_matches_the_original_on_a_snapshot() {
    let Some((exe, pages)) = pages() else {
        return;
    };
    let (a0, _) = fresh(&exe, &pages);
    let live = live_actors(&a0);
    drop(a0);
    if live.is_empty() {
        eprintln!("no live actors on this snapshot: skipped");
        return;
    }
    let mut rng = Xorshift(0x0035_3670);
    let mut c = call;
    let mut reach = [0u32; 8];
    for round in 0..150u32 {
        let (mut a, mut b) = fresh(&exe, &pages);
        let v = b.stack_range().start + 0x400;
        let i = live[rng.below(u32::try_from(live.len()).unwrap()) as usize];
        let actor = ffx_battle::actor::bridge::ms_get_chr(&b, i);
        flags(&mut a, &mut b, &mut rng, actor);
        match round % 5 {
            0 => {
                // a new character at times
                let key = word(&b, actor + 4);
                let created = rng.below(3) == 0;
                // callers pass a byte: the rest of the word is ignored
                let i = i | (rng.below(2) << 8);
                if created {
                    poke(&mut a, &mut b, actor, &[0; 4]);
                }
                reach[usize::from(created)] += 1;
                twin(
                    "MsBtlChrRegist",
                    &mut a,
                    &mut b,
                    0x0079_3670,
                    &[i, key],
                    ALL,
                    |m| rg::ms_btl_chr_regist(m, &stand_in, &mut c, i, key, v),
                );
            }
            1 => {
                if rng.below(3) == 0 {
                    poke(&mut a, &mut b, actor, &[0; 4]);
                }
                twin(
                    "MsBtlReadPlayerModelProcess",
                    &mut a,
                    &mut b,
                    0x0078_3b90,
                    &[i],
                    0,
                    |m| {
                        rg::ms_btl_read_player_model_process(m, &stand_in, &mut c, i, v);
                        0
                    },
                );
            }
            2 => {
                let model = [0, 1, 1, 0xffff][rng.below(4) as usize];
                poke(&mut a, &mut b, actor + 0x24, &[u8::from(rng.below(4) != 0)]);
                poke(
                    &mut a,
                    &mut b,
                    actor + 0xdd7,
                    &[u8::from(rng.below(3) != 0)],
                );
                poke(
                    &mut a,
                    &mut b,
                    actor + 0xe,
                    &u16::try_from(model).unwrap().to_le_bytes(),
                );
                reach[2] += u32::from(model != 0xffff && byte(&b, actor + 0x24) != 0);
                twin(
                    "MsBtlReadMonsterProcess",
                    &mut a,
                    &mut b,
                    0x0078_39b0,
                    &[i],
                    0xffff,
                    |m| rg::ms_btl_read_monster_process(m, &stand_in, &mut c, i, v),
                );
            }
            3 => {
                let mode = [0, 1, 2, 6, 8, 10][rng.below(6) as usize];
                if rng.below(3) == 0 {
                    poke(&mut a, &mut b, actor, &[0; 4]);
                }
                reach[3] += u32::from(mode == 6 || mode == 8);
                twin(
                    "MsBtlReadPlayerProcess",
                    &mut a,
                    &mut b,
                    0x0078_3bc0,
                    &[i, mode],
                    0,
                    |m| {
                        rg::ms_btl_read_player_process(m, &stand_in, &mut c, i, mode, v);
                        0
                    },
                );
            }
            _ => {
                let (mode, mut k) = (rng.below(2), rng.below(2));
                // half the time an actor and weapon with a special record
                let specials: Vec<(u32, u32)> = (0..ffx_battle::actor::ACTORS)
                    .flat_map(|j| [(j, 0), (j, 1)])
                    .filter(|&(j, kk)| {
                        ffx_battle::actor::bridge::ms_get_weapon_special(&b, j, kk) != 0
                    })
                    .collect();
                let mut i = i;
                if !specials.is_empty() && rng.below(2) == 0 {
                    (i, k) = specials[rng.below(u32::try_from(specials.len()).unwrap()) as usize];
                    let owner = ffx_battle::actor::bridge::ms_get_chr(&b, i);
                    flags(&mut a, &mut b, &mut rng, owner);
                }
                let actor = ffx_battle::actor::bridge::ms_get_chr(&b, i);
                let w = ffx_battle::actor::bridge::ms_get_weapon(&b, i, k.cast_signed());
                // the original disposes the save model's block (it
                // passes the new model), and a weapon without a loaded
                // model needs a file read (Ch_BlkGetHandle -> Sg_PcRead) the
                // oracle cannot do: another live character's block
                let other = live[rng.below(u32::try_from(live.len()).unwrap()) as usize];
                let chr = word(&b, ffx_battle::actor::bridge::ms_get_chr(&b, other));
                let blk = word(&b, chr + 0x1c4);
                for o in [&mut a, &mut b] {
                    let at = o.rebase(0x0082_5a40);
                    o.stub_return(at, blk).unwrap();
                }
                let sub = rng.below(4);
                match sub {
                    // a changed model: the old character disposed
                    0 => {
                        poke(&mut a, &mut b, w + 4, &rng.next().to_le_bytes());
                        reach[4] += u32::from(word(&b, w) != 0);
                    }
                    // no character yet: a new one for the model
                    1 => poke(&mut a, &mut b, w, &[0; 4]),
                    _ => {}
                }
                poke(
                    &mut a,
                    &mut b,
                    w + 8,
                    &[u8::from(rng.below(2) == 0), 0, 0, 0],
                );
                let special = ffx_battle::actor::bridge::ms_get_weapon_special(&b, i, k);
                if special != 0 {
                    let kind =
                        [0x1c, 0x1d, 0x1e, 0x1e, byte(&b, special + 0xc)][rng.below(5) as usize];
                    poke(&mut a, &mut b, special + 0xc, &[kind]);
                    scramble(&mut a, &mut b, &mut rng, special);
                }
                scramble(&mut a, &mut b, &mut rng, w);
                poke(&mut a, &mut b, actor + 0x1d, &[u8::from(rng.below(2) == 0)]);
                reach[5] += u32::from(special != 0);
                twin(
                    "MsBtlWeaponRegist",
                    &mut a,
                    &mut b,
                    0x0079_38e0,
                    &[i, mode, k],
                    ALL,
                    |m| rg::ms_btl_weapon_regist(m, &stand_in, &mut c, [i, mode, k], v),
                );
            }
        }
        same(&format!("regist round {round} actor {i}"), &mut a, &mut b);
        reach[7] += 1;
    }
    eprintln!("regist reach {reach:?} on actors {live:?}");
}
