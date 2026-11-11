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

use magic::{Call, Eax, SCRATCH, StubMem, TRAPS, Twin};
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

/// The 16 words at `src` appended to the log at `SCRATCH + 0x2_0000`
/// (32 entries, then over again): what a stub saw in a matrix that only
/// lives in the caller's frame.
fn log16(m: &mut dyn StubMem, src: u32) {
    let k = m.r32(SCRATCH + 0x1_fff0);
    m.w32(SCRATCH + 0x1_fff0, k + 1);
    let to = SCRATCH + 0x2_0000 + 0x40 * (k % 32);
    for i in 0..16 {
        let x = m.r32(src + 4 * i);
        m.w32(to + 4 * i, x);
    }
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

/// `n` floats at `at`: `x0 + 0.25 i` (no NaNs).
fn floats(t: &mut Twin, at: u32, n: u32, x0: f32) {
    for i in 0..n {
        t.set_u32(at + 4 * i, (x0 + 0.25 * i as f32).to_bits());
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
    let cases: [((i32, i32, i32), (u32, u32)); 12] = [
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
        ((0, 1, 2), (2, 1)),
        ((0, 1, 3), (2, 2)),
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
        log16(m, a[1]);
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
        log16(m, a[2]);
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
    // (the counts end the walk after each kind of record: the colour
    // global keeps the last one's)
    for (k, n) in (0u32..).zip([0u32, 10, 7, 3, 4, 9]) {
        junk_ops(&mut t, f, &[0x6d, 0x76, 0x148, 0x1ff, 0x427], k);
        t.set_u32(lock, 0x33 + k);
        let at = w(&t, 0x1f);
        t.set_u32(at, buf);
        let recs: [(bool, i32, f32, (i32, i32), (i32, i32), (bool, bool, bool)); 10] = [
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
            (true, 0, -0.5, (4, 3), (2, 1), (false, false, true)),
            (true, 0, -0.5, (0, 0), (0, 0), (false, false, true)),
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
        let at = w(&t, 0x94);
        t.set_u32(at, n);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0434` 0x100012c0 (`r47::load_0434`) running its start-up
/// 0x10009900 (`r47::start_up_0434`): each wait of the load step (225,
/// 5, 28, 227, the request once), the buffer block given or allocated
/// (slot 803), the history 0..5 bytes, the free ring with 3 and 4 nodes,
/// the task list empty and not, slot 520's answer both ways.
#[test]
fn start_up_0434() {
    let Some(mut t) = twin(434) else { return };
    all_stubs(&mut t);
    let f = t.at(0x12c0);
    let su = t.at(0x9900);
    let g = |t: &Twin, op: u32| t.img.word(f + op);
    let gs = |t: &Twin, op: u32| t.img.word(su + op);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 5, 1, |m, _| m.r32(SCRATCH + 0x14));
    rec(&mut t, 28, 2, |m, _| m.r32(SCRATCH + 0x18));
    rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    rec(&mut t, 520, 0, |m, _| m.r32(SCRATCH + 0x20));
    // the block: slot 803 hands out the same place the global may hold
    let b = 0x6000_0000u32;
    rec(&mut t, 803, 2, move |_, _| b);
    // the caster placement's slots (as `magic_r22.rs`)
    rec(&mut t, 262, 4, |m, a| {
        for (i, v) in [1.5f32, -2.25, 0.75].into_iter().enumerate() {
            m.w32(a[1 + i], (v + (a[0] & 0xf) as f32).to_bits());
        }
        0x0262
    });
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.125 - 1.0);
    for (s, x) in [(277u32, 0.5f64), (15, 1.25), (16, -0.75), (17, 2.5)] {
        rec(&mut t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.0625 + x);
    }
    rec(&mut t, 293, 1, |_, _| 0);
    let list = SCRATCH + 0x760;
    t.write(list, &[5, 0, 9, 0, 0x22, 0, 0, 0]);
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], 3);
        list
    });
    let lock = SCRATCH + 0x7f0;
    t.set_u32(gs(&t, 0xc2), lock);
    // slot 711 sees the game lock held
    rec(&mut t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x7f8, l);
        0x0711_0000 ^ a[0] ^ (l << 8)
    });
    magic_r11::pools(&mut t, su, 995);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    // (225, 5, 28, once, 227, history n, free nodes, listed, 520, block)
    #[allow(clippy::type_complexity)]
    let cases: [(u32, u32, u32, u32, u32, u32, u16, bool, u32, bool); 8] = [
        (1, 0, 0, 0, 1, 0, 4, false, 0, false),
        (0, 1, 0, 0, 1, 0, 4, false, 0, false),
        (0, 0, 2, 0, 1, 0, 4, false, 0, false),
        (0, 0, 0, 0, 0, 0, 4, false, 0, false),
        (0, 0, 0, 1, 1, 0, 4, false, 0x01ad_0001, false),
        (0, 0, 0, 0, 3, 1, 3, true, 0, true),
        (0, 0, 0, 1, 1, 5, 4, true, 0x01ad_0001, false),
        (0, 0, 0, 0, 2, 2, 5, false, 0x01ad_0002, true),
    ];
    for (k, (w225, w5, w28, once, r227, hist, free, listed, r520, given)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x14, w5);
        t.set_u32(SCRATCH + 0x18, w28);
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(SCRATCH + 0x20, r520);
        t.set_u32(g(&t, 0x54), once);
        t.set_u32(gs(&t, 0x6b), hist);
        let hb = gs(&t, 0x93) - 1;
        junk(&mut t, hb, 8, k);
        t.set_u32(lock, 0x5a5a_0000 + k);
        // the effect block the load step reads (the start-up moves it
        // into the buffer block)
        let e0 = SCRATCH + 0x8000;
        t.set_u32(g(&t, 0x1b), e0);
        t.set_u32(e0 + 0x1a80, 0x0000_3014 + k);
        let e = b + 0x22_e500;
        junk(&mut t, e + 0xc70, 0x10, k);
        t.set_u32(e + 0x1a68, (0.375 * k as f32).to_bits());
        t.set_u32(gs(&t, 0x5), if given { b } else { 0 });
        let n = u32::from(free);
        for i in 0..n {
            t.write(node(i), &[0x40 + i as u8; 0x1c]);
            t.set_u32(node(i), node((i + n - 1) % n));
            t.set_u32(node(i) + 4, node((i + 1) % n));
        }
        t.set_u32(gs(&t, 0x26a), node(0));
        t.set_u16(gs(&t, 0x253), free);
        let tail = SCRATCH + 0x1400;
        t.write(tail, &[0x66; 0x1c]);
        t.set_u32(gs(&t, 0x2cc), if listed { tail } else { 0 });
        t.set_u32(gs(&t, 0x2df), tail);
        t.set_u32(gs(&t, 0x3cc), (0.25 * k as f32).to_bits());
        // the words the start-up clears or sets, junk first
        junk_ops(
            &mut t,
            su,
            &[
                0x7a, 0x112, 0x11c, 0x126, 0x130, 0x169, 0x20a, 0x22e, 0x183, 0x215, 0x239,
            ],
            k,
        );
        for o in [0x21du32, 0x241] {
            let at = gs(&t, o);
            t.set_u32(at, 0x5a5a_5a5a ^ o);
        }
        let recs = b + 0x22_6930;
        for r in 0..0x64u32 {
            t.set_u32(recs + 0x140 * r, 0x7e7e_0000 + r);
        }
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// What `r47::start_up_0251` (`magic_0251` 0x10004470) needs: the part
/// set-up's buffers, the object starts, the free ring all the takes share
/// (16 nodes), the actors (`n`, their bytes and handles), the task list
/// (empty or not), the camera flags; slot 262 writes a point from its
/// first word, 268 a vector, the float slots (333, 314, 404, 277, 15-17)
/// answer in ST0, the matrix slots log what they see.
fn start_up_0251_setup(t: &mut Twin, f: u32, k: u32, (n, listed, cam): (u32, bool, u32)) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    junk_ops(
        t,
        f,
        &[
            0x29, 0x33, 0x3d, 0x4b, 0x8c, 0x99, 0xa6, 0xbb, 0x12f, 0x148, 0x154, 0x15e, 0x188,
            0x18e, 0x198, 0x1a2, 0x1c7, 0x1d1, 0x8d7, 0x8e1, 0x8eb, 0x8f5, 0x8ff, 0x909, 0x913,
            0x91d, 0x92d, 0x9e8, 0xa86, 0xaa4, 0xaae, 0xab8, 0xac2, 0xacc, 0xad6, 0xc24,
        ],
        k,
    );
    for o in [0x1acu32, 0x1b5, 0x1bc, 0x927, 0x937, 0xae0] {
        let at = w(t, o);
        t.set_u32(at, 0x5a5a_5a5a ^ o ^ k);
    }
    magic_r11::pools(t, f, 3124);
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
    // 333 and 314 count their calls: the answers cycle (both wraps)
    t.set_u32(SCRATCH + 0x7e70, k);
    for s in [333u32, 314, 404] {
        rec(t, s, 1, move |m, a| {
            let c = m.r32(SCRATCH + 0x7e70) + 1;
            m.w32(SCRATCH + 0x7e70, c);
            (s << 16) ^ a[0]
        });
        t.stub_st0(s, move |m, a| {
            let c = m.r32(SCRATCH + 0x7e70);
            [-9.0, 0.5, 9.0, 2.5][(c % 4) as usize]
                + f64::from(f32::from_bits(a[0])) * 0.0625
                + f64::from(s & 3)
        });
    }
    for (s, x) in [(277u32, 0.5f64), (15, 1.25), (16, -0.75), (17, 2.5)] {
        rec(t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.0625 + x);
    }
    for s in [215u32, 413] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    // the record start's light (217) and its two matrix slots
    rec(t, 217, 2, move |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.125 * i as f32 + k as f32);
        }
        0x217
    });
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(t, 233, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * f32::from_bits(a[2]);
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [328u32, 329, 330] {
        rec(t, s, 3, move |m, a| {
            log16(m, a[1]);
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    rec(t, 229, 3, |m, a| {
        log16(m, a[1]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x248
    });
    for call in [0x74u32, 0x21f] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    // the free ring all the takes share
    magic_r11::ring(t, w(t, 0x7f4), w(t, 0x807), 16, SCRATCH + 0xd800);
    // the actors: count, bytes and handles
    let bytes = SCRATCH + 0x1e0;
    for i in 0..8u8 {
        t.set_u8(bytes + u32::from(i), 2 + 3 * i);
    }
    let at = w(t, 0x234);
    t.set_u32(at, bytes);
    let tab = w(t, 0x23f);
    for i in 0..0x20 {
        t.set_u32(tab + 4 * i, 0x0000_0080 + 7 * i + k);
    }
    let at = w(t, 0x1e1);
    t.set_u32(at, n);
    let at = w(t, 0x84);
    t.set_u32(at, 0x0000_00a0 + k);
    // the objects' vectors and the actors' records
    let at = w(t, 0x1f1);
    t.write(at - 0xd4, &[0x5e; 0x130 * 4]);
    let at = w(t, 0x1ec);
    junk(t, at - 0x58, 0xdc * 4, k);
    // the camera flags
    for (o, v) in [
        (0x142u32, cam & 1),
        (0x17d, cam & 2),
        (0x7a0, cam & 1),
        (0x7c5, cam & 2),
    ] {
        let at = w(t, o);
        t.set_u32(at, v);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    let at = w(t, 0x86c);
    t.set_u32(at, if listed { SCRATCH + 0x600 } else { 0 });
    let at = w(t, 0x885);
    t.set_u32(at, SCRATCH + 0x600);
}

/// `magic_0251` 0x10004470 (`r47::start_up_0251`, internal) and its load
/// step 0x10004340 (`r47::load_0251`): actors 0..3, the task list empty
/// and not, the camera flags both ways; the load step's waits.
#[test]
fn start_up_0251() {
    let Some(mut t) = twin(251) else { return };
    all_stubs(&mut t);
    let f = t.at(0x4470);
    for (k, case) in (0u32..).zip([
        (3u32, false, 0u32),
        (2, true, 1),
        (0, true, 2),
        (1, false, 3),
    ]) {
        start_up_0251_setup(&mut t, f, k, case);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r47::start_up_0251(g, img, fr)
        });
    }
    let load = t.at(0x4340);
    for (busy, asked, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0251_setup(&mut t, f, 1, (2, true, 1));
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

/// One of `r47::draw_0385`'s records at `r` (0x68 bytes of distinct
/// floats, then the case's words): `on`, the wait, the size steps
/// (`+0x44`, `+0x48`, `+0x4c`, `+0x50`), the angle step (both wraps), the
/// actor (-1 or an index), the flags byte, the blend (`n`, `t`), the
/// life, the frame cursor's playing word.
#[allow(clippy::too_many_arguments)]
fn draw_record_0385(
    t: &mut Twin,
    r: u32,
    k: u32,
    (on, wait, life): (bool, i32, i32),
    (n1, hold, n3, at): (i32, i32, i32, i32),
    (step, who, flags): (f32, u32, u8),
    (n, tt, idle): (i32, i32, bool),
) {
    for i in 0..0x1a {
        let x = 0.375 * i as f32 - 3.0 + k as f32 * 0.0625;
        t.set_u32(r + 4 * i, x.to_bits());
    }
    t.set_u32(r + 0x10, 0x10 + k);
    t.set_u32(r + 0x14, u32::from(!idle));
    t.set_u32(r + 0x18, 0x0001_0000 + k);
    t.set_u32(r + 0x2c, step.to_bits());
    t.set_u32(r + 0x30, who);
    t.set_u32(r + 0x34, 0x22 + k);
    t.set_u32(r + 0x38, life.cast_unsigned());
    t.set_u32(r + 0x3c, n.cast_unsigned());
    t.set_u32(r + 0x40, tt.cast_unsigned());
    t.set_u32(r + 0x44, n1.cast_unsigned());
    t.set_u32(r + 0x48, hold.cast_unsigned());
    t.set_u32(r + 0x4c, n3.cast_unsigned());
    t.set_u32(r + 0x50, at.cast_unsigned());
    t.set_u32(r + 0x54, wait.cast_unsigned());
    t.set_u32(r + 0x58, 0x8040_20f0 ^ (k << 3));
    t.set_u32(r + 0x5c, 0x10e0_c060 ^ k);
    t.set_u32(r + 0x60, u32::from(on));
    t.set_u32(r + 0x64, 0x5a5a_5a00 | u32::from(flags));
}

/// `magic_0385` 0x10001ed0 (`r47::draw_0385`, a stored callback): no
/// records, then records off, waiting, each size kind (running and
/// ending), the angle wrapped both ways, both flag bits, the blend
/// (running, ending, done), the life (negative idle and not, counting
/// down to 0 and not).
#[test]
fn draw_0385() {
    let Some(mut t) = twin(385) else { return };
    all_stubs(&mut t);
    let f = t.at(0x1ed0);
    magic_r11::object_slots(&mut t);
    for s in [723u32, 724, 235] {
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, s as f32 * 0.001 + i as f32 * 0.5 - 1.0);
            }
            s
        });
    }
    rec(&mut t, 330, 3, |m, a| {
        log16(m, a[1]);
        let ang = f32::from_bits(a[2]);
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + ang + i as f32;
            wf(m, a[0] + 4 * i, x);
        }
        330
    });
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..3 {
            wf(
                m,
                a[3] + 4 * i,
                (a[0] & 0xff) as f32 * 0.25 + i as f32 + (a[2] & 0xf) as f32,
            );
        }
        0x237
    });
    rec(&mut t, 229, 3, |m, a| {
        log16(m, a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(&mut t, 230, 2, |m, a| {
        log16(m, a[1]);
        for i in 0..16 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x230
    });
    t.stub_edx(772, |_, a| a[1] ^ 0x0772_0000);
    rec(&mut t, 773, 4, |m, a| {
        let c = m.r32(SCRATCH + 0x7e74) + 1;
        m.w32(SCRATCH + 0x7e74, c);
        (c & 1) | (a[3] & 0x100)
    });
    let link = t.img.rel32_target(f + 0x3a2);
    let lock = SCRATCH + 0x4100;
    for o in [0x4fu32, 0x7a, 0xa9, 0xc6, 0x115, 0x13b] {
        let at = t.img.word(link + o);
        t.set_u32(at, lock);
    }
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let bytes = SCRATCH + 0x1e0;
    for i in 0..8u8 {
        t.set_u8(bytes + u32::from(i), 2 + 3 * i);
    }
    let at = w(&t, 0x22f);
    t.set_u32(at, bytes);
    let tab = w(&t, 0x23a);
    for i in 0..0x20 {
        t.set_u32(tab + 4 * i, 0x0000_0080 + 7 * i);
    }
    let at = w(&t, 0x228);
    t.set_u32(at, 0x00ab_0000);
    let buf = SCRATCH + 0x1_8000;
    #[allow(clippy::type_complexity)]
    let recs: [(
        (bool, i32, i32),
        (i32, i32, i32, i32),
        (f32, u32, u8),
        (i32, i32, bool),
    ); 14] = [
        ((false, 0, 0), (2, 0, 0, 0), (0.5, 0, 0), (0, 0, false)),
        ((true, 3, 0), (2, 0, 0, 0), (0.5, 0, 0), (0, 0, false)),
        ((true, 0, 3), (3, 0, 0, 1), (9.0, 1, 3), (4, -1, false)),
        (
            (true, 0, 1),
            (2, 0, 0, 2),
            (-9.0, u32::MAX, 1),
            (3, 3, false),
        ),
        ((true, 0, -1), (0, 2, 0, 1), (0.25, 2, 2), (2, 1, true)),
        ((true, 0, -2), (0, 2, 0, 2), (0.25, 3, 0), (0, 0, false)),
        ((true, 0, 0), (0, 0, 3, 1), (0.5, 4, 1), (5, 4, false)),
        ((true, 0, 2), (0, 0, 2, 2), (-0.5, 5, 2), (-1, 0, false)),
        ((true, 0, 0), (0, 0, 0, 0), (0.5, 6, 3), (0, 0, true)),
        ((true, 0, -1), (0, 0, 0, 0), (0.5, 7, 0), (0, 0, false)),
        ((true, 1, 0), (0, 0, 0, 0), (0.5, 0, 0), (0, 0, false)),
        ((true, 0, 5), (0, 0, 0, 0), (0.5, 1, 1), (2, 0, false)),
        ((true, 0, 2), (3, 0, 0, 2), (0.5, 2, 0), (0, 0, false)),
        ((true, 0, 2), (0, 0, 3, 2), (0.5, 3, 0), (0, 0, false)),
    ];
    // (the counts end the walk after each kind of record: the colour
    // global keeps the last one's)
    for (k, n) in (0u32..).zip([0u32, 14, 3, 4, 5, 6, 7, 8]) {
        junk_ops(&mut t, f, &[0x58, 0x284, 0x2c2, 0x375, 0xce], k);
        t.set_u32(lock, 0x33 + k);
        let at = w(&t, 0x1f);
        t.set_u32(at, buf);
        for (i, &(a, b, c, d)) in (0u32..).zip(recs.iter()) {
            draw_record_0385(&mut t, buf + 0x68 * i, k + i, a, b, c, d);
        }
        let at = w(&t, 0x7e);
        t.set_u32(at, n);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The globals `r47::start_up_0385` stores (their operands).
const STORES_0385: &[u32] = &[
    0x29, 0x33, 0x3d, 0x4b, 0x24c, 0x271, 0x27c, 0x312, 0x3b0, 0x3ba, 0x3c4, 0x3ce, 0x3d8, 0x3e2,
    0x3ec, 0x499, 0x4a3, 0x4ad, 0x4b7, 0x4c1, 0x4cb, 0x4d5, 0x589, 0x593, 0x599, 0x5a3, 0x5ad,
    0x5b7, 0x5c1, 0x5cb, 0x5d5, 0x5df, 0x5e9, 0x5f3, 0x5fd, 0x607, 0x611, 0x61b, 0x625, 0x62f,
    0x639, 0x643, 0x64d, 0x6fa, 0x704, 0x70e, 0x718, 0x722, 0x72c, 0x736, 0x740, 0x74b, 0x751,
    0x75c, 0x7fe, 0x808, 0x812, 0x81c, 0x826, 0x830, 0x83a, 0x844, 0x84e, 0x858, 0x85e, 0x868,
    0x90b, 0x91d, 0x927, 0x951, 0x957, 0x961, 0x96b, 0x975, 0x97e,
];

/// What `r47::start_up_0385` (`magic_0385` 0x10003f00) needs: the stored
/// globals junked, the part set-up's buffers, the object starts, the
/// free ring (16 nodes), the actors (`n`), the sprite records (`recs`,
/// `busy` of them taken, the rover), the task list (empty or not), the
/// camera flags; slot 262 writes a point, 314 answers in ST0.
fn start_up_0385_setup(
    t: &mut Twin,
    f: u32,
    k: u32,
    (n, listed, cam): (u32, bool, u32),
    (recs, busy, rover): (u32, u32, u32),
) {
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    junk_ops(t, f, STORES_0385, k);
    magic_r11::pools(t, f, 2922);
    magic_r11::object_slots(t);
    rec(t, 262, 4, move |m, a| {
        for i in 0..3u32 {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0x262
    });
    rec(t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.75 - 0.125);
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.5);
    for s in [215u32, 413] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    for call in [0x91u32, 0xc5, 0x194] {
        let start = t.img.rel32_target(f + call);
        magic_r11::object_start_auto(t, start);
    }
    magic_r11::ring(t, w(t, 0x3a8), w(t, 0x406), 16, SCRATCH + 0xd800);
    // the actors: count, bytes and handles
    let bytes = SCRATCH + 0x1e0;
    for i in 0..8u8 {
        t.set_u8(bytes + u32::from(i), 2 + 3 * i);
    }
    let at = w(t, 0x122);
    t.set_u32(at, bytes);
    let tab = w(t, 0x12d);
    for i in 0..0x20 {
        t.set_u32(tab + 4 * i, 0x0000_0080 + 7 * i + k);
    }
    let at = w(t, 0xf6);
    t.set_u32(at, n);
    let at = w(t, 0x5b);
    t.set_u32(at, 0x0000_00a0 + k);
    // the objects
    let at = w(t, 0x6d);
    junk(t, at, 0x130 * 22, k);
    let at = w(t, 0x10a);
    junk(t, at, 0x390 * 4, k);
    // the sprite records
    let buf = SCRATCH + 0x1_8000;
    for i in 0..recs.max(1) {
        junk(t, buf + 0x68 * i, 0x68, k + i);
        t.set_u32(buf + 0x68 * i + 0x60, u32::from(i < busy));
    }
    for (o, v) in [(0x28bu32, recs), (0x291, buf), (0x297, rover)] {
        let at = w(t, o);
        t.set_u32(at, v);
    }
    // (the pool reset before the takes points them at the DLL's own 32
    // records, all free, the rover 0: its records junked)
    let pool = w(t, 0x276);
    junk(t, pool, 0x68 * 0x20, k);
    let fr0 = SCRATCH + 0x5c0;
    t.set_u32(fr0 + 0xc, 0x0fa0_0000 + k);
    let at = w(t, 0x30c);
    t.set_u32(at, fr0);
    // the camera flags
    for (o, v) in [(0x900u32, cam & 1), (0x946, cam & 2)] {
        let at = w(t, o);
        t.set_u32(at, v);
    }
    // the task list
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    for o in [0x461u32, 0x9fa] {
        let at = w(t, o);
        t.set_u32(at, if listed { SCRATCH + 0x600 } else { 0 });
    }
    let at = w(t, 0x47b);
    t.set_u32(at, SCRATCH + 0x600);
}

/// `magic_0385` 0x10003f00 (`r47::start_up_0385`, internal) and its load
/// step 0x10003dd0 (`r47::load_0385`): actors 0..3, the sprite records
/// all free, running out (busy ones skipped, the rover wrapping) and
/// none, the task list empty and not, the camera flags both ways; the
/// load step's waits.
#[test]
fn start_up_0385() {
    let Some(mut t) = twin(385) else { return };
    all_stubs(&mut t);
    let f = t.at(0x3f00);
    for (k, (case, pool)) in (0u32..).zip([
        ((3u32, false, 0u32), (24u32, 0u32, 0u32)),
        ((2, true, 1), (10, 3, 7)),
        ((0, true, 2), (0, 0, 0)),
        ((1, false, 3), (30, 25, 29)),
    ]) {
        start_up_0385_setup(&mut t, f, k, case, pool);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r47::start_up_0385(g, img, fr)
        });
    }
    let load = t.at(0x3dd0);
    for (busy, asked, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        start_up_0385_setup(&mut t, f, 1, (2, true, 1), (24, 0, 0));
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

/// A model for `r47::shards_0348` at `m`: `groups` (kind byte, faces)
/// after the header, the vertices after them; the corners of face `j`
/// are vertices `j`, `j + 1`, `j + 3`.
fn shard_model(t: &mut Twin, m: u32, k: u32, groups: &[(u8, u16)]) -> u32 {
    let total: u32 = groups.iter().map(|&(_, n)| u32::from(n)).sum();
    t.write(m, &[0x6b; 0x20]);
    t.set_u32(m + 4, 0x20);
    let mut g = m + 0x20;
    for (i, &(kind, n)) in (0u32..).zip(groups) {
        t.write(g, &[0x3c; 0x10]);
        t.set_u8(g + 1, kind);
        t.set_u16(g + 2, n);
        // the 64-bit word: low bits, the size field, two shifts
        let q: u64 = (0x1234 + u64::from(i))
            | (u64::from(5 + i + k) << 20)
            | (u64::from(3 + i) << 26)
            | (u64::from(7 + i) << 30)
            | (u64::from(0xabc + k) << 40);
        t.set_u32(g + 8, q as u32);
        t.set_u32(g + 0xc, (q >> 32) as u32);
        for j in 0..u32::from(n) {
            let face = g + 0x10 + 0x20 * j;
            t.write(face, &[0x2d; 0x20]);
            for (c, d) in [(0xcu32, 0u32), (0xe, 1), (0x10, 3)] {
                t.set_u16(face + c, (j + d + i) as u16);
            }
        }
        g += 0x10 + 0x20 * u32::from(n);
    }
    let verts = g;
    t.set_u32(m + 8, verts - m);
    for v in 0..0x40u32 {
        for (q, x) in (0u32..).zip([3i32, -5, 7]) {
            let x = (v as i32 * x + (k as i32) * 11 - 40) as i16;
            t.set_u16(verts + 6 * v + 2 * q, x as u16);
        }
    }
    t.set_u16(m + 0x10, total as u16);
    total
}

/// `magic_0348` 0x10006c70 (`r47::shards_start_0348`) and its tail
/// 0x10005360 (`r47::shards_0348`): one group, two and three groups,
/// a group of another kind (first and second), slot 885 refusing, no
/// faces, slot 883 answering non-zero.
#[test]
fn shards_0348() {
    let Some(mut t) = twin(348) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6c70);
    let sh = t.at(0x5360);
    let m = SCRATCH + 0x2_8000;
    rec(&mut t, 719, 3, move |_, _| m);
    rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ a[0]);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.75 - 0.125);
    rec(&mut t, 404, 1, |m, a| {
        let c = m.r32(SCRATCH + 0x7e70) + 1;
        m.w32(SCRATCH + 0x7e70, c);
        0x0404_0000 ^ a[0]
    });
    t.stub_st0(404, |m, a| {
        f64::from(f32::from_bits(a[0])) * 2.5 + f64::from(m.r32(SCRATCH + 0x7e70) % 7) * 0.25
    });
    let lock = SCRATCH + 0x4100;
    for o in [0x3fcu32, 0x477, 0x541, 0x560] {
        let at = t.img.word(sh + o);
        t.set_u32(at, lock);
    }
    // 885 logs the lock it sees, writes into the buffer, answers by case
    rec(&mut t, 885, 7, move |m, a| {
        m.w32(SCRATCH + 0x7f00 + 4 * (a[1] & 7), m.r32(lock));
        m.w32(a[6] + 8, a[0] ^ a[4]);
        m.r32(SCRATCH + 0x7f40)
    });
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| a[0] ^ 0x0077_2000);
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    // 882 copies the three arrays and the buffers' words it was given
    rec(&mut t, 882, 7, |m, a| {
        for i in 0..5 {
            for (j, p) in [a[3], a[4], a[6]].into_iter().enumerate() {
                let x = m.r32(p + 4 * i);
                m.w32(SCRATCH + 0x7f80 + 0x20 * j as u32 + 4 * i, x);
            }
            let x = m.r32(a[5] + 0x100 * i + 8);
            m.w32(SCRATCH + 0x7fe0 + 4 * i, x);
        }
        0x882
    });
    rec(&mut t, 883, 2, |m, _| m.r32(SCRATCH + 0x7f44));
    rec(&mut t, 779, 2, move |m, a| {
        m.w32(SCRATCH + 0x7f48, m.r32(lock));
        0x0779_0000 ^ a[0]
    });
    let s = SCRATCH + 0x2_c000;
    let at = t.img.word(f + 0xe);
    t.set_u32(at, s);
    #[allow(clippy::type_complexity)]
    let cases: [(&[(u8, u16)], u32, u32); 8] = [
        (&[(2, 3)], 1, 0),
        (&[(2, 2), (2, 3)], 1, 0),
        (&[(2, 1), (2, 2), (2, 1)], 1, 0x5a),
        (&[(5, 2)], 1, 0),
        (&[(2, 2), (1, 2)], 1, 0),
        (&[(2, 2), (2, 2)], 0, 0),
        (&[], 1, 0),
        (&[(2, 4)], 1, 7),
    ];
    for (k, (groups, ok, r883)) in (0u32..).zip(cases) {
        shard_model(&mut t, m, k, groups);
        junk(&mut t, s + 0xd30, 0x7c * 12, k);
        t.set_u32(lock, 0x40 + k);
        t.set_u32(SCRATCH + 0x7f40, ok);
        t.set_u32(SCRATCH + 0x7f44, r883);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0297` 0x10003130 (`r47::visit_thunk_0297`) running 0x10002f70
/// (`r47::visit_0297`): the group's mode equal to the owner's (the walk
/// stops), no children and two (their vtables' visits), the camera node
/// facing by slot 333 or 601, two shape functions (the table's entries
/// routed to stubs).
#[test]
fn visit_0297() {
    let Some(mut t) = twin(297) else { return };
    all_stubs(&mut t);
    let f = t.at(0x3130);
    let v = t.at(0x2f70);
    // the visits through vtables and the shape table: stubs that log the
    // group they get and its node
    for s in [1001u32, 1002, 1003, 1004] {
        let n = if s < 1003 { 1 } else { 2 };
        rec(&mut t, s, n, move |m, a| {
            let k = m.r32(SCRATCH + 0x3800);
            m.w32(SCRATCH + 0x3800, k + 1);
            m.w32(SCRATCH + 0x3810 + 4 * (k % 16), m.r32(a[0]) ^ (s << 20));
            if s == 1003 || s == 1004 {
                wf(m, a[1] + 4, 1.5 + k as f32);
            }
            0x1000 + k + s
        });
    }
    let class = SCRATCH + 0x3900;
    t.set_u32(class + 0x1c, TRAPS + 16 * 1001);
    let cam_class = SCRATCH + 0x3940;
    t.set_u32(cam_class + 0x1c, TRAPS + 16 * 1002);
    let tab = t.img.word(v + 0x9f);
    for i in 0..4 {
        t.set_u32(tab + 4 * i, TRAPS + 16 * (1003 + (i & 1)));
    }
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xf) * 0.5 - 1.0);
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], (a[0] & 0xf) as f32 * 0.25 + 0.5);
        0x601
    });
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3u32 {
            let x = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0;
            m.w32(a[1 + i as usize], x.to_bits());
        }
        0x262
    });
    for (s, n) in [(233u32, 3usize), (230, 2), (325, 2)] {
        rec(&mut t, s, n, move |m, a| {
            log16(m, a[1]);
            0x0abc_0000 | s
        });
    }
    let g = SCRATCH + 0x100;
    let n = SCRATCH + 0x1000;
    let c = SCRATCH + 0x1400;
    // (the group's mode against the owner's, children, facing flag, shape)
    for (k, (same, kids, face, shape)) in (0u32..).zip([
        (true, 2u32, 0u8, 0u8),
        (false, 0, 1, 1),
        (false, 2, 0, 2),
        (false, 1, 1, 3),
    ]) {
        junk(&mut t, n, 0x160, k);
        junk(&mut t, c, 0xa0, k + 7);
        t.write(g, &[0x5a, 0x5a, 0x5a, 0x5a, 0x33, 4, 0x77, 0x77]);
        t.set_u32(g, n);
        t.set_u8(n + 0x1a, if same { 4 } else { 9 });
        // the children (0x40 apart), circular through the owner
        let child = |i: u32| SCRATCH + 0x1200 + 0x40 * i;
        for i in 0..kids {
            junk(&mut t, child(i), 0x40, k + i);
            t.set_u32(child(i), class);
            let next = if i + 1 < kids { child(i + 1) } else { n };
            t.set_u32(child(i) + 8, next);
        }
        t.set_u32(n + 0x10, if kids == 0 { n } else { child(0) });
        t.set_u32(n + 0x14c, c);
        t.set_u8(n + 0x148, shape);
        t.set_u32(c, cam_class);
        t.set_u32(c + 0x54, 0x00c0_0000 + k);
        t.set_u32(c + 0x50, 0x00d0_0000 + k);
        t.set_u8(c + 0x90, face);
        floats(&mut t, n + 0x5c, 0x30, -1.0 + k as f32);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[g], Eax::Checked);
    }
}

