//! List `phyre-anim-left`, batch 6 (M5 stage 1): the weighted and the
//! additive blenders' four-float kernels.
//!
//! Both take `(out, mode, sources, n, weights)`: `sources` is an array of
//! `n` pointers to four floats (null entries skipped), `weights` a
//! `PhyreEngine` relative pointer (`weights + [weights]`) to `n` floats.
//! Mode 0 blends linearly, mode 1 as quaternions, mode 2 picks the source
//! with the largest weight. Every intermediate is an `f32` on the x87 at
//! 53 bits, in the original's operand order; the helpers the additive
//! kernel calls through its frame (`0x0050eb70`, `0x0052fa10`,
//! `0x0052ecb0`) are computed on values here, with the same roundings.

#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

/// `_CIsqrt` (import thunk).
pub const CI_SQRT: u32 = 0x0094_97b8;

/// The channel-target type records `0x00531660` recognises.
pub const ROT_TYPE: u32 = 0x00c1_a924;
/// See [`ROT_TYPE`].
pub const TRANS_TYPE: u32 = 0x00c1_a934;
/// See [`ROT_TYPE`].
pub const SCALE_TYPE: u32 = 0x00c1_a944;
/// `crt_memcpy_thunk` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// The target blender's log forwarder (cdecl `(obj, fmt, ...)`).
pub const LOG: u32 = 0x0053_0b20;
/// `Matrix3(Quat)` (thiscall on the matrix, `(q)`).
pub const QUAT_MATRIX: u32 = 0x0053_0c30;
/// The rows-times-scale (cdecl `(out, m, s)`).
pub const SCALE_ROWS: u32 = 0x0053_1310;
/// The bound matrix's set (thiscall on the target, `(m)`).
pub const SET_MATRIX: u32 = 0x0053_1a40;
/// How many words `0x00531660`'s frame takes.
pub const APPLY_FRAME_WORDS: u32 = 0x124 / 4;

pub mod bridge {
    use super::{
        CI_SQRT, Conv, Host, LOG, MEMCPY, Mem, QUAT_MATRIX, ROT_TYPE, SCALE_ROWS, SCALE_TYPE,
        SET_MATRIX, TRANS_TYPE, x87,
    };

    type Q = [u32; 4];

    fn f(b: u32) -> f64 {
        x87::load(b)
    }

    fn s(x: f64) -> u32 {
        x87::store(x)
    }

    /// Four floats as `fld`/`fstp` copy them.
    fn quat(m: &impl Mem, at: u32) -> Q {
        [0, 4, 8, 0xc].map(|o| s(f(m.read(at.wrapping_add(o)))))
    }

    fn put(m: &mut impl Mem, at: u32, q: Q) {
        for (k, v) in (0u32..).zip(q) {
            m.write(at.wrapping_add(4 * k), v);
        }
    }

    /// The weights' address: a relative pointer.
    fn weights(m: &impl Mem, wref: u32) -> u32 {
        m.read::<u32>(wref).wrapping_add(wref)
    }

    /// `al` set to `bit`; the rest of `eax` is not the callers'.
    fn al(bit: bool) -> u32 {
        u32::from(bit)
    }

    /// Mode 2 of both kernels: the source with the largest weight (the
    /// first on ties, NaN weights never win over a held one).
    fn pick(m: &impl Mem, ptrs: u32, n: u32, w: u32) -> u32 {
        let mut best = 0.0;
        let mut found = 0u32;
        for i in 0..n {
            let p: u32 = m.read(ptrs.wrapping_add(4 * i));
            if p == 0 {
                continue;
            }
            let wi = f(m.read(w.wrapping_add(4 * i)));
            if found == 0 || best < wi {
                best = f(s(wi));
                found = p;
            }
        }
        found
    }

    /// `acc * scale` per component, `f32` each (the last as `scale *
    /// acc`).
    fn scaled(acc: Q, k: f64) -> Q {
        acc.map(|a| s(x87::mul(f(a), k)))
    }

