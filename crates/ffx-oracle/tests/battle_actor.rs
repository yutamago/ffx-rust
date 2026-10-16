//! The actor and battle getters (`ffx_battle::actor`, `ffx_battle::state`)
//! against the original, as in `battle.rs`, on the shared battle world.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::world::*;
use battle_common::*;
use ffx_battle::{actor::bridge as act, state::bridge as st};
use ffx_oracle::{Conv, Oracle};

/// `TOBtlCloseSignWindow` (system), which the message cue calls.
const TO_BTL_CLOSE_SIGN_WINDOW: u32 = 0x0089_0ee0;

/// A getter by actor index: name, `ffx.sym` address, the Rust.
type ByIndex = (&'static str, u32, fn(&Oracle, u32) -> u32);

const BY_INDEX: [ByIndex; 21] = [
    ("MsGetRamChrHP", 0x0079_add0, |m, i| {
        act::ms_get_ram_chr_hp(m, i)
    }),
    ("MsGetRamChrHP_0079adf0", 0x0079_adf0, |m, i| {
        act::ms_get_ram_chr_hp_max(m, i)
    }),
    ("MsGetRamChrMP", 0x0079_ae50, |m, i| {
        act::ms_get_ram_chr_mp(m, i)
    }),
    ("MsGetRamChrMPmax", 0x0079_ae70, |m, i| {
        act::ms_get_ram_chr_mp_max(m, i)
    }),
    ("MsGetRamChrMag", 0x0079_ae90, |m, i| {
        act::ms_get_ram_chr_mag(m, i)
    }),
    ("MsGetRamChrName", 0x0079_af10, |m, i| {
        act::ms_get_ram_chr_name(m, i)
    }),
    ("MsGetRamChrIconNumber", 0x0079_ae10, |m, i| {
        act::ms_get_ram_chr_icon_number(m, i)
    }),
    ("MsGetRamChrIconType", 0x0079_ae30, |m, i| {
        act::ms_get_ram_chr_icon_type(m, i)
    }),
    ("MsGetRamChrMegas", 0x0079_aed0, |m, i| {
        act::ms_get_ram_chr_megas(m, i)
    }),
    ("MsGetRamChrDisableZan", 0x0079_ad90, |m, i| {
        act::ms_get_ram_chr_disable_zan(m, i)
    }),
    ("MsGetRamChrCTBmax", 0x0079_ad10, |m, i| {
        act::ms_get_ram_chr_ctb_max(m, i)
    }),
    ("MsGetChrStatDeath", 0x0079_5780, |m, i| {
        act::ms_get_chr_stat_death(m, i)
    }),
    ("MsGetChrStatStone", 0x0079_5800, |m, i| {
        act::ms_get_chr_stat_stone(m, i)
    }),
    ("MsGetChrStatWeak", 0x0079_5840, |m, i| {
        act::ms_get_chr_stat_weak(m, i)
    }),
    ("MsGetChrStatZombie", 0x0079_5880, |m, i| {
        act::ms_get_chr_stat_zombie(m, i)
    }),
    ("MsGetChrStatPoison", 0x0079_57c0, |m, i| {
        act::ms_get_chr_stat_poison(m, i)
    }),
    ("MsGetChrLimitGauge", 0x0079_5550, |m, i| {
        act::ms_get_chr_limit_gauge(m, i)
    }),
    ("MsGetChrLimitGaugeMax", 0x0079_5590, |m, i| {
        act::ms_get_chr_limit_gauge_max(m, i)
    }),
    ("MsGetChrHandle", 0x0079_5080, |m, i| {
        act::ms_get_chr_handle(m, i)
    }),
    ("MsGetChrGroup", 0x0079_5040, |m, i| {
        act::ms_get_chr_group(m, i)
    }),
    ("MsGetChrCursorFlag", 0x0079_4ca0, |m, i| {
        act::ms_get_chr_cursor_flag(m, i)
    }),
];

/// An `f32` an x87 register holds (from `fld m32`), as bits.
fn f80_bits(r: [u8; 10]) -> u32 {
    let mant = u64::from_le_bytes(r[..8].try_into().unwrap());
    let se = u16::from_le_bytes([r[8], r[9]]);
    let sign = u32::from(se >> 15) << 31;
    let exp = i32::from(se & 0x7fff);
    if exp == 0x7fff {
        return sign | 0x7f80_0000 | u32::try_from((mant >> 40) & 0x7f_ffff).unwrap();
    }
    if mant == 0 {
        return sign;
    }
    // exact: 24 significant bits
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let f = ((mant as f64) * 2f64.powi(exp - 16383 - 63)) as f32;
    sign | f.to_bits()
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let records = scratch(a, b, 14 * 0x100);
    let group = scratch(a, b, 4);
    for round in 0..300 {
        w.shuffle(a, b, rng, round);
        if w.synthetic || round > 0 {
            for i in 0..ACTORS {
                let p = w.actor(i);
                for (at, len) in [(0, 4), (0x500, 8), (0xdc8, 0x20), (0x6c0, 0x40)] {
                    fill(a, b, rng, p + at, len);
                }
            }
            fill(a, b, rng, records, 14 * 0x100);
            fill(a, b, rng, group, 4);
            set(a, b, st::FIELD_RECORDS, records);
            set(a, b, st::SCENE_GROUP, group);
            set(
                a,
                b,
                st::FIELD_INDEX,
                i16::try_from(rng.below(0x100)).unwrap(),
            );
            for g in [st::STATUS, st::EXE] {
                let v = [0, 0, 1, -1][rng.below(4) as usize];
                set(a, b, g, v);
            }
        }
        for _ in 0..8 {
            let i = match rng.below(4) {
                0 => rng.next(),
                1 => rng.below(0x100),
                _ => rng.below(ACTORS),
            };
            for (name, sym, rust) in BY_INDEX {
                twin(name, a, b, sym, &[i], ALL, |m| rust(m, i));
            }
            twin("MsGetChrCommand", a, b, 0x007b_09b0, &[i], ALL, |m| {
                act::ms_get_chr_command(m, i)
            });
            twin("MsGetChrCommand2", a, b, 0x007b_09e0, &[i], ALL, |m| {
                act::ms_get_chr_command2(m, i)
            });
        }
        twin("MsBattleCheck", a, b, 0x0078_0c90, &[], ALL, |m| {
            st::ms_battle_check(m)
        });
        twin("MsBattleChrStart", a, b, 0x0078_0cb0, &[], ALL, |m| {
            st::ms_battle_chr_start(m)
        });
        twin("MsBattleGetNum", a, b, 0x0078_16b0, &[], ALL, |m| {
            st::ms_battle_get_num(m)
        });
        // geometry, positions, weapons: floats without signalling NaNs
        for i in 0..ACTORS {
            let p = w.actor(i);
            for at in (0x39c..0x3b0)
                .step_by(4)
                .chain([0x4e4])
                .chain((0x514..0x534).step_by(4))
            {
                let f = quiet(rng.next());
                poke(a, b, p + at, &f.to_le_bytes());
            }
            fill(a, b, rng, p + 0x43f, 2);
            fill(a, b, rng, p + 0x4ff, 1);
            fill(a, b, rng, p + 0x511, 1);
            poke(a, b, p + 0x511, &[rng.below(8).to_le_bytes()[0]]);
            fill(a, b, rng, p + 0xddd, 3);
            fill(a, b, rng, p + 0xf34, 8);
            fill(a, b, rng, p + 0xf54, 8);
        }
        let weapons = ffx_battle::actor::bridge::WEAPON_TOP.get(b);
        if w.synthetic {
            fill(a, b, rng, weapons, 2 * ACTORS * 0x390);
        }
        let out = w.out;
        for _ in 0..8 {
            let i = match rng.below(3) {
                0 => rng.next(),
                _ => rng.below(ACTORS + 2),
            };
            let fa = a
                .call(a.rebase(0x0079_5a20), Conv::Cdecl, &[i])
                .unwrap()
                .st0_raw
                .unwrap();
            assert_eq!(
                f80_bits(fa),
                act::ms_get_chr_width(b, i).to_bits(),
                "MsGetChrWidth {i:#x}"
            );
            let fa = a
                .call(a.rebase(0x0079_48a0), Conv::Cdecl, &[i])
                .unwrap()
                .st0_raw
                .unwrap();
            assert_eq!(
                f80_bits(fa),
                act::ms_get_chr_btl_dir(b, i).to_bits(),
                "MsGetChrBtlDir {i:#x}"
            );
            let p = if rng.below(4) == 0 { 0 } else { out };
            twin("MsGetChrScale", a, b, 0x0079_5720, &[i, p], ALL, |m| {
                act::ms_get_chr_scale(m, i, p).cast_unsigned()
            });
            twin("MsGetChrDist", a, b, 0x0079_4e00, &[i], ALL, |m| {
                act::ms_get_chr_dist(m, i)
            });
            let (x, y) = (
                [0, out + 0x10][rng.below(2) as usize],
                [0, out + 0x14][rng.below(2) as usize],
            );
            twin(
                "MsGetChrGroundPos",
                a,
                b,
                0x0079_4ff0,
                &[i, x, y],
                ALL,
                |m| act::ms_get_chr_ground_pos(m, i, x, y),
            );
            twin(
                "MsGetChrTargetPos",
                a,
                b,
                0x0079_5920,
                &[i, x, y],
                ALL,
                |m| act::ms_get_chr_target_pos(m, i, x, y),
            );
            let e = [0xff, rng.below(4), rng.next()][rng.below(3) as usize];
            twin("MsGetChrEventFlag", a, b, 0x0079_4f20, &[i, e], ALL, |m| {
                act::ms_get_chr_event_flag(m, i, e)
            });
            twin("MsGetChrMotionSignal", a, b, 0x0079_5660, &[i], ALL, |m| {
                act::ms_get_chr_motion_signal(m, i)
            });
            twin("MsGetChrModelID", a, b, 0x0079_5610, &[i], ALL, |m| {
                act::ms_get_chr_model_id(m, i)
            });
            twin("MsGetChrMotionSpeed", a, b, 0x0079_56a0, &[i], ALL, |m| {
                act::ms_get_chr_motion_speed(m, i)
            });
            let k = rng.range(-1, 3).cast_unsigned();
            twin("MsGetWeaponHandle", a, b, 0x0079_5b40, &[i, k], ALL, |m| {
                act::ms_get_weapon_handle(m, i, k.cast_signed())
            });
            let id = [4, 5, 14, rng.below(20)][rng.below(4) as usize];
            let k = u32::from(rng.below(4) == 0);
            twin(
                "MsGetWeaponSpecial",
                a,
                b,
                0x0079_5bb0,
                &[id, k],
                ALL,
                |m| act::ms_get_weapon_special(m, id, k),
            );
            let cam = rng.below(4) | (rng.next() & 0xffff_ff00);
            twin("MsCameraGetID", a, b, 0x007b_d300, &[cam], ALL, |m| {
                ffx_battle::camera::bridge::ms_camera_get_id(m, cam)
            });
        }
        // actor sets
        for g in [act::COMMAND_ACTOR, act::TURN_ACTOR] {
            let v = [0xff, rng.below(0x20).to_le_bytes()[0], rng.byte()][rng.below(3) as usize];
            set(a, b, g, v);
        }
        set(a, b, act::CHR_SET, rng.next());
        fill(
            a,
            b,
            rng,
            ffx_battle::save::bridge::BATTLE_PARTY.ptr(b).addr,
            8,
        );
        if rng.below(2) == 0 {
            // a command cue entry for the command actor
            let cue = ffx_battle::state::bridge::COMMAND_CUE.ptr(b).addr;
            fill(a, b, rng, cue, 0x48 * 4);
        }
        for _ in 0..8 {
            let chr = rng.below(0x20);
            let sel = match rng.below(4) {
                0 => 0x1000 + rng.below(0x40),
                1 => rng.below(0x40),
                2 => rng.next(),
                _ => rng.range(-27, 0).cast_unsigned(),
            };
            let (all, any) = (rng.below(2), rng.below(2));
            twin(
                "MsGetChrBit",
                a,
                b,
                0x0079_4330,
                &[chr, sel, all, any],
                ALL,
                |m| act::ms_get_chr_bit(m, chr, sel, all.cast_signed(), any.cast_signed()),
            );
        }
        // targeting: reach, cursor, guards
        let cmd = w.command(a, b, rng);
        for _ in 0..6 {
            let chr = rng.range(-1, 0x20).cast_unsigned();
            twin(
                "MsGetCommandFar",
                a,
                b,
                0x0079_9690,
                &[chr, cmd],
                ALL,
                |m| act::ms_get_command_far(m, chr.cast_signed(), cmd),
            );
            let sel = match rng.below(3) {
                0 => rng.range(-27, 0).cast_unsigned(),
                1 => rng.below(0x20),
                _ => 0x1000 + rng.below(0x40),
            };
            let ev = [0xff, rng.below(4)][rng.below(2) as usize];
            twin(
                "MsGetCursorTarget",
                a,
                b,
                0x0079_9820,
                &[chr, sel, cmd, ev],
                ALL,
                |m| act::ms_get_cursor_target(m, chr, sel, cmd, ev),
            );
            let (u, t) = (rng.below(0x20), [0, rng.below(0x20)][rng.below(2) as usize]);
            twin("MsGetGuardChr", a, b, 0x0078_eef0, &[u, t], ALL, |m| {
                act::ms_get_guard_chr(m, u, t)
            });
            let i = rng.below(0x20);
            twin("MsGetChrVigor", a, b, 0x0078_cce0, &[i], ALL, |m| {
                act::ms_get_chr_vigor(m, i)
            });
        }
        // the message cue and Rikku's captures; TOBtlCloseSignWindow (the
        // system module's) runs as the original in both
        let close = |m: &mut Oracle| {
            let at = m.rebase(TO_BTL_CLOSE_SIGN_WINDOW);
            m.call(at, Conv::Cdecl, &[]).unwrap();
        };
        let title = [rng.byte().cast_signed(), rng.byte().cast_signed()];
        set(a, b, st::TITLE_WINDOW, title);
        let keep = rng.below(2);
        twin(
            "MsMenuCloseTitleWindow",
            a,
            b,
            0x0078_fa30,
            &[keep],
            0,
            |m| {
                st::ms_menu_close_title_window(m, keep.cast_signed(), close);
                0
            },
        );
        set(a, b, st::MESSAGE_CUE_COUNT, rng.below(6).to_le_bytes()[0]);
        set(a, b, st::MESSAGE_CUE_START, rng.byte());
        let args = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
        twin("MsMessageCueRegist", a, b, 0x0079_cfd0, &args, ALL, |m| {
            let [k, actor, msg, x, y] = args;
            st::ms_message_cue_regist(
                m,
                k.to_le_bytes()[0],
                actor,
                msg,
                [x.to_le_bytes()[0], y.to_le_bytes()[0]],
                close,
            )
        });
        let monster = match rng.below(3) {
            0 => 0xff,
            1 => rng.next(),
            _ => rng.below(0x140),
        };
        if rng.below(2) == 0 {
            let caps = ffx_battle::save::bridge::CAPTURES.ptr(b).addr;
            fill(a, b, rng, caps, 0x2c0);
        }
        let chr = rng.below(0x20);
        twin(
            "MsMonsterCapture",
            a,
            b,
            0x0079_0b30,
            &[chr, monster],
            ALL,
            |m| st::ms_monster_capture(m, chr, monster, close),
        );
        set(a, b, act::CHR_TOP, w.actors);
        same("getters", a, b);
    }
}

#[test]
fn actor_getters_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x7777_1111), true);
}

#[test]
fn actor_getters_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x1111_7777), false);
}
