//! `magic_0512.dll` against its Rust port (`ffx_magic::m0512`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0512};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 512;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10003680: no parent, a parent with no children and one with some.
#[test]
fn insert() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3680);
    recs(&mut t, &[(235, 1), (230, 3), (325, 2)]);
    let (group, node, parent, last) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x2100,
    );
    for case in 0..3 {
        t.write(node, &[0x47; 0x160]);
        t.write(parent, &[0x48; 0x20]);
        t.set_u32(parent + 0x10, if case == 1 { parent } else { last });
        t.set_u16(parent + 0x18, 5);
        t.write(last, &[0x49; 0x10]);
        for (i, v) in (0u32..).zip([node, if case == 0 { 0 } else { parent }, 0xa, 0xb, 0xc, 0xd]) {
            t.set_u32(group + 4 * i, v);
        }
        t.check(at, Call::this(group), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0512::insert(g, img, fr.at, fr.ecx))
        });
    }
}

/// 0x100088b0: every state, both ways where it branches.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x88b0);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0x5a), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (325, 2)]);
    // the node list (the record's list) empty: its last node itself
    let list = t.img.word(at + 0x6d);
    t.set_u32(list + 0x10, list);
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    for (state, wait, flags) in [
        (0u32, 0u32, 0u8),
        (1, 3, 0),
        (1, 0, 0),
        (1, 0xffff_ffff, 0),
        (2, 0, 0),
        (2, 0, 4),
        (0x2710, 0, 0),
        (7, 0, 0),
        (0x4e20, 0, 0),
    ] {
        t.write(r, &[0x3c; 0x300]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, wait);
        t.set_u8(r + 0x2d8, flags);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10007830: the object set-up, the node list empty and not (its words
/// and the ring's distinct), for a few `n`.
#[test]
fn setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7830);
    let list = t.img.word(at + 0x5d);
    let ring_last = t.img.word(at + 0x25);
    let ring_flags = t.img.word(at + 0x43);
    let obj = SCRATCH + 0x1000;
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (n, empty) in [(0u32, true), (1, false), (3, true), (0xffff_fffe, false)] {
        t.write(obj, &[0x5b; 0x200]);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: 0x00ed_0000 + n,
            ..Call::default()
        };
        t.check(at, c, &[n], Eax::Checked, |g, img, fr| {
            Ret::eax(m0512::setup(g, img, fr))
        });
    }
}

/// 0x10008670: the set-up from a source object, the node list empty and
/// not, for a few `n`; the source's block and the ring distinct.
#[test]
fn setup_from() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8670);
    let list = t.img.word(at + 0x5a);
    let ring_last = t.img.word(at + 0x22);
    let ring_flags = t.img.word(at + 0x40);
    recs(&mut t, &[(325, 2), (719, 3)]);
    let obj = SCRATCH + 0x1000;
    let src = SCRATCH + 0x1400;
    let (old, prev, blk) = (SCRATCH + 0x800, SCRATCH + 0x900, SCRATCH + 0xa00);
    for (n, empty) in [(0u32, true), (1, false), (3, true), (0xffff_fffe, false)] {
        t.write(obj, &[0x5b; 0x200]);
        t.set_u32(obj + 0x188, 0x3fc0_0000 + n);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        let mut b = [0u8; 0x40];
        for (i, x) in b.chunks_mut(4).enumerate() {
            x.copy_from_slice(&(0x0b00_0000 + i as u32 * 0x11).to_le_bytes());
        }
        t.write(blk, &b);
        t.write(src, &[0x3c; 0x200]);
        t.set_u32(src + 0x140, SCRATCH + 0xb00);
        t.set_u32(src + 0x144, blk);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: src,
            ..Call::default()
        };
        t.check(at, c, &[n], Eax::Checked, |g, img, fr| {
            Ret::eax(m0512::setup_from(g, img, fr))
        });
    }
}

/// 0x10008980: the ground object's set-up, the node list empty and not.
#[test]
fn ground() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8980);
    let list = t.img.word(at + 0x5a);
    let ring_last = t.img.word(at + 0x22);
    let ring_flags = t.img.word(at + 0x40);
    let obj = SCRATCH + 0x1000;
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (k, empty) in [(0u32, true), (1, false)] {
        t.write(obj, &[0x5b; 0x200]);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: 0x00ed_0000 + k,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0512::ground(g, img, fr))
        });
    }
}
