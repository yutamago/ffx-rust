//! A fourth set of single-DLL rcef chains and slot steps from the own
//! work list (round 4). Operands are read from the code (`w(off)` = the
//! word at `+off`; a slot is its `disp32` / 4).

use crate::l5_objhost::{list_remove, run_objects};
use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::sa_pool::give_back;
use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Calls the slot of the table global at `table` and the `disp32` at
/// `disp` with `words` in front of `stack` (the earlier calls' words,
/// which the original leaves until its `add esp`). Returns its `eax`.
fn push_call<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    stack: &mut Vec<u32>,
    (table, disp): (u32, u32),
    words: &[u32],
) -> u32 {
    stack.splice(0..0, words.iter().copied());
    let s = slot_target(g, img.word(fr.at + table), img.word(fr.at + disp) / 4);
    // call width: 16 at most
    g.call(s, fr.ecx, stack).eax
}

/// `(obj)` (`magic_0360` 0x100010a0, 109 bytes): slot 473 (`+0x0c`) with
/// `(0, 0, 0, 0x40, 0xf)` (the immediates at `+0x17`, `+0x15`, `+0x13`,
/// `+0x11`, `+0x09`), slot 444 (`+0x23`) with `(K)` (`+0x20`),
/// `rcefMainCreateTask(*G, cb)` (`+0x3b`, `G` at `+0x35`, `cb` at
/// `+0x2f`), `rcefTaskSetSeq(obj, cb2)` (`+0x50`, `+0x47`) and
/// `rcefTaskWait(obj, T)` (`+0x62`, `T` at `+0x5c`), each with the
/// earlier words after its own. Returns the last slot's `eax`.
pub fn effect_task_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| disp8(img, f + off);
    let mut st = Vec::new();
    let five = [d(0x17), d(0x15), d(0x13), d(0x11), d(0x09)];
    push_call(g, img, fr, &mut st, (0x04, 0x0c), &five);
    push_call(g, img, fr, &mut st, (0x1b, 0x23), &[d(0x20)]);
    let a = g.read::<u32>(w(0x35));
    push_call(g, img, fr, &mut st, (0x2a, 0x3b), &[a, w(0x2f)]);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x42, 0x50), &[obj, w(0x47)]);
    let t = d(0x5c);
    Ret::eax(push_call(g, img, fr, &mut st, (0x57, 0x62), &[obj, t]))
}

/// `(obj)` (`magic_0525` 0x10001000, 94 bytes): `R` = slot 367 (`+0x24`)
/// with `(P[4], P[0x1a0], 0, 0, 0, 0, 0, 0)` (`P` = `*G`, `G` at `+0x04`;
/// the fields at `+0x1c`, `+0x16`), kept at `+0x2b`; `rcefTaskSetSeq(obj,
/// cb)` (`+0x3e`, `+0x35`) and `rcefTaskCallSeq(obj, cb2)` (`+0x53`,
/// `+0x4a`), each with the earlier words after its own. Returns the last
/// slot's `eax`.
pub fn effect_keep_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let a = g.read::<u32>(p.wrapping_add(disp8(img, f + 0x1c)));
    let b = g.read::<u32>(p.wrapping_add(w(0x16)));
    let mut st = Vec::new();
    let r = push_call(g, img, fr, &mut st, (0x1e, 0x24), &[a, b, 0, 0, 0, 0, 0, 0]);
    g.write::<u32>(w(0x2b), r);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x30, 0x3e), &[obj, w(0x35)]);
    Ret::eax(push_call(
        g,
        img,
        fr,
        &mut st,
        (0x45, 0x53),
        &[obj, w(0x4a)],
    ))
}

