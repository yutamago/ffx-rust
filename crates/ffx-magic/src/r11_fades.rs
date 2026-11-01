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
/// head `first_w`). `flag`: the immediate of a `mov byte [eax + 0x18],
/// imm8` after the test.
pub(crate) fn task_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    call: u32,
    flag: Option<u32>,
    (first, last, last_w, first_w, last_e): (u32, u32, u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(call - 5));
    let head = g.read::<u32>(w(first));
    if let Some(at) = flag {
        g.write::<u8>(node.wrapping_add(0x18), img.word(f + at).to_le_bytes()[0]);
    }
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
    task_node(g, img, f, 0x165, None, (0x16b, 0x17c, 0x189, 0x197, 0x19f));
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

/// The head of the fade start-ups (no frame but `push esi`): slot 719
/// `(NAME, 8, 0)` (table `+0x01`, name `+0x11`) with the stores at the
/// instruction offsets `stores` before the call, the handle into the
/// operand `handle`, the part set up (`part_init`, `rel32` `part`).
fn fade_head<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    stores_at: &[u32],
    (handle, part): (u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let s719 = slot_target(g, w(0x01), 0xb3c / 4);
    stores(g, img, f, stores_at);
    let h = g.call(s719, 0, &[w(0x11), 8, 0]).eax;
    g.write::<u32>(w(handle), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + part));
}

/// The first actor's point: slot 262 `(chr, P, P + 4, P + 8)` (table
/// `t262`, the three immediates pushed `z`, `y`, `x`) and slot 268 `(chr,
/// 0x11, P)` (table `t268`, `P` the immediate `p`), `chr` the first
/// actor's character (`sel`, `chr`). Returns the pushes (7 words).
fn actor_point<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (sel, chr, t262, z, y, x, t268, p): (u32, u32, u32, u32, u32, u32, u32, u32),
) -> crate::pushes::Pushes {
    let w = |o: u32| img.word(f + o);
    let c = first_actor(g, img, f, sel, chr);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(z), w(y), w(x), c]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(t262), 0x418 / 4), 0);
    s.push_all(&[w(p), 0x11, c]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(t268), 0x430 / 4), 0);
    s
}

/// `mov ds:G, eax` (`a3`, operand `+1`) with `eax` 0 at each instruction
/// offset of `ats`.
fn zero_eax<G: Game>(g: &mut G, img: &Image, f: u32, ats: &[u32]) {
    for &at in ats {
        g.write::<u32>(img.word(f + at + 1), 0);
    }
}

/// `magic_0101` 0x10002fa0 (509 bytes): [`object_two_fades`] with more
/// words set before the fades, the colour task
/// ([`colour_task`](crate::s4_small::colour_task)) before them, and the
/// task node's byte `+0x18` 2. Returns 0.
pub fn object_colour_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    fade_head(g, img, f, &[0x15, 0x1f], (0x2f, 0x34));
    let start = Frame {
        at: img.rel32_target(f + 0x49),
        esp: e - 0x18,
        edx: w(0x44),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x3c)))],
        ..*fr
    };
    crate::s6_misc::start_object_reset(g, img, &start);
    let mut s = actor_point(g, img, f, (0x4e, 0x62, 0x67, 0x53, 0x5b, 0x6c, 0x7a, 0x7f));
    stores(
        g,
        img,
        f,
        &[0x90, 0x9a, 0xa4, 0xae, 0xb8, 0xc2, 0xcc, 0xd6, 0xe0],
    );
    g.write::<u16>(w(0xec), 0);
    stores(
        g,
        img,
        f,
        &[0xf0, 0xfa, 0x104, 0x10e, 0x118, 0x122, 0x12c, 0x136],
    );
    zero_eax(g, img, f, &[0x140]);
    let colour = Frame {
        at: img.rel32_target(f + 0x146),
        esp: e - 0x34,
        ..*fr
    };
    crate::s4_small::colour_task(g, img, &colour);
    let word1 = crate::r11_misc::byte(img, f + 0x15b);
    fade(
        g,
        img,
        fr,
        e - 0x38,
        (0x14e, 0x156, word1, w(0x15d), 0x162, 0x167),
    );
    let word2 = crate::r11_misc::byte(img, f + 0x17c);
    let edx = w(0x17e);
    stores(g, img, f, &[0x187]);
    fade(
        g,
        img,
        fr,
        e - 0x3c,
        (0x16f, 0x177, word2, edx, 0x183, 0x192),
    );
    let s271 = g.read::<u32>(w(0x197));
    stores(g, img, f, &[0x19b]);
    s.push_all(&[word1, word2, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, g.read::<u32>(s271 + 0x43c), 0);
    task_node(
        g,
        img,
        f,
        0x1b8,
        Some(0x1c6),
        (0x1be, 0x1d3, 0x1e0, 0x1ee, 0x1f6),
    );
    Ret::eax(0)
}

