//! A fifth set of the own work list's small functions (round 4): rcef
//! slot chains and end checks. Operands are read from each copy's code
//! (`w(off)` = the word at `+off`).

use crate::l5_objhost::run_objects;
use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::s3_motion::motion_step;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The slot of a `mov eax, [eax + disp32]` with the `disp32` at `at`.
fn slot_at(img: &Image, at: u32) -> u32 {
    img.word(at) / 4
}

/// The effect's end of a task callback: `obj + 0x14` = 3 and the next
/// callback `cb` in `obj`'s task (`*(obj + 8) + 0x10`). Returns the task.
fn finish(g: &mut impl crate::Mem, obj: u32, cb: u32) -> u32 {
    g.write::<u32>(obj.wrapping_add(0x14), 3);
    let task = g.read::<u32>(obj.wrapping_add(8));
    g.write::<u32>(task.wrapping_add(0x10), cb);
    task
}

/// `(obj)` (`magic_0194` 0x100010a0, 73 bytes; 2 copies): the slot at
/// `+0x17` (447) with `(V, K)` (`V` the word at `*G + D`, `G` at `+0x04`,
/// `D` at `+0x0c`, `K` the `push imm8` at `+0x09`), `rcefTaskSetSeq(obj,
/// cb)` (`+0x2c`, `cb` at `+0x23`) and `rcefTaskWait(obj, T)` (`+0x3e`, `T`
/// at `+0x38`), each with the earlier words after its own. Returns the
/// last slot's `eax`.
pub fn seq_wait_short<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let k = disp8(img, f + 0x09);
    let s = slot_target(g, w(0x11), slot_at(img, f + 0x17));
    g.call(s, fr.ecx, &[v, k]);
    let obj = fr.arg(g, 0);
    let cb = w(0x23);
    let s = slot_target(g, w(0x1e), slot_at(img, f + 0x2c));
    g.call(s, fr.ecx, &[obj, cb, v, k]);
    let t = disp8(img, f + 0x38);
    let s = slot_target(g, w(0x33), slot_at(img, f + 0x3e));
    Ret::eax(g.call(s, fr.ecx, &[obj, t, obj, cb, v, k]).eax)
}

/// `(obj)` (`magic_0356` 0x100017a0, 75 bytes): the slot at `+0x19`
/// (388) with `(V, A, B)` (`V` the word at `*G + D`, `G` at `+0x04`, `D`
/// at `+0x0e`; the `push imm8` at `+0x0b`, `+0x09`), `rcefTaskSetSeq(obj,
/// cb)` (`+0x2e`, `cb` at `+0x25`) and `rcefTaskWait(obj, T)` (`+0x40`,
/// `T` at `+0x3a`), each with the earlier words after its own. Returns
/// the last slot's `eax`.
pub fn slot3_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0e)));
    let (a, b) = (disp8(img, f + 0x0b), disp8(img, f + 0x09));
    let s = slot_target(g, w(0x13), slot_at(img, f + 0x19));
    g.call(s, fr.ecx, &[v, a, b]);
    let obj = fr.arg(g, 0);
    let cb = w(0x25);
    let s = slot_target(g, w(0x20), slot_at(img, f + 0x2e));
    g.call(s, fr.ecx, &[obj, cb, v, a, b]);
    let t = disp8(img, f + 0x3a);
    let s = slot_target(g, w(0x35), slot_at(img, f + 0x40));
    Ret::eax(g.call(s, fr.ecx, &[obj, t, obj, cb, v, a, b]).eax)
}

/// `magic_0380` 0x10001b10 (71 bytes): seven words set (`mov dword
/// [addr], imm32`, 10 bytes each). `eax` as given.
pub fn seven_stores<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    for k in 0..7 {
        let insn = fr.at + 10 * k;
        g.write::<u32>(img.word(insn + 2), img.word(insn + 6));
    }
    Ret::eax(fr.eax)
}

