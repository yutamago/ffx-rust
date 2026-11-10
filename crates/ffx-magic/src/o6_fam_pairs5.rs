//! Round 6 shape families, second round (agent magic-families2): exact
//! pairs, two DLLs' copies of one function that differ only in their
//! globals and immediates (families of
//! `~/ffx-work/m5/magic/r4/families.tsv`). Each port reads its operands
//! from the copy's code (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::opu_parts::start_part;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object callback on a level look matrix (`(obj)`; a /GS frame of
/// 0x44 bytes, `ebp` = `esp - 4` at the entry; `magic_0664` 0x100016e0
/// and `magic_0045` 0x10001310, 302 bytes; family `F0664_100016e0`;
/// `magic_0665` 0x100016d0, 307 bytes, passes the matrix a stack word):
/// - the level look matrix ([`crate::s6_misc::level_look_matrix`]) into
///   `M` (`ebp - 0x44`) from the source `*G_S` (`*G_T` when `obj` is not
///   within the 0x80 bytes from `BASE`), the height `K` and the scale
///   `obj + 0x60`;
/// - slot 230 `(*G_A, M)`, slot 231 `(*G_C, *G_B, *G_A)`, slot 230 `(*G_D,
///   *G_C)`;
/// - `*G_P = ITEM`; slot 712 `(obj, 0x30)` with the game flag `*G_LOCK` 1
///   around it; when that is nonzero, `*G_P = ITEM`, slot 713 `(obj)`,
///   `*G_P = ITEM` again, slot 941 `(*(*G_Q + 0x10), obj)` gives a part
///   list whose entry (`+0x14` or `+0x34`, the `disp8` at `+0x10a`)
///   starts through the `pppStartPart` wrapper `(obj, 0x1000, entry, 1)`.
///
/// Each call sees the words the earlier ones left on the stack. Returns 1.
pub fn look_matrix_callback<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.arg(g, 0);
    // `magic_0665`'s copy: `push 0x3` (the matrix's stack word) at
    // `+0x2e`, operands after it 2 bytes later, and `add esp, 0x4` after
    // the call, operands from there on 5 bytes later
    let (arg, a, b) = if img.word(f + 0x2e) & 0xff == 0x6a {
        (Some(img.word(f + 0x2f) & 0xff), 2, 5)
    } else {
        (None, 0, 0)
    };
    let ebp = fr.esp.wrapping_sub(4);
    let m = ebp.wrapping_sub(0x44);
    let height = g.read::<u32>(w(0x14));
    let scale = g.read::<u32>(obj.wrapping_add(0x60));
    // test eax, 0xffffff80: obj within the 0x80 bytes from BASE
    let src = if obj.wrapping_sub(w(0x2a)) & 0xffff_ff80 == 0 {
        g.read::<u32>(w(0x1a))
    } else {
        g.read::<u32>(w(0x3a + a))
    };
    // ebp, the 0x44-byte frame and edi pushed: the callee's return
    // address at `ebp - 0x4c`
    let mut esp = ebp.wrapping_sub(0x4c);
    if let Some(x) = arg {
        g.write::<u32>(esp, x);
        esp = esp.wrapping_sub(4);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x3f + a),
        esp,
        ecx: m,
        edx: src,
        xmm: [0, 0, u128::from(height), u128::from(scale)],
        ..Frame::default()
    };
    crate::s6_misc::level_look_matrix(g, img, &callee);
    // (the saved esi under the first call's words is left out: no callee
    // reads that deep)
    let mut s = Pushes::default();
    let slot = |g: &G, t: u32, d: u32| slot_target(g, w(t), w(d) / 4);
    s.push(m);
    let t = slot(g, 0x49 + b, 0x55 + b);
    s.push(g.read::<u32>(w(0x4f + b)));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, m);
    s.push(g.read::<u32>(w(0x5d + b)));
    let t = slot(g, 0x62 + b, 0x6e + b);
    s.push(g.read::<u32>(w(0x68 + b)));
    s.push(g.read::<u32>(w(0x74 + b)));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, m);
    s.push(g.read::<u32>(w(0x7c + b)));
    let t = slot(g, 0x81 + b, 0x8d + b);
    s.push(g.read::<u32>(w(0x87 + b)));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, m);
    let p = g.read::<u32>(w(0x94 + b));
    s.push(disp8(img, f + 0x99 + b));
    g.write::<u32>(p, w(0x9c + b));
    let lock = g.read::<u32>(w(0xa1 + b));
    s.push(obj);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot(g, 0xaf + b, 0xb5 + b);
    // call width: 12 (Pushes caps the words at 12)
    let r = s.call(g, t, m).eax;
    let lock = g.read::<u32>(w(0xbd + b));
    g.write::<u32>(lock, saved);
    if r == 0 {
        return Ret::eax(1);
    }
    let p = g.read::<u32>(w(0xcc + b));
    g.write::<u32>(p, w(0xd3 + b));
    let t = slot(g, 0xd8 + b, 0xde + b);
    // call width: 1
    g.call(t, lock, &[obj]);
    let p = g.read::<u32>(w(0xe5 + b));
    g.write::<u32>(p, w(0xeb + b));
    let q = g.read::<u32>(w(0xf0 + b));
    let d = g.read::<u32>(q.wrapping_add(disp8(img, f + 0xf6 + b)));
    let t = slot(g, 0xf8 + b, 0xfe + b);
    // call width: 2 (slot 713's word under it)
    let list = g.call(t, lock, &[d, obj]).eax;
    let entry = g.read::<u32>(list.wrapping_add(disp8(img, f + 0x10a + b)));
    start_part(
        g,
        img,
        img.rel32_target(f + 0x113 + b),
        obj,
        w(0x10c + b),
        entry,
        disp8(img, f + 0x107 + b),
    );
    Ret::eax(1)
}

/// A record's start on a node of its own (internal: `ecx` = the record
/// `r`, `xmm3` = a scale `s`, stack `(pos, s1, s2)`; a /GS frame of 0x28
/// bytes, `ebp` = `esp - 4` at the entry; `magic_0199` 0x10002160 and
/// `magic_0251` 0x10002550, 621 bytes; family `F0199_10002160`):
/// - slot 217 `(A, &L)` (`A` = `*G_A`, `L` at `ebp - 0x24`), `L`'s four
///   words into `r + 0x20..`, `pos`'s four into `r + 0x30..`;
/// - slot 234 `(&D, pos, &L)` (`D` at `ebp - 0x14`); `r + 0x40..0x48` =
///   0, `D` into `r + 0x80..`;
/// - slot 233 `(r + 0x50, &D, s)`, `(r + 0x60, &D, s1)`, `(r + 0x70, &D,
///   s2)` (each with the earlier words under it);
/// - the diagonal `r + 0x4c, 0x5c, 0x6c, 0x7c, 0x8c` = 1.0, `r + 0xa0` =
///   `A`, `+0xa4` and `+0x9c` the copy's immediates, `+0x90`, `+0x94`,
///   `+0x98`, `+0xa8` = 0, the u16 `+0xb0` = 0;
/// - a node `a` taken from the free ring (count `G_FREE` u16, head
///   `G_HEAD`): unlinked (`a.prev.next = a.next`, `a.next.prev = a.prev`,
///   each re-read), the head moves to `a.next` (0 when it was the last),
///   the count drops by one; `a` cleared (`+0`, `+4`, `+0xc`, `+0x14`;
///   byte `+0x18`), `+8` = `a`, `+0x10` = the callback; then `+0x14` =
///   `r`, byte `+0x18` = 1, `+4` = 0, and `a` appended to the busy list
///   (`G_LAST`, `G_FIRST`: as the only node, or after the last).
///
/// With an empty ring the original writes through `a` = 0: it faults (the
/// port writes there too). Returns `eax` as the original leaves it: the
/// old last node, or 0xffff when the list was empty.
pub fn record_node_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ops = NodeOps {
        a: 0x18,
        t217: (0x21, 0x28),
        t234: (0x7e, 0x84),
        t233: [(0xe9, 0xef), (0x10b, 0x111), (0x12d, 0x133)],
        free: 0x14d,
        k_a4: 0x177,
        k_9c: 0x18b,
        head: 0x1bf,
        callback: 0x215,
        free_w: 0x220,
        last_cmp: 0x226,
        last: 0x23f,
        last_w: 0x252,
        first_w: 0x25d,
    };
    node_start(g, img, fr, fr.ecx, &ops)
}

/// [`record_node_start`] on a record at a fixed address (no `ecx`: the
/// record `R` is the immediate the node gets at `+0x26f`, every field
/// written through its own address; `magic_0253` 0x10004370 and
/// `magic_0311` 0x10001f60, 684 bytes; family `F0253_10004370`).
pub fn global_node_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ops = NodeOps {
        a: 0x17,
        t217: (0x20, 0x27),
        t234: (0x9b, 0xa9),
        t233: [(0xf5, 0x119), (0x136, 0x141), (0x159, 0x164)],
        free: 0x1b2,
        k_a4: 0x1ab,
        k_9c: 0x1c8,
        head: 0x1fb,
        callback: 0x251,
        free_w: 0x25c,
        last_cmp: 0x262,
        last: 0x27e,
        last_w: 0x291,
        first_w: 0x29c,
    };
    let r = img.word(fr.at + 0x26f);
    node_start(g, img, fr, r, &ops)
}

/// The operand offsets of a [`node_start`] copy: `G_A`, the slot tables
/// and displacements (217, 234, the three 233 calls), `G_FREE` (read),
/// the `+0xa4` and `+0x9c` immediates, `G_HEAD`, the callback,
/// `G_FREE` (written), `G_LAST` (compared, read, written), `G_FIRST`.
pub(crate) struct NodeOps {
    pub(crate) a: u32,
    pub(crate) t217: (u32, u32),
    pub(crate) t234: (u32, u32),
    pub(crate) t233: [(u32, u32); 3],
    pub(crate) free: u32,
    pub(crate) k_a4: u32,
    pub(crate) k_9c: u32,
    pub(crate) head: u32,
    pub(crate) callback: u32,
    pub(crate) free_w: u32,
    pub(crate) last_cmp: u32,
    pub(crate) last: u32,
    pub(crate) last_w: u32,
    pub(crate) first_w: u32,
}

/// The body of [`record_node_start`] for the record `r`.
fn node_start<G: Game>(g: &mut G, img: &Image, fr: &Frame, r: u32, ops: &NodeOps) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let a = g.read::<u32>(w(ops.a));
    node_start_with(g, img, fr, r, ops, a, |_| (w(ops.k_a4), w(ops.k_9c)))
}

/// [`node_start`] with the character `a` given and the words `+0xa4`
/// and `+0x9c` from `tail` (called where the copy computes them).
#[allow(clippy::too_many_lines)] // the original's stores, one by one
pub(crate) fn node_start_with<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    r: u32,
    ops: &NodeOps,
    a: u32,
    tail: impl FnOnce(&G) -> (u32, u32),
) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, (t, d): (u32, u32)| slot_target(g, w(t), w(d) / 4);
    let pos = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (l, d) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let mut s = Pushes::default();
    s.push_all(&[l, a]);
    let t = slot(g, ops.t217);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, fr.ecx);
    for (o, src) in [(0x20u32, l), (0x30, pos)] {
        for i in 0..4 {
            let v = g.read::<u32>(src.wrapping_add(4 * i));
            g.write::<u32>(r.wrapping_add(o + 4 * i), v);
        }
    }
    s.push_all(&[l, pos, d]);
    let t = slot(g, ops.t234);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, fr.ecx);
    for o in [0x48u32, 0x44, 0x40] {
        g.write::<u32>(r.wrapping_add(o), 0);
    }
    for i in 0..4 {
        let v = g.read::<u32>(d.wrapping_add(4 * i));
        g.write::<u32>(r.wrapping_add(0x80 + 4 * i), v);
    }
    // add esp, 0x10, then movss [esp], xmm0: the scales take the place of
    // the first call's last word
    s.pop(4);
    for (k, (dst, scale)) in [
        (0x50u32, fr.xmm[3] as u32),
        (0x60, fr.arg(g, 1)),
        (0x70, fr.arg(g, 2)),
    ]
    .into_iter()
    .enumerate()
    {
        if k > 0 {
            s.pop(2);
        }
        s.set_top(scale);
        s.push_all(&[d, r.wrapping_add(dst)]);
        let t = slot(g, ops.t233[k]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, t, fr.ecx);
    }
    for o in [0x4cu32, 0x5c] {
        g.write::<u32>(r.wrapping_add(o), 0x3f80_0000);
    }
    let free = g.read::<u16>(w(ops.free));
    for o in [0x6cu32, 0x7c, 0x8c] {
        g.write::<u32>(r.wrapping_add(o), 0x3f80_0000);
    }
    let (a4, k9c) = tail(g);
    g.write::<u32>(r.wrapping_add(0xa0), a);
    g.write::<u32>(r.wrapping_add(0xa4), a4);
    g.write::<u32>(r.wrapping_add(0x98), 0);
    g.write::<u32>(r.wrapping_add(0x9c), k9c);
    for o in [0x90u32, 0x94, 0xa8] {
        g.write::<u32>(r.wrapping_add(o), 0);
    }
    g.write::<u16>(r.wrapping_add(0xb0), 0);
    Ret::eax(take_busy_node(g, img, f, free, ops, r, true))
}

/// The node take of [`record_node_start`] and [`target_objects`]: with
/// `free` nodes left (the u16 `G_FREE` as read), the ring's head `a` is
/// unlinked (`a.prev.next = a.next`, `a.next.prev = a.prev`, each re-read),
/// the head moves to `a.next` (0 when it was the last), the count drops
/// by one; `a` is cleared (`+0`, `+4`, `+0xc`, `+0x14`; byte `+0x18`),
/// `+8` = `a`, `+0x10` = the callback. Then `+0x14` = `r` (and with
/// `mark` the byte `+0x18` = 1), `+4` = 0, and `a` is appended to the
/// busy list (`G_LAST`, `G_FIRST`: as the only node, or after the last).
/// With no free node the original writes through `a` = 0: it faults (the
/// port writes there too). Returns `eax` as the original leaves it: the
/// old last node, or 0xffff when the list was empty (0 with no free
/// node).
pub(crate) fn take_busy_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    mut free: u16,
    ops: &NodeOps,
    r: u32,
    mark: bool,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let mut eax = 0u32;
    let mut node = 0u32;
    if free != 0 {
        node = g.read::<u32>(w(ops.head));
        let prev = g.read::<u32>(node);
        let next = g.read::<u32>(node.wrapping_add(4));
        g.write::<u32>(prev.wrapping_add(4), next);
        let next = g.read::<u32>(node.wrapping_add(4));
        let prev = g.read::<u32>(node);
        g.write::<u32>(next, prev);
        let head = if free > 1 {
            g.read::<u32>(node.wrapping_add(4))
        } else {
            0
        };
        g.write::<u32>(w(ops.head), head);
        eax = 0xffff;
        free = free.wrapping_sub(1);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node.wrapping_add(o), 0);
        }
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(node.wrapping_add(0x10), w(ops.callback));
        g.write::<u8>(node.wrapping_add(0x18), 0);
        g.write::<u16>(w(ops.free_w), free);
    }
    let empty = g.read::<u32>(w(ops.last_cmp)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), r);
    if mark {
        g.write::<u8>(node.wrapping_add(0x18), 1);
    }
    g.write::<u32>(node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(ops.last_w), node);
    } else {
        let last = g.read::<u32>(w(ops.last));
        eax = last;
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    }
    g.write::<u32>(w(ops.first_w), node);
    g.write::<u32>(node.wrapping_add(8), node);
    eax
}

/// An object's record taken off its list (`(obj)`; `magic_0479`
/// 0x10003c00 and `magic_0673` 0x100015b0, 81 bytes; family
/// `F0479_10003c00`): the record `R = BASE + 0x130 * obj[+IDX]` (`IDX`
/// the `disp8` at `+0x09`; `BASE` an immediate in `0479`, the word
/// `*G_BASE` in `0673`) is removed from the list `L`
/// ([`crate::l5_objhost::list_remove`]); when it was there, `*G_P =
/// ITEM`, slot 713 `(R)` and the u16 `R + 0x124` loses bit 15. Returns 0
/// when it was not there, else the `0x7fff` mask.
pub fn record_unlist<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.arg(g, 0);
    let idx = g.read::<u32>(obj.wrapping_add(disp8(img, f + 0x09)));
    let stride = w(0x0c);
    // `add esi, imm32` (81 c6) or `add esi, [G]` (03 35)
    let imm = img.word(f + 0x10) as u8 == 0x81;
    let base = if imm { w(0x12) } else { g.read::<u32>(w(0x12)) };
    let rec = idx.wrapping_mul(stride).wrapping_add(base);
    let list = if imm { w(0x19) } else { w(0x17) };
    let found = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x1e), list, rec);
    if found == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x27));
    g.write::<u32>(p, w(0x2e));
    let t = slot_target(g, w(0x33), w(0x39) / 4);
    // call width: 1
    g.call(t, list, &[rec]);
    let mask = w(0x40);
    let at = rec.wrapping_add(w(0x4a));
    let v = g.read::<u16>(at) & mask as u16;
    g.write::<u16>(at, v);
    Ret::eax(mask)
}

