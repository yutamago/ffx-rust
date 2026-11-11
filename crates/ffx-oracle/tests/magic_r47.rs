//! Round 47's ports (`ffx_magic::r47`) against each original. Needs
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

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use ownb::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// The words the operands `ops` of the code at `f` name, junked (a
/// store of 0 or of an unchanged value must show).
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32], k: u32) {
    for (i, &o) in (0u32..).zip(ops) {
        let at = t.img.word(f + o);
        t.set_u32(at, 0x5a17_3c01 ^ (i << 8) ^ k);
    }
}

/// Every slot answering `first word ^ (slot << 20)`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// The globals `r47::start_up_0417` stores (their operands).
const STORES_0417: &[u32] = &[
    0x2d, 0x37, 0x41, 0x4f, 0xc1, 0xcd, 0x11e, 0x140, 0x146, 0x150, 0x15a, 0x164, 0x16e, 0x178,
    0x182, 0x18c, 0x196, 0x1a0, 0x1aa, 0x289, 0x29d, 0x2a6, 0x447, 0x451, 0x45b, 0x465, 0x46f,
    0x479, 0x483, 0x48d, 0x497, 0x4a1, 0x4ab, 0x4b5, 0x56e, 0x578, 0x582, 0x58c, 0x596, 0x5a0,
    0x5aa, 0x5b4, 0x5be, 0x5c8, 0x5d1, 0x5d7, 0x5e1, 0x5eb, 0x698, 0x6ae, 0x6ba, 0x6c4, 0x6ce,
    0x6d8, 0x702, 0x708, 0x712, 0x71c, 0x7e5, 0x89a,
];

/// What `r47::start_up_0417` (`magic_0417` 0x10003290) needs: the part
/// buffers' lock, the object starts, the free ring of the inlined takes
/// and the sprite pool (`free` nodes), the targets (`n`, their bytes and
/// handles), the task list (empty or not), the camera flags; slot 262
/// writes a point from its first word, 268 a vector from its first two,
/// 333 a float by the case (both wraps), 215/413 answer their word.
fn start_up_0417_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (n, free, listed, cam): (u32, u16, bool, u32),
) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    junk_ops(t, f, STORES_0417, k);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m, a| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        for i in 0..4u32 {
            let v = (a[0] & 0xff) as f32 * -0.25 + i as f32 * 0.75 + a[1] as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |_, a| {
        [-9.0, 0.5, 9.0][(k % 3) as usize] + f64::from(a[0] & 0xf) * 0.125
    });
    for s in [215u32, 413] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    rec(t, 711, 3, |_, a| 0x0711_0000 ^ a[0]);
    rec(t, 710, 4, |_, a| 0x0710_0000 ^ a[0]);
    // the part buffers' lock word
    let pb = t.img.rel32_target(f + 0x54);
    let lock = SCRATCH + 0x4000;
    t.set_u32(lock, 0x77 + k);
    for o in [0x01u32, 0x31] {
        let at = t.img.word(pb + o);
        t.set_u32(at, lock);
    }
    for call in [0x73u32, 0x2e6] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    // the free ring (the takes and the pool's)
    magic_r11::ring(t, w(t, 0x441), w(t, 0x4d3), free, SCRATCH + 0xd800);
    // the targets: bytes and handles
    let bytes = SCRATCH + 0x1e0;
    for i in 0..8u8 {
        t.set_u8(bytes + u32::from(i), 2 + 3 * i);
    }
    for o in [0x1efu32, 0x2fb, 0x357] {
        let at = w(t, o);
        t.set_u32(at, bytes);
    }
    for o in [0x1fau32, 0x306] {
        let tab = w(t, o);
        for i in 0..0x20 {
            t.set_u32(tab + 4 * i, 0x0000_0080 + 7 * i + k);
        }
    }
    let at = w(t, 0x1bc);
    t.set_u32(at, n);
    for o in [0x7eu32, 0xc7, 0x138] {
        let at = w(t, o);
        t.set_u32(at, 0x0000_00a0 + k);
    }
    // the objects' vectors and records
    let at = w(t, 0x2ba);
    t.write(at - 0xd4, &[0x5e; 0x130 * 4]);
    let at = w(t, 0x3e5);
    t.write(at, &[0x5f; 0x34 * 4]);
    // the camera flags
    for (o, v) in [(0x6a8u32, cam & 1), (0x6f7, cam & 2)] {
        let at = w(t, o);
        t.set_u32(at, v);
    }
    for o in [0x726u32, 0x72f] {
        let at = w(t, o);
        t.set_u32(at, 0x5a5a_5a5a);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    for o in [0x52eu32, 0x7b1, 0x862] {
        let at = w(t, o);
        t.set_u32(at, if listed { SCRATCH + 0x600 } else { 0 });
    }
    let at = w(t, 0x549);
    t.set_u32(at, SCRATCH + 0x600);
}

