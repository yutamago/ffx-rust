//! `magic_0390.dll` against its Rust port (`ffx_magic::m0390`): `DllMain`
//! against every bound port, the task step, the state slot pair, the
//! object callback and the curve entry. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{dump, f, rec, recs, twin};

const N: u16 = 390;
const STATE: u32 = SCRATCH + 0x1000;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

fn with_state(t: &mut magic::Twin) {
    let g = t.img.word(t.at(0x13f0) + 0x03);
    t.set_u32(g, STATE);
}

#[test]
fn state_slots() {
    let Some(mut t) = twin(N) else { return };
    with_state(&mut t);
    rec(&mut t, 623, 7, |_, a| a[0] ^ 0x623);
    rec(&mut t, 624, 1, |_, a| a[0] ^ 0x624);
    let h = t.img.word(t.at(0x15b0) + 0x20);
    for v in [0x1234u32, 0] {
        t.set_u32(h, v);
        t.check_entry(t.at(0x15b0), Call::default(), &[], Eax::Checked);
        t.check_entry(t.at(0x2100), Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn task_step() {
    let Some(mut t) = twin(N) else { return };
    with_state(&mut t);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3), (214, 1)]);
    let f = t.at(0x13f0);
    let ro = t.img.rel32_target(f + 0x08);
    let list = t.img.word(ro + 0x7c);
    let arg = t.img.word(f + 0x20);
    t.set_u32(arg, 0xabcd);
    let task = SCRATCH + 0x200;
    // the object list empty (done), then one object marked skipped (not done)
    for (node, c7c) in [(0u32, 1u32), (0, 0), (0, 2), (SCRATCH + 0x300, 1)] {
        t.set_u32(list + 8, node);
        if node != 0 {
            let obj = SCRATCH + 0x400;
            t.set_u32(node + 4, 0);
            t.set_u32(node + 8, obj);
            t.set_u16(obj + 0x124, 0x8001);
        }
        t.set_u32(STATE + 0xc7c, c7c);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn object_callback() {
    let Some(mut t) = twin(N) else { return };
    recs(&mut t, &[(237, 4), (236, 7), (330, 3)]);
    rec(&mut t, 235, 9, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x3f00_0000 + 0x1_0000 * i);
        }
        0
    });
    rec(&mut t, 231, 6, |m, a| {
        dump(m, 231, a[0], 16);
        dump(m, 232, a[2], 16);
        0x231
    });
    let h = t.img.word(t.at(0x15f0) + 0x2d);
    t.set_u32(h, 0x77);
    let obj = SCRATCH + 0x800;
    for s in [2.0f64, 0.5, -3.25, 0.0, 1e-30] {
        t.stub_st0(277, move |_, _| s);
        for i in 0..16u32 {
            #[allow(clippy::cast_precision_loss)]
            let v = 1.5 + i as f32 * 0.37;
            t.set_u32(obj + 0x80 + 4 * i, f(v));
        }
        t.set_u32(obj + 0x104, 0x5555);
        t.check_entry(t.at(0x15f0), Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn curve() {
    let Some(mut t) = twin(N) else { return };
    with_state(&mut t);
    let e = t.at(0x1be0);
    let dt = SCRATCH + 0x100;
    t.set_u32(t.img.word(e + 0x21), dt);
    let flag = t.img.word(e + 0x23e);
    let index = t.img.word(e + 0x246);
    let h = t.img.word(e + 0x27a);
    t.set_u32(h, 0x99);
    let pick = SCRATCH + 0x104;
    rec(&mut t, 255, 2, |m, a| {
        dump(m, 255, a[1], 3);
        0x255
    });
    rec(&mut t, 11, 1, move |m, _| m.r32(pick));
    recs(&mut t, &[(33, 4), (32, 3)]);
    let points = [
        (1.0f32, 2.0f32, 3.0f32, -4.0f32, 5.5f32, 0.25f32, 7.0f32),
        (-10.0, 0.5, 100.0, 3.0, -2.0, 1.75, -1.0),
    ];
    for (n, d) in [
        (60u32, 1u32),
        (30, 2),
        (23, 0),
        (22, 1),
        (10, 3),
        (2, 5),
        (0, 1),
        (0, 0xffff_fffe),
        (0xffff_fff0, 1),
        (5, 0xffff_fffe),
    ] {
        for &(x0, y, z0, x1, z1, hump, w) in &points {
            for (fl, ix, p) in [
                (0u32, 3u32, 0u32),
                (0, 0x20, 0),
                (1, 3, 1),
                (2, 3, 2),
                (0, 0xffff_ffff, 0),
            ] {
                t.set_u32(STATE + 0xc6c, n);
                t.set_u32(dt, d);
                t.set_u32(STATE + 0xab0, f(x0));
                t.set_u32(STATE + 0xab4, f(y));
                t.set_u32(STATE + 0xab8, f(z0));
                t.set_u32(STATE + 0xac0, f(x1));
                t.set_u32(STATE + 0xac8, f(z1));
                t.set_u32(STATE + 0xc64, f(hump));
                t.set_u32(STATE + 0xabc, f(w));
                t.set_u32(flag, fl);
                t.set_u32(index, ix);
                t.set_u32(pick, p);
                t.check_entry(e, Call::default(), &[], Eax::Checked);
            }
        }
    }
}