/// A turned point and, while a counter lasts, two particles around it
/// (no arguments; a /GS frame of 0x78 bytes, `ebp` = `esp - 4` at the
/// entry; `magic_0162` 0x100048a0 and `magic_0598` 0x10007ef0, 695 bytes;
/// family `F0162_100048a0`). With the object `O = *G_O`:
/// - the matrix `M` (`ebp - 0x74`): slot 235 `(M)`, slot 328 `(M, M, 0)`,
///   slot 329 `(M, M, a)` (`a` the f32 `O + ANGLE`, the `disp32` at
///   `+0x1f`); slot 229 `(N, M, (1, 0, 0, 1))` (`N` at `ebp - 0x34`, the
///   vector at `ebp - 0x14`); the point `P = O + POINT` (`+0x52`) through
///   slot 248 `(P, P, N)`, its `w` 1.0;
/// - when the counter `O + COUNT` (`+0xcc`) is above 0 it drops by one and
///   two particles are taken from the pool ([`crate::sb_misc::pool_take`];
///   none left ends it, returning 0). Each gets the halfword `+0x38` =
///   0xffff, `+0x30` = `*(*G_R + 0xc)`, `+0x34` = 1, the halfword `+0x3a`
///   = 0x200; slot 237 `(T[b], rand() % 101 + 3, 0, &C)` (`b` the byte
///   `*G_S + B`, `T` the table at `+0x13d`, `C` at `ebp - 0x24`, its `w`
///   1.0); `+0..+8` = `C`'s `x, y, z` plus slot 314 `(5.0)` each (ST0 as
///   f32), `+0xc` = 1.0; `+0x10..+0x18` slot 314 `(0.25)`, `+0x1c` slot
///   314 `(pi)`; `+0x20` 0, `+0x24` 0.01, `+0x28` 0, `+0x2c` 1.0; `+0x3c`
///   = `(slot 404 (0.8) + K1) * K2`; `+0x4c` 0x20, `+0x50` 0x10, `+0x54`
///   -16, the byte `+0x6b` and `+0x60` 0.
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// the counter when it was 0 or below, else the last slot 404's `eax`.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn two_particle_burst<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, t: u32, d: u32| slot_target(g, w(t), w(d) / 4);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, n, v, c) = (
        ebp.wrapping_sub(0x74),
        ebp.wrapping_sub(0x34),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x24),
    );
    let obj = g.read::<u32>(w(0x14));
    let angle = g.read::<u32>(obj.wrapping_add(w(0x1f)));
    for (i, x) in [1.0f32, 0.0, 0.0, 1.0].into_iter().enumerate() {
        g.write::<f32>(v.wrapping_add(4 * i as u32), x);
    }
    let point = obj.wrapping_add(w(0x52));
    let mut s = Pushes::default();
    s.push(m);
    let t = slot(g, 0x25, 0x30);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, fr.ecx);
    s.set_top(w(0x5e));
    s.push_all(&[m, m]);
    let t = slot(g, 0x65, 0x6b);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, fr.ecx);
    s.pop(2);
    s.set_top(angle);
    s.push_all(&[m, m]);
    let t = slot(g, 0x84, 0x8a);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, fr.ecx);
    s.push_all(&[v, m, n]);
    g.write::<u32>(ebp.wrapping_sub(8), w(0xa4));
    let t = slot(g, 0x9d, 0xaa);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, fr.ecx);
    s.push_all(&[n, point, point]);
    let t = slot(g, 0xb5, 0xbc);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, fr.ecx);
    g.write::<u32>(point.wrapping_add(disp8(img, f + 0xc5)), w(0xc6));
    let count_at = obj.wrapping_add(w(0xcc));
    let count = g.read::<u32>(count_at);
    if count.cast_signed() <= 0 {
        return Ret::eax(count);
    }
    g.write::<u32>(count_at, count.wrapping_sub(1));
    let owner = g.read::<u32>(g.read::<u32>(w(0xe3)).wrapping_add(disp8(img, f + 0xea)));
    let take = img.rel32_target(f + 0xf1);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let mut eax = 0;
    for _ in 0..2 {
        let p = crate::sb_misc::pool_take(g, img, take);
        if p == 0 {
            return Ret::eax(0);
        }
        g.write::<u16>(p.wrapping_add(0x38), 0xffff);
        g.write::<u32>(p.wrapping_add(0x30), owner);
        g.write::<u32>(p.wrapping_add(0x34), w(0x111));
        g.write::<u16>(p.wrapping_add(0x3a), w(0x107) as u16);
        let r = g.rand().cast_signed() % w(0x121).cast_signed();
        let b = g.read::<u8>(g.read::<u32>(w(0x12c)).wrapping_add(disp8(img, f + 0x135)));
        let colour = g.read::<u32>(w(0x13d).wrapping_add(4 * u32::from(b)));
        let mut s = Pushes::default();
        s.push_all(&[
            c,
            0,
            r.wrapping_add(disp8(img, f + 0x138).cast_signed())
                .cast_unsigned(),
            colour,
        ]);
        let t = slot(g, 0x142, 0x148);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, t, fr.ecx);
        s.pop(3);
        g.write::<u32>(ebp.wrapping_sub(0x18), w(0x159));
        // slot 314 `(x)`, ST0 stored as f32
        let st0 = |g: &mut G, s: &mut Pushes, arg: u32, t: u32, d: u32| {
            s.set_top(arg);
            let target = slot(g, t, d);
            // call width: 12 (Pushes caps the words at 12)
            let out = s.call(g, target, fr.ecx);
            (out.eax, crate::ob_model::st0_f32(out.st0))
        };
        for (i, (k5, t, d)) in [
            (0x166u32, 0x14f, 0x15f),
            (0x181, 0x17a, 0x18b),
            (0x1a6, 0x19f, 0x1b1),
        ]
        .into_iter()
        .enumerate()
        {
            let i = 4 * i as u32;
            let (_, x) = st0(g, &mut s, w(k5), t, d);
            let sum = crate::sb_sse::add(g.read::<f32>(c.wrapping_add(i)), x);
            if i == 8 {
                g.write::<u32>(p.wrapping_add(0xc), w(0x1cc));
            }
            g.write::<f32>(p.wrapping_add(i), sum);
        }
        for (o, (k4, t, d)) in [
            (0x10u32, (0x1d3u32, 0x1c5u32, 0x1de)),
            (0x14, (0x1ef, 0x1e8, 0x1f5)),
            (0x18, (0x206, 0x1ff, 0x20c)),
            (0x1c, (0x21d, 0x216, 0x223)),
        ] {
            let (_, x) = st0(g, &mut s, w(k4), t, d);
            g.write::<f32>(p.wrapping_add(o), x);
        }
        for (o, v) in [
            (0x20u32, 0x234u32),
            (0x24, 0x23b),
            (0x28, 0x242),
            (0x2c, 0x249),
        ] {
            g.write::<u32>(p.wrapping_add(o), w(v));
        }
        let (r404, x) = st0(g, &mut s, w(0x256), 0x22d, 0x24f);
        eax = r404;
        let y = crate::sb_sse::mul(crate::sb_sse::add(x, k(g, 0x268)), k(g, 0x273));
        g.write::<f32>(p.wrapping_add(0x3c), y);
        g.write::<u32>(p.wrapping_add(0x4c), w(0x280));
        g.write::<u32>(p.wrapping_add(0x50), w(0x287));
        g.write::<u32>(p.wrapping_add(0x54), w(0x28e));
        g.write::<u8>(p.wrapping_add(0x6b), 0);
        g.write::<u32>(p.wrapping_add(0x60), 0);
    }
    Ret::eax(eax)
}

/// A node task dropping particles below a level (`(node)`; a /GS frame
/// of 0x50 bytes, `ebp` = `esp - 4` at the entry; `magic_0307` 0x10005310
/// and `magic_0563` 0x100035b0, 711 bytes; family `F0307_10005310`):
/// - while the i16 delay `node + 0x14` is above 0 it counts down (returns
///   it, zero-extended, minus 1);
/// - else the i16 life `node + 0x16` counts down; at 0 or below the node
///   goes back to the pool ([`crate::sa_pool::give_back`], its `eax`);
/// - else four vectors `V0..V3` (`ebp - 0x44`, `- 0x34`, `- 0x24`, `-
///   0x14`) from slot 237 `(*G_M, part, 0, &V)` (the parts the copy's
///   immediates), each `w` 1.0 after its call; the level `y = slot 12
///   (*G_M)` (ST0 as f32) `- K1`; for `V0` and `V1`, unless `y > V.y`, a
///   particle from the pool ([`crate::sb_misc::pool_take`]; none left ends
///   it, returning 0): the halfword `+0x38` = 0xffff, `+0x30` = `*(*G_R +
///   R)`, `+0x34` = 1, the halfword `+0x3a` = 0x200, `+0..` = `V`; `+0`
///   plus slot 314 `(1.0)`, `+4` = `y - K2`, `+8` plus slot 314 `(1.0)`;
///   `+0x10` slot 314 `(0.15)`, `+0x14` slot 314 `(0.04) - K3`, `+0x18`
///   slot 314 `(0.15)`, `+0x1c` slot 314 `(pi)` (each ST0 as f32); `+0x20`
///   0, `+0x24` 1e-4, `+0x28` 0, `+0x2c` 1.0; `+0x3c` = `(x + K2) * K4`
///   with `x` from slot 314 `(0.2)`; `+0x4c` and `+0x50` 0x20, the byte
///   `+0x6b` and `+0x60` 0.
///
/// Returns `ebp - 0x20` after the two vectors (the loop's pointer).
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn level_particles<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, t: u32, d: u32| slot_target(g, w(t), w(d) / 4);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let node = fr.arg(g, 0);
    let delay = g.read::<u16>(node.wrapping_add(0x14));
    if delay.cast_signed() > 0 {
        let left = u32::from(delay).wrapping_sub(1);
        g.write::<u16>(node.wrapping_add(0x14), left as u16);
        return Ret::eax(left);
    }
    let life = g.read::<u16>(node.wrapping_add(0x16)).wrapping_sub(1);
    g.write::<u16>(node.wrapping_add(0x16), life);
    if life.cast_signed() <= 0 {
        let r = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x3b), node);
        return Ret::eax(r);
    }
    let ebp = fr.esp.wrapping_sub(4);
    // (V, the part's operand and whether it is a byte, the `w` store
    // after the call: its immediate; the table, `push [G_M]`, the slot's
    // displacement)
    let vectors = [
        (0x44u32, 0x61u32, true, 0x80u32, 0x54u32, 0x64u32, 0x5cu32),
        (0x34, 0x76, true, 0xa5, 0x6f, 0x79, 0x86),
        (0x24, 0x98, false, 0xca, 0x91, 0x9e, 0xab),
        (0x14, 0xbd, false, 0xe1, 0xb6, 0xc3, 0xd0),
    ];
    let mut s = Pushes::default();
    for (v, part, byte, one, t, gm, d) in vectors {
        let v = ebp.wrapping_sub(v);
        let part = if byte { disp8(img, f + part) } else { w(part) };
        let target = slot(g, t, d);
        s.push_all(&[v, 0, part, g.read::<u32>(w(gm))]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, target, fr.ecx);
        g.write::<u32>(v.wrapping_add(0xc), w(one));
    }
    // add esp, 0x40: the four calls' words gone
    let mut s = Pushes::default();
    s.push(g.read::<u32>(w(0xe7)));
    let t = slot_target(g, w(0xd7), disp8(img, f + 0xed) / 4);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, t, fr.ecx);
    let y = sub(crate::ob_model::st0_f32(out.st0), k(g, 0xfc));
    let take = img.rel32_target(f + 0x11d);
    let mut q = ebp.wrapping_sub(0x40);
    for _ in 0..2 {
        if y > g.read::<f32>(q) {
            q = q.wrapping_add(0x10);
            continue;
        }
        let p = crate::sb_misc::pool_take(g, img, take);
        if p == 0 {
            return Ret::eax(0);
        }
        let owner = g.read::<u32>(g.read::<u32>(w(0x12d)).wrapping_add(disp8(img, f + 0x136)));
        g.write::<u16>(p.wrapping_add(0x38), 0xffff);
        g.write::<u32>(p.wrapping_add(0x30), owner);
        g.write::<u32>(p.wrapping_add(0x34), w(0x144));
        g.write::<u16>(p.wrapping_add(0x3a), w(0x149) as u16);
        for i in 0..4u32 {
            let x = g.read::<u32>(q.wrapping_sub(4).wrapping_add(4 * i));
            g.write::<u32>(p.wrapping_add(4 * i), x);
        }
        let mut s = Pushes::default();
        s.push(q);
        let st0 = |g: &mut G, s: &mut Pushes, arg: u32, t: u32, d: u32| {
            let target = slot(g, t, d);
            s.set_top(arg);
            // call width: 12 (Pushes caps the words at 12)
            crate::ob_model::st0_f32(s.call(g, target, fr.ecx).st0)
        };
        let x = st0(g, &mut s, w(0x176), 0x168, 0x16f);
        let v = add(g.read::<f32>(p), x);
        g.write::<f32>(p, v);
        g.write::<f32>(p.wrapping_add(4), sub(y, k(g, 0x1a1)));
        let x = st0(g, &mut s, w(0x190), 0x189, 0x1ac);
        let v = add(g.read::<f32>(p.wrapping_add(8)), x);
        g.write::<f32>(p.wrapping_add(8), v);
        let x = st0(g, &mut s, w(0x1c7), 0x1c0, 0x1d2);
        g.write::<f32>(p.wrapping_add(0x10), x);
        let x = st0(g, &mut s, w(0x1e3), 0x1dc, 0x1e9);
        g.write::<f32>(p.wrapping_add(0x14), sub(x, k(g, 0x1fb)));
        let x = st0(g, &mut s, w(0x207), 0x200, 0x212);
        g.write::<f32>(p.wrapping_add(0x18), x);
        let x = st0(g, &mut s, w(0x223), 0x21c, 0x229);
        g.write::<f32>(p.wrapping_add(0x1c), x);
        for (o, v) in [
            (0x20u32, 0x23au32),
            (0x24, 0x241),
            (0x28, 0x248),
            (0x2c, 0x24f),
        ] {
            g.write::<u32>(p.wrapping_add(o), w(v));
        }
        let x = st0(g, &mut s, w(0x25c), 0x233, 0x255);
        g.write::<u32>(p.wrapping_add(0x4c), w(0x280));
        g.write::<u32>(p.wrapping_add(0x50), w(0x28f));
        g.write::<u8>(p.wrapping_add(0x6b), 0);
        g.write::<f32>(p.wrapping_add(0x3c), mul(add(x, k(g, 0x26e)), k(g, 0x288)));
        g.write::<u32>(p.wrapping_add(0x60), 0);
        q = q.wrapping_add(0x10);
    }
    Ret::eax(q)
}

/// A two-part node's visit (internal, `ecx` = the cursor `C`; a /GS frame
/// of 0x50 bytes, `ebp` = `esp - 4` at the entry; `magic_0070` 0x10001160
/// and `magic_0577` 0x10001000, 823 bytes; family `F0070_10001160`;
/// reached through the 12-byte thunk [`two_part_visit_thunk`]). With the
/// node `N = *C`:
/// - when `N`'s byte `+0x1a` is `C`'s byte `+5` already, `C`'s byte `+4`
///   is 0 and nothing more happens; else each child of `N`'s ring (from
///   `N + 0x10`, through `+8`, back to `N`) is visited with `*C` = the
///   child (its class's `+0x1c` `(C)`), then `*C = N`, `N + 0x1a` = `C +
///   5` and `C + 4` = 1;
/// - with `S = *C`: its parts `S + 0x6c` and `S + 0x70` visited with a
///   cursor `L` on the frame (`ebp - 0x4c`: the part, the byte `+5` =
///   `C + 5`; each call sees the earlier one's word);
/// - when bit 0 of `S + 0x68` is set and the f32 `S + 0x28` is above 0:
///   slot 231 `(&M, T, B + 0x9c)` (`M` at `ebp - 0x44`, `T` an immediate,
///   `B` = `S + 0x70`), slot 245 `(&L.byte8)`; then when `S + 0x5c` is
///   above 0: slot 821 `(0)`, slot 822 with `S + 0x1c..0x28` times `K2`,
///   slot 823 `(1 / S + 0x5c, ...)` twice, slot 824 `(A + 0x1c, A +
///   0x20, A + 0x24, 1.0)` (`A` = `S + 0x6c`);
/// - when bit 1 of `S + 0x68` is set, `S + 0x1c..0x28` move towards `S +
///   0x2c..0x38` by `S + 0x4c` and `S + 0x5c` towards `S + 0x60` by `S +
///   0x54` (`(to - x) * k + x`); else unless the step `S + 0x50` is at 1
///   (`K`) already, `S + 0x1c..` += `S + 0x3c.. * S + 0x4c` and the step
///   grows by `S + 0x4c`; at 1 or above `S + 0x1c..` = `S + 0x2c..`; then
///   unless `S + 0x58` is at 1, `S + 0x5c += S + 0x64 * S + 0x54` and
///   `S + 0x58 += S + 0x54`; with `S + 0x50` at 1, `S + 0x5c` = `S +
///   0x60`.
///
/// Returns `eax` as the original leaves it: the last call's, or the last
/// word copied; on the first way (nothing to do) only its low byte is
/// known (the byte compared; the rest is the /GS cookie's).
///
/// `magic_0295`'s copy (0x10001000, 813 bytes) lights without testing
/// bit 0 (only `S + 0x28` above 0; `movss xmm0, [esi+0x28]` at `+0x8e`,
/// its `K` at `+0x97`) and eases with any bit of `S + 0x68` set (`cmp
/// dword, 0`); its operands from the light on are ten bytes earlier.
/// `magic_0153`'s copy (0x10001000, 810 bytes) lights when `S + 0x28` is
/// not at most 0 (`comiss 0, [S + 0x28]; jae`: a NaN lights too), takes
/// the slot block when `S + 0x5c` is above the f32 at `+0xe1` (`comiss
/// xmm1, [K]`), and eases on any flag; its operands are 14 bytes earlier
/// from `+0xb4`, 13 from `+0xf8`.
#[allow(clippy::too_many_lines)] // the original's branches, one by one
pub fn two_part_visit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    // `movss xmm3, [K]` (`f3 0f 10 1d`) or `movss xmm0, [esi+0x28]` (`.. 46`)
    let plain = img.word(f + 0x8e) >> 24 == 0x46;
    // `xorps xmm0, xmm0` (0x0f) after that load, not `add esp, 8`
    let nan_lights = !plain && img.word(f + 0x96) & 0xff == 0x0f;
    let m = |o: u32| match o {
        _ if o < 0xb4 => o,
        _ if plain => o - 10,
        _ if !nan_lights => o,
        0xb4..0xf8 => o - 14,
        _ => o - 13,
    };
    let w = |o: u32| img.word(f + m(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let c = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let node = g.read::<u32>(c);
    let mark = g.read::<u8>(node.wrapping_add(0x1a));
    if mark == g.read::<u8>(c.wrapping_add(5)) {
        g.write::<u8>(c.wrapping_add(4), 0);
        return Ret::eax(u32::from(mark));
    }
    let mut eax = 0;
    let mut child = g.read::<u32>(node.wrapping_add(0x10));
    if child != node {
        loop {
            g.write::<u32>(c, child);
            let cur = g.read::<u32>(c);
            child = g.read::<u32>(child.wrapping_add(8));
            let visit = g.read::<u32>(g.read::<u32>(cur).wrapping_add(0x1c));
            // call width: 1
            g.call(visit, c, &[c]);
            if child == node {
                break;
            }
        }
    }
    let byte5 = g.read::<u8>(c.wrapping_add(5));
    g.write::<u32>(c, node);
    g.write::<u8>(node.wrapping_add(0x1a), byte5);
    g.write::<u8>(c.wrapping_add(4), 1);
    // test byte [edi + 4], 1: just set
    let s = g.read::<u32>(c);
    let l = ebp.wrapping_sub(0x4c);
    let mut words = Pushes::default();
    for part in [0x6cu32, 0x70] {
        let byte5 = g.read::<u8>(c.wrapping_add(5));
        let p = g.read::<u32>(s.wrapping_add(part));
        g.write::<u32>(l, p);
        g.write::<u8>(l.wrapping_add(5), byte5);
        let visit = g.read::<u32>(g.read::<u32>(p).wrapping_add(0x1c));
        words.push(l);
        // call width: 12 (Pushes caps the words at 12)
        eax = words.call(g, visit, l).eax;
    }
    let slot = |g: &G, t: u32, d: u32| slot_target(g, w(t), w(d) / 4);
    let mut one = k(g, if plain { 0x97 } else { 0x92 });
    let bit0 = plain || nan_lights || g.read::<u8>(s.wrapping_add(0x68)) & 1 != 0;
    let x28 = g.read::<f32>(s.wrapping_add(0x28));
    // `comiss 0, x; jae` (skipped when 0 is not below: a NaN lights)
    let lit = if nan_lights {
        x28 > 0.0 || x28.is_nan()
    } else {
        x28 > 0.0
    };
    if bit0 && lit {
        let b = g.read::<u32>(s.wrapping_add(0x70)).wrapping_add(w(0xb8));
        let mut st = Pushes::default();
        st.push_all(&[b, w(0xc1), ebp.wrapping_sub(0x44)]);
        let t = slot(g, 0xc7, 0xcd);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, l);
        st.push(ebp.wrapping_sub(0x48));
        let t = slot(g, 0xd8, 0xde);
        // call width: 12 (Pushes caps the words at 12)
        eax = st.call(g, t, l).eax;
        let speed = g.read::<f32>(s.wrapping_add(0x5c));
        let floor = if nan_lights {
            g.read::<f32>(img.word(f + 0xe1))
        } else {
            0.0
        };
        if speed > floor {
            let inv = div(k(g, 0xfc), speed);
            let mut st = Pushes::default();
            st.push(0);
            let t = slot(g, 0x101, 0x10b);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, l);
            let k2 = k(g, 0x11c);
            let scaled = |g: &G, o: u32| mul(g.read::<f32>(s.wrapping_add(o)), k2).to_bits();
            let t = slot(g, 0x126, 0x130);
            // sub esp, 0xc: the 0 pushed before is the fourth word
            st.set_top(scaled(g, 0x28));
            st.push_all(&[scaled(g, 0x24), scaled(g, 0x20), scaled(g, 0x1c)]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, l);
            // add esp, 8, then both words left written over with 1 / speed
            st.pop(4);
            let t = slot(g, 0x171, 0x17a);
            st.push_all(&[inv.to_bits(), inv.to_bits()]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, l);
            let a = g.read::<u32>(s.wrapping_add(0x6c));
            st.pop(2);
            st.push_all(&[w(0x192), g.read::<u32>(a.wrapping_add(0x24))]);
            st.push_all(&[
                g.read::<u32>(a.wrapping_add(0x20)),
                g.read::<u32>(a.wrapping_add(0x1c)),
            ]);
            let t = slot(g, 0x1b4, 0x1bf);
            // call width: 12 (Pushes caps the words at 12)
            eax = st.call(g, t, l).eax;
        }
        one = k(g, 0x1cc);
    }
    let f32_at = |g: &G, o: u32| g.read::<f32>(s.wrapping_add(o));
    let ease = if plain || nan_lights {
        g.read::<u32>(s.wrapping_add(0x68)) != 0
    } else {
        g.read::<u8>(s.wrapping_add(0x68)) & 2 != 0
    };
    if ease {
        let rate = |g: &G| f32_at(g, 0x4c);
        for (x, to) in [(0x1cu32, 0x2cu32), (0x20, 0x30), (0x24, 0x34), (0x28, 0x38)] {
            let v = add(mul(sub(f32_at(g, to), f32_at(g, x)), rate(g)), f32_at(g, x));
            g.write::<f32>(s.wrapping_add(x), v);
        }
        let v = add(
            mul(sub(f32_at(g, 0x60), f32_at(g, 0x5c)), f32_at(g, 0x54)),
            f32_at(g, 0x5c),
        );
        g.write::<f32>(s.wrapping_add(0x5c), v);
        return Ret::eax(eax);
    }
    // comiss, jae: taken when not below (ordered); jb: below or unordered
    let at_end = |x: f32| !(x < one || x.is_nan());
    let copy = |g: &mut G, from: u32, to: u32| {
        let v = g.read::<u32>(s.wrapping_add(from));
        g.write::<u32>(s.wrapping_add(to), v);
        v
    };
    let mut done = at_end(f32_at(g, 0x50));
    if !done {
        for (x, v) in [(0x1cu32, 0x3cu32), (0x20, 0x40), (0x24, 0x44), (0x28, 0x48)] {
            let y = add(mul(f32_at(g, v), f32_at(g, 0x4c)), f32_at(g, x));
            g.write::<f32>(s.wrapping_add(x), y);
        }
        let step = add(f32_at(g, 0x50), f32_at(g, 0x4c));
        g.write::<f32>(s.wrapping_add(0x50), step);
        done = at_end(step);
    }
    if done {
        for (to, from) in [(0x1cu32, 0x2cu32), (0x20, 0x30), (0x24, 0x34), (0x28, 0x38)] {
            eax = copy(g, from, to);
        }
    }
    let fade = f32_at(g, 0x58);
    if !at_end(fade) {
        let y = mul(f32_at(g, 0x64), f32_at(g, 0x54));
        let fade = add(fade, f32_at(g, 0x54));
        let y = add(y, f32_at(g, 0x5c));
        g.write::<f32>(s.wrapping_add(0x58), fade);
        g.write::<f32>(s.wrapping_add(0x5c), y);
        if !at_end(f32_at(g, 0x50)) {
            return Ret::eax(eax);
        }
    }
    eax = copy(g, 0x60, 0x5c);
    Ret::eax(eax)
}

