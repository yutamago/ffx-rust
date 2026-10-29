//! List `phyre-anim-left`, batch 1 (M5 stage 1): the animation utilities
//! the larger `PAnimation` functions build on.
//!
//! - name lookups and the data-member bind targets' (re)binding
//!   (`m_currentPose`), the bind targets' name copies;
//! - the channel interpolators: the component-wise lerp and the quaternion
//!   slerp (x87 at 53 bits, `f32` at every store, `_CIacos`/`_CIsin`);
//! - `PAnimationSetTargetLookup`: its red-black tree of target keys (the
//!   key compare, the insert, the lookup and the constructor);
//! - `PAnimationSet`'s rebuild into a cluster object and the count of its
//!   channels' batch keys;
//! - `PArray` resizes and the placement copy/default constructors of their
//!   elements (`0x34`-byte channels, `0x1c`-byte named targets, `float`s).
//!
//! Callees outside the list stay calls to the original. Shims keep no
//! state: everything is per call (animation batches run on worker
//! threads).

#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_MemDup_0042f220` (cdecl `(src, size)`).
pub const MEM_DUP: u32 = 0x0042_f220;
/// `crt_memcpy_thunk` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// The CRT `memset` (cdecl `(dst, c, n)`).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre::PhyrePrintf(level, fmt, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// `_CIacos` (import thunk; argument and result in `ST0`).
pub const CI_ACOS: u32 = 0x0094_98da;
/// `_CIsin` (import thunk).
pub const CI_SIN: u32 = 0x0094_98c8;
/// The class check `0x0050c850` makes before binding a member
/// (cdecl `(descriptor, name)`, `al`).
pub const HAS_MEMBER: u32 = 0x0050_be90;
/// "Setting animation channel to target unregistered member '%s' in class '%s'".
pub const UNREGISTERED_FMT: u32 = 0x00b2_5b88;
/// The descriptor `0x00432e90` rebinds to (`m_currentPose`'s class).
pub const POSE_DESCRIPTOR: u32 = 0x00ca_7228;
/// `"m_currentPose"`.
pub const CURRENT_POSE: u32 = 0x00b0_e110;
/// The global free list `0x0053a0e0` deletes (the first anim agent's port
/// is `ffx_panim::m5p_anim_misc`; called here at its address).
pub const DELETE_FREE_LIST: u32 = 0x0053_a0e0;
/// The utility list unlink `0x00507b70` (cdecl `(entry)`).
pub const UNLINK: u32 = 0x0050_7b70;
/// The first of the 19 animation channel-target type records
/// `0x00433250` unlinks (`Rotation` .. `UserAttributeVector`, 16 bytes
/// apart).
pub const TARGET_TYPES: u32 = 0x00c1_a924;
/// How many [`TARGET_TYPES`] records there are.
pub const TARGET_TYPE_COUNT: u32 = 19;
/// The static array `0x0053a070` resets: begin, end, capacity, extra
/// (`0x00cacd00..0x00cacd0c`) and the backing `PArray` count/data
/// (`0x00cacedc`, `0x00cacee0`).
pub const STATIC_BEGIN: u32 = 0x00ca_cd00;
/// See [`STATIC_BEGIN`].
pub const STATIC_COUNT: u32 = 0x00ca_cedc;
/// The element copier `0x00486ff0` calls (cdecl `(dst, src, n)`,
/// `0x14`-byte elements).
pub const SEGMENT_COPY: u32 = 0x0048_0b60;
/// How many channel-class slots the batch key bit set covers
/// (`0x00516190`'s `alloca`).
pub const KEY_SLOTS: u32 = 0x00ca_9738;
/// `PAnimationChannelBase`'s copy constructor (thiscall `(src)`).
pub const CHANNEL_BASE_COPY: u32 = 0x0051_30f0;
/// `PAnimationChannelBase`'s constructor (thiscall).
pub const CHANNEL_BASE_CTOR: u32 = 0x0050_d150;
/// `PArray<T>::assign(data, n)` (thiscall).
pub const ARRAY_ASSIGN: u32 = 0x0051_7270;
/// `Phyre::PCluster::deleteObject(obj)` (thiscall).
pub const CLUSTER_DELETE: u32 = 0x0044_1240;
/// `Phyre::PCluster::create(count, descriptor, size)` (thiscall).
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// The class descriptor `0x00516240` creates its object with.
pub const SET_DATA_DESCRIPTOR: u32 = 0x00ca_a558;
/// The set data's constructor `0x005114b0` (thiscall `(arg, set)`).
pub const SET_DATA_CTOR: u32 = 0x0051_14b0;
/// The target key hash `0x005ccd70` (cdecl `(target)`).
pub const TARGET_HASH: u32 = 0x005c_cd70;
/// `Phyre_PRedBlackTree_postInsertFinalize` (thiscall `(node)`).
pub const TREE_FIX: u32 = 0x0057_4980;
/// "" (the name of a target without one).
pub const EMPTY_NAME: u32 = 0x00b3_8620;
/// "Hash collision inserting animation set target lookup entry for %s".
pub const COLLISION_FMT: u32 = 0x00b3_8630;

