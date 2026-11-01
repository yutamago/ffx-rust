//! Round 6 shape families: exact pairs, third file (two DLLs' copies of
//! one function that differ only in their globals and immediates;
//! families of `~/ffx-work/m5/magic/r4/families.tsv`). Each port reads
//! its operands from the copy's code (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::sb_sse::{cvt, div, mul};
use crate::{Callee, Entry, Frame, Game, Image, Ret, Sig};

/// An effect's start with two list nodes (no arguments; `magic_0638`
/// 0x10003270 and `magic_0689` 0x100032c0, 251 bytes; family
/// `F0638_10003270`): three buffer globals from immediates; slot 719
/// `(P, 8, 0)` into `G_H`; the part set up
/// ([`crate::l5_objhost::part_init`]); `(*G_C)[0x130]` = 0 and four more
/// globals from immediates (a word, a word, a u16, a byte); then two
/// pool nodes ([`crate::sa_pool::take`] with the callbacks `CB1`,
/// `CB2`), kinds 1 and 2 (byte `+0x18`), appended to the list (`G_FIRST`
/// `+0x85`, `G_LAST` `+0x9c`). The first is linked after the last node
/// without moving `G_LAST`, the second after the first. With no first
/// node the first becomes it. Returns 0.
///
/// The `G_FIRST == 0` branch after the second take (`+0xe3`) needs the
/// first take to have returned 0, which faults before it in the
/// original; it is ported as written.
pub fn two_node_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = w(0x01);
    let s719 = slot_target(g, t, w(0x08) / 4);
    for (a, v) in [(0x18, 0x1c), (0x22, 0x26), (0x2c, 0x30)] {
        g.write::<u32>(w(a), w(v));
    }
    // call width: 3
    let h = g
        .call(
            s719,
            fr.ecx,
            &[w(0x12), disp8(img, f + 0x10), disp8(img, f + 0x0e)],
        )
        .eax;
    g.write::<u32>(w(0x3a), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x3f));
    let c = g.read::<u32>(w(0x44));
    g.write::<u32>(c.wrapping_add(w(0x4d)), w(0x51));
    g.write::<u32>(w(0x5c), w(0x60));
    g.write::<u32>(w(0x66), w(0x6a));
    g.write::<u16>(w(0x71), w(0x75) as u16);
    g.write::<u8>(w(0x79), w(0x7d) as u8);
    let first_node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x7f), w(0x56));
    let mut first = g.read::<u32>(w(0x85));
    g.write::<u8>(first_node.wrapping_add(0x18), w(0x8e) as u8);
    g.write::<u32>(first_node.wrapping_add(4), w(0x92));
    if first == 0 {
        first = first_node;
        g.write::<u32>(first_node, 0);
        g.write::<u32>(w(0xb1), first_node);
    } else {
        let last = g.read::<u32>(w(0x9c));
        g.write::<u32>(first_node, last);
        g.write::<u32>(last.wrapping_add(4), first_node);
    }
    g.write::<u32>(first_node.wrapping_add(8), first_node);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0xbe), w(0xb6));
    g.write::<u8>(second.wrapping_add(0x18), w(0xc5) as u8);
    g.write::<u32>(second.wrapping_add(4), w(0xc9));
    if first == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0xeb), second);
        g.write::<u32>(second.wrapping_add(8), second);
        g.write::<u32>(w(0xf3), second);
    } else {
        g.write::<u32>(second, first_node);
        g.write::<u32>(first_node.wrapping_add(4), second);
        g.write::<u32>(second.wrapping_add(8), second);
        g.write::<u32>(w(0xdb), second);
    }
    Ret::eax(0)
}

