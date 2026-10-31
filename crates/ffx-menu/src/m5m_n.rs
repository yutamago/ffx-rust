//! M5 stage 1, the menu, battle and script leftovers (round 5): the battle
//! debug windows' helpers (`Ms_Gui_CameraProc`, `Ms_Gui_MapDataProc`,
//! `Ms_Gui_ReferencePos`): their value printers, slider callbacks and
//! camera glue; `Rng_ClockSeed`; the script's `Native_01ed_DebugHang`;
//! the RCUI fade rectangle `0x0092_cfa0`.

#![allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

pub mod bridge {
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::{Conv, Host};

    pub use ffx_battle::pos::bridge::Trig;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// A CRT import (cdecl) through its slot.
    fn crt(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// An `f32` as `fld dword` / `fstp dword` copy it.
    fn fcopy(bits: u32) -> u32 {
        x87::store(x87::load(bits))
    }

    fn f32_at(h: &impl Host, p: u32) -> f64 {
        x87::load(h.read(p))
    }

    fn f64_at(h: &impl Host, sym: u32) -> f64 {
        h.read(h.rebase(sym))
    }

    fn g32(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn set32(h: &mut impl Host, sym: u32, v: u32) {
        let a = h.rebase(sym);
        h.write(a, v);
    }

    /// `x` truncated by `_ftol2_sse` and loaded back by `fild dword` then
    /// stored as `f32`: (the integer, the `f32`).
    fn trunc_f32(h: &impl Host, x: f64) -> (u32, u32) {
        let i = x87::ftol(h, x);
        (i, x87::store(f64::from(i.cast_signed())))
    }

    pub const SPRINTF: u32 = 0x00b0_c3dc;
    pub const GU_PRINT_SET_X: u32 = 0x0084_8820;
    pub const GU_PRINT_SET_Y: u32 = 0x0084_8860;
    pub const GU_PRINTF: u32 = 0x0084_8880;
    /// `"%s\n"`, `Gu_Printf`'s format for a line.
    pub const FMT_LINE: u32 = 0x00b1_2ad0;
    /// `"%.4f"`.
    pub const FMT_VALUE: u32 = 0x00b5_6334;
    /// `"------"`.
    pub const DASHES: u32 = 0x00b5_6750;
    /// `f64` pi and 180 (`.rdata`).
    pub const PI_F64: u32 = 0x00b9_2288;
    pub const DEG_180: u32 = 0x00b4_8378;
    /// `f64` 0.1.
    pub const TENTH: u32 = 0x00b5_51e0;

    // ---- the value printers ----

    /// A label's string in `.rdata`.
    pub const S_X: u32 = 0x00b8_f800;
    pub const S_Y: u32 = 0x00b8_f804;
    pub const S_Z: u32 = 0x00b8_f808;
    pub const S_THETA: u32 = 0x00b5_6370;
    pub const S_PHI: u32 = 0x00b5_6384;
    pub const S_LEN: u32 = 0x00b5_6388;
    pub const S_SPD: u32 = 0x00b5_6768;
    pub const S_PHIPR: u32 = 0x00b5_6758;
    pub const S_LENPR: u32 = 0x00b5_6760;
    pub const S_COUNT: u32 = 0x00b5_676c;
    pub const S_ADD: u32 = 0x00b5_6774;

    /// A field the printers show: an `f32` as it is, or an angle in
    /// radians shown in degrees (`x * 180 / pi`, at 53 bits).
    #[derive(Debug, Clone, Copy)]
    pub enum Value {
        F32(u32),
        Deg(u32),
    }

    /// One printed row, 10 pixels below the last: a label at x 0 and the
    /// value (`"%.4f"`) at x 0x5c, or a dash line at x 0x5c.
    #[derive(Debug, Clone, Copy)]
    pub enum Row {
        Pair(u32, Value),
        Dashes,
    }

    use Row::{Dashes, Pair};
    use Value::{Deg, F32};

    /// The row at `y`: `Gu_PrintSetX(x)`, `Gu_PrintSetY(y)`, the text
    /// `sprintf`ed into `buf` (the original's `ebp - 0x104`) and
    /// `Gu_Printf("%s\n", buf)`.
    fn print_at(h: &mut impl Host, buf: u32, x: u32, y: u32, sprintf: &[u32]) {
        cdecl(h, GU_PRINT_SET_X, &[x]);
        cdecl(h, GU_PRINT_SET_Y, &[y]);
        let mut args = vec![buf];
        args.extend_from_slice(sprintf);
        crt(h, SPRINTF, &args);
        let fmt = h.rebase(FMT_LINE);
        cdecl(h, GU_PRINTF, &[fmt, buf]);
    }

    /// A label (its own `sprintf` format, no arguments).
    fn print_label(h: &mut impl Host, buf: u32, x: u32, y: u32, label: u32) {
        let s = h.rebase(label);
        print_at(h, buf, x, y, &[s]);
    }

    /// `"%.4f"` of a double.
    fn print_value(h: &mut impl Host, buf: u32, y: u32, v: f64) {
        let bits = v.to_bits();
        let fmt = h.rebase(FMT_VALUE);
        print_at(h, buf, 0x5c, y, &[fmt, bits as u32, (bits >> 32) as u32]);
    }

    /// A field as the printers pass it to `sprintf` (`fstp qword`).
    fn value(h: &impl Host, obj: u32, v: Value) -> f64 {
        match v {
            F32(o) => f32_at(h, at(obj, o)),
            Deg(o) => x87::div(
                x87::mul(f32_at(h, at(obj, o)), f64_at(h, DEG_180)),
                f64_at(h, PI_F64),
            ),
        }
    }

    /// The printers' body: `rows` from `y + 10` on, 10 apart. Returns the
    /// last row's `y` (what the original leaves in `eax`).
    pub fn print_rows(h: &mut impl Host, buf: u32, obj: u32, y: u32, rows: &[Row]) -> u32 {
        let mut y = y;
        for &r in rows {
            y = y.wrapping_add(10);
            match r {
                Pair(label, v) => {
                    print_label(h, buf, 0, y, label);
                    let x = value(h, obj, v);
                    print_value(h, buf, y, x);
                }
                Dashes => print_label(h, buf, 0x5c, y, DASHES),
            }
        }
        y
    }

    /// `MsDebugPrintCamera_007d0c90(_, obj, _, y)` (name ours): THETA, PHI
    /// (degrees), LEN, SPD (`+0x40`, `+0x44`, `+0x48`, `+0x4c`), two dash
    /// lines.
    pub const PRINT_CAMERA: [Row; 6] = [
        Pair(S_THETA, Deg(0x40)),
        Pair(S_PHI, Deg(0x44)),
        Pair(S_LEN, F32(0x48)),
        Pair(S_SPD, F32(0x4c)),
        Dashes,
        Dashes,
    ];
    /// `MsDebugPrintPolarXz_007d14f0(_, obj, _, y)` (name ours): X, Z
    /// (`+0x30`, `+0x38`), PHI (degrees, `+0x44`), PHIPR, LEN, LENPR
    /// (`+0x54`, `+0x48`, `+0x58`).
    pub const PRINT_POLAR_XZ: [Row; 6] = [
        Pair(S_X, F32(0x30)),
        Pair(S_Z, F32(0x38)),
        Pair(S_PHI, Deg(0x44)),
        Pair(S_PHIPR, F32(0x54)),
        Pair(S_LEN, F32(0x48)),
        Pair(S_LENPR, F32(0x58)),
    ];
    /// `MsDebugPrintPolarXyz_007d1940(_, _, obj, _, y)` (name ours): X, Y,
    /// Z (`+0x30..+0x38`), PHIPR, LEN, LENPR (`+0x54`, `+0x48`, `+0x58`).
    pub const PRINT_POLAR_XYZ: [Row; 6] = [
        Pair(S_X, F32(0x30)),
        Pair(S_Y, F32(0x34)),
        Pair(S_Z, F32(0x38)),
        Pair(S_PHIPR, F32(0x54)),
        Pair(S_LEN, F32(0x48)),
        Pair(S_LENPR, F32(0x58)),
    ];
    /// `MsDebugSplinePrint(_, obj, _, y)`: count, add (`+0x68`, `+0x6c`),
    /// four dash lines.
    pub const PRINT_SPLINE: [Row; 6] = [
        Pair(S_COUNT, F32(0x68)),
        Pair(S_ADD, F32(0x6c)),
        Dashes,
        Dashes,
        Dashes,
        Dashes,
    ];
    /// `MsDebugPrintTheta_007d6530(_, obj, _, y)` (name ours): THETA
    /// (degrees, `+0x40`), five dash lines.
    pub const PRINT_THETA: [Row; 6] = [
        Pair(S_THETA, Deg(0x40)),
        Dashes,
        Dashes,
        Dashes,
        Dashes,
        Dashes,
    ];
    /// `MsDebugPrintKeepXz_007d6850(_, obj, _, y)` (name ours): THETA
    /// (degrees, `+0x50`), X, Z (`+0x30`, `+0x38`), three dash lines.
    pub const PRINT_KEEP_XZ: [Row; 6] = [
        Pair(S_THETA, Deg(0x50)),
        Pair(S_X, F32(0x30)),
        Pair(S_Z, F32(0x38)),
        Dashes,
        Dashes,
        Dashes,
    ];
    /// `MsDebugPrintCamera3_007d6bb0(_, obj, _, y)` (name ours): THETA,
    /// PHI (degrees), LEN, three dash lines.
    pub const PRINT_CAMERA3: [Row; 6] = [
        Pair(S_THETA, Deg(0x40)),
        Pair(S_PHI, Deg(0x44)),
        Pair(S_LEN, F32(0x48)),
        Dashes,
        Dashes,
        Dashes,
    ];
    /// `MsDebugPrintXyz_007d6ed0(_, obj, _, y)` and
    /// `MsDebugPrintXyz_007dacd0(obj, _, y)` (names ours): X, Y, Z
    /// (`+0x30..+0x38`), three dash lines.
    pub const PRINT_XYZ: [Row; 6] = [
        Pair(S_X, F32(0x30)),
        Pair(S_Y, F32(0x34)),
        Pair(S_Z, F32(0x38)),
        Dashes,
        Dashes,
        Dashes,
    ];

    pub const S_ACT1: u32 = 0x00b5_69c8;
    pub const S_ACT2: u32 = 0x00b5_69e4;
    pub const S_NULL: u32 = 0x00b5_69d0;
    pub const S_ACTOR: u32 = 0x00b5_69dc;
    pub const S_YOFS: u32 = 0x00b5_69ec;
    /// `g_DebugRefActorCount` (`0x0113_f86c`, name ours): the second number
    /// the reference window shows beside each actor.
    pub const REF_ACTOR_COUNT: u32 = 0x0113_f86c;

    /// `MsDebugPrintActors_007d9700(obj, _, y)` (name ours): ACT1 and ACT2
    /// (`u16 +0xc`, `+0xe`: `"NULL:%3d"` for 0xffff, else `"%4d:%3d"`, the
    /// second number `0x0113_f86c`), Yofs (`+0x34`); then `+0x30`, `+0x38`,
    /// `+0x3c` zeroed and three dash lines. Returns the last row's `y`.
    pub fn print_actors(h: &mut impl Host, buf: u32, obj: u32, y: u32) -> u32 {
        let mut y = y;
        for (label, o) in [(S_ACT1, 0xcu32), (S_ACT2, 0xe)] {
            y = y.wrapping_add(10);
            print_label(h, buf, 0, y, label);
            let n = g32(h, REF_ACTOR_COUNT);
            let v = u32::from(h.read::<u16>(at(obj, o)));
            if v == 0xffff {
                let fmt = h.rebase(S_NULL);
                print_at(h, buf, 0x5c, y, &[fmt, n]);
            } else {
                let fmt = h.rebase(S_ACTOR);
                print_at(h, buf, 0x5c, y, &[fmt, v, n]);
            }
        }
        y = y.wrapping_add(10);
        print_label(h, buf, 0, y, S_YOFS);
        let v = f32_at(h, at(obj, 0x34));
        print_value(h, buf, y, v);
        for o in [0x30u32, 0x38, 0x3c] {
            h.write(at(obj, o), 0u32);
        }
        for _ in 0..3 {
            y = y.wrapping_add(10);
            print_label(h, buf, 0x5c, y, DASHES);
        }
        y
    }

    // ---- the slider callbacks ----

    /// How a slider steps a field: by `p * 0.1`, by `p * 0.1 * pi / 180`
    /// (an angle) or by `p`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Step {
        Tenth,
        TenthRad,
        Plain,
    }

    /// What id 0x33 does to a field: truncates it to an integer, or (an
    /// angle) to whole degrees.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Round {
        Int,
        Deg,
    }

    /// `MsDebugSliderPolarXz_007d1300(obj, id, p)` (name ours): ids
    /// 0x27..0x32 step `+0x30`, `+0x38`, the angle `+0x44`, `+0x54` (by
    /// `p`), `+0x48`, `+0x58` (pairs: minus, plus).
    pub const SLIDE_POLAR_XZ: [(u32, Step); 6] = [
        (0x30, Step::Tenth),
        (0x38, Step::Tenth),
        (0x44, Step::TenthRad),
        (0x54, Step::Plain),
        (0x48, Step::Tenth),
        (0x58, Step::Tenth),
    ];
    /// Its id 0x33.
    pub const ROUND_POLAR_XZ: [(u32, Round); 7] = [
        (0x30, Round::Int),
        (0x34, Round::Int),
        (0x38, Round::Int),
        (0x44, Round::Deg),
        (0x54, Round::Int),
        (0x48, Round::Int),
        (0x58, Round::Int),
    ];
    /// `MsDebugSliderPolarXyz_007d1790(obj, id, p)` (name ours): `+0x30`,
    /// `+0x34`, `+0x38`, `+0x54` (by `p`), `+0x48`, `+0x58`.
    pub const SLIDE_POLAR_XYZ: [(u32, Step); 6] = [
        (0x30, Step::Tenth),
        (0x34, Step::Tenth),
        (0x38, Step::Tenth),
        (0x54, Step::Plain),
        (0x48, Step::Tenth),
        (0x58, Step::Tenth),
    ];
    /// Its id 0x33.
    pub const ROUND_POLAR_XYZ: [(u32, Round); 6] = [
        (0x30, Round::Int),
        (0x34, Round::Int),
        (0x38, Round::Int),
        (0x54, Round::Int),
        (0x48, Round::Int),
        (0x58, Round::Int),
    ];

    /// `p * 0.1 * pi / 180` as the sliders compute it.
    fn tenth_rad(h: &impl Host, p: u32) -> f64 {
        x87::div(
            x87::mul(x87::mul(x87::load(p), f64_at(h, TENTH)), f64_at(h, PI_F64)),
            f64_at(h, DEG_180),
        )
    }

    /// A field at `+o` stepped down (`sub`) or up by `step` of `p`.
    fn slide(h: &mut impl Host, obj: u32, o: u32, step: Step, sub: bool, p: u32) {
        let f = f32_at(h, at(obj, o));
        let v = match step {
            Step::Plain => {
                if sub {
                    x87::sub(f, x87::load(p))
                } else {
                    x87::add(f, x87::load(p))
                }
            }
            Step::Tenth | Step::TenthRad => {
                let d = if step == Step::Tenth {
                    x87::mul(x87::load(p), f64_at(h, TENTH))
                } else {
                    tenth_rad(h, p)
                };
                if sub { x87::sub(f, d) } else { x87::add(d, f) }
            }
        };
        h.write(at(obj, o), x87::store(v));
    }

    /// An angle at `+o` rounded to whole degrees: `f = (float)(int)(x *
    /// 180 / pi)` (the integer and then `f` in the id's argument slot),
    /// stored back as `f * pi / 180`. Returns the integer.
    fn round_deg(h: &mut impl Host, obj: u32, o: u32, slot: u32) -> u32 {
        let pi = f64_at(h, PI_F64);
        let deg = f64_at(h, DEG_180);
        let x = x87::div(x87::mul(f32_at(h, at(obj, o)), deg), pi);
        let (i, f) = trunc_f32(h, x);
        h.write(slot, f);
        let v = x87::div(x87::mul(pi, x87::load(f)), deg);
        h.write(at(obj, o), x87::store(v));
        i
    }

    /// The table sliders: ids 0x27..0x32 step `steps` (two ids a field,
    /// minus first), 0x33 rounds `rounds` in order (each integer in the
    /// id's argument slot, `slots + 4`). Returns `eax` as the original
    /// leaves it: `obj` after a step, the last integer after 0x33, else
    /// `id - 0x27`.
    pub fn slider(
        h: &mut impl Host,
        obj: u32,
        id: u32,
        p: u32,
        slots: u32,
        steps: &[(u32, Step)],
        rounds: &[(u32, Round)],
    ) -> u32 {
        let k = id.wrapping_sub(0x27);
        if k == 0xc {
            let slot = at(slots, 4);
            let mut eax = 0;
            for &(o, r) in rounds {
                eax = match r {
                    Round::Int => {
                        let (i, f) = trunc_f32(h, f32_at(h, at(obj, o)));
                        h.write(slot, i);
                        h.write(at(obj, o), f);
                        i
                    }
                    Round::Deg => round_deg(h, obj, o, slot),
                };
            }
            return eax;
        }
        match steps.get((k / 2) as usize) {
            Some(&(o, step)) if k < 0xc => {
                slide(h, obj, o, step, k.is_multiple_of(2), p);
                obj
            }
            _ => k,
        }
    }

    /// `f64` 100, 0.001 and 0.01; `f32` 5.
    pub const HUNDRED: u32 = 0x00b9_22e0;
    pub const THOUSANDTH: u32 = 0x00b9_21b0;
    pub const HUNDREDTH: u32 = 0x00b5_4cb8;
    pub const FIVE: u32 = 0x00b3_eb98;
    /// The zoom steps (7 `i32`, ascending in the game's data).
    pub const ZOOM_STEPS: u32 = 0x00c4_48b8;

    /// A zoom field (`i16`) one step down (`up` false) or up: by 1 for
    /// `p <= 5` (or NaN), else to the next of [`ZOOM_STEPS`] below / above
    /// it, wrapping to the last / first.
    fn zoom_step(h: &mut impl Host, field: u32, up: bool, p: f64) {
        let five = f32_at(h, h.rebase(FIVE));
        let v: i16 = h.read(field);
        // by one for p <= 5 and NaN
        let by_table = five < p;
        if !by_table {
            h.write(
                field,
                if up {
                    v.wrapping_add(1)
                } else {
                    v.wrapping_sub(1)
                },
            );
            return;
        }
        let steps = h.rebase(ZOOM_STEPS);
        let step = |h: &mut _, i: u32| -> i32 { Mem::read(h, at(steps, 4 * i)) };
        let v = i32::from(v);
        let found = if up {
            (0..7).find(|&i| v < step(h, i))
        } else {
            (0..7).rev().find(|&i| v > step(h, i))
        };
        let i = found.unwrap_or(if up { 0 } else { 6 });
        let w: u16 = h.read(at(steps, 4 * i));
        h.write(field, w);
    }

    /// `MsDebugSliderReference_007d1bd0(kind, obj, id, p)` (name ours): ids
    /// 0x27..0x2a step the zooms `i16 +0x10`, `+0x12` ([`zoom_step`]),
    /// 0x2b/0x2c the `f32 +0x5c` by `-/+ p * 0.001` (not for kinds 0xf,
    /// 0x10), 0x33 rounds it to hundredths (`(float)(int)(x * 100) / 100`,
    /// the `f32` in the id's slot). Then, unless kind 0xb, 0x2d..0x30 step
    /// the angles `+0x40`, `+0x44` by `-/+ p * 0.1 * pi / 180`, 0x31/0x32
    /// `+0x60` by `-/+ p * 0.01`, and 0x33 rounds the angles to whole
    /// degrees and `+0x60` to hundredths. Callers drop `eax`.
    pub fn slider_reference(h: &mut impl Host, kind: u32, obj: u32, id: u32, p: u32, slots: u32) {
        let slot = at(slots, 8);
        let pf = x87::load(p);
        let hundred = f64_at(h, HUNDRED);
        let hundredths = |h: &mut _, o: u32| {
            let x = x87::mul(f32_at(h, at(obj, o)), hundred);
            let (_, f) = trunc_f32(h, x);
            Mem::write(h, slot, f);
            Mem::write(h, at(obj, o), x87::store(x87::div(x87::load(f), hundred)));
        };
        match id {
            0x27..=0x2a => {
                let o = if id < 0x29 { 0x10 } else { 0x12 };
                zoom_step(h, at(obj, o), id & 1 == 0, pf);
            }
            0x2b | 0x2c if kind != 0xf && kind != 0x10 => {
                let d = x87::mul(pf, f64_at(h, THOUSANDTH));
                let f = f32_at(h, at(obj, 0x5c));
                let v = if id == 0x2b {
                    x87::sub(f, d)
                } else {
                    x87::add(d, f)
                };
                h.write(at(obj, 0x5c), x87::store(v));
            }
            0x33 => hundredths(h, 0x5c),
            _ => {}
        }
        if kind == 0xb {
            return;
        }
        match id {
            0x2d..=0x30 => {
                let o = if id < 0x2f { 0x40 } else { 0x44 };
                let d = tenth_rad(h, p);
                let f = f32_at(h, at(obj, o));
                let v = if id & 1 == 1 {
                    x87::sub(f, d)
                } else {
                    x87::add(d, f)
                };
                h.write(at(obj, o), x87::store(v));
            }
            0x31 | 0x32 => {
                let d = x87::mul(pf, f64_at(h, HUNDREDTH));
                let f = f32_at(h, at(obj, 0x60));
                let v = if id == 0x31 {
                    x87::sub(f, d)
                } else {
                    x87::add(d, f)
                };
                h.write(at(obj, 0x60), x87::store(v));
            }
            0x33 => {
                round_deg(h, obj, 0x40, slot);
                round_deg(h, obj, 0x44, slot);
                hundredths(h, 0x60);
            }
            _ => {}
        }
    }

    // ---- the camera glue ----

    /// `MsDebugCameraPolar_007d1030(x, dx, len, ang)` (name ours): the
    /// angle `*ang` (degrees) to radians `t`; `*x += dx`; then `a =
    /// atan((*x) / (cos t * *len))`, `*len = cos t * *len / cos a` and
    /// `*ang = a` in degrees. Each step goes through the argument slots as
    /// `f32` (the `x`, `len` and `ang` slots are left holding `a`, `cos t *
    /// len` and `*x`). `trig` is the CRT's `_CIcos` / `_CIatan` on the x87
    /// value, returning what `fstp dword` stores. Callers drop `eax`.
    pub fn camera_polar(
        h: &mut impl Host,
        trig: &mut dyn FnMut(Trig, f64) -> u32,
        x: u32,
        dx: u32,
        len: u32,
        ang: u32,
        slots: u32,
    ) {
        let pi = f64_at(h, PI_F64);
        let deg = f64_at(h, DEG_180);
        let (s_x, s_len, s_ang) = (slots, at(slots, 8), at(slots, 0xc));
        let t = x87::store(x87::div(x87::mul(f32_at(h, ang), pi), deg));
        h.write(s_ang, t);
        let c = trig(Trig::Cos, x87::load(t));
        h.write(s_ang, c);
        let r = x87::store(x87::mul(x87::load(c), f32_at(h, len)));
        h.write(s_len, r);
        let nx = x87::store(x87::add(f32_at(h, x), x87::load(dx)));
        h.write(s_ang, nx);
        let a = trig(Trig::Atan, x87::div(x87::load(nx), x87::load(r)));
        h.write(s_x, a);
        h.write(x, nx);
        let c2 = trig(Trig::Cos, x87::load(a));
        h.write(s_ang, c2);
        h.write(len, x87::store(x87::div(x87::load(r), x87::load(c2))));
        h.write(ang, x87::store(x87::div(x87::mul(x87::load(a), deg), pi)));
    }

    /// The camera keeps (`0x30`-byte records from `0x0113_ed8c`, current
    /// `0x0113_ed5c`) and the window's shown values.
    pub const KEEP_CUR: u32 = 0x0113_ed5c;
    pub const KEEPS: u32 = 0x0113_ed8c;

    /// `MsDebugCameraKeepLoad_007d7130()` (name ours): the current keep's
    /// angle (`+0xc` to `0x0113_ed74`), its two angles in degrees (`+0x10`,
    /// `+0x14` times 180 / pi to `0x0113_ed68`, `0x0113_ed6c`), the distance
    /// (`+0x18` to `0x0113_ed70`), `sin(0x0113_ed6c * pi / 180) * distance`
    /// to `0x0113_f3ac` (`trig`: the CRT's `_CIsin`), the roll (`+0x20` to
    /// `0x0113_ed64`); then the 0x18c words from `0x0113_ed7c` copied to
    /// `0x0113_e668`. Callers drop `eax`.
    pub fn camera_keep_load(h: &mut impl Host, trig: &mut dyn FnMut(Trig, f64) -> u32) {
        let cur = g32(h, KEEP_CUR);
        let k = at(h.rebase(KEEPS), cur.wrapping_mul(0x30));
        let pi = f64_at(h, PI_F64);
        let deg = f64_at(h, DEG_180);
        let v = fcopy(h.read(at(k, 0xc)));
        set32(h, 0x0113_ed74, v);
        let a = x87::store(x87::div(x87::mul(f32_at(h, at(k, 0x10)), deg), pi));
        set32(h, 0x0113_ed68, a);
        let b = x87::store(x87::div(x87::mul(f32_at(h, at(k, 0x14)), deg), pi));
        set32(h, 0x0113_ed6c, b);
        let d = fcopy(h.read(at(k, 0x18)));
        set32(h, 0x0113_ed70, d);
        let b = g32(h, 0x0113_ed6c);
        let s = trig(Trig::Sin, x87::div(x87::mul(pi, x87::load(b)), deg));
        let d = g32(h, 0x0113_ed70);
        set32(
            h,
            0x0113_f3ac,
            x87::store(x87::mul(x87::load(s), x87::load(d))),
        );
        let r = fcopy(h.read(at(k, 0x20)));
        set32(h, 0x0113_ed64, r);
        let (src, dst) = (h.rebase(0x0113_ed7c), h.rebase(0x0113_e668));
        for i in 0..0x18cu32 {
            let w: u32 = h.read(at(src, 4 * i));
            h.write(at(dst, 4 * i), w);
        }
    }

    pub const MS_VECTOR_RECT2_POLAR: u32 = 0x007c_4990;
    pub const MS_DEBUG_SET_CAMERA_MODE: u32 = 0x007c_fb60;
    pub const MS_DEBUG_GET_CAMERA_MODE: u32 = 0x007c_fb20;
    /// The camera window's modes (13 `i32`) and the reference window's
    /// (5 `i32`).
    pub const CAMERA_MODES: u32 = 0x00c4_47f0;
    pub const REF_MODES: u32 = 0x00c4_4a10;
    /// `g_DebugCameraKeepAngleDeg` (`0x0113_ed78`).
    pub const KEEP_ANGLE_DEG: u32 = 0x0113_ed78;

    /// The next mode of a mode table of `n`: `mode`'s index (`n` when
    /// absent) plus `step`, or `pick` when `step` is 0; wrapped to `0..n`
    /// (past the end 0, below 0 the last).
    fn next_mode(h: &impl Host, table: u32, n: i32, mode: u32, step: u32, pick: u32) -> u32 {
        let t = h.rebase(table);
        let mut i = 0;
        while i < n && h.read::<u32>(at(t, 4 * i as u32)) != mode {
            i += 1;
        }
        let j = if step == 0 {
            pick as i32
        } else {
            i.wrapping_add(step as i32)
        };
        let j = if j >= n {
            0
        } else if j < 0 {
            n - 1
        } else {
            j
        };
        h.read(at(t, 4 * j as u32))
    }

    /// The camera window's saved camera (`0x0113_ed18..+0x24`, `0x0113_ed50`,
    /// `+0x54`, `+0x58`): the fields `+0x30..+0x3c`, `+0x40..+0x48`.
    const SAVED: [(u32, u32); 7] = [
        (0x30, 0x0113_ed18),
        (0x34, 0x0113_ed1c),
        (0x38, 0x0113_ed20),
        (0x3c, 0x0113_ed24),
        (0x40, 0x0113_ed50),
        (0x44, 0x0113_ed54),
        (0x48, 0x0113_ed58),
    ];

    fn saved_kind(mode: u32) -> bool {
        matches!(mode, 4 | 5 | 6 | 8 | 9)
    }

