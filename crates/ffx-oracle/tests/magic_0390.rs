//! `magic_0390.dll` against its Rust port (`ffx_magic::m0390`): `DllMain`
//! against every bound port, the task step, the state slot pair, the
//! object callback and the curve entry. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0390};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{dump, f, rec, recs, row, twin};

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

/// The particle part the start-up keeps (`G_PART`, the start-up's
/// operand `+0x3a`), which the object starts read.
fn with_part(t: &mut magic::Twin) {
    let g = t.img.word(t.at(0x2130) + 0x3a);
    let part = SCRATCH + 0x3000;
    t.write(part, &[0x2c; 0x100]);
    t.set_u32(g, part);
    // slot 941: the part's program table
    let table = SCRATCH + 0x3100;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0090_0000 + 0x100 * i);
    }
    rec(t, 941, 1, move |_, _| table);
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

/// A free ring of `n` nodes (0x20 bytes apart from `at`, junk inside) for
/// the DLL's pool take at `take`.
fn pool(t: &mut magic::Twin, take: u32, at: u32, n: u32) {
    let node = |i: u32| at + 0x20 * i;
    for i in 0..n {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node((i + n - 1) % n));
        t.set_u32(node(i) + 4, node((i + 1) % n));
    }
    let head = t.img.word(take + 0x17);
    t.set_u32(head, node(0));
    let count = t.img.word(take + 0x04);
    t.set_u16(count, u16::try_from(n).unwrap_or(0));
}

/// 0x100013b0: the script and the motion each done or not; when both
/// are, the colours and the node given back into an empty and a filled
/// free ring.
#[test]
fn task_step_13b0() {
    task_step_cases(N, 0x13b0);
    task_step_cases(447, 0x4ab0);
}

/// [`task_step_13b0`] in DLL `n` at `rva` (the state block's flag at
/// the code's displacement `+0x21`).
fn task_step_cases(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    recs(&mut t, &[(933, 4), (785, 1)]);
    let at = t.at(rva);
    t.set_u32(t.img.word(at + 0x07), STATE);
    let flag = STATE + t.img.word(at + 0x21);
    let run = t.img.rel32_target(at + 0x0c);
    let motion = t.img.rel32_target(at + 0x13);
    let colour = t.img.rel32_target(at + 0x2a);
    let gb = t.img.rel32_target(at + 0x35);
    let w = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
    t.set_u32(w(&t, run, 0x07), SCRATCH + 0x700);
    t.set_u32(w(&t, colour, 0x38), f(0.75));
    t.set_u32(w(&t, colour, 0x5f), f(-2.5));
    let node = SCRATCH + 0x200;
    let (prev, next) = (SCRATCH + 0x280, SCRATCH + 0x2c0);
    for (mode, done, head) in [
        (0u8, 1u32, 0u32),
        (7, 1, 0),
        (0, 0, 0),
        (7, 0, 0),
        (0, 1, SCRATCH + 0x300),
    ] {
        t.set_u8(w(&t, run, 0x01), mode);
        t.set_u32(w(&t, motion, 0x02), done);
        t.set_u16(w(&t, motion, 0x11), 5);
        t.set_u32(flag, 0x0c7c_0c7c);
        t.set_u32(w(&t, colour, 0x47), 0x4747_4747);
        t.write(node, &[0x3c; 0x20]);
        t.set_u32(node, prev);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, node);
        t.set_u32(node + 0xc, 0);
        t.write(prev, &[0x11; 0x10]);
        t.write(next, &[0x22; 0x10]);
        t.write(SCRATCH + 0x300, &[0x33; 0x10]);
        t.set_u32(w(&t, gb, 0x74), head);
        t.set_u16(w(&t, gb, 0x8e), 7);
        t.set_u32(w(&t, gb, 0x48), node);
        t.set_u32(w(&t, gb, 0x55), node);
        t.set_u32(w(&t, gb, 0x64), 0x6464);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}

/// The slots of the facing and the start-up: 262 fills its point, 333
/// answers `yaw`, 622 and 719 record theirs.
#[allow(clippy::cast_precision_loss)]
fn facing_slots(t: &mut magic::Twin, yaw: f64) {
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, f(1.5 + 2.0 * i as f32));
        }
        0x262
    });
    rec(t, 333, 5, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |_, _| yaw);
    rec(t, 622, 2, |_, a| a[1] ^ 0x0622_0000);
}

/// 0x10001430: the yaw above `HI`, below `LO`, between them and NaN,
/// into an empty and a filled task list.
#[test]
fn facing() {
    facing_cases(N, 0x1430);
    facing_cases(447, 0x4b30);
}

