//! More of the own work list's small functions (round 4): slot calls,
//! float chains and the "effect done" steps of the task callbacks
//! (`obj + 0x14` = 3 and the task's next callback set once the objects
//! and the motion or script are through). Operands are read from each
//! copy's code (`w(off)` = the word at `+off`).

use crate::l5_objhost::run_objects;
use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::s3_misc::fade_task;
use crate::s3_motion::motion_step;
use crate::sa_pool::give_back;
use crate::sa_script::run as script_run;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The slot of a `mov eax, [eax + disp32]` with the `disp32` at `at`.
fn slot_at(img: &Image, at: u32) -> u32 {
    img.word(at) / 4
}

/// The float at `at`.
fn f32_at(g: &impl crate::Mem, at: u32) -> f32 {
    f32::from_bits(g.read::<u32>(at))
}

/// The effect's end of a task callback: `obj + 0x14` = 3 and the next
/// callback `cb` in `obj`'s task (`*(obj + 8) + 0x10`). Returns the task.
fn finish(g: &mut impl crate::Mem, obj: u32, cb: u32) -> u32 {
    g.write::<u32>(obj.wrapping_add(0x14), 3);
    let task = g.read::<u32>(obj.wrapping_add(8));
    g.write::<u32>(task.wrapping_add(0x10), cb);
    task
}

/// `magic_0391` 0x100031a0 (41 bytes): the slot at `+0x0e` (624) with
/// the handle at `*G + D` (`G` at `+0x08`, `D` at `+0x14`), then the
/// handle set to the immediate at `+0x23` (-1; the field again at
/// `+0x1f`). Returns the slot's `eax`.
pub fn handle_release<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x08));
    let s = slot_target(g, w(0x01), slot_at(img, fr.at + 0x0e));
    let h = g.read::<u32>(p.wrapping_add(w(0x14)));
    let r = g.call(s, fr.ecx, &[h]).eax;
    g.write::<u32>(p.wrapping_add(w(0x1f)), w(0x23));
    Ret::eax(r)
}

/// `magic_0391` 0x10001680 (49 bytes; 2 copies): the handle at `*G + D`
/// (`G` at `+0x0e`, `D` at `+0x2b`) = the slot at `+0x08` (623) with
/// `(*H, 0, 0, 0, 0, 0, 0)` (`H` at `+0x20`). Returns it.
pub fn handle_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = slot_target(g, w(0x01), slot_at(img, fr.at + 0x08));
    let p = g.read::<u32>(w(0x0e));
    let h = g.read::<u32>(w(0x20));
    let r = g.call(s, fr.ecx, &[h, 0, 0, 0, 0, 0, 0]).eax;
    g.write::<u32>(p.wrapping_add(w(0x2b)), r);
    Ret::eax(r)
}

/// `magic_0275` 0x10004160 (42 bytes): when the code byte `*P` (`P` =
/// `*G`, `+0x01`) is the immediate at `+0x07` (0xd), the slot at `+0x20`
/// with `(TABLE[P[1]], 1, 1)` (`TABLE` at `+0x15`, the immediates at
/// `+0x11` and `+0x0f`). Returns the slot's `eax`, else `P`.
pub fn code_slot<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    if u32::from(g.read::<u8>(p)) != w(0x07) & 0xff {
        return Ret::eax(p);
    }
    let i = u32::from(g.read::<u8>(p.wrapping_add(1)));
    let v = g.read::<u32>(w(0x15).wrapping_add(4 * i));
    let s = slot_target(g, w(0x1a), slot_at(img, f + 0x20));
    Ret::eax(
        g.call(s, fr.ecx, &[v, disp8(img, f + 0x11), disp8(img, f + 0x0f)])
            .eax,
    )
}

