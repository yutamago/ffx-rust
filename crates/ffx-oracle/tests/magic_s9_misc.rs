//! The shared-9 list's ports (`ffx_magic::s9_misc`) against the DLLs
//! they were ported from and the other DLLs that carry them. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;

use ffx_magic::s9_misc;
use magic::{Call, Eax, SCRATCH, STACK_TOP, StubMem, Twin};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// The word at `f + off` of the code.
fn op(t: &Twin, f: u32, off: u32) -> u32 {
    t.img.word(f + off)
}

/// `n` bytes that differ from their neighbours (`seed` picks the pattern).
fn varied(n: u32, seed: u32) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + seed * 13 + 1) as u8).collect()
}

/// `n` words of small floats that differ from their neighbours.
fn floats(n: u32, seed: u32) -> Vec<u8> {
    (0..n)
        .flat_map(|i| {
            let x = ((i * 37 + seed * 11) % 41) as f32 - 20.0;
            (x * 0.125).to_bits().to_le_bytes()
        })
        .collect()
}

fn set_f(t: &mut Twin, a: u32, x: f32) {
    t.set_u32(a, x.to_bits());
}

/// The entry's stack pointer (at the return address).
const ENTRY_ESP: u32 = STACK_TOP - 0x100;

/// The frame area below the entry's stack pointer filled with a pattern
/// before a run (the stores of the run before, or zeros, would hide a
/// port's dropped store from the stubs).
fn scrub(t: &mut Twin) {
    t.write(ENTRY_ESP - 0x3000, &varied(0x3000, 0x5b));
}

/// The call log: a count at `LOG`, then rows of 0x40 bytes.
const LOG: u32 = SCRATCH + 0x6000;
/// The log's rows.
const ROWS: u32 = 128;

/// Logs each call of `slot` into a row of the shared log (in call order):
/// the slot, its first `n` words, and 4 words at each argument `ptrs`
/// names. Answers `ret(m, args, k)`, `k` the call's number.
fn log_calls(
    t: &mut Twin,
    slot: u32,
    n: usize,
    ptrs: &'static [usize],
    ret: impl Fn(&mut dyn StubMem, &[u32], u32) -> u32 + 'static,
) {
    t.stub(slot, move |m, a| {
        let k = m.r32(LOG);
        m.w32(LOG, k + 1);
        let row = LOG + 4 + 0x40 * (k % ROWS);
        m.w32(row, slot);
        for (i, w) in (1u32..).zip(&a[..n.min(a.len())]) {
            m.w32(row + 4 * i, *w);
        }
        for (j, &p) in (0u32..).zip(ptrs) {
            for i in 0..4 {
                m.w32(row + 0x20 + 0x10 * j + 4 * i, m.r32(a[p] + 4 * i));
            }
        }
        ret(m, a, k)
    });
}

/// The scene start with its node tree in both DLLs: the targets (none
/// and two), the lock and entries seen by the slots.
#[test]
fn scene_starts_tree() {
    for dll in [86u16, 87] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x6c50);
        let lock = SCRATCH + 0x5f80;
        t.set_u32(op(&t, f, 0x2b2), lock);
        log_calls(&mut t, 168, 1, &[], |_, a, _| 0x0168_0000 + a[0]);
        let list = SCRATCH + 0x5d00;
        log_calls(&mut t, 218, 2, &[], move |m, a, _| {
            m.w32(a[1], m.r32(list + 0x7c));
            list
        });
        log_calls(&mut t, 531, 2, &[], |m, a, _| {
            for i in 0..4 {
                m.w32(a[1] + 4 * i, a[0].wrapping_mul(0x0101_0101) ^ i);
            }
            0x531
        });
        log_calls(&mut t, 289, 1, &[], |_, _, _| 0x289);
        t.stub_st0(289, |_, a| f64::from(a[0] & 0xff) * -0.5);
        for (s, n) in [
            (718u32, 1usize),
            (711, 3),
            (710, 4),
            (222, 4),
            (221, 4),
            (551, 3),
        ] {
            log_calls(&mut t, s, n, &[], move |m, a, k| {
                m.w32(SCRATCH + 0x5f84 + 4 * (k % 8), m.r32(lock));
                a[0] ^ (s << 16) ^ k
            });
        }
        log_calls(&mut t, 719, 3, &[], |_, a, k| {
            SCRATCH + 0x4000 + 0x100 * (a[1] & 0xf) + k
        });
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        log_calls(&mut t, 941, 1, &[], move |_, _, _| table);
        // the start's float slots (`magic_0078.rs`' stubs)
        t.stub_st0(790, |_, _| 0.25);
        t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
        t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) + 0.75);
        // yiGetFogColor fills the colour node's four words (left unset,
        // the node read a return address an earlier call left there)
        t.stub(558, |m, a| {
            for i in 0..4 {
                m.w32(a[0] + 4 * i, 0x20 + 3 * i);
            }
            0x558
        });
        for (k, count) in [0u32, 2].into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.set_u32(lock, 0x1100 + k);
            t.write(list, &[0x80, 0x81, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88]);
            t.set_u32(list + 0x7c, count);
            t.set_u16(op(&t, f, 0x14), 0x10 + k as u16);
            t.set_u8(op(&t, f, 0x1fd), 0x5a + k as u8);
            // the floats the start reads (`magic_0078.rs`' values)
            let start = t.img.rel32_target(f + 0x36c);
            for (i, o) in (0u32..).zip([
                0x2f5u32, 0x32d, 0x368, 0x38f, 0x3a2, 0x3b5, 0x3bd, 0x3cd, 0x3d5,
            ]) {
                t.set_u32(op(&t, start, o), (0.75 + i as f32 * 1.5).to_bits());
            }
            scrub(&mut t);
            let c = Call {
                eax: 0x0eee_0000 + k,
                ecx: 0x0ccc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[0x0123_0000 + k], Eax::Checked);
        }
    }
}

