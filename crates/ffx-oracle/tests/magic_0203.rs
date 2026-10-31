//! `magic_0203.dll` against its Rust port (`ffx_magic::m0203`): a
//! node's set-up and a task's release. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

const N: u16 = 203;

/// 0x10007660: a node set up into an empty list and after another.
#[test]
fn node_setup() {
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x7660);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let root = w(&t, 0x31);
    let old = magic::SCRATCH + 0x800;
    for empty in [true, false] {
        t.set_u32(w(&t, 0x24), if empty { root } else { old });
        t.write(old, &[0x21; 0x30]);
        let n = magic::SCRATCH + 0x100;
        t.write(n, &[0x7e; 0x30]);
        t.check_entry(
            at,
            magic::Call::this(n),
            &[0x00a0_00a0, 0x00b0_00b0],
            magic::Eax::Checked,
        );
    }
}

/// 0x100078c0: busy, slot 411 saying 0, and released.
#[test]
fn task_release() {
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x78c0);
    let live = magic::SCRATCH + 0x600;
    magic_owna::rec(&mut t, 411, 1, move |m, _| m.r32(live));
    for s in [228u32, 213] {
        magic_owna::rec(&mut t, s, 2, move |_, a| a[0] ^ (s << 16));
    }
    let (task, rec, part) = (
        magic::SCRATCH + 0x100,
        magic::SCRATCH + 0x200,
        magic::SCRATCH + 0x300,
    );
    let (node, next, prev) = (
        magic::SCRATCH + 0x400,
        magic::SCRATCH + 0x440,
        magic::SCRATCH + 0x480,
    );
    for (busy, alive) in [(1u32, 1u32), (0, 0), (0, 1)] {
        t.set_u32(task, node);
        t.set_u32(task + 4, rec);
        t.set_u32(rec + 0x1c, busy);
        t.set_u32(rec + 0x18, part);
        t.set_u32(part + 0x50, 0x0050_0050);
        t.set_u32(live, alive);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.write(next, &[0x11; 0x10]);
        t.write(prev, &[0x22; 0x10]);
        let c = magic::Call {
            eax: 0x00ea_0000,
            ..magic::Call::default()
        };
        t.check_entry(at, c, &[task], magic::Eax::Checked);
    }
}
