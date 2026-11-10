//! Round 35's ports (`ffx_magic::r35`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
#[path = "magic/ownb.rs"]
mod ownb;
#[path = "magic/summon_stubs.rs"]
mod summon_stubs;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::rec;
use summon_stubs::{all_stubs, junk, junk_floats, prime, rf, visit_stub, wf};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

fn set_f(t: &mut Twin, at: u32, v: f32) {
    t.set_u32(at, v.to_bits());
}

/// `magic_0512` 0x10007a40 (`r35::trail_task_0512`): state 0 waiting and
/// starting; state 1 waiting (the fade 1.0) and fading, with one point
/// (no sprites), with the ring partly and fully counted (the walk
/// stopping at the oldest point), the fade running out (on into 0x1000);
/// the spacing reaching 0 (a smaller `K5`); all 64 sprites (a long ring); slot 773 answering
/// both ways; 0x1000 with and without parents; other states.
#[test]
fn trail_task_0512() {
    let Some(mut t) = twin(512) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7a40);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(&mut t, 667, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) / s;
            wf(m, a[0] + 4 * i, x);
        }
        0x667
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) + 0.0625 * i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x230
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    rec(&mut t, 706, 1, |_, a| 0x0706_0000 ^ a[0]);
    t.stub_st0(706, |m, a| {
        let v = [0, 1, 2].map(|i| f64::from(rf(m, a[0] + 4 * i)));
        (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() * 1.25
    });
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[0] ^ (a[1] << 4));
    t.stub_edx(772, |_, a| 0x0077_2000 + a[1]);
    // slot 773: found for every third handle, or as the cases' word says
    let found = SCRATCH + 0x7f0;
    rec(&mut t, 773, 2, move |m, a| {
        let mode = m.r32(found);
        u32::from(mode == 1 || (mode == 2 && a[1] % 3 == 0))
    });
    // the drawn matrix (the record's `+0x10`) and record words logged
    let mats = SCRATCH + 0x2_8000;
    for (s, n) in [(875u32, 4usize), (873, 5)] {
        rec(&mut t, s, n, move |m, a| {
            let k = m.r32(mats);
            m.w32(mats, k + 1);
            let r = a[3];
            let mat = m.r32(r + 0x10);
            let at = mats + 4 + 0x60 * (k % 0x100);
            for i in 0..16 {
                let x = m.r32(mat + 4 * i);
                m.w32(at + 4 * i, x);
            }
            for i in 0..8 {
                let x = m.r32(r + 4 * i);
                m.w32(at + 0x40 + 4 * i, x);
            }
            (s << 16) ^ a[2]
        });
    }
    for (s, n) in [(775u32, 2usize), (776, 1), (777, 1), (779, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 12) ^ a[0] ^ 0x0100_0101);
    }
    // the lock word
    let lock = SCRATCH + 0x7e0;
    t.set_u32(w(&t, 0x5c8), lock);
    // the DLL's mark byte, camera matrix, point matrix and record word
    t.set_u8(w(&t, 0x2a), 0x37);
    for i in 0..16u32 {
        let at = w(&t, 0x1ec) + 4 * i;
        set_f(&mut t, at, 0.5 + i as f32 * 0.25);
    }
    for (i, op) in (0u32..).zip([
        0x4a1u32, 0x4df, 0x50c, 0x544, 0x3d1, 0x3dc, 0x48d, 0x524, 0x4bd, 0x4ee, 0x518, 0x554,
        0x4d0, 0x4fd, 0x534, 0x566,
    ]) {
        let at = w(&t, op);
        set_f(
            &mut t,
            at,
            if i % 5 == 0 {
                1.5
            } else {
                0.125 * i as f32 - 0.5
            },
        );
    }
    t.set_u32(w(&t, 0x5a6), 0x00c7_2a40);
    let k5 = w(&t, 0x261);
    let k5_was = t.u32(k5);
    // the sprite table: 3 groups, 5 kinds
    let table = SCRATCH + 0x4000;
    t.write(table, &[0x3c; 0x200]);
    t.set_u16(table + 0x12, 3);
    t.set_u16(table + 6, 5);
    for i in 0..5u32 {
        let entry = 0x100 + 0x20 * i;
        t.set_u16(table + 8 * i + 0x10, entry as u16);
        t.set_u32(table + entry + 4, 4 + i);
        t.set_u8(table + entry + 8 + i, 0x40 + i as u8);
    }
    let (a, e, node) = (SCRATCH + 0x100, SCRATCH + 0x1_0000, SCRATCH + 0x200);
    let (prev, next) = (SCRATCH + 0x240, SCRATCH + 0x280);
    let model = SCRATCH + 0x3400;
    let parents = SCRATCH + 0x5000;
    // (state, wait, count, fade, spacing, slot 773's mode, parents, K5)
    let cases: [(u32, i32, u32, f32, f32, u32, bool, f32); 21] = [
        (0, 3, 0, 1.0, 0.02, 0, false, 0.0),
        (0, 0, 7, 0.5, 0.02, 0, false, 0.0),
        (1, 2, 0, 0.5, 0.02, 0, false, 0.0),
        (1, 0, 0, 0.5, 0.02, 0, false, 0.0),
        (1, 0, 1, 0.5, 0.02, 2, false, 0.0),
        (1, 0, 5, 0.5, 0.02, 0, false, 0.0),
        (1, 3, 9, 0.5, 0.004, 1, false, 0.0),
        (1, 0, 0x10, 0.5, 0.03, 2, false, 0.0),
        (1, 0, 0x11, 0.75, 0.01, 2, false, 0.0),
        (1, 0, 0x2e, 0.75, 0.002, 0, false, 0.0),
        (1, 0, 0x2f, 0.25, 0.05, 1, false, 0.0),
        (1, 0, 0x30, 0.3, 0.001, 2, false, 0.0),
        (1, 0, 0x12, 0.5, 0.02, 2, false, 0.05),
        (1, 0, 0x30, 0.5, 1.0, 2, false, 0.0),
        (1, 0, 0x0c, 0.5, 2.0, 0, false, 0.0),
        (1, 0, 0x22, 0.06, 0.02, 0, true, 0.0),
        (1, 0, 0x22, -1.0, 0.02, 0, true, 0.0),
        (0x1000, 4, 0x22, 0.5, 0.02, 0, true, 0.0),
        (0x1000, 4, 0x22, 0.5, 0.02, 0, false, 0.0),
        (0x2000, 4, 0, 0.5, 0.02, 0, false, 0.0),
        (7, -2, 0, 0.5, 0.02, 0, false, 0.0),
    ];
    for (k, (state, wait, count, fade, gap, mode, with_parents, k5_now)) in (0u32..).zip(cases) {
        t.write(e, &[0x5d; 0x200]);
        t.set_u32(e + 0x18, state);
        t.set_u32(e + 0x1c, 0x31 * k);
        t.set_u32(e + 0x20, wait.cast_unsigned());
        t.set_u32(e + 0x24, count);
        // (-1: the fade exactly `K1`, so that it ends at 0.0)
        let fade = if fade < 0.0 {
            f32::from_bits(t.u32(w(&t, 0x4a)))
        } else {
            fade
        };
        set_f(&mut t, e + 0x28, fade);
        t.set_u32(e + 0x2c, table);
        t.set_u32(e + 0x30, model);
        t.write(model, &[0x11; 0x100]);
        t.set_u32(model, class);
        for i in 0..4u32 {
            set_f(
                &mut t,
                model + 0xcc + 4 * i,
                0.75 * i as f32 - k as f32 * 0.125,
            );
        }
        // the ring: points `gap` (and a little more each step) apart
        for i in 0..16u32 {
            let d = gap * (1.0 + 0.1 * ((i * 7 + k) % 5) as f32);
            let x = [d * i as f32, 0.5 * d * i as f32, -d * i as f32, 1.0 + d];
            for (j, v) in (0u32..).zip(x) {
                set_f(&mut t, e + 0x34 + 16 * i + 4 * j, v);
            }
        }
        // the static nodes, children of parents or not
        for (i, o) in (0u32..).zip([0x134u32, 0x160, 0x18c, 0x1b8]) {
            let n = e + o;
            let p = parents + 0x40 * i;
            t.write(p, &[0x66; 0x40]);
            t.set_u16(p + 0x18, 3 + i as u16);
            let (sib, sib2) = (p + 0x20, p + 0x28);
            // the first an only child, the others between siblings
            if i == 0 {
                t.set_u32(n + 4, p);
                t.set_u32(n + 8, p);
            } else {
                t.set_u32(n + 4, sib);
                t.set_u32(n + 8, sib2);
            }
            t.set_u32(n + 0x14, if with_parents { p } else { 0 });
        }
        t.set_u32(a, node);
        t.set_u32(a + 4, e);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.write(prev, &[0x50; 0x10]);
        t.write(next, &[0x4f; 0x10]);
        t.set_u32(found, mode);
        t.set_u32(lock, 0x1234_0000 + k);
        t.set_u32(
            k5,
            if k5_now == 0.0 {
                k5_was
            } else {
                k5_now.to_bits()
            },
        );
        ownb::dirty_stack(&mut t, 0x7a40 + k, &[]);
        let drawn = t.u32(mats);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
        if gap >= 1.0 && state == 1 && count > 1 {
            // the long rings draw every sprite
            assert_eq!(t.u32(mats) - drawn, 64, "case {k}");
        }
    }
}

