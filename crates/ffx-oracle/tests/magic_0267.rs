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
