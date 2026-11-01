//! `magic_0351.dll` against its Rust port (`ffx_magic::m0351`): the
//! sounds. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 351;

/// 0x10002770: stopped, early, and the sounds for counts 0, 2 and 5.
#[test]
fn sounds_3() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2770);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 239, 3, |_, a| a[1] ^ 0x239);
    let bytes = SCRATCH + 0x300;
    for i in 0u8..4 {
        t.set_u8(bytes + u32::from(i), 0x60 + i);
    }
    t.set_u32(w(&t, 0x35), bytes);
    t.set_u32(w(&t, 0x47), 0x0047_0047);
    for (k, f) in (0u32..).zip([1u32, 0, 1, 1]) {
        t.set_u32(w(&t, 0x2d) + 4 * k, f);
    }
    for (stop, clock, count) in [
        (1u32, 0x40u32, 3u32),
        (0, 0x27, 3),
        (0, 0x28, 0),
        (0, 0x30, 2),
        (0, 0x50, 5),
    ] {
        t.set_u32(w(&t, 0x02), stop);
        t.set_u32(w(&t, 0x0b), clock);
        t.set_u32(w(&t, 0x21), count);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001000: the model's reset with the word set and clear.
#[test]
fn model_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1000);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(399u32, 2usize), (363, 3), (382, 2)] {
        magic_owna::rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    t.set_u32(w(&t, 0x09), 0x00ab_0009);
    t.set_u32(w(&t, 0x36), 0x00dd_0036);
    for word in [0u32, 0x00cc_0001] {
        t.set_u32(w(&t, 0x2d), word);
        for op in [0x55, 0x5f, 0x69, 0x73] {
            t.set_u32(w(&t, op), 0x5a5a_5a5b);
        }
        t.check_entry(at, magic::Call::default(), &[], magic::Eax::Checked);
    }
}

/// The part starts' set-up: the program list (slot 941) and
/// pppStartPart's lock, the other slots recording their words.
fn part_setup(t: &mut magic::Twin, at: u32, call: u32) {
    use magic::StubMem;
    let list = SCRATCH + 0x700;
    for i in 0..4 {
        t.set_u32(list + 4 * i, 0x0900_0000 + i);
    }
    magic_owna::rec(t, 941, 1, move |_, _| list);
    magic_owna::rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    magic_owna::rec(t, 325, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ 0x325;
            m.w32(a[0] + 4 * i, v & 0x3fff_ffff);
        }
        0x325
    });
    magic_owna::rec(t, 220, 3, |_, a| 0x0220_0000 ^ a[0]);
    let gp = SCRATCH + 0x680;
    t.set_u32(t.img.word(at + 0x10), gp);
    t.set_u32(gp + 0x10, 0x00de_0010);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x640);
    let sp = t.img.rel32_target(at + call);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
}

/// 0x100015f0, 0x10002130, 0x10002270: the part starts.
#[test]
fn part_starts() {
    let Some(mut t) = twin(N) else { return };
    let obj = SCRATCH + 0x1000;
    for (rva, call) in [(0x15f0u32, 0x36u32), (0x2130, 0x36), (0x2270, 0x35)] {
        let at = t.at(rva);
        part_setup(&mut t, at, call);
        if rva == 0x2270 {
            // slot 220's first word *X
            t.set_u32(t.img.word(at + 0x47), 0x00c4_7047);
        }
        for v in [1.5f32, -0.25] {
            t.write(obj, &[0x5c; 0x80]);
            t.set_u32(obj + 0x44, v.to_bits());
            t.set_u32(obj + 0x40, (v * 2.0).to_bits());
            t.check_entry(at, Call::default(), &[obj], Eax::Checked);
        }
    }
}

/// 0x10002690: the camera fade for two divisors.
#[test]
fn camera_fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2690);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(267u32, 1usize), (801, 5), (785, 1)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    t.set_u32(w(&t, 0x5a), 0.625f32.to_bits());
    for d in [2.0f32, 0.0, -3.0] {
        t.set_u32(w(&t, 0x1b), d.to_bits());
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002c40: not started, paused or not, the timeline running and its
/// last frame (slot 214).
#[test]
fn frame_timeline() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2c40);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let paused = SCRATCH + 0x5f0;
    rec(&mut t, 253, 0, move |m, _| m.r32(paused));
    for (s, n) in [(723u32, 1usize), (724, 2), (214, 1)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let flag = SCRATCH + 0x600;
    let p = SCRATCH + 0x610;
    let q = SCRATCH + 0x620;
    t.set_u32(w(&t, 0x0d), flag);
    t.set_u32(w(&t, 0x34), p);
    t.set_u32(w(&t, 0x66), q);
    for op in [0x3a, 0x58, 0x6b, 0x72, 0x8e] {
        t.set_u32(w(&t, op), 0x00ab_0000 + op);
    }
    let tl = t.img.rel32_target(at + 0x7f);
    let live = t.img.word(tl + 0x06);
    // no events: a live timeline only asks slot 253
    t.set_u32(t.img.word(tl + 0x25), 0);
    for (started, pause, n_live) in [
        (0u32, 0u32, 1u32),
        (1, 0, 1),
        (1, 1, 1),
        (1, 0, 0),
        (2, 5, 0),
    ] {
        t.set_u32(w(&t, 0x02), started);
        t.set_u32(paused, pause);
        t.set_u32(live, n_live);
        t.set_u32(flag, 0x77);
        t.set_u32(p, 0x88);
        t.set_u32(q, 0x99);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
