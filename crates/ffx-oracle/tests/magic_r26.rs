//! Round 26's own ports (`ffx_magic::r26`) against each original (the
//! copies bound to earlier ports are tested with those ports). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
mod magic_owna;
mod magic_r11;

use ffx_magic::r26;
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
        junk_ops(&mut t, f, &[0x85, 0x8f, 0xcb, 0xe7, 0x196]);
        t.write(w(&t, 0xda), &[0x5c; 0xc]);
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
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
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
        t.check_entry(f, Call::default(), &[p], Eax::Checked);
    }
}
