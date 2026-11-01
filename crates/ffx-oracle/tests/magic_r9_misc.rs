//! Round 9's shared own-row functions (`ffx_magic::r9_misc`) against each
//! copy's original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, Xmm, r9_misc};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

/// `magic_0447` 0x10003320 and `magic_0100` 0x100021a0 (internal: `edx`
/// = the object, `xmm3` the scale, the stack `(index)`): appended to an
/// empty list, to a list with a tail, and with the pool empty.
#[test]
fn start_object_state() {
    for (n, rva) in [(447u16, 0x3320u32), (100, 0x21a0)] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(at + 0x5e), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        for i in 0..8 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        t.stub(714, |m, a| {
            for (i, w) in (0u32..).zip(&a[..5]) {
                m.w32(SCRATCH + 0x910 + 4 * i, *w);
            }
            0x714
        });
        let pa = t.img.rel32_target(at + 0x9c);
        let free = t.img.word(pa + 0x04);
        let nodes = t.img.word(pa + 0x27) - 8;
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        let tail = t.img.word(at + 0xa7);
        let obj = SCRATCH + 0x1000;
        for (k, (count, scale, tl)) in [
            (3u16, 0.5f32, 0u32),
            (2, -2.0, SCRATCH + 0x600),
            (0, 1.0, 0),
            (1, f32::NAN, SCRATCH + 0x700),
        ]
        .into_iter()
        .enumerate()
        {
            t.write(obj, &[0x5a; 0x130]);
            t.set_u16(free, count);
            t.set_u32(tail, tl);
            let c = Call {
                edx: obj,
                ecx: 0x0bad_0ecc,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(scale)]);
            let k = u32::try_from(k).unwrap_or(0);
            t.check(
                at,
                c,
                &[k * 2 + 1, 0x0a00 + k],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(r9_misc::start_object_state(
                        g,
                        img,
                        fr.at,
                        fr.edx,
                        fr.xmm_f32(3),
                        fr.arg(g, 0),
                    ))
                },
            );
        }
    }
}

/// The object start with the argument reset (`magic_0182` 0x10001b70 and
/// its copies; internal: `edx` = the object, `xmm3` the scale): the
/// object list empty and not, the pool empty.
#[test]
fn start_object_reset_arg() {
    for (n, rva) in [
        (182u16, 0x1b70u32),
        (197, 0x1ad0),
        (252, 0x2000),
        (266, 0x3a20),
        (311, 0x2c60),
        (365, 0x1680),
    ] {
        reset_cases(n, rva, Shape::Arg);
    }
}

/// The object start with the bare reset (`magic_0245` 0x10002500 and
/// its copies).
#[test]
fn start_object_reset_bare() {
    for (n, rva) in [
        (245u16, 0x2500u32),
        (298, 0x2df0),
        (308, 0x1fd0),
        (435, 0x28a0),
    ] {
        reset_cases(n, rva, Shape::Bare);
    }
}

/// The object start in a frame with the `edx` reset (`magic_0108`
/// 0x10001c00 and its copies; the stack `(index, owner)`).
#[test]
fn start_object_reset_edx() {
    for (n, rva) in [(108u16, 0x1c00u32), (221, 0x1c00), (419, 0x38d0)] {
        reset_cases(n, rva, Shape::Edx);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Arg,
    Bare,
    Edx,
}

/// [`start_object_reset_arg`]'s cases for each shape.
fn reset_cases(dll: u16, rva: u32, shape: Shape) {
    let shift = if shape == Shape::Edx { 3 } else { 0 };
    {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        let start = t.img.rel32_target(f + 0x7d + shift);
        t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f8);
        t.set_u32(op(&t, f, 0x1 + shift), SCRATCH + 0x2f0);
        t.set_u32(op(&t, f, 0x5b + shift), SCRATCH + 0x280);
        for (o, v) in [(0x270u32, 0x0d00_0270u32), (0x290, 0x0d00_0290)] {
            t.set_u32(SCRATCH + o, v);
        }
        let pa = t.img.rel32_target(f + [0x95, 0x8f, 0x9b][shape as usize]);
        let free = op(&t, pa, 0x04);
        let nodes = op(&t, pa, 0x27) - 8;
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        let tail = op(&t, f, [0xa0, 0x9a, 0xa6][shape as usize]);
        let obj = SCRATCH + 0x1000;
        for (k, (count, listed)) in [(5u16, false), (2, true), (0, true)]
            .into_iter()
            .enumerate()
        {
            let k = u32::try_from(k).unwrap_or(0);
            t.write(obj, &[0x5a; 0x140]);
            t.set_u32(SCRATCH + 0x2f0, 0x02f0_0000 + k);
            t.set_u16(free, count);
            t.write(SCRATCH + 0x1400, &[0x14; 0x10]);
            t.set_u32(tail, if listed { SCRATCH + 0x1400 } else { 0 });
            let call = Call {
                edx: obj,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            }
            .with_xmm(&[
                Xmm::f32(9.0),
                Xmm::f32(8.0),
                Xmm::f32(7.0),
                Xmm::f32(1.25 + 0.5 * f32::from(u8::try_from(k).unwrap_or(0))),
            ]);
            t.check(
                f,
                call,
                &[0x0123 + k, 0x0bb0_0000 + k],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(match shape {
                        Shape::Arg => r9_misc::start_object_reset_arg(g, img, fr),
                        Shape::Bare => r9_misc::start_object_reset_bare(g, img, fr),
                        Shape::Edx => r9_misc::start_object_reset_edx(g, img, fr),
                    })
                },
            );
        }
    }
}
