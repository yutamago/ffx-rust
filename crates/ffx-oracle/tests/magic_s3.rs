//! The shared-3 list's ports against the DLLs they were ported from
//! (`ffx_magic::s3_*`, the frameless slot thunks of `ffx_magic::runtime`).
//! The register thunks of `ffx_magic::s3_wrap` are checked with their
//! handlers' tests (`Twin::check_thunked`, `sa::check_this_thunk`).
//! Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;

use magic::{Call, Eax, Twin};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// `mov eax, [G]; mov eax, [eax + 4 * slot]; jmp eax` (13 bytes): the
/// slot's function with the caller's stack.
#[test]
fn bare_slot_thunks() {
    for (n, rvas) in [(297u16, &[0x8bf0u32, 0x8be0, 0x8bd0][..]), (275, &[0x3110])] {
        let Some(mut t) = twin(n) else { return };
        for rva in rvas {
            let slot = (t.img.word(t.at(*rva) + 7)) / 4;
            t.stub(slot, move |m, a| {
                m.w32(magic::SCRATCH + 4 * slot, a[0] ^ a[2]);
                a[1].wrapping_add(slot)
            });
            t.check_entry(
                t.at(*rva),
                Call {
                    eax: 1,
                    ecx: 0x5555,
                    edx: 7,
                    ..Call::default()
                },
                &[3, 5, 9, 11],
                Eax::Checked,
            );
        }
    }
}

use ffx_magic::{Ret, s3_nodes};
use magic::SCRATCH;

const LOG: u32 = SCRATCH + 0x3_0000;

/// Slot `s` logs its first `n` words in row `k` (0x80 bytes from `LOG`),
/// the four words behind its first `deref` arguments too, and answers
/// `ret`.
fn log_slot(t: &mut Twin, s: u32, n: usize, deref: usize, k: u32, ret: u32) {
    let row = LOG + 0x80 * k;
    t.stub(s, move |m, a| {
        let c = m.r32(row + 0x7c).wrapping_add(1);
        m.w32(row + 0x7c, c);
        for (i, w) in (0u32..).zip(&a[..n]) {
            m.w32(row + 4 * i, w.wrapping_add(c << 28));
        }
        for (i, p) in (0u32..).zip(&a[..deref]) {
            for j in 0..4 {
                m.w32(row + 0x40 + 0x10 * i + 4 * j, m.r32(p + 4 * j));
            }
        }
        ret
    });
}

/// `magic_0003` 0x10001340: the node leaves its parent (every position in
/// the parent's list, or no parent), then the display resets.
#[test]
fn leave_and_reset() {
    let Some(mut t) = twin(3) else { return };
    let f = t.at(0x1340);
    let ratio = t.at(0x1870);
    for (o, x) in [(0x94u32, 1.0f32), (0x9c, 2.0), (0xb1, 4.0), (0xb9, 0.5)] {
        t.set_u32(t.img.word(ratio + o), x.to_bits());
    }
    t.set_u32(t.img.word(f + 0xfc), 0.75f32.to_bits());
    for (k, (s, n, d)) in [
        (332u32, 1usize, 1usize),
        (353, 2, 1),
        (267, 3, 0),
        (271, 4, 0),
        (669, 5, 1),
        (785, 1, 0),
        (801, 6, 0),
    ]
    .into_iter()
    .enumerate()
    {
        log_slot(&mut t, s, n, d, k as u32, 0x0100_0000 + s);
    }
    let (cur, node, parent, s1, s2) = (
        SCRATCH,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x3000,
        SCRATCH + 0x3400,
    );
    let floats: [[f32; 4]; 3] = [
        [1.5, -2.7, 3.0e9, f32::NAN],
        [0.0, -0.0, 65_535.9, -2_147_483_648.0],
        [100.25, -1.0e-3, 7.0, -2_147_483_904.0],
    ];
    for (k, (p, prev, next)) in [
        (0, node, node),
        (parent, parent, parent),
        (parent, parent, s1),
        (parent, s1, parent),
        (parent, s1, s2),
    ]
    .into_iter()
    .enumerate()
    {
        for v in &floats {
            for a in [node, parent, s1, s2] {
                t.write(a, &[0x33; 0x100]);
            }
            t.set_u32(cur, node);
            t.set_u32(node + 0x14, p);
            t.set_u32(node + 4, prev);
            t.set_u32(node + 8, next);
            t.set_u16(parent + 0x18, k as u16);
            for (i, x) in (0u32..).zip(v) {
                t.set_u32(node + 0x7c + 4 * i, x.to_bits());
            }
            t.check(f, Call::this(cur), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(s3_nodes::leave_and_reset(g, img, fr))
            });
        }
    }
}

/// `magic_0042` 0x100083c0 and 0x10008570: a node's set-up record on the
/// frame for the particle and effect node builders.
#[test]
fn node_with_record() {
    let Some(mut t) = twin(42) else { return };
    let f = t.at(0x83c0);
    let pn = t.img.rel32_target(f + 0x35);
    let wrapper = t.img.rel32_target(pn + 0x88);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(t.img.word(wrapper + 0x04), lock);
    t.set_u32(lock, 0x77);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
    let owner = SCRATCH + 0x7a00;
    t.set_u32(t.img.word(f + 0x0d), owner);
    for i in 0..0x40 {
        t.set_u32(owner + 4 * i, 0x0a00_0000 + i);
    }
    let table = SCRATCH + 0x7c00;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x9000 + i);
    }
    log_slot(&mut t, 941, 1, 0, 0, table);
    t.stub(714, move |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(LOG + 0x400 + 4 * i, *w);
        }
        m.w32(LOG + 0x420, m.r32(lock));
        0x714
    });
    log_slot(&mut t, 325, 2, 2, 1, 0x325);
    log_slot(&mut t, 719, 3, 0, 2, 0x7190_0719);
    let node = SCRATCH + 0x1000;
    for (k, rva) in [0x83c0u32, 0x8570].into_iter().enumerate() {
        // the DLL's list: empty, or with a last child
        let list = t
            .img
            .word(t.at(rva) + if rva == 0x83c0 { 0x26 } else { 0x1e });
        let kid = SCRATCH + 0x1400;
        for (index, last) in [(0u32, list), (3, kid), (7, list)] {
            t.write(node, &[0xa5; 0x100]);
            t.write(kid, &[0x3c; 0x40]);
            t.set_u32(list + 0x10, last);
            t.set_u16(list + 0x18, 0xfffe);
            t.check(
                t.at(rva),
                Call::this(node),
                &[0x0808_0808, 0x0c0c_0c0c + k as u32, 0xdead, index],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(if rva == 0x83c0 {
                        s3_nodes::particle_node_with(g, img, fr)
                    } else {
                        s3_nodes::effect_node_with(g, img, fr)
                    })
                },
            );
        }
    }
}
