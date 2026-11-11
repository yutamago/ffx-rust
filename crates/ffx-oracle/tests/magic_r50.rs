//! Round 50's ports (`ffx_magic::r50`, the last rows of `magic_0387`)
//! against each original. Needs `FFX_GAME_DIR`.

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

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
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

/// Every slot answering `(slot << 20) | 5` (no argument read: a port
/// deeper in a chain passes none).
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, _| (s << 20) | 5);
    }
}

/// Slot 325 (copy a vector `(to, from)`, changed so that each copy
/// shows), logged.
fn copy_325(t: &mut Twin) {
    rec(t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 2.0 - 1.0;
            wf(m, a[0] + 4 * i, x);
        }
        0x325
    });
}

/// Slot 231 `(to, a, b)`: a 4x4 result from both matrices.
fn matrix_231(t: &mut Twin) {
    rec(t, 231, 3, |m, a| {
        for i in 0..16 {
            let v = rf(m, a[1] + 4 * i) * 0.5 + rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x231
    });
}

/// Where slot 308's stub logs the request it reads.
const DRAW_LOG: u32 = SCRATCH + 0x3_0000;

/// Slot 308 `(mesh, request)`: logs the request's 0x48 bytes, the five
/// matrix-and-vector rows it names (`+0x20`: 0x50 bytes; `+0x30`: 0x40)
/// and the mesh header; answers a count.
fn draw_308(t: &mut Twin) {
    rec(t, 308, 2, |m, a| {
        let k = m.r32(DRAW_LOG);
        m.w32(DRAW_LOG, k + 1);
        let r = a[1];
        let mut at = DRAW_LOG + 4;
        for i in 0..0x12 {
            m.w32(at, m.r32(r + 4 * i));
            at += 4;
        }
        let m1 = m.r32(r + 0x20);
        for i in 0..0x14 {
            m.w32(at, m.r32(m1 + 4 * i));
            at += 4;
        }
        let m2 = m.r32(r + 0x30);
        for i in 0..0x10 {
            m.w32(at, m.r32(m2 + 4 * i));
            at += 4;
        }
        for i in 0..8 {
            m.w32(at, m.r32(a[0] + 4 * i));
            at += 4;
        }
        0x0308_0000 + k
    });
}

/// `magic_0387` 0x10003d00 (`r50::strip_mesh_0387`, internal: `ecx` the
/// effect, `edx` the strip): windows with a head only, head and rest,
/// a rest of one, the whole strip; repeated points (a zero length), a
/// big alpha (`cvttss2si` out of range). The `jmp $` case (a head of
/// one point or none) spins for ever and stays out.
#[test]
fn strip_mesh_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x3d00);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    all_stubs(&mut t);
    copy_325(&mut t);
    matrix_231(&mut t);
    draw_308(&mut t);
    let (e, s, data, ga) = (
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x2000,
        SCRATCH + 0x300,
    );
    // (the strip: length, start, window; repeated points, alpha scale)
    for (k, (len, start, window, same, scale)) in (0u32..).zip([
        (10u16, 2u16, 3u16, false, 40.0f32),
        (10, 8, 5, false, 30.0),
        (10, 7, 6, true, 25.0),
        (10, 0, 10, false, 20.0),
        (6, 3, 4, false, 1.0e9),
        (12, 1, 2, false, 10.0),
        (5, 3, 3, false, 50.0),
    ]) {
        for (n, o) in [(16, 0x3c), (16, 0x30), (16, 0x5e), (16, 0x59)] {
            floats(&mut t, w(o), n, k + o);
        }
        junk(&mut t, w(0x7e), 0x400, k);
        junk(&mut t, e + 0x3d0, 0x30, k);
        setf(&mut t, e + 0x3e4, scale);
        t.set_u32(e + 0x3f0, 0x00c0_ffee ^ (k << 4));
        t.set_u32(e + 0x3f4, 0x1234_5678 + k);
        junk(&mut t, s, 0x40, k + 1);
        t.set_u32(s + 0x1c, data);
        t.write(s + 0x20, &len.to_le_bytes());
        t.write(s + 0x22, &0x10u16.to_le_bytes());
        t.write(s + 0x26, &window.to_le_bytes());
        t.write(s + 0x28, &start.to_le_bytes());
        floats(&mut t, data, 4 * 16, k);
        for i in 0..16 {
            // a positive half width
            setf(&mut t, data + 16 * i + 0xc, 0.25 + (i + k) as f32 * 0.375);
        }
        if same {
            for o in [0u32, 4, 8, 0xc] {
                let x = t.u32(data + 16 * 8 + o);
                t.set_u32(data + 16 * 9 + o, x);
            }
        }
        t.set_u32(w(0x8cc), ga);
        t.set_u32(ga, 0x0ab0_0000 + k);
        t.set_u32(w(0x932), 0x0cd0_0000 + k);
        ownb::dirty_stack(&mut t, 0x3d00 + k, &[]);
        let c = Call {
            ecx: e,
            edx: s,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            ffx_magic::r50::strip_mesh_0387(g, img, fr)
        });
        // the draw ran: the whole loop came through
        assert_eq!(t.u32(DRAW_LOG), k + 1);
    }
}

/// Slots `s` of `(to, a, b)` writing `n` floats: `a K + b` (a different
/// `K` per slot, `b` a float word or a pointer).
fn vec_slot(t: &mut Twin, s: u32, n: u32, k: f32, b_is_word: bool) {
    rec(t, s, 3, move |m, a| {
        for i in 0..n {
            let y = if b_is_word {
                f32::from_bits(a[2])
            } else {
                rf(m, a[2] + 4 * i)
            };
            let x = rf(m, a[1] + 4 * i) * k + y;
            wf(m, a[0] + 4 * i, x);
        }
        s
    });
}