    /// `MsDebugCameraModeNext_007d42f0(mode, step, pick, obj, cam)` (name
    /// ours): in modes 4-6, 8, 9 the window's camera fields are saved,
    /// else they take the camera's `+0x20..+0x2c` and its polar form
    /// (`MsVectorRect2Polar` of `obj + 0x30` and `cam[+0x8c]` into `obj +
    /// 0x40`, `+0x44`, `+0x48`; the `mode` slot keeps `obj + 0x48`). Then the next of
    /// [`CAMERA_MODES`] is set (`MsDebugSetCameraMode(1, ..)`); in the new
    /// mode 8 the keep angle (`0x0113_ed78`, degrees) goes to `+0x40`, in 9
    /// to `+0x50`, both pointing `+8` at `0x0113_ed7c`; modes 4-6, 8, 9 get
    /// the saved fields back. Returns the new mode.
    pub fn camera_mode_next(
        h: &mut impl Host,
        mode: u32,
        step: u32,
        pick: u32,
        obj: u32,
        cam: u32,
        slots: u32,
    ) -> u32 {
        if saved_kind(mode) {
            for (o, sym) in SAVED {
                let v = fcopy(h.read(at(obj, o)));
                set32(h, sym, v);
            }
        } else {
            for o in [0x30u32, 0x34, 0x38, 0x3c] {
                let v = fcopy(h.read(at(cam, o - 0x10)));
                h.write(at(obj, o), v);
            }
            h.write(slots, at(obj, 0x48));
            let r: u32 = h.read(at(cam, 0x8c));
            cdecl(
                h,
                MS_VECTOR_RECT2_POLAR,
                &[
                    at(obj, 0x30),
                    r,
                    at(obj, 0x40),
                    at(obj, 0x44),
                    at(obj, 0x48),
                ],
            );
        }
        let m = next_mode(h, CAMERA_MODES, 13, mode, step, pick);
        cdecl(h, MS_DEBUG_SET_CAMERA_MODE, &[1, m]);
        let now = cdecl(h, MS_DEBUG_GET_CAMERA_MODE, &[1]);
        if now == 8 || now == 9 {
            let a = x87::div(
                x87::mul(f32_at(h, h.rebase(KEEP_ANGLE_DEG)), f64_at(h, PI_F64)),
                f64_at(h, DEG_180),
            );
            h.write(at(obj, if now == 8 { 0x40 } else { 0x50 }), x87::store(a));
            let keep = h.rebase(0x0113_ed7c);
            h.write(at(obj, 8), keep);
        }
        if saved_kind(now) {
            for (o, sym) in SAVED {
                let v = fcopy(g32(h, sym));
                h.write(at(obj, o), v);
            }
        }
        now
    }

    pub const MS_CAMERA_SET_ATEL_FOCUS: u32 = 0x007b_eeb0;
    /// `g_DebugRefActor` (`0x0113_f864`, name ours).
    pub const REF_ACTOR: u32 = 0x0113_f864;

    /// `MsDebugRefModeNext_007d9b00(mode, step, pick, obj, cam, base,
    /// which)` (name ours): the window's `+0x30..+0x3c` take the camera's
    /// `+0x20..+0x2c`; the next of [`REF_MODES`] is set
    /// (`MsDebugSetCameraMode(0, ..)`), the new mode kept in the `mode`
    /// slot. In mode 3 the actor (`0x0113_f864`, -1 as 0) and the window's
    /// two positions are taken (`MsCameraSetAtelFocus_007beeb0(which, 0, act1, act2, obj +
    /// 0x30)` with `+5` set, then again with `+0x34` the height above
    /// `base`, the `cam` slot holding `base`'s height first); else the
    /// fields take `base`'s `+0x30..+0x3c`. Returns the mode.
    #[allow(clippy::too_many_arguments)]
    pub fn ref_mode_next(
        h: &mut impl Host,
        mode: u32,
        step: u32,
        pick: u32,
        obj: u32,
        cam: u32,
        base: u32,
        which: u32,
        slots: u32,
    ) -> u32 {
        for o in [0x30u32, 0x34, 0x38, 0x3c] {
            let v = fcopy(h.read(at(cam, o - 0x10)));
            h.write(at(obj, o), v);
        }
        let m = next_mode(h, REF_MODES, 5, mode, step, pick);
        cdecl(h, MS_DEBUG_SET_CAMERA_MODE, &[0, m]);
        let now = cdecl(h, MS_DEBUG_GET_CAMERA_MODE, &[0]);
        h.write(slots, now);
        if now != 3 {
            for o in [0x30u32, 0x34, 0x38, 0x3c] {
                let v = fcopy(h.read(at(base, o)));
                h.write(at(obj, o), v);
            }
            return now;
        }
        let actor = g32(h, REF_ACTOR);
        let old: u8 = h.read(at(obj, 5));
        let y = fcopy(h.read(at(base, 0x34)));
        h.write(at(slots, 0x10), y);
        for o in [0x30u32, 0x34, 0x38, 0x3c] {
            h.write(at(obj, o), 0u32);
        }
        h.write(at(obj, 5), 1u8);
        set32(h, REF_ACTOR, if actor == u32::MAX { 0 } else { actor });
        let pos = |h: &mut _| {
            let a = u32::from(Mem::read::<u16>(h, at(obj, 0xc)));
            let b = u32::from(Mem::read::<u16>(h, at(obj, 0xe)));
            cdecl(
                h,
                MS_CAMERA_SET_ATEL_FOCUS,
                &[which, 0, a, b, at(obj, 0x30)],
            );
        };
        pos(h);
        let y2 = x87::store(x87::sub(x87::load(y), f32_at(h, at(base, 0x34))));
        h.write(at(obj, 0x34), y2);
        h.write(at(obj, 5), old);
        pos(h);
        now
    }

    pub const MS_CAMERA_GET_NUM: u32 = 0x007b_d340;
    pub const MS_CAMERA_GET_ID: u32 = 0x007b_d300;
    pub const MS_CAMERA_ADD_LINE_FILTER: u32 = 0x007b_f0b0;
    pub const MS_CAMERA_REF_ADD_CIRCLE_AREA: u32 = 0x007b_f1f0;
    /// `f64` angle of the reference arrow's head (`.rdata`).
    pub const ARROW_ANGLE: u32 = 0x00b1_1338;

    /// `MsDebugRefDraw_007d34a0(cam)` (name ours): the camera's channel
    /// (`MsCameraGetNum`) byte `0x0113_7634` cleared; while the scale
    /// `0x0113_ed48` is below 1 the reference's line from `(0x0113_ed28,
    /// 0x0113_ed30)` to `(0x0113_f3d4, 0x0113_f3dc)` both ways, the same line
    /// turned by [`ARROW_ANGLE`] both ways (`_CIcos`, `_CIsin` from `trig`)
    /// and a circle of `0x0113_ed2c` (`MsCameraAddLineFilter_007bf0b0`, `MsCameraRefAddCircleArea_007bf1f0` on
    /// `MsCameraGetID`; the `cam` slot is left holding the cosine); else
    /// (NaN too) the channel's five words `0x0113_76c0..` (0x20 apart)
    /// cleared. Callers drop `eax`.
    pub fn ref_draw(
        h: &mut impl Host,
        trig: &mut dyn FnMut(Trig, f64) -> u32,
        cam: u32,
        slots: u32,
    ) {
        let num = cdecl(h, MS_CAMERA_GET_NUM, &[cam]);
        let id = cdecl(h, MS_CAMERA_GET_ID, &[cam]);
        let ch = cam
            .wrapping_mul(0xda0)
            .wrapping_add(num.wrapping_mul(0x130));
        h.write(at(h.rebase(0x0113_7634), ch), 0u8);
        let scale = f32_at(h, h.rebase(0x0113_ed48));
        // the zero path for scale >= 1 and NaN
        let draw = scale < 1.0;
        if !draw {
            for i in 0..5u32 {
                h.write(at(h.rebase(0x0113_76c0), ch + 0x20 * i), 0u32);
            }
            return;
        }
        let g = |h: &_, sym: u32| fcopy(g32(h, sym));
        let line = |h: &mut _, a: [u32; 4]| {
            let s = fcopy(g32(h, 0x0113_ed48));
            cdecl(
                h,
                MS_CAMERA_ADD_LINE_FILTER,
                &[id, 0, 1, a[0], a[1], a[2], a[3], s],
            );
        };
        let (x0, z0, x1, z1) = (
            g(h, 0x0113_ed28),
            g(h, 0x0113_ed30),
            g(h, 0x0113_f3d4),
            g(h, 0x0113_f3dc),
        );
        line(h, [x0, z0, x1, z1]);
        let (x0, z0, x1, z1) = (
            g(h, 0x0113_ed28),
            g(h, 0x0113_ed30),
            g(h, 0x0113_f3d4),
            g(h, 0x0113_f3dc),
        );
        line(h, [x1, z1, x0, z0]);
        let angle = f64_at(h, ARROW_ANGLE);
        let c = trig(Trig::Cos, angle);
        h.write(slots, c);
        let dx = x87::sub(
            f32_at(h, h.rebase(0x0113_f3d4)),
            f32_at(h, h.rebase(0x0113_ed28)),
        );
        let s = trig(Trig::Sin, angle);
        let z1 = f32_at(h, h.rebase(0x0113_f3dc));
        let z0 = f32_at(h, h.rebase(0x0113_ed30));
        let dz = x87::sub(z1, z0);
        let x0 = f32_at(h, h.rebase(0x0113_ed28));
        let (c, s) = (x87::load(c), x87::load(s));
        let x = x87::store(x87::sub(x87::add(x87::mul(c, dx), x0), x87::mul(s, dz)));
        let z = x87::store(x87::add(x87::mul(c, dz), x87::add(x87::mul(s, dx), z0)));
        let (x0, z0) = (x87::store(x0), x87::store(z0));
        line(h, [x0, z0, x, z]);
        let (x0, z0) = (g(h, 0x0113_ed28), g(h, 0x0113_ed30));
        line(h, [x, z, x0, z0]);
        let s = g(h, 0x0113_ed48);
        let r = g(h, 0x0113_ed2c);
        cdecl(h, MS_CAMERA_REF_ADD_CIRCLE_AREA, &[id, 0, 1, r, r, s]);
    }

    // ---- the rest ----

    pub const RNG_READ_SYSTEM_CLOCK: u32 = 0x0062_f5b0;

    /// `Rng_ClockSeed()` (`0x0079_8950`): the PS2 clock bytes
    /// (`Rng_ReadSystemClock` into the 8-byte `clock`, the original's `ebp -
    /// 8`) combined by XOR, plus 1. The reader never writes bytes 0 and 4.
    pub fn clock_seed(h: &mut impl Host, clock: u32) -> u32 {
        cdecl(h, RNG_READ_SYSTEM_CLOCK, &[clock]);
        // junk read: [ebp-0x8] and [ebp-0x4] never written; 0 (brief 2026-10-24)
        // (the caller's frame is zeroed, so the reads below see 0 there)
        let b: [u8; 8] = h.read(clock);
        b.iter().fold(0u32, |x, &v| x ^ u32::from(v)) + 1
    }

    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const ATEL_POP_STACK_INTEGER: u32 = 0x0086_df00;
    pub const ATEL_PUSH_STACK_INTEGER: u32 = 0x0086_e3f0;
    /// The warning `Native_01ed_DebugHang` prints on every turn.
    pub const HANG_WARNING: u32 = 0x00b5_bbb8;

    /// `Native_01ed_DebugHang(worker, _, stack)` (`0x0085_7f70`): never
    /// returns (as the original): forever a warning (`dbgPrintf`), the
    /// stack's top integer popped and 2 and it pushed back. `turns` bounds
    /// the loop for the tests (`None` in the game).
    pub fn debug_hang(h: &mut impl Host, worker: u32, stack: u32, turns: Option<u32>) {
        let mut n = 0;
        while turns.is_none_or(|t| n < t) {
            let w = h.rebase(HANG_WARNING);
            cdecl(h, DBG_PRINTF, &[w]);
            let v = cdecl(h, ATEL_POP_STACK_INTEGER, &[worker, stack]);
            cdecl(h, ATEL_PUSH_STACK_INTEGER, &[worker, stack, 2]);
            cdecl(h, ATEL_PUSH_STACK_INTEGER, &[worker, stack, v]);
            n += 1;
        }
    }

    pub const G_RCUI_WORK: u32 = 0x0193_c0f0;
    pub const YN_DEPTH_TO_SCREEN: u32 = 0x0092_01a0;
    pub const RC_PACKET_DATA_INIT: u32 = 0x0092_7a20;
    pub const RC_PACKET_DATA_OPEN_ADR: u32 = 0x0092_7a70;
    pub const RC_PACKET_DATA_CLOSE_ADR: u32 = 0x0092_75b0;
    pub const RC_PACKET_DATA_DRAW: u32 = 0x0092_7600;
    /// The packet buffer `rcPacketDataOpenAdr` opens.
    pub const RC_PACKET_BUFFER: u32 = 0x00c4_98dc;
    /// `f32` 0.1, the rectangle's depth.
    pub const RECT_DEPTH: u32 = 0x00b1_0f38;

    /// `RcuiDrawScreenRect_0092cfa0()` (name ours; a draw callback stored
    /// at `0x0092_cf7f`): one GS packet over the screen (`g_RcuiWork`'s
    /// `+0xae4..+0xb04`: the TEX0 fields, the size and its pixel scale) at
    /// depth `Yn_DepthToScreen(0.1)`, through the packet at `ebp - 0x48`
    /// (`rcPacketDataInit`, `OpenAdr`, the 0x90 bytes written at its
    /// cursor `+4`, `CloseAdr`, `Draw`). `ebp` is the original's frame
    /// (0x50 bytes: the table of 13 pixel formats at `ebp - 0x38`, the
    /// locals as it leaves them). Returns `rcPacketDataDraw`'s `eax`.
    pub fn rcui_draw_screen_rect(h: &mut impl Host, ebp: u32) -> u32 {
        let pk = ebp.wrapping_sub(0x48);
        let fmts = ebp.wrapping_sub(0x38);
        let w = g32(h, G_RCUI_WORK);
        let table = [
            0u32, 1, 2, 10, 0x13, 0x14, 0x1b, 0x24, 0x2c, 0x30, 0x31, 0x32, 0x3a,
        ];
        h.write(fmts, table);
        let depth = fcopy(g32(h, RECT_DEPTH));
        let z = cdecl(h, YN_DEPTH_TO_SCREEN, &[depth]);
        h.write(ebp.wrapping_sub(0x4c), z);
        cdecl(h, RC_PACKET_DATA_INIT, &[pk]);
        let buf = g32(h, RC_PACKET_BUFFER);
        cdecl(h, RC_PACKET_DATA_OPEN_ADR, &[pk, buf]);
        let p: u32 = h.read(at(pk, 4));
        h.write(at(p, 8), [0x3fu32, 0]);
        let f = |h: &_, o: u32| -> u32 { Mem::read(h, at(w, o)) };
        let sx = |v: u32| u64::from(v) | (u64::from((v as i32 >> 31) as u32) << 32);
        let fmt: u32 = h.read(at(fmts, f(h, 0xaec).wrapping_mul(4)));
        let mut t = sx(f(h, 0xaf8)) << 7 | sx(f(h, 0xaf4)) | 0x8000_0010;
        t = t << 4 | sx(f(h, 0xaf0));
        t = t << 6 | u64::from(fmt);
        t = t << 6 | sx(f(h, 0xae8));
        t = t << 14 | sx(f(h, 0xae4));
        h.write(at(p, 0x10), [t as u32, (t >> 32) as u32, 6, 0]);
        h.write(
            at(p, 0x20),
            [
                0x44u32,
                0,
                0x42,
                0,
                0x8080_8080,
                0,
                1,
                0,
                0x3_0003,
                0,
                0x47,
                0,
            ],
        );
        let (cw, ch) = (f(h, 0xafc), f(h, 0xb00));
        let (pw, ph) = match f(h, 0xb04) {
            0 => (cw << 4, ch << 4),
            1 => (cw.wrapping_mul(0x18), ch.wrapping_mul(0x18)),
            2 => (cw << 5, ch << 5),
            _ => (0, 0),
        };
        h.write(ebp.wrapping_sub(0x40), [pw, ph]);
        let x0 = 0x7ff8u32.wrapping_sub(((pw as i32) / 2) as u32);
        let y0 = 0x7ff8u32.wrapping_sub(((ph as i32) / 2) as u32);
        let q = at(p, 0x50);
        h.write(ebp.wrapping_sub(0x50), q);
        h.write(q, [0u32, 0]);
        h.write(at(q, 0x10), [x0, y0, z, 0, cw << 4, ch << 4]);
        h.write(
            at(q, 0x30),
            [pw.wrapping_add(x0), ph.wrapping_add(y0), z, 0],
        );
        h.write(at(pk, 4), at(q, 0x40));
        cdecl(h, RC_PACKET_DATA_CLOSE_ADR, &[pk]);
        cdecl(h, RC_PACKET_DATA_DRAW, &[pk])
    }

    // ---- batch 2: the battle position table and the debug camera's state ----

    /// `g_BtlPosTable` (`0x0113_e278`, the position file loaded for the
    /// debug editor): `+4`/`+0x10` offset and scale, `+0x1c` record count,
    /// `+0x24`/`+0x2c` a float table and its count, `+0x30` the records
    /// (0x74 bytes).
    pub const BTL_POS_TABLE: u32 = 0x0113_e278;

    /// `MsBtlPosAccess_007c9ca0(dir, rec, what, i, v)` (name ours): reads
    /// (`dir <= 0`) or writes (`dir > 0`) one vector (3 `f32`) of the
    /// position table: `what` 0/1 the table's scale/offset, 2 the record's
    /// own, 3-8 entry `i` of one of its six lists (`+0x28`, `+0x60`,
    /// `+0x4c`, `+0x2c`, `+0x64`, `+0x50`; counts `+0xc`, `+0x54`, `+0x30`),
    /// 9/10 its `+0x1c`/`+0x40`; 11 one `f32` of the table's `+0x2c` list.
    /// A read with `dir == 0` of a record's vector (2-10) is turned to the
    /// world: `offset + record + scale * v`. An index out of range is 0
    /// and returns -1; no table returns 1; else 0.
    pub fn btl_pos_access(h: &mut impl Host, dir: u32, rec: u32, what: u32, i: u32, v: u32) -> u32 {
        let t = g32(h, BTL_POS_TABLE);
        if t == 0 || h.read::<u32>(at(t, 0x30)) == 0 {
            return 1;
        }
        let recs: u32 = h.read(at(t, 0x30));
        let n: i32 = h.read(at(t, 0x1c));
        let r = rec as i32;
        let r = if r < 0 || r >= n { 0 } else { r as u32 };
        let rp = at(recs, r.wrapping_mul(0x74));
        let mut eax = 0u32;
        let index = |h: &_, count: u32, eax: &mut u32| -> u32 {
            let c: i32 = Mem::read(h, at(rp, count));
            if (i as i32) < 0 || (i as i32) >= c {
                *eax = u32::MAX;
                0
            } else {
                i
            }
        };
        let (p, world) = match what {
            0 => (at(t, 0x10), false),
            1 => (at(t, 4), false),
            2 => (rp, true),
            3..=8 => {
                let (count, list) = match what {
                    3 => (0xc, 0x28),
                    4 => (0x54, 0x60),
                    5 => (0x30, 0x4c),
                    6 => (0xc, 0x2c),
                    7 => (0x54, 0x64),
                    _ => (0x30, 0x50),
                };
                let k = index(h, count, &mut eax);
                let base: u32 = h.read(at(rp, list));
                if base == 0 {
                    return eax;
                }
                (at(base, k.wrapping_mul(12)), true)
            }
            9 => (at(rp, 0x1c), true),
            10 => (at(rp, 0x40), true),
            11 => {
                let c: i32 = h.read(at(t, 0x24));
                let k = if (i as i32) < 0 || (i as i32) >= c {
                    eax = u32::MAX;
                    0
                } else {
                    i
                };
                let list: u32 = h.read(at(t, 0x2c));
                if list != 0 {
                    let cell = at(list, k.wrapping_mul(4));
                    if (dir as i32) > 0 {
                        let x = fcopy(h.read(v));
                        h.write(cell, x);
                    } else {
                        let x = fcopy(h.read(cell));
                        h.write(v, x);
                    }
                }
                return eax;
            }
            _ => return 0,
        };
        if p == 0 {
            return eax;
        }
        if (dir as i32) > 0 {
            for o in [0u32, 4, 8] {
                let x = fcopy(h.read(at(v, o)));
                h.write(at(p, o), x);
            }
            return eax;
        }
        for o in [0u32, 4, 8] {
            let x = fcopy(h.read(at(p, o)));
            h.write(at(v, o), x);
        }
        if world && dir == 0 {
            for o in [0u32, 4, 8] {
                let x = f32_at(h, at(v, o));
                let base = x87::add(f32_at(h, at(t, 4 + o)), f32_at(h, at(rp, o)));
                let y = x87::add(base, x87::mul(f32_at(h, at(t, 0x10 + o)), x));
                h.write(at(v, o), x87::store(y));
            }
        }
        eax
    }

    /// `g_DebugCamera` (`0x0113_70c1`, a byte of screen `[0x0113_9f10]`'s
    /// 0x1c0-byte record) and the battle cameras (0xda0 bytes from
    /// `0x0113_75f0`, 0x130-byte channels; a side is 0x61c bytes).
    pub const DEBUG_CAMERA: u32 = 0x0113_70c1;
    pub const DEBUG_SCREEN: u32 = 0x0113_9f10;

    /// The debug camera, its `MsCameraGetID` and its channel's offset
    /// (`cam * 0xda0 + MsCameraGetNum(cam) * 0x130`), with the channel
    /// number.
    fn debug_channel(h: &mut impl Host) -> (u32, u32, u32, u32) {
        let screen = g32(h, DEBUG_SCREEN);
        let cam = u32::from(h.read::<u8>(at(h.rebase(DEBUG_CAMERA), screen.wrapping_mul(0x1c0))));
        let id = cdecl(h, MS_CAMERA_GET_ID, &[cam]);
        let num = cdecl(h, MS_CAMERA_GET_NUM, &[cam]);
        let ch = cam
            .wrapping_mul(0xda0)
            .wrapping_add(num.wrapping_mul(0x130));
        (cam, id, num, ch)
    }

    pub const MS_CAMERA_SET_RECT: u32 = 0x007b_f940;
    pub const CAM_SET_POLAR: u32 = 0x007b_f700;
    pub const MS_CAMERA_SET_AIM_POLAR: u32 = 0x007b_f330;
    pub const MS_CAMERA_SET_AIM_POLAR2: u32 = 0x007b_f420;
    pub const MS_CAMERA_SET_FOCUS_LIST: u32 = 0x007b_f520;
    pub const MS_CAMERA_SET_TARGET_POLAR: u32 = 0x007b_f600;
    pub const MS_CAMERA_SET_ATEL_YAW: u32 = 0x007b_efc0;
    pub const MS_CAMERA_SET_BTL: u32 = 0x007c_5c60;
    pub const CAMERA_KEEP_BLOCK: u32 = 0x0113_ed7c;

    /// `MsDebugCameraApply_007d2fc0(side)` (name ours): the debug camera's
    /// channel side (`side & 1`) re-requested in the debug mode
    /// (`MsDebugGetCameraMode(side)`): 1 `MsCameraSetRect`, 2
    /// `Cam_SetPolar`, 3 `MsCameraSetAtelFocus`, 4/5/6 the aim requests, 8/9
    /// the keep list (`0x0113_ed7c`), 10 `MsCameraSetAtelYaw`, 11-13, 15, 16
    /// `MsCameraSetBtl` kinds 0-4; the channel side's `+0x30..` values as
    /// `f32` copies. The `side` slot keeps the camera's id. Returns the
    /// callee's `eax`, or the mode less 1.
    pub fn camera_apply(h: &mut impl Host, side: u32, slots: u32) -> u32 {
        let side = side & 1;
        let (_, id, _, ch) = debug_channel(h);
        h.write(slots, id);
        let mode = cdecl(h, MS_DEBUG_GET_CAMERA_MODE, &[side]);
        let c = at(ch, side.wrapping_mul(0x61c));
        let f = |h: &_, sym: u32| -> u32 { fcopy(Mem::read(h, at(Mem::rebase(h, sym), c))) };
        let half = |h: &_, sym: u32, signed: bool| -> u32 {
            let v: u16 = Mem::read(h, at(Mem::rebase(h, sym), c));
            if signed {
                i32::from(v as i16) as u32
            } else {
                u32::from(v)
            }
        };
        let byte =
            |h: &_, sym: u32| -> u32 { u32::from(Mem::read::<u8>(h, at(Mem::rebase(h, sym), c))) };
        let k = mode.wrapping_sub(1);
        match mode {
            1 => {
                let p = at(h.rebase(0x0113_7660), c);
                cdecl(h, MS_CAMERA_SET_RECT, &[id, side, p])
            }
            2 => {
                let a = [
                    id,
                    side,
                    f(h, 0x0113_7670),
                    f(h, 0x0113_7674),
                    f(h, 0x0113_7678),
                ];
                cdecl(h, CAM_SET_POLAR, &a)
            }
            3 => {
                let p = at(h.rebase(0x0113_7660), c);
                let a = [
                    id,
                    side,
                    half(h, 0x0113_763c, false),
                    half(h, 0x0113_763e, false),
                    p,
                ];
                cdecl(h, MS_CAMERA_SET_ATEL_FOCUS, &a)
            }
            4 => {
                let a = [
                    id,
                    side,
                    f(h, 0x0113_7660),
                    f(h, 0x0113_7668),
                    f(h, 0x0113_7674),
                    f(h, 0x0113_7684),
                    f(h, 0x0113_7678),
                    f(h, 0x0113_7688),
                ];
                cdecl(h, MS_CAMERA_SET_AIM_POLAR, &a)
            }
            5 | 6 => {
                let a = [
                    id,
                    side,
                    mode - 5,
                    f(h, 0x0113_7660),
                    f(h, 0x0113_7664),
                    f(h, 0x0113_7668),
                    f(h, 0x0113_7684),
                    f(h, 0x0113_7678),
                    f(h, 0x0113_7688),
                ];
                cdecl(h, MS_CAMERA_SET_AIM_POLAR2, &a)
            }
            8 => {
                let keep = h.rebase(CAMERA_KEEP_BLOCK);
                let a = [id, side, byte(h, 0x0113_7633), keep, f(h, 0x0113_7670)];
                cdecl(h, MS_CAMERA_SET_FOCUS_LIST, &a)
            }
            9 => {
                let keep = h.rebase(CAMERA_KEEP_BLOCK);
                let a = [
                    id,
                    side,
                    byte(h, 0x0113_7633),
                    keep,
                    f(h, 0x0113_7680),
                    f(h, 0x0113_7660),
                    f(h, 0x0113_7668),
                ];
                cdecl(h, MS_CAMERA_SET_TARGET_POLAR, &a)
            }
            10 => {
                let a = [
                    id,
                    side,
                    f(h, 0x0113_7670),
                    f(h, 0x0113_7674),
                    f(h, 0x0113_7678),
                    f(h, 0x0113_767c),
                ];
                cdecl(h, MS_CAMERA_SET_ATEL_YAW, &a)
            }
            11 => {
                let a = [
                    id,
                    side,
                    0,
                    0,
                    half(h, 0x0113_7640, true),
                    half(h, 0x0113_7642, true),
                    f(h, 0x0113_768c),
                    0,
                    0,
                    0,
                ];
                cdecl(h, MS_CAMERA_SET_BTL, &a)
            }
            12 | 13 | 15 | 16 => {
                let kind = match mode {
                    12 => 1,
                    13 => 2,
                    15 => 3,
                    _ => 4,
                };
                let a = [
                    id,
                    side,
                    0,
                    kind,
                    half(h, 0x0113_7640, true),
                    half(h, 0x0113_7642, true),
                    f(h, 0x0113_768c),
                    f(h, 0x0113_7670),
                    f(h, 0x0113_7674),
                    f(h, 0x0113_7690),
                ];
                cdecl(h, MS_CAMERA_SET_BTL, &a)
            }
            _ => k,
        }
    }

    /// `g_BtlDebugCameraKeep` (`0x0113_9f0c`).
    pub const DEBUG_CAMERA_KEEP: u32 = 0x0113_9f0c;

