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

/// `magic_0065` 0x10002fd0: the flag bit set and clear, the task list
/// empty and not.
#[test]
fn colour_start_0065() {
    let Some(mut t) = twin(65) else { return };
    let f = t.at(0x2fd0);
    for (bit, listed) in [(false, false), (true, false), (true, true), (false, true)] {
        magic_r11::colour_start_setup(&mut t, f, bit, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r11_misc::colour_start_0065,
        );
    }
}

/// `magic_0054` 0x10001370: the task list empty and not.
#[test]
fn two_anim_objects_0054() {
    let Some(mut t) = twin(54) else { return };
    let f = t.at(0x1370);
    for listed in [false, true] {
        magic_r11::two_anim_setup(&mut t, f, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r11_misc::two_anim_objects_0054,
        );
    }
}
