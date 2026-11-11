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
    copy_325(&mut t);
    for (s, n) in [(676u32, 3), (941, 1), (678, 1), (268, 3)] {
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
            // (b5 test gap: the vector's raw words logged; tiny junk vanishes in the sum)
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
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
        (0, 0xffff_fffe, 3, 0, 9, 0x2001, 4, 0x2000),
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
        t.set_u32(n + 0x9dc8, gate);
        t.set_u32(n + 0x9cc8, 0x0f0f_0f0f);
        for off in [
            0x70d8u32, 0x5e18, 0x4b58, 0x3898, 0x25d8, 0x1318, 0x8398, 0x58,
        ] {
            t.set_u32(n + off, all);
        }
        // (the shot's state after the eight: a busy one stays busy)
        let s = (at as i32) % 8;
        if shot != 0x2000 {
            t.set_u32(n.wrapping_add(0x58).wrapping_add((s * 0x12c0) as u32), shot);
        }
        junk(&mut t, o, 0x200, k);
        t.set_u32(o + 0x4dc, 2);
        // (b4 test gap: the matrices slot 229 reads, finite)
        floats(&mut t, n + 0x9894, 0x60, k);
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

/// `magic_0387` 0x1000ba40 (`r48::pulse_setup_0387`, internal): none,
/// one and three parts (slot 291's counts varied), the effect's time by
/// slot 333 or slot 601, the root with children or not.
#[test]
fn pulse_setup_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xba40);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 291, 5, |_, a| 3 + (a[1] & 0xf));
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.25);
    rec(&mut t, 601, 2, |m, a| {
        m.w32(a[1], 1.5f32.to_bits());
        0x601
    });
    let tl = img.rel32_target(f + 0xd2);
    let (n, e, o, last, kid) = (
        SCRATCH + 0x1_0000,
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
    );
    junk_ops(&mut t, f, &[0x1af]);
    junk_ops(&mut t, tl, &[0x55, 0x66, 0x6e]);
    for (op, x) in [(0x55u32, 30.0f32), (0x66, 2.0), (0x6e, 0.5)] {
        setf(&mut t, img.word(tl + op), x);
    }
    let root = w(0x5a);
    for (k, parts) in (0u32..).zip([0u32, 1, 3]) {
        junk(&mut t, n, 0x8900, k);
        junk(&mut t, e, 0x100, k);
        t.write(e + 0x90, &[k as u8 & 1]);
        junk(&mut t, last, 0x10, k);
        t.set_u32(w(0x22), last);
        t.set_u32(w(0x40), 0x0f0f_0f0f + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if k == 1 { kid } else { root });
        t.set_u32(o + 0x4dc, parts);
        for j in 0..3u32 {
            t.set_u32(o + 0x6c + 0x98 * j, 0x0a00 + 5 * j + k);
        }
        let call = Call {
            ecx: n,
            edx: e,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0xba40 + k, &[]);
        t.check(
            f,
            call,
            &[o, 0x4000_0000 + k],
            Eax::Checked,
            ffx_magic::r48::pulse_setup_0387,
        );
    }
}

/// `magic_0387` 0x1000bf10 (`r48::glow_task_0387`): every state (0 with
/// the part flag clear and set, 0x50, 0x51 waiting and out, 0x80 into
/// 0x81 and on, 0x81 waiting, 0x1000 gate closed and open, unknown
/// states below and above 0x51).
#[test]
fn glow_task_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xbf10);
    all_stubs(&mut t);
    copy_325(&mut t);
    let p = SCRATCH + 0x400;
    rec(&mut t, 941, 1, move |_, _| p);
    let (tk, d, prev, next, n) = (
        SCRATCH + 0x100,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
        SCRATCH + 0x1_0000,
    );
    // (state, wait, part flag, byte N[0x3458])
    for (k, (state, wait, flag, gate)) in (0u32..).zip([
        (0u32, 3u32, 0x07ffu16, 0x04u8),
        (0, 3, 0x0800, 0x04),
        (0x50, 0, 0x0800, 0x04),
        (0x51, 2, 0x0800, 0x04),
        (0x51, 0xffff_fffe, 0x0800, 0x04),
        (0x80, 3, 0x0800, 0x04),
        (0x81, 3, 0x0800, 0x04),
        (0x81, 1, 0x0800, 0xfb),
        (0x1000, 1, 0x0800, 0xfb),
        (0x1000, 1, 0x0800, 0x0f),
        (0x20, 1, 0x0800, 0x0f),
        (0x60, 1, 0x0800, 0x0f),
    ]) {
        junk(&mut t, n + 0x3000, 0x480, k);
        t.set_u32(n + 0x18, state);
        t.set_u32(n + 0x1c, wait);
        t.write(n + 0x3458, &[gate]);
        floats(&mut t, n + 0x32e8, 1, k);
        junk(&mut t, p, 0x40, k);
        t.write(p + 0x3e, &flag.to_le_bytes());
        for off in [0x32c0u32, 0x3274, 0x3128, 0x30d0, 0x30fc, 0x3320] {
            t.set_u32(n + off + 0x14, 0);
        }
        t.set_u32(n + 0x32c0, n + 0x3200);
        t.set_u32(n + 0x3200 + 0x14, 0);
        let (node, parent) = (n + 0x30d0, SCRATCH + 0x300);
        t.set_u32(node + 0x14, parent);
        t.set_u32(node + 4, parent);
        t.set_u32(node + 8, parent);
        t.set_u32(parent + 0xc, node);
        t.set_u32(parent + 0x10, node);
        t.set_u16(parent + 0x18, 3);
        t.set_u32(tk, d);
        t.set_u32(tk + 4, n);
        for (a, b, c) in [(d, prev, next), (prev, next, d), (next, d, prev)] {
            t.set_u32(a + 4, b);
            t.set_u32(a + 8, c);
        }
        ownb::dirty_stack(&mut t, 0xbf10 + k, &[]);
        t.check_entry(f, Call::default(), &[tk], Eax::Checked);
    }
}

/// `magic_0387` 0x1000b1a0 (`r48::pulse_init_0387`, internal): the task
/// list, the root and the list with and without children; the draws and
/// slot 229's vector logged.
#[test]
fn pulse_init_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xb1a0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    copy_325(&mut t);
    let row = SCRATCH + 0x3_e000;
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, move |m, _| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        f64::from(k as f32 * 0.125 + 0.3)
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            // (b5 test gap: the vector's raw words logged; tiny junk vanishes in the sum)
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    for (s, n) in [(719u32, 3), (235, 1), (230, 2)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    let (n, pv, a4, last, kid) = (
        SCRATCH + 0x1_0000,
        SCRATCH + 0x100,
        SCRATCH + 0x400,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
    );
    let ops = [
        0x110u32, 0x11d, 0x125, 0x142, 0x14f, 0x157, 0x174, 0x188, 0x197,
    ];
    junk_ops(&mut t, f, &ops);
    for (i, &op) in ops.iter().enumerate() {
        setf(&mut t, w(op), 0.5 + i as f32 * 0.25);
    }
    let root = w(0x66);
    let jl = w(0x424);
    for k in 0..3u32 {
        junk(&mut t, n, 0x600, k);
        floats(&mut t, pv, 4, k);
        junk(&mut t, a4, 0x100, k);
        floats(&mut t, a4 + 0x9c, 16, k + 2);
        junk(&mut t, last, 0x10, k);
        t.set_u32(w(0x31), last);
        t.set_u32(w(0x4c), 0x0f0f_0f0f + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if k == 1 { kid } else { root });
        junk(&mut t, jl, 0x20, k + 1);
        t.set_u32(jl + 0x10, if k == 2 { kid } else { jl });
        let args = [
            0x0bad_0000,
            0x0bad_0001,
            0xffff_ffff - k,
            0x0a03,
            a4,
            0x0a05 + k,
        ];
        let call = Call {
            ecx: n,
            edx: pv,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0xb1a0 + k, &[]);
        t.check(f, call, &args, Eax::Checked, |g, img, fr| {
            ffx_magic::r48::pulse_init_0387(g, img, fr)
        });
    }
}

/// `magic_0387` 0x1000b640 (`r48::pulse_task_0387`): state 0 waiting
/// and out (into 10 and 11), 11 waiting, the pulse busy, a drawn part
/// (weights left: each part, a negative `N[0x24]`), a part by the draw's
/// remainder, the count out (into 10000), 10000 with a pulse busy and
/// all done, an unknown state.
#[test]
fn pulse_task_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xb640);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let pi = img.rel32_target(f + 0x1f2);
    let wp = |o: u32| img.word(pi + o);
    all_stubs(&mut t);
    copy_325(&mut t);
    let row = SCRATCH + 0x3_e000;
    rec(&mut t, 324, 0, move |m, _| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        0x1235 + 0x3b * k
    });
    let draw = SCRATCH + 0x3_e010;
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, move |m, _| f64::from(rf(m, draw)));
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            // (b5 test gap: the vector's raw words logged; tiny junk vanishes in the sum)
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    for (s, n) in [(719u32, 3), (235, 1), (230, 2)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    let (tk, d, prev, next, n, o) = (
        SCRATCH + 0x100,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
        SCRATCH + 0x1_8000,
        SCRATCH + 0x1000,
    );
    junk_ops(&mut t, f, &[0xd5]);
    setf(&mut t, w(0xd5), 0.25);
    let root = w(0x119);
    let list = wp(0x424);
    // (state, wait, N[0x24], weights left, count, pulse state, all, draw)
    for (k, (state, wait, at, left, count, pulse, all, x)) in (0u32..).zip([
        (0u32, 2u32, 0u32, 0u32, 9u32, 0x2000u32, 0x2000u32, 0.0f32),
        (0, 0, 3, 0, 9, 0x2001, 0x2000, 0.0),
        (11, 2, 3, 0, 9, 0x2000, 0x2000, 0.0),
        (11, 0, 5, 3, 9, 0x2000, 0x2000, 0.3),
        (11, 0, 6, 3, 9, 0x2000, 0x2000, 0.6),
        (11, 0, 0xffff_fff3, 3, 9, 0x2000, 0x2000, 2.0),
        (11, 0, 2, 0, 9, 0x2000, 0x2000, 0.0),
        (11, 0, 2, 3, 0, 0x2000, 0xffff_dfff, 0.1),
        (10000, 0, 2, 0, 9, 0x2000, 0xffff, 0.0),
        (7, 0, 2, 0, 9, 0x2000, 0x2000, 0.0),
    ]) {
        junk(&mut t, n - 0x8000, 0x11000, k);
        t.set_u32(n + 0x18, state);
        t.set_u32(n + 0x1c, wait);
        t.set_u32(n + 0x20, count);
        t.set_u32(n + 0x24, at);
        t.set_u32(n + 0x28, left);
        for (j, c) in (0u32..).zip([3u32, 2, 1]) {
            t.set_u32(n + 0x2c + 4 * j, c);
            setf(&mut t, n + 0x38 + 4 * j, c as f32 / 6.0);
        }
        t.set_u32(n + 0x48, o);
        for i in 0..16u32 {
            t.set_u32(n + 0x78 + 0x860 * i, all);
        }
        let s = (at as i32) % 16;
        t.set_u32(n.wrapping_add(0x78).wrapping_add((s * 0x860) as u32), pulse);
        junk(&mut t, o, 0x200, k);
        t.set_u32(o + 0x4dc, 3);
        floats(&mut t, n + 0x876c, 16, k);
        for off in [0x868cu32, 0x8660, 0x86d0] {
            t.set_u32(n + off + 0x14, 0);
        }
        let (node, parent) = (n + 0x8660, SCRATCH + 0x300);
        t.set_u32(node + 0x14, parent);
        t.set_u32(node + 4, parent);
        t.set_u32(node + 8, parent);
        t.set_u32(parent + 0xc, node);
        t.set_u32(parent + 0x10, node);
        t.set_u16(parent + 0x18, 3);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, root);
        junk(&mut t, list, 0x20, k + 1);
        t.set_u32(list + 0x10, list);
        t.set_u32(wp(0x31), SCRATCH + 0x200);
        setf(&mut t, draw, x);
        t.set_u32(tk, d);
        t.set_u32(tk + 4, n);
        for (a, b, c) in [(d, prev, next), (prev, next, d), (next, d, prev)] {
            t.set_u32(a + 4, b);
            t.set_u32(a + 8, c);
        }
        ownb::dirty_stack(&mut t, 0xb640 + k, &[]);
        t.check_entry(f, Call::default(), &[tk], Eax::Checked);
    }
}

