//! List `phyre-anim-left`, batch 3 (M5 stage 1): `PAnimatableComponent`
//! and the animation network optimiser's helpers.
//!
//! - the component's arrays: resizes that construct fresh elements
//!   (`0x18`-, `0x10`- and `0x94`-byte records), the appends that grow a
//!   `PArray` by one into storage the caller allocated (words, pairs,
//!   triples, and four copies for 8-byte objects with a vtable);
//! - the optimiser's node table (`0x11c`-byte nodes made with the CRT's
//!   vector iterators), its links and reachability checks;
//! - the component's source records (`0x4c` bytes): binding a clip,
//!   setting a source, setting every source's time.
//!
//! Callees outside the list stay calls to the original. Per-call scratch
//! only (`frame`: words of the caller's frame).

#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE, TIME_CONSTANT};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The array `operator new` (cdecl `(size)`).
pub const NEW_ARRAY: u32 = 0x0042_fb40;
/// The CRT's `eh vector constructor iterator` (stdcall `(p, size, n,
/// ctor, dtor)`).
pub const VECTOR_CTOR: u32 = 0x0094_966a;
/// The CRT's `eh vector destructor iterator` (stdcall `(p, size, n,
/// dtor)`).
pub const VECTOR_DTOR: u32 = 0x0094_96d2;
/// The optimiser node's constructor and destructor.
pub const NODE_CTOR: u32 = 0x0061_64d0;
/// See [`NODE_CTOR`].
pub const NODE_DTOR: u32 = 0x005c_d4c0;
/// A node's setup `(a)` (thiscall).
pub const NODE_SETUP: u32 = 0x0061_6dc0;
/// A node's input link `(other, slot)` (thiscall).
pub const NODE_LINK_IN: u32 = 0x0061_66f0;
/// A node's output link `(other)` (thiscall).
pub const NODE_LINK_OUT: u32 = 0x0061_66b0;
/// A node's output add `(word)` (thiscall).
pub const NODE_ADD_OUTPUT: u32 = 0x0061_6740;
/// The resolve call `0x006167a0` makes per entry (thiscall on its
/// argument, `(key, &context)`).
pub const RESOLVE: u32 = 0x0061_61e0;
/// The context's vtable and destructor (`0x006167a0`'s local).
pub const CONTEXT_VTABLE: u32 = 0x00b3_f17c;
/// See [`CONTEXT_VTABLE`].
pub const CONTEXT_DTOR: u32 = 0x0061_6140;
/// The release `0x00616930` calls (thiscall on its argument, `(id)`).
pub const RELEASE: u32 = 0x0061_6310;
/// The triple copier `0x00616c30` calls (cdecl `(dst, src, n)`).
pub const COPY_TRIPLES: u32 = 0x0061_6430;
/// The record copier `0x0053bfd0` calls (cdecl `(dst, src, n)`,
/// `0x94`-byte records).
pub const COPY_RECORDS: u32 = 0x0053_1e60;
/// The clip bind of a source record (thiscall on `record + 0x38`,
/// `(clip)`).
pub const RECORD_BIND: u32 = 0x0052_6590;
/// The clip's time range (thiscall on the clip, `(&start, &end)`).
pub const CLIP_RANGE: u32 = 0x0051_6a20;
/// The bind's time-controller set (thiscall on `record + 0x38`, `(ctl)`).
pub const RECORD_SET_TIME: u32 = 0x0052_65f0;
/// The f32 the record's `+0x20` starts at.
pub const RECORD_SPEED: u32 = 0x00b2_be08;
/// The blender's source set (thiscall on `component + 0x1c`, `(i, src)`).
pub const BLENDER_SET_SOURCE: u32 = 0x0052_cec0;
/// The blender's weight set (thiscall on `component + 0x1c`, `(i, w)`).
pub const BLENDER_SET_WEIGHT: u32 = 0x0052_e660;
/// The component's refresh (thiscall).
pub const COMPONENT_REFRESH: u32 = 0x0054_1c00;

