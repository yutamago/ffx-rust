//! Round 48's own ports (`ffx_magic::r48`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_owna;
mod magic_r11;
#[path = "magic/ownb.rs"]
mod ownb;
#[path = "magic/sa.rs"]
mod sa;
#[path = "magic/summon_stubs.rs"]
mod summon_stubs;

use magic::{Call, Eax, SCRATCH, StubMem, TRAPS, Twin};
use sa::rec;

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

fn setf(t: &mut Twin, at: u32, v: f32) {
    t.set_u32(at, v.to_bits());
}

/// `n` floats at `at`, varied by `k` (small, never NaN).
fn floats(t: &mut Twin, at: u32, n: u32, k: u32) {
    for i in 0..n {
        let v = ((i * 37 + k * 11) % 41) as f32 * 0.125 - 2.5;
        setf(t, at + 4 * i, v);
    }
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// The slot whose trap stands for a child's visit method outside the
/// overlay (`Ch_Allocate`, unused by the overlays).
const VISIT_SLOT: u32 = 0;
/// A class record whose visit method is [`VISIT_SLOT`]'s trap.
const TRAP_CLASS: u32 = SCRATCH + 0x3_f000;

/// The visit stub: logs the visitor, the node it holds then and its mode
/// byte, and marks the node (`+0x30` counts its visits).
fn visit_stub(t: &mut Twin) {
    let row = SCRATCH + 0x3_f800;
    t.stub(VISIT_SLOT, move |m, a| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        let v = a[0];
        let node = m.r32(v);
        m.w32(row + 4 + 8 * (k % 64), node ^ k);
        m.w32(row + 8 + 8 * (k % 64), m.r32(v + 4) ^ v);
        let n = m.r32(node + 0x30);
        m.w32(node + 0x30, n.wrapping_add(1));
        0x0e00_0000 ^ node
    });
    t.set_u32(TRAP_CLASS + 0x1c, TRAPS + 16 * VISIT_SLOT);
}

/// A node at `at` with class `class`, mode byte `+0x1a` = `mode`, no
/// children (`+0x10` itself).
fn node(t: &mut Twin, at: u32, class: u32, mode: u8, k: u32) {
    junk(t, at, 0x40, k);
    t.set_u32(at, class);
    t.write(at + 0x1a, &[mode]);
    t.set_u32(at + 0x10, at);
}

/// Children `kids` under `parent` (`+0x10` the first, `+8` the next, the
/// parent ending the list).
fn children(t: &mut Twin, parent: u32, kids: &[u32]) {
    let first = kids.first().copied().unwrap_or(parent);
    t.set_u32(parent + 0x10, first);
    for (i, &c) in kids.iter().enumerate() {
        let next = kids.get(i + 1).copied().unwrap_or(parent);
        t.set_u32(c + 8, next);
    }
}

/// `magic_0387`'s bone place set-up (r45's test): the slots it calls and
/// the rotation pointer's sine table.
fn bone_place_setup(t: &mut Twin) {
    let f = t.at(0x4800);
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let mut v = 0.0f32;
            for j in 0..4 {
                v += rf(m, a[1] + 16 * i + 4 * j) * rf(m, a[2] + 4 * j);
            }
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    let rot = t.img.word(t.img.word(f + 0x3c));
    let table = SCRATCH + 0x1_0000;
    let words: Vec<u8> = (0..0x1000u32)
        .flat_map(|i| (i as f32 / 4096.0 - 0.5).to_bits().to_le_bytes())
        .collect();
    t.write(table, &words);
    let rf_at = t.img.rel32_target(rot + 11);
    t.set_u32(t.img.word(rf_at + 0x17), table);
    t.set_u32(t.img.word(f + 0x95), 0x3f40_0000);
}

/// Slot 325 (copy a vector `(to, from)`), logged.
fn copy_325(t: &mut Twin) {
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 2.0 - 1.0;
            wf(m, a[0] + 4 * i, x);
        }
        0x325
    });
}

