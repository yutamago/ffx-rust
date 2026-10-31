//! A fourth set of the own work list's small functions (round 4): slot
//! call chains, static set-ups, the task callbacks' end checks.
//! Operands are read from each copy's code (`w(off)` = the word at
//! `+off`).

use crate::l5_objhost::run_objects;
use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::s3_motion::motion_step;
use crate::sa_misc::register_parts;
use crate::sa_script::run as script_run;
use crate::sb_misc::draw_grid;
use crate::sb_sse::add;
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

/// `(i)` (`magic_0351` 0x10002730, 63 bytes): for an index up to `N - 1`
/// (`N` the word at `+0x04`) whose table word (`TABLE` at `+0x13`) is
/// set, the slot at `+0x2f` with `(B[i], 5, 1)` (`B` the byte table
/// `*G`, `G` at `+0x1b`; the immediates at `+0x26`, `+0x20`). Returns 1.
pub fn index_slot<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(0x04)).wrapping_sub(1);
    let i = fr.arg(g, 0);
    if i.cast_signed() > last.cast_signed() || g.read::<u32>(w(0x13).wrapping_add(4 * i)) == 0 {
        return Ret::eax(1);
    }
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x1b)).wrapping_add(i)));
    let s = slot_target(g, w(0x29), slot_at(img, f + 0x2f));
    g.call(s, fr.ecx, &[b, disp8(img, f + 0x26), disp8(img, f + 0x20)]);
    Ret::eax(1)
}

/// No arguments (`magic_0708` 0x10001830, 63 bytes): the slot at `+0x0d`
/// (804) for each of three words (`*A`, `*B`, `*C` at `+0x07`, `+0x1a`,
/// `+0x2d`; the slots read at `+0x20`, `+0x33`), each call with the
/// earlier words after its own. Returns 0.
pub fn slot_three_words<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x07));
    let s = slot_target(g, w(0x01), slot_at(img, f + 0x0d));
    g.call(s, fr.ecx, &[a]);
    let b = g.read::<u32>(w(0x1a));
    let s = slot_target(g, w(0x14), slot_at(img, f + 0x20));
    g.call(s, fr.ecx, &[b, a]);
    let c = g.read::<u32>(w(0x2d));
    let s = slot_target(g, w(0x27), slot_at(img, f + 0x33));
    g.call(s, fr.ecx, &[c, b, a]);
    Ret::eax(0)
}

/// No arguments (`magic_0197` 0x10001500, 64 bytes): while the word `A`
/// (`+0x02`) is 0 and the count `N` (`+0x0a`) is below the `imm8` at
/// `+0x10`, the slot at `+0x1d` with `(N, 1)`; else the slot at `+0x36`
/// with `(*H, 0)` (`H` at `+0x30`). Returns the slot's `eax`.
pub fn slot_by_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x02)) == 0 {
        let n = g.read::<u32>(w(0x0a));
        if n.cast_signed() < disp8(img, f + 0x10).cast_signed() {
            let s = slot_target(g, w(0x17), slot_at(img, f + 0x1d));
            return Ret::eax(g.call(s, fr.ecx, &[n, disp8(img, f + 0x14)]).eax);
        }
    }
    let h = g.read::<u32>(w(0x30));
    let s = slot_target(g, w(0x28), slot_at(img, f + 0x36));
    Ret::eax(g.call(s, fr.ecx, &[h, disp8(img, f + 0x2d)]).eax)
}

/// `magic_0273` 0x10007690 (64 bytes, internal: a float in `xmm2`): a
/// static record set: the byte `A` gets bits 3 (`+0x02`, `+0x06`), the
/// byte `B` = the `imm8` at `+0x12` (`+0x0e`), `C` = the float (`+0x17`),
/// the words `D`, `F`, `G` their immediates (`+0x1d`/`+0x21`,
/// `+0x2d`/`+0x31`, `+0x37`/`+0x3b`), the halfword `E` (`+0x27`) the
/// immediate at `+0x08`. Returns that immediate.
pub fn record_float<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = w(0x02);
    let v = g.read::<u8>(a);
    g.write::<u8>(a, v | (w(0x06) as u8));
    let e = w(0x08);
    g.write::<u8>(w(0x0e), w(0x12) as u8);
    g.write::<u32>(w(0x17), fr.xmm[2] as u32);
    g.write::<u32>(w(0x1d), w(0x21));
    g.write::<u16>(w(0x27), e as u16);
    g.write::<u32>(w(0x2d), w(0x31));
    g.write::<u32>(w(0x37), w(0x3b));
    Ret::eax(e)
}

