//! The game's `std::sort` instantiations for its 16- and 28-byte fixup
//! records (MSVC 2012's introsort over a comparator function pointer):
//! `_Sort`, `_Unguarded_partition`, `_Median`/`_Med3`, the heap
//! functions and `_Insertion_sort`. The algorithm is the standard
//! library's; the port follows the compiled code's order of comparisons
//! and moves, so a comparator sees exactly the calls the original makes.
//!
//! The comparator is a callback (`less(a, b)`, two record addresses); the
//! values the originals keep on the stack go to `tmp`, one record of
//! scratch memory the caller provides.

/// The 16-byte record instantiations, (address, function).
pub const SORT16: [(u32, Algo); 14] = [
    (0x0060_aeb0, Algo::AdjustHeap),
    (0x0060_b0f0, Algo::InsertionSort),
    (0x0060_b2d0, Algo::InsertionSort),
    (0x0060_b310, Algo::MakeHeap),
    (0x0060_b550, Algo::Med3),
    (0x0060_b6f0, Algo::Median),
    (0x0060_b950, Algo::PopHeapTo),
    (0x0060_b990, Algo::PopHeap),
    (0x0060_baa0, Algo::PopHeap),
    (0x0060_bb80, Algo::PushHeap),
    (0x0060_bc90, Algo::Sort),
    (0x0060_bed0, Algo::SortHeap),
    (0x0060_c030, Algo::Partition),
    (0x0060_c890, Algo::StdSort),
];

/// The 28-byte record instantiations.
pub const SORT28: [(u32, Algo); 14] = [
    (0x0060_afc0, Algo::AdjustHeap),
    (0x0060_b1f0, Algo::InsertionSort),
    (0x0060_b2f0, Algo::InsertionSort),
    (0x0060_b420, Algo::MakeHeap),
    (0x0060_b640, Algo::Med3),
    (0x0060_b790, Algo::Median),
    (0x0060_b9f0, Algo::PopHeapTo),
    (0x0060_ba40, Algo::PopHeap),
    (0x0060_bb00, Algo::PopHeap),
    (0x0060_bc00, Algo::PushHeap),
    (0x0060_bd80, Algo::Sort),
    (0x0060_bf60, Algo::SortHeap),
    (0x0060_c390, Algo::Partition),
    (0x0060_c8b0, Algo::StdSort),
];

