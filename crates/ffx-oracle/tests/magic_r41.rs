//! Round 41's own ports (`ffx_magic::r41`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
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

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use ownb::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    magic_h6::scrub(&mut t);
    Some(t)
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

fn kf(t: &Twin, f: u32, o: u32) -> f32 {
    f32::from_bits(t.u32(t.img.word(f + o)))
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

/// A trap whose method logs a hash of the cursor it is given (slot
/// `1001`, the children's vtable `+0x1c` at `vt`).
fn child_vtable(t: &mut Twin, vt: u32) {
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1001);
    rec(t, 1001, 1, |m, a| {
        let at = SCRATCH + 0x7b0;
        let v = m.r32(at).rotate_left(5)
            ^ m.r32(a[0])
            ^ m.r32(a[0] + 4)
            ^ m.r32(a[0] + 8).rotate_left(9)
            ^ a[0];
        m.w32(at, v);
        0x1001 ^ v
    });
}

/// An empty list head at `l` (its child list itself).
fn empty_list(t: &mut Twin, l: u32) {
    t.write(l, &[0; 0x20]);
    t.set_u32(l + 0x10, l);
    t.set_u32(l + 0xc, l);
}

/// A task at `SCRATCH + 0x700` for the record `obj` (its node in a ring
/// of three).
fn task_at(t: &mut Twin, obj: u32) -> u32 {
    let (task, tn, a, b) = (
        SCRATCH + 0x700,
        SCRATCH + 0x740,
        SCRATCH + 0x7c0,
        SCRATCH + 0x7e0,
    );
    for (x, next, prev) in [(tn, a, b), (a, b, tn), (b, tn, a)] {
        t.write(x, &[0x44; 0x10]);
        t.set_u32(x + 4, next);
        t.set_u32(x + 8, prev);
    }
    t.set_u32(task, tn);
    t.set_u32(task + 4, obj);
    task
}

/// The node slots the tasks' callees make: 941 a program table, 714 the
/// part start, 713 the give-back, 325 a vector copy, 705 a float in ST0
/// that steps with each call; the particle start's lock and pointer
/// globals (the start wrapper behind `particle node` at `pn`) and the
/// game's current-word pointer `cur`.
fn node_slots(t: &mut Twin, pn: u32, cur: u32) {
    let table = SCRATCH + 0x3400;
    for j in 0..0x100 {
        t.set_u32(table + 4 * j, 0x1111 * (j + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(t, 713, 1, |_, a| 0x0713_0000 ^ a[0]);
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x10);
        }
        0x325
    });
    let k705 = SCRATCH + 0x3300;
    rec(t, 705, 0, move |m, _| {
        let c = m.r32(k705);
        m.w32(k705, c + 1);
        0x705 + c
    });
    t.stub_st0(705, move |m, _| f64::from(m.r32(k705) % 7) * 1.5 - 3.0);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(wrapper + 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    t.set_u32(cur, SCRATCH + 0x7f20);
}

/// A particle set-up's owner at `owner` (its vectors), named by the
/// global `g`.
fn owner_at(t: &mut Twin, g: u32, owner: u32) {
    t.set_u32(g, owner);
    for j in 0..0x80 {
        t.set_u32(owner + 4 * j, (0.25 * j as f32 - 1.0).to_bits());
    }
}