    /// `0x0052dae0` (cdecl `(out, mode, sources, n, weights)`, `al`): the
    /// weighted blender's kernel. Modes 0 and 1 sum `source * weight`
    /// (mode 1 turns a source whose dot product with the sum so far is
    /// negative by negating its weight) and the weights; a total above 0
    /// scales the sum by `f32(1 / total)` and answers 1, else 0 with the
    /// sum as it is. Mode 2 answers the largest-weight source (0 and
    /// zeros for none). Other modes write zeros and answer 0. `out` is
    /// written unless `out` or `sources` is null (0).
    pub fn weighted(m: &mut impl Mem, out: u32, mode: u32, ptrs: u32, n: u32, wref: u32) -> u32 {
        if out == 0 || ptrs == 0 {
            return al(false);
        }
        let w = weights(m, wref);
        let mut acc: Q = [0; 4];
        let mut ok = false;
        match mode {
            0 | 1 => {
                let mut total = 0u32;
                for i in 0..n {
                    let p: u32 = m.read(ptrs.wrapping_add(4 * i));
                    if p == 0 {
                        continue;
                    }
                    let mut wi = s(f(m.read(w.wrapping_add(4 * i))));
                    total = s(x87::add(f(wi), f(total)));
                    let q: Q = [0, 4, 8, 0xc].map(|o| m.read(p.wrapping_add(o)));
                    if mode == 1 {
                        let mut dot = s(x87::mul(f(q[0]), f(acc[0])));
                        dot = s(x87::add(f(dot), x87::mul(f(q[1]), f(acc[1]))));
                        dot = s(x87::add(f(dot), x87::mul(f(q[2]), f(acc[2]))));
                        dot = s(x87::add(f(dot), x87::mul(f(q[3]), f(acc[3]))));
                        if f(dot) < 0.0 {
                            wi = s(-f(wi));
                        }
                    }
                    let prod: Q = [0, 1, 2, 3].map(|k| {
                        if k == 3 {
                            s(x87::mul(f(wi), f(q[k])))
                        } else {
                            s(x87::mul(f(q[k]), f(wi)))
                        }
                    });
                    acc = [0, 1, 2, 3].map(|k| s(x87::add(f(prod[k]), f(acc[k]))));
                }
                if f(total) > 0.0 {
                    let inv = f(s(x87::div(1.0, f(total))));
                    acc = scaled(acc, inv);
                    ok = true;
                }
            }
            2 => {
                let p = pick(m, ptrs, n, w);
                if p != 0 {
                    acc = quat(m, p);
                    ok = true;
                }
            }
            _ => {}
        }
        put(m, out, acc);
        al(ok)
    }

    /// `0x0050eb70`'s lerp on values: `(b - a) * t + a`.
    fn lerp(t: u32, a: Q, b: Q) -> Q {
        let t = f(t);
        let d: Q = [0, 1, 2, 3].map(|i| s(x87::sub(f(b[i]), f(a[i]))));
        [0, 1, 2, 3].map(|i| {
            if i == 3 {
                s(x87::add(f(a[3]), f(s(x87::mul(t, f(d[3]))))))
            } else {
                s(x87::add(f(s(x87::mul(f(d[i]), t))), f(a[i])))
            }
        })
    }

    /// `0x0052fa10`'s normalisation on values.
    fn normalize(h: &mut impl Host, q: Q) -> Q {
        let [x, y, z, w] = q.map(f);
        let mut sum = f(s(x87::mul(x, x)));
        sum = f(s(x87::add(sum, x87::mul(y, y))));
        sum = f(s(x87::add(sum, x87::mul(z, z))));
        sum = f(s(x87::add(sum, x87::mul(w, w))));
        let at = h.rebase(CI_SQRT);
        let len = x87::load(h.call_intrinsic(at, sum).to_bits());
        let inv = f(s(x87::div(1.0, len)));
        q.map(|c| s(x87::mul(f(c), inv)))
    }

    /// `0x0052ecb0`'s product `a * b` on values.
    fn mul(a: Q, b: Q) -> Q {
        let [ax, ay, az, aw] = a.map(f);
        let [bx, by, bz, bw] = b.map(f);
        let (add, sub, mul) = (x87::add, x87::sub, x87::mul);
        [
            sub(add(add(mul(bw, ax), mul(bx, aw)), mul(bz, ay)), mul(by, az)),
            sub(add(add(mul(bw, ay), mul(by, aw)), mul(bx, az)), mul(ax, bz)),
            sub(add(add(mul(bw, az), mul(bz, aw)), mul(ax, by)), mul(bx, ay)),
            sub(sub(sub(mul(bw, aw), mul(bx, ax)), mul(by, ay)), mul(bz, az)),
        ]
        .map(s)
    }

