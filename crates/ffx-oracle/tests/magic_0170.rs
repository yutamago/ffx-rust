//! `magic_0170.dll` against its Rust port (`ffx_magic::m0170`): the start-up
//! at 0x10001690, with the OPU set-up it calls. Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};

const N: u16 = 170;

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// Writes the first `n` argument words a callee gets into scratch at `at`
/// (so a dropped or reordered word shows in the memory comparison).
fn log(m: &mut dyn StubMem, at: u32, a: &[u32], n: usize) {
    for (i, w) in (0u32..).zip(a.iter().take(n)) {
        m.w32(at + 4 * i, *w);
    }
}

#[test]
fn start_up_both_data_choices() {
    let Some(mut t) = twin() else { return };
    t.stub(803, |_, _| SCRATCH + 0x1_0000);
    t.stub(718, |m, a| {
        log(m, SCRATCH + 0x2_0000, a, 1);
        0
    });
    t.stub(173, |m, a| {
        log(m, SCRATCH + 0x2_0010, a, 2);
        0x173
    });
    t.stub(719, |m, a| {
        log(m, SCRATCH + 0x2_0020, a, 3);
        a[0].wrapping_add(0x10)
    });
    // slot 715 reads its eleven words
    t.stub(715, |m, a| {
        log(m, SCRATCH + 0x3_0000, a, 11);
        SCRATCH
    });
    t.stub(720, |m, a| {
        log(m, SCRATCH + 0x2_0030, a, 2);
        0
    });
    t.stub(717, |m, a| {
        log(m, SCRATCH + 0x2_0040, a, 1);
        0
    });
    t.stub(816, |m, a| {
        log(m, SCRATCH + 0x2_0050, a, 12);
        0x816
    });
    let start = t.at(0x1690);
    let setup = t.at(0x1000);
    // the set-up's tables hold old values it must clear
    for (op, len) in [(0x71, 0x100), (0x76, 0x2000), (0xa8, 0x100), (0xad, 0x30)] {
        let at = t.img.word(setup + op);
        t.write(at, &vec![0xa5; len]);
    }
    let ctrl = SCRATCH + 0x5000;
    let flags = SCRATCH + 0x5800;
    // the set-up's own globals: its flags block and its control block
    let setup_flags = t.img.word(setup + 0x24);
    let setup_ctrl = t.img.word(setup + 0x33);
    t.set_u32(setup_flags, flags);
    t.set_u32(setup_ctrl, ctrl);
    // the start's globals: the control block, the slots pointer, the flag
    // byte, the default data word
    t.set_u32(t.img.word(start + 0x1b), ctrl);
    t.set_u32(t.img.word(start + 0x21), SCRATCH + 0x6000);
    let flag_g = t.img.word(start + 0x3b);
    t.set_u32(t.img.word(start + 0x43), 0x00da_7a00);
    for (cb10, flag, word40, set_up_flag) in [
        (0u8, 0u8, 0u16, 0u8),
        (1, 0x10, 0x00, 0),
        // the set-up's slot 717 path: the control word has bit 4
        (1, 0x10, 0x10, 1),
        (1, 0x01, 0x10, 1),
        (0, 0x10, 0x10, 1),
        (1, 0x10, 0x10, 0),
    ] {
        t.set_u8(ctrl + 0x10, cb10);
        t.set_u16(ctrl + 0x40, word40);
        t.set_u8(flag_g, flag);
        t.set_u8(flags + 0x10, set_up_flag);
        t.check_entry(
            start,
            Call {
                eax: 0x1234,
                ecx: 0x77,
                edx: 0,
                ..Call::default()
            },
            &[0x00ab_cdef],
            Eax::Checked,
        );
    }
}
