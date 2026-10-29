//! `magic_0219.dll` against its Rust port: the object host
//! (`ffx_magic::l5_objhost`: start, end, the draw and update callbacks,
//! the task steps, the object list and the object scripts) and the DLL's
//! own per-frame entry with its start-up (`ffx_magic::m0219`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

#[path = "magic/l5.rs"]
mod l5;
mod magic;

use magic::{Call, Eax, SCRATCH, StubMem, TRAPS, Twin};

const N: u16 = 219;

/// Cells the stubs read, so a test can steer them.
const CFG: u32 = SCRATCH + 0x100;
const CFG_ALT: u32 = CFG;
const CFG_N: u32 = CFG + 4;
const CFG_225: u32 = CFG + 8;
const CFG_227: u32 = CFG + 0xc;
const CFG_712: u32 = CFG + 0x10;
const CFG_408: u32 = CFG + 0x14;
/// The byte list slot 218 hands out.
const BYTES: u32 = SCRATCH + 0x2000;
const SCRIPT: u32 = SCRATCH + 0x5000;
const CMDS: u32 = SCRATCH + 0x5800;
const PART: u32 = SCRATCH + 0x6000;

/// Records the first `n` words of each call of `slot` into its own row
/// (a count, then the words) and answers `ret`.
fn rec(t: &mut Twin, slot: u32, n: usize, ret: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static) {
    let row = SCRATCH + 0x2_0000 + 0x40 * (slot % 0x400);
    t.stub(slot, move |m, a| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        for (i, w) in (0u32..).zip(&a[..n]) {
            m.w32(row + 4 + 4 * i, *w ^ k);
        }
        ret(m, a)
    });
}

fn stubs(t: &mut Twin) {
    let be0c = t.img.word(t.at(0x14b0) + 0x0a);
    rec(t, 168, 1, move |m, _| {
        if m.r32(CFG_ALT) != 0 {
            m.w32(be0c, 1);
        }
        SCRATCH + 0x1000
    });
    rec(t, 215, 1, |_, a| 0x0215_0000 + a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 + a[0]);
    rec(t, 218, 2, |m, a| {
        let n = m.r32(CFG_N);
        m.w32(a[1], n);
        BYTES
    });
    rec(t, 291, 5, |_, a| 0x10 + a[1]);
    rec(t, 367, 8, |_, a| 0x0367_0000 + a[1]);
    for (s, n) in [
        (556, 1),
        (555, 1),
        (331, 1),
        (720, 2),
        (718, 1),
        (222, 4),
        (221, 4),
        (790, 0),
    ] {
        rec(t, s, n, |_, _| 0);
    }
    t.stub_st0(790, |_, _| 0.1);
    for (s, n) in [
        (353, 1),
        (397, 1),
        (332, 1),
        (267, 1),
        (933, 4),
        (785, 1),
        (543, 1),
        (250, 2),
        (226, 2),
        (711, 3),
        (710, 4),
        (714, 5),
        (262, 4),
        (268, 3),
    ] {
        rec(t, s, n, |_, a| a[0] ^ 0x5a5a);
    }
    rec(t, 225, 1, |m, _| m.r32(CFG_225));
    rec(t, 227, 1, |m, _| m.r32(CFG_227));
    rec(
        t,
        719,
        3,
        |_, a| {
            if a[1] == 8 { PART } else { 0x0719_0000 + a[2] }
        },
    );
    rec(t, 941, 1, |_, _| PART + 0x100);
    // the frame
    for (s, n) in [
        (723, 1),
        (724, 1),
        (552, 2),
        (231, 3),
        (160, 3),
        (713, 1),
        (214, 1),
        (239, 3),
        (220, 3),
        (393, 3),
        (900, 1),
    ] {
        rec(t, s, n, |_, a| a[0] ^ 0x1234);
    }
    let draw = t.at(0x1290);
    let (fx, fy) = (t.img.word(draw + 0x3b), t.img.word(draw + 0x1f));
    rec(t, 230, 2, move |m, a| {
        let row = SCRATCH + 0x3_0000 + 0x40 * (m.r32(SCRATCH + 0x3_fff0) % 0x40);
        m.w32(SCRATCH + 0x3_fff0, m.r32(SCRATCH + 0x3_fff0) + 1);
        if a[1] >= 0x1000_0000 {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(row + 4 * i, v);
            }
        }
        m.w32(fx, 0x3f80_0000 + a[0]);
        m.w32(fy, 0x4000_0000 ^ a[0]);
        0
    });
    // matrices and vectors the slots fill
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x3f00_0000 + 0x1_0000 * i);
        }
        0
    });
    rec(t, 554, 3, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x4000_0000 + 0x3_0000 * i);
        }
        0
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v ^ (a[2] & 0x00ff_0000) ^ (s << 4));
            }
            0
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x3e80_0000 + 0x2_0000 * i);
        }
        0
    });
    rec(t, 712, 2, |m, _| m.r32(CFG_712));
    rec(
        t,
        408,
        3,
        |m, a| {
            if m.r32(CFG_408) != 0 { 0 } else { a[2] & 1 }
        },
    );
}

