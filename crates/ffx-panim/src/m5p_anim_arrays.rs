//! List `phyre-anim`, batch 3 (M5 stage 1): `PArray` / `PSharray`
//! allocation and resize helpers, the channel base's copy and compare,
//! keyed lookups, and the event controller's and the component's update
//! steps that pass their frames to callees.
//!
//! Where the original hands a callee the address of a word of its own
//! frame (an argument slot or a local), the port uses the host's scratch,
//! laid out at the original's offsets from the frame base given in each
//! function's documentation, so the parity tests can put it at the
//! original's address.

#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_misc::{MEMCPY, PHYRE_MALLOC};
use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE, STRING_ASSIGN, TREE_CLEAR};

/// The class registry singleton `0x0051dbf0` asks for the type system
/// (its vtable slot 1, thiscall, no words).
pub const REGISTRY: u32 = 0x00ca_a7e8;
/// The type system's array registration `(storage, 1)` (thiscall).
pub const REGISTER_STORAGE: u32 = 0x0043_c160;
/// `PSharray<PAnimationClip *>::setArray(els, n)` (thiscall).
pub const CLIPS_SET_ARRAY: u32 = 0x0051_dac0;
/// The hash lookup `(&key)` (thiscall on the table at `this + 8`).
pub const HASH_FIND: u32 = 0x005c_c560;
/// `PString` comparison `(text)` (thiscall; `al`).
pub const STRING_EQUALS: u32 = 0x004e_a7a0;
/// The animation event dispatch `(prev, now, callback, ctx)` (thiscall on
/// the event list).
pub const EVENT_DISPATCH: u32 = 0x0052_07c0;
/// The callback the event controller hands it.
pub const EVENT_CALLBACK: u32 = 0x0052_4850;
/// The component lists' triple append `(storage, &triple)` (thiscall).
pub const APPEND_TRIPLE: u32 = 0x0061_6c30;
/// The remove step's resize of a word array `(n)` (thiscall).
pub const WORDS_RESIZE: u32 = 0x0051_6fe0;
/// `PAnimationChannelTarget`'s array copy-construct `(dst, src, n)`
/// (cdecl).
pub const TARGETS_COPY: u32 = 0x0051_7800;
/// `PAnimationChannelTarget`'s array construct `(dst, n)` (cdecl).
pub const TARGETS_CONSTRUCT: u32 = 0x0051_bb80;
/// The clip's time range `(&start, &end)` (thiscall on the clip).
pub const CLIP_RANGE: u32 = 0x0051_6a20;
/// `PRedBlackTree`'s node release (thiscall on the tree).
pub const TREE_RELEASE: u32 = 0x0057_4970;
/// The ring buffer's block allocation `(bytes)` (thiscall).
pub const RING_ALLOCATE: u32 = 0x005c_e6e0;
/// The block id counter `0x005ce930` reads.
pub const BLOCK_ID: u32 = 0x00cb_d9c0;
/// The render command queue `0x00642b10` posts to (a pointer).
pub const COMMAND_QUEUE: u32 = 0x00cc_ca98;
/// Its post `(msg, size, kind, handler)` (thiscall).
pub const QUEUE_POST: u32 = 0x006a_de20;
/// The handler `0x00642b10` posts.
pub const QUEUE_HANDLER: u32 = 0x0063_7d40;

/// What a word-array allocation does past the copied words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rest {
    /// Left as allocated (`0x00528270`).
    Keep,
    /// Zeroed (`0x00513b80` and its copy `0x0052bc40`).
    Zero,
}

