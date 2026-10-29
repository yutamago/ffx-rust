//! The OPU host's particle-part callbacks (the overlay table's `+0x40`
//! onwards; the game's OPU runner calls them): 64 part slots
//! (`G_PARTS`, a key per slot), each with a 0x80-byte buffer
//! (`G_BUFS`), started with `pppStartPart` (slot 714), run with
//! `pppRunPart` (712) and freed with `pppAllFreePObject` (713); the
//! previous parts' data (`G_PREV`, 64 words, saved in `G_SAVED`) for
//! `pppInitData` (711). Addresses are `magic_0604`'s; 98 to 130 overlays
//! carry each function. `G_ENV` (0x100b6a80) is the particle environment
//! every ppp call sees through `*G_ENVP` (0x100b68a8).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `*G_ENVP = G_ENV` (the instruction `mov [reg], G_ENV` at `off`).
fn set_env<G: Game>(g: &mut G, img: &Image, envp: u32, env_op: u32) {
    let p = g.read::<u32>(envp);
    g.write::<u32>(p, img.word(env_op));
}

/// The three matrix words every part callback refreshes first: slot 941
/// of the part data's `+0x14`, `+0x18`, `+0x1c` into `G_M0..G_M2`; the
/// words pushed stay on the stack, so each call sees the earlier ones
/// after its own. `ops` are the (table, destination) operand offsets.
fn matrices<G: Game>(g: &mut G, img: &Image, f: u32, data: u32, ops: [(u32, u32); 3]) -> [u32; 3] {
    let mut w = [0u32; 3];
    for (k, (t, dst)) in ops.iter().enumerate() {
        let slot = slot_target(g, img.word(f + t), 941);
        // each word is read when it is pushed, after the earlier calls
        w[k] = g.read::<u32>(data + 0x14 + 4 * k as u32);
        let mut args = vec![w[k]];
        args.extend(w[..k].iter().rev());
        // call width: 3 (k < 3)
        let r = g.call(slot, 0, &args).eax;
        g.write::<u32>(img.word(f + dst), r);
    }
    w
}

/// `pppStartPart(ecx, edx, a0, a1, 0)` with the game flag `*G_LOCK` set
/// to 1 around it (`magic_0604` 0x10002230, `ecx`/`edx` and two stack
/// words; 537 overlays). Returns the eax it leaves (the lock pointer).
pub fn start_part<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ecx: u32,
    edx: u32,
    a0: u32,
    a1: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let lock = g.read::<u32>(w(0x04));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let slot714 = slot_target(g, w(0x1a), 714);
    g.call(slot714, ecx, &[ecx, edx, a0, a1, 0]);
    let lock = g.read::<u32>(w(0x29));
    g.write::<u32>(lock, saved);
    lock
}

/// The `pppStartPart` wrapper (53 bytes).
pub const START_PART: Sig = Sig {
    len: 53,
    hash: 0x6214_5929_e822_23bb,
    calls: &[],
    imports: &[],
};