/// `(obj)` (`magic_0521` 0x10001170, 74 bytes): `rcefMainCreateTask(*G,
/// cb)` (`+0x15`, `G` at `+0x0f`, `cb` at `+0x09`), `rcefTaskSetSeq(obj,
/// cb2)` (`+0x2a`, `+0x21`) and `rcefTaskCallSeq(obj, cb3)` (`+0x3f`,
/// `+0x36`), each with the earlier words after its own. Returns the last
/// slot's `eax`.
pub fn task_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = g.read::<u32>(w(0x0f));
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x15));
    g.call(s, fr.ecx, &[v, w(0x09)]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x1c), slot_at(img, f + 0x2a));
    g.call(s, fr.ecx, &[obj, w(0x21), v, w(0x09)]);
    let s = slot_target(g, w(0x31), slot_at(img, f + 0x3f));
    let r = g.call(s, fr.ecx, &[obj, w(0x36), obj, w(0x21), v, w(0x09)]);
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0529` 0x10001210, 74 bytes): `V` = the word at `*G +
/// D` (`G` at `+0x04`, `D` at `+0x0b`); the slots at `+0x18` (238) with
/// `(V, A)` and `+0x28` (213) with `(V, B)` (`A`, `B` the `push imm8` at
/// `+0x15`, `+0x25`), then `rcefTaskSetSeq(obj, cb)` (`+0x3e`, `+0x35`),
/// each with the earlier words after its own. Returns its `eax`.
pub fn two_slots_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0b)));
    let (a, b) = (disp8(img, f + 0x15), disp8(img, f + 0x25));
    let s = slot_target(g, w(0x10), slot_at(img, f + 0x18));
    g.call(s, fr.ecx, &[v, a]);
    let s = slot_target(g, w(0x20), slot_at(img, f + 0x28));
    g.call(s, fr.ecx, &[v, b, v, a]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x30), slot_at(img, f + 0x3e));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x35), v, b, v, a]).eax)
}

/// `(obj)` (`magic_0252` 0x10001540, 72 bytes): the motion steps
/// ([`motion_step`], `+0x05`) and the objects run ([`run_objects`],
/// `+0x0c`); when both are through (1), the slot at `+0x28` (238) with
/// `(*H, K)` (`H` at `+0x22`, `K` the `push imm8` at `+0x1f`) and the
/// effect ends (the callback at `+0x41`). Returns the task, else
/// `run_objects`' result.
pub fn done_slot_h<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = motion_step(g, img, img.rel32_target(f + 0x05));
    // push ebp; push esi; call
    let r = run_objects(g, img, img.rel32_target(f + 0x0c), fr.esp.wrapping_sub(0xc));
    if r != 1 || m != r {
        return Ret::eax(r);
    }
    let h = g.read::<u32>(w(0x22));
    let s = slot_target(g, w(0x1a), slot_at(img, f + 0x28));
    g.call(s, fr.ecx, &[h, disp8(img, f + 0x1f)]);
    let obj = fr.arg(g, 0);
    Ret::eax(finish(g, obj, w(0x41)))
}

/// `(obj)` (`magic_0492` 0x100031b0, 73 bytes): the motion steps and the
/// objects run (as [`done_slot_h`]); when both are through (1), the
/// counter `C` (`+0x1b`) is not above 0 and the halfword `*G + d8` (`G`
/// at `+0x23`, `d8` at `+0x2f`) has the bits at `+0x28` (0x8000), the
/// effect ends (the callback at `+0x42`). Returns the task; `run_objects`'
/// result when a check fails before the flag; `*G` when only the flag is
/// clear.
pub fn done_count_bit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = motion_step(g, img, img.rel32_target(f + 0x05));
    let r = run_objects(g, img, img.rel32_target(f + 0x0c), fr.esp.wrapping_sub(0xc));
    if r != 1 || m != r || g.read::<u32>(w(0x1b)).cast_signed() > 0 {
        return Ret::eax(r);
    }
    // mov eax, ds:G; test word [eax + d8], cx
    let p = g.read::<u32>(w(0x23));
    let bits = g.read::<u16>(p.wrapping_add(disp8(img, f + 0x2f)));
    if u32::from(bits) & w(0x28) == 0 {
        return Ret::eax(p);
    }
    let obj = fr.arg(g, 0);
    Ret::eax(finish(g, obj, w(0x42)))
}