/// `magic_0387` 0x10003680 (`r48::line_place_0387`, internal) and its
/// thunk 0x10003760: the mode already set; children at a trap and at
/// real classes (a plain node, a matrix move); the strip's window with a
/// head part (slot 325) or none (the bone place), wrapping or not.
#[test]
fn line_place_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x3680);
    all_stubs(&mut t);
    visit_stub(&mut t);
    copy_325(&mut t);
    bone_place_setup(&mut t);
    let plain = t.at(0x113d8); // EgoNode: the plain walk
    let (v, o, c, k1, k2, k3, data) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x500,
        SCRATCH + 0x600,
        SCRATCH + 0x700,
        SCRATCH + 0x2000,
    );
    // (mode, the strip: length, start, window)
    for (k, (mode, len, start, window)) in (0u32..).zip([
        (7u8, 10u16, 2u16, 3u16),
        (3, 10, 2, 3),
        (3, 10, 8, 5),
        (3, 10, 2, 0),
        (3, 10, 10, 4),
        (5, 4, 1, 2),
        (3, 10, 2, 3),
    ]) {
        node(&mut t, o, TRAP_CLASS, 7, k);
        node(&mut t, k1, TRAP_CLASS, 1, k + 1);
        node(&mut t, k2, plain, 2, k + 2);
        node(&mut t, k3, TRAP_CLASS, 4, k + 3);
        if k == 5 {
            children(&mut t, o, &[]);
        } else {
            children(&mut t, o, &[k1, k2, k3]);
        }
        node(&mut t, c, TRAP_CLASS, 9, k + 4);
        junk(&mut t, c + 0x40, 0x80, k);
        floats(&mut t, c + 0x2c, 4, k);
        for (d, x) in [(0u32, 0x1230u32), (4, 0x4560), (8, 0x7890)] {
            t.set_u32(c + 0x4c + d, x * (k + 1));
        }
        t.set_u32(c + 0x6c, SCRATCH + 0x800);
        floats(&mut t, SCRATCH + 0x800 + 0x9c, 16, k + 4);
        t.set_u32(c + 0x1c, data);
        t.write(c + 0x20, &len.to_le_bytes());
        t.write(c + 0x22, &0x10u16.to_le_bytes());
        t.write(c + 0x26, &window.to_le_bytes());
        t.write(c + 0x28, &start.to_le_bytes());
        floats(&mut t, data, 0x40, k);
        t.set_u32(o + 0x2c, c);
        floats(&mut t, o + 0x1c, 4, k + 7);
        junk(&mut t, v, 8, k);
        t.set_u32(v, o);
        t.write(v + 5, &[mode]);
        ownb::dirty_stack(&mut t, 0x3680 + k, &[]);
        if k == 6 {
            t.check_entry(t.at(0x3760), Call::default(), &[v], Eax::Checked);
        } else {
            t.check(f, Call::this(v), &[], Eax::Checked, |g, img, fr| {
                ffx_magic::r48::line_place_0387(g, img, fr)
            });
        }
    }
}

