//! The camera start-ups of the `magic_0078` family: `magic_0089`,
//! `0090`, `0091`, `0094`, `0095` and `0096` (one each, 929 to 1,240
//! bytes). Each is a start-up of [`crate::m0078`] (the head, two static
//! nodes and `node insert`, then the particle node and effect nodes)
//! that ends like `m0078::start_0083`: a colour node and the camera move
//! set up inline. They differ in the head, in how many effect nodes they
//! place and in the camera block's constants, so each has its own table
//! of operand offsets here.

use crate::m0078::{
    EachNode, TWO_0080, TwoNodes, each_head, each_node, kind_head, start_head, stores, two_nodes,
};
use crate::runtime::slot_target;
use crate::sb_nodes::{EFFECT_NODE, PARTICLE_NODE, effect_node, particle_node};
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The particle node's record (on the frame at `ebp - 0x18`): words 0..2
/// (operands), the pointer global `*A` it reads first, word 4 (operand),
/// word 5 (the immediate at `index`, or the index the head chose), the
/// node's vtable store and the `rel32`.
struct Part {
    words: [u32; 3],
    a: u32,
    fifth: u32,
    index: Option<u32>,
    vt: (u32, u32),
    call: u32,
}

/// An effect node: its record's five words (operands) and sixth (the
/// immediate at `index`), the `rel32`, the vtable written into the node
/// it returns, and the flag words it sets (bit 0 of the first, the second
/// cleared) when the start-up does.
struct Fx {
    words: [u32; 5],
    index: u32,
    call: u32,
    vt: u32,
    flags: Option<(u32, u32)>,
}

/// The colour node: its group `[node, parent]` on the frame at `ebp -
/// 8`, the node's vtable store and the `rel32`.
struct Colour {
    words: [u32; 2],
    vt: (u32, u32),
    call: u32,
}

/// A float the camera block stores: `K[i] - *src` (`K` the block's
/// constants) or `*a - *b`.
#[derive(Clone, Copy)]
enum D {
    K(usize, u32),
    M(u32, u32),
}

/// The camera block: the constants it reads (one or two operands), the
/// flags word that loses bit 0, the words set to their immediates
/// (`mov dword [addr], imm32`: the immediate 4 bytes after the address)
/// and the differences it stores (destination operand, value).
struct Camera {
    k: &'static [u32],
    flag: u32,
    imms: &'static [u32],
    diffs: &'static [(u32, D)],
}

/// The part after the head: the particle node, the effect nodes, the
/// colour node and the camera block.
struct Tail {
    part: Part,
    fx: &'static [Fx],
    colour: Colour,
    cam: Camera,
}

/// The particle node (record at `rec`; `index` when the record's sixth
/// word is not an immediate).
fn part_node<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32, index: u32, p: &Part) {
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(p.a));
    let index = p.index.map_or(index, w);
    for (k, v) in (0u32..).zip([
        w(p.words[0]),
        w(p.words[1]),
        w(p.words[2]),
        a,
        w(p.fifth),
        index,
    ]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    stores(g, img, f, &[p.vt]);
    particle_node(g, img, img.rel32_target(f + p.call), rec);
}

/// The effect nodes in order (record at `rec`).
fn fx_nodes<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32, fx: &[Fx]) {
    let w = |off: u32| img.word(f + off);
    for x in fx {
        for (k, &op) in (0u32..).zip(&x.words) {
            g.write::<u32>(rec + 4 * k, w(op));
        }
        g.write::<u32>(rec + 20, w(x.index));
        effect_node(g, img, img.rel32_target(f + x.call), rec);
        let node = g.read::<u32>(rec);
        g.write::<u32>(node, w(x.vt));
        if let Some((set, clear)) = x.flags {
            let v = g.read::<u32>(w(set));
            g.write::<u32>(w(set), v | 1);
            g.write::<u32>(w(clear), 0);
        }
    }
}

