//! M5 finish, WP5 part 5 (wp5f): the visibility check's occluder sort
//! (`PVisibilityCheck::buildOccluders` sorts its entries before it builds
//! the occluders): the C++ library's introsort as `FFX.exe` instantiates
//! it for 8-byte entries (a word and an `f32` key at `+4`) ordered by
//! descending key, read from the machine code. Every helper is cdecl and
//! passes the empty predicate along as two words (`pred`), which nothing
//! reads; the x87 compares are the plain IEEE ones (a NaN key compares
//! false either way).

pub mod bridge {
    use ffx_graphic::host::Host;

    /// The introsort loop `(first, last, ideal, pred)`, the partition
    /// `(&out, first, last, pred)`, the median `(first, mid, last, pred)`,
    /// the median of three `(a, b, c, pred)`, the insertion sort `(first,
    /// last, pred, 0)`, make-heap `(first, last, pred, 0, 0)`, sort-heap
    /// `(first, last, pred)`, adjust-heap `(first, hole, bottom, &value,
    /// pred)` and push-heap `(first, hole, top, &value, pred)`.
    pub const SORT: u32 = 0x009f_16c0;
    pub const PARTITION: u32 = 0x009f_1a30;
    pub const MEDIAN: u32 = 0x009f_1180;
    pub const MED3: u32 = 0x009f_1060;
    pub const INSERTION_SORT: u32 = 0x009f_0c80;
    pub const MAKE_HEAP: u32 = 0x009f_0e40;
    pub const SORT_HEAP: u32 = 0x009f_1900;
    pub const ADJUST_HEAP: u32 = 0x009f_0ae0;
    pub const PUSH_HEAP: u32 = 0x009f_15d0;

    /// The library's `_ISORT_MAX`: shorter ranges get the insertion sort.
    const ISORT_MAX: i32 = 32;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn back(p: u32, o: u32) -> u32 {
        p.wrapping_sub(o)
    }

    /// Entry `i` from `first`.
    fn nth(first: u32, i: i32) -> u32 {
        first.wrapping_add(i.cast_unsigned().wrapping_mul(8))
    }

    fn key(h: &impl Host, e: u32) -> f32 {
        f32::from_bits(h.read(at(e, 4)))
    }

    /// The order: `a` goes before `b` when its key is greater.
    fn before(h: &impl Host, a: u32, b: u32) -> bool {
        key(h, a) > key(h, b)
    }

    fn get(h: &impl Host, e: u32) -> [u32; 2] {
        [h.read(e), h.read(at(e, 4))]
    }

    fn put(h: &mut impl Host, e: u32, v: [u32; 2]) {
        h.write(e, v[0]);
        h.write(at(e, 4), v[1]);
    }

    fn swap(h: &mut impl Host, a: u32, b: u32) {
        let (x, y) = (get(h, a), get(h, b));
        put(h, a, y);
        put(h, b, x);
    }

    /// Entries between `first` and `last`, as the original counts them
    /// (the byte difference shifted down, signed).
    fn count(first: u32, last: u32) -> i32 {
        last.wrapping_sub(first).cast_signed() >> 3
    }

    /// The introsort loop (`0x009f16c0`): while more than 32 entries are
    /// left and `ideal` allows, partition ([`partition`]) and sort the
    /// smaller side first (recursively), the other in the loop (`ideal`
    /// down to three quarters each time); then a heap sort when the
    /// divisions ran out, else the insertion sort for more than one.
    pub fn sort(h: &mut impl Host, mut first: u32, mut last: u32, mut ideal: i32, pred: [u32; 2]) {
        let mut n = count(first, last);
        while n > ISORT_MAX {
            if ideal <= 0 {
                if (last.wrapping_sub(first) & !7).cast_signed() > 8 {
                    make_heap(h, first, last, pred);
                }
                sort_heap(h, first, last, pred);
                return;
            }
            let (pf, pl) = partition(h, first, last, pred);
            ideal = ideal / 2 + ideal / 2 / 2;
            let low = (pf.wrapping_sub(first) & !7).cast_signed();
            let high = (last.wrapping_sub(pl) & !7).cast_signed();
            if low < high {
                sort(h, first, pf, ideal, pred);
                first = pl;
            } else {
                sort(h, pl, last, ideal, pred);
                last = pf;
            }
            n = count(first, last);
        }
        if n > 1 {
            insertion_sort(h, first, last, pred);
        }
    }

