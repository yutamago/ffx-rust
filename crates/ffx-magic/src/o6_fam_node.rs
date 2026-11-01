//! Round 6 shape families: a node's insertion and reset with its sound
//! handle (fuzzy family `F0577_100036d0` of
//! `~/ffx-work/m5/magic/r4/families.tsv`; 3 copies: `magic_0577`
//! 0x100036d0, 498 bytes, and `magic_0155` 0x10002fc0, `magic_0070`
//! 0x10003ed0, 513 bytes, which call slot 281 too).

use crate::o6_fam_cursor::Cursor;
use crate::runtime::slot_target;
use crate::sa_summon::st0_f32;
use crate::{Game, Image, Sig};

/// Inserts and resets a node (internal: `ecx` = a record `[node,
/// parent, x]`; no stack words): the node (pass byte `+0x1a` 0xff, an
/// empty child list, its parent `+0x14`) is put first among the parent's
/// children when there is one (the parent's count `+0x18` goes up); its
/// matrices and fields set from the copy's constant stores (u16 `+0x98`
/// and byte `+0x137` 0); the handle `h` =
/// slot 287 `(1)` into `+0x130`; slot 282's float (ST0) into `+0x68`;
/// `x` into `+0x64` and `+0xc8`, `+0xe8` 0; in the two longer copies slot
/// 281 `(1)`; then slot 283 `(h, 0)` and `(h, 1)`. The slot calls see the
/// words pushed before them (`7, 0` first). Returns the last call's
/// `eax`.
pub fn node_reset_sound<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32) -> u32 {
    let node = g.read::<u32>(rec);
    let parent = g.read::<u32>(rec.wrapping_add(4));
    g.write::<u16>(node.wrapping_add(0x1a), 0xff);
    g.write::<u32>(node.wrapping_add(0x10), node);
    g.write::<u32>(node.wrapping_add(0xc), node);
    g.write::<u32>(node.wrapping_add(0x14), parent);
    if parent != 0 {
        g.write::<u32>(node.wrapping_add(4), parent);
        let first = g.read::<u32>(parent.wrapping_add(0x10));
        g.write::<u32>(node.wrapping_add(8), first);
        let first = g.read::<u32>(parent.wrapping_add(0x10));
        if first == parent {
            g.write::<u32>(parent.wrapping_add(0xc), node);
        } else {
            g.write::<u32>(first.wrapping_add(4), node);
        }
        let n = g.read::<u16>(parent.wrapping_add(0x18));
        g.write::<u16>(parent.wrapping_add(0x18), n.wrapping_add(1));
        g.write::<u32>(parent.wrapping_add(0x10), node);
    }
    // the reset: a run of constant stores (`mov [esi + d], imm`; the
    // copies differ in some values: identity rows or zeros), the u16
    // `+0x98` cleared with the first two pushes, and a byte store, up to
    // the first slot load
    let mut c = Cursor::new(img, f + 0x3b);
    loop {
        match img.word(c.at) as u8 {
            0xc7 => {
                let (d, v) = c.store_imm();
                g.write::<u32>(node.wrapping_add(d), v);
            }
            // xor eax, eax; push 7; push eax; mov word [esi + d], ax
            0x33 => {
                c.skip(5);
                let d = c.word(3, 7);
                g.write::<u16>(node.wrapping_add(d), 0);
            }
            // mov eax, [T]
            0xa1 => {
                c.skip(5);
            }
            // mov byte [esi + d], imm8
            0xc6 => {
                let d = c.word(2, 0);
                let v = img.word(c.at + 6) as u8;
                c.skip(7);
                g.write::<u8>(node.wrapping_add(d), v);
            }
            _ => break,
        }
    }
    // the slot table global (every `mov eax, [T]`, the one at +0x196)
    let t = img.word(f + 0x197);
    let long =
        img.matches(f, &NODE_RESET_SOUND_0155_2FC0) || img.matches(f, &NODE_RESET_SOUND_0070_3ED0);
    let mut stack = vec![1u32, 0, 7];
    // call width: 3 (the earlier pushes stay on the stack)
    let h = g.call(slot_target(g, t, 287), rec, &stack).eax;
    g.write::<u32>(node.wrapping_add(0x130), h);
    // call width: 3 (the earlier pushes stay on the stack)
    let out = g.call(slot_target(g, t, 282), rec, &stack);
    g.write::<u32>(node.wrapping_add(0x68), st0_f32(out));
    let x = g.read::<u32>(rec.wrapping_add(8));
    g.write::<u32>(node.wrapping_add(0x64), x);
    g.write::<u32>(node.wrapping_add(0xc8), x);
    g.write::<u32>(node.wrapping_add(0xe8), 0);
    if long {
        stack.insert(0, 1);
        // call width: 4 (the earlier pushes stay on the stack)
        g.call(slot_target(g, t, 281), rec, &stack);
    }
    let h = g.read::<u32>(node.wrapping_add(0x130));
    stack.splice(0..0, [h, 0]);
    // call width: 6 (the earlier pushes stay on the stack)
    g.call(slot_target(g, t, 283), rec, &stack);
    let h = g.read::<u32>(node.wrapping_add(0x130));
    stack.splice(0..0, [h, 1]);
    // call width: 8 (the earlier pushes stay on the stack)
    g.call(slot_target(g, t, 283), rec, &stack).eax
}

include!("o6_fam_node_table.rs.in");
