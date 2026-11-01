//! The functions the first overlay census hid inside merged rows
//! (`ffx_magic::sf_hidden`, census2 2026-11-01) against every DLL
//! carrying each. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

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