/// A scrolling texture drawn from a GS `TEX0` word (no arguments; an
/// aligned /GS frame: `ebp` = the entry `esp - 4`, the locals from `E` =
/// `(ebp & !0xf) - 0x1c8`; `magic_0188` 0x10001220 and `magic_0351`
/// 0x10001080, 650 bytes; family `F0188_10001220`):
/// - slot 719 `(*G_TEX, 2, n)` (`n` 3 or 5, the `push imm8` at `+0x27`)
///   gives the `TEX0` word `V` (64 bits); slot 235 `(M1)`, `(M2)` (`M1` =
///   `E + 0x40`, `M2` = `E + 0x80`);
/// - `M1`'s diagonal `+0`, `+0x14`, `+0x28` = 0.001; the count `C0 =
///   *G_CNT` moves on by `*G_D + *G_E`;
/// - slot 885 `(TBP0, PSM, 0, 0, 1 << TW, 1 << TH, M3)` (`V`'s bits 0-13,
///   20-25, 26-29 and 30-33; `M3` = `E + 0xc0`) with the game lock
///   `*G_LOCK` 1 around it; when it gives 0, that is all (returns 0);
/// - else, the lock 1 again, slot 879 `(*(*G_F + 0x830), M3, v, u, &Q, K,
///   M1, W, H, 0)` with `W`/`H` the texture's sizes as floats, `u = ((*G_C1
///   % (H << 16)) >> 4) / H * KD`, `v = ((C0 % (W << 16)) >> 4) / W * KD`
///   (integer divisions), `K` the f32 at `+0x33`, `Q` = `(1.5625,
///   0.234375, 0.390625, (k & 0xff) * KC)` at `E + 0x30` (`k` = `K * KB`
///   truncated); the lock gets its old value. Returns the lock's address.
///
/// Each call sees the words the earlier ones left on the stack.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn scrolled_texture<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, t: u32, d: u32| slot_target(g, w(t), w(d) / 4);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !0xf).wrapping_sub(0x1c8);
    let (m1, m2, m3, q) = (
        e.wrapping_add(0x40),
        e.wrapping_add(0x80),
        e.wrapping_add(0xc0),
        e.wrapping_add(0x30),
    );
    let ka = k(g, 0x33);
    let mut s = Pushes::default();
    let t = slot(g, 0x1b, 0x22);
    s.push_all(&[disp8(img, f + 0x28), disp8(img, f + 0x38)]);
    s.push(g.read::<u32>(w(0x3b)));
    // call width: 12 (Pushes caps the words at 12)
    let tex = s.call(g, t, fr.ecx).eax;
    for (m, tab, d) in [(m1, 0x49, 0x51), (m2, 0x5e, 0x6c)] {
        let t = slot(g, tab, d);
        s.push(m);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, t, fr.ecx);
    }
    let alpha = crate::s4_summon::trunc(mul(ka, k(g, 0x7c)));
    let count = g.read::<u32>(w(0x82));
    g.write::<u32>(m1, w(0x92));
    let c1 = g.read::<u32>(w(0x9b));
    let step = g.read::<u32>(w(0xa4)).wrapping_add(g.read::<u32>(w(0xaa)));
    g.write::<u32>(m1.wrapping_add(0x14), w(0xb2));
    g.write::<u32>(m1.wrapping_add(0x28), w(0xc4));
    let fx = g.read::<u32>(g.read::<u32>(w(0xb9)).wrapping_add(w(0xca)));
    let v = u64::from(g.read::<u32>(tex)) | u64::from(g.read::<u32>(tex.wrapping_add(4))) << 32;
    g.write::<u32>(w(0xdb), count.wrapping_add(step));
    // the shifts keep 16 bits (`shl ax, cl`)
    let tw = u32::from(1u16 << ((v >> 26) & 0xf));
    let th = u32::from(1u16 << ((v >> 30) & 0xf));
    let lock = g.read::<u32>(w(0x10f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, w(0x120));
    let psm = ((v >> 20) & 0x3f) as u32;
    let tbp = (v & 0x3fff) as u32;
    s.push_all(&[m3, th, tw, 0, 0, psm, tbp]);
    let t = slot(g, 0x156, 0x15c);
    // call width: 12 (Pushes caps the words at 12)
    let made = s.call(g, t, fr.ecx).eax;
    let lock = g.read::<u32>(w(0x164));
    g.write::<u32>(lock, saved);
    if made == 0 {
        return Ret::eax(0);
    }
    let tint = mul(cvt(alpha & 0xff), k(g, 0x192));
    g.write::<u32>(lock, w(0x1c4));
    let u = mul(cvt(tex_scroll(c1, th)), k(g, 0x229));
    let vv = mul(cvt(tex_scroll(count, tw)), k(g, 0x24d));
    for (i, o) in [0x1fbu32, 0x203, 0x20b].into_iter().enumerate() {
        g.write::<u32>(q.wrapping_add(4 * i as u32), w(o));
    }
    g.write::<f32>(q.wrapping_add(0xc), tint);
    let mut s = Pushes::default();
    s.push_all(&[
        0,
        cvt(th).to_bits(),
        cvt(tw).to_bits(),
        m1,
        ka.to_bits(),
        q,
        u.to_bits(),
        vv.to_bits(),
        m3,
        fx,
    ]);
    let t = slot(g, 0x258, 0x262);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, fr.ecx);
    let lock = g.read::<u32>(w(0x269));
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// The children of the cursor's node visited once per pass (the start of
/// [`two_part_visit`] and [`ribbon_visit`]): when the node's byte `+0x1a`
/// is the cursor's byte `+5` already, the cursor's byte `+4` = 0 and the
/// result is `None`; else each child of the node's ring (from `+0x10`,
/// through `+8`, back to the node) is visited with `*C` = the child (its
/// class's `+0x1c` `(C)`), then `*C` = the node, its `+0x1a` = `C + 5`
/// and `C + 4` = 1. Returns the last visit's `eax` (0 without children:
/// the original's is then the /GS cookie's).
fn visit_children<G: Game>(g: &mut G, c: u32) -> Option<u32> {
    let node = g.read::<u32>(c);
    let mark = g.read::<u8>(node.wrapping_add(0x1a));
    if mark == g.read::<u8>(c.wrapping_add(5)) {
        g.write::<u8>(c.wrapping_add(4), 0);
        return None;
    }
    let mut child = g.read::<u32>(node.wrapping_add(0x10));
    let mut eax = 0;
    while child != node {
        g.write::<u32>(c, child);
        let cur = g.read::<u32>(c);
        child = g.read::<u32>(child.wrapping_add(8));
        let visit = g.read::<u32>(g.read::<u32>(cur).wrapping_add(0x1c));
        // call width: 1
        eax = g.call(visit, c, &[c]).eax;
    }
    let byte5 = g.read::<u8>(c.wrapping_add(5));
    g.write::<u32>(c, node);
    g.write::<u8>(node.wrapping_add(0x1a), byte5);
    g.write::<u8>(c.wrapping_add(4), 1);
    Some(eax)
}

/// A ribbon node's visit (internal, `ecx` = the cursor `C`; an aligned
/// /GS frame: `ebp` = the entry `esp - 4`, the locals from `E` = `(ebp &
/// !0xf) - 0x58`; `magic_0070` 0x100086d0, 364 bytes, and `magic_0155`
/// 0x10008170, 376 bytes; family `F0070_100086d0`; reached through the
/// 12-byte thunks [`ribbon_visit_thunk`]):
/// - the children visited once per pass ([`visit_children`]; nothing
///   more when the node was marked already); `S = *C`;
/// - the vector `V` (`E + 0x44`) = `(0, 0, -1, 1)`, `X` (`E + 0x10`) =
///   `(1.0, 1.0833)` (the copy's immediates);
/// - when bit 0 of `S + 0x54` is set: slot 677 `(S + 0x30, *(S + 0x50),
///   *(S + 0x44) + 0x1c, V)` and the bit cleared;
/// - slot 232 `(V, *(S + 0x48) + K)` (`K` the `add eax, imm32` at
///   `+0xc5`), `V.w` = 1.0;
/// - the record `P` (`E + 0x18`) = `(S + 0x30, *(S + 0x4c) + 0x1c, *(S +
///   0x40), *(S + 0x50), *(S + 0x44) + 0x1c, V)`; slot 679 `(P)` (with
///   the two earlier words under it in `0070`; in `0155` only while
///   `*G_SKIP` is 0);
/// - slot 680 `(S + 0x30, I2, X, *(S + 0x40), I1, 0x48, 0, 0, 0, S +
///   0x1c, S)` (`I1`, `I2` the copy's immediates).
///
/// Returns slot 680's `eax`; when the node was marked, only the low byte
/// of the original's `eax` is known (the byte compared; the rest is the
/// /GS cookie's), and the port returns that byte.
pub fn ribbon_visit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, t: u32, d: u32| slot_target(g, w(t), w(d) / 4);
    let c = fr.ecx;
    // `lea eax, [esp + 0x54]` (0070: the words for slot 232 stay) or `add
    // esp, 8` (0155) at +0x10e
    let kept = img.word(f + 0x10e) as u8 == 0x8d;
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !0xf).wrapping_sub(0x58);
    let (v, x, p) = (
        e.wrapping_add(0x44),
        e.wrapping_add(0x10),
        e.wrapping_add(0x18),
    );
    for (i, o) in [0x1bu32, 0x25, 0x2d, 0x35].into_iter().enumerate() {
        g.write::<u32>(v.wrapping_add(4 * i as u32), w(o));
    }
    g.write::<u32>(x, w(0x3d));
    g.write::<u32>(x.wrapping_add(4), w(0x45));
    let node = g.read::<u32>(c);
    g.write::<u32>(e.wrapping_add(0xc), node);
    if visit_children(g, c).is_none() {
        return Ret::eax(u32::from(g.read::<u8>(node.wrapping_add(0x1a))));
    }
    let s = g.read::<u32>(c);
    let field = |g: &G, o: u32| g.read::<u32>(s.wrapping_add(o));
    if g.read::<u8>(s.wrapping_add(0x54)) & 1 != 0 {
        let words = [
            s.wrapping_add(0x30),
            field(g, 0x50),
            field(g, 0x44).wrapping_add(0x1c),
            v,
        ];
        let t = slot(g, 0xaf, 0xb5);
        // call width: 4
        g.call(t, v, &words);
        let b = g.read::<u8>(s.wrapping_add(0x54)) & 0xfe;
        g.write::<u8>(s.wrapping_add(0x54), b);
    }
    let mut st = Pushes::default();
    st.push_all(&[field(g, 0x48).wrapping_add(w(0xc6)), v]);
    let t = slot(g, 0xd1, 0xd7);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, v);
    g.write::<u32>(v.wrapping_add(0xc), w(0xe1));
    let record = [
        s.wrapping_add(0x30),
        field(g, 0x4c).wrapping_add(0x1c),
        field(g, 0x40),
        field(g, 0x50),
        field(g, 0x44).wrapping_add(0x1c),
        v,
    ];
    for (i, r) in (0u32..).zip(record) {
        g.write::<u32>(p.wrapping_add(4 * i), r);
    }
    // (slot 680's operands: `push 0x48`, `push I1`, the table, `push I2`,
    // the displacement)
    let ops680 = if kept {
        st.push(p);
        let t = slot(g, 0x11c, 0x122);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, v);
        [0x137u32, 0x139, 0x146, 0x14b, 0x151]
    } else {
        if g.read::<u32>(w(0x113)) == 0 {
            let t = slot(g, 0x128, 0x12e);
            // call width: 1 (the earlier words popped)
            g.call(t, v, &[p]);
        }
        [0x143, 0x145, 0x152, 0x157, 0x15d]
    };
    let words = [
        s,
        s.wrapping_add(0x1c),
        0,
        0,
        0,
        disp8(img, f + ops680[0]),
        w(ops680[1]),
        field(g, 0x40),
        x,
        w(ops680[3]),
        s.wrapping_add(0x30),
    ];
    let mut st = Pushes::default();
    st.push_all(&words);
    let t = slot(g, ops680[2], ops680[4]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(st.call(g, t, v).eax)
}

/// The thunks of [`ribbon_visit`] (`magic_0070` 0x10008840, `magic_0155`
/// 0x10008340).
pub fn ribbon_visit_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::o6_fam_pairs3::thunk_to(g, img, fr, ribbon_visit::<G>)
}

/// The operand offsets of a [`timed_items_visit`] copy (the two copies
/// lay the node out 0x10 bytes apart and branch differently).
struct TimedOps {
    /// The flags word `S + FLAGS`, the count `S + COUNT`, the first
    /// item's vector `S + VEC` and record `S + ITEM`, the step between
    /// records, the time `S + TIME`, the slot table (`mov eax, [T]`) of
    /// the first and the second loop.
    flags: u32,
    count: u32,
    vec: u32,
    item: u32,
    stride: u32,
    time: u32,
    table1: u32,
    table2: u32,
    /// `K1`'s and `K2`'s operands, `*G_X`'s, the time-left call's
    /// `rel32` field.
    k1: u32,
    k2: u32,
    gx: u32,
    left: u32,
}

