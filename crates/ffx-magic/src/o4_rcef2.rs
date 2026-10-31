//! rcef chains of the own work list, one DLL each (round 4): a few slot
//! calls with the earlier calls' words left on the stack, then the
//! task's sequence and wait. Operands are read from the code (`w(off)` =
//! the word at `+off`; a slot is its `disp32` / 4).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The slot of a `mov eax, [eax + disp32]` with the `disp32` at `at`.
fn slot_at(img: &Image, at: u32) -> u32 {
    img.word(at) / 4
}

/// Calls the slot of the table global at `table` and the `disp32` at
/// `disp` with `words` in front of the stack `stack` (which keeps them,
/// as the original's pushes stay until its `add esp`). Returns its `eax`.
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
    // call width: 16 at most (the earlier calls' words stay on the stack)
    g.call(s, fr.ecx, stack).eax
}

/// `(obj)` (`magic_0191` 0x10001000, 84 bytes): `C` = 0 (`+0x0a`/`+0x0e`),
/// slot 454 (`+0x1f`) with `(V)` (`V` the word at `*G + D`, `G` at `+0x04`,
/// `D` at `+0x14`), `rcefTaskSetSeq(obj, cb)` (`+0x34`, `+0x2b`) and
/// `rcefTaskCallSeq(obj, cb2)` (`+0x49`, `+0x40`). Returns the last
/// slot's `eax`.
pub fn clear_slot_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x04));
    g.write::<u32>(w(0x0a), w(0x0e));
    let v = g.read::<u32>(p.wrapping_add(w(0x14)));
    let obj = fr.arg(g, 0);
    let mut st = Vec::new();
    push_call(g, img, fr, &mut st, (0x19, 0x1f), &[v]);
    push_call(g, img, fr, &mut st, (0x26, 0x34), &[obj, w(0x2b)]);
    Ret::eax(push_call(
        g,
        img,
        fr,
        &mut st,
        (0x3b, 0x49),
        &[obj, w(0x40)],
    ))
}

/// `(obj)` (`magic_0191` 0x100010b0, 88 bytes): slot 460 (`+0x26`) with
/// `(P[D], P[E + 4 * i], 1, 2)` (`P` = `*G`, `G` at `+0x05`; `i` = `*I`,
/// `I` at `+0x0a`; `D`, `E` at `+0x20`, `+0x15`; the immediates at
/// `+0x11`, `+0x0f`), `rcefTaskSetSeq(obj, cb)` (`+0x3b`, `+0x32`) and
/// `rcefTaskWait(obj, T)` (`+0x4d`, `T` at `+0x47`). Returns the last
/// slot's `eax`.
pub fn indexed_slot_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x05));
    let i = g.read::<u32>(w(0x0a));
    let b = g.read::<u32>(p.wrapping_add(w(0x15)).wrapping_add(i.wrapping_mul(4)));
    let a = g.read::<u32>(p.wrapping_add(w(0x20)));
    let obj = fr.arg(g, 0);
    let mut st = Vec::new();
    let first = [a, b, disp8(img, f + 0x11), disp8(img, f + 0x0f)];
    push_call(g, img, fr, &mut st, (0x1a, 0x26), &first);
    push_call(g, img, fr, &mut st, (0x2d, 0x3b), &[obj, w(0x32)]);
    let t = disp8(img, f + 0x47);
    Ret::eax(push_call(g, img, fr, &mut st, (0x42, 0x4d), &[obj, t]))
}