/// No arguments (`magic_0497` 0x10001150, 64 bytes): the slots at
/// `+0x0f` and `+0x24` each with `(*G, 0)` (`G` at `+0x09`/`+0x1e`; the
/// second with the first's words after its own), then the slot at
/// `+0x34` with none. Returns 1.
pub fn slots_g_g_none<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x09));
    let s = slot_target(g, w(0x01), slot_at(img, f + 0x0f));
    g.call(s, fr.ecx, &[a, disp8(img, f + 0x06)]);
    let b = g.read::<u32>(w(0x1e));
    let s = slot_target(g, w(0x16), slot_at(img, f + 0x24));
    g.call(
        s,
        fr.ecx,
        &[b, disp8(img, f + 0x1b), a, disp8(img, f + 0x06)],
    );
    let s = slot_target(g, w(0x2b), slot_at(img, f + 0x34));
    g.call(s, fr.ecx, &[]);
    Ret::eax(1)
}

/// `this` = `ecx` (`magic_0514` 0x10003180, 64 bytes): `M` = the word at
/// `+d2` of the record at `this + d1` (`d1` at `+0x02`, `d2` at `+0x06`);
/// the slots at `+0x10` and `+0x22` with `(M, -1, -1)`, the slot at
/// `+0x34` with `(M, 0)`, each with the earlier words after its own.
/// Returns the last slot's `eax`.
pub fn three_slots_m<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| disp8(img, f + off);
    let rec = g.read::<u32>(fr.ecx.wrapping_add(d(0x02)));
    let m = g.read::<u32>(rec.wrapping_add(d(0x06)));
    let s = slot_target(g, w(0x08), slot_at(img, f + 0x10));
    g.call(s, fr.ecx, &[m, d(0x15), d(0x0d)]);
    let s = slot_target(g, w(0x1a), slot_at(img, f + 0x22));
    g.call(s, fr.ecx, &[m, d(0x27), d(0x1f), m, d(0x15), d(0x0d)]);
    let s = slot_target(g, w(0x2c), slot_at(img, f + 0x34));
    let r = g.call(
        s,
        fr.ecx,
        &[m, d(0x31), m, d(0x27), d(0x1f), m, d(0x15), d(0x0d)],
    );
    Ret::eax(r.eax)
}

/// No arguments (`magic_0303` 0x100016e0, 65 bytes): `x = A + K` (`A`
/// at `+0x04`, `K` at `+0x0c`); four words their immediates (`mov dword
/// [addr], imm32` from `+0x10`, 10 bytes each); `A' = x` (`+0x3c`). `eax`
/// as given.
pub fn add_and_stores<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let x = add(
        f32::from_bits(g.read::<u32>(w(0x04))),
        f32::from_bits(g.read::<u32>(w(0x0c))),
    );
    for k in 0..4 {
        let insn = f + 0x10 + 10 * k;
        g.write::<u32>(img.word(insn + 2), img.word(insn + 6));
    }
    g.write::<u32>(w(0x3c), x.to_bits());
    Ret::eax(fr.eax)
}

/// `(obj)` (`magic_0361` 0x100017a0, 65 bytes): the slot at `+0x0e`
/// with `(obj)`, `rcefTaskSetSeq(obj, cb)` (`+0x21`, `cb` at `+0x1b`),
/// `rcefTaskCallSeq(obj, cb2)` (`+0x34`, `+0x2e`), each with the earlier
/// words after its own. Returns the last slot's `eax`.
pub fn slot_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0e));
    g.call(s, fr.ecx, &[obj]);
    let s = slot_target(g, w(0x16), slot_at(img, f + 0x21));
    g.call(s, fr.ecx, &[obj, w(0x1b), obj]);
    let s = slot_target(g, w(0x29), slot_at(img, f + 0x34));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x2e), obj, w(0x1b), obj]).eax)
}

