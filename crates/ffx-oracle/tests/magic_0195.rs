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
    };
    t.check_entry(t.at(0x1220), entry, &[obj], Eax::Checked);
}
