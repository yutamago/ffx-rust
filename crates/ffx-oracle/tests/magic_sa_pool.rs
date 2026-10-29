//! The node pool (`ffx_magic::sa_pool`, `magic_0009`'s copy) against the
//! original: taking nodes off the free ring and giving them back, with
//! chained nodes, the tracking globals and the full pool. Needs
//! `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 9;
const TAKE: u32 = 0x39a0;
const GIVE: u32 = 0x3a20;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

fn node(i: u32) -> u32 {
    SCRATCH + 0x1000 + 0x40 * i
}

/// A free ring of `n` nodes (1..=n) with the head on node 1 and the
/// count `value` (`u32::MAX`: `n`).
fn ring(t: &mut Twin, n: u32, (free, count): (u32, u32), value: u32) {
    for i in 1..=n {
        t.write(node(i), &[0x5a; 0x1c]);
        let next = if i == n { 1 } else { i + 1 };
        let prev = if i == 1 { n } else { i - 1 };
        t.set_u32(node(i), node(prev));
        t.set_u32(node(i) + 4, node(next));
    }
    t.set_u32(free, if n == 0 { 0 } else { node(1) });
    let v = if value == u32::MAX { n } else { value };
    t.set_u16(count, u16::try_from(v).unwrap());
}

#[test]
fn take_off_the_free_ring() {
    let Some(mut t) = twin() else { return };
    let f = t.at(TAKE);
    let count = t.img.word(f + 0x04);
    let free = t.img.word(f + 0x17);
    for (n, c) in [
        (0u32, 0u32),
        (1, 1),
        (2, 2),
        (3, 3),
        (3, 0x1234),
        (1, 0xffff),
    ] {
        ring(&mut t, n, (free, count), c);
        t.check(
            f,
            Call {
                eax: 0x7777_7777,
                ecx: 0x00c0_ffee,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| ffx_magic::Ret::eax(ffx_magic::sa_pool::take(g, img, fr.at, fr.ecx)),
        );
    }
}

/// `give_back` of node 10 in a sibling list 9 <-> 10 <-> 11 (either side
/// left out), with `chain` nodes hooked to it through `+0x8`/`+0xc`.
#[test]
fn give_back_to_the_ring() {
    let Some(mut t) = twin() else { return };
    let f = t.at(GIVE);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let (cur, head, tail, free, count) = (
        w(&t, 0x48),
        w(&t, 0x55),
        w(&t, 0x64),
        w(&t, 0x74),
        w(&t, 0x8e),
    );
    let me = node(10);
    for case in 0u32..16 {
        let (prev, next) = match case % 4 {
            0 => (node(9), node(11)),
            1 => (0, node(11)),
            2 => (node(9), 0),
            _ => (0, 0),
        };
        ring(&mut t, (case / 4) % 3, (free, count), u32::MAX);
        for i in 9..=24 {
            t.write(node(i), &[0x3c; 0x1c]);
        }
        t.set_u32(me, prev);
        t.set_u32(me + 4, next);
        // the chain: me+8 -> 12 -> 13 -> ... (+0xc), the last with +0xc 0;
        // each chained node's own +8 is itself
        let chain = case % 3;
        t.set_u32(me + 8, node(12));
        for k in 0..=chain {
            let c = node(12 + k);
            t.set_u32(c + 8, c);
            t.set_u32(c + 0xc, if k == chain { 0 } else { node(13 + k) });
        }
        t.set_u32(cur, if case & 1 == 0 { me } else { node(20) });
        t.set_u32(head, if case & 2 == 0 { me } else { node(21) });
        t.set_u32(tail, if case & 4 == 0 { me } else { node(22) });
        // a full pool after this one (0xff free before), or not
        if case == 5 || case == 6 {
            t.set_u16(count, 0xff);
        } else {
            t.set_u16(count, 0x1000 + u16::try_from(case).unwrap());
        }
        t.check(
            f,
            Call {
                eax: 0x9999_0000,
                ecx: me,
                edx: 0x55,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| ffx_magic::Ret::eax(ffx_magic::sa_pool::give_back(g, img, fr.at, fr.ecx)),
        );
    }
}
