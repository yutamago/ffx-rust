//! `magic_0070.dll`'s functions shared with other overlays (`0071`,
//! `0106`, `0150`, `0155`, `0164`, `0186`, `0387`, `0388`, `0445`,
//! `0570`, `0577`, `0578`, `0597`, `0603`, `0609`, `0613`): more shapes
//! of the effect node set-up ([`crate::m0003::node_init`]) and a part
//! release. Internal functions with register arguments; addresses are
//! `magic_0070`'s.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Mem, Ret, Sig};

/// Appends `node` to `parent`'s children (links `+0x4`/`+0x8`, the
/// parent's first `+0xc` and last `+0x10` child, u16 count `+0x18`).
/// `p` holds the addresses of the parent's `+0x10`, `+0xc` and `+0x18`
/// (each read from its own instruction where the parent is a global).
fn link(g: &mut impl Mem, node: u32, parent: u32, p: [u32; 3]) {
    let [last_at, first_at, count_at] = p;
    g.write::<u32>(node + 4, parent);
    let last = g.read::<u32>(last_at);
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(last_at);
    if last == parent {
        g.write::<u32>(first_at, node);
    } else {
        g.write::<u32>(last + 4, node);
    }
    let count = g.read::<u16>(count_at);
    g.write::<u16>(count_at, count.wrapping_add(1));
    g.write::<u32>(last_at, node);
}

/// The common start: pass byte `+0x1a` 0xff (as a u16), an empty child
/// list (`+0x10`, `+0xc` the node itself).
fn fresh(g: &mut impl Mem, node: u32) {
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
}

/// 0x10003150 (internal; `ecx` = `[node, parent, a, b, c]`): a fresh
/// node under `parent` (when not 0), `+0x13c` = a, `+0x144` = b,
/// `+0x140` = c, `+0x128` 0, byte `+0x148` 0. Returns c.
pub fn node_init_big(g: &mut impl Mem, args: u32) -> u32 {
    let node = g.read::<u32>(args);
    let parent = g.read::<u32>(args + 4);
    fresh(g, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        link(
            g,
            node,
            parent,
            [parent + 0x10, parent + 0xc, parent + 0x18],
        );
    }
    let a = g.read::<u32>(args + 8);
    g.write::<u32>(node + 0x13c, a);
    let b = g.read::<u32>(args + 0xc);
    g.write::<u32>(node + 0x144, b);
    let c = g.read::<u32>(args + 0x10);
    g.write::<u32>(node + 0x140, c);
    g.write::<u32>(node + 0x128, 0);
    g.write::<u8>(node + 0x148, 0);
    c
}

/// `node_init_big` (102 bytes; 9 overlays).
pub const NODE_INIT_BIG: Sig = Sig {
    len: 102,
    hash: 0x040a_3c86_3b25_f891,
    calls: &[],
    imports: &[],
};

/// 0x100098e0 (internal; `ecx` = the node): a fresh node with its handler
/// table, under the overlay's root node (a global), `+0x4c..+0x54` 0 and
/// `+0x58` 0x480000. Returns the root's old last child.
pub fn node_init_root(g: &mut impl Mem, img: &Image, f: u32, node: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(node, w(0x02));
    fresh(g, node);
    g.write::<u32>(node + 0x14, w(0x15));
    let root = w(0x1c);
    // the root's +0x10 is read at 0x21 and 0x29 and written at 0x48 (one
    // global); +0xc at 0x36; +0x18 at 0x42
    let last = {
        g.write::<u32>(node + 4, root);
        let last = g.read::<u32>(w(0x21));
        g.write::<u32>(node + 8, last);
        let last = g.read::<u32>(w(0x29));
        if last == w(0x2e) {
            g.write::<u32>(w(0x36), node);
        } else {
            g.write::<u32>(last + 4, node);
        }
        let count = g.read::<u16>(w(0x42));
        g.write::<u16>(w(0x42), count.wrapping_add(1));
        g.write::<u32>(w(0x48), node);
        last
    };
    for off in [0x4c, 0x50, 0x54] {
        g.write::<u32>(node + off, 0);
    }
    g.write::<u32>(node + 0x58, 0x0048_0000);
    last
}

