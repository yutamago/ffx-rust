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

/// `mov dword [reg + d8], imm32` (`c7 4x d8`) at each instruction offset
/// of `ats`, `reg` = `base`.
fn disp8_stores<G: Game>(g: &mut G, img: &Image, f: u32, base: u32, ats: &[u32]) {
    for &at in ats {
        let d = byte(img, f + at + 2);
        g.write::<u32>(base.wrapping_add(d), img.word(f + at + 3));
    }
}

/// Parameter family P070 (`magic_0162` 0x10002f90 and `magic_0586`
/// 0x10002ba0, 309 bytes; the block offsets differ, read from the
/// code): with `B = *G_B` (`+0x0c`), the object `B + 0x390` starts
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale
/// `*K` `+0x04`, index 3); its point gets four words of `B` (the `mov`
/// pairs from `+0x22`), `y` less `K2` (`+0x64`), and one more word (`+0x5a`,
/// `+0x68`); the state `S = B + d` (`+0x82`) set (`+0x0`..`+0x14`, halfword
/// `+0x18` 0) and handed to a task node ([`take`](crate::sa_pool::take),
/// owner `+0x74`; byte 1, `+0x14` = `S`) appended to the task list (first
/// `+0xba`, last `+0xd1`); the fader `F = B + d` (`+0xe9`) set up
/// ([`fader_task`](crate::s4_small::fader_task)), its halfword `+0x2c` |=
/// 3 and seven words set. Returns the fader task's `eax`.
pub fn fader_start_p070<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x04));
    let b = g.read::<u32>(w(0x0c));
    let obj = b.wrapping_add(w(0x15));
    let at = img.rel32_target(f + 0x1e);
    crate::s4_misc::start_object_plain(g, img, at, obj, scale, byte(img, f + 0x1a));
    for k in 0..4 {
        let v = g.read::<u32>(b.wrapping_add(w(0x24 + 12 * k)));
        g.write::<u32>(obj.wrapping_add(w(0x2a + 12 * k)), v);
    }
    let y = sub(
        g.read::<f32>(obj.wrapping_add(w(0x56))),
        g.read::<f32>(w(0x64)),
    );
    let v = g.read::<u32>(b.wrapping_add(w(0x5c)));
    g.write::<u32>(obj.wrapping_add(w(0x6a)), v);
    g.write::<f32>(obj.wrapping_add(w(0x7c)), y);
    let state = b.wrapping_add(w(0x82));
    g.write::<u32>(state, w(0x88));
    disp8_stores(g, img, f, state, &[0x8c, 0x93, 0x9a, 0xa1, 0xa8]);
    g.write::<u16>(state + 0x18, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xb4), w(0x74));
    let first = g.read::<u32>(w(0xba));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), state);
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xe3), node);
    } else {
        let last = g.read::<u32>(w(0xd1));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    let fader = b.wrapping_add(w(0xe9));
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0xf3), node);
    let call = Frame {
        at: img.rel32_target(f + 0xf8),
        esp: fr.esp.wrapping_sub(0xc),
        ecx: fader,
        ..*fr
    };
    let out = crate::s4_small::fader_task(g, img, &call);
    let bits = g.read::<u16>(fader + 0x2c);
    g.write::<u16>(fader + 0x2c, bits | 3);
    disp8_stores(
        g,
        img,
        f,
        fader,
        &[0x102, 0x109, 0x110, 0x117, 0x11e, 0x125, 0x12c],
    );
    Ret::eax(out.eax)
}

/// [`fader_start_p070`] in each member (hash, start, take, fader `Sig`s).
macro_rules! fader_start_p070_copies {
    ($($sig:ident, $hash:literal, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`fader_start_p070`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 309,
                hash: $hash,
                calls: &[
                    (0x1e, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
                    (0xb4, Callee::Sig(&crate::sa_pool::TAKE)),
                    (0xf8, Callee::Sig(&crate::s4_small::FADER_TASK_40)),
                ],
                imports: &[],
            };
        )*
    };
}

fader_start_p070_copies! {
    FADER_START_0162, 0x058e_f6b6_e9fd_b1da, "`magic_0162` 0x10002f90";
    FADER_START_0586, 0x93ce_da9d_0e84_1fba, "`magic_0586` 0x10002ba0";
}