/// No arguments (`magic_0511` 0x10002af0, 42 bytes): a fade record at
/// `*G + D` (`G` at `+0x02`, `D` at `+0x1a`) started ([`fade_task`],
/// `+0x24`: `xmm1`/`xmm2` the floats at `+0x12`/`+0x0a`, `edx` the
/// immediate at `+0x1f`, the stack word 0, written where the original
/// pushes it). Returns the fade's node.
pub fn fade_at<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x02));
    let (x2, x1) = (g.read::<u32>(w(0x0a)), g.read::<u32>(w(0x12)));
    // push 0
    g.write::<u32>(fr.esp.wrapping_sub(4), 0);
    let callee = Frame {
        at: img.rel32_target(f + 0x24),
        esp: fr.esp.wrapping_sub(8),
        eax: 0,
        ecx: p.wrapping_add(w(0x1a)),
        edx: w(0x1f),
        xmm: [0, u128::from(x1), u128::from(x2), 0],
    };
    Ret::eax(fade_task(g, img, &callee))
}

/// `(obj)` (`magic_0186` 0x10005360, 43 bytes): the slot of `[eax +
/// disp8]` (`+0x23`) with `(P[A], P[B], K)`, `P` = `*obj` (`A` at
/// `+0x18`, `B` at `+0x12`, `K` the immediate at `+0x0c`). Returns its
/// `eax`.
pub fn slot_pair_k<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let p = g.read::<u32>(obj);
    let b = g.read::<u32>(p.wrapping_add(w(0x12)));
    let a = g.read::<u32>(p.wrapping_add(w(0x18)));
    let s = slot_target(g, w(0x1d), disp8(img, f + 0x23) / 4);
    Ret::eax(g.call(s, fr.ecx, &[a, b, w(0x0c)]).eax)
}

/// `(obj)` (`magic_0416` 0x10001d80, 43 bytes; 2 copies): the slot at
/// `+0x1a` (447) with `(V, K)` (`V` the word at `*G + D`, `G` at `+0x04`,
/// `D` at `+0x0f`; `K` the `push imm32` at `+0x09`), then `obj` gets bit
/// 2. Returns `obj`.
pub fn slot_field_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0f)));
    let s = slot_target(g, w(0x14), slot_at(img, f + 0x1a));
    g.call(s, fr.ecx, &[v, w(0x09)]);
    let obj = fr.arg(g, 0);
    let x = g.read::<u32>(obj);
    g.write::<u32>(obj, x | 4);
    Ret::eax(obj)
}

/// `(obj)` (`magic_0193` 0x100014a0, 44 bytes): the slot at `+0x0c`
/// with the `push imm8` at `+0x09`, then the slot at `+0x21` with `(obj,
/// cb)` (`cb` at `+0x18`; the first call's word after them). Returns its
/// `eax`.
pub fn slot_then_cb<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = disp8(img, f + 0x09);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0c));
    g.call(s, fr.ecx, &[k]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x13), slot_at(img, f + 0x21));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x18), k]).eax)
}

/// No arguments (`magic_0304` 0x100037b0, 44 bytes; 2 copies): `r` = the
/// slot at `+0x12` (219) with `(B, 0)`, `B` the byte at `*G` (`+0x01`);
/// then the slot at `+0x22` (606) with `(r, K)` (`K` at `+0x19`; the first
/// call's words after them). Returns its `eax`.
pub fn byte_slot_chain<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x01))));
    let s = slot_target(g, w(0x0c), slot_at(img, f + 0x12));
    let r = g.call(s, fr.ecx, &[b, 0]).eax;
    let s = slot_target(g, w(0x1c), slot_at(img, f + 0x22));
    Ret::eax(g.call(s, fr.ecx, &[r, disp8(img, f + 0x19), b, 0]).eax)
}

/// No arguments (`magic_0669` 0x10001000, 44 bytes): the slot at `+0x09`
/// (269) with `(B, A, K)` (`B` at `+0x13`, `A` at `+0x0e`, `K` the `push
/// imm8` at `+0x06`), then `G = V` (`+0x1e`, `+0x22`). Returns 1.
pub fn slot3_then_store<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x01), slot_at(img, f + 0x09));
    g.call(s, fr.ecx, &[w(0x13), w(0x0e), disp8(img, f + 0x06)]);
    g.write::<u32>(w(0x1e), w(0x22));
    Ret::eax(1)
}

