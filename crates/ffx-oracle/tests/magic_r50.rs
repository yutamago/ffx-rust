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