fn start(t: &mut Twin, n: u32, alt: u32) {
    // every pool-A payload busy before the start clears them
    let payloads = t.img.word(t.at(0x1000) + 0x81);
    t.write(payloads, &vec![0xa5; 12 * 0x3ff]);
    t.set_u32(CFG_N, n);
    t.set_u32(CFG_ALT, alt);
    for (i, b) in [3u8, 1, 4, 1, 5, 9, 2, 6].iter().enumerate() {
        t.set_u8(BYTES + i as u32, *b);
    }
    t.check_entry(
        t.at(0x1000),
        Call {
            eax: 0,
            ecx: 0x55,
            edx: 0,
        },
        &[0x00ab_c000],
        Eax::Checked,
    );
}

#[test]
fn start_and_end() {
    let Some(mut t) = l5::twin(N) else { return };
    stubs(&mut t);
    for (n, alt) in [(0, 0), (3, 0), (2, 1), (8, 0)] {
        start(&mut t, n, alt);
        t.check_entry(t.at(0x1120), Call::default(), &[], Eax::Checked);
    }
    t.check_entry(t.at(0x1300), Call::default(), &[1, 2], Eax::Checked);
}

/// Started, then the per-frame entry until the effect runs.
fn booted() -> Option<Twin> {
    let mut t = l5::twin(N)?;
    stubs(&mut t);
    start(&mut t, 3, 0);
    let pf = t.at(0x1210);
    for (s225, s227) in [(1, 1), (0, 0), (0, 0), (0, 1)] {
        t.set_u32(CFG_225, s225);
        t.set_u32(CFG_227, s227);
        t.check_entry(pf, Call::default(), &[], Eax::Checked);
    }
    Some(t)
}

/// The effect's object.
fn obj(t: &Twin) -> u32 {
    t.img.word(t.at(0x1370) + 0x44)
}

fn script(t: &mut Twin, cmds: &[(u32, u32, i16, i16)]) {
    let o = obj(t);
    t.set_u32(o + 0x110, SCRIPT);
    t.set_u32(SCRIPT + 0x20, 0x0123);
    t.set_u32(SCRIPT + 0x24, CMDS);
    t.set_u32(SCRIPT + 0x28, 0);
    t.set_u32(SCRIPT + 0x2c, 0);
    for (i, &(op, v, a, b)) in (0u32..).zip(cmds) {
        let c = CMDS + 12 * i;
        t.set_u32(c, op);
        t.set_u32(c + 4, v);
        t.set_u16(c + 8, a.cast_unsigned());
        t.set_u16(c + 0xa, b.cast_unsigned());
    }
}

fn update(t: &mut Twin) {
    t.check_entry(
        t.at(0x12e0),
        Call {
            eax: 0,
            ecx: 0x99,
            edx: 0,
        },
        &[],
        Eax::Checked,
    );
}

fn draw(t: &mut Twin) {
    t.check_entry(t.at(0x1290), Call::default(), &[], Eax::Checked);
}

