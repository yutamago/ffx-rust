//! `magic_0316.dll` against its Rust ports (`ffx_magic::m0316`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 316;

/// The objects host and motion step of a frame callback: `busy` puts an
/// object on the list (so the objects do not answer 1), `done` is the
/// motion step's done word. `ro`/`ms` are the callees' addresses (0 for
/// none).
fn host(t: &mut Twin, ro: u32, ms: u32, busy: bool, done: bool) {
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    if ro != 0 {
        let list = t.img.word(ro + 0x7c);
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
    }
    if ms != 0 {
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
    }
}

/// Runs the frame callback at `at` for a fresh task.
fn run(t: &mut Twin, at: u32) {
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    t.write(task, &[0x11; 0x20]);
    t.set_u32(task + 8, record);
    t.write(record, &[0x22; 0x20]);
    t.check_entry(at, Call::default(), &[task], Eax::Checked);
}

/// The slots the frame callbacks call, each recording its words.
fn stubs(t: &mut Twin) {
    for (s, n) in [
        (296u32, 2usize),
        (257, 2),
        (525, 1),
        (719, 3),
        (306, 3),
        (790, 0),
    ] {
        rec(t, s, n, move |_, a| {
            (0x0100_0000 + s) ^ a.first().copied().unwrap_or(0)
        });
    }
    t.stub_st0(790, |_, _| 2.5);
}

/// 0x10001430: the objects busy or done, the effect's done words.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    stubs(&mut t);
    let at = t.at(0x1430);
    let ro = t.img.rel32_target(at + 0x0b);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x06), fx);
    // one part for the tail's register_parts (slots 941, 772, 779), its
    // lock at P + 0x200, so the callee's address matters
    let rp = t.img.rel32_target(at + 0x6c);
    let parts = SCRATCH + 0x5000;
    t.write(parts, &[0x5e; 0x80]);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(t.img.word(rp + 0x43), parts + 0x200);
    t.set_u32(t.img.word(rp + 0x5f), parts + 0x200);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0] ^ a[1]);
    t.stub_edx(772, |_, a| 0x00ed_0000 ^ a[1]);
    rec(&mut t, 779, 4, |_, a| 0x0779_0000 ^ a[2]);
    for (busy, d1, d2) in [
        (false, 1u32, 1u32),
        (false, 1, 0),
        (false, 2, 1),
        (true, 1, 1),
    ] {
        // the three words the end sets to 1.0 hold junk
        for op in [0x29, 0x3d, 0x53] {
            t.set_u32(t.img.word(at + op), 0x2929_0000 + op);
        }
        t.set_u32(fx + 0x110, d1);
        t.set_u32(fx + 0x114, d2);
        host(&mut t, ro, 0, busy, true);
        run(&mut t, at);
    }
}

/// 0x10001760: the sprite task into an empty and a filled task list.
#[test]
fn sprite_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1760);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x03), fx);
    let take = t.img.rel32_target(at + 0x2b);
    let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let node = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
    let last = SCRATCH + 0x1a00;
    for first in [0u32, last] {
        for k in 0..2 {
            t.write(node(k), &[0x5a; 0x40]);
            t.set_u32(node(k), node(1 - k));
            t.set_u32(node(k) + 4, node(1 - k));
        }
        t.set_u32(head, node(0));
        t.set_u16(count, 2);
        t.write(last, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x31), first);
        t.set_u32(w(&t, 0x44), first);
        t.write(fx + 0x80, &[0x44; 0x20]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100013e0: the script ended or waiting, the motion done or not.
#[test]
fn step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x13e0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let fx = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x03), fx);
    let run = t.img.rel32_target(at + 0x2f);
    let ms = t.img.rel32_target(at + 0x3a);
    let s = t.img.word(run + 0x1c);
    for (mode, done) in [(0u8, true), (2, true), (0, false), (2, false)] {
        t.set_u32(w(&t, 0x22), 0x0022_0022);
        t.set_u16(w(&t, 0x1d), 0x1234);
        t.write(w(&t, 0x0c).wrapping_sub(4), &[0x6b; 0x38]);
        // the script ended (mode 0, returns 1) or waiting for ever (2)
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u16(s + 0x54, 0xffff);
        t.set_u32(s + 0x24, 0);
        t.set_u32(s + 0x28, 0);
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.write(fx + 0x100, &[0x55; 0x20]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
