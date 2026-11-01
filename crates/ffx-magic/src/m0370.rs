//! `magic_0370.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The part's start (0x10001720, no arguments, no frame): the pointer
/// `*P` (`+0x01`) its block (`+0x07`); slot 941 `((*G)[0x10])` (`G`
/// `+0x0c`) gives the program list and its first program starts on the
/// part `+0x2c` ([`start_part`](crate::opu_parts::start_part) `(part,
/// 0x1000, P, 1)`); two 16-byte constants copied (`+0x39` to `+0x41`,
/// `+0x48` to `+0x5c`), `+0x51` 1.0 and two vectors set (`+0x62`..
/// `+0x80`, `+0x8a`..`+0x9e`). Returns 1.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ptr = g.read::<u32>(w(0x01));
    g.write::<u32>(ptr, w(0x07));
    let gp = g.read::<u32>(w(0x0c));
    let s = slot_target(g, w(0x14), 0xeb4 / 4);
    let x = g.read::<u32>(gp + 0x10);
    let list = g.call(s, 0, &[x]).eax;
    let prog = g.read::<u32>(list);
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x31), w(0x2c), 0x1000, prog, 1);
    let copy = |g: &mut G, src: u32, dst: u32| {
        let v: [u32; 4] = std::array::from_fn(|i| g.read::<u32>(w(src) + 4 * i as u32));
        for (k, x) in (0u32..).zip(v) {
            g.write::<u32>(w(dst) + 4 * k, x);
        }
    };
    copy(g, 0x39, 0x41);
    let v: [u32; 4] = std::array::from_fn(|i| g.read::<u32>(w(0x48) + 4 * i as u32));
    g.write::<u32>(w(0x51), 0x3f80_0000);
    for (k, x) in (0u32..).zip(v) {
        g.write::<u32>(w(0x5c) + 4 * k, x);
    }
    for at in [0x62, 0x6c, 0x76, 0x80, 0x8a, 0x94, 0x9e] {
        g.write::<u32>(w(at), w(at + 4));
    }
    Ret::eax(1)
}

