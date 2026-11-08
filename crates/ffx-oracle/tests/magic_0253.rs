//! `magic_0253.dll` against its Rust ports (`ffx_magic::m0253`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::needless_range_loop
)]

mod magic;
mod magic_owna;

const N: u16 = 253;

/// 0x10001340: the script and the objects running or done, the
/// halfword set and clear, the pool node there or not.
#[test]
fn frame() {
    use magic::{SCRATCH, Twin};
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x1340);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(213u32, 2usize), (238, 2), (719, 3), (306, 3)] {
        magic_owna::rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let sr = t.img.rel32_target(at + 0x05);
    let ro = t.img.rel32_target(at + 0x0c);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    let node = SCRATCH + 0x3000;
    for (ended, busy, half, with_node) in [
        (true, false, 0u16, true),
        (true, false, 1, false),
        (false, false, 0, true),
        (true, true, 0, true),
    ] {
        // the script's mode byte (0 ended, 2 waiting for ever), no
        // callback after
        t.set_u8(t.img.word(sr + 0x01), if ended { 0 } else { 2 });
        t.set_u16(t.img.word(sr + 0x59), 0xffff);
        t.set_u32(t.img.word(sr + 0x91), 0);
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        // the halfword's neighbour junk (the test reads 16 bits)
        t.set_u32(w(&t, 0x1c), 0x5a5a_0000 | u32::from(half));
        t.set_u32(w(&t, 0x2b), 0x00ab_002b);
        t.set_u32(w(&t, 0x43), 0x00ab_0043);
        t.write(node, &[0x4c; 0x20]);
        t.set_u32(w(&t, 0x5b), if with_node { node } else { 0 });
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check_entry(at, magic::Call::default(), &[task], magic::Eax::Checked);
    }
}

/// 0x10001930: the two part slots of records 0 and 3.
#[test]
fn part_slots() {
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x1930);
    for i in [0u32, 3] {
        t.set_u32(t.img.word(at + 0x01), i);
        t.check_entry(at, magic::Call::default(), &[], magic::Eax::Checked);
    }
}

/// 0x10001710: `r15::camera_targets_0253` (the point from slot 217,
/// other values before in every global it sets).
#[test]
fn camera_targets() {
    use magic::{Call, Eax, Twin};
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x1710);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    magic_owna::rec(&mut t, 0x364 / 4, 2, |m, a| {
        for (i, x) in [1.25f32, -2.5, 3.75, 9.0].into_iter().enumerate() {
            m.w32(a[1] + 4 * i as u32, x.to_bits());
        }
        0x364
    });
    magic_owna::recs(&mut t, &[(0x868 / 4, 2), (0x968 / 4, 2)]);
    for (i, op) in [0x1bu32, 0x4f, 0x16a].into_iter().enumerate() {
        t.set_u32(w(&t, op), 0x0a00_0000 + i as u32);
    }
    let first = w(&t, 0x5e);
    t.write(first, &[0x5a; 0x90]);
    for op in [0x13d, 0x147, 0x151] {
        t.set_u32(w(&t, op), 0x0bad_0000 + op);
    }
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x100013d0: `r15::objects_0253`: five object starts
/// (`s4_misc::start_object_anim`), three places, the yaws above, below
/// and inside the wrap bounds.
#[test]
fn objects() {
    use magic::{Call, Eax, SCRATCH, Twin};
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x13d0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let so = t.img.rel32_target(at + 0x26);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    magic_owna::rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
    let sp = t.img.rel32_target(so + 0x84);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    magic_owna::rec(&mut t, 0x418 / 4, 4, |m, a| {
        for i in 1..4 {
            m.w32(a[i], (0.5 * i as f32 - 3.0).to_bits());
        }
        0x418
    });
    magic_owna::rec(&mut t, 0x430 / 4, 3, |m, a| {
        let k = m.r32(magic_owna::row(0x430 / 4));
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (1.5 + i as f32 - k as f32 * 0.75).to_bits());
        }
        0x430
    });
    // the yaws: 3pi/2 - r with r per call (above, below, inside the bounds)
    let yaw = SCRATCH + 0x3500;
    magic_owna::rec(&mut t, 0x534 / 4, 1, |_, a| a[0]);
    t.stub_st0(0x534 / 4, move |m, _| {
        let k = m.r32(magic_owna::row(0x534 / 4));
        f64::from(f32::from_bits(m.r32(yaw + 4 * (k % 4))))
    });
    // (`float_product`'s slots: 364 reads one word, 16 two)
    for (s, n) in [(364u32, 1usize), (16, 2)] {
        magic_owna::rec(&mut t, s, n, move |_, a| s ^ a[0]);
    }
    t.stub_st0(364, |_, _| 1.75);
    t.stub_st0(16, |_, _| -0.5);
    for (i, op) in [0x31u32, 0x84, 0x135, 0x171, 0x214, 0x250]
        .into_iter()
        .enumerate()
    {
        t.set_u32(w(&t, op), 0x0a00_0000 + i as u32);
    }
    for (case, yaws) in [
        [-9.0f32, 9.0, 4.0, 0.0],
        [9.0, 4.0, -9.0, 0.0],
        [4.0, -9.0, 9.0, 0.0],
    ]
    .into_iter()
    .enumerate()
    {
        for (i, y) in (0u32..).zip(yaws) {
            t.set_u32(yaw + 4 * i, y.to_bits());
        }
        t.set_u32(magic_owna::row(0x534 / 4), 0);
        t.set_u32(magic_owna::row(0x430 / 4), case as u32);
        scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// Junk on the stack below the entry (the frames' never-read words).
fn scrub(t: &mut magic::Twin) {
    let entry = magic::STACK_TOP - 0x100;
    let junk: Vec<u8> = (0..0x1000u32)
        .map(|i| (i.wrapping_mul(0x2b) ^ 0x5b).to_le_bytes()[0])
        .collect();
    t.write(entry - 0x1000, &junk);
}
