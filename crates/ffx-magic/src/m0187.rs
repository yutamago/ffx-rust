//! The functions whose lowest DLL is `magic_0187` (docs/formats/magic.md,
//! "Porting the overlays"). Addresses are `magic_0187`'s; each reads its
//! operands from the instructions (the context global at `0x10069230`, the
//! table global at `0x10069248`, the callbacks). A call sees the words
//! pushed before it ([`Pushes`]); the saved registers (`push esi/edi`) lie
//! below every call's arguments and are not modelled. The wrappers at
//! 0x1210 and 0x1420 are the map effects' task functions
//! ([`crate::rcef`]: `set_seq_call_seq`, `main_init_create_task`).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// 0x10001000 (67 bytes, 3 overlays): slot 448 with (2, 0); slot 450 with
/// (`arg`, the callback at 0x10001050); slot 449 with (`arg`, the callback
/// at 0x10001cc0); returns slot 449's `eax`.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let mut s = Pushes::default();
    s.push(0);
    s.push(2);
    let slot448 = slot_target(g, w(0x04), 448);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot448, fr.ecx);
    s.push(w(0x1a));
    s.push(arg);
    let slot450 = slot_target(g, w(0x15), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(w(0x2f));
    s.push(arg);
    let slot449 = slot_target(g, w(0x2a), 449);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot449, fr.ecx);
    Ret::eax(out.eax)
}

/// 0x10001000 (67 bytes, 3 overlays).
pub const E1000: Sig = Sig {
    len: 67,
    hash: 0xc266_1781_24aa_5fd6,
    calls: &[],
    imports: &[],
};

/// 0x10001050 (65 bytes, 1 overlay): slot 433 with `0`; slot 450 with
/// (`arg`, the callback at 0x100010a0); slot 449 with (`arg`, the callback
/// at 0x10001ce0); returns slot 449's `eax`.
pub fn e1050<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let mut s = Pushes::default();
    s.push(0);
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

/// 0x10001050 (65 bytes, 1 overlay).
pub const E1050: Sig = Sig {
    len: 65,
    hash: 0x5992_4792_6766_dc0b,
    calls: &[],
    imports: &[],
};

/// 0x100010a0 (76 bytes, 1 overlay): `ctx` is the global at `0x10069230`'s
/// value and `v` the word at `ctx + 0x198`. Slot 447 with (`v`, 0xba); slot
/// 450 with (`arg`, the callback at 0x100010f0); slot 451 with (`arg`,
/// 0x14); returns slot 451's `eax`.
pub fn e10a0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let ctx = g.read::<u32>(w(0x04));
    let mut s = Pushes::default();
    s.push(0xba);
    let v = g.read::<u32>(ctx + 0x198);
    s.push(v);
    let slot447 = slot_target(g, w(0x14), 447);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot447, fr.ecx);
    s.push(w(0x26));
    s.push(arg);
    let slot450 = slot_target(g, w(0x21), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(0x14);
    s.push(arg);
    let slot451 = slot_target(g, w(0x36), 451);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot451, fr.ecx);
    Ret::eax(out.eax)
}

/// 0x100010a0 (76 bytes, 1 overlay).
pub const E10A0: Sig = Sig {
    len: 76,
    hash: 0xbc1e_5c49_19b9_46b4,
    calls: &[],
    imports: &[],
};

/// 0x10001170 (95 bytes, 2 overlays): `obj` is `arg` and `ctx` the global
/// at `0x10069230`'s value; `v` is the word at `ctx + 0x198`. Slot 447 with
/// (`v`, 0xbc); slot 450 with (`obj`, the callback at 0x100011d0); slot 449
/// with (`obj`, the callback at 0x10001c80). Then `d = 2 * [obj + 0xac]` (the
/// word after the calls), `[obj + 8 * d + 0x2c] = v`, and the result is `d`.
pub fn e1170<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ctx = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(ctx + 0x198);
    let obj = fr.arg(g, 0);
    let mut s = Pushes::default();
    s.push(0xbc);
    s.push(v);
    let slot447 = slot_target(g, w(0x11), 447);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot447, fr.ecx);
    s.push(w(0x32));
    s.push(obj);
    let slot450 = slot_target(g, w(0x24), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(w(0x3f));
    s.push(obj);
    let slot449 = slot_target(g, w(0x3a), 449);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot449, fr.ecx);
    let word = g.read::<u32>(obj + 0xac);
    let doubled = word.wrapping_add(word);
    g.write::<u32>(
        obj.wrapping_add(doubled.wrapping_mul(8)).wrapping_add(0x2c),
        v,
    );
    Ret::eax(doubled)
}

/// 0x10001170 (95 bytes, 2 overlays).
pub const E1170: Sig = Sig {
    len: 95,
    hash: 0x2fff_ddda_5b7c_48dc,
    calls: &[],
    imports: &[],
};

/// 0x100010f0 (119 bytes, 1 overlay): `obj` is `arg`, `ctx` the global at
/// `0x10069230`'s value and `v` the word at `ctx + 0x198`. Slot 434 with
/// (`ctx`, the callback at 0x10001260); slot 434 with (the global's value,
/// 0x10001360); slot 450 with (`obj`, the callback at 0x10001170); slot 449
/// with (`obj`, the callback at 0x10001c80). Then `d = 2 * [obj + 0xac]`
/// (the word after the calls), `[obj + 8 * d + 0x2c] = v`, and the result is
/// `d`.
pub fn e10f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ctx = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(ctx + 0x198);
    let mut s = Pushes::default();
    s.push(w(0x11));
    s.push(ctx);
    let slot434 = slot_target(g, w(0x17), 434);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot434, fr.ecx);
    s.push(w(0x29));
    let ctx_now = g.read::<u32>(w(0x2f));
    s.push(ctx_now);
    let slot434_again = slot_target(g, w(0x24), 434);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot434_again, fr.ecx);
    let obj = fr.arg(g, 0);
    s.push(w(0x4a));
    s.push(obj);
    let slot450 = slot_target(g, w(0x3c), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(w(0x57));
    s.push(obj);
    let slot449 = slot_target(g, w(0x52), 449);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot449, fr.ecx);
    let word = g.read::<u32>(obj + 0xac);
    let doubled = word.wrapping_add(word);
    g.write::<u32>(
        obj.wrapping_add(doubled.wrapping_mul(8)).wrapping_add(0x2c),
        v,
    );
    Ret::eax(doubled)
}

/// 0x100010f0 (119 bytes, 1 overlay).
pub const E10F0: Sig = Sig {
    len: 119,
    hash: 0xd850_b82a_7f69_1970,
    calls: &[],
    imports: &[],
};
