//! M5 finish, WP5 part 2 (`ffx_prender::wp5_sort`): the light sort's
//! `std::sort` family in its three instantiations (4-byte keys, 4-byte
//! values by a predicate, 8-byte records by a predicate). Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`wp5b_*`).
//!
//! Threads: the render thread and the scene workers (the light sort).
//! Every shim's held values live in its own `StackFrame`.

use ffx_prender::wp5_sort::bridge as b;
use ffx_prender::wp5_sort::{FRAME_WORDS, Order};

use super::{StackFrame, mem, ported};

ported! {
    /// `std_adjustHeap_keys_004d0fa0`: the hole sunk, the value sifted up; the value.
    std_adjustHeap_keys_004d0fa0 @ 0x004d_0fa0 => extern "C" fn ws_004d0fa0(first: u32, hole: u32, bottom: u32, value: u32) -> u32 {
        b::adjust_heap(&mut mem(), Order::Below, first, hole, bottom, value)
    }
    /// `std_adjustHeap_values_004d1030`: by a predicate; the value.
    std_adjustHeap_values_004d1030 @ 0x004d_1030 => extern "C" fn ws_004d1030(first: u32, hole: u32, bottom: u32, value: u32, pred: u32) -> u32 {
        b::adjust_heap(&mut mem(), Order::ByValue(pred), first, hole, bottom, value)
    }
    /// `std_adjustHeap_records_004d10d0`: 8-byte records by a predicate; the value's second word.
    std_adjustHeap_records_004d10d0 @ 0x004d_10d0 => extern "C" fn ws_004d10d0(first: u32, hole: u32, bottom: u32, value: u32, pred: u32) -> u32 {
        b::adjust_heap(&mut mem(), Order::ByPointer(pred), first, hole, bottom, value)
    }
    /// `std_insertionSort_keys_004d11c0`: 4-byte keys sorted by insertion.
    std_insertionSort_keys_004d11c0 @ 0x004d_11c0 => extern "C" fn ws_004d11c0(first: u32, last: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::insertion_sort(&mut mem(), Order::Below, first, last, frame.at());
    }
    /// `std_insertionSort_values_004d1240`: by a predicate.
    std_insertionSort_values_004d1240 @ 0x004d_1240 => extern "C" fn ws_004d1240(first: u32, last: u32, pred: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::insertion_sort(&mut mem(), Order::ByValue(pred), first, last, frame.at());
    }
    /// `std_insertionSort_records_004d12f0`: 8-byte records by a predicate.
    std_insertionSort_records_004d12f0 @ 0x004d_12f0 => extern "C" fn ws_004d12f0(first: u32, last: u32, pred: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::insertion_sort(&mut mem(), Order::ByPointer(pred), first, last, frame.at());
    }
    /// `std_makeHeap_keys_004d1430`: 4-byte keys made a heap.
    std_makeHeap_keys_004d1430 @ 0x004d_1430 => extern "C" fn ws_004d1430(first: u32, last: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::make_heap(&mut mem(), Order::Below, first, last, frame.at());
    }
    /// `std_makeHeap_values_004d14e0`: by a predicate.
    std_makeHeap_values_004d14e0 @ 0x004d_14e0 => extern "C" fn ws_004d14e0(first: u32, last: u32, pred: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::make_heap(&mut mem(), Order::ByValue(pred), first, last, frame.at());
    }
    /// `std_makeHeap_records_004d1590`: 8-byte records by a predicate.
    std_makeHeap_records_004d1590 @ 0x004d_1590 => extern "C" fn ws_004d1590(first: u32, last: u32, pred: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::make_heap(&mut mem(), Order::ByPointer(pred), first, last, frame.at());
    }
    /// `_Med3_unchecked<MemBlock*,...>` (8-byte records by a predicate): three ordered.
    std_med3_records_004d1700 @ 0x004d_1700 => extern "C" fn ws_004d1700(a: u32, m: u32, c: u32, pred: u32) {
        b::med3(&mut mem(), Order::ByPointer(pred), a, m, c);
    }
    /// `std_median_keys_004d1780`: the median of three or the ninther.
    std_median_keys_004d1780 @ 0x004d_1780 => extern "C" fn ws_004d1780(first: u32, mid: u32, last: u32) {
        b::median(&mut mem(), Order::Below, first, mid, last);
    }
    /// `std_median_values_004d18b0`: by a predicate.
    std_median_values_004d18b0 @ 0x004d_18b0 => extern "C" fn ws_004d18b0(first: u32, mid: u32, last: u32, pred: u32) {
        b::median(&mut mem(), Order::ByValue(pred), first, mid, last);
    }
    /// `std_median_records_004d1a90`: 8-byte records by a predicate.
    std_median_records_004d1a90 @ 0x004d_1a90 => extern "C" fn ws_004d1a90(first: u32, mid: u32, last: u32, pred: u32) {
        b::median(&mut mem(), Order::ByPointer(pred), first, mid, last);
    }
    /// `std_pushHeap_values_004d1f10`: the value sifted up; the value, or `first` when `top >= hole`.
    std_pushHeap_values_004d1f10 @ 0x004d_1f10 => extern "C" fn ws_004d1f10(first: u32, hole: u32, top: u32, value: u32, pred: u32) -> u32 {
        b::push_heap(&mut mem(), Order::ByValue(pred), first, hole, top, value)
    }
    /// `std_pushHeap_records_004d1f80`: 8-byte records; the value's second word.
    std_pushHeap_records_004d1f80 @ 0x004d_1f80 => extern "C" fn ws_004d1f80(first: u32, hole: u32, top: u32, value: u32, pred: u32) -> u32 {
        b::push_heap(&mut mem(), Order::ByPointer(pred), first, hole, top, value)
    }
    /// `std_sort_keys_004d1ff0`: 4-byte keys sorted (introsort).
    std_sort_keys_004d1ff0 @ 0x004d_1ff0 => extern "C" fn ws_004d1ff0(first: u32, last: u32, ideal: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::sort(&mut mem(), Order::Below, first, last, ideal, frame.at());
    }
    /// `std_sort_values_004d20d0`: by a predicate.
    std_sort_values_004d20d0 @ 0x004d_20d0 => extern "C" fn ws_004d20d0(first: u32, last: u32, ideal: u32, pred: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::sort(&mut mem(), Order::ByValue(pred), first, last, ideal, frame.at());
    }
    /// `std_sort_records_004d21c0`: 8-byte records by a predicate.
    std_sort_records_004d21c0 @ 0x004d_21c0 => extern "C" fn ws_004d21c0(first: u32, last: u32, ideal: u32, pred: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::sort(&mut mem(), Order::ByPointer(pred), first, last, ideal, frame.at());
    }
    /// `std_sortHeap_keys_004d22b0`: a heap of 4-byte keys sorted.
    std_sortHeap_keys_004d22b0 @ 0x004d_22b0 => extern "C" fn ws_004d22b0(first: u32, last: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::sort_heap(&mut mem(), Order::Below, first, last, frame.at());
    }
    /// `std_sortHeap_values_004d2300`: by a predicate.
    std_sortHeap_values_004d2300 @ 0x004d_2300 => extern "C" fn ws_004d2300(first: u32, last: u32, pred: u32) {
        let mut frame = StackFrame::<FRAME_WORDS>::new();
        b::sort_heap(&mut mem(), Order::ByValue(pred), first, last, frame.at());
    }
}