/// A timed node's visit (internal, `ecx` = the cursor `C`; an aligned
/// /GS frame: `ebp` = the entry `esp - 4`, the locals from `E` = `(ebp &
/// !7) - 0x24`; `magic_0155` 0x10007d70, 706 bytes, and `magic_0164`
/// 0x10007df0, 707 bytes; family `F0155_10007d70`; reached through the
/// 12-byte thunks [`timed_items_visit_thunk`]):
/// - the children visited once per pass ([`visit_children`]; nothing more
///   when the node was marked already); `S = *C`, `n = S + COUNT`;
/// - with bit 1 of the flags: each item's vector copied, slot 325
///   `(S + ITEM2 + STRIDE * i, S + 0x2c + 0x10 * i)`;
/// - with bit 2, for each item `R = S + ITEM + STRIDE * i` without bit 0
///   of the halfword `R + 0x26e`: its time `R + 0x268 += S + TIME`; slot
///   262 `(R + 0x54, &a, &b, &c)` (`E + 0x10`, `+ 0x14`, `+ 0x18`), `b`
///   = slot 12 `(R + 0x54)` (ST0 as f32), `a` = `R + 0x230`; then with
///   the i16 `R + 0x26c` below 1, `b -= slot 216 (R + 0x50) * K1`; else
///   in `0155` `b += R + 0x274` unless slot 534 `(R + 0x50)` is 0 and
///   slot 390 `(R + 0x50)` is not 0x1079; in `0164` `b +=` slot 664
///   `(R + 0x50)` when slot 534 is not 0;
/// - with `t = R + 0x268` at 0 or above and `t + a` at `b` or above: once
///   `R + 0x26c` is 1 or more, `R + 0x230 = b` and bit 0 of `R + 0x26e`
///   set; else `R + 0x230 = b - (b - R + 0x230) / t * t`, `R + 0x268 =
///   t * K2`, the halfword `R + 0x26c` up by one and, reaching 1, the time
///   left ([`crate::m0042::time_left`]) of `(R, R + 0x200)` and slot 239
///   `(*G_X, R + 0x50, 0)`; otherwise (`t` below 0 or `t + a` below `b`)
///   `R + 0x204 += R + 0x270` while `R + 0x26c` is 0 or below, and `R +
///   0x230 += t`.
///
/// Returns `eax` as the original leaves it; on the first way only its low
/// byte is known (the byte compared; the rest is the /GS cookie's).
#[allow(clippy::too_many_lines)] // the original's branches, one by one
pub fn timed_items_visit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // `cmp word [esi + 0x26c], 1; push [esi + 0x50]; jge`: the 0164 copy
    // has `movss [esp + 0x20], xmm1` after the first way's `subss` at +0x16f
    let second = img.word(f + 0x16f) as u8 == 0xf3;
    let ops = if second {
        TimedOps {
            flags: 0x1640,
            count: 0x162c,
            vec: 0x2e8,
            item: 0xbc,
            stride: 0x274,
            time: 0x1630,
            table1: 0x82,
            table2: 0xfd,
            k1: 0x15e,
            k2: 0x21b,
            gx: 0x269,
            left: 0x253,
        }
    } else {
        TimedOps {
            flags: 0x1650,
            count: 0x163c,
            vec: 0x2d8,
            item: 0xac,
            stride: 0x278,
            time: 0x1640,
            table1: 0x82,
            table2: 0xfd,
            k1: 0x15e,
            k2: 0x21a,
            gx: 0x268,
            left: 0x252,
        }
    };
    let c = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x24);
    let (a, b, cc) = (
        e.wrapping_add(0x10),
        e.wrapping_add(0x14),
        e.wrapping_add(0x18),
    );
    let node = g.read::<u32>(c);
    let Some(last) = visit_children(g, c) else {
        return Ret::eax(u32::from(g.read::<u8>(node.wrapping_add(0x1a))));
    };
    // `mov al, [esi + 5]`: the rest of eax is the last visit's (or the /GS
    // cookie's without children)
    let mut eax = last & !0xff | u32::from(g.read::<u8>(c.wrapping_add(5)));
    let s = g.read::<u32>(c);
    let count = |g: &G| g.read::<u32>(s.wrapping_add(ops.count)).cast_signed();
    let slot = |g: &G, k: u32| slot_target(g, w(ops.table1), k);
    let flags = |g: &G| g.read::<u8>(s.wrapping_add(ops.flags));
    if flags(g) & 2 != 0 && count(g) > 0 {
        let mut i = 0i32;
        let mut vec = s.wrapping_add(0x2c);
        let mut item = s.wrapping_add(ops.vec);
        loop {
            let t = slot(g, 0x514 / 4);
            // call width: 2
            g.call(t, c, &[item, vec]);
            i += 1;
            vec = vec.wrapping_add(0x10);
            eax = vec;
            item = item.wrapping_add(ops.stride);
            if i >= count(g) {
                break;
            }
        }
    }
    if flags(g) & 4 == 0 || count(g) <= 0 {
        return Ret::eax(eax);
    }
    let slot2 = |g: &G, k: u32| slot_target(g, w(ops.table2), k);
    let mut i = 0i32;
    let mut r = s.wrapping_add(ops.item);
    loop {
        let at = |o: u32| r.wrapping_add(o);
        if g.read::<u16>(at(0x26e)) & 1 == 0 {
            let t0 = add(
                g.read::<f32>(s.wrapping_add(ops.time)),
                g.read::<f32>(at(0x268)),
            );
            g.write::<f32>(at(0x268), t0);
            let key = g.read::<u32>(at(0x54));
            let t = slot2(g, 0x418 / 4);
            // call width: 4
            g.call(t, c, &[key, a, b, cc]);
            let key = g.read::<u32>(at(0x54));
            let t = slot2(g, 12);
            // call width: 5 (the earlier words under it)
            let out = g.call(t, c, &[key, key, a, b, cc]);
            g.write::<f32>(b, crate::ob_model::st0_f32(out.st0));
            let x230 = g.read::<u32>(at(0x230));
            g.write::<u32>(a, x230);
            let key = g.read::<u32>(at(0x50));
            let k1 = g.read::<f32>(w(ops.k1));
            let steps = g.read::<u16>(at(0x26c)).cast_signed();
            if steps < 1 {
                let t = slot2(g, 0x360 / 4);
                // call width: 1
                let out = g.call(t, c, &[key]);
                eax = out.eax;
                let y = mul(crate::ob_model::st0_f32(out.st0), k1);
                g.write::<f32>(b, sub(g.read::<f32>(b), y));
            } else {
                let t = slot2(g, 0x858 / 4);
                // call width: 1
                eax = g.call(t, c, &[key]).eax;
                if second {
                    if eax != 0 {
                        let key = g.read::<u32>(at(0x50));
                        let t = slot2(g, 0xa60 / 4);
                        // call width: 1
                        let out = g.call(t, c, &[key]);
                        eax = out.eax;
                        let y = add(g.read::<f32>(b), crate::ob_model::st0_f32(out.st0));
                        g.write::<f32>(b, y);
                    }
                } else {
                    let bump = eax != 0 || {
                        let key = g.read::<u32>(at(0x50));
                        let t = slot2(g, 0x618 / 4);
                        // call width: 1
                        eax = g.call(t, c, &[key]).eax;
                        eax == 0x1079
                    };
                    if bump {
                        let y = add(g.read::<f32>(at(0x274)), g.read::<f32>(b));
                        g.write::<f32>(b, y);
                    }
                }
            }
            let target = g.read::<f32>(b);
            let time = g.read::<f32>(at(0x268));
            // comiss, jb: below or unordered
            let below = |x: f32, y: f32| x < y || x.is_nan() || y.is_nan();
            if below(time, 0.0) || below(add(time, g.read::<f32>(a)), target) {
                if g.read::<u16>(at(0x26c)).cast_signed() <= 0 {
                    eax = g.read::<u32>(at(0x270));
                    let v = g.read::<u32>(at(0x204)).wrapping_add(eax);
                    g.write::<u32>(at(0x204), v);
                }
                let v = add(time, g.read::<f32>(at(0x230)));
                g.write::<f32>(at(0x230), v);
            } else if g.read::<u16>(at(0x26c)).cast_signed() >= 1 {
                g.write::<f32>(at(0x230), target);
                let v = g.read::<u16>(at(0x26e)) | 1;
                g.write::<u16>(at(0x26e), v);
            } else {
                let back = mul(div(sub(target, g.read::<f32>(at(0x230))), time), time);
                let x = sub(target, back);
                let t2 = mul(time, g.read::<f32>(w(ops.k2)));
                g.write::<f32>(at(0x230), x);
                let steps = g.read::<u16>(at(0x26c)).wrapping_add(1);
                g.write::<u16>(at(0x26c), steps);
                g.write::<f32>(at(0x268), t2);
                if steps.cast_signed() >= 1 {
                    let pair = e.wrapping_add(8);
                    g.write::<u32>(pair, r);
                    g.write::<u32>(pair.wrapping_add(4), at(0x200));
                    crate::m0042::time_left(
                        g,
                        img,
                        img.rel32_target(f + ops.left),
                        pair,
                        e.wrapping_sub(0x10),
                    );
                    let key = g.read::<u32>(at(0x50));
                    let gx = g.read::<u32>(w(ops.gx));
                    let t = slot2(g, 0x3bc / 4);
                    // call width: 3
                    eax = g.call(t, c, &[gx, key, 0]).eax;
                }
            }
        }
        i += 1;
        r = r.wrapping_add(ops.stride);
        if i >= count(g) {
            break;
        }
    }
    Ret::eax(eax)
}

/// The thunks of [`timed_items_visit`] (`magic_0155` 0x10008050,
/// `magic_0164` 0x100080d0).
pub fn timed_items_visit_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::o6_fam_pairs3::thunk_to(g, img, fr, timed_items_visit::<G>)
}

/// A texture scroll step: `((x % (size << 16)) >> 4) / size` with `idiv`
/// (the remainder's sign follows `x`) and `sar`.
fn tex_scroll(x: u32, size: u32) -> u32 {
    let r = x.cast_signed().wrapping_rem((size << 16).cast_signed()) >> 4;
    r.wrapping_div(size.cast_signed()).cast_unsigned()
}

/// A sprite node's visit drawing a scrolled texture (internal, `ecx` =
/// the cursor `C`; an aligned /GS frame: `ebp` = the entry `esp - 4`,
/// the locals from `E` = `(ebp & !0xf) - 0x208`; `magic_0578`
/// 0x100030d0, 749 bytes, and `magic_0585` 0x10003110, 754 bytes; family
/// `F0578_100030d0`; reached through the 12-byte thunk
/// [`sprite_visit_thunk`]):
/// - the children visited once per pass ([`visit_children`]; nothing more
///   when the node was marked already); `S = *C`;
/// - the delay, the u16 `S + 0xaa`, counts down; above 0 before, that is
///   all; else when the byte `S + 0x7b` is set: slot 231 `(M, I, *(S +
///   0x6c) + 0x9c)` (`M` = `E + 0xc0`, `I` an immediate); the effect word
///   `FX = *(*(*(S + 0x68) + 0x54) + 0x830)`;
///   from the GS `TEX0` word at `*(S + 0x70)` slot 885 `(TBP0, PSM, 0, 0,
///   1 << TW, 1 << TH, M2)` (`M2` = `E + 0x100`) with the game lock
///   `*G_LOCK` 1 around it; when that is not 0, the colour `S + 0x78`'s
///   bytes times `K` make `Q` (`E + 0xb0`) and, with the lock 1, slot 879
///   `(FX, M2, v, u, Q, Q.w, P, W, H, 0)` (`P` = `S + 0x1c` in `0578`,
///   `*(S + 0x6c) + 0x9c` in `0585`; `u` from `S + 0x9c` and `H`, `v`
///   from `S + 0x98` and `W`, [`tex_scroll`] times `KD`); the lock gets
///   its old value;
/// - then (unless the delay ran) with bit 4 of `S + 0xa4` the node leaves
///   its parent ([`crate::sa_misc::leave`]).
///
/// Returns `eax` as the original leaves it; on the first way only its
/// low byte is known (the byte compared; the rest is the /GS cookie's).
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn sprite_visit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, t: u32, d: u32| slot_target(g, w(t), w(d) / 4);
    // `sub esp, 0xc` (0578) or `mov ecx, [esp + 0x34]` (0585) at +0x1ad
    let first = img.word(f + 0x1ad) as u8 == 0x83;
    // (KD twice, slot 879's table and displacement, `G_LOCK`, the lock's
    // 1)
    let ops = if first {
        [0x278u32, 0x29c, 0x2a7, 0x2b1, 0x2b8, 0x1b2]
    } else {
        [0x27d, 0x2a1, 0x2ac, 0x2b6, 0x2bd, 0x231]
    };
    let c = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !0xf).wrapping_sub(0x208);
    let node = g.read::<u32>(c);
    g.write::<u32>(e.wrapping_add(4), node);
    if visit_children(g, c).is_none() {
        return Ret::eax(u32::from(g.read::<u8>(node.wrapping_add(0x1a))));
    }
    let s = g.read::<u32>(c);
    g.write::<u32>(e.wrapping_add(8), s);
    let field = |g: &G, o: u32| g.read::<u32>(s.wrapping_add(o));
    let delay = g.read::<u16>(s.wrapping_add(0xaa));
    let mut eax = u32::from(delay).wrapping_sub(1);
    g.write::<u16>(s.wrapping_add(0xaa), eax as u16);
    if delay.cast_signed() > 0 {
        return Ret::eax(eax);
    }
    if g.read::<u8>(s.wrapping_add(0x7b)) != 0 {
        let (v_src, u_src, colour) = (field(g, 0x98), field(g, 0x9c), field(g, 0x78));
        let (m, m2, q) = (
            e.wrapping_add(0xc0),
            e.wrapping_add(0x100),
            e.wrapping_add(0xb0),
        );
        let mut st = Pushes::default();
        st.push_all(&[field(g, 0x6c).wrapping_add(w(0xbd)), w(0xca), m]);
        let t = slot(g, 0xd0, 0xd6);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, c);
        let owner = g.read::<u32>(g.read::<u32>(c).wrapping_add(0x68));
        let fx = g.read::<u32>(
            g.read::<u32>(owner.wrapping_add(0x54))
                .wrapping_add(w(0xeb)),
        );
        let tex = field(g, 0x70);
        let v = u64::from(g.read::<u32>(tex)) | u64::from(g.read::<u32>(tex.wrapping_add(4))) << 32;
        // the shifts keep 16 bits (`shl ax, cl`)
        let tw = u32::from(1u16 << ((v >> 26) & 0xf));
        let th = u32::from(1u16 << ((v >> 30) & 0xf));
        let lock = g.read::<u32>(w(0x126));
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, w(0x137));
        let psm = ((v >> 20) & 0x3f) as u32;
        let tbp = (v & 0x3fff) as u32;
        st.push_all(&[m2, th, tw, 0, 0, psm, tbp]);
        let t = slot(g, 0x15b, 0x167);
        // call width: 12 (Pushes caps the words at 12)
        eax = st.call(g, t, c).eax;
        let lock = g.read::<u32>(w(0x17b));
        g.write::<u32>(lock, saved);
        if eax != 0 {
            let kc = k(g, 0x198);
            let byte = |i: u32| cvt(colour >> (8 * i) & 0xff);
            let channels = [mul(byte(0), kc), mul(byte(1), kc), mul(byte(2), kc)];
            let alpha = mul(byte(3), kc);
            let part = if first {
                s.wrapping_add(0x1c)
            } else {
                field(g, 0x6c).wrapping_add(w(0x214))
            };
            g.write::<u32>(lock, w(ops[5]));
            for (i, x) in (0u32..).zip(channels) {
                g.write::<f32>(q.wrapping_add(4 * i), x);
            }
            g.write::<f32>(q.wrapping_add(0xc), alpha);
            let u = mul(cvt(tex_scroll(u_src, th)), k(g, ops[0]));
            let vv = mul(cvt(tex_scroll(v_src, tw)), k(g, ops[1]));
            let mut st = Pushes::default();
            st.push_all(&[
                0,
                cvt(th).to_bits(),
                cvt(tw).to_bits(),
                part,
                alpha.to_bits(),
                q,
                u.to_bits(),
                vv.to_bits(),
                m2,
                fx,
            ]);
            let t = slot(g, ops[2], ops[3]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, c);
            eax = g.read::<u32>(w(ops[4]));
            g.write::<u32>(eax, saved);
        }
    }
    if g.read::<u8>(s.wrapping_add(0xa4)) & 0x10 != 0 {
        // `call leave` with `ecx` = S
        eax = crate::sa_misc::leave(g, s, eax);
    }
    Ret::eax(eax)
}

/// The thunk of [`sprite_visit`] (`magic_0578` 0x100033c0).
pub fn sprite_visit_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::o6_fam_pairs3::thunk_to(g, img, fr, sprite_visit::<G>)
}

/// A light's set-up for the frame (internal: `ecx` = the angles `A`,
/// `edx` = the position `P`, one stack word `n`; an aligned /GS frame:
/// `ebp` = the entry `esp - 4`, the locals from `E` = `(ebp & !7) -
/// 0x190`; `magic_0644` 0x1000b390 and `magic_0565` 0x100074f0, 876
/// bytes; family `F0644_1000b390`):
/// - slot 562 `()`; `h` = slot 219 `(*G_H, 0)`; the matrix `M0` (`E + 8`)
///   from slot 235; `a = slot 277 (h) * K1` and the diagonal `M0[0]`,
///   `M0[5]`, `M0[10]` = `a * slot 15 (h)`, `a * slot 16 (h)`, `a * slot
///   17 (h)`, each times `K2` (the slots' ST0 as f32);
/// - `M0` turned by slots 330, 328, 329 `(M0, M0, A.z / A.x / A.y)`, its
///   last row `P`;
/// - while the count `*G_N` is below 0x15: slot 230 `(T + 0x40 * N, M0)`
///   and the f32 `T2[N]` = `n * K3`;
/// - slot 723 `(M1)`, 724 `(M2)`, 231 `(M3, M2, M0)`, 231 `(M3, M1, M3)`
///   (`M1` = `E + 0x88`, `M2` = `E + 0xc8`, `M3` = `E + 0x48`); the
///   scaled camera ([`crate::m0052::scaled_camera`], `ecx` = `E + 0x108`);
///   slot 230 `(E + 0x148, slot 545 ())`;
/// - slots 357 `(h, -1, 0x3c00)`, 356 `(h, -1, 0x2ee0)`, 355 `(h, -1)`,
///   354 `(h, -1)`; with `*G_N` 0 slot 847 `(*(h + 0x830), CB)`; below
///   0x15 slot 931 `(*(h + 0x830), 0)` and `*G_N` up by one; slots 357
///   and 356 `(h, -1, -1)`.
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// the last slot 356's `eax`.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn light_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{cvt, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let (angles, pos, n) = (fr.ecx, fr.edx, fr.arg(g, 0));
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x190);
    let at = |o: u32| e.wrapping_add(o);
    let m0 = at(8);
    let mut st = Pushes::default();
    let t = slot(g, 562);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, angles);
    st.push(0);
    st.push(g.read::<u32>(w(0x36)));
    let t = slot(g, 219);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t, angles).eax;
    st.push(m0);
    let t = slot(g, 235);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, angles);
    let float = |g: &mut G, st: &mut Pushes, n: u32| {
        st.push(h);
        let t = slot(g, n);
        // call width: 12 (Pushes caps the words at 12)
        st0_f32(st.call(g, t, angles).st0)
    };
    let a = mul(float(g, &mut st, 277), k(g, 0x79));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(at(8 + o), a);
    }
    let t1 = float(g, &mut st, 15);
    let b0 = mul(g.read::<f32>(at(8)), t1);
    g.write::<f32>(at(8), b0);
    let t2 = float(g, &mut st, 16);
    let b1 = mul(g.read::<f32>(at(0x1c)), t2);
    g.write::<f32>(at(0x1c), b1);
    let t3 = float(g, &mut st, 17);
    let k2 = k(g, 0xf3);
    let c2 = mul(g.read::<f32>(at(0x30)), t3);
    g.write::<f32>(at(8), mul(g.read::<f32>(at(8)), k2));
    g.write::<f32>(at(0x1c), mul(g.read::<f32>(at(0x1c)), k2));
    g.write::<f32>(at(0x30), mul(c2, k2));
    // add esp, 0x18: the `push 0` is left, the angles take its place
    st.pop(6);
    for (i, s) in [(2u32, 330u32), (0, 328), (1, 329)] {
        if s != 330 {
            st.pop(2);
        }
        st.set_top(g.read::<u32>(angles.wrapping_add(4 * i)));
        st.push_all(&[m0, m0]);
        let t = slot(g, s);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, angles);
    }
    for i in 0..4u32 {
        let v = g.read::<u32>(pos.wrapping_add(4 * i));
        g.write::<u32>(at(0x38 + 4 * i), v);
    }
    let limit = disp8(img, f + 0x1bb).cast_signed();
    let count = g.read::<u32>(w(0x18b));
    let mut st = Pushes::default();
    if count.cast_signed() < limit {
        let row = (count << 6).wrapping_add(w(0x1c6));
        let t = slot(g, 230);
        // call width: 2
        g.call(t, angles, &[row, m0]);
        let x = mul(cvt(n), k(g, 0x1ed));
        let i = g.read::<u32>(w(0x1df));
        g.write::<f32>(w(0x1f6).wrapping_add(i << 2), x);
    }
    let (m1, m2, m3) = (at(0x88), at(0xc8), at(0x48));
    for (words, s) in [
        (&[m1][..], 723u32),
        (&[m2][..], 724),
        (&[m0, m2, m3][..], 231),
        (&[m3, m1, m3][..], 231),
    ] {
        st.push_all(words);
        let t = slot(g, s);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, angles);
    }
    // ten words pushed: `esp` = E - 0x28 at the call
    crate::m0052::scaled_camera(
        g,
        img,
        img.rel32_target(f + 0x26a),
        at(0x108),
        e.wrapping_sub(0x2c),
    );
    let t = slot(g, 545);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t, angles).eax;
    st.push_all(&[r, at(0x148)]);
    let t = slot(g, 230);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, angles);
    let calls = [(357u32, w(0x2a1)), (356, w(0x2b0))];
    for (s, x) in calls {
        st.push_all(&[x, u32::MAX, h]);
        let t = slot(g, s);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, angles);
    }
    let mut st = Pushes::default();
    for s in [355u32, 354] {
        st.push_all(&[u32::MAX, h]);
        let t = slot(g, s);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, angles);
    }
    let fx = |g: &G| g.read::<u32>(h.wrapping_add(0x830));
    if g.read::<u32>(w(0x2e3)) == 0 {
        let t = slot(g, 847);
        // call width: 2
        g.call(t, angles, &[fx(g), w(0x2f4)]);
    }
    if g.read::<u32>(w(0x307)).cast_signed() < disp8(img, f + 0x310).cast_signed() {
        let t = slot(g, 931);
        // call width: 2
        g.call(t, angles, &[fx(g), 0]);
        let v = g.read::<u32>(w(0x32d)).wrapping_add(1);
        g.write::<u32>(w(0x32d), v);
    }
    let mut st = Pushes::default();
    let mut eax = 0;
    for s in [357u32, 356] {
        st.push_all(&[u32::MAX, u32::MAX, h]);
        let t = slot(g, s);
        // call width: 12 (Pushes caps the words at 12)
        eax = st.call(g, t, angles).eax;
    }
    Ret::eax(eax)
}

