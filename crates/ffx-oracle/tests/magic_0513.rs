//! `magic_0513.dll` against its Rust port (`ffx_magic::m0513`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0513};
use magic::{Call, Eax, SCRATCH, TRAPS};
use magic_owna::{rec, recs, twin};

const N: u16 = 513;

/// The trap slot the model nodes' children's vtable method answers through.
const METHOD: u32 = 900;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x100089a0: no node; a node that is its neighbours' first and last;
/// one with other neighbours.
#[test]
fn end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x89a0);
    recs(&mut t, &[(250, 2)]);
    let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
    let n = SCRATCH + 0x1000;
    let (a, b) = (SCRATCH + 0x2000, SCRATCH + 0x3000);
    for case in 0..3 {
        t.write(n, &[0x31; 0x20]);
        t.set_u16(n + 0x18, 5);
        t.write(a, &[0x32; 0x20]);
        t.write(b, &[0x33; 0x20]);
        t.set_u32(w(&t, 0x2), if case == 0 { 0 } else { n });
        let (next, prev) = if case == 1 { (n, n) } else { (a, b) };
        t.set_u32(w(&t, 0x15), next);
        t.set_u32(w(&t, 0x1a), prev);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10007090: the task through its states (the wait running, out and
/// below 0; the flag bit; 10000 and others), the light block and the
/// record's globals distinct.
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7090);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0xd1), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (325, 2), (719, 3), (306, 3)]);
    let list = t.img.word(at + 0xe4);
    t.set_u32(list + 0x10, list);
    // the flag byte and the light block's words non-zero first
    let flag = t.img.word(at + 0x61);
    t.set_u32(flag, 0x5a5a_5a5a);
    let block = t.img.word(at + 0x83);
    t.write(block, &[0x4d; 0x28]);
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    for (state, wait, flags) in [
        (0u32, 0u32, 0u8),
        (1, 3, 0),
        (1, 0, 0),
        (1, 0xffff_ffff, 0),
        (2, 0, 0),
        (2, 0, 4),
        (0x2710, 0, 0),
        (7, 0, 0),
        (0x4e20, 0, 0),
    ] {
        t.write(r, &[0x3c; 0x300]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, wait);
        t.set_u8(r + 0x2d8, flags);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10007200: the set-up over a model node: the node's mode already the
/// DLL's, or not with no children and with three (their methods traps
/// that record the group they get), the node list empty and not.
#[test]
fn over_node() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7200);
    let list = t.img.word(at + 0x5a);
    let ring_last = t.img.word(at + 0x22);
    let ring_flags = t.img.word(at + 0x40);
    let mode = t.img.word(at + 0x110);
    recs(&mut t, &[(325, 2)]);
    // the children's method: records the group's words and changes the
    // mode byte the group carries
    rec(&mut t, METHOD, 1, |m, a| {
        let w0 = m.r32(a[0]);
        let w1 = m.r32(a[0] + 4);
        m.w32(SCRATCH + 0x2_0000, w0);
        // only the mode byte: the group's other bytes are frame junk
        m.w32(SCRATCH + 0x2_0004, w1 & 0xff00);
        m.w32(a[0] + 4, w1.wrapping_add(0x0100));
        0x900
    });
    let vt = SCRATCH + 0x600;
    t.set_u32(vt + 0x1c, TRAPS + 16 * METHOD);
    let obj = SCRATCH + 0x1000;
    let node = SCRATCH + 0x1400;
    let kids = [SCRATCH + 0x1600, SCRATCH + 0x1680, SCRATCH + 0x1700];
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (same, n, empty) in [
        (true, 3usize, true),
        (false, 0, false),
        (false, 3, true),
        (false, 1, false),
    ] {
        t.write(obj, &[0x5b; 0x200]);
        t.write(node, &[0x2c; 0x40]);
        t.set_u8(mode, 0x37);
        t.set_u8(node + 0x1a, if same { 0x37 } else { 0x15 });
        let mut cur = node;
        for k in kids.iter().take(n).rev() {
            t.set_u32(*k, vt);
            t.set_u32(k + 8, cur);
            cur = *k;
        }
        t.set_u32(node + 0x10, cur);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: SCRATCH + 0xc00,
            ..Call::default()
        };
        t.check(at, c, &[node], Eax::Checked, |g, img, fr| {
            Ret::eax(m0513::over_node(g, img, fr))
        });
    }
}