    /// `MsDebugCameraEditLoad_007d3f40(edit)` (name ours): the debug
    /// camera's channel copied into the camera editor's record `edit` (its
    /// `+0x6b8` = 1): the view (`+0x5c0..`) and side-0 values as `f32`
    /// copies, the side's kinds and actors (`u16`), the side-1 mode
    /// (`MsDebugGetCameraMode(1)`) as the editor's kind `+0x34` (8: `+0x60`
    /// the angle in degrees; 9: the keep's), the side-0 mode as `+0x6a0`,
    /// `+0x6cc` = `0x0113_9f0c` (also the `eax` it returns).
    pub fn camera_edit_load(h: &mut impl Host, edit: u32) -> u32 {
        let (cam, _, num, ch) = debug_channel(h);
        let m1 = cdecl(h, MS_DEBUG_GET_CAMERA_MODE, &[1]);
        let f = |h: &_, sym: u32| -> u32 { fcopy(Mem::read(h, at(Mem::rebase(h, sym), ch))) };
        h.write(at(edit, 0x6b8), 1u16);
        for (o, sym) in [
            (0u32, 0x0113_7c7cu32),
            (4, 0x0113_7c80),
            (8, 0x0113_7c84),
            (0xc, 0x0113_7c88),
            (0x38, 0x0113_7c8c),
            (0x3c, 0x0113_7c90),
            (0x40, 0x0113_7c94),
            (0x24, 0x0113_7ca0),
            (0x28, 0x0113_7c94),
            (0x2c, 0x0113_7ca4),
            (0x6d0, 0x0113_7c98),
        ] {
            let v = f(h, sym);
            h.write(at(edit, o), v);
        }
        let b = u32::from(h.read::<u8>(at(h.rebase(0x0113_7c4f), ch)));
        h.write(at(edit, 0x48), b);
        let cn = cam.wrapping_mul(0xda0).wrapping_add(num.wrapping_mul(0x30));
        let v = fcopy(h.read(at(h.rebase(0x0113_8200), cn)));
        h.write(at(edit, 0x698), v);
        let v = fcopy(h.read(at(h.rebase(0x0113_81f8), cn)));
        h.write(at(edit, 0x69c), v);
        for (o, sym) in [(0x6d4u32, 0x0113_7c5cu32), (0x6d6, 0x0113_7c5e)] {
            let v: u16 = h.read(at(h.rebase(sym), ch));
            h.write(at(edit, o), v);
        }
        for (o, sym) in [(0x6d8u32, 0x0113_7ca8u32), (0x6dc, 0x0113_7cac)] {
            let v = f(h, sym);
            h.write(at(edit, o), v);
        }
        let kind = match m1 {
            1 => Some(6u32),
            2 => Some(3),
            4 => Some(0),
            5 => Some(1),
            6 => Some(2),
            8 => Some(4),
            9 => Some(5),
            10 => Some(7),
            11 => Some(8),
            12 => Some(9),
            13 => Some(10),
            15 => Some(11),
            16 => Some(12),
            _ => None,
        };
        if let Some(k) = kind {
            h.write(at(edit, 0x34), k);
        }
        if m1 == 8 {
            let a = x87::div(
                x87::mul(f32_at(h, at(h.rebase(0x0113_7c8c), ch)), f64_at(h, DEG_180)),
                f64_at(h, PI_F64),
            );
            h.write(at(edit, 0x60), x87::store(a));
        } else if m1 == 9 {
            let v = f(h, 0x0113_7c9c);
            h.write(at(edit, 0x60), v);
        }
        let m0 = cdecl(h, MS_DEBUG_GET_CAMERA_MODE, &[0]);
        for (o, sym) in [
            (0x6a4u32, 0x0113_7660u32),
            (0x6a8, 0x0113_7664),
            (0x6ac, 0x0113_7668),
            (0x6b0, 0x0113_766c),
            (0x20, 0x0113_7664),
        ] {
            let v = f(h, sym);
            h.write(at(edit, o), v);
        }
        for (o, sym) in [
            (0x6b4u32, 0x0113_763cu32),
            (0x6b6, 0x0113_763e),
            (0x6e0, 0x0113_7640),
            (0x6e2, 0x0113_7642),
        ] {
            let v: u16 = h.read(at(h.rebase(sym), ch));
            h.write(at(edit, o), v);
        }
        for (o, sym) in [
            (0x6e4u32, 0x0113_768cu32),
            (0x6f0, 0x0113_7690),
            (0x6e8, 0x0113_7670),
            (0x6ec, 0x0113_7674),
        ] {
            let v = f(h, sym);
            h.write(at(edit, o), v);
        }
        let k0 = match m0 {
            1 => Some(1u32),
            3 => Some(0),
            11 => Some(2),
            12 => Some(3),
            13 => Some(4),
            _ => None,
        };
        if let Some(k) = k0 {
            h.write(at(edit, 0x6a0), k);
        }
        let keep = g32(h, DEBUG_CAMERA_KEEP);
        h.write(at(edit, 0x6cc), keep);
        keep
    }

    pub const MS_DEBUG_ACTOR_COUNT: u32 = 0x007d_9ab0;
    pub const S_TAGT1: u32 = 0x00b5_6778;
    pub const S_TAGT2: u32 = 0x00b5_6780;
    pub const S_HEI: u32 = 0x00b5_6788;
    pub const S_SPACE: u32 = 0x00b2_0b30;
    pub const S_THETA_LC: u32 = 0x00b5_6790;
    pub const S_PHI_LC: u32 = 0x00b5_6798;
    pub const S_LEN_PCT: u32 = 0x00b5_679c;
    pub const S_LEN_LC: u32 = 0x00b5_67a4;
    pub const S_FMT_S: u32 = 0x00b0_e160;
    pub const S_FMT_ACTOR: u32 = 0x00b5_5240;
    /// The names of the reference targets: below 0 from `0x00c4_4824`
    /// downwards, from 0 on (actors without a battle id) at `0x00c4_4950`.
    pub const REF_NAMES_NEG: u32 = 0x00c4_4828;
    pub const REF_NAMES: u32 = 0x00c4_4950;

    /// A target index (`i16` at `field`) kept in `-0x12 ..= count - 1`:
    /// below -0x12 it becomes `count - 1`, at or past `count` -0x12.
    fn ref_target_clamp(h: &mut impl Host, field: u32, count: u32) {
        let v: i16 = h.read(field);
        if v < -0x12 {
            h.write(field, count.wrapping_sub(1) as u16);
        }
        let v: i16 = h.read(field);
        if i32::from(v) >= count as i32 {
            h.write(field, 0xffeeu16);
        }
    }

    /// A target's text: a negative index's name, `"%2d:%2d"` of it and the
    /// count when the window counts battle actors, else its actor's name.
    fn print_target(h: &mut impl Host, buf: u32, y: u32, v: i16, actors: bool, count: u32) {
        let v = i32::from(v);
        if v < 0 {
            let at = h
                .rebase(REF_NAMES_NEG)
                .wrapping_sub((v as u32).wrapping_mul(4).wrapping_add(4));
            let name: u32 = h.read(at);
            let fmt = h.rebase(S_FMT_S);
            print_at(h, buf, 0x5c, y, &[fmt, name]);
        } else if actors {
            let fmt = h.rebase(S_FMT_ACTOR);
            print_at(h, buf, 0x5c, y, &[fmt, v as u32, count]);
        } else {
            let name: u32 = h.read(at(h.rebase(REF_NAMES), (v as u32).wrapping_mul(4)));
            let fmt = h.rebase(S_FMT_S);
            print_at(h, buf, 0x5c, y, &[fmt, name]);
        }
    }

