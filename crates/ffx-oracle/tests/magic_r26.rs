//! Round 26's own ports (`ffx_magic::r26`) against each original (the
//! copies bound to earlier ports are tested with those ports). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_owna;
mod magic_r11;

use ffx_magic::{Ret, r26};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{filler, rec, recs, twin};

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// The load step of a start-up (at `load`): busy, asked or not, the data
/// loaded or not; `setup` prepares the start-up before each case.
fn load_cases(t: &mut Twin, load: u32, setup: &dyn Fn(&mut Twin)) {
    load_cases_d(t, load, 0, setup);
}

/// [`load_cases`] for either form of the step (`d` = 1: the 119-byte
/// one, its operands from `+0x1e` on a byte later).
fn load_cases_d(t: &mut Twin, load: u32, d: u32, setup: &dyn Fn(&mut Twin)) {
    for (busy, loaded, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        setup(t);
        rec(t, 225, 1, move |_, _| busy);
        rec(t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(t, 0x18);
        t.set_u32(at, loaded);
        for o in [0x26u32 + d, 0x46 + d] {
            let at = op(t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        junk_ops(t, load, &[0x61 + d, 0x6b + d]);
        magic_h6::scrub(t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// The target byte (at `idx`, its neighbour junk) behind the operand
/// `sel` of `f` and the table the operand `tab` names.
fn target(t: &mut Twin, f: u32, (sel, tab): (u32, u32), i: u8) {
    let idx = SCRATCH + 0x1e0;
    t.set_u8(idx, i);
    t.set_u8(idx + 1, 0x77);
    let at = t.img.word(f + sel);
    t.set_u32(at, idx);
    let tab = t.img.word(f + tab);
    for i in 0..0x20 {
        t.set_u32(tab + 4 * i, 0x0000_0080 + 3 * i);
    }
}

/// What `r26::start_up_0246` (`magic_0246` 0x10001340) needs: the pools,
/// the part set-up's and the object starts' buffers, the actor and the
/// target, the task list (empty or not); slot 262 writes a point from its
/// first word, 268 a vector, 229 and 248 combine theirs, the float slots
/// answer from their word (slot 333 the case's angle).
fn start_up_0246_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1153);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    let yaw = [0.25f64, 3.0, 9.5, 2.0][k as usize % 4];
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    for s in [364u32, 16] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| {
            f64::from(a[0] & 0xff) * 0.125 - 0.75 + f64::from(s) * 0.03125
        });
    }
    filler(t, 235, 1, 16);
    for s in [328u32, 329] {
        rec(t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ s;
                m.w32(a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (m.r32(a[1] + 4 * i) & 3) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4)]);
    for call in [0x5du32, 0x31d] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    // the actor, the target byte and its table (three reads)
    for o in [0x7du32, 0x8a] {
        t.set_u32(w(t, o), 0x0000_0040 + k);
    }
    target(t, f, (0x1fb, 0x246), (2 + k) as u8);
    // the actor block's battle flag
    let me = t.img.rel32_target(f + 0x3f4);
    t.set_u32(t.img.word(me + 0x05), battle);
    // junk in the globals it stores constants to
    junk_ops(
        t,
        f,
        &[
            0x26, 0x30, 0x90, 0x102, 0x108, 0x112, 0x11c, 0x126, 0x130, 0x13a, 0x144, 0x14e, 0x158,
            0x203, 0x211, 0x225, 0x22d, 0x233, 0x2ee, 0x2f8, 0x302, 0x314, 0x358, 0x368, 0x378,
            0x38f, 0x395, 0x39f, 0x3a9, 0x3b3, 0x3bd, 0x3c7, 0x3d1, 0x3db, 0x3eb, 0x46d,
        ],
    );
    t.set_u32(w(t, 0xe9), 0x5a5a_5a5a);
    t.set_u32(w(t, 0x3e5), 0x5a5a_5a5a);
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x407), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x41a), SCRATCH + 0x600);
}

/// `magic_0246` 0x10001340 (`r26::start_up_0246`, internal) and its load
/// step 0x10001210 (`r9_load::load_0246`): in and out of a battle, the
/// task list empty and not, the yaw above, inside and below its wrap.
#[test]
fn start_up_0246() {
    let Some(mut t) = twin(246) else { return };
    let f = t.at(0x1340);
    for (k, battle, listed) in [
        (0u32, 0u32, false),
        (1, 1, true),
        (2, 0, true),
        (3, 1, false),
    ] {
        start_up_0246_setup(&mut t, f, k, (battle, listed));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0246);
    }
    let load = t.at(0x1210);
    load_cases(&mut t, load, &|t| start_up_0246_setup(t, f, 2, (1, true)));
}

/// `magic_0246` 0x10001810 (`r26::task_0246`, cdecl `(node)`): the delay
/// running; the count above 1, at 1, at 0 and below (the object and the
/// target point); the time short of its end and past it (the node given
/// back), and a NaN time.
#[test]
fn task_0246() {
    let Some(mut t) = twin(246) else { return };
    let f = t.at(0x1810);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x5f);
    magic_r11::object_start_auto(&mut t, start);
    rec(&mut t, 262, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    recs(&mut t, &[(325, 2), (326, 4)]);
    // (delay, count, time)
    for (k, (delay, count, time)) in [
        (3u32, 1i32, 0.5f32),
        (0, 3, 0.5),
        (0, 1, 0.25),
        (0, 0, 0.97),
        (0, -1, 0.97),
        (0, 2, 1.25),
        (0, 2, f32::NAN),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0x11), delay);
        t.set_u32(w(&t, 0x33), count.cast_unsigned());
        t.set_u32(w(&t, 0xf3), time.to_bits());
        t.set_u32(w(&t, 0x178), (2.5 + k as f32).to_bits());
        target(&mut t, f, (0x7b, 0xa5), (3 + k) as u8);
        // (the count's -1 goes back to the count itself: no junk there)
        junk_ops(&mut t, f, &[0x85, 0x8f, 0xcb, 0x196]);
        // the twelve flags and the word after them (a wider store would
        // reach it)
        t.write(w(&t, 0xda), &[0x5c; 0x10]);
        // the node: alone on its child list, linked between two others
        let node = SCRATCH + 0x900;
        let (prev, next) = (SCRATCH + 0x940, SCRATCH + 0x980);
        t.write(node, &[0x66; 0x20]);
        t.write(prev, &[0x67; 0x20]);
        t.write(next, &[0x68; 0x20]);
        t.set_u32(node, prev);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, node);
        t.set_u32(node + 0xc, 0);
        let back = t.img.rel32_target(f + 0x19f);
        let free = SCRATCH + 0xa00;
        let (count, head) = (t.img.word(back + 0x8e), t.img.word(back + 0x74));
        magic_r11::ring(&mut t, count, head, 3, free);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
    assert!(t.u32(magic_owna::row(262)) >= 2, "the target point ran");
}

/// What `r26::start_up_0180` (`magic_0180` 0x100042f0) needs: the pools
/// (the aim task and the pool reset take from the same ring), the part set-up's
/// and the object start's buffers, the actor and the target, the sprite's
/// owner, the task list (empty or not).
fn start_up_0180_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1259);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    let yaw = [0.25f64, 3.0, 9.5, 2.0][k as usize % 4];
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    filler(t, 235, 1, 16);
    for s in [328u32, 329] {
        rec(t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ s;
                m.w32(a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (m.r32(a[1] + 4 * i) & 3) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4)]);
    let start = t.img.rel32_target(f + 0x74);
    magic_r11::object_start_auto(t, start);
    // the actor (two reads), the target byte and its table
    for o in [0x94u32, 0xa1] {
        t.set_u32(w(t, o), 0x0000_0040 + k);
    }
    target(t, f, (0x115, 0x181), (2 + k) as u8);
    // the sprite's owner `**G_O` and its colour `Q`
    let o = SCRATCH + 0x1f0;
    t.set_u32(o, 0x0b0b_0000 + k);
    t.set_u32(w(t, 0x305), o);
    // the aim task's two points (its own operands)
    let aim = t.img.rel32_target(f + 0x435);
    for (o, v) in [(0x18u32, 1.5f32), (0x4a, 2.0), (0x28, -3.0)] {
        t.set_u32(t.img.word(aim + o), (v + k as f32).to_bits());
    }
    // (the pool reset takes from the same free ring as the start-up and
    // the aim task: `pools` seeds it)
    // the actor block's battle flag
    let me = t.img.rel32_target(f + 0x449);
    t.set_u32(t.img.word(me + 0x05), battle);
    // junk in the globals it stores constants to
    junk_ops(
        t,
        f,
        &[
            0x31, 0x3b, 0x45, 0xa7, 0x11b, 0x12d, 0x137, 0x141, 0x14b, 0x155, 0x15f, 0x169, 0x173,
            0x30d, 0x31d, 0x32d, 0x33b, 0x349, 0x357, 0x36f, 0x379, 0x385, 0x38b, 0x395, 0x39f,
            0x3a9, 0x3be, 0x3da, 0x3e6, 0x3f0, 0x3fa, 0x409, 0x413, 0x42c, 0x440,
        ],
    );
    for o in [0x110u32, 0x369, 0x3b8, 0x3e0, 0x41d] {
        t.set_u32(w(t, o), 0x5a5a_5a5a);
    }
    // the task list (the pool reset appends to the same one)
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x45c), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x470), SCRATCH + 0x600);
}

