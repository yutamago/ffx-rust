//! A summon's start (a table entry, cdecl `(actor)`; 416 bytes in
//! `magic_0077`..`0081`): the targets, the DLL's task list and two static
//! nodes, the part's loading and its two motions, then the DLL's own
//! start-up (the second `rel32` callee, which differs per DLL). The
//! copies differ in that callee and two immediates (slot 719's third
//! word, slot 710's size), read from the code; each binds by a [`Sig`]
//! of its own ([`copies!`]).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The start: [`targets_init_b`](crate::s4_summon::targets_init_b) for
/// `actor`; the list words (`+0x17`/`+0x1b` .. `+0x35`/`+0x39`, and
/// `+0x3f` = 1) and the two static nodes (callbacks `+0x49`/`+0x7a`, halfwords
/// `+0x54` 0x1ff and `+0x85` 0xff, self-links, a word 0) set; slot 718
/// `(PART)` (`G_X` `+0xb6` = the immediate `+0xba` first); slot 719
/// `(PART, 8, B)` (`B` the byte `+0xcf`) gives the handle `G_H`
/// (`+0xe7`); slot 711 `(h, PROG, 0)` with `**G_LOCK` 1 around it; slot
/// 710 `(ENV, *G_H, BUF, SIZE)` (`SIZE` the immediate `+0x114`); slots
/// 222 and 221 `(PART, 0, 1000, 2)`; slot 719 `(PART, 6, 1)` and slot 551
/// `(its eax, 0, 2)`; then `start` (the `rel32` at `+0x195`). Returns 0.
/// Each slot call passes the words the original leaves on the stack.
pub fn summon_start<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    start: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let actor = fr.arg(g, 0);
    // push ebp; and esp, -8; push ecx; push esi; call
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc);
    crate::s4_summon::targets_init_b(g, img, img.rel32_target(f + 0x0c), esp, actor);
    let t = w(0x11);
    for (dst, src) in [(0x17, 0x1b), (0x21, 0x25), (0x2b, 0x2f), (0x35, 0x39)] {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(w(0x3f), 1);
    g.write::<u32>(w(0x49), w(0x4d));
    g.write::<u16>(w(0x54), 0x1ff);
    g.write::<u32>(w(0x5c), w(0x60));
    g.write::<u32>(w(0x66), w(0x6a));
    g.write::<u32>(w(0x70), 0);
    g.write::<u32>(w(0x7a), w(0x7e));
    g.write::<u16>(w(0x85), 0xff);
    g.write::<u32>(w(0x8d), w(0x91));
    g.write::<u32>(w(0x97), w(0x9b));
    g.write::<u32>(w(0xa1), 0);
    let mut s = Pushes::default();
    s.push(w(0xb0));
    g.write::<u32>(w(0xb6), w(0xba));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, 0xb38 / 4), 0);
    s.pop(1);
    tail(g, img, fr, 0, esp, start)
}

/// The part's loading, its motions and the start-up ([`summon_start`]
/// from slot 719 on; the operands `d` bytes later). Returns 0.
fn tail<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    d: u32,
    esp: u32,
    start: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o + d);
    let mut s = Pushes::default();
    s.push_all(&[w(0xcf) & 0xff, 8, w(0xd3)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot_target(g, w(0xc1), 0xb3c / 4), 0).eax;
    s.pop(3);
    let lock = g.read::<u32>(w(0xdb));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xe7), h);
    s.push_all(&[0, w(0xec), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xf2), 0xb1c / 4), 0);
    s.pop(3);
    let lock = g.read::<u32>(w(0x105));
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0x125));
    s.push_all(&[w(0x114), w(0x11f), h, w(0x12a)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x10f), 0xb18 / 4), 0);
    s.push_all(&[2, 1000, 0, w(0x145)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x131), 0x378 / 4), 0);
    s.push_all(&[2, 1000, 0, w(0x160)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x14c), 0x374 / 4), 0);
    s.push_all(&[1, 6, w(0x176)]);
    // call width: 12 (Pushes caps the words at 12)
    let m = s.call(g, slot_target(g, w(0x167), 0xb3c / 4), 0).eax;
    s.pop(15);
    s.push_all(&[2, 0, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x185), 0x89c / 4), 0);
    let callee = Frame {
        at: img.rel32_target(f + 0x195 + d),
        esp,
        ..*fr
    };
    start(g, img, &callee);
    Ret::eax(0)
}