/// [`facing`] in DLL `n` at `rva`.
fn facing_cases(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    t.set_u32(t.img.word(at + 0x08), STATE);
    let st = t.img.rel32_target(at + 0xbc);
    let take = t.img.rel32_target(st + 0x54);
    let sel = SCRATCH + 0x180;
    t.set_u32(t.img.word(at + 0x8c), sel);
    t.set_u32(t.img.word(at + 0x22), 0x0000_b0b0);
    t.set_u32(t.img.word(at + 0x3c), 0x0000_3c3c);
    t.set_u32(t.img.word(at + 0x9b), 0x0000_9b9b);
    for (v, last) in [
        (0.0f64, 0u32),
        (8.0, 0),
        // K - v == HI (pi) exactly
        (
            f64::from(1.5 * std::f32::consts::PI - std::f32::consts::PI),
            0,
        ),
        (2.0, SCRATCH + 0x600),
        (f64::NAN, 0),
        (-1.0, SCRATCH + 0x600),
    ] {
        facing_slots(&mut t, v);
        pool(&mut t, take, SCRATCH + 0x8000, 3);
        t.write(STATE + 0x5f0, &[0x6c; 0x6a0]);
        t.set_u8(sel, 0x17);
        t.set_u8(sel + 1, 0x5a);
        t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
        t.set_u32(t.img.word(st + 0x5a), last);
        t.set_u32(t.img.word(st + 0x6e), last);
        t.set_u32(t.img.word(at + 0xe4), 0x00e4_00e4);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0390::facing(g, img, fr.at, fr.esp))
        });
    }
}

/// 0x10002130 (with the facing): two or three free nodes, into an empty
/// and a filled task list; slot 790's float.
#[test]
fn start_up() {
    start_up_cases(N, 0x2130, 0x122, m0390::start_up);
    start_up_cases(447, 0x54e0, 0xe5, m0390::start_up_0447);
}

/// [`start_up`] in DLL `n` at `rva` (`end`: an operand the start-up
/// stores to last).
fn start_up_cases(
    n: u16,
    rva: u32,
    end: u32,
    port: fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> Ret,
) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let face = t.img.rel32_target(at + 0x49);
    t.set_u32(t.img.word(face + 0x08), STATE);
    let take = t.img.rel32_target(at + 0x89);
    let st = t.img.rel32_target(face + 0xbc);
    let sel = SCRATCH + 0x180;
    t.set_u32(t.img.word(face + 0x8c), sel);
    t.set_u8(sel, 3);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    recs(&mut t, &[(271, 3), (711, 3), (710, 7)]);
    t.stub_st0(790, |_, _| 0.375);
    for (k, last) in [(3u32, 0u32), (4, SCRATCH + 0x600), (3, SCRATCH + 0x600)] {
        facing_slots(&mut t, 1.25);
        pool(&mut t, take, SCRATCH + 0x8000, k);
        t.write(STATE + 0x5f0, &[0x6c; 0x6a0]);
        t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
        t.set_u32(t.img.word(at + 0x8f), last);
        t.set_u32(t.img.word(at + 0xa6), last);
        t.set_u32(t.img.word(st + 0x5a), last);
        t.set_u32(t.img.word(st + 0x6e), last);
        for op in [0x18u32, 0x3a, 0x5e, end] {
            t.set_u32(t.img.word(at + op), 0x0a0a_0000 + op);
        }
        if n == N {
            // the floats the start-up sets to 1.0
            for op in [0xfcu32, 0x10c, 0x116] {
                t.set_u32(t.img.word(at + op), 0x0a0a_0000 + op);
            }
        }
        t.set_u32(t.img.word(at + 0x69), 0x6969_6969);
        t.set_u32(t.img.word(at + 0x71), 0x7171_7171);
        t.check(at, Call::default(), &[], Eax::Checked, port);
    }
}

