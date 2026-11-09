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
mod magic_r11;

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
            // junk where the frame's record goes (zeros written over zeros
            // prove nothing)
            t.write(magic::STACK_TOP - 0x1100, &[0x5c + k as u8; 0x1000]);
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
            t.set_u8(actor + 1, 0x70 + k as u8);
            t.set_u32(w(&t, actor_at), actor);
            if !v55 {
                let tab = w(&t, 0xf0);
                for i in 0..0x20u32 {
                    t.set_u32(tab + 4 * i, 0x0ab0_0000 + i);
                }
            }
            // the place: by model index (mode 0) or by handle
            t.set_u32(t.img.word(place + 0x02), mode);
            // (the place's index pointer is the actor byte's: no write
            // over it)
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
    // the slots run under the lock log it
    let seen = SCRATCH + 0x740;
    let log_lock = move |m: &mut dyn StubMem, s: u32| {
        let at = seen + 4 * (s % 4);
        let v = m.r32(at).rotate_left(5) ^ m.r32(lock) ^ s;
        m.w32(at, v);
    };
    for (s, n, i) in [(773u32, 4usize, 0u32), (883, 2, 1), (886, 4, 2)] {
        rec(&mut t, s, n, move |m: &mut dyn StubMem, _: &[u32]| {
            log_lock(m, s);
            m.r32(ans + 4 * i)
        });
    }
    for s in [779u32, 870] {
        rec(&mut t, s, 2, move |m: &mut dyn StubMem, a: &[u32]| {
            log_lock(m, s);
            a[0] ^ (s << 16)
        });
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
        // the code's qword and halfword (the record's `E + 0x14..`)
        t.set_u32(w(&t, 0xd2), 0x0d20_0000 + k);
        t.set_u32(w(&t, 0xd2) + 4, 0x0d24_0000 + k);
        t.set_u16(w(&t, 0xd9), 0x0d90 + k as u16);
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

/// What `r24::start_up_0311` (`magic_0311`, at `f`) needs: the pools,
/// the part set-up's buffers, the object start's, the record start's free
/// ring (`free` nodes) and the two lists (empty or not); slot 262 writes
/// a point from its first word.
fn start_up_0311_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1208);
    magic_r11::object_slots(t);
    rec(t, 262, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    for s in [12u32, 333] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.375 - 1.5);
    }
    magic_owna::filler(t, 235, 1, 16);
    for s in [328u32, 329] {
        rec(t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ s;
                m.w32(a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + i as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4)]);
    // the record start's vectors: slot 217 fills its second word's, 234
    // and 233 their first's
    rec(t, 217, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            m.w32(
                a[1] + 4 * i,
                (0.75 + i as f32 + (a[0] & 0xf) as f32).to_bits(),
            );
        }
        0x217
    });
    filler(t, 234, 3, 4);
    filler(t, 233, 3, 4);
    let start = t.img.rel32_target(f + 0x63);
    magic_r11::object_start_auto(t, start);
    // the record start's free ring and busy list
    let gn = t.img.rel32_target(f + 0x34d);
    let gw = |t: &Twin, o: u32| t.img.word(gn + o);
    // (the same ring as the pool take's: every node take uses it)
    magic_r11::ring(t, gw(t, 0x1b2), gw(t, 0x1fb), 8, SCRATCH + 0x9000);
    t.write(SCRATCH + 0x640, &[0x46; 0x20]);
    t.set_u32(gw(t, 0x262), if listed { SCRATCH + 0x640 } else { 0 });
    t.write(gw(t, 0x26f), &[0x26; 0xc0]);
    // the actor and the index byte with its table
    t.set_u32(w(t, 0x6e), 0x0000_0040 + k);
    let idx = SCRATCH + 0x1e0;
    t.set_u8(idx, (2 + k) as u8);
    t.set_u32(w(t, 0x19a), idx);
    for i in 0..0x20 {
        t.set_u32(w(t, 0x1bb) + 4 * i, 0x0000_0080 + 3 * i);
    }
    t.set_u32(w(t, 0x35b), battle);
    t.set_u32(w(t, 0x396), battle);
    t.set_u32(w(t, 0x352), 0x0035_0000 + k);
    // junk in the globals it stores constants to (the DLL's data may
    // already hold them)
    junk_ops(
        t,
        f,
        &[
            0xa8, 0xf9, 0x139, 0x143, 0x14d, 0x157, 0x175, 0x17f, 0x377, 0x3a7, 0x3b1, 0x3d4,
            0x3e6, 0x3ee, 0x3f5, 0x400,
        ],
    );
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x475), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x485), SCRATCH + 0x600);
}

