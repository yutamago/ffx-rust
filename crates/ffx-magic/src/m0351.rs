//! `magic_0351.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// 0x10002770 (no arguments): 0 when `*A` (`+0x02`) is set; 1 while the
/// clock `*T` (`+0x0b`) is below 0x28; else for the first three `i` below
/// the count `*N` (`+0x21`) whose flag `F[i]` is set (the table at
/// `+0x2d`), slot 239 `(*H, (*B)[i], 0)` (`H` at `+0x47`, the bytes `B`
/// at `+0x35`). Returns 0.
pub fn sounds_3<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    if g.read::<u32>(w(0x02)) != 0 {
        return Ret::eax(0);
    }
    if g.read::<i32>(w(0x0b)) < 0x28 {
        return Ret::eax(1);
    }
    for i in 0..3u32 {
        if i.cast_signed() > g.read::<i32>(w(0x21)).wrapping_sub(1) {
            break;
        }
        if g.read::<u32>(w(0x2d).wrapping_add(4 * i)) == 0 {
            continue;
        }
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x35)).wrapping_add(i)));
        let h = g.read::<u32>(w(0x47));
        let s = slot_target(g, w(0x41), 239);
        g.call(s, 0, &[h, b, 0]);
    }
    Ret::eax(0)
}

/// [`sounds_3`] (96 bytes, 1 overlay).
pub const SOUNDS_3: Sig = Sig {
    len: 96,
    hash: 0x3f08_74f1_0ded_cfec,
    calls: &[],
    imports: &[],
};

/// The model's reset (0x10001000, no arguments, no frame): slot 399
/// `(*M, 1)` and slot 363 `(*M, 2, 1)` (`M` `+0x09`/`+0x26`); the word
/// `*W` (`+0x2d`) gets `*D` (`+0x36`) when it is 0; slot 382 `(0, 0)`;
/// two words cleared and two set to 0x1ccc8. Returns slot 382's `eax`.
/// Each slot call passes the words the original leaves on the stack.
pub fn model_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[1, g.read::<u32>(w(0x09))]);
    let s = slot_target(g, w(0x01), 0x63c / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[1, 2, g.read::<u32>(w(0x26))]);
    let s = slot_target(g, w(0x16), 0x5ac / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let mut v = g.read::<u32>(w(0x2d));
    if v == 0 {
        v = g.read::<u32>(w(0x36));
    }
    g.write::<u32>(w(0x3b), v);
    p.push_all(&[0, 0]);
    let s = slot_target(g, w(0x40), 0x5f8 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    g.write::<u32>(w(0x55), 0);
    g.write::<u32>(w(0x5f), 0);
    g.write::<u32>(w(0x69), 0x1ccc8);
    g.write::<u32>(w(0x73), 0x1ccc8);
    Ret::eax(eax)
}

/// [`model_reset`] (124 bytes, 1 overlay).
pub const MODEL_RESET: Sig = Sig {
    len: 124,
    hash: 0x06da_98d9_fdeb_3205,
    calls: &[],
    imports: &[],
};

/// The head of the part starts (`magic_0351` and `0372`, cdecl `(obj)`):
/// the pointer `*P` (`+0x04`) gets its block (`+0x0b`), slot 941
/// `((*G)[0x10])` (`G` `+0x10`) gives the program list and its word `k`
/// starts on the object ([`start_part`](crate::opu_parts::start_part)
/// `(obj, 0x1000, list[k], 1)`, the `rel32` at `call`). Returns the
/// object and the words left on the stack.
pub(crate) fn part_head<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    k: u32,
    call: u32,
) -> (u32, crate::pushes::Pushes) {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ptr = g.read::<u32>(w(0x04));
    g.write::<u32>(ptr, w(0x0b));
    let gp = g.read::<u32>(w(0x10));
    let mut p = crate::pushes::Pushes::default();
    p.push(g.read::<u32>(gp + 0x10));
    let s = slot_target(g, w(0x18), 0xeb4 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let list = p.call(g, s, 0).eax;
    let obj = fr.arg(g, 0);
    let prog = g.read::<u32>(list.wrapping_add(4 * k));
    p.push_all(&[0, 1, prog]);
    crate::opu_parts::start_part(g, img, img.rel32_target(f + call), obj, 0x1000, prog, 1);
    (obj, p)
}

/// The part's motion words: `+0x50..+0x58` 0, `+0x5c` 1, the scale
/// `+0x60..+0x68` `s` and `+0x6c` 1.0.
pub(crate) fn part_motion<G: Game>(g: &mut G, obj: u32, s: u32) {
    for off in [0x50, 0x54, 0x58] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0x5c, 1);
    for off in [0x60, 0x64, 0x68] {
        g.write::<u32>(obj + off, s);
    }
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
}

