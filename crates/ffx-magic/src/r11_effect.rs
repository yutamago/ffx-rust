//! Round 11 (magic-r11-own): kin of `r9_misc::effect_start` (one effect
//! object, its point and yaw, a record and a task node).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// [`effect_start`](crate::r9_misc::effect_start) without the point's
/// drop and with one more word (`magic_0252` 0x100013a0, 402 bytes; a
/// frame with one local): `*G_A`/`*G_B` set, slot 719 `(NAME, 8, 0)`
/// into `G_PART`, the part set up; the object `OBJ` (`+0x47`, scale `*K`
/// `+0x3f`) starts ([`start_object_reset_arg`](crate::r9_misc::start_object_reset_arg));
/// slot 262 `(*G_H, P, P + 4, P + 8)` (`+0x6c`; the immediates `+0x66`,
/// `+0x61`, `+0x56`), `P.w` (`+0x7f`) 1.0; slot 333 `(*G_H)`'s float `v`
/// makes the yaw `K3 - v` (`+0x96`) wrapped once (above `+0xa5` less
/// `+0xaf`, below `+0xb9` plus `+0xc6`; a NaN stays) into `+0xd5`; the
/// word `+0xdb` and the record words (`+0xe5` .. `+0x13b`, a halfword
/// `+0x135`); the actor block ([`me_block`](crate::sa_misc::me_block),
/// the immediate `+0xcd`); a free-list node
/// ([`pop_node`](crate::m0032::pop_node)) appended to the task list (first
/// `+0x152`, last `+0x162`). Returns 0.
pub fn effect_start_flat<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s719 = slot_target(g, w(0x05), 0xb3c / 4);
    stores(g, img, f, &[0x18, 0x22]);
    let part = g.call(s719, 0, &[w(0x14), 8, 0]).eax;
    g.write::<u32>(w(0x32), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x37));
    // push ebp; push ecx; three pushes, one popped; sub esp, 8; call
    let call = Frame {
        at: img.rel32_target(f + 0x4c),
        esp: fr.esp.wrapping_sub(0x1c),
        edx: w(0x47),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x3f)))],
        ..*fr
    };
    crate::r9_misc::start_object_reset_arg(g, img, &call);
    let mut s = crate::pushes::Pushes::default();
    let h = g.read::<u32>(w(0x6c));
    s.push_all(&[w(0x56), w(0x61), w(0x66), h]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x51), 0x418 / 4), 0);
    let t333 = g.read::<u32>(w(0x73));
    s.push(g.read::<u32>(w(0x79)));
    stores(g, img, f, &[0x7d]);
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(s.call(g, g.read::<u32>(t333 + 0x534), 0).st0);
    let x = sub(k(g, 0x96), v);
    let yaw = if x > k(g, 0xa5) {
        sub(x, k(g, 0xaf))
    } else if k(g, 0xb9) > x {
        add(x, k(g, 0xc6))
    } else {
        x
    };
    g.write::<f32>(w(0xd5), yaw);
    stores(
        g,
        img,
        f,
        &[0xd9, 0xe3, 0xed, 0xf7, 0x101, 0x10b, 0x115, 0x11f, 0x129],
    );
    g.write::<u16>(w(0x135), 0);
    stores(g, img, f, &[0x139]);
    // push IMM; call: the block's word (an argument of the port)
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x144), w(0xcd));
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x14c));
    let first = g.read::<u32>(w(0x152));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x180), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x188), node);
    } else {
        let last = g.read::<u32>(w(0x162));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x16f), node);
    }
    Ret::eax(0)
}

