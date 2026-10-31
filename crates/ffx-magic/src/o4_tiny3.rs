//! A third set of the own work list's small functions (round 4): the
//! task callbacks' "effect done" checks in their variants (objects,
//! motion, script, flags), static record set-ups and float steps.
//! Operands are read from each copy's code (`w(off)` = the word at
//! `+off`).

use crate::Xmm;
use crate::l5_objhost::run_objects;
use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::s3_motion::motion_step;
use crate::sa_misc::register_parts;
use crate::sa_pool::give_back;
use crate::sa_script::run as script_run;
use crate::sb_misc::draw_grid;
use crate::sb_sse::{add, mul, sub};
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

/// `(obj)` (`magic_0473` 0x100017c0, 51 bytes): the objects run
/// ([`run_objects`], `+0x0b`); when none ran and `*G + D` is 1 (`G` at
/// `+0x06`, `D` at `+0x16`), the effect ends ([`finish`], `+0x2c`).
/// Returns the task, else `run_objects`' result.
pub fn done_field<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x06));
    // push ebp; push esi; call
    let r = run_objects(g, img, img.rel32_target(f + 0x0b), fr.esp.wrapping_sub(0xc));
    if r != 1 || g.read::<u32>(p.wrapping_add(w(0x16))) != r {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    Ret::eax(finish(g, obj, w(0x2c)))
}

/// `(obj)` (`magic_0446` 0x10004ba0, 58 bytes): [`done_field`] (the
/// callback at `+0x2d`) ending in the part registration
/// ([`register_parts`], a tail jump, `+0x33`) with the task.
pub fn done_field_register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x06));
    let r = run_objects(g, img, img.rel32_target(f + 0x0b), fr.esp.wrapping_sub(0xc));
    if r != 1 || g.read::<u32>(p.wrapping_add(w(0x16))) != r {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    let task = finish(g, obj, w(0x2d));
    Ret::eax(register_parts(g, img, img.rel32_target(f + 0x33), task))
}

/// No arguments (`magic_0391` 0x10001380, 58 bytes; 2 copies): the
/// objects run ([`run_objects`], `+0x08`); when none ran, the word `B`
/// (`+0x13`) is 0 and `*G + D` is 1 (`G` at `+0x03`, `D` at `+0x1c`),
/// slot 214 ends the effect (`(*H)`, `H` at `+0x29`, the slot at
/// `+0x2f`). Returns its `eax`, else `run_objects`' result.
pub fn end_when_idle<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x03));
    // push esi; call
    let r = run_objects(g, img, img.rel32_target(f + 0x08), fr.esp.wrapping_sub(8));
    if r != 1 || g.read::<u32>(w(0x13)) != 0 || g.read::<u32>(p.wrapping_add(w(0x1c))) != r {
        return Ret::eax(r);
    }
    let h = g.read::<u32>(w(0x29));
    let s = slot_target(g, w(0x23), slot_at(img, f + 0x2f));
    Ret::eax(g.call(s, fr.ecx, &[h]).eax)
}

/// No arguments (`magic_0606` 0x100069e0, 54 bytes): the objects run
/// ([`run_objects`], `+0x08`); when none ran and `*G + D` is 1 (`G` at
/// `+0x03`, `D` at `+0x13`), the parts are registered ([`register_parts`],
/// `+0x1a`) and slot 214 ends the effect (`(*H)`, `H` at `+0x25`, the
/// slot at `+0x2b`). Returns its `eax`, else `run_objects`' result.
pub fn end_register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x03));
    let r = run_objects(g, img, img.rel32_target(f + 0x08), fr.esp.wrapping_sub(8));
    if r != 1 || g.read::<u32>(p.wrapping_add(w(0x13))) != r {
        return Ret::eax(r);
    }
    register_parts(g, img, img.rel32_target(f + 0x1a), r);
    let h = g.read::<u32>(w(0x25));
    let s = slot_target(g, w(0x1f), slot_at(img, f + 0x2b));
    Ret::eax(g.call(s, fr.ecx, &[h]).eax)
}

