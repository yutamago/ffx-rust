//! `magic_0046.dll` against its Rust port: the state reset with `+0x11c`
//! = 0 (`ffx_magic::m0032`). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

use ffx_magic::{Ret, m0032};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 46;

#[test]
fn reset_state_writes_the_block() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let this = SCRATCH + 0x1000;
    t.write(this + 0xb0, &[0x5a; 0x90]);
    t.check(
        t.at(0x1a10),
        Call {
            eax: 0,
            ecx: this,
            edx: 0,
            ..Call::default()
        },
        &[0x1111, 0x1000_2130],
        Eax::Junk,
        |g, _, fr| {
            let arg = fr.arg(g, 1);
            m0032::reset_state(g, fr.ecx, arg, 0, 0);
            Ret::default()
        },
    );
}

/// The object start (0x10001b30, internal: `edx` = the object, `xmm3` the
/// scale): appended to an empty list, then to a list with a tail, and
/// with the pool empty.
#[test]
fn start_object() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let at = t.at(0x1b30);
    t.set_u32(t.img.word(at + 0x01), SCRATCH + 0x200);
    t.set_u32(t.img.word(at + 0x5b), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    t.set_u32(SCRATCH + 0x400, 0x1111);
    t.stub(941, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        SCRATCH + 0x400
    });
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    let pa = t.img.rel32_target(at + 0x95);
    let free = t.img.word(pa + 0x04);
    let tail = t.img.word(at + 0xa0);
    let obj = SCRATCH + 0x1000;
    for (n, k, tl) in [
        (3u16, 0.5f32, 0u32),
        (2, -2.0, SCRATCH + 0x600),
        (0, 1.0, 0),
    ] {
        t.write(obj, &[0x5a; 0x130]);
        t.set_u16(free, n);
        t.set_u32(tail, tl);
        let c = Call {
            edx: obj,
            ..Call::default()
        }
        .with_xmm(&[
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(k),
        ]);
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(ffx_magic::m0046::start_object(
                g,
                img,
                fr.at,
                fr.edx,
                fr.xmm_f32(3),
            ))
        });
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The start-up (0x10001370): into an empty and a filled task list.
#[test]
fn start_up() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let at = t.at(0x1370);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let so = t.img.rel32_target(at + 0x49);
    t.set_u32(t.img.word(so + 0x01), SCRATCH + 0x200);
    t.set_u32(t.img.word(so + 0x5b), SCRATCH + 0x300);
    t.stub(941, |_, _| SCRATCH + 0x400);
    for (slot, k) in [(719u32, 0u32), (262, 1), (268, 2), (714, 3)] {
        t.stub(slot, move |m, a| {
            for (i, v) in (0u32..).zip(&a[..3]) {
                m.w32(SCRATCH + 0x800 + 0x40 * k + 4 * i, *v);
            }
            0x0100_0000 * (k + 1)
        });
    }
    let b = SCRATCH + 0x500;
    t.set_u32(w(&t, 0x4e), b);
    t.write(b, &[2, 0x40]);
    // the words the start-up clears hold junk
    for op in [0x95, 0x9f, 0xa9, 0xb3, 0xbd, 0xdb, 0xe5, 0xef] {
        t.set_u32(w(&t, op), 0x5a5a_5a5a);
    }
    t.set_u32(w(&t, 0x62) + 8, 0x00ee_0002);
    // the free list's head for pop_node: a ring of two
    let pop = t.img.rel32_target(at + 0xfe);
    let (count, head) = (t.img.word(pop + 0x04), t.img.word(pop + 0x13));
    let (n1, n2) = (SCRATCH + 0x1800, SCRATCH + 0x1840);
    let last = SCRATCH + 0x1900;
    for first in [0u32, last] {
        t.write(n1, &[0x5a; 0x80]);
        t.set_u32(n1, n2);
        t.set_u32(n1 + 4, n2);
        t.set_u32(n2, n1);
        t.set_u32(n2 + 4, n1);
        t.set_u32(head, n1);
        t.set_u16(count, 2);
        t.write(last, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x104), first);
        t.set_u32(w(&t, 0x115), first);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
