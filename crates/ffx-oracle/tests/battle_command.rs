//! The command lookups and costs (`ffx_battle::command`) against the
//! original, as in `battle_damage.rs`: synthetic actors and kernel tables
//! on the image alone, the snapshots' own where they have them.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::*;
use battle_common::*;
use ffx_battle::command::bridge as c;
use ffx_oracle::Oracle;

/// A command ID: of each table, out of range, or a stop ID.
fn command_id(rng: &mut Xorshift) -> u32 {
    match rng.below(8) {
        0 => [0x311e, 0x3024, 0x302a, 0x3028, 0x3029, 0x305f, 0xff][rng.below(7) as usize],
        1 => rng.next(),
        2 => 0x5000 + rng.below(0x40),
        _ => [0x2000, 0x3000, 0x4000, 0x6000][rng.below(4) as usize] + rng.below(0x48),
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let (out, cue) = (w.out, w.results);
    for round in 0..2000 {
        w.shuffle(a, b, rng, round);
        let i = rng.below(ACTORS);
        let chr = if rng.below(8) == 0 {
            rng.below(0x100)
        } else {
            i
        };
        let user = w.actor(i);
        for at in [0x6de, 0x6ce] {
            let v = u8::from(rng.below(3) == 0);
            poke(a, b, user + at, &[v]);
        }
        for g in [ffx_battle::damage::bridge::DEBUG_NO_COST, c::DEBUG_FREE_MP] {
            let v = u8::from(rng.below(5) == 0);
            set(a, b, g, v);
        }
        // MsGetCommand: a list, a single ID, nothing
        let ids = [command_id(rng), command_id(rng)];
        let list = out + 0x10;
        poke(
            a,
            b,
            list,
            &[
                ids[0].to_le_bytes()[..2].to_vec(),
                ids[1].to_le_bytes()[..2].to_vec(),
            ]
            .concat(),
        );
        poke(a, b, out + 0x20, &command_id(rng).to_le_bytes());
        let stop = if rng.below(2) == 0 {
            ids[0]
        } else {
            command_id(rng)
        };
        let args = [
            [chr, 0, stop, list, out + 0x20],
            [chr, 0, stop, list, 0],
            [chr, 0, stop, 0, out + 0x20],
            [chr, 0, stop, 0, 0],
        ][rng.below(4) as usize];
        let rec = c::ms_get_command(b, args[2], args[3], args[4]);
        let r = a
            .call(a.rebase(0x0078_ce50), ffx_oracle::Conv::Cdecl, &args)
            .unwrap();
        assert!(r.st0.is_none(), "MsGetCommand {args:x?}: a float in ST0");
        let ra = r.eax;
        assert_eq!(ra, rec, "MsGetCommand {args:x?}");
        let cmd = if rec != 0 && rng.below(2) == 0 {
            rec
        } else {
            w.command(a, b, rng)
        };
        twin("MsGetCommandMP", a, b, 0x0078_cf70, &[chr, cmd], ALL, |m| {
            c::ms_get_command_mp(m, chr, cmd).cast_unsigned()
        });
        let p = if rng.below(8) == 0 { 0 } else { cmd };
        let extra = rng.range(-5, 50).cast_unsigned();
        twin(
            "MsCheckUseCommand",
            a,
            b,
            0x0078_c690,
            &[chr, p, extra],
            ALL,
            |m| c::ms_check_use_command(m, chr, p, extra.cast_signed()).cast_unsigned(),
        );
        // a cue entry: actor, flags, n commands of 16 bytes
        fill(a, b, rng, cue, 0x60);
        let n = rng.below(4).to_le_bytes()[0];
        poke(
            a,
            b,
            cue,
            &[i.to_le_bytes()[0], 0, u8::from(rng.below(2) == 0), n],
        );
        for k in 0..3 {
            let pair = [command_id(rng), command_id(rng)];
            let at = cue + 8 + 16 * k;
            poke(
                a,
                b,
                at,
                &[
                    pair[0].to_le_bytes()[..2].to_vec(),
                    pair[1].to_le_bytes()[..2].to_vec(),
                ]
                .concat(),
            );
        }
        let k = rng.below(3);
        let p = if rng.below(10) == 0 { 0 } else { cue };
        twin("MsCalcMagic", a, b, 0x0078_ab20, &[p, k], ALL, |m| {
            c::ms_calc_magic(m, p, k).cast_unsigned()
        });
        twin("MsCalcCostCTB", a, b, 0x0078_95b0, &[p], ALL, |m| {
            c::ms_calc_cost_ctb(m, p)
        });
        // CTB effects: 31 CTB values, haste and slow bytes
        let (ctb, haste, slow) = (out + 0x30, out + 0xb0, out + 0xd0);
        let values: Vec<u8> = (0..31)
            .flat_map(|_| [rng.range(-2, 300), -1, 0][rng.below(3) as usize].to_le_bytes())
            .collect();
        poke(a, b, ctb, &values);
        fill(a, b, rng, haste, 31);
        fill(a, b, rng, slow, 31);
        let rank = rng.range(-1, 5).cast_unsigned();
        twin(
            "MsCalcDamageCTB",
            a,
            b,
            0x0078_98d0,
            &[cue, ctb, rank, haste, slow],
            0,
            |m| {
                c::ms_calc_damage_ctb(m, cue, ctb, rank.cast_signed(), haste, slow);
                0
            },
        );
        same("commands", a, b);
    }
}

#[test]
fn commands_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x2468_ace0), true);
}

#[test]
fn commands_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x1357_2468), false);
}