/// `magic_0387` 0x1000a8e0 (`r48::burst_setup_0387`, internal): the task
/// list and the root with and without children.
#[test]
fn burst_setup_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xa8e0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    copy_325(&mut t);
    for (s, n) in [(941u32, 1), (719, 3)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    particle_globals(&mut t, f, 0x272);
    let (n, q, last, kid) = (
        SCRATCH + 0x1_0000,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
    );
    junk_ops(&mut t, f, &[0x134, 0x217]);
    setf(&mut t, w(0x134), 0.75);
    let root = w(0x57);
    let plist = w(0x259);
    for k in 0..3u32 {
        junk(&mut t, n, 0x300, k);
        floats(&mut t, n + 0xc0, 1, k);
        junk(&mut t, q, 0x200, k);
        floats(&mut t, q + 0xcc, 4, k);
        t.set_u32(q + 0x140, SCRATCH + 0x1400);
        floats(&mut t, SCRATCH + 0x1400 + 0x1c, 4, k + 1);
        t.set_u32(q + 0x144, SCRATCH + 0x1500);
        junk(&mut t, SCRATCH + 0x1500, 0x40, k + 2);
        junk(&mut t, last, 0x10, k);
        t.set_u32(w(0x22), last);
        t.set_u32(w(0x3d), 0x0f0f_0f0f + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if k == 1 { kid } else { root });
        junk(&mut t, plist, 0x20, k + 1);
        t.set_u32(plist + 0x10, if k == 2 { kid } else { plist });
        let call = Call {
            ecx: n,
            edx: q,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0xa8e0 + k, &[]);
        t.check(f, call, &[], Eax::Checked, ffx_magic::r48::burst_setup_0387);
    }
}

/// `magic_0387` 0x1000c150 (`r48::glow_setup_0387`, internal): the task
/// list, the root and the lists with and without children.
#[test]
fn glow_setup_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xc150);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    for (s, n) in [(941u32, 1), (676, 3), (678, 1), (325, 2)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    particle_globals(&mut t, f, 0x2ce);
    let (n, last, kid) = (SCRATCH + 0x1_0000, SCRATCH + 0x200, SCRATCH + 0x300);
    junk_ops(&mut t, f, &[0x273]);
    let root = w(0x60);
    let plist = w(0x2b5);
    for k in 0..3u32 {
        junk(&mut t, n, 0x3400, k);
        junk(&mut t, last, 0x10, k);
        t.set_u32(w(0x22), last);
        t.set_u32(w(0x40), 0x0f0f_0f0f + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if k == 1 { kid } else { root });
        junk(&mut t, plist, 0x20, k + 1);
        t.set_u32(plist + 0x10, if k == 2 { kid } else { plist });
        let call = Call {
            ecx: n,
            edx: 0x0e0e_0000 + k,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0xc150 + k, &[]);
        t.check(
            f,
            call,
            &[0x0a0a_0000 + k],
            Eax::Checked,
            ffx_magic::r48::glow_setup_0387,
        );
    }
}

/// `magic_0387` 0x1000c4a0 (`r48::laser_task_0387`): every state and
/// its gates (the `N[0xbb8]` levels, the waits, the count, `N[0x694]`,
/// the byte counter reaching 0x30), unknown states on both sides.
#[test]
fn laser_task_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xc4a0);
    all_stubs(&mut t);
    copy_325(&mut t);
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x0941_0000);
    particle_globals(&mut t, f, 0xb7);
    let (tk, d, prev, next, n) = (
        SCRATCH + 0x100,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
        SCRATCH + 0x1_0000,
    );
    let plist = t.img.word(f + 0xa2);
    junk_ops(&mut t, f, &[0x8c]);
    // (state, wait, count, N[0xbb8], i16 N[0x694], byte N[0xb18], byte N[0x698])
    for (k, (state, wait, count, lvl, h, b, g8)) in (0u32..).zip([
        (0u32, 1u32, 1u32, 0x4fu32, 0i16, 0u8, 0u8),
        (0, 1, 1, 0x50, 0, 0, 0),
        (0x10, 1, 1, 0x50, 0, 0, 0),
        (0x11, 0xffff_ffff, 1, 0x50, 0, 0, 0),
        (0x11, 3, 1, 0x50, 0, 0, 0),
        (0x40, 3, 1, 0x7f, 0, 0, 0),
        (0x40, 3, 1, 0x80, 1, 0, 0),
        (0x40, 3, 1, 0x80, 0, 0x2d, 0),
        (0x42, 0, 0, 0x80, -1, 0x2c, 0),
        // (b7 test gap: 0 itself; the sprite size writes 8 there on the 0x40 path)
        (0x42, 0, 3, 0x80, 0, 0x2c, 0),
        (0x43, 0, 0, 0x80, 0, 0x2d, 0),
        (0x43, 2, 0, 0x80, 0, 0x30, 0),
        (0x43, 2, 0, 0x80, 0, 0x2f, 0),
        (0x43, 2, 3, 0x80, 0, 0x2f, 0),
        (0x44, 1, 3, 0x80, 0, 0, 0),
        (0x44, 0, 3, 0x2000, 0, 0, 0xfb),
        (0x44, 0, 3, 0x1fff, 0, 0, 0x04),
        (0x1000, 0, 3, 0x2000, 0, 0, 0x04),
        (0x45, 0, 3, 0x2000, 0, 0, 0x04),
        (0x2000, 0, 3, 0x2000, 0, 0, 0x04),
        (0x30, 0, 3, 0x2000, 0, 0, 0x04),
    ]) {
        junk(&mut t, n, 0xc00, k);
        t.set_u32(n + 0x18, state);
        t.set_u32(n + 0x1c, wait);
        t.set_u32(n + 0x20, count);
        t.set_u32(n + 0xbb8, lvl);
        t.write(n + 0x694, &h.to_le_bytes());
        t.write(n + 0xb18, &[b]);
        t.write(n + 0x698, &[g8]);
        for off in [0x464u32, 0x438, 0xb2c, 0x730, 0x6b0] {
            t.set_u32(n + off + 0x14, 0);
        }
        let (node, parent) = (n + 0x730, SCRATCH + 0x300);
        t.set_u32(node + 0x14, parent);
        t.set_u32(node + 4, parent);
        t.set_u32(node + 8, parent);
        t.set_u32(parent + 0xc, node);
        t.set_u32(parent + 0x10, node);
        t.set_u16(parent + 0x18, 3);
        junk(&mut t, plist, 0x20, k);
        t.set_u32(plist + 0x10, plist);
        t.set_u32(tk, d);
        t.set_u32(tk + 4, n);
        for (a, b, c) in [(d, prev, next), (prev, next, d), (next, d, prev)] {
            t.set_u32(a + 4, b);
            t.set_u32(a + 8, c);
        }
        ownb::dirty_stack(&mut t, 0xc4a0 + k, &[]);
        t.check_entry(f, Call::default(), &[tk], Eax::Checked);
    }
}

/// `magic_0387` 0x1000c6e0 (`r48::laser_setup_0387`, internal): the draws
/// varied (both sides' signs), the root with and without children.
#[test]
fn laser_setup_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xc6e0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let gs = img.rel32_target(f + 0x280);
    all_stubs(&mut t);
    for (s, n) in [(941u32, 1), (676, 3), (678, 1), (325, 2), (719, 3)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    let row = SCRATCH + 0x3_e000;
    rec(&mut t, 324, 0, move |m, _| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        0x0e01_0000_u32.wrapping_add(0x0a07 * k) ^ (k << 12)
    });
    particle_globals(&mut t, gs, 0x2ce);
    junk_ops(&mut t, gs, &[0x273]);
    let (n, last, kid) = (SCRATCH + 0x1_0000, SCRATCH + 0x200, SCRATCH + 0x300);
    let root = w(0x57);
    let plist = img.word(gs + 0x2b5);
    for k in 0..4u32 {
        junk(&mut t, n, 0x3f00, k);
        junk(&mut t, last, 0x10, k);
        t.set_u32(w(0x21), last);
        t.set_u32(w(0x3f), 0x0f0f_0f0f + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if k == 1 { kid } else { root });
        junk(&mut t, plist, 0x20, k + 1);
        t.set_u32(plist + 0x10, plist);
        let call = Call {
            ecx: n,
            edx: 0x0e0e_0000 + k,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0xc6e0 + k, &[]);
        t.check(
            f,
            call,
            &[0x0a0a_0000 + k],
            Eax::Checked,
            ffx_magic::r48::laser_setup_0387,
        );
    }
}

/// `magic_0387` 0x1000d000 (`r48::lasers_setup_0387`, internal): the
/// effect's time by slot 333 or 601, the root with and without children.
#[test]
fn lasers_setup_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xd000);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let ls = img.rel32_target(f + 0x254);
    let gs = img.rel32_target(ls + 0x280);
    let tl = img.rel32_target(f + 0x112);
    all_stubs(&mut t);
    for (s, n) in [(941u32, 1), (676, 3), (678, 1), (325, 2), (719, 3)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.25);
    rec(&mut t, 601, 2, |m, a| {
        m.w32(a[1], 1.5f32.to_bits());
        0x601
    });
    let row = SCRATCH + 0x3_e000;
    rec(&mut t, 324, 0, move |m, _| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        0x0e01_0000_u32.wrapping_add(0x0a07 * k) ^ (k << 12)
    });
    particle_globals(&mut t, gs, 0x2ce);
    junk_ops(&mut t, gs, &[0x273]);
    junk_ops(&mut t, tl, &[0x55, 0x66, 0x6e]);
    for (op, x) in [(0x55u32, 30.0f32), (0x66, 2.0), (0x6e, 0.5)] {
        setf(&mut t, img.word(tl + op), x);
    }
    let (n, e, last, kid) = (
        SCRATCH + 0x1_0000,
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
    );
    let root = w(0x59);
    let plist = img.word(gs + 0x2b5);
    for k in 0..3u32 {
        junk(&mut t, n, 0x8a00, k);
        junk(&mut t, e, 0x100, k);
        t.write(e + 0x90, &[k as u8 & 1]);
        junk(&mut t, last, 0x10, k);
        t.set_u32(w(0x24), last);
        t.set_u32(w(0x3f), 0x0f0f_0f0f + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if k == 1 { kid } else { root });
        junk(&mut t, plist, 0x20, k + 1);
        t.set_u32(plist + 0x10, plist);
        let call = Call {
            ecx: n,
            edx: e,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0xd000 + k, &[]);
        let args = [SCRATCH + 0x400, 0x0a01 + k, 0x0a02 + k];
        t.check(
            f,
            call,
            &args,
            Eax::Checked,
            ffx_magic::r48::lasers_setup_0387,
        );
    }
}