pub mod bridge {
    use super::{
        APPEND_TRIPLE, BLOCK_ID, CLIP_RANGE, CLIPS_SET_ARRAY, COMMAND_QUEUE, Conv, EVENT_CALLBACK,
        EVENT_DISPATCH, HASH_FIND, Host, MEMCPY, Mem, NOT_OWNED, PHYRE_FREE, PHYRE_MALLOC,
        QUEUE_HANDLER, QUEUE_POST, REGISTER_STORAGE, REGISTRY, RING_ALLOCATE, Rest, STRING_ASSIGN,
        STRING_EQUALS, TARGETS_CONSTRUCT, TARGETS_COPY, TREE_CLEAR, TREE_RELEASE, WORDS_RESIZE,
        x87,
    };

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    /// The `n` words from `src` into `dst` (skipping a null element
    /// address; the source only advances past copied words), then,
    /// for [`Rest::Zero`], words `m..total` zeroed.
    fn fill(m: &mut impl Mem, dst: u32, src: u32, n: u32, total: u32, rest: Rest) {
        let mut from = src;
        for i in 0..n {
            let at = dst.wrapping_add(i.wrapping_mul(4));
            if at != 0 {
                let w: u32 = m.read(from);
                m.write(at, w);
                from = from.wrapping_add(4);
            }
        }
        if rest == Rest::Zero {
            // rep stos of `total - n` words (the callers keep n <= total)
            let base = dst.wrapping_add(n.wrapping_mul(4));
            for k in 0..total.wrapping_sub(n) {
                m.write(base.wrapping_add(k.wrapping_mul(4)), 0u32);
            }
        }
    }

