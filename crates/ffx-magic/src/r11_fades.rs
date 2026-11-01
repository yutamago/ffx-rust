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