/// A resize that constructs fresh elements (`0x0053bde0`, `0x0053bf50`):
/// element size, alignment, the cdecl `(p, n)` initialiser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshResize {
    pub size: u32,
    pub align: u32,
    pub init: u32,
}

/// The [`FreshResize`] copies.
pub const FRESH_RESIZES: &[(u32, FreshResize)] = &[
    (
        0x0053_bde0,
        FreshResize {
            size: 0x18,
            align: 4,
            init: 0x0053_9550,
        },
    ),
    (
        0x0053_bf50,
        FreshResize {
            size: 0x10,
            align: 0x10,
            init: 0x0053_9600,
        },
    ),
];

/// The [`FRESH_RESIZES`] entry of `f`.
#[must_use]
pub const fn fresh_of(f: u32) -> FreshResize {
    let mut i = 0;
    while i < FRESH_RESIZES.len() {
        if FRESH_RESIZES[i].0 == f {
            return FRESH_RESIZES[i].1;
        }
        i += 1;
    }
    panic!("not a fresh resize")
}

/// The appends of 8-byte objects (`0x0053b370` and copies): the vtable
/// each element gets and the release `(data, count)` (cdecl) of the old
/// storage.
pub const OBJECT_APPENDS: &[(u32, u32, u32)] = &[
    (0x0053_b370, 0x00b2_a340, 0x0053_7650),
    (0x0053_b4a0, 0x00b2_a358, 0x0053_76d0),
    (0x0053_b540, 0x00b2_a364, 0x0053_7710),
    (0x0053_b5e0, 0x00b2_a334, 0x0053_7750),
];

/// The [`OBJECT_APPENDS`] entry of `f`: `(vtable, release)`.
#[must_use]
pub const fn object_append_of(f: u32) -> (u32, u32) {
    let mut i = 0;
    while i < OBJECT_APPENDS.len() {
        if OBJECT_APPENDS[i].0 == f {
            return (OBJECT_APPENDS[i].1, OBJECT_APPENDS[i].2);
        }
        i += 1;
    }
    panic!("not an object append")
}

/// The word appends (`0x00616b00` and its copy `0x0053b250`).
pub const WORD_APPENDS: &[u32] = &[0x0061_6b00, 0x0053_b250];