/// `magic_0374` 0x100079d0 (`r41::effect_task_0374`): states 0 and 10;
/// 1000 below its mark, at it (the first and the last row); 1001 below
/// 0x23 and at it (the give-back's flag bit clear and set); 10000; states
/// no case takes.
#[test]
fn effect_task_0374() {
    let Some(mut t) = twin(374) else { return };
    let f = t.at(0x79d0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let pn = t.img.rel32_target(f + 0x108);
    let cur = w(0x324);
    node_slots(&mut t, pn, cur);
    let r = SCRATCH + 0x900;
    let marks = w(0x132);
    // (state, count, index, flag byte)
    let cases: [(u32, u32, u32, u8); 13] = [
        (0, 7, 0, 0),
        (0xa, 0, 3, 0),
        (0x3e8, 0, 0, 0),
        (0x3e8, u32::MAX, 0, 0),
        (0x3e8, u32::MAX, 6, 0),
        (0x3e8, u32::MAX, 3, 0),
        (0x3e9, 0x22, 0, 0),
        (0x3e9, 0x23, 0, 0),
        (0x3e9, 0x40, 0, 4),
        (0x2710, 5, 0, 0),
        (5, 5, 0, 0),
        (0x3ea, 5, 0, 0),
        (0x2711, 5, 0, 0),
    ];
    for (k, (state, count, index, flag)) in (0u32..).zip(cases) {
        for o in [0x61u32, 0x146, 0x1d6, 0xdd, 0x291] {
            empty_list(&mut t, w(o));
        }
        owner_at(&mut t, w(0xc9), SCRATCH + 0x5000);
        owner_at(&mut t, w(0x285), SCRATCH + 0x5400);
        let count = if count == u32::MAX {
            t.u32(marks + 4 * index) + k % 3
        } else {
            count
        };
        junk(&mut t, r, 0x40, k);
        floats(&mut t, r + 0x1c, 2, k);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x24, index);
        t.set_u32(r + 0x28, count);
        // the main particle's node (its parent cleared or a list)
        let main = w(0x311);
        t.set_u32(main + 0x14, 0);
        t.set_u8(w(0x31c), flag | 0x10);
        t.set_u32(w(0x32a), 0x0032_a000 + k);
        let task = task_at(&mut t, r);
        ownb::dirty_stack(&mut t, 0x79d0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0374` 0x100074b0 (`r41::summon_task_0374`): state 0; 10 below
/// 4 and from it (below 0x50 and at it); 11; 12 with slot 705 above `R +
/// 0xd0` and not (flag bits set and clear); 10000; states no case takes.
#[test]
fn summon_task_0374() {
    let Some(mut t) = twin(374) else { return };
    let f = t.at(0x74b0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let pn = t.img.rel32_target(f + 0x152);
    let cur = w(0x251);
    node_slots(&mut t, pn, cur);
    for s in [413u32, 215, 14, 333] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.stub_st0(333, |_, _| 0.75);
    let r = SCRATCH + 0x900;
    // (state, count, R[0xd0], flag bits of the 8 particles)
    let cases: [(u32, u32, f32, u8); 13] = [
        (0, 9, 0.0, 0),
        (0xa, 3, 0.0, 0),
        (0xa, 4, 0.0, 0),
        (0xa, 0x50, 0.0, 0),
        (0xb, 0x4f, 0.0, 0),
        (0xb, 0x50, 0.0, 0),
        (0xc, 1, -10.0, 0),
        (0xc, 2, 10.0, 0),
        (0xc, 3, 10.0, 0xff),
        (0xc, 4, 10.0, 0x55),
        (0x2710, 5, 0.0, 0),
        (5, 5, 0.0, 0),
        (0x2711, 5, 0.0, 0),
    ];
    for (k, (state, count, d0, flags)) in (0u32..).zip(cases) {
        for o in [0x56u32, 0xe1, 0x1a6, 0x2e7, 0x452, 0x4d2] {
            empty_list(&mut t, w(o));
        }
        empty_list(&mut t, w(0x178));
        if flags == 0x55 {
            // the light list not empty: a last node of its own
            let last = SCRATCH + 0x6100;
            t.write(last, &[0x6c; 0x20]);
            t.set_u32(w(0x2e7) + 0x10, last);
        }
        owner_at(&mut t, w(0x113), SCRATCH + 0x5000);
        owner_at(&mut t, w(0x43e), SCRATCH + 0x5400);
        owner_at(&mut t, w(0x482), SCRATCH + 0x5800);
        junk(&mut t, r, 0x100, k);
        t.set_u32(r + 0xf4, state);
        t.set_u32(r + 0xf8, count);
        setf(&mut t, r + 0xd0, d0);
        floats(&mut t, r + 0xcc, 1, k);
        // the 8 particle records (0x100 bytes apart): parents cleared
        // (one kept), flag bytes from `flags`
        empty_list(&mut t, SCRATCH + 0x6200);
        for i in 0..8u32 {
            let rec_ = w(0x23e) + 0x100 * i;
            let parent = if i == 3 { SCRATCH + 0x6200 } else { 0 };
            t.set_u32(rec_ + 0x14, parent);
            if parent != 0 {
                t.set_u32(rec_ + 4, parent);
                t.set_u32(rec_ + 8, parent);
            }
        }
        t.set_u8(w(0x249), (flags & 1) << 2);
        for i in 0..7u32 {
            let e = w(0x273) + 0x100 * i;
            t.set_u8(e + 0x98, ((flags >> (i + 1)) & 1) << 2);
            t.set_u32(e, 0x0029_a000 + i);
        }
        t.set_u32(w(0x257), 0x0025_7000 + k);
        floats(&mut t, w(0x3e5), 1, k);
        floats(&mut t, w(0x3fc), 1, k + 1);
        let task = task_at(&mut t, r);
        ownb::dirty_stack(&mut t, 0x74b0 + k, &[]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0374` 0x10001460 / 0x10001290 (`r41::glow_thunk_0374` over
/// `glow_0374`): the walk stopped and run (no child, one child); the
/// glow drawn and not (`N + 0x28` at and above `K`); the zoom eased with
/// `N + 0x68` set; the clock below `K'` (`N + 0x50` below and at `K'`)
/// and at it.
#[test]
fn glow_0374() {
    let Some(mut t) = twin(374) else { return };
    let thunk = t.at(0x1460);
    let f = t.at(0x1290);
    all_stubs(&mut t);
    let vt = SCRATCH + 0x880;
    child_vtable(&mut t, vt);
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[2] + 4 * (i % 8)) * 0.5 + i as f32 + rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
    rec(&mut t, 245, 1, |m, a| {
        m.w32(a[0], 0x0245_0000 ^ m.r32(SCRATCH + 0x7b0));
        0x245
    });
    rec(&mut t, 563, 4, |m, a| {
        let at = SCRATCH + 0x7c0;
        let mut v = m.r32(at);
        for i in 0..64 {
            v = v.rotate_left(3) ^ m.r32(a[0] + 4 * i);
        }
        m.w32(at, v);
        0x563 ^ a[1]
    });
    let k = kf(&t, f, 0x99);
    let kc = kf(&t, f, 0x172);
    let grp = SCRATCH + 0x1000;
    let n = SCRATCH + 0x2000;
    let child = SCRATCH + 0x2200;
    let (c1, c2) = (SCRATCH + 0x2400, SCRATCH + 0x2600);
    let packets = SCRATCH + 0x4000;
    let gp = t.img.word(f + 0xd5);
    // (modes equal, a child, N[0x28], N[0x68], clock, N[0x50])
    let cases: [(bool, bool, f32, u32, f32, f32); 7] = [
        (true, false, k + 1.0, 0, 0.0, 0.0),
        (false, true, k + 1.0, 1, 0.0, 0.0),
        (false, false, k, 0, kc - 1.0, kc - 2.0),
        (false, true, k + 2.0, 0, kc - 0.5, kc),
        (false, false, k + 3.0, 0, kc, 0.0),
        (false, false, k + 0.5, 0, kc + 1.0, kc + 1.0),
        (false, true, k - 1.0, 5, kc - 3.0, kc + 3.0),
    ];
    for (k_, (same, kid, x28, x68, clock, x50)) in (0u32..).zip(cases) {
        junk(&mut t, n, 0x80, k_);
        t.set_u32(grp, n);
        t.set_u8(grp + 4, 0x40);
        t.set_u8(grp + 5, 3);
        t.set_u8(n + 0x1a, if same { 3 } else { 1 });
        if kid {
            t.set_u32(n + 0x10, child);
            t.set_u32(child, vt);
            t.set_u32(child + 8, n);
        } else {
            t.set_u32(n + 0x10, n);
        }
        floats(&mut t, n + 0x1c, 3, k_);
        setf(&mut t, n + 0x28, x28);
        floats(&mut t, n + 0x50, 6, k_ + 3);
        setf(&mut t, n + 0x50, x50);
        setf(&mut t, n + 0x58, clock);
        t.set_u32(n + 0x68, x68);
        for c in [c1, c2] {
            t.set_u32(c, vt);
            floats(&mut t, c + 0x1c, 4, k_);
            floats(&mut t, c + 0x9c, 4, k_ + 1);
        }
        t.set_u32(n + 0x6c, c1);
        t.set_u32(n + 0x70, c2);
        floats(&mut t, c1 + 0x1c, 4, k_ + 5);
        junk(&mut t, packets, 0x180, k_);
        t.set_u32(gp, packets + 0x10 * k_);
        // the sprite record's never-written words and the packet's own
        // corners (junk reads: 0 on both sides)
        ownb::dirty_stack(&mut t, 0x1290 + k_, &[0x68, 0x60, 0xb8, 0xb4]);
        t.check_entry(thunk, Call::default(), &[grp], Eax::Checked);
    }
    assert_ne!(t.u32(SCRATCH + 0x7c0), 0, "the packets were sent");
}

/// `magic_0374` 0x10001a80 (`r41::sparks_0374`, `r36::sparks_0295`'s
/// layout with slot 231 and the plain leave): the walk stopped and run;
/// sparks spawned (both rings, one wrapping), the node leaving with no
/// sparks out, the draw alone; sparks alive and not, fading in below and
/// to 1.
#[test]
fn sparks_0374() {
    let Some(mut t) = twin(374) else { return };
    let f = t.at(0x1a80);
    all_stubs(&mut t);
    let vt = SCRATCH + 0x880;
    child_vtable(&mut t, vt);
    let rnd = SCRATCH + 0x14c;
    rec(&mut t, 324, 0, move |m, _| {
        let s = m.r32(rnd).wrapping_mul(1_103_515_245).wrapping_add(12345);
        m.w32(rnd, s);
        s >> 8
    });
    t.stub_st0(307, move |m, _| f64::from(m.r32(rnd) & 0xff) / 256.0);
    for (s, n, src) in [(229u32, 3usize, 1usize), (234, 3, 1), (231, 3, 2)] {
        rec(&mut t, s, n, move |m, a| {
            // the matrix read, summed into a log word
            let at = SCRATCH + 0x7c0;
            for i in 0..16 {
                let v = m.r32(at).rotate_left(3) ^ m.r32(a[src] + 4 * i);
                m.w32(at, v);
            }
            for i in 0..4 {
                let v = rf(m, a[src] + 4 * i) * 0.5 + 0.125 * (s % 5) as f32;
                wf(m, a[0] + 4 * i, v);
            }
            s ^ a[1]
        });
    }
    // the streams open; slot 774 logs the sprite record it is given
    rec(&mut t, 773, 2, |_, _| 1);
    rec(&mut t, 774, 5, |m, a| {
        let at = SCRATCH + 0x7c4;
        let mut v = m.r32(at).rotate_left(7) ^ (m.r32(a[3]) >> 16);
        for i in 1..8 {
            v = v.rotate_left(3) ^ m.r32(a[3] + 4 * i);
        }
        m.w32(at, v);
        0x774
    });
    // the stream's locks
    let lock = SCRATCH + 0x7e0;
    let sb = t.img.rel32_target(f + 0x49e);
    for op in [0x27u32, 0x42] {
        t.set_u32(t.img.word(sb + op), lock);
    }
    let grp = SCRATCH + 0x1000;
    let n = SCRATCH + 0x2000;
    let parent = SCRATCH + 0x2200;
    let (c1, c2) = (SCRATCH + 0x2400, SCRATCH + 0x2600);
    let base = SCRATCH + 0x8000;
    let table = SCRATCH + 0x2800;
    let k_clock = kf(&t, f, 0xb3);
    // (modes equal, clock, sparks out, ring head, used, a parent)
    let cases: [(bool, f32, u32, u16, u16, bool); 6] = [
        (true, 0.0, 0, 0, 0, false),
        (false, 0.0, 0, 0, 0, false),
        (false, 0.0, 3, 6, 8, false),
        (false, k_clock, 0, 0, 0, true),
        (false, k_clock, 0, 0, 0, false),
        (false, k_clock, 2, 2, 5, true),
    ];
    for (k, (same, clock, out, head, used, parented)) in (0u32..).zip(cases) {
        t.write(n, &[0; 0x60]);
        t.set_u32(grp, n);
        t.set_u8(grp + 4, 0x40);
        t.set_u8(grp + 5, 3);
        t.set_u8(n + 0x1a, if same { 3 } else { 1 });
        t.set_u32(n + 0x10, n);
        t.set_u32(n + 0xc, n);
        // the node's own links and parent (the leave)
        let (prev, next) = (SCRATCH + 0x2300, SCRATCH + 0x2340);
        t.write(prev, &[0x61; 0x20]);
        t.write(next, &[0x62; 0x20]);
        t.write(parent, &[0x63; 0x20]);
        t.set_u32(n + 4, if k % 2 == 0 { parent } else { prev });
        t.set_u32(n + 8, if k % 3 == 0 { parent } else { next });
        t.set_u32(n + 0x14, if parented { parent } else { 0 });
        for c in [c1, c2] {
            t.set_u32(c, vt);
            floats(&mut t, c + 0x9c, 0x14, k);
        }
        t.set_u32(n + 0x48, c1);
        t.set_u32(n + 0x4c, c2);
        setf(&mut t, n + 0x58, clock);
        t.set_u32(n + 0x5c, out);
        floats(&mut t, n + 0x38, 4, k);
        // the ring: 8 sparks of 0x30 bytes
        t.set_u16(n + 0x1c, 8);
        t.set_u16(n + 0x1e, 0x30);
        t.set_u32(n + 0x20, base);
        t.set_u16(n + 0x30, head);
        t.set_u16(n + 0x32, used);
        t.set_u16(n + 0x34, 1);
        t.set_u32(n + 0x24, base + 0x30);
        t.set_u32(n + 0x28, base);
        t.set_u16(n + 0x2c, 3);
        t.set_u16(n + 0x2e, 2);
        // two stream tables: one block of 0x200 bytes each
        for (j, tb) in [table, table + 0x100].into_iter().enumerate() {
            t.write(tb, &[0; 0x40]);
            t.set_u16(tb + 6, 1);
            t.set_u16(tb + 0x10, 0x20);
            t.set_u16(tb + 0x12, 0x200);
            t.write(tb + 0x20, &[0x11 * (j as u8 + 1); 0x20]);
            t.set_u32(n + 0x50 + 4 * j as u32, tb);
        }
        for i in 0..8u32 {
            let e = base + 0x30 * i;
            floats(&mut t, e, 12, i + k);
            t.set_u32(e + 0x10, if i & 1 == 0 { 0x0000_ffff } else { 0 });
            t.set_u32(e + 0x24, i & 1);
            t.set_u8(e + 0x2b, [0u8, 1, 4, 0x80][(i % 4) as usize]);
            setf(
                &mut t,
                e + 0x2c,
                [0.5f32, 0.95, 1.0, 0.1][((i + k) % 4) as usize],
            );
        }
        t.set_u32(rnd, 0x1234 + 77 * k);
        t.set_u32(lock, 0x40 + k);
        ownb::dirty_stack(&mut t, 0x1a80 + k, &[]);
        t.check_entry(f, Call::default(), &[grp], Eax::Checked);
    }
}

/// The load step at `load` in its four cases (busy, asked, loaded and
/// not), each after `setup`.
fn load_cases(t: &mut Twin, load: u32, setup: &dyn Fn(&mut Twin)) {
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        setup(t);
        rec(t, 225, 1, move |_, _| busy);
        rec(t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32, 0x46] {
            let at = op(t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(t, load, &[0x61, 0x6b]);
        magic_h6::scrub(t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// What `r41::start_up_0271` (`magic_0271` 0x10001460) needs: the pools,
/// the part set-up's and the object's buffers, the caster, the target
/// selection, the state block, the actor flags, the task list (empty or
/// not); slot 333's yaw `yaw` and the target's place `tz`.
fn start_up_0271_setup(t: &mut Twin, f: u32, k: u32, listed: bool, yaw: f64, tz: f32) {
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(t);
    magic_r11::pools(t, f, 835);
    magic_r11::object_slots(t);
    magic_r11::object_start_auto(t, t.img.rel32_target(f + 0x7a));
    rec(t, 262, 4, move |m, a| {
        for i in 0..3u32 {
            let v = ((a[0] ^ (a[0] >> 11)) & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            let v = if i == 2 && a[0] == 0x0000_0099 { tz } else { v };
            wf(m, a[1 + i as usize], v);
        }
        0x262
    });
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    for (s, n) in [(271u32, 1usize), (220, 3), (413, 1), (215, 1)] {
        rec(t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[1] << 8));
    // the caster and the selected target (its byte 3 in the table)
    t.set_u32(w(0x9a), 0x0000_0047 + k);
    t.set_u32(w(0x116), 0x0000_0047 + k);
    t.set_u32(w(0xa7), 0x0000_0033);
    let sel = SCRATCH + 0x180;
    t.write(sel, &[3, 9, 0x13, 2]);
    t.set_u32(w(0x135), sel);
    t.set_u32(w(0x156) + 12, 0x0000_0099);
    // the state block (the fade's record inside it)
    let s = SCRATCH + 0x2_0000;
    junk(t, s + 0x1e00, 0x200, k);
    t.set_u32(w(0xfc), s);
    junk_ops(t, f, &[0x2d, 0x37, 0x41, 0x4b, 0x59, 0x11e, 0x124, 0xad]);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(0x29a), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(0x2b1), SCRATCH + 0x600);
}

/// `magic_0271` 0x10001460 (`r41::start_up_0271`, internal) and its load
/// step 0x100012b0 (`r41::load_0271`): the task list empty and not, the
/// yaw wrapped down, kept and wrapped up, the angle to the target wrapped
/// both ways and kept.
#[test]
fn start_up_0271() {
    let Some(mut t) = twin(271) else { return };
    let f = t.at(0x1460);
    for (k, (listed, yaw, tz)) in (0u32..).zip([
        (false, 0.25f64, 30.0f32),
        (true, 9.5, -30.0),
        (true, -7.0, 0.5),
        (false, 3.0, -0.75),
    ]) {
        start_up_0271_setup(&mut t, f, k, listed, yaw, tz);
        ownb::dirty_stack(&mut t, 0x1460 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r41::start_up_0271,
        );
    }
    let load = t.at(0x12b0);
    load_cases(&mut t, load, &|t| {
        start_up_0271_setup(t, f, 2, true, 1.0, 2.0);
    });
}

/// The trail's slots (`magic_0271`'s curve task and trail): vectors and
/// matrices that mix what they read, slot 941's frame table (`count`
/// frames at `+0x10`, the last frame's loop byte `looped`), handles,
/// slot 245's pad word, slot 773's answer `known`.
fn trail_stubs(t: &mut Twin, count: i16, looped: bool, known: u32, pkt: u32) {
    all_stubs(t);
    filler_16(t, 235);
    for (s, n) in [(229u32, 3usize), (230, 2), (330, 3)] {
        rec(t, s, n, move |m, a| {
            let k = (s & 0xf) as f32;
            let turn = if s == 330 { f32::from_bits(a[2]) } else { 0.0 };
            for i in 0..16 {
                // slot 229's point (its third word) read too
                let p = if s == 229 {
                    rf(m, a[2] + 4 * (i % 4))
                } else {
                    0.0
                };
                let v = rf(m, a[1] + 4 * i) * 0.5 + i as f32 * 0.25 - k + turn + p;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[2] + 4 * i) - rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x234
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x248
    });
    rec(t, 245, 1, |m, a| {
        let c = m.r32(SCRATCH + 0xa10);
        m.w32(SCRATCH + 0xa10, c + 1);
        m.w32(a[0], c);
        0x245
    });
    for s in [723u32, 724, 603, 604] {
        rec(t, s, 1, move |m, a| {
            let at = SCRATCH + 0xa20;
            let mut v = m.r32(at).rotate_left(5) ^ s;
            if s == 603 || s == 604 {
                // the sprite record on the frame and the matrix the
                // packet global points at
                for i in 0..0x20 {
                    v = v.rotate_left(3) ^ m.r32(a[0] + 4 * i);
                }
                let mx = m.r32(pkt);
                for i in 0..16 {
                    v = v.rotate_left(3) ^ m.r32(mx + 4 * i);
                }
            }
            m.w32(at, v);
            if s == 724 {
                // the camera matrix out
                for i in 0..16u32 {
                    wf(m, a[0] + 4 * i, i as f32 * 0.5 - 1.0);
                }
            }
            s ^ a[0]
        });
    }
    rec(t, 942, 1, |_, a| 0x0942_0000 ^ a[0]);
    let table = SCRATCH + 0x3400;
    let bytes: Vec<u8> = (0..0x200u32).map(|i| (i * 7 + 3) as u8).collect();
    t.write(table, &bytes);
    t.set_u16(table + 6, count as u16);
    for i in 0..0x20u32 {
        let e = table + 0x10 + 8 * i;
        t.set_u16(e, 0x10 + 8 * i as u16);
        t.set_u16(e + 2, 0x300 + 0x40 * i as u16);
        t.set_u32(e + 4, 0x40);
    }
    if looped {
        let last = table + 0x10 + 8 * (count as u32).saturating_sub(1);
        t.set_u8(last + 4, 0x80);
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| a[0] ^ 0x77);
    rec(t, 773, 4, move |_, _| known);
    for (s, n) in [(774u32, 5usize), (775, 2), (776, 1), (777, 1), (778, 6)] {
        rec(t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
}

/// A slot that fills a 4x4 matrix at its argument (a unit matrix with a
/// count in the corner).
fn filler_16(t: &mut Twin, s: u32) {
    rec(t, s, 1, move |m, a| {
        let c = m.r32(SCRATCH + 0xa30);
        m.w32(SCRATCH + 0xa30, c + 1);
        for i in 0..16u32 {
            let v = if i % 5 == 0 { 1.0 } else { 0.0 };
            wf(m, a[0] + 4 * i, v);
        }
        wf(m, a[0] + 12, c as f32);
        s
    });
}

/// `magic_0271` 0x10001e80 (`r41::curve_task_0271`, cdecl `(node)`) and
/// its trail 0x100021c0 (`r41::trail_0271`): the count above 1, at 1
/// (the object started) and below; the trail's time left and out (the
/// node given back); segments walking back through the ring (equal
/// points skipped, an unset flag byte, the 32 points run out), points
/// moved along and copied, with and without an alpha, the frame wrapped.
#[test]
fn curve_task_0271() {
    let Some(mut t) = twin(271) else { return };
    let f = t.at(0x1e80);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let trail = img.rel32_target(f + 0x210);
    let tw = |o: u32| img.word(trail + o);
    let give = img.rel32_target(f + 0x22b);
    let start = img.rel32_target(f + 0x1b1);
    let s = SCRATCH + 0x2_0000;
    // (count, time, segments, spacing, alpha, flags unset from, equal
    // pair, frame count, known)
    let cases: [(i32, i32, i32, f32, u8, u32, bool, i16, u32); 9] = [
        (5, 3, 4, 0.75, 0x80, 32, false, 4, 0),
        (1, 3, 3, 0.25, 0, 32, false, 4, 1),
        (0, 0, 3, 0.5, 0x40, 32, false, 4, 0),
        (-1, 2, 5, 1.5, 0x20, 2, false, 3, 1),
        (-1, 2, 6, 0.5, 0x20, 32, true, 2, 0),
        (4, 1, 0, 0.5, 0x20, 32, false, 4, 0),
        (3, 1, 40, 4.0, 0x20, 32, false, 5, 1),
        (2, 1, 3, 0.1, 0x20, 32, false, 1, 0),
        (-5, 0, 3, 0.1, 0x20, 32, false, 1, 0),
    ];
    for (k, (count, time, segs, spacing, alpha, unset, equal, frames, known)) in (0u32..).zip(cases)
    {
        magic_r11::pools(&mut t, f, 576);
        magic_r11::object_slots(&mut t);
        magic_r11::object_start_auto(&mut t, start);
        trail_stubs(&mut t, frames, k % 2 == 0, known, tw(0x17e));
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        let idx = k % 3;
        t.set_u32(node + 0x14, idx);
        t.set_u32(w(0x11), s);
        junk(&mut t, s + 0x1f00, 0x40, k);
        let r = s + 0x3d0 * idx;
        junk(&mut t, r, 0x3d0, k);
        floats(&mut t, r, 16, k);
        t.set_u32(r + 0x3bc, count.cast_unsigned());
        t.set_u32(r + 0x398, time.cast_unsigned());
        // the trail: the anim cursor, the ring, the colours and widths
        let tr = r + 0x170;
        t.set_u32(tr, 0x0000_0071);
        t.set_u32(tr + 4, k & 1);
        t.set_u16(tr + 8, (k % 3) as u16);
        t.set_u16(tr + 0xa, if k % 2 == 0 { 0x100 } else { 0x300 });
        let cur = (k * 7) % 32;
        for i in 0..32u32 {
            let p = tr + 0xc + 16 * i;
            let back = (cur + 32 - i) % 32;
            let x = back as f32 * 0.5 + (k as f32) * 0.25;
            setf(&mut t, p, x);
            setf(&mut t, p + 4, back as f32 * -0.25);
            setf(&mut t, p + 8, 1.0 - back as f32 * 0.125);
            setf(&mut t, p + 12, 1.0);
            let set = back < unset || back == 0;
            let first = i == (cur + 31) % 32;
            t.set_u8(tr + 0x22c + i, if first { 7 } else { u8::from(set) });
        }
        if equal {
            // the second-newest point the same as the third
            let a = tr + 0xc + 16 * ((cur + 31) % 32);
            let b = tr + 0xc + 16 * ((cur + 30) % 32);
            for i in 0..4 {
                let v = t.u32(b + 4 * i);
                t.set_u32(a + 4 * i, v);
            }
        }
        t.write(
            tr + 0x20c,
            &[0x40, 0x80, 0xc0, alpha, 0x10, 0x20, 0x30, alpha / 2],
        );
        setf(&mut t, tr + 0x214, 2.0);
        setf(&mut t, tr + 0x218, 0.5);
        setf(&mut t, tr + 0x21c, spacing);
        t.set_u32(tr + 0x220, 0x1234 + 99 * k);
        t.set_u32(tr + 0x224, (cur + 31) % 32);
        t.set_u32(tr + 0x228, segs.cast_unsigned());
        // the trail's globals: `*G_C`'s words, the packet, the lock
        let c = SCRATCH + 0x5000;
        floats(&mut t, c, 0x20, k);
        t.set_u32(tw(0x12c), c);
        t.set_u32(tw(0x6e3), SCRATCH + 0x5100 + k);
        let lock = SCRATCH + 0x7f40;
        for o in [0x745u32, 0x767, 0x787] {
            t.set_u32(tw(o), lock);
        }
        t.set_u32(lock, 0x40 + k);
        let link = img.rel32_target(trail + 0x6de);
        for o in [0x4fu32, 0x7a, 0xa9, 0xc6, 0x115, 0x13b] {
            t.set_u32(img.word(link + o), lock);
        }
        ownb::dirty_stack(&mut t, 0x1e80 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// The slots of `magic_0370`'s sprite frame: 941 the frame table `tbl`
/// (3 frames), 772/773 a handle and whether it is drawn (by the part's
/// low bit), 774/778 hashing the draw record `D` they are given, the
/// matrix slots logging their words. Returns the lock word's address.
fn sprite_stubs_0370(t: &mut Twin, sf: u32, tbl: u32, (lock_op, k_op): (u32, u32)) -> u32 {
    all_stubs(t);
    let lock = SCRATCH + 0x3100;
    t.set_u32(t.img.word(sf + lock_op), lock);
    floats(t, t.img.word(sf + k_op), 1, 3);
    rec(t, 941, 1, move |_, _| tbl);
    rec(t, 772, 2, |_, a| a[0] ^ a[1]);
    t.stub_edx(772, |_, _| 0x0772_0002);
    rec(t, 773, 4, |_, a| u32::from(a[3] & 1 == 0));
    for (s, n) in [(774u32, 5usize), (778, 6)] {
        rec(t, s, n, move |m, a| {
            // the draw record and the game lock as the call sees it
            let mut v = s ^ m.r32(lock);
            for i in 0..8 {
                v = v.rotate_left(5) ^ m.r32(a[3] + 4 * i);
            }
            m.w32(SCRATCH + 0xa40, v ^ m.r32(SCRATCH + 0xa40));
            s
        });
    }
    for (s, n) in [
        (775u32, 2usize),
        (776, 1),
        (777, 1),
        (723, 1),
        (724, 1),
        (235, 1),
        (383, 2),
        (231, 3),
        (330, 3),
        (230, 2),
        (325, 2),
        (726, 2),
    ] {
        rec(t, s, n, move |m, a| (s << 16) ^ a[0] ^ m.r32(lock));
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..3 {
            let v = 0.5 * i as f32 + rf(m, a[2]);
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    t.write(tbl, &[0x37; 0x100]);
    t.set_u16(tbl + 4, 3);
    for i in 0..3u32 {
        t.set_u16(tbl + 8 * i + 0x10, (0x80 + 8 * i) as u16);
        t.set_u16(tbl + 8 * i + 0x12, (0x100 * (i + 1)) as u16);
    }
    for i in 0..4u32 {
        t.set_u32(tbl + 0x80 + 8 * i + 4, 0x10 + i);
    }
    lock
}

/// `magic_0370` 0x10002f80 (`r41::block_frames_0370`) with its sprite
/// frame 0x10002810 (`r41::sprite_frame_0370`): no records, three (one
/// empty), with and without slot 229's point, timers running out, drawn
/// and not; the sprite frame alone without a record (the printf path).
#[test]
fn block_frames_0370() {
    use ffx_magic::Xmm;
    let Some(mut t) = twin(370) else { return };
    let f = t.at(0x2f80);
    let sf = t.at(0x2810);
    let tbl = SCRATCH + 0x3200;
    let lock = sprite_stubs_0370(&mut t, sf, tbl, (0x28e, 0x181));
    t.set_u32(t.img.word(f + 0x11), 0x00ab_0001);
    let (b, base, d) = (SCRATCH + 0x3f00, SCRATCH + 0x4000, SCRATCH + 0x3e00);
    for (c, (n, flag)) in (0u32..).zip([(0u32, 0u32), (3, 1), (3, 0), (2, 1)]) {
        floats(&mut t, b, 0x10, c + 7);
        t.set_u32(b, n);
        t.set_u32(b + 4, base);
        t.set_u32(b + 0x18, flag);
        t.set_u32(lock, 0x55 + c);
        junk(&mut t, d, 0x40, c);
        for i in 0..3u32 {
            let rec_ = base + 0x94 * i;
            floats(&mut t, rec_, 0x94 / 4, c + i);
            t.set_u32(rec_, u32::from(i != 1 || c == 3));
            let r = rec_ + 0x30;
            t.set_u32(r, 0x0941_0000 + i);
            t.set_u16(r + 8, ((i + c) % 3) as u16);
            t.set_u16(r + 0x10, if (i + c) % 2 == 0 { 0x200 } else { 0x300 });
            t.set_u32(r + 0xc, tbl + 0x80);
        }
        ownb::dirty_stack(&mut t, 0x2f80 + c, &[]);
        let call = Call {
            ecx: b,
            edx: d,
            ..Call::default()
        }
        .with_xmm(&[
            Xmm::f32(9.0),
            Xmm::f32(8.0),
            Xmm::f32(7.0),
            Xmm::f32(0.25 + c as f32),
        ]);
        t.check(f, call, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r41::block_frames_0370(g, img, fr)
        });
    }
    // no record: the printf wrapper
    let call = Call {
        ecx: 0,
        edx: d,
        ..Call::default()
    };
    ownb::dirty_stack(&mut t, 0x2810, &[]);
    t.check(sf, call, &[5, 6], Eax::Checked, |g, img, fr| {
        ffx_magic::r41::sprite_frame_0370(g, img, fr)
    });
}

/// `magic_0370` 0x10001320 (`r41::sprite_reset_0370`): the draw records
/// and both sprite systems with their 500 sprites, all junk before.
#[test]
fn sprite_reset_0370() {
    let Some(mut t) = twin(370) else { return };
    let f = t.at(0x1320);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[2] << 8));
    for k in 0..2u32 {
        t.set_u32(w(0x26), 0x0000_0026 + k);
        junk(&mut t, w(0x41), 0x60, k);
        junk(&mut t, w(0x4b), 8, k);
        let (b0, end) = (w(0x72) - 8, w(0xe5) - 8);
        junk(&mut t, b0, end - b0, k + 1);
        let n = (end - b0) / 0x5c;
        for i in 0..n {
            let sp = SCRATCH + 0x4_0000 + 0x1_3000 * i;
            t.set_u32(w(0x6a) + 4 * i, sp);
            junk(&mut t, sp, 0x94 * 500, k + i);
        }
        ownb::dirty_stack(&mut t, 0x1320 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0370` 0x100015c0 (`r41::draw_0370`): slot 712 answering 0 and
/// not, slot 261 answering 0 and not.
#[test]
fn draw_0370() {
    let Some(mut t) = twin(370) else { return };
    let f = t.at(0x15c0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let lock = SCRATCH + 0x7f40;
    t.set_u32(w(0x8a), lock);
    let tbl = SCRATCH + 0x3400;
    junk(&mut t, tbl, 0x40, 3);
    rec(&mut t, 941, 1, move |_, _| tbl);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    let wrapper = t.img.rel32_target(f + 0x120);
    t.set_u32(t.img.word(wrapper + 4), lock);
    let answers = SCRATCH + 0x7f50;
    rec(&mut t, 712, 2, move |m, _| m.r32(answers));
    rec(&mut t, 261, 1, move |m, _| m.r32(answers + 4));
    for (k, (ok, busy)) in (0u32..).zip([(0u32, 0u32), (1, 1), (1, 0), (5, 0)]) {
        t.set_u32(answers, ok);
        t.set_u32(answers + 4, busy);
        t.set_u32(lock, 0x40 + k);
        for o in [0x1u32, 0x7d, 0xd2, 0xef] {
            t.set_u32(w(o), SCRATCH + 0x7f60 + 4 * k);
        }
        junk_ops(&mut t, f, &[0x8, 0x26, 0x46, 0x51, 0x5d, 0x65, 0x70]);
        let d = SCRATCH + 0x7f80;
        junk(&mut t, d, 0x20, k);
        t.set_u32(w(0xfa), d);
        ownb::dirty_stack(&mut t, 0x15c0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0370` 0x10001a70 (`r41::start_0370`, cdecl `(a)`): the start-up
/// on junk globals.
#[test]
fn start_0370() {
    let Some(mut t) = twin(370) else { return };
    let f = t.at(0x1a70);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let lock = SCRATCH + 0x7f40;
    t.set_u32(w(0xa6), lock);
    let parts = SCRATCH + 0x7f80;
    for k in 0..2u32 {
        junk(&mut t, parts, 0x40, k);
        rec(&mut t, 719, 3, move |_, _| parts);
        rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
        rec(&mut t, 803, 2, move |_, _| 0x0800_0000 + k);
        rec(&mut t, 711, 3, move |m, a| m.r32(lock) ^ a[0]);
        for o in [0x14au32, 0x17e, 0x199] {
            junk(&mut t, w(o), 0x110, k);
        }
        junk_ops(&mut t, f, &[0x87, 0x9e, 0x220, 0x237, 0x10f, 0x129, 0x14f]);
        t.set_u32(lock, 0x40 + k);
        ownb::dirty_stack(&mut t, 0x1a70 + k, &[]);
        t.check_entry(f, Call::default(), &[0x0abc_0000 + k], Eax::Checked);
    }
}

/// `magic_0370` 0x10001420 (`r41::sprite_update_0370`) with its sparks
/// 0x100011a0 (`r41::spark_0370`): both sparks, one, none (the random
/// draws), the pools with a free sprite and full; the systems' steps and
/// frames over what the sparks left.
#[test]
fn sprite_update_0370() {
    let Some(mut t) = twin(370) else { return };
    let f = t.at(0x1420);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let sf = t.at(0x2810);
    let tbl = SCRATCH + 0x3200;
    let lock = sprite_stubs_0370(&mut t, sf, tbl, (0x28e, 0x181));
    t.set_u32(img.word(sf + 0x28e), lock);
    let rnd = SCRATCH + 0x14c;
    rec(&mut t, 324, 0, move |m, _| {
        let s = m.r32(rnd).wrapping_mul(1_103_515_245).wrapping_add(12345);
        m.w32(rnd, s);
        s >> 8
    });
    rec(&mut t, 403, 1, |_, a| 0x0403_0000 ^ a[0]);
    t.stub_st0(403, |_, a| f64::from(f32::from_bits(a[0])) * 2.0);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            wf(m, a[3] + 4 * i, i as f32 + 0.5);
        }
        0x237
    });
    for k in 0..6u32 {
        t.set_u32(rnd, 0x1234 + 0x9e37 * k);
        // the two systems (0x5c bytes): 2 sprites each, free or taken
        for j in 0..2u32 {
            let sys = w(0xd1) + 0x5c * j;
            junk(&mut t, sys, 0x5c, k + j);
            let sp = SCRATCH + 0x9000 + 0x200 * j;
            junk(&mut t, sp, 0x128, k);
            t.set_u32(sys, 2);
            t.set_u32(sys + 4, sp);
            for o in [8u32, 0x10, 0x18] {
                t.set_u32(sys + o, 0);
            }
            for i in 0..2 {
                t.set_u32(sp + 0x94 * i, u32::from(k % 3 == 2));
            }
        }
        junk(&mut t, w(0xd6), 0x60, k);
        t.set_u32(lock, 0x40 + k);
        junk_ops(&mut t, f, &[0x2c, 0x70, 0xb8, 0xfe]);
        ownb::dirty_stack(&mut t, 0x1420 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// Points each pointer global named by the operands `ops` of `f` at a
/// cell of its own (`SCRATCH + 0x7c00 + 0x10 i`, junk inside).
fn cells(t: &mut Twin, f: u32, ops: &[u32]) {
    for (i, &o) in (0u32..).zip(ops) {
        let cell = SCRATCH + 0x7c00 + 0x10 * i;
        t.set_u32(cell, 0x00c0_0000 + i);
        let at = t.img.word(f + o);
        t.set_u32(at, cell);
    }
}

/// The slots of `magic_0376`'s part callbacks: 941 a program table, 714
/// the part start, 712/261/918 logged with answers from `ANSWERS`, the
/// view slots, 237/234/248 writing vectors; each part start's lock (the
/// wrappers at the `rel32`s `calls` of `f`) at `SCRATCH + 0x7f00`; the
/// part global `*G_PART` (operand `part`) a record with its key.
const ANSWERS: u32 = SCRATCH + 0x7f50;

fn part_stubs_0376(t: &mut Twin, f: u32, calls: &[u32], parts: &[u32]) {
    all_stubs(t);
    let table = SCRATCH + 0x3400;
    junk(t, table, 0x80, 9);
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    rec(t, 712, 2, |m, _| m.r32(ANSWERS));
    rec(t, 261, 1, |m, _| m.r32(ANSWERS + 4));
    rec(t, 918, 1, |_, a| 0x0918_0000 ^ a[0]);
    let rnd = SCRATCH + 0x14c;
    rec(t, 324, 0, move |m, _| {
        let s = m.r32(rnd).wrapping_mul(1_103_515_245).wrapping_add(12345);
        m.w32(rnd, s);
        s >> 8
    });
    rec(t, 258, 1, |_, a| 0x0258_0000 ^ a[0]);
    rec(t, 231, 3, |_, a| a[0] ^ a[1] ^ a[2]);
    rec(t, 230, 2, |_, a| a[0] ^ a[1]);
    for s in [237u32, 234, 248] {
        let n = if s == 237 { 4 } else { 3 };
        rec(t, s, n, move |m, a| {
            let (dst, src) = if s == 237 { (a[3], a[0]) } else { (a[0], a[1]) };
            for i in 0..4u32 {
                let x = if s == 237 {
                    (src & 0xff) as f32 + i as f32
                } else {
                    rf(m, src + 4 * i) * 0.5 + rf(m, a[2] + 4 * i)
                };
                wf(m, dst + 4 * i, x);
            }
            s
        });
    }
    let lock = SCRATCH + 0x7f00;
    for &c in calls {
        let wrapper = t.img.rel32_target(f + c);
        let at = t.img.word(wrapper + 4);
        t.set_u32(at, lock);
        let at = t.img.word(wrapper + 0x29);
        t.set_u32(at, lock);
    }
    t.set_u32(lock, 0x77);
    let rec_ = SCRATCH + 0x7e00;
    junk(t, rec_, 0x20, 4);
    for &o in parts {
        let at = t.img.word(f + o);
        t.set_u32(at, rec_);
    }
}

/// `magic_0376`'s part callbacks 0x10001260 (`r41::glow_0376`),
/// 0x10001480 (`ring_0376`), 0x100015b0 (`pillar_0376`), 0x10001780
/// (`pillar_step_0376`), 0x100018e0 (`orb_0376`), 0x10002090
/// (`beam_0376`): parts before, in and past their table's cases, the
/// clock's three ways, slots 712/261 answering 0 and not.
#[test]
fn part_callbacks_0376() {
    let Some(mut t) = twin(376) else { return };
    let img = t.img.clone();
    // (address, the table operand, the part starts, the part globals,
    // the scene/lock pointer globals)
    let funcs: [(u32, u32, &[u32], &[u32], &[u32]); 6] = [
        (0x1260, 0x31, &[], &[], &[0x113, 0x118]),
        (0x1480, 0x0, &[0xb7], &[0x94], &[0x89, 0xdc, 0xe1]),
        (
            0x15b0,
            0xa,
            &[0x49, 0x9e, 0xf3, 0x145],
            &[0x26, 0x7b, 0xd0, 0x122],
            &[0x1b, 0x66, 0xbb, 0x10d],
        ),
        (0x1780, 0xb, &[], &[], &[0xe5, 0xea, 0x133]),
        (0x18e0, 0xa, &[0x44, 0xb3], &[0x21, 0x90], &[0x16, 0x85]),
        (
            0x2090,
            0xa,
            &[0x44, 0xab, 0xfe],
            &[0x21, 0x88, 0xdb],
            &[0x16, 0x7d, 0xd0],
        ),
    ];
    for (rva, tb, calls, parts, ptrs) in funcs {
        let f = t.at(rva);
        part_stubs_0376(&mut t, f, calls, parts);
        let base = if tb == 0 {
            SCRATCH + 0x6000
        } else {
            img.word(f + tb)
        };
        let clock = t.img.word(t.at(0x1480) + 0x6a);
        for k in 0..6u32 {
            cells(&mut t, f, ptrs);
            // the counters 0x1780 steps (some past 0x20)
            for o in [0x17u32, 0x30, 0x49, 0x62] {
                let at = t.img.word(t.at(0x1780) + o);
                t.set_u32(at, [3u32, 0x20, 0x25, 0x40][(k % 4) as usize] + (o & 3));
            }
            t.set_u32(clock, [0xa8u32, 0xb1, 0xb2, 0xb3, 0x10, 0xa7][k as usize]);
            t.set_u32(ANSWERS, k & 1);
            t.set_u32(ANSWERS + 4, (k >> 1) & 1);
            // (the glow reads the same clock as its timer)
            let p = if k == 5 {
                SCRATCH + 0x6800
            } else {
                base + 0x80 * k
            };
            junk(&mut t, p, 0x80, k);
            floats(&mut t, p + 0x40, 4, k);
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[p], Eax::Checked);
        }
    }
}

/// `magic_0376` 0x10001f20 (`r41::embers_0376`) and 0x10001d00
/// (`burst_0376`) with their spark 0x10001aa0 (`spark_0376`): the clock
/// on and off its ticks, the system with a free sprite and full, live
/// sprites coloured, stepped and drawn.
#[test]
fn sparks_0376() {
    let Some(mut t) = twin(376) else { return };
    let img = t.img.clone();
    for (rva, clock_op) in [(0x1f20u32, 0x32u32), (0x1d00, 0x38)] {
        let f = t.at(rva);
        let rnd = SCRATCH + 0x14c;
        let tbl = SCRATCH + 0x3200;
        let sf =
            img.rel32_target(img.rel32_target(f + if rva == 0x1f20 { 0xa5 } else { 0x194 }) + 0x83);
        sprite_stubs_0370(&mut t, sf, tbl, (0x290, 0x179));
        rec(&mut t, 324, 0, move |m, _| {
            let s = m.r32(rnd).wrapping_mul(1_103_515_245).wrapping_add(12345);
            m.w32(rnd, s);
            s >> 8
        });
        rec(&mut t, 237, 4, |m, a| {
            for i in 0..4 {
                wf(m, a[3] + 4 * i, i as f32 + 0.5);
            }
            0x237
        });
        let clock = img.word(f + clock_op);
        // the slot 237 word and the sprites' resource (the spark's operand)
        junk_ops(&mut t, f, if rva == 0x1f20 { &[0x2a] } else { &[0x2d] });
        let spark = img.rel32_target(f + if rva == 0x1f20 { 0x5f } else { 0x4a });
        junk_ops(&mut t, spark, &[0x6]);
        for k in 0..6u32 {
            t.set_u32(rnd, 0x1234 + 0x9e37 * k);
            t.set_u32(clock, [0u32, 5, 7, 0x235, 0x63, 0x64][k as usize]);
            let sys = SCRATCH + 0x8800;
            let sp = SCRATCH + 0x9000;
            junk(&mut t, sys, 0x40, k);
            junk(&mut t, sp, 0x128, k);
            t.set_u32(sys, 2);
            t.set_u32(sys + 4, sp);
            for o in [8u32, 0x10, 0x18] {
                t.set_u32(sys + o, 0);
            }
            t.set_u32(sp, u32::from(k % 3 == 2));
            t.set_u32(sp + 0x94, u32::from(k % 2 == 1));
            for i in 0..2u32 {
                t.set_u32(sp + 0x94 * i + 0x28, i + k % 3);
            }
            ownb::dirty_stack(&mut t, rva + k, &[]);
            t.check_entry(f, Call::default(), &[sys], Eax::Checked);
        }
    }
}

/// `magic_0376` 0x10002330 (`r41::start_0376`, cdecl `(a)`): the
/// start-up on junk globals.
#[test]
fn start_0376() {
    let Some(mut t) = twin(376) else { return };
    let f = t.at(0x2330);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    let lock = SCRATCH + 0x7f40;
    t.set_u32(w(0xa6), lock);
    let parts = SCRATCH + 0x7f80;
    for k in 0..2u32 {
        junk(&mut t, parts, 0x40, k);
        rec(&mut t, 719, 3, move |_, _| parts);
        rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
        rec(&mut t, 803, 2, move |_, _| 0x0800_0000 + k);
        rec(&mut t, 711, 3, move |m, a| m.r32(lock) ^ a[0]);
        junk(&mut t, w(0x147), 0x30, k);
        for o in [0x168u32, 0x179, 0x1f8, 0x231] {
            junk(&mut t, w(o), 0x110, k);
        }
        junk_ops(&mut t, f, &[0x87, 0x9e, 0x2e3, 0x2fa, 0x10f, 0x129, 0x14c]);
        t.set_u32(lock, 0x40 + k);
        ownb::dirty_stack(&mut t, 0x2330 + k, &[]);
        t.check_entry(f, Call::default(), &[0x0abc_0000 + k], Eax::Checked);
    }
}

/// The pool `release_0269`'s inlined give-back and `give_back` itself
/// keep (their globals at the operands of `at` and `gb`): case 1 an empty
/// free ring, case 2 the node as every tracker with 0xff free.
fn release_pool_0269(t: &mut Twin, at: u32, gb: u32, n: u32, case: u32) {
    let ring = SCRATCH + 0x1600;
    for g in [gb, at] {
        let ops = if g == at {
            [0xb7u32, 0xc4, 0xd3, 0xe3, 0xfd]
        } else {
            [0x48, 0x55, 0x64, 0x74, 0x8e]
        };
        let [cur, head, tail, free, count] = ops.map(|op| t.img.word(g + op));
        let mark = if case == 2 { n } else { SCRATCH + 0x3f00 };
        t.set_u32(cur, mark);
        t.set_u32(head, mark);
        t.set_u32(tail, mark);
        t.set_u32(free, if case == 1 { 0 } else { ring });
        t.set_u16(count, if case == 2 { 0xff } else { 4 });
    }
    t.set_u32(ring, ring);
    t.set_u32(ring + 4, ring);
}

/// The node `release_0269` frees (`n`; with `kids` three children and
/// both neighbours).
fn release_node_0269(t: &mut Twin, n: u32, kids: bool) {
    let (prev, next, c1, c2, c3) = (
        SCRATCH + 0x1100,
        SCRATCH + 0x1200,
        SCRATCH + 0x1300,
        SCRATCH + 0x1400,
        SCRATCH + 0x1500,
    );
    t.write(SCRATCH + 0x1000, &[0x31; 0x600]);
    if kids {
        t.set_u32(n, prev);
        t.set_u32(n + 4, next);
        t.set_u32(n + 8, c1);
        for (c, nx) in [(c1, c2), (c2, c3), (c3, 0)] {
            t.set_u32(c, 0);
            t.set_u32(c + 4, 0);
            t.set_u32(c + 8, c);
            t.set_u32(c + 0xc, nx);
        }
    } else {
        t.set_u32(n, 0);
        t.set_u32(n + 4, 0);
        t.set_u32(n + 8, n);
        t.set_u32(n + 0xc, 0);
    }
}

/// `magic_0269` 0x100025c0 (`r41::release_0269`) and the task 0x10001ac0
/// (`r41::task_0269`) that reaches it by a tail jump: the parts
/// unregistered, the node freed with and without children, the free ring
/// empty and not; the task with an object running, the motion waiting,
/// both done (the characters' slot 213 for 0..3 selected).
#[test]
fn release_task_0269() {
    let Some(mut t) = twin(269) else { return };
    let at = t.at(0x25c0);
    let task = t.at(0x1ac0);
    let img = t.img.clone();
    let w = |o: u32| img.word(at + o);
    all_stubs(&mut t);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| a[0] ^ (a[1] << 4));
    t.stub_edx(772, |_, a| a[1] ^ 0x77);
    let lock = SCRATCH + 0x7f40;
    rec(&mut t, 779, 2, move |m, a| a[0] ^ a[1] ^ m.r32(lock));
    t.set_u32(w(0x42), lock);
    let gb = t.img.rel32_target(at + 0x91);
    let n = SCRATCH + 0x1000;
    for case in 1..3u32 {
        release_node_0269(&mut t, n, case == 2);
        release_pool_0269(&mut t, at, gb, n, case);
        t.set_u32(w(0x72), n);
        t.set_u32(lock, 0x40 + case);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
    let ms = img.rel32_target(task + 0x5);
    let ro = img.rel32_target(task + 0xc);
    let sel = SCRATCH + 0x180;
    t.write(sel, &[4, 9, 0x13, 2]);
    t.set_u32(img.word(task + 0x30), sel);
    t.set_u32(img.word(task + 0x29), 0x47);
    let (node, tk) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
    for (k, (done, running, count)) in (0u32..).zip([
        (true, true, 2u32),
        (false, false, 2),
        (true, false, 0),
        (true, false, 3),
    ]) {
        magic_hi::motion(&mut t, ms, done);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(img.word(task + 0x1d), count);
        release_node_0269(&mut t, n, k == 3);
        release_pool_0269(&mut t, at, gb, n, 1 + k % 2);
        t.set_u32(w(0x72), n);
        t.write(node, &[0x4e; 0x20]);
        t.set_u32(node + 8, tk);
        t.write(tk, &[0x7a; 0x20]);
        t.check_entry(task, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0269` 0x10002450 (`r41::strike_0269`, cdecl `(node)`): the
/// count running, no strikes left (the node given back), strikes on the
/// caster (-1) and a selected target with both objects, one and none.
#[test]
fn strike_0269() {
    let Some(mut t) = twin(269) else { return };
    let f = t.at(0x2450);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    magic_r11::object_slots(&mut t);
    for c in [0x58u32, 0xbc] {
        magic_r11::object_start_auto(&mut t, img.rel32_target(f + c));
    }
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            wf(m, a[1 + i as usize], (a[0] & 0xff) as f32 * 0.5 + i as f32);
        }
        0x262
    });
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| 1.25 + f64::from(a[0] & 7));
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, a| 0.5 + f64::from(a[0] & 3));
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4u32 {
            wf(m, a[2] + 4 * i, a[1] as f32 + i as f32);
        }
        0x268
    });
    let take = img.rel32_target(f + 0x39);
    let give = img.rel32_target(f + 0x24);
    let ap = img.rel32_target(f + 0x67);
    let ta = img.rel32_target(f + 0x71);
    // the selection and the characters (the actor point's and the
    // angle's operands, and the strike's own)
    let sel = SCRATCH + 0x180;
    t.write(sel, &[4, 9, 0x13, 2]);
    let chrs = SCRATCH + 0x2_0000;
    for (g, (m, c, me)) in [(ap, (0x26u32, 0x31u32, 0x1fu32)), (ta, (0x11, 0x1c, 0x0a))] {
        t.set_u32(img.word(g + m), sel);
        for i in 0..0x20u32 {
            t.set_u32(img.word(g + c) + 4 * i, chrs + 0x900 * i);
        }
        t.set_u32(img.word(g + me), chrs + 0x900 * 0x1f);
    }
    for o in [0x8au32, 0xda] {
        t.set_u32(w(o), sel);
    }
    for i in 0..0x20u32 {
        t.set_u32(w(0xe5) + 4 * i, chrs + 0x900 * i);
    }
    t.set_u32(w(0x83), 0x83);
    t.set_u32(w(0xd3), chrs + 0x900 * 0x1e);
    let recs_ = SCRATCH + 0x9000;
    let order = w(0x33);
    // (count, strikes, the order's byte, free objects)
    let cases: [(i32, u32, i8, u32); 6] = [
        (3, 2, 0, 2),
        (0, 0, 0, 2),
        (0, 2, -1, 2),
        (-1, 3, 2, 2),
        (0, 1, 1, 1),
        (0, 1, 3, 0),
    ];
    for (k, (count, strikes, who, free)) in (0u32..).zip(cases) {
        let r = SCRATCH + 0x8f00;
        t.set_u32(r, k);
        t.set_u32(r + 4, count.cast_unsigned());
        t.set_u32(r + 8, strikes);
        t.set_u8(order + k, who.cast_unsigned());
        // the pool of 0x140-byte records: `free` of 3 free
        let (idx, cnt, base) = (
            img.word(take + 1),
            img.word(take + 9),
            img.word(take + 0x16),
        );
        for i in 0..3u32 {
            junk(&mut t, recs_ + 0x140 * i, 0x140, k + i);
            t.set_u32(recs_ + 0x140 * i + 0x130, u32::from(i >= free));
            t.set_u16(recs_ + 0x140 * i + 0x124, 0x8000);
        }
        t.set_u32(idx, 0);
        t.set_u32(cnt, 3);
        t.set_u32(base, recs_);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        t.set_u32(node + 0x14, r);
        ownb::dirty_stack(&mut t, 0x2450 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// What `r41::start_up_0269` (`magic_0269` 0x100013e0) needs: the pools
/// (the inlined take's ring is the take's), the part set-up's and the
/// fades' buffers, the sprite table, the caster and the flags, the task
/// list (empty or not).
fn start_up_0269_setup(t: &mut Twin, f: u32, k: u32, listed: bool) {
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(t);
    magic_r11::pools(t, f, 735);
    magic_r11::object_slots(t);
    for s in [215u32, 413] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    rec(t, 271, 1, |_, a| 0x0271_0000 ^ a[0]);
    rec(t, 562, 0, |_, _| 0x562);
    let sprites = SCRATCH + 0x1_0000;
    junk(t, sprites, 0x60 * 0x80 + 0x60, k);
    t.set_u32(w(0x9e), sprites);
    junk(t, w(0xc7), 0x140 * 0x40, k + 1);
    for o in [0x5eu32, 0x7d] {
        junk(t, w(o), 0x38, k + 2);
    }
    t.set_u32(w(0xfb), 0x0000_0047 + k);
    t.set_u32(w(0x104), k & 1);
    t.set_u32(w(0x149), (k >> 1) & 1);
    junk_ops(
        t,
        f,
        &[
            0x16, 0x20, 0x2a, 0x34, 0x42, 0xbe, 0x10a, 0x116, 0x120, 0x12a, 0x154,
        ],
    );
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(0x200), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(0x214), SCRATCH + 0x600);
}

/// `magic_0269` 0x100013e0 (`r41::start_up_0269`, internal) and its load
/// step 0x100012b0 (`r41::load_0269`): the task list empty and not, the
/// actor's slot and side both ways.
#[test]
fn start_up_0269() {
    let Some(mut t) = twin(269) else { return };
    let f = t.at(0x13e0);
    for (k, listed) in [(0u32, false), (1, true), (2, true), (3, false)] {
        start_up_0269_setup(&mut t, f, k, listed);
        ownb::dirty_stack(&mut t, 0x13e0 + k, &[]);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r41::start_up_0269,
        );
    }
    let load = t.at(0x12b0);
    load_cases(&mut t, load, &|t| start_up_0269_setup(t, f, 2, true));
}

/// `magic_0269` 0x10001750 (`r41::launch_0269`, cdecl `(node)`): the
/// count running and out; the yaw wrapped down, kept and up; the task
/// list empty and not.
#[test]
fn launch_0269() {
    let Some(mut t) = twin(269) else { return };
    let f = t.at(0x1750);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let ms = img.rel32_target(f + 0xe);
    let deal = img.rel32_target(f + 0x281);
    for (k, (count, yaw, listed)) in (0u32..).zip([
        (3i32, 0.5f64, false),
        (1, 0.5, false),
        (0, 9.5, true),
        (1, -7.0, true),
        (-2, 3.0, false),
    ]) {
        all_stubs(&mut t);
        magic_r11::pools(&mut t, f, 877);
        magic_r11::object_slots(&mut t);
        for c in [0x8eu32, 0x105, 0x186, 0x207] {
            magic_r11::object_start_auto(&mut t, img.rel32_target(f + c));
        }
        magic_hi::motion(&mut t, ms, k % 2 == 0);
        rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
        t.stub_st0(333, move |_, _| yaw);
        rec(&mut t, 262, 4, |m, a| {
            for i in 0..3u32 {
                wf(m, a[1 + i as usize], (a[0] & 0xff) as f32 * 0.5 + i as f32);
            }
            0x262
        });
        rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
        t.stub_st0(12, |_, a| 1.25 + f64::from(a[0] & 7));
        let rnd = SCRATCH + 0x14c;
        t.set_u32(rnd, 0x1234 + 0x9e37 * k);
        rec(&mut t, 324, 0, move |m, _| {
            let s = m.r32(rnd).wrapping_mul(1_103_515_245).wrapping_add(12345);
            m.w32(rnd, s);
            s >> 8
        });
        // the deal: 3 kinds, 2 of each, 5 dealt
        t.set_u32(img.word(deal + 0x17), 3);
        t.set_u32(img.word(deal + 0x1e), 5);
        for i in 0..3u32 {
            t.set_u32(img.word(deal + 0x46) + 0x1c * i, 2);
        }
        junk(&mut t, w(0x260), 8, k);
        for o in [0x26u32, 0xae, 0x125, 0x1a6, 0x20d] {
            t.set_u32(w(o), 0x0000_0040 + o);
        }
        let node = SCRATCH + 0x5000;
        let tk = SCRATCH + 0x5100;
        t.write(node, &[0x4e; 0x20]);
        t.set_u32(node + 8, tk);
        t.set_u32(node + 0x14, count.cast_unsigned());
        t.write(tk, &[0x7a; 0x20]);
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(w(0x2b9), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(0x2d3), SCRATCH + 0x600);
        ownb::dirty_stack(&mut t, 0x1750 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `magic_0269` 0x10001cf0 (`r41::shard_0269`, cdecl `(node)`): the count
/// running and out (the node given back), a free shard first, later and
/// none; both yaws wrapped down, kept and up.
#[test]
fn shard_0269() {
    let Some(mut t) = twin(269) else { return };
    let f = t.at(0x1cf0);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let give = img.rel32_target(f + 0x1d);
    let shards = SCRATCH + 0x1_0000;
    for (k, (count, free, yaw)) in (0u32..).zip([
        (1i32, 0u32, 0.5f64),
        (2, 0, 0.5),
        (3, 5, 9.5),
        (2, 0x7f, -7.0),
        (2, 0x80, 3.0),
    ]) {
        all_stubs(&mut t);
        rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
        t.stub_st0(333, move |_, a| yaw + f64::from(a[0] & 1));
        let rnd = SCRATCH + 0xa00;
        for s in [314u32, 404] {
            rec(&mut t, s, 1, move |m, a| {
                let c = m.r32(rnd);
                m.w32(rnd, c + 1);
                (s << 16) ^ a[0] ^ c
            });
            t.stub_st0(s, move |m, a| {
                f64::from(m.r32(rnd) % 5) * 0.375 + f64::from(f32::from_bits(a[0]))
            });
        }
        rec(&mut t, 262, 4, |m, a| {
            for i in 0..3u32 {
                wf(m, a[1 + i as usize], (a[0] & 0xff) as f32 * 0.5 + i as f32);
            }
            0x262
        });
        filler_16(&mut t, 235);
        for s in [328u32, 329] {
            rec(&mut t, s, 3, move |m, a| {
                let ang = f32::from_bits(a[2]);
                for i in 0..16u32 {
                    let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0;
                    wf(m, a[0] + 4 * i, v);
                }
                s
            });
        }
        for s in [229u32, 248] {
            rec(&mut t, s, 3, move |m, a| {
                for i in 0..4u32 {
                    let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i) + i as f32;
                    wf(m, a[0] + 4 * i, v);
                }
                s
            });
        }
        t.set_u32(w(0x31), shards);
        for i in 0..0x80u32 {
            let sh = shards + 0x60 * i;
            junk(&mut t, sh, 0x60, k + i);
            t.set_u32(sh + 0x58, u32::from(i != free));
        }
        t.set_u32(w(0x7a), 0x47);
        t.set_u32(w(0x14f), 0x48);
        t.set_u32(w(0x135), 0x49);
        let node = SCRATCH + 0x900;
        magic_hi::lone_node(&mut t, give, node);
        t.set_u32(node + 0x14, count.cast_unsigned());
        ownb::dirty_stack(&mut t, 0x1cf0 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}
