//! Helpers of list 5's overlay tests (`magic_NNNN.rs` files that
//! `#[path]` this in next to `magic/mod.rs`).

#![allow(dead_code)]

use super::magic::{Call, Eax, SCRATCH, Twin};

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
pub fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// Stubs for an OPU start-up: the allocation (slot 803), the effect entry
/// (719), the object (715, at SCRATCH) and the set-up's callees; every
/// stub records the words it gets in its own scratch row.
pub fn boot_stubs(t: &mut Twin) {
    // (slot, the words its callee reads)
    let arity = [
        (803u32, 2usize),
        (718, 1),
        (719, 3),
        (715, 7),
        (173, 2),
        (717, 1),
        (720, 2),
        (816, 1),
        (804, 1),
    ];
    for (k, (slot, n)) in (0u32..).zip(arity) {
        let row = SCRATCH + 0x3_0000 + 0x100 * k;
        t.stub(slot, move |m, a| {
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 * i, *w);
            }
            match slot {
                803 => SCRATCH + 0x1_0000,
                719 => a[0] + 0x10,
                715 => SCRATCH,
                _ => 0x7000 + slot,
            }
        });
    }
}

/// Runs an OPU start-up (`f`, the table's `+0x0`) against the port over
/// the battle flag, the mode byte and the battle data. `ops`: the
/// function offsets of the operands (flags, slots, mode, data).
pub fn boot_cases(t: &mut Twin, f: u32, ops: (u32, u32, u32, u32)) {
    boot_stubs(t);
    let flags = t.img.word(f + ops.0);
    let slots = t.img.word(f + ops.1);
    let mode = t.img.word(f + ops.2);
    let data = t.img.word(f + ops.3);
    // the set-up's tables hold old values it must clear
    let setup = t.at(0x1000);
    for (op, len) in [(0x71, 0x100), (0x76, 0x2000), (0xa8, 0x100), (0xad, 0x30)] {
        let at = t.img.word(setup + op);
        t.write(at, &vec![0xa5; len]);
    }
    for (flag, m, d) in [
        (0u8, 0u8, 0u32),
        (0, 0x10, 0x00da_7a00),
        (1, 0x10, 0x00da_7a00),
        (1, 0x0f, 0x00da_7a00),
        (1, 0xff, 0),
        (1, 0x30, 0x00d0_0000),
    ] {
        let fl = t.u32(flags);
        t.set_u8(fl + 0x10, flag);
        t.set_u8(mode, m);
        t.set_u32(data, d);
        let s = t.u32(slots);
        t.set_u32(s + 4, 0);
        t.check_entry(
            f,
            Call {
                eax: 0,
                ecx: 0x77,
                edx: 0,
            },
            &[0x00ab_cdef, 0x1234],
            Eax::Checked,
        );
    }
}

/// The shutdown (`f`, the table's `+0x4`) with and without an object.
pub fn shutdown_cases(t: &mut Twin, f: u32) {
    boot_stubs(t);
    let obj = t.img.word(f + 0x0a);
    let flags = t.u32(t.img.word(f + 0x01));
    for o in [0, SCRATCH + 0x1_0000] {
        t.set_u8(flags + 0x11, 0x5a);
        t.set_u32(obj, o);
        t.check_entry(
            f,
            Call {
                eax: 0x3333,
                ecx: 0,
                edx: 0,
            },
            &[],
            Eax::Checked,
        );
    }
}

/// Every stub of the part callbacks: records all the words it is given
/// into its own scratch row (so a dropped or reordered word shows) and
/// answers something the port must pass on.
pub fn part_stubs(t: &mut Twin) {
    // (slot, the words the original's callee reads)
    let arity = [
        (710u32, 4usize),
        (711, 3),
        (712, 5),
        (713, 1),
        (714, 5),
        (719, 3),
        (722, 2),
        (941, 1),
    ];
    for (k, (slot, n_args)) in (0u32..).zip(arity) {
        let row = SCRATCH + 0x2_0000 + 0x100 * k;
        t.stub(slot, move |m, a| {
            let n = m.r32(row);
            for (i, w) in (0u32..).zip(&a[..n_args]) {
                m.w32(row + 4 + 0x18 * (n % 8) + 4 * i, *w);
            }
            m.w32(row, n + 1);
            match slot {
                941 => SCRATCH + 0x8000,
                719 => {
                    if a[2] == 1 {
                        SCRATCH + 0x9000
                    } else {
                        0x0abc_0000 + a[2]
                    }
                }
                712 => n & 1,
                _ => 0x7100 + slot,
            }
        });
    }
}

pub const PART: u32 = SCRATCH + 0xa000;

pub fn part(t: &mut Twin, key: (u8, u16), state: u8, buf: u32) {
    t.set_u32(PART + 0xd0, SCRATCH + 0x9000);
    t.set_u8(PART + 0x1e, key.0);
    t.set_u16(PART + 0x1a, key.1);
    t.set_u8(PART + 0xbb, state);
    t.set_u32(PART + 0xbc, buf);
    t.set_u8(PART + 0xd4, 2);
    for (i, w) in (0u32..).zip([0x14u32, 0x18, 0x1c, 0x10]) {
        t.set_u32(SCRATCH + 0x9000 + w, 0x0d00_0000 + i);
    }
    for i in 0..4 {
        t.set_u32(SCRATCH + 0x8000 + 4 * i, 0x0e00_0000 + i);
    }
}

/// The part init-data callback (`f`): the entry found at 1 (fresh, or
/// already recorded), not found, no entries, no effect.
pub fn init_data_cases(t: &mut Twin, f: u32) {
    part_stubs(t);
    let slots = t.u32(t.img.word(f + 0x5d));
    let prev = t.img.word(f + 0xaf);
    let effect = SCRATCH + 0xc000;
    for (count, prev1) in [(3u16, 0u32), (3, 5), (1, 0), (0, 0)] {
        part(t, (1, 2), 3, 0);
        t.set_u32(slots + 4, effect);
        t.set_u16(effect + 0x50, count);
        t.set_u32(prev + 4, prev1);
        t.check_entry(f, Call::default(), &[PART], Eax::Checked);
    }
    t.set_u32(slots + 4, 0);
    t.check_entry(f, Call::default(), &[PART], Eax::Checked);
}
