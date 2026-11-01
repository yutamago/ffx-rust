//! Small own functions below `magic_0450` (agent magic-small-lo), batch
//! 2: scale-task and object starts, a mean-point visitor. Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss, clippy::many_single_char_names)]

mod magic;
mod magic_hi;
mod magic_owna;

use ffx_magic::{Image, sa_pool};
use magic::{Call, Eax, SCRATCH, TRAPS, Twin};
use magic_hi::pool;
use magic_owna::{rec, twin};

/// Junk under the entry on both sides.
fn dirty(t: &mut Twin) {
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
}

/// The pool takes reached from `at` (`len` bytes, one call deep).
fn takes(img: &Image, at: u32, len: u32, depth: u32, out: &mut Vec<u32>) {
    for p in at..at + len {
        if img.word(p).to_le_bytes()[0] != 0xe8 {
            continue;
        }
        let t = img.rel32_target(p + 1);
        if !img.in_text(t) {
            continue;
        }
        if img.matches(t, &sa_pool::TAKE) {
            if !out.contains(&t) {
                out.push(t);
            }
        } else if depth > 0 {
            takes(img, t, 0x200, depth - 1, out);
        }
    }
}

/// Free rings of `n` nodes for every pool take reached from `at`.
fn pools(t: &mut Twin, at: u32, len: u32, n: u32) {
    let mut v = Vec::new();
    takes(&t.img, at, len, 1, &mut v);
    for (k, tk) in (0u32..).zip(v) {
        pool(t, tk, SCRATCH + 0x8000 + 0x400 * k, n);
    }
}

/// The task list of a scale task (`s4_misc::scale_task` at `s`): empty
/// or holding a node.
fn task_list(t: &mut Twin, head: u32, tail: u32, full: bool) {
    let node = SCRATCH + 0x7000;
    t.write(node, &[0x3a; 0x1c]);
    t.set_u32(head, if full { node } else { 0 });
    t.set_u32(tail, if full { node } else { 0 });
}

