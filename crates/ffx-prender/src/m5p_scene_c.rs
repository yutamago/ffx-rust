//! M5 stage 1, list `scene`, batch 3: the utilities' registry, array
//! resizes and element constructors, list unlinks, the scene cursor's
//! advance and the remaining small methods of the scene and gameplay
//! classes. Conventions as in [`crate::m5p_scene_b`] (array headers:
//! count with bit 31 for an external buffer, then the buffer).
//!
//! Image addresses are rebased before use. Nothing here keeps state of
//! its own (worker threads: per-call scratch only).

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::{OPERATOR_DELETE, PHYRE_FREE};

/// `Phyre_PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The registry of the `PUtility*` singletons (linked through `+4`).
pub const UTILITY_LIST: u32 = 0x00c9_01a8;
/// The vtable the `PUtility*` destructors store (`PUtility`'s).
pub const UTILITY_VTABLE: u32 = 0x00b0_e078;
/// `PhyreSceneCursor_SetNode` (thiscall `(cursor; node)`, `ret 4`).
pub const CURSOR_SET_NODE: u32 = 0x0044_1dc0;
/// The scene cursor advance `0x009f2470` runs after its set-up.
pub const CURSOR_ADVANCE_RENDER: u32 = 0x009f_38c0;
/// `Phyre_PRedBlackTree_remove` (thiscall `(tree; node)`, `ret 4`).
pub const TREE_REMOVE: u32 = 0x0057_5000;
/// `Phyre_PRedBlackTree_postInsertFinalize` (thiscall `(tree; node)`).
pub const TREE_FIXUP: u32 = 0x0057_4980;
/// `0x0044e960` (thiscall `(tree; node)`, `ret 4`): the one-word-key
/// tree's attach (the node found, or 0 when it went in).
pub const TREE_ATTACH1: u32 = 0x0044_e960;
/// The entity class tree.
pub const CLASS_TREE: u32 = 0x00c0_bb70;
/// The world class `0x0044ff60` looks for on a class's base chain.
pub const WORLD_CLASS: u32 = 0x00c9_1c18;
/// `Phyre_PBase_GetClassDescriptor` (cdecl).
pub const BASE_CLASS: u32 = 0x0043_5740;
/// `PConstantBuffer::~PConstantBuffer` (thiscall).
pub const CONSTANT_BUFFER_DTOR: u32 = 0x0049_fa10;
/// `0x004e4160(n, old, keep)` (cdecl): the render pass pointer array's
/// reallocation.
pub const RENDER_PASS_REALLOC: u32 = 0x004e_4160;
/// `0x005d5450(p)` (cdecl): the timer's release.
pub const TIMER_RELEASE: u32 = 0x005d_5450;
/// The light/shadow caster type pairs that are compatible:
/// `(light, caster)`; the second light type takes two casters.
pub const COMPATIBLE: [(u32, u32); 3] = [
    (0x00c1_0850, 0x00c1_58c4),
    (0x00c1_0828, 0x00c1_58d8),
    (0x00c1_0828, 0x00c1_58ec),
];
/// `PType_find_004d3780(name)` (cdecl): the context switch type list's
/// lookup.
pub const SWITCH_TYPE_FIND: u32 = 0x004d_3780;
/// The context switch type list: its head and its count.
pub const SWITCH_TYPES: [u32; 2] = [0x00ca_4e18, 0x00ca_4e20];
/// The vtable the context switch type constructor stores.
pub const SWITCH_TYPE_VTABLE: u32 = 0x00b1_f7ac;
/// The trigger utility's global object (`0x00550750` calls its slot 1).
pub const TRIGGER_GLOBAL: u32 = 0x00ca_f388;
/// `Phyre_PClassDescriptor_applyDefaultBufferWithValidation` (thiscall,
/// two words, `ret 8`).
pub const APPLY_DEFAULT_BUFFER: u32 = 0x0043_c160;
/// `0x005506c0` (thiscall, two words, `ret 8`): the trigger receiver
/// array's resize.
pub const RECEIVERS_RESIZE: u32 = 0x0055_06c0;

