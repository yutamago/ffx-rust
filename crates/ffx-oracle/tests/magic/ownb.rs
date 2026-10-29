//! Shared helpers of list ownB's overlay tests (`magic_NNNN.rs`).
#![allow(dead_code)]

use crate::magic::{SCRATCH, STACK_TOP, StubMem, Twin};

/// The call log the [`rec`] stubs append to (a cursor word, then the
/// entries), past the rows other tests use.
pub const LOG: u32 = SCRATCH + 0x3_0000;
const LOG_LEN: u32 = 0xfff0;

/// Answers `slot` with `ret` and appends the slot and the first `n`
/// argument words of each call to [`LOG`] (both sides alike: a port that
/// passes fewer than `n` words fails, one that passes others differs in
/// memory).
pub fn rec(
    t: &mut Twin,
    slot: u32,
    n: usize,
    ret: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static,
) {
    t.stub(slot, move |m, a| {
        let mut at = m.r32(LOG);
        let mut put = |m: &mut dyn StubMem, v: u32| {
            m.w32(LOG + 4 + at % LOG_LEN, v);
            at += 4;
        };
        put(m, 0x5100_0000 | slot);
        for w in &a[..n] {
            put(m, *w);
        }
        m.w32(LOG, at);
        ret(m, a)
    });
}

/// Per-slot call counters of [`rec_seq`].
const COUNTERS: u32 = SCRATCH + 0x2_f000;

/// [`rec`] with the word count of each call in turn (`ns[k]` for the
/// slot's `k`-th call, the last count for later ones): for slots called
/// with different numbers of words on the stack.
pub fn rec_seq(
    t: &mut Twin,
    slot: u32,
    ns: &'static [usize],
    ret: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static,
) {
    t.stub(slot, move |m, a| {
        let ctr = COUNTERS + 4 * (slot % 0x400);
        let k = m.r32(ctr);
        m.w32(ctr, k + 1);
        let n = ns[(k as usize).min(ns.len() - 1)];
        let mut at = m.r32(LOG);
        let mut put = |m: &mut dyn StubMem, v: u32| {
            m.w32(LOG + 4 + at % LOG_LEN, v);
            at += 4;
        };
        put(m, 0x5200_0000 | slot);
        for w in &a[..n] {
            put(m, *w);
        }
        m.w32(LOG, at);
        ret(m, a)
    });
}

/// Restarts every [`rec_seq`] sequence (before each checked call).
pub fn reset_counters(t: &mut Twin) {
    t.write(COUNTERS, &[0; 0x1000]);
}

/// [`rec`] for several slots answering a constant each (`0xNNNN_0000 |
/// slot`, distinct per slot).
pub fn rec_all(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, _| 0x0abc_0000 | s);
    }
}

/// Fills `n` words at `at` on both sides with distinct non-zero values.
pub fn fill(t: &mut Twin, at: u32, n: u32, seed: u32) {
    for i in 0..n {
        t.set_u32(
            at + 4 * i,
            seed.wrapping_mul(0x9e37_79b9).wrapping_add(i) | 1,
        );
    }
}

/// Writes `v` (floats) at `at` on both sides.
pub fn floats(t: &mut Twin, at: u32, v: &[f32]) {
    for (i, x) in (0u32..).zip(v) {
        t.set_u32(at + 4 * i, x.to_bits());
    }
}

/// The original's entry stack pointer in the harness.
pub const ENTRY_ESP: u32 = STACK_TOP - 0x100;

/// Fills the 4 KiB below the entry stack pointer (the callee's frame)
/// with junk on both sides, so a local the port fails to write differs
/// from the original's; `zero` are the frame words (offsets below the
/// entry `esp`) the original reads without writing (the junk reads,
/// 0 on both sides).
pub fn dirty_stack(t: &mut Twin, seed: u32, zero: &[u32]) {
    let base = ENTRY_ESP - 0x1000;
    let words: Vec<u8> = (0..0x400u32)
        .flat_map(|i| (seed ^ i.wrapping_mul(0x0100_0193) ^ 0x5a5a_0000).to_le_bytes())
        .collect();
    t.write(base, &words);
    for &z in zero {
        t.set_u32(ENTRY_ESP - z, 0);
    }
}

/// Rotations whose angle `A - rot` (operands `a`, `hi`, `lo` of `f`)
/// wraps down, stays, wraps up, lands on either limit, or is NaN.
#[allow(clippy::many_single_char_names)]
pub fn rotations(t: &Twin, f: u32, a: u32, hi: u32, lo: u32) -> Vec<f32> {
    let k = |op: u32| f32::from_bits(t.u32(t.img.word(f + op)));
    let (a, hi, lo) = (k(a), k(hi), k(lo));
    let mut v = vec![0.0f32, a, a + 4.0, -2.0, f32::NAN];
    for limit in [hi, lo] {
        let guess = (a - limit).to_bits();
        if let Some(r) = (0..256u32)
            .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
            .map(f32::from_bits)
            .find(|&r| (a - r).to_bits() == limit.to_bits())
        {
            v.push(r);
        }
    }
    v
}
