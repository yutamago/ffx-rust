//! `magic_0387.dll` against its Rust ports (`ffx_magic::m0387`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 387;

/// `n` bytes, each byte other than its neighbours (`seed` makes runs
/// differ), so that a read of a neighbouring word gives another value.
fn pattern(seed: u8, n: usize) -> Vec<u8> {
    (0u8..=255)
        .cycle()
        .take(n)
        .map(|b| b.wrapping_mul(0x1d) ^ seed)
        .collect()
}

/// The node under no parent, a parent with no child and one with a child.
fn cases(
    rva: u32,
    port: fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> ffx_magic::Ret,
) {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(rva);
    let r = SCRATCH + 0x100;
    let node = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x2000;
    let kid = SCRATCH + 0x2800;
    for case in 0..3 {
        t.write(node, &[0x5c; 0x400]);
        t.write(parent, &[0x3c; 0x40]);
        t.set_u16(parent + 0x18, 5);
        t.write(kid, &[0x2c; 0x40]);
        t.set_u32(parent + 0x10, if case == 2 { kid } else { parent });
        t.write(r, &[0x7a; 0x20]);
        t.set_u32(r, node);
        t.set_u32(r + 4, if case == 0 { 0 } else { parent });
        t.set_u32(r + 8, 0x00a0_0008);
        t.set_u32(r + 0xc, 0x00b0_000c);
        t.set_u32(r + 0x10, 0x00c0_0010);
        t.check(at, Call::this(r), &[], Eax::Checked, port);
    }
}

/// 0x10003c70.
#[test]
fn sprite_record() {
    cases(0x3c70, ffx_magic::m0387::sprite_record);
}

/// 0x100048b0.
#[test]
fn strip_record() {
    cases(0x48b0, ffx_magic::m0387::strip_record);
}

/// 0x10008a70 (internal: `ecx` = the sprite, `edx` = a vector, one stack
/// word): two sets of halfwords.
#[test]
fn sprite_size() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x8a70);
    magic_owna::rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x325);
        }
        0x8325_0000
    });
    let sp = SCRATCH + 0x1000;
    let v = SCRATCH + 0x200;
    for (k, base) in [(0u16, 0u16), (1, 0x7fff)] {
        // the flags word +0xe8 other than its neighbours
        t.write(sp, &pattern(0x5c ^ u8::try_from(k).unwrap(), 0x100));
        for i in 0..4u32 {
            t.set_u16(
                sp + 0xd0 + 2 * i,
                base.wrapping_add(k * 300 + u16::try_from(i).unwrap() * 77),
            );
        }
        t.write(v, &[0x11; 0x10]);
        let c = Call {
            ecx: sp,
            edx: v,
            ..Call::default()
        };
        t.check(
            at,
            c,
            &[0xabcd_0000 + u32::from(k)],
            Eax::Checked,
            ffx_magic::m0387::sprite_size,
        );
    }
}
