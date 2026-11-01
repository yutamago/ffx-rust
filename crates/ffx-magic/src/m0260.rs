//! `magic_0260.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The parts' programs (0x100021f0, internal: `ecx` = the effect): for
/// each of five part blocks (`+0x354`, 0x3a0 bytes apart) the count word
/// is set to 0x40 and, for i below it (read again each time), slot 772
/// `(p, i)`'s 64-bit result goes to slot 779 `(lo, hi, p, i)` (`p` the
/// block's `-0x224`) with the game flag `*L` 1 around it (`L` at `+0x3f`,
/// again at `+0x5b`). Returns `L` (the last read; the caller's `eax` is
/// never left).
pub fn part_programs<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut count = fr.ecx.wrapping_add(0x354);
    let mut lock = 0;
    for _ in 0..5 {
        g.write::<u32>(count, 0x40);
        let part = count.wrapping_sub(0x224);
        let mut i = 0u32;
        loop {
            let s = slot_target(g, w(0x31), 0xc10 / 4);
            let v = g.call(s, 0, &[part, i]);
            let l = g.read::<u32>(w(0x3f));
            let saved = g.read::<u32>(l);
            g.write::<u32>(l, 1);
            let s = slot_target(g, w(0x48), 0xc2c / 4);
            g.call(s, 0, &[v.eax, v.edx, part, i]);
            lock = g.read::<u32>(w(0x5b));
            g.write::<u32>(lock, saved);
            i += 1;
            if i.cast_signed() >= g.read::<i32>(count) {
                break;
            }
        }
        count = count.wrapping_add(0x3a0);
    }
    Ret::eax(lock)
}

/// [`part_programs`] (124 bytes, 1 overlay).
pub const PART_PROGRAMS: Sig = Sig {
    len: 124,
    hash: 0x5f73_9990_3239_8b69,
    calls: &[],
    imports: &[],
};

/// A task step (0x10003bf0, cdecl `(task)`): the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)) and the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both give 1,
/// the task's state is 3, its record's `+0x10` the immediate `+0x29`, and
/// a tail jump into the part programs ([`part_programs`], `ecx` the
/// immediate `+0x2f`). Returns their result, else the objects'.
pub fn programs_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x5));
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0xc), fr.esp.wrapping_sub(12));
    if r != 1 || m != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0x29));
    let callee = Frame {
        at: img.rel32_target(f + 0x35),
        ecx: w(0x2f),
        eax: rec,
        ..*fr
    };
    part_programs(g, img, &callee)
}

/// [`programs_step`] (60 bytes, 1 overlay).
pub const PROGRAMS_STEP: Sig = Sig {
    len: 60,
    hash: 0xe81c_2105_72ec_3a96,
    calls: &[
        (0x5, crate::Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, crate::Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x35, crate::Callee::Sig(&PART_PROGRAMS)),
    ],
    imports: &[],
};

/// Starts an actor's object (0x10003c30, cdecl `(p)`): with `i =
/// p[0x390]`, the object `O = T + 0x130 i` (`T` the immediate `+0x22`;
/// [`start_object_at`](crate::s4_misc::start_object_at), scale the float
/// at `+0x0c`, index 0, owners 0 and, for `i` 0, the immediate `+0x27`);
/// slot 325 `(O + 0xd0, p + 0x358)` (the start's words left under it);
/// `O + 0xc4` = `p[0xc4]` plus `A` (`+0x64`), less `W` (`+0x78`) above
/// `H` (`+0x6e`) or plus `W` (`+0x8f`) below `L` (`+0x82`). For `i` 0 the
/// record's words (operands `+0xa8`..`+0xee`, 0 but `+0xd0`/`+0xda`, the
/// immediates `+0xd4`/`+0xde`; the halfword `+0xf8` 0) and `O + 0x110` =
/// the immediate `+0x102`. Returns 0 for `i` 0, else slot 325's `eax`.
pub fn actor_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let p = fr.arg(g, 0);
    let scale = k(g, 0xc);
    let i = g.read::<u32>(p + 0x390);
    let o = i.wrapping_mul(0x130).wrapping_add(w(0x22));
    let b = if i == 0 { w(0x27) } else { 0 };
    crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x39), o, scale, 0, 0, b);
    let t = slot_target(g, w(0x4c), 325);
    let eax = g.call(t, 0, &[o + 0xd0, p + 0x358, 0, 0, i, b]).eax;
    let mut x = add(g.read::<f32>(p + 0xc4), k(g, 0x64));
    // comiss x, H; jbe: below, equal or unordered
    if x > k(g, 0x6e) {
        x = sub(x, k(g, 0x78));
    } else if k(g, 0x82) > x {
        x = add(x, k(g, 0x8f));
    }
    g.write::<f32>(o + 0xc4, x);
    if g.read::<u32>(p + 0x390) != 0 {
        return Ret::eax(eax);
    }
    for off in [0xa8, 0xb2, 0xbc, 0xc6, 0xe4, 0xee] {
        g.write::<u32>(w(off), 0);
    }
    g.write::<u32>(w(0xd0), w(0xd4));
    g.write::<u32>(w(0xda), w(0xde));
    g.write::<u16>(w(0xf8), 0);
    g.write::<u32>(o + 0x110, w(0x102));
    Ret::eax(0)
}

/// [`actor_start`] (266 bytes, 1 overlay).
pub const ACTOR_START: Sig = Sig {
    len: 266,
    hash: 0xfeea_eed5_2739_ac13,
    calls: &[(0x39, crate::Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};
