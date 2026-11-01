//! `magic_0563.dll` against its Rust port (`ffx_magic::m0563`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::f;
use magic_owna::{rec, recs, twin};

const N: u16 = 563;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

const FX: u32 = SCRATCH + 0x1_0000;

/// The object start's (`s4_misc::start_object_anim` at `so`) part table,
/// lock and pool.
fn object_start(t: &mut Twin, so: u32) {
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
    let sp = t.img.rel32_target(so + 0x84);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
}

/// 0x100014a0: frame 0x82 (the scene light) and not; the objects still
/// running; `+0x3ba8` 1 (slot 214 and the parts) and not.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x14a0);
    t.set_u32(t.img.word(at + 0xd), FX);
    let sl = t.img.rel32_target(at + 0x14);
    for (s, n) in [
        (235u32, 1usize),
        (328, 3),
        (329, 3),
        (229, 3),
        (248, 3),
        (554, 3),
        (327, 2),
        (858, 5),
        (214, 1),
        (628, 1),
        (384, 1),
        (941, 1),
        (772, 2),
        (779, 4),
    ] {
        rec(&mut t, s, n, move |_, a| {
            (0x5a5a_0000 + s) ^ a.first().copied().unwrap_or(0)
        });
    }
    // the scene light's block (its operand +0x1e) is *G_FX itself
    assert_eq!(t.img.word(sl + 0x1e), t.img.word(at + 0xd));
    t.set_u32(t.img.word(at + 0x31), 0x0031_0001);
    let rp = t.img.rel32_target(at + 0x41);
    let parts = SCRATCH + 0x200;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0941_0001);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    t.set_u32(t.img.word(rp + 0x76), 0);
    t.set_u32(t.img.word(rp + 0x43), SCRATCH + 0x300);
    let ro = t.img.rel32_target(at + 0x19);
    for (count, running, flag) in [
        (0x81u32, false, 1u32),
        (0x83, true, 1),
        (0x84, false, 0),
        (0x82, false, 2),
    ] {
        t.set_u32(t.img.word(at + 0x2), count);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(FX + 0x3ba8, flag);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002440: slot 371 twice, the scale task.
#[test]
fn scale_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2440);
    t.set_u32(t.img.word(at + 0x2a), FX);
    recs(&mut t, &[(371, 4)]);
    let st = t.img.rel32_target(at + 0x48);
    let take = t.img.rel32_target(st + 0x7b);
    magic_hi::pool(&mut t, take, SCRATCH + 0x6000, 3);
    t.write(FX + 0x3864, &[0x3d; 0x40]);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x100025a0 and 0x10002660: the objects' starts.
#[test]
fn object_starts() {
    let Some(mut t) = twin(N) else { return };
    for (rva, ops) in [(0x25a0u32, [0xb, 0x19]), (0x2660, [0xb, 0x1f])] {
        let at = t.at(rva);
        t.set_u32(t.img.word(at + ops[0]), FX);
        let so = t.img.rel32_target(at + ops[1]);
        object_start(&mut t, so);
        t.write(FX, &[0x3c; 0x3c00]);
        for i in 0..4 {
            t.set_u32(FX + 0x2b20 + 4 * i, f(1.5 + i as f32));
        }
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