/// `+0x44` (0x10001250, `(part)`): gives the part a slot: the one already
/// holding its key (`(byte +0x1e << 16) + u16 +0x1a`), whose buffer is
/// freed first, else the first free one (if none is free, slot 64 past
/// the end, freed, as the original does); stores the key, points the
/// part's `+0xbc` at the slot's buffer and starts it: `pppStartPart(buf,
/// 0x1000, slot941(data +0x10)[byte +0xd4], 1)`. Returns 0.
pub fn alloc<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = fr.arg(g, 0);
    let data = g.read::<u32>(part + 0xd0);
    matrices(g, img, f, data, [(0x04, 0x20), (0x25, 0x35), (0x3a, 0x50)]);
    let mut envp = g.read::<u32>(w(0x4b));
    g.write::<u32>(envp, w(0x56));
    let key = (u32::from(g.read::<u8>(part + 0x1e)) << 16) + u32::from(g.read::<u16>(part + 0x1a));
    let parts = w(0x76);
    let found = (0..0x40u32).find(|&i| g.read::<u32>(parts + 4 * i) == key);
    let (slot, free_first) = match found {
        Some(i) => (i, true),
        None => match (0..0x40u32).find(|&i| g.read::<u32>(w(0x96) + 4 * i) == 0) {
            Some(i) => (i, false),
            None => (0x40, true),
        },
    };
    if free_first {
        let buf = w(0xab) + 0x80 * slot;
        let slot713 = slot_target(g, w(0xb1), 713);
        g.write::<u32>(envp, w(0xb7));
        g.call(slot713, 0, &[buf]);
        envp = g.read::<u32>(w(0xc5));
    }
    g.write::<u32>(w(0xd2) + 4 * slot, key);
    let buf = w(0xdb) + 0x80 * slot;
    g.write::<u32>(part + 0xbc, buf);
    g.write::<u32>(envp, w(0xe7));
    let data = g.read::<u32>(part + 0xd0);
    let x10 = g.read::<u32>(data + 0x10);
    let slot941 = slot_target(g, w(0xf5), 941);
    let list = g.call(slot941, 0, &[x10]).eax;
    let which = u32::from(g.read::<u8>(part + 0xd4));
    let program = g.read::<u32>(list.wrapping_add(4 * which));
    start_part(g, img, img.rel32_target(f + 0x117), buf, 0x1000, program, 1);
    Ret::eax(0)
}

/// `alloc` (293 bytes).
pub const ALLOC: Sig = Sig {
    len: 293,
    hash: 0x86fb_4a0d_02cd_55f5,
    calls: &[(0x117, Callee::Sig(&START_PART))],
    imports: &[],
};

/// `+0x48` (0x10001380, `(part)`): runs the part's buffer one step
/// (`pppRunPart(buf, 0x30, ...)` with the flag set); when it says done,
/// clears the slot key (u16 `+0x1e` of the buffer) and frees it. Returns
/// what `pppRunPart` returned.
pub fn run<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = fr.arg(g, 0);
    let buf = g.read::<u32>(part + 0xbc);
    let data = g.read::<u32>(part + 0xd0);
    let m = matrices(g, img, f, data, [(0x15, 0x26), (0x2b, 0x3b), (0x40, 0x50)]);
    set_env(g, img, w(0x55), f + 0x5d);
    let lock = g.read::<u32>(w(0x62));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let slot712 = slot_target(g, w(0x70), 712);
    let done = g.call(slot712, 0, &[buf, 0x30, m[2], m[1], m[0]]).eax;
    let lock = g.read::<u32>(w(0x7e));
    g.write::<u32>(lock, saved);
    if done != 0 {
        let i = u32::from(g.read::<u16>(buf + 0x1e));
        let slot713 = slot_target(g, w(0x92), 713);
        g.write::<u32>(w(0x99) + 4 * i, 0);
        set_env(g, img, w(0xa3), f + 0xaa);
        g.call(slot713, 0, &[buf]);
    }
    Ret::eax(done)
}

/// `run` (192 bytes).
pub const RUN: Sig = Sig {
    len: 192,
    hash: 0xb9d1_41d7_ec74_320e,
    calls: &[],
    imports: &[],
};

/// Frees every buffer whose slot has a key (`[key] = 0`, then
/// `pppAllFreePObject(buf)` with the environment set). `ops`: the
/// operands of `G_ENVP` (twice), `G_ENV` (once per free), `G_BUFS`,
/// `G_PARTS`, the table and the end. Returns the last free's eax, else 0.
fn free_all<G: Game>(g: &mut G, img: &Image, f: u32, o: [u32; 7]) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut envp = g.read::<u32>(w(o[0]));
    let mut eax = 0;
    let (mut buf, mut key) = (w(o[3]), w(o[4]));
    while key.cast_signed() < w(o[6]).cast_signed() {
        if g.read::<u32>(key) != 0 {
            let slot713 = slot_target(g, w(o[5]), 713);
            g.write::<u32>(key, 0);
            g.write::<u32>(envp, w(o[2]));
            eax = g.call(slot713, 0, &[buf]).eax;
            envp = g.read::<u32>(w(o[1]));
        }
        key += 4;
        buf += 0x80;
    }
    eax
}