/// The object start's set-up (`s4_misc::start_object_at` at `soa`): its
/// globals, slots 941 and 714, and a pool of `free` nodes.
fn object_start_setup(t: &mut Twin, soa: u32, free: u16) {
    t.set_u32(op(t, soa, 0x04), SCRATCH + 0x200);
    t.set_u32(op(t, soa, 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    log_calls(t, 941, 1, &[], |_, _, _| SCRATCH + 0x400);
    log_calls(t, 714, 5, &[], |_, a, _| 0x0714_0000 ^ a[2]);
    let pa = t.img.rel32_target(soa + 0x9d);
    let nodes = op(t, pa, 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(op(t, pa, 0x04), free);
    t.set_u32(op(t, soa, 0xa8), 0);
}

/// The task ring of `free_to_live` (operands `count`, `head`, `first`,
/// `last` of `f`): `free` nodes linked in a ring (each node's `+0` the
/// next, `+4` the previous; the take moves the head to the previous one),
/// the live list empty or not (`k` odd). None free makes the original
/// write at address 4, so `free` is at least 1.
fn task_ring(
    t: &mut Twin,
    f: u32,
    (count, head, first, last): (u32, u32, u32, u32),
    k: u32,
    free: u32,
) {
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    t.write(node(0), &varied(0x40 * (free + 1), k + 1));
    for i in 0..free {
        t.set_u32(node(i), node((i + 1) % free));
        t.set_u32(node(i) + 4, node((i + free - 1) % free));
    }
    t.set_u16(op(t, f, count), free as u16);
    t.set_u32(op(t, f, head), node(0));
    t.set_u32(op(t, f, first), if k % 2 == 1 { node(free) } else { 0 });
    t.set_u32(op(t, f, last), node(free));
}

/// Rows 176/177: the actors' paired objects started in both DLLs (no
/// entries, one, three; the pool full and short, the task ring with 2
/// or 1 free nodes and the live list empty or not), and the load wait
/// that starts them (busy, the first call, ready).
#[test]
fn paired_objects_started() {
    for (dll, rva) in [(114u16, 0x1210u32), (115, 0x29a0)] {
        let Some(mut t) = twin(dll) else { return };
        let load = t.at(rva);
        let f = t.img.rel32_target(load + 0x5c);
        let soa = t.img.rel32_target(f + 0xce);
        let ans = SCRATCH + 0x2f0;
        log_calls(&mut t, 225, 1, &[], move |m, _, _| m.r32(ans));
        log_calls(&mut t, 226, 2, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 227, 1, &[], move |m, _, _| m.r32(ans + 4));
        log_calls(&mut t, 719, 3, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 711, 3, &[], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 710, 4, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 262, 4, &[], |m, a, k| {
            for (i, x) in (1u32..).zip(&a[1..4]) {
                m.w32(*x, (0.5 * i as f32 + k as f32).to_bits());
            }
            0x262
        });
        log_calls(&mut t, 268, 3, &[], |m, a, k| {
            m.w32(a[2] + 4, a[0] ^ k);
            0x268
        });
        let pi = t.img.rel32_target(f + 0x45);
        t.set_u32(op(&t, pi, 0x01), SCRATCH + 0x7e0);
        let w = |t: &Twin, o: u32| op(t, f, o);
        for (o, x) in [
            (0xb5u32, 0.75f32),
            (0x128, 1.5),
            (0x156, -2.5),
            (0x22d, 0.375),
        ] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let bl = SCRATCH + 0x900;
        let actors = w(&t, 0x93);
        // (entries, pool nodes, load: busy / first / ready)
        for (k, (n, free, busy, once, ready)) in [
            (0i32, 4u16, 0u32, 1u32, 1u32),
            (1, 4, 0, 0, 1),
            (3, 8, 0, 1, 2),
            (3, 2, 0, 1, 1),
            (2, 4, 1, 0, 1),
            (2, 4, 0, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            object_start_setup(&mut t, soa, free);
            t.set_u32(ans, busy);
            t.set_u32(ans + 4, ready);
            t.set_u32(op(&t, load, 0x18), once);
            t.set_u32(op(&t, load, 0x27), 0x0b0b_0000 + k);
            for o in [0x62u32, 0x6c] {
                t.set_u32(op(&t, load, o), 0x0c0c_0000 + o);
            }
            t.set_u32(w(&t, 0x5c), n.cast_unsigned());
            t.set_u32(w(&t, 0x68), bl);
            t.write(bl, &[2, 1, 7, 0]);
            for i in 0..8u32 {
                t.set_u32(actors + 4 * i, 0x0ac7_0000 + i);
            }
            // the objects, points and records seeded (the zeroed words)
            t.write(w(&t, 0x4f), &floats(0x260 * 3 / 4, k + 5));
            t.write(w(&t, 0x6e), &varied(0x130 * 6, k + 6));
            t.write(w(&t, 0x17a), &varied(0x34 * 3, k + 7));
            task_ring(&mut t, f, (0x28c, 0x29b, 0x302, 0x313), k, 1 + k % 2);
            scrub(&mut t);
            let c = Call {
                eax: 0x1370_0000 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
            // the same through the load wait
            t.set_u32(LOG, 0);
            object_start_setup(&mut t, soa, free);
            task_ring(&mut t, f, (0x28c, 0x29b, 0x302, 0x313), k + 1, 2 - k % 2);
            scrub(&mut t);
            t.check_entry(load, c, &[], Eax::Checked);
        }
    }
}

/// A strip of `len` glyphs at `at` (`[len, bytes]`, the bytes after it;
/// glyph 10 among them).
fn reel_strip(t: &mut Twin, at: u32, len: u32, seed: u32) {
    t.set_u32(at, len);
    t.set_u32(at + 4, at + 0x10);
    let bytes: Vec<u8> = (0..len + 8).map(|i| ((i * 3 + seed) % 12) as u8).collect();
    t.write(at + 0x10, &bytes);
}

/// The reel draw slots logged (the colour, vertex, end, clip, texture
/// and glyph slots) and the texture handle slot 719 answers.
fn reel_slots(t: &mut Twin) {
    let tex = SCRATCH + 0x800;
    t.set_u32(tex, 0x0123_4567);
    t.set_u32(tex + 4, 0x89ab_cdef);
    log_calls(t, 719, 3, &[], move |m, a, _| {
        m.w32(SCRATCH + 0x810, a[2]);
        tex
    });
    for (s, n) in [
        (382u32, 2usize),
        (810, 3),
        (811, 5),
        (809, 3),
        (812, 4),
        (813, 4),
        (814, 0),
        (208, 2),
        (209, 2),
    ] {
        log_calls(t, s, n, &[], move |_, _, k| (s << 16) + k);
    }
}

/// A reel at `r`: the strip, the position word, the bounce.
fn reel_at(t: &mut Twin, r: u32, strip: u32, pos: u32, bounce: i32) {
    t.write(r, &varied(0x1c, pos));
    t.set_u32(r, strip);
    t.set_u32(r + 4, pos);
    t.set_u32(r + 0x14, bounce.cast_unsigned());
}

/// Rows 4, 5, 7, 8: the reel game's frame, window, reel and next glyphs
/// in both layouts (`magic_0565`, `magic_0644`): positions and sizes
/// (negative too), every bounce (below, at and past 3; glyph 10 with each
/// sign), the strip wrapping, the count above, at and below 0.
#[test]
fn reel_parts() {
    for (dll, base) in [(565u16, 0x2300u32), (644, 0x6490)] {
        let Some(mut t) = twin(dll) else { return };
        reel_slots(&mut t);
        let frame = t.at(base);
        let window = t.at(base + 0x700);
        let draw = t.at(base + 0x980);
        let next = t.at(base + 0xae0);
        for (k, (x, y, args)) in [
            (3u32, 5u32, [0x10u32, 0x20, 0x80, 0x40, 0x7f]),
            (0, 0, [1, 1, 0, 0xff, 0]),
            (0xffff_fff0, 0x30, [0x100, 0xffff_ff00, 0x55, 0x66, 0x77]),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            let c = Call {
                ecx: x,
                edx: y,
                eax: 0x2300 + k,
                ..Call::default()
            };
            t.check(frame, c, &args, Eax::Checked, s9_misc::reel_frame);
            t.set_u32(LOG, 0);
            t.check(window, c, &[], Eax::Checked, s9_misc::reel_window);
        }
        let strip = SCRATCH + 0x1800;
        let reels = SCRATCH + 0x1a00;
        // (strip length, position, bounce, column, row)
        for (k, (len, pos, bounce, x, row)) in [
            (12u32, 0x0003_8000u32, 0i32, 0x20u32, 0x10u32),
            (6, 0x0005_ffff, 2, 0, 0xffff_fff0),
            (9, 0x0000_0001, 3, 0x123, 7),
            (10, 0x0002_4000, -5, 0x40, 0x41),
            (10, 0xfffe_1234, -2, 0x40, 0x41),
            (11, 0x000a_0000, 5, 1, 2),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            reel_strip(&mut t, strip, len, k);
            reel_at(&mut t, reels, strip, pos, bounce);
            scrub(&mut t);
            let c = Call {
                ecx: reels,
                edx: row,
                eax: 0x2c80 + k,
                ..Call::default()
            };
            t.check(draw, c, &[x], Eax::Checked, s9_misc::reel_draw);
            // the next glyphs: three reels, the count `+0x6c`
            t.set_u32(LOG, 0);
            for i in 0..3 {
                let r = reels + 0x1c * i;
                reel_at(&mut t, r, strip, pos, bounce);
                // the position just before the strip's end, or inside it
                let at = if (k + i).is_multiple_of(2) {
                    len - 1
                } else {
                    i + k
                };
                t.set_u16(r + 6, at as u16);
            }
            t.set_u32(reels + 0x6c, [0u32, 1, 5, 0xffff_fffe, 8, 9][k as usize]);
            scrub(&mut t);
            t.check(next, c, &[], Eax::Checked, s9_misc::reel_next_glyphs);
        }
    }
}

/// Row 9: the reel game drawn in both layouts: no machines, one and
/// three (each with its reels on and off, the next glyphs for the
/// machines below `*G_K`).
#[test]
fn reels_drawn() {
    for (dll, rva) in [(565u16, 0x2e80u32), (644, 0x7010)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        reel_slots(&mut t);
        let strip = SCRATCH + 0x1800;
        let m0 = op(&t, f, 0x7a);
        // (machines, which are on, *G_K)
        for (k, (n, on, below)) in [
            (0u32, 0u32, 0u32),
            (1, 1, 1),
            (3, 0b101, 2),
            (3, 0b111, 0),
            (2, 0b10, 5),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            reel_strip(&mut t, strip, 8 + k, k);
            t.set_u32(op(&t, f, 0x6b), n);
            t.set_u32(op(&t, f, 0xd8), below);
            for i in 0..3u32 {
                let m = m0 + 0x78 * i;
                for j in 0..3 {
                    let r = m - 0x5c + 0x1c * j;
                    reel_at(
                        &mut t,
                        r,
                        strip,
                        0x0001_0000 * (i + j) + 0x1234 * k,
                        j.cast_signed() - 1,
                    );
                }
                t.set_u32(m - 0x5c + 0x6c, i + k);
                t.set_u32(m - 4, 0x30 + 0x10 * i);
                t.set_u32(m, 0x20 * i + k);
                t.set_u32(m + 0x18, (on >> i) & 1);
            }
            scrub(&mut t);
            let c = Call {
                eax: 0x2e80 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
}

/// A model with sprite groups at `s` for `sprite_packets`: the groups of
/// `(type, records)`, each record's colours varied; the count at `+0x10`
/// is `total`.
fn sprite_model(t: &mut Twin, s: u32, groups: &[(u8, i16)], total: i16, seed: u32) {
    t.write(s, &varied(0x40, seed));
    t.set_u32(s + 4, 0x40);
    t.set_u16(s + 0x10, total.cast_unsigned());
    let mut g = s + 0x40;
    for (i, &(kind, n)) in (0u32..).zip(groups) {
        t.write(g, &varied(0x10, seed + i));
        t.set_u8(g + 1, kind);
        t.set_u16(g + 2, n.cast_unsigned());
        let n = u32::try_from(n.max(0)).unwrap_or(0);
        t.write(g + 0x10, &varied(0x28 * n, seed + 7 * i));
        g += 0x10 + 0x28 * n;
    }
}

/// A sprite model's groups `(type, records)`, its count and the alpha
/// scale.
type Groups = (&'static [(u8, i16)], i16, u32);

/// The matrix and transform slots of `sprite_packets` (235, 554) logged.
fn sprite_slots(t: &mut Twin) {
    log_calls(t, 235, 1, &[], |m, a, k| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, (0.25 * i as f32 + k as f32).to_bits());
        }
        0x235
    });
    log_calls(t, 554, 4, &[0, 2], |_, _, _| 0x554);
}

/// Row 11: the sprite packets in two layouts (`magic_0084`,
/// `magic_0085`): no records, groups of type 3 (one, several, an empty
/// one), a group of another type first and after one, the count running
/// out inside a group; alpha scales positive and negative.
#[test]
fn sprite_packets() {
    for (dll, rva) in [(84u16, 0x1fd0u32), (85, 0x15b0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        sprite_slots(&mut t);
        let s = SCRATCH + 0x2000;
        let p = SCRATCH + 0x3000;
        let gp = op(&t, f, 0x1a);
        let cases: [Groups; 7] = [
            (&[], 0, 0x80),
            (&[(3, 1)], 1, 0x80),
            (&[(3, 2), (3, 0), (3, 3)], 5, 0x40),
            (&[(2, 2)], 2, 0x100),
            (&[(3, 1), (4, 1)], 3, 0xffff_ff80),
            (&[(3, 3)], 2, 7),
            (&[(3, 1)], -1, 0x80),
        ];
        for (k, (groups, total, scale)) in cases.into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(LOG, 0);
            sprite_model(&mut t, s, groups, total, k);
            t.write(p, &varied(0x1a0 * 6, k + 3));
            t.set_u32(gp, p);
            scrub(&mut t);
            let c = Call {
                ecx: s,
                eax: 0x1fd0 + k,
                ..Call::default()
            };
            let args = [0x0a00_0000 + k, 0x0b00_0000 + k, scale];
            t.check(f, c, &args, Eax::Checked, s9_misc::sprite_packets);
        }
    }
}

/// Rows 114, 115, 189, 190, 193, 195: a model's sprites drawn in each
/// copy and DLL (the alpha field positive and negative, two sprite groups).
#[test]
fn model_sprites() {
    // (dll, address): the four layouts in every DLL carrying them
    for (dll, rva) in [
        (84u16, 0x17c0u32),
        (413, 0x3420),
        (675, 0x1aa0),
        (84, 0x18c0),
        (675, 0x1ba0),
        (85, 0x34b0),
        (619, 0x37b0),
        (93, 0x34e0),
        (681, 0x3840),
        (440, 0x34d0),
        (680, 0x1b70),
        (440, 0x3600),
        (680, 0x1ca0),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        sprite_slots(&mut t);
        let stores = t.img.word(f + 0x1a) & 0xffff == 0x0ff3;
        let (call, gx, lock) = if stores {
            (
                if t.img.word(f + 0x80) & 0xff00 == 0x4900 {
                    0x84
                } else {
                    0x87
                },
                0x77,
                0xcf,
            )
        } else {
            (
                if t.img.word(f + 0x52) & 0xff00 == 0x4900 {
                    0x56
                } else {
                    0x59
                },
                0x49,
                0xa1,
            )
        };
        let lock = if call == 0x59 || call == 0x87 {
            lock + 6
        } else {
            lock
        };
        let packets = t.img.rel32_target(f + call);
        let tex = SCRATCH + 0x800;
        t.set_u32(tex, 0x0123_4567);
        t.set_u32(tex + 4, 0x89ab_cdef);
        log_calls(&mut t, 719, 3, &[], move |_, _, _| tex);
        let lk = SCRATCH + 0x7f0;
        t.set_u32(op(&t, f, lock), lk);
        t.stub(815, move |m, a| {
            let k = m.r32(LOG);
            m.w32(LOG, k + 1);
            let row = LOG + 4 + 0x80 * (k % 32);
            for (i, x) in (0u32..).zip(&a[..3]) {
                m.w32(row + 4 * i, *x);
            }
            m.w32(row + 0xc, m.r32(lk));
            // the record's words the copy writes (the rest is stale stack)
            for o in [0u32, 8, 0x10, 0x14, 0x18, 0x20, 0x44] {
                m.w32(row + 0x10 + o, m.r32(a[2] + o));
            }
            0x815
        });
        let x = SCRATCH + 0x4800;
        let c = SCRATCH + 0x4900;
        let s = SCRATCH + 0x2000;
        t.set_u32(op(&t, f, gx), x - 4);
        t.set_u32(x - 4, x);
        t.write(x, &varied(0x40, 9));
        t.set_u32(x + 0x20, c);
        t.write(c, &varied(0x400, 4));
        let gp = op(&t, packets, 0x1a);
        if stores {
            // the four globals the copy sets first, seeded non-zero
            for (i, o) in [0x30u32, 0x3c, 0x42, 0x4c].into_iter().enumerate() {
                t.set_u32(op(&t, f, o), 0x5a5a_0000 + i as u32);
            }
        }
        for (k, half) in [0x1234i16, -0x0567, 0x7fff].into_iter().enumerate() {
            let k = k as u32;
            t.set_u32(LOG, 0);
            // every model offset of the copies points at the model
            for o in [0x3cu32, 0x11c, 0xfc, 0x15c, 0x5c, 0x23c] {
                t.set_u32(c + o, s);
            }
            sprite_model(&mut t, s, &[(3, 2), (3, 1)], 3, k);
            t.set_u32(lk, 0x0100 + k);
            t.write(SCRATCH + 0x3000, &varied(0x1a0 * 4, k + 3));
            t.set_u32(gp, SCRATCH + 0x3000);
            let o = SCRATCH + 0x5000;
            t.write(o, &varied(0x140, k + 2));
            for at in [0x116u32, 0x136] {
                t.set_u16(o + at, half.cast_unsigned());
            }
            scrub(&mut t);
            let call = Call {
                eax: 0x17c0 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[o], Eax::Checked);
        }
    }
}

/// Row 13: the beams aimed and started in two of the DLLs: no entries,
/// one and three (the yaw past both bounds, the count's sign), the
/// object on the list and not.
#[test]
fn beams_aimed() {
    for dll in [446u16, 555] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x2c20);
        let w = |t: &Twin, o: u32| op(t, f, o);
        log_calls(&mut t, 219, 2, &[], |_, a, k| a[0] ^ 0x0219_0000 ^ k);
        log_calls(&mut t, 237, 4, &[], |m, a, k| {
            for i in 0..3 {
                m.w32(a[3] + 4 * i, (1.5 * i as f32 - 2.0 + k as f32).to_bits());
            }
            0x237
        });
        log_calls(&mut t, 262, 4, &[], |m, a, k| {
            for (i, x) in (0u32..).zip(&a[1..4]) {
                let v = [3.0f32, -1.25, 4.5][i as usize] * (k as f32 + 1.0) - 2.0;
                m.w32(*x, v.to_bits());
            }
            0x262
        });
        log_calls(&mut t, 268, 3, &[0, 2], |_, _, _| 0x268);
        log_calls(&mut t, 713, 1, &[0], |_, _, _| 0x713);
        log_calls(&mut t, 257, 2, &[], |_, a, _| a[0] ^ 0x0257_0000);
        let soa = t.img.rel32_target(f + 0x272);
        let b = SCRATCH + 0x8000;
        t.set_u32(w(&t, 0x29), b);
        let list = w(&t, 0x38b);
        let gp = SCRATCH + 0x7e8;
        t.set_u32(w(&t, 0x399), gp);
        for (o, x) in [
            (0x5bu32, 0.75f32),
            (0x1aa, 1.0),
            (0x1b9, 0.5),
            (0x1c3, 2.0),
            (0x1cd, -0.5),
            (0x1da, 2.5),
            (0x22c, 4.0),
        ] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let (t1, t2) = (w(&t, 0x262), w(&t, 0x26b));
        let bl = SCRATCH + 0x900;
        let actors = w(&t, 0xbf);
        // (entries, kind, on the list)
        for (k, (n, kind, listed)) in [
            (0u32, 0u32, false),
            (1, 1, true),
            (3, 2, false),
            (2, 0, true),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            object_start_setup(&mut t, soa, 8);
            t.set_u32(w(&t, 0x1c), n);
            t.set_u32(w(&t, 0xb4), bl);
            t.write(bl, &[2, 1, 7, 0]);
            for i in 0..8u32 {
                t.set_u32(actors + 4 * i, 0x0ac7_0000 + i);
            }
            t.write(b, &varied(0x520 * 3 + 0x5e98, k + 1));
            t.write(b + 0x8520, &varied(0x10, k));
            t.set_u32(b + 0x8524, kind);
            // the last case's counts truncate to 0 (made 1)
            let ks = w(&t, 0x22c);
            set_f(&mut t, ks, if k == 3 { 0.01 } else { 4.0 });
            set_f(&mut t, t1 + 4 * kind, 0.5 + kind as f32);
            t.set_u32(t2 + 4 * kind, kind + 1);
            let obj = SCRATCH + 0x5000;
            t.write(obj, &varied(0x130, k + 2));
            t.set_u32(b + 0x8520, obj);
            let node = SCRATCH + 0x5400;
            t.write(node, &varied(0x10, k));
            if listed {
                t.set_u32(node, 0);
                t.set_u32(node + 4, 0);
                t.set_u32(node + 8, obj);
                t.set_u32(list, node);
                t.set_u32(list + 4, node);
                t.set_u32(list + 8, node);
            } else {
                t.set_u32(list, 0);
            }
            t.set_u32(list + 0xc, 1);
            task_ring(&mut t, f, (0x27c, 0x2b5, 0x31c, 0x334), k, n + k % 2);
            scrub(&mut t);
            let c = Call {
                eax: 0x2c20 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
}

/// Row 14: the beams with sprites in two of the DLLs: no entries, one
/// and three, the sprite kind (0) and two object kinds.
#[test]
fn beams_with_sprites() {
    for (dll, rva) in [(446u16, 0x2500u32), (555, 0x24d0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| op(t, f, o);
        log_calls(&mut t, 219, 2, &[], |_, a, k| a[0] ^ 0x0219_0000 ^ k);
        log_calls(&mut t, 237, 4, &[], |m, a, k| {
            for i in 0..3 {
                m.w32(a[3] + 4 * i, (1.5 * i as f32 - 2.0 + k as f32).to_bits());
            }
            0x237
        });
        log_calls(&mut t, 262, 4, &[], |m, a, k| {
            for (i, x) in (0u32..).zip(&a[1..4]) {
                let v = [3.0f32, -1.25, 4.5][i as usize] * (k as f32 + 1.0) - 2.0;
                m.w32(*x, v.to_bits());
            }
            0x262
        });
        log_calls(&mut t, 268, 3, &[0, 2], |_, _, _| 0x268);
        log_calls(&mut t, 257, 2, &[], |_, a, _| a[0] ^ 0x0257_0000);
        let soa = t.img.rel32_target(f + 0x2ad);
        let b = SCRATCH + 0x8000;
        t.set_u32(w(&t, 0x15), b);
        let gy = SCRATCH + 0x7e0;
        t.set_u32(w(&t, 0x25f), gy);
        t.set_u32(gy + 0x24, 0x0123_4567);
        for (o, x) in [
            (0x64u32, 0.75f32),
            (0x1ae, 1.0),
            (0x1bd, 0.5),
            (0x1c7, 2.0),
            (0x1d1, -0.5),
            (0x1de, 2.5),
            (0x22f, 0.25),
        ] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let (t1, t2) = (w(&t, 0x29a), w(&t, 0x2a6));
        let bl = SCRATCH + 0x900;
        let actors = w(&t, 0xbf);
        // (entries, kind)
        for (k, (n, kind)) in [(0u32, 1u32), (1, 0), (3, 2), (2, 0), (1, 1)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            object_start_setup(&mut t, soa, 8);
            t.set_u32(w(&t, 0x1d), n);
            t.set_u32(w(&t, 0xb4), bl);
            t.write(bl, &[2, 1, 7, 0]);
            for i in 0..8u32 {
                t.set_u32(actors + 4 * i, 0x0ac7_0000 + i);
            }
            t.write(b, &varied(0x5c10, k + 1));
            t.set_u32(b + 0x5c04, kind);
            set_f(&mut t, t1 + 4 * kind, 0.5 + kind as f32);
            t.set_u32(t2 + 4 * kind, kind + 1);
            task_ring(&mut t, f, (0x2db, 0x2f0, 0x357, 0x36d), k, n + k % 2);
            scrub(&mut t);
            let c = Call {
                eax: 0x2500 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
}

/// Row 15: the beams launched one by one in three of the DLLs: the wait
/// above 0 and at 0; the block's point or the scene's (`R[0x5208]`), the
/// last beam and one before it, the live list empty and not, the roll's
/// sign.
#[test]
fn beams_one_by_one() {
    for (dll, rva) in [(446u16, 0x3b90u32), (543, 0x3b60), (544, 0x3b30)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| op(t, f, o);
        log_calls(&mut t, 219, 2, &[], |_, a, k| a[0] ^ 0x0219_0000 ^ k);
        log_calls(&mut t, 237, 4, &[], |m, a, k| {
            for i in 0..3 {
                m.w32(a[3] + 4 * i, (1.5 * i as f32 - 2.0 + k as f32).to_bits());
            }
            0x237
        });
        log_calls(&mut t, 262, 4, &[], |m, a, k| {
            for (i, x) in (0u32..).zip(&a[1..4]) {
                let v = [3.0f32, -1.25, 4.5][i as usize] * (k as f32 + 1.0) - 2.0;
                m.w32(*x, v.to_bits());
            }
            0x262
        });
        log_calls(&mut t, 268, 3, &[0, 2], |_, _, _| 0x268);
        log_calls(&mut t, 314, 1, &[], |_, a, _| a[0]);
        t.stub_st0(314, |m, a| {
            let k = m.r32(LOG);
            f64::from(f32::from_bits(a[0])) * 0.375 - f64::from(k % 5)
        });
        let roll = SCRATCH + 0x7d0;
        log_calls(&mut t, 324, 0, &[], move |m, _, _| m.r32(roll));
        // the point turn's slots (`s4_small::turn_point`)
        log_calls(&mut t, 235, 1, &[], |_, _, _| 0x235);
        log_calls(&mut t, 328, 3, &[], |_, _, _| 0x328);
        log_calls(&mut t, 329, 3, &[], |_, _, _| 0x329);
        log_calls(&mut t, 229, 3, &[2], |m, a, _| {
            m.w32(a[0], 0x3f00_0000);
            m.w32(a[0] + 4, 0xbf00_0000);
            m.w32(a[0] + 8, 0x3e80_0000);
            0x229
        });
        log_calls(&mut t, 248, 3, &[1, 2], |m, a, _| {
            m.w32(a[0], 0x4000_0000);
            0x248
        });
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x1b), b);
        let take = t.img.rel32_target(f + 0x421);
        for (o, x) in [
            (0x163u32, 0.5f32),
            (0x35b, 1.0),
            (0x36a, 0.5),
            (0x374, 2.0),
            (0x37e, -0.5),
            (0x38b, 2.5),
            (0x3f6, 0.25),
        ] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let bl = SCRATCH + 0x900;
        let actors = w(&t, 0x1ce);
        let obj = SCRATCH + 0x5000;
        // (wait, scene point, beam index, beams, roll)
        for (k, (wait, scene, at, last, r)) in [
            (3i32, 0u32, 0u32, 2u32, 5i32),
            (0, 0, 0, 2, 5),
            (0, 1, 1, 2, -13),
            (0, 0, 1, 3, 0x7fff_ffff),
            (-1, 1, 2, 3, -8),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(b, &varied(0xec00, k + 1));
            let rr = b + 0x8540;
            t.set_u32(rr + 0x5200, wait.cast_unsigned());
            t.set_u32(rr + 0x5204, at);
            t.set_u32(rr + 0x5208, scene);
            t.set_u32(rr + 0x520c, last);
            t.write(b + 0x2f80, &floats(4, k));
            set_f(&mut t, b + 0xebf0, 0.25 * k as f32);
            t.set_u32(roll, r.cast_unsigned());
            t.set_u32(w(&t, 0x1bb), bl);
            t.write(bl, &[2 + k as u8, 1, 7, 0]);
            for i in 0..8u32 {
                t.set_u32(actors + 4 * i, 0x0ac7_0000 + i);
            }
            let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
            t.write(node(0), &varied(0x140, k + 3));
            let n = 1 + k % 2;
            for i in 1..=n {
                t.set_u32(node(i), node(if i == 1 { n } else { i - 1 }));
                t.set_u32(node(i) + 4, node(if i == n { 1 } else { i + 1 }));
            }
            t.set_u32(op(&t, take, 0x17), node(1));
            t.set_u16(op(&t, take, 0x04), n as u16);
            t.set_u32(w(&t, 0x427), if k.is_multiple_of(2) { 0 } else { node(4) });
            t.set_u32(w(&t, 0x43a), node(4));
            t.write(obj, &varied(0x60, k));
            scrub(&mut t);
            let c = Call {
                eax: 0x3b90 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[obj], Eax::Checked);
        }
    }
}

/// Rows 16, 62, 63: three beams spread from the actors' mean in seven of
/// the DLLs (four with slot 38 after the tail, two without the height): one to three actors, the object on its list and not, a count
/// that truncates to 0.
#[test]
fn beams_spread() {
    for (dll, rva) in [
        (446u16, 0x4480u32),
        (543, 0x4450),
        (555, 0x4440),
        (565, 0x65e0),
        (697, 0x65e0),
        (644, 0xa430),
        (692, 0xa430),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        // `magic_0644`'s copy has no height step: 0x20 bytes less from +0x9d
        let sh = if rva == 0xa430 { 0x20 } else { 0 };
        let w = move |t: &Twin, o: u32| op(t, f, if o >= 0x9d { o - sh } else { o });
        log_calls(&mut t, 219, 2, &[], |_, a, k| a[0] ^ 0x0219_0000 ^ k);
        log_calls(&mut t, 237, 4, &[], |m, a, k| {
            for i in 0..3 {
                m.w32(a[3] + 4 * i, (1.5 * i as f32 - 2.0 + k as f32).to_bits());
            }
            0x237
        });
        log_calls(&mut t, 262, 4, &[], |m, a, k| {
            for (i, x) in (0u32..).zip(&a[1..4]) {
                let v = [3.0f32, -1.25, 4.5][i as usize] * (k as f32 + 1.0) - 2.0;
                m.w32(*x, v.to_bits());
            }
            0x262
        });
        log_calls(&mut t, 12, 1, &[], |_, a, _| a[0]);
        t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.5 - 1.0);
        log_calls(&mut t, 314, 1, &[], |_, a, _| a[0]);
        t.stub_st0(314, |m, a| {
            f64::from(f32::from_bits(a[0])) * 0.375 - f64::from(m.r32(LOG) % 5)
        });
        log_calls(&mut t, 235, 1, &[], |m, a, k| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, (0.125 * i as f32 - k as f32).to_bits());
            }
            0x235
        });
        log_calls(&mut t, 328, 3, &[0], |_, _, _| 0x328);
        log_calls(&mut t, 329, 3, &[0], |_, _, _| 0x329);
        log_calls(&mut t, 229, 3, &[1, 2], |m, a, k| {
            for i in 0..3 {
                m.w32(a[0] + 4 * i, (0.5 * i as f32 + k as f32).to_bits());
            }
            0x229
        });
        log_calls(&mut t, 248, 3, &[1, 2], |m, a, _| {
            m.w32(a[0], 0x4000_0000);
            m.w32(a[0] + 8, 0xc000_0000);
            0x248
        });
        log_calls(&mut t, 713, 1, &[0], |_, _, _| 0x713);
        log_calls(&mut t, 257, 2, &[], |_, a, _| a[0] ^ 0x0257_0000);
        log_calls(&mut t, 38, 3, &[], |_, a, _| a[0] ^ 0x0038_0000);
        let soa = t.img.rel32_target(f + 0x374 - sh);
        let mean = t.img.rel32_target(f + 0x79);
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x29), b);
        let list = w(&t, 0x487);
        let gp = SCRATCH + 0x7e8;
        t.set_u32(w(&t, 0x495), gp);
        for (o, x) in [
            (0x2adu32, 1.0f32),
            (0x2bc, 0.5),
            (0x2c6, 2.0),
            (0x2d0, -0.5),
            (0x2dd, 2.5),
            (0x35c, 0.75),
        ] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let bl = SCRATCH + 0x900;
        let actors = w(&t, 0x88);
        // (actors, on the list, the count's divisor)
        for (k, (n, listed, kd)) in [(1u32, true, 0.25f32), (3, false, 0.5), (2, true, 1000.0)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            object_start_setup(&mut t, soa, 8);
            let kda = w(&t, 0x32d);
            set_f(&mut t, kda, kd);
            // the mean position's actors (`sb_misc::mean_position`)
            t.set_u32(op(&t, mean, 0x11), n);
            t.set_u32(op(&t, mean, 0x4d), bl);
            t.set_u32(w(&t, 0x7e), bl);
            t.write(bl, &[2 + k as u8, 1, 7, 0]);
            for i in 0..8u32 {
                t.set_u32(actors + 4 * i, 0x0ac7_0000 + i);
            }
            t.write(b, &varied(0xec00, k + 1));
            set_f(&mut t, b + 0xebf0, 0.5 - 0.25 * k as f32);
            // the scene global slot 219 reads (twice)
            let gc = w(&t, 0x32);
            t.set_u32(gc, 0x0c0c_0000 + k);
            let obj = b + w(&t, 0x46c);
            let node = SCRATCH + 0x5400;
            t.write(node, &varied(0x10, k));
            if listed {
                t.set_u32(node, 0);
                t.set_u32(node + 4, 0);
                t.set_u32(node + 8, obj);
                t.set_u32(list, node);
                t.set_u32(list + 4, node);
                t.set_u32(list + 8, node);
            } else {
                t.set_u32(list, 0);
            }
            t.set_u32(list + 0xc, 1);
            let ring = (0x37e - sh, 0x3b7 - sh, 0x41e - sh, 0x436 - sh);
            task_ring(&mut t, f, ring, k, 3 + k % 2);
            scrub(&mut t);
            let c = Call {
                eax: 0x4480 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, c, &[], Eax::Checked);
        }
    }
}

/// `mov eax, 1; ret` (`runtime::one_ret`) in five of the 50 DLLs that
/// carry it.
#[test]
fn one_ret() {
    for (dll, rva) in [
        (523u16, 0x16a0u32),
        (536, 0x14d0),
        (20, 0x1df0),
        (45, 0x1e80),
        (98, 0x23c0),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let c = Call {
            eax: 0x0eee_0000 + rva,
            ecx: 0x0ecc_0000,
            ..Call::default()
        };
        t.check_entry(f, c, &[0x0a00_0001, 0x0a00_0002], Eax::Checked);
    }
}

/// The trail draw's stubs (`magic_o6_fam.rs`' trail tests) for the draw
/// at `trail_at` (`flat`: `magic_0565`'s copy): its frame table (slot
/// 941), handle (772), the points' difference (234) and sum (248), the
/// sprite call's lock globals and the sprite slots hashing what they draw.
fn trail_stubs(t: &mut Twin, trail_at: u32, flat: bool) {
    // the trail draw (`magic_o6_fam.rs`' trail tests): its frame table
    // (slot 941), handle (772), the points' difference (234) and sum
    // (248), and the sprite call's lock globals
    for (s, n) in [(723u32, 1usize), (724, 1), (772, 2)] {
        log_calls(t, s, n, &[], move |_, a, _| s ^ a[0]);
    }
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    let table = SCRATCH + 0x3800;
    t.write(table, &varied(0x80, 17));
    t.set_u16(table + 6, 5);
    t.stub(941, move |_, _| table);
    t.stub(234, |m, a| {
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x234
    });
    t.stub(248, |m, a| {
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x248
    });
    let (call, lock_offs): (u32, &[u32]) = if flat {
        (0x531, &[0x4e, 0x79, 0xa8, 0xc5, 0x114, 0x13a])
    } else if op(t, trail_at, 0x344) as u8 == 0x40 {
        (0x546, &[0x4e, 0x79, 0xa8, 0xc5, 0x114, 0x13a])
    } else {
        (0x55d, &[0x4f, 0x7a, 0xa9, 0xc6, 0x115, 0x13b])
    };
    let link = t.img.rel32_target(trail_at + call);
    for &o in lock_offs {
        let lk = op(t, link, o);
        t.set_u32(lk, SCRATCH + 0x3f00);
    }
    // the sprite slots 774/778 hash the matrices `*G_M3`, `*G_M4` and
    // the colour (what the sprite is drawn with)
    let (m3_at, m4_at) = if flat { (0x61, 0x54) } else { (0x57, 0x63) };
    let (gm3, gm4, gc) = (
        op(t, trail_at, m3_at),
        op(t, trail_at, m4_at),
        op(t, trail_at, 0x236),
    );
    t.set_u32(gm3, SCRATCH + 0x3a00);
    t.set_u32(gm4, SCRATCH + 0x3a40);
    for s in [774u32, 778] {
        t.stub(s, move |m, a| {
            let mut x = s ^ a[2];
            for g in [gm3, gm4] {
                let at = m.r32(g);
                if at == 0 {
                    continue;
                }
                for i in 0..16 {
                    x = x.rotate_left(5) ^ m.r32(at + 4 * i);
                }
            }
            x ^= m.r32(gc).rotate_left(3);
            let k = m.r32(SCRATCH + 0x3fc0);
            m.w32(SCRATCH + 0x3fc4 + 4 * (k % 15), x);
            m.w32(SCRATCH + 0x3fc0, k + 1);
            x
        });
    }
}

/// Rows 22/118/43: the trail's head in six DLLs (the three trail draws):
/// the wait running with each kind (0, 2 and 3 move the point by their
/// sparks; 1 and 5 do not; `magic_0565`'s copies only with the kind 0's
/// sparks, whose set-up is shared), run out with the kind 0 and a count (the
/// kept point, the trail drawn with no sprites), and run out with a kind
/// or no count left (the object goes back to its pool). The count
/// is also the trail's number of sprites.
#[test]
fn trail_heads() {
    for (dll, rva) in [
        (446u16, 0x1e80u32),
        (555, 0x1e80),
        (544, 0x1e80),
        (546, 0x1e80),
        (565, 0x3a20),
        (697, 0x3a20),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let flat = rva == 0x3a20;
        let w = |t: &Twin, o: u32| op(t, f, o);
        log_calls(&mut t, 219, 2, &[], |_, a, k| a[0] ^ 0x0219_0000 ^ k);
        log_calls(&mut t, 237, 4, &[], |m, a, k| {
            for i in 0..3 {
                m.w32(a[3] + 4 * i, (1.5 * i as f32 - 2.0 + k as f32).to_bits());
            }
            0x237
        });
        log_calls(&mut t, 235, 1, &[], |m, a, k| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, (0.125 * i as f32 + k as f32).to_bits());
            }
            0x235
        });
        // the sparks' slots and pools (`magic_s4_misc.rs`' sparks tests)
        log_calls(&mut t, 719, 3, &[], |_, a, _| 0x7190_0000 ^ a[1]);
        log_calls(&mut t, 941, 1, &[], |_, a, _| 0x9410_0000 ^ a[0]);
        for s in [314u32, 404] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| s ^ a[0]);
            t.stub_st0(s, move |_, a| {
                f64::from(f32::from_bits(a[0])) * if s == 404 { 0.75 } else { 0.5 }
            });
        }
        let recs = SCRATCH + 0x4000;
        let (gf, r2) = (SCRATCH + 0x800, SCRATCH + 0x880);
        let mut takes = Vec::new();
        for (call, take_at, gf_at, r2_at) in [
            (0xfeu32, 0x9u32, 0x150u32, &[0x190u32][..]),
            (0xf4, 0x13, 0x14c, &[0x5, 0x18f][..]),
            (0xea, 0x17, 0x5, &[0x27, 0x18c][..]),
        ]
        .into_iter()
        .take(if flat { 1 } else { 3 })
        {
            let sp = t.img.rel32_target(f + call);
            let take = t.img.rel32_target(sp + take_at);
            t.set_u32(op(&t, take, 0x37), recs);
            t.set_u32(op(&t, take, 0x61), recs);
            t.set_u32(op(&t, sp, gf_at), gf);
            for &o in r2_at {
                t.set_u32(op(&t, sp, o), r2);
            }
            takes.push(take);
        }
        for i in 0..8 {
            t.set_u32(r2 + 4 * i, 0x0e00_0000 + i);
        }
        let trail_at = t.img.rel32_target(f + 0x173);
        trail_stubs(&mut t, trail_at, flat);
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x14), b);
        let obj = SCRATCH + 0x5000;
        // (wait, kind, count)
        for (k, (wait, kind, count)) in [
            (3i32, 0u32, 1i32),
            (2, 2, 0),
            (1, 3, 0),
            (4, 1, 0),
            (2, 5, 0),
            (0, 0, 2),
            (-1, 0, 1),
            (0, 0, 0),
            (0, 2, 3),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            if flat && (kind == 2 || kind == 3) && wait > 0 {
                continue;
            }
            t.set_u32(LOG, 0);
            t.write(b, &varied(0x5c10, k + 1));
            t.set_u32(b + 0x5c00, wait.cast_unsigned());
            t.set_u32(b + 0x5c04, kind);
            t.set_u32(b + 0x5bd8, count.cast_unsigned());
            t.write(b + 0x30a0, &floats(4, k));
            // the trail (its sprite count is the head's count `B[0x5bd8]`)
            let o = b + 0x59b0;
            t.set_u32(o, 0x0041_0000 + k);
            t.set_u32(o + 4, k % 2);
            t.set_u16(o + 8, 2);
            t.set_u16(o + 0xa, 0x100);
            for i in 0..32u32 {
                let x = i as f32;
                let xyz = [x * 1.5 + k as f32, x * 0.5 - 2.0, x * 0.25 - 3.0, 1.0];
                for (j, v) in (0u32..).zip(xyz) {
                    t.set_u32(o + 0xc + 16 * i + 4 * j, v.to_bits());
                }
                t.set_u8(o + 0x22c + i, 1);
            }
            t.write(o + 0x20c, &[0x20 + k as u8, 0x90, 0x10, 0x80]);
            t.write(o + 0x210, &[0xe0, 0x30, 0x70, 0x60]);
            t.set_u32(o + 0x214, 1.5f32.to_bits());
            t.set_u32(o + 0x218, 0.5f32.to_bits());
            t.set_u32(o + 0x21c, 0.6f32.to_bits());
            t.set_u32(o + 0x220, 0x1234 + 0x111 * k);
            t.set_u32(o + 0x224, 5);
            t.set_u32(b + 0x5bd8, count.cast_unsigned());
            t.write(SCRATCH + 0x3f00, &[0x31; 0x40]);
            t.set_u8(gf, 0);
            for &take in &takes {
                t.set_u32(op(&t, take, 0x08), 0);
                t.set_u32(op(&t, take, 0x0f), 8);
                t.set_u32(op(&t, take, 0x97), 8);
                t.set_u32(op(&t, take, 0x2b), 0);
            }
            for i in 0..8 {
                t.write(recs + 0x74 * i, &[0x22; 0x74]);
                t.set_u32(recs + 0x74 * i + 0x6c, 0);
            }
            // the object given back: no siblings, no children
            t.write(obj, &varied(0x20, k));
            t.set_u32(obj, 0);
            t.set_u32(obj + 4, 0);
            t.set_u32(obj + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            let c = w(&t, 0x36);
            t.set_u32(c, 0x0c0c_0000 + k);
            scrub(&mut t);
            let call = Call {
                eax: 0x1e80 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[obj], Eax::Checked);
        }
    }
}