/// `magic_0297` 0x10003d20 (`r47::image_thunk_0297`) running 0x100034f0
/// (`r47::image_visit_0297`): the walk stopping, the life running out,
/// the scale stepping (clamped up and down), the fade bytes (in the
/// middle, from below 0, past 0x100), nothing left (the node leaving its
/// parent or with none), paused or not, the draw with and without the
/// fade, slot 885 answering 0 or not.
#[test]
fn image_visit_0297() {
    let Some(mut t) = twin(297) else { return };
    all_stubs(&mut t);
    let f = t.at(0x3d20);
    let v = t.at(0x34f0);
    let w = |t: &Twin, o: u32| t.img.word(v + o);
    for s in [544u32, 588, 327] {
        rec(&mut t, s, 2, move |m, a| {
            for i in 0..4 {
                wf(
                    m,
                    a[1] + 4 * i,
                    s as f32 * 0.01 + i as f32 - (a[0] & 3) as f32,
                );
            }
            s
        });
    }
    for (s, n) in [(232u32, 2usize), (233, 3), (229, 3), (231, 3)] {
        rec(&mut t, s, n, move |m, a| {
            log16(m, a[1]);
            for i in 0..4 {
                let x = rf(m, a[1] + 4 * i) * 0.5 + s as f32 * 0.001;
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 277, 1, |_, a| 0x0277_0000 ^ a[0]);
    t.stub_st0(277, |_, a| f64::from(a[0] & 0xf) * 0.25 + 0.5);
    // the image: slot 941 hands out a buffer; for the mesh's one part a
    // part whose first strip ends it
    let image = SCRATCH + 0x2_0800;
    let part = SCRATCH + 0x2_0c00;
    rec(&mut t, 941, 1, move |_, a| {
        if a[0] == 0x00ee_0000 { part } else { image }
    });
    rec(&mut t, 857, 6, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[2 + i]);
            m.w32(SCRATCH + 0x7f60 + 4 * i as u32, x);
        }
        0x857
    });
    rec(&mut t, 885, 7, |m, a| {
        m.w32(a[6], a[0] ^ 0x885);
        m.r32(SCRATCH + 0x7f40)
    });
    // the mesh packets' work block
    let block = SCRATCH + 0x3_0000;
    rec(&mut t, 264, 1, move |_, _| block);
    let lock = SCRATCH + 0x4100;
    for o in [0x773u32, 0x7a1] {
        let at = w(&t, o);
        t.set_u32(at, lock);
    }
    // the byte table (the global holds its address)
    let tab = SCRATCH + 0x2_0f00;
    for i in 0..0x20u32 {
        t.set_u8(tab + i, (i as i8 * 3 - 40) as u8);
    }
    let at = w(&t, 0x25b);
    t.set_u32(at, tab);
    let c = SCRATCH + 0x5c0;
    t.set_u32(c + 0x28, 0x2828_0000);
    let at = w(&t, 0x613);
    t.set_u32(at, c);
    let g = SCRATCH + 0x100;
    let n = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x1300;
    let obj = SCRATCH + 0x1400;
    let cam = SCRATCH + 0x1500;
    let model = SCRATCH + 0x1700;
    // (same mode, paused, life, bits, step, fade, base, parent, 885)
    #[allow(clippy::type_complexity)]
    let cases: [(bool, u32, u16, u16, f32, f32, u16, bool, u32); 10] = [
        (true, 0, 5, 0xf, 0.5, 0.5, 0x40, false, 1),
        (false, 0, 0, 0xf, 0.5, 0.5, 0x40, true, 1),
        (false, 0, 5, 0xf, 0.5, 0.25, 0x40, false, 1),
        (false, 0, 5, 0xd, -0.5, -0.25, 0x20, true, 0),
        (false, 0, 5, 0x8, 0.5, -0.5, 0x20, false, 1),
        (false, 1, 5, 0xc, 0.25, 2.5, 0x30, false, 1),
        (false, 1, 5, 0x6, 4.0, 0.25, 0x30, false, 0),
        (false, 0, 5, 0x4, -4.0, 0.25, 0x30, false, 1),
        (false, 0, 5, 0x8, 0.5, 0.06, 0x08, false, 1),
        (false, 1, 5, 0x8, 0.5, 0.9375, 0x30, false, 1),
    ];
    for (k, (same, paused, life, bits, step, fade, base, with_parent, ok)) in (0u32..).zip(cases) {
        junk(&mut t, n, 0xc0, k);
        junk(&mut t, image, 0x180, k);
        junk(&mut t, parent, 0x20, k);
        t.write(g, &[0x5a, 0x5a, 0x5a, 0x5a, 0x33, 4, 0x77, 0x77]);
        t.set_u32(g, n);
        t.set_u8(n + 0x1a, if same { 4 } else { 9 });
        t.set_u32(n + 0x10, n);
        t.set_u32(n + 0x88, obj);
        t.set_u32(obj + 0x54, 0x00e0_0000 + k);
        t.set_u32(n + 0x8c, cam);
        floats(&mut t, cam + 0x9c, 0x10, 1.0 + k as f32);
        t.set_u32(n + 0x90, model);
        t.write(model, &[0; 0x20]);
        t.set_u8(model + 4, u8::from(k % 2 == 1));
        t.set_u32(model + 0x10, 0x00ee_0000);
        t.write(part, &[0x31; 0x40]);
        t.set_u32(part + 4, 0x10);
        t.set_u8(part + 0x11, 0xff);
        t.set_u32(n + 0x80, 0x0001_3456 + k);
        t.set_u16(n + 0xba, life);
        t.set_u16(n + 0xb4, bits);
        t.set_u16(n + 0xb8, base);
        t.set_u16(n + 0xb0, 0x0123);
        t.set_u8(n + 0xbc, 0x45);
        t.set_u32(n + 0xac, 0x00ac_0000 + k);
        for (o, x) in [
            (0xa0u32, step),
            (0x94, 1.0),
            (0x9c, 2.0),
            (0xa4, fade),
            (0xa8, 0.125),
            (0x98, 0.75),
        ] {
            t.set_u32(n + o, x.to_bits());
        }
        // the parent with this node its only child (or none)
        if with_parent {
            t.set_u32(n + 0x14, parent);
            t.set_u32(n + 4, parent);
            t.set_u32(n + 8, parent);
            t.set_u16(parent + 0x18, 1);
        } else {
            t.set_u32(n + 0x14, 0);
        }
        let at = w(&t, 0x72);
        t.set_u32(at, paused);
        t.set_u32(lock, 0x40 + k);
        t.set_u32(SCRATCH + 0x7f40, ok);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[g], Eax::Checked);
    }
}