/// The colour node ([`crate::s3_misc::colour_node`]; `esp` its frame's,
/// at the return address). Returns its `eax`.
fn colour<G: Game>(g: &mut G, img: &Image, f: u32, ebp: u32, esp: u32, c: &Colour) -> u32 {
    let w = |off: u32| img.word(f + off);
    let grp = ebp.wrapping_sub(8);
    g.write::<u32>(grp, w(c.words[0]));
    g.write::<u32>(grp + 4, w(c.words[1]));
    stores(g, img, f, &[c.vt]);
    crate::s3_misc::colour_node(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + c.call),
            esp,
            ecx: grp,
            ..Frame::default()
        },
    )
}

/// The camera block (its words are distinct globals, so the stores'
/// order among themselves does not matter).
fn camera<G: Game>(g: &mut G, img: &Image, f: u32, c: &Camera) {
    let w = |off: u32| img.word(f + off);
    let mut k = [0f32; 2];
    for (v, &op) in k.iter_mut().zip(c.k) {
        *v = g.read::<f32>(w(op));
    }
    let fl = g.read::<u32>(w(c.flag));
    g.write::<u32>(w(c.flag), fl & !1);
    for &at in c.imms {
        g.write::<u32>(w(at), w(at + 4));
    }
    for &(dst, d) in c.diffs {
        let v = match d {
            D::K(i, src) => sub(k[i], g.read::<f32>(w(src))),
            D::M(a, b) => sub(g.read::<f32>(w(a)), g.read::<f32>(w(b))),
        };
        g.write::<u32>(w(dst), v.to_bits());
    }
}

/// [`Tail`] after a head that left the particle index `index` (`esp`
/// the colour node's frame). Returns the colour node's `eax`.
fn tail<G: Game>(g: &mut G, img: &Image, fr: &Frame, index: u32, esp: u32, t: &Tail) -> Ret {
    let f = fr.at;
    let ebp = fr.esp.wrapping_sub(4);
    let rec = ebp.wrapping_sub(0x18);
    part_node(g, img, f, rec, index, &t.part);
    fx_nodes(g, img, f, rec, t.fx);
    let eax = colour(g, img, f, ebp, esp, &t.colour);
    camera(g, img, f, &t.cam);
    Ret::eax(eax)
}

/// The particle record of the start-ups with `m0078::each_start`'s head.
const EACH_PART: Part = Part {
    words: [0x1fc, 0x203, 0x20a],
    a: 0x1f2,
    fifth: 0x214,
    index: Some(0x21b),
    vt: (0x221, 0x225),
    call: 0x22a,
};

/// Their first effect node.
const EACH_FX1: [u32; 5] = [0x234, 0x23b, 0x242, 0x249, 0x250];

/// `magic_0089` 0x10006860 (no arguments, a frame of 0x18 bytes):
/// `m0078::each_start`'s head, the particle node, one effect node, the
/// colour node and the camera block (one constant). Returns the colour
/// node's `eax`.
pub fn start_0089<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    each_head(g, img, fr);
    let t = Tail {
        part: EACH_PART,
        fx: &[Fx {
            words: EACH_FX1,
            index: 0x257,
            call: 0x25c,
            vt: 0x268,
            flags: Some((0x26e, 0x275)),
        }],
        colour: Colour {
            words: [0x280, 0x287],
            vt: (0x28d, 0x291),
            call: 0x296,
        },
        cam: Camera {
            k: &[0x29e],
            flag: 0x2af,
            imms: &[
                0x2b6, 0x2c0, 0x2ca, 0x2e7, 0x2f1, 0x2fb, 0x305, 0x322, 0x32c,
            ],
            diffs: &[
                (0x2d6, D::K(0, 0x2a9)),
                (0x311, D::K(0, 0x2e1)),
                (0x338, D::K(0, 0x31c)),
                (0x34b, D::K(0, 0x343)),
                (0x35e, D::K(0, 0x356)),
                (0x379, D::K(0, 0x369)),
                (0x391, D::K(0, 0x371)),
                (0x399, D::M(0x381, 0x389)),
            ],
        },
    };
    tail(g, img, fr, 0, fr.esp.wrapping_sub(0x20), &t)
}

