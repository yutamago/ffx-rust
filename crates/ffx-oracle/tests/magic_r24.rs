//! Round 24's own ports (`ffx_magic::r24`) against each original (the
//! copies bound to earlier ports are tested with those ports). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
mod magic_owna;

use ffx_magic::r24;
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{dump, filler, rec, twin};

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// Slots recorded with an answer from their first word.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (s << 20) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `r24::light_setup_0446` (`magic_0446` 0x100053b0, `magic_0543`
/// 0x10005310): three lights each (angles, point, `n`'s low byte); the
/// float slots answer in ST0 from their word, the matrix slots fill or
/// combine what they are handed, slot 567 logs the record and the
/// matrices, slot 553 the camera rows.
#[test]
fn light_setup_0446() {
    for (n, rva, g2) in [(446u16, 0x53b0u32, 0x344u32), (543, 0x5310, 0x385)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let h = SCRATCH + 0x3000;
        rec(&mut t, 219, 2, move |_, a| {
            h ^ (a[1] & 0x10) ^ (a[0] & 0x100)
        });
        for s in [277u32, 15, 16, 17] {
            rec(&mut t, s, 1, move |_, a| s ^ a[0]);
            t.stub_st0(s, move |_, a| {
                f64::from(a[0] & 0xff) * 0.125 + f64::from(s) * 0.5 + 0.1
            });
        }
        for s in [235u32, 723, 724] {
            filler(&mut t, s, 1, 16);
        }
        // the turns and products: each output word from both inputs
        for s in [330u32, 328, 329] {
            rec(&mut t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
                for i in 0..16 {
                    let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ s;
                    m.w32(a[0] + 4 * i, v);
                }
                s
            });
        }
        rec(&mut t, 231, 3, |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(5);
                m.w32(a[0] + 4 * i, v);
            }
            0x231
        });
        let src = SCRATCH + 0x3400;
        for i in 0..16 {
            t.set_u32(src + 4 * i, 0x0545_0000 + i);
        }
        rec(&mut t, 545, 0, move |_, _| src);
        rec(&mut t, 230, 2, |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v);
            }
            0x230
        });
        rec(&mut t, 825, 1, |_, a| 0x0825_0000 ^ a[0]);
        rec(&mut t, 843, 2, |m: &mut dyn StubMem, a: &[u32]| {
            dump(m, 843, a[1], 16);
            0x843
        });
        // the scaled camera's slots
        rec(&mut t, 544, 2, |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..4 {
                m.w32(a[1] + 4 * i, (0.5 * (a[0] + i + 1) as f32).to_bits());
            }
            0x544
        });
        rec(&mut t, 553, 5, |m: &mut dyn StubMem, a: &[u32]| {
            dump(m, 553, a[1], 16);
            0x553
        });
        rec(&mut t, 567, 5, |m: &mut dyn StubMem, a: &[u32]| {
            dump(m, 567, a[1], 16);
            dump(m, 568, a[1] + 0x40, 4);
            dump(m, 569, a[2], 16);
            dump(m, 570, a[3], 16);
            dump(m, 571, a[4], 16);
            for (k, o) in (572u32..).zip([0x20u32, 0x24, 0x28]) {
                let at = m.r32(a[1] + o);
                dump(m, k, at, 16);
            }
            0x567
        });
        recs(&mut t, &[(562, 0), (357, 3), (356, 3), (355, 2), (354, 2)]);
        let (angles, pos) = (SCRATCH + 0x3100, SCRATCH + 0x3200);
        for k in 0..3u32 {
            junk_ops(&mut t, f, &[0x53, g2]);
            t.set_u32(t.img.word(f + 0x53), 0x0021_9000 + 0x110 * k);
            for i in 0..4 {
                t.set_u32(angles + 4 * i, (0.5 + (i + k) as f32).to_bits());
                t.set_u32(pos + 4 * i, (-1.5 * (i + 1 + k) as f32).to_bits());
            }
            let call = Call {
                ecx: angles,
                edx: pos,
                ..Call::default()
            };
            let n = 0x1234_5600 + 0x3f * k + 7;
            t.check(f, call, &[n], Eax::Checked, |g, img, fr| {
                r24::light_setup_0446(g, img, fr)
            });
        }
        assert!(t.u32(magic_owna::row(567)) >= 3, "slot 567 ran");
    }
}