/// [`effect_start_flat`] (`magic_0252` 0x100013a0).
pub const EFFECT_START_FLAT_0252: Sig = Sig {
    len: 402,
    hash: 0x90d1_fc0c_86da_b386,
    calls: &[
        (0x37, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4c, Callee::Sig(&crate::r9_misc::START_OBJECT_RESET_0252)),
        (0x144, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x14c, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// One effect object at the first actor whose point's height comes from
/// slot 268 (`magic_0365` 0x10002c10, 467 bytes, a /GS frame): `*G_A`/
/// `*G_B` set, slot 719 `(NAME, 8, 0)` into `G_PART`, the part set up;
/// the object `OBJ` (`+0x54`, scale `*K` `+0x4c`) starts
/// ([`start_object_reset_arg`](crate::r9_misc::start_object_reset_arg));
/// the first actor's character (`+0x5e`, `+0x72`) puts the point `P`
/// with slot 262 `(chr, P, P + 4, P + 8)` (the immediates `+0x7c`,
/// `+0x6b`, `+0x63`), slot 268 `(chr, 6, &V)` (`V` on the frame at `ebp -
/// 0x14`) gives `P.y` = `V.y` (`+0xab`), `P.w` 1.0; slot 333 `(chr)`'s
/// float `v` makes the yaw `K3 - v` (`+0xd7`) wrapped once (above
/// `+0xe7` less `+0xf1`, below `+0xfb` plus `+0x108`; a NaN stays) into
/// `+0x122`, `P.y` less `K4` (`+0x118`); the record words (`+0x128` ..
/// `+0x186`, a halfword `+0x180`); a free-list node
/// ([`pop_node`](crate::m0032::pop_node)) appended to the task list (first
/// `+0x195`, last `+0x1a5`). Returns 0.
pub fn effect_start_height<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let v_at = ebp - 0x14;
    let s719 = slot_target(g, w(0x11), 0xb3c / 4);
    stores(g, img, f, &[0x25, 0x2f]);
    let part = g.call(s719, 0, &[w(0x21), 8, 0]).eax;
    g.write::<u32>(w(0x3f), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x44));
    // sub esp, 0x18; push esi; two words left; sub esp, 8; call
    let call = Frame {
        at: img.rel32_target(f + 0x59),
        esp: ebp - 0x18 - 4 - 8 - 8 - 4,
        edx: w(0x54),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x4c)))],
        ..*fr
    };
    crate::r9_misc::start_object_reset_arg(g, img, &call);
    let chr = |g: &G, sel: u32, table: u32| {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
        g.read::<u32>(w(table).wrapping_add(4 * b))
    };
    let c = chr(g, 0x5e, 0x72);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x63), w(0x6b), w(0x7c), c]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x77), 0x418 / 4), 0);
    s.push_all(&[v_at, 6, c]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x8e), 0x430 / 4), 0);
    let y = g.read::<u32>(v_at + 4);
    g.write::<u32>(w(0xab), y);
    stores(g, img, f, &[0xaf]);
    s.push(chr(g, 0xa3, 0xbf));
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0xc4), 0x534 / 4), 0).st0);
    let x = sub(k(g, 0xd7), v);
    let yaw = if x > k(g, 0xe7) {
        sub(x, k(g, 0xf1))
    } else if k(g, 0xfb) > x {
        add(x, k(g, 0x108))
    } else {
        x
    };
    let py = sub(k(g, 0x110), k(g, 0x118));
    g.write::<f32>(w(0x122), yaw);
    stores(g, img, f, &[0x126, 0x130]);
    g.write::<f32>(w(0x13e), py);
    stores(g, img, f, &[0x142, 0x14c, 0x156, 0x160, 0x16a, 0x174]);
    g.write::<u16>(w(0x180), 0);
    stores(g, img, f, &[0x184]);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x18f));
    let first = g.read::<u32>(w(0x195));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x1b7), node);
    } else {
        let last = g.read::<u32>(w(0x1a5));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x1c2), node);
    // __security_check_cookie leaves `eax`
    Ret::eax(0)
}

/// [`effect_start_height`] (`magic_0365` 0x10002c10).
pub const EFFECT_START_HEIGHT_0365: Sig = Sig {
    len: 467,
    hash: 0xb113_fb64_fc47_b839,
    calls: &[
        (0x44, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x59, Callee::Sig(&crate::r9_misc::START_OBJECT_RESET_0365)),
        (0x18f, Callee::Sig(&crate::m0032::POP_NODE)),
        // __security_check_cookie
        (0x1cb, Callee::Any),
    ],
    imports: &[],
};

