//! `magic_0205.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::mul;
use crate::{Frame, Game, Image, Ret, Sig};

/// A model's scale matrix (0x10001ff0, internal: `edx` = the matrix
/// `m`): slot 235 `(m)`; with `b` the byte `(*P)[1]` (`P` at `+0x17`)
/// and `x = T[b]` (`T` at `+0x22`, both read again for the second part),
/// slot 277 `(x)`'s float times `K` (`+0x3f`) into the diagonal `m[0]`,
/// `m[5]`, `m[10]`, which then get multiplied by slots 15, 16 and 17
/// `(x)`'s floats in turn. Returns slot 17's `eax`. Each slot call passes
/// the words the original leaves on the stack.
pub fn scale_matrix<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let m = fr.edx;
    let mut p = Pushes::default();
    p.push(m);
    let s = slot_target(g, w(0x05), 0x3ac / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let pick = |g: &G, at: u32, table: u32| {
        let b = g.read::<u8>(g.read::<u32>(w(at)) + 1);
        g.read::<u32>(w(table).wrapping_add(4 * u32::from(b)))
    };
    p.push(pick(g, 0x17, 0x22));
    let s = slot_target(g, w(0x27), 0x454 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let v = p.call(g, s, 0).st0.unwrap_or(0.0) as f32;
    let d = mul(v, g.read::<f32>(w(0x3f)));
    for off in [0x28, 0x14, 0] {
        g.write::<f32>(m + off, d);
    }
    let x = pick(g, 0x52, 0x5d);
    let mut eax = 0;
    for (off, table, slot) in [(0, 0x62, 0x3c), (0x14, 0x79, 0x40), (0x28, 0x95, 0x44)] {
        p.push(x);
        let s = slot_target(g, w(table), slot / 4);
        // call width: 12 (Pushes caps the words at 12)
        let out = p.call(g, s, 0);
        eax = out.eax;
        let k = out.st0.unwrap_or(0.0) as f32;
        let r = mul(g.read::<f32>(m + off), k);
        g.write::<f32>(m + off, r);
    }
    Ret::eax(eax)
}

/// [`scale_matrix`] (191 bytes, 1 overlay).
pub const SCALE_MATRIX: Sig = Sig {
    len: 191,
    hash: 0xbd7e_ed4d_2b14_26a3,
    calls: &[],
    imports: &[],
};
