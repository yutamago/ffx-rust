//! Shared functions of several families the shared-B list ported
//! (`ffx_magic::sb_misc`), each against the first DLL carrying it. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;
mod magic_l3;

use ffx_magic::{Ret, Xmm, sb_misc};
use magic::{Call, Eax, SCRATCH, Twin};

const LOG: u32 = SCRATCH + 0x8000;
/// Answers the stubs read (`ANS + 4 * slot % 0x100`).
const ANS: u32 = SCRATCH + 0x9000;

/// Each slot logs its first `n` words (a fresh row per call, 4 calls per
/// slot) and answers the word at `ANS + 4 * k` (`k` its index here).
fn log_slots(t: &mut Twin, slots: &[(u32, usize)]) {
    for (k, &(s, n)) in (0u32..).zip(slots) {
        let row = LOG + 0x100 * k;
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0xfc);
            m.w32(row + 0xfc, c.wrapping_add(1));
            let at = row + 0x40 * (c & 3);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(at + 4 * i, *w);
            }
            m.r32(ANS + 4 * k)
        });
    }
}

fn answers(t: &mut Twin, v: &[u32]) {
    t.write(LOG, &[0; 0x1000]);
    for (i, x) in (0u32..).zip(v) {
        t.set_u32(ANS + 4 * i, *x);
    }
}

#[test]
fn dll_mains() {
    for n in [52u16, 53, 42, 71, 187, 20, 45, 82, 54, 390] {
        magic_l3::dll_main(n);
    }
}

#[test]
fn mean_position() {
    let Some(mut t) = magic_l3::twin(52) else {
        return;
    };
    let f = t.at(0x1d80);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (count, own, bytes, chrs) = (w(&t, 0x11), w(&t, 0x46), w(&t, 0x4d), w(&t, 0x58));
    t.set_u32(own, 0x0c0c);
    t.set_u32(bytes, SCRATCH + 0x100);
    t.write(SCRATCH + 0x100, &[3, 1, 0, 2, 7]);
    for i in 0..8u32 {
        t.set_u32(chrs + 4 * i, 0xc400 + i);
    }
    // slot 262 writes a position from the character number
    t.stub(262, |m, a| {
        let c = a[0] & 0xff;
        m.w32(a[1], (1.5f32 * c as f32).to_bits());
        m.w32(a[2], (-2.0f32 + c as f32).to_bits());
        m.w32(a[3], (0.25f32 * c as f32).to_bits());
        m.w32(LOG, m.r32(LOG) + a[0]);
        0x262
    });
    let out = SCRATCH + 0x200;
    for n in [0u32, 1, 3, 5, 0xffff_ffff] {
        t.set_u32(count, n);
        t.check(f, Call::this(out), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_misc::mean_position(g, img, fr.at, fr.esp, fr.ecx))
        });
    }
}

#[test]
fn rotate_offset_and_list_pop() {
    let Some(mut t) = magic_l3::twin(52) else {
        return;
    };
    log_slots(&mut t, &[(235, 1), (328, 3), (329, 3), (229, 3), (248, 3)]);
    answers(&mut t, &[1, 2, 3, 4, 0x248]);
    let (src, dst, out) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    t.set_u32(src, 0.5f32.to_bits());
    t.set_u32(src + 4, (-1.25f32).to_bits());
    t.write(dst, &[0x11; 0x10]);
    t.write(out, &[0x22; 0x10]);
    t.check(
        t.at(0x2840),
        Call {
            eax: 0,
            ecx: src,
            edx: dst,
            ..Call::default()
        },
        &[out],
        Eax::Checked,
        |g, img, fr| {
            Ret::eax(sb_misc::rotate_offset(
                g, img, fr.at, fr.esp, fr.ecx, fr.edx,
            ))
        },
    );
    // the list: three nodes, the cursor on each in turn, then empty
    let list = SCRATCH + 0x400;
    let node = |i: u32| SCRATCH + 0x500 + 0x20 * i;
    for cur in [0u32, 1, 2] {
        for len in [0u32, 1, 3] {
            for i in 0..3 {
                t.set_u32(node(i), if i + 1 < len { node(i + 1) } else { 0 });
                t.set_u32(node(i) + 4, if i == 0 { 0x0bad } else { node(i - 1) });
                t.set_u32(node(i) + 8, 0x8888);
            }
            t.set_u32(list, node(len.saturating_sub(1)));
            t.set_u32(list + 4, if len == 0 { 0 } else { node(0) });
            t.set_u32(list + 8, node(cur));
            t.set_u32(list + 0xc, len);
            t.check(
                t.at(0x28f0),
                Call::this(list),
                &[],
                Eax::Checked,
                |g, img, fr| Ret::eax(sb_misc::list_pop(g, img, fr.at, fr.ecx)),
            );
        }
    }
}

