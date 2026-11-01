//! Round 11 (magic-r11-own): small own functions of the DLLs below
//! `magic_0450` (each named for the DLL carrying it).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The code byte at `at` (an 8-bit immediate).
#[must_use]
pub(crate) fn byte(img: &Image, at: u32) -> u32 {
    u32::from(img.word(at).to_le_bytes()[0])
}

/// The code byte at `at` sign-extended (`add r32, imm8`, `push imm8`).
#[must_use]
pub(crate) fn imm8(img: &Image, at: u32) -> u32 {
    i32::from(img.word(at).to_le_bytes()[0].cast_signed()).cast_unsigned()
}

/// Starts the main object (no arguments, no frame; `magic_0383`
/// 0x10001370, 39 bytes): `OBJ` (operand `+0x0c`, scale `*K` `+0x04`)
/// starts ([`start_object_first`](crate::o6_fam_object::start_object_first),
/// `rel32` `+0x11`, below `sub esp, 0x10`), then the 16 bytes at `+0x18`
/// are copied to `+0x22` (`movaps`). Returns the object start's `eax`.
pub fn main_object_0383<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let at = img.rel32_target(f + 0x11);
    let call = Frame {
        at,
        esp: fr.esp.wrapping_sub(0x14),
        edx: w(0x0c),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x04)))],
        ..*fr
    };
    let obj = crate::o6_fam_object::start_object_first(g, img, at, &call);
    let (from, to) = (w(0x18), w(0x22));
    let v: [u32; 4] = core::array::from_fn(|i| g.read::<u32>(from + 4 * i as u32));
    for (i, x) in (0u32..).zip(v) {
        g.write::<u32>(to + 4 * i, x);
    }
    Ret::eax(obj)
}

/// [`main_object_0383`] (`magic_0383` 0x10001370).
pub const MAIN_OBJECT_0383: Sig = Sig {
    len: 39,
    hash: 0x0896_2ff1_b119_e300,
    calls: &[(
        0x11,
        Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
    )],
    imports: &[],
};

/// Starts the main object at its actor (no arguments; `magic_0369`
/// 0x10001370, 213 bytes): `OBJ` (`+0x11`, scale `*K` `+0x08`) starts
/// ([`start_object_first`](crate::o6_fam_object::start_object_first),
/// `rel32` `+0x16`); slot 262 `(*A, &P.x, &P.y, &P.z)` (`A` `+0x21`, the
/// immediates `+0x36`, `+0x31`, `+0x2c`); slot 12 `(*A)`'s float less `K2`
/// (`+0x5a`) into `P.y` (`+0x4a`, `+0x77`), `P.w` (`+0x6b`) 1.0; slot 333
/// `(*A)`'s float `v` gives the yaw `K3 - v` (`+0x8a`), wrapped once
/// (above `HI` `+0x9a` less `T` `+0xa4`, below `LO` `+0xb8` plus `T`
/// `+0xc5`; `comiss` + `jbe`: a NaN stays) into `G_YAW` (`+0xac`/`+0xcd`).
/// Returns slot 333's `eax`. Each slot call passes the words the original
/// leaves on the stack.
pub fn main_object_0369<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; push ecx; push esi; sub esp, 0x10; call
    let at = img.rel32_target(f + 0x16);
    let call = Frame {
        at,
        esp: fr.esp.wrapping_sub(0x20),
        edx: w(0x11),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x08)))],
        ..*fr
    };
    crate::o6_fam_object::start_object_first(g, img, at, &call);
    let actor = g.read::<u32>(w(0x21));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x2c), w(0x31), w(0x36), actor]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1b), 0x418 / 4), 0);
    s.push(actor);
    // call width: 12 (Pushes caps the words at 12)
    let top = st0_f32(s.call(g, slot_target(g, w(0x3e), 0x30 / 4), 0).st0);
    g.write::<f32>(w(0x4a), top);
    let y = sub(g.read::<f32>(w(0x52)), k(g, 0x5a));
    let t333 = g.read::<u32>(w(0x5f));
    s.push(g.read::<u32>(w(0x65)));
    g.write::<u32>(w(0x6b), 0x3f80_0000);
    g.write::<f32>(w(0x77), y);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, g.read::<u32>(t333.wrapping_add(0x534)), 0);
    let x = sub(k(g, 0x8a), st0_f32(out.st0));
    if x > k(g, 0x9a) {
        g.write::<f32>(w(0xac), sub(x, k(g, 0xa4)));
    } else if k(g, 0xb8) > x {
        g.write::<f32>(w(0xcd), add(x, k(g, 0xc5)));
    } else {
        g.write::<f32>(w(0xcd), x);
    }
    Ret::eax(out.eax)
}

