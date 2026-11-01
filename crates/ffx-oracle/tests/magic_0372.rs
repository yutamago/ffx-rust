//! `magic_0372.dll` against its Rust ports (`ffx_magic::m0372`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 372;

/// 0x10001000: the part start for two heights.
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
    rec(&mut t, 410, 2, |m, a| {
        let v = m.r32(a[1] + 4);
        m.w32(a[1] + 4, v ^ 0x0041_0000);
        0x0410_0000 ^ a[0]
    });
    let gp = SCRATCH + 0x680;
    t.set_u32(t.img.word(at + 0x10), gp);
    t.set_u32(gp + 0x10, 0x00de_0010);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x640);
    t.set_u32(t.img.word(at + 0x45), 0x00ab_0045);
    let sp = t.img.rel32_target(at + 0x36);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
    let obj = SCRATCH + 0x1000;
    for v in [1.5f32, -0.25] {
        t.write(obj, &[0x5c; 0x80]);
        t.set_u32(obj + 0x44, v.to_bits());
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001490: not running, slot 261 answering 0, the timeline with
/// nothing live and with an event live.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1490);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(723u32, 1usize), (724, 1), (214, 1)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    rec(&mut t, 261, 1, |m, _| m.r32(SCRATCH + 0x6f0));
    rec(&mut t, 253, 0, |_, _| 1);
    let tl = t.img.rel32_target(at + 0x61);
    let tw = |t: &magic::Twin, off: u32| t.img.word(tl + off);
    t.set_u32(w(&t, 0x0d), SCRATCH + 0x640);
    t.set_u32(w(&t, 0x44), 0x00aa_0044);
    for (running, on, live) in [(0u32, 1u32, 0u32), (1, 0, 0), (1, 1, 0), (1, 1, 1)] {
        t.set_u32(w(&t, 0x02), running);
        t.set_u32(SCRATCH + 0x6f0, on);
        // the timeline's live count; with one live event its record
        // (start 0 at frame 5, length 100, no callbacks)
        t.set_u32(tw(&t, 0x06), live);
        t.set_u32(tw(&t, 0x25), live);
        let ev = SCRATCH + 0x5000;
        t.write(ev, &[0; 0x18]);
        t.set_u32(ev + 4, 100);
        t.set_u32(tw(&t, 0x17), ev);
        t.set_u32(tw(&t, 0x1f), live);
        t.set_u32(tw(&t, 0x41), 5);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
