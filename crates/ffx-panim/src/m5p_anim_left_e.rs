//! List `phyre-anim-left`, batch 5 (M5 stage 1): the channel-target binds
//! to bound functions and methods, the scene bind's node lookup and its
//! walk over a node tree, and the network optimiser's run.
//!
//! Callees outside the list stay calls to the original; the optimiser's
//! context lives in the caller's frame (`frame`).

#![allow(clippy::many_single_char_names)]

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyrePrintf(level, fmt, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// The type id a bound callable's signature must start with (cdecl, no
/// arguments; or'ed with 1).
pub const VECTOR_TYPE: u32 = 0x0046_0070;

/// A bind of bound callables (`0x005cdf50` functions, `0x005ce050`
/// methods).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallableBind {
    /// The target type it binds.
    pub ty: u32,
    /// The lookup by name (thiscall on the class, `(name)`).
    pub lookup: u32,
    /// Where the signature's argument type sits (the word after it must be
    /// 0).
    pub sig: u32,
    /// The warning when the signature differs (`"Bound function %s does
    /// not take a single const Vectormath::Aos..."`).
    pub fmt: u32,
    /// Methods also store the object (`+0xc`) and use state 2 (`+0x10` the
    /// method), functions state 3 (`+0x14` the function).
    pub method: bool,
}

/// The [`CallableBind`] copies.
pub const CALLABLE_BINDS: &[(u32, CallableBind)] = &[
    (
        0x005c_df50,
        CallableBind {
            ty: 4,
            lookup: 0x0043_5b80,
            sig: 0,
            fmt: 0x00b3_8850,
            method: false,
        },
    ),
    (
        0x005c_e050,
        CallableBind {
            ty: 3,
            lookup: 0x0043_5c10,
            sig: 4,
            fmt: 0x00b3_88e0,
            method: true,
        },
    ),
];

/// The [`CALLABLE_BINDS`] entry of `f`.
#[must_use]
pub const fn callable_bind_of(f: u32) -> CallableBind {
    let mut i = 0;
    while i < CALLABLE_BINDS.len() {
        if CALLABLE_BINDS[i].0 == f {
            return CALLABLE_BINDS[i].1;
        }
        i += 1;
    }
    panic!("not a callable bind")
}

/// The optimiser context's constructor and destructor (thiscall).
pub const CONTEXT_CTOR: u32 = 0x0061_6120;
/// See [`CONTEXT_CTOR`].
pub const CONTEXT_DTOR: u32 = 0x0061_6150;
/// A node's reset (thiscall).
pub const NODE_RESET: u32 = 0x0061_6d60;
/// Whether a node is placed (thiscall, `al`: its `+0x118` set).
pub const NODE_PLACED: u32 = 0x0061_6d50;
/// Whether a node's inputs are all placed (thiscall, `al`).
pub const NODE_READY: u32 = 0x0061_6d10;
/// A node's place `(k)` (thiscall; `+0x118 = k`).
pub const NODE_PLACE: u32 = 0x0061_6eb0;
/// The node's entry release `0x00616930` and resolve `0x006167a0`
/// (thiscall `(ctx)`, in this list).
pub const NODE_RELEASE: u32 = 0x0061_6930;
/// See [`NODE_RELEASE`].
pub const NODE_RESOLVE: u32 = 0x0061_67a0;
/// The blender's weight and source sets (thiscall on `component + 0x1c`,
/// `(i, w)` / `(i, src)`).
pub const BLENDER_SET_WEIGHT: u32 = 0x0052_e660;
/// See [`BLENDER_SET_WEIGHT`].
pub const BLENDER_SET_SOURCE: u32 = 0x0052_cec0;
/// How many words the optimiser's context takes in the frame.
pub const CONTEXT_WORDS: u32 = 4;

pub mod bridge {
    use super::{
        BLENDER_SET_SOURCE, BLENDER_SET_WEIGHT, CONTEXT_CTOR, CONTEXT_DTOR, CallableBind, Conv,
        Host, Mem, NODE_PLACE, NODE_PLACED, NODE_READY, NODE_RELEASE, NODE_RESET, NODE_RESOLVE,
        PRINTF, VECTOR_TYPE,
    };
    use crate::m5p_anim_left_a::text;
    use crate::m5p_anim_left_d::bridge::compare_scene_keys;