/// `(obj)` (`magic_0100` 0x10001680, 56 bytes): the objects run
/// ([`run_objects`], `+0x04`); when none ran, the counter `C` (`+0x0f`)
/// is not above the `imm8` at `+0x13` and the flag word `F` (`+0x18`)
/// has the bit at `+0x1c`, the effect ends ([`finish`], `+0x32`).
pub fn done_count_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // push ebp; call
    let r = run_objects(g, img, img.rel32_target(f + 0x04), fr.esp.wrapping_sub(8));
    if r != 1
        || g.read::<u32>(w(0x0f)).cast_signed() > disp8(img, f + 0x13).cast_signed()
        || g.read::<u32>(w(0x18)) & w(0x1c) == 0
    {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    Ret::eax(finish(g, obj, w(0x32)))
}

/// `(obj)` (`magic_0246` 0x100017d0, 56 bytes; 2 copies): the motion
/// steps ([`motion_step`], `+0x05`) and the objects run ([`run_objects`],
/// `+0x0c`); when both are through (1) and the word `G` (`+0x1b`) is 1,
/// the effect ends ([`finish`], `+0x31`).
pub fn done_motion_word<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = motion_step(g, img, img.rel32_target(f + 0x05));
    let r = run_objects(g, img, img.rel32_target(f + 0x0c), fr.esp.wrapping_sub(0xc));
    if r != 1 || a != r || g.read::<u32>(w(0x1b)) != r {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    Ret::eax(finish(g, obj, w(0x31)))
}

/// `(obj)` (`magic_0444` 0x10003000, 57 bytes): the motion and the
/// objects through (as [`done_motion_word`]) and the counter `G`
/// (`+0x1b`) not above 0: the effect ends ([`finish`], `+0x32`).
pub fn done_motion_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = motion_step(g, img, img.rel32_target(f + 0x05));
    let r = run_objects(g, img, img.rel32_target(f + 0x0c), fr.esp.wrapping_sub(0xc));
    if r != 1 || a != r || g.read::<u32>(w(0x1b)).cast_signed() > 0 {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    Ret::eax(finish(g, obj, w(0x32)))
}

/// `(obj)` (`magic_0275` 0x10003a00, 62 bytes): the motion and the
/// objects through (as [`done_motion_word`]) and `*G + D` equal to the
/// `imm8` at `+0x24` (`G` at `+0x1a`, `D` at `+0x20`): the effect ends
/// ([`finish`], `+0x37`). Returns the task; `run_objects`' result when
/// the motion or the objects are not through; `*G` when only the field
/// differs.
pub fn done_motion_field<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = motion_step(g, img, img.rel32_target(f + 0x05));
    let r = run_objects(g, img, img.rel32_target(f + 0x0c), fr.esp.wrapping_sub(0xc));
    if r != 1 || a != r {
        return Ret::eax(r);
    }
    // mov eax, ds:G: the pointer is what a mismatch returns
    let p = g.read::<u32>(w(0x1a));
    if g.read::<u32>(p.wrapping_add(w(0x20))) != disp8(img, f + 0x24) {
        return Ret::eax(p);
    }
    let obj = fr.arg(g, 0);
    Ret::eax(finish(g, obj, w(0x37)))
}

/// `(obj)` (`magic_0243` 0x10001840, 55 bytes): the objects run
/// ([`run_objects`], `+0x05`) and the motion steps ([`motion_step`],
/// `+0x0c`); when both are through (1) the effect ends ([`finish`],
/// `+0x2a`) and the grid is drawn ([`draw_grid`], a tail jump, `+0x30`).
/// Returns the motion step's result otherwise.
pub fn done_then_grid<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; push esi; call
    let a = run_objects(g, img, img.rel32_target(f + 0x05), fr.esp.wrapping_sub(0xc));
    let r = motion_step(g, img, img.rel32_target(f + 0x0c));
    if r != 1 || a != r {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    finish(g, obj, img.word(f + 0x2a));
    Ret::eax(draw_grid(g, img, img.rel32_target(f + 0x30)))
}

/// `(node)` (`magic_0432` 0x10001310, 56 bytes): the script runs
/// ([`script_run`], `+0x0c`) and the motion steps ([`motion_step`],
/// `+0x13`); when neither is 0, `*G + D` = the immediate at `+0x28` (`G`
/// at `+0x07`, `D` at `+0x24`) and the node goes back to the pool
/// ([`give_back`], a tail jump, `+0x30`). Returns the motion step's result
/// otherwise.
pub fn script_motion_release<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x07));
    let a = script_run(g, img, img.rel32_target(f + 0x0c));
    let m = motion_step(g, img, img.rel32_target(f + 0x13));
    if a == 0 || m == 0 {
        return Ret::eax(m);
    }
    let node = fr.arg(g, 0);
    g.write::<u32>(p.wrapping_add(w(0x24)), w(0x28));
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x30), node))
}