/// [`main_object_0369`] (`magic_0369` 0x10001370).
pub const MAIN_OBJECT_0369: Sig = Sig {
    len: 213,
    hash: 0x05e0_0f52_5dcc_0835,
    calls: &[(
        0x16,
        Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
    )],
    imports: &[],
};

/// A pool node ([`take`](crate::sa_pool::take), `rel32` `call`, owner the
/// `mov ecx, imm32` before it) with its byte `+0x18` = `flag`, `+0x4` 0.
fn flagged_node<G: Game>(g: &mut G, img: &Image, f: u32, call: u32, flag: u8) -> u32 {
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), img.word(f + call - 5));
    g.write::<u8>(node.wrapping_add(0x18), flag);
    g.write::<u32>(node.wrapping_add(4), 0);
    node
}

/// The colour start-up (no arguments, no frame; `magic_0065` 0x10002fd0,
/// 411 bytes): `E = TABLE[*G_I]` (`+0x01`, `+0x13`) and the flag `b` =
/// bit 10 of `*G_F` (`+0x08`) go into words (`+0x26`, `+0x36`), with two
/// more set; when `b`, eight more words (`E` into `+0x66`) and the colour
/// task ([`colour_task`](crate::s4_small::colour_task)), the state
/// record `S` = `+0x8e`, else `+0x95`; `S + 4` = `E` and the state task
/// ([`state_task`](crate::s4_misc::state_task), `ecx` `+0x9c`, the word
/// `S`); a word set (`+0xaf`) and the actor block
/// ([`me_block`](crate::sa_misc::me_block), the immediate `+0xa9`); then
/// task nodes: the first (byte 1) last on the list, the records block
/// (`+0xf6`, [`records_init_80`](crate::sa_misc::records_init_80)), when
/// `b` a second one (no byte) after it without becoming the last, and a
/// third (byte 2) after the first or second. Returns 0.
pub fn colour_start_0065<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let i = g.read::<u32>(w(0x01));
    let bit = (g.read::<u32>(w(0x08)).cast_signed() >> 10) & 1 != 0;
    let entry = g.read::<u32>(w(0x13).wrapping_add(i.wrapping_mul(4)));
    stores(g, img, f, &[0x1a]);
    g.write::<u32>(w(0x26), u32::from(bit));
    stores(g, img, f, &[0x2a]);
    g.write::<u32>(w(0x36), entry);
    let state = if bit {
        stores(g, img, f, &[0x3c, 0x46, 0x50, 0x5a]);
        g.write::<u32>(w(0x66), entry);
        stores(g, img, f, &[0x6a, 0x74, 0x7e]);
        let colour = Frame {
            at: img.rel32_target(f + 0x89),
            esp: e - 0xc,
            ..*fr
        };
        crate::s4_small::colour_task(g, img, &colour);
        w(0x8e)
    } else {
        w(0x95)
    };
    // push ecx; push eax: the state task's word
    g.write::<u32>(e - 0x10, state);
    g.write::<u32>(state.wrapping_add(4), entry);
    let task = Frame {
        at: img.rel32_target(f + 0xa4),
        esp: e - 0x14,
        ecx: w(0x9c),
        ..*fr
    };
    crate::s4_misc::state_task(g, img, &task);
    stores(g, img, f, &[0xad]);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0xb8), w(0xa9));
    {
        let n = flagged_node(g, img, f, 0xc5, 1);
        let head = g.read::<u32>(w(0xcb));
        if head == 0 {
            g.write::<u32>(n, 0);
            g.write::<u32>(w(0xf1), n);
        } else {
            let tail = g.read::<u32>(w(0xdf));
            g.write::<u32>(n, tail);
            g.write::<u32>(tail + 4, n);
        }
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(w(0xfe), n);
    }
    crate::sa_misc::records_init_80(g, img, img.rel32_target(f + 0x103), w(0xf6));
    // the list's first and the node the third goes after
    let (head, prev) = if bit {
        let n = crate::sa_pool::take(g, img, img.rel32_target(f + 0x112), w(0x10d));
        let head = g.read::<u32>(w(0x118));
        g.write::<u32>(n.wrapping_add(4), 0);
        let head = if head == 0 {
            g.write::<u32>(n, 0);
            g.write::<u32>(w(0x143), n);
            n
        } else {
            let tail = g.read::<u32>(w(0x12b));
            g.write::<u32>(n, tail);
            g.write::<u32>(tail + 4, n);
            head
        };
        g.write::<u32>(n.wrapping_add(8), n);
        (head, n)
    } else {
        (g.read::<u32>(w(0x14e)), g.read::<u32>(w(0x154)))
    };
    let n = flagged_node(g, img, f, 0x15e, 2);
    if head == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x18b), n);
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(w(0x193), n);
    } else {
        g.write::<u32>(n, prev);
        g.write::<u32>(prev + 4, n);
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(w(0x17b), n);
    }
    Ret::eax(0)
}

