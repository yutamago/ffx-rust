//! The visibility entries' sort: `std::sort` as FFX.exe instanced it for
//! 12-byte entries (`std_sort_VisibleEntries` `0x009f17c0` and its
//! helpers) and its comparator (`PCheckVisibleBoundsJob_compareEntries`
//! `0x009f2d70`).
//!
//! An entry is three words: the mesh instance, the depth key (`f32`) and
//! the render order (`i32`). The sort is the introspective one of the
//! compiler's library: insertion sort up to 32 entries, the "fat"
//! partition around a median of three (of nine above 40) otherwise, and a
//! heap sort once the partition budget (`ideal`, three quarters per level)
//! runs out. The helpers take the comparator as a function pointer and
//! hand it entry addresses; values the original keeps in its frame and
//! passes by address (the entry being placed) live in a 12-byte `tmp`
//! the caller provides (a frame of its own, or the shim's).
//!
//! Derived from FFX.exe's machine code: every helper does its loads,
//! stores and comparator calls in the original's order, so the bytes and
//! the comparator calls are the same, self-swaps included.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// Bytes per entry.
pub const ENTRY: u32 = 12;

/// An entry as stored: instance, depth key bits, render order.
pub type Entry = [u32; 3];

/// Render orders at or above this (`m_DObjKind` 1) form their own class,
/// sorted first.
pub const CLASS: i32 = 100_000_000;

/// `PCheckVisibleBoundsJob_compareEntries` on the values: `a` before `b`.
/// Class 1 (`order / 1e8 == 1`) first; then `order % 1000`, then
/// `(order % 100000) / 1000` ascending; then the key strictly smaller (a
/// NaN key is never before). `i32` division and remainder truncate, as
/// `idiv` does.
#[must_use]
pub fn compare(a_key: f32, a_order: i32, b_key: f32, b_order: i32) -> bool {
    let a1 = a_order / CLASS == 1;
    let b1 = b_order / CLASS == 1;
    if a1 != b1 {
        return a1;
    }
    let (a3, b3) = (a_order % 1000, b_order % 1000);
    if a3 != b3 {
        return a3 < b3;
    }
    let (a5, b5) = ((a_order % 100_000) / 1000, (b_order % 100_000) / 1000);
    if a5 != b5 {
        return a5 < b5;
    }
    a_key < b_key
}

/// Signed entry count between two addresses (`(last - first) / 12`, as
/// the original's `imul`-by-reciprocal division of the byte difference).
#[must_use]
#[allow(clippy::cast_possible_wrap)] // a pointer difference
pub fn count(first: u32, last: u32) -> i32 {
    (last.wrapping_sub(first) as i32) / ENTRY as i32
}

/// The entry `i` places after `p` (negative before).
#[must_use]
#[allow(clippy::cast_sign_loss)] // wrapping pointer arithmetic
pub fn at(p: u32, i: i32) -> u32 {
    p.wrapping_add((i.wrapping_mul(ENTRY as i32)) as u32)
}

pub mod bridge {
    //! The originals over game memory; each helper is its own hook.

    use super::{Conv, ENTRY, Entry, Host, Mem, at, compare, count};

    /// `PCheckVisibleBoundsJob_compareEntries(a, b)` (cdecl): `a` before
    /// `b`, in `al`.
    pub fn compare_entries(m: &impl Mem, a: u32, b: u32) -> bool {
        let ao: i32 = m.read(a + 8);
        let bo: i32 = m.read(b + 8);
        // the original reads the keys only when the orders tie; reading
        // them first reads the same memory
        compare(m.read(a + 4), ao, m.read(b + 4), bo)
    }

    /// The comparator call: `pred(a, b)` (cdecl), its `al`.
    pub fn less(h: &mut impl Host, pred: u32, a: u32, b: u32) -> bool {
        h.call(pred, Conv::Cdecl, &[a, b]) & 0xff != 0
    }

    fn load(h: &impl Mem, p: u32) -> Entry {
        h.read(p)
    }

    fn store(h: &mut impl Mem, p: u32, e: Entry) {
        h.write(p, e);
    }

    /// `iter_swap`: both read, then both written.
    fn swap(h: &mut impl Mem, a: u32, b: u32) {
        let (x, y) = (load(h, a), load(h, b));
        store(h, a, y);
        store(h, b, x);
    }