/// `(obj)` (`magic_0525` 0x10001190, 47 bytes): `rcefTaskSetSeq(obj,
/// cb)` (the slot at `+0x12`, `cb` at `+0x09`) and `rcefTaskWait(obj,
/// T)` (`+0x24`, `T` the `push imm8` at `+0x1e`; the first call's words
/// after them). Returns its `eax`.
pub fn seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x12));
    g.call(s, fr.ecx, &[obj, w(0x09)]);
    let s = slot_target(g, w(0x19), slot_at(img, f + 0x24));
    Ret::eax(
        g.call(s, fr.ecx, &[obj, disp8(img, f + 0x1e), obj, w(0x09)])
            .eax,
    )
}

/// `(obj)` (`magic_0101` 0x100031a0, 47 bytes): the objects run
/// ([`run_objects`], `+0x04`); when none ran and the flag word `G`
/// (`+0x0f`) has the bit at `+0x13` (0x8000), the effect ends
/// ([`finish`], the callback at `+0x29`). Returns the task, else what
/// `run_objects` returned.
pub fn done_when_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // push ebp; call
    let r = run_objects(g, img, img.rel32_target(f + 0x04), fr.esp.wrapping_sub(8));
    if r != 1 || g.read::<u32>(w(0x0f)) & w(0x13) == 0 {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    Ret::eax(finish(g, obj, w(0x29)))
}

/// No arguments (`magic_0661` 0x10001670, 49 bytes): `G = V` (`+0x1e`,
/// `+0x22`), then the slot at `+0x0c` (420) with the four immediates at
/// `+0x1b`, `+0x16`, `+0x11`, `+0x06` (first word first). Returns 1.
pub fn store_then_slot4<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x01), slot_at(img, f + 0x0c));
    g.write::<u32>(w(0x1e), w(0x22));
    g.call(
        s,
        fr.ecx,
        &[disp8(img, f + 0x1b), w(0x16), w(0x11), w(0x06)],
    );
    Ret::eax(1)
}

/// No arguments (`magic_0661` 0x100016b0, 49 bytes): the counter `G`
/// (`+0x01`) less the `imm8` at `+0x07`; below 0 it becomes 0 (`+0x0c`)
/// and 0 is returned; else it is kept at most the immediate at `+0x18`
/// (0x80; `+0x22`) and whether it is not 0 is returned.
pub fn count_down<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = g
        .read::<u32>(w(0x01))
        .wrapping_sub(disp8(img, f + 0x07))
        .cast_signed();
    if v < 0 {
        g.write::<u32>(w(0x0c), 0);
        return Ret::eax(0);
    }
    let hi = w(0x18).cast_signed();
    // cmovg
    let v = if v > hi { hi } else { v };
    g.write::<u32>(w(0x22), v.cast_unsigned());
    Ret::eax(u32::from(v != 0))
}

/// `(node)` (`magic_0672` 0x10001480, 49 bytes): the script runs
/// ([`script_run`], `+0x0c`) and the motion steps ([`motion_step`],
/// `+0x13`); when the script ended (1), `*G + D` = 1 (`G` at `+0x07`, `D`
/// at `+0x21`) and the node goes back to the pool ([`give_back`], a tail
/// jump, `+0x29`). Returns the motion step's result otherwise.
pub fn script_motion_give_back<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x07));
    let a = script_run(g, img, img.rel32_target(f + 0x0c));
    let m = motion_step(g, img, img.rel32_target(f + 0x13));
    if a != 1 {
        return Ret::eax(m);
    }
    let node = fr.arg(g, 0);
    g.write::<u32>(p.wrapping_add(w(0x21)), a);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x29), node))
}

