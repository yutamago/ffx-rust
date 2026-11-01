//! `magic_0397.dll` against its Rust port (`ffx_magic::m0397`): the
//! task's end. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 397;

/// 0x10004250: paused, the script or the motion still running, and the
/// end (the node alone given back to the pool).
#[test]
fn script_end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4250);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let paused = SCRATCH + 0x600;
    rec(&mut t, 561, 0, move |m, _| m.r32(paused));
    rec(&mut t, 628, 1, |_, a| a[0] ^ 0x628);
    let e = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x07), e);
    let run = t.img.rel32_target(at + 0x0c);
    let ms = t.img.rel32_target(at + 0x25);
    let give = t.img.rel32_target(at + 0x54);
    let s = t.img.word(run + 0x1c);
    let node = SCRATCH + 0x2000;
    for (p, mode, done) in [(1u32, 0u8, true), (0, 2, true), (0, 0, false), (0, 0, true)] {
        t.set_u32(paused, p);
        // the script ended (mode 0, returns 1) or waiting for ever (2)
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u16(s + 0x54, 0xffff);
        t.set_u32(s + 0x24, 0);
        t.set_u32(s + 0x28, 0);
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(e + 0x2408, 0x2408);
        // the node alone (no siblings, nothing chained) for the pool
        t.write(node, &[0; 0x1c]);
        t.set_u32(node + 8, node);
        for op in [0x48, 0x55, 0x64, 0x74] {
            t.set_u32(t.img.word(give + op), SCRATCH + 0x3000 + op);
        }
        t.set_u32(SCRATCH + 0x3074, SCRATCH + 0x3074);
        t.set_u32(SCRATCH + 0x3078, SCRATCH + 0x3074);
        t.set_u16(t.img.word(give + 0x8e), 7);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}

/// 0x10005800: the caster (-1) and a party actor.
#[test]
fn actor_position() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5800);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 262, 4, |m, a| {
        for (i, at) in (0u32..).zip(&a[1..4]) {
            m.w32(*at, 0x0262_0000 + i);
        }
        0x262
    });
    rec(&mut t, 268, 3, |_, a| 0x0268_0000 ^ a[0]);
    t.set_u32(w(&t, 0x1b), 0x00ca_001b);
    let bytes = SCRATCH + 0x700;
    t.write(bytes, &[3, 1, 2, 0]);
    t.set_u32(w(&t, 0x22), bytes);
    let table = w(&t, 0x2d);
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x0b00_0000 + i);
    }
    let obj = SCRATCH + 0x1000;
    for i in [0xffff_ffffu32, 1] {
        t.write(obj, &[0x5c; 0x130]);
        t.set_u32(obj + 0x120, i);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x100064b0: no party, members all answering 0, one answering, the
/// caster's own byte among them.
#[test]
fn party_check() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x64b0);
    let list = SCRATCH + 0x700;
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], m.r32(SCRATCH + 0x6f0));
        list
    });
    rec(&mut t, 637, 1, |m, a| {
        u32::from(a[0] == m.r32(SCRATCH + 0x6f4))
    });
    let own = t.img.word(at + 0x21);
    t.set_u8(own, 7);
    let state = SCRATCH + 0x1000;
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for (n, bytes, yes) in [
        (0u32, [1u8, 2, 3, 7], 0xffu32),
        (3, [1, 2, 3, 7], 0xff),
        (3, [1, 2, 3, 7], 2),
        (4, [1, 2, 3, 7], 0xff),
        (4, [7, 9, 3, 7], 3),
    ] {
        t.write(list, &bytes);
        t.set_u32(SCRATCH + 0x6f0, n);
        t.set_u32(SCRATCH + 0x6f4, yes);
        t.write(state, &[0x22; 0x60]);
        t.check_entry(at, Call::default(), &[state], Eax::Checked);
    }
}