    /// The unguarded partition (`0x009f1a30`) -> `(pivot first, pivot
    /// last)`: the median of the range's ends and middle ([`median`]) is
    /// the pivot; its equal neighbours join it; then the entries before
    /// it go left, after it right, equal ones into the pivot run (the
    /// scans' swaps unconditional, as the original's), rotating the pivot
    /// when one side runs out.
    pub fn partition(h: &mut impl Host, first: u32, last: u32, pred: [u32; 2]) -> (u32, u32) {
        let mid = nth(first, count(first, last) / 2);
        median(h, first, mid, back(last, 8), pred);
        let mut pf = mid;
        let mut pl = at(pf, 8);
        while first < pf && !before(h, back(pf, 8), pf) && !before(h, pf, back(pf, 8)) {
            pf = back(pf, 8);
        }
        while pl < last && !before(h, pl, pf) && !before(h, pf, pl) {
            pl = at(pl, 8);
        }
        let mut gf = pl;
        let mut gl = pf;
        loop {
            while gf < last {
                if before(h, pf, gf) {
                    // after the pivot already
                } else if before(h, gf, pf) {
                    break;
                } else {
                    swap(h, pl, gf);
                    pl = at(pl, 8);
                }
                gf = at(gf, 8);
            }
            while first < gl {
                let g = back(gl, 8);
                if before(h, g, pf) {
                    // before the pivot already
                } else if before(h, pf, g) {
                    break;
                } else {
                    pf = back(pf, 8);
                    swap(h, pf, g);
                }
                gl = g;
            }
            if gl == first && gf == last {
                return (pf, pl);
            }
            if gl == first {
                // no room below: the pivot run moves up
                if pl != gf {
                    swap(h, pf, pl);
                }
                pl = at(pl, 8);
                swap(h, pf, gf);
                pf = at(pf, 8);
                gf = at(gf, 8);
            } else if gf == last {
                // no room above: the pivot run moves down
                gl = back(gl, 8);
                pf = back(pf, 8);
                if gl != pf {
                    swap(h, gl, pf);
                }
                pl = back(pl, 8);
                swap(h, pf, pl);
            } else {
                gl = back(gl, 8);
                swap(h, gf, gl);
                gf = at(gf, 8);
            }
        }
    }

    /// [`partition`] as the original returns it: the pair at `out`,
    /// `out` returned.
    pub fn partition_out(
        h: &mut impl Host,
        out: u32,
        first: u32,
        last: u32,
        pred: [u32; 2],
    ) -> u32 {
        let (pf, pl) = partition(h, first, last, pred);
        h.write(out, pf);
        h.write(at(out, 4), pl);
        out
    }

    /// The median (`0x009f1180`, `last` inclusive): of nine for more than
    /// 40 entries (steps of an eighth), else of three ([`med3`]).
    pub fn median(h: &mut impl Host, first: u32, mid: u32, last: u32, pred: [u32; 2]) {
        let n = count(first, last);
        if n > 40 {
            let step = (n + 1) / 8;
            let s = step.cast_unsigned().wrapping_mul(8);
            med3(h, first, at(first, s), at(first, s.wrapping_mul(2)), pred);
            med3(h, back(mid, s), mid, at(mid, s), pred);
            med3(h, back(last, s.wrapping_mul(2)), back(last, s), last, pred);
            med3(h, at(first, s), mid, back(last, s), pred);
        } else {
            med3(h, first, mid, last, pred);
        }
    }

    /// The median of three (`0x009f1060`): `a`, `b`, `c` put in order.
    pub fn med3(h: &mut impl Host, a: u32, b: u32, c: u32, _pred: [u32; 2]) {
        if before(h, b, a) {
            swap(h, b, a);
        }
        if before(h, c, b) {
            swap(h, c, b);
            if before(h, b, a) {
                swap(h, b, a);
            }
        }
    }