/// `node_init_root` (105 bytes; 8 overlays).
pub const NODE_INIT_ROOT: Sig = Sig {
    len: 105,
    hash: 0x7e4b_4cf5_842a_e8e0,
    calls: &[],
    imports: &[],
};

/// 0x100099f0 (internal; `ecx` = the node, `edx` = the parent, one stack
/// word `a`, cdecl): a fresh node under the parent (when not 0), scale
/// `+0x38` 1.0, `+0x3c` = a, then its handler table. Returns a.
pub fn node_init_small(
    g: &mut impl Mem,
    img: &Image,
    f: u32,
    node: u32,
    parent: u32,
    a: u32,
) -> u32 {
    fresh(g, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        link(
            g,
            node,
            parent,
            [parent + 0x10, parent + 0xc, parent + 0x18],
        );
    }
    g.write::<u32>(node + 0x38, 1.0f32.to_bits());
    g.write::<u32>(node + 0x3c, a);
    g.write::<u32>(node, img.word(f + 0x44));
    a
}

/// `node_init_small` (74 bytes; 6 overlays).
pub const NODE_INIT_SMALL: Sig = Sig {
    len: 74,
    hash: 0xa225_fb14_5c0e_727e,
    calls: &[],
    imports: &[],
};

/// 0x10009a80 (internal; `ecx` = a part, `eax` the caller's): unless bit
/// 2 of byte `+0xe8` is set, the particle environment pointer `*G_ENVP`
/// gets the part's `+0x50` and `pppAllFreePObject(part + 0x60)` (slot
/// 713) runs. Returns slot 713's `eax`, or the caller's.
pub fn free_part<G: Game>(g: &mut G, img: &Image, f: u32, part: u32, eax: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    if g.read::<u8>(part + 0xe8) & 4 != 0 {
        return eax;
    }
    let envp = g.read::<u32>(w(0x0a));
    let env = g.read::<u32>(part + 0x50);
    g.write::<u32>(envp, env);
    let s713 = slot_target(g, w(0x18), 713);
    g.call(s713, 0, &[part + 0x60]).eax
}

/// `free_part` (38 bytes; 8 overlays).
pub const FREE_PART: Sig = Sig {
    len: 38,
    hash: 0x0c9e_4b9c_1bfa_dfa3,
    calls: &[],
    imports: &[],
};

/// `magic_0071` 0x10002fe0: [`node_init_big`] that stores the handler
/// table last (the instruction at `+0x64`). Returns c.
pub fn node_init_big_table(g: &mut impl Mem, img: &Image, f: u32, args: u32) -> u32 {
    let c = node_init_big(g, args);
    let node = g.read::<u32>(args);
    g.write::<u32>(node, img.word(f + 0x66));
    // mov [edx], imm: eax still holds c
    c
}

/// `node_init_big_table` (108 bytes; 9 overlays).
pub const NODE_INIT_BIG_TABLE: Sig = Sig {
    len: 108,
    hash: 0x3d9b_7834_452e_60bc,
    calls: &[],
    imports: &[],
};

/// A float the game returned in ST0, stored with `fstp dword` (no float:
/// the x87's empty-stack store, the indefinite).
fn st0_f32(out: crate::Out) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // fstp dword: round to f32
    out.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32)
}

/// 0x100046e0 (internal; `ecx` = `[effect, out]`): the effect model's
/// state into `out` (4 floats): with bit 0 of u16 `+0x90`, slots 15, 16,
/// 17 of the model `+0x54` (ST0 each; the earlier words stay on the
/// stack) into `out[0..3]`; else slot 290 (`+0x50`, out) fills them.
/// Then `out[3]` = slot 277 (model) * `K`. Returns `out`.
pub fn model_state<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let obj = g.read::<u32>(this);
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        let mut pushed = Vec::new();
        for (k, (t, slot)) in (0u32..).zip([(0x05, 15), (0x30, 16), (0x4d, 17)]) {
            let s = slot_target(g, w(t), slot);
            let m = g.read::<u32>(obj + 0x54);
            let mut args = vec![m];
            args.extend(pushed.iter().rev());
            // call width: 3 (the model and the two earlier pushes at most)
            let v = st0_f32(g.call(s, 0, &args));
            pushed.push(m);
            let out = g.read::<u32>(this + 4);
            g.write::<u32>(out + 4 * k, v.to_bits());
        }
    } else {
        let s = slot_target(g, w(0x05), 290);
        let out = g.read::<u32>(this + 4);
        let m = g.read::<u32>(obj + 0x50);
        g.call(s, 0, &[m, out]);
    }
    let s = slot_target(g, w(0x80), 277);
    let m = g.read::<u32>(obj + 0x54);
    let v = st0_f32(g.call(s, 0, &[m])) * f32::from_bits(g.read::<u32>(w(0x9b)));
    let out = g.read::<u32>(this + 4);
    g.write::<u32>(out + 0xc, v.to_bits());
    out
}