pub mod bridge {
    use super::{
        BLENDER_SET_SOURCE, BLENDER_SET_WEIGHT, CLIP_RANGE, COMPONENT_REFRESH, CONTEXT_DTOR,
        CONTEXT_VTABLE, COPY_RECORDS, COPY_TRIPLES, Conv, FreshResize, Host, Mem, NEW_ARRAY,
        NODE_ADD_OUTPUT, NODE_CTOR, NODE_DTOR, NODE_LINK_IN, NODE_LINK_OUT, NODE_SETUP, NOT_OWNED,
        PHYRE_FREE, PHYRE_MALLOC, RECORD_BIND, RECORD_SET_TIME, RECORD_SPEED, RELEASE, RESOLVE,
        TIME_CONSTANT, VECTOR_CTOR, VECTOR_DTOR, x87,
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

    fn f32_of(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    /// `al` set to `bit` over `eax`.
    const fn with_al(eax: u32, bit: bool) -> u32 {
        eax & !0xff | bit as u32
    }

    /// The old storage of a `PArray` at `arr` freed when it is not `new`,
    /// owned and set; then `new` and `n` stored.
    fn replace(h: &mut impl Host, arr: u32, new: u32, n: u32) {
        let old: u32 = h.read(arr.wrapping_add(4));
        let raw: u32 = h.read(arr);
        if old != new && raw & NOT_OWNED == 0 && old != 0 {
            free(h, old);
        }
        h.write(arr.wrapping_add(4), new);
        h.write(arr, n);
    }

    /// [`super::FRESH_RESIZES`] (`thiscall(n)`, `ret 4`): nothing when `n`
    /// is the raw count word; else fresh storage of `n` elements (0xd when
    /// that fails for `n` != 0) initialised by `r.init(p, n)`, the old
    /// storage released; 0.
    pub fn fresh_resize(h: &mut impl Host, r: FreshResize, this: u32, n: u32) -> u32 {
        if n == h.read::<u32>(this) {
            return 0;
        }
        let mut p = 0;
        if n != 0 {
            p = malloc(h, n.wrapping_mul(r.size), r.align);
            if p == 0 {
                return 0xd;
            }
            h.call_sym(r.init, Conv::Cdecl, &[p, n]);
        }
        replace(h, this, p, n);
        0
    }

    /// `0x00539480` (cdecl `(dst, n)`): `n` `0x94`-byte records zeroed
    /// (null slots skipped).
    pub fn construct_records(m: &mut impl Mem, dst: u32, n: u32) {
        for i in 0..n {
            let p = dst.wrapping_add(i.wrapping_mul(0x94));
            if p != 0 {
                m.write_bytes(p, &[0; 0x94]);
            }
        }
    }

    /// `0x0053bfd0` (`thiscall(n)`, `ret 4`): the `0x94`-byte record array
    /// resized: nothing when `n` is the raw count word; else new storage
    /// (0xd when that fails for `n` != 0), the kept records copied
    /// ([`COPY_RECORDS`]), the rest zeroed ([`construct_records`]), the
    /// old storage released; 0.
    pub fn resize_records(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let raw: u32 = h.read(this);
        if n == raw {
            return 0;
        }
        let keep = n.min(raw & !NOT_OWNED);
        let old: u32 = h.read(this.wrapping_add(4));
        let mut p = 0;
        if n != 0 {
            p = malloc(h, n.wrapping_mul(0x94), 4);
            if p == 0 {
                return 0xd;
            }
            h.call_sym(COPY_RECORDS, Conv::Cdecl, &[p, old, keep]);
            construct_records(h, p.wrapping_add(keep.wrapping_mul(0x94)), n - keep);
        }
        replace(h, this, p, n);
        0
    }

    /// The end of an append: the old storage freed when it is not `new`
    /// and owned (and set, when `null_check`: the word appends free a
    /// null one too); `new` and the count plus one (without the flag)
    /// stored; 0.
    fn finish_append(h: &mut impl Host, this: u32, new: u32, n: u32, null_check: bool) -> u32 {
        let old: u32 = h.read(this.wrapping_add(4));
        let raw: u32 = h.read(this);
        if old != new && raw & NOT_OWNED == 0 && (old != 0 || !null_check) {
            free(h, old);
        }
        h.write(this.wrapping_add(4), new);
        h.write(this, n.wrapping_add(1));
        0
    }

    /// Copies `n` elements of `words` words from `src` to `dst` (a null
    /// destination slot takes nothing and the source stays).
    fn copy_words(m: &mut impl Mem, dst: u32, src: u32, n: u32, words: u32) {
        let (mut d, mut s) = (dst, src);
        for _ in 0..n {
            if d != 0 {
                for k in 0..words {
                    let v: u32 = m.read(s.wrapping_add(4 * k));
                    m.write(d.wrapping_add(4 * k), v);
                }
                s = s.wrapping_add(4 * words);
            }
            d = d.wrapping_add(4 * words);
        }
    }

    /// The appends of `words`-word elements (`0x00616b00`/`0x0053b250`:
    /// 1, `0x00616b90`: 2; `thiscall(new, &value)`, `ret 8`): 0xd without
    /// storage; the elements copied into `new`, the value after them, the
    /// old storage released (see `finish_append`), the count one more; 0.
    pub fn append(h: &mut impl Host, words: u32, this: u32, new: u32, value: u32) -> u32 {
        if new == 0 {
            return 0xd;
        }
        let n = count(h, this);
        let old: u32 = h.read(this.wrapping_add(4));
        copy_words(h, new, old, n, words);
        let at = new.wrapping_add(n.wrapping_mul(4 * words));
        if at != 0 {
            copy_words(h, at, value, 1, words);
        }
        finish_append(h, this, new, n, words != 1)
    }

    /// `0x00616c30` (`thiscall(new, &value)`, `ret 8`): as [`append`] for
    /// triples, the old ones copied by [`COPY_TRIPLES`].
    pub fn append_triple(h: &mut impl Host, this: u32, new: u32, value: u32) -> u32 {
        if new == 0 {
            return 0xd;
        }
        let n = count(h, this);
        let old: u32 = h.read(this.wrapping_add(4));
        h.call_sym(COPY_TRIPLES, Conv::Cdecl, &[new, old, n]);
        let at = new.wrapping_add(n.wrapping_mul(12));
        if at != 0 {
            copy_words(h, at, value, 1, 3);
        }
        finish_append(h, this, new, n, true)
    }

    /// [`super::OBJECT_APPENDS`] (`thiscall(new, &value)`, `ret 8`): as
    /// [`append`] for 8-byte objects: each copy gets `vtable` and the
    /// source's second word; the old storage goes to `release(old,
    /// count)` (no null check) when it is not `new` and owned.
    pub fn append_object(
        h: &mut impl Host,
        (vtable, release): (u32, u32),
        this: u32,
        new: u32,
        value: u32,
    ) -> u32 {
        if new == 0 {
            return 0xd;
        }
        let vt = h.rebase(vtable);
        let old: u32 = h.read(this.wrapping_add(4));
        let n = count(h, this);
        let (mut d, mut s) = (new, old);
        for _ in 0..n {
            if d != 0 {
                h.write(d, vt);
                let w: u32 = h.read(s.wrapping_add(4));
                h.write(d.wrapping_add(4), w);
                s = s.wrapping_add(8);
            }
            d = d.wrapping_add(8);
        }
        let at = new.wrapping_add(n.wrapping_mul(8));
        if at != 0 {
            h.write(at, vt);
            let w: u32 = h.read(value.wrapping_add(4));
            h.write(at.wrapping_add(4), w);
        }
        let cur: u32 = h.read(this.wrapping_add(4));
        if cur != new {
            let raw: u32 = h.read(this);
            if raw & NOT_OWNED == 0 {
                h.call_sym(release, Conv::Cdecl, &[cur, raw & !NOT_OWNED]);
            }
        }
        h.write(this.wrapping_add(4), new);
        h.write(this, n.wrapping_add(1));
        0
    }

    /// `0x006168a0` (`thiscall(index)`, `ret 4`, `al`): whether every
    /// owner (`[this + 0x114]`, count `+0x110`) that lists `(this, index)`
    /// among its links (`+0x104`, 8 bytes each, count `+0x100`) has its
    /// `+0x118` set (not -1). `eax` is the end of the owner list with
    /// `al` the answer (1 for none).
    pub fn links_resolved(m: &impl Mem, this: u32, index: u32) -> u32 {
        let n = count(m, this.wrapping_add(0x110));
        if n == 0 {
            return with_al(0, true);
        }
        let mut ok = true;
        let mut at: u32 = m.read(this.wrapping_add(0x114));
        for _ in 0..n {
            let owner: u32 = m.read(at);
            let links = count(m, owner.wrapping_add(0x100));
            let base: u32 = m.read(owner.wrapping_add(0x104));
            let listed = (0..links).any(|j| {
                let rec = base.wrapping_add(j.wrapping_mul(8));
                m.read::<u32>(rec) == this && m.read::<u32>(rec.wrapping_add(4)) == index
            });
            if listed {
                ok = ok && m.read::<u32>(owner.wrapping_add(0x118)) != u32::MAX;
            }
            at = at.wrapping_add(4);
        }
        with_al(at, ok)
    }

    /// `0x00616930` (`thiscall(arg)`, `ret 4`, `al` 1): each entry
    /// (`[this + 0x10c]`, 12 bytes, count `+0x108` re-read) in state 1
    /// whose links are resolved ([`links_resolved`]) with an id (`+8`)
    /// not negative and a key (`+0`) not -1 is released ([`RELEASE`] on
    /// `arg`) and goes to state 2. `eax` is the last count with `al` 1.
    pub fn release_resolved(h: &mut impl Host, this: u32, arg: u32) -> u32 {
        let mut i = 0u32;
        while i < count(h, this.wrapping_add(0x108)) {
            let base: u32 = h.read(this.wrapping_add(0x10c));
            let e = base.wrapping_add(i.wrapping_mul(12));
            if h.read::<u32>(e.wrapping_add(4)) == 1 && links_resolved(h, this, i) & 0xff != 0 {
                let id: u32 = h.read(e.wrapping_add(8));
                if id.cast_signed() >= 0 && h.read::<u32>(e) != u32::MAX {
                    h.call_sym(RELEASE, Conv::Thiscall(arg), &[id]);
                    let base: u32 = h.read(this.wrapping_add(0x10c));
                    h.write(base.wrapping_add(i.wrapping_mul(12)).wrapping_add(4), 2u32);
                }
            }
            i += 1;
        }
        with_al(count(h, this.wrapping_add(0x108)), true)
    }

    /// `0x006167a0` (`thiscall(arg)`, `ret 4`, `al` 1): each entry
    /// (`[this + 0x10c]`, 12 bytes, count `+0x108` re-read) resolved:
    /// a context `{vtable, this, key}` in the caller's frame (`frame`, 3
    /// words) handed to [`RESOLVE`] on `arg` with the key; the result to
    /// the entry's `+8` (through the array pointer read before the call),
    /// state 1 to `+4`, the context destroyed. `eax`'s upper bits are the
    /// original's frame address; callers read `al`.
    pub fn resolve_entries(h: &mut impl Host, this: u32, arg: u32, frame: u32) -> u32 {
        let vt = h.rebase(CONTEXT_VTABLE);
        let mut i = 0u32;
        while i < count(h, this.wrapping_add(0x108)) {
            let base: u32 = h.read(this.wrapping_add(0x10c));
            let e = base.wrapping_add(i.wrapping_mul(12));
            let key: u32 = h.read(e);
            h.write(frame, vt);
            h.write(frame.wrapping_add(4), this);
            h.write(frame.wrapping_add(8), key);
            let key: u32 = h.read(e);
            let r = h.call_sym(RESOLVE, Conv::Thiscall(arg), &[key, frame]);
            h.write(e.wrapping_add(8), r);
            let base: u32 = h.read(this.wrapping_add(0x10c));
            h.write(base.wrapping_add(i.wrapping_mul(12)).wrapping_add(4), 1u32);
            h.write(frame, vt);
            h.call_sym(CONTEXT_DTOR, Conv::Thiscall(frame), &[]);
            i += 1;
        }
        with_al(count(h, this.wrapping_add(0x108)), true)
    }

    /// `0x005cd920` (`thiscall(i, a, n, pairs, m, words)`, `ret 0x18`,
    /// `al`): optimiser node `i` (`0x11c` bytes from `[this]`, count `+4`)
    /// set up with `a`, linked with the `n` `(node, slot)` pairs both ways,
    /// and given `m` outputs (`words`' entries, or 1 each without
    /// `words`). `al` 0 for an index out of range (over `this`), else 1
    /// over the last call's `eax`.
    #[allow(clippy::too_many_arguments)]
    pub fn link_node(
        h: &mut impl Host,
        this: u32,
        i: u32,
        a: u32,
        n: u32,
        pairs: u32,
        m: u32,
        words: u32,
    ) -> u32 {
        if i >= h.read::<u32>(this.wrapping_add(4)) {
            return with_al(this, false);
        }
        let nodes: u32 = h.read(this);
        let e = i.wrapping_mul(0x11c).wrapping_add(nodes);
        let mut last = h.call_sym(NODE_SETUP, Conv::Thiscall(e), &[a]);
        for k in 0..n {
            let p = pairs.wrapping_add(k.wrapping_mul(8));
            let j: u32 = h.read(p);
            let slot: u32 = h.read(p.wrapping_add(4));
            let nodes: u32 = h.read(this);
            let other = j.wrapping_mul(0x11c).wrapping_add(nodes);
            h.call_sym(NODE_LINK_IN, Conv::Thiscall(e), &[other, slot]);
            let j: u32 = h.read(p);
            let nodes: u32 = h.read(this);
            let other = j.wrapping_mul(0x11c).wrapping_add(nodes);
            last = h.call_sym(NODE_LINK_OUT, Conv::Thiscall(other), &[e]);
        }
        for k in 0..m {
            let w = if words == 0 {
                1
            } else {
                h.read(words.wrapping_add(k.wrapping_mul(4)))
            };
            last = h.call_sym(NODE_ADD_OUTPUT, Conv::Thiscall(e), &[w]);
        }
        with_al(last, true)
    }

    /// `0x005cd9e0` (`thiscall(n)`, `ret 4`): the optimiser's node table
    /// remade: the old one (`[this]`, its count in the word before it)
    /// destroyed by the CRT's vector destructor and freed; `n` nodes
    /// allocated (`n * 0x11c + 4` bytes, saturating, through the array
    /// `new`), the count stored first and the nodes constructed; `+0` the
    /// nodes, `+4` `n` (0 and 0 when the allocation failed); 1 or 0.
    pub fn make_nodes(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let old: u32 = h.read(this);
        if old != 0 {
            let k: u32 = h.read(old.wrapping_sub(4));
            let dtor = h.rebase(NODE_DTOR);
            h.call_sym(VECTOR_DTOR, Conv::Stdcall, &[old, 0x11c, k, dtor]);
            free(h, old.wrapping_sub(4));
        }
        let size = u64::from(n) * 0x11c + 4;
        let size = u32::try_from(size).unwrap_or(u32::MAX);
        let p = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[size]);
        let nodes = if p == 0 {
            0
        } else {
            h.write(p, n);
            let (ctor, dtor) = (h.rebase(NODE_CTOR), h.rebase(NODE_DTOR));
            let at = p.wrapping_add(4);
            h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[at, 0x11c, n, ctor, dtor]);
            at
        };
        h.write(this.wrapping_add(4), if nodes == 0 { 0 } else { n });
        h.write(this, nodes);
        u32::from(nodes != 0)
    }

