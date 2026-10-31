//! `magic_0053.dll`'s own functions (besides the runtime, the object
//! host and the shared code), and `magic_0065`'s copy of its sound step
//! without the slot 594 wait.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Operand offsets of the sound steps ([`sound_step`]).
struct SoundOps {
    /// `rel32` of the motion step; the waits `*A`, `*B`, `*C`.
    motion: u32,
    a: u32,
    b: u32,
    c: u32,
    /// The slot 594 wait: `*D`, `*E` and slot 594's table global (none
    /// in `magic_0065`).
    gate: Option<(u32, u32, u32)>,
    /// The sound byte's pointer, the flag `*V`, slot 530's table global,
    /// the sound table, slot 249's table global.
    s: u32,
    v: u32,
    g530: u32,
    table: u32,
    g249: u32,
    /// The callback for the task's record; `rel32` of the registration.
    cb: u32,
    reg: u32,
}

/// A task step (cdecl `(task)`): the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)); unless it says 1,
/// its answer. With `*A` 1 it waits (1) for bit 0x8000 of `*B` and for
/// `*C` to clear. With the gate: unless `*D` is set, slot 594 must say 0
/// and `*E` be set (else slot 594's answer). Then the sound: the byte
/// `b = **S` through slot 249 `(table[b], 0x100)` when `*V` is set, else
/// slot 530 `(b, -1)` below 0x20 or slot 249 `(table[b], -1)` from 0x20
/// on; the task's state `+0x14` becomes 3, its record (`task[8]`) gets
/// the callback, and the parts are registered
/// ([`register_parts`](crate::sa_misc::register_parts), a tail jump with
/// the record in `eax`).
fn sound_step<G: Game>(g: &mut G, img: &Image, fr: &Frame, o: &SoundOps) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = crate::s3_motion::motion_step(g, img, img.rel32_target(f + o.motion));
    if r != 1 {
        return Ret::eax(r);
    }
    if g.read::<u32>(w(o.a)) == 1
        && (g.read::<u32>(w(o.b)) & 0x8000 == 0 || g.read::<u32>(w(o.c)) != 0)
    {
        return Ret::eax(1);
    }
    if let Some((d, e, g594)) = o.gate
        && g.read::<u32>(w(d)) == 0
    {
        let on = g.read::<u32>(w(e)) != 0;
        let s = slot_target(g, w(g594), 0x948 / 4);
        let r2 = g.call(s, 0, &[]).eax;
        if r2 != 0 || !on {
            return Ret::eax(r2);
        }
    }
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(o.s))));
    if g.read::<u32>(w(o.v)) != 0 {
        let s = slot_target(g, w(o.g249), 0x3e4 / 4);
        let v = g.read::<u32>(w(o.table).wrapping_add(4 * b));
        g.call(s, 0, &[v, 0x100]);
    } else if b < 0x20 {
        let s = slot_target(g, w(o.g530), 0x848 / 4);
        g.call(s, 0, &[b, u32::MAX]);
    } else {
        let s = slot_target(g, w(o.g249), 0x3e4 / 4);
        let v = g.read::<u32>(w(o.table).wrapping_add(4 * b));
        g.call(s, 0, &[v, u32::MAX]);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(o.cb));
    let reg = img.rel32_target(f + o.reg);
    Ret::eax(crate::sa_misc::register_parts(g, img, reg, rec))
}

/// 0x100013d0 (cdecl `(task)`): [`sound_step`] with the slot 594 gate
/// (operands `+0x13`, `+0x1b`, `+0x2b`; gate `+0x38`, `+0x49`; sound
/// `+0x68`, `+0x71`, table `+0x9b`; callback `+0xc0`).
pub fn sound_gate_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = SoundOps {
        motion: 0x04,
        a: 0x13,
        b: 0x1b,
        c: 0x2b,
        gate: Some((0x38, 0x49, 0x51)),
        s: 0x68,
        v: 0x71,
        g530: 0x82,
        table: 0x9b,
        g249: 0xa0,
        cb: 0xc0,
        reg: 0xc6,
    };
    sound_step(g, img, fr, &o)
}

/// [`sound_gate_step`] (205 bytes, 1 overlay).
pub const SOUND_GATE_STEP: Sig = Sig {
    len: 205,
    hash: 0x602f_4945_613d_47c0,
    calls: &[
        (0x04, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc6, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// `magic_0065` 0x100031a0 (cdecl `(task)`): [`sound_step`] without the
/// gate (operands `+0x13`, `+0x1b`, `+0x27`; sound `+0x30`, `+0x39`,
/// table `+0x63`; callback `+0x87`).
pub fn sound_plain_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = SoundOps {
        motion: 0x04,
        a: 0x13,
        b: 0x1b,
        c: 0x27,
        gate: None,
        s: 0x30,
        v: 0x39,
        g530: 0x4a,
        table: 0x63,
        g249: 0x68,
        cb: 0x87,
        reg: 0x8d,
    };
    sound_step(g, img, fr, &o)
}

/// [`sound_plain_step`] (147 bytes, 1 overlay).
pub const SOUND_PLAIN_STEP: Sig = Sig {
    len: 147,
    hash: 0xb66c_59dc_89dc_7f3f,
    calls: &[
        (0x04, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x8d, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};
