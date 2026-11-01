//! Round 11 (magic-r11-own): start-ups with one object per actor, the
//! kin of [`objects_start`](crate::r9_misc::objects_start), that take
//! three task nodes inline from the free ring.

use crate::r9_misc::{ObjectStart, actor_objects};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Unlinks ring node `n` (its neighbours point past it).
fn unlink<G: Game>(g: &mut G, n: u32) {
    let (next, prev) = (g.read::<u32>(n), g.read::<u32>(n + 4));
    g.write::<u32>(next + 4, prev);
    let (prev, next) = (g.read::<u32>(n + 4), g.read::<u32>(n));
    g.write::<u32>(prev, next);
}

/// A taken node's fresh words: links 0, `+8` itself, `+0x10` the owner,
/// byte `+0x18` 0.
fn clear<G: Game>(g: &mut G, n: u32, owner: u32) {
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(n + o, 0);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 0x10, owner);
    g.write::<u8>(n + 0x18, 0);
}

/// `mov dword [G], imm32` at each instruction offset of `ats` (operand
/// `+2`, immediate `+6`).
fn stores<G: Game>(g: &mut G, img: &Image, f: u32, ats: impl IntoIterator<Item = u32>) {
    for at in ats {
        g.write::<u32>(img.word(f + at + 2), img.word(f + at + 6));
    }
}

/// The start-ups' head (`f` where [`three_nodes`]' code starts; a /GS
/// frame's set-up comes before it in the others): `*G_A`/`*G_B` set
/// (`+0x1f`, `+0x29`), slot 719 `(NAME, 8, 0)` into `G_PART` (`+0x37`),
/// the part set up ([`part_init`](crate::l5_objhost::part_init), `rel32`
/// `+0x3c`).
fn head<G: Game>(g: &mut G, img: &Image, f: u32) {
    head_with(g, img, f, None);
}

/// [`head`] whose `rel32` callee is the 0x50000 parts buffer
/// ([`parts_buffer`](crate::s4_small::parts_buffer), run on the frame
/// `fr`) instead of `part_init`.
fn head_with<G: Game>(g: &mut G, img: &Image, f: u32, buffer: Option<&Frame>) {
    let w = |o: u32| img.word(f + o);
    let s719 = slot_target(g, w(0x07), 0xb3c / 4);
    g.write::<u32>(w(0x1f), w(0x23));
    g.write::<u32>(w(0x29), w(0x2d));
    let part = g.call(s719, 0, &[w(0x19), 8, 0]).eax;
    g.write::<u32>(w(0x37), part);
    let at = img.rel32_target(f + 0x3c);
    match buffer {
        None => crate::l5_objhost::part_init(g, img, at),
        Some(fr) => {
            crate::s4_small::parts_buffer(g, img, &Frame { at, ..*fr });
        }
    }
}

/// The scale of an actor's object in [`three_nodes_kind`] and
/// [`three_nodes_scaled`]: `(slot 16 (chr) * slot 364 (chr)) / D`
/// clamped to `[LO, HI]` (`comiss` + `ja`/`jbe`: a NaN stays), times `K`
/// (operands `+d`, `+lo`, `+hi`, `+k`).
fn actor_scale<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    chr: u32,
    (t364, t16): (u32, u32),
    (d, lo, hi, k): (u32, u32, u32, u32),
) -> f32 {
    use crate::m0018::st0_f32;
    use crate::sb_sse::{div, mul};
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let mut s = crate::pushes::Pushes::default();
    s.push(chr);
    // call width: 12 (Pushes caps the words at 12)
    let a = st0_f32(s.call(g, slot_target(g, w(t364), 0x5b0 / 4), 0).st0);
    s.push(chr);
    // call width: 12 (Pushes caps the words at 12)
    let b = st0_f32(s.call(g, slot_target(g, w(t16), 0x40 / 4), 0).st0);
    let x = div(mul(b, a), kf(g, d));
    let x = if kf(g, lo) > x {
        kf(g, lo)
    } else if x > kf(g, hi) {
        kf(g, hi)
    } else {
        x
    };
    mul(x, kf(g, k))
}