/// An object's matrix and scale set-up (cdecl `(obj)`; a /GS frame of
/// 0x98 bytes, `ebp` = `esp - 4` at the entry; `magic_0434` 0x10007c90
/// and `magic_0671` 0x10001390, 230 bytes, the same code; family
/// `F0434_10007c90`): with the frame's matrices `M1` (`ebp - 0x94`), `M2`
/// (`ebp - 0x54`) and vector `V` (`ebp - 0x14`), slot 723 `(M1)`, slot
/// 724 `(M2)`, slot 327 `(obj + 0x80, M2)`, `V` from four immediates and
/// slot 229 `(obj + 0xd0, obj + 0x80, V)`, each call seeing the earlier
/// words; `obj + 0xdc` = 1.0; then the scale `K1 / slot 282 * K2` (ST0
/// as `f32`) into `obj + 0x60..+0x6c`. Returns slot 282's `eax` (the
/// cookie check leaves it).
pub fn matrix_scale_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (m1, m2, v) = (
        ebp.wrapping_sub(0x94),
        ebp.wrapping_sub(0x54),
        ebp.wrapping_sub(0x14),
    );
    let t = w(0x20);
    let mut stack = vec![m1];
    // call width: 1
    g.call(slot_target(g, t, w(0x26) / 4), fr.ecx, &stack);
    stack.insert(0, m2);
    // call width: 2 (the earlier call's words stay on the stack)
    g.call(slot_target(g, w(0x31), w(0x37) / 4), fr.ecx, &stack);
    let at80 = obj.wrapping_add(w(0x48));
    stack.splice(0..0, [at80, m2]);
    // call width: 4 (the earlier calls' words stay on the stack)
    g.call(slot_target(g, w(0x42), w(0x4e) / 4), fr.ecx, &stack);
    let s229 = slot_target(g, w(0x62), w(0x84) / 4);
    for (i, at) in (0u32..).zip([0x69u32, 0x70, 0x77, 0x7e]) {
        g.write::<u32>(v.wrapping_add(4 * i), w(at));
    }
    stack.splice(0..0, [obj.wrapping_add(w(0x5b)), at80, v]);
    // call width: 7 (the earlier calls' words stay on the stack)
    g.call(s229, fr.ecx, &stack);
    g.write::<u32>(obj.wrapping_add(w(0x91)), w(0x95));
    // call width: 0
    let out = g.call(slot_target(g, w(0x8b), w(0x9b) / 4), fr.ecx, &[]);
    let r = f32::from_bits(crate::sa_summon::st0_f32(out));
    let x = mul(div(g.read::<f32>(w(0xae)), r), g.read::<f32>(w(0xc1)));
    for o in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<f32>(obj.wrapping_add(o), x);
    }
    Ret::eax(out.eax)
}

/// A track's next frame byte: the entry after `index` (an `i16`) in the
/// track `{count, bytes}` at `track`, or the first when it would reach
/// `count` (signed).
fn next_frame_byte<G: Game>(g: &G, track: u32, index: u32) -> u8 {
    let mut i = i32::from(g.read::<i16>(index)) + 1;
    if i >= g.read::<i32>(track) {
        i = 0;
    }
    let bytes = g.read::<u32>(track.wrapping_add(4));
    g.read::<u8>(bytes.wrapping_add(i.cast_unsigned()))
}

/// The party's colours for the screen (no arguments; a /GS frame of
/// 0x14 bytes, `ebp` = `esp - 4` at the entry; `magic_0644` 0x10006150
/// and `magic_0565` 0x10001fc0, 253 bytes, the same code; family
/// `F0644_10006150`): the colour table `C` (three bytes per member at
/// `ebp - 0x10`, starting all 0xff) gets, for each of the `*G_N`
/// members (records of 0x78 bytes from `A`, member numbers from `B`,
/// `+0x33`, `+0x38`), the next frame byte of its three tracks (track
/// pointers at `-0x1c`, `+0`, `+0x1c` of the record, the current indices
/// at `-0x16`, `+6`, `+0x22`); then slot 808 `(*G_P, C, slot 651, 20.0)`
/// (slot 651's ST0 as `f32`). Returns slot 808's `eax` (the cookie check
/// leaves it).
pub fn party_colours<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let colours = ebp.wrapping_sub(0x10);
    let n = g.read::<i32>(w(0x13));
    for i in 0..3u32 {
        g.write::<u32>(colours.wrapping_add(4 * i), w(0x1a + 7 * i));
    }
    if n > 0 {
        let (mut rec, mut member) = (w(0x33), w(0x38));
        for _ in 0..n {
            let at = colours.wrapping_add(g.read::<u32>(member).wrapping_mul(3));
            for (k, (track, index)) in
                (0u32..).zip([(0xffff_ffe4u32, 0xffff_ffeau32), (0, 6), (0x1c, 0x22)])
            {
                let b = next_frame_byte(
                    g,
                    g.read::<u32>(rec.wrapping_add(track)),
                    rec.wrapping_add(index),
                );
                g.write::<u8>(at.wrapping_add(k), b);
            }
            rec = rec.wrapping_add(0x78);
            member = member.wrapping_add(4);
        }
    }
    let t = w(0xb0);
    // call width: 0
    let out = g.call(slot_target(g, t, w(0xb6) / 4), fr.ecx, &[]);
    let r = crate::sa_summon::st0_f32(out);
    let p = g.read::<u32>(w(0xdf));
    // call width: 4
    let eax = g
        .call(
            slot_target(g, w(0xd9), w(0xe5) / 4),
            fr.ecx,
            &[p, colours, r, w(0xce)],
        )
        .eax;
    Ret::eax(eax)
}