/// The globals `r47::summon_task_0297` stores (their operands).
const STORES_7520: &[u32] = &[
    0xe5, 0xef, 0x129, 0x145, 0x15d, 0x165, 0x16f, 0x179, 0x183, 0x18c, 0x199, 0x1b8, 0x1c2, 0x1cc,
    0x1d6, 0x1e0, 0x1ea, 0x1f4, 0x1fb, 0x205, 0x210, 0x218, 0x222, 0x22c, 0x236, 0x23f, 0x24c,
    0x26b, 0x27e, 0x288, 0x292, 0x29c, 0x2a6, 0x2b0, 0x2ba, 0x338, 0x340, 0x348, 0x356, 0x376,
    0x37f, 0x38a, 0x394, 0x39a, 0x3a8, 0x3b3, 0x3bb, 0x3c5, 0x3cf, 0x3d9, 0x3e2, 0x3ef, 0x40e,
    0x418, 0x422, 0x42c, 0x436, 0x440, 0x44b, 0x453, 0x45d, 0x467, 0x471, 0x47a, 0x487, 0x4a7,
    0x4b1, 0x4bb, 0x4c5, 0x4cf, 0x4d9, 0x4e3, 0x4eb, 0x4f3, 0x4fd, 0x507, 0x511, 0x51b, 0x52d,
    0x539, 0x545, 0x552, 0x56a, 0x570, 0x57a, 0x584, 0x58e, 0x594, 0x59e, 0x5a8, 0x5fd, 0x607,
    0x611, 0x61b, 0x625, 0x62f, 0x639, 0x643, 0x660, 0x667, 0x671, 0x67b, 0x705, 0x70f, 0x719,
    0x723, 0x73a, 0x744, 0x74e, 0x758, 0x7a0, 0x7a7, 0x7b1, 0x7bb, 0x7c5, 0x853, 0x85d, 0x867,
    0x871, 0x898, 0x8a4, 0x8ae, 0x8b4,
];

