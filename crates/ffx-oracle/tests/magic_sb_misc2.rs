//! More of `ffx_magic::sb_misc` (the shared-B list's second batch), each
//! against the first DLL carrying the function. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;
mod magic_l3;

use ffx_magic::{Ret, Xmm, sb_misc};
use magic::{Call, Eax, SCRATCH, TRAPS, Twin};

const LOG: u32 = SCRATCH + 0x8000;
const ANS: u32 = SCRATCH + 0x9000;

fn trap(s: u32) -> u32 {
    TRAPS + 16 * s
}

/// Each slot logs its first `n` words (a fresh row per call, 4 rows) and
/// answers the word at `ANS + 4 * k`.
fn log_slots(t: &mut Twin, slots: &[(u32, usize)]) {
    for (k, &(s, n)) in (0u32..).zip(slots) {
        let row = LOG + 0x100 * k;
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0xfc);
            m.w32(row + 0xfc, c.wrapping_add(1));
            let at = row + 0x40 * (c & 3);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(at + 4 * i, w.wrapping_add(c << 28));
            }
            m.r32(ANS + 4 * k)
        });
    }
}

fn answers(t: &mut Twin, v: &[u32]) {
    t.write(LOG, &[0; 0x1000]);
    for (i, x) in (0u32..).zip(v) {
        t.set_u32(ANS + 4 * i, *x);
    }
}

/// Methods/callbacks 1000.. as table traps: 1000 logs its two words and
/// answers the word at `ANS + 0x40` (the next `ip`), 1001 logs one word
/// and answers `ANS + 0x44`, 1002 sets the context's state byte to the
/// word at `ANS + 0x48`.
fn callbacks(t: &mut Twin) {
    for s in 1000..1004 {
        t.set_u32(magic::TABLE + 4 * s, trap(s));
    }
    t.stub(1000, |m, a| {
        let c = m.r32(LOG + 0xffc).wrapping_add(1);
        m.w32(LOG + 0xffc, c);
        m.w32(LOG + 0xe00 + 8 * (c & 0x1f), a[0]);
        m.w32(LOG + 0xe04 + 8 * (c & 0x1f), a[1]);
        // stop after a few steps: the state byte
        if c >= m.r32(ANS + 0x4c) {
            let v = m.r32(a[0] + 0x54) & 0xff00_ffff;
            m.w32(a[0] + 0x54, v | (m.r32(ANS + 0x48) << 16));
        }
        m.r32(ANS + 0x40).wrapping_add(c)
    });
    t.stub(1001, |m, a| {
        let c = m.r32(LOG + 0xff8).wrapping_add(1);
        m.w32(LOG + 0xff8, c);
        m.w32(LOG + 0xd00 + 4 * (c & 0x3f), a[0]);
        m.r32(ANS + 0x44)
    });
}

#[test]
fn dll_mains() {
    for n in [446u16, 269, 132, 206, 255, 171, 102, 565, 84, 197] {
        magic_l3::dll_main(n);
    }
}

#[test]
fn script_run() {
    let Some(mut t) = magic_l3::twin(162) else {
        return;
    };
    callbacks(&mut t);
    let f = t.at(0x7a00);
    let table = t.img.word(f + 0x17);
    for op in 0..4 {
        t.set_u32(table + 4 * op, trap(1000));
    }
    let step = SCRATCH + 0x60;
    t.set_u32(t.img.word(f + 0x56), step);
    let (ctx, ip) = (SCRATCH + 0x100, SCRATCH + 0x200);
    for state in [0u8, 1, 2, 3, 4] {
        for (wait, st) in [(0xffffu16, 5u32), (10, 3), (3, 3), (0, 0)] {
            for (cb, after) in [(0u32, 0u32), (trap(1001), 2), (trap(1001), 0)] {
                answers(&mut t, &[]);
                t.set_u32(ANS + 0x40, ip);
                t.set_u32(ANS + 0x48, after);
                t.set_u32(ANS + 0x4c, 2);
                t.write(ctx, &[0; 0x60]);
                t.set_u32(ctx, ip);
                t.set_u8(ctx + 0x56, state);
                t.set_u16(ctx + 0x54, wait);
                t.set_u32(ctx + 0x24, trap(1001));
                t.set_u32(ctx + 0x28, cb);
                t.set_u32(step, st);
                t.check(f, Call::this(ctx), &[], Eax::Checked, |g, img, fr| {
                    Ret::eax(sb_misc::script_run(g, img, fr.at, fr.ecx))
                });
            }
        }
    }
}