/// A global singly linked list a node unlinks from: its head and the
/// offset of the link in a node (`0x00433550`, `0x00545e80`).
#[derive(Clone, Copy, Debug)]
pub struct List {
    pub head: u32,
    pub link: u32,
}

/// The gameplay type list (`0x00433550`).
pub const GAMEPLAY_TYPES: List = List {
    head: 0x00ca_ece8,
    link: 4,
};
/// The locator shape type list (`0x00545e80`).
pub const LOCATOR_SHAPES: List = List {
    head: 0x00ca_e548,
    link: 8,
};

pub mod bridge {
    //! The ports over game memory.

    use super::{
        APPLY_DEFAULT_BUFFER, BASE_CLASS, CLASS_TREE, COMPATIBLE, CONSTANT_BUFFER_DTOR,
        CURSOR_SET_NODE, Conv, Host, List, Mem, OPERATOR_DELETE, PHYRE_FREE, PHYRE_MALLOC,
        RECEIVERS_RESIZE, RENDER_PASS_REALLOC, SWITCH_TYPE_FIND, SWITCH_TYPE_VTABLE, SWITCH_TYPES,
        TIMER_RELEASE, TREE_ATTACH1, TREE_FIXUP, TREE_REMOVE, TRIGGER_GLOBAL, UTILITY_LIST,
        UTILITY_VTABLE, WORLD_CLASS,
    };
    use crate::m5p_scene_b::bridge::owner_lists;

    const EXTERNAL: u32 = 0x8000_0000;

