//! The node family's functions the shared-B list ported
//! (`ffx_magic::sb_nodes`) against `magic_0003.dll`. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;

use ffx_magic::{Ret, Xmm, sb_nodes};
use magic::{Call, Eax, SCRATCH, TRAPS, Twin};

const N: u16 = 3;

/// The cursor: `+0` the owner node, `+4` the flag, `+5` the pass.
const GROUP: u32 = SCRATCH + 0x1000;
/// The owner node: `+0x10` its first child, `+0x14` its parent.
const OWNER: u32 = SCRATCH + 0x2000;
const PARENT: u32 = SCRATCH + 0x2400;
/// Children, 0x100 bytes apart.
const KIDS: u32 = SCRATCH + 0x3000;
/// The owner's other nodes (`+0x2c`, `+0x30`, `+0x38`, `+0x3c`).
const NODE_A: u32 = SCRATCH + 0x5000;
const NODE_B: u32 = SCRATCH + 0x5400;
/// Children of `NODE_A`, 0x100 bytes apart.
const GRANDKIDS: u32 = SCRATCH + 0x6000;
/// Vtables, 0x40 bytes apart.
const VTABLES: u32 = SCRATCH + 0x800;
const LOG: u32 = SCRATCH + 0x4000;

fn trap(s: u32) -> u32 {
    TRAPS + 16 * s
}

fn twin() -> Option<Twin> {
    twin_of(N)
}

fn twin_of(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// Methods 1000..1004 (table words, so both sides see slots): 1000 logs
/// its cursor and the cursor's two words, 1001 changes the group's flag
/// and pass, 1002 answers 0, 1003 logs the cursor's second word, 1004
/// sets its cursor's pass byte. Slots 229, 668, 779, 801 log their words.
fn stubs(t: &mut Twin) {
    for s in 1000..1005 {
        t.set_u32(magic::TABLE + 4 * s, trap(s));
    }
    t.stub(1000, |m, a| {
        let n = m.r32(LOG + 8).wrapping_add(1);
        m.w32(LOG + 8, n);
        m.w32(LOG + 0x100 + 0x10 * n, a[0]);
        m.w32(LOG + 0x104 + 0x10 * n, m.r32(a[0]));
        m.w32(LOG + 0x108 + 0x10 * n, m.r32(a[0] + 4) & 0xff00);
        0x5a00_0000 + n
    });
    t.stub(1001, |m, a| {
        m.w32(LOG + 0x10, a[0]);
        m.w32(GROUP + 4, 0x0000_3c00);
        0xc0de_0000
    });
    t.stub(1003, |m, a| {
        m.w32(LOG + 0x1c, m.r32(a[0] + 4) & 0xff00);
        0x1003_0003
    });
    t.stub(1004, |m, a| {
        let v = m.r32(a[0] + 4) & 0xffff_00ff;
        m.w32(a[0] + 4, v | 0x5d00);
        0x1004_0004
    });
    for (k, (s, n)) in [(229u32, 3), (668, 3), (779, 2), (801, 6)]
        .into_iter()
        .enumerate()
    {
        let row = LOG + 0x40 + 0x40 * u32::try_from(k).unwrap();
        t.stub(s, move |m, a| {
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 * i, *w);
            }
            0x0100_0000 + s
        });
    }
}

/// A circular child list of `parent` at `first` (0x100 apart), one child
/// per method (vtable `+off` the method; vtables from `vt`).
fn kids(t: &mut Twin, parent: u32, first: u32, vt: u32, off: u32, methods: &[u32]) {
    for (k, &m) in methods.iter().enumerate() {
        let k = u32::try_from(k).unwrap();
        let c = first + 0x100 * k;
        let v = vt + 0x40 * k;
        t.set_u32(v + off, m);
        t.set_u32(c, v);
        let prev = if k == 0 { parent } else { c - 0x100 };
        let next = if k as usize + 1 == methods.len() {
            parent
        } else {
            c + 0x100
        };
        t.set_u32(c + 4, prev);
        t.set_u32(c + 8, next);
        t.set_u32(c + 0x14, parent);
        t.set_u8(c + 0x1a, 0x30 + u8::try_from(k).unwrap());
        for i in 0..4 {
            t.set_u32(c + 0x1c + 4 * i, (0x3f80_0000 + 0x1_1111 * k) ^ (i << 3));
        }
    }
    t.set_u32(
        parent + 0x10,
        if methods.is_empty() { parent } else { first },
    );
}