/// `magic_0180` 0x100042f0 (`r26::start_up_0180`, internal) and its load
/// step 0x100041c0 (`r9_load::load_0180`): in and out of a battle, the
/// task list empty and not, the yaw above, inside and below its wrap.
#[test]
fn start_up_0180() {
    let Some(mut t) = twin(180) else { return };
    let f = t.at(0x42f0);
    for (k, battle, listed) in [
        (0u32, 0u32, false),
        (1, 1, true),
        (2, 0, true),
        (3, 1, false),
    ] {
        start_up_0180_setup(&mut t, f, k, (battle, listed));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0180);
    }
    let load = t.at(0x41c0);
    load_cases(&mut t, load, &|t| start_up_0180_setup(t, f, 2, (1, true)));
}

/// `magic_0180` 0x10004870 (`r26::sparks_0180`, cdecl `(p)`): no
/// records, all busy, four free among busy ones (the cursor wrapping),
/// fewer free than four.
#[test]
fn sparks_0180() {
    let Some(mut t) = twin(180) else { return };
    let f = t.at(0x4870);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(325, 2)]);
    for s in [314u32, 307, 404] {
        rec(&mut t, s, 1, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
        let cnt = SCRATCH + 0x7a0 + 4 * (s % 4);
        t.stub_st0(s, move |m, a| {
            let k = m.r32(cnt).wrapping_add(1);
            m.w32(cnt, k);
            f64::from(k) * 0.375 - 1.25 + f64::from(a.first().copied().unwrap_or(0) & 0xff)
        });
    }
    let recs_at = SCRATCH + 0x2000;
    let o = SCRATCH + 0x1f0;
    t.set_u32(o, 0x0c0c_0000);
    t.set_u32(o + 4, 0x0d0d_0000);
    t.set_u32(w(&t, 0x80), o);
    let p = SCRATCH + 0x1c00;
    t.write(p, &[0x31; 0x40]);
    // (records, cursor, busy mask)
    for (n, cur, busy) in [
        (0i32, 0i32, 0u32),
        (3, 1, 0b111),
        (8, 6, 0b0101_0010),
        (5, 2, 0b1_1011),
    ] {
        t.set_u32(w(&t, 0x12), n.cast_unsigned());
        t.set_u32(w(&t, 0x17), cur.cast_unsigned());
        t.set_u32(w(&t, 0x27), recs_at);
        for i in 0..8u32 {
            let r = recs_at + 0x50 * i;
            t.write(r, &[0x27; 0x50]);
            t.set_u32(r + 0x48, (busy >> i) & 1);
            for (j, x) in (0u32..).zip([0.5f32, -1.0, 2.25]) {
                t.set_u32(r + 0xc + 4 * j, (x + i as f32).to_bits());
            }
        }
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[p], Eax::Checked);
    }
}

/// `r26::draw_0426` (`magic_0426` 0x10003130, `ecx` = the cursor), its
/// thunk 0x100038a0 and `r26::mesh_packets_0426` under it: the pass
/// already done; the children visited, then the delay running, the shown
/// bit clear, no model; the draw with and without bit 2 (slots 355/354)
/// and bit 4 (the leave and slots 357/356); the model's parts with strips
/// of faces (a strip with none; a part's strips ended by 0xff or by a
/// byte other than 0x10), and slot 941 moving the packet cursor near its
/// buffer's end (the buffer swap), past its start (the closing end mark)
/// or not at all.
#[test]
#[allow(clippy::too_many_lines)] // the stubs, the model and the cases
fn draw_0426() {
    let Some(mut t) = twin(426) else { return };
    let f = t.at(0x3130);
    // the methods +0x1c: slot 1001, logging its cursor and the node in it
    let vt = SCRATCH + 0x880;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1001);
    rec(&mut t, 1001, 1, |m: &mut dyn StubMem, a: &[u32]| {
        let at = SCRATCH + 0x7b0;
        let v = m.r32(at).rotate_left(5) ^ m.r32(a[0]) ^ a[0];
        m.w32(at, v);
        0x1001
    });
    let blk = SCRATCH + 0x1_0000;
    rec(&mut t, 264, 1, move |_, _| blk);
    recs(&mut t, &[(265, 1), (355, 2), (354, 2), (357, 3), (356, 3)]);
    // slot 941: the part from its word; by the mode word its second call
    // moves the cursor near the buffer's end (a swap follows) or a little
    // past its start (the end mark at the end)
    let parts = SCRATCH + 0x6000;
    let cnt = SCRATCH + 0x7a0;
    let mode = SCRATCH + 0x7a4;
    rec(&mut t, 941, 1, move |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(cnt).wrapping_add(1);
        m.w32(cnt, k);
        if k == 2 {
            match m.r32(mode) {
                1 => {
                    let end = m.r32(blk + 0xd8);
                    m.w32(blk + 0xdc, end - 0x100);
                }
                2 => {
                    let start = m.r32(blk + 0xd4);
                    m.w32(blk + 0xdc, start + 0x40);
                }
                _ => {}
            }
        }
        parts + 0x400 * (a[0] & 3)
    });
    for s in [544u32, 588] {
        rec(&mut t, s, 2, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..4 {
                m.w32(
                    a[1] + 4 * i,
                    (0.5 * i as f32 + (a[0] + s) as f32 * 0.125).to_bits(),
                );
            }
            s
        });
    }
    filler(&mut t, 327, 2, 16);
    rec(&mut t, 232, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) * 0.75 + 0.25;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x232
    });
    rec(&mut t, 233, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) * f32::from_bits(a[2]);
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x233
    });
    rec(&mut t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (m.r32(a[1] + 4 * i) & 3) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(&mut t, 231, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(1);
            m.w32(a[0] + 4 * i, v);
        }
        0x231
    });
    rec(&mut t, 277, 1, |_, a| 0x0277_0000 ^ a[0]);
    t.stub_st0(277, |_, a| f64::from(a[0] & 0xff) * 0.0625 + 0.5);
    // the model: two parts, entries 16 bytes apart from +0x10
    let model = SCRATCH + 0x5800;
    t.write(model, &[0x29; 0x40]);
    t.set_u8(model + 4, 2);
    t.set_u32(model + 0x10, 0x0000_0102);
    // each part: the strips at +0x40, the vertices at +0x200
    for p in 0..4u32 {
        let at = parts + 0x400 * p;
        t.write(at, &[0x3b; 0x400]);
        t.set_u32(at + 4, 0x40);
        t.set_u32(at + 8, 0x200);
        for v in 0..0x20u32 {
            for j in 0..4 {
                t.set_u32(
                    at + 0x200 + 0x10 * v + 4 * j,
                    (v as f32 + 0.25 * j as f32 + p as f32).to_bits(),
                );
            }
        }
        // strips: three faces, none, two faces; then the end byte
        let mut s = at + 0x40;
        for (n, last) in [(3u16, false), (0, false), (2, true)] {
            t.set_u8(s + 1, 0x10);
            t.set_u16(s + 2, n);
            t.set_u32(s + 8, 0x0800_0000 + s);
            let mut face = s + 0x20;
            for i in 0..u32::from(n) {
                for j in 0..9u32 {
                    t.set_u16(face + 2 * j, ((i * 3 + j * 5 + p) % 0x1f) as u16);
                }
                face += 0x1c;
            }
            // part 1 ends its strips with a byte other than 0x10 or 0xff
            t.set_u8(
                face + 1,
                if last {
                    if p == 1 { 0x22 } else { 0xff }
                } else {
                    0x10
                },
            );
            s = face;
        }
    }
    let c = SCRATCH + 0x100;
    let n0 = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x1400;
    let cam = SCRATCH + 0x1600;
    let obj = SCRATCH + 0x1700;
    // (pass done, children, delay, flags, model, parent)
    for (k, (done, kids, delay, flags, with_model, par)) in [
        (true, 0u32, 0u16, 0x17u16, true, false),
        (false, 2, 3, 0x17, true, false),
        (false, 1, 0, 0x16, true, false),
        (false, 0, 0, 0x17, false, false),
        (false, 1, 0, 0x01, true, false),
        (false, 0, 0, 0x13, true, false),
        (false, 2, 0, 0x17, true, true),
        (false, 0, 0xffff, 0x11, true, false),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(cnt, 0);
        t.set_u32(mode, k % 3);
        t.write(n0, &[0x5c; 0x100]);
        t.write(c, &[0x11; 0x10]);
        t.set_u32(c, n0);
        t.set_u8(c + 5, 7);
        t.set_u8(n0 + 0x1a, if done { 7 } else { 3 });
        let kid = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
        t.set_u32(n0 + 0x10, if kids == 0 { n0 } else { kid(0) });
        for i in 0..kids {
            t.write(kid(i), &[0x22; 0x40]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { n0 } else { kid(i + 1) });
        }
        t.set_u16(n0 + 0xba, delay);
        t.set_u16(n0 + 0xb4, flags);
        t.set_u32(n0 + 0x90, if with_model { model } else { 0 });
        // the second part: one ending with 0xff (the count runs out) or
        // part 1, whose strips end with another byte
        t.set_u32(
            model + 0x20,
            if k.is_multiple_of(2) { 0x103 } else { 0x101 },
        );
        t.write(cam, &[0x45; 0x100]);
        t.set_u32(n0 + 0x8c, cam);
        t.write(obj, &[0x46; 0x60]);
        t.set_u32(obj + 0x54, 0x0000_0540 + k);
        t.set_u32(n0 + 0x88, obj);
        t.set_u32(n0 + 0x80, 0x0800_0080 + k);
        t.set_u32(n0 + 0x84, 0x0800_0084 + k);
        t.set_u32(n0 + 0x94, (1.5 + k as f32).to_bits());
        t.set_u32(n0 + 0x98, (0.25 * k as f32 - 1.0).to_bits());
        // the leave's parent (none: `+0x14` 0)
        t.write(parent, &[0x44; 0x20]);
        t.set_u32(n0 + 0x14, if par { parent } else { 0 });
        t.set_u32(n0 + 4, SCRATCH + 0x1480);
        t.set_u32(n0 + 8, SCRATCH + 0x1490);
        t.write(blk, &[0x5d; 0x3a00]);
        junk_ops(&mut t, f, &[0x3bf]);
        magic_h6::scrub(&mut t);
        t.check(f, Call::this(c), &[], Eax::Checked, r26::draw_0426);
        if k == 2 {
            // the thunk: `ecx` from its stack word
            t.set_u8(c + 5, 9);
            t.set_u16(n0 + 0xba, 0);
            t.check_entry(t.at(0x38a0), Call::default(), &[c], Eax::Checked);
        }
    }
    assert!(t.u32(magic_owna::row(941)) >= 4, "the parts ran");
}