/// Parameter family P061 (`magic_0168` 0x10003500 and `magic_0587`
/// 0x10003030, 367 bytes; the block offsets differ, read from the code):
/// with `B = *G_B` (`+0x0c`), the object `B + d` (`+0x15`) starts
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale `*K`
/// `+0x04`, `(1, 0)`); its point gets four words of `B` (the `mov` pairs
/// from `+0x24`), `y` less `K2` (`+0x66`) and one more word; four words
/// of the point go back into `B` (from `+0x7b`), that `y` less `K3`
/// (`+0xa5`); then slot 541 `(*C, 0, 1)` while `*F` (`+0xc5`) is 0 and
/// `*C` (`+0xcd`) below 0x20, else slot 38 `(*H, 0, 1)` (`+0x105`); `B`'s
/// angle (`+0x10f`) plus `K4` (`+0x117`) wrapped once (above `+0x121`
/// less `+0x12b`, below `+0x135` plus `+0x142`; a NaN stays) into
/// `+0x14a`; three words set. Returns the slot's `eax`.
pub fn anim_start_p061<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = g.read::<u32>(w(0x0c));
    let obj = b.wrapping_add(w(0x15));
    let at = img.rel32_target(f + 0x20);
    let index = byte(img, f + 0x1c);
    crate::s4_misc::start_object_anim(g, img, at, obj, k(g, 0x04), index, byte(img, f + 0x1a));
    for i in 0..4 {
        let v = g.read::<u32>(b.wrapping_add(w(0x26 + 12 * i)));
        g.write::<u32>(obj.wrapping_add(w(0x2c + 12 * i)), v);
    }
    let y = sub(g.read::<f32>(obj.wrapping_add(w(0x58))), k(g, 0x66));
    let v = g.read::<u32>(b.wrapping_add(w(0x5e)));
    g.write::<u32>(obj.wrapping_add(w(0x6c)), v);
    g.write::<f32>(obj.wrapping_add(w(0x77)), y);
    let back = |g: &mut G, from: u32, to: u32| {
        let v = g.read::<u32>(obj.wrapping_add(w(from)));
        g.write::<u32>(b.wrapping_add(w(to)), v);
    };
    back(g, 0x7d, 0x83);
    back(g, 0x89, 0x8f);
    let y2 = sub(g.read::<f32>(b.wrapping_add(w(0x97))), k(g, 0xa5));
    back(g, 0x9d, 0xab);
    back(g, 0xb1, 0xb7);
    g.write::<f32>(b.wrapping_add(w(0xbf)), y2);
    let mut s = crate::pushes::Pushes::default();
    let count = g.read::<u32>(w(0xcd));
    let out = if g.read::<u32>(w(0xc5)) == 0 && count.cast_signed() < 0x20 {
        s.push_all(&[1, 0, count]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0xe2), 0x874 / 4), 0)
    } else {
        let t = g.read::<u32>(w(0xef));
        s.push_all(&[1, 0, g.read::<u32>(w(0x105))]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, g.read::<u32>(t + 0x98), 0)
    };
    let x = add(g.read::<f32>(b.wrapping_add(w(0x10f))), k(g, 0x117));
    let x = if x > k(g, 0x121) {
        sub(x, k(g, 0x12b))
    } else if k(g, 0x135) > x {
        add(x, k(g, 0x142))
    } else {
        x
    };
    g.write::<f32>(b.wrapping_add(w(0x14a)), x);
    for at in [0x14e, 0x158, 0x162] {
        g.write::<u32>(b.wrapping_add(w(at + 2)), w(at + 6));
    }
    Ret::eax(out.eax)
}