/// `(obj)` (`magic_0514` 0x10003520, 73 bytes): `M` = the word at `+d2`
/// of the record at `+d1` of `*obj` (`d1` at `+0x0d`, `d2` at `+0x12`);
/// the slots at `+0x1b` and `+0x2a` with `(M, -1, -1)`, the slot at
/// `+0x3c` with `(M, 0)`, each with the earlier words after its own.
/// Returns the last slot's `eax`.
pub fn three_slots_obj<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| disp8(img, f + off);
    let obj = fr.arg(g, 0);
    let a = g.read::<u32>(obj);
    let rec = g.read::<u32>(a.wrapping_add(d(0x0d)));
    let m = g.read::<u32>(rec.wrapping_add(d(0x12)));
    let s = slot_target(g, w(0x14), slot_at(img, f + 0x1b));
    g.call(s, fr.ecx, &[m, d(0x0f), d(0x0a)]);
    let s = slot_target(g, w(0x22), slot_at(img, f + 0x2a));
    g.call(s, fr.ecx, &[m, d(0x2f), d(0x27), m, d(0x0f), d(0x0a)]);
    let s = slot_target(g, w(0x34), slot_at(img, f + 0x3c));
    let r = g.call(
        s,
        fr.ecx,
        &[m, d(0x39), m, d(0x2f), d(0x27), m, d(0x0f), d(0x0a)],
    );
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0382` 0x10001170, 76 bytes): `R` =
/// `rcefMainCreateTask(*G, cb)` (`+0x15`, `G` at `+0x0f`, `cb` at
/// `+0x09`, `R` at `+0x1c`), `rcefTaskSetSeq(obj, cb2)` (`+0x2f`,
/// `+0x26`) and `rcefTaskWait(obj, T)` (`+0x41`, `T` at `+0x3b`), each
/// with the earlier words after its own. Returns the last slot's `eax`.
pub fn task_keep_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = g.read::<u32>(w(0x0f));
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x15));
    let task = g.call(s, fr.ecx, &[v, w(0x09)]).eax;
    g.write::<u32>(w(0x1c), task);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x21), slot_at(img, f + 0x2f));
    g.call(s, fr.ecx, &[obj, w(0x26), v, w(0x09)]);
    let t = disp8(img, f + 0x3b);
    let s = slot_target(g, w(0x36), slot_at(img, f + 0x41));
    let r = g.call(s, fr.ecx, &[obj, t, obj, w(0x26), v, w(0x09)]);
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0522` 0x10001200, 76 bytes): the slot at `+0x17`
/// with `(V, K)` (`V` the word at `*G + D`, `G` at `+0x04`, `D` at
/// `+0x0c`, `K` the `push imm8` at `+0x09`), `rcefTaskSetSeq(obj, cb)`
/// (`+0x2c`, `+0x23`) and `rcefTaskCallSeq(obj, cb2)` (`+0x41`, `+0x38`),
/// each with the earlier words after its own. Returns the last slot's
/// `eax`.
pub fn field_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let k = disp8(img, f + 0x09);
    let s = slot_target(g, w(0x11), slot_at(img, f + 0x17));
    g.call(s, fr.ecx, &[v, k]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x1e), slot_at(img, f + 0x2c));
    g.call(s, fr.ecx, &[obj, w(0x23), v, k]);
    let s = slot_target(g, w(0x33), slot_at(img, f + 0x41));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x38), obj, w(0x23), v, k]).eax)
}

/// `(obj)` (`magic_0195` 0x10001300, 77 bytes): two
/// `rcefMainCreateTask(*G, cb)` (`+0x15`/`+0x2d`, `G` at `+0x0f`/`+0x27`,
/// `cb` at `+0x09`/`+0x21`), then `rcefTaskSetSeq(obj, cb3)` (`+0x42`,
/// `+0x39`), each with the earlier words after its own. Returns its
/// `eax`.
pub fn two_tasks_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x0f));
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x15));
    g.call(s, fr.ecx, &[a, w(0x09)]);
    let b = g.read::<u32>(w(0x27));
    let s = slot_target(g, w(0x1c), slot_at(img, f + 0x2d));
    g.call(s, fr.ecx, &[b, w(0x21), a, w(0x09)]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x34), slot_at(img, f + 0x42));
    Ret::eax(
        g.call(s, fr.ecx, &[obj, w(0x39), b, w(0x21), a, w(0x09)])
            .eax,
    )
}