/// One effect object after the actor lookup (`magic_0197` 0x10001370,
/// 389 bytes, a frame with one local): four words set (`+0x04` ..
/// `+0x22`; the third is the count `C`), the actors looked up
/// ([`lookup_actors`](crate::sb_misc::lookup_actors)); slot 719 `(NAME,
/// 8, 0)` into `G_PART` (`+0x4b`), the part set up; the object `OBJ`
/// (`+0x60`, scale `*K` `+0x58`) starts
/// ([`start_object_reset_arg`](crate::r9_misc::start_object_reset_arg));
/// slot 262 `(*G_H, P, P + 4, P + 8)` (`+0x85`; the immediates `+0x7f`,
/// `+0x7a`, `+0x6f`), `P.w` (`+0x98`) 1.0; slot 333 `(*G_H)`'s float `v`
/// makes the yaw `K3 - v` (`+0xaf`) wrapped once (above `+0xbe` less
/// `+0xc8`, below `+0xd2` plus `+0xdf`; a NaN stays) into `+0xfe` and
/// `P.y` less `K4` (`+0xf6`); then slot 257 `(*G_H, 1)` when `*G_F`
/// (`+0xe5`) is set or `*C` (`+0x10d`) is 0x20 or more, else slot 296
/// `(*C, 0)`; a free-list node ([`pop_node`](crate::m0032::pop_node))
/// appended to the task list (first `+0x145`, last `+0x155`). Returns 0.
pub fn effect_start_lookup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    stores(g, img, f, &[0x04, 0x0e, 0x18, 0x22]);
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x2d));
    let s719 = slot_target(g, w(0x32), 0xb3c / 4);
    let part = g.call(s719, 0, &[w(0x41), 8, 0]).eax;
    g.write::<u32>(w(0x4b), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x50));
    // push ebp; push ecx; two words left; sub esp, 8; call
    let call = Frame {
        at: img.rel32_target(f + 0x65),
        esp: fr.esp.wrapping_sub(0x1c),
        edx: w(0x60),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x58)))],
        ..*fr
    };
    crate::r9_misc::start_object_reset_arg(g, img, &call);
    let mut s = crate::pushes::Pushes::default();
    let h = g.read::<u32>(w(0x85));
    s.push_all(&[w(0x6f), w(0x7a), w(0x7f), h]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x6a), 0x418 / 4), 0);
    let t333 = g.read::<u32>(w(0x8c));
    s.push(g.read::<u32>(w(0x92)));
    stores(g, img, f, &[0x96]);
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(s.call(g, g.read::<u32>(t333 + 0x534), 0).st0);
    let x = sub(k(g, 0xaf), v);
    let yaw = if x > k(g, 0xbe) {
        sub(x, k(g, 0xc8))
    } else if k(g, 0xd2) > x {
        add(x, k(g, 0xdf))
    } else {
        x
    };
    let busy = g.read::<u32>(w(0xe5)) != 0;
    let y = sub(k(g, 0xee), k(g, 0xf6));
    g.write::<f32>(w(0xfe), yaw);
    g.write::<f32>(w(0x106), y);
    let mut s = crate::pushes::Pushes::default();
    let count = if busy { 0x20 } else { g.read::<u32>(w(0x10d)) };
    if busy || count.cast_signed() >= 0x20 {
        let t = g.read::<u32>(w(0x127));
        s.push_all(&[1, g.read::<u32>(w(0x12f))]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, g.read::<u32>(t + 0x404), 0);
    } else {
        s.push_all(&[0, count]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x11a), 0x4a0 / 4), 0);
    }
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x13f));
    let first = g.read::<u32>(w(0x145));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x173), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x17b), node);
    } else {
        let last = g.read::<u32>(w(0x155));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x162), node);
    }
    Ret::eax(0)
}