#[test]
fn pool_take() {
    let Some(mut t) = magic_l3::twin(53) else {
        return;
    };
    let f = t.at(0x2410);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (idx, n, table, page) = (w(&t, 0x08), w(&t, 0x0f), w(&t, 0x37), w(&t, 0x2b));
    let recs = SCRATCH + 0x1000;
    t.set_u32(table, recs);
    t.stub(941, |m, a| {
        m.w32(LOG, m.r32(LOG) ^ a[0]);
        a[0] + 1
    });
    for (count, start, busy) in [
        (4u32, 0u32, 0b0000u32),
        (4, 2, 0b0100),
        (4, 3, 0b1000),
        (4, 1, 0b1111),
        (0, 0, 0),
        (6, 5, 0b10_0000),
        (6, 4, 0b11_0000),
        // all busy from the last record: the wrap comes first
        (4, 3, 0b1111),
    ] {
        for pg in [0u32, 1] {
            for i in 0..8 {
                t.write(recs + 0x74 * i, &[0x5a; 0x74]);
                t.set_u32(recs + 0x74 * i + 0x30, 0x300 + i);
                t.set_u32(recs + 0x74 * i + 0x6c, (busy >> i) & 1);
            }
            t.set_u32(n, count);
            t.set_u32(idx, start);
            // (page 1: the refill's keys go to the second 0x200 words)
            t.set_u32(page, pg);
            t.write(LOG, &[0; 4]);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(sb_misc::pool_take(g, img, fr.at))
            });
        }
    }
}

