//! `magic_0661.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Stops the effect's sound (0x100015e0, an overlay table entry, no
/// arguments; the record `R = *G_R`, operand `+0x03`): 0 without a
/// record. Else the byte `R[0x180]` 1 becomes 0; slot 932, then slots
/// 931, 847 and 922 `(R[0x830], 0)` (each seeing the earlier calls'
/// words). Returns 1.
pub fn sound_stop<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let r = g.read::<u32>(w(0x3));
    if r == 0 {
        return Ret::eax(0);
    }
    if g.read::<u8>(r + 0x180) == 1 {
        g.write::<u8>(r + 0x180, 0);
    }
    let t = slot_target(g, w(0x20), 932);
    g.call(t, 0, &[]);
    let mut stack = Vec::new();
    for (op, slot) in [(0x2d, 931), (0x42, 847), (0x57, 922)] {
        let a = g.read::<u32>(r + 0x830);
        stack.splice(0..0, [a, 0]);
        let t = slot_target(g, w(op), slot);
        // call width: 6 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    Ret::eax(1)
}

/// [`sound_stop`] (117 bytes, 14 table entries, 1 overlay).
pub const SOUND_STOP: Sig = Sig {
    len: 117,
    hash: 0xffb4_f25e_b81d_4d7d,
    calls: &[],
    imports: &[],
};

/// Starts a glow part (0x10001c20, an overlay table entry, `(obj)`;
/// seven table entries): the part started with program 0
/// ([`start_part_eax`](crate::part_draw::start_part_eax): `*G_P = ITEM`
/// `+0x04`/`+0x0b`, `G_O` `+0x10`, slot 941 `+0x18`); its colour `+0x40`
/// from the first of the 14 slots (pairs from the immediate `+0x3f` - 4)
/// that is on and free (slot 325 `(obj + 0x40, COLOURS + 16 k)`, the
/// table at `+0x5e`; `k` -1 when none is); `+0x50..+0x58` 0, `+0x5c` 1,
/// `+0x60..+0x68` 0.12760... (0x3e02a8ca), `+0x6c` 1.0. Returns 1.
pub fn glow_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    crate::part_draw::start_part_eax(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x35], 0, &[]);
    let slots = w(0x3f);
    let mut k = u32::MAX;
    for i in 0..0xe {
        let at = slots + 8 * i;
        if g.read::<u32>(at - 4) == 1 && g.read::<u32>(at) == 0 {
            k = i;
            break;
        }
    }
    let colour = (k << 4).wrapping_add(w(0x5e));
    let t = slot_target(g, w(0x68), 325);
    g.call(t, 0, &[obj + 0x40, colour]);
    for (off, v) in [
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x60, 0x3e02_a8ca),
        (0x64, 0x3e02_a8ca),
        (0x68, 0x3e02_a8ca),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(obj + off, v);
    }
    Ret::eax(1)
}

/// [`glow_part`] (183 bytes, 1 overlay).
pub const GLOW_PART: Sig = Sig {
    len: 183,
    hash: 0x1ac9_7b60_6218_18ad,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