/// `(obj)` (`magic_0191` 0x100012f0, 66 bytes): `V` = the word at `*G +
/// D` (`G` at `+0x04`, `D` at `+0x0a`); the slots at `+0x15` and `+0x22`
/// with `(V)` (the word stays for the second), then `rcefTaskSetSeq(obj,
/// cb)` (`+0x37`, `cb` at `+0x2e`). Returns its `eax`.
pub fn field_slots_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0a)));
    let s = slot_target(g, w(0x0f), slot_at(img, f + 0x15));
    g.call(s, fr.ecx, &[v]);
    let s = slot_target(g, w(0x1c), slot_at(img, f + 0x22));
    g.call(s, fr.ecx, &[v]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x29), slot_at(img, f + 0x37));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x2e), v]).eax)
}

/// `(obj)` (`magic_0348` 0x10006df0, 66 bytes): slot 237 (`+0x0b`) with
/// `(*G, obj[A], 0, V)` (`G` at `+0x24`, `A` at `+0x1e`, `V` = `obj + E`,
/// `E` at `+0x15`); the float `V + 0xc` = the immediate at `+0x30`;
/// `obj + B` = the immediate at `+0x3a` (`B` at `+0x36`). Returns the
/// slot's `eax`.
pub fn slot237_obj<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0b));
    let obj = fr.arg(g, 0);
    let v = obj.wrapping_add(w(0x15));
    let a = g.read::<u32>(obj.wrapping_add(w(0x1e)));
    let h = g.read::<u32>(w(0x24));
    let r = g.call(s, fr.ecx, &[h, a, 0, v]);
    g.write::<u32>(v.wrapping_add(0xc), w(0x30));
    g.write::<u32>(obj.wrapping_add(w(0x36)), w(0x3a));
    Ret::eax(r.eax)
}

/// `(obj)` (`magic_0205` 0x100015c0, 67 bytes): the objects run
/// ([`run_objects`], `+0x0b`); when none ran, `*G + D` is 1 (`G` at
/// `+0x06`, `D` at `+0x16`) and `*G + E` not above the `imm8` at `+0x22`
/// (`E` at `+0x1e`), the effect ends (`obj + 0x14` = 3, the task's
/// callback the one at `+0x36`) and the parts are registered
/// ([`register_parts`], a tail jump, `+0x3c`). Returns `run_objects`'
/// result otherwise.
pub fn done_two_fields_register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x06));
    let r = run_objects(g, img, img.rel32_target(f + 0x0b), fr.esp.wrapping_sub(0xc));
    if r != 1
        || g.read::<u32>(p.wrapping_add(w(0x16))) != r
        || g.read::<u32>(p.wrapping_add(w(0x1e))).cast_signed() > disp8(img, f + 0x22).cast_signed()
    {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    let task = finish(g, obj, w(0x36));
    Ret::eax(register_parts(g, img, img.rel32_target(f + 0x3c), task))
}

/// `this` = `ecx` (`magic_0146` 0x10007170, 66 bytes): a task node set
/// up and put at the head of the list `G_HEAD` (`+0x17`/`+0x26`/`+0x36`):
/// its vtable and callback (`+0x08`, `+0x0f`), `+0x14` itself, `+0x8` the
/// old head, `+0x4` the list anchor (`+0x21`), the old head's `+0x4`
/// it, the list flag word (`+0x2f`) without bit 0, `+0x18` 0. Returns the
/// old head.
pub fn push_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = fr.ecx;
    g.write::<u32>(n.wrapping_add(8), n);
    g.write::<u32>(n.wrapping_add(4), n);
    g.write::<u32>(n, w(0x08));
    g.write::<u32>(n.wrapping_add(0x10), w(0x0f));
    g.write::<u32>(n.wrapping_add(0x14), n);
    let head = g.read::<u32>(w(0x17));
    g.write::<u32>(n.wrapping_add(8), head);
    g.write::<u32>(n.wrapping_add(4), w(0x21));
    let head = g.read::<u32>(w(0x26));
    g.write::<u32>(head.wrapping_add(4), n);
    let flags = g.read::<u32>(w(0x2f));
    g.write::<u32>(w(0x2f), flags & disp8(img, f + 0x33));
    g.write::<u32>(w(0x36), n);
    g.write::<u32>(n.wrapping_add(0x18), 0);
    Ret::eax(head)
}