    /// `0x009f0d20` (cdecl `(first, last, pred, unused)`): insertion sort.
    /// The entry being placed is copied to `tmp`, which the comparator
    /// gets as its first argument.
    pub fn insertion_sort(h: &mut impl Host, tmp: u32, first: u32, last: u32, pred: u32) {
        if first == last {
            return;
        }
        let mut next = first + ENTRY;
        let mut prev = first;
        while next != last {
            let val = load(h, next);
            store(h, tmp, val);
            if less(h, pred, tmp, first) {
                // before everything: move [first, next) up one place
                let mut p = next;
                while p != first {
                    let e = load(h, p - ENTRY);
                    store(h, p, e);
                    p -= ENTRY;
                }
                store(h, first, val);
            } else {
                let mut hole = next;
                let mut q = prev;
                while less(h, pred, tmp, q) {
                    let e = load(h, q);
                    store(h, hole, e);
                    hole = q;
                    q -= ENTRY;
                }
                store(h, hole, val);
            }
            prev += ENTRY;
            next += ENTRY;
        }
    }

    /// `0x009f10e0` (cdecl `(a, b, c, pred)`): orders three entries.
    pub fn median3(h: &mut impl Host, a: u32, b: u32, c: u32, pred: u32) {
        if less(h, pred, b, a) {
            swap(h, b, a);
        }
        if less(h, pred, c, b) {
            swap(h, c, b);
            if less(h, pred, b, a) {
                swap(h, b, a);
            }
        }
    }

    /// `0x009f1230` (cdecl `(first, mid, last, pred)`; `last` is the last
    /// entry, not past it): the median of three, of nine above 40 entries.
    pub fn median(h: &mut impl Host, first: u32, mid: u32, last: u32, pred: u32) {
        let n = count(first, last);
        if n > 40 {
            let step = (n + 1) / 8;
            let one = at(first, step);
            median3(h, first, one, at(first, 2 * step), pred);
            median3(h, at(mid, -step), mid, at(mid, step), pred);
            let back = at(last, -step);
            median3(h, at(last, -2 * step), back, last, pred);
            median3(h, one, mid, back, pred);
        } else {
            median3(h, first, mid, last, pred);
        }
    }

    /// `0x009f1c60` (cdecl `(out, first, last, pred)` -> `out`): the fat
    /// partition. Returns the range of entries equal to the pivot
    /// (`out[0]`, `out[1]`).
    pub fn partition(h: &mut impl Host, first: u32, last: u32, pred: u32) -> (u32, u32) {
        let mid = at(first, count(first, last) / 2);
        median(h, first, mid, last - ENTRY, pred);
        // the run of entries equal to the pivot around it
        let mut pfirst = mid;
        let mut plast = mid + ENTRY;
        while first < pfirst {
            let before = pfirst - ENTRY;
            if less(h, pred, before, pfirst) || less(h, pred, pfirst, before) {
                break;
            }
            pfirst = before;
        }
        while plast < last {
            if less(h, pred, plast, pfirst) || less(h, pred, pfirst, plast) {
                break;
            }
            plast += ENTRY;
        }
        let mut gfirst = plast;
        let mut glast = pfirst;
        loop {
            // above the pivots: skip greater entries, gather equal ones
            while gfirst < last {
                if less(h, pred, pfirst, gfirst) {
                    gfirst += ENTRY;
                    continue;
                }
                if less(h, pred, gfirst, pfirst) {
                    break;
                }
                swap(h, plast, gfirst);
                plast += ENTRY;
                gfirst += ENTRY;
            }
            // below: skip smaller entries, gather equal ones
            while first < glast {
                let below = glast - ENTRY;
                if less(h, pred, below, pfirst) {
                    glast = below;
                    continue;
                }
                if less(h, pred, pfirst, below) {
                    break;
                }
                pfirst -= ENTRY;
                swap(h, pfirst, below);
                glast = below;
            }
            if glast == first {
                if gfirst == last {
                    return (pfirst, plast);
                }
                // no room below: move the pivots up
                if plast != gfirst {
                    swap(h, pfirst, plast);
                }
                plast += ENTRY;
                swap(h, pfirst, gfirst);
                pfirst += ENTRY;
                gfirst += ENTRY;
            } else {
                glast -= ENTRY;
                if gfirst == last {
                    // no room above: move the pivots down
                    pfirst -= ENTRY;
                    if glast != pfirst {
                        swap(h, glast, pfirst);
                    }
                    plast -= ENTRY;
                    swap(h, pfirst, plast);
                } else {
                    swap(h, gfirst, glast);
                    gfirst += ENTRY;
                }
            }
        }
    }

