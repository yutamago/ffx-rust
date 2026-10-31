//! `magic_0150.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sa_summon::{MODEL_NODE_INIT, model_node_init};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `mov dword [w(dst)], w(val)` for each pair, in order.
fn stores<G: Game>(g: &mut G, img: &Image, f: u32, pairs: &[(u32, u32)]) {
    for &(dst, val) in pairs {
        g.write::<u32>(img.word(f + dst), img.word(f + val));
    }
}

/// The start-up (0x1000cb70, no arguments, a frame of 8 bytes): a node
/// put at the ring's tail (`+0x04`..`+0x38`), the root's flag cleared
/// (`+0x43`), a word cleared (`+0x57`), the static node `N` (`+0x61`..)
/// appended to the root list (count `+0xbc`), its flag byte `+0xcf` and
/// the word `+0xd6` set to 1, and the colour node
/// ([`colour_node`](crate::s3_misc::colour_node), the group `[node,
/// parent]` on the frame, the node's vtable `+0xf2`). Returns the colour
/// node's `eax`.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let tail = g.read::<u32>(w(0x04));
    g.write::<u32>(w(0x09), tail);
    stores(g, img, f, &[(0x0f, 0x13), (0x19, 0x1d)]);
    g.write::<u32>(w(0x23), 0);
    stores(g, img, f, &[(0x2d, 0x31)]);
    g.write::<u32>(tail + 4, w(0x38));
    let last = g.read::<u32>(w(0x3d));
    let v = g.read::<u32>(w(0x43));
    g.write::<u32>(w(0x43), v & !1);
    stores(g, img, f, &[(0x4d, 0x51)]);
    g.write::<u32>(w(0x57), 0);
    stores(g, img, f, &[(0x61, 0x65)]);
    g.write::<u16>(w(0x6c), 0xff);
    stores(
        g,
        img,
        f,
        &[(0x74, 0x78), (0x7e, 0x82), (0x88, 0x8c), (0x92, 0x96)],
    );
    g.write::<u32>(w(0x9b), last);
    if last == w(0xa0) {
        stores(g, img, f, &[(0xa8, 0xac)]);
    } else {
        g.write::<u32>(last + 4, w(0xb5));
    }
    let c = g.read::<u16>(w(0xbc));
    g.write::<u16>(w(0xbc), c.wrapping_add(1));
    stores(g, img, f, &[(0xc5, 0xc9)]);
    g.write::<u8>(w(0xcf), 1);
    g.write::<u32>(w(0xd6), 1);
    // push ebp; sub esp, 8: the group at [ebp - 8]
    let ebp = fr.esp.wrapping_sub(4);
    let grp = ebp.wrapping_sub(8);
    g.write::<u32>(grp, w(0xe1));
    g.write::<u32>(grp + 4, w(0xe8));
    stores(g, img, f, &[(0xee, 0xf2)]);
    let at = img.rel32_target(f + 0xf7);
    let eax = crate::s3_misc::colour_node(
        g,
        img,
        &Frame {
            at,
            esp: ebp.wrapping_sub(0xc),
            ecx: grp,
            ..Frame::default()
        },
    );
    Ret::eax(eax)
}

/// [`start_up`] (255 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 255,
    hash: 0x0071_115a_6ec1_b050,
    calls: &[(0xf7, Callee::Sig(&crate::s3_misc::COLOUR_NODE))],
    imports: &[],
};