/// Rows 35/36: the light flown in, in four DLLs (both copies): the count
/// above 1 (still flying), at 1 and at 0 (the child started, the object
/// on its list and not, then given back).
#[test]
fn lights_flown_in() {
    for (dll, rva) in [
        (565u16, 0x4750u32),
        (691, 0x4750),
        (644, 0x8770),
        (698, 0x8770),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let light = t.img.rel32_target(f + 0xde);
        light_slots(&mut t, light);
        // the child's start (`s4_fam446::start_child_sound_b`)
        let child = t.img.rel32_target(f + 0x12e);
        let soa = t.img.rel32_target(child + 0x2e);
        log_calls(&mut t, 719, 3, &[], |_, a, _| 0x7190_0000 ^ a[2]);
        log_calls(&mut t, 393, 3, &[1], |_, _, _| 0x393);
        log_calls(&mut t, 306, 3, &[], |_, a, _| 0x0306_0000 ^ a[2]);
        log_calls(&mut t, 713, 1, &[0], |_, _, _| 0x713);
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x21), b);
        let k1 = w(&t, 0x4a);
        set_f(&mut t, k1, 1.0);
        let list = w(&t, 0x138);
        let gp = SCRATCH + 0x7e8;
        t.set_u32(w(&t, 0x146), gp);
        let (obj, s) = (SCRATCH + 0x5000, SCRATCH + 0x8000);
        // (count, total, on the list)
        for (k, (count, total, listed)) in [
            (5i32, 8i32, false),
            (1, 8, true),
            (0, 4, false),
            (-2, 3, true),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            object_start_setup(&mut t, soa, 8);
            t.write(b, &varied(0x8540, k + 1));
            let kind_at = b + w(&t, 0x121);
            t.set_u32(kind_at, k % 3);
            t.write(s, &floats(0x150, k));
            t.set_u32(s + 0x50c, count.cast_unsigned());
            t.set_u32(s + 0x510, total.cast_unsigned());
            t.set_u32(s + 0x518, k % 2);
            t.write(obj, &varied(0x20, k));
            t.set_u32(obj, 0);
            t.set_u32(obj + 4, 0);
            t.set_u32(obj + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            t.set_u32(obj + 0x14, s);
            let node = SCRATCH + 0x5400;
            t.write(node, &varied(0x10, k));
            if listed {
                t.set_u32(node, 0);
                t.set_u32(node + 4, 0);
                t.set_u32(node + 8, s);
                t.set_u32(list, node);
                t.set_u32(list + 4, node);
                t.set_u32(list + 8, node);
            } else {
                t.set_u32(list, 0);
            }
            t.set_u32(list + 0xc, 1);
            scrub(&mut t);
            let call = Call {
                eax: 0x4750 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[obj], Eax::Checked);
        }
    }
}

