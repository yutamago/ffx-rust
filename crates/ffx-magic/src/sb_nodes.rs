//! More of the node family ([`crate::nodes`]: `magic_0003`'s and
//! `magic_0087`'s code, the same bytes in about 116 DLLs), ported by the
//! shared-B list. Addresses are `magic_0003`'s; each doc names the size
//! and how many DLLs carry the function. A cursor (`this`) holds the node
//! it visits at `+0`, a "moved" byte at `+4` and the pass number at `+5`;
//! the methods a node's vtable holds at `+0x1c` take a cursor.

use crate::l3_common::{ListAt, attach, node_init};
use crate::nodes::walk;
use crate::opu_parts::{START_PART, start_part};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A cursor on the caller's frame at `local` (`[ebp - 8]`, the pass
/// number at `[ebp - 3]`): visits `node` through its method `+0x1c`
/// with `ecx` = the cursor. Returns the method's `eax`.
fn visit_local<G: Game>(g: &mut G, local: u32, node: u32, pass: u8) -> u32 {
    g.write::<u32>(local, node);
    g.write::<u8>(local + 5, pass);
    let method = g.read::<u32>(g.read::<u32>(node) + 0x1c);
    g.call(method, local, &[local]).eax
}

/// The walk, then the owner's two nodes `a` (`+0x2c`) and `b` (`+0x30`)
/// visited with a cursor on the frame ([`visit_local`]; `esp` the
/// entry's, the cursor at `esp - 12`) and slot 229 (table global at
/// operand `+0x91`) called with `(owner + 0x1c, b + 0x9c, a + 0x1c)`,
/// `a` and `b` read again. Returns the slot's `eax` (the walk's when the
/// owner was visited already).
///
/// `0x10003f00` (167 bytes, thiscall `ecx` = the cursor, 116 DLLs).
pub fn visit_pair_229<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    this: u32,
    eax: u32,
) -> u32 {
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let local = esp.wrapping_sub(12);
    let pass = g.read::<u8>(this + 5);
    let a = g.read::<u32>(owner + 0x2c);
    visit_local(g, local, a, pass);
    let pass = g.read::<u8>(this + 5);
    let b = g.read::<u32>(owner + 0x30);
    visit_local(g, local, b, pass);
    let a = g.read::<u32>(owner + 0x2c);
    let b = g.read::<u32>(owner + 0x30);
    let s229 = slot_target(g, img.word(f + 0x91), 229);
    g.call(s229, local, &[owner + 0x1c, b + 0x9c, a + 0x1c]).eax
}

/// [`visit_pair_229`].
pub const VISIT_PAIR_229: Sig = Sig {
    len: 167,
    hash: 0x905e_e35d_2f3f_e22e,
    calls: &[],
    imports: &[],
};

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

fn set_f32<G: Game>(g: &mut G, a: u32, x: f32) {
    g.write::<u32>(a, x.to_bits());
}

/// The walk, then (`addss`, in the original's order) the owner's floats
/// `+0x1c`..`+0x24` added into its parent's (`+0x14`) and the owner's
/// `+0x2c` into its own `+0x20`. Returns the parent (the walk's `eax`
/// when the owner was visited already).
///
/// `0x100061b0` (141 bytes, thiscall `ecx` = the cursor, 116 DLLs).
pub fn visit_add_vector<G: Game>(g: &mut G, this: u32, eax: u32) -> u32 {
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let parent = g.read::<u32>(owner + 0x14);
    let x = f32_at(g, parent + 0x1c) + f32_at(g, owner + 0x1c);
    set_f32(g, parent + 0x1c, x);
    for off in [0x20, 0x24] {
        let x = f32_at(g, owner + off) + f32_at(g, parent + off);
        set_f32(g, parent + off, x);
    }
    let x = f32_at(g, owner + 0x2c) + f32_at(g, owner + 0x20);
    set_f32(g, owner + 0x20, x);
    parent
}