/// [`three_nodes`] with the parts buffer, scaled objects and a callback
/// per actor count (`magic_0126` 0x10001460, 1401 bytes, a frame of 0x14):
/// [`head_with`] the buffer; `CB` = `+0x4e` for one actor, `+0x53`
/// otherwise; for each of the `*G_N` (`+0x41`, read once) actors `i`
/// (`chr` = `CHR[SEL[i]]`, `+0x89`, `+0x7e`): the scale ([`actor_scale`]),
/// the actor's kind ([`actor_kind_check`](crate::sa_misc::actor_kind_check),
/// `rel32` `+0xee`); the object `OBJ + 0x130 i` (`+0x104`) starts
/// (`o6_fam_object::start_object_first`, `(kind != 0, owner 15 i)`); its
/// point `p = obj + 0xd0` from slots 262 and 268 (`chr` again: `+0x13b`,
/// `+0x130`), `p + 0xc` 1.0; `obj + 0x120` the actor's byte (`SEL`
/// `+0x16e`); the record `REC + 0x34 i` (`+0x1a4`, `+0x19a` cleared first;
/// `+0x24` = `CB`, `+0x20` the immediate `+0x1cb`) into `obj + 0x110`;
/// then `y = p.y + K` (`+0x212`) into `p + 4`, or slot 12 `(chr)`'s
/// height less `K2` (`+0x22f`) when `y` is above it. The three nodes
/// follow 0xec bytes later than in [`three_nodes`].
#[allow(clippy::too_many_lines)]
pub fn three_nodes_kind<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; sub esp, 0x14; push ebx/esi/edi; two words left: the
    // buffer's frame
    let ebp = fr.esp.wrapping_sub(4);
    let buffer = Frame {
        esp: ebp - 0x14 - 0xc - 8 - 4,
        ..*fr
    };
    head_with(g, img, f, Some(&buffer));
    let n = g.read::<u32>(w(0x41));
    let callback = if n == 1 { w(0x4e) } else { w(0x53) };
    let mut owner = 0u32;
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let sel = g.read::<u32>(w(0x7e));
        let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        let chr = g.read::<u32>(w(0x89).wrapping_add(4 * b));
        let scale = actor_scale(g, img, f, chr, (0x8e, 0x9f), (0xc5, 0xba, 0xd2, 0xe2));
        g.write::<f32>(ebp - 4, scale);
        let kind = crate::sa_misc::actor_kind_check(g, img, img.rel32_target(f + 0xee), i);
        let obj = w(0x104).wrapping_add(i.wrapping_mul(0x130));
        // sub esp, 8; push owner; push kind or 1; call
        let esp = ebp - 0x14 - 0xc - 0x14;
        g.write::<u32>(esp + 4, u32::from(kind != 0));
        g.write::<u32>(esp + 8, owner);
        let at = img.rel32_target(f + 0x111);
        let call = Frame {
            at,
            esp,
            edx: obj,
            xmm: [0, 0, 0, u128::from(scale.to_bits())],
            ..*fr
        };
        crate::o6_fam_object::start_object_first(g, img, at, &call);
        let p = obj + 0xd0;
        let sel = g.read::<u32>(w(0x130));
        let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        let chr = g.read::<u32>(w(0x13b).wrapping_add(4 * b));
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[p + 8, p + 4, p, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x148), 0x418 / 4), 0);
        s.push_all(&[p, 0x11, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x15a), 0x430 / 4), 0);
        let sel = g.read::<u32>(w(0x16e));
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        g.write::<u32>(obj + 0x120, b);
        let off = i.wrapping_mul(0x34);
        g.write::<u32>(w(0x19a).wrapping_add(off), 0);
        let rec = w(0x1a4).wrapping_add(off);
        g.write::<u32>(obj + 0x110, rec);
        for o in [0x14, 0x10, 0x1c] {
            g.write::<u32>(rec + o, 0);
        }
        g.write::<u32>(rec + 0x24, callback);
        g.write::<u32>(rec + 0x20, w(0x1cb));
        g.write::<u32>(rec + 0x28, 0);
        g.write::<u32>(rec + 0x2c, 0);
        g.write::<u16>(rec + 0x30, 0);
        let chr = g.read::<u32>(w(0x1f4).wrapping_add(4 * b));
        let mut s = crate::pushes::Pushes::default();
        s.push(chr);
        // call width: 12 (Pushes caps the words at 12)
        let top = s.call(g, slot_target(g, w(0x1fa), 0x30 / 4), 0).st0;
        let top = crate::m0018::st0_f32(top);
        g.write::<f32>(ebp - 0xc, top);
        let y = add(g.read::<f32>(p + 4), g.read::<f32>(w(0x212)));
        g.write::<f32>(p + 4, y);
        if y > top {
            g.write::<f32>(p + 4, sub(top, g.read::<f32>(w(0x22f))));
        }
        owner = owner.wrapping_add(0xf);
        i = i.wrapping_add(1);
    }
    nodes_tail(g, img, f + 0xec, End::Split)
}