/// A `PString`-style word: the text, without its "not owned" bit 0.
pub fn text(m: &impl Mem, at: u32) -> u32 {
    let w: u32 = m.read(at);
    w.wrapping_sub(w & 1)
}

/// The originals' inline `strcmp`: -1, 0 or 1 by the first differing
/// unsigned byte.
pub fn strcmp(m: &impl Mem, a: u32, b: u32) -> i32 {
    let mut i = 0u32;
    loop {
        let x: u8 = m.read(a.wrapping_add(i));
        let y: u8 = m.read(b.wrapping_add(i));
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            return 0;
        }
        i = i.wrapping_add(1);
    }
}

/// The inline `strlen`.
pub fn strlen(m: &impl Mem, s: u32) -> u32 {
    let mut n = 0u32;
    while m.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

pub mod bridge {
    use super::{
        ARRAY_ASSIGN, CHANNEL_BASE_COPY, CHANNEL_BASE_CTOR, CI_ACOS, CI_SIN, CLUSTER_CREATE,
        CLUSTER_DELETE, COLLISION_FMT, CURRENT_POSE, Conv, DELETE_FREE_LIST, EMPTY_NAME,
        HAS_MEMBER, Host, MEM_DUP, MEMCPY, Mem, NOT_OWNED, PHYRE_FREE, PHYRE_MALLOC,
        POSE_DESCRIPTOR, PRINTF, SEGMENT_COPY, SET_DATA_CTOR, SET_DATA_DESCRIPTOR, STATIC_BEGIN,
        STATIC_COUNT, TARGET_HASH, TARGET_TYPE_COUNT, TARGET_TYPES, TREE_FIX, UNLINK,
        UNREGISTERED_FMT, strcmp, strlen, text, x87,
    };

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    fn malloc(h: &mut impl Host, size: u32, align: u32) -> u32 {
        h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, align])
    }

    fn free(h: &mut impl Host, p: u32) {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
    }

    /// `0x00493f50` (`thiscall(name)`, `ret 4`): the index of `name` among
    /// the names at `[this + 0x24]` (count at `+0x20`), -1 when absent or
    /// for a null `name`. Null names in the array are skipped.
    pub fn find_name(m: &impl Mem, this: u32, name: u32) -> u32 {
        if name == 0 {
            return u32::MAX;
        }
        let n = count(m, this.wrapping_add(0x20));
        let names: u32 = m.read(this.wrapping_add(0x24));
        for i in 0..n {
            let s = text(m, names.wrapping_add(i.wrapping_mul(4)));
            if s != 0 && strcmp(m, name, s) == 0 {
                return i;
            }
        }
        u32::MAX
    }

    /// `0x0050c850` (`thiscall(descriptor, obj, name, index)`, `ret 0x10`):
    /// `PAnimationChannelTargetBindDataMember`'s target set: 1 without a
    /// name; warns when the class has no such member, copies the name
    /// (0xd when that fails), frees the old one when owned, and stores the
    /// descriptor, object, index and state 2 (`+8`, `+0xc` cleared); 0.
    pub fn set_target(
        h: &mut impl Host,
        this: u32,
        descriptor: u32,
        obj: u32,
        name: u32,
        index: u32,
    ) -> u32 {
        if name == 0 {
            return 1;
        }
        let ok = h.call_sym(HAS_MEMBER, Conv::Cdecl, &[descriptor, name]);
        if ok.trailing_zeros() >= 8 {
            let class: u32 = h.read(descriptor.wrapping_add(0x18));
            let fmt = h.rebase(UNREGISTERED_FMT);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, fmt, name, class]);
        }
        let len = strlen(h, name).wrapping_add(1);
        let copy = malloc(h, len, 2);
        if copy == 0 {
            return 0xd;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[copy, name, len]);
        let old: u32 = h.read(this.wrapping_add(0x14));
        if old & 1 == 0 {
            free(h, old);
        }
        h.write(this.wrapping_add(0x14), copy);
        h.write(this, descriptor);
        h.write(this.wrapping_add(4), obj);
        h.write(this.wrapping_add(0x18), index);
        h.write(this.wrapping_add(8), 0u32);
        h.write(this.wrapping_add(0xc), 0u32);
        h.write(this.wrapping_add(0x10), 2u32);
        0
    }

    /// `0x00432e90` (cdecl `(bind, owner)`): rebinds a data-member target
    /// to `owner`'s `m_currentPose`. The name looked up in `[owner]`'s
    /// names is the target's own, or, when it already is `m_currentPose`
    /// of the pose descriptor with an object, that object's class's name
    /// at the target's index. 0 when the name is null or not found, else
    /// [`set_target`]'s result.
    pub fn bind_current_pose(h: &mut impl Host, bind: u32, owner: u32) -> u32 {
        let mut name = text(h, bind.wrapping_add(0x14));
        if name == 0 {
            return 0;
        }
        let pose_name = h.rebase(CURRENT_POSE);
        let pose = h.rebase(POSE_DESCRIPTOR);
        if strcmp(h, name, pose_name) == 0 && h.read::<u32>(bind) == pose {
            let obj: u32 = h.read(bind.wrapping_add(4));
            if obj != 0 {
                let class: u32 = h.read(obj);
                let i: u32 = h.read(bind.wrapping_add(0x18));
                if i < count(h, class.wrapping_add(0x18)) {
                    let names: u32 = h.read(class.wrapping_add(0x24));
                    name = text(h, names.wrapping_add(i.wrapping_mul(4)));
                }
            }
        }
        let list: u32 = h.read(owner);
        let i = find_name(h, list, name);
        if i == u32::MAX {
            return 0;
        }
        set_target(h, bind, pose, owner, pose_name, i)
    }

    /// `0x0053a070` (cdecl, 0): empties the static array (begin, end,
    /// capacity and its fourth word zero when begin is the backing
    /// store's data), then releases the backing `PArray` (freed when
    /// owned and set) when its count is non-zero.
    pub fn reset_static(h: &mut impl Host) -> u32 {
        let begin = h.rebase(STATIC_BEGIN);
        let cnt = h.rebase(STATIC_COUNT);
        let data: u32 = h.read(cnt.wrapping_add(4));
        if h.read::<u32>(begin) == data {
            h.write(begin.wrapping_add(4), 0u32);
            h.write(begin, 0u32);
            h.write(begin.wrapping_add(8), 0u32);
            h.write(begin.wrapping_add(0xc), 0u32);
        }
        let n: u32 = h.read(cnt);
        if n != 0 {
            if data != 0 && n & NOT_OWNED == 0 {
                free(h, data);
            }
            h.write(cnt.wrapping_add(4), 0u32);
            h.write(cnt, 0u32);
        }
        0
    }

    /// `0x00433250` (cdecl): the animation module's shutdown: the free
    /// list, the static array ([`reset_static`]), then the channel-target
    /// type records unlinked one by one until an unlink fails; the last
    /// unlink's `eax`.
    pub fn shutdown(h: &mut impl Host) -> u32 {
        h.call_sym(DELETE_FREE_LIST, Conv::Cdecl, &[]);
        reset_static(h);
        let mut r = 0;
        for k in 0..TARGET_TYPE_COUNT {
            let rec = h.rebase(TARGET_TYPES.wrapping_add(k * 0x10));
            r = h.call_sym(UNLINK, Conv::Cdecl, &[rec]);
            if r != 0 {
                break;
            }
        }
        r
    }

    /// The storage of a `PArray` with one element in place (`[a]` count,
    /// `+4` the element or the pointer): the pointer past one element,
    /// `a + 4` for one, 0 for none.
    fn small_data(m: &impl Mem, a: u32) -> u32 {
        let n = count(m, a);
        if n > 1 {
            m.read(a.wrapping_add(4))
        } else if n != 0 {
            a.wrapping_add(4)
        } else {
            0
        }
    }

    /// `0x00486ff0` (`thiscall(src)`, `ret 4`): copy constructor of a
    /// `PArray` of `0x14`-byte elements that keeps one element in place:
    /// count 0, storage allocated past one element, the elements copied
    /// ([`SEGMENT_COPY`]), the old storage freed when it is neither the new
    /// one nor in place and owned, then the count and (when not in place)
    /// the pointer stored; `this`.
    pub fn small_array_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        h.write(this, 0u32);
        let n = count(h, src);
        let from = small_data(h, src);
        let inline = this.wrapping_add(4);
        let mut to = inline;
        if n > 1 {
            to = malloc(h, n.wrapping_mul(0x14), 4);
        }
        if to != 0 {
            h.call_sym(SEGMENT_COPY, Conv::Cdecl, &[to, from, n]);
        }
        let raw: u32 = h.read(this);
        let old = small_data(h, this);
        if old != to && old != inline && raw & NOT_OWNED == 0 && old != 0 {
            free(h, old);
        }
        h.write(this, n);
        if to != inline {
            h.write(inline, to);
        }
        this
    }

    /// A channel's batch rank for [`compare_batch`]: its type's `+0xc`
    /// (0 without a type) in the top byte, or'ed with its `+8`.
    fn batch_rank(m: &impl Mem, ch: u32) -> u32 {
        let ty: u32 = m.read(ch);
        let hi = if ty == 0 {
            0
        } else {
            m.read::<u32>(ty.wrapping_add(0xc))
        };
        hi << 24 | m.read::<u32>(ch.wrapping_add(8))
    }

    /// `0x00508ba0` (cdecl `(a, b)`): the sort order of two channels by
    /// [`batch_rank`], highest first: -1 when `a` ranks higher, 1 when
    /// lower, 0 when equal.
    pub fn compare_batch(m: &impl Mem, a: u32, b: u32) -> u32 {
        let (ra, rb) = (batch_rank(m, a), batch_rank(m, b));
        if ra > rb {
            u32::MAX
        } else {
            u32::from(ra < rb)
        }
    }

    /// `0x00508f00` (cdecl `(dst, src, n)`): the placement copy of `n`
    /// floats (`fld`/`fstp`, so signalling NaNs come out quiet). A null
    /// destination slot takes nothing and the source stays; `n` counts
    /// as unsigned. Returns the end of the destination.
    pub fn copy_floats(m: &mut impl Mem, dst: u32, src: u32, n: u32) -> u32 {
        let (mut d, mut s) = (dst, src);
        for _ in 0..n {
            if d != 0 {
                let v = x87::store(x87::load(m.read(s)));
                m.write(d, v);
                s = s.wrapping_add(4);
            }
            d = d.wrapping_add(4);
        }
        d
    }

    /// `0x0050c7e0` (`thiscall(begin, end)`, `ret 8`): the name at `this`
    /// set to the text `begin..end`: 1 when either is null, 5 when `end`
    /// is before `begin`, 0xd when the copy fails; the old name freed when
    /// owned (bit 0 clear); 0.
    pub fn assign_range(h: &mut impl Host, this: u32, begin: u32, end: u32) -> u32 {
        if begin == 0 || end == 0 {
            return 1;
        }
        if end < begin {
            return 5;
        }
        let len = end - begin;
        let copy = h.call_sym(MEM_DUP, Conv::Cdecl, &[begin, len.wrapping_add(1)]);
        if copy == 0 {
            return 0xd;
        }
        h.write(copy.wrapping_add(len), 0u8);
        let old: u32 = h.read(this);
        if old & 1 == 0 {
            free(h, old);
        }
        h.write(this, copy);
        0
    }

    fn vec4(m: &impl Mem, at: u32) -> [u32; 4] {
        [0, 4, 8, 0xc].map(|o| m.read(at.wrapping_add(o)))
    }

    /// `0x0050eb70` / `0x0050ebf0` (cdecl `(out, t, a, b)`): `out = (b -
    /// a) * t + a` per component, each difference, product and sum stored
    /// as `f32` (the last sum adds the product to `a`); `out`.
    pub fn lerp4(m: &mut impl Mem, out: u32, t: u32, a: u32, b: u32) -> u32 {
        let (va, vb) = (vec4(m, a), vec4(m, b));
        let t = x87::load(t);
        let d = [0, 1, 2, 3].map(|i| x87::store(x87::sub(x87::load(vb[i]), x87::load(va[i]))));
        let p = [0, 1, 2].map(|i| x87::store(x87::mul(x87::load(d[i]), t)));
        let p3 = x87::store(x87::mul(t, x87::load(d[3])));
        for (i, (pi, ai)) in (0u32..).zip(p.iter().zip(va)) {
            let s = x87::store(x87::add(x87::load(*pi), x87::load(ai)));
            m.write(out.wrapping_add(4 * i), s);
        }
        let s = x87::store(x87::add(x87::load(va[3]), x87::load(p3)));
        m.write(out.wrapping_add(0xc), s);
        out
    }

    /// A CRT intrinsic's result as the caller's `fstp m32` / `fld m32`
    /// leave it.
    fn intrinsic(h: &mut impl Host, f: u32, x: f64) -> f64 {
        let at = h.rebase(f);
        x87::load(h.call_intrinsic(at, x).to_bits())
    }

    fn f32_of(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    /// `0x0050ed70` (cdecl `(out, t, a, b, threshold)`): quaternion slerp
    /// from `a` to `b`. The dot product is summed with an `f32` store per
    /// term; below zero `a` and the dot are negated. When `threshold` is
    /// not above the cosine (or either is NaN) the weights are `1 - t` and
    /// `t`; else, with `theta = acos(cos)`, `sin((1 - t) theta) / sin
    /// theta` and `sin(t theta) / sin theta` (`_CIacos`, `_CIsin`; every
    /// intermediate an `f32`). `out = a' * w0 + b * w1` per component
    /// (`b`'s product first in the sum's operand order); `out`.
    pub fn slerp(h: &mut impl Host, out: u32, t: u32, a: u32, b: u32, threshold: u32) -> u32 {
        let (va, vb) = (vec4(h, a), vec4(h, b));
        let mut dot = x87::store(x87::mul(x87::load(va[0]), x87::load(vb[0])));
        for i in 1..4 {
            let term = x87::mul(x87::load(va[i]), x87::load(vb[i]));
            dot = x87::store(x87::add(x87::load(dot), term));
        }
        let mut cos = x87::load(dot);
        let mut qa = va.map(x87::load);
        if cos < 0.0 {
            cos = f32_of(-cos);
            qa = va.map(|w| f32_of(-x87::load(w)));
        } else {
            qa = qa.map(f32_of);
        }
        let t = x87::load(t);
        let threshold = x87::load(threshold);
        let (w0, w1) = if threshold > cos {
            let theta = intrinsic(h, CI_ACOS, cos);
            let sin = intrinsic(h, CI_SIN, theta);
            let inv = f32_of(x87::div(1.0, sin));
            let x = f32_of(x87::mul(x87::sub(1.0, t), theta));
            let s0 = intrinsic(h, CI_SIN, x);
            let w0 = f32_of(x87::mul(s0, inv));
            let y = f32_of(x87::mul(theta, t));
            let s1 = intrinsic(h, CI_SIN, y);
            (w0, f32_of(x87::mul(s1, inv)))
        } else {
            (f32_of(x87::sub(1.0, t)), t)
        };
        let pb = vb.map(|q| f32_of(x87::mul(x87::load(q), w1)));
        let pa = qa.map(|q| f32_of(x87::mul(q, w0)));
        for (i, (x, y)) in (0u32..).zip(pa.into_iter().zip(pb)) {
            h.write(out.wrapping_add(4 * i), x87::store(x87::add(x, y)));
        }
        out
    }

    /// `0x00516190` (`thiscall`): how many distinct batch sort keys
    /// (`0x0050e730`) the channels at `[this + 8]` (count at `+4`, read
    /// each step) have. The original marks them in a zeroed `alloca` bit
    /// set sized from [`KEY_SLOTS`] (`memset` through the CRT); keys past
    /// it would land in its frame, here they are counted like the rest.
    pub fn distinct_keys(h: &mut impl Host, this: u32) -> u32 {
        let mut seen: Vec<u32> = Vec::new();
        let mut distinct = 0u32;
        let mut i = 0u32;
        while i < count(h, this.wrapping_add(4)) {
            let chans: u32 = h.read(this.wrapping_add(8));
            let ch: u32 = h.read(chans.wrapping_add(i.wrapping_mul(4)));
            let key = crate::m5p_anim_small::bridge::batch_sort_key(h, ch);
            let word = (key >> 5) as usize;
            if seen.len() <= word {
                seen.resize(word + 1, 0);
            }
            let bit = 1u32 << (key & 0x1f);
            if seen[word] & bit == 0 {
                seen[word] |= bit;
                distinct = distinct.wrapping_add(1);
            }
            i = i.wrapping_add(1);
        }
        distinct
    }

    /// `0x005cccd0` (cdecl `(a, b)`): the target-key order: words `+0`,
    /// `+8` .. `+0x1c` unsigned, then the names at `+4`; -1, 0 or 1.
    pub fn compare_keys(m: &impl Mem, a: u32, b: u32) -> u32 {
        for off in [0u32, 8, 0xc, 0x10, 0x14, 0x18, 0x1c] {
            let x: u32 = m.read(a.wrapping_add(off));
            let y: u32 = m.read(b.wrapping_add(off));
            if x < y {
                return u32::MAX;
            }
            if x > y {
                return 1;
            }
        }
        let (x, y) = (m.read(a.wrapping_add(4)), m.read(b.wrapping_add(4)));
        strcmp(m, x, y).cast_unsigned()
    }

    /// A tree node's key (`+0xc`), 0 for no node.
    fn key_of(node: u32) -> u32 {
        if node == 0 { 0 } else { node.wrapping_add(0xc) }
    }

    /// `0x005cc9f0` (`thiscall(node)`, `ret 4`): the lookup tree's insert
    /// (the tree header is its own null child; its `+0xc` slot holds the
    /// root): `node`'s children set to the header and its colour bit (the
    /// parent word's bit 0) set; down from the root by [`compare_keys`]
    /// (existing against new: below zero right, else left). An equal key
    /// returns that node; else `node` is linked under the last one (or as
    /// the root) and 0.
    pub fn tree_insert(m: &mut impl Mem, tree: u32, node: u32) -> u32 {
        let pw = node.wrapping_add(8);
        let w: u32 = m.read(pw);
        m.write(pw, w.wrapping_add(1 - (w & 1)));
        m.write(node, tree);
        m.write(node.wrapping_add(4), tree);
        let mut parent = tree.wrapping_add(0xc);
        let mut cur: u32 = m.read(parent);
        let mut right = false;
        while cur != tree {
            parent = cur;
            let c = compare_keys(m, key_of(cur), node.wrapping_add(0xc));
            if c == 0 {
                return cur;
            }
            right = c.cast_signed() < 0;
            cur = m.read(cur.wrapping_add(u32::from(right) * 4));
        }
        let w: u32 = m.read(pw);
        m.write(pw, (w & 1).wrapping_add(parent));
        m.write(parent.wrapping_add(u32::from(right) * 4), node);
        0
    }

    /// `0x005cc870` (`thiscall(key)`, `ret 4`): the node with `key`
    /// ([`compare_keys`] inlined: node key against `key`), 0 when absent.
    pub fn tree_find(m: &impl Mem, tree: u32, key: u32) -> u32 {
        let mut cur: u32 = m.read(tree.wrapping_add(0xc));
        while cur != tree {
            let c = compare_keys(m, key_of(cur), key);
            if c == 0 {
                return cur;
            }
            cur = m.read(cur.wrapping_add(u32::from(c.cast_signed() < 0) * 4));
        }
        0
    }

    /// `0x005cca80` (`thiscall(set)`, `ret 4`):
    /// `PAnimationSetTargetLookup::PAnimationSetTargetLookup`: `+0` the
    /// set, the tree header at `+8` (all six words pointing at itself),
    /// `0x2c`-byte nodes allocated for the set's targets (`[set + 0xc]`,
    /// `0x1c` bytes each, count at `+8`); each present target's node keyed
    /// by its hash ([`TARGET_HASH`]), name (or ""), words `+0..+0x10` and
    /// `+0x18`, inserted and rebalanced, or reported as a hash collision;
    /// `this`.
    pub fn lookup_ctor(h: &mut impl Host, this: u32, set: u32) -> u32 {
        h.write(this, set);
        let hdr = this.wrapping_add(8);
        for off in [0u32, 4, 8, 0x14, 0x10, 0xc] {
            h.write(hdr.wrapping_add(off), hdr);
        }
        let n = count(h, set.wrapping_add(8));
        let nodes = malloc(h, n.wrapping_mul(0x2c), 4);
        h.write(this.wrapping_add(4), nodes);
        if nodes == 0 {
            return this;
        }
        for i in 0..n {
            if i >= count(h, set.wrapping_add(8)) {
                continue;
            }
            let targets: u32 = h.read(set.wrapping_add(0xc));
            let src = targets.wrapping_add(i.wrapping_mul(0x1c));
            if src == 0 {
                continue;
            }
            let base: u32 = h.read(this.wrapping_add(4));
            let node = base.wrapping_add(i.wrapping_mul(0x2c));
            h.write(node, node);
            h.write(node.wrapping_add(4), node);
            h.write(node.wrapping_add(8), node);
            let mut name = text(h, src.wrapping_add(0x14));
            if name == 0 {
                name = h.rebase(EMPTY_NAME);
            }
            h.write(node.wrapping_add(0x10), name);
            for k in 0..5u32 {
                let v: u32 = h.read(src.wrapping_add(4 * k));
                h.write(node.wrapping_add(0x14 + 4 * k), v);
            }
            let v: u32 = h.read(src.wrapping_add(0x18));
            h.write(node.wrapping_add(0x28), v);
            let hash = h.call_sym(TARGET_HASH, Conv::Cdecl, &[src]);
            h.write(node.wrapping_add(0xc), hash);
            if tree_insert(h, hdr, node) == 0 {
                h.call_sym(TREE_FIX, Conv::Thiscall(hdr), &[node]);
            } else {
                let name = text(h, src.wrapping_add(0x14));
                let fmt = h.rebase(COLLISION_FMT);
                h.call_sym(PRINTF, Conv::Cdecl, &[3, fmt, name]);
            }
        }
        this
    }

    /// `0x00511b40` (cdecl `(dst, src, n)`): placement copy of `n`
    /// `0x34`-byte channels: the base copy constructor, then words
    /// `+0x1c..+0x30`. A null destination takes nothing and the source
    /// stays.
    pub fn copy_channels(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let (mut d, mut s) = (dst, src);
        for _ in 0..n {
            if d != 0 {
                h.call_sym(CHANNEL_BASE_COPY, Conv::Thiscall(d), &[s]);
                for off in (0x1c..0x34).step_by(4) {
                    let v: u32 = h.read(s.wrapping_add(off));
                    h.write(d.wrapping_add(off), v);
                }
                s = s.wrapping_add(0x34);
            }
            d = d.wrapping_add(0x34);
        }
    }

    /// `0x00515880` (cdecl `(dst, n)`): placement default construction of
    /// `n` `0x34`-byte channels: the base constructor, then `+0x30`,
    /// `+0x2c`, `+0x28`, `+0x24` = 0.0 (null slots skipped).
    pub fn construct_channels(h: &mut impl Host, dst: u32, n: u32) {
        for i in 0..n {
            let p = i.wrapping_mul(0x34).wrapping_add(dst);
            if p != 0 {
                h.call_sym(CHANNEL_BASE_CTOR, Conv::Thiscall(p), &[]);
                for off in [0x30u32, 0x2c, 0x28, 0x24] {
                    h.write(p.wrapping_add(off), 0u32);
                }
            }
        }
    }

    /// `0x00516240` (`thiscall(cluster, arg)`, `ret 8`): `PAnimationSet`'s
    /// data rebuilt: the old object (`+0`) deleted from `cluster` (both
    /// set), then one object of [`SET_DATA_DESCRIPTOR`] created in
    /// `cluster`, sized from the channel count (`+4`), the slot count
    /// (`+0xc`) and [`distinct_keys`], and constructed
    /// ([`SET_DATA_CTOR`] `(arg, this)`); stored at `+0` and returned.
    pub fn rebuild(h: &mut impl Host, this: u32, cluster: u32, arg: u32) -> u32 {
        let old: u32 = h.read(this);
        if old != 0 && cluster != 0 {
            h.call_sym(CLUSTER_DELETE, Conv::Thiscall(cluster), &[old]);
        }
        let slots = count(h, this.wrapping_add(0xc));
        let chans = count(h, this.wrapping_add(4));
        h.write(this, 0u32);
        let keys = distinct_keys(h, this);
        let words =
            (keys.wrapping_add(3) & !3).wrapping_add(slots.wrapping_add(chans).wrapping_mul(2));
        let bytes = words.wrapping_mul(2).wrapping_add(3) & !3;
        let size = bytes.wrapping_add(chans << 4).wrapping_add(0xf) & !0xf;
        let desc = h.rebase(SET_DATA_DESCRIPTOR);
        let obj = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, desc, size]);
        if obj != 0 {
            h.call_sym(SET_DATA_CTOR, Conv::Thiscall(obj), &[arg, this]);
        }
        h.write(this, obj);
        obj
    }

    /// `0x00516fe0` (`thiscall(n)`, `ret 4`): the word `PArray` at `+4`
    /// (count, data at `+8`) resized to `n`: nothing when `n` equals the
    /// raw count word; else new storage (0xd when that fails for `n` != 0,
    /// `this` untouched), the kept words copied (a null slot takes none),
    /// the rest zeroed, the old storage freed when different and owned.
    /// `+0` = 0 and 0 on success.
    pub fn resize_words(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let raw: u32 = h.read(this.wrapping_add(4));
        if n != raw {
            let keep = n.min(raw & !NOT_OWNED);
            let old: u32 = h.read(this.wrapping_add(8));
            let mut new = 0;
            if n != 0 {
                new = malloc(h, n.wrapping_mul(4), 4);
                if new == 0 {
                    return 0xd;
                }
                let (mut d, mut s) = (new, old);
                for _ in 0..keep {
                    if d != 0 {
                        let v: u32 = h.read(s);
                        h.write(d, v);
                        s = s.wrapping_add(4);
                    }
                    d = d.wrapping_add(4);
                }
                for k in keep..n {
                    h.write(new.wrapping_add(k.wrapping_mul(4)), 0u32);
                }
            }
            let cur: u32 = h.read(this.wrapping_add(8));
            let w: u32 = h.read(this.wrapping_add(4));
            if cur != new && w & NOT_OWNED == 0 {
                free(h, cur);
            }
            h.write(this.wrapping_add(8), new);
            h.write(this.wrapping_add(4), n);
        }
        h.write(this, 0u32);
        0
    }

    /// The channel `PArray` resize shared by `0x00516f20` (the array at
    /// `this`) and `0x005170c0` (at `this + 0xc`, then `+0` = 0): nothing
    /// when `n` equals the raw count word; else new storage of `n`
    /// `0x34`-byte channels (0xd when that fails for `n` != 0), the kept
    /// ones copied ([`copy_channels`]), the rest constructed
    /// ([`construct_channels`]), and the array assigned ([`ARRAY_ASSIGN`]
    /// `(new, n)`); 0.
    fn resize_channels_at(h: &mut impl Host, arr: u32, n: u32) -> u32 {
        let raw: u32 = h.read(arr);
        if n == raw {
            return 0;
        }
        let keep = n.min(raw & !NOT_OWNED);
        let old: u32 = h.read(arr.wrapping_add(4));
        let mut new = 0;
        if n != 0 {
            new = malloc(h, n.wrapping_mul(0x34), 4);
            if new == 0 {
                return 0xd;
            }
            copy_channels(h, new, old, keep);
            construct_channels(h, keep.wrapping_mul(0x34).wrapping_add(new), n - keep);
        }
        h.call_sym(ARRAY_ASSIGN, Conv::Thiscall(arr), &[new, n]);
        0
    }

    /// `0x00516f20` (`thiscall(n)`, `ret 4`): see [`resize_channels_at`].
    pub fn resize_channels(h: &mut impl Host, this: u32, n: u32) -> u32 {
        resize_channels_at(h, this, n)
    }

    /// `0x005170c0` (`thiscall(n)`, `ret 4`): [`resize_channels_at`] on the
    /// array at `+0xc`, then `+0` = 0 unless the allocation failed.
    pub fn resize_set_channels(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let r = resize_channels_at(h, this.wrapping_add(0xc), n);
        if r == 0 {
            h.write(this, 0u32);
        }
        r
    }

    /// `0x00517800` (cdecl `(dst, src, n)`): placement copy of `n`
    /// `0x1c`-byte named targets: words `+0..+0x10` and `+0x18`, the name
    /// (`+0x14`, bit 0 off) duplicated with `PhyreMalloc(len + 1, 2)` and
    /// `memcpy` (0 for no name; the failed allocation's 0 kept). A null
    /// destination takes nothing and the source stays.
    pub fn copy_targets(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let (mut d, mut s) = (dst, src);
        for _ in 0..n {
            if d != 0 {
                for off in [0u32, 4, 8, 0xc, 0x10] {
                    let v: u32 = h.read(s.wrapping_add(off));
                    h.write(d.wrapping_add(off), v);
                }
                let name = text(h, s.wrapping_add(0x14));
                let mut copy = 0;
                if name != 0 {
                    let len = strlen(h, name).wrapping_add(1);
                    copy = malloc(h, len, 2);
                    if copy != 0 {
                        h.call_sym(MEMCPY, Conv::Cdecl, &[copy, name, len]);
                    }
                }
                h.write(d.wrapping_add(0x14), copy);
                let v: u32 = h.read(s.wrapping_add(0x18));
                h.write(d.wrapping_add(0x18), v);
                s = s.wrapping_add(0x1c);
            }
            d = d.wrapping_add(0x1c);
        }
    }
}
