//! Small own functions below `magic_0450` (agent magic-small-lo), batch
//! 4: `magic_0380`-`0445`. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss, clippy::many_single_char_names)]

mod magic;
mod magic_hi;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, STACK_TOP, TRAPS, Twin};
use magic_owna::{rec, twin};

/// Junk under the entry on both sides.
fn dirty(t: &mut Twin) {
    t.write(STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
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

/// A start's set-up found by its callees: the part lock and block, slot
/// 941's table and slot 714, and `count` free nodes in the pool of the
/// first `l5_objhost::pool_alloc` called from `s`; the lock words at
/// `+0x04`/`+0x5e`.
fn obj_setup(t: &mut Twin, s: u32, count: u16) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let calls: Vec<u32> = (0..0x100)
        .map(|o| s + o)
        .filter(|&p| t.img.word(p).to_le_bytes()[0] == 0xe8)
        .map(|p| t.img.rel32_target(p + 1))
        .filter(|&c| t.img.in_text(c))
        .collect();
    for c in calls {
        if t.img.matches(c, &ffx_magic::l5_objhost::POOL_ALLOC) {
            let nodes = t.img.word(c + 0x27) - 8;
            for i in 0..8 {
                t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
                t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
                t.set_u32(nodes + 12 * i + 8, 0);
            }
            t.set_u16(t.img.word(c + 0x04), count);
            return;
        }
    }
}

/// A motion script record at `sc` done or waiting.
fn script(t: &mut Twin, sc: u32, done: bool) {
    t.write(sc, &[0x5a; 0x20]);
    t.set_u32(sc + 0x14, u32::from(done));
    t.set_u16(sc + 0x18, 3);
}

/// `m0380::scripts_step` (0x10001340) and `m0381::scripts_step`
/// (0x100013e0): a script waiting (each), the objects running, all done.
#[test]
fn scripts_steps_0380_0381() {
    for (n, rva) in [(380u16, 0x1340u32), (381, 0x13e0)] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        let ro = t.img.rel32_target(at + 0x1e);
        let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
        for (a, b, running) in [
            (false, true, false),
            (true, false, false),
            (true, true, true),
            (true, true, false),
        ] {
            let (sa, sb) = (t.img.word(at + 0x6), t.img.word(at + 0x10));
            script(&mut t, sa, a);
            script(&mut t, sb, b);
            magic_hi::objects(&mut t, ro, running);
            if n == 380 {
                t.set_u32(t.img.word(at + 0x37), 0xffff_ffff);
                t.set_u32(t.img.word(at + 0x43), 0xffff_ffff);
            }
            t.write(task, &[0x11; 0x20]);
            t.set_u32(task + 8, r);
            t.write(r, &[0x22; 0x20]);
            dirty(&mut t);
            t.check_entry(at, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// `m0393::fifth_start` (0x10002fd0), `ninth_start` (0x100023b0) and
/// `m0432::object_start` (0x10001da0): the pool free and empty.
#[test]
fn object_starts_0393_0432() {
    for (n, rva, sop, cop) in [
        (393u16, 0x2fd0u32, 0xbu32, 0x1eu32),
        (393, 0x23b0, 0xc, 0x20),
        (432, 0x1da0, 0xb, 0x19),
    ] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        let s = t.img.rel32_target(at + cop);
        let blk = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(at + sop), blk);
        for count in [3u16, 0] {
            obj_setup(&mut t, s, count);
            t.write(blk, &[0x5a; 0x3400]);
            dirty(&mut t);
            t.check_entry(at, Call::this(0x0ecc_0005), &[], Eax::Checked);
        }
    }
}

/// `m0430::scale_e3c` (0x10002280): the pool free twice.
#[test]
fn scale_e3c_0430() {
    let Some(mut t) = twin(430) else { return };
    let at = t.at(0x2280);
    rec(&mut t, 371, 2, |_, a| a[0] ^ a[1] ^ 0x371);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0xe), blk);
    for k in 0..2u32 {
        pools(&mut t, at, 0x58, 2 + k);
        t.write(blk + 0xe00, &[0x55; 0x80]);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0432::point_below` (0x10001d30).
#[test]
fn point_below_0432() {
    let Some(mut t) = twin(432) else { return };
    let at = t.at(0x1d30);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ a[1];
            m.w32(a[3] + 4 * i, v);
        }
        0x237
    });
    t.set_u32(t.img.word(at + 0x2b), 0x00e0_002b);
    let a = SCRATCH + 0x3000;
    t.write(a, &[0x44; 0x100]);
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
}

/// `m0434::second_start` (0x10009330): the pool free and empty.
#[test]
fn second_start_0434() {
    let Some(mut t) = twin(434) else { return };
    let at = t.at(0x9330);
    let s = t.img.rel32_target(at + 0x3e);
    rec(&mut t, 282, 0, |_, _| 0x282);
    t.stub_st0(282, |m, _| f64::from(m.r32(SCRATCH + 0x20)) * 0.125 + 0.3);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x12), blk);
    for (k, count) in [3u16, 0].into_iter().enumerate() {
        obj_setup(&mut t, s, count);
        t.set_u32(SCRATCH + 0x20, u32::try_from(k).unwrap() + 3);
        t.write(blk, &[0x5a; 0x800]);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0396::object_or_fade` (0x10005170): the flag clear (index 7) and set
/// (the fade task, index 8).
#[test]
fn object_or_fade_0396() {
    let Some(mut t) = twin(396) else { return };
    let at = t.at(0x5170);
    let s = t.img.rel32_target(at + 0x62);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0xc), blk);
    for (flag, count) in [(0u32, 3u16), (1, 0), (1, 3)] {
        obj_setup(&mut t, s, count);
        pools(&mut t, at, 0x40, 3);
        t.set_u32(t.img.word(at + 0x2), flag);
        t.write(blk, &[0x5a; 0x2500]);
        dirty(&mut t);
        t.check_entry(at, Call::this(0x0ecc_0006), &[], Eax::Checked);
    }
}