/// A timeline at `this` with three events.
fn timeline(t: &mut Twin, this: u32, now: u32, live: [u32; 3], lens: [u32; 3]) {
    let ev = SCRATCH + 0x600;
    t.write(this, &[0; 0x110]);
    t.set_u32(this, now);
    t.set_u32(this + 4, 3);
    t.set_u32(this + 8, live.iter().sum());
    t.set_u32(this + 0x10c, ev);
    for i in 0..3u32 {
        t.set_u32(this + 0xc + 8 * i, live[i as usize]);
        let e = ev + 0x18 * i;
        t.set_u32(e, 2 * i);
        t.set_u32(e + 4, lens[i as usize]);
        t.set_u32(e + 8, if i == 1 { 0 } else { trap(1001) });
        t.set_u32(e + 0xc, if i == 2 { 0 } else { trap(1001) });
        t.set_u32(e + 0x10, trap(1001));
        t.set_u32(e + 0x14, 0xa0 + i);
    }
}

#[test]
fn timelines() {
    let Some(mut t) = magic_l3::twin(20) else {
        return;
    };
    callbacks(&mut t);
    log_slots(&mut t, &[(253, 0)]);
    let this = SCRATCH + 0x100;
    for now in [0u32, 2, 3, 4, 5] {
        for live in [[1u32, 1, 1], [0, 1, 0], [1, 0, 0], [0, 0, 0]] {
            for lens in [[1u32, 2, 1], [u32::MAX, u32::MAX, 3]] {
                for (tick, ready) in [(0u32, 0u32), (1, 1)] {
                    answers(&mut t, &[ready]);
                    t.set_u32(ANS + 0x44, tick);
                    timeline(&mut t, this, now, live, lens);
                    t.check(
                        t.at(0x2dd0),
                        Call::this(this),
                        &[],
                        Eax::Checked,
                        |g, img, fr| Ret::eax(sb_misc::timeline_run(g, img, fr.at, fr.ecx)),
                    );
                    timeline(&mut t, this, now, live, lens);
                    t.check(
                        t.at(0x2eb0),
                        Call::this(this),
                        &[],
                        Eax::Checked,
                        |g, _img, fr| Ret::eax(sb_misc::timeline_stop(g, fr.ecx)),
                    );
                }
            }
        }
    }
}

#[test]
fn timeline_stop_global() {
    for (n, rva) in [(206u16, 0x30f0u32), (255, 0x2430)] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        callbacks(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let (live_n, count, evp, live) = (w(&t, 0x02), w(&t, 0x0d), w(&t, 0x18), w(&t, 0x1d));
        let ev = SCRATCH + 0x600;
        for (lv, c) in [
            ([0u32, 0, 0], 3u32),
            ([1, 0, 1], 3),
            ([1, 1, 1], 2),
            ([1, 1, 1], 0),
        ] {
            answers(&mut t, &[]);
            t.set_u32(live_n, lv.iter().sum());
            t.set_u32(count, c);
            t.set_u32(evp, ev);
            for i in 0..3u32 {
                t.set_u32(live + 8 * i, lv[i as usize]);
                t.set_u32(ev + 0x18 * i + 0x10, if i == 1 { 0 } else { trap(1001) });
                t.set_u32(ev + 0x18 * i + 0x14, 0xb0 + i);
            }
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(sb_misc::timeline_stop_global(g, img, fr.at))
            });
        }
    }
}

