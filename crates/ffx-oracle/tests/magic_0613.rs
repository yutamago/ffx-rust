//! `magic_0613.dll` against its Rust port (`ffx_magic::m0613`). Needs
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

use ffx_magic::{Ret, m0613};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{recs, twin};

const N: u16 = 613;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10009500: an object with bit 0 of `+0x90` clear and set.
#[test]
fn task_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x9500);
    recs(&mut t, &[(30, 1), (36, 3), (228, 2)]);
    let tail = SCRATCH + 0x3800;
    let obj = SCRATCH + 0x1000;
    for bits in [0u16, 0x8001] {
        t.write(tail, &[0x1b; 0x20]);
        t.set_u32(t.img.word(at + 0x4), tail);
        t.write(obj, &[0x4e; 0x100]);
        t.set_u16(obj + 0x90, bits);
        let c = Call {
            edx: obj,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0613::task_start(g, img, fr))
        });
    }
}

/// 0x100074e0: no parts, a negative count, one and three parts.
#[test]
fn centre() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x74e0);
    let base = SCRATCH + 0x400;
    for i in 0..4 {
        t.set_u32(base + 4 * i, (10.5 - 3.25 * i as f32).to_bits());
    }
    magic_owna::rec(&mut t, 325, 2, move |m, a| {
        for i in 0..4 {
            let v = m.r32(base + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x0032_5000
    });
    magic_owna::rec(&mut t, 268, 3, |m, a| {
        for i in 0..3 {
            m.w32(
                a[2] + 4 * i,
                ((a[0] & 0xff) as f32 * 1.75 - i as f32).to_bits(),
            );
        }
        0x0268_0000 + a[0]
    });
    let (rec, obj, out) = (SCRATCH + 0x100, SCRATCH + 0x1_0000, SCRATCH + 0x200);
    t.set_u32(rec, obj);
    t.set_u32(rec + 4, out);
    t.set_u32(rec + 8, SCRATCH + 0x500);
    for k in 0..3u32 {
        t.set_u32(obj + 0x110 + 0x274 * k, 0x00a0_0003 + 5 * k);
    }
    for n in [0i32, -2, 1, 3] {
        t.set_u32(obj + 0x162c, n.cast_unsigned());
        t.write(out, &[0x55; 0x10]);
        t.check(at, Call::this(rec), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0613::centre(g, img, fr))
        });
    }
}