    /// The insertion sort (`0x009f0c80`): each entry moved down past the
    /// ones it goes before (to the front when it goes before the first).
    pub fn insertion_sort(h: &mut impl Host, first: u32, last: u32, _pred: [u32; 2]) {
        if first == last {
            return;
        }
        let mut next = at(first, 8);
        while next != last {
            let v = get(h, next);
            let k = f32::from_bits(v[1]);
            if key(h, first) < k {
                let mut p = next;
                while p != first {
                    let w = get(h, back(p, 8));
                    put(h, p, w);
                    p = back(p, 8);
                }
                put(h, first, v);
            } else {
                let mut hole = next;
                loop {
                    let prev = back(hole, 8);
                    if key(h, prev) < k {
                        let w = get(h, prev);
                        put(h, hole, w);
                        hole = prev;
                    } else {
                        break;
                    }
                }
                put(h, hole, v);
            }
            next = at(next, 8);
        }
    }

    /// Make-heap (`0x009f0e40`): each parent from the last one down sifted
    /// ([`adjust_heap`]'s walk, inlined in the original).
    pub fn make_heap(h: &mut impl Host, first: u32, last: u32, _pred: [u32; 2]) {
        let bottom = count(first, last);
        let mut hole = bottom / 2;
        while hole > 0 {
            hole -= 1;
            let v = get(h, nth(first, hole));
            adjust(h, first, hole, bottom, v);
        }
    }

    /// Sort-heap (`0x009f1900`): while more than one entry is left, the
    /// top swapped to the end and the rest re-heaped.
    pub fn sort_heap(h: &mut impl Host, first: u32, last: u32, _pred: [u32; 2]) {
        let mut end = last;
        while (end.wrapping_sub(first) & !7).cast_signed() > 8 {
            end = back(end, 8);
            let v = get(h, end);
            let top = get(h, first);
            put(h, end, top);
            adjust(h, first, 0, count(first, end), v);
        }
    }

    /// Adjust-heap (`0x009f0ae0`) with the value at `value` (read once:
    /// the callers' values are their own locals).
    pub fn adjust_heap(
        h: &mut impl Host,
        first: u32,
        hole: i32,
        bottom: i32,
        value: u32,
        _pred: [u32; 2],
    ) {
        let v = get(h, value);
        adjust(h, first, hole, bottom, v);
    }

    /// The hole at `hole` moved down to the bottom along the larger
    /// children, then `v` pushed up from there ([`push`]).
    fn adjust(h: &mut impl Host, first: u32, hole: i32, bottom: i32, v: [u32; 2]) {
        let top = hole;
        let mut hole = hole;
        let mut idx = hole.wrapping_mul(2).wrapping_add(2);
        while idx < bottom {
            if key(h, nth(first, idx - 1)) < key(h, nth(first, idx)) {
                idx -= 1;
            }
            let w = get(h, nth(first, idx));
            put(h, nth(first, hole), w);
            hole = idx;
            idx = idx.wrapping_mul(2).wrapping_add(2);
        }
        if idx == bottom {
            let w = get(h, nth(first, bottom - 1));
            put(h, nth(first, hole), w);
            hole = bottom - 1;
        }
        push(h, first, hole, top, v);
    }

    /// Push-heap (`0x009f15d0`) with the value at `value` (read once, as
    /// [`adjust_heap`]).
    pub fn push_heap(
        h: &mut impl Host,
        first: u32,
        hole: i32,
        top: i32,
        value: u32,
        _pred: [u32; 2],
    ) {
        let v = get(h, value);
        push(h, first, hole, top, v);
    }

    /// `v` moved up from `hole` while its parent's key is smaller (down
    /// to `top`).
    fn push(h: &mut impl Host, first: u32, hole: i32, top: i32, v: [u32; 2]) {
        let k = f32::from_bits(v[1]);
        let mut hole = hole;
        let mut idx = (hole - 1) / 2;
        while top < hole && k < key(h, nth(first, idx)) {
            let w = get(h, nth(first, idx));
            put(h, nth(first, hole), w);
            hole = idx;
            idx = (hole - 1) / 2;
        }
        put(h, nth(first, hole), v);
    }
}
