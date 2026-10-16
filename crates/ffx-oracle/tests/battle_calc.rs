//! `MsCalcCommand`, its target step and `MsActTargetSearch`
//! (`ffx_battle::stat`) against the original, as in `battle.rs`: the
//! shared battle world with random actors, commands and cue entries, on the
//! image alone and on both snapshots. `TOGetFFXLang` and the achievement
//! unlock (main's) are the same stubs in both; the trigonometric and square
//! root imports the same stand-ins (`battle_stat.rs`).

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::{ACHIEVEMENT, ACTORS, TO_GET_FFX_LANG, World, world};
use battle_common::*;
use ffx_battle::stat::bridge as s;
use ffx_oracle::{Conv, Oracle};

fn patch(o: &mut Oracle) {
    for (at, code) in [
        (0x0094_98c2, [0xd8, 0xc8, 0xc3]),
        (0x0094_98c8, [0xd8, 0xc0, 0xc3]),
        (0x0094_97b8, [0xd9, 0xfa, 0xc3]),
    ] {
        let at = o.rebase(at);
        o.write(at, &code).unwrap();
    }
}

fn stand_in(t: ffx_battle::pos::bridge::Trig, x: u32) -> u32 {
    use ffx_battle::pos::bridge::Trig;
    use ffx_bridge::x87::{add, load, mul, store};
    match t {
        Trig::Cos => store(mul(load(x), load(x))),
        Trig::Sin => store(add(load(x), load(x))),
    }
}

fn lang(m: &mut Oracle) -> u32 {
    let at = m.rebase(TO_GET_FFX_LANG);
    m.call(at, Conv::Cdecl, &[]).unwrap().eax
}

fn achievement(m: &mut Oracle, n: u32) {
    let at = m.rebase(ACHIEVEMENT);
    m.call(at, Conv::Cdecl, &[n]).unwrap();
}

/// What targeting reads beyond the world's shuffle: the cursor and reach
/// bytes, reflect, the action buffers' free marks, spoils (synthetic).
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, spoils: u32) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        for (at, len) in [
            (0x4ff, 1),
            (0x6d3, 2),
            (0xdd9, 1),
            (0x62c, 1),
            (0x60d, 1),
            (0x6c0, 1),
        ] {
            fill(a, b, rng, p + at, len);
        }
        if rng.below(2) == 0 {
            poke(a, b, p + 0x60d, &[0]);
            poke(a, b, p + 0x62c, &[0]);
        }
        for o in [0x776, 0xa4e] {
            let v = if rng.below(3) == 0 { rng.byte() } else { 0xff };
            poke(a, b, p + o, &[v]);
        }
        // groups (MsGetRandomGroupTarget)
        poke(a, b, p + 0x504, &[rng.byte()]);
        if w.synthetic {
            // some formulas divide by the maxima (a fault on 0)
            for at in [0x594, 0x598] {
                if u32::from_le_bytes(b.read(p + at, 4).unwrap().try_into().unwrap()) == 0 {
                    poke(a, b, p + at, &1u32.to_le_bytes());
                }
            }
            let sp = if rng.below(2) == 0 { 0 } else { spoils };
            poke(a, b, p + 0xf88, &sp.to_le_bytes());
        }
    }
    if w.synthetic {
        fill(a, b, rng, spoils, 0x120);
    }
    // a guard now and then (any actor, Tidus often): able, status 2 0x1000
    // or 0x2000, nothing in the way of an automatic command
    for _ in 0..rng.below(3) {
        let i = if rng.below(2) == 0 {
            0
        } else {
            rng.below(ACTORS)
        };
        let p = w.actor(i);
        poke(a, b, p + 0xdc8, &[1]);
        poke(a, b, p + 0xdcc, &[0, 0, 0]);
        poke(a, b, p + 0x606, &[0, 0, 0]);
        poke(a, b, p + 0x617, &[[0x10, 0x20][rng.below(2) as usize]]);
    }
    set(
        a,
        b,
        ffx_battle::save::bridge::STEAL_RULE,
        u8::try_from(rng.below(3)).unwrap(),
    );
}

