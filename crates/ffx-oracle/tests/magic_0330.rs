//! `magic_0330.dll` against its Rust ports (`ffx_magic::m0330`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::m0330;
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 330;

/// 0x10005db0 (internal, `ecx` = the group): no models and three, with
/// and without a parent.
#[test]
fn model_nodes() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5db0);
    let models = SCRATCH + 0x700;
    t.write(models, &[3, 0, 5, 9]);
    rec(&mut t, 412, 3, move |m, a| {
        m.w32(a[1], m.r32(SCRATCH + 0x6f8));
        models
    });
    for (s, n) in [(215u32, 1usize), (413, 1), (14, 4)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
    let grp = SCRATCH + 0x100;
    let node = SCRATCH + 0x1_0000;
    let parent = SCRATCH + 0x800;
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for (count, with_parent) in [(0u32, false), (3, true), (2, false)] {
        t.write(node, &[0x5c; 0x40]);
        t.write(parent, &[0x3c; 0x20]);
        t.set_u16(parent + 0x18, 5);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(SCRATCH + 0x6f8, count);
        for (k, v) in (0u32..).zip([
            node,
            if with_parent { parent } else { 0 },
            0x00ac_0008,
            0x00c0_000c,
            0x0000_0101,
        ]) {
            t.set_u32(grp + 4 * k, v);
        }
        let c = Call {
            ecx: grp,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, m0330::model_nodes);
    }
}