/// `(obj)` (`magic_0300` 0x10001170, 95 bytes): two
/// `rcefMainCreateTask(*G, cb)` (`+0x15`/`+0x2d`, `G` at `+0x0f`/`+0x27`,
/// `cb` at `+0x09`/`+0x21`), `rcefTaskSetSeq(obj, cb3)` (`+0x42`,
/// `+0x39`) and `rcefTaskWait(obj, T)` (`+0x54`, `T` at `+0x4e`). Returns
/// the last slot's `eax`.
pub fn two_tasks_seq_wait8<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut st = Vec::new();
    let a = g.read::<u32>(w(0x0f));
    push_call(g, img, fr, &mut st, (0x04, 0x15), &[a, w(0x09)]);
    let b = g.read::<u32>(w(0x27));
    push_call(g, img, fr, &mut st, (0x1c, 0x2d), &[b, w(0x21)]);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x34, 0x42), &[obj, w(0x39)]);
    let t = disp8(img, f + 0x4e);
    Ret::eax(push_call(g, img, fr, &mut st, (0x49, 0x54), &[obj, t]))
}

/// `(obj)` (`magic_0300` 0x100011d0, 93 bytes): `V` = the word at `*G +
/// D` (`G` at `+0x04`, `D` at `+0x0b`); slot 444 (`+0x18`) with `(K)`
/// (`+0x15`), slot 446 (`+0x27`) with `(V, 0)` (`+0x24`),
/// `rcefTaskSetSeq(obj, cb)` (`+0x38`, `+0x3d`) and
/// `rcefTaskCallSeq(obj, cb2)` (`+0x50`, `+0x4a`). Returns the last
/// slot's `eax`.
pub fn slots_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0b)));
    let mut st = Vec::new();
    push_call(g, img, fr, &mut st, (0x10, 0x18), &[disp8(img, f + 0x15)]);
    push_call(
        g,
        img,
        fr,
        &mut st,
        (0x1f, 0x27),
        &[v, disp8(img, f + 0x24)],
    );
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x2f, 0x38), &[obj, w(0x3d)]);
    Ret::eax(push_call(
        g,
        img,
        fr,
        &mut st,
        (0x45, 0x50),
        &[obj, w(0x4a)],
    ))
}

/// `(obj)` (`magic_0301` 0x10001740, 81 bytes): slot 460 (`+0x1f`) with
/// `(P[0x198], P[0x1a0], 2, 2)` (`P` = `*G`, `G` at `+0x04`; the fields at
/// `+0x14`, `+0x0e`, the immediates at `+0x0b`, `+0x09`),
/// `rcefTaskSetSeq(obj, cb)` (`+0x34`, `+0x2b`) and `rcefTaskWait(obj, T)`
/// (`+0x46`, `T` at `+0x40`). Returns the last slot's `eax`.
pub fn pair_slot_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let b = g.read::<u32>(p.wrapping_add(w(0x0e)));
    let a = g.read::<u32>(p.wrapping_add(w(0x14)));
    let obj = fr.arg(g, 0);
    let mut st = Vec::new();
    let first = [a, b, disp8(img, f + 0x0b), disp8(img, f + 0x09)];
    push_call(g, img, fr, &mut st, (0x19, 0x1f), &first);
    push_call(g, img, fr, &mut st, (0x26, 0x34), &[obj, w(0x2b)]);
    let t = disp8(img, f + 0x40);
    Ret::eax(push_call(g, img, fr, &mut st, (0x3b, 0x46), &[obj, t]))
}

/// `(obj)` (`magic_0301` 0x100017a0, 100 bytes): slot 447 (`+0x1a`) with
/// `(P[D], K)` (`P` = `*G`, `G` at `+0x04`, `D` at `+0x0f`, `K` the `push
/// imm32` at `+0x09`), `rcefMainCreateTask(*G, cb)` (`+0x32`, `G` at
/// `+0x2c`, `cb` at `+0x26`), `rcefTaskSetSeq(obj, cb2)` (`+0x47`,
/// `+0x3e`) and `rcefTaskWait(obj, T)` (`+0x59`, `T` at `+0x53`). Returns
/// the last slot's `eax`.
pub fn field_task_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0f)));
    let mut st = Vec::new();
    push_call(g, img, fr, &mut st, (0x14, 0x1a), &[v, w(0x09)]);
    let a = g.read::<u32>(w(0x2c));
    push_call(g, img, fr, &mut st, (0x21, 0x32), &[a, w(0x26)]);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x39, 0x47), &[obj, w(0x3e)]);
    let t = disp8(img, f + 0x53);
    Ret::eax(push_call(g, img, fr, &mut st, (0x4e, 0x59), &[obj, t]))
}

