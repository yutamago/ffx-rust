//! Round 6 shape families: exact pairs (two DLLs' copies of one function
//! that differ only in their globals and immediates; families of
//! `~/ffx-work/m5/magic/r4/families.tsv`). Each port reads its operands
//! from the copy's code (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A scene's start (no arguments; `magic_0162` 0x10003820 and
/// `magic_0586` 0x10003430, 212 bytes; family `F0162_10003820`): with
/// `S` = `*G_S` (`+0x08`), `S + 0x85d4..+0x85e0` from the immediates;
/// slot 399 `(*G_A, 1)` (`+0x38`) and slot 399 `(TABLE[(*G_B)[1]], 1)`
/// (`TABLE` at `+0x52`, `G_B` at `+0x45`; the earlier call's words below
/// them); `S + 0xa20..+0xa28` from the immediates; a node from the pool
/// ([`crate::sa_pool::take`], its owner the callback `CB` at `+0x67`)
/// appended to the list (`G_FIRST` `+0x90`, `G_LAST` `+0xa0`) and kept
/// in `S + 0x85c4`. An empty pool gives node 0, which the original
/// writes through (it faults; so does the port). Returns the node.
pub fn scene_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = w(0x01);
    let s = g.read::<u32>(w(0x08));
    let store = |g: &mut G, d: u32, v: u32| g.write::<u32>(s.wrapping_add(w(d)), w(v));
    for (d, v) in [(0x10, 0x14), (0x1a, 0x1e), (0x24, 0x28), (0x2e, 0x32)] {
        store(g, d, v);
    }
    let mut stack = vec![g.read::<u32>(w(0x38)), disp8(img, f + 0x0d)];
    // call width: 2
    g.call(slot_target(g, t, w(0x3e) / 4), fr.ecx, &stack);
    let b = g.read::<u32>(w(0x45));
    let i = u32::from(g.read::<u8>(b.wrapping_add(1)));
    let entry = g.read::<u32>(w(0x52).wrapping_add(4 * i));
    stack.splice(0..0, [entry, disp8(img, f + 0x4a)]);
    // call width: 4 (the earlier call's words stay on the stack)
    g.call(slot_target(g, w(0x57), w(0x5d) / 4), fr.ecx, &stack);
    for (d, v) in [(0x6d, 0x71), (0x77, 0x7b), (0x81, 0x85)] {
        store(g, d, v);
    }
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x8a), w(0x67));
    let empty = g.read::<u32>(w(0x90)) == 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(s.wrapping_add(w(0xc4)), node);
        g.write::<u32>(w(0xc9), node);
        g.write::<u32>(w(0xce), node);
    } else {
        let last = g.read::<u32>(w(0xa0));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(s.wrapping_add(w(0xae)), node);
        g.write::<u32>(w(0xb3), node);
    }
    Ret::eax(node)
}

/// A ring effect's part start (cdecl `(obj)`; `magic_0666` 0x10001250
/// and 0x10002080, 213 bytes; family `F0666_10001250`): `*G_P = ITEM`
/// (`+0x04`, `+0x0b`); the program list `L` = slot 941 `((*G_D)[0x10])`
/// (`G_D` at `+0x10`) and the part `L[k]` (`+0x2e`) started on `obj`
/// ([`crate::opu_parts::start_part`], `edx` 0x1000, `(L[k], 1)`); eight
/// constant stores `+0x40..+0x5c`; the colour `+0x60..+0x6c` from four
/// immediates, replaced by four others when `obj` is one of the first
/// 0x80 bytes past `BASE` (`+0x81`) and the count `*G_N` (`+0xaa`) is at
/// least `M` (`+0xae`). Returns 1.
pub fn ring_part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.arg(g, 0);
    let p = g.read::<u32>(w(0x04));
    g.write::<u32>(p, w(0x0b));
    let d = g.read::<u32>(w(0x10));
    let arg = g.read::<u32>(d.wrapping_add(disp8(img, f + 0x17)));
    // call width: 1
    let list = g
        .call(slot_target(g, w(0x19), w(0x1f) / 4), fr.ecx, &[arg])
        .eax;
    let program = g.read::<u32>(list.wrapping_add(disp8(img, f + 0x2e)));
    let one = disp8(img, f + 0x2b);
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(f + 0x37),
        obj,
        w(0x30),
        program,
        one,
    );
    for k in 0..8u32 {
        let at = f + 0x47 + 7 * k;
        g.write::<u32>(obj.wrapping_add(disp8(img, at + 2)), img.word(at + 3));
    }
    let colour = [0x3du32, 0x40, 0x43, 0x46].map(|o| obj.wrapping_add(disp8(img, f + o)));
    for (&at, imm) in colour.iter().zip([0x8a, 0x90, 0x96, 0x9c]) {
        g.write::<u32>(at, w(imm));
    }
    let k = obj.wrapping_sub(w(0x81));
    if k & 0xffff_ff80 == 0 && g.read::<i32>(w(0xaa)) >= w(0xae).cast_signed() {
        for (&at, imm) in colour.iter().zip([0xb6, 0xbc, 0xc2, 0xc8]) {
            g.write::<u32>(at, w(imm));
        }
    }
    Ret::eax(w(0xce))
}

