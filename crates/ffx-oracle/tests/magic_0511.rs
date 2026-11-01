//! `magic_0511.dll` against its Rust port (`ffx_magic::m0511`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_hi::{list, pool};
use magic_owna::twin;

const N: u16 = 511;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002be0: the count running and run out.
#[test]
fn fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2be0);
    let fx = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + 0x2), fx);
    for n in [5u32, 1, 0, 0xffff_fff0] {
        t.write(fx + 0xe90, &[0x1d; 0x20]);
        t.set_u32(fx + 0x225c, n);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002c50 and 0x10002da0: an empty list and one with a node.
#[test]
fn starts() {
    let Some(mut t) = twin(N) else { return };
    for rva in [0x2c50u32, 0x2da0] {
        let at = t.at(rva);
        let fx = SCRATCH + 0x8000;
        t.set_u32(t.img.word(at + 0x4), fx);
        let take = t.img.rel32_target(at + 0x67);
        for full in [false, true] {
            pool(&mut t, take, SCRATCH + 0x2000, 3);
            let (head, tail) = (t.img.word(at + 0x6d), t.img.word(at + 0x83));
            list(&mut t, head, tail, SCRATCH + 0x3000, full);
            // the whole effect block dirty (the parts' words it clears show)
            t.write(fx, &[0x5a; 0x2100]);
            magic_h6::scrub(&mut t);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// 0x100042f0: an empty list and one with a node.
#[test]
fn sprite_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x42f0);
    let s = SCRATCH + 0x100;
    t.set_u32(t.img.word(at + 0x11), s);
    t.set_u32(s + 0x28, 0x00a0_0028);
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x18), fx);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x21), b);
    // the byte after the index non-zero (an index read as u16 shows)
    t.set_u32(b, 0x3c03);
    magic_owna::rec(&mut t, 219, 2, |_, a| 0x0219_0000 ^ a[0]);
    magic_owna::rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, 0x1111_1111 * (i + 1));
        }
        0x268
    });
    let take = t.img.rel32_target(at + 0xa5);
    for full in [false, true] {
        pool(&mut t, take, SCRATCH + 0x2000, 2);
        let (head, tail) = (t.img.word(at + 0xbe), t.img.word(at + 0xab));
        list(&mut t, head, tail, SCRATCH + 0x3000, full);
        t.write(fx + 0x2000, &[0x5a; 0x280]);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// The object start's (`s4_misc::start_object_anim` at `so`) part table,
/// lock and pool.
fn object_start(t: &mut magic::Twin, so: u32) {
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    magic_owna::rec(t, 941, 1, move |_, _| table);
    magic_owna::rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
    let sp = t.img.rel32_target(so + 0x84);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
}

/// 0x100020d0: the node there and not.
#[test]
fn models_end() {
    let Some(mut t) = twin(511) else { return };
    let at = t.at(0x20d0);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x1), b);
    t.set_u32(b, 0x0302_0100);
    t.set_u32(b + 4, 0x0706_0504);
    let table = t.img.word(at + 0xc);
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0384_0000 + i);
    }
    magic_owna::recs(
        &mut t,
        &[
            (384, 3),
            (25, 3),
            (24, 3),
            (271, 3),
            (772, 2),
            (779, 4),
            (941, 1),
        ],
    );
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0xea), fx);
    let rp = t.img.rel32_target(at + 0xfa);
    let parts = SCRATCH + 0x200;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0941_0001);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    t.set_u32(t.img.word(rp + 0x76), 0);
    let r3 = t.img.rel32_target(at + 0xf5);
    let lock = SCRATCH + 0x300;
    t.set_u32(t.img.word(rp + 0x43), lock);
    for op in [0x21, 0x3d] {
        t.set_u32(t.img.word(r3 + op), lock);
    }
    let give = t.img.rel32_target(at + 0xda);
    let node = SCRATCH + 0x5000;
    for has in [false, true] {
        magic_hi::lone_node(&mut t, give, node);
        t.set_u32(t.img.word(at + 0xce), if has { node } else { 0 });
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002b40, 0x10002ef0, 0x10003e30 and 0x10003150 (the old object
/// listed and not; the angle wrapped down, up and not).
#[test]
fn object_starts() {
    let Some(mut t) = twin(511) else { return };
    let fx = SCRATCH + 0x8000;
    for rva in [0x2b40u32, 0x2ef0, 0x3e30, 0x3150] {
        let at = t.at(rva);
        let so = t.img.rel32_target(
            at + if rva == 0x3150 {
                0x5e
            } else if rva == 0x3e30 {
                0x1e
            } else {
                0x20
            },
        );
        object_start(&mut t, so);
        let g = if rva == 0x3e30 {
            0xb
        } else if rva == 0x3150 {
            0x4
        } else {
            0xc
        };
        t.set_u32(t.img.word(at + g), fx);
        let cases: &[(f32, bool)] = if rva == 0x3150 {
            &[(0.5, false), (100.0, true), (-100.0, false), (0.0, true)]
        } else {
            &[(0.5, false)]
        };
        for &(a, listed) in cases {
            t.write(fx, &[0x3c; 0x2300]);
            for i in 0..4 {
                t.set_u32(fx + 0x1a60 + 4 * i, magic_hi::f(1.5 + i as f32));
            }
            t.set_u32(fx + 0x2258, magic_hi::f(a));
            if rva == 0x3150 {
                let list = t.img.word(at + 0x9);
                let n = SCRATCH + 0x6000;
                t.write(n, &[0; 0x10]);
                t.set_u32(n + 8, fx + 0xbe0);
                t.set_u32(list, if listed { n } else { 0 });
                t.set_u32(list + 4, if listed { n } else { 0 });
                t.set_u32(t.img.word(at + 0x1f), SCRATCH + 0x6100);
                magic_owna::recs(&mut t, &[(713, 1)]);
            }
            magic_h6::scrub(&mut t);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}