/// `magic_0090` 0x10006890: as [`start_0089`] with a second effect node
/// (without flag words) and two camera constants. Returns the colour
/// node's `eax`.
pub fn start_0090<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    each_head(g, img, fr);
    let t = Tail {
        part: EACH_PART,
        fx: &[
            Fx {
                words: EACH_FX1,
                index: 0x257,
                call: 0x25c,
                vt: 0x268,
                flags: None,
            },
            Fx {
                words: [0x26f, 0x276, 0x27d, 0x284, 0x28b],
                index: 0x292,
                call: 0x297,
                vt: 0x2a3,
                flags: Some((0x2a9, 0x2b0)),
            },
        ],
        colour: Colour {
            words: [0x2bb, 0x2c2],
            vt: (0x2c8, 0x2cc),
            call: 0x2d1,
        },
        cam: Camera {
            k: &[0x2d9, 0x2e1],
            flag: 0x2fa,
            imms: &[
                0x301, 0x30b, 0x328, 0x332, 0x33c, 0x346, 0x363, 0x36d, 0x377,
            ],
            diffs: &[
                (0x317, D::K(0, 0x2ec)),
                (0x352, D::K(1, 0x322)),
                (0x383, D::K(0, 0x2f4)),
                (0x38b, D::K(1, 0x35d)),
                (0x39e, D::K(1, 0x396)),
                (0x3b1, D::K(1, 0x3a9)),
                (0x3d1, D::M(0x3b9, 0x3c1)),
                (0x3d9, D::K(1, 0x3c9)),
            ],
        },
    };
    tail(g, img, fr, 0, fr.esp.wrapping_sub(0x20), &t)
}

/// `magic_0091` 0x10006870: `m0078::start_0083` (the same nodes) with
/// two camera constants. Returns the colour node's `eax`.
pub fn start_0091<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    each_head(g, img, fr);
    let t = Tail {
        part: EACH_PART,
        fx: &[
            Fx {
                words: EACH_FX1,
                index: 0x257,
                call: 0x25c,
                vt: 0x268,
                flags: Some((0x26e, 0x275)),
            },
            Fx {
                words: [0x280, 0x287, 0x28e, 0x295, 0x29c],
                index: 0x2a3,
                call: 0x2a8,
                vt: 0x2b4,
                flags: Some((0x2ba, 0x2c1)),
            },
        ],
        colour: Colour {
            words: [0x2cc, 0x2d3],
            vt: (0x2d9, 0x2dd),
            call: 0x2e2,
        },
        cam: Camera {
            k: &[0x2ea, 0x2f2],
            flag: 0x30b,
            imms: &[
                0x312, 0x31c, 0x339, 0x343, 0x34d, 0x357, 0x374, 0x37e, 0x388,
            ],
            diffs: &[
                (0x328, D::K(0, 0x2fd)),
                (0x363, D::K(1, 0x333)),
                (0x394, D::K(0, 0x305)),
                (0x39c, D::K(1, 0x36e)),
                (0x3af, D::K(1, 0x3a7)),
                (0x3ca, D::K(1, 0x3ba)),
                (0x3e2, D::K(1, 0x3c2)),
                (0x3ea, D::M(0x3d2, 0x3da)),
            ],
        },
    };
    tail(g, img, fr, 0, fr.esp.wrapping_sub(0x20), &t)
}

