//! List `phyre-anim-left`, batch 7 (M5 stage 1): `PAnimationSet`'s
//! packed data, built by `0x005114b0` into the cluster object
//! `0x00516240` allocates.
//!
//! The data starts with a header of `u16`s (its size, the channel, slot
//! and batch-key counts), the per-key channel counts, then the channels'
//! target indices and records and the slots' indices. The target lookup
//! (`0x005cca80`) and the bind table (`0x005cc670`) it consults live in
//! the caller's frame (`frame`), as does the key they are asked with.

#![allow(clippy::many_single_char_names)]

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The set's distinct batch keys `0x00516190` (thiscall).
pub const DISTINCT_KEYS: u32 = 0x0051_6190;
/// The target lookup's constructor and destructor (thiscall).
pub const LOOKUP_CTOR: u32 = 0x005c_ca80;
/// See [`LOOKUP_CTOR`].
pub const LOOKUP_DTOR: u32 = 0x005c_cc50;
/// A target's key in the lookup (thiscall on the lookup, `(target)`).
pub const LOOKUP_KEY: u32 = 0x005c_ce50;
/// The bind table's constructor, lookup `(&key)` and destructor
/// (thiscall).
pub const TABLE_CTOR: u32 = 0x005c_c670;
/// See [`TABLE_CTOR`].
pub const TABLE_FIND: u32 = 0x005c_c800;
/// See [`TABLE_CTOR`].
pub const TABLE_DTOR: u32 = 0x005c_c770;
/// A channel's batch sort key (thiscall).
pub const BATCH_SORT_KEY: u32 = 0x0050_e730;
/// MSVCR110's `qsort` import slot, and the channel record order.
pub const QSORT: u32 = 0x00b0_c3ac;
/// See [`QSORT`].
pub const RECORD_ORDER: u32 = 0x0051_0f60;
/// How many channel classes there are (the key histogram has four words
/// per class).
pub const KEY_SLOTS: u32 = 0x00ca_9738;
/// How many words of the caller's frame [`bridge::build_set_data`]
/// takes: the lookup (`ebp - 0x90`), the table (`- 0x70`) and the key
/// (`- 0x3c`).
pub const FRAME_WORDS: u32 = 0x64 / 4;

pub mod bridge {
    use super::{
        BATCH_SORT_KEY, Conv, DISTINCT_KEYS, Host, KEY_SLOTS, LOOKUP_CTOR, LOOKUP_DTOR, LOOKUP_KEY,
        Mem, NOT_OWNED, PHYRE_FREE, PHYRE_MALLOC, QSORT, RECORD_ORDER, TABLE_CTOR, TABLE_DTOR,
        TABLE_FIND,
    };

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    fn u16_at(m: &impl Mem, at: u32) -> u32 {
        u32::from(m.read::<u16>(at))
    }

    #[allow(clippy::cast_possible_truncation)] // the u16 stores
    fn put16(m: &mut impl Mem, at: u32, v: u32) {
        m.write(at, v as u16);
    }

    /// The key `0x005114b0` asks the table with (`{target class, 3,
    /// lookup key, 0}` at `key`), and the table's answer.
    fn table_find(h: &mut impl Host, frame: u32, class: u32, item: u32) -> u32 {
        let (lookup, table, key) = (frame, frame.wrapping_add(0x20), frame.wrapping_add(0x54));
        let r = h.call_sym(LOOKUP_KEY, Conv::Thiscall(lookup), &[item]);
        h.write(key.wrapping_add(8), r);
        h.write(key.wrapping_add(0xc), 0u32);
        h.write(key, class);
        h.write(key.wrapping_add(4), 3u32);
        h.call_sym(TABLE_FIND, Conv::Thiscall(table), &[key])
    }