/// `model_state` (176 bytes; 11 overlays).
pub const MODEL_STATE: Sig = Sig {
    len: 176,
    hash: 0x1415_d215_435f_fce1,
    calls: &[],
    imports: &[],
};

/// 0x10004790 (internal; `ecx` = `[effect, byte on]`): shows or hides
/// the effect: slot 276 (model `+0x54`, on); without bit 0 of u16
/// `+0x90`, the two parts slot 219 (`+0x50`, 0 and 1) finds get slot 276
/// (part, on) too. On: slot 548 (model) and bit 11 set (returns 0x800);
/// off: bit 11 cleared (returns 0xf7ff).
pub fn show<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let obj = g.read::<u32>(this);
    let on = u32::from(g.read::<u8>(this + 4));
    let s276 = slot_target(g, w(0x0c), 276);
    let m = g.read::<u32>(obj + 0x54);
    g.call(s276, 0, &[m, on]);
    if g.read::<u8>(obj + 0x90) & 1 == 0 {
        for (i, (t, t2)) in (0u32..).zip([(0x28, 0x47), (0x57, 0x76)]) {
            let s219 = slot_target(g, w(t), 219);
            let parts = g.read::<u32>(obj + 0x50);
            let part = g.call(s219, 0, &[parts, i]).eax;
            if part == 0 {
                if i == 1 {
                    break;
                }
                continue;
            }
            let on = u32::from(g.read::<u8>(this + 4));
            let s276 = slot_target(g, w(t2), 276);
            g.call(s276, 0, &[part, on]);
        }
    }
    if g.read::<u8>(this + 4) != 0 {
        let s548 = slot_target(g, w(0x8c), 548);
        let m = g.read::<u32>(obj + 0x54);
        g.call(s548, 0, &[m]);
        let bits = g.read::<u16>(obj + 0x90) | 0x800;
        g.write::<u16>(obj + 0x90, bits);
        0x800
    } else {
        let bits = g.read::<u16>(obj + 0x90) & 0xf7ff;
        g.write::<u16>(obj + 0x90, bits);
        0xf7ff
    }
}

/// `show` (188 bytes; 7 overlays).
pub const SHOW: Sig = Sig {
    len: 188,
    hash: 0x10d5_b7c8_1a10_4282,
    calls: &[],
    imports: &[],
};

