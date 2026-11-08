//! Round 21's own ports (`ffx_magic::r21`) against each original (the
//! copies bound to earlier ports are tested with those ports). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;
mod magic_owna;
mod magic_r11;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// Slots recorded with an answer from their first word.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (s << 20) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `r21::start_0431` (`magic_0431` 0x10004390) with the task list empty
/// and not, and its load step `r21::load_0431` (0x10001250): busy, the
/// sounds not ready, asking, not loaded, loaded (the start-up runs on
/// both sides).
#[test]
fn start_0431() {
    let Some(mut t) = twin(431) else { return };
    let f = t.at(0x4390);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    recs(&mut t, &[(719, 3), (711, 3), (710, 4), (271, 3)]);
    // the camera start's slots (`magic_0431.rs`' camera test)
    let cam = t.img.rel32_target(f + 0x79);
    rec(&mut t, 262, 4, |m, a| {
        for (i, at) in (0u32..).zip(&a[1..4]) {
            m.w32(*at, 0x0262_0000 + i);
        }
        0x262
    });
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 0.5);
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(cam + 0x08), fx);
    let starts = |t: &mut Twin, listed: bool| {
        junk_ops(
            t,
            f,
            &[
                0x16, 0x20, 0x2a, 0x34, 0x3e, 0x48, 0x52, 0x5c, 0x6a, 0x84, 0x8e,
            ],
        );
        t.set_u32(w(t, 0x99), 0x5a5a_5a5a);
        t.set_u32(w(t, 0xa1), 0x5a5a_5a5a);
        magic_r11::pools(t, f, 325);
        t.write(fx + 0x810, &[0x33; 0x20]);
        t.write(fx + 0xc60, &[0x55; 0xc0]);
        t.write(w(t, 0xea), &[0x5c; 0x40]);
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(w(t, 0xbf), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(t, 0xd3), SCRATCH + 0x600);
    };
    for listed in [false, true] {
        starts(&mut t, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r21::start_0431,
        );
    }
    // the load step
    let at = t.at(0x1250);
    let (busy, loaded, sound) = (SCRATCH + 0x5e0, SCRATCH + 0x5e4, SCRATCH + 0x5e8);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    rec(&mut t, 5, 1, move |m, _| m.r32(sound));
    rec(&mut t, 28, 2, |_, _| 0);
    t.set_u32(t.img.word(at + 0x32), 0x0000_5e5e);
    for (b, s, asked, l) in [
        (1u32, 0u32, 0u32, 1u32),
        (0, 3, 0, 1),
        (0, 0, 0, 0),
        (0, 0, 1, 2),
    ] {
        starts(&mut t, false);
        t.set_u32(busy, b);
        t.set_u32(sound, s);
        t.set_u32(loaded, l);
        t.set_u32(t.img.word(at + 0x22), asked);
        junk_ops(&mut t, at, &[0x6d, 0x77]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
