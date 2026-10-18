//! `yiScreff`: the field's screen distortion (heat haze, under water): the
//! per-frame update of its 9 x 9 vertex grid (`docs/formats/field.md`).
//!
//! Each vertex gets texture coordinates offset by two sine waves (the
//! phases at `+0xbe080`/`+0xbe084` of the renderer, amplitude
//! `+0xbe098`), clamped to 0..1, and a colour: the tint `+0xbe0a4..` or,
//! while the effect fades in (`+0xbe078` bit 0), a mix toward grey by the
//! fade `t`.

use ffx_bridge::Mem;

use crate::camera::FArgs;
use crate::x87;

/// The `.rdata` constants (`f64`): 208 (the fade's screen half-height),
/// 64, 1/128, 2π, 3, 1/8, π, 52, 128, 1.5.
pub const K: [u32; 10] = [
    0x00b5_4c70,
    0x00b4_1308,
    0x00b1_70b8,
    0x00b6_7128,
    0x00b2_c5c8,
    0x00b9_2200,
    0x00b6_7118,
    0x00b5_f1e8,
    0x00b4_c170,
    0x00b4_1d00,
];
/// `FFXVu0InversMatrix(dst, src)` (`0x006ed450`).
pub const INVERS_MATRIX: u32 = 0x006e_d450;
/// `FFXVu0ApplyMatrix(dst, m, v)` (`0x006ed2e0`).
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `graphicSetDistortionBlur(level)` (`0x00642f30`).
pub const SET_BLUR: u32 = 0x0064_2f30;
/// `graphicUpdateDistortionVertex(row, col, r, g, b, a, u, v)`
/// (`0x00644940`).
pub const UPDATE_VERTEX: u32 = 0x0064_4940;
/// `_CIfmod` (`0x009498e0`) and `_CIsin` (`0x009498c8`): the CRT's x87
/// intrinsics.
pub const CI_FMOD: u32 = 0x0094_98e0;
/// See [`CI_FMOD`].
pub const CI_SIN: u32 = 0x0094_98c8;

/// The scratch `yiScreff_draw` hands its matrix calls: a matrix, a
/// vector and the result (0x60 bytes).
pub const WORK: u32 = 0x60;

/// `fistp` under the round-toward-zero control word the original sets:
/// the low 32 bits of the 64-bit truncation (the integer indefinite, whose
/// low half is 0, outside `i64`).
fn chop(x: f64) -> u32 {
    ffx_bridge::x87::ftol2(x)
}