/// 0x10001740 (internal; `ecx` = a request: `+0x0` the ring, u16 `+0x4`
/// the count wanted; it gets u16 `+0x6` and `+0x8`, the two pieces'
/// lengths, `+0xc` the first piece's address and `+0x10` the second's):
/// takes `count` entries from a ring of u16 `+0x1c` entries of u16
/// `+0x1e` bytes at `+0x20`, starting at the head u16 `+0x30`, wrapping
/// at the end (then two pieces). Then the ring's filled span: u16
/// `+0x32` grows by the count up to the capacity (once full, the start
/// u16 `+0x34` follows the head) and is described as one or two pieces
/// (`+0x24` its start address, u16 `+0x2c`/`+0x2e` the lengths, `+0x28`
/// the address after the wrap). A request larger than the ring gets
/// nothing (`+0x6` = 0, returns 0). The arithmetic is the original's
/// 16-bit (lengths) and 32-bit (addresses). Returns the `eax` the
/// original leaves: the span's length, or the ring's base when the span
/// wraps.
pub fn ring_take(g: &mut impl Mem, req: u32) -> u32 {
    let ring = g.read::<u32>(req);
    let n = g.read::<u16>(req + 4);
    let cap = g.read::<u16>(ring + 0x1c);
    if n > cap {
        g.write::<u16>(req + 6, 0);
        return 0;
    }
    let head = g.read::<u16>(ring + 0x30);
    let end = n.wrapping_add(head);
    let stride = u32::from(g.read::<u16>(ring + 0x1e));
    let base = g.read::<u32>(ring + 0x20);
    g.write::<u32>(req + 0xc, (u32::from(head) * stride).wrapping_add(base));
    if end > cap {
        let head = g.read::<u16>(ring + 0x30);
        g.write::<u32>(req + 0x10, base);
        let first = cap.wrapping_sub(head);
        g.write::<u16>(req + 6, first);
        let rest = g.read::<u16>(req + 4).wrapping_sub(first);
        g.write::<u16>(req + 8, rest);
        g.write::<u16>(ring + 0x30, end.wrapping_sub(cap));
    } else {
        let n = g.read::<u16>(req + 4);
        g.write::<u16>(req + 6, n);
        g.write::<u16>(req + 8, 0);
        g.write::<u16>(ring + 0x30, if end == cap { 0 } else { end });
    }
    let used = g.read::<u16>(ring + 0x32);
    if used < cap {
        let used = g.read::<u16>(req + 4).wrapping_add(used);
        g.write::<u16>(ring + 0x32, used);
        if used > cap {
            g.write::<u16>(ring + 0x32, cap);
        }
    } else {
        let head = g.read::<u16>(ring + 0x30);
        g.write::<u16>(ring + 0x34, head);
    }
    let stride = u32::from(g.read::<u16>(ring + 0x1e));
    let start = g.read::<u16>(ring + 0x34);
    let at = (u32::from(start) * stride).wrapping_add(g.read::<u32>(ring + 0x20));
    g.write::<u32>(ring + 0x24, at);
    let start = g.read::<u16>(ring + 0x34);
    let used = g.read::<u16>(ring + 0x32);
    let cap = g.read::<u16>(ring + 0x1c);
    if start.wrapping_add(used) > cap {
        let first = cap.wrapping_sub(start);
        g.write::<u16>(ring + 0x2c, first);
        let rest = g.read::<u16>(ring + 0x32).wrapping_sub(first);
        g.write::<u16>(ring + 0x2e, rest);
        let base = g.read::<u32>(ring + 0x20);
        g.write::<u32>(ring + 0x28, base);
        base
    } else {
        g.write::<u16>(ring + 0x2c, used);
        g.write::<u16>(ring + 0x2e, 0);
        let stride = u32::from(g.read::<u16>(ring + 0x1e));
        let used = u32::from(g.read::<u16>(ring + 0x32));
        let after = (stride * used).wrapping_add(g.read::<u32>(ring + 0x20));
        g.write::<u32>(ring + 0x28, after);
        used
    }
}

/// `ring_take` (250 bytes; 7 overlays).
pub const RING_TAKE: Sig = Sig {
    len: 250,
    hash: 0x5517_d44f_6321_c4a4,
    calls: &[],
    imports: &[],
};

/// 0x10009500 and 0x10009590 (internal; `ecx` = an effect object, `edx`
/// = a value, `eax` the caller's): unless bit 0 of byte `+0x90` is set,
/// `slot` (`+0x50`, value): slot 255 at 0x9500, 531 at 0x9590. Returns
/// the slot's `eax`, or the caller's.
pub fn part_call<G: Game>(g: &mut G, img: &Image, f: u32, slot: u32, regs: [u32; 3]) -> u32 {
    let [eax, obj, value] = regs;
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        return eax;
    }
    let s = slot_target(g, img.word(f + 0x0a), slot);
    let part = g.read::<u32>(obj + 0x50);
    g.call(s, 0, &[part, value]).eax
}

/// `part_call` with slot 255 (30 bytes; 3 overlays).
pub const PART_CALL_255: Sig = Sig {
    len: 30,
    hash: 0xe9b3_e75c_b8aa_9d7c,
    calls: &[],
    imports: &[],
};