/// `(obj)` (`magic_0359` 0x10001530, 68 bytes): while the counter `C`
/// (`+0x05`) is above 0, `rcefMainCreateTask(*G, cb)` (the slot at
/// `+0x13`, `G` at `+0x1e`, `cb` at `+0x18`) and the counter down by one
/// (`+0x29`); else `rcefTaskSetSeq(obj, cb2)` (`+0x31`, `+0x36`). Returns
/// the slot's `eax`.
pub fn task_while_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x05)).cast_signed() > 0 {
        let v = g.read::<u32>(w(0x1e));
        let s = slot_target(g, w(0x0b), slot_at(img, f + 0x13));
        let r = g.call(s, fr.ecx, &[v, w(0x18)]).eax;
        let c = g.read::<u32>(w(0x29));
        g.write::<u32>(w(0x29), c.wrapping_sub(1));
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x0b), slot_at(img, f + 0x31));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x36)]).eax)
}

/// `(obj)` (`magic_0528` 0x100010d0, 68 bytes): the slot at `+0x0c` with
/// the `push imm8` at `+0x09`, `rcefMainCreateTask(*G, cb)` (`+0x24`,
/// `G` at `+0x1e`, `cb` at `+0x18`), then the slot at `+0x39` with `(obj,
/// cb2)` (`+0x30`), each with the earlier words after its own. Returns
/// its `eax`.
pub fn slot_task_cb<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = disp8(img, f + 0x09);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0c));
    g.call(s, fr.ecx, &[k]);
    let v = g.read::<u32>(w(0x1e));
    let s = slot_target(g, w(0x13), slot_at(img, f + 0x24));
    g.call(s, fr.ecx, &[v, w(0x18), k]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x2b), slot_at(img, f + 0x39));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x30), v, w(0x18), k]).eax)
}

/// `(obj)` (`magic_0270` 0x10001410, 69 bytes): the motion steps
/// ([`motion_step`], `+0x06`), the script runs ([`script_run`], `+0x0d`)
/// and the objects run ([`run_objects`], `+0x14`); when all three are
/// through (1) the effect ends (the callback at `+0x37`) and the grid is
/// drawn ([`draw_grid`], a tail jump, `+0x3d`). Returns `run_objects`'
/// result otherwise.
pub fn done_all_grid<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let m = motion_step(g, img, img.rel32_target(f + 0x06));
    let s = script_run(g, img, img.rel32_target(f + 0x0d));
    // push ebp; push esi; push edi; call
    let r = run_objects(
        g,
        img,
        img.rel32_target(f + 0x14),
        fr.esp.wrapping_sub(0x10),
    );
    if r != 1 || m != r || s != r {
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    finish(g, obj, img.word(f + 0x37));
    Ret::eax(draw_grid(g, img, img.rel32_target(f + 0x3d)))
}

/// `(obj)` (`magic_0359` 0x10001000, 70 bytes; 2 copies):
/// `rcefTaskSetSeq(obj, cb)` (the slot at `+0x12`, `cb` at `+0x09`) with
/// the words `A`, `B` set first (`+0x18`/`+0x1c`, `+0x22`/`+0x26`), then
/// `rcefTaskCallSeq(obj, cb2)` (`+0x3b`, `+0x32`; the first call's words
/// after them). Returns its `eax`.
pub fn set_two_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x12));
    g.write::<u32>(w(0x18), w(0x1c));
    g.write::<u32>(w(0x22), w(0x26));
    g.call(s, fr.ecx, &[obj, w(0x09)]);
    let s = slot_target(g, w(0x2d), slot_at(img, f + 0x3b));
    Ret::eax(g.call(s, fr.ecx, &[obj, w(0x32), obj, w(0x09)]).eax)
}

