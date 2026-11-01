//! `magic_0313.dll`'s own functions (besides the runtime, the object
//! host and the shared code). Every slot call passes the words the
//! original leaves on the stack.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::div;
use crate::{Frame, Game, Image, Ret, Sig};

/// Slot `slot` (table operand `table`) on the words pushed so far.
fn call<G: Game>(g: &mut G, img: &Image, f: u32, p: &Pushes, table: u32, slot: u32) -> u32 {
    let s = slot_target(g, img.word(f + table), slot);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0).eax
}

/// The object's rotation `+0x100..+0x10c` = `(0, y, 0, 0)`.
fn turn<G: Game>(g: &mut G, obj: u32, y: u32) {
    g.write::<u32>(obj + 0x100, 0);
    g.write::<u32>(obj + 0x104, y);
    g.write::<u32>(obj + 0x108, 0);
    g.write::<u32>(obj + 0x10c, 0);
}

/// An object's start (0x10001400, cdecl `(obj)`): slots 444 `(0)` and
/// 445 `(1)`; the rotation from the angle `*A` (`+0x42`); slot 440 `(*C,
/// obj, 0, 1)` (`C` `+0x49`) with the step count `+0x7e` cleared; slots
/// 450 `(obj, F)` (`+0x71`) and 451 `(obj, 10)`. Returns slot 451's
/// `eax`.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = Pushes::default();
    p.push(0);
    call(g, img, f, &p, 0x04, 0x6f0 / 4);
    p.push(1);
    call(g, img, f, &p, 0x14, 0x6f4 / 4);
    let obj = fr.arg(g, 0);
    p.push_all(&[1, 0]);
    g.write::<u32>(obj + 0x100, 0);
    let a = g.read::<u32>(w(0x42));
    p.push_all(&[obj, g.read::<u32>(w(0x49))]);
    g.write::<u32>(obj + 0x104, a);
    g.write::<u32>(obj + 0x108, 0);
    g.write::<u32>(obj + 0x10c, 0);
    call(g, img, f, &p, 0x26, 0x6e0 / 4);
    p.push_all(&[w(0x71), obj]);
    g.write::<u32>(w(0x7e), 0);
    call(g, img, f, &p, 0x6c, 0x708 / 4);
    p.push_all(&[0xa, obj]);
    Ret::eax(call(g, img, f, &p, 0x89, 0x70c / 4))
}

/// [`object_start`] (158 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 158,
    hash: 0x2d4f_7a2c_c44e_e5b8,
    calls: &[],
    imports: &[],
};

/// One turning step (0x100014a0 and 0x10001570, cdecl `(obj)`, a local
/// at `ebp - 4`): slot 482 `(a, b, k / K, 1)`'s float (`k` the step
/// count `*S`, `K` at `+0x18`; `a`, `b` the angles `+0x40` and `+0x32`,
/// swapped in the second) is the object's rotation; the count goes up;
/// at 60 the object moves on (slots 450 `(obj, F1)` and 449 `(obj, F2)`;
/// the first function also restarts the count). Returns slot 449's `eax`,
/// else the new count.
fn turn_step<G: Game>(g: &mut G, img: &Image, fr: &Frame, back: bool) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    #[allow(clippy::cast_precision_loss)]
    let k = g.read::<i32>(w(0x08)) as f32;
    let t = div(k, g.read::<f32>(w(0x18)));
    let mut p = Pushes::default();
    p.push(1);
    let b = g.read::<u32>(w(0x32));
    let a = g.read::<u32>(w(0x40));
    p.push_all(&[t.to_bits(), b, a]);
    let s = slot_target(g, w(0x10), 0x788 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let y = p.call(g, s, 0).st0.unwrap_or(0.0) as f32;
    // fstp [ebp - 4]: the local
    g.write::<f32>(fr.esp.wrapping_sub(8), y);
    let obj = fr.arg(g, 0);
    let n = g.read::<u32>(w(0x52)).wrapping_add(1);
    turn(g, obj, y.to_bits());
    g.write::<u32>(w(0x86), n);
    if n.cast_signed() < 0x3c {
        return Ret::eax(n);
    }
    p.pop(4);
    p.push_all(&[w(0x95), obj]);
    if !back {
        g.write::<u32>(w(0xa2), 0);
    }
    call(g, img, f, &p, 0x90, 0x708 / 4);
    let f2 = if back { 0xa8 } else { 0xb2 };
    p.push_all(&[w(f2), obj]);
    Ret::eax(call(g, img, f, &p, f2 - 5, 0x704 / 4))
}

