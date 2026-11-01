//! `magic_0253.dll` against its Rust ports (`ffx_magic::m0253`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

const N: u16 = 253;

/// 0x10001340: the script and the objects running or done, the
/// halfword set and clear, the pool node there or not.
#[test]
fn frame() {
    use magic::{SCRATCH, Twin};
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x1340);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(213u32, 2usize), (238, 2), (719, 3), (306, 3)] {
        magic_owna::rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let sr = t.img.rel32_target(at + 0x05);
    let ro = t.img.rel32_target(at + 0x0c);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    let node = SCRATCH + 0x3000;
    for (ended, busy, half, with_node) in [
        (true, false, 0u16, true),
        (true, false, 1, false),
        (false, false, 0, true),
        (true, true, 0, true),
    ] {
        // the script's mode byte (0 ended, 2 waiting for ever), no
        // callback after
        t.set_u8(t.img.word(sr + 0x01), if ended { 0 } else { 2 });
        t.set_u16(t.img.word(sr + 0x59), 0xffff);
        t.set_u32(t.img.word(sr + 0x91), 0);
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        // the halfword's neighbour junk (the test reads 16 bits)
        t.set_u32(w(&t, 0x1c), 0x5a5a_0000 | u32::from(half));
        t.set_u32(w(&t, 0x2b), 0x00ab_002b);
        t.set_u32(w(&t, 0x43), 0x00ab_0043);
        t.write(node, &[0x4c; 0x20]);
        t.set_u32(w(&t, 0x5b), if with_node { node } else { 0 });
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check_entry(at, magic::Call::default(), &[task], magic::Eax::Checked);
    }
}

/// 0x10001930: the two part slots of records 0 and 3.
#[test]
fn part_slots() {
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x1930);
    for i in [0u32, 3] {
        t.set_u32(t.img.word(at + 0x01), i);
        t.check_entry(at, magic::Call::default(), &[], magic::Eax::Checked);
    }
}