/// `part_call` with slot 531 (30 bytes; 3 overlays).
pub const PART_CALL_531: Sig = Sig {
    len: 30,
    hash: 0xc5da_fefa_571f_866d,
    calls: &[],
    imports: &[],
};

/// 0x100095b0 (internal; `edx` = a vector of 3 floats): slot 262
/// (`G_MODEL`, &v[0], &v[1], &v[2]) fills it, then `v[1]` = slot 12
/// (`G_MODEL`) (ST0, stored as f32; the first call's words stay on the
/// stack). Returns slot 12's `eax`.
pub fn model_position<G: Game>(g: &mut G, img: &Image, f: u32, v: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s262 = slot_target(g, w(0x06), 262);
    let model = g.read::<u32>(w(0x17));
    let first = [model, v, v + 4, v + 8];
    g.call(s262, 0, &first);
    let s12 = slot_target(g, w(0x1e), 12);
    let model = g.read::<u32>(w(0x24));
    let mut args = vec![model];
    args.extend(first);
    // call width: 5 (the model and the first call's four words)
    let out = g.call(s12, 0, &args);
    g.write::<u32>(v + 4, st0_f32(out).to_bits());
    out.eax
}

/// `model_position` (52 bytes; 3 overlays).
pub const MODEL_POSITION: Sig = Sig {
    len: 52,
    hash: 0xe8c6_7b82_a1f1_d0b8,
    calls: &[],
    imports: &[],
};

/// 0x10009620 (internal; `ecx` = an effect object): with bit 0 of byte
/// `+0x90`, slot 257 (model `+0x54`, 0); else slot 296 (`+0x50`, 1).
/// Returns the slot's `eax`.
pub fn hide_model<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32) -> u32 {
    let table = img.word(f + 0x08);
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        let s = slot_target(g, table, 257);
        let m = g.read::<u32>(obj + 0x54);
        g.call(s, 0, &[m, 0]).eax
    } else {
        let s = slot_target(g, table, 296);
        let p = g.read::<u32>(obj + 0x50);
        g.call(s, 0, &[p, 1]).eax
    }
}

/// `hide_model` (48 bytes; 3 overlays).
pub const HIDE_MODEL: Sig = Sig {
    len: 48,
    hash: 0x0403_1610_a47e_6694,
    calls: &[],
    imports: &[],
};

/// 0x10005a50 (internal; `ecx` = `[node, parent, a]`): an empty child
/// list with pass byte 0xff (u16), the parent `+0x14` and, under a
/// parent, the link as its last child (the count `+0x18` up); `+0x3c` =
/// `a`, `+0x40..+0x50` 0, scale `+0x28` 1.0. Returns `a`.
pub fn node_init_scaled(g: &mut impl Mem, args: u32) -> u32 {
    let node = g.read::<u32>(args);
    let parent = g.read::<u32>(args + 4);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
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
    let a = g.read::<u32>(args + 8);
    g.write::<u32>(node + 0x3c, a);
    for o in [0x40u32, 0x44, 0x48, 0x4c, 0x50] {
        g.write::<u32>(node + o, 0);
    }
    g.write::<u32>(node + 0x28, 0x3f80_0000);
    a
}

/// `node_init_scaled` (106 bytes, 1 overlay).
pub const NODE_INIT_SCALED: Sig = Sig {
    len: 106,
    hash: 0x5a88_096a_c15d_f22c,
    calls: &[],
    imports: &[],
};

/// 0x10009750 (internal; `ecx` = the node, cdecl `(a, b)`): the node
/// with its handler table `T` (operand `+0x05`), an empty child list,
/// pass byte 0xff, appended to the root node `ROOT` (`+0x18`; its last
/// child `+0x24`, first `+0x39`, count `+0x48`); `+0x3c` = `a`, `+0x40`
/// = `b`. Returns `b`.
pub fn root_child<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, a: u32, b: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(node, w(0x05));
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, w(0x18));
    g.write::<u32>(node + 4, w(0x1f));
    let last = g.read::<u32>(w(0x24));
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(w(0x2c));
    if last == w(0x31) {
        g.write::<u32>(w(0x39), node);
    } else {
        g.write::<u32>(last + 4, node);
    }
    let n = g.read::<u16>(w(0x48)).wrapping_add(1);
    g.write::<u16>(w(0x48), n);
    g.write::<u32>(w(0x4e), node);
    g.write::<u32>(node + 0x3c, a);
    g.write::<u32>(node + 0x40, b);
    b
}