/// An object's transform from its bone (cdecl `(obj)`; a /GS frame of
/// 0x18 bytes, `ebp` = `esp - 4` at the entry; `magic_0249` 0x10001d70
/// and `magic_0562` 0x10001d80, 297 bytes, the same code; family
/// `F0249_10001d70`): with the frame's vector `V` (`ebp - 0x14`, four
/// immediates), slot 237 `(*G_A, K, V, obj + 0xd0)`, `obj + 0xdc` = 1.0,
/// slot 236 `(*G_A, K, obj + 0x80)` (the earlier words below), then `s =
/// K2 / slot 277 (*G_A)` (ST0 as `f32`) scales the 3x3 block at `obj +
/// 0x80` (the words `+0, +4, +8, +0x10, +0x14, +0x18, +0x20, +0x24,
/// +0x28`, each product in the original's operand order) and `obj +
/// 0x126` = 2. Returns slot 277's `eax` (the cookie check leaves it).
pub fn bone_transform<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.arg(g, 0);
    let v = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let t = w(0x21);
    let at_d0 = obj.wrapping_add(w(0x1a));
    let a = g.read::<u32>(w(0x29));
    let k = disp8(img, f + 0x26);
    let s237 = slot_target(g, t, w(0x2f) / 4);
    for (i, at) in (0u32..).zip([0x36u32, 0x3d, 0x44, 0x4b]) {
        g.write::<u32>(v.wrapping_add(4 * i), w(at));
    }
    let mut stack = vec![a, k, v, at_d0];
    // call width: 4
    g.call(s237, fr.ecx, &stack);
    g.write::<u32>(at_d0.wrapping_add(disp8(img, f + 0x58)), w(0x59));
    let m = obj.wrapping_add(w(0x65));
    let a = g.read::<u32>(w(0x6e));
    stack.splice(0..0, [a, disp8(img, f + 0x6b), m]);
    // call width: 7 (the earlier call's words stay on the stack)
    g.call(slot_target(g, w(0x52), w(0x5f) / 4), fr.ecx, &stack);
    let a = g.read::<u32>(w(0x7b));
    stack.insert(0, a);
    // call width: 8 (the earlier calls' words stay on the stack)
    let out = g.call(slot_target(g, w(0x75), w(0x81) / 4), fr.ecx, &stack);
    let r = f32::from_bits(crate::sa_summon::st0_f32(out));
    let s = div(g.read::<f32>(w(0x8e)), r);
    // (offset, scale first): `movaps xmm0, xmm1; mulss xmm0, [m + k]`
    // puts the scale first
    for (o, scale_first) in [
        (0u32, false),
        (4, true),
        (8, false),
        (0x10, false),
        (0x14, true),
        (0x18, true),
        (0x20, true),
        (0x24, false),
        (0x28, false),
    ] {
        let x = g.read::<f32>(m.wrapping_add(o));
        let y = if scale_first { mul(s, x) } else { mul(x, s) };
        g.write::<f32>(m.wrapping_add(o), y);
    }
    g.write::<u8>(obj.wrapping_add(w(0x119)), w(0x11d) as u8);
    Ret::eax(out.eax)
}