/// `magic_0095` 0x10006ae0: `m0078::each_start`'s head, except that the
/// second static node's four words after its vtable are one 16-byte copy
/// of a constant (`movaps`, `+0x17f` to `+0x1a4`); then as `start_0083`
/// with other operands. Returns the colour node's `eax`.
#[allow(clippy::too_many_lines)]
pub fn start_0095<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    start_head(g, img, f, ebp, 0);
    each_node(
        g,
        img,
        f,
        &EachNode {
            last_r: 0x91,
            half: 0x98,
            st: [(0xa0, 0xa4), (0xaa, 0xae), (0xb4, 0xb8), (0xbe, 0xc2)],
            last_w1: 0xc7,
            sentinel: 0xcc,
            first: (0xd4, 0xd8),
            next: 0xe1,
            count: 0xed,
            last_w: (0x100, 0x104),
            vt: (0x10a, 0x10e),
        },
    );
    let s = slot_target(g, w(0xe6), 0x430 / 4);
    let m = g.read::<u32>(w(0xfa));
    g.call(s, 0, &[m, 0x11, w(0xf2)]);
    g.write::<u32>(w(0x124), 0x3f80_0000);
    let block: [u32; 4] = std::array::from_fn(|i| g.read::<u32>(w(0x17f) + 4 * i as u32));
    each_node(
        g,
        img,
        f,
        &EachNode {
            last_r: 0x11b,
            half: 0x12f,
            st: [
                (0x137, 0x13b),
                (0x141, 0x145),
                (0x14b, 0x14f),
                (0x155, 0x159),
            ],
            last_w1: 0x15e,
            sentinel: 0x163,
            first: (0x16b, 0x16f),
            next: 0x178,
            count: 0x186,
            last_w: (0x18f, 0x193),
            vt: (0x199, 0x19d),
        },
    );
    for (k, v) in (0u32..).zip(block) {
        g.write::<u32>(w(0x1a4) + 4 * k, v);
    }
    let rec = ebp.wrapping_sub(0x18);
    for (k, op) in (0u32..).zip([0x1ab, 0x1b2, 0x1b9, 0x1c0, 0x1c7, 0x1ce]) {
        g.write::<u32>(rec + 4 * k, w(op));
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x1d3),
            ecx: rec,
            ..Frame::default()
        },
    );
    let t = Tail {
        part: Part {
            words: [0x1e2, 0x1e9, 0x1f0],
            a: 0x1d8,
            fifth: 0x1fa,
            index: Some(0x201),
            vt: (0x207, 0x20b),
            call: 0x210,
        },
        fx: &[
            Fx {
                words: [0x21a, 0x221, 0x228, 0x22f, 0x236],
                index: 0x23d,
                call: 0x242,
                vt: 0x24e,
                flags: Some((0x254, 0x25b)),
            },
            Fx {
                words: [0x266, 0x26d, 0x274, 0x27b, 0x282],
                index: 0x289,
                call: 0x28e,
                vt: 0x29a,
                flags: Some((0x2a0, 0x2a7)),
            },
        ],
        colour: Colour {
            words: [0x2b2, 0x2b9],
            vt: (0x2bf, 0x2c3),
            call: 0x2c8,
        },
        cam: Camera {
            k: &[0x2d0],
            flag: 0x2e1,
            imms: &[
                0x2e8, 0x2f2, 0x2fc, 0x319, 0x323, 0x32d, 0x337, 0x354, 0x35e,
            ],
            diffs: &[
                (0x308, D::K(0, 0x2db)),
                (0x343, D::K(0, 0x313)),
                (0x36a, D::K(0, 0x34e)),
                (0x37d, D::K(0, 0x375)),
                (0x390, D::K(0, 0x388)),
                (0x3a3, D::K(0, 0x39b)),
                (0x3c3, D::M(0x3ab, 0x3b3)),
                (0x3cb, D::K(0, 0x3bb)),
            ],
        },
    };
    tail(g, img, fr, 0, fr.esp.wrapping_sub(0x20), &t)
}

/// `magic_0096` 0x10006870 (a frame of 0x1c bytes and `esi`):
/// `m0078::start_0080` up to the particle node (the kind's path chooses
/// the particle index), then two effect nodes, the colour node and the
/// camera block. Returns the colour node's `eax`.
pub fn start_0096<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let index = kind_head(g, img, fr);
    let t = Tail {
        part: Part {
            words: [0x2ab, 0x2b2, 0x2b9],
            a: 0x2a1,
            fifth: 0x2c3,
            index: None,
            vt: (0x2c9, 0x2cd),
            call: 0x2d2,
        },
        fx: &[
            Fx {
                words: [0x2dc, 0x2e3, 0x2ea, 0x2f1, 0x2f8],
                index: 0x2ff,
                call: 0x304,
                vt: 0x310,
                flags: Some((0x316, 0x31d)),
            },
            Fx {
                words: [0x328, 0x32f, 0x336, 0x33d, 0x344],
                index: 0x34b,
                call: 0x350,
                vt: 0x35c,
                flags: Some((0x362, 0x369)),
            },
        ],
        colour: Colour {
            words: [0x374, 0x37b],
            vt: (0x381, 0x385),
            call: 0x38a,
        },
        cam: Camera {
            k: &[0x392],
            flag: 0x3a3,
            imms: &[
                0x3aa, 0x3b4, 0x3be, 0x3db, 0x3e5, 0x3ef, 0x3f9, 0x416, 0x420,
            ],
            diffs: &[
                (0x3ca, D::K(0, 0x39d)),
                (0x405, D::K(0, 0x3d5)),
                (0x42d, D::K(0, 0x410)),
                (0x440, D::K(0, 0x438)),
                (0x453, D::K(0, 0x44b)),
                (0x46e, D::K(0, 0x45e)),
                (0x486, D::K(0, 0x466)),
                (0x48e, D::M(0x476, 0x47e)),
            ],
        },
    };
    // `push esi` under the 0x1c-byte frame
    tail(g, img, fr, index, fr.esp.wrapping_sub(0x28), &t)
}

