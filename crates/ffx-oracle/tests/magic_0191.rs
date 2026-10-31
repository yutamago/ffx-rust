//! `magic_0191.dll` against the ports bound to two of its functions: the wrapper
//! at 0x1060 (`ffx_magic::m0187`'s 0x1050 port) and the effect at 0x1ca0 (its
//! 0x1420 port). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};

const N: u16 = 191;

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
    t.check_entry(t.at(0x1060), entry, &[obj], Eax::Checked);
    // the effect reads the context global its operand names (+0x39)
    let ctx_global = t.img.word(t.at(0x1ca0) + 0x39);
    t.set_u32(ctx_global, SCRATCH + 0x3000);
    t.check_entry(t.at(0x1ca0), entry, &[obj], Eax::Checked);
}

/// 0x100011b0: an object set up with two list positions.
#[test]
fn object_setup() {
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x11b0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let o = SCRATCH + 0x400;
    magic_owna::rec(&mut t, 434, 2, move |_, _| o);
    magic_owna::rec(&mut t, 450, 2, |_, a| a[1] ^ 0x450);
    magic_owna::rec(&mut t, 449, 2, |_, a| a[1] ^ 0x449);
    let a = SCRATCH + 0x800;
    t.set_u32(w(&t, 0x16), a);
    t.set_u32(a + 0x198, 0x0198_0000);
    t.set_u32(w(&t, 0x3c), 0x003c_0000);
    let obj = SCRATCH + 0x1000;
    for (i, j) in [(0u32, 1u32), (2, 0)] {
        t.write(o, &[0x11; 0x100]);
        t.set_u32(o + 0xac, i);
        t.write(obj, &[0x22; 0x100]);
        t.set_u32(obj + 0xac, j);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001000, 0x100010b0, 0x10001110, 0x10001160, 0x10001240 (below the
/// count, at it), 0x100012f0, 0x10001350: the object state chain.
#[test]
fn object_chain() {
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    for (s, n) in [
        (454u32, 1usize),
        (450, 2),
        (449, 2),
        (460, 4),
        (451, 2),
        (447, 2),
        (434, 2),
        (461, 2),
        (459, 3),
        (455, 1),
        (432, 1),
        (441, 3),
        (443, 2),
        (440, 4),
    ] {
        magic_owna::rec(&mut t, s, n, move |_, a| a[0] ^ (s << 16));
    }
    let start = t.at(0x1000);
    let e = magic::SCRATCH + 0x2000;
    let c = t.img.word(start + 0x0a);
    t.set_u32(t.img.word(start + 0x04), e);
    t.write(e + 0x180, &[0x33; 0x80]);
    t.set_u32(e + 0x198, 0x0198_0000);
    t.set_u32(e + 0x19c, 3);
    for i in 0..4 {
        t.set_u32(e + 0x1a0 + 4 * i, 0x01a0_0000 + i);
    }
    let obj = magic::SCRATCH + 0x100;
    for (at, count) in [
        (0x1000, 0u32),
        (0x10b0, 2),
        (0x1110, 0),
        (0x1160, 0),
        (0x1240, 0),
        (0x1240, 2),
        (0x1240, 7),
        (0x12f0, 0),
        (0x1350, 0),
    ] {
        t.write(obj, &[0x4d; 0x140]);
        t.set_u32(obj + 0xac, 3);
        t.set_u32(c, count);
        let at = t.at(at);
        t.check_entry(at, magic::Call::default(), &[obj], magic::Eax::Checked);
    }
}