/// `magic_0387` 0x1000c980 (`r48::lasers_task_0387`): state 0 (each
/// laser short, both ready), 0x10 into 0x11, 0x11 waiting and each step
/// (the first with the effect node, the last into 0x12), 0x12 waiting and
/// out (into 0x1000), 0x1000 closed and open, unknown states.
#[test]
fn lasers_task_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xc980);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    copy_325(&mut t);
    for (s, n) in [(941u32, 1), (719, 3), (388, 3), (239, 3)] {
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
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    particle_globals(&mut t, f, 0x518);
    junk_ops(
        &mut t,
        f,
        &[0x4e9, 0x60, 0x40e, 0x423, 0x53b, 0x54a, 0x555, 0x55d],
    );
    for (op, x) in [
        (0x40e, 0.5f32),
        (0x423, 1.5),
        (0x53b, 0.25),
        (0x54a, 2.0),
        (0x555, 0.75),
        (0x55d, 3.0),
    ] {
        setf(&mut t, w(op), x);
    }
    let (tk, d, prev, next, n, o, p) = (
        SCRATCH + 0x100,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
        SCRATCH + 0x1_0000,
        SCRATCH + 0x1000,
        SCRATCH + 0x1400,
    );
    let root = w(0x28c);
    let plist = w(0x4f9);
    // (state, wait, step, N[0xb98], N[0x4ba8])
    for (k, (state, wait, step, l1, l2)) in (0u32..).zip([
        (0u32, 1u32, 0u32, 0x10u32, 0x11u32),
        (0, 1, 0, 0x11, 0x10),
        (0, 1, 0, 0x11, 0x11),
        (0x10, 1, 0, 0, 0),
        (0x11, 2, 0, 0, 0),
        (0x11, 0, 0, 0, 0),
        (0x11, 0, 1, 0, 0),
        (0x11, 0xffff_fff0, 2, 0, 0),
        (0x12, 1, 0, 0, 0),
        (0x12, 0, 0, 0x2000, 0xdfff),
        (0x1000, 0, 0, 0x2000, 0x3000),
        (0x13, 0, 0, 0, 0),
        (5, 0, 0, 0, 0),
    ]) {
        junk(&mut t, n, 0x4c00, k);
        t.set_u32(n + 0x18, state);
        t.set_u32(n + 0x1c, wait);
        t.set_u32(n + 0x20, step);
        t.set_u32(n + 0x24, p);
        t.set_u32(n + 0x28, o);
        t.set_u32(n + 0xb98, l1);
        t.set_u32(n + 0x4ba8, l2);
        floats(&mut t, n + 0x254, 16, k);
        junk(&mut t, p, 0x40, k);
        junk(&mut t, o, 0x200, k);
        t.set_u32(o + 0x4dc, 2);
        for off in [0x6e8u32, 0x18c, 0x1b8, 0x2c, 0x160] {
            t.set_u32(n + off + 0x14, 0);
        }
        let (node, parent) = (n + 0x1b8, SCRATCH + 0x300);
        t.set_u32(node + 0x14, parent);
        t.set_u32(node + 4, parent);
        t.set_u32(node + 8, parent);
        t.set_u32(parent + 0xc, node);
        t.set_u32(parent + 0x10, node);
        t.set_u16(parent + 0x18, 3);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, root);
        junk(&mut t, plist, 0x20, k + 1);
        t.set_u32(plist + 0x10, plist);
        t.set_u32(tk, d);
        t.set_u32(tk + 4, n);
        for (a, b, c) in [(d, prev, next), (prev, next, d), (next, d, prev)] {
            t.set_u32(a + 4, b);
            t.set_u32(a + 8, c);
        }
        ownb::dirty_stack(&mut t, 0xc980 + k, &[]);
        t.check_entry(f, Call::default(), &[tk], Eax::Checked);
    }
}

/// `magic_0387` 0x10009820 (`r48::shots_setup_0387`, internal): the
/// effect's time by slot 333 or 601, the root with and without children.
#[test]
fn shots_setup_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x9820);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let tl = img.rel32_target(f + 0xad);
    all_stubs(&mut t);
    for (s, n) in [(941u32, 1), (325, 2)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.25);
    rec(&mut t, 601, 2, |m, a| {
        m.w32(a[1], 1.5f32.to_bits());
        0x601
    });
    particle_globals(&mut t, f, 0x362);
    junk_ops(&mut t, f, &[0x339]);
    junk_ops(&mut t, tl, &[0x55, 0x66, 0x6e]);
    for (op, x) in [(0x55u32, 30.0f32), (0x66, 2.0), (0x6e, 0.5)] {
        setf(&mut t, img.word(tl + op), x);
    }
    let (n, e, last, kid) = (
        SCRATCH + 0x1_0000,
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
    );
    let root = w(0x66);
    let plist = w(0x34c);
    for k in 0..3u32 {
        junk(&mut t, n, 0x9d00, k);
        junk(&mut t, e, 0x100, k);
        t.write(e + 0x90, &[k as u8 & 1]);
        junk(&mut t, last, 0x10, k);
        t.set_u32(w(0x28), last);
        t.set_u32(w(0x46), 0x0f0f_0f0f + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, if k == 1 { kid } else { root });
        junk(&mut t, plist, 0x20, k + 1);
        t.set_u32(plist + 0x10, if k == 2 { kid } else { plist });
        let call = Call {
            ecx: n,
            edx: e,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0x9820 + k, &[]);
        let args = [0x0a01 + k, 0x0a02 + k];
        t.check(
            f,
            call,
            &args,
            Eax::Checked,
            ffx_magic::r48::shots_setup_0387,
        );
    }
}

/// `magic_0387` 0x1000d320 (`r48::main_task_0387`): state 0 (into 10,
/// the clock low), 10 low and at 0x28, 11 low and at 0x34 (into 30), 30
/// low and at 0x66 (into 20), 20 low and at 0x12a, 21 counting and out
/// (into 10000, `Q` busy), 10000 with `Q` done, unknown states.
#[test]
fn main_task_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xd320);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let tgt = |o: u32| img.rel32_target(f + o);
    all_stubs(&mut t);
    copy_325(&mut t);
    let list = SCRATCH + 0x3_d000;
    t.write(list, &[3, 5, 7, 9]);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 2);
        list
    });
    for (s, n) in [
        (941u32, 1),
        (676, 3),
        (678, 1),
        (719, 3),
        (291, 5),
        (250, 2),
    ] {
        rec(&mut t, s, n, move |_, a| {
            a.first().copied().unwrap_or(0) ^ (s << 16)
        });
    }
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.25);
    rec(&mut t, 601, 2, |m, a| {
        m.w32(a[1], 1.5f32.to_bits());
        0x601
    });
    t.stub_st0(790, |_, _| 0.375);
    // the colour node's four integers (written by the slot)
    rec(&mut t, 558, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x40 + 0x11 * i);
        }
        0x558
    });
    let row = SCRATCH + 0x3_e000;
    rec(&mut t, 324, 0, move |m, _| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        0x0e01_0000_u32.wrapping_add(0x0a07 * k) ^ (k << 12)
    });
    // the set-ups' globals
    let ss = tgt(0x563);
    let ps = tgt(0x3fb);
    let ls = tgt(0x5a9);
    let la = img.rel32_target(ls + 0x254);
    let gs = img.rel32_target(la + 0x280);
    particle_globals(&mut t, ss, 0x362);
    particle_globals(&mut t, gs, 0x2ce);
    for (fx, ops) in [
        (ss, &[0x339u32][..]),
        (gs, &[0x273][..]),
        (img.rel32_target(ss + 0xad), &[0x55, 0x66, 0x6e][..]),
        (img.rel32_target(ps + 0xd2), &[0x55, 0x66, 0x6e][..]),
        (img.rel32_target(ls + 0x112), &[0x55, 0x66, 0x6e][..]),
        (ps, &[0x1af][..]),
    ] {
        junk_ops(&mut t, fx, ops);
    }
    for tl in [
        img.rel32_target(ss + 0xad),
        img.rel32_target(ps + 0xd2),
        img.rel32_target(ls + 0x112),
    ] {
        for (op, x) in [(0x55u32, 30.0f32), (0x66, 2.0), (0x6e, 0.5)] {
            setf(&mut t, img.word(tl + op), x);
        }
    }
    let (tk, d, prev, next, n) = (
        SCRATCH + 0x100,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
        SCRATCH + 0x1_0000,
    );
    let fx = w(0x77);
    // the centre's targets: two
    let cf = tgt(0x589);
    t.set_u32(img.word(cf + 0x48), 2);
    t.write(img.word(cf + 0x55), &[1, 2]);
    t.write(img.word(cf + 0x75), &[3, 4]);
    rec(&mut t, 531, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, a[0] as f32 + i as f32);
        }
        0x531
    });
    // (state, clock, count, Q's state)
    for (k, (state, clock, count, qs)) in (0u32..).zip([
        (0u32, 0u32, 3u32, 0u32),
        (10, 0x27, 3, 0),
        (10, 0x28, 3, 0),
        (11, 0x33, 3, 0),
        (11, 0x34, 3, 0),
        (0x1e, 0x66, 3, 0),
        (0x14, 0x129, 3, 0),
        (0x14, 0x12a, 3, 0),
        (0x15, 0, 3, 0),
        (0x15, 0, 0, 0),
        (10000, 0, 0, 20000),
        (5, 0, 0, 0),
        (0x20, 0, 0, 0),
    ]) {
        junk(&mut t, n, 0x1_3000, k);
        // the owner's parts (N + 0x24's model nodes, also the pulses' owner)
        t.set_u32(n + 0x24 + 0x4dc, 2);
        for j in 0..2u32 {
            t.set_u32(n + 0x24 + 0x6c + 0x98 * j, 0x0a00 + j);
        }
        t.set_u32(n + 0x18, state);
        t.set_u32(n + 0x1c, clock);
        t.set_u32(n + 0x20, count);
        t.set_u32(n + 0x5c4 + 0x18, qs);
        for off in [0x514u32, 0x24] {
            t.set_u32(n + off + 0x14, 0);
        }
        // the DLL's node globals: junk, then the lists empty
        junk(&mut t, fx, 0x160, k);
        for root in [
            w(0x7f),
            w(0xea),
            w(0x135),
            w(0x269),
            img.word(ss + 0x34c),
            img.word(gs + 0x2b5),
        ] {
            junk(&mut t, root, 0x20, k + 1);
            t.set_u32(root + 0x10, root);
        }
        for op in [0x14c, 0x25] {
            setf(&mut t, w(op), 1.0);
        }
        // (the list's junk covers the centre's count)
        t.set_u32(img.word(cf + 0x48), 2);
        // (b10 test gaps: the effect's colour words finite, its flags word
        // with bit 0 set and bit 2 clear, the list's current node linked)
        floats(&mut t, fx + 0xa0, 0x30, k);
        t.set_u32(w(0x431), 0x0f0f_0f0b + 16 * k);
        let lf = tgt(0x605);
        let cur = SCRATCH + 0x3_d100;
        junk(&mut t, cur, 0x20, k);
        t.set_u32(img.word(lf + 2), if k == 10 { cur } else { 0 });
        // the task list's last task (every set-up links after it)
        junk(&mut t, SCRATCH + 0x200, 0x10, k);
        t.set_u32(w(0x388), SCRATCH + 0x200);
        t.set_u32(tk, d);
        t.set_u32(tk + 4, n);
        for (a, b, c) in [(d, prev, next), (prev, next, d), (next, d, prev)] {
            t.set_u32(a + 4, b);
            t.set_u32(a + 8, c);
        }
        ownb::dirty_stack(&mut t, 0xd320 + k, &[]);
        t.check_entry(f, Call::default(), &[tk], Eax::Checked);
    }
}

