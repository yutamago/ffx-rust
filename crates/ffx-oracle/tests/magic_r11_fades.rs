//! Round 11's start-ups with an object and two fades
//! (`ffx_magic::r11_fades`) against each original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;
mod magic_r11;

use ffx_magic::r11_fades;
use magic::{Call, Eax};
use magic_owna::twin;

/// `magic_0105` 0x10003130: the task list empty and not (twice each, the
/// pools' rings moving on).
#[test]
fn object_two_fades_0105() {
    let Some(mut t) = twin(105) else { return };
    let f = t.at(0x3130);
    for listed in [false, true, true, false] {
        magic_r11::object_two_fades_setup(&mut t, f, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            r11_fades::object_two_fades,
        );
    }
}

/// The other fade start-ups (`magic_0101`, `0093`, `0085`, `0103`): the
/// task list empty and not; `0085`'s yaw on each side of its wrap and
/// NaN; `0103`'s mean over one and three actors.
#[test]
fn fade_start_ups() {
    for &(dll, rva, len, starts, actor, junk, (first, last)) in magic_r11::FADES {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let yaw = magic::SCRATCH + 0x90;
        magic_owna::rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
        t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
        let port: fn(&mut _, &_, &_) -> _ = match dll {
            101 => r11_fades::object_colour_two_fades,
            93 => r11_fades::part_object_two_fades,
            85 => r11_fades::yaw_two_fades,
            _ => r11_fades::mean_two_fades,
        };
        let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
        let k = if dll == 85 { kf(&t, 0xc0) } else { 0.0 };
        let (hi, lo) = if dll == 85 {
            (kf(&t, 0xd0), kf(&t, 0xe4))
        } else {
            (0.0, 0.0)
        };
        for (case, (listed, v)) in [
            (false, 0.0f32),
            (true, k - hi - 1.0),
            (true, f32::NAN),
            (false, k - lo + 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            magic_r11::fades_setup(&mut t, f, len, starts, actor, junk, (first, last, listed));
            t.set_u32(yaw, v.to_bits());
            if dll == 85 {
                // slot 333's actor
                t.set_u32(t.img.word(f + 0xa3), 0x00a3_0000 ^ (v.to_bits() >> 8));
            }
            if dll == 103 {
                let m = t.img.rel32_target(f + 0x44);
                t.set_u32(t.img.word(m + 0x11), if case % 2 == 0 { 1 } else { 3 });
                magic_r11::first_actor(&mut t, m, 0x4d, 0x58);
            }
            t.check(f, Call::default(), &[], Eax::Checked, port);
        }
    }
}

/// `magic_0252` 0x100013a0 (`r11_effect::effect_start_flat`): the yaw
/// on each side of the wrap, between and NaN; the task list empty and
/// not.
#[test]
fn effect_start_flat() {
    let Some(mut t) = twin(252) else { return };
    let f = t.at(0x13a0);
    let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
    let k = kf(&t, 0x96);
    let (hi, lo) = (kf(&t, 0xa5), kf(&t, 0xb9));
    for (v, listed) in [
        (0.0f32, false),
        (k - hi - 1.0, true),
        (k - lo + 1.0, true),
        (f32::NAN, false),
    ] {
        magic_r11::effect_flat_setup(&mut t, f, v, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r11_effect::effect_start_flat,
        );
    }
}

/// `magic_0365` 0x10002c10 (`r11_effect::effect_start_height`): as
/// [`effect_start_flat`].
#[test]
fn effect_start_height() {
    let Some(mut t) = twin(365) else { return };
    let f = t.at(0x2c10);
    let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
    let k = kf(&t, 0xd7);
    let (hi, lo) = (kf(&t, 0xe7), kf(&t, 0xfb));
    for (v, listed) in [
        (0.0f32, false),
        (k - hi - 1.0, true),
        (k - lo + 1.0, true),
        (f32::NAN, false),
    ] {
        magic_r11::effect_height_setup(&mut t, f, v, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r11_effect::effect_start_height,
        );
    }
}

/// `magic_0197` 0x10001370 (`r11_effect::effect_start_lookup`): the yaw
/// as in [`effect_start_flat`], the flag set and clear (the count the
/// start-up stores is below 0x20, so slot 296 when clear).
#[test]
fn effect_start_lookup() {
    let Some(mut t) = twin(197) else { return };
    let f = t.at(0x1370);
    let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
    let k = kf(&t, 0xaf);
    let (hi, lo) = (kf(&t, 0xbe), kf(&t, 0xd2));
    for (v, busy, listed) in [
        (0.0f32, false, false),
        (k - hi - 1.0, true, true),
        (k - lo + 1.0, false, true),
        (f32::NAN, true, false),
    ] {
        magic_r11::effect_lookup_setup(&mut t, f, v, busy, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r11_effect::effect_start_lookup,
        );
    }
}

/// `magic_0231` 0x10001370 (`r11_effect::effect_start_kind`): both
/// kinds, the yaw as in [`effect_start_flat`], the task list empty and
/// not.
#[test]
fn effect_start_kind() {
    let Some(mut t) = twin(231) else { return };
    let f = t.at(0x1370);
    let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
    let k = kf(&t, 0x151);
    let (hi, lo) = (kf(&t, 0x161), kf(&t, 0x175));
    for (v, kind, listed) in [
        (0.0f32, 0u32, false),
        (k - hi - 1.0, 1, true),
        (k - lo + 1.0, 0, true),
        (f32::NAN, 7, false),
    ] {
        magic_r11::effect_kind_setup(&mut t, f, v, kind, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r11_effect::effect_start_kind,
        );
    }
}