/// `magic_0094` 0x10006b50 (a frame of 0x1c bytes and `esi`):
/// `m0078::start_0080`'s head and static nodes (no immediate word after
/// the three 1.0 words) and `node insert`, two effect nodes, then the
/// kind's path: slot 390's kind for the byte `*K` (`+0x24c`) through the
/// byte table (`+0x27f`) and jump table (`+0x286`; `+0x29f` the yes path
/// itself, every other entry and kind asks slot 534 `(*K)` first). Yes:
/// slot 268 `(*M, 0x11, X)`, `+0x2be` = 1.0, two words their immediates,
/// particle index 3; no: slot 268, `+0x307` = 1.0, slot 12 `(*M)`'s float
/// into `+0x31c` and that less `C` into `+0x352`, two words their
/// immediates, particle index 0. Then the particle node, the colour node
/// and the camera block. Returns the colour node's `eax`.
#[allow(clippy::too_many_lines)]
pub fn start_0094<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    start_head(g, img, f, ebp, 1);
    two_nodes(
        g,
        img,
        f,
        &TwoNodes {
            imm: None,
            ..TWO_0080
        },
    );
    let rec = ebp.wrapping_sub(0x18);
    for (k, op) in (0u32..).zip([0x196, 0x19d, 0x1a4, 0x1ab, 0x1b2, 0x1b9]) {
        g.write::<u32>(rec + 4 * k, w(op));
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x1be),
            ecx: rec,
            ..Frame::default()
        },
    );
    fx_nodes(
        g,
        img,
        f,
        rec,
        &[
            Fx {
                words: [0x1c8, 0x1cf, 0x1d6, 0x1dd, 0x1e4],
                index: 0x1eb,
                call: 0x1f0,
                vt: 0x1fc,
                flags: Some((0x202, 0x209)),
            },
            Fx {
                words: [0x214, 0x21b, 0x222, 0x229, 0x230],
                index: 0x237,
                call: 0x23c,
                vt: 0x245,
                flags: Some((0x257, 0x25e)),
            },
        ],
    );
    let key = u32::from(g.read::<u8>(w(0x24c)));
    let s = slot_target(g, w(0x251), 0x618 / 4);
    let k = g.call(s, 0, &[key]).eax.wrapping_sub(0x1071);
    let ask = if k <= 0x1b {
        // the byte table is in .text: read from the image
        let i = img.word(w(0x27f).wrapping_add(k)) & 0xff;
        img.word(w(0x286).wrapping_add(4 * i)).wrapping_sub(f) != 0x29f
    } else {
        true
    };
    let yes = !ask || {
        let s = slot_target(g, w(0x28b), 0x858 / 4);
        g.call(s, 0, &[key]).eax != 0
    };
    let index = if yes {
        let s = slot_target(g, w(0x2a0), 0x430 / 4);
        let m = g.read::<u32>(w(0x2b3));
        g.call(s, 0, &[m, 0x11, w(0x2a5)]);
        g.write::<u32>(w(0x2be), 0x3f80_0000);
        stores(g, img, f, &[(0x2c8, 0x2cc), (0x2d2, 0x2d6)]);
        3
    } else {
        let s = slot_target(g, w(0x2e4), 0x430 / 4);
        let m = g.read::<u32>(w(0x2f7));
        g.call(s, 0, &[m, 0x11, w(0x2e9)]);
        let table = g.read::<u32>(w(0x2fe));
        g.write::<u32>(w(0x307), 0x3f80_0000);
        let m = g.read::<u32>(w(0x311));
        let s = g.read::<u32>(table.wrapping_add(0x30));
        let v = g.call(s, 0, &[m]).st0.unwrap_or(0.0) as f32;
        g.write::<u32>(w(0x31c), v.to_bits());
        let y = sub(g.read::<f32>(w(0x324)), g.read::<f32>(w(0x32c)));
        stores(g, img, f, &[(0x335, 0x339), (0x33f, 0x343)]);
        g.write::<u32>(w(0x352), y.to_bits());
        0
    };
    let t = Tail {
        part: Part {
            words: [0x361, 0x368, 0x36f],
            a: 0x357,
            fifth: 0x379,
            index: None,
            vt: (0x37f, 0x383),
            call: 0x388,
        },
        fx: &[],
        colour: Colour {
            words: [0x392, 0x399],
            vt: (0x39f, 0x3a3),
            call: 0x3a8,
        },
        cam: Camera {
            k: &[0x3b0],
            flag: 0x3c1,
            imms: &[
                0x3c8, 0x3d2, 0x3dc, 0x3f9, 0x403, 0x40d, 0x417, 0x434, 0x43e,
            ],
            diffs: &[
                (0x3e8, D::K(0, 0x3bb)),
                (0x423, D::K(0, 0x3f3)),
                (0x44b, D::K(0, 0x42e)),
                (0x45e, D::K(0, 0x456)),
                (0x471, D::K(0, 0x469)),
                (0x48c, D::K(0, 0x47c)),
                (0x4a4, D::K(0, 0x484)),
                (0x4ac, D::M(0x494, 0x49c)),
            ],
        },
    };
    tail(g, img, fr, index, fr.esp.wrapping_sub(0x28), &t)
}