/// `magic_0417` 0x10003290 (`r47::start_up_0417`, internal) and its load
/// step 0x100030e0 (`r47::load_0417`): targets 0..3 (the farthest first,
/// last, in the middle), the ring with 5 and 7 free nodes, the task list
/// empty and not, the camera flags both ways, the summon's yaw wrapped
/// both ways and not; the load step's waits.
#[test]
fn start_up_0417() {
    let Some(mut t) = twin(417) else { return };
    all_stubs(&mut t);
    let f = t.at(0x3290);
    for (k, case) in (0u32..).zip([
        (3u32, 5u16, false, 0u32),
        (2, 7, true, 1),
        (0, 5, true, 2),
        (1, 6, false, 3),
    ]) {
        start_up_0417_setup(&mut t, f, k, case);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r47::start_up_0417(g, img, fr)
        });
    }
    let load = t.at(0x30e0);
    for (busy, asked, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0417_setup(&mut t, f, 1, (2, 6, true, 1));
        rec(&mut t, 225, 1, move |_, _| busy);
        rec(&mut t, 226, 2, |_, a| 0x0226_0000 ^ a[0]);
        rec(&mut t, 227, 1, move |_, _| ready);
        let op = |t: &Twin, o: u32| t.img.word(load + o);
        let at = op(&t, 0x18);
        t.set_u32(at, asked);
        for o in [0x26u32, 0x46] {
            let at = op(&t, o);
            t.set_u32(at, 0x0b0b_0000 + o);
        }
        for o in [0x61u32, 0x6b] {
            let at = op(&t, o);
            t.set_u32(at, 0x5a5a_0000 ^ o);
        }
        magic_h6::scrub(&mut t);
        t.check_entry(load, Call::default(), &[], Eax::Checked);
    }
}

/// What `r47::flames_0417` (`magic_0417` 0x10003cc0) needs: its waits
/// and counts, the summon's handle, the count word, the handle words
/// `G_H`, the sprite records (`n`, `busy` of them taken), the float
/// slots (314, 404, 333 in ST0; 314 cycles so both wraps of `P` and `Q`
/// come) and the matrix slots as data flow.
fn flames_0417_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (wait, count, fade): (i32, i32, i32),
    (n, busy): (u32, u32),
) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    junk_ops(t, f, &[0x3c, 0x4b], k);
    for (o, v) in [
        (0x11u32, wait.cast_unsigned()),
        (0x2e, count.cast_unsigned()),
        (0x41, fade.cast_unsigned()),
    ] {
        let at = w(t, o);
        t.set_u32(at, v);
    }
    for o in [0x93u32, 0xa5] {
        let at = w(t, o);
        t.set_u32(at, 0x00a0_0000 + k);
    }
    let at = w(t, 0x20e);
    t.set_u32(at, 5 + 3 * k);
    // the handle words
    let h = SCRATCH + 0x5f0;
    for i in 0..3 {
        t.set_u32(h + 4 * i, 0x0070_0000 + 0x10 * i + k);
    }
    for o in [0x3c3u32, 0x53b] {
        let at = w(t, o);
        t.set_u32(at, h);
    }
    // the records: `n` of 0x7c bytes, `busy` taken
    let take = t.img.rel32_target(f + 0x223);
    let buf = SCRATCH + 0x1_8000;
    for i in 0..n.max(1) {
        t.write(buf + 0x7c * i, &[0x3c; 0x7c]);
        t.set_u32(buf + 0x7c * i + 0x78, u32::from(i < busy));
    }
    for (o, v) in [(0x02u32, k % 2), (0x0a, n), (0x17, buf)] {
        let at = t.img.word(take + o);
        t.set_u32(at, v);
    }
    rec(t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 1.5 - 0.25);
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.5 - 1.0);
    // 314 counts its calls: the answers cycle through -9, 0.5 and 9
    t.set_u32(SCRATCH + 0x7e70, k);
    rec(t, 314, 1, |m, a| {
        let c = m.r32(SCRATCH + 0x7e70) + 1;
        m.w32(SCRATCH + 0x7e70, c);
        (314 << 16) ^ a[0]
    });
    t.stub_st0(314, |m, a| {
        let c = m.r32(SCRATCH + 0x7e70);
        [-9.0, 0.5, 9.0][(c % 3) as usize] + f64::from(f32::from_bits(a[0])) * 0.75
    });
    rec(t, 237, 4, move |m, a| {
        for i in 0..3 {
            wf(
                m,
                a[3] + 4 * i,
                (a[0] & 0xff) as f32 * 0.25 + i as f32 - k as f32,
            );
        }
        0x237
    });
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [328u32, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let raw = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x7e60 + 4 * i, raw);
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ (a[1] << 8));
}