    /// `0x0052ef70` (cdecl `(out, mode, sources, n, weights)`, `al`): the
    /// additive blender's kernel. The first source is the base (zeros, or
    /// the identity quaternion in mode 1, without one); each further
    /// source adds `source * weight` (mode 0), or in mode 1 is lerped from
    /// identity by its weight, normalised to `n`, and the sum becomes
    /// `(n * sum) * conj(n)`. Mode 2 picks as the weighted kernel. `out`
    /// is written only when some source took part (1), else 0.
    pub fn additive(h: &mut impl Host, out: u32, mode: u32, ptrs: u32, n: u32, wref: u32) -> u32 {
        if out == 0 || ptrs == 0 {
            return al(false);
        }
        let w = weights(h, wref);
        if mode == 2 {
            let p = pick(h, ptrs, n, w);
            if p == 0 {
                return al(false);
            }
            let q = quat(h, p);
            put(h, out, q);
            return al(true);
        }
        if mode > 2 {
            return al(false);
        }
        let one = 1.0f32.to_bits();
        let mut acc: Q = if mode == 1 { [0, 0, 0, one] } else { [0; 4] };
        let first: u32 = h.read(ptrs);
        let mut ok = false;
        if first != 0 {
            acc = quat(h, first);
            ok = true;
        }
        for i in 1..n {
            let p: u32 = h.read(ptrs.wrapping_add(4 * i));
            if p == 0 {
                continue;
            }
            ok = true;
            let wi = s(f(h.read(w.wrapping_add(4 * i))));
            if mode == 0 {
                let q: Q = [0, 4, 8, 0xc].map(|o| h.read(p.wrapping_add(o)));
                let prod: Q = [0, 1, 2, 3].map(|k| {
                    if k == 3 {
                        s(x87::mul(f(wi), f(q[k])))
                    } else {
                        s(x87::mul(f(q[k]), f(wi)))
                    }
                });
                acc = [0, 1, 2, 3].map(|k| s(x87::add(f(prod[k]), f(acc[k]))));
            } else {
                let q = quat(h, p);
                let l = lerp(wi, [0, 0, 0, one], q);
                let nq = normalize(h, l);
                let conj = [s(-f(nq[0])), s(-f(nq[1])), s(-f(nq[2])), nq[3]];
                acc = mul(mul(nq, acc), conj);
            }
        }
        if ok {
            put(h, out, acc);
        }
        al(ok)
    }

    /// Where `0x00531660`'s locals sit in its frame (`frame` is its
    /// `ebp - 0x124`).
    mod at {
        pub const VEC: u32 = 0x124 - 0x1c;
        pub const ROT: u32 = 0x124 - 0x40;
        pub const SCALE: u32 = 0x124 - 0x30;
        pub const TRANS: u32 = 0x124 - 0x50;
        pub const ROT_MATRIX: u32 = 0x124 - 0xc4;
        pub const SCALED: u32 = 0x124 - 0xf4;
        pub const SCALE_MATRIX: u32 = 0x124 - 0x94;
        pub const ROT_ONLY: u32 = 0;
    }