/// `(obj)` (`magic_0360` 0x10001300, 78 bytes): the slot at `+0x0c`
/// (485) with the nine `push imm8` (the last pushed first: `+0x1f`,
/// `+0x1d`, `+0x1b`, `+0x19`, `+0x17`, `+0x15`, `+0x13`, `+0x11`,
/// `+0x09`), `rcefTaskSetSeq(obj, cb)` (`+0x31`, `+0x28`) and
/// `rcefTaskWait(obj, T)` (`+0x43`, `T` at `+0x3d`), each with the earlier
/// words after its own. Returns the last slot's `eax`.
pub fn slot9_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| disp8(img, f + off);
    let k = [
        d(0x1f),
        d(0x1d),
        d(0x1b),
        d(0x19),
        d(0x17),
        d(0x15),
        d(0x13),
        d(0x11),
        d(0x09),
    ];
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0c));
    g.call(s, fr.ecx, &k);
    let obj = fr.arg(g, 0);
    let cb = w(0x28);
    let s = slot_target(g, w(0x23), slot_at(img, f + 0x31));
    g.call(
        s,
        fr.ecx,
        &[
            obj, cb, k[0], k[1], k[2], k[3], k[4], k[5], k[6], k[7], k[8],
        ],
    );
    let t = d(0x3d);
    let s = slot_target(g, w(0x38), slot_at(img, f + 0x43));
    let r = g.call(
        s,
        fr.ecx,
        &[
            obj, t, obj, cb, k[0], k[1], k[2], k[3], k[4], k[5], k[6], k[7], k[8],
        ],
    );
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0195` 0x10001270, 79 bytes): `rcefMainCreateTask(*G,
/// cb)` (`+0x15`, `G` at `+0x0f`, `cb` at `+0x09`), the slot at `+0x2f`
/// (447) with `(V, K)` (`V` the word at `*G + D`, `G` again at `+0x1c`,
/// `D` at `+0x24`, `K` at `+0x21`), the slot at `+0x44` with `(obj, cb2)`
/// (`+0x3b`), each with the earlier words after its own. Returns its
/// `eax`.
pub fn task_field_cb<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x0f));
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x15));
    g.call(s, fr.ecx, &[a, w(0x09)]);
    let p = g.read::<u32>(w(0x1c));
    let v = g.read::<u32>(p.wrapping_add(w(0x24)));
    let k = disp8(img, f + 0x21);
    let s = slot_target(g, w(0x29), slot_at(img, f + 0x2f));
    g.call(s, fr.ecx, &[v, k, a, w(0x09)]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x36), slot_at(img, f + 0x44));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x3b), v, k, a, w(0x09)]).eax)
}

/// `(obj)` (`magic_0378` 0x10001120, 79 bytes): the slot at `+0x1a`
/// (239) with `(P[d8], P[D], K)` (`P` = `*G`, `G` at `+0x04`, `d8` at
/// `+0x12`, `D` at `+0x0c`, `K` the `push imm8` at `+0x09`),
/// `rcefTaskSetSeq(obj, cb)` (`+0x2f`, `+0x26`) and
/// `rcefTaskCallSeq(obj, cb2)` (`+0x44`, `+0x3b`), each with the earlier
/// words after its own. Returns the last slot's `eax`.
pub fn pair_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let u = g.read::<u32>(p.wrapping_add(disp8(img, f + 0x12)));
    let k = disp8(img, f + 0x09);
    let s = slot_target(g, w(0x14), slot_at(img, f + 0x1a));
    g.call(s, fr.ecx, &[u, v, k]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x21), slot_at(img, f + 0x2f));
    g.call(s, fr.ecx, &[obj, w(0x26), u, v, k]);
    let s = slot_target(g, w(0x36), slot_at(img, f + 0x44));
    Ret::eax(
        g.call(s, fr.ecx, &[obj, w(0x3b), obj, w(0x26), u, v, k])
            .eax,
    )
}

/// [`seq_wait_short`] in `magic_0194` (0x100010a0, 73 bytes).
pub const SEQ_WAIT_SHORT_0194_10A0: Sig = Sig {
    len: 73,
    hash: 0x87e5_1935_75ea_52af,
    calls: &[],
    imports: &[],
};

