//! `magic_0262.dll` against its Rust ports (`ffx_magic::m0262`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::m0262;
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 262;

/// The static nodes (0x10006d20, 0x10006dc0, 0x10006e90) into an empty
/// root list and after another node; the words they set hold junk.
#[test]
fn static_nodes() {
    let Some(mut t) = twin(N) else { return };
    for (rva, stored) in [
        (
            0x6d20u32,
            &[
                0x0a, 0x15, 0x1d, 0x27, 0x31, 0x3b, 0x44, 0x6e, 0x78, 0x81, 0x87,
            ][..],
        ),
        (
            0x6dc0,
            &[0x0b, 0x13, 0x1d, 0x27, 0x31, 0x3a, 0x64, 0x6d, 0x73, 0x7d][..],
        ),
        (
            0x6e90,
            &[
                0x0a, 0x15, 0x1d, 0x27, 0x31, 0x3b, 0x44, 0x6e, 0x77, 0x7d, 0x87, 0x91, 0x9b, 0xa5,
                0xaf,
            ][..],
        ),
    ] {
        let at = t.at(rva);
        let w = |t: &Twin, off: u32| t.img.word(at + off);
        // the root list: its last (+0x04) and the sentinel it is compared
        // with (the empty case)
        let sentinel = w(&t, 0x49 - if rva == 0x6dc0 { 0x0a } else { 0 });
        let other = SCRATCH + 0x600;
        for (k, last) in [sentinel, other].into_iter().enumerate() {
            for &op in stored {
                t.set_u32(w(&t, op), 0x5a5a_5a5b);
            }
            t.write(other, &[0x66; 0x10]);
            t.set_u32(w(&t, 0x04), last);
            t.set_u16(w(&t, if rva == 0x6dc0 { 0x5e } else { 0x68 }), 7);
            let c = Call {
                eax: 0x00ea_0000 + u32::try_from(k).unwrap(),
                ..Call::default()
            };
            t.check_entry(at, c, &[0x00a1_0000 + rva], Eax::Checked);
        }
    }
}

/// 0x10006290 (internal, `ecx` = `[group]`): model nodes with and
/// without parents (first, middle and last children), the group node
/// with and without one.
#[test]
fn models_leave() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6290);
    let grp = SCRATCH + 0x1_0000;
    let parent = SCRATCH + 0x800;
    let (left, right) = (SCRATCH + 0x900, SCRATCH + 0xa00);
    let cell = SCRATCH + 0x100;
    for (n, gparent) in [(0i32, false), (3, true), (2, false), (-1, true)] {
        t.write(grp, &[0x5c; 0x500]);
        t.write(parent, &[0x3c; 0x20]);
        t.write(left, &[0x1c; 0x20]);
        t.write(right, &[0x2c; 0x20]);
        t.set_u32(grp + 0x4dc, n.cast_unsigned());
        // node 0: under the parent between a and b; node 1: none; node
        // 2: the parent's only child (both ends the parent)
        for (i, (p, prev, next)) in [
            (parent, left, right),
            (0, left, right),
            (parent, parent, parent),
        ]
        .into_iter()
        .enumerate()
        {
            let node = grp + 0x1c + 0x98 * u32::try_from(i).unwrap();
            t.set_u32(node + 4, prev);
            t.set_u32(node + 8, next);
            t.set_u32(node + 0x14, p);
        }
        t.set_u32(grp + 4, parent);
        t.set_u32(grp + 8, left);
        t.set_u32(grp + 0x14, if gparent { parent } else { 0 });
        t.set_u16(parent + 0x18, 5);
        t.set_u32(cell, grp);
        let c = Call {
            eax: 0x00ea_00ea,
            ecx: cell,
            ..Call::default()
        };
        t.check(at, c, &[], Eax::Checked, m0262::models_leave);
    }
}

/// 0x10007440: the model start with the ring's tail anywhere and the
/// model group's actor list empty and of two.
#[test]
fn model_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7440);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let models = SCRATCH + 0x700;
    // slot 218: the actor's model list, its count into the second word
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], m.r32(SCRATCH + 0x6f8));
        models
    });
    t.write(models, &[3, 0, 5, 0, 0, 0, 0, 0]);
    // the model nodes' slot 333 answers a float (ST0)
    t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
    let tail = SCRATCH + 0x500;
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for (count, actor) in [(0u32, 0x00ac_0001u32), (2, 0x00ac_0002)] {
        for op in [0x0c, 0x12, 0x1c, 0x26, 0x30, 0x46, 0x50, 0x76, 0x85] {
            t.set_u32(w(&t, op), 0x5a5a_5a5b);
        }
        t.write(tail, &[0x55; 0x10]);
        t.set_u32(w(&t, 0x07), tail);
        t.set_u32(w(&t, 0x40), actor);
        // the model group's parent (the root): an empty child list
        let root = w(&t, 0x62);
        t.set_u32(root + 0x10, root);
        t.set_u32(SCRATCH + 0x6f8, count);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