#[test]
fn sprites() {
    let Some(mut t) = magic_l3::twin(271) else {
        return;
    };
    let obj = SCRATCH + 0x100;
    for _ in 0..3 {
        t.write(obj, &[0xa5; 0x260]);
        t.check(
            t.at(0x2130),
            Call {
                eax: 0,
                ecx: obj,
                edx: 0x0bb0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, _img, fr| Ret::eax(sb_misc::sprite_init(g, fr.ecx, fr.edx)),
        );
    }
    drop(t);
    let Some(mut t) = magic_l3::twin(132) else {
        return;
    };
    let col = SCRATCH + 0x80;
    for i in 0..4 {
        t.set_u32(col + 4 * i, 0x1111_1111 * (i + 1));
    }
    t.write(obj, &[0xa5; 0x260]);
    t.check(
        t.at(0x2640),
        Call {
            eax: 0,
            ecx: obj,
            edx: 0x0bb0,
            ..Call::default()
        },
        &[col],
        Eax::Checked,
        |g, _img, fr| Ret::eax(sb_misc::sprite_init_colour(g, fr.esp, fr.ecx, fr.edx)),
    );
}

#[test]
fn actor_scale_and_reset() {
    let Some(mut t) = magic_l3::twin(162) else {
        return;
    };
    let f = t.at(0x6180);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (own, bytes, chrs) = (w(&t, 0x0e), w(&t, 0x15), w(&t, 0x20));
    t.set_u32(own, 0x0c0c);
    t.set_u32(bytes, SCRATCH + 0x100);
    t.write(SCRATCH + 0x100, &[3, 1, 0, 2]);
    for i in 0..4u32 {
        t.set_u32(chrs + 4 * i, 0xc400 + i);
    }
    t.stub(364, |m, a| {
        m.w32(LOG, a[0]);
        0x364
    });
    t.stub_st0(364, |_, a| f64::from(a[0] & 0xff) * 0.5 + 0.25);
    t.stub(16, |m, a| {
        m.w32(LOG + 4, a[0]);
        0x16
    });
    t.stub_st0(16, |_, a| 3.0 - f64::from(a[0] & 0xf));
    for index in [u32::MAX, 0, 1, 3] {
        t.check(f, Call::this(index), &[], Eax::Checked, |g, img, fr| {
            let (x, eax) = sb_misc::actor_scale(g, img, fr.at, fr.ecx);
            Ret::xmm0(Xmm::f32(x), eax)
        });
    }
    // the camera reset
    log_slots(&mut t, &[(287, 3), (283, 2), (281, 1)]);
    let f = t.at(0x50d0);
    let mode = t.img.word(f + 0x42);
    for m in [0u32, 1] {
        answers(&mut t, &[0x287, 0x283, 0x281]);
        t.set_u32(mode, m);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_misc::effect_reset(g, img, fr.at))
        });
    }
}

#[test]
fn lookup_actors() {
    let Some(mut t) = magic_l3::twin(197) else {
        return;
    };
    let f = t.at(0x1540);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (n, mode, bytes) = (w(&t, 0x34), w(&t, 0x42), w(&t, 0x4a));
    log_slots(&mut t, &[(413, 1), (215, 1)]);
    t.write(bytes, &[2, 0, 5, 1, 7]);
    for (count, m) in [(0u32, 0u32), (3, 0), (5, 1), (0xffff_ffff, 0)] {
        answers(&mut t, &[0x413, 0x215]);
        t.set_u32(n, count);
        t.set_u32(mode, m);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_misc::lookup_actors(g, img, fr.at))
        });
    }
}

#[test]
fn draw_frame() {
    let Some(mut t) = magic_l3::twin(446) else {
        return;
    };
    let f = t.at(0x57c0);
    let lock = SCRATCH + 0x50;
    t.set_u32(t.img.word(f + 0x4e), lock);
    let table = SCRATCH + 0x400;
    log_slots(
        &mut t,
        &[
            (941, 1),
            (772, 2),
            (773, 2),
            (774, 5),
            (775, 2),
            (776, 1),
            (777, 1),
            (778, 6),
        ],
    );
    let obj = SCRATCH + 0x100;
    let entry = SCRATCH + 0x200;
    let rec = SCRATCH + 0x300;
    t.set_u32(obj, 0x0bbb);
    t.set_u16(obj + 8, 0x0808);
    t.set_u16(entry, 0x10);
    // the frame entry at table + 0x10: +4 an offset to its byte
    t.set_u32(table + 0x14, 0x20);
    t.set_u8(table + 0x10 + 0x20 + 4, 0x5c);
    t.set_u32(rec + 0x1c, 0x1c1c);
    t.stub_edx(772, |_, a| a[0] ^ 0x00ed_0000);
    for ready in [0u32, 1] {
        answers(
            &mut t,
            &[table, 0x772, ready, 0x774, 0x775, 0x0700, 0x0007, 0x778],
        );
        t.set_u32(lock, 0x42);
        t.check(
            f,
            Call {
                eax: 0,
                ecx: obj,
                edx: entry,
                ..Call::default()
            },
            &[rec],
            Eax::Checked,
            |g, img, fr| Ret::eax(sb_misc::draw_frame(g, img, fr.at, fr.esp, fr.ecx, fr.edx)),
        );
    }
}