/// `(obj)` (`magic_0359` 0x10001160, 95 bytes): `P` = `*G` (`+0x04`),
/// `V` = `P[D]` (`D` at `+0x0c`); `rcefMainCreateTask(P, cb)` (`+0x1d`,
/// `cb` at `+0x11`), `rcefTaskSetSeq(obj, cb2)` (`+0x2d`, `+0x32`) and
/// `rcefTaskCallSeq(obj, cb3)` (`+0x45`, `+0x3f`); then `V` in the
/// object's list `obj + 0x2c + 16 * i` (`i` the word at `obj + 0xac`).
/// Returns `2 * i`.
pub fn task_seq_keep<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let mut st = Vec::new();
    push_call(g, img, fr, &mut st, (0x17, 0x1d), &[p, w(0x11)]);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x24, 0x2d), &[obj, w(0x32)]);
    push_call(g, img, fr, &mut st, (0x3a, 0x45), &[obj, w(0x3f)]);
    // add eax, eax; mov [esi + eax * 8 + 0x2c], edi
    let i = g.read::<u32>(obj.wrapping_add(0xac));
    let twice = i.wrapping_add(i);
    g.write::<u32>(
        obj.wrapping_add(twice.wrapping_mul(8)).wrapping_add(0x2c),
        v,
    );
    Ret::eax(twice)
}

/// `(obj)` (`magic_0359` 0x100014d0, 92 bytes): while the counter `C`
/// (`+0x05`) is above 0, two `rcefMainCreateTask(*G, cb)` (`+0x13`,
/// `+0x36`; `G` at `+0x1e`/`+0x30`, `cb` at `+0x18`/`+0x2a`) and `C`
/// down by one (`+0x41`); else `rcefTaskSetSeq(obj, cb3)` (`+0x49`,
/// `+0x4e`). Returns the last slot's `eax`.
pub fn tasks_while_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut st = Vec::new();
    if g.read::<u32>(w(0x05)).cast_signed() > 0 {
        let a = g.read::<u32>(w(0x1e));
        push_call(g, img, fr, &mut st, (0x0b, 0x13), &[a, w(0x18)]);
        let b = g.read::<u32>(w(0x30));
        let r = push_call(g, img, fr, &mut st, (0x25, 0x36), &[b, w(0x2a)]);
        let c = g.read::<u32>(w(0x41));
        g.write::<u32>(w(0x41), c.wrapping_sub(1));
        return Ret::eax(r);
    }
    let obj = fr.arg(g, 0);
    Ret::eax(push_call(
        g,
        img,
        fr,
        &mut st,
        (0x0b, 0x49),
        &[obj, w(0x4e)],
    ))
}

/// `(obj)` (`magic_0361` 0x10001260, 100 bytes): `R` =
/// `rcefMainCreateTask(*G, cb)` (`+0x15`, `G` at `+0x0f`, `cb` at
/// `+0x09`; `R` at `+0x1c`), a second create (`+0x32`, `+0x2c`, `+0x26`),
/// `rcefTaskSetSeq(obj, cb3)` (`+0x47`, `+0x3e`) and `rcefTaskWait(obj,
/// T)` (`+0x59`, `T` at `+0x53`). Returns the last slot's `eax`.
pub fn keep_task_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut st = Vec::new();
    let a = g.read::<u32>(w(0x0f));
    let r = push_call(g, img, fr, &mut st, (0x04, 0x15), &[a, w(0x09)]);
    g.write::<u32>(w(0x1c), r);
    let b = g.read::<u32>(w(0x2c));
    push_call(g, img, fr, &mut st, (0x21, 0x32), &[b, w(0x26)]);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x39, 0x47), &[obj, w(0x3e)]);
    let t = disp8(img, f + 0x53);
    Ret::eax(push_call(g, img, fr, &mut st, (0x4e, 0x59), &[obj, t]))
}