/// [`colour_start_0065`] (`magic_0065` 0x10002fd0).
pub const COLOUR_START_0065: Sig = Sig {
    len: 411,
    hash: 0x73f5_6325_38da_8a57,
    calls: &[
        (0x89, Callee::Sig(&crate::s4_small::COLOUR_TASK)),
        (0xa4, Callee::Sig(&crate::s4_misc::STATE_TASK)),
        (0xb8, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0xc5, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x103, Callee::Sig(&crate::sa_misc::RECORDS_INIT_80)),
        (0x112, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x15e, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// Two animated objects at the effect and the first actor (no arguments,
/// no frame; `magic_0054` 0x10001370, 413 bytes): `*G_A`/`*G_B` set, slot
/// 719 `(NAME, 8, 0)` into `G_PART` (`+0x2e`), the particle data set up
/// ([`init_ppp`](crate::m0054::init_ppp)); the object `OBJ1` (`+0x44`,
/// scale `*K` `+0x3b`) starts
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), `(0, 0)`),
/// slot 262 `(*G_H, P, P + 4, P + 8)` (`+0x69`; the immediates `+0x63`,
/// `+0x5e`, `+0x53`); `OBJ2` (`+0x91`, scale `+0x83`, `(1, 0x14)`) starts
/// after `P.w` 1.0, `P.y` less `K2` (`+0x7b`) and the record words
/// (`+0xa9` .. `+0xff`, a halfword `+0xf9`); slot 262 `(chr, Q, Q + 4, Q +
/// 8)` for the first actor's character (`+0x10d`, `+0x126`), `Q.w` 1.0,
/// `Q.y` less `K2` (`+0x143`); a free-list node
/// ([`pop_node`](crate::m0032::pop_node)) appended to the task list (first
/// `+0x163`, last `+0x173`). Returns 0.
pub fn two_anim_objects_0054<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s719 = slot_target(g, w(0x01), 0xb3c / 4);
    stores(g, img, f, &[0x14, 0x1e]);
    let part = g.call(s719, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x2e), part);
    crate::m0054::init_ppp(g, img, img.rel32_target(f + 0x33));
    let at = img.rel32_target(f + 0x49);
    crate::s4_misc::start_object_anim(g, img, at, w(0x44), k(g, 0x3b), 0, 0);
    let mut s = crate::pushes::Pushes::default();
    let h = g.read::<u32>(w(0x69));
    s.push_all(&[w(0x53), w(0x5e), w(0x63), h]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x4e), 0x418 / 4), 0);
    let y = sub(k(g, 0x73), k(g, 0x7b));
    let scale = k(g, 0x83);
    stores(g, img, f, &[0x95]);
    g.write::<f32>(w(0xa3), y);
    stores(g, img, f, &[0xa7, 0xb1, 0xbb, 0xc5, 0xcf, 0xd9, 0xe3, 0xed]);
    g.write::<u16>(w(0xf9), 0);
    stores(g, img, f, &[0xfd]);
    let index = crate::r11_misc::byte(img, f + 0x8f);
    let value = crate::r11_misc::byte(img, f + 0x8d);
    let at = img.rel32_target(f + 0x108);
    crate::s4_misc::start_object_anim(g, img, at, w(0x91), scale, index, value);
    let sel = g.read::<u32>(w(0x10d));
    let b = u32::from(g.read::<u8>(sel));
    let chr = g.read::<u32>(w(0x126).wrapping_add(4 * b));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[value, index, w(0x112), w(0x11a), w(0x11f), chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x12b), 0x418 / 4), 0);
    let y = sub(k(g, 0x13b), k(g, 0x143));
    stores(g, img, f, &[0x14a]);
    g.write::<f32>(w(0x158), y);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x15d));
    let first = g.read::<u32>(w(0x163));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x18e), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x196), node);
    } else {
        let last = g.read::<u32>(w(0x173));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x180), node);
    }
    Ret::eax(0)
}

