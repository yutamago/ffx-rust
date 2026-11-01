//! Round 6 shape families: more exact pairs (two DLLs' copies of one
//! function that differ only in their globals and immediates; families
//! of `~/ffx-work/m5/magic/r4/families.tsv`). Each port reads its
//! operands from the copy's code (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Takes `node` out of its parent `parent`'s child list (`parent != 0`):
/// the parent's u16 count `+0x18` drops by one, the first (`+0x10`) or
/// the previous sibling's next and the last (`+0xc`) or the next
/// sibling's previous are joined around it (each link re-read), and
/// `node`'s links (`+4`, `+8`) point at itself. `+0x14` is the caller's.
fn unlink_child<G: Game>(g: &mut G, node: u32, parent: u32) {
    let n = g.read::<u16>(parent.wrapping_add(0x18));
    g.write::<u16>(parent.wrapping_add(0x18), n.wrapping_add(0xffff));
    let prev = g.read::<u32>(node.wrapping_add(4));
    let next = g.read::<u32>(node.wrapping_add(8));
    if prev == parent {
        g.write::<u32>(parent.wrapping_add(0x10), next);
    } else {
        g.write::<u32>(prev.wrapping_add(8), next);
    }
    let next = g.read::<u32>(node.wrapping_add(8));
    let prev = g.read::<u32>(node.wrapping_add(4));
    if next == parent {
        g.write::<u32>(parent.wrapping_add(0xc), prev);
    } else {
        g.write::<u32>(next.wrapping_add(4), prev);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(node.wrapping_add(4), node);
}

/// Detaches a summon's models (internal: `ecx` = a record whose `+0`
/// is the summon `s`; an `ebp` frame of 8 bytes and three saved
/// registers; `magic_0164` 0x10007cf0 and `magic_0155` 0x10007c70, 248
/// bytes; family `F0164_10007cf0`): for each of the `s + 0x162c` models
/// (records of 0x274 bytes from `s + 0xbc`), the model node leaves its
/// parent (`+0x14`, then cleared), the model is set up again
/// ([`crate::sa_summon::model_setup`] with a cursor on the frame at
/// `ebp - 4`), and its second node (`+0x98`) leaves its parent (`+0xac`,
/// then cleared). Returns `s` with no models, else 0xffff.
pub fn models_detach<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = g.read::<u32>(fr.ecx);
    let ebp = fr.esp.wrapping_sub(4);
    let count = s.wrapping_add(img.word(f + 0x17));
    if g.read::<i32>(count) <= 0 {
        return Ret::eax(s);
    }
    let cursor = ebp.wrapping_sub(4);
    let mut model = s.wrapping_add(img.word(f + 0x11));
    let mut i = 0i32;
    loop {
        let parent = g.read::<u32>(model.wrapping_add(0x14));
        if parent != 0 {
            unlink_child(g, model, parent);
            g.write::<u32>(model.wrapping_add(0x14), 0);
        }
        g.write::<u32>(cursor, model);
        let setup = Frame {
            at: img.rel32_target(f + 0x8b),
            // sub esp, 8; three pushes; the call's return address
            esp: ebp.wrapping_sub(8 + 12 + 4),
            ecx: cursor,
            ..*fr
        };
        crate::sa_summon::model_setup(g, img, &setup);
        let second = model.wrapping_add(0x98);
        let parent = g.read::<u32>(second.wrapping_add(0x14));
        if parent != 0 {
            unlink_child(g, second, parent);
            g.write::<u32>(second.wrapping_add(0x14), 0);
        }
        i += 1;
        model = model.wrapping_add(img.word(f + 0xd6));
        if i >= g.read::<i32>(count) {
            return Ret::eax(0xffff);
        }
    }
}

