//! `magic_0165.dll` against its Rust port (`ffx_magic::m0165`): the
//! camera look-at points, the point copy and the sounds' fade. Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{f, rec, twin};

const N: u16 = 165;

/// The object behind the DLL's pointer global, its words floats.
fn object(t: &mut Twin, at: u32, op: u32) -> u32 {
    let p = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + op), p);
    for i in 0..0x280 {
        #[allow(clippy::cast_precision_loss)]
        t.set_u32(p + 4 * i, f(0.5 + i as f32 * 0.125));
    }
    p
}

/// 0x10004890 and 0x100047d0: the look-at points.
#[test]
fn look_points() {
    let Some(mut t) = twin(N) else { return };
    rec(&mut t, 327, 2, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x0100);
        }
        0x327
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v & 0x3fff_ffff);
        }
        0x229
    });
    for rva in [0x4890, 0x47d0] {
        let at = t.at(rva);
        object(&mut t, at, 0x13);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10004ac0: the point copy.
#[test]
fn copy_point() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4ac0);
    object(&mut t, at, 0x02);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x100035a0: the fade with the countdown above 0 and at 0, the flag
/// set and clear, the list's bytes below and from 0x20, volumes of both
/// signs, and no sounds.
#[test]
fn sound_fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x35a0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 249, 2, |_, a| a[0] ^ a[1] ^ 0x249);
    rec(&mut t, 530, 2, |_, a| a[0] ^ a[1] ^ 0x530);
    let p = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x09), p);
    for (i, v) in (0u32..).zip([120i32, -300, 7, 0x7fff_0000, 60]) {
        t.set_u32(p + 0x5eec + 4 * i, v.cast_unsigned());
    }
    let list = SCRATCH + 0x700;
    t.write(list, &[3, 0x20, 0x1f, 0x45, 0]);
    t.set_u32(w(&t, 0x56), list);
    let table = w(&t, 0x9b);
    for b in [3u32, 0x20, 0x1f, 0x45, 0] {
        t.set_u32(table + 4 * b, 0x00ab_0000 + b);
    }
    let task = SCRATCH + 0x100;
    for (count, flag, n) in [
        (5i32, 0u32, 5u32),
        (0, 0, 5),
        (3, 1, 5),
        (-2, 1, 3),
        (9, 0, 0),
    ] {
        t.write(task, &[0x11; 0x40]);
        t.set_u32(task + 0x30, count.cast_unsigned());
        t.set_u32(w(&t, 0x4f), flag);
        t.set_u32(w(&t, 0x1e), n);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// A free ring of four pool nodes for the pool's `take` at `take`.
fn ring(t: &mut Twin, take: u32) {
    let node = |i: u32| SCRATCH + 0x2000 + 0x40 * i;
    let (count, free) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    for i in 1..=4 {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node(if i == 1 { 4 } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == 4 { 1 } else { i + 1 }));
    }
    t.set_u32(free, node(1));
    t.set_u16(count, 4);
}

/// The task-list starts: 0x10002cc0, 0x10004b50, 0x10003750 and
/// 0x100044e0, into an empty and a filled list.
#[test]
fn task_starts() {
    let Some(mut t) = twin(N) else { return };
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, 0x0010_0000 + i);
        }
        0x237
    });
    rec(&mut t, 713, 1, |_, a| a[0] ^ 0x713);
    let tail = SCRATCH + 0x900;
    for (rva, take, head, tl, list) in [
        (0x2cc0u32, 0x9eu32, 0xa4u32, 0xb7u32, None),
        (0x4b50, 0xf1, 0xf7, 0x10b, None),
        (0x3750, 0x131, 0x137, 0x14d, Some(0x0au32)),
        (0x44e0, 0x67, 0x6d, 0x82, None),
    ] {
        let at = t.at(rva);
        let w = |t: &Twin, off: u32| t.img.word(at + off);
        let p = object(&mut t, at, if rva == 0x2cc0 { 0x18 } else { 0x05 });
        if rva == 0x4b50 {
            t.set_u32(w(&t, 0x02), p);
        }
        if rva == 0x2cc0 {
            t.set_u32(w(&t, 0x11), SCRATCH + 0x880);
            t.set_u32(SCRATCH + 0x888, 0x00a0_0000);
            // the sprite's handle (a global)
            t.set_u32(w(&t, 0x39), 0x0039_0039);
        }
        if let Some(op) = list {
            // the object list: the second object listed, the others not
            let l = w(&t, op);
            let (n1, n2) = (SCRATCH + 0x3000, SCRATCH + 0x3040);
            t.write(n1, &[0; 0x20]);
            t.write(n2, &[0; 0x20]);
            t.set_u32(l, n1);
            t.set_u32(l + 4, n2);
            t.set_u32(l + 8, n1);
            t.set_u32(n1 + 4, n2);
            t.set_u32(n2, n1);
            // the object of index 2 (p + 2 * 0x130) and p + 0x390 listed,
            // their flags (u16 +0x124) with bit 15 and a different +0x128
            t.set_u32(n1 + 8, p + 0x390);
            t.set_u32(n2 + 8, p + 0x260);
            t.set_u32(p + 0x5f1c, 2);
            for obj in [p + 0x260, p + 0x390] {
                t.set_u16(obj + 0x124, 0x8123);
                t.set_u16(obj + 0x128, 0x0456);
            }
        }
        let take_f = t.img.rel32_target(at + take);
        for filled in [false, true] {
            // the effect's blocks past the floats hold junk (each store
            // must show), the selected object's index kept
            t.write(p + 0x5800, &[0x6d; 0x700]);
            t.set_u32(p + 0x5f1c, 2);
            ring(&mut t, take_f);
            t.write(tail, &[0x66; 0x20]);
            t.set_u32(w(&t, head), if filled { tail } else { 0 });
            t.set_u32(w(&t, tl), if filled { tail } else { 0 });
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// 0x10004930: the follow with slot 520 answering 0x1590000 or not.
#[test]
fn look_follow() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4930);
    rec(&mut t, 327, 2, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x0100);
        }
        0x327
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v & 0x3fff_ffff);
        }
        0x229
    });
    let ans = SCRATCH + 0x600;
    rec(&mut t, 520, 0, move |m, _| {
        let k = m.r32(ans + 8);
        m.w32(ans + 8, k + 1);
        m.r32(ans + 4 * (k & 1))
    });
    object(&mut t, at, 0x13);
    for (a, b) in [
        (0x0159_0000u32, 0x0159_0000u32),
        (0, 0x0159_0000),
        (0x0159_0000, 3),
        (1, 2),
    ] {
        t.set_u32(ans, a);
        t.set_u32(ans + 4, b);
        t.set_u32(ans + 8, 0);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