/// 0x10007430: the set-up between two model nodes: each node's mode the
/// DLL's already or not, with no children and with some (their method a
/// trap that records the group and changes its mode byte), the node
/// list empty and not.
#[test]
fn between() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7430);
    let list = t.img.word(at + 0x5c);
    let ring_last = t.img.word(at + 0x24);
    let ring_flags = t.img.word(at + 0x42);
    let mode = t.img.word(at + 0x87);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0x267), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(325, 2), (714, 5)]);
    rec(&mut t, METHOD, 1, |m, a| {
        let w0 = m.r32(a[0]);
        let w1 = m.r32(a[0] + 4);
        m.w32(SCRATCH + 0x2_0000, w0);
        // only the mode byte: the group's other bytes are frame junk
        m.w32(SCRATCH + 0x2_0004, w1 & 0xff00);
        m.w32(a[0] + 4, w1.wrapping_add(0x0100));
        0x900
    });
    let vt = SCRATCH + 0x600;
    t.set_u32(vt + 0x1c, TRAPS + 16 * METHOD);
    // the particle node's list empty: its last node itself
    let list2 = t.img.word(at + 0x280);
    t.set_u32(list2 + 0x10, list2);
    let obj = SCRATCH + 0x1000;
    let (p, q) = (SCRATCH + 0x1400, SCRATCH + 0x1500);
    let kids = [
        SCRATCH + 0x1600,
        SCRATCH + 0x1680,
        SCRATCH + 0x1700,
        SCRATCH + 0x1780,
    ];
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (same_p, np, same_q, nq, empty) in [
        (true, 2usize, true, 1usize, true),
        (false, 0, false, 2, false),
        (false, 2, false, 0, true),
        (false, 1, true, 0, false),
    ] {
        t.write(obj, &[0x5b; 0x300]);
        t.set_u8(mode, 0x37);
        for (node, same, n, ks) in [(p, same_p, np, &kids[..2]), (q, same_q, nq, &kids[2..])] {
            t.write(node, &[0x2c; 0x40]);
            t.set_u8(node + 0x1a, if same { 0x37 } else { 0x15 });
            let mut cur = node;
            for k in ks.iter().take(n).rev() {
                t.set_u32(*k, vt);
                t.set_u32(k + 8, cur);
                cur = *k;
            }
            t.set_u32(node + 0x10, cur);
        }
        t.set_u32(q + 0x1c, 0x00a1_0000);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: p,
            ..Call::default()
        };
        t.check(at, c, &[q], Eax::Checked, |g, img, fr| {
            Ret::eax(m0513::between(g, img, fr))
        });
    }
}

/// 0x100077d0: the set-up for a target: the target's size through slot
/// 333 or slot 601 (bit 0 of `+0x90`), slot 251's answer below, between
/// and above the bounds, the node list empty and not.
#[test]
fn for_target() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x77d0);
    let list = t.img.word(at + 0x5d);
    let ring_last = t.img.word(at + 0x25);
    let ring_flags = t.img.word(at + 0x43);
    let cell = SCRATCH + 0x10;
    recs(&mut t, &[(268, 3)]);
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, move |m, _| {
        f64::from(f32::from_bits(m.r32(cell))) * 1.25
    });
    rec(&mut t, 601, 2, move |m, a| {
        let v = m.r32(cell);
        m.w32(a[1], v ^ 0x0040_0000);
        0x601
    });
    rec(&mut t, 251, 1, |_, _| 0x251);
    t.stub_st0(251, move |m, _| f64::from(f32::from_bits(m.r32(cell + 4))));
    let obj = SCRATCH + 0x1000;
    let (src, a) = (SCRATCH + 0x1400, SCRATCH + 0x1500);
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (bit, size, y, empty) in [
        (1u8, 3.5f32, 1.0f32, true),
        (0, 3.5, 12.0, false),
        (1, -7.25, 1000.0, false),
        (0, 0.75, f32::NAN, true),
    ] {
        t.write(obj, &[0x5b; 0x200]);
        t.write(src, &[0x2c; 0x100]);
        t.write(a, &[0x3d; 0x100]);
        t.set_u8(a + 0x90, bit | 0x5a);
        t.set_u32(cell, size.to_bits());
        t.set_u32(cell + 4, y.to_bits());
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: src,
            ..Call::default()
        };
        t.check(at, c, &[a], Eax::Checked, |g, img, fr| {
            Ret::eax(m0513::for_target(g, img, fr))
        });
    }
}

/// 0x10006a90: the motion task through every state (the motion done or
/// not; the object's flag bit for the leave), and states it ignores.
#[test]
fn motion_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6a90);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 411, 1, move |m, _| m.r32(cell) & 1);
    rec(&mut t, 228, 2, |_, a| 0x0228_0000 ^ a[1]);
    recs(
        &mut t,
        &[(234, 3), (233, 3), (248, 3), (213, 2), (211, 2), (255, 2)],
    );
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    let (o, o2) = (SCRATCH + 0x1400, SCRATCH + 0x1500);
    let mut cases = Vec::new();
    for s in [
        0u32, 1, 5, 10, 11, 0x14, 0x1e, 0x1f, 0x40, 0x2710, 0x2711, 0x2712, 0x2713, 0x2714, 0x2720,
    ] {
        cases.push((s, 0u32));
        cases.push((s, 1));
    }
    for (state, bits) in cases {
        t.write(r, &[0x3c; 0x80]);
        t.set_u32(r + 0x20, 9.5f32.to_bits());
        t.set_u32(r + 0x24, 2.5f32.to_bits());
        t.set_u32(r + 0x48, o);
        t.set_u32(r + 0x4c, o2);
        t.write(o, &[0x4d; 0x100]);
        t.set_u32(o + 0x50, 0x00c0_0050);
        t.set_u8(o + 0x90, 0x10 | (bits & 1) as u8);
        t.set_u32(o + 0x4c, SCRATCH + 0x1800);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, 7);
        t.set_u32(cell, bits);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10006db0: the lit set-up over a model node: the node's mode the
