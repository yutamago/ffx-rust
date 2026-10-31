//! `magic_0093.dll` against its Rust port (`ffx_magic::m0093`): `DllMain`
//! against every bound port and the state reset with the count 0x12
//! (internal). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0093};
use magic::{Call, Eax, SCRATCH};
use magic_owna::twin;

const N: u16 = 93;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn reset_state_12() {
    let Some(mut t) = twin(N) else { return };
    let obj = SCRATCH + 0x100;
    for arg in [0u32, 0x1000_2c30] {
        t.write(obj, &[0xa5; 0x130]);
        t.check(
            t.at(0x1b80),
            Call::this(obj),
            &[0x77, arg],
            Eax::Checked,
            |g, _, fr| {
                let arg = fr.arg(g, 1);
                Ret::eax(m0093::reset_state_12(g, fr.ecx, arg))
            },
        );
    }
}

/// 0x10001ca0: a part object with no free pool node, as the first node
/// of an empty list and appended to a list.
#[test]
fn part_object() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1ca0);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let (obj, owner, table) = (SCRATCH + 0x1000, SCRATCH + 0x300, SCRATCH + 0x400);
    t.set_u32(w(&t, 0x01), SCRATCH + 0x200);
    t.set_u32(w(&t, 0x5b), owner);
    t.set_u32(owner + 0x10, 0x00a1_0010);
    t.set_u32(table, 0x0f00_0000);
    t.stub(941, move |m, a| {
        m.w32(SCRATCH + 0x3000, a[0]);
        table
    });
    t.stub(714, |m, a| {
        for (i, v) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x3010 + 4 * i, *v);
        }
        0x714
    });
    let alloc = t.img.rel32_target(at + 0x95);
    let (free, nodes) = (t.img.word(alloc + 0x04), t.img.word(alloc + 0x27) - 8);
    let tail = SCRATCH + 0x500;
    for (case, (n_free, has_tail)) in [(0u16, false), (5, false), (5, true)]
        .into_iter()
        .enumerate()
    {
        t.write(obj, &[0x3c; 0x130]);
        t.set_u16(free, n_free);
        t.write(nodes, &[0x5d; 0x30]);
        for i in 0..4 {
            t.set_u32(nodes + 12 * i + 8, if i == 1 { 0x77 } else { 0 });
        }
        t.set_u32(w(&t, 0xa0), if has_tail { tail } else { 0 });
        t.write(tail, &[0x55; 0x10]);
        let scale = [0.5f32, -2.0, 1.0e3][case];
        let c = Call {
            edx: obj,
            ..Call::default()
        }
        .with_xmm(&[
            ffx_magic::Xmm::f32(9.0),
            ffx_magic::Xmm::f32(8.0),
            ffx_magic::Xmm::f32(7.0),
            ffx_magic::Xmm::f32(scale),
        ]);
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0093::part_object(
                g,
                img,
                fr.at,
                fr.edx,
                fr.xmm_f32(3).to_bits(),
            ))
        });
    }
}