/// [`object_colour_two_fades`] (`magic_0101` 0x10002fa0).
pub const OBJECT_COLOUR_TWO_FADES_0101: Sig = Sig {
    len: 509,
    hash: 0xde5c_6749_1c01_105a,
    calls: &[
        (0x34, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x49, Callee::Sig(&crate::s7_misc::START_OBJECT_RESET_1E)),
        (0x146, Callee::Sig(&crate::s4_small::COLOUR_TASK)),
        (0x167, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x192, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x1b8, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0093` 0x10003250 (472 bytes): [`object_two_fades`] with three
/// head words, the object started by
/// [`part_object`](crate::m0093::part_object), more words set before the
/// fades (four of them 0 from `eax`). Returns 0.
pub fn part_object_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    fade_head(g, img, f, &[0x15, 0x1f, 0x29], (0x39, 0x3e));
    let scale = g.read::<u32>(w(0x46));
    crate::m0093::part_object(g, img, img.rel32_target(f + 0x53), w(0x4e), scale);
    let mut s = actor_point(g, img, f, (0x58, 0x6c, 0x71, 0x5d, 0x65, 0x76, 0x84, 0x89));
    stores(
        g,
        img,
        f,
        &[0xb3, 0xbd, 0xc7, 0xd1, 0xdb, 0xe5, 0xef, 0xf9, 0x103],
    );
    g.write::<u16>(w(0x10f), 0);
    stores(g, img, f, &[0x113, 0x11d, 0x127]);
    zero_eax(g, img, f, &[0x131, 0x136, 0x13b, 0x140]);
    fade(g, img, fr, e - 0x38, (0x9c, 0xa4, 0, 8, 0xaf, 0x146));
    let word2 = crate::r11_misc::byte(img, f + 0x15b);
    let edx = w(0x15d);
    stores(g, img, f, &[0x166]);
    fade(
        g,
        img,
        fr,
        e - 0x3c,
        (0x14e, 0x156, word2, edx, 0x162, 0x171),
    );
    let s271 = g.read::<u32>(w(0x176));
    stores(g, img, f, &[0x17a]);
    s.push_all(&[0, word2, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, g.read::<u32>(s271 + 0x43c), 0);
    task_node(g, img, f, 0x197, None, (0x19d, 0x1ae, 0x1bb, 0x1c9, 0x1d1));
    Ret::eax(0)
}

/// [`part_object_two_fades`] (`magic_0093` 0x10003250).
pub const PART_OBJECT_TWO_FADES_0093: Sig = Sig {
    len: 472,
    hash: 0xf84a_6c72_3dfa_3231,
    calls: &[
        (0x3e, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x53, Callee::Sig(&crate::m0093::PART_OBJECT)),
        (0x146, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x171, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x197, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0085` 0x100031d0 (546 bytes, a frame with one local): the head
/// with three words (operands 4 bytes later than [`object_two_fades`]'),
/// the object ([`start_object_reset`](crate::s6_misc::start_object_reset)),
/// the first actor's point; slot 333 `(*A)` (`+0xa3`) gives the yaw `K -
/// v` (`+0xc0`) wrapped once (above `HI` `+0xd0` less `T` `+0xda`, below
/// `LO` `+0xe4` plus `T` `+0xf1`; a NaN stays) into `+0x10b`; the record
/// words, two fades (`xmm1`/`xmm2` from `+0xf9`/`+0x101`, then `+0xf9`
/// for `xmm2` and `+0x1ac` for `xmm1`), slot 271 `(4)`, the task node.
/// Returns 0.
pub fn yaw_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.esp;
    // push ebp; push ecx; push esi: the head's code 4 bytes later
    fade_head(g, img, f + 4, &[0x15, 0x1f, 0x29], (0x39, 0x3e));
    let start = Frame {
        at: img.rel32_target(f + 0x57),
        esp: e - 0x20,
        edx: w(0x52),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x4a)))],
        ..*fr
    };
    crate::s6_misc::start_object_reset(g, img, &start);
    let mut s = actor_point(g, img, f, (0x5c, 0x70, 0x75, 0x61, 0x69, 0x7a, 0x88, 0x8d));
    let t333 = g.read::<u32>(w(0x9d));
    s.push(g.read::<u32>(w(0xa3)));
    stores(g, img, f, &[0xa7]);
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(s.call(g, g.read::<u32>(t333 + 0x534), 0).st0);
    let x = sub(k(g, 0xc0), v);
    let yaw = if x > k(g, 0xd0) {
        sub(x, k(g, 0xda))
    } else if k(g, 0xe4) > x {
        add(x, k(g, 0xf1))
    } else {
        x
    };
    g.write::<f32>(w(0x10b), yaw);
    stores(
        g,
        img,
        f,
        &[0x11b, 0x125, 0x12f, 0x139, 0x143, 0x14d, 0x157, 0x161],
    );
    g.write::<u16>(w(0x16d), 0);
    stores(g, img, f, &[0x171, 0x17b, 0x185]);
    zero_eax(g, img, f, &[0x18f, 0x194, 0x199, 0x19e]);
    fade(g, img, fr, e - 0x10, (0x101, 0xf9, 0, 8, 0x117, 0x1a4));
    let word2 = crate::r11_misc::byte(img, f + 0x1b1);
    fade(
        g,
        img,
        fr,
        e - 0x14,
        (0xf9, 0x1ac, word2, w(0x1b3), 0x1bb, 0x1c0),
    );
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[0, word2, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1c5), 0x43c / 4), 0);
    task_node(g, img, f, 0x1dc, None, (0x1e2, 0x1f2, 0x1ff, 0x210, 0x218));
    Ret::eax(0)
}

