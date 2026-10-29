//! `magic_0196.dll`'s effect wrappers: straight-line sequences of slot calls
//! that hand the effect's callbacks (overlay functions, by address) to the
//! game (docs/formats/magic.md, "Porting the overlays"). Each reads its
//! operands from this DLL's instructions (`img.word(f + off)`: the context
//! global, the table global and the callbacks); the comments give
//! `magic_0196`'s addresses. A call sees the words pushed before it
//! ([`Pushes`]); the callee's result of the last call is the function's.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// 0x10001040 (65 bytes, 6 overlays): slot 433 with `1`; slot 450 with
/// (`arg`, the callback at 0x10001090); slot 449 with (`arg`, the callback
/// at 0x10001d00); returns slot 449's `eax`.
pub fn e1040<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let mut s = Pushes::default();
    s.push(1);
    let slot433 = slot_target(g, w(0x04), 433);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot433, fr.ecx);
    s.push(w(0x18));
    s.push(arg);
    let slot450 = slot_target(g, w(0x13), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(w(0x2d));
    s.push(arg);
    let slot449 = slot_target(g, w(0x28), 449);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot449, fr.ecx);
    Ret::eax(out.eax)
}

/// 0x10001040 (65 bytes, 6 overlays).
pub const E1040: Sig = Sig {
    len: 65,
    hash: 0xcbb8_0648_accd_cec2,
    calls: &[],
    imports: &[],
};

/// 0x10001090 (94 bytes, 2 overlays): slot 473 with (0, 0, 0, 0x40, 0xf);
/// slot 434 with (the word at the global `0x10085a50`, the callback at
/// 0x100011d0); slot 450 with (`arg`, the callback at 0x100010f0); slot 451
/// with (`arg`, 0x1e); returns slot 451's `eax`.
pub fn e1090<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let mut s = Pushes::default();
    s.push(0xf);
    s.push(0x40);
    s.push(0);
    s.push(0);
    s.push(0);
    let slot473 = slot_target(g, w(0x04), 473);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot473, fr.ecx);
    s.push(w(0x20));
    let value = g.read::<u32>(w(0x26));
    s.push(value);
    let slot434 = slot_target(g, w(0x1b), 434);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot434, fr.ecx);
    s.push(w(0x38));
    s.push(arg);
    let slot450 = slot_target(g, w(0x33), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(0x1e);
    s.push(arg);
    let slot451 = slot_target(g, w(0x48), 451);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot451, fr.ecx);
    Ret::eax(out.eax)
}

/// 0x10001090 (94 bytes, 2 overlays).
pub const E1090: Sig = Sig {
    len: 94,
    hash: 0xe8d2_e932_49bf_c718,
    calls: &[],
    imports: &[],
};

/// 0x100010f0 (76 bytes, 8 overlays): slot 446 with (the word at `ctx +
/// 0x1a0`, 0), where `ctx` is the global at `0x10085a50`'s value; slot 450
/// with (`arg`, the callback at 0x10001140); slot 449 with (`arg`, the
/// callback at 0x10001d20); returns slot 449's `eax`.
pub fn e10f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let ctx = g.read::<u32>(w(0x04));
    let mut s = Pushes::default();
    s.push(0);
    let word = g.read::<u32>(ctx + 0x1a0);
    s.push(word);
    let slot446 = slot_target(g, w(0x11), 446);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot446, fr.ecx);
    s.push(w(0x23));
    s.push(arg);
    let slot450 = slot_target(g, w(0x1e), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(w(0x38));
    s.push(arg);
    let slot449 = slot_target(g, w(0x33), 449);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot449, fr.ecx);
    Ret::eax(out.eax)
}

/// 0x100010f0 (76 bytes, 8 overlays).
pub const E10F0: Sig = Sig {
    len: 76,
    hash: 0xf67f_fc7f_4183_9cde,
    calls: &[],
    imports: &[],
};

/// 0x10001140 (63 bytes, 2 overlays): slot 472 with no arguments; slot 450
/// with (`arg`, the callback at 0x10001180); slot 449 with (`arg`, the
/// callback at 0x10001d60); returns slot 449's `eax`.
pub fn e1140<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let mut s = Pushes::default();
    let slot472 = slot_target(g, w(0x04), 472);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot472, fr.ecx);
    s.push(w(0x16));
    s.push(arg);
    let slot450 = slot_target(g, w(0x11), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(w(0x2b));
    s.push(arg);
    let slot449 = slot_target(g, w(0x26), 449);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot449, fr.ecx);
    Ret::eax(out.eax)
}

/// 0x10001140 (63 bytes, 2 overlays).
pub const E1140: Sig = Sig {
    len: 63,
    hash: 0xe380_238d_b4cf_c294,
    calls: &[],
    imports: &[],
};

/// 0x100011d0 (173 bytes, 2 overlays): `obj` is `arg`. Slot 442 with
/// (`obj`, the word at `ctx + 0x1a0`, 3); slot 443 with (`obj`, 0x40); the
/// object's floats at `+0x130`, `+0x134` and `+0x138` are 0.07 (0x3d8f5c29)
/// and `+0x13c` is 0; slot 444 with 0; slot 440 with (the global at
/// `0x10085a50`'s value, `obj`, 0, 0); slot 450 with (`obj`, the callback at
/// 0x100011c0); slot 449 with (`obj`, the callback at 0x10001cc0); returns
/// slot 449's `eax`. `ctx` is the global at `0x10085a50`'s value.
pub fn e11d0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let ctx = g.read::<u32>(w(0x04));
    let mut s = Pushes::default();
    s.push(3);
    let word = g.read::<u32>(ctx + 0x1a0);
    s.push(word);
    s.push(obj);
    let slot442 = slot_target(g, w(0x15), 442);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot442, fr.ecx);
    s.push(0x40);
    s.push(obj);
    let slot443 = slot_target(g, w(0x23), 443);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot443, fr.ecx);
    g.write::<u32>(obj + 0x130, 0x3d8f_5c29);
    g.write::<u32>(obj + 0x134, 0x3d8f_5c29);
    g.write::<u32>(obj + 0x138, 0x3d8f_5c29);
    g.write::<u32>(obj + 0x13c, 0);
    s.push(0);
    let slot444 = slot_target(g, w(0x33), 444);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot444, fr.ecx);
    s.push(0);
    s.push(0);
    s.push(obj);
    let value = g.read::<u32>(w(0x7b));
    s.push(value);
    let slot440 = slot_target(g, w(0x6a), 440);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot440, fr.ecx);
    s.push(w(0x87));
    s.push(obj);
    let slot450 = slot_target(g, w(0x82), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(w(0x9a));
    s.push(obj);
    let slot449 = slot_target(g, w(0x95), 449);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot449, fr.ecx);
    Ret::eax(out.eax)
}

/// 0x100011d0 (173 bytes, 2 overlays).
pub const E11D0: Sig = Sig {
    len: 173,
    hash: 0xe1b9_faf9_85a7_876d,
    calls: &[],
    imports: &[],
};
