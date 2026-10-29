//! List `phyre-anim-left`, batch 2 (M5 stage 1): the controllers' and
//! blenders' methods.
//!
//! - `PAnimationEventController`'s event dispatch over a time interval
//!   (with wrap-around), `PTimeIntervalController`'s looped time
//!   (`_CIfmod`), the key-time binary search;
//! - the batch headers the controllers and blenders write for the
//!   animation batch jobs, and their sizes;
//! - the blenders: deleting destructor, visit, source and weight array
//!   resizes, the "used slot" marks of weighted and additive blending,
//!   the batch runner that hands each batch's sources to a blend kernel;
//! - quaternion product, normalisation and the rotation matrix of a
//!   quaternion (Vectormath on the x87 at 53 bits; every store an `f32`
//!   in the original's order, the matrix's `1 - xx` kept as a double);
//! - the target blender's log forwarder (a `va_list` handed on).
//!
//! Callees outside the list stay calls to the original. No state: the
//! batch runner's source list lives in the caller's frame (`list`).

#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_left_a::bridge::copy_floats;
use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE, TIME_CONSTANT};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `_CIfmod` (import thunk; `x` in `ST1`, `y` in `ST0`).
pub const CI_FMOD: u32 = 0x0094_98e0;
/// `_CIsqrt` (import thunk).
pub const CI_SQRT: u32 = 0x0094_97b8;
/// The f32 -1.0 `0x00522cd0` loads for a negative interval.
pub const MINUS_ONE: u32 = 0x00b9_2340;
/// `PAnimationBlenderController`'s vtable (`0x0052bb60` stores it).
pub const BLENDER_VTABLE: u32 = 0x00b2_96d0;
/// `PAnimationDataSource::~PAnimationDataSource` (thiscall).
pub const DATA_SOURCE_DTOR: u32 = 0x0052_3d60;
/// `PArray<PAnimationDataSource *>`'s allocating copy (cdecl `(total,
/// src, n)`).
pub const SOURCES_ALLOC_COPY: u32 = 0x0052_bc40;
/// The channel-target type records `0x00530750` names (rotation, scale,
/// translation).
pub const ROTATION_TYPE: u32 = 0x00c1_a924;
/// See [`ROTATION_TYPE`].
pub const SCALE_TYPE: u32 = 0x00c1_a944;
/// See [`ROTATION_TYPE`].
pub const TRANSLATION_TYPE: u32 = 0x00c1_a934;
/// How many words of the caller's frame the batch runner's source list
/// may use before it allocates (`0x0052c8f0`'s `alloca`).
pub const LIST_WORDS: u32 = 128;

/// The weight-array copies: weighted (`0x0052e550`, `0x0052e3f0`,
/// `0x0052e2f0`) and additive (`0x0052fbf0`, `0x0052fa90`, `0x0052f8a0`)
/// blenders share these bodies (identical but for addresses).
pub const RESIZE_WEIGHTS: &[u32] = &[0x0052_e550, 0x0052_fbf0];
/// See [`RESIZE_WEIGHTS`].
pub const WEIGHT_BATCH_DATA: &[u32] = &[0x0052_e3f0, 0x0052_fa90];

