//! `magic_0153.dll` against its Rust port: `DllMain` against every bound
//! port, the node constructors and destructor and the data wait. Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

use std::cell::Cell;
use std::rc::Rc;

use ffx_magic::{Ret, l3_common, m0153};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 153;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

const LIST: u32 = SCRATCH + 0x1000;
const NODE: u32 = SCRATCH + 0x2000;
const OTHER: u32 = SCRATCH + 0x3000;
const REC: u32 = SCRATCH + 0x4000;

/// Old bytes everywhere a constructor writes.
fn dirty(t: &mut Twin) {
    for a in [LIST, NODE, OTHER, REC] {
        t.write(a, &[0xa5; 0x100]);
    }
}

fn this_call(c: u32) -> Call {
    Call {
        eax: 0x1111,
        ecx: c,
        edx: 0x2222,
    }
}

/// The record constructors (0x1900, 0x47a0): no list, an empty list (its
/// last word is itself), a list with a last node.
#[test]
fn record_constructors() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    for rva in [0x1900u32, 0x47a0] {
        for (list, last) in [(0, 0), (LIST, LIST), (LIST, OTHER)] {
            dirty(&mut t);
            t.set_u32(REC, NODE);
            t.set_u32(REC + 4, list);
            t.set_u32(REC + 8, 0x0808);
            t.set_u32(REC + 0xc, 0x0c0c);
            t.set_u32(REC + 0x10, 0x1010);
            t.set_u32(REC + 0x14, 0x1414);
            t.set_u32(LIST + 0x10, last);
            t.set_u16(LIST + 0x18, 0xfffe);
            let f = t.at(rva);
            t.check(f, this_call(REC), &[], Eax::Checked, move |g, img, fr| {
                Ret::eax(if rva == 0x1900 {
                    m0153::particle_node(g, img, fr.at, fr.ecx)
                } else {
                    l3_common::record_node(g, img, fr.at, fr.ecx)
                })
            });
        }
    }
}

/// The static-list constructors (0x79d0, 0x7a30, 0x7b80): the list empty
/// (its last word holds the sentinel) or with a last node.
#[test]
fn static_list_constructors() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    // (rva, operand of the last word, of the sentinel, of the count)
    for (rva, last_op, sentinel_op, count_op) in [
        (0x79d0u32, 0x1b, 0x28, 0x3c),
        (0x7a30, 0x27, 0x34, 0x48),
        (0x7b80, 0x24, 0x31, 0x45),
    ] {
        let f = t.at(rva);
        let last_at = t.img.word(f + last_op);
        let sentinel = t.img.word(f + sentinel_op);
        let count = t.img.word(f + count_op);
        if rva == 0x7a30 {
            t.set_u32(t.img.word(f + 0x02), 0x00ed_0000);
        }
        for last in [sentinel, OTHER] {
            dirty(&mut t);
            t.set_u32(last_at, last);
            t.set_u16(count, 0xffff);
            t.check(
                f,
                this_call(NODE),
                &[0x00a1_0000],
                Eax::Checked,
                move |g, img, fr| {
                    Ret::eax(match rva {
                        0x79d0 => l3_common::scale_node(g, img, fr.at, fr.ecx),
                        0x7a30 => m0153::timed_node(g, img, fr.at, fr.ecx),
                        _ => m0153::owner_node(g, img, fr.at, fr.ecx, fr.arg(g, 0)),
                    })
                },
            );
        }
    }
}