/// The free ring of `n` nodes at `nodes` (`free` the count, `head` the
/// ring's node), as r32's tests set it up.
fn free_ring(t: &mut Twin, nodes: u32, (free, head): (u32, u32), nfree: u16, k: u32) {
    for i in 0..3u32 {
        let n = nodes + 0x40 * i;
        junk(t, n, 0x40, k + i);
        t.set_u32(n, nodes + 0x40 * ((i + 2) % 3));
        t.set_u32(n + 4, nodes + 0x40 * ((i + 1) % 3));
    }
    t.set_u16(free, nfree);
    t.set_u32(head, nodes + 0x40);
}

/// `magic_0397` 0x10005020 (`r48::actors_start_0397`): three, one, two
/// and no actors (the yaws and angles wrapping both ways), slot 532
/// answering or not, the free ring with one and more nodes, the task list
/// empty and not.
#[test]
fn actors_start_0397() {
    let Some(mut t) = sa::twin(397) else { return };
    all_stubs(&mut t);
    let f = t.at(0x5020);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        let scale = rf(m, SCRATCH + 0xa04);
        wf(m, a[1], scale * (k - 2.0));
        wf(m, a[2], 0.5 * k + 0.375);
        wf(m, a[3], scale * (3.0 - k));
        0x262
    });
    for s in [12u32, 333, 314] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.625);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.125);
    t.stub_st0(333, |m, a| {
        f64::from(rf(m, SCRATCH + 0xa08)) * f64::from((a[0] & 0xff) + 1)
    });
    rec(&mut t, 532, 1, |_, a| a[0] & 1);
    for s in [528u32, 295] {
        rec(&mut t, s, 2, move |_, a| (s << 16) ^ a[0] ^ a[1]);
    }
    let mode = t.img.rel32_target(f + 0x32f);
    for o in [0x43u32, 0x87, 0x261, 0x30d, 0x340, 0x368] {
        let l = w(&t, o);
        t.set_u32(l, SCRATCH + 0x700);
    }
    t.write(SCRATCH + 0x700, &[3, 1, 4, 1, 5, 2, 6, 0]);
    for o in [0x4eu32, 0x92, 0x26c] {
        for i in 0..8 {
            let at = w(&t, o) + 4 * i;
            t.set_u32(at, 0x0b00_0010 + 3 * i);
        }
    }
    let l = t.img.word(mode + 0x21);
    t.set_u32(l, SCRATCH + 0x700);
    let l = t.img.word(mode + 0x2e);
    for i in 0..8 {
        t.set_u32(l + 4 * i, 0x0c00_0000 + i);
    }
    let s = SCRATCH + 0x1_0000;
    let (gs, gn) = (w(&t, 0x15), w(&t, 0x1c));
    let (free, head, tail, last) = (w(&t, 0x395), w(&t, 0x3a4), w(&t, 0x40b), w(&t, 0x41e));
    let nodes = SCRATCH + 0x9000;
    // (count, place scale, slot 333's angle step, free nodes, list tail)
    let cases: [(i32, f32, f32, u16, u32); 5] = [
        (3, 40.0, 1.5, 3, SCRATCH + 0x9800),
        (1, 0.25, -2.0, 1, 0),
        (2, 9.0, 0.25, 2, SCRATCH + 0x9800),
        (0, 1.0, 1.0, 1, 0),
        (-1, 1.0, 1.0, 2, SCRATCH + 0x9800),
    ];
    for (k, (count, scale, step, nfree, tl)) in (0u32..).zip(cases) {
        t.set_u32(gs, s);
        t.set_u32(gn, count.cast_unsigned());
        junk(&mut t, s + 0x1e00, 0x400, k);
        junk(&mut t, s + 0x23e0, 0x20, k + 1);
        floats(&mut t, s + 0x1900, 4, k);
        t.set_u32(SCRATCH + 0xa04, scale.to_bits());
        t.set_u32(SCRATCH + 0xa08, step.to_bits());
        t.set_u32(SCRATCH + 0xa00, k);
        free_ring(&mut t, nodes, (free, head), nfree, k);
        t.set_u32(tail, tl);
        junk(&mut t, SCRATCH + 0x9800, 0x10, k);
        t.set_u32(last, SCRATCH + 0x9800);
        ownb::dirty_stack(&mut t, 0x5020 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0396` 0x100047d0 (`r48::load_0396`, the r9 load step over
/// `r45::start_up_0396`): busy, loading, loaded and the start-up run.
#[test]
fn load_0396() {
    let Some(mut t) = sa::twin(396) else { return };
    let load = t.at(0x47d0);
    let f = t.img.rel32_target(load + 0x5b);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let fc = t.img.rel32_target(f + 0xee);
    let fw = |o: u32| img.word(fc + o);
    for (k, (busy, loaded, ready)) in
        (0u32..).zip([(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)])
    {
        all_stubs(&mut t);
        for (s, n) in [(719u32, 3usize), (710, 4), (271, 1), (711, 3)] {
            rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0] ^ k);
        }
        rec(&mut t, 262, 4, |m, a| {
            for i in 1..4 {
                wf(m, a[i], (a[0] & 0xf) as f32 * 0.75 + i as f32 - 3.0);
            }
            0x262
        });
        rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
        t.stub_st0(333, move |_, _| 0.5 + f64::from(k) * 2.75 - 4.0);
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let state = SCRATCH + 0x3000;
        junk(&mut t, state, 0x400, k);
        t.set_u32(fw(0x08), state);
        t.set_u32(fw(0x22), 0x0c0c_0022);
        let lock = SCRATCH + 0x5e0;
        t.set_u32(w(0x74), lock);
        t.set_u32(lock, 0x77 + k);
        magic_r11::ring(&mut t, w(0x128), w(0x13f), 5, SCRATCH + 0x5000);
        t.write(SCRATCH + 0x600, &[0x44; 0x20]);
        t.set_u32(w(0x1a1), SCRATCH + 0x600);
        t.set_u32(w(0x1b4), SCRATCH + 0x600);
        junk_ops(
            &mut t,
            f,
            &[0xb, 0x15, 0x1f, 0x29, 0xc7, 0xd1, 0xdb, 0xe5, 0xf9, 0x103],
        );
        let at = t.img.word(load + 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = t.img.word(load + o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(&mut t, load, &[0x61, 0x6b]);
        ownb::dirty_stack(&mut t, 0x47d0 + k, &[]);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0397` 0x100058d0 (`r48::landing_0397`): three, one and no
/// actors; slot 532 answering for some (the other start), in and out of a
/// battle; the free ring with enough nodes, the task list empty and not.
#[test]
fn landing_0397() {
    let Some(mut t) = sa::twin(397) else { return };
    all_stubs(&mut t);
    let f = t.at(0x58d0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    magic_r11::object_slots(&mut t);
    magic_r11::object_start_auto(&mut t, img.rel32_target(f + 0x272));
    rec(&mut t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        wf(m, a[1], 2.0 * (k - 2.0));
        wf(m, a[2], 0.5 * k + 0.375);
        wf(m, a[3], 1.5 * (3.0 - k));
        0x262
    });
    for s in [12u32, 314, 277, 15, 16, 17] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xf) * 0.625);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.125);
    for s in [277u32, 15, 16, 17] {
        t.stub_st0(s, move |_, a| f64::from((a[0] & 7) + s) * 0.0625);
    }
    rec(&mut t, 532, 1, |_, a| a[0] & 1);
    for s in [330u32, 328, 329, 268] {
        rec(&mut t, s, 3, move |m, a| {
            let x = m.r32(a[0]) ^ a[2] ^ s;
            m.w32(a[0], x);
            s
        });
    }
    rec(&mut t, 295, 2, |_, a| 0x0295_0000 ^ a[0] ^ a[1]);
    for o in [0x5du32, 0x22a, 0x2d4, 0x4aa, 0x519, 0x583] {
        t.set_u32(w(o), SCRATCH + 0x700);
    }
    t.write(SCRATCH + 0x700, &[3, 2, 4, 1, 5, 2, 6, 0]);
    for o in [0x68u32, 0x524] {
        for i in 0..8 {
            t.set_u32(w(o) + 4 * i, 0x0b00_0010 + 3 * i);
        }
    }
    let sc = img.rel32_target(f + 0x121);
    for o in [0x28u32, 0x74] {
        t.set_u32(img.word(sc + o), SCRATCH + 0x700);
    }
    for o in [0x33u32, 0x7f] {
        for i in 0..8 {
            t.set_u32(img.word(sc + o) + 4 * i, 0x0b00_0010 + 3 * i);
        }
    }
    junk_ops(&mut t, f, &[0xaf, 0x1c9, 0x254, 0x2ac, 0x341, 0x4ee]);
    for (op, x) in [
        (0xaf, 0.5f32),
        (0x1c9, 4.0),
        (0x254, 1.25),
        (0x2ac, 0.75),
        (0x341, 2.5),
        (0x4ee, 3.0),
    ] {
        setf(&mut t, w(op), x);
    }
    t.set_u32(w(0x1d7), 0x8000_0000);
    let s = SCRATCH + 0x1_0000;
    let nodes = SCRATCH + 0x5000;
    // (count, battle, listed)
    for (k, (count, battle, listed)) in (0u32..).zip([
        (3i32, 1u32, false),
        (1, 0, true),
        (0, 1, true),
        (2, 0, false),
        // (b13 test gap: no actors and an empty list: the settle task heads it)
        (0, 0, false),
    ]) {
        t.set_u32(w(0x13), s);
        t.set_u32(w(0x27), count.cast_unsigned());
        junk(&mut t, s + 0x1800, 0xc00, k);
        // the sway task given back: no children, no links
        let back = SCRATCH + 0x4000;
        junk(&mut t, back, 0x20, k);
        t.set_u32(back, 0);
        t.set_u32(back + 4, 0);
        t.set_u32(back + 8, back);
        t.set_u32(back + 0xc, 0);
        t.set_u32(s + 0x23f0, back);
        t.set_u32(w(0x2f2), battle);
        magic_r11::pools(&mut t, f, 1941);
        magic_r11::ring(&mut t, w(0x3af), w(0x3bb), 8, nodes);
        t.write(SCRATCH + 0x600, &[0x44; 0x20]);
        t.set_u32(w(0x429), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(0x43f), SCRATCH + 0x600);
        ownb::dirty_stack(&mut t, 0x58d0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0431` 0x10003ef0 (`r48::orbs_start_0431`): a task to give back
/// and none; the task list empty and not.
#[test]
fn orbs_start_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    all_stubs(&mut t);
    let f = t.at(0x3ef0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    magic_r11::object_slots(&mut t);
    magic_r11::object_start_auto(&mut t, img.rel32_target(f + 0x186));
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 0.75);
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + (a[2] & 0xff) as f32;
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            wf(m, a[1] + 4 * i, (a[0] & 0xf) as f32 + i as f32 * 0.25);
        }
        0x210
    });
    for (s, n) in [(257u32, 2), (239, 3)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0] ^ a[1]);
    }
    t.set_u32(w(0x36), SCRATCH + 0x700);
    t.write(SCRATCH + 0x700, &[3, 2]);
    for i in 0..4 {
        t.set_u32(w(0x43) + 4 * i, 0x0b00_0010 + 3 * i);
    }
    junk_ops(&mut t, f, &[0x122, 0x175, 0x181, 0x253]);
    for (op, x) in [(0x122, 3.0f32), (0x175, 0.5), (0x181, 1.5)] {
        setf(&mut t, w(op), x);
    }
    let s = SCRATCH + 0x1_0000;
    let pts = SCRATCH + 0x6000;
    for (k, (back, listed)) in (0u32..).zip([(true, false), (false, true), (true, true)]) {
        t.set_u32(w(0x18), s);
        junk(&mut t, s, 0xd40, k);
        let bn = SCRATCH + 0x4000;
        junk(&mut t, bn, 0x20, k);
        t.set_u32(bn, 0);
        t.set_u32(bn + 4, 0);
        t.set_u32(bn + 8, bn);
        t.set_u32(bn + 0xc, 0);
        t.set_u32(s + 0xc64, if back { bn } else { 0 });
        floats(&mut t, pts, 0x30, k);
        for j in 0..3u32 {
            let cp = s + 0xc94 + 0x30 * j;
            t.set_u32(cp - 8, 0x10 + j);
            t.set_u32(cp - 4, pts + 0x10 * j);
            t.set_u32(cp, pts + 0x40 + 0x10 * j);
            t.set_u32(cp + 4, pts + 0x80 + 0x10 * j);
        }
        magic_r11::pools(&mut t, f, 815);
        magic_r11::ring(&mut t, w(0x25c), w(0x282), 4, SCRATCH + 0x5000);
        t.write(SCRATCH + 0x600, &[0x44; 0x20]);
        t.set_u32(w(0x2e9), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(0x301), SCRATCH + 0x600);
        ownb::dirty_stack(&mut t, 0x3ef0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0431` 0x10001a90 (`r48::model_bounds_0431`, internal): no
/// models; two models with vertices, triangle groups (type 0x10) and a
/// group of another type, none or several primitives.
#[test]
fn model_bounds_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x1a90);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let models = SCRATCH + 0x8000;
    rec(&mut t, 941, 1, move |_, a| models + 0x800 * (a[0] & 3));
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x230
    });
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 229, 3, |m, a| {
        let mut out = [0.0f32; 4];
        for i in 0..4 {
            for j in 0..4 {
                out[i as usize] += rf(m, a[1] + 16 * j + 4 * i) * rf(m, a[2] + 4 * j);
            }
        }
        out[3] = out[3].abs() + 1.0;
        for i in 0..4 {
            wf(m, a[0] + 4 * i, out[i as usize]);
        }
        0x229
    });
    junk_ops(&mut t, f, &[0xee, 0x172]);
    setf(&mut t, w(0xee), 0.5);
    setf(&mut t, w(0x172), 2.0);
    let (set, xm, buf, lo, hi) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x4000,
        SCRATCH + 0x300,
        SCRATCH + 0x320,
    );
    // a model at `models + 0x800 n`: `+4` the groups, `+8` the vertices
    let model = |t: &mut Twin, n: u32, nv: u16, groups: &[(u8, i16)], k: u32| {
        let at = models + 0x800 * n;
        junk(t, at, 0x800, k + n);
        t.set_u32(at + 4, 0x40);
        t.set_u32(at + 8, 0x400);
        t.write(at + 0x12, &nv.to_le_bytes());
        let prims: i16 = groups.iter().map(|g| g.1).sum();
        t.write(at + 0x10, &prims.to_le_bytes());
        floats(t, at + 0x400, 4 * u32::from(nv), k + n);
        let mut p = at + 0x40;
        for &(kind, cnt) in groups {
            t.write(p + 1, &[kind]);
            t.write(p + 2, &cnt.to_le_bytes());
            p += 0x20;
            if kind == 0x10 {
                for tn in 0..cnt.max(0) as u32 {
                    for c in 0..3u32 {
                        let idx = ((tn * 3 + c + k) % u32::from(nv.max(1))) as u16;
                        t.write(p + 2 * c, &idx.to_le_bytes());
                    }
                    p += 0x1c;
                }
            }
        }
        // (b15 test gap: a triangle group past the counted ones, on a
        // vertex past the model's, never read)
        t.write(p + 1, &[0x10]);
        t.write(p + 2, &1i16.to_le_bytes());
        for c in 0..3u32 {
            t.write(p + 0x20 + 2 * c, &(nv + 3).to_le_bytes());
        }
    };
    for (k, n) in (0u32..).zip([0u16, 2, 1]) {
        junk(&mut t, set, 0x40, k);
        t.write(set + 4, &n.to_le_bytes());
        for i in 0..u32::from(n) {
            t.set_u32(set + 0x10 + 0x10 * i, i + 1);
            t.set_u32(set + 0x14 + 0x10 * i, 3 - i);
        }
        model(&mut t, 1, 6, &[(0x10, 2), (0x07, 3), (0x10, 1)], k);
        model(&mut t, 2, 4, &[(0x10, 3)], k);
        floats(&mut t, models + 0x800 * 3, 16, k);
        floats(&mut t, models + 0x800 * 2 + 0x400, 16, k);
        floats(&mut t, xm, 16, k + 1);
        for i in 0..0x80u32 {
            setf(&mut t, buf + 4 * i, if i % 2 == 0 { 900.0 } else { -900.0 });
        }
        junk(&mut t, lo, 0x40, k);
        let call = Call {
            ecx: set,
            edx: xm,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0x1a90 + k, &[]);
        t.check(
            f,
            call,
            &[buf, lo, hi],
            Eax::Checked,
            ffx_magic::r48::model_bounds_0431,
        );
    }
}