/// `x` wrapped once into `[LO, HI]` by a turn (the `comiss`/`jbe` pair:
/// `x - turn` above `HI`, `x + turn` below `LO`; NaN stays).
pub(crate) fn wrap_turn(x: f32, hi: f32, lo: f32, turn: f32) -> f32 {
    if x > hi {
        crate::sb_sse::sub(x, turn)
    } else if lo > x {
        crate::sb_sse::add(x, turn)
    } else {
        x
    }
}

/// A camera box from an object's turn (no arguments; a /GS frame of
/// 0x124 bytes, `ebp` = `esp - 4` at the entry; `magic_0307` 0x10001d20
/// and `magic_0605` 0x100076a0, 752 bytes; family `F0307_10001d20`). With
/// `O = *G_O` and its turn `r` (the f32 at `+0x2d`'s displacement) and
/// point `P` (`+0x7e`.., copied to `ebp - 0x14`):
/// - the matrix `M` (`ebp - 0xe4`): slot 235, slot 328 `(M, M, 0)`, slot
///   329 `(M, M, y)` with `y = r - KA` wrapped by a turn into `[KC, KB]`
///   (`ebp - 0x40`); slot 229 `(N, M, V)` (`N` = `ebp - 0xa4`, `V` =
///   `ebp - 0x54` the copy's vector, `w` 1.0); slot 248 `(P, P, N)`, `P.w`
///   = 1.0;
/// - the matrix `Q` (`ebp - 0x94`): slot 235, slot 554 `(Q, Q, &Z)` with
///   `Z` = `(0, r - KD wrapped, 0, 0)` at `ebp - 0x44`; `ebp - 0x64` =
///   `P * KS` (`w` = `P.w`); slot 327 `(S, Q)` (`S` = `ebp - 0x124`);
/// - the box corners `B` (`ebp - 0x24`) and `A` (`ebp - 0x34`) from the
///   copy's immediates, slot 858 `(1, S, B, A, 0x3900)`; `B` and `A` into
///   `O + 0xcc0..` (`+0x282`'s displacement on).
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// slot 858's `eax`.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn turn_box<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0xa4), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let (m, n, v, p, z) = (at(0xe4), at(0xa4), at(0x54), at(0x14), at(0x44));
    let (q, s, b, a) = (at(0x94), at(0x124), at(0x24), at(0x34));
    let obj = g.read::<u32>(w(0x1e));
    let ox = |o: u32| obj.wrapping_add(w(o));
    for (i, o) in [0x25u32, 0x3c, 0x43, 0x4a].into_iter().enumerate() {
        g.write::<u32>(v.wrapping_add(4 * i as u32), w(o));
    }
    g.write::<u32>(z, w(0x51));
    let turn = k(g, 0x17);
    let y = sub(g.read::<f32>(ox(0x2d)), k(g, 0x35));
    let y = wrap_turn(y, k(g, 0x58), k(g, 0x68), turn);
    g.write::<f32>(z.wrapping_add(4), y);
    for (i, o) in [0x7eu32, 0x8b, 0x98, 0xb1].into_iter().enumerate() {
        let x = g.read::<u32>(ox(o));
        g.write::<u32>(p.wrapping_add(4 * i as u32), x);
    }
    g.write::<u32>(z.wrapping_add(8), w(0xbe));
    g.write::<u32>(z.wrapping_add(0xc), w(0xc5));
    let mut st = Pushes::default();
    st.push(m);
    let t = slot(g, 235);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    for (top, s) in [(z, 328u32), (z.wrapping_add(4), 329)] {
        if s == 329 {
            st.pop(2);
        }
        st.set_top(g.read::<u32>(top));
        st.push_all(&[m, m]);
        let t = slot(g, s);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, fr.ecx);
    }
    st.push_all(&[v, m, n]);
    g.write::<u32>(v.wrapping_add(0xc), w(0x12b));
    let t = slot(g, 229);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    st.push_all(&[n, p, p]);
    let t = slot(g, 248);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    let y2 = sub(g.read::<f32>(ox(0x154)), k(g, 0x15c));
    g.write::<u32>(p.wrapping_add(0xc), w(0x166));
    let y2 = wrap_turn(y2, k(g, 0x16d), k(g, 0x181), k(g, 0x177));
    let mut st = Pushes::default();
    st.push(q);
    g.write::<f32>(z.wrapping_add(4), y2);
    let t = slot(g, 235);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    st.push_all(&[z, q, q]);
    let t = slot(g, 554);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    let ks = k(g, 0x1c8);
    let r = at(0x64);
    for i in 0..3u32 {
        let x = mul(g.read::<f32>(p.wrapping_add(4 * i)), ks);
        g.write::<f32>(r.wrapping_add(4 * i), x);
    }
    let pw = g.read::<u32>(p.wrapping_add(0xc));
    g.write::<u32>(r.wrapping_add(0xc), pw);
    st.push_all(&[q, s]);
    let t = slot(g, 327);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    for (corner, ops) in [
        (b, [0x237u32, 0x23e, 0x245, 0x24c]),
        (a, [0x253, 0x25a, 0x261, 0x268]),
    ] {
        for (i, o) in (0u32..).zip(ops) {
            g.write::<u32>(corner.wrapping_add(4 * i), w(o));
        }
    }
    st.push_all(&[w(0x21c), a, b, s, disp8(img, f + 0x273)]);
    let t = slot(g, 858);
    // call width: 12 (Pushes caps the words at 12)
    let eax = st.call(g, t, fr.ecx).eax;
    let outs = [0x282u32, 0x28f, 0x29c, 0x2a9, 0x2b6, 0x2c3, 0x2d0, 0x2e0];
    for (i, o) in (0u32..).zip(outs) {
        let src = if i < 4 { b } else { a };
        let x = g.read::<u32>(src.wrapping_add(4 * (i % 4)));
        g.write::<u32>(ox(o), x);
    }
    Ret::eax(eax)
}

/// A burst of 25 x 8 particles around an object's point (no arguments; a
/// /GS frame of 0xb0 bytes, `ebp` = `esp - 4` at the entry; `magic_0304`
/// 0x10002ae0 and `magic_0568` 0x10002510, 770 bytes; family
/// `F0304_10002ae0`). With `A = *G_A`: the matrix `L` (`ebp - 0xa4`) from
/// slot 235; `V` (`ebp - 0x14`) = `(0, 0, -, 1)`. Then for each of 25
/// rows `i`, up to 8 particles from the pool
/// ([`crate::sb_misc::pool_take`]; none left ends the row): the halfword
/// `+0x38` = 0xffff, `+0x30` = `*(*G_R + 8)`, `+0x34` = 1, the halfword
/// `+0x3a` = 0x200, `+0..` = `A`'s point (`+0x9a`'s displacement on);
/// `+4 += slot 314 (20.0) - K1`; `R` (`ebp - 0x64`) = slot 329 `(R, L,
/// slot 314 (pi))`; `V.z` = slot 314 `(25.0)`; `W` (`ebp - 0x24`) = slot
/// 229 `(W, R, V)`; `+0 += W.x`, `+8 = W.z + +8`; `+0x10 = W.x * K2 +
/// slot 314 (0.02)`; `+0x14 = -slot 404 (2.0)`; `+0x18 = W.z * K2 + slot
/// 314 (0.02)`; `+0x1c` = slot 314 `(pi)`; `+0x20` 0; `+0x24 = slot 314
/// (0.01) + K3`; `+0x28` 0, `+0x2c` 1.0; `+0x3c = (slot 314 (0.2) + K4) *
/// K5`; `+0x40` 0, `+0x58` and `+0x4c` 0x8c, `+0x5c` and `+0x50` 0,
/// `+0x60` = `i` (the slots' ST0 as f32; the immediates the copy's).
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// the last row's particle count, or 0 when the pool ran out in it.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn particle_rows<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, d: u32| slot_target(g, w(0x28), d / 4);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let (l, r, v, wv) = (at(0xa4), at(0x64), at(0x14), at(0x24));
    let a = g.read::<u32>(w(0x14));
    let t = slot(g, 0x3ac);
    // call width: 1
    g.call(t, fr.ecx, &[l]);
    for (o, x) in [(0u32, 0x3au32), (4, 0x41), (0xc, 0x48)] {
        g.write::<u32>(v.wrapping_add(o), w(x));
    }
    let take = img.rel32_target(f + 0x61);
    let mut eax = 0;
    let rows = disp8(img, f + 0x2eb).cast_signed();
    let per_row = disp8(img, f + 0x2e1).cast_signed();
    let mut i = 0i32;
    while i < rows {
        let mut j = 0i32;
        while j < per_row {
            let p = crate::sb_misc::pool_take(g, img, take);
            if p == 0 {
                eax = 0;
                break;
            }
            let pf = |o: u32| p.wrapping_add(o);
            let owner = g.read::<u32>(g.read::<u32>(w(0x71)).wrapping_add(disp8(img, f + 0x7a)));
            g.write::<u16>(pf(0x38), 0xffff);
            g.write::<u32>(pf(0x30), owner);
            g.write::<u32>(pf(0x34), w(0x8b));
            g.write::<u16>(pf(0x3a), w(0x90) as u16);
            for (n, o) in (0u32..).zip([0x9au32, 0xa2, 0xab, 0xb4]) {
                let x = g.read::<u32>(a.wrapping_add(w(o)));
                g.write::<u32>(pf(4 * n), x);
            }
            // each slot 314 call writes its float over the word on top
            let mut st = Pushes::default();
            st.push(a);
            let st0 = |g: &mut G, st: &mut Pushes, d: u32, x: u32| {
                st.set_top(x);
                let t = slot(g, d);
                // call width: 12 (Pushes caps the words at 12)
                st0_f32(st.call(g, t, fr.ecx).st0)
            };
            let t1 = st0(g, &mut st, 0x4e8, w(0xca));
            let y = add(sub(t1, k(g, 0xe2)), g.read::<f32>(pf(4)));
            g.write::<f32>(pf(4), y);
            let angle = st0(g, &mut st, 0x4e8, w(0xee));
            st.set_top(angle.to_bits());
            st.push_all(&[l, r]);
            let t = slot(g, 0x524);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, fr.ecx);
            st.pop(2);
            let z = st0(g, &mut st, 0x4e8, w(0x140));
            g.write::<f32>(v.wrapping_add(8), z);
            st.push_all(&[v, r, wv]);
            let t = slot(g, 0x394);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, fr.ecx);
            st.pop(3);
            let x0 = add(g.read::<f32>(pf(0)), g.read::<f32>(wv));
            g.write::<f32>(pf(0), x0);
            let x2 = add(g.read::<f32>(wv.wrapping_add(8)), g.read::<f32>(pf(8)));
            g.write::<f32>(pf(8), x2);
            let t = st0(g, &mut st, 0x4e8, w(0x176));
            let x10 = add(mul(g.read::<f32>(wv), k(g, 0x1a4)), t);
            g.write::<f32>(pf(0x10), x10);
            let t = st0(g, &mut st, 0x650, w(0x1b0));
            let mask = g.read::<u32>(w(0x1da));
            g.write::<u32>(pf(0x14), t.to_bits() ^ mask);
            let t = st0(g, &mut st, 0x4e8, w(0x1e6));
            let x18 = add(mul(g.read::<f32>(wv.wrapping_add(8)), k(g, 0x206)), t);
            g.write::<f32>(pf(0x18), x18);
            let t = st0(g, &mut st, 0x4e8, w(0x212));
            g.write::<f32>(pf(0x1c), t);
            g.write::<u32>(pf(0x20), w(0x236));
            let t = st0(g, &mut st, 0x4e8, w(0x243));
            g.write::<u32>(pf(0x28), w(0x267));
            g.write::<u32>(pf(0x2c), w(0x26e));
            g.write::<f32>(pf(0x24), add(t, k(g, 0x25b)));
            let t = st0(g, &mut st, 0x4e8, w(0x275));
            let x = mul(add(t, k(g, 0x298)), k(g, 0x2b1));
            j += 1;
            eax = j.cast_unsigned();
            for (o, x) in [(0x40u32, 0x2a9u32), (0x58, 0x2b8), (0x5c, 0x2bf)] {
                g.write::<u32>(pf(o), w(x));
            }
            g.write::<f32>(pf(0x3c), x);
            for (o, x) in [(0x4cu32, 0x2cbu32), (0x50, 0x2d2)] {
                g.write::<u32>(pf(o), w(x));
            }
            g.write::<u32>(pf(0x60), i.cast_unsigned());
        }
        i += 1;
    }
    Ret::eax(eax)
}

/// Writes `words` as the stack arguments of a call whose callee's `esp`
/// (its return address) is `esp`, and gives the callee's frame.
pub(crate) fn stack_args<G: Game>(
    g: &mut G,
    at: u32,
    esp: u32,
    ecx: u32,
    edx: u32,
    words: &[u32],
) -> Frame {
    for (i, &v) in (0u32..).zip(words) {
        g.write::<u32>(esp.wrapping_add(4 + 4 * i), v);
    }
    Frame {
        at,
        esp,
        ecx,
        edx,
        ..Frame::default()
    }
}

/// An effect task starting ten rings of nodes, one per pass (`(task)`
/// with the effect `E = task[+4]`; an aligned frame: `ebp` = the entry
/// `esp - 4`, `S0` = `(ebp & !7) - 0x10` after the saved registers;
/// `magic_0150` 0x1000b210 and `magic_0603` 0x1000b140, 900 bytes;
/// family `F0150_1000b210`). Nothing while `*G_SKIP` is set (`eax` as
/// given). With the ring index `i = E + COUNT`:
/// - with `i` above 0 the f32 `E + 0xe64 -= K0` (below 0: 0); unless the
///   word `E + WAIT` is 1, that is all (`eax` = it minus 1); `E + T +=
///   K1` (`T` = `+0x63`'s displacement); below `E + LIMIT` (or NaN) that
///   is all (`eax` = 0);
/// - from `i` = 5 on, slot 239 `(*G_X, -5, 0)`;
/// - when the f32 `E + 0x468 + 0x10 i` is above `K2`: the nodes `N1 = E +
///   0x2c (i + 0xe4)` and `N2 = E + 0xe80 + 0x2c i` appended to the list
///   `L` ([`crate::sb_misc::node_append`]) with slot 325 `(N1 + 0x1c,
///   E + 0x45c + 0x10 i)` between and `N1 + 0x28` = 1.0, `N2 +
///   0x1c..0x24` = 1.0; with `*G_MODE` 1 `N2 + 0x28 = (x - K3) * K6 +
///   K7`, else `N1 + 0x20 -= K3` and `N2 + 0x28 = (x - K3) * K4 + K5`
///   (`x` = slot 307 `()`); the owned node `O = E + 0x1038 + 0x14c i`
///   ([`crate::sa_misc::node_append_owned`] `(owner, N1, B, N2)` into
///   `L`) and the particle node `E + 0x1d30 + 0x100 i`
///   ([`crate::s3_nodes::particle_node_with`] `(O, _, _, 1 or 4)`);
/// - with `i` 0 the two effect nodes `X1`, `X2`
///   ([`crate::s3_nodes::effect_node_with`] `(owner, P, _, 3 / 4)`), each
///   then with bit 0 of its word `+0x2c` set and `+0x30` 0;
/// - `i` up by one; at 10 the task leaves its list (`task[0]`'s links
///   joined, its own pointing at itself); `E + LIMIT += K5`.
///
/// Returns `eax` as the original leaves it (the last call's or the
/// task's previous link).
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn node_rings<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, t: u32, d: u32| slot_target(g, w(t), w(d) / 4);
    let task = fr.arg(g, 0);
    let e = g.read::<u32>(task.wrapping_add(4));
    if g.read::<u32>(w(0x0b)) != 0 {
        return Ret::eax(fr.eax);
    }
    let ef = |o: u32| e.wrapping_add(o);
    let count_at = ef(w(0x20));
    let i = g.read::<u32>(count_at);
    if i != 0 {
        let x = sub(g.read::<f32>(ef(0xe64)), k(g, 0x34));
        g.write::<f32>(ef(0xe64), x);
        if 0.0 > x {
            g.write::<u32>(ef(0xe64), w(0x4e));
        }
    }
    let wait = g.read::<u32>(ef(w(0x54))).wrapping_sub(1);
    if wait != 0 {
        return Ret::eax(wait);
    }
    let t = add(g.read::<f32>(ef(w(0x63))), k(g, 0x6b));
    g.write::<f32>(ef(w(0x63)), t);
    let limit = g.read::<f32>(ef(w(0x72)));
    if t < limit || t.is_nan() || limit.is_nan() {
        return Ret::eax(0);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let s0 = (ebp & !7).wrapping_sub(0x10);
    if i.cast_signed() >= disp8(img, f + 0x86).cast_signed() {
        let x = g.read::<u32>(w(0x93));
        let t = slot(g, 0x8b, 0x99);
        // call width: 3
        g.call(t, fr.ecx, &[x, disp8(img, f + 0x90), 0]);
    }
    let i = g.read::<u32>(count_at);
    let list = w(0xce);
    let mut eax = i.wrapping_add(i);
    if g.read::<f32>(ef(0x468).wrapping_add(i << 4)) > k(g, 0xb8) {
        let n1 = ef(i.wrapping_add(0xe4).wrapping_mul(0x2c));
        let append = img.rel32_target(f + 0xd3);
        crate::sb_misc::node_append(g, img, append, n1, list, eax);
        let i = g.read::<u32>(count_at);
        let words = [
            ef(0x274c).wrapping_add(i.wrapping_mul(0x2c)),
            ef(0x45c).wrapping_add(i << 4),
        ];
        let t = slot(g, 0xf7, 0xfd);
        // call width: 2
        g.call(t, fr.ecx, &words);
        let i = g.read::<u32>(count_at);
        g.write::<u32>(ef(0x2758).wrapping_add(i.wrapping_mul(0x2c)), w(0x119));
        let i = g.read::<u32>(count_at);
        let n2 = ef(0xe80).wrapping_add(i.wrapping_mul(0x2c));
        crate::sb_misc::node_append(g, img, append, n2, list, i.wrapping_mul(0x2c));
        let i = g.read::<u32>(count_at);
        g.write::<u32>(ef(i.wrapping_add(0x55).wrapping_mul(0x2c)), w(0x144));
        let i = g.read::<u32>(count_at);
        g.write::<u32>(ef(0xea0).wrapping_add(i.wrapping_mul(0x2c)), w(0x158));
        let i = g.read::<u32>(count_at);
        g.write::<u32>(ef(0xea4).wrapping_add(i.wrapping_mul(0x2c)), w(0x16c));
        // (slot 307's table and displacement, K3, the scale, the offset,
        // the owned call's operands: B, owner, the call; the particle
        // node's last word)
        let second = g.read::<u32>(w(0x171)).wrapping_sub(1) == 0;
        let ops = if second {
            [
                0x22cu32, 0x232, 0x246, 0x257, 0x264, 0x294, 0x2a2, 0x2a9, 0x2b1,
            ]
        } else {
            let i = g.read::<u32>(count_at);
            let at = ef(0x2750).wrapping_add(i.wrapping_mul(0x2c));
            let x = sub(g.read::<f32>(at), k(g, 0x192));
            g.write::<f32>(at, x);
            [
                0x1a0, 0x1a6, 0x1ba, 0x1cb, 0x1d8, 0x208, 0x216, 0x21d, 0x225,
            ]
        };
        let t = slot(g, ops[0], ops[1]);
        // call width: 0
        let out = g.call(t, fr.ecx, &[]);
        let x = crate::ob_model::st0_f32(out.st0);
        let y = add(mul(sub(x, k(g, ops[2])), k(g, ops[3])), k(g, ops[4]));
        let i = g.read::<u32>(count_at);
        g.write::<f32>(ef(0xea8).wrapping_add(i.wrapping_mul(0x2c)), y);
        let i = g.read::<u32>(count_at);
        let owned = ef(0x1038).wrapping_add(i.wrapping_mul(0x14c));
        let words = [
            w(ops[6]),
            ef(i.wrapping_add(0xe4).wrapping_mul(0x2c)),
            w(ops[5]),
            ef(0xe80).wrapping_add(i.wrapping_mul(0x2c)),
        ];
        let callee = stack_args(
            g,
            img.rel32_target(f + ops[7]),
            s0.wrapping_sub(0x14),
            owned,
            list,
            &words,
        );
        crate::sa_misc::node_append_owned(g, img, &callee);
        let i = g.read::<u32>(count_at);
        let node = ef(0x1d30).wrapping_add(i << 8);
        let rec = ef(0x1038).wrapping_add(i.wrapping_mul(0x14c));
        // push 1 or 4; sub esp, 8 (the owned call's words stay there);
        // push the record
        g.write::<u32>(s0.wrapping_sub(4), disp8(img, f + ops[8]));
        g.write::<u32>(s0.wrapping_sub(0x10), rec);
        let callee = Frame {
            at: img.rel32_target(f + 0x2d7),
            esp: s0.wrapping_sub(0x14),
            ecx: node,
            ..Frame::default()
        };
        eax = crate::s3_nodes::particle_node_with(g, img, &callee);
    }
    if g.read::<u32>(count_at) == 0 {
        for (part, own, node, call, or_at, zero_at) in [
            (0x2e8u32, 0x2f0u32, 0x2f5u32, 0x2fau32, 0x300u32, 0x30au32),
            (0x313, 0x31b, 0x320, 0x325, 0x32e, 0x335),
        ] {
            // push n; push ecx (a word the callee does not read); push P;
            // push owner
            g.write::<u32>(s0.wrapping_sub(4), disp8(img, f + part));
            let words = [w(own), w(own - 5)];
            let callee = stack_args(
                g,
                img.rel32_target(f + call),
                s0.wrapping_sub(0x14),
                w(node),
                0,
                &words,
            );
            eax = crate::s3_nodes::effect_node_with(g, img, &callee);
            let v = g.read::<u32>(w(or_at)) | disp8(img, f + or_at + 4);
            g.write::<u32>(w(or_at), v);
            g.write::<u32>(w(zero_at), w(zero_at + 4));
        }
    }
    let i = g.read::<u32>(count_at).wrapping_add(1);
    g.write::<u32>(count_at, i);
    if i.cast_signed() >= disp8(img, f + 0x349).cast_signed() {
        let node = g.read::<u32>(task);
        let prev = g.read::<u32>(node.wrapping_add(4));
        let next = g.read::<u32>(node.wrapping_add(8));
        g.write::<u32>(prev.wrapping_add(8), next);
        let next = g.read::<u32>(node.wrapping_add(8));
        eax = g.read::<u32>(node.wrapping_add(4));
        g.write::<u32>(next.wrapping_add(4), eax);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(node.wrapping_add(4), node);
    }
    let x = add(g.read::<f32>(ef(w(0x36a))), k(g, 0x372));
    g.write::<f32>(ef(w(0x36a)), x);
    Ret::eax(eax)
}

