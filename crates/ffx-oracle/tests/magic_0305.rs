//! `magic_0305.dll` against its Rust ports (`ffx_magic::m0305`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 305;

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

/// 0x10001bf0: each countdown reaching 0 or not, the motion step and the
/// objects running or done, the flag bit 15 set or clear.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    stubs(&mut t);
    let at = t.at(0x1bf0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let ms = t.img.rel32_target(at + 0x7d);
    let ro = t.img.rel32_target(at + 0x84);
    // one part for the tail's register_parts (slots 941, 772, 779), its
    // lock at P + 0x200, so the callee's address matters
    let rp = t.img.rel32_target(at + 0xb4);
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
    for (a, b, busy, done, bits) in [
        (1i32, 1i32, false, true, 0x8000u32),
        (2, 1, false, true, 0x8000),
        (1, 0, false, false, 0x8000),
        (0, 3, true, true, 0x8000),
        (1, 1, false, true, 0),
    ] {
        t.set_u32(w(&t, 0x04), a.cast_unsigned());
        t.set_u32(w(&t, 0x40), b.cast_unsigned());
        t.set_u32(w(&t, 0x8f), bits);
        host(&mut t, ro, ms, busy, done);
        run(&mut t, at);
    }
}