/// What `r26::start_up_0442` (`magic_0442` 0x100027f0) needs: the part
/// set-up's buffers, the object starts', the actors for the mean position
/// (two), the target and its table, the kind slot 390 answers (listed or
/// not), the pair count, the free ring (8 nodes) and the task list.
fn start_up_0442_setup(t: &mut Twin, f: u32, k: u32, (battle, listed, kinds): (u32, bool, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1814);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    let yaw = [0.25f64, 3.0, 9.5, 2.0][k as usize % 4];
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    for s in [12u32, 314, 404] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        let cnt = SCRATCH + 0x7a0 + 4 * (s % 4);
        t.stub_st0(s, move |m, a| {
            let c = m.r32(cnt).wrapping_add(1);
            m.w32(cnt, c);
            f64::from(c) * 0.375 - 1.25 + f64::from(f32::from_bits(a[0]))
        });
    }
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4), (271, 1)]);
    for call in [0x1f3u32, 0x293] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    // the mean position's two actors
    let m = t.img.rel32_target(f + 0x4c);
    t.set_u32(t.img.word(m + 0x11), 2);
    magic_r11::first_actor(t, m, 0x4d, 0x58);
    // the kind: the list's last entry, or one not in it
    let mut l = w(t, 0x70);
    while t.u32(l + 4) != u32::MAX {
        l += 4;
    }
    let kind = if kinds { t.u32(l) } else { 0x7777_0001 };
    rec(t, 390, 1, move |_, _| kind);
    for (sel, tab) in [(0x51u32, 0x9e), (0x94, 0x9e), (0x10f, 0x119)] {
        target(t, f, (sel, tab), (2 + k) as u8);
    }
    t.set_u32(w(t, 0xd3), 0x0000_0040 + k);
    t.set_u32(w(t, 0x17a), [2u32, 1, 0, 3][k as usize % 4]);
    t.set_u32(w(t, 0x386), battle);
    t.set_u32(w(t, 0x3bd), battle);
    // the free ring and the task list
    magic_r11::ring(t, w(t, 0x3fc), w(t, 0x48e), 8, SCRATCH + 0xb00);
    junk_ops(
        t,
        f,
        &[
            0x27, 0x31, 0x36e, 0x38c, 0x398, 0x39e, 0x3c8, 0x3ce, 0x3d8, 0x3e2, 0x402, 0x40c,
            0x416, 0x420, 0x42a, 0x434, 0x43e, 0x448, 0x452, 0x45c, 0x466, 0x470, 0x528, 0x532,
            0x53c, 0x546, 0x550, 0x55a, 0x564, 0x56e, 0x578, 0x582, 0x58b, 0x591, 0x59b, 0x5a5,
        ],
    );
    t.set_u32(w(t, 0x3ec), 0x5a5a_5a5a);
    t.set_u32(w(t, 0x3f5), 0x5a5a_5a5a);
    t.write(w(t, 0x1ad) - 0x14, &[0x5b; 0x100]);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x4e9), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x504), SCRATCH + 0x600);
}

/// `magic_0442` 0x100027f0 (`r26::start_up_0442`, internal) and its load
/// step 0x100026c0 (`r9_load::load_0442`): the kind listed or not (the
/// target's point or the height), 0 to 3 object pairs (the count word's
/// sign), in and out of a battle, the task list empty and not, the yaw
/// above, inside and below its wrap.
#[test]
fn start_up_0442() {
    let Some(mut t) = twin(442) else { return };
    let f = t.at(0x27f0);
    for (k, battle, listed, kinds) in [
        (0u32, 0u32, false, true),
        (1, 1, true, false),
        (2, 0, true, true),
        (3, 1, false, false),
    ] {
        start_up_0442_setup(&mut t, f, k, (battle, listed, kinds));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0442);
    }
    let load = t.at(0x26c0);
    load_cases(&mut t, load, &|t| {
        start_up_0442_setup(t, f, 3, (1, true, true));
    });
}

/// What `r26::start_up_0260` (`magic_0260` 0x10003d40) needs: the pools
/// (the aim's take and the start-up's inlined ones share the ring), the
/// part set-up's and the object starts' buffers, the actor and the
/// target, the shards' part word, the task list (empty or not).
fn start_up_0260_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1977);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    let yaw = [0.25f64, 3.0, 9.5, 2.0][k as usize % 4];
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    rec(t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    let cnt = SCRATCH + 0x7a0;
    t.stub_st0(314, move |m, a| {
        let c = m.r32(cnt).wrapping_add(1);
        m.w32(cnt, c);
        f64::from(c) * 0.375 - 1.25 + f64::from(f32::from_bits(a[0]))
    });
    filler(t, 235, 1, 16);
    for s in [328u32, 329] {
        rec(t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ s;
                m.w32(a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (m.r32(a[1] + 4 * i) & 3) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    rec(t, 325, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    rec(t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4)]);
    for call in [0x76u32, 0x3ff] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    for o in [0x96u32, 0xa3] {
        t.set_u32(w(t, o), 0x0000_0040 + k);
    }
    target(t, f, (0x117, 0x183), (2 + k) as u8);
    let pw = SCRATCH + 0x1f0;
    t.set_u32(pw, 0x0c0c_0000 + k);
    t.set_u32(w(t, 0x4e4), pw);
    t.set_u32(w(t, 0x60e), battle);
    t.set_u32(w(t, 0x629), battle);
    // the shards' blocks: junk
    t.write(w(t, 0x3da) - 0x138, &[0x5e; 0x3a0 * 5]);
    junk_ops(
        t,
        f,
        &[
            0x33, 0x3d, 0x47, 0xa9, 0x11d, 0x12f, 0x139, 0x143, 0x14d, 0x157, 0x161, 0x16b, 0x175,
            0x317, 0x5ee, 0x5fa, 0x604, 0x634, 0x63a, 0x644, 0x64e,
        ],
    );
    for o in [0x112u32, 0x658, 0x661] {
        t.set_u32(w(t, o), 0x5a5a_5a5a);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x6dd), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x6f1), SCRATCH + 0x600);
}

/// `magic_0260` 0x10003d40 (`r26::start_up_0260`, internal) and its load
/// step 0x10003a90 (`r9_load::load_0260`): in and out of a battle, the
/// task list empty and not, the yaw above, inside and below its wrap.
#[test]
fn start_up_0260() {
    let Some(mut t) = twin(260) else { return };
    let f = t.at(0x3d40);
    for (k, battle, listed) in [
        (0u32, 0u32, false),
        (1, 1, true),
        (2, 0, true),
        (3, 1, false),
    ] {
        start_up_0260_setup(&mut t, f, k, (battle, listed));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0260);
    }
    let load = t.at(0x3a90);
    load_cases(&mut t, load, &|t| start_up_0260_setup(t, f, 2, (1, true)));
}