/// Two passes of 60 particles from an object's point (no arguments; a
/// /GS frame of 0xc4 bytes, `ebp` = `esp - 4` at the entry; `magic_0304`
/// 0x10002740 and `magic_0568` 0x10002170, 924 bytes; family
/// `F0304_10002740`). With `O = *G_O`: the point `P` (`ebp - 0x14`) =
/// `O`'s (`+0x26`'s displacement on) with `y - K1` (the second pass: as
/// it is); `V` (`ebp - 0x24`) = `(0, 0, -, 1)`; the matrix `L` (`ebp -
/// 0xb4`) from slot 235. Each particle from the pool
/// ([`crate::sb_misc::pool_take`]; none left ends the pass): the halfword
/// `+0x3a` = 0x200, `+0x30` = `*(*G_R + 0xc)`, `+0x34` = 1, the halfword
/// `+0x38` = 0xffff; `R` (`ebp - 0x74`) = slot 329 `(R, L, slot 314
/// (pi))`; `V.z = slot 404 (50) + slot 404 (50) + K2` (the second
/// first); `W` (`ebp - 0x34`) = slot 229 `(W, R, V)`; `+0..` = `P`; `+0
/// += 314 (10)`, `+4 += 314 (10)`, `+8 += 314 (10)`; `+0x10 = W.x * K3 +
/// 314 (1)`; `+0x14` = 314 `(0.3)`; `+0x18 = W.z * K3 + 314 (1)`; `+0x1c`
/// = 314 `(pi)`; `+0x20` 0, `+0x24` 0.05, `+0x28` 0, `+0x2c` 1.0; `+0x3c
/// = (314 (0.4) + K4) * K5`; `+0x4c` 0x46, `+0x50` 0; `+0x60 = rand() %
/// 5`; `+0x48` = 314 `(0.628)` (the slots' ST0 as f32; the immediates
/// the copy's).
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// the last slot 314's `eax`, or 0 when the pool ran out last.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn two_pass_particles<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x49), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let (p, v, l, r, wv) = (at(0x14), at(0x24), at(0xb4), at(0x74), at(0x34));
    let obj = g.read::<u32>(w(0x18));
    let point = |g: &mut G, ops: [u32; 4], first: bool| {
        for (i, o) in (0u32..).zip(ops) {
            let x = g.read::<f32>(obj.wrapping_add(w(o)));
            let x = if first && i == 1 {
                sub(x, k(g, 0x36))
            } else {
                x
            };
            g.write::<f32>(p.wrapping_add(4 * i), x);
        }
    };
    point(g, [0x26, 0x2e, 0x43, 0x56], true);
    for (i, o) in (0u32..).zip([0x63u32, 0x6a, 0x71, 0x78]) {
        g.write::<u32>(v.wrapping_add(4 * i), w(o));
    }
    let t = slot(g, 235);
    // call width: 1
    g.call(t, fr.ecx, &[l]);
    let take = img.rel32_target(f + 0xa6);
    let mut eax = 0;
    let mut passes = w(0x97);
    loop {
        let mut j = 0i32;
        while j < disp8(img, f + 0x33e).cast_signed() {
            let q = crate::sb_misc::pool_take(g, img, take);
            if q == 0 {
                eax = 0;
                break;
            }
            let pf = |o: u32| q.wrapping_add(o);
            let owner = g.read::<u32>(g.read::<u32>(w(0xb6)).wrapping_add(disp8(img, f + 0xc1)));
            g.write::<u16>(pf(0x3a), w(0xbb) as u16);
            g.write::<u32>(pf(0x30), owner);
            g.write::<u32>(pf(0x34), w(0xd2));
            g.write::<u16>(pf(0x38), 0xffff);
            let mut st = Pushes::default();
            st.push(owner);
            let st0 = |g: &mut G, st: &mut Pushes, n: u32, x: u32| {
                st.set_top(x);
                let t = slot(g, n);
                // call width: 12 (Pushes caps the words at 12)
                let out = st.call(g, t, fr.ecx);
                (out.eax, st0_f32(out.st0))
            };
            let (_, angle) = st0(g, &mut st, 314, w(0xe3));
            st.set_top(angle.to_bits());
            st.push_all(&[l, r]);
            let t = slot(g, 329);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, fr.ecx);
            st.pop(2);
            let (_, a) = st0(g, &mut st, 404, w(0x125));
            let (_, b) = st0(g, &mut st, 404, w(0x139));
            g.write::<f32>(v.wrapping_add(8), add(add(b, a), k(g, 0x167)));
            st.push_all(&[v, r, wv]);
            let t = slot(g, 229);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, fr.ecx);
            for i in 0..4u32 {
                let x = g.read::<u32>(p.wrapping_add(4 * i));
                g.write::<u32>(pf(4 * i), x);
            }
            st.pop(3);
            let (_, t) = st0(g, &mut st, 314, w(0x1a5));
            g.write::<f32>(pf(0), add(t, g.read::<f32>(pf(0))));
            for (o, x) in [(4u32, 0x1c5u32), (8, 0x1f0)] {
                let (_, t) = st0(g, &mut st, 314, w(x));
                g.write::<f32>(pf(o), add(g.read::<f32>(pf(o)), t));
            }
            let k3 = k(g, 0x23c);
            let (_, t) = st0(g, &mut st, 314, w(0x21c));
            g.write::<f32>(pf(0x10), add(mul(g.read::<f32>(wv), k3), t));
            let (_, t) = st0(g, &mut st, 314, w(0x248));
            g.write::<f32>(pf(0x14), t);
            let (_, t) = st0(g, &mut st, 314, w(0x26c));
            let z = mul(g.read::<f32>(wv.wrapping_add(8)), k(g, 0x287));
            g.write::<f32>(pf(0x18), add(z, t));
            let (_, t) = st0(g, &mut st, 314, w(0x293));
            g.write::<f32>(pf(0x1c), t);
            for (o, x) in [
                (0x20u32, 0x2b7u32),
                (0x24, 0x2be),
                (0x28, 0x2c5),
                (0x2c, 0x2cc),
            ] {
                g.write::<u32>(pf(o), w(x));
            }
            let (_, t) = st0(g, &mut st, 314, w(0x2d9));
            g.write::<u32>(pf(0x4c), w(0x2f8));
            g.write::<u32>(pf(0x50), w(0x2ff));
            g.write::<f32>(pf(0x3c), mul(add(t, k(g, 0x2f1)), k(g, 0x307)));
            let n = g.rand().cast_signed() % w(0x318).cast_signed();
            g.write::<u32>(pf(0x60), n.cast_unsigned());
            let (e, t) = st0(g, &mut st, 314, w(0x326));
            eax = e;
            g.write::<f32>(pf(0x48), t);
            j += 1;
        }
        passes = passes.wrapping_sub(1);
        point(g, [0x355, 0x362, 0x36f, 0x37c], false);
        if passes == 0 {
            return Ret::eax(eax);
        }
    }
}

/// The centres of 18 meshes (no arguments; a /GS frame of 0x38 bytes,
/// `ebp` = `esp - 4` at the entry; `magic_0307` 0x10002010 and
/// `magic_0605` 0x10007990, 1045 bytes; family `F0307_10002010`): for
/// each mesh `M = *(*G_M + 0x20 k + D)` (`D` the `disp32` at `+0x38`),
/// with its packets from `M + M[4]` and its i16 vertices (x, y, z) from
/// `M + M[8]`, the sum `S` (`ebp - 0x24`) starts at 0; the packets run
/// while the i16 count `M + 0x10` (less each packet's i16 `+2`) is above
/// 0: a packet of type 6 (the signed byte `+1`) holds `+2` quads of 0x28
/// bytes (vertex indices, u16, at `+0x10..+0x16` of each), type 2
/// triangles of 0x20 bytes (`+0xc..+0x10`), the others none; each face's
/// vertices are summed (as f32, in their order), slot 564 `(V, V, n)`
/// scales the sum `V` (`ebp - 0x14`, `w` 0) by `n` (4.0 or 3.0) and slot
/// 248 `(S, S, V)` adds it. Then with the count above 0 slot 564 `(S, S,
/// count)`; `S` goes to the output `*G_O + OUT + 0x60 k` with `w` = 1.0.
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// the last mesh's slot 564 `eax`, or its count (a u16) when 0 or below.
#[allow(clippy::too_many_lines)] // the original's loops, one by one
pub fn mesh_centres<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x1fe), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (s, v) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let mut out = g.read::<u32>(w(0x14)).wrapping_add(w(0x1a));
    let mut eax = 0;
    let mut k = 0u32;
    while k.cast_signed() < w(0x3fa).cast_signed() {
        let m = g.read::<u32>(g.read::<u32>(w(0x31)).wrapping_add(k).wrapping_add(w(0x38)));
        let mut packet = m.wrapping_add(g.read::<u32>(m.wrapping_add(4)));
        let verts = m.wrapping_add(g.read::<u32>(m.wrapping_add(8)));
        for i in 0..4u32 {
            g.write::<u32>(s.wrapping_add(4 * i), 0);
        }
        g.write::<u32>(v.wrapping_add(0xc), 0);
        // one coordinate of vertex `index` as f32
        let coord = |g: &G, index: u16, c: u32| {
            let at = verts.wrapping_add(6 * u32::from(index)).wrapping_add(2 * c);
            cvt(i32::from(g.read::<i16>(at)).cast_unsigned())
        };
        let mut left = i32::from(g.read::<i16>(m.wrapping_add(0x10)));
        while left > 0 {
            let kind = g.read::<i8>(packet.wrapping_add(1));
            let mut face = packet.wrapping_add(0x10);
            // (the first index's offset in a face, the corners, the face
            // size, the divisor's operand)
            let shape = match kind {
                6 => Some((0x10u32, 4u32, 0x28u32, 0x1f7u32)),
                2 => Some((0xc, 3, 0x20, 0x25c)),
                _ => None,
            };
            if let Some((first, corners, size, div)) = shape {
                let mut j = 0i32;
                while j < i32::from(g.read::<i16>(packet.wrapping_add(2))) {
                    for c in 0..3 {
                        let mut x = 0.0f32;
                        for corner in 0..corners {
                            let index = g.read::<u16>(face.wrapping_add(first + 2 * corner));
                            let y = coord(g, index, c);
                            x = if corner == 0 { y } else { add(y, x) };
                        }
                        g.write::<f32>(v.wrapping_add(4 * c), x);
                    }
                    let mut st = Pushes::default();
                    st.push_all(&[w(div), v, v]);
                    let t = slot(g, 564);
                    // call width: 12 (Pushes caps the words at 12)
                    st.call(g, t, fr.ecx);
                    st.push_all(&[v, s, s]);
                    let t = slot(g, 248);
                    // call width: 12 (Pushes caps the words at 12)
                    st.call(g, t, fr.ecx);
                    j += 1;
                    face = face.wrapping_add(size);
                }
            }
            left -= i32::from(g.read::<i16>(packet.wrapping_add(2)));
            packet = face;
        }
        let count = g.read::<u16>(m.wrapping_add(0x10));
        eax = u32::from(count);
        if count.cast_signed() > 0 {
            let mut st = Pushes::default();
            st.push_all(&[
                cvt(i32::from(count.cast_signed()).cast_unsigned()).to_bits(),
                s,
                s,
            ]);
            let t = slot(g, 564);
            // call width: 12 (Pushes caps the words at 12)
            eax = st.call(g, t, fr.ecx).eax;
        }
        for i in 0..4u32 {
            let x = g.read::<u32>(s.wrapping_add(4 * i));
            g.write::<u32>(out.wrapping_add(4 * i), x);
        }
        g.write::<u32>(out.wrapping_add(0xc), w(0x3eb));
        out = out.wrapping_add(disp8(img, f + 0x3f1));
        k = k.wrapping_add(0x20);
    }
    Ret::eax(eax)
}

