//! `magic_0187.dll` against its Rust ports (`ffx_magic::m0187`): the effect
//! functions at 0x1000, 0x1050, 0x1170, 0x10a0, 0x10f0, 0x1210 and 0x1420,
//! and 0x11d0 (the wrapper `m0194` ports). Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};

const N: u16 = 187;

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// The words each slot's callee reads (the words the wrappers push for it).
const SLOTS: [(u32, usize); 9] = [
    (432, 0),
    (433, 1),
    (434, 2),
    (436, 9),
    (447, 2),
    (448, 2),
    (449, 2),
    (450, 2),
    (451, 2),
];

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
fn wrappers_pass_their_callbacks_and_the_argument() {
    let Some(mut t) = twin() else { return };
    record(&mut t);
    let obj = SCRATCH + 0x6000;
    // the context global (0x10069230) points to a block whose word at +0x198
    // the wrappers pass on; the object's word at +0xac sets the store
    let ctx_global = t.img.word(t.at(0x10a0) + 0x04);
    let ctx = SCRATCH + 0x3000;
    t.set_u32(ctx_global, ctx);
    t.set_u32(ctx + 0x198, 0x0198_0001);
    t.set_u32(obj + 0xac, 3);
    let entry = Call {
        eax: 0x55,
        ecx: 3,
        edx: 4,
        ..Call::default()
    };
    for rva in [0x1000, 0x1050, 0x10a0, 0x1170, 0x10f0, 0x1210, 0x11d0] {
        t.check_entry(t.at(rva), entry, &[obj], Eax::Checked);
    }
    t.check_entry(t.at(0x1420), entry, &[obj], Eax::Checked);
    // another value at the object's +0xac: a different store offset
    t.set_u32(obj + 0xac, 0x11);
    for rva in [0x1170, 0x10f0, 0x1420] {
        t.check_entry(t.at(rva), entry, &[obj], Eax::Checked);
    }
}

/// 0x10001360: the object set-up (own row).
#[test]
fn object_setup_own() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let at = t.at(0x1360);
    let ctx = SCRATCH + 0x800;
    t.set_u32(t.img.word(at + 0x04), ctx);
    t.set_u32(ctx + 0x1a0, 0x01a0_0000);
    for (k, (s, n)) in (0u32..).zip([
        (444u32, 1usize),
        (442, 3),
        (443, 2),
        (440, 4),
        (446, 2),
        (450, 2),
        (449, 2),
    ]) {
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