/// `(obj)` (`magic_0165` 0x10002ed0, 53 bytes; 2 copies): slot 237
/// `(*G, K, 0, V)` with `V` = `obj + E` (`G` at `+0x22`, `K` the `push
/// imm32` at `+0x1c`, `E` at `+0x14`, the slot at `+0x0e`), then the
/// float at `V + 0xc` set to the immediate at `+0x2e` (1.0). Returns the
/// slot's `eax`.
pub fn slot237_scale32<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0e));
    let v = obj.wrapping_add(w(0x14));
    let h = g.read::<u32>(w(0x22));
    let r = g.call(s, fr.ecx, &[h, w(0x1c), 0, v]);
    g.write::<u32>(v.wrapping_add(0xc), w(0x2e));
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0526` 0x10001100, 53 bytes): `rcefMainCreateTask(*G,
/// cb)` (the slot at `+0x15`, `G` at `+0x0f`, `cb` at `+0x09`), then the
/// slot at `+0x2a` with `(obj, cb2)` (`+0x21`; the first call's words
/// after them). Returns its `eax`.
pub fn task_then_cb<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = g.read::<u32>(w(0x0f));
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x15));
    g.call(s, fr.ecx, &[v, w(0x09)]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x1c), slot_at(img, f + 0x2a));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x21), v, w(0x09)]).eax)
}

/// `(obj)` (`magic_0301` 0x10001cc0, 62 bytes): two
/// `rcefMainCreateTask(*G, cb)` (the slots at `+0x15`/`+0x2d`, `G` at
/// `+0x0f`/`+0x27`, `cb` at `+0x09`/`+0x21`; the second with the first's
/// words after its own), then `obj` gets bit 2. Returns `obj`.
pub fn two_tasks_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = g.read::<u32>(w(0x0f));
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x15));
    g.call(s, fr.ecx, &[v, w(0x09)]);
    let v2 = g.read::<u32>(w(0x27));
    let s = slot_target(g, w(0x1c), slot_at(img, f + 0x2d));
    g.call(s, fr.ecx, &[v2, w(0x21), v, w(0x09)]);
    let obj = fr.arg(g, 0);
    let x = g.read::<u32>(obj);
    g.write::<u32>(obj, x | 4);
    Ret::eax(obj)
}

/// `magic_0307` 0x10005230 (53 bytes): the halfword `F` (`+0x08`) loses
/// bit 0; four words set (`mov dword [addr], imm32` from `+0x0c`, 10
/// bytes each). Returns 0xfffe (the mask left in `eax`).
pub fn clear_bit_four<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let a = img.word(f + 0x08);
    let v = g.read::<u16>(a);
    g.write::<u16>(a, v & 0xfffe);
    stores(g, img, f + 0x0c, 4);
    Ret::eax(0xfffe)
}

/// `n` `mov dword [addr], imm32` (10 bytes each) from `at`.
fn stores(g: &mut impl crate::Mem, img: &Image, at: u32, n: u32) {
    for k in 0..n {
        let insn = at + 10 * k;
        g.write::<u32>(img.word(insn + 2), img.word(insn + 6));
    }
}

/// `magic_0303` 0x10001660 (61 bytes; 2 copies): six words set (`mov
/// dword [addr], imm32`, 10 bytes each). `eax` as given.
pub fn six_stores<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    stores(g, img, fr.at, 6);
    Ret::eax(fr.eax)
}

/// `magic_0253` 0x100018a0 (59 bytes): the record `BASE + I * N` (`I`
/// the word at `+0x01`, `N` the `imul` immediate at `+0x07`, `BASE` at
/// `+0x0c`) gets six words (`mov dword [eax + d8], imm32` from `+0x10`, 7
/// bytes each). Returns the record.
pub fn record_six<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = g
        .read::<u32>(w(0x01))
        .wrapping_mul(w(0x07))
        .wrapping_add(w(0x0c));
    for k in 0..6 {
        let insn = f + 0x10 + 7 * k;
        g.write::<u32>(rec.wrapping_add(disp8(img, insn + 2)), img.word(insn + 3));
    }
    Ret::eax(rec)
}

