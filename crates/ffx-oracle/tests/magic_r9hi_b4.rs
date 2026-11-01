//! Round 9 (own rows from `magic_0450` up), batch 4: small functions of
//! `magic_0473`, `0511`, `0537`, `0577`, `0587`, `0605` and `0671` against their Rust
//! ports. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

mod magic;
mod magic_l3;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::rec;

/// Every slot in `slots` recorded with its word count, answering a
/// distinct value mixed with its first word.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (0x0abc_0000 | s) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `m0577::draw_detached_arg` (0x100025d0): the holder's node in a list
/// and in none.
#[test]
fn draw_detached_arg_0577() {
    let Some(mut t) = magic_l3::twin(577) else {
        return;
    };
    let f = t.at(0x25d0);
    recs(&mut t, &[(332, 1), (353, 2), (267, 3), (271, 4), (669, 5)]);
    let (holder, node, list, a, b) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
        SCRATCH + 0x340,
    );
    for inlist in [false, true] {
        t.write(node, &[0; 0x100]);
        t.write(list, &[0x2c; 0x20]);
        t.write(a, &[0x2a; 0x80]);
        for i in 0..4 {
            t.set_u32(node + 0x7c + 4 * i, (1.6 * i as f32 - 2.75).to_bits());
        }
        t.set_u32(holder, node);
        t.set_u32(node + 4, a);
        t.set_u32(node + 8, b);
        t.set_u32(node + 0x14, if inlist { list } else { 0 });
        t.check_entry(f, Call::default(), &[holder], Eax::Checked);
    }
}

/// `m0587::reload_groups` (0x10002f20): no entries and some.
#[test]
fn reload_groups_0587() {
    let Some(mut t) = magic_l3::twin(587) else {
        return;
    };
    let f = t.at(0x2f20);
    let lg = t.img.rel32_target(f + 0x10);
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x0941_0000);
    rec(&mut t, 772, 2, |_, a| a[0] ^ a[1]);
    t.stub_edx(772, |_, a| a[1] + 0x77);
    rec(&mut t, 779, 2, |_, a| 0x5a5a_0779 ^ a[0]);
    let flag = SCRATCH + 0x10;
    t.set_u32(t.img.word(lg + 0x5b), flag);
    t.set_u32(flag, 0x00f1_0000);
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(f + 0x1), r);
    for n in [0u32, 2] {
        t.set_u32(r + 0x30fc, 0x30fc);
        for k in 0..4u32 {
            t.set_u32(r + 0x166c + 0x8a8 * k, if k == 1 { n } else { 0 });
            t.set_u32(r + 0xe48 + 0x8a8 * k, 0x00e0_0000 + k);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0473::end_objects` (0x100020e0): slot 239, then the (empty) object
/// list freed.
#[test]
fn end_objects_0473() {
    let Some(mut t) = magic_l3::twin(473) else {
        return;
    };
    let f = t.at(0x20e0);
    recs(&mut t, &[(239, 3), (713, 1)]);
    t.set_u32(t.img.word(f + 0x11), 0x00a5_0011);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

/// `m0537::draw_entry` (0x10001230): the parts of entries 0 and 3, slot
/// 712 answering 0 and not.
#[test]
fn draw_entry_0537() {
    let Some(mut t) = magic_l3::twin(537) else {
        return;
    };
    let f = t.at(0x1230);
    let fd = t.img.rel32_target(f + 0x19);
    let (gp, lock) = (SCRATCH + 0x20, SCRATCH + 0x24);
    t.set_u32(t.img.word(fd + 0x4b), gp);
    t.set_u32(t.img.word(fd + 0x58), lock);
    let table = SCRATCH + 0x800;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0b00_0000 + 0x11 * i);
    }
    // the effect object whose `+0x10` slot 941 gets (the restart's)
    let o = SCRATCH + 0x100;
    t.set_u32(0x1003_0fa4, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 712, 2, |m, _| m.r32(SCRATCH + 0x10));
    recs(&mut t, &[(258, 1), (231, 3), (230, 2), (713, 1), (714, 5)]);
    let base = t.img.word(f + 0x8);
    for (j, s712) in [(0u32, 0u32), (3, 1)] {
        t.set_u32(SCRATCH + 0x10, s712);
        t.set_u32(lock, 0x4444 + j);
        t.check_entry(f, Call::default(), &[base + 4 * j], Eax::Checked);
    }
}

/// `m0605::scale_30` (0x1000aef0): the task from a free ring, the list
/// empty and holding a node.
#[test]
fn scale_30_0605() {
    let Some(mut t) = magic_l3::twin(605) else {
        return;
    };
    let f = t.at(0xaef0);
    let block = SCRATCH + 0x8000;
    t.set_u32(t.img.word(f + 0x2), block);
    let s = t.img.rel32_target(f + 0x22);
    let take = t.img.rel32_target(s + 0x73);
    for listed in [false, true] {
        let node = |i: u32| SCRATCH + 0x4000 + 0x40 * i;
        for i in 1..=3u32 {
            t.write(node(i), &[0x5a; 0x1c]);
            t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
            t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
        }
        t.set_u32(t.img.word(take + 0x17), node(1));
        t.set_u16(t.img.word(take + 0x04), 3);
        t.write(block + 0x2d80, &[0x44; 0x60]);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(t.img.word(s + 0x79), SCRATCH + 0x5000);
            t.set_u32(t.img.word(s + 0x91), SCRATCH + 0x5000);
        } else {
            t.set_u32(t.img.word(s + 0x79), 0);
        }
        t.check_entry(f, Call::this(0x0bad_0ecc), &[], Eax::Checked);
    }
}

/// `m0511::place_at_point` (0x10003df0): two model bytes, the object's
/// point at `+0xd0`.
#[test]
fn place_at_point_0511() {
    let Some(mut t) = magic_l3::twin(511) else {
        return;
    };
    let f = t.at(0x3df0);
    rec(&mut t, 219, 2, |_, a| 0x0219_0000 ^ a[0]);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, a[0] ^ (i << 8));
        }
        0x268
    });
    let mb = SCRATCH + 0x40;
    t.set_u32(t.img.word(f + 0x4), mb);
    let obj = SCRATCH + 0x1000;
    for b in [0x5a5a_5a37u32, 0x0000_01c4] {
        t.set_u32(mb, b);
        t.write(obj, &[0x33; 0x100]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `m0671::end` (0x100014e0): the flag 1 (the empty object list freed)
/// and not (the caller's `eax` kept).
#[test]
fn end_0671() {
    let Some(mut t) = magic_l3::twin(671) else {
        return;
    };
    let f = t.at(0x14e0);
    recs(&mut t, &[(311, 1), (713, 1)]);
    let obj = SCRATCH + 0x1000;
    for flag in [0u32, 1, 2] {
        t.set_u32(t.img.word(f + 0x5), flag);
        t.set_u32(t.img.word(f + 0x18), 0x0018_0000 + flag);
        t.write(obj, &[0x33; 0x80]);
        let c = Call {
            eax: 0x0eaa_0000 + flag,
            ..Call::default()
        };
        t.check_entry(f, c, &[obj], Eax::Checked);
    }
}