/// [`three_nodes_kind`] (`magic_0126` 0x10001460).
pub const THREE_NODES_KIND_0126: Sig = Sig {
    len: 1401,
    hash: 0xb063_b3b1_2f80_7591,
    calls: &[
        (0x3c, Callee::Sig(&crate::s4_small::PARTS_BUFFER_50000)),
        (0xee, Callee::Sig(&crate::sa_misc::ACTOR_KIND_CHECK)),
        (
            0x111,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0126_22A0),
        ),
    ],
    imports: &[],
};

/// [`three_nodes_kind`] without the kind check and the height (`magic_0127`
/// 0x10002d30, 1312 bytes): per actor the scale ([`actor_scale`]: `chr`
/// = `CHR[SEL[i]]`, `+0x87`, `+0x6f`), the object `OBJ + 0x130 i`
/// (`+0xf1`) started ([`start_object_first`](crate::s4_misc::start_object_first),
/// owner `15 i`), slots 262 and 268 for its point (`+0x120`, `+0x115`),
/// `p + 0xc` 1.0 and `p.y + K` (`+0x173`), its byte (`SEL` `+0x154`) and
/// record (`+0x1a8`, `+0x19e`; `CB` from `+0x4e`/`+0x53`, the immediate
/// `+0x1cd`). The three nodes follow 0x93 bytes later.
pub fn three_nodes_scaled<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let buffer = Frame {
        esp: ebp - 0x14 - 0xc - 8 - 4,
        ..*fr
    };
    head_with(g, img, f, Some(&buffer));
    let n = g.read::<u32>(w(0x41));
    let callback = if n == 1 { w(0x4e) } else { w(0x53) };
    let mut owner = 0u32;
    let mut i = 0u32;
    let mut sel = g.read::<u32>(w(0x6f));
    while i.cast_signed() < n.cast_signed() {
        let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        let chr = g.read::<u32>(w(0x87).wrapping_add(4 * b));
        let scale = actor_scale(g, img, f, chr, (0x8c, 0x9d), (0xc3, 0xb8, 0xd0, 0xe0));
        let obj = w(0xf1).wrapping_add(i.wrapping_mul(0x130));
        // sub esp, 8; push owner; push ecx; call (the start takes the
        // owner as an argument: the words are below the entry)
        let at = img.rel32_target(f + 0xfa);
        crate::s4_misc::start_object_first(g, img, at, obj, scale, owner);
        let p = obj + 0xd0;
        let sel1 = g.read::<u32>(w(0x115));
        let b = u32::from(g.read::<u8>(sel1.wrapping_add(i)));
        let chr = g.read::<u32>(w(0x120).wrapping_add(4 * b));
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[p + 8, p + 4, p, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x12d), 0x418 / 4), 0);
        s.push_all(&[p, 0x11, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x143), 0x430 / 4), 0);
        sel = g.read::<u32>(w(0x154));
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        let y = add(g.read::<f32>(p + 4), g.read::<f32>(w(0x173)));
        g.write::<f32>(p + 4, y);
        let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        g.write::<u32>(obj + 0x120, b);
        let off = i.wrapping_mul(0x34);
        g.write::<u32>(w(0x19e).wrapping_add(off), 0);
        let rec = w(0x1a8).wrapping_add(off);
        for o in [0x14, 0x10, 0x1c] {
            g.write::<u32>(rec + o, 0);
        }
        g.write::<u32>(rec + 0x24, callback);
        g.write::<u32>(rec + 0x20, w(0x1cd));
        g.write::<u32>(rec + 0x28, 0);
        g.write::<u32>(rec + 0x2c, 0);
        g.write::<u16>(rec + 0x30, 0);
        g.write::<u32>(obj + 0x110, rec);
        owner = owner.wrapping_add(0xf);
        i = i.wrapping_add(1);
    }
    nodes_tail(g, img, f + 0x93, End::Split)
}