/// `magic_0297` 0x10007520 (`r47::summon_task_0297`, a task step): every
/// state (0 running into 10, 11 with either flag and slot 20/411 at 0 or
/// not, 20, 21 and 23 before and after their counts, 22 with the model id
/// 0 or not, 10000, states without a case below 20 and above 23).
#[test]
fn summon_task_0297() {
    let Some(mut t) = twin(297) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7520);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    // slot 719's records (read back by the set-up)
    let p = SCRATCH + 0x2_1000;
    for i in 0..4 {
        t.set_u32(p + 4 * i, 0x7190_0000 + 0x1111 * i);
    }
    rec(&mut t, 719, 3, move |_, a| p + 4 * (a[1] & 1));
    for s in [20u32, 411] {
        rec(&mut t, s, 1, move |m, a| {
            m.r32(SCRATCH + 0x7f50) ^ (a[0] & 0xf0)
        });
    }
    for s in [333u32, 277] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, |_, a| f64::from(a[0] & 0xf) * 0.5 - 1.0);
    }
    rec(&mut t, 601, 2, |m, a| {
        wf(m, a[1], (a[0] & 0xf) as f32 * 0.25 + 0.5);
        0x601
    });
    rec(&mut t, 14, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.25 * i as f32 + 0.5);
        }
        0x14
    });
    // the particle nodes' owner (a global the game sets)
    let pnw = t.img.rel32_target(f + 0x799);
    let owner = SCRATCH + 0x2_1100;
    junk(&mut t, owner, 0x40, 3);
    let at = t.img.word(pnw + 0xd);
    t.set_u32(at, owner);
    let task = SCRATCH + 0x7c00;
    let r = SCRATCH + 0x7c40;
    let node = SCRATCH + 0x7c80;
    let (prev, next) = (SCRATCH + 0x7cc0, SCRATCH + 0x7d00);
    // (state, counter, flag byte, slot 20/411's answer)
    for (k, (state, count, flag, answer)) in (0u32..).zip([
        (0u32, 0u32, 1u8, 1u32),
        (0, 0, 0, 1),
        (0xa, 3, 0, 1),
        (0xb, 3, 1, 0),
        (0xb, 3, 1, 1),
        (0xb, 3, 0, 1),
        (0x14, 0x13, 0, 1),
        (0x14, 0x14, 1, 1),
        (0x15, 0x7c, 0, 1),
        (0x15, 0x7d, 0, 1),
        (0x16, 0, 1, 0),
        (0x16, 0, 0, 2),
        (0x17, 0xd1, 0, 1),
        (0x17, 0xd2, 1, 1),
        (0x2710, 0, 0, 1),
        (5, 0, 0, 1),
        (0x13, 0, 0, 1),
        (0x30, 0, 0, 1),
    ]) {
        t.write(task, &[0x31; 0x20]);
        junk(&mut t, r, 0x40, k);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x28, count);
        junk(&mut t, node, 0x20, k + 1);
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        junk(&mut t, prev, 0x20, k + 2);
        junk(&mut t, next, 0x20, k + 3);
        let at = w(&t, 0x4d);
        t.set_u32(at, u32::from(flag) | 0x5a00);
        t.set_u32(SCRATCH + 0x7f50, answer);
        junk_ops(&mut t, f, STORES_7520, k);
        // the scene's two list heads, empty (one with a child for odd k)
        for o in [0x116u32, 0x230] {
            let head = w(&t, o);
            junk(&mut t, head, 0x40, k);
            t.set_u32(head + 0xc, head);
            t.set_u32(head + 0x10, head);
            t.set_u16(head + 0x18, 0);
            if k % 2 == 1 {
                let c = SCRATCH + 0x7e00 + 0x40 * (o & 1);
                junk(&mut t, c, 0x40, k);
                t.set_u32(head + 0xc, c);
                t.set_u32(head + 0x10, c);
                t.set_u16(head + 0x18, 1);
            }
        }
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0267` 0x10002500 (`r47::start_up_0267`, internal) and its load
/// step 0x10001210 (`r47::load_0267`): the two yaws wrapped both ways and
/// not, the task list empty and not; the load step's waits.
#[test]
fn start_up_0267() {
    let Some(mut t) = twin(267) else { return };
    all_stubs(&mut t);
    let f = t.at(0x2500);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let setup = |t: &mut Twin, k: u32, listed: bool| {
        junk_ops(
            t,
            f,
            &[
                0x8, 0x12, 0x1c, 0x26, 0x4d, 0x9b, 0xf2, 0x11e, 0x17d, 0x1cc, 0x1d9, 0x1e6, 0x1f3,
                0x207, 0x234, 0x240, 0x246, 0x250, 0x262, 0x26c, 0x276, 0x280, 0x28a, 0x2a6, 0x2b5,
                0x2c0, 0x2d6, 0x2e0, 0x2f7, 0x301, 0x30c, 0x319, 0x323, 0x32d, 0x337, 0x341, 0x34b,
                0x355, 0x35f, 0x369,
            ],
            k,
        );
        // the unaligned word and its bytes
        let at = w(t, 0x29e);
        t.write(at, &[0x5b; 8]);
        magic_r11::pools(t, f, 1021);
        magic_r11::object_slots(t);
        rec(t, 262, 4, move |m, a| {
            for i in 0..3u32 {
                let v = (a[0] & 0xff) as f32 * 0.5 + i as f32 * 1.25 - 2.0 - k as f32;
                m.w32(a[1 + i as usize], v.to_bits());
            }
            0x262
        });
        t.set_u32(SCRATCH + 0x7e70, k);
        rec(t, 333, 1, |m, a| {
            let c = m.r32(SCRATCH + 0x7e70) + 1;
            m.w32(SCRATCH + 0x7e70, c);
            (333 << 16) ^ a[0]
        });
        t.stub_st0(333, |m, a| {
            let c = m.r32(SCRATCH + 0x7e70);
            [-9.0, 0.5, 9.0][(c % 3) as usize] + f64::from(a[0] & 0xf) * 0.125
        });
        for s in [215u32, 413, 312] {
            rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        }
        for call in [0x68u32, 0x14a] {
            let start = t.img.rel32_target(f + call);
            magic_r11::object_start_auto(t, start);
        }
        for o in [0x88u32, 0x95, 0x16a, 0x177, 0x22e] {
            let at = w(t, o);
            t.set_u32(at, 0x00a0_0000 + k);
        }
        // the task list
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        let at = w(t, 0x381);
        t.set_u32(at, if listed { SCRATCH + 0x600 } else { 0 });
        let at = w(t, 0x398);
        t.set_u32(at, SCRATCH + 0x600);
    };
    for (k, listed) in (0u32..).zip([false, true, false, true]) {
        setup(&mut t, k, listed);
        magic_h6::scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r47::start_up_0267(g, img, fr)
        });
    }
    let load = t.at(0x1210);
    for (busy, asked, ready) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)] {
        setup(&mut t, 1, true);
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

/// `magic_0267` 0x10002900 (`r47::spin_thunk_0267`) running 0x100029c0
/// (`r47::spin_0267`): the wait counting down and running out, the scale
/// 0 (and -0) with a wait at 0 and below, the angles wrapped both ways
/// and not, the scale growing, shrinking and stopping at 0.
#[test]
fn spin_0267() {
    let Some(mut t) = twin(267) else { return };
    all_stubs(&mut t);
    let f = t.at(0x2900);
    let v = t.at(0x29c0);
    let w = |t: &Twin, o: u32| t.img.word(v + o);
    magic_r11::object_slots(&mut t);
    rec(&mut t, 277, 1, |_, a| 0x0277_0000 ^ a[0]);
    t.stub_st0(277, |_, a| f64::from(a[0] & 0xf) * 0.25 + 0.5);
    for (s, x) in [(15u32, 1.25f64), (16, -0.75), (17, 2.5)] {
        rec(&mut t, s, 1, move |_, a| a[0] ^ (s << 16));
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.0625 + x);
    }
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |m, a| {
            log16(m, a[1]);
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let x = rf(m, a[1] + 4 * i) * 0.5 + ang + i as f32 + s as f32 * 0.001;
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    // (wait, scale, speed, the angles' start)
    for (k, (wait, scale, speed, ang)) in (0u32..).zip([
        (3i32, 0.5f32, 0.1f32, 0.0f32),
        (1, 0.5, 0.1, 0.0),
        (0, 0.0, 0.1, 0.0),
        (-1, -0.0, 0.1, 0.0),
        (0, 0.5, 0.125, 3.1),
        (0, 0.5, -0.25, -3.1),
        (0, 0.05, -0.1, 1.0),
        (-2, 0.25, -0.25, 2.0),
    ]) {
        let at = w(&t, 0x21c);
        junk(&mut t, at, 0x60, k);
        let at = w(&t, 0x2d3);
        floats(&mut t, at, 4, 1.5 + k as f32);
        for (o, x) in [
            (0x4du32, scale),
            (0xf5, speed),
            (0x65, ang),
            (0xad, -ang * 0.5),
            (0xd5, ang * 0.75),
            (0x6d, 0.3),
            (0x12b, 0.75),
            (0x182, 2.0),
        ] {
            let at = w(&t, o);
            t.set_u32(at, x.to_bits());
        }
        let at = w(&t, 0x11);
        t.set_u32(at, wait.cast_unsigned());
        for o in [0x15bu32, 0x2c] {
            let at = w(&t, o);
            t.set_u32(at, 0x00b0_0000 + k);
        }
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0267` 0x100013f0 (`r47::mesh_register_0267`, internal: `ecx` =
/// the model, cdecl `(lo, hi)`): one group, three groups of kind 1 and
/// others, no faces, slot 883 answering non-zero.
#[test]
fn mesh_register_0267() {
    let Some(mut t) = twin(267) else { return };
    all_stubs(&mut t);
    let f = t.at(0x13f0);
    rec(&mut t, 776, 1, |_, a| 0x0776_0000 ^ a[0]);
    // 882 copies the three arrays and the buffers' first words
    rec(&mut t, 882, 7, |m, a| {
        for i in 0..8 {
            for (j, p) in [a[3], a[4], a[6]].into_iter().enumerate() {
                let x = m.r32(p + 4 * i);
                m.w32(SCRATCH + 0x7a00 + 0x20 * j as u32 + 4 * i, x);
            }
            for q in 0..3 {
                let x = m.r32(a[5] + 0x100 * i + 4 * q);
                m.w32(SCRATCH + 0x7b00 + 0x10 * i + 4 * q, x);
            }
        }
        0x882
    });
    rec(&mut t, 883, 2, |m, _| m.r32(SCRATCH + 0x7f44));
    let lock = SCRATCH + 0x4100;
    for o in [0x17cu32, 0x198] {
        let at = t.img.word(f + o);
        t.set_u32(at, lock);
    }
    rec(&mut t, 779, 2, move |m, a| {
        m.w32(SCRATCH + 0x7f48, m.r32(lock));
        0x0779_0000 ^ a[0]
    });
    let m = SCRATCH + 0x2_8000;
    #[allow(clippy::type_complexity)]
    let cases: [(&[(u8, u16)], u32); 5] = [
        (&[(1, 3)], 0),
        (&[(1, 2), (2, 3), (1, 1)], 0),
        (&[(0, 2), (1, 4)], 0),
        (&[], 0),
        (&[(1, 4)], 7),
    ];
    for (k, (groups, r883)) in (0u32..).zip(cases) {
        // the model: the groups after the header (kind 1 followed by
        // its faces)
        t.write(m, &[0x6b; 0x20]);
        t.set_u32(m + 4, 0x20);
        let mut g = m + 0x20;
        let mut total = 0u16;
        for &(kind, n) in groups {
            t.write(g, &[0x3c; 0x10]);
            t.set_u8(g + 1, kind);
            t.set_u16(g + 2, n);
            g += 0x10;
            if kind == 1 {
                t.write(g, &vec![0x2d; 0x1c * usize::from(n)]);
                g += 0x1c * u32::from(n);
            }
            total += n;
        }
        t.set_u16(m + 0x10, total);
        t.set_u32(lock, 0x40 + k);
        t.set_u32(SCRATCH + 0x7f44, r883);
        magic_h6::scrub(&mut t);
        let call = Call::this(m);
        let (lo, hi) = (0x0012_3400 + k, 0x00ab_0000 + k);
        t.check(f, call, &[lo, hi], Eax::Checked, |g, img, fr| {
            ffx_magic::r47::mesh_register_0267(g, img, fr)
        });
    }
}

/// `magic_0348` 0x10006b40 (`r47::shards_step_0348`, a task step) running
/// the shard mesh 0x100058e0 (`r47::shards_draw_0348`): slot 773 with no
/// mesh, one group with living and dead shards, groups of another kind
/// and one slot 886 refuses, more than 20 shards (the count's run), no
/// shard drawn (the handle released, the task given back).
#[test]
fn shards_draw_0348() {
    let Some(mut t) = twin(348) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6b40);
    let m = SCRATCH + 0x2_8000;
    let e = SCRATCH + 0x2_c000;
    let at = t.img.word(f + 0x17);
    t.set_u32(at, e);
    rec(&mut t, 719, 3, move |_, _| m);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| a[0] ^ 0x0077_2000);
    rec(&mut t, 773, 2, |m, _| m.r32(SCRATCH + 0x7f40));
    rec(&mut t, 887, 2, |_, a| 0x0887_0000 + a[1]);
    rec(&mut t, 886, 4, |m, a| {
        u32::from(m.r32(SCRATCH + 0x7f44) != a[1])
    });
    // the packet's buffers
    let (pos, uv, col, idx) = (
        SCRATCH + 0x3_0000,
        SCRATCH + 0x3_2000,
        SCRATCH + 0x3_4000,
        SCRATCH + 0x3_8000,
    );
    for (s, b) in [(898u32, pos), (901, uv), (900, col), (902, idx)] {
        rec(&mut t, s, 1, move |_, a| b + 0x400 * (a[0] & 3));
    }
    let (mm, bmin, bmax) = (SCRATCH + 0x3_a000, SCRATCH + 0x3_a100, SCRATCH + 0x3_a200);
    for (s, b) in [(893u32, mm), (890, bmin), (891, bmax)] {
        rec(&mut t, s, 1, move |_, _| b);
    }
    for s in [723u32, 724, 235] {
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, s as f32 * 0.001 + i as f32 * 0.5 - 1.0);
            }
            s
        });
    }
    for s in [329u32, 328, 230] {
        let n = if s == 230 { 2 } else { 3 };
        rec(&mut t, s, n, move |m, a| {
            log16(m, a[1]);
            let ang = if s == 230 {
                0.0
            } else {
                f32::from_bits(a[2]) * 0.01
            };
            for i in 0..16 {
                let x = rf(m, a[1] + 4 * i) * 0.5 + ang + i as f32 * 0.125;
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 4 * i) * 0.75 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    // 229: a point with a w of its own (the corners divide by it)
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 0x30 + 4 * i) + rf(m, a[2] + 4 * i) * 1.5;
            wf(m, a[0] + 4 * i, x);
        }
        let ww = 1.25 + rf(m, a[2]) * 0.0625 + rf(m, a[2] + 0xc) * 0.5;
        wf(m, a[0] + 0xc, ww);
        0x229
    });
    // 231 (out, a, b): both inputs count (b's last row the face's place)
    rec(&mut t, 231, 3, |m, a| {
        log16(m, a[2]);
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i) * 0.25;
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    let lock = SCRATCH + 0x4100;
    let sd = t.img.rel32_target(f + 0xb3);
    for o in [0x86u32, 0xb0, 0xc53, 0xc74] {
        let at = t.img.word(sd + o);
        t.set_u32(at, lock);
    }
    for o in [0xebu32, 0x107] {
        let at = t.img.word(f + o);
        t.set_u32(at, lock);
    }
    let at = t.img.word(sd + 0xba8);
    t.set_u32(at, SCRATCH + 0x3_a300);
    floats(&mut t, SCRATCH + 0x3_a300, 16, 0.5);
    for s in [779u32, 775] {
        rec(&mut t, s, 2, move |m, a| {
            m.w32(SCRATCH + 0x7f48 + (s & 4), m.r32(lock));
            (s << 16) ^ a[0]
        });
    }
    let task = SCRATCH + 0x7c00;
    #[allow(clippy::type_complexity)]
    let cases: [(&[(u8, u16)], u32, u32); 7] = [
        (&[(2, 3)], 0, 9),
        (&[(2, 3)], 1, 9),
        (&[(2, 2), (1, 2), (2, 3)], 1, 9),
        (&[(2, 2), (2, 3)], 1, 0),
        (&[(2, 12), (2, 11)], 1, 9),
        (&[(2, 2)], 1, 9),
        (&[(2, 28)], 1, 9),
    ];
    for (k, (groups, mesh, refuse)) in (0u32..).zip(cases) {
        shard_model(&mut t, m, k, groups);
        floats(&mut t, e + 0xd20, 4, 1.0 + k as f32);
        // the records: a life by index (0 dead), the divisor, steps
        let total: u32 = groups.iter().map(|&(_, n)| u32::from(n)).sum();
        for i in 0..total {
            let r = e + 0xd30 + 0x7c * i;
            floats(&mut t, r, 0x1f, -2.0 + 0.125 * i as f32);
            let life = if k == 5 || (k != 6 && i % 4 == 1) {
                0
            } else {
                1 + i
            };
            t.set_u32(r + 0x78, life);
            // (a negative divisor now and then: the alpha's sign bit)
            let div = if i % 7 == 3 {
                -3i32
            } else {
                3 + (i % 5) as i32
            };
            t.set_u32(r + 0x74, div.cast_unsigned());
        }
        t.set_u32(SCRATCH + 0x7f40, mesh);
        t.set_u32(SCRATCH + 0x7f44, refuse);
        t.set_u32(lock, 0x40 + k);
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, task + 0x10);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// A part for `r47::mesh_0434` at `p`: `groups` (kind byte, faces of
/// 0x1c bytes) from `+0x20`, then 0x40 points and 0x40 normals (16 bytes
/// each); face `j`'s corners are points `j`, `j + 1`, `j + 2` and normals
/// `j + 3`, `j + 1`, `j`.
fn mesh_part(t: &mut Twin, p: u32, k: u32, groups: &[(u8, u16)]) {
    let total: u32 = groups.iter().map(|&(_, n)| u32::from(n)).sum();
    t.write(p, &[0x6b; 0x20]);
    t.set_u32(p + 4, 0x20);
    let mut g = p + 0x20;
    for &(kind, n) in groups {
        t.write(g, &[0x3c; 0x20]);
        t.set_u8(g + 1, kind);
        t.set_u16(g + 2, n);
        g += 0x20;
        if kind == 0x10 {
            for j in 0..u32::from(n) {
                let face = g + 0x1c * j;
                t.write(face, &[0x2d; 0x1c]);
                for (c, (pi, ni)) in [(0u32, (0u32, 3u32)), (2, (1, 1)), (4, (2, 0))] {
                    t.set_u16(face + c, ((j + pi) % 0x40) as u16);
                    t.set_u16(face + 0x14 + c, ((j + ni) % 0x40) as u16);
                }
            }
            g += 0x1c * u32::from(n);
        }
    }
    let verts = g;
    t.set_u32(p + 8, verts - p);
    floats(t, verts, 0x100, -3.0 + k as f32);
    let normals = verts + 0x400;
    t.set_u32(p + 0xc, normals - p);
    for i in 0..0x40u32 {
        let x = (i as f32 * 0.37 + k as f32).sin();
        let y = (i as f32 * 0.21).cos();
        for (q, v) in [x, y, 0.5 - x * 0.25, 0.0].into_iter().enumerate() {
            t.set_u32(normals + 0x10 * i + 4 * q as u32, v.to_bits());
        }
    }
    t.set_u16(p + 0x10, total as u16);
    t.set_u16(p + 0x12, 0x40);
}

/// `magic_0434` 0x10004840 (`r47::mesh_0434`, internal: `edx` = the
/// model, seven stack words): slot 773 answering 0 or not, one group, a
/// group of another kind, more than 20 faces (the packet buffers
/// flipped twice), no faces.
#[test]
fn mesh_0434() {
    let Some(mut t) = twin(434) else { return };
    all_stubs(&mut t);
    let f = t.at(0x4840);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let part = SCRATCH + 0x2_0000;
    let bone = SCRATCH + 0x2_3000;
    floats(&mut t, bone, 16, 0.25);
    rec(&mut t, 941, 1, move |_, a| {
        if a[0] == 0x0941_0002 { bone } else { part }
    });
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
    t.stub_edx(772, |_, a| a[1] ^ 0x0077_2000);
    rec(&mut t, 773, 2, |m, _| m.r32(SCRATCH + 0x7f40));
    let texp = SCRATCH + 0x2_3100;
    t.set_u32(texp, 0x7e70_0000);
    rec(&mut t, 719, 3, move |_, _| texp);
    let proj = SCRATCH + 0x2_4000;
    rec(&mut t, 562, 0, move |_, _| proj);
    for s in [235u32, 723, 724] {
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, s as f32 * 0.001 + i as f32 * 0.25 - 1.0);
            }
            s
        });
    }
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x230
    });
    rec(&mut t, 231, 3, |m, a| {
        log16(m, a[2]);
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i) * 0.25;
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    // 229 (out, M, v): every word of M and v counts
    rec(&mut t, 229, 3, |m, a| {
        let mut o = [0f32; 4];
        for (i, oi) in o.iter_mut().enumerate() {
            let i = i as u32;
            let mut x = 0.0;
            for j in 0..4u32 {
                x += rf(m, a[1] + 0x10 * j + 4 * i) * rf(m, a[2] + 4 * j) * 0.125;
            }
            *oi = x;
        }
        o[3] += 1.25;
        for (i, x) in (0u32..).zip(o) {
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    rec(&mut t, 829, 5, |m, a| {
        m.w32(a[3], 0x0829_0000);
        m.w32(a[4], a[1]);
        0x829
    });
    let lock = SCRATCH + 0x4100;
    for o in [0x51u32, 0x6e, 0x3f9, 0x426, 0xbeb, 0xc15, 0xd65, 0xd89] {
        let at = w(&t, o);
        t.set_u32(at, lock);
    }
    rec(&mut t, 830, 3, move |m, a| {
        let c = m.r32(SCRATCH + 0x7f4c) + 1;
        m.w32(SCRATCH + 0x7f4c, c);
        m.w32(SCRATCH + 0x7f50 + 4 * (c % 8), m.r32(lock) ^ a[1]);
        0x830
    });
    let (za, zz, zx, zy) = (
        SCRATCH + 0x2_3200,
        SCRATCH + 0x2_3300,
        SCRATCH + 0x2_3400,
        SCRATCH + 0x2_3500,
    );
    for (i, at) in (0u32..).zip([za, zz, zx, zy]) {
        floats(&mut t, at, 16, 0.5 + i as f32);
    }
    let model = SCRATCH + 0x2_3600;
    #[allow(clippy::type_complexity)]
    let cases: [(&[(u8, u16)], u32); 4] = [
        (&[(0x10, 3)], 0),
        (&[(0x10, 2), (5, 1), (0x10, 22)], 1),
        (&[], 1),
        (&[(0x10, 20)], 2),
    ];
    for (k, (groups, answer)) in (0u32..).zip(cases) {
        mesh_part(&mut t, part, k, groups);
        // the two packet buffers
        for o in [0x1a4u32, 0x196] {
            let at = w(&t, o);
            junk(&mut t, at, 0x1a60, k + o);
        }
        t.set_u32(model, 0x0941_0001);
        t.set_u32(model + 4, 0x0941_0002);
        t.set_u32(SCRATCH + 0x7f40, answer);
        t.set_u32(lock, 0x40 + k);
        let call = Call {
            edx: model,
            ..Call::default()
        };
        let args = [
            za,
            zz,
            zx,
            zy,
            0x0001_2300 + k,
            0x0004_5600 + k,
            (0.75f32).to_bits(),
        ];
        magic_h6::scrub(&mut t);
        t.check(f, call, &args, Eax::Checked, |g, img, fr| {
            ffx_magic::r47::mesh_0434(g, img, fr)
        });
    }
}

/// `magic_0434` 0x10006f60 (`r47::summon_draw_0434`, a stored callback)
/// running the mesh 0x10004840: the drift with the first wait running,
/// the second wait turning the spin (wrapped both ways and not), both
/// waits out; the summon model with and without a handle (the bones).
#[test]
fn summon_draw_0434() {
    let Some(mut t) = twin(434) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6f60);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let mf = t.img.rel32_target(f + 0x5ce);
    let wm = |t: &Twin, o: u32| t.img.word(mf + o);
    let part = SCRATCH + 0x2_0000;
    let bone = SCRATCH + 0x2_3000;
    floats(&mut t, bone, 16, 0.25);
    rec(&mut t, 941, 1, move |_, a| {
        if a[0] == 0x0941_0002 { bone } else { part }
    });
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
    t.stub_edx(772, |_, a| a[1] ^ 0x0077_2000);
    let texp = SCRATCH + 0x2_3100;
    t.set_u32(texp, 0x7e70_0000);
    rec(&mut t, 719, 3, move |_, _| texp);
    let proj = SCRATCH + 0x2_4000;
    rec(&mut t, 562, 0, move |_, _| proj);
    for s in [235u32, 723, 724] {
        rec(&mut t, s, 1, move |m, a| {
            for i in 0..16 {
                wf(m, a[0] + 4 * i, s as f32 * 0.001 + i as f32 * 0.25 - 1.0);
            }
            s
        });
    }
    rec(&mut t, 544, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, a[0] as f32 + i as f32 * 0.5);
        }
        0x544
    });
    for s in [330u32, 328, 329] {
        rec(&mut t, s, 3, move |m, a| {
            log16(m, a[1]);
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let x = rf(m, a[1] + 4 * i) * 0.5 + ang * 0.25 + s as f32 * 0.001;
                wf(m, a[0] + 4 * i, x);
            }
            s
        });
    }
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    rec(&mut t, 230, 2, |m, a| {
        for i in 0..16 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x230
    });
    rec(&mut t, 231, 3, |m, a| {
        log16(m, a[2]);
        for i in 0..16 {
            let x = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i) * 0.25;
            wf(m, a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 229, 3, |m, a| {
        let mut o = [0f32; 4];
        for (i, oi) in o.iter_mut().enumerate() {
            let i = i as u32;
            let mut x = 0.0;
            for j in 0..4u32 {
                x += rf(m, a[1] + 0x10 * j + 4 * i) * rf(m, a[2] + 4 * j) * 0.125;
            }
            *oi = x;
        }
        o[3] += 1.25;
        for (i, x) in (0u32..).zip(o) {
            wf(m, a[0] + 4 * i, x);
        }
        0x229
    });
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..3 {
            let x = rf(m, a[1] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    let lock = SCRATCH + 0x4100;
    for o in [0x51u32, 0x6e, 0x3f9, 0x426, 0xbeb, 0xc15, 0xd65, 0xd89] {
        let at = wm(&t, o);
        t.set_u32(at, lock);
    }
    // the selection, the summon model, the effect block, the mesh's model
    let sel = SCRATCH + 0x1e0;
    t.set_u8(sel, 3);
    let at = w(&t, 0x1b);
    t.set_u32(at, sel);
    let h = SCRATCH + 0x2_5000;
    let tab = w(&t, 0x27);
    t.set_u32(tab + 4 * 3, h);
    let e = SCRATCH + 0x2_8000;
    let at = w(&t, 0x32);
    t.set_u32(at, e);
    let b = SCRATCH + 0x2_3600;
    let at = w(&t, 0x58a);
    t.set_u32(at, b);
    // (waits, spin, with a model handle)
    for (k, (wait, wait2, spin, handle)) in (0u32..).zip([
        (3i32, 2i32, 0.5f32, true),
        (0, 2, 3.1, false),
        (0, 2, -3.1, true),
        (0, 0, 1.0, true),
        (-1, 1, 0.25, false),
    ]) {
        mesh_part(&mut t, part, k, &[(0x10, 2)]);
        for o in [0x1a4u32, 0x196] {
            let at = wm(&t, o);
            junk(&mut t, at, 0x1a60, k + o);
        }
        t.set_u32(b + 0x30, 0x0941_0001);
        t.set_u32(b + 0x34, 0x0941_0002);
        junk(&mut t, h, 0x840, k);
        t.set_u32(h + 0x830, if handle { 0x0083_0000 + k } else { 0 });
        let recs = SCRATCH + 0x2_7000;
        junk(&mut t, recs, 0x100, k);
        t.set_u32(h + 0x1bc, recs);
        t.set_u32(h + 0x328, SCRATCH + 0x2_7400);
        floats(&mut t, e + 0xc20, 0x30, 0.5 + k as f32);
        floats(&mut t, e + 0x1a6c, 2, 1.0 + k as f32);
        t.set_u32(e + 0x1a74, spin.to_bits());
        t.set_u32(e + 0x1a78, wait.cast_unsigned());
        t.set_u32(e + 0x1a7c, wait2.cast_unsigned());
        t.set_u32(lock, 0x40 + k);
        magic_h6::scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
