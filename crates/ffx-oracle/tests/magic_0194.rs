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

/// Every slot the wrapper and the copies of other ports call answers with
/// `0x7000 + slot` and writes the words its callee reads (432 takes none, 433
/// one, 434 and 449/450 two, 436 nine) into its own scratch row.
fn record(t: &mut Twin) {
    for (s, n) in [
        (432u32, 0usize),
        (433, 1),
        (434, 2),
        (436, 9),
        (449, 2),
        (450, 2),
    ] {
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
            Call {
                eax,
                ecx,
                edx: 9,
                ..Call::default()
            },
            &[obj],
            Eax::Checked,
        );
    }
    // copies of other ports: 0x1050 (the wrapper of m0187's 0x1050) and 0x16d0
    // (the effect of m0187's 0x1420, with its own context operand)
    let entry = Call {
        eax: 0x55,
        ecx: 3,
        edx: 4,
        ..Call::default()
    };
    t.check_entry(t.at(0x1050), entry, &[obj], Eax::Checked);
    let ctx = t.img.word(t.at(0x16d0) + 0x39);
    t.set_u32(ctx, SCRATCH + 0x3000);
    t.check_entry(t.at(0x16d0), entry, &[obj], Eax::Checked);
}

/// 0x10001280: the object start (own row).
#[test]
fn object_start_own() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let at = t.at(0x1280);
    let ctx = SCRATCH + 0x800;
    t.set_u32(t.img.word(at + 0x04), ctx);
    t.set_u32(ctx + 0x198, 0x0198_0000);
    for (k, (s, n)) in (0u32..).zip([(441u32, 3usize), (440, 4), (450, 2), (449, 2)]) {
        t.stub(s, move |m: &mut dyn StubMem, a: &[u32]| {
            for (i, v) in (0u32..).zip(&a[..n]) {
                m.w32(SCRATCH + 0x3000 + 0x40 * k + 4 * i, *v);
            }
            0x0100_0000 + s
        });
    }
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x22; 0x140]);
    t.check_entry(at, Call::default(), &[obj], Eax::Checked);
}
