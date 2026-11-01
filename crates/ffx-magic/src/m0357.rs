//! `magic_0357.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// An object's start (0x100010f0, cdecl `(obj)`): with `m` =
/// `(*C)[0x198]` (`C` `+0x04`): slot 444 `(1)`, slot 434 `(*C, F)` (`F`
/// `+0x25`), slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0x46`,
/// `+0x53`); the object's current list word (`+0x2c + 16 [+0xac]`) gets
/// `m`. Returns twice the list index. Each slot call passes the words
/// the original leaves on the stack.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let m = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x198);
    let mut p = Pushes::default();
    p.push(1);
    let s = slot_target(g, w(0x11), 0x6f0 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x25), g.read::<u32>(w(0x2b))]);
    let s = slot_target(g, w(0x20), 0x6c8 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let obj = fr.arg(g, 0);
    p.push_all(&[w(0x46), obj]);
    let s = slot_target(g, w(0x38), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x53), obj]);
    let s = slot_target(g, w(0x4e), 0x704 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), m);
    Ret::eax(i)
}

/// [`object_start`] (115 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 115,
    hash: 0xaa78_fdf9_f785_dbb5,
    calls: &[],
    imports: &[],
};

/// The copies' step (0x100011e0, cdecl `(obj)`): slot 434 `(*C, F)` four
/// times (`C`, `F` the operands of each call); the counter `*N` (`+0x64`)
/// goes up and from 180 on slot 450 `(obj, F2)` (`+0x7e`) moves the
/// object on. Returns slot 450's `eax`, else the counter. Each slot call
/// passes the words the original leaves on the stack.
pub fn copies_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = Pushes::default();
    for (cb, list, table) in [
        (0x09, 0x0f, 0x04),
        (0x21, 0x27, 0x1c),
        (0x39, 0x3f, 0x34),
        (0x51, 0x57, 0x4c),
    ] {
        p.push_all(&[w(cb), g.read::<u32>(w(list))]);
        let s = slot_target(g, w(table), 0x6c8 / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
    }
    p.pop(8);
    let n = g.read::<u32>(w(0x64)).wrapping_add(1);
    g.write::<u32>(w(0x6d), n);
    if n.cast_signed() < 0xb4 {
        return Ret::eax(n);
    }
    p.push_all(&[w(0x7e), fr.arg(g, 0)]);
    let s = slot_target(g, w(0x79), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`copies_step`] (146 bytes, 1 overlay).
pub const COPIES_STEP: Sig = Sig {
    len: 146,
    hash: 0xae17_a88c_0244_2a84,
    calls: &[],
    imports: &[],
};

/// The mark's repeat (0x10001860, cdecl `(obj)`): the counter `*N`
/// (`+0x0f`) goes up; below 10 slot 388 `(m, 5, 1)` (`m` = `(*C)[0x1a0]`,
/// `C` `+0x04`) and slots 450 `(obj, F)` (`+0x36`) and 451 `(obj, 0x14)`;
/// from 10 on slot 446 `(m, 0)` and the object's flags gain bit 2.
/// Returns slot 451's `eax`, else the object.
pub fn mark_repeat<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let m = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x1a0);
    let n = g.read::<u32>(w(0x0f)).wrapping_add(1);
    g.write::<u32>(w(0x15), n);
    let obj = fr.arg(g, 0);
    if n.cast_signed() >= 0xa {
        let s = slot_target(g, w(0x1d), 0x6f8 / 4);
        g.call(s, 0, &[m, 0]);
        let fl = g.read::<u32>(obj);
        g.write::<u32>(obj, fl | 4);
        return Ret::eax(obj);
    }
    let mut p = Pushes::default();
    p.push_all(&[1, 5, m]);
    let s = slot_target(g, w(0x1d), 0x610 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x36), obj]);
    let s = slot_target(g, w(0x31), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0x14, obj]);
    let s = slot_target(g, w(0x46), 0x70c / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`mark_repeat`] (114 bytes, 1 overlay).
pub const MARK_REPEAT: Sig = Sig {
    len: 114,
    hash: 0xaa73_c88e_aac0_b6bb,
    calls: &[],
    imports: &[],
};
