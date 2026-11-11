//! Round 49's ports (`ffx_magic::r49`) against each original. Needs
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
mod magic_h6;
mod magic_hi;
mod magic_owna;
mod magic_r11;
#[path = "magic/ownb.rs"]
mod ownb;
#[path = "magic/sa.rs"]
mod sa;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use ownb::rec;

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

fn setf(t: &mut Twin, at: u32, v: f32) {
    t.set_u32(at, v.to_bits());
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// `n` floats at `at`: `x0 + 0.25 i` (no NaNs).
fn floats(t: &mut Twin, at: u32, n: u32, x0: f32) {
    for i in 0..n {
        t.set_u32(at + 4 * i, (x0 + 0.25 * i as f32).to_bits());
    }
}

/// The matrix and vector slots as data flow: 235 a unit matrix, 328 and
/// 329 a turn by their angle, 229 `out = m + 2 v` (row 0), 248 `a = b +
/// c`; 262 a point from the handle, 268 a vector from the handle and
/// its word. Each logs its words.
fn vu0_stubs(t: &mut Twin, k: u32) {
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i) + 0.125 * i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    rec(t, 262, 4, move |m, a| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        for i in 0..4u32 {
            let x = rf(m, a[2] + 4 * i) * 0.5;
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32 + x;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
}

/// What `magic_0396`'s curve start-ups (`f`: 0x10005740 or 0x10006eb0)
/// need: the summon `S` (its point, yaw and records junked), the word
/// block, the object starts, the selected actor's handle, the part table,
/// the free ring (`free` nodes) and the task list (empty or not).
fn curves_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (yaw, free, listed): (f32, u16, bool),
    ops: &CurvesOps,
) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    all_stubs(t);
    magic_r11::object_slots(t);
    for &c in ops.starts {
        let start = t.img.rel32_target(f + c);
        magic_r11::object_start_auto(t, start);
    }
    vu0_stubs(t, k);
    let s = SCRATCH + 0x1_0000;
    let at = w(t, ops.summon);
    t.set_u32(at, s);
    junk(t, s + 0xb00, 0x1a00, k);
    floats(t, s + 0xbe0, 4, k as f32 - 1.5);
    setf(t, s + 0x2450, yaw);
    let blk = SCRATCH + 0x7a00;
    junk(t, blk, 0x20, k + 9);
    let at = w(t, ops.word);
    t.set_u32(at, blk);
    // the selected actor and the handles
    let sel = SCRATCH + 0x7b00;
    t.set_u8(sel, (2 + k) as u8);
    let at = w(t, ops.sel);
    t.set_u32(at, sel);
    let tab = w(t, ops.handles);
    for i in 0..8 {
        t.set_u32(tab + 4 * i, 0x0000_0040 + 0x11 * i + k);
    }
    // the part table
    let parts = w(t, ops.parts);
    for i in 0..4 {
        t.set_u32(parts + 4 * i, 0x0bad_0000 + i + k);
    }
    // the free ring and the task list
    let (count, head) = (w(t, ops.ring.0), w(t, ops.ring.1));
    magic_r11::ring(t, count, head, free, SCRATCH + 0xd800);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    let (first, last) = (w(t, ops.list.0), w(t, ops.list.1));
    t.set_u32(first, if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(last, SCRATCH + 0x600);
    magic_h6::scrub(t);
}

/// Summon yaws `Y` whose `Y - K` (operands `a`, `hi`, `lo` of `f`)
/// stays, wraps down, wraps up, or lands on either limit (where an
/// `f32` reaches it exactly: not for `magic_0396`'s `K`).
fn yaw_cases(t: &Twin, f: u32, a: u32, hi: u32, lo: u32) -> Vec<f32> {
    let k = |op: u32| f32::from_bits(t.u32(t.img.word(f + op)));
    let (a, hi, lo) = (k(a), k(hi), k(lo));
    let mut v = vec![a + 1.0, a + 4.0, a - 4.0, a - 3.0];
    for limit in [hi, lo] {
        let guess = (a + limit).to_bits();
        if let Some(y) = (0..256u32)
            .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
            .map(f32::from_bits)
            .find(|&y| (y - a).to_bits() == limit.to_bits())
        {
            v.push(y);
        }
    }
    v
}

/// The operands [`curves_setup`] reads its globals through.
struct CurvesOps {
    starts: &'static [u32],
    summon: u32,
    word: u32,
    sel: u32,
    handles: u32,
    parts: u32,
    ring: (u32, u32),
    list: (u32, u32),
}

const CURVES_5740: CurvesOps = CurvesOps {
    starts: &[0x118, 0x635, 0xb51, 0x106f],
    summon: 0x1b,
    word: 0x14,
    sel: 0x261,
    handles: 0x26e,
    parts: 0x15cc,
    ring: (0x165b, 0x167a),
    list: (0x16e1, 0x16f9),
};

/// `magic_0396` 0x10005740 (`r49::curves_start_0396`, internal): the yaw
/// wrapping both ways and not, the ring with more nodes than records and
/// with exactly four, the task list empty and not.
#[test]
fn curves_start_0396() {
    let Some(mut t) = sa::twin(396) else { return };
    let f = t.at(0x5740);
    let yaws = yaw_cases(&t, f, 0x2e, 0x3d, 0x59);
    assert!(yaws.len() >= 4, "the yaw cases");
    let rings = [(8u16, false), (4, true), (6, true), (5, false)];
    for (k, &yaw) in (0u32..).zip(&yaws) {
        let (free, listed) = rings[k as usize % rings.len()];
        curves_setup(&mut t, f, k, (yaw, free, listed), &CURVES_5740);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::r49::curves_start_0396(g, img, fr))
        });
    }
}