/// `magic_0311` 0x100013e0 (`r24::start_up_0311`, internal) and its load
/// step 0x100012b0 (`r9_load::load_0311`): in and out of a battle, the
/// lists empty and not; the turns' wraps through the angle slots; the
/// load step's waits and its start.
#[test]
fn start_up_0311() {
    let Some(mut t) = twin(311) else { return };
    let f = t.at(0x13e0);
    for (k, more) in [(0u32, false), (1, true), (0, true), (1, false)]
        .into_iter()
        .enumerate()
    {
        start_up_0311_setup(&mut t, f, k as u32, more);
        t.check(f, Call::default(), &[], Eax::Checked, r24::start_up_0311);
    }
    let load = t.at(0x12b0);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0311_setup(&mut t, f, 3, (1, true));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// What `r24::start_up_0149` (`magic_0149`, at `f`) needs: the pools,
/// the part set-up's buffers, the four object starts', the target byte
/// and its table, the task start's and the task list (empty or not);
/// slot 262 writes a point from its first word, 268 a vector, 12 a float.
fn start_up_0149_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1278);
    magic_r11::object_slots(t);
    rec(t, 262, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + 0.5;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.375 - 1.5);
    recs(t, &[(326, 5), (413, 1), (215, 1), (711, 3), (710, 4)]);
    for call in [0x62u32, 0x24a, 0x379, 0x447] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    // the actor, the target byte and its table
    t.set_u32(w(t, 0x6d), 0x0000_0040 + k);
    let idx = SCRATCH + 0x1e0;
    t.set_u8(idx, (2 + k) as u8);
    t.set_u8(idx + 1, 0x77);
    t.set_u32(w(t, 0xa5), idx);
    for i in 0..0x20 {
        t.set_u32(w(t, 0xc1) + 4 * i, 0x0000_0080 + 3 * i);
    }
    // the actor block's battle flag
    let me = t.img.rel32_target(f + 0x4ab);
    t.set_u32(t.img.word(me + 0x05), battle);
    // junk in the globals it stores constants to
    junk_ops(
        t,
        f,
        &[
            0xb3, 0x21a, 0x241, 0x2b4, 0x310, 0x31a, 0x324, 0x32e, 0x338, 0x356, 0x360, 0x36a,
            0x3cd, 0x3d7, 0x43e, 0x498, 0x4a2,
        ],
    );
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x4be), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x4ce), SCRATCH + 0x600);
    // the task start's list
    let ts = t.img.rel32_target(f + 0x223);
    t.write(SCRATCH + 0x680, &[0x46; 0x10]);
    t.set_u32(
        t.img.word(ts + 0x75),
        if listed { 0 } else { SCRATCH + 0x680 },
    );
    t.set_u32(t.img.word(ts + 0x8c), SCRATCH + 0x680);
}

