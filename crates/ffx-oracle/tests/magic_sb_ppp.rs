//! The node family's particle nodes (`ffx_magic::sb_ppp`) against
//! `magic_0003.dll`. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;

use ffx_magic::{Ret, sb_ppp};
use magic::{Call, Eax, SCRATCH, Twin};

const GROUP: u32 = SCRATCH + 0x1000;
const OWNER: u32 = SCRATCH + 0x2000;
const PARENT: u32 = SCRATCH + 0x2400;
const MODEL: u32 = SCRATCH + 0x5000;
const LOG: u32 = SCRATCH + 0x8000;
/// The DLL's pointer globals' targets.
const PTRS: u32 = SCRATCH + 0x9000;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(3)?;
    t.init();
    Some(t)
}

/// Each slot logs its first `n` words (and the frame vector for the
/// pointer arguments it gets) in its own row and answers `ret`.
fn stubs(t: &mut Twin, lock: u32) {
    for (k, (s, n, ret)) in [
        (325u32, 2usize, 0x325u32),
        (230, 2, 0x230),
        (233, 3, 0x233),
        (668, 3, 0x668),
        (231, 3, 0x231),
        (713, 1, 0x713),
        (160, 3, 0x160),
    ]
    .into_iter()
    .enumerate()
    {
        let row = LOG + 0x80 * u32::try_from(k).unwrap();
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0x7c).wrapping_add(1);
            m.w32(row + 0x7c, c);
            let at = row + 0x40 * (c & 1);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(at + 4 * i, *w);
                // what a pointer argument points at (the frame vector)
                if *w >= magic::STACK_TOP - magic::STACK_LEN && *w < magic::STACK_TOP {
                    for j in 0..4 {
                        m.w32(at + 0x10 + 4 * j, m.r32(*w + 4 * j));
                    }
                }
            }
            ret
        });
    }
    // pppRunPart sees the lock; says done when the node's +0x5c says so
    t.stub(712, move |m, a| {
        m.w32(LOG + 0x400, a[0]);
        m.w32(LOG + 0x404, a[1]);
        m.w32(LOG + 0x408, m.r32(lock));
        m.r32(a[0] - 4)
    });
}

#[allow(clippy::too_many_arguments)]
fn node(t: &mut Twin, flags: u32, delay: u32, e4: u16, e6: u16, done: u32, func: u32) {
    t.write(LOG, &[0; 0x480]);
    t.set_u32(GROUP, OWNER);
    t.set_u8(GROUP + 4, 0);
    t.set_u8(GROUP + 5, 3);
    t.set_u8(OWNER + 0x1a, 9);
    t.set_u32(OWNER + 0x10, OWNER);
    t.set_u32(OWNER + 0x14, PARENT);
    t.set_u32(OWNER + 4, PARENT);
    t.set_u32(OWNER + 8, PARENT);
    t.set_u32(PARENT + 0xc, OWNER);
    t.set_u32(PARENT + 0x10, OWNER);
    t.set_u16(PARENT + 0x18, 1);
    for i in 0..4 {
        t.set_u32(OWNER + 0x1c + 4 * i, (1.5f32 + i as f32).to_bits());
        t.set_u32(OWNER + 0x2c + 4 * i, (200.0f32 - 30.0 * i as f32).to_bits());
        t.set_u32(OWNER + 0x3c + 4 * i, (10.0f32 + 7.0 * i as f32).to_bits());
    }
    t.set_u32(OWNER + 0x4c, MODEL);
    t.set_u32(OWNER + 0x50, 0x5050);
    t.set_u32(OWNER + 0x5c, done);
    t.set_u32(OWNER + 0xd0, 0xdddd_dddd);
    t.set_u32(OWNER + 0xd4, 0xdddd_dddd);
    t.set_u32(OWNER + 0xe0, func);
    t.set_u16(OWNER + 0xe4, e4);
    t.set_u16(OWNER + 0xe6, e6);
    t.set_u32(OWNER + 0xe8, flags);
    t.set_u32(OWNER + 0xec, delay);
    t.set_u32(OWNER + 0xf4, 0xf4f4);
    t.set_u32(MODEL + 0x138, 0.5f32.to_bits());
}

#[test]
fn run_part() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x3660);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // the pointer globals: pause (+0x75), colour (+0x17e), matrix
    // (+0x1c2), the handed-over words (+0x2e0, +0x2ec), the lock (+0x2f6),
    // matrices A/B/C (+0x2bf, +0x2b3, +0x2d2)
    let pause = PTRS;
    let lock = PTRS + 0x10;
    for (op, v) in [
        (0x75u32, pause),
        (0x17e, PTRS + 0x100),
        (0x1c2, PTRS + 0x200),
        (0x2e0, PTRS + 0x20),
        (0x2ec, PTRS + 0x30),
        (0x2f6, lock),
        (0x2bf, PTRS + 0x300),
        (0x2b3, PTRS + 0x400),
        (0x2d2, PTRS + 0x500),
    ] {
        let g = w(&t, op);
        t.set_u32(g, v);
    }
    t.set_u32(w(&t, 0x2e5), 0x2e5_2e5);
    stubs(&mut t, lock);
    for (flags, delay) in [
        (0u32, 0u32),
        (1, 0),
        (2, 0),
        (3, 5),
        (0, 1),
        (1, 0x8000_0000),
        (0x11, 0),
        (5, 0),
    ] {
        for paused in [0u32, 1] {
            for (e4, e6) in [(0u16, 8u16), (3, 8), (0xfffe, 2), (5, 0)] {
                for done in [0u32, 1] {
                    for func in [0u32, 0x00ab_cdef] {
                        node(&mut t, flags, delay, e4, e6, done, func);
                        t.set_u32(pause, paused);
                        t.set_u32(lock, 0x42);
                        t.check_thunked(
                            f,
                            t.at(0x3a20),
                            false,
                            Call {
                                eax: 0x77,
                                ecx: GROUP,
                                edx: 0,
                                ..Call::default()
                            },
                            &[],
                            Eax::Checked,
                            |g, img, fr| Ret::eax(sb_ppp::run_part(g, img, fr.at, fr.esp, fr.ecx)),
                        );
                    }
                }
            }
        }
    }
}