#[test]
fn small_0446() {
    let Some(mut t) = magic_l3::twin(446) else {
        return;
    };
    // the two state matches
    for rva in [0x3020u32, 0x4980] {
        let f = t.at(rva);
        let block = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(f + 0x05), block);
        let (a, b) = (t.img.word(f + 0x0b), t.img.word(f + 0x11));
        let ctx = SCRATCH + 0x100;
        for (x, y) in [(1u32, 1u32), (1, 2)] {
            t.set_u32(block + a, x);
            t.set_u32(block + b, y);
            t.set_u32(ctx + 0x54, 0xffff_ffff);
            t.check_entry(f, Call::default(), &[ctx], Eax::Checked);
        }
    }
    // the model placements
    log_slots(&mut t, &[(219, 2), (237, 4), (236, 3), (257, 4)]);
    t.stub_st0(277, |_, a| 0.5 + f64::from(a[0] & 0xf));
    let node = SCRATCH + 0x200;
    for i in 0..0x40u32 {
        t.set_u32(node + 0x80 + 4 * i, (1.0f32 + i as f32).to_bits());
    }
    answers(&mut t, &[0x0219_0003, 0x237, 0x236, 0x257]);
    t.check_entry(t.at(0x5740), Call::default(), &[node], Eax::Checked);
    t.check_entry(t.at(0x13f0), Call::default(), &[node], Eax::Checked);
    t.check_entry(t.at(0x5790), Call::default(), &[], Eax::Checked);
    // the light ranges
    log_slots(&mut t, &[(357, 3), (356, 3), (362, 4)]);
    t.check(
        t.at(0x4be0),
        Call::default(),
        &[],
        Eax::Checked,
        |g, img, fr| Ret::eax(sb_misc::light_ranges(g, img, fr.at)),
    );
    // the own part's release: the list holds it or not
    let f = t.at(0x4080);
    let blk = SCRATCH + 0x2_0000;
    t.set_u32(t.img.word(f + 0x03), blk);
    t.set_u32(t.img.word(f + 0x1e), SCRATCH + 0x60);
    log_slots(&mut t, &[(713, 1)]);
    let list = t.img.word(f + 0x10);
    let part = blk + 0x1300;
    let node = SCRATCH + 0x700;
    for holds in [false, true] {
        t.set_u32(node, 0);
        t.set_u32(node + 4, 0);
        t.set_u32(node + 8, if holds { part } else { 0x77 });
        t.set_u32(list, node);
        t.set_u32(list + 4, node);
        t.set_u32(list + 8, 0);
        t.set_u32(list + 0xc, 1);
        t.set_u16(part + 0x124, 0xffff);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn pools() {
    let Some(mut t) = magic_l3::twin(269) else {
        return;
    };
    let f = t.at(0x3490);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (idx, n, base) = (w(&t, 0x01), w(&t, 0x09), w(&t, 0x16));
    let recs = SCRATCH + 0x1000;
    t.set_u32(base, recs);
    for (count, start, busy, keep) in [
        (4u32, 0u32, 0u32, 0xffffu32),
        (4, 2, 0b0100, 0xffff),
        (4, 1, 0b1111, 0xffff),
        (4, 1, 0b1111, 0b0100),
        (0, 0, 0, 0),
    ] {
        for i in 0..6 {
            let r = recs + 0x140 * i;
            t.set_u32(r + 0x130, (busy >> i) & 1);
            t.set_u16(
                r + 0x124,
                if (keep >> i) & 1 != 0 { 0x8000 } else { 0x7fff },
            );
        }
        t.set_u32(n, count);
        t.set_u32(idx, start);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_misc::pool_take_140(g, img, fr.at))
        });
    }
    // the shuffle
    let f = t.at(0x2b10);
    let (kinds, total) = (t.img.word(f + 0x17), t.img.word(f + 0x1e));
    let counts = t.img.word(f + 0x46);
    let out = SCRATCH + 0x3000;
    t.stub(324, |m, _| {
        let s = m.r32(LOG).wrapping_mul(1_103_515_245).wrapping_add(12345);
        m.w32(LOG, s);
        s >> 8
    });
    for (k, tot, cs) in [
        (1u32, 5u32, [2u32, 3, 0]),
        (0, 0, [0, 0, 0]),
        (3, 6, [1, 2, 3]),
        (2, 4, [4, 1, 0]),
        (3, 0, [1, 1, 1]),
    ] {
        t.write(out, &[0xee; 0x10]);
        t.set_u32(kinds, k);
        t.set_u32(total, tot);
        for (i, c) in (0u32..).zip(cs) {
            t.set_u32(counts + 0x1c * i, c);
        }
        t.check(f, Call::this(out), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_misc::deal(g, img, fr.at, fr.esp, fr.ecx))
        });
    }
}

