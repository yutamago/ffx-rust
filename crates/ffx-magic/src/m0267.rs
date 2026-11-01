//! `magic_0267.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::{div, mul};
use crate::{Frame, Game, Image, Ret, Sig};

/// Points through two transforms (0x10001340, internal: `ecx` = `b`,
/// `edx` = the points (16 bytes each), stack `(n, out, m)`): nothing for
/// `n <= 0` (the caller's `eax`). Else `out -= b` (in its argument word)
/// and for each point `p` (the i-th, `b` growing by 16): slot 229 `(p,
/// NAME, b)` (`NAME` at `+0x30`), slot 229 `(out + b, m, b)`, then `p`'s
/// x, y and z times `K / w` and `w` = `K / w` (`K` at `+0x52`); `n`
/// counts down in its argument word. Returns the last slot call's `eax`.
pub fn transform_points<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n_at = fr.esp + 4;
    if g.read::<i32>(n_at) <= 0 {
        return Ret::eax(fr.eax);
    }
    let mut b = fr.ecx;
    let out = g.read::<u32>(fr.esp + 8).wrapping_sub(b);
    g.write::<u32>(fr.esp + 8, out);
    let mut p = fr.edx;
    let mut eax;
    loop {
        let s = slot_target(g, w(0x21), 0x394 / 4);
        g.call(s, 0, &[p, w(0x30), b]);
        // the argument words are read again each time
        let out = g.read::<u32>(fr.esp + 8);
        let m = g.read::<u32>(fr.esp + 0xc);
        let s = slot_target(g, w(0x42), 0x394 / 4);
        // call width: 6 (the earlier call's words under it)
        eax = g
            .call(s, 0, &[out.wrapping_add(b), m, b, p, w(0x30), b])
            .eax;
        let k = g.read::<f32>(w(0x52));
        let scale = div(k, g.read::<f32>(p + 0xc));
        for off in [0, 4, 8] {
            let v = mul(g.read::<f32>(p + off), scale);
            g.write::<f32>(p + off, v);
        }
        let v = div(k, g.read::<f32>(p + 0xc));
        g.write::<f32>(p + 0xc, v);
        b = b.wrapping_add(0x10);
        p = p.wrapping_add(0x10);
        let n = g.read::<u32>(n_at).wrapping_sub(1);
        g.write::<u32>(n_at, n);
        if n == 0 {
            break;
        }
    }
    Ret::eax(eax)
}

/// [`transform_points`] (170 bytes, 1 overlay).
pub const TRANSFORM_POINTS: Sig = Sig {
    len: 170,
    hash: 0xb2ab_4ffe_b669_936a,
    calls: &[],
    imports: &[],
};