/// The 12-byte node-handler thunks (cdecl `(cursor)`: `push ebp; mov ebp,
/// esp; mov ecx, [ebp + 8]; pop ebp; jmp T`; family `F0070_100014a0`,
/// whose members differ only in `T`): the handler at `T` with `ecx` = the
/// cursor and the caller's stack (the jump leaves it as it was).
fn thunk_to<G: Game>(g: &mut G, img: &Image, fr: &Frame, handler: Entry<G>) -> Ret {
    let inner = Frame {
        at: img.rel32_target(fr.at + 8),
        ecx: fr.arg(g, 0),
        ..*fr
    };
    handler(g, img, &inner)
}

/// The thunk of [`crate::o6_fam_pairs2::models_detach`] (`magic_0164`
/// 0x100080c0, `magic_0155` 0x10008040).
pub fn models_detach_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    thunk_to(g, img, fr, crate::o6_fam_pairs2::models_detach::<G>)
}

/// The thunk of [`crate::s4_small::walk_scale_steps`] (`magic_0262`
/// 0x10004650).
pub fn walk_scale_steps_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    thunk_to(g, img, fr, crate::s4_small::walk_scale_steps::<G>)
}

/// The thunk of [`crate::m0042::visit_pool`] (`magic_0042` 0x100010b0).
pub fn visit_pool_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    thunk_to(g, img, fr, crate::m0042::visit_pool::<G>)
}

/// The thunk of [`crate::m0071::e3f00`] (the spinning node's visit;
/// `magic_0071` 0x10004080).
pub fn spin_visit_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    thunk_to(g, img, fr, crate::m0071::e3f00::<G>)
}

/// A particle's random set-up (cdecl `(obj)`; an `ebp` frame of 8 bytes;
/// `magic_0359` 0x10001580 and 0x100016d0, 321 bytes; family
/// `F0359_10001580`): `x = (*G_I + 1) / *G_N` (both integers), `y =
/// rand() / K1 * K2`; slot 441 `(obj, (*G_A)[0x198], K)`, slot 443 `(obj,
/// K')`; `obj + 0xbc |= 0x4000`, `obj + 0x108` = `y`; the particle's
/// words from immediates (`+0x17c` = `x`); slot 458 `(*G_A, obj, 0)`,
/// slot 450 `(obj, CB)`, slot 451 `(obj, K'')`, each call seeing the
/// earlier words. Returns slot 451's `eax`.
pub fn random_particle_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let i = g.read::<u32>(w(0x07)).wrapping_add(1);
    let x = div(cvt(i), cvt(g.read::<u32>(w(0x0f))));
    let r = cvt(g.rand());
    let y = mul(div(r, g.read::<f32>(w(0x3e))), g.read::<f32>(w(0x49)));
    let obj = fr.arg(g, 0);
    let a = g.read::<u32>(w(0x36));
    let word = g.read::<u32>(a.wrapping_add(w(0x51)));
    let t = w(0x56);
    let mut stack = vec![obj, word, disp8(img, f + 0x4e)];
    // call width: 3
    g.call(slot_target(g, t, w(0x5d) / 4), fr.ecx, &stack);
    stack.splice(0..0, [obj, disp8(img, f + 0x6e)]);
    // call width: 5 (the earlier call's words stay on the stack)
    g.call(slot_target(g, w(0x69), w(0x71) / 4), fr.ecx, &stack);
    let flags = obj.wrapping_add(w(0x7a));
    let fl = g.read::<u32>(flags);
    g.write::<u32>(flags, fl | w(0x7e));
    g.write::<f32>(obj.wrapping_add(w(0x90)), y);
    let a = g.read::<u32>(w(0x9e));
    let table = g.read::<u32>(w(0x88));
    for (d, v) in [
        (0xa4, 0xa8),
        (0xae, 0xb2),
        (0xb8, 0xbc),
        (0xc2, 0xc6),
        (0xcc, 0xd0),
        (0xd6, 0xda),
    ] {
        g.write::<u32>(obj.wrapping_add(w(d)), w(v));
    }
    g.write::<f32>(obj.wrapping_add(w(0xe2)), x);
    for (d, v) in [(0xe8, 0xec), (0xf2, 0xf6), (0xfc, 0x100), (0x106, 0x10a)] {
        g.write::<u32>(obj.wrapping_add(w(d)), w(v));
    }
    stack.splice(0..0, [a, obj, disp8(img, f + 0x9a)]);
    let target = g.read::<u32>(table.wrapping_add(4 * (w(0x110) / 4)));
    // call width: 8 (the earlier calls' words stay on the stack)
    g.call(target, fr.ecx, &stack);
    stack.splice(0..0, [obj, w(0x11c)]);
    // call width: 10 (the earlier calls' words stay on the stack)
    g.call(slot_target(g, w(0x117), w(0x122) / 4), fr.ecx, &stack);
    stack.splice(0..0, [obj, disp8(img, f + 0x12f)]);
    // call width: 12 (the earlier calls' words stay on the stack)
    let eax = g
        .call(slot_target(g, w(0x12a), w(0x132) / 4), fr.ecx, &stack)
        .eax;
    Ret::eax(eax)
}

