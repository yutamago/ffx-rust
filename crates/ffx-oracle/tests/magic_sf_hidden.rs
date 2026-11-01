//! The functions the first overlay census hid inside merged rows
//! (`ffx_magic::sf_hidden`, census2 2026-11-01) against every DLL
//! carrying each. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use magic::{Call, Eax, SCRATCH, STACK_TOP, StubMem, Twin};
use sa::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// The word at `f + off` of the code.
fn op(t: &Twin, f: u32, off: u32) -> u32 {
    t.img.word(f + off)
}

fn varied(n: u32, seed: u32) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + seed * 13 + 1) as u8).collect()
}

/// The frame below the entry's stack pointer filled with a pattern.
fn scrub(t: &mut Twin) {
    let top = STACK_TOP - 0x100;
    t.write(top - 0x3000, &varied(0x3000, 0x5b));
}

/// The port bound at `f`, checked to be `name`.
fn bound(t: &Twin, f: u32, name: &str) {
    assert_eq!(
        ffx_magic::bind(&t.img, f).map(|i| ffx_magic::PORTS[i].0),
        Some(name),
        "magic_{:04} {f:#x}",
        t.img.number
    );
}

/// A stub that copies the four words at `a[i]` into its row (each call
/// its own row) and writes four words of its own to `a[out]`.
fn rec_point(t: &mut Twin, slot: u32, n: usize, read: Option<usize>, out: usize) {
    let row = SCRATCH + 0x3_0000 + 0x100 * (slot % 0x100);
    rec(t, slot, n, move |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        if let Some(i) = read {
            for j in 0..4 {
                let w = m.r32(a[i] + 4 * j);
                m.w32(row + 4 + 0x10 * (k % 8) + 4 * j, w);
            }
        }
        for j in 0..4 {
            m.w32(a[out] + 4 * j, (slot << 16) ^ (k << 8) ^ j);
        }
        0x5100_0000 ^ slot ^ (k << 12)
    });
}

/// `magic_0314` 0x10001740: the element's point by the part's table
/// index.
#[test]
fn elem_point_table() {
    let Some(mut t) = twin(314) else { return };
    let f = t.at(0x1740);
    bound(&t, f, "sf elem point table");
    t.set_u32(op(&t, f, 0x26), 0x0c0c_0314);
    rec_point(&mut t, 237, 4, Some(2), 3);
    let p = SCRATCH + 0x1000;
    for k in 0..3u32 {
        t.write(p, &varied(0x140, k));
        t.set_u32(p + 0x120, [0, 3, 0x11][k as usize]);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[p], Eax::Checked);
    }
}

/// `magic_0367` 0x100013a0: the part on its character's matrix.
#[test]
fn optpos_point() {
    let Some(mut t) = twin(367) else { return };
    let f = t.at(0x13a0);
    bound(&t, f, "sf optpos point");
    t.set_u32(op(&t, f, 0x1f), 0x0c0c_0367);
    rec_point(&mut t, 621, 4, None, 3);
    let p = SCRATCH + 0x1000;
    for k in 0..2u32 {
        t.write(p, &varied(0x140, k));
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[p], Eax::Checked);
    }
}

/// `magic_0274` 0x10003760: the element's point with the fixed offset in
/// the frame (the stub reads it there).
#[test]
fn elem_point_offset() {
    let Some(mut t) = twin(274) else { return };
    let f = t.at(0x3760);
    bound(&t, f, "sf elem point offset");
    t.set_u32(op(&t, f, 0x29), 0x0c0c_0274);
    rec_point(&mut t, 237, 4, Some(2), 3);
    let p = SCRATCH + 0x1000;
    for k in 0..2u32 {
        t.write(p, &varied(0x140, k));
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[p], Eax::Checked);
    }
}

fn set_f(t: &mut Twin, a: u32, x: f32) {
    t.set_u32(a, x.to_bits());
}