#[test]
fn frames_modes_and_script() {
    let Some(mut t) = booted() else { return };
    let o = obj(&t);
    script(
        &mut t,
        &[
            (2, 0x41, 3, 5),
            (0, 0x42, -2, 7),
            (1, 0x43, 1, -1),
            (4, 0, 0, 0),
            (9, 0, 0, 0),
            (3, 1, 0, 0),
            (2, 0x44, 0, 2),
            (5, 0, 0, 0),
        ],
    );
    // mode, flags, delay, frame, before-callback, done-callback, texture
    let cases: [(u8, u16, u32, u32, u32, u32, u32); 5] = [
        (0, 0x8000, 0, 0, 0, 0, 0),
        (
            1,
            0x8000,
            0,
            0xf,
            TRAPS + 16 * 900,
            SCRATCH + 0x7000,
            0x0007_e000,
        ),
        (2, 0x8000, 0, 3, 0, 0, 0),
        (0, 0x8000, 2, 3, 0, 0, 0),
        (7, 0x8001, 0, 3, 0, 0, 0),
    ];
    // a second object after the first (mode 2, flagged): the walk's
    // current node moves past each object that stays
    let list = t.img.word(t.at(0x1c20) + 0x7c);
    let o2 = SCRATCH + 0xa000;
    let n2 = SCRATCH + 0x9f00;
    let n1 = t.u32(list);
    t.set_u32(n1 + 4, n2);
    t.set_u32(n2, n1);
    t.set_u32(n2 + 4, 0);
    t.set_u32(n2 + 8, o2);
    t.set_u32(list + 4, n2);
    t.set_u32(list + 0xc, 2);
    t.set_u16(o2 + 0x124, 0x8000);
    t.set_u8(o2 + 0x126, 2);
    t.set_u32(o2 + 0x11c, 0xf);
    for (mode, flags, delay, frame, before, done, tex) in cases {
        t.set_u8(o + 0x126, mode);
        t.set_u16(o + 0x124, flags);
        t.set_u32(o + 0x114, delay);
        t.set_u32(o + 0x118, frame);
        t.set_u32(o + 0x104, before);
        t.set_u32(o + 0x10c, done);
        t.set_u32(o + 0x108, tex);
        for i in 0..12 {
            t.set_u32(o + 0x80 + 4 * i, 0x4040_0000 + 0x1_1000 * i);
        }
        for (i, v) in (0u32..).zip([0x3fc0_0000u32, 0xbf40_0000, 0x4110_0000]) {
            t.set_u32(o + 0x60 + 4 * i, v);
            t.set_u32(o + 0xd0 + 4 * i, v ^ 0x0080_0000);
            t.set_u32(o + 0xc0 + 4 * i, v ^ 0x0040_0000);
        }
        update(&mut t);
    }
    // the task in group 1: the draw callback runs it
    let task = t.u32(t.img.word(t.at(0x21a0) + 0x01));
    t.set_u8(task + 0x18, 1);
    t.set_u16(o + 0x124, 0x8000);
    draw(&mut t);
    // the script's list emptied: the object's script flag 0x4000
    t.set_u32(CFG_408, 1);
    draw(&mut t);
    // nothing runs: the task's countdown takes over
    t.set_u16(o + 0x124, 0);
    t.set_u16(o2 + 0x124, 0);
    for _ in 0..3 {
        draw(&mut t);
    }
}

/// An object removed (flag 0x4000, or the part done): the task's
/// countdown follows, then slot 214. (One `Twin` per test: a second load
/// on the same thread clears the heap its Unicorn thread block is in.)
fn removal(flags: u16, done: u32) {
    let Some(mut t) = booted() else { return };
    let o = obj(&t);
    t.set_u16(o + 0x124, flags);
    t.set_u32(CFG_712, done);
    for _ in 0..6 {
        update(&mut t);
    }
}

#[test]
fn removal_by_flag_and_countdown() {
    removal(0xc000, 0);
}

#[test]
fn removal_when_the_part_is_done() {
    removal(0x8000, 1);
}

#[test]
fn dll_main_writes_only_crt_state() {
    let Some(mut t) = Twin::load_unstarted(N) else {
        return;
    };
    t.dll_main_check(
        &[
            0x1000, 0x1120, 0x1210, 0x1290, 0x12e0, 0x1300, 0x1310, 0x1340, 0x2130,
        ],
        &[
            (0x1370, 319),
            (0x14b0, 117),
            (0x1530, 436),
            (0x16f0, 253),
            (0x17f0, 66),
            (0x1840, 166),
            (0x18f0, 12),
            (0x1900, 113),
            (0x1980, 131),
            (0x1a10, 273),
            (0x1b30, 231),
            // run_objects past its cookie load (+0x9: the CRT's cookie, which
            // only __security_check_cookie reads; the port has no check)
            (0x1c2e, 1289 - 0xe),
            (0x2150, 80),
            (0x21a0, 87),
            (0x2200, 124),
        ],
    );
}

