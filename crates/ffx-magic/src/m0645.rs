//! `magic_0645.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::m0585::task_link;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Sets up the effect's object (0x10006ec0, internal: `ecx` = the
/// object, `edx` = the source): `+0xe68` = the object; its task `+0xe54`
/// linked ([`task_link`], no own word); `+0xe7c` 0; slot 719 `(DATA, 9,
/// 0)` into `+0xe50`, `+0xe80` = the source, `+0xe6c` 0. Returns slot
/// 719's `eax`.
pub fn setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (obj, src) = (fr.ecx, fr.edx);
    g.write::<u32>(obj + 0xe68, obj);
    task_link(
        g,
        img,
        f,
        obj + 0xe54,
        [0xf, 0x19, 0x2c, 0x36, 0x3b, 0x46, 0x52],
        false,
    );
    g.write::<u32>(obj + 0xe7c, 0);
    let t = slot_target(g, w(0x4c), 719);
    let eax = g.call(t, 0, &[w(0x69), 9, 0]).eax;
    g.write::<u32>(obj + 0xe80, src);
    g.write::<u32>(obj + 0xe50, eax);
    g.write::<u32>(obj + 0xe6c, 0);
    eax
}

/// [`setup`] (142 bytes, 1 overlay).
pub const SETUP: Sig = Sig {
    len: 142,
    hash: 0xa703_2912_9748_607e,
    calls: &[],
    imports: &[],
};

/// The value task (0x10007b20, `(task)` on `r = task[1]`, the counter
/// `+0x1c` counting down each call, the state `+0x18`): state 0 waits
/// until the object `r[0x20]` is ready (slot 20 `(model)` when bit 0 of
/// its `+0x90` is set, else slot 411 `(object)`), then gives it the value
/// 0x4b ([`set_value`](crate::m0042::set_value)) and moves on; state
/// 10000 takes the task's node off its ring (state 20000). Returns the
/// set-value's `eax`, the unlinked node's next, 0 while waiting, or the
/// state.
pub fn value_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let tick = |g: &mut G| {
        let n = g.read::<u32>(r + 0x1c);
        g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    };
    let state = g.read::<u32>(r + 0x18);
    match state {
        0 => {
            let o = g.read::<u32>(r + 0x20);
            let ready = if g.read::<u8>(o + 0x90) & 1 != 0 {
                let x = g.read::<u32>(o + 0x54);
                let t = slot_target(g, w(0x4f), 20);
                g.call(t, 0, &[x]).eax
            } else {
                let x = g.read::<u32>(o + 0x50);
                let t = slot_target(g, w(0x5c), 411);
                g.call(t, 0, &[x]).eax
            };
            if ready == 0 {
                tick(g);
                return Ret::eax(0);
            }
            let o = g.read::<u32>(r + 0x20);
            let eax = crate::m0042::set_value(g, img, img.rel32_target(f + 0x78), o, 0x4b);
            let s = g.read::<u32>(r + 0x18);
            g.write::<u32>(r + 0x18, s.wrapping_add(1));
            tick(g);
            Ret::eax(eax)
        }
        0x2710 => {
            let node = g.read::<u32>(task);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(node + 8);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            tick(g);
            g.write::<u32>(r + 0x18, 0x4e20);
            Ret::eax(next)
        }
        s => {
            tick(g);
            Ret::eax(s)
        }
    }
}

/// [`value_task`] (133 bytes, 1 overlay).
pub const VALUE_TASK: Sig = Sig {
    len: 133,
    hash: 0xd0b1_2da4_e7f2_0b70,
    calls: &[(0x78, Callee::Sig(&crate::m0042::SET_VALUE))],
    imports: &[],
};

/// Starts the value task (0x10007bb0, internal: `ecx` = the task, `edx`
/// = an effect object): the task ([`value_task`] its callback) linked
/// ([`task_link`]); the object gets the value 0x4a (inline
/// [`set_value`](crate::m0042::set_value): with bit 0 of `+0x90` slot
/// 30 `(model)`, slot 36 `(model, 0x4a)` and bit 9, else slot 228
/// `(object, 0x4a)`; `+0x60` = 0x4a and bit 8); `+0x20` = the object,
/// state 0. Returns 0x100.
pub fn value_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (t, o) = (fr.ecx, fr.edx);
    task_link(
        g,
        img,
        f,
        t,
        [0xd, 0x14, 0x1c, 0x26, 0x2b, 0x36, 0x42],
        true,
    );
    let table = w(0x3c);
    if g.read::<u8>(o + 0x90) & 1 != 0 {
        let m = g.read::<u32>(o + 0x54);
        let s = slot_target(g, table, 30);
        g.call(s, 0, &[m]);
        let s = slot_target(g, w(0x58), 36);
        let m = g.read::<u32>(o + 0x54);
        g.call(s, 0, &[m, 0x4a]);
        let bits = g.read::<u16>(o + 0x90);
        g.write::<u16>(o + 0x90, bits | 0x200);
    } else {
        let s = slot_target(g, table, 228);
        let x = g.read::<u32>(o + 0x50);
        g.call(s, 0, &[x, 0x4a]);
    }
    g.write::<u32>(o + 0x60, 0x4a);
    let bits = g.read::<u16>(o + 0x90);
    g.write::<u16>(o + 0x90, bits | 0x100);
    g.write::<u32>(t + 0x20, o);
    g.write::<u32>(t + 0x18, 0);
    0x100
}

/// [`value_start`] (174 bytes, 1 overlay).
pub const VALUE_START: Sig = Sig {
    len: 174,
    hash: 0xfdbf_ba1a_e621_0c0b,
    calls: &[],
    imports: &[],
};

/// Sets up a mover (0x10007a40, internal: `ecx` = the object, `edx` =
/// the source; a frame with a stack cookie): `+0x24` = the object; its
/// task `+0x10` linked ([`task_link`], no own word); `+0x58` and `+0x318`
/// 20000; the source's model matrix `m` at `ebp - 0x58` (slot 210
/// `(src[0x54], m)`) and the vector (0, 0, -0, 1) at `ebp - 0x14` through
/// it into the object (slot 229 `(obj, m, v)`, the first call's words
/// under it); `+0x34` = the source, `+0x28..+0x30` 0. Returns slot 229's
/// `eax`.
pub fn mover<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x58), ebp.wrapping_sub(0x14));
    let (obj, src) = (fr.ecx, fr.edx);
    g.write::<u32>(obj + 0x24, obj);
    task_link(
        g,
        img,
        f,
        obj + 0x10,
        [0x19, 0x20, 0x30, 0x3a, 0x3f, 0x4a, 0x51],
        false,
    );
    g.write::<u32>(obj + 0x58, 0x4e20);
    g.write::<u32>(obj + 0x318, 0x4e20);
    let x = g.read::<u32>(src + 0x54);
    for (i, b) in (0u32..).zip([0, 0, 0x8000_0000, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, b);
    }
    let t = slot_target(g, w(0x5a), 210);
    g.call(t, 0, &[x, m]);
    let t = slot_target(g, w(0x9f), 229);
    let eax = g.call(t, 0, &[obj, m, v, x, m]).eax;
    g.write::<u32>(obj + 0x34, src);
    for off in [0x30, 0x2c, 0x28] {
        g.write::<u32>(obj + off, 0);
    }
    eax
}

/// [`mover`] (216 bytes, 1 overlay).
pub const MOVER: Sig = Sig {
    len: 216,
    hash: 0x6246_37aa_03a9_f74b,
    // __security_check_cookie
    calls: &[(0xd0, Callee::Any)],
    imports: &[],
};