/// `magic_0018` 0x10001b00 and `magic_0019` 0x10004010: the three angles
/// wrapped down, up, kept, and one a quiet NaN.
#[test]
fn spin_angles() {
    for (n, rva) in [(18u16, 0x1b00u32), (19, 0x4010)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        bound(&t, f, "sf spin angles");
        let chrs = SCRATCH + 0x4000;
        t.set_u32(op(&t, f, 0x24), chrs);
        let p = SCRATCH + 0x1000;
        for (k, (a, c)) in [
            ([3.0f32, -3.0, 0.5], [0.5f32, -0.5, 0.1]),
            ([-0.25, 6.0, -6.0], [0.0, 1.0, -1.0]),
            ([f32::NAN, 3.1, -3.1], [1.0, 0.05, -0.05]),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.write(p, &varied(0x3a0, k));
            t.set_u32(p + 0x390, k);
            let c0 = chrs + 0x4f0 * k;
            t.write(c0, &varied(0x4f0, k + 3));
            for j in 0..3 {
                set_f(&mut t, p + 0xc0 + 4 * j as u32, a[j]);
                set_f(&mut t, c0 + 0x4d0 + 4 * j as u32, c[j]);
            }
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[p], Eax::Checked);
        }
    }
}

/// Every copy of [`ffx_magic::sf_hidden::caster_script_task`].
const CASTER_SCRIPT_TASKS: &[(u16, u32)] = &[
    (543, 0x5770),
    (544, 0x54c0),
    (545, 0x54c0),
    (546, 0x54c0),
    (555, 0x54e0),
    (556, 0x54b0),
    (557, 0x54b0),
    (558, 0x54b0),
    (559, 0x54b0),
];

/// The caster's script task in every DLL: an unknown step, the script
/// started and ending at once (the caster shown, the task given back),
/// ending with the index below 1, waiting, and the motion step waiting
/// instead of done.
#[test]
fn caster_script_task() {
    for &(n, rva) in CASTER_SCRIPT_TASKS {
        caster_script_task_in(n, rva);
    }
}