/// `r24::curve_step_0435` (`magic_0435` 0x10001a60, `magic_0055`
/// 0x10001ec0, `magic_0067` 0x10004520): the count reaching 0 (the give-back; `magic_0055`'s slot
/// 239), 10 (the fade) and neither; the yaw above, below and inside its
/// bounds (`K1` set far up, far down, kept); `magic_0055`'s flag set (no
/// yaw); the place by model or by handle.
#[test]
fn curve_step_0435() {
    // (DLL, RVA, the shift up to the `z` curve, the yaw keys, `K1`, the
    // place, the actor byte's pointer, the flag)
    for (n, rva, sh, kz, k1, place, actor_at, flag_at) in [
        (
            435u16, 0x1a60u32, 0u32, 0x95u32, 0xacu32, 0x108u32, 0xe0u32, None,
        ),
        (55, 0x1ec0, 0x1e, 0xc7, 0xd7, 0x12d, 0x10b, Some(0xb2u32)),
        (67, 0x4520, 0x1e, 0xb3, 0xca, 0x120, 0xfe, None),
    ] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let v55 = n != 435;
        recs(
            &mut t,
            &[
                (239, 3),
                (541, 3),
                (38, 3),
                (34, 2),
                (170, 2),
                (255, 2),
                (11, 1),
                (33, 4),
                (32, 3),
            ],
        );
        // the curves' keys (16 bytes apart, six from the immediates on)
        let (kx, ky) = (0x5c + sh, 0x83 + sh);
        for (j, o) in [kx, ky, kz].into_iter().enumerate() {
            let keys = w(&t, o);
            for i in 0..6u32 {
                let v = 0.75f32 * i as f32 - 1.0 + j as f32 * 0.5;
                t.set_u32(keys + 16 * i, v.to_bits());
            }
        }
        let k1 = w(&t, k1);
        let k1v = t.u32(k1);
        let place = t.img.rel32_target(f + place);
        let (obj, actor) = (SCRATCH + 0x5000, SCRATCH + 0x5400);
        let time = w(&t, 0x4e + sh);
        // (count, time, K1, flag, place mode)
        for (k, (count, tm, kk, flag, mode)) in [
            (1u32, 0.5f32, None, 0u32, 0u32),
            (11, 0.25, None, 0, 1),
            (5, 1.4, Some(1000.0f32), 0, 0),
            (5, 2.1, Some(-1000.0), 0, 1),
            (7, 0.9, None, 1, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u32;
            t.set_u32(w(&t, 0x11), count);
            t.set_u32(time, tm.to_bits());
            t.set_u32(k1, kk.map_or(k1v, f32::to_bits));
            if let Some(fl) = flag_at {
                t.set_u32(w(&t, fl), flag);
            }
            if v55 {
                t.set_u32(w(&t, 0x32), 0x0032_0000 + k);
            }
            junk_ops(&mut t, f, &[0x22 + if v55 { 0x1d } else { 0 }]);
            t.set_u32(w(&t, 0x79 + sh), (0.125 + k as f32).to_bits());
            // the actor byte and its table
            t.set_u8(actor, (3 + k) as u8);
            t.set_u32(w(&t, actor_at), actor);
            if !v55 {
                let tab = w(&t, 0xf0);
                for i in 0..0x20u32 {
                    t.set_u32(tab + 4 * i, 0x0ab0_0000 + i);
                }
            }
            // the place: by model index (mode 0) or by handle
            t.set_u32(t.img.word(place + 0x02), mode);
            t.set_u8(t.img.word(place + 0x09), (2 + k) as u8);
            // the object given back: no siblings, no children
            t.write(obj, &[0x11; 0x20]);
            t.set_u32(obj, 0);
            t.set_u32(obj + 4, 0);
            t.set_u32(obj + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
        assert!(t.u32(magic_owna::row(255)) >= 1, "placed");
    }
}

/// `r24::render_0352` (`magic_0352` 0x10005020): both counts out (the
/// give-back); the first or the second counting; slot 773 answering 0
/// (slot 883 answering 0 or not) or a record (slot 886 answering 0 or
/// not; the second count negative, small and large); the slots handing
/// out buffers, the locked ones logging the lock.
#[test]
fn render_0352() {
    let Some(mut t) = twin(352) else { return };
    let f = t.at(0x5020);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let ans = SCRATCH + 0x700;
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x80), lock);
    for (s, n, i) in [(773u32, 4usize, 0u32), (883, 2, 1), (886, 4, 2)] {
        rec(&mut t, s, n, move |m, a| {
            m.r32(ans + 4 * i) ^ (a[0] & 0x100)
        });
    }
    for s in [779u32, 870] {
        rec(&mut t, s, 2, move |m, a| m.r32(lock) ^ a[0] ^ (s << 16));
    }
    rec(&mut t, 772, 2, |_, a| 0x0077_2000 ^ a[0]);
    rec(&mut t, 776, 1, |_, a| 0x0077_6000 ^ a[0]);
    rec(&mut t, 881, 3, |_, _| 0x881);
    rec(&mut t, 882, 7, |m: &mut dyn StubMem, a: &[u32]| {
        for (k, &p) in (882u32..).zip(&a[3..7]) {
            dump(m, k, p, 4);
        }
        0x882
    });
    let bufs = SCRATCH + 0x1_0000;
    for (k, s) in (0u32..).zip([887u32, 898, 900, 902, 893, 890, 891]) {
        rec(&mut t, s, if s == 887 { 2 } else { 1 }, move |_, _| {
            bufs + 0x100 * k
        });
    }
    rec(&mut t, 230, 2, |m: &mut dyn StubMem, a: &[u32]| {
        dump(m, 230, a[1], 16);
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x0230_0000 + i);
        }
        0x230
    });
    recs(&mut t, &[(892, 2), (884, 3)]);
    let b = SCRATCH + 0x2000;
    t.set_u32(w(&t, 0x1c), b);
    let obj = SCRATCH + 0x5000;
    // (B[0xbdc], B[0xbe0], 773's, 883's, 886's answers)
    for (k, (c1, c2, r773, r883, r886)) in [
        (0i32, 0i32, 0u32, 0u32, 0u32),
        (3, 9, 0, 0, 0),
        (0, 5, 0, 0x88, 0),
        (2, 7, 0x00c0_0000, 0, 0),
        (2, 61, 0x00c0_0010, 0, 1),
        (1, -7, 0x00c0_0020, 0, 1),
        (0, 0x0100_0000, 0x00c0_0030, 0, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.write(b + 0xbd0, &[0x5a; 0x20]);
        t.set_u32(b + 0xbdc, c1.cast_unsigned());
        t.set_u32(b + 0xbe0, c2.cast_unsigned());
        t.set_u32(ans, r773);
        t.set_u32(ans + 4, r883);
        t.set_u32(ans + 8, r886);
        t.set_u32(lock, 0x0010_c000 + k);
        t.write(bufs, &[0x33; 0x700]);
        t.set_u32(w(&t, 0x332), SCRATCH + 0x3000 + 0x40 * k);
        // the object given back: no siblings, no children
        t.write(obj, &[0x11; 0x20]);
        t.set_u32(obj, 0);
        t.set_u32(obj + 4, 0);
        t.set_u32(obj + 8, SCRATCH + 0x5100);
        t.set_u32(SCRATCH + 0x510c, 0);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    assert!(t.u32(magic_owna::row(870)) >= 3, "the records drawn");
}
