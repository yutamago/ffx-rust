//! `magic_0562.dll` against its Rust port (`ffx_magic::m0562`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 562;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001a00: the objects running; the motion not done; the end (the
/// lights off, the grid empty).
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a00);
    for s in [210u32, 13] {
        rec(&mut t, s, 2, move |m, a| {
            for i in 0..16 {
                m.w32(a[1] + 4 * i, (0x3f00_0000 + s) ^ i);
            }
            s
        });
    }
    rec(&mut t, 544, 2, |m, a| {
        for j in 0..4 {
            m.w32(a[1] + 4 * j, (0x3f80_0000 + a[0]) ^ j);
        }
        0x544
    });
    recs(
        &mut t,
        &[
            (553, 5),
            (6, 2),
            (547, 3),
            (357, 6),
            (356, 9),
            (238, 2),
            (856, 0),
            (404, 1),
        ],
    );
    t.stub_st0(404, |_, _| 3.25);
    t.set_u32(t.img.word(at + 0x3c), 0x003c_0001);
    t.set_u32(t.img.word(at + 0x4d), 0x004d_0001);
    t.set_u32(t.img.word(at + 0x95), 0x0095_0001);
    t.set_u32(t.img.word(at + 0xb6), 0x00b6_0001);
    t.set_u32(t.img.word(at + 0xe2), 0x00e2_0001);
    let flag = SCRATCH + 0x40;
    t.set_u32(t.img.word(at + 0x73), flag);
    let pl = t.img.rel32_target(at + 0x21);
    // the lights on, all fading above the cap (they say done)
    t.set_u32(t.img.word(pl + 0x12), 1);
    let lights = t.img.word(pl + 0x3a);
    let ms = t.img.rel32_target(at + 0x1a);
    let ro = t.img.rel32_target(at + 0x65);
    let dg = t.img.rel32_target(at + 0x103);
    t.set_u32(t.img.word(dg + 0x08), 0);
    let (node, task) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
    for (running, done) in [(true, true), (false, false), (false, true)] {
        for i in 0..6 {
            t.set_u32(lights + 0x34 * i + 8, 1);
            t.set_u32(lights + 0x34 * i - 8, 1000.0f32.to_bits());
        }
        magic_hi::objects(&mut t, ro, running);
        magic_hi::motion(&mut t, ms, done);
        t.set_u32(flag, 7);
        t.write(node, &[0x4e; 0x20]);
        t.set_u32(node + 8, task);
        t.write(task, &[0x7a; 0x20]);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
    // the end ran: slot 6 was called
    assert_ne!(t.u32(magic_owna::row(6)), 0);
}