pub mod bridge {
    use super::{
        BLENDER_VTABLE, CI_FMOD, CI_SQRT, Conv, DATA_SOURCE_DTOR, Host, LIST_WORDS, MINUS_ONE, Mem,
        NOT_OWNED, PHYRE_FREE, PHYRE_MALLOC, ROTATION_TYPE, SCALE_TYPE, SOURCES_ALLOC_COPY,
        TIME_CONSTANT, TRANSLATION_TYPE, copy_floats, x87,
    };

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    fn f32_of(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn ld(m: &impl Mem, at: u32) -> f64 {
        x87::load(m.read(at))
    }

    /// `al` set to `bit` over `eax`.
    const fn with_al(eax: u32, bit: bool) -> u32 {
        eax & !0xff | bit as u32
    }

    /// `0x005207c0` (`thiscall(from, to, callback, ctx)`, `ret 0x10`):
    /// `PAnimationEventController`'s events (`[this + 4]`, 8 bytes each,
    /// the time first; count at `+0`) between `from` and `to`: 1 without a
    /// callback. From the first event not before `from`: when `to` is
    /// before `from` (the interval wrapped) every event to the end, then
    /// from the start; then each event up to `to` (not past it, not NaN).
    /// The callback is cdecl `(event, ctx)`; 0.
    pub fn dispatch_events(
        h: &mut impl Host,
        this: u32,
        from: u32,
        to: u32,
        callback: u32,
        ctx: u32,
    ) -> u32 {
        if callback == 0 {
            return 1;
        }
        let n = count(h, this);
        let (from, to) = (x87::load(from), x87::load(to));
        let mut i = 0u32;
        while i < n {
            let evs: u32 = h.read(this.wrapping_add(4));
            // not before `from` (NaN stops too)
            if ld(h, evs.wrapping_add(i.wrapping_mul(8))).partial_cmp(&from)
                != Some(std::cmp::Ordering::Less)
            {
                break;
            }
            i += 1;
        }
        if to < from {
            while i < n {
                let evs: u32 = h.read(this.wrapping_add(4));
                h.call(
                    callback,
                    Conv::Cdecl,
                    &[evs.wrapping_add(i.wrapping_mul(8)), ctx],
                );
                i += 1;
            }
            i = 0;
        }
        while i < n {
            let evs: u32 = h.read(this.wrapping_add(4));
            let ev = evs.wrapping_add(i.wrapping_mul(8));
            // past `to`, or NaN
            if !matches!(
                ld(h, ev).partial_cmp(&to),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            ) {
                break;
            }
            h.call(callback, Conv::Cdecl, &[ev, ctx]);
            i += 1;
        }
        0
    }

    /// `0x00522cd0` (`thiscall`, `ST0`): `PTimeIntervalController`'s time:
    /// the source's time (its slot 4; the constant time without one) less
    /// the start (`+0xc`), wrapped into the interval (`+0x14`, by its
    /// absolute value through `_CIfmod`, a negative remainder lifted by
    /// it), turned back by the interval's sign, plus the offset (`+0x10`).
    pub fn interval_time(h: &mut impl Host, this: u32) -> f32 {
        let src: u32 = h.read(this.wrapping_add(8));
        let time = if src == 0 {
            let at = h.rebase(TIME_CONSTANT);
            ld(h, at)
        } else {
            let vt: u32 = h.read(src);
            let f: u32 = h.read(vt.wrapping_add(0x10));
            x87::load(h.call_st0(f, Conv::Thiscall(src), &[]).to_bits())
        };
        let time = f32_of(time);
        let len = ld(h, this.wrapping_add(0x14));
        let sign = if len < 0.0 {
            let at = h.rebase(MINUS_ONE);
            ld(h, at)
        } else {
            1.0
        };
        let sign = f32_of(sign);
        let span = f32_of(x87::mul(len, sign));
        let x = f32_of(x87::sub(time, ld(h, this.wrapping_add(0xc))));
        let at = h.rebase(CI_FMOD);
        let mut r = x87::load(h.call_intrinsic2(at, x, span).to_bits());
        if 0.0 > r {
            r = f32_of(x87::add(r, span));
        }
        let v = x87::add(x87::mul(r, sign), ld(h, this.wrapping_add(0x10)));
        f32::from_bits(x87::store(v))
    }

    /// `0x005253c0` (cdecl `(keys, n, value)`): the index of the last key
    /// time not after `value` in the sorted `f32` times `keys` (a binary
    /// search on signed indices; ties and NaN go right), 0 and below kept
    /// as they come.
    pub fn key_search(m: &impl Mem, keys: u32, n: u32, value: u32) -> u32 {
        let v = x87::load(value);
        let key = |i: i32| ld(m, keys.wrapping_add(i.cast_unsigned().wrapping_mul(4)));
        let mut hi = n.cast_signed().wrapping_sub(1);
        let mut lo = 0i32;
        let mut mid = hi / 2;
        if hi != 0 {
            loop {
                if key(mid) > v {
                    hi = mid.wrapping_sub(1).max(lo);
                } else {
                    lo = mid.wrapping_add(1).min(hi);
                }
                mid = hi.wrapping_add(lo) / 2;
                if lo == hi {
                    break;
                }
            }
        }
        if mid > 0 && key(mid) > v {
            return mid.wrapping_sub(1).cast_unsigned();
        }
        mid.cast_unsigned()
    }

    /// `0x005262c0` (`thiscall`): the largest batch size of the clips in
    /// the `PArray` at `[this + 8]` (one clip kept in place): per clip
    /// with `k` channels (`+4`) and constant channels (`+0xc`) or not,
    /// `((2k + 1 (+1)) rounded to even) * 8 + 0x1f` and `6 (k - 1) +
    /// 0x17`, each down to 16. 0 for none.
    pub fn max_batch_size(m: &impl Mem, this: u32) -> u32 {
        let arr: u32 = m.read(this.wrapping_add(8));
        let n = count(m, arr);
        let mut best = 0u32;
        for i in 0..n {
            let cur = count(m, arr);
            let clip: u32 = if i >= cur {
                0
            } else {
                let base = if cur > 1 {
                    m.read(arr.wrapping_add(4))
                } else {
                    arr.wrapping_add(4)
                };
                m.read(base.wrapping_add(i.wrapping_mul(4)))
            };
            let k = count(m, clip.wrapping_add(4));
            let mut words = k.wrapping_mul(2).wrapping_add(1);
            if count(m, clip.wrapping_add(0xc)) != 0 {
                words = words.wrapping_add(1);
            }
            let even = words.wrapping_add(1) & !1;
            let a = even.wrapping_mul(8).wrapping_add(0x1f) & !0xf;
            let b = k.wrapping_sub(1).wrapping_mul(6).wrapping_add(0x17) & !0xf;
            let size = b.wrapping_add(a);
            if best <= size {
                best = size;
            }
        }
        best
    }

    /// `0x00526480` (`thiscall(_, out, _)`, `ret 0xc`):
    /// `PAnimationController`'s batch header: `out[0..4] = 0x20, 2, 0,
    /// 0`; the time (the time controller's slot 4 at `+0x10`, else 0) at
    /// `+0x20`; from the clip (`+0xc`): its channel count (`u16` at
    /// `+0x24`), its constant channel count (`u16` at `+0x26`), its first
    /// word's low half (`+0x10`), the clip (`+0x14`), its `0x34`-byte
    /// constant channels' size rounded to 16 (`+0x18`) and its `+0x10`
    /// (`+0x1c`); 0x30.
    pub fn controller_header(h: &mut impl Host, this: u32, out: u32) -> u32 {
        let clip: u32 = h.read(this.wrapping_add(0xc));
        let first: u32 = h.read(clip);
        let chans = count(h, clip.wrapping_add(4));
        let consts = count(h, clip.wrapping_add(0xc));
        for (v, k) in [0x20u32, 2, 0, 0].into_iter().zip(0u32..) {
            h.write(out.wrapping_add(4 * k), v);
        }
        let ctl: u32 = h.read(this.wrapping_add(0x10));
        let time = if ctl == 0 {
            0
        } else {
            let vt: u32 = h.read(ctl);
            let f: u32 = h.read(vt.wrapping_add(0x10));
            x87::store(x87::load(h.call_st0(f, Conv::Thiscall(ctl), &[]).to_bits()))
        };
        #[allow(clippy::cast_possible_truncation)] // the u16 stores
        {
            h.write(out.wrapping_add(0x26), consts as u16);
            h.write(out.wrapping_add(0x20), time);
            h.write(out.wrapping_add(0x24), chans as u16);
        }
        let low = u32::from(h.read::<u16>(first));
        h.write(out.wrapping_add(0x10), low);
        h.write(out.wrapping_add(0x14), first);
        let tail: u32 = h.read(clip.wrapping_add(0x10));
        h.write(out.wrapping_add(0x1c), tail);
        h.write(
            out.wrapping_add(0x18),
            consts.wrapping_mul(0x34).wrapping_add(0xf) & !0xf,
        );
        0x30
    }

    /// `0x0052bb60` (`thiscall(flags)`, `ret 4`):
    /// `PAnimationBlenderController`'s scalar deleting destructor: its
    /// vtable, the sources array (`+0xc` count, `+0x10` data) freed when
    /// owned and cleared, the data-source destructor, `PhyreFree(this)`
    /// when `flags & 1`; `this`.
    pub fn blender_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(BLENDER_VTABLE);
        h.write(this, vt);
        if h.read::<u32>(this.wrapping_add(0xc)) & NOT_OWNED == 0 {
            let p: u32 = h.read(this.wrapping_add(0x10));
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        h.write(this.wrapping_add(0x10), 0u32);
        h.write(this.wrapping_add(0xc), 0u32);
        h.call_sym(DATA_SOURCE_DTOR, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x0052c8f0` (cdecl `(out, sources, n, a, b, batches, kernel)`):
    /// the blenders' batch runner. For each batch (`batches[0]` the
    /// 16-byte records, `[batches + 4]` how many, re-read), the `n`
    /// sources' batch entries (`[[sources + 4k]] + 0x20 b`) with a non-zero
    /// `+0x10` go into the list (the others as 0) and the largest `+0x14`
    /// (signed) is taken; when any is there, `kernel(out entry, largest,
    /// list, n, a, b)` (cdecl; `al`) and on success the out entry's `+0x10`,
    /// `+0x14`, `+0x18` = the record's `+0`, the largest, the record's
    /// `+8`. Out entries are `0x20` bytes from `[out]`. `list` holds the
    /// caller's [`LIST_WORDS`] words; a longer list is allocated
    /// (`PhyreMalloc`) and freed. 0.
    #[allow(clippy::too_many_arguments)]
    pub fn blend_batches(
        h: &mut impl Host,
        out: u32,
        sources: u32,
        n: u32,
        a: u32,
        b: u32,
        batches: u32,
        kernel: u32,
        list: u32,
    ) -> u32 {
        let heap = n > LIST_WORDS;
        let list = if heap {
            h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4])
        } else {
            list
        };
        let mut entry: u32 = h.read(out);
        let mut i = 0u32;
        while i < h.read::<u32>(batches.wrapping_add(4)) {
            let mut any = false;
            let mut largest = 0i32;
            for k in 0..n {
                let src: u32 = h.read(sources.wrapping_add(k.wrapping_mul(4)));
                let base: u32 = h.read(src);
                let e = base.wrapping_add(i.wrapping_mul(0x20));
                let slot = list.wrapping_add(k.wrapping_mul(4));
                h.write(slot, 0u32);
                if h.read::<u32>(e.wrapping_add(0x10)) != 0 {
                    h.write(slot, e);
                    let size: u32 = h.read(e.wrapping_add(0x14));
                    if largest <= size.cast_signed() {
                        largest = size.cast_signed();
                    }
                    any = true;
                }
            }
            if any {
                let size = largest.cast_unsigned();
                let r = h.call(kernel, Conv::Cdecl, &[entry, size, list, n, a, b]);
                if r.trailing_zeros() < 8 {
                    let recs: u32 = h.read(batches);
                    let rec = recs.wrapping_add(i.wrapping_mul(0x10));
                    let first: u32 = h.read(rec);
                    let third: u32 = h.read(rec.wrapping_add(8));
                    h.write(entry.wrapping_add(0x10), first);
                    h.write(entry.wrapping_add(0x14), size);
                    h.write(entry.wrapping_add(0x18), third);
                }
            }
            entry = entry.wrapping_add(0x20);
            i = i.wrapping_add(1);
        }
        if heap {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[list]);
        }
        0
    }

    /// `0x0052cf10` (`thiscall(n)`, `ret 4`): the blender's sources array
    /// (`+0xc` count, `+0x10` data) resized to `n`: nothing when the count
    /// is `n`; unless `n` is the raw count word, new storage with the kept
    /// pointers ([`SOURCES_ALLOC_COPY`]; 0xd when that fails for `n` !=
    /// 0), the old freed when different and owned; the new slots zeroed;
    /// 0.
    pub fn resize_sources(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let raw: u32 = h.read(this.wrapping_add(0xc));
        let have = raw & !NOT_OWNED;
        if have == n {
            return 0;
        }
        if n != raw {
            let old: u32 = h.read(this.wrapping_add(0x10));
            let p = h.call_sym(SOURCES_ALLOC_COPY, Conv::Cdecl, &[n, old, n.min(have)]);
            if p == 0 && n != 0 {
                return 0xd;
            }
            let cur: u32 = h.read(this.wrapping_add(0x10));
            if cur != p && h.read::<u32>(this.wrapping_add(0xc)) & NOT_OWNED == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[cur]);
            }
            h.write(this.wrapping_add(0x10), p);
            h.write(this.wrapping_add(0xc), n);
        }
        let mut i = have;
        while i < n {
            let data: u32 = h.read(this.wrapping_add(0x10));
            h.write(data.wrapping_add(i.wrapping_mul(4)), 0u32);
            i += 1;
        }
        0
    }

    /// `0x0052d180` (`thiscall(visitor, ctx)`, `ret 8`): the blender's
    /// visit: the visitor's begin (slot 1, `(this, ctx)`); each non-null
    /// source's visit (slot 3, `(visitor, this)`); the visitor's end
    /// (slot 3, `(this, ctx)`). `al` 1 when everything went on, else 0
    /// over the `eax` of the call that stopped.
    pub fn blender_visit(h: &mut impl Host, this: u32, visitor: u32, ctx: u32) -> u32 {
        let r = h.call_virtual(visitor, 1, &[this, ctx]);
        if r.trailing_zeros() >= 8 {
            return r;
        }
        let n = count(h, this.wrapping_add(0xc));
        for i in 0..n {
            let data: u32 = h.read(this.wrapping_add(0x10));
            let src: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
            if src != 0 {
                let r = h.call_virtual(src, 3, &[visitor, this]);
                if r.trailing_zeros() >= 8 {
                    return with_al(r, false);
                }
            }
        }
        let r = h.call_virtual(visitor, 3, &[this, ctx]);
        with_al(r, true)
    }

    /// Marks `flags[ids[i]] = 1` for `i` in `from..n` (unsigned) where the
    /// weight `[[this + 0x18] + 4i]` is not 0 (NaN counts as not 0).
    fn mark(m: &mut impl Mem, this: u32, flags: u32, ids: u32, from: u32, n: u32) {
        let mut i = from;
        while i < n {
            let w: u32 = m.read(this.wrapping_add(0x18));
            let x = ld(m, w.wrapping_add(i.wrapping_mul(4)));
            if x != 0.0 {
                let id: u32 = m.read(ids.wrapping_add(i.wrapping_mul(4)));
                m.write(flags.wrapping_add(id), 1u8);
            }
            i = i.wrapping_add(1);
        }
    }

    /// `0x0052e2f0` (`thiscall(flags, ids, n)`, `ret 0xc`): the weighted
    /// blender's used slots: see `mark`.
    pub fn mark_weighted(m: &mut impl Mem, this: u32, flags: u32, ids: u32, n: u32) {
        mark(m, this, flags, ids, 0, n);
    }

    /// `0x0052f8a0` (`thiscall(flags, ids, n)`, `ret 0xc`): the additive
    /// blender's: the base slot (`ids[0]`) always, the rest by weight.
    pub fn mark_additive(m: &mut impl Mem, this: u32, flags: u32, ids: u32, n: u32) {
        if n == 0 {
            return;
        }
        let id: u32 = m.read(ids);
        m.write(flags.wrapping_add(id), 1u8);
        mark(m, this, flags, ids, 1, n);
    }

    /// [`super::WEIGHT_BATCH_DATA`] (`thiscall(_, out, _)`, `ret 0xc`): the
    /// blender's batch data: a 16-byte header (0x10, 0, 0, 0) and its
    /// weights (`+0x18`, count `+0x14`) copied after it as `f32`
    /// (`fld`/`fstp`); the size, the weights rounded up to 16 plus 16.
    pub fn weight_batch_data(m: &mut impl Mem, this: u32, out: u32) -> u32 {
        let n = count(m, this.wrapping_add(0x14));
        for (v, k) in [0x10u32, 0, 0, 0].into_iter().zip(0u32..) {
            m.write(out.wrapping_add(4 * k), v);
        }
        for i in 0..n {
            let w: u32 = m.read(this.wrapping_add(0x18));
            let v = x87::store(ld(m, w.wrapping_add(i.wrapping_mul(4))));
            m.write(out.wrapping_add(0x10).wrapping_add(i.wrapping_mul(4)), v);
        }
        (n.wrapping_mul(4).wrapping_add(0xf) & !0xf).wrapping_add(0x10)
    }

    /// [`super::RESIZE_WEIGHTS`] (`thiscall(n)`, `ret 4`): the sources
    /// resized ([`resize_sources`], its error returned), then the weights
    /// (`+0x14` count, `+0x18` data) likewise: new storage with the kept
    /// floats ([`copy_floats`]; 0xd when the allocation fails for `n` !=
    /// 0), the old freed when different, owned and set; the new weights
    /// 0.0; 0.
    pub fn resize_weights(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let r = resize_sources(h, this, n);
        if r != 0 {
            return r;
        }
        let raw: u32 = h.read(this.wrapping_add(0x14));
        let have = raw & !NOT_OWNED;
        if have == n {
            return 0;
        }
        if n != raw {
            let old: u32 = h.read(this.wrapping_add(0x18));
            let mut p = 0;
            if n != 0 {
                p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4]);
                if p == 0 {
                    return 0xd;
                }
                copy_floats(h, p, old, n.min(have));
            }
            let cur: u32 = h.read(this.wrapping_add(0x18));
            if cur != p && h.read::<u32>(this.wrapping_add(0x14)) & NOT_OWNED == 0 && cur != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[cur]);
            }
            h.write(this.wrapping_add(0x18), p);
            h.write(this.wrapping_add(0x14), n);
        }
        let mut i = have;
        while i < n {
            let data: u32 = h.read(this.wrapping_add(0x18));
            h.write(data.wrapping_add(i.wrapping_mul(4)), 0u32);
            i += 1;
        }
        0
    }

    fn quat(m: &impl Mem, at: u32) -> [f64; 4] {
        [0, 4, 8, 0xc].map(|o| ld(m, at.wrapping_add(o)))
    }

    /// `0x0052ecb0` (`thiscall(out, q)`, `ret 8`): the quaternion product
    /// `this * q` (Vectormath's order of terms, each component one x87
    /// expression stored as `f32`); `out`.
    pub fn quat_mul(m: &mut impl Mem, this: u32, out: u32, q: u32) -> u32 {
        let (add, sub, mul) = (x87::add, x87::sub, x87::mul);
        // each component reads both quaternions again: `out` may be one
        let [ax, ay, az, aw] = quat(m, this);
        let [bx, by, bz, bw] = quat(m, q);
        let x = sub(add(add(mul(bw, ax), mul(bx, aw)), mul(bz, ay)), mul(by, az));
        m.write(out, x87::store(x));
        let [ax, ay, az, aw] = quat(m, this);
        let [bx, by, bz, bw] = quat(m, q);
        let y = sub(add(add(mul(bw, ay), mul(by, aw)), mul(bx, az)), mul(ax, bz));
        m.write(out.wrapping_add(4), x87::store(y));
        let [ax, ay, az, aw] = quat(m, this);
        let [bx, by, bz, bw] = quat(m, q);
        let z = sub(add(add(mul(bw, az), mul(bz, aw)), mul(ax, by)), mul(bx, ay));
        m.write(out.wrapping_add(8), x87::store(z));
        let [ax, ay, az, aw] = quat(m, this);
        let [bx, by, bz, bw] = quat(m, q);
        let w = sub(sub(sub(mul(bw, aw), mul(bx, ax)), mul(by, ay)), mul(bz, az));
        m.write(out.wrapping_add(0xc), x87::store(w));
        out
    }

    /// `0x0052fa10` (cdecl `(out, q)`): `q` normalised: the squares summed
    /// with an `f32` store per step, `_CIsqrt`, `f32(1 / length)`, each
    /// component times it; `out`.
    pub fn quat_normalize(h: &mut impl Host, out: u32, q: u32) -> u32 {
        let [x, y, z, w] = quat(h, q);
        let mut s = f32_of(x87::mul(x, x));
        s = f32_of(x87::add(s, x87::mul(y, y)));
        s = f32_of(x87::add(s, x87::mul(z, z)));
        s = f32_of(x87::add(s, x87::mul(w, w)));
        let at = h.rebase(CI_SQRT);
        let len = x87::load(h.call_intrinsic(at, s).to_bits());
        let inv = f32_of(x87::div(1.0, len));
        for k in 0..4u32 {
            let c = ld(h, q.wrapping_add(4 * k));
            h.write(out.wrapping_add(4 * k), x87::store(x87::mul(c, inv)));
        }
        out
    }

    /// `0x00530750` (`thiscall(_, out, _)`, `ret 0xc`): the SPU target
    /// blender's batch header: 0x10, 0, 0, 0, the rotation, scale and
    /// translation type records, then `n + 1` zero words (`n` the targets
    /// at `[[this + 8] + 0x10]`); the size `((4n + 0x1f) & !0xf) + 0x10`.
    pub fn target_header(h: &mut impl Host, this: u32, out: u32) -> u32 {
        let owner: u32 = h.read(this.wrapping_add(8));
        let n = count(h, owner.wrapping_add(0x10));
        let types = [ROTATION_TYPE, SCALE_TYPE, TRANSLATION_TYPE].map(|t| h.rebase(t));
        for (v, k) in [0x10u32, 0, 0, 0].into_iter().chain(types).zip(0u32..) {
            h.write(out.wrapping_add(4 * k), v);
        }
        for i in 0..=n {
            h.write(out.wrapping_add(0x1c).wrapping_add(i.wrapping_mul(4)), 0u32);
        }
        (n.wrapping_mul(4).wrapping_add(0x1f) & !0xf).wrapping_add(0x10)
    }

    /// `0x00530b20` (cdecl `(obj, fmt, ...)`): the message handed to the
    /// logger at `[obj + 0x14]` (slot 2, `(fmt, 0, 0, &va)`), `va` the
    /// address of the variable arguments, kept in `cell` (the original's
    /// frame word); the logger's `eax`, or `obj` without one.
    pub fn log(h: &mut impl Host, obj: u32, fmt: u32, va: u32, cell: u32) -> u32 {
        let logger: u32 = h.read(obj.wrapping_add(0x14));
        if logger == 0 {
            return obj;
        }
        h.write(cell, va);
        h.call_virtual(logger, 2, &[fmt, 0, 0, cell])
    }

    /// `0x00530c30` (`thiscall(q)`, `ret 4`): the rotation matrix of `q`
    /// into the three rows at `this` (`+0`, `+0x10`, `+0x20`; the fourth
    /// words untouched), Vectormath's `Matrix3(Quat)`: the doubled
    /// components and the nine products stored as `f32`, `1 - xx` kept as
    /// a double; `this`.
    pub fn quat_matrix(m: &mut impl Mem, this: u32, q: u32) -> u32 {
        let [x, y, z, w] = quat(m, q).map(f32_of);
        let (add, sub, mul) = (x87::add, x87::sub, x87::mul);
        let x2 = f32_of(add(x, x));
        let y2 = f32_of(add(y, y));
        let z2 = f32_of(add(z, z));
        let xx = f32_of(mul(x2, x));
        let xy = f32_of(mul(y2, x));
        let xz = f32_of(mul(x, z2));
        let wx = f32_of(mul(x2, w));
        let yy = f32_of(mul(y2, y));
        let yz = f32_of(mul(y, z2));
        let wy = f32_of(mul(y2, w));
        let zz = f32_of(mul(z, z2));
        let wz = f32_of(mul(z2, w));
        let one_xx = sub(1.0, xx);
        let rows = [
            [sub(sub(1.0, yy), zz), add(wz, xy), sub(xz, wy)],
            [sub(xy, wz), sub(one_xx, zz), add(yz, wx)],
            [add(xz, wy), sub(yz, wx), sub(one_xx, yy)],
        ];
        for (r, row) in (0u32..).zip(rows) {
            for (c, v) in (0u32..).zip(row) {
                m.write(this.wrapping_add(r * 0x10 + c * 4), x87::store(v));
            }
        }
        this
    }
}