/// The cursor, its owner (children with `methods`), the owner's parent and
/// the owner's node words: `+0x2c` A, `+0x30` B, `+0x38` A, `+0x3c` B
/// (A's method `ma`, B's `mb`).
fn group(t: &mut Twin, flag: u8, mode: u8, owner_mode: u8, methods: &[u32], ma: u32, mb: u32) {
    t.write(LOG, &[0; 0x200]);
    t.set_u32(GROUP, OWNER);
    t.set_u8(GROUP + 4, flag);
    t.set_u8(GROUP + 5, mode);
    t.set_u8(OWNER + 0x1a, owner_mode);
    t.set_u32(OWNER + 0x14, PARENT);
    kids(t, OWNER, KIDS, VTABLES, 0x1c, methods);
    for i in 0..6 {
        let f = 1.5f32 + 0.25 * f32::from(u8::try_from(i).unwrap());
        t.set_u32(OWNER + 0x1c + 4 * i, f.to_bits());
        t.set_u32(PARENT + 0x1c + 4 * i, (-3.0f32 * f).to_bits());
    }
    t.set_u32(OWNER + 0x2c, NODE_A);
    t.set_u32(OWNER + 0x30, NODE_B);
    t.set_u32(OWNER + 0x38, NODE_A);
    t.set_u32(OWNER + 0x3c, NODE_B);
    for (n, m, v) in [(NODE_A, ma, VTABLES + 0x400), (NODE_B, mb, VTABLES + 0x440)] {
        t.set_u32(v + 0x1c, m);
        t.set_u32(n, v);
        for i in 0..4 {
            t.set_u32(
                n + 0x1c + 4 * i,
                (0.75f32 * f32::from(u8::try_from(i + 1).unwrap())).to_bits(),
            );
        }
    }
}

/// (flag, mode, owner's `+0x1a`): equal modes take the early exit.
const MODES: [(u8, u8, u8); 4] = [(0, 3, 3), (1, 7, 7), (0, 3, 9), (1, 0, 0xff)];

fn method_sets() -> [Vec<u32>; 3] {
    [
        vec![],
        vec![trap(1000)],
        vec![trap(1001), trap(1002), trap(1000)],
    ]
}

fn this_call(ecx: u32, eax: u32) -> Call {
    Call {
        eax,
        ecx,
        edx: 0x77,
        ..Call::default()
    }
}

#[test]
fn visit_pair_229() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    for (flag, mode, om) in MODES {
        for ms in method_sets() {
            for (ma, mb) in [(trap(1000), trap(1003)), (trap(1001), trap(1000))] {
                group(&mut t, flag, mode, om, &ms, ma, mb);
                t.check(
                    t.at(0x3f00),
                    this_call(GROUP, 0x1234_5678),
                    &[],
                    Eax::Checked,
                    |g, img, fr| {
                        Ret::eax(sb_nodes::visit_pair_229(
                            g, img, fr.at, fr.esp, fr.ecx, fr.eax,
                        ))
                    },
                );
            }
        }
    }
}

#[test]
fn visit_add_vector() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    for (flag, mode, om) in MODES {
        for ms in method_sets() {
            group(&mut t, flag, mode, om, &ms, 0, 0);
            t.check(
                t.at(0x61b0),
                this_call(GROUP, 0xffff_ff00),
                &[],
                Eax::Checked,
                |g, _img, fr| Ret::eax(sb_nodes::visit_add_vector(g, fr.ecx, fr.eax)),
            );
        }
    }
}

