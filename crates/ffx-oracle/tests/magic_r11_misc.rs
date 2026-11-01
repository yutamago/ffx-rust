//! Round 11's small own functions (`ffx_magic::r11_misc`) against each
//! original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;
mod magic_r11;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

/// `magic_0383` 0x10001370: the main object starts and 16 bytes are
/// copied (junk in the target first).
#[test]
fn main_object_0383() {
    let Some(mut t) = twin(383) else { return };
    let f = t.at(0x1370);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x11);
    magic_r11::object_start_setup(&mut t, start, (0x7d, 0x8f));
    let to = t.img.word(f + 0x22);
    t.write(to, &[0x5e; 16]);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

/// `magic_0369` 0x10001370: the yaw on each side of the wrap, between
/// and NaN.
#[test]
fn main_object_0369() {
    let Some(mut t) = twin(369) else { return };
    let f = t.at(0x1370);
    let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x16);
    magic_r11::object_start_setup(&mut t, start, (0x7d, 0x8f));
    t.set_u32(op(&t, 0x21), 0x00a5_0000);
    t.stub_st0(12, |_, a| f64::from(a[0] >> 16) * 0.5);
    let yaw = SCRATCH + 0x90;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
    let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(op(t, o)));
    let k3 = kf(&t, 0x8a);
    for v in [
        0.0f32,
        8.0,
        -8.0,
        f32::NAN,
        k3 - kf(&t, 0x9a),
        k3 - kf(&t, 0xb8),
    ] {
        t.set_u32(yaw, v.to_bits());
        t.write(op(&t, 0xac), &[0x5f; 4]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