    /// A time controller's value (its slot 4) as `fstp m32` keeps it.
    fn time_of(h: &mut impl Host, ctl: u32) -> f64 {
        let vt: u32 = h.read(ctl);
        let f: u32 = h.read(vt.wrapping_add(0x10));
        x87::load(h.call_st0(f, Conv::Thiscall(ctl), &[]).to_bits())
    }

    /// `0x00543330` (`thiscall(clip)`, `ret 4`): a `0x4c`-byte source
    /// record bound to `clip`: the record's time controller (`+0x24`)
    /// sampled at scale 1 with offset `-T` (the constant time) and at its
    /// old `+8`, the offset `+0x34` moved by the difference; `+0` the
    /// clip; the bind ([`RECORD_BIND`]) and the clip's range
    /// ([`CLIP_RANGE`], into `frame`: end, start) and when both succeed
    /// `+0x1c` 0, `+0x20` [`RECORD_SPEED`], the offset `start - time` and
    /// [`RECORD_SET_TIME`]. The failing call's `eax`, or the last one's.
    pub fn bind_record(h: &mut impl Host, rec: u32, clip: u32, frame: u32) -> u32 {
        let t = h.rebase(TIME_CONSTANT);
        let minus_t = f32_of(-x87::load(h.read(t)));
        let old = f32_of(x87::load(h.read(rec.wrapping_add(8))));
        let ctl = rec.wrapping_add(0x24);
        h.write(rec.wrapping_add(0x30), 1.0f32.to_bits());
        h.write(rec.wrapping_add(0x34), x87::store(minus_t));
        let r1 = time_of(h, ctl);
        h.write(rec.wrapping_add(0x30), x87::store(old));
        let r2 = time_of(h, ctl);
        h.write(rec, clip);
        let off = x87::load(h.read(rec.wrapping_add(0x34)));
        let moved = x87::add(x87::sub(r1, r2), off);
        h.write(rec.wrapping_add(0x34), x87::store(moved));
        let bind = rec.wrapping_add(0x38);
        let r = h.call_sym(RECORD_BIND, Conv::Thiscall(bind), &[clip]);
        if r != 0 {
            return r;
        }
        let end = frame;
        let start = frame.wrapping_add(4);
        let r = h.call_sym(CLIP_RANGE, Conv::Thiscall(clip), &[start, end]);
        if r != 0 {
            return r;
        }
        h.write(rec.wrapping_add(0x1c), 0u32);
        let speed = h.rebase(RECORD_SPEED);
        let speed = x87::store(x87::load(h.read(speed)));
        h.write(rec.wrapping_add(0x20), speed);
        h.write(rec.wrapping_add(0x34), 0u32);
        let s = x87::load(h.read(start));
        let vt: u32 = h.read(ctl);
        let f: u32 = h.read(vt.wrapping_add(0x10));
        let now = h.call_st0_f64(f, Conv::Thiscall(ctl), &[]);
        h.write(rec.wrapping_add(0x34), x87::store(x87::sub(s, now)));
        h.call_sym(RECORD_SET_TIME, Conv::Thiscall(bind), &[ctl])
    }

