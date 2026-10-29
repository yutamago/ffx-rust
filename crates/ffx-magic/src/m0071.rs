//! `magic_0071.dll`'s own functions (besides the runtime and the shared
//! code): node set-ups and a spinning node's visit.

use crate::runtime::slot_target;
use crate::sb_sse::{add, mul};
use crate::{Frame, Game, Image, Ret, Sig};

/// Links `node` as the last child of `parent` (`+0x4`/`+0x8`, the
/// parent's first `+0xc` / last `+0x10`, the u16 count `+0x18`), reading
/// the parent's last child twice as the original does.
fn append<G: Game>(g: &mut G, node: u32, parent: u32) {
    g.write::<u32>(node + 4, parent);
    let last = g.read::<u32>(parent + 0x10);
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(parent + 0x10);
    if last == parent {
        g.write::<u32>(parent + 0xc, node);
    } else {
        g.write::<u32>(last + 4, node);
    }
    let n = g.read::<u16>(parent + 0x18).wrapping_add(1);
    g.write::<u16>(parent + 0x18, n);
    g.write::<u32>(parent + 0x10, node);
}

/// 0x100013a0 (internal; `ecx` = `[node, parent, out, out2, v]`): `out` =
/// `node + 0x5c`, `out2` = 0x400080; an empty child list with pass byte
/// 0xff under the parent (when not 0); `+0x20` = `node + 0x5c`, `+0x1c`
/// = 0x400080, `+0x30` (u16) and `+0x32` 0; slot 325 `(node + 0x38, v)`;
/// `+0x50`, `+0x58` 0. Returns slot 325's result.
pub fn node_init_vec<G: Game>(g: &mut G, img: &Image, f: u32, args: u32) -> u32 {
    let parent = g.read::<u32>(args + 4);
    let node = g.read::<u32>(args);
    let tail = node + 0x5c;
    g.write::<u32>(args + 8, tail);
    g.write::<u32>(args + 0xc, 0x0040_0080);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        append(g, node, parent);
    }
    let v = g.read::<u32>(args + 0x10);
    g.write::<u32>(node + 0x20, tail);
    g.write::<u32>(node + 0x1c, 0x0040_0080);
    g.write::<u32>(node + 0x32, 0);
    g.write::<u16>(node + 0x30, 0);
    let s325 = slot_target(g, img.word(f + 0x64), 325);
    let r = g.call(s325, parent, &[node + 0x38, v]).eax;
    g.write::<u32>(node + 0x50, 0);
    g.write::<u32>(node + 0x58, 0);
    r
}

/// `node_init_vec` (133 bytes, 1 overlay).
pub const NODE_INIT_VEC: Sig = Sig {
    len: 133,
    hash: 0x808f_bb3a_79e6_30de,
    calls: &[],
    imports: &[],
};

/// The visit of a spinning node (0x10003f00; `ecx` = the group, reached
/// through the thunk at 0x10004080): the node walk
/// ([`crate::nodes::walk`]); when it ran, a walk of the owner's `+0x2c`
/// node's children with a group of its own (a local at `ebp - 8`, the
/// mode the outer group's), then the spin: angle `+0x30 += +0x34`, the
/// speed `+0x34 *= +0x38`, the angle reset to 1.0 once its square
/// reaches `LIMIT`, and the position `+0x1c..+0x24` from the `+0x2c`
/// node's centre `+0x1c..` and its two axes `+0x2c..`, `+0x3c..` turned by
/// the angle (sine table `*G_SIN`, index `angle * K` truncated, the
/// cosine 0x4000 on). Returns `eax` as the original leaves it.
pub fn spin_visit<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, eax: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let (eax, walked) = crate::nodes::walk(g, this, this, eax);
    if walked.is_none() {
        return eax;
    }
    let owner = g.read::<u32>(this);
    let mode = g.read::<u8>(this + 5);
    let inner = g.read::<u32>(owner + 0x2c);
    // the local group {node, flag, mode} at ebp - 8
    let group = esp - 0xc;
    g.write::<u32>(group, inner);
    g.write::<u8>(group + 5, mode);
    let _ = crate::nodes::walk(g, group, group, eax);
    let a = add(g.read::<f32>(owner + 0x30), g.read::<f32>(owner + 0x34));
    g.write::<f32>(owner + 0x30, a);
    let s = mul(g.read::<f32>(owner + 0x38), g.read::<f32>(owner + 0x34));
    g.write::<f32>(owner + 0x34, s);
    let a = g.read::<f32>(owner + 0x30);
    // comiss + jb: below (or unordered) keeps the angle
    if mul(a, a) >= g.read::<f32>(w(0xc1)) {
        g.write::<u32>(owner + 0x30, 0x3f80_0000);
    }
    let x = mul(g.read::<f32>(owner + 0x30), g.read::<f32>(w(0xd7)));
    let n = ffx_bridge::x87::cvttsd2si(f64::from(x)).cast_signed();
    let src = g.read::<u32>(owner + 0x2c);
    let table = g.read::<u32>(w(0xe0));
    let ic = ((n.wrapping_add(0x4000) >> 4) & 0xfff).cast_unsigned();
    let is = ((n >> 4) & 0xfff).cast_unsigned();
    let c = g.read::<f32>(table.wrapping_add(4 * ic));
    let s = g.read::<f32>(table.wrapping_add(4 * is));
    for k in 0..3u32 {
        let v = add(
            mul(g.read::<f32>(src + 0x3c + 4 * k), s),
            mul(g.read::<f32>(src + 0x2c + 4 * k), c),
        );
        let v = add(v, g.read::<f32>(src + 0x1c + 4 * k));
        g.write::<f32>(owner + 0x1c + 4 * k, v);
    }
    g.write::<u32>(owner + 0x28, 0x3f80_0000);
    ic
}

