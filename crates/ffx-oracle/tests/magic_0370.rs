//! `magic_0370.dll` against its Rust ports (`ffx_magic::m0370`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 370;

/// 0x10001720: the part's start (its words junk first).
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1720);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let list = SCRATCH + 0x700;
    t.set_u32(list, 0x0900_0000);
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let gp = SCRATCH + 0x680;
    t.set_u32(w(&t, 0x0c), gp);
    t.set_u32(gp + 0x10, 0x00de_0010);
    t.set_u32(w(&t, 0x01), SCRATCH + 0x640);
    let sp = t.img.rel32_target(at + 0x31);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
    for (k, src) in (0u32..).zip([0x39u32, 0x48]) {
        for i in 0..4 {
            t.set_u32(w(&t, src) + 4 * i, 0x0a00_0000 + 0x10 * k + i);
        }
    }
    for dst in [0x41u32, 0x5c] {
        t.write(w(&t, dst), &[0x5a; 0x10]);
    }
    for op in [0x51, 0x62, 0x6c, 0x76, 0x80, 0x8a, 0x94, 0x9e] {
        t.set_u32(w(&t, op), 0x5a5a_5a5b);
    }
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x100017d0: the part's draw with the lock set and clear before (the
/// lock seen during slot 712 copied out).
#[test]
fn draw() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x17d0);
    let lock = SCRATCH + 0x600;
    // the slots copy the block pointer **P out (it changes before 712)
    let pp = SCRATCH + 0x640;
    for (k, (s, n)) in (0u32..).zip([(723u32, 1usize), (724, 2), (258, 3), (231, 6), (230, 8)]) {
        rec(&mut t, s, n, move |m, a| {
            let v = m.r32(pp);
            m.w32(lock + 0x48 + 4 * k, v);
            (0x0100_0000 + s) ^ a[0]
        });
    }
    rec(&mut t, 712, 10, move |m, a| {
        // **P first: the lock's copy lands on P's word (lock + 0x40)
        let v = m.r32(pp);
        m.w32(lock + 0x60, v);
        let seen = m.r32(lock);
        m.w32(lock + 0x40, seen);
        0x0712_0000 ^ a[1]
    });
    t.set_u32(t.img.word(at + 0x8a), lock);
    t.set_u32(t.img.word(at + 0x01), SCRATCH + 0x640);
    for (k, op) in [0x08u32, 0x26, 0x46, 0x5d, 0x70].into_iter().enumerate() {
        t.set_u32(t.img.word(at + op), 0x00ab_0000 + u32::try_from(k).unwrap());
    }
    for old in [0u32, 0x77] {
        t.set_u32(lock, old);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001d80: not started; the flags each set or clear (the timelines
/// idle), flag 3 clear and set (the stops and slot 214).
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1d80);
    let flags = SCRATCH + 0x6e0;
    rec(&mut t, 261, 1, move |m, a| m.r32(flags + 4 * (a[0] & 3)));
    rec(&mut t, 214, 1, |_, a| 0x0214_0000 ^ a[0]);
    let p = SCRATCH + 0x610;
    t.set_u32(t.img.word(at + 0x0e), p);
    for op in [0x13u32, 0x1a, 0xbe] {
        t.set_u32(t.img.word(at + op), 0x00ab_0000 + op);
    }
    // the timelines live (no events), so a run moves their time on
    // (slot 253 says so): which ones run shows
    rec(&mut t, 253, 0, |_, _| 1);
    let tls = [0x3au32, 0x5a, 0x7a].map(|op| t.img.word(at + op));
    for (started, set) in [
        (0u32, [1u32, 1, 1, 1]),
        (1, [0, 0, 0, 0]),
        (1, [1, 0, 1, 0]),
        (1, [0, 1, 0, 1]),
        (3, [1, 1, 1, 1]),
    ] {
        t.set_u32(t.img.word(at + 0x02), started);
        for (k, v) in (0u32..).zip(set) {
            t.set_u32(flags + 4 * k, v);
        }
        t.set_u32(p, 0x88);
        for (k, tl) in (0u32..).zip(tls) {
            t.set_u32(tl, 0x100 * k);
            t.set_u32(tl + 4, 0);
            t.set_u32(tl + 8, 1);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001000: a part's start for a part inside and past the table's
/// first 0x80 bytes.
#[test]
fn part_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1000);
    let list = SCRATCH + 0x700;
    for i in 0..4 {
        t.set_u32(list + 4 * i, 0x0900_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let sp = t.img.rel32_target(at + 0x36);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x600);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x640);
    let l = SCRATCH + 0x680;
    t.set_u32(t.img.word(at + 0x10), l);
    t.set_u32(l + 0x10, 0x00ab_0010);
    let table = t.img.word(at + 0x3d);
    for part in [table, table + 0x40, table + 0x80, table + 0x100] {
        t.write(part, &[0x5c; 0x80]);
        t.check_entry(at, Call::default(), &[part], Eax::Checked);
    }
}