/// [`start`] (172 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 172,
    hash: 0xea6b_0fd8_fb1e_8037,
    calls: &[(0x31, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The part's draw (0x100017d0, no arguments): `**P` (`P` `+0x01`) = the
/// block `+0x0e`, slot 723 `(*A)` (`A` `+0x08`), slot 724 `(*B)` (`B`
/// `+0x26`), slot 258 `(T)` (`T` the immediate `+0x38`), slot 231 `(*D,
/// *B, *C)` (`D` `+0x5d`, `B` again `+0x51`, `C` `+0x46`), slot 230 `(*E,
/// *D)` (`E` `+0x70`, `D` again `+0x65`), `**P` again (`+0x7d`/`+0x85`)
/// and slot 712 `(T, 0x30)` (`T` `+0x8f`) with the game flag `*L` (`L` =
/// `*Q`, `+0x8a`, again `+0xa9`) 1 around it. Returns 1. The slot calls
/// pass the words the original leaves on the stack.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let mut st = crate::pushes::Pushes::default();
    st.push(g.read::<u32>(w(0x08)));
    g.write::<u32>(p, w(0x0e));
    let s = slot_target(g, w(0x13), 0xb4c / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push(g.read::<u32>(w(0x26)));
    let s = slot_target(g, w(0x20), 0xb50 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push(w(0x38));
    let s = slot_target(g, w(0x33), 0x408 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let c = g.read::<u32>(w(0x46));
    let s = slot_target(g, w(0x4b), 0x39c / 4);
    let b = g.read::<u32>(w(0x51));
    let d = g.read::<u32>(w(0x5d));
    st.push_all(&[c, b, d]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let d = g.read::<u32>(w(0x65));
    let s = slot_target(g, w(0x6a), 0x398 / 4);
    let e = g.read::<u32>(w(0x70));
    st.push_all(&[d, e]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let p = g.read::<u32>(w(0x7d));
    st.push(0x30);
    g.write::<u32>(p, w(0x85));
    let lock = g.read::<u32>(w(0x8a));
    st.push(w(0x8f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s = slot_target(g, w(0x9c), 0xb20 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let lock = g.read::<u32>(w(0xa9));
    g.write::<u32>(lock, saved);
    Ret::eax(1)
}

/// [`draw`] (185 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 185,
    hash: 0x1cc9_adf8_8389_a5f5,
    calls: &[],
    imports: &[],
};

/// The frame callback (0x10001d80, no arguments): nothing (0) until
/// started (`*S`, `+0x02`); else `**P` (`P` `+0x0e`) = `*V` (`+0x13`) and
/// `*W` (`+0x1f`) = `*U` (`+0x1a`); for the flags 0, 1, 2 that slot 261
/// answers set the timelines `+0x3a`, `+0x5a`, `+0x7a` run
/// ([`timeline_run`](crate::sb_misc::timeline_run)); while flag 3 is
/// clear the frame count `*N` (`+0xd5`) goes up, else the three timelines
/// stop ([`timeline_stop`](crate::sb_misc::timeline_stop)) and slot 214
/// `(*X)` (`X` `+0xbe`) runs. Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x0e));
    let v = g.read::<u32>(w(0x13));
    g.write::<u32>(p, v);
    let u = g.read::<u32>(w(0x1a));
    g.write::<u32>(w(0x1f), u);
    for (flag, table, tl, call) in [
        (0u32, 0x24, 0x3a, 0x3f),
        (1, 0x44, 0x5a, 0x5f),
        (2, 0x64, 0x7a, 0x7f),
    ] {
        let s = slot_target(g, w(table), 0x414 / 4);
        if g.call(s, 0, &[flag]).eax != 0 {
            crate::sb_misc::timeline_run(g, img, img.rel32_target(f + call), w(tl));
        }
    }
    let s = slot_target(g, w(0x84), 0x414 / 4);
    if g.call(s, 0, &[3]).eax == 0 {
        let n = g.read::<u32>(w(0xd5));
        g.write::<u32>(w(0xd5), n.wrapping_add(1));
        return Ret::eax(1);
    }
    for tl in [0x9a, 0xa4, 0xae] {
        crate::sb_misc::timeline_stop(g, w(tl));
    }
    let s = slot_target(g, w(0xb8), 0x358 / 4);
    let x = g.read::<u32>(w(0xbe));
    g.call(s, 0, &[x]);
    Ret::eax(1)
}

/// [`frame`] (223 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 223,
    hash: 0x06cd_c0f7_50ef_274d,
    calls: &[
        (0x3f, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x5f, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x7f, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x9f, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xa9, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xb3, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// A part's start (0x10001000, cdecl `(part)`): `**P` (`P` `+0x04`) = the
/// block `+0x0b`; slot 941 `((*L)[4])`'s list (`L` `+0x10`) gives the
/// program (its `+0xc`), started at the part
/// ([`start_part`](crate::opu_parts::start_part) `(part, 0x1000, program,
/// 1)`, `+0x36`); the part's `+0x4c` = 1.0, `+0x50`/`+0x58` 0, `+0x5c` 1,
/// `+0x6c` 1.0, and the position, colour and size words (`+0x40..+0x48`,
/// `+0x54`, `+0x60..+0x68`) one of two sets: the first for a part within
/// 0x80 bytes of the table `+0x3d`. Returns the part's offset from that
/// table.
pub fn part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    g.write::<u32>(p, w(0x0b));
    let l = g.read::<u32>(w(0x10));
    let s = slot_target(g, w(0x18), 0xeb4 / 4);
    let x = g.read::<u32>(l + 0x10);
    let list = g.call(s, 0, &[x]).eax;
    let part = fr.arg(g, 0);
    let prog = g.read::<u32>(list + 0xc);
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x36), part, 0x1000, prog, 1);
    let off = part.wrapping_sub(w(0x3d));
    for (o, v) in [
        (0x4c, 0x3f80_0000),
        (0x50, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(part + o, v);
    }
    let set: [u32; 7] = if off & 0xffff_ff80 == 0 {
        [
            0x41a3_4d0f,
            0xc20f_c5e4,
            0xc30e_7724,
            0xffff_aa00,
            0x3ed3_32ad,
            0x3ed3_32ad,
            0x3ed3_32ad,
        ]
    } else {
        [
            0x41dd_c4da,
            0xc1d2_307c,
            0xc338_51c9,
            0,
            0x3f39_36c6,
            0x3f39_36c6,
            0x3f39_36c6,
        ]
    };
    for (o, v) in [0x40, 0x44, 0x48, 0x54, 0x60, 0x64, 0x68]
        .into_iter()
        .zip(set)
    {
        g.write::<u32>(part + o, v);
    }
    Ret::eax(off)
}

/// [`part_start`] (214 bytes, 1 overlay).
pub const PART_START: Sig = Sig {
    len: 214,
    hash: 0x5233_224e_5267_09b7,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
