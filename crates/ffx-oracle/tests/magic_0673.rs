//! `magic_0673.dll` against its Rust port (`ffx_magic::m0673`). Needs
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

const N: u16 = 673;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001dd0: entries 0 and 2, the list empty and not.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1dd0);
    let r = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x23), r);
    let take = t.img.rel32_target(at + 0x68);
    let ctx = SCRATCH + 0x100;
    for (k, full) in [(0u32, false), (2, true)] {
        pool(&mut t, take, SCRATCH + 0x1000, 3);
        let (head, tail) = (t.img.word(at + 0x85), t.img.word(at + 0x6e));
        list(&mut t, head, tail, SCRATCH + 0x3000, full);
        t.write(r + 0x59f8, &[0x3b; 0x100]);
        t.write(ctx, &[0; 0x40]);
        t.set_u32(ctx + 0x38, k);
        // the template's last word non-zero (the record's `+0x24` takes it)
        let e = t.img.word(at + 0x1d) + 32 * k;
        t.set_u32(e + 0x1c, 0x00e1_c000 + k);
        t.check_entry(at, Call::default(), &[ctx], Eax::Checked);
    }
}

/// The object start (`s4_misc::start_object_anim` at `so`): part
/// table, lock and pool.
fn anim_start(t: &mut magic::Twin, so: u32) {
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

/// 0x10001520 (rows 2 and 9), 0x100015a0, 0x10001b00, 0x10001b60.
#[test]
fn rows() {
    let Some(mut t) = twin(N) else { return };
    let sr = t.at(0x1520);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(sr + 0x18), fx);
    let so = t.img.rel32_target(sr + 0x31);
    anim_start(&mut t, so);
    t.write(fx, &vec![0x3c; 0x130 * 0x1d]);
    for i in [2u32, 9] {
        magic_h6::scrub(&mut t);
        t.check(sr, Call::this(i), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::m0673::start_row(g, img, fr))
        });
    }
    let rec = SCRATCH + 0x200;
    t.set_u32(rec + 0x2c, 4);
    t.check_entry(t.at(0x15a0), Call::default(), &[rec], Eax::Checked);
    t.check_entry(t.at(0x1b00), Call::default(), &[], Eax::Checked);
    t.check_entry(t.at(0x1b60), Call::default(), &[], Eax::Checked);
}

/// 0x10001300: the second script running (0) and done (the objects
/// freed, the parts registered, the node given back).
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1300);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x6), fx);
    let (s1, s2) = (t.img.word(at + 0x2d), t.img.word(at + 0x3c));
    let rp = t.img.rel32_target(at + 0x59);
    let parts = SCRATCH + 0x200;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0941_0001);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    t.set_u32(t.img.word(rp + 0x76), 0);
    t.set_u32(t.img.word(rp + 0x43), SCRATCH + 0x300);
    magic_owna::recs(&mut t, &[(941, 1), (772, 2), (779, 4)]);
    let give = t.img.rel32_target(at + 0x63);
    let node = SCRATCH + 0x5000;
    t.set_u32(t.img.word(at + 0x25), 0x0025_1234);
    for (first, second) in [(0u8, 4u8), (4, 0)] {
        let block: Vec<u8> = (0..0x30u8).map(|i| 0x51 ^ i.wrapping_mul(7)).collect();
        t.write(t.img.word(at + 0xf), &block);
        t.set_u8(s1 + 0x56, first);
        t.set_u8(s2 + 0x56, second);
        t.set_u32(fx + 0x5e84, 7);
        magic_hi::lone_node(&mut t, give, node);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}

/// 0x10001cd0: a record's pair (slot 314's ST0 two values).
#[test]
fn record_pair() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1cd0);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x16), fx);
    let so = t.img.rel32_target(at + 0x5a);
    anim_start(&mut t, so);
    let calls = SCRATCH + 0x50;
    magic_owna::rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, move |m, _| {
        let k = m.r32(calls);
        m.w32(calls, k + 1);
        0.375 + f64::from(k) * 1.25
    });
    let rec = SCRATCH + 0x200;
    for i in [0u32, 2] {
        t.set_u32(rec + 0x34, i);
        t.write(fx, &vec![0x3c; 0x260 * 0x1c]);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[rec], Eax::Checked);
    }
}