/// `+0x4c` (0x10001440): saves the 64 previous-part words to `G_SAVED`,
/// clears them, sets the environment and frees every part.
pub fn save_and_free<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let envp = g.read::<u32>(w(0x02));
    let (prev, saved) = (w(0x0e), w(0x13));
    for i in 0..0x40 {
        let v = g.read::<u32>(prev + 4 * i);
        g.write::<u32>(saved + 4 * i, v);
    }
    let prev = w(0x21);
    for i in 0..0x40 {
        g.write::<u32>(prev + 4 * i, 0);
    }
    g.write::<u32>(envp, w(0x27));
    let eax = free_all(g, img, f, [0x02, 0x58, 0x49, 0x2e, 0x33, 0x3d, 0x67]);
    Ret::eax(eax)
}

/// `save_and_free` (112 bytes).
pub const SAVE_AND_FREE: Sig = Sig {
    len: 112,
    hash: 0x42cf_75e6_a7b8_0667,
    calls: &[],
    imports: &[],
};

/// `+0x5c` (0x100014b0): the 64 previous-part words back from `G_SAVED`.
/// eax is the caller's.
pub fn restore<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let (saved, prev) = (img.word(f + 0x08), img.word(f + 0x0d));
    for i in 0..0x40 {
        let v = g.read::<u32>(saved + 4 * i);
        g.write::<u32>(prev + 4 * i, v);
    }
    Ret::eax(fr.eax)
}

/// `restore` (22 bytes).
pub const RESTORE: Sig = Sig {
    len: 22,
    hash: 0xba71_f851_8d31_7614,
    calls: &[],
    imports: &[],
};

/// `+0x50` (0x100014d0): clears the previous-part words, sets the
/// environment and frees every part.
pub fn clear_and_free<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let envp = g.read::<u32>(w(0x02));
    let prev = w(0x10);
    for i in 0..0x40 {
        g.write::<u32>(prev + 4 * i, 0);
    }
    g.write::<u32>(envp, w(0x16));
    let eax = free_all(g, img, f, [0x02, 0x47, 0x38, 0x1d, 0x22, 0x2c, 0x56]);
    Ret::eax(eax)
}

/// `clear_and_free` (95 bytes).
pub const CLEAR_AND_FREE: Sig = Sig {
    len: 95,
    hash: 0xe643_e09c_2fd6_f493,
    calls: &[],
    imports: &[],
};

/// `+0x54` (0x10001530, `(part)`): a part in state 3 with a buffer frees
/// it (`pppAllFreePObject` with the environment set), clears its slot
/// key, its state and its buffer pointer; returns the slot index. Any
/// other part: nothing, eax the caller's.
pub fn free<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = fr.arg(g, 0);
    if g.read::<u8>(part + 0xbb) != 3 || g.read::<u32>(part + 0xbc) == 0 {
        return Ret::eax(fr.eax);
    }
    let envp = g.read::<u32>(w(0x1a));
    g.write::<u32>(envp, w(0x21));
    let buf = g.read::<u32>(part + 0xbc);
    g.write::<u32>(envp, w(0x2d));
    let slot713 = slot_target(g, w(0x32), 713);
    g.call(slot713, 0, &[buf]);
    let i = u32::from(g.read::<u16>(buf + 0x1e));
    g.write::<u32>(w(0x49) + 4 * i, 0);
    g.write::<u8>(part + 0xbb, 0);
    g.write::<u32>(part + 0xbc, 0);
    Ret::eax(i)
}

