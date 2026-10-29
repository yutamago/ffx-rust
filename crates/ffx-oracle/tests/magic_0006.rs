//! `magic_0006.dll` against its Rust port: the follow handler of the
//! `magic_0003` family (`ffx_magic::m0003::follow`). Needs
//! `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, TRAPS, Twin};

const N: u16 = 6;
const HANDLER_SLOT: u32 = 0;
const CURSOR: u32 = SCRATCH;
const TABLE: u32 = SCRATCH + 0x100;
const NODE: u32 = SCRATCH + 0x1000;
const CHILD: u32 = SCRATCH + 0x1100;

fn floats(t: &mut Twin, at: u32, v: &[f32]) {
    for (i, x) in (0u32..).zip(v) {
        t.set_u32(at + 4 * i, x.to_bits());
    }
}

#[test]
fn follow_moves_the_node_and_reports_it() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    t.stub(HANDLER_SLOT, |m, a| {
        m.w32(SCRATCH + 0x800, m.r32(a[0]));
        0x4455_6677
    });
    t.stub(791, |m, a| {
        for (i, w) in (0u32..).zip(&a[..3]) {
            m.w32(SCRATCH + 0x900 + 4 * i, *w);
        }
        0x0791_0000
    });
    let f = t.at(0x1190);
    // (follow flag +0x54, clock +0x50, rate, pass, cursor pass, a child)
    let cases: [(u32, f32, f32, u8, u8, bool); 8] = [
        (1, 0.0, 0.25, 1, 2, false),
        (1, 0.0, 0.5, 1, 2, true),
        (0, 0.5, 0.125, 3, 4, false),
        (0, 2.0, 0.125, 3, 4, false),
        (0, 1.0, 0.5, 3, 4, true),
        (0, f32::NAN, 0.5, 3, 4, false),
        (0, -7.0, 3.0, 3, 4, false),
        (1, 0.0, 1.0, 5, 5, false),
    ];
    for (follow, clock, rate, pass, cpass, child) in cases {
        t.write(NODE, &[0; 0x70]);
        t.set_u32(NODE, TABLE);
        t.set_u32(TABLE + 0x1c, TRAPS + 16 * HANDLER_SLOT);
        t.set_u8(NODE + 0x1a, pass);
        if child {
            t.set_u32(NODE + 0x10, CHILD);
            t.set_u32(CHILD, TABLE);
            t.set_u32(CHILD + 8, NODE);
        } else {
            t.set_u32(NODE + 0x10, NODE);
        }
        floats(&mut t, NODE + 0x1c, &[1.0, -2.5, 3.75, 100.0]);
        floats(&mut t, NODE + 0x2c, &[4.0, 0.1, -3.0, 1.0e-3]);
        floats(&mut t, NODE + 0x3c, &[0.5, -0.25, 7.0, 1.0e7]);
        floats(&mut t, NODE + 0x4c, &[rate, clock]);
        t.set_u32(NODE + 0x54, follow);
        floats(&mut t, NODE + 0x5c, &[10.0, 20.0, -30.0]);
        t.set_u32(CURSOR, NODE);
        t.set_u8(CURSOR + 4, 0x7e);
        t.set_u8(CURSOR + 5, cpass);
        t.check_entry(
            f,
            Call {
                eax: 0x1122_3344,
                ecx: 0,
                edx: 0,
                ..Call::default()
            },
            &[CURSOR],
            Eax::Checked,
        );
    }
}
