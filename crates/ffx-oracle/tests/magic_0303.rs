//! `magic_0303.dll` against its Rust ports (`ffx_magic::m0303`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 303;

/// 0x10001370: the motion step, the scripts and the objects running or
/// done; the node there or not; the two sound branches.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1370);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(296u32, 2usize), (257, 2), (719, 3), (306, 3)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let ms = t.img.rel32_target(at + 0x07);
    let ro = t.img.rel32_target(at + 0x26);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    let node = SCRATCH + 0x3000;
    let ctxs = [w(&t, 0x0c), w(&t, 0x18)];
    // the sound *T of slot 257's branch
    t.set_u32(w(&t, 0x80), 0x00ab_0080);
    for (done, s1, s2, busy, with_node, flag, sound) in [
        (true, 0u8, 0u8, false, true, 0u32, 3i32),
        (true, 0, 0, false, false, 1, 3),
        (true, 0, 0, false, true, 0, 0x20),
        (false, 0, 0, false, true, 0, 3),
        (true, 2, 0, false, true, 0, 3),
        (true, 0, 2, false, true, 0, 3),
        (true, 0, 0, true, true, 0, 3),
    ] {
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        // the scripts: mode byte +0x56 (0 ended), wait +0x54 forever
        for (k, c) in ctxs.into_iter().enumerate() {
            t.write(c, &[0; 0x60]);
            t.set_u8(c + 0x56, if k == 1 { s2 } else { s1 });
            t.set_u16(c + 0x54, 0xffff);
        }
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        t.write(node, &[0x4c; 0x20]);
        t.set_u32(w(&t, 0x3d), if with_node { node } else { 0 });
        t.set_u32(w(&t, 0x56), flag);
        t.set_u32(w(&t, 0x5e), sound.cast_unsigned());
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10001540: the caller placed, in a battle and outside it as a party
/// member and as another actor (grounded or not).
#[test]
fn place() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1540);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(
                a[1 + i],
                (1.5f32 + f32::from(u8::try_from(i).unwrap())).to_bits(),
            );
        }
        0x262
    });
    // slot 255 copies the vector out (its fourth word 1.0 included)
    rec(&mut t, 255, 2, |m, a| {
        magic_owna::dump(m, 255, a[1], 4);
        0x0100_0255 ^ a[0]
    });
    for (s, n) in [(33u32, 4usize), (32, 3)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    rec(&mut t, 11, 1, |m, _| m.r32(SCRATCH + 0x6f0));
    let pm = t.img.rel32_target(at + 0x5f);
    let pw = |t: &Twin, off: u32| t.img.word(pm + off);
    t.set_u32(t.img.word(at + 0x23), 0x00ab_0023);
    for (battle, me, grounded) in [(0u32, 3u32, 0u32), (1, 3, 0), (0, 0x40, 1), (1, 5, 1)] {
        // the frame scrubbed each case (the vector's 1.0 is not left over)
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.set_u32(pw(&t, 0x02), battle);
        t.set_u32(pw(&t, 0x0d), me);
        t.set_u32(pw(&t, 0x32), 0x0b0b_0032);
        t.set_u32(SCRATCH + 0x6f0, grounded);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