/// [`visit_add_vector`].
pub const VISIT_ADD_VECTOR: Sig = Sig {
    len: 141,
    hash: 0x5284_330e_b597_0893,
    calls: &[],
    imports: &[],
};

/// The walk, then the owner's node `c` (`+0x38`) walked with a cursor
/// on the frame (`esp - 12`, the pass at `esp - 7`) as the walk does
/// (when `c`'s pass byte `+0x1a` is not this pass: each child of `c`
/// visited, then `c` takes the pass byte as the frame holds it, which
/// the children may have changed), and the owner's words `+0x1c`..`+0x24`
/// set to its `+0x2c`..`+0x34` plus `c`'s `+0x1c`..`+0x24` (integers).
/// Returns the last sum (the walk's `eax` when the owner was visited
/// already).
///
/// `0x10003ab0` (166 bytes, thiscall `ecx` = the cursor, 116 DLLs).
pub fn visit_offset_int<G: Game>(g: &mut G, esp: u32, this: u32, eax: u32) -> u32 {
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let local = esp.wrapping_sub(12);
    let pass = g.read::<u8>(this + 5);
    let c = g.read::<u32>(owner + 0x38);
    g.write::<u32>(local, c);
    g.write::<u8>(local + 5, pass);
    if g.read::<u8>(c + 0x1a) != pass {
        let mut child = g.read::<u32>(c + 0x10);
        let mut pass = pass;
        if child != c {
            loop {
                g.write::<u32>(local, child);
                let method = g.read::<u32>(g.read::<u32>(child) + 0x1c);
                child = g.read::<u32>(child + 8);
                g.call(method, local, &[local]);
                if child == c {
                    break;
                }
            }
            pass = g.read::<u8>(local + 5);
        }
        g.write::<u8>(c + 0x1a, pass);
    }
    let mut sum = 0;
    for off in [0x1c, 0x20, 0x24] {
        sum = g
            .read::<u32>(owner + off + 0x10)
            .wrapping_add(g.read::<u32>(c + off));
        g.write::<u32>(owner + off, sum);
    }
    sum
}

/// [`visit_offset_int`].
pub const VISIT_OFFSET_INT: Sig = Sig {
    len: 166,
    hash: 0x9287_49e7_ebe7_3525,
    calls: &[],
    imports: &[],
};

/// The walk, then the owner's node `c` (`+0x38`) visited with a cursor
/// on the frame ([`visit_local`], `esp - 12`) and the owner's floats
/// `+0x1c`..`+0x24` set to its `+0x2c`..`+0x34` plus `c`'s
/// `+0x1c`..`+0x24` (`addss`). Returns the method's `eax` (the walk's
/// when the owner was visited already).
///
/// `0x10003b70` (154 bytes, thiscall `ecx` = the cursor, 116 DLLs).
pub fn visit_offset_float<G: Game>(g: &mut G, esp: u32, this: u32, eax: u32) -> u32 {
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let local = esp.wrapping_sub(12);
    let pass = g.read::<u8>(this + 5);
    let c = g.read::<u32>(owner + 0x38);
    let eax = visit_local(g, local, c, pass);
    for off in [0x1c, 0x20, 0x24] {
        let x = f32_at(g, owner + off + 0x10) + f32_at(g, c + off);
        set_f32(g, owner + off, x);
    }
    eax
}

/// [`visit_offset_float`].
pub const VISIT_OFFSET_FLOAT: Sig = Sig {
    len: 154,
    hash: 0x678e_09d7_9b14_61d8,
    calls: &[],
    imports: &[],
};