/// `(obj)` (`magic_0307` 0x10004da0, 60 bytes): once the frame count
/// `obj + 0x118` is above the `imm8` at `+0x0c` (0x50), `obj + 0x104` =
/// 0; then, every call, the scale `obj + 0x60` plus the constant at
/// `+0x22` into `+0x60`..`+0x6c`. Returns `obj`.
pub fn grow_after<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let count = g.read::<u32>(obj.wrapping_add(w(0x08))).cast_signed();
    if count > disp8(img, f + 0x0c).cast_signed() {
        g.write::<u32>(obj.wrapping_add(w(0x11)), w(0x15));
    }
    let x = add(
        f32_at(g, obj.wrapping_add(disp8(img, f + 0x1d))),
        f32_at(g, w(0x22)),
    );
    for at in [0x2a, 0x2f, 0x34, 0x39] {
        g.write::<u32>(obj.wrapping_add(disp8(img, f + at)), x.to_bits());
    }
    Ret::eax(obj)
}

/// No arguments (`magic_0304` 0x10002ef0, 62 bytes): on `P` = `*G`:
/// `P[A] -= K1` (`+0x09`/`+0x19`, `K1` at `+0x11`), then `y = P[B] - K2`
/// (`+0x21`, `+0x29`) stored at `P[B]` and `P[C]` (`+0x31`, `+0x39`).
/// Returns `P`.
pub fn sub_two<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let at = |o: u32| p.wrapping_add(w(o));
    let x = sub(f32_at(g, at(0x09)), f32_at(g, w(0x11)));
    g.write::<u32>(at(0x19), x.to_bits());
    let y = sub(f32_at(g, at(0x21)), f32_at(g, w(0x29)));
    g.write::<u32>(at(0x31), y.to_bits());
    g.write::<u32>(at(0x39), y.to_bits());
    Ret::eax(p)
}

/// No arguments (`magic_0304` 0x10003500, 62 bytes): on `P` = `*G`: `a
/// = P[A] + K`, `b = P[B] + K` (`+0x09`, `+0x11`; `K` at `+0x19` and
/// `+0x21`), stored at `+0x29` (`a`), `+0x31` (`b`), `+0x39` (`a`).
/// Returns `P`.
pub fn add_two<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let at = |o: u32| p.wrapping_add(w(o));
    let (a0, b0) = (f32_at(g, at(0x09)), f32_at(g, at(0x11)));
    let a = add(a0, f32_at(g, w(0x19)));
    let b = add(b0, f32_at(g, w(0x21)));
    g.write::<u32>(at(0x29), a.to_bits());
    g.write::<u32>(at(0x31), b.to_bits());
    g.write::<u32>(at(0x39), a.to_bits());
    Ret::eax(p)
}

/// No arguments (`magic_0304` 0x100034c0, 63 bytes): on `P` = `*G`
/// (`+0x02`): `x = P[A] - K` (`+0x0a`, `+0x12`) into `P[B]` and `P[C]`
/// (`+0x20`, `+0x28`); `P[E] = P[D]` (`+0x18`, `+0x2e`) and `P[G] = P[F]`
/// (`+0x34`, `+0x3a`; `P[F]` read after the stores). Returns the last word.
pub fn sub_copy<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x02));
    let at = |o: u32| p.wrapping_add(w(o));
    let x = sub(f32_at(g, at(0x0a)), f32_at(g, w(0x12)));
    let d = g.read::<u32>(at(0x18));
    g.write::<u32>(at(0x20), x.to_bits());
    g.write::<u32>(at(0x28), x.to_bits());
    g.write::<u32>(at(0x2e), d);
    let v = g.read::<u32>(at(0x34));
    g.write::<u32>(at(0x3a), v);
    Ret::eax(v)
}

/// No arguments (`magic_0430` 0x100028c0, 61 bytes; 2 copies): `r` = the
/// slot at `+0x08` (312) with `(cb, 0, 0, 0)` (`cb` at `+0x19`); two
/// words of `*G` set (`G` at `+0x0e`; `mov dword [esi + d], imm32` at
/// `+0x22`, `+0x2c`); `R = r` (`+0x37`). Returns `r`.
pub fn slot_keep_two<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x01), slot_at(img, f + 0x08));
    let p = g.read::<u32>(w(0x0e));
    let r = g.call(s, fr.ecx, &[w(0x19), 0, 0, 0]).eax;
    for at in [0x22, 0x2c] {
        g.write::<u32>(p.wrapping_add(w(at + 2)), w(at + 6));
    }
    g.write::<u32>(w(0x37), r);
    Ret::eax(r)
}

