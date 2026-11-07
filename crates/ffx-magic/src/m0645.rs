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

/// The caster's fade task (0x10006f50, a task callback, cdecl `(task)`,
/// `S = task[1]`; the argument's word reused as scratch): two random
/// offsets `(K2 - (slot 307 - K1)) * K3` added to `S+0x28`/`+0x2c` into
/// `S+0x4c`/`+0x54`; then by the step `S+0x18`: 0 starts the wait 30; 1
/// waits, then step 20; 20 sets the fade (`+0x24` 0.005, `+0x20` 0.05,
/// the wait 180) and becomes 21; 21 waits, then fades the character
/// ([`crate::l3_common::chr_fade`] on `[S[0x1084], 0, 0x40]` at `ebp -
/// 0xc`), `+0x107c` 1, the wait 64 and step 22; 22 waits, then step
/// 0x2710: the task's node leaves its ring and the step is 0x4e20. The
/// wait `S+0x1c` drops by one on every way out. Returns the `eax` the
/// original leaves.
pub fn caster_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    caster_fade_with(g, img, fr, false)
}

/// [`caster_fade`] with the alpha ramp (0x100072c0): the record at
/// `+0x1058` and the flag at `+0x1050`; 21 waiting and 22 also step the
/// ramp (`+0x20 += +0x24`, `+0x50 +=` it, the halfword-sized counter
/// `+0x80 += 0x20`) before their wait, and 21's fade goes on to it.
pub fn caster_fade_ramp<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    caster_fade_with(g, img, fr, true)
}

/// The two fades' body (`ramp`: the 0x100072c0 copy).
#[allow(clippy::too_many_lines)] // the original's steps, one by one
fn caster_fade_with<G: Game>(g: &mut G, img: &Image, fr: &Frame, ramp: bool) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    let a = fr.esp + 4;
    let mut eax = 0;
    for (table, ops, src, dst) in [
        (0x07u32, [0x27u32, 0x2f, 0x40], 0x28u32, 0x4cu32),
        (0x34, [0x62, 0x6a, 0x76], 0x2c, 0x54),
    ] {
        let t = slot_target(g, w(table), 307);
        // call width: 0
        let out = g.call(t, 0, &[]);
        eax = out.eax;
        let r = crate::ob_model::st0_f32(out.st0);
        g.write::<f32>(a, r);
        let x = sub(r, k(g, ops[0]));
        let v = mul(sub(k(g, ops[1]), x), k(g, ops[2]));
        let v = add(v, g.read::<f32>(s + src));
        g.write::<f32>(s + dst, v);
    }
    // the ramp's step (0x100072c0), then the wait: true when it runs out
    let ramp_step = |g: &mut G| -> bool {
        let r = add(g.read::<f32>(s + 0x24), g.read::<f32>(s + 0x20));
        g.write::<f32>(s + 0x20, r);
        let c = g.read::<u32>(s + 0x80).wrapping_add(0x20);
        g.write::<u32>(s + 0x80, c);
        let v = add(g.read::<f32>(s + 0x50), r);
        g.write::<f32>(s + 0x50, v);
        g.read::<i32>(s + 0x1c) <= 0
    };
    let (rec_op, flag_op, call) = if ramp {
        (0xccu32, 0xeeu32, 0xe5u32)
    } else {
        (0xd0, 0xf2, 0xe9)
    };
    loop {
        let step = g.read::<u32>(s + 0x18);
        let wait = g.read::<i32>(s + 0x1c);
        if step > 0x15 {
            if step == 0x16 {
                if ramp {
                    if !ramp_step(g) {
                        break;
                    }
                } else if wait > 0 {
                    break;
                }
                g.write::<u32>(s + 0x18, 0x2710);
                continue;
            }
            if step == 0x2710 {
                eax = crate::r12h_volley::unring(g, g.read::<u32>(task));
                g.write::<u32>(s + 0x18, 0x4e20);
            }
            break;
        }
        if step != 0x15 {
            match step {
                0 => {
                    g.write::<u32>(s + 0x18, 1);
                    g.write::<u32>(s + 0x1c, 0x1d);
                    return Ret::eax(0);
                }
                1 => {
                    eax = 0;
                    if wait > 0 {
                        break;
                    }
                    g.write::<u32>(s + 0x18, 0x14);
                    continue;
                }
                0x14 => {
                    eax = 0x15;
                    g.write::<u32>(s + 0x24, 0x3ba3_d70a);
                    g.write::<u32>(s + 0x20, 0x3d4c_cccd);
                    g.write::<u32>(s + 0x1c, 0xb4);
                    g.write::<u32>(s + 0x18, 0x15);
                }
                _ => {
                    eax = step.wrapping_sub(0x14);
                    break;
                }
            }
        }
        if g.read::<i32>(s + 0x1c) <= 0 {
            let rec = fr.esp.wrapping_sub(0x10);
            let chr = g.read::<u32>(s.wrapping_add(w(rec_op)));
            for (i, v) in (0u32..).zip([chr, 0, 0x40]) {
                g.write::<u32>(rec + 4 * i, v);
            }
            eax = crate::l3_common::chr_fade(g, img, img.rel32_target(f + call), rec);
            g.write::<u32>(s + 0x18, 0x16);
            g.write::<u32>(s.wrapping_add(w(flag_op)), 1);
            g.write::<u32>(s + 0x1c, 0x40);
        }
        if ramp && ramp_step(g) {
            g.write::<u32>(s + 0x18, 0x2710);
            continue;
        }
        break;
    }
    let wait = g.read::<u32>(s + 0x1c).wrapping_sub(1);
    g.write::<u32>(s + 0x1c, wait);
    Ret::eax(eax)
}