/// The callees every start-up here names: the model node, `node insert`,
/// the particle node, the effect nodes and the colour node (`rel32`
/// offsets per start-up).
macro_rules! calls {
    ($m:expr, $ins:expr, $p:expr, [$($fx:expr),*], $c:expr) => {
        &[
            ($m, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
            ($ins, Callee::Sig(&crate::nodes::INSERT)),
            ($p, Callee::Sig(&PARTICLE_NODE)),
            $(($fx, Callee::Sig(&EFFECT_NODE)),)*
            ($c, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        ]
    };
}

/// [`start_0089`] (929 bytes, 1 overlay).
pub const START_0089: Sig = Sig {
    len: 929,
    hash: 0x123d_83f1_8136_8c7d,
    calls: calls!(0x8c, 0x1ed, 0x22a, [0x25c], 0x296),
    imports: &[],
};

/// [`start_0090`] (993 bytes, 1 overlay).
pub const START_0090: Sig = Sig {
    len: 993,
    hash: 0x34a2_7e9b_e882_b423,
    calls: calls!(0x8c, 0x1ed, 0x22a, [0x25c, 0x297], 0x2d1),
    imports: &[],
};

/// [`start_0091`] (1,010 bytes, 1 overlay).
pub const START_0091: Sig = Sig {
    len: 1010,
    hash: 0x5572_804a_d3aa_6e1a,
    calls: calls!(0x8c, 0x1ed, 0x22a, [0x25c, 0x2a8], 0x2e2),
    imports: &[],
};

/// [`start_0095`] (979 bytes, 1 overlay).
pub const START_0095: Sig = Sig {
    len: 979,
    hash: 0x9d74_f244_b9a3_f481,
    calls: calls!(0x8c, 0x1d3, 0x210, [0x242, 0x28e], 0x2c8),
    imports: &[],
};

/// [`start_0096`] (1,161 bytes and its two jump tables, 1 overlay).
pub const START_0096: Sig = Sig {
    len: 1212,
    hash: 0x345f_683f_f25d_cc79,
    calls: calls!(0x8d, 0x1c8, 0x2d2, [0x304, 0x350], 0x38a),
    imports: &[],
};

/// [`start_0094`] (1,204 bytes and its two jump tables, 1 overlay).
pub const START_0094: Sig = Sig {
    len: 1240,
    hash: 0xb0fe_16a2_5776_c39b,
    calls: calls!(0x8d, 0x1be, 0x388, [0x1f0, 0x23c], 0x3a8),
    imports: &[],
};