/// DLL's already or not, with no children and with some, the node list
/// empty and not; the light block and the flag byte non-zero first.
#[test]
fn lit_node() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6db0);
    let list = t.img.word(at + 0x5a);
    let ring_last = t.img.word(at + 0x22);
    let ring_flags = t.img.word(at + 0x40);
    let mode = t.img.word(at + 0xf8);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(at + 0x207), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(325, 2), (714, 5), (719, 3), (306, 3)]);
    rec(&mut t, METHOD, 1, |m, a| {
        let w0 = m.r32(a[0]);
        let w1 = m.r32(a[0] + 4);
        m.w32(SCRATCH + 0x2_0000, w0);
        // only the mode byte: the group's other bytes are frame junk
        m.w32(SCRATCH + 0x2_0004, w1 & 0xff00);
        m.w32(a[0] + 4, w1.wrapping_add(0x0100));
        0x900
    });
    let vt = SCRATCH + 0x600;
    t.set_u32(vt + 0x1c, TRAPS + 16 * METHOD);
    let list2 = t.img.word(at + 0x220);
    t.set_u32(list2 + 0x10, list2);
    let flag = t.img.word(at + 0x244);
    t.set_u32(flag, 0x5a5a_5a5a);
    let block = t.img.word(at + 0x25f);
    t.write(block, &[0x4d; 0x24]);
    let obj = SCRATCH + 0x1000;
    let node = SCRATCH + 0x1400;
    let kids = [SCRATCH + 0x1600, SCRATCH + 0x1680];
    let (old, prev) = (SCRATCH + 0x800, SCRATCH + 0x900);
    for (same, n, empty) in [(true, 2usize, true), (false, 0, false), (false, 2, true)] {
        t.write(obj, &[0x5b; 0x300]);
        t.write(node, &[0x2c; 0x40]);
        t.set_u8(mode, 0x37);
        t.set_u8(node + 0x1a, if same { 0x37 } else { 0x15 });
        let mut cur = node;
        for k in kids.iter().take(n).rev() {
            t.set_u32(*k, vt);
            t.set_u32(k + 8, cur);
            cur = *k;
        }
        t.set_u32(node + 0x10, cur);
        t.write(old, &[0x6c; 0x40]);
        t.write(prev, &[0x7d; 0x40]);
        t.write(list, &[0x4e; 0x20]);
        t.set_u32(list + 0x10, if empty { list } else { prev });
        t.set_u32(list + 0xc, 0x00f1_0000);
        t.set_u16(list + 0x18, 0x0305);
        t.set_u32(ring_last, old);
        t.set_u32(ring_flags, 0x8000_0007);
        let c = Call {
            ecx: obj,
            edx: SCRATCH + 0xc00,
            ..Call::default()
        };
        t.check(at, c, &[node], Eax::Checked, |g, img, fr| {
            Ret::eax(m0513::lit_node(g, img, fr))
        });
    }
}

/// 0x100076e0: the light task through its states (the wait running, out
/// and below 0; the flag bit; 10000 and others); the globals it reads
/// distinct.
#[test]
fn light_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x76e0);
    let pnw = t.img.rel32_target(at + 0x56);
    let table = SCRATCH + 0x300;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x400;
    t.set_u32(t.img.word(pnw + 0x0d), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (325, 2), (719, 3), (306, 3), (239, 3)]);
    let list = t.img.word(pnw + 0x26);
    t.set_u32(list + 0x10, list);
    let b = t.img.word(at + 0x5d);
    t.set_u32(b, 0x5a5a_5aa7);
    let a = t.img.word(at + 0x6b);
    t.set_u32(a, 0x00a0_006b);
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    for (state, wait, flags) in [
        (0u32, 0u32, 0u8),
        (1, 3, 0),
        (1, 0, 0),
        (1, 0xffff_ffff, 0),
        (2, 0, 0),
        (2, 0, 4),
        (0x2710, 0, 0),
        (7, 0, 0),
        (0x4e20, 0, 0),
    ] {
        t.write(r, &[0x3c; 0x300]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, wait);
        t.set_u8(r + 0x2d8, flags);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