    /// [`partition`] as the original: the pair stored at `out`, `out`
    /// returned.
    pub fn partition_out(h: &mut impl Host, out: u32, first: u32, last: u32, pred: u32) -> u32 {
        let (a, b) = partition(h, first, last, pred);
        h.write(out, a);
        h.write(out + 4, b);
        out
    }

    /// `0x009f1630` (cdecl `(first, hole, top, val, pred)`): `push_heap`'s
    /// sift-up of `*val` from `hole` towards `top`.
    pub fn push_heap(h: &mut impl Host, first: u32, hole: i32, top: i32, val: u32, pred: u32) {
        let mut hole = hole;
        while top < hole {
            let parent = (hole - 1) / 2;
            let p = at(first, parent);
            if !less(h, pred, p, val) {
                break;
            }
            let e = load(h, p);
            store(h, at(first, hole), e);
            hole = parent;
        }
        let v = load(h, val);
        store(h, at(first, hole), v);
    }

    /// The hole's descent to the bottom (the larger child each step), as
    /// both [`adjust_heap`] and [`make_heap`] do it: the last hole.
    fn sift_down(h: &mut impl Host, first: u32, hole: i32, bottom: i32, pred: u32) -> i32 {
        let mut hole = hole;
        let mut child = 2 * hole + 2;
        while child < bottom {
            let c = at(first, child);
            if less(h, pred, c, c - ENTRY) {
                child -= 1;
            }
            let e = load(h, at(first, child));
            store(h, at(first, hole), e);
            hole = child;
            child = 2 * child + 2;
        }
        if child == bottom {
            let e = load(h, at(first, bottom - 1));
            store(h, at(first, hole), e);
            hole = bottom - 1;
        }
        hole
    }

    /// `0x009f0b60` (cdecl `(first, hole, bottom, val, pred)`):
    /// `adjust_heap`, the hole down to the bottom, then `*val` up.
    pub fn adjust_heap(h: &mut impl Host, first: u32, hole: i32, bottom: i32, val: u32, pred: u32) {
        let last = sift_down(h, first, hole, bottom, pred);
        push_heap(h, first, last, hole, val, pred);
    }

    /// `0x009f0f50` (cdecl `(first, last, pred, 0, 0)`): `make_heap`. The
    /// entry being placed is copied to `tmp`.
    pub fn make_heap(h: &mut impl Host, tmp: u32, first: u32, last: u32, pred: u32) {
        let bottom = count(first, last);
        let mut hole = bottom / 2;
        while hole > 0 {
            hole -= 1;
            let val = load(h, at(first, hole));
            store(h, tmp, val);
            let end = sift_down(h, first, hole, bottom, pred);
            push_heap(h, first, end, hole, tmp, pred);
        }
    }

    /// `0x009f1970` (cdecl `(first, last, pred)`): `sort_heap`. The entry
    /// taken off the end is copied to `tmp`.
    pub fn sort_heap(h: &mut impl Host, tmp: u32, first: u32, last: u32, pred: u32) {
        let mut end = last;
        while count(first, end) > 1 {
            let back = end - ENTRY;
            let val = load(h, back);
            store(h, tmp, val);
            let top = load(h, first);
            store(h, back, top);
            adjust_heap(h, first, 0, count(first, back), tmp, pred);
            end = back;
        }
    }

    /// `std_sort_VisibleEntries` `0x009f17c0` (cdecl `(first, last,
    /// ideal, pred)`): sorts `[first, last)`, `ideal` partitions deep
    /// before heap sort takes over.
    pub fn sort(h: &mut impl Host, tmp: u32, first: u32, last: u32, ideal: i32, pred: u32) {
        let (mut first, mut last, mut ideal) = (first, last, ideal);
        loop {
            let n = count(first, last);
            if n <= 32 {
                if n > 1 {
                    insertion_sort(h, tmp, first, last, pred);
                }
                return;
            }
            if ideal <= 0 {
                if count(first, last) > 1 {
                    make_heap(h, tmp, first, last, pred);
                }
                sort_heap(h, tmp, first, last, pred);
                return;
            }
            let (mid_first, mid_last) = partition(h, first, last, pred);
            ideal = ideal / 2 + (ideal / 2) / 2;
            if count(first, mid_first) < count(mid_last, last) {
                sort(h, tmp, first, mid_first, ideal, pred);
                first = mid_last;
            } else {
                sort(h, tmp, mid_last, last, ideal, pred);
                last = mid_first;
            }
        }
    }
}