/// Visits the children of the cursor's node (its list at `+0x4`, first
/// `+0xc`, each child's `+8` the next) through their method `+0x20`,
/// each with a cursor on the frame (`[ebp - 0xc]`: `esp - 16`) holding
/// the child, unless the DLL's global (operand `+0x0b`) is set; then,
/// when the list is empty (its `+8` is the list), sets bit 0 of the
/// node's `+0x10`. Returns the last method's `eax`, or the cursor.
///
/// `0x10005d60` (75 bytes, cdecl `(cursor)`, 116 DLLs; a vtable entry).
pub fn visit_children_20<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let cursor = fr.arg(g, 0);
    let mut eax = cursor;
    let node = g.read::<u32>(cursor);
    let list = node + 4;
    let mut child = g.read::<u32>(node + 0xc);
    if g.read::<u32>(img.word(fr.at + 0x0b)) == 0 && child != list {
        let local = fr.esp.wrapping_sub(16);
        loop {
            g.write::<u32>(local, child);
            let method = g.read::<u32>(g.read::<u32>(child) + 0x20);
            child = g.read::<u32>(child + 8);
            eax = g.call(method, local, &[local]).eax;
            if child == list {
                break;
            }
        }
    }
    if g.read::<u32>(list + 8) == list {
        let v = g.read::<u32>(node + 0x10);
        g.write::<u32>(node + 0x10, v | 1);
    }
    Ret::eax(eax)
}

/// [`visit_children_20`].
pub const VISIT_CHILDREN_20: Sig = Sig {
    len: 75,
    hash: 0x94a3_1058_7a0b_c29e,
    calls: &[],
    imports: &[],
};

/// The walk (`ecx` as the caller left it), then the owner's node `c`
/// (`+0x3c`) visited with a cursor on the frame ([`visit_local`],
/// `esp - 12`) and slot 668 (table global at operand `+0x74`) called
/// with `(owner + 0x1c, c + 0x1c, owner + 0x2c)`, `c` read again.
/// Returns the slot's `eax` (the walk's when the owner was visited
/// already).
///
/// `0x10003310` (138 bytes, cdecl `(cursor)`, 116 DLLs; a vtable entry).
pub fn visit_668<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let (eax, tail) = walk(g, this, fr.ecx, fr.eax);
    let Some(owner) = tail else {
        return Ret::eax(eax);
    };
    let local = fr.esp.wrapping_sub(12);
    let pass = g.read::<u8>(this + 5);
    let c = g.read::<u32>(owner + 0x3c);
    visit_local(g, local, c, pass);
    let c = g.read::<u32>(owner + 0x3c);
    let s668 = slot_target(g, img.word(fr.at + 0x74), 668);
    Ret::eax(
        g.call(s668, local, &[owner + 0x1c, c + 0x1c, owner + 0x2c])
            .eax,
    )
}

/// [`visit_668`].
pub const VISIT_668: Sig = Sig {
    len: 138,
    hash: 0x225b_4609_6cb2_3a79,
    calls: &[],
    imports: &[],
};

/// Calls slot 779 with the cursor's node's `+0x20` and 0 while the word
/// the DLL's pointer global (operand `+0x13`, read again at `+0x2e`)
/// points at holds 1; the old value is put back after. Returns the
/// pointer (`eax`).
///
/// `0x10003e30` (60 bytes, cdecl `(cursor)`, 112 DLLs; a vtable entry).
pub fn call_779_flagged<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let cursor = fr.arg(g, 0);
    let node = g.read::<u32>(cursor);
    let arg = g.read::<u32>(node + 0x20);
    let p = g.read::<u32>(w(0x13));
    let old = g.read::<u32>(p);
    g.write::<u32>(p, 1);
    let s779 = slot_target(g, w(0x21), 779);
    g.call(s779, arg, &[arg, 0]);
    let p = g.read::<u32>(w(0x2e));
    g.write::<u32>(p, old);
    Ret::eax(p)
}

/// [`call_779_flagged`].
pub const CALL_779_FLAGGED: Sig = Sig {
    len: 60,
    hash: 0x978d_79d7_dd84_0e06,
    calls: &[],
    imports: &[],
};

/// Slot 289 (table global at operand `+0x05`) with `ecx`; its ST0 stored
/// as an `f32` and returned in `xmm0` (`eax` is the slot's).
///
/// `0x10007480` (33 bytes, `ecx` the argument, 118 DLLs).
pub fn float_289<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32) -> (f32, u32) {
    let s289 = slot_target(g, img.word(f + 0x05), 289);
    let out = g.call(s289, ecx, &[ecx]);
    (
        out.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32),
        out.eax,
    )
}