/// [`anim_start_p061`] in each member.
macro_rules! anim_start_p061_copies {
    ($($sig:ident, $hash:literal, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`anim_start_p061`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 367,
                hash: $hash,
                calls: &[(0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
                imports: &[],
            };
        )*
    };
}

anim_start_p061_copies! {
    ANIM_START_0168, 0xd51f_bfa8_2481_c700, "`magic_0168` 0x10003500";
    ANIM_START_0587, 0x514e_d979_4351_788b, "`magic_0587` 0x10003030";
}

/// Parameter family P055 (`magic_0421` 0x10001b60 and `magic_0535`
/// 0x10001b40, 434 bytes; an 8-aligned frame; the globals differ, read
/// from the code): slot 482 `(*X, *Y, n / K, 1)` (`X` `+0x45`, `Y`
/// `+0x37`, `n = *N` `+0x0d` as a float, `K` `+0x1d`) gives a float `L`;
/// slot 434 `(*G, CB1)` (`+0x60`, `+0x5a`) an object launched
/// ([`random_launch`](crate::o6_fam_launch::random_launch), `edx` 0,
/// `xmm1` `L`); when `n % 16` is 0, slot 434 `(*G, CB2)` (`+0xea`) a second
/// object that gets the first one's `+0xd0..+0xd8` and `+0xe0..+0xe8`
/// (`+0xdc`, `+0xec` 0); slot 434 `(*G, CB3)` (`+0x16d`) a third, launched
/// with `edx` 1; then `*N` + 1, and above 0x26 the argument's word gets
/// bit 2. Returns the new count, or the argument when above 0x26.
pub fn launches_p055<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, div};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; sub esp, 0x1c
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    let n = g.read::<u32>(w(0x0d));
    let q = div(cvt(n), g.read::<f32>(w(0x1d)));
    let t482 = slot_target(g, w(0x15), 0x788 / 4);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[
        1,
        q.to_bits(),
        g.read::<u32>(w(0x37)),
        g.read::<u32>(w(0x45)),
    ]);
    // call width: 12 (Pushes caps the words at 12)
    let level = crate::m0018::st0_f32(s.call(g, t482, 0).st0);
    g.write::<f32>(base, level);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x5a), g.read::<u32>(w(0x60))]);
    // call width: 12 (Pushes caps the words at 12)
    let first = s.call(g, slot_target(g, w(0x55), 0x6c8 / 4), 0).eax;
    let launch = |g: &mut G, obj: u32, edx: u32, call: u32, under: u32| {
        let fr2 = Frame {
            at: img.rel32_target(f + call),
            esp: base.wrapping_sub(0xc),
            ecx: obj,
            edx,
            xmm: [0, u128::from(level.to_bits()), 0, 0],
            ..*fr
        };
        // the word left under the call (slot 482's last, or the third
        // slot 434's callback)
        g.write::<u32>(base.wrapping_sub(8), under);
        crate::o6_fam_launch::random_launch(g, img, fr2.at, &fr2);
    };
    launch(g, first, 0, 0x7c, 1);
    // test byte [N], 0xf (the count read again)
    if g.read::<u8>(w(0x85)).trailing_zeros() >= 4 {
        let fields = [0xd0u32, 0xd4, 0xd8, 0xe0, 0xe4, 0xe8];
        let v: [u32; 6] = core::array::from_fn(|i| g.read::<u32>(first + fields[i]));
        let t = g.read::<u32>(w(0x99));
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[w(0xea), g.read::<u32>(w(0xf0))]);
        // call width: 12 (Pushes caps the words at 12)
        let second = s.call(g, g.read::<u32>(t + 0x6c8), 0).eax;
        for (o, x) in fields.into_iter().zip(v) {
            g.write::<u32>(second + o, x);
        }
        g.write::<u32>(second + 0xdc, 0);
        g.write::<u32>(second + 0xec, 0);
    }
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x16d), g.read::<u32>(w(0x173))]);
    // call width: 12 (Pushes caps the words at 12)
    let third = s.call(g, slot_target(g, w(0x168), 0x6c8 / 4), 0).eax;
    launch(g, third, 1, 0x190, w(0x16d));
    let count = g.read::<u32>(w(0x195)).wrapping_add(1);
    g.write::<u32>(w(0x19e), count);
    if count.cast_signed() > 0x26 {
        let arg = fr.arg(g, 0);
        let v = g.read::<u32>(arg);
        g.write::<u32>(arg, v | 4);
        return Ret::eax(arg);
    }
    Ret::eax(count)
}

/// [`launches_p055`] in each member (the launch's `Sig` its own).
macro_rules! launches_p055_copies {
    ($($sig:ident, $hash:literal, $launch:path, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`launches_p055`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 434,
                hash: $hash,
                calls: &[
                    (0x7c, Callee::Sig(&$launch)),
                    (0x190, Callee::Sig(&$launch)),
                ],
                imports: &[],
            };
        )*
    };
}

launches_p055_copies! {
    LAUNCHES_0421, 0x06de_37a8_4fad_462e, crate::o6_fam_launch::RANDOM_LAUNCH_0421_15E0, "`magic_0421` 0x10001b60";
    LAUNCHES_0535, 0x06de_37a8_4fad_462e, crate::o6_fam_launch::RANDOM_LAUNCH_0535_15C0, "`magic_0535` 0x10001b40";
}