/// `m0445::fade_visit` (0x100034c0) and its thunk (0x100035b0): visited;
/// no count (the flag clear, set: the node leaves); a count with the
/// level below and at `K` (the never-written colour words 0 on both
/// sides).
#[test]
fn fade_visit_0445() {
    let Some(mut t) = twin(445) else { return };
    let at = t.at(0x34c0);
    let thunk = t.at(0x35b0);
    let slot = 1000u32;
    t.set_u32(magic::TABLE + 4 * slot, TRAPS + 16 * slot);
    rec(&mut t, slot, 1, |m, a| m.r32(a[0]) ^ 0x1000);
    rec(&mut t, 252, 4, |_, a| a[0] ^ a[1] ^ a[2] ^ 0x252);
    rec(&mut t, 234, 3, |_, a| a[0] ^ 0x234);
    let k = f32::from_bits(t.u32(t.img.word(at + 0x99)));
    t.set_u32(t.img.word(at + 0x82), 0x00e0_0082);
    let vt = SCRATCH + 0x900;
    t.set_u32(vt + 0x1c, TRAPS + 16 * slot);
    let (cur, node, parent) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x1400);
    for (i, (same, count, flag, level)) in [
        (true, 0u32, 0u8, 0.0f32),
        (false, 0, 0, 0.0),
        (false, 0, 1, 0.0),
        (false, 2, 0, k - 1.0),
        (false, 2, 0, k),
        (false, 1, 1, f32::NAN),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(node, &[0x22; 0x60]);
        t.set_u32(node, vt);
        t.set_u8(node + 0x1a, 5);
        t.set_u32(node + 0x10, node);
        t.set_u32(node + 0x14, parent);
        t.set_u32(node + 4, parent);
        t.set_u32(node + 8, parent);
        t.write(parent, &[0x33; 0x20]);
        t.set_u16(parent + 0x18, 2);
        t.set_u32(node + 0x54, count);
        t.set_u8(node + 0x58, flag);
        t.set_u32(node + 0x4c, level.to_bits());
        t.set_u32(cur, node);
        t.set_u8(cur + 4, 0x7f);
        t.set_u8(cur + 5, if same { 5 } else { 6 });
        dirty(&mut t);
        // the colour words and the alpha word's upper bytes, never
        // written by the original (the port reads 0 there)
        let thunked = i == 5;
        // the thunk pops its frame before the jump: the same `ebp` either way
        let ebp = magic::STACK_TOP - 0x100 - 4;
        t.write(ebp - 0x14, &[0; 0x14]);
        let c = Call {
            eax: 0x1234_5678,
            ecx: cur,
            ..Call::default()
        };
        if thunked {
            t.check_entry(thunk, Call { ecx: 0, ..c }, &[cur], Eax::Checked);
        } else {
            t.check_entry(at, c, &[], Eax::Checked);
        }
    }
}