#[test]
fn draw_loops() {
    let Some(mut t) = magic_l3::twin(132) else {
        return;
    };
    let f = t.at(0x38a0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let lock = SCRATCH + 0x50;
    t.set_u32(w(&t, 0x43), lock);
    let recs = SCRATCH + 0x1000;
    t.set_u32(w(&t, 0x13), recs);
    let (n, m) = (w(&t, 0x0b), w(&t, 0x76));
    t.stub(941, |_, a| a[0] + 1);
    t.stub(772, |m, a| {
        let c = m.r32(LOG).wrapping_add(a[0] ^ a[1]);
        m.w32(LOG, c);
        c
    });
    t.stub(779, move |m, a| {
        let c = m.r32(LOG + 4).wrapping_add(a[0]).wrapping_add(m.r32(lock));
        m.w32(LOG + 4, c);
        0x779
    });
    for (nn, mm) in [(0u32, 0u32), (2, 0), (0, 1), (2, 2)] {
        t.set_u32(n, nn);
        t.set_u32(m, mm);
        t.set_u32(lock, 7);
        t.check(
            f,
            Call {
                eax: 0x99,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(sb_misc::draw_all(g, img, fr.at, fr.eax)),
        );
    }
    drop(t);
    let Some(mut t) = magic_l3::twin(171) else {
        return;
    };
    let f = t.at(0x3900);
    let lock = SCRATCH + 0x50;
    t.set_u32(t.img.word(f + 0x53), lock);
    t.set_u32(t.img.word(f + 0x0f), SCRATCH + 0x1000);
    let n = t.img.word(f + 0x08);
    log_slots(&mut t, &[(941, 1), (772, 2), (779, 2)]);
    for nn in [0u32, 1, 3] {
        answers(&mut t, &[0x941, 0x772, 0x779]);
        t.set_u32(n, nn);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_misc::draw_grid(g, img, fr.at))
        });
    }
}