    /// [`super::CALLABLE_BINDS`] (`thiscall(out, targets, n)`, `ret 0xc`):
    /// `PAnimationChannelTargetBindFunction/Method::bindTargets`: 0 for
    /// none, 1 without targets or outputs. Each unbound output (0x18
    /// bytes) of a target of the kind (0x1c bytes) whose class is on the
    /// bind's class chain (`[this + 4]`, linked through `+0x40`) gets the
    /// callable found by name in the chain's head: a warning when its
    /// signature is not `(const Vector &)`, then the state and the
    /// callable (and for methods the object: the bind's `+8`, else the
    /// target's); 0.
    pub fn bind_callables(
        h: &mut impl Host,
        k: CallableBind,
        this: u32,
        out: u32,
        targets: u32,
        n: u32,
    ) -> u32 {
        if n == 0 {
            return 0;
        }
        if targets == 0 || out == 0 {
            return 1;
        }
        for i in 0..n {
            let t = targets.wrapping_add(i.wrapping_mul(0x1c));
            let o = out.wrapping_add(i.wrapping_mul(0x18));
            if h.read::<u32>(t.wrapping_add(0x10)) != k.ty {
                continue;
            }
            let head: u32 = h.read(this.wrapping_add(4));
            let class: u32 = h.read(t);
            let mut cur = head;
            while cur != 0 && cur != class {
                cur = h.read(cur.wrapping_add(0x40));
            }
            if cur == 0 || h.read::<u32>(o) != 0 {
                continue;
            }
            let name = text(h, t.wrapping_add(0x14));
            let p = h.call_sym(k.lookup, Conv::Thiscall(head), &[name]);
            if p == 0 {
                continue;
            }
            let x: u32 = h.read(p.wrapping_add(0x14));
            let sig = if x == 0 { 0 } else { h.call_virtual(x, 4, &[]) };
            let id = h.call_sym(VECTOR_TYPE, Conv::Cdecl, &[]) | 1;
            let at = sig.wrapping_add(k.sig);
            if h.read::<u32>(at) != id || h.read::<u32>(at.wrapping_add(4)) != 0 {
                let fmt = h.rebase(k.fmt);
                h.call_sym(PRINTF, Conv::Cdecl, &[1, fmt, name]);
            }
            if k.method {
                let mut obj: u32 = h.read(this.wrapping_add(8));
                if obj == 0 {
                    obj = h.read(t.wrapping_add(4));
                }
                h.write(o, 2u32);
                h.write(o.wrapping_add(0x10), p);
                h.write(o.wrapping_add(0xc), obj);
            } else {
                h.write(o, 3u32);
                h.write(o.wrapping_add(0x14), p);
            }
        }
        0
    }

    /// The scene bind's key of a name: the hash (`0x7b5`, times 33 plus
    /// each byte's low 5 bits), the length, the text.
    fn name_key(m: &impl Mem, name: u32) -> (u32, u32) {
        let mut hash = 0x7b5u32;
        let mut len = 0u32;
        loop {
            let c: u8 = m.read(name.wrapping_add(len));
            if c == 0 {
                break;
            }
            hash = hash.wrapping_mul(33).wrapping_add(u32::from(c & 0x1f));
            len = len.wrapping_add(1);
        }
        (hash, len)
    }

    /// `0x00616fd0` (`thiscall(&key)`, `ret 4`): the scene bind tree's
    /// node with `key` ([`compare_scene_keys`], node key first), 0 when
    /// absent.
    pub fn scene_tree_find(m: &impl Mem, tree: u32, key: u32) -> u32 {
        let mut cur: u32 = m.read(tree.wrapping_add(0xc));
        while cur != tree {
            let at = if cur == 0 { 0 } else { cur.wrapping_add(0xc) };
            let c = compare_scene_keys(m, at, key);
            if c == 0 {
                return cur;
            }
            cur = m.read(cur.wrapping_add(u32::from(c.cast_signed() < 0) * 4));
        }
        0
    }