/// The loading steps and the frame (0x1000de60, no arguments, a frame of
/// 8 bytes): the step `*S` (`+0x04`): 0 and 1 wait for slots 225 and 313
/// `(2)` to say 0 (each then the next step), 2 for slot 549 `(2)`, then
/// slot 226 `(*F, 0)` (`+0x71`) and step 4. From step 3 on, the frame:
/// slot 673, `**P` the immediate `+0xa5`, the pass byte `+0x9f` up, a word
/// copied (`+0xaa` to `+0xc4`); unless the flag `+0xb8` is set, every
/// child of the root (`+0xcc`, the first child `+0xb0`) gets its method
/// `+0x20` with a cursor on the frame (`ebp - 8`; its pass byte never
/// written there: junk read, 0, brief 2026-10-24); with no child left
/// the done bit 0 of `+0x107` is set. The static nodes `+0x113` and
/// `+0x12e` are visited through [`visit_gated`](crate::nodes::visit_gated)
/// with the pass. Returns 0.
pub fn load_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let mut step = g.read::<u32>(w(0x04));
    loop {
        match step {
            0 | 1 => {
                let s = if step == 0 {
                    table(g, 0x26, 225)
                } else {
                    table(g, 0x19, 313)
                };
                if g.call(s, 0, &[2]).eax != 0 {
                    return Ret::eax(0);
                }
                step = g.read::<u32>(w(0x40)).wrapping_add(1);
                g.write::<u32>(w(0x46), step);
            }
            2 => {
                let s = table(g, 0x50, 549);
                if g.call(s, 0, &[2]).eax != 0 {
                    return Ret::eax(0);
                }
                let file = g.read::<u32>(w(0x71));
                let s = table(g, 0x6b, 226);
                g.call(s, 0, &[file, 0]);
                g.write::<u32>(w(0x82), w(0x86));
                break;
            }
            _ => break,
        }
    }
    let s = table(g, 0x8b, 673);
    g.call(s, 0, &[]);
    let p = g.read::<u32>(w(0x99));
    let mut pass = g.read::<u8>(w(0x9f)).wrapping_add(1);
    g.write::<u32>(p, w(0xa5));
    let x = g.read::<u32>(w(0xaa));
    let mut child = g.read::<u32>(w(0xb0));
    let skip = g.read::<u32>(w(0xb8)) != 0;
    g.write::<u8>(w(0xbf), pass);
    g.write::<u32>(w(0xc4), x);
    // push ebp; sub esp, 8: the cursor at [ebp - 8]
    let ebp = fr.esp.wrapping_sub(4);
    let cursor = ebp.wrapping_sub(8);
    let done = if skip {
        child == w(0xfe)
    } else if child == w(0xcc) {
        true
    } else {
        // junk read: [ebp - 4] never written; 0 (brief 2026-10-24)
        g.write::<u32>(cursor + 4, 0);
        loop {
            g.write::<u32>(cursor, child);
            let m = g.read::<u32>(g.read::<u32>(child) + 0x20);
            child = g.read::<u32>(child + 8);
            // push ecx: the cursor's address, then the call
            g.write::<u32>(ebp.wrapping_sub(0x10), cursor);
            g.call(m, cursor, &[cursor]);
            if child == w(0xea) {
                break;
            }
        }
        pass = g.read::<u8>(w(0xf2));
        g.read::<u32>(w(0xf8)) == w(0xfe)
    };
    if done {
        let v = g.read::<u32>(w(0x107));
        g.write::<u32>(w(0x107), v | 1);
    }
    // pop esi; push eax (twice, the first not popped): the cursor's
    // address at [ebp - 0xc], then at [ebp - 0x10]
    for (k, (node_at, call_at, pass_at)) in
        (0u32..).zip([(0x113u32, 0x11bu32, None), (0x12e, 0x133, Some(0x120u32))])
    {
        if let Some(at) = pass_at {
            pass = g.read::<u8>(w(at));
        }
        g.write::<u8>(cursor + 5, pass);
        g.write::<u32>(cursor, w(node_at));
        g.write::<u32>(ebp.wrapping_sub(0xc + 4 * k), cursor);
        let fr = Frame {
            at: img.rel32_target(f + call_at),
            esp: ebp.wrapping_sub(0x10 + 4 * k),
            ..*fr
        };
        crate::nodes::visit_gated(g, img, &fr);
    }
    Ret::eax(0)
}