/// [`float_289`].
pub const FLOAT_289: Sig = Sig {
    len: 33,
    hash: 0x3e83_fdce_cce2_2bf6,
    calls: &[],
    imports: &[],
};

/// `comiss k, x; jb` (keep `x` when `k < x` or unordered).
fn below(k: f32, x: f32) -> bool {
    k < x || k.is_nan() || x.is_nan()
}

/// Clamps four floats at `this` (each at or below the constant at
/// operand `+0x04` becomes 0.01, then each at or above the one at
/// `+0x45` becomes 100.0), divides them by the DLL's four saved values
/// (operands `+0x94`, `+0x9c`, `+0xb1`, `+0xb9`: `q0`..`q3`) and calls
/// slot 801 (table global at `+0xd0`) with `(q0, q1, q2, q3, m, m)`, `m`
/// the largest of `q0`..`q2` as the original picks it; then saves the
/// four floats. Returns the slot's `eax`.
///
/// `0x10001870` (314 bytes, `ecx` = the four floats, 116 DLLs).
pub fn scale_ratio_801<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let lo = f32_at(g, w(0x04));
    for i in 0..4 {
        if !below(lo, f32_at(g, this + 4 * i)) {
            g.write::<u32>(this + 4 * i, 0x3c23_d70a);
        }
    }
    let hi = f32_at(g, w(0x45));
    for i in 0..4 {
        if !below(f32_at(g, this + 4 * i), hi) {
            g.write::<u32>(this + 4 * i, 0x42c8_0000);
        }
    }
    let saved = [w(0x94), w(0x9c), w(0xb1), w(0xb9)];
    let q1 = f32_at(g, this) / f32_at(g, saved[0]);
    let q2 = f32_at(g, this + 4) / f32_at(g, saved[1]);
    let q0 = f32_at(g, this + 8) / f32_at(g, saved[2]);
    let q4 = f32_at(g, this + 12) / f32_at(g, saved[3]);
    // comiss q1, q2; jbe: q2 unless q1 > q2; then comiss m, q0; ja
    let m = if q1 > q2 { q1 } else { q2 };
    let m = if m > q0 { m } else { q0 };
    let s801 = slot_target(g, w(0xd0), 801);
    let args = [q1, q2, q0, q4, m, m].map(f32::to_bits);
    let eax = g.call(s801, this, &args).eax;
    for (i, &s) in (0u32..).zip(&saved) {
        let v = g.read::<u32>(this + 4 * i);
        g.write::<u32>(s, v);
    }
    eax
}

/// [`scale_ratio_801`].
pub const SCALE_RATIO_801: Sig = Sig {
    len: 314,
    hash: 0xdd26_d2a3_2c6a_da14,
    calls: &[],
    imports: &[],
};

/// A node from a set-up record (`+0x0` the node, `+0x4` its list or 0):
/// [`node_init`] and [`attach`] as the other node constructors do.
fn node_from_record<G: Game>(g: &mut G, rec: u32) -> u32 {
    let node = g.read::<u32>(rec);
    let list = g.read::<u32>(rec + 4);
    node_init(g, node, list);
    if list != 0 {
        attach(g, node, list, ListAt::of(list));
    }
    node
}