/// [`crate::o6_fam_pairs::angle_reset`] after a sound list (no arguments;
/// `magic_0348` 0x10006600 and `magic_0350` 0x10005a50, 282 bytes;
/// family `F0348_10006600`): with `B` = `*G_B` (`+0x09`), slot 288 `(9,
/// &n)` gives a byte list (`n` a local at `ebp - 4`) and slot 296 `(c,
/// 0)` runs for each of its `n` bytes `c`; then `B + 0xd10` = 0; the
/// angle `x = K1 - slot 333 (*G_A)` (`G_A` at `+0x5f`) brought into range
/// once (`x > K2`: `x - K3`; else `K4 > x`: `x + K3`) into `B + 0xd14`,
/// `+0xd18`/`+0xd1c` 0; for `V = B + 0xd20`, slot 262 `(*G_A, V, V + 4,
/// V + 8)` and slot 12 `(*G_A)` (its float into `V + 4`), `V + 0xc` =
/// 1.0; `B + 0xd180` 0 and `*G_DONE` = 1. Returns `B`.
pub fn list_angle_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = w(0x14);
    let b = g.read::<u32>(w(0x09));
    let n_at = fr.esp.wrapping_sub(8);
    // call width: 2
    let list = g
        .call(
            slot_target(g, t, w(0x1c) / 4),
            fr.ecx,
            &[disp8(img, f + 0x19), n_at],
        )
        .eax;
    let mut i = 0i32;
    while i < g.read::<i32>(n_at) {
        let c = u32::from(g.read::<u8>(list.wrapping_add(i.cast_unsigned())));
        // call width: 2
        g.call(
            slot_target(g, w(0x36), w(0x3e) / 4),
            fr.ecx,
            &[c, disp8(img, f + 0x3b)],
        );
        i += 1;
    }
    g.write::<u32>(b.wrapping_add(w(0x55)), w(0x59));
    let a = g.read::<u32>(w(0x5f));
    // call width: 1
    let out = g.call(slot_target(g, w(0x4f), w(0x65) / 4), fr.ecx, &[a]);
    let r = f32::from_bits(crate::sa_summon::st0_f32(out));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let mut x = sub(k(g, 0x72), r);
    if x > k(g, 0x81) {
        x = sub(x, k(g, 0x8b));
    } else if k(g, 0x95) > x {
        x = add(x, k(g, 0xa2));
    }
    g.write::<f32>(b.wrapping_add(w(0xaa)), x);
    g.write::<u32>(b.wrapping_add(w(0xb0)), w(0xb4));
    g.write::<u32>(b.wrapping_add(w(0xba)), w(0xbe));
    let a = g.read::<u32>(w(0xc4));
    let v = b.wrapping_add(w(0xca));
    let (v8, v4) = (
        v.wrapping_add(disp8(img, f + 0xd0)),
        v.wrapping_add(disp8(img, f + 0xd9)),
    );
    let mut stack = vec![a, v, v4, v8];
    // call width: 4
    g.call(slot_target(g, w(0xd3), w(0xdc) / 4), fr.ecx, &stack);
    stack.insert(0, a);
    // call width: 5 (the earlier call's words stay on the stack)
    let out = g.call(
        slot_target(g, w(0xe6), disp8(img, f + 0xed) / 4),
        fr.ecx,
        &stack,
    );
    g.write::<u32>(v4, crate::sa_summon::st0_f32(out));
    g.write::<u32>(v.wrapping_add(disp8(img, f + 0xfa)), w(0xfb));
    g.write::<u32>(b.wrapping_add(w(0x103)), w(0x107));
    g.write::<u32>(w(0x10d), w(0x111));
    Ret::eax(b)
}

