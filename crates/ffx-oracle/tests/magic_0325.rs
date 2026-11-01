//! `magic_0325.dll` against its Rust ports (`ffx_magic::m0325`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 325;

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

/// 0x10001910: the countdown at and above 1 (both sound branches), the
/// objects and the motion step running or done.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    stubs(&mut t);
    let at = t.at(0x1910);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let ro = t.img.rel32_target(at + 0x67);
    let ms = t.img.rel32_target(at + 0x6e);
    for (count, flag, sound, busy, done) in [
        (1i32, 0u32, 3i32, false, true),
        (1, 1, 3, false, true),
        (1, 0, 0x40, false, false),
        (2, 0, 3, true, true),
        (0, 0, 3, false, true),
    ] {
        t.set_u32(w(&t, 0x04), count.cast_unsigned());
        t.set_u32(w(&t, 0x16), flag);
        t.set_u32(w(&t, 0x1e), sound.cast_unsigned());
        t.set_u32(w(&t, 0x40), 0x00ab_0040);
        host(&mut t, ro, ms, busy, done);
        run(&mut t, at);
    }
}
