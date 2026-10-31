//! `magic_0018.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::l5_objhost::list_remove;
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The x87 indefinite an `fstp` of an empty stack stores (a call-out
/// that left no float).
const INDEFINITE: f32 = f32::from_bits(0xffc0_0000);

/// `f32` of a slot's ST0 (`fstp dword`).
#[must_use]
pub fn st0_f32(st0: Option<f64>) -> f32 {
    st0.map_or(INDEFINITE, |v| v as f32)
}

/// The angle wrap of [`bounce_step`]: `v - T` above `HI`, `v + T` below
/// `LO` (`comiss` + `jbe`: an unordered `v` stays).
#[must_use]
pub fn wrap(v: f32, hi: f32, lo: f32, t: f32) -> f32 {
    if v > hi {
        sub(v, t)
    } else if lo > v {
        add(v, t)
    } else {
        v
    }
}

/// A task step (stored at 0x100023f5, `(task)`, the task unused): moves
/// the 24 debris records of `G_REC + 0` (0x44 bytes each, operand
/// `+0x24`) that have a count at `+0x3c`. Mode `+0x40` 1 counts down: at
/// 0 the record's object (`+0x20`) leaves the list `LIST` (operand
/// `+0x53`, [`list_remove`]) and, when it was there, goes to slot 713
/// with `*G_P = ITEM` and bit 15 of `obj + 0x124` cleared; otherwise the
/// object's `+0x76` gets `(count << 12) / 20` (0x1000 in other modes).
/// Then the object's rotation `+0xc0..` turns by `+0x0..` (wrapped), its
/// position `+0xd0..` moves by the velocity `+0x10..` (y also by the
/// fall `+0x24 += +0x28`), x and z are damped by `+0x30`; at or under
/// the floor `+0x2c` it bounces: the overshoot taken back along x/z, y
/// set to the floor, the spin `+0x38` (when above `EPS`) turned into
/// velocity through slot 314 and damped, three new rotation speeds from
/// slot 314 `(K)`, y velocity `-bounce * dy` (the last step's), everything stopped when
/// that is under the fall's pull, and mode 1. Returns `eax` as the last
/// record left it.
#[allow(clippy::too_many_lines)]
pub fn bounce_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let abs_mask = g.read::<u64>(w(0x0d));
    let abs = |x: f32| f64::from_bits(f64::from(x).to_bits() & abs_mask);
    let (hi, t, lo) = (k(g, 0x15), k(g, 0x1d), k(g, 0xdd));
    let mut eax = fr.eax;
    let mut esi = g.read::<u32>(w(0x24)).wrapping_add(0x3c);
    for _ in 0..0x18 {
        let r = esi - 0x3c;
        esi = esi.wrapping_add(0x44);
        let n = g.read::<u32>(r + 0x3c);
        if n == 0 {
            continue;
        }
        let obj = g.read::<u32>(r + 0x20);
        let tilt = if g.read::<u32>(r + 0x40) == 1 {
            let n = n.wrapping_sub(1);
            g.write::<u32>(r + 0x3c, n);
            if n == 0 {
                eax = list_remove(g, img, img.rel32_target(f + 0x58), w(0x53), obj);
                if eax != 0 {
                    let p = g.read::<u32>(w(0x61));
                    g.write::<u32>(p, w(0x68));
                    let s713 = slot_target(g, w(0x6d), 713);
                    g.call(s713, w(0x53), &[obj]);
                    eax = 0x7fff;
                    let v = g.read::<u16>(obj + 0x124) & 0x7fff;
                    g.write::<u16>(obj + 0x124, v);
                }
                continue;
            }
            // imul by 0x66666667, sar 3, plus the sign bit: / 20
            let q = (n << 12).cast_signed() / 20;
            u32::from(q.cast_unsigned() as u16)
        } else {
            0x1000
        };
        eax = tilt;
        g.write::<u16>(obj + 0x76, tilt as u16);
        for i in 0..3u32 {
            let v = add(g.read::<f32>(obj + 0xc0 + 4 * i), g.read::<f32>(r + 4 * i));
            g.write::<f32>(obj + 0xc0 + 4 * i, wrap(v, hi, lo, t));
        }
        let fall = add(g.read::<f32>(r + 0x24), g.read::<f32>(r + 0x28));
        g.write::<f32>(r + 0x24, fall);
        let x = add(g.read::<f32>(obj + 0xd0), g.read::<f32>(r + 0x10));
        g.write::<f32>(obj + 0xd0, x);
        let dy = add(fall, g.read::<f32>(r + 0x14));
        let y = add(g.read::<f32>(obj + 0xd4), dy);
        g.write::<f32>(obj + 0xd4, y);
        let z = add(g.read::<f32>(r + 0x18), g.read::<f32>(obj + 0xd8));
        g.write::<f32>(obj + 0xd8, z);
        let damp = g.read::<f32>(r + 0x30);
        let vx = mul(damp, g.read::<f32>(r + 0x10));
        g.write::<f32>(r + 0x10, vx);
        let vz = mul(g.read::<f32>(r + 0x18), damp);
        g.write::<f32>(r + 0x18, vz);
        let y = g.read::<f32>(obj + 0xd4);
        let floor = g.read::<f32>(r + 0x2c);
        // comiss + jb: above the floor (or unordered) goes on falling
        if y < floor || y.is_nan() || floor.is_nan() {
            continue;
        }
        if g.read::<f32>(r + 0x14) != 0.0 && dy != 0.0 {
            let q = abs(sub(y, floor)) / abs(dy);
            let back = (f64::from_bits(g.read::<u64>(w(0x21d))) - q) as f32;
            let x = sub(
                g.read::<f32>(obj + 0xd0),
                mul(back, g.read::<f32>(r + 0x10)),
            );
            g.write::<f32>(obj + 0xd0, x);
            let z = sub(
                g.read::<f32>(obj + 0xd8),
                mul(g.read::<f32>(r + 0x18), back),
            );
            g.write::<f32>(obj + 0xd8, z);
        }
        eax = g.read::<u32>(r + 0x2c);
        g.write::<u32>(obj + 0xd4, eax);
        let spin = g.read::<f32>(r + 0x38);
        let ecx = n;
        if abs(spin) > f64::from_bits(g.read::<u64>(w(0x27b))) {
            let s314 = slot_target(g, w(0x282), 314);
            let o = g.call(s314, ecx, &[spin.to_bits()]);
            let vx = add(st0_f32(o.st0), g.read::<f32>(r + 0x10));
            g.write::<f32>(r + 0x10, vx);
            let s314 = slot_target(g, w(0x2a4), 314);
            let spin = g.read::<u32>(r + 0x38);
            let o = g.call(s314, ecx, &[spin]);
            let vz = add(g.read::<f32>(r + 0x18), st0_f32(o.st0));
            g.write::<f32>(r + 0x18, vz);
            let spin = mul(g.read::<f32>(r + 0x38), k(g, 0x2df));
            g.write::<f32>(r + 0x38, spin);
        }
        for (i, op) in (0u32..).zip([0x2e9u32, 0x301, 0x318]) {
            let s314 = slot_target(g, w(op), 314);
            let o = g.call(s314, ecx, &[0x3ef1_463b]);
            eax = o.eax;
            g.write::<f32>(r + 4 * i, st0_f32(o.st0));
        }
        let neg = g.read::<u32>(w(0x347));
        let vy = f32::from_bits(mul(g.read::<f32>(r + 0x34), dy).to_bits() ^ neg);
        g.write::<f32>(r + 0x14, vy);
        if f64::from(g.read::<f32>(r + 0x28)) > abs(vy) {
            let c = k(g, 0x372);
            let vx = mul(g.read::<f32>(r + 0x10), c);
            g.write::<u32>(r + 0x14, 0);
            g.write::<f32>(r + 0x10, vx);
            let vz = mul(g.read::<f32>(r + 0x18), k(g, 0x38b));
            g.write::<f32>(r + 0x18, vz);
            g.write::<u32>(r + 0x28, 0);
        }
        g.write::<u32>(r + 0x24, 0);
        g.write::<u32>(r + 0x40, 1);
    }
    Ret::eax(eax)
}