/// `(obj)` (`magic_0534` 0x10001040, 90 bytes): slot 433 (`+0x0b`) with
/// `(K)` (`+0x10`); `rcefTaskSetSeq(obj, cb)` (`+0x2c`) with `cb` the
/// operand at `+0x33` while `P[0x19c]` (`P` = `*G`, `G` at `+0x14`) is
/// above 1, else the one at `+0x3a`; `rcefTaskCallSeq(obj, cb2)` (`+0x4d`,
/// `+0x47`; the first call's words after them). Returns its `eax`.
pub fn seq_by_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x04), w(0x0b) / 4);
    g.call(s, fr.ecx, &[disp8(img, f + 0x10)]);
    let p = g.read::<u32>(w(0x14));
    let obj = fr.arg(g, 0);
    let many = g.read::<u32>(p.wrapping_add(0x19c)).cast_signed() > 1;
    let cb = if many { w(0x33) } else { w(0x3a) };
    let s = slot_target(g, w(0x26), w(0x2c) / 4);
    g.call(s, fr.ecx, &[obj, cb]);
    let s = slot_target(g, w(0x42), w(0x4d) / 4);
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x47), obj, cb]).eax)
}

/// No arguments (`magic_0536` 0x100011a0, 84 bytes): `X` (`+0x04`) plus
/// the step at `+0x0c`, kept at most the limit at `+0x14` (`comiss`;
/// stored at `+0x1f` and, when capped, `+0x2c`); slot 38 (`+0x39`) with
/// `(*H, X, 0)` (`H` at `+0x45`). Returns 1.
pub fn step_capped<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let f32_at = |g: &G, a: u32| f32::from_bits(g.read::<u32>(a));
    let x = add(f32_at(g, w(0x04)), f32_at(g, w(0x0c)));
    let limit = f32_at(g, w(0x14));
    g.write::<u32>(w(0x1f), x.to_bits());
    // comiss x, limit; jb: kept when below (or unordered)
    let x = if x < limit || x.is_nan() || limit.is_nan() {
        x
    } else {
        g.write::<u32>(w(0x2c), limit.to_bits());
        limit
    };
    let h = g.read::<u32>(w(0x45));
    let s = slot_target(g, w(0x31), w(0x39) / 4);
    g.call(s, fr.ecx, &[h, x.to_bits(), 0]);
    Ret::eax(1)
}

/// No arguments (`magic_0185` 0x10001500, 93 bytes): three parts freed:
/// for each of the buffers at `+0x06`, `+0x23`, `+0x40`, `*G_P = ITEM`
/// (`G_P` at `+0x01`/`+0x1e`/`+0x3b`, `ITEM` at `+0x0c`/`+0x29`/`+0x46`)
/// and slot 713 (`+0x17`/`+0x34`/`+0x51`) with the buffer (the earlier
/// calls' words after it). Returns 0.
pub fn free_three<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut st = Vec::new();
    for (buf, gp, item, table, disp) in [
        (0x06, 0x01, 0x0c, 0x11, 0x17),
        (0x23, 0x1e, 0x29, 0x2e, 0x34),
        (0x40, 0x3b, 0x46, 0x4b, 0x51),
    ] {
        let p = g.read::<u32>(w(gp));
        g.write::<u32>(p, w(item));
        push_call(g, img, fr, &mut st, (table, disp), &[w(buf)]);
    }
    Ret::eax(0)
}

/// `(obj)` (`magic_0163` 0x10001cb0, 95 bytes): the countdown `C`
/// (`+0x04`/`+0x0e`), when above 0, down by one; reaching 0, `r` = slot
/// 719 (`+0x1d`) with `(P, 9, 0)` (`P` at `+0x24`, the immediates at
/// `+0x22`, `+0x1a`) and slot 306 (`+0x34`) with `(7, 0, r)` (`+0x39`,
/// `+0x31`; the first call's words after them). Then the objects run
/// ([`run_objects`], `+0x40`); when none ran, the effect ends (`obj +
/// 0x14` = 3, the task's callback the one at `+0x59`). Returns the task,
/// else `run_objects`' result.
pub fn countdown_then_done<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| disp8(img, f + off);
    let c = g.read::<u32>(w(0x04));
    if c.cast_signed() > 0 {
        g.write::<u32>(w(0x0e), c - 1);
        if c == 1 {
            let mut st = Vec::new();
            let r = push_call(
                g,
                img,
                fr,
                &mut st,
                (0x15, 0x1d),
                &[w(0x24), d(0x22), d(0x1a)],
            );
            push_call(g, img, fr, &mut st, (0x2c, 0x34), &[d(0x39), d(0x31), r]);
        }
    }
    // push ebp; call
    let r = run_objects(g, img, img.rel32_target(f + 0x40), fr.esp.wrapping_sub(8));
    if r != 1 {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    g.write::<u32>(obj.wrapping_add(0x14), 3);
    let task = g.read::<u32>(obj.wrapping_add(8));
    g.write::<u32>(task.wrapping_add(0x10), w(0x59));
    Ret::eax(task)
}