    /// `0x00543480` (`thiscall(i, clip)`, `ret 8`): source `i` of the
    /// component (`0x4c`-byte records at `[this + 0x18]`, count `+0x14`)
    /// set to `clip`: 3 out of range, 1 without a clip, the record's bind
    /// error ([`bind_record`]); else the blender (`+0x1c`) gets the source
    /// and its weight (`+4`), the component refreshes and `+0x40` takes
    /// `+0x10`; 0.
    pub fn set_source(h: &mut impl Host, this: u32, i: u32, clip: u32, frame: u32) -> u32 {
        if i >= count(h, this.wrapping_add(0x14)) {
            return 3;
        }
        if clip == 0 {
            return 1;
        }
        let recs: u32 = h.read(this.wrapping_add(0x18));
        let rec = i.wrapping_mul(0x4c).wrapping_add(recs);
        let r = bind_record(h, rec, clip, frame);
        if r != 0 {
            return r;
        }
        let blender = this.wrapping_add(0x1c);
        h.call_sym(
            BLENDER_SET_SOURCE,
            Conv::Thiscall(blender),
            &[i, rec.wrapping_add(0x38)],
        );
        let w = x87::store(x87::load(h.read(rec.wrapping_add(4))));
        h.call_sym(BLENDER_SET_WEIGHT, Conv::Thiscall(blender), &[i, w]);
        h.call_sym(COMPONENT_REFRESH, Conv::Thiscall(this), &[]);
        let v: u32 = h.read(this.wrapping_add(0x10));
        h.write(this.wrapping_add(0x40), v);
        0
    }

    /// `0x00544040` (`thiscall(t)`, `ret 4`): every source record's time
    /// set to `t`: `+8` = `t`, the controller sampled before and after
    /// its scale (`+0x30`) becomes `t`, the offset `+0x34` moved by the
    /// difference.
    pub fn set_times(h: &mut impl Host, this: u32, t: u32) {
        let n = count(h, this.wrapping_add(0x14));
        let t = x87::store(x87::load(t));
        for k in 0..n {
            let recs: u32 = h.read(this.wrapping_add(0x18));
            let rec = recs.wrapping_add(k.wrapping_mul(0x4c));
            h.write(rec.wrapping_add(8), t);
            let ctl = rec.wrapping_add(0x24);
            let r1 = time_of(h, ctl);
            h.write(rec.wrapping_add(0x30), t);
            let r2 = time_of(h, ctl);
            let off = x87::load(h.read(rec.wrapping_add(0x34)));
            h.write(
                rec.wrapping_add(0x34),
                x87::store(x87::add(x87::sub(r1, r2), off)),
            );
        }
    }
}
