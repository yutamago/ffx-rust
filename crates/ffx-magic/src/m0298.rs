//! `magic_0298.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Models to a list (0x10001fe0, internal: `ecx` = the model bytes,
/// `edx` = their count, one stack word `list`): for each byte `b` the
/// model of slot 215 `(b)` goes to slot 276 `(model, list)`, and slot
/// 219 `(b, 0)` and `(b, 1)`'s answers likewise when non-zero. Returns
/// the last slot call's `eax` (the count when there are none).
pub fn models_to_list<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (bytes, n) = (fr.ecx, fr.edx);
    let list = fr.arg(g, 0);
    let mut eax = n;
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let b = u32::from(g.read::<u8>(bytes.wrapping_add(i)));
        let s = slot_target(g, w(0x25), 0x35c / 4);
        let model = g.call(s, 0, &[b]).eax;
        let s = slot_target(g, w(0x35), 0x450 / 4);
        // call width: 3 (the earlier call's word under it)
        g.call(s, 0, &[model, list, b]);
        let s = slot_target(g, w(0x42), 0x36c / 4);
        // call width: 5 (the earlier calls' words under it)
        let x = g.call(s, 0, &[b, 0, model, list, b]).eax;
        if x != 0 {
            let s = slot_target(g, w(0x5b), 0x450 / 4);
            g.call(s, 0, &[x, list]);
        }
        let s = slot_target(g, w(0x6b), 0x36c / 4);
        eax = g.call(s, 0, &[b, 1]).eax;
        if eax != 0 {
            let s = slot_target(g, w(0x84), 0x450 / 4);
            eax = g.call(s, 0, &[eax, list]).eax;
        }
        i += 1;
    }
    Ret::eax(eax)
}

/// [`models_to_list`] (163 bytes, 1 overlay).
pub const MODELS_TO_LIST: Sig = Sig {
    len: 163,
    hash: 0x6471_3a73_0816_9360,
    calls: &[],
    imports: &[],
};