/// A particle node from a set-up record (`ecx`: `+0x0` the node, `+0x4`
/// its list, `+0x8` a word for `+0x4c`, `+0xc` the program table's
/// owner, `+0x10` the part (for `+0x50`, and stored through the DLL's
/// pointer global, operand `+0x45`), `+0x14` the program index):
/// `+0xe8`/`+0xec` cleared, slot 941 `(owner +0x10)` gives the program
/// table, the `pppStartPart` wrapper (`rel32` at `+0x88`) starts the
/// node's buffer `+0x60` (0x1000 bytes) with the indexed program, and
/// slot 325 copies the record's vector (`owner + (index + 2) * 16`) into
/// the node's `+0x1c`; `+0xf4` and `+0xe0` cleared. Returns slot 325's
/// `eax`.
///
/// `0x10003590` (198 bytes, `ecx` = the record, 115 DLLs).
pub fn particle_node<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let node = node_from_record(g, rec);
    g.write::<u32>(node + 0x4c, g.read::<u32>(rec + 8));
    let part = g.read::<u32>(rec + 0x10);
    g.write::<u32>(node + 0x50, part);
    g.write::<u32>(node + 0xe8, 0);
    g.write::<u32>(node + 0xec, 0);
    let p = g.read::<u32>(w(0x45));
    g.write::<u32>(p, part);
    let owner = g.read::<u32>(rec + 0xc);
    let s941 = slot_target(g, w(0x69), 941);
    let table = g.call(s941, part, &[g.read::<u32>(owner + 0x10)]).eax;
    let index = g.read::<u32>(rec + 0x14);
    let program = g.read::<u32>(table.wrapping_add(4 * index));
    let wrapper = img.rel32_target(f + 0x88);
    start_part(g, img, wrapper, node + 0x60, 0x1000, program, 1);
    let index = g.read::<u32>(rec + 0x14);
    let v = (index.wrapping_add(2) << 4).wrapping_add(g.read::<u32>(rec + 0xc));
    let dst = g.read::<u32>(rec) + 0x1c;
    let s325 = slot_target(g, w(0xa0), 325);
    let eax = g.call(s325, 0, &[dst, v]).eax;
    g.write::<u32>(node + 0xf4, 0);
    g.write::<u32>(node + 0xe0, 0);
    eax
}

/// [`particle_node`].
pub const PARTICLE_NODE: Sig = Sig {
    len: 198,
    hash: 0x63a8_a140_1e93_2ebf,
    calls: &[(0x88, Callee::Sig(&START_PART))],
    imports: &[],
};

/// A node from a set-up record (`ecx`: `+0x0` the node, `+0x4` its list,
/// `+0x8`/`+0xc` two words for `+0x1c`/`+0x20`, `+0x10`/`+0x14` slot
/// 719's arguments): `+0x24` = slot 719 `(+0x10, 6, +0x14)`, `+0x34` =
/// 0x40, `+0x28` = 0x100, `+0x2c` = 0. Returns the record's `+0xc`.
///
/// `0x10004060` (122 bytes, `ecx` = the record, 106 DLLs).
pub fn effect_node<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32) -> u32 {
    let node = node_from_record(g, rec);
    let s719 = slot_target(g, img.word(f + 0x3f), 719);
    let a = [g.read::<u32>(rec + 0x10), 6, g.read::<u32>(rec + 0x14)];
    let e = g.call(s719, rec, &a).eax;
    g.write::<u32>(node + 0x24, e);
    g.write::<u32>(node + 0x1c, g.read::<u32>(rec + 8));
    let eax = g.read::<u32>(rec + 0xc);
    g.write::<u32>(node + 0x20, eax);
    g.write::<u32>(node + 0x34, 0x40);
    g.write::<u32>(node + 0x28, 0x100);
    g.write::<u32>(node + 0x2c, 0);
    eax
}

/// [`effect_node`].
pub const EFFECT_NODE: Sig = Sig {
    len: 122,
    hash: 0x07b6_797c_f25b_4a14,
    calls: &[],
    imports: &[],
};