#[test]
fn aim_parent() {
    let Some(mut t) = magic_l3::twin(42) else {
        return;
    };
    callbacks(&mut t);
    let (cur, owner, parent, na, nb) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x600,
        SCRATCH + 0x700,
    );
    let vt = SCRATCH + 0x800;
    t.set_u32(vt + 0x1c, trap(1001));
    // slot 234 writes a vector from its two sources
    t.stub(234, |m, a| {
        for i in 0..4 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x234
    });
    for (mode, om, pa, pb) in [
        (3u8, 3u8, [1.0f32, 2.0, 3.0], [0.0f32, 0.0, 0.0]),
        (3, 9, [1.0, 2.0, 3.0], [-4.0, 0.5, 1.0]),
        (3, 9, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        (3, 9, [-2.0, -1.0, 0.0], [0.0, 1.0, 5.0]),
        (3, 9, [0.0, 4.0, -3.0], [0.0, 0.0, 0.0]),
    ] {
        t.set_u32(cur, owner);
        t.set_u8(cur + 4, 0);
        t.set_u8(cur + 5, mode);
        t.set_u8(owner + 0x1a, om);
        t.set_u32(owner + 0x10, owner);
        t.set_u32(owner + 0x14, parent);
        t.set_u32(owner + 0x2c, na);
        t.set_u32(owner + 0x30, nb);
        for i in 0..3u32 {
            t.set_u32(owner + 0x34 + 4 * i, 0x100 * (i + 1));
            t.set_u32(owner + 0x40 + 4 * i, (0.25f32 * (i + 1) as f32).to_bits());
            t.set_u32(parent + 0x1c + 4 * i, 0x0001_7ff0 - 0x4000 * i);
            t.set_u32(na + 0x1c + 4 * i, pa[i as usize].to_bits());
            t.set_u32(nb + 0x1c + 4 * i, pb[i as usize].to_bits());
        }
        t.set_u32(na, vt);
        t.set_u32(nb, vt);
        t.check_thunked(
            t.at(0x4fe0),
            t.at(0x51a0),
            false,
            Call::this(cur),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(sb_misc::aim_parent(g, img, fr.at, fr.esp, fr.ecx)),
        );
    }
}

#[test]
fn small_misc() {
    let Some(mut t) = magic_l3::twin(42) else {
        return;
    };
    t.check(
        t.at(0x7e90),
        Call {
            eax: 5,
            ..Call::default()
        },
        &[],
        Eax::Checked,
        |g, img, fr| Ret::eax(sb_misc::static_nodes_init(g, img, fr.at, fr.eax)),
    );
    drop(t);
    let Some(mut t) = magic_l3::twin(102) else {
        return;
    };
    let obj = SCRATCH + 0x100;
    t.write(obj, &[0xa5; 0x140]);
    t.check(
        t.at(0x2030),
        Call::this(obj),
        &[0x1111, 0x2222],
        Eax::Checked,
        |g, _img, fr| Ret::eax(sb_misc::anim_reset_2(g, fr.esp, fr.ecx)),
    );
    drop(t);
    let Some(mut t) = magic_l3::twin(565) else {
        return;
    };
    log_slots(
        &mut t,
        &[
            (719, 3),
            (220, 6),
            (628, 1),
            (213, 2),
            (312, 4),
            (371, 2),
            (646, 1),
        ],
    );
    answers(&mut t, &[0x719, 0x220, 0x628, 0x213, 0x312, 0x371, 2]);
    t.check(
        t.at(0x8610),
        Call {
            eax: 0,
            ecx: 0x0c0c,
            edx: 0x0d0d,
            ..Call::default()
        },
        &[0xa0, 0xa1],
        Eax::Checked,
        |g, img, fr| Ret::eax(sb_misc::call_220(g, img, fr.at, fr.esp, fr.ecx, fr.edx)),
    );
    let f = t.at(0x6dd0);
    for v in [0u16, 1] {
        t.set_u16(t.img.word(f + 0x15), v);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_misc::call_628_213(g, img, fr.at))
        });
    }
    t.check_entry(t.at(0x1650), Call::default(), &[], Eax::Checked);
    let f = t.at(0x17e0);
    let count = t.img.word(f + 0x5c);
    for _ in 0..2 {
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_misc::pick_variant(g, img, fr.at))
        });
        t.set_u32(count, 3);
    }
    log_slots(&mut t, &[(305, 0), (302, 3), (303, 3), (304, 3)]);
    for r in [0u32, 1, 2, 3] {
        answers(&mut t, &[r, 0x0001_0302, 0x0002_0303, 0xabc4_0304]);
        t.check(
            t.at(0xa310),
            Call::default(),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(sb_misc::read_pads(g, img, fr.at)),
        );
    }
    drop(t);
    let Some(mut t) = magic_l3::twin(84) else {
        return;
    };
    t.stub_st0(790, |_, _| 1.5);
    log_slots(&mut t, &[(719, 3), (551, 3)]);
    answers(&mut t, &[0x0719_0000, 0x551]);
    t.check_entry(t.at(0x10a0), Call::default(), &[0x00ab_cdef], Eax::Checked);
}