/// The light set-up's slots as `magic_o6_fam.rs`' light test answers
/// them, slot 230 logging the matrix it copies (its last row the point).
fn light_slots(t: &mut Twin, light: u32) {
    let h = SCRATCH + 0x3000;
    log_calls(t, 219, 2, &[], move |_, a, _| h ^ (a[1] & 0x10));
    for s in [277u32, 15, 16, 17] {
        log_calls(t, s, 1, &[], move |_, a, _| s ^ a[0]);
        t.stub_st0(s, move |_, a| {
            f64::from(a[0] & 0xff) * 0.125 + f64::from(s) * 0.5
        });
    }
    for s in [235u32, 723, 724] {
        log_calls(t, s, 1, &[], move |m, a, _| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, (0.25 * (i + s) as f32).to_bits());
            }
            s
        });
    }
    log_calls(t, 545, 0, &[], |_, _, _| 0x0054_5000);
    t.set_u32(h + 0x830, 0x0830_0001);
    t.stub(230, |m, a| {
        let k = m.r32(LOG);
        m.w32(LOG, k + 1);
        let row = LOG + 4 + 0x40 * (k % ROWS);
        m.w32(row, 230);
        for i in 0..12 {
            m.w32(row + 4 + 4 * i, m.r32(a[1] + 0x10 + 4 * i));
        }
        0x230
    });
    t.set_u32(op(t, light, 0x36), 0x0021_9000);
}

