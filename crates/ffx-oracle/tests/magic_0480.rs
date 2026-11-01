//! `magic_0480.dll` against its Rust port (`ffx_magic::m0480`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0480};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{recs, twin};

const N: u16 = 480;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002760: no node; a lone node into an empty free ring; a node with
/// siblings and two children that is the pool's cursor, head and tail,
/// into a ring, the count reaching 0x100.
#[test]
fn clear() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2760);
    recs(&mut t, &[(590, 2), (589, 2)]);
    let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
    let gb = t.img.rel32_target(at + 0x84);
    let n = SCRATCH + 0x1000;
    let (prev, next, c1, c2, c3) = (
        SCRATCH + 0x1100,
        SCRATCH + 0x1200,
        SCRATCH + 0x1300,
        SCRATCH + 0x1400,
        SCRATCH + 0x1500,
    );
    let ring = SCRATCH + 0x1600;
    for case in 0..3 {
        t.write(SCRATCH + 0x1000, &[0x31; 0x700]);
        for g in [gb, at] {
            let (cur, head, tail, free, count) = if g == at {
                (0xaa, 0xb7, 0xc6, 0xd6, 0xf0)
            } else {
                (0x48, 0x55, 0x64, 0x74, 0x8e)
            };
            let [cur, head, tail, free, count] =
                [cur, head, tail, free, count].map(|op: u32| t.img.word(g + op));
            let mark = if case == 2 { n } else { SCRATCH + 0x3f00 };
            t.set_u32(cur, mark);
            t.set_u32(head, mark);
            t.set_u32(tail, mark);
            t.set_u32(free, if case == 1 { 0 } else { ring });
            t.set_u16(count, if case == 2 { 0xff } else { 4 });
        }
        t.set_u32(ring, ring);
        t.set_u32(ring + 4, ring);
        t.set_u32(w(&t, 0x5b), if case == 0 { 0 } else { n });
        if case == 2 {
            // n's children c1 -> c2 -> c3 (c3's +0xc 0 ends them)
            t.set_u32(n, prev);
            t.set_u32(n + 4, next);
            t.set_u32(n + 8, c1);
            for (c, nx) in [(c1, c2), (c2, c3), (c3, 0)] {
                t.set_u32(c, 0);
                t.set_u32(c + 4, 0);
                t.set_u32(c + 8, c);
                t.set_u32(c + 0xc, nx);
            }
        } else {
            t.set_u32(n, 0);
            t.set_u32(n + 4, 0);
            t.set_u32(n + 8, n);
            t.set_u32(n + 0xc, 0);
        }
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0480::clear(g, img, fr.at))
        });
    }
}

/// 0x100043a0: an object still running, the script not done, both done
/// (the lights cleared, no node).
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x43a0);
    recs(&mut t, &[(590, 2), (589, 2)]);
    let run = t.img.rel32_target(at + 0x5);
    let ro = t.img.rel32_target(at + 0xc);
    let cl = t.img.rel32_target(at + 0x1a);
    t.set_u32(t.img.word(cl + 0x5b), 0);
    let (node, task) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
    for (mode, running) in [(0u8, true), (2, false), (0, false)] {
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u16(t.img.word(run + 0x59), 0xffff);
        magic_hi::objects(&mut t, ro, running);
        t.write(node, &[0x4e; 0x20]);
        t.set_u32(node + 8, task);
        t.write(task, &[0x7a; 0x20]);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}

/// 0x10004530: the records set (the local's never-written word the same
/// stack junk on both sides).
#[test]
fn records_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4530);
    let a = t.img.rel32_target(at + 0x29);
    let base = t.img.word(a + 0x13);
    t.write(base - 0x40, &[0x5d; 0x200]);
    let b = t.img.rel32_target(at + 0xc8);
    let base = t.img.word(b + 0x13);
    t.write(base - 0x40, &[0x6e; 0x200]);
    magic_h6::scrub(&mut t);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
