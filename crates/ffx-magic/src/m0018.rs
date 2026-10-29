//! `magic_0018.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::l5_objhost::list_remove;
use crate::runtime::slot_target;
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
        v - t
    } else if lo > v {
        v + t
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
            let v = g.read::<f32>(obj + 0xc0 + 4 * i) + g.read::<f32>(r + 4 * i);
            g.write::<f32>(obj + 0xc0 + 4 * i, wrap(v, hi, lo, t));
        }
        let fall = g.read::<f32>(r + 0x24) + g.read::<f32>(r + 0x28);
        g.write::<f32>(r + 0x24, fall);
        let x = g.read::<f32>(obj + 0xd0) + g.read::<f32>(r + 0x10);
        g.write::<f32>(obj + 0xd0, x);
        let dy = fall + g.read::<f32>(r + 0x14);
        let y = g.read::<f32>(obj + 0xd4) + dy;
        g.write::<f32>(obj + 0xd4, y);
        let z = g.read::<f32>(r + 0x18) + g.read::<f32>(obj + 0xd8);
        g.write::<f32>(obj + 0xd8, z);
        let damp = g.read::<f32>(r + 0x30);
        let vx = damp * g.read::<f32>(r + 0x10);
        g.write::<f32>(r + 0x10, vx);
        let vz = g.read::<f32>(r + 0x18) * damp;
        g.write::<f32>(r + 0x18, vz);
        let y = g.read::<f32>(obj + 0xd4);
        let floor = g.read::<f32>(r + 0x2c);
        // comiss + jb: above the floor (or unordered) goes on falling
        if y < floor || y.is_nan() || floor.is_nan() {
            continue;
        }
        if g.read::<f32>(r + 0x14) != 0.0 && dy != 0.0 {
            let q = abs(y - floor) / abs(dy);
            let back = (f64::from_bits(g.read::<u64>(w(0x21d))) - q) as f32;
            let x = g.read::<f32>(obj + 0xd0) - back * g.read::<f32>(r + 0x10);
            g.write::<f32>(obj + 0xd0, x);
            let z = g.read::<f32>(obj + 0xd8) - g.read::<f32>(r + 0x18) * back;
            g.write::<f32>(obj + 0xd8, z);
        }
        eax = g.read::<u32>(r + 0x2c);
        g.write::<u32>(obj + 0xd4, eax);
        let spin = g.read::<f32>(r + 0x38);
        let ecx = n;
        if abs(spin) > f64::from_bits(g.read::<u64>(w(0x27b))) {
            let s314 = slot_target(g, w(0x282), 314);
            let o = g.call(s314, ecx, &[spin.to_bits()]);
            let vx = st0_f32(o.st0) + g.read::<f32>(r + 0x10);
            g.write::<f32>(r + 0x10, vx);
            let s314 = slot_target(g, w(0x2a4), 314);
            let spin = g.read::<u32>(r + 0x38);
            let o = g.call(s314, ecx, &[spin]);
            let vz = g.read::<f32>(r + 0x18) + st0_f32(o.st0);
            g.write::<f32>(r + 0x18, vz);
            let spin = g.read::<f32>(r + 0x38) * k(g, 0x2df);
            g.write::<f32>(r + 0x38, spin);
        }
        for (i, op) in (0u32..).zip([0x2e9u32, 0x301, 0x318]) {
            let s314 = slot_target(g, w(op), 314);
            let o = g.call(s314, ecx, &[0x3ef1_463b]);
            eax = o.eax;
            g.write::<f32>(r + 4 * i, st0_f32(o.st0));
        }
        let neg = g.read::<u32>(w(0x347));
        let vy = f32::from_bits((g.read::<f32>(r + 0x34) * dy).to_bits() ^ neg);
        g.write::<f32>(r + 0x14, vy);
        if f64::from(g.read::<f32>(r + 0x28)) > abs(vy) {
            let c = k(g, 0x372);
            let vx = g.read::<f32>(r + 0x10) * c;
            g.write::<u32>(r + 0x14, 0);
            g.write::<f32>(r + 0x10, vx);
            let vz = g.read::<f32>(r + 0x18) * k(g, 0x38b);
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
