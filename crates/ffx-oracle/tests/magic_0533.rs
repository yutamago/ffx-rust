//! `magic_0533.dll` against its Rust port (`ffx_magic::m0533`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0533};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 533;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10003720: the model object's set-up, with a parent and with none,
/// the object's child list empty and not.
#[test]
fn model_setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3720);
    rec(&mut t, 287, 3, |_, _| 0x0287_0000);
    recs(&mut t, &[(281, 1), (283, 2)]);
    let group = SCRATCH + 0x100;
    let obj = SCRATCH + 0x1000;
    let parent = SCRATCH + 0x1400;
    let kid = SCRATCH + 0x1600;
    for (with_parent, kids) in [(true, false), (false, true)] {
        t.write(obj, &[0x5b; 0x200]);
        t.write(parent, &[0x2c; 0x40]);
        t.set_u32(parent + 0x10, parent);
        t.write(kid, &[0x3d; 0x40]);
        // the object's own child list: empty (itself) or one child
        t.set_u32(obj + 0x10, if kids { kid } else { obj });
        t.set_u16(obj + 0x18, 0x0102);
        for i in 0..10u32 {
            t.set_u32(group + 4 * i, 0x00c0_0000 + 0x11 * i);
        }
        t.set_u32(group, obj);
        t.set_u32(group + 4, if with_parent { parent } else { 0 });
        t.check(at, Call::this(group), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0533::model_setup(g, img, fr))
        });
    }
}