/// The walk, then three sprite rows: slot 231 builds a matrix on the
/// frame (`esp - 0x58`) from the constant matrix (operand `+0x6b`) and
/// the owner's `+0x1c` node's `+0x9c`; slot 264 allocates 0x200 bytes
/// (a header `0, 0, 0x41, 0`, then 64 bytes per axis `i`: a colour with
/// 0xff in word `i` and 0x80 in word 3 at `+0`, the same without the
/// axis at `+0x20`, and two positions slot 565 makes from the matrix and
/// a vector on the frame (`esp - 0x18`: the axis set to the owner's
/// float `+0x20`, then to that times the constant at `+0x155`; `w` 1.0)
/// at `+0x10` and `+0x30`); slot 382 `(0, 0)` twice and slot 265 frees
/// it. Returns slot 265's `eax` (the walk's when the owner was visited
/// already).
///
/// `0x100033a0` (474 bytes, thiscall `ecx` = the cursor, 107 DLLs).
pub fn axis_sprites<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp.wrapping_sub(4);
    // the frame's cookie (`mov eax, [__security_cookie]; xor eax, ebp`)
    // is the eax the walk starts from
    let eax = g.read::<u32>(w(0x07)) ^ ebp;
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let mat = ebp.wrapping_sub(0x54);
    let vec = ebp.wrapping_sub(0x14);
    let src = g.read::<u32>(owner + 0x1c) + 0x9c;
    let s231 = slot_target(g, w(0x71), 231);
    g.call(s231, this, &[mat, w(0x6b), src]);
    let s264 = slot_target(g, w(0x7e), 264);
    let buf = g.call(s264, this, &[0x200]).eax;
    g.write::<u32>(buf, 0);
    g.write::<u32>(buf + 4, 0);
    g.write::<u32>(buf + 8, 0x41);
    g.write::<u32>(buf + 0xc, 0);
    let x = g.read::<u32>(g.read::<u32>(this) + 0x20);
    let mut row = buf + 0x10;
    for i in 0..3u32 {
        g.write::<u32>(row + 0xc, 0);
        g.write::<u32>(row, 0);
        g.write::<u32>(row + 4, 0);
        g.write::<u32>(row + 8, 0);
        for j in 0..4 {
            g.write::<u32>(vec + 4 * j, 0);
        }
        g.write::<u32>(row + 0xc, 0x80);
        g.write::<u32>(row + 4 * i, 0xff);
        for j in 0..4 {
            let v = g.read::<u32>(row + 4 * j);
            g.write::<u32>(row + 0x20 + 4 * j, v);
        }
        g.write::<u32>(row + 0x20 + 4 * i, 0);
        g.write::<u32>(vec + 0xc, 0x3f80_0000);
        g.write::<u32>(vec + 4 * i, x);
        let s565 = slot_target(g, w(0x136), 565);
        g.call(s565, this, &[row + 0x10, mat, vec, 0]);
        let y = f32_at(g, vec + 4 * i) * f32_at(g, w(0x155));
        set_f32(g, vec + 4 * i, y);
        let s565 = slot_target(g, w(0x168), 565);
        g.call(s565, this, &[row + 0x30, mat, vec, 0]);
        row += 0x40;
    }
    let s382 = slot_target(g, w(0x193), 382);
    g.call(s382, this, &[0, 0]);
    let s382 = slot_target(g, w(0x1a4), 382);
    g.call(s382, this, &[0, 0]);
    let s265 = slot_target(g, w(0x1b5), 265);
    g.call(s265, this, &[0x200]).eax
}

/// [`axis_sprites`].
pub const AXIS_SPRITES: Sig = Sig {
    len: 474,
    hash: 0x630e_efcd_8082_5b5c,
    // __security_check_cookie
    calls: &[(0x1d2, Callee::Any)],
    imports: &[],
};

