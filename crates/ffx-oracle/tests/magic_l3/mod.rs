//! Shared parity checks for list 3's overlays (docs/formats/magic.md,
//! "Porting the overlays"): the DLL's `DllMain` against every port bound
//! in it, and the OPU start-up family.
#![allow(dead_code)]

use crate::magic::{Call, Eax, SCRATCH, Twin};

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
pub fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// The RVAs where a port binds (every 16-byte boundary of the code) and
/// the internal functions (RVA, length) the ports reproduce.
pub fn ported(t: &Twin) -> (Vec<u32>, Vec<(u32, u32)>) {
    let (t0, t1) = t.img.text;
    let mut entries = Vec::new();
    let mut internal = Vec::new();
    for rva in (t0..t1).step_by(16) {
        let a = t.at(rva);
        if ffx_magic::bind(&t.img, a).is_some() {
            entries.push(rva);
        } else if let Some((_, sig)) = ffx_magic::INTERNAL
            .iter()
            .find(|(_, sig)| t.img.matches(a, sig))
        {
            internal.push((rva, sig.len));
        }
    }
    (entries, internal)
}

/// `DllMain` attach and detach write only CRT words no port reads.
pub fn dll_main(n: u16) {
    let Some(mut t) = Twin::load_unstarted(n) else {
        return;
    };
    let (entries, internal) = ported(&t);
    t.dll_main_check(&entries, &internal);
}

/// Every word the start-up's slots see is recorded in its own scratch
/// row; the slots answer real-looking pointers: the allocation (803),
/// the effect entry (719) and the OPU object (715, at SCRATCH).
fn start_up_stubs(t: &mut Twin, n715: usize) {
    t.stub(803, |m, a| {
        m.w32(SCRATCH + 0x3_1000, a[0]);
        m.w32(SCRATCH + 0x3_1004, a[1]);
        SCRATCH + 0x1_0000
    });
    t.stub(718, |m, a| {
        m.w32(SCRATCH + 0x3_1010, a[0]);
        0x7180
    });
    t.stub(173, |m, a| {
        for (i, w) in (0u32..).zip(&a[..3]) {
            m.w32(SCRATCH + 0x3_1020 + 4 * i, *w);
        }
        0x1730
    });
    t.stub(719, |m, a| {
        for (i, w) in (0u32..).zip(&a[..3]) {
            m.w32(SCRATCH + 0x3_1040 + 4 * i, *w);
        }
        a[0] + 0x10
    });
    t.stub(715, move |m, a| {
        for (i, w) in (0u32..).zip(&a[..n715]) {
            m.w32(SCRATCH + 0x3_0000 + 4 * i, *w);
        }
        SCRATCH
    });
    t.stub(717, |m, a| {
        m.w32(SCRATCH + 0x3_1060, a[0]);
        0
    });
    t.stub(720, |m, a| {
        m.w32(SCRATCH + 0x3_1070, a[0]);
        m.w32(SCRATCH + 0x3_1074, a[1]);
        0
    });
    t.stub(804, |m, a| {
        m.w32(SCRATCH + 0x3_1080, a[0]);
        0
    });
    t.stub(816, |m, a| {
        m.w32(SCRATCH + 0x3_1090, a[0]);
        0x8160
    });
    t.stub(817, |_, _| 0x8170);
}

/// The OPU start-up with the control block in the DLL's data (`f` its
/// address; operands: flags `+0x1b`, slots `+0x21`, the control byte
/// `+0x3b`, its data word `+0x43`, the set-up's `rel32` at `setup_call`)
/// over every branch, then the shutdown at `shutdown` (when the DLL has
/// one) after each.
pub fn start_up(t: &mut Twin, f: u32, setup_call: u32, n715: usize, shutdown: Option<u32>) {
    start_up_stubs(t, n715);
    let flags = t.img.word(f + 0x1b);
    let slots = t.img.word(f + 0x21);
    let ctrl = t.img.word(f + 0x3b);
    let alt = t.img.word(f + 0x43);
    let setup = t.img.rel32_target(f + setup_call);
    for (flag, byte, data) in [
        (0u8, 0u8, 0u32),
        (0, 0x10, 0x00da_7a00),
        (1, 0x10, 0x00da_7a00),
        (1, 0x0f, 0x00d0_0000),
        (1, 0xff, 0),
        (2, 0x30, 0x00d0_0010),
    ] {
        // the set-up's tables hold old values it must clear
        for (op, len) in [(0x71, 0x100), (0x76, 0x2000), (0xa8, 0x100), (0xad, 0x30)] {
            let at = t.img.word(setup + op);
            t.write(at, &vec![0xa5; len]);
        }
        t.set_u8(ctrl, byte);
        t.set_u32(alt, data);
        let fl = t.u32(flags);
        t.set_u8(fl + 0x10, flag);
        let s = t.u32(slots);
        t.set_u32(s + 4, 0x5555);
        t.check_entry(
            f,
            Call {
                eax: 0x1234,
                ecx: 0x77,
                edx: 0x99,
            },
            &[0x00ab_cdef, 0x0102_0304],
            Eax::Checked,
        );
        if let Some(s) = shutdown {
            t.check_entry(s, Call::default(), &[], Eax::Checked);
        }
    }
}