/// `magic_0149` 0x10003450 (`r24::start_up_0149`, internal) and its load
/// step 0x10003320 (`r9_load::load_0149`): in and out of a battle, the
/// lists empty and not; the pitch and the turns through the point slots;
/// the load step's waits and its start.
#[test]
fn start_up_0149() {
    let Some(mut t) = twin(149) else { return };
    let f = t.at(0x3450);
    for (k, more) in [(0u32, false), (1, true), (0, true), (1, false)]
        .into_iter()
        .enumerate()
    {
        start_up_0149_setup(&mut t, f, k as u32, more);
        t.check(f, Call::default(), &[], Eax::Checked, r24::start_up_0149);
    }
    let load = t.at(0x3320);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0149_setup(&mut t, f, 3, (1, true));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// What `r24::start_up_0435` (`magic_0435`, at `f`) needs: the pools,
/// the part set-up's buffers, the object start's, the key curves' free
/// ring (`free` nodes) and busy list, the task list (empty or not); slot
/// 262 writes a point from its first word, 229 a vector from its last
/// two, 333 an angle.
fn start_up_0435_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 550);
    magic_r11::object_slots(t);
    rec(t, 262, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    // the angle from the case: the yaws above, inside and below their wraps
    let ang = SCRATCH + 0x7a0;
    t.stub_st0(333, move |m, a| {
        f64::from(f32::from_bits(m.r32(ang))) + f64::from(a[0] & 0xf) * 0.01
    });
    t.set_u32(ang, ([-9.0f32, 0.3, 9.0, 2.0][(k % 4) as usize]).to_bits());
    filler(t, 235, 1, 16);
    rec(t, 329, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2];
            m.w32(a[0] + 4 * i, v);
        }
        0x329
    });
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 1.5 + (m.r32(a[1] + 4 * i) & 7) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    recs(
        t,
        &[(219, 2), (220, 3), (413, 1), (215, 1), (711, 3), (710, 4)],
    );
    let start = t.img.rel32_target(f + 0x77);
    magic_r11::object_start_auto(t, start);
    let keys = t.img.rel32_target(f + 0x116);
    let kw = |t: &Twin, o: u32| t.img.word(keys + o);
    // the vectors at `V`
    for i in 0..12u32 {
        t.set_u32(kw(t, 0x180) + 4 * i, (0.5 * i as f32 - 1.75).to_bits());
    }
    // the target byte (its neighbour junk) and its table
    let idx = SCRATCH + 0x1e0;
    t.set_u8(idx, (2 + k) as u8);
    t.set_u8(idx + 1, 0x77);
    t.set_u32(kw(t, 0x1b), idx);
    for i in 0..0x20 {
        t.set_u32(kw(t, 0x37) + 4 * i, 0x0000_0080 + 3 * i);
    }
    // the key curves' free ring and busy list
    magic_r11::ring(t, kw(t, 0x2f2), kw(t, 0x346), 4, SCRATCH + 0x9000);
    t.write(SCRATCH + 0x640, &[0x46; 0x20]);
    t.set_u32(kw(t, 0x3ad), if listed { SCRATCH + 0x640 } else { 0 });
    t.set_u32(kw(t, 0x3c0), SCRATCH + 0x640);
    t.set_u32(kw(t, 0x2a0), battle);
    t.set_u32(w(t, 0x163), battle);
    t.set_u32(w(t, 0x19e), battle);
    t.set_u32(w(t, 0xd), 0x0000_0040 + k);
    t.set_u32(w(t, 0x97), 0x0000_0050 + k);
    // junk in the globals it stores constants to
    junk_ops(
        t,
        f,
        &[
            0x3b, 0x42, 0xba, 0x130, 0x13a, 0x17f, 0x1af, 0x1b9, 0x1c3, 0x1cd, 0x1d6,
        ],
    );
    junk_ops(
        t,
        keys,
        &[0x2b7, 0x2c1, 0x2d5, 0x302, 0x30d, 0x317, 0x321, 0x116],
    );
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x1e6), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x1f6), SCRATCH + 0x600);
}