/// [`three_nodes_scaled`] (`magic_0127` 0x10002d30).
pub const THREE_NODES_SCALED_0127: Sig = Sig {
    len: 1312,
    hash: 0xf5cb_a52d_f88d_a4ac,
    calls: &[
        (0x3c, Callee::Sig(&crate::s4_small::PARTS_BUFFER_50000)),
        (0xfa, Callee::Sig(&crate::s4_misc::START_OBJECT_FIRST)),
    ],
    imports: &[],
};

/// The start-up with three task nodes (internal, no arguments; 1165
/// bytes, `magic_0110` 0x10001410, `0112` 0x10001410, `0121`
/// 0x10002ec0; they differ in immediates): `*G_A`/`*G_B` set (`+0x1f`,
/// `+0x29`), slot 719 `(NAME, 8, 0)` into `G_PART` (`+0x37`), the part
/// set up ([`part_init`](crate::l5_objhost::part_init)); the actors'
/// objects ([`actor_objects`]: the first owner the immediate `+0x46` in
/// `magic_0110`, 0 in the others); twelve words set; then three nodes
/// off the free ring (count `u16 +0x165`, head `+0x1ed`):
/// - the first (owner `+0x241`; byte `+0x18` 1, `+0x14` the immediate
///   `+0x25d`) goes last on the task list (first `+0x252`, last `+0x26d`)
///   without becoming its last;
/// - after eleven more words and `15 * *G_N + K` (`K` the `add` byte
///   `+0x2a3`) into `+0x2ff`, the second (owner `+0x37c`; byte 1, `+0x14`
///   the immediate `+0x392`) goes after the first and becomes the last;
///   slot 271 `(4)`;
/// - the third (owner `+0x437`) goes last on the list.
///
/// Returns 0. With the ring empty the original goes on with node 0 (and
/// faults); so does the port.
#[allow(clippy::too_many_lines)]
pub fn three_nodes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    head(g, img, f);
    // `mov eax, 8` (magic_0110: the first owner; the count's test one
    // instruction later) or `xor esi, esi`
    let (n_first, owner0) = if crate::r11_misc::byte(img, f + 0x45) == 0xb8 {
        (0x4f, w(0x46))
    } else {
        (0x4c, 0)
    };
    let start = if img.matches(
        img.rel32_target(f + 0x7e),
        &crate::s4_misc::START_OBJECT_FIRST,
    ) {
        ObjectStart::First
    } else {
        ObjectStart::Fam
    };
    actor_objects(g, img, fr, start, n_first, owner0);
    nodes_tail(g, img, f, End::Split)
}