/// 0x10006240: the two sprite tasks into an empty and a filled list.
#[test]
fn sprite_tasks() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6240);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x04), fx);
    let take = t.img.rel32_target(at + 0x67);
    let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let node = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
    let last = SCRATCH + 0x1a00;
    for first in [0u32, last] {
        for k in 0..4 {
            t.write(node(k), &[0x5a; 0x40]);
            t.set_u32(node(k), node((k + 3) % 4));
            t.set_u32(node(k) + 4, node((k + 1) % 4));
        }
        t.set_u32(head, node(0));
        t.set_u16(count, 4);
        t.write(last, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x6d), first);
        t.set_u32(w(&t, 0x83), first);
        t.write(fx + 0x1a40, &[0x44; 0xc0]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10004560: start and target on each side of 0 (near and more than pi
/// apart), NaN, the count at 0, 1 and 20, and blends past the range.
#[test]
fn camera_turn() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4560);
    rec(&mut t, 34, 2, |_, a| 0x0034_0000 ^ a[1]);
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x02), fx);
    t.set_u32(t.img.word(at + 0xf3), 0x00ab_00f3);
    let pairs = [
        (-1.0f32, 2.0f32),
        (-2.5, 2.5),
        (2.5, -2.5),
        (1.0, -1.0),
        (0.5, 1.5),
        (-0.5, -1.5),
        (0.0, -3.0),
        (f32::NAN, 1.0),
        (1.0, f32::NAN),
        (3.1, 3.1),
        (-3.1, -3.1),
    ];
    let counts = [0u32, 1, 20, 0x8000_0000].into_iter().cycle();
    let mut cases: Vec<(f32, f32, u32)> = pairs
        .into_iter()
        .zip(counts)
        .map(|((a, b), c)| (a, b, c))
        .collect();
    // with the count left at 0 the wrap undoes the turn round: these keep
    // a blend (count 2), put a or b at 0 exactly and the blend on PI and
    // LO themselves
    let pi = f32::from_bits(t.u32(t.img.word(at + 0x33)));
    let lo = f32::from_bits(t.u32(t.img.word(at + 0x2b)));
    cases.extend([
        (-2.5, 2.5, 2),
        (2.5, -2.5, 2),
        (0.0, -3.2, 2),
        (-3.2, 0.0, 2),
        (3.2, 0.0, 2),
        (0.5, pi, 0),
        (-0.5, lo, 0),
    ]);
    for (a, b, count) in cases {
        t.write(fx + 0x23f0, &[0x77; 0x10]);
        t.set_u32(fx + 0x23f4, a.to_bits());
        t.set_u32(fx + 0x23f8, b.to_bits());
        t.set_u32(fx + 0x23fc, count);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001120 (internal): the party's mean position for no, one and three
/// actors (junk under the entry for the frame).
#[test]
fn party_mean() {
    use magic::StubMem;
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1120);
    let list = SCRATCH + 0x700;
    let count = SCRATCH + 0x6f0;
    // the second actor's x not 0, so the sums' order shows
    t.write(list, &[4, 1, 9, 2]);
    rec(&mut t, 288, 2, move |m: &mut dyn StubMem, a: &[u32]| {
        let n = m.r32(count);
        m.w32(a[1], n);
        list
    });
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(&mut t, 262, 5, |m: &mut dyn StubMem, a: &[u32]| {
        for (k, at) in (0u32..).zip(&a[1..4]) {
            let v = f32::from(u16::try_from(a[0] & 0xff).unwrap()) * 0.75
                + 0.5 * f32::from(u16::try_from(k).unwrap());
            m.w32(*at, v.to_bits());
        }
        0x262
    });
    let out = SCRATCH + 0x1000;
    for n in [0u32, 1, 3, 0x8000_0000] {
        t.set_u32(count, n);
        t.write(out, &[0x5c; 0x10]);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check(
            at,
            Call::this(out),
            &[],
            Eax::Checked,
            ffx_magic::m0397::party_mean,
        );
    }
}

/// 0x10004660: the point below frame 0xc0 and the turned point after it
/// (junk under the entry for the frame).
#[test]
fn point_step() {
    use magic::StubMem;
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4660);
    rec(&mut t, 237, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for k in 0..3 {
            m.w32(a[3] + 4 * k, 0x4100_0000 + k);
        }
        0x237
    });
    rec(&mut t, 235, 1, |m: &mut dyn StubMem, a: &[u32]| {
        for k in 0..16 {
            m.w32(a[0] + 4 * k, 0x3f00_0000 + k);
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(&mut t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
            let v = m.r32(a[0]);
            m.w32(a[0], v ^ a[2] ^ s);
            s
        });
    }
    rec(&mut t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for k in 0..4 {
            let v = m.r32(a[1] + 4 * k) ^ m.r32(a[2] + 4 * k);
            m.w32(a[0] + 4 * k, v);
        }
        0x229
    });
    rec(&mut t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for k in 0..3 {
            let v = m.r32(a[2] + 4 * k);
            m.w32(a[0] + 4 * k, v ^ 0x248);
        }
        0x248
    });
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x68), fx);
    t.set_u32(fx + 0x23f8, 0x3fc9_0fdb);
    t.set_u32(t.img.word(at + 0x35), 0x00ab_0035);
    let obj = SCRATCH + 0x1000;
    for frame in [0u32, 0xbf, 0xc0, 0x200] {
        t.write(obj, &[0x5c; 0x120]);
        t.set_u32(obj + 0x118, frame);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
