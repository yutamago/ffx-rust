//! M5 stage 1, list `phyre-core`, batch 3: lookups, copies and the
//! context switches' evaluators.
//!
//! - the named-object tree's lookup by name, `PFreeList`'s merge, a
//!   16-bit index buffer widened to 32 bits;
//! - `PShaderPass`'s copy constructor and its two array copies;
//! - the shader context switches' evaluators (vtable slots, stdcall: they
//!   write one word at a cursor and move it on): the LOD level, the
//!   orthographic camera test, the light-type define, the LOD level's
//!   per-segment array;
//! - the lazily built `PClassDescriptorConcrete<PArray<T>>` statics'
//!   registers ([`LAZY`]), an external-buffer setter, a two-stage
//!   validation;
//! - `PAnimatableComponent`'s 0x11c-byte elements (destructor, finds by
//!   key), the animation hash key and its map lookup, the rigid body's
//!   forwards.

/// `Phyre::PhyreFree(p)`, `Phyre::PhyreMalloc(size, align)` and the CRT's
/// `memcpy` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const MEMCPY: u32 = 0x0094_9256;
/// The CRT's `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `Phyre_PRedBlackTree_successor_005751d0(tree; node)` (thiscall).
pub const TREE_SUCCESSOR: u32 = 0x0057_51d0;
/// `Phyre_PArray_resize_0047ef50(array; n)` (thiscall).
pub const ARRAY32_RESIZE: u32 = 0x0047_ef50;
/// The shader pass arrays' element copies `(dst, src, n)` (cdecl): 2-byte
/// and 16-byte elements.
pub const COPY2: u32 = 0x004b_2d00;
pub const COPY16: u32 = 0x004b_2d70;
/// `0x004bbc30` / `0x004bbdc0(dst; &src)` (thiscall), the pass's arrays.
pub const PASS_ARRAY2_COPY: u32 = 0x004b_bc30;
pub const PASS_ARRAY16_COPY: u32 = 0x004b_bdc0;
/// The orthographic-camera test's threshold.
pub const ORTHO_THRESHOLD: u32 = 0x00b1_f2b0;
/// `PArray_releaseTaggedWords_00434360(data, count)` (cdecl).
pub const RELEASE_TAGGED_WORDS: u32 = 0x0043_4360;
/// `PStringBuilder`: the constructor from a text, `+= text`, `+= int`
/// and the destructor (thiscall), and the define's texts (`"-D"`, `"="`).
pub const BUILDER_NEW: u32 = 0x0044_49e0;
pub const BUILDER_APPEND_TEXT: u32 = 0x0044_4bf0;
pub const BUILDER_APPEND_INT: u32 = 0x0044_4b90;
pub const BUILDER_DTOR: u32 = 0x0044_4ab0;
pub const DEFINE_PREFIX: u32 = 0x00b1_f99c;
pub const DEFINE_EQUALS: u32 = 0x00b7_ca94;
/// The LOD level's per-segment arrays' release `0x004d3720(data, count)`.
pub const SEGMENT_ARRAYS_RELEASE: u32 = 0x004d_3720;
/// The class record's register `0x0043d6a0(class; object, flag)`.
pub const CLASS_REGISTER: u32 = 0x0043_d6a0;
/// The animation channel arrays' destroy `0x00528a30(data, count)`.
pub const SLOT_FILTER_FREE: u32 = 0x0052_8a30;
/// `PAnimatableComponent`'s element: its key `0x00616aa0(e)` and the
/// members the finds forward to (thiscall).
pub const ELEMENT_KEY: u32 = 0x0061_6aa0;
pub const ELEMENT_SET: u32 = 0x0061_69e0;
pub const ELEMENT_GET: u32 = 0x0061_6a20;
pub const ELEMENT_SET_B: u32 = 0x0061_6a40;
/// The animation key's empty text, the map's find `0x005cc870(map;
/// &key)` (thiscall) and the fallback `0x0051cbb0(owner; &key)`.
pub const EMPTY_TEXT: u32 = 0x00b3_8620;
pub const KEY_HASH: u32 = 0x005c_cd70;
pub const MAP_FIND: u32 = 0x005c_c870;
pub const KEY_FALLBACK: u32 = 0x0051_cbb0;
/// The class descriptors the rigid body's forwards look for (`+0x40`
/// links).
pub const RIGID_BODY_CLASS: u32 = 0x00ca_f780;
pub const RIGID_BODY_OTHER_CLASS: u32 = 0x00cb_e410;

