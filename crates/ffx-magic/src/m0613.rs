//! `magic_0613.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Starts the DLL's static task `T` (0x10009500, internal: `edx` = an
/// effect object): `T`'s vtable, callback and own word (the operand
/// pairs at `+0x0a`, `+0x14`, `+0x1e`) set, linked at the end of the task
/// ring (the last word `+0x04`/`+0x27`, the sentinel `+0x31`, the flag
/// word `+0x3e` bit 0 cleared, the last word written `+0x4a`); `T`'s
/// object word `+0x54` = the object, which gets the value 0xbc
/// ([`set_value`](crate::m0042::set_value)); `+0x5f` and `+0x69` 0.
/// Returns the set-value's `eax`.
pub fn task_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.edx;
    let tail = g.read::<u32>(w(0x4));
    for (at, v) in [(0xa, 0xe), (0x14, 0x18), (0x1e, 0x22)] {
        g.write::<u32>(w(at), w(v));
    }
    g.write::<u32>(w(0x27), tail);
    g.write::<u32>(w(0x2d), w(0x31));
    g.write::<u32>(tail.wrapping_add(4), w(0x38));
    let fl = g.read::<u32>(w(0x3e));
    g.write::<u32>(w(0x3e), fl & !1);
    g.write::<u32>(w(0x4a), w(0x4e));
    g.write::<u32>(w(0x54), obj);
    let eax = crate::m0042::set_value(g, img, img.rel32_target(f + 0x59), obj, 0xbc);
    g.write::<u32>(w(0x5f), 0);
    g.write::<u32>(w(0x69), 0);
    eax
}

/// [`task_start`] (115 bytes, 1 overlay).
pub const TASK_START: Sig = Sig {
    len: 115,
    hash: 0x2f61_85b0_354c_960a,
    calls: &[(0x59, Callee::Sig(&crate::m0042::SET_VALUE))],
    imports: &[],
};

/// The centre of the parts (0x100074e0, internal: `ecx` = a record
/// `[obj, out, src]`; a frame with a stack cookie): the base `b` at
/// `ebp - 0x24` (slot 325 `(b, src)`); for each of the object's
/// `+0x162c` parts (signed; 0x274 bytes each from `+0x110`), slot 268
/// `(part[0], 0x11, v)` gives a position at `v = ebp - 0x14`, which
/// becomes `v - b` (`v.w` 1.0) and adds to the sum. `out` = `b + sum *
/// (K / count)` (operand `+0xe9`, `+0x31` without parts), `out.w` 1.0.
/// Returns `out`.
pub fn centre<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (b, v) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let rec = fr.ecx;
    let obj = g.read::<u32>(rec);
    let src = g.read::<u32>(rec + 8);
    let t = slot_target(g, w(0x21), 325);
    g.call(t, 0, &[b, src]);
    let mut k = g.read::<f32>(w(0x31));
    let mut sum = [0.0f32; 3];
    let mut i = 0i32;
    if 0 < g.read::<i32>(obj + 0x162c) {
        let mut part = obj + 0x110;
        loop {
            let p = g.read::<u32>(part);
            let t = slot_target(g, w(0x6a), 268);
            g.call(t, 0, &[p, 0x11, v]);
            let d: [f32; 3] = std::array::from_fn(|j| {
                let j = 4 * u32::try_from(j).unwrap_or(0);
                sub(g.read::<f32>(v + j), g.read::<f32>(b + j))
            });
            i += 1;
            for j in [2usize, 1, 0] {
                g.write::<f32>(v + 4 * u32::try_from(j).unwrap_or(0), d[j]);
                sum[j] = add(d[j], sum[j]);
            }
            g.write::<u32>(v + 0xc, ONE);
            part += 0x274;
            if i >= g.read::<i32>(obj + 0x162c) {
                break;
            }
        }
        k = g.read::<f32>(w(0xe9));
    }
    let n = cvt(g.read::<u32>(obj + 0x162c));
    let k = div(k, n);
    let out = g.read::<u32>(rec + 4);
    let x = add(g.read::<f32>(b), mul(sum[0], k));
    g.write::<f32>(out, x);
    let y = add(g.read::<f32>(b + 4), mul(sum[1], k));
    g.write::<f32>(out + 4, y);
    let z = add(g.read::<f32>(b + 8), mul(sum[2], k));
    g.write::<f32>(out + 8, z);
    g.write::<u32>(out + 0xc, ONE);
    out
}

/// [`centre`] (349 bytes, 1 overlay).
pub const CENTRE: Sig = Sig {
    len: 349,
    hash: 0x64be_6599_d5ad_4e8d,
    // __security_check_cookie
    calls: &[(0x155, Callee::Any)],
    imports: &[],
};