/// [`spin_visit`] called directly (`ecx` = the group).
pub fn e3f00<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(spin_visit(g, img, fr.at, fr.ecx, fr.eax, fr.esp))
}

/// 0x10003f00 (375 bytes, 1 overlay).
pub const E3F00: Sig = Sig {
    len: 375,
    hash: 0x4191_3295_8dd7_c608,
    calls: &[],
    imports: &[],
};

/// 0x10007570 (internal; no arguments): the static node `NODE` (operand
/// `+0x14`) as an empty list with pass byte 0xff, appended to `ROOT`
/// (`+0x28`; its last child `+0x01`, first `+0x44`, count `+0x58`),
/// scale `+0x38` 1.0, `+0x3c` = `P` (`+0x76`) and its handler table
/// (`+0x80`) last. Returns the old last child.
pub fn static_node<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(0x01));
    g.write::<u16>(w(0x08), 0xff);
    for (a, v) in [(0x10u32, 0x14u32), (0x1a, 0x1e), (0x24, 0x28), (0x2e, 0x32)] {
        g.write::<u32>(w(a), w(v));
    }
    g.write::<u32>(w(0x37), last);
    if last == w(0x3c) {
        g.write::<u32>(w(0x44), w(0x48));
    } else {
        g.write::<u32>(last + 4, w(0x51));
    }
    let n = g.read::<u16>(w(0x58)).wrapping_add(1);
    g.write::<u16>(w(0x58), n);
    for (a, v) in [(0x5eu32, 0x62u32), (0x68, 0x6c), (0x72, 0x76), (0x7c, 0x80)] {
        g.write::<u32>(w(a), w(v));
    }
    last
}

/// `static_node` (133 bytes, 1 overlay).
pub const STATIC_NODE: Sig = Sig {
    len: 133,
    hash: 0xc2e7_b6cd_f4a3_97fb,
    calls: &[],
    imports: &[],
};

/// 0x10007670 (internal; `ecx` = the node): its handler table (operand
/// `+0x02`), an empty child list with pass byte 0xff, appended to `ROOT`
/// (`+0x15`; last child `+0x21`, first `+0x36`, count `+0x42`); `+0x2c`
/// = `P` (`+0x4f`), `+0x30..+0x38` 0. Returns the old last child.
pub fn root_node<G: Game>(g: &mut G, img: &Image, f: u32, node: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(node, w(0x02));
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, w(0x15));
    g.write::<u32>(node + 4, w(0x1c));
    let last = g.read::<u32>(w(0x21));
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(w(0x29));
    if last == w(0x2e) {
        g.write::<u32>(w(0x36), node);
    } else {
        g.write::<u32>(last + 4, node);
    }
    let n = g.read::<u16>(w(0x42)).wrapping_add(1);
    g.write::<u16>(w(0x42), n);
    g.write::<u32>(w(0x48), node);
    g.write::<u32>(node + 0x2c, w(0x4f));
    for o in [0x30u32, 0x34, 0x38] {
        g.write::<u32>(node + o, 0);
    }
    last
}

/// `root_node` (105 bytes, 1 overlay).
pub const ROOT_NODE: Sig = Sig {
    len: 105,
    hash: 0x746d_a942_f0f1_21c8,
    calls: &[],
    imports: &[],
};