#[test]
fn visit_offsets() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    for (flag, mode, om) in MODES {
        for ms in method_sets() {
            // A's children: none, one, one that changes the pass
            for (a_mode, grand) in [
                (mode, vec![trap(1000)]),
                (mode ^ 1, vec![]),
                (mode ^ 1, vec![trap(1000), trap(1003)]),
                (0x40, vec![trap(1004), trap(1000)]),
            ] {
                for ma in [trap(1000), trap(1004)] {
                    group(&mut t, flag, mode, om, &ms, ma, 0);
                    kids(&mut t, NODE_A, GRANDKIDS, VTABLES + 0x600, 0x1c, &grand);
                    t.set_u8(NODE_A + 0x1a, a_mode);
                    t.check(
                        t.at(0x3ab0),
                        this_call(GROUP, 0x0101_0101),
                        &[],
                        Eax::Checked,
                        |g, _img, fr| {
                            Ret::eax(sb_nodes::visit_offset_int(g, fr.esp, fr.ecx, fr.eax))
                        },
                    );
                    group(&mut t, flag, mode, om, &ms, ma, 0);
                    t.check(
                        t.at(0x3b70),
                        this_call(GROUP, 0x0202_0202),
                        &[],
                        Eax::Checked,
                        |g, _img, fr| {
                            Ret::eax(sb_nodes::visit_offset_float(g, fr.esp, fr.ecx, fr.eax))
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn visit_668() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    for (flag, mode, om) in MODES {
        for ms in method_sets() {
            for mb in [trap(1000), trap(1003)] {
                group(&mut t, flag, mode, om, &ms, 0, mb);
                t.check_entry(
                    t.at(0x3310),
                    Call {
                        eax: 0x3333,
                        ecx: 0x4444,
                        edx: 0x5555,
                        ..Call::default()
                    },
                    &[GROUP],
                    Eax::Checked,
                );
            }
        }
    }
}

#[test]
fn visit_children_20() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let f = t.at(0x5d60);
    let gate = t.img.word(f + 0x0b);
    let node = OWNER;
    let list = node + 4;
    for gate_v in [0u32, 1, 0x8000_0000] {
        for ms in method_sets() {
            {
                group(&mut t, 0, 0, 0, &[], 0, 0);
                t.set_u32(gate, gate_v);
                kids(&mut t, node, KIDS, VTABLES, 0x20, &ms);
                t.set_u32(
                    node + 0x10,
                    if gate_v == 1 {
                        0x0f0f_0f0f
                    } else {
                        0xf0f0_f0f2
                    },
                );
                // `kids` links the list through the node (+0x10 first):
                // relink at the list (+0xc first, also the list's +8, which
                // tells an empty list; the list ends it)
                for k in 0..ms.len() {
                    let c = KIDS + 0x100 * u32::try_from(k).unwrap();
                    if k + 1 == ms.len() {
                        t.set_u32(c + 8, list);
                    }
                }
                t.set_u32(node + 0xc, if ms.is_empty() { list } else { KIDS });
                t.check_entry(f, Call::default(), &[GROUP], Eax::Checked);
            }
        }
    }
}

#[test]
fn call_779_flagged() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let f = t.at(0x3e30);
    let pg = t.img.word(f + 0x13);
    let word = SCRATCH + 0x7000;
    // the slot sees the flag set: it records the word
    t.stub(779, move |m, a| {
        m.w32(LOG + 0x100, a[0]);
        m.w32(LOG + 0x104, a[1]);
        m.w32(LOG + 0x108, m.r32(word));
        0x779
    });
    for old in [0u32, 7, 0xffff_ffff] {
        group(&mut t, 0, 0, 0, &[], 0, 0);
        t.set_u32(OWNER + 0x20, 0x00ab_cdef);
        t.set_u32(pg, word);
        t.set_u32(word, old);
        t.check_entry(f, Call::default(), &[GROUP], Eax::Checked);
    }
}

#[test]
fn float_289() {
    let Some(mut t) = twin() else { return };
    t.stub(289, |m, a| {
        m.w32(LOG, a[0]);
        0x289
    });
    for v in [0.0f64, 1.5, -2.25, 1e40, 1.000_000_1, f64::NAN] {
        t.stub_st0(289, move |_, a| v + f64::from(a[0] & 0xf));
        for ecx in [0u32, 3, 0x1234_5678] {
            let r = t.check(
                t.at(0x7480),
                Call::this(ecx),
                &[],
                Eax::Junk,
                |g, img, fr| {
                    let (x, eax) = sb_nodes::float_289(g, img, fr.at, fr.ecx);
                    Ret::xmm0(Xmm::f32(x), eax)
                },
            );
            assert!(r.xmm0.is_some());
        }
    }
}

#[test]
fn scale_ratio_801() {
    let Some(mut t) = twin() else { return };
    stubs(&mut t);
    let f = t.at(0x1870);
    let saved = [0x94u32, 0x9c, 0xb1, 0xb9].map(|o| t.img.word(f + o));
    let lo = f32::from_bits(t.u32(t.img.word(f + 0x04)));
    let hi = f32::from_bits(t.u32(t.img.word(f + 0x45)));
    let this = SCRATCH + 0x7100;
    let cases: [[f32; 4]; 8] = [
        [1.0, 2.0, 3.0, 4.0],
        [lo, hi, lo - 1.0, hi + 1.0],
        [4.0, 2.0, 1.0, 0.5],
        [2.0, 2.0, 2.0, 2.0],
        [f32::NAN, 1.0, f32::NAN, 3.0],
        [0.5, f32::NAN, 7.0, f32::INFINITY],
        [-1.0, 50.0, 99.0, 0.02],
        [0.005, -0.0, 150.0, 99.99],
    ];
    for v in cases {
        for s in [[1.0f32, 2.0, 4.0, 0.5], [3.0, 0.25, 1.0, 1.0]] {
            for (i, x) in (0u32..).zip(v) {
                t.set_u32(this + 4 * i, x.to_bits());
            }
            for (a, x) in saved.iter().zip(s) {
                t.set_u32(*a, x.to_bits());
            }
            t.check(f, this_call(this, 0x99), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(sb_nodes::scale_ratio_801(g, img, fr.at, fr.ecx))
            });
        }
    }
}

/// Slot `s` logs its first `n` words in row `k` (0x40 bytes from
/// `LOG + 0x200`) and answers `ret`.
fn log_slot(t: &mut Twin, s: u32, n: usize, k: u32, ret: u32) {
    let row = LOG + 0x200 + 0x40 * k;
    t.stub(s, move |m, a| {
        for (i, w) in (0u32..).zip(&a[..n]) {
            m.w32(row + 4 * i, *w);
        }
        ret
    });
}

/// A set-up record at `REC` for the node at `NODE_A` in `list` (0 for
/// none; else `PARENT` with `last` as its last child).
const REC: u32 = SCRATCH + 0x7800;

fn record(t: &mut Twin, list: u32, last: u32) {
    t.write(NODE_A, &[0xa5; 0x100]);
    t.write(PARENT, &[0x5a; 0x40]);
    t.write(KIDS, &[0x3c; 0x40]);
    t.set_u32(REC, NODE_A);
    t.set_u32(REC + 4, list);
    t.set_u32(REC + 8, 0x0808_0808);
    t.set_u32(REC + 0xc, SCRATCH + 0x7a00);
    t.set_u32(REC + 0x10, 0x1010_1010);
    t.set_u32(REC + 0x14, 3);
    t.set_u32(PARENT + 0x10, last);
    t.set_u16(PARENT + 0x18, 0xfffe);
    t.set_u32(SCRATCH + 0x7a10, 0x7a10);
}

#[test]
fn particle_node() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x3590);
    let p = t.img.word(f + 0x45);
    let wrapper = t.img.rel32_target(f + 0x88);
    let lock_g = t.img.word(wrapper + 0x04);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(lock_g, lock);
    t.set_u32(lock, 0x77);
    t.set_u32(p, SCRATCH + 0x7f10);
    // slot 941's table: program words
    let table = SCRATCH + 0x7c00;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x9000 + i);
    }
    log_slot(&mut t, 941, 1, 0, table);
    // pppStartPart (slot 714) sees the lock set
    t.stub(714, move |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(LOG + 0x240 + 4 * i, *w);
        }
        m.w32(LOG + 0x260, m.r32(lock));
        0x714
    });
    log_slot(&mut t, 325, 2, 3, 0x325);
    for (list, last) in [(0, 0), (PARENT, PARENT), (PARENT, KIDS)] {
        record(&mut t, list, last);
        t.check(f, Call::this(REC), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(sb_nodes::particle_node(g, img, fr.at, fr.ecx))
        });
    }
}