/// [`two_anim_objects_0054`] (`magic_0054` 0x10001370).
pub const TWO_ANIM_OBJECTS_0054: Sig = Sig {
    len: 413,
    hash: 0x5868_905a_bbf2_8bac,
    calls: &[
        (0x33, Callee::Sig(&crate::m0054::INIT_PPP)),
        (0x49, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x108, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x15d, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// Sets an object moving from the effect's actor to a target (cdecl
/// `(obj)`, a /GS frame; `magic_0191` 0x10001440, 477 bytes): `G = *G_P`
/// (`+0x12`); the target actor `T = G[0x1a0 + 4 obj[0x2c + 16
/// obj[0xac]]]`; slot 456 `(G[0x198], 15, &A)` and slot 457 `(T, 6, &B)`
/// fill two points on the frame (`ebp - 0x24`, `- 0x34`); `D = B - A`
/// (four lanes, at `ebp - 0x14`); slot 453 `(&D)` gives its length `L`;
/// `n = trunc(L / K1)` (`+0xb4`) and `s = K2 / n` (`+0xbc`); the object
/// gets `+0xd0..` = `A.xyz` (w 0), `+0xe0..` = `D.xyz * s` (w 0),
/// `+0x130..0x138` 0.17, `+0x13c` 0 and `n` at `16 (obj[0xac] + 3)`;
/// then slot 444 `(0)`, slot 440 `(*G_P, obj, 0, 1)`, slot 450 `(obj,
/// CB)` (`+0x1bd`). Returns slot 450's `eax`.
pub fn aim_at_target_0191<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (a, b, d) = (ebp - 0x24, ebp - 0x34, ebp - 0x14);
    let gp = g.read::<u32>(w(0x12));
    let obj = fr.arg(g, 0);
    let idx = g.read::<u32>(obj + 0xac);
    let e = g.read::<u32>(obj.wrapping_add(idx.wrapping_mul(16)).wrapping_add(0x2c));
    let target = g.read::<u32>(gp.wrapping_add(e.wrapping_mul(4)).wrapping_add(0x1a0));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[a, 0xf, g.read::<u32>(gp + 0x198)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x33), 0x720 / 4), 0);
    s.push_all(&[b, 6, target]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x4c), 0x724 / 4), 0);
    for i in 0..4 {
        let v = sub(g.read::<f32>(b + 4 * i), g.read::<f32>(a + 4 * i));
        g.write::<f32>(d + 4 * i, v);
    }
    s.push(d);
    // call width: 12 (Pushes caps the words at 12)
    let len = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x6a), 0x714 / 4), 0).st0);
    let n = crate::s4_summon::trunc(div(len, g.read::<f32>(w(0xb4))));
    let scale = div(g.read::<f32>(w(0xbc)), cvt(n));
    let step: [f32; 4] = core::array::from_fn(|i| mul(g.read::<f32>(d + 4 * i as u32), scale));
    g.write::<f32>(d + 0xc, step[3]);
    for i in 0..3 {
        let v = g.read::<u32>(a + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, v);
    }
    g.write::<u32>(obj + 0xdc, 0);
    for (i, v) in (0u32..).zip(&step[..3]) {
        g.write::<f32>(obj + 0xe0 + 4 * i, *v);
    }
    g.write::<u32>(obj + 0xec, 0);
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(obj + o, 0x3e2e_147b);
    }
    g.write::<u32>(obj + 0x13c, 0);
    let slot = g.read::<u32>(obj + 0xac).wrapping_add(3);
    for (i, v) in (0u32..).zip(&step[..3]) {
        g.write::<f32>(d + 4 * i, *v);
    }
    g.write::<u32>(obj.wrapping_add(slot.wrapping_mul(16)), n);
    s.push(0);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x191), 0x6f0 / 4), 0);
    let t440 = g.read::<u32>(w(0x1a0));
    s.push_all(&[1, 0, obj, g.read::<u32>(w(0x1b1))]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, g.read::<u32>(t440 + 0x6e0), 0);
    s.push_all(&[w(0x1bd), obj]);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot_target(g, w(0x1b8), 0x708 / 4), 0);
    // __security_check_cookie leaves `eax`
    Ret::eax(out.eax)
}