/// What `r26::start_up_0305` (`magic_0305` 0x100013e0) needs: the pools
/// (the aim task and the pool reset take from the start-up's ring), the
/// part set-up's and the object starts' buffers, the fades', the actor and
/// the target, the task list (empty or not). `h` caps the target's height
/// in the odd cases.
fn start_up_0305_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 2001);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    let top = [40.0f64, 1.0, 40.0, -3.0][k as usize % 4];
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, move |_, _| top);
    filler(t, 235, 1, 16);
    for s in [328u32, 329] {
        rec(t, s, 3, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i).rotate_left(3) ^ a[2] ^ s;
                m.w32(a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (m.r32(a[1] + 4 * i) & 3) as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(t, 248, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x248
    });
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4), (271, 1)]);
    for call in [0x2d2u32, 0x332, 0x464, 0x53d, 0x648] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    // the fades: the records their `ecx` names, junk first
    for o in [0x6dau32, 0x700] {
        t.write(w(t, o), &[0x5f; 0x40]);
    }
    for o in [0x9eu32, 0x2f2, 0x484] {
        t.set_u32(w(t, o), 0x0000_0040 + k);
    }
    for (sel, tab) in [(0x5eu32, 0x68u32), (0xb1, 0xc3)] {
        target(t, f, (sel, tab), (2 + 5 * k) as u8);
    }
    let me = t.img.rel32_target(f + 0x72e);
    t.set_u32(t.img.word(me + 0x05), battle);
    junk_ops(
        t,
        f,
        &[
            0x32, 0x3c, 0x46, 0x317, 0x323, 0x329, 0x340, 0x34e, 0x35c, 0x36d, 0x383, 0x391, 0x39f,
            0x3af, 0x3bb, 0x3c5, 0x3cf, 0x3d9, 0x3e3, 0x3ed, 0x3f7, 0x401, 0x411, 0x41d, 0x423,
            0x42d, 0x436, 0x43c, 0x499, 0x4a7, 0x4b5, 0x4c3, 0x4d1, 0x54b, 0x559, 0x567, 0x575,
            0x58c, 0x5e9, 0x5ef, 0x5f9, 0x603, 0x60d, 0x617, 0x621, 0x62b, 0x635, 0x63f, 0x656,
            0x664, 0x672, 0x680, 0x696, 0x6e2, 0x71f, 0x729,
        ],
    );
    for o in [0x40bu32, 0x446, 0x5d5] {
        t.set_u32(w(t, o), 0x5a5a_5a5a);
    }
    // the task list (the aim task and the pool reset append to it)
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x741), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x755), SCRATCH + 0x600);
}

/// `magic_0305` 0x100013e0 (`r26::start_up_0305`, internal) and its load
/// step 0x100012b0 (`r9_load::load_0305`): in and out of a battle, the
/// task list empty and not, the target's height capped or not, near and
/// far targets.
#[test]
fn start_up_0305() {
    let Some(mut t) = twin(305) else { return };
    let f = t.at(0x13e0);
    for (k, battle, listed) in [
        (0u32, 0u32, false),
        (1, 1, true),
        (2, 0, true),
        (3, 1, false),
    ] {
        start_up_0305_setup(&mut t, f, k, (battle, listed));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0305);
    }
    let load = t.at(0x12b0);
    load_cases(&mut t, load, &|t| start_up_0305_setup(t, f, 2, (1, true)));
}

/// `magic_0430` 0x10003630 (`m0588::actor_matrix` through
/// `r26::ACTOR_MATRIX_0430`): two actors, the matrix's diagonal scaled
/// and turned (as `magic_0588.rs` tests its copy).
#[test]
fn actor_matrix_0430() {
    let Some(mut t) = twin(430) else { return };
    let at = t.at(0x3630);
    let s = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x18), s);
    let actors = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x11), actors);
    let hs = t.img.word(at + 0x25);
    for i in 0..8u32 {
        t.set_u32(hs + 4 * i, 0x00ac_0000 + i);
    }
    for (slot, n) in [(277u32, 1usize), (15, 1), (16, 1), (17, 1)] {
        rec(&mut t, slot, n, move |_, _| slot);
        t.stub_st0(slot, move |_, a| {
            0.375 + f64::from(slot) / 64.0 + f64::from(a[0] & 7)
        });
    }
    for (slot, n) in [
        (210u32, 2usize),
        (211, 2),
        (235, 1),
        (330, 3),
        (328, 3),
        (329, 3),
        (257, 2),
    ] {
        rec(&mut t, slot, n, move |m, a| {
            if slot == 210 {
                for i in 0..16u32 {
                    m.w32(a[1] + 4 * i, (1.5 + i as f32).to_bits());
                }
            }
            if slot >= 328 {
                magic_owna::dump(m, 0x100 + slot % 0x40, a[0], 16);
            }
            slot
        });
    }
    for b in [1u8, 3] {
        t.write(s, &[0x2b; 0x1000]);
        for i in 0..0x40u32 {
            t.set_u32(s + 0xb80 + 4 * i, (0.5 + 0.25 * i as f32).to_bits());
        }
        t.write(actors, &[b, b + 1, b + 2, b + 3]);
        magic_h6::scrub(&mut t);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::m0588::actor_matrix(g, img, fr)
        });
    }
}

/// What `magic_0395`'s stages (at `f`, `len` bytes, `B`'s operand `b_op`)
/// need: the effect block `B` (junk, a point and an angle), the pools,
/// the object and point slots, the actor block's slots, the task list.
fn stage_0395_common(t: &mut Twin, f: u32, len: u32, b_op: u32) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, len);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.375 - 1.5);
    recs(t, &[(413, 1), (215, 1)]);
    let b = SCRATCH + 0x1_0000;
    t.write(b, &[0x5d; 0x1600]);
    for i in 0..4u32 {
        t.set_u32(b + 0x10a0 + 4 * i, (1.5 * i as f32 - 2.0).to_bits());
    }
    t.set_u32(b + 0x1524, 0.75f32.to_bits());
    t.set_u32(w(t, b_op), b);
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
}

