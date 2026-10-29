//! `magic_0078.dll` against its Rust port (`ffx_magic::m0078`). Needs
//! `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 78;

#[test]
fn clock_unlinks_the_task_when_due() {
    let Some(mut t) = Twin::load(N) else { return };
    t.init();
    let f = t.at(0x6470);
    let k0 = f32::from_bits(t.u32(t.img.word(f + 0x1e)));
    let k1 = f32::from_bits(t.u32(t.img.word(f + 0x2f)));
    eprintln!("K0 {k0} K1 {k1}");
    t.set_u32(t.img.word(f + 0x46), 0x00aa_0046);
    t.stub(239, |m, a| {
        for (i, w) in (0u32..).zip(&a[..3]) {
            m.w32(SCRATCH + 0x800 + 4 * i, *w);
        }
        0x0239
    });
    let task = SCRATCH;
    let obj = SCRATCH + 0x1000;
    let node = SCRATCH + 0x100;
    let (prev, next) = (SCRATCH + 0x200, SCRATCH + 0x300);
    for (held, clock) in [
        (0u16, k1 - k0),
        (0, k1 - k0 - 1.0),
        (1, k1),
        (0, k1 + 5.0),
        (0, f32::NAN),
        (0, k1 - 2.0 * k0),
        (0x100, -1.0),
        (0x100, k1 + 1.0),
    ] {
        t.set_u32(task, node);
        t.set_u32(task + 4, obj);
        t.set_u16(obj + 0x2c8, held);
        t.set_u32(obj + 0x2c4, clock.to_bits());
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.write(prev, &[0x11; 0x10]);
        t.write(next, &[0x22; 0x10]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}
