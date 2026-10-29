//! M5 stage 1, list `scene`, batch 9: the light sort's partition step
//! (MSVC's `_Partition_by_median_guess_unchecked`), in three
//! instantiations: 4-byte keys with `<` (`0x004d2400`), 4-byte values
//! with a predicate called by value (`0x004d2560`) and 8-byte records
//! with a predicate called by pointer (`0x004d2740`). Each takes its
//! result pair by a hidden pointer (cdecl `(&out, first, last[, pred])`)
//! and returns that pointer.
//!
//! The predicate is game code (cdecl, two words, `al`); the median of
//! three (`0x004d1780`, `0x004d18b0`, `0x004d1a90`, cdecl) stays a call to
//! the original. Calls, swaps and their order are the original's.

use ffx_graphic::host::{Conv, Host};

/// The medians of three each instantiation calls first.
pub const MEDIANS: [u32; 3] = [0x004d_1780, 0x004d_18b0, 0x004d_1a90];

/// How an instantiation orders two elements.
#[derive(Clone, Copy, Debug)]
pub enum Order {
    /// 4-byte keys, unsigned `<`.
    Below,
    /// 4-byte values, `pred(a, b)` by value.
    ByValue(u32),
    /// 8-byte records, `pred(&a, &b)` by pointer.
    ByPointer(u32),
}

impl Order {
    /// The element size.
    #[must_use]
    pub const fn size(self) -> u32 {
        match self {
            Self::Below | Self::ByValue(_) => 4,
            Self::ByPointer(_) => 8,
        }
    }
}

pub mod bridge {
    //! The ports over game memory.

    use super::{Conv, Host, MEDIANS, Order};

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

    /// Exchanges the elements at `a` and `b`.
    fn swap(h: &mut impl Host, order: Order, a: u32, b: u32) {
        for i in 0..order.size() / 4 {
            let x: u32 = h.read(a + 4 * i);
            let y: u32 = h.read(b + 4 * i);
            h.write(a + 4 * i, y);
            h.write(b + 4 * i, x);
        }
    }

    /// `0x004d2400` (`Order::Below`), `0x004d2560` (`Order::ByValue`),
    /// `0x004d2740` (`Order::ByPointer`): `[first, last)` partitioned
    /// around the median of three: below it, the run equal to it
    /// (`[out[0], out[1])`), above it. `out`.
    pub fn partition(h: &mut impl Host, order: Order, out: u32, first: u32, last: u32) -> u32 {
        let size = order.size();
        let shift = size.trailing_zeros();
        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)] // the original's sar
        let half = ((last.wrapping_sub(first) as i32 >> shift) / 2) as u32;
        let mid = first.wrapping_add(half.wrapping_mul(size));
        let tail = last.wrapping_sub(size);
        match order {
            Order::Below => h.call_sym(MEDIANS[0], Conv::Cdecl, &[first, mid, tail]),
            Order::ByValue(pred) => h.call_sym(MEDIANS[1], Conv::Cdecl, &[first, mid, tail, pred]),
            Order::ByPointer(pred) => {
                h.call_sym(MEDIANS[2], Conv::Cdecl, &[first, mid, tail, pred])
            }
        };
        let mut pfirst = mid;
        let mut plast = pfirst + size;
        while first < pfirst
            && !less(h, order, pfirst - size, pfirst)
            && !less(h, order, pfirst, pfirst - size)
        {
            pfirst -= size;
        }
        while plast < last && !less(h, order, plast, pfirst) && !less(h, order, pfirst, plast) {
            plast += size;
        }
        let mut gfirst = plast;
        let mut glast = pfirst;
        loop {
            while gfirst < last {
                if less(h, order, pfirst, gfirst) {
                    // above: stays
                } else if less(h, order, gfirst, pfirst) {
                    break;
                } else {
                    swap(h, order, plast, gfirst);
                    plast += size;
                }
                gfirst += size;
            }
            while first < glast {
                if less(h, order, glast - size, pfirst) {
                    // below: stays
                } else if less(h, order, pfirst, glast - size) {
                    break;
                } else {
                    pfirst -= size;
                    swap(h, order, pfirst, glast - size);
                }
                glast -= size;
            }
            if glast == first && gfirst == last {
                h.write(out, pfirst);
                h.write(out + 4, plast);
                return out;
            }
            if glast == first {
                if plast != gfirst {
                    swap(h, order, pfirst, plast);
                }
                plast += size;
                swap(h, order, pfirst, gfirst);
                pfirst += size;
                gfirst += size;
            } else if gfirst == last {
                glast -= size;
                pfirst -= size;
                if glast != pfirst {
                    swap(h, order, glast, pfirst);
                }
                plast -= size;
                swap(h, order, pfirst, plast);
            } else {
                glast -= size;
                swap(h, order, gfirst, glast);
                gfirst += size;
            }
        }
    }
}
