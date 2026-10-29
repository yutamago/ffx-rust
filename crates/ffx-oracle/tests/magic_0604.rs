//! `magic_0604.dll` (battle-overdrive) against its Rust port: the runtime
//! (`DllMain`, `InitMagicPRX`, `GetEffectOverlayTable`, the slot thunks),
//! the OPU host and the DLL's own two entries. Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 604;

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

#[test]
fn get_effect_overlay_table() {
    let Some(mut t) = twin() else { return };
    let f = t.img.export("GetEffectOverlayTable").unwrap();
    for which in [0, 1, 2, 0xffff_ffff] {
        t.check_entry(f, Call::default(), &[which], Eax::Checked);
    }
}

#[test]
fn init_magic_prx_copies_the_table_words() {
    let Some(mut t) = Twin::load(N) else { return };
    // distinct words in every data slot and in g_OpusBattle's 64 bytes
    for slot in 741..1024u32 {
        if let Some(d) = data_slot(&t, slot) {
            t.set_u32(d, 0x5a00_0000 + slot);
            t.set_u32(magic::TABLE + 4 * slot, d);
        }
    }
    let opus = t.u32(magic::TABLE + 0xb9c);
    for i in 0..16 {
        t.set_u32(opus + 4 * i, 0x0b00_0000 + i);
    }
    let f = t.img.export("InitMagicPRX").unwrap();
    t.check_entry(f, Call::default(), &[magic::TABLE], Eax::Checked);
}

fn data_slot(t: &Twin, slot: u32) -> Option<u32> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.data(slot))).ok()
}

/// The OPU host's globals in this DLL (operands of `start`).
struct G {
    obj: u32,
    cur: u32,
    lock: u32,
}

fn globals(t: &Twin) -> G {
    let f = t.at(0x10e0);
    G {
        obj: t.img.word(f + 0x03),
        cur: t.img.word(f + 0x0f),
        lock: t.img.word(f + 0x17),
    }
}

/// An OPU object at SCRATCH with `+0x54`/`+0x58` and the lock flag set.
fn opu_object(t: &mut Twin, over: u32, saved: u32, cur: u32, lock_value: u32) -> G {
    let g = globals(t);
    t.set_u32(g.obj, SCRATCH);
    t.set_u32(SCRATCH + 0x54, saved);
    t.set_u32(SCRATCH + 0x58, over);
    t.set_u32(g.cur, cur);
    let lock = t.u32(g.lock);
    t.set_u32(lock, lock_value);
    g
}

#[test]
fn opu_start_run_stop() {
    let Some(mut t) = twin() else { return };
    // slot results the port must pass on; the OPU calls see the game flag
    // (copied to scratch) and may move the object's current (+0x54)
    let flag = t.u32(globals(&t).lock);
    t.stub(716, move |m, a| {
        m.w32(SCRATCH + 0x200 + 4 * a[1], m.r32(flag));
        if a[1] == 0 {
            m.w32(SCRATCH + 0x54, m.r32(SCRATCH + 0x60));
        }
        0x7160 + a[1]
    });
    t.stub(721, move |m, _| {
        m.w32(SCRATCH + 0x210, m.r32(flag));
        0x7210
    });
    t.stub(727, |_, _| 0x7270);
    t.stub(724, |m, a| {
        m.w32(SCRATCH + 0x100, a[0]);
        0x7240
    });
    for (over, saved, moved) in [
        (0, 0, 0),
        (0, 0x1234, 0x1234),
        (0x55, 0x55, 0x55),
        (0x55, 0x66, 0x55),
        (0x55, 0x66, 0x66),
        (0x77, 0, 0x77),
    ] {
        for rva in [0x10e0, 0x1160, 0x11f0] {
            let _ = opu_object(&mut t, over, saved, 0xc0c0, 0x99);
            t.set_u32(SCRATCH + 0x60, moved);
            t.set_u32(t.img.word(t.at(0x1160) + 0x1e), 0x00ca_0000);
            t.set_u32(t.img.word(t.at(0x1160) + 0x31), 0x005c_0000);
            t.check_entry(
                t.at(rva),
                Call {
                    eax: 0xeeee,
                    ecx: 0x4444,
                    edx: 0,
                    ..Call::default()
                },
                &[],
                Eax::Checked,
            );
        }
    }
    // no object: stop leaves eax as it was
    let g = globals(&t);
    t.set_u32(g.obj, 0);
    t.check_entry(
        t.at(0x11f0),
        Call {
            eax: 0xabcd,
            ecx: 0,
            edx: 0,
            ..Call::default()
        },
        &[],
        Eax::Checked,
    );
}