/// `(obj)` (`magic_0416` 0x10001530, 61 bytes): the slot at `+0x0c`
/// with the `push imm8` at `+0x09`; the slot at `+0x1b` with the
/// immediates at `+0x20`, `+0x18`; the slot at `+0x32` with `(obj, cb)`
/// (`cb` at `+0x29`), each with the earlier words after its own. Returns
/// its `eax`.
pub fn three_slots<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| disp8(img, f + off);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0c));
    g.call(s, fr.ecx, &[d(0x09)]);
    let s = slot_target(g, w(0x13), slot_at(img, f + 0x1b));
    g.call(s, fr.ecx, &[d(0x20), d(0x18), d(0x09)]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x24), slot_at(img, f + 0x32));
    let r = g.call(s, fr.ecx, &[obj, w(0x29), d(0x20), d(0x18), d(0x09)]);
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0360` 0x100018f0, 61 bytes): the counter `C` (`+0x05`)
/// down by one; while it stays at or above 0, `rcefMainCreateTask(*G,
/// cb)` (the slot at `+0x12`, `G` at `+0x1d`, `cb` at `+0x17`); once
/// below, `rcefTaskSetSeq(obj, cb2)` (the slot at `+0x2a`, `cb2` at
/// `+0x2f`). Returns the slot's `eax`.
pub fn count_then_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let c = g.read::<u32>(w(0x05)).wrapping_sub(1);
    g.write::<u32>(w(0x05), c);
    if c.cast_signed() < 0 {
        let s = slot_target(g, w(0x0a), slot_at(img, f + 0x2a));
        let obj = fr.arg(g, 0);
        return Ret::eax(g.call(s, fr.ecx, &[obj, w(0x2f)]).eax);
    }
    let s = slot_target(g, w(0x0a), slot_at(img, f + 0x12));
    let v = g.read::<u32>(w(0x1d));
    Ret::eax(g.call(s, fr.ecx, &[v, w(0x17)]).eax)
}

/// `(rec)` (`magic_0537` 0x10001260, 57 bytes): `*G_P = ITEM` (`+0x04`,
/// `+0x0a`), then slot 713 (`+0x29`) frees the part of the record's
/// index: `PARTS + ((rec - RECS) >> 2) * 0x80` (`RECS` at `+0x12`, `PARTS`
/// at `+0x1d`). Returns 1.
pub fn free_part_of<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    g.write::<u32>(p, w(0x0a));
    let rec = fr.arg(g, 0);
    // sub; sar 2; shl 7; add
    let i = rec.wrapping_sub(w(0x12)).cast_signed() >> 2;
    let part = i.cast_unsigned().wrapping_shl(7).wrapping_add(w(0x1d));
    let s = slot_target(g, w(0x23), slot_at(img, f + 0x29));
    g.call(s, fr.ecx, &[part]);
    Ret::eax(1)
}

/// `(parts)` (`magic_0641` 0x10001970, 57 bytes): for each of `N` (the
/// immediate at `+0x09`) part buffers of 0x80 bytes from `parts`: `*G_P
/// = ITEM` (`+0x11`, `+0x18`) and slot 713 (`+0x23`) frees it. Returns 1.
pub fn free_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut part = fr.arg(g, 0);
    for _ in 0..w(0x09) {
        let p = g.read::<u32>(w(0x11));
        g.write::<u32>(p, w(0x18));
        let s = slot_target(g, w(0x1d), slot_at(img, f + 0x23));
        g.call(s, fr.ecx, &[part]);
        part = part.wrapping_add(0x80);
    }
    Ret::eax(1)
}

/// No arguments (`magic_0253` 0x10002bf0, 62 bytes, internal: a float
/// in `xmm0`): two float slots of the actor `*G` (`+0x0e`): `a` = slot
/// 364 (`+0x14`) and `b` = slot 16 (`+0x26`, the first call's word after
/// its own), each `fstp dword`; returns `b * a` in `xmm0` (`eax` the
/// second slot's).
pub fn float_product<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let actor = g.read::<u32>(w(0x0e));
    let s = slot_target(g, w(0x07), slot_at(img, f + 0x14));
    let a = g.call(s, fr.ecx, &[actor]).st0.unwrap_or(0.0) as f32;
    let s = slot_target(g, w(0x1f), disp8(img, f + 0x26) / 4);
    let r = g.call(s, fr.ecx, &[actor, actor]);
    let b = r.st0.unwrap_or(0.0) as f32;
    Ret::xmm0(Xmm::f32(mul(b, a)), r.eax)
}

