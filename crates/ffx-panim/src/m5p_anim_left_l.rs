//! List `phyre-anim-left`, batch 12 (M5 stage 1):
//! `PAnimationChannelTargetBindScene`'s target binding: the targets'
//! node names keyed into a temporary tree (`0x00617350`), the scene
//! walked to find the nodes (`0x00616ee0`), and the found nodes handed
//! to the outputs (`0x005ce1a0`). The temporary bind object, its name
//! and result arrays live in the caller's frame.

#![allow(clippy::many_single_char_names)]

use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE};

/// The node array's constructor (thiscall on a `PArray`, `(n)`): `n`
/// self-linked 0x18-byte nodes.
pub const NODES_CTOR: u32 = 0x0061_7170;
/// MSVCR110's `strchr` import slot.
pub const STRCHR: u32 = 0x00b0_c3c0;
/// `Phyre_PType_nameHash_004377d0` (cdecl `(name, seed, len)`).
pub const NAME_HASH: u32 = 0x0043_77d0;
/// The scene tree's insert `0x006170e0` and walk `0x00616ee0` (in this
/// list).
pub const TREE_INSERT: u32 = 0x0061_70e0;
/// See [`TREE_INSERT`].
pub const BIND_TREE: u32 = 0x0061_6ee0;
/// `Phyre_PRedBlackTree_postInsertFinalize` (thiscall `(node)`).
pub const TREE_FIX: u32 = 0x0057_4980;
/// The tree header's reset (thiscall) and the tree's clear (thiscall).
pub const TREE_RESET: u32 = 0x0057_4970;
/// See [`TREE_RESET`].
pub const TREE_CLEAR: u32 = 0x0057_4760;
/// The class a scene target must derive from (its chain through `+0x40`).
pub const SCENE_NODE_CLASS: u32 = 0x00ca_91f8;
/// The `name` binder's `0x00617350` takes its node array in the caller's
/// frame (`ebp - 0x18`).
pub const NAMES_FRAME_WORDS: u32 = 2;
/// The bind object's words (`ebp - 0x3c`).
pub const BIND_WORDS: u32 = 9;
/// How many targets [`bridge::bind_targets`]'s frame block covers; more
/// use a heap block.
pub const FRAME_TARGETS: u32 = 64;

pub mod bridge {
    use super::{
        BIND_TREE, Conv, Host, NAME_HASH, NODES_CTOR, NOT_OWNED, PHYRE_FREE, SCENE_NODE_CLASS,
        STRCHR, TREE_CLEAR, TREE_FIX, TREE_INSERT, TREE_RESET,
    };
    use crate::m5p_anim_left_a::{strlen, text};

    /// `0x00617350` (`thiscall(names, results, n)`, `ret 0xc`): the bind
    /// object's results (`+4`) set; `n` nodes made in a node array in the
    /// frame (`frame`: count, data); each name not starting with `#` keyed
    /// (hash over the text up to a `#`, its length, the text) and inserted
    /// into the tree at `+0xc` (rebalanced when new); the scene walked
    /// with the nodes at `+8`, then `+8` cleared and the tree reset; the
    /// nodes freed.
    pub fn bind_names(h: &mut impl Host, this: u32, names: u32, results: u32, n: u32, frame: u32) {
        if n == 0 {
            return;
        }
        h.write(this.wrapping_add(4), results);
        h.call_sym(NODES_CTOR, Conv::Thiscall(frame), &[n]);
        let data: u32 = h.read(frame.wrapping_add(4));
        if data != 0 {
            let tree = this.wrapping_add(0xc);
            let slot = h.rebase(STRCHR);
            let strchr: u32 = h.read(slot);
            for i in 0..n {
                let name: u32 = h.read(names.wrapping_add(4 * i));
                let node = data.wrapping_add(0x18 * i);
                if name == 0 || h.read::<u8>(name) == b'#' {
                    continue;
                }
                let hash_at = h.call(strchr, Conv::Cdecl, &[name, u32::from(b'#')]);
                let len = if hash_at == 0 {
                    strlen(h, name)
                } else {
                    hash_at.wrapping_sub(name)
                };
                h.write(node, node);
                h.write(node.wrapping_add(4), node);
                h.write(node.wrapping_add(8), node);
                let hash = h.call_sym(NAME_HASH, Conv::Cdecl, &[name, 0x7b5, len]);
                h.write(node.wrapping_add(0x10), len);
                h.write(node.wrapping_add(0xc), hash);
                h.write(node.wrapping_add(0x14), name);
                if h.call_sym(TREE_INSERT, Conv::Thiscall(tree), &[node]) == 0 {
                    h.call_sym(TREE_FIX, Conv::Thiscall(tree), &[node]);
                }
            }
            h.write(this.wrapping_add(8), data);
            let root: u32 = h.read(this);
            h.call_sym(BIND_TREE, Conv::Cdecl, &[this, root]);
            h.write(this.wrapping_add(8), 0u32);
            h.call_sym(TREE_RESET, Conv::Thiscall(tree), &[]);
        }
        let held: u32 = h.read(frame);
        let data: u32 = h.read(frame.wrapping_add(4));
        if held & NOT_OWNED == 0 && data != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }

    /// Where `0x005ce1a0` keeps its temporaries: the bind object (9
    /// words), the names and the results (`n` words each).
    #[derive(Debug, Clone, Copy)]
    pub struct Scratch {
        pub bind: u32,
        pub names: u32,
        pub results: u32,
    }

    /// `0x005ce1a0` (`thiscall(out, targets, n)`, `ret 0xc`):
    /// `PAnimationChannelTargetBindScene::bindTargets`: 0 for none, 1
    /// without outputs or targets. The names of the unbound type-1 targets
    /// whose class derives from the scene node class are gathered, a bind
    /// object over the bind's root (`+4`) with an empty tree made, the
    /// names bound ([`bind_names`] at its address); each found node's
    /// output gets `+0` 1, `+4` the node + 0x10, `+8` 0x40; the tree
    /// cleared; 0.
    pub fn bind_targets(
        h: &mut impl Host,
        this: u32,
        out: u32,
        targets: u32,
        n: u32,
        s: Scratch,
    ) -> u32 {
        if n == 0 {
            return 0;
        }
        if targets == 0 || out == 0 {
            return 1;
        }
        let class_want = h.rebase(SCENE_NODE_CLASS);
        for i in 0..n {
            let t = targets.wrapping_add(0x1c * i);
            let mut name = 0;
            let mut cls: u32 = h.read(t);
            if h.read::<u32>(t.wrapping_add(0x10)) == 1 && cls != 0 {
                while cls != 0 && cls != class_want {
                    cls = h.read(cls.wrapping_add(0x40));
                }
                if cls != 0 && h.read::<u32>(out.wrapping_add(0x18 * i)) == 0 {
                    name = text(h, t.wrapping_add(0x14));
                }
            }
            h.write(s.names.wrapping_add(4 * i), name);
        }
        for i in 0..n {
            h.write(s.results.wrapping_add(4 * i), 0u32);
        }
        let root: u32 = h.read(this.wrapping_add(4));
        let hdr = s.bind.wrapping_add(0xc);
        h.write(s.bind, root);
        h.write(s.bind.wrapping_add(4), 0u32);
        h.write(s.bind.wrapping_add(8), 0u32);
        for off in [0u32, 4, 8, 0x14, 0x10, 0xc] {
            h.write(hdr.wrapping_add(off), hdr);
        }
        h.call_sym(
            super::BIND_NAMES,
            Conv::Thiscall(s.bind),
            &[s.names, s.results, n],
        );
        for i in 0..n {
            let r: u32 = h.read(s.results.wrapping_add(4 * i));
            if r != 0 {
                let o = out.wrapping_add(0x18 * i);
                h.write(o, 1u32);
                h.write(o.wrapping_add(4), r.wrapping_add(0x10));
                h.write(o.wrapping_add(8), 0x40u32);
            }
        }
        h.call_sym(TREE_CLEAR, Conv::Thiscall(hdr), &[]);
        0
    }

    /// [`bind_targets`] with its temporaries in `frame` ([`super::FRAME_TARGETS`]
    /// targets' worth after the bind object), or for more in a
    /// `PhyreMalloc` block freed afterwards (the original's `alloca`).
    pub fn bind_targets_in(
        h: &mut impl Host,
        this: u32,
        out: u32,
        targets: u32,
        n: u32,
        frame: u32,
    ) -> u32 {
        let words = super::BIND_WORDS;
        if n <= super::FRAME_TARGETS {
            let s = Scratch {
                bind: frame,
                names: frame.wrapping_add(4 * words),
                results: frame.wrapping_add(4 * (words + super::FRAME_TARGETS)),
            };
            return bind_targets(h, this, out, targets, n, s);
        }
        let size = n.wrapping_mul(8).wrapping_add(4 * words);
        let block = h.call_sym(super::PHYRE_MALLOC, Conv::Cdecl, &[size, 4]);
        if block == 0 {
            return 0xd;
        }
        let s = Scratch {
            bind: block,
            names: block.wrapping_add(4 * words),
            results: block.wrapping_add(4 * words).wrapping_add(4 * n),
        };
        let r = bind_targets(h, this, out, targets, n, s);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[block]);
        r
    }
}

/// The names binder `0x00617350` (in this list; called at its address).
pub const BIND_NAMES: u32 = 0x0061_7350;

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The words [`bridge::bind_targets_in`]'s frame takes.
pub const TARGETS_FRAME_WORDS: u32 = BIND_WORDS + 2 * FRAME_TARGETS;
