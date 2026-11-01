//! Round 6 shape families: a record pool's reset with its first node
//! taken (families `F0180_10002140` and `F0250_10001e00` of
//! `~/ffx-work/m5/magic/r4/families.tsv`; 8 copies in `magic_0180`,
//! `0205`, `0243`, `0350`, `0385`, `0417` (221 bytes) and `0250`, `0284`
//! (216 bytes, without the second used word). The copies differ in the
//! record count and stride, the cleared field and the callback; every
//! operand is read from the copy's code (`w(off)` = the word at `+off`,
//! offsets of the 221-byte copies).

use crate::o4_parts::disp8;
use crate::{Game, Image, Sig};

/// Resets the record pool at `pool` and takes its first node (internal:
/// `ecx` = `pool`; `magic_0180` 0x10002140, 221 bytes, 6 copies).
///
/// `G_POOL` (`+0x02`) = `pool`; the word at `pool + OFF + STRIDE * i` is
/// cleared for `i` in `0..N` (`OFF` the `add ecx, imm8` at `+0x08`,
/// `STRIDE` the `lea` at `+0x18`, `N` the `mov eax` at `+0x0a`); the
/// pool's used words `+0x25`, `+0x34` get 0 and its size `+0x2b` the
/// immediate at `+0x2f` (the 216-byte copies have no `+0x34` store, and
/// every later operand 5 bytes earlier).
///
/// Then a node `a` is taken from the free ring (count `G_FREE` u16 at
/// `+0x20`, ring head `G_HEAD` at `+0x3e`): `a` is unlinked
/// (`a.prev.next = a.next`, `a.next.prev = a.prev`, each re-read), the
/// head moves to `a.next` (0 when it was the last), the count drops by
/// one; `a` is cleared (`+0`, `+4`, `+0xc`, `+0x14`; byte `+0x18`), its
/// `+8` points at itself and `+0x10` holds the callback (`+0x95`). It is
/// appended to the busy list (`G_FIRST` `+0xa6`, `G_LAST` `+0xb7`): as
/// the first node, or after the last. Returns `a`.
///
/// With an empty ring (`G_FREE` 0) the original skips the take and writes
/// `[0 + 4]`: it faults. The port writes there too (the same fault in the
/// game's memory).
pub fn pool_reset_take<G: Game>(g: &mut G, img: &Image, f: u32, pool: u32) -> u32 {
    // the second used word's `mov [G], eax` at +0x33, or the test of the
    // free count there (the shorter copies)
    let short = img.word(f + 0x33) as u8 != 0xa3;
    let w = |o: u32| {
        let o = if short && o > 0x33 { o - 5 } else { o };
        img.word(f.wrapping_add(o))
    };
    g.write::<u32>(w(0x02), pool);
    let n = w(0x0a);
    let (off, stride) = (disp8(img, f + 0x08), disp8(img, f + 0x18));
    let mut at = pool.wrapping_add(off);
    let mut i = n;
    loop {
        g.write::<u32>(at, 0);
        at = at.wrapping_add(stride);
        i = i.wrapping_sub(1);
        if i == 0 {
            break;
        }
    }
    let mut free = g.read::<u16>(w(0x20));
    // eax is 0 after the loop
    g.write::<u32>(w(0x25), 0);
    g.write::<u32>(w(0x2b), w(0x2f));
    if !short {
        g.write::<u32>(w(0x34), 0);
    }
    let mut a = 0u32;
    if free != 0 {
        a = g.read::<u32>(w(0x3e));
        let prev = g.read::<u32>(a);
        let next = g.read::<u32>(a.wrapping_add(4));
        g.write::<u32>(prev.wrapping_add(4), next);
        let next = g.read::<u32>(a.wrapping_add(4));
        let prev = g.read::<u32>(a);
        g.write::<u32>(next, prev);
        let head = if free > 1 {
            g.read::<u32>(a.wrapping_add(4))
        } else {
            0
        };
        g.write::<u32>(w(0x3e), head);
        free = free.wrapping_sub(1);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(a.wrapping_add(o), 0);
        }
        g.write::<u32>(a.wrapping_add(8), a);
        g.write::<u32>(a.wrapping_add(0x10), w(0x95));
        g.write::<u8>(a.wrapping_add(0x18), 0);
        g.write::<u16>(w(0xa0), free);
    }
    let empty = g.read::<u32>(w(0xa6)) == 0;
    g.write::<u32>(a.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(a, 0);
        g.write::<u32>(w(0xd0), a);
        g.write::<u32>(a.wrapping_add(8), a);
        g.write::<u32>(w(0xd8), a);
    } else {
        let last = g.read::<u32>(w(0xb7));
        g.write::<u32>(a, last);
        g.write::<u32>(last.wrapping_add(4), a);
        g.write::<u32>(a.wrapping_add(8), a);
        g.write::<u32>(w(0xc4), a);
    }
    a
}

include!("o6_fam_pool_table.rs.in");