/// [`caster_fade`] (369 bytes, 1 overlay).
pub const CASTER_FADE: Sig = Sig {
    len: 369,
    hash: 0x25be_f89c_276a_46e2,
    calls: &[(0xe9, Callee::Sig(&crate::l3_common::CHR_FADE))],
    imports: &[],
};

/// [`caster_fade_ramp`] (405 bytes, 1 overlay).
pub const CASTER_FADE_RAMP: Sig = Sig {
    len: 405,
    hash: 0xbebc_5fd0_45f2_5dc8,
    calls: &[(0xe5, Callee::Sig(&crate::l3_common::CHR_FADE))],
    imports: &[],
};

/// A colour node's visit (0x100033d0, thiscall: `ecx` = the visit group
/// `V`; a /GS frame): the walk over the node's children
/// ([`crate::nodes::walk`]); when it ran (`V+4` bit 0) and the node `N`
/// (`V[0]`) has steps left (`+0x54`), its colour `+0x1c..+0x28` moves by
/// `+0x3c.. * +0x50` and its integer parts make the colour word `((c3 <<
/// 8 | c2) << 8 | c1) << 8 | c0`; `+0x4c += +0x50`; slot 252 `(*G_S,
/// colour, the byte +0x5a, 0x80)` into `G_S` (the byte's word on the frame
/// has three never-written bytes: 0, brief 2026-10-24); once `+0x4c`
/// reaches `K`, slot 234 `(N + 0x1c, N + 0x2c, N + 0x3c)`, a step fewer
/// and `+0x4c` 0. With no steps left and `+0x58` bit 0, the node leaves
/// ([`crate::sa_misc::leave`]). Returns the `eax` the original leaves.
pub fn colour_visit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = fr.ecx;
    // eax: the frame's cookie word (`*G_COOKIE ^ ebp`) when the walk starts
    let cookie = g.read::<u32>(w(0x07)) ^ fr.esp.wrapping_sub(4);
    let (eax, _) = crate::nodes::walk(g, v, v, cookie);
    if g.read::<u8>(v + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(v);
    if g.read::<u32>(n + 0x54) == 0 {
        if g.read::<u8>(n + 0x58) & 1 == 0 {
            return Ret::eax(eax);
        }
        return Ret::eax(crate::sa_misc::leave(g, n, eax));
    }
    let ebp = fr.esp.wrapping_sub(4);
    let rate = g.read::<f32>(n + 0x50);
    let mut parts = [0u32; 4];
    for (i, part) in (0u32..).zip(parts.iter_mut()) {
        let x = add(
            mul(g.read::<f32>(n + 0x3c + 4 * i), rate),
            g.read::<f32>(n + 0x1c + 4 * i),
        );
        *part = crate::s4_summon::trunc(x);
        g.write::<f32>(n + 0x1c + 4 * i, x);
        g.write::<u32>(ebp.wrapping_sub(0x14 - 4 * i), *part);
    }
    // junk read: [ebp-0x17..-0x15] never written; 0 (brief 2026-10-24)
    let byte = u32::from(g.read::<u8>(n + 0x5a));
    g.write::<u32>(ebp.wrapping_sub(0x18), byte);
    let t = add(g.read::<f32>(n + 0x50), g.read::<f32>(n + 0x4c));
    let colour = (((parts[3] << 8 | parts[2]) << 8 | parts[1]) << 8) | parts[0];
    g.write::<f32>(n + 0x4c, t);
    let s = g.read::<u32>(w(0xcf));
    let tbl = slot_target(g, w(0xc9), 252);
    // call width: 4
    let r = g.call(tbl, 0, &[s, colour, byte, 0x80]).eax;
    g.write::<u32>(w(0xeb), r);
    let k = g.read::<f32>(w(0xe6));
    // comiss + jb: below or unordered keeps going
    let x = g.read::<f32>(n + 0x4c);
    if x < k || x.is_nan() || k.is_nan() {
        return Ret::eax(r);
    }
    let tbl = slot_target(g, w(0xfa), 234);
    // call width: 3
    let r = g.call(tbl, 0, &[n + 0x1c, n + 0x2c, n + 0x3c]).eax;
    let left = g.read::<u32>(n + 0x54).wrapping_sub(1);
    g.write::<u32>(n + 0x54, left);
    g.write::<u32>(n + 0x4c, 0);
    Ret::eax(r)
}

/// The thunk over [`colour_visit`] (0x10003520, 12 bytes, cdecl
/// `(group)`): `ecx` = the group, then the tail jump.
pub fn colour_visit_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    let visit = Frame {
        at: img.rel32_target(fr.at + 8),
        ecx,
        ..*fr
    };
    colour_visit(g, img, &visit)
}

/// [`colour_visit_thunk`] (the `rel32`: [`colour_visit`]).
pub const COLOUR_VISIT_THUNK: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&COLOUR_VISIT))],
    imports: &[],
};

/// [`colour_visit`] (323 bytes, 1 overlay).
pub const COLOUR_VISIT: Sig = Sig {
    len: 323,
    hash: 0x948f_3f6a_06a7_bd08,
    calls: &[
        // __security_check_cookie
        (0x11d, Callee::Any),
        (0x12e, Callee::Sig(&crate::sa_misc::LEAVE)),
        // __security_check_cookie
        (0x13b, Callee::Any),
    ],
    imports: &[],
};