/// `magic_0597` 0x10009350 (`r35::scene_task_0597`): states 0, a wait, the
/// set-up 0x384 (slot 520's five answers, the model's flag bit both ways
/// and set by slot 255, the static list empty or not, slot 288's list
/// with each kind of byte skipped and none), 0x385, 0x386..0x388 waiting
/// and going on, 0x389, 0x2710 and above.
#[test]
fn scene_task_0597() {
    let Some(mut t) = twin(597) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x9350);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 292, 1, |m, a| {
        for i in 0..4 {
            wf(m, a[0] + 4 * i, 1.5 * i as f32 - 2.0);
        }
        0x292
    });
    let (who, flag_set) = (SCRATCH + 0x7f0, SCRATCH + 0x7f4);
    rec(&mut t, 520, 0, move |m, _| m.r32(who));
    // slot 255 sets the model's flag bit 0 when the case says so
    let flag = w(&t, 0x106);
    rec(&mut t, 255, 2, move |m, a| {
        if m.r32(flag_set) != 0 {
            let b = m.r32(flag) | 1;
            m.w32(flag, b);
        }
        0x0255_0000 ^ a[0]
    });
    // slot 790 takes no words
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25)] {
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    // slot 288: the byte list and its length
    let (bytes, len) = (SCRATCH + 0x7a0, SCRATCH + 0x7f8);
    rec(&mut t, 288, 2, move |m, a| {
        let n = m.r32(len);
        m.w32(a[1], n);
        bytes
    });
    prime(&mut t, f, SCRATCH + 0x3_8000);
    // the nodes' vtables (the DLL's) visit through a stub: calls into
    // overlay code are call-outs on the port's side
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let owned_vt = t.img.word(t.img.rel32_target(f + 0x2e5) + 5);
    for vt in [
        owned_vt,
        w(&t, 0x68),
        w(&t, 0x1f0),
        w(&t, 0x22a),
        w(&t, 0x7db),
        w(&t, 0x581),
    ] {
        t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    }
    // the bone's function table: slot 32's stub
    let table = t.img.word(t.img.rel32_target(f + 0x2fe) + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    let (a, r, node) = (SCRATCH + 0x100, SCRATCH + 0x1_0000, SCRATCH + 0x200);
    let (prev, next) = (SCRATCH + 0x240, SCRATCH + 0x280);
    let own = w(&t, 0x8d4);
    let last = w(&t, 0x8df);
    // (state, the count `R + 0x38`, slot 520, the flag byte, slot 255
    // sets it, the list's length)
    let cases: [(u32, i32, u32, u8, bool, u32); 22] = [
        (0, 0, 0, 0, false, 0),
        (5, 3, 0, 0, false, 0),
        (0x384, 0, 0x0235_0000, 0, false, 6),
        (0x384, 0, 0x0236_0000, 0, true, 6),
        (0x384, 0, 0x0237_0000, 1, false, 6),
        (0x384, 0, 0x0238_0000, 0, false, 0),
        (0x384, 0, 0x0239_0000, 0, false, 3),
        (0x384, 0, 0x0234_0000, 1, false, 6),
        (0x385, 9, 0, 0, false, 0),
        (0x386, 0x45, 0, 0, false, 0),
        (0x386, 0x46, 0, 0, false, 0),
        (0x387, 0x13, 0, 0, false, 0),
        (0x387, 0x14, 0, 0, false, 0),
        (0x388, 0x40, 0, 0, false, 0),
        (0x388, 0x41, 0, 0, false, 0),
        (0x389, 0, 0, 0, false, 0),
        (0x1000, 0, 0, 0, false, 0),
        (0x2710, 0, 0, 0, false, 0),
        (0x2711, 0, 0, 0, false, 0),
        (0x383, 0, 0, 0, false, 0),
        (0x384, 0, 0x0235_0000, 0, false, 1),
        (0x384, 0, 0x0237_0000, 0, true, 2),
    ];
    for (k, (state, count, answer, fl, sets, n)) in (0u32..).zip(cases) {
        for i in 0..0x40u32 {
            let x = ((i * 7 + k * 3) % 37) as f32 * 0.25 - 4.0;
            t.set_u32(r + 4 * i, x.to_bits());
        }
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x34, 0x40 + k);
        t.set_u32(r + 0x38, count.cast_unsigned());
        t.set_u32(who, answer);
        t.set_u32(flag_set, u32::from(sets));
        t.set_u8(flag, fl | 0xa0);
        t.set_u32(len, n);
        // the DLL's own byte (the model's flags too) and the mark byte;
        // junk at the words the set-up stores
        let ownb = w(&t, 0x41);
        t.set_u8(ownb, 0x30 + k as u8);
        let markb = w(&t, 0x151);
        t.set_u8(markb, 0x60 + k as u8);
        junk_floats(
            &mut t,
            f,
            &[
                0x966, 0x9a1, 0x9c8, 0x9db, 0x9ee, 0xa09, 0xa21, 0xa29, 0x2be, 0x89a, 0x20e,
            ],
            k,
        );
        // the list: the DLL's own byte, the last's, 0xff and others
        let ob = t.u32(own) as u8;
        let lb = t.u32(last) as u8;
        t.write(bytes, &[ob, 0x31 + k as u8, lb, 0xff, 0x07, 0x52]);
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.write(prev, &[0x50; 0x10]);
        t.write(next, &[0x4f; 0x10]);
        // the static parent: its child list empty or with a last child
        let p = w(&t, 0x55);
        let kid = SCRATCH + 0x3000;
        t.write(kid, &[0x21; 0x40]);
        t.set_u32(p + 0x10, if k % 3 == 1 { kid } else { p });
        t.set_u32(p + 0xc, p);
        t.set_u16(p + 0x18, 2);
        // the colour node's parent: an empty list
        let cp = w(&t, 0x917);
        t.set_u32(cp + 0x10, cp);
        t.set_u32(cp + 0xc, cp);
        ownb::dirty_stack(&mut t, 0x9350 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0613` 0x1000a2e0 and `magic_0609` 0x1000a730
/// (`r35::summon_main_0613`, `summon_main_0609`): state 0 (slot 294
/// answering 0xff and not) and every entry into the chain to 0x33, the
/// fade word above and below its floor, the light's flag and the parts'
/// flags both ways, every wait's bound, 0609's own state 0x49, 10000 and
/// the states past the table; the static lists empty and not.
#[test]
fn summon_main_0613_0609() {
    for (n, rva, shift) in [(613u16, 0xa2e0u32, 0u32), (609, 0xa730, 1)] {
        summon_main_case(n, rva, shift == 1);
    }
}

fn summon_main_case(n: u16, rva: u32, is_0609: bool) {
    let Some(mut t) = twin(n) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(rva);
    // 0613's offset `x` in this copy (as the port's table)
    let o = move |x: u32| {
        if !is_0609 {
            return x;
        }
        let d = [
            (0u32, 0u32),
            (0xd69, 10),
            (0xd83, 20),
            (0x12a1, 9),
            (0x12b4, 6),
            (0x13d2, 18),
            (0x1456, 28),
        ]
        .iter()
        .rev()
        .find(|(from, _)| *from <= x)
        .map_or(0, |(_, d)| *d);
        x + d
    };
    let w = move |t: &Twin, x: u32| t.img.word(f + o(x));
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 294, 1, move |m, _| m.r32(answer));
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25)] {
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    rec(&mut t, 719, 3, |_, a| {
        0x0719_0000 ^ a[0] ^ (a[1] << 8) ^ (a[2] << 12)
    });
    // slot 771's handle: its high word too
    t.stub_edx(771, |_, a| 0x0077_1000 ^ a[0]);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    // the owned node's vtable visits through a stub; the bones' table
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let owned = t.img.rel32_target(f + o(0xd0c));
    let vt = t.img.word(owned + 5);
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    let table = t.img.word(t.img.rel32_target(f + o(0xd21)) + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let max = t.img.word(f + o(0x4c)) & 0xff;
    // the static parents (the scene's, the first node's) and the light
    // list's
    let parents = [w(&t, 0x221), w(&t, 0x175)];
    // (state, the count `R + 0x2c`, slot 294, the flag bits, lists empty)
    let mut cases: Vec<(u32, i32, u32, u8, bool)> = vec![
        (0, 0, 0xff, 0, true),
        (0, 0, 3, 5, false),
        (0xa, 0, 0, 0, true),
        (0xb, 0, 0, 0, false),
        (0xc, 0, 0, 0, true),
        (0xd, 0, 0, 0, false),
        (0x14, 3, 0, 0, true),
        (0x15, 0, 0, 0, false),
        (0x16, 0, 0, 0, true),
        (0x17, 0, 0, 0, true),
        (0x18, 0, 0, 0, true),
        (0x1e, 2, 0, 0, false),
        (0x1f, 0, 0, 0, true),
        (0x20, 0, 0, 0, false),
        (0x21, 0, 0, 0, true),
        (0x28, 0, 0, 0, true),
        (0x29, 0, 0, 0, true),
        (0x2a, 0, 0, 0, true),
        (0x2b, 0, 0, 0, true),
        (0x2c, 0, 0, 0, true),
        (0x32, 0, 0, 0, true),
        (0x33, 0, 0, 0, true),
        (0x34, 0x42, 0, 0, true),
        (0x34, 0x43, 0, 1, true),
        (0x35, 0, 0, 0, true),
        (0x35, 0, 0, 1, true),
        (0x35, 0, 0, 5, true),
        (0x3c, 0, 0, 0, true),
        (0x3d, 0, 0, 4, true),
        (0x3e, 7, 0, 0, true),
        (0x3e, 8, 0, 0, true),
        (0x3f, 0x1d, 0, 0, true),
        (0x3f, 0x1e, 0, 0, true),
        (0x40, 0x39, 0, 0, true),
        (0x40, 0x3a, 0, 0, true),
        (0x40, 0x3a, 0, 4, true),
        (0x46, 0, 0, 0, true),
        (0x47, 0, 0, 0, true),
        (0x47, 1, 0, 0, true),
        (0x48, 1, 0, 0, true),
        (0x48, 2, 0, 0, true),
        (0x2710, 0, 0, 0, true),
        (0x2711, 0, 0, 0, true),
        (5, 0, 0, 0, true),
        (max + 1, 0, 0, 0, true),
        (0x1000, 0, 0, 0, true),
    ];
    let bound = (t.img.word(f + o(0xe44) + 3) & 0xff).cast_signed();
    cases.push((max, bound - 1, 0, 0, true));
    cases.push((max, bound, 0, 0, true));
    if is_0609 {
        cases.push((0x49, 0x4a, 0, 0, true));
        cases.push((0x49, 0x4b, 0, 0, true));
    }
    for (k, (state, count, ans, bits, empty)) in (0u32..).zip(cases) {
        junk(&mut t, r, 0x60, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x2c, count.cast_unsigned());
        t.set_u32(answer, ans);
        t.set_u32(a + 4, r);
        let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
        t.set_u32(a, node);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        for (i, &p) in (0u32..).zip(&parents) {
            let blk = SCRATCH + 0x7800 + 0x40 * i;
            junk(&mut t, blk, 0x20, k + i);
            t.set_u32(p + 0x10, if empty { p } else { blk });
            t.set_u32(p + 0xc, p);
            t.set_u16(p + 0x18, 1 + i as u16);
        }
        // the light's flag (bit 0) and the parts' flags (bit 2)
        for x in [0xd9bu32, 0xdb9, 0xe12, 0x109d] {
            let at = w(&t, x);
            t.set_u8(at, bits | 0xa0);
        }
        // the fade word above its floor or at it
        let fade = w(&t, 0xf83);
        let floor = f32::from_bits(t.u32(w(&t, 0xf8a)));
        t.set_u32(fade, (floor + if k % 2 == 0 { 0.5 } else { 0.0 }).to_bits());
        junk_floats(
            &mut t,
            f,
            &[0x11c, 0xb87, 0xd26, 0x1295, 0x1034, 0x1159, 0xf59],
            k,
        );
        ownb::dirty_stack(&mut t, rva + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0577` 0x1000b530 (`r35::spark_task_0577`): every state waiting
/// and going on, the fade above and below its bounds, the parts' flags
/// both ways, the sparks' flags agreeing and not (on into 10000), 10000
/// itself and the states past the table.
#[test]
fn spark_task_0577() {
    let Some(mut t) = twin(577) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0xb530);
    let call = |t: &Twin, o: u32| t.img.rel32_target(f + o);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    rec(&mut t, 210, 2, |_, a| SCRATCH + 0x5000 + (a[0] & 0xf0));
    // slot 324 (a random number) takes no words: a counter
    let seed = SCRATCH + 0x7e0;
    rec(&mut t, 324, 0, move |m, _| {
        let x = m.r32(seed).wrapping_mul(0x0101_0101).wrapping_add(0x2b7);
        m.w32(seed, x);
        x >> 8
    });
    // slot 531 writes the target's position (the centre reads it back)
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 + (a[0] & 7) as f32);
        }
        0x531
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25)] {
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    // the bones' function table: matrices from slots 32..35
    let bone = call(&t, 0xa25);
    let table = t.img.word(bone + 0x103);
    for i in 0..4u32 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * (32 + i));
        rec(&mut t, 32 + i, 2, move |m, a| {
            for j in 0..16 {
                let x = 0.125 * j as f32 - rf(m, a[1]) + i as f32;
                wf(m, a[0] + 4 * j, x);
            }
            (i << 16) ^ a[1]
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    // the small node's vtable (the bones' cursor) visits through the stub
    let small = call(&t, 0x409);
    let svt = t.img.word(small + 0x44);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    // (state, the wait, the fade word R + 0x304, a float scale, the parts'
    // flag bits, the sparks agree)
    let cases: [(u32, i32, i32, f32, u8, bool); 30] = [
        (0, 0, 0, 0.0, 0, false),
        (1, 3, 0, 0.0, 0, false),
        (1, 0, 0, 0.0, 0, false),
        (2, 1, 0, 0.0, 0, false),
        (2, 0, 0, 0.0, 0, false),
        (0x14, 0, 0, 0.5, 0, false),
        (0x15, 0, 0, -1.9, 0, false),
        (0x15, 0, 0, 5.0, 0, false),
        (0x16, 0, -100, -1.5, 0, false),
        (0x16, 0, -10, 9.0, 4, false),
        (0x16, 0, -40, 9.0, 0, false),
        (0x33, 11, -50, 0.0, 0, false),
        (0x33, 10, -20, 0.0, 4, false),
        (0x33, 2, -20, 0.0, 0, false),
        (0x34, 0, -40, 0.0, 0, false),
        (0x34, 0, -31, 0.0, 0, false),
        (0x35, 1, 0, 0.0, 0, false),
        (0x35, 0, 0, 0.0, 0, false),
        (0x1f, 11, 0, 0.0, 0, false),
        (0x1f, 10, 0, 0.0, 0, false),
        (0x20, 1, 0, 0.0, 0, false),
        (0x20, 0, 0, 0.0, 0, false),
        (0x20, 0, 0, 0.0, 4, false),
        (0xb, 0, 0, -1.0, 0, false),
        (0xb, 0, 0, 9.0, 0, false),
        (0xb, 0, 0, 9.0, 4, true),
        (0x3d, 0, 0, 0.0, 0, true),
        (0x2710, 0, 0, 0.0, 0, false),
        (0x2711, 0, 0, 0.0, 0, false),
        (0x3e, 0, 0, 0.0, 0, false),
    ];
    for (k, (state, wait, fade, x, bits, agree)) in (0u32..).zip(cases) {
        t.write(r, &[0; 0x2000]);
        for o in (0x68u32..0x88).step_by(4) {
            t.set_u32(r + o, (x + o as f32 / 64.0).to_bits());
        }
        for o in [0x1a8u32, 0x200, 0x2a4, 0x2a8, 0x2ac, 0x88] {
            t.set_u32(r + o, (1.5f32 + o as f32 / 256.0).to_bits());
        }
        // the move's two points and words the states clear or set bytes of
        for o in (0x40u32..0x60).step_by(4) {
            t.set_u32(
                r + o,
                (0.75f32 * (o - 0x40) as f32 / 4.0 - 1.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(r + 0xd20, 0x5d5d_5d5d);
        t.set_u8(r + 0x456, 0x5d);
        t.set_u8(r + 0x89d, 0x5d);
        t.set_u32(r + 0x60, state);
        t.set_u32(r + 0x64, wait.cast_unsigned());
        t.set_u32(r + 0x304, fade.cast_unsigned());
        t.set_u32(r + 0xfc, SCRATCH + 0x5800);
        t.set_u32(r + 0xd28, SCRATCH + 0x5900);
        t.set_u32(r + 0x30c, class);
        t.set_u32(svt + 0x1c, magic::TRAPS + 16 * visit);
        for o in [0x9f0u32, 0xaf0, 0xbf0] {
            t.set_u8(r + o + 0xe8, bits);
        }
        t.set_u8(r + 0xcd8, bits);
        t.set_u8(r + 0xbd8, bits);
        for j in 0..20u32 {
            let v = if agree {
                0x4e20
            } else if j == 7 {
                0x20
            } else {
                0xffff
            };
            t.set_u32(r + 0xd4c + 0xe4 * j, v);
        }
        // the end's nodes under a parent
        let p = SCRATCH + 0x5c00;
        t.write(p, &[0; 0x20]);
        t.set_u32(p + 0x10, p);
        t.set_u32(p + 0xc, p);
        t.set_u16(p + 0x18, 9);
        for o in [0x8a0u32, 0x608, 0x4bc, 0x140, 0x30c, 0xa8] {
            t.set_u32(r + o + 0x14, if state == 0x2710 { p } else { 0 });
            t.set_u32(r + o + 4, p);
            t.set_u32(r + o + 8, p);
        }
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the particle and effect nodes' lists: empty
        for (c, op) in [(0x4e1u32, 0x26u32), (0x4f4, 0x18)] {
            let l = t.img.word(t.img.rel32_target(f + c) + op);
            t.set_u32(l + 0x10, l);
            t.set_u32(l + 0xc, l);
        }
        // the static list: empty or with a last child
        let list = t.img.word(f + 0x402);
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        t.set_u32(list + 0x10, if k % 3 == 1 { kid } else { list });
        t.set_u32(list + 0xc, list);
        t.set_u16(list + 0x18, 2);
        // the centre's targets: three (the prime block's count would run
        // for ever)
        let centre = t.img.rel32_target(f + 0xaf4);
        let n = t.img.word(centre + 0x48);
        t.set_u32(n, 3);
        ownb::dirty_stack(&mut t, 0xb530 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0577` 0x1000c750 (`r35::burst_task_0577`): every state waiting
/// and going on (the chains 0xa -> 0x14, 0x15 -> 0x28 -> 0x2a, 0x2a ->
/// 0x1e, 0x20 -> 0x32, 0x33 -> 0x3c, 0x42 -> 10000), the clock's bounds,
/// slot 313 both ways, the parts' flags both ways, the sparks counted, the
/// end's conditions, the states past the table.
#[test]
fn burst_task_0577() {
    let Some(mut t) = twin(577) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0xc750);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let call = |t: &Twin, o: u32| t.img.rel32_target(f + o);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 313, 1, move |m, _| m.r32(answer));
    let seed = SCRATCH + 0x7e0;
    rec(&mut t, 324, 0, move |m, _| {
        let x = m.r32(seed).wrapping_mul(0x0101_0101).wrapping_add(0x2b7);
        m.w32(seed, x);
        x >> 8
    });
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 + (a[0] & 7) as f32);
        }
        0x531
    });
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    // slot 218: the actor's two model numbers
    rec(&mut t, 218, 2, |m, a| {
        m.w32(a[1], 2);
        SCRATCH + 0x700
    });
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25), (277, 4.0)] {
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    t.stub_edx(771, |_, a| 0x0077_1000 ^ a[0]);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    let m = SCRATCH + 0x5800;
    // (state, the wait, the clock, slot 313, the parts' flag bits, the
    // sparks, the end's words)
    let cases: [(u32, i32, u32, u32, u8, u32, bool); 34] = [
        (0, 0, 0, 0, 0, 2, false),
        (0xa, 0, 0, 0, 0, 2, false),
        (0x14, 0, 0, 0, 0, 2, false),
        (0x15, 2, 0, 0, 0, 2, false),
        (0x15, 0, 0, 0, 0, 2, false),
        (0x28, 0, 0, 0, 0, 2, false),
        (0x29, 0, 0, 0, 0, 2, false),
        (0x2a, 1, 0, 0, 0, 2, false),
        (0x2a, 0, 0, 0, 0, 2, false),
        (0x1f, 1, 0, 0, 0, 2, false),
        (0x1f, 0, 0, 0, 0, 2, false),
        (0x20, 1, 0, 0, 0, 2, false),
        (0x20, 0, 0, 0, 0, 2, false),
        (0x20, 0, 0, 0, 4, 2, false),
        (0x32, 0, 0xa9, 0, 0, 2, false),
        (0x32, 0, 0xaa, 0, 0, 2, false),
        (0x33, 0, 0x199, 0, 0, 2, false),
        (0x33, 0, 0x19a, 0, 0, 2, false),
        (0x3d, 0xb, 0, 0, 0, 2, false),
        (0x3d, 0xa, 0, 0, 0, 2, false),
        (0x3e, 1, 0, 0, 0, 2, false),
        (0x3e, 0, 0, 0, 0, 3, false),
        (0x3e, 0, 0, 0, 4, 0, false),
        (0x3f, 0, 0, 1, 0, 2, false),
        (0x3f, 0, 0, 0, 0, 2, false),
        (0x3f, 0, 0, 0, 4, 0, false),
        (0x40, 1, 0, 0, 0, 2, false),
        (0x40, 0, 0, 0, 4, 2, false),
        (0x41, 0, 0, 1, 0, 2, false),
        (0x41, 0, 0, 0, 0, 2, false),
        (0x42, 0, 0, 0, 0, 2, false),
        (0x2711, 0, 0, 0, 4, 2, true),
        (0x2711, 0, 0, 0, 0, 2, true),
        (0x50, 0, 0, 0, 0, 2, false),
    ];
    for (k, (state, wait, clock, ans, bits, sparks, ends)) in (0u32..).zip(cases) {
        t.write(r, &[0; 0x3c00]);
        for o in (0x1180u32..0x1300).step_by(4) {
            t.set_u32(
                r + o,
                (0.25f32 * (o % 0x40) as f32 - 3.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, 0x40 + k);
        t.set_u32(r + 0x20, wait.cast_unsigned());
        t.set_u32(r + 0x500, sparks);
        let mb: Vec<u8> = (0..0x100u32).map(|i| (i * 7 + k) as u8).collect();
        t.write(m, &mb);
        t.set_u32(r + 0x514, m);
        // the spark's block and the nodes' bytes the set-ups clear
        let sb: Vec<u8> = (0..0x80u32).map(|i| (i * 13 + k) as u8).collect();
        t.write(SCRATCH + 0x5900, &sb);
        for o in [0xc50u32, 0xd9c, 0xee8] {
            t.set_u8(r + o + 0x148, 0x5d);
            t.set_u8(r + o + 0x14c, 0x5d);
        }
        t.set_u32(r + 0x1334, SCRATCH + 0x5900);
        t.set_u32(r + 0xd90, SCRATCH + 0x5a00);
        t.set_u32(SCRATCH + 0x5a28, 0x3f40_0000);
        // the owned node's block (the body reads its `+0x1c..`)
        t.set_u32(r + 0x1034 + 0x144, SCRATCH + 0x5b00);
        for o in [0xa38u32, 0xc38] {
            t.set_u8(r + o, bits);
        }
        for o in [0x950u32, 0xb50] {
            t.set_u8(r + o + 0xe8, bits);
        }
        let end = if ends { 0x4e20 } else { 0 };
        t.set_u32(r + 0x1360, end);
        t.set_u32(r + 0x13f0, end);
        t.set_u32(w(&t, 0x91e), clock);
        t.set_u32(answer, ans);
        // the end's node under a parent
        let p = SCRATCH + 0x5c00;
        t.write(p, &[0; 0x20]);
        t.set_u32(p + 0x10, p);
        t.set_u32(p + 0xc, p);
        t.set_u16(p + 0x18, 9);
        t.set_u32(r + 0x24 + 0x14, if ends { p } else { 0 });
        t.set_u32(r + 0x24 + 4, p);
        t.set_u32(r + 0x24 + 8, p);
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the lists: the static one empty or with a last child, the
        // particle nodes' ones empty
        let list = w(&t, 0x67);
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        t.set_u32(list + 0x10, if k % 3 == 1 { kid } else { list });
        t.set_u32(list + 0xc, list);
        t.set_u16(list + 0x18, 2);
        let pl = w(&t, 0x2f0);
        t.set_u32(pl + 0x10, pl);
        t.set_u32(pl + 0xc, pl);
        for (c, op) in [(0xd42u32, 0x26u32), (0xb93, 0x18)] {
            let l = t.img.word(call(&t, c) + op);
            t.set_u32(l + 0x10, l);
            t.set_u32(l + 0xc, l);
        }
        let centre = call(&t, 0xcd0);
        let n = t.img.word(centre + 0x48);
        t.set_u32(n, 3);
        ownb::dirty_stack(&mut t, 0xc750 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0577` 0x100091d0 (`r35::scene_task_0577`): every state waiting
/// and going on (the chains 0 -> 0x64 -> 0x65, 0x65 -> 0x96 -> 0x97, 0x97
/// -> 0xa0 -> 0xa1, 0xa2 -> 0xaa -> 0xab, 0xad -> 0x19a -> 0x19c, 0x19e ->
/// 0x1f4 -> 0x1f6, 0x1f6 -> 0x258 -> 0x259, 0x259 -> 10000 -> 0x2711),
/// the fade-out's floor, the limits, the model's flag both ways, the
/// states between and past the tables.
#[test]
fn scene_task_0577() {
    let Some(mut t) = twin(577) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x91d0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let call = |t: &Twin, o: u32| t.img.rel32_target(f + o);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 + (a[0] & 7) as f32);
        }
        0x531
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25)] {
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let vt = w(&t, 0x3dd);
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    let m = SCRATCH + 0x5800;
    // (state, the wait, `R + 0x28`, the model's flag)
    let cases: [(u32, i32, f32, u8); 38] = [
        (0, 0, 1.0, 0),
        (0x64, 0, 1.0, 0),
        (0x65, 2, 1.0, 0),
        (0x65, 0, 1.0, 0),
        (0x97, 1, 1.0, 0),
        (0x97, 0, 1.0, 0),
        (0xa1, 1, 1.0, 0),
        (0xa1, 0, 1.0, 0),
        (0xa2, 1, 1.0, 0),
        (0xa2, 0, 1.0, 0),
        (0xab, 1, 1.0, 0),
        (0xab, 0, 1.0, 0),
        (0xac, 0x29, 1.0, 0),
        (0xac, 0x28, 1.0, 0),
        (0xad, 1, 1.0, 0),
        (0xad, 0, 1.0, 0),
        (0x19b, 0x28, 1.0, 0),
        (0x19b, 0x27, 1.0, 0),
        (0x19c, 0, 1.0, 0),
        (0x19d, 1, 1.0, 0),
        (0x19d, 0, 1.0, 0),
        (0x19e, 1, 1.0, 0),
        (0x19e, 0, 1.0, 0),
        (0x1f5, 0, 0.003, 0),
        (0x1f5, 0, 0.005, 0),
        (0x2710, 5, 1.0, 0),
        (0x1f5, 5, 0.5, 0),
        (0x1f6, 1, 1.0, 0),
        (0x1f6, 0, 1.0, 1),
        (0x1f6, 0, 1.0, 0),
        (0x259, 1, 1.0, 0),
        (0x259, 0, 1.0, 0),
        (0x2711, 3, 1.0, 0),
        (0x50, 0, 1.0, 0),
        (0x150, 0, 1.0, 0),
        (0x1a0, 0, 1.0, 0),
        (0x200, 0, 1.0, 0),
        (0x3000, 0, 1.0, 0),
    ];
    for (k, (state, wait, x28, flag)) in (0u32..).zip(cases) {
        t.write(r, &[0; 0x900]);
        for o in (0x20u32..0x34).step_by(4) {
            t.set_u32(
                r + o,
                (0.5f32 * (o - 0x20) as f32 - 2.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(r + 0x28, x28.to_bits());
        for o in (0x258u32..0x264).step_by(4) {
            t.set_u32(r + o, 0x700 + 0x330 * (o - 0x258) + k);
        }
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, wait.cast_unsigned());
        // the camera block's words and the record's the states clear
        for g in (0x1047_10dcu32..0x1047_112c).step_by(4) {
            t.set_u32(
                g,
                (0.375f32 * ((g >> 2) & 0x1f) as f32 - 1.0 + k as f32).to_bits(),
            );
        }
        for o in [0xc8u32, 0xcc, 0xdc, 0xe4, 0x3e0 + 0x148] {
            t.set_u32(r + o, 0x5d5d_5d5d);
        }
        let mk = w(&t, 0xf7f);
        t.set_u8(mk, 0x40 + k as u8);
        let mb: Vec<u8> = (0..0x100u32).map(|i| (i * 7 + k) as u8).collect();
        t.write(m, &mb);
        t.set_u8(m + 0x90, flag);
        t.set_u32(r + 0x838, m);
        let sb: Vec<u8> = (0..0x80u32).map(|i| (i * 13 + k) as u8).collect();
        t.write(SCRATCH + 0x5900, &sb);
        t.set_u32(r + 0x83c, SCRATCH + 0x5900);
        t.set_u32(r + 0x184, 0x0184_0000 + k);
        // the sub-walk's object: no children
        let o = r + 0x6b8;
        t.set_u32(o, class);
        for x in [4u32, 8, 0xc, 0x10] {
            t.set_u32(o + x, o);
        }
        // its object (visited through the stub)
        let ob = SCRATCH + 0x5d00;
        t.write(ob, &[0x31; 0x100]);
        t.set_u32(ob, class);
        t.set_u32(o + 0x3c, ob);
        // the sound node under a parent
        let p = SCRATCH + 0x5c00;
        t.write(p, &[0; 0x20]);
        t.set_u32(p + 0x10, p);
        t.set_u32(p + 0xc, p);
        t.set_u16(p + 0x18, 9);
        t.set_u32(r + 0x54 + 0x14, p);
        t.set_u32(r + 0x54 + 4, p);
        t.set_u32(r + 0x54 + 8, p);
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the lists: the static one empty or with a last child, the
        // particle and effect nodes' ones empty
        let list = w(&t, 0xa4);
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        t.set_u32(list + 0x10, if k % 3 == 1 { kid } else { list });
        t.set_u32(list + 0xc, list);
        t.set_u16(list + 0x18, 2);
        for (c, op) in [(0x1036u32, 0x26u32), (0x11dd, 0x18)] {
            let l = t.img.word(call(&t, c) + op);
            t.set_u32(l + 0x10, l);
            t.set_u32(l + 0xc, l);
        }
        let n = t.img.word(call(&t, 0xa0e) + 0x48);
        t.set_u32(n, 3);
        ownb::dirty_stack(&mut t, 0x91d0 + k, &[]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}

/// `magic_0533` 0x10008490 (`r35::summon_task_0533`): every state of the
/// DLL's state global (waiting and going on where they wait), slot 520's
/// answer both ways, the fade byte above and below 2, the count reaching
/// 0x80, 10000, the states past the table and a negative one.
#[test]
fn summon_task_0533() {
    let Some(mut t) = twin(533) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0x8490);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 520, 0, move |m, _| m.r32(answer));
    rec(&mut t, 557, 0, |_, _| 0x5571);
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [(12u32, -1.5f64), (333, 0.75), (282, 2.25), (307, 0.625)] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    rec(&mut t, 307, 0, |_, _| 0x307);
    prime(&mut t, f, SCRATCH + 0x3_8000);
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    let gs = w(&t, 0x10);
    // (state, `R + 0x20`, `R + 0x1c`, slot 520's answer, the byte `R +
    // 0x390`, `R + 0x310`)
    let cases: [(u32, u32, f32, u32, u8, u32); 42] = [
        (0, 0, 0.0, 0, 0, 0),
        (1, 3, 0.0, 0, 0, 0),
        (1, 0x27, 0.0, 0, 0, 0),
        (0x51, 0, 0.0, 0, 0, 0),
        (0x5a, 0, 0.0, 0, 0, 0),
        (0x5b, 0, 0.0, 0, 0, 0),
        (0x5c, 0, -100.0, 0, 0, 0),
        (0x5c, 0, 0.0, 0, 0, 0),
        (0x5c, 0, 1000.0, 0, 0, 0),
        (0xa, 0, 0.0, 0, 0, 0),
        (0xb, 0, 0.0, 0, 0, 0),
        (0x64, 0, 0.0, 0x024b_0005, 5, 3),
        (0x64, 0, 0.0, 7, 1, 0x7e),
        (0x65, 0, 0.0, 0, 9, 3),
        (0x65, 0, 0.0, 0, 2, 0x7e),
        (0x65, 0, 0.0, 0, 0, 3),
        (0x66, 0, 0.0, 0, 0, 0x7f),
        (0x66, 0, 0.0, 0, 0, 3),
        (0x14, 0, 0.0, 0, 0, 0),
        (0x1e, 0, 0.0, 0, 0, 0),
        (0x1f, 0, 0.0, 0, 0, 0),
        (0x28, 0, 0.0, 0, 0, 0),
        (0x29, 0, 0.0, 0, 0, 0),
        (0x33, 0, 0.0, 0, 0, 0),
        (0x32, 0, 0.0, 0, 0, 0),
        (0x3c, 0, 0.0, 0, 0, 0),
        (0x3d, 0, 0.0, 0, 0, 0),
        (0x3e, 0, 0.0, 0, 0, 0),
        (0x3f, 0, 0.0, 0, 0, 0),
        (0x40, 0, 0.0, 0, 0, 0),
        (0x46, 0, 0.0, 0, 0, 0),
        (0x47, 0, -100.0, 0, 0, 0),
        (0x47, 0, 1000.0, 0, 0, 0),
        (0x50, 0, 0.0, 0, 0, 0),
        (0x6e, 0, 0.0, 0, 0, 0),
        (0x6f, 0, 0.0, 0, 0, 0),
        (0x70, 3, 0.0, 0, 0, 0),
        (0x70, 0x4a, 0.0, 0, 0, 0),
        (0x2710, 0, 0.0, 0, 0, 0),
        (0x2711, 0, 0.0, 0, 0, 0),
        (5, 0, 0.0, 0, 0, 0),
        (0xffff_fff0, 0, 0.0, 0, 0, 0),
    ];
    for (k, (state, n20, x1c, ans, b390, n310)) in (0u32..).zip(cases) {
        t.write(r, &[0; 0x400]);
        for o in (0x18u32..0x30).step_by(4) {
            t.set_u32(
                r + o,
                (0.5f32 * (o - 0x18) as f32 - 1.0 + k as f32).to_bits(),
            );
        }
        for o in (0x330u32..0x370).step_by(4) {
            t.set_u32(
                r + o,
                (0.25f32 * (o - 0x330) as f32 - 2.0 + k as f32).to_bits(),
            );
        }
        t.set_u32(r + 0x1c, x1c.to_bits());
        t.set_u32(r + 0x20, n20);
        t.set_u8(r + 0x390, b390);
        // words the states clear, the flag globals they set bits in
        t.set_u32(r + 0x2e4, 0x5d5d_5d5d);
        t.set_u32(r + 0x398, 0x5d5d_5d5d);
        for op in [0x3fau32, 0xc72] {
            let g = w(&t, op);
            t.set_u32(g, 0x5d5d_5d50 + k);
        }
        t.set_u32(r + 0x310, n310);
        t.set_u32(gs, state);
        // the parent node inside the record (state 0xb's append)
        let pn = r + 0x2c4;
        t.set_u32(pn + 0x10, pn);
        t.set_u32(pn + 0xc, pn);
        t.set_u32(answer, ans);
        // the link node under a parent (10000)
        let p = SCRATCH + 0x5c00;
        t.write(p, &[0; 0x20]);
        t.set_u32(p + 0x10, p);
        t.set_u32(p + 0xc, p);
        t.set_u16(p + 0x18, 9);
        t.set_u32(r + 0x374 + 0x14, p);
        t.set_u32(r + 0x374 + 4, p);
        t.set_u32(r + 0x374 + 8, p);
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the parents: empty or with a last child
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        for op in [0x97u32, 0x198, 0x2b0, 0xabb] {
            let l = w(&t, op);
            t.set_u32(l + 0x10, if k % 3 == 1 { kid } else { l });
            t.set_u32(l + 0xc, l);
            t.set_u16(l + 0x18, 2);
        }
        ownb::dirty_stack(&mut t, 0x8490 + k, &[]);
        let call = Call {
            eax: 0x00ea_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, call, &[a], Eax::Checked);
    }
}

/// `magic_0570` 0x1000b3e0 (`r35::summon_task_0570`): every state waiting
/// and going on (the chains 0x3e -> 0x32 -> 0x28 -> 0x29 -> 0x46, 0x68 ->
/// 0x5a, 0x60 -> 10000), slots 313 and 411 both ways, the slot-366 flag
/// both ways, the parts' bit 0 both ways, 10000, 0x2711, the end and the
/// states past the table.
#[test]
fn summon_task_0570() {
    let Some(mut t) = twin(570) else { return };
    all_stubs(&mut t);
    summon_stubs::float_matrix_stubs(&mut t);
    summon_stubs::vector_stubs(&mut t);
    summon_stubs::summon_stubs(&mut t);
    let f = t.at(0xb3e0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let call = |t: &Twin, o: u32| t.img.rel32_target(f + o);
    let answer = SCRATCH + 0x7f0;
    rec(&mut t, 313, 1, move |m, _| m.r32(answer));
    rec(&mut t, 411, 1, move |m, a| m.r32(answer) ^ a[0]);
    rec(&mut t, 538, 2, |_, a| 0x0538_0000 ^ a[0]);
    // slot 255 keeps the vector it is given
    rec(&mut t, 255, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(SCRATCH + 0x7c0 + 4 * i, x);
        }
        0x255
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, 0.25 * i as f32 - 1.0 + (a[1] & 3) as f32);
        }
        0x230
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x ^ 0x10);
        }
        0x0325_0000 ^ a[0]
    });
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            wf(m, a[3] + 4 * i, 0.5 * i as f32 + (a[1] & 7) as f32);
        }
        0x237
    });
    rec(&mut t, 558, 1, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 * i + 7);
        }
        0x558
    });
    t.stub_st0(790, |_, _| 0.375);
    for (s, x) in [
        (12u32, -1.5f64),
        (333, 0.75),
        (282, 2.25),
        (277, 4.0),
        (307, 0.625),
    ] {
        t.stub_st0(s, move |_, a| {
            x + f64::from(a.first().copied().unwrap_or(0) & 3)
        });
    }
    prime(&mut t, f, SCRATCH + 0x3_8000);
    // the owned nodes' vtable visits through a stub; the bone's table
    let (visit, class) = (1000u32, SCRATCH + 0x3700);
    visit_stub(&mut t, visit, class);
    let vt = t.img.word(call(&t, 0x54f) + 5);
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * visit);
    let bone = t.img.rel32_target(call(&t, 0xb04) + 0x15);
    let table = t.img.word(bone + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    let (a, r) = (SCRATCH + 0x100, SCRATCH + 0x1_0000);
    let (node, n0, n1) = (SCRATCH + 0x3600, SCRATCH + 0x3680, SCRATCH + 0x36c0);
    // `E + 0x44`, read before written (state 0x65): 0 as the port
    let base = ((ownb::ENTRY_ESP - 4) & !7) - 0x98;
    let junk_at = ownb::ENTRY_ESP - (base + 0x44);
    // (state, the wait `R + 0x2c`, the slots' answer, the flag byte, the
    // parts' bit 0)
    let cases: [(u32, i32, u32, u8, u8); 66] = [
        (0, 0, 0, 0, 0),
        (0xa, 0, 0, 0, 0),
        (0xb, 0, 0, 0, 0),
        (0xc, 9, 0, 0, 0),
        (0xc, 0xa, 0, 0, 0),
        (0xc, 0xa, 0, 1, 1),
        (0x1e, 0x4f, 0, 0, 0),
        (0x1e, 0x50, 0, 0, 0),
        (0x1f, 0x4f, 0, 0, 0),
        (0x1f, 0x50, 0, 0, 0),
        (0x20, 0x59, 0, 0, 0),
        (0x20, 0x5a, 0, 0, 0),
        (0x3c, 0x74, 0, 0, 0),
        (0x3c, 0x75, 0, 0, 0),
        (0x3d, 6, 0, 0, 0),
        (0x3d, 7, 0, 0, 0),
        (0x3e, 7, 0, 0, 0),
        (0x3e, 8, 0, 0, 0),
        (0x3e, 8, 3, 0, 0),
        (0x32, 0, 0, 0, 0),
        (0x33, 0, 5, 0, 0),
        (0x34, 0, 0, 0, 0),
        (0x28, 0, 1, 0, 0),
        (0x28, 0, 0, 0, 0),
        (0x29, 0x30, 0, 0, 0),
        (0x46, 0x21, 0, 0, 0),
        (0x46, 0x22, 0, 0, 0),
        (0x47, 0, 0, 0, 0),
        (0x47, 0, 0, 0, 1),
        (0x48, 0x73, 0, 0, 0),
        (0x48, 0x74, 0, 0, 0),
        (0x49, 0x73, 0, 0, 0),
        (0x49, 0x74, 0, 0, 0),
        (0x14, 0, 0, 0, 0),
        (0x15, 1, 0, 0, 0),
        (0x15, 2, 0, 0, 0),
        (0x16, 0x58, 0, 0, 0),
        (0x16, 0x59, 0, 0, 0),
        (0x64, 0, 0, 0, 0),
        (0x65, 0, 0, 0, 0),
        (0x65, 1, 0, 0, 0),
        (0x65, 1, 0, 0, 1),
        (0x66, 0xb, 0, 0, 0),
        (0x66, 0xc, 0, 0, 0),
        (0x67, 0xb, 0, 0, 0),
        (0x67, 0xc, 0, 0, 0),
        (0x68, 0x1d, 0, 0, 0),
        (0x68, 0x1e, 0, 0, 0),
        (0x60, 0x7c, 0, 0, 0),
        (0x60, 0x7d, 0, 0, 0),
        (0x50, 0, 0, 0, 0),
        (0x51, 0, 0, 0, 0),
        (0x51, 0, 0x40, 0, 0),
        (0x5a, 0, 0, 0, 0),
        (0x5b, 0, 0, 0, 0),
        (0x5b, 0, 0, 0, 1),
        (0x5c, 0x1f, 0, 0, 0),
        (0x5c, 0x20, 0, 0, 0),
        (0x5d, 0x34, 0, 0, 0),
        (0x5d, 0x35, 0, 0, 0),
        (0x5e, 0x3b, 0, 0, 0),
        (0x5e, 0x3c, 0, 0, 0),
        (0x5f, 0x63, 0, 0, 0),
        (0x5f, 0x64, 0, 0, 0),
        (0x2712, 0, 0, 0, 1),
        (0x2712, 0, 0, 0, 0),
    ];
    let more: [(u32, i32, u32, u8, u8); 6] = [
        (0x2710, 0, 0, 0, 0),
        (0x2711, 0, 0, 0, 0),
        (0x2713, 0, 0, 0, 0),
        (0x100, 0, 0, 0, 0),
        (5, 0, 0, 0, 0),
        (0xffff_fff0, 0, 0, 0, 0),
    ];
    for (k, (state, wait, ans, fl, bit)) in (0u32..).zip(cases.into_iter().chain(more)) {
        t.write(r, &[0; 0x100]);
        t.set_u32(r + 0x18, state);
        set_f(&mut t, r + 0x1c, 0.75 + k as f32);
        set_f(&mut t, r + 0x20, -0.5 * k as f32);
        t.set_u32(r + 0x28, 0x100 + k);
        t.set_u32(r + 0x2c, wait.cast_unsigned());
        t.set_u32(answer, ans);
        t.set_u8(w(&t, 0x5d9), fl);
        // the flag words the states set and clear bits of
        for (i, o) in (0u32..).zip([
            0xccu32, 0x13b, 0x356, 0x3b1, 0x82b, 0xbfc, 0xc03, 0xcc3, 0xcca, 0xdc3, 0x110e, 0x119b,
        ]) {
            t.set_u32(w(&t, o), 0x5a5a_0f00 ^ (k << 4) ^ i);
        }
        for o in [0x251u32, 0x25f, 0x284, 0x292, 0xcae, 0xf67] {
            t.set_u8(w(&t, o) + 0x90, bit);
        }
        t.set_u32(a, node);
        t.set_u32(a + 4, r);
        for (x, next, prev) in [(node, n0, n1), (n0, n1, node), (n1, node, n0)] {
            t.set_u32(x + 4, next);
            t.set_u32(x + 8, prev);
        }
        // the DLL's list empty or with a last child; the particle and
        // effect nodes' lists empty
        let list = w(&t, 0x4af);
        let kid = SCRATCH + 0x5e00;
        t.write(kid, &[0x21; 0x40]);
        t.set_u32(list + 0x10, if k % 3 == 1 { kid } else { list });
        t.set_u32(list + 0xc, list);
        t.set_u16(list + 0x18, 2);
        for (c, op) in [(0x720u32, 0x26u32), (0x351, 0x1e)] {
            let l = t.img.word(call(&t, c) + op);
            t.set_u32(l + 0x10, l);
            t.set_u32(l + 0xc, l);
        }
        // the bone's node (an owned node of state 0x1e): its vtable, empty
        // lists, its three objects
        let bn = w(&t, 0xaf8);
        t.write(bn, &[0; 0x150]);
        t.set_u32(bn, vt);
        t.set_u32(bn + 0x10, bn);
        t.set_u32(bn + 0xc, bn);
        t.set_u8(bn + 0x148, 1);
        for (i, o) in (0u32..).zip([0x13cu32, 0x140, 0x144]) {
            let obj = SCRATCH + 0x6000 + 0x100 * i;
            t.write(obj, &[0; 0x100]);
            t.set_u32(obj + 0x10, obj);
            t.set_u32(obj + 0xc, obj);
            set_f(&mut t, obj + 0x28, 0.5);
            t.set_u32(bn + o, obj);
        }
        ownb::dirty_stack(&mut t, 0xb3e0 + k, &[junk_at]);
        t.check_entry(f, Call::default(), &[a], Eax::Checked);
    }
}