/// A lazily built descriptor static and its register: the guard word
/// (bit 0), the static, its constructor (thiscall), the vtable, the
/// `atexit` destructor and the flag passed to the register.
#[derive(Clone, Copy, Debug)]
pub struct Lazy {
    pub guard: u32,
    pub descriptor: u32,
    pub ctor: u32,
    pub vtable: u32,
    pub dtor: u32,
    pub flag: u32,
}

const CHANNELS: Lazy = Lazy {
    guard: 0x00ca_acdc,
    descriptor: 0x00ca_ac48,
    ctor: 0x0051_2cb0,
    vtable: 0x00b2_6df4,
    dtor: 0x00b0_0500,
    flag: 1,
};
const CONSTANT_CHANNELS: Lazy = Lazy {
    guard: 0x00ca_abbc,
    descriptor: 0x00ca_ab28,
    ctor: 0x0051_2d50,
    vtable: 0x00b2_6cec,
    dtor: 0x00b0_0520,
    flag: 1,
};

/// The `PClassDescriptorConcrete<PArray<T>>` registers of this list
/// (cdecl `(p)`): `PArray<PAnimationChannel *>` and
/// `PArray<PAnimationConstantChannel>`, flag 1 and flag 0.
pub const LAZY: &[(u32, Lazy)] = &[
    (0x0051_2740, CHANNELS),
    (0x0051_27d0, CONSTANT_CHANNELS),
    (
        0x0051_2880,
        Lazy {
            flag: 0,
            ..CHANNELS
        },
    ),
    (
        0x0051_2910,
        Lazy {
            flag: 0,
            ..CONSTANT_CHANNELS
        },
    ),
];

