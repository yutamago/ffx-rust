//! `magic_0361.dll`'s own functions (besides the runtime, the object
//! host and the shared code). Every slot call passes the words the
//! original leaves on the stack.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Slot `slot` (table operand `table`) on the words pushed so far.
fn call<G: Game>(g: &mut G, img: &Image, f: u32, p: &Pushes, table: u32, slot: u32) -> u32 {
    let s = slot_target(g, img.word(f + table), slot);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0).eax
}

/// The objects' last two slots and the list word: 450 `(obj, F1)`, 449
/// `(obj, F2)`, then the object's current list word gets `m`. Returns
/// twice the list index.
#[allow(clippy::too_many_arguments)]
fn close_listed<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    p: &mut Pushes,
    obj: u32,
    m: u32,
    (t450, f1): (u32, u32),
    (t449, f2): (u32, u32),
) -> u32 {
    p.push_all(&[img.word(f + f1), obj]);
    call(g, img, f, p, t450, 0x708 / 4);
    p.push_all(&[img.word(f + f2), obj]);
    call(g, img, f, p, t449, 0x704 / 4);
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), m);
    i
}

/// A counting step (0x100014e0, cdecl `(obj)`): with `m` = `(*A)[0x198]`
/// (`A` `+0x04`): slot 444 `(1)`; the current list word of the object
/// `*O` (`+0x21`) goes up; slot 434 `(*A, F)` (`+0x26`); slots 450 and
/// 449 (`+0x58`, `+0x65`); the list word gets `m`. Returns twice the list
/// index.
pub fn count_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x198);
    let mut p = Pushes::default();
    p.push(1);
    call(g, img, f, &p, 0x11, 0x6f0 / 4);
    let o = g.read::<u32>(w(0x21));
    p.push_all(&[w(0x26), g.read::<u32>(w(0x32))]);
    let i = g.read::<u32>(o + 0xac).wrapping_mul(2);
    let at = o.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c);
    let v = g.read::<u32>(at).wrapping_add(1);
    g.write::<u32>(at, v);
    call(g, img, f, &p, 0x3d, 0x6c8 / 4);
    let obj = fr.arg(g, 0);
    Ret::eax(close_listed(
        g,
        img,
        f,
        &mut p,
        obj,
        m,
        (0x4a, 0x58),
        (0x60, 0x65),
    ))
}

/// [`count_step`] (133 bytes, 1 overlay).
pub const COUNT_STEP: Sig = Sig {
    len: 133,
    hash: 0xff94_fe7e_8cf7_d968,
    calls: &[],
    imports: &[],
};

/// The next step (0x10001570, cdecl `(obj)`): slot 432 `()`; slot 486
/// `(m, 1, 0, 0, 0x1e, 0x20, 0x20, 0, 0, 0, 0)`; slots 450 and 449
/// (`+0x6e`, `+0x7b`); the list word gets `m`. Returns twice the list
/// index.
pub fn next_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let m = g.read::<u32>(g.read::<u32>(img.word(f + 0x04)) + 0x198);
    let mut p = Pushes::default();
    call(g, img, f, &p, 0x11, 0x6c0 / 4);
    p.push_all(&[0, 0, 0, 0]);
    p.push_all(&[0x20, 0x20, 0x1e, 0, 0, 1, m]);
    call(g, img, f, &p, 0x1e, 0x798 / 4);
    let obj = fr.arg(g, 0);
    Ret::eax(close_listed(
        g,
        img,
        f,
        &mut p,
        obj,
        m,
        (0x60, 0x6e),
        (0x76, 0x7b),
    ))
}

/// [`next_step`] (155 bytes, 1 overlay).
pub const NEXT_STEP: Sig = Sig {
    len: 155,
    hash: 0x68df_387b_c50f_b30e,
    calls: &[],
    imports: &[],
};

/// The effect's start (0x10001000, cdecl `(task)`): with `E` = `*G`
/// (`+0x04`), `a` = `E + 0x198` and `b` = `E + 0x1a0`: `*R` (`+0x25`) =
/// slot 493 `(b)`, `*Z` (`+0x37`) = 0, `*K` (`+0x6d`) = whether slot 390
/// `(a)` answers 0x1041, `*H` (`+0x74`) = slot 367 `(E' + 4, b, 0, 0, 0,
/// 0, 0, 0)` (`E'` = `*G` again, `+0x51`); slot 448 `(2, 0)`, slot 450
/// `(task, F)` and slot 449 `(task, F2)` (`F`/`F2` the callbacks `+0x98`/
/// `+0xa5`). Returns slot 449's `eax`. Each slot call passes the words the
/// original leaves on the stack.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let e = g.read::<u32>(w(0x04));
    let a = g.read::<u32>(e + 0x198);
    let b = g.read::<u32>(e + 0x1a0);
    let mut st = Pushes::default();
    st.push(b);
    let s = slot_target(g, w(0x17), 0x7b4 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, s, 0).eax;
    g.write::<u32>(w(0x25), r);
    st.push(a);
    let s = slot_target(g, w(0x2a), 0x618 / 4);
    g.write::<u32>(w(0x37), 0);
    // call width: 12 (Pushes caps the words at 12)
    let kind = st.call(g, s, 0).eax;
    let e2 = g.read::<u32>(w(0x51));
    st.push_all(&[0, 0, 0, 0, 0, 0, b, g.read::<u32>(e2 + 4)]);
    let s = slot_target(g, w(0x61), 0x5bc / 4);
    g.write::<u32>(w(0x6d), u32::from(kind == 0x1041));
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, s, 0).eax;
    g.write::<u32>(w(0x74), h);
    st.push_all(&[0, 2]);
    let s = slot_target(g, w(0x79), 0x700 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let task = fr.arg(g, 0);
    let s = slot_target(g, w(0x8a), 0x708 / 4);
    st.push_all(&[w(0x98), task]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push_all(&[w(0xa5), task]);
    let s = slot_target(g, w(0xa0), 0x704 / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(st.call(g, s, 0).eax)
}

/// [`start`] (185 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 185,
    hash: 0x3532_55cf_e0b8_3dee,
    calls: &[],
    imports: &[],
};