/// `root_child` (93 bytes, 1 overlay).
pub const ROOT_CHILD: Sig = Sig {
    len: 93,
    hash: 0x9a44_c222_fc54_ce59,
    calls: &[],
    imports: &[],
};

/// 0x10009800 (internal; cdecl `(a)`): the static node `NODE` (operand
/// `+0x0a`) set up like [`root_child`]'s (table `+0x0e`), appended to
/// `ROOT`; scale `+0x28` 1.0, `+0x2c` = `a`, `+0x30` = 14. Returns `a`.
pub fn static_child<G: Game>(g: &mut G, img: &Image, f: u32, a: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(0x04));
    g.write::<u32>(w(0x0a), w(0x0e));
    g.write::<u16>(w(0x15), 0xff);
    g.write::<u32>(w(0x1d), w(0x21));
    g.write::<u32>(w(0x27), w(0x2b));
    g.write::<u32>(w(0x31), w(0x35));
    g.write::<u32>(w(0x3b), w(0x3f));
    g.write::<u32>(w(0x44), last);
    if last == w(0x49) {
        g.write::<u32>(w(0x51), w(0x55));
    } else {
        g.write::<u32>(last + 4, w(0x5e));
    }
    let n = g.read::<u16>(w(0x68)).wrapping_add(1);
    g.write::<u16>(w(0x68), n);
    g.write::<u32>(w(0x6e), w(0x72));
    g.write::<u32>(w(0x78), w(0x7c));
    g.write::<u32>(w(0x81), a);
    g.write::<u32>(w(0x87), 0xe);
    a
}

/// `static_child` (145 bytes, 1 overlay).
pub const STATIC_CHILD: Sig = Sig {
    len: 145,
    hash: 0x7ba0_d61a_f3e0_3d35,
    calls: &[],
    imports: &[],
};

/// A step (0x1000d3c0, no arguments): slot 215 `(G_X)`, slot 276 `(h,
/// 0)`, slot 381 `(0)` (each call with the words the earlier ones left).
/// Returns 0.
pub fn e_d3c0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s215 = slot_target(g, w(0x01), 215);
    let x = g.read::<u32>(w(0x07));
    let h = g.call(s215, fr.ecx, &[x]).eax;
    let s276 = slot_target(g, w(0x17), 276);
    // call width: 3
    g.call(s276, fr.ecx, &[h, 0, x]);
    let s381 = slot_target(g, w(0x24), 381);
    // call width: 4
    g.call(s381, fr.ecx, &[0, h, 0, x]);
    Ret::eax(0)
}

/// 0x1000d3c0 (56 bytes, 1 overlay).
pub const E_D3C0: Sig = Sig {
    len: 56,
    hash: 0xabf5_5b71_48b2_eacd,
    calls: &[],
    imports: &[],
};

/// A call of [`crate::nodes::visit_gated`] as the original's `push
/// &group; call` makes it: the argument at `esp + 4`, the callee's `esp`
/// at its return address.
fn gated<G: Game>(g: &mut G, img: &Image, target: u32, esp: u32, group: u32, ecx: u32) {
    g.write::<u32>(esp + 4, group);
    let fr = Frame {
        at: target,
        esp,
        ecx,
        ..Frame::default()
    };
    crate::nodes::visit_gated(g, img, &fr);
}

