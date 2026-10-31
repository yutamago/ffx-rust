//! `magic_0605.dll` against its Rust port (`ffx_magic::m0605`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::m0605;
use magic::{Call, Eax, SCRATCH};
use magic_hi::{f, konst, lone_node};
use magic_owna::{rec, recs, twin};

const N: u16 = 605;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10009fc0: angles above, inside and below their range, NaN.
#[test]
fn move_pieces() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x9fc0);
    let (hi, lo) = (konst(&t, at, 0x9), konst(&t, at, 0x19));
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x1), r);
    for i in 0..0x12u32 {
        let p = r + 0x1f5c + 0x60 * i;
        for k in 0..0x18u32 {
            t.set_u32(p - 0x38 + 4 * k, f(0.5 * k as f32 - 1.25 * i as f32));
        }
        let a = [hi + 0.5, hi, f32::midpoint(hi, lo), lo, lo - 2.0, f32::NAN];
        for (k, x) in (0u32..).zip([
            a[(i % 6) as usize],
            a[((i + 2) % 6) as usize],
            a[((i + 4) % 6) as usize],
        ]) {
            t.set_u32(p - 0x38 + 4 * k, f(x));
            t.set_u32(p - 0x28 + 4 * k, f(0.0));
        }
    }
    t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
        ffx_magic::Ret::eax(m0605::move_pieces(g, img, fr))
    });
}

/// 0x1000a390: no node and a node; the object first, last, in the middle
/// of and missing from its list.
#[test]
fn end() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xa390);
    rec(&mut t, 772, 2, |_, a| a[0] ^ 0x0772_0000);
    recs(&mut t, &[(779, 4), (713, 1)]);
    let flag = SCRATCH + 0x10;
    t.set_u32(t.img.word(at + 0x49), flag);
    t.set_u32(t.img.word(at + 0x31), 0x00e0_0000);
    let p = SCRATCH + 0x14;
    t.set_u32(t.img.word(at + 0x90), p);
    let r = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0xa), r);
    let obj = r + 0x4c0;
    let give = t.img.rel32_target(at + 0x1b);
    let list = t.img.word(at + 0x82);
    let nodes = [SCRATCH + 0x200, SCRATCH + 0x220, SCRATCH + 0x240];
    for (full, pos) in [
        (false, Some(0usize)),
        (true, Some(0)),
        (true, Some(1)),
        (true, Some(2)),
        (true, None),
    ] {
        t.write(obj, &[0x4f; 0x200]);
        if full {
            lone_node(&mut t, give, SCRATCH + 0x1000);
        }
        t.set_u32(r + 0x30b4, if full { SCRATCH + 0x1000 } else { 0 });
        for (i, &n) in nodes.iter().enumerate() {
            t.set_u32(n, if i == 0 { 0 } else { nodes[i - 1] });
            t.set_u32(n + 4, if i == 2 { 0 } else { nodes[i + 1] });
            t.set_u32(
                n + 8,
                if pos == Some(i) {
                    obj
                } else {
                    0x00ee_0000 + u32::try_from(i).unwrap()
                },
            );
        }
        for (k, v) in [nodes[0], nodes[2], nodes[1], 3].into_iter().enumerate() {
            t.set_u32(list + 4 * u32::try_from(k).unwrap(), v);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x1000a450: actor -1 and listed ones, in and out of battle.
#[test]
fn actor_part() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xa450);
    let mode = t.img.rel32_target(at + 0x4d);
    let w = |t: &magic::Twin, off: u32| t.img.word(mode + off);
    let (list, me_obj, objs, battle, me) = (
        w(&t, 0x15),
        w(&t, 0x21),
        w(&t, 0x2e),
        w(&t, 0x4c),
        w(&t, 0x56),
    );
    t.set_u32(list, SCRATCH + 0x300);
    t.write(SCRATCH + 0x300, &[0x05, 0x25, 0x20, 0x1f]);
    t.set_u32(me_obj, 0x0bad_0b1e);
    for n in [5u32, 0x25, 0x20, 0x1f] {
        t.set_u32(objs + 4 * n, 0x0b00_0000 + n);
    }
    t.set_u32(me, 0x11);
    for op in [0x2c, 0x5e, 0x88] {
        t.set_u32(t.img.word(at + op), SCRATCH + 0x300);
    }
    for op in [0x8, 0x57, 0x81] {
        t.set_u32(t.img.word(at + op), 0x11);
    }
    for (s, n) in [(210u32, 2usize), (211, 2), (217, 1), (255, 1)] {
        rec(&mut t, s, n, move |_, a| s << 16 ^ a[0]);
    }
    rec(&mut t, 719, 3, |_, a| a[0] ^ 0x0719_0000);
    recs(&mut t, &[(239, 3), (388, 3), (528, 2), (393, 8)]);
    let r = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x1e), r);
    let obj = SCRATCH + 0x1000;
    for b in [0u32, 1] {
        t.set_u32(battle, b);
        for i in [u32::MAX, 0, 1, 3] {
            t.write(obj, &[0; 0x200]);
            t.set_u32(obj + 0x120, i);
            t.check(at, Call::default(), &[obj], Eax::Checked, |g, img, fr| {
                ffx_magic::Ret::eax(m0605::actor_part(g, img, fr))
            });
        }
    }
}