#[test]
fn ret_keeps_eax() {
    let Some(mut t) = twin() else { return };
    t.check_entry(
        t.at(0x10d0),
        Call {
            eax: 0x1234_5678,
            ecx: 1,
            edx: 2,
            ..Call::default()
        },
        &[7],
        Eax::Checked,
    );
}

#[test]
fn slot_thunks_jump_with_the_stack() {
    let Some(mut t) = twin() else { return };
    t.stub(911, |_, a| a[0] ^ a[1]);
    t.stub(71, |_, a| a[2]);
    t.stub(156, |_, a| a[0] + 1);
    for rva in [0x2430, 0x1dd0, 0x2210] {
        t.check_entry(t.at(rva), Call::default(), &[3, 5, 9, 11], Eax::Checked);
    }
}

/// Table +0x0 with the slots answering real-looking pointers: the
/// allocation, the OPU object (at SCRATCH) and the effect entry.
#[test]
fn start_up_and_shutdown() {
    let Some(mut t) = twin() else { return };
    t.stub(803, |_, _| SCRATCH + 0x1_0000);
    t.stub(719, |_, a| a[0] + 0x10);
    // slot 715 sees every word the original leaves on the stack (11)
    t.stub(715, |m, a| {
        for (i, w) in (0u32..).zip(&a[..11]) {
            m.w32(SCRATCH + 0x3_0000 + 4 * i, *w);
        }
        SCRATCH
    });
    t.stub(717, |_, _| 0);
    t.stub(804, |_, _| 0);
    let f = t.at(0x1660);
    let ctrl = t.img.word(f + 0x13);
    let flags = t.img.word(f + 0x22);
    let slots = t.img.word(f + 0x28);
    // the set-up's tables hold old values it must clear
    let setup = t.at(0x1000);
    for (op, len) in [(0x71, 0x100), (0x76, 0x2000), (0xa8, 0x100), (0xad, 0x30)] {
        let at = t.img.word(setup + op);
        t.write(at, &vec![0xa5; len]);
    }
    for (flag, word, data) in [
        (0u8, 0u16, 0u32),
        (1, 0x10, 0x00da_7a00),
        (1, 0x0f, 0),
        (1, 0x1234, 0x00d0_0000),
    ] {
        let c = t.u32(ctrl);
        t.set_u16(c + 0x40, word);
        t.set_u32(c + 0xa8, data);
        let fl = t.u32(flags);
        t.set_u8(fl + 0x10, flag);
        let s = t.u32(slots);
        t.set_u32(s + 4, 0);
        t.check_entry(
            f,
            Call {
                eax: 0,
                ecx: 0x77,
                edx: 0,
                ..Call::default()
            },
            &[0x00ab_cdef],
            Eax::Checked,
        );
        t.check_entry(t.at(0x1820), Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let Some(mut t) = Twin::load_unstarted(N) else {
        return;
    };
    t.dll_main_check(
        &[
            0x10d0, 0x10e0, 0x1160, 0x11f0, 0x1250, 0x1380, 0x1440, 0x14b0, 0x14d0, 0x1530, 0x15a0,
            0x1600, 0x1660, 0x1710, 0x1820, 0x1860, 0x1880,
        ],
        &[(0x1000, 206), (0x2230, 53), (0x2270, 28)],
    );
}

/// Every stub of the part callbacks: records all the words it is given
/// into its own scratch row (so a dropped or reordered word shows) and
/// answers something the port must pass on.
fn part_stubs(t: &mut Twin) {
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
                // the program list: SCRATCH + 0x8000 holds 4 words
                941 => SCRATCH + 0x8000,
                // the effect's entry: the part's data for i == 1
                719 => {
                    if a[2] == 1 {
                        SCRATCH + 0x9000
                    } else {
                        0x0abc_0000 + a[2]
                    }
                }
                // done every other run
                712 => n & 1,
                _ => 0x7100 + slot,
            }
        });
    }
}

const PART: u32 = SCRATCH + 0xa000;