/// [`aim_at_target_0191`] (`magic_0191` 0x10001440).
pub const AIM_AT_TARGET_0191: Sig = Sig {
    len: 477,
    hash: 0x4c63_b4a4_9f8d_e50f,
    calls: &[
        // __security_check_cookie
        (0x1d5, Callee::Any),
    ],
    imports: &[],
};

/// `mov dword [obj + disp32], imm32` (`c7 86`) at each instruction offset
/// of `ats`.
fn obj_stores<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, ats: &[u32]) {
    for &at in ats {
        let d = img.word(f + at + 2);
        g.write::<u32>(obj.wrapping_add(d), img.word(f + at + 6));
    }
}

/// Sets an object moving between the effect's two actors (cdecl `(obj)`,
/// a /GS frame; `magic_0194` 0x10001370, 687 bytes): `obj + 0xbc` |=
/// 0x2000; slot 456 `(G[0x198], 0x8c, &A)` and slot 457 `(G[0x1a0], 6,
/// &B)` (`G = *G_P`, `+0x23`; the points on the frame at `ebp - 0x14`,
/// `- 0x24`); `D = B - A` (four lanes, `ebp - 0x34`) through slot 464
/// `(&D)`; the ends pulled in, `A' = A - D K0` and `B' = B - D K1`
/// (`+0xae`, `+0x12a`); the object gets `+0xd0..` = `A'.xyz` (w 0),
/// `+0xe0..` = `(B' - A').xyz K2` (`+0x17e`; w 0) and the words of the
/// `mov [esi + d], imm` stores (`+0x100` .. `+0x18c`); slot 450 `(obj,
/// CB)` (`+0x286`), slot 451 `(obj, 10)`. Returns slot 451's `eax`.
pub fn between_actors_0194<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let (a, b, d) = (ebp - 0x14, ebp - 0x24, ebp - 0x34);
    let lane = |g: &G, at: u32, i: u32| g.read::<f32>(at + 4 * i);
    let obj = fr.arg(g, 0);
    let flags = g.read::<u32>(obj + 0xbc);
    g.write::<u32>(obj + 0xbc, flags | 0x2000);
    let mut s = crate::pushes::Pushes::default();
    let gp = g.read::<u32>(w(0x23));
    s.push_all(&[a, 0x8c, g.read::<u32>(gp + 0x198)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x33), 0x720 / 4), 0);
    let gp = g.read::<u32>(w(0x44));
    s.push_all(&[b, 6, g.read::<u32>(gp + 0x1a0)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x51), 0x724 / 4), 0);
    for i in 0..4 {
        let v = sub(lane(g, b, i), lane(g, a, i));
        g.write::<f32>(d + 4 * i, v);
    }
    s.push(d);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x6c), 0x740 / 4), 0);
    let dir: [f32; 4] = core::array::from_fn(|i| lane(g, d, i as u32));
    let (k0, k1) = (k(g, 0xae), k(g, 0x12a));
    let near: [f32; 4] = core::array::from_fn(|i| sub(lane(g, a, i as u32), mul(dir[i], k0)));
    for (i, v) in (0u32..).zip(near) {
        g.write::<f32>(a + 4 * i, v);
    }
    let far: [f32; 4] = core::array::from_fn(|i| sub(lane(g, b, i as u32), mul(dir[i], k1)));
    for (i, v) in (0u32..).zip(far) {
        g.write::<f32>(b + 4 * i, v);
    }
    let k2 = k(g, 0x17e);
    let t450 = g.read::<u32>(w(0x183));
    for i in 0..3 {
        g.write::<f32>(obj + 0xd0 + 4 * i, near[i as usize]);
    }
    g.write::<u32>(obj + 0xdc, 0);
    for i in 0..3 {
        let v = mul(sub(far[i as usize], near[i as usize]), k2);
        g.write::<f32>(obj + 0xe0 + 4 * i, v);
    }
    g.write::<u32>(obj + 0xec, 0);
    let ats: [u32; 16] = core::array::from_fn(|i| 0x1df + 10 * i as u32);
    obj_stores(g, img, f, obj, &ats);
    s.push_all(&[w(0x286), obj]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, g.read::<u32>(t450 + 0x708), 0);
    s.push_all(&[byte(img, f + 0x293), obj]);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot_target(g, w(0x28e), 0x70c / 4), 0);
    // __security_check_cookie leaves `eax`
    Ret::eax(out.eax)
}