/// [`seq_wait_short`] in `magic_0525` (0x10001600, 73 bytes).
pub const SEQ_WAIT_SHORT_0525_1600: Sig = Sig {
    len: 73,
    hash: 0x5f55_bae3_a42a_a56c,
    calls: &[],
    imports: &[],
};

/// [`slot3_seq_wait`] in `magic_0356` (0x100017a0, 75 bytes).
pub const SLOT3_SEQ_WAIT_0356_17A0: Sig = Sig {
    len: 75,
    hash: 0x637b_0253_6ccf_d16a,
    calls: &[],
    imports: &[],
};

/// [`seven_stores`] in `magic_0380` (0x10001b10, 71 bytes).
pub const SEVEN_STORES_0380_1B10: Sig = Sig {
    len: 71,
    hash: 0xc454_bed1_b404_0eca,
    calls: &[],
    imports: &[],
};

/// [`task_seq_call`] in `magic_0521` (0x10001170, 74 bytes).
pub const TASK_SEQ_CALL_0521_1170: Sig = Sig {
    len: 74,
    hash: 0x95d6_4fef_3310_b5dd,
    calls: &[],
    imports: &[],
};

/// [`two_slots_seq`] in `magic_0529` (0x10001210, 74 bytes).
pub const TWO_SLOTS_SEQ_0529_1210: Sig = Sig {
    len: 74,
    hash: 0xdddc_a60a_d6da_0298,
    calls: &[],
    imports: &[],
};

/// [`done_slot_h`] in `magic_0252` (0x10001540, 72 bytes).
pub const DONE_SLOT_H_0252_1540: Sig = Sig {
    len: 72,
    hash: 0x9355_f316_ac8b_b49e,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};

/// [`done_count_bit`] in `magic_0492` (0x100031b0, 73 bytes).
pub const DONE_COUNT_BIT_0492_31B0: Sig = Sig {
    len: 73,
    hash: 0x23af_1adb_2319_3849,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};

/// [`three_slots_obj`] in `magic_0514` (0x10003520, 73 bytes).
pub const THREE_SLOTS_OBJ_0514_3520: Sig = Sig {
    len: 73,
    hash: 0x87d4_8d78_4d55_1e02,
    calls: &[],
    imports: &[],
};

/// [`task_keep_seq_wait`] in `magic_0382` (0x10001170, 76 bytes).
pub const TASK_KEEP_SEQ_WAIT_0382_1170: Sig = Sig {
    len: 76,
    hash: 0xb31d_d1bc_8cd7_89a4,
    calls: &[],
    imports: &[],
};

/// [`field_seq_call`] in `magic_0522` (0x10001200, 76 bytes).
pub const FIELD_SEQ_CALL_0522_1200: Sig = Sig {
    len: 76,
    hash: 0xde97_3348_c33b_3d18,
    calls: &[],
    imports: &[],
};

/// [`two_tasks_seq`] in `magic_0195` (0x10001300, 77 bytes).
pub const TWO_TASKS_SEQ_0195_1300: Sig = Sig {
    len: 77,
    hash: 0xefae_c8c3_e06e_2ee0,
    calls: &[],
    imports: &[],
};

/// [`slot9_seq_wait`] in `magic_0360` (0x10001300, 78 bytes).
pub const SLOT9_SEQ_WAIT_0360_1300: Sig = Sig {
    len: 78,
    hash: 0xbdf9_1c7f_29e8_0b89,
    calls: &[],
    imports: &[],
};

/// [`task_field_cb`] in `magic_0195` (0x10001270, 79 bytes).
pub const TASK_FIELD_CB_0195_1270: Sig = Sig {
    len: 79,
    hash: 0x3746_0453_c2ef_3673,
    calls: &[],
    imports: &[],
};

/// [`pair_seq_call`] in `magic_0378` (0x10001120, 79 bytes).
pub const PAIR_SEQ_CALL_0378_1120: Sig = Sig {
    len: 79,
    hash: 0xf06b_f2b9_2d61_8ada,
    calls: &[],
    imports: &[],
};
