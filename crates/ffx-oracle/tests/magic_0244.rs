//! `magic_0244.dll` against its Rust ports (`ffx_magic::m0244`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Xmm, m0244};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 244;

/// 0x10001a10 (internal: `ecx` = the context, `edx` = the object,
/// `xmm3` = the scale, two stack words): into an empty and a filled
/// list, and with the pool empty.
#[test]
fn object_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a10);
    let list = SCRATCH + 0x700;
    t.set_u32(list, 0x0900_0000);
    rec(&mut t, 941, 1, move |_, _| list);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    // pppStartPart's lock and the global the context goes to
    let sp = t.img.rel32_target(at + 0x7d);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x600);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x640);
    // the pool (pool_alloc's free count and nodes)
    let pa = t.img.rel32_target(at + 0x92);
    let free = t.img.word(pa + 0x04);
    let ctx = SCRATCH + 0x100;
    let obj = SCRATCH + 0x1000;
    let old = SCRATCH + 0x2000;
    let model = SCRATCH + 0x800;
    for (last, nodes) in [(0u32, 5u16), (old, 5), (old, 0)] {
        t.write(ctx, &[0x3c; 0x40]);
        t.set_u32(ctx + 0x2c, model);
        t.set_u32(model + 0x10, 0x00de_0010);
        t.set_u32(ctx + 0x34, last);
        t.set_u32(ctx + 0x3c, 7);
        t.write(old, &[0x2c; 0x10]);
        t.write(obj, &[0x5c; 0x140]);
        t.set_u16(free, nodes);
        let c = Call {
            ecx: ctx,
            edx: obj,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(9.0), Xmm::f32(8.0), Xmm::f32(7.0), Xmm::f32(0.375)]);
        t.check(
            at,
            c,
            &[0x0bad_f00d, 0x00a1_0005],
            Eax::Checked,
            m0244::object_start,
        );
    }
}