/// Four timelines run by button (no arguments; `magic_0375` 0x10002370
/// and `magic_0376` 0x10002700, 314 bytes; family `F0375_10002370`): 0
/// while `*G_ON` (`+0x02`) is 0; else `*G_P = ITEM`, slot 723 `(*G_M)`,
/// slot 724 `(*G_N)`, `**G_X = *G_Y`, `*G_W = *G_Z`; for the buttons 0..3
/// slot 261 `(i)` and, when it answers, the timeline `TL_i` runs
/// ([`crate::sb_misc::timeline_run`]); with button 4 all four stop
/// ([`crate::sb_misc::timeline_stop`]) and slot 214 `(*G_E)` follows,
/// without it the counter `*G_C` goes up. Returns 1.
pub fn button_timelines<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x0d));
    let m = g.read::<u32>(w(0x13));
    g.write::<u32>(p, w(0x19));
    let mut stack = vec![m];
    // call width: 1
    g.call(slot_target(g, w(0x1e), w(0x24) / 4), fr.ecx, &stack);
    stack.insert(0, g.read::<u32>(w(0x31)));
    // call width: 2 (the earlier call's words stay on the stack)
    g.call(slot_target(g, w(0x2b), w(0x37) / 4), fr.ecx, &stack);
    let x = g.read::<u32>(w(0x3f));
    let y = g.read::<u32>(w(0x44));
    g.write::<u32>(x, y);
    let z = g.read::<u32>(w(0x4b));
    g.write::<u32>(w(0x50), z);
    stack.insert(0, disp8(img, f + 0x5a));
    // call width: 3 (the earlier calls' words stay on the stack)
    let mut pressed = g
        .call(slot_target(g, w(0x55), w(0x5d) / 4), fr.ecx, &stack)
        .eax
        != 0;
    // (the timeline's `mov ecx` immediate, the run's rel32, the next
    // button's `push imm8`, its `mov eax, [T]` and slot load)
    for (tl, run, push, t, slot) in [
        (0x6b, 0x70, 0x7a, 0x75, 0x7d),
        (0x8b, 0x90, 0x9a, 0x95, 0x9d),
        (0xab, 0xb0, 0xba, 0xb5, 0xbd),
        (0xcb, 0xd0, 0xda, 0xd5, 0xdd),
    ] {
        if pressed {
            crate::sb_misc::timeline_run(g, img, img.rel32_target(f + run), w(tl));
        }
        // call width: 1
        pressed =
            g.call(
                slot_target(g, w(t), w(slot) / 4),
                fr.ecx,
                &[disp8(img, f + push)],
            )
            .eax != 0;
    }
    if !pressed {
        let c = g.read::<u32>(w(0x130));
        g.write::<u32>(w(0x130), c.wrapping_add(1));
        return Ret::eax(w(0x135));
    }
    for tl in [0xebu32, 0xf5, 0xff, 0x109] {
        crate::sb_misc::timeline_stop(g, w(tl));
    }
    let e = g.read::<u32>(w(0x119));
    // call width: 1
    g.call(slot_target(g, w(0x113), w(0x11f) / 4), fr.ecx, &[e]);
    Ret::eax(w(0x129))
}