    /// `0x005114b0` (`thiscall(targets, set)`, `ret 8`): `PAnimationSet`'s
    /// packed data at `this`: header `u16`s (`+0` its size rounded to 16,
    /// `+2` channels, `+4` slots, `+6` distinct batch keys); with the
    /// target lookup and bind table made over `targets` in the frame,
    /// each channel's record (24 bytes in a `PhyreMalloc` block: bind
    /// index, target `+0x20`, its type's `+8`, `+0x2c`, type `+8 << 24 |
    /// +0x30`, batch key, the channel's number) is counted under its key,
    /// the records sorted (`qsort`, [`RECORD_ORDER`]); the non-zero key
    /// counts (padded to 4) follow the header, then the records' indices
    /// and their four middle words; each slot (`0x34` bytes) gets its bind
    /// index and `+0x20`. The block freed, the table and lookup
    /// destroyed; 0.
    #[allow(clippy::too_many_lines)]
    pub fn build_set_data(h: &mut impl Host, this: u32, targets: u32, set: u32, frame: u32) -> u32 {
        put16(h, this.wrapping_add(4), 0);
        h.write(this, 0x10u32);
        let chans = count(h, set.wrapping_add(4));
        let slots = count(h, set.wrapping_add(0xc));
        let keys = h.call_sym(DISTINCT_KEYS, Conv::Thiscall(set), &[]);
        put16(h, this.wrapping_add(6), keys);
        put16(h, this.wrapping_add(4), slots);
        let words = ((keys & 0xffff).wrapping_add(3) & !3)
            .wrapping_add(slots.wrapping_add(2).wrapping_add(chans).wrapping_mul(2));
        let size = words.wrapping_mul(2).wrapping_add(0xf) & 0xfff0;
        put16(h, this.wrapping_add(2), chans);
        put16(h, this, size);
        let (lookup, table) = (frame, frame.wrapping_add(0x20));
        h.call_sym(LOOKUP_CTOR, Conv::Thiscall(lookup), &[targets]);
        h.call_sym(TABLE_CTOR, Conv::Thiscall(table), &[targets]);
        let header = |m: &dyn Fn(u32) -> u32| {
            let k = m(6).wrapping_add(3) & !3;
            (k, m(2), m(4))
        };
        let rd = |a: u32| u16_at(h, this.wrapping_add(a));
        let (k4, c16, s16) = header(&rd);
        let base = this.wrapping_add(
            k4.wrapping_add(s16.wrapping_add(c16.wrapping_add(2)).wrapping_mul(2))
                .wrapping_mul(2),
        );
        let mut tail = base.wrapping_add(base.wrapping_neg() & 3);
        let block = if chans == 0 {
            0
        } else {
            h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[chans.wrapping_mul(24), 4])
        };
        let held = if block == 0 { 0 } else { chans };
        if block != 0 {
            let slots_at = h.rebase(KEY_SLOTS);
            let classes = h.read::<u32>(slots_at).wrapping_mul(4);
            let mut histogram = vec![0u16; classes as usize];
            for i in 0..chans {
                let list: u32 = h.read(set.wrapping_add(8));
                let ch: u32 = h.read(list.wrapping_add(4 * i));
                let target: u32 = h.read(ch.wrapping_add(0x24));
                let class: u32 = h.read(ch.wrapping_add(0x1c));
                let v = table_find(h, frame, class, ch);
                let rec = block.wrapping_add(24 * i);
                put16(h, rec, v);
                let w: u32 = h.read(ch.wrapping_add(0x20));
                put16(h, rec.wrapping_add(2), w);
                put16(h, rec.wrapping_add(0x10), i);
                let w: u32 = h.read(target.wrapping_add(8));
                h.write(rec.wrapping_add(4), w);
                let w: u32 = h.read(ch.wrapping_add(0x2c));
                h.write(rec.wrapping_add(8), w);
                let class: u32 = h.read(ch.wrapping_add(0x1c));
                let id: u32 = h.read(class.wrapping_add(8));
                let low: u32 = h.read(ch.wrapping_add(0x30));
                h.write(rec.wrapping_add(0xc), id << 24 | low);
                let key = h.call_sym(BATCH_SORT_KEY, Conv::Thiscall(ch), &[]);
                h.write(rec.wrapping_add(0x14), key);
                let slot = key as usize;
                if slot >= histogram.len() {
                    histogram.resize(slot + 1, 0);
                }
                histogram[slot] = histogram[slot].wrapping_add(1);
            }
            let qsort_slot = h.rebase(QSORT);
            let qsort: u32 = h.read(qsort_slot);
            let order = h.rebase(RECORD_ORDER);
            h.call(qsort, Conv::Cdecl, &[block, chans, 24, order]);
            let mut n = 0u32;
            for &c in histogram.iter().take(classes as usize) {
                if c != 0 {
                    put16(h, this.wrapping_add(8 + 2 * n), u32::from(c));
                    n = n.wrapping_add(1);
                }
            }
            while n & 3 != 0 {
                put16(h, this.wrapping_add(8 + 2 * n), 0);
                n = n.wrapping_add(1);
            }
            let k = u16_at(h, this.wrapping_add(6)).wrapping_add(3) & !3;
            let index_at = this.wrapping_add(k.wrapping_add(4).wrapping_mul(2));
            for i in 0..chans {
                let rec = block.wrapping_add(24 * i);
                let w: u32 = h.read(rec);
                h.write(index_at.wrapping_add(4 * i), w);
                for o in 0..4u32 {
                    let w: u32 = h.read(rec.wrapping_add(4 + 4 * o));
                    h.write(tail.wrapping_add(4 * o), w);
                }
                tail = tail.wrapping_add(16);
            }
        }
        let k = u16_at(h, this.wrapping_add(6)).wrapping_add(3) & !3;
        let c = u16_at(h, this.wrapping_add(2));
        let slot_at = this.wrapping_add(
            k.wrapping_add(c.wrapping_add(2).wrapping_mul(2))
                .wrapping_mul(2),
        );
        for j in 0..slots {
            let recs: u32 = h.read(set.wrapping_add(0x10));
            let s = recs.wrapping_add(0x34 * j);
            let class: u32 = h.read(s.wrapping_add(0x1c));
            let v = table_find(h, frame, class, s);
            put16(h, slot_at.wrapping_add(4 * j), v);
            let w: u32 = h.read(s.wrapping_add(0x20));
            put16(h, slot_at.wrapping_add(4 * j + 2), w);
        }
        if held & NOT_OWNED == 0 && block != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[block]);
        }
        h.call_sym(TABLE_DTOR, Conv::Thiscall(table), &[]);
        h.call_sym(LOOKUP_DTOR, Conv::Thiscall(lookup), &[]);
        0
    }
}