/// Parameter family P034 (`magic_0393` 0x10003690 and `magic_0450`
/// 0x10007670, 769 bytes, a frame of 8; the block offsets differ, read
/// from the code). `R(x)` is slot 314 `(x)`'s float (a random value). With
/// `B = *G_B` (`+0x0a`): the counter `B + d` (`+0x10`) goes down; at 0, a
/// record ([`pool_take_140`](crate::sb_misc::pool_take_140)), when there is
/// one, starts as an object ([`start_object_at`](crate::s4_misc::start_object_at),
/// scale `(R(0.2) + K1) K2` (`+0x4b`, `+0x5b`), `(11, 0, -, 0)`) with six
/// words of `B` (the `mov` pairs from `+0x68`) and `+0xc8` = `R(pi)`, and
/// the counter becomes 3. Then three times a sprite record
/// ([`pool_take`](crate::sb_misc::pool_take); none ends the function) is
/// set: `+0x30` = `(*P)[0x1c]` (`+0xf2`), `+0x34` 1, halfwords `+0x38`
/// -1 and `+0x3a` 0x200, `+0x0..+0xc` four words of `B` (from `+0x113`)
/// with `R(4)` added to the first three, `+0x10..+0x18` `R(0.3)`, `+0x1c`
/// `R(pi)`, `+0x20..+0x28` = `-(R(0.01) + K1) * +0x10..+0x18` (the sign
/// by `xorps` with `*S` `+0x23b`), `+0x2c` 1.0, `+0x3c` = `(R(0.2) + K3)
/// K4` (`+0x2c9`, `+0x2df`), `+0x4c` 0x1f, `+0x50` 0. Returns 3, or 0 when
/// no sprite record is left.
#[allow(clippy::too_many_lines)]
pub fn sparks_p034<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // R(x): `push ecx; mov [esp], x` (or a later `mov [esp], x` over it)
    let random = |g: &mut G, table: u32, x: u32| {
        let mut s = crate::pushes::Pushes::default();
        s.push(x);
        // call width: 12 (Pushes caps the words at 12)
        st0_f32(s.call(g, slot_target(g, w(table), 0x4e8 / 4), 0).st0)
    };
    let b = g.read::<u32>(w(0x0a));
    let counter = b.wrapping_add(w(0x10));
    let left = g.read::<u32>(counter).wrapping_sub(1);
    g.write::<u32>(counter, left);
    if left == 0 {
        let obj = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + 0x1b));
        if obj != 0 {
            let r = random(g, 0x2c, w(0x33));
            let scale = mul(add(r, k(g, 0x4b)), k(g, 0x5b));
            let at = img.rel32_target(f + 0x64);
            let index = byte(img, f + 0x62);
            crate::s4_misc::start_object_at(g, img, at, obj, scale, index, byte(img, f + 0x60), 0);
            for i in 0..6 {
                let v = g.read::<u32>(b.wrapping_add(w(0x6a + 12 * i)));
                g.write::<u32>(obj.wrapping_add(w(0x70 + 12 * i)), v);
            }
            let r = random(g, 0xb1, w(0xc1));
            g.write::<f32>(obj.wrapping_add(w(0xc9)), r);
        }
        g.write::<u32>(b.wrapping_add(w(0xd2)), w(0xd6));
    }
    let mut i = 0u32;
    while i < 3 {
        let node = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0xe2));
        if node == 0 {
            return Ret::eax(0);
        }
        let p = g.read::<u32>(w(0xf2));
        let v = g.read::<u32>(p.wrapping_add(0x1c));
        g.write::<u16>(node + 0x38, 0xffff);
        g.write::<u32>(node + 0x30, v);
        g.write::<u32>(node + 0x34, 1);
        let half = w(0x10b).to_le_bytes();
        g.write::<u16>(node + 0x3a, u16::from_le_bytes([half[0], half[1]]));
        let words = [0x115u32, 0x11d, 0x126, 0x12f];
        for (j, o) in (0u32..).zip(words) {
            let v = g.read::<u32>(b.wrapping_add(w(o)));
            g.write::<u32>(node + 4 * j, v);
        }
        for (j, (table, x)) in (0u32..).zip([(0x137, 0x145), (0x158, 0x15f), (0x17d, 0x184)]) {
            let r = random(g, table, w(x));
            let v = add(g.read::<f32>(node + 4 * j), r);
            g.write::<f32>(node + 4 * j, v);
        }
        for (j, (table, x)) in (0u32..).zip([
            (0x1a3, 0x1aa),
            (0x1bf, 0x1c6),
            (0x1d6, 0x1dd),
            (0x1ed, 0x1f4),
        ]) {
            let r = random(g, table, w(x));
            g.write::<f32>(node + 0x10 + 4 * j, r);
        }
        let sign = |g: &G, at: u32| g.read::<u32>(w(at));
        for (j, (table, x, kk, s)) in (0u32..).zip([
            (0x204, 0x20b, 0x223, 0x23b),
            (0x228, 0x22f, 0x258, 0x264),
            (0x26e, 0x275, 0x28d, 0x2ac),
        ]) {
            let r = random(g, table, w(x));
            let v = mul(add(r, k(g, kk)), g.read::<f32>(node + 0x10 + 4 * j));
            if j == 2 {
                g.write::<u32>(node + 0x2c, 0x3f80_0000);
            }
            let flipped = f32::from_bits(v.to_bits() ^ sign(g, s));
            g.write::<f32>(node + 0x20 + 4 * j, flipped);
        }
        let r = random(g, 0x292, w(0x2a0));
        g.write::<u32>(node + 0x4c, w(0x2d7));
        let v = mul(add(r, k(g, 0x2c9)), k(g, 0x2df));
        g.write::<u32>(node + 0x50, 0);
        g.write::<f32>(node + 0x3c, v);
        i += 1;
    }
    Ret::eax(3)
}

