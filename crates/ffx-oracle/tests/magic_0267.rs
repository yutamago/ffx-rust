//! `magic_0267.dll` against its Rust ports (`ffx_magic::m0267`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

const N: u16 = 267;

/// 0x10001340 (internal: `ecx` = b, `edx` = the points, stack `(n, out,
/// m)`): no points, one and three (the argument words it rewrites
/// compared too).
#[test]
fn transform_points() {
    use magic::{SCRATCH, StubMem};
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x1340);
    // slot 229 writes a vector (4 words) at its first argument from the
    // other two
    magic_owna::rec(&mut t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(magic_owna::row(229));
        for i in 0..4 {
            let v = 0.5 + f32::from(u16::try_from((a[2] + i + k) & 0xff).unwrap()) * 0.25;
            m.w32(a[0] + 4 * i, magic_owna::f(v));
        }
        0x0229_0000 ^ a[1]
    });
    let pts = SCRATCH + 0x1000;
    for n in [0i32, 1, 3, -2] {
        t.write(pts, &[0x5c; 0x40]);
        let c = magic::Call {
            eax: 0x00ea_00ea,
            ecx: 0x20,
            edx: pts,
            ..magic::Call::default()
        };
        t.check(
            at,
            c,
            &[n.cast_unsigned(), SCRATCH + 0x2000, 0x00aa_0010],
            magic::Eax::Checked,
            ffx_magic::m0267::transform_points,
        );
    }
}

/// 0x10003210 (internal): the scale matrix with the slots' floats.
#[test]
fn scale_matrix() {
    use magic::{SCRATCH, StubMem};
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x3210);
    magic_owna::rec(&mut t, 235, 1, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    for (s, v) in [(277u32, 2.5f64), (15, 0.5), (16, -3.0), (17, 1.25)] {
        magic_owna::rec(&mut t, s, 2, move |_, a| (0x0100_0000 + s) ^ a[0]);
        t.stub_st0(s, move |_, _| v);
    }
    t.set_u32(t.img.word(at + 0x1d), 0x00ab_001d);
    t.set_u32(t.img.word(at + 0x4e), 0x00ab_004e);
    let mat = SCRATCH + 0x1000;
    t.write(mat, &[0x5c; 0x40]);
    let c = magic::Call {
        edx: mat,
        ..magic::Call::default()
    };
    t.check(
        at,
        c,
        &[],
        magic::Eax::Checked,
        ffx_magic::m0267::scale_matrix,
    );
}

/// 0x10002910: the caster's position into the object (its script waiting,
/// its list empty); the objects busy, the motion running and the end.
#[test]
fn frame() {
    use magic::{SCRATCH, StubMem};
    let Some(mut t) = magic_owna::twin(N) else {
        return;
    };
    let at = t.at(0x2910);
    magic_owna::rec(&mut t, 262, 4, |m: &mut dyn StubMem, a: &[u32]| {
        for (i, at) in (0u32..).zip(&a[1..4]) {
            m.w32(*at, 0x4100_0000 + i);
        }
        0x262
    });
    for (s, n) in [(719u32, 3usize), (306, 3)] {
        magic_owna::rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    t.set_u32(t.img.word(at + 0x28), 0x00ab_0028);
    let obj = t.img.word(at + 0x57);
    let ms = t.img.rel32_target(at + 0x7a);
    let ro = t.img.rel32_target(at + 0x81);
    let list = t.img.word(ro + 0x7c);
    let (n, busy_obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, busy_obj);
    t.set_u16(busy_obj + 0x124, 0x8001);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    for (busy, done) in [(true, true), (false, false), (false, true)] {
        // the script waits (+0x2c) with an empty own list (+0x10)
        t.write(obj + 0x10, &[0; 0x24]);
        t.set_u32(obj + 0x2c, 5);
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check_entry(at, magic::Call::default(), &[task], magic::Eax::Checked);
    }
}
