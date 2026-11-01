//! Round 6 shape families: effect object starts with the first part of
//! the effect's program list (families `F0074_10001c70` and
//! `F0369_10002c10` of `~/ffx-work/m5/magic/r4/families.tsv`; 6 copies in
//! `magic_0074`, `0247`, `0200`, `0369`, `0166`, `0606`; and
//! `F0102_10002140`'s `magic_0132`, `0126`, whose entry is indexed by a
//! stack word). They are near
//! copies of [`crate::s4_misc::start_object_at`]'s shape: with or without
//! a frame, another program entry, and one of four state resets (told
//! apart by the callee's identity), so the port walks the code with a
//! [`Cursor`].

use crate::l5_objhost::pool_alloc;
use crate::m0032::{RESET_STATE_EDX, reset_state};
use crate::m0166::reset_record;
use crate::o6_fam_cursor::Cursor;
use crate::opu_parts::start_part;
use crate::runtime::slot_target;
use crate::sa_misc::{
    RESET_TRANSFORM, RESET_TRANSFORM_0, RESET_TRANSFORM_14, reset_transform, reset_transform_14,
};
use crate::sb_misc::{ANIM_RESET_2, anim_reset_2};
use crate::{Callee, Frame, Game, Image, Sig};

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale; with a frame (`magic_0074` 0x10001c70, 239 bytes, 3 copies) the
/// stack holds `(-, owner, -, value)`, without one (`magic_0369`
/// 0x10002c10, 225 bytes, 3 copies) nothing is read from it):
/// - the model words: `+0x40..+0x48` and `+0x50..+0x5c` 0, `+0x4c` 1.0,
///   the scale into `+0x60..+0x6c` (each store's operands from the code);
/// - `*G_P = ITEM`; the program list `L` = slot 941 `((*G_D)[0x10])`
///   and its entry `L[k]` (`push [eax]` or `push [eax + 4k]`) started on
///   the object (`start_part`, `edx` 0x1000, `(L[k], 1)`);
/// - the state reset the copy calls: `magic_0032`'s with `+0x114` =
///   `owner` and `+0x100` = `I` (an immediate), [`reset_transform`] or
///   [`reset_transform_14`] with `owner` and `I` (or `value`),
///   [`anim_reset_2`] with its two stack words,
///   [`crate::sa_misc::reset_transform_0`] or `magic_0166`'s record reset;
/// - a pool node ([`pool_alloc`]) holding the object appended to the
///   object list (first and last node `G_FIRST`, `G_LAST`, count `G_N`);
///   nothing when the pool is empty.
///
/// Returns the object.
#[allow(clippy::too_many_lines)] // the reset's set-up words and the list append, in order
pub fn start_object_first<G: Game>(g: &mut G, img: &Image, f: u32, fr: &Frame) -> u32 {
    let obj = fr.edx;
    let scale = fr.xmm[3] as u32;
    let framed = img.word(f) as u8 == 0x55;
    let mut c = Cursor::new(img, f + if framed { 3 } else { 0 });
    // mov eax, [G_P]; push esi; mov esi, edx
    let gp = c.word(1, 5);
    c.skip(3);
    let p = g.read::<u32>(gp);
    for _ in 0..8 {
        let (d, v) = c.store_imm();
        g.write::<u32>(obj.wrapping_add(d), v);
    }
    for _ in 0..4 {
        // movss [esi + d8], xmm3
        let d = c.disp8(4, 5);
        g.write::<u32>(obj.wrapping_add(d), scale);
    }
    // mov dword [eax], ITEM
    g.write::<u32>(p, c.word(2, 6));
    // mov eax, [G_D]; push [eax + 0x10]
    let gd = c.word(1, 5);
    let d = c.disp8(2, 3);
    let arg = g.read::<u32>(g.read::<u32>(gd).wrapping_add(d));
    // mov eax, [T]; mov eax, [eax + S]; call eax
    let t = c.word(1, 5);
    let slot = c.slot_load();
    c.skip(2);
    let list = g.call(slot_target(g, t, slot), fr.ecx, &[arg]).eax;
    // the entry's index: `mov ecx, [ebp + d8]` (a stack word) or none
    let index = if img.word(c.at) as u8 == 0x8b {
        let d = c.disp8(2, 3);
        g.read::<u32>(fr.esp.wrapping_add(d).wrapping_sub(4))
    } else {
        0
    };
    // push 0; push 1; push [eax] / [eax + d8] / [eax + ecx * 4]
    // push 0: the third word, which start_part passes as its own 0
    c.push_imm();
    let one = c.push_imm();
    let entry = match img.word(c.at + 1) as u8 {
        0x30 => {
            c.skip(2);
            0
        }
        0x34 => {
            c.skip(3);
            index.wrapping_mul(4)
        }
        _ => c.disp8(2, 3),
    };
    let program = g.read::<u32>(list.wrapping_add(entry));
    // mov edx, 0x1000; mov ecx, esi; call start_part
    let edx = c.word(1, 5);
    c.skip(2);
    start_part(g, img, img.rel32_target(c.at + 1), obj, edx, program, one);
    c.skip(5);
    // the reset's set-up, up to its call: `mov edx, [ebp + d8]` (its
    // owner), `add esp, n` (start_part's words go), the words it gets
    // (`push imm`, `push [ebp + d8]`, `push ecx`, which no reset reads)
    // and `mov ecx, esi`. `pending` counts the words on the stack below
    // the entry's two pushes (start_part's four first).
    let (mut owner, mut words, mut pending) = (0u32, Vec::new(), 4u32);
    let stack_word = |g: &G, d: u32| g.read::<u32>(fr.esp.wrapping_add(d).wrapping_sub(4));
    loop {
        match (img.word(c.at) as u8, img.word(c.at + 1) as u8) {
            (0x8b, 0x55) => {
                let d = c.disp8(2, 3);
                owner = stack_word(g, d);
            }
            (0x8b, 0xce) => {
                c.skip(2);
            }
            (0x83, _) => pending -= c.disp8(2, 3) / 4,
            (0xff, _) => {
                let d = c.disp8(2, 3);
                words.insert(0, stack_word(g, d));
                pending += 1;
            }
            (0x51, _) => {
                c.skip(1);
                words.insert(0, 0);
                pending += 1;
            }
            (0x6a | 0x68, _) => {
                words.insert(0, c.push_imm());
                pending += 1;
            }
            _ => break,
        }
    }
    let second = words.get(1).copied().unwrap_or(0);
    let reset = img.rel32_target(c.at + 1);
    if img.matches(reset, &RESET_STATE_EDX) {
        reset_state(g, obj, second, owner, 0x12);
    } else if img.matches(reset, &RESET_TRANSFORM) {
        reset_transform(g, obj, owner, second);
    } else if img.matches(reset, &RESET_TRANSFORM_14) {
        reset_transform_14(g, obj, owner, second);
    } else if img.matches(reset, &RESET_TRANSFORM_0) {
        reset_transform(g, obj, 0, 0);
    } else if img.matches(reset, &ANIM_RESET_2) {
        // it reads its two words from the stack: put them where the
        // original's are (below `push ebp; push esi`, the pending words
        // and the return address)
        let esp = fr.esp.wrapping_sub(8 + 4 * pending + 4);
        for (i, &v) in (1u32..).zip(&words) {
            g.write::<u32>(esp.wrapping_add(4 * i), v);
        }
        anim_reset_2(g, esp, obj);
    } else {
        reset_record(g, img, reset, obj);
    }
    // call; add esp, n; call pool_alloc
    c.skip(5 + 3);
    let node = pool_alloc(g, img, img.rel32_target(c.at + 1));
    c.skip(5);
    if node == 0 {
        return obj;
    }
    // x: mov ecx, eax; test; je; mov eax, [G_LAST] (+7); mov [ecx + 4], 0
    let x = c.at;
    let w = |o: u32| img.word(x + o);
    let old_tail = g.read::<u32>(w(0x07));
    g.write::<u32>(node.wrapping_add(4), 0);
    g.write::<u32>(node, old_tail);
    if old_tail == 0 {
        g.write::<u32>(w(0x1a), node);
        g.write::<u32>(w(0x20), node);
        g.write::<u32>(node.wrapping_add(8), obj);
        let n = g.read::<u32>(w(0x29));
        g.write::<u32>(w(0x29), n.wrapping_add(1));
    } else {
        // the second branch after the first's epilogue
        let e = x + if framed { 0x32 } else { 0x31 };
        let w = |o: u32| img.word(e + o);
        let tail = g.read::<u32>(w(0x03));
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(w(0x0c), node);
        g.write::<u32>(node.wrapping_add(8), obj);
        let n = g.read::<u32>(w(0x15));
        g.write::<u32>(w(0x15), n.wrapping_add(1));
    }
    obj
}

include!("o6_fam_object_table.rs.in");