/// `magic_0435` 0x10001340 (`r24::start_up_0435`, internal, with
/// `r24::curve_keys_0435` 0x10001660) and its load step 0x10001210
/// (`r9_load::load_0435`): in and out of a battle, the lists empty and
/// not, the angles across the wraps; the key curves on their own; the
/// load step's waits and its start.
#[test]
fn start_up_0435() {
    let Some(mut t) = twin(435) else { return };
    let f = t.at(0x1340);
    // (case, in a battle, the lists busy)
    for (k, battle, listed) in [
        (0u32, 0u32, false),
        (1, 1, true),
        (2, 0, true),
        (3, 1, false),
        (6, 0, true),
    ] {
        start_up_0435_setup(&mut t, f, k, (battle, listed));
        t.check(f, Call::default(), &[], Eax::Checked, r24::start_up_0435);
    }
    let keys = t.at(0x1660);
    for k in [3u32, 9] {
        start_up_0435_setup(&mut t, f, k, (k & 1, true));
        t.check(
            keys,
            Call::default(),
            &[],
            Eax::Checked,
            r24::curve_keys_0435,
        );
    }
    let load = t.at(0x1210);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0435_setup(&mut t, f, 3, (1, true));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// `r24::visit_0342` (`magic_0342` 0x10001990, `ecx` = the cursor) and
/// its thunk 0x10001e80: the pass already done (the cookie word); the
/// children visited, then past `K` with no records (the leave, with and
/// without a parent) or with some (the draw); below `K` counting down
/// (the draw) or out (new records from the ring, one piece and two); the
/// methods logged through slot 1001, slot 324 a counter, the records'
/// motion steps past their last step or loaded; from the fourth case on
/// the fade's bound is 1.5, so the 0.7 cap is reached.
#[test]
#[allow(clippy::too_many_lines)] // the stubs, the ring and the cases
fn visit_0342() {
    let Some(mut t) = twin(342) else { return };
    let f = t.at(0x1990);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // the methods +0x1c: slot 1001, logging its cursor and the node in it
    let vt = SCRATCH + 0x880;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1001);
    rec(&mut t, 1001, 1, |m: &mut dyn StubMem, a: &[u32]| {
        let at = SCRATCH + 0x7b0;
        let v = m.r32(at).rotate_left(5) ^ m.r32(a[0]) ^ a[0];
        m.w32(at, v);
        0x1001
    });
    let cnt = SCRATCH + 0x7a0;
    rec(&mut t, 324, 0, move |m: &mut dyn StubMem, _: &[u32]| {
        let k = m.r32(cnt).wrapping_add(1);
        m.w32(cnt, k);
        if k.is_multiple_of(2) {
            // multiples of 4, some of them odd multiples (both record kinds)
            (k >> 1) << 2
        } else {
            k.wrapping_mul(0x9e37_79b9) ^ (k << 3)
        }
    });
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, move |m, _| f64::from(m.r32(cnt) & 0xf) * 0.25 + 0.5);
    filler(&mut t, 235, 1, 16);
    rec(&mut t, 237, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, (0.5 * i as f32 + (a[0] & 7) as f32).to_bits());
        }
        0x237
    });
    rec(&mut t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (m.r32(a[1] + 4 * i) & 3) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(&mut t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    rec(&mut t, 231, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(1);
            m.w32(a[0] + 4 * i, v);
        }
        0x231
    });
    // the motion steps' slots (the lock word logged)
    let step_at = t.img.rel32_target(f + 0x44f);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(t.img.word(step_at + 0x24), lock);
    for s in [773u32, 774, 775, 778, 779] {
        rec(&mut t, s, 2, move |m: &mut dyn StubMem, a: &[u32]| {
            let at = SCRATCH + 0x7c0 + 4 * (s % 8);
            let v = m.r32(at).rotate_left(3) ^ m.r32(lock) ^ a[0];
            m.w32(at, v);
            if s == 773 { a[0] & 1 } else { s }
        });
    }
    recs(&mut t, &[(776, 1), (777, 1)]);
    // the rotation's sine table
    let rot = t.img.rel32_target(f + 0x255);
    let table = SCRATCH + 0x2_0000;
    for i in 0..0x1000u32 {
        t.set_u32(table + 4 * i, (i as f32 / 4096.0 - 0.5).to_bits());
    }
    t.set_u32(t.img.word(rot + 0x17), table);
    let c = SCRATCH + 0x100;
    let n0 = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x1400;
    let recs_at = SCRATCH + 0x4000;
    let k_lim = f32::from_bits(t.u32(w(&t, 0x98)));
    // (pass done, children, n0[0x54] below K, count, records, parent,
    // ring head, ring used)
    for (k, (done, kids, below, count, live, par, head, used)) in [
        (true, 0u32, false, 0u32, 0u32, false, 0u16, 0u16),
        (false, 2, false, 0, 0, true, 0, 0),
        (false, 1, false, 0, 0, false, 0, 0),
        (false, 0, false, 0, 3, false, 0, 8),
        (false, 2, true, 3, 2, false, 1, 4),
        (false, 0, true, 0, 0, false, 2, 1),
        (false, 1, true, 0, 1, false, 6, 8),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(cnt, 0x10 * k);
        t.set_u32(lock, 0x5a5a_0000 + k);
        t.write(n0, &[0x5c; 0x100]);
        t.write(c, &[0x11; 0x10]);
        t.set_u32(c, n0);
        t.set_u8(c + 5, 7);
        t.set_u8(n0 + 0x1a, if done { 7 } else { 3 });
        // the children's ring (`+0x10`, linked by `+8`)
        let kid = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
        t.set_u32(n0 + 0x10, if kids == 0 { n0 } else { kid(0) });
        for i in 0..kids {
            t.write(kid(i), &[0x22; 0x40]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { n0 } else { kid(i + 1) });
        }
        // the child the cursor visits
        let ch = SCRATCH + 0x1c00;
        t.write(ch, &[0x33; 0x60]);
        t.set_u32(ch, vt);
        t.set_u32(ch + 0x54, 0x0000_0054 + k);
        t.set_u32(n0 + 0x48, ch);
        let x = if below { k_lim - 1.0 } else { k_lim + 0.5 };
        t.set_u32(n0 + 0x54, x.to_bits());
        t.set_u32(n0 + 0x5c, count);
        t.set_u32(n0 + 0x58, live);
        // the leave's parent (none: `+0x14` 0)
        t.write(parent, &[0x44; 0x20]);
        t.set_u32(n0 + 0x14, if par { parent } else { 0 });
        t.set_u32(n0 + 4, SCRATCH + 0x1480);
        t.set_u32(n0 + 8, SCRATCH + 0x1490);
        // the ring of 8 records of 0x40 bytes and its span
        t.set_u16(n0 + 0x1c, 8);
        t.set_u16(n0 + 0x1e, 0x40);
        t.set_u32(n0 + 0x20, recs_at);
        t.set_u16(n0 + 0x30, head);
        t.set_u16(n0 + 0x32, used);
        t.set_u16(n0 + 0x34, 0);
        t.set_u32(n0 + 0x24, recs_at);
        t.set_u16(n0 + 0x2c, used.min(5));
        t.set_u16(n0 + 0x2e, used.saturating_sub(5));
        t.set_u32(n0 + 0x28, recs_at + 0x140);
        // two motion sequences: kind 0 past its last step, kind 1 two steps
        let (q0, q1) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
        for (q, n) in [(q0, 0i16), (q1, 2)] {
            t.write(q, &[0x18; 0x100]);
            t.set_u16(q + 6, n.cast_unsigned());
            for i in 0..4u32 {
                t.set_u16(q + 0x10 + 8 * i, 0x80);
                t.set_u16(q + 0x12 + 8 * i, 0x200);
            }
            t.set_u32(q + 0x84, 0x10);
        }
        t.set_u32(n0 + 0x4c, q0);
        t.set_u32(n0 + 0x50, q1);
        for i in 0..8u32 {
            let r = recs_at + 0x40 * i;
            t.write(r, &[0x26; 0x40]);
            t.set_u8(r + 0x34, u8::from(i % 3 != 2));
            t.set_u32(r + 0x24, i & 1);
            t.set_u16(r + 0x10, 0);
            t.set_u16(r + 0x12, (0x40 * i) as u16);
            t.set_u32(r + 0x20, (1.0 + i as f32).to_bits());
            t.set_u32(r + 0x2c, (0.2 * i as f32).to_bits());
        }
        junk_ops(&mut t, f, &[0x2e0, 0x4a1]);
        if k == 3 {
            // the fade's bound raised from 0.7 to 1.5: records past 1.0
            // after the step reach the cap (unreachable with the DLL's 0.7)
            t.set_u32(w(&t, 0x408), 1.5f32.to_bits());
        }
        let call = Call::this(c);
        t.check(f, call, &[], Eax::Checked, r24::visit_0342);
        if k == 2 {
            // the thunk: `ecx` from its stack word
            t.set_u8(c + 5, 9);
            t.check_entry(t.at(0x1e80), Call::default(), &[c], Eax::Checked);
        }
    }
    assert!(t.u32(magic_owna::row(1001)) >= 4, "the methods ran");
}

