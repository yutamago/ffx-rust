//! `magic_0045.dll` against its Rust port (`ffx_magic::m0045`): `DllMain`
//! against every bound port, the object callbacks and the start-up.
//! Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{f, filler, keeper, rec, recs, twin};

const N: u16 = 45;
const OBJ: u32 = SCRATCH + 0x2000;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The callbacks' slots: the part list, the matrices the camera slots
/// fill, the buffers they get kept per call.
fn stubs(t: &mut Twin) -> u32 {
    let at = t.at(0x1250);
    let gp = t.img.word(at + 0x04);
    t.set_u32(gp, SCRATCH + 0x200);
    let gd = t.img.word(at + 0x10);
    t.set_u32(gd, SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    let list = SCRATCH + 0x400;
    for i in 0..16u32 {
        t.set_u32(list + 4 * i, 0x1100 + i);
    }
    rec(t, 941, 1, move |_, _| list);
    let lock = t.img.word(t.at(0x16b0) + 0x16e);
    t.set_u32(lock, SCRATCH + 0x204);
    t.set_u32(SCRATCH + 0x204, 9);
    // pppStartPart records the lock it runs under
    rec(t, 714, 5, |m, _| {
        let v = m.r32(SCRATCH + 0x204);
        m.w32(SCRATCH + 0x220, v);
        0x714
    });
    rec(t, 712, 7, |m, _| {
        let v = m.r32(SCRATCH + 0x204);
        m.w32(SCRATCH + 0x224, v);
        m.r32(SCRATCH + 0x208)
    });
    recs(t, &[(713, 1), (724, 5), (328, 3)]);
    filler(t, 236, 3, 16);
    filler(t, 235, 1, 16);
    filler(t, 327, 7, 16);
    keeper(t, 232, 5, 0, 4, 0x100);
    keeper(t, 230, 5, 1, 16, 0x140);
    keeper(t, 231, 3, 2, 16, 0x180);
    rec(t, 229, 9, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, f(2.5 - i as f32));
        }
        0x229
    });
    keeper(t, 325, 11, 1, 4, 0x1c0);
    SCRATCH + 0x208
}

#[test]
fn objects() {
    let Some(mut t) = twin(N) else { return };
    let done = stubs(&mut t);
    let base = t.img.word(t.at(0x1250) + 0x3d);
    for obj in [OBJ, base, base + 0x80] {
        for e in [0x1250, 0x1470, 0x15a0, 0x18b0] {
            t.write(obj + 0x40, &[0x5a; 0x30]);
            t.check_entry(t.at(e), Call::default(), &[obj], Eax::Checked);
        }
    }
    for r in [0u32, 1, 2] {
        t.set_u32(done, r);
        for e in [0x16b0, 0x19c0] {
            t.set_u32(OBJ + 0x60, f(0.75 + r as f32));
            t.check_entry(t.at(e), Call::default(), &[OBJ], Eax::Checked);
        }
    }
}

#[test]
fn start_up() {
    let Some(mut t) = twin(N) else { return };
    let e = t.at(0x1c10);
    let lock = t.img.word(e + 0x83);
    t.set_u32(lock, SCRATCH + 0x204);
    t.set_u32(SCRATCH + 0x204, 5);
    for i in 0..8u32 {
        t.set_u32(SCRATCH + 0x600 + i * 4, 0x0600_0000 + i);
    }
    rec(&mut t, 803, 2, |_, _| SCRATCH + 0x1_0000);
    rec(&mut t, 719, 10, |_, a| SCRATCH + 0x600 + a[1]);
    rec(&mut t, 711, 13, |m, _| {
        let v = m.r32(SCRATCH + 0x204);
        m.w32(SCRATCH + 0x220, v);
        0x711
    });
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x941);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 + a[0]);
    recs(&mut t, &[(718, 3), (222, 7), (710, 17)]);
    for o in [0x127, 0x13d, 0x18a, 0x1af] {
        let b = t.img.word(e + o);
        t.write(b, &[0xa5; 0x110]);
    }
    for arg in [0u32, 7] {
        t.check_entry(e, Call::default(), &[arg], Eax::Checked);
    }
}
