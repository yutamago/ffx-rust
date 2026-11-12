//! M5 finish, WP5 part 2: the light sort's `std::sort` family
//! (`0x004d0fa0`..`0x004d2300`), MSVC 2012's introsort in the three
//! instantiations [`crate::m5p_scene_i`] ports the partition step of:
//! 4-byte keys with unsigned `<` ([`Order::Below`]), 4-byte values with a
//! predicate called by value ([`Order::ByValue`]) and 8-byte records with
//! a predicate called by pointer ([`Order::ByPointer`]). Ports that sort
//! (`p5_lights`, `l9_scene`, `m5p_scene_i`/`_j`) called these in Unicorn:
//! about 7 M emulated blocks in gate run 1.
//!
//! Every function is cdecl; the predicate is game code (cdecl, two words,
//! `al`). Each port compares the same elements in the same order as the
//! original and leaves the same memory. Values the original keeps in a
//! register or its frame (the element being inserted or sifted, the
//! partition's result pair) live in `frame`, game memory the caller
//! provides (the hook's `StackFrame`): the by-pointer predicate gets a
//! pointer to them. [`FRAME_WORDS`] words cover every function here.
//!
//! The original's own calls inside the family run as Rust calls; the
//! partition step is [`crate::m5p_scene_i::bridge::partition`] (which
//! calls the medians through the function table) and the 8-byte
//! `_Sort_heap` is [`crate::m5p_scene_j::bridge::sort_heap`].

pub use crate::m5p_scene_i::Order;

/// The frame words the ports here use: the partition's pair at `+0`, the
/// held element at `+8`.
pub const FRAME_WORDS: usize = 4;

/// The functions, by instantiation ([`Order::Below`], [`Order::ByValue`],
/// [`Order::ByPointer`]).
pub mod addr {
    /// `_Adjust_heap(first, hole, bottom, &value[, pred])`.
    pub const ADJUST_HEAP: [u32; 3] = [0x004d_0fa0, 0x004d_1030, 0x004d_10d0];
    /// `_Insertion_sort(first, last[, pred])`.
    pub const INSERTION_SORT: [u32; 3] = [0x004d_11c0, 0x004d_1240, 0x004d_12f0];
    /// `_Make_heap(first, last[, pred])`.
    pub const MAKE_HEAP: [u32; 3] = [0x004d_1430, 0x004d_14e0, 0x004d_1590];
    /// `_Med3(a, b, c, pred)` (by pointer; the other two are inlined).
    pub const MED3: u32 = 0x004d_1700;
    /// `_Median(first, mid, last[, pred])`.
    pub const MEDIAN: [u32; 3] = [0x004d_1780, 0x004d_18b0, 0x004d_1a90];
    /// `_Push_heap(first, hole, top, &value, pred)` (the keys' one is
    /// inlined).
    pub const PUSH_HEAP: [u32; 2] = [0x004d_1f10, 0x004d_1f80];
    /// `_Sort(first, last, ideal[, pred])`.
    pub const SORT: [u32; 3] = [0x004d_1ff0, 0x004d_20d0, 0x004d_21c0];
    /// `_Sort_heap(first, last[, pred])` (the records' one is
    /// `m5p_scene_j`'s).
    pub const SORT_HEAP: [u32; 3] = [0x004d_22b0, 0x004d_2300, 0x004d_2360];
}

pub mod bridge {
    //! The ports over game memory.

    use ffx_graphic::host::{Conv, Host};

    use super::Order;
    use crate::{m5p_scene_i, m5p_scene_j};

    /// The insertion sort's limit: ranges of at most 32 elements.
    const ISORT_MAX: i32 = 32;

    /// Whether the element at `a` orders before the one at `b`.
    fn less(h: &mut impl Host, order: Order, a: u32, b: u32) -> bool {
        match order {
            Order::Below => {
                let x: u32 = h.read(a);
                let y: u32 = h.read(b);
                x < y
            }
            Order::ByValue(pred) => {
                let x: u32 = h.read(a);
                let y: u32 = h.read(b);
                h.call(pred, Conv::Cdecl, &[x, y]) & 0xff != 0
            }
            Order::ByPointer(pred) => h.call(pred, Conv::Cdecl, &[a, b]) & 0xff != 0,
        }
    }

    /// The element at `src` copied to `dst`.
    fn copy(h: &mut impl Host, order: Order, dst: u32, src: u32) {
        for i in 0..order.size() / 4 {
            let w: u32 = h.read(src + 4 * i);
            h.write(dst + 4 * i, w);
        }
    }

    /// Exchanges the elements at `a` and `b`.
    fn swap(h: &mut impl Host, order: Order, a: u32, b: u32) {
        for i in 0..order.size() / 4 {
            let x: u32 = h.read(a + 4 * i);
            let y: u32 = h.read(b + 4 * i);
            h.write(a + 4 * i, y);
            h.write(b + 4 * i, x);
        }
    }

