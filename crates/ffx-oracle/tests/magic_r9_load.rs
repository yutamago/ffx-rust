//! The load-step family (`ffx_magic::r9_load`) against each copy's
//! original: slot 225 busy, the first and a later request, the data not
//! loaded yet and loaded (the DLL's start-up then runs on both sides, its
//! pools given free nodes). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Image, l5_objhost, m0032, sa_pool};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// What the start-up's callees need, found by their `Sig`s (`e8`
/// targets from `at` on, `depth` calls deep): pool takes (`count`, `head`
/// operands) and part set-ups (three buffer pointers).
#[derive(Default)]
struct Needs {
    rings: Vec<(u32, u32)>,
    parts: Vec<u32>,
}

fn needs(img: &Image, at: u32, len: u32, depth: u32, out: &mut Needs) {
    for p in at..at + len {
        if img.word(p).to_le_bytes()[0] != 0xe8 {
            continue;
        }
        let t = img.rel32_target(p + 1);
        if !img.in_text(t) {
            continue;
        }
        let ring = if img.matches(t, &sa_pool::TAKE) {
            Some((img.word(t + 0x04), img.word(t + 0x17)))
        } else if img.matches(t, &m0032::POP_NODE) {
            Some((img.word(t + 0x04), img.word(t + 0x13)))
        } else {
            None
        };
        if let Some(r) = ring {
            if !out.rings.contains(&r) {
                out.rings.push(r);
            }
        } else if img.matches(t, &l5_objhost::PART_INIT) {
            if !out.parts.contains(&t) {
                out.parts.push(t);
            }
        } else if depth > 0 {
            needs(img, t, 0x200, depth - 1, out);
        }
    }
}

/// A free ring of 8 nodes at `at` whose head and count are `(count,
/// head)`.
fn ring(t: &mut Twin, (count, head): (u32, u32), at: u32) {
    let node = |i: u32| at + 0x40 * i;
    for i in 0..8 {
        t.write(node(i), &[0x5a; 0x40]);
        t.set_u32(node(i), node((i + 7) % 8));
        t.set_u32(node(i) + 4, node((i + 1) % 8));
    }
    t.set_u32(head, node(0));
    t.set_u16(count, 8);
}

/// Free nodes for each pool the start-up takes from, and buffers for its
/// part set-ups.
fn pools(t: &mut Twin, start: u32) {
    let mut n = Needs::default();
    needs(&t.img, start, 0x300, 1, &mut n);
    for (k, r) in (0u32..).zip(n.rings) {
        ring(t, r, SCRATCH + 0x8000 + 0x400 * k);
    }
    for (k, p) in (0u32..).zip(n.parts) {
        for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
            let at = t.img.word(p + o);
            t.set_u32(at, SCRATCH + 0x4000 + 0x200 * k + 0x40 * j);
        }
    }
}

/// One copy at `rva` (`one`: the 119-byte shape).
fn copy(n: u16, rva: u32, one: bool, extra: fn(&mut Twin, u32)) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let d = u32::from(one);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    let busy = SCRATCH + 0x100;
    let loaded = SCRATCH + 0x104;
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    // the start-up's own slots: a pointer to junk for whatever asks
    let data = SCRATCH + 0x2000;
    t.write(data, &[0x40; 0x400]);
    for s in [719u32, 718, 215, 413, 219, 941] {
        rec(&mut t, s, 1, move |_, _| data);
    }
    for s in [790u32, 12, 333, 314] {
        t.stub_st0(s, move |_, _| f64::from(s) * 0.25);
    }
    t.set_u32(w(&t, 0x26 + d), 0x0000_5e5e);
    let start = t.img.rel32_target(at + 0x5b + d);
    pools(&mut t, start);
    extra(&mut t, start);
    for (b, asked, l) in [
        (1u32, 0u32, 1u32),
        (0, 0, 0),
        (0, 1, 0xffff_ffff),
        (0, 0, 1),
        (0, 1, 2),
    ] {
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o + d), 0x0bad_0000 + o);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// Nothing more to set up.
fn none(_: &mut Twin, _: u32) {}

/// `m0018::start`'s inline take (count `+0xf4`, head `+0x10e`).
fn inline_0018(t: &mut Twin, start: u32) {
    let r = (t.img.word(start + 0xf4), t.img.word(start + 0x10e));
    ring(t, r, SCRATCH + 0xa000);
}

/// `m0046::start_up`'s selector byte (`+0x4e`) and its callee's two
/// pointers.
fn sel_0046(t: &mut Twin, start: u32) {
    let so = t.img.rel32_target(start + 0x49);
    t.set_u32(t.img.word(so + 0x01), SCRATCH + 0x200);
    t.set_u32(t.img.word(so + 0x5b), SCRATCH + 0x300);
    let b = SCRATCH + 0x500;
    t.set_u32(t.img.word(start + 0x4e), b);
    t.write(b, &[2, 0x40]);
}

/// `m0390`'s facing: the selector byte.
fn sel_0390(t: &mut Twin, start: u32) {
    let face = t.img.rel32_target(start + 0x49);
    t.set_u32(t.img.word(face + 0x8c), SCRATCH + 0x180);
    t.set_u32(t.img.word(face + 0x08), SCRATCH + 0x5000);
}

#[test]
fn load_0018() {
    copy(18, 0x1210, false, inline_0018);
}

#[test]
fn load_0019() {
    copy(19, 0x3720, false, inline_0018);
}

#[test]
fn load_0046() {
    copy(46, 0x1210, true, sel_0046);
}

#[test]
fn load_0064() {
    copy(64, 0x26d0, false, none);
}

#[test]
fn load_0171() {
    copy(171, 0x1210, false, none);
}

#[test]
fn load_0367() {
    copy(367, 0x1210, false, none);
}

#[test]
fn load_0369() {
    copy(369, 0x1210, false, none);
}

#[test]
fn load_0383() {
    copy(383, 0x1210, false, none);
}

#[test]
fn load_0390() {
    copy(390, 0x12b0, false, sel_0390);
}

#[test]
fn load_0395() {
    copy(395, 0x3df0, false, none);
}

#[test]
fn load_0447() {
    copy(447, 0x49b0, false, sel_0390);
}