/// `magic_0387` 0x10004ca0 (`r48::matrix_move_0387`, internal) and its
/// thunk 0x10004e90: the mode already set; children at a trap and a
/// plain node, the two visited children, the slots' vectors logged.
#[test]
fn matrix_move_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x4ca0);
    all_stubs(&mut t);
    visit_stub(&mut t);
    copy_325(&mut t);
    for s in [232u32, 230] {
        rec(&mut t, s, 2, move |m, a| {
            for i in 0..4 {
                let x = rf(m, a[1] + 4 * i) + 0.25;
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(&mut t, 266, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x266
    });
    let plain = t.at(0x113d8);
    let (v, o, a, b, k1, k2) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x500,
        SCRATCH + 0x600,
        SCRATCH + 0x700,
    );
    junk_ops(&mut t, f, &[0xd5]);
    let kk = t.img.word(f + 0xd5);
    setf(&mut t, kk, 1.0);
    for (k, mode) in (0u32..).zip([7u8, 3, 4, 4]) {
        node(&mut t, o, TRAP_CLASS, 7, k);
        junk(&mut t, o + 0x40, 0x140, k);
        floats(&mut t, o + 0x1c, 0x30, k + 2);
        setf(&mut t, o + 0x14c, 0.25 + k as f32 * 0.3);
        node(&mut t, k1, TRAP_CLASS, 1, k + 1);
        node(&mut t, k2, plain, 2, k + 2);
        children(&mut t, o, &[k1, k2]);
        node(&mut t, a, TRAP_CLASS, 9, k + 4);
        node(&mut t, b, plain, 9, k + 5);
        floats(&mut t, b + 0x1c, 4, k + 3);
        floats(&mut t, a + 0x1c, 4, k + 5);
        t.set_u32(o + 0x13c, a);
        t.set_u32(o + 0x150, b);
        junk(&mut t, v, 8, k);
        t.set_u32(v, o);
        t.write(v + 5, &[mode]);
        ownb::dirty_stack(&mut t, 0x4ca0 + k, &[]);
        if k == 3 {
            t.check_entry(t.at(0x4e90), Call::default(), &[v], Eax::Checked);
        } else {
            t.check(f, Call::this(v), &[], Eax::Checked, |g, img, fr| {
                ffx_magic::r48::matrix_move_0387(g, img, fr)
            });
        }
    }
}

/// `magic_0387` 0x10004940 (`r48::laser_move_0387`, internal) and its
/// call thunk 0x10004b70: the mode already set; the bone (taking its
/// early way out), `O[0x70]` negative or not, each gate (`O[0x7a]`, `G`,
/// `O[0x7c]`) closed, the ring's head wrapping; the loop's slots logged.
#[test]
fn laser_move_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x4940);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    visit_stub(&mut t);
    copy_325(&mut t);
    bone_place_setup(&mut t);
    rec(&mut t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    let plain = t.at(0x113d8);
    let (v, o, bn, k1, k2, base) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x600,
        SCRATCH + 0x700,
        SCRATCH + 0x3000,
    );
    junk_ops(&mut t, f, &[0x8f, 0xcd, 0x1a7]);
    setf(&mut t, w(0xcd), 3.0);
    setf(&mut t, w(0x1a7), 0.75);
    // (mode, O[0x70], u16 O[0x7a], G, u16 O[0x7c], head, thunk)
    for (k, (mode, y, a, gl, c, head, thunk)) in (0u32..).zip([
        (7u8, 1.0f32, 1u16, 0u32, 1u16, 2u16, false),
        (3, -1.0, 0, 0, 1, 2, false),
        (3, 1.0, 1, 5, 1, 2, false),
        (3, 1.0, 1, 0, 0, 2, false),
        (3, -2.5, 1, 0, 1, 2, false),
        (3, 0.5, 2, 0, 3, 14, false),
        (4, 0.5, 2, 0, 3, 5, true),
    ]) {
        node(&mut t, o, TRAP_CLASS, 7, k);
        junk(&mut t, o + 0x40, 0x40, k);
        node(&mut t, k1, TRAP_CLASS, 1, k + 1);
        node(&mut t, k2, plain, 2, k + 2);
        children(&mut t, o, &[k1, k2]);
        node(&mut t, bn, TRAP_CLASS, mode, k + 3);
        floats(&mut t, bn + 0x9c, 16, k + 4);
        t.set_u32(o + 0x6c, bn);
        // the ring is the node itself
        t.set_u32(o + 0x1c, base);
        t.write(o + 0x20, &16u16.to_le_bytes());
        t.write(o + 0x22, &0x10u16.to_le_bytes());
        t.write(o + 0x24, &head.to_le_bytes());
        t.write(o + 0x26, &3u16.to_le_bytes());
        floats(&mut t, o + 0x2c, 4, k);
        floats(&mut t, o + 0x3c, 4, k + 1);
        for (d, x) in [(0u32, 0x1230u32), (4, 0x4560), (8, 0x7890)] {
            t.set_u32(o + 0x4c + d, x * (k + 1));
        }
        for (d, x) in [(0u32, 0x1001u32), (4, 0xffff_e003), (8, 0x77)] {
            t.set_u32(o + 0x5c + d, x + k);
        }
        setf(&mut t, o + 0x70, y);
        t.write(o + 0x7a, &a.to_le_bytes());
        t.write(o + 0x7c, &c.to_le_bytes());
        t.set_u32(w(0x8f), gl);
        junk(&mut t, base, 0x120, k);
        junk(&mut t, v, 8, k);
        t.set_u32(v, o);
        t.write(v + 5, &[mode]);
        ownb::dirty_stack(&mut t, 0x4940 + k, &[]);
        if thunk {
            t.check_entry(t.at(0x4b70), Call::this(0x0c0c_4b70), &[v], Eax::Checked);
        } else {
            t.check(f, Call::this(v), &[], Eax::Checked, |g, img, fr| {
                ffx_magic::r48::laser_move_0387(g, img, fr)
            });
        }
    }
}