    /// `MsDebugBtlPrint(kind, _, obj, id, y)`: the reference window's
    /// values; `ebp` is the original's frame (the line buffer at `ebp -
    /// 0x104`, the actor count and its kind flag at `- 0x108`, `- 0x10c`).
    /// The two targets (`i16 +0x10`, `+0x12`) are first kept in range of
    /// `MsDebugActorCount_007d9ab0(id, &flag)`; then TAGT1, TAGT2, the
    /// height (`HEI %`: `+0x5c * 100`; none for kinds 0xf, 0x10), and
    /// unless kind 0xb theta, phi (degrees) and the length (`+0x60 * 100`;
    /// `len %` for kinds 0xc, 0xf), else three dash lines. Returns the
    /// last row's `y`.
    pub fn btl_print(h: &mut impl Host, ebp: u32, kind: u32, obj: u32, id: u32, y: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x104);
        let flag_at = ebp.wrapping_sub(0x10c);
        let count = cdecl(h, MS_DEBUG_ACTOR_COUNT, &[id, flag_at]);
        h.write(ebp.wrapping_sub(0x108), count);
        ref_target_clamp(h, at(obj, 0x10), count);
        ref_target_clamp(h, at(obj, 0x12), count);
        let mut y = y;
        for (label, o) in [(S_TAGT1, 0x10u32), (S_TAGT2, 0x12)] {
            y = y.wrapping_add(10);
            print_label(h, buf, 0, y, label);
            let v: i16 = h.read(at(obj, o));
            let flag: u32 = h.read(flag_at);
            print_target(h, buf, y, v, flag != 0, count);
        }
        y = y.wrapping_add(10);
        let no_height = kind == 0xf || kind == 0x10;
        print_label(h, buf, 0, y, if no_height { S_SPACE } else { S_HEI });
        if no_height {
            print_label(h, buf, 0x5c, y, DASHES);
        } else {
            let v = x87::mul(f64_at(h, HUNDRED), f32_at(h, at(obj, 0x5c)));
            print_value(h, buf, y, v);
        }
        y = y.wrapping_add(10);
        if kind == 0xb {
            print_label(h, buf, 0x5c, y, DASHES);
            for _ in 0..2 {
                y = y.wrapping_add(10);
                print_label(h, buf, 0x5c, y, DASHES);
            }
            return y;
        }
        for (label, o) in [(S_THETA_LC, 0x40u32), (S_PHI_LC, 0x44)] {
            print_label(h, buf, 0, y, label);
            let v = value(h, obj, Deg(o));
            print_value(h, buf, y, v);
            y = y.wrapping_add(10);
        }
        let label = if kind == 0xc || kind == 0xf {
            S_LEN_PCT
        } else {
            S_LEN_LC
        };
        print_label(h, buf, 0, y, label);
        let v = x87::mul(f32_at(h, at(obj, 0x60)), f64_at(h, HUNDRED));
        print_value(h, buf, y, v);
        y
    }

    pub const SET_GLOBAL_0113F864: u32 = 0x007d_9af0;

    /// `MsDebugCameraEditStore_007d38c0(edit)` (name ours): the camera
    /// editor's record written back to the debug camera's channel (the
    /// inverse of [`camera_edit_load`]; `+0x6bc..` keeps `+0..+0xc` while
    /// `+0x6b8` is 0), the side-1 request by the editor's kind `+0x34`
    /// (0-12: aim polar, aim polar 2 long/short, polar, the keep list (the
    /// `edit` slot holding the angle in radians), the target polar, rect,
    /// yaw, `MsCameraSetBtl` kinds 0-4), `+0x6a8` = `+0x20`, the side-0
    /// values and request by `+0x6a0` (0-4: focus, rect, `MsCameraSetBtl`
    /// kinds 0-2), the reference actor (`SetGlobal0113f864_007d9af0` of
    /// `i16 +0x6ba`) and `0x0113_9f0c` = `+0x6cc` (also the `eax` it
    /// returns).
    pub fn camera_edit_store(h: &mut impl Host, edit: u32, slots: u32) -> u32 {
        let (cam, id, num, ch) = debug_channel(h);
        let e = |h: &_, o: u32| -> u32 { fcopy(Mem::read(h, at(edit, o))) };
        let put = |h: &mut _, sym: u32, v: u32| {
            let a = at(Mem::rebase(h, sym), ch);
            Mem::write(h, a, v);
        };
        if h.read::<u16>(at(edit, 0x6b8)) == 0 {
            for o in [0u32, 4, 8, 0xc] {
                let v = e(h, o);
                h.write(at(edit, 0x6bc + o), v);
            }
        }
        for (sym, o) in [
            (0x0113_7c7cu32, 0u32),
            (0x0113_7c80, 4),
            (0x0113_7c84, 8),
            (0x0113_7c88, 0xc),
            (0x0113_7c8c, 0x38),
            (0x0113_7c90, 0x3c),
            (0x0113_7c94, 0x40),
            (0x0113_7ca0, 0x24),
            (0x0113_7c94, 0x28),
            (0x0113_7ca4, 0x2c),
            (0x0113_7c98, 0x6d0),
        ] {
            let v = e(h, o);
            put(h, sym, v);
        }
        let b: u8 = h.read(at(edit, 0x48));
        h.write(at(h.rebase(0x0113_7c4f), ch), b);
        let cn = cam.wrapping_mul(0xda0).wrapping_add(num.wrapping_mul(0x30));
        let v = e(h, 0x698);
        h.write(at(h.rebase(0x0113_8200), cn), v);
        let v = e(h, 0x69c);
        h.write(at(h.rebase(0x0113_81f8), cn), v);
        for (sym, o) in [(0x0113_7c5cu32, 0x6d4u32), (0x0113_7c5e, 0x6d6)] {
            let v: u16 = h.read(at(edit, o));
            h.write(at(h.rebase(sym), ch), v);
        }
        for (sym, o) in [(0x0113_7ca8u32, 0x6d8u32), (0x0113_7cac, 0x6dc)] {
            let v = e(h, o);
            put(h, sym, v);
        }
        let ch_f = |h: &_, sym: u32| -> u32 { fcopy(Mem::read(h, at(Mem::rebase(h, sym), ch))) };
        let ch_s = |h: &_, sym: u32| -> u32 {
            i32::from(Mem::read::<i16>(h, at(Mem::rebase(h, sym), ch))) as u32
        };
        let angle = |h: &mut _| -> u32 {
            let a = x87::store(x87::div(
                x87::mul(f32_at(h, at(edit, 0x60)), f64_at(h, PI_F64)),
                f64_at(h, DEG_180),
            ));
            Mem::write(h, slots, a);
            a
        };
        match h.read::<u32>(at(edit, 0x34)) {
            0 => {
                let a = [
                    id,
                    1,
                    e(h, 0),
                    e(h, 8),
                    e(h, 0x3c),
                    e(h, 0x24),
                    e(h, 0x28),
                    e(h, 0x2c),
                ];
                cdecl(h, MS_CAMERA_SET_AIM_POLAR, &a);
            }
            k @ (1 | 2) => {
                let a = [
                    id,
                    1,
                    k - 1,
                    e(h, 0),
                    e(h, 4),
                    e(h, 8),
                    e(h, 0x24),
                    e(h, 0x28),
                    e(h, 0x2c),
                ];
                cdecl(h, MS_CAMERA_SET_AIM_POLAR2, &a);
            }
            3 => {
                let a = [id, 1, e(h, 0x38), e(h, 0x3c), e(h, 0x40)];
                cdecl(h, CAM_SET_POLAR, &a);
            }
            4 => {
                let a = angle(h);
                let b: u32 = h.read(at(edit, 0x48));
                cdecl(h, MS_CAMERA_SET_FOCUS_LIST, &[id, 1, b, at(edit, 0x64), a]);
            }
            5 => {
                let (x, z) = (e(h, 8), e(h, 0));
                let a = angle(h);
                let b: u32 = h.read(at(edit, 0x48));
                cdecl(
                    h,
                    MS_CAMERA_SET_TARGET_POLAR,
                    &[id, 1, b, at(edit, 0x64), a, z, x],
                );
            }
            6 => {
                cdecl(h, MS_CAMERA_SET_RECT, &[id, 1, edit]);
            }
            7 => {
                let a = [id, 1, e(h, 0x38), e(h, 0x3c), e(h, 0x40), e(h, 0x6d0)];
                cdecl(h, MS_CAMERA_SET_ATEL_YAW, &a);
            }
            8 => {
                let a = [
                    id,
                    1,
                    0,
                    0,
                    ch_s(h, 0x0113_7c5c),
                    ch_s(h, 0x0113_7c5e),
                    ch_f(h, 0x0113_7ca8),
                    0,
                    0,
                    0,
                ];
                cdecl(h, MS_CAMERA_SET_BTL, &a);
            }
            k @ 9..=12 => {
                let a = [
                    id,
                    1,
                    0,
                    k - 8,
                    ch_s(h, 0x0113_7c5c),
                    ch_s(h, 0x0113_7c5e),
                    ch_f(h, 0x0113_7ca8),
                    e(h, 0x38),
                    e(h, 0x3c),
                    e(h, 0x6dc),
                ];
                cdecl(h, MS_CAMERA_SET_BTL, &a);
            }
            _ => {}
        }
        let v = e(h, 0x20);
        h.write(at(edit, 0x6a8), v);
        for (sym, o) in [
            (0x0113_7660u32, 0x6a4u32),
            (0x0113_7664, 0x6a8),
            (0x0113_7668, 0x6ac),
        ] {
            let v = e(h, o);
            put(h, sym, v);
        }
        for (sym, o) in [
            (0x0113_763cu32, 0x6b4u32),
            (0x0113_763e, 0x6b6),
            (0x0113_7640, 0x6e0),
            (0x0113_7642, 0x6e2),
        ] {
            let v: u16 = h.read(at(edit, o));
            h.write(at(h.rebase(sym), ch), v);
        }
        for (sym, o) in [
            (0x0113_768cu32, 0x6e4u32),
            (0x0113_7690, 0x6f0),
            (0x0113_7670, 0x6e8),
            (0x0113_7674, 0x6ec),
        ] {
            let v = e(h, o);
            put(h, sym, v);
        }
        let es = |h: &_, o: u32| -> u32 { i32::from(Mem::read::<i16>(h, at(edit, o))) as u32 };
        match h.read::<u32>(at(edit, 0x6a0)) {
            0 => {
                let a = [id, 0, es(h, 0x6b4), es(h, 0x6b6), at(edit, 0x6a4)];
                cdecl(h, MS_CAMERA_SET_ATEL_FOCUS, &a);
            }
            1 => {
                cdecl(h, MS_CAMERA_SET_RECT, &[id, 0, at(edit, 0x6a4)]);
            }
            2 => {
                let a = [
                    id,
                    0,
                    0,
                    0,
                    ch_s(h, 0x0113_7640),
                    ch_s(h, 0x0113_7642),
                    ch_f(h, 0x0113_768c),
                    0,
                    0,
                    0,
                ];
                cdecl(h, MS_CAMERA_SET_BTL, &a);
            }
            k @ (3 | 4) => {
                let a = [
                    id,
                    0,
                    0,
                    k - 2,
                    ch_s(h, 0x0113_7640),
                    ch_s(h, 0x0113_7642),
                    ch_f(h, 0x0113_768c),
                    e(h, 0x6e8),
                    e(h, 0x6ec),
                    e(h, 0x6f0),
                ];
                cdecl(h, MS_CAMERA_SET_BTL, &a);
            }
            _ => {}
        }
        let r = es(h, 0x6ba);
        cdecl(h, SET_GLOBAL_0113F864, &[r]);
        let keep: u32 = h.read(at(edit, 0x6cc));
        set32(h, DEBUG_CAMERA_KEEP, keep);
        keep
    }

    // ---- batch 3: the debug camera's script dump ----

    /// A CRT import (cdecl) through its slot, for the long `sprintf`s.
    fn crt_wide(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call_wide(f, args)
    }

    /// An `f64` as two stack words.
    fn dw(v: f64) -> [u32; 2] {
        let b = v.to_bits();
        [b as u32, (b >> 32) as u32]
    }

    /// `sprintf(buf, fmt, words..)` with the format at `fmt` (`ffx.sym`).
    fn sprintf(h: &mut impl Host, buf: u32, fmt: u32, words: &[u32]) {
        let mut a = vec![buf, h.rebase(fmt)];
        a.extend_from_slice(words);
        crt_wide(h, SPRINTF, &a);
    }

    /// `sprintf` of a format and doubles.
    fn sprintf_f(h: &mut impl Host, buf: u32, fmt: u32, pre: &[u32], vals: &[f64]) {
        let mut w = pre.to_vec();
        for &v in vals {
            w.extend(dw(v));
        }
        // call width: 18 (the eight doubles of `.float ..`, plus the
        // buffer and the format)
        sprintf(h, buf, fmt, &w);
    }

    pub const S_NL: u32 = 0x00b7_cae0;
    pub const S_LINE_NL2: u32 = 0x00b5_5760;
    pub const S_DATA: u32 = 0x00b5_63b0;
    pub const S_FLOATS: u32 = 0x00b5_63d8;
    pub const S_FLOAT_REF: u32 = 0x00b5_6430;
    pub const S_CLOSE: u32 = 0x00b5_6458;
    pub const S_REF_FILTER2: u32 = 0x00b5_645c;
    pub const S_REF_FILTER_Y2: u32 = 0x00b5_647c;
    pub const S_SET_POS: u32 = 0x00b5_6498;
    pub const S_SET_POLAR: u32 = 0x00b5_64b0;
    pub const S_SET_ACT: u32 = 0x00b5_64cc;
    pub const S_SET_HYPOT: u32 = 0x00b5_64e4;
    pub const S_SET_HYPOT2: u32 = 0x00b5_650c;
    pub const S_SET_HYPOT3: u32 = 0x00b5_6538;
    pub const S_SET_POLAR_OFFSET: u32 = 0x00b5_6564;
    pub const S_SET_DATA_POINT: u32 = 0x00b5_6584;
    pub const S_SET_DATA_POINT2: u32 = 0x00b5_65a8;
    pub const S_SET_DATA_POINT_HYPOT: u32 = 0x00b5_65cc;
    pub const S_SET_DATA_POINT_HYPOT2: u32 = 0x00b5_6600;
    pub const S_SET_ACT2: u32 = 0x00b5_6634;
    pub const S_FMT_D: u32 = 0x00b0_f604;
    pub const S_SET_BTL: u32 = 0x00b5_6654;
    pub const S_SET_BTL_POLAR: u32 = 0x00b5_666c;
    pub const S_SET_BTL_POLAR2: u32 = 0x00b5_6698;
    pub const S_SET_CHR_POLAR: u32 = 0x00b5_66c4;
    pub const S_SET_CHR_POLAR2: u32 = 0x00b5_66f0;
    pub const S_CAMERA_SCRIPT: u32 = 0x00b5_671c;
    /// The camera actors' names (indices below 0, downwards from
    /// `0x00c4_486c`).
    pub const CAMERA_ACTOR_NAMES: u32 = 0x00c4_4870;
    pub const G_CAMERAS: u32 = 0x0113_7540;
    pub const TK_REMOVE: u32 = 0x0088_c740;

    fn dbg(h: &mut impl Host, fmt: u32, words: &[u32]) -> u32 {
        let mut a = vec![h.rebase(fmt)];
        a.extend_from_slice(words);
        // call width: 14 (`chr %2d : ..`: the format, an index and six
        // doubles)
        h.call_sym_wide(DBG_PRINTF, &a)
    }

    /// `(a * 180) / pi` and `(180 * a) / pi` as the dump computes them.
    fn deg(h: &impl Host, a: f64) -> f64 {
        x87::div(x87::mul(a, f64_at(h, DEG_180)), f64_at(h, PI_F64))
    }

    /// `MsDebugOutCamera()`: the debug camera's two sides written out as
    /// camera script (`dbgPrintf`): for each side its keep list as data
    /// (modes 8, 9), the reference filter's lines on side 0 (scale
    /// `0x0113_ed48` below 1), and the side's request as a `Set..` line
    /// into the frame's `ebp - 0x304 + 0x100 * side` (the actors' names
    /// into `ebp - 0x404`/`- 0x504`); then the whole script
    /// (`ref%s\ncam%s` with the roll and screen depth), `TkRemove`d and
    /// printed. `ebp` is the original's frame (the line buffer `ebp -
    /// 0x104`), `trig` the CRT's `_CIcos`/`_CIsin`. Returns the last
    /// `dbgPrintf`'s `eax`.
    #[allow(clippy::similar_names)]
    pub fn out_camera(h: &mut impl Host, trig: &mut dyn FnMut(Trig, f64) -> u32, ebp: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x104);
        let (n1, n2) = (ebp.wrapping_sub(0x404), ebp.wrapping_sub(0x504));
        dbg(h, S_NL, &[]);
        let screen = g32(h, DEBUG_SCREEN);
        let cam = u32::from(h.read::<u8>(at(h.rebase(DEBUG_CAMERA), screen.wrapping_mul(0x1c0))));
        let base = at(h.rebase(G_CAMERAS), cam.wrapping_mul(0xda0));
        let num = cdecl(h, MS_CAMERA_GET_NUM, &[cam]);
        for side in 0..2u32 {
            let c = at(
                base,
                side.wrapping_mul(0x61c)
                    .wrapping_add(num.wrapping_mul(0x130)),
            );
            let f = |h: &_, o: u32| f32_at(h, at(c, o));
            let mode = cdecl(h, MS_DEBUG_GET_CAMERA_MODE, &[side]);
            if mode == 8 || mode == 9 {
                let list: u32 = h.read(at(c, 0xf8));
                let flags = u32::from(h.read::<u8>(at(c, 0xf3)));
                let count: u32 = h.read(list);
                sprintf(h, buf, S_DATA, &[count, flags, 0, 0]);
                dbg(h, FMT_LINE, &[buf]);
                let mut k = 0i32;
                while k < h.read::<i32>(list) {
                    let e = at(h.rebase(KEEPS), (k as u32).wrapping_mul(0x30));
                    let v: Vec<f64> = (0..7).map(|i| f32_at(h, at(e, 4 * i))).collect();
                    sprintf_f(
                        h,
                        buf,
                        S_FLOATS,
                        &[],
                        &[v[0], v[1], v[2], v[3], v[4], v[5], v[6], 0.0],
                    );
                    dbg(h, FMT_LINE, &[buf]);
                    let r = f32_at(h, at(e, 0x20));
                    sprintf_f(h, buf, S_FLOAT_REF, &[], &[r, 0.0, 0.0, 0.0]);
                    let last = k == h.read::<i32>(list).wrapping_sub(1);
                    dbg(h, if last { FMT_LINE } else { S_LINE_NL2 }, &[buf]);
                    k += 1;
                }
                sprintf(h, buf, S_CLOSE, &[]);
                dbg(h, FMT_LINE, &[buf]);
            }
            let scale = f32_at(h, h.rebase(0x0113_ed48));
            if side == 0 && scale < 1.0 {
                let g = |h: &_, sym: u32| f32_at(h, Mem::rebase(h, sym));
                let (x0, z0, x1, z1) = (
                    g(h, 0x0113_ed28),
                    g(h, 0x0113_ed30),
                    g(h, 0x0113_f3d4),
                    g(h, 0x0113_f3dc),
                );
                sprintf_f(h, buf, S_REF_FILTER2, &[], &[x0, z0, x1, z1, scale]);
                dbg(h, FMT_LINE, &[buf]);
                let s = g(h, 0x0113_ed48);
                let (x0, z0, x1, z1) = (
                    g(h, 0x0113_ed28),
                    g(h, 0x0113_ed30),
                    g(h, 0x0113_f3d4),
                    g(h, 0x0113_f3dc),
                );
                sprintf_f(h, buf, S_REF_FILTER2, &[], &[x1, z1, x0, z0, s]);
                dbg(h, FMT_LINE, &[buf]);
                let angle = f64_at(h, ARROW_ANGLE);
                let cs = trig(Trig::Cos, angle);
                let dx = x87::sub(g(h, 0x0113_f3d4), g(h, 0x0113_ed28));
                let sn = trig(Trig::Sin, angle);
                let dz = x87::sub(g(h, 0x0113_f3dc), g(h, 0x0113_ed30));
                let x0 = g(h, 0x0113_ed28);
                let z0 = g(h, 0x0113_ed30);
                let (cs, sn) = (x87::load(cs), x87::load(sn));
                let x = x87::store(x87::sub(x87::add(x87::mul(cs, dx), x0), x87::mul(sn, dz)));
                let z = x87::store(x87::add(x87::mul(cs, dz), x87::add(x87::mul(sn, dx), z0)));
                let (x, z) = (x87::load(x), x87::load(z));
                let s = g(h, 0x0113_ed48);
                sprintf_f(h, buf, S_REF_FILTER2, &[], &[x0, z0, x, z, s]);
                dbg(h, FMT_LINE, &[buf]);
                let s = g(h, 0x0113_ed48);
                let (x0, z0) = (g(h, 0x0113_ed28), g(h, 0x0113_ed30));
                sprintf_f(h, buf, S_REF_FILTER2, &[], &[x, z, x0, z0, s]);
                dbg(h, FMT_LINE, &[buf]);
                let s = g(h, 0x0113_ed48);
                let r = g(h, 0x0113_ed2c);
                sprintf_f(h, buf, S_REF_FILTER_Y2, &[], &[r, r, s]);
                dbg(h, FMT_LINE, &[buf]);
            }
            let out = ebp.wrapping_sub(0x304).wrapping_add(side << 8);
            let p180 = f64_at(h, DEG_180);
            let pi = f64_at(h, PI_F64);
            // (180 * f) / pi, the dumps' first polar angle
            let deg_first = |h: &_, o: u32| x87::div(x87::mul(p180, f32_at(h, at(c, o))), pi);
            let hundred = f64_at(h, HUNDRED);
            match mode {
                1 => sprintf_f(
                    h,
                    out,
                    S_SET_POS,
                    &[],
                    &[f(h, 0x120), f(h, 0x124), f(h, 0x128)],
                ),
                2 => {
                    let v = [deg_first(h, 0x130), deg(h, f(h, 0x134)), f(h, 0x138)];
                    sprintf_f(h, out, S_SET_POLAR, &[], &v);
                }
                3 => {
                    let a = i32::from(h.read::<i16>(at(c, 0xfc))) as u32;
                    let b = i32::from(h.read::<i16>(at(c, 0xfe))) as u32;
                    sprintf_f(h, out, S_SET_ACT, &[a, b], &[f(h, 0x124)]);
                }
                4 => {
                    let v = [
                        f(h, 0x120),
                        f(h, 0x128),
                        deg(h, f(h, 0x134)),
                        f(h, 0x144),
                        f(h, 0x138),
                        f(h, 0x148),
                    ];
                    sprintf_f(h, out, S_SET_HYPOT, &[], &v);
                }
                5 | 6 => {
                    let v = [
                        f(h, 0x120),
                        f(h, 0x124),
                        f(h, 0x128),
                        f(h, 0x144),
                        f(h, 0x138),
                        f(h, 0x148),
                    ];
                    let fmt = if mode == 5 {
                        S_SET_HYPOT2
                    } else {
                        S_SET_HYPOT3
                    };
                    sprintf_f(h, out, fmt, &[], &v);
                }
                7 => {
                    let v = [
                        deg_first(h, 0x130),
                        deg(h, f(h, 0x134)),
                        deg(h, f(h, 0x138)),
                    ];
                    sprintf_f(h, out, S_SET_POLAR_OFFSET, &[], &v);
                }
                8 => {
                    let v = deg(h, f(h, 0x130));
                    let one = h.read::<u8>(at(c, 0xf3)) & 1 != 0;
                    let fmt = if one {
                        S_SET_DATA_POINT
                    } else {
                        S_SET_DATA_POINT2
                    };
                    sprintf_f(h, out, fmt, &[], &[v]);
                }
                9 => {
                    let v = [deg(h, f(h, 0x140)), f(h, 0x120), f(h, 0x128)];
                    let one = h.read::<u8>(at(c, 0xf3)) & 1 != 0;
                    let fmt = if one {
                        S_SET_DATA_POINT_HYPOT
                    } else {
                        S_SET_DATA_POINT_HYPOT2
                    };
                    sprintf_f(h, out, fmt, &[], &v);
                }
                10 => {
                    let v = [
                        deg_first(h, 0x130),
                        deg(h, f(h, 0x134)),
                        f(h, 0x138),
                        f(h, 0x13c),
                    ];
                    sprintf_f(h, out, S_SET_ACT2, &[], &v);
                }
                11..=13 | 15 | 16 => {
                    for (o, name) in [(0x100u32, n1), (0x102, n2)] {
                        let v = i32::from(h.read::<i16>(at(c, o)));
                        if v < 0 {
                            let at_ = h
                                .rebase(CAMERA_ACTOR_NAMES)
                                .wrapping_sub((v as u32).wrapping_mul(4).wrapping_add(4));
                            let s: u32 = h.read(at_);
                            sprintf(h, name, S_FMT_S, &[s]);
                        } else {
                            sprintf(h, name, S_FMT_D, &[v as u32]);
                        }
                    }
                    let polar = |h: &_| [deg_first(h, 0x130), deg(h, f32_at(h, at(c, 0x134)))];
                    match mode {
                        11 => {
                            let v = x87::mul(f(h, 0x14c), hundred);
                            sprintf_f(h, out, S_SET_BTL, &[n1, n2], &[v]);
                        }
                        12 | 13 => {
                            let [a2, a3] = polar(h);
                            let v = [
                                x87::mul(hundred, f(h, 0x14c)),
                                a2,
                                a3,
                                x87::mul(f(h, 0x150), hundred),
                            ];
                            let fmt = if mode == 12 {
                                S_SET_BTL_POLAR
                            } else {
                                S_SET_BTL_POLAR2
                            };
                            sprintf_f(h, out, fmt, &[n1, n2], &v);
                        }
                        _ => {
                            let [a2, a3] = polar(h);
                            let v = [0.0, a2, a3, x87::mul(f(h, 0x150), hundred)];
                            let fmt = if mode == 15 {
                                S_SET_CHR_POLAR
                            } else {
                                S_SET_CHR_POLAR2
                            };
                            sprintf_f(h, out, fmt, &[n1, n2], &v);
                        }
                    }
                }
                _ => {}
            }
        }
        let n = num.wrapping_mul(0x30);
        let scr = f32_at(h, at(base, n.wrapping_add(0xcc0)));
        let roll = deg(h, f32_at(h, at(base, n.wrapping_add(0xcb8))));
        let (r, cm) = (ebp.wrapping_sub(0x304), ebp.wrapping_sub(0x204));
        sprintf_f(h, buf, S_CAMERA_SCRIPT, &[r, cm], &[roll, scr]);
        cdecl(h, TK_REMOVE, &[buf]);
        dbg(h, S_FMT_S, &[buf])
    }

    // ---- batch 4: the battle position windows ----

    pub const MS_BTL_GET_POS: u32 = 0x007a_bfe0;
    pub const MS_DEBUG_POS_WINDOWS: u32 = 0x007d_7a40;
    /// The 0x60 position windows (`0x0113_f528`..`0x0113_f6a8`).
    pub const POS_WINDOWS: u32 = 0x0113_f528;
    pub const G_BTL_POS_FILE: u32 = 0x0112_a9b0;
    /// The position screen's kind (`0x00c4_49cc`), its picked position per
    /// kind (`0x0113_f6dc..`), the blink flag `0x0113_f708`, the screen's
    /// off flag `0x0113_f704`.
    pub const POS_KIND: u32 = 0x00c4_49cc;

    /// The position windows of one kind from slot `slot` on: one window
    /// (`MsDebugWinAlloc_007c8bb0`) per position `MsBtlGetPos(0, 0, group,
    /// kind, i, win)` finds, coloured `rgb` with mark `mark` (`+0x22`),
    /// halved when it is the picked one (kind `0x00c4_49cc`, index `pick`,
    /// blink `0x0113_f708`). Returns the next slot.
    fn pos_windows(
        h: &mut impl Host,
        group: u32,
        kind: u32,
        slot: u32,
        rgb: [u32; 3],
        mark: u8,
        pick: u32,
    ) -> u32 {
        let mut slot = slot;
        if (slot as i32) >= 0x60 {
            return slot;
        }
        let mut i = 0u32;
        let mut p = at(h.rebase(POS_WINDOWS), slot.wrapping_mul(4));
        let end = h.rebase(0x0113_f6a8);
        loop {
            let win: u32 = h.read(p);
            if cdecl(h, MS_BTL_GET_POS, &[0, 0, group, kind, i, win]) != 0 {
                return slot;
            }
            cdecl(h, MS_DEBUG_WIN_ALLOC, &[p]);
            let w: u32 = h.read(p);
            if w != 0 {
                let picked = g32(h, POS_KIND) == kind;
                for (k, c) in rgb.into_iter().enumerate() {
                    let w: u32 = h.read(p);
                    h.write(at(w, 0x28 + 4 * k as u32), c);
                }
                let w: u32 = h.read(p);
                h.write(at(w, 0x22), mark);
                if picked && i == g32(h, pick) && g32(h, 0x0113_f708) != 0 {
                    for o in [0x28u32, 0x2c, 0x30] {
                        let w: u32 = h.read(p);
                        let v: i32 = h.read(at(w, o));
                        h.write(at(w, o), v / 2);
                    }
                }
            }
            p = at(p, 4);
            i += 1;
            slot += 1;
            // a signed compare of the addresses, as the original's
            if (p as i32) >= (end as i32) {
                return slot;
            }
        }
    }

    pub const MS_DEBUG_WIN_ALLOC: u32 = 0x007c_8bb0;

    /// `MsDebugPosWindowsAll_007d74e0(group)` (name ours): unless the
    /// screen is off (`0x0113_f704`), the battle position screen's windows:
    /// the group's marks (`MsDebugPosWindows_007d7a40` kind 2), then the
    /// positions of kinds 3 and 6 and, on a map without a position file
    /// flag (`*g_BtlPosFile` 0), kinds 9 and 10's marks and positions of
    /// kinds 4, 7, 5, 8, each in its colour; the slots left over are
    /// freed (`+0x20`, `+0x21` cleared, the slot 0). Callers drop `eax`.
    pub fn pos_windows_all(h: &mut impl Host, group: u32) {
        if g32(h, 0x0113_f704) != 0 {
            return;
        }
        let mut slot = cdecl(
            h,
            MS_DEBUG_POS_WINDOWS,
            &[1, group, 2, 0, 0xff, 0xff, 0xff, 5],
        );
        slot = pos_windows(h, group, 3, slot, [0x37, 0x78, 0xff], 5, 0x0113_f6dc);
        slot = pos_windows(h, group, 6, slot, [0x23, 0x46, 0x69], 3, 0x0113_f6e8);
        let file = g32(h, G_BTL_POS_FILE);
        if h.read::<u8>(file) == 0 {
            let s = cdecl(
                h,
                MS_DEBUG_POS_WINDOWS,
                &[1, group, 9, slot, 4, 0x14, 0xff, 5],
            );
            slot = cdecl(
                h,
                MS_DEBUG_POS_WINDOWS,
                &[1, group, 10, s, 0xff, 0x14, 4, 5],
            );
            slot = pos_windows(h, group, 4, slot, [0xeb, 0xeb, 0x81], 5, 0x0113_f6e0);
            slot = pos_windows(h, group, 7, slot, [0x69, 0x69, 0x4f], 3, 0x0113_f6ec);
            slot = pos_windows(h, group, 5, slot, [0xeb, 0x78, 0x37], 5, 0x0113_f6e4);
            slot = pos_windows(h, group, 8, slot, [0x69, 0x46, 0x23], 3, 0x0113_f6f0);
        }
        if (slot as i32) < 0x60 {
            let end = h.rebase(0x0113_f6a8);
            let mut p = at(h.rebase(POS_WINDOWS), slot.wrapping_mul(4));
            loop {
                let w: u32 = h.read(p);
                if w != 0 {
                    h.write(at(w, 0x20), 0u8);
                    h.write(at(w, 0x21), 0u8);
                    h.write(p, 0u32);
                }
                p = at(p, 4);
                if (p as i32) >= (end as i32) {
                    break;
                }
            }
        }
    }

    pub const MS_VECTOR_LEN: u32 = 0x007c_4280;
    pub const MS_DEBUG_WIDTH_AT: u32 = 0x007d_93f0;
    pub const MS_GET_CHR: u32 = 0x0079_4020;
    pub const MS_GET_CHR_POS: u32 = 0x0079_56e0;
    pub const MS_GET_CHR_BTL_POS: u32 = 0x0079_48e0;
    /// `f64` 1 metre in the debug print's units (`.rdata`).
    pub const METRE: u32 = 0x00b9_22b0;
    pub const S_DASH10: u32 = 0x00b5_6868;
    pub const S_DASH2: u32 = 0x00b5_68a8;
    pub const S_PLY_PLY_W: u32 = 0x00b5_6874;
    pub const S_PLY_PLY: u32 = 0x00b5_6894;
    pub const S_PLY_MON_W: u32 = 0x00b5_68ac;
    pub const S_PLY_MON: u32 = 0x00b5_68cc;
    pub const S_MON_MON_W: u32 = 0x00b5_68e0;
    pub const S_MON_MON: u32 = 0x00b5_6900;
    pub const S_PLY_POS: u32 = 0x00b5_6914;
    pub const S_MON_POS: u32 = 0x00b5_6928;
    pub const S_CHR_POS: u32 = 0x00b5_693c;

    /// `v2 -= v1` (the frame's `ebp - 0x14`, `- 0x24`; `f32` each) and
    /// its length (`MsVectorLen`, as `f32` bits).
    fn pos_sub_len(h: &mut impl Host, ebp: u32) -> u32 {
        let (v1, v2) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
        for o in [0u32, 4, 8, 0xc] {
            let d = x87::store(x87::sub(f32_at(h, at(v2, o)), f32_at(h, at(v1, o))));
            h.write(at(v2, o), d);
        }
        h.call_sym_st0(MS_VECTOR_LEN, Conv::Cdecl, &[v2]).to_bits()
    }

    /// The distance from `a` to `b` (positions of `kind_a`/`kind_b` in
    /// the frame vectors `v1`, `v2`): `v2 -= v1` (as `f32`s), its length
    /// (`MsVectorLen`), and with `widths` less both actors' widths
    /// (`MsDebugWidthAt_007d93f0`, their indices into `n1`, `n2`), printed
    /// in metres with `fmt_w` (and the indices) or `fmt`.
    #[allow(clippy::too_many_arguments)]
    fn pos_distance(
        h: &mut impl Host,
        ebp: u32,
        group: u32,
        widths: bool,
        (kind_a, a): (u32, u32),
        (kind_b, b): (u32, u32),
        fmt_w: u32,
        fmt: u32,
    ) {
        let (n1, n2) = (ebp.wrapping_sub(0x4c), ebp.wrapping_sub(0x48));
        let len = pos_sub_len(h, ebp);
        let metre = f64_at(h, METRE);
        if widths {
            let t = x87::load(len);
            let r = h.call_sym_st0_f64(MS_DEBUG_WIDTH_AT, Conv::Cdecl, &[group, kind_a, a, n1]);
            let t = x87::sub(t, r);
            let r = h.call_sym_st0_f64(MS_DEBUG_WIDTH_AT, Conv::Cdecl, &[group, kind_b, b, n2]);
            let d = x87::store(x87::sub(t, r));
            let v = dw(x87::div(x87::load(d), metre));
            let (i1, i2): (u32, u32) = (h.read(n1), h.read(n2));
            dbg(h, fmt_w, &[a, b, v[0], v[1], i1, i2]);
        } else {
            let v = dw(x87::div(x87::load(len), metre));
            dbg(h, fmt, &[a, b, v[0], v[1]]);
        }
    }

    /// Whether monster position `b` of `group` has a live actor: the
    /// first of the 0x1f (`MsGetChr`) at place (group, 5, b) (`i8 +0x4fc`
    /// .. `+0x4fe`), its `+0x10` set.
    fn pos_monster_live(h: &mut impl Host, group: u32, b: u32) -> bool {
        for i in 0..0x1fu32 {
            let c = cdecl(h, MS_GET_CHR, &[i]);
            let g = i32::from(h.read::<i8>(at(c, 0x4fc))) as u32;
            if g == group && h.read::<u8>(at(c, 0x4fd)) == 5 {
                let k = i32::from(h.read::<i8>(at(c, 0x4fe))) as u32;
                if k == b {
                    return h.read::<i8>(at(c, 0x10)) != 0;
                }
            }
        }
        false
    }

    /// `MsDebugPosDistances_007d7b20(group, widths)` (name ours): the
    /// battle positions' report (`dbgPrintf`): every party-party distance
    /// (kind 3), party-monster and monster-monster distance (kind 5, live
    /// monsters only), with `widths` less the actors' widths; then the
    /// positions of kinds 3 and 5 and the 0x1c actors' positions
    /// (`MsGetChrPos`, `MsGetChrBtlPos`). `ebp` is the original's frame
    /// (the vectors `MsBtlGetPos` fills at `ebp - 0x24`, `- 0x14`, the
    /// actor's at `- 0x34`, the indices at `- 0x4c`, `- 0x48`). Returns the
    /// last `dbgPrintf`'s `eax`.
    pub fn pos_distances(h: &mut impl Host, ebp: u32, group: u32, widths: u32) -> u32 {
        let (v1, v2, w) = (
            ebp.wrapping_sub(0x24),
            ebp.wrapping_sub(0x14),
            ebp.wrapping_sub(0x34),
        );
        let widths = widths != 0;
        let pos = |h: &mut _, kind: u32, i: u32, v: u32| {
            cdecl(h, MS_BTL_GET_POS, &[0, 0, group, kind, i, v]) == 0
        };
        dbg(h, S_DASH10, &[]);
        let mut a = 0;
        while pos(h, 3, a, v1) {
            let mut b = 0;
            while pos(h, 3, b, v2) {
                if a == b {
                    // the length is still taken
                    pos_sub_len(h, ebp);
                } else {
                    pos_distance(
                        h,
                        ebp,
                        group,
                        widths,
                        (3, a),
                        (3, b),
                        S_PLY_PLY_W,
                        S_PLY_PLY,
                    );
                }
                b += 1;
            }
            a += 1;
        }
        dbg(h, S_DASH2, &[]);
        let mut a = 0;
        while pos(h, 3, a, v1) {
            let mut b = 0;
            while pos(h, 5, b, v2) {
                if pos_monster_live(h, group, b) {
                    pos_distance(
                        h,
                        ebp,
                        group,
                        widths,
                        (3, a),
                        (5, b),
                        S_PLY_MON_W,
                        S_PLY_MON,
                    );
                }
                b += 1;
            }
            a += 1;
        }
        dbg(h, S_DASH2, &[]);
        let mut a = 0;
        while pos(h, 5, a, v1) {
            let mut b = 0;
            while pos(h, 5, b, v2) {
                if pos_monster_live(h, group, b) {
                    if a == b {
                        pos_sub_len(h, ebp);
                    } else {
                        pos_distance(
                            h,
                            ebp,
                            group,
                            widths,
                            (5, a),
                            (5, b),
                            S_MON_MON_W,
                            S_MON_MON,
                        );
                    }
                }
                b += 1;
            }
            a += 1;
        }
        dbg(h, S_DASH2, &[]);
        for (kind, fmt) in [(3u32, S_PLY_POS), (5, S_MON_POS)] {
            let mut a = 0;
            while pos(h, kind, a, v1) {
                let v: Vec<u32> = [0u32, 4, 8]
                    .iter()
                    .flat_map(|&o| dw(f32_at(h, at(v1, o))))
                    .collect();
                let mut words = vec![a];
                words.extend(v);
                // call width: 8 (the format, an index, three doubles)
                dbg(h, fmt, &words);
                a += 1;
            }
        }
        let mut eax = dbg(h, S_DASH2, &[]);
        for i in 0..0x1cu32 {
            cdecl(h, MS_GET_CHR, &[i]);
            cdecl(h, MS_GET_CHR_POS, &[i, v1]);
            cdecl(h, MS_GET_CHR_BTL_POS, &[i, w]);
            let mut words = vec![i];
            for p in [v1, w] {
                for o in [0u32, 4, 8] {
                    words.extend(dw(f32_at(h, at(p, o))));
                }
            }
            // call width: 14 (the format, an index, six doubles)
            eax = dbg(h, S_CHR_POS, &words);
        }
        eax
    }

    // ---- batch 5: the battle actor dumps ----

    pub const MS_ENV_WRITE_TEXT_7DE700: u32 = 0x007d_e700;
    pub const S_MOTION_TYPE: u32 = 0x00b5_7424;
    pub const S_MOTION_TYPE_END: u32 = 0x00b5_7668;

    /// The motion type flags (`+0x4ac`) `MsDebugChrMotionSave_007dcf70`
    /// names, in its order: (bit, `"%s | motion_type_.."`).
    pub const MOTION_TYPE_FLAGS: [(u32, u32); 17] = [
        (1, 0x00b5_7444),
        (2, 0x00b5_7468),
        (4, 0x00b5_7488),
        (8, 0x00b5_74a8),
        (0x10, 0x00b5_74c8),
        (0x1000, 0x00b5_74ec),
        (0x20, 0x00b5_750c),
        (0x40, 0x00b5_7528),
        (0x80, 0x00b5_754c),
        (0x100, 0x00b5_756c),
        (0x200, 0x00b5_758c),
        (0x400, 0x00b5_75a8),
        (0x800, 0x00b5_75c4),
        (0x2000, 0x00b5_75e0),
        (0x4000, 0x00b5_7600),
        (0x8000, 0x00b5_7620),
        (0x1_0000, 0x00b5_7644),
    ];

    /// A field a script line shows.
    #[derive(Debug, Clone, Copy)]
    pub enum Field {
        U8(u32),
        I8(u32),
        U16(u32),
        F32(u32),
    }

    /// `MsDebugChrMotionSave_007dcf70`'s lines after the motion type: (the
    /// `setstat2`/`setmotiondata2` format, the actor's field).
    pub const MOTION_LINES: [(u32, Field); 20] = [
        (0x00b5_7670, Field::U8(0x41d)),
        (0x00b5_76a0, Field::U8(0x41b)),
        (0x00b5_76cc, Field::F32(0x4bc)),
        (0x00b5_7700, Field::F32(0x4c0)),
        (0x00b5_7730, Field::F32(0x4c4)),
        (0x00b5_7760, Field::F32(0x4c8)),
        (0x00b5_7794, Field::F32(0x4d4)),
        (0x00b5_77c0, Field::F32(0x4d8)),
        (0x00b5_77f0, Field::F32(0x4cc)),
        (0x00b5_781c, Field::F32(0x4d0)),
        (0x00b5_784c, Field::F32(0x4dc)),
        (0x00b5_7874, Field::U16(0x4b0)),
        (0x00b5_7894, Field::U16(0x4b2)),
        (0x00b5_78b8, Field::U8(0x4b9)),
        (0x00b5_78e0, Field::U8(0x4ba)),
        (0x00b5_7908, Field::I8(0x4b8)),
        (0x00b5_792c, Field::U8(0x4bb)),
        (0x00b5_7950, Field::U8(0x40c)),
        (0x00b5_797c, Field::U8(0x40d)),
        (0x00b5_79a8, Field::U8(0x40e)),
    ];

    /// A field as `sprintf` takes it (an `f32` as a double).
    fn field_words(h: &impl Host, p: u32, f: Field) -> Vec<u32> {
        match f {
            Field::U8(o) => vec![u32::from(h.read::<u8>(at(p, o)))],
            Field::I8(o) => vec![i32::from(h.read::<i8>(at(p, o))) as u32],
            Field::U16(o) => vec![u32::from(h.read::<u16>(at(p, o)))],
            Field::F32(o) => dw(f32_at(h, at(p, o))).to_vec(),
        }
    }

    /// `MsDebugChrMotionSave_007dcf70(i, writer)` (name ours): actor `i`'s
    /// motion settings written as battle script (`MsEnvWriteText_007de700`
    /// of each line; the line buffer `buf`, the original's `ebp - 0x404`):
    /// the motion type flags `+0x4ac` as a `|` list, then
    /// [`MOTION_LINES`]. Returns the last write's `eax`.
    pub fn chr_motion_save(h: &mut impl Host, buf: u32, i: u32, writer: u32) -> u32 {
        let c = cdecl(h, MS_GET_CHR, &[i]);
        let flags: u32 = h.read(at(c, 0x4ac));
        sprintf(h, buf, S_MOTION_TYPE, &[]);
        for (bit, fmt) in MOTION_TYPE_FLAGS {
            if flags & bit != 0 {
                sprintf(h, buf, fmt, &[buf]);
            }
        }
        sprintf(h, buf, S_MOTION_TYPE_END, &[buf]);
        let mut eax = cdecl(h, MS_ENV_WRITE_TEXT_7DE700, &[writer, buf]);
        for (fmt, f) in MOTION_LINES {
            let w = field_words(h, c, f);
            // call width: 4 (the buffer, the format, a double)
            sprintf(h, buf, fmt, &w);
            eax = cdecl(h, MS_ENV_WRITE_TEXT_7DE700, &[writer, buf]);
        }
        eax
    }

    /// A value a battle info line shows: an actor field, a bit of a `u16`
    /// field, or a global `i8`.
    #[derive(Debug, Clone, Copy)]
    pub enum Info {
        U8(u32),
        I8(u32),
        U16(u32),
        U32(u32),
        Bit(u32, u32),
    }

    use Info::{Bit, I8 as S8, U8 as B8, U16 as W16, U32 as D32};

    /// The battle info window's plain lines: (id, format, values).
    pub const INFO_LINES: &[(u32, u32, &[Info])] = &[
        (
            0x4f,
            0x00b5_6c78,
            &[
                B8(0x65c),
                B8(0x65d),
                B8(0x5ac),
                S8(0x4fc),
                S8(0x4fd),
                S8(0x4fe),
            ],
        ),
        (
            0x50,
            0x00b5_6c9c,
            &[D32(0x5d0), D32(0x594), D32(0x5d4), D32(0x598)],
        ),
        (
            0x54,
            0x00b5_6cd0,
            &[B8(0x5a8), B8(0x5a9), B8(0x5aa), B8(0x5ab)],
        ),
        (
            0x55,
            0x00b5_6cec,
            &[B8(0x5ac), B8(0x5ad), B8(0x5ae), B8(0x5af)],
        ),
        (
            0x56,
            0x00b5_6d08,
            &[B8(0x5ba), B8(0x5bb), B8(0x5bc), B8(0x5bd)],
        ),
        (
            0x57,
            0x00b5_6d24,
            &[W16(0x5b8), B8(0x592), B8(0x593), B8(0x40c)],
        ),
        (
            0x59,
            0x00b5_6d68,
            &[B8(0x5da), B8(0x5db), B8(0x5dc), B8(0x5dd)],
        ),
        (
            0x5a,
            0x00b5_6d8c,
            &[
                B8(0x65e),
                B8(0x65f),
                B8(0x660),
                B8(0x661),
                B8(0x662),
                B8(0x663),
            ],
        ),
        (
            0x5c,
            0x00b5_6ddc,
            &[
                S8(0xdf1),
                B8(0xde6),
                B8(0xde7),
                S8(0xdfb),
                S8(0xdf2),
                S8(0x432),
                S8(0x433),
                S8(0x508),
                S8(0x50e),
            ],
        ),
        (
            0x8f,
            0x00b5_6db0,
            &[
                S8(0xddd),
                S8(0x4ff),
                S8(0x504),
                Bit(0x606, 0),
                S8(0xdd9),
                S8(0xdc8),
                S8(0x10),
            ],
        ),
        (0x99, 0x00b5_6e70, &[B8(0xf2d), B8(0xf2c)]),
        (0xcd, 0x00b5_6e80, &[B8(0x5c2), B8(0x5c3)]),
        (
            0xce,
            0x00b5_6e90,
            &[
                S8(0xdcd),
                S8(0xdfa),
                W16(0x400),
                B8(0x44f),
                B8(0x63d),
                B8(0xf5e),
            ],
        ),
        (
            0xdc,
            0x00b5_6f4c,
            &[
                S8(0x11),
                S8(0x12),
                B8(0x14),
                B8(0x43f),
                B8(0x441),
                B8(0x440),
            ],
        ),
        (
            0x9e,
            0x00b5_702c,
            &[B8(0x641), B8(0x642), B8(0x643), B8(0x644), B8(0x645)],
        ),
        (
            0x9f,
            0x00b5_7054,
            &[B8(0x646), B8(0x647), B8(0x648), B8(0x649), B8(0x64a)],
        ),
        (
            0xa0,
            0x00b5_7170,
            &[B8(0x64b), B8(0x64c), B8(0x64d), B8(0x64e), B8(0x64f)],
        ),
        (
            0xa1,
            0x00b5_70a8,
            &[B8(0x650), B8(0x651), B8(0x652), B8(0x653), B8(0x654)],
        ),
        (
            0xa2,
            0x00b5_70d0,
            &[B8(0x655), B8(0x656), B8(0x657), B8(0x658), B8(0x659)],
        ),
        (
            0xa9,
            0x00b5_70a8,
            &[B8(0x60b), B8(0x60c), B8(0x60d), B8(0x60e), B8(0x60f)],
        ),
        (
            0xaa,
            0x00b5_70d0,
            &[B8(0x610), B8(0x611), B8(0x612), B8(0x613), B8(0x614)],
        ),
    ];

    /// Lines of five bits of a `u16` status field: (id, format, field,
    /// first bit).
    pub const INFO_BITS: [(u32, u32, u32, u32); 7] = [
        (0xa6, 0x00b5_702c, 0x606, 0),
        (0xa7, 0x00b5_7054, 0x606, 5),
        (0xab, 0x00b5_70f8, 0x616, 0),
        (0xac, 0x00b5_7120, 0x616, 5),
        (0xa3, 0x00b5_70f8, 0x65a, 0),
        (0xa4, 0x00b5_7120, 0x65a, 5),
        (0xa5, 0x00b5_7198, 0x65a, 10),
    ];

    fn info(h: &impl Host, a: u32, v: Info) -> u32 {
        match v {
            Info::U8(o) => u32::from(h.read::<u8>(at(a, o))),
            Info::I8(o) => i32::from(h.read::<i8>(at(a, o))) as u32,
            Info::U16(o) => u32::from(h.read::<u16>(at(a, o))),
            Info::U32(o) => h.read(at(a, o)),
            Info::Bit(o, b) => (u32::from(h.read::<u16>(at(a, o))) >> b) & 1,
        }
    }

    pub const MS_GET_NEXT_AP: u32 = 0x0078_4e90;
    pub const MS_GET_SAVE_NEXT_AP: u32 = 0x0078_52b0;
    pub const MS_GET_CHR_VIGOR: u32 = 0x0078_cce0;
    pub const CH_EFFECT_GET_EFFECT_LEVEL: u32 = 0x0082_a9a0;
    pub const CH_GET_BODY_HIT: u32 = 0x0082_ab10;
    pub const MS_CHECK_BTL_POS: u32 = 0x007a_9230;
    pub const MS_GET_CHR_MOVE_FRAME_REST: u32 = 0x007a_9320;
    pub const CH_GET_WAIT_MOTION: u32 = 0x0083_92c0;
    pub const CH_GET_ORG_MATRIX: u32 = 0x0082_ac60;
    pub const MS_MENU_WEAK_ELEMENT: u32 = 0x007a_9a20;
    pub const MS_EFFECT_CHECK_MOTION: u32 = 0x0078_7840;
    pub const MS_EFFECT_CHECK_MOTION2: u32 = 0x0078_78a0;
    pub const CH_GET_CAM_HEIGHT: u32 = 0x0082_ab30;
    pub const MS_GET_CHR_CAM_WIDTH: u32 = 0x0079_4a70;
    pub const MS_GET_CHR_HEIGHT: u32 = 0x0079_50f0;
    pub const MS_GET_CHR_WIDTH: u32 = 0x0079_5a20;
    pub const MS_GET_CHR_GROUND: u32 = 0x0079_4fb0;
    pub const MS_GET_SUMMONER_NUM: u32 = 0x0078_8e10;
    pub const MS_GET_STATUS_LOOK: u32 = 0x0079_e7c0;
    /// The battle's `i8` at `0x0112_c9d3` (the info window's `T%1d`).
    pub const BTL_TURN_FLAG: u32 = 0x0112_c9d3;
    pub const G_PLY_SAVE_LEVELS: u32 = 0x0113_2097;

    /// `MsDebugChrInfoLine_007dbee0(model, dst, src, id, actor)` (name
    /// ours): one line of the battle info window: `sprintf(dst, fmt, src,
    /// values..)` for line `id` ([`INFO_LINES`], [`INFO_BITS`] and the
    /// lines with callees: AP, attack, effect level, body hit, motion
    /// frames, wait motion, sizes, position, summoner, damage, action,
    /// status), else `"%s"` of `src`. `model` picks the AP line's source;
    /// `ebp` is the original's frame (the position at `ebp - 0x14`).
    /// Returns `sprintf`'s `eax`.
    #[allow(clippy::too_many_arguments)]
    pub fn chr_info_line(
        h: &mut impl Host,
        ebp: u32,
        model: u32,
        dst: u32,
        src: u32,
        id: u32,
        a: u32,
    ) -> u32 {
        let n = u32::from(h.read::<u8>(at(a, 0xc)));
        let m: u32 = h.read(a);
        let put = |h: &mut _, fmt: u32, words: &[u32]| -> u32 {
            let mut w = vec![dst, Mem::rebase(h, fmt), src];
            w.extend_from_slice(words);
            // call width: 15 (the body hit line's twelve values)
            crt_wide(h, SPRINTF, &w)
        };
        if let Some(&(_, fmt, vals)) = INFO_LINES.iter().find(|r| r.0 == id) {
            let w: Vec<u32> = vals.iter().map(|&v| info(h, a, v)).collect();
            return put(h, fmt, &w);
        }
        if let Some(&(_, fmt, o, b)) = INFO_BITS.iter().find(|r| r.0 == id) {
            let w: Vec<u32> = (b..b + 5).map(|k| info(h, a, Bit(o, k))).collect();
            return put(h, fmt, &w);
        }
        let metre = f64_at(h, METRE);
        match id {
            0x51 => {
                let w = if model == 0 {
                    let lv = at(h.rebase(G_PLY_SAVE_LEVELS), n.wrapping_mul(0x94));
                    let (b97, b98) = (
                        u32::from(h.read::<u8>(lv)),
                        u32::from(h.read::<u8>(at(lv, 1))),
                    );
                    let x = cdecl(h, MS_GET_NEXT_AP, &[n, b97, b98]);
                    let y = cdecl(h, MS_GET_SAVE_NEXT_AP, &[n, x]);
                    [y, x, b97, b98]
                } else {
                    let p: u32 = h.read(at(a, 0xf88));
                    if p == 0 {
                        [0, 0, 0, 0]
                    } else {
                        let v = |o| u32::from(Mem::read::<u16>(h, at(p, o)));
                        [v(2), v(4), 0, 0]
                    }
                };
                put(h, 0x00b5_6cb4, &w)
            }
            0x58 => {
                let vg = cdecl(h, MS_GET_CHR_VIGOR, &[n]);
                let w = [
                    info(h, a, B8(0x5d9)),
                    info(h, a, W16(0x604)),
                    info(h, a, W16(0x65a)),
                    vg,
                ];
                put(h, 0x00b5_6d44, &w)
            }
            0x96 => {
                let c = if m == 0 {
                    0
                } else {
                    cdecl(h, CH_EFFECT_GET_EFFECT_LEVEL, &[m])
                };
                let mut w = vec![info(h, a, S8(0xe0d)), c];
                for v in [B8(0xe12), B8(0xe13), S8(0x448), B8(0x406), B8(0x40a)] {
                    w.push(info(h, a, v));
                }
                put(h, 0x00b5_6e0c, &w)
            }
            0xc6 => {
                let c = if m == 0 {
                    0
                } else {
                    cdecl(h, CH_GET_BODY_HIT, &[m])
                };
                let mut w = vec![c];
                for v in [
                    S8(0x2e),
                    S8(0x2f),
                    S8(0x30),
                    S8(0x31),
                    B8(0x41c),
                    S8(0x32),
                    B8(0x43e),
                    B8(0x426),
                    S8(0x50e),
                    S8(0x505),
                    S8(0x34),
                ] {
                    w.push(info(h, a, v));
                }
                put(h, 0x00b5_6e38, &w)
            }
            0xcf => {
                let r1 = {
                    let p = info(h, a, S8(0x34));
                    cdecl(h, MS_CHECK_BTL_POS, &[n, a, p])
                };
                let b = info(h, a, B8(0x42e));
                let r2 = cdecl(h, MS_GET_CHR_MOVE_FRAME_REST, &[n]);
                let w = [
                    info(h, a, B8(0x415)),
                    info(h, a, W16(0x452)),
                    info(h, a, W16(0x45a)),
                    r2,
                    b,
                    r1,
                ];
                put(h, 0x00b5_6eb8, &w)
            }
            0xd0 => {
                let (wait, org) = if m == 0 {
                    (0, false)
                } else {
                    let w = cdecl(h, CH_GET_WAIT_MOTION, &[m]);
                    (w, cdecl(h, CH_GET_ORG_MATRIX, &[m]) != 0)
                };
                let e = info(h, a, S8(0x434));
                let weak = cdecl(h, MS_MENU_WEAK_ELEMENT, &[n, e]);
                let (b425, b429, s3f3) = (
                    info(h, a, B8(0x425)),
                    info(h, a, B8(0x429)),
                    info(h, a, S8(0x3f3)),
                );
                let m2 = !cdecl(h, MS_EFFECT_CHECK_MOTION2, &[n]) & 1;
                let m1 = !cdecl(h, MS_EFFECT_CHECK_MOTION, &[n]) & 1;
                let w = [wait, m1, m2, s3f3, b429, b425, u32::from(org), weak, e];
                put(h, 0x00b5_6edc, &w)
            }
            0x90 => {
                let v = if m == 0 {
                    [0.0; 4]
                } else {
                    let ch = h.call_sym_st0_f64(CH_GET_CAM_HEIGHT, Conv::Cdecl, &[m]);
                    let ch = x87::store(x87::div(x87::mul(ch, f32_at(h, at(a, 0x3a4))), metre));
                    let cw = h.call_sym_st0_f64(MS_GET_CHR_CAM_WIDTH, Conv::Cdecl, &[n]);
                    let cw = x87::store(x87::div(cw, metre));
                    let ht = h.call_sym_st0_f64(MS_GET_CHR_HEIGHT, Conv::Cdecl, &[n]);
                    let ht = x87::store(x87::div(ht, metre));
                    let wd = h.call_sym_st0_f64(MS_GET_CHR_WIDTH, Conv::Cdecl, &[n]);
                    let wd = x87::store(x87::div(wd, metre));
                    [cw, ch, wd, ht].map(x87::load)
                };
                let w: Vec<u32> = v.iter().flat_map(|&x| dw(x)).collect();
                put(h, 0x00b5_6f0c, &w)
            }
            0xd3 => {
                let p = ebp.wrapping_sub(0x14);
                cdecl(h, MS_GET_CHR_POS, &[n, p]);
                let mut flag = 0;
                if m != 0 {
                    let org = cdecl(h, CH_GET_ORG_MATRIX, &[m]);
                    if org != 0 {
                        flag = 1;
                        for o in [0u32, 4, 8] {
                            let v = fcopy(h.read(at(org, 0x30 + o)));
                            h.write(at(p, o), v);
                        }
                    }
                }
                let xyz: Vec<u32> = [0u32, 4, 8]
                    .iter()
                    .map(|&o| x87::ftol(h, f32_at(h, at(p, o))))
                    .collect();
                let g = h.call_sym_st0_f64(MS_GET_CHR_GROUND, Conv::Cdecl, &[n]);
                let g = x87::ftol(h, g);
                let w = [flag, info(h, a, S8(0x510)), xyz[0], xyz[1], xyz[2], g];
                put(h, 0x00b5_6f2c, &w)
            }
            0xde => {
                let (d, b718, b719, w8) = (
                    f32_at(h, at(a, 0x4ec)),
                    info(h, a, B8(0x718)),
                    info(h, a, B8(0x719)),
                    info(h, a, D32(8)),
                );
                let s = cdecl(h, MS_GET_SUMMONER_NUM, &[n]);
                let d = dw(d);
                let w = [
                    info(h, a, B8(0x6d8)),
                    info(h, a, B8(0x6d9)),
                    s,
                    b718,
                    b719,
                    d[0],
                    d[1],
                    w8,
                ];
                put(h, 0x00b5_6f74, &w)
            }
            0xdf => {
                let mut w: Vec<u32> = (0x15..=0x19u32).map(|o| info(h, a, S8(o))).collect();
                w.push(m);
                put(h, 0x00b5_6f9c, &w)
            }
            0xe0 => {
                let c = u32::from(h.read::<u8>(at(a, 0x776)) == 0xff)
                    + u32::from(h.read::<u8>(at(a, 0xa4e)) == 0xff);
                let w = [
                    info(h, a, D32(0x6f4)),
                    c,
                    info(h, a, B8(0x700)),
                    info(h, a, B8(0x41e)),
                    info(h, a, B8(0x3f9)),
                ];
                put(h, 0x00b5_6fc0, &w)
            }
            0xe2 => {
                let g = i32::from(h.read::<i8>(h.rebase(BTL_TURN_FLAG))) as u32;
                let w = [
                    info(h, a, S8(0xdf0)),
                    g,
                    info(h, a, B8(0xe13)),
                    info(h, a, S8(0x432)),
                    info(h, a, S8(0x436)),
                    info(h, a, B8(0x3f8)),
                ];
                put(h, 0x00b5_6fe4, &w)
            }
            0xe4 => {
                let s38 = info(h, a, S8(0x38));
                let look = cdecl(h, MS_GET_STATUS_LOOK, &[n, s38]);
                let w = [
                    info(h, a, B8(0x446)),
                    info(h, a, D32(0x39c)),
                    info(h, a, B8(0x413)),
                    info(h, a, B8(0x414)),
                    look,
                    s38,
                ];
                put(h, 0x00b5_7008, &w)
            }
            0xa8 => {
                let w = [
                    info(h, a, Bit(0x606, 10)),
                    info(h, a, Bit(0x606, 11)),
                    info(h, a, B8(0x5c6)),
                    info(h, a, B8(0x608)),
                    info(h, a, B8(0x609)),
                    info(h, a, B8(0x60a)),
                ];
                put(h, 0x00b5_707c, &w)
            }
            0xad => {
                let st = u32::from(h.read::<u16>(at(a, 0x616)));
                let ds = if st & 0x4000 != 0 {
                    info(h, a, B8(0x5c8))
                } else {
                    0
                };
                let b6d4 = h.read::<u8>(at(a, 0x6d4));
                let level = if b6d4 == 0xff { 0x63 } else { u32::from(b6d4) };
                let pr = if st & 0x1000 != 0 { level } else { u32::MAX };
                let ir = if st & 0x2000 != 0 { level } else { u32::MAX };
                let w = [(st >> 10) & 1, (st >> 11) & 1, pr, ir, ds];
                put(h, 0x00b5_7148, &w)
            }
            _ => put(h, S_FMT_S, &[]),
        }
    }

    // ---- batch 6: the position editor's table from a position file ----

    pub const MS_STRUCT_CLEAR: u32 = 0x0079_f230;
    pub const MS_MALLOC_7CF970: u32 = 0x007c_f970;

    /// A cleared block of `n` bytes from `ms_malloc_007cf970`.
    fn alloc_clear(h: &mut impl Host, n: u32) -> u32 {
        let p = cdecl(h, MS_MALLOC_7CF970, &[n]);
        cdecl(h, MS_STRUCT_CLEAR, &[p, n]);
        p
    }

    fn i8_at(h: &impl Host, p: u32) -> i32 {
        i32::from(h.read::<i8>(p))
    }

    /// `n` points (3 `f32`, 0x10 apart in the file) from `src`, less
    /// `origin` (3 `f32`) when given, into a cleared block of `n * 12`
    /// bytes at the table's `+slot`. The count is re-read from `count`
    /// after each point.
    fn pos_points(
        h: &mut impl Host,
        rec: u32,
        slot: u32,
        count: u32,
        src: u32,
        origin: Option<u32>,
    ) {
        let n = i8_at(h, count);
        let p = alloc_clear(h, (n.wrapping_mul(12)) as u32);
        h.write(at(rec, slot), p);
        let mut k = 0i32;
        while k < i8_at(h, count) {
            let s = at(src, (k as u32).wrapping_mul(0x10));
            for o in [0u32, 4, 8] {
                let v = match origin {
                    Some(org) => x87::store(x87::sub(f32_at(h, at(s, o)), f32_at(h, at(org, o)))),
                    None => fcopy(h.read(at(s, o))),
                };
                let dst: u32 = h.read(at(rec, slot));
                h.write(at(dst, (k as u32).wrapping_mul(12).wrapping_add(o)), v);
            }
            k += 1;
        }
    }

    /// `MsBtlPosFromFile_007c9450(file)` (name ours): the position
    /// editor's table (`g_BtlPosTable`, cleared, scale 1) built from the
    /// map's position file: the flag (`+0`), the group count (`+1`),
    /// records of 0x74 bytes. Without the flag (`+0` 0) the file's float
    /// list (`+9` floats at `+[0x2c]`) and its 0x60-byte groups from `+4`:
    /// origin `+0x2c`, the two ends `+0x3c`/`+0x4c` and the point lists
    /// (`+0`, `+2`, `+1` counts, pairs of lists at `+[0xc]`/`+[0x10]`,
    /// `+[0x1c]`/`+[0x20]`, `+[0x14]`/`+[0x18]`) relative to it, the
    /// sub-groups (`+4` of 0x14 bytes from `+[0x24]`); with the flag the
    /// 0x20-byte groups' first list as it is and a second one sized and
    /// filled from the first group (as the original). Callers drop `eax`.
    #[allow(clippy::too_many_lines)]
    pub fn btl_pos_from_file(h: &mut impl Host, file: u32) {
        if file == 0 {
            return;
        }
        let tab = g32(h, BTL_POS_TABLE);
        cdecl(h, MS_STRUCT_CLEAR, &[tab, 0x34]);
        let t = g32(h, BTL_POS_TABLE);
        let one = 0x3f80_0000u32;
        h.write(at(t, 4), 0u32);
        h.write(t, i8_at(h, file) as u32);
        h.write(at(t, 8), [0u32, 0, one, one, one]);
        let groups = i8_at(h, at(file, 1));
        h.write(at(t, 0x1c), groups as u32);
        h.write(at(t, 0x20), 0u32);
        let recs = cdecl(h, MS_MALLOC_7CF970, &[(groups.wrapping_mul(0x74)) as u32]);
        let t = g32(h, BTL_POS_TABLE);
        h.write(at(t, 0x30), recs);
        let n: i32 = h.read(at(t, 0x1c));
        cdecl(h, MS_STRUCT_CLEAR, &[recs, (n.wrapping_mul(0x74)) as u32]);
        if h.read::<u8>(file) != 0 {
            let t = g32(h, BTL_POS_TABLE);
            if h.read::<i32>(at(t, 0x1c)) <= 0 {
                return;
            }
            let first = at(file, 4);
            let mut g = 0i32;
            loop {
                let t = g32(h, BTL_POS_TABLE);
                let src = at(first, (g as u32).wrapping_mul(0x20));
                let rec = at(h.read::<u32>(at(t, 0x30)), (g as u32).wrapping_mul(0x74));
                h.write(at(rec, 0xc), i8_at(h, src) as u32);
                if i8_at(h, src) > 0 {
                    let off: u32 = h.read(at(src, 0xc));
                    pos_points(h, rec, 0x28, src, at(file, off), None);
                    // sized by the first group, filled from its +0x14 list
                    // less the file's +0x30 (as the original)
                    let off: u32 = h.read(at(file, 0x14));
                    pos_points(h, rec, 0x2c, first, at(file, off), Some(at(file, 0x30)));
                }
                g += 1;
                let t = g32(h, BTL_POS_TABLE);
                if g >= h.read::<i32>(at(t, 0x1c)) {
                    return;
                }
            }
        }
        let t = g32(h, BTL_POS_TABLE);
        let nf = i8_at(h, at(file, 9));
        h.write(at(t, 0x24), nf as u32);
        if nf > 0 {
            let p = cdecl(h, MS_MALLOC_7CF970, &[(nf << 2) as u32]);
            let t = g32(h, BTL_POS_TABLE);
            h.write(at(t, 0x2c), p);
            cdecl(
                h,
                MS_STRUCT_CLEAR,
                &[p, (i8_at(h, at(file, 9)) << 2) as u32],
            );
            let src = at(file, h.read::<u32>(at(file, 0x2c)));
            let mut k = 0i32;
            while k < i8_at(h, at(file, 9)) {
                let v = fcopy(h.read(at(src, (k as u32) * 4)));
                let t = g32(h, BTL_POS_TABLE);
                let dst: u32 = h.read(at(t, 0x2c));
                h.write(at(dst, (k as u32) * 4), v);
                k += 1;
            }
        }
        let t = g32(h, BTL_POS_TABLE);
        if h.read::<i32>(at(t, 0x1c)) <= 0 {
            return;
        }
        let mut g = 0i32;
        loop {
            let s = at(file, 4 + (g as u32).wrapping_mul(0x60));
            let t = g32(h, BTL_POS_TABLE);
            let rec = at(h.read::<u32>(at(t, 0x30)), (g as u32).wrapping_mul(0x74));
            let org = at(s, 0x2c);
            for o in [0u32, 4, 8] {
                let v = fcopy(h.read(at(org, o)));
                h.write(at(rec, o), v);
            }
            for (from, to) in [(0u32, 0xcu32), (1, 0x54), (2, 0x30), (4, 0x68)] {
                h.write(at(rec, to), i8_at(h, at(s, from)) as u32);
            }
            for o in [0u32, 4, 8] {
                let v = x87::store(x87::sub(f32_at(h, at(s, 0x3c + o)), f32_at(h, at(org, o))));
                h.write(at(rec, 0x1c + o), v);
            }
            h.write(at(rec, 0x18), i8_at(h, s.wrapping_sub(2)) as u32);
            if i8_at(h, s) > 0 {
                for (slot, off) in [(0x28u32, 0xcu32), (0x2c, 0x10)] {
                    let src = at(file, h.read::<u32>(at(s, off)));
                    pos_points(h, rec, slot, s, src, Some(org));
                }
            }
            for o in [0u32, 4, 8] {
                let v = x87::store(x87::sub(f32_at(h, at(s, 0x4c + o)), f32_at(h, at(org, o))));
                h.write(at(rec, 0x40 + o), v);
            }
            h.write(at(rec, 0x3c), i8_at(h, s.wrapping_sub(1)) as u32);
            if i8_at(h, at(s, 2)) > 0 {
                for (slot, off) in [(0x4cu32, 0x1cu32), (0x50, 0x20)] {
                    let src = at(file, h.read::<u32>(at(s, off)));
                    pos_points(h, rec, slot, at(s, 2), src, Some(org));
                }
            }
            if i8_at(h, at(s, 1)) > 0 {
                for (slot, off) in [(0x60u32, 0x14u32), (0x64, 0x18)] {
                    let src = at(file, h.read::<u32>(at(s, off)));
                    pos_points(h, rec, slot, at(s, 1), src, Some(org));
                }
            }
            let nsub = i8_at(h, at(s, 4));
            if nsub > 0 {
                let subs = alloc_clear(h, (nsub.wrapping_mul(0x14)) as u32);
                h.write(at(rec, 0x70), subs);
                let mut e = at(file, h.read::<u32>(at(s, 0x24)));
                let mut k = 0i32;
                while k < i8_at(h, at(s, 4)) {
                    let base: u32 = h.read(at(rec, 0x70));
                    let d = at(base, (k as u32).wrapping_mul(0x14));
                    for o in 0..3u32 {
                        h.write(at(d, 4 * o), i8_at(h, at(e, 4 + o)) as u32);
                    }
                    if h.read::<i32>(at(d, 8)) > 0 {
                        let np = i8_at(h, at(e, 6));
                        let p = cdecl(h, MS_MALLOC_7CF970, &[(np.wrapping_mul(12)) as u32]);
                        let base: u32 = h.read(at(rec, 0x70));
                        h.write(
                            at(base, (k as u32).wrapping_mul(0x14).wrapping_add(0x10)),
                            p,
                        );
                        let np = i8_at(h, at(e, 6));
                        let base: u32 = h.read(at(rec, 0x70));
                        let blk: u32 =
                            h.read(at(base, (k as u32).wrapping_mul(0x14).wrapping_add(0x10)));
                        cdecl(h, MS_STRUCT_CLEAR, &[blk, (np.wrapping_mul(12)) as u32]);
                        let mut src = at(file, h.read::<u32>(e));
                        let mut j = 0i32;
                        while j < i8_at(h, at(e, 6)) {
                            for o in [0u32, 4, 8] {
                                let v = x87::store(x87::sub(
                                    f32_at(h, at(src, o)),
                                    f32_at(h, at(org, o)),
                                ));
                                let base: u32 = h.read(at(rec, 0x70));
                                let blk: u32 = h.read(at(
                                    base,
                                    (k as u32).wrapping_mul(0x14).wrapping_add(0x10),
                                ));
                                h.write(at(blk, (j as u32).wrapping_mul(12).wrapping_add(o)), v);
                            }
                            src = at(src, 0x10);
                            j += 1;
                        }
                    }
                    e = at(e, 0x10);
                    k += 1;
                }
            }
            g += 1;
            let t = g32(h, BTL_POS_TABLE);
            if g >= h.read::<i32>(at(t, 0x1c)) {
                return;
            }
        }
    }

    // ---- batch 7: the battle debug environment file ----

    /// A value `MsDebugEnvSave` writes: a global (`i8`, `u8`, `u32`) or a
    /// getter's result.
    #[derive(Debug, Clone, Copy)]
    pub enum EnvValue {
        I8(u32),
        U8(u32),
        U32(u32),
        Call(u32),
    }

    /// The key table (`.data` `0x00c4_43f0`, 12-byte entries) and the
    /// writer `MsDebugEnvSave` fills.
    pub const ENV_KEYS: u32 = 0x00c4_43f0;
    pub const MS_ENV_FIND_KEY: u32 = 0x007c_f140;
    pub const MS_ENV_WRITE_TEXT_7CE9F0: u32 = 0x007c_e9f0;
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_WRITE: u32 = 0x0062_fdc0;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const MALLOC: u32 = 0x00b0_c468;
    pub const FREE: u32 = 0x00b0_c370;
    pub const S_ENV_HEADER: u32 = 0x00b5_5c1c;
    pub const S_ENV_ERROR: u32 = 0x00b5_54f8;
    pub const S_ENV_WRITTEN: u32 = 0x00b5_54e4;

    /// `MsDebugEnvSave`'s lines in order: (value, key, `"%s\n%d\n"` or
    /// `"%s\nx%x\n"`).
    pub const ENV_ROWS: [(EnvValue, u32, u32); 48] = [
        (EnvValue::I8(0x0112_a8f6), 0x1, 0x00b5_5c3c),
        (EnvValue::Call(0x007c_5f50), 0x2, 0x00b5_5c44),
        (EnvValue::I8(0x0112_a905), 0x18, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a8e1), 0x3, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a8f8), 0x4, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a8f9), 0xc, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a8f5), 0x5, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a8fc), 0x6, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a900), 0xa, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a901), 0xb, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a8fa), 0xd, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a8fb), 0xe, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a918), 0x1d, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a91c), 0x23, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a914), 0x20, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a902), 0x14, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a903), 0xf, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a922), 0x1a, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a915), 0x1b, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a917), 0x1f, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a908), 0x10, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a909), 0x11, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a90d), 0x21, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a90a), 0x12, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a904), 0x15, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a906), 0x16, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a907), 0x17, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a90b), 0x13, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a90c), 0x19, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a916), 0x1c, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a91a), 0x1e, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a91b), 0x22, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a91d), 0x24, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a91e), 0x25, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a91f), 0x26, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a920), 0x27, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a921), 0x28, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a924), 0x29, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a925), 0x2a, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a926), 0x2b, 0x00b5_5c3c),
        (EnvValue::I8(0x0112_a927), 0x2c, 0x00b5_5c3c),
        (EnvValue::Call(0x0088_7c30), 0x2d, 0x00b5_5c3c),
        (EnvValue::U32(0x0231_13e0), 0x2e, 0x00b5_5c3c),
        (EnvValue::Call(0x0088_7c20), 0x30, 0x00b5_5c3c),
        (EnvValue::Call(0x0078_51d0), 0x2f, 0x00b5_5c3c),
        (EnvValue::U8(0x0113_07e8), 0x7, 0x00b5_5c3c),
        (EnvValue::U8(0x0113_07e9), 0x8, 0x00b5_5c3c),
        (EnvValue::U8(0x0113_07ea), 0x9, 0x00b5_5c3c),
    ];

    /// `MsDebugEnvSave(name)`: the battle debug settings written to the
    /// file `name` (`sceOpen(name, 0x602)`; a 0x1000-byte writer at `ebp -
    /// 0x118`: buffer `malloc`ed, its line buffer at `ebp - 0x104`): a
    /// header and per [`ENV_ROWS`] the key's name (`MsEnvFindKey_007cf140`)
    /// and the value, each appended with `MsEnvWriteText_007ce9f0`; then
    /// the rest written, the buffer freed and the file closed. Returns -1,
    /// or 0 (after an error line) when the file doesn't open.
    pub fn env_save(h: &mut impl Host, ebp: u32, name: u32) -> u32 {
        let w = ebp.wrapping_sub(0x118);
        let buf = ebp.wrapping_sub(0x104);
        let fd = cdecl(h, SCE_OPEN, &[name, 0x602]);
        h.write(at(w, 8), fd);
        h.write(at(w, 4), 1u32);
        h.write(at(w, 0x10), 0x1000u32);
        let mem = crt(h, MALLOC, &[0x1000]);
        h.write(w, mem);
        h.write(at(w, 0xc), 0u32);
        if (fd as i32) < 0 {
            dbg(h, S_ENV_ERROR, &[name]);
            return 0;
        }
        sprintf(h, buf, S_ENV_HEADER, &[]);
        cdecl(h, MS_ENV_WRITE_TEXT_7CE9F0, &[w, buf]);
        for (v, key, fmt) in ENV_ROWS {
            let value = match v {
                EnvValue::I8(s) => i32::from(h.read::<i8>(h.rebase(s))) as u32,
                EnvValue::U8(s) => u32::from(h.read::<u8>(h.rebase(s))),
                EnvValue::U32(s) => g32(h, s),
                EnvValue::Call(f) => cdecl(h, f, &[]),
            };
            let keys = h.rebase(ENV_KEYS);
            let k = cdecl(h, MS_ENV_FIND_KEY, &[keys, key]);
            sprintf(h, buf, fmt, &[k, value]);
            cdecl(h, MS_ENV_WRITE_TEXT_7CE9F0, &[w, buf]);
        }
        dbg(h, S_ENV_WRITTEN, &[name]);
        let (fill, fd, mem): (i32, u32, u32) = (h.read(at(w, 0xc)), h.read(at(w, 8)), h.read(w));
        if fill > 0 && h.read::<u32>(at(w, 4)) != 0 {
            cdecl(h, SCE_WRITE, &[fd, mem, fill as u32]);
        }
        if mem != 0 {
            crt(h, FREE, &[mem]);
        }
        if (fd as i32) > 0 {
            cdecl(h, SCE_CLOSE, &[fd]);
        }
        u32::MAX
    }

    // ---- batch 8: the position editor's script writer ----

    /// `g_BtlPosHashList` (`0x00c44270`): the position script's keys.
    pub const POS_KEYS: u32 = 0x00c4_4270;
    pub const MS_ENV_WRITE_TEXT_7CD680: u32 = 0x007c_d680;
    /// The position writer's line buffer (`0x0113e390`) and cursors:
    /// the record `0x0113e27c`, the point `0x0113e280`, the sub-record
    /// `0x0113e284`.
    pub const POS_LINE: u32 = 0x0113_e390;
    pub const POS_CUR_REC: u32 = 0x0113_e27c;
    pub const POS_CUR_POINT: u32 = 0x0113_e280;
    pub const POS_CUR_SUB: u32 = 0x0113_e284;
    /// The parser's file name and line (`0x0113e28c`, `0x0113e288`), its
    /// error (`0x0113e238`), the focus flag `0x023114f4`.
    pub const POS_FILE_NAME: u32 = 0x0113_e28c;
    pub const POS_FILE_LINE: u32 = 0x0113_e288;
    pub const POS_ERROR: u32 = 0x0113_e238;
    pub const BTL_POS_FOCUS: u32 = 0x0231_14f4;

    /// One script line: the key's name (`MsEnvFindKey_007cf140`) and
    /// `vals` `sprintf`ed with `fmt` into the line buffer, appended with
    /// `MsEnvWriteText_007cd680`.
    fn pos_line(h: &mut impl Host, w: u32, key: u32, fmt: u32, vals: &[u32]) {
        let keys = h.rebase(POS_KEYS);
        let name = cdecl(h, MS_ENV_FIND_KEY, &[keys, key]);
        let mut a = vec![name];
        a.extend_from_slice(vals);
        let buf = h.rebase(POS_LINE);
        // call width: 11 (the buffer, the format, the name, three doubles
        // and an integer)
        sprintf(h, buf, fmt, &a);
        cdecl(h, MS_ENV_WRITE_TEXT_7CD680, &[w, buf]);
    }

    /// A bare line (`"\n"`).
    fn pos_newline(h: &mut impl Host, w: u32) {
        let buf = h.rebase(POS_LINE);
        sprintf(h, buf, S_NL, &[]);
        cdecl(h, MS_ENV_WRITE_TEXT_7CD680, &[w, buf]);
    }

    /// The doubles of the vector at `p`.
    fn vec3(h: &impl Host, p: u32) -> Vec<u32> {
        [0u32, 4, 8]
            .iter()
            .flat_map(|&o| dw(f32_at(h, at(p, o))))
            .collect()
    }

    /// The `count` points of the list at the record's `+list` (12 bytes
    /// apart), each through the point cursor as a `key` line.
    fn pos_point_lines(
        h: &mut impl Host,
        w: u32,
        cursor: u32,
        count: u32,
        list: u32,
        key: u32,
        fmt: u32,
    ) {
        let mut k = 0i32;
        loop {
            let r = g32(h, cursor);
            if k >= h.read::<i32>(at(r, count)) {
                return;
            }
            let p = at(h.read::<u32>(at(r, list)), (k as u32).wrapping_mul(12));
            set32(h, POS_CUR_POINT, p);
            let v = vec3(h, p);
            pos_line(h, w, key, fmt, &v);
            k += 1;
        }
    }

    /// `MsBtlPosWriteScr(name)`: the position editor's table written as
    /// position script to `name` (`sceOpen(name, 0x602)`; a 0x1000-byte
    /// writer at `ebp - 0x24`; the lines through the global buffer and
    /// cursors): per record its points, ends, lists and sub-records (the
    /// short form for a table with the flag), then the end key; written,
    /// freed, closed. Returns -1, or 0 after the parser's error lines
    /// (error 2) when the file doesn't open.
    #[allow(clippy::too_many_lines)]
    pub fn pos_write_scr(h: &mut impl Host, ebp: u32, name: u32) -> u32 {
        const F_KEY: u32 = 0x00b5_56b0;
        const F_INT1: u32 = 0x00b5_56b8;
        const F_HEAD: u32 = 0x00b5_56c4;
        const F_INT2: u32 = 0x00b5_56cc;
        const F_PT2: u32 = 0x00b5_56d8;
        const F_END1: u32 = 0x00b5_56e8;
        const F_VEC1: u32 = 0x00b5_56f0;
        const F_FLOAT1: u32 = 0x00b5_5700;
        const F_VEC2: u32 = 0x00b5_570c;
        const F_VEC2I: u32 = 0x00b5_571c;
        const F_SUB: u32 = 0x00b5_5730;
        const F_INT3: u32 = 0x00b5_5738;
        const F_PT3: u32 = 0x00b5_5744;
        const F_SUBEND: u32 = 0x00b5_5758;
        const F_FILEEND: u32 = 0x00b5_5760;
        let w = ebp.wrapping_sub(0x24);
        let fd = cdecl(h, SCE_OPEN, &[name, 0x602]);
        h.write(at(w, 8), fd);
        h.write(at(w, 4), 1u32);
        h.write(at(w, 0x10), 0x1000u32);
        let mem = crt(h, MALLOC, &[0x1000]);
        h.write(w, mem);
        h.write(at(w, 0xc), 0u32);
        if (fd as i32) < 0 {
            let f = g32(h, POS_FILE_NAME);
            if f != 0 {
                dbg(h, 0x00b5_5904, &[f]);
            }
            let l = g32(h, POS_FILE_LINE);
            if l != 0 {
                dbg(h, 0x00b5_590c, &[l]);
            }
            dbg(h, 0x00b5_5938, &[name]);
            set32(h, POS_ERROR, 2);
            return 0;
        }
        if g32(h, BTL_POS_FOCUS) != 0 {
            dbg(h, 0x00b5_56a4, &[name]);
        }
        pos_line(h, w, 1, F_KEY, &[]);
        let t = g32(h, BTL_POS_TABLE);
        let n: u32 = h.read(at(t, 0x1c));
        pos_line(h, w, 5, F_INT1, &[n]);
        let t = g32(h, BTL_POS_TABLE);
        let groups = |h: &_| -> i32 { Mem::read(h, at(g32(h, BTL_POS_TABLE), 0x1c)) };
        let rec_at = |h: &_, g: i32| -> u32 {
            at(
                Mem::read::<u32>(h, at(g32(h, BTL_POS_TABLE), 0x30)),
                (g as u32).wrapping_mul(0x74),
            )
        };
        if h.read::<u32>(t) != 0 {
            pos_line(h, w, 0x1a, F_HEAD, &[]);
            let mut g = 0;
            while g < groups(h) {
                let r = rec_at(h, g);
                set32(h, POS_CUR_REC, r);
                pos_line(h, w, 6, F_HEAD, &[]);
                let c: u32 = h.read(at(g32(h, POS_CUR_REC), 0xc));
                pos_line(h, w, 9, F_INT2, &[c]);
                pos_point_lines(h, w, POS_CUR_REC, 0xc, 0x28, 0xb, F_PT2);
                pos_point_lines(h, w, POS_CUR_REC, 0xc, 0x2c, 0x1c, F_PT2);
                pos_newline(h, w);
                pos_line(h, w, 7, F_END1, &[]);
                g += 1;
            }
        } else {
            let v = vec3(h, at(t, 4));
            pos_line(h, w, 3, F_VEC1, &v);
            let t = g32(h, BTL_POS_TABLE);
            let v = vec3(h, at(t, 0x10));
            pos_line(h, w, 4, F_VEC1, &v);
            let t = g32(h, BTL_POS_TABLE);
            let nf: u32 = h.read(at(t, 0x24));
            pos_line(h, w, 0x18, F_INT1, &[nf]);
            let mut k = 0i32;
            loop {
                let t = g32(h, BTL_POS_TABLE);
                if k >= h.read::<i32>(at(t, 0x24)) {
                    break;
                }
                let list: u32 = h.read(at(t, 0x2c));
                let v = dw(f32_at(h, at(list, (k as u32) * 4)));
                pos_line(h, w, 0x19, F_FLOAT1, &v);
                k += 1;
            }
            let mut g = 0;
            while g < groups(h) {
                let r = rec_at(h, g);
                set32(h, POS_CUR_REC, r);
                let cur = |h: &_| g32(h, POS_CUR_REC);
                pos_line(h, w, 6, F_HEAD, &[]);
                let v = vec3(h, cur(h));
                pos_line(h, w, 8, F_VEC2, &v);
                let mut v = vec3(h, at(cur(h), 0x1c));
                v.push(h.read(at(cur(h), 0x18)));
                // call width: 11 (three doubles and an integer)
                pos_line(h, w, 0xa, F_VEC2I, &v);
                let c: u32 = h.read(at(cur(h), 0xc));
                pos_line(h, w, 9, F_INT2, &[c]);
                pos_point_lines(h, w, POS_CUR_REC, 0xc, 0x28, 0xb, F_PT2);
                pos_point_lines(h, w, POS_CUR_REC, 0xc, 0x2c, 0x1c, F_PT2);
                pos_newline(h, w);
                let mut v = vec3(h, at(cur(h), 0x40));
                v.push(h.read(at(cur(h), 0x3c)));
                // call width: 11 (three doubles and an integer)
                pos_line(h, w, 0xd, F_VEC2I, &v);
                let c: u32 = h.read(at(cur(h), 0x30));
                pos_line(h, w, 0xc, F_INT2, &[c]);
                pos_point_lines(h, w, POS_CUR_REC, 0x30, 0x4c, 0xe, F_PT2);
                pos_point_lines(h, w, POS_CUR_REC, 0x30, 0x50, 0x1d, F_PT2);
                pos_newline(h, w);
                let c: u32 = h.read(at(cur(h), 0x54));
                pos_line(h, w, 0x1b, F_INT2, &[c]);
                pos_point_lines(h, w, POS_CUR_REC, 0x54, 0x60, 0xf, F_PT2);
                pos_point_lines(h, w, POS_CUR_REC, 0x54, 0x64, 0x1e, F_PT2);
                pos_newline(h, w);
                let c: u32 = h.read(at(cur(h), 0x68));
                pos_line(h, w, 0x10, F_INT2, &[c]);
                let mut s = 0i32;
                while s < h.read::<i32>(at(cur(h), 0x68)) {
                    let sub = at(
                        h.read::<u32>(at(cur(h), 0x70)),
                        (s as u32).wrapping_mul(0x14),
                    );
                    set32(h, POS_CUR_SUB, sub);
                    pos_line(h, w, 0x11, F_SUB, &[]);
                    for (k, o) in [(0x13u32, 0u32), (0x14, 4), (0x15, 8)] {
                        let v: u32 = h.read(at(g32(h, POS_CUR_SUB), o));
                        pos_line(h, w, k, F_INT3, &[v]);
                    }
                    pos_point_lines(h, w, POS_CUR_SUB, 8, 0x10, 0x16, F_PT3);
                    pos_line(h, w, 0x12, F_SUBEND, &[]);
                    s += 1;
                }
                pos_line(h, w, 7, F_END1, &[]);
                g += 1;
            }
        }
        pos_line(h, w, 2, F_FILEEND, &[]);
        let (fill, fd, mem): (i32, u32, u32) = (h.read(at(w, 0xc)), h.read(at(w, 8)), h.read(w));
        if fill > 0 && h.read::<u32>(at(w, 4)) != 0 {
            cdecl(h, SCE_WRITE, &[fd, mem, fill as u32]);
        }
        if mem != 0 {
            crt(h, FREE, &[mem]);
        }
        if (fd as i32) > 0 {
            cdecl(h, SCE_CLOSE, &[fd]);
        }
        u32::MAX
    }

    // ---- batch 9: the position script parser ----

    pub const MS_KEYWORD_LOOKUP: u32 = 0x007c_eeb0;
    pub const MS_READ_VEC3: u32 = 0x007c_d170;
    pub const MS_READ_INT: u32 = 0x007c_d420;
    pub const MS_READ_FLOAT: u32 = 0x007c_d360;
    pub const MS_READ_TOKEN: u32 = 0x007c_f180;
    pub const MS_BTL_POS_CHECK_VERSION: u32 = 0x007c_c1f0;
    pub const MS_BTL_POS_CHECK_VERSION2: u32 = 0x007c_c250;
    pub const MS_BTL_POS_ALLOC_POINTS: u32 = 0x007c_d4e0;
    /// The code `MsBtlPosParseToken_007cc420` tail-jumps to for key 0xc:
    /// the record's `+0x30` lists allocated (not a function of its own).
    pub const MS_BTL_POS_ALLOC_SIDE_0X30: u32 = 0x007c_91a0;
    /// Its key 9 target: the `+0xc` lists (`0x007cc180`).
    pub const MS_BTL_POS_ALLOC_SIDE_0XC: u32 = 0x007c_c180;
    /// The parser's state (`0x0113e270`: 0 outside, 1 in the data, 2 in a
    /// record, 3 in a sub-record), the sub-state `0x0113e274`, the
    /// table it fills (`0x0113e23c`).
    pub const POS_STATE: u32 = 0x0113_e270;
    pub const POS_SUB_STATE: u32 = 0x0113_e274;
    pub const POS_PARSE_TABLE: u32 = 0x0113_e23c;
    pub const POS_FILE_NAME_BUF: u32 = 0x0113_e290;
    pub const S_POS_BAD_STATE: u32 = 0x00b5_59f4;

    /// The parser's error: the file and line when known, `fmt` with the
    /// token, the error `code` (`0x0113e238`).
    fn pos_error(h: &mut impl Host, fmt: u32, name: u32, code: u32) {
        let f = g32(h, POS_FILE_NAME);
        if f != 0 {
            dbg(h, 0x00b5_5904, &[f]);
        }
        let l = g32(h, POS_FILE_LINE);
        if l != 0 {
            dbg(h, 0x00b5_590c, &[l]);
        }
        dbg(h, fmt, &[name]);
        set32(h, POS_ERROR, code);
    }

    /// The next point of a record list while in state `state`: the count
    /// `+count`, the index `+index` (raised), the list `+list` of the
    /// cursor at `cur`; the point (cursor `0x0113e280`) read as three
    /// floats. Else the state error.
    #[allow(clippy::too_many_arguments)]
    fn pos_parse_point(
        h: &mut impl Host,
        reader: u32,
        name: u32,
        state: u32,
        cur: u32,
        index: u32,
        count: u32,
        list: u32,
    ) {
        let c = g32(h, cur);
        let i: i32 = h.read(at(c, index));
        if g32(h, POS_STATE) == state && i < h.read::<i32>(at(c, count)) {
            let p = at(h.read::<u32>(at(c, list)), (i as u32).wrapping_mul(12));
            set32(h, POS_CUR_POINT, p);
            cdecl(h, MS_READ_VEC3, &[reader, name, p, at(p, 4), at(p, 8)]);
            let c = g32(h, cur);
            let i: u32 = h.read(at(c, index));
            h.write(at(c, index), i.wrapping_add(1));
            return;
        }
        pos_error(h, S_POS_BAD_STATE, name, 7);
    }

    /// `MsStructClear` as the parser tail-jumps to it: its two words in the
    /// parser's own argument slots.
    fn pos_tail_clear(h: &mut impl Host, slots: u32, obj: u32, cnt: u32, size: u32, list: u32) {
        let o = g32(h, obj);
        let n: i32 = h.read(at(o, cnt));
        let p = cdecl(h, MS_MALLOC_7CF970, &[(n.wrapping_mul(size as i32)) as u32]);
        let o = g32(h, obj);
        h.write(at(o, list), p);
        let n: i32 = h.read(at(o, cnt));
        let bytes = (n.wrapping_mul(size as i32)) as u32;
        h.write(slots, [p, bytes]);
        cdecl(h, MS_STRUCT_CLEAR, &[p, bytes]);
    }

    /// `MsBtlPosParseToken_007cc420(reader, name)` (name ours): one token
    /// of the position script (`MsKeywordLookup_007ceeb0` over
    /// `g_BtlPosHashList`; unknown: error 8): the data and record
    /// brackets (state `0x0113e270`), the table's scale and offset, the
    /// record count and records (allocated), each record's points, ends,
    /// lists and sub-records, the float list, the file's line and name, the
    /// flag. Values through `MsReadVec3`/`MsReadInt`/`MsReadFloat`; a
    /// token in the wrong state is error 7. The tail-jumps to
    /// `MsStructClear` leave their words in the argument slots. Callers
    /// drop `eax`.
    #[allow(clippy::too_many_lines)]
    pub fn pos_parse_token(h: &mut impl Host, reader: u32, name: u32, slots: u32) {
        let keys = h.rebase(POS_KEYS);
        let key = cdecl(h, MS_KEYWORD_LOOKUP, &[name, keys]);
        if key == 0 {
            pos_error(h, 0x00b5_5a18, name, 8);
        }
        let t = |h: &_| g32(h, BTL_POS_TABLE);
        let c = |h: &_| g32(h, POS_CUR_REC);
        let check = |h: &mut _, v: u32| cdecl(h, MS_BTL_POS_CHECK_VERSION, &[name, v]) != 0;
        let read_int = |h: &mut _, p: u32| {
            cdecl(h, MS_READ_INT, &[reader, name, p]);
        };
        let read_vec = |h: &mut _, p: u32| {
            cdecl(h, MS_READ_VEC3, &[reader, name, p, at(p, 4), at(p, 8)]);
        };
        match key {
            1 => {
                if g32(h, POS_STATE) != 0 {
                    pos_error(h, S_POS_BAD_STATE, name, 7);
                } else {
                    set32(h, POS_STATE, 1);
                    let p = h.rebase(POS_PARSE_TABLE);
                    set32(h, BTL_POS_TABLE, p);
                }
            }
            2 => {
                if g32(h, POS_STATE) == 0 {
                    pos_error(h, S_POS_BAD_STATE, name, 7);
                } else {
                    set32(h, POS_STATE, 0);
                }
            }
            3 | 4 => {
                if check(h, 1) {
                    let p = at(t(h), if key == 3 { 4 } else { 0x10 });
                    read_vec(h, p);
                }
            }
            5 => {
                if check(h, 1) {
                    if h.read::<u32>(at(t(h), 0x1c)) != 0 {
                        pos_error(h, 0x00b5_5994, name, 4);
                        return;
                    }
                    read_int(h, at(t(h), 0x1c));
                    h.write(at(t(h), 0x20), 0u32);
                    pos_tail_clear(h, slots, BTL_POS_TABLE, 0x1c, 0x74, 0x30);
                }
            }
            6 => {
                let tb = t(h);
                let i: i32 = h.read(at(tb, 0x20));
                if g32(h, POS_STATE) == 1 && i < h.read::<i32>(at(tb, 0x1c)) {
                    set32(h, POS_STATE, 2);
                    let r = at(h.read::<u32>(at(tb, 0x30)), (i as u32).wrapping_mul(0x74));
                    set32(h, POS_CUR_REC, r);
                } else {
                    pos_error(h, S_POS_BAD_STATE, name, 7);
                }
            }
            7 => {
                if check(h, 2) {
                    let tb = t(h);
                    let i: u32 = h.read(at(tb, 0x20));
                    h.write(at(tb, 0x20), i.wrapping_add(1));
                    set32(h, POS_STATE, 1);
                }
            }
            8 => {
                if check(h, 2) {
                    read_vec(h, c(h));
                }
            }
            9 => {
                if check(h, 2) {
                    read_int(h, at(c(h), 0xc));
                    cdecl(h, MS_BTL_POS_ALLOC_SIDE_0XC, &[]);
                }
            }
            0xa | 0xd => {
                if check(h, 2) {
                    let (v, i) = if key == 0xa {
                        (0x1c, 0x18)
                    } else {
                        (0x40, 0x3c)
                    };
                    read_vec(h, at(c(h), v));
                    read_int(h, at(c(h), i));
                }
            }
            0xb => pos_parse_point(h, reader, name, 2, POS_CUR_REC, 0x10, 0xc, 0x28),
            0xc => {
                if check(h, 2) {
                    read_int(h, at(c(h), 0x30));
                    cdecl(h, MS_BTL_POS_ALLOC_SIDE_0X30, &[]);
                }
            }
            0xe => pos_parse_point(h, reader, name, 2, POS_CUR_REC, 0x34, 0x30, 0x4c),
            0xf => {
                if h.read::<u32>(at(c(h), 0x54)) == 0 {
                    h.write(at(c(h), 0x54), 1u32);
                    cdecl(h, MS_BTL_POS_ALLOC_POINTS, &[]);
                }
                pos_parse_point(h, reader, name, 2, POS_CUR_REC, 0x58, 0x54, 0x60);
            }
            0x10 => {
                if check(h, 2) {
                    read_int(h, at(c(h), 0x68));
                    h.write(at(c(h), 0x6c), 0u32);
                    pos_tail_clear(h, slots, POS_CUR_REC, 0x68, 0x14, 0x70);
                }
            }
            0x11 => {
                let r = c(h);
                let i: i32 = h.read(at(r, 0x6c));
                if g32(h, POS_STATE) == 2 && i < h.read::<i32>(at(r, 0x68)) {
                    set32(h, POS_STATE, 3);
                    set32(h, POS_SUB_STATE, 3);
                    let s = at(h.read::<u32>(at(r, 0x70)), (i as u32).wrapping_mul(0x14));
                    set32(h, POS_CUR_SUB, s);
                } else {
                    pos_error(h, S_POS_BAD_STATE, name, 7);
                }
            }
            0x12..=0x15 => {
                if cdecl(h, MS_BTL_POS_CHECK_VERSION2, &[name, 3, 3]) != 0 {
                    let s = g32(h, POS_CUR_SUB);
                    match key {
                        0x12 => {
                            let r = c(h);
                            let i: u32 = h.read(at(r, 0x6c));
                            h.write(at(r, 0x6c), i.wrapping_add(1));
                            set32(h, POS_STATE, 2);
                            set32(h, POS_SUB_STATE, 0);
                        }
                        0x13 => read_int(h, s),
                        0x14 => read_int(h, at(s, 4)),
                        _ => {
                            read_int(h, at(s, 8));
                            h.write(at(g32(h, POS_CUR_SUB), 0xc), 0u32);
                            pos_tail_clear(h, slots, POS_CUR_SUB, 8, 12, 0x10);
                        }
                    }
                }
            }
            0x16 => pos_parse_point(h, reader, name, 3, POS_CUR_SUB, 0xc, 8, 0x10),
            0x17 => {
                let line = h.rebase(POS_FILE_LINE);
                read_int(h, line);
                let buf = h.rebase(POS_FILE_NAME_BUF);
                cdecl(h, MS_READ_TOKEN, &[reader, buf, 0x100]);
                let l = g32(h, POS_FILE_LINE);
                set32(h, POS_FILE_LINE, l.wrapping_sub(1));
                set32(h, POS_FILE_NAME, buf);
            }
            0x18 => {
                if !check(h, 1) {
                    pos_error(h, S_POS_BAD_STATE, name, 7);
                    return;
                }
                read_int(h, at(t(h), 0x24));
                h.write(at(t(h), 0x28), 0u32);
                pos_tail_clear(h, slots, BTL_POS_TABLE, 0x24, 4, 0x2c);
            }
            0x19 => {
                if !check(h, 1) {
                    pos_error(h, S_POS_BAD_STATE, name, 7);
                    return;
                }
                let tb = t(h);
                let (n, i): (i32, i32) = (h.read(at(tb, 0x24)), h.read(at(tb, 0x28)));
                if n <= i {
                    pos_error(h, 0x00b5_5a34, name, 9);
                    return;
                }
                let p = at(h.read::<u32>(at(tb, 0x2c)), (i as u32).wrapping_mul(4));
                cdecl(h, MS_READ_FLOAT, &[reader, name, p]);
                let tb = t(h);
                let i: u32 = h.read(at(tb, 0x28));
                h.write(at(tb, 0x28), i.wrapping_add(1));
            }
            0x1a => h.write(t(h), 1u32),
            0x1b => {
                if check(h, 2) {
                    read_int(h, at(c(h), 0x54));
                    cdecl(h, MS_BTL_POS_ALLOC_POINTS, &[]);
                }
            }
            0x1c => pos_parse_point(h, reader, name, 2, POS_CUR_REC, 0x14, 0xc, 0x2c),
            0x1d => pos_parse_point(h, reader, name, 2, POS_CUR_REC, 0x38, 0x30, 0x50),
            0x1e => pos_parse_point(h, reader, name, 2, POS_CUR_REC, 0x5c, 0x54, 0x64),
            _ => {}
        }
    }

    // ---- batch 10: the battle debug windows' procedures ----

    pub const GU_WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
    pub const GU_WINDOW_ADD_BUTTON: u32 = 0x0084_a470;
    pub const GU_WINDOW_SET_BUTTON_TEXT: u32 = 0x0084_a710;
    pub const MS_AUTO_REPEAT_INIT: u32 = 0x007c_89a0;
    pub const MS_AUTO_REPEAT_CHECK: u32 = 0x007c_8950;
    pub const FFX_GET_ASIA_MODE: u32 = 0x0088_7c20;
    pub const TK_GET_LANG: u32 = 0x0088_7c30;
    pub const MS_GET_SAVE_CONFIG_ENGLISH: u32 = 0x0078_51d0;

    /// `Gu_WindowAddButton(win, id, label, x, y, w, h)`.
    fn button(h: &mut impl Host, win: u32, id: u32, label: u32, rect: [u32; 4]) {
        cdecl(
            h,
            GU_WINDOW_ADD_BUTTON,
            &[win, id, label, rect[0], rect[1], rect[2], rect[3]],
        );
    }

    /// A `.rdata` string's address.
    fn s(h: &impl Host, sym: u32) -> u32 {
        h.rebase(sym)
    }

    /// The word `i` of the table at `table` (`.data`).
    fn table_word(h: &impl Host, table: u32, i: u32) -> u32 {
        h.read(at(h.rebase(table), i.wrapping_mul(4)))
    }

    /// `Gu_PrintSetX(x)`, `Gu_PrintSetY(y)`.
    fn print_xy(h: &mut impl Host, x: u32, y: u32) {
        cdecl(h, GU_PRINT_SET_X, &[x]);
        cdecl(h, GU_PRINT_SET_Y, &[y]);
    }

    /// `Gu_Printf(text)` (the text as its own format).
    fn gu_print(h: &mut impl Host, text: u32) {
        cdecl(h, GU_PRINTF, &[text]);
    }

    pub const S_PARTY_SLOT: u32 = 0x0113_07e8;
    pub const MS_EXCHANGE_SAVE_PARTY: u32 = 0x0078_4d00;
    pub const MS_GET_SAVE_PLY_JOIN: u32 = 0x0078_5380;
    pub const MS_DEBUG_NEXT_MEMBER: u32 = 0x007c_60a0;
    pub const MS_DEBUG_SCENE_WALK: u32 = 0x007c_5dd0;
    pub const MS_BATTLE_EXE: u32 = 0x0078_1020;
    pub const G_BTL_STATUS: u32 = 0x0112_a8e0;
    /// The scene picker's field (`0x00c44248`) and encounter
    /// (`0x01139e44`), the master mode (`i8 0x0112a8f6`: PC, CD, local).
    pub const DEBUG_FIELD: u32 = 0x00c4_4248;
    pub const DEBUG_ENCOUNTER: u32 = 0x0113_9e44;
    pub const DEBUG_MASTER: u32 = 0x0112_a8f6;
    pub const DEBUG_VIDEO_PAL: u32 = 0x0231_13e0;

    /// The console's party buttons: the next joined character up from
    /// party slot `slot`'s that no other slot holds (0xff past 0x11).
    fn console_party_up(h: &mut impl Host, slot: u32) {
        let slots = h.rebase(S_PARTY_SLOT);
        let mut c: u8 = h.read(at(slots, slot));
        let chosen = 'outer: loop {
            c = c.wrapping_add(1);
            if c >= 0x12 {
                break 0xff;
            }
            if c == 0xff {
                break 0xff;
            }
            if cdecl(h, MS_GET_SAVE_PLY_JOIN, &[u32::from(c)]) == 0 {
                continue;
            }
            for k in 0..3u32 {
                if k != slot && c == h.read::<u8>(at(slots, k)) {
                    continue 'outer;
                }
            }
            break c;
        };
        cdecl(h, MS_EXCHANGE_SAVE_PARTY, &[u32::from(chosen), slot]);
    }

    /// The new language of the console's language buttons: `down` the
    /// first table, else the second; other languages stay.
    fn console_lang(lang: u32, down: bool) -> u32 {
        let next: &[(u32, u32)] = if down {
            &[(2, 4), (4, 1), (5, 2), (3, 5), (0x12, 3), (0, 0x12), (1, 0)]
        } else {
            &[(1, 4), (0, 1), (4, 2), (2, 5), (5, 3), (3, 0x12), (0x12, 0)]
        };
        next.iter().find(|p| p.0 == lang).map_or(lang, |p| p.1)
    }

    pub const TK_SET_LANG: u32 = 0x0088_7ca0;
    pub const MS_GET_KUNI: u32 = 0x0078_2c00;
    pub const MS_SET_SAVE_CONFIG_ENGLISH: u32 = 0x0078_5ed0;
    pub const FFX_SET_ASIA_MODE: u32 = 0x0088_7c70;

    /// `Ms_Gui_BattleConsoleProc(win, msg, id)`: the battle console debug
    /// window's procedure. 0x8001 its buttons (scene, encounter, party,
    /// master, save/reload/exe, language, video) and auto-repeats;
    /// 0x8003 closed; 0x8006/0x800a/0x800e the auto-repeats reset;
    /// 0x8005/0x8009/0x800d a button (steps 1, 10, 100 by the mouse
    /// button): the field and encounter, the master mode, the battle
    /// started (EXE, SREL: everything reloaded first), saved
    /// (`MsDebugEnvSave`), the party slots, the language, PAL, English,
    /// Asia; 0x8011 its texts. `ebp` is the original's frame (the line
    /// buffer at `ebp - 0x104`, the region at `- 0x108`). Returns 0.
    #[allow(clippy::too_many_lines)]
    pub fn console_proc(h: &mut impl Host, ebp: u32, win: u32, msg: u32, id: u32) -> u32 {
        const LT: u32 = 0x00b5_5220;
        const GT: u32 = 0x00b4_8250;
        let buf = ebp.wrapping_sub(0x104);
        match msg {
            0x8001 => {
                let t = s(h, 0x00b5_5210);
                cdecl(h, GU_WINDOW_SET_TITLE, &[win, t]);
                for (bid, label, x) in [
                    (0x3bu32, LT, 0x1eu32),
                    (0x3c, GT, 0x60),
                    (0x3d, LT, 0x74),
                    (0x3e, GT, 0xbc),
                    (0x4d, 0x00b3_0da0, 0xd0),
                    (0x3f, 0x00b5_5224, 0xe4),
                ] {
                    let l = s(h, label);
                    button(h, win, bid, l, [x, 0, 0x12, 8]);
                }
                for rep in [0x0113_9e58u32, 0x0113_9e6c, 0x0113_9e80] {
                    let r = h.rebase(rep);
                    cdecl(h, MS_AUTO_REPEAT_INIT, &[r, 0xf, 3]);
                }
                for (bid, label, x) in [
                    (0x61u32, LT, 4u32),
                    (0x62, GT, 0x40),
                    (0x63, LT, 0x56),
                    (0x64, GT, 0x92),
                    (0x65, LT, 0xa8),
                    (0x66, GT, 0xe4),
                ] {
                    let l = s(h, label);
                    button(h, win, bid, l, [x, 0x14, 0x12, 8]);
                }
                let m = i32::from(h.read::<i8>(h.rebase(DEBUG_MASTER))) as u32;
                let l = table_word(h, 0x00c4_424c, m);
                button(h, win, 0x5e, l, [2, 0x1e, 0x52, 8]);
                for (bid, label, x) in [
                    (0x5fu32, 0x00b5_5228u32, 0x5au32),
                    (0x4d, 0x00b5_5230, 0x90),
                    (0x3f, 0x00b5_5238, 0xc6),
                ] {
                    let l = s(h, label);
                    button(h, win, bid, l, [x, 0x1e, 0x30, 8]);
                }
                let asia = cdecl(h, FFX_GET_ASIA_MODE, &[]);
                let l = table_word(h, 0x00c4_4258, asia);
                button(h, win, 0xfa, l, [0x40, 0x28, 0x12, 8]);
                for (bid, label, x) in [
                    (0x67u32, LT, 0x56u32),
                    (0x68, GT, 0x92),
                    (0x69, LT, 0xa8),
                    (0x6a, GT, 0xe4),
                ] {
                    let l = s(h, label);
                    button(h, win, bid, l, [x, 0x28, 0x12, 8]);
                }
            }
            0x8003 => set32(h, 0x0113_9ef8, 0),
            0x8006 | 0x800a | 0x800e => {
                let rep = match msg {
                    0x8006 => 0x0113_9e58,
                    0x800e => 0x0113_9e6c,
                    _ => 0x0113_9e80,
                };
                let r = h.rebase(rep);
                cdecl(h, MS_AUTO_REPEAT_INIT, &[r, 0xf, 3]);
            }
            0x8005 | 0x8009 | 0x800d => {
                let (rep, step) = match msg {
                    0x8005 => (0x0113_9e58, 1u32),
                    0x800d => (0x0113_9e6c, 100),
                    _ => (0x0113_9e80, 10),
                };
                let r = h.rebase(rep);
                if cdecl(h, MS_AUTO_REPEAT_CHECK, &[r, id]) == 0 {
                    return 0;
                }
                console_button(h, msg, id, step);
            }
            0x8011 => console_paint(h, ebp, buf, win),
            _ => {}
        }
        0
    }

    /// A console button (after its auto-repeat).
    fn console_button(h: &mut impl Host, msg: u32, id: u32, step: u32) {
        let field = h.rebase(DEBUG_FIELD);
        let enc = h.rebase(DEBUG_ENCOUNTER);
        let idle = |h: &_| matches!(Mem::read::<u8>(h, Mem::rebase(h, G_BTL_STATUS)), 0 | 1);
        let exe = |h: &mut _| {
            let w = [
                0,
                g32(h, DEBUG_FIELD),
                g32(h, 0x0113_9e48),
                g32(h, 0x0113_9e4c),
            ];
            cdecl(h, MS_BATTLE_EXE, &w);
        };
        match id {
            0x3b | 0x3c => {
                let v: u32 = h.read(field);
                h.write(
                    field,
                    if id == 0x3b {
                        v.wrapping_sub(step)
                    } else {
                        v.wrapping_add(step)
                    },
                );
            }
            0x3d | 0x3e => {
                let v: u32 = h.read(enc);
                h.write(
                    enc,
                    if id == 0x3d {
                        v.wrapping_sub(step)
                    } else {
                        v.wrapping_add(step)
                    },
                );
            }
            0x5e => {
                let m = h.rebase(DEBUG_MASTER);
                if msg == 0x8005 {
                    let v = i32::from(h.read::<u8>(m).wrapping_add(1) as i8);
                    h.write(m, (v % 3) as u8);
                } else {
                    let v = h.read::<u8>(m).wrapping_sub(1);
                    h.write(m, if (v as i8) < 0 { 2 } else { v });
                }
            }
            0x5f => {
                if idle(h) {
                    let name = h.rebase(0x0231_1400);
                    cdecl(h, 0x007c_d800, &[name]);
                }
            }
            0x4e => {
                cdecl(h, 0x0078_6b00, &[]);
            }
            0x4d => {
                if idle(h) {
                    for f in [
                        0x007f_f8e0u32,
                        0x008a_b590,
                        0x007a_00b0,
                        0x007b_d5d0,
                        0x0078_0b40,
                        0x0079_e630,
                        0x0078_1d00,
                        0x0078_2770,
                        0x0078_1700,
                        0x0078_2740,
                        MS_DEBUG_SCENE_WALK,
                    ] {
                        cdecl(h, f, &[]);
                    }
                    exe(h);
                }
            }
            0x3f => {
                if idle(h) {
                    for f in [
                        0x007f_f8e0u32,
                        0x008a_b590,
                        MS_DEBUG_SCENE_WALK,
                        0x007a_00b0,
                    ] {
                        cdecl(h, f, &[]);
                    }
                    exe(h);
                }
            }
            0x61 | 0x63 | 0x65 => {
                let slot = (id - 0x61) / 2;
                let c = cdecl(h, MS_DEBUG_NEXT_MEMBER, &[slot, u32::MAX]) & 0xff;
                cdecl(h, MS_EXCHANGE_SAVE_PARTY, &[c, slot]);
            }
            0x62 | 0x64 | 0x66 => console_party_up(h, (id - 0x62) / 2),
            0x67 | 0x68 => {
                let lang = cdecl(h, TK_GET_LANG, &[]);
                let new = console_lang(lang, id == 0x67);
                cdecl(h, TK_SET_LANG, &[new]);
                let k = cdecl(h, MS_GET_KUNI, &[0]);
                set32(h, DEBUG_VIDEO_PAL, k);
            }
            0x69 | 0x6a => {
                let v = g32(h, DEBUG_VIDEO_PAL);
                set32(h, DEBUG_VIDEO_PAL, u32::from(v == 0));
            }
            0xe9 | 0xea => {
                cdecl(h, MS_SET_SAVE_CONFIG_ENGLISH, &[id - 0xe9]);
            }
            0xfa => {
                let a = cdecl(h, FFX_GET_ASIA_MODE, &[]);
                cdecl(h, FFX_SET_ASIA_MODE, &[a.wrapping_sub(1) & 1]);
            }
            _ => {}
        }
    }

    pub const MS_DEBUG_SCENE_NAME: u32 = 0x007c_5fc0;
    pub const MS_STR_COPY_7CF6C0: u32 = 0x007c_f6c0;

    /// The console's 0x8011 texts.
    fn console_paint(h: &mut impl Host, ebp: u32, buf: u32, win: u32) {
        let m = i32::from(h.read::<i8>(h.rebase(DEBUG_MASTER))) as u32;
        let l = table_word(h, 0x00c4_424c, m);
        cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, 0x5e, l]);
        let asia = cdecl(h, FFX_GET_ASIA_MODE, &[]);
        let l = table_word(h, 0x00c4_4258, asia);
        cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, 0xfa, l]);
        let e = cdecl(h, MS_GET_SAVE_CONFIG_ENGLISH, &[]);
        let l: u32 = h.read(h.rebase(0x00c4_4690).wrapping_sub(e.wrapping_shl(2)));
        cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, 0xe9, l]);
        let e = cdecl(h, MS_GET_SAVE_CONFIG_ENGLISH, &[]);
        let l = table_word(h, 0x00c4_468c, e);
        cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, 0xea, l]);
        cdecl(h, MS_DEBUG_SCENE_WALK, &[]);
        print_xy(h, 2, 1);
        sprintf(h, buf, 0x00b5_523c, &[]);
        gu_print(h, buf);
        print_xy(h, 0x3a, 1);
        let f = g32(h, DEBUG_FIELD);
        sprintf(h, buf, 0x00b5_4c6c, &[f]);
        gu_print(h, buf);
        print_xy(h, 0x8e, 1);
        let grp = g32(h, 0x0113_9e98);
        let n = u32::from(h.read::<u8>(grp)).wrapping_sub(1);
        let e = g32(h, DEBUG_ENCOUNTER);
        sprintf(h, buf, 0x00b5_5240, &[e, n]);
        gu_print(h, buf);
        print_xy(h, 2, 0xb);
        cdecl(h, MS_DEBUG_SCENE_NAME, &[1, buf]);
        gu_print(h, buf);
        print_xy(h, 0x60, 0xb);
        let rec = g32(h, 0x0113_9e9c);
        let c = (u32::from(h.read::<u8>(at(rec, 2))) << 8) + u32::from(h.read::<u8>(at(rec, 1)));
        let b4 = u32::from(h.read::<u8>(at(rec, 4)));
        let b_ea0 = u32::from(h.read::<u8>(at(g32(h, 0x0113_9ea0), 1))) & 0xf;
        let a = u32::from(h.read::<u8>(at(g32(h, 0x0113_9e98), 1)));
        let g = g32(h, 0x0113_9e48).wrapping_add(1);
        let fmt = if c & 0x400 != 0 {
            0x00b5_5248
        } else {
            0x00b5_5260
        };
        sprintf(h, buf, fmt, &[g, a, c & 0x3ff, b_ea0, b4]);
        gu_print(h, buf);
        let slots = h.rebase(S_PARTY_SLOT);
        for i in 0..3u32 {
            print_xy(h, 0x1a + 0x52 * i, 0x15);
            let c: u8 = h.read(at(slots, i));
            if c == 0xff {
                sprintf(h, buf, 0x00b5_5278, &[]);
            } else {
                let name = table_word(h, 0x00c4_4950, u32::from(c));
                cdecl(h, MS_STR_COPY_7CF6C0, &[name, buf, 5]);
            }
            gu_print(h, buf);
        }
        print_xy(h, 2, 0x29);
        let k = s(h, 0x00b5_5280);
        gu_print(h, k);
        let region = ebp.wrapping_sub(0x108);
        cdecl(h, MS_GET_KUNI, &[region]);
        print_xy(h, 0x74, 0x29);
        gu_print(h, region);
        print_xy(h, 0xbe, 0x29);
        let v = if g32(h, DEBUG_VIDEO_PAL) == 0 {
            0x00b5_5288
        } else {
            0x00b5_5290
        };
        let v = s(h, v);
        gu_print(h, v);
    }

    /// `g_DebugChrSel` (`0x0113f8a0`, name ours): the actor the battle
    /// CHR data windows show.
    pub const DEBUG_CHR_SEL: u32 = 0x0113_f8a0;
    pub const MS_GET_PLY_SAVE_NAME: u32 = 0x007d_cec0;
    pub const MS_PLY_SAVE: u32 = 0x007d_d5e0;
    pub const MS_SET_FILE_NAME: u32 = 0x007d_ce80;
    pub const MS_MON_SAVE: u32 = 0x007d_d480;
    pub const MS_GET_RAM_CHR_MONSTER: u32 = 0x0079_aef0;
    /// The motion flags the CHR data 1 window toggles, buttons 0xae..0xbe
    /// in order: a bit of `+0x4ac`, or (0) the byte `+0x41d`/`+0x41b`.
    pub const CHR1_FLAGS: [(u32, u32); 17] = [
        (0xae, 1),
        (0xaf, 2),
        (0xb0, 4),
        (0xb1, 8),
        (0xb2, 0x10),
        (0xb3, 0x1000),
        (0xb4, 0x20),
        (0xb5, 0),
        (0xb6, 0),
        (0xb7, 0x100),
        (0xb8, 0x200),
        (0xb9, 0x400),
        (0xba, 0x800),
        (0xbb, 0x2000),
        (0xbc, 0x4000),
        (0xbd, 0x8000),
        (0xbe, 0x1_0000),
    ];

    /// The actor selector's step: by 1 with wrap (0..0x1e), or by 10/100
    /// a jump between the party (0) and the monsters (0x14).
    fn chr_sel_step(h: &mut impl Host, up: bool, step: u32) {
        let s = h.rebase(DEBUG_CHR_SEL);
        let v: i32 = h.read(s);
        if step != 1 {
            h.write(s, if v > 0x13 { 0u32 } else { 0x14 });
            return;
        }
        if up {
            let v = v.wrapping_add(1);
            h.write(s, if v > 0x1e { 0 } else { v });
        } else {
            let v = v.wrapping_sub(1);
            h.write(s, if v < 0 { 0x1e } else { v });
        }
    }

    /// `Ms_Gui_BattleChr1Proc(win, msg, id)`: the battle "CHR data 1"
    /// window: 0x8001 its buttons (actor, FIT, PLY SV, MON SV, 17 motion
    /// flags), 0x8003 closed, 0x8006/0x800a/0x800e the auto-repeats
    /// reset, 0x8005/0x8009/0x800d in battle a button (the actor by 1 or
    /// between party and monsters, a flag toggled, FIT the motion
    /// distances truncated to integers, the party or monster saved),
    /// 0x8011 in battle the flags, the actor's name and file and the
    /// flags' names. `ebp` is the original's frame (the buffer at `ebp -
    /// 0x104`). Returns 0.
    #[allow(clippy::too_many_lines)]
    pub fn chr1_proc(h: &mut impl Host, ebp: u32, win: u32, msg: u32, id: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x104);
        let reps = [0x0113_f8a4u32, 0x0113_f8b8, 0x0113_f8cc];
        let in_battle = |h: &_| Mem::read::<u8>(h, Mem::rebase(h, G_BTL_STATUS)) != 0;
        match msg {
            0x8001 => {
                let t = s(h, 0x00b5_72a0);
                cdecl(h, GU_WINDOW_SET_TITLE, &[win, t]);
                for r in reps {
                    let r = h.rebase(r);
                    cdecl(h, MS_AUTO_REPEAT_INIT, &[r, 0xf, 3]);
                }
                for (bid, label, x, w) in [
                    (0x61u32, 0x00b5_5220u32, 8u32, 0x12u32),
                    (0x62, 0x00b4_8250, 0x1e, 0x12),
                    (0xd5, 0x00b5_72ac, 0xd6, 0x28),
                    (0xd6, 0x00b5_72b0, 0x102, 0x40),
                    (0xd7, 0x00b5_72b8, 0x146, 0x40),
                ] {
                    let l = s(h, label);
                    button(h, win, bid, l, [x, 0, w, 8]);
                }
                for (i, &(bid, _)) in CHR1_FLAGS.iter().enumerate() {
                    let l = s(h, S_SPACE);
                    button(h, win, bid, l, [8, 0x14 + 10 * i as u32, 0x12, 8]);
                }
            }
            0x8003 => set32(h, 0x0113_9f00, 0),
            0x8006 | 0x800a | 0x800e => {
                let r = match msg {
                    0x8006 => reps[0],
                    0x800a => reps[2],
                    _ => reps[1],
                };
                let r = h.rebase(r);
                cdecl(h, MS_AUTO_REPEAT_INIT, &[r, 0xf, 3]);
            }
            0x8005 | 0x8009 | 0x800d => {
                if !in_battle(h) {
                    return 0;
                }
                let sel = g32(h, DEBUG_CHR_SEL);
                let a = cdecl(h, MS_GET_CHR, &[sel]);
                let (rep, step) = match msg {
                    0x8005 => (reps[0], 1),
                    0x8009 => (reps[2], 10),
                    _ => (reps[1], 100),
                };
                let r = h.rebase(rep);
                if cdecl(h, MS_AUTO_REPEAT_CHECK, &[r, id]) == 0 {
                    return 0;
                }
                match id {
                    0x61 | 0x62 => chr_sel_step(h, id == 0x62, step),
                    0xae..=0xbe => {
                        if let Some(&(_, mask)) = CHR1_FLAGS.iter().find(|f| f.0 == id) {
                            if mask == 0 {
                                let o = if id == 0xb5 { 0x41d } else { 0x41b };
                                let b: u8 = h.read(at(a, o));
                                h.write(at(a, o), b.wrapping_sub(1) & 1);
                            } else {
                                let f: u32 = h.read(at(a, 0x4ac));
                                h.write(at(a, 0x4ac), f ^ mask);
                            }
                        }
                    }
                    0xd5 => {
                        for o in [0x4bcu32, 0x4c0, 0x4c8, 0x4d4, 0x4d8, 0x4cc, 0x4d0, 0x4dc] {
                            let (_, f) = trunc_f32(h, f32_at(h, at(a, o)));
                            h.write(at(a, o), f);
                        }
                    }
                    0xd6 => {
                        let name = s(h, 0x00b5_48f0);
                        cdecl(h, MS_GET_PLY_SAVE_NAME, &[buf, name]);
                        dbg(h, 0x00b5_72c0, &[buf]);
                        cdecl(h, MS_PLY_SAVE, &[buf]);
                    }
                    0xd7 => {
                        let sel = g32(h, DEBUG_CHR_SEL);
                        if (sel as i32) >= 0x14 {
                            let m: u32 = h.read(at(a, 4));
                            cdecl(h, MS_SET_FILE_NAME, &[buf, m]);
                            dbg(h, 0x00b5_72c0, &[buf]);
                            let m: u32 = h.read(at(a, 4));
                            let sel = g32(h, DEBUG_CHR_SEL);
                            cdecl(h, MS_MON_SAVE, &[buf, sel, m]);
                        }
                    }
                    _ => {}
                }
            }
            0x8011 => {
                if !in_battle(h) {
                    return 0;
                }
                let sel = g32(h, DEBUG_CHR_SEL);
                let a = cdecl(h, MS_GET_CHR, &[sel]);
                for (bid, mask) in CHR1_FLAGS {
                    let on = if mask == 0 {
                        let o = if bid == 0xb5 { 0x41d } else { 0x41b };
                        h.read::<u8>(at(a, o)) != 0
                    } else {
                        h.read::<u32>(at(a, 0x4ac)) & mask != 0
                    };
                    let l = table_word(h, 0x00c4_468c, u32::from(on));
                    cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, bid, l]);
                }
                print_xy(h, 0x36, 1);
                let sel = g32(h, DEBUG_CHR_SEL);
                let name = table_word(h, 0x00c4_4950, sel);
                cdecl(h, MS_STR_COPY_7CF6C0, &[name, buf, 0x100]);
                gu_print(h, buf);
                print_xy(h, 0x98, 1);
                let sel = g32(h, DEBUG_CHR_SEL);
                if cdecl(h, MS_GET_RAM_CHR_MONSTER, &[sel]) == 0 {
                    let n = u32::from(h.read::<u8>(at(a, 0xc)));
                    sprintf(h, buf, 0x00b5_72cc, &[n]);
                } else {
                    let n = h.read::<u32>(at(a, 4)) & 0x3ff;
                    sprintf(h, buf, 0x00b5_72d8, &[n]);
                }
                gu_print(h, buf);
                for (i, label) in [
                    0x00b5_72e4u32,
                    0x00b5_72f8,
                    0x00b5_7308,
                    0x00b5_7320,
                    0x00b5_7330,
                    0x00b5_7344,
                    0x00b5_735c,
                    0x00b5_736c,
                    0x00b5_737c,
                    0x00b5_738c,
                    0x00b5_73a0,
                    0x00b5_73b0,
                    0x00b5_73c0,
                    0x00b5_73d0,
                    0x00b5_73e0,
                    0x00b5_73f4,
                    0x00b5_7410,
                ]
                .into_iter()
                .enumerate()
                {
                    print_xy(h, 0x20, 0x15 + 10 * i as u32);
                    let l = s(h, label);
                    gu_print(h, l);
                }
            }
            _ => {}
        }
        0
    }

    pub const MS_DEBUG_CAMERA_KEEP: u32 = 0x007c_8c50;
    pub const MS_INFO_WINS_INIT: u32 = 0x007c_fca0;
    pub const MS_INFO_WIN_SET: u32 = 0x007c_fba0;
    pub const MS_CAMERA_KIND_CHAR: u32 = 0x007c_faa0;
    pub const TK_GET_CONTROLER_HIST: u32 = 0x0088_8d70;
    pub const MS_BATTLE_GET_STATUS: u32 = 0x0078_16f0;
    pub const MS_BATTLE_SPECIAL_CAMERA_KIND: u32 = 0x0079_dd70;
    pub const MS_MOVE_TARGET_POS: u32 = 0x007a_b160;
    pub const ADDR_OF_0113ED18: u32 = 0x007d_2420;
    pub const ADDR_OF_0113ED28: u32 = 0x007d_2410;
    pub const G_BTL_SCREEN: u32 = 0x0231_14c0;
    /// The info window's shown roll (degrees) and screen depth
    /// (`0x0113e4d0`, `0x0113e4d4`), its page (`0x0113e4d8`, 0-3), its
    /// blink counter (`0x0113e4cc`), its four windows (`0x0113e494..`).
    pub const INFO_ROLL: u32 = 0x0113_e4d0;
    pub const INFO_DEPTH: u32 = 0x0113_e4d4;
    pub const INFO_PAGE: u32 = 0x0113_e4d8;
    pub const INFO_BLINK: u32 = 0x0113_e4cc;
    pub const INFO_WINS: u32 = 0x0113_e494;

    /// `Gu_Printf(fmt, word)`.
    fn gu_printf(h: &mut impl Host, fmt: u32, word: u32) {
        let f = h.rebase(fmt);
        cdecl(h, GU_PRINTF, &[f, word]);
    }

    /// A `sprintf` of doubles printed with `Gu_Printf(line_fmt, buf)` at
    /// `(x, y)` (`y` none: only `x` set).
    fn info_line(
        h: &mut impl Host,
        buf: u32,
        x: u32,
        y: Option<u32>,
        fmt: u32,
        vals: &[f64],
        line_fmt: u32,
    ) {
        cdecl(h, GU_PRINT_SET_X, &[x]);
        if let Some(y) = y {
            cdecl(h, GU_PRINT_SET_Y, &[y]);
        }
        sprintf_f(h, buf, fmt, &[], vals);
        gu_printf(h, line_fmt, buf);
    }

    /// `Ms_Gui_InfoProc(win, msg, id)`: the battle camera info window:
    /// first the shown roll and depth refreshed from the debug camera;
    /// 0x8001 title, buttons (keep, mode, page, OUT), auto-repeats and the
    /// four info windows; 0x8003 closed (its windows freed);
    /// 0x8006/0x800a the auto-repeats reset; 0x8005/0x8009 a button
    /// (OUT: the camera's positions written as script, the keep stepped,
    /// the keep flag toggled, the next page); 0x8011 its page (0 the
    /// reference and camera positions and kinds, 1 the screen's sizes
    /// moved by the pad, 2 focus and ids, 3 offsets) and the reference
    /// windows, depth and roll. `ebp` is the original's frame (the
    /// buffer at `ebp - 0x104`, the polar form at `- 0x114`, `- 0x110`,
    /// `- 0x11c`). Returns 0.
    // the original compares the shown values exactly (`fucomp`)
    #[allow(clippy::too_many_lines, clippy::float_cmp)]
    pub fn info_proc(h: &mut impl Host, ebp: u32, win: u32, msg: u32, id: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x104);
        let (th, ph, ln) = (
            ebp.wrapping_sub(0x114),
            ebp.wrapping_sub(0x110),
            ebp.wrapping_sub(0x11c),
        );
        let screen = g32(h, DEBUG_SCREEN);
        let cam = u32::from(h.read::<u8>(at(h.rebase(DEBUG_CAMERA), screen.wrapping_mul(0x1c0))));
        let num = cdecl(h, MS_CAMERA_GET_NUM, &[cam]);
        let keep = cdecl(h, MS_DEBUG_CAMERA_KEEP, &[0]);
        let c = cam.wrapping_mul(0xda0);
        let cf = |h: &_, sym: u32| f32_at(h, at(Mem::rebase(h, sym), c));
        let roll = deg(h, cf(h, 0x0113_81d0));
        if f32_at(h, h.rebase(INFO_ROLL)) != roll {
            set32(h, INFO_ROLL, x87::store(roll));
        }
        if cf(h, 0x0113_81dc) != f32_at(h, h.rebase(INFO_DEPTH)) {
            let v = fcopy(h.read(at(h.rebase(0x0113_81dc), c)));
            set32(h, INFO_DEPTH, v);
        }
        let reps = [0x0113_e4a4u32, 0x0113_e4b8];
        match msg {
            0x8001 => {
                let t = s(h, 0x00b5_5e08);
                cdecl(h, GU_WINDOW_SET_TITLE, &[win, t]);
                let k = g32(h, DEBUG_CAMERA_KEEP);
                let l = table_word(h, 0x00c4_468c, k);
                set32(h, INFO_PAGE, 0);
                button(h, win, 0x35, l, [0xe0, 0x1e, 0xe, 8]);
                let l = table_word(h, 0x00c4_4264, keep);
                button(h, win, 6, l, [0xf4, 0x1e, 0x30, 8]);
                let l = s(h, S_SPACE);
                button(h, win, 0x3a, l, [0x126, 0x1e, 6, 8]);
                let l = s(h, 0x00b5_5e14);
                button(h, win, 5, l, [0x12e, 0x1e, 0x30, 8]);
                for r in reps {
                    let r = h.rebase(r);
                    cdecl(h, MS_AUTO_REPEAT_INIT, &[r, 0xf, 3]);
                }
                let roll = deg(h, cf(h, 0x0113_81d0));
                set32(h, INFO_ROLL, x87::store(roll));
                let v = fcopy(h.read(at(h.rebase(0x0113_81dc), c)));
                set32(h, INFO_DEPTH, v);
                cdecl(h, MS_INFO_WINS_INIT, &[]);
            }
            0x8003 => {
                set32(h, 0x0113_9ee0, 0);
                for k in 0..4u32 {
                    let g = INFO_WINS + 4 * k;
                    let w = g32(h, g);
                    h.write(at(w, 0x20), 0u8);
                    set32(h, g, 0);
                }
            }
            0x8006 | 0x800a => {
                let r = h.rebase(if msg == 0x8006 { reps[0] } else { reps[1] });
                cdecl(h, MS_AUTO_REPEAT_INIT, &[r, 0xf, 3]);
            }
            0x8005 | 0x8009 => {
                let r = h.rebase(if msg == 0x8005 { reps[0] } else { reps[1] });
                if cdecl(h, MS_AUTO_REPEAT_CHECK, &[r, id]) == 0 {
                    return 0;
                }
                match id {
                    5 => {
                        let v = at(h.rebase(0x0113_7bdc), c);
                        let r: u32 = h.read(at(h.rebase(0x0113_7c18), c));
                        cdecl(h, MS_VECTOR_RECT2_POLAR, &[v, r, th, ph, ln]);
                        let vals = [
                            cf(h, 0x0113_7590),
                            cf(h, 0x0113_7594),
                            cf(h, 0x0113_7598),
                            cf(h, 0x0113_7bac),
                            cf(h, 0x0113_7bb0),
                            cf(h, 0x0113_7bb4),
                        ];
                        sprintf_f(h, buf, 0x00b5_5e18, &[], &vals);
                        cdecl(h, TK_REMOVE, &[buf]);
                        dbg(h, S_FMT_S, &[buf]);
                    }
                    6 => {
                        let step = if msg == 0x8005 { 1 } else { u32::MAX };
                        cdecl(h, MS_DEBUG_CAMERA_KEEP, &[step]);
                    }
                    0x35 => {
                        let k = g32(h, DEBUG_CAMERA_KEEP);
                        set32(h, DEBUG_CAMERA_KEEP, k.wrapping_sub(1) & 1);
                    }
                    0x3a => {
                        let p = g32(h, INFO_PAGE).wrapping_add(1);
                        set32(h, INFO_PAGE, if (p as i32) > 3 { 0 } else { p });
                    }
                    _ => {}
                }
            }
            0x8011 => info_paint(h, buf, win, keep, num, c, [th, ph, ln]),
            _ => {}
        }
        0
    }

    /// The info window's 0x8011 page and footer.
    #[allow(clippy::too_many_lines)]
    fn info_paint(
        h: &mut impl Host,
        buf: u32,
        win: u32,
        keep: u32,
        num: u32,
        c: u32,
        polar: [u32; 3],
    ) {
        const LINE: u32 = 0x00b1_2ad0;
        let k = g32(h, DEBUG_CAMERA_KEEP);
        let l = table_word(h, 0x00c4_468c, k);
        let b = g32(h, INFO_BLINK);
        set32(h, INFO_BLINK, b.wrapping_add(1));
        cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, 0x35, l]);
        let l = table_word(h, 0x00c4_4264, keep);
        cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, 6, l]);
        let mut y = 1u32;
        let cf = |h: &_, sym: u32, o: u32| f32_at(h, at(at(Mem::rebase(h, sym), c), o));
        let scr = |h: &_| g32(h, G_BTL_SCREEN);
        match g32(h, INFO_PAGE) {
            0 => {
                for side in 0..2u32 {
                    let so = side.wrapping_mul(0x61c);
                    cdecl(h, GU_PRINT_SET_X, &[2]);
                    if side == 0 {
                        cdecl(h, GU_PRINT_SET_Y, &[y]);
                        let m = cdecl(h, MS_DEBUG_GET_CAMERA_MODE, &[0]);
                        let t = table_word(h, 0x00c4_4640, m);
                        gu_printf(h, 0x00b5_5e44, t);
                    } else {
                        cdecl(h, GU_PRINT_SET_Y, &[0xb]);
                        let m = cdecl(h, MS_DEBUG_GET_CAMERA_MODE, &[1]);
                        let t = table_word(h, 0x00c4_4640, m);
                        gu_printf(h, 0x00b5_5e4c, t);
                    }
                    for (x, fmt, o) in [
                        (0x50u32, 0x00b5_5e54u32, 0u32),
                        (0xaa, 0x00b5_5e5c, 4),
                        (0x104, 0x00b5_5e64, 8),
                    ] {
                        let v = cf(h, 0x0113_7580, so + o);
                        info_line(h, buf, x, Some(y), fmt, &[v], LINE);
                    }
                    let w = g32(h, INFO_WINS);
                    if side == 0 && w != 0 {
                        let blink = g32(h, INFO_BLINK) & 1 != 0;
                        h.write(at(w, 0x21), 1u8);
                        for o in [0u32, 4, 8] {
                            let v = fcopy(h.read(at(at(h.rebase(0x0113_7580), c), o)));
                            let w = g32(h, INFO_WINS);
                            h.write(at(w, o), v);
                        }
                        let w = g32(h, INFO_WINS);
                        h.write(at(w, 0x22), 2u8);
                        let rgb = if blink {
                            [0xcdu32, 0xcd, 0xff]
                        } else {
                            [0xbc, 0xbc, 0xe4]
                        };
                        for (i, v) in rgb.into_iter().enumerate() {
                            let w = g32(h, INFO_WINS);
                            h.write(at(w, 0x28 + 4 * i as u32), v);
                        }
                    }
                    y += 10;
                }
                print_xy(h, 2, y);
                sprintf(h, buf, 0x00b5_5e6c, &[]);
                gu_printf(h, LINE, buf);
                let mut p = at(h.rebase(0x0113_81ac), c);
                let mut x = 0x17u32;
                while x < 0x47 {
                    print_xy(h, x - 1, y);
                    let kind = u32::from(h.read::<u16>(p.wrapping_sub(2))) & 0xff;
                    let ch = i32::from(cdecl(h, MS_CAMERA_KIND_CHAR, &[kind]) as u8 as i8) as u32;
                    gu_printf(h, 0x00b5_5e70, ch);
                    if h.read::<u8>(p) == 0 {
                        print_xy(h, x, y);
                        gu_printf(h, 0x00b5_5e70, ch);
                    }
                    p = at(p, 8);
                    x += 0xc;
                }
                print_xy(h, num.wrapping_mul(12).wrapping_add(0x16), y + 2);
                let u = s(h, 0x00b5_5e74);
                gu_print(h, u);
                // junk read: [ebp-0x114], [ebp-0x110], [ebp-0x11c] never
                // written in this call (the polar form comes later); the
                // shim's frame is zeroed, so 0 (brief 2026-10-24)
                cdecl(h, GU_PRINT_SET_X, &[0x50]);
                cdecl(h, GU_PRINT_SET_Y, &[y]);
                let t = x87::store(deg(h, f32_at(h, polar[0])));
                let lim = f32_at(h, h.rebase(0x00b5_5f80));
                let t = if x87::load(t) < lim {
                    x87::load(x87::store(x87::add(x87::load(t), f64_at(h, 0x00b4_8380))))
                } else {
                    x87::load(t)
                };
                sprintf_f(h, buf, 0x00b5_5e78, &[], &[t]);
                gu_printf(h, LINE, buf);
                let v = deg(h, f32_at(h, polar[1]));
                info_line(h, buf, 0xaa, Some(y), 0x00b5_5e80, &[v], LINE);
                let v = f32_at(h, polar[2]);
                info_line(h, buf, 0x104, Some(y), 0x00b5_5e88, &[v], LINE);
            }
            1 => {
                let pad = cdecl(h, TK_GET_CONTROLER_HIST, &[0, 0, 0]) & 0xffff;
                print_xy(h, 2, 1);
                let g = scr(h);
                let v = [f32_at(h, at(g, 0x10)), f32_at(h, at(g, 0x14))];
                sprintf_f(h, buf, 0x00b5_5e90, &[], &v);
                gu_printf(h, S_FMT_S, buf);
                let step = if pad & 8 != 0 {
                    Some(1.0)
                } else if pad & 2 != 0 {
                    Some(f32_at(h, h.rebase(0x00b9_2318)))
                } else {
                    None
                };
                if let Some(st) = step {
                    let st = x87::load(x87::store(st));
                    for (bit, o, sub) in [
                        (0x8000u32, 0x20u32, false),
                        (0x2000, 0x20, true),
                        (0x1000, 0x24, false),
                        (0x4000, 0x24, true),
                    ] {
                        if pad & bit != 0 {
                            let g = scr(h);
                            let f = f32_at(h, at(g, o));
                            let v = if sub {
                                x87::sub(f, st)
                            } else {
                                x87::add(f, st)
                            };
                            h.write(at(g, o), x87::store(v));
                        }
                    }
                }
                for (x, y, fmt, o) in [
                    (0xc4u32, 1u32, 0x00b5_5ea4u32, 0x20u32),
                    (2, 0xb, 0x00b5_5eb0, 0x40),
                    (0xc4, 0xb, 0x00b5_5ec4, 0x48),
                    (2, 0x15, 0x00b5_5ed4, 0x50),
                    (0xc4, 0x15, 0x00b5_5ec4, 0x58),
                ] {
                    let g = scr(h);
                    let v = [f32_at(h, at(g, o)), f32_at(h, at(g, o + 4))];
                    info_line(h, buf, x, Some(y), fmt, &v, S_FMT_S);
                }
                y = 0x15;
            }
            2 => {
                print_xy(h, 2, 1);
                let g = scr(h);
                let w: Vec<u32> = [0xccu32, 0xd0, 0xd8, 0xd4]
                    .iter()
                    .map(|&o| Mem::read(h, at(g, o)))
                    .collect();
                // call width: 6 (the buffer, the format, four words)
                sprintf(h, buf, 0x00b5_5ee8, &w);
                gu_printf(h, S_FMT_S, buf);
                print_xy(h, 2, 0xb);
                let id = cdecl(h, MS_CAMERA_GET_ID, &[0]);
                sprintf(h, buf, 0x00b5_5f08, &[id]);
                gu_printf(h, S_FMT_S, buf);
                print_xy(h, 2, 0x15);
                let a = i32::from(h.read::<i8>(h.rebase(0x0112_c9fc))) as u32;
                let b = i32::from(h.read::<i8>(h.rebase(0x0112_a8e7))) as u32;
                let st = cdecl(h, MS_BATTLE_GET_STATUS, &[]);
                let kind = cdecl(h, MS_BATTLE_SPECIAL_CAMERA_KIND, &[st]);
                let sp = i32::from(h.read::<i8>(h.rebase(0x0112_a8e6))) as u32;
                sprintf(h, buf, 0x00b5_5f10, &[sp, kind, st, 1, b, a]);
                gu_printf(h, S_FMT_S, buf);
                y = 0x1f;
            }
            3 => {
                print_xy(h, 2, 1);
                let g = scr(h);
                let v = f32_at(h, at(g, 0x30));
                sprintf_f(h, buf, 0x00b5_5f38, &[], &[v]);
                gu_printf(h, S_FMT_S, buf);
                let g = scr(h);
                let v = f32_at(h, at(g, 0x34));
                info_line(h, buf, 0x72, None, 0x00b5_5f40, &[v], S_FMT_S);
                let mut yy = 0xbu32;
                for side in 0..2u32 {
                    let p = at(at(h.rebase(0x0113_7604), c), side * 0x61c);
                    cdecl(h, GU_PRINT_SET_Y, &[yy]);
                    for (x, fmt, o) in [
                        (2u32, 0x00b5_5f48u32, 0u32),
                        (0x72, 0x00b5_5f50, 4),
                        (0xe2, 0x00b5_5f58, 8),
                    ] {
                        let v = f32_at(h, p.wrapping_sub(4).wrapping_add(o));
                        info_line(h, buf, x, None, fmt, &[v], S_FMT_S);
                    }
                    yy += 10;
                }
                y = 0x1f;
            }
            _ => {}
        }
        let p = cdecl(h, ADDR_OF_0113ED18, &[]);
        cdecl(h, MS_INFO_WIN_SET, &[1, 1, p]);
        let p = cdecl(h, ADDR_OF_0113ED28, &[]);
        cdecl(h, MS_INFO_WIN_SET, &[2, 1, p]);
        if cdecl(h, MS_BATTLE_GET_STATUS, &[]) != 0 {
            let p = cdecl(h, MS_MOVE_TARGET_POS, &[]);
            cdecl(h, MS_INFO_WIN_SET, &[3, 1, p]);
        }
        let v = at(h.rebase(0x0113_7bac), c);
        let r: u32 = h.read(at(h.rebase(0x0113_7c18), c));
        cdecl(
            h,
            MS_VECTOR_RECT2_POLAR,
            &[v, r, polar[0], polar[1], polar[2]],
        );
        y += 10;
        print_xy(h, 2, y);
        sprintf(h, buf, 0x00b5_5f60, &[]);
        gu_printf(h, LINE, buf);
        let v = f32_at(h, h.rebase(0x0113_701c));
        info_line(h, buf, 0x3e, Some(y), 0x00b5_5f68, &[v], LINE);
        print_xy(h, 0x7c, y);
        sprintf(h, buf, 0x00b5_5f70, &[]);
        gu_printf(h, LINE, buf);
        let v = f32_at(h, h.rebase(INFO_ROLL));
        info_line(h, buf, 0xa2, Some(y), 0x00b5_5f78, &[v], LINE);
    }

    pub const MS_DEBUG_CAMERA_KEEP_ADD: u32 = 0x007d_0ee0;
    pub const MS_DEBUG_CAMERA_COPY: u32 = 0x007d_1220;
    pub const MS_DEBUG_KEEP_WINDOWS: u32 = 0x007d_71d0;
    pub const MS_DEBUG_CAMERA_APPLY: u32 = 0x007d_2fc0;
    pub const MS_DEBUG_OUT_CAMERA: u32 = 0x007d_2430;
    pub const MS_DEBUG_CAMERA_POLAR: u32 = 0x007d_1030;
    pub const MS_DEBUG_CAMERA_KEEP_LOAD: u32 = 0x007d_7130;
    /// The point editor's undo copy of the keep block (`0x0113e668`) and
    /// the keep values it edits (`0x0113ed64..0x0113ed74`, the offset
    /// `0x0113f3ac`).
    pub const KEEP_UNDO: u32 = 0x0113_e668;

    /// A button label of the point editor: a string, or a table entry.
    #[derive(Debug, Clone, Copy)]
    enum MapLabel {
        S(u32),
        Flag,
        Speed,
        Keep,
    }

    /// `Ms_Gui_MapDataProc(win, msg, id)`: the battle "Camera Point Data"
    /// window: first the camera's id and channel, the keep count and the
    /// keep mode (`MsDebugCameraKeep_007c8c50(0)`; no keeps: one added);
    /// 0x8001 title, buttons and the undo copy, auto-repeats;
    /// 0x8003 closed; 0x8006/0x800a/0x800e the auto-repeats reset;
    /// 0x8005/0x8009/0x800d a button (steps 1/10/100): the keep's angles,
    /// length, roll, weight, offset (`MsDebugCameraPolar_007d1030`),
    /// truncation, undo, the current keep, add, set, delete, flags, OUT;
    /// 0x8011 the keep refreshed from the camera and its values printed,
    /// the camera re-requested. Then (every message) `0x0113ed80` = the
    /// channel's flag byte. `ebp` is the original's frame (the buffer at
    /// `ebp - 0x104`). Returns 0.
    #[allow(clippy::too_many_lines)]
    pub fn map_data_proc(
        h: &mut impl Host,
        trig: &mut dyn FnMut(Trig, f64) -> u32,
        ebp: u32,
        win: u32,
        msg: u32,
        id: u32,
    ) -> u32 {
        const LT: u32 = 0x00b5_5220;
        const GT: u32 = 0x00b4_8250;
        let buf = ebp.wrapping_sub(0x104);
        let screen = g32(h, DEBUG_SCREEN);
        let cam = u32::from(h.read::<u8>(at(h.rebase(DEBUG_CAMERA), screen.wrapping_mul(0x1c0))));
        cdecl(h, MS_CAMERA_GET_ID, &[cam]);
        let rec = at(h.rebase(0x0113_7b8c), cam.wrapping_mul(0xda0));
        let num = cdecl(h, MS_CAMERA_GET_NUM, &[cam]);
        let flag_at = at(rec, num.wrapping_mul(0x130).wrapping_add(0xc3));
        let cnt = g32(h, CAMERA_KEEP_BLOCK);
        let keep = cdecl(h, MS_DEBUG_CAMERA_KEEP, &[0]);
        if cnt == 0 {
            cdecl(h, MS_DEBUG_CAMERA_KEEP_ADD, &[]);
        }
        let reps = [0x0113_e624u32, 0x0113_e638, 0x0113_e64c];
        let copy_block = |h: &mut _, from: u32, to: u32| {
            let (f, t) = (Mem::rebase(h, from), Mem::rebase(h, to));
            for i in 0..0x18cu32 {
                let w: u32 = Mem::read(h, at(f, 4 * i));
                Mem::write(h, at(t, 4 * i), w);
            }
        };
        match msg {
            0x8001 => {
                let t = s(h, 0x00b5_634c);
                cdecl(h, GU_WINDOW_SET_TITLE, &[win, t]);
                let rows: [(u32, MapLabel, [u32; 4]); 25] = [
                    (0xf, MapLabel::S(LT), [0x40, 0, 0x12, 8]),
                    (0x10, MapLabel::S(GT), [0x54, 0, 0x12, 8]),
                    (0x15, MapLabel::S(LT), [0xea, 0, 0x12, 8]),
                    (0x16, MapLabel::S(GT), [0xfe, 0, 0x12, 8]),
                    (0x11, MapLabel::S(LT), [0x40, 0xa, 0x12, 8]),
                    (0x12, MapLabel::S(GT), [0x54, 0xa, 0x12, 8]),
                    (0x19, MapLabel::S(0x00b5_631c), [0xd6, 0xa, 0x34, 8]),
                    (0x1b, MapLabel::S(0x00b5_6360), [0x110, 0xa, 0x34, 8]),
                    (0x13, MapLabel::S(LT), [0x40, 0x14, 0x12, 8]),
                    (0x14, MapLabel::S(GT), [0x54, 0x14, 0x12, 8]),
                    (0x1a, MapLabel::S(0x00b5_6364), [0xd6, 0x14, 0x34, 8]),
                    (0x1c, MapLabel::Flag, [0x110, 0x14, 0x34, 8]),
                    (0xb, MapLabel::S(LT), [0x40, 0x1e, 0x12, 8]),
                    (0xc, MapLabel::S(GT), [0x54, 0x1e, 0x12, 8]),
                    (0x20, MapLabel::S(0x00b5_6368), [0xd6, 0x1e, 0x34, 8]),
                    (0x33, MapLabel::S(0x00b5_6308), [0x110, 0x1e, 0x34, 8]),
                    (0x23, MapLabel::S(LT), [0x40, 0x28, 0x12, 8]),
                    (0x24, MapLabel::S(GT), [0x54, 0x28, 0x12, 8]),
                    (0x37, MapLabel::Speed, [0xd6, 0x28, 0xe, 8]),
                    (0x38, MapLabel::Speed, [0xfc, 0x28, 0xe, 8]),
                    (0x39, MapLabel::Speed, [0x122, 0x28, 0xe, 8]),
                    (0x17, MapLabel::S(LT), [0x40, 0x32, 0x12, 8]),
                    (0x18, MapLabel::S(GT), [0x54, 0x32, 0x12, 8]),
                    (6, MapLabel::Keep, [0xd6, 0x32, 0x34, 8]),
                    (5, MapLabel::S(0x00b5_5e14), [0x110, 0x32, 0x34, 8]),
                ];
                for (bid, label, rect) in rows {
                    let l = match label {
                        MapLabel::S(sym) => s(h, sym),
                        MapLabel::Flag => {
                            let f = u32::from(h.read::<u8>(flag_at)) & 1;
                            table_word(h, 0x00c4_48d4, f)
                        }
                        MapLabel::Speed => {
                            let v = i32::from(h.read::<i8>(h.rebase(0x0112_a8e1))) as u32;
                            table_word(h, 0x00c4_468c, v)
                        }
                        MapLabel::Keep => table_word(h, 0x00c4_4264, keep),
                    };
                    button(h, win, bid, l, rect);
                }
                copy_block(h, CAMERA_KEEP_BLOCK, KEEP_UNDO);
                for r in reps {
                    let r = h.rebase(r);
                    cdecl(h, MS_AUTO_REPEAT_INIT, &[r, 0xf, 1]);
                }
            }
            0x8003 => set32(h, 0x0113_9eec, 0),
            0x8006 | 0x800a | 0x800e => {
                let r = match msg {
                    0x8006 => reps[0],
                    0x800e => reps[1],
                    _ => reps[2],
                };
                let r = h.rebase(r);
                cdecl(h, MS_AUTO_REPEAT_INIT, &[r, 0xf, 1]);
            }
            0x8005 | 0x8009 | 0x800d => {
                let (rep, step) = match msg {
                    0x8005 => (reps[0], 1i32),
                    0x800d => (reps[1], 100),
                    _ => (reps[2], 10),
                };
                let r = h.rebase(rep);
                if cdecl(h, MS_AUTO_REPEAT_CHECK, &[r, id]) != 0 {
                    map_data_button(h, msg, id, f64::from(step), cnt, rec, flag_at);
                }
            }
            0x8011 => {
                cdecl(h, MS_DEBUG_CAMERA_COPY, &[]);
                cdecl(h, MS_DEBUG_KEEP_WINDOWS, &[u32::MAX]);
                let l = table_word(h, 0x00c4_4264, keep);
                cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, 6, l]);
                let f = u32::from(h.read::<u8>(flag_at));
                let l = table_word(h, 0x00c4_48d4, f & 1);
                cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, 0x1c, l]);
                for (bid, k) in [(0x37u32, 1u32), (0x38, 2), (0x39, 3)] {
                    let f = u32::from(!h.read::<u8>(flag_at));
                    let l = table_word(h, 0x00c4_468c, (f >> k) & 1);
                    cdecl(h, GU_WINDOW_SET_BUTTON_TEXT, &[win, bid, l]);
                }
                map_data_paint(h, trig, buf, cnt);
            }
            _ => {}
        }
        let f = u32::from(h.read::<u8>(flag_at));
        set32(h, 0x0113_ed80, f);
        0
    }

    /// A keep value at `sym` stepped by `-/+ step * k` (`k` the `f64` at
    /// `ksym`).
    fn keep_step(h: &mut impl Host, sym: u32, ksym: u32, sub: bool, step: f64) {
        let d = x87::mul(step, f64_at(h, ksym));
        let f = f32_at(h, h.rebase(sym));
        let v = if sub { x87::sub(f, d) } else { x87::add(d, f) };
        set32(h, sym, x87::store(v));
    }

    /// A point editor button (after its auto-repeat).
    #[allow(clippy::too_many_lines)]
    fn map_data_button(
        h: &mut impl Host,
        msg: u32,
        id: u32,
        step: f64,
        cnt: u32,
        rec: u32,
        flag_at: u32,
    ) {
        let cur = |h: &_| g32(h, KEEP_CUR);
        let keep_rec = |h: &_| at(Mem::rebase(h, KEEPS), cur(h).wrapping_mul(0x30));
        match id {
            0x33 => {
                for sym in [0x0113_ed68u32, 0x0113_ed6c, 0x0113_ed70] {
                    let (_, f) = trunc_f32(h, f32_at(h, h.rebase(sym)));
                    set32(h, sym, f);
                }
                let x = x87::div(f32_at(h, h.rebase(0x0113_ed74)), f64_at(h, TENTH));
                let (_, f) = trunc_f32(h, x);
                set32(
                    h,
                    0x0113_ed74,
                    x87::store(x87::div(x87::load(f), f64_at(h, METRE))),
                );
                let (_, f) = trunc_f32(h, f32_at(h, h.rebase(0x0113_ed64)));
                set32(h, 0x0113_ed64, f);
            }
            0xf..=0x14 => {
                let sym = [0x0113_ed68u32, 0x0113_ed6c, 0x0113_ed70][((id - 0xf) / 2) as usize];
                keep_step(h, sym, TENTH, id & 1 == 1, step);
            }
            0xb | 0xc => {
                let k = if id == 0xb { 0x00b5_67b0 } else { TENTH };
                let d = x87::store(x87::mul(step, f64_at(h, k)));
                let a = [
                    h.rebase(0x0113_f3ac),
                    d,
                    h.rebase(0x0113_ed70),
                    h.rebase(0x0113_ed6c),
                ];
                cdecl(h, MS_DEBUG_CAMERA_POLAR, &a);
            }
            0x17 | 0x18 => keep_step(h, 0x0113_ed74, HUNDREDTH, id == 0x17, step),
            0x20 => {
                let (f, t) = (h.rebase(KEEP_UNDO), h.rebase(CAMERA_KEEP_BLOCK));
                for i in 0..0x18cu32 {
                    let w: u32 = h.read(at(f, 4 * i));
                    h.write(at(t, 4 * i), w);
                }
                cdecl(h, MS_DEBUG_CAMERA_KEEP_LOAD, &[]);
            }
            0x23 | 0x24 => keep_step(h, 0x0113_ed64, TENTH, id == 0x23, step),
            0x15 | 0x16 => {
                let c = cur(h) as i32;
                let mut c = if id == 0x15 {
                    c.wrapping_sub(1)
                } else {
                    c.wrapping_add(1)
                };
                if c < 0 {
                    c = (cnt as i32).wrapping_sub(1);
                }
                set32(h, KEEP_CUR, if c >= cnt as i32 { 0 } else { c as u32 });
                cdecl(h, MS_DEBUG_CAMERA_KEEP_LOAD, &[]);
            }
            0x1c | 0x37 | 0x38 | 0x39 => {
                let bit = match id {
                    0x1c => 1u8,
                    0x37 => 2,
                    0x38 => 4,
                    _ => 8,
                };
                let f: u8 = h.read(flag_at);
                h.write(flag_at, f ^ bit);
            }
            6 => {
                let st = if msg == 0x8005 { 1 } else { u32::MAX };
                cdecl(h, MS_DEBUG_CAMERA_KEEP, &[st]);
            }
            0x1b => {
                cdecl(h, MS_DEBUG_CAMERA_KEEP_ADD, &[]);
                cdecl(h, MS_DEBUG_CAMERA_KEEP_LOAD, &[]);
            }
            0x19 => {
                for o in [0u32, 4, 8] {
                    let v = fcopy(h.read(at(rec, 0x30 + o)));
                    let k = keep_rec(h);
                    h.write(at(k, o), v);
                }
            }
            0x1a => {
                if (cnt as i32) > 1 {
                    let n = cnt.wrapping_sub(1);
                    set32(h, CAMERA_KEEP_BLOCK, n);
                    let c = cur(h);
                    if (c as i32) < 0x1f {
                        let end = h.rebase(0x0113_f35c);
                        let mut p = keep_rec(h);
                        loop {
                            for o in (0..0x30u32).step_by(4) {
                                let v = fcopy(h.read(at(p, 0x30 + o)));
                                h.write(at(p, o), v);
                            }
                            p = at(p, 0x30);
                            if (p as i32) >= (end as i32) {
                                break;
                            }
                        }
                    }
                    if (cur(h) as i32) >= n as i32 {
                        set32(h, KEEP_CUR, n.wrapping_sub(1));
                    }
                }
                cdecl(h, MS_DEBUG_CAMERA_KEEP_LOAD, &[]);
            }
            5 => {
                cdecl(h, MS_DEBUG_OUT_CAMERA, &[]);
            }
            _ => {}
        }
    }

    /// The point editor's 0x8011 values: the current keep refreshed
    /// (weight at least 0, the angles in radians, the distance, the
    /// offset `sin(phi) * distance` with `_CIsin`, the roll), then printed;
    /// the camera re-requested (`MsDebugCameraApply_007d2fc0(1)`).
    fn map_data_paint(
        h: &mut impl Host,
        trig: &mut dyn FnMut(Trig, f64) -> u32,
        buf: u32,
        cnt: u32,
    ) {
        const LINE: u32 = 0x00b1_2ad0;
        const F4: u32 = 0x00b5_6378;
        let g = |h: &_, sym: u32| f32_at(h, Mem::rebase(h, sym));
        let keep_rec = |h: &_| at(Mem::rebase(h, KEEPS), g32(h, KEEP_CUR).wrapping_mul(0x30));
        let w = g(h, 0x0113_ed74);
        let w = if w < 0.0 {
            set32(h, 0x0113_ed74, 0);
            0
        } else {
            x87::store(w)
        };
        h.write(at(keep_rec(h), 0xc), w);
        let (pi, d180) = (f64_at(h, PI_F64), f64_at(h, DEG_180));
        let a = x87::store(x87::div(x87::mul(g(h, 0x0113_ed68), pi), d180));
        h.write(at(keep_rec(h), 0x10), a);
        let b = x87::store(x87::div(x87::mul(g(h, 0x0113_ed6c), pi), d180));
        h.write(at(keep_rec(h), 0x14), b);
        let d = fcopy(g32(h, 0x0113_ed70));
        h.write(at(keep_rec(h), 0x18), d);
        let x = x87::div(x87::mul(pi, g(h, 0x0113_ed6c)), d180);
        let sn = trig(Trig::Sin, x);
        let off = x87::store(x87::mul(x87::load(sn), g(h, 0x0113_ed70)));
        set32(h, 0x0113_f3ac, off);
        let r = fcopy(g32(h, 0x0113_ed64));
        h.write(at(keep_rec(h), 0x20), r);
        let label = |h: &mut _, x: u32, y: u32, sym: u32| {
            print_xy(h, x, y);
            sprintf(h, buf, sym, &[]);
            gu_printf(h, LINE, buf);
        };
        label(h, 0, 1, 0x00b5_6370);
        info_line(h, buf, 0x6c, Some(1), F4, &[g(h, 0x0113_ed68)], LINE);
        label(h, 0xd4, 1, 0x00b5_6380);
        print_xy(h, 0x118, 1);
        let c = g32(h, KEEP_CUR).wrapping_add(1);
        sprintf(h, buf, 0x00b5_5240, &[c, cnt]);
        gu_printf(h, LINE, buf);
        label(h, 0, 0xb, 0x00b5_6384);
        info_line(h, buf, 0x6c, Some(0xb), F4, &[g(h, 0x0113_ed6c)], LINE);
        label(h, 0, 0x15, 0x00b5_6388);
        info_line(h, buf, 0x6c, Some(0x15), F4, &[g(h, 0x0113_ed70)], LINE);
        label(h, 0, 0x1f, 0x00b5_638c);
        info_line(
            h,
            buf,
            0x6c,
            Some(0x1f),
            FMT_VALUE,
            &[g(h, 0x0113_f3ac)],
            LINE,
        );
        label(h, 0, 0x29, 0x00b5_6394);
        info_line(h, buf, 0x6c, Some(0x29), F4, &[g(h, 0x0113_ed64)], LINE);
        for (x, sym) in [
            (0xeau32, 0x00b5_639cu32),
            (0x110, 0x00b5_63a0),
            (0x136, 0x00b5_63a4),
        ] {
            print_xy(h, x, 0x29);
            gu_printf(h, sym, buf);
        }
        label(h, 0, 0x33, 0x00b5_63a8);
        info_line(
            h,
            buf,
            0x6c,
            Some(0x33),
            FMT_VALUE,
            &[g(h, 0x0113_ed74)],
            LINE,
        );
        cdecl(h, MS_DEBUG_CAMERA_APPLY, &[1]);
    }
}