/// The list primitives on crafted lists: `list_remove` of the head, the
/// tail, a middle node, a missing item; `list_pop_tail` of one node and
/// of an empty list (pool-A nodes in scratch).
#[test]
fn list_primitives() {
    let Some(mut t) = l5::twin(N) else { return };
    let rm = t.at(0x1840);
    let pop = t.at(0x17f0);
    let free = t.img.word(rm + 0x4d);
    let list = SCRATCH + 0x8000;
    let node = |i: u32| SCRATCH + 0x8100 + 12 * i;
    // a list of n nodes holding items 0x100 + i, the current at `cur`
    let make = |t: &mut Twin, n: u32, cur: u32| {
        for i in 0..n {
            let prev = if i == 0 { 0 } else { node(i - 1) };
            let next = if i + 1 == n { 0 } else { node(i + 1) };
            t.set_u32(node(i), prev);
            t.set_u32(node(i) + 4, next);
            t.set_u32(node(i) + 8, 0x100 + i);
        }
        t.set_u32(list, if n == 0 { 0 } else { node(0) });
        t.set_u32(list + 4, if n == 0 { 0 } else { node(n - 1) });
        t.set_u32(list + 8, if cur < n { node(cur) } else { 0 });
        t.set_u32(list + 0xc, n);
        t.set_u16(free, 7);
    };
    for (n, cur, item) in [
        (4u32, 0u32, 0x100u32),
        (4, 1, 0x100),
        (1, 0, 0x100),
        (4, 3, 0x103),
        (4, 2, 0x102),
        (4, 0, 0x102),
        (4, 0, 0x999),
        (0, 9, 0x100),
    ] {
        make(&mut t, n, cur);
        let c = Call {
            eax: 0,
            ecx: list,
            edx: item,
        };
        t.check(rm, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::l5_objhost::list_remove(
                g, img, fr.at, fr.ecx, fr.edx,
            ))
        });
    }
    for (n, cur) in [(1u32, 0u32), (3, 2), (3, 0), (0, 0)] {
        make(&mut t, n, cur);
        let c = Call {
            eax: 0,
            ecx: list,
            edx: 0,
        };
        t.check(pop, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::l5_objhost::list_pop_tail(g, img, fr.at, fr.ecx))
        });
    }
    // list_cur
    make(&mut t, 2, 1);
    for l in [list, SCRATCH + 0x9000] {
        let c = Call {
            eax: 0,
            ecx: l,
            edx: 0,
        };
        t.check(t.at(0x18f0), c, &[], Eax::Checked, |g, _, fr| {
            ffx_magic::Ret::eax(ffx_magic::l5_objhost::list_cur(g, fr.ecx))
        });
    }
}

/// `pool_alloc` with busy nodes from the rover on and with none free;
/// `task_alloc` with 2, 1 and 0 free.
#[test]
fn pools() {
    let Some(mut t) = l5::twin(N) else { return };
    let pa = t.at(0x1900);
    let free = t.img.word(pa + 0x04);
    let rover = t.img.word(pa + 0x16);
    let payloads = t.img.word(pa + 0x27);
    for (count, r, busy) in [(5u16, 0u16, 0u32), (5, 3, 4), (5, 0x3fd, 3), (0, 0, 0)] {
        t.write(payloads, &vec![0; 12 * 0x3ff]);
        for k in 0..busy {
            let i = (u32::from(r) + 2 * k) % 0x3ff;
            t.set_u32(payloads + 12 * i, 1);
        }
        t.set_u16(free, count);
        t.set_u16(rover, r);
        t.check(pa, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::l5_objhost::pool_alloc(g, img, fr.at))
        });
    }
    let init = t.at(0x2150);
    t.check(init, Call::default(), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(ffx_magic::l5_objhost::tasks_init(g, img, fr.at))
    });
    let ta = t.at(0x2200);
    let tfree = t.img.word(ta + 0x04);
    for n in [2u16, 1, 0] {
        t.set_u16(tfree, n);
        t.check(ta, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::l5_objhost::task_alloc(g, img, fr.at))
        });
    }
}

