//! `magic_0605.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::give_back;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Moves the 18 pieces (0x10009fc0, internal, no arguments; the record
/// `R = *G_R`, operand `+0x01`; 0x60 bytes each from `R + 0x1f5c - 0x38`):
/// with `p` the piece's `+0x38`, the speed `p - 0x8..` grows by the
/// acceleration `p + 0x8..`, the position `p - 0x18..` by the speed; the
/// angles `p - 0x38..` grow by `p - 0x28..`, each wrapped once by `W`
/// (`+0x11`) above `H` (`+0x09`) or below `L` (`+0x19`). Returns the
/// cursor past the last piece.
pub fn move_pieces<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = g.read::<u32>(w(0x1)).wrapping_add(0x1f5c);
    let hi = g.read::<f32>(w(0x9));
    let wrap = g.read::<f32>(w(0x11));
    let lo = g.read::<f32>(w(0x19));
    let at = |p: u32, d: i32| p.wrapping_add(d.cast_unsigned());
    for _ in 0..0x12 {
        for (dst, src) in [
            (-8, 8),
            (-4, 0xc),
            (0, 0x10),
            (-0x18, -8),
            (-0x14, -4),
            (-0x10, 0),
        ] {
            let v = add(g.read::<f32>(at(p, src)), g.read::<f32>(at(p, dst)));
            g.write::<f32>(at(p, dst), v);
        }
        for (dst, src) in [(-0x38, -0x28), (-0x34, -0x24), (-0x30, -0x20)] {
            let mut x = add(g.read::<f32>(at(p, src)), g.read::<f32>(at(p, dst)));
            // comiss x, H; jbe: below, equal or unordered
            if x > hi {
                x = sub(x, wrap);
            } else if lo > x {
                x = add(x, wrap);
            }
            g.write::<f32>(at(p, dst), x);
        }
        p = p.wrapping_add(0x60);
    }
    p
}

/// [`move_pieces`] (251 bytes, 1 overlay).
pub const MOVE_PIECES: Sig = Sig {
    len: 251,
    hash: 0x0419_1bb2_5702_e491,
    calls: &[],
    imports: &[],
};

/// The effect's end (0x1000a390, an overlay table entry, no arguments;
/// the record `R = *G_R`, operand `+0x0a`): when `R[0x30b4]` holds a
/// node, it goes back to the pool ([`give_back`]), the word is 0 and the
/// 18 entries `*G_T + 0x1a0 + 0x20 k` (operand `+0x31`) are loaded
/// (slot 772 `(entry, 0)`, then slot 779 with its `eax`/`edx`, the word
/// at `*G_F` (`+0x49`) 1 around the call). Then the object `R + 0x4c0`
/// leaves the list (immediate `+0x82`,
/// [`list_remove`](crate::l5_objhost::list_remove)); when it was there,
/// `*G_P` (`+0x90`) gets the immediate `+0x97`, slot 713 `(obj)` and bit
/// 15 of the halfword `obj + 0x124` is cleared. Returns 0x7fff, or 0 when
/// the object was not in the list.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = g.read::<u32>(w(0xa));
    let node = g.read::<u32>(r + 0x30b4);
    if node != 0 {
        give_back(g, img, img.rel32_target(f + 0x1b), node);
        g.write::<u32>(r + 0x30b4, 0);
        let mut k = 0x1a0u32;
        while k < 0x3e0 {
            let e = g.read::<u32>(w(0x31)).wrapping_add(k);
            let t = slot_target(g, w(0x3b), 772);
            let o = g.call(t, 0, &[e, 0]);
            let fl = g.read::<u32>(w(0x49));
            let saved = g.read::<u32>(fl);
            g.write::<u32>(fl, 1);
            let t = slot_target(g, w(0x52), 779);
            g.call(t, 0, &[o.eax, o.edx, e, 0]);
            let fl = g.read::<u32>(w(0x65));
            g.write::<u32>(fl, saved);
            k += 0x20;
        }
    }
    let obj = r + 0x4c0;
    let removed = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x87), w(0x82), obj);
    if removed == 0 {
        return Ret::eax(0);
    }
    g.write::<u32>(g.read::<u32>(w(0x90)), w(0x97));
    let t = slot_target(g, w(0x9c), 713);
    g.call(t, 0, &[obj]);
    let h = g.read::<u16>(obj + 0x124);
    g.write::<u16>(obj + 0x124, h & 0x7fff);
    Ret::eax(0x7fff)
}

