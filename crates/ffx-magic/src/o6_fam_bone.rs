//! Round 6 shape families: a summon model's bone node set-up (family
//! `F0201_10006f20` of `~/ffx-work/m5/magic/r4/families.tsv`; 3 copies in
//! `magic_0201`, `0203`, `0273`, 428 bytes each). The copies differ only
//! in their globals and constants, read from each copy's code (`w(off)` =
//! the word at `+off`).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Sets up a model's bone node (internal: `ecx` = the model `m`, `edx`
/// the source `src`, one stack word `arg`; a /GS frame of 0x90 bytes,
/// `ebp` = `esp - 4` at the entry). The model's offsets are the copy's
/// (`m + 0x4c` or `+0x48`, and so on); the ones below are `magic_0201`'s.
/// 1. slot 719 `(P, K1, K2)` (`P` at `+0x2c`, the `imm8`s at `+0x2a`,
///    `+0x22`: 2 and 6 or 5);
/// 2. the node `n = m + 0x4c` (u16 `+0x1a` 0xff, `+0xc`/`+0x10` itself)
///    put last in the root `R`'s ring (`R` the immediate at `+0x4b`; the
///    root's last `R + 0x10` at `+0x57`, its first `R + 0xc` at `+0x6c`,
///    its u16 count `R + 0x18` at `+0x78`); `R + 0x10` = `n` and `n`'s
///    vtable `V` (`+0x99`);
/// 3. slot 233 `(m + 0x68, src + 0x1c, 0.1)` (the float at `+0x83`);
///    `m + 0x74` = 1.0;
/// 4. a group `{m + 0xd0, R, A, n, arg, B}` at `ebp - 0x8c` (`A`, `B` at
///    `+0xd4`, `+0xe1`) inserted ([`crate::nodes::node_insert`]);
/// 5. a cursor `{m + 0xd0, pass}` at `ebp - 0x74` (the pass byte at
///    `+0xeb`) and the bone updated ([`crate::sa_summon::bone_plain`]);
/// 6. two constant vectors at `ebp - 0x18` and `ebp - 0x28` (`+0x110`..),
///    slot 230 `(M, m + 0x16c)` into the matrix `M` at `ebp - 0x6c`,
///    slot 327 `(M, M)`, slot 858 `(1, M, &v1, &v2, 0)`, slot 859
///    `(0x3800)`;
/// 7. `m + 0x18` = 0.
///
/// Returns slot 859's `eax` (the cookie check leaves it). `magic_0272`'s
/// copy reads `arg` and `src` from two globals instead
/// (`r24::BONE_NODE_SETUP_0272`).
pub fn bone_node_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // `magic_0272`'s copy (0x100067a0) takes `arg` and `src` from globals
    // (`mov eax, [G_A]` at +0x13, `mov esi, [G_S]` at +0x1a, no `mov esi,
    // edx`): its operands sit 8 bytes later up to that `mov` and 6 after
    let globals = img.word(f + 0x13) as u8 == 0xa1;
    let sh = |o: u32| match (globals, o < 0x30) {
        (false, _) => o,
        (true, true) => o + 8,
        (true, false) => o + 6,
    };
    let w = |o: u32| img.word(f + sh(o));
    let d8 = |o: u32| disp8(img, f + sh(o));
    let (m, src, arg) = if globals {
        let a = g.read::<u32>(img.word(f + 0x14));
        let s = g.read::<u32>(img.word(f + 0x1c));
        (fr.ecx, s, a)
    } else {
        (fr.ecx, fr.edx, fr.arg(g, 0))
    };
    let ebp = fr.esp.wrapping_sub(4);
    // the frame: 0x90 bytes and three saved registers
    let esp = ebp.wrapping_sub(0x90 + 12);
    let local = |o: u32| ebp.wrapping_sub(o);
    g.write::<u32>(local(0x70), arg);
    let t = w(0x1d);
    let s = slot_target(g, t, w(0x25) / 4);
    // call width: 3
    g.call(s, fr.ecx, &[w(0x2c), d8(0x2a), d8(0x22)]);
    // the model's fields: `lea`/`mov` displacements from the code
    let n = m.wrapping_add(d8(0x38));
    let root = w(0x4b);
    g.write::<u16>(n.wrapping_add(0x1a), 0xff);
    g.write::<u32>(n.wrapping_add(0x10), n);
    g.write::<u32>(n.wrapping_add(0xc), n);
    g.write::<u32>(n.wrapping_add(0x14), root);
    g.write::<u32>(n.wrapping_add(4), w(0x52));
    let last = g.read::<u32>(w(0x57));
    g.write::<u32>(n.wrapping_add(8), last);
    let last = g.read::<u32>(w(0x5f));
    if last == w(0x64) {
        g.write::<u32>(w(0x6c), n);
    } else {
        g.write::<u32>(last.wrapping_add(4), n);
    }
    let count = g.read::<u16>(w(0x78));
    g.write::<u16>(w(0x78), count.wrapping_add(1));
    g.write::<u32>(w(0x93), n);
    g.write::<u32>(n, w(0x99));
    let s = slot_target(g, w(0x8d), w(0x9f) / 4);
    let words = [
        m.wrapping_add(d8(0x8a)),
        src.wrapping_add(d8(0x7f)),
        w(0x83),
    ];
    // call width: 3
    g.call(s, fr.ecx, &words);
    let group = local(0x8c);
    let bone_node = m.wrapping_add(w(0xaa));
    g.write::<u32>(m.wrapping_add(d8(0xb9)), w(0xba));
    g.write::<u32>(group, bone_node);
    g.write::<u32>(group.wrapping_add(4), w(0xca));
    g.write::<u32>(group.wrapping_add(8), w(0xd4));
    g.write::<u32>(group.wrapping_add(0xc), n);
    g.write::<u32>(group.wrapping_add(0x10), arg);
    g.write::<u32>(group.wrapping_add(0x14), w(0xe1));
    let insert = Frame {
        at: img.rel32_target(f + sh(0xe6)),
        esp: esp.wrapping_sub(4),
        ecx: group,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    let pass = g.read::<u8>(w(0xeb));
    let cursor = local(0x74);
    g.write::<u32>(cursor, bone_node);
    g.write::<u8>(cursor.wrapping_add(5), pass);
    let bone = Frame {
        at: img.rel32_target(f + sh(0xf9)),
        esp: esp.wrapping_sub(4),
        ecx: cursor,
        ..*fr
    };
    crate::sa_summon::bone_plain(g, img, &bone);
    let (v1, v2, mat) = (local(0x18), local(0x28), local(0x6c));
    for (k, off) in (0u32..).zip([0x110u32, 0x117, 0x11e, 0x125]) {
        g.write::<u32>(v1.wrapping_add(4 * k), w(off));
    }
    for (k, off) in (0u32..).zip([0x12cu32, 0x133, 0x13a, 0x141]) {
        g.write::<u32>(v2.wrapping_add(4 * k), w(off));
    }
    let t = w(0x109);
    let s = slot_target(g, t, w(0x147) / 4);
    let mut stack = vec![mat, m.wrapping_add(w(0xff))];
    // call width: 2
    g.call(s, fr.ecx, &stack);
    let s = slot_target(g, w(0x153), w(0x159) / 4);
    stack.splice(0..0, [mat, mat]);
    // call width: 4 (the earlier call's words stay on the stack)
    g.call(s, fr.ecx, &stack);
    let s = slot_target(g, w(0x16e), w(0x176) / 4);
    stack.splice(0..0, [1, mat, v1, v2, 0]);
    // call width: 9 (the earlier calls' words stay on the stack)
    g.call(s, fr.ecx, &stack);
    let s = slot_target(g, w(0x17d), w(0x186) / 4);
    // call width: 1
    let eax = g.call(s, fr.ecx, &[w(0x18b)]).eax;
    g.write::<u32>(m.wrapping_add(d8(0x199)), w(0x19a));
    Ret::eax(eax)
}

include!("o6_fam_bone_table.rs.in");