/// `magic_0431` 0x10001ee0 (`r48::facing_tris_0431`, internal): the
/// tests on and off (front, aimed, scaled), the reach small and large,
/// the height limit low and high, two models (a type-0x10 group and
/// another).
#[test]
fn facing_tris_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x1ee0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let models = SCRATCH + 0x8000;
    rec(&mut t, 941, 1, move |_, a| models + 0x800 * (a[0] & 3));
    rec(&mut t, 232, 2, |m, a| {
        let v: Vec<f32> = (0..3).map(|i| rf(m, a[1] + 4 * i)).collect();
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, v[i] / n);
        }
        0x232
    });
    let ops = [
        0x41u32, 0x1b2, 0x2bd, 0x394, 0x3a3, 0x3ad, 0x3b7, 0x3c4, 0x3d1, 0x3db, 0x3f1,
    ];
    junk_ops(&mut t, f, &ops);
    for (op, x) in [
        (0x41u32, 0.5f32),
        (0x1b2, 3.0),
        (0x2bd, 1.0),
        (0x394, 1.5),
        (0x3a3, std::f32::consts::PI),
        (0x3ad, std::f32::consts::TAU),
        (0x3b7, -std::f32::consts::PI),
        (0x3c4, std::f32::consts::TAU),
        (0x3d1, 1.2),
        (0x3db, -1.2),
        (0x3f1, 0.9),
    ] {
        setf(&mut t, w(op), x);
    }
    let (set, out, eye, buf, a0, a1) = (
        SCRATCH + 0x100,
        SCRATCH + 0x2000,
        SCRATCH + 0x200,
        SCRATCH + 0x4000,
        SCRATCH + 0x300,
        SCRATCH + 0x320,
    );
    t.set_u32(w(0x47), set);
    let model = |t: &mut Twin, n: u32, nv: u16, groups: &[(u8, i16)], k: u32| {
        let at = models + 0x800 * n;
        junk(t, at, 0x800, k + n);
        t.set_u32(at + 4, 0x40);
        t.set_u32(at + 8, 0x400);
        t.write(at + 0x12, &nv.to_le_bytes());
        let prims: i16 = groups.iter().map(|g| g.1).sum();
        t.write(at + 0x10, &prims.to_le_bytes());
        let mut p = at + 0x40;
        for &(kind, cnt) in groups {
            t.write(p + 1, &[kind]);
            t.write(p + 2, &cnt.to_le_bytes());
            p += 0x20;
            if kind == 0x10 {
                for tn in 0..cnt.max(0) as u32 {
                    for c in 0..3u32 {
                        let idx = ((tn * 2 + c * (1 + k) + k) % u32::from(nv.max(1))) as u16;
                        t.write(p + 2 * c, &idx.to_le_bytes());
                    }
                    p += 0x1c;
                }
            }
        }
    };
    // (front, aimed, scaled, reach, A1.y)
    for (k, (front, aimed, scaled, reach, top)) in (0u32..).zip([
        (0u32, 0u32, 0u32, 100.0f32, 50.0f32),
        (1, 0, 0, 100.0, 50.0),
        (0, 1, 0, 100.0, 50.0),
        (0, 0, 1, 100.0, 1.0),
        (1, 1, 1, 4.0, 50.0),
        (0, 0, 0, 100.0, 1.0),
        (0, 0, 0, 100.0, -50.0),
        // (b16 test gaps: reaches and tests splitting the triangles)
        (0, 0, 0, 3.0, 50.0),
        (0, 0, 0, 5.0, 50.0),
        (0, 0, 0, 7.0, 50.0),
        (1, 0, 0, 100.0, 50.0),
        (0, 1, 0, 100.0, 50.0),
        (1, 1, 0, 6.0, 0.5),
        (0, 1, 1, 100.0, 2.0),
        (0, 0, 0, 2.0, 50.0),
        (0, 0, 0, 4.0, 50.0),
        (0, 0, 0, 6.0, 50.0),
        (0, 0, 0, 8.0, 50.0),
        (0, 1, 0, 100.0, 50.0),
        (0, 1, 0, 100.0, 50.0),
        (0, 1, 0, 100.0, 50.0),
    ]) {
        // (b17 test gap: narrow aim windows in the last three, so the
        // normal's yaw and pitch decide)
        let narrow = k >= 18;
        let win = [
            (1.2f32, -1.2f32, 0.9f32),
            (2.5, 0.8, 0.6),
            (-0.5, -2.5, 0.3),
            (1.2, -0.5, 0.9),
        ];
        let (k6, k7, k8) = win[if narrow { (k - 17) as usize } else { 0 }];
        setf(&mut t, w(0x3d1), k6);
        setf(&mut t, w(0x3db), k7);
        setf(&mut t, w(0x3f1), k8);
        junk(&mut t, set, 0x40, k);
        t.write(set + 4, &2u16.to_le_bytes());
        for i in 0..2u32 {
            t.set_u32(set + 0x10 + 0x10 * i, i + 1);
            t.set_u32(set + 0x14 + 0x10 * i, 3 - i);
        }
        model(&mut t, 1, 6, &[(0x10, 4), (0x07, 3), (0x10, 3)], k);
        model(&mut t, 2, 5, &[(0x10, 5)], k);
        for i in 0..0x80u32 {
            let v = ((i * 7 + k * 5) % 23) as f32 * 0.5 - 5.5;
            setf(&mut t, buf + 4 * i, v);
        }
        floats(&mut t, eye, 4, k);
        floats(&mut t, a0, 4, k + 1);
        floats(&mut t, a1, 4, k + 2);
        setf(&mut t, a1 + 4, top);
        junk(&mut t, out, 0x200, k);
        let call = Call {
            ecx: out,
            edx: eye,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0x1ee0 + k, &[]);
        let args = [a0, a1, buf, reach.to_bits(), scaled, aimed, front];
        t.check(
            f,
            call,
            &args,
            Eax::Checked,
            ffx_magic::r48::facing_tris_0431,
        );
    }
}