/// [`effect_start_lookup`] (`magic_0197` 0x10001370).
pub const EFFECT_START_LOOKUP_0197: Sig = Sig {
    len: 389,
    hash: 0xbfd2_1fd8_5859_9630,
    calls: &[
        (0x2d, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x50, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x65, Callee::Sig(&crate::r9_misc::START_OBJECT_RESET_0197)),
        (0x13f, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// One effect object at the first actor, placed by the actor's kind
/// (`magic_0231` 0x10001370, 565 bytes, a /GS frame): `*G_A`/`*G_B` set,
/// slot 719 `(NAME, 8, 0)` into `G_PART` (`+0x3f`), the part set up; slot
/// 293 `(SEL[0])` (`+0x49`) gives the kind; the object `OBJ` (`+0x6d`,
/// scale `*K` `+0x62`) starts
/// ([`start_object_reset`](crate::s6_misc::start_object_reset)); slot 262
/// `(chr, P, P + 4, P + 8)` for the first actor's character, then for
/// kind 0 slot 268 `(chr, 6, &V)` (`V` on the frame at `ebp - 0x14`) and
/// `P.y` = `V.y - K2` (`+0xc3`, into `+0xcb`), else slot 268 `(chr, 0x10,
/// P)`; `P.w` 1.0 and a word set (`+0x11c`, `+0x126`); slot 333 `(chr)`'s
/// float `v` makes the yaw `K3 - v` (`+0x151`) wrapped once (above
/// `+0x161` less `+0x16b`, below `+0x175` plus `+0x182`; a NaN stays)
/// into `+0x18c`; the record words (`+0x192` .. `+0x1e8`, a halfword
/// `+0x1e2`); a free-list node ([`pop_node`](crate::m0032::pop_node))
/// appended to the task list (first `+0x1f7`, last `+0x207`). Returns 0.
pub fn effect_start_kind<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let v_at = ebp - 0x14;
    let s719 = slot_target(g, w(0x11), 0xb3c / 4);
    stores(g, img, f, &[0x25, 0x2f]);
    let part = g.call(s719, 0, &[w(0x21), 8, 0]).eax;
    g.write::<u32>(w(0x3f), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x44));
    let first_byte = |g: &G, sel: u32| u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
    let chr = |g: &G, sel: u32, table: u32| {
        let b = first_byte(g, sel);
        g.read::<u32>(w(table).wrapping_add(4 * b))
    };
    let mut s = crate::pushes::Pushes::default();
    s.push(first_byte(g, 0x49));
    // call width: 12 (Pushes caps the words at 12)
    let kind = s.call(g, slot_target(g, w(0x52), 0x494 / 4), 0).eax;
    // sub esp, 0x18; push esi; add esp, 0xc; sub esp, 0x10; call
    let (start, ops) = if kind == 0 {
        (0x76, (0x7b, 0x8f, 0x94, 0x80, 0x88, 0x99, 0xab))
    } else {
        (0xd2, (0xd7, 0xeb, 0xf0, 0xdc, 0xe4, 0xf5, 0x103))
    };
    let call = Frame {
        at: img.rel32_target(f + start),
        esp: ebp - 0x18 - 4 - 0x10 - 4,
        edx: w(0x6d),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x62)))],
        ..*fr
    };
    crate::s6_misc::start_object_reset(g, img, &call);
    let (sel, table, t262, z, y, x, t268) = ops;
    let c = chr(g, sel, table);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(z), w(y), w(x), c]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(t262), 0x418 / 4), 0);
    if kind == 0 {
        s.push_all(&[v_at, 6, c]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(t268), 0x430 / 4), 0);
        let py = sub(g.read::<f32>(v_at + 4), k(g, 0xc3));
        g.write::<f32>(w(0xcb), py);
    } else {
        s.push_all(&[w(0x108), 0x10, c]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(t268), 0x430 / 4), 0);
    }
    stores(g, img, f, &[0x11c, 0x126]);
    let mut s = crate::pushes::Pushes::default();
    s.push(chr(g, 0x118, 0x139));
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x13e), 0x534 / 4), 0).st0);
    let x = sub(k(g, 0x151), v);
    let yaw = if x > k(g, 0x161) {
        sub(x, k(g, 0x16b))
    } else if k(g, 0x175) > x {
        add(x, k(g, 0x182))
    } else {
        x
    };
    g.write::<f32>(w(0x18c), yaw);
    stores(
        g,
        img,
        f,
        &[0x190, 0x19a, 0x1a4, 0x1ae, 0x1b8, 0x1c2, 0x1cc, 0x1d6],
    );
    g.write::<u16>(w(0x1e2), 0);
    stores(g, img, f, &[0x1e6]);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x1f1));
    let first = g.read::<u32>(w(0x1f7));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x219), node);
    } else {
        let last = g.read::<u32>(w(0x207));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x224), node);
    // __security_check_cookie leaves `eax`
    Ret::eax(0)
}

/// [`effect_start_kind`] (`magic_0231` 0x10001370).
pub const EFFECT_START_KIND_0231: Sig = Sig {
    len: 565,
    hash: 0x370a_1dbd_715d_1140,
    calls: &[
        (0x44, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x76, Callee::Sig(&crate::s6_misc::START_OBJECT_RESET)),
        (0xd2, Callee::Sig(&crate::s6_misc::START_OBJECT_RESET)),
        (0x1f1, Callee::Sig(&crate::m0032::POP_NODE)),
        // __security_check_cookie
        (0x22d, Callee::Any),
    ],
    imports: &[],
};