    /// [`scene_tree_find`] with the key's words given (hash, length,
    /// text), the original's stack key.
    fn find_named(m: &impl Mem, tree: u32, hash: u32, len: u32, name: u32) -> u32 {
        let mut cur: u32 = m.read(tree.wrapping_add(0xc));
        while cur != tree {
            let at = if cur == 0 { 0 } else { cur.wrapping_add(0xc) };
            let c = compare_named(m, at, hash, len, name);
            if c == 0 {
                return cur;
            }
            cur = m.read(cur.wrapping_add(u32::from(c < 0) * 4));
        }
        0
    }

    /// [`compare_scene_keys`] of the node key at `a` and a key held in
    /// values.
    fn compare_named(m: &impl Mem, a: u32, hash: u32, len: u32, name: u32) -> i32 {
        for (off, y) in [(0u32, hash), (4, len)] {
            let x: u32 = m.read(a.wrapping_add(off));
            if x > y {
                return 1;
            }
            if x < y {
                return -1;
            }
        }
        let pa: u32 = m.read(a.wrapping_add(8));
        for i in 0..len {
            let (x, y): (u8, u8) = (m.read(name.wrapping_add(i)), m.read(pa.wrapping_add(i)));
            if x != y {
                return if x < y { -1 } else { 1 };
            }
        }
        0
    }

    /// `0x006174b0` (`thiscall(node)`, `ret 4`): a scene node (its name at
    /// `+0x50`, bit 0 off) bound: when named, its key looked up in the
    /// bind's tree (`+0xc`); the found entry's slot (`[this + 4]`, by the
    /// entry's index among the 0x18-byte entries at `[this + 8]`) takes
    /// the node when empty; 0.
    pub fn bind_scene_node(m: &mut impl Mem, this: u32, node: u32) -> u32 {
        let name = text(m, node.wrapping_add(0x50));
        if name == 0 || m.read::<u8>(name) == 0 {
            return 0;
        }
        let (hash, len) = name_key(m, name);
        let found = find_named(m, this.wrapping_add(0xc), hash, len, name);
        if found != 0 {
            let entries: u32 = m.read(this.wrapping_add(8));
            let idx = found.wrapping_sub(entries).cast_signed() / 0x18;
            let slots: u32 = m.read(this.wrapping_add(4));
            let slot = slots.wrapping_add(idx.cast_unsigned().wrapping_mul(4));
            if m.read::<u32>(slot) == 0 {
                m.write(slot, node);
            }
        }
        0
    }

    /// `0x00616ee0` (cdecl `(this, root)`): [`bind_scene_node`] on `root`
    /// and every node under it, depth first (first child `+8`, next
    /// sibling `+0`, parent `+4`); 0.
    pub fn bind_scene_tree(m: &mut impl Mem, this: u32, root: u32) -> u32 {
        bind_scene_node(m, this, root);
        let mut cur: u32 = m.read(root.wrapping_add(8));
        if cur == 0 {
            return 0;
        }
        loop {
            bind_scene_node(m, this, cur);
            let child: u32 = m.read(cur.wrapping_add(8));
            let sibling: u32 = m.read(cur);
            let next = if child != 0 {
                child
            } else if sibling != 0 {
                sibling
            } else {
                if cur == root {
                    return 0;
                }
                loop {
                    cur = m.read(cur.wrapping_add(4));
                    let s: u32 = m.read(cur);
                    if s != 0 {
                        if cur == root {
                            return 0;
                        }
                        break s;
                    }
                    if cur == root {
                        return 0;
                    }
                }
            };
            if next == root {
                return 0;
            }
            cur = next;
        }
    }

    /// Every optimiser node's entries released ([`NODE_RELEASE`] with the
    /// context; the count re-read).
    fn release_all(h: &mut impl Host, this: u32, frame: u32) {
        let mut j = 0u32;
        while j < h.read::<u32>(this.wrapping_add(4)) {
            let nodes: u32 = h.read(this);
            let other = nodes.wrapping_add(j.wrapping_mul(0x11c));
            h.call_sym(NODE_RELEASE, Conv::Thiscall(other), &[frame]);
            j += 1;
        }
    }