/// `run_group` over crafted tasks whose steps are game stubs, one of
/// them moving the walk's current node (as a step that unlinks itself
/// does); the steps' and the task step's results.
#[test]
fn tasks_and_steps() {
    let Some(mut t) = l5::twin(N) else { return };
    stubs(&mut t);
    let rg = t.at(0x21a0);
    let head = t.img.word(rg + 0x01);
    let cur = t.img.word(rg + 0x0d);
    let task = |i: u32| SCRATCH + 0x8400 + 0x20 * i;
    // step 950 records, step 951 also skips the next task
    rec(&mut t, 950, 1, |_, a| a[0]);
    rec(&mut t, 951, 1, move |m, a| {
        let next = m.r32(a[0] + 4);
        let after = if next == 0 { 0 } else { m.r32(next + 4) };
        m.w32(cur, after);
        0
    });
    let owner = SCRATCH + 0x8800;
    let owner2 = SCRATCH + 0x8840;
    t.set_u32(owner + 0x10, TRAPS + 16 * 950);
    t.set_u32(owner2 + 0x10, TRAPS + 16 * 951);
    for i in 0..5u32 {
        t.set_u32(task(i), if i == 0 { 0 } else { task(i - 1) });
        t.set_u32(task(i) + 4, if i == 4 { 0 } else { task(i + 1) });
        t.set_u32(task(i) + 8, if i == 1 { owner2 } else { owner });
        t.set_u8(task(i) + 0x18, [0u8, 0, 0, 1, 2][i as usize]);
    }
    for (h, group) in [(task(0), 0u8), (task(0), 1), (task(0), 2), (0, 0)] {
        t.set_u32(head, h);
        let c = Call {
            eax: 0,
            ecx: u32::from(group),
            edx: 0,
        };
        t.check(rg, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::l5_objhost::run_group(
                g,
                img,
                fr.at,
                fr.esp,
                fr.ecx as u8,
            ))
        });
    }
    // the countdown and the task step called directly (their results)
    let tk = task(0);
    t.set_u32(tk + 8, owner);
    for n in [3u32, 1, 0, 0xffff_ffff] {
        t.set_u32(tk + 0x14, n);
        t.check_entry(t.at(0x1310), Call::default(), &[tk], Eax::Checked);
    }
    // the task step with an empty object list (done) and with an object
    t.check_entry(t.at(0x1340), Call::default(), &[tk], Eax::Checked);
}

/// `obj_block_init` called directly: its result (the id bytes, or 0).
#[test]
fn object_block() {
    let Some(mut t) = l5::twin(N) else { return };
    stubs(&mut t);
    for (n, alt) in [(0u32, 0u32), (2, 1), (5, 0)] {
        t.set_u32(CFG_N, n);
        t.set_u32(CFG_ALT, alt);
        let c = Call {
            eax: 0,
            ecx: 0x00ab_c000,
            edx: 0,
        };
        t.check(t.at(0x14b0), c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::l5_objhost::obj_block_init(g, img, fr.at, fr.ecx))
        });
    }
}

/// `obj_script` called directly: the list offered and emptied, then the
/// object flagged 0x8000 and 0x4000.
#[test]
fn object_script() {
    let Some(mut t) = l5::twin(N) else { return };
    stubs(&mut t);
    // pool A: all free
    let payloads = t.img.word(t.at(0x1900) + 0x27);
    t.write(payloads, &vec![0; 12 * 0x3ff]);
    t.set_u16(t.img.word(t.at(0x1900) + 0x04), 0x3ff);
    for i in 0..4u32 {
        let c = CMDS + 12 * i;
        let (op, v) = [(2u32, 5u32), (2, 6), (3, 0), (5, 0)][i as usize];
        t.set_u32(c, op);
        t.set_u32(c + 4, v);
        t.set_u16(c + 0xa, u16::try_from(i).unwrap());
    }
    t.set_u32(SCRIPT + 0x20, 0x77);
    t.set_u32(SCRIPT + 0x24, CMDS);
    for (wait, drop_all) in [(0u32, 0u32), (0, 0), (0, 0), (0, 1), (0, 1)] {
        t.set_u32(SCRIPT + 0x2c, wait);
        t.set_u32(CFG_408, drop_all);
        let c = Call {
            eax: 0,
            ecx: SCRIPT,
            edx: 0,
        };
        t.check(t.at(0x1530), c, &[], Eax::Junk, |g, img, fr| {
            ffx_magic::l5_objhost::obj_script(g, img, fr.at, fr.ecx);
            ffx_magic::Ret::eax(0)
        });
    }
}
