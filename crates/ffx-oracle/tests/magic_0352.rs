//! `magic_0352.dll` against its Rust ports (`ffx_magic::m0352`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, row, twin};

const N: u16 = 352;

/// 0x10004c10: the script and the motion step running or done.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4c10);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 772, 2, |m, a| {
        let k = m.r32(row(772));
        0x0772_0000 ^ a[0] ^ (k << 8)
    });
    t.stub_edx(772, |_, a| 0x00ed_0000 ^ a[0]);
    // slot 779 copies the game flag out (it is 1 during the call)
    rec(&mut t, 779, 4, |m, a| {
        let k = m.r32(row(779));
        let v = m.r32(SCRATCH + 0x600);
        m.w32(SCRATCH + 0x4000 + 4 * (k % 0x40), v);
        0x0779_0000 ^ a[2]
    });
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x0a), fx);
    t.set_u32(w(&t, 0x3f), SCRATCH + 0x600);
    let sr = t.img.rel32_target(at + 0x0f);
    let ms = t.img.rel32_target(at + 0x16);
    let node = SCRATCH + 0x3000;
    for (ended, done) in [(true, true), (false, true), (true, false)] {
        t.set_u8(t.img.word(sr + 0x01), if ended { 0 } else { 2 });
        t.set_u16(t.img.word(sr + 0x59), 0xffff);
        t.set_u32(t.img.word(sr + 0x91), 0);
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.write(node, &[0x4c; 0x20]);
        t.write(fx + 0xbd0, &[0x44; 0x10]);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}

/// 0x10004e60 and 0x10004f60: the camera point and its show (junk under
/// the entry for the frame).
#[test]
fn camera_point() {
    use magic::StubMem;
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4e60);
    for (s, k) in [(723u32, 0x0723u32), (724, 0x0724)] {
        rec(&mut t, s, 1, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, k + i);
            }
            k
        });
    }
    rec(&mut t, 327, 2, |m: &mut dyn StubMem, a: &[u32]| {
        let v = m.r32(a[1]);
        m.w32(a[0], v ^ 0x327);
        0x327
    });
    rec(&mut t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 904, 2, |_, a| 0x0904_0000 ^ a[0] ^ (a[1] << 4));
    let obj = SCRATCH + 0x1000;
    // 0x10004f60: the same, then slot 904
    for at in [at, t.at(0x4f60)] {
        t.write(obj, &[0x5c; 0x120]);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10004cd0 (internal): the camera's start, the angle above, inside
/// and below the range (and NaN).
#[test]
fn camera_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4cd0);
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
    rec(&mut t, 12, 2, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, _| 7.5);
    let fx = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x0e), fx);
    t.set_u32(w(&t, 0x26), 0x00ab_0026);
    let base = f32::from_bits(t.u32(w(&t, 0x36)));
    let hi = f32::from_bits(t.u32(w(&t, 0x45)));
    let lo = f32::from_bits(t.u32(w(&t, 0x59)));
    // h = 0, above HI, below LO, NaN, and inside (LO, 0)
    let mut angles = vec![base, base - hi - 1.0, base - lo + 1.0, f32::NAN, base + 1.0];
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
        t.write(fx + 0x970, &[0x33; 0x40]);
        t.set_u32(fx + 0xbd8, 7);
        t.set_u32(w(&t, 0xd1), 7);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0352::camera_start,
        );
    }
}