    /// `0x005cd7a0` (`thiscall(release_first)`, `ret 4`, `al` 1): the
    /// network optimiser's run over its nodes (`0x11c` bytes from
    /// `[this]`, count `+4`, re-read): a context made in the caller's
    /// frame (`frame`), every node reset; then while some node is not
    /// placed but ready, the first such is placed next in order and its
    /// entries resolved and every node's released (releases first when
    /// `release_first`'s low byte is set); the context destroyed. `al` 1
    /// over the destructor's `eax`.
    pub fn run(h: &mut impl Host, this: u32, release_first: u32, frame: u32) -> u32 {
        h.call_sym(CONTEXT_CTOR, Conv::Thiscall(frame), &[]);
        let mut i = 0u32;
        while i < h.read::<u32>(this.wrapping_add(4)) {
            let nodes: u32 = h.read(this);
            h.call_sym(
                NODE_RESET,
                Conv::Thiscall(nodes.wrapping_add(i.wrapping_mul(0x11c))),
                &[],
            );
            i += 1;
        }
        let mut order = 0u32;
        loop {
            let mut pick = None;
            let mut i = 0u32;
            while i < h.read::<u32>(this.wrapping_add(4)) {
                let nodes: u32 = h.read(this);
                let node = nodes.wrapping_add(i.wrapping_mul(0x11c));
                if h.call_sym(NODE_PLACED, Conv::Thiscall(node), &[])
                    .trailing_zeros()
                    >= 8
                {
                    let nodes: u32 = h.read(this);
                    let node = nodes.wrapping_add(i.wrapping_mul(0x11c));
                    if h.call_sym(NODE_READY, Conv::Thiscall(node), &[]) & 0xff != 0 {
                        pick = Some(i);
                        break;
                    }
                }
                i += 1;
            }
            let Some(i) = pick.filter(|i| i.cast_signed() >= 0) else {
                break;
            };
            let nodes: u32 = h.read(this);
            let node = i.wrapping_mul(0x11c).wrapping_add(nodes);
            h.call_sym(NODE_PLACE, Conv::Thiscall(node), &[order]);
            order = order.wrapping_add(1);
            if release_first & 0xff != 0 {
                release_all(h, this, frame);
                h.call_sym(NODE_RESOLVE, Conv::Thiscall(node), &[frame]);
            } else {
                h.call_sym(NODE_RESOLVE, Conv::Thiscall(node), &[frame]);
                release_all(h, this, frame);
            }
        }
        let r = h.call_sym(CONTEXT_DTOR, Conv::Thiscall(frame), &[]);
        r & !0xff | 1
    }

    /// `0x00543570` (`thiscall(arg)`, `ret 4`): the component made to play
    /// its first source alone: 1 for a null `arg` (only checked), 0x11
    /// without a clip set; with no sources one is made first (as
    /// `0x00543e50` with 1, inlined there; its error returned); the
    /// blender's weights 1.0 for source 0 and 0.0 for the rest, source 0
    /// handed to it again, `+0x40` the clip set; 0.
    pub fn use_first_source(h: &mut impl Host, this: u32, arg: u32) -> u32 {
        if arg == 0 {
            return 1;
        }
        if h.read::<u32>(this.wrapping_add(0x10)) == 0 {
            return 0x11;
        }
        let mut n = h.read::<u32>(this.wrapping_add(0x14)) & !0x8000_0000;
        if n == 0 {
            let r = crate::m5p_anim_left_d::bridge::set_source_count(h, this, 1);
            if r != 0 {
                return r;
            }
            n = 1;
        }
        let blender = this.wrapping_add(0x1c);
        h.call_sym(
            BLENDER_SET_WEIGHT,
            Conv::Thiscall(blender),
            &[0, 1.0f32.to_bits()],
        );
        for i in 1..n {
            h.call_sym(BLENDER_SET_WEIGHT, Conv::Thiscall(blender), &[i, 0]);
        }
        let recs: u32 = h.read(this.wrapping_add(0x18));
        h.call_sym(
            BLENDER_SET_SOURCE,
            Conv::Thiscall(blender),
            &[0, recs.wrapping_add(0x38)],
        );
        let set: u32 = h.read(this.wrapping_add(0x10));
        h.write(this.wrapping_add(0x40), set);
        0
    }
}
