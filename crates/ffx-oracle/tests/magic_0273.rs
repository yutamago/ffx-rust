//! `magic_0273.dll` against its Rust ports (`ffx_magic::m0273`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::m0273;
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 273;

/// 0x10007600: the static node into an empty list and after another.
#[test]
fn static_node() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7600);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let other = SCRATCH + 0x600;
    for last in [w(&t, 0x46), other] {
        for op in [0x07, 0x12, 0x1a, 0x24, 0x2e, 0x38, 0x41, 0x55, 0x5f] {
            t.set_u32(w(&t, op), 0x5a5a_5a5b);
        }
        t.write(other, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x01), last);
        t.set_u16(w(&t, 0x4f), 7);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10008170 (internal: one stack word and `edx`): the node at the
/// ring's tail.
#[test]
fn tail_node() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8170);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let tail = SCRATCH + 0x500;
    for (x, edx) in [(0x00a1_0001u32, 0x00ed_0001u32), (0, 0xffff_ffff)] {
        for op in [0x09, 0x0f, 0x19, 0x23, 0x2d, 0x41, 0x48, 0x51, 0x57, 0x5d] {
            t.set_u32(w(&t, op), 0x5a5a_5a5b);
        }
        t.write(tail, &[0x55; 0x10]);
        t.set_u32(w(&t, 0x04), tail);
        let c = Call {
            edx,
            ..Call::default()
        };
        t.check(at, c, &[x], Eax::Checked, m0273::tail_node);
    }
}

/// 0x10002a30 (internal): the sprite set up and already set up (bit 0).
#[test]
fn sprite_setup() {
    use magic::StubMem;
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2a30);
    t.stub(538, |_, a| 0x0538_0000 ^ a[0] ^ (a[1] << 8));
    t.stub(217, |m: &mut dyn StubMem, a: &[u32]| {
        let v = m.r32(a[1]);
        m.w32(SCRATCH + 0x700, v ^ a[0] ^ a[2]);
        0x217
    });
    let r = SCRATCH + 0x100;
    let sprite = SCRATCH + 0x1000;
    // the kind byte +0x10 with a non-zero byte above it
    for (k, v) in (0u32..).zip([sprite, 0xa, 0xb, 0xc, 0x0000_7799, 0xd]) {
        t.set_u32(r + 4 * k, v);
    }
    for flags in [0u16, 0xffff, 0x0400, 0xfffe] {
        // each word of the sprite other than its neighbours
        let fill: Vec<u8> = (0u8..0xa0).map(|b| b.wrapping_mul(0x1d) ^ 0x5c).collect();
        t.write(sprite, &fill);
        t.set_u16(sprite + 0x90, flags);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check(at, Call::this(r), &[], Eax::Checked, m0273::sprite_setup);
    }
}
