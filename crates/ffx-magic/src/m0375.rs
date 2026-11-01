//! `magic_0375.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A part start by slot (0x10001bc0, cdecl `(obj)`; `m0351`'s kin):
/// [`part_head`](crate::m0351::part_head) with the list's word 0; with
/// `i` the object's slot in the block `+0x3c` (128 bytes each), slot 0
/// and 1 get their positions `+0x40..+0x48` and the motion words
/// (`+0x4c` 1.0, `+0x50..+0x58` 0, `+0x5c` 1, the scale `+0x60..+0x68`
/// 0.0686 and `+0x6c` 1.0); other slots nothing more. Returns `i`.
pub fn part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (obj, _) = crate::m0351::part_head(g, img, fr, 0, 0x35);
    let i = (obj.wrapping_sub(w(0x3c)).cast_signed() >> 7).cast_unsigned();
    let pos: [u32; 3] = match i {
        0 => [0x4140_0000, 0xc138_0000, 0x4208_0000],
        1 => [0xc110_0000, 0xc148_0000, 0xc1d0_0000],
        _ => return Ret::eax(i),
    };
    for (k, v) in (0u32..).zip(pos) {
        g.write::<u32>(obj + 0x40 + 4 * k, v);
    }
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    for off in [0x68, 0x64, 0x60] {
        g.write::<u32>(obj + off, 0x3d8c_4828);
    }
    g.write::<u32>(obj + 0x5c, 1);
    for off in [0x58, 0x54, 0x50] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    Ret::eax(i)
}

/// [`part_start`] (189 bytes, 1 overlay).
pub const PART_START: Sig = Sig {
    len: 189,
    hash: 0xbed2_3b0e_3d83_d76f,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// A part draw (0x100010a0, cdecl `(obj)`): slot 258 `(obj)`, slot 231
/// `(*D, *E, *C)` (`D` `+0x2d`, `E` `+0x21`, `C` `+0x16`), slot 230 `(*B,
/// *D)` (`B` `+0x40`, `D` again `+0x35`), the pointer `*P` (`+0x4d`) gets
/// the block `+0x58` and slot 712 `(obj, 0x30)` runs with the game flag
/// `*L` (`L` = `*Q`, `+0x5d`, again `+0x78`) 1 around it. When slot 712
/// answers 0 slot 261 `(1)` is asked: returns 1 when that answers 0, else
/// 0. The slot calls pass the words the original leaves on the stack.
pub fn part_draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let mut st = crate::pushes::Pushes::default();
    st.push(obj);
    let s = crate::runtime::slot_target(g, w(0x04), 0x408 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let c = g.read::<u32>(w(0x16));
    let s = crate::runtime::slot_target(g, w(0x1b), 0x39c / 4);
    let e = g.read::<u32>(w(0x21));
    let d = g.read::<u32>(w(0x2d));
    st.push_all(&[c, e, d]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let d = g.read::<u32>(w(0x35));
    let s = crate::runtime::slot_target(g, w(0x3a), 0x398 / 4);
    let b = g.read::<u32>(w(0x40));
    st.push_all(&[d, b]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let ptr = g.read::<u32>(w(0x4d));
    st.push_all(&[0x30, obj]);
    g.write::<u32>(ptr, w(0x58));
    let lock = g.read::<u32>(w(0x5d));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s = crate::runtime::slot_target(g, w(0x6a), 0xb20 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, s, 0).eax;
    let lock = g.read::<u32>(w(0x78));
    g.write::<u32>(lock, saved);
    if r != 0 {
        return Ret::eax(0);
    }
    let s = crate::runtime::slot_target(g, w(0x8b), 0x414 / 4);
    let on = g.call(s, 0, &[1]).eax;
    Ret::eax(u32::from(on == 0))
}

/// [`part_draw`] (163 bytes, 1 overlay).
pub const PART_DRAW: Sig = Sig {
    len: 163,
    hash: 0x9a28_dbeb_a334_f12e,
    calls: &[],
    imports: &[],
};

/// A part draw that restarts (0x100019a0, cdecl `(part)`): slot 258
/// `(part)`, slot 231 `(*D, *E, *C)` (`D` `+0x2f`, `E` `+0x23`, `C`
/// `+0x18`), slot 230 `(*B, *D)` (`B` `+0x42`, `D` again `+0x37`), the
/// pointer `*P` (`+0x4f`) gets the block `+0x57` and slot 712 `(part,
/// 0x30)` runs with the game flag `*L` (`L` = `*Q`, `+0x5c`, again
/// `+0x78`) 1 around it; when slot 712 answers non-zero: `*P` again
/// (`+0x86`/`+0x8d`), slot 713 `(part)`, `*P` again (`+0x9f`/`+0xa5`) and
/// the program from slot 941 `((*S)[4])`'s list (`S` `+0xaa`, its
/// `+0x28`) started at the part ([`start_part`](crate::opu_parts::start_part)
/// `(part, 0x1000, program, 1)`, `+0xcd`). Returns 1. The slot calls pass
/// the words the original leaves on the stack.
pub fn part_restart<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = fr.arg(g, 0);
    let mut st = crate::pushes::Pushes::default();
    let s = crate::runtime::slot_target(g, w(0x04), 0x408 / 4);
    st.push(part);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let c = g.read::<u32>(w(0x18));
    let s = crate::runtime::slot_target(g, w(0x1d), 0x39c / 4);
    let e = g.read::<u32>(w(0x23));
    let d = g.read::<u32>(w(0x2f));
    st.push_all(&[c, e, d]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let d = g.read::<u32>(w(0x37));
    let s = crate::runtime::slot_target(g, w(0x3c), 0x398 / 4);
    let b = g.read::<u32>(w(0x42));
    st.push_all(&[d, b]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let ptr = g.read::<u32>(w(0x4f));
    st.push(0x30);
    g.write::<u32>(ptr, w(0x57));
    let lock = g.read::<u32>(w(0x5c));
    st.push(part);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s = crate::runtime::slot_target(g, w(0x6a), 0xb20 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, s, 0).eax;
    let lock = g.read::<u32>(w(0x78));
    g.write::<u32>(lock, saved);
    if r == 0 {
        return Ret::eax(1);
    }
    let mut st = crate::pushes::Pushes::default();
    let ptr = g.read::<u32>(w(0x86));
    st.push(part);
    g.write::<u32>(ptr, w(0x8d));
    let s = crate::runtime::slot_target(g, w(0x92), 0xb24 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let ptr = g.read::<u32>(w(0x9f));
    g.write::<u32>(ptr, w(0xa5));
    let x = g.read::<u32>(g.read::<u32>(w(0xaa)) + 0x10);
    st.push(x);
    let s = crate::runtime::slot_target(g, w(0xb2), 0xeb4 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let list = st.call(g, s, 0).eax;
    let prog = g.read::<u32>(list + 0x28);
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0xcd), part, 0x1000, prog, 1);
    Ret::eax(1)
}

/// [`part_restart`] (221 bytes, 1 overlay).
pub const PART_RESTART: Sig = Sig {
    len: 221,
    hash: 0x39f3_ebb7_f0cf_e7f1,
    calls: &[(0xcd, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
