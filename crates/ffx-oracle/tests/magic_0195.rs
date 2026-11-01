//! `magic_0195.dll` against the port bound to its wrapper at 0x1220
//! (`ffx_magic::m0187`'s 0x1050 port). Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};

const N: u16 = 195;

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// The words each slot's callee reads (the words the function pushes for it).
const SLOTS: [(u32, usize); 3] = [(433, 1), (449, 2), (450, 2)];

/// Every slot answers `0x7000 + slot` and writes the words its callee reads
/// into its own scratch row.
fn record(t: &mut Twin) {
    for (k, (s, n)) in (0u32..).zip(SLOTS) {
        let row = SCRATCH + 0x1_0000 + 0x100 * k;
        t.stub(s, move |m: &mut dyn StubMem, a: &[u32]| {
            for (i, w) in (0u32..).zip(a.iter().take(n)) {
                m.w32(row + 4 * i, *w);
            }
            0x7000 + s
        });
    }
}

#[test]
fn copy_of_the_bound_port() {
    let Some(mut t) = twin() else { return };
    record(&mut t);
    let obj = SCRATCH + 0x6000;
    let entry = Call {
        eax: 0x55,
        ecx: 3,
        edx: 4,
        ..Call::default()
    };
    t.check_entry(t.at(0x1220), entry, &[obj], Eax::Checked);
}

/// 0x10001350: the count step below 7, and past it with the second
/// counter below and at 2.
#[test]
fn count_step_own() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let at = t.at(0x1350);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let ctx = SCRATCH + 0x800;
    t.set_u32(w(&t, 0x04), ctx);
    t.set_u32(ctx + 0x198, 0x0198_0000);
    for (k, s) in (0u32..).zip([461u32, 450, 449]) {
        t.stub(s, move |m: &mut dyn StubMem, a: &[u32]| {
            m.w32(SCRATCH + 0x3000 + 0x40 * k, a[0]);
            m.w32(SCRATCH + 0x3004 + 0x40 * k, a[1]);
            0x0100_0000 + s
        });
    }
    let obj = SCRATCH + 0x1000;
    for (c, d, i) in [
        (0u32, 0u32, 0u32),
        (6, 0, 1),
        (6, 1, 2),
        (9, 5, 0),
        (0xffff_fff0, 0, 0),
    ] {
        t.write(obj, &[0x22; 0x100]);
        t.set_u32(obj + 0xac, i);
        t.set_u32(w(&t, 0x10), c);
        t.set_u32(w(&t, 0x3b), d);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001910 and 0x10001990: the flag step with the word set and clear,
/// and the next step.
#[test]
fn flag_and_next_step() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    for (k, (s, n)) in (0u32..).zip([(444u32, 1usize), (446, 2), (450, 2), (449, 2), (451, 2)]) {
        t.stub(s, move |m: &mut dyn StubMem, a: &[u32]| {
            for (i, v) in (0u32..).zip(&a[..n]) {
                m.w32(SCRATCH + 0x3000 + 0x40 * k + 4 * i, *v);
            }
            0x0100_0000 + s
        });
    }
    let at = t.at(0x1910);
    let table = t.img.word(at + 0x49);
    for b in 0..4 {
        t.set_u32(table + 4 * b, 0x00ab_0000 + b);
    }
    let obj = SCRATCH + 0x1000;
    for (i, b, e) in [(0u32, 2u32, 0u32), (1, 3, 5)] {
        t.write(obj, &[0x22; 0x100]);
        t.set_u32(obj + 0xac, i);
        t.set_u32(obj + 0x2c + 16 * i, b);
        t.set_u32(obj + 0x30 + 16 * i, e);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
    let at = t.at(0x1990);
    t.check_entry(at, Call::default(), &[obj], Eax::Checked);
}

/// 0x10001430: the object start for two table indices.
#[test]
fn object_start_own() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let at = t.at(0x1430);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let ctx = SCRATCH + 0x800;
    t.set_u32(w(&t, 0x11), ctx);
    t.set_u32(ctx + 0x198, 0x0198_0000);
    for (k, (s, n)) in (0u32..).zip([(441u32, 3usize), (440, 4), (450, 2), (449, 2)]) {
        t.stub(s, move |m: &mut dyn StubMem, a: &[u32]| {
            for (i, v) in (0u32..).zip(&a[..n]) {
                m.w32(SCRATCH + 0x3000 + 0x40 * k + 4 * i, *v);
            }
            0x0100_0000 + s
        });
    }
    let table = w(&t, 0x0c);
    for i in 0..3 {
        t.set_u32(table + 4 * i, 0x0c00_0000 + i);
    }
    let obj = SCRATCH + 0x1000;
    for c in [0u32, 2] {
        t.write(obj, &[0x22; 0x140]);
        t.set_u32(w(&t, 0x04), c);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// Slots answering `0x0100_0000 * (k + 1) + slot` and recording their
/// first `n` words (with a call count) in rows of their own.
fn counted(t: &mut Twin, slots: &[(u32, usize)]) {
    for (k, &(s, n)) in (0u32..).zip(slots) {
        let row = SCRATCH + 0x3000 + 0x40 * k;
        t.stub(s, move |m: &mut dyn StubMem, a: &[u32]| {
            let c = m.r32(row);
            m.w32(row, c + 1);
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 + 4 * i, *w ^ c);
            }
            0x0100_0000 * (k + 1) + s
        });
    }
}

/// 0x100017d0: the copy for two list indices of each object.
#[test]
fn copy_start() {
    let Some(mut t) = twin() else { return };
    let at = t.at(0x17d0);
    let copy = SCRATCH + 0x2000;
    t.stub(434, move |m: &mut dyn StubMem, a: &[u32]| {
        m.w32(SCRATCH + 0x3000, a[0]);
        m.w32(SCRATCH + 0x3004, a[1]);
        copy
    });
    let obj = SCRATCH + 0x1000;
    for (i, j) in [(0u32, 1u32), (2, 0), (1, 1)] {
        t.write(copy, &[0x33; 0x100]);
        for k in 0..0x40 {
            t.set_u32(obj + 4 * k, 0x0b00_0000 + k);
        }
        t.set_u32(obj, 0x10);
        t.set_u32(obj + 0xac, i);
        t.set_u32(copy + 0xac, j);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001830: the copy's start in mode 1 and another mode.
#[test]
fn table_start() {
    let Some(mut t) = twin() else { return };
    let at = t.at(0x1830);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    counted(&mut t, &[(440, 4), (450, 2), (451, 2), (449, 2)]);
    let modes = w(&t, 0x17);
    for c in 0..4u32 {
        t.set_u32(modes + 4 * c, [1, 0, 2, 1][c as usize]);
        for (k, op) in (0u32..).zip([0x21, 0x2d, 0x39]) {
            t.set_u32(w(&t, op) + 16 * c, 0x0d00_0000 + 0x10 * c + k);
        }
    }
    let obj = SCRATCH + 0x1000;
    for (i, c) in [(0u32, 0u32), (1, 1), (2, 2), (0, 3)] {
        t.write(obj, &[0x22; 0x140]);
        t.set_u32(obj + 0xac, i);
        t.set_u32(obj + 0x2c + 16 * i, c);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