/// Which algorithm an instantiation is (all cdecl, the comparator last).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algo {
    /// `_Adjust_heap(first, hole, bottom, &val, less)`.
    AdjustHeap,
    /// `_Insertion_sort(first, last, less)`.
    InsertionSort,
    /// `_Make_heap(first, last, less)`.
    MakeHeap,
    /// `_Med3(a, b, c, less)`.
    Med3,
    /// `_Median(first, mid, last, less)` (`last` inclusive).
    Median,
    /// `_Pop_heap(first, last, dest, &val, less)`.
    PopHeapTo,
    /// `_Pop_heap(first, last, less)`.
    PopHeap,
    /// `_Push_heap(first, hole, top, &val, less)`.
    PushHeap,
    /// `_Sort(first, last, ideal, less)`.
    Sort,
    /// `sort_heap(first, last, less)`.
    SortHeap,
    /// `std::sort(first, last, less)`: `_Sort` with the record count as
    /// the division budget.
    StdSort,
    /// `_Unguarded_partition(&result, first, last, less)`: the equal range
    /// `(pfirst, plast)` stored at `result`, which it returns.
    Partition,
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// One instantiation: the record size, a scratch record and the
    /// comparator.
    pub struct Sorter<F> {
        /// Bytes per record (16 or 28).
        pub size: u32,
        /// One record of scratch memory for the values the originals keep
        /// on the stack.
        pub tmp: u32,
        /// `less(a, b)` on two record addresses.
        pub less: F,
    }

    /// The signed record count between two addresses.
    #[allow(clippy::cast_possible_wrap)] // a record size
    fn count(first: u32, last: u32, size: u32) -> i32 {
        last.wrapping_sub(first).cast_signed() / size as i32
    }

    /// `(i - 1) / 2`, truncating as the originals' `cdq; sub; sar`.
    const fn parent(i: i32) -> i32 {
        (i - 1) / 2
    }

    impl<F> Sorter<F> {
        fn at(&self, first: u32, i: i32) -> u32 {
            first.wrapping_add(i.cast_unsigned().wrapping_mul(self.size))
        }

        fn copy<M: Mem>(&self, mem: &mut M, dst: u32, src: u32) {
            for k in (0..self.size).step_by(4) {
                let w: u32 = mem.read(src.wrapping_add(k));
                mem.write(dst.wrapping_add(k), w);
            }
        }

        fn swap<M: Mem>(&self, mem: &mut M, a: u32, b: u32) {
            for k in (0..self.size).step_by(4) {
                let x: u32 = mem.read(a.wrapping_add(k));
                let y: u32 = mem.read(b.wrapping_add(k));
                mem.write(b.wrapping_add(k), x);
                mem.write(a.wrapping_add(k), y);
            }
        }
    }

    impl<F> Sorter<F> {
        fn lt<M: Mem>(&mut self, mem: &mut M, a: u32, b: u32) -> bool
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            (self.less)(mem, a, b)
        }

        /// `_Push_heap`: `val` sifted up from `hole` while above `top`.
        pub fn push_heap<M: Mem>(
            &mut self,
            mem: &mut M,
            first: u32,
            mut hole: i32,
            top: i32,
            val: u32,
        ) where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            let mut idx = parent(hole);
            while top < hole {
                let p = self.at(first, idx);
                if !self.lt(mem, p, val) {
                    break;
                }
                let h = self.at(first, hole);
                self.copy(mem, h, p);
                hole = idx;
                idx = parent(hole);
            }
            let h = self.at(first, hole);
            self.copy(mem, h, val);
        }

        /// `_Adjust_heap`: the hole sifted down to the bottom along the
        /// larger children, then `val` pushed back up.
        pub fn adjust_heap<M: Mem>(
            &mut self,
            mem: &mut M,
            first: u32,
            mut hole: i32,
            bottom: i32,
            val: u32,
        ) where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            let top = hole;
            let mut idx = 2 * hole + 2;
            while idx < bottom {
                let (r, l) = (self.at(first, idx), self.at(first, idx - 1));
                if self.lt(mem, r, l) {
                    idx -= 1;
                }
                let (h, c) = (self.at(first, hole), self.at(first, idx));
                self.copy(mem, h, c);
                hole = idx;
                idx = 2 * idx + 2;
            }
            if idx == bottom {
                let (h, c) = (self.at(first, hole), self.at(first, bottom - 1));
                self.copy(mem, h, c);
                hole = bottom - 1;
            }
            self.push_heap(mem, first, hole, top, val);
        }

        /// `_Make_heap`: every inner node adjusted, bottom-up.
        pub fn make_heap<M: Mem>(&mut self, mem: &mut M, first: u32, last: u32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            let bottom = count(first, last, self.size);
            let mut hole = bottom / 2;
            while hole > 0 {
                hole -= 1;
                let (t, h) = (self.tmp, self.at(first, hole));
                self.copy(mem, t, h);
                self.adjust_heap(mem, first, hole, bottom, t);
            }
        }

        /// `_Pop_heap(first, last, dest, &val)`: the top to `dest`, `val`
        /// adjusted in from the root.
        pub fn pop_heap_to<M: Mem>(
            &mut self,
            mem: &mut M,
            first: u32,
            last: u32,
            dest: u32,
            val: u32,
        ) where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            self.copy(mem, dest, first);
            let n = count(first, last, self.size);
            self.adjust_heap(mem, first, 0, n, val);
        }

        /// `_Pop_heap(first, last)`: the top moved to the last record.
        pub fn pop_heap<M: Mem>(&mut self, mem: &mut M, first: u32, last: u32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            let end = last.wrapping_sub(self.size);
            let t = self.tmp;
            self.copy(mem, t, end);
            self.pop_heap_to(mem, first, end, end, t);
        }

        /// `sort_heap`: the top popped until one record is left.
        pub fn sort_heap<M: Mem>(&mut self, mem: &mut M, first: u32, mut last: u32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            while count(first, last, self.size) >= 2 {
                self.pop_heap(mem, first, last);
                last = last.wrapping_sub(self.size);
            }
        }

        /// `_Insertion_sort`.
        pub fn insertion_sort<M: Mem>(&mut self, mem: &mut M, first: u32, last: u32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            if first == last {
                return;
            }
            let s = self.size;
            let t = self.tmp;
            let mut next = first.wrapping_add(s);
            while next != last {
                self.copy(mem, t, next);
                if self.lt(mem, t, first) {
                    let mut e = next;
                    while e != first {
                        let src = e.wrapping_sub(s);
                        self.copy(mem, e, src);
                        e = src;
                    }
                    self.copy(mem, first, t);
                } else {
                    let mut hole = next;
                    let mut prev = next.wrapping_sub(s);
                    while self.lt(mem, t, prev) {
                        self.copy(mem, hole, prev);
                        hole = prev;
                        prev = prev.wrapping_sub(s);
                    }
                    self.copy(mem, hole, t);
                }
                next = next.wrapping_add(s);
            }
        }

        /// `_Med3`: the three records put in order.
        pub fn med3<M: Mem>(&mut self, mem: &mut M, a: u32, b: u32, c: u32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            if self.lt(mem, b, a) {
                self.swap(mem, a, b);
            }
            if self.lt(mem, c, b) {
                self.swap(mem, b, c);
                if self.lt(mem, b, a) {
                    self.swap(mem, a, b);
                }
            }
        }

        /// `_Median`: the median of three, or of nine above 40 records.
        pub fn median<M: Mem>(&mut self, mem: &mut M, first: u32, mid: u32, last: u32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            let n = count(first, last, self.size);
            if n > 40 {
                let step = self.size.wrapping_mul(((n + 1) / 8).cast_unsigned());
                let two = step.wrapping_mul(2);
                self.med3(
                    mem,
                    first,
                    first.wrapping_add(step),
                    first.wrapping_add(two),
                );
                self.med3(mem, mid.wrapping_sub(step), mid, mid.wrapping_add(step));
                self.med3(mem, last.wrapping_sub(two), last.wrapping_sub(step), last);
                self.med3(mem, first.wrapping_add(step), mid, last.wrapping_sub(step));
            } else {
                self.med3(mem, first, mid, last);
            }
        }

        /// `_Unguarded_partition`: the range split around a median pivot
        /// into less, equal and greater runs; the equal run's bounds.
        pub fn partition<M: Mem>(&mut self, mem: &mut M, first: u32, last: u32) -> (u32, u32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            let s = self.size;
            let mid = self.at(first, count(first, last, s) / 2);
            self.median(mem, first, mid, last.wrapping_sub(s));
            let mut pfirst = mid;
            let mut plast = pfirst.wrapping_add(s);
            while first < pfirst {
                let below = pfirst.wrapping_sub(s);
                if self.lt(mem, below, pfirst) || self.lt(mem, pfirst, below) {
                    break;
                }
                pfirst = below;
            }
            while plast < last {
                if self.lt(mem, plast, pfirst) || self.lt(mem, pfirst, plast) {
                    break;
                }
                plast = plast.wrapping_add(s);
            }
            let mut gfirst = plast;
            let mut glast = pfirst;
            loop {
                while gfirst < last {
                    if self.lt(mem, pfirst, gfirst) {
                    } else if self.lt(mem, gfirst, pfirst) {
                        break;
                    } else {
                        self.swap(mem, plast, gfirst);
                        plast = plast.wrapping_add(s);
                    }
                    gfirst = gfirst.wrapping_add(s);
                }
                while first < glast {
                    let g = glast.wrapping_sub(s);
                    if self.lt(mem, g, pfirst) {
                    } else if self.lt(mem, pfirst, g) {
                        break;
                    } else {
                        pfirst = pfirst.wrapping_sub(s);
                        self.swap(mem, pfirst, g);
                    }
                    glast = g;
                }
                if glast == first && gfirst == last {
                    return (pfirst, plast);
                }
                if glast == first {
                    if plast != gfirst {
                        self.swap(mem, pfirst, plast);
                    }
                    plast = plast.wrapping_add(s);
                    self.swap(mem, pfirst, gfirst);
                    pfirst = pfirst.wrapping_add(s);
                    gfirst = gfirst.wrapping_add(s);
                } else if gfirst == last {
                    glast = glast.wrapping_sub(s);
                    pfirst = pfirst.wrapping_sub(s);
                    if glast != pfirst {
                        self.swap(mem, glast, pfirst);
                    }
                    plast = plast.wrapping_sub(s);
                    self.swap(mem, pfirst, plast);
                } else {
                    glast = glast.wrapping_sub(s);
                    self.swap(mem, gfirst, glast);
                    gfirst = gfirst.wrapping_add(s);
                }
            }
        }

        /// `std::sort`: [`Self::sort`] with the record count as `ideal`.
        pub fn std_sort<M: Mem>(&mut self, mem: &mut M, first: u32, last: u32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            let n = count(first, last, self.size);
            self.sort(mem, first, last, n);
        }

        /// `_Sort`: quicksort while more than 32 records and the division
        /// budget `ideal` lasts, then heap sort or insertion sort.
        pub fn sort<M: Mem>(&mut self, mem: &mut M, mut first: u32, mut last: u32, mut ideal: i32)
        where
            F: FnMut(&mut M, u32, u32) -> bool,
        {
            let s = self.size;
            let mut n = count(first, last, s);
            while n > 32 && ideal > 0 {
                let (lo, hi) = self.partition(mem, first, last);
                ideal /= 2;
                ideal += ideal / 2;
                if count(first, lo, s) < count(hi, last, s) {
                    self.sort(mem, first, lo, ideal);
                    first = hi;
                } else {
                    self.sort(mem, hi, last, ideal);
                    last = lo;
                }
                n = count(first, last, s);
            }
            if n > 32 {
                if n > 1 {
                    self.make_heap(mem, first, last);
                }
                self.sort_heap(mem, first, last);
            } else if n > 1 {
                self.insertion_sort(mem, first, last);
            }
        }
    }
}