/// `magic_0387` 0x100037a0 (`r50::laser_visit_0387`, internal) and its
/// thunk 0x100046b0: the mode already set; the strip's laser move at its
/// own mode (the move's walk stops: r48 tests the rest); the mesh with
/// a window of 2 or more, none below; new particles from `j` (none at
/// `c` >= 0x30 or `j` >= `c`, a negative `j`), the live ones past their
/// frames or not, the last head point, half widths dropping to 0, `G_F`
/// set or not.
#[test]
fn laser_visit_0387() {
    let Some(mut t) = sa::twin(387) else { return };
    let f = t.at(0x37a0);
    let fm = t.at(0x3d00);
    let img = t.img.clone();
    let w = |o: u32| img.word(f + o);
    let wm = |o: u32| img.word(fm + o);
    all_stubs(&mut t);
    copy_325(&mut t);
    matrix_231(&mut t);
    draw_308(&mut t);
    vec_slot(&mut t, 234, 4, -1.0, false);
    vec_slot(&mut t, 233, 4, 0.75, true);
    vec_slot(&mut t, 248, 4, 1.5, false);
    vec_slot(&mut t, 229, 4, 0.5, false);
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.25 + 0.125;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    let row = SCRATCH + 0x3_e000;
    rec(&mut t, 307, 0, move |m, _| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        0x307
    });
    t.stub_st0(307, move |m, _| f64::from(m.r32(row) as f32 * 0.0625 + 0.1));
    let (v, o, s, q, data, ga) = (
        SCRATCH + 0x100,
        SCRATCH + 0x4000,
        SCRATCH + 0x200,
        SCRATCH + 0x600,
        SCRATCH + 0x2000,
        SCRATCH + 0x300,
    );
    let class = SCRATCH + 0x700;
    // (O's mode, the window: length, start, window; c, j, G_F)
    for (k, (omode, len, start, window, c, j, gf)) in (0u32..).zip([
        (3u8, 10u16, 2u16, 3u16, 5u8, 2i32, 0u32),
        (7, 10, 2, 3, 5, 2, 0),
        (3, 10, 4, 1, 4, 4, 0),
        (3, 10, 1, 6, 0x30, 0, 1),
        (3, 10, 8, 5, 6, -1, 0),
        (3, 8, 0, 8, 9, 3, 0),
        (3, 10, 3, 4, 3, 0, 0),
    ]) {
        for (n, op) in [(16, 0x3c), (16, 0x30), (16, 0x5e), (16, 0x59)] {
            floats(&mut t, wm(op), n, k + op);
        }
        floats(&mut t, w(0x3fa), 16, k + 3);
        floats(&mut t, w(0x438), 16, k + 5);
        t.set_u32(wm(0x8cc), ga);
        t.set_u32(ga, 0x0ab0_0000 + k);
        t.set_u32(w(0x201), 0x0cd0_0000 + k);
        t.set_u32(w(0x216), gf);
        junk(&mut t, class, 0x20, k);
        // O: no children, its mode `omode`
        junk(&mut t, o, 0x400, k);
        t.set_u32(o, class);
        t.write(o + 0x1a, &[omode]);
        t.set_u32(o + 0x10, o);
        floats(&mut t, o + 0x1c, 0xc0, k + 1);
        for i in 0..0x30u32 {
            // the records: frames 0..3 against 3 frames; phases
            let fr = ((i + k) % 4) as u16;
            let ph = (i * 0x0d5 + k * 0x101) as u16;
            t.write(o + 0x31c + 4 * i, &fr.to_le_bytes());
            t.write(o + 0x31e + 4 * i, &ph.to_le_bytes());
        }
        t.set_u32(o + 0x3dc, s);
        setf(&mut t, o + 0x3e4, 30.0);
        t.write(o + 0x3e8, &[c]);
        t.set_u32(o + 0x3ec, q);
        t.set_u32(o + 0x3f0, 0x00c0_ffee ^ k);
        t.set_u32(o + 0x3f4, 0x1234_5678 + k);
        t.set_u32(o + 0x3f8, j as u32);
        // Q: 3 frames, their lengths (one negative: always past)
        junk(&mut t, q, 0x40, k);
        t.write(q + 6, &3u16.to_le_bytes());
        for (i, l) in [0x0180u16, 0x7fff, 0x8000, 0x0100].into_iter().enumerate() {
            t.write(q + 0x12 + 8 * i as u32, &l.to_le_bytes());
        }
        // the strip: at mode 3 (the laser move's walk stops)
        junk(&mut t, s, 0x80, k + 1);
        t.set_u32(s, class);
        t.write(s + 0x1a, &[3]);
        t.set_u32(s + 0x1c, data);
        t.write(s + 0x20, &len.to_le_bytes());
        t.write(s + 0x22, &0x10u16.to_le_bytes());
        t.write(s + 0x26, &window.to_le_bytes());
        t.write(s + 0x28, &start.to_le_bytes());
        floats(&mut t, data - 0x20, 4 * 20, k);
        for i in 0..16 {
            // half widths, some dropping to 0 or below
            let hw = (i + k) as f32 * 0.375 - 1.5;
            setf(&mut t, data + 16 * i + 0xc, hw);
        }
        junk(&mut t, v, 8, k);
        t.set_u32(v, o);
        t.write(v + 5, &[3]);
        ownb::dirty_stack(&mut t, 0x37a0 + k, &[]);
        if k == 6 {
            t.check_entry(t.at(0x46b0), Call::default(), &[v], Eax::Checked);
        } else {
            t.check(f, Call::this(v), &[], Eax::Checked, |g, img, fr| {
                ffx_magic::r50::laser_visit_0387(g, img, fr)
            });
        }
    }
}