/// A camera block's angle reset (no arguments; `magic_0353` 0x10001980
/// and `magic_0672` 0x10001500, 232 bytes; family `F0353_10001980`):
/// with `B` = `*G_B` (`+0x0e`), `B + 0xe40` = 0; the angle `x = K1 -
/// slot 333 (*G_A)` (ST0 as `f32`; `G_A` at `+0x26`) brought into range
/// once (`x > K2`: `x - K3`; else `K4 > x`: `x + K3`; NaN stays), stored
/// in `B + 0xe44`, `+0xe48`/`+0xe4c` 0; for the vector block `V = B +
/// 0xe50`, slot 262 `(*G_A, V, V + 4, V + 8)` and slot 12 `(*G_A)` (its
/// float into `V + 4`, the earlier words below), `V + 0xc` = 1.0; `B +
/// 0x13e0`, `B + 0x13fc` 0 and `*G_DONE` = 1. Returns `B`.
pub fn angle_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = w(0x07);
    let b = g.read::<u32>(w(0x0e));
    g.write::<u32>(b.wrapping_add(w(0x15)), w(0x19));
    let a = g.read::<u32>(w(0x26));
    // call width: 1
    let out = g.call(slot_target(g, t, w(0x1f) / 4), fr.ecx, &[a]);
    let r = f32::from_bits(crate::sa_summon::st0_f32(out));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let mut x = sub(k(g, 0x36), r);
    if x > k(g, 0x45) {
        x = sub(x, k(g, 0x4f));
    } else if k(g, 0x59) > x {
        x = add(x, k(g, 0x66));
    }
    g.write::<f32>(b.wrapping_add(w(0x6e)), x);
    g.write::<u32>(b.wrapping_add(w(0x74)), w(0x78));
    g.write::<u32>(b.wrapping_add(w(0x7e)), w(0x82));
    let a = g.read::<u32>(w(0x88));
    let v = b.wrapping_add(w(0x8e));
    let (v8, v4) = (
        v.wrapping_add(disp8(img, f + 0x94)),
        v.wrapping_add(disp8(img, f + 0x9d)),
    );
    let mut stack = vec![a, v, v4, v8];
    // call width: 4
    g.call(slot_target(g, w(0x97), w(0xa0) / 4), fr.ecx, &stack);
    stack.insert(0, a);
    // call width: 5 (the earlier call's words stay on the stack)
    let out = g.call(
        slot_target(g, w(0xaa), disp8(img, f + 0xb1) / 4),
        fr.ecx,
        &stack,
    );
    g.write::<u32>(v4, crate::sa_summon::st0_f32(out));
    g.write::<u32>(v.wrapping_add(disp8(img, f + 0xbe)), w(0xbf));
    g.write::<u32>(b.wrapping_add(w(0xc7)), w(0xcb));
    g.write::<u32>(b.wrapping_add(w(0xd1)), w(0xd5));
    g.write::<u32>(w(0xdb), w(0xdf));
    Ret::eax(b)
}