/// A part's matrix hand-over to the game (internal: `ecx` = the part
/// `p`, `edx` its source matrix `src`; a /GS frame of 0xf4 bytes, `ebp`
/// = `esp - 4` at the entry; `magic_0307` 0x10002550 and `magic_0605`
/// 0x10007ed0, 342 bytes; family `F0307_10002550`): the handle `h` (64
/// bits) = slot 772 `(p, 0)`; with the frame's matrices `M1` (`ebp -
/// 0xc8`), `M2` (`ebp - 0x48`), `M3` (`ebp - 0x88`): slot 723 `(M1)`,
/// slot 724 `(M2)`, slot 231 `(M3, M2, src)`; the scene record `R =
/// *G_S + K` gets `+0x20` = `M3`, `+0x34` = `*G_T`, `+0x44` = 0; with
/// the game lock `**G_L` set to 1, slot 773 `(h)` asks; the lock is put
/// back and set to 1 again, the frame record `L` (`ebp - 0xf0`) gets `p`
/// at `+0x1c`; then slot 819 `(h, L, R)` and slot 870 `(h)` (lock set to
/// 1 again) when 773 answered, else slot 871 `(h, L, R, 0x248)`; the
/// lock gets its old value. Each call sees the words left before it.
/// Returns `*G_L` (the lock's pointer).
pub fn part_matrix_handover<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (p, src) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let (m1, m2, m3, rec) = (
        ebp.wrapping_sub(0xc8),
        ebp.wrapping_sub(0x48),
        ebp.wrapping_sub(0x88),
        ebp.wrapping_sub(0xf0),
    );
    g.write::<u32>(ebp.wrapping_sub(0xd0), p);
    let t = w(0x22);
    let mut stack = vec![p, disp8(img, f + 0x1f)];
    // call width: 2
    let h = g.call(slot_target(g, t, w(0x2a) / 4), fr.ecx, &stack);
    let (lo, hi) = (h.eax, h.edx);
    stack.insert(0, m1);
    let s723 = slot_target(g, w(0x32), w(0x3a) / 4);
    // call width: 3 (the earlier call's words stay on the stack)
    g.call(s723, fr.ecx, &stack);
    // call width: 1
    g.call(slot_target(g, w(0x51), w(0x57) / 4), fr.ecx, &[m2]);
    // call width: 3
    g.call(slot_target(g, w(0x6d), w(0x73) / 4), fr.ecx, &[m3, m2, src]);
    let r = g.read::<u32>(w(0x7b)).wrapping_add(w(0x81));
    g.write::<u32>(r.wrapping_add(disp8(img, f + 0x8d)), m3);
    let v = g.read::<u32>(w(0x8f));
    g.write::<u32>(r.wrapping_add(disp8(img, f + 0x95)), v);
    let lock = g.read::<u32>(w(0x97));
    g.write::<u32>(r.wrapping_add(disp8(img, f + 0x9d)), w(0x9e));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, w(0xa9));
    g.write::<u32>(ebp.wrapping_sub(0xcc), saved);
    // call width: 2
    let asked = g
        .call(slot_target(g, w(0xae), w(0xb5) / 4), fr.ecx, &[lo, hi])
        .eax;
    let lock = g.read::<u32>(w(0xc4));
    g.write::<u32>(lock, saved);
    g.write::<u32>(ebp.wrapping_sub(0xd4), p);
    g.write::<u32>(lock, w(0xe3));
    if asked != 0 {
        let mut stack = vec![lo, hi, rec, r];
        // call width: 4
        g.call(slot_target(g, w(0xf2), w(0xf9) / 4), fr.ecx, &stack);
        let lock = g.read::<u32>(w(0x101));
        g.write::<u32>(lock, w(0x108));
        stack.splice(0..0, [lo, hi]);
        // call width: 6 (the earlier call's words stay on the stack)
        g.call(slot_target(g, w(0x10d), w(0x114) / 4), fr.ecx, &stack);
    } else {
        // call width: 5
        g.call(
            slot_target(g, w(0x127), w(0x12e) / 4),
            fr.ecx,
            &[lo, hi, rec, r, w(0x120)],
        );
    }
    let lock = g.read::<u32>(w(0x139));
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// An object's matrix from its bone, once it is near (cdecl `(obj)`; a
/// /GS frame of 0x48 bytes, `ebp` = `esp - 4` at the entry; `magic_0172`
/// 0x10001920 and `magic_0174` 0x10001ad0, 377 bytes; family
/// `F0172_10001920`): past the distance 0x50 (`obj + 0x118`, signed) only
/// `obj + 0x104` = 0 (and `eax` is the cookie word the frame set-up
/// left). Else, with the frame's matrix `M` (`ebp - 0x44`): slot 235
/// `(M)`, slot 328 `(M, M, K)`, `M[0x38]` = an immediate, slot 236
/// `(*G_A, K', obj + 0x80)`, the scale `s = K2 / slot 277 (*G_A)` on the
/// 3x3 block at `obj + 0x80` (each product in the original's operand
/// order), slot 231 `(obj + 0x80, obj + 0x80, M)`; then `obj + 0xd0..`
/// = `obj + 0xb0..` and `obj + 0x126` = 2. Each call sees the words left
/// before it (the caller's `esi`, saved first, is the deepest and no slot
/// reads it). Returns `obj[0xbc]`.
pub fn near_bone_matrix<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    if g.read::<i32>(obj.wrapping_add(w(0x16))) > disp8(img, f + 0x1a).cast_signed() {
        g.write::<u32>(obj.wrapping_add(w(0x1f)), w(0x23));
        // mov eax, [__security_cookie]; xor eax, ebp
        return Ret::eax(g.read::<u32>(w(0x07)) ^ ebp);
    }
    let m = ebp.wrapping_sub(0x44);
    let t = w(0x3c);
    // call width: 1
    g.call(slot_target(g, t, w(0x42) / 4), fr.ecx, &[m]);
    let mut stack = vec![m, m, w(0x4e)];
    // call width: 3
    g.call(slot_target(g, w(0x55), w(0x5b) / 4), fr.ecx, &stack);
    let block = obj.wrapping_add(w(0x68));
    let a = g.read::<u32>(w(0x71));
    g.write::<u32>(ebp.wrapping_sub(0xc), w(0x78));
    stack.splice(0..0, [a, disp8(img, f + 0x6e), block]);
    // call width: 6 (the earlier calls' words stay on the stack)
    g.call(slot_target(g, w(0x62), w(0x7e) / 4), fr.ecx, &stack);
    let a = g.read::<u32>(w(0x8b));
    stack.insert(0, a);
    // call width: 7 (the earlier calls' words stay on the stack)
    let out = g.call(slot_target(g, w(0x85), w(0x91) / 4), fr.ecx, &stack);
    let r = f32::from_bits(crate::sa_summon::st0_f32(out));
    let s = div(g.read::<f32>(w(0x9e)), r);
    let s231 = slot_target(g, w(0xb3), w(0xc6) / 4);
    // (offset, scale first): `movaps xmm0, xmm1; mulss xmm0, [m + k]` and
    // the last `mulss xmm1, [esi + 0x28]` put the scale first
    for (o, scale_first) in [
        (0u32, true),
        (4, false),
        (8, false),
        (0x10, true),
        (0x14, false),
        (0x18, false),
        (0x20, false),
        (0x24, true),
        (0x28, true),
    ] {
        let x = g.read::<f32>(block.wrapping_add(o));
        let y = if scale_first { mul(s, x) } else { mul(x, s) };
        g.write::<f32>(block.wrapping_add(o), y);
    }
    stack.splice(0..0, [block, block, m]);
    // call width: 10 (the earlier calls' words stay on the stack)
    g.call(s231, fr.ecx, &stack);
    let mut eax = 0;
    for (from, to) in [
        (0x131u32, 0x13a),
        (0x140, 0x146),
        (0x14c, 0x155),
        (0x15b, 0x161),
    ] {
        eax = g.read::<u32>(obj.wrapping_add(w(from)));
        g.write::<u32>(obj.wrapping_add(w(to)), eax);
    }
    g.write::<u8>(obj.wrapping_add(w(0x168)), w(0x16c) as u8);
    Ret::eax(eax)
}

include!("o6_fam_pairs3_table.rs.in");
