//! Round 11 (magic-r11-own): the start-ups with one object at the first
//! actor, two fades ([`fade_task`](crate::s3_misc::fade_task)) and a task
//! node (`magic_0101`, `0105`, ...), and the pieces they share.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `mov dword [G], imm32` at each instruction offset of `ats` (operand
/// `+2`, immediate `+6`).
pub(crate) fn stores<G: Game>(g: &mut G, img: &Image, f: u32, ats: &[u32]) {
    for &at in ats {
        g.write::<u32>(img.word(f + at + 2), img.word(f + at + 6));
    }
}

/// `mov dword [G], 0` at each instruction offset of `ats`.
pub(crate) fn zeros<G: Game>(g: &mut G, img: &Image, f: u32, ats: &[u32]) {
    for &at in ats {
        g.write::<u32>(img.word(f + at + 2), 0);
    }
}

/// One fade: `xmm2`/`xmm1` the floats at the operands `x2`/`x1`, `edx`
/// and `ecx` (`rec`, an operand) given, the pushed `word` at `esp + 4`
/// (the callee's frame `esp`), [`fade_task`](crate::s3_misc::fade_task)
/// at the `rel32` `call`. Returns the node.
pub(crate) fn fade<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    esp: u32,
    (x2, x1, word, edx, rec, call): (u32, u32, u32, u32, u32, u32),
) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(esp + 4, word);
    let fade = Frame {
        at: img.rel32_target(f + call),
        esp,
        ecx: w(rec),
        edx,
        xmm: [
            0,
            u128::from(g.read::<u32>(w(x1))),
            u128::from(g.read::<u32>(w(x2))),
            0,
        ],
        ..*fr
    };
    crate::s3_misc::fade_task(g, img, &fade)
}

/// A pool node ([`take`](crate::sa_pool::take), `rel32` `call`, the owner
/// the `mov ecx, imm32` before it) goes last on the task list: `first`
/// the head word tested, `last` the tail read, then the tail written
/// (`last_w` when the list was not empty, `last_e` when it was, with the
/// head `first_w`).
pub(crate) fn task_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    call: u32,
    (first, last, last_w, first_w, last_e): (u32, u32, u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(call - 5));
    let head = g.read::<u32>(w(first));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(first_w), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(last_e), node);
    } else {
        let tail = g.read::<u32>(w(last));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(last_w), node);
    }
}

/// The first actor's character `CHR[SEL[0]]` (operands `sel`, `chr`).
fn first_actor<G: Game>(g: &G, img: &Image, f: u32, sel: u32, chr: u32) -> u32 {
    let s = g.read::<u32>(img.word(f + sel));
    let b = u32::from(g.read::<u8>(s));
    g.read::<u32>(img.word(f + chr).wrapping_add(4 * b))
}

/// The start-up with an object and two fades (internal, no arguments, no
/// frame; `magic_0105` 0x10003130, 422 bytes): `*G_A`/`*G_B` set (`+0x17`,
/// `+0x21`), slot 719 `(NAME, 8, 0)` into `G_PART` (`+0x2f`), the part set
/// up ([`part_init`](crate::l5_objhost::part_init)); the object `OBJ`
/// (`+0x44`, scale `*K` `+0x3c`) starts
/// ([`start_object_reset`](crate::s6_misc::start_object_reset)); the first
/// actor's character (`+0x4e`, `+0x62`) puts the point `P` (`+0x6c`) with
/// slots 262 and 268, `P.w` (`+0xab`) 1.0; the record words `+0xb5` ..
/// `+0x10b` (0, the immediates, a halfword 0); two fades (`ecx` `+0xa5`,
/// `edx` 8, the word 0; `ecx` `+0x130`, `edx` 0x10, the word 0x45, a word
/// set between); a word set (`+0x14a`), slot 271 `(4)`; a pool node
/// (owner `+0x160`) appended to the task list. Returns 0.
pub fn object_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let s719 = slot_target(g, w(0x01), 0xb3c / 4);
    stores(g, img, f, &[0x15, 0x1f]);
    let part = g.call(s719, 0, &[w(0x11), 8, 0]).eax;
    g.write::<u32>(w(0x2f), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x34));
    // push esi; three pushes, one popped; sub esp, 8; call
    let start = Frame {
        at: img.rel32_target(f + 0x49),
        esp: e - 0x18,
        edx: w(0x44),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x3c)))],
        ..*fr
    };
    crate::s6_misc::start_object_reset(g, img, &start);
    let chr = first_actor(g, img, f, 0x4e, 0x62);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x53), w(0x5b), w(0x6c), chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x67), 0x418 / 4), 0);
    s.push_all(&[w(0x7f), 0x11, chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x7a), 0x430 / 4), 0);
    stores(g, img, f, &[0xa9]);
    zeros(g, img, f, &[0xb3, 0xbd, 0xc7, 0xd1]);
    stores(g, img, f, &[0xdb, 0xe5]);
    zeros(g, img, f, &[0xef, 0xf9]);
    g.write::<u16>(w(0x105), 0);
    stores(g, img, f, &[0x109]);
    let call = |k: u32| e - 0x24 - 0xc - 4 * k - 4;
    fade(g, img, fr, call(1), (0x92, 0x9a, 0, 8, 0xa5, 0x114));
    let word = img.word(f + 0x129) & 0xff;
    let edx = w(0x12b);
    stores(g, img, f, &[0x134]);
    fade(g, img, fr, call(2), (0x11c, 0x124, word, edx, 0x130, 0x13f));
    let s271 = g.read::<u32>(w(0x144));
    stores(g, img, f, &[0x148]);
    s.push_all(&[0, word, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, g.read::<u32>(s271 + 0x43c), 0);
    task_node(g, img, f, 0x165, (0x16b, 0x17c, 0x189, 0x197, 0x19f));
    Ret::eax(0)
}

/// [`object_two_fades`] (`magic_0105` 0x10003130).
pub const OBJECT_TWO_FADES_0105: Sig = Sig {
    len: 422,
    hash: 0x71ed_8395_8057_f300,
    calls: &[
        (0x34, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x49, Callee::Sig(&crate::s7_misc::START_OBJECT_RESET_1E)),
        (0x114, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x13f, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x165, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