/// `(obj)` (`magic_0430` 0x10002450, 70 bytes): `r` = the slot at
/// `+0x16` (219) with `(B, 0)` (`B` the byte at `*G`, `G` at `+0x04`);
/// the slot at `+0x30` (268) with `(r, K, V)` (`K` the `push imm8` at
/// `+0x27`, `V` = `obj + E`, `E` at `+0x21`; the first call's words after
/// them); `obj + D` = the immediate at `+0x3f` (`D` at `+0x3b`). Returns
/// the second slot's `eax`.
pub fn byte_slot_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x04))));
    let s = slot_target(g, w(0x10), slot_at(img, f + 0x16));
    let r = g.call(s, fr.ecx, &[b, 0]).eax;
    let obj = fr.arg(g, 0);
    let v = obj.wrapping_add(w(0x21));
    let s = slot_target(g, w(0x2a), slot_at(img, f + 0x30));
    let e = g.call(s, fr.ecx, &[r, disp8(img, f + 0x27), v, b, 0]).eax;
    g.write::<u32>(obj.wrapping_add(w(0x3b)), w(0x3f));
    Ret::eax(e)
}

/// `this` = `ecx` (`magic_0388` 0x10006810, 69 bytes, internal: a float
/// in `xmm1`): the slot at `+0x1f` with `(this + C, this + B, this + A)`
/// (the `disp8` at `+0x16`, `+0x12`, `+0x0e`); then `this + 0x54` = 1,
/// the float `this + 0x50`, `this + 0x4c` = 0 (`+0x2f`/`+0x30`, `+0x38`,
/// `+0x3b`/`+0x3c`). Returns the slot's `eax`.
pub fn vectors_then_float<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| disp8(img, f + off);
    let this = fr.ecx;
    let s = slot_target(g, w(0x19), slot_at(img, f + 0x1f));
    let args = [
        this.wrapping_add(d(0x16)),
        this.wrapping_add(d(0x12)),
        this.wrapping_add(d(0x0e)),
    ];
    let r = g.call(s, fr.ecx, &args).eax;
    g.write::<u32>(this.wrapping_add(d(0x2f)), w(0x30));
    g.write::<u32>(this.wrapping_add(d(0x38)), fr.xmm[1] as u32);
    g.write::<u32>(this.wrapping_add(d(0x3b)), w(0x3c));
    Ret::eax(r)
}

/// [`index_slot`] in `magic_0351` (0x10002730, 63 bytes).
pub const INDEX_SLOT_0351_2730: Sig = Sig {
    len: 63,
    hash: 0x0ff2_3f1b_3b33_817f,
    calls: &[],
    imports: &[],
};

/// [`slot_three_words`] in `magic_0708` (0x10001830, 63 bytes).
pub const SLOT_THREE_WORDS_0708_1830: Sig = Sig {
    len: 63,
    hash: 0xce30_ac27_39c8_d425,
    calls: &[],
    imports: &[],
};

/// [`slot_by_count`] in `magic_0197` (0x10001500, 64 bytes).
pub const SLOT_BY_COUNT_0197_1500: Sig = Sig {
    len: 64,
    hash: 0x4362_1703_70e4_5540,
    calls: &[],
    imports: &[],
};

/// [`record_float`] in `magic_0273` (0x10007690, 64 bytes).
pub const RECORD_FLOAT_0273_7690: Sig = Sig {
    len: 64,
    hash: 0xfe88_0fb0_435b_cc20,
    calls: &[],
    imports: &[],
};

/// [`slots_g_g_none`] in `magic_0497` (0x10001150, 64 bytes).
pub const SLOTS_G_G_NONE_0497_1150: Sig = Sig {
    len: 64,
    hash: 0xe1d4_92fb_30c7_52eb,
    calls: &[],
    imports: &[],
};

/// [`three_slots_m`] in `magic_0514` (0x10003180, 64 bytes).
pub const THREE_SLOTS_M_0514_3180: Sig = Sig {
    len: 64,
    hash: 0xf97b_1d3f_5a74_5199,
    calls: &[],
    imports: &[],
};

