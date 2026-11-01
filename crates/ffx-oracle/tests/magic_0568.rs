//! `magic_0568.dll` against its Rust port (`ffx_magic::m0568`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 568;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001c30: the sway at times inside the table, at its last key and
/// past it.
#[test]
fn sway() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1c30);
    let s = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x3), s);
    let tab = t.img.word(at + 0x17);
    for i in 0..6u32 {
        t.set_u32(tab + 4 * i, (0.5 + 1.25 * i as f32).to_bits());
    }
    for time in [0.0f32, 1.3, 3.75, 4.0, 4.5] {
        t.write(s, &[0x2b; 0x1800]);
        t.set_u32(s + 0x16c4, time.to_bits());
        t.set_u32(s + 0x143c, 2.5f32.to_bits());
        magic_h6::scrub(&mut t);
        t.check(at, Call::default(), &[], Eax::Junk, |g, img, fr| {
            ffx_magic::m0568::sway(g, img, fr)
        });
    }
}

/// The object start's (`s4_small::start_host_object` at `so`) part
/// table and pool.
fn host_start(t: &mut magic::Twin, so: u32, host: u32) {
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    magic_owna::rec(t, 941, 1, move |_, _| table);
    magic_owna::rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x4), SCRATCH + 0x2f0);
    let sp = t.img.rel32_target(so + 0x81);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    let d = SCRATCH + 0x2e00;
    t.set_u32(host + 0x2c, d);
    t.set_u32(d + 0x10, 0x00d0_0010);
}

/// 0x10001cc0 and 0x10002840: the host objects' starts.
#[test]
fn object_starts() {
    let Some(mut t) = twin(N) else { return };
    let fx = SCRATCH + 0x1_0000;
    for (rva, g_op, calls) in [
        (0x1cc0u32, 0x10u32, &[(0x36u32, 0x2cu32), (0xac, 0x93)][..]),
        (0x2840, 0xb, &[(0x25, 0x20)][..]),
    ] {
        let at = t.at(rva);
        t.set_u32(t.img.word(at + g_op), fx);
        t.write(fx, &[0x3c; 0x1700]);
        for &(call, host_op) in calls {
            let so = t.img.rel32_target(at + call);
            let host = t.img.word(at + host_op);
            host_start(&mut t, so, host);
        }
        for i in 0..4 {
            t.set_u32(fx + 0x1428 + 4 * i, magic_hi::f(1.5 + i as f32));
            t.set_u32(fx + 0x1448 + 4 * i, magic_hi::f(-2.5 + i as f32));
        }
        t.set_u32(fx + 0x16bc, magic_hi::f(0.75));
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001440: the first script running (the camera rigs' step) and done
/// (the node there or not; the own node given back).
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1440);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x7), fx);
    let (s1, s2) = (t.img.word(at + 0x2e), t.img.word(at + 0x3d));
    let ms = t.img.rel32_target(at + 0x7d);
    let rp = t.img.rel32_target(at + 0xad);
    let parts = SCRATCH + 0x200;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0941_0001);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    t.set_u32(t.img.word(rp + 0x76), 0);
    t.set_u32(t.img.word(rp + 0x43), SCRATCH + 0x300);
    magic_owna::recs(&mut t, &[(941, 1), (772, 2), (779, 4)]);
    let give = t.img.rel32_target(at + 0xb8);
    let (own, held) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
    t.set_u32(t.img.word(at + 0x26), 0x0026_1234);
    for (first, second, has) in [(4u8, 0u8, false), (0, 0, true), (0, 4, false)] {
        t.write(t.img.word(at + 0x10), &[0x5c; 0x30]);
        for (s, state) in [(s1, first), (s2, second)] {
            t.set_u8(s + 0x56, state);
        }
        magic_hi::motion(&mut t, ms, false);
        t.write(fx + 0x1400, &[0x2d; 0x300]);
        for i in 0..4 {
            t.set_u32(fx + 0x1428 + 4 * i, 0x1428_0000 + i);
        }
        magic_hi::lone_node(&mut t, give, own);
        t.write(held, &[0; 0x1c]);
        t.set_u32(held + 8, held);
        t.set_u32(t.img.word(at + 0x8b), if has { held } else { 0 });
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[own], Eax::Checked);
    }
}