    /// The address of element `i` (signed, as the original's scaled index).
    #[allow(clippy::cast_sign_loss)] // two's complement, as `lea`
    fn at(order: Order, first: u32, i: i32) -> u32 {
        first.wrapping_add((i as u32).wrapping_mul(order.size()))
    }

    /// `(last - first) / size` (the original's `sar`).
    #[allow(clippy::cast_possible_wrap)] // the original's signed shift
    fn count(order: Order, first: u32, last: u32) -> i32 {
        (last.wrapping_sub(first) as i32) >> order.size().trailing_zeros()
    }

    /// `last - first` rounded down to whole elements, signed (the
    /// original's `and` and signed compares).
    #[allow(clippy::cast_possible_wrap)]
    fn span(order: Order, first: u32, last: u32) -> i32 {
        (last.wrapping_sub(first) & !(order.size() - 1)) as i32
    }

    /// The word the original leaves in `eax` after storing the element at
    /// `value`: its last word.
    fn last_word(h: &mut impl Host, order: Order, value: u32) -> u32 {
        h.read(value + order.size() - 4)
    }

    /// The sift-up shared by `_Adjust_heap` and `_Push_heap`: while `hole`
    /// is below `top` and its parent orders before the value at `value`,
    /// the parent moves down; the value goes into the hole. Whether the
    /// loop was entered.
    fn sift_up(
        h: &mut impl Host,
        order: Order,
        first: u32,
        hole: i32,
        top: i32,
        value: u32,
    ) -> bool {
        let entered = top < hole;
        let mut hole = hole;
        let mut i = hole.wrapping_sub(1) / 2;
        while top < hole && less(h, order, at(order, first, i), value) {
            copy(h, order, at(order, first, hole), at(order, first, i));
            hole = i;
            i = hole.wrapping_sub(1) / 2;
        }
        copy(h, order, at(order, first, hole), value);
        entered
    }

