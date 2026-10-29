//! List `phyre-anim`, batch 2 (M5 stage 1): the animation classes' small
//! methods and the accessors their `Bind()` tables register.
//!
//! - the hierarchy visits (`visit(visitor, ctx)`, vtable slot 3): the
//!   visitor's begin callback, the one child's visit, the end callback
//!   ([`bridge::visit`], [`VISITS`]);
//! - time controllers' times (`PTimeScaleOffsetController`: `scale * t +
//!   offset` on the x87 at 53 bits, rounded to `f32`);
//! - `PAnimatableComponent`'s and the blenders' accessors: elements of
//!   their `PArray`s of `0x4c`-byte sources, the weights, the
//!   name-addressed setters that forward to the component's lookups;
//! - `PArray` and `PSharray` helpers (truncate, release, find, the
//!   placement constructors of element arrays);
//! - a few list unlinks.
//!
//! Callees outside the list stay calls to the original (`PhyreMalloc`,
//! `PhyreFree`, the CRT, the component's lookups and adders, which are in
//! the list but not ported yet).

#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE, TIME_CONSTANT};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// MSVCR110's `strncpy` import slot.
pub const STRNCPY: u32 = 0x00b0_c368;
/// `crt_memcpy_thunk` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre::PFreeList::~PFreeList` (thiscall).
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
/// The free list `0x0053a0e0` deletes (a `PFreeList *`).
pub const FREE_LIST: u32 = 0x00ca_cd10;
/// The head of the list `0x00507b80` unlinks from (next at `+4`).
pub const UTILITY_LIST: u32 = 0x00ca_9734;
/// The owner `0x005337d0` looks for along `+0x40`.
pub const GAMEPLAY_OWNER: u32 = 0x00ca_91f8;
/// `PAnimationHierarchyNode::~PAnimationHierarchyNode` (thiscall).
pub const HIERARCHY_NODE_DTOR: u32 = 0x0052_1220;
/// `PAnimationDataSource`'s vtable.
pub const DATA_SOURCE_VTABLE: u32 = 0x00b2_8870;
/// `PSharray<T>::find(&key, from)` (thiscall, `ret 8`): the index or -1.
pub const SHARRAY_FIND: u32 = 0x0051_8a60;
/// The batch runner the blender batch functions forward to (cdecl,
/// seven words: six of the batch call's and the blend kernel).
pub const BLEND_BATCH: u32 = 0x0052_c8f0;
/// `PAnimationChannelBase::operator=` (thiscall `(other)`).
pub const CHANNEL_BASE_ASSIGN: u32 = 0x0050_b730;
/// `PArray<PAnimationConstantChannel>::resize(n)` (thiscall).
pub const CONSTANT_CHANNELS_RESIZE: u32 = 0x0051_70c0;
/// `PAnimationSet`'s slot lookup `(n)` (thiscall).
pub const SET_LOOKUP: u32 = 0x0051_d720;
/// `PAnimationSet`'s slot insert `(at, value)` (thiscall).
pub const SET_INSERT: u32 = 0x0051_d7b0;
/// The component's source lookup by name `(name)` (thiscall on the
/// `PAnimationSet` at `+0x10`).
pub const SOURCE_BY_NAME: u32 = 0x0051_ca40;
/// The component's lookup of a source `(source)` (thiscall).
pub const COMPONENT_FIND: u32 = 0x0054_3570;
/// The `PAvoidOutputOverwritingInput` lookup `(key)` (thiscall).
pub const AVOID_FIND: u32 = 0x0061_6cb0;
/// The component lists' appends `(storage, &value)` (thiscall).
pub const APPEND_WORD: u32 = 0x0061_6b00;
/// As [`APPEND_WORD`] for a pair.
pub const APPEND_PAIR: u32 = 0x0061_6b90;

/// One child's visit: the visitor's begin slot, where the child is, the
/// end slot (vtable word indices).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit {
    pub begin: u32,
    /// The child's offset in `this` (its visit is vtable slot 3).
    pub child: Option<u32>,
    pub end: u32,
}

/// The visits: `(copy, shape)`.
pub const VISITS: &[(u32, Visit)] = &[
    // PTimeController
    (
        0x0052_2240,
        Visit {
            begin: 2,
            child: None,
            end: 4,
        },
    ),
    // PTimeIntervalController, PTimeScaleOffsetController: the time source
    (
        0x0052_2ea0,
        Visit {
            begin: 2,
            child: Some(8),
            end: 4,
        },
    ),
    (
        0x0052_3a60,
        Visit {
            begin: 2,
            child: Some(8),
            end: 4,
        },
    ),
    // PAnimationEventController, PAnimationController: the time controller
    (
        0x0052_4ec0,
        Visit {
            begin: 1,
            child: Some(0x10),
            end: 3,
        },
    ),
    (
        0x0052_6690,
        Visit {
            begin: 1,
            child: Some(0x10),
            end: 3,
        },
    ),
    // PAnimationSpuTargetBlenderController: the source
    (
        0x0053_08e0,
        Visit {
            begin: 1,
            child: Some(0xc),
            end: 3,
        },
    ),
];