fn id(rng: &mut Xorshift) -> u16 {
    let v = match rng.below(4) {
        0 => 0x2000 + rng.below(0x40),
        1 => 0xff,
        _ => 0x3000 + rng.below(0x40),
    };
    u16::try_from(v).unwrap()
}

fn selector(rng: &mut Xorshift) -> u32 {
    match rng.below(4) {
        0 => rng.range(-26, 0).cast_unsigned(),
        1 => rng.below(0x24),
        _ => rng.next() & 0x7fff_ffff,
    }
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, cue: u32) {
    let out = w.out;
    // the search
    let chr = rng.below(0x24);
    let sel = selector(rng);
    let kind = [0x5c, 0, 1, 9, 0x16, 0x1c, 0x1e, 0x50, 0xfb, 0x11b, 0x159][rng.below(11) as usize];
    let which = rng.below(0x44);
    let mode = [0, 1, 2, 3, 0x80, 0x81, 0x82][rng.below(7) as usize];
    let resolved = rng.below(2);
    let pick = if rng.below(4) == 0 { 0 } else { out + 0x80 };
    let args = [chr, sel, kind, rng.next(), which, mode, resolved, pick];
    twin("MsActTargetSearch", a, b, 0x007a_cd40, &args, ALL, |m| {
        s::ms_act_target_search(m, &stand_in, chr, sel, kind, which, mode, resolved, pick)
    });
    same("MsActTargetSearch", a, b);
    // command data, equipment names, default targets
    if w.synthetic {
        // equipment of names the fake table has
        let mut slots = Vec::new();
        slots.push((ffx_battle::save::bridge::EQUIPMENT.ptr(b).addr, 200));
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
        let menu = rng.next();
        set(a, b, ffx_battle::save::bridge::MENU, menu);
    }
    let data = (rng.below(0xe) << 12) | [rng.below(0x40), rng.below(0x1000)][rng.below(2) as usize];
    let data = if rng.below(10) == 0 { 0xff } else { data };
    let name = if rng.below(3) == 0 { 0 } else { out + 0x90 };
    twin("MsGetComData", a, b, 0x0079_a4b0, &[data, name], ALL, |m| {
        s::ms_get_com_data(m, data, name)
    });
    same("MsGetComData", a, b);
    twin(
        "MsGetSaveWeapon",
        a,
        b,
        0x007a_bbd0,
        &[data, name],
        ALL,
        |m| ffx_battle::save::bridge::ms_get_save_weapon(m, data, name),
    );
    same("MsGetSaveWeapon", a, b);
    twin(
        "MsGetSaveWeaponName",
        a,
        b,
        0x007a_bdf0,
        &[data],
        ALL,
        |m| ffx_battle::save::bridge::ms_get_save_weapon_name(m, data),
    );
    same("MsGetComData", a, b);
    let bits = rng.next() & 0x7fff_ffff;
    let bits = [bits, bits & rng.next(), 0, 1 << rng.below(0x1f)][rng.below(4) as usize];
    twin(
        "MsGetRandomGroupTarget",
        a,
        b,
        0x0079_9910,
        &[bits],
        ALL,
        |m| s::ms_get_random_group_target(m, bits),
    );
    same("MsGetRandomGroupTarget", a, b);
    let chr = rng.below(0x24);
    let cid = u32::from(id(rng));
    let side = rng.range(-2, 3);
    let args = [chr, cid, side.cast_unsigned(), rng.next()];
    twin("MsGetCommandTarget", a, b, 0x0079_96d0, &args, ALL, |m| {
        s::ms_get_command_target(m, &stand_in, chr, cid, side, args[3])
    });
    same("MsGetCommandTarget", a, b);
    // the target step
    let user = rng.below(0x1f);
    let cmd = w.command(a, b, rng);
    // hit counts the game has: more than 16 hits overrun a target's
    // results block (0x2d8 bytes) into the actor's spoils pointer
    poke(a, b, cmd + 0x2b, &[u8::try_from(rng.below(5)).unwrap()]);
    let event = u32::from(id(rng));
    poke(a, b, out + 0x40, &(rng.next() & 0x7fff_ffff).to_le_bytes());
    poke(a, b, out + 0x44, &rng.below(100).to_le_bytes());
    let args = [user, out, out + 0x20, cmd, event, out + 0x40, out + 0x44];
    twin("MsCalcCommandTarget", a, b, 0x0078_ba70, &args, ALL, |m| {
        s::ms_calc_command_target(
            m,
            &stand_in,
            user,
            out,
            out + 0x20,
            cmd,
            event,
            out + 0x40,
            out + 0x44,
        )
    });
    same("MsCalcCommandTarget", a, b);
    // a whole command
    let n = rng.below(3);
    let mut c = vec![0u8; 0x40];
    c[0] = u8::try_from(rng.below(0x1f)).unwrap();
    c[3] = u8::try_from(n).unwrap();
    for k in 0..2usize {
        let e = k * 16;
        c[e + 8..e + 10].copy_from_slice(&id(rng).to_le_bytes());
        c[e + 10..e + 12].copy_from_slice(&id(rng).to_le_bytes());
        // up to three targets: hits beyond 16 on one target (guards and
        // reflection gather them) overrun its results block
        let bits = match rng.below(3) {
            0 => 1 << rng.below(0x1f),
            1 => 0,
            _ => (1 << rng.below(0x1f)) | (1 << rng.below(0x1f)) | (1 << rng.below(0x1f)),
        };
        c[e + 0x10..e + 0x14].copy_from_slice(&u32::to_le_bytes(bits));
    }
    poke(a, b, cue, &c);
    let k = rng.below(3);
    let cue_at = if rng.below(10) == 0 { 0 } else { cue };
    twin("MsCalcCommand", a, b, 0x0078_92e0, &[cue_at, k], ALL, |m| {
        s::ms_calc_command(m, &stand_in, cue_at, k, lang, achievement)
    });
    same("MsCalcCommand", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    patch(a);
    patch(b);
    let w = world(a, b, rng, synthetic);
    for o in [&mut *a, &mut *b] {
        let at = o.rebase(ACHIEVEMENT);
        o.stub_return(at, 0).unwrap();
        if w.synthetic {
            let at = o.rebase(TO_GET_FFX_LANG);
            o.stub_return(at, 1).unwrap();
        }
    }
    if w.synthetic {
        // the fake kernel records' hit counts as the game's (0-4)
        for g in [ffx_kernel::bridge::COMMAND, ffx_kernel::bridge::ITEM] {
            let t = g.get(b);
            for i in 0..0x40 {
                poke(
                    a,
                    b,
                    t + 0x20 + i * 0x60 + 0x2b,
                    &[u8::try_from(rng.below(5)).unwrap()],
                );
            }
        }
    }
    // kernel tables where there are none
    for (g, stride) in [
        (ffx_battle::menu::bridge::WEAPON_NAMES, 0x60),
        (ffx_battle::minor::bridge::IMPORTANT_ROM, 0x10),
        (ffx_kernel::bridge::A_ABILITY, 0x40),
    ] {
        if w.synthetic || !readable(a, g.get(b), 0x20) {
            let t = battle_common::world::fake_records(a, b, rng, 0x40, stride);
            set(a, b, g, t);
        }
    }
    let spoils = scratch(a, b, 0x120);
    let cue = scratch(a, b, 0x40);
    for round in 0..1500 {
        w.shuffle(a, b, rng, round);
        dress(a, b, rng, &w, spoils);
        calls(a, b, rng, &w, cue);
    }
}

#[test]
fn commands_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xca1c_0001), true);
}

#[test]
fn commands_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xca1c_0002), false);
}
