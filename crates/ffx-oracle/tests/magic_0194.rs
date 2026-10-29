//! `magic_0194.dll` against its Rust port (`ffx_magic::m0194`): the effect
//! wrapper at 0x1230, which seven overlays carry. Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};

const N: u16 = 194;

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// Every slot the wrapper calls answers with `0x7000 + slot` and writes the
/// words its callee reads (432 takes none, 449 and 450 two) into its own
/// scratch row.
fn record(t: &mut Twin) {
    for (s, n) in [(432u32, 0usize), (449, 2), (450, 2)] {
        let row = SCRATCH + 0x1_0000 + 0x100 * (s - 432);
        t.stub(s, move |m: &mut dyn StubMem, a: &[u32]| {
            for (i, w) in (0u32..).zip(a.iter().take(n)) {
                m.w32(row + 4 * i, *w);
            }
            0x7000 + s
        });
    }
}

#[test]
fn wrapper_passes_its_callbacks_and_the_argument() {
    let Some(mut t) = twin() else { return };
    record(&mut t);
    let obj = SCRATCH + 0x2000;
    for (eax, ecx) in [(0u32, 0u32), (0x1234, 0x77)] {
        t.check_entry(
            t.at(0x1230),
            Call { eax, ecx, edx: 9 },
            &[obj],
            Eax::Checked,
        );
    }
}
