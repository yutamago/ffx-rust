//! `magic_0193.dll` against the ports bound to three of its functions: the
//! wrapper at 0x1050 (`ffx_magic::m0187`'s 0x1050 port), the wrapper at 0x1610
//! (`m0194`'s 0x1230 port) and the effect at 0x1d00 (`m0187`'s 0x1420 port).
//! Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};

const N: u16 = 193;

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// The words each slot's callee reads (the words the functions push for it).
const SLOTS: [(u32, usize); 6] = [(432, 0), (433, 1), (434, 2), (436, 9), (449, 2), (450, 2)];

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
fn copies_of_the_bound_ports() {
    let Some(mut t) = twin() else { return };
    record(&mut t);
    let obj = SCRATCH + 0x6000;
    let entry = Call {
        eax: 0x55,
        ecx: 3,
        edx: 4,
        ..Call::default()
    };
    t.check_entry(t.at(0x1050), entry, &[obj], Eax::Checked);
    t.check_entry(t.at(0x1610), entry, &[obj], Eax::Checked);
    // the effect reads the context global its operand names (+0x39)
    let ctx_global = t.img.word(t.at(0x1d00) + 0x39);
    t.set_u32(ctx_global, SCRATCH + 0x3000);
    t.check_entry(t.at(0x1d00), entry, &[obj], Eax::Checked);
}