    /// A new array of `total` words (`PhyreMalloc(4 total, 4)`) holding the
    /// first `n` words of `src` (`0x00528270`: `Rest::Keep`;
    /// `0x00513b80`, `0x0052bc40`: `Rest::Zero`; cdecl `(total, src, n)`).
    /// 0 for `total == 0` or a failed allocation.
    pub fn alloc_copy(h: &mut impl Host, rest: Rest, total: u32, src: u32, n: u32) -> u32 {
        if total == 0 {
            return 0;
        }
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[total << 2, 4]);
        if p == 0 {
            return 0;
        }
        fill(h, p, src, n, total, rest);
        p
    }

    /// `PSharray<T>`'s storage for `total` words (`0x0051c270`,
    /// `thiscall(total, src, n)`, `ret 0xc`): inline at `this + 4` for at
    /// most one, else `PhyreMalloc`'d; the first `n` words of `src`, the
    /// rest zero. The storage, or 0 when the allocation failed.
    pub fn sharray_storage(h: &mut impl Host, this: u32, total: u32, src: u32, n: u32) -> u32 {
        let p = if total > 1 {
            h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[total << 2, 4])
        } else {
            this.wrapping_add(4)
        };
        if p == 0 {
            return 0;
        }
        fill(h, p, src, n, total, Rest::Zero);
        p
    }

    /// `PArray<T *>::resize(n)` (`0x00516eb0`, `0x0052cde0`;
    /// `thiscall`, `ret 4`): unless the count word already is `n`, a new
    /// array of `n` words holding the old ones (`alloc`, [`alloc_copy`]
    /// `Rest::Zero` at its address), the old owned one freed; 0, or 0xd
    /// when the allocation failed.
    pub fn resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let c: u32 = h.read(this);
        if n == c {
            return 0;
        }
        let low = c & !NOT_OWNED;
        let keep = if n < low { n } else { low };
        let old: u32 = h.read(this.wrapping_add(4));
        let p = alloc_copy(h, Rest::Zero, n, old, keep);
        if p == 0 && n != 0 {
            return 0xd;
        }
        let prev: u32 = h.read(this.wrapping_add(4));
        if prev != p && h.read::<u32>(this) & NOT_OWNED == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[prev]);
        }
        h.write(this.wrapping_add(4), p);
        h.write(this, n);
        0
    }

    /// `PSharray<T>::setArray(els, n)` as the clips' dynamic member
    /// gives it (`0x0051dbf0`, `thiscall`, `ret 8`): the registry's
    /// type system (its slot 1); `els` being the inline storage with
    /// `n == 1` and a type system registers that storage, anything else
    /// goes to [`CLIPS_SET_ARRAY`]. 0.
    pub fn clips_set_array(h: &mut impl Host, this: u32, els: u32, n: u32) -> u32 {
        let reg = h.rebase(REGISTRY);
        let vt: u32 = h.read(reg);
        let f: u32 = h.read(vt.wrapping_add(4));
        let ts = h.call(f, Conv::Thiscall(reg), &[]);
        let inline = this.wrapping_add(4);
        if els == inline && n == 1 && ts != 0 {
            h.call_sym(REGISTER_STORAGE, Conv::Thiscall(ts), &[inline, 1]);
        } else {
            h.call_sym(CLIPS_SET_ARRAY, Conv::Thiscall(this), &[els, n]);
        }
        0
    }

    /// A channel key: the channel's type id (`[[key] + 0xc]`, 0 without a
    /// channel) in the top byte over its `+8` word.
    fn channel_key(m: &impl Mem, key: u32) -> u32 {
        let ch: u32 = m.read(key);
        let id: u32 = if ch == 0 {
            0
        } else {
            m.read(ch.wrapping_add(0xc))
        };
        id << 24 | m.read::<u32>(key.wrapping_add(8))
    }

    /// `0x0051cb40` (`thiscall(key)`, `ret 4`): the index of the first
    /// 16-byte record at `[this + 0x14]` (count at `+0x10`) with
    /// `key`'s channel key, or -1.
    pub fn find_channel(m: &impl Mem, this: u32, key: u32) -> u32 {
        let n = count(m, this.wrapping_add(0x10));
        let want = channel_key(m, key);
        let recs: u32 = m.read(this.wrapping_add(0x14));
        for i in 0..n {
            if channel_key(m, recs.wrapping_add(i.wrapping_mul(16))) == want {
                return i;
            }
        }
        u32::MAX
    }

    /// `Phyre_PAnimation_PAnimationSetTargetLookup`'s find (`0x005cc800`,
    /// `thiscall(key)`, `ret 4`): without a table (`[this + 4]`), the
    /// set's [`find_channel`] (a tail jump on `[this]`); else the hash
    /// lookup of the channel key (the key word by address: scratch +0,
    /// the original's argument slot) on `this + 8`: the entry's index
    /// (16-byte entries from `[this + 4]`), or -1.
    pub fn lookup_channel(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let table: u32 = h.read(this.wrapping_add(4));
        if table == 0 {
            let set: u32 = h.read(this);
            return find_channel(h, set, key);
        }
        let k = channel_key(h, key);
        let at = h.scratch();
        h.write(at, k);
        let r = h.call_sym(HASH_FIND, Conv::Thiscall(this.wrapping_add(8)), &[at]);
        if r == 0 {
            u32::MAX
        } else {
            (r.wrapping_sub(table).cast_signed() >> 4).cast_unsigned()
        }
    }

    /// `PAnimationChannelBase::operator=` (`0x0050b730`,
    /// `thiscall(other)`, `ret 4`): unless `other` is `this`, the words
    /// `+0x10`, `+0..+0xc`, the name (`PString::Assign` of `other`'s text,
    /// its flag bit 0 cleared) and `+0x18`; `this`.
    pub fn assign_channel_base(h: &mut impl Host, this: u32, other: u32) -> u32 {
        if this == other {
            return this;
        }
        for off in [0x10, 0, 4, 8, 0xc] {
            let w: u32 = h.read(other.wrapping_add(off));
            h.write(this.wrapping_add(off), w);
        }
        let name = other.wrapping_add(0x14);
        if this.wrapping_add(0x14) != name {
            let p: u32 = h.read(name);
            let text = p.wrapping_sub(p & 1);
            h.call_sym(
                STRING_ASSIGN,
                Conv::Thiscall(this.wrapping_add(0x14)),
                &[text],
            );
        }
        let w: u32 = h.read(other.wrapping_add(0x18));
        h.write(this.wrapping_add(0x18), w);
        this
    }

    /// `PAnimationChannelBase::operator==` (`0x0050b7b0`,
    /// `thiscall(other)`, `ret 4`): the words `+0x10`, `+0..+0xc`, the
    /// names ([`STRING_EQUALS`] with `other`'s text) and `+0x18`; `al`.
    pub fn channel_base_equals(h: &mut impl Host, this: u32, other: u32) -> u8 {
        for off in [0x10, 0, 4, 8, 0xc] {
            if h.read::<u32>(this.wrapping_add(off)) != h.read::<u32>(other.wrapping_add(off)) {
                return 0;
            }
        }
        let p: u32 = h.read(other.wrapping_add(0x14));
        let text = p.wrapping_sub(p & 1);
        let r = h.call_sym(
            STRING_EQUALS,
            Conv::Thiscall(this.wrapping_add(0x14)),
            &[text],
        );
        if r.trailing_zeros() >= 8 {
            return 0;
        }
        u8::from(h.read::<u32>(this.wrapping_add(0x18)) == h.read::<u32>(other.wrapping_add(0x18)))
    }

    /// `0x0051cbb0` (`thiscall(channel)`, `ret 4`): the index of the first
    /// 28-byte channel base at `[this + 0xc]` (count at `+8`) equal to
    /// `channel` ([`channel_base_equals`]), or -1.
    pub fn find_channel_base(h: &mut impl Host, this: u32, channel: u32) -> u32 {
        let n = count(h, this.wrapping_add(8));
        for i in 0..n {
            let els: u32 = h.read(this.wrapping_add(0xc));
            if channel_base_equals(h, els.wrapping_add(i.wrapping_mul(0x1c)), channel) != 0 {
                return i;
            }
        }
        u32::MAX
    }

    /// `0x00616d60` (`thiscall`): `[this + 0x118] = -1`, and every
    /// 12-byte record at `[this + 0x10c]` (count at `+0x108`) gets word 2
    /// = -1, word 1 = 0.
    pub fn reset_records(m: &mut impl Mem, this: u32) {
        m.write(this.wrapping_add(0x118), u32::MAX);
        let mut i = 0;
        while i < count(m, this.wrapping_add(0x108)) {
            let recs: u32 = m.read(this.wrapping_add(0x10c));
            let rec = recs.wrapping_add(i.wrapping_mul(12));
            m.write(rec.wrapping_add(8), u32::MAX);
            let recs: u32 = m.read(this.wrapping_add(0x10c));
            m.write(recs.wrapping_add(i.wrapping_mul(12)).wrapping_add(4), 0u32);
            i += 1;
        }
    }

    /// `0x00616cb0` (`thiscall(key)`, `ret 4`): through the pairs at
    /// `[this + 0x104]` (count at `+0x100`), the first record (`[[pair] +
    /// 0x10c] + 12 [pair + 4]`, not null) whose word 2 is `key`, or 0.
    pub fn find_bound(m: &impl Mem, this: u32, key: u32) -> u32 {
        let n = count(m, this.wrapping_add(0x100));
        let pairs: u32 = m.read(this.wrapping_add(0x104));
        for i in 0..n {
            let pair = pairs.wrapping_add(i.wrapping_mul(8));
            let k: u32 = m.read(pair.wrapping_add(4));
            let obj: u32 = m.read(pair);
            let recs: u32 = m.read(obj.wrapping_add(0x10c));
            let rec = recs.wrapping_add(k.wrapping_mul(12));
            if rec != 0 && m.read::<u32>(rec.wrapping_add(8)) == key {
                return rec;
            }
        }
        0
    }

    /// `0x00616740` (`thiscall(a)`, `ret 4`): appends the record `(a, 0,
    /// -1)` (by address: scratch +0..+0xc, the original's local at
    /// `ebp-0xc`) to the list at `this + 0x108` with a new buffer of
    /// `count + 1` records; 1 when the append returned 0, else 0.
    pub fn append_record(h: &mut impl Host, this: u32, a: u32) -> u32 {
        let list = this.wrapping_add(0x108);
        let n = count(h, list).wrapping_add(1).wrapping_mul(3) << 2;
        let at = h.scratch();
        h.write(at, a);
        h.write(at.wrapping_add(4), 0u32);
        h.write(at.wrapping_add(8), u32::MAX);
        let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n, 4]);
        let r = h.call_sym(APPEND_TRIPLE, Conv::Thiscall(list), &[buf, at]);
        u32::from(r == 0)
    }

    /// `PSharray<T>::set(i, v)` (`0x0051d7b0`, `thiscall`, `ret 8`):
    /// element `i` (inline for a one-element array) = `v`; 0, 1 for a
    /// null `v`, 3 out of range.
    pub fn sharray_set(m: &mut impl Mem, this: u32, i: u32, v: u32) -> u32 {
        if v == 0 {
            return 1;
        }
        let n = count(m, this);
        if i >= n {
            return 3;
        }
        let base = if n > 1 {
            m.read(this.wrapping_add(4))
        } else {
            this.wrapping_add(4)
        };
        m.write(base.wrapping_add(i.wrapping_mul(4)), v);
        0
    }

    /// `PAnimationSlotFilter`'s batch setup (`0x0052a6b0`, vtable slot 10,
    /// `thiscall(_, desc, _)`, `ret 0xc`): the descriptor's header `(0x10,
    /// 0, 0, 0, n)` and the `n` slot words from `[this + 0x10]` (`n` =
    /// the count at `+0xc`) after it; the bytes used, `((4n + 0x13) &
    /// !0xf) + 0x10`.
    pub fn slot_filter_setup(h: &mut impl Host, this: u32, desc: u32) -> u32 {
        let n = count(h, this.wrapping_add(0xc));
        h.write(desc, 0x10u32);
        h.write(desc.wrapping_add(4), 0u32);
        h.write(desc.wrapping_add(8), 0u32);
        h.write(desc.wrapping_add(0xc), 0u32);
        h.write(desc.wrapping_add(0x10), n);
        let src: u32 = h.read(this.wrapping_add(0x10));
        h.call_sym(MEMCPY, Conv::Cdecl, &[desc.wrapping_add(0x14), src, n << 2]);
        (n.wrapping_mul(4).wrapping_add(0x13) & !0xf).wrapping_add(0x10)
    }

    /// `PAnimationEventController`'s update (`0x00524d80`, vtable slot
    /// 10, `thiscall(time, _, ctx)`, `ret 0xc`): with a time controller
    /// (`+0x10`) and events (`+0xc`): `ctx[0] = time`, `ctx[2]` = the
    /// controller's time (slot 4), the events dispatched from `ctx[1]` to
    /// it with [`EVENT_CALLBACK`], then `ctx[1]` = it. 0.
    pub fn event_update(h: &mut impl Host, this: u32, time: u32, ctx: u32) -> u32 {
        let tc: u32 = h.read(this.wrapping_add(0x10));
        if tc == 0 {
            return 0;
        }
        let vt: u32 = h.read(tc);
        let f: u32 = h.read(vt.wrapping_add(0x10));
        let now = h.call_st0(f, Conv::Thiscall(tc), &[]).to_bits();
        let events: u32 = h.read(this.wrapping_add(0xc));
        if events == 0 {
            return 0;
        }
        h.write(ctx.wrapping_add(8), now);
        h.write(ctx, time);
        let prev = x87::store(x87::load(h.read(ctx.wrapping_add(4))));
        let cb = h.rebase(EVENT_CALLBACK);
        h.call_sym(
            EVENT_DISPATCH,
            Conv::Thiscall(events),
            &[prev, now, cb, ctx],
        );
        h.write(ctx.wrapping_add(4), now);
        0
    }

    /// `0x00617170` (`thiscall(n)`, `ret 4`): `n` empty tree headers of
    /// 24 bytes (each linked to itself in its first three words,
    /// `PhyreMalloc(24 n, 4)`): `[this + 4]` the array, `[this]` `n` (0
    /// without one); `this`.
    pub fn tree_array(h: &mut impl Host, this: u32, n: u32) -> u32 {
        h.write(this, 0u32);
        let mut p = 0;
        if n != 0 {
            p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(24), 4]);
            if p != 0 {
                for i in 0..n {
                    let e = p.wrapping_add(i.wrapping_mul(24));
                    if e != 0 {
                        h.write(e, e);
                        h.write(e.wrapping_add(4), e);
                        h.write(e.wrapping_add(8), e);
                    }
                }
            }
        }
        h.write(this.wrapping_add(4), p);
        h.write(this, if p == 0 { 0 } else { n });
        this
    }

    /// `0x00642b10` (cdecl `(object, params)`): with both set, posts the
    /// 0x44-byte message `(object, params[0..16])` (scratch +0..+0x44,
    /// the original's local at `ebp-0x48`) to the render command queue.
    /// `eax`: 0 for a null object, the object for null params, else the
    /// post's.
    pub fn post_command(h: &mut impl Host, object: u32, params: u32) -> u32 {
        if object == 0 {
            return 0;
        }
        if params == 0 {
            return object;
        }
        let at = h.scratch();
        h.write(at, object);
        for k in 0..16 {
            let w: u32 = h.read(params.wrapping_add(4 * k));
            h.write(at.wrapping_add(4 + 4 * k), w);
        }
        let q = h.rebase(COMMAND_QUEUE);
        let queue: u32 = h.read(q);
        let handler = h.rebase(QUEUE_HANDLER);
        h.call_sym(
            QUEUE_POST,
            Conv::Thiscall(queue),
            &[at, 0x44, 0x2001, handler],
        )
    }

    /// `0x00516e00` (`thiscall(i)`, `ret 4`): removes word `i` of the
    /// array at `[this + 8]` (count at `+4`; the last word moved over it,
    /// the last cleared), then [`WORDS_RESIZE`] to one less; its `eax`, or
    /// 3 out of range.
    pub fn remove_word(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let n = count(h, this.wrapping_add(4));
        if i >= n {
            return 3;
        }
        let last = n - 1;
        if i != last {
            let els: u32 = h.read(this.wrapping_add(8));
            let w: u32 = h.read(els.wrapping_add(last.wrapping_mul(4)));
            h.write(els.wrapping_add(i.wrapping_mul(4)), w);
            if last < count(h, this.wrapping_add(4)) {
                let els: u32 = h.read(this.wrapping_add(8));
                h.write(els.wrapping_add(last.wrapping_mul(4)), 0u32);
            }
        }
        h.call_sym(WORDS_RESIZE, Conv::Thiscall(this), &[last])
    }

    /// `PArray<PAnimationChannelTarget>`'s allocation (`0x00519d30`,
    /// cdecl `(total, src, n)`): `PhyreMalloc(28 total, 4)`, the first `n`
    /// copied from `src`, the rest constructed; the array, or 0.
    pub fn alloc_targets(h: &mut impl Host, total: u32, src: u32, n: u32) -> u32 {
        if total == 0 {
            return 0;
        }
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[total.wrapping_mul(28), 4]);
        if p == 0 {
            return 0;
        }
        h.call_sym(TARGETS_COPY, Conv::Cdecl, &[p, src, n]);
        h.call_sym(
            TARGETS_CONSTRUCT,
            Conv::Cdecl,
            &[p.wrapping_add(n.wrapping_mul(28)), total.wrapping_sub(n)],
        );
        p
    }

    /// `0x00544110` (`thiscall(i, t)`, `ret 8`): source record `i`'s
    /// time is set: `+8 = t`, the record's controller (`+0x24`, slot 4)
    /// read before and after `+0x30 = t`, and their difference added to
    /// `+0x34` (at 53 bits, rounded to `f32`); 0, or 3 out of range.
    pub fn set_source_time(h: &mut impl Host, this: u32, i: u32, t: u32) -> u32 {
        if i >= count(h, this.wrapping_add(0x14)) {
            return 3;
        }
        let els: u32 = h.read(this.wrapping_add(0x18));
        let rec = i.wrapping_mul(0x4c).wrapping_add(els);
        let t = x87::store(x87::load(t));
        h.write(rec.wrapping_add(8), t);
        let ctl = rec.wrapping_add(0x24);
        let vt: u32 = h.read(ctl);
        let f: u32 = h.read(vt.wrapping_add(0x10));
        let before = h.call_st0(f, Conv::Thiscall(ctl), &[]).to_bits();
        h.write(rec.wrapping_add(0x30), t);
        let vt: u32 = h.read(ctl);
        let f: u32 = h.read(vt.wrapping_add(0x10));
        let after = h.call_st0(f, Conv::Thiscall(ctl), &[]).to_bits();
        let acc: u32 = h.read(rec.wrapping_add(0x34));
        let d = x87::sub(x87::load(before), x87::load(after));
        h.write(
            rec.wrapping_add(0x34),
            x87::store(x87::add(d, x87::load(acc))),
        );
        0
    }

    /// The lookup table's destructor (`0x005cc770`, `0x005ccc50`;
    /// `thiscall`): the tree at `+8` releases its nodes, `PhyreFree` of
    /// `[this + 4]`, the tree cleared; the clear's `eax`.
    pub fn lookup_dtor(h: &mut impl Host, this: u32) -> u32 {
        let tree = this.wrapping_add(8);
        h.call_sym(TREE_RELEASE, Conv::Thiscall(tree), &[]);
        let p: u32 = h.read(this.wrapping_add(4));
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(tree), &[])
    }

    /// `0x005ce930` (`thiscall(bytes)`, `ret 4`): a block of `bytes +
    /// 0x14` from the ring buffer at `this + 0x14` ([`RING_ALLOCATE`]); it
    /// gets the id `[BLOCK_ID] + 1` (1 when that is 0) at `+4` and five
    /// zero words at `+0x10`; returns the block's data (`+0x24`), or 0.
    pub fn allocate_block(h: &mut impl Host, this: u32, block: u32) -> u32 {
        let g = h.rebase(BLOCK_ID);
        let id = h.read::<u32>(g).wrapping_add(1);
        let r = h.call_sym(
            RING_ALLOCATE,
            Conv::Thiscall(this.wrapping_add(0x14)),
            &[block.wrapping_add(0x14)],
        );
        if r == 0 {
            return 0;
        }
        h.write(r.wrapping_add(4), if id == 0 { 1 } else { id });
        let e = r.wrapping_add(0x10);
        if e == 0 {
            return 0;
        }
        for k in 0..5 {
            h.write(e.wrapping_add(4 * k), 0u32);
        }
        e.wrapping_add(0x14)
    }

    /// The component's "past the end" checks (`0x00542ff0` record 0,
    /// `0x005430a0` record `i`): the record's clip range (`CLIP_RANGE` on
    /// `[rec]`, into two words of the frame), and whether the record's
    /// value (its controller's slot 4 at `+0xc` when `[rec]` is set, else
    /// 0, rounded to `f32`) is above the end. `al`; 0 out of range or
    /// without a clip. `start` and `end` are where the two words go
    /// (scratch offsets: `0x00542ff0` +4 and +0, `0x005430a0` +0xc and +0,
    /// the original's frame).
    pub fn past_end(h: &mut impl Host, this: u32, i: u32, start: u32, end: u32) -> u8 {
        if i >= count(h, this.wrapping_add(0x14)) {
            return 0;
        }
        let els: u32 = h.read(this.wrapping_add(0x18));
        let rec = i.wrapping_mul(0x4c).wrapping_add(els);
        let clip: u32 = h.read(rec);
        if clip == 0 {
            return 0;
        }
        let at = h.scratch();
        let (s, e) = (at.wrapping_add(start), at.wrapping_add(end));
        h.call_sym(CLIP_RANGE, Conv::Thiscall(clip), &[s, e]);
        let v = if h.read::<u32>(rec) == 0 {
            0.0f32
        } else {
            let ctl = rec.wrapping_add(0xc);
            let vt: u32 = h.read(ctl);
            let f: u32 = h.read(vt.wrapping_add(0x10));
            h.call_st0(f, Conv::Thiscall(ctl), &[])
        };
        // the original stores the value over the start word, then compares
        h.write(s, v.to_bits());
        let v = x87::load(v.to_bits());
        let last = x87::load(h.read(e));
        // fcompp: C0 (last < v) alone; unordered sets C0 and C2 (al 0)
        u8::from(last < v)
    }
}