/// [`yaw_two_fades`] (`magic_0085` 0x100031d0).
pub const YAW_TWO_FADES_0085: Sig = Sig {
    len: 546,
    hash: 0x601f_1ee2_a3af_225e,
    calls: &[
        (0x42, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x57, Callee::Sig(&crate::s6_misc::START_OBJECT_RESET)),
        (0x1a4, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x1c0, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x1dc, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0103` 0x10003380 (462 bytes, a frame of 0x10): the head (slot
/// 719, two words, the part set up); the actors' mean position into the
/// frame (`ebp - 0x10`; [`mean_position`](crate::sb_misc::mean_position));
/// two objects start ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2):
/// `OBJ1` `+0x5b` with `(0, -, 0x1e, CB)` (`+0x56`, `+0x51`), `OBJ2`
/// `+0x96` with `(1, -, 0, 0)`; scales `+0x4c`, `+0x6d`), each followed by
/// the mean position copied into a point (`+0x7b..`, `+0x120..`), the
/// record words between them; two fades, slot 271 `(4)`, the task node.
/// Returns 0.
pub fn mean_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let out = e - 4 - 0x10;
    let s719 = slot_target(g, w(0x07), 0xb3c / 4);
    stores(g, img, f, &[0x1a, 0x24]);
    let h = g.call(s719, 0, &[w(0x16), 8, 0]).eax;
    g.write::<u32>(w(0x34), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x39));
    crate::sb_misc::mean_position(g, img, img.rel32_target(f + 0x44), e - 0x18, out);
    // the starts' four words below the frame: (index, ecx, a, b)
    let below = e - 0x24;
    let start = |g: &mut G, (scale, obj, call): (u32, u32, u32), args: [u32; 4]| {
        for (i, v) in (0u32..).zip(args) {
            g.write::<u32>(below + 4 * i, v);
        }
        let fr2 = Frame {
            at: img.rel32_target(f + call),
            esp: below - 4,
            edx: w(obj),
            xmm: [0, 0, 0, u128::from(g.read::<u32>(w(scale)))],
            ..*fr
        };
        crate::s6_misc::start_object_anim_2(g, img, &fr2);
    };
    let a = crate::r11_misc::byte(img, f + 0x56);
    start(g, (0x4c, 0x5b, 0x60), [0, 0, a, w(0x51)]);
    let copy = |g: &mut G, ops: [u32; 3]| {
        for (i, o) in (0u32..).zip(ops) {
            let v = g.read::<u32>(out + 4 * i);
            g.write::<u32>(w(o), v);
        }
    };
    copy(g, [0x7b, 0x89, 0x9e]);
    stores(g, img, f, &[0xa2, 0xac, 0xb6, 0xc0, 0xca, 0xd4, 0xde, 0xe8]);
    g.write::<u16>(w(0xf4), 0);
    stores(g, img, f, &[0xf8]);
    start(g, (0x6d, 0x96, 0x103), [1, 0, 0, 0]);
    copy(g, [0x120, 0x12d, 0x146]);
    let word1 = crate::r11_misc::byte(img, f + 0x137);
    fade(
        g,
        img,
        fr,
        e - 0x2c,
        (0x110, 0x118, word1, w(0x139), 0x13e, 0x14b),
    );
    let word2 = crate::r11_misc::byte(img, f + 0x160);
    fade(
        g,
        img,
        fr,
        e - 0x30,
        (0x153, 0x15b, word2, w(0x162), 0x167, 0x16c),
    );
    let mut s = crate::pushes::Pushes::default();
    // the second start's words below: its `ecx` (whatever the first left)
    s.push_all(&[1, word1, word2, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x171), 0x43c / 4), 0);
    task_node(g, img, f, 0x188, None, (0x18e, 0x19e, 0x1ab, 0x1bc, 0x1c4));
    Ret::eax(0)
}

/// [`mean_two_fades`] (`magic_0103` 0x10003380).
pub const MEAN_TWO_FADES_0103: Sig = Sig {
    len: 462,
    hash: 0x80af_4633_c848_be2f,
    calls: &[
        (0x39, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x44, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x60, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x103, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x14b, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x16c, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x188, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0100` 0x100013e0 (668 bytes): the head (two words, the part
/// set up), then by the actor's state
/// ([`state_listed`](crate::s4_misc::state_listed)) one of two objects
/// started ([`start_object_state`](crate::r9_misc::start_object_state),
/// scale `+0x45`): unlisted, `OBJ1` (`+0x55`, index 0) at the first
/// actor's point `P1` (slots 262 and 268, then slot 12's height less `K`
/// into `P1.y`, `P1.w` 1.0); listed, `OBJ2` (`+0xdf`, index 1) at `P2`
/// (slots 262 and 268, `P2.w` 1.0); the object's record (`+0x14e` ..
/// `+0x1a0`, into its `+0x110`) and the colour words, the colour task
/// ([`colour_task`](crate::s4_small::colour_task)), two fades (`K` and
/// `K5` `+0x146` crossed), slot 271 `(4)`, the task node (byte 2).
/// Returns 0.
#[allow(clippy::too_many_lines)]
pub fn state_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let s719 = slot_target(g, w(0x01), 0xb3c / 4);
    stores(g, img, f, &[0x16, 0x20]);
    let h = g.call(s719, 0, &[w(0x12), 8, 0]).eax;
    g.write::<u32>(w(0x30), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x35));
    let listed = Frame {
        at: img.rel32_target(f + 0x3d),
        esp: e - 0xc,
        ..*fr
    };
    let state = crate::s4_misc::state_listed(g, img, &listed).eax;
    let scale = g.read::<f32>(w(0x45));
    let (obj, k) = if state == 0 {
        let obj = w(0x55);
        let at = img.rel32_target(f + 0x5d);
        crate::r9_misc::start_object_state(g, img, at, obj, scale, 0);
        let mut s = actor_point(g, img, f, (0x62, 0x76, 0x7b, 0x67, 0x6f, 0x80, 0x8e, 0x93));
        s.push(first_actor(g, img, f, 0x62, 0x76));
        // call width: 12 (Pushes caps the words at 12)
        let top = s.call(g, slot_target(g, w(0xa3), 0x30 / 4), 0).st0;
        g.write::<f32>(w(0xaf), crate::m0018::st0_f32(top));
        let y = crate::sb_sse::sub(g.read::<f32>(w(0xb7)), g.read::<f32>(w(0xbf)));
        stores(g, img, f, &[0xca]);
        g.write::<f32>(w(0xd8), y);
        (obj, 0xbf)
    } else {
        let obj = w(0xdf);
        let at = img.rel32_target(f + 0xe8);
        crate::r9_misc::start_object_state(g, img, at, obj, scale, 1);
        actor_point(
            g,
            img,
            f,
            (0xed, 0x101, 0x106, 0xf2, 0xfa, 0x10b, 0x119, 0x11e),
        );
        stores(g, img, f, &[0x138]);
        (obj, 0x131)
    };
    stores(
        g,
        img,
        f,
        &[0x14c, 0x156, 0x160, 0x16a, 0x174, 0x17e, 0x188, 0x192],
    );
    g.write::<u16>(w(0x19e), 0);
    g.write::<u32>(obj + 0x110, w(0x1a8));
    stores(
        g,
        img,
        f,
        &[0x1ac, 0x1b6, 0x1c0, 0x1ca, 0x1d4, 0x1de, 0x1e8],
    );
    zero_eax(g, img, f, &[0x1f2]);
    let colour = Frame {
        at: img.rel32_target(f + 0x1f8),
        esp: e - 0xc,
        ..*fr
    };
    crate::s4_small::colour_task(g, img, &colour);
    let word1 = crate::r11_misc::byte(img, f + 0x1fd);
    fade(
        g,
        img,
        fr,
        e - 0x10,
        (k, 0x146, word1, w(0x1ff), 0x20a, 0x20f),
    );
    let word2 = crate::r11_misc::byte(img, f + 0x214);
    let edx = w(0x216);
    stores(g, img, f, &[0x225]);
    fade(g, img, fr, e - 0x14, (0x146, k, word2, edx, 0x221, 0x230));
    let s271 = g.read::<u32>(w(0x235));
    stores(g, img, f, &[0x239]);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[word1, word2, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, g.read::<u32>(s271 + 0x43c), 0);
    task_node(
        g,
        img,
        f,
        0x256,
        Some(0x265),
        (0x25c, 0x272, 0x27f, 0x28d, 0x295),
    );
    Ret::eax(0)
}

/// [`state_two_fades`] (`magic_0100` 0x100013e0).
pub const STATE_TWO_FADES_0100: Sig = Sig {
    len: 668,
    hash: 0x10a8_47ae_ebc0_cc98,
    calls: &[
        (0x35, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x3d, Callee::Sig(&crate::s4_misc::STATE_LISTED)),
        (0x5d, Callee::Sig(&crate::r9_misc::START_OBJECT_STATE_0100)),
        (0xe8, Callee::Sig(&crate::r9_misc::START_OBJECT_STATE_0100)),
        (0x1f8, Callee::Sig(&crate::s4_small::COLOUR_TASK)),
        (0x20f, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x230, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x256, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0084` 0x10001430 (730 bytes, a frame with one local): the head
/// (three words, the part set up; operands 4 bytes later than
/// [`object_two_fades`]'), then by the actor's state
/// ([`state_listed`](crate::s4_misc::state_listed)) one of two objects
/// started ([`start_object`](crate::m0084::start_object), scale `+0x53`):
/// unlisted, `OBJ1` (`+0x63`, index 0) at the first actor's point `P1`
/// (slots 262 and 268, slot 12's height less `K` `+0xcd` into `P1.y`,
/// `P1.w` 1.0, two words set); listed, `OBJ2` (`+0xfd`, index 1) at `P2`
/// (slots 262 and 268, three words set); slot 333 `(*A)` (`+0x173`) gives
/// the yaw `K3 - v` (`+0x186`) wrapped once (above `+0x195` less `+0x19f`,
/// below `+0x1a9` plus `+0x1b6`; a NaN stays) into the object's `+0xc4`;
/// its record (`+0x1e2` .. `+0x234`, into its `+0x110`) and four words; two
/// fades (`xmm1`/`xmm2` from `+0x1be`/`+0x1c6`, then `+0x262`/`+0x1be`),
/// slot 271 `(4)`, the task node. Returns 0.
#[allow(clippy::too_many_lines)]
pub fn state_yaw_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.esp;
    // push ebp; push ecx; push esi; push edi
    let s719 = slot_target(g, w(0x05), 0xb3c / 4);
    stores(g, img, f, &[0x1a, 0x24, 0x2e]);
    let h = g.call(s719, 0, &[w(0x16), 8, 0]).eax;
    g.write::<u32>(w(0x3e), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x43));
    let listed = Frame {
        at: img.rel32_target(f + 0x4b),
        esp: e - 0x14,
        ..*fr
    };
    let state = crate::s4_misc::state_listed(g, img, &listed).eax;
    let scale = k(g, 0x53);
    let obj = if state == 0 {
        let obj = w(0x63);
        crate::m0084::start_object(g, img, img.rel32_target(f + 0x6b), obj, scale, 0);
        let mut s = actor_point(g, img, f, (0x70, 0x84, 0x89, 0x75, 0x7d, 0x8e, 0x9c, 0xa1));
        s.push(first_actor(g, img, f, 0x70, 0x84));
        // call width: 12 (Pushes caps the words at 12)
        let top = s.call(g, slot_target(g, w(0xb1), 0x30 / 4), 0).st0;
        g.write::<f32>(w(0xbd), crate::m0018::st0_f32(top));
        let y = sub(k(g, 0xc5), k(g, 0xcd));
        stores(g, img, f, &[0xd4, 0xde, 0xe8]);
        g.write::<f32>(w(0xf6), y);
        obj
    } else {
        let obj = w(0xfd);
        crate::m0084::start_object(g, img, img.rel32_target(f + 0x106), obj, scale, 1);
        actor_point(
            g,
            img,
            f,
            (0x10b, 0x11f, 0x124, 0x110, 0x118, 0x129, 0x137, 0x13c),
        );
        stores(g, img, f, &[0x14e, 0x158, 0x162]);
        obj
    };
    let mut s = crate::pushes::Pushes::default();
    let t333 = g.read::<u32>(w(0x16d));
    s.push(g.read::<u32>(w(0x173)));
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(s.call(g, g.read::<u32>(t333 + 0x534), 0).st0);
    let x = sub(k(g, 0x186), v);
    let yaw = if x > k(g, 0x195) {
        sub(x, k(g, 0x19f))
    } else if k(g, 0x1a9) > x {
        add(x, k(g, 0x1b6))
    } else {
        x
    };
    g.write::<f32>(obj + 0xc4, yaw);
    stores(
        g,
        img,
        f,
        &[0x1e0, 0x1ea, 0x1f4, 0x1fe, 0x208, 0x212, 0x21c, 0x226],
    );
    g.write::<u16>(w(0x232), 0);
    g.write::<u32>(obj + 0x110, w(0x23c));
    stores(g, img, f, &[0x240]);
    zero_eax(g, img, f, &[0x24a, 0x24f, 0x254]);
    fade(g, img, fr, e - 0x18, (0x1c6, 0x1be, 0, 8, 0x1dc, 0x25a));
    let word2 = crate::r11_misc::byte(img, f + 0x267);
    fade(
        g,
        img,
        fr,
        e - 0x1c,
        (0x1be, 0x262, word2, w(0x269), 0x271, 0x276),
    );
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[0, word2, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x27b), 0x43c / 4), 0);
    task_node(g, img, f, 0x292, None, (0x298, 0x2aa, 0x2b7, 0x2c8, 0x2d0));
    Ret::eax(0)
}

/// [`state_yaw_two_fades`] (`magic_0084` 0x10001430).
pub const STATE_YAW_TWO_FADES_0084: Sig = Sig {
    len: 730,
    hash: 0x9271_1ef4_1b0c_d3d4,
    calls: &[
        (0x43, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4b, Callee::Sig(&crate::s4_misc::STATE_LISTED)),
        (0x6b, Callee::Sig(&crate::m0084::START_OBJECT)),
        (0x106, Callee::Sig(&crate::m0084::START_OBJECT)),
        (0x25a, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x276, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x292, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// Whether `v` is in the word list at `list` (ended by -1).
fn listed<G: Game>(g: &G, list: u32, v: u32) -> bool {
    let mut at = list;
    loop {
        let x = g.read::<u32>(at);
        if x == u32::MAX {
            return false;
        }
        if x == v {
            return true;
        }
        at = at.wrapping_add(4);
    }
}

/// `magic_0102` 0x10001460 (740 bytes, a /GS frame): the head (two
/// words, the part set up); the actors' mean position into the frame
/// (`V` at `ebp - 0x14`; [`mean_position`](crate::sb_misc::mean_position));
/// slot 390 `(SEL[0])`: a kind in the list `+0x70` puts `V` at the first
/// actor's point (slots 262 and 268 into `V`) and `Y` = `V.y`; else `Y` =
/// slot 12 `(chr)`'s height less `K` (`+0xb8`); slot 390 `(*A)` (`+0xc6`):
/// a kind in the list `+0xe1` picks the callback `+0x2b8`, else `+0x104`;
/// two objects start ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2):
/// `OBJ1` `+0x11b` with `(0, -, 0x1e, CB1)` (`+0x116`, `+0x111`), `OBJ2`
/// `+0x156` with `(1, -, 0, 0)`; scale `+0x10c`), each followed by `(V.x,
/// Y, V.z)` copied into a point; the record between them (its callback
/// the one picked); two fades, slot 271 `(4)`, the task node. Returns 0.
#[allow(clippy::too_many_lines)]
pub fn kinds_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let ebp = e - 4;
    let v = ebp - 0x14;
    let local_y = ebp - 0x18;
    let s719 = slot_target(g, w(0x11), 0xb3c / 4);
    stores(g, img, f, &[0x25, 0x2f]);
    let h = g.call(s719, 0, &[w(0x21), 8, 0]).eax;
    g.write::<u32>(w(0x3f), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x44));
    crate::sb_misc::mean_position(g, img, img.rel32_target(f + 0x4c), e - 0x2c, v);
    let byte = |g: &G, sel: u32| u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
    let mut s = crate::pushes::Pushes::default();
    s.push(byte(g, 0x51));
    // call width: 12 (Pushes caps the words at 12)
    let kind = s.call(g, slot_target(g, w(0x5a), 0x618 / 4), 0).eax;
    let y = if listed(g, w(0x70), kind) {
        let chr = g.read::<u32>(w(0x271).wrapping_add(4 * byte(g, 0x267)));
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[v + 8, v + 4, v, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x282), 0x418 / 4), 0);
        s.push_all(&[v, 0x11, chr]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x294), 0x430 / 4), 0);
        let y = g.read::<f32>(v + 4);
        g.write::<u32>(ebp - 8, 0x3f80_0000);
        y
    } else {
        let chr = g.read::<u32>(w(0x9e).wrapping_add(4 * byte(g, 0x94)));
        let mut s = crate::pushes::Pushes::default();
        s.push(chr);
        // call width: 12 (Pushes caps the words at 12)
        let top = st0_f32(s.call(g, slot_target(g, w(0xa3), 0x30 / 4), 0).st0);
        crate::sb_sse::sub(top, g.read::<f32>(w(0xb8)))
    };
    let t390 = g.read::<u32>(w(0xc0));
    let mut s = crate::pushes::Pushes::default();
    s.push(g.read::<u32>(w(0xc6)));
    g.write::<f32>(local_y, y);
    // call width: 12 (Pushes caps the words at 12)
    let kind2 = s.call(g, g.read::<u32>(t390 + 0x618), 0).eax;
    let callback = if listed(g, w(0xe1), kind2) {
        w(0x2b8)
    } else {
        w(0x104)
    };
    // the starts' four words below the frame: (index, ecx, a, b)
    let below = e - 0x30;
    let start = |g: &mut G, (obj, call): (u32, u32), args: [u32; 4]| {
        for (i, x) in (0u32..).zip(args) {
            g.write::<u32>(below + 4 * i, x);
        }
        let fr2 = Frame {
            at: img.rel32_target(f + call),
            esp: below - 4,
            edx: w(obj),
            xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x10c)))],
            ..*fr
        };
        crate::s6_misc::start_object_anim_2(g, img, &fr2);
    };
    let a = crate::r11_misc::byte(img, f + 0x116);
    start(g, (0x11b, 0x120), [0, 0, a, w(0x111)]);
    let copy = |g: &mut G, ops: [u32; 3]| {
        for (o, at) in ops.into_iter().zip([v, local_y, v + 8]) {
            let x = g.read::<u32>(at);
            g.write::<u32>(w(o), x);
        }
    };
    copy(g, [0x13b, 0x149, 0x15e]);
    stores(g, img, f, &[0x162, 0x16c, 0x176, 0x180, 0x18a]);
    g.write::<u32>(w(0x196), callback);
    stores(g, img, f, &[0x19a, 0x1a4]);
    g.write::<u16>(w(0x1b0), 0);
    stores(g, img, f, &[0x1b4]);
    start(g, (0x156, 0x1bf), [1, 0, 0, 0]);
    copy(g, [0x1dc, 0x1e9, 0x202]);
    let word1 = crate::r11_misc::byte(img, f + 0x1f3);
    fade(
        g,
        img,
        fr,
        e - 0x38,
        (0x1cc, 0x1d4, word1, w(0x1f5), 0x1fa, 0x207),
    );
    let word2 = crate::r11_misc::byte(img, f + 0x21c);
    fade(
        g,
        img,
        fr,
        e - 0x3c,
        (0x20f, 0x217, word2, w(0x21e), 0x223, 0x228),
    );
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[1, word1, word2, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x22d), 0x43c / 4), 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x244), w(0x23f));
    let first = g.read::<u32>(w(0x24a));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x2c8), node);
    } else {
        let last = g.read::<u32>(w(0x25b));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x2d3), node);
    // __security_check_cookie leaves `eax`
    Ret::eax(0)
}

/// [`kinds_two_fades`] (`magic_0102` 0x10001460).
pub const KINDS_TWO_FADES_0102: Sig = Sig {
    len: 740,
    hash: 0x21e2_3609_4b4a_8ea8,
    calls: &[
        (0x44, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4c, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x120, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x1bf, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x207, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x228, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x244, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x2dc, Callee::Any),
    ],
    imports: &[],
};
