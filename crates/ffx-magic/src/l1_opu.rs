//! Variants of the OPU host (`opu.rs`, `opu_parts.rs`, `m0604.rs`) that
//! list 1's overlays carry: the same code with one change each.
//! Addresses are `magic_0021`'s (the lowest DLL carrying them).

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x0` (`magic_0021` 0x10001690, cdecl `(arg)`): `m0604`'s
/// start-up ([`crate::m0604::e1660`]: the OPU object's memory, the effect
/// data, `op_oef2_set2`/`op_oef2_a_get`, the object from slot 715) ending
/// in a tail jump to the set-up ([`opu::setup`], `ecx` the object)
/// instead of slot 816: the set-up's `this + 0x21200` is what the caller
/// gets.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let slot803 = slot_target(g, w(0x04), 803);
    let ctrl = g.read::<u32>(w(0x13));
    let buffer = g.call(slot803, fr.ecx, &[0xfa000, 0x10]).eax;
    let flags = g.read::<u32>(w(0x22));
    let slots = g.read::<u32>(w(0x28)).wrapping_add(4);
    g.write::<u32>(w(0x33), buffer);
    let data = if g.read::<u8>(flags + 0x10) != 0 && g.read::<u8>(ctrl + 0x40) & 0x10 != 0 {
        g.read::<u32>(ctrl + 0xa8)
    } else {
        w(0x52)
    };
    g.write::<u32>(slots, data);
    let data = g.read::<u32>(slots);
    let slot718 = slot_target(g, w(0x57), 718);
    g.call(slot718, fr.ecx, &[data]);
    let slot719 = slot_target(g, w(0x67), 719);
    let entry = g.call(slot719, fr.ecx, &[data, 1, 0, data]).eax;
    let slot715 = slot_target(g, w(0x86), 715);
    // the pushes of the earlier calls stay on the stack: 11 words
    let obj = g
        .call(
            slot715,
            fr.ecx,
            &[1, entry, data, buffer, 0x200, 0, arg, data, 1, 0, data],
        )
        .eax;
    Ret::eax(opu::setup(g, img, img.rel32_target(f + 0x9e), obj))
}

/// `start_up` (162 bytes; `magic_0021`-`0023`, `0326`).
pub const START_UP: Sig = Sig {
    len: 162,
    hash: 0x9789_d9b3_c626_170b,
    calls: &[(0x9e, Callee::Sig(&opu::SETUP))],
    imports: &[],
};

/// `+0x40` (`magic_0021` 0x10001740, `(part)`): `pppInitData` for a part
/// the first time its data is seen ([`crate::opu_parts::init_data`]),
/// except that a part whose entry was already recorded returns its data
/// pointer (the `jne` skips the `xor eax, eax`), not 0.
pub fn init_data<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = fr.arg(g, 0);
    let data = g.read::<u32>(part + 0xd0);
    // slot 941 of the data's +0x14, +0x18, +0x1c into G_M0..G_M2; each
    // call sees the earlier pushes after its own word
    let mut pushed = Vec::with_capacity(3);
    for (t, dst) in [(0x04, 0x20), (0x25, 0x35), (0x3a, 0x4a)] {
        let slot = slot_target(g, w(t), 941);
        let k = pushed.len() as u32;
        let mut args = vec![g.read::<u32>(data + 0x14 + 4 * k)];
        args.extend(pushed.iter().rev());
        // call width: 3 (the word and the two earlier pushes at most)
        let r = g.call(slot, 0, &args).eax;
        pushed.push(args[0]);
        g.write::<u32>(w(dst), r);
    }
    // *G_ENVP = G_ENV
    let envp = g.read::<u32>(w(0x4f));
    g.write::<u32>(envp, w(0x58));
    let slots = g.read::<u32>(w(0x5d));
    let want = g.read::<u32>(part + 0xd0);
    let effect = g.read::<u32>(slots + 4);
    if effect == 0 || g.read::<u16>(effect + 0x50) == 0 {
        return Ret::eax(0);
    }
    let mut i = 0u32;
    loop {
        let slot719 = slot_target(g, w(0x81), 719);
        if g.call(slot719, 0, &[effect, 8, i]).eax == want {
            break;
        }
        let n = u32::from(g.read::<u16>(effect + 0x50));
        i += 1;
        if i.cast_signed() >= n.cast_signed() {
            return Ret::eax(0);
        }
    }
    // (test esi, esi; js: i is never negative here)
    if g.read::<u32>(w(0xaf) + 4 * i) != 0 {
        // jne past the xor: eax is still the entry slot 719 returned
        return Ret::eax(want);
    }
    let program = g.read::<u32>(w(0xbc) + 4 * i);
    let d = g.read::<u32>(part + 0xd0);
    g.write::<u32>(w(0xc9) + 4 * i, d);
    let lock = g.read::<u32>(w(0xce));
    let d = g.read::<u32>(part + 0xd0);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let slot711 = slot_target(g, w(0xe1), 711);
    g.call(slot711, 0, &[d, program, 0]);
    let lock = g.read::<u32>(w(0xf2));
    g.write::<u32>(lock, saved);
    Ret::eax(0)
}

/// `init_data` (258 bytes; 20 overlays, `magic_0021`-`0039`, `0069`,
/// `0073`, `0326`).
pub const INIT_DATA: Sig = Sig {
    len: 258,
    hash: 0xa18b_4139_751b_d0b7,
    calls: &[],
    imports: &[],
};