fn caster_script_task_in(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    let f = t.at(rva);
    bound(&t, f, "sf caster script task");
    let run = t.img.rel32_target(f + 0x85);
    let sc = t.img.rel32_target(f + 0x9d);
    let ms = t.img.rel32_target(f + 0x42);
    let (s, table, step) = (op(&t, run, 0x1c), op(&t, run, 0x17), op(&t, run, 0x68));
    for i in 0..8 {
        t.set_u32(table + 4 * i, magic::TRAPS);
    }
    // the command: moves on 4 and sets the mode from the cell at
    // SCRATCH + 0x200
    rec(&mut t, 0, 2, move |m, a| {
        let mode = m.r32(SCRATCH + 0x200);
        let v = m.r32(s + 0x54);
        m.w32(s + 0x54, v & 0xff00_ffff | (mode & 0xff) << 16);
        a[1] + 4
    });
    t.set_u32(SCRATCH + 0x300, 3);
    t.set_u32(step, SCRATCH + 0x300);
    let prog = SCRATCH + 0x400;
    for i in 0..8u32 {
        t.set_u32(prog + 4 * i, 0);
    }
    let scripts = op(&t, f, 0x4f);
    for i in 0..4u32 {
        t.set_u32(SCRATCH + 0x500 + 4 * i, prog + 4 * i);
        t.set_u32(scripts + 4 * i, SCRATCH + 0x500 + 4 * i);
    }
    let block = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, f, 0x06), block);
    // the caster shown
    rec(&mut t, 213, 2, |_, _| 0x213);
    rec(&mut t, 219, 2, |_, a| 0x0219_0000 ^ a[0]);
    rec(&mut t, 257, 2, |_, _| 0x257);
    t.set_u32(op(&t, sc, 0x13), 0x0aaa_0001);
    t.set_u32(op(&t, sc, 0x2b), 0x0bbb_0002);
    t.set_u16(op(&t, sc, 0x3), 0);
    let task = SCRATCH + 0x800;
    let done = op(&t, ms, 0x02);
    let wait = op(&t, ms, 0x11);
    for (k, (st, index, mode, finished)) in [
        (2u32, 0u32, 0u32, 1u32),
        (0, 0, 0, 1),
        (1, 0, 0, 1),
        (1, 2, 2, 1),
        (0, 1, 2, 1),
        (1, 3, 0, 1),
        (7, 0, 0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(op(&t, f, 0x0f), &varied(0x30, k));
        t.set_u32(op(&t, f, 0x25), 0x0d0d_0000 + k);
        t.write(block + 0xebf0, &varied(0x10, k));
        t.set_u32(block + 0xebf4, index);
        t.set_u32(block + 0xebf8, st);
        t.set_u32(SCRATCH + 0x200, mode);
        t.write(s, &[0; 0x60]);
        t.set_u32(s, prog);
        t.set_u8(s + 0x56, 1);
        // the task node at rest for the give-back
        t.write(task, &varied(0x20, k));
        t.set_u32(task, 0);
        t.set_u32(task + 4, 0);
        t.set_u32(task + 8, SCRATCH + 0x840);
        t.set_u32(SCRATCH + 0x84c, 0);
        t.set_u32(done, finished);
        t.set_u32(wait, 5);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0241` 0x100018b0 and its copies: the countdown above 1, at 1
/// (the end point taken), done; `t` short of the end and past it (the
/// mover's item in the list or not).
#[test]
fn fly_in_task() {
    for n in [241u16, 650, 652, 653, 656] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(0x18b0);
        bound(&t, f, "sf fly-in task");
        let w = |t: &Twin, o: u32| op(t, f, o);
        t.set_u32(w(&t, 0x34), 0x0c0c_0241);
        rec_point(&mut t, 237, 4, None, 3);
        rec_point(&mut t, 326, 4, Some(2), 0);
        rec(&mut t, 713, 1, |_, a| 0x7130_0000 ^ a[0]);
        let (list, item) = (w(&t, 0xf6), w(&t, 0xf1));
        let node = SCRATCH + 0x900;
        let task = SCRATCH + 0x800;
        for (k, (count, at, found)) in [
            (3i32, 0.25f32, false),
            (1, 0.25, false),
            (0, 0.5, false),
            (-2, 1.0, false),
            (0, 1.0, true),
            (1, 1.5, true),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(w(&t, 0x04), count.cast_unsigned());
            t.set_u32(w(&t, 0x1b), k % 2);
            t.set_u32(w(&t, 0x29) + 4 * (k % 2), 0x20 + k);
            let ta = w(&t, 0x51);
            set_f(&mut t, ta, at);
            t.write(w(&t, 0x71), &varied(0x10, k));
            t.write(w(&t, 0x20), &varied(0x10, k + 1));
            t.write(list, &varied(0x10, k + 2));
            t.write(node, &varied(0x10, k + 3));
            t.set_u32(list, if found { node } else { 0 });
            t.set_u32(list + 8, node);
            t.set_u32(node + 4, 0);
            t.set_u32(node + 8, item);
            t.set_u16(w(&t, 0x12b), 0x8123);
            // the task node at rest for the give-back
            t.write(task, &varied(0x20, k));
            t.set_u32(task, 0);
            t.set_u32(task + 4, 0);
            t.set_u32(task + 8, SCRATCH + 0x840);
            t.set_u32(SCRATCH + 0x84c, 0);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// `magic_0387` 0x100088d0: every step (the table's cases, the default,
/// above the table), the motion running on and ended, the frame count
/// short of 0x59 and at it.
#[test]
fn motion_sequence_task() {
    let Some(mut t) = twin(387) else { return };
    let f = t.at(0x88d0);
    bound(&t, f, "sf motion sequence task");
    let ended = SCRATCH + 0x700;
    rec(&mut t, 228, 2, |_, a| 0x2280_0000 ^ a[1]);
    rec(&mut t, 411, 1, move |m, _| m.r32(ended));
    let (task, s, o) = (SCRATCH + 0x800, SCRATCH + 0x900, SCRATCH + 0xa00);
    let (node, prev, next) = (SCRATCH + 0xb00, SCRATCH + 0xb40, SCRATCH + 0xb80);
    let mut k = 0u32;
    for step in [0u32, 1, 2, 3, 9, 10, 11, 12, 20, 21, 0x15 + 1, 0x4e20] {
        for (end, frames) in [(0u32, 0x10u32), (1, 0x59), (2, 0x58)] {
            k += 1;
            t.write(task, &varied(0x20, k));
            t.write(s, &varied(0x40, k));
            t.write(o, &varied(0x60, k));
            t.set_u32(task, node);
            t.set_u32(task + 4, s);
            t.set_u32(s + 0x3c, o);
            t.set_u32(s + 0x18, step);
            t.set_u32(s + 0x34, frames);
            t.set_u32(ended, end);
            for (a, p, n) in [(node, prev, next), (prev, next, node), (next, node, prev)] {
                t.set_u32(a + 4, p);
                t.set_u32(a + 8, n);
            }
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// Each listed slot recorded with `n` words, answering a number of its
/// own.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (s << 20) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `magic_0310` 0x10006570: the set-up from step 0 and from step 10 (the
/// list's last the sentinel or a node), the end with the flag clear and
/// set, an unknown step.
#[test]
fn two_model_task() {
    let Some(mut t) = twin(310) else { return };
    let f = t.at(0x6570);
    bound(&t, f, "sf two model task");
    let w = |t: &Twin, o: u32| op(t, f, o);
    recs(
        &mut t,
        &[
            (413, 1),
            (215, 1),
            (14, 2),
            (237, 4),
            (325, 2),
            (714, 5),
            (719, 3),
            (257, 2),
            (38, 3),
            (296, 2),
            (541, 3),
            (538, 2),
            (213, 2),
            (211, 2),
            (35, 5),
            (34, 2),
            (547, 2),
            (276, 2),
            (219, 2),
            (695, 1),
        ],
    );
    rec(&mut t, 333, 1, |_, a| 0x3330_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 0.5);
    let table = SCRATCH + 0x3000;
    for i in 0..0x40 {
        t.set_u32(table + 4 * i, 0x0a0a_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    t.set_u32(w(&t, 0x139), 0x0c0c_0310);
    t.set_u32(w(&t, 0x96), 0x0b0b_0310);
    // the particle's program owner (its `+0x10` for slot 941)
    let owner = SCRATCH + 0x3800;
    t.write(owner, &varied(0x100, 9));
    t.set_u32(w(&t, 0x238), owner);
    let (task, s) = (SCRATCH + 0x800, SCRATCH + 0x900);
    let (node, prev, next) = (SCRATCH + 0xb00, SCRATCH + 0xb40, SCRATCH + 0xb80);
    let other = SCRATCH + 0xc00;
    let flags = w(&t, 0x2e);
    for (k, (step, last_is_sentinel, flag)) in [
        (0u32, true, 0u8),
        (0xa, false, 0),
        (0x2710, true, 0),
        (0x2710, true, 4),
        (5, true, 4),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(task, &varied(0x20, k));
        t.write(s, &varied(0x40, k));
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.set_u32(s + 0x18, step);
        for (a, p, n) in [(node, prev, next), (prev, next, node), (next, node, prev)] {
            t.set_u32(a + 4, p);
            t.set_u32(a + 8, n);
        }
        t.write(other, &varied(0x20, k + 1));
        let last = if last_is_sentinel {
            w(&t, 0x109)
        } else {
            other
        };
        t.set_u32(w(&t, 0xce), last);
        t.set_u8(flags, flag);
        // the lists empty: the model and node list (the parts' sentinel)
        // and the particle list
        for l in [w(&t, 0xaa), w(&t, 0x253)] {
            t.write(l, &[0; 0x20]);
            t.set_u32(l + 0xc, l);
            t.set_u32(l + 0x10, l);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0257` 0x10006690: every step, each frame gate short and met,
/// the centre slot taken and skipped (`G_HIDE` bit 0).
#[test]
#[allow(clippy::too_many_lines)] // one set-up per callee
fn centred_parts_task() {
    let Some(mut t) = twin(257) else { return };
    let f = t.at(0x6690);
    bound(&t, f, "sf centred parts task");
    let w = |t: &Twin, o: u32| op(t, f, o);
    recs(
        &mut t,
        &[
            (413, 1),
            (215, 1),
            (14, 2),
            (218, 2),
            (228, 2),
            (366, 2),
            (325, 2),
            (714, 5),
            (719, 3),
            (8, 2),
            (257, 2),
            (38, 3),
            (296, 2),
            (541, 3),
            (538, 2),
            (213, 2),
            (211, 2),
            (35, 5),
            (34, 2),
            (547, 2),
            (276, 2),
            (219, 2),
        ],
    );
    rec(&mut t, 7, 1, |_, a| 0x0700_0007 ^ a[0]);
    rec(&mut t, 333, 1, |_, a| 0x3330_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 0.5);
    let table = SCRATCH + 0x3000;
    for i in 0..0x40 {
        t.set_u32(table + 4 * i, 0x0a0a_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let owner = SCRATCH + 0x3800;
    t.write(owner, &varied(0x100, 9));
    t.set_u32(w(&t, 0x1ad), owner);
    t.set_u32(w(&t, 0xf8), 0x0c0c_0257);
    t.set_u32(w(&t, 0x12b), 0x0c0c_1257);
    t.set_u32(w(&t, 0x1fe), 0x0c0c_2257);
    t.set_u32(w(&t, 0x7f), 0x0b0b_0257);
    t.set_u32(w(&t, 0xb8), 0x0b0b_1257);
    let (task, s) = (SCRATCH + 0x800, SCRATCH + 0x900);
    let (node, prev, next) = (SCRATCH + 0xb00, SCRATCH + 0xb40, SCRATCH + 0xb80);
    let hide = w(&t, 0x118);
    let mut k = 0u32;
    for (step, frames, hidden) in [
        (0u32, 0i32, 0u8),
        (0xa, 0, 1),
        (0x1e, 0xa9, 0),
        (0x1e, 0xaa, 0),
        (0x3c, -1, 0),
        (0x3c, 0, 0),
        (0x3c, 5, 1),
        (0x3d, 0xc7, 0),
        (0x3d, 0xc8, 0),
        (0x2710, 0, 0),
        (0x1f, 0, 0),
        (0x4000, 0, 0),
    ] {
        k += 1;
        t.write(task, &varied(0x20, k));
        t.write(s, &varied(0x40, k));
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.set_u32(s + 0x18, step);
        t.set_u32(s + 0x2c, frames.cast_unsigned());
        for (a, p, n) in [(node, prev, next), (prev, next, node), (next, node, prev)] {
            t.set_u32(a + 4, p);
            t.set_u32(a + 8, n);
        }
        t.set_u8(hide, hidden);
        // the lists empty: the parts' list and the particle list
        for l in [w(&t, 0x93), w(&t, 0x1c8)] {
            t.write(l, &[0; 0x20]);
            t.set_u32(l + 0xc, l);
            t.set_u32(l + 0x10, l);
        }
        let scale = w(&t, 0x2cd);
        set_f(&mut t, scale, 0.25 * k as f32);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0342` 0x10006c40: every step (the set-up from 0 and from 10,
/// each colour step with its gate short and met, 0x3c, 0x3d, the end,
/// outside the table).
#[test]
#[allow(clippy::too_many_lines)] // one set-up per callee
fn shade_fade_task() {
    let Some(mut t) = twin(342) else { return };
    let f = t.at(0x6c40);
    bound(&t, f, "sf shade fade task");
    let w = |t: &Twin, o: u32| op(t, f, o);
    recs(
        &mut t,
        &[
            (413, 1),
            (215, 1),
            (14, 2),
            (256, 5),
            (325, 2),
            (714, 5),
            (719, 3),
            (538, 2),
            (213, 2),
            (211, 2),
            (35, 5),
            (34, 2),
            (547, 2),
            (276, 2),
            (219, 2),
        ],
    );
    rec(&mut t, 333, 1, |_, a| 0x3330_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 0.5);
    let table = SCRATCH + 0x3000;
    for i in 0..0x40 {
        t.set_u32(table + 4 * i, 0x0a0a_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let owner = SCRATCH + 0x3800;
    t.write(owner, &varied(0x100, 9));
    t.set_u32(w(&t, 0x1f6), owner);
    t.set_u32(w(&t, 0x7a), 0x0c0c_0342);
    t.set_u32(w(&t, 0x43), 0x0b0b_0342);
    let (task, s) = (SCRATCH + 0x800, SCRATCH + 0x1000);
    let (node, prev, next) = (SCRATCH + 0xb00, SCRATCH + 0xb40, SCRATCH + 0xb80);
    let list = w(&t, 0x57);
    let plist = w(&t, 0x243);
    let mut k = 0u32;
    for (step, frames, last_is_list) in [
        (0u32, 0i32, true),
        (0xa, 3, false),
        (0x1e, 0, true),
        (0x1f, 0xe, true),
        (0x1f, 0xf, true),
        (0x20, 0xf, true),
        (0x21, 0x10, true),
        (0x22, 0xf, true),
        (0x23, 0xf, true),
        (0x24, 0xe, true),
        (0x24, 0xf, true),
        (0x25, 0xf, true),
        (0x3c, -1, true),
        (0x3c, 0, true),
        (0x3d, 0xc7, true),
        (0x3d, 0xc8, true),
        (0x3e, 0, true),
        (0x2710, 0, true),
        (0x2711, 0, true),
    ] {
        k += 1;
        t.write(task, &varied(0x20, k));
        t.write(s, &varied(0x300, k));
        t.set_u32(task, node);
        t.set_u32(task + 4, s);
        t.set_u32(s + 0x18, step);
        t.set_u32(s + 0x2c, frames.cast_unsigned());
        for (a, p, n) in [(node, prev, next), (prev, next, node), (next, node, prev)] {
            t.set_u32(a + 4, p);
            t.set_u32(a + 8, n);
        }
        // the lists: the parts' list empty or holding a node; the
        // particle list empty
        let other = SCRATCH + 0xc00;
        t.write(other, &varied(0x20, k + 1));
        for l in [list, plist] {
            t.write(l, &[0; 0x20]);
            t.set_u32(l + 0xc, l);
            t.set_u32(l + 0x10, l);
        }
        if !last_is_list {
            t.set_u32(list + 0x10, other);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `n` free nodes on the pool's ring for [`ffx_magic::sa_pool::take`]
/// (the function at `take`), 0x40 bytes apart from `at`.
fn free_ring(t: &mut Twin, take: u32, at: u32, n: u32) {
    let node = |i: u32| at + 0x40 * i;
    for i in 1..=n {
        t.write(node(i), &varied(0x1c, i + 0x30));
        t.set_u32(node(i), node(if i == 1 { n } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == n { 1 } else { i + 1 }));
    }
    t.set_u32(t.img.word(take + 0x17), node(1));
    t.set_u16(t.img.word(take + 0x04), n as u16);
}

/// `magic_0271` 0x10001810: the countdown still running; a spark with
/// sparks left (the turn and the yaw wrapped down, up and kept, the inner
/// points' heights capped or not); the last spark and the end (the object
/// start's environment as in `magic_o6_fam.rs`).
#[test]
#[allow(clippy::too_many_lines)] // one set-up per callee
fn spark_burst_task() {
    let Some(mut t) = twin(271) else { return };
    let f = t.at(0x1810);
    bound(&t, f, "sf spark burst task");
    let w = |t: &Twin, o: u32| op(t, f, o);
    rec(&mut t, 719, 3, |_, a| 0x7190_0000 ^ a[1]);
    rec_point(&mut t, 237, 4, Some(2), 3);
    // slot 262: three floats, one per pointer
    let row = SCRATCH + 0x3_8000;
    rec(&mut t, 262, 4, move |m, a| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        for (i, x) in [1.5f32, -0.75, 2.25].into_iter().enumerate() {
            let v = x + (k as f32) * 0.5 + f32::from((a[0] & 7) as u8);
            m.w32(a[1 + i], v.to_bits());
        }
        0x262
    });
    rec_point(&mut t, 268, 3, None, 2);
    rec_point(&mut t, 235, 1, None, 0);
    rec_point(&mut t, 329, 3, Some(1), 0);
    rec_point(&mut t, 330, 3, Some(1), 0);
    rec_point(&mut t, 229, 3, Some(2), 0);
    rec_point(&mut t, 326, 4, Some(2), 0);
    for s in [314u32, 404, 12] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    let scale = SCRATCH + 0x3_9000;
    t.stub_st0(314, move |m, a| {
        f64::from(f32::from_bits(a[0])) * f64::from(f32::from_bits(m.r32(scale)))
    });
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 0.375);
    t.stub_st0(12, |_, _| -1.25);
    // the object start (the end): slot 941's list, slot 714 logged, the
    // pool with free nodes
    let at = t.img.rel32_target(f + 0x55f);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(at + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        SCRATCH + 0x400
    });
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    let pa = t.img.rel32_target(at + 0x9d);
    let free = t.img.word(pa + 0x04);
    let nodes = t.img.word(pa + 0x27) - 8;
    let tail = t.img.word(at + 0xa8);
    let take = t.img.rel32_target(f + 0x4e8);
    let blk = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x1a), blk);
    t.set_u32(w(&t, 0xa5), 0x0c0c_0271);
    let sel = SCRATCH + 0x700;
    t.set_u32(w(&t, 0xd3), sel);
    for i in 0..4 {
        t.set_u32(w(&t, 0xe3) + 4 * i, 0x0a0a_0000 + i);
    }
    let task = SCRATCH + 0x800;
    let mut k = 0u32;
    for (count, left, angle, rnd) in [
        (3i32, 2i32, 0.0f32, 1.0f32),
        (1, 3, 3.1, 1.0),
        (0, 2, -3.1, -1.0),
        (1, 4, 0.5, 0.25),
        (1, 1, 2.0, 0.5),
        (-5, 1, -1.0, 2.0),
    ] {
        k += 1;
        t.write(blk, &varied(0x2000, k));
        t.set_u32(blk + 0x1f28, count.cast_unsigned());
        t.set_u32(blk + 0x1f2c, left.cast_unsigned());
        set_f(&mut t, blk + 0x1f24, angle);
        set_f(&mut t, scale, rnd);
        t.set_u8(sel, (k % 4) as u8);
        free_ring(&mut t, take, SCRATCH + 0x5000, 4);
        t.set_u32(
            w(&t, 0x4ee),
            if k.is_multiple_of(2) {
                0
            } else {
                SCRATCH + 0x6000
            },
        );
        t.set_u32(w(&t, 0x507), SCRATCH + 0x6000);
        t.write(SCRATCH + 0x6000, &varied(0x20, k));
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        t.set_u16(free, 3);
        t.set_u32(tail, 0);
        // the task node at rest for the give-back
        t.write(task, &varied(0x20, k));
        t.set_u32(task, 0);
        t.set_u32(task + 4, 0);
        t.set_u32(task + 8, SCRATCH + 0x840);
        t.set_u32(SCRATCH + 0x84c, 0);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}