/// [`end`] (190 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 190,
    hash: 0x24b1_c0be_f240_ba89,
    calls: &[
        (0x1b, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x87, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
    ],
    imports: &[],
};

/// The actor's id: `me` for actor -1, else the byte `LIST[i]` (the list
/// at `*G_L`).
fn who<G: Game>(g: &mut G, me_at: u32, list_at: u32, i: u32) -> u32 {
    if i == u32::MAX {
        g.read::<u32>(me_at)
    } else {
        let list = g.read::<u32>(list_at);
        u32::from(g.read::<u8>(list.wrapping_add(i)))
    }
}

/// An actor's part callback (0x1000a450, cdecl `(obj)`, pushed by the
/// DLL's code): with `i = obj[0x120]` and the record `P = *G_R + 0xd4 i`
/// (operand `+0x1e`), slot 239 `(G_ME, who, 2)`; the actor's mode 0
/// ([`actor_mode`](crate::sa_script::actor_mode)); slot 388 `(who, 6,
/// 1)`; slot 528 `(who, 1)`, slot 719 `(DATA, 6, 3)` and slot 393
/// `(that, P + 0x18e0, 0x100)` (the earlier words left under them); then
/// `P[0x1920]` 1. `who` is `G_ME` for actor -1, else the actor list's
/// byte. Returns slot 393's `eax`.
pub fn actor_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let me = g.read::<u32>(w(0x8));
    let i = g.read::<u32>(obj + 0x120);
    let p = i.wrapping_mul(0xd4).wrapping_add(g.read::<u32>(w(0x1e)));
    let a = if i == u32::MAX {
        me
    } else {
        let list = g.read::<u32>(w(0x2c));
        u32::from(g.read::<u8>(list.wrapping_add(i)))
    };
    let t = slot_target(g, w(0x38), 239);
    g.call(t, 0, &[me, a, 2]);
    let mode = Frame {
        at: img.rel32_target(f + 0x4d),
        // push ebp; push esi; push edi; the call
        esp: fr.esp.wrapping_sub(0x10),
        ecx: i,
        edx: 0,
        ..*fr
    };
    crate::sa_script::actor_mode(g, img, &mode);
    let b = who(g, w(0x57), w(0x5e), i);
    let t = slot_target(g, w(0x6c), 388);
    g.call(t, 0, &[b, 6, 1]);
    let c = who(g, w(0x81), w(0x88), i);
    let t = slot_target(g, w(0x94), 528);
    g.call(t, 0, &[c, 1]);
    let t = slot_target(g, w(0xa1), 719);
    let d = g.call(t, 0, &[w(0xb0), 6, 3, c, 1]).eax;
    let t = slot_target(g, w(0xc4), 393);
    // call width: 8 (the earlier calls' words stay on the stack)
    let eax = g
        .call(t, 0, &[d, p + 0x18e0, 0x100, w(0xb0), 6, 3, c, 1])
        .eax;
    g.write::<u32>(p + 0x1920, 1);
    eax
}

/// [`actor_part`] (225 bytes, 1 overlay).
pub const ACTOR_PART: Sig = Sig {
    len: 225,
    hash: 0x54f8_df7a_aa24_34f9,
    calls: &[(0x4d, Callee::Sig(&crate::sa_script::ACTOR_MODE))],
    imports: &[],
};