    /// `_Adjust_heap(first, hole, bottom, &value[, pred])` (`0x004d0fa0`,
    /// `0x004d1030`, `0x004d10d0`): the hole sunk to a leaf along the
    /// larger children, then the value at `value` sifted up from there
    /// (not above `hole`). `eax`: the value's last word.
    pub fn adjust_heap(
        h: &mut impl Host,
        order: Order,
        first: u32,
        hole: u32,
        bottom: u32,
        value: u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_wrap)] // signed indices
        let (top, bottom) = (hole as i32, bottom as i32);
        let mut hole = top;
        let mut i = hole.wrapping_mul(2).wrapping_add(2);
        while i < bottom {
            if less(
                h,
                order,
                at(order, first, i),
                at(order, first, i.wrapping_sub(1)),
            ) {
                i -= 1;
            }
            copy(h, order, at(order, first, hole), at(order, first, i));
            hole = i;
            i = i.wrapping_mul(2).wrapping_add(2);
        }
        if i == bottom {
            copy(
                h,
                order,
                at(order, first, hole),
                at(order, first, bottom.wrapping_sub(1)),
            );
            hole = bottom.wrapping_sub(1);
        }
        sift_up(h, order, first, hole, top, value);
        last_word(h, order, value)
    }

    /// `_Push_heap(first, hole, top, &value, pred)` (`0x004d1f10` by
    /// value, `0x004d1f80` by pointer): the value sifted up from `hole`.
    /// `eax`: by value the value when `top < hole`, else `first`; by
    /// pointer the value's last word.
    pub fn push_heap(
        h: &mut impl Host,
        order: Order,
        first: u32,
        hole: u32,
        top: u32,
        value: u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_wrap)] // signed indices
        let entered = sift_up(h, order, first, hole as i32, top as i32, value);
        match order {
            Order::ByValue(_) if !entered => first,
            _ => last_word(h, order, value),
        }
    }

    /// `_Make_heap(first, last[, pred])` (`0x004d1430`, `0x004d14e0`,
    /// `0x004d1590`): from the middle down to 0, each element taken out
    /// (to `frame + 8`) and sifted back in.
    pub fn make_heap(h: &mut impl Host, order: Order, first: u32, last: u32, frame: u32) {
        let bottom = count(order, first, last);
        let mut hole = bottom / 2;
        let value = frame + 8;
        while hole > 0 {
            hole -= 1;
            copy(h, order, value, at(order, first, hole));
            #[allow(clippy::cast_sign_loss)] // passed on as words
            adjust_heap(h, order, first, hole as u32, bottom as u32, value);
        }
    }

    /// `_Sort_heap(first, last[, pred])` (`0x004d22b0`, `0x004d2300`; the
    /// records' `0x004d2360` is [`m5p_scene_j::bridge::sort_heap`]): while
    /// two or more are left, the last taken out (to `frame + 8`), the top
    /// moved to its place and the taken one sifted in.
    pub fn sort_heap(h: &mut impl Host, order: Order, first: u32, last: u32, frame: u32) {
        if let Order::ByPointer(pred) = order {
            m5p_scene_j::bridge::sort_heap(h, first, last, pred, frame + 8);
            return;
        }
        let size = order.size();
        let value = frame + 8;
        let mut end = last;
        #[allow(clippy::cast_possible_wrap)] // signed compare
        while span(order, first, end) > size as i32 {
            end = end.wrapping_sub(size);
            copy(h, order, value, end);
            copy(h, order, end, first);
            #[allow(clippy::cast_sign_loss)] // passed on as a word
            let len = count(order, first, end) as u32;
            adjust_heap(h, order, first, 0, len, value);
        }
    }

    /// `_Insertion_sort(first, last[, pred])` (`0x004d11c0`, `0x004d1240`,
    /// `0x004d12f0`): each element (held at `frame + 8`) moved to the
    /// front when it orders before the first (the ones before it shifted
    /// up a place), else down past the ones it orders before.
    pub fn insertion_sort(h: &mut impl Host, order: Order, first: u32, last: u32, frame: u32) {
        let size = order.size();
        if first == last {
            return;
        }
        let value = frame + 8;
        let mut next = first.wrapping_add(size);
        while next != last {
            copy(h, order, value, next);
            if less(h, order, value, first) {
                let n = next.wrapping_sub(first) as usize;
                let mut moved = vec![0u8; n];
                h.read_bytes(first, &mut moved);
                h.write_bytes(first.wrapping_add(size), &moved);
                copy(h, order, first, value);
            } else {
                let mut hole = next;
                let mut prev = next.wrapping_sub(size);
                while less(h, order, value, prev) {
                    copy(h, order, hole, prev);
                    hole = prev;
                    prev = prev.wrapping_sub(size);
                }
                copy(h, order, hole, value);
            }
            next = next.wrapping_add(size);
        }
    }

    /// `_Med3(a, b, c[, pred])` (`0x004d1700` by pointer; inlined in the
    /// other two medians): the three ordered by at most three exchanges.
    pub fn med3(h: &mut impl Host, order: Order, a: u32, b: u32, c: u32) {
        if less(h, order, b, a) {
            swap(h, order, b, a);
        }
        if less(h, order, c, b) {
            swap(h, order, c, b);
            if less(h, order, b, a) {
                swap(h, order, b, a);
            }
        }
    }

    /// `_Median(first, mid, last[, pred])` (`0x004d1780`, `0x004d18b0`,
    /// `0x004d1a90`; `last` is the last element): over 40 elements Tukey's
    /// ninther (three medians of three spaced `(n + 1) / 8` apart, then
    /// their median), else the median of the three.
    pub fn median(h: &mut impl Host, order: Order, first: u32, mid: u32, last: u32) {
        let n = count(order, first, last);
        if n > 40 {
            #[allow(clippy::cast_sign_loss)] // a positive step
            let step = ((n + 1) / 8) as u32 * order.size();
            let two = 2 * step;
            let (a, b) = (first.wrapping_add(step), first.wrapping_add(two));
            med3(h, order, first, a, b);
            let (a, b) = (mid.wrapping_sub(step), mid.wrapping_add(step));
            med3(h, order, a, mid, b);
            let (a, b) = (last.wrapping_sub(two), last.wrapping_sub(step));
            med3(h, order, a, b, last);
            let (a, b) = (first.wrapping_add(step), last.wrapping_sub(step));
            med3(h, order, a, mid, b);
        } else {
            med3(h, order, first, mid, last);
        }
    }

    /// `_Sort(first, last, ideal[, pred])` (`0x004d1ff0`, `0x004d20d0`,
    /// `0x004d21c0`): while over 32 elements and `ideal` above 0, the
    /// range partitioned (the pair at `frame`), the smaller side sorted
    /// recursively with `ideal / 2 + ideal / 4` and the larger one
    /// looped on; at `ideal <= 0` a heap sort; at the end an insertion
    /// sort of what is left when two or more.
    pub fn sort(h: &mut impl Host, order: Order, first: u32, last: u32, ideal: u32, frame: u32) {
        let (mut first, mut last) = (first, last);
        #[allow(clippy::cast_possible_wrap)] // signed, as the original's
        let mut ideal = ideal as i32;
        #[allow(clippy::cast_possible_wrap)]
        let size = order.size() as i32;
        let mut n = count(order, first, last);
        while n > ISORT_MAX {
            if ideal <= 0 {
                if span(order, first, last) > size {
                    make_heap(h, order, first, last, frame);
                }
                sort_heap(h, order, first, last, frame);
                return;
            }
            m5p_scene_i::bridge::partition(h, order, frame, first, last);
            let lo: u32 = h.read(frame);
            let hi: u32 = h.read(frame + 4);
            let half = ideal / 2;
            ideal = half + half / 2;
            #[allow(clippy::cast_sign_loss)] // passed on as a word
            if span(order, first, lo) < span(order, hi, last) {
                sort(h, order, first, lo, ideal as u32, frame);
                first = hi;
            } else {
                sort(h, order, hi, last, ideal as u32, frame);
                last = lo;
            }
            n = count(order, first, last);
        }
        if n > 1 {
            insertion_sort(h, order, first, last, frame);
        }
    }
}