/// `m0162::scale_start` (0x10002e40) and `m0165::scale_start`
/// (0x100029b0): the scales from the DLL, the lists empty and not.
#[test]
fn scale_start_0162() {
    let Some(mut t) = twin(162) else { return };
    let at = t.at(0x2e40);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    rec(&mut t, 371, 2, |_, a| a[0] ^ a[1] ^ 0x371);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x13), s);
    for full in [false, true] {
        pools(&mut t, at, 202, 3);
        t.write(s + 0x8390, &[0x77; 0x20]);
        t.write(s + 0x8428, &[0x55; 0x40]);
        let (h, l) = (w(&t, 0x92), w(&t, 0x9a));
        task_list(&mut t, h, l, full);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn scale_start_0165() {
    let Some(mut t) = twin(165) else { return };
    let at = t.at(0x29b0);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    rec(&mut t, 371, 2, |_, a| a[0] ^ a[1] ^ 0x371);
    let b = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x2), b);
    for k in 0..2u32 {
        pools(&mut t, at, 79, 2 + k);
        t.write(b + 0x59f8, &[0x55; 0x40]);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// The object start's set-up (`s4_misc::start_object_plain` at `s`).
fn object_setup(t: &mut Twin, s: u32, count: u16, tail: u32) {
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
    t.set_u32(t.img.word(s + 0xa1), tail);
}

/// `m0162::object_start` (0x10002f10): a free pool and a tail, and the
/// pool empty.
#[test]
fn object_start_0162() {
    let Some(mut t) = twin(162) else { return };
    let at = t.at(0x2f10);
    let s = t.img.rel32_target(at + 0x1e);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0xb), blk);
    for (count, tail) in [(3u16, 0u32), (2, SCRATCH + 0x600), (0, 0)] {
        object_setup(&mut t, s, count, tail);
        t.write(blk + 0x130, &[0x5a; 0x130]);
        for i in 0..4u32 {
            t.set_u32(blk + 0x960 + 4 * i, (1.5 + i as f32).to_bits());
        }
        t.set_u32(blk + 0x85c8, 0x0085_c800 + u32::from(count));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0164::mean_visit` (0x10004900) through its thunk (0x100049a0) and
/// alone: visited, not visited with no object, with an object (the
/// flags bit 3 set: slot 234 alone).
#[test]
fn mean_visit_0164() {
    let Some(mut t) = twin(164) else { return };
    let at = t.at(0x4900);
    let thunk = t.at(0x49a0);
    let slot = 1000u32;
    t.set_u32(magic::TABLE + 4 * slot, TRAPS + 16 * slot);
    rec(&mut t, slot, 1, |m, a| {
        m.r32(a[0]) ^ m.r32(a[0] + 4) ^ 0x1000
    });
    rec(&mut t, 234, 3, |_, a| a[0] ^ a[1] ^ a[2]);
    let vt = SCRATCH + 0x900;
    t.set_u32(vt + 0x1c, TRAPS + 16 * slot);
    let (cur, node, kid, obj, out) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x1200,
        SCRATCH + 0x1400,
        SCRATCH + 0x1600,
    );
    for (k, (same, kids, has)) in [
        (true, false, true),
        (false, false, false),
        (false, true, true),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(node, &[0x22; 0x40]);
        t.set_u32(node, vt);
        t.set_u8(node + 0x1a, 5);
        t.set_u32(node + 0x10, if kids { kid } else { node });
        t.write(kid, &[0x44; 0x20]);
        t.set_u32(kid, vt);
        t.set_u32(kid + 8, node);
        t.set_u32(node + 0x2c, if has { obj } else { 0 });
        t.set_u32(node + 0x1c, out);
        t.write(obj, &[0x33; 0x1700]);
        t.set_u32(obj, vt);
        t.set_u8(obj + 0x1640, 8);
        t.set_u32(cur, node);
        t.set_u8(cur + 4, 0x7f);
        t.set_u8(cur + 5, if same { 5 } else { 6 });
        dirty(&mut t);
        let c = Call {
            eax: 0x1234_5678,
            ecx: cur,
            ..Call::default()
        };
        if k == 2 {
            t.check_entry(thunk, Call { ecx: 0, ..c }, &[cur], Eax::Checked);
        } else {
            t.check_entry(at, c, &[], Eax::Checked);
        }
    }
}

/// `m0165`'s object starts (0x10002db0, 0x10002f10, 0x100038d0): a free
/// pool and a tail, the pool empty; the record's list empty and not; the
/// angle above, inside and below its range.
#[test]
fn object_starts_0165() {
    let Some(mut t) = twin(165) else { return };
    rec(&mut t, 237, 4, |m, a| {
        m.w32(a[3], a[0] ^ 0x237);
        0x0237_0000 ^ a[1]
    });
    let blk = SCRATCH + 0x1_0000;
    for rva in [0x2db0u32, 0x2f10, 0x38d0] {
        let at = t.at(rva);
        let (sop, cop) = match rva {
            0x2db0 => (0xb, 0x27),
            0x2f10 => (0xc, 0x1c),
            _ => (0xd, 0x1f),
        };
        let s = t.img.rel32_target(at + cop);
        t.set_u32(t.img.word(at + sop), blk);
        for (k, (count, tail)) in [(3u16, 0u32), (2, SCRATCH + 0x600), (0, 0)]
            .into_iter()
            .enumerate()
        {
            let k = u32::try_from(k).unwrap();
            object_setup(&mut t, s, count, tail);
            t.write(blk, &[0x5a; 0x1000]);
            for i in 0..4u32 {
                t.set_u32(blk + 0x9f0 + 4 * i, (2.5 + i as f32).to_bits());
            }
            t.set_u32(blk + 0x5f1c, k + 1);
            t.write(blk + 0x5960, &[0x77; 0x20]);
            let angle = [7.0f32, 0.25, -7.0][k as usize];
            t.set_u32(blk + 0x5ee8, angle.to_bits());
            if rva == 0x2f10 {
                pools(&mut t, at, 206, 2);
                let (h, l) = (t.img.word(at + 0xc1), t.img.word(at + 0xb2));
                task_list(&mut t, h, l, k == 1);
            }
            dirty(&mut t);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `m0168::object_task` (0x10007b50): the list empty and holding a node.
#[test]
fn object_task_0168() {
    let Some(mut t) = twin(168) else { return };
    let at = t.at(0x7b50);
    let obj = SCRATCH + 0x1000;
    for full in [false, true] {
        pools(&mut t, at, 112, 2);
        let (h, l) = (t.img.word(at + 0x63), t.img.word(at + 0x54));
        task_list(&mut t, h, l, full);
        t.write(obj, &[0x5a; 0x130]);
        t.check(at, Call::this(obj), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::m0168::object_task(g, img, fr.at, fr.ecx))
        });
    }
}

/// `m0168::object_start` (0x10002f40): a free pool and a tail, the pool
/// empty.
#[test]
fn object_start_0168() {
    let Some(mut t) = twin(168) else { return };
    let at = t.at(0x2f40);
    let s = t.img.rel32_target(at + 0x20);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0xc), blk);
    for (count, tail) in [(3u16, 0u32), (2, SCRATCH + 0x600), (0, 0)] {
        anim_setup(&mut t, s, count, tail);
        t.write(blk, &[0x5a; 0x1000]);
        t.set_u32(blk + 0x3150, 0x0031_5000 + u32::from(count));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// The object start's set-up (`s4_misc::start_object_anim` at `s`).
fn anim_setup(t: &mut Twin, s: u32, count: u16, tail: u32) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let pa = t.img.rel32_target(s + 0x93);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
    t.set_u32(t.img.word(s + 0x9e), tail);
}

/// A motion script record at `sc` done or waiting.
fn script(t: &mut Twin, sc: u32, done: bool) {
    t.write(sc, &[0x5a; 0x20]);
    t.set_u32(sc + 0x14, u32::from(done));
    t.set_u16(sc + 0x18, 3);
}

/// `m0171::grid_step` (0x10001a80): the script waiting, the objects
/// running, the flag off, all done (the grid drawn).
#[test]
fn grid_step_0171() {
    let Some(mut t) = twin(171) else { return };
    let at = t.at(0x1a80);
    let ro = t.img.rel32_target(at + 0x11);
    let grid = t.img.rel32_target(at + 0x27);
    t.set_u32(t.img.word(grid + 0x53), SCRATCH + 0x50);
    t.set_u32(t.img.word(grid + 0x0f), SCRATCH + 0x1000);
    t.set_u32(t.img.word(grid + 0x08), 0);
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    for (done, running, flag) in [
        (false, false, 1u32),
        (true, true, 1),
        (true, false, 0),
        (true, false, 1),
    ] {
        let sc = t.img.word(at + 0x5);
        script(&mut t, sc, done);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(t.img.word(at + 0x20), flag);
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, r);
        t.write(r, &[0x22; 0x20]);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0180::square_step` (0x10004810): the motion running, the objects
/// running, the flag off, all done.
#[test]
fn square_step_0180() {
    let Some(mut t) = twin(180) else { return };
    let at = t.at(0x4810);
    let ms = t.img.rel32_target(at + 0x9);
    let ro = t.img.rel32_target(at + 0x10);
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    for (done, running, flag) in [
        (false, false, 1u32),
        (true, true, 1),
        (true, false, 0),
        (true, false, 1),
    ] {
        magic_hi::motion(&mut t, ms, done);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(t.img.word(at + 0x1f), flag);
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, r);
        t.write(r, &[0x22; 0x20]);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0180::main_start` (0x10004a80): the angle above, inside, below its
/// range; the pool free and empty.
#[test]
fn main_start_0180() {
    let Some(mut t) = twin(180) else { return };
    let at = t.at(0x4a80);
    let s = t.img.rel32_target(at + 0x1b);
    rec(&mut t, 325, 6, |_, a| a[0] ^ a[1] ^ 0x325);
    let obj = t.img.word(at + 0x16);
    for (k, angle) in [9.0f32, 0.5, -9.0].into_iter().enumerate() {
        let count = [3u16, 0, 2][k];
        let tail = if k == 2 { SCRATCH + 0x600 } else { 0 };
        at_setup(&mut t, s, count, tail);
        t.write(obj, &[0x5a; 0x130]);
        t.set_u32(t.img.word(at + 0x3e), angle.to_bits());
        for o in [
            0x81u32, 0x8b, 0x95, 0x9f, 0xa9, 0xb3, 0xbd, 0xc7, 0xd7, 0xe1,
        ] {
            t.set_u32(t.img.word(at + o), 0x7777_7777);
        }
        t.set_u32(t.img.word(at + 0xd1), 0x7777_7777);
        dirty(&mut t);
        let c = Call {
            ecx: 0x0ecc_0000 + u32::try_from(k).unwrap(),
            ..Call::default()
        };
        t.check_entry(at, c, &[SCRATCH + 0x3000], Eax::Checked);
    }
}

/// The object start's set-up (`s4_misc::start_object_at` at `s`).
fn at_setup(t: &mut Twin, s: u32, count: u16, tail: u32) {
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
    t.set_u32(t.img.word(s + 0xa2), tail);
}

/// `m0184::scatter` (0x10001280): a free record and none.
#[test]
fn scatter_0184() {
    let Some(mut t) = twin(184) else { return };
    let at = t.at(0x1280);
    let ff = t.img.rel32_target(at + 0x22);
    let start = t.img.word(ff + 0x3);
    let end = t.img.word(ff + 0x12);
    rec(&mut t, 241, 4, |m, a| {
        let k = m.r32(SCRATCH + 0x80) + 1;
        m.w32(SCRATCH + 0x80, k);
        m.w32(a[0], (0.37 * k as f32 - 0.5).to_bits());
        0x241
    });
    let base = SCRATCH + 0x100;
    for (i, v) in [1.5f32, -2.25, 3.0].into_iter().enumerate() {
        t.set_u32(base + 4 * u32::try_from(i).unwrap(), v.to_bits());
    }
    for free in [true, false] {
        let mut p = start;
        while p < end {
            t.set_u16(p, 0);
            p += 0x18;
        }
        if free {
            t.set_u16(start + 0x18 * 2, 0xffff);
        }
        dirty(&mut t);
        let c = Call::this(base).with_xmm(&[
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(0.75),
            ffx_magic::Xmm::f32(-1.25),
        ]);
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::m0184::scatter(g, img, fr)
        });
    }
}

/// `m0186::model_node` (0x10007700) and `cleared_node` (0x10007750):
/// the parent (a DLL global) with no child and with one.
#[test]
fn nodes_0186() {
    let Some(mut t) = twin(186) else { return };
    rec(&mut t, 236, 3, |m, a| {
        for i in 0..16 {
            m.w32(a[2] + 4 * i, a[0].rotate_left(i) ^ a[1]);
        }
        0x236
    });
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ a[2];
            m.w32(a[0] + 4 * i, v);
        }
        0x233
    });
    rec(&mut t, 325, 2, |_, a| 0x0325_0000 ^ a[0]);
    let node = SCRATCH + 0x1000;
    let kid = SCRATCH + 0x2800;
    let (mn, cn) = (t.at(0x7700), t.at(0x7750));
    for (at, pop) in [(mn, 0x24u32), (cn, 0x12)] {
        let parent = t.img.word(at + pop);
        for with_kid in [false, true] {
            t.write(node, &[0x5c; 0x200]);
            t.write(kid, &[0x2c; 0x40]);
            t.set_u32(parent + 0x10, if with_kid { kid } else { parent });
            t.set_u16(parent + 0x18, 5);
            dirty(&mut t);
            let c = Call {
                eax: 0x00ea_0000,
                ecx: node,
                ..Call::default()
            };
            let args = [0x00a0_0008, 0x00b0_000c, 0x00c0_0010, 0x00d0_0014];
            if at == mn {
                t.check(at, c, &args, Eax::Checked, |g, img, fr| {
                    ffx_magic::m0186::model_node(g, img, fr)
                });
            } else {
                t.check(at, c, &args, Eax::Checked, |g, img, fr| {
                    ffx_magic::Ret::eax(ffx_magic::m0186::cleared_node(
                        g, img, fr.at, fr.ecx, fr.esp, fr.eax,
                    ))
                });
            }
        }
    }
}