/// [`three_nodes`] whose actors' points take their height from slot 12
/// (`magic_0120` 0x10001410, 1213 bytes): in the loop (`*G_N` `+0x4c`,
/// `+0x188`) each object `OBJ + 0x130 i` (`+0x75`) starts
/// ([`start_object_first`](crate::s4_misc::start_object_first), scale
/// `*K` `+0x64`, owner `15 i`); slot 262 `(chr, p, p + 4, p + 8)` and slot
/// 268 `(chr, 0x11, p)` for its point `p = obj + 0xd0` (`chr` =
/// `CHR[SEL[i]]`, `+0xa4`, `+0x99`), then slot 12 `(chr)`'s float into
/// `p + 4`, `p + 0xc` 1.0 and `p + 4` less `*K2` (`+0x100`); `obj + 0x120`
/// the actor's byte (`+0x11c`) and the record `REC + 0x34 i` (`+0x13b`;
/// `+0x131` cleared first, the immediates `+0x159`, `+0x160`) into `obj +
/// 0x110`. The three nodes follow 0x30 bytes later than in
/// [`three_nodes`].
pub fn three_nodes_height<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    head(g, img, f);
    // the object start's `(ecx, owner)` pushes: the start takes the owner
    // as an argument
    let mut owner = 0u32;
    let mut i = 0u32;
    if g.read::<u32>(w(0x4c)).cast_signed() > 0 {
        loop {
            let obj = w(0x75).wrapping_add(i.wrapping_mul(0x130));
            let scale = g.read::<f32>(w(0x64));
            let at = img.rel32_target(f + 0x7e);
            crate::s4_misc::start_object_first(g, img, at, obj, scale, owner);
            let p = obj + 0xd0;
            let sel = g.read::<u32>(w(0x99));
            let k = u32::from(g.read::<u8>(sel.wrapping_add(i)));
            let chr = g.read::<u32>(w(0xa4).wrapping_add(4 * k));
            let mut s = crate::pushes::Pushes::default();
            s.push_all(&[p + 8, p + 4, p, chr]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0xad), 0x418 / 4), 0);
            s.push_all(&[p, 0x11, chr]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0xca), 0x430 / 4), 0);
            s.push(chr);
            // call width: 12 (Pushes caps the words at 12)
            let y = s.call(g, slot_target(g, w(0xdc), 0x30 / 4), 0).st0;
            g.write::<f32>(p + 4, crate::m0018::st0_f32(y));
            g.write::<u32>(p + 0xc, 0x3f80_0000);
            let y = sub(g.read::<f32>(p + 4), g.read::<f32>(w(0x100)));
            g.write::<f32>(p + 4, y);
            let sel = g.read::<u32>(w(0x11c));
            let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
            g.write::<u32>(obj + 0x120, b);
            let off = i.wrapping_mul(0x34);
            g.write::<u32>(w(0x131).wrapping_add(off), 0);
            let rec = w(0x13b).wrapping_add(off);
            for o in [0x14, 0x10, 0x1c] {
                g.write::<u32>(rec + o, 0);
            }
            g.write::<u32>(rec + 0x20, w(0x159));
            g.write::<u32>(rec + 0x24, w(0x160));
            g.write::<u32>(rec + 0x28, 0);
            g.write::<u32>(rec + 0x2c, 0);
            g.write::<u16>(rec + 0x30, 0);
            g.write::<u32>(obj + 0x110, rec);
            owner = owner.wrapping_add(0xf);
            i = i.wrapping_add(1);
            if i.cast_signed() >= g.read::<u32>(w(0x188)).cast_signed() {
                break;
            }
        }
    }
    nodes_tail(g, img, f + 0x30, End::Split)
}

/// [`three_nodes_height`] (`magic_0120` 0x10001410).
pub const THREE_NODES_HEIGHT_0120: Sig = Sig {
    len: 1213,
    hash: 0x8fd6_58d9_8c28_83ab,
    calls: &[
        (0x3c, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x7e, Callee::Sig(&crate::s4_misc::START_OBJECT_FIRST)),
    ],
    imports: &[],
};

/// How the third node goes last on the task list: [`three_nodes`]
/// stores the last in each branch, `magic_0116`'s copy once after them
/// (the operands' offsets in [`nodes_tail`]'s numbering).
#[derive(Clone, Copy)]
enum End {
    /// `+0x47a` the first, `+0x483` / `+0x468` the last.
    Split,
    /// The first and the last at these offsets.
    Joined(u32, u32),
}

