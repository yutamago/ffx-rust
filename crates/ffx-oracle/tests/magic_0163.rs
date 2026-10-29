//! `magic_0163.dll` against its Rust port: `DllMain` against every bound
//! port, the particle step and the target angle. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

use std::cell::Cell;
use std::rc::Rc;

use ffx_magic::{Ret, m0163};
use magic::{Call, Eax, SCRATCH};

const N: u16 = 163;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn particle_step() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x1d10);
    let p = SCRATCH + 0x100;
    for (n, v) in [
        (0u32, 1.5f32),
        (0xffff_ffff, -3.0),
        (15, 0.1),
        (16, 2.0),
        (17, -0.3),
        (18, 4.0),
        (0x7fff_ffff, 1.0),
    ] {
        t.set_u32(p + 0x118, n);
        t.set_u32(p + 0xd4, v.to_bits());
        t.set_u32(p + 0x104, 0x5555);
        t.check_entry(f, Call::default(), &[p], Eax::Checked);
    }
}

#[test]
fn target_angle() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x1e00);
    let own = t.img.word(f + 0x0a);
    let map = t.img.word(f + 0x11);
    let chrs = t.img.word(f + 0x1c);
    t.stub_float(333, |_, a| f64::from(f32::from_bits(a[0])) - 1e-9);
    t.set_u32(map, SCRATCH + 0x600);
    for (i, k) in (0u32..8).zip((0u8..8).rev()) {
        t.set_u8(SCRATCH + 0x600 + i, k);
    }
    let mut rots = vec![
        0.0f32, 1.0, -1.0, 3.5, -3.5, 7.0, -7.0, 100.0, 1.0e-3, -1.0e-3,
    ];
    // a rotation whose angle lands exactly on the upper wrap limit (HI,
    // pi). The lower one (-pi) can't be hit: A - rot for rot near 7.85 is
    // a multiple of 2^-21 and -pi is not, so `>` and `>=` agree there.
    let k = |op: u32| f32::from_bits(t.u32(t.img.word(f + op)));
    let (a, hi) = (k(0x35), k(0x44));
    let guess = (a - hi).to_bits();
    let hit = (0..64u32)
        .flat_map(|d| [guess + d, guess - d])
        .map(f32::from_bits)
        .find(|&r| (a - r).to_bits() == hi.to_bits())
        .expect("a rotation on the limit");
    rots.push(hit);
    for (k, &r) in (0u32..).zip(&rots) {
        for index in [0xffff_ffffu32, k % 8] {
            t.set_u32(own, r.to_bits());
            for (j, d) in (0u32..8).zip([0.0f32, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5]) {
                t.set_u32(chrs + 4 * j, (r + d).to_bits());
            }
            let got = Rc::new(Cell::new(0u32));
            let g2 = got.clone();
            t.check(
                f,
                Call {
                    eax: 0,
                    ecx: index,
                    edx: 0,
                },
                &[],
                Eax::Junk,
                move |g, img, fr| {
                    g2.set(m0163::target_angle(g, img, fr.at, fr.ecx).to_bits());
                    Ret::default()
                },
            );
            assert_eq!(got.get(), t.xmm0_f32(), "index {index:#x} rot {r}");
        }
    }
}