/// No arguments (`magic_0168` 0x100033c0, 50 bytes; 2 copies): on `P` =
/// `*G` (`+0x01`): `x = P[A] - P[B]`, `y = P[C] - x` (the fields at
/// `+0x09`, `+0x11`, `+0x19`); `P[D] = x`, `P[E] = y` (`+0x25`, `+0x2d`).
/// Returns `P`.
pub fn sub_chain<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let at = |o: u32| p.wrapping_add(w(o));
    let x = sub(f32_at(g, at(0x09)), f32_at(g, at(0x11)));
    let y = sub(f32_at(g, at(0x19)), x);
    g.write::<u32>(at(0x25), x.to_bits());
    g.write::<u32>(at(0x2d), y.to_bits());
    Ret::eax(p)
}

/// [`sub_chain`] with `addss` (`magic_0563` 0x100024f0, 50 bytes).
pub fn add_chain<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let at = |o: u32| p.wrapping_add(w(o));
    let x = add(f32_at(g, at(0x09)), f32_at(g, at(0x11)));
    let y = add(f32_at(g, at(0x19)), x);
    g.write::<u32>(at(0x25), x.to_bits());
    g.write::<u32>(at(0x2d), y.to_bits());
    Ret::eax(p)
}

/// No arguments (`magic_0307` 0x100029f0, 50 bytes): on `P` = `*G`
/// (`+0x01`): `x = P[A] + P[B]` stored at `P[C]` (`+0x09`, `+0x11`,
/// `+0x19`), then `P[E] = P[D] + x` (`+0x21`, `+0x2d`). Returns `P`.
pub fn add_chain_stored<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let at = |o: u32| p.wrapping_add(w(o));
    let x = add(f32_at(g, at(0x09)), f32_at(g, at(0x11)));
    g.write::<u32>(at(0x19), x.to_bits());
    let y = add(f32_at(g, at(0x21)), x);
    g.write::<u32>(at(0x2d), y.to_bits());
    Ret::eax(p)
}

/// [`handle_release`] in `magic_0391` (0x100031a0, 41 bytes).
pub const HANDLE_RELEASE_0391_31A0: Sig = Sig {
    len: 41,
    hash: 0x117c_3154_bdd4_d7bf,
    calls: &[],
    imports: &[],
};

/// [`handle_start`] in `magic_0391` (0x10001680, 49 bytes).
pub const HANDLE_START_0391_1680: Sig = Sig {
    len: 49,
    hash: 0x8ad2_6e9f_52e5_9e1e,
    calls: &[],
    imports: &[],
};

/// [`handle_start`] in `magic_0392` (0x10001e10, 49 bytes).
pub const HANDLE_START_0392_1E10: Sig = Sig {
    len: 49,
    hash: 0x7b79_ae09_6a0c_d2c3,
    calls: &[],
    imports: &[],
};

/// [`code_slot`] in `magic_0275` (0x10004160, 42 bytes).
pub const CODE_SLOT_0275_4160: Sig = Sig {
    len: 42,
    hash: 0x6e50_e0a8_3954_2762,
    calls: &[],
    imports: &[],
};

/// [`fade_at`] in `magic_0511` (0x10002af0, 42 bytes).
pub const FADE_AT_0511_2AF0: Sig = Sig {
    len: 42,
    hash: 0xc0f1_2942_6978_9c29,
    calls: &[(0x24, Callee::Sig(&crate::s3_misc::FADE_TASK))],
    imports: &[],
};

/// [`slot_pair_k`] in `magic_0186` (0x10005360, 43 bytes).
pub const SLOT_PAIR_K_0186_5360: Sig = Sig {
    len: 43,
    hash: 0x282d_c5e6_7635_b52f,
    calls: &[],
    imports: &[],
};

/// [`slot_field_flag`] in `magic_0416` (0x10001d80, 43 bytes).
pub const SLOT_FIELD_FLAG_0416_1D80: Sig = Sig {
    len: 43,
    hash: 0xf059_5536_71dd_ecd2,
    calls: &[],
    imports: &[],
};

