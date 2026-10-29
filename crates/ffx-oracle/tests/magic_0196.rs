//! `magic_0196.dll` against its Rust ports (`ffx_magic::m0196`): the effect
//! wrappers at 0x1040, 0x1090, 0x10f0, 0x1140 and 0x11d0, and 0x1180 (the
//! wrapper `magic_0194` carries too). Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};

const N: u16 = 196;

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// The arguments each slot's callee reads: the words the wrappers push for
/// its call (the words below them are the caller's, which no callee reads).
fn arity(slot: u32) -> usize {
    match slot {
        432 | 472 => 0,
        433 | 444 => 1,
        442 => 3,
        436 => 9,
        440 => 4,
        473 => 5,
        _ => 2,
    }
}

/// Every slot the wrappers call answers with `0x7000 + slot` and writes the
/// words its callee reads into its own scratch row (so a dropped or
/// reordered word shows in the memory comparison).
fn record(t: &mut Twin) {
    for s in 432..474 {
        let row = SCRATCH + 0x1_0000 + 0x100 * (s - 432);
        let n = arity(s);
        t.stub(s, move |m: &mut dyn StubMem, a: &[u32]| {
            for (i, w) in (0u32..).zip(a.iter().take(n)) {
                m.w32(row + 4 * i, *w);
            }
            0x7000 + s
        });
    }
}

#[test]
fn wrappers_pass_their_callbacks_and_the_argument() {
    let Some(mut t) = twin() else { return };
    record(&mut t);
    let obj = SCRATCH + 0x4000;
    // the context global (0x10085a50 in this DLL) points to a block whose
    // word at +0x1a0 the wrappers pass on
    let ctx_global = t.img.word(t.at(0x11d0) + 0x04);
    let ctx = SCRATCH + 0x3000;
    t.set_u32(ctx_global, ctx);
    t.set_u32(ctx + 0x1a0, 0x01a0_0001);
    let entry = Call {
        eax: 0x55,
        ecx: 3,
        edx: 4,
    };
    t.check_entry(t.at(0x1040), entry, &[obj], Eax::Checked);
    t.check_entry(t.at(0x1090), entry, &[obj], Eax::Checked);
    t.check_entry(t.at(0x10f0), entry, &[obj], Eax::Checked);
    t.check_entry(t.at(0x1140), entry, &[obj], Eax::Checked);
    t.check_entry(t.at(0x1180), entry, &[obj], Eax::Checked);
    // copies of other ports: 0x1000 (the wrapper of m0187's 0x1210) and
    // 0x1280 (the effect of m0187's 0x1420, with its own context operand)
    t.check_entry(t.at(0x1000), entry, &[obj], Eax::Checked);
    let ctx_280 = t.img.word(t.at(0x1280) + 0x39);
    t.set_u32(ctx_280, SCRATCH + 0x3000);
    t.check_entry(t.at(0x1280), entry, &[obj], Eax::Checked);
    // the context's word changes between the calls, and so does the context
    t.set_u32(ctx + 0x1a0, 0x0bad_f00d);
    t.check_entry(t.at(0x11d0), entry, &[obj], Eax::Checked);
    t.set_u32(ctx_global, SCRATCH + 0x3800);
    t.set_u32(SCRATCH + 0x3800 + 0x1a0, 0x0000_0042);
    t.check_entry(t.at(0x11d0), entry, &[obj], Eax::Checked);
}