/// [`load_frame`] (320 bytes, 1 overlay).
pub const LOAD_FRAME: Sig = Sig {
    len: 320,
    hash: 0x8d80_e85a_222e_b6c8,
    calls: &[
        (0x11b, Callee::Sig(&crate::nodes::GATED)),
        (0x133, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};

/// The model start-up (0x1000d070, no arguments, a frame of 0x10 bytes):
/// a node put at the ring's tail (`+0x04`..`+0x38`), the root's flag
/// cleared (`+0x43`), the static node `N` (`+0x5f`..) appended to the root
/// list (count `+0xb4`) with its words set (`+0xc0`..`+0x13e`); slot 294
/// `(*A)` (`+0xba`) gives a model number (`0xff`: 1, else slot 294
/// asked again) for the first model node (`+0x191`, [`model_node_init`]
/// with a record `[node, parent, number, 0]` on the frame), and the byte
/// `*B` (`+0x1a1`) the second's (`+0x1bf`). Returns the last
/// `model_node_init`'s `eax`.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn model_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let tail = g.read::<u32>(w(0x04));
    g.write::<u32>(w(0x09), tail);
    stores(
        g,
        img,
        f,
        &[(0x0f, 0x13), (0x19, 0x1d), (0x23, 0x27), (0x2d, 0x31)],
    );
    g.write::<u32>(tail + 4, w(0x38));
    let last = g.read::<u32>(w(0x3d));
    let v = g.read::<u32>(w(0x43));
    g.write::<u32>(w(0x43), v & !1);
    stores(g, img, f, &[(0x4d, 0x51)]);
    g.write::<u8>(w(0x57), 0);
    g.write::<u16>(w(0x5f), 0xff);
    stores(
        g,
        img,
        f,
        &[(0x67, 0x6b), (0x71, 0x75), (0x7b, 0x7f), (0x85, 0x89)],
    );
    g.write::<u32>(w(0x8e), last);
    if last == w(0x93) {
        stores(g, img, f, &[(0x9b, 0x9f)]);
    } else {
        g.write::<u32>(last + 4, w(0xa8));
    }
    let table = g.read::<u32>(w(0xad));
    let c = g.read::<u16>(w(0xb4));
    g.write::<u16>(w(0xb4), c.wrapping_add(1));
    let a = g.read::<u32>(w(0xba));
    stores(g, img, f, &[(0xc0, 0xc4), (0xca, 0xce), (0xd4, 0xd8)]);
    g.write::<u32>(w(0xde), 0);
    g.write::<u32>(w(0xe8), 0);
    g.write::<u16>(w(0xf3), 0x100);
    g.write::<u8>(w(0xfb), 0);
    stores(g, img, f, &[(0x102, 0x106)]);
    for at in [0x10c, 0x116, 0x120] {
        g.write::<u32>(w(at), 0);
    }
    // mov dword [addr], imm32: the immediate 4 bytes after the address
    for at in [0x12a, 0x134, 0x13e] {
        g.write::<u32>(w(at), w(at + 4));
    }
    let s = g.read::<u32>(table.wrapping_add(0x498));
    let n = g.call(s, 0, &[a]).eax;
    // push ebp; sub esp, 0x10: the record at [ebp - 0x10]
    let rec = fr.esp.wrapping_sub(0x14);
    g.write::<u32>(rec, w(0x154));
    g.write::<u32>(rec + 4, w(0x15b));
    let number = if n == 0xff {
        1
    } else {
        let a = g.read::<u32>(w(0x176));
        let s = slot_target(g, w(0x170), 294);
        g.call(s, 0, &[a]).eax
    };
    g.write::<u32>(rec + 8, number);
    g.write::<u8>(rec + 0xc, 0);
    stores(g, img, f, &[(0x191, 0x195)]);
    model_node_init(g, img, img.rel32_target(f + 0x19a), rec);
    let b = u32::from(g.read::<u8>(w(0x1a1)));
    g.write::<u32>(rec, w(0x1ab));
    g.write::<u32>(rec + 4, w(0x1b2));
    g.write::<u32>(rec + 8, b);
    g.write::<u8>(rec + 0xc, 0);
    stores(g, img, f, &[(0x1bf, 0x1c3)]);
    Ret::eax(model_node_init(g, img, img.rel32_target(f + 0x1c8), rec))
}

/// [`model_start`] (464 bytes, 1 overlay).
pub const MODEL_START: Sig = Sig {
    len: 464,
    hash: 0xc6cf_b4a3_3cae_39bb,
    calls: &[
        (0x19a, Callee::Sig(&MODEL_NODE_INIT)),
        (0x1c8, Callee::Sig(&MODEL_NODE_INIT)),
    ],
    imports: &[],
};