/// [`turn_step`] there (0x100014a0).
pub fn turn_there<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    turn_step(g, img, fr, false)
}

/// [`turn_step`] back (0x10001570; the angles swapped, the count kept at
/// 60).
pub fn turn_back<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    turn_step(g, img, fr, true)
}

/// [`turn_there`] (199 bytes, 1 overlay).
pub const TURN_THERE: Sig = Sig {
    len: 199,
    hash: 0xa786_2966_990f_811d,
    calls: &[],
    imports: &[],
};

/// [`turn_back`] (189 bytes, 1 overlay).
pub const TURN_BACK: Sig = Sig {
    len: 189,
    hash: 0x91ad_ea71_7e81_55db,
    calls: &[],
    imports: &[],
};

/// An object's mark (0x10001a40, cdecl `(obj)`): with `m` = `C[0x1a0 +
/// 4 c]` (`C` = `*G`, `+0x17`; `c` the object's current list word):
/// slots 442 `(obj, m, 6)` and 443 `(obj, 0x10)`, the scale
/// `+0x130..+0x138` 0.25 (`+0x13c` 0), slot 446 `(m, 0)`, slot 440 `(*G,
/// obj, 0, 0)`, slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0x97`,
/// `+0xaa`). Returns slot 449's `eax`.
pub fn mark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    p.push(6);
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    let c = g.read::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c));
    let m = g.read::<u32>(
        g.read::<u32>(w(0x17))
            .wrapping_add(c.wrapping_mul(4))
            .wrapping_add(0x1a0),
    );
    p.push_all(&[m, obj]);
    call(g, img, f, &p, 0x23, 0x6e8 / 4);
    p.push_all(&[0x10, obj]);
    call(g, img, f, &p, 0x32, 0x6ec / 4);
    for (off, v) in [
        (0x130, 0x3e80_0000),
        (0x134, 0x3e80_0000),
        (0x138, 0x3e80_0000),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    p.push_all(&[0, m]);
    call(g, img, f, &p, 0x42, 0x6f8 / 4);
    p.push_all(&[0, 0, obj, g.read::<u32>(w(0x8b))]);
    call(g, img, f, &p, 0x7a, 0x6e0 / 4);
    p.push_all(&[w(0x97), obj]);
    call(g, img, f, &p, 0x92, 0x708 / 4);
    p.push_all(&[w(0xaa), obj]);
    Ret::eax(call(g, img, f, &p, 0xa5, 0x704 / 4))
}

/// [`mark`] (190 bytes, 1 overlay).
pub const MARK: Sig = Sig {
    len: 190,
    hash: 0x3582_5cc8_c5dd_292b,
    calls: &[],
    imports: &[],
};

/// An effect object's set-up (0x100016f0, cdecl `(a)`): `a + 0x100` 0,
/// `+0x104` the float at `+0x24`, `+0x108`/`+0x10c` 0; slot 440 `(*G_E,
/// a, 0, 2)` (operand `+0x2b`); `*G_F` (`+0x60`) 0; slot 450 `(a, F)` (`F`
/// the immediate `+0x53`; the first call's words under it). Returns slot
/// 450's `eax`.
pub fn object_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let a = fr.arg(g, 0);
    g.write::<u32>(a + 0x100, 0);
    let k = g.read::<u32>(w(0x24));
    g.write::<u32>(a + 0x104, k);
    g.write::<u32>(a + 0x108, 0);
    g.write::<u32>(a + 0x10c, 0);
    let e = g.read::<u32>(w(0x2b));
    let mut s = Pushes::default();
    s.push_all(&[2, 0, a, e]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x4), 440), fr.ecx);
    g.write::<u32>(w(0x60), 0);
    s.push_all(&[w(0x53), a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0x4e), 450), fr.ecx).eax)
}

/// [`object_setup`] (112 bytes, 1 overlay).
pub const OBJECT_SETUP: Sig = Sig {
    len: 112,
    hash: 0xd0fc_82a0_3c32_6df0,
    calls: &[],
    imports: &[],
};