/// The start-ups' three task nodes, from `mov di, [count]` (at `f +
/// 0x162`; [`three_nodes`]' offsets).
#[allow(clippy::too_many_lines)]
fn nodes_tail<G: Game>(g: &mut G, img: &Image, f: u32, end: End) -> Ret {
    let w = |o: u32| img.word(f + o);
    let mut free = g.read::<u16>(w(0x165));
    stores(g, img, f, (0..12).map(|k| 0x169 + 10 * k));
    // the first node (`edx` keeps the ring's head for the second)
    let mut head;
    let a = if free == 0 {
        head = g.read::<u32>(w(0x1ed));
        0
    } else {
        let n = g.read::<u32>(w(0x1f7));
        unlink(g, n);
        head = if free > 1 { g.read::<u32>(n + 4) } else { 0 };
        free -= 1;
        g.write::<u32>(w(0x21c), head);
        clear(g, n, w(0x241));
        g.write::<u16>(w(0x24c), free);
        n
    };
    let mut first = g.read::<u32>(w(0x252));
    g.write::<u8>(a.wrapping_add(0x18), 1);
    g.write::<u32>(a.wrapping_add(0x14), w(0x25d));
    g.write::<u32>(a.wrapping_add(4), 0);
    if first == 0 {
        first = a;
        g.write::<u32>(a, 0);
        g.write::<u32>(w(0x282), a);
    } else {
        let last = g.read::<u32>(w(0x26d));
        g.write::<u32>(a, last);
        g.write::<u32>(last + 4, a);
    }
    g.write::<u32>(a.wrapping_add(8), a);
    // shl eax, 4; sub eax, [G_N]; add eax, imm8
    let n = g.read::<u32>(w(0x28a)) << 4;
    let k = crate::r11_misc::imm8(img, f + 0x2a3);
    let v = n.wrapping_sub(g.read::<u32>(w(0x293))).wrapping_add(k);
    stores(g, img, f, [0x297, 0x2a4]);
    stores(g, img, f, (0..8).map(|k| 0x2ae + 10 * k));
    g.write::<u32>(w(0x2ff), v);
    stores(g, img, f, [0x303, 0x30d, 0x317]);
    // the second node
    let b = if free == 0 {
        0
    } else {
        let n = head;
        unlink(g, n);
        if free > 1 {
            head = g.read::<u32>(n + 4);
            g.write::<u32>(w(0x343), head);
        } else {
            g.write::<u32>(w(0x34b), 0);
        }
        free -= 1;
        clear(g, n, w(0x37c));
        g.write::<u16>(w(0x387), free);
        n
    };
    g.write::<u8>(b.wrapping_add(0x18), 1);
    g.write::<u32>(b.wrapping_add(0x14), w(0x392));
    g.write::<u32>(b.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(b, 0);
        g.write::<u32>(w(0x3b0), b);
    } else {
        g.write::<u32>(b, a);
        g.write::<u32>(a.wrapping_add(4), b);
    }
    let s271 = slot_target(g, w(0x3b5), 0x43c / 4);
    g.write::<u32>(b.wrapping_add(8), b);
    g.write::<u32>(w(0x3c6), b);
    g.call(s271, 0, &[4]);
    // the third node
    let free = g.read::<u16>(w(0x3cf));
    let c = if free == 0 {
        0
    } else {
        let n = g.read::<u32>(w(0x3e1));
        unlink(g, n);
        if free > 1 {
            let next = g.read::<u32>(n + 4);
            g.write::<u32>(w(0x3fe), next);
        } else {
            g.write::<u32>(w(0x406), 0);
        }
        clear(g, n, w(0x437));
        g.write::<u16>(w(0x442), free - 1);
        n
    };
    let first = g.read::<u32>(w(0x448));
    g.write::<u32>(c.wrapping_add(4), 0);
    let (first_w, last_w) = match end {
        End::Split => (0x47a, if first == 0 { 0x483 } else { 0x468 }),
        End::Joined(a, b) => (a, b),
    };
    if first == 0 {
        g.write::<u32>(c, 0);
        g.write::<u32>(w(first_w), c);
    } else {
        let last = g.read::<u32>(w(0x45a));
        g.write::<u32>(c, last);
        g.write::<u32>(last + 4, c);
    }
    g.write::<u32>(c.wrapping_add(8), c);
    g.write::<u32>(w(last_w), c);
    Ret::eax(0)
}

