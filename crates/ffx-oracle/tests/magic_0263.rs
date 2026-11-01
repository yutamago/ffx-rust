//! `magic_0263.dll` against its Rust port (`ffx_magic::m0263`): a
//! node's set-up. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::twin;

const N: u16 = 263;

/// 0x10006dc0: a node set up into an empty list and after another.
#[test]
fn node_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6dc0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let root = w(&t, 0x2b);
    let old = SCRATCH + 0x800;
    for empty in [true, false] {
        t.set_u32(w(&t, 0x1e), if empty { root } else { old });
        t.write(old, &[0x21; 0x30]);
        let n = SCRATCH + 0x100;
        t.write(n, &[0x7e; 0x40]);
        t.check_entry(at, Call::this(n), &[0x00a0_00a0], Eax::Checked);
    }
}

/// 0x10006e80: the static node into an empty root list and after
/// another node (the words it sets hold junk).
#[test]
fn static_node() {
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x6e80);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let sentinel = w(&t, 0x46);
    let other = magic::SCRATCH + 0x600;
    for last in [sentinel, other] {
        for op in [
            0x07, 0x12, 0x1a, 0x24, 0x2e, 0x38, 0x41, 0x68, 0x72, 0x7c, 0x86, 0x90, 0x9a, 0xa4,
            0xae,
        ] {
            t.set_u32(w(&t, op), 0x5a5a_5a5b);
        }
        t.write(other, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x01), last);
        t.set_u16(w(&t, 0x62), 7);
        t.check_entry(at, magic::Call::default(), &[], magic::Eax::Checked);
    }
}

/// 0x10007530: the model start, then the model number found among the
/// nodes (first, later), not found, and no nodes.
#[test]
fn model_start() {
    use magic::SCRATCH;
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x7530);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let models = SCRATCH + 0x700;
    magic_owna::rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 0);
        models
    });
    magic_owna::rec(&mut t, 703, 1, |m, _| m.r32(SCRATCH + 0x6f0));
    let nodes = w(&t, 0xaa);
    // the model nodes' slot 333 answers a float (ST0)
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
    let tail = SCRATCH + 0x500;
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for (n, number) in [
        (4i32, 0x7710u32),
        (4, 0x5513),
        (3, 0x13),
        (0, 0x10),
        (-2, 0x10),
    ] {
        for op in [0x0c, 0x12, 0x1c, 0x26, 0x30, 0x46, 0x51, 0x77, 0xc1, 0xda] {
            t.set_u32(w(&t, op), 0x5a5a_5a5b);
        }
        t.write(tail, &[0x55; 0x10]);
        t.set_u32(w(&t, 0x07), tail);
        t.set_u32(w(&t, 0x40), 0x00ac_0001);
        // the model group's parent (the root): an empty child list
        let root = w(&t, 0x63);
        t.set_u32(root + 0x10, root);
        t.set_u32(w(&t, 0x8b), 0x00ab_008b);
        t.set_u32(w(&t, 0x99), n.cast_unsigned());
        t.set_u32(SCRATCH + 0x6f0, number);
        // the model numbers, each case (the model nodes write over them)
        for i in 0..4 {
            t.set_u32(nodes + 0x98 * i, 0x10 + i);
        }
        t.check_entry(at, magic::Call::default(), &[], magic::Eax::Checked);
    }
}