#[test]
fn node_drop() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    t.stub(791, |m, a| {
        for (i, w) in (0u32..).zip(&a[..3]) {
            m.w32(SCRATCH + 0x800 + 4 * i, *w);
        }
        0x7910
    });
    let f = t.at(0x7aa0);
    for (list, next, prev) in [
        (0, OTHER, OTHER),
        (LIST, LIST, LIST),
        (LIST, LIST, OTHER),
        (LIST, OTHER, LIST),
        (LIST, OTHER, OTHER + 0x80),
    ] {
        dirty(&mut t);
        t.set_u32(NODE + 0x14, list);
        t.set_u32(NODE + 4, next);
        t.set_u32(NODE + 8, prev);
        t.set_u16(LIST + 0x18, 0);
        for (i, v) in (0u32..).zip([1.5f32, -2.0, 0.25]) {
            t.set_u32(NODE + 0x5c + 4 * i, v.to_bits());
        }
        t.check(f, this_call(NODE), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0153::node_drop(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn data_wait() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x8920);
    let state = t.img.word(f + 0x01);
    t.set_u32(t.img.word(f + 0x5e), 0x0001_d000);
    // each check answers the word in its scratch cell; every call is
    // counted in a row so the order shows
    for (k, slot) in (0u32..).zip([225u32, 313, 549, 226, 673]) {
        t.stub(slot, move |m, a| {
            let n = m.r32(SCRATCH + 0x900);
            m.w32(SCRATCH + 0x904 + 8 * (n % 16), slot);
            // slot 673 takes no arguments
            let a0 = if slot == 673 { 0 } else { a[0] };
            m.w32(SCRATCH + 0x908 + 8 * (n % 16), a0);
            m.w32(SCRATCH + 0x900, n + 1);
            if slot == 226 {
                m.w32(SCRATCH + 0x9a0, a[1]);
            }
            m.r32(SCRATCH + 0x980 + 4 * k)
        });
    }
    for s in [0u32, 1, 2, 3, 4, 0xffff_ffff] {
        for answers in [[0u32, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1], [5, 5, 5]] {
            t.set_u32(state, s);
            t.set_u32(SCRATCH + 0x900, 0);
            for (i, a) in (0u32..).zip(answers) {
                t.set_u32(SCRATCH + 0x980 + 4 * i, a);
            }
            t.check_entry(f, this_call(0x33), &[], Eax::Checked);
        }
    }
}

#[test]
fn release_effects() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x2080);
    let flag = t.u32(t.img.word(f + 0x6b));
    // graphicVFXDestroy records the handle words and the flag it sees;
    // the third call shrinks the second array's count
    t.stub(779, move |m, a| {
        let n = m.r32(SCRATCH + 0xa00);
        m.w32(SCRATCH + 0xa04 + 12 * (n % 32), a[0]);
        m.w32(SCRATCH + 0xa08 + 12 * (n % 32), a[1]);
        m.w32(SCRATCH + 0xa0c + 12 * (n % 32), m.r32(flag));
        m.w32(SCRATCH + 0xa00, n + 1);
        if n == 2 {
            let c = m.r32(NODE + 0x2c);
            m.w32(NODE + 0x2c, (c & 0xffff) | 0x0001_0000);
        }
        0x7790
    });
    // (list, count 0, count 1, array 0, array 1)
    for (list, n0, n1, a0, a1) in [
        (0, 0u16, 0u16, OTHER, OTHER),
        (LIST, 2, 3, OTHER, 0x8000_0000),
        (0, 0, 4, OTHER, 0x7fff_fff0),
        (LIST, 1, 0, 0xffff_fff0, OTHER),
    ] {
        dirty(&mut t);
        t.set_u32(SCRATCH + 0xa00, 0);
        t.set_u32(flag, 0x0bad);
        t.set_u32(REC, NODE);
        t.set_u32(NODE + 0x14, list);
        t.set_u32(NODE + 4, LIST);
        t.set_u32(NODE + 8, OTHER);
        t.set_u32(NODE + 0x24, a0);
        t.set_u32(NODE + 0x28, a1);
        t.set_u16(NODE + 0x2c, n0);
        t.set_u16(NODE + 0x2e, n1);
        t.check(f, this_call(REC), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0153::release_effects(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn ambient_node() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    t.stub(792, |m, a| {
        for (i, p) in (0u32..).zip(&a[..3]) {
            m.w32(*p, 0x3f00_0000 + i);
            m.w32(SCRATCH + 0xb00 + 4 * i, *p);
        }
        0x7920
    });
    let f = t.at(0x2150);
    for (list, last) in [(0, 0), (LIST, LIST), (LIST, OTHER)] {
        dirty(&mut t);
        t.set_u32(REC, NODE);
        t.set_u32(REC + 4, list);
        t.set_u32(LIST + 0x10, last);
        t.set_u16(LIST + 0x18, 7);
        t.check(f, this_call(REC), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0153::ambient_node(g, img, fr.at, fr.ecx))
        });
    }
}

/// Both vector angles over every quadrant, the axes, zeros, NaN and
/// infinities; the result is `xmm0` (`eax` junk).
#[test]
fn vector_angle() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    let vals = [
        1.0f32, -1.0, 0.5, -2.0, 3.0, -0.25, 0.0, -0.0, nan, inf, -inf, 1.0e-30, -7.5e20,
    ];
    let v = SCRATCH + 0xc00;
    for (rva, second, a_at, b_at) in [(0x1350u32, false, 0u32, 8u32), (0x1450, true, 0xc, 4)] {
        let f = t.at(rva);
        for &a in &vals {
            for &b in &vals {
                t.write(v, &[0x5a; 0x10]);
                t.set_u32(v + a_at, a.to_bits());
                t.set_u32(v + b_at, b.to_bits());
                let got = Rc::new(Cell::new(0u32));
                let g2 = got.clone();
                t.check(f, this_call(v), &[], Eax::Junk, move |g, img, fr| {
                    g2.set(m0153::vector_angle(g, img, fr.at, fr.ecx, second).to_bits());
                    Ret::default()
                });
                assert_eq!(got.get(), t.xmm0_f32(), "{rva:#x}: ({a}, {b})");
            }
        }
    }
}