/// `free` (102 bytes).
pub const FREE: Sig = Sig {
    len: 102,
    hash: 0x3aaa_8f81_d4d1_1372,
    calls: &[],
    imports: &[],
};

/// `+0x58` (0x100015a0, `(data)`): for each of the data's u16 `+0x50`
/// entries, `op_oef2_a_get(data, 8, i)` (slot 719); a non-zero result is
/// cleared from the previous-part words. Returns the count it last read
/// (0 for none).
pub fn forget<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let data = fr.arg(g, 0);
    let mut eax = 0;
    if g.read::<u16>(data + 0x50) == 0 {
        return Ret::eax(eax);
    }
    let mut i = 0u32;
    loop {
        let slot719 = slot_target(g, w(0x13), 719);
        let r = g.call(slot719, 0, &[data, 8, i]).eax;
        if r != 0 {
            let (mut p, end) = (w(0x2b), w(0x3f));
            while p.cast_signed() < end.cast_signed() {
                if g.read::<u32>(p) == r {
                    g.write::<u32>(p, 0);
                }
                p += 4;
            }
        }
        eax = u32::from(g.read::<u16>(data + 0x50));
        i += 1;
        if i.cast_signed() >= eax.cast_signed() {
            return Ret::eax(eax);
        }
    }
}

/// `forget` (82 bytes).
pub const FORGET: Sig = Sig {
    len: 82,
    hash: 0x9f53_a786_d9e1_b9e3,
    calls: &[],
    imports: &[],
};

/// `+0x5c`.. (0x10001600, `(effect)`): before the first matrix refresh
/// (`G_M0` still 0), `pppInitEnv(G_ENV, 0, effect +0xa0, effect +0xa4)`
/// (slot 710) and, with the battle flag byte `+0x10` set,
/// `op_heap_display_set_par(G_ENV, 0x8003)` (slot 722).
pub fn init_env<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x05)) != 0 {
        return Ret::eax(fr.eax);
    }
    let effect = fr.arg(g, 0);
    let a4 = g.read::<u32>(effect + 0xa4);
    let a0 = g.read::<u32>(effect + 0xa0);
    let slot710 = slot_target(g, w(0x1c), 710);
    g.call(slot710, 0, &[w(0x29), 0, a0, a4]);
    let flags = g.read::<u32>(w(0x30));
    if g.read::<u8>(flags + 0x10) == 0 {
        return Ret::eax(flags);
    }
    let slot722 = slot_target(g, w(0x3e), 722);
    Ret::eax(g.call(slot722, 0, &[w(0x4e), 0x8003]).eax)
}

/// `init_env` (89 bytes).
pub const INIT_ENV: Sig = Sig {
    len: 89,
    hash: 0x1548_2773_bc33_410f,
    calls: &[],
    imports: &[],
};

/// `+0x40` (0x10001710, `(part)`): finds the part's data among the
/// effect's entries (`op_oef2_a_get(effect, 8, i)`, slot 719, the effect
/// being `*G_SLOTS + 4`) and, the first time for that entry, records it
/// in the previous-part words and runs `pppInitData(data, program[i], 0)`
/// (slot 711, the program table in `.rdata`) with the flag set. Returns 0.
pub fn init_data<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let part = fr.arg(g, 0);
    let data = g.read::<u32>(part + 0xd0);
    matrices(g, img, f, data, [(0x04, 0x20), (0x25, 0x35), (0x3a, 0x4a)]);
    set_env(g, img, w(0x4f), f + 0x58);
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
    if g.read::<u32>(w(0xaf) + 4 * i) != 0 {
        return Ret::eax(0);
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

/// `init_data` (258 bytes, 98 overlays).
pub const INIT_DATA: Sig = Sig {
    len: 258,
    hash: 0xee8b_f9ba_56dd_0b49,
    calls: &[],
    imports: &[],
};