    /// `0x00432b80`, `0x00432ce0`, `0x00433460`, `0x005041e0` (thiscall
    /// `(flags)`, `ret 4`): a `PUtility*` singleton's deleting
    /// destructor: `PUtility`'s vtable, the singleton unlinked from the
    /// registry, `operator delete` for `flags & 1`. `this`.
    pub fn utility_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(UTILITY_VTABLE);
        h.write(this, vt);
        let head = h.rebase(UTILITY_LIST);
        let first: u32 = h.read(head);
        if first != 0 {
            let mut slot = head;
            loop {
                let e: u32 = h.read(slot);
                if e == this {
                    break;
                }
                slot = e + 4;
                let next: u32 = h.read(slot);
                if next == 0 {
                    break;
                }
            }
            let at: u32 = h.read(slot);
            if at != 0 {
                let next: u32 = h.read(this + 4);
                h.write(slot, next);
                h.write(this + 4, 0u32);
            }
        }
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// The buffer a `PSharray<T*>` of `n` gets: the header's own word
    /// (`this + 4`) for at most one element, else `PhyreMalloc(4n, 4)`.
    fn sharray_buffer(h: &mut impl Host, this: u32, n: u32) -> u32 {
        if n > 1 {
            h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4])
        } else {
            this + 4
        }
    }

    /// Copies `n` words from `src` to `buf` (a null destination slot
    /// skipped without using up a source word).
    fn copy_words(m: &mut impl Mem, buf: u32, src: u32, n: u32) {
        let mut d = buf;
        let mut s = src;
        for _ in 0..n {
            if d != 0 {
                let w: u32 = m.read(s);
                m.write(d, w);
                s = s.wrapping_add(4);
            }
            d = d.wrapping_add(4);
        }
    }

    /// `0x004f6db0`, `0x005a6aa0` (thiscall `(n, src)`, `ret 8`): the
    /// buffer of a `PSharray<T*>` of `n` with `src`'s words copied in;
    /// the buffer (0 when the allocation failed).
    pub fn sharray_copy(h: &mut impl Host, this: u32, n: u32, src: u32) -> u32 {
        let buf = sharray_buffer(h, this, n);
        if buf != 0 {
            copy_words(h, buf, src, n);
        }
        buf
    }

    /// `0x004f6e10`, `0x0054fa00`, `0x005a6ba0` (thiscall `(n, src, m)`,
    /// `ret 0xc`): as [`sharray_copy`] with `m` words copied and the
    /// other `n - m` zeroed.
    pub fn sharray_copy_fill(h: &mut impl Host, this: u32, n: u32, src: u32, m: u32) -> u32 {
        let buf = sharray_buffer(h, this, n);
        if buf != 0 {
            copy_words(h, buf, src, m);
            let mut p = buf.wrapping_add(m.wrapping_mul(4));
            for _ in 0..n.wrapping_sub(m) {
                h.write(p, 0u32);
                p = p.wrapping_add(4);
            }
        }
        buf
    }

    /// `0x004df240` (cdecl `(items, n)`): each 32-byte item's eight words
    /// zeroed (a null slot skipped). Nothing returned (the original's
    /// `eax` is a cursor, or its caller's for no items).
    pub fn zero_items32(m: &mut impl Mem, items: u32, n: u32) {
        let mut p = items;
        for _ in 0..n {
            if p != 0 {
                for i in 0..8 {
                    m.write(p + 4 * i, 0u32);
                }
            }
            p = p.wrapping_add(0x20);
        }
    }

    /// `0x00a307f0` (thiscall `(n)`, `ret 4`): the count cut to `n` (bit
    /// 31 kept); an owned array cut to nothing frees its buffer. Its
    /// `eax`: `PhyreFree`'s, else the new count's bit 31 with `al`
    /// inverted.
    pub fn array_shrink(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header: u32 = h.read(this);
        let count = header & !EXTERNAL;
        let count = if count < n { count } else { n };
        let header = (header & EXTERNAL) | count;
        h.write(this, header);
        if header & EXTERNAL == 0 && header == 0 {
            let buffer: u32 = h.read(this + 4);
            let eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
            h.write(this + 4, 0u32);
            return eax;
        }
        (header >> 31) ^ 0xff
    }

    /// `0x004e74c0` (thiscall `(n)`, `ret 4`): a `PArray<PSceneRenderPass*>`
    /// resized to `n` through its reallocation (`(n, old, min(n,
    /// count))`); an owned old buffer freed unless reused. 0, or 0xd when
    /// the reallocation failed. Nothing changes when the header word is
    /// `n` already.
    pub fn render_pass_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = header & !EXTERNAL;
        let keep = if n < count { n } else { count };
        let old: u32 = h.read(this + 4);
        let new = h.call_sym(RENDER_PASS_REALLOC, Conv::Cdecl, &[n, old, keep]);
        if new == 0 && n != 0 {
            return 0xd;
        }
        let old: u32 = h.read(this + 4);
        if old != new {
            let header: u32 = h.read(this);
            if header & EXTERNAL == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
            }
        }
        h.write(this + 4, new);
        h.write(this, n);
        0
    }

    /// `0x00a30780` (thiscall `(n)`, `ret 4`): an 8-byte-element array
    /// given a fresh buffer of `n` (`PhyreMalloc(8n, 4)`, none for 0); an
    /// owned, non-null old buffer other than it freed. 0, or 0xd when the
    /// allocation failed. Nothing changes when the header word is `n`.
    pub fn array8_reset(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let mut new = 0;
        if n != 0 {
            new = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(8), 4]);
            if new == 0 {
                return 0xd;
            }
        }
        let old: u32 = h.read(this + 4);
        let header: u32 = h.read(this);
        if old != new && header & EXTERNAL == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, new);
        h.write(this, n);
        0
    }

    /// `0x004fd160` (thiscall on the header): an owned, non-null array of
    /// 0x3c-byte `PConstantBuffer`s destroyed and freed; the buffer then
    /// the count zeroed. Its `eax`: `PhyreFree`'s, else the buffer (0) or
    /// the count's bit 31 with `al` inverted.
    pub fn constant_buffers_clear(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = (count >> 31) ^ 0xff;
        if count & EXTERNAL == 0 {
            let buffer: u32 = h.read(this + 4);
            eax = buffer;
            if buffer != 0 {
                let mut p = buffer;
                for _ in 0..count & !EXTERNAL {
                    h.call_sym(CONSTANT_BUFFER_DTOR, Conv::Thiscall(p), &[]);
                    p = p.wrapping_add(0x3c);
                }
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x00433550`, `0x00545e80` (cdecl `(node)`): `node` unlinked from
    /// the global list `l` (its link cleared). 0.
    pub fn list_unlink(h: &mut impl Host, l: &List, node: u32) -> u32 {
        let head = h.rebase(l.head);
        let mut slot = head;
        let mut e: u32 = h.read(head);
        while e != 0 {
            if e == node {
                let next: u32 = h.read(e + l.link);
                h.write(slot, next);
                h.write(e + l.link, 0u32);
                return 0;
            }
            slot = e + l.link;
            e = h.read(slot);
        }
        0
    }

    /// `0x00450450` (thiscall `(component)`, `ret 4`): a component
    /// detached from its owner `this` (its `+4`): unlinked from the
    /// owner's list (from `+0`, linked through `+0`) and its owner word
    /// cleared. 5 when `this` isn't its owner, else 0.
    pub fn component_detach(m: &mut impl Mem, this: u32, component: u32) -> u32 {
        let owner: u32 = m.read(component + 4);
        if owner != this {
            return 5;
        }
        let mut slot = this;
        let mut e: u32 = m.read(this);
        while e != 0 {
            if e == component {
                let next: u32 = m.read(e);
                m.write(slot, next);
                m.write(e, 0u32);
                break;
            }
            slot = e;
            e = m.read(e);
        }
        m.write(component + 4, 0u32);
        0
    }

    /// `0x00450500` (thiscall `(class)`, `ret 4`): the entity's class
    /// node (`+8`, a one-word-key tree node keyed by the class) taken out
    /// of the class tree when it was in, then for a class: rebuilt, the
    /// entity at `+0x18`, and put in (attach, then the insert fix-up when
    /// it went in). Its `eax`: 0 for no class, else the node found or the
    /// fix-up's.
    pub fn entity_set_class(h: &mut impl Host, this: u32, class: u32) -> u32 {
        let tree = h.rebase(CLASS_TREE);
        let node = this + 8;
        let had: u32 = h.read(this + 4);
        if had != 0 {
            h.call_sym(TREE_REMOVE, Conv::Thiscall(tree), &[node]);
        }
        h.write(this + 4, class);
        if class == 0 {
            return 0;
        }
        h.write(node, node);
        h.write(node + 4, node);
        h.write(node + 8, node);
        h.write(node + 0xc, class);
        h.write(node + 0x10, 0u32);
        h.write(this + 0x18, this);
        let found = h.call_sym(TREE_ATTACH1, Conv::Thiscall(tree), &[node]);
        if found != 0 {
            return found;
        }
        h.call_sym(TREE_FIXUP, Conv::Thiscall(tree), &[node])
    }

    /// Whether `x` is on the chain from `p` linked through `+0x40`.
    fn on_chain(m: &impl Mem, mut p: u32, x: u32) -> bool {
        while p != 0 {
            if p == x {
                return true;
            }
            p = m.read(p + 0x40);
        }
        false
    }

    /// `0x0044ff60` (thiscall `(class)`, `ret 4`, `al`): whether a class
    /// derived from the world class (its base chain through `+0x40`) is
    /// new to the list at `this` (from `+0`, linked through `+0`, each
    /// entry's class at `+8`): no entry's class is on its chain or has it
    /// on theirs. 0 for a null class or one not derived from it.
    pub fn world_class_new(m: &impl Mem, this: u32, class: u32) -> u8 {
        let world = m.rebase(WORLD_CLASS);
        if !on_chain(m, class, world) {
            return 0;
        }
        let mut e: u32 = m.read(this);
        while e != 0 {
            let c: u32 = m.read(e + 8);
            if on_chain(m, c, class) || (c != 0 && on_chain(m, class, c)) {
                return 0;
            }
            e = m.read(e);
        }
        1
    }

    /// `0x004599c0` (thiscall `(out, class)`, `ret 8`): the first element
    /// of the 8-byte array at `+0xc`/`+0x10` whose class chain (from its
    /// `+4`, through `+0x40`) holds `class`, copied to `out`; else
    /// `(0, PBase's class descriptor)`. `out`.
    pub fn instances_find(h: &mut impl Host, this: u32, out: u32, class: u32) -> u32 {
        if class != 0 {
            let count: u32 = h.read(this + 0xc);
            let base: u32 = h.read(this + 0x10);
            let end = base.wrapping_add((count & !EXTERNAL).wrapping_mul(8));
            let mut e = base;
            while e < end {
                let c: u32 = h.read(e + 4);
                if on_chain(h, c, class) {
                    let a: u32 = h.read(e);
                    h.write(out, a);
                    let b: u32 = h.read(e + 4);
                    h.write(out + 4, b);
                    return out;
                }
                e = e.wrapping_add(8);
            }
        }
        h.write(out, 0u32);
        let d = h.call_sym(BASE_CLASS, Conv::Cdecl, &[]);
        h.write(out + 4, d);
        out
    }

    /// The scene cursor's step: the next node (`+0x18`'s first word),
    /// none at the end marker `+0x14`.
    fn cursor_step(m: &mut impl Mem, it: u32) {
        let p: u32 = m.read(it + 0x18);
        if p != 0 {
            let next: u32 = m.read(p);
            let end: u32 = m.read(it + 0x14);
            m.write(it + 0x18, if next == end { 0 } else { next });
        }
    }

    /// The cursor's end: a node set up through `PhyreSceneCursor_SetNode`
    /// (its `eax`), or the five words cleared (0).
    fn cursor_settle(h: &mut impl Host, it: u32) -> u32 {
        let node: u32 = h.read(it + 0x18);
        if node != 0 {
            return h.call_sym(CURSOR_SET_NODE, Conv::Thiscall(it), &[node]);
        }
        for i in 0..5 {
            h.write(it + 4 * i, 0u32);
        }
        0
    }

    /// `0x00443930`, `0x009f38c0` (thiscall on a scene cursor): steps
    /// until the node's owner lists it (no step when it does already),
    /// then settles.
    pub fn cursor_seek(h: &mut impl Host, it: u32) -> u32 {
        if owner_lists(h, it) == 0 {
            loop {
                cursor_step(h, it);
                if owner_lists(h, it) != 0 {
                    break;
                }
            }
        }
        cursor_settle(h, it)
    }

    /// `0x004439a0` (thiscall on a scene cursor): one step at least, then
    /// as [`cursor_seek`].
    pub fn cursor_next(h: &mut impl Host, it: u32) -> u32 {
        loop {
            cursor_step(h, it);
            if owner_lists(h, it) != 0 {
                break;
            }
        }
        cursor_settle(h, it)
    }

    /// `0x009f2470` (thiscall `(owner, mask)`, `ret 8`): a scene cursor
    /// over `owner`'s node ring (`+0x1c`, empty when it points at
    /// itself): five words cleared, the end marker, the first node, the
    /// mask, then [`cursor_seek`]. `this`.
    pub fn cursor_ctor(h: &mut impl Host, this: u32, owner: u32, mask: u32) -> u32 {
        let ring = owner.wrapping_add(0x1c);
        for i in 0..5 {
            h.write(this + 4 * i, 0u32);
        }
        h.write(this + 0x14, ring);
        let first: u32 = h.read(ring);
        h.write(this + 0x18, if first == ring { 0 } else { first });
        h.write(this + 0x1c, mask);
        cursor_seek(h, this);
        this
    }

    /// `PShadowCaster::CheckLightShadowCasterCompatibility(light, caster)`
    /// (`0x004cae20`, cdecl, `al`): whether the pair is one of
    /// [`COMPATIBLE`].
    pub fn light_caster_compatible(m: &impl Mem, light: u32, caster: u32) -> u8 {
        let ok = COMPATIBLE
            .iter()
            .any(|&(l, c)| light == m.rebase(l) && caster == m.rebase(c));
        u8::from(ok)
    }

    /// `0x00550430` (thiscall `(name)`, `ret 4`, `al`): whether the name
    /// `*this` points at equals `name` (bytes; false for a null name or
    /// none).
    pub fn name_is(m: &impl Mem, this: u32, name: u32) -> u8 {
        if name == 0 {
            return 0;
        }
        let p: u32 = m.read(this);
        let s: u32 = m.read(p);
        if s == 0 {
            return 0;
        }
        let mut i = 0u32;
        loop {
            let a: u8 = m.read(s.wrapping_add(i));
            let b: u8 = m.read(name.wrapping_add(i));
            if a != b {
                return 0;
            }
            if a == 0 {
                return 1;
            }
            i += 1;
        }
    }

    /// `0x004d2e50` (thiscall `(unused, name)`, `ret 8`): a context
    /// switch type's constructor: `(vtable, name, 0, 1, 1, 1)`; a name not
    /// listed yet ([`SWITCH_TYPE_FIND`]) is pushed on the type list and
    /// counted. `this`.
    pub fn switch_type_ctor(h: &mut impl Host, this: u32, name: u32) -> u32 {
        h.write(this + 4, name);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 1u8);
        let vt = h.rebase(SWITCH_TYPE_VTABLE);
        h.write(this, vt);
        h.write(this + 0x10, 1u32);
        h.write(this + 0x14, 1u8);
        if name != 0 && h.call_sym(SWITCH_TYPE_FIND, Conv::Cdecl, &[name]) == 0 {
            let head = h.rebase(SWITCH_TYPES[0]);
            let count = h.rebase(SWITCH_TYPES[1]);
            let first: u32 = h.read(head);
            h.write(this + 8, first);
            let n: u32 = h.read(count);
            h.write(count, n.wrapping_add(1));
            h.write(head, this);
            h.write(this + 0x10, 1u32);
        }
        this
    }

    /// `0x00555e20` (thiscall `(src)`, `ret 4`): the timer component's
    /// copy: 1 for no source; else a held, owned timer released, then
    /// virtual slot 7 (`+0x1c`) of `this` with the source's slot 8
    /// (`+0x20`, no words). 0.
    pub fn timer_assign(h: &mut impl Host, this: u32, src: u32) -> u32 {
        if src == 0 {
            return 1;
        }
        let owned: u8 = h.read(this + 0x28);
        let timer: u32 = h.read(this + 0x1c);
        if owned != 0 && timer != 0 {
            h.call_sym(TIMER_RELEASE, Conv::Cdecl, &[timer]);
            h.write(this + 0x28, 0u8);
            h.write(this + 0x1c, 0u32);
        }
        let src_vt: u32 = h.read(src);
        let vt: u32 = h.read(this);
        let get: u32 = h.read(src_vt + 0x20);
        let v = h.call(get, Conv::Thiscall(src), &[]);
        let set: u32 = h.read(vt + 0x1c);
        h.call(set, Conv::Thiscall(this), &[v]);
        0
    }

    /// `0x00550750` (thiscall `(buffer, n)`, `ret 8`): the receivers
    /// array (`this + 4`) set: the trigger global's slot 1 gives a class
    /// descriptor; the header's own word with one element applies its
    /// default buffer to it, anything else goes through the resize
    /// ([`RECEIVERS_RESIZE`]). 0.
    pub fn receivers_set(h: &mut impl Host, this: u32, buffer: u32, n: u32) -> u32 {
        let g = h.rebase(TRIGGER_GLOBAL);
        let vt: u32 = h.read(g);
        let f: u32 = h.read(vt + 4);
        let d = h.call(f, Conv::Thiscall(g), &[]);
        let own = this.wrapping_add(4);
        if buffer == own && n == 1 && d != 0 {
            h.call_sym(APPLY_DEFAULT_BUFFER, Conv::Thiscall(d), &[own, n]);
        } else {
            h.call_sym(RECEIVERS_RESIZE, Conv::Thiscall(this), &[buffer, n]);
        }
        0
    }

    // ---- under five bytes (not hookable: symbols/m5-not-portable.tsv) ----

    /// `0x00432bf0` (`PUtilityScene` slot 2), `0x005042a0` (`PUtilityLOD`
    /// slot 2): `xor eax, eax; ret`.
    #[must_use]
    pub const fn stub_zero() -> u32 {
        0
    }

    /// `0x00555d00` (`PTimerComponent` slot 11): `xor al, al; ret`.
    #[must_use]
    pub const fn stub_false() -> u8 {
        0
    }

    /// `0x004fd120` (thiscall): `mov eax, ecx; ret`.
    #[must_use]
    pub const fn stub_this(this: u32) -> u32 {
        this
    }

    /// `0x004fc880` (thiscall): `lea eax, [ecx+0xc]; ret`.
    #[must_use]
    pub const fn stub_field_c(this: u32) -> u32 {
        this.wrapping_add(0xc)
    }

    /// `0x005d7440` (`PWorldMatrixController` slot 3): a bare `ret`.
    pub const fn stub_nothing() {}
}
