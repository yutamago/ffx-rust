//! `magic_0307.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's end (0x10006e50, no arguments, an aligned frame): when
/// the effect (`*E`, `+0x09`) holds a pool node (`+0x2864`) it is given
/// back ([`give_back`](crate::sa_pool::give_back)), the field cleared,
/// and the handles of the 18 part blocks (`*P + 0x340..0x580`, 0x20 apart,
/// `P` `+0x31`) are registered: slot 772 `(block, 0)`'s 64-bit result to
/// slot 779 `(lo, hi, block, 0)` with the game flag `*L` (`+0x49`/`+0x65`)
/// 1 around it. Then the part item `+0x7a` leaves the object list `+0x7f`
/// ([`list_remove`](crate::l5_objhost::list_remove)); when it was there
/// the pointer `*Q` (`+0x8d`) gets its block, slot 713 `(item)` frees
/// it and the flag halfword `+0xb4` loses bit 15. Returns 0x7fff then,
/// else 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x09));
    let node = g.read::<u32>(fx + 0x2864);
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x19), node);
        g.write::<u32>(fx + 0x2864, 0);
        for off in (0x340u32..0x580).step_by(0x20) {
            let block = g.read::<u32>(w(0x31)).wrapping_add(off);
            let s = slot_target(g, w(0x3b), 0xc10 / 4);
            let h = g.call(s, 0, &[block, 0]);
            let lock = g.read::<u32>(w(0x49));
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let s = slot_target(g, w(0x52), 0xc2c / 4);
            g.call(s, 0, &[h.eax, h.edx, block, 0]);
            let lock = g.read::<u32>(w(0x65));
            g.write::<u32>(lock, saved);
        }
    }
    let removed =
        crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x84), w(0x7f), w(0x7a));
    if removed == 0 {
        return Ret::eax(0);
    }
    let ptr = g.read::<u32>(w(0x8d));
    g.write::<u32>(ptr, w(0x98));
    let s = slot_target(g, w(0x9d), 0xb24 / 4);
    g.call(s, 0, &[w(0x92)]);
    let fl = g.read::<u16>(w(0xb4));
    g.write::<u16>(w(0xb4), fl & 0x7fff);
    Ret::eax(0x7fff)
}

/// [`end`] (190 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 190,
    hash: 0x7beb_93b8_bb55_073a,
    calls: &[
        (0x19, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x84, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
    ],
    imports: &[],
};

/// A part's sound and model (0x10006f10, cdecl `(obj)`): with `i` the
/// object's actor (`+0x120`; -1 uses `*H` itself) slot 239 `(*H, B[i],
/// 2)` (`H` `+0x08`, the byte list `*B`), the actor's mode 0
/// ([`actor_mode`](crate::sa_script::actor_mode)), slot 388 `(B[i], 6,
/// 1)` and slot 528 `(B[i], 1)`; then slot 393 `(h, rec + 0x1090, 0x100)`
/// with `h` slot 719 `(F, 6, 3)`'s handle (`F` `+0xb0`) and `rec` the
/// actor's record (`*R + 0xd4 i`, `R` `+0x1e`), whose `+0x10d0`
/// becomes 1. Returns slot 393's `eax`. Each slot call passes the words the
/// original leaves on the stack.
pub fn part_model<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let h = g.read::<u32>(w(0x08));
    let i = g.read::<u32>(obj + 0x120);
    let rec = i.wrapping_mul(0xd4).wrapping_add(g.read::<u32>(w(0x1e)));
    let byte = |g: &G, minus: u32, list: u32| {
        if i == u32::MAX {
            g.read::<u32>(w(minus))
        } else {
            let b = g.read::<u32>(w(list));
            u32::from(g.read::<u8>(b.wrapping_add(i)))
        }
    };
    let b = if i == u32::MAX {
        h
    } else {
        let list = g.read::<u32>(w(0x2c));
        u32::from(g.read::<u8>(list.wrapping_add(i)))
    };
    let s = slot_target(g, w(0x38), 0x3bc / 4);
    g.call(s, 0, &[h, b, 2]);
    // push ebp; esi; edi; the call's return address
    crate::sa_script::actor_mode(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x4d),
            esp: fr.esp.wrapping_sub(0x10),
            ecx: i,
            edx: 0,
            ..*fr
        },
    );
    let b = byte(g, 0x57, 0x5e);
    let s = slot_target(g, w(0x6c), 0x610 / 4);
    g.call(s, 0, &[b, 6, 1]);
    let b = byte(g, 0x81, 0x88);
    let mut p = Pushes::default();
    p.push_all(&[1, b]);
    let s = slot_target(g, w(0x94), 0x840 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[3, 6, w(0xb0)]);
    let s = slot_target(g, w(0xa1), 0xb3c / 4);
    // call width: 12 (Pushes caps the words at 12)
    let handle = p.call(g, s, 0).eax;
    p.push_all(&[0x100, rec.wrapping_add(0x1090), handle]);
    let s = slot_target(g, w(0xc4), 0x624 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    g.write::<u32>(rec.wrapping_add(0x10d0), 1);
    Ret::eax(eax)
}

/// [`part_model`] (225 bytes, 1 overlay).
pub const PART_MODEL: Sig = Sig {
    len: 225,
    hash: 0x1c16_ee8c_3560_9966,
    calls: &[(0x4d, Callee::Sig(&crate::sa_script::ACTOR_MODE))],
    imports: &[],
};