/// `(obj)` (`magic_0361` 0x100012d0, 91 bytes): `V` = `P[D]` (`P` = `*G`,
/// `G` at `+0x04`, `D` at `+0x0c`); slot 445 (`+0x19`) with `(K)`
/// (`+0x16`), `rcefTaskSetSeq(obj, cb)` (`+0x29`, `+0x2e`) and
/// `rcefTaskCallSeq(obj, cb2)` (`+0x41`, `+0x3b`); then `V` in the
/// object's list `obj + 0x2c + 16 * i` (`i` the word at `obj + 0xac`).
/// Returns `2 * i`.
pub fn slot_seq_keep<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let mut st = Vec::new();
    push_call(g, img, fr, &mut st, (0x11, 0x19), &[disp8(img, f + 0x16)]);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x20, 0x29), &[obj, w(0x2e)]);
    push_call(g, img, fr, &mut st, (0x36, 0x41), &[obj, w(0x3b)]);
    // add eax, eax; mov [esi + eax * 8 + 0x2c], edi
    let i = g.read::<u32>(obj.wrapping_add(0xac));
    let twice = i.wrapping_add(i);
    g.write::<u32>(
        obj.wrapping_add(twice.wrapping_mul(8)).wrapping_add(0x2c),
        v,
    );
    Ret::eax(twice)
}

/// `(obj)` (`magic_0526` 0x10001090, 102 bytes): `V` = `P[D]` (`P` =
/// `*G`, `G` at `+0x04`, `D` at `+0x0b`); slot 460 (`+0x21`) with `(V,
/// P[E], 2, 4)` (`E` at `+0x15`, the immediates at `+0x12`, `+0x10`),
/// slot 447 (`+0x33`) with `(V, K)` (`K` at `+0x2d`), `rcefTaskSetSeq(obj,
/// cb)` (`+0x44`, `+0x49`) and `rcefTaskWait(obj, T)` (`+0x59`, `T` at
/// `+0x56`). Returns the last slot's `eax`.
pub fn pair_field_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0b)));
    let e = g.read::<u32>(p.wrapping_add(w(0x15)));
    let mut st = Vec::new();
    let first = [v, e, disp8(img, f + 0x12), disp8(img, f + 0x10)];
    push_call(g, img, fr, &mut st, (0x1a, 0x21), &first);
    push_call(g, img, fr, &mut st, (0x28, 0x33), &[v, w(0x2d)]);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x3b, 0x44), &[obj, w(0x49)]);
    let t = disp8(img, f + 0x56);
    Ret::eax(push_call(g, img, fr, &mut st, (0x51, 0x59), &[obj, t]))
}

/// `(obj)` (`magic_0526` 0x100011c0, 109 bytes): `P` = `*G` (`G` at
/// `+0x05`), `i` = `*I` (`+0x0a`), `V` = `P[D]` (`D` at `+0x11`); slot 460
/// (`+0x28`) with `(V, P[E + 4i], 4, 8)` (`E` at `+0x1c`, the immediates
/// at `+0x18`, `+0x16`), slot 447 (`+0x3a`) with `(V, K)` (`K` at
/// `+0x34`), `rcefTaskSetSeq(obj, cb)` (`+0x4b`, `+0x50`) and
/// `rcefTaskWait(obj, T)` (`+0x60`, `T` at `+0x5d`). Returns the last
/// slot's `eax`.
pub fn indexed_field_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x05));
    let i = g.read::<u32>(w(0x0a));
    let v = g.read::<u32>(p.wrapping_add(w(0x11)));
    let e = g.read::<u32>(p.wrapping_add(w(0x1c)).wrapping_add(i.wrapping_mul(4)));
    let mut st = Vec::new();
    let first = [v, e, disp8(img, f + 0x18), disp8(img, f + 0x16)];
    push_call(g, img, fr, &mut st, (0x21, 0x28), &first);
    push_call(g, img, fr, &mut st, (0x2f, 0x3a), &[v, w(0x34)]);
    let obj = fr.arg(g, 0);
    push_call(g, img, fr, &mut st, (0x42, 0x4b), &[obj, w(0x50)]);
    let t = disp8(img, f + 0x5d);
    Ret::eax(push_call(g, img, fr, &mut st, (0x58, 0x60), &[obj, t]))
}

