//! `magic_0449.dll` against its Rust ports (`ffx_magic::m0449`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{rec, twin};

const N: u16 = 449;

/// 0x10005450: the objects busy, the effect not ready, the flag set, the
/// actor past 31, and the actor's two calls (junk under the entry for the
/// frame's matrix).
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5450);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(719u32, 3usize), (306, 3), (211, 2), (214, 3)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    for (s, n, k) in [(217u32, 2usize, 0x0217), (255, 4, 0x0255)] {
        rec(&mut t, s, n, move |m: &mut dyn StubMem, a: &[u32]| {
            let v = m.r32(a[1]);
            m.w32(a[1], v ^ k);
            m.w32(a[1] + 8, a[0] ^ k);
            k
        });
    }
    let ro = t.img.rel32_target(at + 0x18);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let e = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x13), e);
    t.set_u32(w(&t, 0x33), 0x00ab_0033);
    t.set_u32(w(&t, 0x82), 0x00ab_0082);
    for (busy, ready, flag, actor) in [
        (true, 1u32, 0u32, 0u32),
        (false, 0, 0, 0),
        (false, 2, 0, 0),
        (false, 1, 1, 0),
        (false, 1, 0, 0x20),
        (false, 1, 0, u32::MAX),
        (false, 1, 0, 0x1f),
    ] {
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        t.set_u32(e + 0x56fc, ready);
        t.set_u32(w(&t, 0x2b), flag);
        t.set_u32(w(&t, 0x3b), actor);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10005500: the angle above, inside and below the range (and NaN).
#[test]
fn camera_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5500);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 262, 4, |m, a| {
        for (i, at) in (0u32..).zip(&a[1..4]) {
            m.w32(*at, 0x0262_0000 + i);
        }
        0x262
    });
    let angle = SCRATCH + 0x600;
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(angle))));
    rec(&mut t, 622, 2, |_, a| 0x0622_0000 ^ a[1]);
    let fx = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x08), fx);
    t.set_u32(w(&t, 0x22), 0x00ab_0022);
    t.set_u32(w(&t, 0x9b), 0x00ab_009b);
    let byte = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x8c), byte);
    t.set_u8(byte, 0x13);
    // a non-zero byte above the index byte
    t.set_u8(byte + 1, 0x7f);
    let base = f32::from_bits(t.u32(w(&t, 0x4f)));
    let hi = f32::from_bits(t.u32(w(&t, 0x5e)));
    let lo = f32::from_bits(t.u32(w(&t, 0x72)));
    let mut angles = vec![
        base,
        base - hi - 1.0,
        base - lo + 1.0,
        base - 0.5,
        // h inside (LO, 0)
        base + 1.0,
        f32::NAN,
        -20.0,
        20.0,
    ];
    for bound in [hi, lo] {
        let guess = (base - bound).to_bits();
        if let Some(r) = (0..256u32)
            .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
            .map(f32::from_bits)
            .find(|&r| (base - r).to_bits() == bound.to_bits())
        {
            angles.push(r);
        }
    }
    for a in angles {
        t.set_u32(angle, a.to_bits());
        t.write(fx + 0xf60, &[0x33; 0x20]);
        t.write(fx + 0x56d0, &[0x55; 0x30]);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0449::camera_start,
        );
    }
}

/// 0x100055e0: the actors' table with gaps and the last actor set or not.
#[test]
fn actors_pass() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x55e0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0] ^ (a[1] << 8));
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    let flag = SCRATCH + 0x600;
    // slot 779 copies the flag out (its answer is not used)
    rec(&mut t, 779, 4, move |m, a| {
        let k = m.r32(magic_owna::row(779));
        let v = m.r32(flag);
        m.w32(SCRATCH + 0x4000 + 4 * (k % 0x100), v);
        v ^ a[3]
    });
    t.set_u32(w(&t, 0x34), flag);
    let table = w(&t, 0x0c);
    for last in [0u32, 0x0a0f] {
        for i in 0..16 {
            let v = if i == 15 {
                last
            } else if i % 5 == 1 {
                0x0a00 + i
            } else {
                0
            };
            t.set_u32(table + 4 * i, v);
        }
        t.set_u32(flag, 0x77);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0449::actors_pass,
        );
    }
}

/// 0x100053b0: the script and the motion each done or not; when both
/// are, the actors' pass and the node given back.
#[test]
fn task_step_pass() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x53b0);
    let w = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, at, 0x07), s);
    t.write(w(&t, at, 0x10), &[0x3b; 0x30]);
    t.set_u32(w(&t, at, 0x26), 0x0026_0026);
    let run = t.img.rel32_target(at + 0x33);
    let motion = t.img.rel32_target(at + 0x3a);
    let pass = t.img.rel32_target(at + 0x51);
    let gb = t.img.rel32_target(at + 0x5c);
    t.set_u32(w(&t, run, 0x07), SCRATCH + 0x700);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0] ^ (a[1] << 8));
    rec(&mut t, 779, 4, |_, a| a[3]);
    let flag = SCRATCH + 0x600;
    t.set_u32(w(&t, pass, 0x34), flag);
    let table = w(&t, pass, 0x0c);
    for i in 0..16 {
        t.set_u32(table + 4 * i, if i == 3 { 0x0a03 } else { 0 });
    }
    let node = SCRATCH + 0x200;
    let (prev, next) = (SCRATCH + 0x280, SCRATCH + 0x2c0);
    for (mode, done) in [(0u8, 1u32), (7, 1), (0, 0), (7, 0)] {
        t.set_u8(w(&t, run, 0x01), mode);
        t.set_u32(w(&t, motion, 0x02), done);
        t.set_u16(w(&t, motion, 0x11), 5);
        t.write(s, &vec![0x6c; 0x6000]);
        for i in 0..4 {
            t.set_u32(s + 0xf60 + 4 * i, 0x0f60_0000 + i);
        }
        t.write(node, &[0x3c; 0x20]);
        t.set_u32(node, prev);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, node);
        t.set_u32(node + 0xc, 0);
        t.write(prev, &[0x11; 0x10]);
        t.write(next, &[0x22; 0x10]);
        t.set_u32(w(&t, gb, 0x74), 0);
        t.set_u16(w(&t, gb, 0x8e), 7);
        t.set_u32(w(&t, gb, 0x48), node);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}