/// `yiScreff_draw(scene)` (`0x00913200`, name ours; `yiScreff_entry`'s
/// body): with the effect fading in (`+0xbe078` bit 0) the fade line from
/// the inverted camera matrix (`+0x67f78`) and the depth `+0xbe07c`; the
/// blur level `+0xbe0a0` / 128 (`graphicSetDistortionBlur`); then the 81
/// vertices, row by row, to `graphicUpdateDistortionVertex`; the frame
/// count `+0xbe0a8` and the phases move on (`+0xbe080` by 1.5 times
/// `+0xbe090`, `+0xbe084` by it). `work` is [`WORK`] bytes. Returns the
/// last vertex call's `eax`.
#[allow(clippy::too_many_lines)] // the two row kinds, then the vertex
pub fn yi_screff_draw<M, C, F>(
    mem: &mut M,
    scene: u32,
    work: u32,
    call: &mut C,
    fcall: &mut F,
) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
    F: FnMut(&mut M, u32, FArgs<'_>) -> f64,
{
    let at = |o: u32| scene.wrapping_add(o);
    let k = |mem: &M, i: usize| {
        let a = mem.rebase(K[i]);
        mem.read::<f64>(a)
    };
    let (half_h, sixty_four, eighth128, two_pi, three, eighth, pi, fifty_two, c128, one_half) = (
        k(mem, 0),
        k(mem, 1),
        k(mem, 2),
        k(mem, 3),
        k(mem, 4),
        k(mem, 5),
        k(mem, 6),
        k(mem, 7),
        k(mem, 8),
        k(mem, 9),
    );
    let (ld, st) = (x87::load, x87::store);
    let f = |mem: &M, o: u32| ld(mem.read::<u32>(at(o)));
    // an integer through `fild`, stored as f32
    let int = |v: i32| st(f64::from(v));
    let fmod = |mem: &mut M, fcall: &mut F, x: f64| {
        let ff = mem.rebase(CI_FMOD);
        st(fcall(mem, ff, FArgs::St01(x, two_pi)))
    };
    let sin = |mem: &mut M, fcall: &mut F, x: u32| {
        let fs = mem.rebase(CI_SIN);
        st(fcall(mem, fs, FArgs::St0(ld(x))))
    };

    // the fade line: where the camera's view of the plane at the depth lies
    let mut fade = 0u32;
    if mem.read::<u8>(at(0xbe078)) & 1 != 0 {
        let (m, v, out) = (work, work.wrapping_add(0x40), work.wrapping_add(0x50));
        for i in 0..16 {
            let w = mem.read::<u32>(at(0x67f78 + 4 * i));
            mem.write(m.wrapping_add(4 * i), w);
        }
        let depth = st(-f(mem, 0xbe07c));
        for (i, w) in (0u32..).zip([0, depth, 0x3f80_0000, 0x3f80_0000]) {
            mem.write(v.wrapping_add(4 * i), w);
        }
        let fi = mem.rebase(INVERS_MATRIX);
        call(mem, fi, &[m, m]);
        let fa = mem.rebase(APPLY_MATRIX);
        call(mem, fa, &[out, m, v]);
        let y = ld(mem.read::<u32>(out.wrapping_add(4)));
        let scale = st(half_h / f(mem, 0x68360) * sixty_four);
        fade = st(y / ld(scale) * half_h - half_h);
    }
    let level = int(mem.read::<i32>(at(0xbe0a0)));
    let blur = mem.rebase(SET_BLUR);
    call(mem, blur, &[st(ld(level) * eighth128)]);

    let speed = f(mem, 0xbe094);
    let amp = f(mem, 0xbe098);
    let step = st(speed * sixty_four);
    // carried across rows and columns as the original's frame keeps them;
    // the top row never sets the fade `t`, so its waves are 0 there
    let (mut t, mut phase_a, mut phase_b) = (0u32, 0u32, 0u32);
    let mut eax = 0;
    // one wave phase step, wrapped past pi
    let advance = |p: u32| {
        let p = st(ld(step) + ld(p));
        if pi < ld(p) { st(ld(p) - two_pi) } else { p }
    };
    for row in 0..9u32 {
        let ph0 = f(mem, 0xbe080);
        let ph1 = f(mem, 0xbe084);
        #[allow(clippy::cast_possible_wrap)] // 0..9
        let row_i = row as i32;
        // the first row samples the waves at 0, row n (n > 0) at 64n + 64;
        // the original also evaluates row n at 64n first, unused
        let row_f = ld(int(if row == 0 { 0 } else { row_i * 64 + 64 }));
        let (speed3, ph0_3) = (speed * three, -ph0 * three);
        let ang_a = fmod(mem, fcall, ld(st(speed * row_f + ph0)));
        let ang_b = fmod(mem, fcall, ld(st(speed3 * row_f + ph0_3)));
        if row != 0 {
            // the column waves restart from the phase `+0xbe084`
            phase_a = fmod(mem, fcall, ph1);
            phase_b = fmod(mem, fcall, ld(st(-ph1 * three)));
        }
        let sin_a = sin(mem, fcall, ang_a);
        let u0 = st(ld(int(row_i)) * eighth);
        let sin_b = sin(mem, fcall, ang_b);
        for col in 0..9u32 {
            #[allow(clippy::cast_possible_wrap)] // 0..9
            let col_i = col as i32;
            let flags = mem.read::<u32>(at(0xbe078));
            if row != 0 {
                if flags & 1 == 0 {
                    t = 0x3f80_0000;
                } else {
                    if flags & 2 == 0 {
                        let x = st(ld(int(col_i)) * fifty_two);
                        t = st((ld(x) - -ld(fade)) * eighth128);
                    }
                    if ld(t) < 0.0 {
                        t = 0;
                    } else if 1.0 < ld(t) {
                        t = 0x3f80_0000;
                    }
                }
            }
            let mut u = st(ld(sin_a) * amp * ld(t) + ld(u0));
            let s3 = sin(mem, fcall, phase_a);
            let col8 = st(ld(int(col_i)) * eighth);
            let mut v = st(ld(s3) * amp * ld(t) + ld(col8));
            phase_a = advance(phase_a);
            u = st(ld(t) * (amp * ld(sin_b) / three) + ld(u));
            let s4 = if row == 0 {
                let x = ld(int(col_i * 52));
                let w = fmod(mem, fcall, ld(st(-ph1 * three + speed3 * x)));
                sin(mem, fcall, w)
            } else {
                let s4 = sin(mem, fcall, phase_b);
                phase_b = advance(phase_b);
                s4
            };
            v = st(ld(s4) * amp / three * ld(t) + ld(v));
            if ld(u) < 0.0 {
                u = 0;
            }
            if ld(v) < 0.0 {
                v = 0;
            }
            if 1.0 < ld(u) {
                u = 0x3f80_0000;
            }
            if 1.0 < ld(v) {
                v = 0x3f80_0000;
            }
            // the colour: the tint, or toward grey while fading in
            let tint = |mem: &M, i: u32| u32::from(mem.read::<u8>(at(0xbe0a4 + i)));
            let fading = if row == 0 {
                mem.read::<u8>(at(0xbe078)) & 1 != 0
            } else {
                flags & 1 != 0
            };
            let rgba = if fading && ld(t) < 1.0 {
                let mix = |c: u32| chop((f64::from(c) - c128) * ld(t) + c128);
                [
                    mix(tint(mem, 0)),
                    mix(tint(mem, 1)),
                    mix(tint(mem, 2)),
                    chop(ld(t) * c128),
                ]
            } else {
                [tint(mem, 0), tint(mem, 1), tint(mem, 2), 0x80]
            };
            // each as unsigned through fild (+2^32 when negative), times 1/128
            let ch = |c: u32| st(ld(st(f64::from(c))) * eighth128);
            let fv = mem.rebase(UPDATE_VERTEX);
            eax = call(
                mem,
                fv,
                &[
                    row,
                    col,
                    ch(rgba[0]),
                    ch(rgba[1]),
                    ch(rgba[2]),
                    ch(rgba[3]),
                    u,
                    v,
                ],
            );
        }
    }
    let n = mem.read::<u32>(at(0xbe0a8)).wrapping_add(1);
    mem.write(at(0xbe0a8), n);
    let speed2 = f(mem, 0xbe090);
    let ph0 = st(speed2 * one_half + f(mem, 0xbe080));
    mem.write(at(0xbe080), ph0);
    let ph1 = st(f(mem, 0xbe084) + f(mem, 0xbe090));
    mem.write(at(0xbe084), ph1);
    eax
}