/// `(obj)` (`magic_0526` 0x10001270, 109 bytes): `r` =
/// `rcefMainCreateTask(*G, cb)` (`+0x15`, `G` at `+0x0f`, `cb` at
/// `+0x09`); the counter `C` (`+0x23`) into `r`'s list `r + 0x2c + 16 *
/// r[0xac]`; `C + 1` stored back (`+0x43`); while it is below `P[0x19c]`
/// (`P` = `*G`, `+0x31`) the slot at `+0x49` (`+0x734`) with `(obj, cb2)`
/// (`+0x50`), else with `(obj, cb3)` (`+0x5f`). Returns its `eax`.
pub fn task_count_cb<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x0f));
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x15));
    let r = g.call(s, fr.ecx, &[a, w(0x09)]).eax;
    let k = g.read::<u32>(r.wrapping_add(0xac));
    let c = g.read::<u32>(w(0x23));
    g.write::<u32>(
        r.wrapping_add(k.wrapping_add(k).wrapping_mul(8))
            .wrapping_add(0x2c),
        c,
    );
    let p = g.read::<u32>(w(0x31));
    let n = c.wrapping_add(1);
    let more = n.cast_signed() < g.read::<u32>(p.wrapping_add(0x19c)).cast_signed();
    g.write::<u32>(w(0x43), n);
    let s = slot_target(g, w(0x3d), slot_at(img, f + 0x49));
    let obj = fr.arg(g, 0);
    let cb = if more { w(0x50) } else { w(0x5f) };
    Ret::eax(g.call(s, fr.ecx, &[obj, cb]).eax)
}

/// `(obj)` (`magic_0529` 0x10001650, 109 bytes): `R` = slot 468
/// (`+0x0a`); slot 465 (`+0x27`) with `(R, *A, 1, 1)` (`A` at `+0x16`,
/// `R` kept at `+0x1b`); slot 473 (`+0x36`) with `(0, 0, 0, 0x40, 0xf)`;
/// slot 527 (`+0x4d`) with `(1)`; `rcefTaskSetSeq(obj, cb)` (`+0x62`,
/// `+0x59`), each with the earlier words after its own. Returns its
/// `eax`.
pub fn effect_slots_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| disp8(img, f + off);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0a));
    let r = g.call(s, fr.ecx, &[]).eax;
    let a = g.read::<u32>(w(0x16));
    g.write::<u32>(w(0x1b), r);
    let mut st = Vec::new();
    push_call(g, img, fr, &mut st, (0x21, 0x27), &[r, a, d(0x13), d(0x11)]);
    let five = [d(0x41), d(0x3f), d(0x3d), d(0x3b), d(0x33)];
    push_call(g, img, fr, &mut st, (0x2e, 0x36), &five);
    push_call(g, img, fr, &mut st, (0x45, 0x4d), &[d(0x4a)]);
    let obj = fr.arg(g, 0);
    Ret::eax(push_call(
        g,
        img,
        fr,
        &mut st,
        (0x54, 0x62),
        &[obj, w(0x59)],
    ))
}

/// [`clear_slot_seq_call`] in `magic_0191` (0x10001000, 84 bytes).
pub const CLEAR_SLOT_SEQ_CALL_0191_1000: Sig = Sig {
    len: 84,
    hash: 0x3f54_5e38_d202_0191,
    calls: &[],
    imports: &[],
};