/// Rows 47/48: the light flown along its arc in four DLLs (both copies):
/// the count passing 8 (the actor block filled, in and out of a battle),
/// reaching 0 (slots 219/257), running, and run out (the wait down, slot
/// 268, the object given back once the wait is 0 too).
#[test]
fn lights_on_arcs() {
    for (dll, rva) in [
        (565u16, 0x5220u32),
        (693, 0x5220),
        (644, 0x9060),
        (696, 0x9060),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let light = t.img.rel32_target(f + 0x215);
        light_slots(&mut t, light);
        for s in [215u32, 413] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| (s << 16) ^ a[0]);
        }
        log_calls(&mut t, 257, 2, &[], |_, a, _| a[0] ^ 0x0257_0000);
        log_calls(&mut t, 268, 3, &[2], |_, _, _| 0x268);
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x21), b);
        for (o, x) in [(0x110u32, 1.0f32), (0x146, 4.0), (0x1ab, 2.5)] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let (obj, s) = (SCRATCH + 0x5000, SCRATCH + 0x8000);
        let wait = w(&t, 0xbd);
        // (count, total, in a battle, the wait)
        for (k, (count, total, battle, wt)) in [
            (9i32, 12i32, 0u32, 3i32),
            (9, 12, 1, 3),
            (1, 4, 0, 2),
            (5, 8, 1, 2),
            (0, 4, 0, 2),
            (0, 4, 0, 1),
            (-1, 3, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(b, &varied(0xd800, k + 1));
            t.set_u32(b + wait, wt.cast_unsigned());
            t.set_u32(w(&t, 0x3f), battle);
            let x = w(&t, 0x45);
            t.set_u32(x, 0x0abc_0000 + k);
            let y = w(&t, 0x1e7);
            t.set_u32(y, 0x0def_0000 + k);
            t.write(s, &floats(0x150, k));
            t.set_u32(s + 0x50c, count.cast_unsigned());
            t.set_u32(s + 0x510, total.cast_unsigned());
            t.write(obj, &varied(0x20, k));
            t.set_u32(obj, 0);
            t.set_u32(obj + 4, 0);
            t.set_u32(obj + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            t.set_u32(obj + 0x14, s);
            scrub(&mut t);
            let call = Call {
                eax: 0x5220 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[obj], Eax::Checked);
        }
    }
}

/// Row 30: the light drifting in two DLLs: the count far above 20, below
/// it, at 1 and run out (the object given back); the angles past both
/// bounds.
#[test]
fn lights_drifting() {
    for dll in [644u16, 694] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x8f20);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let light = t.img.rel32_target(f + 0x117);
        light_slots(&mut t, light);
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x14), b);
        for (o, x) in [(0x2fu32, 0.25f32), (0x3d, 3.0), (0x45, 6.0), (0x4d, -3.0)] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let (obj, s) = (SCRATCH + 0x5000, SCRATCH + 0x8000);
        // (count, the angles, their speeds)
        for (k, (count, angles, speeds)) in [
            (30i32, (2.5f32, -2.5f32), (1.0f32, -1.0f32)),
            (12, (0.5, 1.0), (0.25, -0.5)),
            (1, (-2.75, 2.75), (-0.5, 0.5)),
            (0, (0.0, 0.0), (0.125, 0.25)),
            (-3, (1.0, -1.0), (3.0, -3.0)),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(b, &varied(0xd700, k + 1));
            t.write(s, &floats(0x150, k));
            t.set_u32(s + 0x50c, count.cast_unsigned());
            set_f(&mut t, s + 0x280, angles.0);
            set_f(&mut t, s + 0x284, angles.1);
            set_f(&mut t, s + 0x290, speeds.0);
            set_f(&mut t, s + 0x298, speeds.1);
            t.write(obj, &varied(0x20, k));
            t.set_u32(obj, 0);
            t.set_u32(obj + 4, 0);
            t.set_u32(obj + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            t.set_u32(obj + 0x14, s);
            scrub(&mut t);
            let call = Call {
                eax: 0x8f20 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[obj], Eax::Checked);
        }
    }
}

/// Rows 51/52: the light landing in four DLLs (both copies): the count
/// still running, and run out with and without the sounds (`S[0x514]`)
/// and with no, one and three battle numbers, the object on its list and
/// not.
#[test]
fn lights_landing() {
    for (dll, rva) in [
        (644u16, 0xa180u32),
        (692, 0xa180),
        (565, 0x6330),
        (695, 0x6330),
    ] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let sh = if rva == 0x6330 { 0xb } else { 0 };
        let light = t.img.rel32_target(f + 0xdb);
        light_slots(&mut t, light);
        let soa = t.img.rel32_target(f + 0x1f0 + sh);
        log_calls(&mut t, 239, 3, &[], |_, a, _| a[1] ^ 0x0239_0000);
        log_calls(&mut t, 719, 3, &[], |_, a, _| 0x7190_0000 ^ a[2]);
        log_calls(&mut t, 306, 3, &[], |_, a, _| 0x0306_0000 ^ a[2]);
        log_calls(&mut t, 393, 3, &[1], |_, _, _| 0x393);
        log_calls(&mut t, 713, 1, &[0], |_, _, _| 0x713);
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x16), b);
        let k1 = w(&t, 0x4d);
        set_f(&mut t, k1, 1.0);
        let list = w(&t, 0x240 + sh);
        let gp = SCRATCH + 0x7e8;
        t.set_u32(w(&t, 0x25c + sh), gp);
        let nums = SCRATCH + 0x7a0;
        t.set_u32(w(&t, 0x155), nums);
        t.write(nums, &[3, 1, 4, 1, 5, 9, 2, 6]);
        let (obj, s) = (SCRATCH + 0x5000, SCRATCH + 0x8000);
        let l = SCRATCH + 0x7c0;
        // (count, no sounds, battle numbers, on the list)
        for (k, (count, quiet, n, listed)) in [
            (4i32, 0u32, 1u32, false),
            (1, 0, 3, true),
            (0, 1, 2, false),
            (0, 0, 0, true),
            (-1, 0, 1, false),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            object_start_setup(&mut t, soa, 8);
            t.write(b, &varied(0xed00, k + 1));
            let d1 = w(&t, 0x12f);
            t.set_u32(b + d1, l);
            t.write(l, &[0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 5, 1]);
            t.set_u32(l + 4, n);
            let c = w(&t, 0x147);
            t.set_u32(c, 0x0c0c_0000 + k);
            t.write(s, &floats(0x150, k));
            t.set_u32(s + 0x50c, count.cast_unsigned());
            t.set_u32(s + 0x510, 6);
            t.set_u32(s + 0x514, quiet);
            t.write(obj, &varied(0x20, k));
            t.set_u32(obj, 0);
            t.set_u32(obj + 4, 0);
            t.set_u32(obj + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            t.set_u32(obj + 0x14, s);
            let node = SCRATCH + 0x5400;
            t.write(node, &varied(0x10, k));
            if listed {
                t.set_u32(node, 0);
                t.set_u32(node + 4, 0);
                t.set_u32(node + 8, s);
                t.set_u32(list, node);
                t.set_u32(list + 4, node);
                t.set_u32(list + 8, node);
            } else {
                t.set_u32(list, 0);
            }
            t.set_u32(list + 0xc, 1);
            scrub(&mut t);
            let call = Call {
                eax: 0xa180 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[obj], Eax::Checked);
        }
    }
}

/// Row 55: the light bouncing in two DLLs: in the air, on the floor with
/// each wall side and speed sign (the axes stopping or not), the count
/// above 20, below it and run out.
#[test]
fn lights_bouncing() {
    for dll in [565u16, 697] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x4f00);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let light = t.img.rel32_target(f + 0x2f1);
        light_slots(&mut t, light);
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0xe), b);
        for (o, x) in [
            (0x5bu32, 0.5f32),
            (0xad, 0.25),
            (0xd6, -0.5),
            (0xe6, 0.75),
            (0xee, 0.75),
            (0x267, 3.0),
            (0x26f, 6.0),
            (0x277, -3.0),
        ] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let (obj, s) = (SCRATCH + 0x5000, SCRATCH + 0x8000);
        // (count, height, speed, walls)
        for (k, (count, y, v, walls)) in [
            (30i32, -5.0f32, [1.0f32, 0.5, -1.0], [1.0f32, -1.0]),
            (12, 2.0, [1.0, 0.5, -1.0], [1.0, -1.0]),
            (12, 2.0, [-1.0, -0.5, 1.0], [-1.0, 1.0]),
            (5, 3.0, [-2.0, 1.5, 2.0], [1.0, -1.0]),
            (1, 2.5, [2.0, -3.0, -2.0], [-1.0, 1.0]),
            (0, 4.0, [0.0, 1.0, 0.0], [0.0, 0.0]),
            // rising into the floor: `y` stops
            (3, 3.0, [1.0, -0.75, 1.0], [1.0, 1.0]),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.write(b, &varied(0xd800, k + 1));
            let d = w(&t, 0xa5);
            set_f(&mut t, b + d, 1.0);
            t.write(s, &floats(0x150, k));
            t.set_u32(s + 0x50c, count.cast_unsigned());
            set_f(&mut t, s + 0x264, y);
            for (i, x) in (0u32..).zip(v) {
                set_f(&mut t, s + 0x2a0 + 4 * i, x);
            }
            set_f(&mut t, s + 0x2b0, walls[0]);
            set_f(&mut t, s + 0x2b8, walls[1]);
            set_f(&mut t, s + 0x27c, 2.0);
            t.write(obj, &varied(0x20, k));
            t.set_u32(obj, 0);
            t.set_u32(obj + 4, 0);
            t.set_u32(obj + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            t.set_u32(obj + 0x14, s);
            scrub(&mut t);
            let call = Call {
                eax: 0x4f00 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[obj], Eax::Checked);
        }
    }
}