/// `magic_0387` 0x10008b10 (`r48::start_task_0387`): states 0 and 0x10
/// (the part flag clear and set; with and without `N[0x1c]`, none or
/// two parts; the root list empty or not), 0x1000 (gate closed, open:
/// the leaves, the unlink), an unknown state.
#[test]
fn start_task_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x8b10);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let (tk, d, prev, next, n, p, root_kid, o) = (
        SCRATCH + 0x100,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
        SCRATCH + 0x4000,
        SCRATCH + 0x200,
        SCRATCH + 0x800,
        SCRATCH + 0x1000,
    );
    // slot 941: the part record (also the particle's program table)
    rec(&mut t, 941, 1, move |_, _| p);
    for (s, n) in [(685u32, 2), (325, 2), (388, 3), (239, 3)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    rec(&mut t, 324, 0, |m, _| {
        let k = m.r32(SCRATCH + 0x3_e000);
        m.w32(SCRATCH + 0x3_e000, k + 1);
        k * 3
    });
    // the particle node's globals (r27's particle set-up)
    let pf = img.rel32_target(f + 0x28c);
    let wrapper = img.rel32_target(pf + 0x88);
    t.set_u32(img.word(wrapper + 4), SCRATCH + 0x7f00);
    t.set_u32(SCRATCH + 0x7f00, 0x77);
    t.set_u32(img.word(pf + 0x45), SCRATCH + 0x7f10);
    let root = w(0x7a);
    let list = w(0x26f);
    junk_ops(&mut t, f, &[0x250, 0x2f0, 0x21f]);
    setf(&mut t, w(0x21f), 1.5);
    // (state, part flag, N[0x1c], parts, root's last (0: itself), gate)
    for (k, (state, flag, n1c, parts, kid, gate)) in (0u32..).zip([
        (0u32, 0x0800u16, 1u32, 2u32, false, 4u32),
        (0, 0x07ff, 1, 2, false, 4),
        (0x10, 0x0801, 0, 2, true, 4),
        (0x10, 0x0800, 1, 0, true, 4),
        (0x1000, 0x0800, 1, 2, false, 0xfb),
        (0x1000, 0x0800, 1, 2, false, 0xff),
        (5, 0x0800, 1, 2, false, 4),
    ]) {
        junk(&mut t, n, 0x1300, k);
        t.set_u32(n + 0x18, state);
        t.set_u32(n + 0x20, 9 + k);
        t.set_u32(n + 0x1c, n1c);
        t.set_u32(n + 0x1248, gate);
        t.set_u32(n + 0x1148, 0x0f0f_0f0f);
        t.set_u32(n + 0x28, o + 0x400);
        t.set_u32(n + 0x2c, o);
        t.set_u32(o + 0x4dc, parts);
        for j in 0..2u32 {
            t.set_u32(o + 0x6c + 0x98 * j, 0x0a00 + j + k);
        }
        junk(&mut t, p, 0x100, k);
        t.write(p + 0x3e, &flag.to_le_bytes());
        t.write(p + 0x36, &[k as u8 % 3]);
        // the nodes the leaves take out: none with a parent but two
        for off in [0xd38u32, 0xd64, 0xf08, 0x1260, 0xce0, 0xd0c, 0xd90, 0xdbc] {
            t.set_u32(n + off + 0x14, 0);
        }
        t.set_u32(n + 0x1260, n + 0x1200);
        t.set_u32(n + 0x1200 + 0x14, 0);
        for (node, parent) in [(n + 0xd64, SCRATCH + 0x300), (n + 0xd0c, SCRATCH + 0x340)] {
            t.set_u32(node + 0x14, parent);
            t.set_u32(node + 4, parent);
            t.set_u32(node + 8, parent);
            t.set_u32(parent + 0xc, node);
            t.set_u32(parent + 0x10, node);
            t.set_u16(parent + 0x18, 3);
        }
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if kid { root_kid } else { root });
        junk(&mut t, list, 0x20, k + 1);
        t.set_u32(list + 0x10, list);
        t.set_u32(tk, d);
        t.set_u32(tk + 4, n);
        for (a, b, c) in [(d, prev, next), (prev, next, d), (next, d, prev)] {
            t.set_u32(a + 4, b);
            t.set_u32(a + 8, c);
        }
        ownb::dirty_stack(&mut t, 0x8b10 + k, &[]);
        t.check_entry(f, Call::default(), &[tk], Eax::Checked);
    }
}