/// The step at 0x1000d410 (no arguments): the waits of `G_STATE`
/// (operand `+0x04`), each passed once and in order: 0 slot 225 `(2)`, 1
/// slot 313 `(2)`, 2 slot 549 `(2)` (any of them nonzero waits) then
/// slot 226 `(G_ARG, 1)`, 3 slot 227 `(G_ARG)` (below 1 waits). Then slot
/// 673, `*G_P = ITEM`, the frame byte `G_B` up, two globals copied, and
/// unless `G_HOLD` is set each node of the list `G_LIST` (to `END`) run
/// through its handler's `+0x20` with a pointer to a local holding it;
/// an empty list sets bit 0 of `G_DONE`. Last the two nodes `NODE1`,
/// `NODE2` visited through [`crate::nodes::visit_gated`] with a local
/// group (`ebp - 8`, mode `G_B`). Returns 0.
#[allow(clippy::too_many_lines)]
pub fn step_d410<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ecx = fr.ecx;
    let mut st = g.read::<u32>(w(0x04));
    if st <= 3 {
        loop {
            match st {
                0 | 1 => {
                    let (tab, slot) = if st == 0 { (0x1c, 225) } else { (0x38, 313) };
                    let s = slot_target(g, w(tab), slot);
                    if g.call(s, ecx, &[2]).eax != 0 {
                        return Ret::eax(0);
                    }
                }
                2 => {
                    let s549 = slot_target(g, w(0x54), 549);
                    if g.call(s549, ecx, &[2]).eax != 0 {
                        return Ret::eax(0);
                    }
                    let s226 = slot_target(g, w(0x6e), 226);
                    let a = g.read::<u32>(w(0x76));
                    g.call(s226, ecx, &[a, 1]);
                }
                _ => {
                    let s227 = slot_target(g, w(0x9c), 227);
                    let a = g.read::<u32>(w(0xa2));
                    if g.call(s227, ecx, &[a]).eax.cast_signed() < 1 {
                        return Ret::eax(0);
                    }
                    let n = g.read::<u32>(w(0xbc));
                    g.write::<u32>(w(0xbc), n.wrapping_add(1));
                    break;
                }
            }
            st = g.read::<u32>(w(0x86)).wrapping_add(1);
            g.write::<u32>(w(0x8c), st);
            if st > 3 {
                break;
            }
        }
    }
    let s673 = slot_target(g, w(0xc1), 673);
    g.call(s673, ecx, &[]);
    let p = g.read::<u32>(w(0xcf));
    let mut b = g.read::<u8>(w(0xd5));
    g.write::<u32>(p, w(0xdb));
    let x = g.read::<u32>(w(0xe0));
    let mut node = g.read::<u32>(w(0xe6));
    g.write::<u32>(w(0xeb), x);
    let z = g.read::<u32>(w(0xf0));
    b = b.wrapping_add(1);
    let hold = g.read::<u32>(w(0xf8));
    g.write::<u8>(w(0xff), b);
    g.write::<u32>(w(0x105), 0);
    g.write::<u32>(w(0x10e), z);
    let ebp = fr.esp - 4;
    let local = ebp - 8;
    if hold == 0 && node != w(0x116) {
        loop {
            g.write::<u32>(local, node);
            let method = g.read::<u32>(g.read::<u32>(node) + 0x20);
            node = g.read::<u32>(node + 8);
            // lea ecx, [ebp - 8]; push ecx; call
            g.call(method, local, &[local]);
            if node == w(0x138) {
                break;
            }
        }
        b = g.read::<u8>(w(0x140));
        node = g.read::<u32>(w(0x146));
    }
    if node == w(0x14c) {
        let v = g.read::<u32>(w(0x155)) | 1;
        g.write::<u32>(w(0x155), v);
    }
    let target = img.rel32_target(f + 0x169);
    g.write::<u32>(local, w(0x161));
    g.write::<u8>(local + 5, b);
    gated(g, img, target, ebp - 0x10, local, ecx);
    let b = g.read::<u8>(w(0x16e));
    g.write::<u8>(local + 5, b);
    g.write::<u32>(local, w(0x17c));
    let target = img.rel32_target(f + 0x181);
    // (the first call's argument stays below)
    gated(g, img, target, ebp - 0x14, local, ecx);
    Ret::eax(0)
}

/// 0x1000d410 (434 bytes with the switch table and the following
/// `GetEffectOverlayTable`, which the census cut into it; 1 overlay).
pub const STEP_D410: Sig = Sig {
    len: 434,
    hash: 0xa8c3_75b0_c7a5_240e,
    calls: &[
        (0x169, Callee::Sig(&crate::nodes::GATED)),
        (0x181, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};