    /// `0x00531660` (`thiscall(binds, values)`, `ret 8`): the target
    /// blender's apply: 1 without `binds`. The owner's records (`[[this +
    /// 8] + 0x14]`, 16 bytes, count `+0x10`) are walked in runs of one
    /// target (the record each names by its `+0xc` gives the target at
    /// `+8`); a target whose bind (`[binds + 0x10]`, 24 bytes) is 0 is
    /// skipped. Each record's value (`[values]`, 32 bytes: four floats
    /// and a type) sets the rotation (normalised as `0x0052fa10`), scale
    /// or translation, or for other types goes to the bind: a `memcpy` to
    /// `+4` of `+8` bytes (kind 1) or the log `0x00530b20` (kinds 2, 3).
    /// A kind-1 bind with a rotation and/or scale then gets its 3x3
    /// matrix (`0x00531a40`), and the translation at `+0x30`. The values
    /// persist across targets in the caller's frame (`frame`); 0.
    #[allow(clippy::too_many_lines)]
    pub fn apply_targets(h: &mut impl Host, this: u32, binds: u32, values: u32, frame: u32) -> u32 {
        if binds == 0 {
            return 1;
        }
        for off in [at::ROT, at::TRANS, at::SCALE] {
            for k in 0..4u32 {
                if off != at::ROT && k == 3 {
                    continue;
                }
                h.write(frame.wrapping_add(off + 4 * k), 0u32);
            }
        }
        let vals: u32 = h.read(values);
        let owner: u32 = h.read(this.wrapping_add(8));
        let n = h.read::<u32>(owner.wrapping_add(0x10)) & !0x8000_0000;
        let recs: u32 = h.read(owner.wrapping_add(0x14));
        let end = recs.wrapping_add(n << 4);
        let target_of = |m: &dyn Fn(u32) -> u32, cur: u32| {
            let idx = m(cur.wrapping_add(0xc));
            m(recs.wrapping_add(idx << 4).wrapping_add(8))
        };
        let (rot_t, trans_t, scale_t) = (
            h.rebase(ROT_TYPE),
            h.rebase(TRANS_TYPE),
            h.rebase(SCALE_TYPE),
        );
        let vec = frame.wrapping_add(at::VEC);
        let mut cur = recs;
        while cur < end {
            let rd = |a: u32| -> u32 { h.read(a) };
            let tgt = target_of(&rd, cur);
            let table: u32 = h.read(binds.wrapping_add(0x10));
            let entry = table.wrapping_add(tgt.wrapping_mul(24));
            if h.read::<u32>(entry) == 0 {
                loop {
                    cur = cur.wrapping_add(16);
                    if cur >= end {
                        return 0;
                    }
                    let rd = |a: u32| -> u32 { h.read(a) };
                    if target_of(&rd, cur) != tgt {
                        break;
                    }
                }
                continue;
            }
            let mut flags = 0u32;
            loop {
                let idx: u32 = h.read(cur.wrapping_add(0xc));
                let v = vals.wrapping_add(idx << 5);
                for k in 0..4u32 {
                    let w: u32 = h.read(v.wrapping_add(4 * k));
                    h.write(vec.wrapping_add(4 * k), w);
                }
                let ty: u32 = h.read(v.wrapping_add(0x10));
                let q: Q = [0, 4, 8, 0xc].map(|o| h.read(vec.wrapping_add(o)));
                if ty == rot_t {
                    let r = normalize(h, q);
                    put(h, frame.wrapping_add(at::ROT), r);
                    flags |= 1;
                } else if ty == trans_t {
                    for (k, c) in (0u32..3).zip(q) {
                        h.write(frame.wrapping_add(at::TRANS + 4 * k), s(f(c)));
                    }
                    flags |= 4;
                } else if ty == scale_t {
                    for (k, c) in (0u32..3).zip(q) {
                        h.write(frame.wrapping_add(at::SCALE + 4 * k), s(f(c)));
                    }
                    flags |= 2;
                } else if ty != 0 {
                    let kind: u32 = h.read(entry);
                    let word = |k: u32| -> u32 { h.read(entry.wrapping_add(k)) };
                    match kind {
                        1 => {
                            let (dst, len) = (word(4), word(8));
                            h.call_sym(MEMCPY, Conv::Cdecl, &[dst, vec, len]);
                        }
                        2 => {
                            let (obj, fmt) = (word(0x10), word(0xc));
                            h.call_sym(LOG, Conv::Cdecl, &[obj, fmt, vec]);
                        }
                        3 => {
                            let obj = word(0x14);
                            h.call_sym(LOG, Conv::Cdecl, &[obj, 0, vec]);
                        }
                        _ => {}
                    }
                }
                cur = cur.wrapping_add(16);
                if cur >= end {
                    break;
                }
                let rd = |a: u32| -> u32 { h.read(a) };
                if target_of(&rd, cur) != tgt {
                    break;
                }
            }
            if flags & 7 != 0 && h.read::<u32>(entry) == 1 {
                let dst: u32 = h.read(entry.wrapping_add(4));
                let rot = frame.wrapping_add(at::ROT);
                let matrix = match flags & 3 {
                    3 => {
                        let m1 = frame.wrapping_add(at::ROT_MATRIX);
                        let m1 = h.call_sym(QUAT_MATRIX, Conv::Thiscall(m1), &[rot]);
                        let o = frame.wrapping_add(at::SCALED);
                        let sc = frame.wrapping_add(at::SCALE);
                        Some(h.call_sym(SCALE_ROWS, Conv::Cdecl, &[o, m1, sc]))
                    }
                    2 => {
                        let m = frame.wrapping_add(at::SCALE_MATRIX);
                        let sc = frame.wrapping_add(at::SCALE);
                        let sv: [u32; 3] = [0, 4, 8].map(|o| s(f(h.read(sc.wrapping_add(o)))));
                        for (off, v) in [
                            (0u32, sv[0]),
                            (4, 0),
                            (8, 0),
                            (0x10, 0),
                            (0x14, sv[1]),
                            (0x18, 0),
                            (0x20, 0),
                            (0x24, 0),
                            (0x28, sv[2]),
                        ] {
                            h.write(m.wrapping_add(off), v);
                        }
                        Some(m)
                    }
                    1 => {
                        let m = frame.wrapping_add(at::ROT_ONLY);
                        Some(h.call_sym(QUAT_MATRIX, Conv::Thiscall(m), &[rot]))
                    }
                    _ => None,
                };
                if let Some(m) = matrix {
                    h.call_sym(SET_MATRIX, Conv::Thiscall(dst), &[m]);
                }
                if flags & 4 != 0 {
                    for k in 0..3u32 {
                        let t: u32 = h.read(frame.wrapping_add(at::TRANS + 4 * k));
                        h.write(dst.wrapping_add(0x30 + 4 * k), s(f(t)));
                    }
                }
            }
        }
        0
    }
}
