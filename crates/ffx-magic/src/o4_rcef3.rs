//! More rcef chains of the own work list, one DLL each (round 4).
//! Operands are read from the code (`w(off)` = the word at `+off`; a slot
//! is its `disp32` / 4).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

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

/// `(obj)` (`magic_0361` 0x10002770, 91 bytes): `V` = `P[D]` (`P` =
/// `*G`, `G` at `+0x04`, `D` at `+0x0b`); slot 506 (`+0x1b`) with `(V,
/// R)` (`R` at `+0x15`), slot 483 (`+0x2a`) with `(V)`, slot 447 (`+0x39`)
/// with `(V, K)` (`K` at `+0x36`) and `rcefTaskSetSeq(obj, cb)` (`+0x4f`,
/// `+0x46`), each with the earlier words after its own. Returns its `eax`.
pub fn field_slots_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0b)));
    let mut st = Vec::new();
    push_call(g, img, fr, &mut st, (0x10, 0x1b), &[v, w(0x15)]);
    push_call(g, img, fr, &mut st, (0x23, 0x2a), &[v]);
    push_call(
        g,
        img,
        fr,
        &mut st,
        (0x31, 0x39),
        &[v, disp8(img, f + 0x36)],
    );
    let obj = fr.arg(g, 0);
    Ret::eax(push_call(
        g,
        img,
        fr,
        &mut st,
        (0x41, 0x4f),
        &[obj, w(0x46)],
    ))
}

/// `(obj)` (`magic_0382` 0x10001790, 91 bytes): the counter `C`
/// (`+0x05`) down by one; while it stays at or above 0,
/// `rcefMainCreateTask(*G, cb)` (`+0x32`, `G` at `+0x2c`, `cb` at
/// `+0x26`) six times (the count at `+0x0d`); once below,
/// `rcefTaskSetSeq(obj, cb2)` (`+0x50`, `+0x47`). Returns the last slot's
/// `eax`.
pub fn tasks_or_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let c = g.read::<u32>(w(0x05)).wrapping_sub(1);
    g.write::<u32>(w(0x05), c);
    if c.cast_signed() < 0 {
        let obj = fr.arg(g, 0);
        let s = slot_target(g, w(0x42), w(0x50) / 4);
        return Ret::eax(g.call(s, fr.ecx, &[obj, w(0x47)]).eax);
    }
    let mut eax = 0;
    for _ in 0..w(0x0d) {
        let a = g.read::<u32>(w(0x2c));
        let s = slot_target(g, w(0x21), w(0x32) / 4);
        eax = g.call(s, fr.ecx, &[a, w(0x26)]).eax;
    }
    Ret::eax(eax)
}

/// `(obj)` (`magic_0382` 0x10001c70, 103 bytes): `v` = the object's list
/// word `obj + 0x2c + 16 * i` (`i` the word at `obj + 0xac`) plus one;
/// below `*N` (`+0x15`) the object takes the source `*S`'s (`+0x21`)
/// `+0xd0` and `+0xd8` (the fields at `+0x29`, `+0x31` into `+0x39`,
/// `+0x4b`) and two immediates (`+0x3f`/`+0x43`, `+0x51`/`+0x55`) and `v`
/// goes back into the list; else `obj` gets bit 2. Returns `2 * i`.
pub fn step_or_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let slot = |g: &G| {
        let i = g.read::<u32>(obj.wrapping_add(0xac));
        i.wrapping_add(i)
    };
    let twice = slot(g);
    let v = g
        .read::<u32>(obj.wrapping_add(twice.wrapping_mul(8)).wrapping_add(0x2c))
        .wrapping_add(1);
    if v.cast_signed() >= g.read::<u32>(w(0x15)).cast_signed() {
        let x = g.read::<u32>(obj);
        g.write::<u32>(obj, x | 4);
        return Ret::eax(twice);
    }
    let s = g.read::<u32>(w(0x21));
    let a = g.read::<u32>(s.wrapping_add(w(0x29)));
    let b = g.read::<u32>(s.wrapping_add(w(0x31)));
    g.write::<u32>(obj.wrapping_add(w(0x39)), a);
    g.write::<u32>(obj.wrapping_add(w(0x3f)), w(0x43));
    g.write::<u32>(obj.wrapping_add(w(0x4b)), b);
    g.write::<u32>(obj.wrapping_add(w(0x51)), w(0x55));
    let twice = slot(g);
    g.write::<u32>(
        obj.wrapping_add(twice.wrapping_mul(8)).wrapping_add(0x2c),
        v,
    );
    Ret::eax(twice)
}

/// `(obj)` (`magic_0313` 0x100019e0, 92 bytes): `r` =
/// `rcefMainCreateTask(*G, cb)` (`+0x15`, `G` at `+0x0f`, `cb` at
/// `+0x09`); `*H` (`+0x23`) into `r`'s list `r + 0x2c + 16 * r[0xac]`;
/// `rcefTaskSetSeq(obj, cb2)` (`+0x3c`, `+0x2a`) and
/// `rcefTaskCallSeq(obj, cb3)` (`+0x51`, `+0x48`), each with the earlier
/// words after its own. Returns the last slot's `eax`.
pub fn task_keep_seq_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x0f));
    let mut st = Vec::new();
    let r = push_call(g, img, fr, &mut st, (0x04, 0x15), &[a, w(0x09)]);
    let i = g.read::<u32>(r.wrapping_add(0xac));
    let h = g.read::<u32>(w(0x23));
    let obj = fr.arg(g, 0);
    g.write::<u32>(
        r.wrapping_add(i.wrapping_add(i).wrapping_mul(8))
            .wrapping_add(0x2c),
        h,
    );
    push_call(g, img, fr, &mut st, (0x36, 0x3c), &[obj, w(0x2a)]);
    Ret::eax(push_call(
        g,
        img,
        fr,
        &mut st,
        (0x43, 0x51),
        &[obj, w(0x48)],
    ))
}

/// [`field_slots_seq`] in `magic_0361` (0x10002770, 91 bytes).
pub const FIELD_SLOTS_SEQ_0361_2770: Sig = Sig {
    len: 91,
    hash: 0x538c_0f35_b8cd_99c4,
    calls: &[],
    imports: &[],
};

/// [`tasks_or_seq`] in `magic_0382` (0x10001790, 91 bytes).
pub const TASKS_OR_SEQ_0382_1790: Sig = Sig {
    len: 91,
    hash: 0xe54b_1d4c_c0be_9308,
    calls: &[],
    imports: &[],
};

/// [`step_or_flag`] in `magic_0382` (0x10001c70, 103 bytes).
pub const STEP_OR_FLAG_0382_1C70: Sig = Sig {
    len: 103,
    hash: 0x9ca3_54ab_4f6a_f92b,
    calls: &[],
    imports: &[],
};

/// [`task_keep_seq_call`] in `magic_0313` (0x100019e0, 92 bytes).
pub const TASK_KEEP_SEQ_CALL_0313_19E0: Sig = Sig {
    len: 92,
    hash: 0xe252_8164_7757_a8c8,
    calls: &[],
    imports: &[],
};