/// `magic_0395` 0x10004950 (`r26::stage_0395`, a table entry): the fade
/// on and off, in and out of a battle, the task list empty and not.
#[test]
fn stage_0395() {
    let Some(mut t) = twin(395) else { return };
    let f = t.at(0x4950);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    for (k, (faded, battle, listed)) in [
        (1u32, 0u32, false),
        (1, 1, true),
        (0, 0, true),
        (0, 1, false),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        stage_0395_common(&mut t, f, 621, 0xc);
        let start = t.img.rel32_target(f + 0x20);
        magic_r11::object_start_auto(&mut t, start);
        t.set_u32(w(&t, 0x157), faded);
        for o in [0x186u32, 0x1b1, 0x212, 0x23d] {
            t.set_u32(w(&t, o), battle);
        }
        t.set_u32(w(&t, 0x165), 0x0000_0050 + k);
        junk_ops(
            &mut t,
            f,
            &[
                0x16a, 0x176, 0x18d, 0x1a8, 0x1bd, 0x1c7, 0x1d0, 0x219, 0x234, 0x249, 0x253, 0x25c,
            ],
        );
        for o in [0x1f7u32, 0x267] {
            t.set_u32(w(&t, o), 0x5a5a_5a5a);
        }
        t.set_u32(w(&t, 0x12c), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(&t, 0x13f), SCRATCH + 0x600);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0395` 0x10004380 (`r26::stage_0395_a`, a table entry): 0 to 4
/// actors dealt from two kinds (slot 324 picks), each kind listed or not
/// by the actor's parity (slot 390), the effect's flag (the callbacks,
/// the indices, the spark sizes and the battle block) on and off, the
/// task list empty and not.
#[test]
#[allow(clippy::too_many_lines)] // the deal, the kinds, the ring and the cases
fn stage_0395_a() {
    let Some(mut t) = twin(395) else { return };
    let f = t.at(0x4380);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let deal = t.img.rel32_target(f + 0x20);
    let kc = t.img.rel32_target(f + 0x7c);
    // the kind check's list: its last entry is "listed"
    let mut l = t.img.word(kc + 0x30);
    while t.u32(l + 4) != u32::MAX {
        l += 4;
    }
    let listed_kind = t.u32(l);
    rec(&mut t, 390, 1, move |_, a| {
        if a[0] & 1 == 0 {
            listed_kind
        } else {
            0x7777_0001
        }
    });
    // an unlisted kind's answer: slot 534's, 0 for some characters
    rec(&mut t, 534, 1, |_, a| (a[0] >> 1) & 1);
    let cnt = SCRATCH + 0x7a0;
    rec(&mut t, 324, 0, move |m: &mut dyn StubMem, _: &[u32]| {
        let k = m.r32(cnt).wrapping_add(1);
        m.w32(cnt, k);
        k.wrapping_mul(0x9e37_79b9) >> 7
    });
    for (k, (n, flag, listed)) in [
        (0u32, (2u32, 1u32, false)),
        (1, (4, 0, true)),
        (2, (0, 1, true)),
        (3, (3, 1, true)),
        (4, (1, 0, false)),
    ] {
        t.set_u32(cnt, 0x10 * k);
        stage_0395_common(&mut t, f, 1426, 0x13);
        for call in [0xbau32, 0x157] {
            let start = t.img.rel32_target(f + call);
            magic_r11::object_start_auto(&mut t, start);
        }
        // the deal: two kinds, four of each, `n` actors (the count global
        // the stage reads too)
        t.set_u32(t.img.word(deal + 0x17), 2);
        t.set_u32(t.img.word(deal + 0x1e), n);
        for i in 0..2u32 {
            t.set_u32(t.img.word(deal + 0x46) + 0x1c * i, 4);
        }
        t.set_u32(w(&t, 0x58), n);
        // the actors' characters and bytes
        let bytes = SCRATCH + 0x1e0;
        for i in 0..8u8 {
            t.set_u8(bytes + u32::from(i), 3 + 2 * i);
        }
        for (sel, tab) in [(0xcfu32, 0xdau32), (0x16c, 0x177)] {
            t.set_u32(w(&t, sel), bytes);
            for i in 0..0x20 {
                t.set_u32(w(&t, tab) + 4 * i, 0x0000_0080 + 3 * i);
            }
        }
        t.set_u32(w(&t, 0x1d1), bytes);
        for o in [0x10u32, 0x74] {
            t.set_u32(t.img.word(kc + o), bytes);
        }
        for o in [0x25u32, 0x8a, 0x145, 0x254, 0x450] {
            t.set_u32(w(&t, o), flag);
        }
        // the free ring (16 nodes) and the task list
        let (count, head) = (w(&t, 0x1e6), w(&t, 0x277));
        magic_r11::ring(&mut t, count, head, 16, SCRATCH + 0xb00);
        junk_ops(&mut t, f, &[0x46e, 0x478]);
        t.set_u32(w(&t, 0x2de), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(&t, 0x2f4), SCRATCH + 0x600);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    // both ways ran: the height (slot 12) only for a check answering 0
    assert!(t.u32(magic_owna::row(390)) >= 8, "the kinds were asked");
    assert!(t.u32(magic_owna::row(12)) >= 2, "a check answered 0");
}

/// `r21::lift_0301` in `magic_0301` 0x10001810 (`r26::LIFT_0301_1810`):
/// two records from slot 434 (their current entries' kinds 0 and 1), the
/// third record, slots 450 and 449 and the object's current entry; the
/// records' and the object's entry indices varied.
#[test]
fn lift_0301_1810() {
    let Some(mut t) = twin(301) else { return };
    let f = t.at(0x1810);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(444, 1), (450, 12), (449, 2)]);
    let (r1, r2, r3) = (SCRATCH + 0x1000, SCRATCH + 0x1400, SCRATCH + 0x1800);
    rec(&mut t, 434, 3, move |m, _| {
        match m.r32(magic_owna::row(434)) % 3 {
            1 => r1,
            2 => r2,
            _ => r3,
        }
    });
    rec(&mut t, 474, 7, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + (a[1] as f32);
            m.w32(a[3] + 4 * i, v.to_bits());
        }
        0x474
    });
    let a = SCRATCH + 0x800;
    t.set_u32(w(&t, 0x11), a);
    t.set_u32(a + 0x198, 0x0198_0001);
    let obj = SCRATCH + 0x1c00;
    for (k, (c1, c2, c3)) in [(0u32, 3u32, 1u32), (2, 0, 4)].into_iter().enumerate() {
        let k = k as u32;
        for (r, c) in [(r1, c1), (r2, c2), (obj, c3)] {
            t.write(r, &[0x3c + k as u8; 0x200]);
            t.set_u32(r + 0xac, c);
        }
        t.set_u32(magic_owna::row(434), 0);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `r26::boxes_0301` (`magic_0301` 0x10001000, cdecl `(obj)`): the two
/// bones' points (slot 476), the distance (slot 453), the two bone
/// matrices (slot 463, from the bone), `rand` shared by both sides; three
/// cases with other bones, indices and distances.
#[test]
fn boxes_0301() {
    let Some(mut t) = twin(301) else { return };
    let f = t.at(0x1000);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 476, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[1] as f32) * 0.25 + i as f32 * 1.5 - (a[0] & 0xf) as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x476
    });
    rec(&mut t, 453, 1, |_, a| 0x0453_0000 ^ a[0]);
    t.stub_st0(453, |m, a| {
        f64::from(f32::from_bits(m.r32(a[0]))) * 0.75 + 2.0
    });
    rec(&mut t, 463, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..16u32 {
            let v = (a[0] & 0xff) as f32 * 0.125 + i as f32 * 0.375 - 3.0;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x463
    });
    recs(&mut t, &[(450, 2), (449, 2)]);
    let a = SCRATCH + 0x800;
    t.set_u32(w(&t, 0x15), a);
    let obj = SCRATCH + 0x1c00;
    for (k, (idx, skel)) in [(0u32, 0x51u32), (2, 0x63), (1, 0x7f)]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        t.write(a, &[0x2a; 0x200]);
        t.set_u32(a + 0x198, skel);
        t.set_u32(a + 0x19c, idx);
        for i in 0..4u32 {
            t.set_u32(a + 0x1a0 + 4 * i, 0x0000_00a0 + 0x11 * i + k);
        }
        junk_ops(&mut t, f, &[0xf2, 0x336, 0x3c6, 0x3e3, 0x613, 0x699]);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// What `r26::start_up_0354` (`magic_0354` 0x100042b0) needs: the pools
/// (the path task, the task start and the records take from the same
/// ring), the part set-up's and the object starts' buffers, the actor, the
/// second point (slot 531), the target, the path task's slots, the task
/// list (empty or not).
fn start_up_0354_setup(t: &mut Twin, f: u32, k: u32, (battle, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1071);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 531, 2, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.75 + i as f32 * 2.5 + 7.0 * k as f32;
            m.w32(a[1] + 4 * i, v.to_bits());
        }
        0x531
    });
    // the path task's vector slots (as `magic_r24.rs`' 0419 test answers them)
    rec(t, 217, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            m.w32(
                a[1] + 4 * i,
                (0.25 * i as f32 + (a[0] & 7) as f32).to_bits(),
            );
        }
        0x217
    });
    filler(t, 234, 3, 4);
    filler(t, 233, 3, 4);
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4)]);
    for call in [0x1e7u32, 0x2f6] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    for o in [0x6cu32, 0x84] {
        t.set_u32(w(t, o), 0x0000_0040 + k);
    }
    for (sel, tab) in [(0x1ecu32, 0x205u32), (0x2fb, 0x314)] {
        target(t, f, (sel, tab), (2 + k) as u8);
    }
    let me = t.img.rel32_target(f + 0x390);
    t.set_u32(t.img.word(me + 0x05), battle);
    junk_ops(
        t,
        f,
        &[
            0x2b, 0x35, 0x3f, 0x128, 0x132, 0x1d8, 0x1de, 0x22b, 0x237, 0x297, 0x29d, 0x2a7, 0x2b1,
            0x2bb, 0x2c5, 0x2cf, 0x2d9, 0x2e3, 0x2ed, 0x32f, 0x33d, 0x34b, 0x369, 0x377, 0x387,
        ],
    );
    t.set_u32(w(t, 0x27e), 0x5a5a_5a5a);
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x3a3), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x3b7), SCRATCH + 0x600);
}

/// `magic_0354` 0x100042b0 (`r26::start_up_0354`, internal) and its load
/// step 0x10004180 (`r9_load::load_0354`): in and out of a battle, the
/// task list empty and not, near and far second points (the count word
/// at its floor of 1 and above), the yaw on both sides of its wrap.
#[test]
fn start_up_0354() {
    let Some(mut t) = twin(354) else { return };
    let f = t.at(0x42b0);
    for (k, battle, listed) in [
        (0u32, 0u32, false),
        (1, 1, true),
        (2, 0, true),
        (3, 1, false),
        (40, 0, true),
    ] {
        start_up_0354_setup(&mut t, f, k, (battle, listed));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0354);
    }
    let load = t.at(0x4180);
    load_cases(&mut t, load, &|t| start_up_0354_setup(t, f, 2, (1, true)));
}

/// `magic_0354` 0x100047a0 (`r26::sparks_0354`, a stored callback): the
/// pool with 6, 1 and 0 free records (both sparks, the first only, none);
/// slot 314 answering from its argument and count.
#[test]
fn sparks_0354() {
    let Some(mut t) = twin(354) else { return };
    let f = t.at(0x47a0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 262, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(&mut t, 314, 1, |m, a| m.r32(magic_owna::row(314)) ^ a[0]);
    t.stub_st0(314, |m, a| {
        f64::from(f32::from_bits(a[0])) * 0.375 + f64::from(m.r32(magic_owna::row(314)) % 7)
    });
    let take = t.img.rel32_target(f + 0x3b);
    let pw = |t: &Twin, o: u32| t.img.word(take + o);
    let (idx, n, table, page) = (pw(&t, 0x08), pw(&t, 0x0f), pw(&t, 0x37), pw(&t, 0x2b));
    let recs_at = SCRATCH + 0x8000;
    t.set_u32(table, recs_at);
    let parts = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x4b), parts);
    t.set_u32(w(&t, 0x24), 0x0000_0044);
    for (k, records) in [6u32, 1, 0].into_iter().enumerate() {
        let k = k as u32;
        for i in 0..4 {
            t.set_u32(parts + 4 * i, 0x0aa0_0000 + i + k);
        }
        for i in 0..6 {
            t.write(recs_at + 0x74 * i, &[0x5a; 0x74]);
            t.set_u32(recs_at + 0x74 * i + 0x6c, 0);
        }
        t.set_u32(n, records);
        t.set_u32(idx, 0);
        t.set_u32(page, 0);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// What `r26::start_up_0243` (`magic_0243` 0x10001340) needs: the pools
/// (the task start, the inlined takes and the pool reset share the ring),
/// the object starts, the target and the actors (`n`), the slots, the task
/// list (empty or not).
fn start_up_0243_setup(t: &mut Twin, f: u32, k: u32, (n, battle, listed): (u32, u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1232);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    recs(
        t,
        &[
            (413, 1),
            (215, 1),
            (711, 3),
            (710, 4),
            (596, 1),
            (597, 4),
            (598, 5),
        ],
    );
    for call in [0x6du32, 0xea] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    target(t, f, (0x72, 0x8b), (2 + k) as u8);
    // the actors: their bytes and characters
    let bytes = SCRATCH + 0x1f0;
    for i in 0..8u8 {
        t.set_u8(bytes + u32::from(i), 3 + 2 * i);
    }
    for o in [0xffu32, 0x15c] {
        t.set_u32(w(t, o), bytes);
    }
    for i in 0..0x20 {
        t.set_u32(w(t, 0x10a) + 4 * i, 0x0000_0090 + 5 * i);
    }
    t.set_u32(w(t, 0xb7), n);
    t.set_u32(w(t, 0x17e), 0x0000_0077 + k);
    t.set_u32(w(t, 0x243), battle);
    t.set_u32(w(t, 0x288), battle);
    t.set_u32(w(t, 0x23a), 0x0000_0050 + k);
    t.write(w(t, 0xc2) - 0xd4, &[0x5e; 0x130 * 4]);
    junk_ops(
        t,
        f,
        &[
            0x29, 0x33, 0x3d, 0xa3, 0xad, 0x249, 0x255, 0x25f, 0x269, 0x293, 0x299, 0x2a3, 0x2ad,
            0x4bd,
        ],
    );
    for o in [0x2b7u32, 0x2c0] {
        t.set_u32(w(t, o), 0x5a5a_5a5a);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x33c), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x350), SCRATCH + 0x600);
}

/// `magic_0243` 0x10001340 (`r26::start_up_0243`, internal) and its load
/// step 0x10001210 (`r9_load::load_0243`): 0 to 3 actors, in and out of a
/// battle, the task list empty and not.
#[test]
fn start_up_0243() {
    let Some(mut t) = twin(243) else { return };
    let f = t.at(0x1340);
    for (k, n, battle, listed) in [
        (0u32, 2u32, 0u32, false),
        (1, 3, 1, true),
        (2, 0, 0, true),
        (3, 1, 1, false),
    ] {
        start_up_0243_setup(&mut t, f, k, (n, battle, listed));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0243);
    }
    let load = t.at(0x1210);
    load_cases(&mut t, load, &|t| {
        start_up_0243_setup(t, f, 2, (2, 1, true));
    });
}

/// `magic_0243` 0x10001880 (`r26::sparks_0243`, a stored callback): the
/// count run out; the ring with every entry free, one taken in the second
/// part, the first part running out (5 entries) and the second finding
/// none; slot 237 writes a point from its words, 314/307 answer floats.
#[test]
fn sparks_0243() {
    let Some(mut t) = twin(243) else { return };
    let f = t.at(0x1880);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 237, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.5 + (a[2] & 0xf) as f32;
            m.w32(a[3] + 4 * i, v.to_bits());
        }
        0x237
    });
    for s in [314u32, 307] {
        rec(&mut t, s, 1, move |m, a| {
            m.r32(magic_owna::row(s)) ^ a.first().copied().unwrap_or(0)
        });
        t.stub_st0(s, move |m, a| {
            f64::from(f32::from_bits(a.first().copied().unwrap_or(0))) * 0.375
                + f64::from(m.r32(magic_owna::row(s)) % 7)
                + f64::from(s & 1)
        });
    }
    let rt = t.img.rel32_target(f + 0xa6);
    let rw = |t: &Twin, o: u32| t.img.word(rt + o);
    let (cur, n, base) = (rw(&t, 0x01), rw(&t, 0x09), rw(&t, 0x16));
    let ring = SCRATCH + 0x8000;
    t.set_u32(base, ring);
    let parts = SCRATCH + 0x700;
    t.set_u32(w(&t, 0xda), parts);
    for i in 0..4 {
        t.set_u32(parts + 4 * i, 0x0aa0_0000 + i);
    }
    for o in [0x3au32, 0x5e] {
        t.set_u32(w(&t, o), 0x0000_0044);
    }
    // (count, ring entries, busy mask)
    for (count, entries, busy) in [
        (0u32, 16u32, 0u32),
        (3, 16, 0),
        (1, 16, 0b10_0000_0000),
        (2, 5, 0),
        (1, 9, 0),
    ] {
        t.set_u32(w(&t, 0x11), count);
        t.set_u32(n, entries);
        t.set_u32(cur, 2);
        for i in 0..16u32 {
            let e = ring + 0x50 * i;
            t.write(e, &[0x5b; 0x50]);
            t.set_u32(e + 0x48, (busy >> i) & 1);
        }
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// What `r26::start_up_0163` (`magic_0163` 0x10001430) needs: the pools
/// (the static record node, the task start and the inlined takes share
/// the ring), the object starts, the target and its kind (slot 390), the
/// state check's list (slot 390's answer for it too), the task list.
fn start_up_0163_setup(t: &mut Twin, f: u32, k: u32, (kind, listed): (u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 2172);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    let yaw = [0.25f64, 3.0, 9.5, 2.0][k as usize % 4];
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    rec(t, 390, 1, move |_, _| kind);
    // an unlisted state's answer: 0 (the two extra objects)
    rec(t, 534, 1, |_, _| 0);
    recs(t, &[(711, 3), (710, 4), (271, 1)]);
    for call in [0x14cu32, 0x1e2, 0x2b5, 0x31a, 0x3b5] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    for (sel, tab) in [(0x48u32, 0x52u32), (0xca, 0xd6), (0xca, 0x11c)] {
        target(t, f, (sel, tab), (2 + k) as u8);
    }
    t.write(w(t, 0x37d) - 0xd0, &[0x5e; 0x130 * 3]);
    junk_ops(
        t,
        f,
        &[
            0x26, 0x30, 0x166, 0x178, 0x18a, 0x197, 0x1ba, 0x1cc, 0x1dd, 0x1ef, 0x1fc, 0x209,
            0x216, 0x22e, 0x238, 0x244, 0x24a, 0x254, 0x25e, 0x268, 0x272, 0x27c, 0x28c, 0x2ca,
            0x2d7, 0x2e4, 0x2f4, 0x315, 0x327, 0x334, 0x341, 0x34e, 0x366, 0x3f8, 0x416, 0x41c,
            0x422, 0x428, 0x42e, 0x456, 0x4b0, 0x56c, 0x5d0, 0x5da, 0x5e4, 0x6ce, 0x6e1, 0x6f5,
            0x7c1,
        ],
    );
    t.set_u32(w(t, 0x286), 0x5a5a_5a5a);
    t.write(w(t, 0x6b7), &[0x5c; 0x1c]);
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x533), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x54e), SCRATCH + 0x600);
}

/// `magic_0163` 0x10001430 (`r26::start_up_0163`, internal) and its load
/// step 0x10001300 (`r9_load::load_0163`, the 119-byte form): the target's
/// kind 0x108c or another, the state listed (slot 390's answer in the
/// DLL's list) or not, the task list empty and not, the yaw on both sides
/// of its wrap.
#[test]
fn start_up_0163() {
    let Some(mut t) = twin(163) else { return };
    let f = t.at(0x1430);
    // the state check's list: its last entry is "listed"
    let sl = t.img.rel32_target(f + 0x295);
    let mut l = t.img.word(sl + 0x20);
    while t.u32(l + 4) != u32::MAX {
        l += 4;
    }
    let listed_kind = t.u32(l);
    for (k, kind, listed) in [
        (0u32, 0x108cu32, false),
        (1, listed_kind, true),
        (2, 0x7777_0001, true),
        (3, 0x108c, false),
    ] {
        start_up_0163_setup(&mut t, f, k, (kind, listed));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0163);
    }
    let load = t.at(0x1300);
    load_cases_d(&mut t, load, 1, &|t| {
        start_up_0163_setup(t, f, 2, (0x108c, true));
    });
}

/// What `r26::start_up_0299` (`magic_0299` 0x100013e0) needs: the pools
/// (the task start and the inlined takes share the ring), the object
/// starts, the actor and the actors (`n`), the slots, the task list.
fn start_up_0299_setup(t: &mut Twin, f: u32, k: u32, (n, battle, listed): (u32, u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 2467);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - 3.0 * k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    let yaw = [0.25f64, 3.0, 9.5, 2.0][k as usize % 4];
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, _| yaw);
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.375 - 1.5);
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4), (271, 1)]);
    for call in [0x65u32, 0x303] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    t.set_u32(w(t, 0x70), 0x0000_0040 + k);
    t.set_u32(w(t, 0xb1), 0x0000_0040 + k);
    // the actors: their bytes and characters
    let bytes = SCRATCH + 0x1f0;
    for i in 0..8u8 {
        t.set_u8(bytes + u32::from(i), 3 + 2 * i);
    }
    for o in [0x1c6u32, 0x318, 0x362] {
        t.set_u32(w(t, o), bytes);
    }
    for tab in [0x1e3u32, 0x323] {
        for i in 0..0x20 {
            t.set_u32(w(t, tab) + 4 * i, 0x0000_0090 + 5 * i);
        }
    }
    t.set_u32(w(t, 0x1ba), n);
    t.set_u32(w(t, 0x867), battle);
    t.set_u32(w(t, 0x8ac), battle);
    t.set_u32(w(t, 0x85e), 0x0000_0050 + k);
    t.write(w(t, 0x1cb) - 0xd4, &[0x5e; 0x130 * 4]);
    t.write(w(t, 0x3a1), &[0x5f; 0x34 * 4]);
    junk_ops(
        t,
        f,
        &[
            0x2c, 0x36, 0xa7, 0xfe, 0x118, 0x122, 0x12e, 0x164, 0x16e, 0x178, 0x182, 0x18c, 0x196,
            0x1a6, 0x3fb, 0x44b, 0x45f, 0x518, 0x590, 0x59a, 0x658, 0x688, 0x69c, 0x6b0, 0x771,
            0x77b, 0x78f, 0x86d, 0x879, 0x883, 0x8b7, 0x8c7, 0x8d1,
        ],
    );
    for o in [0x1a0u32, 0x6c4, 0x7ad, 0x8db, 0x8e4] {
        t.set_u32(w(t, o), 0x5a5a_5a5a);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x4e2), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x4fc), SCRATCH + 0x600);
}

/// `magic_0299` 0x100013e0 (`r26::start_up_0299`, internal) and its load
/// step 0x100012b0 (`r9_load::load_0299`): 0 to 3 actors, in and out of a
/// battle, the task list empty and not, the yaw on both sides of its
/// wrap.
#[test]
fn start_up_0299() {
    let Some(mut t) = twin(299) else { return };
    let f = t.at(0x13e0);
    for (k, n, battle, listed) in [
        (0u32, 2u32, 0u32, false),
        (1, 3, 1, true),
        (2, 0, 0, true),
        (3, 1, 1, false),
    ] {
        start_up_0299_setup(&mut t, f, k, (n, battle, listed));
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0299);
    }
    let load = t.at(0x12b0);
    load_cases(&mut t, load, &|t| {
        start_up_0299_setup(t, f, 2, (2, 1, true));
    });
}

/// What `r26::start_up_0377` (`magic_0377` 0x100036d0) needs: the free
/// ring (16 nodes; the loop's takes, the two later ones and the records'
/// set-up share it), the object starts, the actor and the actors (`n`),
/// the slots, the task list.
fn start_up_0377_setup(t: &mut Twin, f: u32, k: u32, (n, battle, listed): (u32, u32, bool)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 1500);
    magic_r11::object_slots(t);
    let (count, head) = (w(t, 0x401), w(t, 0x3f4));
    magic_r11::ring(t, count, head, 16, SCRATCH + 0xb00);
    rec(t, 262, 4, move |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, a| {
        f64::from(a[0] & 0xf) * 0.75 - 2.0 + f64::from(k)
    });
    recs(t, &[(413, 1), (215, 1), (711, 3), (710, 4)]);
    for call in [0x69u32, 0x1bc] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    for o in [0x89u32, 0x96] {
        t.set_u32(w(t, o), 0x0000_0040 + k);
    }
    let bytes = SCRATCH + 0x1f0;
    for i in 0..8u8 {
        t.set_u8(bytes + u32::from(i), 3 + 2 * i);
    }
    for o in [0x1d1u32, 0x22b, 0x295] {
        t.set_u32(w(t, o), bytes);
    }
    for tab in [0x1dcu32, 0x236] {
        for i in 0..0x20 {
            t.set_u32(w(t, tab) + 4 * i, 0x0000_0090 + 5 * i);
        }
    }
    t.set_u32(w(t, 0x17f), n);
    t.set_u32(w(t, 0x496), battle);
    t.set_u32(w(t, 0x4dc), battle);
    t.set_u32(w(t, 0x49c), 0x0000_0050 + k);
    t.write(w(t, 0x18a) - 0xd4, &[0x5e; 0x150 * 4]);
    t.write(w(t, 0x2b1), &[0x5f; 0x34 * 4]);
    junk_ops(
        t,
        f,
        &[
            0x26, 0x30, 0x3a, 0x9c, 0xeb, 0x10a, 0x110, 0x11a, 0x142, 0x156, 0x170, 0x4a1, 0x4b3,
            0x4bd, 0x4e7, 0x4f7, 0x501,
        ],
    );
    for o in [0x16au32, 0x50b, 0x514] {
        t.set_u32(w(t, o), 0x5a5a_5a5a);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(w(t, 0x3a0), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(w(t, 0x3b8), SCRATCH + 0x600);
    t.set_u32(w(t, 0x3fa), SCRATCH + 0x600);
}

/// `magic_0377` 0x100036d0 (`r26::start_up_0377`, internal) and its load
/// step 0x100035a0 (`r9_load::load_0377`): 0 to 3 actors, in and out of a
/// battle, the task list empty and not; `rand` shared.
#[test]
fn start_up_0377() {
    let Some(mut t) = twin(377) else { return };
    let f = t.at(0x36d0);
    for (k, n, battle, listed) in [
        (0u32, 2u32, 0u32, false),
        (1, 3, 1, true),
        (2, 0, 0, true),
        (3, 1, 1, false),
        (0, 0, 1, false),
    ] {
        start_up_0377_setup(&mut t, f, k, (n, battle, listed));
        if n == 0 && !listed {
            // no actor and an empty list: the first later node starts it
            t.set_u32(t.img.word(f + 0x46c), 0);
        }
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r26::start_up_0377);
    }
    let load = t.at(0x35a0);
    load_cases(&mut t, load, &|t| {
        start_up_0377_setup(t, f, 2, (2, 1, true));
    });
}

/// `magic_0377`'s two spark functions: the burst 0x10003d20
/// (`r26::burst_0377`; the object's word `+0x118` past 0x19 or not) and
/// the object task 0x10003f00 (`r26::task_0377`: the delay running, the
/// count running out (the give-back), one spark); the pool with 6, 2 and
/// 0 free records; slot 314/404 answering from their argument and count.
#[test]
fn sparks_0377() {
    let Some(mut t) = twin(377) else { return };
    let (fb, ft) = (t.at(0x3d20), t.at(0x3f00));
    for s in [314u32, 404] {
        rec(&mut t, s, 1, move |m, a| m.r32(magic_owna::row(s)) ^ a[0]);
        t.stub_st0(s, move |m, a| {
            f64::from(f32::from_bits(a[0])) * 0.375 + f64::from(m.r32(magic_owna::row(s)) % 7)
        });
    }
    let take = t.img.rel32_target(fb + 0x21);
    let pw = |t: &Twin, o: u32| t.img.word(take + o);
    let (idx, n, table, page) = (pw(&t, 0x08), pw(&t, 0x0f), pw(&t, 0x37), pw(&t, 0x2b));
    let recs_at = SCRATCH + 0x8000;
    t.set_u32(table, recs_at);
    let parts = SCRATCH + 0x700;
    t.set_u32(t.img.word(fb + 0x31), parts);
    let obj = SCRATCH + 0x1000;
    let task = SCRATCH + 0x1400;
    let back = t.img.rel32_target(ft + 0x30);
    for (k, (records, word, delay, count)) in [
        (6u32, 0x20i32, 0i32, 3i32),
        (2, 0x10, 2, 1),
        (0, 0x30, 0, 2),
        (6, 0x19, 0, 5),
        (6, 0x40, 0, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(parts, 0x0aa0_0000 + k);
        for i in 0..6 {
            t.write(recs_at + 0x74 * i, &[0x5a; 0x74]);
            t.set_u32(recs_at + 0x74 * i + 0x6c, 0);
        }
        t.set_u32(n, records);
        t.set_u32(idx, 0);
        t.set_u32(page, 0);
        t.write(obj, &[0x3d; 0x160]);
        for i in 0..4u32 {
            t.set_u32(
                obj + 0xd0 + 4 * i,
                (1.5 * i as f32 - 2.0 + k as f32).to_bits(),
            );
            t.set_u32(
                obj + 0x130 + 4 * i,
                (0.5 * i as f32 + 1.0 - k as f32).to_bits(),
            );
        }
        t.set_u32(obj + 0x118, word.cast_unsigned());
        t.set_u32(obj + 0x140, delay.cast_unsigned());
        t.set_u32(obj + 0x144, count.cast_unsigned());
        magic_h6::scrub(&mut t);
        t.check_entry(fb, Call::default(), &[obj], Eax::Checked);
        // the task: alone on its child list, linked between two others,
        // the give-back's free ring of 3
        t.write(task, &[0x66; 0x20]);
        let (prev, next) = (SCRATCH + 0x1440, SCRATCH + 0x1480);
        t.write(prev, &[0x67; 0x20]);
        t.write(next, &[0x68; 0x20]);
        t.set_u32(task, prev);
        t.set_u32(task + 4, next);
        t.set_u32(task + 8, task);
        t.set_u32(task + 0xc, 0);
        t.set_u32(task + 0x14, obj);
        let (c, h) = (t.img.word(back + 0x8e), t.img.word(back + 0x74));
        magic_r11::ring(&mut t, c, h, 3, SCRATCH + 0x1500);
        for i in 0..6 {
            t.write(recs_at + 0x74 * i, &[0x5b; 0x74]);
            t.set_u32(recs_at + 0x74 * i + 0x6c, 0);
        }
        t.set_u32(idx, 0);
        magic_h6::scrub(&mut t);
        t.check_entry(ft, Call::default(), &[task], Eax::Checked);
    }
}

/// The slots `magic_0375`'s sprites need: 324 a counter's hash, 403 a
/// float from its word, 941/942 tables, 772 a handle, 773..779 logged,
/// 229/248 vectors; the sprite-frame callee's lock and constant.
fn sprites_0375_stubs(t: &mut Twin) -> u32 {
    let cnt = SCRATCH + 0x7a0;
    rec(t, 324, 0, move |m: &mut dyn StubMem, _: &[u32]| {
        let k = m.r32(cnt).wrapping_add(1);
        m.w32(cnt, k);
        k.wrapping_mul(0x9e37_79b9) >> 9
    });
    rec(t, 403, 1, |_, a| 0x0403_0000 ^ a[0]);
    t.stub_st0(403, |_, a| f64::from(f32::from_bits(a[0])) * 0.25 + 1.0);
    let tbl = SCRATCH + 0x3200;
    t.write(tbl, &[0x31; 0x100]);
    t.set_u16(tbl + 4, 3);
    for i in 0..3u32 {
        t.set_u16(tbl + 8 * i + 0x10, (0x80 + 8 * i) as u16);
        t.set_u16(tbl + 8 * i + 0x12, (0x100 * (i + 1)) as u16);
    }
    for s in [941u32, 942] {
        rec(t, s, 1, move |_, _| tbl);
    }
    rec(t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    for (s, n) in [
        (773u32, 4usize),
        (774, 5),
        (775, 2),
        (776, 1),
        (777, 1),
        (778, 6),
        (779, 2),
        (325, 2),
    ] {
        rec(t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    rec(t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..3 {
            m.w32(a[0] + 4 * i, (0.5 * i as f32 + 1.0).to_bits());
        }
        0x229
    });
    cnt
}

/// A sprite pool of `n` sprites (0x94 bytes) at `sprites`, `busy` of them
/// live (and held, so the step keeps them).
fn sprite_pool(t: &mut Twin, pool: u32, sprites: u32, (n, busy): (u32, u32)) {
    t.write(sprites, &[0x5a; 0x94 * 4]);
    for i in 0..4u32 {
        let p = sprites + 0x94 * i;
        t.set_u32(p, u32::from(i < busy));
        t.set_u32(p + 0x2c, 1);
        t.set_u32(p + 0x28, 1 + i);
        t.set_u32(p + 0x30, 0x0300_0000 + i);
    }
    t.write(pool, &[0u8; 0x20]);
    t.set_u32(pool, n);
    t.set_u32(pool + 4, sprites);
}

/// `r26::spark_0375` at `magic_0375` 0x10001180 and 0x10001390 (`ecx` =
/// the pool, the stack `(P)`): a free sprite and none.
#[test]
fn sparks_0375() {
    let Some(mut t) = twin(375) else { return };
    let cnt = sprites_0375_stubs(&mut t);
    let (pool, sprites, p) = (SCRATCH + 0x300, SCRATCH + 0x1000, SCRATCH + 0x200);
    for (k, rva) in [0x1180u32, 0x1390].into_iter().enumerate() {
        let f = t.at(rva);
        for (n, busy) in [(3u32, 1u32), (2, 2)] {
            t.set_u32(cnt, 0x10 * k as u32 + n);
            sprite_pool(&mut t, pool, sprites, (n, busy));
            for i in 0..4u32 {
                t.set_u32(p + 4 * i, (1.5 * i as f32 - 2.0 + n as f32).to_bits());
            }
            magic_h6::scrub(&mut t);
            t.check(f, Call::this(pool), &[p], Eax::Checked, |g, img, fr| {
                Ret::eax(r26::spark_0375(g, img, fr.at, fr.esp, fr.ecx))
            });
        }
    }
}

/// `magic_0375` 0x100016c0 (`r26::sparks_step_0375`, cdecl `(pool)`): the
/// sprite count below and past 0x50, slot 261 answering or not, the
/// second count below and past 0x19; the pool full of live sprites (the
/// frame words, the step and the draw) or empty.
#[test]
fn sparks_step_0375() {
    let Some(mut t) = twin(375) else { return };
    let f = t.at(0x16c0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let cnt = sprites_0375_stubs(&mut t);
    rec(&mut t, 237, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4u32 {
            m.w32(a[3] + 4 * i, (0.75 * i as f32 - 1.0).to_bits());
        }
        0x237
    });
    let answer = SCRATCH + 0x7a8;
    rec(&mut t, 261, 1, move |m: &mut dyn StubMem, a: &[u32]| {
        m.r32(answer) & a[0]
    });
    // the step's lock, the draw's `x` and its frame's lock and constant
    let step = t.img.rel32_target(f + 0xc5);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(t.img.word(step + 0x58), lock);
    let draw = t.img.rel32_target(f + 0xd5);
    t.set_u32(t.img.word(draw + 0x11), 0x00ab_0001);
    let sf = t.img.rel32_target(draw + 0x79);
    t.set_u32(t.img.word(sf + 0x290), lock);
    t.set_u32(t.img.word(sf + 0x179), 2.5f32.to_bits());
    let (pool, sprites) = (SCRATCH + 0x300, SCRATCH + 0x1000);
    for (k, (n1, ans, n2, live)) in [
        (0u32, (0x10u32, 3u32, 0x10u32, 0u32)),
        (1, (0x60, 0, 0x10, 3)),
        (2, (0x10, 3, 0x30, 2)),
        (3, (0x60, 1, 0x30, 3)),
    ] {
        t.set_u32(cnt, 0x10 * k);
        t.set_u32(answer, ans);
        t.set_u32(w(&t, 0x3a), n1);
        t.set_u32(w(&t, 0x70), n2);
        sprite_pool(&mut t, pool, sprites, (live, live));
        for i in 0..4u32 {
            let p = sprites + 0x94 * i;
            t.set_u32(p + 0x30 + 0xc, SCRATCH + 0x3280);
            t.set_u16(p + 0x30 + 8, (i % 3) as u16);
            t.set_u16(p + 0x30 + 0x10, 0x200);
        }
        junk_ops(&mut t, f, &[0xda]);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[pool], Eax::Checked);
    }
}

/// `magic_0375` 0x10001fc0 (`r26::setup_0375`, cdecl `(a)`): the block
/// handed out, the slots logged, the five blocks junked first.
#[test]
fn setup_0375() {
    let Some(mut t) = twin(375) else { return };
    let f = t.at(0x1fc0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let block = SCRATCH + 0x1_0000;
    rec(&mut t, 803, 2, move |_, _| block);
    let c = SCRATCH + 0x900;
    t.write(c, &[0x2c; 0x40]);
    rec(&mut t, 719, 3, move |_, _| c);
    recs(
        &mut t,
        &[(718, 1), (222, 4), (711, 3), (710, 4), (941, 1), (260, 1)],
    );
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x9c), lock);
    t.set_u32(lock, 0x55);
    for (o, n) in [
        (0x13du32, 0x30usize),
        (0x15e, 0x110),
        (0x16f, 0x110),
        (0x1bc, 0x110),
        (0x1ff, 0x110),
    ] {
        t.write(w(&t, o), &vec![0x5c; n]);
    }
    for k in 0..2u32 {
        junk_ops(
            &mut t,
            f,
            &[
                0x19, 0x23, 0x2d, 0x37, 0x3c, 0x44, 0x5a, 0xa5, 0x105, 0x11f, 0x142, 0x2c8,
            ],
        );
        t.check_entry(f, Call::default(), &[0x0a00 + k], Eax::Checked);
    }
}