/// [`add_and_stores`] in `magic_0303` (0x100016e0, 65 bytes).
pub const ADD_AND_STORES_0303_16E0: Sig = Sig {
    len: 65,
    hash: 0x3c06_2df5_b196_966b,
    calls: &[],
    imports: &[],
};

/// [`slot_seq_call`] in `magic_0361` (0x100017a0, 65 bytes).
pub const SLOT_SEQ_CALL_0361_17A0: Sig = Sig {
    len: 65,
    hash: 0xa631_6f00_ee0c_4a62,
    calls: &[],
    imports: &[],
};

/// [`field_slots_seq`] in `magic_0191` (0x100012f0, 66 bytes).
pub const FIELD_SLOTS_SEQ_0191_12F0: Sig = Sig {
    len: 66,
    hash: 0x29b6_5b38_7681_577e,
    calls: &[],
    imports: &[],
};

/// [`slot237_obj`] in `magic_0348` (0x10006df0, 66 bytes).
pub const SLOT237_OBJ_0348_6DF0: Sig = Sig {
    len: 66,
    hash: 0x8bd6_7b11_c603_73e8,
    calls: &[],
    imports: &[],
};

/// [`done_two_fields_register`] in `magic_0205` (0x100015c0, 67 bytes).
pub const DONE_TWO_FIELDS_REGISTER_0205_15C0: Sig = Sig {
    len: 67,
    hash: 0x7a64_2de1_4ae1_10bc,
    calls: &[
        (0xb, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x3c, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// [`push_task`] in `magic_0146` (0x10007170, 66 bytes).
pub const PUSH_TASK_0146_7170: Sig = Sig {
    len: 66,
    hash: 0x377f_6099_03f7_3437,
    calls: &[],
    imports: &[],
};

/// [`task_while_count`] in `magic_0359` (0x10001530, 68 bytes).
pub const TASK_WHILE_COUNT_0359_1530: Sig = Sig {
    len: 68,
    hash: 0xfe82_2b71_159d_8809,
    calls: &[],
    imports: &[],
};

/// [`slot_task_cb`] in `magic_0528` (0x100010d0, 68 bytes).
pub const SLOT_TASK_CB_0528_10D0: Sig = Sig {
    len: 68,
    hash: 0x8bda_d74c_2705_b302,
    calls: &[],
    imports: &[],
};

/// [`done_all_grid`] in `magic_0270` (0x10001410, 69 bytes).
pub const DONE_ALL_GRID_0270_1410: Sig = Sig {
    len: 69,
    hash: 0x8227_9e1c_7a09_88b3,
    calls: &[
        (0x6, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xd, Callee::Sig(&crate::sa_script::RUN)),
        (0x14, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x3d, Callee::Sig(&crate::sb_misc::DRAW_GRID)),
    ],
    imports: &[],
};

/// [`set_two_seq_call`] in `magic_0359` (0x10001000, 70 bytes).
pub const SET_TWO_SEQ_CALL_0359_1000: Sig = Sig {
    len: 70,
    hash: 0x7a1b_470c_8cd2_6ec0,
    calls: &[],
    imports: &[],
};

/// [`set_two_seq_call`] in `magic_0360` (0x10001000, 70 bytes).
pub const SET_TWO_SEQ_CALL_0360_1000: Sig = Sig {
    len: 70,
    hash: 0xbd69_7199_afa4_cb5c,
    calls: &[],
    imports: &[],
};

/// [`byte_slot_part`] in `magic_0430` (0x10002450, 70 bytes).
pub const BYTE_SLOT_PART_0430_2450: Sig = Sig {
    len: 70,
    hash: 0xf2e5_e54f_4001_aba5,
    calls: &[],
    imports: &[],
};

/// [`vectors_then_float`] in `magic_0388` (0x10006810, 69 bytes).
pub const VECTORS_THEN_FLOAT_0388_6810: Sig = Sig {
    len: 69,
    hash: 0x6397_e620_17c3_32a9,
    calls: &[],
    imports: &[],
};