/// [`slot_field_flag`] in `magic_0521` (0x10001400, 43 bytes).
pub const SLOT_FIELD_FLAG_0521_1400: Sig = Sig {
    len: 43,
    hash: 0xda0e_1ecc_2d7e_4eb0,
    calls: &[],
    imports: &[],
};

/// [`slot_then_cb`] in `magic_0193` (0x100014a0, 44 bytes).
pub const SLOT_THEN_CB_0193_14A0: Sig = Sig {
    len: 44,
    hash: 0xab5f_8c39_ff69_56a5,
    calls: &[],
    imports: &[],
};

/// [`byte_slot_chain`] in `magic_0304` (0x100037b0, 44 bytes).
pub const BYTE_SLOT_CHAIN_0304_37B0: Sig = Sig {
    len: 44,
    hash: 0x8ade_dd83_5202_69cc,
    calls: &[],
    imports: &[],
};

/// [`byte_slot_chain`] in `magic_0304` (0x100037e0, 44 bytes).
pub const BYTE_SLOT_CHAIN_0304_37E0: Sig = Sig {
    len: 44,
    hash: 0xd116_f462_365f_9427,
    calls: &[],
    imports: &[],
};

/// [`slot3_then_store`] in `magic_0669` (0x10001000, 44 bytes).
pub const SLOT3_THEN_STORE_0669_1000: Sig = Sig {
    len: 44,
    hash: 0x99fd_f682_78c3_064a,
    calls: &[],
    imports: &[],
};

/// [`seq_wait`] in `magic_0525` (0x10001190, 47 bytes).
pub const SEQ_WAIT_0525_1190: Sig = Sig {
    len: 47,
    hash: 0x1862_ecf5_f359_2fb0,
    calls: &[],
    imports: &[],
};

/// [`done_when_flag`] in `magic_0101` (0x100031a0, 47 bytes).
pub const DONE_WHEN_FLAG_0101_31A0: Sig = Sig {
    len: 47,
    hash: 0xfd8c_6b6a_db24_1004,
    calls: &[(0x4, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`store_then_slot4`] in `magic_0661` (0x10001670, 49 bytes).
pub const STORE_THEN_SLOT4_0661_1670: Sig = Sig {
    len: 49,
    hash: 0xc452_b534_f3a5_6670,
    calls: &[],
    imports: &[],
};

/// [`count_down`] in `magic_0661` (0x100016b0, 49 bytes).
pub const COUNT_DOWN_0661_16B0: Sig = Sig {
    len: 49,
    hash: 0x0ff4_b379_980c_689e,
    calls: &[],
    imports: &[],
};

/// [`script_motion_give_back`] in `magic_0672` (0x10001480, 49 bytes).
pub const SCRIPT_MOTION_GIVE_BACK_0672_1480: Sig = Sig {
    len: 49,
    hash: 0xdde7_6a9e_fc44_2d1e,
    calls: &[
        (0xc, Callee::Sig(&crate::sa_script::RUN)),
        (0x13, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x29, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`sub_chain`] in `magic_0168` (0x100033c0, 50 bytes).
pub const SUB_CHAIN_0168_33C0: Sig = Sig {
    len: 50,
    hash: 0xca3f_d4f8_831a_5492,
    calls: &[],
    imports: &[],
};

/// [`sub_chain`] in `magic_0587` (0x10002ee0, 50 bytes).
pub const SUB_CHAIN_0587_2EE0: Sig = Sig {
    len: 50,
    hash: 0x8322_d5e6_34df_9132,
    calls: &[],
    imports: &[],
};

/// [`add_chain`] in `magic_0563` (0x100024f0, 50 bytes).
pub const ADD_CHAIN_0563_24F0: Sig = Sig {
    len: 50,
    hash: 0xf1db_4671_0135_26ef,
    calls: &[],
    imports: &[],
};

/// [`add_chain_stored`] in `magic_0307` (0x100029f0, 50 bytes).
pub const ADD_CHAIN_STORED_0307_29F0: Sig = Sig {
    len: 50,
    hash: 0xc128_1db2_66d6_0edc,
    calls: &[],
    imports: &[],
};