/// [`sparks_p034`] in each member.
macro_rules! sparks_p034_copies {
    ($($sig:ident, $hash:literal, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`sparks_p034`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 769,
                hash: $hash,
                calls: &[
                    (0x1b, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
                    (0x64, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
                    (0xe2, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
                ],
                imports: &[],
            };
        )*
    };
}

sparks_p034_copies! {
    SPARKS_0393, 0x8010_7b07_d0f9_b650, "`magic_0393` 0x10003690";
    SPARKS_0450, 0x00bf_ba2a_5fff_f448, "`magic_0450` 0x10007670";
}

/// Parameter family P048 (`magic_0421` 0x10001960 and `magic_0535`
/// 0x10001940, 501 bytes; an 8-aligned frame): [`launches_p055`]'s kin.
/// Slot 482 `(*X, *Y, n / K, 1)` (`X` `+0x45`, `Y` `+0x37`, `n = *N`
/// `+0x0d`, `K` `+0x1d`) gives `L`; at `n` 0 slot 434 `(*G, CB0)` (`+0x6c`,
/// `+0x66`); slot 434 `(*G, CB1)` (`+0x81`) an object launched
/// ([`random_launch`](crate::o6_fam_launch::random_launch), `edx` 0, `xmm1`
/// `L`); when `n % 16` is 0 a second object (`CB2` `+0x111`) with the
/// first one's `+0xd0..+0xd8`, `+0xe0..+0xe8` (`+0xdc`, `+0xec` 0); a
/// third (`CB1` again, `+0x194`) launched with `edx` 1; then `*N` + 1, and
/// from 0x26 on `*N` = 0 and slot 450 `(arg, CB3)` (`+0x1d4`). Returns
/// slot 450's `eax`, else the new count.
pub fn launches_p048<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, div};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    let n = g.read::<u32>(w(0x0d));
    let q = div(cvt(n), g.read::<f32>(w(0x1d)));
    let t482 = slot_target(g, w(0x15), 0x788 / 4);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[
        1,
        q.to_bits(),
        g.read::<u32>(w(0x37)),
        g.read::<u32>(w(0x45)),
    ]);
    // call width: 12 (Pushes caps the words at 12)
    let level = crate::m0018::st0_f32(s.call(g, t482, 0).st0);
    g.write::<f32>(base, level);
    let object = |g: &mut G, table: u32, cb: u32, gop: u32| {
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[w(cb), g.read::<u32>(w(gop))]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(table), 0x6c8 / 4), 0).eax
    };
    if g.read::<u32>(w(0x59)) == 0 {
        object(g, 0x61, 0x66, 0x6c);
    }
    let launch = |g: &mut G, obj: u32, edx: u32, call: u32, under: u32| {
        let fr2 = Frame {
            at: img.rel32_target(f + call),
            esp: base.wrapping_sub(0xc),
            ecx: obj,
            edx,
            xmm: [0, u128::from(level.to_bits()), 0, 0],
            ..*fr
        };
        g.write::<u32>(base.wrapping_sub(8), under);
        crate::o6_fam_launch::random_launch(g, img, fr2.at, &fr2);
    };
    let first = object(g, 0x7c, 0x81, 0x87);
    launch(g, first, 0, 0xa3, w(0x81));
    // test byte [N], 0xf (the count read again)
    if g.read::<u8>(w(0xac)).trailing_zeros() >= 4 {
        let fields = [0xd0u32, 0xd4, 0xd8, 0xe0, 0xe4, 0xe8];
        let v: [u32; 6] = core::array::from_fn(|i| g.read::<u32>(first + fields[i]));
        let t = g.read::<u32>(w(0xc0));
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[w(0x111), g.read::<u32>(w(0x117))]);
        // call width: 12 (Pushes caps the words at 12)
        let second = s.call(g, g.read::<u32>(t + 0x6c8), 0).eax;
        for (o, x) in fields.into_iter().zip(v) {
            g.write::<u32>(second + o, x);
        }
        g.write::<u32>(second + 0xdc, 0);
        g.write::<u32>(second + 0xec, 0);
    }
    let third = object(g, 0x18f, 0x194, 0x19a);
    launch(g, third, 1, 0x1b7, w(0x194));
    let count = g.read::<u32>(w(0x1bc)).wrapping_add(1);
    g.write::<u32>(w(0x1c5), count);
    if count.cast_signed() < 0x26 {
        return Ret::eax(count);
    }
    let t450 = g.read::<u32>(w(0x1cf));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x1d4), fr.arg(g, 0)]);
    g.write::<u32>(w(0x1e3), w(0x1e7));
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, g.read::<u32>(t450 + 0x708), 0).eax)
}

/// [`launches_p048`] in each member (the launch's `Sig` its own).
macro_rules! launches_p048_copies {
    ($($sig:ident, $hash:literal, $launch:path, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`launches_p048`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 501,
                hash: $hash,
                calls: &[
                    (0xa3, Callee::Sig(&$launch)),
                    (0x1b7, Callee::Sig(&$launch)),
                ],
                imports: &[],
            };
        )*
    };
}

launches_p048_copies! {
    LAUNCHES_P048_0421, 0x4a6c_8793_4c93_096b, crate::o6_fam_launch::RANDOM_LAUNCH_0421_15E0, "`magic_0421` 0x10001960";
    LAUNCHES_P048_0535, 0x4a6c_8793_4c93_096b, crate::o6_fam_launch::RANDOM_LAUNCH_0535_15C0, "`magic_0535` 0x10001940";
}