/// `magic_0431` 0x100023a0 (`r48::tri_pick_0431`, internal): too few
/// primitives; enough, with the reach and height putting the first pass
/// over three and running all five.
#[test]
fn tri_pick_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x23a0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let ft = img.rel32_target(f + 0x68);
    let wt = |o: u32| img.word(ft + o);
    let fb = img.rel32_target(f + 0x36);
    let cr = img.rel32_target(f + 0xf);
    all_stubs(&mut t);
    let models = SCRATCH + 0x8000;
    let buf = SCRATCH + 0x4000;
    rec(&mut t, 941, 1, move |_, a| models + 0x800 * (a[0] & 3));
    rec(&mut t, 562, 0, move |_, _| buf);
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x230
    });
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 229, 3, |m, a| {
        let mut out = [0.0f32; 4];
        for i in 0..4 {
            for j in 0..4 {
                out[i as usize] += rf(m, a[1] + 16 * j + 4 * i) * rf(m, a[2] + 4 * j);
            }
        }
        out[3] = out[3].abs() + 1.0;
        for i in 0..4 {
            wf(m, a[0] + 4 * i, out[i as usize]);
        }
        0x229
    });
    rec(&mut t, 232, 2, |m, a| {
        let v: Vec<f32> = (0..3).map(|i| rf(m, a[1] + 4 * i)).collect();
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, v[i] / n);
        }
        0x232
    });
    t.set_u32(img.word(cr + 1), SCRATCH + 0x700);
    t.write(SCRATCH + 0x700, &[2, 1]);
    junk_ops(&mut t, cr, &[0x11, 0x4c]);
    junk_ops(&mut t, fb, &[0xee, 0x172]);
    setf(&mut t, img.word(fb + 0xee), 0.5);
    setf(&mut t, img.word(fb + 0x172), 2.0);
    for (op, x) in [
        (0x41u32, 0.5f32),
        (0x1b2, 3.0),
        (0x2bd, 1.0),
        (0x394, 1.5),
        (0x3a3, std::f32::consts::PI),
        (0x3ad, std::f32::consts::TAU),
        (0x3b7, -std::f32::consts::PI),
        (0x3c4, std::f32::consts::TAU),
        (0x3d1, 1.2),
        (0x3db, -1.2),
        (0x3f1, 0.9),
    ] {
        setf(&mut t, wt(op), x);
    }
    let (set, out, view, eye) = (
        SCRATCH + 0x100,
        SCRATCH + 0x2000,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
    );
    t.set_u32(w(0x1a), set);
    t.set_u32(wt(0x47), set);
    // (b17 test gap: exactly three primitives)
    for (k, (prims, spread)) in (0u32..).zip([(2i16, 1.0f32), (8, 1.0), (8, 40.0), (3, 1.0)]) {
        junk(&mut t, set, 0x40, k);
        t.write(set + 4, &1u16.to_le_bytes());
        t.set_u32(set + 0x10, 1);
        t.set_u32(set + 0x14, 3);
        let at = models + 0x800;
        junk(&mut t, at, 0x800, k);
        t.set_u32(at + 4, 0x40);
        t.set_u32(at + 8, 0x400);
        t.write(at + 0x12, &6u16.to_le_bytes());
        t.write(at + 0x10, &prims.to_le_bytes());
        t.write(at + 0x41, &[0x10]);
        t.write(at + 0x42, &prims.to_le_bytes());
        for tn in 0..prims as u32 {
            for c in 0..3u32 {
                t.write(
                    at + 0x60 + 0x1c * tn + 2 * c,
                    &(((tn + c * 2) % 6) as u16).to_le_bytes(),
                );
            }
        }
        for i in 0..24u32 {
            let v = ((i * 7 + k * 5) % 23) as f32 * 0.25 * spread - 2.75 * spread;
            setf(&mut t, at + 0x400 + 4 * i, v);
        }
        floats(&mut t, models + 0x800 * 3, 16, k);
        for i in 0..16u32 {
            setf(&mut t, view + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        floats(&mut t, eye, 4, k);
        junk(&mut t, out, 0x200, k);
        let call = Call {
            ecx: out,
            edx: view,
            ..Call::default()
        };
        ownb::dirty_stack(&mut t, 0x23a0 + k, &[]);
        t.check(f, call, &[eye], Eax::Checked, ffx_magic::r48::tri_pick_0431);
    }
}

/// `magic_0431` 0x100024c0 (`r48::orbs_aim_0431`, internal): too few
/// triangles (the three orbs aimed by the fallback) and enough (drawn
/// from each third, `rand` landing on used and unused records).
#[test]
fn orbs_aim_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x24c0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let tp = img.rel32_target(f + 0x373);
    let ft = img.rel32_target(tp + 0x68);
    let wt = |o: u32| img.word(ft + o);
    let fb = img.rel32_target(tp + 0x36);
    let cr = img.rel32_target(tp + 0xf);
    all_stubs(&mut t);
    let models = SCRATCH + 0x8000;
    let buf = SCRATCH + 0x1_8000;
    rec(&mut t, 941, 1, move |_, a| models + 0x800 * (a[0] & 3));
    rec(&mut t, 562, 0, move |_, _| buf);
    for s in [230u32, 210] {
        rec(&mut t, s, 2, move |m, a| {
            let (dst, src) = if s == 230 { (a[0], a[1]) } else { (a[1], a[0]) };
            for i in 0..16 {
                let x = ((src >> 4) & 0xf) as f32 * 0.125 + i as f32 * 0.0625;
                wf(m, dst + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [231u32, 329, 328, 248] {
        rec(&mut t, s, 3, move |m, a| {
            let n = if s == 248 { 4 } else { 16 };
            for i in 0..n {
                let y = if s == 231 || s == 248 {
                    rf(m, a[2] + 4 * i)
                } else {
                    f32::from_bits(a[2])
                };
                let x = rf(m, a[1] + 4 * i) * 0.5 + y;
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        let mut out = [0.0f32; 4];
        for i in 0..4 {
            for j in 0..4 {
                out[i as usize] += rf(m, a[1] + 16 * j + 4 * i) * rf(m, a[2] + 4 * j);
            }
        }
        out[3] = out[3].abs() + 1.0;
        for i in 0..4 {
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
            wf(m, a[0] + 4 * i, out[i as usize]);
        }
        0x229
    });
    rec(&mut t, 232, 2, |m, a| {
        let v: Vec<f32> = (0..3).map(|i| rf(m, a[1] + 4 * i)).collect();
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, v[i] / n);
        }
        0x232
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xf) as f32 * 0.75 + i as f32 - 3.0);
        }
        0x262
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(
                m,
                a[2] + 4 * i,
                (a[0] & 7) as f32 * 0.5 + (a[1] + i) as f32 * 0.25,
            );
        }
        0x268
    });
    rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x3_e000);
        m.w32(SCRATCH + 0x3_e000, k + 1);
        f64::from(f32::from_bits(a[0])) * (f64::from(k % 7) / 7.0 - 0.5)
    });
    rec(&mut t, 257, 2, |_, a| 0x0257_0000 ^ a[0] ^ a[1]);
    for o in [0x34u32, 0xd1, 0x2e6, 0x38a, 0x6ac, 0x649, 0xb0b] {
        t.set_u32(w(o), SCRATCH + 0x700);
    }
    t.write(SCRATCH + 0x700, &[3, 2, 1, 0, 4, 5]);
    for o in [0x4cu32, 0xf2, 0x30b, 0x39e, 0x6b6, 0x654, 0xb16] {
        for i in 0..8 {
            t.set_u32(w(o) + 4 * i, 0x0b00_0010 + 3 * i);
        }
    }
    t.set_u32(w(0x64), 0x0b00_0099);
    // the pick's set-up (tri_pick_0431's test)
    t.set_u32(img.word(cr + 1), SCRATCH + 0x700);
    junk_ops(&mut t, cr, &[0x11, 0x4c]);
    setf(&mut t, img.word(fb + 0xee), 0.5);
    setf(&mut t, img.word(fb + 0x172), 2.0);
    for (op, x) in [
        (0x41u32, 0.5f32),
        (0x1b2, 3.0),
        (0x2bd, 1.0),
        (0x394, 1.5),
        (0x3a3, std::f32::consts::PI),
        (0x3ad, std::f32::consts::TAU),
        (0x3b7, -std::f32::consts::PI),
        (0x3c4, std::f32::consts::TAU),
        (0x3d1, 3.0),
        (0x3db, -3.0),
        (0x3f1, 3.0),
    ] {
        setf(&mut t, wt(op), x);
    }
    for (op, x) in [
        (0x1ff, 1.5f32),
        (0x216, std::f32::consts::PI),
        (0x20f, std::f32::consts::TAU),
        (0x22c, -std::f32::consts::PI),
        (0x26c, 0.25),
        (0x857, 0.5),
        (0xaba, 1.5),
    ] {
        setf(&mut t, w(op), x);
    }
    for i in 0..4u32 {
        setf(&mut t, w(0x9f6) + 4 * i, 1.0 + i as f32 * 0.5);
    }
    let set = SCRATCH + 0x100;
    t.set_u32(img.word(tp + 0x1a), set);
    t.set_u32(wt(0x47), set);
    let s0 = SCRATCH + 0x1_0000;
    t.set_u32(w(0x1b), s0);
    for (k, prims) in (0u32..).zip([2i16, 12, 9]) {
        junk(&mut t, s0, 0xd40, k);
        junk(&mut t, set, 0x40, k);
        t.write(set + 4, &1u16.to_le_bytes());
        t.set_u32(set + 0x10, 1);
        t.set_u32(set + 0x14, 3);
        let at = models + 0x800;
        junk(&mut t, at, 0x800, k);
        t.set_u32(at + 4, 0x40);
        t.set_u32(at + 8, 0x400);
        t.write(at + 0x12, &6u16.to_le_bytes());
        t.write(at + 0x10, &prims.to_le_bytes());
        t.write(at + 0x41, &[0x10]);
        t.write(at + 0x42, &prims.to_le_bytes());
        for tn in 0..prims as u32 {
            for c in 0..3u32 {
                t.write(
                    at + 0x60 + 0x1c * tn + 2 * c,
                    &(((tn + c * 2) % 6) as u16).to_le_bytes(),
                );
            }
        }
        for i in 0..24u32 {
            let v = ((i * 7 + k * 5) % 23) as f32 * 0.25 - 2.75;
            setf(&mut t, at + 0x400 + 4 * i, v);
        }
        floats(&mut t, models + 0x800 * 3, 16, k);
        junk(&mut t, buf + 0x8_0000, 0x200, k);
        ownb::dirty_stack(&mut t, 0x24c0 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r48::orbs_aim_0431,
        );
    }
}

