//! Round 20's ports (`ffx_magic::r20`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::manual_midpoint,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
mod magic_owna;
mod magic_r11;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{rec, row, twin};

/// Junk in the stack below the entry (the frames' locals start as it).
fn scrub(t: &mut Twin) {
    let entry = magic::STACK_TOP - 0x100;
    let junk: Vec<u8> = (0..0x1000u32)
        .map(|i| (i.wrapping_mul(0x2b) ^ 0x5b).to_le_bytes()[0])
        .collect();
    t.write(entry - 0x1000, &junk);
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, 0x5a5a_5a5a ^ o);
    }
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Folds `v` into word `i` of `slot`'s dump row (the outputs a stub writes
/// to the stack, which the comparison skips).
fn fold(m: &mut dyn StubMem, slot: u32, i: u32, v: u32) {
    let at = SCRATCH + 0x2_0000 + 0x40 * (slot % 0x400) + 4 * i;
    let x = m.r32(at).rotate_left(5) ^ v;
    m.w32(at, x);
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

fn kf(t: &Twin, f: u32, o: u32) -> f32 {
    f32::from_bits(t.u32(t.img.word(f + o)))
}

/// The vector and matrix slots as float maths both sides share: 234
/// (`a - b`), 248 (`a + b`), 235 (a matrix varied by its call), 328/329
/// (one word of the matrix plus the angle), 229 (`M[i] * 0.5 + v[i]`).
fn vector_slots(t: &mut Twin) {
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
    rec(t, 235, 1, |m, a| {
        let k = m.r32(row(235)) as f32;
        for i in 0..16u32 {
            let v = if i % 5 == 0 {
                1.0
            } else {
                0.125 * (i as f32) - k * 0.25
            };
            wf(m, a[0] + 4 * i, v);
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(t, s, 3, move |m, a| {
            let k = m.r32(row(s)) % 16;
            let v = rf(m, a[1] + 4 * k) + f32::from_bits(a[2]);
            wf(m, a[0] + 4 * k, v);
            fold(m, s, k, v.to_bits());
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            // the input words themselves too (a tiny float would vanish
            // in the sum)
            let x = m.r32(a[2] + 4 * i);
            fold(m, 229, 4 + i, x);
            let v = rf(m, a[1] + 4 * i) * 0.5 + f32::from_bits(x);
            wf(m, a[0] + 4 * i, v);
            fold(m, 229, i, v.to_bits());
        }
        0x229
    });
}

/// Float slots: 251, 19, 17 (by their first word) and 12 (the ground `g`
/// plus the call count).
fn float_slots(t: &mut Twin, g: f64) {
    for (s, v) in [(251u32, 1.5f64), (19, 0.75), (17, 2.5)] {
        rec(t, s, if s == 17 { 2 } else { 1 }, move |_, a| {
            (s << 16) ^ a[0]
        });
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 3));
    }
    rec(t, 12, 4, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, move |m, _| g + f64::from(m.r32(row(12))) * 0.25);
}

/// A task list (operands `head`, `tail` of `f`): empty or one node.
fn task_list(t: &mut Twin, f: u32, (head, tail): (u32, u32), listed: bool, k: u32) {
    let tl = SCRATCH + 0x740;
    junk(t, tl, 0x20, k);
    let h = t.img.word(f + head);
    t.set_u32(h, if listed { tl } else { 0 });
    let l = t.img.word(f + tail);
    t.set_u32(l, tl);
}

/// What [`flight_task`](ffx_magic::s5_misc::flight_task) at `ft` needs:
/// slots 217 and 233, its free ring (`free` nodes) and its task list.
fn flight_setup(t: &mut Twin, ft: u32, free: u16, listed: bool, k: u32) {
    let op = |t: &Twin, o: u32| t.img.word(ft + o);
    for (s, n, out) in [(217u32, 2usize, 1usize), (233, 3, 0)] {
        rec(t, s, n, move |m, a| {
            for i in 0..4 {
                wf(m, a[out] + 4 * i, 0.375 * i as f32 + s as f32 * 0.01);
            }
            s
        });
    }
    t.set_u32(op(t, 0x18), 0x0cc0_0007);
    let (count, head) = (op(t, 0x158), op(t, 0x1cf));
    magic_r11::ring(t, count, head, free, SCRATCH + 0x1d000);
    task_list(t, ft, (0x236, 0x24f), listed, k);
}

/// `magic_0508` 0x10001210 (`r20::load_0508`, the start-up
/// `r20::start_up_0508`): busy, not loaded, then the start-up with the
/// yaw wrapped down, up and kept, the height of a near target (slot 251)
/// or a far one (`*G_F` set, or a target 0x20 and up), listed (two
/// objects) or not (one), slot 293 0 or not, the free rings of one or
/// three, the task lists empty or not.
#[test]
fn load_0508() {
    let Some(mut t) = twin(508) else { return };
    let f = t.at(0x1210);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let start = t.img.rel32_target(f + 0x5b);
    let op = |t: &Twin, o: u32| t.img.word(start + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    let (listed_at, s293) = (SCRATCH + 0x110, SCRATCH + 0x114);
    let k0 = kf(&t, start, 0x64);
    let (hi, lo) = (kf(&t, start, 0x73), kf(&t, start, 0x8c));
    // (busy, loaded, yaw, *G_F, target byte, listed, slot 293, free, task list)
    let cases: [(u32, u32, f32, u32, u8, u32, u32, u16, bool); 7] = [
        (1, 1, 0.0, 0, 3, 0, 0, 3, false),
        (0, 0, 0.0, 0, 3, 0, 0, 3, false),
        (0, 1, hi + 0.5, 0, 3, 0, 0, 3, false),
        (0, 2, lo - 0.5, 1, 3, 1, 1, 1, true),
        (0, 1, 0.5 * (hi + lo) + 0.25, 0, 0x25, 1, 0, 3, true),
        (0, 1, 0.5 * (hi + lo) - 0.25, 0, 0x1f, 0, 2, 3, false),
        (0, 1, hi - 0.125, 1, 0x30, 1, 0, 3, false),
    ];
    for (k, (b, l, yaw, near_off, tb, listed, r293, free, tl)) in (0u32..).zip(cases) {
        magic_r11::object_slots(&mut t);
        vector_slots(&mut t);
        float_slots(&mut t, 0.5 - f64::from(k));
        rec(&mut t, 262, 6, move |m, a| {
            let c = m.r32(row(262)) as f32;
            for i in 1..4 {
                wf(m, a[i], 0.5 * i as f32 + c * 0.75 - k as f32);
            }
            0x262
        });
        rec(&mut t, 268, 3, |m, a| {
            let c = m.r32(row(268)) as f32;
            for i in 0..4 {
                wf(m, a[2] + 4 * i, 1.25 - i as f32 + c * 0.5);
            }
            0x268
        });
        // the yaw: `K - slot 333`
        let r = k0 - yaw;
        rec(&mut t, 333, 3, |_, a| 0x0333_0000 ^ a[0]);
        t.stub_st0(333, move |_, _| f64::from(r));
        rec(&mut t, 390, 1, |_, a| 0x0390_0000 ^ a[0]);
        rec(&mut t, 534, 1, move |m, _| m.r32(listed_at));
        rec(&mut t, 293, 1, move |m, _| m.r32(s293));
        for (s, n) in [(623u32, 8usize), (306, 3), (413, 1), (215, 1)] {
            rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
        }
        rec(&mut t, 225, 1, move |m, _| m.r32(busy));
        rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
        rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(listed_at, listed);
        t.set_u32(s293, r293);
        t.set_u32(w(&t, 0x18), k & 1);
        t.set_u32(w(&t, 0x26), 0x0000_5e5e);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        magic_r11::pools(&mut t, start, 1755);
        let so = t.img.rel32_target(start + 0xb9);
        magic_r11::object_start_auto(&mut t, so);
        // the target byte, the actors, the caster, `*G_F`, `*G_S`
        let sel = SCRATCH + 0x5000;
        t.write(sel, &[tb, 1, 0, 3]);
        t.set_u32(op(&t, 0xe5), sel);
        for o in [0x123u32, 0x19f, 0x2f3, 0x3f8] {
            for i in 0..0x40 {
                t.set_u32(op(&t, o) + 4 * i, 0x0c0c_0100 + i + (o << 12));
            }
        }
        t.set_u32(op(&t, 0x51), 0x0a00_0051 + k);
        t.set_u32(op(&t, 0x15a), near_off);
        t.set_u32(op(&t, 0x62d), 0x0a00_062d + k);
        junk_ops(
            &mut t,
            start,
            &[
                0x29, 0x33, 0x41, 0xed, 0x103, 0x10d, 0x293, 0x29d, 0x2a7, 0x2b1, 0x31e, 0x32b,
                0x338, 0x345, 0x35d, 0x367, 0x373, 0x379, 0x383, 0x38d, 0x397, 0x3a1, 0x3ab, 0x3b5,
                0x3bf, 0x3c5, 0x43b, 0x449, 0x45d, 0x469, 0x46f, 0x479, 0x483, 0x48d, 0x497, 0x4a1,
                0x4ab, 0x4b5, 0x4bf, 0x4c9, 0x4d3, 0x4d9, 0x520, 0x52d, 0x53a, 0x547, 0x55d, 0x569,
                0x577, 0x57e, 0x588, 0x592, 0x59c, 0x5a6, 0x5b0, 0x5ba, 0x5c4, 0x5ce, 0x5d6, 0x5de,
                0x5e5, 0x5ec, 0x61b, 0x632, 0x63d,
            ],
        );
        // the caster's point (slot 262 fills it; `y` read back)
        t.set_u32(op(&t, 0xf5), 0x3fc0_0000 + k);
        let ft = t.img.rel32_target(start + 0x288);
        flight_setup(&mut t, ft, free, tl, k);
        // the flight's ring is the pool's: five takes in all (`free` + 4
        // nodes, the last take with one left or more)
        let take = t.img.rel32_target(start + 0x693);
        let (count, head) = (t.img.word(take + 4), t.img.word(take + 0x17));
        magic_r11::ring(&mut t, count, head, free + 4, SCRATCH + 0x1d000);
        let tlist = SCRATCH + 0x760;
        junk(&mut t, tlist, 0x20, k + 9);
        t.set_u32(op(&t, 0x699), if tl { tlist } else { 0 });
        t.set_u32(op(&t, 0x6ad), tlist);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(229)) >= 5, "the start-up ran");
    assert!(t.u32(row(251)) >= 1 && t.u32(row(19)) >= 2, "both heights");
}

/// What `m0514`'s glow and spark set-ups (at `glow`, `spark`) need: their
/// node lists (empty or not), rings, slots and globals (as
/// `magic_0514.rs`'s `glow_setup` and `spark_setup`).
fn setups_0514(t: &mut Twin, glow: u32, spark: u32, empty: bool, k: u32) {
    let w = |t: &Twin, f: u32, o: u32| t.img.word(f + o);
    let (old, prev, prev2) = (SCRATCH + 0x800, SCRATCH + 0x900, SCRATCH + 0xa00);
    t.write(old, &[0x6c; 0x40]);
    t.write(prev, &[0x7d; 0x40]);
    t.write(prev2, &[0x7e; 0x40]);
    for (l, p) in [
        (w(t, glow, 0x5c), prev),
        (w(t, glow, 0x18c), prev2),
        (w(t, spark, 0x95), prev),
    ] {
        t.write(l, &[0x4e; 0x20]);
        t.set_u32(l + 0x10, if empty { l } else { p });
        t.set_u32(l + 0xc, 0x00f1_0000);
        t.set_u16(l + 0x18, 0x0305);
    }
    t.set_u32(w(t, glow, 0x22), old);
    t.set_u32(w(t, glow, 0x3c), 0x8000_0007);
    t.set_u32(w(t, spark, 0x5d), old);
    t.set_u32(w(t, spark, 0x7b), 0x8000_0007);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(w(t, spark, 0x295), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(t, 941, 1, move |_, _| table);
    for (s, n) in [
        (325u32, 2usize),
        (714, 5),
        (268, 3),
        (388, 3),
        (239, 3),
        (719, 3),
        (306, 3),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    let cell = SCRATCH + 0x10;
    t.set_u32(cell, (0.5 + 600.0 * k as f32).to_bits());
    t.set_u32(cell + 4, k);
    t.set_u32(cell + 8, k);
    rec(t, 251, 1, |_, _| 0x251);
    t.stub_st0(251, move |m, _| f64::from(f32::from_bits(m.r32(cell))));
    rec(t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, move |m, _| {
        let k = m.r32(cell + 4);
        m.w32(cell + 4, k + 1);
        0.125 + f64::from(k) * 0.3125
    });
    rec(t, 324, 0, move |m, _| m.r32(cell + 8));
    let list2 = w(t, spark, 0x2ae);
    t.set_u32(list2 + 0x10, list2);
    t.set_u32(w(t, spark, 0x9), 0x5a5a_5aa7);
    t.set_u32(w(t, spark, 0x30d), 0x00a0_030d);
}

/// `magic_0514` 0x100078f0 (`r20::glow_task_0514`): states 0, 1 (waiting
/// and not), 2 (waiting; the glow or the spark of `n % 5` not done, a
/// negative count; both done with the count left and run out), 10000
/// (the flag bit clear, the glows' states sharing no bit, the nodes with
/// and without parents) and another state.
#[test]
fn glow_task_0514() {
    let Some(mut t) = twin(514) else { return };
    let f = t.at(0x78f0);
    let glow = t.img.rel32_target(f + 0xb8);
    let spark = t.img.rel32_target(f + 0xd8);
    let (task, node, nprev, nnext) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    let r = SCRATCH + 0x8000;
    let a = SCRATCH + 0x1800;
    let par = SCRATCH + 0x2800;
    // (state, wait, count, glow/spark done (bits of i), flag byte, glows'
    // states, parents)
    let cases: [(u32, u32, i32, (u32, u32), u8, u32, bool); 15] = [
        (0, 0, 3, (0, 0), 0, 0, false),
        (1, 3, 3, (0, 0), 0, 0, false),
        (1, 0, 3, (0x1f, 0x1f), 0, 0, false),
        (1, 0xffff_fffe, 3, (0x02, 0x00), 0, 0, true),
        (2, 2, 7, (0x1f, 0x1f), 0, 0, false),
        (2, 0, 7, (0x1b, 0x1f), 0, 0, false),
        (2, 0, 8, (0x1f, 0x17), 0, 0, false),
        (2, 0, 4, (0x1f, 0x1f), 4, 0x4e20, true),
        (2, 0, 1, (0x1f, 0x1f), 5, 0x0020, false),
        (2, 0, -3, (0x1f, 0x1f), 0, 0, false),
        (0x2710, 0, 1, (0, 0), 0xfb, 0x4e20, false),
        (0x2710, 1, 1, (0, 0), 4, 0x4e20, true),
        (0x2710, 0, 1, (0, 0), 0x0c, 0x0001, false),
        (0x2710, 0, 1, (0, 0), 4, 0x4e20, false),
        (5, 2, 1, (0, 0), 4, 0x4e20, true),
    ];
    for (k, (state, wait, n, (gd, sd), flag, gs, parents)) in (0u32..).zip(cases) {
        setups_0514(&mut t, glow, spark, k % 2 == 0, k % 3);
        junk(&mut t, r - 0x1000, 0x3000, k);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.write(node, &[0x3b; 0x20]);
        t.write(nprev, &[0x3c; 0x20]);
        t.write(nnext, &[0x3d; 0x20]);
        t.set_u32(node + 4, nprev);
        t.set_u32(node + 8, nnext);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, wait);
        t.set_u32(r + 0x20, n.cast_unsigned());
        let i = n.rem_euclid(5);
        for j in 0..5u32 {
            let gst = if gd >> j & 1 != 0 { 0x4e20 } else { 0x2710 };
            let sst = if sd >> j & 1 != 0 { 0x4e20 } else { 0x1 };
            t.set_u32(r + 0x2f0 + 0x1ec * j, gst);
            t.set_u32(r + 0xc98 + 0x2d0 * j, sst);
            for q in 0..4u32 {
                t.set_u32(
                    r + 0xc80 + 0x2d0 * j + 0x18c + 4 * q,
                    (1.5 + q as f32).to_bits(),
                );
            }
        }
        // a negative count's index: the words before the record
        if n < 0 {
            let back = n % 5;
            let g = r.wrapping_add_signed(back * 0x1ec + 0x2f0);
            let s = r.wrapping_add_signed(back * 0x2d0 + 0xc98);
            t.set_u32(g, 0x4e20);
            t.set_u32(s, if i == 2 { 0x4e20 } else { 0 });
        }
        if state == 0x2710 || (state == 2 && n == 1) {
            for (j, o) in [0xaa0u32, 0x8b4, 0x6c8, 0x4dc, 0x2f0]
                .into_iter()
                .enumerate()
            {
                let v = if j == 4 && state == 2 {
                    0x4e20
                } else {
                    gs | (0x0101 << j)
                };
                t.set_u32(r + o, v);
            }
            if state == 2 {
                t.set_u32(r + 0x2f0 + 0x1ec, 0x4e20);
            }
        }
        t.set_u8(r + 0x2b8, flag);
        t.set_u32(r + 0x170, 0x00ed_0170 + k);
        t.set_u32(r + 0x174, 0x00ed_0174 + k);
        t.set_u32(r + 0x178 + 0x1c, 0x3fc0_001c);
        t.set_u32(r + 0x178 + 0x24, 0x3fc0_0024);
        t.write(a, &[0x2a; 0x60]);
        t.set_u32(a + 0x54, 0x00c0_0054);
        t.set_u32(r + 0x2d0, a);
        t.set_u32(r + 0x2d4, 0x00b0_0000 + k);
        // the three nodes that leave: a parent (count, links) or none
        for (j, o) in [0x24u32, 0x178, 0x1a4].into_iter().enumerate() {
            let p = par + 0x100 * j as u32;
            t.write(p, &[0x47; 0x40]);
            t.set_u32(p + 0xc, 3);
            let sib = p + 0x80;
            t.write(sib, &[0x48; 0x20]);
            // the first node first among its siblings, the last one last
            t.set_u32(r + o + 4, if j == 0 { p } else { sib });
            t.set_u32(r + o + 8, if j == 2 { p } else { sib + 0x10 });
            t.set_u32(r + o + 0x14, if parents { p } else { 0 });
        }
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0680` 0x100012b0 (`r20::load_0680`, the start-up
/// `r20::start_up_0680`): busy, not loaded, then the start-up with no,
/// two, three and five objects, dealt kinds whose actors check 0 (two
/// objects) and not (one), bytes past the dealt ones -1 (the caster),
/// the pool with enough nodes, the last one taken by the third node, the
/// task list empty and not.
#[test]
fn load_0680() {
    let Some(mut t) = twin(680) else { return };
    let f = t.at(0x12b0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let start = t.img.rel32_target(f + 0x5c);
    let op = |t: &Twin, o: u32| t.img.word(start + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    let deal = t.img.rel32_target(start + 0x58);
    let kc = t.img.rel32_target(start + 0x155);
    let list = SCRATCH + 0x180;
    // (busy, loaded, objects, dealt, free nodes, task list)
    let cases: [(u32, u32, u32, u32, u16, bool); 7] = [
        (1, 1, 2, 2, 3, false),
        (0, 0, 2, 2, 3, false),
        (0, 1, 0, 2, 3, false),
        (0, 1, 2, 2, 3, true),
        (0, 2, 3, 3, 4, false),
        (0, 1, 5, 3, 3, true),
        (0, 1, 3, 1, 5, false),
    ];
    for (k, (b, l, n, total, free, listed)) in (0u32..).zip(cases) {
        magic_r11::object_slots(&mut t);
        magic_r11::pools(&mut t, start, 0x60);
        let so = t.img.rel32_target(start + 0x17d);
        magic_r11::object_start_auto(&mut t, so);
        t.set_u32(t.img.word(deal + 0x17), 3);
        for i in 0..3 {
            t.set_u32(t.img.word(deal + 0x46) + 0x1c * i, 2);
        }
        t.set_u32(t.img.word(deal + 0x1e), total);
        t.stub(324, |m, _| {
            let k = m.r32(SCRATCH + 0x710);
            m.w32(SCRATCH + 0x710, k + 1);
            k * 7 + 1
        });
        t.set_u32(SCRATCH + 0x710, k);
        rec(&mut t, 333, 3, |_, a| a[0] ^ 0x333);
        t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
        for (s, x) in [(314u32, 0.75f64), (404, 0.25), (12, -1.5)] {
            rec(&mut t, s, 1, move |_, a| a[0] ^ (s << 16));
            t.stub_st0(s, move |m, a| {
                f64::from(f32::from_bits(a[0]) * 0.5) + x + f64::from(m.r32(row(s)))
            });
        }
        rec(&mut t, 262, 4, move |m, a| {
            let c = m.r32(row(262)) as f32;
            for i in 1..4 {
                wf(m, a[i], 0.5 * i as f32 + c * 0.75 - k as f32);
            }
            0x262
        });
        rec(&mut t, 268, 3, |m, a| {
            let c = m.r32(row(268)) as f32;
            for i in 0..4 {
                wf(m, a[2] + 4 * i, 1.25 - i as f32 + c * 0.5);
            }
            0x268
        });
        rec(&mut t, 271, 1, |_, a| a[0] ^ 0x271);
        // the kinds: none in the DLL's list, slot 534 by the number's parity
        rec(&mut t, 390, 1, |_, a| 0x7777_0000 ^ a[0]);
        rec(&mut t, 534, 1, |_, a| a[0] & 1);
        for o in [0x106u32, 0x1d2, 0x294, 0x34d] {
            t.set_u32(op(&t, o), list);
        }
        t.set_u32(t.img.word(kc + 0x10), list);
        t.set_u32(t.img.word(kc + 0x74), list);
        for i in 0..8u8 {
            t.set_u8(list + u32::from(i), 3 + 5 * i);
        }
        for o in [0x111u32, 0x29f] {
            for i in 0..0x30 {
                t.set_u32(op(&t, o) + 4 * i, 0x00c0_0000 + 0x100 * o + i);
            }
        }
        // the caster and its number
        for o in [0xffu32, 0x28d] {
            t.set_u32(op(&t, o), 0x00ca_0000 + o);
        }
        for o in [0x1cbu32, 0x346] {
            t.set_u32(op(&t, o), 0x0000_0040 + k);
        }
        t.set_u32(t.img.word(kc + 0x9), 0x0000_0021 + k);
        t.set_u32(t.img.word(kc + 0x55), 0x0000_0022 + k);
        t.set_u32(op(&t, 0x63), 0x00ac_7000);
        t.set_u32(op(&t, 0x99), n);
        let (objs, recs) = (op(&t, 0x70) - 0xd4, op(&t, 0xa4) - 0x14);
        junk(&mut t, objs, 0x130 * 10, k);
        junk(&mut t, recs, 0x34 * 6, k + 1);
        junk_ops(
            &mut t,
            start,
            &[
                0x29, 0x33, 0x3d, 0x4b, 0x82, 0x8c, 0x3d9, 0x3e3, 0x3ed, 0x3f7, 0x401, 0x40b,
                0x415, 0x41f, 0x429, 0x433, 0x43d, 0x447, 0x451, 0x503, 0x50d, 0x517, 0x521, 0x52b,
                0x535, 0x53f, 0x549, 0x553, 0x55d, 0x567, 0x570, 0x576, 0x580, 0x58a, 0x639,
            ],
        );
        // the pool: three takes (with three nodes the last takes the
        // last one)
        let (count, head) = (op(&t, 0x3d3), op(&t, 0x46a));
        magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1400);
        let other = SCRATCH + 0x5f0;
        t.write(other, &[0x44; 0x20]);
        t.set_u32(op(&t, 0x4ca), if listed { other } else { 0 });
        t.set_u32(op(&t, 0x4e4), other);
        rec(&mut t, 225, 1, move |m, _| m.r32(busy));
        rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
        rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), k & 1);
        t.set_u32(w(&t, 0x27), 0x0000_5e5e);
        for o in [0x62u32, 0x6c] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        scrub(&mut t);
        // the bytes past the dealt ones: -1 (the load step makes no frame:
        // the start-up's `ebp - 0x14` is the entry's `esp - 0x1c`)
        t.write(magic::STACK_TOP - 0x100 - 0x1c, &[0xff; 0x10]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(271)) >= 5, "the start-up ran");
    assert!(t.u32(row(12)) >= 1, "two objects for a kind");
}

/// `magic_0645` 0x100077e0 (`r20::colour_setup_0645`): the node list
/// empty and not; slot 325 copies the word's four floats.
#[test]
fn colour_setup_0645() {
    let Some(mut t) = twin(645) else { return };
    let at = t.at(0x77e0);
    let list = t.img.word(at + 0x5d);
    let ring_last = t.img.word(at + 0x25);
    let ring_flags = t.img.word(at + 0x43);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0x167), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    let list2 = t.img.word(at + 0x183);
    t.set_u32(list2 + 0x10, list2);
    let obj = SCRATCH + 0x1000;
    let src = SCRATCH + 0x600;
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (k, empty) in [(0u32, true), (1, false)] {
        t.write(obj, &[0x5b; 0x300]);
        junk(&mut t, src, 0x10, k);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: src,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::r20::colour_setup_0645(g, img, fr))
        });
    }
}

/// `magic_0610` 0x1000b500 (`r20::summon_matrix_0610`): three targets
/// and records. (The `M == 0` way needs `*G_E` = -0xc20, whose words the
/// function then reads at 0x40: not run.)
#[test]
fn summon_matrix_0610() {
    let Some(mut t) = twin(610) else { return };
    let f = t.at(0xb500);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let r = SCRATCH + 0x8000;
    let buf = SCRATCH + 0xa000;
    rec(&mut t, 0, 1, move |_, _| buf);
    for (s, n) in [(26u32, 3usize), (10, 2), (210, 2), (211, 2), (257, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    vector_slots(&mut t);
    let s = 330u32;
    rec(&mut t, s, 3, move |m, a| {
        let k = m.r32(row(s)) % 16;
        let v = rf(m, a[1] + 4 * k) - f32::from_bits(a[2]);
        wf(m, a[0] + 4 * k, v);
        s
    });
    for (s, v) in [(277u32, 1.25f64), (15, 0.75), (16, -2.5), (17, 3.0)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |m, a| {
            v + f64::from(a[0] & 7) + f64::from(m.r32(row(s))) * 0.5
        });
    }
    let sel = SCRATCH + 0x5000;
    for k in 0..3u32 {
        junk(&mut t, r, 0x1000, k);
        t.set_u32(op(&t, 0x1a), r);
        t.write(sel, &[2 + k as u8, 1, 0, 3]);
        for o in [0x57u32, 0x142, 0x17c, 0x23d] {
            t.set_u32(op(&t, o), sel);
        }
        for o in [0x6au32, 0x14c, 0x186, 0x249] {
            for i in 0..8 {
                t.set_u32(op(&t, o) + 4 * i, 0x00a0_0000 + 0x100 * o + i);
            }
        }
        junk_ops(&mut t, f, &[0x42, 0x49]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0705` 0x100020c0 (`r20::frame_0705`): off, a negative time,
/// then the times of the tint and glow bands with and without the fade,
/// part flags set (some cleared by their refresh) or all clear, points
/// flagged or not, the done way with `*G_E` set and not, the stop word
/// set and not.
#[test]
fn frame_0705() {
    let Some(mut t) = twin(705) else { return };
    let f = t.at(0x20c0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let rf_at = t.img.rel32_target(f + 0xab);
    let rop = |t: &Twin, o: u32| t.img.word(rf_at + o);
    for (i, o) in [0x1fu32, 0x2a, 0x36, 0x49].into_iter().enumerate() {
        t.set_u32(rop(&t, o), 0x0600_0000 + i as u32);
    }
    t.set_u32(rop(&t, 0x56), SCRATCH + 0x700);
    t.set_u32(rop(&t, 0x63), SCRATCH + 0x800);
    rec(&mut t, 258, 1, |_, a| 0x2580_0000 ^ a[0]);
    rec(&mut t, 231, 4, |_, a| 0x2310_0000 ^ a[0]);
    rec(&mut t, 230, 6, |_, a| 0x2300_0000 ^ a[0]);
    // slot 712: 0 for objects whose address has bit 7 set (their flag
    // stays) unless the cell says all
    rec(&mut t, 712, 8, |m, a| {
        m.w32(SCRATCH + 0x804, m.r32(SCRATCH + 0x800));
        u32::from(m.r32(SCRATCH + 0x900) != 0 || a[0] & 0x80 == 0)
    });
    rec(&mut t, 353, 1, |m, a| {
        magic_owna::dump(m, 353, a[0], 4);
        0x5a5a_0353
    });
    rec(&mut t, 776, 1, |_, a| a[0] ^ 0x0776_0000);
    for (s, n) in [
        (915u32, 5usize),
        (723, 1),
        (724, 2),
        (236, 3),
        (213, 2),
        (267, 1),
        (713, 1),
        (214, 1),
        (821, 1),
        (822, 4),
        (823, 2),
        (824, 4),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 229, i, v.to_bits());
        }
        0x229
    });
    let (p1, p2, p3) = (SCRATCH + 0x200, SCRATCH + 0x210, SCRATCH + 0x220);
    // (on, time, `*G_C`, part flags (bit i), points (bit i), all refreshed, *G_E, stop)
    let cases: [(u32, i32, u32, u32, u32, u32, u32, u32); 8] = [
        (0, 5, 0, 0, 0, 0, 1, 0),
        (1, -1, 0, 0, 0, 0, 1, 0),
        (1, 30, 0x40, 0x8421, 0x0a5, 0, 1, 0),
        (1, 100, 0x60, 0, 0x800, 1, 1, 0),
        (1, 110, 0x51, 0xffff, 0xfff, 1, 0, 1),
        (1, 280, 0x90, 0x1, 0, 1, 1, 0),
        (1, 0, 0x70, 0x0300, 0x101, 0, 1, 1),
        (1, 119, 0x50, 0, 0, 0, 1, 0),
    ];
    for (k, (on, time, c, parts, points, all, e, stop)) in (0u32..).zip(cases) {
        t.set_u32(op(&t, 0x12), on);
        t.set_u32(op(&t, 0x1f), time.cast_unsigned());
        t.set_u32(op(&t, 0x90), c);
        for (o, p) in [(0x2bu32, p1), (0x3c, p2), (0x246, p3)] {
            t.set_u32(op(&t, o), p);
        }
        junk(&mut t, p1, 0x30, k);
        t.set_u32(p3, stop);
        for o in [0x31u32, 0x5a, 0xef, 0x176, 0x22e] {
            t.set_u32(op(&t, o), 0x0a00_0000 + o + k);
        }
        t.set_u32(op(&t, 0x159), e);
        t.set_u32(SCRATCH + 0x900, all);
        t.set_u32(SCRATCH + 0x800, 0x5555);
        // the two objects, the 16 parts and the 12 points
        for (j, o) in [0xa6u32, 0xb0].into_iter().enumerate() {
            let obj = op(&t, o);
            junk(&mut t, obj, 0x40, k + j as u32);
            t.set_u16(obj + 0x1e, u16::from(k % 2 == 0));
        }
        let p0 = op(&t, 0xbf) - 0x1e;
        for i in 0..16 {
            let obj = p0 + 0x80 * i;
            junk(&mut t, obj, 0x40, k + i);
            t.set_u16(obj + 0x1e, (parts >> i & 1) as u16);
        }
        let q0 = op(&t, 0xff);
        for i in 0..12 {
            let q = q0 + 0x80 * i;
            junk(&mut t, q - 0x40, 0x50, k + 2 * i);
            t.set_u16(q - 0x40 + 0x1e, ((points >> i & 1) * (i & 1)) as u16);
            t.set_u16(q - 0x22, (points >> i & 1) as u16);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(713)) >= 30, "the done way ran");
    assert!(t.u32(row(915)) >= 1, "the fade ran");
}

/// `magic_0705` 0x100017e0 (`r20::step_0705`): the load's steps (busy,
/// asked, not loaded), off, the sound's start (id 0xff or not) and each
/// state at its count and not (one out of range), the points every 7th
/// time (and the 12th), the two parts running or not and the second's
/// start, the motions' counter negative, at 0x50, even, odd, at 0x1e,
/// past 0x20 and with 16 parts done, the stop word set.
#[test]
fn step_0705() {
    let Some(mut t) = twin(705) else { return };
    let f = t.at(0x17e0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let (p2, p3, lock) = (SCRATCH + 0x200, SCRATCH + 0x210, SCRATCH + 0x220);
    let cell = SCRATCH + 0x240;
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    for (s, n) in [
        (226u32, 2usize),
        (220, 3),
        (228, 2),
        (213, 2),
        (714, 5),
        (239, 3),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(&mut t, 225, 1, move |m, _| m.r32(cell));
    rec(&mut t, 227, 1, move |m, _| m.r32(cell + 4));
    rec(&mut t, 253, 0, move |m, _| m.r32(cell + 8));
    rec(&mut t, 237, 4, |m, a| {
        let k = m.r32(row(237)) as f32;
        for i in 0..4 {
            wf(
                m,
                a[3] + 4 * i,
                0.5 * i as f32 + k * 0.25 + (a[1] as f32) * 0.125,
            );
        }
        0x237
    });
    magic_owna::filler(&mut t, 247, 2, 16);
    vector_slots(&mut t);
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, |m, _| f64::from(m.r32(row(307)) % 11) * 0.37 - 1.1);
    let (pa, pb) = (op(&t, 0x1e8), op(&t, 0x4f9));
    // defaults: loaded, on, time 5, no state, no parts, no motions
    let defaults: [(u32, u32); 14] = [
        (0x15, 1),
        (0x42, 1),
        (0x6d, 1),
        (0xc7, 5),
        (0x14d, 0xc5),
        (0x16e, 0x33),
        (0x298, 0),
        (0x67b, 0xffff_ffff),
        (0x68b, 0),
        (0x23, 0x00ae_0000),
        (0x3c1, 0x00ac_0000),
        (0x6af, 0x0060_06af),
        (0x2ed, 0x0050_3000),
        (0x142, 0x0000_0077),
    ];
    // (overrides, 225 busy, 227 loaded, 253 on, stop, sound id, h, run halfwords)
    #[allow(clippy::type_complexity)]
    let cases: Vec<(
        Vec<(u32, u32)>,
        u32,
        u32,
        u32,
        u32,
        u32,
        i16,
        (u16, u16, u32),
    )> = vec![
        (vec![(0x15, 0)], 1, 1, 1, 0, 3, 0, (0, 0, 0)),
        (
            vec![(0x15, 0), (0x42, 0), (0x6d, 0)],
            0,
            0,
            1,
            0,
            3,
            0,
            (0, 0, 0),
        ),
        (vec![(0x6d, 0)], 0, 2, 0, 0, 3, 0, (0, 0, 0)),
        (vec![(0xc7, 0)], 0, 1, 1, 0, 0x22, 7, (0, 0, 0)),
        (vec![(0xc7, 0)], 0, 1, 1, 1, 0xff, 7, (0, 0, 0)),
        (
            vec![(0x14d, 0xc0), (0x16e, 0x77)],
            0,
            1,
            1,
            0,
            0x22,
            7,
            (0, 0, 0),
        ),
        (
            vec![(0x14d, 0xc0), (0x16e, 0x78)],
            0,
            1,
            1,
            0,
            0x22,
            7,
            (0, 0, 0),
        ),
        (
            vec![(0x14d, 0xc1), (0x16e, 0x58)],
            0,
            1,
            1,
            0,
            0x22,
            -9,
            (0, 0, 0),
        ),
        (
            vec![(0x14d, 0xc2), (0x16e, 0x77)],
            0,
            1,
            1,
            0,
            0x22,
            7,
            (0, 0, 0),
        ),
        (
            vec![(0x14d, 0xc3), (0x16e, 0x2c)],
            0,
            1,
            1,
            0,
            0x22,
            7,
            (0, 0, 0),
        ),
        (
            vec![(0x14d, 0xc4), (0x16e, 0x77)],
            0,
            1,
            1,
            0,
            0x22,
            7,
            (0, 0, 0),
        ),
        (
            vec![(0x14d, 0xc4), (0x16e, 0x76)],
            0,
            1,
            1,
            0,
            0x22,
            7,
            (0, 0, 0),
        ),
        (
            vec![(0xc7, 15), (0x298, 3)],
            0,
            1,
            1,
            0,
            0x22,
            300,
            (0, 0, 0),
        ),
        (
            vec![(0xc7, 15 + 7 * 12), (0x298, 11)],
            0,
            1,
            1,
            0,
            0x22,
            -3,
            (0, 0, 0),
        ),
        (
            vec![(0xc7, 22), (0x298, 12)],
            0,
            1,
            1,
            0,
            0x22,
            0,
            (0, 0, 0),
        ),
        (vec![(0xc7, 16)], 0, 1, 1, 0, 0x22, 0, (1, 0, 0x3c000)),
        (vec![(0xc7, 103)], 0, 1, 1, 0, 0x22, 5, (1, 0, 0x3c001)),
        (vec![], 0, 1, 1, 0, 0x22, 5, (0, 1, 0)),
        (
            vec![(0x67b, 0x50), (0x68b, 3)],
            0,
            1,
            1,
            0,
            0x22,
            5,
            (0, 0, 0),
        ),
        (
            vec![(0x67b, 0x56), (0x68b, 15)],
            0,
            1,
            1,
            0,
            0x22,
            -5,
            (1, 1, 0),
        ),
        (
            vec![(0x67b, 0x57), (0x68b, 4)],
            0,
            1,
            1,
            0,
            0x22,
            5,
            (0, 0, 0),
        ),
        (
            vec![(0x67b, 0x6e), (0x68b, 1)],
            0,
            1,
            1,
            1,
            0x22,
            5,
            (0, 0, 0),
        ),
        (
            vec![(0x67b, 0x70), (0x68b, 2)],
            0,
            1,
            1,
            0,
            0x22,
            5,
            (0, 0, 0),
        ),
        (
            vec![(0x67b, 0x10), (0x68b, 2)],
            0,
            1,
            1,
            0,
            0x22,
            5,
            (0, 0, 0),
        ),
        (
            vec![(0x67b, 0x52), (0x68b, 16)],
            0,
            1,
            1,
            0,
            0x22,
            5,
            (0, 0, 0),
        ),
    ];
    for (k, (over, busy, loaded, on, stop, id, h, (ra, rb, run))) in (0u32..).zip(cases) {
        for &(o, v) in defaults.iter().chain(over.iter()) {
            t.set_u32(op(&t, o), v);
        }
        for o in [0x2d9u32, 0x4cf] {
            t.set_u32(op(&t, o), p2);
        }
        t.set_u32(op(&t, 0xa9), p3);
        junk(&mut t, p2, 0x30, k);
        t.set_u32(p3, stop);
        let sp = t.img.rel32_target(f + 0x316);
        for o in [0x04u32, 0x29] {
            t.set_u32(t.img.word(sp + o), lock);
        }
        t.set_u32(lock, 0x5555 + k);
        t.set_u32(cell, busy);
        t.set_u32(cell + 4, loaded);
        t.set_u32(cell + 8, on);
        let snd = op(&t, 0x17e);
        junk(&mut t, snd, 0x14, k);
        t.set_u32(snd, id);
        t.set_u16(op(&t, 0x19c), h.cast_unsigned());
        // the parts and points
        let p0 = op(&t, 0x2f9);
        junk(&mut t, p0, 0x80 * 12, k);
        junk(&mut t, pa - 0x20, 0x80, k + 1);
        junk(&mut t, pb - 0x20, 0x80, k + 2);
        t.set_u16(op(&t, 0x3a1), ra);
        t.set_u16(op(&t, 0x565), rb);
        t.set_u32(op(&t, 0x45d), run);
        let q0 = op(&t, 0x82f);
        junk(&mut t, q0, 0x80 * 16, k + 3);
        let r0 = op(&t, 0x401);
        junk(&mut t, r0, 0x10, k + 4);
        let s0 = op(&t, 0x5c5);
        junk(&mut t, s0, 0x30, k + 5);
        // (the parts' junk covers the time and counter words: set again)
        for &(o, v) in defaults.iter().chain(over.iter()) {
            t.set_u32(op(&t, o), v);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(row(239)) >= 1, "the 0x1e motion");
    assert!(t.u32(row(714)) >= 6, "parts started");
    assert!(
        t.u32(row(228)) >= 4 && t.u32(row(213)) >= 1,
        "the sound states"
    );
}

/// The slots of `magic_0473`'s layers: 885 (by the cell's bits: the
/// first layer and/or the second), 776 and 905 (recording the colour and
/// texture words).
fn layer_slots_0473(t: &mut Twin, cell: u32) {
    rec(t, 885, 7, move |m, a| {
        let k = m.r32(row(885));
        magic_owna::dump(m, 885, a[6], 4);
        (m.r32(cell) >> ((k + 1) % 2)) & 1
    });
    rec(t, 776, 1, |_, a| a[0] ^ 0x0776_0000);
    rec(t, 905, 9, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[7] + 4 * i);
            fold(m, 905, i, v);
        }
        0x0905_0000 ^ a[2]
    });
}

/// `magic_0473` 0x100013e0 (`r20::layers_0473`): both layers, either and
/// none; colour words above 2^31 and keys using every field.
#[test]
fn layers_0473() {
    let Some(mut t) = twin(473) else { return };
    let f = t.at(0x13e0);
    let cell = SCRATCH + 0x240;
    layer_slots_0473(&mut t, cell);
    let lock = SCRATCH + 0x250;
    t.set_u32(t.img.word(f + 0xcd), lock);
    let obj = SCRATCH + 0x1000;
    let (c1, c2) = (SCRATCH + 0x300, SCRATCH + 0x320);
    for (k, which) in (0u32..).zip([3u32, 1, 2, 0]) {
        t.set_u32(cell, which);
        t.set_u32(lock, 0x5555 + k);
        junk(&mut t, obj + 0x800, 0x40, k);
        for (i, v) in (0u32..).zip([0x8000_0001u32, 0xffff_ffff, 7, 0x1234_5678]) {
            t.set_u32(c1 + 4 * i, v.rotate_left(k));
            t.set_u32(c2 + 4 * i, v ^ (0x1111 * k));
        }
        let args = [
            0x00a0_0000,
            0x00a1_0000 + k,
            0x00a2_0000 + k,
            0x4abc_d123 ^ (k << 20),
            0x0000_0003 + k,
            0xffff_ffff,
            0x8765_4321 ^ k,
            0x3f00_0000,
            0x3f40_0000,
            0x4000_0000 + k,
            0xbf80_0000,
            c1,
            c2,
        ];
        let c = Call {
            ecx: obj,
            ..Call::default()
        };
        t.check(f, c, &args, Eax::Checked, |g, img, fr| {
            ffx_magic::r20::layers_0473(g, img, fr)
        });
    }
}

/// `magic_0473` 0x10001800 (`r20::task_0473`): waiting, living, done
/// (the node given back); the angles past 0 and not; both cycles
/// restarting (from `rand`) and running, their channels mirrored past
/// their peaks and not.
#[test]
fn task_0473() {
    let Some(mut t) = twin(473) else { return };
    let f = t.at(0x1800);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    let lf = t.img.rel32_target(f + 0x4ac);
    let cell = SCRATCH + 0x240;
    layer_slots_0473(&mut t, cell);
    t.set_u32(cell, 3);
    let lock = SCRATCH + 0x250;
    t.set_u32(t.img.word(lf + 0xcd), lock);
    let keys = SCRATCH + 0x280;
    rec(&mut t, 719, 3, move |m, a| {
        let k = m.r32(row(719));
        m.w32(keys + 8 * (k % 4), 0x7b00_0000 ^ k ^ a[2]);
        m.w32(keys + 8 * (k % 4) + 4, 0x0000_0e00 ^ (k << 28));
        keys + 8 * (k % 4)
    });
    rec(&mut t, 210, 2, |_, a| a[0] ^ 0x210);
    vector_slots(&mut t);
    let s = 330u32;
    rec(&mut t, s, 3, move |m, a| {
        let k = m.r32(row(s)) % 16;
        let v = rf(m, a[1] + 4 * k) - f32::from_bits(a[2]);
        wf(m, a[0] + 4 * k, v);
        fold(m, s, k, v.to_bits());
        s
    });
    let r = SCRATCH + 0x8000;
    let obj = SCRATCH + 0x9000;
    t.set_u32(op(&t, 0x20), r);
    t.set_u32(op(&t, 0x3b7), obj);
    let gb = t.img.rel32_target(f + 0x48);
    let (node, prev, next) = (SCRATCH + 0x600, SCRATCH + 0x640, SCRATCH + 0x680);
    // (wait, life, angles, first counter, second counter, spans)
    let cases: [(i32, i32, (f32, f32), (i32, i32), (i32, i32), i32); 6] = [
        (3, 5, (0.5, 0.25), (4, 30), (2, 25), 40),
        (0, 5, (0.0, 3.0), (0, 30), (0, 25), 60),
        (0, 9, (-1.0, 0.0), (17, 20), (11, 20), 200),
        (-2, 1, (2.0, -0.5), (1, 25), (30, 31), 90),
        (0, 0, (1.0, 1.0), (5, 30), (5, 30), 40),
        (1, 7, (0.125, 0.0), (29, 30), (1, 44), 300),
    ];
    for (k, (wait, life, (a1, a2), (c1, n1), (c2, n2), span)) in (0u32..).zip(cases) {
        junk(&mut t, r + 0x500, 0x200, k);
        junk(&mut t, obj + 0x800, 0x40, k);
        let set = |t: &mut Twin, o: u32, v: i32| t.set_u32(r + o, v.cast_unsigned());
        set(&mut t, 0x604, wait);
        set(&mut t, 0x60c, life);
        set(&mut t, 0x608, 3 + k.cast_signed());
        t.set_u32(r + 0x5fc, a1.to_bits());
        t.set_u32(r + 0x600, a2.to_bits());
        set(&mut t, 0x5ac, c1);
        set(&mut t, 0x5b0, n1);
        set(&mut t, 0x5d4, c2);
        set(&mut t, 0x5d8, n2);
        for (i, o) in (0i32..).zip([0x5b4u32, 0x5b8, 0x5bc, 0x5c0, 0x5dc, 0x5e0, 0x5e4, 0x5e8]) {
            set(&mut t, o, span + 7 * i);
        }
        for (i, o) in (0i32..).zip([0x5c4u32, 0x5c8, 0x5cc, 0x5d0, 0x5ec, 0x5f0, 0x5f4, 0x5f8]) {
            set(&mut t, o, span / 3 + 5 * i - 10);
        }
        // the node given back: alone, its links junk
        junk(&mut t, node, 0x20, k);
        t.set_u32(node, prev);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, node);
        t.set_u32(node + 0xc, 0);
        junk(&mut t, prev, 0x20, k + 1);
        junk(&mut t, next, 0x20, k + 2);
        for o in [0x48u32, 0x55] {
            t.set_u32(t.img.word(gb + o), 0x0bad_0000 + o);
        }
        magic_r11::pools(&mut t, f, 0x60);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
    assert!(t.u32(row(905)) >= 8, "the layers ran");
}

/// `magic_0473` 0x10001cd0 (`r20::set_up_0473`): the yaw past each bound
/// and inside, the task list empty and not (the two frame words the
/// original never writes pinned to 0).
#[test]
fn set_up_0473() {
    let Some(mut t) = twin(473) else { return };
    let f = t.at(0x1cd0);
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x26);
    magic_r11::object_start_auto(&mut t, start);
    let yaw_in = SCRATCH + 0x714;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw_in))));
    let r = SCRATCH + 0x8000;
    t.set_u32(op(&t, 0x1c), r);
    t.set_u32(op(&t, 0x40), 0x00ac_0040);
    let kv = f32::from_bits(t.u32(op(&t, 0x6d)));
    let other = SCRATCH + 0x680;
    for (k, (yaw, listed)) in (0u32..).zip([(-7.0f32, false), (7.0, true), (0.5, true)]) {
        junk(&mut t, r, 0x600, k);
        t.set_u32(yaw_in, (kv - yaw).to_bits());
        magic_r11::pools(&mut t, f, 827);
        t.write(other, &[0x44; 0x20]);
        t.set_u32(op(&t, 0x2fd), if listed { other } else { 0 });
        t.set_u32(op(&t, 0x30f), other);
        for o in [0x27u32, 0x2f] {
            let sc = t.img.rel32_target(f + 0x2ea);
            t.set_u32(t.img.word(sc + o), 0x5a5a_5a5a ^ o);
        }
        scrub(&mut t);
        // junk read: [ebp-0x8] and [ebp-0xc] (never written) pinned to 0
        for o in [0xcu32, 0x10] {
            t.set_u32(magic::STACK_TOP - 0x100 - o, 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