/// 0x10001ef0: object 0 or 4, slot 520's mode and not, into an empty
/// and a filled task list.
#[test]
#[allow(clippy::cast_precision_loss)]
fn entry_1ef0() {
    let Some(mut t) = twin(N) else { return };
    with_state(&mut t);
    with_part(&mut t);
    let at = t.at(0x1ef0);
    let take = t.img.rel32_target(at + 0x185);
    let sel = SCRATCH + 0x180;
    t.set_u32(t.img.word(at + 0x58), sel);
    t.set_u8(sel, 5);
    t.set_u8(sel + 1, 0x5a);
    for i in 0..8 {
        t.set_u32(t.img.word(at + 0x6b) + 4 * i, 0x00c0_0000 + i);
    }
    t.set_u32(t.img.word(at + 0x1c9), 0x1c9);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, f(-1.5 + 0.5 * i as f32));
        }
        0x262
    });
    recs(&mut t, &[(268, 3), (12, 1), (538, 2)]);
    t.stub_st0(12, |_, _| 4.5);
    let mode = SCRATCH + 0x190;
    rec(&mut t, 520, 0, move |m, _| m.r32(mode));
    for (c78, md, last) in [
        (0u32, 0u32, 0u32),
        (1, 0x018b_0046, 0),
        (0, 0x018b_0046, SCRATCH + 0x600),
        (7, 0x018b_0047, SCRATCH + 0x600),
    ] {
        pool(&mut t, take, SCRATCH + 0x8000, 3);
        t.write(STATE, &[0x6c; 0xd00]);
        t.set_u32(STATE + 0xc78, c78);
        t.set_u32(STATE + 0xab4, f(-7.25));
        t.set_u32(STATE + 0xc68, f(0.625));
        t.set_u32(mode, md);
        t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
        t.set_u32(t.img.word(at + 0x18b), last);
        t.set_u32(t.img.word(at + 0x19e), last);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001750: the height from slot 251 or slots 17 and 19, the yaw on
/// each side of the wrap, the distance in each band, with and without
/// free nodes in the ring, into an empty and a filled task list.
#[test]
#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn entry_1750() {
    let Some(mut t) = twin(N) else { return };
    with_state(&mut t);
    with_part(&mut t);
    let at = t.at(0x1750);
    let sel = SCRATCH + 0x180;
    t.set_u32(t.img.word(at + 0x1b), sel);
    t.set_u32(t.img.word(at + 0xd2), sel);
    for i in 0..0x30 {
        t.set_u32(t.img.word(at + 0x2d) + 4 * i, 0x00c0_0000 + i);
        t.set_u32(t.img.word(at + 0x113) + 4 * i, 0x00d0_0000 + i);
    }
    t.set_u32(t.img.word(at + 0x74), 0x74);
    t.set_u32(t.img.word(at + 0x265), 0x265);
    t.set_u32(t.img.word(at + 0x27d), 0x27d);
    let q = SCRATCH + 0x1a0;
    // the first call of each case gives `B`, the second `Q`
    rec(&mut t, 262, 6, move |m, a| {
        let second = m.r32(row(262)) % 2 == 0;
        for i in 0..3 {
            let v = if second {
                m.r32(q + 4 * i)
            } else {
                f(-2.0 + 1.5 * i as f32)
            };
            m.w32(a[1] + 4 * i, v);
        }
        0x262
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, f(0.25 * i as f32 - 1.0));
        }
        0x235
    });
    rec(&mut t, 229, 3, |m, a| {
        dump(m, 229, a[1], 16);
        dump(m, 230, a[2], 4);
        for i in 0..4 {
            m.w32(a[0] + 4 * i, f(3.0 + i as f32));
        }
        0x229
    });
    rec(&mut t, 248, 3, |m, a| {
        dump(m, 248, a[2], 4);
        dump(m, 249, a[1], 4);
        0x248
    });
    recs(&mut t, &[(328, 3), (329, 3), (538, 2), (314, 1)]);
    let (yaw, h) = (SCRATCH + 0x1c0, SCRATCH + 0x1c8);
    rec(&mut t, 333, 6, |_, a| a[0]);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
    rec(&mut t, 251, 1, |_, a| a[0]);
    t.stub_st0(251, move |m, _| f64::from(f32::from_bits(m.r32(h))));
    recs(&mut t, &[(17, 2), (19, 1), (12, 5)]);
    t.stub_st0(17, |_, _| 1.75);
    t.stub_st0(19, |_, _| -0.5);
    t.stub_st0(12, |_, _| 2.25);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 3.0);
    let cases = [
        // flag, selector, yaw, height, Q (x, z), free nodes, list
        (0u32, 3u8, 0.0f32, 1.5f32, (10.0f32, 20.0f32), 3u16, 0u32),
        (0, 0x20, 8.0, 1.5, (150.0, 1.0), 2, SCRATCH + 0x600),
        (1, 3, 2.0, 1.5, (300.0, 7.0), 1, 0),
        (0, 0x1f, f32::NAN, -4.0, (60.0, 90.0), 3, SCRATCH + 0x600),
        (0, 0, -1.0, 0.0, (101.5, 0.0), 2, 0),
        // the distance exactly 100 and 200
        (0, 3, 2.0, 1.0, (96.5, -4.5), 2, 0),
        (0, 3, 2.0, 1.0, (196.5, -4.5), 2, 0),
        (2, 0x2f, 1.0, 0.0, (199.0, 1.5), 1, SCRATCH + 0x600),
    ];
    for (flag, s, y, hh, (qx, qz), free, last) in cases {
        t.set_u32(t.img.word(at + 0xcb), flag);
        t.set_u8(sel, s);
        t.set_u8(sel + 1, 0x5a);
        t.set_u32(yaw, f(y));
        t.set_u32(h, f(hh));
        t.set_u32(q, f(qx + 1.5));
        t.set_u32(q + 4, f(3.0));
        t.set_u32(q + 8, f(qz + 5.5));
        t.set_u32(q + 0xc, 0);
        t.write(STATE, &[0x6c; 0xd00]);
        // ring of `free` nodes (head at 0x3cf, count at 0x353)
        let node = |i: u32| SCRATCH + 0x8000 + 0x20 * i;
        let n = u32::from(free);
        for i in 0..n {
            t.write(node(i), &[0x5a; 0x20]);
            t.set_u32(node(i), node((i + n - 1) % n));
            t.set_u32(node(i) + 4, node((i + 1) % n));
        }
        t.set_u32(t.img.word(at + 0x3cf), node(0));
        t.set_u16(t.img.word(at + 0x353), free);
        t.set_u32(t.img.word(at + 0x353) + 2, 0x7777_7777);
        t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
        t.set_u32(t.img.word(at + 0x436), last);
        t.set_u32(t.img.word(at + 0x44c), last);
        for op in [0x35fu32, 0x369] {
            t.set_u32(t.img.word(at + op), 0x0b0b_0000 + op);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