/// Row 181: the texture fade in both DLLs: held back by the clock, the
/// fade dropping or not (slot 253), below 0, above the top, inside, and
/// ending exactly at `K3`.
#[test]
fn texture_fades() {
    for dll in [188u16, 690] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x1db0);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let down = SCRATCH + 0x7d0;
        log_calls(&mut t, 253, 0, &[], move |m, _, _| m.r32(down));
        log_calls(&mut t, 38, 3, &[], |_, a, _| a[1] ^ 0x0038_0000);
        // the scrolled texture's handle (`magic_o6_fam.rs`' test)
        let tex = SCRATCH + 0x3000;
        t.set_u32(tex, 0x4c0c_1234);
        t.set_u32(tex + 4, 1);
        log_calls(&mut t, 719, 3, &[], move |_, _, _| tex);
        let scroll = t.img.rel32_target(f + 0x81);
        t.set_u32(op(&t, scroll, 0x3b), 0x0071_9000);
        t.set_u32(op(&t, scroll, 0x10f), SCRATCH + 0x3040);
        t.set_u32(op(&t, scroll, 0xb9), SCRATCH + 0x3100);
        for (o, x) in [(0x36u32, 0.25f32), (0x4e, 1.0), (0x90, 0.5)] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        // (`*G_A`, clock, slot 253, fade)
        for (k, (a, clock, d, v)) in [
            (1u32, 0x98u32, 0u32, 0.5f32),
            (1, 0x99, 1, 0.75),
            (0, 0, 1, 0.125),
            (0, 0, 0, 1.5),
            (1, 0x200, 0, 0.75),
            (0, 0, 0, -0.5),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            t.set_u32(w(&t, 0x2), a);
            t.set_u32(w(&t, 0xb), clock);
            t.set_u32(down, d);
            let fade = w(&t, 0x2a);
            set_f(&mut t, fade, v);
            let gw = w(&t, 0x77);
            t.set_u32(gw, 0x0abc_0000 + k);
            t.set_u32(w(&t, 0x9c), 0x5a5a);
            t.write(SCRATCH + 0x3040, &varied(0x100, k));
            scrub(&mut t);
            let call = Call {
                eax: 0x1db0 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[], Eax::Checked);
        }
    }
}

/// The DLL's free node ring for `sa_pool::take` at `take`: `n` nodes at
/// `at` (`+0` the previous, `+4` the next).
fn take_ring(t: &mut Twin, take: u32, at: u32, n: u32, k: u32) {
    let node = |i: u32| at + 0x40 * i;
    t.write(at, &varied(0x40 * n, k + 3));
    for i in 0..n {
        t.set_u32(node(i), node((i + n - 1) % n));
        t.set_u32(node(i) + 4, node((i + 1) % n));
    }
    t.set_u32(op(t, take, 0x17), node(0));
    t.set_u16(op(t, take, 0x04), n as u16);
}

/// Rows 27/28: the summon's pair start in three DLLs, directly and
/// through its start task (busy, the first call, ready): the busy list
/// empty and not, the yaws past both bounds and inside.
#[test]
fn summon_pairs_started() {
    for dll in [242u16, 651, 657] {
        let Some(mut t) = twin(dll) else { return };
        let task = t.at(0x3550);
        let f = t.at(0x3680);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let soa = t.img.rel32_target(f + 0x7a);
        let take = t.img.rel32_target(f + 0x2ed);
        let mean = t.img.rel32_target(f + 0x166);
        let pi = t.img.rel32_target(f + 0x4f);
        t.set_u32(op(&t, pi, 0x01), SCRATCH + 0x7e0);
        let ans = SCRATCH + 0x7d0;
        log_calls(&mut t, 225, 1, &[], move |m, _, _| m.r32(ans));
        log_calls(&mut t, 226, 2, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 227, 1, &[], move |m, _, _| m.r32(ans + 4));
        log_calls(&mut t, 719, 3, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 711, 3, &[], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 710, 4, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 262, 4, &[], |m, a, k| {
            for (i, x) in (1u32..).zip(&a[1..4]) {
                m.w32(*x, (0.5 * i as f32 + k as f32).to_bits());
            }
            0x262
        });
        for s in [215u32, 413] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| (s << 16) ^ a[0]);
        }
        let yaw = SCRATCH + 0x7d8;
        log_calls(&mut t, 333, 1, &[], |_, a, _| a[0]);
        t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
        for (o, x) in [(0xc4u32, 1.0f32), (0xdb, 3.0), (0xd1, 6.0), (0xeb, -3.0)] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        t.set_u32(w(&t, 0x55), 1);
        // the camera actor `*G_A` (slots 262 and 333)
        let ga = w(&t, 0x9a);
        t.set_u32(ga, 0x0ca0_0001);
        let owners = w(&t, 0x64);
        t.set_u32(owners + 4, 0x30);
        let bl = SCRATCH + 0x900;
        let actors = op(&t, mean, 0x58);
        t.set_u32(op(&t, mean, 0x11), 2);
        t.set_u32(op(&t, mean, 0x4d), bl);
        t.set_u32(w(&t, 0x189), bl);
        t.write(bl, &[2, 1, 7, 0]);
        for i in 0..8u32 {
            t.set_u32(actors + 4 * i, 0x0ac7_0000 + i);
        }
        // (the yaw slot 333 answers, the busy list empty, load: busy / ready)
        for (k, (y, empty, busy, ready)) in [
            (0.5f32, true, 0u32, 1u32),
            (-4.0, false, 0, 2),
            (5.0, true, 1, 1),
            (0.25, false, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            for direct in [true, false] {
                t.set_u32(LOG, 0);
                set_f(&mut t, yaw, y);
                t.set_u32(ans, if direct { 0 } else { busy });
                t.set_u32(ans + 4, ready);
                object_start_setup(&mut t, soa, 8);
                take_ring(&mut t, take, SCRATCH + 0x1000, 5, k);
                let first = w(&t, 0x2f3);
                t.set_u32(first, if empty { 0 } else { SCRATCH + 0x1800 });
                t.set_u32(w(&t, 0x30a), SCRATCH + 0x1840);
                t.write(SCRATCH + 0x1800, &varied(0x80, k));
                scrub(&mut t);
                let call = Call {
                    eax: 0x3680 + k,
                    ecx: 0x0ecc_0000 + k,
                    ..Call::default()
                };
                t.check_entry(if direct { f } else { task }, call, &[], Eax::Checked);
            }
        }
    }
}

/// Rows 25/26: the summon's actor pair start in three DLLs, directly and
/// through its start task: the actors' points from slot 262 (one and
/// two actors answering 0x1066 to slot 390) or their mean (none), in and
/// out of a battle, the busy list empty and not, the yaws past both
/// bounds and inside.
#[test]
fn summon_actor_pairs_started() {
    for dll in [241u16, 650, 656] {
        let Some(mut t) = twin(dll) else { return };
        let task = t.at(0x1210);
        let f = t.at(0x1340);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let soa = t.img.rel32_target(f + 0x73);
        let mean = t.img.rel32_target(f + 0x1ea);
        let pi = t.img.rel32_target(f + 0x46);
        t.set_u32(op(&t, pi, 0x01), SCRATCH + 0x7e0);
        let ans = SCRATCH + 0x7d0;
        log_calls(&mut t, 225, 1, &[], move |m, _, _| m.r32(ans));
        log_calls(&mut t, 226, 2, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 227, 1, &[], move |m, _, _| m.r32(ans + 4));
        log_calls(&mut t, 719, 3, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 711, 3, &[], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 710, 4, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 262, 4, &[], |m, a, k| {
            for (i, x) in (1u32..).zip(&a[1..4]) {
                m.w32(*x, (0.5 * i as f32 + k as f32).to_bits());
            }
            0x262
        });
        for s in [215u32, 413] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| (s << 16) ^ a[0]);
        }
        let status = SCRATCH + 0x7c0;
        log_calls(&mut t, 390, 1, &[], move |m, a, _| {
            m.r32(status + 4 * (a[0] & 3))
        });
        let yaw = SCRATCH + 0x7d8;
        log_calls(&mut t, 333, 1, &[], |_, a, _| a[0]);
        t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
        log_calls(&mut t, 12, 1, &[], |_, a, _| a[0]);
        t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.5 - 0.75);
        for (o, x) in [
            (0xbdu32, 1.0f32),
            (0xd4, 3.0),
            (0xca, 6.0),
            (0xe4, -3.0),
            (0x2ad, 0.75),
        ] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        t.set_u32(w(&t, 0x4b), 1);
        let owners = w(&t, 0x5a);
        t.set_u32(owners + 4, 0x30);
        let ga = w(&t, 0x93);
        t.set_u32(ga, 0x0ca0_0001);
        let bl = SCRATCH + 0x900;
        let actors = op(&t, mean, 0x58);
        t.set_u32(op(&t, mean, 0x11), 2);
        t.set_u32(op(&t, mean, 0x4d), bl);
        t.set_u32(w(&t, 0x174), bl);
        t.write(bl, &[2, 1, 7, 0]);
        for i in 0..8u32 {
            t.set_u32(actors + 4 * i, 0x0ac7_0000 + i);
        }
        let ring = |t: &mut Twin, k: u32| {
            let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
            t.write(node(0), &varied(0x140, k + 3));
            // two nodes every other case: the second take empties the ring
            let n = if k.is_multiple_of(2) { 4 } else { 2 };
            for i in 0..n {
                t.set_u32(node(i), node((i + n - 1) % n));
                t.set_u32(node(i) + 4, node((i + 1) % n));
            }
            t.set_u16(op(t, f, 0x3f9), n as u16);
            t.set_u32(op(t, f, 0x413), node(0));
        };
        // (yaw, actors' answers, battle, busy list empty, load busy / ready)
        for (k, (y, answers, battle, empty, busy, ready)) in [
            (0.5f32, [0x1066u32, 0, 0x1066], 0u32, true, 0u32, 1u32),
            (-4.0, [0, 0, 0], 1, false, 0, 2),
            (5.0, [0, 0x1066, 0], 0, true, 1, 1),
            (0.25, [0x1066, 0x1066, 0], 1, false, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            for direct in [true, false] {
                t.set_u32(LOG, 0);
                set_f(&mut t, yaw, y);
                for (i, a) in (0u32..).zip(answers) {
                    t.set_u32(status + 4 * i, a);
                }
                t.set_u32(op(&t, f, 0x161), 3);
                t.set_u32(w(&t, 0x2ba), battle);
                // the record words the start clears, seeded
                for o in [0x2c1u32, 0x2c6, 0x3d5, 0x3cb] {
                    let g = w(&t, o);
                    t.set_u32(g, 0x5a5a_0000 + o);
                }
                t.set_u32(ans, if direct { 0 } else { busy });
                t.set_u32(ans + 4, ready);
                object_start_setup(&mut t, soa, 8);
                ring(&mut t, k);
                let first = w(&t, 0x46e);
                t.set_u32(first, if empty { 0 } else { SCRATCH + 0x1800 });
                t.set_u32(w(&t, 0x47e), SCRATCH + 0x1840);
                t.write(SCRATCH + 0x1800, &varied(0x80, k));
                scrub(&mut t);
                let call = Call {
                    eax: 0x1340 + k,
                    ecx: 0x0ecc_0000 + k,
                    ..Call::default()
                };
                t.check_entry(if direct { f } else { task }, call, &[], Eax::Checked);
            }
        }
    }
}