/// One effect object aimed at each target (no arguments; an aligned /GS
/// frame: `ebp` = the entry `esp - 4`, `E` = `(ebp & !7) - 0x60` after
/// the saved registers; `magic_0644` 0x10008900 and `magic_0565`
/// 0x100048e0, 1089 bytes; family `F0644_10008900`). With `O = *G_O`,
/// its target list `T = O + TL` (`+0x21`'s displacement; the count `T +
/// 4`, the target ids bytes from `T + 8`): `h` = slot 219 `(*G_H, 0)`,
/// slot 237 `(h, 0, 0, &L)` (`L` = `E + 0x4c`). For each target `k`, its
/// record `R = O + RB + 0x520 k`:
/// - `R - 0x18..` = `(L.x, L.y, L.z, K)`; the key `C[A[id]]` (`A` =
///   `*G_A`, `C` the table at `+0xc9`): slot 262 `(C, R - 8, R - 4, R)`,
///   slot 268 `(C, 0x10, R - 8)`; `R + 4` = 1.0;
/// - with `dy = R[-4] - R[-0x14]`, `dx = R[-8] - R[-0x18]`, `dz = R[0] -
///   R[-0x10]` (the f32 words): `R + 8` = `atan2(dy, sqrt(dz dz + dx
///   dx))`, `R + 0xc` = `K2 - atan2(dz, dx)` wrapped once by a turn
///   into `[KC, KB]`, `R + 0x10` = 0 (`_libm_sse2_sqrt_precise` is
///   correctly rounded, as `f64::sqrt`; `_CIatan2` is the game's);
/// - `n` = `sqrt(|R[-0x18..-0x10] - R[-8..0]|^2) * K3` truncated, 1 when
///   negative, into `R + 0x294` and `R + 0x298`;
/// - the object `R - 0x278` started ([`crate::s4_misc::start_object_at`]
///   with the scale `F[e]` and index `I[e]`, `e` = `O + 0x83f4`), `R +
///   8..0x14` copied to `R - 0x1b8..`, `R + 0x29c` = the id, `R + 0x2a0` =
///   `k`, and a node taken for it ([`take_busy_node`], unmarked).
///
/// Then `O + 0x83f8` = the count, `O + 0x83fc` = 0, the object `O +
/// 0x83f0` leaves the list `LS` ([`crate::l5_objhost::list_remove`]; when
/// it was there `*G_P = ITEM`, slot 713 `(obj)` and bit 15 of its `+0x124`
/// cleared); slot 257 `(slot 219 (*G_H, 0), 1)`, slot 38 `(slot 219 (*G_H,
/// 0), 0, 1)`.
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// slot 38's `eax`.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn target_objects<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x2f), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x60);
    let l = e.wrapping_add(0x4c);
    let obj = g.read::<u32>(w(0x19));
    let targets = g.read::<u32>(obj.wrapping_add(w(0x21)));
    let count = g.read::<u32>(targets.wrapping_add(4));
    let mut st = Pushes::default();
    st.push_all(&[0, g.read::<u32>(w(0x27))]);
    let t = slot(g, 219);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t, fr.ecx).eax;
    st.push_all(&[l, 0, 0, h]);
    let t = slot(g, 237);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    let kw = g.read::<u32>(w(0x5e));
    let mut r = obj.wrapping_add(w(0x77));
    let node_ops = NodeOps {
        a: 0,
        t217: (0, 0),
        t234: (0, 0),
        t233: [(0, 0); 3],
        free: 0x2b2,
        k_a4: 0,
        k_9c: 0,
        head: 0x2ca,
        callback: 0x320,
        free_w: 0x32b,
        last_cmp: 0x331,
        last: 0x349,
        last_w: 0x35c,
        first_w: 0x370,
    };
    let mut i = 0i32;
    while i < count.cast_signed() {
        let rf = |o: u32| r.wrapping_add(o);
        let list = g.read::<u32>(obj.wrapping_add(w(0x82)));
        let id = u32::from(g.read::<u8>(list.wrapping_add(i.cast_unsigned()).wrapping_add(8)));
        for (n, o) in (0u32..).zip([0u32, 4, 8]) {
            let x = g.read::<u32>(l.wrapping_add(o));
            g.write::<u32>(rf(0u32.wrapping_sub(0x18 - 4 * n)), x);
        }
        g.write::<u32>(rf(0u32.wrapping_sub(0xc)), kw);
        // (`cmp edi, -1` on a zero-extended byte: the id is never -1)
        let a = g.read::<u8>(g.read::<u32>(w(0xbe)).wrapping_add(id));
        let key = g.read::<u32>(w(0xc9).wrapping_add(4 * u32::from(a)));
        let mut st = Pushes::default();
        st.push_all(&[r, rf(0u32.wrapping_sub(4)), rf(0u32.wrapping_sub(8)), key]);
        let t = slot(g, 262);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, fr.ecx);
        st.push_all(&[rf(0u32.wrapping_sub(8)), disp8(img, f + 0xf2), key]);
        let t = slot(g, 268);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, fr.ecx);
        g.write::<u32>(rf(4), w(0x102));
        let at = |g: &G, o: u32| g.read::<f32>(rf(0u32.wrapping_sub(o)));
        let dy = sub(at(g, 4), at(g, 0x14));
        let dx = sub(at(g, 8), at(g, 0x18));
        let dz = sub(at(g, 0), at(g, 0x10));
        let flat = add(mul(dz, dz), mul(dx, dx));
        let pitch = g.atan2(f64::from(dy), f64::from(flat).sqrt());
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: round to f32
        g.write::<f32>(rf(8), pitch as f32);
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: round to f32
        let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let turn = wrap_turn(sub(k(g, 0x1b8), yaw), k(g, 0x1c7), k(g, 0x1db), k(g, 0x1d1));
        g.write::<f32>(rf(0xc), turn);
        g.write::<u32>(rf(0x10), w(0x1f4));
        let a = sub(at(g, 0x14), at(g, 4));
        let b = sub(at(g, 0x18), at(g, 8));
        let c = sub(at(g, 0x10), at(g, 0));
        let sq = add(add(mul(a, a), mul(b, b)), mul(c, c));
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss: round to f32
        let dist = f64::from(sq).sqrt() as f32;
        let n = crate::s4_summon::trunc(mul(dist, k(g, 0x23b)));
        let n = if n.cast_signed() < 0 { w(0x248) } else { n };
        g.write::<u32>(rf(0x294), n);
        g.write::<u32>(rf(0x298), n);
        let e_idx = g.read::<u32>(obj.wrapping_add(w(0x267)));
        let scale = g.read::<f32>(w(0x271).wrapping_add(4 * e_idx));
        let index = g.read::<u32>(w(0x27a).wrapping_add(4 * e_idx));
        let o = rf(0u32.wrapping_sub(0x278));
        crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x281), o, scale, index, 0, 0);
        for n in 0..4u32 {
            let x = g.read::<u32>(rf(8 + 4 * n));
            g.write::<u32>(rf(0u32.wrapping_sub(0x1b8 - 4 * n)), x);
        }
        g.write::<u32>(rf(0x29c), id);
        let free = g.read::<u16>(w(0x2b2));
        g.write::<u32>(rf(0x2a0), i.cast_unsigned());
        take_busy_node(g, img, f, free, &node_ops, o, false);
        i += 1;
        r = r.wrapping_add(w(0x367));
    }
    g.write::<u32>(obj.wrapping_add(w(0x38b)), count);
    let gone = g.read::<u32>(obj.wrapping_add(w(0x391)));
    g.write::<u32>(obj.wrapping_add(w(0x39e)), w(0x3a2));
    let list = w(0x398);
    if crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x3a7), list, gone) != 0 {
        let p = g.read::<u32>(w(0x3b0));
        g.write::<u32>(p, w(0x3b7));
        let t = slot(g, 713);
        // call width: 1
        g.call(t, list, &[gone]);
        let v = g.read::<u16>(gone.wrapping_add(w(0x3d3))) & w(0x3c9) as u16;
        g.write::<u16>(gone.wrapping_add(w(0x3d3)), v);
    }
    let mut st = Pushes::default();
    st.push_all(&[0, g.read::<u32>(w(0x3e0))]);
    let t = slot(g, 219);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t, fr.ecx).eax;
    st.push_all(&[disp8(img, f + 0x3ed), h]);
    let t = slot(g, 257);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    st.push_all(&[0, g.read::<u32>(w(0x405))]);
    let t = slot(g, 219);
    // call width: 12 (Pushes caps the words at 12)
    let h2 = st.call(g, t, fr.ecx).eax;
    let mut st = Pushes::default();
    st.push_all(&[disp8(img, f + 0x415), w(0x41a), h2]);
    let t = slot(g, 38);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(st.call(g, t, fr.ecx).eax)
}

/// A sprite record's frame drawn (internal: `ecx` = the record `r`,
/// `xmm3` = a time step `dt`, stack `(id, part)`; a /GS frame of 0x9c
/// bytes, `ebp` = `esp - 4` at the entry; `magic_0206` 0x10002730 and
/// `magic_0371` 0x10002480, 997 bytes; family `F0206_10002730`). Without
/// `r`, or when slot 941 `(*r)` gives 0, the error string goes to the
/// OPU's printf (slot 726 through the 28-byte wrapper) and the result is
/// 0. Else with `tbl` = slot 941 `(*r)`:
/// - slot 723 `(M1)`, 724 `(M2)`; slot 235 on the DLL's matrices `A`,
///   `B`, `C`; slot 383 `(A, r.f24)`; slot 231 `(A, C, A)`; slot 330 `(A,
///   A, r.f28)`; slot 231 `(A, B, A)`; slot 229 `(D, M2, r + 0x14)`; slot
///   230 `(E, A)`; `G_A = A`, `G_E = E`, the f32 `G_T -= dt`;
/// - the u16 timer `r + 0x10` drops by 0x200; reaching 0 the frame `r +
///   8` moves on (back to 0 at `tbl[+4]` frames), with `t` = slot 941
///   `(*r)` read again for each word: `r + 4` = `t + 8 i + 0x10`, `r + 0xc
///   = t + i16 t[8 i + 0x10]`, the timer += `t[8 i + 0x12]`;
/// - the colour bytes `G_C..+3` = `r + 0x2c`, `+0x30`, `+0x34` and
///   `r.f38 * K` truncated; `G_ID` = `id`; `h` = slot 772 `(tbl, part)`
///   (64 bits); with the game lock `*G_LOCK` 1 around each call: when
///   slot 773 `(h, tbl, part)` is not 0, slot 774 `(h, r, G_C, frame)`
///   and slot 775 `(h)`; else slot 778 `(h, r, G_C, frame, slot 776
///   (byte) | slot 777 (*G_E))` (the byte at `r[0xc] + r[0xc][4] + 4`).
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// `id`.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn sprite_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    const MODEL: FrameLayout = FrameLayout {
        shift: crate::r32::Shift {
            runs: &[],
            moved: &[],
        },
        desc: None,
    };
    sprite_frame_with(g, img, fr, &MODEL)
}

/// How a copy of [`sprite_frame`] differs: its operands (the model's
/// offsets, [`Shift`](crate::r32::Shift)) and, with `desc` (the copy's
/// operand offsets of the matrix and the light words it stores), the
/// draw record in `edx` (`D`) instead of the DLL's globals: `D + 0x10` =
/// the matrix, `D + 0x1c` = the light word, the colour bytes `D + 4..8`,
/// the id `D + 0x14`, `D` handed to slots 774/778 and `D + 0x1c` to slot
/// 777 (`magic_0370` 0x10002810).
pub(crate) struct FrameLayout {
    pub(crate) shift: crate::r32::Shift,
    pub(crate) desc: Option<(u32, u32)>,
}

/// [`sprite_frame`] for a copy laid out as `lay` says.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub(crate) fn sprite_frame_with<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    lay: &FrameLayout,
) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + lay.shift.at(o));
    let rd = fr.edx;
    let slot = |g: &G, n: u32| slot_target(g, w(0x29), n);
    let r = fr.ecx;
    let dt = fr.xmm_f32(3);
    let (id, part) = (fr.arg(g, 0), fr.arg(g, 1));
    let ebp = fr.esp.wrapping_sub(4);
    let tbl_of = |g: &mut G| {
        let key = g.read::<u32>(r);
        let t = slot(g, 941);
        // call width: 1
        g.call(t, r, &[key]).eax
    };
    if r == 0 || tbl_of(g) == 0 {
        // the 28-byte wrapper: `vprintf(fmt, &args)` through slot 726,
        // `&args` above its pushed string (`ebp - 0xa8` here)
        let wrapper = img.rel32_target(f + lay.shift.at(0x3cb));
        let t = slot_target(g, img.word(wrapper + 8), 726);
        // call width: 2
        g.call(t, r, &[w(0x3c6), ebp.wrapping_sub(0xa8)]);
        return Ret::eax(0);
    }
    let tbl = tbl_of(g);
    let (m1, m2) = (ebp.wrapping_sub(0x88), ebp.wrapping_sub(0x48));
    let (a, b, c, d, e) = (w(0x93), w(0xa8), w(0xbd), w(0x14f), w(0x16f));
    for (m, n) in [(m1, 723u32), (m2, 724), (a, 235), (b, 235)] {
        let t = slot(g, n);
        // call width: 1
        g.call(t, r, &[m]);
    }
    let mut st = Pushes::default();
    st.push(c);
    let t = slot(g, 235);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, r);
    st.set_top(g.read::<u32>(r.wrapping_add(0x24)));
    st.push(a);
    let t = slot(g, 383);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, r);
    st.push_all(&[a, c, a]);
    let t = slot(g, 231);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, r);
    st.pop(4);
    st.set_top(g.read::<u32>(r.wrapping_add(0x28)));
    st.push_all(&[a, a]);
    let t = slot(g, 330);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, r);
    let mut st = Pushes::default();
    st.push_all(&[a, b, a]);
    let t = slot(g, 231);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, r);
    let mut st = Pushes::default();
    st.push_all(&[r.wrapping_add(0x14), m2, d]);
    let t = slot(g, 229);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, r);
    let mut st = Pushes::default();
    st.push_all(&[a, e]);
    let t = slot(g, 230);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, r);
    let time = sub(g.read::<f32>(w(0x179)), dt);
    if let Some((m, e)) = lay.desc {
        g.write::<f32>(w(0x1a5), time);
        g.write::<u32>(rd.wrapping_add(0x10), img.word(f + m));
        g.write::<u32>(rd.wrapping_add(0x1c), img.word(f + e));
    } else {
        g.write::<u32>(w(0x18a), w(0x18e));
        g.write::<u32>(w(0x194), w(0x198));
        g.write::<f32>(w(0x1a5), time);
    }
    let timer = g
        .read::<u16>(r.wrapping_add(0x10))
        .wrapping_add(w(0x19d) as u16);
    g.write::<u16>(r.wrapping_add(0x10), timer);
    if timer == 0 {
        let frame = g.read::<u16>(r.wrapping_add(8)).wrapping_add(1);
        g.write::<u16>(r.wrapping_add(8), frame);
        let t = tbl_of(g);
        if g.read::<i16>(r.wrapping_add(8)) >= g.read::<i16>(t.wrapping_add(4)) {
            g.write::<u16>(r.wrapping_add(8), 0);
        }
        let t = tbl_of(g);
        let i = i32::from(g.read::<i16>(r.wrapping_add(8))).cast_unsigned();
        let t2 = tbl_of(g);
        let i2 = i32::from(g.read::<i16>(r.wrapping_add(8))).cast_unsigned();
        g.write::<u32>(
            r.wrapping_add(4),
            t2.wrapping_add(8 * i2).wrapping_add(0x10),
        );
        let t3 = tbl_of(g);
        let at = t.wrapping_add(8 * i);
        let base = i32::from(g.read::<i16>(at.wrapping_add(0x10))).cast_unsigned();
        g.write::<u32>(r.wrapping_add(0xc), base.wrapping_add(t3));
        let step = g.read::<u16>(at.wrapping_add(0x12));
        let timer = g.read::<u16>(r.wrapping_add(0x10)).wrapping_add(step);
        g.write::<u16>(r.wrapping_add(0x10), timer);
    }
    // the colour bytes, alpha and id: the globals, or `D + 4..8`, `D + 0x14`
    let field = |at: u32, k: u32| {
        if lay.desc.is_some() {
            rd.wrapping_add(k)
        } else {
            w(at)
        }
    };
    for (o, at, k) in [
        (0x2cu32, 0x23bu32, 4u32),
        (0x30, 0x244, 5),
        (0x34, 0x250, 6),
    ] {
        let v = g.read::<u8>(r.wrapping_add(o));
        g.write::<u8>(field(at, k), v);
    }
    let alpha = crate::s4_summon::trunc(mul(
        g.read::<f32>(r.wrapping_add(0x38)),
        g.read::<f32>(w(0x25d)),
    ));
    g.write::<u8>(field(0x26c, 7), alpha as u8);
    g.write::<u32>(field(0x274, 0x14), id);
    let anim = g.read::<u32>(r.wrapping_add(0xc));
    let mut st = Pushes::default();
    st.push_all(&[part, tbl]);
    let t = slot(g, 772);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t, r);
    let (lo, hi) = (h.eax, h.edx);
    let lock = g.read::<u32>(w(0x290));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    st.push_all(&[hi, lo]);
    let t = slot(g, 773);
    // call width: 12 (Pushes caps the words at 12)
    let drawn = st.call(g, t, r).eax;
    let lock = g.read::<u32>(w(0x290));
    g.write::<u32>(lock, saved);
    let colours = if lay.desc.is_some() { rd } else { w(0x2e2) };
    if drawn != 0 {
        let frame = u32::from(g.read::<u16>(r.wrapping_add(8)));
        g.write::<u32>(lock, 1);
        let words = [lo, hi, r, colours, frame];
        let t = slot(g, 774);
        // call width: 5
        g.call(t, r, &words);
        let lock = g.read::<u32>(w(0x290));
        g.write::<u32>(lock, 1);
        let t = slot(g, 775);
        // call width: 2
        g.call(t, r, &[lo, hi]);
        let lock = g.read::<u32>(w(0x290));
        g.write::<u32>(lock, saved);
        return Ret::eax(id);
    }
    let byte = g.read::<u8>(
        g.read::<u32>(anim.wrapping_add(4))
            .wrapping_add(anim)
            .wrapping_add(4),
    );
    let t = slot(g, 776);
    // call width: 1
    let x = g.call(t, r, &[u32::from(byte)]).eax;
    let ge = g.read::<u32>(field(0x365, 0x1c));
    let t = slot(g, 777);
    // call width: 1
    let y = g.call(t, r, &[ge]).eax;
    let frame = u32::from(g.read::<u16>(r.wrapping_add(8)));
    let lock = g.read::<u32>(w(0x290));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let words = [lo, hi, r, colours, frame, x | y];
    let t = slot(g, 778);
    // call width: 6
    g.call(t, r, &words);
    let lock = g.read::<u32>(w(0x290));
    g.write::<u32>(lock, saved);
    Ret::eax(id)
}

/// `|f64(x)|` (`cvtps2pd`, then `andpd` with the mask `m`).
fn abs_f64(x: f32, m: u64) -> f64 {
    f64::from_bits(f64::from(x).to_bits() & m)
}