/// Starts an actor's effect object (internal: `ecx` = the actor `a`,
/// `edx` a position `pos`; `magic_0644` 0x10008df0 and `magic_0565`
/// 0x10004dd0, 295 bytes; family `F0644_10008df0`): `k` = slot 641
/// `(*G_K)` (`+0x10`); the object `o = a + 0x130` is started
/// ([`crate::s4_misc::start_object_at`]) with the scale `SCALES[k]` and
/// the program index `INDEX[k]` (tables at `+0x1f`, `+0x37`), owner 0 and
/// the callback `CB` (`+0x2d`); `o + 0xd0..` = `a + 0x270..`, `o + 0xc0`,
/// `+0xc4` = `pos[0]`, `pos[1]`; `o + 0x120` = `*G_K` itself when
/// `a[0x514]` is -1, else the byte `(*G_T)[a[0x514]]`; a handle `h` = slot 719 `(P, 9,
/// 1)` and slot 306 `(7, f, h)` (`f` 1 when `(*G_M)[0xd618]` is set,
/// else 0); then `h2` = slot 719 `(Q, 6, s)` (`s` 0xb or 0xd the same
/// way) and slot 393 `(h2, o + 0xd0, 0x100)`. Each call sees the words
/// the earlier ones left. Returns slot 393's `eax`.
pub fn actor_object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (a, pos) = (fr.ecx, fr.edx);
    let t = w(0x01);
    // call width: 1
    let k = g
        .call(
            slot_target(g, t, w(0x08) / 4),
            fr.ecx,
            &[g.read::<u32>(w(0x10))],
        )
        .eax;
    let scale = g.read::<f32>(w(0x1f).wrapping_add(k.wrapping_mul(4)));
    let index = g.read::<u32>(w(0x37).wrapping_add(k.wrapping_mul(4)));
    let o = a.wrapping_add(w(0x28));
    crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + 0x3e),
        o,
        scale,
        index,
        0,
        w(0x2d),
    );
    for i in 0..4u32 {
        let v = g.read::<u32>(a.wrapping_add(w(0x44 + 12 * i)));
        g.write::<u32>(o.wrapping_add(w(0x4a + 12 * i)), v);
    }
    let v = g.read::<u32>(pos);
    g.write::<u32>(o.wrapping_add(w(0x76)), v);
    let v = g.read::<u32>(pos.wrapping_add(4));
    g.write::<u32>(o.wrapping_add(w(0x7f)), v);
    let i = g.read::<u32>(a.wrapping_add(w(0x85)));
    // -1: the word `*G_K` itself (the jump skips the byte load)
    let v = if i == u32::MAX {
        g.read::<u32>(w(0x92))
    } else {
        let table = g.read::<u32>(w(0x99));
        u32::from(g.read::<u8>(table.wrapping_add(i)))
    };
    g.write::<u32>(o.wrapping_add(w(0xa3)), v);
    let set = g.read::<u32>(g.read::<u32>(w(0xa8)).wrapping_add(w(0xb0))) != 0;
    let (s, flag) = if set {
        (w(0xd6), disp8(img, f + 0xde))
    } else {
        (w(0xca), disp8(img, f + 0xd2))
    };
    let mut stack = vec![w(0xc3), disp8(img, f + 0xbb), disp8(img, f + 0xad)];
    // call width: 3
    let h = g
        .call(slot_target(g, w(0xb6), w(0xbe) / 4), fr.ecx, &stack)
        .eax;
    stack.splice(0..0, [disp8(img, f + 0xe5), flag, h]);
    // call width: 6 (the earlier call's words stay on the stack)
    g.call(slot_target(g, w(0xe0), w(0xe8) / 4), fr.ecx, &stack);
    let mut stack = vec![w(0x100), disp8(img, f + 0xfe), s];
    // call width: 3
    let h2 = g
        .call(slot_target(g, w(0xef), w(0xf8) / 4), fr.ecx, &stack)
        .eax;
    stack.splice(0..0, [h2, o.wrapping_add(w(0x10d)), w(0x107)]);
    // call width: 6 (the earlier call's words stay on the stack)
    let eax = g
        .call(slot_target(g, w(0x114), w(0x11a) / 4), fr.ecx, &stack)
        .eax;
    Ret::eax(eax)
}