#[test]
fn node_append() {
    let Some(mut t) = magic_l3::twin(42) else {
        return;
    };
    let (node, list, last) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    for (l, tail) in [(0u32, 0u32), (list, list), (list, last)] {
        t.write(node, &[0xaa; 0x20]);
        t.write(last, &[0xbb; 0x20]);
        t.set_u32(list + 0x10, tail);
        t.set_u32(list + 0xc, 0x0c0c);
        t.set_u16(list + 0x18, 0xffff);
        t.check(
            t.at(0x7d90),
            Call {
                eax: 0x4242,
                ecx: node,
                edx: l,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(sb_misc::node_append(g, img, fr.at, fr.ecx, fr.edx, fr.eax)),
        );
    }
}

#[test]
fn part_setup_and_ease() {
    let Some(mut t) = magic_l3::twin(162) else {
        return;
    };
    let f = t.at(0x9500);
    let lock = SCRATCH + 0x50;
    t.set_u32(t.img.word(f + 0x04), lock);
    t.set_u32(t.img.word(f + 0x0a), 0x0000_7a77);
    t.stub(711, move |m, a| {
        for i in 0..3 {
            m.w32(LOG + 4 * i, a[i as usize]);
        }
        m.w32(LOG + 0xc, m.r32(lock));
        0x711
    });
    t.stub(710, |m, a| {
        for i in 0..7 {
            m.w32(LOG + 0x20 + 4 * i, a[i as usize]);
        }
        0x710
    });
    t.set_u32(lock, 9);
    // the four words part_setup clears hold something first
    for o in [0x58u32, 0x62, 0x6c, 0x76] {
        t.set_u32(t.img.word(f + o), 0x77);
    }
    t.check(
        f,
        Call::default(),
        &[0x00b0_f000],
        Eax::Checked,
        |g, img, fr| Ret::eax(sb_misc::part_setup(g, img, fr.at, fr.esp)),
    );
    // the key table: 6 keys, stride 2; the angles straddle the wrap
    let tab = SCRATCH + 0x400;
    let keys = [
        0.5f32, 2.75, -3.0, 3.1, 1.0, -0.5, 0.0, 6.0, -6.0, 2.0, 0.25, -1.5,
    ];
    // and: the same past a whole turn (the angle's turns taken off), every
    // key at -pi (the angle exactly 0) and neighbours exactly a half turn
    // apart (pi is the angle's offset `+0x07`)
    let pi = std::f32::consts::PI;
    let tables = [
        keys,
        keys.map(|k| k + 4.0),
        [-pi; 12],
        [0.0, pi, 0.0, -pi, pi, 0.0, -pi, 0.0, pi, pi, 0.0, -pi],
    ];
    for keys in tables {
        for (i, k) in (0u32..).zip(keys) {
            t.set_u32(tab + 4 * i, k.to_bits());
        }
        for (rva, angle) in [(0x90a0u32, false), (0x91a0, true)] {
            for time in [
                0.0f32, 0.25, 0.5, 0.75, 1.0, 1.4, 2.6, 3.0, 3.9, 4.5, 4.99, -0.0,
            ] {
                for (n, stride) in [(6u32, 2u32), (5, 2), (3, 4), (12, 1)] {
                    if time >= n as f32 {
                        continue;
                    }
                    t.check(
                        t.at(rva),
                        Call {
                            eax: 0,
                            ecx: tab,
                            edx: n,
                            ..Call::default()
                        }
                        .with_xmm(&[Xmm::f32(time)]),
                        &[stride],
                        Eax::Junk,
                        |g, img, fr| {
                            let x = if angle {
                                sb_misc::ease_angle(
                                    g,
                                    img,
                                    fr.at,
                                    fr.esp,
                                    fr.ecx,
                                    fr.edx,
                                    fr.xmm_f32(0),
                                )
                            } else {
                                sb_misc::ease_sample(
                                    g,
                                    img,
                                    fr.at,
                                    fr.esp,
                                    fr.ecx,
                                    fr.edx,
                                    fr.xmm_f32(0),
                                )
                            };
                            Ret::xmm0(Xmm::f32(x), 0)
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn start_steps() {
    let Some(mut t) = magic_l3::twin(71) else {
        return;
    };
    let f = t.at(0x93c0);
    let state = t.img.word(f + 0x01);
    log_slots(&mut t, &[(225, 1), (313, 1), (549, 1), (226, 2)]);
    // slot 226's global argument (non-zero, so that reading it shows)
    t.set_u32(t.img.word(f + 0x5e), 0x0226_005e);
    for s in [0u32, 1, 2, 3, 4, 0xffff_ffff] {
        for a in [[0u32, 0, 0, 7], [1, 0, 0, 7], [0, 1, 0, 7], [0, 0, 1, 7]] {
            answers(&mut t, &a);
            t.set_u32(state, s);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

#[test]
fn small_entries() {
    let Some(mut t) = magic_l3::twin(187) else {
        return;
    };
    log_slots(&mut t, &[(439, 1), (438, 1)]);
    answers(&mut t, &[0x439, 0x438]);
    let g = t.img.word(t.at(0x14a0) + 0x07);
    t.set_u32(g, 0x0123_4567);
    t.check_entry(t.at(0x14a0), Call::default(), &[], Eax::Checked);
    t.check_entry(t.at(0x14c0), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = magic_l3::twin(20) else {
        return;
    };
    log_slots(&mut t, &[(713, 1)]);
    let p = t.img.word(t.at(0x16b0) + 0x04);
    t.set_u32(p, SCRATCH + 0x10);
    t.check_entry(t.at(0x16b0), Call::default(), &[0x00ab_0000], Eax::Checked);
}

#[test]
fn load_step() {
    let Some(mut t) = magic_l3::twin(45) else {
        return;
    };
    let f = t.at(0x1e90);
    let (a, b, c) = (
        t.img.word(f + 0x14),
        t.img.word(f + 0x3c),
        t.img.word(f + 0x67),
    );
    log_slots(&mut t, &[(253, 0), (225, 1), (226, 2), (227, 1)]);
    // the two counters and the slots' global arguments: non-zero, so
    // that reading them shows
    for (o, v) in [
        (0x36u32, 0x100u32),
        (0x4c, 0x0226_004c),
        (0x75, 0x0227_0075),
        (0x8b, 0x200),
    ] {
        t.set_u32(t.img.word(f + o), v);
    }
    // the counters are the flags `+0x14`/`+0x67` (the same globals), so
    // they are 0 whenever they are counted up: the words a misread
    // operand would name instead (one byte on) are made non-zero
    for o in [0x37u32, 0x8c] {
        t.set_u32(t.img.word(f + o), 0x5a5a);
    }
    for ready in [0u32, 1] {
        for r225 in [0u32, 1] {
            for r227 in [0u32, 1, 2, 0x8000_0000] {
                for flags in 0..8u32 {
                    answers(&mut t, &[ready, r225, 0x226, r227]);
                    t.set_u32(a, flags & 1);
                    t.set_u32(b, (flags >> 1) & 1);
                    t.set_u32(c, (flags >> 2) & 1);
                    t.check_entry(f, Call::default(), &[], Eax::Checked);
                }
            }
        }
    }
}

#[test]
fn move_start() {
    let Some(mut t) = magic_l3::twin(82) else {
        return;
    };
    // the start and end points: whatever the DLL's data holds, made
    // non-zero
    let f = t.at(0x6710);
    let lo = t.img.word(f + 0x0c);
    for i in 0..0x30u32 {
        t.set_u32(lo + 4 * i, (1.0f32 + i as f32 * 0.375).to_bits());
    }
    let fl = t.img.word(f + 0x12);
    for flags in [0u32, 1, 0xffff_ffff] {
        t.set_u32(fl, flags);
        t.check(
            f,
            Call {
                eax: 0x77,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(2.5)]),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(sb_misc::move_start(g, img, fr.at, fr.xmm_f32(2), fr.eax)),
        );
        // the word move_start_2 clears holds something first
        t.set_u32(t.img.word(t.at(0x67f0) + 0x3d), 0x77);
        t.check(
            t.at(0x67f0),
            Call::default().with_xmm(&[
                Xmm::f32(0.0),
                Xmm::f32(0.0),
                Xmm::f32(2.5),
                Xmm::f32(-7.0),
            ]),
            &[],
            Eax::Checked,
            |g, img, fr| {
                Ret::eax(sb_misc::move_start_2(
                    g,
                    img,
                    fr.at,
                    fr.xmm_f32(2),
                    fr.xmm_f32(3),
                ))
            },
        );
    }
}

#[test]
fn anim_reset() {
    let Some(mut t) = magic_l3::twin(54) else {
        return;
    };
    let obj = SCRATCH + 0x100;
    t.write(obj, &[0xa5; 0x140]);
    t.check(
        t.at(0x1a70),
        Call {
            eax: 0,
            ecx: obj,
            edx: 0x1234,
            ..Call::default()
        },
        &[],
        Eax::Checked,
        |g, _img, fr| Ret::eax(sb_misc::anim_reset(g, fr.ecx, fr.edx)),
    );
}

#[test]
fn op_free_part() {
    let Some(mut t) = magic_l3::twin(390) else {
        return;
    };
    let f = t.at(0x2fc0);
    log_slots(&mut t, &[(713, 1)]);
    t.set_u32(t.img.word(f + 0x1e), SCRATCH + 0x10);
    let (ip, owner, part) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    t.set_u32(ip + 4, owner);
    t.set_u32(ip + 8, part);
    t.set_u16(part + 0x124, 0xffff);
    // the list at owner +0x30 (head, tail, cursor, count; a node's +0 the
    // previous, +4 the next, +8 the item): empty, then three nodes with
    // the part in each place
    let list = owner + 0x30;
    let node = |i: u32| SCRATCH + 0x400 + 0x10 * i;
    t.write(owner, &[0; 0x60]);
    t.check_entry(f, Call::default(), &[SCRATCH, ip], Eax::Checked);
    for at in [0u32, 1, 2, 3] {
        for i in 0..3 {
            t.set_u32(node(i), if i == 0 { 0 } else { node(i - 1) });
            t.set_u32(node(i) + 4, if i == 2 { 0 } else { node(i + 1) });
            t.set_u32(node(i) + 8, if i == at { part } else { 0x0e00 + i });
        }
        t.set_u32(list, node(0));
        t.set_u32(list + 4, node(2));
        t.set_u32(list + 8, node(1));
        t.set_u32(list + 0xc, 3);
        t.set_u16(part + 0x124, 0xffff);
        t.check_entry(f, Call::default(), &[SCRATCH, ip], Eax::Checked);
    }
}

// Mutation testing (M5 magic round 3, tools/mutants.py, a mutant per
// code line plus a deletion per store or call):
// - ffx_magic::sb_misc (against this file and magic_sb_misc2.rs): 923
//   mutants, 901 killed. Equivalent:
//   - mean_position: the `i == u32::MAX` arm (i counts up from 0 and
//     stops at a signed count) and the frame vector's `w = 1.0` (slot 262
//     gets x, y, z only; nothing reads it).
//   - pool_take: `esi < esi + 0x200` as `<=` (true either way, `esi` is
//     `page << 9`); `l4`'s step (it only places a second refill in the
//     same call, and the search ends after `n` probes, one wrap at most,
//     with the count slot 941 does not change).
//   - pool_take_140: `tried` counting down (the search then ends after
//     2^31 probes instead of `n`, with the same 0: the loop stores nothing
//     until it finds a record) and `>=` as `>` (one more probe of a record
//     already refused).
//   - unwrap_near's `a > b` as `>=` (at `a == b` both arms give `b`,
//     `0 > k5` being false for k5 = pi); ease_angle's `turns > 0` as `>=`
//     (taking off `0 * k6` = +0 leaves any `x2` as it is).
//   - timeline_run's `r` initial value (only `r == 0` matters).
//   - the ST0-less defaults of actor_scale, place_model_unscaled and
//     start_551 (the slots always return a float; Unicorn reads an empty
//     x87 register as 0 where the CPU stores the indefinite NaN).
//   - lookup_actors: `+0x34`/`+0x73` swapped (the same global); slot 215
//     as 219 (the mode word `+0x42` is `+0x18`, which the function sets
//     to 1 first: the 215 arm never runs).
//   - start_551: the `ecx` of the cdecl slot 221 call.
//   - sprite_init_colour's and aim_parent's `eax` initial values (the
//     loops set them).
//   - deal: `ebp` 8 bytes up (the frame's scratch counts and pool move,
//     still below the entry for any pool the original's 256 bytes hold);
//     `max(0)` as `max(1)` (n > 1 there); `c > 0` as `>= 0` (a zero count
//     adds nothing); `unwrap_or(1)` (only for an empty pool, where the
//     original's `idiv` faults).
//   - pick_variant's record step (the count `+0x5c` is `+0x24`, set to 1
//     just before: one record).