/// [`done_field`] in `magic_0473` (0x100017c0, 51 bytes).
pub const DONE_FIELD_0473_17C0: Sig = Sig {
    len: 51,
    hash: 0xece8_74cc_2e0e_3756,
    calls: &[(0xb, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`done_field_register`] in `magic_0446` (0x10004ba0, 58 bytes).
pub const DONE_FIELD_REGISTER_0446_4BA0: Sig = Sig {
    len: 58,
    hash: 0xedb5_830d_2783_8d9d,
    calls: &[
        (0xb, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x33, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`end_when_idle`] in `magic_0391` (0x10001380, 58 bytes).
pub const END_WHEN_IDLE_0391_1380: Sig = Sig {
    len: 58,
    hash: 0xad25_0557_55c7_56c6,
    calls: &[(0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`end_when_idle`] in `magic_0448` (0x10004c80, 58 bytes).
pub const END_WHEN_IDLE_0448_4C80: Sig = Sig {
    len: 58,
    hash: 0x518d_cb6f_36a1_a0b6,
    calls: &[(0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`end_register`] in `magic_0606` (0x100069e0, 54 bytes).
pub const END_REGISTER_0606_69E0: Sig = Sig {
    len: 54,
    hash: 0xa72a_7104_d9d6_d246,
    calls: &[
        (0x8, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x1a, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`done_count_flag`] in `magic_0100` (0x10001680, 56 bytes).
pub const DONE_COUNT_FLAG_0100_1680: Sig = Sig {
    len: 56,
    hash: 0xd5cf_1a36_cb9d_d232,
    calls: &[(0x4, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`done_motion_word`] in `magic_0246` (0x100017d0, 56 bytes) and
/// `magic_0276` (0x100032b0, the same code).
pub const DONE_MOTION_WORD_0246_17D0: Sig = Sig {
    len: 56,
    hash: 0x2ec6_93be_41b1_44ca,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};

/// [`done_motion_count`] in `magic_0444` (0x10003000, 57 bytes).
pub const DONE_MOTION_COUNT_0444_3000: Sig = Sig {
    len: 57,
    hash: 0xfd14_9868_796d_0654,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};

/// [`done_motion_field`] in `magic_0275` (0x10003a00, 62 bytes).
pub const DONE_MOTION_FIELD_0275_3A00: Sig = Sig {
    len: 62,
    hash: 0x7fbb_afe2_e67c_81e9,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};

/// [`done_then_grid`] in `magic_0243` (0x10001840, 55 bytes).
pub const DONE_THEN_GRID_0243_1840: Sig = Sig {
    len: 55,
    hash: 0x9c2e_268a_a3d3_5b79,
    calls: &[
        (0x5, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0xc, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x30, Callee::Sig(&crate::sb_misc::DRAW_GRID)),
    ],
    imports: &[],
};

/// [`script_motion_release`] in `magic_0432` (0x10001310, 56 bytes).
pub const SCRIPT_MOTION_RELEASE_0432_1310: Sig = Sig {
    len: 56,
    hash: 0xb279_f665_59fa_e9ca,
    calls: &[
        (0xc, Callee::Sig(&crate::sa_script::RUN)),
        (0x13, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x30, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`slot237_scale32`] in `magic_0165` (0x10002ed0, 53 bytes).
pub const SLOT237_SCALE32_0165_2ED0: Sig = Sig {
    len: 53,
    hash: 0x3a77_0619_0c3c_f93d,
    calls: &[],
    imports: &[],
};

/// [`slot237_scale32`] in `magic_0590` (0x10001640, 53 bytes).
pub const SLOT237_SCALE32_0590_1640: Sig = Sig {
    len: 53,
    hash: 0x1b46_0028_ec11_a60e,
    calls: &[],
    imports: &[],
};

/// [`task_then_cb`] in `magic_0526` (0x10001100, 53 bytes).
pub const TASK_THEN_CB_0526_1100: Sig = Sig {
    len: 53,
    hash: 0x5ce1_ae7d_81e4_f6df,
    calls: &[],
    imports: &[],
};

/// [`two_tasks_flag`] in `magic_0301` (0x10001cc0, 62 bytes).
pub const TWO_TASKS_FLAG_0301_1CC0: Sig = Sig {
    len: 62,
    hash: 0x579c_1f6e_8594_5b33,
    calls: &[],
    imports: &[],
};

/// [`clear_bit_four`] in `magic_0307` (0x10005230, 53 bytes).
pub const CLEAR_BIT_FOUR_0307_5230: Sig = Sig {
    len: 53,
    hash: 0x6d54_b254_e4cd_56c9,
    calls: &[],
    imports: &[],
};

/// [`six_stores`] in `magic_0303` (0x10001660, 61 bytes).
pub const SIX_STORES_0303_1660: Sig = Sig {
    len: 61,
    hash: 0x8a02_f119_409d_26c8,
    calls: &[],
    imports: &[],
};

/// [`six_stores`] in `magic_0303` (0x100016a0, 61 bytes).
pub const SIX_STORES_0303_16A0: Sig = Sig {
    len: 61,
    hash: 0x0a61_3a25_0bf9_9f41,
    calls: &[],
    imports: &[],
};

/// [`record_six`] in `magic_0253` (0x100018a0, 59 bytes).
pub const RECORD_SIX_0253_18A0: Sig = Sig {
    len: 59,
    hash: 0x0e3d_420a_34c3_4f5d,
    calls: &[],
    imports: &[],
};

/// [`grow_after`] in `magic_0307` (0x10004da0, 60 bytes).
pub const GROW_AFTER_0307_4DA0: Sig = Sig {
    len: 60,
    hash: 0x5aab_a03a_0504_542e,
    calls: &[],
    imports: &[],
};

/// [`sub_two`] in `magic_0304` (0x10002ef0, 62 bytes).
pub const SUB_TWO_0304_2EF0: Sig = Sig {
    len: 62,
    hash: 0xc347_c1d3_8a45_9434,
    calls: &[],
    imports: &[],
};

/// [`add_two`] in `magic_0304` (0x10003500, 62 bytes).
pub const ADD_TWO_0304_3500: Sig = Sig {
    len: 62,
    hash: 0x4081_5637_6441_e258,
    calls: &[],
    imports: &[],
};

/// [`sub_copy`] in `magic_0304` (0x100034c0, 63 bytes).
pub const SUB_COPY_0304_34C0: Sig = Sig {
    len: 63,
    hash: 0x15dd_c601_6286_2d5e,
    calls: &[],
    imports: &[],
};

/// [`slot_keep_two`] in `magic_0430` (0x100028c0, 61 bytes).
pub const SLOT_KEEP_TWO_0430_28C0: Sig = Sig {
    len: 61,
    hash: 0x6db7_0abf_5d9a_0505,
    calls: &[],
    imports: &[],
};

/// [`slot_keep_two`] in `magic_0588` (0x10002090, 61 bytes).
pub const SLOT_KEEP_TWO_0588_2090: Sig = Sig {
    len: 61,
    hash: 0x59c5_cc12_8ee0_fe15,
    calls: &[],
    imports: &[],
};

/// [`three_slots`] in `magic_0416` (0x10001530, 61 bytes).
pub const THREE_SLOTS_0416_1530: Sig = Sig {
    len: 61,
    hash: 0xe7ec_3754_c104_d46a,
    calls: &[],
    imports: &[],
};

/// [`count_then_task`] in `magic_0360` (0x100018f0, 61 bytes).
pub const COUNT_THEN_TASK_0360_18F0: Sig = Sig {
    len: 61,
    hash: 0x5319_074a_68a0_9dff,
    calls: &[],
    imports: &[],
};

/// [`free_part_of`] in `magic_0537` (0x10001260, 57 bytes).
pub const FREE_PART_OF_0537_1260: Sig = Sig {
    len: 57,
    hash: 0xe6f3_9be8_3f79_2942,
    calls: &[],
    imports: &[],
};

/// [`free_parts`] in `magic_0641` (0x10001970, 57 bytes).
pub const FREE_PARTS_0641_1970: Sig = Sig {
    len: 57,
    hash: 0xeba1_cb16_6862_87fe,
    calls: &[],
    imports: &[],
};

/// [`float_product`] in `magic_0253` (0x10002bf0, 62 bytes).
pub const FLOAT_PRODUCT_0253_2BF0: Sig = Sig {
    len: 62,
    hash: 0x7188_d794_8440_b6a5,
    calls: &[],
    imports: &[],
};