/// The start with the node graph (887 bytes, `magic_0083` 0x10006c50 and
/// its copies; cdecl `(actor)`): the targets
/// ([`targets_init_b`](crate::s4_summon::targets_init_b)); the DLL's list
/// words, its static nodes, and the node graph's records set up by
/// immediate stores (each `mov [G], imm` of the code, the halfword
/// count `+0x14` going up by three, two inlined list inserts deciding by
/// constant compares, and the flag byte `+0x1fd` copied into three
/// nodes); the six node pointers of a local group handed to
/// [`node_insert`](crate::nodes::node_insert); slot 718 `(PART)`; then
/// [`summon_start`]'s tail (its operands 0x1d7 bytes later). Returns 0.
#[allow(clippy::too_many_lines)]
pub fn summon_start_nodes<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    start: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let actor = fr.arg(g, 0);
    // push ebp; and esp, -8; sub esp, 0x1c; push esi; call
    let top = fr.esp.wrapping_sub(4) & !7;
    let esp = top.wrapping_sub(0x24);
    crate::s4_summon::targets_init_b(g, img, img.rel32_target(f + 0x0e), esp, actor);
    // `mov [G], imm` (dword, halfword or byte): the immediate four bytes
    // after the address operand
    let st32 = |g: &mut G, o: u32| g.write::<u32>(w(o), w(o + 4));
    let st16 = |g: &mut G, o: u32| g.write::<u16>(w(o), w(o + 4) as u16);
    let count = g.read::<u16>(w(0x14));
    let node_a = w(0x19);
    for o in [0x21, 0x2b, 0x35, 0x3f, 0x49, 0x53, 0x5d] {
        st32(g, o);
    }
    st16(g, 0x68);
    for o in [0x70, 0x7a] {
        st32(g, o);
    }
    st16(g, 0x85);
    for o in [0x8d, 0x97, 0xa1] {
        st32(g, o);
    }
    st16(g, 0xac);
    for o in [0xb4, 0xbe, 0xc8, 0xd2, 0xdc, 0xe6, 0xf0] {
        st32(g, o);
    }
    g.write::<u8>(w(0xfa), w(0xfe) as u8);
    st32(g, 0x101);
    st16(g, 0x10c);
    for o in [0x114, 0x11e, 0x128, 0x132, 0x13c] {
        st32(g, o);
    }
    // the inlined inserts compare constants
    st32(g, if node_a == w(0x146) { 0x14e } else { 0x15a });
    let node_b = w(0x163);
    for o in [0x16b, 0x175, 0x17f, 0x189, 0x193] {
        st32(g, o);
    }
    st16(g, 0x19e);
    for o in [0x1a6, 0x1b0, 0x1ba, 0x1c4, 0x1ce] {
        st32(g, o);
    }
    st32(g, if node_b == w(0x1d8) { 0x1e0 } else { 0x1ec });
    g.write::<u16>(w(0x1f8), count.wrapping_add(3));
    let flag = g.read::<u8>(w(0x1fd));
    for o in [0x207, 0x211, 0x21b, 0x225, 0x22f, 0x239] {
        st32(g, o);
    }
    for o in [0x242, 0x247, 0x24c] {
        g.write::<u8>(w(o), flag);
    }
    // the group: six words in the frame (`lea ecx, [esp + 8]`)
    let group = top.wrapping_sub(0x18);
    for (i, o) in (0u32..).zip([0x254, 0x25c, 0x264, 0x26c, 0x274, 0x27c]) {
        g.write::<u32>(group + 4 * i, w(o));
    }
    let insert = Frame {
        at: img.rel32_target(f + 0x281),
        esp,
        ecx: group,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    // call width: 1
    g.call(slot_target(g, w(0x286), 0xb38 / 4), 0, &[w(0x28b)]);
    tail(g, img, fr, 0x1d7, esp, start)
}