/// What `r24::start_up_0419` (`magic_0419`, at `f`) needs: the pools,
/// the part set-up's and the object start's buffers, the target byte
/// and its table, the path task's free ring and list, the task list
/// (empty or not); slot 262 writes a point from its first word, 229 a
/// vector, 248 a sum, the float slots answer from their word.
fn start_up_0419_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1618);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    for s in [12u32, 333, 251, 17, 19] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| {
            f64::from(a[0] & 0xff) * 0.375 - 1.5 + f64::from(s) * 0.125
        });
    }
    filler(t, 235, 1, 16);
    for s in [328u32, 329] {
        rec(t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ s;
                m.w32(a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (m.r32(a[1] + 4 * i) & 3) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    // the path task's vector slots (as `magic_s7_misc.rs`' test answers them)
    rec(t, 217, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            m.w32(
                a[1] + 4 * i,
                (0.25 * i as f32 + (a[0] & 7) as f32).to_bits(),
            );
        }
        0x217
    });
    filler(t, 234, 3, 4);
    filler(t, 233, 3, 4);
    recs(t, &[(256, 5), (413, 1), (215, 1), (711, 3), (710, 4)]);
    let start = t.img.rel32_target(f + 0x2c1);
    magic_r11::object_start_auto(t, start);
    // the actor, the target byte (its neighbour junk) and its table
    t.set_u32(w(t, 0x73), 0x0000_0040 + k);
    let idx = SCRATCH + 0x1e0;
    t.set_u8(idx, (2 + 0x10 * k) as u8);
    t.set_u8(idx + 1, 0x77);
    t.set_u32(w(t, 0x80), idx);
    for i in 0..0x40 {
        t.set_u32(w(t, 0xa1) + 4 * i, 0x0000_0080 + 3 * i);
    }
    t.set_u32(w(t, 0x150), battle);
    t.set_u32(w(t, 0x42a), battle);
    t.set_u32(w(t, 0x465), battle);
    // (the path task takes from the same free ring and list as the
    // start-up: `pools` and the task list below set them)
    // junk in the globals it stores constants to
    junk_ops(
        t,
        f,
        &[
            0x37b, 0x3bb, 0x3c5, 0x3cf, 0x3d9, 0x3f7, 0x401, 0x446, 0x476, 0x480, 0x48a, 0x494,
            0x49d, 0x4ad, 0x4c3, 0x4e5, 0x4ef, 0x4f7, 0x4ff, 0x50c, 0x301,
        ],
    );
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x592), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x5a9), SCRATCH + 0x600);
}

/// `magic_0419` 0x100013e0 (`r24::start_up_0419`, internal) and its load
/// step 0x100012b0 (`r9_load::load_0419`): in and out of a battle (the
/// height from slot 251 or slots 17/19; a target index past 0x20), the
/// lists empty and not, near and far targets (9 or 16 fade steps); the
/// load step's waits and its start.
#[test]
fn start_up_0419() {
    let Some(mut t) = twin(419) else { return };
    let f = t.at(0x13e0);
    for (k, battle, listed) in [
        (0u32, 0u32, false),
        (1, 1, true),
        (2, 0, true),
        (3, 1, false),
    ] {
        start_up_0419_setup(&mut t, f, k, (battle, listed));
        t.check(f, Call::default(), &[], Eax::Checked, r24::start_up_0419);
    }
    let load = t.at(0x12b0);
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0419_setup(&mut t, f, 3, (1, true));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}