/// The walk (`ecx` the owner at every method), then the owner's `+0x1c`
/// vector and its parent's `+0x1c` added into a frame vector (`esp -
/// 0x18`, slot 248); when its `y` is above the owner's float `+0x34`
/// the owner bounces: `t = (y - floor) / +0x20`, the vector becomes the
/// owner's `+0x1c..` minus their `t` share, the owner's `+0x20` is
/// `-(+0x2c) * (+0x20 * t)`, its `+0x1c`/`+0x24` become their `t` share
/// times `+0x30`, and the vector's `x`/`z` move by those (the parent's
/// `+0x1c`/`+0x24` added in), `y` to `floor + +0x20`. Slot 325 copies the
/// frame vector into the parent's `+0x1c`. Returns slot 325's `eax`
/// (the walk's when the owner was visited already).
///
/// `0x10006030` (359 bytes, thiscall `ecx` = the cursor, 107 DLLs).
pub fn bounce<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let owner0 = g.read::<u32>(this);
    // the walk starts from the frame's cookie (`[__security_cookie] ^ ebp`)
    let eax = g.read::<u32>(w(0x07)) ^ esp.wrapping_sub(4);
    let (eax, tail) = walk(g, this, owner0, eax);
    let Some(o) = tail else {
        return eax;
    };
    let vec = esp.wrapping_sub(0x18);
    let parent = g.read::<u32>(o + 0x14);
    let dst = parent + 0x1c;
    let s248 = slot_target(g, w(0x71), 248);
    g.call(s248, parent, &[vec, dst, o + 0x1c]);
    let floor = f32_at(g, o + 0x34);
    let y = f32_at(g, vec + 4);
    if y > floor {
        let d = y - floor;
        set_f32(g, vec + 4, d);
        let t = d / f32_at(g, o + 0x20);
        let x0 = f32_at(g, o + 0x1c);
        let x1 = f32_at(g, o + 0x20) * t;
        let x3 = x0 * t;
        let x2 = f32_at(g, o + 0x24) * t;
        set_f32(g, vec, x0 - x3);
        set_f32(g, vec + 4, f32_at(g, o + 0x20) - x1);
        set_f32(g, vec + 8, f32_at(g, o + 0x24) - x2);
        let mask = g.read::<u32>(w(0xf2));
        let neg = f32::from_bits(g.read::<u32>(o + 0x2c) ^ mask);
        let x1 = x1 * neg;
        set_f32(g, o + 0x20, x1);
        set_f32(g, vec + 4, f32_at(g, o + 0x34) + x1);
        let x3 = x3 * f32_at(g, o + 0x30);
        set_f32(g, o + 0x1c, x3);
        let x2 = x2 * f32_at(g, o + 0x30);
        set_f32(g, o + 0x24, x2);
        let v0 = x3 + f32_at(g, dst) + f32_at(g, vec);
        set_f32(g, vec, v0);
        let v2 = f32_at(g, parent + 0x24) + x2 + f32_at(g, vec + 8);
        set_f32(g, vec + 8, v2);
    }
    let s325 = slot_target(g, w(0x147), 325);
    g.call(s325, parent, &[dst, vec]).eax
}

/// [`bounce`].
pub const BOUNCE: Sig = Sig {
    len: 359,
    hash: 0x2f68_51ac_e7d6_4d0f,
    // __security_check_cookie
    calls: &[(0x15f, Callee::Any)],
    imports: &[],
};