/// One entry and one [`Sig`] per copy: `name => SIG, hash, start-up
/// port, its Sig`.
macro_rules! copies {
    ($($(#[$doc:meta])* $f:ident => $sig:ident, $hash:literal, $start:path, $ssig:path;)*) => {
        $(
            $(#[$doc])*
            pub fn $f<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
                summon_start(g, img, fr, $start)
            }

            #[doc = concat!("[`", stringify!($f), "`].")]
            pub const $sig: Sig = Sig {
                len: 416,
                hash: $hash,
                calls: &[
                    (0x0c, Callee::Sig(&crate::s4_summon::TARGETS_INIT_B)),
                    (0x195, Callee::Sig(&$ssig)),
                ],
                imports: &[],
            };
        )*
    };
}

copies! {
    /// `magic_0077` 0x100069d0.
    start_0077 => START_0077, 0x4cfb_6821_a542_43f8, crate::m0076::model_start_0077, crate::m0076::MODEL_START_0077;
    /// `magic_0078` 0x100067b0.
    start_0078 => START_0078, 0xffe3_094d_87bb_4633, crate::m0078::start_0078, crate::m0078::START_0078;
    /// `magic_0079` 0x100067b0.
    start_0079 => START_0079, 0xffe3_094d_87bb_4633, crate::m0078::start_0079, crate::m0078::START_0079;
    /// `magic_0080` 0x10006880.
    start_0080 => START_0080, 0xfab0_d598_fe8d_ebe3, crate::m0078::start_0080, crate::m0078::START_0080;
    /// `magic_0081` 0x100067b0.
    start_0081 => START_0081, 0xffe3_094d_87bb_4633, crate::m0078::start_0081, crate::m0078::START_0081;
}

/// One entry and one [`Sig`] per copy of [`summon_start_nodes`].
macro_rules! node_copies {
    ($($(#[$doc:meta])* $f:ident => $sig:ident, $hash:literal, $start:path, $ssig:path;)*) => {
        $(
            $(#[$doc])*
            pub fn $f<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
                summon_start_nodes(g, img, fr, $start)
            }

            #[doc = concat!("[`", stringify!($f), "`].")]
            pub const $sig: Sig = Sig {
                len: 887,
                hash: $hash,
                calls: &[
                    (0x0e, Callee::Sig(&crate::s4_summon::TARGETS_INIT_B)),
                    (0x281, Callee::Sig(&crate::nodes::INSERT)),
                    (0x36c, Callee::Sig(&$ssig)),
                ],
                imports: &[],
            };
        )*
    };
}

node_copies! {
    /// `magic_0083` 0x10006c50.
    nodes_0083 => NODES_0083, 0xe66d_7c19_aba9_bf35, crate::m0078::start_0083, crate::m0078::START_0083;
    /// `magic_0091` 0x10006c70.
    nodes_0091 => NODES_0091, 0x5c43_380b_a8f7_9702, crate::m0089::start_0091, crate::m0089::START_0091;
    /// `magic_0094` 0x10007030.
    nodes_0094 => NODES_0094, 0xe66d_7c19_aba9_bf35, crate::m0089::start_0094, crate::m0089::START_0094;
    /// `magic_0095` 0x10006ec0.
    nodes_0095 => NODES_0095, 0x1238_b4b7_7a7a_1b45, crate::m0089::start_0095, crate::m0089::START_0095;
    /// `magic_0096` 0x10006d30.
    nodes_0096 => NODES_0096, 0x1238_b4b7_7a7a_1b45, crate::m0089::start_0096, crate::m0089::START_0096;
    /// `magic_0097` 0x10006c60.
    nodes_0097 => NODES_0097, 0x1238_b4b7_7a7a_1b45, crate::m0078::start_0083, crate::m0078::START_0097;
}