/// The start-up with two objects per actor and three task nodes
/// (internal, no arguments, a /GS frame; `magic_0116` 0x10001460, 1476
/// bytes, and `magic_0117` 0x10002f00, 1475: its second object's index
/// is `push ecx`, 0, instead of `push 1`, and every later operand a byte
/// earlier): [`head`]; the callback `CB` = `+0x56` for one actor, `+0x5b`
/// otherwise; for each of the `*G_N` (`+0x4b`, read once) actors `i`,
/// with `obj = OBJ + 0x260 i` (`+0x65`) and its two points `p = P +
/// 0x260 i` (`+0x85`) and `p + 0x130`:
/// - slot 262 `(chr, &x, &y, &z)` (locals at `ebp - 0x14..`), `chr` the
///   actor's character `CHR[SEL[i]]` (`+0xaa`, `+0x80`);
/// - the first object starts ([`start_object_at`](crate::s4_misc::start_object_at),
///   scale `*K1` `+0xcc`, index the immediate `+0xe3`, owner `15 i`, 0);
///   slot 262 `(chr, p, p + 4, p + 8)`, slot 268 `(chr, 0x11, p)`, `p +
///   0xc` 1.0, `obj + 0xd4` = `y - *K2` (`+0x13f`), the started object's
///   `+0x64` times `*K3` (`+0x169`); its record `REC + 0x34 i` (`+0x191`;
///   `+0x10`..`+0x2c` 0 but `+0x20` the immediate `+0x1b5` and `+0x24`
///   `CB`, halfword `+0x30` 0; `+0x187` the word cleared first) into
///   `obj + 0x110`;
/// - the second object `obj + 0x130` starts (scale `*K4` `+0x154`, the
///   index, owner `15 i + K` (the `add` byte `+0x1dc`), the immediate
///   `+0x1d5`); the same two slots for `p + 0x130`, its `+0xc` 1.0, `obj +
///   0x204` = `y - *K5` (`+0x244`), `obj + 0x250` the actor's byte
///   `SEL[i]` (`+0x24a`).
///
/// Then [`three_nodes`]' three task nodes (from `+0x29c`). Returns 0.
#[allow(clippy::too_many_lines)]
pub fn two_per_actor<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; mov ebp, esp; sub esp, 0x30 (the cookie at ebp - 4)
    let ebp = fr.esp.wrapping_sub(4);
    let (x, y, z) = (ebp - 0x14, ebp - 0x10, ebp - 0xc);
    head(g, img, f + 0xa);
    // `push 1` (6a 01) or `push ecx` (51) for the second index
    let (index2, d) = if crate::r11_misc::byte(img, f + 0x1de) == 0x6a {
        (crate::r11_misc::byte(img, f + 0x1df), 0)
    } else {
        (0, 1)
    };
    let w2 = |o: u32| img.word(f + o - d);
    let n = g.read::<u32>(w(0x4b));
    let callback = if n == 1 { w(0x56) } else { w(0x5b) };
    let index1 = crate::r11_misc::byte(img, f + 0xe3);
    let add = crate::r11_misc::imm8(img, f + 0x1dc);
    let mut sel = g.read::<u32>(w(0x80));
    let mut owner = 0u32;
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let obj = w(0x65).wrapping_add(i.wrapping_mul(0x260));
        let p = w(0x85).wrapping_add(i.wrapping_mul(0x260));
        let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        let chr = g.read::<u32>(w(0xaa).wrapping_add(4 * b));
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[z, y, x, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0xbc), 0x418 / 4), 0);
        s.pop(4);
        g.write::<u32>(ebp - 8, 0x3f80_0000);
        let at = img.rel32_target(f + 0xe5);
        let started =
            crate::s4_misc::start_object_at(g, img, at, obj, k(g, 0xcc), index1, owner, 0);
        let sel1 = g.read::<u32>(w(0xfc));
        let b = u32::from(g.read::<u8>(sel1.wrapping_add(i)));
        let chr = g.read::<u32>(w(0x107).wrapping_add(4 * b));
        s.push_all(&[p + 8, p + 4, p, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x117), 0x418 / 4), 0);
        s.push_all(&[p, 0x11, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x124), 0x430 / 4), 0);
        s.pop(7);
        let ya = sub(g.read::<f32>(y), k(g, 0x13f));
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        g.write::<f32>(obj + 0xd4, ya);
        let scaled = mul(g.read::<f32>(started.wrapping_add(0x64)), k(g, 0x169));
        g.write::<f32>(started.wrapping_add(0x64), scaled);
        let off = i.wrapping_mul(0x34);
        g.write::<u32>(w(0x187).wrapping_add(off), 0);
        let rec = w(0x191).wrapping_add(off);
        for o in [0x14, 0x10, 0x1c] {
            g.write::<u32>(rec + o, 0);
        }
        g.write::<u32>(rec + 0x24, callback);
        g.write::<u32>(rec + 0x20, w(0x1b5));
        g.write::<u32>(rec + 0x28, 0);
        g.write::<u32>(rec + 0x2c, 0);
        g.write::<u16>(rec + 0x30, 0);
        g.write::<u32>(obj + 0x110, rec);
        let (obj2, p2) = (obj + 0x130, p + 0x130);
        let at = img.rel32_target(f + 0x1ed - d);
        let o2 = owner.wrapping_add(add);
        crate::s4_misc::start_object_at(g, img, at, obj2, k(g, 0x154), index2, o2, w(0x1d5));
        let sel2 = g.read::<u32>(w2(0x201));
        let b = u32::from(g.read::<u8>(sel2.wrapping_add(i)));
        let chr = g.read::<u32>(w2(0x20c).wrapping_add(4 * b));
        s.push_all(&[p2 + 8, p2 + 4, p2, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w2(0x21c), 0x418 / 4), 0);
        s.push_all(&[p2, 0x11, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w2(0x229), 0x430 / 4), 0);
        let yb = sub(g.read::<f32>(y), g.read::<f32>(w2(0x244)));
        sel = g.read::<u32>(w2(0x24a));
        g.write::<u32>(p2 + 0xc, 0x3f80_0000);
        g.write::<f32>(obj + 0x204, yb);
        let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        g.write::<u32>(obj + 0x250, b);
        owner = owner.wrapping_add(0xf);
        i = i.wrapping_add(1);
    }
    nodes_tail(g, img, f + 0x13a - d, End::Joined(0x46d, 0x47b))
}

