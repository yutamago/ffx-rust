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
        for i in 0..8u32 {
            let rec_ = w(0x23e) + 0x100 * i;
            let parent = if i == 3 { w(0x2e7) } else { 0 };
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
        t.set_u16(e, 0x10);
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
fn sprite_stubs_0370(t: &mut Twin, sf: u32, tbl: u32) -> u32 {
    all_stubs(t);
    let lock = SCRATCH + 0x3100;
    t.set_u32(t.img.word(sf + 0x28e), lock);
    floats(t, t.img.word(sf + 0x181), 1, 3);
    rec(t, 941, 1, move |_, _| tbl);
    rec(t, 772, 2, |_, a| a[0] ^ a[1]);
    t.stub_edx(772, |_, _| 0x0772_0002);
    rec(t, 773, 4, |_, a| u32::from(a[3] & 1 == 0));
    for (s, n) in [(774u32, 5usize), (778, 6)] {
        rec(t, s, n, move |m, a| {
            let mut v = s;
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
    let lock = sprite_stubs_0370(&mut t, sf, tbl);
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
    let lock = sprite_stubs_0370(&mut t, sf, tbl);
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