/// Rows 24/221: the object carried and launched in three DLLs (the
/// part's matrix and the weapon's): the count running, reaching 0 (the
/// launch with `z` inside and outside its window, the trail record on
/// and off, the busy list empty and not).
#[test]
fn launches_after_carry() {
    for dll in [544u16, 557, 555] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x3470);
        let w = |t: &Twin, o: u32| op(t, f, o);
        // the matrix's slots (`sa_misc::part_matrix`, `s5_misc::weapon_matrix`)
        let h = SCRATCH + 0x3000;
        log_calls(&mut t, 219, 2, &[], move |_, a, _| h ^ (a[1] & 0x10));
        for s in [277u32, 15, 16, 17] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| s ^ a[0]);
            t.stub_st0(s, move |_, a| {
                f64::from(a[0] & 0xff) * 0.125 + f64::from(s) * 0.5
            });
        }
        log_calls(&mut t, 235, 1, &[], |m, a, k| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, (0.25 * i as f32 - k as f32).to_bits());
            }
            0x235
        });
        for s in [328u32, 329, 330] {
            log_calls(&mut t, s, 3, &[0], move |_, _, _| s);
        }
        log_calls(&mut t, 229, 3, &[1, 2], |m, a, k| {
            for i in 0..4 {
                m.w32(a[0] + 4 * i, (0.5 * i as f32 + k as f32 - 1.0).to_bits());
            }
            0x229
        });
        log_calls(&mut t, 248, 3, &[1, 2], |m, a, k| {
            for i in 0..4 {
                m.w32(a[0] + 4 * i, (1.5 - 0.25 * i as f32 + k as f32).to_bits());
            }
            0x248
        });
        let r = SCRATCH + 0x7c0;
        for s in [314u32, 404] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| s ^ a[0]);
            t.stub_st0(s, move |m, a| {
                f64::from(f32::from_bits(a[0])) + f64::from(f32::from_bits(m.r32(r)))
            });
        }
        // the child start's (`s4_fam446::start_child_c`)
        let child = t.img.rel32_target(f + 0xf8);
        let soa = t.img.rel32_target(child + 0x3e);
        log_calls(&mut t, 641, 1, &[], |_, a, _| a[0] ^ 0x0641_0000);
        log_calls(&mut t, 719, 3, &[], |_, a, _| 0x7190_0000 ^ a[2]);
        log_calls(&mut t, 393, 3, &[1], |_, _, _| 0x393);
        log_calls(&mut t, 306, 3, &[], |_, a, _| 0x0306_0000 ^ a[2]);
        let take = t.img.rel32_target(f + 0x5ca);
        let b = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x1f), b);
        for (o, x) in [
            (0x5bu32, 1.0f32),
            (0x197, 0.5),
            (0x1ca, 2.0),
            (0x1d2, 4.0),
            (0x1e5, 1.0),
            (0x1f2, 0.75),
            (0x34c, 1.0),
            (0x357, 3.0),
            (0x367, 6.0),
            (0x371, -3.0),
            (0x37e, 6.0),
            (0x50e, 0.5),
            (0x516, 2.0),
            (0x53d, 0.25),
            (0x556, 1.5),
        ] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let (obj, x, next) = (SCRATCH + 0x5000, SCRATCH + 0x8000, SCRATCH + 0x5800);
        // (count, slot 314's offset, the trail record, busy list empty)
        for (k, (count, ro, trail, empty)) in [
            (5i32, 0.0f32, 1u32, true),
            (1, 0.5, 1, true),
            (0, 3.5, 0, false),
            (0, 2.75, 1, false),
            (-1, -6.0, 1, true),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(LOG, 0);
            set_f(&mut t, r, ro);
            object_start_setup(&mut t, soa, 8);
            t.write(b, &varied(0xd800, k + 1));
            let d = w(&t, 0x56e);
            t.set_u32(b + d, trail);
            t.write(x, &floats(0x150, k));
            t.set_u32(x + 0x50c, count.cast_unsigned());
            t.set_u32(x + 0x510, 6);
            t.set_u32(x + 0x518, k % 2);
            t.write(obj, &varied(0x20, k));
            t.set_u32(obj + 8, next);
            t.write(next, &varied(0x20, k + 2));
            t.set_u32(obj + 0x14, x);
            take_ring(&mut t, take, SCRATCH + 0x1000, 3, k);
            let first = w(&t, 0x5d0);
            t.set_u32(first, if empty { 0 } else { SCRATCH + 0x1800 });
            t.set_u32(w(&t, 0x5e3), SCRATCH + 0x1840);
            t.write(SCRATCH + 0x1800, &varied(0x80, k));
            scrub(&mut t);
            let call = Call {
                eax: 0x3470 + k,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            };
            t.check_entry(f, call, &[obj], Eax::Checked);
        }
    }
}

/// Rows 163/164: the camera aimed in both DLLs, directly and through the
/// camera's frame: both ids in the first list (the direction to `G_V`),
/// both in the second (the look matrix), one in each and neither (the
/// two points of slot 217).
#[test]
fn cameras_aimed() {
    for dll in [188u16, 690] {
        let Some(mut t) = twin(dll) else { return };
        let frame = t.at(0x2800);
        let f = t.at(0x1940);
        let w = |t: &Twin, o: u32| op(t, f, o);
        // the vector slots write what they are handed, scaled
        for s in [234u32, 266] {
            log_calls(&mut t, s, 3, &[1, 2], move |m, a, k| {
                for i in 0..4 {
                    let x =
                        f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
                    m.w32(
                        a[0] + 4 * i,
                        (x + (s & 7) as f32 + k as f32 * 0.5).to_bits(),
                    );
                }
                s
            });
        }
        log_calls(&mut t, 232, 2, &[1], |m, a, k| {
            for i in 0..4 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) * 0.5 + k as f32;
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x232
        });
        log_calls(&mut t, 235, 1, &[], |m, a, k| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, (0.25 * i as f32 - k as f32).to_bits());
            }
            0x235
        });
        for s in [325u32, 217] {
            log_calls(&mut t, s, 2, &[], move |m, a, k| {
                let out = if s == 325 { a[0] } else { a[1] };
                for i in 0..4 {
                    m.w32(
                        out + 4 * i,
                        (1.5 * i as f32 + k as f32 - (s & 3) as f32).to_bits(),
                    );
                }
                s
            });
        }
        // slot 231 hashes its whole second matrix
        log_calls(&mut t, 231, 3, &[2], |m, a, k| {
            let mut x = a[0] ^ k;
            for i in 0..16 {
                x = x.rotate_left(5) ^ m.r32(a[1] + 4 * i);
            }
            // kept in memory too (the first call's `eax` is dropped)
            m.w32(SCRATCH + 0x7f0 + 4 * u32::from(a[1] == a[0]), x);
            x
        });
        log_calls(&mut t, 395, 3, &[2], |_, _, _| 0x395);
        log_calls(&mut t, 211, 2, &[], |_, a, _| a[0] ^ 0x0211_0000);
        // the id lists (`s4_small::id_list`: slot 288 `(kind, &n)`)
        let lists = SCRATCH + 0x7a0;
        log_calls(&mut t, 288, 2, &[], move |m, a, _| {
            let at = lists + if a[0] == 9 { 0 } else { 0x10 };
            m.w32(a[1], m.r32(at + 0xc));
            at
        });
        // the motion script at its end (`s6_misc::motion_script_step`)
        let step = t.img.rel32_target(frame + 0x49);
        let script = SCRATCH + 0x780;
        t.set_u32(op(&t, step, 0xb), script);
        t.write(script, &[0; 0x10]);
        t.set_u32(op(&t, step, 0x2), 0);
        let (id1, id2) = (w(&t, 0x50), w(&t, 0x5e));
        // (first list, second list, id 1, id 2)
        for (k, (l9, l11, a, b)) in [
            ([3u8, 5, 7], [9u8, 11, 0], 5u32, 7u32),
            ([3, 5, 7], [9, 11, 0], 9, 11),
            ([3, 5, 7], [9, 11, 0], 3, 11),
            ([3, 5, 7], [9, 11, 0], 2, 4),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            for direct in [true, false] {
                t.set_u32(LOG, 0);
                t.write(lists, &l9);
                t.set_u32(lists + 0xc, 3);
                t.write(lists + 0x10, &l11);
                t.set_u32(lists + 0x1c, 3);
                t.set_u32(id1, a);
                t.set_u32(id2, b);
                let y = w(&t, 0x97);
                t.set_u32(y, (0.5 + k as f32).to_bits());
                for (o, x) in [
                    (0x3aeu32, 2.0f32),
                    (0x3b6, 0.75),
                    (0x3d6, 0.5),
                    (0x3f4, 1.25),
                ] {
                    let at = w(&t, o);
                    set_f(&mut t, at, x);
                }
                t.write(w(&t, 0x20), &floats(16, k));
                // the done flag the frame answers from
                let done = op(&t, frame, 0x51);
                t.set_u32(done, k % 2);
                scrub(&mut t);
                let call = Call {
                    eax: 0x1940 + k,
                    ecx: 0x0ecc_0000 + k,
                    ..Call::default()
                };
                if direct {
                    t.check(f, call, &[], Eax::Checked, s9_misc::camera_aimed);
                } else {
                    t.check_entry(frame, call, &[], Eax::Checked);
                }
            }
        }
    }
}

/// Row 50: `magic_0565`'s trail task in two DLLs: the timer running with
/// the kind 0's sparks and without, reaching 0 (the child with its sound,
/// the object off its list or not there; the kind 0 goes on, another
/// gives the task back), below 0 with the trail drawn, its count run out.
#[test]
fn trail_tasks_0565() {
    for dll in [565u16, 693] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x3e40);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let light = t.img.rel32_target(f + 0x120);
        light_slots(&mut t, light);
        let trail_at = t.img.rel32_target(f + 0x24b);
        trail_stubs(&mut t, trail_at, true);
        // the kind 0's sparks (`s4_fam446::sparks_b`)
        log_calls(&mut t, 719, 3, &[], |_, a, _| 0x7190_0000 ^ a[1] ^ a[2]);
        for s in [314u32, 404] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| s ^ a[0]);
            t.stub_st0(s, move |_, a| {
                f64::from(f32::from_bits(a[0])) * if s == 404 { 0.75 } else { 0.5 }
            });
        }
        let sp = t.img.rel32_target(f + 0x10d);
        let take = t.img.rel32_target(sp + 0x9);
        let recs = SCRATCH + 0x4000;
        t.set_u32(op(&t, take, 0x37), recs);
        t.set_u32(op(&t, take, 0x61), recs);
        let gf = SCRATCH + 0x800;
        t.set_u32(op(&t, sp, 0x150), gf);
        let r2 = SCRATCH + 0x880;
        t.set_u32(op(&t, sp, 0x190), r2);
        for i in 0..8 {
            t.set_u32(r2 + 4 * i, 0x0e00_0000 + i);
        }
        // the child with its sound (`s4_fam446::start_child_sound_a`)
        let child = t.img.rel32_target(f + 0x174);
        let soa = t.img.rel32_target(child + 0x2d);
        log_calls(&mut t, 393, 3, &[1], |_, _, _| 0x393);
        log_calls(&mut t, 306, 3, &[], |_, a, _| 0x0306_0000 ^ a[2]);
        log_calls(&mut t, 713, 1, &[0], |_, _, _| 0x713);
        let ww = SCRATCH + 0x10000;
        t.set_u32(w(&t, 0x21), ww);
        let k = w(&t, 0x5d);
        set_f(&mut t, k, 1.0);
        let list = w(&t, 0x17e);
        let gp = SCRATCH + 0x7e8;
        t.set_u32(w(&t, 0x18c), gp);
        let (task, o) = (SCRATCH + 0x5000, SCRATCH + 0x8000);
        // (timer, kind, the trail's count, on the list)
        for (c, (timer, kind, count, listed)) in [
            (5i32, 0u32, 6u32, false),
            (5, 1, 6, false),
            (1, 0, 6, true),
            (1, 4, 6, true),
            (1, 4, 6, false),
            (-1, 0, 5, false),
            (-1, 1, 3, false),
            (-1, 0, 0, false),
            (0, 0, 6, false),
        ]
        .into_iter()
        .enumerate()
        {
            let c = c as u32;
            t.set_u32(LOG, 0);
            object_start_setup(&mut t, soa, 8);
            // slot 941 answers the object start's table, which the trail
            // draw also reads its frame count from (`+6`)
            t.set_u16(SCRATCH + 0x406, 5);
            t.set_u8(gf, 0);
            t.set_u32(op(&t, take, 0x08), 0);
            t.set_u32(op(&t, take, 0x0f), 8);
            t.set_u32(op(&t, take, 0x97), 8);
            t.set_u32(op(&t, take, 0x2b), 0);
            for i in 0..8 {
                t.write(recs + 0x74 * i, &[0x22; 0x74]);
                t.set_u32(recs + 0x74 * i + 0x6c, 0);
            }
            t.write(ww, &varied(0x5c20, c + 1));
            t.set_u32(ww + 0x5c04, kind);
            // the task at rest for the pool's give-back
            t.write(task, &varied(0x20, c));
            t.set_u32(task, 0);
            t.set_u32(task + 4, 0);
            t.set_u32(task + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            t.set_u32(task + 0x14, o);
            // the object, its place, the timer
            t.write(o, &floats(0x180, c));
            t.set_u32(o + 0x50c, timer.cast_unsigned());
            t.set_u32(o + 0x510, 10 + c);
            t.set_u32(o + 0x518, c % 2);
            t.set_u16(o + 0x124, 0xabcd);
            // the trail
            let trail = o + 0x2c0;
            t.set_u32(trail, 0x0041_0000 + c);
            t.set_u32(trail + 4, 1);
            t.set_u16(trail + 8, 2);
            t.set_u16(trail + 0xa, 0x100);
            for i in 0..32u32 {
                let x = i as f32;
                let xyz = [x * 1.5 + c as f32, x * 0.5 - 2.0, x * 0.25 - 3.0, 1.0];
                for (j, v) in (0u32..).zip(xyz) {
                    t.set_u32(trail + 0xc + 16 * i + 4 * j, v.to_bits());
                }
                t.set_u8(trail + 0x22c + i, 1);
            }
            t.write(trail + 0x20c, &[0x20 + c as u8, 0x90, 0x10, 0x80]);
            t.write(trail + 0x210, &[0xe0, 0x30, 0x70, 0x60]);
            t.set_u32(trail + 0x214, 1.5f32.to_bits());
            t.set_u32(trail + 0x218, 0.5f32.to_bits());
            t.set_u32(trail + 0x21c, 0.6f32.to_bits());
            t.set_u32(trail + 0x220, 0x1234 + 0x111 * c);
            t.set_u32(trail + 0x224, 5);
            t.set_u32(trail + 0x228, count);
            t.write(SCRATCH + 0x3f00, &[0x31; 0x40]);
            // the host list: empty or holding the object
            let node = SCRATCH + 0x5400;
            t.write(node, &[0x3c; 0x10]);
            t.set_u32(node, 0);
            t.set_u32(node + 4, 0);
            t.set_u32(node + 8, o);
            t.set_u32(list, if listed { node } else { 0 });
            t.set_u32(list + 4, if listed { node } else { 0 });
            t.set_u32(list + 8, 0);
            t.set_u32(list + 0xc, 1);
            scrub(&mut t);
            let call = Call {
                eax: 0x3e40 + c,
                ecx: 0x0ecc_0000 + c,
                ..Call::default()
            };
            t.check_entry(f, call, &[task], Eax::Checked);
        }
    }
}