/// `m0367::pair_start` (0x10001410): the pool free and empty.
#[test]
fn pair_start_0367() {
    let Some(mut t) = twin(367) else { return };
    let at = t.at(0x1410);
    let s = t.img.rel32_target(at + 0x13);
    for count in [3u16, 1, 0] {
        obj_setup(&mut t, s, count);
        t.write(t.img.word(at + 0xc), &[0x5a; 0x130]);
        t.write(t.img.word(at + 0x23), &[0x5a; 0x130]);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0354::countdown_step` (0x10004710): the count 0, 3, 1 (the block
/// and slot 537), and the steps' outcomes.
#[test]
fn countdown_step_0354() {
    let Some(mut t) = twin(354) else { return };
    let at = t.at(0x4710);
    let ro = t.img.rel32_target(at + 0x5d);
    let mb = t.img.rel32_target(at + 0x1a);
    recs(
        &mut t,
        &[
            (537, 7),
            (941, 1),
            (772, 2),
            (779, 2),
            (262, 1),
            (6, 1),
            (548, 1),
            (413, 1),
            (215, 1),
        ],
    );
    let _ = mb;
    let byte = SCRATCH + 0x600;
    t.set_u32(t.img.word(at + 0x1f), byte);
    t.write(byte, &[4, 0x40]);
    t.set_u32(t.img.word(at + 0x3c), 0x00e0_003c);
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    for (count, d, done, running) in [
        (0u32, 1u32, true, false),
        (3, 1, true, false),
        (1, 0, true, false),
        (0, 0, true, false),
        (0, 1, false, false),
        (0, 1, true, true),
        (0xffff_fff0, 1, true, false),
    ] {
        t.set_u32(t.img.word(at + 0x4), count);
        t.set_u32(t.img.word(at + 0x47), d);
        let sc = t.img.word(at + 0x51);
        t.write(sc, &[0x5a; 0x20]);
        t.set_u32(sc + 0x14, u32::from(done));
        t.set_u16(sc + 0x18, 3);
        magic_hi::objects(&mut t, ro, running);
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, r);
        t.write(r, &[0x22; 0x20]);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0393::ninth_scaled` (0x10003cd0) and `m0397::object_fade`
/// (0x10004790): the pools free and empty.
#[test]
fn starts_with_tasks_0393_0397() {
    for (n, rva, sop, cop) in [
        (393u16, 0x3cd0u32, 0xcu32, 0x50u32),
        (397, 0x4790, 0xb, 0x1a),
    ] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        let s = t.img.rel32_target(at + cop);
        let blk = SCRATCH + 0x1_0000;
        t.set_u32(t.img.word(at + sop), blk);
        for (k, count) in [3u16, 0].into_iter().enumerate() {
            obj_setup(&mut t, s, count);
            pools(&mut t, at, 0xb0, 2 + u32::try_from(k).unwrap());
            t.write(blk, &[0x5a; 0x2500]);
            dirty(&mut t);
            t.check_entry(at, Call::this(0x0ecc_0009), &[], Eax::Checked);
        }
    }
}
