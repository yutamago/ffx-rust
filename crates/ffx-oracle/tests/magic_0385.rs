//! `magic_0385.dll` against its Rust port (`ffx_magic::m0385`): the
//! task step. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 385;

/// 0x10004aa0: the motion and objects done or not, the count above 0,
/// each bit 15 clear, and ready.
#[test]
fn step_ready() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4aa0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let ms = t.img.rel32_target(at + 0x05);
    let ro = t.img.rel32_target(at + 0x0c);
    let list = t.img.word(ro + 0x7c);
    let (node, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(node + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let (pa, pb) = (SCRATCH + 0x500, SCRATCH + 0x580);
    t.set_u32(w(&t, 0x23), pa);
    t.set_u32(w(&t, 0x33), pb);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    for (done, busy, count, bp, bq) in [
        (true, false, 0i32, true, true),
        (false, false, 0, true, true),
        (true, true, 0, true, true),
        (true, false, 1, true, true),
        (true, false, -3, false, true),
        (true, false, 0, true, false),
    ] {
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(list + 8, if busy { node } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { node } else { 0 });
        t.set_u32(w(&t, 0x1b), count.cast_unsigned());
        t.set_u16(pa + 0x2c, if bp { 0x8001 } else { 0x7fff });
        t.set_u16(pb + 0x18, if bq { 0x8002 } else { 0x7ffe });
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