/// Moving sprites, each frame (no arguments; an aligned /GS frame: `ebp`
/// = the entry `esp - 4`, `E` = `(ebp & !7) - 0x118` after the saved
/// registers; `magic_0250` 0x10001ee0 and `magic_0251` 0x10002090, 1051
/// bytes; family `F0250_10001ee0`). Slot 723 `(M1)`, 724 `(M2)` (`M1` =
/// `E + 0xd0`, `M2` = `E + 0x90`); `*G_M` = `M3` (`E + 0x50`), `*G_Z` =
/// 0. For each of the `*G_N` records (0x68 bytes from `*G_P`; `p` =
/// record + 0x14) with `p + 0x4c` set:
/// - the motion: `p.18 += p.1c`; `p[-8] += p.c`; `y = p.18 + p.10`;
///   `p[-4] += y`; `p.0 += p.14`; the callback `p + 8` (when set) gets
///   `(record)`;
/// - unless bit 0 of `p + 0x50`, once `p[-4]` reaches `p.28`: (with `p.10`
///   and `y` not 0) `s = K1 + (p[-4] - y) / y`, `p[-8] -= p.c s`, `p.0 -=
///   p.14 s`; `p[-4] = p.28`; `p.1c *= p.20`, and when `|p.1c| > |p.10|`
///   (as f64) `p.10 = 0`, `p.1c = p.20`; `p.18 = 0`, `p.c *= p.24`, `p.14
///   *= p.24`;
/// - the matrix `M4` (`E + 0x10`, slot 235, its diagonal `p.34`) turned
///   by slot 330 `(M3, M4, a)` (`a = p.30 + p.2c` wrapped once by a turn
///   into `[KC, KB]`, stored in `p.2c`); slot 229 `(M5, M2, p - 8)` (`M5`
///   = `E + 0x80`);
/// - the colour `G_C` (4 bytes): with `n = p.3c` above 0 each channel
///   `(c0 (n - d) + c1 d) / n` (`d = |p.40|`, `c0`/`c1` the bytes at `p +
///   0x44`/`+ 0x48`), `p.40` up by one (past `n`, `n` = 0); else 0x80808080;
/// - the frame: with `t` = slot 941 `(record[0])` and the frame `f` =
///   the u16 `record + 8`: while `record + 4` is set the u16 timer
///   `record + 0xa` drops by 0x200; at 0 or below the frame moves on (at
///   `t`'s count: back to 0 when the last entry's byte `+4` has bit 7,
///   else it stays at the last and `record + 4` = 0) and the timer is the
///   new entry's `+2`; then with `p.38` below 0 and `record + 4` not 1,
///   `p.38` is 1; else the part is linked
///   ([`crate::sa_misc::link_part`] `(G_REC, index)`, `edx` = the entry);
/// - `p.38` above 0 counts down; reaching 0 the record stops (`p.4c` = 0).
///
/// Returns the record count reached (0 without records).
#[allow(clippy::too_many_lines)] // the original's branches, one by one
pub fn moving_sprites<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x2b), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x118);
    let at = |o: u32| e.wrapping_add(o);
    let (m1, m2, m3, m4, m5) = (at(0xd0), at(0x90), at(0x50), at(0x10), at(0x80));
    let mut p = g.read::<u32>(w(0x1e));
    for m in [(m1, 723u32), (m2, 724)] {
        let t = slot(g, m.1);
        // call width: 1
        g.call(t, fr.ecx, &[m.0]);
    }
    g.write::<u32>(w(0x54), m3);
    g.write::<u32>(w(0x5f), w(0x63));
    let mut i = 0u32;
    if g.read::<u32>(w(0x6d)).cast_signed() <= 0 {
        return Ret::eax(0);
    }
    let mask = u64::from(g.read::<u32>(w(0x7b))) | u64::from(g.read::<u32>(w(0x7b) + 4)) << 32;
    p = p.wrapping_add(disp8(img, f + 0x81));
    loop {
        let pf = |o: u32| p.wrapping_add(o);
        let pm = |o: u32| p.wrapping_sub(o);
        let rd = |g: &G, a: u32| g.read::<f32>(a);
        if g.read::<u32>(pf(0x4c)) != 0 {
            let v = add(rd(g, pf(0x18)), rd(g, pf(0x1c)));
            g.write::<f32>(pf(0x18), v);
            let x = add(rd(g, pm(8)), rd(g, pf(0xc)));
            g.write::<f32>(pm(8), x);
            let mut y = add(v, rd(g, pf(0x10)));
            let x = add(rd(g, pm(4)), y);
            g.write::<f32>(pm(4), x);
            g.write::<f32>(at(8), y);
            let x = add(rd(g, pf(0x14)), rd(g, pf(0)));
            g.write::<f32>(pf(0), x);
            let cb = g.read::<u32>(pf(8));
            if cb != 0 {
                // call width: 1
                g.call(cb, cb, &[pm(0x14)]);
                y = rd(g, at(8));
            }
            let held = g.read::<u8>(pf(0x50)) & 1 != 0;
            let level = rd(g, pm(4));
            let limit = rd(g, pf(0x28));
            // comiss, jb: below or unordered
            let below = level < limit || level.is_nan() || limit.is_nan();
            if !held && !below {
                // `ucomiss; lahf; test ah, 0x44; jnp`: taken when equal
                if rd(g, pf(0x10)) != 0.0 && y != 0.0 {
                    let q = div(sub(level, y), y);
                    let q = f32::from_bits(q.to_bits() ^ g.read::<u32>(w(0x141)));
                    let s = sub(k(g, 0x12d), q);
                    let x = sub(rd(g, pm(8)), mul(rd(g, pf(0xc)), s));
                    g.write::<f32>(pm(8), x);
                    let x = sub(rd(g, pf(0)), mul(rd(g, pf(0x14)), s));
                    g.write::<f32>(pf(0), x);
                }
                let v = g.read::<u32>(pf(0x28));
                g.write::<u32>(pm(4), v);
                let p20 = rd(g, pf(0x20));
                let p10 = rd(g, pf(0x10));
                let n = mul(p20, rd(g, pf(0x1c)));
                g.write::<f32>(pf(0x1c), n);
                if abs_f64(n, mask) > abs_f64(p10, mask) {
                    g.write::<u32>(pf(0x10), w(0x19e));
                    g.write::<f32>(pf(0x1c), p20);
                }
                let c = mul(rd(g, pf(0x24)), rd(g, pf(0xc)));
                g.write::<u32>(pf(0x18), w(0x1b4));
                g.write::<f32>(pf(0xc), c);
                let x = mul(rd(g, pf(0x14)), rd(g, pf(0x24)));
                g.write::<f32>(pf(0x14), x);
            }
            let t = slot(g, 235);
            // call width: 1
            g.call(t, fr.ecx, &[m4]);
            let s34 = g.read::<u32>(pf(0x34));
            for o in [0x28u32, 0x14, 0] {
                g.write::<u32>(m4.wrapping_add(o), s34);
            }
            let a = add(rd(g, pf(0x30)), rd(g, pf(0x2c)));
            let a = wrap_turn(a, k(g, 0x205), k(g, 0x219), k(g, 0x20f));
            g.write::<f32>(pf(0x2c), a);
            let mut st = Pushes::default();
            st.push_all(&[a.to_bits(), m4, m3]);
            let t = slot(g, 330);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, fr.ecx);
            let mut st = Pushes::default();
            st.push_all(&[pm(8), m2, m5]);
            let t = slot(g, 229);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, fr.ecx);
            let n = g.read::<u32>(pf(0x3c));
            if n.cast_signed() > 0 {
                let mut d = g.read::<u32>(pf(0x40));
                g.write::<u32>(at(8), d);
                if d.cast_signed() < 0 {
                    d = d.wrapping_neg();
                    g.write::<u32>(at(8), d);
                }
                for (ch, out) in [(0u32, 0x2a4u32), (1, 0x2c6), (2, 0x2e8), (3, 0x30a)] {
                    let n = g.read::<u32>(pf(0x3c));
                    let d = g.read::<u32>(at(8));
                    let c0 = u32::from(g.read::<u8>(pf(0x44 + ch)));
                    let c1 = u32::from(g.read::<u8>(pf(0x48 + ch)));
                    let sum = c0
                        .wrapping_mul(n.wrapping_sub(d))
                        .wrapping_add(c1.wrapping_mul(d));
                    let q = sum.cast_signed().wrapping_div(n.cast_signed());
                    g.write::<u8>(w(out), q.cast_unsigned() as u8);
                }
                let d = g.read::<u32>(pf(0x40)).wrapping_add(1);
                g.write::<u32>(pf(0x40), d);
                if d.cast_signed() > g.read::<u32>(pf(0x3c)).cast_signed() {
                    g.write::<u32>(pf(0x3c), 0);
                }
            } else {
                g.write::<u32>(w(0x324), w(0x328));
            }
            let key = g.read::<u32>(pm(0x14));
            let t = slot(g, 941);
            // call width: 1
            let tbl = g.call(t, fr.ecx, &[key]).eax;
            let frame = g.read::<u16>(pm(0xc));
            let entry =
                |n: u16| tbl.wrapping_add((i32::from(n.cast_signed()) + 2).cast_unsigned() << 3);
            let mut edx = entry(frame);
            if g.read::<u32>(pm(0x10)) != 0 {
                let timer = g.read::<u16>(pm(0xa)).wrapping_add(w(0x355) as u16);
                g.write::<u16>(pm(0xa), timer);
                if timer.cast_signed() <= 0 {
                    let mut next = frame.wrapping_add(1);
                    g.write::<u16>(pm(0xc), next);
                    let mut moved = true;
                    if next.cast_signed() >= g.read::<i16>(tbl.wrapping_add(6)) {
                        if g.read::<u8>(edx.wrapping_add(4)) & 0x80 != 0 {
                            next = 0;
                            g.write::<u16>(pm(0xc), next);
                        } else {
                            g.write::<u16>(pm(0xc), next.wrapping_sub(1));
                            g.write::<u32>(pm(0x10), 0);
                            moved = false;
                        }
                    }
                    if moved {
                        let n = g.read::<u16>(pm(0xc));
                        edx = entry(n);
                        let step = g.read::<u16>(edx.wrapping_add(2));
                        g.write::<u16>(pm(0xa), step);
                    }
                }
            }
            if g.read::<u32>(pf(0x38)).cast_signed() < 0 && g.read::<u32>(pm(0x10)) != 1 {
                g.write::<u32>(pf(0x38), w(0x3b0));
            } else {
                let callee = stack_args(
                    g,
                    img.rel32_target(f + 0x3c3),
                    e.wrapping_sub(0xc),
                    pm(0x14),
                    edx,
                    &[w(0x3be), i],
                );
                crate::sa_misc::link_part(g, img, &callee);
            }
            let left = g.read::<u32>(pf(0x38));
            if left.cast_signed() > 0 {
                let left = left.wrapping_sub(1);
                g.write::<u32>(pf(0x38), left);
                if left == 0 {
                    g.write::<u32>(pf(0x4c), w(0x3e9));
                }
            }
        }
        i = i.wrapping_add(1);
        p = p.wrapping_add(disp8(img, f + 0x3f6));
        if i.cast_signed() >= g.read::<u32>(w(0x3fd)).cast_signed() {
            return Ret::eax(i);
        }
    }
}

/// An actor's light task (`(task)` with the record `S = task[+0x14]`; an
/// aligned /GS frame: `ebp` = the entry `esp - 4`, `E` = `(ebp & !7) -
/// 0xd0` after the saved registers; `magic_0565` 0x10005470 and
/// `magic_0644` 0x100092b0, 1665 bytes; family `F0565_10005470`):
/// - the countdown `S + 0x50c` drops (when above 0); `u = n / S[0x510]`
///   (as f32), the light's point `V` (`E + 0x28`) = `S.260.. * u +
///   S.270.. * (K1 - u)`, `w` 1.0; the light set up ([`light_setup`]:
///   `ecx` = `S + 0x280`, `edx` = `V`, `(0x80)`); while the countdown is
///   above 0 that is all (its `eax`);
/// - else the actor's object restarted
///   ([`crate::o6_fam_pairs2::actor_object_start`]); with the effect's
///   `*G_O + FLAG` 1 a glow record (`*G_O + G`, its words from the copy's
///   immediates) gets a node ([`crate::sa_pool::take`] with the
///   callback, appended to the busy list), the period `S + 0x510` grows by
///   8 and restarts the countdown, slot 268 `(*G_X, 0xb, S + 0x260)`,
///   `S + 0x26c` = 1.0, and the task's callback `task[8] + 0x10` = `CB2`;
/// - otherwise the light's new aim: `S + 0x260..` = `V`; `V` = `(slot 314
///   (a), -(slot 314 (a) + K2), -(x * KD), 1)` with `x = slot 314 (b) +
///   KA`, or `KA` itself when inside `(KC, KB)`; the matrix `M` (`E +
///   0x48`) from slots 235, 328 `(S.280)`, 329 `(S.284)`; `S + 0x2a0` =
///   slot 248 of `(0, 0, 0, 1)` and slot 229 `(W, M, V)` (`W` = `E +
///   0x38`); the pitch `atan2(V.y, sqrt(V.z^2 + V.x^2))` and the yaw `K5 -
///   atan2(V.z, V.x)` wrapped once by a turn into `[KC2, KB2]` turn the
///   matrix `N` (`E + 0x88`, slots 235, 328, 329) applied to `(0, 0, -0.7,
///   1)` into `S + 0x2b0` (slots 229, 248); `S + 0x270..` = `S + 0x2a0..`
///   with `w` = its length; `S.290 = (slot 314 (0.1) + K6) * K7`, `S.294 =
///   (slot 404 (0.1) + K6) * K8`, `S.298` = 0, the countdown 0x28 and the
///   task's callback `CB3`.
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// [`light_setup`]'s `eax` while counting down, else `task[8]`.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn actor_light<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1ac), n);
    let task = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0xd0);
    let at = |o: u32| e.wrapping_add(o);
    let gobj = g.read::<u32>(w(0x1e));
    g.write::<u32>(at(8), gobj);
    g.write::<u32>(at(0x1c), task);
    let s = g.read::<u32>(task.wrapping_add(0x14));
    let sf = |o: u32| s.wrapping_add(o);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let n = g.read::<u32>(sf(0x50c));
    if n.cast_signed() > 0 {
        g.write::<u32>(sf(0x50c), n.wrapping_sub(1));
    }
    let u = div(cvt(g.read::<u32>(sf(0x50c))), cvt(g.read::<u32>(sf(0x510))));
    let rest = sub(k(g, 0x5a), u);
    let v = at(0x28);
    for (i, (a, b)) in [(0x260u32, 0x270u32), (0x264, 0x274), (0x268, 0x278)]
        .into_iter()
        .enumerate()
    {
        let x = add(mul(rd(g, sf(a)), u), mul(rest, rd(g, sf(b))));
        g.write::<f32>(v.wrapping_add(4 * i as u32), x);
    }
    g.write::<u32>(v.wrapping_add(0xc), w(0xcf));
    let callee = stack_args(
        g,
        img.rel32_target(f + 0xde),
        e.wrapping_sub(8),
        sf(0x280),
        v,
        &[w(0x79)],
    );
    let light = light_setup(g, img, &callee);
    if g.read::<u32>(sf(0x50c)).cast_signed() > 0 {
        return light;
    }
    let callee = Frame {
        at: img.rel32_target(f + 0xfb),
        esp: e.wrapping_sub(4),
        ecx: s,
        edx: sf(0x280),
        ..Frame::default()
    };
    crate::o6_fam_pairs2::actor_object_start(g, img, &callee);
    let gobj = g.read::<u32>(at(8));
    if g.read::<u32>(gobj.wrapping_add(w(0x105))) == 1 {
        let glow = gobj.wrapping_add(w(0x112));
        for (o, x) in [(0x80u32, 0x121u32), (0x84, 0x12b), (0x88, 0x135)] {
            g.write::<u32>(glow.wrapping_add(o), w(x));
        }
        g.write::<u16>(glow.wrapping_add(0x98), w(0x140) as u16);
        for (o, x) in [(0x90u32, 0x148u32), (0x8c, 0x152), (0x94, 0x15c)] {
            g.write::<u32>(glow.wrapping_add(o), w(x));
        }
        let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x161), w(0x117));
        let first = g.read::<u32>(w(0x167));
        g.write::<u32>(node.wrapping_add(0x14), glow);
        g.write::<u32>(node.wrapping_add(4), 0);
        if first != 0 {
            let last = g.read::<u32>(w(0x17a));
            g.write::<u32>(node, last);
            g.write::<u32>(last.wrapping_add(4), node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0x18c), node);
        }
        g.write::<u32>(node.wrapping_add(8), node);
        let period = g.read::<u32>(sf(0x510)).wrapping_add(disp8(img, f + 0x199));
        g.write::<u32>(sf(0x510), period);
        g.write::<u32>(w(0x19b), node);
        let period = g.read::<u32>(sf(0x510));
        g.write::<u32>(sf(0x50c), period);
        let x = g.read::<u32>(w(0x1c1));
        let t = slot(g, 268);
        // call width: 3
        g.call(t, s, &[x, disp8(img, f + 0x1be), sf(0x260)]);
        g.write::<u32>(sf(0x26c), w(0x1ce));
        let cb = g.read::<u32>(task.wrapping_add(8));
        g.write::<u32>(cb.wrapping_add(0x10), w(0x1db));
        return Ret::eax(cb);
    }
    for i in 0..4u32 {
        let x = g.read::<u32>(v.wrapping_add(4 * i));
        g.write::<u32>(sf(0x260 + 4 * i), x);
    }
    for (o, x) in [
        (0x2a0u32, 0u32),
        (0x2a4, 0),
        (0x2a8, 0),
        (0x2ac, 0x3f80_0000),
    ] {
        g.write::<u32>(sf(o), x);
    }
    let mut st = Pushes::default();
    st.push(0);
    let st0 = |g: &mut G, st: &mut Pushes, n: u32, x: u32| {
        st.set_top(x);
        let t = slot(g, n);
        // call width: 12 (Pushes caps the words at 12)
        st0_f32(st.call(g, t, s).st0)
    };
    let vx = st0(g, &mut st, 314, w(0x262));
    g.write::<f32>(v, vx);
    let t2 = st0(g, &mut st, 314, w(0x274));
    let y = add(t2, k(g, 0x28e));
    let neg = |g: &G, x: f32, o: u32| f32::from_bits(x.to_bits() ^ g.read::<u32>(w(o)));
    g.write::<f32>(v.wrapping_add(4), neg(g, y, 0x2a1));
    let t3 = st0(g, &mut st, 314, w(0x29a));
    let ka = k(g, 0x2c1);
    let x = add(t3, ka);
    let x = if k(g, 0x2c9) > x && x > k(g, 0x2dc) {
        ka
    } else {
        x
    };
    let z = neg(g, mul(x, k(g, 0x2e9)), 0x2f5);
    let m = at(0x48);
    let mut st = Pushes::default();
    st.push(m);
    g.write::<f32>(v.wrapping_add(8), z);
    g.write::<u32>(v.wrapping_add(0xc), w(0x308));
    let t = slot(g, 235);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, s);
    let wv = at(0x38);
    // slots 328 and 329 on the matrix, each angle read when it is used
    let turn =
        |g: &mut G, st: &mut Pushes, mtx: u32, a: &dyn Fn(&G) -> u32, b: &dyn Fn(&G) -> u32| {
            for (x, n) in [(a, 328u32), (b, 329)] {
                if n == 329 {
                    st.pop(2);
                }
                st.set_top(x(g));
                st.push_all(&[mtx, mtx]);
                let t = slot(g, n);
                // call width: 12 (Pushes caps the words at 12)
                st.call(g, t, s);
            }
        };
    turn(
        g,
        &mut st,
        m,
        &|g: &G| g.read::<u32>(sf(0x280)),
        &|g: &G| g.read::<u32>(sf(0x284)),
    );
    st.push_all(&[v, m, wv]);
    g.write::<u32>(v.wrapping_add(0xc), w(0x36f));
    let t = slot(g, 229);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, s);
    st.push_all(&[wv, sf(0x2a0), sf(0x2a0)]);
    let t = slot(g, 248);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, s);
    g.write::<u32>(sf(0x2ac), w(0x39b));
    let zero = 0.0f32;
    let dy = sub(rd(g, v.wrapping_add(4)), zero);
    let dz = sub(rd(g, v.wrapping_add(8)), zero);
    let dx = sub(rd(g, v), zero);
    let flat = add(mul(dz, dz), mul(dx, dx));
    let pitch = g.atan2(f64::from(dy), f64::from(flat).sqrt());
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: round to f32
    let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let aim = wrap_turn(sub(k(g, 0x443), yaw), k(g, 0x44e), k(g, 0x468), k(g, 0x45e));
    let v2 = v;
    for (i, o) in (0u32..).zip([0x48au32, 0x492, 0x49a, 0x4a2]) {
        g.write::<u32>(v2.wrapping_add(4 * i), w(o));
    }
    let nm = at(0x88);
    let mut st = Pushes::default();
    st.push(nm);
    for (o, x) in [
        (0x2b0u32, 0x4bau32),
        (0x2b4, 0x4c4),
        (0x2b8, 0x4ce),
        (0x2bc, 0x4d8),
    ] {
        g.write::<u32>(sf(o), w(x));
    }
    let t = slot(g, 235);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, s);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: round to f32
    let pitch = pitch as f32;
    turn(g, &mut st, nm, &|_: &G| pitch.to_bits(), &|_: &G| {
        aim.to_bits()
    });
    st.push_all(&[v2, nm, wv]);
    g.write::<u32>(v2.wrapping_add(0xc), w(0x542));
    let t = slot(g, 229);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, s);
    st.push_all(&[wv, sf(0x2b0), sf(0x2b0)]);
    let t = slot(g, 248);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, s);
    g.write::<u32>(sf(0x2bc), w(0x565));
    for i in 0..4u32 {
        let x = g.read::<u32>(sf(0x2a0 + 4 * i));
        g.write::<u32>(sf(0x270 + 4 * i), x);
    }
    let (a0, a4, a8) = (rd(g, sf(0x2a0)), rd(g, sf(0x2a4)), rd(g, sf(0x2a8)));
    let len2 = add(add(mul(a0, a0), mul(a4, a4)), mul(a8, a8));
    // add esp, 0x20: one word of the nine left
    st.pop(8);
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss: round to f32
    g.write::<f32>(sf(0x27c), f64::from(len2).sqrt() as f32);
    let t = st0(g, &mut st, 314, w(0x5ea));
    st.set_top(w(0x5f3));
    g.write::<f32>(sf(0x290), mul(add(t, k(g, 0x605)), k(g, 0x60d)));
    let t = st0(g, &mut st, 404, w(0x5f3));
    g.write::<u32>(sf(0x298), w(0x642));
    g.write::<f32>(sf(0x294), mul(add(t, k(g, 0x634)), k(g, 0x64d)));
    g.write::<u32>(sf(0x50c), w(0x65f));
    let cb = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(cb.wrapping_add(0x10), w(0x669));
    Ret::eax(cb)
}

/// The thunk of [`two_part_visit`] (`magic_0070` 0x100014a0).
pub fn two_part_visit_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::o6_fam_pairs3::thunk_to(g, img, fr, two_part_visit::<G>)
}

include!("o6_fam_pairs5_table.rs.in");