/// 0x10001ed0 (971 bytes, 1 overlay).
pub const BOUNCE_STEP: Sig = Sig {
    len: 971,
    hash: 0xcfb7_324f_4a31_7177,
    calls: &[(0x58, Callee::Sig(&crate::l5_objhost::LIST_REMOVE))],
    imports: &[],
};

/// One node off the task pool inline (the two takes of [`start`]): with
/// free nodes (the u16 count at operand `count`), the head (`head_r`)
/// leaves the free ring, the head moves to its next (`head_w`, 0 for the
/// last node), the node is reset (links, `+0xc`, `+0x14` 0, `+0x8`
/// itself, `+0x10` the callback immediate at `cb`, byte `+0x18` 0) and
/// the count drops (`count_w`). With none, the original goes on with node
/// 0 (and faults at its first store). Returns the node and the head the
/// code keeps in a register (the new head, or the old one when empty).
#[allow(clippy::too_many_arguments)]
fn take_inline<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    count: &mut u16,
    node: Option<u32>,
    head_w: (u32, u32),
    cb: u32,
    count_w: u32,
) -> (u32, u32) {
    let w = |off: u32| img.word(f + off);
    let Some(n) = node.filter(|_| *count != 0) else {
        return (0, node.unwrap_or(0));
    };
    let prev = g.read::<u32>(n);
    g.write::<u32>(prev + 4, g.read::<u32>(n + 4));
    let next = g.read::<u32>(n + 4);
    g.write::<u32>(next, g.read::<u32>(n));
    // one store of the new head, through the operand of its branch
    let (head, at) = if *count > 1 {
        (g.read::<u32>(n + 4), head_w.0)
    } else {
        (0, head_w.1)
    };
    *count = count.wrapping_sub(1);
    g.write::<u32>(w(at), head);
    for off in [0, 4, 0xc, 0x14] {
        g.write::<u32>(n + off, 0);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 0x10, w(cb));
    g.write::<u8>(n + 0x18, 0);
    g.write::<u16>(w(count_w), *count);
    (n, head)
}