/// [`between_actors_0194`] (`magic_0194` 0x10001370).
pub const BETWEEN_ACTORS_0194: Sig = Sig {
    len: 687,
    hash: 0xf01c_595c_9399_ffca,
    calls: &[
        // __security_check_cookie
        (0x2a7, Callee::Any),
    ],
    imports: &[],
};

/// Starts an object's part at the effect's position (cdecl `(obj)`, a /GS
/// frame; `magic_0429` 0x10001030, 310 bytes): `**G_P` = `ITEM` (`+0x11`,
/// `+0x1b`); slot 941 `(*(*G_D + 0x10))` (`+0x20`) gives the program
/// list whose `+0xc` entry starts the part
/// ([`start_part`](crate::opu_parts::start_part), `edx` 0x1000, `(prog,
/// 1)`); slot 268 `(*C, 0x10, obj + 0x40)` (`C` `+0x5b`), slot 210 `(*C,
/// &M)` (a matrix on the frame at `ebp - 0x54`), slot 325 `(&P, &Q)`
/// (`ebp - 0x14`, `- 0x24`); when `|trunc(obj.y - P.y)|` (`obj + 0x44`)
/// is below the double `*K` (`+0x9f`), `obj.y` = `P.y - K` (in double,
/// rounded once); the words `+0x50..+0x6c` set and slot 393 `(*Q2, obj +
/// 0x40, 0x100)` (`+0xda`). Returns 1.
pub fn part_at_effect_0429<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, p, q) = (ebp - 0x54, ebp - 0x14, ebp - 0x24);
    let obj = fr.arg(g, 0);
    let item_at = g.read::<u32>(w(0x11));
    g.write::<u32>(item_at, w(0x1b));
    let mut s = crate::pushes::Pushes::default();
    let d = g.read::<u32>(w(0x20));
    s.push(g.read::<u32>(d.wrapping_add(0x10)));
    // call width: 12 (Pushes caps the words at 12)
    let list = s.call(g, slot_target(g, w(0x29), 0xeb4 / 4), 0).eax;
    let program = g.read::<u32>(list.wrapping_add(0xc));
    let start = img.rel32_target(f + 0x44);
    crate::opu_parts::start_part(g, img, start, obj, w(0x3d), program, 1);
    s.push_all(&[0, 1, program]);
    let c = g.read::<u32>(w(0x5b));
    s.push_all(&[obj + 0x40, 0x10, c]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x49), 0x430 / 4), 0);
    s.push_all(&[m, g.read::<u32>(w(0x6c))]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x66), 0x348 / 4), 0);
    s.push_all(&[q, p]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x81), 0x514 / 4), 0);
    let py = g.read::<f32>(p + 4);
    let dy = crate::sb_sse::sub(g.read::<f32>(obj + 0x44), py);
    let n = crate::s4_summon::trunc(dy).cast_signed().wrapping_abs();
    let k = f64::from_bits(g.read::<u64>(w(0x9f)));
    // comisd + jbe: only below K
    if k > f64::from(n) {
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: one rounding
        let y = (f64::from(py) - k) as f32;
        g.write::<f32>(obj + 0x44, y);
    }
    let t393 = g.read::<u32>(w(0xce));
    let q2 = g.read::<u32>(w(0xda));
    for at in [0xde, 0xe5, 0xec, 0xf3, 0xfa, 0x101, 0x108, 0x10f] {
        let disp = u32::from(img.word(f + at + 2).to_le_bytes()[0]);
        g.write::<u32>(obj + disp, img.word(f + at + 3));
    }
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[0x100, obj + 0x40, q2]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, g.read::<u32>(t393 + 0x624), 0);
    // __security_check_cookie leaves `eax`
    Ret::eax(1)
}

/// [`part_at_effect_0429`] (`magic_0429` 0x10001030).
pub const PART_AT_EFFECT_0429: Sig = Sig {
    len: 310,
    hash: 0xb83b_eb19_67dc_2435,
    calls: &[
        (0x44, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x12e, Callee::Any),
    ],
    imports: &[],
};
