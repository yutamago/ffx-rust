//! Small own functions below `magic_0450` (agent magic-small-lo), batch
//! 3: `magic_0307`'s starts, set-ups and steps of `0313`-`0350`. Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss, clippy::many_single_char_names)]

mod magic;
mod magic_hi;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// Junk under the entry on both sides.
fn dirty(t: &mut Twin) {
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
}

/// `n` bytes that differ from word to word (seeded by `seed`), so a port
/// that reads the wrong word of a block reads another value.
fn varied(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| u8::try_from((i * 7 + usize::from(seed)) % 251).unwrap())
        .collect()
}

/// Every slot in `slots` recorded with its word count, answering a
/// distinct value mixed with its first word.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (0x0abc_0000 | s) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// Free rings of `n` nodes for every pool take reached from `at` (`len`
/// bytes, two calls deep).
fn pools(t: &mut Twin, at: u32, len: u32, n: u32) {
    fn takes(img: &ffx_magic::Image, at: u32, len: u32, depth: u32, out: &mut Vec<u32>) {
        for p in at..at + len {
            if !matches!(img.word(p).to_le_bytes()[0], 0xe8 | 0xe9) {
                continue;
            }
            let t = img.rel32_target(p + 1);
            if !img.in_text(t) {
                continue;
            }
            if img.matches(t, &ffx_magic::sa_pool::TAKE) {
                if !out.contains(&t) {
                    out.push(t);
                }
            } else if depth > 0 {
                takes(img, t, 0x200, depth - 1, out);
            }
        }
    }
    let mut v = Vec::new();
    takes(&t.img, at, len, 2, &mut v);
    for (k, tk) in (0u32..).zip(v) {
        magic_hi::pool(t, tk, SCRATCH + 0x8000 + 0x400 * k, n);
    }
}

/// The object start's set-up (`s4_misc::start_object_at` at `s`).
fn at_setup(t: &mut Twin, s: u32, count: u16) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let pa = t.img.rel32_target(s + 0x97);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
    t.set_u32(t.img.word(s + 0xa2), 0);
}