/// The start-up (`magic_0018` 0x100022a0 and `magic_0019` 0x10004620,
/// no arguments; the two differ in the record table's field and stride,
/// read from the code): four pointer globals set (`+0x1a`.. pairs),
/// slot 719 `(NAME, 8, 0)` (`+0x14`) into `+0x46`, the part's set-up
/// ([`part_init`](crate::l5_objhost::part_init)), the field `+imm8 +0x59`
/// of 0x18 records (stride `+0x68`) from `*T` (`+0x50`) cleared; the
/// motion block (`+0x72` = `*C`, `+0x6d`; `+0x7e` the immediate `+0x82`,
/// `+0x88` -1) gets slot 215 or, when `*F` (`+0x92`) is set, slot 413 of
/// `*C` into `+0xb8`, `+0xbe` -1, `+0xc8` 0, `+0xd2` 1 and the u16 `+0xdc`
/// 0 (or 1 at `+0xe5` with `*F` set); the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)); then two task nodes
/// off the pool (count `+0xf4`, head `+0x10e`), the callbacks `+0x158`
/// and `+0x1fb`, the second with `+0x14` = `table[*C]` (`+0x20d`) and
/// byte `+0x18` 2, appended to the task list (first `+0x169`, last
/// `+0x179`/`+0x22f`). Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s719 = slot_target(g, w(0x05), 0xb3c / 4);
    for (dst, v) in [(0x1a, 0x1e), (0x24, 0x28), (0x2e, 0x32), (0x38, 0x3c)] {
        g.write::<u32>(w(dst), w(v));
    }
    let h = g.call(s719, fr.ecx, &[w(0x14), 8, 0]).eax;
    g.write::<u32>(w(0x46), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x4b));
    let first = crate::o4_parts::disp8(img, f + 0x59);
    let stride = crate::o4_parts::disp8(img, f + 0x68);
    let mut at = g.read::<u32>(w(0x50)).wrapping_add(first);
    for _ in 0..w(0x5b) {
        g.write::<u32>(at, 0);
        at = at.wrapping_add(stride);
    }
    let c = g.read::<u32>(w(0x6d));
    g.write::<u32>(w(0x72), c);
    g.write::<u32>(w(0x7e), w(0x82));
    g.write::<u32>(w(0x88), u32::MAX);
    let slot = if g.read::<u32>(w(0x92)) == 0 {
        0x35c / 4
    } else {
        0x674 / 4
    };
    let s = slot_target(g, w(0x78), slot);
    let m = g.call(s, 0, &[c]).eax;
    g.write::<u32>(w(0xb8), m);
    g.write::<u32>(w(0xbe), u32::MAX);
    g.write::<u32>(w(0xc8), 0);
    g.write::<u32>(w(0xd2), 1);
    if g.read::<u32>(w(0xad)) == 0 {
        g.write::<u16>(w(0xdc), 0);
    } else {
        g.write::<u16>(w(0xe5), 1);
    }
    crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0xed));
    let mut count = g.read::<u16>(w(0xf4));
    let n1 = (count != 0).then(|| g.read::<u32>(w(0x10e)));
    let (n1, head) = if n1.is_some() {
        take_inline(g, img, f, &mut count, n1, (0x133, 0x133), 0x158, 0x163)
    } else {
        // no free node: the head is read and node 0 goes on
        (0, g.read::<u32>(w(0x104)))
    };
    let mut first = g.read::<u32>(w(0x169));
    g.write::<u32>(n1 + 4, 0);
    if first == 0 {
        first = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x18e), first);
    } else {
        let last = g.read::<u32>(w(0x179));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    let idx = g.read::<u32>(w(0x196));
    let (n2, _) = take_inline(
        g,
        img,
        f,
        &mut count,
        Some(head),
        (0x1bf, 0x1c7),
        0x1fb,
        0x206,
    );
    let v = g.read::<u32>(w(0x20d).wrapping_add(idx.wrapping_mul(4)));
    g.write::<u32>(n2 + 0x14, v);
    g.write::<u8>(n2 + 0x18, 2);
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x244), n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x24d), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x22f), n2);
    }
    Ret::eax(0)
}

const START_CALLS: &[(u32, Callee)] = &[
    (0x4b, Callee::Sig(&crate::l5_objhost::PART_INIT)),
    (0xed, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
];

/// [`start`] in `magic_0018` (600 bytes).
pub const START_0018: Sig = Sig {
    len: 600,
    hash: 0xe6eb_bf57_171f_1c79,
    calls: START_CALLS,
    imports: &[],
};

/// [`start`] in `magic_0019` (600 bytes).
pub const START_0019: Sig = Sig {
    len: 600,
    hash: 0xaebf_7ea4_7361_0499,
    calls: START_CALLS,
    imports: &[],
};