/// A part start (0x100015f0, cdecl `(obj)`): [`part_head`] with the
/// list's word 4, slot 325 `(obj + 0x40, V)` (`V` `+0x3e`),
/// [`part_motion`] with the scale 0.035 and the word `+0x8d` 0x80.
/// Returns 1.
pub fn part_start_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (obj, mut p) = part_head(g, img, fr, 4, 0x36);
    p.push_all(&[w(0x3e), obj + 0x40]);
    let s = slot_target(g, w(0x44), 0x514 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    part_motion(g, obj, 0x3d0f_5c29);
    g.write::<u32>(w(0x8d), 0x80);
    Ret::eax(1)
}

/// [`part_start_a`] (157 bytes, 1 overlay).
pub const PART_START_A: Sig = Sig {
    len: 157,
    hash: 0xdda9_0779_aa5d_c79f,
    calls: &[(0x36, crate::Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// A part start (0x10002130, cdecl `(obj)`): [`part_head`] with the
/// list's word 1, slot 325 `(obj + 0x40, V)` (`V` `+0x3e`), the height
/// `+0x44` up by `K` (`+0x59`), [`part_motion`] with the scale 0.0427.
/// Returns 1.
pub fn part_start_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (obj, mut p) = part_head(g, img, fr, 1, 0x36);
    p.push_all(&[w(0x3e), obj + 0x40]);
    let s = slot_target(g, w(0x44), 0x514 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let h = crate::sb_sse::add(g.read::<f32>(obj + 0x44), g.read::<f32>(w(0x59)));
    g.write::<f32>(obj + 0x44, h);
    part_motion(g, obj, 0x3d2f_1aa0);
    Ret::eax(1)
}

/// [`part_start_b`] (165 bytes, 1 overlay).
pub const PART_START_B: Sig = Sig {
    len: 165,
    hash: 0x84c2_4ebb_c0d6_8800,
    calls: &[(0x36, crate::Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// A part start (0x10002270, cdecl `(obj)`): [`part_head`] with the
/// list's word 0, [`part_motion`] with the scale 0.0713, slot 220 `(*X,
/// 0x40, 0x100)` (`X` `+0x47`). Returns 1.
pub fn part_start_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (obj, mut p) = part_head(g, img, fr, 0, 0x35);
    p.push_all(&[0x100, 0x40, g.read::<u32>(w(0x47))]);
    part_motion(g, obj, 0x3d91_eb85);
    let s = slot_target(g, w(0x3a), 0x370 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    Ret::eax(1)
}

/// [`part_start_c`] (150 bytes, 1 overlay).
pub const PART_START_C: Sig = Sig {
    len: 150,
    hash: 0xf95f_f09c_d40f_8e6e,
    calls: &[(0x35, crate::Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The camera fade (0x10002690, no arguments, no frame): slot 267 `(4)`;
/// `v` = `K / *S` (`K` `+0x13`, `S` `+0x1b`) goes five times to slot 801
/// `(v, v, v, v, v)` (written over the first call's word); `*S` = 1.0;
/// slot 785 `(*T)` twice (`T` `+0x5a`/`+0x81`, read each time). Returns 1.
pub fn camera_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = crate::pushes::Pushes::default();
    p.push(4);
    let s = slot_target(g, w(0x01), 0x42c / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let v = crate::sb_sse::div(g.read::<f32>(w(0x13)), g.read::<f32>(w(0x1b))).to_bits();
    p.set_top(v);
    p.push_all(&[v, v, v, v, v]);
    let s = slot_target(g, w(0x20), 0xc84 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.pop(5);
    g.write::<u32>(w(0x69), 0x3f80_0000);
    for (table, src) in [(0x52, 0x5a), (0x79, 0x81)] {
        p.set_top(g.read::<u32>(w(src)));
        let s = slot_target(g, w(table), 0xc44 / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
    }
    Ret::eax(1)
}

/// [`camera_fade`] (155 bytes, 1 overlay).
pub const CAMERA_FADE: Sig = Sig {
    len: 155,
    hash: 0xedf9_c791_8ecc_e02b,
    calls: &[],
    imports: &[],
};

/// The frame callback (0x10002c40, no arguments): nothing (0) until
/// started (`*S`, `+0x02`); else `**F` (`F` `+0x0d`) = 0 and, unless the
/// game is paused (slot 253), 1 (`F` again `+0x29`); `**P` (`P` `+0x34`) =
/// the immediate `+0x40`, slot 723 `(*A)` (`A` `+0x3a`) and slot 724
/// `(*B)` (`B` `+0x58`), `**Q` (`Q` `+0x66`) = `*V` (`+0x6b`) and `*W`
/// (`+0x7a`) = `*U` (`+0x72`); while the timeline runs
/// ([`timeline`](crate::sa_misc::timeline), `+0x7f`) the frame count `*N`
/// (`+0xa5`) goes up, else slot 214 `(*X)` (`X` `+0x8e`) runs. Returns 1.
/// The slot calls pass the words the original leaves on the stack.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let fl = g.read::<u32>(w(0x0d));
    g.write::<u32>(fl, 0);
    let s = slot_target(g, w(0x18), 0x3f4 / 4);
    if g.call(s, 0, &[]).eax == 0 {
        let fl = g.read::<u32>(w(0x29));
        g.write::<u32>(fl, 1);
    }
    let p = g.read::<u32>(w(0x34));
    let mut st = crate::pushes::Pushes::default();
    st.push(g.read::<u32>(w(0x3a)));
    g.write::<u32>(p, w(0x40));
    let s = slot_target(g, w(0x45), 0xb4c / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push(g.read::<u32>(w(0x58)));
    let s = slot_target(g, w(0x52), 0xb50 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let q = g.read::<u32>(w(0x66));
    let v = g.read::<u32>(w(0x6b));
    g.write::<u32>(q, v);
    let u = g.read::<u32>(w(0x72));
    g.write::<u32>(w(0x7a), u);
    if crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x7f)) != 0 {
        let n = g.read::<u32>(w(0xa5));
        g.write::<u32>(w(0xa5), n.wrapping_add(1));
        return Ret::eax(1);
    }
    let s = slot_target(g, w(0x88), 0x358 / 4);
    let x = g.read::<u32>(w(0x8e));
    g.call(s, 0, &[x]);
    Ret::eax(1)
}

/// [`frame`] (175 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 175,
    hash: 0x053f_2cb1_6968_4f6a,
    calls: &[(0x7f, crate::Callee::Sig(&crate::sa_misc::TIMELINE))],
    imports: &[],
};

/// The model reset ([`model_reset`]), then `*G_S` (operand `+0x07`) 1.0
/// (0x100014e0, a table entry, no arguments). Returns 1.
pub fn reset_scale<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let callee = Frame {
        at: img.rel32_target(fr.at + 0x1),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    model_reset(g, img, &callee);
    g.write::<u32>(img.word(fr.at + 0x7), 0x3f80_0000);
    Ret::eax(1)
}

/// [`reset_scale`] (21 bytes, 1 overlay).
pub const RESET_SCALE: Sig = Sig {
    len: 21,
    hash: 0x2da0_fcee_9248_a170,
    calls: &[(0x1, crate::Callee::Sig(&MODEL_RESET))],
    imports: &[],
};
