//! `magic_0196.dll`'s effect wrappers: straight-line sequences of slot calls
//! that hand the effect's callbacks (overlay functions, by address) to the
//! game (docs/formats/magic.md, "Porting the overlays"). Each reads its
//! operands from this DLL's instructions (`img.word(f + off)`: the context
//! global, the table global and the callbacks); the comments give
//! `magic_0196`'s addresses. A call sees the words pushed before it
//! ([`Pushes`]); the callee's result of the last call is the function's.
//! The wrappers at 0x1040, 0x1090, 0x10f0 and 0x1140 are the map effects'
//! task functions ([`crate::rcef`]: `effect_ready_seq`,
//! `main_create_task_wait`, `damage_seq`, `task_seq_no_args`).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

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