/// `m0307`'s scale tasks (0x10007980, 0x10002940): the pool free twice.
#[test]
fn scales_0307() {
    let Some(mut t) = twin(307) else { return };
    rec(&mut t, 371, 2, |_, a| a[0] ^ a[1] ^ 0x371);
    let blk = SCRATCH + 0x1_0000;
    for (rva, gop) in [(0x7980u32, 0x2u32), (0x2940, 0x2a)] {
        let at = t.at(rva);
        t.set_u32(t.img.word(at + gop), blk);
        for k in 0..2u32 {
            pools(&mut t, at, 0x50, 2 + k);
            t.write(blk + 0x2500, &varied(0x80, 0x55));
            dirty(&mut t);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `m0307`'s object starts (0x10002aa0, 0x10002b70) and the third
/// object's restart (0x10005270: in the list and not), the pool free and
/// empty.
#[test]
fn starts_0307() {
    let Some(mut t) = twin(307) else { return };
    rec(&mut t, 713, 1, |_, a| 0x0713_0000 ^ a[0]);
    let blk = SCRATCH + 0x1_0000;
    for (rva, sop, cop) in [
        (0x2aa0u32, 0xbu32, 0x1cu32),
        (0x2b70, 0xb, 0x1f),
        (0x5270, 0x3, 0x5b),
    ] {
        let at = t.at(rva);
        let s = t.img.rel32_target(at + cop);
        t.set_u32(t.img.word(at + sop), blk);
        for (k, count) in [3u16, 0].into_iter().enumerate() {
            at_setup(&mut t, s, count);
            t.write(blk, &varied(0x2900, 0x5a));
            if rva == 0x5270 {
                let (list, item) = (t.img.word(at + 0xd), t.img.word(at + 0x8));
                let node = SCRATCH + 0x700;
                t.write(node, &[0; 0x10]);
                t.set_u32(node + 8, item);
                t.set_u32(list, if k == 0 { node } else { 0 });
                t.set_u32(list + 8, 0);
                let p = SCRATCH + 0x740;
                t.set_u32(t.img.word(at + 0x1b), p);
                t.set_u16(t.img.word(at + 0x42), 0x8123);
            }
            dirty(&mut t);
            t.check_entry(at, Call::this(0x0ecc_0007), &[], Eax::Checked);
        }
    }
}

/// `m0313::object_setup` (0x100016f0) and `m0316::scale_two`
/// (0x10001670).
#[test]
fn setups_0313_0316() {
    let Some(mut t) = twin(313) else { return };
    recs(&mut t, &[(440, 4), (450, 2)]);
    let at = t.at(0x16f0);
    t.set_u32(t.img.word(at + 0x2b), 0x00e0_002b);
    t.set_u32(t.img.word(at + 0x60), 0x7777_7777);
    let a = SCRATCH + 0x3000;
    t.write(a, &varied(0x140, 0x44));
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(316) else { return };
    recs(&mut t, &[(256, 5)]);
    let at = t.at(0x1670);
    t.set_u32(t.img.word(at + 0x29), 0x00e0_0029);
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// `m0318::count_step` (0x100014d0): the motion running, the objects
/// running, the count below 32, the count at 32, done set.
#[test]
fn count_step_0318() {
    let Some(mut t) = twin(318) else { return };
    let at = t.at(0x14d0);
    let ms = t.img.rel32_target(at + 0x5);
    let ro = t.img.rel32_target(at + 0xc);
    recs(&mut t, &[(296, 2), (257, 2)]);
    t.set_u32(t.img.word(at + 0x45), 0x00e0_0045);
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    for (done, running, n, fin) in [
        (false, false, 3u32, 0u32),
        (true, true, 3, 0),
        (true, false, 0x1f, 0),
        (true, false, 0x20, 0),
        (true, false, 0xffff_fff0, 0),
        (true, false, 3, 1),
    ] {
        magic_hi::motion(&mut t, ms, done);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(t.img.word(at + 0x23), n);
        t.set_u32(t.img.word(at + 0x1b), fin);
        t.write(task, &varied(0x20, 0x11));
        t.set_u32(task + 8, r);
        t.write(r, &varied(0x20, 0x22));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0341::bone_midpoint` (0x10001400).
#[test]
fn bone_midpoint_0341() {
    let Some(mut t) = twin(341) else { return };
    let at = t.at(0x1400);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(
                a[3] + 4 * i,
                (a[1] as f32 * 0.5 + i as f32 - 1.25).to_bits(),
            );
        }
        0x237
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ a[1];
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    t.set_u32(t.img.word(at + 0x29), 0x00e0_0029);
    t.set_u32(t.img.word(at + 0x44), 0x00e0_0044);
    let a = SCRATCH + 0x3000;
    t.write(a, &varied(0x100, 0x44));
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
}

/// `m0348::lights_models` (0x10006720) and `m0350::lights_models_parts`
/// (0x10005b70): no models and three (the record's node none).
#[test]
fn lights_models_0348_0350() {
    for (n, rva) in [(348u16, 0x6720u32), (350, 0x5b70)] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        recs(
            &mut t,
            &[(590, 2), (589, 2), (296, 2), (941, 1), (772, 2), (779, 2)],
        );
        let bytes = SCRATCH + 0x600;
        t.write(bytes, &[3, 5, 7, 9]);
        let cnt = SCRATCH + 0x10;
        rec(&mut t, 288, 2, move |m, a| {
            let c = m.r32(cnt);
            m.w32(a[1], c);
            bytes
        });
        let e = SCRATCH + 0x1000;
        t.set_u32(t.img.word(at + 0x6), e - t.img.word(at + 0xe));
        for c in [0u32, 3] {
            t.write(e, &varied(0x240, 0x5a));
            t.set_u32(e + 0x228, 0);
            t.set_u32(cnt, c);
            dirty(&mut t);
            t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
                if n == 348 {
                    ffx_magic::Ret::eax(ffx_magic::m0348::lights_models(g, img, fr))
                } else {
                    ffx_magic::m0350::lights_models_parts(g, img, fr)
                }
            });
        }
    }
}

/// The object start's set-up (`s4_misc::start_object_plain` at `s`).
fn plain_setup(t: &mut Twin, s: u32, count: u16) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let pa = t.img.rel32_target(s + 0x96);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
    t.set_u32(t.img.word(s + 0xa1), 0);
}

/// `m0351::reset_scale` (0x100014e0).
#[test]
fn reset_scale_0351() {
    let Some(mut t) = twin(351) else { return };
    let at = t.at(0x14e0);
    let mr = t.img.rel32_target(at + 0x1);
    recs(&mut t, &[(399, 2), (363, 3), (382, 2)]);
    t.set_u32(t.img.word(mr + 0x09), 0x00ab_0009);
    t.set_u32(t.img.word(mr + 0x36), 0x00dd_0036);
    for word in [0u32, 0x00cc_0001] {
        t.set_u32(t.img.word(mr + 0x2d), word);
        t.set_u32(t.img.word(at + 0x7), 0x5a5a_5a5a);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0352::fifth_start` (0x10004f00): the pool free and empty.
#[test]
fn fifth_start_0352() {
    let Some(mut t) = twin(352) else { return };
    let at = t.at(0x4f00);
    let s = t.img.rel32_target(at + 0x3c);
    rec(&mut t, 282, 0, |_, _| 0x282);
    t.stub_st0(282, |m, _| f64::from(m.r32(SCRATCH + 0x20)) * 0.125 + 0.3);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x12), blk);
    for (k, count) in [3u16, 0].into_iter().enumerate() {
        plain_setup(&mut t, s, count);
        t.set_u32(SCRATCH + 0x20, u32::try_from(k).unwrap() + 3);
        t.write(blk, &varied(0x800, 0x5a));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0361::flag_after_213` (0x10002920).
#[test]
fn flag_after_213_0361() {
    let Some(mut t) = twin(361) else { return };
    let at = t.at(0x2920);
    recs(&mut t, &[(213, 2)]);
    let m = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0x4), m);
    t.set_u32(m + 0x1a0, 0x00e0_01a0);
    let a = SCRATCH + 0x3000;
    t.set_u32(a, 0x8001_0010);
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
}

/// `m0371::records_start` (0x10001440) and `m0375::records_start`
/// (0x10001670): the records cleared.
#[test]
fn records_start_0371_0375() {
    for (n, rva, ops) in [
        (371u16, 0x1440u32, &[0x29u32, 0x33, 0x3d, 0x47][..]),
        (375, 0x1670, &[0x29, 0x33, 0x3d]),
    ] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        rec(&mut t, 719, 3, |_, a| a[0] ^ a[2] ^ 0x719);
        t.set_u32(t.img.word(at + 0x11), 0x00d0_0011);
        let rc = t.img.rel32_target(at + 0x20);
        let recs_at = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(rc + 0x7), recs_at);
        t.write(recs_at, &varied(0x1000, 0x5a));
        for &o in ops {
            t.set_u32(t.img.word(at + o), 0x7777_7777);
        }
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0353::record_words` (0x10002580): the pool free twice.
#[test]
fn record_words_0353() {
    let Some(mut t) = twin(353) else { return };
    let at = t.at(0x2580);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x3), blk);
    for k in 0..2u32 {
        pools(&mut t, at, 0x20, 2 + k);
        t.write(blk + 0x10e0, &varied(0x40, 0x5a));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0367::fourth_start` (0x100014a0): the pool free and empty.
#[test]
fn fourth_start_0367() {
    let Some(mut t) = twin(367) else { return };
    let at = t.at(0x14a0);
    let s = t.img.rel32_target(at + 0x13);
    for count in [3u16, 0] {
        plain_setup(&mut t, s, count);
        t.write(t.img.word(at + 0xc), &varied(0x130, 0x5a));
        t.write(t.img.word(at + 0x24), &varied(0x10, 0x24));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0377::parts_step` (0x10003ce0): the motion running, the objects
/// running, the wait word set, all done (the parts registered: none).
#[test]
fn parts_step_0377() {
    let Some(mut t) = twin(377) else { return };
    let at = t.at(0x3ce0);
    let ms = t.img.rel32_target(at + 0x5);
    let ro = t.img.rel32_target(at + 0xc);
    recs(&mut t, &[(941, 1), (772, 2), (779, 2)]);
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    for (done, running, wait) in [
        (false, false, 0u32),
        (true, true, 0),
        (true, false, 1),
        (true, false, 0),
    ] {
        magic_hi::motion(&mut t, ms, done);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(t.img.word(at + 0x17), wait);
        t.write(task, &varied(0x20, 0x11));
        t.set_u32(task + 8, r);
        t.write(r, &varied(0x20, 0x22));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0389::static_strip` (0x10006eb0): the static's parent (a DLL global)
/// with no child and with one.
#[test]
fn static_strip_0389() {
    let Some(mut t) = twin(389) else { return };
    let at = t.at(0x6eb0);
    let node = t.img.word(at + 0xe);
    let parent = t.img.word(at + 0x15);
    let kid = SCRATCH + 0x2800;
    for with_kid in [false, true] {
        t.write(node, &varied(0x80, 0x5c));
        t.write(parent, &varied(0x40, 0x3c));
        t.set_u16(parent + 0x18, 5);
        t.write(kid, &varied(0x40, 0x2c));
        t.set_u32(parent + 0x10, if with_kid { kid } else { parent });
        dirty(&mut t);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0389::static_strip,
        );
    }
}
