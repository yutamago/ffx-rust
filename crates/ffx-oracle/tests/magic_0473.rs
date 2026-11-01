//! `magic_0473.dll` against its Rust port (`ffx_magic::m0473`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, Xmm, m0473};
use magic::{Call, Eax, SCRATCH};
use magic_hi::{list, pool};
use magic_owna::twin;

const N: u16 = 473;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10003c20: scales above 0, at 0, below, NaN; an empty list and one
/// with a node.
#[test]
fn scale_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3c20);
    let take = t.img.rel32_target(at + 0x7b);
    let obj = SCRATCH + 0x1000;
    let node0 = SCRATCH + 0x3000;
    for (k, (a, b, full)) in [
        (1.5f32, 2.0f32, false),
        (0.0, -3.0, true),
        (-0.0, 0.25, false),
        (f32::NAN, -1.0e-3, true),
        (7.0, f32::INFINITY, true),
    ]
    .into_iter()
    .enumerate()
    {
        pool(
            &mut t,
            take,
            SCRATCH + 0x2000,
            1 + u32::try_from(k).unwrap() % 3,
        );
        let w = |op: u32| t.img.word(at + op);
        let (head, tail) = (w(0x99), w(0x81));
        list(&mut t, head, tail, node0, full);
        t.write(obj, &[0x55; 0x40]);
        let c = Call::this(obj).with_xmm(&[Xmm::f32(9.0), Xmm::f32(a), Xmm::f32(b)]);
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0473::scale_start(
                g,
                img,
                fr.at,
                fr.ecx,
                fr.xmm_f32(1),
                fr.xmm_f32(2),
            ))
        });
    }
}

/// 0x10001740: the script or the motion not done; both done, with and
/// without the model's slot 213 (no lights node; the node given back).
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1740);
    let fx = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + 0x7), fx);
    magic_owna::recs(&mut t, &[(213, 2), (590, 2), (589, 2)]);
    t.set_u32(t.img.word(at + 0x3f), 0x003f_0001);
    let run = t.img.rel32_target(at + 0xc);
    let ms = t.img.rel32_target(at + 0x13);
    let give = t.img.rel32_target(at + 0x6a);
    let node = SCRATCH + 0x5000;
    for (mode, done, half) in [(2u8, true, 0u16), (0, false, 0), (0, true, 0), (0, true, 3)] {
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u16(t.img.word(run + 0x59), 0xffff);
        magic_hi::motion(&mut t, ms, done);
        t.set_u16(t.img.word(at + 0x29), half);
        t.write(fx + 0x380, &[0x2c; 0x240]);
        t.set_u32(fx + 0x380 + 0x228, 0);
        t.set_u32(fx + 0x610, 0x0610_0000);
        magic_hi::lone_node(&mut t, give, node);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}
