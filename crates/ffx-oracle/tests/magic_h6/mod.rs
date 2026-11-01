//! Helpers for the per-DLL parity tests of the r6 own ports and the
//! mutation fixes of `magic_0450` and up (`magic_NNNN.rs`; docs/formats/
//! magic.md, "Porting the overlays").
#![allow(dead_code)]

use crate::magic::{SCRATCH, StubMem, Twin};
use crate::magic_owna::{rec, row};

/// Where [`rec_cells`] keeps the cells it sees: per slot, 16 calls of 4
/// words.
#[must_use]
pub fn cells_row(slot: u32) -> u32 {
    SCRATCH + 0x3_8000 + 0x100 * (slot % 0x40)
}

/// [`rec`] that also keeps the words in `cells` at each call (the game
/// flag a slot runs under, the part pointer's cell), so a port that
/// skips a store before the call shows.
pub fn rec_cells(
    t: &mut Twin,
    slot: u32,
    n: usize,
    cells: &[u32],
    ret: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static,
) {
    let cells = cells.to_vec();
    rec(t, slot, n, move |m, a| {
        let k = m.r32(row(slot)).wrapping_sub(1);
        for (i, c) in (0u32..).zip(&cells) {
            let v = m.r32(*c);
            m.w32(cells_row(slot) + 0x10 * (k % 0x10) + 4 * (i % 4), v);
        }
        ret(m, a)
    });
}

/// The DLL globals at the operands `ops` of the function at `f` set to
/// distinct values (`0x00a5_0000 + op`): words a port reads and passes
/// on, so reading a neighbouring operand shows.
pub fn distinct(t: &mut Twin, f: u32, ops: &[u32]) {
    for &op in ops {
        let g = t.img.word(f + op);
        t.set_u32(g, 0x00a5_0000 + op);
    }
}

/// The cell a DLL pointer global (operand `op` of `f`) points at.
#[must_use]
pub fn cell(t: &Twin, f: u32, op: u32) -> u32 {
    t.u32(t.img.word(f + op))
}

/// A timeline ([`timeline_run`](ffx_magic::sb_misc::timeline_run)'s
/// layout) with one live event that hasn't started (its start far
/// ahead, no callbacks; the event record at `ev`) and the time `time`:
/// a run calls slot 253 and moves the time on, a stop ends the event
/// (the live word and the count go to 0), so a port that runs or stops
/// the wrong timeline, or skips one, shows. Set it again before each
/// case (a stop leaves it idle).
pub fn live_timeline(t: &mut Twin, tl: u32, ev: u32, time: u32) {
    t.set_u32(tl, time);
    t.set_u32(tl + 4, 1);
    t.set_u32(tl + 8, 1);
    t.set_u32(tl + 0xc, 1);
    t.set_u32(tl + 0x10c, ev);
    t.write(ev, &[0; 0x18]);
    t.set_u32(ev, 0x7fff_0000);
    t.set_u32(ev + 4, 1);
}

/// [`live_timeline`] for a DLL's own timeline
/// ([`timeline`](ffx_magic::sa_misc::timeline) at `run`; its globals: the
/// live count `+0x06`, the events pointer `+0x17`, the live words
/// `+0x1f`, the event count `+0x25`, the time `+0x41`): one live event
/// that hasn't started, at `ev`.
pub fn live_timeline_global(t: &mut Twin, run: u32, ev: u32, time: u32) {
    let w = |t: &Twin, op: u32| t.img.word(run + op);
    let (live, events, flags, count, now) =
        (w(t, 0x06), w(t, 0x17), w(t, 0x1f), w(t, 0x25), w(t, 0x41));
    t.set_u32(live, 1);
    t.set_u32(events, ev);
    t.set_u32(flags, 1);
    t.set_u32(count, 1);
    t.set_u32(now, time);
    t.write(ev, &[0; 0x18]);
    t.set_u32(ev, 0x7fff_0000);
    t.set_u32(ev + 4, 1);
}

/// The 4 KiB of frames below the entry's stack pointer filled with a
/// varied pattern on both sides before a case: the stores of the case
/// before, or zeros, would hide a store to a local the port drops from a
/// stub that looks at the local (as in `magic_s7_misc.rs`).
pub fn scrub(t: &mut Twin) {
    let entry = crate::magic::STACK_TOP - 0x100;
    let junk: Vec<u8> = (0..0x1000u32)
        .map(|i| (i.wrapping_mul(0x2b) ^ 0x5b).to_le_bytes()[0])
        .collect();
    t.write(entry - 0x1000, &junk);
}

/// An `x` with `x + step == target` in `f32` (a case exactly at a bound
/// the code compares after the step), searched from `target - step`.
#[must_use]
pub fn hits(target: f32, step: f32) -> f32 {
    let mut x = target - step;
    for _ in 0..64 {
        let s = x + step;
        if s.to_bits() == target.to_bits() {
            return x;
        }
        x = if s < target {
            x.next_up()
        } else {
            x.next_down()
        };
    }
    panic!("no x with x + {step} == {target}");
}

/// [`magic_hi::models`](crate::magic_hi::models), and past each model's
/// last part one more entry, a part of kind `kind` (byte 0x48, two
/// entries) the walk must not reach: a part loop that runs once more, or
/// enters a model without parts, shows. Needs `mod magic_hi;`.
pub fn models_beyond(t: &mut Twin, g_at: u32, models: &[crate::magic_hi::Model], kind: u32) {
    crate::magic_hi::models(t, g_at, models);
    for (i, m) in (0u32..).zip(models) {
        let Some(parts) = m else { continue };
        let model = SCRATCH + 0x2000 + 0x400 * i;
        let j = u32::try_from(parts.len()).unwrap_or(0);
        let off = 0x80 + 0x100 * j;
        if off >= 0x400 {
            continue;
        }
        t.set_u16(model + 0x10 + 8 * j, u16::try_from(off).unwrap_or(0));
        let part = model + off;
        t.set_u32(part + 0x10, kind);
        t.set_u16(part + 2, 2);
        t.set_u32(part + 4, 0x20);
        t.set_u8(part, 0x48);
        for k in 0..4 {
            t.set_u8(part + 0x24 + 8 * k, 0x48);
        }
    }
}

/// `n` bytes counting up from `seed` by 7 (each word differs from its
/// neighbours, so a field read one word on shows).
#[must_use]
pub fn varied(n: u32, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| seed.wrapping_add((i.wrapping_mul(7)).to_le_bytes()[0]))
        .collect()
}