/// The particle node's globals (`sb_nodes::particle_node` at the
/// `rel32` field `f + off`): its lock and part pointer.
fn particle_globals(t: &mut Twin, f: u32, off: u32) {
    let pf = t.img.rel32_target(f + off);
    let wrapper = t.img.rel32_target(pf + 0x88);
    let lock = t.img.word(wrapper + 4);
    t.set_u32(lock, SCRATCH + 0x7f00);
    t.set_u32(SCRATCH + 0x7f00, 0x77);
    let part = t.img.word(pf + 0x45);
    t.set_u32(part, SCRATCH + 0x7f10);
}

/// `magic_0387` 0x10008e80 (`r48::shot_setup_0387`, internal): the task
/// list with a last task, the root with and without children, the
/// colour node's parent list empty or not.
#[test]
fn shot_setup_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x8e80);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    for (s, n) in [(325u32, 2), (676, 3), (941, 1), (678, 1)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    particle_globals(&mut t, f, 0x25e);
    let (n, pv, last, kid, a0) = (
        SCRATCH + 0x4000,
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
        SCRATCH + 0x400,
    );
    junk_ops(&mut t, f, &[0x180, 0x203, 0x45]);
    setf(&mut t, w(0x180), 0.5);
    let root = w(0x5f);
    let clist = w(0x29b);
    for k in 0..3u32 {
        junk(&mut t, n, 0x1300, k);
        floats(&mut t, pv, 4, k);
        junk(&mut t, last, 0x10, k);
        t.set_u32(w(0x27), last);
        t.set_u32(w(0x45), 0x0f0f_0f0f + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if k == 1 { kid } else { root });
        junk(&mut t, clist, 0x20, k + 1);
        t.set_u32(clist + 0x10, if k == 2 { kid } else { clist });
        let args = [a0, 3 + k, 0xffff_fffd + k, 0x0a03, k & 1, 0x0a05];
        let call = Call {
            ecx: n,
            edx: pv,
            xmm: [0, 0, u128::from((1.25f32 + k as f32).to_bits()), 0],
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0x8e80 + k, &[]);
        t.check(f, call, &args, Eax::Checked, |g, img, fr| {
            ffx_magic::r48::shot_setup_0387(g, img, fr)
        });
    }
}

