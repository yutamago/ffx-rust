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