const CURVES_6EB0: CurvesOps = CurvesOps {
    starts: &[0xf6],
    summon: 0x1b,
    word: 0x14,
    sel: 0x251,
    handles: 0x25e,
    parts: 0xd01,
    ring: (0xd80, 0xdba),
    list: (0xe21, 0xe39),
};

/// `magic_0396` 0x10006eb0 (`r49::curves_start2_0396`, an entry): the yaw
/// wrapping both ways and not, the ring with more nodes than records,
/// and exactly three (with none left the original writes through node 0),
/// the task list empty and not (each of `eax`'s three sources).
#[test]
fn curves_start2_0396() {
    let Some(mut t) = sa::twin(396) else { return };
    let f = t.at(0x6eb0);
    let yaws = yaw_cases(&t, f, 0x2e, 0x3d, 0x59);
    assert!(yaws.len() >= 4, "the yaw cases");
    let rings = [(8u16, false), (3, true), (5, false), (6, true)];
    for (k, &yaw) in (0u32..).zip(&yaws) {
        let (free, listed) = rings[k as usize % rings.len()];
        curves_setup(&mut t, f, k, (yaw, free, listed), &CURVES_6EB0);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0396` 0x10007d50 (`r49::curves_0396`, a table entry): `*G_ON`
/// set (the four curves and the fade, the task list empty and not) and 0
/// (the tail jump to the three curves).
#[test]
fn curves_0396() {
    let Some(mut t) = sa::twin(396) else { return };
    let f = t.at(0x7d50);
    let img = t.img.clone();
    let start = img.rel32_target(f + 0x11);
    let start2 = img.rel32_target(f + 0x48);
    let fade = img.rel32_target(f + 0x33);
    let take = img.rel32_target(fade + 0x75);
    let yaws = yaw_cases(&t, start, 0x2e, 0x3d, 0x59);
    for (k, (on, free, listed)) in (0u32..).zip([
        (1u32, 8u16, false),
        (1, 7, true),
        (0, 6, true),
        (0, 8, false),
    ]) {
        let yaw = yaws[k as usize % yaws.len()];
        let (at, ops) = if on == 0 {
            (start2, &CURVES_6EB0)
        } else {
            (start, &CURVES_5740)
        };
        curves_setup(&mut t, at, k, (yaw, free, listed), ops);
        // the fade's ring (when its globals are not the curves')
        let count = img.word(take + 0x04);
        if count != img.word(at + ops.ring.0) {
            magic_r11::ring(&mut t, count, img.word(take + 0x17), 4, SCRATCH + 0x5400);
        }
        t.set_u32(img.word(f + 0x2), on);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The stack word `S0 + 0x1c` of `r49::curves_task_0396`'s frame (the
/// point's never-written `w`), below the entry `esp`.
fn task_junk_word() -> u32 {
    let e = magic::STACK_TOP - 0x100;
    let b0 = ((e - 4) & !7) - 0x5c;
    e - (b0 - 0xc + 0x1c)
}

fn fold(m: &mut dyn StubMem, slot: u32, i: u32, v: u32) {
    let at = SCRATCH + 0x2_0000 + 0x40 * (slot % 0x400) + 4 * i;
    let x = m.r32(at).rotate_left(5) ^ v;
    m.w32(at, x);
}

/// The trail draw's slots and globals for the trail draw at `td` (as
/// `magic_r29.rs`'s).
fn trail_stubs(t: &mut Twin, td: u32) {
    let lock = SCRATCH + 0x9000;
    t.set_u32(t.img.word(td + 0x463), lock);
    t.set_u32(t.img.word(td + 0x520), 0x0cc0_0cc0);
    rec(t, 234, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..3 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
            fold(m, 248, i, v.to_bits());
        }
        0x248
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) * 0.5 + rf(m, a[1] + 4 * i) * 0.25 + 0.5 + i as f32;
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    let gp = t.img.word(td + 0x63);
    rec(t, 772, 2, move |m, a| {
        let sm = m.r32(gp);
        for j in 0..12 {
            let v = m.r32(sm + 4 * j);
            fold(m, 772, 3 + j, v);
        }
        0x7720_0000 ^ a[1]
    });
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(t, 773, 4, |_, a| (a[3] >> 1) & 1);
    for s in [774u32, 775, 778, 779] {
        rec(t, s, 2, move |_, a| s ^ a[0]);
    }
    rec(t, 776, 1, |_, a| a[0] << 4);
    rec(t, 777, 1, |_, a| 0x0ff0 ^ a[0]);
}

/// The trail block at `obj` (`n` sprites).
fn trail_block(t: &mut Twin, obj: u32, n: u32, k: u32) {
    t.write(obj, &[0x3c; 0x230]);
    t.set_u32(obj + 4, 1);
    t.set_u16(obj + 8, (k % 3) as u16);
    t.set_u16(obj + 0xa, 0x100);
    for j in 0..32u32 {
        let r = 1.0 + j as f32 * 0.3;
        let a = j as f32 * 0.7;
        for (i, x) in (0u32..).zip([r * a.cos(), j as f32 * 0.2, r * a.sin(), 1.0]) {
            t.set_u32(obj + 0xc + 16 * j + 4 * i, x.to_bits());
        }
    }
    t.write(obj + 0x20c, &[10, 200, 0, 0x80, 250, 3, 128, 0x40]);
    for (off, x) in [(0x214u32, 1.5f32), (0x218, 0.25), (0x21c, 0.75)] {
        t.set_u32(obj + off, x.to_bits());
    }
    t.set_u32(obj + 0x220, 3 + k);
    t.set_u32(obj + 0x224, n);
}

/// One case of `r49::curves_task_0396`: the wait, the step `c`, the
/// trail count, the flag words, whether the record is in the object
/// list (and at its head), whether the point sits on `R + 0xd0`.
#[derive(Clone, Copy)]
struct TaskCase {
    wait: i32,
    c: i32,
    count: i32,
    flags: (u32, u32),
    listed: Option<bool>,
    on_point: bool,
}

/// `magic_0396` 0x10005380 (`r49::curves_task_0396`, cdecl `(node)`):
/// the wait running; the step running with each flag (the aim with the
/// point away and on `R + 0xd0`, the trail); the step reaching 0 with
/// each flag (the record in the object list at its head, inside, or not;
/// the finish callback or none; the node kept or given back); the step
/// spent with the trail count above and at 1 (given back), flag 2 and 0.
/// `eax`: the task runner (0x10004353) overwrites it, so it is not
/// compared.
#[test]
fn curves_task_0396() {
    let Some(mut t) = sa::twin(396) else { return };
    let f = t.at(0x5380);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let base = TaskCase {
        wait: 0,
        c: 10,
        count: 6,
        flags: (0, 0),
        listed: None,
        on_point: false,
    };
    let cases = [
        TaskCase { wait: 3, ..base },
        base,
        TaskCase {
            flags: (2, 0),
            ..base
        },
        TaskCase {
            c: 7,
            flags: (2, 1),
            on_point: true,
            ..base
        },
        TaskCase {
            c: 1,
            flags: (2, 1),
            listed: Some(false),
            ..base
        },
        TaskCase {
            c: 1,
            flags: (0, 0),
            ..base
        },
        TaskCase {
            c: 0,
            flags: (1, 1),
            listed: Some(true),
            ..base
        },
        TaskCase {
            c: 0,
            flags: (2, 0),
            listed: Some(true),
            ..base
        },
        TaskCase {
            c: 50,
            flags: (1, 0),
            ..base
        },
        TaskCase {
            c: -1,
            count: 5,
            flags: (2, 0),
            ..base
        },
        TaskCase {
            c: -1,
            count: 1,
            flags: (2, 0),
            ..base
        },
        TaskCase {
            c: -1,
            count: 4,
            flags: (0, 0),
            ..base
        },
        TaskCase {
            c: -3,
            count: 2,
            flags: (1, 0),
            ..base
        },
    ];
    for (k, cs) in (0u32..).zip(cases) {
        all_stubs(&mut t);
        magic_r11::object_slots(&mut t);
        let start = img.rel32_target(f + 0x31a);
        magic_r11::object_start_auto(&mut t, start);
        trail_stubs(&mut t, img.rel32_target(f + 0x286));
        rec(&mut t, 235, 1, |m, a| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
            }
            0x235
        });
        rec(&mut t, 713, 1, |m, a| {
            let x = m.r32(a[0] + 0x124);
            m.w32(SCRATCH + 0x7e00, x);
            0x713
        });
        // the record
        let r = SCRATCH + 0x1_1000;
        junk(&mut t, r, 0x540, k);
        trail_block(&mut t, r + 0x260, 3 + k % 4, k);
        for i in 0..7u32 {
            for j in 0..4u32 {
                let x = 1.5 * i as f32 - 0.75 * j as f32 + 0.125 * k as f32;
                setf(&mut t, r + 0x488 + 0x10 * i + 4 * j, x);
            }
        }
        floats(&mut t, r + 0xc0, 4, -0.5);
        floats(&mut t, r + 0xd0, 4, 2.0 + k as f32);
        t.set_u32(r + 0x528, cs.wait.cast_unsigned());
        t.set_u32(r + 0x52c, cs.c.cast_unsigned());
        t.set_u32(r + 0x484, cs.count.cast_unsigned());
        t.set_u32(r + 0x530, cs.flags.0);
        t.set_u32(r + 0x534, cs.flags.1);
        if cs.on_point {
            // every key 0: the sample is 0 on each axis, as is `R + 0xd0`
            for i in 0..7u32 {
                for j in 0..4u32 {
                    t.set_u32(r + 0x488 + 0x10 * i + 4 * j, 0);
                }
            }
            for j in 0..3u32 {
                t.set_u32(r + 0xd0 + 4 * j, 0);
            }
        }
        // the node in a sibling list, and the pool's globals
        let node = SCRATCH + 0x9800;
        let (prev, next) = (SCRATCH + 0x9900, SCRATCH + 0x9a00);
        junk(&mut t, node, 0x20, k + 3);
        junk(&mut t, prev, 0x20, k + 4);
        junk(&mut t, next, 0x20, k + 5);
        t.set_u32(node, prev);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, node);
        t.set_u32(node + 0xc, 0);
        t.set_u32(node + 0x14, r);
        let gb = img.rel32_target(f + 0x38d);
        let gw = |o: u32| img.word(gb + o);
        magic_r11::ring(&mut t, gw(0x8e), gw(0x74), 3, SCRATCH + 0xd800);
        for o in [0x48u32, 0x55, 0x64] {
            t.set_u32(gw(o), if k % 2 == 0 { node } else { prev });
        }
        // the object list: two nodes, the record's at the head, inside or
        // nowhere
        let list = w(0x29d);
        let (n1, n2, n3) = (SCRATCH + 0x9b00, SCRATCH + 0x9b20, SCRATCH + 0x9b40);
        for (n, p, x, item) in [
            (
                n1,
                0,
                n2,
                if cs.listed == Some(true) {
                    r
                } else {
                    0x0101_0101
                },
            ),
            (
                n2,
                n1,
                n3,
                if cs.listed == Some(false) {
                    r
                } else {
                    0x0202_0202
                },
            ),
            (n3, n2, 0, 0x0303_0303),
        ] {
            t.set_u32(n, p);
            t.set_u32(n + 4, x);
            t.set_u32(n + 8, item);
        }
        t.set_u32(list, n1);
        t.set_u32(list + 4, n3);
        t.set_u32(list + 8, n2);
        t.set_u32(list + 0xc, 3);
        t.set_u32(w(0x2ab), SCRATCH + 0x7c00);
        ownb::dirty_stack(&mut t, 0x5380 + k, &[task_junk_word()]);
        t.check_entry(f, Call::default(), &[node], Eax::Junk);
    }
}
