//! `magic_0589.dll` against its Rust port (`ffx_magic::m0589`). Needs
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

const N: u16 = 589;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100028b0: the list (shared with the fade task) empty and not.
#[test]
fn flash() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x28b0);
    let r = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x13), r);
    let fade = t.img.rel32_target(at + 0x25);
    let take = t.img.rel32_target(at + 0x8a);
    for empty in [true, false] {
        pool(&mut t, take, SCRATCH + 0x1000, 3);
        t.write(r + 0x1a00, &[0x3b; 0x300]);
        let (fh, ft) = (t.img.word(fade + 0x7b), t.img.word(fade + 0x93));
        list(&mut t, fh, ft, SCRATCH + 0x3000, !empty);
        magic_h6::scrub(&mut t);
        // the caller's words above the return address non-zero (the callee's
        // stack word is the 0 the frame pushes, not those)
        let above = [0x00ca_0001, 0x00ca_0002, 0x00ca_0003, 0x00ca_0004];
        t.check_entry(at, Call::default(), &above, Eax::Checked);
    }
}

/// 0x10002980: an empty list and one with a node.
#[test]
fn two_parts() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2980);
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x4), fx);
    let take = t.img.rel32_target(at + 0x67);
    for full in [false, true] {
        pool(&mut t, take, SCRATCH + 0x2000, 3);
        let (head, tail) = (t.img.word(at + 0x6d), t.img.word(at + 0x83));
        list(&mut t, head, tail, SCRATCH + 0x3000, full);
        t.write(fx + 0x1a00, &[0x5a; 0x200]);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100047d0: the colour blend with the countdowns above, at and below
/// 0.
#[test]
fn blend() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x47d0);
    let s = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x5), s);
    for counts in [[5u32, 1, 0], [0, 0xffff_fffe, 30], [12, 7, 3]] {
        t.write(s, &[0x2b; 0x2300]);
        for i in 0..0x18u32 {
            t.set_u32(s + 0x1a10 + 4 * i, (0.125 + 0.5 * i as f32).to_bits());
        }
        for (j, n) in (0u32..).zip(counts) {
            t.set_u32(s + 0x2208 + 4 * j, n);
        }
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001e90 (`m0511::models_end`, family P072): the node there and not.
#[test]
fn models_end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1e90);
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

/// 0x10002ad0 (`m0511::object_d10`, family P086).
#[test]
fn object_d10() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2ad0);
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0xc), fx);
    let so = t.img.rel32_target(at + 0x20);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    magic_owna::rec(&mut t, 941, 1, move |_, _| table);
    magic_owna::rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
    let sp = t.img.rel32_target(so + 0x84);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    t.write(fx, &[0x3c; 0x2300]);
    for i in 0..4 {
        t.set_u32(fx + 0x1a00 + 4 * i, magic_hi::f(1.5 + i as f32));
    }
    t.set_u32(fx + 0x21f8, 0x21f8_0001);
    magic_h6::scrub(&mut t);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
