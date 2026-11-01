//! `magic_0292.dll` and `magic_0293.dll` against their Rust ports
//! (`ffx_magic::m0292`). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// The draws' slots; 712 answers the case's word.
fn stubs(t: &mut Twin) {
    for (s, n) in [(258u32, 1usize), (231, 3), (230, 2), (714, 5)] {
        rec(t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    rec(t, 712, 2, |m, _| m.r32(SCRATCH + 0x6f0));
    // slot 713 copies the flag (`SCRATCH + 0x600`) it sees out
    rec(t, 713, 1, |m, a| {
        let seen = m.r32(SCRATCH + 0x600);
        m.w32(SCRATCH + 0x6e0, seen);
        (0x0100_0000 + 713) ^ a[0]
    });
}

/// `magic_0293` 0x10001070 and `magic_0292` 0x10001110: slot 712
/// answering 0 and not, the flag set and clear before.
#[test]
fn draws() {
    for (n, rva) in [(293u16, 0x1070u32), (292, 0x1110)] {
        let Some(mut t) = twin(n) else { return };
        stubs(&mut t);
        let at = t.at(rva);
        let w = |t: &Twin, off: u32| t.img.word(at + off);
        let lock = SCRATCH + 0x600;
        t.set_u32(w(&t, 0x4d), lock);
        t.set_u32(w(&t, 0x52), SCRATCH + 0x640);
        if rva == 0x1110 {
            // the restart's program list (slot 941 answers its word)
            rec(&mut t, 941, 1, |_, _| SCRATCH + 0x700);
            t.set_u32(SCRATCH + 0x700, 0x0900_0000);
            let gp = SCRATCH + 0x680;
            t.set_u32(w(&t, 0xb7), gp);
            t.set_u32(gp + 0x10, 0x00de_0010);
            // the restart's block pointer, apart from the draw's
            t.set_u32(w(&t, 0x8f), SCRATCH + 0x650);
            let sp = t.img.rel32_target(at + 0xdc);
            t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
        }
        for (drawn, old) in [(0u32, 0u32), (5, 0x77), (0, 1)] {
            t.set_u32(lock, old);
            t.set_u32(SCRATCH + 0x6f0, drawn);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `magic_0292` 0x10001000: the part's start-up.
#[test]
fn start_up() {
    let Some(mut t) = twin(292) else { return };
    let at = t.at(0x1000);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x700);
    t.set_u32(SCRATCH + 0x700, 0x0900_0000);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(&mut t, 405, 1, |_, _| 0x0405_0405);
    rec(&mut t, 406, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, 0x0406_0000 + i);
        }
        0x406
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, m.r32(a[0] + 4 * i) ^ 0x325);
        }
        0x325
    });
    let gp = SCRATCH + 0x680;
    t.set_u32(w(&t, 0x1c), gp);
    t.set_u32(gp + 0x10, 0x00de_0010);
    let sp = t.img.rel32_target(at + 0x41);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x6c0);
    t.set_u32(w(&t, 0x11), SCRATCH + 0x640);
    for v in [1.5f32, -0.25] {
        t.set_u32(w(&t, 0xac), v.to_bits());
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
