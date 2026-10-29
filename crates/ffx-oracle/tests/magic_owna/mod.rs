//! Helpers for the per-DLL parity tests of the second round's own-code
//! ports (`magic_NNNN.rs`; docs/formats/magic.md, "Porting the
//! overlays").
#![allow(dead_code)]

use crate::magic::{SCRATCH, StubMem, Twin};

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
pub fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// Where [`rec`] keeps `slot`'s calls: a count, then the words.
#[must_use]
pub fn row(slot: u32) -> u32 {
    SCRATCH + 0x3_0000 + 0x40 * (slot % 0x400)
}

/// Records the first `n` words of each call of `slot` (each word xor the
/// call's count, so a later call doesn't hide an earlier one) and
/// answers `ret`.
pub fn rec(
    t: &mut Twin,
    slot: u32,
    n: usize,
    ret: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static,
) {
    let row = row(slot);
    t.stub(slot, move |m, a| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        for (i, w) in (0u32..).zip(&a[..n.min(a.len())]) {
            m.w32(row + 4 + 4 * i, *w ^ k);
        }
        ret(m, a)
    });
}

/// [`rec`] for several slots answering a value of their own
/// (`0x5a5a_0000 + slot` xor the first word).
pub fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (0x5a5a_0000 + s) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// Copies `n` (at most 16) words at `at` into a row of `slot`'s own,
/// for a stub that gets a pointer to a buffer.
pub fn dump(m: &mut dyn StubMem, slot: u32, at: u32, n: u32) {
    let row = SCRATCH + 0x2_0000 + 0x40 * (slot % 0x400);
    for i in 0..n.min(16) {
        let v = m.r32(at + 4 * i);
        m.w32(row + 4 * i, v);
    }
}

/// A slot that fills `n` words at its first argument (a matrix or a
/// vector the caller passes; each call other values) and records `words`
/// of its arguments.
pub fn filler(t: &mut Twin, slot: u32, words: usize, n: u32) {
    rec(t, slot, words, move |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(row(slot));
        for i in 0..n {
            #[allow(clippy::cast_precision_loss)]
            let v = 0.5 + i as f32 * 0.25 + (slot + k) as f32 * 0.125;
            m.w32(a[0] + 4 * i, f(v));
        }
        slot
    });
}

/// A slot that records `words` of its arguments and keeps `n` words at
/// argument `arg` (a buffer the caller passes), each call in a row of its
/// own (`base` + the call count).
pub fn keeper(t: &mut Twin, slot: u32, words: usize, arg: usize, n: u32, base: u32) {
    rec(t, slot, words, move |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(row(slot));
        if a[arg] >= 0x2000_0000 {
            dump(m, base + k % 0x40, a[arg], n);
        }
        slot
    });
}

/// `f32` bits.
#[must_use]
pub fn f(x: f32) -> u32 {
    x.to_bits()
}