/// `magic_0431` 0x100031b0 (`r48::orbs_setup_0431`): the sounds, the
/// matrices, the actors' motions and the orbs' aim (too few triangles and
/// enough), the task list empty and not.
#[test]
fn orbs_setup_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f0 = t.at(0x31b0);
    let img = t.img.clone();
    let w0 = |o: u32| img.word(f0 + o);
    let f = img.rel32_target(f0 + 0x1a7);
    let w = |o: u32| img.word(f + o);
    let tp = img.rel32_target(f + 0x373);
    let ft = img.rel32_target(tp + 0x68);
    let wt = |o: u32| img.word(ft + o);
    let fb = img.rel32_target(tp + 0x36);
    let cr = img.rel32_target(tp + 0xf);
    all_stubs(&mut t);
    let models = SCRATCH + 0x8000;
    let buf = SCRATCH + 0x1_8000;
    rec(&mut t, 941, 1, move |_, a| models + 0x800 * (a[0] & 3));
    rec(&mut t, 562, 0, move |_, _| buf);
    for s in [230u32, 210] {
        rec(&mut t, s, 2, move |m, a| {
            let (dst, src) = if s == 230 { (a[0], a[1]) } else { (a[1], a[0]) };
            for i in 0..16 {
                let x = ((src >> 4) & 0xf) as f32 * 0.125 + i as f32 * 0.0625;
                wf(m, dst + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [231u32, 329, 328, 248] {
        rec(&mut t, s, 3, move |m, a| {
            let n = if s == 248 { 4 } else { 16 };
            for i in 0..n {
                let y = if s == 231 || s == 248 {
                    rf(m, a[2] + 4 * i)
                } else {
                    f32::from_bits(a[2])
                };
                let x = rf(m, a[1] + 4 * i) * 0.5 + y;
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        let mut out = [0.0f32; 4];
        for i in 0..4 {
            for j in 0..4 {
                out[i as usize] += rf(m, a[1] + 16 * j + 4 * i) * rf(m, a[2] + 4 * j);
            }
        }
        out[3] = out[3].abs() + 1.0;
        for i in 0..4 {
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
            wf(m, a[0] + 4 * i, out[i as usize]);
        }
        0x229
    });
    rec(&mut t, 232, 2, |m, a| {
        let v: Vec<f32> = (0..3).map(|i| rf(m, a[1] + 4 * i)).collect();
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, v[i] / n);
        }
        0x232
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], (a[0] & 0xf) as f32 * 0.75 + i as f32 - 3.0);
        }
        0x262
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(
                m,
                a[2] + 4 * i,
                (a[0] & 7) as f32 * 0.5 + (a[1] + i) as f32 * 0.25,
            );
        }
        0x268
    });
    rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x3_e000);
        m.w32(SCRATCH + 0x3_e000, k + 1);
        f64::from(f32::from_bits(a[0])) * (f64::from(k % 7) / 7.0 - 0.5)
    });
    rec(&mut t, 257, 2, |_, a| 0x0257_0000 ^ a[0] ^ a[1]);
    for o in [0x34u32, 0xd1, 0x2e6, 0x38a, 0x6ac, 0x649, 0xb0b] {
        t.set_u32(w(o), SCRATCH + 0x700);
    }
    t.write(SCRATCH + 0x700, &[3, 2, 1, 0, 4, 5]);
    for o in [0x4cu32, 0xf2, 0x30b, 0x39e, 0x6b6, 0x654, 0xb16] {
        for i in 0..8 {
            t.set_u32(w(o) + 4 * i, 0x0b00_0010 + 3 * i);
        }
    }
    t.set_u32(w(0x64), 0x0b00_0099);
    // the pick's set-up (tri_pick_0431's test)
    t.set_u32(img.word(cr + 1), SCRATCH + 0x700);
    junk_ops(&mut t, cr, &[0x11, 0x4c]);
    setf(&mut t, img.word(fb + 0xee), 0.5);
    setf(&mut t, img.word(fb + 0x172), 2.0);
    for (op, x) in [
        (0x41u32, 0.5f32),
        (0x1b2, 3.0),
        (0x2bd, 1.0),
        (0x394, 1.5),
        (0x3a3, std::f32::consts::PI),
        (0x3ad, std::f32::consts::TAU),
        (0x3b7, -std::f32::consts::PI),
        (0x3c4, std::f32::consts::TAU),
        (0x3d1, 3.0),
        (0x3db, -3.0),
        (0x3f1, 3.0),
    ] {
        setf(&mut t, wt(op), x);
    }
    for (op, x) in [
        (0x1ff, 1.5f32),
        (0x216, std::f32::consts::PI),
        (0x20f, std::f32::consts::TAU),
        (0x22c, -std::f32::consts::PI),
        (0x26c, 0.25),
        (0x857, 0.5),
        (0xaba, 1.5),
    ] {
        setf(&mut t, w(op), x);
    }
    for i in 0..4u32 {
        setf(&mut t, w(0x9f6) + 4 * i, 1.0 + i as f32 * 0.5);
    }
    let set = SCRATCH + 0x100;
    t.set_u32(img.word(tp + 0x1a), set);
    t.set_u32(wt(0x47), set);
    let s0 = SCRATCH + 0x1_0000;
    t.set_u32(w(0x1b), s0);
    for (o, at) in [(0x115u32, SCRATCH + 0x700), (0x1ac, SCRATCH + 0x700)] {
        t.set_u32(w0(o), at);
    }
    for o in [0x12du32, 0x1b9] {
        for i in 0..8 {
            t.set_u32(w0(o) + 4 * i, 0x0b00_0010 + 3 * i);
        }
    }
    rec(&mut t, 0, 1, |_, a| 0x0c00_0000 ^ a[0]);
    for (s, n) in [
        (26u32, 2),
        (10, 2),
        (211, 2),
        (357, 3),
        (356, 3),
        (362, 4),
        (399, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    junk_ops(&mut t, f0, &[0x195]);
    let take = img.rel32_target(f0 + 0x1e0);
    for (k, (prims, listed)) in (0u32..).zip([(2i16, false), (12, true)]) {
        junk(&mut t, s0, 0xd40, k);
        junk(&mut t, set, 0x40, k);
        t.write(set + 4, &1u16.to_le_bytes());
        t.set_u32(set + 0x10, 1);
        t.set_u32(set + 0x14, 3);
        let at = models + 0x800;
        junk(&mut t, at, 0x800, k);
        t.set_u32(at + 4, 0x40);
        t.set_u32(at + 8, 0x400);
        t.write(at + 0x12, &6u16.to_le_bytes());
        t.write(at + 0x10, &prims.to_le_bytes());
        t.write(at + 0x41, &[0x10]);
        t.write(at + 0x42, &prims.to_le_bytes());
        for tn in 0..prims as u32 {
            for c in 0..3u32 {
                t.write(
                    at + 0x60 + 0x1c * tn + 2 * c,
                    &(((tn + c * 2) % 6) as u16).to_le_bytes(),
                );
            }
        }
        for i in 0..24u32 {
            let v = ((i * 7 + k * 5) % 23) as f32 * 0.25 - 2.75;
            setf(&mut t, at + 0x400 + 4 * i, v);
        }
        floats(&mut t, models + 0x800 * 3, 16, k);
        junk(&mut t, buf + 0x8_0000, 0x200, k);
        magic_r11::ring(
            &mut t,
            img.word(take + 0x04),
            img.word(take + 0x17),
            4,
            SCRATCH + 0x5000,
        );
        t.write(SCRATCH + 0x600, &[0x44; 0x20]);
        t.set_u32(w0(0x1e6), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w0(0x1ff), SCRATCH + 0x600);
        ownb::dirty_stack(&mut t, 0x31b0 + k, &[]);
        t.check_entry(f0, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0431` 0x100035a0 (`r48::orbs_task_0431`): the delays counting
/// (first, second, both out); three orbs with corners giving normals
/// whose angles wrap.
#[test]
fn orbs_task_0431() {
    let Some(mut t) = sa::twin(431) else { return };
    let f = t.at(0x35a0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let gd = img.rel32_target(f + 0x763);
    all_stubs(&mut t);
    for s in [723u32, 724] {
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(
                    m,
                    a[0] + 4 * i,
                    (i as f32 + 1.0) * if s == 723 { 1.0 } else { 0.5 },
                );
            }
            s
        });
    }
    rec(&mut t, 544, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, a[0] as f32 + i as f32 * 0.25);
        }
        0x544
    });
    rec(&mut t, 567, 5, |m, a| {
        let mut x = 0x567u32;
        for i in 0..0x14 {
            x = x.rotate_left(3) ^ m.r32(a[1] + 4 * i);
        }
        // the glow's matrix and record, kept where the comparison sees them
        for i in 0..16 {
            let v = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3_d800 + 4 * i, v);
        }
        for i in 0..0x14 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(SCRATCH + 0x3_d900 + 4 * i, v);
        }
        0x567
    });
    // (slots 562 and 545 read no words: the answers ignore the stack)
    for s in [562u32, 545] {
        rec(&mut t, s, 0, move |_, _| s << 16);
    }
    rec(&mut t, 553, 5, |_, a| 0x0553_0000 ^ a[0]);
    for s in [230u32, 210] {
        rec(&mut t, s, 2, move |m, a| {
            let (dst, src) = if s == 230 { (a[0], a[1]) } else { (a[1], a[0]) };
            for i in 0..16 {
                let x = ((src >> 4) & 0xf) as f32 * 0.125 + i as f32 * 0.0625;
                wf(m, dst + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            wf(m, a[0] + 4 * i, if i % 5 == 0 { 1.0 } else { 0.0 });
        }
        0x235
    });
    for s in [231u32, 329, 328] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..16 {
                let y = if s == 231 {
                    rf(m, a[2] + 4 * i)
                } else {
                    f32::from_bits(a[2])
                };
                let x = rf(m, a[1] + 4 * i) * 0.5 + y;
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 229, 3, |m, a| {
        let mut out = [0.0f32; 4];
        for i in 0..4 {
            for j in 0..4 {
                out[i as usize] += rf(m, a[1] + 16 * j + 4 * i) * rf(m, a[2] + 4 * j);
            }
        }
        for i in 0..4 {
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
            wf(m, a[0] + 4 * i, out[i as usize]);
        }
        0x229
    });
    rec(&mut t, 232, 2, |m, a| {
        let v: Vec<f32> = (0..3).map(|i| rf(m, a[1] + 4 * i)).collect();
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, v[i] / n);
        }
        0x232
    });
    for (s, n) in [(357u32, 3), (356, 3), (355, 2), (354, 2)] {
        rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0] ^ a[n - 1]);
    }
    for o in [0x4fu32, 0x6b] {
        t.set_u32(w(o), SCRATCH + 0x700);
    }
    t.write(SCRATCH + 0x700, &[3, 2, 1, 0]);
    for o in [0x59u32, 0x7d] {
        for i in 0..8 {
            t.set_u32(w(o) + 4 * i, 0x0b00_0010 + 3 * i);
        }
    }
    let ops = [
        0x1cc, 0x364, 0x439, 0x448, 0x458, 0x462, 0x46f, 0x5f0, 0x671,
    ];
    junk_ops(&mut t, f, &ops);
    for (op, x) in [
        (0x1cc, 0.5f32),
        (0x364, 1.5),
        (0x439, 1.5),
        (0x448, std::f32::consts::PI),
        (0x458, std::f32::consts::TAU),
        (0x462, -std::f32::consts::PI),
        (0x46f, std::f32::consts::TAU),
        (0x5f0, 3.0),
        (0x671, 0.75),
    ] {
        setf(&mut t, w(op), x);
    }
    for op in [0x526u32, 0x578] {
        setf(&mut t, w(op), std::f32::consts::PI);
    }
    for op in [0x530u32, 0x582, 0x547, 0x599] {
        setf(&mut t, w(op), std::f32::consts::TAU);
    }
    for op in [0x53au32, 0x58c] {
        setf(&mut t, w(op), -std::f32::consts::PI);
    }
    let s0 = SCRATCH + 0x1_0000;
    t.set_u32(w(0x1e), s0);
    t.set_u32(img.word(gd + 0x34), s0);
    let pts = SCRATCH + 0x6000;
    for (k, (d1, d2)) in (0u32..).zip([(3u32, 2u32), (0, 2), (0, 0)]) {
        junk(&mut t, s0, 0xd40, k);
        t.set_u32(s0 + 0xd08, d1);
        t.set_u32(s0 + 0xd0c, d2);
        t.set_u32(img.word(gd + 0x18f), 0x0e0e_018f + k);
        for i in 0..0x30u32 {
            let v = ((i * 11 + k * 7) % 19) as f32 * 0.5 - 4.5;
            setf(&mut t, pts + 4 * i, v);
        }
        for j in 0..3u32 {
            let r = s0 + 0xc6c + 0x30 * j;
            for (o, x) in [
                (0u32, 0.25f32 + j as f32),
                (4, -1.5 + k as f32),
                (8, 2.0),
                (0x10, 0.5),
                (0x14, -0.75 * j as f32),
            ] {
                setf(&mut t, r + o, x);
            }
            t.set_u32(r + 0x20, j + 1);
            t.set_u32(r + 0x24, pts + 0x30 * j);
            t.set_u32(r + 0x28, pts + 0x30 * j + 0x10);
            t.set_u32(r + 0x2c, pts + 0x30 * j + 0x20);
        }
        ownb::dirty_stack(&mut t, 0x35a0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0387` 0x1000ab90 (`r48::pulse_step_0387`): every state (0 and
/// 1 with `N[0x110]` low and high, 0x10 into 0x11 waiting and on into
/// 0x20 and the burst, 0x21 giving up on each test and bending, 0x50,
/// 0x51 waiting and out, 0x1000 closed and open, a state in the byte
/// table's gap, unknown ones).
#[test]
fn pulse_step_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0xab90);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let bs = img.rel32_target(f + 0x2d4);
    let wb = |o: u32| img.word(bs + o);
    let pw = img.rel32_target(f + 0x4ee);
    all_stubs(&mut t);
    copy_325(&mut t);
    for (s, n) in [(941u32, 1), (719, 3), (779, 2)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    let row = SCRATCH + 0x3_e000;
    rec(&mut t, 307, 0, |_, _| 0x307);
    t.stub_st0(307, move |m, _| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        f64::from(k as f32 * 0.125 + 0.2)
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            wf(m, a[2] + 4 * i, (a[0] & 7) as f32 * 0.5 + i as f32);
        }
        0x268
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let log = SCRATCH + 0x3_e100 + 4 * i;
            m.w32(log, m.r32(log) ^ m.r32(a[2] + 4 * i));
            let x = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[2] + 4 * i) - rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(&mut t, 232, 2, |m, a| {
        let v: Vec<f32> = (0..3).map(|i| rf(m, a[1] + 4 * i)).collect();
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
        for i in 0..3 {
            wf(m, a[0] + 4 * i as u32, v[i] / n);
        }
        let x = rf(m, a[1] + 12);
        wf(m, a[0] + 12, x);
        0x232
    });
    let dot = SCRATCH + 0x3_e200;
    rec(&mut t, 394, 2, |_, _| 0x394);
    t.stub_st0(394, move |m, a| {
        let x = rf(m, a[0]) * rf(m, a[1]) + rf(m, a[0] + 4) * rf(m, a[1] + 4) + rf(m, dot);
        f64::from(x)
    });
    rec(&mut t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    particle_globals(&mut t, bs, 0x272);
    particle_globals(&mut t, pw, 0x35);
    junk_ops(&mut t, bs, &[0x134, 0x217]);
    junk_ops(&mut t, pw, &[0x0d]);
    let ops = [
        0xa3u32, 0x207, 0xf9, 0x1af, 0x2f8, 0x300, 0x308, 0x4af, 0x4c3, 0x4d1, 0x4d9, 0x504, 0x513,
    ];
    junk_ops(&mut t, f, &ops);
    for (op, x) in [
        (0xa3u32, 0.75f32),
        (0x207, 6.0),
        (0xf9, 0.25),
        (0x1af, 0.1),
        (0x2f8, 0.875),
        (0x300, 1.0),
        (0x308, 0.125),
        (0x4af, 0.5),
        (0x4c3, 0.25),
        (0x4d1, 1.0),
        (0x4d9, 0.5),
        (0x504, 0.75),
        (0x513, 1.25),
    ] {
        setf(&mut t, w(op), x);
    }
    t.set_u32(w(0xb7), 0x8000_0000);
    let lock = SCRATCH + 0x5f0;
    t.set_u32(w(0x42a), lock);
    let (tk, d, prev, next, n, o, p) = (
        SCRATCH + 0x100,
        SCRATCH + 0x140,
        SCRATCH + 0x180,
        SCRATCH + 0x1c0,
        SCRATCH + 0x1_0000,
        SCRATCH + 0x1000,
        SCRATCH + 0x1400,
    );
    let root = wb(0x57);
    let plist = wb(0x259);
    // (state, wait, N[0x110], N[0xbc], the dot's bias, gate byte, N[0x538])
    for (k, (state, wait, lvl, bc, bias, gate, flags)) in (0u32..).zip([
        (0u32, 2u32, 0x4000u32, -9.0f32, 0.0f32, 4u8, 0x2000u32),
        (1, 2, 0x4001, -9.0, 0.0, 4, 0x2000),
        (0x10, 2, 0, 9.0, 0.0, 4, 0x2000),
        (0x11, 2, 0, -9.0, 0.0, 4, 0x2000),
        (0x21, 2, 0, 0.0, -50.0, 4, 0x2000),
        (0x21, 2, 0, 0.0, 50.0, 4, 0x2000),
        (0x21, 2, 0, 0.0, 50.0, 4, 0x2000),
        (0x50, 2, 0, 0.0, 0.0, 4, 0x2000),
        (0x51, 2, 0, 0.0, 0.0, 4, 0x2000),
        (0x51, 0, 0, 0.0, 0.0, 4, 0x2000),
        (0x1000, 0, 0, 0.0, 0.0, 0xfb, 0x2000),
        (0x1000, 0, 0, 0.0, 0.0, 4, 0xdfff),
        (0x1000, 0, 0, 0.0, 0.0, 4, 0x2000),
        (5, 0, 0, 0.0, 0.0, 4, 0x2000),
        (0x30, 0, 0, 0.0, 0.0, 4, 0x2000),
    ]) {
        junk(&mut t, n, 0x600, k);
        t.set_u32(n + 0x18, state);
        t.set_u32(n + 0x1c, wait);
        t.set_u32(n + 0x110, lvl);
        floats(&mut t, n + 0x20, 4, k);
        floats(&mut t, n + 0x34, 0x10, k + 1);
        // (one bend case with no length: `N + 0x60` = `N + 0x34`)
        if k == 5 {
            for i in 0..4u32 {
                let x = t.u32(n + 0x34 + 4 * i);
                t.set_u32(n + 0x60 + 4 * i, x);
            }
        }
        floats(&mut t, n + 0x98, 1, k);
        floats(&mut t, n + 0xb8, 3, k + 2);
        setf(&mut t, n + 0xbc, bc);
        floats(&mut t, n + 0x1e8, 16, k + 3);
        setf(&mut t, dot, bias);
        t.set_u32(n + 0x2c, o);
        t.set_u32(o + 0x54, 0x0b00_0054);
        t.set_u32(n + 0x28c, p);
        setf(&mut t, p + 0x28, 2.0);
        t.set_u32(n + 0x30, p + 0x100);
        junk(&mut t, p + 0x100, 0x40, k);
        t.write(n + 0x4d8, &[gate]);
        t.set_u32(n + 0x538, flags);
        // the burst's source, the unlinked node, the leaves
        t.set_u32(n + 0x14c + 0x140, SCRATCH + 0x1600);
        t.set_u32(n + 0x14c + 0x144, SCRATCH + 0x1700);
        junk(&mut t, SCRATCH + 0x1600, 0x200, k);
        for off in [0x4f0u32, 0xc8, 0x120, 0x14c, 0xf4, 0x70, 0x298, 0x44] {
            t.set_u32(n + off + 0x14, 0);
        }
        let (node, parent) = (n + 0x4f0, SCRATCH + 0x300);
        t.set_u32(node + 0x14, parent);
        t.set_u32(node + 4, parent);
        t.set_u32(node + 8, parent);
        t.set_u32(parent + 0xc, node);
        t.set_u32(parent + 0x10, node);
        t.set_u16(parent + 0x18, 3);
        t.set_u32(lock, 0x77 + k);
        junk(&mut t, root, 0x20, k);
        t.set_u32(root + 0x10, root);
        junk(&mut t, plist, 0x20, k + 1);
        t.set_u32(plist + 0x10, plist);
        t.set_u32(wb(0x22), SCRATCH + 0x200);
        t.set_u32(tk, d);
        t.set_u32(tk + 4, n);
        for (a, b, c) in [(d, prev, next), (prev, next, d), (next, d, prev)] {
            t.set_u32(a + 4, b);
            t.set_u32(a + 8, c);
        }
        ownb::dirty_stack(&mut t, 0xab90 + k, &[]);
        t.check_entry(f, Call::default(), &[tk], Eax::Checked);
    }
}