/// Parameter family P063 (`magic_0393` 0x100041c0 and `magic_0450`
/// 0x10007fe0, 353 bytes; no frame): five words set and slot 719 `(NAME,
/// 8, 0)` into `G_PART` (`+0x4c`); the particle set-up
/// ([`part_setup`](crate::sb_misc::part_setup), the pushed buffer `+0x51`);
/// the DLL's own facing (`magic_0393`'s [`face`](crate::m0393::face),
/// `magic_0450`'s [`orient`](crate::m0450::orient)); a word set and `+0x0`
/// of 0x40 records (0x140 bytes from the immediate `+0x6d`) cleared; five
/// more words, slot 271 `(4)`; two task nodes ([`take`](crate::sa_pool::take);
/// bytes 1 and 2) appended to the task list with the pool reset between
/// them ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take), the
/// pool `+0x106`). Returns 0.
pub fn records_start_p063<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let s719 = slot_target(g, w(0x01), 0xb3c / 4);
    stores(g, img, f, &[0x14, 0x1e, 0x28, 0x32, 0x3c]);
    let part = g.call(s719, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x4c), part);
    // push 0 stays; push BUF; call
    g.write::<u32>(e - 8, w(0x51));
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x56), e - 0xc);
    let facing = img.rel32_target(f + 0x5e);
    if img.matches(facing, &crate::m0393::FACE) {
        crate::m0393::face(g, img, facing);
    } else {
        crate::m0450::orient(g, img, facing);
    }
    stores(g, img, f, &[0x62]);
    let mut rec = w(0x6d);
    for _ in 0..w(0x72) {
        g.write::<u32>(rec, 0);
        rec = rec.wrapping_add(0x140);
    }
    let s271 = slot_target(g, w(0x90), 0x43c / 4);
    stores(g, img, f, &[0x94]);
    g.write::<u32>(w(0xa0), 0);
    g.write::<u16>(w(0xa7), 0);
    g.write::<u8>(w(0xad), img.word(f + 0xb1).to_le_bytes()[0]);
    g.write::<u32>(w(0xbc), 0);
    stores(g, img, f, &[0xc0]);
    g.call(s271, 0, &[4]);
    let node = |g: &mut G, call: u32, flag: u8, (first, last): (u32, u32)| {
        let n = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(call - 5));
        let head = g.read::<u32>(w(first));
        g.write::<u8>(n.wrapping_add(0x18), flag);
        g.write::<u32>(n.wrapping_add(4), 0);
        (n, head, last)
    };
    let (a, head, last) = node(g, 0xd5, 1, (0xdb, 0xef));
    if head == 0 {
        g.write::<u32>(a, 0);
        g.write::<u32>(w(0x101), a);
    } else {
        let tail = g.read::<u32>(w(last));
        g.write::<u32>(a, tail);
        g.write::<u32>(tail + 4, a);
    }
    g.write::<u32>(a.wrapping_add(8), a);
    g.write::<u32>(w(0x10e), a);
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x113), w(0x106));
    let (b, head, last) = node(g, 0x11d, 2, (0x123, 0x137));
    if head == 0 {
        g.write::<u32>(b, 0);
        g.write::<u32>(w(0x152), b);
        g.write::<u32>(b.wrapping_add(8), b);
        g.write::<u32>(w(0x15a), b);
    } else {
        let tail = g.read::<u32>(w(last));
        g.write::<u32>(b, tail);
        g.write::<u32>(tail + 4, b);
        g.write::<u32>(b.wrapping_add(8), b);
        g.write::<u32>(w(0x144), b);
    }
    Ret::eax(0)
}

/// [`records_start_p063`] in each member (its facing's `Sig`).
macro_rules! records_start_p063_copies {
    ($($sig:ident, $hash:literal, $facing:path, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`records_start_p063`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 353,
                hash: $hash,
                calls: &[
                    (0x56, Callee::Sig(&crate::sb_misc::PART_SETUP)),
                    (0x5e, Callee::Sig(&$facing)),
                    (0xd5, Callee::Sig(&crate::sa_pool::TAKE)),
                    (0x113, Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0205_2A60)),
                    (0x11d, Callee::Sig(&crate::sa_pool::TAKE)),
                ],
                imports: &[],
            };
        )*
    };
}

records_start_p063_copies! {
    RECORDS_START_0393, 0x0ca3_9f48_ba84_daf6, crate::m0393::FACE, "`magic_0393` 0x100041c0";
    RECORDS_START_0450, 0x0ca3_9f48_ba84_daf6, crate::m0450::ORIENT, "`magic_0450` 0x10007fe0";
}

