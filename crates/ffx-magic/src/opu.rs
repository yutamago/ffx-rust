//! The OPU host: the functions 128 to 130 overlays share (an effect
//! driven by the game's OPU program runner, `opu_main_host` slot 716 and
//! `opp_main` slot 721). Addresses in the comments are `magic_0604`'s;
//! each port reads the DLL's own globals from its instructions' operands
//! (`img.word(f + offset)`, the offset of the operand in the function).
//!
//! Globals (0604 addresses): `G_TABLE` 0x100b689c (`g_MagicImports`),
//! `G_OBJ` 0x100b6944 (the OPU object slot 715 made), `G_CUR` 0x100b68a0,
//! `G_LOCK` 0x100b6900 (a pointer to a game flag the overlay sets to 1
//! around the OPU calls), `G_FLAGS` 0x100b68a4, `G_CTRL` 0x100b68ac,
//! `G_SCREEN` 0x100b6894 and `G_CAMERA` 0x100b6898 (matrix pointers),
//! `G_PARTS` 0x100b6bc0 (64 words), `G_BUFS` 0x100b4890 (64 x 0x80
//! bytes), `G_PREV` 0x100b6980 (64 words), `G_ENV` 0x100b6a80 (0x2c
//! bytes).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `this->+0x21200`-returning set-up of a new OPU object (`ecx`):
/// slot 720 (0x3400, 0x800); when the battle flag byte `+0x10` is set and
/// the control word `+0x40` has bit 4, slot 717 (1) with the word forced
/// to 0x10 around the call; the object's `+0x50` gets the overlay table;
/// the 64 part slots are cleared and each 0x80-byte buffer zeroed with
/// its index at `+0x1e`; the 64 previous-part words and the 0x2c-byte
/// environment block are cleared; `+0xc` = `this + 0x21200`.
pub fn setup<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let table = w(0x05);
    let slot720 = slot_target(g, table, 720);
    g.call(slot720, this, &[0x3400, 0x800]);
    let flags = g.read::<u32>(w(0x24));
    if g.read::<u8>(flags + 0x10) != 0 {
        let ctrl = g.read::<u32>(w(0x33));
        let word = g.read::<u16>(ctrl + 0x40);
        if word & 0x10 != 0 {
            g.write::<u16>(ctrl + 0x40, 0x10);
            let slot717 = slot_target(g, w(0x4b), 717);
            g.call(slot717, this, &[1]);
            g.write::<u16>(ctrl + 0x40, word);
        }
    }
    g.write::<u32>(this + 0x50, w(0x63));
    let parts = w(0x71);
    for i in 0..0x40 {
        g.write::<u32>(parts + 4 * i, 0);
    }
    let bufs = w(0x76);
    for i in 0..0x40u16 {
        let b = bufs + 0x80 * u32::from(i);
        g.write_bytes(b, &[0; 0x80]); // memset(b, 0, 0x80)
        g.write::<u16>(b + 0x1e, i);
    }
    let prev = w(0xa8);
    for i in 0..0x40 {
        g.write::<u32>(prev + 4 * i, 0);
    }
    g.write_bytes(w(0xad), &[0; 0x2c]); // memset(env, 0, 0x2c)
    let end = this.wrapping_add(0x21200);
    g.write::<u32>(this + 0xc, end);
    end
}

/// `setup` (`magic_0604` 0x10001000, 206 bytes, 128 overlays).
pub const SETUP: Sig = Sig {
    len: 206,
    hash: 0x0ba0_1a3f_2076_6785,
    calls: &[
        (0x89, Callee::Import("memset")),
        (0xb4, Callee::Import("memset")),
    ],
    imports: &[],
};

/// `vprintf(fmt, &args)` through slot 726 (the 28-byte varargs wrapper,
/// `magic_0604` 0x10002270): `va` is the address of the caller's first
/// variadic word.
pub(crate) fn debug_printf<G: Game>(g: &mut G, table: u32, fmt: u32, va: u32) {
    let slot726 = slot_target(g, table, 726);
    g.call(slot726, 0, &[fmt, va]);
}

/// The printf wrapper.
pub const DEBUG_PRINTF: Sig = Sig {
    len: 28,
    hash: 0x6a04_2735_19ff_88be,
    calls: &[],
    imports: &[],
};

/// The `G_CUR` save/restore around the OPU calls (+0x54 holds it, +0x58
/// overrides it).
fn enter_cur<G: Game>(g: &mut G, obj: u32, cur: u32) {
    let v = g.read::<u32>(obj + 0x58);
    let v = if v == 0 { g.read::<u32>(cur) } else { v };
    g.write::<u32>(obj + 0x54, v);
}