/// No arguments (`magic_0353` 0x10002e00, 90 bytes): the object `*G`
/// (`+0x03`) taken off the list `L` ([`list_remove`], `+0x0f`, `L` at
/// `+0x0a`); if it was there, `*G_P = ITEM` (`+0x18`, `+0x1f`), slot 713
/// (`+0x2a`) frees its part and its flags (`u16 +0x124`) lose 0x8000.
/// Then its pool node `+0x13e0`, if any, goes back ([`give_back`],
/// `+0x4a`) and the field is cleared. Returns what the last call left in
/// `eax` (0 when the object was not on the list and had no node).
pub fn remove_and_release<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = g.read::<u32>(w(0x03));
    let mut eax = list_remove(g, img, img.rel32_target(f + 0x0f), w(0x0a), obj);
    if eax != 0 {
        let gp = g.read::<u32>(w(0x18));
        g.write::<u32>(gp, w(0x1f));
        let s = slot_target(g, w(0x24), w(0x2a) / 4);
        g.call(s, fr.ecx, &[obj]);
        let flags = g.read::<u16>(obj.wrapping_add(0x124));
        g.write::<u16>(obj.wrapping_add(0x124), flags & 0x7fff);
        eax = 0x7fff;
    }
    let node = g.read::<u32>(obj.wrapping_add(w(0x41)));
    if node != 0 {
        eax = give_back(g, img, img.rel32_target(f + 0x4a), node);
        g.write::<u32>(obj.wrapping_add(w(0x50)), 0);
    }
    Ret::eax(eax)
}

/// [`effect_task_seq_wait`] in `magic_0360` (0x100010a0, 109 bytes).
pub const EFFECT_TASK_SEQ_WAIT_0360_10A0: Sig = Sig {
    len: 109,
    hash: 0x45e0_5539_cf29_9cd5,
    calls: &[],
    imports: &[],
};

/// [`effect_keep_seq_call`] in `magic_0525` (0x10001000, 94 bytes).
pub const EFFECT_KEEP_SEQ_CALL_0525_1000: Sig = Sig {
    len: 94,
    hash: 0x1c4c_8b3e_8a02_b52b,
    calls: &[],
    imports: &[],
};

/// [`seq_by_count`] in `magic_0534` (0x10001040, 90 bytes).
pub const SEQ_BY_COUNT_0534_1040: Sig = Sig {
    len: 90,
    hash: 0xb36f_275c_0d86_df0f,
    calls: &[],
    imports: &[],
};

/// [`step_capped`] in `magic_0536` (0x100011a0, 84 bytes).
pub const STEP_CAPPED_0536_11A0: Sig = Sig {
    len: 84,
    hash: 0x0d7f_9fb4_cc28_40c0,
    calls: &[],
    imports: &[],
};

/// [`free_three`] in `magic_0185` (0x10001500, 93 bytes).
pub const FREE_THREE_0185_1500: Sig = Sig {
    len: 93,
    hash: 0x42b9_8ead_3dc4_ce14,
    calls: &[],
    imports: &[],
};

/// [`countdown_then_done`] in `magic_0163` (0x10001cb0, 95 bytes).
pub const COUNTDOWN_THEN_DONE_0163_1CB0: Sig = Sig {
    len: 95,
    hash: 0x737a_11c7_ffe0_5c34,
    calls: &[(0x40, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`remove_and_release`] in `magic_0353` (0x10002e00, 90 bytes).
pub const REMOVE_AND_RELEASE_0353_2E00: Sig = Sig {
    len: 90,
    hash: 0xa67f_22ab_2912_98f5,
    calls: &[
        (0xf, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x4a, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