/// [`two_per_actor`] (with the object start's and the cookie check's
/// `rel32`).
pub const TWO_PER_ACTOR_0116: Sig = Sig {
    len: 1476,
    hash: 0x8503_aff8_440b_6ecb,
    calls: &[
        (0x46, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xe5, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1ed, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        // __security_check_cookie
        (0x5bc, Callee::Any),
    ],
    imports: &[],
};

/// [`two_per_actor`] in `magic_0117` (0x10002f00).
pub const TWO_PER_ACTOR_0117: Sig = Sig {
    len: 1475,
    hash: 0xc8a9_0339_8c62_fb50,
    calls: &[
        (0x46, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xe5, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1ec, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        // __security_check_cookie
        (0x5bb, Callee::Any),
    ],
    imports: &[],
};

/// [`three_nodes`]' identity in each DLL (`hash`, the object start's
/// `Sig`).
macro_rules! three_nodes_copies {
    ($($sig:ident, $hash:literal, $start:path, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`three_nodes`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 1165,
                hash: $hash,
                calls: &[
                    (0x3c, Callee::Sig(&crate::l5_objhost::PART_INIT)),
                    (0x7e, Callee::Sig(&$start)),
                ],
                imports: &[],
            };
        )*
    };
}

three_nodes_copies! {
    THREE_NODES_0110, 0x7704_9756_7fda_9e92, crate::o6_fam_object::START_OBJECT_FIRST_0074_1C70,
        "`magic_0110` 0x10001410";
    THREE_NODES_0112, 0x634c_e5e5_532f_5032, crate::o6_fam_object::START_OBJECT_FIRST_0074_1C70,
        "`magic_0112` 0x10001410";
    THREE_NODES_0121, 0x1e48_5c43_827e_25ef, crate::s4_misc::START_OBJECT_FIRST,
        "`magic_0121` 0x10002ec0";
}