/// The [`VISITS`] entry of copy `f`.
#[must_use]
pub const fn visit_of(f: u32) -> Visit {
    let mut i = 0;
    while i < VISITS.len() {
        if VISITS[i].0 == f {
            return VISITS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The placement constructors of element arrays (cdecl `(dst, [src,] n)`):
/// what each element gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    /// A copy of the next source element (the source only advances past
    /// elements constructed).
    Copy,
    /// These words.
    Words(&'static [u32]),
}

/// A placement constructor: the element size, what it gets, and where
/// the original's `eax` ends: `dst + n * size + end` (`end` is `-size`
/// for the ones that leave the last element's address).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Construct {
    pub size: u32,
    pub fill: Fill,
    pub end: u32,
}

/// The placement constructors: `(copy, shape)`.
pub const CONSTRUCTS: &[(u32, Construct)] = &[
    (
        0x0051_7900,
        Construct {
            size: 0x10,
            fill: Fill::Copy,
            end: 0u32.wrapping_sub(0x10),
        },
    ),
    (
        0x0051_bc30,
        Construct {
            size: 0x10,
            fill: Fill::Words(&[0, 3, 0, 0]),
            end: 8,
        },
    ),
    (
        0x0053_1e60,
        Construct {
            size: 0x94,
            fill: Fill::Copy,
            end: 0,
        },
    ),
    (
        0x0053_9550,
        Construct {
            size: 0x18,
            fill: Fill::Words(&[0; 6]),
            end: 8,
        },
    ),
    (
        0x0053_9600,
        Construct {
            size: 0x10,
            fill: Fill::Words(&[0; 4]),
            end: 0,
        },
    ),
    (
        0x0061_6430,
        Construct {
            size: 0xc,
            fill: Fill::Copy,
            end: 0u32.wrapping_sub(0xc),
        },
    ),
];

/// The [`CONSTRUCTS`] entry of copy `f`.
#[must_use]
pub const fn construct_of(f: u32) -> Construct {
    let mut i = 0;
    while i < CONSTRUCTS.len() {
        if CONSTRUCTS[i].0 == f {
            return CONSTRUCTS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// `al` set to `bit` over `eax`.
const fn with_al(eax: u32, bit: bool) -> u32 {
    eax & !0xff | bit as u32
}

pub mod bridge {
    use super::{
        APPEND_PAIR, APPEND_WORD, AVOID_FIND, BLEND_BATCH, CHANNEL_BASE_ASSIGN, COMPONENT_FIND,
        CONSTANT_CHANNELS_RESIZE, Construct, Conv, DATA_SOURCE_VTABLE, FREE_LIST, FREE_LIST_DTOR,
        Fill, GAMEPLAY_OWNER, HIERARCHY_NODE_DTOR, Host, MEMCPY, Mem, NOT_OWNED, PHYRE_FREE,
        PHYRE_MALLOC, SET_INSERT, SET_LOOKUP, SHARRAY_FIND, SOURCE_BY_NAME, STRNCPY, TIME_CONSTANT,
        UTILITY_LIST, Visit, with_al, x87,
    };

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    /// The virtual method at word `slot` of `obj`, as an address.
    fn method(m: &impl Mem, obj: u32, slot: u32) -> u32 {
        let vt: u32 = m.read(obj);
        m.read(vt.wrapping_add(slot * 4))
    }

    /// A [`super::VISITS`] copy (`thiscall(visitor, ctx)`, `ret 8`): the
    /// visitor's begin callback `(this, ctx)`; when it says go on, the
    /// child's visit `(visitor, this)`, then the end callback. `al` 1 when
    /// everything went on, else 0 over the `eax` of the call that
    /// stopped.
    pub fn visit(h: &mut impl Host, v: Visit, this: u32, visitor: u32, ctx: u32) -> u32 {
        let r = h.call_virtual(visitor, v.begin, &[this, ctx]);
        if r.trailing_zeros() >= 8 {
            return r;
        }
        if let Some(off) = v.child {
            let child: u32 = h.read(this.wrapping_add(off));
            if child != 0 {
                let r = h.call_virtual(child, 3, &[visitor, this]);
                if r.trailing_zeros() >= 8 {
                    return r;
                }
            }
        }
        let r = h.call_virtual(visitor, v.end, &[this, ctx]);
        with_al(r, true)
    }

    /// `PTimeScaleOffsetController`'s time (`0x00523890`, vtable slot 4):
    /// the time source's (its slot 4), or the constant time without one,
    /// times the scale (`+0xc`) plus the offset (`+0x10`), at 53 bits and
    /// rounded to `f32`.
    pub fn scaled_time(h: &mut impl Host, this: u32) -> f32 {
        let src: u32 = h.read(this.wrapping_add(8));
        let t = if src == 0 {
            let at = h.rebase(TIME_CONSTANT);
            x87::store(x87::load(h.read(at)))
        } else {
            let f = method(h, src, 4);
            h.call_st0(f, Conv::Thiscall(src), &[]).to_bits()
        };
        let scale: u32 = h.read(this.wrapping_add(0xc));
        let offset: u32 = h.read(this.wrapping_add(0x10));
        let product = x87::mul(x87::load(scale), x87::load(t));
        f32::from_bits(x87::store(x87::add(product, x87::load(offset))))
    }

    /// The `0x4c`-byte source records of a component (`[this + 0x18]`,
    /// count at `+0x14`): record `i`'s address, if in range.
    fn source(m: &impl Mem, this: u32, i: u32) -> Option<u32> {
        if i < count(m, this.wrapping_add(0x14)) {
            let els: u32 = m.read(this.wrapping_add(0x18));
            Some(i.wrapping_mul(0x4c).wrapping_add(els))
        } else {
            None
        }
    }

    /// A source record's time source (the object at `+0xc`) asked for its
    /// value (slot 4, `ST0`).
    fn record_value(h: &mut impl Host, record: u32) -> f32 {
        let obj = record.wrapping_add(0xc);
        let f = method(h, obj, 4);
        h.call_st0(f, Conv::Thiscall(obj), &[])
    }

    /// `0x00542560` (`thiscall`, `ST0`): the value of the first source
    /// record with its `+0` set, or 0.
    pub fn first_value(h: &mut impl Host, this: u32) -> f32 {
        let n = count(h, this.wrapping_add(0x14));
        let els: u32 = h.read(this.wrapping_add(0x18));
        for i in 0..n {
            let rec = i.wrapping_mul(0x4c).wrapping_add(els);
            if h.read::<u32>(rec) != 0 {
                return record_value(h, rec);
            }
        }
        0.0
    }

    /// `0x005425c0` (`thiscall(i)`, `ST0`): source record `i`'s value, 0
    /// out of range or with its `+0` clear.
    pub fn value_at(h: &mut impl Host, this: u32, i: u32) -> f32 {
        match source(h, this, i) {
            Some(rec) if h.read::<u32>(rec) != 0 => record_value(h, rec),
            _ => 0.0,
        }
    }

    /// `0x00542e90` (`+8`) / `0x00542fb0` (`+4`) (`thiscall(i)`, `ST0`):
    /// a float of source record `i`, through the x87; 0 out of range.
    pub fn float_at(m: &impl Mem, this: u32, i: u32, off: u32) -> f32 {
        match source(m, this, i) {
            Some(rec) => f32::from_bits(x87::store(x87::load(m.read(rec.wrapping_add(off))))),
            None => 0.0,
        }
    }

    /// `0x00542500` (`thiscall(i)`): source record `i`'s first word, 0
    /// out of range.
    pub fn word_at(m: &impl Mem, this: u32, i: u32) -> u32 {
        source(m, this, i).map_or(0, |rec| m.read(rec))
    }

    /// `0x0052e660` (`thiscall(i, f)`, `ret 8`): weight `i` (`[this +
    /// 0x18]`, count at `+0x14`) = `f` through the x87; 0, or 3 out of
    /// range.
    pub fn set_weight(m: &mut impl Mem, this: u32, i: u32, f: u32) -> u32 {
        if i < count(m, this.wrapping_add(0x14)) {
            let els: u32 = m.read(this.wrapping_add(0x18));
            m.write(
                els.wrapping_add(i.wrapping_mul(4)),
                x87::store(x87::load(f)),
            );
            0
        } else {
            3
        }
    }

    /// `0x005442e0` (`thiscall(i, f)`, `ret 8`): source record `i`'s
    /// weight (`+4`) and the blender's (`this + 0x1c`, [`set_weight`]) =
    /// `f`; 0, or 3 out of range.
    pub fn set_source_weight(m: &mut impl Mem, this: u32, i: u32, f: u32) -> u32 {
        let Some(rec) = source(m, this, i) else {
            return 3;
        };
        let v = x87::store(x87::load(f));
        m.write(rec.wrapping_add(4), v);
        set_weight(m, this.wrapping_add(0x1c), i, v);
        0
    }

    /// `0x0050acf0` (`thiscall(first, last)`, `ret 8`): the first and the
    /// last float of the array at `[this + 8]` (count `[this]`), through
    /// the x87; 0, or 7 for an empty array.
    pub fn first_last(m: &mut impl Mem, this: u32, first: u32, last: u32) -> u32 {
        let n: u32 = m.read(this);
        if n == 0 {
            return 7;
        }
        let els: u32 = m.read(this.wrapping_add(8));
        let a: u32 = m.read(els);
        m.write(first, x87::store(x87::load(a)));
        let b: u32 = m.read(els.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4));
        m.write(last, x87::store(x87::load(b)));
        0
    }

    /// `0x005337d0` (`thiscall`): along the list from `[this + 4]`
    /// (next at `+0x40`), when [`GAMEPLAY_OWNER`] is on it, `[this] - 4`
    /// (0 for a null `[this]`); else 0.
    pub fn owner_part(m: &impl Mem, this: u32) -> u32 {
        let owner = m.rebase(GAMEPLAY_OWNER);
        let mut p: u32 = m.read(this.wrapping_add(4));
        while p != 0 {
            if p == owner {
                let q: u32 = m.read(this);
                return if q == 0 { 0 } else { q.wrapping_sub(4) };
            }
            p = m.read(p.wrapping_add(0x40));
        }
        0
    }

    /// `0x00530580` (cdecl, the SPU target blender's batch function, six
    /// words): `memcpy([out], [[in]], [desc + 4] * 32)`; 0.
    pub fn copy_batch(h: &mut impl Host, out: u32, input: u32, desc: u32) -> u32 {
        let n: u32 = h.read(desc.wrapping_add(4));
        let src: u32 = h.read(input);
        let src: u32 = h.read(src);
        let dst: u32 = h.read(out);
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, n << 5]);
        0
    }

    /// `0x005241d0` (`thiscall(flags, idx, n)`, `ret 0xc`, `this`
    /// unread): `flags[idx[i]] = 1` for `i < n`; `eax = n`.
    pub fn mark(m: &mut impl Mem, flags: u32, idx: u32, n: u32) -> u32 {
        for i in 0..n {
            let k: u32 = m.read(idx.wrapping_add(i.wrapping_mul(4)));
            m.write(flags.wrapping_add(k), 1u8);
        }
        n
    }

    /// `0x00616dc0` (`thiscall(name)`, `ret 4`): `strncpy(this, name,
    /// 0xff)`, `this[0xff] = 0`; `al = 1` over `strncpy`'s `eax`.
    pub fn set_name(h: &mut impl Host, this: u32, name: u32) -> u32 {
        let slot = h.rebase(STRNCPY);
        let f: u32 = h.read(slot);
        let r = h.call(f, Conv::Cdecl, &[this, name, 0xff]);
        h.write(this.wrapping_add(0xff), 0u8);
        with_al(r, true)
    }

    /// `PAnimationDataSource`'s scalar deleting destructor (`0x00523dd0`,
    /// `thiscall(flags)`): its vtable, `~PAnimationHierarchyNode`,
    /// `PhyreFree` when `flags & 1`; `this`.
    pub fn delete_data_source(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(DATA_SOURCE_VTABLE);
        h.write(this, vt);
        h.call_sym(HIERARCHY_NODE_DTOR, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x00519150` (`thiscall`, `PArray<T>::~PArray`): `PhyreFree` of
    /// owned elements (null or not), both words zero. `eax`: `0xfe` when
    /// not owned, else `PhyreFree`'s.
    pub fn release_free(h: &mut impl Host, this: u32) -> u32 {
        let c: u32 = h.read(this);
        let mut eax = 0xfe;
        if c & NOT_OWNED == 0 {
            let els: u32 = h.read(this.wrapping_add(4));
            eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[els]);
        }
        h.write(this, 0u32);
        h.write(this.wrapping_add(4), 0u32);
        eax
    }

    /// `0x00519340` (`thiscall`, `PSharray<T>::~PSharray`): an owned array
    /// of more than one element (one is kept inline) goes to `PhyreFree`;
    /// both words zero. `eax`: `0xfe` not owned, `0xff` inline, else
    /// `PhyreFree`'s.
    pub fn release_sharray(h: &mut impl Host, this: u32) -> u32 {
        let c: u32 = h.read(this);
        let mut eax = 0xfe;
        if c & NOT_OWNED == 0 {
            eax = 0xff;
            if c > 1 {
                let els: u32 = h.read(this.wrapping_add(4));
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[els]);
            }
        }
        h.write(this.wrapping_add(4), 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x0053a0e0` (cdecl): deletes the free list at [`FREE_LIST`] when
    /// set (`~PFreeList`, `PhyreFree`, the pointer cleared); 0.
    pub fn delete_free_list(h: &mut impl Host) -> u32 {
        let g = h.rebase(FREE_LIST);
        let p: u32 = h.read(g);
        if p != 0 {
            h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(p), &[]);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            h.write(g, 0u32);
        }
        0
    }

    /// `0x00616870` (`PAvoidOutputOverwritingInput`'s slot 0,
    /// `thiscall(key, _)`, `ret 8`): looks `key` up in `[this + 4]`; `al`
    /// 0 when found with its first word above `[this + 8]`, else 1, over
    /// that word (0 when not found).
    pub fn may_write(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let table: u32 = h.read(this.wrapping_add(4));
        let r = h.call_sym(AVOID_FIND, Conv::Thiscall(table), &[key]);
        if r == 0 {
            return 1;
        }
        let v: u32 = h.read(r);
        let limit: u32 = h.read(this.wrapping_add(8));
        with_al(v, v <= limit)
    }

    /// `0x0051c2e0` (`thiscall(value)`): the set's lookup of `count + 1`;
    /// when that finds nothing, insert `value` at `count`. The callee's
    /// `eax`.
    pub fn append(h: &mut impl Host, this: u32, value: u32) -> u32 {
        let n = count(h, this);
        let r = h.call_sym(SET_LOOKUP, Conv::Thiscall(this), &[n.wrapping_add(1)]);
        if r != 0 {
            return r;
        }
        h.call_sym(SET_INSERT, Conv::Thiscall(this), &[n, value])
    }

    /// `0x00616a40` (`thiscall(i)`): word 2 of the 12-byte record `i` at
    /// `[this + 0x10c]` (count at `+0x108`), or -1.
    pub fn record_word(m: &impl Mem, this: u32, i: u32) -> u32 {
        if i < count(m, this.wrapping_add(0x108)) {
            let els: u32 = m.read(this.wrapping_add(0x10c));
            m.read(els.wrapping_add(i.wrapping_mul(12)).wrapping_add(8))
        } else {
            u32::MAX
        }
    }

    /// `0x006169e0` (`thiscall(i)`): through pair `i` at `[this + 0x104]`
    /// (count at `+0x100`; an object and an index), the object's record
    /// word ([`record_word`]'s table, unchecked), or -1.
    pub fn pair_record_word(m: &impl Mem, this: u32, i: u32) -> u32 {
        if i < count(m, this.wrapping_add(0x100)) {
            let els: u32 = m.read(this.wrapping_add(0x104));
            let pair = els.wrapping_add(i.wrapping_mul(8));
            let obj: u32 = m.read(pair);
            let k: u32 = m.read(pair.wrapping_add(4));
            let recs: u32 = m.read(obj.wrapping_add(0x10c));
            m.read(recs.wrapping_add(k.wrapping_mul(12)).wrapping_add(8))
        } else {
            u32::MAX
        }
    }

    /// `0x00616d10` (`thiscall`): whether every object of the pairs at
    /// `[this + 0x104]` (count at `+0x100`) has its `+0x118` set (not
    /// -1); `al`.
    pub fn all_bound(m: &impl Mem, this: u32) -> u8 {
        let n = count(m, this.wrapping_add(0x100));
        let els: u32 = m.read(this.wrapping_add(0x104));
        let mut ok = true;
        for i in 0..n {
            if ok {
                let obj: u32 = m.read(els.wrapping_add(i.wrapping_mul(8)));
                ok = m.read::<u32>(obj.wrapping_add(0x118)) != u32::MAX;
            }
        }
        u8::from(ok)
    }

    /// `0x00526290` (`PAnimationController`'s slot 8, `thiscall`): the
    /// clip's output bytes: `0xb0` per channel set (`[p + 4]`), `0x34`
    /// per constant channel (`[p + 0xc]`) and the header's first `u16`
    /// (`[[p]]`), `p = [this + 0xc]`; 0 without a clip.
    pub fn clip_bytes(m: &impl Mem, this: u32) -> u32 {
        let p: u32 = m.read(this.wrapping_add(0xc));
        if p == 0 {
            return 0;
        }
        let a = count(m, p.wrapping_add(4)).wrapping_mul(0xb0);
        let b = count(m, p.wrapping_add(0xc)).wrapping_mul(0x34);
        let hdr: u32 = m.read(p);
        let c: u16 = m.read(hdr);
        a.wrapping_add(b).wrapping_add(u32::from(c))
    }

    /// `0x0053afd0` (`thiscall(i)`): the address of the 28-byte element
    /// `i` of the array at `[this + 0xc]` (count at `+8`), or 0.
    pub fn element_28(m: &impl Mem, this: u32, i: u32) -> u32 {
        if i < count(m, this.wrapping_add(8)) {
            let els: u32 = m.read(this.wrapping_add(0xc));
            els.wrapping_add(i.wrapping_mul(28))
        } else {
            0
        }
    }

    /// `0x005ce9d0` (`thiscall(n)`, a ring buffer: start `+0x18`, end
    /// `+0x1c`, write `+0x24`, read `+0x28`): whether `n` bytes from the
    /// write position would run into the reader. `al` over the read
    /// position.
    pub fn would_overrun(m: &impl Mem, this: u32, n: u32) -> u32 {
        let read: u32 = m.read(this.wrapping_add(0x28));
        let write: u32 = m.read(this.wrapping_add(0x24));
        let to = n.wrapping_add(write);
        if write < read && to >= read {
            return with_al(read, true);
        }
        let end: u32 = m.read(this.wrapping_add(0x1c));
        let start: u32 = m.read(this.wrapping_add(0x18));
        with_al(read, to >= end && read == start)
    }

    /// `0x00531a10` (`thiscall(source)`): the source when it shares this
    /// blender's set (`+8`): stored at `+0xc`, 0; 0x11 without a set, 0x13
    /// for another set.
    pub fn set_source(m: &mut impl Mem, this: u32, source: u32) -> u32 {
        let set: u32 = m.read(this.wrapping_add(8));
        if set == 0 {
            return 0x11;
        }
        if set != m.read::<u32>(source.wrapping_add(8)) {
            return 0x13;
        }
        m.write(this.wrapping_add(0xc), source);
        0
    }

    /// `0x0052cec0` (`thiscall(i, source)`, `ret 8`): as [`set_source`]
    /// into slot `i` of the sources at `[this + 0x10]` (count at `+0xc`);
    /// 3 out of range.
    pub fn set_source_at(m: &mut impl Mem, this: u32, i: u32, source: u32) -> u32 {
        let set: u32 = m.read(this.wrapping_add(8));
        if set == 0 {
            return 0x11;
        }
        if set != m.read::<u32>(source.wrapping_add(8)) {
            return 0x13;
        }
        if i < count(m, this.wrapping_add(0xc)) {
            let els: u32 = m.read(this.wrapping_add(0x10));
            m.write(els.wrapping_add(i.wrapping_mul(4)), source);
            0
        } else {
            3
        }
    }

    /// A [`super::CONSTRUCTS`] copy (cdecl `(dst, src, n)` for copies,
    /// `(dst, n)` else): each of the `n` elements, skipped where its
    /// address is 0 (placement new). The original's `eax` (the caller's
    /// for `n == 0`, given here as `dst`).
    pub fn construct(m: &mut impl Mem, c: Construct, dst: u32, src: u32, n: u32) -> u32 {
        let mut from = src;
        for i in 0..n {
            let at = dst.wrapping_add(i.wrapping_mul(c.size));
            if at == 0 {
                continue;
            }
            match c.fill {
                Fill::Copy => {
                    for k in (0..c.size).step_by(4) {
                        let w: u32 = m.read(from.wrapping_add(k));
                        m.write(at.wrapping_add(k), w);
                    }
                    from = from.wrapping_add(c.size);
                }
                Fill::Words(words) => {
                    for (k, &w) in (0u32..).zip(words) {
                        m.write(at.wrapping_add(4 * k), w);
                    }
                }
            }
        }
        if n == 0 {
            dst
        } else {
            dst.wrapping_add(n.wrapping_mul(c.size)).wrapping_add(c.end)
        }
    }

    /// `PSharray<T>::contains(key)` (`0x0051ca20`, `thiscall`): `key` when
    /// [`SHARRAY_FIND`] finds it (from 0), else 0. The key goes by address
    /// (the host's scratch for the original's argument word).
    pub fn contains(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let at = h.scratch();
        h.write(at, key);
        let i = h.call_sym(SHARRAY_FIND, Conv::Thiscall(this), &[at, 0]);
        if i.cast_signed() < 0 { 0 } else { h.read(at) }
    }

    /// `0x00526590` (`thiscall(target)`): the target when the slot list
    /// at `[this + 8]` has it ([`contains`]): stored at `+0xc`, 0; 0x11
    /// without a list, 0x13 when it lacks it.
    pub fn set_target(h: &mut impl Host, this: u32, target: u32) -> u32 {
        let list: u32 = h.read(this.wrapping_add(8));
        if list == 0 {
            return 0x11;
        }
        if contains(h, list, target) == 0 {
            return 0x13;
        }
        h.write(this.wrapping_add(0xc), target);
        0
    }

    /// `0x006166b0` (`thiscall(value)`): [`APPEND_WORD`] of `value` (by
    /// address) to the list at `this + 0x110`, with a new buffer of
    /// `count + 1` words; 1 when that returned 0, else 0.
    pub fn append_word(h: &mut impl Host, this: u32, value: u32) -> u32 {
        let list = this.wrapping_add(0x110);
        let n = count(h, list).wrapping_add(1) << 2;
        let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n, 4]);
        let at = h.scratch();
        h.write(at, value);
        let r = h.call_sym(APPEND_WORD, Conv::Thiscall(list), &[buf, at]);
        u32::from(r == 0)
    }

    /// `0x006166f0` (`thiscall(a, b)`): as [`append_word`] for the pair
    /// `(a, b)` and the list at `this + 0x100` (8-byte elements).
    pub fn append_pair(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let list = this.wrapping_add(0x100);
        let n = count(h, list).wrapping_add(1) << 3;
        let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n, 4]);
        let at = h.scratch();
        h.write(at, a);
        h.write(at.wrapping_add(4), b);
        let r = h.call_sym(APPEND_PAIR, Conv::Thiscall(list), &[buf, at]);
        u32::from(r == 0)
    }

    /// `0x00543240` / `0x00543440` (`thiscall(a, name)`, `ret 8`): the
    /// source `name` names in the set at `[this + 0x10]`, handed to
    /// `then(a, source)`; 0x11 without a set, 0x13 when not found.
    pub fn by_name2(h: &mut impl Host, then: u32, this: u32, a: u32, name: u32) -> u32 {
        let set: u32 = h.read(this.wrapping_add(0x10));
        if set == 0 {
            return 0x11;
        }
        let src = h.call_sym(SOURCE_BY_NAME, Conv::Thiscall(set), &[name]);
        if src == 0 {
            return 0x13;
        }
        h.call_sym(then, Conv::Thiscall(this), &[a, src])
    }

    /// `0x005431f0` / `0x005433f0` (`thiscall(name)`): as [`by_name2`],
    /// then [`by_source`] with the source.
    pub fn by_name(h: &mut impl Host, then: u32, this: u32, name: u32) -> u32 {
        let set: u32 = h.read(this.wrapping_add(0x10));
        if set == 0 {
            return 0x11;
        }
        let src = h.call_sym(SOURCE_BY_NAME, Conv::Thiscall(set), &[name]);
        if src == 0 {
            return 0x13;
        }
        by_source(h, then, this, src)
    }

    /// `0x00543120` / `0x00543300` (`thiscall(source)`): the component's
    /// lookup of `source`; when that gives 0, `then(0, source)`. Its
    /// `eax`.
    pub fn by_source(h: &mut impl Host, then: u32, this: u32, source: u32) -> u32 {
        let r = h.call_sym(COMPONENT_FIND, Conv::Thiscall(this), &[source]);
        if r != 0 {
            return r;
        }
        h.call_sym(then, Conv::Thiscall(this), &[0, source])
    }

    /// `PSharray<T>::operator[](i)` (`0x0051cc50`, `thiscall(i)`): element
    /// `i` (inline at `+4` for a one-element array), 0 out of range.
    pub fn sharray_get(m: &impl Mem, this: u32, i: u32) -> u32 {
        let n = count(m, this);
        if i >= n {
            return 0;
        }
        let base = if n > 1 {
            m.read(this.wrapping_add(4))
        } else {
            this.wrapping_add(4)
        };
        m.read(base.wrapping_add(i.wrapping_mul(4)))
    }

    /// `0x005cd750` (`thiscall(key)`): the index of the first of the
    /// `[this + 4]` records of `0x11c` bytes at `[this]` whose `+0x118`
    /// is `key`, or -1.
    pub fn find_record(m: &impl Mem, this: u32, key: u32) -> u32 {
        let n: u32 = m.read(this.wrapping_add(4));
        let base: u32 = m.read(this);
        for i in 0..n {
            let rec = base.wrapping_add(i.wrapping_mul(0x11c));
            if m.read::<u32>(rec.wrapping_add(0x118)) == key {
                return i;
            }
        }
        u32::MAX
    }

    /// `0x005441e0` (`thiscall(els, n)`, `ret 8`): `setBorrowed` whose old
    /// owned elements go to `helper(old, count)`; 0.
    pub fn set_borrowed_with(h: &mut impl Host, helper: u32, this: u32, els: u32, n: u32) -> u32 {
        if n & NOT_OWNED == 0 {
            let prev: u32 = h.read(this.wrapping_add(4));
            if prev != els {
                let c: u32 = h.read(this);
                if c & NOT_OWNED == 0 {
                    h.call_sym(helper, Conv::Cdecl, &[prev, c & !NOT_OWNED]);
                }
            }
            h.write(this.wrapping_add(4), els);
            h.write(this, n | NOT_OWNED);
        }
        0
    }

    /// `0x005cea90` (`thiscall(node)`): unlinks `node` from the list at
    /// `this + 0x44` (next at `+0xc`) when its owner (`+0x10`) is `this`:
    /// its `+0xc` and `+0x10` cleared, 0; 0x13 when not there.
    pub fn unlink_node(m: &mut impl Mem, this: u32, node: u32) -> u32 {
        if m.read::<u32>(node.wrapping_add(0x10)) != this {
            return 0x13;
        }
        let mut link = this.wrapping_add(0x44);
        loop {
            let p: u32 = m.read(link);
            if p == 0 {
                return 0x13;
            }
            if p == node {
                let next: u32 = m.read(node.wrapping_add(0xc));
                m.write(link, next);
                m.write(node.wrapping_add(0xc), 0u32);
                m.write(node.wrapping_add(0x10), 0u32);
                return 0;
            }
            link = p.wrapping_add(0xc);
        }
    }

    /// `PAnimationConstantChannel::operator=` (`0x0050f220`,
    /// `thiscall(other)`): unless `other` is `this`, the base's, then
    /// the words `+0x1c..+0x34`; `this`.
    pub fn assign_constant_channel(h: &mut impl Host, this: u32, other: u32) -> u32 {
        if this != other {
            h.call_sym(CHANNEL_BASE_ASSIGN, Conv::Thiscall(this), &[other]);
            for off in [0x1c, 0x20, 0x1c, 0x24, 0x28, 0x2c, 0x30] {
                let w: u32 = h.read(other.wrapping_add(off));
                h.write(this.wrapping_add(off), w);
            }
        }
        this
    }

    /// `0x0053cf20` (`thiscall(n)`, `PArray<T>::truncate`): the count
    /// lowered to `n` (its top bit kept); an owned array left empty
    /// frees its elements (`PhyreFree`, pointer zero). `eax`: `0xfe` not
    /// owned, `0xff` not empty, else `PhyreFree`'s.
    pub fn truncate(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let c: u32 = h.read(this);
        let low = c & !NOT_OWNED;
        let kept = if low < n { low } else { n };
        let c = c & NOT_OWNED | kept;
        h.write(this, c);
        if c & NOT_OWNED != 0 {
            return 0xfe;
        }
        if c != 0 {
            return 0xff;
        }
        let els: u32 = h.read(this.wrapping_add(4));
        let r = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[els]);
        h.write(this.wrapping_add(4), 0u32);
        r
    }

    /// `0x0056d690` (`thiscall(i, out)`, `ret 8`): `*out` = the address
    /// of the 48-byte frame `[[this + 0xc8] + 4 i]` in `[this + 0xb8]`;
    /// 0, 9 without frames, 5 for an index past `[this + 0x20]`.
    pub fn frame(m: &mut impl Mem, this: u32, i: u32, out: u32) -> u32 {
        let frames: u32 = m.read(this.wrapping_add(0xb8));
        if frames == 0 {
            return 9;
        }
        let map: u32 = m.read(this.wrapping_add(0xc8));
        let k: u32 = m.read(map.wrapping_add(i.wrapping_mul(4)));
        if k >= m.read::<u32>(this.wrapping_add(0x20)) {
            return 5;
        }
        m.write(out, k.wrapping_mul(48).wrapping_add(frames));
        0
    }

    /// `0x00516e60` (`thiscall(i)`): removes constant channel `i` (of
    /// `0x34` bytes at `[this + 0x10]`, count at `+0xc`): the last one
    /// assigned over it ([`assign_constant_channel`]), then the array
    /// resized to one less. The resize's `eax`, or 3 out of range.
    pub fn remove_constant_channel(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let n = count(h, this.wrapping_add(0xc));
        if i >= n {
            return 3;
        }
        let last = n - 1;
        if i != last {
            let els: u32 = h.read(this.wrapping_add(0x10));
            let to = i.wrapping_mul(0x34).wrapping_add(els);
            let from = last.wrapping_mul(0x34).wrapping_add(els);
            assign_constant_channel(h, to, from);
        }
        h.call_sym(CONSTANT_CHANNELS_RESIZE, Conv::Thiscall(this), &[last])
    }

    /// `0x00507b80` (cdecl `(x)`; `0x00507b70` jumps here): unlinks `x`
    /// from the list at [`UTILITY_LIST`] (next at `+4`); 0.
    pub fn unlink_utility(m: &mut impl Mem, x: u32) -> u32 {
        let mut link = m.rebase(UTILITY_LIST);
        loop {
            let p: u32 = m.read(link);
            if p == 0 {
                return 0;
            }
            if p == x {
                let next: u32 = m.read(p.wrapping_add(4));
                m.write(link, next);
                m.write(p.wrapping_add(4), 0u32);
                return 0;
            }
            link = p.wrapping_add(4);
        }
    }

    /// `0x0052e140` / `0x0052f600` (cdecl, six words): the blend batch
    /// runner with the copy's kernel; its `eax`.
    pub fn blend_batch(h: &mut impl Host, kernel: u32, args: [u32; 6]) -> u32 {
        let k = h.rebase(kernel);
        let [a, b, c, d, e, f] = args;
        h.call_sym(BLEND_BATCH, Conv::Cdecl, &[a, b, c, d, e, f, k])
    }
}