/// An effect object's part start by place (cdecl `(obj)`; `magic_0206`
/// 0x100017b0 and `magic_0376` 0x10001170, 235 bytes; family
/// `F0206_100017b0`): `*G_P = ITEM` (`+0x14`, `+0x1a`); the program list
/// `L` = slot 941 `((*G_D)[0x10])` (`G_D` at `+0x1f`); when `obj` is one
/// of the first 0x80 bytes past `BASE` (`+0x0a`) the part `L[2]`
/// (`+0x3b`) is started on it ([`crate::opu_parts::start_part`]) with
/// the first set of seven stores (`+0x48..`), else `L[3]` (`+0x83`) with
/// the second (`+0x90..`); then `+0x6c`, `+0x5c` (1), `+0x58`, `+0x50`,
/// `+0x4c` from the tail's stores. Returns 1.
pub fn placed_part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.arg(g, 0);
    let near = obj.wrapping_sub(w(0x0a)) & 0xffff_ff80 == 0;
    let p = g.read::<u32>(w(0x14));
    g.write::<u32>(p, w(0x1a));
    let d = g.read::<u32>(w(0x1f));
    let arg = g.read::<u32>(d.wrapping_add(disp8(img, f + 0x25)));
    // call width: 1
    let list = g
        .call(slot_target(g, w(0x27), w(0x2d) / 4), fr.ecx, &[arg])
        .eax;
    // the branch's operands: program entry, `push 1`, `mov edx`, the
    // call's rel32, the first store
    let (entry, one, edx, call, stores) = if near {
        (0x3b, 0x38, 0x3d, 0x44, 0x48)
    } else {
        (0x83, 0x80, 0x85, 0x8c, 0x90)
    };
    let program = g.read::<u32>(list.wrapping_add(disp8(img, f + entry)));
    let one = disp8(img, f + one);
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(f + call),
        obj,
        w(edx),
        program,
        one,
    );
    for k in 0..7u32 {
        let at = f + stores + 7 * k;
        g.write::<u32>(obj.wrapping_add(disp8(img, at + 2)), img.word(at + 3));
    }
    let eax = w(0xc2);
    g.write::<u32>(obj.wrapping_add(disp8(img, f + 0xcb)), w(0xcc));
    g.write::<u32>(obj.wrapping_add(disp8(img, f + 0xd2)), eax);
    for at in [0xd3u32, 0xda, 0xe1] {
        g.write::<u32>(obj.wrapping_add(disp8(img, f + at + 2)), w(at + 3));
    }
    Ret::eax(eax)
}

/// The `mov dword [esi + d8], imm32` stores at `f + at`, 7 bytes each,
/// on `obj`.
fn stores7<G: Game>(g: &mut G, img: &Image, obj: u32, f: u32, at: &[u32]) {
    for &a in at {
        g.write::<u32>(obj.wrapping_add(disp8(img, f + a + 2)), img.word(f + a + 3));
    }
}

/// An effect object's part start by slot (cdecl `(obj)`; `magic_0371`
/// 0x10001520 and 0x10001620, 246 bytes; family `F0371_10001520`): `*G_P
/// = ITEM` (`+0x04`, `+0x0e`); the program list `L` = slot 941
/// `((*G_D)[0x10])` (`G_D` at `+0x13`) and `L[0]` started on `obj`
/// ([`crate::opu_parts::start_part`]); then by the object's slot `i =
/// (obj - BASE) >> 7` (`BASE` at `+0x2a`, arithmetic shift): 0, 1 and 2
/// each store their position (and colour; 1 shares 0's colour), any
/// other skips to the common tail; slots 0..2 also set `+0x4c`, `+0x6c`;
/// every slot sets `+0x50..+0x5c`. Returns 1.
pub fn slot_part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.arg(g, 0);
    let p = g.read::<u32>(w(0x04));
    g.write::<u32>(p, w(0x0e));
    let d = g.read::<u32>(w(0x13));
    let arg = g.read::<u32>(d.wrapping_add(disp8(img, f + 0x1a)));
    let slot = obj.wrapping_sub(w(0x2a)).cast_signed() >> 7;
    // call width: 1
    let list = g
        .call(slot_target(g, w(0x1c), w(0x24) / 4), fr.ecx, &[arg])
        .eax;
    let program = g.read::<u32>(list);
    let one = disp8(img, f + 0x36);
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(f + 0x41),
        obj,
        w(0x3a),
        program,
        one,
    );
    let tail = match slot {
        0 => {
            stores7(g, img, obj, f, &[0x4c, 0x53, 0x5a, 0x61, 0x68, 0x6f]);
            true
        }
        1 => {
            stores7(g, img, obj, f, &[0x7d, 0x84, 0x8b, 0x61, 0x68, 0x6f]);
            true
        }
        2 => {
            stores7(g, img, obj, f, &[0x99, 0xa0, 0xa7, 0xae, 0xb5, 0xbc]);
            true
        }
        _ => false,
    };
    if tail {
        stores7(g, img, obj, f, &[0xc3, 0xca]);
    }
    stores7(g, img, obj, f, &[0xd2, 0xd9, 0xe0, 0xe7]);
    Ret::eax(w(0xef))
}

include!("o6_fam_pairs_table.rs.in");
