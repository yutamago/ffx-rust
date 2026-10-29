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
            0x10d0, 0x10e0, 0x1160, 0x11f0, 0x1660, 0x1820, 0x1860, 0x1880,
        ],
        &[(0x1000, 206), (0x2230, 53), (0x2270, 28)],
    );
}