/// `magic_0387` 0x10009170 (`r48::shots_task_0387`): state 0 waiting
/// and out (into 10 and 11), 11 waiting, with the shot busy, the first
/// shot (`q = n / 2`), a negative `N[0x24]`, the count out (into
/// 10000), 10000's two gates and the leaves, an unknown state.
#[test]
fn shots_task_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x9170);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let sf = img.rel32_target(f + 0x321);
    let ws = |o: u32| img.word(sf + o);
    all_stubs(&mut t);
    for (s, n) in [(325u32, 2), (676, 3), (941, 1), (678, 1), (268, 3)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    let row = SCRATCH + 0x3_e000;
    rec(&mut t, 324, 0, move |m, _| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        0x1235 + 0x3b * k
    });
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, move |m, _| f64::from(m.r32(row) as f32 * 0.0625 + 0.1));
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    particle_globals(&mut t, sf, 0x25e);
    let (tk, d, prev, next, n, o, last) = (
        SCRATCH + 0x100,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
        SCRATCH + 0x1_0000,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
    );
    junk_ops(&mut t, f, &[0x1ed, 0x241, 0x249, 0x276, 0x283]);
    for (op, x) in [
        (0x1ed, 0.75f32),
        (0x241, 0.5),
        (0x249, 1.25),
        (0x276, 2.0),
        (0x283, 0.375),
    ] {
        setf(&mut t, w(op), x);
    }
    junk_ops(&mut t, sf, &[0x180, 0x203, 0x45]);
    let root = w(0x83);
    let clist = ws(0x29b);
    // (state, wait, N[0x24], N[0x28], count, shot state, gate, all)
    for (k, (state, wait, at, first, count, shot, gate, all)) in (0u32..).zip([
        (0u32, 2u32, 0u32, 0u32, 9u32, 0x2000u32, 4u32, 0x2000u32),
        (0, 0, 3, 0, 9, 0x2000, 4, 0x2000),
        (11, 3, 3, 0, 9, 0x2000, 4, 0x2000),
        (11, 0, 5, 0, 9, 0x2001, 4, 0x2000),
        (11, 0, 6, 1, 20, 0x2000, 4, 0x2000),
        (11, 0, 0xffff_fffd, 1, 21, 0x2000, 4, 0x2000),
        (11, 0, 2, 0, 0, 0x2000, 4, 0x2000),
        (10000, 0, 2, 0, 9, 0x2000, 0xfb, 0x2000),
        (10000, 0, 2, 0, 9, 0x2000, 4, 0xdfff),
        (10000, 0, 2, 0, 9, 0x2000, 0xff, 0x3fff),
        (7, 0, 2, 0, 9, 0x2000, 4, 0x2000),
    ]) {
        junk(&mut t, n - 0x4000, 0xe000, k);
        t.set_u32(n + 0x18, state);
        t.set_u32(n + 0x1c, wait);
        t.set_u32(n + 0x20, count);
        t.set_u32(n + 0x24, at);
        t.set_u32(n + 0x28, first);
        t.set_u32(n + 0x30, o);
        let s = (at as i32) % 8;
        t.set_u32(n.wrapping_add(0x58).wrapping_add((s * 0x12c0) as u32), shot);
        t.set_u32(n + 0x9dc8, gate);
        t.set_u32(n + 0x9cc8, 0x0f0f_0f0f);
        for off in [
            0x70d8u32, 0x5e18, 0x4b58, 0x3898, 0x25d8, 0x1318, 0x8398, 0x58,
        ] {
            t.set_u32(n + off, all);
        }
        junk(&mut t, o, 0x200, k);
        t.set_u32(o + 0x4dc, 2);
        // the leaves: parents only on a few nodes
        let nodes: Vec<u32> = (0..2u32)
            .flat_map(|i| [0x9774 + 0x2c * i, 0x9de0 + 0x44 * i])
            .chain((0..2u32).flat_map(|j| {
                [
                    0x971c + 0x2c * j,
                    0x96c4 + 0x2c * j,
                    0x9640 + 0x2c * j,
                    0x97f8 + 0x14c * j,
                ]
            }))
            .chain([0x9698, 0x9a90])
            .collect();
        for (i, &off) in nodes.iter().enumerate() {
            let node = n + off;
            if i % 3 == 1 {
                let parent = SCRATCH + 0x600 + 0x40 * i as u32;
                t.set_u32(node + 0x14, parent);
                t.set_u32(node + 4, parent);
                t.set_u32(node + 8, parent);
                t.set_u32(parent + 0xc, node);
                t.set_u32(parent + 0x10, node);
                t.set_u16(parent + 0x18, 3);
            } else {
                t.set_u32(node + 0x14, 0);
            }
        }
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, root);
        junk(&mut t, clist, 0x20, k + 1);
        t.set_u32(clist + 0x10, clist);
        junk(&mut t, last, 0x10, k);
        t.set_u32(ws(0x27), last);
        t.set_u32(tk, d);
        t.set_u32(tk + 4, n);
        for (a, b, c) in [(d, prev, next), (prev, next, d), (next, d, prev)] {
            t.set_u32(a + 4, b);
            t.set_u32(a + 8, c);
        }
        ownb::dirty_stack(&mut t, 0x9170 + k, &[]);
        t.check_entry(f, Call::default(), &[tk], Eax::Checked);
    }
}