/// Table `+0xc` (`magic_0604` 0x100010e0): starts the OPU program:
/// `opu_main_host(1, 0)` with the game flag at `*G_LOCK` set to 1 around
/// it. If the object's `+0x58` override differs from the saved current
/// (`+0x54`), reports it (slot 726, the string at `+0x57`) and calls
/// `debug_exit_trace` (slot 727) with `(1, string)`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = g.read::<u32>(w(0x03));
    enter_cur(g, obj, w(0x0f));
    let lock = g.read::<u32>(w(0x17));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let slot716 = slot_target(g, w(0x25), 716);
    g.call(slot716, fr.ecx, &[1, 0]);
    let lock = g.read::<u32>(w(0x36));
    g.write::<u32>(lock, saved);
    let over = g.read::<u32>(obj + 0x58);
    if over == 0 {
        let cur = g.read::<u32>(obj + 0x54);
        g.write::<u32>(w(0x4b), cur);
        return Ret::eax(cur);
    }
    if over == g.read::<u32>(obj + 0x54) {
        return Ret::eax(over);
    }
    let msg = w(0x57);
    // the wrapper's `&args` is [ebp + 0xc] of its frame: the word above
    // the pushed string, 4 below this function's return address
    debug_printf(g, w(0x61), msg, fr.esp.wrapping_sub(4));
    let slot727 = slot_target(g, w(0x61), 727);
    let r = g.call(slot727, 0, &[1, msg]);
    Ret::eax(r.eax)
}

/// `start` (116 bytes, 130 overlays).
pub const START: Sig = Sig {
    len: 116,
    hash: 0x71b6_0bca_c74e_4eb9,
    calls: &[(0x5c, Callee::Sig(&DEBUG_PRINTF))],
    imports: &[],
};

/// Table `+0x10` (`magic_0604` 0x10001160), every frame: the camera and
/// screen matrices (slots 724 and 723), then `opu_main_host(1, 1)` and
/// `opp_main(1, 1, 1)` with the game flag set; the arguments pushed for
/// the earlier calls stay on the stack, so each call sees them after its
/// own.
pub fn run<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = g.read::<u32>(w(0x03));
    enter_cur(g, obj, w(0x0f));
    let camera = g.read::<u32>(w(0x1e));
    let slot724 = slot_target(g, w(0x17), 724);
    g.call(slot724, fr.ecx, &[camera]);
    let screen = g.read::<u32>(w(0x31));
    let slot723 = slot_target(g, w(0x2b), 723);
    g.call(slot723, fr.ecx, &[screen, camera]);
    let lock = g.read::<u32>(w(0x3e));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let slot716 = slot_target(g, w(0x4d), 716);
    g.call(slot716, fr.ecx, &[1, 1, screen, camera]);
    let lock = g.read::<u32>(w(0x5c));
    g.write::<u32>(lock, 1);
    let slot721 = slot_target(g, w(0x69), 721);
    g.call(slot721, fr.ecx, &[1, 1, 1, screen, camera]);
    let lock = g.read::<u32>(w(0x76));
    g.write::<u32>(lock, saved);
    if g.read::<u32>(obj + 0x58) != 0 {
        return Ret::eax(lock);
    }
    let cur = g.read::<u32>(obj + 0x54);
    g.write::<u32>(w(0x8a), cur);
    Ret::eax(cur)
}

/// `run` (144 bytes, 130 overlays).
pub const RUN: Sig = Sig {
    len: 144,
    hash: 0xac4c_ee0e_9da1_af75,
    calls: &[],
    imports: &[],
};

/// Table callback (`magic_0604` 0x100011f0, `+0x74` of the overlay
/// table): `opu_main_host(1, 2)` with the game flag set, when there is
/// an OPU object.
pub fn stop<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = g.read::<u32>(w(0x03));
    if obj == 0 {
        // test edi, edi; je -> pop edi; ret: eax is the caller's
        return Ret::eax(fr.eax);
    }
    if g.read::<u32>(obj + 0x58) == 0 {
        let cur = g.read::<u32>(w(0x12));
        g.write::<u32>(obj + 0x54, cur);
    }
    let lock = g.read::<u32>(w(0x1a));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let slot716 = slot_target(g, w(0x28), 716);
    g.call(slot716, fr.ecx, &[1, 2]);
    let lock = g.read::<u32>(w(0x39));
    g.write::<u32>(lock, saved);
    if g.read::<u32>(obj + 0x58) != 0 {
        return Ret::eax(lock);
    }
    let cur = g.read::<u32>(obj + 0x54);
    g.write::<u32>(w(0x4d), cur);
    Ret::eax(cur)
}

/// `stop` (83 bytes, 130 overlays).
pub const STOP: Sig = Sig {
    len: 83,
    hash: 0x53db_da37_b89b_9242,
    calls: &[],
    imports: &[],
};

/// A lone `ret` (table `+0x8`, `+0x14`, `+0x18` and the unused callbacks;
/// 267 overlays): eax stays the caller's.
pub fn nop<G: Game>(_g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    Ret::eax(fr.eax)
}

/// `ret`.
pub const NOP: Sig = Sig {
    len: 1,
    hash: 0xaf64_7e4c_8602_ffd2,
    calls: &[],
    imports: &[],
};
