//! `magic_0019.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::l5_objhost::list_remove;
use crate::m0018::wrap;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (stored at 0x10004775, `(task)`, the task unused): moves
/// the 24 debris records of `G_REC + 0` (0x34 bytes each, operand
/// `+0x34`) that have a count at `+0x2c`. While the delay `+0x30` runs
/// the object (`+0x20`) gets `+0x76 = 0x1000`; after it the count goes
/// down: at 0 the object leaves the list `LIST` (operand `+0x61`,
/// [`list_remove`]) and, when it was there, goes to slot 713 with `*G_P
/// = ITEM` and bit 15 of `obj + 0x124` cleared; otherwise `+0x76 =
/// (count << 12) / 20`. Then the object's rotation `+0xc0..` turns by
/// `+0x0..` (wrapped), its position `+0xd0..` moves by the velocity
/// `+0x10..` (y also by `+0x24`), the velocity is damped by `+0x28` and
/// each rotation speed above `EPS` by `K`. Returns `eax` as the last
/// record left it.
pub fn spin_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let abs_mask = g.read::<u64>(w(0x0d));
    let abs = |x: f32| f64::from_bits(f64::from(x).to_bits() & abs_mask);
    let (hi, t, eps, slow, lo) = (
        k(g, 0x15),
        k(g, 0x1d),
        f64::from_bits(g.read::<u64>(w(0x25))),
        k(g, 0x2d),
        k(g, 0x100),
    );
    let mut eax = fr.eax;
    let mut esi = g.read::<u32>(w(0x34)).wrapping_add(0x2c);
    for _ in 0..0x18 {
        let r = esi - 0x2c;
        esi = esi.wrapping_add(0x34);
        let n = g.read::<u32>(r + 0x2c);
        if n == 0 {
            continue;
        }
        let delay = g.read::<u32>(r + 0x30);
        let obj = g.read::<u32>(r + 0x20);
        let tilt = if delay == 0 {
            let n = n.wrapping_sub(1);
            g.write::<u32>(r + 0x2c, n);
            if n == 0 {
                eax = list_remove(g, img, img.rel32_target(f + 0x66), w(0x61), obj);
                if eax != 0 {
                    let p = g.read::<u32>(w(0x6f));
                    g.write::<u32>(p, w(0x76));
                    let s713 = slot_target(g, w(0x7b), 713);
                    g.call(s713, w(0x61), &[obj]);
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
            g.write::<u32>(r + 0x30, delay - 1);
            0x1000
        };
        eax = tilt;
        g.write::<u16>(obj + 0x76, tilt as u16);
        for i in 0..3u32 {
            let v = g.read::<f32>(r + 4 * i) + g.read::<f32>(obj + 0xc0 + 4 * i);
            g.write::<f32>(obj + 0xc0 + 4 * i, wrap(v, hi, lo, t));
        }
        let x = g.read::<f32>(obj + 0xd0) + g.read::<f32>(r + 0x10);
        let dy = g.read::<f32>(r + 0x24) + g.read::<f32>(r + 0x14);
        g.write::<f32>(obj + 0xd0, x);
        let y = g.read::<f32>(obj + 0xd4) + dy;
        g.write::<f32>(obj + 0xd4, y);
        let z = g.read::<f32>(obj + 0xd8) + g.read::<f32>(r + 0x18);
        g.write::<f32>(obj + 0xd8, z);
        for o in [0x10u32, 0x14, 0x18] {
            let v = g.read::<f32>(r + 0x28) * g.read::<f32>(r + o);
            g.write::<f32>(r + o, v);
        }
        for o in [0u32, 4, 8] {
            let v = g.read::<f32>(r + o);
            if abs(v) > eps {
                g.write::<f32>(r + o, v * slow);
            }
        }
    }
    Ret::eax(eax)
}

/// 0x100043c0 (598 bytes, 1 overlay).
pub const SPIN_STEP: Sig = Sig {
    len: 598,
    hash: 0x1b55_eea9_ea73_adf1,
    calls: &[(0x66, Callee::Sig(&crate::l5_objhost::LIST_REMOVE))],
    imports: &[],
};