/// A shaking node's visit (internal: `ecx` = the group; `magic_0201`
/// 0x10004780 and `magic_0387` 0x10005eb0, 337 bytes; family
/// `F0201_10004780`; reached through the 12-byte thunks
/// [`shake_visit_thunk`]): the node walk ([`crate::nodes::walk`]); when
/// it ran and the owner's flags byte `+0x30` has bit 0 (without it,
/// `magic_0387`'s copy calls slot 250 `(0, 0)`), the timer `+0x2c`
/// counts down by `K` (`+0x6a`). While it was not 0 (NaN included) the
/// camera shakes: `r = slot 307 - K2` (ST0, `K2` at `+0x95`), two
/// signs from slot 324 (`1 - (eax & 2)`), and slot 250 gets `((+0x1c +
/// +0x20 * r) * s1, (+0x24 + +0x28 * r) * s2)`. At 0 the flag's bit 0
/// clears; with bit 1 the node leaves its parent
/// ([`crate::sa_misc::leave`]) and slot 250 gets `(0, 0)`. Returns the
/// last call's `eax`, or the walk's with the flags byte in its low byte
/// (and, after the timer, the comparison's `lahf` flags 0x42 in `ah`).
pub fn shake_visit<G: Game>(g: &mut G, img: &Image, f: u32, group: u32, eax: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let (eax, _) = crate::nodes::walk(g, group, group, eax);
    if g.read::<u8>(group.wrapping_add(4)) & 1 == 0 {
        return eax;
    }
    let owner = g.read::<u32>(group);
    let flags = g.read::<u8>(owner.wrapping_add(0x30));
    let eax = (eax & !0xff) | u32::from(flags);
    if flags & 1 == 0 {
        // the copies' one difference: `magic_0387`'s `je` (rel32 at
        // +0x57) goes to the `(0, 0)` call instead of the end
        if img.rel32_target(f + 0x57).wrapping_sub(f) != 0x128 {
            return eax;
        }
        // call width: 2
        return g
            .call(slot_target(g, w(0x13b), w(0x141) / 4), group, &[0, 0])
            .eax;
    }
    let timer = g.read::<f32>(owner.wrapping_add(0x2c));
    let t = w(0x7e);
    g.write::<f32>(owner.wrapping_add(0x2c), sub(timer, g.read::<f32>(w(0x6a))));
    let (x, y) = if timer == 0.0 {
        let flags = flags & 0xfe;
        g.write::<u8>(owner.wrapping_add(0x30), flags);
        if flags & 2 == 0 {
            // `ucomiss` with 0 then `lahf`: equal leaves ZF (0x40) and the
            // always-set bit 1 in `ah`
            return (eax & 0xffff_0000) | 0x4200 | u32::from(flags);
        }
        crate::sa_misc::leave(g, owner, eax);
        (0, 0)
    } else {
        // call width: 0
        let out = g.call(slot_target(g, t, w(0x84) / 4), group, &[]);
        let r = sub(
            f32::from_bits(crate::sa_summon::st0_f32(out)),
            g.read::<f32>(w(0x96)),
        );
        let field = |g: &G, o: u32| g.read::<f32>(owner.wrapping_add(o));
        // call width: 0
        let e1 = g.call(slot_target(g, w(0x9b), w(0xa1) / 4), group, &[]).eax;
        let x = add(mul(field(g, 0x20), r), field(g, 0x1c));
        let x = mul(x, cvt(1u32.wrapping_sub(e1 & 2)));
        // call width: 0
        let e2 = g.call(slot_target(g, w(0xc8), w(0xd2) / 4), group, &[]).eax;
        let y = add(mul(field(g, 0x28), r), field(g, 0x24));
        let y = mul(y, cvt(1u32.wrapping_sub(e2 & 2)));
        (x.to_bits(), y.to_bits())
    };
    // call width: 2
    g.call(slot_target(g, w(0x13b), w(0x141) / 4), group, &[x, y])
        .eax
}

/// The 12-byte thunk of [`shake_visit`] (cdecl `(group)`: `mov ecx,
/// [ebp + 8]` and a tail jump; `magic_0201` 0x10004910, `magic_0387`
/// 0x10006040).
pub fn shake_visit_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    Ret::eax(shake_visit(
        g,
        img,
        img.rel32_target(fr.at + 8),
        group,
        fr.eax,
    ))
}

include!("o6_fam_pairs2_table.rs.in");
