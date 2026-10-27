//! The render-side glue, forty-ninth batch (M4 port round 14, list
//! `h3-glue-render`): a character's unload. The exception frame and
//! cookie are left out, as in [`crate::glue10`].

/// `Phyre_PRendering_PRenderer_syncRender` (called with the renderer in
/// `ecx`, which it ignores).
pub const SYNC_RENDER: u32 = 0x005b_c3f0;
/// The MSVC string's `assign(this; src, pos, n)` and `substr(this; out,
/// pos, n)` (both `ret 0xc`; `substr` returns `out`).
pub const STRING_ASSIGN_SUBSTR: u32 = 0x0040_3a10;
pub const STRING_SUBSTR: u32 = 0x0040_2e20;
/// `memmove`'s import slot (cdecl).
pub const MEMMOVE: u32 = 0x00b0_c34c;
/// `operator delete` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `TextureAnimationManager_GetInstance()` (cdecl) and
/// `unregisterByName(manager; name)` (`ret 4`).
pub const TEXTURE_ANIMATION_INSTANCE: u32 = 0x006b_02e0;
pub const UNREGISTER_BY_NAME: u32 = 0x006b_1a70;
/// The cluster vector's copy constructor (`this; other`, `ret 4`).
pub const VECTOR_COPY: u32 = 0x0064_d620;
/// `ClusterManager_GetInstance()` (cdecl) and `releasePCluster(manager;
/// cluster)` (`ret 4`).
pub const CLUSTER_MANAGER_INSTANCE: u32 = 0x0069_a1a0;
pub const RELEASE_PCLUSTER: u32 = 0x0069_bdb0;
/// The loaded-character tree's `erase(tree; &out, node)` (`ret 8`).
pub const TREE_ERASE: u32 = 0x0063_7190;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// The string's text: its buffer from capacity 16 on, else inline.
    pub(crate) fn text(h: &impl Host, s: u32) -> u32 {
        if h.read::<u32>(s + 0x14) >= 0x10 {
            h.read(s)
        } else {
            s
        }
    }

    /// An empty MSVC string at `s` (capacity 15, size 0, the text's
    /// first byte 0, in that order).
    pub(crate) fn clear_string(h: &mut impl Host, s: u32) {
        h.write(s + 0x14, 0xfu32);
        h.write(s + 0x10, 0u32);
        h.write(s, 0u8);
    }

    /// The tree's in-order successor of `node` (`+0` left, `+4` parent,
    /// `+8` right, byte `+0xd` the head's mark). A head stays where it
    /// is, as in the original.
    pub(crate) fn successor(h: &impl Host, node: u32) -> u32 {
        let nil = |n: u32| h.read::<u8>(n.wrapping_add(0xd)) != 0;
        if nil(node) {
            return node;
        }
        let right: u32 = h.read(node.wrapping_add(8));
        if nil(right) {
            let mut node = node;
            let mut parent: u32 = h.read(node.wrapping_add(4));
            while !nil(parent) && node == h.read::<u32>(parent.wrapping_add(8)) {
                node = parent;
                parent = h.read(parent.wrapping_add(4));
            }
            parent
        } else {
            let mut node = right;
            loop {
                let left: u32 = h.read(node);
                if nil(left) {
                    return node;
                }
                node = left;
            }
        }
    }

    /// The last index of `c` in the string at `s`, or -1 (`rfind` from
    /// the end; the original compares the cursor with the string's text
    /// before each step back).
    pub(crate) fn rfind(h: &impl Host, s: u32, c: u8) -> u32 {
        let size: u32 = h.read(s + 0x10);
        if size == 0 {
            return u32::MAX;
        }
        let start = text(h, s);
        let mut p = start.wrapping_add(size - 1);
        loop {
            if h.read::<u8>(p) == c {
                return p.wrapping_sub(text(h, s));
            }
            if p == text(h, s) {
                return u32::MAX;
            }
            p = p.wrapping_sub(1);
        }
    }

    /// `FUN_00670b20(this; character)` (thiscall, `ret 4`, eax 0; frame
    /// from `ebp - 0x54`): the renderer (`+0xd0`) flushed (its virtual
    /// `+8`, `syncRender`). With the character's key (`+4`) set, the
    /// scene's loaded-character tree (`+0x11dc`) is walked in order to
    /// the node whose value's first word is the key (none: a null node,
    /// which the original reads from too). The node's name (`+0x10`)
    /// copied to `ebp - 0x28` and cut to the part between its last `/`
    /// and its last `.` (`substr` into `ebp - 0x40`, moved back); its
    /// texture animations unregistered; the value's cluster vector
    /// (`+4`) copied to `ebp - 0x34`, each cluster and then the key
    /// released, the value's vector emptied and freed, the value freed,
    /// the node erased (its result at `ebp - 0x4c`). Then the first of
    /// the scene's 8 unload slots (`+0xfccc`, 0x20c bytes apart) in use
    /// for this character is cleared.
    pub fn unload_character(h: &mut impl Host, ebp: u32, this: u32, character: u32) -> u32 {
        h.write(ebp - 0x48, this);
        h.write(ebp - 0x50, character);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_virtual(renderer, 2, &[]);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SYNC_RENDER, Conv::Thiscall(renderer), &[]);
        let key: u32 = h.read(character.wrapping_add(4));
        h.write(ebp - 0x4c, key);
        if key != 0 {
            release_character(h, ebp, this, key);
        }
        for k in 0..8 {
            let slot = this + 0xfccc + 0x20c * k;
            if h.read::<u32>(slot) != 0 && h.read::<u32>(slot + 4) == character {
                h.write(slot, 0u32);
                h.write(slot + 4, 0u32);
                h.write(slot + 8, 0u32);
                break;
            }
        }
        0
    }

    /// [`unload_character`]'s work for a set key.
    fn release_character(h: &mut impl Host, ebp: u32, this: u32, key: u32) {
        let tree = this + 0x11dc;
        h.write(ebp - 0x54, tree);
        let head: u32 = h.read(tree);
        let mut node: u32 = h.read(head);
        let found = loop {
            if node == head {
                break 0;
            }
            let value: u32 = h.read(node.wrapping_add(0x28));
            if h.read::<u32>(value) == key {
                break node;
            }
            node = successor(h, node);
        };
        let name = ebp - 0x28;
        clear_string(h, name);
        h.call_sym(
            STRING_ASSIGN_SUBSTR,
            Conv::Thiscall(name),
            &[found.wrapping_add(0x10), 0, u32::MAX],
        );
        h.write(ebp - 0x44, b'/');
        let slash = rfind(h, name, b'/');
        h.write(ebp - 0x44, b'.');
        let dot = rfind(h, name, b'.');
        let temp = ebp - 0x40;
        let cut = h.call_sym(
            STRING_SUBSTR,
            Conv::Thiscall(name),
            &[
                temp,
                slash.wrapping_add(1),
                dot.wrapping_sub(slash).wrapping_sub(1),
            ],
        );
        if cut != name {
            move_string(h, name, cut);
        }
        if h.read::<u32>(temp + 0x14) >= 0x10 {
            let data: u32 = h.read(temp);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
        let text = text(h, name);
        let manager = h.call_sym(TEXTURE_ANIMATION_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(UNREGISTER_BY_NAME, Conv::Thiscall(manager), &[text]);
        let value: u32 = h.read(found.wrapping_add(0x28));
        let clusters = ebp - 0x34;
        h.call_sym(
            VECTOR_COPY,
            Conv::Thiscall(clusters),
            &[value.wrapping_add(4)],
        );
        let begin: u32 = h.read(clusters);
        let end: u32 = h.read(clusters + 4);
        let mut p = begin;
        while p != end {
            let cluster: u32 = h.read(p);
            release(h, cluster);
            p = p.wrapping_add(4);
        }
        let value: u32 = h.read(found.wrapping_add(0x28));
        let first: u32 = h.read(value.wrapping_add(4));
        h.write(value.wrapping_add(8), first);
        release(h, key);
        let value: u32 = h.read(found.wrapping_add(0x28));
        if value != 0 {
            let data: u32 = h.read(value.wrapping_add(4));
            if data != 0 {
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
                h.write(value.wrapping_add(4), 0u32);
                h.write(value.wrapping_add(8), 0u32);
                h.write(value.wrapping_add(0xc), 0u32);
            }
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[value]);
        }
        h.call_sym(TREE_ERASE, Conv::Thiscall(tree), &[ebp - 0x4c, found]);
        let data: u32 = h.read(clusters);
        if data != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
        if h.read::<u32>(name + 0x14) >= 0x10 {
            let data: u32 = h.read(name);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
    }

    /// `ClusterManager_releasePCluster` on the manager's instance.
    fn release(h: &mut impl Host, cluster: u32) {
        let manager = h.call_sym(CLUSTER_MANAGER_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(RELEASE_PCLUSTER, Conv::Thiscall(manager), &[cluster]);
    }

    /// The string move-assignment the original inlines: `to`'s buffer
    /// freed, `to` cleared, `from`'s text taken (copied with its 0 when
    /// inline, else its buffer), `from` cleared.
    pub(crate) fn move_string(h: &mut impl Host, to: u32, from: u32) {
        if h.read::<u32>(to + 0x14) >= 0x10 {
            let data: u32 = h.read(to);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
        clear_string(h, to);
        if h.read::<u32>(from + 0x14) < 0x10 {
            let n = h.read::<u32>(from + 0x10).wrapping_add(1);
            if n != 0 {
                let f: u32 = h.read(h.rebase(MEMMOVE));
                h.call(f, Conv::Cdecl, &[to, from, n]);
            }
        } else {
            let data: u32 = h.read(from);
            h.write(to, data);
            h.write(from, 0u32);
        }
        let size: u32 = h.read(from + 0x10);
        h.write(to + 0x10, size);
        let capacity: u32 = h.read(from + 0x14);
        h.write(to + 0x14, capacity);
        h.write(from + 0x14, 0xfu32);
        h.write(from + 0x10, 0u32);
        h.write(from, 0u8);
    }
}