/// `o6_fam_object::start_object_first` at `at` (its `call pool_alloc` at
/// `+call`): the program list (slot 941), the part start (714), a pool
/// of `free` nodes, the object list's tail empty or not (`k` odd) and its
/// counts.
fn start_first_setup(t: &mut Twin, at: u32, call: u32, free: u16, k: u32) {
    let framed = t.img.word(at) as u8 == 0x55;
    let (p_op, d_op) = if framed { (0x04, 0x5e) } else { (0x01, 0x5b) };
    t.set_u32(op(t, at, p_op), SCRATCH + 0x200);
    t.set_u32(op(t, at, d_op), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    log_calls(t, 941, 1, &[], |_, _, _| SCRATCH + 0x400);
    log_calls(t, 714, 5, &[], |_, a, _| 0x0714_0000 ^ a[2]);
    let pa = t.img.rel32_target(at + call + 1);
    let nodes = op(t, pa, 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(op(t, pa, 0x04), free);
    let tail = op(t, at, call + 0xc);
    t.set_u32(tail, if k % 2 == 1 { SCRATCH + 0x600 } else { 0 });
    let framed_off = if framed { 0x32 } else { 0x31 };
    for o in [call + 5 + 0x29, call + 5 + framed_off + 0x15] {
        let count_at = op(t, at, o);
        t.set_u32(count_at, 0x40 + o);
    }
}

/// Rows 165/166: the summon's ring of six objects per actor in both
/// DLLs, directly and through its start task: no actors, one, two and
/// three (the pool running out), near and far, in and out of a battle,
/// the task lists empty and not.
#[test]
fn summon_rings_started() {
    for dll in [247u16, 659] {
        let Some(mut t) = twin(dll) else { return };
        let task = t.at(0x1210);
        let f = t.at(0x1340);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let pi = t.img.rel32_target(f + 0x4a);
        t.set_u32(op(&t, pi, 0x01), SCRATCH + 0x7e0);
        let start = t.img.rel32_target(f + 0x166);
        let pick = t.img.rel32_target(f + 0x265);
        let ts = t.img.rel32_target(f + 0x30b);
        let ans = SCRATCH + 0x7d0;
        log_calls(&mut t, 225, 1, &[], move |m, _, _| m.r32(ans));
        log_calls(&mut t, 226, 2, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 227, 1, &[], move |m, _, _| m.r32(ans + 4));
        log_calls(&mut t, 719, 3, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 711, 3, &[], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 710, 4, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 262, 4, &[], |m, a, k| {
            for (i, x) in (1u32..).zip(&a[1..4]) {
                let v = 0.5 * i as f32 + (k % 7) as f32 - a[0] as f32 * 0.75;
                m.w32(*x, v.to_bits());
            }
            0x262
        });
        log_calls(&mut t, 268, 3, &[], |m, a, k| {
            m.w32(a[2] + 4, (k as f32 * 0.375).to_bits());
            m.w32(a[2], 0x2680_0000 ^ k);
            0x268
        });
        log_calls(&mut t, 314, 1, &[], |_, a, k| a[0] ^ k);
        t.stub_st0(314, |m, _| f64::from(m.r32(LOG) % 5) * 0.25 - 0.5);
        for s in [215u32, 413] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| (s << 16) ^ a[0]);
        }
        for (o, x) in [(0x115u32, 2.0f32), (0x156, 1.5), (0x1dd, 0.75)] {
            let a = w(&t, o);
            set_f(&mut t, a, x);
        }
        let bl = SCRATCH + 0x900;
        let picks = SCRATCH + 0x980;
        t.set_u32(w(&t, 0x9d), bl);
        t.write(bl, &[2, 1, 3, 0]);
        let ptab = op(&t, pick, 0x0c);
        t.set_u32(ptab, picks);
        t.write(picks, &[0x21, 0x32, 0x43, 0x54]);
        let actors = w(&t, 0xa8);
        for i in 0..8u32 {
            t.set_u32(actors + 4 * i, i);
        }
        t.set_u32(w(&t, 0x64), 5);
        let (head, tail) = (op(&t, ts, 0x75), op(&t, ts, 0x8c));
        // (actors, pool nodes, in a battle, load: busy / ready)
        for (k, (n, free, battle, busy, ready)) in [
            (0u32, 8u16, 0u32, 0u32, 1u32),
            (1, 8, 1, 0, 1),
            (2, 8, 0, 1, 1),
            (3, 5, 1, 0, 2),
            (1, 3, 0, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            for direct in [true, false] {
                t.set_u32(LOG, 0);
                t.set_u32(ans, if direct { 0 } else { busy });
                t.set_u32(ans + 4, ready);
                t.set_u32(w(&t, 0x7f), n);
                t.set_u32(w(&t, 0x319), battle);
                t.set_u32(w(&t, 0x310), 0x0c0c_0000 + k);
                // the global the task start's set-up clears
                t.set_u32(w(&t, 0x302), 0x5a5a_0000 + k);
                start_first_setup(&mut t, start, 0x9a, free, k);
                // the objects and records seeded (the words the start keeps)
                let objs = w(&t, 0x13a) - 0xd4;
                t.write(objs, &floats(0x720 * n.max(1) / 4, k + 3));
                let recs = w(&t, 0x12d) - 0x14;
                t.write(recs, &varied(0x138 * n.max(1), k + 4));
                t.set_u32(head, if k % 2 == 1 { SCRATCH + 0x2800 } else { 0 });
                t.set_u32(tail, SCRATCH + 0x2800);
                t.write(SCRATCH + 0x2800, &varied(0x40, k));
                // one pool for the task start's node and the listed one
                task_ring(&mut t, f, (0x39d, 0x3ac, 0x413, 0x422), k, 2 + k % 2);
                scrub(&mut t);
                let call = Call {
                    eax: 0x1340 + k,
                    ecx: 0x0ecc_0000 + k,
                    ..Call::default()
                };
                t.check_entry(if direct { f } else { task }, call, &[], Eax::Checked);
            }
        }
    }
}

/// Rows 81/82: the summon's five task nodes in two DLLs, directly and
/// through its start task (busy, the first call, ready): the free ring
/// with 6 to 8 nodes, the task list empty or not.
#[test]
fn summon_nodes_started() {
    for (dll, task, f) in [(132u16, 0x12b0u32, 0x13e0u32), (135, 0x3c60, 0x3d90)] {
        let Some(mut t) = twin(dll) else { return };
        let task = t.at(task);
        let f = t.at(f);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let pi = t.img.rel32_target(f + 0x44);
        t.set_u32(op(&t, pi, 0x01), SCRATCH + 0x7e0);
        let ans = SCRATCH + 0x7d0;
        log_calls(&mut t, 225, 1, &[], move |m, _, _| m.r32(ans));
        log_calls(&mut t, 226, 2, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 227, 1, &[], move |m, _, _| m.r32(ans + 4));
        log_calls(&mut t, 719, 3, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 711, 3, &[], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 710, 4, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 291, 5, &[], |_, a, k| a[0] ^ a[1] ^ k);
        log_calls(&mut t, 271, 1, &[], |_, a, _| a[0]);
        t.set_u32(w(&t, 0x6a), 0x0a0a_0001);
        t.set_u32(w(&t, 0x64), 0x0b0b_0002);
        // (free nodes, load: busy / first / ready)
        for (k, (free, busy, once, ready)) in [
            (6u32, 0u32, 0u32, 1u32),
            (7, 0, 1, 1),
            (8, 0, 0, 2),
            (6, 1, 0, 1),
            (7, 0, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            for direct in [true, false] {
                t.set_u32(LOG, 0);
                t.set_u32(ans, if direct { 0 } else { busy });
                t.set_u32(ans + 4, ready);
                t.set_u32(op(&t, task, 0x18), once);
                // the globals the start sets hold other values
                for o in [0x7bu32, 0x3f, 0x137, 0x25f, 0x381] {
                    t.set_u32(w(&t, o), 0x5a5a_0000 + o);
                }
                for i in 0..0x20 {
                    t.set_u32(w(&t, 0x140) + 4 * i, 0x4400_0000 + i);
                }
                task_ring(&mut t, f, (0x73, 0xa9, 0x10e, 0x124), k, free);
                scrub(&mut t);
                let call = Call {
                    eax: 0x13e0 + k,
                    ecx: 0x0ecc_0000 + k,
                    ..Call::default()
                };
                t.check_entry(if direct { f } else { task }, call, &[], Eax::Checked);
            }
        }
    }
}

/// Rows 167/168: the summon's task nodes per target in both DLLs,
/// directly and through its start task: no targets, one, three (found in
/// the table or not, the table empty), in and out of a battle, the free
/// ring with enough nodes, the task list empty or not.
#[test]
fn summon_targets_started() {
    for (dll, task, f) in [(347u16, 0x3490u32, 0x35c0u32), (491, 0x3480, 0x35b0)] {
        let Some(mut t) = twin(dll) else { return };
        let task = t.at(task);
        let f = t.at(f);
        let w = |t: &Twin, o: u32| op(t, f, o);
        let pi = t.img.rel32_target(f + 0x46);
        t.set_u32(op(&t, pi, 0x01), SCRATCH + 0x7e0);
        let ans = SCRATCH + 0x7d0;
        let targets = SCRATCH + 0x7c0;
        let list = SCRATCH + 0x900;
        log_calls(&mut t, 225, 1, &[], move |m, _, _| m.r32(ans));
        log_calls(&mut t, 226, 2, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 227, 1, &[], move |m, _, _| m.r32(ans + 4));
        log_calls(&mut t, 719, 3, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 711, 3, &[], |_, a, k| a[1] ^ k);
        log_calls(&mut t, 710, 4, &[], |_, a, k| a[0] ^ k);
        log_calls(&mut t, 617, 1, &[], move |m, a, _| {
            m.w32(a[1], m.r32(targets));
            list
        });
        log_calls(&mut t, 271, 1, &[], |_, a, _| a[0]);
        for s in [215u32, 413] {
            log_calls(&mut t, s, 1, &[], move |_, a, _| (s << 16) ^ a[0]);
        }
        t.set_u32(w(&t, 0x55), 0x0a0a_0001);
        t.write(list, &[7, 3, 9, 1]);
        let table = SCRATCH + 0x980;
        t.write(table, &[1, 9, 5, 3, 8]);
        t.set_u32(w(&t, 0xb6), table);
        t.set_u32(w(&t, 0x1bb), 0x0c0c_0005);
        // (targets, table bytes, in a battle, load: busy / first / ready)
        for (k, (n, m, battle, busy, once, ready)) in [
            (0i32, 5i32, 0u32, 0u32, 0u32, 1u32),
            (1, 5, 1, 0, 1, 1),
            (3, 5, 0, 0, 0, 2),
            (3, 0, 1, 1, 0, 1),
            (2, 3, 0, 0, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            for direct in [true, false] {
                t.set_u32(LOG, 0);
                t.set_u32(ans, if direct { 0 } else { busy });
                t.set_u32(ans + 4, ready);
                t.set_u32(op(&t, task, 0x18), once);
                t.set_u32(targets, n.cast_unsigned());
                t.set_u32(w(&t, 0x92), m.cast_unsigned());
                t.set_u32(w(&t, 0x1ce), battle);
                t.set_u32(w(&t, 0x205), battle);
                // the globals and records the start sets hold other values
                for o in [0x1b6u32, 0x1d4, 0x1e0, 0x210, 0x22a, 0x234, 0x23d, 0x3ce] {
                    t.set_u32(w(&t, o), 0x5a5a_0000 + o);
                }
                let recs = w(&t, 0x8b) - 0x508;
                t.write(recs, &varied(0x520 * 3 + 0x10, k + 2));
                let free = n.cast_unsigned() + 5 + k % 2;
                task_ring(&mut t, f, (0xd1, 0x86, 0x151, 0x169), k, free);
                scrub(&mut t);
                let call = Call {
                    eax: 0x35c0 + k,
                    ecx: 0x0ecc_0000 + k,
                    ..Call::default()
                };
                t.check_entry(if direct { f } else { task }, call, &[], Eax::Checked);
            }
        }
    }
}