/// The [`LAZY`] entry of copy `f`.
#[must_use]
pub const fn lazy_of(f: u32) -> Lazy {
    let mut i = 0;
    while i < LAZY.len() {
        if LAZY[i].0 == f {
            return LAZY[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The animation key's hash over a name: from 0x7b5, `h * 33 + (c &
/// 0x1f)` per byte.
#[must_use]
pub fn name_hash(name: &[u8]) -> u32 {
    name.iter().fold(0x7b5u32, |h, &c| {
        h.wrapping_add(h << 5).wrapping_add(u32::from(c & 0x1f))
    })
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn thiscall(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn free(h: &mut impl Host, p: u32) -> u32 {
        cdecl(h, PHYRE_FREE, &[p])
    }

    /// Writes `v` at the cursor's word and moves the cursor on (the
    /// `add [cursor], 4` reads it again after the store).
    fn emit(h: &mut impl Host, cursor: u32, v: u32) {
        let at: u32 = h.read(cursor);
        h.write(at, v);
        let at: u32 = h.read(cursor);
        h.write(cursor, at.wrapping_add(4));
    }

    /// Whether the C strings at `a` and `b` are equal.
    fn same_text(h: &impl Host, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(i));
            if x != h.read::<u8>(b.wrapping_add(i)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            i = i.wrapping_add(1);
        }
    }

    /// `0x0047d330(tree; name)` -> the object or 0: walks the tree from
    /// its first node (the root `+0xc`, left links `+0`, the tree as nil)
    /// in order, and returns the first node's object (`+0xc`) whose name
    /// (`+0x18`) is `name`.
    pub fn find_by_name(h: &mut impl Host, tree: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let root: u32 = h.read(tree + 0xc);
        let mut node = root;
        if root != tree && root != 0 {
            loop {
                let left: u32 = h.read(node);
                if left == tree {
                    break;
                }
                node = left;
            }
        }
        if node == tree {
            return 0;
        }
        let mut next = thiscall(h, TREE_SUCCESSOR, tree, &[node]);
        loop {
            let obj: u32 = h.read(node.wrapping_add(0xc));
            if obj != 0 {
                let text: u32 = h.read(obj.wrapping_add(0x18));
                if same_text(h, text, name) {
                    return obj;
                }
            }
            node = next;
            if next != tree {
                next = thiscall(h, TREE_SUCCESSOR, tree, &[next]);
            }
            if node == tree {
                return 0;
            }
        }
    }

    /// `0x0049bd10(buffer; &out)` -> the 32-bit indices or 0: format 8
    /// (`+0x19`) has them at `+8` already; format 12 (16-bit) widens the
    /// `[+0]` indices into `out` (resized by `Phyre_PArray_resize_0047ef50`;
    /// 0 when that fails) and returns its data.
    pub fn indices32(h: &mut impl Host, this: u32, out: u32) -> u32 {
        let format: u8 = h.read(this + 0x19);
        if format == 8 {
            return h.read(this + 8);
        }
        if format != 0xc || out == 0 {
            return 0;
        }
        let n: u32 = h.read(this);
        if thiscall(h, ARRAY32_RESIZE, out, &[n]) != 0 {
            return 0;
        }
        let src: u32 = h.read(this + 8);
        let dst: u32 = h.read(out + 4);
        let mut i = 0u32;
        while i < h.read::<u32>(this) {
            let v: u16 = h.read(src.wrapping_add(2 * i));
            h.write(dst.wrapping_add(4 * i), u32::from(v));
            i += 1;
        }
        h.read(out + 4)
    }

    /// Moves the chain at `from` (links at `+0`) onto the one at `to`, a
    /// node at a time (each pushed on the front).
    fn move_chain(h: &mut impl Host, from: u32, to: u32) {
        loop {
            let node: u32 = h.read(from);
            if node == 0 {
                return;
            }
            let next: u32 = h.read(node);
            h.write(from, next);
            let head: u32 = h.read(to);
            h.write(node, head);
            h.write(to, node);
        }
    }

    /// `0x0049c730(list; &other)` -> 0, or 5 when the element sizes
    /// (`+4`) differ: `other`'s free chain (`+0x14`) and block chain
    /// (`+0`) move onto this list's.
    pub fn free_list_merge(h: &mut impl Host, this: u32, other: u32) -> u32 {
        if h.read::<u32>(other + 4) != h.read::<u32>(this + 4) {
            return 5;
        }
        move_chain(h, other + 0x14, this + 0x14);
        move_chain(h, other, this);
        0
    }

    /// A shader pass array's copy constructor `(this; &src)` -> `this`:
    /// four words, then the `+0x10` count / `+0x14` data array copied
    /// into `PhyreMalloc(n * stride, align)` by `copy(block, data, n)`
    /// (count 0 when nothing came).
    pub fn pass_array_copy(
        h: &mut impl Host,
        this: u32,
        src: u32,
        stride: u32,
        align: u32,
        copy: u32,
    ) -> u32 {
        for w in 0..4 {
            let v: u32 = h.read(src + 4 * w);
            h.write(this + 4 * w, v);
        }
        h.write(this + 0x10, 0u32);
        let n = h.read::<u32>(src + 0x10) & 0x7fff_ffff;
        let data: u32 = h.read(src + 0x14);
        let mut block = 0;
        if n != 0 {
            block = cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(stride), align]);
            if block != 0 {
                cdecl(h, copy, &[block, data, n]);
            }
        }
        h.write(this + 0x14, block);
        h.write(this + 0x10, if block == 0 { 0 } else { n });
        this
    }

    /// `PShaderPass`'s copy constructor `0x004bbcb0(this; &src)` ->
    /// `this`: the 2-byte array at `+0`, 0x5f words from `+0x18`, eight
    /// 16-byte arrays from `+0x194` (0x18 apart).
    pub fn shader_pass_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        thiscall(h, PASS_ARRAY2_COPY, this, &[src]);
        for w in 0..0x5f {
            let v: u32 = h.read(src + 0x18 + 4 * w);
            h.write(this + 0x18 + 4 * w, v);
        }
        for k in 0..8 {
            let off = 0x194 + 0x18 * k;
            thiscall(h, PASS_ARRAY16_COPY, this + off, &[src + off]);
        }
        this
    }

    /// The LOD level's evaluator `0x004d4430(shader, segment, a2, cursor,
    /// a4, a5, a6)` (stdcall, seven words) -> 0: the segment's level
    /// (`[[+0x20]] + 0x30`, 0 without one), capped at the shader's level
    /// count (`+0x3c`) less one (0 for none).
    pub fn lod_level(h: &mut impl Host, shader: u32, segment: u32, cursor: u32) -> u32 {
        let mut level = 0u32;
        let p: u32 = h.read(segment.wrapping_add(0x20));
        if p != 0 {
            let q: u32 = h.read(p);
            level = h.read(q.wrapping_add(0x30));
        }
        let n: u32 = h.read(shader.wrapping_add(0x3c));
        let v = if n == 0 {
            0
        } else if level < n - 1 {
            level
        } else {
            n - 1
        };
        emit(h, cursor, v);
        0
    }

    /// The orthographic camera's evaluator `0x004d4480(a0, a1, a2, cursor,
    /// a4, &camera)` (stdcall, six words) -> 0: 1 when the camera's
    /// `+0x6c` is at least the threshold (or unordered), else 0 (and 0
    /// without a camera).
    pub fn ortho_camera(h: &mut impl Host, cursor: u32, camera: u32) -> u32 {
        let mut set = false;
        if camera != 0 {
            let cam: u32 = h.read(camera);
            if cam != 0 {
                let k = x87::load(h.read(h.rebase(ORTHO_THRESHOLD)));
                let w = x87::load(h.read(cam.wrapping_add(0x6c)));
                // fcomp: C0 or C3 (k < w, k == w, unordered)
                set = k.partial_cmp(&w) != Some(core::cmp::Ordering::Greater);
            }
        }
        emit(h, cursor, u32::from(set));
        0
    }

    /// The light-type define's evaluator `0x004d46a0(switch; &cursor, a1,
    /// &defines)` (`ret 0xc`) -> 0, or 0xd when an allocation fails: takes
    /// the next value from the cursor; for one, the defines array becomes
    /// one owned string, set to `"-D" + name (+4) + "=" + value` (built in
    /// the frame's `PStringBuilder` at `builder`).
    pub fn light_define(
        h: &mut impl Host,
        this: u32,
        cursor: u32,
        defines: u32,
        builder: u32,
    ) -> u32 {
        let at: u32 = h.read(cursor);
        h.write(cursor, at.wrapping_add(4));
        let value: u32 = h.read(at);
        if value == 0 {
            return 0;
        }
        if h.read::<u32>(defines) != 1 {
            let block = cdecl(h, PHYRE_MALLOC, &[4, 4]);
            if block == 0 {
                return 0xd;
            }
            h.write(block, 0u32);
            let c: u32 = h.read(defines);
            let d: u32 = h.read(defines + 4);
            if d != block && c >> 31 == 0 {
                cdecl(h, RELEASE_TAGGED_WORDS, &[d, c & 0x7fff_ffff]);
            }
            h.write(defines + 4, block);
            h.write(defines, 1u32);
        }
        let prefix = h.rebase(DEFINE_PREFIX);
        thiscall(h, BUILDER_NEW, builder, &[prefix]);
        let name: u32 = h.read(this + 4);
        thiscall(h, BUILDER_APPEND_TEXT, builder, &[name]);
        let equals = h.rebase(DEFINE_EQUALS);
        thiscall(h, BUILDER_APPEND_TEXT, builder, &[equals]);
        thiscall(h, BUILDER_APPEND_INT, builder, &[value]);
        let len: u32 = h.read(builder + 8);
        let slot: u32 = h.read(defines + 4);
        let text = if len == 0 { 0 } else { h.read(builder + 4) };
        let size = if len == 0 { 1 } else { len };
        let mut copy = 0;
        if text != 0 {
            copy = cdecl(h, PHYRE_MALLOC, &[size, 2]);
            if copy != 0 {
                cdecl(h, MEMCPY, &[copy, text, size]);
            }
            if copy == 0 {
                thiscall(h, BUILDER_DTOR, builder, &[]);
                return 0xd;
            }
        }
        let old: u32 = h.read(slot);
        if old & 1 == 0 {
            free(h, old);
        }
        h.write(slot, copy);
        thiscall(h, BUILDER_DTOR, builder, &[]);
        0
    }

    /// The LOD level's per-segment evaluator `0x004d5400(shader, &arrays)`
    /// (stdcall, two words) -> 0 or 0xd: `arrays` gets one 8-byte array
    /// header per level (`+0x3c`; the old ones released by `0x004d3720`
    /// unless external), each of one owned word, set to its index.
    pub fn lod_segment_arrays(h: &mut impl Host, shader: u32, arrays: u32) -> u32 {
        let n: u32 = h.read(shader.wrapping_add(0x3c));
        if n != h.read::<u32>(arrays) {
            let mut block = 0;
            if n != 0 {
                block = cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(8), 4]);
                if block == 0 {
                    return 0xd;
                }
                for k in 0..n {
                    let e = block.wrapping_add(8 * k);
                    if e != 0 {
                        h.write(e, 0u32);
                        h.write(e.wrapping_add(4), 0u32);
                    }
                }
            }
            let d: u32 = h.read(arrays + 4);
            if d != block {
                let c: u32 = h.read(arrays);
                if c >> 31 == 0 {
                    cdecl(h, SEGMENT_ARRAYS_RELEASE, &[d, c & 0x7fff_ffff]);
                }
            }
            h.write(arrays + 4, block);
            h.write(arrays, n);
        }
        for i in 0..n {
            let e = h.read::<u32>(arrays + 4).wrapping_add(8 * i);
            if h.read::<u32>(e) != 1 {
                let block = cdecl(h, PHYRE_MALLOC, &[4, 4]);
                if block == 0 {
                    return 0xd;
                }
                let old: u32 = h.read(e.wrapping_add(4));
                if old != block && h.read::<u32>(e) >> 31 == 0 && old != 0 {
                    free(h, old);
                }
                h.write(e.wrapping_add(4), block);
                h.write(e, 1u32);
            }
            let p: u32 = h.read(h.read::<u32>(arrays + 4).wrapping_add(8 * i + 4));
            h.write(p, i);
        }
        0
    }

    /// A lazily built descriptor's register `(p)` -> the register's
    /// `eax`: on first use (guard bit 0) the static is constructed, its
    /// `+0x90` loses bit 1, gets its vtable and an `atexit` destructor;
    /// then `0x0043d6a0(descriptor; p, flag)`.
    pub fn lazy_register(h: &mut impl Host, p: u32, l: Lazy) -> u32 {
        let guard = h.rebase(l.guard);
        let d = h.rebase(l.descriptor);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            thiscall(h, l.ctor, d, &[]);
            let f: u32 = h.read(d + 0x90);
            h.write(d + 0x90, f & !2);
            let vt = h.rebase(l.vtable);
            h.write(d, vt);
            let dtor = h.rebase(l.dtor);
            cdecl(h, ATEXIT, &[dtor]);
        }
        thiscall(h, CLASS_REGISTER, d, &[p, l.flag])
    }

    /// `0x0052ab30(array; data, n)` -> 0: unless `n` is negative, `data`
    /// becomes the external storage (count `n | 0x80000000`); the old
    /// owned storage goes through `0x00528a30(data, count)`.
    pub fn slot_filters_set_external(h: &mut impl Host, this: u32, data: u32, n: u32) -> u32 {
        if (n as i32) < 0 {
            return 0;
        }
        let d: u32 = h.read(this + 4);
        if d != data {
            let c: u32 = h.read(this);
            if c >> 31 == 0 {
                cdecl(h, SLOT_FILTER_FREE, &[d, c & 0x7fff_ffff]);
            }
        }
        h.write(this + 4, data);
        h.write(this, n | 0x8000_0000);
        0
    }

    /// `0x00531b40(this; a, b)` -> `al` over the last callee's `eax`: `a`'s
    /// slot 1 `(this, b)`, then the target `[this + 0xc]`'s slot 3 `(a,
    /// this)` when there is one, then `a`'s slot 3 `(this, b)`; 0 as soon
    /// as a check says no.
    pub fn validate_pair(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let vt: u32 = h.read(a);
        let f: u32 = h.read(vt.wrapping_add(4));
        let r = h.call(f, Conv::Thiscall(a), &[this, b]);
        if r as u8 == 0 {
            return r & !0xff;
        }
        let target: u32 = h.read(this + 0xc);
        if target != 0 {
            let vt: u32 = h.read(target);
            let f: u32 = h.read(vt.wrapping_add(0xc));
            let r = h.call(f, Conv::Thiscall(target), &[a, this]);
            if r as u8 == 0 {
                return r & !0xff;
            }
        }
        let vt: u32 = h.read(a);
        let f: u32 = h.read(vt.wrapping_add(0xc));
        let r = h.call(f, Conv::Thiscall(a), &[this, b]);
        r & !0xff | 1
    }

    /// The element with key `key` among the `[this + 4]` 0x11c-byte
    /// elements at `[this]` (`0x00616aa0(e)` is the key).
    fn element_with_key(h: &mut impl Host, this: u32, key: u32) -> Option<u32> {
        let mut i = 0u32;
        while i < h.read::<u32>(this + 4) {
            let e = h.read::<u32>(this).wrapping_add(i.wrapping_mul(0x11c));
            if thiscall(h, ELEMENT_KEY, e, &[]) == key {
                return Some(h.read::<u32>(this).wrapping_add(i.wrapping_mul(0x11c)));
            }
            i += 1;
        }
        None
    }

    /// `0x005cd660(this; key, v)` -> `0x006169e0(e; v)`'s `eax`, -1 when
    /// no element has the key.
    pub fn element_set(h: &mut impl Host, this: u32, key: u32, v: u32) -> u32 {
        match element_with_key(h, this, key) {
            Some(e) => thiscall(h, ELEMENT_SET, e, &[v]),
            None => u32::MAX,
        }
    }

    /// `0x005cd6b0(this; key)` -> `0x00616a20(e)`'s `eax`, 0 when none.
    pub fn element_get(h: &mut impl Host, this: u32, key: u32) -> u32 {
        match element_with_key(h, this, key) {
            Some(e) => thiscall(h, ELEMENT_GET, e, &[]),
            None => 0,
        }
    }

    /// `0x005cd700(this; key, v)` -> `0x00616a40(e; v)`'s `eax`, -1 when
    /// none.
    pub fn element_set_b(h: &mut impl Host, this: u32, key: u32, v: u32) -> u32 {
        match element_with_key(h, this, key) {
            Some(e) => thiscall(h, ELEMENT_SET_B, e, &[v]),
            None => u32::MAX,
        }
    }

    /// The owned-array reset of `+0` count / `+4` data; `free` unless
    /// external, with or without a null test. `eax` as the originals
    /// leave it.
    fn reset(h: &mut impl Host, at: u32, null_check: bool) -> u32 {
        let count: u32 = h.read(at);
        let mut eax = (count >> 31) ^ 0xff;
        if count >> 31 == 0 {
            let data: u32 = h.read(at + 4);
            if null_check {
                eax = data;
                if data != 0 {
                    eax = free(h, data);
                }
            } else {
                eax = free(h, data);
            }
        }
        h.write(at + 4, 0u32);
        h.write(at, 0u32);
        eax
    }

    /// `PAnimatableComponent`'s element destructor `0x005cd4c0(e)` ->
    /// `eax`: the arrays at `+0x110` (no null test), `+0x108` and
    /// `+0x100`.
    pub fn element_dtor(h: &mut impl Host, e: u32) -> u32 {
        reset(h, e + 0x110, false);
        reset(h, e + 0x108, true);
        reset(h, e + 0x100, true)
    }

    /// The animation key's hash `0x005ccd70(&key)`: [`name_hash`] of the
    /// name (`+0x14`, tag bit 0 dropped; the empty text for none) xored
    /// with the key's other words (`+0` to `+0x10`, `+0x18`).
    pub fn key_hash(h: &mut impl Host, key: u32) -> u32 {
        let tagged: u32 = h.read(key + 0x14);
        let mut p = tagged - (tagged & 1);
        if p == 0 {
            p = h.rebase(EMPTY_TEXT);
        }
        let mut hash = 0x7b5u32;
        loop {
            let c: u8 = h.read(p);
            if c == 0 {
                break;
            }
            hash = hash
                .wrapping_add(hash << 5)
                .wrapping_add(u32::from(c & 0x1f));
            p = p.wrapping_add(1);
        }
        let mut x = hash;
        for off in [0x18, 0x10, 0xc, 8, 4, 0] {
            x ^= h.read::<u32>(key + off);
        }
        x
    }

    /// `0x005cce50(this; &key)` -> the entry's index, -1 when missing:
    /// without a map (`+4`) the owner's `0x0051cbb0([this]; key)`;
    /// otherwise the map at `+8` is searched for the key's copy in the
    /// frame at `local` (hash, name, the five words, `+0x18`; eight
    /// words), and the found entry's offset from `[this + 4]` is divided
    /// by its size 0x2c.
    pub fn key_index(h: &mut impl Host, this: u32, key: u32, local: u32) -> u32 {
        if h.read::<u32>(this + 4) == 0 {
            let owner: u32 = h.read(this);
            return thiscall(h, KEY_FALLBACK, owner, &[key]);
        }
        let tagged: u32 = h.read(key + 0x14);
        let mut name = tagged - (tagged & 1);
        if name == 0 {
            name = h.rebase(EMPTY_TEXT);
        }
        for w in 0..5 {
            let v: u32 = h.read(key + 4 * w);
            h.write(local + 8 + 4 * w, v);
        }
        let tail: u32 = h.read(key + 0x18);
        h.write(local + 4, name);
        h.write(local + 0x1c, tail);
        let hash = cdecl(h, KEY_HASH, &[key]);
        h.write(local, hash);
        let found = thiscall(h, MAP_FIND, this + 8, &[local]);
        if found == 0 {
            return u32::MAX;
        }
        let base: u32 = h.read(this + 4);
        (found.wrapping_sub(base) as i32 / 0x2c) as u32
    }

    /// The object behind the rigid body's link: `[this + 4] - 4` when the
    /// class chain at `+8` (links `+0x40`) holds `class` and that is not
    /// null.
    fn linked_object(h: &mut impl Host, this: u32, class: u32) -> Option<u32> {
        let want = h.rebase(class);
        let mut c: u32 = h.read(this + 8);
        loop {
            if c == 0 {
                return None;
            }
            if c == want {
                break;
            }
            c = h.read(c.wrapping_add(0x40));
        }
        let p: u32 = h.read(this + 4);
        if p == 0 {
            return None;
        }
        let obj = p.wrapping_sub(4);
        if obj == 0 { None } else { Some(obj) }
    }

    /// `0x005d3e30(this)` -> `eax`: `[this + 4]` when the class chain
    /// holds `0x00cbe410` (and that is not 0); else, with the rigid-body
    /// descriptor `0x00caf780` in the chain, the linked object's vtable
    /// `+0x18` (thiscall, a tail jump); else 0.
    pub fn rigid_body_slot6(h: &mut impl Host, this: u32) -> u32 {
        let other = h.rebase(RIGID_BODY_OTHER_CLASS);
        let mut c: u32 = h.read(this + 8);
        let mut found = 0;
        while c != 0 {
            if c == other {
                found = h.read(this + 4);
                break;
            }
            c = h.read(c.wrapping_add(0x40));
        }
        if found != 0 {
            return found;
        }
        match linked_object(h, this, RIGID_BODY_CLASS) {
            Some(obj) => {
                let vt: u32 = h.read(obj);
                let f: u32 = h.read(vt.wrapping_add(0x18));
                h.call(f, Conv::Thiscall(obj), &[])
            }
            None => 0,
        }
    }
}