/// `magic_0417` 0x10003cc0 (`r47::flames_0417`, internal): the wait
/// running, the count at 0 and below, the fade running and done, a burst
/// with the records full, running out after the first, second and fourth
/// record and with a record left; then its frame step 0x10003b70
/// (`r47::frame_0417`): the motion and the objects running or not, the
/// flag, the wait, 0..2 sounds.
#[test]
fn flames_0417() {
    let Some(mut t) = twin(417) else { return };
    all_stubs(&mut t);
    let f = t.at(0x3cc0);
    // ((wait, count, fade), (records, busy))
    let cases: [((i32, i32, i32), (u32, u32)); 10] = [
        ((3, 1, 0), (8, 0)),
        ((0, 0, 4), (8, 0)),
        ((-2, -3, 4), (8, 0)),
        ((0, 2, 0), (8, 0)),
        ((0, 2, 30), (8, 0)),
        ((-1, 1, -2), (1, 0)),
        ((0, 1, 7), (3, 1)),
        ((0, 1, 1), (5, 1)),
        ((0, 3, 12), (6, 1)),
        ((0, 1, 59), (9, 0)),
    ];
    for (k, (waits, pool)) in (0u32..).zip(cases) {
        flames_0417_setup(&mut t, f, k, waits, pool);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r47::flames_0417(g, img, fr)
        });
    }
    let step = t.at(0x3b70);
    let ro = t.img.rel32_target(step + 0xe);
    let ms = t.img.rel32_target(step + 0x15);
    rec(&mut t, 772, 2, |m, a| {
        let k = m.r32(SCRATCH + 0x7f74).wrapping_add(1);
        m.w32(SCRATCH + 0x7f74, k);
        a[0] ^ (a[1] << 12) ^ k
    });
    t.stub_edx(772, |m, a| m.r32(SCRATCH + 0x7f74) ^ a[1]);
    // 779 logs the lock word it sees
    let lock = SCRATCH + 0x7e10;
    rec(&mut t, 779, 2, move |m, a| {
        let k = m.r32(SCRATCH + 0x7e00);
        m.w32(SCRATCH + 0x7e00, k + 1);
        m.w32(SCRATCH + 0x7e40 + 4 * (k % 8), m.r32(lock));
        0x0779_0000 ^ a[0] ^ a[1]
    });
    // (motion done, objects running, the flag, the wait, sounds)
    for (k, (done, running, flag, wait, n)) in (0u32..).zip([
        (false, false, 0u32, 0i32, 2u32),
        (true, true, 0, 0, 2),
        (true, false, 1, 0, 2),
        (true, false, 0, 3, 2),
        (true, false, 0, 0, 0),
        (true, false, 0, -1, 1),
        (true, false, 0, 0, 2),
    ]) {
        flames_0417_setup(&mut t, f, k, (1 + k as i32, 1, 0), (8, 0));
        magic_hi::objects(&mut t, ro, running);
        magic_hi::motion(&mut t, ms, done);
        let w = |t: &Twin, o: u32| t.img.word(step + o);
        for (o, v) in [
            (0x2cu32, flag),
            (0x39, wait.cast_unsigned()),
            (0x5c, n),
            (0x9a, lock),
        ] {
            let at = w(&t, o);
            t.set_u32(at, v);
        }
        t.set_u32(lock, 0x40 + k);
        let task = SCRATCH + 0x7c00;
        let rec_at = SCRATCH + 0x7c40;
        t.write(task, &[0x31; 0x20]);
        t.write(rec_at, &[0x32; 0x20]);
        t.set_u32(task + 8, rec_at);
        magic_h6::scrub(&mut t);
        t.check_entry(step, Call::default(), &[task], Eax::Checked);
    }
}