/// [`indexed_slot_seq_wait`] in `magic_0191` (0x100010b0, 88 bytes).
pub const INDEXED_SLOT_SEQ_WAIT_0191_10B0: Sig = Sig {
    len: 88,
    hash: 0xa9ef_e78c_4928_b481,
    calls: &[],
    imports: &[],
};

/// [`two_tasks_seq_wait8`] in `magic_0300` (0x10001170, 95 bytes).
pub const TWO_TASKS_SEQ_WAIT8_0300_1170: Sig = Sig {
    len: 95,
    hash: 0x685b_61d2_891c_959e,
    calls: &[],
    imports: &[],
};

/// [`slots_seq_call`] in `magic_0300` (0x100011d0, 93 bytes).
pub const SLOTS_SEQ_CALL_0300_11D0: Sig = Sig {
    len: 93,
    hash: 0x10d4_55e6_cc1c_16dc,
    calls: &[],
    imports: &[],
};

/// [`pair_slot_seq_wait`] in `magic_0301` (0x10001740, 81 bytes).
pub const PAIR_SLOT_SEQ_WAIT_0301_1740: Sig = Sig {
    len: 81,
    hash: 0x8c8b_9d95_7930_d503,
    calls: &[],
    imports: &[],
};

/// [`field_task_seq_wait`] in `magic_0301` (0x100017a0, 100 bytes).
pub const FIELD_TASK_SEQ_WAIT_0301_17A0: Sig = Sig {
    len: 100,
    hash: 0xe21a_5101_b5f6_307c,
    calls: &[],
    imports: &[],
};

/// [`task_seq_keep`] in `magic_0359` (0x10001160, 95 bytes).
pub const TASK_SEQ_KEEP_0359_1160: Sig = Sig {
    len: 95,
    hash: 0xbe1c_7035_8543_774b,
    calls: &[],
    imports: &[],
};

/// [`tasks_while_count`] in `magic_0359` (0x100014d0, 92 bytes).
pub const TASKS_WHILE_COUNT_0359_14D0: Sig = Sig {
    len: 92,
    hash: 0x06bb_86d7_6bca_9ad6,
    calls: &[],
    imports: &[],
};

/// [`keep_task_seq_wait`] in `magic_0361` (0x10001260, 100 bytes).
pub const KEEP_TASK_SEQ_WAIT_0361_1260: Sig = Sig {
    len: 100,
    hash: 0x2107_d2f2_ee0c_860f,
    calls: &[],
    imports: &[],
};

/// [`slot_seq_keep`] in `magic_0361` (0x100012d0, 91 bytes).
pub const SLOT_SEQ_KEEP_0361_12D0: Sig = Sig {
    len: 91,
    hash: 0x141c_e25f_f3ff_084f,
    calls: &[],
    imports: &[],
};

/// [`pair_field_seq_wait`] in `magic_0526` (0x10001090, 102 bytes).
pub const PAIR_FIELD_SEQ_WAIT_0526_1090: Sig = Sig {
    len: 102,
    hash: 0x88cc_ac00_d653_9727,
    calls: &[],
    imports: &[],
};

/// [`indexed_field_seq_wait`] in `magic_0526` (0x100011c0, 109 bytes).
pub const INDEXED_FIELD_SEQ_WAIT_0526_11C0: Sig = Sig {
    len: 109,
    hash: 0xc00a_86d3_3a61_2f73,
    calls: &[],
    imports: &[],
};

/// [`task_count_cb`] in `magic_0526` (0x10001270, 109 bytes).
pub const TASK_COUNT_CB_0526_1270: Sig = Sig {
    len: 109,
    hash: 0x40a2_7fe2_f6b4_ed95,
    calls: &[],
    imports: &[],
};

/// [`effect_slots_seq`] in `magic_0529` (0x10001650, 109 bytes).
pub const EFFECT_SLOTS_SEQ_0529_1650: Sig = Sig {
    len: 109,
    hash: 0x4463_b598_0eae_28b8,
    calls: &[],
    imports: &[],
};