/// The walk (`ecx` the owner at every method), then the owner's model
/// (`+0x20`, `S`) drawn from a record on the 16-aligned frame (`B` =
/// `((esp - 4) & !15) - 0xf0`; at `B + 0x30`: flags from `S`'s byte 0
/// (bit 0 -> 0x80, bit 1 clear -> 0x10), the word the DLL's pointer
/// global (operand `+0xb6`) points at, `S`'s `+0x16`, the owner's
/// `+0x24` at `+0x18`, the matrix pointer four times from `+0x20`, `S`'s
/// two tables at `+0x38`, zeroes) and a matrix at `B + 0xa8` (slot 231
/// from the constant at `+0xf0` and the owner's `+0x1c` node's `+0x9c`);
/// `S` at `B + 0xc` and `B + 0x2c`. With the game lock (`*G_LOCK`,
/// operand `+0x11a`) set, slot 773 `(S, 0)` says whether the model is
/// ready: then slot 819 `(S, 0, B + 0x10, B + 0x30)` and slot 775
/// `(S, 0)`, else slot 820 `(S, 0, B + 0x10, B + 0x30, 776(byte) |
/// 777(0))`. The lock word is put back. Returns the lock pointer (the
/// walk's `eax` when the owner was visited already).
///
/// `0x10003c20` (528 bytes, thiscall `ecx` = the cursor, 103 DLLs).
pub fn draw_model<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let b = (esp.wrapping_sub(4) & !15).wrapping_sub(0xf0);
    let owner0 = g.read::<u32>(this);
    // the walk starts from the frame's cookie (`[__security_cookie] ^ esp`,
    // `esp` = `B + 8` then)
    let eax = g.read::<u32>(w(0x0d)) ^ (b + 8);
    let (eax, tail) = walk(g, this, owner0, eax);
    let Some(o) = tail else {
        return eax;
    };
    let s = g.read::<u32>(o + 0x20);
    g.write::<u32>(b + 0xc, s);
    let b0 = g.read::<u8>(s);
    let mut flags = u32::from(b0 & 1) << 7;
    if b0 & 2 == 0 {
        flags |= 0x10;
    }
    g.write::<u32>(b + 0x30, flags);
    g.write::<u16>(b + 0x38, g.read::<u16>(s + 0x16));
    g.write::<u32>(b + 0x4c, 0);
    g.write::<u32>(b + 0x68, g.read::<u32>(s + 8).wrapping_add(s));
    g.write::<u32>(b + 0x6c, g.read::<u32>(s + 0xc).wrapping_add(s));
    let p = g.read::<u32>(w(0xb6));
    g.write::<u32>(b + 0x34, g.read::<u32>(p));
    g.write::<u32>(b + 0x3c, 0);
    g.write::<u32>(b + 0x40, 0);
    g.write::<u32>(b + 0x48, g.read::<u32>(o + 0x24));
    g.write::<u32>(b + 0x70, 0);
    let mat = b + 0xa8;
    let src = g.read::<u32>(o + 0x1c) + 0x9c;
    let s231 = slot_target(g, w(0xf6), 231);
    g.call(s231, o, &[mat, w(0xf0), src]);
    for off in [0x50, 0x54, 0x58, 0x5c] {
        g.write::<u32>(b + off, mat);
    }
    let lock = g.read::<u32>(w(0x11a));
    g.write::<u32>(b + 0x74, 0);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    g.write::<u32>(b + 0x2c, s);
    let s773 = slot_target(g, w(0x135), 773);
    let ready = g.call(s773, o, &[s, 0]).eax;
    let lock = g.read::<u32>(w(0x14a));
    g.write::<u32>(lock, saved);
    let mut saved = saved;
    if ready != 0 {
        g.write::<u32>(lock, 1);
        let s819 = slot_target(g, w(0x162), 819);
        g.call(s819, lock, &[s, 0, b + 0x10, b + 0x30]);
        let lock = g.read::<u32>(w(0x178));
        g.write::<u32>(lock, 1);
        let s775 = slot_target(g, w(0x185), 775);
        g.call(s775, lock, &[s, 0]);
    } else {
        let s776 = slot_target(g, w(0x19e), 776);
        let byte = u32::from(g.read::<u8>(b + 0x38));
        let x = g.call(s776, lock, &[byte]).eax;
        let s777 = slot_target(g, w(0x1ac), 777);
        let y = g.call(s777, s777, &[g.read::<u32>(b + 0x74)]).eax;
        let lock = g.read::<u32>(w(0x1c3));
        saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        let s = g.read::<u32>(b + 0xc);
        let s820 = slot_target(g, w(0x1da), 820);
        g.call(s820, lock, &[s, 0, b + 0x10, b + 0x30, x | y]);
    }
    let lock = g.read::<u32>(w(0x1f6));
    g.write::<u32>(lock, saved);
    lock
}

/// [`draw_model`].
pub const DRAW_MODEL: Sig = Sig {
    len: 528,
    hash: 0x15ae_bac5_85b2_9ac5,
    // __security_check_cookie
    calls: &[(0x208, Callee::Any)],
    imports: &[],
};
