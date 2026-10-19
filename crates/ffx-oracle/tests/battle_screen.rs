//! The battle screens' moves and links (`ffx_battle::screen`) against the
//! original, as in `battle.rs`: results and every written byte.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::*;
use ffx_battle::screen::bridge as sc;
use ffx_oracle::Oracle;

/// A screen index: mostly valid, sometimes not.
fn index(rng: &mut Xorshift) -> u32 {
    if rng.below(6) == 0 {
        rng.word()
    } else {
        rng.below(3)
    }
}

/// A float's bits, NaNs quiet.
fn float(rng: &mut Xorshift) -> u32 {
    if rng.below(8) == 0 {
        quiet(rng.next())
    } else {
        (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 97.0).to_bits()
    }
}

/// The low half of a word.
fn lo16(x: u32) -> u16 {
    u16::try_from(x & 0xffff).unwrap()
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let screens = a.rebase(0x0113_7000);
    let cams = a.rebase(0x0113_7540);
    for _ in 0..4000 {
        // a screen's positions (quiet floats) and move records, freshly drawn
        let s = rng.below(3);
        let pos: Vec<u8> = (0..0x18).flat_map(|_| float(rng).to_le_bytes()).collect();
        poke(a, b, screens + s * 0x1c0 + 0x40, &pos);
        fill(a, b, rng, screens + s * 0x1c0 + 0xdc, 0x80);
        if rng.below(2) == 0 {
            let c = rng.below(3);
            fill(a, b, rng, cams + c * 0xda0 + 0xc68, 0x20);
        }
        let i = index(rng);
        let side = if rng.below(4) == 0 {
            rng.word()
        } else {
            rng.below(2)
        };
        let (p, q, r) = (rng.word(), rng.word(), rng.word());
        match rng.below(6) {
            0 => {
                let k = if rng.below(4) == 0 {
                    rng.word() & 0xff
                } else {
                    rng.below(16)
                };
                twin("MsScreenLinkSet", a, b, 0x007b_bdd0, &[i, k, p], 0, |m| {
                    sc::ms_screen_link_set(m, i, k, p.to_le_bytes()[0]);
                    0
                });
            }
            1 => {
                let id = rng.below(3) | (rng.below(3) << 8) | (rng.below(2) << 16);
                let (x, y) = (float(rng), float(rng));
                set(a, b, sc::BTL_SCREEN, screens + rng.below(3) * 0x1c0);
                twin(
                    "MsScreenOffsetID",
                    a,
                    b,
                    0x007b_c0e0,
                    &[id, x, y],
                    u32::MAX,
                    |m| sc::ms_screen_offset_id(m, id, x, y),
                );
            }
            2 => {
                twin(
                    "MsScreenMoveCheck",
                    a,
                    b,
                    0x007b_bfc0,
                    &[i, side],
                    u32::MAX,
                    |m| sc::ms_screen_move_check(m, i, side),
                );
            }
            3 => {
                twin("MsScreenMoveReset", a, b, 0x007b_c010, &[i, side], 0, |m| {
                    sc::ms_screen_move_reset(m, i, side);
                    0
                });
            }
            4 => {
                twin(
                    "MsScreenMove",
                    a,
                    b,
                    0x007b_bdf0,
                    &[i, side, p, q],
                    0,
                    |m| {
                        sc::ms_screen_move(m, i, side, lo16(p), lo16(q));
                        0
                    },
                );
            }
            _ => {
                twin(
                    "MsScreenMoveAcc",
                    a,
                    b,
                    0x007b_beb0,
                    &[i, side, p, q, r],
                    0,
                    |m| {
                        sc::ms_screen_move_acc(m, i, side, lo16(p), lo16(q), lo16(r));
                        0
                    },
                );
            }
        }
        same("the screens", a, b);
    }
}

#[test]
fn screens_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x5c4e_e115));
}

#[test]
fn screens_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x0dd_ba11));
}
