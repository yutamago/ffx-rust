//! `magic_0272.dll` against its Rust ports: the calling thunk at
//! 0x100032b0 over the summon family's vector operation
//! (`ffx_magic::m0272`, `ffx_magic::s3_wrap::ecx_call`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use magic::{Call, Eax};
use sa::{CURSOR, RINGS, handler_stubs, node, rec, ring};

const N: u16 = 272;

/// 0x100032b0: each operation byte, the owner's flag clear and set, on
/// every ring shape (the cursor moving or not).
#[test]
fn vector_op_through_the_calling_thunk() {
    let Some(mut t) = sa::twin(N) else { return };
    handler_stubs(&mut t);
    rec(&mut t, 262, 4, |_, _| 0x262);
    rec(&mut t, 29, 2, |_, _| 0x29);
    rec(&mut t, 268, 3, |_, _| 0x268);
    for s in [366, 665, 531] {
        rec(&mut t, s, 2, move |_, _| s);
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.3);
    let thunk = t.at(0x32b0);
    let owner = node(6);
    for (k, &(c, p, cp)) in RINGS.iter().enumerate() {
        for op in [0u8, 2, 3, 6, 0xff] {
            for oflags in [0u8, 1] {
                ring(&mut t, c, p, cp, 0x30 + k as u8);
                t.write(owner, &[0x44; 0x100]);
                t.set_u8(owner + 0x90, oflags);
                for (i, v) in (0u32..).zip([1.0f32, -2.5, 1.0e-3]) {
                    t.set_u32(node(0) + 0x1c + 4 * i, v.to_bits());
                    t.set_u32(node(0) + 0x2c + 4 * i, (v * 3.0).to_bits());
                }
                t.set_u32(node(0) + 0x38, owner);
                t.set_u32(node(0) + 0x3c, 0x0003_c3c3);
                t.set_u8(node(0) + 0x40, op);
                let c = Call {
                    eax: 0xaabb_ccdd,
                    ecx: 0x0bad_0ecc,
                    edx: 0x55,
                    ..Call::default()
                };
                t.check_entry(thunk, c, &[CURSOR, 0x0bad_f00e], Eax::Checked);
            }
        }
    }
}
