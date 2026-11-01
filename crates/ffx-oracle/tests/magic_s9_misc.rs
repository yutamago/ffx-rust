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