/// Parameter family P030 (`magic_0150` 0x1000b720 and `magic_0603`
/// 0x1000b650, 903 bytes, a frame of 0x24; `ecx` = the effect `E`, `edx`
/// a word `A`; the field offsets and globals differ, read from the code):
/// two of `E`'s nodes put at the head of the scene list (`*L`, the flag
/// word `*F` bit 0 cleared), three of its children (`E + d`) given an
/// owner ring each and appended to the root's child list (`R`, count
/// `C`), colour words 1.0; a child object set up on the frame block
/// ([`child_object_setup`](crate::s8_misc::child_object_setup)), slot 325
/// `(P, Q)`; slots 719 `(NAME, 4, 0)` and `(NAME, 9, 0)` and a sprite
/// node ([`sprite_node`](crate::s4_small::sprite_node)) with them, the
/// flag `*N == 1`; three `rand`-like slot 324 values `r % 20 * 0x4000 /
/// 90` into `E`; three words. Returns the last value.
#[allow(clippy::too_many_lines)]
pub fn scene_ctor_p030<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let e = fr.ecx;
    let arg = fr.edx;
    let at = |o: u32| e.wrapping_add(w(o));
    // `E[d] = E`, `E[d] = imm` twice
    g.write::<u32>(at(0x0d), e);
    g.write::<u32>(at(0x13), w(0x17));
    g.write::<u32>(at(0x1d), w(0x21));
    // a node goes first on the scene list
    let push_front =
        |g: &mut G, n: u32, (l1, head, l2, fl, imm8, l3): (u32, u32, u32, u32, u32, u32)| {
            let first = g.read::<u32>(w(l1));
            g.write::<u32>(n + 8, first);
            g.write::<u32>(n + 4, w(head));
            let first = g.read::<u32>(w(l2));
            g.write::<u32>(first + 4, n);
            let flags = g.read::<u32>(w(fl));
            g.write::<u32>(
                w(fl),
                flags
                    & i32::from(img.word(f + imm8).to_le_bytes()[0].cast_signed()).cast_unsigned(),
            );
            g.write::<u32>(w(l3), n);
        };
    let a = at(0x27);
    let b = at(0x2d);
    g.write::<u32>(a + 8, a);
    g.write::<u32>(a + 4, a);
    push_front(g, a, (0x38, 0x42, 0x47, 0x52, 0x56, 0x59));
    g.write::<u32>(at(0x5f), w(0x63));
    g.write::<u32>(b, w(0x69));
    g.write::<u32>(b + 8, b);
    g.write::<u32>(b + 4, b);
    g.write::<u32>(b + 0x10, w(0x76));
    g.write::<u32>(b + 0x14, e);
    push_front(g, b, (0x7e, 0x88, 0x8d, 0x9c, 0xa0, 0xa3));
    g.write::<u32>(at(0xa9), w(0xad));
    // a child of the root: owner ring, appended to the root's children
    let child = |g: &mut G, c: u32, ops: [u32; 9]| {
        let [vtable, half, a14, a4, q1, q2, root, r_first, cnt_q] = ops;
        if vtable != 0 {
            g.write::<u32>(c, w(vtable));
        }
        let h = img.word(f + half).to_le_bytes();
        g.write::<u16>(c + 0x1a, u16::from_le_bytes([h[0], h[1]]));
        g.write::<u32>(c + 0x10, c);
        g.write::<u32>(c + 0xc, c);
        g.write::<u32>(c + 0x14, w(a14));
        g.write::<u32>(c + 4, w(a4));
        let last = g.read::<u32>(w(q1));
        g.write::<u32>(c + 8, last);
        let last = g.read::<u32>(w(q2));
        if last == w(root) {
            g.write::<u32>(w(r_first), c);
        } else {
            g.write::<u32>(last + 4, c);
        }
        let n = g.read::<u16>(w(cnt_q));
        g.write::<u16>(w(cnt_q), n.wrapping_add(1));
        c
    };
    let s = child(
        g,
        at(0x93),
        [0, 0xb5, 0xc0, 0xc7, 0xcc, 0xd4, 0xdc, 0xe4, 0xf0],
    );
    g.write::<u32>(w(0xf6), s);
    g.write::<u32>(s, w(0xfc));
    for o in [0x100u32, 0x10a, 0x114, 0x11e] {
        g.write::<u32>(at(o + 2), w(o + 6));
    }
    let d = child(
        g,
        at(0x12a),
        [
            0x130, 0x138, 0x143, 0x14a, 0x14f, 0x157, 0x15c, 0x164, 0x170,
        ],
    );
    g.write::<u32>(w(0x176), d);
    let c = child(
        g,
        at(0x17c),
        [
            0x182, 0x18a, 0x195, 0x19c, 0x1a1, 0x1a9, 0x1ae, 0x1b6, 0x1c2,
        ],
    );
    g.write::<u32>(w(0x1c8), c);
    // the frame block for the child object set-up
    let block = ebp - 0x24;
    let obj = at(0x1ce);
    g.write::<u32>(block, obj);
    let sv = g.read::<u32>(w(0x1d6));
    g.write::<u32>(c.wrapping_add(w(0x1dc)), w(0x1e0));
    g.write::<u32>(c.wrapping_add(w(0x1e6)), d);
    g.write::<u32>(c.wrapping_add(w(0x1ec)), s);
    g.write::<u32>(c.wrapping_add(w(0x1f2)), w(0x1f6));
    g.write::<u8>(
        c.wrapping_add(w(0x1fc)),
        img.word(f + 0x200).to_le_bytes()[0],
    );
    g.write::<u32>(block + 0x10, c);
    g.write::<u32>(block + 4, w(0x20a));
    g.write::<u32>(block + 8, e);
    g.write::<u32>(block + 0xc, w(0x214));
    g.write::<u32>(block + 0x14, arg);
    g.write::<u32>(block + 0x18, w(0x21e));
    g.write::<u32>(block + 0x1c, sv);
    g.write::<u32>(ebp - 4, arg);
    let esp = ebp - 0x24 - 0xc;
    let call = Frame {
        at: img.rel32_target(f + 0x226),
        esp: esp - 4,
        ecx: block,
        ..*fr
    };
    crate::s8_misc::child_object_setup(g, img, &call);
    let first = g.read::<u32>(block);
    g.write::<u32>(first, w(0x234));
    let mut st = crate::pushes::Pushes::default();
    st.push_all(&[w(0x22e), w(0x23e)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x239), 0x514 / 4), 0);
    let flag = u32::from(g.read::<u32>(w(0x24b)) == 1);
    let mut st = crate::pushes::Pushes::default();
    st.push_all(&[0, byte(img, f + 0x26c), w(0x26e)]);
    // call width: 12 (Pushes caps the words at 12)
    let h4 = st.call(g, slot_target(g, w(0x25f), 0xb3c / 4), 0).eax;
    g.write::<u32>(block + 0xc, at(0x278));
    g.write::<u32>(block + 0x10, arg);
    let mut st = crate::pushes::Pushes::default();
    st.push_all(&[0, byte(img, f + 0x29c), w(0x29e)]);
    let t719 = slot_target(g, w(0x289), 0xb3c / 4);
    g.write::<u32>(block, at(0x297));
    g.write::<u32>(block + 4, w(0x2a8));
    g.write::<u32>(block + 8, w(0x2af));
    // call width: 12 (Pushes caps the words at 12)
    let h9 = st.call(g, t719, 0).eax;
    g.write::<u32>(block + 0x14, h9);
    g.write::<u32>(block + 0x18, h4);
    g.write::<u32>(block + 0x1c, flag);
    let call = Frame {
        at: img.rel32_target(f + 0x2c5),
        esp: esp - 4,
        ecx: block,
        ..*fr
    };
    crate::s4_small::sprite_node(g, img, &call);
    let first = g.read::<u32>(block);
    g.write::<u32>(first, w(0x2ce));
    let divisor = w(0x2e1);
    let magic = w(0x2e8).cast_signed();
    let mut last = 0u32;
    for (table, dst) in [(0x2d3u32, 0x301u32), (0x306, 0x335), (0x325, 0x35e)] {
        let r = g
            .call(slot_target(g, w(table), 0x510 / 4), 0, &[])
            .eax
            .cast_signed();
        let m = r.wrapping_rem(divisor.cast_signed()).wrapping_shl(14);
        // imul: the high word of the product
        #[allow(clippy::cast_possible_truncation)]
        let hi = ((i64::from(magic) * i64::from(m)) >> 32) as i32;
        let q = hi.wrapping_add(m) >> 6;
        last = q
            .wrapping_add((q.cast_unsigned() >> 31).cast_signed())
            .cast_unsigned();
        g.write::<u32>(e.wrapping_add(w(dst)), last);
    }
    for o in [0x362u32, 0x36c, 0x376] {
        g.write::<u32>(at(o + 2), w(o + 6));
    }
    Ret::eax(last)
}

/// [`scene_ctor_p030`] in each member.
macro_rules! scene_ctor_p030_copies {
    ($($sig:ident, $hash:literal, $doc:literal;)*) => {
        $(
            #[doc = concat!("[`scene_ctor_p030`] in ", $doc, ".")]
            pub const $sig: Sig = Sig {
                len: 903,
                hash: $hash,
                calls: &[
                    (0x226, Callee::Sig(&crate::s8_misc::CHILD_OBJECT_SETUP)),
                    (0x2c5, Callee::Sig(&crate::s4_small::SPRITE_NODE)),
                ],
                imports: &[],
            };
        )*
    };
}

scene_ctor_p030_copies! {
    SCENE_CTOR_0150, 0x25aa_41a9_5c71_15ab, "`magic_0150` 0x1000b720";
    SCENE_CTOR_0603, 0x2eeb_05f6_0af8_194f, "`magic_0603` 0x1000b650";
}
