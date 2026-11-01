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

/// Starts a beam part (0x10001700, an overlay table entry, `(obj)`; ten
/// table entries): `k` = the part's index in its array (operand `+0x0d`,
/// 0x80 bytes each, signed), `i` the first of the 14 slots that is on and
/// free (pairs from the immediate `+0x17` - 4; -1 when none is); the part
/// started with program 5 ([`start_part_eax`](crate::part_draw::start_part_eax):
/// `*G_P = ITEM` `+0x38`/`+0x3e`, `G_O` `+0x43`, slot 941 `+0x4b`); its
/// colour `+0x40` from slot 325 `(obj + 0x40, TABLE + 16 i)` (the table at
/// `+0x77` for the first part, else `+0x7e`); `+0x58` -0x4000 for the
/// slots 2..5, else 0; `+0x5c` 1, `+0x50`/`+0x54` 0, `+0x60..+0x68`
/// 0x3e02a8ca, `+0x6c` 1.0. Returns 1.
pub fn beam_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let k = obj.wrapping_sub(w(0xd)).cast_signed() >> 7;
    let slots = w(0x17);
    let mut i = u32::MAX;
    for j in 0..0xe {
        let at = slots + 8 * j;
        if g.read::<u32>(at - 4) == 1 && g.read::<u32>(at) == 0 {
            i = j;
            break;
        }
    }
    crate::part_draw::start_part_eax(g, img, f, obj, [0x38, 0x3e, 0x43, 0x4b, 0x66], 0x14, &[]);
    let table = if k == 0 { w(0x77) } else { w(0x7e) };
    let colour = (i << 4).wrapping_add(table);
    let t = slot_target(g, w(0x88), 325);
    g.call(t, 0, &[obj + 0x40, colour]);
    let lift = if (2..=5).contains(&i) { 0xffff_c000 } else { 0 };
    for (off, v) in [
        (0x58, lift),
        (0x5c, 1),
        (0x54, 0),
        (0x50, 0),
        (0x60, 0x3e02_a8ca),
        (0x64, 0x3e02_a8ca),
        (0x68, 0x3e02_a8ca),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(obj + off, v);
    }
    Ret::eax(1)
}

/// [`beam_part`] (246 bytes, 1 overlay).
pub const BEAM_PART: Sig = Sig {
    len: 246,
    hash: 0x836e_f6f7_76d7_4ae2,
    calls: &[(0x66, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's start at an actor (0x10001a50, an overlay table entry,
/// `(obj)`): program 2 started ([`start_part`](crate::part_draw::start_part));
/// the first of the DLL's 14 actor slots (8 bytes each from operand
/// `+0x4d`) with kind 1 and no model (`-1` without one) picks the point
/// `P[i]` (16 bytes each, the table at `+0x72` for the array's first
/// object, `+0x79` for the others) that slot 325 `(obj + 0x40, P[i])`
/// copies; slots 2-5 turn the object by `(0, 0, -0x4000)`, 12 and 13 by
/// `(0, 0x4718, 0x4718)`, others not; `+0x5c` 1, the scales 0.1276
/// (`+0x6c` 1.0). Returns 1.
pub fn start_at<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let k = obj.wrapping_sub(w(0x2b)).cast_signed() >> 7;
    crate::part_draw::start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x1d, 0x43], 8, &[]);
    let slots = w(0x4d);
    let found = (0..14u32).find(|&i| {
        let p = slots + 8 * i;
        g.read::<u32>(p - 4) == 1 && g.read::<u32>(p) == 0
    });
    let i = found.unwrap_or(u32::MAX);
    let tab = if k == 0 { w(0x72) } else { w(0x79) };
    let t = slot_target(g, w(0x83), 325);
    g.call(t, 0, &[obj + 0x40, i.wrapping_shl(4).wrapping_add(tab)]);
    match i {
        2..=5 => {
            g.write::<u32>(obj + 0x58, 0xffff_c000);
            g.write::<u32>(obj + 0x54, 0);
        }
        12 | 13 => {
            g.write::<u32>(obj + 0x54, 0x4718);
            g.write::<u32>(obj + 0x58, 0x4718);
        }
        _ => {
            g.write::<u32>(obj + 0x58, 0);
            g.write::<u32>(obj + 0x54, 0);
        }
    }
    for (o, v) in [
        (0x50u32, 0u32),
        (0x5c, 1),
        (0x60, 0x3e02_a8ca),
        (0x64, 0x3e02_a8ca),
        (0x68, 0x3e02_a8ca),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(obj + o, v);
    }
    Ret::eax(1)
}

/// [`start_at`] (267 bytes, 1 overlay).
pub const START_AT: Sig = Sig {
    len: 267,
    hash: 0x355a_23e5_9c2e_7c71,
    calls: &[(0x43, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