fn part(t: &mut Twin, key: (u8, u16), state: u8, buf: u32) {
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

#[test]
fn opu_part_callbacks() {
    let Some(mut t) = twin() else { return };
    part_stubs(&mut t);
    let alloc = t.at(0x1250);
    let parts = t.img.word(alloc + 0x76);
    let bufs = t.img.word(alloc + 0xab);
    // parts: some keys, some free slots, the part's own key at 5
    for i in 0..0x40u32 {
        let v = if i % 3 == 0 { 0 } else { 0x0055_0000 + i };
        t.set_u32(parts + 4 * i, v);
        t.set_u16(bufs + 0x80 * i + 0x1e, u16::try_from(i).unwrap());
    }
    t.set_u32(parts + 4 * 5, 0x0007_0042);
    // the game flag the OPU calls raise holds a value to restore
    let lock = t.u32(globals(&t).lock);
    t.set_u32(lock, 0x0bad);
    for key in [(7u8, 0x42u16), (9, 0x99)] {
        part(&mut t, key, 3, 0);
        t.check_entry(alloc, Call::default(), &[PART], Eax::Checked);
    }
    // every slot taken and no match: slot 64, past the table
    for i in 0..0x40u32 {
        t.set_u32(parts + 4 * i, 0x0066_0000 + i);
    }
    part(&mut t, (1, 1), 3, 0);
    t.check_entry(alloc, Call::default(), &[PART], Eax::Checked);
    // run, twice (done on the second), free, the others
    for _ in 0..2 {
        part(&mut t, (7, 0x42), 3, bufs + 0x80 * 5);
        t.check_entry(t.at(0x1380), Call::default(), &[PART], Eax::Checked);
    }
    for (state, buf) in [(3u8, bufs + 0x80 * 9), (2, bufs), (3, 0)] {
        part(&mut t, (7, 0x42), state, buf);
        t.check_entry(
            t.at(0x1530),
            Call {
                eax: 0x1234,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[PART],
            Eax::Checked,
        );
    }
    let prev = t.img.word(t.at(0x1440) + 0x0e);
    for i in 0..0x40u32 {
        t.set_u32(prev + 4 * i, 0x0abc_0000 + (i % 5));
        t.set_u32(parts + 4 * i, if i % 4 == 0 { 0 } else { i });
    }
    // forget: entries 0..5 of the effect at SCRATCH + 0x9000
    t.set_u16(SCRATCH + 0x9000 + 0x50, 5);
    t.check_entry(
        t.at(0x15a0),
        Call::default(),
        &[SCRATCH + 0x9000],
        Eax::Checked,
    );
    t.set_u16(SCRATCH + 0x9000 + 0x50, 0);
    t.check_entry(
        t.at(0x15a0),
        Call::default(),
        &[SCRATCH + 0x9000],
        Eax::Checked,
    );
    t.check_entry(t.at(0x1440), Call::default(), &[], Eax::Checked);
    t.check_entry(
        t.at(0x14b0),
        Call {
            eax: 0x77,
            ecx: 0,
            edx: 0,
            ..Call::default()
        },
        &[],
        Eax::Checked,
    );
    for i in 0..0x40u32 {
        t.set_u32(parts + 4 * i, if i % 7 == 0 { i } else { 0 });
        t.set_u32(prev + 4 * i, 0x0def_0000 + i);
    }
    t.check_entry(t.at(0x14d0), Call::default(), &[], Eax::Checked);
    t.check_entry(t.at(0x14d0), Call::default(), &[], Eax::Checked);
}

#[test]
fn opu_init_data_and_env() {
    let Some(mut t) = twin() else { return };
    part_stubs(&mut t);
    let f = t.at(0x1710);
    let slots = t.u32(t.img.word(f + 0x5d));
    let prev = t.img.word(f + 0xaf);
    let effect = SCRATCH + 0xc000;
    for (count, prev1) in [(3u16, 0u32), (3, 5), (1, 0), (0, 0)] {
        part(&mut t, (1, 2), 3, 0);
        t.set_u32(slots + 4, effect);
        t.set_u16(effect + 0x50, count);
        t.set_u32(prev + 4, prev1);
        t.check_entry(f, Call::default(), &[PART], Eax::Checked);
    }
    t.set_u32(slots + 4, 0);
    t.check_entry(f, Call::default(), &[PART], Eax::Checked);
    let e = t.at(0x1600);
    let m0 = t.img.word(e + 0x05);
    let flags = t.u32(t.img.word(e + 0x30));
    t.set_u32(SCRATCH + 0xd0a0, 0x1111);
    t.set_u32(SCRATCH + 0xd0a4, 0x2222);
    for (m, flag) in [(0u32, 0u8), (0, 1), (5, 1)] {
        t.set_u32(m0, m);
        t.set_u8(flags + 0x10, flag);
        t.check_entry(
            e,
            Call {
                eax: 0x99,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[SCRATCH + 0xd000],
            Eax::Checked,
        );
    }
}
