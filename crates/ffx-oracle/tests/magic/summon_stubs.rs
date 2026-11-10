//! The summon tasks' test stubs and memory helpers (from round 33's
//! `magic_r33.rs`; shared with `magic_r35.rs`).

#![allow(dead_code)]

use crate::magic::{SCRATCH, StubMem, Twin};
use crate::ownb::rec;

pub fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

pub fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

/// `n` bytes of junk at `at` (varied by `k`).
pub fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Every slot answering `first word ^ (slot << 20)`.
pub fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |_, a| a.first().copied().unwrap_or(0) ^ (s << 20));
    }
}

/// Junk floats at the globals the operands `ops` of `f` name.
pub fn junk_floats(t: &mut Twin, f: u32, ops: &[u32], k: u32) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (2.5f32 + o as f32 / 512.0 + k as f32).to_bits());
    }
}

/// The matrix and vector slots the summon tasks call, with results
/// written: 229 `out = row0 + row3 + 2 v`; 327 a matrix; 262 a place
/// from the object word; 366 a vector; 14 three floats; 601 a float; 290
/// four floats; 32 (the bones' function table) four floats.
pub fn summon_stubs(t: &mut Twin) {
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = rf(m, a[1] + 4 * i) + rf(m, a[1] + 0x30 + 4 * i) + 2.0 * rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 327, 2, |m, a| {
        for i in 0..16 {
            wf(
                m,
                a[0] + 4 * i,
                if i % 5 == 0 { 1.0 } else { 0.125 * i as f32 },
            );
        }
        0x327
    });
    rec(t, 262, 4, |m, a| {
        let k = (a[0] & 0xff) as f32;
        wf(m, a[1], 2.0 * (k - 2.0));
        wf(m, a[2], 0.5 * k);
        wf(m, a[3], 3.0 - k);
        0x262
    });
    rec(t, 366, 2, |m, a| {
        for i in 0..3 {
            wf(m, a[1] + 4 * i, 0.75 * i as f32 - (a[0] & 7) as f32);
        }
        0x366
    });
    rec(t, 14, 4, |m, a| {
        for i in 1..4 {
            wf(m, a[i], 0.25 * i as f32);
        }
        0x14
    });
    rec(t, 601, 2, |m, a| {
        wf(m, a[1], 0.25 * (a[0] & 0xf) as f32 - 1.0);
        0x601
    });
    rec(t, 290, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 1.5 - i as f32);
        }
        0x290
    });
    rec(t, 32, 2, |m, a| {
        for i in 0..4 {
            wf(m, a[1] + 4 * i, 0.5 * i as f32 - 1.0);
        }
        0x32
    });
}

/// The float slots of the matrices: 235 a unit matrix; 330/328/329 `out =
/// in + angle (i + 1) / 8`; the scale slots 15/16/17/277 and the random
/// 404/314 floats in ST0 (as `magic_r31.rs`).
pub fn float_matrix_stubs(t: &mut Twin) {
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            let ang = f32::from_bits(a[2]);
            for i in 0..16 {
                let v = rf(m, a[1] + 4 * i) + ang * (i + 1) as f32 / 8.0 + s as f32 / 1024.0;
                wf(m, a[0] + 4 * i, v);
            }
            s
        });
    }
    for (s, x) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0), (277, 40.0)] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| x + f64::from(a[0] & 3));
    }
    for (s, x) in [(404u32, 0.0625f64), (314, -0.125)] {
        rec(t, s, 1, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            m.w32(SCRATCH + 0xa00, k + 1);
            (s << 16) ^ a[0]
        });
        t.stub_st0(s, move |m, a| {
            x * f64::from(m.r32(SCRATCH + 0xa00) % 5) + f64::from(f32::from_bits(a[0]))
        });
    }
}

/// The vector slots with their results written: 232 `out = v / 2`, 233
/// `out = v s`, 234 `out = a - b`, 266 the cross product, 230 `a += b`.
pub fn vector_stubs(t: &mut Twin) {
    rec(t, 232, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * 0.5;
            wf(m, a[0] + 4 * i, x);
        }
        0x232
    });
    rec(t, 233, 3, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) * s;
            wf(m, a[0] + 4 * i, x);
        }
        0x233
    });
    rec(t, 234, 3, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[1] + 4 * i) - rf(m, a[2] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x234
    });
    rec(t, 266, 3, |m, a| {
        let u = [0, 1, 2].map(|i| rf(m, a[1] + 4 * i));
        let v = [0, 1, 2].map(|i| rf(m, a[2] + 4 * i));
        let c = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for (i, x) in (0u32..).zip(c) {
            wf(m, a[0] + 4 * i, x);
        }
        0x266
    });
    rec(t, 230, 2, |m, a| {
        for i in 0..4 {
            let x = rf(m, a[0] + 4 * i) + rf(m, a[1] + 4 * i);
            wf(m, a[0] + 4 * i, x);
        }
        0x230
    });
}

/// The child visit stub (slot `visit`): logs the cursor's node and mark.
pub fn visit_stub(t: &mut Twin, visit: u32, class: u32) {
    t.stub(visit, move |m, a| {
        let k = m.r32(SCRATCH + 0x3800);
        m.w32(SCRATCH + 0x3800, k + 1);
        let cur = m.r32(a[0]);
        m.w32(SCRATCH + 0x3810 + 8 * (k % 8), cur);
        m.w32(
            SCRATCH + 0x3814 + 8 * (k % 8),
            (m.r32(a[0] + 4) >> 8) & 0xff,
        );
        0x1000 + k
    });
    t.set_u32(class + 0x1c, crate::magic::TRAPS + 16 * visit);
}

/// The DLL globals the function at `f` and its callees name (relocated
/// operands outside the code) that hold 0 get blocks of their own whose
/// every word points at the block: an empty list ring, a node with itself
/// as neighbours (as `magic_pf_throws.rs`).
pub fn prime(t: &mut Twin, f: u32, mut next: u32) {
    let mut todo = vec![(f, 0)];
    let mut seen = Vec::new();
    while let Some((at, depth)) = todo.pop() {
        if seen.contains(&at) {
            continue;
        }
        seen.push(at);
        let Some(len) = ffx_magic::INTERNAL
            .iter()
            .chain(ffx_magic::PORTS)
            .find(|(_, sig)| t.img.matches(at, sig))
            .map(|(_, sig)| sig.len)
        else {
            continue;
        };
        for v in t.img.operands(at, len) {
            if t.img.in_text(v)
                || !(t.img.base..t.img.base + 0x0100_0000).contains(&v)
                || t.u32(v) != 0
            {
                continue;
            }
            for i in 0..0x40 {
                t.set_u32(next + 4 * i, next);
            }
            t.set_u32(v, next);
            next += 0x100;
        }
        if depth < 3 {
            for p in at..at + len.saturating_sub(4) {
                if t.img.word(p) as u8 == 0xe8 {
                    let c = t.img.rel32_target(p + 1);
                    if t.img.in_text(c) {
                        todo.push((c, depth + 1));
                    }
                }
            }
        }
    }
}