#[test]
fn effect_node() {
    let Some(mut t) = twin() else { return };
    log_slot(&mut t, 719, 3, 0, 0x7190_0719);
    for (list, last) in [(0, 0), (PARENT, PARENT), (PARENT, KIDS)] {
        record(&mut t, list, last);
        t.check(
            t.at(0x4060),
            Call::this(REC),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(sb_nodes::effect_node(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn axis_sprites() {
    axis_sprites_in(N, 0x33a0);
}

/// The copy in `magic_0087` (other read-only constants, the same code).
#[test]
fn axis_sprites_0087() {
    axis_sprites_in(87, 0x35e0);
}

fn axis_sprites_in(n: u16, rva: u32) {
    let Some(mut t) = twin_of(n) else { return };
    stubs(&mut t);
    let buf = SCRATCH + 0x8000;
    log_slot(&mut t, 231, 3, 0, 0x231);
    log_slot(&mut t, 264, 1, 1, buf);
    // slot 565 logs the frame vector and changes it: the port reads the
    // axis again after it
    t.stub(565, |m, a| {
        let n = m.r32(LOG + 0x2c0).wrapping_add(1);
        m.w32(LOG + 0x2c0, n);
        let row = LOG + 0x300 + 0x20 * n;
        for i in 0..4 {
            m.w32(row + 4 * i, a[i as usize]);
            m.w32(row + 0x10 + 4 * i, m.r32(a[2] + 4 * i));
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) + 0.5;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x565
    });
    log_slot(&mut t, 382, 2, 2, 0x382);
    log_slot(&mut t, 265, 1, 3, 0x2650_0265);
    for (flag, mode, om) in MODES {
        for ms in method_sets() {
            group(&mut t, flag, mode, om, &ms, 0, 0);
            t.set_u32(OWNER + 0x1c, NODE_A);
            t.write(buf, &[0xee; 0x200]);
            t.check(
                t.at(rva),
                this_call(GROUP, 0x4444),
                &[],
                Eax::Checked,
                |g, img, fr| Ret::eax(sb_nodes::axis_sprites(g, img, fr.at, fr.esp, fr.ecx)),
            );
        }
    }
}

#[test]
fn bounce() {
    bounce_in(N, 0x6030);
}

/// The copy in `magic_0087` (other read-only constants, the same code).
#[test]
fn bounce_0087() {
    bounce_in(87, 0x61b0);
}

fn bounce_in(n: u16, rva: u32) {
    let Some(mut t) = twin_of(n) else { return };
    stubs(&mut t);
    // slot 248: a[0] = a[1] + a[2] (three floats), as FFXVu0AddVector
    t.stub(248, |m, a| {
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        m.w32(LOG + 0x280, a[1]);
        m.w32(LOG + 0x284, a[2]);
        0x248
    });
    t.stub(325, |m, a| {
        m.w32(LOG + 0x290, a[0]);
        for i in 0..4 {
            m.w32(LOG + 0x294 + 4 * i, m.r32(a[1] + 4 * i));
        }
        0x325
    });
    for (flag, mode, om) in MODES {
        for ms in method_sets() {
            for floor in [-100.0f32, 0.0, 4.0, 100.0, f32::NAN] {
                group(&mut t, flag, mode, om, &ms, 0, 0);
                t.set_u32(OWNER + 0x34, floor.to_bits());
                t.check(
                    t.at(rva),
                    this_call(GROUP, 0x5555),
                    &[],
                    Eax::Checked,
                    |g, img, fr| Ret::eax(sb_nodes::bounce(g, img, fr.at, fr.esp, fr.ecx)),
                );
            }
        }
    }
}

#[test]
fn draw_model() {
    draw_model_in(N, 0x3c20);
}

/// The copy in `magic_0087` (other read-only constants, the same code).
#[test]
fn draw_model_0087() {
    draw_model_in(87, 0x3e70);
}

fn draw_model_in(n: u16, rva: u32) {
    let Some(mut t) = twin_of(n) else { return };
    stubs(&mut t);
    let f = t.at(rva);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(f + 0x11a), lock);
    let pw = SCRATCH + 0x7f20;
    t.set_u32(t.img.word(f + 0xb6), pw);
    t.set_u32(pw, 0x00c0_ffee);
    let s = SCRATCH + 0x7e00;
    // each slot logs its words and the lock word it sees
    for (k, (slot, n, ret)) in [
        (231u32, 3usize, 0x231u32),
        (819, 4, 0x819),
        (775, 2, 0x775),
        (776, 1, 0x0700),
        (777, 1, 0x0007),
        (820, 5, 0x820),
    ]
    .into_iter()
    .enumerate()
    {
        let row = LOG + 0x200 + 0x40 * u32::try_from(k).unwrap();
        t.stub(slot, move |m, a| {
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 * i, *w);
            }
            m.w32(row + 0x3c, m.r32(lock));
            ret
        });
    }
    // the draw record on the frame: below the entry, so logged
    for (slot, n, row) in [(819u32, 4usize, LOG + 0x500), (820, 5, LOG + 0x600)] {
        t.stub(slot, move |m, a| {
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 * i, *w);
            }
            m.w32(row + 0x14, m.r32(lock));
            m.w32(row + 0x18, m.r32(a[2] + 0x1c));
            for i in 0..0x12 {
                m.w32(row + 0x20 + 4 * i, m.r32(a[3] + 4 * i));
            }
            slot
        });
    }
    let ready = SCRATCH + 0x7f30;
    t.stub(773, move |m, a| {
        m.w32(LOG + 0x3f0, a[0]);
        m.w32(LOG + 0x3f4, a[1]);
        m.w32(LOG + 0x3f8, m.r32(lock));
        m.r32(ready)
    });
    for (flag, mode, om) in MODES {
        for ms in method_sets() {
            for (b0, r) in [(0u8, 0u32), (1, 1), (2, 0), (3, 0x8000_0000), (0xfe, 5)] {
                group(&mut t, flag, mode, om, &ms, 0, 0);
                t.set_u32(OWNER + 0x20, s);
                t.set_u32(OWNER + 0x1c, NODE_A);
                t.set_u8(s, b0);
                t.set_u32(s + 8, 0x100);
                t.set_u32(s + 0xc, 0x200);
                t.set_u16(s + 0x16, 0x1234);
                t.set_u32(lock, 0x42);
                t.set_u32(ready, r);
                // junk under the entry: the draw record's fields must be written
                t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
                t.check(
                    f,
                    this_call(GROUP, 0x6666),
                    &[],
                    Eax::Checked,
                    |g, img, fr| Ret::eax(sb_nodes::draw_model(g, img, fr.at, fr.esp, fr.ecx)),
                );
            }
        }
    }
}