/// One of `r47::draw_0417`'s records at `r` (0x7c bytes of distinct
/// floats, then the case's words): `on` `+0x78`, the wait `+0x60`, the
/// fade `+0x44`, the blend `n` `+0x64` and `t` `+0x68`, the size steps
/// `+0x6c`/`+0x70`, the ceiling `+0x5c` (above or below `y`), `+0x74`
/// and `+4`; the frame cursor (`+0`, playing, frame, time).
#[allow(clippy::too_many_arguments)]
fn draw_record(
    t: &mut Twin,
    r: u32,
    k: u32,
    on: bool,
    wait: i32,
    fade: f32,
    (n, tt): (i32, i32),
    (steps, at): (i32, i32),
    (above, last, idle): (bool, bool, bool),
) {
    for i in 0..0x1f {
        let x = 0.375 * i as f32 - 3.0 + k as f32 * 0.0625;
        t.set_u32(r + 4 * i, x.to_bits());
    }
    t.set_u32(r, 0x10 + k);
    t.set_u32(r + 4, u32::from(!idle));
    t.set_u32(r + 8, 0x0001_0000 + k);
    t.set_u32(r + 0x3c, 0x8040_20f0 ^ (k << 3));
    t.set_u32(r + 0x40, 0x10e0_c060 ^ k);
    t.set_u32(r + 0x60, wait.cast_unsigned());
    t.set_u32(r + 0x44, fade.to_bits());
    t.set_u32(r + 0x64, n.cast_unsigned());
    t.set_u32(r + 0x68, tt.cast_unsigned());
    t.set_u32(r + 0x6c, steps.cast_unsigned());
    t.set_u32(r + 0x70, at.cast_unsigned());
    let x = |i: u32| 0.375 * i as f32 - 3.0 + k as f32 * 0.0625;
    let y = x(4) + (x(8) + x(12));
    let top = if above { y - 1.5 } else { y + 2.5 };
    t.set_u32(r + 0x5c, top.to_bits());
    t.set_u32(r + 0x74, u32::from(last));
    t.set_u32(r + 0x78, u32::from(on));
}

/// `magic_0417` 0x100044a0 (`r47::draw_0417`, a stored callback): no
/// records, then records off, waiting, fading, blending (`t` negative,
/// running past `n`), done blending, past the ceiling, the size steps
/// running and ending, the last frame with `+4` 0 and not.
#[test]
fn draw_0417() {
    let Some(mut t) = twin(417) else { return };
    all_stubs(&mut t);
    let f = t.at(0x44a0);
    magic_r11::object_slots(&mut t);
    for s in [723u32, 724, 235] {
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, s as f32 * 0.001 + i as f32 * 0.5 - 1.0);
            }
            s
        });
    }
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |m, a| {
        let c = m.r32(SCRATCH + 0x7e70) + 1;
        m.w32(SCRATCH + 0x7e70, c);
        f64::from(f32::from_bits(a[0])) * 1.5 + f64::from(c) * 0.25
    });
    rec(&mut t, 330, 3, |m, a| {
        let ang = f32::from_bits(a[2]);
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + ang + i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        330
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x230
    });
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i) * 0.25;
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    t.stub_edx(772, |_, a| a[1] ^ 0x0772_0000);
    // 773 answers known / not known in turn
    rec(&mut t, 773, 4, |m, a| {
        let c = m.r32(SCRATCH + 0x7e74) + 1;
        m.w32(SCRATCH + 0x7e74, c);
        (c & 1) | (a[3] & 0x100)
    });
    let link = t.img.rel32_target(f + 0x3f0);
    let lock = SCRATCH + 0x4100;
    for o in [0x4fu32, 0x7a, 0xa9, 0xc6, 0x115, 0x13b] {
        let at = t.img.word(link + o);
        t.set_u32(at, lock);
    }
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let buf = SCRATCH + 0x1_8000;
    for k in 0..3u32 {
        junk_ops(&mut t, f, &[0x6d, 0x76, 0x148, 0x1ff, 0x427], k);
        t.set_u32(lock, 0x33 + k);
        let at = w(&t, 0x1f);
        t.set_u32(at, buf);
        let recs: [(bool, i32, f32, (i32, i32), (i32, i32), (bool, bool, bool)); 8] = [
            (false, 0, 0.5, (2, 1), (3, 1), (false, false, false)),
            (true, 3, 0.5, (2, 1), (3, 1), (false, false, false)),
            (true, 0, 0.75, (2, 1), (3, 1), (false, false, false)),
            (
                true,
                -1,
                0.0,
                (5, -2 - k as i32),
                (3, 3),
                (true, false, false),
            ),
            (true, 0, -0.5, (3, 3), (4, 1), (false, true, true)),
            (true, 0, -0.5, (0, 1), (0, 0), (true, true, false)),
            (true, 0, -0.5, (-1, 1), (-2, 0), (false, true, true)),
            (true, 1, 0.5, (2, 1), (3, 1), (false, false, false)),
        ];
        for (i, &(on, wait, fade, blend, steps, flags)) in (0u32..).zip(recs.iter()) {
            draw_record(
                &mut t,
                buf + 0x7c * i,
                k + i,
                on,
                wait,
                fade,
                blend,
                steps,
                flags,
            );
        }
        let n = [0u32, 8, 7][k as usize];
        let at = w(&t, 0x94);
        t.set_u32(at, n);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
